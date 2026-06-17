#![warn(clippy::all, clippy::pedantic, clippy::nursery)]
#![allow(
    clippy::unnecessary_debug_formatting,
    clippy::needless_borrow,
    clippy::needless_borrows_for_generic_args,
    clippy::used_underscore_binding,
    clippy::cognitive_complexity,
    clippy::too_many_lines,
    clippy::needless_raw_string_hashes,
    clippy::missing_errors_doc // TODO Remove and document
)]

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
pub mod oct;
pub mod oct_tree;
pub mod quad;
pub mod quad_tree;

pub use err::*;
pub use geometry::{Point2D, Rect};
pub use quad_tree::{DataPoint2D, QuadTree};
