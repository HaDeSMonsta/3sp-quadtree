use crate::err::TreeErr2D;
use crate::geometry::{Coord, Point2D, Rect};
use color_eyre::Result;
use color_eyre::eyre::{Context, bail};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::fs;
use std::mem::size_of;
use std::path::Path;
use tracing::debug;
// Some thoughts:
// Duplicate coordinates are allowed (might be different events or whatever)
// Duplicate IDs are also allowed, that's the users' problem, we assume there are none

use crate::PARALLEL_THRESHOLD;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[allow(clippy::unsafe_derive_deserialize)] // The unsafe is in knn, doesn't matter here
pub struct QuadTree<T: Coord> {
    pub(crate) area: Rect<T>,
    pub(crate) points: Vec<DataPoint2D<T>>, // No smallvec, because only top lvl would be on the stack
    pub(crate) len: usize,
    pub(crate) level_cap: usize,
    /// 0: upper-left <br/>
    /// 1: upper-right <br/>
    /// 2: lower-left <br/>
    /// 3: lower-right <br/>
    pub(crate) children: Option<Box<[Self; 4]>>,
    pub(crate) depth: usize,
    pub(crate) max_depth: usize,
}

// Pub because while we own we don't give out mutable references, and when the user owns it
// he can do as he pleases
#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DataPoint2D<T: Coord> {
    pub id: usize,
    pub point: Point2D<T>,
}

impl<T: Coord> QuadTree<T> {
    pub const fn new(area: Rect<T>, level_cap: usize, max_depth: usize) -> Self {
        Self {
            area,
            points: Vec::new(),
            len: 0,
            level_cap,
            children: None,
            depth: 0,
            max_depth,
        }
    }

    pub fn from_coords(
        lower_x: T,
        lower_y: T,
        upper_x: T,
        upper_y: T,
        level_cap: usize,
        max_depth: usize,
    ) -> Result<Self> {
        let point_lower = Point2D::new(lower_x, lower_y);
        let point_upper = Point2D::new(upper_x, upper_y);
        let rect = Rect::new(point_lower, point_upper)
            .context("failed to create tree due to invalid rect")?;

        Ok(Self::new(rect, level_cap, max_depth))
    }

    /// Create Tree from a Vec of points.
    /// Multi threads when there are enough points
    ///
    /// # Arguments
    ///
    /// * `points`: The points
    /// * `level_cap`: The maximum number of elements to store in non-leaf nodes.
    ///   Subdivides after reaching this number.
    ///   Each division will allocate 4xcap, so keep it low to avoid waste of space
    /// * `max_depth`: Maximum levels in the tree
    ///
    /// returns: `Result<QuadTree<T>, Report>`
    ///
    /// # Errors
    ///
    /// * `TreeErr2D::InsufficientPoints` if `points.len() < 2`
    /// * `rayon::ThreadPoolBuildError` If Rayon can't create a threadpool
    /// * If the build fails for some reason
    ///
    /// # Panics
    ///
    /// If we fucked up the logic and miscalculate the borders of the area
    // Why take the vec and not slice:
    // With a slice we have to copy all the data at some point anyway
    // This can be a problem for a big input
    // So the user can decide, if they don't need the data any more, give up ownership, otherwise clone
    //
    // Actually, you still have to allocate and copy all that data once before, if you need the data later, hmmm
    // CONSIDER Slice version
    pub fn from_points(
        points: Vec<DataPoint2D<T>>,
        level_cap: usize,
        max_depth: usize,
    ) -> Result<Self> {
        let area =
            Self::area_from_points(&points).context("failed to construct area from points")?;

        debug!("building quadtree from {} points", points.len());
        let tree = if points.len() > PARALLEL_THRESHOLD {
            crate::thread_pool()?
                .install(|| Self::build(area, points, level_cap, 0, max_depth))
                .context("failed to build tree")?
        } else {
            Self::build(area, points, level_cap, 0, max_depth)?
        };

        Ok(tree)
    }

    /// Helper function to extract common functionality shared between points creation function that takes ownership and the one that doesn't
    // The second doesn't exist anymore
    fn area_from_points(points: &[DataPoint2D<T>]) -> Result<Rect<T>> {
        let len = points.len();
        if len < 2 {
            bail!(TreeErr2D::InsufficientPoints::<T>(len));
        }

        let first = &points[0];

        let (mut min_x, mut min_y) = first.coords();
        let (mut max_x, mut max_y) = first.coords();

        for dp in &points[1..] {
            let (x, y) = dp.coords();

            if x < min_x {
                min_x = x;
            }
            if x > max_x {
                max_x = x;
            }
            if y < min_y {
                min_y = y;
            }
            if y > max_y {
                max_y = y;
            }
        }

        let rect = Rect::from_coords(min_x, min_y, max_x, max_y)
            .expect("miscalculated borders (impossible (hopefully))");
        Ok(rect)
    }

    /// Build a quadtree from a point set using a top-down bulk construction.
    ///
    /// The resulting tree is equivalent to inserting all points sequentially, but
    /// recursively builds the four subtrees in parallel.
    fn build(
        area: Rect<T>,
        mut points: Vec<DataPoint2D<T>>,
        level_cap: usize,
        depth: usize,
        max_depth: usize,
    ) -> Result<Self> {
        if points.len() <= level_cap || depth == max_depth || !Self::is_subdividable_rect(area) {
            let mut tree = Self::new_child(area, level_cap, depth, max_depth);

            tree.points.append(&mut points);
            tree.len = tree.points.len();

            return Ok(tree);
        }

        let child_depth = depth + 1;

        let (cx, cy) = area.get_centers();

        let rect_0 = Rect::from_coords(area.lower.x, cy, cx, area.upper.y)
            .context("failed to create bulk-build rect 0")?;
        let rect_1 = Rect::from_coords(cx, cy, area.upper.x, area.upper.y)
            .context("failed to create bulk-build rect 1")?;
        let rect_2 = Rect::from_coords(area.lower.x, area.lower.y, cx, cy)
            .context("failed to create bulk-build rect 2")?;
        let rect_3 = Rect::from_coords(cx, area.lower.y, area.upper.x, cy)
            .context("failed to create bulk-build rect 3")?;

        let mut q0 = Vec::new();
        let mut q1 = Vec::new();
        let mut q2 = Vec::new();
        let mut q3 = Vec::new();

        for point in points {
            match Self::child_idx_for_point(area, point.point) {
                0 => q0.push(point),
                1 => q1.push(point),
                2 => q2.push(point),
                3 => q3.push(point),
                i => unreachable!("only indices 0..4 should be valid, got: {i}"),
            }
        }

        let children = if (q0.len() + q1.len() + q2.len() + q3.len()) > PARALLEL_THRESHOLD {
            let ((c0, c1), (c2, c3)) = rayon::join(
                || {
                    rayon::join(
                        || Self::build(rect_0, q0, level_cap, child_depth, max_depth),
                        || Self::build(rect_1, q1, level_cap, child_depth, max_depth),
                    )
                },
                || {
                    rayon::join(
                        || Self::build(rect_2, q2, level_cap, child_depth, max_depth),
                        || Self::build(rect_3, q3, level_cap, child_depth, max_depth),
                    )
                },
            );
            [c0?, c1?, c2?, c3?]
        } else {
            [
                Self::build(rect_0, q0, level_cap, child_depth, max_depth)?,
                Self::build(rect_1, q1, level_cap, child_depth, max_depth)?,
                Self::build(rect_2, q2, level_cap, child_depth, max_depth)?,
                Self::build(rect_3, q3, level_cap, child_depth, max_depth)?,
            ]
        };

        let mut tree = Self::new_child(area, level_cap, depth, max_depth);
        tree.len = children.iter().map(|c| c.len).sum();
        tree.children = Some(Box::new(children));
        Ok(tree)
    }

    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self>
    where
        T: DeserializeOwned,
    {
        let content = fs::read_to_string(&path).context("failed to read json file")?;

        let parsed = serde_json::from_str(&content).context("failed to parse json file")?;

        Ok(parsed)
    }

    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<()>
    where
        T: Serialize,
    {
        let content = serde_json::to_string(&self).context("failed to serialize tree")?;

        fs::write(&path, &content).context("failed to write to file")?;

        Ok(())
    }

    const fn new_child(area: Rect<T>, level_cap: usize, depth: usize, max_depth: usize) -> Self {
        Self {
            area,
            points: Vec::new(),
            len: 0,
            level_cap,
            children: None,
            depth,
            max_depth,
        }
    }

    /// Insert Point into the `QuadTree`
    ///
    /// # Arguments
    ///
    /// * `data_point`: The `DataPoint` to insert into the tree
    ///
    /// Note: Point IDs are never checked and duplicated IDs are considered a user-error
    ///
    /// returns: `Result<(), TreeErr<T>>`
    ///
    /// * `TreeErr::InsertionOutOfBounds`: If the point is not in the area of the tree
    ///
    /// # Panics
    ///
    /// This should never happen.
    /// The function panics if it determines it is safe to subdivide, but it actually is not.
    ///
    /// # Examples
    ///
    /// ```
    /// use quadtree::{DataPoint2D, Point2D, QuadTree};
    ///
    /// let mut tree = QuadTree::from_coords(0.0, 0.0, 1.1, 1.1, 10, 10).expect("failed to create valid tree");
    /// let dp = DataPoint2D::new(1, Point2D::new(0.5, 0.5));
    ///
    /// tree.insert(dp).expect("failed to insert");
    /// ```
    pub fn insert(&mut self, data_point: DataPoint2D<T>) -> Result<(), TreeErr2D<T>> {
        if !self.area.contains(data_point.point) {
            return Err(TreeErr2D::InsertionOutOfBounds {
                rect: self.area,
                data_point,
            });
        }

        // Leaf -> Insert, or generate children
        // Otherwise, children already exist
        if self.children.is_none() {
            // Leaf with space, or leaf at max depth
            if self.points.len() < self.level_cap
                || self.depth == self.max_depth
                || !self.is_subdividable()
            {
                self.points.push(data_point);
                self.len += 1;
                return Ok(());
            }

            self.subdivide()
                .expect("failed to subdivide after checking it is safe to");
        }

        let idx = Self::child_idx_for_point(self.area, data_point.point);
        // no self.children.as_mut().expect() because I want a formatted error
        let Some(child) = &mut self.children else {
            panic!(
                "child not found after subdivide at depth {}, max_depth={}, idx={idx}",
                self.depth, self.max_depth
            );
        };

        child[idx].insert(data_point)?;
        self.len += 1;

        Ok(())
    }

    pub fn remove(&mut self, id: usize) -> Option<DataPoint2D<T>> {
        let Some(children) = &mut self.children else {
            let idx = self.points.iter().position(|dp| dp.id == id)?;
            self.len -= 1;
            return Some(self.points.swap_remove(idx));
        };

        for child in children.iter_mut() {
            if let Some(removed) = child.remove(id) {
                self.len -= 1;
                self.rebalance();
                return Some(removed);
            }
        }

        None
    }

    // Only looks children at current level -> After finding point, rebalance up and after last return one last time
    fn rebalance(&mut self) {
        if self.len > self.level_cap {
            return;
        }
        let Some(children) = &mut self.children else {
            return;
        };

        let mut points = Vec::with_capacity(self.level_cap);

        for child in children.iter_mut() {
            child.drain_points(&mut points);
        }

        self.points = points;
        self.children = None;
    }

    fn drain_points(&mut self, out: &mut Vec<DataPoint2D<T>>) {
        if let Some(children) = &mut self.children {
            for child in children.iter_mut() {
                child.drain_points(out); // Not even sure this is ever called, remove calls on every level, no?
            }
        }

        out.append(&mut self.points);
    }

    // Problem, we only cover very little space, which leads to this:
    // 2026-05-13T10:21:52.837243Z  INFO quadtree: min_x=52.26618607 min_y=13.049611666666665 max_x=52.601945 max_y=13.58743953 size=372469
    // Error:
    //    0: failed to insert DataPoint { id: 1051, point: Point { x: 52.49868333333334, y: 13.301113333333335 } }
    //    1: failed to subdivide
    //    2: failed to create rect 0
    //    3: lower (52.49868333333333, 13.301113333333337) must be lower-left then upper (52.49868333333333, 13.301113333333344)
    // Solution: Do not divide if floats don't play nicely
    // CONSIDER: Before inserting, just shift the values so this never executes (but keep as a fallback (or remove, because less paths to check))
    fn is_subdividable(&self) -> bool {
        Self::is_subdividable_rect(self.area)
    }

    fn is_subdividable_rect(rect: Rect<T>) -> bool {
        let (cx, cy) = rect.get_centers();

        cx > rect.lower.x && cx < rect.upper.x && cy > rect.lower.y && cy < rect.upper.y
    }

    fn subdivide(&mut self) -> Result<()> {
        if self.depth == self.max_depth {
            // bail instead of assert because of backtrace, we want to know which caller fucked up
            bail!("tried to subdivide at max depth, should always be checked before call");
        }

        let rect = self.area;
        let (cx, cy) = rect.get_centers();

        let rect_0 = Rect::from_coords(rect.lower.x, cy, cx, rect.upper.y)
            .context("failed to create rect 0")?;
        let rect_1 = Rect::from_coords(cx, cy, rect.upper.x, rect.upper.y)
            .context("failed to create rect 1")?;
        let rect_2 = Rect::from_coords(rect.lower.x, rect.lower.y, cx, cy)
            .context("failed to create rect 2")?;
        let rect_3 = Rect::from_coords(cx, rect.lower.y, rect.upper.x, cy)
            .context("failed to create rect 3")?;

        let mut children = [
            Self::new_child(rect_0, self.level_cap, self.depth + 1, self.max_depth),
            Self::new_child(rect_1, self.level_cap, self.depth + 1, self.max_depth),
            Self::new_child(rect_2, self.level_cap, self.depth + 1, self.max_depth),
            Self::new_child(rect_3, self.level_cap, self.depth + 1, self.max_depth),
        ];

        for point in self.points.drain(..) {
            let idx = Self::child_idx_for_point(self.area, point.point);
            children[idx]
                .insert(point)
                .context("failed to insert valid point into newly created child")?;
        }
        self.points.shrink_to_fit();

        self.children = Some(Box::new(children));

        Ok(())
    }

    #[allow(clippy::similar_names)]
    fn child_idx_for_point(rect: Rect<T>, point: Point2D<T>) -> usize {
        let (cx, cy) = rect.get_centers();

        let mut ret = 0;

        if point.x > cx {
            ret += 1;
        }

        if point.y < cy {
            ret += 2;
        }

        ret
    }

    pub const fn len(&self) -> usize {
        self.len
    }

    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn nodes(&self) -> usize {
        let child_cnt = self.children.as_ref().map_or(0, |children| {
            let mut sum = 0;
            for child in children {
                sum += child.nodes();
            }
            sum
        });
        child_cnt + 1
    }

    /// Size in bytes of the tree, including every node and point buffer
    pub fn size_bytes(&self) -> usize {
        let mut total = size_of::<Self>();
        total += self.heap_bytes();
        total
    }

    /// Size of the tree formatted
    pub fn size_bytes_formatted(&self) -> String {
        crate::format_bytes(self.size_bytes())
    }

    fn heap_bytes(&self) -> usize {
        let mut total = self.points.capacity() * size_of::<DataPoint2D<T>>();

        if let Some(children) = &self.children {
            total += size_of::<[Self; 4]>();
            for child in children {
                total += child.heap_bytes();
            }
        }

        total
    }

    pub fn print_structure(&self) {
        self.print_structure_inner("", true, "root");
    }

    fn print_structure_inner(&self, prefix: &str, is_last: bool, name: &str) {
        let connector = if is_last { "└──" } else { "├──" };
        let elements = if self.children.is_some() {
            String::new()
        } else {
            format!(" {} elements", self.len())
        };

        println!("{prefix}{connector} {name}:{elements}");

        let Some(children) = &self.children else {
            return;
        };

        let child_prefix = if is_last {
            format!("{prefix}    ")
        } else {
            format!("{prefix}│   ")
        };

        let child_names = ["upper-left", "upper-right", "lower-left", "lower-right"];

        for (idx, child) in children.iter().enumerate() {
            child.print_structure_inner(&child_prefix, idx == children.len() - 1, child_names[idx]);
        }
    }

    // Use this if you care about the id
    pub fn contains_dp(&self, data_point: DataPoint2D<T>) -> bool {
        self.children.as_ref().map_or_else(
            || self.points.contains(&data_point),
            |children| {
                let idx = Self::child_idx_for_point(self.area, data_point.point);
                children[idx].contains_dp(data_point)
            },
        )
    }

    // Use this if you only care about the coords
    pub fn contains_p(&self, point: Point2D<T>) -> bool {
        self.children.as_ref().map_or_else(
            || self.points.iter().any(|dp| dp.point == point),
            |children| {
                let idx = Self::child_idx_for_point(self.area, point);
                children[idx].contains_p(point)
            },
        )
    }
}

impl<T: Coord> DataPoint2D<T> {
    pub const fn new(id: usize, point: Point2D<T>) -> Self {
        Self { id, point }
    }

    pub const fn new_coords(id: usize, x: T, y: T) -> Self {
        let point = Point2D::new(x, y);
        Self::new(id, point)
    }

    pub const fn id(&self) -> usize {
        self.id
    }

    pub const fn point(&self) -> Point2D<T> {
        self.point
    }

    pub const fn coords(&self) -> (T, T) {
        (self.point.x, self.point.y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create() -> Result<()> {
        let _ = QuadTree::from_coords(1, 2, 3, 4, 5, 6).context("failed to create valid tree")?;
        Ok(())
    }
}
