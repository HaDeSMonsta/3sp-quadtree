use crate::thread_pool;
use color_eyre::Result;
use color_eyre::eyre::Context;
use quadtree::oc_tree::DataPoint3D;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tracing::{debug, info, instrument};

#[instrument]
#[allow(clippy::type_complexity)]
pub fn get_points() -> Result<(Vec<DataPoint3D<f64>>, HashMap<usize, f32>)> {
    let tiffs = thread_pool()?
        .install(crate::read_tiffs)
        .context("failed to read tiffs")?;
    debug!("created {} tiff structs", tiffs.len());
    info!("parsed tiffs");

    let tiff_refs = tiffs
        .iter()
        .map(|(t, f)| (t, f.as_str()))
        .collect::<Vec<_>>();

    let res = crate::geotiff_helper::geotiffs_to_3d_points_mapping(&tiff_refs)
        .context("failed to parse geotiffs to 3d points")?;

    Ok(res)
}

#[instrument(skip(points))]
pub fn build_oct(cap: usize, depth: usize, points: Vec<DataPoint3D<f64>>) -> Result<Duration> {
    let start = Instant::now();

    debug!("benching build octree");
    quadtree::oc_tree::OcTree::from_points(points, cap, depth)
        .context("failed to create octree")?;

    let end = start.elapsed();
    Ok(end)
}
