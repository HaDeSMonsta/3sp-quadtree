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

use crate::bin_assets::cli::{Args, Cmd};
use crate::bin_assets::simra;
use clap::{CommandFactory, Parser};
use color_eyre::Result;
use color_eyre::eyre::Context;
use quadtree::geometry::Point;
use quadtree::tree::QuadTree;
use std::io;
use std::process::exit;
use std::time::Instant;
use tracing::{info, subscriber};
use tracing_subscriber::FmtSubscriber;

mod bin_assets;

#[cfg(target_env = "musl")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

const BIN_NAME: &str = env!("CARGO_PKG_NAME");
const BIN_VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() -> Result<()> {
    // At some point it will probably hold a query
    let _args = setup().context("setup failed")?;

    info!("{BIN_NAME} - {BIN_VERSION}");

    let parsing_start = Instant::now();
    let data = simra::parse("simra").context("failed to parse simra data")?;
    let parsing_time = parsing_start.elapsed();

    let insert_start = Instant::now();
    let tree =
        QuadTree::from_points(&data).context("failed to construct tree from SimRa points")?;
    let insert_time = insert_start.elapsed();

    // tree.print_structure();
    info!(tree_size = tree.len(), tree_nodes = tree.nodes());
    info!(?parsing_time, ?insert_time);

    let save_start = Instant::now();
    tree.save("tree.json")?;
    let save_time = save_start.elapsed();

    let load_start = Instant::now();
    let _new_tree: QuadTree<f64> = QuadTree::from_file("tree.json")?;
    let load_time = load_start.elapsed();

    info!(?save_time, ?load_time);

    let knn_start = Instant::now();
    let _knn = tree.knn(&Point::new(52.54906128, 13.42094266), 3);
    let knn_time = knn_start.elapsed();
    info!(?knn_time);
    // info!(?knn);

    Ok(())
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
