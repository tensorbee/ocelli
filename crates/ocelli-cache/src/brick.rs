//! Brick addressing and residency.
//!
//! HLD section 19's layout, read at brick scale, and section 30's edge.
//! The edge numbers live in [`BrickGrid::new`] and nowhere else. Chunked
//! addressing is the only path: a volume of one brick and a volume of
//! several hundred use this constructor.

use alloc::vec::Vec;

use crate::{Admission, Budgeted, CacheTier, Lru, Pressure};

/// A volume identity inside a [`BrickKey`].
///
/// A `u64` rather than a UID string, on the same reasoning as [`crate::SeriesId`].
/// Assigning an id belongs to whoever owns the session.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VolumeId(pub u64);

/// The sample width that chooses a brick edge.
///
/// Signed and unsigned sixteen-bit samples are one width. The volume crate
/// converts its own voxel kind into this one.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum BrickWidth {
    /// One byte per voxel. Edge 128.
    EightBit,
    /// Two bytes per voxel. Edge 64.
    SixteenBit,
    /// Four bytes per voxel. Edge 64.
    Float32,
}

impl BrickWidth {
    /// Bytes one voxel occupies. This is not the edge.
    pub const fn bytes_per_voxel(self) -> usize {
        match self {
            Self::EightBit => 1,
            Self::SixteenBit => 2,
            Self::Float32 => 4,
        }
    }
}

/// One brick of one volume at one level of detail.
///
/// The ordering is the `BTreeMap` ordering [`Lru`] is built on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BrickKey {
    /// The volume this brick was cut from.
    pub volume: VolumeId,
    /// The multiscale level. Phase 1 stores level 0.
    pub level: u32,
    /// The brick coordinate, x then y then z.
    pub brick: [u32; 3],
}

/// A voxel or a brick coordinate that is outside the grid.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BrickError {
    /// `value` on `axis` is not a voxel of the volume. Axes are 0, 1, 2.
    VoxelOutOfRange { axis: u8, value: u32 },
    /// `value` on `axis` is not a brick of the grid. Axes are 0, 1, 2.
    BrickOutOfRange { axis: u8, value: u32 },
}

/// The brick decomposition of one level of one volume.
///
/// Pure arithmetic over `dims`. No allocation and no I/O.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BrickGrid {
    /// Voxel dimensions, x then y then z.
    pub dims: [u32; 3],
    /// The brick edge in voxels. Set only by [`BrickGrid::new`].
    pub edge: u32,
    /// How many bricks cover `dims`, ceiling division per axis.
    pub grid: [u32; 3],
    /// The level these bricks belong to. [`BrickGrid::new`] starts at 0.
    pub level: u32,
}

impl BrickGrid {
    /// Decompose `dims` for `voxel`. One match owns the edge.
    pub fn new(dims: [u32; 3], voxel: BrickWidth) -> Self {
        let edge = match voxel {
            BrickWidth::EightBit => 128,
            BrickWidth::SixteenBit => 64,
            BrickWidth::Float32 => 64,
        };
        let [dx, dy, dz] = dims;
        Self {
            dims,
            edge,
            grid: [ceil_div(dx, edge), ceil_div(dy, edge), ceil_div(dz, edge)],
            level: 0,
        }
    }

    /// The brick containing a voxel, one component per axis.
    ///
    /// # Errors
    ///
    /// [`BrickError::VoxelOutOfRange`] when a component is past `dims`.
    pub fn brick_of(&self, voxel: [u32; 3]) -> Result<[u32; 3], BrickError> {
        self.voxel_in_range(voxel)?;
        let [x, y, z] = voxel;
        Ok([
            div_edge(x, self.edge),
            div_edge(y, self.edge),
            div_edge(z, self.edge),
        ])
    }

    /// The voxel offset of a brick origin within the level.
    ///
    /// # Errors
    ///
    /// [`BrickError::BrickOutOfRange`] when a component is past `grid`.
    pub fn origin_of(&self, brick: [u32; 3]) -> Result<[u32; 3], BrickError> {
        self.brick_in_range(brick)?;
        let [x, y, z] = brick;
        Ok([
            mul_edge(x, self.edge, 0)?,
            mul_edge(y, self.edge, 1)?,
            mul_edge(z, self.edge, 2)?,
        ])
    }

    /// The true extent of a brick, clipped at the volume faces.
    ///
    /// # Errors
    ///
    /// [`BrickError::BrickOutOfRange`] when `brick` is outside the grid.
    pub fn extent_of(&self, brick: [u32; 3]) -> Result<[u32; 3], BrickError> {
        let [ox, oy, oz] = self.origin_of(brick)?;
        let [dx, dy, dz] = self.dims;
        Ok([
            clipped(dx, ox, self.edge),
            clipped(dy, oy, self.edge),
            clipped(dz, oz, self.edge),
        ])
    }

    /// The linear brick index, x fastest, then y, then z.
    ///
    /// # Errors
    ///
    /// [`BrickError::BrickOutOfRange`] when `brick` is outside the grid, or
    /// when the index does not fit in `u64` and then in `usize`. The error
    /// carries axis 2 and the z component.
    pub fn index_of(&self, brick: [u32; 3]) -> Result<usize, BrickError> {
        self.brick_in_range(brick)?;
        let [x, y, z] = brick;
        let [gx, gy, _] = self.grid;
        let wide = checked_linear(x, y, z, gx, gy)
            .ok_or(BrickError::BrickOutOfRange { axis: 2, value: z })?;
        usize::try_from(wide).map_err(|_err| BrickError::BrickOutOfRange { axis: 2, value: z })
    }

    /// The volume byte offset of a voxel.
    ///
    /// Section 19 layout: `((z * dims[1] + y) * dims[0] + x) * bytes_per_voxel`.
    /// Each product is a checked `u64`, then narrowed once.
    ///
    /// # Errors
    ///
    /// [`BrickError::VoxelOutOfRange`] when `voxel` is outside `dims`, or when
    /// the address does not fit in `u64` and then in `usize`. The error
    /// carries axis 2 and the z component.
    pub fn byte_offset(
        &self,
        voxel: [u32; 3],
        bytes_per_voxel: usize,
    ) -> Result<usize, BrickError> {
        self.voxel_in_range(voxel)?;
        let [x, y, z] = voxel;
        let [dx, dy, _] = self.dims;
        let out = BrickError::VoxelOutOfRange { axis: 2, value: z };
        let linear = checked_linear(x, y, z, dx, dy).ok_or(out)?;
        let width = u64::try_from(bytes_per_voxel).map_err(|_err| out)?;
        let bytes = linear.checked_mul(width).ok_or(out)?;
        usize::try_from(bytes).map_err(|_err| out)
    }

    fn voxel_in_range(&self, voxel: [u32; 3]) -> Result<(), BrickError> {
        let [x, y, z] = voxel;
        let [dx, dy, dz] = self.dims;
        if x >= dx {
            return Err(BrickError::VoxelOutOfRange { axis: 0, value: x });
        }
        if y >= dy {
            return Err(BrickError::VoxelOutOfRange { axis: 1, value: y });
        }
        if z >= dz {
            return Err(BrickError::VoxelOutOfRange { axis: 2, value: z });
        }
        Ok(())
    }

    fn brick_in_range(&self, brick: [u32; 3]) -> Result<(), BrickError> {
        let [x, y, z] = brick;
        let [gx, gy, gz] = self.grid;
        if x >= gx {
            return Err(BrickError::BrickOutOfRange { axis: 0, value: x });
        }
        if y >= gy {
            return Err(BrickError::BrickOutOfRange { axis: 1, value: y });
        }
        if z >= gz {
            return Err(BrickError::BrickOutOfRange { axis: 2, value: z });
        }
        Ok(())
    }
}

/// Ceiling division. `edge` is 64 or 128, so the widen is what keeps a `dim`
/// near `u32::MAX` from wrapping the sum.
fn ceil_div(dim: u32, edge: u32) -> u32 {
    let wide = u64::from(dim).div_ceil(u64::from(edge));
    u32::try_from(wide).unwrap_or(u32::MAX)
}

fn div_edge(value: u32, edge: u32) -> u32 {
    match edge {
        0 => 0,
        _ => value / edge,
    }
}

/// `((z * dy + y) * dx + x)` in `u64`. `None` when a product does not fit.
fn checked_linear(x: u32, y: u32, z: u32, dx: u32, dy: u32) -> Option<u64> {
    let zy = u64::from(z).checked_mul(u64::from(dy))?;
    let row = zy.checked_add(u64::from(y))?;
    let slab = row.checked_mul(u64::from(dx))?;
    slab.checked_add(u64::from(x))
}

fn mul_edge(value: u32, edge: u32, axis: u8) -> Result<u32, BrickError> {
    match value.checked_mul(edge) {
        Some(product) => Ok(product),
        None => Err(BrickError::BrickOutOfRange { axis, value }),
    }
}

fn clipped(dim: u32, origin: u32, edge: u32) -> u32 {
    let rest = dim.saturating_sub(origin);
    if rest < edge { rest } else { edge }
}

/// The voxels of one brick, x fastest, at true extent.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Brick {
    /// Voxels along x, y and z. An edge brick is shorter than the edge.
    pub extent: [u32; 3],
    /// The allocated samples, contiguous in x and packed across rows.
    pub samples: Vec<u8>,
}

impl Brick {
    /// Copy one brick out of a volume buffer.
    ///
    /// A brick is contiguous in x and strided in y and z. The copy is the
    /// only place the volume layout and the brick layout meet, and it is an
    /// assembly call, not a render-loop call.
    ///
    /// # Errors
    ///
    /// [`BrickError::BrickOutOfRange`] when `brick` is outside `grid`.
    /// [`BrickError::VoxelOutOfRange`] when a source row is not in `volume`.
    pub fn cut(
        grid: &BrickGrid,
        brick: [u32; 3],
        volume: &[u8],
        bytes_per_voxel: usize,
    ) -> Result<Self, BrickError> {
        let extent = grid.extent_of(brick)?;
        let [ex, ey, ez] = extent;
        let [ox, oy, oz] = grid.origin_of(brick)?;
        let row = match usize::try_from(ex)
            .ok()
            .and_then(|count| count.checked_mul(bytes_per_voxel))
        {
            Some(row) => row,
            None => {
                return Err(BrickError::VoxelOutOfRange { axis: 0, value: ex });
            }
        };
        let rows = u64::from(ey).saturating_mul(u64::from(ez));
        let total = match usize::try_from(rows)
            .ok()
            .and_then(|count| count.checked_mul(row))
        {
            Some(total) => total,
            None => {
                return Err(BrickError::VoxelOutOfRange { axis: 2, value: ez });
            }
        };
        let mut samples = alloc::vec![0u8; total];
        let mut written = 0usize;
        let mut dz = 0u32;
        while dz < ez {
            let z = match oz.checked_add(dz) {
                Some(z) => z,
                None => {
                    return Err(BrickError::VoxelOutOfRange { axis: 2, value: oz });
                }
            };
            let mut dy = 0u32;
            while dy < ey {
                let y = match oy.checked_add(dy) {
                    Some(y) => y,
                    None => {
                        return Err(BrickError::VoxelOutOfRange { axis: 1, value: oy });
                    }
                };
                let offset = grid.byte_offset([ox, y, z], bytes_per_voxel)?;
                let end = match offset.checked_add(row) {
                    Some(end) => end,
                    None => {
                        return Err(BrickError::VoxelOutOfRange { axis: 0, value: ox });
                    }
                };
                let Some(src) = volume.get(offset..end) else {
                    return Err(BrickError::VoxelOutOfRange { axis: 0, value: ox });
                };
                let dest_end = match written.checked_add(row) {
                    Some(dest_end) => dest_end,
                    None => {
                        return Err(BrickError::VoxelOutOfRange { axis: 0, value: ex });
                    }
                };
                let Some(dst) = samples.get_mut(written..dest_end) else {
                    return Err(BrickError::VoxelOutOfRange { axis: 0, value: ex });
                };
                dst.copy_from_slice(src);
                written = dest_end;
                dy += 1;
            }
            dz += 1;
        }
        Ok(Self { extent, samples })
    }
}

impl Budgeted for Brick {
    /// The allocated buffer, which is what this entry costs its tier.
    fn bytes(&self) -> usize {
        self.samples.len()
    }
}

/// Resident bricks for one GPU budget.
///
/// The bytes are still a host buffer. F-040 removes that buffer in the step
/// that uploads it, so one brick is never counted against two budgets.
pub struct BrickCache {
    bricks: Lru<BrickKey, Brick>,
}

impl BrickCache {
    /// An empty GPU-tier cache holding at most `budget` bytes.
    pub fn new(budget: usize) -> Self {
        Self {
            bricks: Lru::new(CacheTier::Gpu, budget),
        }
    }

    /// Insert one brick. The [`Admission`] is returned unchanged.
    pub fn insert(&mut self, key: BrickKey, brick: Brick) -> Admission<BrickKey, Brick> {
        self.bricks.insert(key, brick)
    }

    /// The brick, if it is resident. A hit is a use.
    pub fn get(&mut self, key: &BrickKey) -> Option<&Brick> {
        self.bricks.get(key)
    }

    /// Remove one brick without calling it an eviction.
    pub fn remove(&mut self, key: &BrickKey) -> Option<Brick> {
        self.bricks.remove(key)
    }

    /// The GPU tier's pressure. A reading is not a use.
    pub fn pressure(&self) -> Pressure {
        self.bricks.pressure()
    }

    /// The bricks of `grid.level` that `get` would miss, in [`BrickGrid::index_of`] order.
    ///
    /// This does not touch recency.
    pub fn missing(&self, volume: VolumeId, grid: &BrickGrid) -> Vec<BrickKey> {
        let mut missing = Vec::new();
        let [gx, gy, gz] = grid.grid;
        let mut z = 0u32;
        while z < gz {
            let mut y = 0u32;
            while y < gy {
                let mut x = 0u32;
                while x < gx {
                    let key = BrickKey {
                        volume,
                        level: grid.level,
                        brick: [x, y, z],
                    };
                    if !self.bricks.contains_key(&key) {
                        missing.push(key);
                    }
                    x += 1;
                }
                y += 1;
            }
            z += 1;
        }
        missing
    }

    /// Whether every brick of `grid.level` for `volume` is resident.
    ///
    /// An empty grid is not a complete level.
    pub fn is_resident(&self, volume: VolumeId, grid: &BrickGrid) -> bool {
        let [gx, gy, gz] = grid.grid;
        if gx == 0 || gy == 0 || gz == 0 {
            return false;
        }
        self.missing(volume, grid).is_empty()
    }
}
