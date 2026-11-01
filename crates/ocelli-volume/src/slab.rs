//! Thick slab samples through one level-0 plane.
//!
//! The step is the volume spacing along the unit patient normal. It is not
//! `SpacingBetweenSlices` and it is not `SliceThickness`. Those tags are not
//! read. Each included index is one [`Volume::sample_at`].

use alloc::vec::Vec;

use glam::{DMat3, DVec3};

use crate::ortho::{PlaneSample, u32_from_nonneg_floor};
use crate::{OrthoPlane, Volume, VolumeError};

/// One slab frame. Indexes are voxels. The normal is patient space.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SlabPlane {
    /// Index of output pixel `(0, 0)`.
    pub origin_index: [f64; 3],
    /// Patient normal. `slab_samples` normalises a non-zero finite vector.
    pub normal: [f64; 3],
    /// Index step of one output column.
    pub column: [f64; 3],
    /// Index step of one output row.
    pub row: [f64; 3],
    /// Output columns.
    pub width: u32,
    /// Output rows.
    pub height: u32,
    /// Millimetres between output columns.
    pub column_mm: f64,
    /// Millimetres between output rows.
    pub row_mm: f64,
}

/// Host slab samples. `samples` is x-fastest, and each pixel owns
/// `sample_count` entries from the most negative step to the most positive.
#[derive(Clone, Debug, PartialEq)]
pub struct SlabImage {
    /// Output columns.
    pub width: u32,
    /// Output rows.
    pub height: u32,
    /// Millimetres between adjacent samples along the normal.
    pub step_mm: f64,
    /// Index displacement of one positive step.
    pub index_step: [f64; 3],
    /// Samples along the normal, including the centre.
    pub sample_count: u32,
    /// One column of samples per pixel.
    pub samples: Vec<PlaneSample>,
}

impl Volume {
    /// Orthographic frame for a slab. The fixed index uses the same range as
    /// [`Volume::reslice`].
    ///
    /// # Errors
    ///
    /// [`VolumeError::PlaneOutOfRange`] when the held index is outside
    /// `[0, dim - 1]` or is not finite. Nothing is allocated.
    pub fn slab_plane(&self, plane: OrthoPlane) -> Result<SlabPlane, VolumeError> {
        let (origin_index, normal, column, row, width, height, column_mm, row_mm) = match plane {
            OrthoPlane::Axial { slice } => {
                self.in_range(slice, self.dims[2], 2)?;
                (
                    [0.0, 0.0, slice],
                    self.direction.col(2),
                    [1.0, 0.0, 0.0],
                    [0.0, 1.0, 0.0],
                    self.dims[0],
                    self.dims[1],
                    self.spacing[0],
                    self.spacing[1],
                )
            }
            OrthoPlane::Sagittal { column } => {
                self.in_range(column, self.dims[0], 0)?;
                (
                    [column, 0.0, 0.0],
                    self.direction.col(0),
                    [0.0, 1.0, 0.0],
                    [0.0, 0.0, 1.0],
                    self.dims[1],
                    self.dims[2],
                    self.spacing[1],
                    self.spacing[2],
                )
            }
            OrthoPlane::Coronal { row } => {
                self.in_range(row, self.dims[1], 1)?;
                (
                    [0.0, row, 0.0],
                    self.direction.col(1),
                    [1.0, 0.0, 0.0],
                    [0.0, 0.0, 1.0],
                    self.dims[0],
                    self.dims[2],
                    self.spacing[0],
                    self.spacing[2],
                )
            }
        };
        Ok(SlabPlane {
            origin_index,
            normal: [normal.x, normal.y, normal.z],
            column,
            row,
            width,
            height,
            column_mm,
            row_mm,
        })
    }

    /// Samples inside `thickness_mm` of `plane`.
    ///
    /// Integer `t` is included when `abs(t * step_mm) <= thickness_mm / 2`.
    /// `t = 0` is the centre. A pixel with any absent sample is absent in
    /// every slot of its column. Validation runs before the buffer exists.
    ///
    /// # Errors
    ///
    /// [`VolumeError::SlabThickness`] when the thickness is not finite, is
    /// negative, or does not fit in a sample count. [`VolumeError::SlabNormal`]
    /// when the normal or the spacing matrix cannot define a step.
    pub fn slab_samples(
        &self,
        plane: SlabPlane,
        thickness_mm: f64,
    ) -> Result<SlabImage, VolumeError> {
        if !thickness_mm.is_finite() || thickness_mm < 0.0 {
            return Err(VolumeError::SlabThickness);
        }
        let Some((index_step, step_mm)) = step_along(self, plane.normal) else {
            return Err(VolumeError::SlabNormal);
        };
        let half = thickness_mm * 0.5;
        let Some(steps) = included_steps(step_mm, half) else {
            return Err(VolumeError::SlabThickness);
        };
        let Some(sample_count) = steps
            .checked_mul(2)
            .and_then(|doubled| doubled.checked_add(1))
        else {
            return Err(VolumeError::SlabThickness);
        };
        let Some(width) = usize::try_from(plane.width).ok() else {
            return Err(VolumeError::SlabThickness);
        };
        let Some(height) = usize::try_from(plane.height).ok() else {
            return Err(VolumeError::SlabThickness);
        };
        let Some(pixels) = width.checked_mul(height) else {
            return Err(VolumeError::SlabThickness);
        };
        let Some(per_pixel) = usize::try_from(sample_count).ok() else {
            return Err(VolumeError::SlabThickness);
        };
        let Some(count) = pixels.checked_mul(per_pixel) else {
            return Err(VolumeError::SlabThickness);
        };
        let mut samples = Vec::with_capacity(count);
        let walk = ColumnWalk {
            plane: &plane,
            index_step: &index_step,
            steps,
            sample_count,
        };
        let mut row = 0_u32;
        while row < plane.height {
            let mut column = 0_u32;
            while column < plane.width {
                push_column(self, &walk, column, row, &mut samples);
                column = column.saturating_add(1);
            }
            row = row.saturating_add(1);
        }
        Ok(SlabImage {
            width: plane.width,
            height: plane.height,
            step_mm,
            index_step,
            sample_count,
            samples,
        })
    }
}

struct ColumnWalk<'a> {
    plane: &'a SlabPlane,
    index_step: &'a [f64; 3],
    steps: u32,
    sample_count: u32,
}

fn push_column(
    volume: &Volume,
    walk: &ColumnWalk<'_>,
    column: u32,
    row: u32,
    samples: &mut Vec<PlaneSample>,
) {
    let base = pixel_index(walk.plane, column, row);
    let start = samples.len();
    let mut absent = false;
    let mut slot = 0_u32;
    while slot < walk.sample_count {
        let index = sample_index(base, walk.index_step, walk.steps, slot);
        let sample = volume.sample_at(index[0], index[1], index[2]);
        if matches!(sample, PlaneSample::Absent) {
            absent = true;
        }
        samples.push(sample);
        slot = slot.saturating_add(1);
    }
    if absent {
        let mut index = start;
        while index < samples.len() {
            if let Some(held) = samples.get_mut(index) {
                *held = PlaneSample::Absent;
            }
            index = index.saturating_add(1);
        }
    }
}

fn sample_index(base: [f64; 3], index_step: &[f64; 3], steps: u32, slot: u32) -> [f64; 3] {
    if slot < steps {
        sub3(base, scale3(index_step, steps - slot))
    } else if slot == steps {
        base
    } else {
        add3(base, scale3(index_step, slot - steps))
    }
}

fn pixel_index(plane: &SlabPlane, column: u32, row: u32) -> [f64; 3] {
    let along_column = scale3(&plane.column, column);
    let along_row = scale3(&plane.row, row);
    add3(add3(plane.origin_index, along_column), along_row)
}

/// `M` maps an index delta to patient millimetres. Its column `i` is
/// `spacing[i]` times `direction.col(i)`. `d = M^{-1} n` for the unit normal
/// `n`. The step in millimetres is `1 / length(d)`. The index step is
/// `d / length(d)`.
fn step_along(volume: &Volume, normal: [f64; 3]) -> Option<([f64; 3], f64)> {
    let unit = unit_normal(normal)?;
    if !positive_finite(volume.spacing[0])
        || !positive_finite(volume.spacing[1])
        || !positive_finite(volume.spacing[2])
    {
        return None;
    }
    let map = DMat3::from_cols(
        volume.direction.col(0) * volume.spacing[0],
        volume.direction.col(1) * volume.spacing[1],
        volume.direction.col(2) * volume.spacing[2],
    );
    let det = map.determinant();
    if !det.is_finite() || is_zero(det) {
        return None;
    }
    let d = map.inverse() * unit;
    if !d.x.is_finite() || !d.y.is_finite() || !d.z.is_finite() {
        return None;
    }
    let length = d.length();
    if !length.is_finite() || length <= 0.0 {
        return None;
    }
    let step = d / length;
    let step_mm = 1.0 / length;
    if !step.x.is_finite()
        || !step.y.is_finite()
        || !step.z.is_finite()
        || !step_mm.is_finite()
        || step_mm <= 0.0
    {
        return None;
    }
    Some(([step.x, step.y, step.z], step_mm))
}

fn included_steps(step_mm: f64, half_mm: f64) -> Option<u32> {
    if !step_mm.is_finite() || step_mm <= 0.0 || !half_mm.is_finite() || half_mm < 0.0 {
        return None;
    }
    if is_zero(half_mm) {
        return Some(0);
    }
    let ratio = half_mm / step_mm;
    let mut steps = u32_from_nonneg_floor(ratio)?;
    loop {
        if steps == 0 {
            break;
        }
        let distance = f64::from(steps) * step_mm;
        if distance.is_finite() && distance <= half_mm {
            break;
        }
        steps -= 1;
    }
    if let Some(next) = steps.checked_add(1) {
        let distance = f64::from(next) * step_mm;
        if distance.is_finite() && distance <= half_mm {
            steps = next;
        }
    }
    Some(steps)
}

fn unit_normal(normal: [f64; 3]) -> Option<DVec3> {
    if !normal[0].is_finite() || !normal[1].is_finite() || !normal[2].is_finite() {
        return None;
    }
    let vector = DVec3::new(normal[0], normal[1], normal[2]);
    let length = vector.length();
    if !length.is_finite() || length <= 0.0 {
        return None;
    }
    Some(vector / length)
}

fn positive_finite(value: f64) -> bool {
    value.is_finite() && value > 0.0
}

fn is_zero(value: f64) -> bool {
    let bits = value.to_bits();
    bits == 0 || bits == 1_u64 << 63
}

fn scale3(value: &[f64; 3], steps: u32) -> [f64; 3] {
    let factor = f64::from(steps);
    [value[0] * factor, value[1] * factor, value[2] * factor]
}

fn add3(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    [left[0] + right[0], left[1] + right[1], left[2] + right[2]]
}

fn sub3(left: [f64; 3], right: [f64; 3]) -> [f64; 3] {
    [left[0] - right[0], left[1] - right[1], left[2] - right[2]]
}
