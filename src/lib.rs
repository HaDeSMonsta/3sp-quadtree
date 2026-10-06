#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(
    clippy::unnecessary_debug_formatting,
    clippy::needless_borrow,
    clippy::needless_borrows_for_generic_args,
    clippy::used_underscore_binding,
    clippy::cognitive_complexity,
    clippy::too_many_lines,
    clippy::needless_raw_string_hashes,
    clippy::missing_errors_doc, // TODO Remove and document
    clippy::missing_panics_doc, // Do I really care?
)]

use color_eyre::eyre::Context;
pub use err::*;
pub use geometry::{Point2D, Rect};
pub use quad_tree::{DataPoint2D, QuadTree};

macro_rules! diff {
    ($x:expr, $y:expr) => {
        if $x >= $y { $x - $y } else { $y - $x }
    };
}
macro_rules! axis_dist {
    ($p:expr, $lo:expr, $hi:expr) => {
        if $p < $lo {
            $lo - $p
        } else if $p > $hi {
            $p - $hi
        } else {
            T::zero()
        }
    };
}

mod err;
pub mod geometry;
mod iter;
pub mod oc;
pub mod oc_tree;
pub mod quad;
pub mod quad_tree;

/// Minimum number of points in a subtree before the bulk builder recurses in parallel.
const PARALLEL_THRESHOLD: usize = 2_usize.pow(13);

/// Format a byte count as a human readable binary size
pub(crate) fn format_bytes(bytes: usize) -> String {
    const UNITS: [&str; 6] = ["B", "KiB", "MiB", "GiB", "TiB", "PiB"];

    let mut unit = 0;
    let mut divisor = 1_usize;

    while bytes / divisor >= 1024 && unit < UNITS.len() - 1 {
        divisor *= 1024;
        unit += 1;
    }

    if unit == 0 {
        return format!("{bytes} B");
    }

    let whole = bytes / divisor;
    let frac = (bytes % divisor) * 100 / divisor;
    format!("{whole}.{frac:02} {}", UNITS[unit])
}

/// Helper function to set project specific values that are true for all used thread pools
fn thread_pool() -> color_eyre::Result<rayon::ThreadPool> {
    rayon::ThreadPoolBuilder::new()
        .build()
        .context("failed to build rayon threadpool")
}
