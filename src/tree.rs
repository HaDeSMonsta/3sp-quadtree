use crate::geometry::{Coord, Point, Rect};

const INITIAL_VEC_CAP: usize = 64; // Eyeballed

#[derive(Clone, Debug)]
pub struct QuadTree<T: Coord> {
    area: Rect<T>,
    points: Vec<Point<T>>, // No smallvec, because only top lvl would be on the stack
    children: Option<Box<[Self; 4]>>,
    depth: usize,
    max_depth: usize,
}

impl<T: Coord> QuadTree<T> {
    pub fn new(area: Rect<T>, max_depth: usize) -> Self {
        Self {
            area,
            points: Vec::with_capacity(INITIAL_VEC_CAP),
            children: None,
            depth: 0,
            max_depth,
        }
    }
}
