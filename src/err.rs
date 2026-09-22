use crate::geometry::{Coord, Cuboid, Point2D, Point3D, Rect};
use crate::oc_tree::DataPoint3D;
use crate::quad_tree::DataPoint2D;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum GeometryErr2D<T: Coord> {
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
pub enum GeometryErr3D<T: Coord> {
    #[error(
        "lower ({lower_x}, {lower_y}, {lower_z}) must be lower-left-prev then upper ({upper_x}, {upper_y}, {upper_z})",
        lower_x = lower.x,
        lower_y = lower.y,
        lower_z = lower.z,
        upper_x = upper.x,
        upper_y = upper.y,
        upper_z = upper.z,
    )]
    IncorrectOrd {
        lower: Point3D<T>,
        upper: Point3D<T>,
    },
}

#[derive(Debug, Error)]
pub enum TreeErr2D<T: Coord> {
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

#[derive(Debug, Error)]
pub enum TreeErr3D<T: Coord> {
    #[error(
        "tried to insert ({}, {}, {}), id={}, into ({}, {}, {})-({}, {}, {})",
        data_point.point.x,
        data_point.point.y,
        data_point.point.z,
        data_point.id,
        rect.lower.x,
        rect.lower.y,
        rect.lower.z,
        rect.upper.x,
        rect.upper.y,
        rect.upper.z,
    )]
    InsertionOutOfBounds {
        rect: Cuboid<T>,
        data_point: DataPoint3D<T>,
    },
    #[error("tried to construct tree from ({0}) points, needs at least 2")]
    InsufficientPoints(usize),
}
