//! Circle and sphere stamps on a label grid.
//!
//! F-080. The squared-distance rule is the approved plan. It is not an HLD
//! formula. PS3.3 C.8.20.2 keeps segment 0 as background, and writing 0 erases.

use crate::labelmap::LabelGrid;
use alloc::vec::Vec;

const NEGATIVE_ZERO_BITS: u64 = 0x8000_0000_0000_0000;

/// Which distance the stamp uses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrushShape {
    /// `x` and `y` on the one slice named by `center_z`.
    Circle,
    /// `x`, `y` and `z` on every supplied slice.
    Sphere,
}

/// One stamp. The centre and the radius are index units.
#[derive(Debug, Clone, Copy)]
pub struct Brush {
    /// Circle or sphere.
    pub shape: BrushShape,
    /// Column of the centre.
    pub center_x: f64,
    /// Row of the centre.
    pub center_y: f64,
    /// Slice of the centre.
    pub center_z: f64,
    /// Radius in index units. Both zero bit patterns are a zero radius.
    pub radius: f64,
    /// Segment number to write. 0 erases.
    pub segment: u16,
}

/// How many samples the stamp covered and how many of those changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BrushCoverage {
    /// Samples inside the brush, including ones already holding `segment`.
    pub covered: u32,
    /// Samples whose stored number differed from `segment`.
    pub changed: u32,
}

/// Why a stamp wrote nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BrushError {
    /// The centre or the radius is not finite.
    Frame,
    /// The radius is negative, or the grids are not one stack of equal slices.
    Degenerate,
}

/// Write `brush.segment` into every sample on or inside the radius.
///
/// A refusal leaves every grid unchanged.
///
/// # Errors
///
/// [`BrushError::Frame`] when the centre or the radius is not finite.
/// [`BrushError::Degenerate`] when the radius is negative or the grids do
/// not form one stack of equal non-empty slices.
pub fn stamp_brush(grids: &mut [LabelGrid], brush: Brush) -> Result<BrushCoverage, BrushError> {
    if !brush.center_x.is_finite()
        || !brush.center_y.is_finite()
        || !brush.center_z.is_finite()
        || !brush.radius.is_finite()
    {
        return Err(BrushError::Frame);
    }
    if strictly_negative(brush.radius) {
        return Err(BrushError::Degenerate);
    }
    let Some(first) = grids.first() else {
        return Err(BrushError::Degenerate);
    };
    let width = first.width;
    let height = first.height;
    if width == 0 || height == 0 {
        return Err(BrushError::Degenerate);
    }
    let Some(width_usize) = usize::try_from(width).ok() else {
        return Err(BrushError::Degenerate);
    };
    let Some(height_usize) = usize::try_from(height).ok() else {
        return Err(BrushError::Degenerate);
    };
    let Some(pixels) = width_usize.checked_mul(height_usize) else {
        return Err(BrushError::Degenerate);
    };
    for grid in grids.iter() {
        if grid.width != width || grid.height != height || grid.indices.len() != pixels {
            return Err(BrushError::Degenerate);
        }
    }

    let radius_sq = brush.radius * brush.radius;
    let mut staged = Vec::new();
    for grid in grids.iter() {
        staged.push(grid.indices.clone());
    }

    let mut covered = 0_u32;
    let mut changed = 0_u32;
    for (slice, samples) in staged.iter_mut().enumerate() {
        let Ok(z) = u32::try_from(slice) else {
            return Err(BrushError::Degenerate);
        };
        let z_f = f64::from(z);
        if matches!(brush.shape, BrushShape::Circle) && !same_number(z_f, brush.center_z) {
            continue;
        }
        for y in 0..height {
            for x in 0..width {
                if !inside(&brush, x, y, z_f, radius_sq) {
                    continue;
                }
                let Some(row) = y.checked_mul(width) else {
                    return Err(BrushError::Degenerate);
                };
                let Some(offset) = row.checked_add(x) else {
                    return Err(BrushError::Degenerate);
                };
                let Ok(index) = usize::try_from(offset) else {
                    return Err(BrushError::Degenerate);
                };
                let Some(slot) = samples.get_mut(index) else {
                    return Err(BrushError::Degenerate);
                };
                covered = covered.saturating_add(1);
                if *slot != brush.segment {
                    changed = changed.saturating_add(1);
                    *slot = brush.segment;
                }
            }
        }
    }

    for (grid, samples) in grids.iter_mut().zip(staged) {
        grid.indices = samples;
    }
    Ok(BrushCoverage { covered, changed })
}

fn inside(brush: &Brush, x: u32, y: u32, z_f: f64, radius_sq: f64) -> bool {
    let dx = f64::from(x) - brush.center_x;
    let dy = f64::from(y) - brush.center_y;
    let dz = match brush.shape {
        BrushShape::Circle => 0.0,
        BrushShape::Sphere => z_f - brush.center_z,
    };
    let dist_sq = dx * dx + dy * dy + dz * dz;
    dist_sq.total_cmp(&radius_sq).is_le()
}

fn strictly_negative(value: f64) -> bool {
    value.total_cmp(&0.0).is_lt() && value.to_bits() != NEGATIVE_ZERO_BITS
}

fn same_number(left: f64, right: f64) -> bool {
    let left_bits = left.to_bits();
    let right_bits = right.to_bits();
    let left_zero = left_bits == 0 || left_bits == NEGATIVE_ZERO_BITS;
    let right_zero = right_bits == 0 || right_bits == NEGATIVE_ZERO_BITS;
    if left_zero && right_zero {
        return true;
    }
    left.total_cmp(&right).is_eq()
}
