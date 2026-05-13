use crate::err::TreeErr;
use crate::geometry::{Coord, Point, Rect};
use color_eyre::Result;
use color_eyre::eyre::{Context, bail};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::fs;
use std::path::Path;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[allow(clippy::unsafe_derive_deserialize)] // The unsafe is in knn, doesn't matter here
pub struct QuadTree<T: Coord> {
    pub(crate) area: Rect<T>,
    pub(crate) points: Vec<DataPoint<T>>, // No smallvec, because only top lvl would be on the stack
    pub(crate) level_cap: usize,
    /// 0: upper-left <br/>
    /// 1: upper-right <br/>
    /// 2: lower-left <br/>
    /// 3: lower-right <br/>
    pub(crate) children: Option<Box<[Self; 4]>>,
    pub(crate) depth: usize,
    pub(crate) max_depth: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DataPoint<T: Coord> {
    pub(crate) id: usize,
    pub(crate) point: Point<T>,
}

impl<T: Coord> QuadTree<T> {
    pub fn new(area: Rect<T>, level_cap: usize, max_depth: usize) -> Self {
        Self {
            area,
            points: Vec::with_capacity(level_cap),
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
        let point_lower = Point::new(lower_x, lower_y);
        let point_upper = Point::new(upper_x, upper_y);
        let rect = Rect::new(point_lower, point_upper)
            .context("failed to create tree due to invalid rect")?;

        Ok(Self::new(rect, level_cap, max_depth))
    }

    pub fn from_points(points: &[DataPoint<T>]) -> Result<Self> {
        let len = points.len();
        if len < 2 {
            bail!(TreeErr::InsufficientPoints::<T>(len));
        }

        let first = &points[1];

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

        let mut tree = Self::from_coords(min_x, min_y, max_x, max_y, 100, 100)
            .context("failed to construct tree from min/max coords")?;

        for dp in points {
            tree.insert(dp.clone())
                .with_context(|| format!("failed to insert {dp:?}"))?; // Is this even possible?
        }

        Ok(tree)
    }

    pub fn from_file<P: AsRef<Path> + Debug>(path: P) -> Result<Self>
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

    fn new_child(area: Rect<T>, level_cap: usize, depth: usize, max_depth: usize) -> Self {
        Self {
            area,
            points: Vec::with_capacity(level_cap),
            level_cap,
            children: None,
            depth,
            max_depth,
        }
    }

    pub fn insert(&mut self, data_point: DataPoint<T>) -> Result<()> {
        if !self.area.contains(&data_point.point) {
            bail!(TreeErr::InsertionOutOfBounds {
                rect: self.area.clone(),
                data_point,
            })
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
                return Ok(());
            }

            self.subdivide().context("failed to subdivide")?;
        }

        let idx = Self::child_idx_for_point(&self.area, &data_point.point);
        let Some(child) = self.children.as_mut() else {
            bail!(TreeErr::Internal::<T>(format!(
                "child not found after subdivide at depth {}, max_depth={}, idx={idx}",
                self.depth, self.max_depth
            )))
        };

        child[idx].insert(data_point)?;

        Ok(())
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
        let rect = &self.area;
        let (cx, cy) = rect.get_centers();

        cx > rect.lower.x && cx < rect.upper.x && cy > rect.lower.y && cy < rect.upper.y
    }

    fn subdivide(&mut self) -> Result<()> {
        if self.depth == self.max_depth {
            bail!(TreeErr::MaxDepthReached::<T>(self.depth)); // Should never execute btw., we always check before
        }
        let rect = &self.area;
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
            let idx = Self::child_idx_for_point(&self.area, &point.point);
            children[idx]
                .insert(point)
                .context("failed to insert valid point into newly created child")?;
        }

        self.children = Some(Box::new(children));

        Ok(())
    }

    #[allow(clippy::similar_names)]
    fn child_idx_for_point(rect: &Rect<T>, point: &Point<T>) -> usize {
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

    pub fn len(&self) -> usize {
        self.children
            .as_ref()
            .map_or(self.points.len(), |children| {
                let mut sum = 0;
                for child in children.iter() {
                    sum += child.len();
                }
                sum
            })
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn nodes(&self) -> usize {
        let child_cnt = self.children.as_ref().map_or(0, |children| {
            let mut sum = 0;
            for child in children.iter() {
                sum += child.nodes();
            }
            sum
        });
        child_cnt + 1
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
    pub fn contains_dp(&self, data_point: &DataPoint<T>) -> bool {
        self.children.as_ref().map_or_else(
            || self.points.iter().any(|dp| dp == data_point),
            |children| {
                let idx = Self::child_idx_for_point(&self.area, &data_point.point);
                children[idx].contains_dp(&data_point)
            },
        )
    }

    // Use this if you only care about the coords
    pub fn contains_p(&self, point: &Point<T>) -> bool {
        self.children.as_ref().map_or_else(
            || self.points.iter().any(|dp| dp.point == *point),
            |children| {
                let idx = Self::child_idx_for_point(&self.area, &point);
                children[idx].contains_p(&point)
            },
        )
    }
}

impl<T: Coord> DataPoint<T> {
    pub const fn new(id: usize, point: Point<T>) -> Self {
        Self { id, point }
    }

    pub const fn new_coords(id: usize, x: T, y: T) -> Self {
        let point = Point::new(x, y);
        Self::new(id, point)
    }

    pub const fn id(&self) -> usize {
        self.id
    }

    pub const fn point(&self) -> &Point<T> {
        &self.point
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
