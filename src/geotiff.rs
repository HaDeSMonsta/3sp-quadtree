use crate::TiffMapErr;
use crate::geometry::{Point2D, Point3D};
use crate::oct_tree::DataPoint3D;
use crate::quad_tree::DataPoint2D;
use color_eyre::Result;
use color_eyre::eyre::{Context, bail};
use jiff::civil::DateTime;
use jiff::tz::TimeZone;
use std::collections::HashMap;

/// Convert a `GeoTiff` raster into a flat list of pixel-center 2D points.
///
/// Each point gets an `id` computed as `id_offset + row * raster_width + col`,
/// making IDs unique within a single `GeoTiff`. When merging points from multiple
/// `GeoTiff`s, pass a different `id_offset` for each to avoid collisions.
///
/// Coordinates are placed at pixel centers by linearly interpolating between the
/// corners of the model extent (`min_x`,`min_y`) → (`max_x`,`max_y`). The y-axis
/// is flipped so that row 0 maps to the northern (top) edge.
///
/// # Parameters
///
/// * `geotiff` - The source `GeoTiff` raster. Its raster dimensions and model
///   extent determine the number and spatial layout of the output points.
/// * `id_offset` - An optional base value added to every point's `id`. Use
///   `None` (equivalent to `0`) for a single `GeoTiff`, or `Some(n)` to shift IDs
///   when combining multiple rasters.
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

/// Convert a slice of `GeoTiff` rasters into a single flat list of pixel-center 2D points.
///
/// IDs are allocated in non-overlapping ranges: the first `GeoTiff` occupies IDs
/// `0 .. raster_width × raster_height`, the second picks up where the first left
/// off, and so on. Pass progressively increasing `id_offset` values to achieve
/// the same effect manually.
///
/// # Parameters
///
/// * `geotiffs` - A slice of `GeoTiff` rasters to flatten. Each raster's points are
///   appended in order, producing one contiguous list.
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

/// Look up the `GeoTiff` raster value at each point's coordinates and build an
/// `id → value` map.
///
/// For every point in the input slice, the function samples band `0` of the
/// `GeoTiff` at `(point.point.x, point.point.y)`. If a coordinate falls outside
/// the raster or the pixel is nodata, the function returns
/// [`TiffMapErr::MissingValue`]. Duplicate point IDs trigger
/// [`TiffMapErr::DuplicateId`].
///
/// # Parameters
///
/// * `geotiff` — An owned `GeoTiff` whose raster values are sampled.
/// * `points` — A slice of 2D points. Each point must carry a unique `id` that
///   will become the map key. Coordinates are expected to be in the same CRS as
///   the `GeoTiff`.
///
/// # Errors
///
/// Returns [`TiffMapErr::MissingValue`] if a point's coordinate cannot be
/// mapped to a valid raster pixel. Returns [`TiffMapErr::DuplicateId`] if two
/// points share the same `id`.
#[allow(dead_code)] // TODO Use function
pub fn map_points(
    geotiff: &geotiff::GeoTiff,
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
            .ok_or(TiffMapErr::MissingValue { x, y })?;
        if let Some(_old) = map.insert(id, val) {
            return Err(TiffMapErr::DuplicateId(id));
        }
    }

    Ok(map)
}

/// Parse a `DWD` filename of the form `YW_2017.002_YYYYMMDD_HHMM.tif` and return
/// the Unix epoch timestamp (seconds since 1970-01-01 00:00:00 UTC) as an `f64`.
///
/// The filename must be at least 25 characters long. The timestamp portion is
/// extracted from byte offsets 12..25 (`YYYYMMDD_HHMM`). If the string is too
/// short or the date/time components are invalid, an error is returned.
///
/// # Parameters
///
/// * `filename` — A `DWD`-style filename string containing an embedded UTC
///   timestamp in `YYYYMMDD_HHMM` format at positions 12–24.
///
/// # Errors
///
/// Returns an error if the filename is shorter than 25 characters, if any
/// date/time component fails to parse, or if the parsed fields do not form a
/// valid `DateTime`.
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

    #[allow(clippy::cast_precision_loss)] // I'm not happy about that one TODO Fix somehow
    Ok(zdt.timestamp().as_second() as f64)
}

/// Convert a `GeoTiff` raster into a flat list of pixel-center 3D points.
///
/// Each point carries `(x, y, time)` where `x` and `y` are the pixel-center
/// model coordinates (north-up y-axis) and `time` is the caller-supplied
/// `epoch_seconds`.  The point `id` is `id_offset + row * raster_width + col`,
/// unique within a single raster.  Pass different `id_offset` values when
/// merging rasters to avoid collisions.
///
/// # Parameters
///
/// * `geotiff` — The source `GeoTiff` raster whose spatial extent and dimensions
///   drive the pixel-center coordinate computation.
/// * `epoch_seconds` — A Unix timestamp (seconds since 1970-01-01 00:00:00 UTC)
///   assigned to every point in the output as the z / time dimension.
/// * `id_offset` — An optional base value added to every point's `id`. Use
///   `None` (equivalent to `0`) for a single raster, or `Some(n)` to shift IDs
///   when combining rasters.
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
/// pixel-center 3D points, parsing each filename's embedded `DWD` timestamp as
/// the z / time dimension.
///
/// IDs are allocated in non-overlapping ranges: the first pair occupies IDs
/// `0 .. raster_width × raster_height`, the second picks up where the first
/// left off, and so on.  Each filename is expected to follow the `DWD`
/// convention `YW_2017.002_YYYYMMDD_HHMM.tif`.
///
/// # Parameters
///
/// * `geotiffs` — A slice of `(&GeoTiff, &str)` tuples. The `GeoTiff` provides
///   spatial coordinates and dimensions; the string is a `DWD` filename from which
///   the UTC timestamp is extracted.
///
/// # Errors
///
/// Returns an error if any filename is too short, fails to parse, or contains
/// an invalid date/time.
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
