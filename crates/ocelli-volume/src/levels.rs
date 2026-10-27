//! F-058. The multiscale level axis.
//!
//! Level 0 stays the full-resolution prefix. A later entry is caller bytes
//! at the end of the same store. This module does not downsample.

use ocelli_cache::{BrickGrid, BrickWidth};

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
