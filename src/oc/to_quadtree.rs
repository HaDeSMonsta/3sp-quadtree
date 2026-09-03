use crate::geometry::{Coord, Point2D};
use crate::oc_tree::{OcNode, OcTree};
use crate::quad_tree::{DataPoint2D, QuadTree};
use color_eyre::Result;
use color_eyre::eyre::Context;

impl<T: Coord> OcTree<T> {
    /// Build a `QuadTree` from all points whose z-coordinate lies in `[z_lower, z_upper]`,
    /// dropping the z-coordinate.
    ///
    /// `level_cap` and `max_depth` default to this tree's values when `None`.
    ///
    /// # Errors
    ///
    /// Returns an error if fewer than two points fall into the range or the tree cannot be built.
    pub fn to_quadtree_in_z_range(
        &self,
        z_lower: T,
        z_upper: T,
        level_cap: Option<usize>,
        max_depth: Option<usize>,
    ) -> Result<QuadTree<T>> {
        let mut points = Vec::new();
        let level_cap = level_cap.unwrap_or(self.level_cap);
        let max_depth = max_depth.unwrap_or(self.max_depth);
        self.root
            .collect_2d_in_z_range(z_lower, z_upper, &mut points);
        QuadTree::from_points(points, level_cap, max_depth)
            .context("failed to build quadtree from octree z range")
    }
}

impl<T: Coord> OcNode<T> {
    fn collect_2d_in_z_range(&self, z_lower: T, z_upper: T, out: &mut Vec<DataPoint2D<T>>) {
        if self.area.lower.z > z_upper || self.area.upper.z < z_lower {
            return;
        }

        if let Some(children) = &self.children {
            for child in children.iter() {
                child.collect_2d_in_z_range(z_lower, z_upper, out);
            }
            return;
        }

        for data_point in &self.points {
            if data_point.point.z >= z_lower && data_point.point.z <= z_upper {
                out.push(DataPoint2D::new(
                    data_point.id,
                    Point2D::new(data_point.point.x, data_point.point.y),
                ));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::oc_tree::DataPoint3D;

    fn make_tree() -> OcTree<f64> {
        OcTree::from_coords(0.0, 0.0, 0.0, 100.0, 100.0, 100.0, 4, 10).unwrap()
    }

    fn insert(tree: &mut OcTree<f64>, id: usize, x: f64, y: f64, z: f64) {
        tree.insert(DataPoint3D::new_coords(id, x, y, z)).unwrap();
    }

    #[test]
    fn z_range_keeps_only_points_in_range() {
        let mut tree = make_tree();
        insert(&mut tree, 0, 10.0, 10.0, 10.0);
        insert(&mut tree, 1, 20.0, 20.0, 20.0);
        insert(&mut tree, 2, 30.0, 30.0, 30.0);
        insert(&mut tree, 3, 90.0, 90.0, 90.0);

        let qt = tree
            .to_quadtree_in_z_range(15.0, 35.0, Some(4), Some(10))
            .unwrap();
        let mut ids: Vec<usize> = qt.iter().map(|dp| dp.id).collect();
        ids.sort_unstable();
        assert_eq!(ids, vec![1, 2]);
    }

    #[test]
    fn z_range_is_inclusive_on_boundaries() {
        let mut tree = make_tree();
        insert(&mut tree, 0, 10.0, 10.0, 10.0);
        insert(&mut tree, 1, 20.0, 20.0, 20.0);
        insert(&mut tree, 2, 30.0, 30.0, 30.0);

        let qt = tree
            .to_quadtree_in_z_range(10.0, 20.0, Some(4), Some(10))
            .unwrap();
        let mut ids: Vec<usize> = qt.iter().map(|dp| dp.id).collect();
        ids.sort_unstable();
        assert_eq!(ids, vec![0, 1]);
    }

    #[test]
    fn z_range_works_when_subdivided() {
        let mut tree = make_tree();
        for id in 0..20 {
            let c = f64::from(u8::try_from(id).unwrap()).mul_add(5.0, 1.0);
            insert(&mut tree, id, c, c, c);
        }

        let qt = tree
            .to_quadtree_in_z_range(0.0, 50.0, Some(4), Some(10))
            .unwrap();
        assert_eq!(qt.len(), 10);
    }

    #[test]
    fn z_range_defaults_to_octree_caps() {
        let mut tree = make_tree();
        for id in 0..20 {
            let c = f64::from(u8::try_from(id).unwrap()).mul_add(5.0, 1.0);
            insert(&mut tree, id, c, c, c);
        }

        let qt = tree.to_quadtree_in_z_range(0.0, 50.0, None, None).unwrap();
        assert_eq!(qt.len(), 10);
    }

    #[test]
    fn z_range_without_points_errors() {
        let mut tree = make_tree();
        insert(&mut tree, 0, 10.0, 10.0, 10.0);
        insert(&mut tree, 1, 20.0, 20.0, 20.0);

        assert!(
            tree.to_quadtree_in_z_range(80.0, 90.0, Some(4), Some(10))
                .is_err()
        );
    }
}
