use crate::geometry::{Coord, Cuboid};
use crate::oc_tree::{DataPoint3D, OcNode, OcTree};

impl<T: Coord> OcTree<T> {
    pub fn range(&self, query: &Cuboid<T>) -> Vec<&DataPoint3D<T>> {
        let mut result = Vec::new();
        self.root.range_inner(query, &mut result);
        result
    }
}

impl<T: Coord> OcNode<T> {
    fn range_inner<'a>(&'a self, query: &Cuboid<T>, result: &mut Vec<&'a DataPoint3D<T>>) {
        if !cuboids_intersect(&self.area, query) {
            return;
        }

        if let Some(children) = &self.children {
            for child in children.iter() {
                child.range_inner(query, result);
            }

            return;
        }

        for data_point in &self.points {
            if query.contains(&data_point.point) {
                result.push(data_point);
            }
        }
    }
}

#[inline]
fn cuboids_intersect<T: Coord>(a: &Cuboid<T>, b: &Cuboid<T>) -> bool {
    a.lower.x <= b.upper.x
        && a.upper.x >= b.lower.x
        && a.lower.y <= b.upper.y
        && a.upper.y >= b.lower.y
        && a.lower.z <= b.upper.z
        && a.upper.z >= b.lower.z
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::{Cuboid, Point3D};

    fn make_tree() -> OcTree<f64> {
        OcTree::from_coords(0.0, 0.0, 0.0, 100.0, 100.0, 100.0, 4, 10).unwrap()
    }

    fn insert(tree: &mut OcTree<f64>, id: usize, x: f64, y: f64, z: f64) {
        tree.insert(DataPoint3D::new_coords(id, x, y, z)).unwrap();
    }

    fn cuboid(
        lower_x: f64,
        lower_y: f64,
        lower_z: f64,
        upper_x: f64,
        upper_y: f64,
        upper_z: f64,
    ) -> Cuboid<f64> {
        Cuboid::new(
            Point3D::new(lower_x, lower_y, lower_z),
            Point3D::new(upper_x, upper_y, upper_z),
        )
        .unwrap()
    }

    #[test]
    fn range_returns_points_inside_query_cuboid() {
        let mut tree = make_tree();
        insert(&mut tree, 0, 10.0, 10.0, 10.0);
        insert(&mut tree, 1, 20.0, 20.0, 20.0);
        insert(&mut tree, 2, 90.0, 90.0, 90.0);

        let result = tree.range(&cuboid(0.0, 0.0, 0.0, 25.0, 25.0, 25.0));
        let ids: Vec<usize> = result.iter().map(|dp| dp.id).collect();

        assert_eq!(ids, vec![0, 1]);
    }

    #[test]
    fn range_filters_on_z() {
        let mut tree = make_tree();
        // Same x/y, different z
        insert(&mut tree, 0, 10.0, 10.0, 10.0);
        insert(&mut tree, 1, 10.0, 10.0, 90.0);

        let result = tree.range(&cuboid(0.0, 0.0, 0.0, 25.0, 25.0, 25.0));
        let ids: Vec<usize> = result.iter().map(|dp| dp.id).collect();

        assert_eq!(ids, vec![0]);
    }

    #[test]
    fn range_returns_empty_when_no_points_match() {
        let mut tree = make_tree();
        insert(&mut tree, 0, 10.0, 10.0, 10.0);
        insert(&mut tree, 1, 20.0, 20.0, 20.0);

        let result = tree.range(&cuboid(50.0, 50.0, 50.0, 60.0, 60.0, 60.0));

        assert!(result.is_empty());
    }

    #[test]
    fn range_includes_boundary_points() {
        let mut tree = make_tree();
        insert(&mut tree, 0, 10.0, 10.0, 10.0);
        insert(&mut tree, 1, 20.0, 20.0, 20.0);
        insert(&mut tree, 2, 30.0, 30.0, 30.0);

        let result = tree.range(&cuboid(10.0, 10.0, 10.0, 20.0, 20.0, 20.0));
        let ids: Vec<usize> = result.iter().map(|dp| dp.id).collect();

        assert_eq!(ids, vec![0, 1]);
    }

    #[test]
    fn range_empty_tree_returns_empty() {
        let tree = make_tree();

        let result = tree.range(&cuboid(0.0, 0.0, 0.0, 100.0, 100.0, 100.0));

        assert!(result.is_empty());
    }

    #[test]
    fn range_can_query_partial_tree_area() {
        let mut tree = make_tree();
        insert(&mut tree, 0, 1.0, 1.0, 1.0);
        insert(&mut tree, 1, 50.0, 50.0, 50.0);
        insert(&mut tree, 2, 99.0, 99.0, 99.0);

        let result = tree.range(&cuboid(90.0, 90.0, 90.0, 110.0, 110.0, 110.0));
        let ids: Vec<usize> = result.iter().map(|dp| dp.id).collect();

        assert_eq!(ids, vec![2]);
    }

    #[test]
    fn range_integer_coords() {
        let mut tree = OcTree::from_coords(0, 0, 0, 100, 100, 100, 4, 10).unwrap();
        tree.insert(DataPoint3D::new_coords(0, 10, 10, 10)).unwrap();
        tree.insert(DataPoint3D::new_coords(1, 20, 20, 20)).unwrap();
        tree.insert(DataPoint3D::new_coords(2, 90, 90, 90)).unwrap();

        let query = Cuboid::new(Point3D::new(0, 0, 0), Point3D::new(25, 25, 25)).unwrap();
        let result = tree.range(&query);
        let ids: Vec<usize> = result.iter().map(|dp| dp.id).collect();

        assert_eq!(ids, vec![0, 1]);
    }
}
