use crate::err::GeometryErr;
use color_eyre::Result;
use num_traits::{One, Zero};
use std::ops::{Add, Div, Mul, Sub};

pub trait Coord:
    Copy
    + Clone
    + PartialOrd
    + Add<Output = Self>
    + Sub<Output = Self>
    + Mul<Output = Self>
    + Div<Output = Self>
    + Zero
    + One
{
    #[must_use]
    fn mid(self, other: Self) -> Self;
}

impl<T> Coord for T
where
    T: Copy
        + Clone
        + PartialOrd
        + Add<Output = Self>
        + Sub<Output = Self>
        + Mul<Output = Self>
        + Div<Output = Self>
        + Zero
        + One,
{
    #[allow(clippy::suspicious_operation_groupings)] // Self::one() == other::one() == T::one()
    fn mid(self, other: Self) -> Self {
        self + ((other - self) / (T::one() + T::one()))
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Point<T: Coord> {
    x: T,
    y: T,
}

#[derive(Copy, Clone, Debug)]
pub struct Rect<T: Coord> {
    lower: Point<T>,
    upper: Point<T>,
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
}
