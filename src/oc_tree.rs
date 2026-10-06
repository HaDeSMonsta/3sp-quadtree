use crate::geometry::{Coord, Cuboid, Point3D};
use crate::{PARALLEL_THRESHOLD, TreeErr3D};
use color_eyre::Result;
use color_eyre::eyre::{Context, bail};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::fs;
use std::fs::File;
use std::io::BufWriter;
use std::io::Write as _;
use std::mem::size_of;
use std::path::Path;
use tracing::debug;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[allow(clippy::unsafe_derive_deserialize)] // The unsafe is in knn, doesn't matter here
pub struct OcTree<T: Coord> {
    pub(crate) root: OcNode<T>,
    pub(crate) level_cap: usize,
    pub(crate) max_depth: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct OcNode<T: Coord> {
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

#[derive(Copy, Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DataPoint3D<T: Coord> {
    pub id: usize,
    pub point: Point3D<T>,
}

impl<T: Coord> OcTree<T> {
    pub const fn new(area: Cuboid<T>, level_cap: usize, max_depth: usize) -> Self {
        Self {
            root: OcNode::new(area),
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
        points: Vec<DataPoint3D<T>>,
        level_cap: usize,
        max_depth: usize,
    ) -> Result<Self> {
        let cube =
            Self::cube_from_points(&points).context("failed to construct cube from points")?;

        debug!("building octree from {} points", points.len());
        let tree = if points.len() > PARALLEL_THRESHOLD {
            crate::thread_pool()?
                .install(|| Self::build(cube, points, level_cap, 0, max_depth))
                .context("failed to build tree")?
        } else {
            Self::build(cube, points, level_cap, 0, max_depth)?
        };

        Ok(tree)
    }

    fn cube_from_points(points: &[DataPoint3D<T>]) -> Result<Cuboid<T>> {
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

        let point_min = Point3D::new(min_x, min_y, min_z);
        let point_max = Point3D::new(max_x, max_y, max_z);
        let cube = Cuboid::new(point_min, point_max)
            .context("failed to create tree due to invalid rect")?;
        Ok(cube)
    }

    fn build(
        cube: Cuboid<T>,
        points: Vec<DataPoint3D<T>>,
        level_cap: usize,
        depth: usize,
        max_depth: usize,
    ) -> Result<Self> {
        let root = OcNode::build_node(cube, points, level_cap, depth, max_depth)?;
        Ok(Self {
            root,
            level_cap,
            max_depth,
        })
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

    /// Size in bytes of the tree, including every node and point buffer
    pub fn size_bytes(&self) -> usize {
        let mut total = size_of::<Self>();
        total += self.root.heap_bytes();
        total
    }

    /// Size of the tree formatted
    pub fn size_bytes_formatted(&self) -> String {
        crate::format_bytes(self.size_bytes())
    }

    /// Prints the structure of the octree to a file
    ///
    /// # Arguments
    ///
    /// * `file_name`: The file name to save the contents to
    ///
    /// returns: Result<(), Report> If the file cannot be created or written to
    pub fn print_to_file<P: AsRef<Path> + Debug>(&self, file_name: P) -> Result<()> {
        // Printing takes ~70s, so let's write instead (>3G)
        let file = File::create(file_name.as_ref())
            .with_context(|| format!("failed to create {file_name:?}"))?;
        let mut writer = BufWriter::new(file);
        self.root
            .print_structure_inner("", true, "root", &mut writer)
            .with_context(|| format!("failed to store contents in {file_name:?}"))?;
        Ok(())
    }

    pub fn contains(&self, data_point: DataPoint3D<T>) -> bool {
        self.root.contains(data_point)
    }
}

impl<T: Coord> OcNode<T> {
    const fn new(area: Cuboid<T>) -> Self {
        Self {
            area,
            points: Vec::new(),
            len: 0,
            children: None,
            depth: 0,
        }
    }

    const fn new_child(area: Cuboid<T>, depth: usize) -> Self {
        Self {
            area,
            points: Vec::new(),
            len: 0,
            children: None,
            depth,
        }
    }

    fn build_node(
        cube: Cuboid<T>,
        mut points: Vec<DataPoint3D<T>>,
        level_cap: usize,
        depth: usize,
        max_depth: usize,
    ) -> Result<Self> {
        if points.len() <= level_cap || depth == max_depth || !Self::is_subdividable_cube(cube) {
            let mut node = Self::new_child(cube, depth);
            node.points.append(&mut points);
            node.len = node.points.len();
            return Ok(node);
        }

        let child_depth = depth + 1;
        let (cx, cy, cz) = cube.get_centers();

        let cube_0 = Cuboid::from_coords(cube.lower.x, cy, cube.lower.z, cx, cube.upper.y, cz)
            .context("failed to create bulk-build cube 0")?;
        let cube_1 = Cuboid::from_coords(cx, cy, cube.lower.z, cube.upper.x, cube.upper.y, cz)
            .context("failed to create bulk-build cube 1")?;
        let cube_2 = Cuboid::from_coords(cube.lower.x, cube.lower.y, cube.lower.z, cx, cy, cz)
            .context("failed to create bulk-build cube 2")?;
        let cube_3 = Cuboid::from_coords(cx, cube.lower.y, cube.lower.z, cube.upper.x, cy, cz)
            .context("failed to create bulk-build cube 3")?;
        let cube_4 = Cuboid::from_coords(cube.lower.x, cy, cz, cx, cube.upper.y, cube.upper.z)
            .context("failed to create bulk-build cube 4")?;
        let cube_5 = Cuboid::from_coords(cx, cy, cz, cube.upper.x, cube.upper.y, cube.upper.z)
            .context("failed to create bulk-build cube 5")?;
        let cube_6 = Cuboid::from_coords(cube.lower.x, cube.lower.y, cz, cx, cy, cube.upper.z)
            .context("failed to create bulk-build cube 6")?;
        let cube_7 = Cuboid::from_coords(cx, cube.lower.y, cz, cube.upper.x, cy, cube.upper.z)
            .context("failed to create bulk-build cube 7")?;

        let mut q0 = Vec::new();
        let mut q1 = Vec::new();
        let mut q2 = Vec::new();
        let mut q3 = Vec::new();
        let mut q4 = Vec::new();
        let mut q5 = Vec::new();
        let mut q6 = Vec::new();
        let mut q7 = Vec::new();

        for point in points {
            match Self::child_idx_for_point(cube, point.point) {
                0 => q0.push(point),
                1 => q1.push(point),
                2 => q2.push(point),
                3 => q3.push(point),
                4 => q4.push(point),
                5 => q5.push(point),
                6 => q6.push(point),
                7 => q7.push(point),
                i => unreachable!("only indices 0..8 should be valid, got: {i}"),
            }
        }

        let children = if (q0.len()
            + q1.len()
            + q2.len()
            + q3.len()
            + q4.len()
            + q5.len()
            + q6.len()
            + q7.len())
            > PARALLEL_THRESHOLD
        {
            let (((c0, c1), (c2, c3)), ((c4, c5), (c6, c7))) = rayon::join(
                || {
                    rayon::join(
                        || {
                            rayon::join(
                                || Self::build_node(cube_0, q0, level_cap, child_depth, max_depth),
                                || Self::build_node(cube_1, q1, level_cap, child_depth, max_depth),
                            )
                        },
                        || {
                            rayon::join(
                                || Self::build_node(cube_2, q2, level_cap, child_depth, max_depth),
                                || Self::build_node(cube_3, q3, level_cap, child_depth, max_depth),
                            )
                        },
                    )
                },
                || {
                    rayon::join(
                        || {
                            rayon::join(
                                || Self::build_node(cube_4, q4, level_cap, child_depth, max_depth),
                                || Self::build_node(cube_5, q5, level_cap, child_depth, max_depth),
                            )
                        },
                        || {
                            rayon::join(
                                || Self::build_node(cube_6, q6, level_cap, child_depth, max_depth),
                                || Self::build_node(cube_7, q7, level_cap, child_depth, max_depth),
                            )
                        },
                    )
                },
            );
            [c0?, c1?, c2?, c3?, c4?, c5?, c6?, c7?]
        } else {
            [
                Self::build_node(cube_0, q0, level_cap, child_depth, max_depth)?,
                Self::build_node(cube_1, q1, level_cap, child_depth, max_depth)?,
                Self::build_node(cube_2, q2, level_cap, child_depth, max_depth)?,
                Self::build_node(cube_3, q3, level_cap, child_depth, max_depth)?,
                Self::build_node(cube_4, q4, level_cap, child_depth, max_depth)?,
                Self::build_node(cube_5, q5, level_cap, child_depth, max_depth)?,
                Self::build_node(cube_6, q6, level_cap, child_depth, max_depth)?,
                Self::build_node(cube_7, q7, level_cap, child_depth, max_depth)?,
            ]
        };

        let mut node = Self::new_child(cube, depth);
        node.len = children.iter().map(|c| c.len).sum();
        node.children = Some(Box::new(children));
        Ok(node)
    }

    fn insert(
        &mut self,
        data_point: DataPoint3D<T>,
        max_depth: usize,
        level_cap: usize,
    ) -> Result<(), TreeErr3D<T>> {
        if !self.area.contains(data_point.point) {
            return Err(TreeErr3D::InsertionOutOfBounds {
                rect: self.area,
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

        let idx = Self::child_idx_for_point(self.area, data_point.point);
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
        Self::is_subdividable_cube(self.area)
    }

    fn is_subdividable_cube(cube: Cuboid<T>) -> bool {
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

        let cube = self.area;
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
            Self::new_child(cube_0, self.depth + 1),
            Self::new_child(cube_1, self.depth + 1),
            Self::new_child(cube_2, self.depth + 1),
            Self::new_child(cube_3, self.depth + 1),
            Self::new_child(cube_4, self.depth + 1),
            Self::new_child(cube_5, self.depth + 1),
            Self::new_child(cube_6, self.depth + 1),
            Self::new_child(cube_7, self.depth + 1),
        ];

        for point in self.points.drain(..) {
            let idx = Self::child_idx_for_point(self.area, point.point);
            children[idx]
                .insert(point, max_depth, level_cap)
                .context("failed to insert valid point in newly created child")?;
        }
        self.points.shrink_to_fit();

        self.children = Some(Box::new(children));

        Ok(())
    }

    fn child_idx_for_point(cube: Cuboid<T>, point: Point3D<T>) -> usize {
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
            for child in children {
                sum += child.nodes();
            }
            sum
        });
        child_cnt + 1
    }

    fn heap_bytes(&self) -> usize {
        let mut total = self.points.capacity() * size_of::<DataPoint3D<T>>();

        if let Some(children) = &self.children {
            total += size_of::<[Self; 8]>();
            for child in children {
                total += child.heap_bytes();
            }
        }

        total
    }

    fn print_structure_inner(
        &self,
        prefix: &str,
        is_last: bool,
        name: &str,
        writer: &mut BufWriter<File>,
    ) -> Result<()> {
        let connector = if is_last { "└──" } else { "├──" };
        let elements = if self.children.is_some() {
            String::new()
        } else {
            format!(" {} elements", self.len)
        };

        // println!("{prefix}{connector} {name}:{elements}");
        writeln!(writer, "{prefix}{connector} {name}:{elements}").context("failed to write")?;

        let Some(children) = &self.children else {
            return Ok(());
        };

        let child_prefix = if is_last {
            format!("{prefix}    ")
        } else {
            format!("{prefix}│   ")
        };

        let child_names = [
            "upper-left-past",
            "upper-right-past",
            "lower-left-past",
            "lower-right-past",
            "upper-left-fut",
            "upper-right-fut",
            "lower-left-fut",
            "lower-right-fut",
        ];

        for (idx, child) in children.iter().enumerate() {
            child.print_structure_inner(
                &child_prefix,
                idx == children.len() - 1,
                child_names[idx],
                writer,
            )?;
        }
        Ok(())
    }

    pub fn contains(&self, data_point: DataPoint3D<T>) -> bool {
        self.children.as_ref().map_or_else(
            || self.points.contains(&data_point),
            |children| {
                let idx = Self::child_idx_for_point(self.area, data_point.point);
                children[idx].contains(data_point)
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

    pub const fn point(&self) -> Point3D<T> {
        self.point
    }

    pub const fn coords(&self) -> (T, T, T) {
        let point = self.point;
        (point.x, point.y, point.z)
    }
}
