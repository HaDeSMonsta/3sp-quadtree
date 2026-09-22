use color_eyre::Result;
use color_eyre::eyre::{Context, bail};
use jiff::civil::DateTime;
use jiff::tz::TimeZone;
use quadtree::geometry::Point3D;
use quadtree::oc_tree::DataPoint3D;
use rayon::prelude::*;
use std::collections::HashMap;
use std::sync::Mutex;

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

#[allow(clippy::cast_precision_loss)]
pub fn geotiff_to_3d_points_mapping<S: ::std::hash::BuildHasher>(
    geotiff: &geotiff::GeoTiff,
    epoch_seconds: f64,
    id_offset: Option<usize>,
    map: &Mutex<HashMap<usize, f32, S>>,
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

            match geotiff.get_value_at::<f32>(&geo_types::Coord { x, y }, 0) {
                Some(0.0) => {}
                Some(val) => {
                    let mut guard = map.lock().expect("we won't panic");
                    guard.insert(id, val);
                }
                None => panic!("no value @ {x}/{y}"),
            }
        }
    }

    points
}

#[allow(clippy::type_complexity)] // Yeah, this is not good
pub fn geotiffs_to_3d_points_mapping(
    geotiffs: &[(&geotiff::GeoTiff, &str)],
) -> Result<(Vec<DataPoint3D<f64>>, HashMap<usize, f32>)> {
    let map = Mutex::new(HashMap::new());

    crate::thread_pool()?.install(|| {
        let sizes = geotiffs
            .iter()
            .map(|(tiff, _)| tiff.raster_width * tiff.raster_height)
            .collect::<Vec<_>>();
        let mut offset = 0;
        let offsets = sizes
            .iter()
            .map(|size| {
                let cur = offset;
                offset += size;
                cur
            })
            .collect::<Vec<_>>();

        let tiff_points = geotiffs
            .par_iter()
            .enumerate()
            .map(|(i, &(tiff, filename))| {
                let time = filename_to_epoch_seconds(filename)
                    .with_context(|| format!("failed to parse time from {filename}"))?;
                let points = geotiff_to_3d_points_mapping(tiff, time, Some(offsets[i]), &map);
                Ok(points)
            })
            .collect::<Result<Vec<_>>>()?;

        Ok((
            tiff_points.into_iter().flatten().collect(),
            map.into_inner().context("surely, this won't happen")?,
        ))
    })
}
