use crate::thread_pool;
use color_eyre::Result;
use color_eyre::eyre::Context;
use quadtree::oc_tree::DataPoint3D;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tracing::{debug, info, instrument};

#[derive(Clone, Debug)]
#[allow(dead_code)] // We probably don't need the ids at all
pub struct GetPointsData {
    pub points: Vec<DataPoint3D<f64>>,
    pub ids: HashMap<usize, f32>,
    pub read_time: Duration,
    pub parse_time: Duration,
}

#[instrument]
#[allow(clippy::type_complexity)]
pub fn get_points() -> Result<GetPointsData> {
    let read_start = Instant::now();
    let tiffs = thread_pool()?
        .install(crate::read_tiffs)
        .context("failed to read tiffs")?;
    debug!("created {} tiff structs", tiffs.len());
    info!("parsed tiff files");
    let read_dur = read_start.elapsed();

    let tiff_refs = tiffs
        .iter()
        .map(|(t, f)| (t, f.as_str()))
        .collect::<Vec<_>>();

    let map_start = Instant::now();
    let (points, ids) = crate::geotiff_helper::geotiffs_to_3d_points_mapping(&tiff_refs)
        .context("failed to parse geotiffs to 3d points")?;
    let map_dur = map_start.elapsed();

    let data = GetPointsData {
        points,
        ids,
        read_time: read_dur,
        parse_time: map_dur,
    };

    Ok(data)
}

#[instrument(skip(points))]
pub fn build_oct(cap: usize, depth: usize, points: Vec<DataPoint3D<f64>>) -> Result<Duration> {
    let start = Instant::now();

    debug!("benching build octree");
    let tree = quadtree::oc_tree::OcTree::from_points(points, cap, depth)
        .context("failed to create octree")?;

    let end = start.elapsed();
    let size = tree.size_bytes_formatted();
    dbg!(size);
    Ok(end)
}
