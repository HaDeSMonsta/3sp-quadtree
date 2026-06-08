use crate::geometry::{Coord, Point2D};
use crate::quad_tree::QuadTree;
use color_eyre::eyre::Result;
use color_eyre::eyre::bail;

// Currently stays here if used for more than PiP -> Move to geometry
#[derive(Clone, Debug)]
pub struct Polygon<T: Coord> {
    vertices: Vec<Point2D<T>>,
    min_x: T,
    min_y: T,
    max_x: T,
    max_y: T,
}

impl<T: Coord> Polygon<T> {
    pub fn new(vertices: Vec<Point2D<T>>) -> Result<Self> {
        if vertices.len() < 3 {
            bail!(
                "at least three vertices required, {} supplied",
                vertices.len()
            );
        }

        let mut min_x = vertices[0].x;
        let mut min_y = vertices[0].y;
        let mut max_x = vertices[0].x;
        let mut max_y = vertices[0].y;

        for point in &vertices[1..] {
            if point.x < min_x {
                min_x = point.x;
            }
            if point.x > max_x {
                max_x = point.x;
            }
            if point.y < min_y {
                min_y = point.y;
            }
            if point.y > max_y {
                max_y = point.y;
            }
        }

        let polygon = Self {
            vertices,
            min_x,
            min_y,
            max_x,
            max_y,
        };

        Ok(polygon)
    }

    #[inline]
    fn contains(&self, point: &Point2D<T>) -> bool {
        let n = self.vertices.len();

        if n < 3
            || point.x < self.min_x
            || point.x > self.max_x
            || point.y < self.min_y
            || point.y > self.max_y
        {
            return false;
        }

        let mut inside = false;
        let mut j = n - 1;

        for i in 0..n {
            let a = &self.vertices[j];
            let b = &self.vertices[i];

            if Self::point_on_segment(point, a, b) {
                return true;
            }

            let ay_gt = a.y > point.y;
            let by_gt = b.y > point.y;

            if ay_gt != by_gt {
                let lhs = (point.x - a.x) * (b.y - a.y);
                let rhs = (b.x - a.x) * (point.y - a.y);

                if if b.y > a.y { lhs < rhs } else { lhs > rhs } {
                    inside = !inside;
                }
            }

            j = i;
        }

        inside
    }

    #[inline]
    fn point_on_segment(point: &Point2D<T>, a: &Point2D<T>, b: &Point2D<T>) -> bool {
        let cross = (b.x - a.x) * (point.y - a.y) - (b.y - a.y) * (point.x - a.x);

        cross == T::zero()
            && point.x >= Self::min(a.x, b.x)
            && point.x <= Self::max(a.x, b.x)
            && point.y >= Self::min(a.y, b.y)
            && point.y <= Self::max(a.y, b.y)
    }

    #[inline]
    fn min(a: T, b: T) -> T {
        if a < b { a } else { b }
    }

    #[inline]
    fn max(a: T, b: T) -> T {
        if a > b { a } else { b }
    }
}

impl<T: Coord> QuadTree<T> {
    pub fn pip(&self, point: &Point2D<T>, polygon: &Polygon<T>) -> bool {
        polygon.contains(point)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::quad_tree::QuadTree;

    fn p(x: f64, y: f64) -> Point2D<f64> {
        Point2D::new(x, y)
    }

    fn square() -> Polygon<f64> {
        Polygon::new(vec![p(0.0, 0.0), p(10.0, 0.0), p(10.0, 10.0), p(0.0, 10.0)]).unwrap()
    }

    fn concave_l_shape() -> Polygon<f64> {
        Polygon::new(vec![
            p(0.0, 0.0),
            p(10.0, 0.0),
            p(10.0, 4.0),
            p(4.0, 4.0),
            p(4.0, 10.0),
            p(0.0, 10.0),
        ])
        .unwrap()
    }

    // --- Basic correctness ---

    #[test]
    fn pip_square_contains_inner_point() {
        let polygon = square();

        assert!(polygon.contains(&p(5.0, 5.0)));
    }

    #[test]
    fn pip_square_rejects_outside_point() {
        let polygon = square();

        assert!(!polygon.contains(&p(15.0, 5.0)));
    }

    #[test]
    fn pip_square_rejects_bbox_outside_point() {
        let polygon = square();

        assert!(!polygon.contains(&p(-1.0, 5.0)));
        assert!(!polygon.contains(&p(5.0, -1.0)));
        assert!(!polygon.contains(&p(11.0, 5.0)));
        assert!(!polygon.contains(&p(5.0, 11.0)));
    }

    #[test]
    fn pip_is_independent_of_vertex_order_for_simple_square() {
        let clockwise =
            Polygon::new(vec![p(0.0, 0.0), p(0.0, 10.0), p(10.0, 10.0), p(10.0, 0.0)]).unwrap();
        let counter_clockwise = square();

        let query = p(5.0, 5.0);

        assert!(clockwise.contains(&query));
        assert!(counter_clockwise.contains(&query));
    }

    // --- Boundary cases ---

    #[test]
    fn pip_treats_edge_as_inside() {
        let polygon = square();

        assert!(polygon.contains(&p(5.0, 0.0)));
        assert!(polygon.contains(&p(10.0, 5.0)));
        assert!(polygon.contains(&p(5.0, 10.0)));
        assert!(polygon.contains(&p(0.0, 5.0)));
    }

    #[test]
    fn pip_treats_vertices_as_inside() {
        let polygon = square();

        assert!(polygon.contains(&p(0.0, 0.0)));
        assert!(polygon.contains(&p(10.0, 0.0)));
        assert!(polygon.contains(&p(10.0, 10.0)));
        assert!(polygon.contains(&p(0.0, 10.0)));
    }

    #[test]
    fn pip_handles_points_close_to_horizontal_edges() {
        let polygon = square();

        assert!(polygon.contains(&p(5.0, 0.000_001)));
        assert!(polygon.contains(&p(5.0, 9.999_999)));
        assert!(!polygon.contains(&p(5.0, -0.000_001)));
        assert!(!polygon.contains(&p(5.0, 10.000_001)));
    }

    // --- Concave polygons ---

    #[test]
    fn pip_concave_polygon_contains_point_in_filled_area() {
        let polygon = concave_l_shape();

        assert!(polygon.contains(&p(2.0, 2.0)));
        assert!(polygon.contains(&p(2.0, 8.0)));
        assert!(polygon.contains(&p(8.0, 2.0)));
    }

    #[test]
    fn pip_concave_polygon_rejects_point_in_cutout() {
        let polygon = concave_l_shape();

        assert!(!polygon.contains(&p(6.0, 6.0)));
        assert!(!polygon.contains(&p(9.0, 9.0)));
    }

    #[test]
    fn pip_concave_polygon_treats_inner_corner_boundary_as_inside() {
        let polygon = concave_l_shape();

        assert!(polygon.contains(&p(4.0, 4.0)));
        assert!(polygon.contains(&p(7.0, 4.0)));
        assert!(polygon.contains(&p(4.0, 7.0)));
    }

    // --- Holes / doughnut-style composition ---

    #[test]
    fn pip_doughnut_can_be_composed_from_outer_and_hole() {
        let outer = square();
        let hole = Polygon::new(vec![p(3.0, 3.0), p(7.0, 3.0), p(7.0, 7.0), p(3.0, 7.0)]).unwrap();

        let in_shell = p(2.0, 5.0);
        let in_hole = p(5.0, 5.0);
        let outside = p(12.0, 5.0);

        assert!(outer.contains(&in_shell) && !hole.contains(&in_shell));
        assert!(outer.contains(&in_hole) && hole.contains(&in_hole));
        assert!(!outer.contains(&outside));
    }

    // --- Edge cases ---

    #[test]
    fn pip_rejects_too_few_vertices() {
        assert!(Polygon::<f64>::new(vec![]).is_err());
        assert!(Polygon::new(vec![p(0.0, 0.0)]).is_err());
        assert!(Polygon::new(vec![p(0.0, 0.0), p(1.0, 1.0)]).is_err());
    }

    #[test]
    fn pip_repeated_closing_vertex_still_works() {
        let polygon = Polygon::new(vec![
            p(0.0, 0.0),
            p(10.0, 0.0),
            p(10.0, 10.0),
            p(0.0, 10.0),
            p(0.0, 0.0),
        ])
        .unwrap();

        assert!(polygon.contains(&p(5.0, 5.0)));
        assert!(!polygon.contains(&p(15.0, 5.0)));
    }

    // --- Wrapper API ---

    #[test]
    fn quadtree_pip_delegates_to_polygon() {
        let tree = QuadTree::from_coords(0.0, 0.0, 100.0, 100.0, 4, 10).unwrap();
        let polygon = square();

        assert!(tree.pip(&p(5.0, 5.0), &polygon));
        assert!(!tree.pip(&p(15.0, 5.0), &polygon));
    }

    // --- Integer coordinates ---

    #[test]
    fn pip_integer_coords() {
        let polygon = Polygon::new(vec![
            Point2D::new(0, 0),
            Point2D::new(10, 0),
            Point2D::new(10, 10),
            Point2D::new(0, 10),
        ])
        .unwrap();

        assert!(polygon.contains(&Point2D::new(5, 5)));
        assert!(polygon.contains(&Point2D::new(10, 5)));
        assert!(!polygon.contains(&Point2D::new(15, 5)));
    }
}
