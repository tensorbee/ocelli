//! F-058 and F-119. The multiscale level axis.
//!
//! Level 0 is the full-resolution prefix. F-119 downsamples that prefix
//! into coarser levels in the same store. `append_level` still copies
//! caller bytes and does not downsample. Spacing is written only by
//! `append_level`.

use alloc::vec::Vec;

use ocelli_cache::{Brick, BrickGrid, BrickSample, BrickWidth};

use crate::ortho::narrow_f32;
use crate::{Level, Volume, VolumeError, VoxelKind};

impl Volume {
    /// Level `index`, or [`VolumeError::LevelAbsent`].
    ///
    /// # Errors
    ///
    /// [`VolumeError::LevelAbsent`] when `index` is past the axis.
    pub fn level(&self, index: u32) -> Result<&Level, VolumeError> {
        let slot = usize::try_from(index).map_err(|_err| VolumeError::LevelAbsent { index })?;
        self.levels
            .get(slot)
            .ok_or(VolumeError::LevelAbsent { index })
    }

    /// Bytes stored for `index`.
    ///
    /// # Errors
    ///
    /// [`VolumeError::LevelAbsent`] when `index` is past the axis.
    /// [`VolumeError::LevelShape`] when the recorded span does not fit the store.
    pub fn level_bytes(&self, index: u32) -> Result<&[u8], VolumeError> {
        let level = *self.level(index)?;
        let count = byte_len(level.dims, self.voxel).ok_or(VolumeError::LevelShape)?;
        let end = level
            .offset
            .checked_add(count)
            .ok_or(VolumeError::LevelShape)?;
        self.data
            .as_bytes()
            .get(level.offset..end)
            .ok_or(VolumeError::LevelShape)
    }

    /// Append one coarser level. Level 0 is unchanged.
    ///
    /// `samples` is copied to the end of the same store. The new spacing is
    /// level 0's spacing times `dims_0[i] / dims[i]` on each axis.
    ///
    /// # Errors
    ///
    /// [`VolumeError::LevelShape`] when the grid or the buffer is refused.
    /// The volume is unchanged in that case.
    /// [`VolumeError::LevelAbsent`] when level 0 itself is missing.
    pub fn append_level(&mut self, dims: [u32; 3], samples: &[u8]) -> Result<u32, VolumeError> {
        let level0 = *self.level(0)?;
        if !coarser(level0.dims, dims) {
            return Err(VolumeError::LevelShape);
        }
        let expected = byte_len(dims, self.voxel).ok_or(VolumeError::LevelShape)?;
        if samples.len() != expected {
            return Err(VolumeError::LevelShape);
        }
        let spacing = scaled(level0.dims, level0.spacing, dims).ok_or(VolumeError::LevelShape)?;
        let offset = self.data.len();
        let index = u32::try_from(self.levels.len()).map_err(|_err| VolumeError::LevelShape)?;
        self.data.extend_bytes(samples);
        self.levels.push(Level {
            dims,
            spacing,
            offset,
        });
        Ok(index)
    }

    /// The brick grid of one stored level. [`BrickGrid::level`] is that index.
    ///
    /// # Errors
    ///
    /// [`VolumeError::LevelAbsent`] when `index` is past the axis.
    pub fn brick_grid_at(&self, index: u32) -> Result<BrickGrid, VolumeError> {
        let level = *self.level(index)?;
        let mut grid = BrickGrid::new(level.dims, BrickWidth::from(self.voxel));
        grid.level = index;
        Ok(grid)
    }

    /// Downsample level 0 to `dims` and append it. Returns the new index.
    ///
    /// The exact-overlap box filter of F-119 decision 1, over stored values.
    /// Spacing comes from `append_level`.
    ///
    /// # Errors
    ///
    /// [`VolumeError::SliceNotPresent`] when level 0 is incomplete.
    /// [`VolumeError::LevelShape`] when `dims` is not strictly coarser.
    /// [`VolumeError::LevelAbsent`] when `levels` is empty.
    /// The volume is unchanged in each case.
    pub fn downsample_level(&mut self, dims: [u32; 3]) -> Result<u32, VolumeError> {
        // Stored container values, PS3.3 C.11.1. Rescale is affine, so the
        // mean of stored values is the mean of modality values before rounding.
        // A Modality LUT Sequence is not affine and this volume has no modality
        // stage to consult, so that case is not refused.
        ensure_complete(self)?;
        let level0 = *self.level(0)?;
        if !coarser(level0.dims, dims) {
            return Err(VolumeError::LevelShape);
        }
        let source_len = byte_len(level0.dims, self.voxel).ok_or(VolumeError::LevelShape)?;
        let buffer = {
            let source = self
                .data
                .as_bytes()
                .get(..source_len)
                .ok_or(VolumeError::LevelShape)?;
            downsample_stored(source, level0.dims, dims, self.voxel)?
        };
        self.append_level(dims, &buffer)
    }

    /// Replace every level above 0 with the generated pyramid.
    ///
    /// Level `k` has `max(1, ceil(dims_0 / 2^k))` on each axis. Generation
    /// stops after the first level whose brick grid is one brick. Returns
    /// `levels.len()` after generation, which is 1 for a one-brick level 0.
    ///
    /// # Errors
    ///
    /// [`VolumeError::SliceNotPresent`] when level 0 is incomplete, with the
    /// volume unchanged.
    /// [`VolumeError::LevelShape`] when level 0 has a zero dimension, with
    /// the volume unchanged.
    /// [`VolumeError::LevelAbsent`] when `levels` is empty, with the volume
    /// unchanged.
    pub fn generate_pyramid(&mut self) -> Result<u32, VolumeError> {
        ensure_complete(self)?;
        if self.levels.len() > 1 {
            self.data.truncate();
            self.levels.truncate(1);
        }
        let full = self.level(0)?.dims;
        loop {
            let prev = u32::try_from(self.levels.len().saturating_sub(1))
                .map_err(|_err| VolumeError::LevelShape)?;
            let grid = self.brick_grid_at(prev)?;
            if grid.grid == [1, 1, 1] {
                break;
            }
            let level = u32::try_from(self.levels.len()).map_err(|_err| VolumeError::LevelShape)?;
            let dims = pyramid_dims(full, level).ok_or(VolumeError::LevelShape)?;
            self.downsample_level(dims)?;
        }
        u32::try_from(self.levels.len()).map_err(|_err| VolumeError::LevelShape)
    }

    /// Copy one brick of stored level `index`.
    ///
    /// Level 0 is [`Volume::cut_ready_brick`].
    ///
    /// # Errors
    ///
    /// [`VolumeError::LevelAbsent`] past the axis, [`VolumeError::BrickCut`]
    /// outside that level's grid, and level 0's own refusals.
    pub fn cut_level_brick(&self, index: u32, brick: [u32; 3]) -> Result<Brick, VolumeError> {
        if index == 0 {
            return self.cut_ready_brick(brick);
        }
        let grid = self.brick_grid_at(index)?;
        let bytes = self.level_bytes(index)?;
        Brick::cut(&grid, brick, bytes, BrickSample::from(self.voxel))
            .map_err(crate::brick_cut_error)
    }

    /// The level 0 continuous voxel index of a level `index` voxel centre.
    ///
    /// `(c + 0.5) * dims_0 / dims_k - 0.5` on each axis.
    /// Level 0 returns the input unchanged.
    ///
    /// # Errors
    ///
    /// [`VolumeError::LevelAbsent`] past the axis.
    pub fn level_voxel_to_level0(
        &self,
        index: u32,
        voxel: [f64; 3],
    ) -> Result<[f64; 3], VolumeError> {
        let child = *self.level(index)?;
        let full = *self.level(0)?;
        let [cx, cy, cz] = voxel;
        let [fx, fy, fz] = full.dims;
        let [dx, dy, dz] = child.dims;
        Ok([
            level_centre(cx, fx, dx),
            level_centre(cy, fy, dy),
            level_centre(cz, fz, dz),
        ])
    }
}

/// True when every axis is in `1..=full` and at least one axis is shorter.
fn coarser(full: [u32; 3], dims: [u32; 3]) -> bool {
    let mut smaller = false;
    for (axis, &next) in dims.iter().enumerate() {
        let Some(limit) = full.get(axis).copied() else {
            return false;
        };
        if next == 0 || next > limit {
            return false;
        }
        if next < limit {
            smaller = true;
        }
    }
    smaller
}

fn byte_len(dims: [u32; 3], kind: VoxelKind) -> Option<usize> {
    let mut count = 1_usize;
    for dim in dims {
        let wide = usize::try_from(dim).ok()?;
        count = count.checked_mul(wide)?;
    }
    count.checked_mul(kind.bytes_per_voxel())
}

/// `spacing_0 * dims_0 / dims` per axis. A zero denominator is `None`.
fn scaled(full_dims: [u32; 3], full_spacing: [f64; 3], dims: [u32; 3]) -> Option<[f64; 3]> {
    let mut spacing = [0.0_f64; 3];
    for axis in 0..3 {
        let numer = f64::from(*full_dims.get(axis)?);
        let denom = f64::from(*dims.get(axis)?);
        if denom == 0.0 {
            return None;
        }
        let step = full_spacing.get(axis).copied()?;
        let slot = spacing.get_mut(axis)?;
        *slot = step * numer / denom;
    }
    Some(spacing)
}

fn ensure_complete(volume: &Volume) -> Result<(), VolumeError> {
    let [_dx, _dy, count] = volume.dims;
    let mut index = 0u32;
    while index < count {
        if !volume.present.test(index) {
            return Err(VolumeError::SliceNotPresent { index });
        }
        index += 1;
    }
    Ok(())
}

/// `(c + 0.5) * full / coarse - 0.5`. Level 0, where the two dims match, is identity.
fn level_centre(child: f64, full: u32, coarse: u32) -> f64 {
    // Equal dims are identity in exact arithmetic. The product form is off
    // by one ULP for some fractional inputs, so the input is returned.
    if full == coarse {
        return child;
    }
    (child + 0.5) * f64::from(full) / f64::from(coarse) - 0.5
}

/// `max(1, ceil(dim / 2^level))` on every axis.
fn pyramid_dims(full: [u32; 3], level: u32) -> Option<[u32; 3]> {
    let [dx, dy, dz] = full;
    Some([
        axis_halved(dx, level)?,
        axis_halved(dy, level)?,
        axis_halved(dz, level)?,
    ])
}

fn axis_halved(dim: u32, level: u32) -> Option<u32> {
    if level >= 32 {
        return Some(1);
    }
    let denom = 1_u32.checked_shl(level)?;
    let numer = u64::from(dim)
        .checked_add(u64::from(denom))?
        .checked_sub(1)?;
    let quotient = numer / u64::from(denom);
    let value = u32::try_from(quotient).ok()?;
    Some(value.max(1))
}

fn downsample_stored(
    source: &[u8],
    full: [u32; 3],
    dims: [u32; 3],
    kind: VoxelKind,
) -> Result<Vec<u8>, VolumeError> {
    let count = byte_len(dims, kind).ok_or(VolumeError::LevelShape)?;
    let mut buffer = alloc::vec![0_u8; count];
    let [dx, dy, dz] = dims;
    let mut z = 0u32;
    while z < dz {
        let mut y = 0u32;
        while y < dy {
            let mut x = 0u32;
            while x < dx {
                write_voxel(&mut buffer, source, full, dims, kind, [x, y, z])?;
                x += 1;
            }
            y += 1;
        }
        z += 1;
    }
    Ok(buffer)
}

fn write_voxel(
    buffer: &mut [u8],
    source: &[u8],
    full: [u32; 3],
    dims: [u32; 3],
    kind: VoxelKind,
    voxel: [u32; 3],
) -> Result<(), VolumeError> {
    let [x, y, z] = voxel;
    let offset =
        stored_offset(dims, x, y, z, kind.bytes_per_voxel()).ok_or(VolumeError::LevelShape)?;
    match kind {
        VoxelKind::F32 => {
            let mean = mean_f32(source, full, dims, voxel)?;
            put(buffer, offset, &mean.to_le_bytes())
        }
        VoxelKind::U8 => {
            let mean = mean_i128(source, full, dims, kind, voxel)?;
            let narrow = u8::try_from(mean).map_err(|_err| VolumeError::LevelShape)?;
            put(buffer, offset, &[narrow])
        }
        VoxelKind::I16 => {
            let mean = mean_i128(source, full, dims, kind, voxel)?;
            let narrow = i16::try_from(mean).map_err(|_err| VolumeError::LevelShape)?;
            put(buffer, offset, &narrow.to_le_bytes())
        }
        VoxelKind::U16 => {
            let mean = mean_i128(source, full, dims, kind, voxel)?;
            let narrow = u16::try_from(mean).map_err(|_err| VolumeError::LevelShape)?;
            put(buffer, offset, &narrow.to_le_bytes())
        }
    }
}

fn put(buffer: &mut [u8], offset: usize, bytes: &[u8]) -> Result<(), VolumeError> {
    let end = offset
        .checked_add(bytes.len())
        .ok_or(VolumeError::LevelShape)?;
    let slot = buffer.get_mut(offset..end).ok_or(VolumeError::LevelShape)?;
    slot.copy_from_slice(bytes);
    Ok(())
}

fn mean_i128(
    source: &[u8],
    full: [u32; 3],
    dims: [u32; 3],
    kind: VoxelKind,
    voxel: [u32; 3],
) -> Result<i128, VolumeError> {
    let sum = fold_parents(full, dims, voxel, 0_i128, |sum, weights, parent| {
        let weight = weight_product(weights)?;
        let factor = i128::try_from(weight).map_err(|_err| VolumeError::LevelShape)?;
        let sample = read_stored(source, full, parent, kind)?;
        let term = sample.checked_mul(factor).ok_or(VolumeError::LevelShape)?;
        sum.checked_add(term).ok_or(VolumeError::LevelShape)
    })?;
    round_ties_away(sum, weight_total(full)?)
}

fn mean_f32(
    source: &[u8],
    full: [u32; 3],
    dims: [u32; 3],
    voxel: [u32; 3],
) -> Result<f32, VolumeError> {
    let sum = fold_parents(full, dims, voxel, 0.0_f64, |sum, weights, parent| {
        let [wx, wy, wz] = weights;
        let factor = weight_f64(wx)? * weight_f64(wy)? * weight_f64(wz)?;
        let sample = read_f32(source, full, parent)?;
        Ok(sum + factor * f64::from(sample))
    })?;
    let [fx, fy, fz] = full;
    let divisor = f64::from(fx) * f64::from(fy) * f64::from(fz);
    Ok(narrow_f32(sum / divisor))
}

fn fold_parents<T>(
    full: [u32; 3],
    dims: [u32; 3],
    voxel: [u32; 3],
    mut state: T,
    mut fold: impl FnMut(T, [u64; 3], [u32; 3]) -> Result<T, VolumeError>,
) -> Result<T, VolumeError> {
    let [x, y, z] = voxel;
    let [fx, fy, fz] = full;
    let [dx, dy, dz] = dims;
    let (x0, x1) = parent_span(x, fx, dx).ok_or(VolumeError::LevelShape)?;
    let (y0, y1) = parent_span(y, fy, dy).ok_or(VolumeError::LevelShape)?;
    let (z0, z1) = parent_span(z, fz, dz).ok_or(VolumeError::LevelShape)?;
    let mut pz = z0;
    loop {
        let mut py = y0;
        loop {
            let mut px = x0;
            loop {
                let weights = [
                    axis_weight(x, px, fx, dx).ok_or(VolumeError::LevelShape)?,
                    axis_weight(y, py, fy, dy).ok_or(VolumeError::LevelShape)?,
                    axis_weight(z, pz, fz, dz).ok_or(VolumeError::LevelShape)?,
                ];
                state = fold(state, weights, [px, py, pz])?;
                if px == x1 {
                    break;
                }
                px += 1;
            }
            if py == y1 {
                break;
            }
            py += 1;
        }
        if pz == z1 {
            break;
        }
        pz += 1;
    }
    Ok(state)
}

/// Inclusive parent range `c * D / d` through `((c + 1) * D - 1) / d`.
fn parent_span(child: u32, full: u32, coarse: u32) -> Option<(u32, u32)> {
    if coarse == 0 {
        return None;
    }
    let c = u64::from(child);
    let wide = u64::from(full);
    let narrow = u64::from(coarse);
    let start = c.checked_mul(wide)? / narrow;
    let end = c.checked_add(1)?.checked_mul(wide)?.checked_sub(1)? / narrow;
    if start > end {
        return None;
    }
    Some((u32::try_from(start).ok()?, u32::try_from(end).ok()?))
}

/// Overlap of child `child` with parent `parent`.
///
/// The child covers `[c * D, (c + 1) * D)` and the parent covers
/// `[p * d, (p + 1) * d)`. One child's weights sum to `D`.
fn axis_weight(child: u32, parent: u32, full: u32, coarse: u32) -> Option<u64> {
    let c = u64::from(child);
    let p = u64::from(parent);
    let wide = u64::from(full);
    let narrow = u64::from(coarse);
    let child_hi = c.checked_add(1)?.checked_mul(wide)?;
    let parent_hi = p.checked_add(1)?.checked_mul(narrow)?;
    let child_lo = c.checked_mul(wide)?;
    let parent_lo = p.checked_mul(narrow)?;
    let upper = child_hi.min(parent_hi);
    let lower = child_lo.max(parent_lo);
    Some(upper.saturating_sub(lower))
}

fn weight_product(weights: [u64; 3]) -> Result<u128, VolumeError> {
    let [wx, wy, wz] = weights;
    u128::from(wx)
        .checked_mul(u128::from(wy))
        .and_then(|value| value.checked_mul(u128::from(wz)))
        .ok_or(VolumeError::LevelShape)
}

fn weight_total(full: [u32; 3]) -> Result<u128, VolumeError> {
    let [dx, dy, dz] = full;
    u128::from(dx)
        .checked_mul(u128::from(dy))
        .and_then(|value| value.checked_mul(u128::from(dz)))
        .ok_or(VolumeError::LevelShape)
}

/// Nearest integer, ties away from zero.
///
/// For `S >= 0` the value is `(2S + W) / (2W)`. For `S < 0` it is
/// `-((-2S + W) / (2W))`. Both divisions are of non-negative values,
/// so truncating division matches the rule.
fn round_ties_away(sum: i128, weight: u128) -> Result<i128, VolumeError> {
    let w = i128::try_from(weight).map_err(|_err| VolumeError::LevelShape)?;
    if w <= 0 {
        return Err(VolumeError::LevelShape);
    }
    let double_w = w.checked_mul(2).ok_or(VolumeError::LevelShape)?;
    if sum >= 0 {
        let numer = sum
            .checked_mul(2)
            .and_then(|value| value.checked_add(w))
            .ok_or(VolumeError::LevelShape)?;
        return Ok(numer / double_w);
    }
    let positive = sum.checked_neg().ok_or(VolumeError::LevelShape)?;
    let numer = positive
        .checked_mul(2)
        .and_then(|value| value.checked_add(w))
        .ok_or(VolumeError::LevelShape)?;
    let magnitude = numer / double_w;
    magnitude.checked_neg().ok_or(VolumeError::LevelShape)
}

fn weight_f64(weight: u64) -> Result<f64, VolumeError> {
    let narrow = u32::try_from(weight).map_err(|_err| VolumeError::LevelShape)?;
    Ok(f64::from(narrow))
}

fn read_stored(
    source: &[u8],
    full: [u32; 3],
    voxel: [u32; 3],
    kind: VoxelKind,
) -> Result<i128, VolumeError> {
    let [x, y, z] = voxel;
    let offset =
        stored_offset(full, x, y, z, kind.bytes_per_voxel()).ok_or(VolumeError::LevelShape)?;
    match kind {
        VoxelKind::U8 => source.get(offset).copied().map(i128::from),
        VoxelKind::U16 => two_at(source, offset).map(|bytes| i128::from(u16::from_le_bytes(bytes))),
        VoxelKind::I16 => two_at(source, offset).map(|bytes| i128::from(i16::from_le_bytes(bytes))),
        VoxelKind::F32 => None,
    }
    .ok_or(VolumeError::LevelShape)
}

fn read_f32(source: &[u8], full: [u32; 3], voxel: [u32; 3]) -> Result<f32, VolumeError> {
    let [x, y, z] = voxel;
    let offset = stored_offset(full, x, y, z, 4).ok_or(VolumeError::LevelShape)?;
    let bytes = [
        source.get(offset).copied().ok_or(VolumeError::LevelShape)?,
        source
            .get(offset.checked_add(1).ok_or(VolumeError::LevelShape)?)
            .copied()
            .ok_or(VolumeError::LevelShape)?,
        source
            .get(offset.checked_add(2).ok_or(VolumeError::LevelShape)?)
            .copied()
            .ok_or(VolumeError::LevelShape)?,
        source
            .get(offset.checked_add(3).ok_or(VolumeError::LevelShape)?)
            .copied()
            .ok_or(VolumeError::LevelShape)?,
    ];
    Ok(f32::from_le_bytes(bytes))
}

fn two_at(source: &[u8], offset: usize) -> Option<[u8; 2]> {
    let next = offset.checked_add(1)?;
    Some([source.get(offset).copied()?, source.get(next).copied()?])
}

/// Section 19 layout: `((z * dy + y) * dx + x) * bytes_per_voxel`.
fn stored_offset(dims: [u32; 3], x: u32, y: u32, z: u32, width: usize) -> Option<usize> {
    let [dx, dy, dz] = dims;
    if x >= dx || y >= dy || z >= dz || width == 0 {
        return None;
    }
    let linear = u64::from(z)
        .checked_mul(u64::from(dy))?
        .checked_add(u64::from(y))?
        .checked_mul(u64::from(dx))?
        .checked_add(u64::from(x))?;
    let bytes = linear.checked_mul(u64::try_from(width).ok()?)?;
    usize::try_from(bytes).ok()
}

#[cfg(test)]
mod offset_bounds {
    use super::{
        axis_halved, axis_weight, coarser, parent_span, round_ties_away, scaled, stored_offset,
    };
    use crate::VolumeError;

    /// Section 19. `((z * dy + y) * dx + x) * width`.
    /// Dims `[2, 3, 4]`, voxel `(1, 2, 1)`, width 1.
    /// `(1 * 3 + 2) * 2 + 1 = 11`.
    /// x 2, y 3 and z 4 are one past each axis.
    /// Without the y check, y 3 is index `(0 * 3 + 3) * 2 + 0 = 6`.
    /// Width 0 is not a voxel, so the address is refused.
    #[test]
    fn stored_offset_refuses_each_axis() {
        assert_eq!(stored_offset([2, 3, 4], 1, 2, 1, 1), Some(11));
        assert_eq!(stored_offset([2, 3, 4], 2, 0, 0, 1), None);
        assert_eq!(stored_offset([2, 3, 4], 0, 3, 0, 1), None);
        assert_eq!(stored_offset([2, 3, 4], 0, 0, 4, 1), None);
        assert_eq!(stored_offset([2, 3, 4], 0, 0, 0, 0), None);
    }

    /// A zero axis is outside `1..=full`, on x, y and z.
    #[test]
    fn coarser_refuses_a_zero_axis() {
        assert!(!coarser([4, 4, 4], [0, 2, 2]));
        assert!(!coarser([4, 4, 4], [2, 0, 2]));
        assert!(!coarser([4, 4, 4], [2, 2, 0]));
    }

    /// A zero denominator has no spacing.
    #[test]
    fn scaled_refuses_a_zero_denominator() {
        assert_eq!(scaled([4, 4, 4], [1.0, 1.0, 1.0], [0, 2, 2]), None);
        assert_eq!(scaled([4, 4, 4], [1.0, 1.0, 1.0], [2, 0, 2]), None);
        assert_eq!(scaled([4, 4, 4], [1.0, 1.0, 1.0], [2, 2, 0]), None);
    }

    /// Child 0 against parent 5. The child covers `[0, 4)` and the parent
    /// covers `[10, 12)`. The overlap is 0, not a wrapped difference.
    #[test]
    fn axis_weight_of_a_disjoint_parent_is_zero() {
        assert_eq!(axis_weight(0, 5, 4, 2), Some(0));
    }

    /// Coarse 0 is not a parent range.
    #[test]
    fn parent_span_refuses_a_zero_coarse_axis() {
        assert_eq!(parent_span(0, 4, 0), None);
    }

    /// Weight 0 is not a mean.
    #[test]
    fn round_ties_away_refuses_a_zero_weight() {
        assert_eq!(round_ties_away(1, 0), Err(VolumeError::LevelShape));
    }

    /// Decision 6. `max(1, ceil(dim / 2^level))`.
    /// `ceil(0 / 2) = 0`, then the outer max makes 1.
    /// `ceil(5 / 2^32) = 1`. A shift of 32 does not fit in `u32`, so the
    /// level guard returns 1 rather than refusing the shape.
    #[test]
    fn axis_halved_keeps_the_max_and_the_wide_shift() {
        assert_eq!(axis_halved(0, 1), Some(1));
        assert_eq!(axis_halved(5, 32), Some(1));
        assert_eq!(axis_halved(5, 40), Some(1));
    }
}
