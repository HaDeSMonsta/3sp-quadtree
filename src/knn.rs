use crate::geometry::{Coord, Point, Rect};
use crate::tree::{DataPoint, QuadTree};
use std::cmp::Ordering;
use std::collections::BinaryHeap;

#[derive(Clone, Copy, Debug)]
struct BestNeighbour<'a, T: Coord> {
    dist2: T,
    data_point: &'a DataPoint<T>,
}

impl<T: Coord> PartialEq for BestNeighbour<'_, T> {
    fn eq(&self, other: &Self) -> bool {
        self.dist2 == other.dist2 && self.data_point.id == other.data_point.id
    }
}

impl<T: Coord> Eq for BestNeighbour<'_, T> {}

impl<T: Coord> PartialOrd for BestNeighbour<'_, T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: Coord> Ord for BestNeighbour<'_, T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.dist2
            .partial_cmp(&other.dist2)
            .unwrap_or(Ordering::Equal)
            .then_with(|| self.data_point.id.cmp(&other.data_point.id))
    }
}

#[derive(Clone, Copy, Debug)]
struct CandidateNode<'a, T: Coord> {
    min_dist2: T,
    node: &'a QuadTree<T>,
}

impl<T: Coord> PartialEq for CandidateNode<'_, T> {
    fn eq(&self, other: &Self) -> bool {
        self.min_dist2 == other.min_dist2
    }
}

impl<T: Coord> Eq for CandidateNode<'_, T> {}

impl<T: Coord> PartialOrd for CandidateNode<'_, T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: Coord> Ord for CandidateNode<'_, T> {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .min_dist2
            .partial_cmp(&self.min_dist2)
            .unwrap_or(Ordering::Equal)
    }
}

impl<T: Coord> QuadTree<T> {
    // Thoughts: Point because it doesn't have to be included
    pub fn knn<'a>(&'a self, query: &Point<T>, k: usize) -> Vec<&'a DataPoint<T>> {
        if k == 0 {
            return Vec::new();
        }

        let mut candidates = BinaryHeap::new();
        let mut best: BinaryHeap<BestNeighbour<'a, T>> = BinaryHeap::with_capacity(k + 1);

        candidates.push(CandidateNode {
            min_dist2: dist2_point_rect(query, &self.area),
            node: self,
        });

        while let Some(candidate) = candidates.pop() {
            if best.len() == k {
                // Safety: k > 0 && len == k
                let worst = unsafe { best.peek().unwrap_unchecked() };
                if candidate.min_dist2 >= worst.dist2 {
                    break;
                }
            }

            if let Some(children) = &candidate.node.children {
                for child in children.iter() {
                    let min_dist2 = dist2_point_rect(query, &child.area);

                    if best.len() < k || best.peek().is_some_and(|worst| min_dist2 < worst.dist2) {
                        candidates.push(CandidateNode {
                            min_dist2,
                            node: child,
                        });
                    }
                }
            } else {
                for data_point in &candidate.node.points {
                    let dist2 = dist2_point_point(query, &data_point.point);

                    if best.len() < k {
                        best.push(BestNeighbour { dist2, data_point });
                    } else if best.peek().is_some_and(|worst| dist2 < worst.dist2) {
                        best.pop();
                        best.push(BestNeighbour { dist2, data_point });
                    }
                }
            }
        }

        let mut result = best.into_vec();

        // Usually faster than stable + no alloc and we don't need stable
        result.sort_unstable_by(|a, b| {
            a.dist2
                .partial_cmp(&b.dist2)
                .unwrap_or(Ordering::Equal)
                .then_with(|| a.data_point.id.cmp(&b.data_point.id))
        });

        result
            .into_iter()
            .map(|neighbour| neighbour.data_point)
            .collect()
    }
}

#[inline]
fn dist2_point_point<T: Coord>(a: &Point<T>, b: &Point<T>) -> T {
    let dx = if a.x >= b.x { a.x - b.x } else { b.x - a.x };
    let dy = if a.y >= b.y { a.y - b.y } else { b.y - a.y };

    dx * dx + dy * dy
}

#[inline]
fn dist2_point_rect<T: Coord>(point: &Point<T>, rect: &Rect<T>) -> T {
    let dx = if point.x < rect.lower.x {
        rect.lower.x - point.x
    } else if point.x > rect.upper.x {
        point.x - rect.upper.x
    } else {
        T::zero()
    };

    let dy = if point.y < rect.lower.y {
        rect.lower.y - point.y
    } else if point.y > rect.upper.y {
        point.y - rect.upper.y
    } else {
        T::zero()
    };

    dx * dx + dy * dy
}

#[cfg(test)]
mod tests {
    use crate::geometry::Point;
    use crate::tree::{DataPoint, QuadTree};

    fn make_tree() -> QuadTree<f64> {
        QuadTree::from_coords(0.0, 0.0, 100.0, 100.0, 4, 10).unwrap()
    }

    fn insert(tree: &mut QuadTree<f64>, id: usize, x: f64, y: f64) {
        tree.insert(DataPoint::new_coords(id, x, y)).unwrap();
    }

    fn query(x: f64, y: f64) -> Point<f64> {
        Point::new(x, y)
    }

    // --- Basic correctness ---

    #[test]
    fn knn_returns_correct_nearest() {
        let mut tree = make_tree();
        insert(&mut tree, 0, 10.0, 10.0);
        insert(&mut tree, 1, 20.0, 20.0);
        insert(&mut tree, 2, 90.0, 90.0);

        let result = tree.knn(&query(12.0, 12.0), 1);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id(), 0);
    }

    #[test]
    fn knn_returns_k_results_sorted_by_distance() {
        let mut tree = make_tree();
        insert(&mut tree, 0, 10.0, 10.0); // closest
        insert(&mut tree, 1, 20.0, 20.0); // second
        insert(&mut tree, 2, 90.0, 90.0); // farthest

        let result = tree.knn(&query(5.0, 5.0), 2);

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].id(), 0);
        assert_eq!(result[1].id(), 1);
    }

    #[test]
    fn knn_returns_all_points_when_k_exceeds_tree_size() {
        let mut tree = make_tree();
        insert(&mut tree, 0, 10.0, 10.0);
        insert(&mut tree, 1, 50.0, 50.0);

        let result = tree.knn(&query(5.0, 5.0), 100);

        assert_eq!(result.len(), 2);
    }

    // --- Edge cases ---

    #[test]
    fn knn_k_zero_returns_empty() {
        let mut tree = make_tree();
        insert(&mut tree, 0, 10.0, 10.0);

        let result = tree.knn(&query(10.0, 10.0), 0);

        assert!(result.is_empty());
    }

    #[test]
    fn knn_empty_tree_returns_empty() {
        let tree = make_tree();

        let result = tree.knn(&query(50.0, 50.0), 3);

        assert!(result.is_empty());
    }

    #[test]
    fn knn_query_on_exact_point() {
        let mut tree = make_tree();
        insert(&mut tree, 0, 30.0, 30.0);
        insert(&mut tree, 1, 60.0, 60.0);

        let result = tree.knn(&query(30.0, 30.0), 1);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id(), 0);
    }

    #[test]
    fn knn_query_outside_tree_bounds() {
        let mut tree = make_tree(); // bounds: (0,0)-(100,100)
        insert(&mut tree, 0, 1.0, 1.0);
        insert(&mut tree, 1, 99.0, 99.0);

        // Query outside the tree area entirely
        let result = tree.knn(&query(200.0, 200.0), 1);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].id(), 1);
    }

    // --- Larger dataset for cross-checking with brute force ---

    fn brute_force_knn(points: &[DataPoint<f64>], query: &Point<f64>, k: usize) -> Vec<usize> {
        let mut with_dist: Vec<(usize, f64)> = points
            .iter()
            .map(|dp| {
                let dx = dp.point().x - query.x;
                let dy = dp.point().y - query.y;
                (dp.id(), dx.mul_add(dx, dy * dy))
            })
            .collect();

        with_dist.sort_by(|a, b| a.1.total_cmp(&b.1).then(a.0.cmp(&b.0)));
        with_dist.into_iter().take(k).map(|(id, _)| id).collect()
    }

    #[test]
    #[allow(clippy::cast_precision_loss)]
    fn knn_matches_brute_force_small_grid() {
        let mut tree = make_tree();
        let mut all_points = Vec::new();

        for i in 0..10 {
            for j in 0..10 {
                let id = i * 10 + j;
                let x = (i as f64).mul_add(9.0, 1.0);
                let y = (j as f64).mul_add(9.0, 1.0);
                insert(&mut tree, id, x, y);
                all_points.push(DataPoint::new_coords(id, x, y));
            }
        }

        let q = query(45.0, 45.0);
        let k = 5;

        let tree_result: Vec<usize> = tree.knn(&q, k).iter().map(|dp| dp.id).collect();
        let brute_result = brute_force_knn(&all_points, &q, k);

        assert_eq!(tree_result, brute_result);
    }

    #[test]
    #[allow(clippy::cast_precision_loss)]
    fn knn_matches_brute_force_corner_query() {
        let mut tree = make_tree();
        let mut all_points = Vec::new();

        for i in 0..8 {
            for j in 0..8 {
                let id = i * 8 + j;
                let x = (i as f64).mul_add(12.0, 2.0);
                let y = (j as f64).mul_add(12.0, 2.0);
                insert(&mut tree, id, x, y);
                all_points.push(DataPoint::new_coords(id, x, y));
            }
        }

        let q = query(1.0, 1.0);
        let k = 3;

        let tree_result: Vec<usize> = tree.knn(&q, k).iter().map(|dp| dp.id).collect();
        let brute_result = brute_force_knn(&all_points, &q, k);

        assert_eq!(tree_result, brute_result);
    }

    // --- Integer coordinates ---

    #[test]
    fn knn_integer_coords() {
        let mut tree = QuadTree::from_coords(0, 0, 100, 100, 4, 10).unwrap();
        tree.insert(DataPoint::new_coords(0, 10, 10)).unwrap();
        tree.insert(DataPoint::new_coords(1, 20, 20)).unwrap();
        tree.insert(DataPoint::new_coords(2, 90, 90)).unwrap();

        let result = tree.knn(&Point::new(12, 12), 2);

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].id(), 0);
        assert_eq!(result[1].id(), 1);
    }
}
