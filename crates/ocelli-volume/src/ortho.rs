//! Orthographic planes on the volume's own axes.
//!
//! PS3.3 C.7.6.2.1.1 gives the in-plane patient point. The third term is this
//! volume's spacing and normal. Axial, sagittal and coronal name those axes.
//! A sagittal line steps on the column cosine and the normal.

use alloc::vec::Vec;

use glam::DVec3;
use ocelli_core::{Pt, World};

use crate::{Volume, VolumeError, VoxelKind};

/// Which volume axis a plane holds still.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum OrthoPlane {
    /// Constant slice index. Steps along both in-plane cosines.
    Axial {
        /// Slice index `k`.
        slice: f64,
    },
    /// Constant column index. Steps along the column cosine and the normal.
    Sagittal {
        /// Column index `i`.
        column: f64,
    },
    /// Constant row index. Steps along the row cosine and the normal.
    Coronal {
        /// Row index `j`.
        row: f64,
    },
}

/// One resliced sample. Absence is not a stored zero.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PlaneSample {
    /// A container value, little-endian, with no second Bits Stored mask.
    Present(f32),
    /// A corner with non-zero weight sits on a slice that has not landed.
    Absent,
}

/// One orthographic plane. Samples are x-fastest.
#[derive(Clone, Debug, PartialEq)]
pub struct OrthoImage {
    /// Output columns.
    pub width: u32,
    /// Output rows.
    pub height: u32,
    /// Millimetres between output columns.
    pub column_mm: f64,
    /// Millimetres between output rows.
    pub row_mm: f64,
    /// One sample per pixel, x-fastest.
    pub samples: Vec<PlaneSample>,
}

impl Volume {
    /// Patient point of a continuous voxel index.
    ///
    /// `index` is `(i, j, k)` on this volume's axes. The origin is the centre
    /// of voxel `(0, 0, 0)`.
    pub fn voxel_world(&self, index: [f64; 3]) -> Pt<World> {
        let mut point = DVec3::new(self.origin.x, self.origin.y, self.origin.z);
        point += self.direction.col(0) * (self.spacing[0] * index[0]);
        point += self.direction.col(1) * (self.spacing[1] * index[1]);
        point += self.direction.col(2) * (self.spacing[2] * index[2]);
        Pt::<World>::new(point.x, point.y, point.z)
    }

    /// Reslice one orthographic plane from level 0.
    ///
    /// # Errors
    ///
    /// [`VolumeError::PlaneOutOfRange`] when the fixed index is outside
    /// `[0, dim - 1]` or is not finite. Nothing is allocated in that case.
    pub fn reslice(&self, plane: OrthoPlane) -> Result<OrthoImage, VolumeError> {
        let (width, height, column_mm, row_mm) = match plane {
            OrthoPlane::Axial { slice } => {
                self.in_range(slice, self.dims[2], 2)?;
                (self.dims[0], self.dims[1], self.spacing[0], self.spacing[1])
            }
            OrthoPlane::Sagittal { column } => {
                self.in_range(column, self.dims[0], 0)?;
                (self.dims[1], self.dims[2], self.spacing[1], self.spacing[2])
            }
            OrthoPlane::Coronal { row } => {
                self.in_range(row, self.dims[1], 1)?;
                (self.dims[0], self.dims[2], self.spacing[0], self.spacing[2])
            }
        };
        let mut samples = Vec::new();
        let mut row = 0_u32;
        while row < height {
            let mut column = 0_u32;
            while column < width {
                let (x, y, z) = voxel_of(plane, column, row);
                samples.push(self.sample_at(x, y, z));
                column = column.saturating_add(1);
            }
            row = row.saturating_add(1);
        }
        Ok(OrthoImage {
            width,
            height,
            column_mm,
            row_mm,
            samples,
        })
    }

    fn in_range(&self, index: f64, dim: u32, axis: u8) -> Result<(), VolumeError> {
        let inside = dim > 0 && index.is_finite() && index >= 0.0 && index <= f64_from_u32(dim - 1);
        if inside {
            Ok(())
        } else {
            Err(VolumeError::PlaneOutOfRange { axis, index })
        }
    }

    fn sample_at(&self, x: f64, y: f64, z: f64) -> PlaneSample {
        let Some(x0) = u32_from_nonneg_floor(x) else {
            return PlaneSample::Absent;
        };
        let Some(y0) = u32_from_nonneg_floor(y) else {
            return PlaneSample::Absent;
        };
        let Some(z0) = u32_from_nonneg_floor(z) else {
            return PlaneSample::Absent;
        };
        let fx = x - f64_from_u32(x0);
        let fy = y - f64_from_u32(y0);
        let fz = z - f64_from_u32(z0);
        let mut acc = 0.0_f64;
        let mut saw = false;
        for (dx, wx) in [(0_u32, 1.0 - fx), (1_u32, fx)] {
            if is_zero(wx) {
                continue;
            }
            for (dy, wy) in [(0_u32, 1.0 - fy), (1_u32, fy)] {
                if is_zero(wy) {
                    continue;
                }
                for (dz, wz) in [(0_u32, 1.0 - fz), (1_u32, fz)] {
                    if is_zero(wz) {
                        continue;
                    }
                    let weight = wx * wy * wz;
                    if is_zero(weight) {
                        continue;
                    }
                    let ix = x0.saturating_add(dx);
                    let iy = y0.saturating_add(dy);
                    let iz = z0.saturating_add(dz);
                    if !self.present.test(iz) {
                        return PlaneSample::Absent;
                    }
                    let Some(value) = self.container_at(ix, iy, iz) else {
                        return PlaneSample::Absent;
                    };
                    acc += weight * f64::from(value);
                    saw = true;
                }
            }
        }
        if !saw {
            return PlaneSample::Absent;
        }
        PlaneSample::Present(narrow_f32(acc))
    }

    fn container_at(&self, x: u32, y: u32, z: u32) -> Option<f32> {
        if x >= self.dims[0] || y >= self.dims[1] || z >= self.dims[2] {
            return None;
        }
        let bytes = self.data.slice(z)?;
        let width = self.voxel.bytes_per_voxel();
        let columns = usize::try_from(self.dims[0]).ok()?;
        let offset = usize::try_from(y)
            .ok()?
            .checked_mul(columns)?
            .checked_add(usize::try_from(x).ok()?)?
            .checked_mul(width)?;
        let chunk = bytes.get(offset..offset.checked_add(width)?)?;
        decode_one(chunk, self.voxel)
    }
}

fn voxel_of(plane: OrthoPlane, a: u32, b: u32) -> (f64, f64, f64) {
    let along_a = f64_from_u32(a);
    let along_b = f64_from_u32(b);
    match plane {
        OrthoPlane::Axial { slice } => (along_a, along_b, slice),
        OrthoPlane::Sagittal { column } => (column, along_a, along_b),
        OrthoPlane::Coronal { row } => (along_a, row, along_b),
    }
}

fn is_zero(weight: f64) -> bool {
    let bits = weight.to_bits();
    bits == 0 || bits == 1_u64 << 63
}

/// Floor of a non-negative finite value. `f64::floor` is not in `no_std`.
fn floor_nonneg(index: f64) -> f64 {
    let bits = index.to_bits();
    let exponent = u32::try_from((bits >> 52) & 0x7ff).unwrap_or(0);
    if exponent < 1023 {
        return 0.0;
    }
    let shift = exponent - 1023;
    if shift >= 52 {
        return index;
    }
    let places = 52 - shift;
    f64::from_bits(bits & !((1_u64 << places) - 1))
}

fn f64_from_u32(value: u32) -> f64 {
    let high = u16::try_from(value >> 16).unwrap_or(0);
    let low = u16::try_from(value & 0xffff).unwrap_or(0);
    f64::from(high) * 65_536.0 + f64::from(low)
}

fn u32_from_nonneg_floor(index: f64) -> Option<u32> {
    if !index.is_finite() || index < 0.0 {
        return None;
    }
    let whole = floor_nonneg(index);
    if is_zero(whole) {
        return Some(0);
    }
    let bits = whole.to_bits();
    let exponent = u32::try_from((bits >> 52) & 0x7ff).ok()?;
    if exponent < 1023 {
        return Some(0);
    }
    let shift = exponent - 1023;
    if shift > 31 {
        return None;
    }
    let fraction = bits & 0x000f_ffff_ffff_ffff;
    let significand = fraction | (1_u64 << 52);
    let places = 52 - shift;
    u32::try_from(significand >> places).ok()
}

/// Ties-to-even narrowing. `as f32` is denied, so the bits are written out.
fn narrow_f32(value: f64) -> f32 {
    if value.is_nan() {
        return f32::from_bits(0x7fc0_0000);
    }
    let bits = value.to_bits();
    let sign = u32::try_from(bits >> 63).unwrap_or(0);
    let exponent = u32::try_from((bits >> 52) & 0x7ff).unwrap_or(0);
    let fraction = bits & 0x000f_ffff_ffff_ffff;
    if exponent == 0x7ff {
        return f32::from_bits((sign << 31) | 0x7f80_0000);
    }
    if exponent == 0 {
        return f32::from_bits(sign << 31);
    }
    let target = i32::try_from(exponent).unwrap_or(0) - 1023 + 127;
    let significand = fraction | (1_u64 << 52);
    if target >= 255 {
        return f32::from_bits((sign << 31) | 0x7f80_0000);
    }
    if target <= 0 {
        let rounded = rounded_shift(significand, 30 - target);
        if rounded >= (1_u64 << 23) {
            return f32::from_bits((sign << 31) | (1_u32 << 23));
        }
        return f32::from_bits((sign << 31) | u32::try_from(rounded).unwrap_or(0));
    }
    let rounded = rounded_shift(significand, 29);
    let mut out_exponent = u32::try_from(target).unwrap_or(0);
    let mantissa = if rounded >= (1_u64 << 24) {
        out_exponent = out_exponent.saturating_add(1);
        0
    } else {
        u32::try_from(rounded & 0x7f_ffff).unwrap_or(0)
    };
    if out_exponent >= 255 {
        return f32::from_bits((sign << 31) | 0x7f80_0000);
    }
    f32::from_bits((sign << 31) | (out_exponent << 23) | mantissa)
}

fn rounded_shift(sig: u64, shift: i32) -> u64 {
    if shift <= 0 {
        return sig;
    }
    let Ok(places) = u32::try_from(shift) else {
        return 0;
    };
    if places == 0 || places >= 64 {
        return 0;
    }
    let main = sig >> places;
    let half = 1_u64 << (places - 1);
    let remainder = sig & ((1_u64 << places) - 1);
    let round_up = remainder > half || (remainder == half && (main & 1) == 1);
    if round_up {
        main.saturating_add(1)
    } else {
        main
    }
}

fn decode_one(chunk: &[u8], kind: VoxelKind) -> Option<f32> {
    match kind {
        VoxelKind::U8 => chunk.first().copied().map(f32::from),
        VoxelKind::I16 => {
            let pair = [chunk.first().copied()?, chunk.get(1).copied()?];
            Some(f32::from(i16::from_le_bytes(pair)))
        }
        VoxelKind::U16 => {
            let pair = [chunk.first().copied()?, chunk.get(1).copied()?];
            Some(f32::from(u16::from_le_bytes(pair)))
        }
        VoxelKind::F32 => {
            let quad = [
                chunk.first().copied()?,
                chunk.get(1).copied()?,
                chunk.get(2).copied()?,
                chunk.get(3).copied()?,
            ];
            Some(f32::from_le_bytes(quad))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::PlaneSample;
    use crate::{Level, SlicePresence, Volume, VoxelKind, VoxelStore};
    use glam::DMat3;
    use ocelli_core::{Pt, World};

    #[test]
    fn a_fractional_sagittal_sample_does_not_blend_an_absent_slice() {
        let mut volume = Volume {
            dims: [2, 2, 2],
            spacing: [0.5, 2.0, 3.0],
            origin: Pt::<World>::new(10.0, 20.0, 30.0),
            direction: DMat3::IDENTITY,
            voxel: VoxelKind::U8,
            data: VoxelStore::new([2, 2, 2], VoxelKind::U8),
            levels: vec![Level {
                dims: [2, 2, 2],
                spacing: [0.5, 2.0, 3.0],
                offset: 0,
            }],
            present: SlicePresence::new(2),
            order: Vec::new(),
        };
        assert!(volume.place(0, &[0, 1, 10, 11]).is_ok());
        // Column 0.5, pixel (0, 0.5) is voxel (0.5, 0, 0.5). Slice 1 has weight.
        assert!(matches!(
            volume.sample_at(0.5, 0.0, 0.5),
            PlaneSample::Absent
        ));
    }
}
