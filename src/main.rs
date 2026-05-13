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

    let min_x = data
        .iter()
        .map(|dp| dp.coords().0)
        .fold(f64::INFINITY, f64::min);

    let max_x = data
        .iter()
        .map(|dp| dp.coords().0)
        .fold(f64::NEG_INFINITY, f64::max);

    let min_y = data
        .iter()
        .map(|dp| dp.coords().1)
        .fold(f64::INFINITY, f64::min);

    let max_y = data
        .iter()
        .map(|dp| dp.coords().1)
        .fold(f64::NEG_INFINITY, f64::max);
    info!(min_x, min_y, max_x, max_y, size = data.len());

    let mut tree = QuadTree::from_coords(min_x, min_y, max_x, max_y, 100, 100)
        .context("failed to construct tree")?;

    let insert_start = Instant::now();
    for dp in &data {
        tree.insert(dp.clone())
            .with_context(|| format!("failed to insert {dp:?}"))?;
    }
    let insert_time = insert_start.elapsed();

    tree.print_structure();
    info!(tree_size = tree.len(), tree_nodes = tree.nodes());
    info!(?parsing_time, ?insert_time);

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
