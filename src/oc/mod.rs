use crate::geometry::Coord;
use crate::oc_tree::{DataPoint3D, OcTree};

mod knn;
mod pip;
mod range;
mod to_quadtree;

impl<T: Coord> OcTree<T> {
    pub fn get_by_id(&self, id: usize) -> Option<DataPoint3D<T>> {
        self.iter().find(|point| point.id == id).copied()
    }
}
