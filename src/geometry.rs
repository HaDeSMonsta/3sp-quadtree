use crate::err::GeometryErr;
use color_eyre::Result;
use num_traits::{ConstOne, One, Zero};
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
                self + ((other - self) / (T::ONE + T::ONE))
            }
        }
    };
}

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
        + ConstOne
        + Send
        + Sync
        + 'static
);

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Point<T: Coord> {
    pub x: T,
    pub y: T,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rect<T: Coord> {
    pub lower: Point<T>,
    pub upper: Point<T>,
}

impl<T: Coord> Point<T> {
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
    /// use quadtree::geometry::{Point, Rect};
    ///
    /// let lower = Point::new(0, 0);
    /// let upper = Point::new(1, 1);
    ///
    /// let rect = Rect::new(lower, upper).expect("lower was not below upper");
    /// ```
    pub fn new(lower: Point<T>, upper: Point<T>) -> Result<Self, GeometryErr<T>> {
        if lower.x >= upper.x || lower.y >= upper.y {
            return Err(GeometryErr::IncorrectOrd { lower, upper });
        }
        Ok(Self { lower, upper })
    }

    pub fn from_coords(low_x: T, low_y: T, high_x: T, high_y: T) -> Result<Self, GeometryErr<T>> {
        let lower = Point::new(low_x, low_y);
        let upper = Point::new(high_x, high_y);

        Self::new(lower, upper)
    }

    pub fn contains(&self, point: &Point<T>) -> bool {
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
