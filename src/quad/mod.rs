use crate::geometry::Coord;
use crate::{DataPoint2D, QuadTree};

mod knn;
pub mod pip;
mod range;

impl<T: Coord> QuadTree<T> {
    pub fn get_by_id(&self, id: usize) -> Option<DataPoint2D<T>> {
        self.iter().find(|point| point.id == id).copied()
    }
}
