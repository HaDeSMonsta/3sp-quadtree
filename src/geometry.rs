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
    #[allow(clippy::suspicious_operation_groupings)] // Self::one() == other::one()
    fn mid(self, other: Self) -> Self {
        self + ((other - self) / (Self::one() + Self::one()))
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
    pub fn new(x: T, y: T) -> Self {
        Self { x, y }
    }
}

impl<T: Coord> Rect<T> {
    pub fn new(lower: Point<T>, upper: Point<T>) -> Result<Self, GeometryErr<T>> {
        if lower.x >= upper.x || lower.y >= upper.y {
            return Err(GeometryErr::IncorrectOrd { upper, lower });
        }
        Ok(Self { upper, lower })
    }
}
