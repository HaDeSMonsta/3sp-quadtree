use crate::geometry::{Coord, Point2D, Rect};
use crate::quad_tree::DataPoint2D;
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
    IncorrectOrd {
        lower: Point2D<T>,
        upper: Point2D<T>,
    },
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
        data_point: DataPoint2D<T>,
    },
    #[error("tried to construct tree from ({0}) points, needs at least 2")]
    InsufficientPoints(usize),
}
