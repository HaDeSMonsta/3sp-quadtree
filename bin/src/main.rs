#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(
    clippy::unnecessary_debug_formatting,
    clippy::needless_borrow,
    clippy::needless_borrows_for_generic_args,
    clippy::used_underscore_binding,
    clippy::cognitive_complexity,
    clippy::too_many_lines,
    clippy::needless_raw_string_hashes
)]

use crate::bench::{BenchKnnResult, BenchPipResult, BenchRangeResult};
use crate::cli::{Args, Benches, Cmd};
use clap::{CommandFactory, Parser};
use color_eyre::Result;
use color_eyre::eyre::{Context, ContextCompat};
use geotiff::GeoTiff;
use quadtree::Point2D;
use quadtree::geometry::Cuboid;
use quadtree::oc_tree::OcTree;
use quadtree::quad::pip::Polygon;
use quadtree::quad_tree::QuadTree;
use rand::SeedableRng;
use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rayon::prelude::*;
use serde_json::json;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io;
use std::path::Path;
use std::process::exit;
use std::time::Instant;
use tracing::{debug, info, instrument, subscriber};
use tracing_subscriber::FmtSubscriber;

mod bench;
mod cli;
mod geotiff_helper;
mod simra;

#[cfg(target_env = "musl")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

const BIN_NAME: &str = env!("CARGO_BIN_NAME");
const BIN_VERSION: &str = env!("CARGO_PKG_VERSION");
const SIMRA_DIR: &str = "simra/";
const TIFF_DIR: &str = "dwd-geotiff/";
const RUNS: usize = 30;
const SEED: u64 = 0x5EED_5EED_5EED_5EED;
const KNN_N: usize = 5;
const PIP_HALF: f64 = 0.01;
const RANGE_HALF: f64 = 0.05;
const Z_HALF: f64 = 1.0;

#[allow(unreachable_code)]
fn main() -> Result<()> {
    let args = setup().context("setup failed")?;

    info!("{BIN_NAME} - {BIN_VERSION}");

    if let Some(Cmd::Bench { target }) = &args.cmd {
        let results = match target {
            Benches::BuildOct => {
                let full_start = Instant::now();
                let data = bench::get_points().context("failed to get points")?;
                let (octree, oct_creation_dur) =
                    bench::build_oct(args.cap, args.depth, data.points)
                        .context("failed to bench build_oct")?;

                let mut candidates = data
                    .ids
                    .iter()
                    .map(|(&id, &value)| (id, value))
                    .collect::<Vec<_>>();
                assert!(
                    candidates.len() >= RUNS,
                    "expected at least {RUNS} candidates, found {}",
                    candidates.len()
                );
                // So the random shuffle has the same start
                candidates.sort_unstable_by_key(|&(id, _)| id);

                let mut rng = StdRng::seed_from_u64(SEED);
                candidates.shuffle(&mut rng);

                let mut source_points = Vec::with_capacity(RUNS);
                let mut knn_results = Vec::with_capacity(RUNS);
                let mut pip_results = Vec::with_capacity(RUNS);
                let mut range_results = Vec::with_capacity(RUNS);
                for (id, value) in candidates.into_iter().take(RUNS) {
                    let target = octree.get_by_id(id).context("valid id not found")?;
                    source_points.push(target);

                    let knn_start = Instant::now();
                    let points = octree.knn(target.point, KNN_N);
                    let knn_time = knn_start.elapsed();
                    // I'm sure this will always be called with sufficiently large datasets
                    assert_eq!(
                        KNN_N,
                        points.len(),
                        "expected {KNN_N} points, got {}",
                        points.len()
                    );

                    let contained_polygon = square(target.point.x, target.point.y, PIP_HALF)
                        .context("failed to build contained polygon")?;
                    let pip_start = Instant::now();
                    let pip_contained = octree.pip(
                        target.point,
                        &contained_polygon,
                        target.point.z - Z_HALF,
                        target.point.z + Z_HALF,
                    );
                    let pip_contained_time = pip_start.elapsed();
                    assert!(pip_contained, "pip not working");

                    let outside_polygon = square(
                        4.0f64.mul_add(PIP_HALF, target.point.x),
                        target.point.y,
                        PIP_HALF,
                    )
                    .context("failed to build outside polygon")?;
                    let pip_start = Instant::now();
                    let pip_not_contained = octree.pip(
                        target.point,
                        &outside_polygon,
                        target.point.z - Z_HALF,
                        target.point.z + Z_HALF,
                    );
                    let pip_not_contained_time = pip_start.elapsed();
                    assert!(!pip_not_contained, "pip not working");

                    let query = Cuboid::from_coords(
                        target.point.x - RANGE_HALF,
                        target.point.y - RANGE_HALF,
                        target.point.z - Z_HALF,
                        target.point.x + RANGE_HALF,
                        target.point.y + RANGE_HALF,
                        target.point.z + Z_HALF,
                    )
                    .context("failed to build range cuboid")?;
                    let range_start = Instant::now();
                    let range_points = octree.range(query);
                    let range_time = range_start.elapsed();

                    knn_results.push(BenchKnnResult {
                        point_id: id,
                        point_rain_value: value,
                        dur: knn_time,
                        points: points.into_iter().copied().collect(),
                    });

                    pip_results.push(BenchPipResult {
                        point_id: id,
                        point_rain_value: value,
                        contained_dur: pip_contained_time,
                        not_contained_dur: pip_not_contained_time,
                    });

                    range_results.push(BenchRangeResult {
                        point_id: id,
                        point_rain_value: value,
                        dur: range_time,
                        points: range_points.into_iter().copied().collect(),
                    });
                }

                let full_time = full_start.elapsed();
                json!({
                    "full_runtime": full_time,
                    "cap": args.cap,
                    "depth": args.depth,
                    "data read time": data.read_time,
                    "data parse time": data.parse_time,
                    "octree construction time": oct_creation_dur,
                    "octree size": octree.size_bytes(),
                    "source points": source_points,
                    "knn results": knn_results,
                    "knn n": KNN_N,
                    "pip results": pip_results,
                    "range results": range_results,
                })
            }
        };

        let data = json!({
            "type": target.bench_name(),
            "data": results,
        });
        store_report(&data).context("failed to save benchmark results")?;

        return Ok(());
    }

    let start = Instant::now();

    let quadtree = quadtree(args.cap, args.depth).context("failed to create quadtree")?;
    let (octree, ids) = octree(args.cap, args.depth).context("failed to create octree")?;

    let mut v = ids.into_iter().collect::<Vec<_>>();
    v.sort_unstable_by(|a, b| a.1.partial_cmp(&b.1).expect("we know no NaN").reverse());
    let max = v.swap_remove(0);
    let max = dbg!(max);

    let max_point = octree.get_by_id(max.0).context("we know any id is valid")?;
    dbg!(max_point);
    let nearest = quadtree.knn(Point2D::new(max_point.point.x, max_point.point.y), 10);
    dbg!(nearest);

    debug!("took {:?}", start.elapsed());

    Ok(())
}

fn square(center_x: f64, center_y: f64, half: f64) -> Result<Polygon<f64>> {
    let polygon = Polygon::new(vec![
        Point2D::new(center_x - half, center_y - half),
        Point2D::new(center_x + half, center_y - half),
        Point2D::new(center_x + half, center_y + half),
        Point2D::new(center_x - half, center_y + half),
    ])
    .context("failed to build square polygon")?;

    Ok(polygon)
}

fn read_tiffs() -> Result<Vec<(GeoTiff, String)>> {
    let mut tiffs = fs::read_dir(TIFF_DIR)
        .context("failed to read tiff dir")?
        .collect::<std::result::Result<Vec<_>, _>>()
        .context("failed to read dir entry")?
        .par_iter()
        .map(fs::DirEntry::path)
        .filter(|path| path.is_file() && path.extension().is_some_and(|e| e == "tif"))
        .map(|path| {
            let filename = path
                .file_name()
                .with_context(|| format!("no filename: {path:?}"))?
                .to_str()
                .with_context(|| format!("invalid filename: {path:?}"))?
                .to_owned();

            let file =
                File::open(path.as_path()).with_context(|| format!("failed to open {path:?}"))?;
            let tiff = GeoTiff::read(file)
                .with_context(|| format!("failed to read tiff data from {path:?}"))?;
            Ok((tiff, filename))
        })
        .collect::<Result<Vec<_>>>()
        .context("failed to create tiffs/paths from file names")?;
    tiffs.sort_unstable_by(|a, b| a.1.cmp(&b.1));
    Ok(tiffs)
}

fn setup() -> Result<Args> {
    color_eyre::install().context("failed to install :(")?;

    let args = Args::parse();

    let log_lvl = args
        .log_lvl
        .parse::<tracing::Level>()
        .with_context(|| format!("failed to parse {:?} to tracing::Level", args.log_lvl))?;
    let fmt_sub = FmtSubscriber::builder().with_max_level(log_lvl).finish();

    subscriber::set_global_default(fmt_sub).context("failed to set global subscriber")?;

    if let Some(cmd) = &args.cmd {
        match cmd {
            Cmd::Bench { target } => {
                debug!(?target, "benching");
            }
            Cmd::Complete { shell } => {
                let mut cmd = Args::command();
                clap_complete::generate(*shell, &mut cmd, BIN_NAME, &mut io::stdout());
                exit(0);
            }
        }
    }

    Ok(args)
}

#[instrument(skip_all)]
fn quadtree(cap: usize, depth: usize) -> Result<QuadTree<f64>> {
    let data = simra::parse(SIMRA_DIR).context("failed to parse simra data")?;

    info!("read {} simra points", data.len());

    let tree = QuadTree::from_points(data, cap, depth)
        .context("failed to construct tree from SimRa points")?;
    info!("created tree with {} elements", tree.len());

    Ok(tree)
}

#[instrument(skip_all)]
fn octree(cap: usize, depth: usize) -> Result<(OcTree<f64>, HashMap<usize, f32>)> {
    let tiffs = rayon::ThreadPoolBuilder::new()
        .build()
        .context("failed to build rayon threadpool")?
        .install(read_tiffs)
        .context("failed to read tiffs")?;

    info!("created {} tiff structs", tiffs.len());

    let tiff_refs = tiffs
        .iter()
        .map(|(t, f)| (t, f.as_str()))
        .collect::<Vec<_>>();

    let (points, map) = geotiff_helper::geotiffs_to_3d_points_mapping(&tiff_refs)
        .context("failed to parse geotiffs to 3d points")?;
    let treee =
        OcTree::from_points(points, cap, depth).context("failed to create tree from tiffs")?;
    info!("created tree with {} elements", treee.len());

    Ok((treee, map))
}

#[instrument(skip(data))]
fn store_report(data: &serde_json::Value) -> Result<()> {
    let result_dir = Path::new("results");
    let mut i = 0_usize;
    let final_path = loop {
        let result_file = result_dir.join(format!("benchmark_{BIN_NAME}_{i:02}.json"));
        if !fs::exists(&result_file)
            .with_context(|| format!("failed to check if result file {result_file:?} exists"))?
        {
            break result_file;
        }
        i += 1;
    };
    if i == 0 {
        fs::create_dir_all(&result_dir)
            .with_context(|| format!("failed to create result dir {result_dir:?}"))?;
    }

    let data_str =
        serde_json::to_string_pretty(&data).context("failed to parse data to pretty str")?;
    fs::write(&final_path, &data_str)
        .with_context(|| format!("failed to write parsed data to {final_path:?}"))?;

    debug!("saved data");
    Ok(())
}

fn thread_pool() -> Result<rayon::ThreadPool> {
    rayon::ThreadPoolBuilder::new()
        .build()
        .context("failed to build rayon threadpool")
}
