use crate::geometry::{Coord, Point, Rect};
use crate::tree::{DataPoint, QuadTree};
use std::cmp::Ordering;
use std::collections::BinaryHeap;

#[derive(Clone, Debug)]
struct BestNeighbour<T: Coord> {
    dist2: T,
    data_point: DataPoint<T>,
}

impl<T: Coord> PartialEq for BestNeighbour<T> {
    fn eq(&self, other: &Self) -> bool {
        self.dist2 == other.dist2 && self.data_point.id == other.data_point.id
    }
}

impl<T: Coord> Eq for BestNeighbour<T> {}

impl<T: Coord> PartialOrd for BestNeighbour<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<T: Coord> Ord for BestNeighbour<T> {
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
    pub fn knn(&self, query: &Point<T>, k: usize) -> Vec<DataPoint<T>> {
        if k == 0 {
            return Vec::new();
        }

        let mut candidates = BinaryHeap::new();
        let mut best: BinaryHeap<BestNeighbour<T>> = BinaryHeap::with_capacity(k + 1);

        candidates.push(CandidateNode {
            min_dist2: dist2_point_rect(&query, &self.area),
            node: &self,
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
                    let dist2 = dist2_point_point(&query, &data_point.point);

                    if best.len() < k {
                        best.push(BestNeighbour {
                            dist2,
                            data_point: data_point.clone(),
                        });
                    } else if best.peek().is_some_and(|worst| dist2 < worst.dist2) {
                        best.pop();
                        best.push(BestNeighbour {
                            dist2,
                            data_point: data_point.clone(),
                        });
                    }
                }
            }
        }

        let mut result = best.into_vec();

        result.sort_by(|a, b| {
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
