use crate::TiffMapErr;
use crate::geometry::{Point2D, Point3D};
use crate::oct_tree::DataPoint3D;
use crate::quad_tree::DataPoint2D;
use color_eyre::Result;
use color_eyre::eyre::{Context, bail};
use jiff::civil::DateTime;
use jiff::tz::TimeZone;
use std::collections::HashMap;

/// Convert a `GeoTiff` into a flat list of pixel-center points in model space.
///
/// The point `id` is the raster index `row * raster_width + col`, so it is unique
/// within a single `GeoTIFF`. If you merge several `GeoTIFFs`, offset the IDs yourself.
///
/// Coordinates are computed by linearly interpolating between the corners of
/// [`GeoTiff::model_extent`]. This is exact for the common scale/translate
/// affine case; rotated or sheared transforms will be approximated.
#[allow(clippy::cast_precision_loss)]
#[must_use]
pub fn geotiff_to_points(
    geotiff: &geotiff::GeoTiff,
    id_offset: Option<usize>,
) -> Vec<DataPoint2D<f64>> {
    let id_offset = id_offset.unwrap_or(0);
    let extent = geotiff.model_extent();
    let width = geotiff.raster_width;
    let height = geotiff.raster_height;

    let min_x = extent.min().x;
    let min_y = extent.min().y;
    let max_x = extent.max().x;
    let max_y = extent.max().y;

    let dx = (max_x - min_x) / width as f64;
    let dy = (max_y - min_y) / height as f64;

    let mut points = Vec::with_capacity(width * height);
    for row in 0..height {
        for col in 0..width {
            let x = (col as f64 + 0.5).mul_add(dx, min_x);
            let y = (row as f64 + 0.5).mul_add(-dy, max_y);
            let id = id_offset + row * width + col;
            points.push(DataPoint2D::new(id, Point2D::new(x, y)));

            // TODO Remove this, this is just an example get of a value
            // TODO We probably want to create a map id -> Point
            // let val = geotiff.get_value_at::<f32>(&geo_types::Coord { x, y }, 0);
            // match val {
            //     Some(0.0) => {}
            //     Some(f) => {
            //         dbg!(f);
            //     }
            //     None => panic!("no value @ {x}/{y}"),
            // }
        }
    }
    // assert_eq!(geotiff.num_samples, points.len());

    points
}

/// Convert a slice of `GeoTiff`s into a single flat list of pixel-center points.
///
/// IDs are offset so that each input `GeoTiff` contributes a non-overlapping ID
/// range: the first file starts at `0`, the second at `raster_width * raster_height`,
/// and so on.
#[must_use]
pub fn geotiffs_to_points(geotiffs: &[geotiff::GeoTiff]) -> Vec<DataPoint2D<f64>> {
    let mut points = Vec::new();
    let mut id_offset = 0;
    for tiff in geotiffs {
        let tiff_size = tiff.raster_width * tiff.raster_height;

        let mut tiff_points = geotiff_to_points(tiff, Some(id_offset));
        assert_eq!(tiff_size, tiff_points.len());

        id_offset += tiff_size;
        points.append(&mut tiff_points);
    }
    points
}

// TODO Make this all generic
pub fn map_points(
    geotiff: geotiff::GeoTiff,
    points: &[DataPoint2D<f64>],
) -> Result<HashMap<usize, f32>, TiffMapErr<f64>> {
    let mut map = HashMap::new();

    for point in points {
        let id = point.id;
        let x = point.point.x;
        let y = point.point.y;

        let coord = geo_types::Coord { x, y };
        let val = geotiff
            .get_value_at::<f32>(&coord, 0)
            .ok_or_else(|| TiffMapErr::MissingValue { x, y })?;
        if let Some(_old) = map.insert(id, val) {
            return Err(TiffMapErr::DuplicateId(id));
        }
    }

    Ok(map)
}

/// Seconds since Unix epoch from a DWD filename `YW_2017.002_YYYYMMDD_HHMM.tif`.
fn filename_to_epoch_seconds(filename: &str) -> Result<f64> {
    if filename.len() < 25 {
        bail!("filename too short to parse");
    }

    macro_rules! parse {
        ($source:expr, $range:expr, $to:ty, $e:expr) => {
            $source[$range]
                .parse::<$to>()
                .context(concat!("failed to parse ", $e))?
        };
    }

    let ts = &filename[12..25];
    let year = parse!(ts, 0..4, i16, "year");
    let month = parse!(ts, 4..6, i8, "month");
    let day = parse!(ts, 6..8, i8, "day");
    let hour = parse!(ts, 9..11, i8, "hour");
    let minute = parse!(ts, 11..13, i8, "minute");

    let zdt = DateTime::new(year, month, day, hour, minute, 0, 0)
        .context("invalid datetime")?
        .to_zoned(TimeZone::UTC)
        .context("failed to get zoned datetime")?;
    Ok(zdt.timestamp().as_second() as f64)
}

/// Same as [`geotiff_to_points`], but with a third dimension.
#[allow(clippy::cast_precision_loss)]
#[must_use]
pub fn geotiff_to_3d_points(
    geotiff: &geotiff::GeoTiff,
    epoch_seconds: f64,
    id_offset: Option<usize>,
) -> Vec<DataPoint3D<f64>> {
    let id_offset = id_offset.unwrap_or(0);
    let extent = geotiff.model_extent();
    let width = geotiff.raster_width;
    let height = geotiff.raster_height;

    let min_x = extent.min().x;
    let min_y = extent.min().y;
    let max_x = extent.max().x;
    let max_y = extent.max().y;

    let dx = (max_x - min_x) / width as f64;
    let dy = (max_y - min_y) / height as f64;

    let mut points = Vec::with_capacity(width * height);
    for row in 0..height {
        for col in 0..width {
            let x = (col as f64 + 0.5).mul_add(dx, min_x);
            let y = (row as f64 + 0.5).mul_add(-dy, max_y);
            let id = id_offset + row * width + col;
            points.push(DataPoint3D::new(id, Point3D::new(x, y, epoch_seconds)));
        }
    }

    points
}

/// Convert a slice of `(GeoTiff, filename)` pairs into a single flat list of
/// 3D points, parsing the timestamp from each filename. IDs are offset per
/// file to avoid collisions.
///
/// # Errors
///
/// Returns an error if a filename cannot be parsed into a valid timestamp.
pub fn geotiffs_to_3d_points(
    geotiffs: &[(&geotiff::GeoTiff, &str)],
) -> Result<Vec<DataPoint3D<f64>>> {
    let mut points = Vec::new();
    let mut id_offset = 0;
    for &(tiff, filename) in geotiffs {
        let time = filename_to_epoch_seconds(filename)
            .with_context(|| format!("failed to parse time from {filename}"))?;
        let tiff_size = tiff.raster_width * tiff.raster_height;
        let mut tiff_points = geotiff_to_3d_points(tiff, time, Some(id_offset));
        assert_eq!(tiff_size, tiff_points.len());
        id_offset += tiff_size;
        points.append(&mut tiff_points);
    }
    Ok(points)
}

#[cfg(test)]
mod tests {
    use super::{geotiff_to_points, geotiffs_to_points};
    use crate::quad_tree::QuadTree;
    use color_eyre::Result;
    use geotiff::GeoTiff;
    use std::fs::File;

    #[test]
    fn read_first_geotiff_into_tree() -> Result<()> {
        let path = "dwd-geotiff-epsg4326/YW_2017.002_20220101_0000.tif";
        let file = File::open(path)?;
        let tiff = [GeoTiff::read(file)?];
        let tree = QuadTree::from_geotiffs(&tiff, 10, 10)?;
        let tiff = &tiff[0];
        assert_eq!(tree.len(), tiff.raster_width * tiff.raster_height);
        Ok(())
    }

    #[test]
    fn geotiff_points_have_north_up_orientation() -> Result<()> {
        let path = "dwd-geotiff-epsg4326/YW_2017.002_20220101_0000.tif";
        let file = File::open(path)?;
        let tiff = GeoTiff::read(file)?;
        let points = geotiff_to_points(&tiff, None);
        let extent = tiff.model_extent();
        let min = extent.min();
        let max = extent.max();

        let first = points.first().expect("empty raster");
        let last = points.last().expect("empty raster");

        // Row 0 (first pixel) should be near the northern/top edge (max y).
        // The last row should be near the southern/bottom edge (min y).
        assert!(
            first.point.y > last.point.y,
            "expected first point to be north of last point"
        );
        assert!(first.point.y > min.y);
        assert!(first.point.y < max.y);
        assert!(last.point.y > min.y);
        assert!(last.point.y < max.y);
        assert!(first.point.x > min.x);
        assert!(first.point.x < max.x);
        Ok(())
    }

    #[test]
    fn read_multiple_geotiffs_into_one_tree() -> Result<()> {
        let paths = [
            "dwd-geotiff-epsg4326/YW_2017.002_20220101_0000.tif",
            "dwd-geotiff-epsg4326/YW_2017.002_20220101_0005.tif",
        ];
        let tiffs = paths
            .iter()
            .map(|path| {
                let file = File::open(path)?;
                GeoTiff::read(file).map_err(color_eyre::eyre::Error::from)
            })
            .collect::<Result<Vec<_>>>()?;

        let points = geotiffs_to_points(&tiffs);
        let expected_len: usize = tiffs.iter().map(|t| t.raster_width * t.raster_height).sum();
        assert_eq!(points.len(), expected_len);

        let tree = QuadTree::from_geotiffs(&tiffs, 10, 10)?;
        assert_eq!(tree.len(), expected_len);
        Ok(())
    }
}
