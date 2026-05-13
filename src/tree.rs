use crate::err::TreeErr;
use crate::geometry::{Coord, Point, Rect};
use color_eyre::Result;
use color_eyre::eyre::{Context, bail};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct QuadTree<T: Coord> {
    area: Rect<T>,
    points: Vec<DataPoint<T>>, // No smallvec, because only top lvl would be on the stack
    level_cap: usize,
    /// 0: upper-left <br/>
    /// 1: upper-right <br/>
    /// 2: lower-left <br/>
    /// 3: lower-right <br/>
    children: Option<Box<[Self; 4]>>,
    depth: usize,
    max_depth: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
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
            if self.points.len() < self.level_cap || self.depth == self.max_depth {
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

    fn subdivide(&mut self) -> Result<()> {
        if self.depth == self.max_depth {
            bail!(TreeErr::MaxDepthReached::<T>(self.depth));
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
}
impl<T: Coord> DataPoint<T> {
    pub fn new(id: usize, point: Point<T>) -> Self {
        Self { id, point }
    }

    pub fn id(&self) -> usize {
        self.id
    }

    pub fn coords(&self) -> (T, T) {
        (self.point.x, self.point.y)
    }
}
