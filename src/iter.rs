use crate::geometry::Coord;
use crate::oc_tree::{DataPoint3D, OcNode, OcTree};
use crate::quad_tree::{DataPoint2D, QuadTree};
use std::iter::FusedIterator;

/// A single frame on the DFS stack while iterating a tree node.
///
/// `N` is the concrete node type (`QuadTree<T>` or `OcNode<T>`); the traversal
/// logic is identical for both, so only the node type differs per tree.
struct Frame<'a, N> {
    node: &'a N,
    point_idx: usize,
    child_idx: usize,
}

macro_rules! impl_dfs_iter {
    ($iter:ident, $into_iter:ident, $node_ty:ident, $point_ty:ident) => {
        /// Pre-order DFS iterator over the points of a tree.
        ///
        /// A node's own points are yielded first, then its children are visited
        /// in order. Internal nodes carry no points and are skipped.
        pub struct $iter<'a, T: Coord> {
            stack: Vec<Frame<'a, $node_ty<T>>>,
            remaining: usize,
        }

        impl<'a, T: Coord> $node_ty<T> {
            pub fn iter(&'a self) -> $iter<'a, T> {
                $iter {
                    stack: vec![Frame {
                        node: self,
                        point_idx: 0,
                        child_idx: 0,
                    }],
                    remaining: self.len,
                }
            }
        }

        impl<'a, T: Coord> Iterator for $iter<'a, T> {
            type Item = &'a $point_ty<T>;

            fn next(&mut self) -> Option<Self::Item> {
                loop {
                    let frame = self.stack.last_mut()?;

                    if frame.point_idx < frame.node.points.len() {
                        let point = &frame.node.points[frame.point_idx];
                        frame.point_idx += 1;
                        self.remaining -= 1;
                        return Some(point);
                    }

                    let Some(children) = &frame.node.children else {
                        self.stack.pop();
                        continue;
                    };

                    if frame.child_idx < children.len() {
                        let child = &children[frame.child_idx];
                        frame.child_idx += 1;
                        self.stack.push(Frame {
                            node: child,
                            point_idx: 0,
                            child_idx: 0,
                        });
                        continue;
                    }

                    self.stack.pop();
                }
            }

            fn size_hint(&self) -> (usize, Option<usize>) {
                (self.remaining, Some(self.remaining))
            }
        }

        impl<'a, T: Coord> ExactSizeIterator for $iter<'a, T> {}
        impl<'a, T: Coord> FusedIterator for $iter<'a, T> {}

        /// Consuming pre-order DFS iterator over the points of a tree.
        ///
        /// Owns the nodes it traverses, popping points and children out of them.
        pub struct $into_iter<T: Coord> {
            stack: Vec<$node_ty<T>>,
            remaining: usize,
        }

        impl<T: Coord> Iterator for $into_iter<T> {
            type Item = $point_ty<T>;

            fn next(&mut self) -> Option<Self::Item> {
                loop {
                    let node = self.stack.last_mut()?;

                    if let Some(point) = node.points.pop() {
                        self.remaining -= 1;
                        return Some(point);
                    }

                    let Some(children) = node.children.take() else {
                        self.stack.pop();
                        continue;
                    };

                    self.stack.pop();
                    for mut child in children.into_iter().rev() {
                        child.points.reverse();
                        self.stack.push(child);
                    }
                }
            }

            fn size_hint(&self) -> (usize, Option<usize>) {
                (self.remaining, Some(self.remaining))
            }
        }

        impl<T: Coord> ExactSizeIterator for $into_iter<T> {}
        impl<T: Coord> FusedIterator for $into_iter<T> {}

        impl<T: Coord> IntoIterator for $node_ty<T> {
            type Item = $point_ty<T>;
            type IntoIter = $into_iter<T>;

            fn into_iter(mut self) -> Self::IntoIter {
                self.points.reverse();
                let remaining = self.len;
                $into_iter {
                    stack: vec![self],
                    remaining,
                }
            }
        }
    };
}

impl_dfs_iter!(QuadIter, QuadIntoIter, QuadTree, DataPoint2D);
impl_dfs_iter!(OcIter, OcIntoIter, OcNode, DataPoint3D);

impl<'a, T: Coord> IntoIterator for &'a QuadTree<T> {
    type Item = &'a DataPoint2D<T>;
    type IntoIter = QuadIter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, T: Coord> OcTree<T> {
    pub fn iter(&'a self) -> OcIter<'a, T> {
        self.root.iter()
    }
}

impl<'a, T: Coord> IntoIterator for &'a OcTree<T> {
    type Item = &'a DataPoint3D<T>;
    type IntoIter = OcIter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a, T: Coord> IntoIterator for &'a OcNode<T> {
    type Item = &'a DataPoint3D<T>;
    type IntoIter = OcIter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<T: Coord> IntoIterator for OcTree<T> {
    type Item = DataPoint3D<T>;
    type IntoIter = OcIntoIter<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.root.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_quad() -> QuadTree<f64> {
        QuadTree::from_coords(0.0, 0.0, 100.0, 100.0, 4, 10).unwrap()
    }

    fn insert_quad(tree: &mut QuadTree<f64>, id: usize, x: f64, y: f64) {
        tree.insert(DataPoint2D::new_coords(id, x, y)).unwrap();
    }

    fn make_oct() -> OcTree<f64> {
        OcTree::from_coords(0.0, 0.0, 0.0, 100.0, 100.0, 100.0, 4, 10).unwrap()
    }

    fn insert_oct(tree: &mut OcTree<f64>, id: usize, x: f64, y: f64, z: f64) {
        tree.insert(DataPoint3D::new_coords(id, x, y, z)).unwrap();
    }

    #[test]
    fn quad_iter_yields_points_in_insertion_order() {
        let mut tree = make_quad();
        insert_quad(&mut tree, 0, 10.0, 10.0);
        insert_quad(&mut tree, 1, 20.0, 20.0);
        insert_quad(&mut tree, 2, 90.0, 90.0);

        let ids: Vec<usize> = tree.iter().map(|dp| dp.id).collect();
        assert_eq!(ids, vec![0, 1, 2]);
    }

    #[test]
    fn quad_iter_yields_all_points_when_subdivided() {
        let mut tree = make_quad();
        for id in 0..10 {
            let x = f64::from(u8::try_from(id).unwrap()).mul_add(10.0, 1.0);
            insert_quad(&mut tree, id, x, x);
        }

        let mut ids: Vec<usize> = tree.iter().map(|dp| dp.id).collect();
        ids.sort_unstable();
        assert_eq!(ids, (0..10).collect::<Vec<_>>());
    }

    #[test]
    fn quad_iter_empty_tree_yields_nothing() {
        let tree = make_quad();
        assert_eq!(tree.iter().count(), 0);
    }

    #[test]
    fn quad_into_iter_for_ref_works() {
        let mut tree = make_quad();
        insert_quad(&mut tree, 7, 50.0, 50.0);

        let ids: Vec<usize> = tree.iter().map(|dp| dp.id).collect();
        assert_eq!(ids, vec![7]);
    }

    #[test]
    fn oct_iter_yields_points_in_insertion_order() {
        let mut tree = make_oct();
        insert_oct(&mut tree, 0, 10.0, 10.0, 10.0);
        insert_oct(&mut tree, 1, 20.0, 20.0, 20.0);
        insert_oct(&mut tree, 2, 90.0, 90.0, 90.0);

        let ids: Vec<usize> = tree.iter().map(|dp| dp.id).collect();
        assert_eq!(ids, vec![0, 1, 2]);
    }

    #[test]
    fn oct_iter_yields_all_points_when_subdivided() {
        let mut tree = make_oct();
        for id in 0..10 {
            let c = f64::from(u8::try_from(id).unwrap()).mul_add(10.0, 1.0);
            insert_oct(&mut tree, id, c, c, c);
        }

        let mut ids: Vec<usize> = tree.iter().map(|dp| dp.id).collect();
        ids.sort_unstable();
        assert_eq!(ids, (0..10).collect::<Vec<_>>());
    }

    #[test]
    fn oct_iter_empty_tree_yields_nothing() {
        let tree = make_oct();
        assert_eq!(tree.iter().count(), 0);
    }

    #[test]
    fn quad_into_iter_matches_borrowing_order() {
        let mut tree = make_quad();
        for id in 0..10 {
            let x = f64::from(u8::try_from(id).unwrap()).mul_add(10.0, 1.0);
            insert_quad(&mut tree, id, x, x);
        }

        let borrowed: Vec<usize> = tree.iter().map(|dp| dp.id).collect();
        let consumed: Vec<usize> = tree.into_iter().map(|dp| dp.id).collect();
        assert_eq!(consumed, borrowed);
    }

    #[test]
    fn quad_into_iter_empty_tree_yields_nothing() {
        let tree = make_quad();
        assert_eq!(tree.into_iter().count(), 0);
    }

    #[test]
    fn oct_into_iter_matches_borrowing_order() {
        let mut tree = make_oct();
        for id in 0..10 {
            let c = f64::from(u8::try_from(id).unwrap()).mul_add(10.0, 1.0);
            insert_oct(&mut tree, id, c, c, c);
        }

        let borrowed: Vec<usize> = tree.iter().map(|dp| dp.id).collect();
        let consumed: Vec<usize> = tree.into_iter().map(|dp| dp.id).collect();
        assert_eq!(consumed, borrowed);
    }

    #[test]
    fn oct_node_into_iter_works() {
        let mut tree = make_oct();
        insert_oct(&mut tree, 3, 50.0, 50.0, 50.0);

        let ids: Vec<usize> = tree.root.into_iter().map(|dp| dp.id).collect();
        assert_eq!(ids, vec![3]);
    }

    #[test]
    fn iter_size_hint_is_exact() {
        let mut tree = make_quad();
        for id in 0..10 {
            let x = f64::from(u8::try_from(id).unwrap()).mul_add(10.0, 1.0);
            insert_quad(&mut tree, id, x, x);
        }

        let mut it = tree.iter();
        assert_eq!(it.len(), 10);
        let mut yielded = 0;
        while let Some(_) = it.next() {
            yielded += 1;
            assert_eq!(it.len(), 10 - yielded);
        }
        assert_eq!(it.len(), 0);
    }

    #[test]
    fn into_iter_size_hint_is_exact() {
        let mut tree = make_oct();
        for id in 0..10 {
            let c = f64::from(u8::try_from(id).unwrap()).mul_add(10.0, 1.0);
            insert_oct(&mut tree, id, c, c, c);
        }

        let mut it = tree.into_iter();
        assert_eq!(it.len(), 10);
        let mut yielded = 0;
        while let Some(_) = it.next() {
            yielded += 1;
            assert_eq!(it.len(), 10 - yielded);
        }
        assert_eq!(it.len(), 0);
    }
}
