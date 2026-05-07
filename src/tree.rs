use crate::geometry::{Coord, Rect};
use smallvec::SmallVec;

const SMALLVEC_SIZE: usize = 64; // eyeballed

#[derive(Clone, Debug)]
pub struct QuadTree<T: Coord> {
    area: Rect<T>,
    items: SmallVec<[T; SMALLVEC_SIZE]>,
    children: Option<Box<[QuadTree<T>; 4]>>,
    depth: usize,
    max_depth: usize,
}

impl<T: Coord> QuadTree<T> {
    pub fn new(rect: Rect<T>, max_depth: usize) -> Self {
        todo!()
    }
}
