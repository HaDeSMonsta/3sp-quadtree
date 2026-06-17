use crate::geometry::{Coord, Point2D, Point3D};
use crate::oct_tree::OctTree;
use crate::quad::pip::Polygon;

impl<T: Coord> OctTree<T> {
    /// Point-in-polygon for 3D points: the x/y projection must be inside the
    /// polygon and z must lie within `[z_min, z_max]` (inclusive). The polygon
    /// is effectively extruded into a prism along the z axis.
    pub fn pip(&self, point: &Point3D<T>, polygon: &Polygon<T>, z_min: T, z_max: T) -> bool {
        if point.z < z_min || point.z > z_max {
            return false;
        }

        let projected = Point2D::new(point.x, point.y);
        polygon.contains(&projected)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::oct_tree::OctTree;
    use crate::quad::pip::Polygon;

    fn p2(x: f64, y: f64) -> Point2D<f64> {
        Point2D::new(x, y)
    }

    fn p3(x: f64, y: f64, z: f64) -> Point3D<f64> {
        Point3D::new(x, y, z)
    }

    fn square() -> Polygon<f64> {
        Polygon::new(vec![
            p2(0.0, 0.0),
            p2(10.0, 0.0),
            p2(10.0, 10.0),
            p2(0.0, 10.0),
        ])
        .unwrap()
    }

    fn tree() -> OctTree<f64> {
        OctTree::from_coords(0.0, 0.0, 0.0, 100.0, 100.0, 100.0, 4, 10).unwrap()
    }

    #[test]
    fn pip_inside_polygon_and_z_range() {
        let tree = tree();
        let polygon = square();

        assert!(tree.pip(&p3(5.0, 5.0, 5.0), &polygon, 0.0, 10.0));
    }

    #[test]
    fn pip_inside_polygon_but_below_z_range() {
        let tree = tree();
        let polygon = square();

        assert!(!tree.pip(&p3(5.0, 5.0, -1.0), &polygon, 0.0, 10.0));
    }

    #[test]
    fn pip_inside_polygon_but_above_z_range() {
        let tree = tree();
        let polygon = square();

        assert!(!tree.pip(&p3(5.0, 5.0, 11.0), &polygon, 0.0, 10.0));
    }

    #[test]
    fn pip_outside_polygon_within_z_range() {
        let tree = tree();
        let polygon = square();

        assert!(!tree.pip(&p3(15.0, 5.0, 5.0), &polygon, 0.0, 10.0));
    }

    #[test]
    fn pip_z_range_boundaries_are_inclusive() {
        let tree = tree();
        let polygon = square();

        assert!(tree.pip(&p3(5.0, 5.0, 0.0), &polygon, 0.0, 10.0));
        assert!(tree.pip(&p3(5.0, 5.0, 10.0), &polygon, 0.0, 10.0));
    }

    #[test]
    fn pip_integer_coords() {
        let tree = OctTree::from_coords(0, 0, 0, 100, 100, 100, 4, 10).unwrap();
        let polygon = Polygon::new(vec![
            Point2D::new(0, 0),
            Point2D::new(10, 0),
            Point2D::new(10, 10),
            Point2D::new(0, 10),
        ])
        .unwrap();

        assert!(tree.pip(&Point3D::new(5, 5, 5), &polygon, 0, 10));
        assert!(!tree.pip(&Point3D::new(15, 5, 5), &polygon, 0, 10));
        assert!(!tree.pip(&Point3D::new(5, 5, 20), &polygon, 0, 10));
    }
}
