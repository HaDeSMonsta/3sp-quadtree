use crate::err::{GeometryErr2D, GeometryErr3D};
use color_eyre::Result;
use num_traits::{One, Zero};
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Display};
use std::ops::{Add, Div, Mul, Sub};

macro_rules! impl_coord {
    ($($bound:tt)+) => {
        pub trait Coord:
            $($bound)+
        {
            #[must_use]
            fn mid(self, other: Self) -> Self;
        }

        impl<T> Coord for T
        where
            T: $($bound)+,
        {
            #[allow(clippy::suspicious_operation_groupings)] // Self::one() == other::one() == T::one()
            #[inline] // Should happen automatically, but whatever
            fn mid(self, other: Self) -> Self {
                self + ((other - self) / (T::one() + T::one()))
            }
        }
    };
}

// The const impl is just inline { 0.0/1.0 }, so no difference to the method traits and the method traits are less restrictive (I guess)
impl_coord!(
    Copy
        + Clone
        + Debug
        + Display
        + PartialEq
        + PartialOrd
        + Add<Output = Self>
        + Sub<Output = Self>
        + Mul<Output = Self>
        + Div<Output = Self>
        + Zero
        + One
        + Send
        + Sync
        + 'static
);

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Point2D<T: Coord> {
    pub x: T,
    pub y: T,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Point3D<T: Coord> {
    pub x: T,
    pub y: T,
    pub z: T,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive] // Makes it impossible to construct via Rect { } outside of this crate, so users have to use the new function which checks for validity
pub struct Rect<T: Coord> {
    pub lower: Point2D<T>,
    pub upper: Point2D<T>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct Cuboid<T: Coord> {
    pub lower: Point3D<T>,
    pub upper: Point3D<T>,
}

impl<T: Coord> Point2D<T> {
    pub const fn new(x: T, y: T) -> Self {
        Self { x, y }
    }
}

impl<T: Coord> Rect<T> {
    /// Generate Rect
    ///
    /// # Arguments
    ///
    /// * `lower`: Lower left bound
    /// * `upper`: Upper right bound
    ///
    /// returns: `Result<Rect<T>, GeometryErr<T>>`
    ///
    /// # Errors
    ///
    /// `GeometryErr` if the `lower` point is not below left of the `upper` point
    ///
    /// # Examples
    ///
    /// ```
    /// use quadtree::geometry::{Point2D, Rect};
    ///
    /// let lower = Point2D::new(0, 0);
    /// let upper = Point2D::new(1, 1);
    ///
    /// let rect = Rect::new(lower, upper).expect("lower was not below upper");
    /// ```
    pub fn new(lower: Point2D<T>, upper: Point2D<T>) -> Result<Self, GeometryErr2D<T>> {
        if lower.x >= upper.x || lower.y >= upper.y {
            return Err(GeometryErr2D::IncorrectOrd { lower, upper });
        }
        Ok(Self { lower, upper })
    }

    pub fn from_coords(low_x: T, low_y: T, high_x: T, high_y: T) -> Result<Self, GeometryErr2D<T>> {
        let lower = Point2D::new(low_x, low_y);
        let upper = Point2D::new(high_x, high_y);

        Self::new(lower, upper)
    }

    pub fn contains(&self, point: &Point2D<T>) -> bool {
        point.x >= self.lower.x
            && point.x <= self.upper.x
            && point.y >= self.lower.y
            && point.y <= self.upper.y
    }

    pub(crate) fn get_centers(&self) -> (T, T) {
        let center_x = self.lower.x.mid(self.upper.x);
        let center_y = self.lower.y.mid(self.upper.y);

        (center_x, center_y)
    }
}

impl<T: Coord> Point3D<T> {
    pub const fn new(x: T, y: T, z: T) -> Self {
        Self { x, y, z }
    }
}

impl<T: Coord> Cuboid<T> {
    pub fn new(lower: Point3D<T>, upper: Point3D<T>) -> Result<Self, GeometryErr3D<T>> {
        if lower.x >= upper.x || lower.y >= upper.y || lower.z >= upper.z {
            return Err(GeometryErr3D::IncorrectOrd { lower, upper });
        }

        Ok(Self { lower, upper })
    }

    pub fn from_coords(
        low_x: T,
        low_y: T,
        low_z: T,
        high_x: T,
        high_y: T,
        high_z: T,
    ) -> Result<Self, GeometryErr3D<T>> {
        let lower = Point3D::new(low_x, low_y, low_z);
        let upper = Point3D::new(high_x, high_y, high_z);

        Self::new(lower, upper)
    }

    pub fn contains(&self, point: &Point3D<T>) -> bool {
        point.x >= self.lower.x
            && point.x <= self.upper.x
            && point.y >= self.lower.y
            && point.y <= self.upper.y
            && point.z >= self.lower.z
            && point.z <= self.upper.z
    }

    pub(crate) fn get_centers(&self) -> (T, T, T) {
        let center_x = self.lower.x.mid(self.upper.x);
        let center_y = self.lower.y.mid(self.upper.y);
        let center_z = self.lower.z.mid(self.upper.z);

        (center_x, center_y, center_z)
    }
}
