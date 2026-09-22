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

use crate::cli::{Args, Cmd};
use clap::{CommandFactory, Parser};
use color_eyre::Result;
use color_eyre::eyre::{Context, ContextCompat};
use geotiff::GeoTiff;
use quadtree::Point2D;
use quadtree::oc_tree::OcTree;
use quadtree::quad_tree::QuadTree;
use rayon::prelude::*;
use std::collections::HashMap;
use std::fs::{self, File};
use std::io;
use std::process::exit;
use std::time::Instant;
use tracing::{debug, info, instrument, subscriber};
use tracing_subscriber::FmtSubscriber;

mod cli;
mod geotiff_helper;
mod simra;

#[cfg(target_env = "musl")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

const BIN_NAME: &str = env!("CARGO_BIN_NAME");
const BIN_VERSION: &str = env!("CARGO_PKG_VERSION");
const SIMRA_DIR: &str = "simra/";
const TIFF_DIR: &str = "dwd-geotiff-epsg4326/";

#[allow(unreachable_code)]
fn main() -> Result<()> {
    // At some point it will probably hold a query
    let args = setup().context("setup failed")?;

    info!("{BIN_NAME} - {BIN_VERSION}");
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

fn read_tiffs() -> Result<Vec<(GeoTiff, String)>> {
    let tiffs = fs::read_dir(TIFF_DIR)
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

fn thread_pool() -> Result<rayon::ThreadPool> {
    rayon::ThreadPoolBuilder::new()
        .build()
        .context("failed to build rayon threadpool")
}
