use crate::geometry::{Coord, Rect};
use crate::tree::{DataPoint, QuadTree};

impl<T: Coord> QuadTree<T> {
    pub fn range(&self, query: &Rect<T>) -> Vec<DataPoint<T>> {
        let mut result = Vec::new();
        self.range_inner(query, &mut result);
        result
    }

    fn range_inner(&self, query: &Rect<T>, result: &mut Vec<DataPoint<T>>) {
        if !rects_intersect(&self.area, query) {
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
                result.push(data_point.clone());
            }
        }
    }
}

#[inline]
fn rects_intersect<T: Coord>(a: &Rect<T>, b: &Rect<T>) -> bool {
    a.lower.x <= b.upper.x
        && a.upper.x >= b.lower.x
        && a.lower.y <= b.upper.y
        && a.upper.y >= b.lower.y
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::{Point, Rect};

    fn make_tree() -> QuadTree<f64> {
        QuadTree::from_coords(0.0, 0.0, 100.0, 100.0, 4, 10).unwrap()
    }

    fn insert(tree: &mut QuadTree<f64>, id: usize, x: f64, y: f64) {
        tree.insert(DataPoint::new_coords(id, x, y)).unwrap();
    }

    fn rect(lower_x: f64, lower_y: f64, upper_x: f64, upper_y: f64) -> Rect<f64> {
        Rect::new(Point::new(lower_x, lower_y), Point::new(upper_x, upper_y)).unwrap()
    }

    #[test]
    fn range_returns_points_inside_query_rect() {
        let mut tree = make_tree();
        insert(&mut tree, 0, 10.0, 10.0);
        insert(&mut tree, 1, 20.0, 20.0);
        insert(&mut tree, 2, 90.0, 90.0);

        let result = tree.range(&rect(0.0, 0.0, 25.0, 25.0));
        let ids: Vec<usize> = result.iter().map(DataPoint::id).collect();

        assert_eq!(ids, vec![0, 1]);
    }

    #[test]
    fn range_returns_empty_when_no_points_match() {
        let mut tree = make_tree();
        insert(&mut tree, 0, 10.0, 10.0);
        insert(&mut tree, 1, 20.0, 20.0);

        let result = tree.range(&rect(50.0, 50.0, 60.0, 60.0));

        assert!(result.is_empty());
    }

    #[test]
    fn range_includes_boundary_points() {
        let mut tree = make_tree();
        insert(&mut tree, 0, 10.0, 10.0);
        insert(&mut tree, 1, 20.0, 20.0);
        insert(&mut tree, 2, 30.0, 30.0);

        let result = tree.range(&rect(10.0, 10.0, 20.0, 20.0));
        let ids: Vec<usize> = result.iter().map(DataPoint::id).collect();

        assert_eq!(ids, vec![0, 1]);
    }

    #[test]
    fn range_empty_tree_returns_empty() {
        let tree = make_tree();

        let result = tree.range(&rect(0.0, 0.0, 100.0, 100.0));

        assert!(result.is_empty());
    }

    #[test]
    fn range_can_query_partial_tree_area() {
        let mut tree = make_tree();
        insert(&mut tree, 0, 1.0, 1.0);
        insert(&mut tree, 1, 50.0, 50.0);
        insert(&mut tree, 2, 99.0, 99.0);

        let result = tree.range(&rect(90.0, 90.0, 110.0, 110.0));
        let ids: Vec<usize> = result.iter().map(DataPoint::id).collect();

        assert_eq!(ids, vec![2]);
    }

    #[test]
    fn range_integer_coords() {
        let mut tree = QuadTree::from_coords(0, 0, 100, 100, 4, 10).unwrap();
        tree.insert(DataPoint::new_coords(0, 10, 10)).unwrap();
        tree.insert(DataPoint::new_coords(1, 20, 20)).unwrap();
        tree.insert(DataPoint::new_coords(2, 90, 90)).unwrap();

        let query = Rect::new(Point::new(0, 0), Point::new(25, 25)).unwrap();
        let result = tree.range(&query);
        let ids: Vec<usize> = result.iter().map(DataPoint::id).collect();

        assert_eq!(ids, vec![0, 1]);
    }
}
