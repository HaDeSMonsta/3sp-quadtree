use crate::TreeErr3D;
use crate::geometry::{Coord, Cuboid, Point3D};
use color_eyre::Result;
use color_eyre::eyre::{Context, bail};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::fs;
use std::path::Path;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[allow(clippy::unsafe_derive_deserialize)] // The unsafe is in knn, doesn't matter here
pub struct OctTree<T: Coord> {
    pub(crate) root: OctNode<T>,
    pub(crate) level_cap: usize,
    pub(crate) max_depth: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OctNode<T: Coord> {
    pub(crate) area: Cuboid<T>,
    pub(crate) points: Vec<DataPoint3D<T>>,
    pub(crate) len: usize,
    /// 0: upper-left-past <br/>
    /// 1: upper-right-past <br/>
    /// 2: lower-left-past <br/>
    /// 3: lower-right-past<br/>
    /// 4: upper-left-fut <br/>
    /// 5: upper-right-fut <br/>
    /// 6: lower-left-fut <br/>
    /// 7: lower-right-fut <br/>
    pub(crate) children: Option<Box<[Self; 8]>>,
    pub(crate) depth: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DataPoint3D<T: Coord> {
    pub id: usize,
    pub point: Point3D<T>,
}

impl<T: Coord> OctTree<T> {
    pub fn new(area: Cuboid<T>, level_cap: usize, max_depth: usize) -> Self {
        Self {
            root: OctNode::new(area, level_cap),
            level_cap,
            max_depth,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn from_coords(
        lower_x: T,
        lower_y: T,
        lower_z: T,
        upper_x: T,
        upper_y: T,
        upper_z: T,
        level_cap: usize,
        max_depth: usize,
    ) -> Result<Self> {
        let point_lower = Point3D::new(lower_x, lower_y, lower_z);
        let point_upper = Point3D::new(upper_x, upper_y, upper_z);
        let cube = Cuboid::new(point_lower, point_upper)
            .context("failed to create tree due to invalid rect")?;

        Ok(Self::new(cube, level_cap, max_depth))
    }

    pub fn from_points(
        points: &[DataPoint3D<T>],
        level_cap: usize,
        max_depth: usize,
    ) -> Result<Self> {
        let len = points.len();
        if len < 2 {
            bail!(TreeErr3D::InsufficientPoints::<T>(len));
        }

        let first = &points[0];

        let (mut min_x, mut min_y, mut min_z) = first.coords();
        let (mut max_x, mut max_y, mut max_z) = first.coords();

        for dp in &points[1..] {
            let (x, y, z) = dp.coords();

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

            if z < min_z {
                min_z = z;
            }
            if z > max_z {
                max_z = z;
            }
        }

        let mut tree = Self::from_coords(
            min_x, min_y, min_z, max_x, max_y, max_z, level_cap, max_depth,
        )
        .context("failed to construct tree from min/max coords")?;

        for dp in points {
            tree.insert(dp.clone())
                .with_context(|| format!("failed to insert {dp:?}"))?;
        }

        Ok(tree)
    }

    pub fn from_file<P: AsRef<Path>>(path: P) -> Result<Self>
    where
        T: DeserializeOwned,
    {
        let content = fs::read_to_string(&path).context("failed to read json file")?;

        let parsed = serde_json::from_str(&content).context("failed to parse json")?;

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

    pub fn insert(&mut self, data_point: DataPoint3D<T>) -> Result<(), TreeErr3D<T>> {
        self.root.insert(data_point, self.max_depth, self.level_cap)
    }

    pub fn remove(&mut self, id: usize) -> Option<DataPoint3D<T>> {
        self.root.remove(id, self.level_cap)
    }

    pub const fn len(&self) -> usize {
        self.root.len
    }

    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn nodes(&self) -> usize {
        self.root.nodes()
    }

    pub fn contains(&self, data_point: &DataPoint3D<T>) -> bool {
        self.root.contains(&data_point)
    }
}

impl<T: Coord> OctNode<T> {
    fn new(area: Cuboid<T>, level_cap: usize) -> Self {
        Self {
            area,
            points: Vec::with_capacity(level_cap),
            len: 0,
            children: None,
            depth: 0,
        }
    }

    fn new_child(area: Cuboid<T>, level_cap: usize, depth: usize) -> Self {
        Self {
            area,
            points: Vec::with_capacity(level_cap),
            len: 0,
            children: None,
            depth,
        }
    }

    fn insert(
        &mut self,
        data_point: DataPoint3D<T>,
        max_depth: usize,
        level_cap: usize,
    ) -> Result<(), TreeErr3D<T>> {
        if !self.area.contains(&data_point.point) {
            return Err(TreeErr3D::InsertionOutOfBounds {
                rect: self.area.clone(),
                data_point,
            });
        }

        if self.children.is_none() {
            if self.points.len() < level_cap || self.depth == max_depth || !self.is_subdividable() {
                self.points.push(data_point);
                self.len += 1;
                return Ok(());
            }

            self.subdivide(max_depth, level_cap)
                .expect("failed to subdivide after checking it is safe to do");
        }

        let idx = Self::child_idx_for_point(&self.area, &data_point.point);
        let Some(child) = &mut self.children else {
            panic!(
                "child not found after subdivide at depth {}, max_depth={max_depth}, idx={idx}",
                self.depth
            );
        };

        child[idx].insert(data_point, max_depth, level_cap)?;
        self.len += 1;

        Ok(())
    }

    pub fn remove(&mut self, id: usize, level_cap: usize) -> Option<DataPoint3D<T>> {
        let Some(children) = &mut self.children else {
            let idx = self.points.iter().position(|dp| dp.id == id)?;
            self.len -= 1;
            return Some(self.points.swap_remove(idx));
        };

        for child in children.iter_mut() {
            if let Some(removed) = child.remove(id, level_cap) {
                self.len -= 1;
                self.rebalance(level_cap);
                return Some(removed);
            }
        }

        None
    }

    fn rebalance(&mut self, level_cap: usize) {
        if self.len > level_cap {
            return;
        }
        let Some(children) = &mut self.children else {
            return;
        };

        let mut points = Vec::with_capacity(level_cap);

        for child in children.iter_mut() {
            child.drain_points(&mut points);
        }

        self.points = points;
        self.children = None;
    }

    fn drain_points(&mut self, out: &mut Vec<DataPoint3D<T>>) {
        if let Some(children) = &mut self.children {
            for child in children.iter_mut() {
                child.drain_points(out);
            }
        }

        out.append(&mut self.points);
    }

    fn is_subdividable(&self) -> bool {
        let cube = &self.area;
        let (cx, cy, cz) = cube.get_centers();

        cx > cube.lower.x
            && cx < cube.upper.x
            && cy > cube.lower.y
            && cy < cube.upper.y
            && cz > cube.lower.z
            && cz < cube.upper.z
    }

    fn subdivide(&mut self, max_depth: usize, level_cap: usize) -> Result<()> {
        if self.depth == max_depth {
            bail!("tried to subdivide at bax depth, should always be checked by the caller");
        }

        let cube = &self.area;
        let (cx, cy, cz) = cube.get_centers();

        let cube_0 = Cuboid::from_coords(cube.lower.x, cy, cube.lower.z, cx, cube.upper.y, cz)
            .context("failed to create cube 0")?;
        let cube_1 = Cuboid::from_coords(cx, cy, cube.lower.z, cube.upper.x, cube.upper.y, cz)
            .context("failed to create cube 1")?;
        let cube_2 = Cuboid::from_coords(cube.lower.x, cube.lower.y, cube.lower.z, cx, cy, cz)
            .context("failed to create cube 2")?;
        let cube_3 = Cuboid::from_coords(cx, cube.lower.y, cube.lower.z, cube.upper.x, cy, cz)
            .context("failed to create cube 3")?;
        let cube_4 = Cuboid::from_coords(cube.lower.x, cy, cz, cx, cube.upper.y, cube.upper.z)
            .context("failed to create cube 4")?;
        let cube_5 = Cuboid::from_coords(cx, cy, cz, cube.upper.x, cube.upper.y, cube.upper.z)
            .context("failed to create cube 5")?;
        let cube_6 = Cuboid::from_coords(cube.lower.x, cube.lower.y, cz, cx, cy, cube.upper.z)
            .context("failed to create cube 6")?;
        let cube_7 = Cuboid::from_coords(cx, cube.lower.y, cz, cube.upper.x, cy, cube.upper.z)
            .context("failed to create cube 7")?;

        let mut children = [
            Self::new_child(cube_0, level_cap, self.depth + 1),
            Self::new_child(cube_1, level_cap, self.depth + 1),
            Self::new_child(cube_2, level_cap, self.depth + 1),
            Self::new_child(cube_3, level_cap, self.depth + 1),
            Self::new_child(cube_4, level_cap, self.depth + 1),
            Self::new_child(cube_5, level_cap, self.depth + 1),
            Self::new_child(cube_6, level_cap, self.depth + 1),
            Self::new_child(cube_7, level_cap, self.depth + 1),
        ];

        for point in self.points.drain(..) {
            let idx = Self::child_idx_for_point(&self.area, &point.point);
            children[idx]
                .insert(point, max_depth, level_cap)
                .context("failed to insert valid point in newly created child")?;
        }

        self.children = Some(Box::new(children));

        Ok(())
    }

    fn child_idx_for_point(cube: &Cuboid<T>, point: &Point3D<T>) -> usize {
        let (cx, cy, cz) = cube.get_centers();

        let mut ret = 0;

        if point.x > cx {
            ret += 1;
        }

        if point.y < cy {
            ret += 2;
        }

        if point.z > cz {
            ret += 4;
        }

        ret
    }

    fn nodes(&self) -> usize {
        let child_cnt = self.children.as_ref().map_or(0, |children| {
            let mut sum = 0;
            for child in children.iter() {
                sum += child.nodes();
            }
            sum
        });
        child_cnt + 1
    }

    pub fn contains(&self, data_point: &DataPoint3D<T>) -> bool {
        self.children.as_ref().map_or_else(
            || self.points.iter().any(|dp| dp == data_point),
            |children| {
                let idx = Self::child_idx_for_point(&self.area, &data_point.point);
                children[idx].contains(&data_point)
            },
        )
    }
}

impl<T: Coord> DataPoint3D<T> {
    pub const fn new(id: usize, point: Point3D<T>) -> Self {
        Self { id, point }
    }

    pub const fn new_coords(id: usize, x: T, y: T, z: T) -> Self {
        let point = Point3D::new(x, y, z);
        Self::new(id, point)
    }

    pub const fn id(&self) -> usize {
        self.id
    }

    pub const fn point(&self) -> &Point3D<T> {
        &self.point
    }

    pub const fn coords(&self) -> (T, T, T) {
        let point = &self.point;
        (point.x, point.y, point.z)
    }
}
