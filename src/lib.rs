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

mod err;
pub mod geometry;
mod knn;
mod pip;
mod range;
pub mod tree;

pub use geometry::{Point, Rect};
pub use pip::Polygon;
pub use tree::{DataPoint, QuadTree};
