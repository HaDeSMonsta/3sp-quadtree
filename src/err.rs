use crate::geometry::{Coord, Point, Rect};
use crate::tree::DataPoint;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GeometryErr<T: Coord> {
    #[error(
        "lower ({lower_x}, {lower_y}) must be lower-left then upper ({upper_x}, {upper_y})",
        lower_x = lower.x,
        lower_y = lower.y,
        upper_x = upper.x,
        upper_y = upper.y,
    )]
    IncorrectOrd { lower: Point<T>, upper: Point<T> },
}

#[derive(Debug, Error)]
pub enum TreeErr<T: Coord> {
    #[error(
        "tried to insert ({}, {}), id={}, into ({}, {})-({}, {})",
        data_point.point.x,
        data_point.point.y,
        data_point.id,
        rect.lower.x,
        rect.lower.y,
        rect.upper.x,
        rect.upper.y
    )]
    InsertionOutOfBounds {
        rect: Rect<T>,
        data_point: DataPoint<T>,
    },
    #[error("tried to construct tree from to few point ({0}), needs at least 2")]
    InsufficientPoints(usize),
    #[error("max depth ({0}) reached")]
    MaxDepthReached(usize),
    #[error("this should never happen: {0}")]
    Internal(String),
}
