use crate::geometry::{Coord, Point};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GeometryErr<T: Coord> {
    #[error("x/y for {lower:?} must be lower then {upper:?}")]
    IncorrectOrd { lower: Point<T>, upper: Point<T> },
}
