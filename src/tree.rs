use crate::geometry::{Coord, Rect};

const INITIAL_VEC_CAP: usize = 64; // Eyeballed

#[derive(Clone, Debug)]
pub struct QuadTree<T: Coord> {
    area: Rect<T>,
    items: Vec<T>, // No smallvec, because only top lvl would be on the stack
    children: Option<Box<[QuadTree<T>; 4]>>,
    depth: usize,
    max_depth: usize,
}

impl<T: Coord> QuadTree<T> {
    pub fn new(rect: Rect<T>, max_depth: usize) -> Self {
        todo!()
    }
}
