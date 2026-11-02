//! Rectangle, circle and sphere erasers.
//!
//! F-081. A scissor writes segment 0. The circle and the sphere call the
//! F-080 stamp. The rectangle span is the approved plan. It is not an HLD
//! formula. PS3.3 C.8.20.2 keeps segment 0 as background.

use crate::brush::{
    Brush, BrushError, BrushShape, index_in_span, require_stack, same_number, span_inverted,
    stamp_brush,
};
use crate::labelmap::LabelGrid;
use alloc::vec::Vec;

/// Which samples the scissor erases.
#[derive(Debug, Clone, Copy)]
pub enum ScissorShape {
    /// Inclusive index bounds on the slice named by `z`.
    Rectangle {
        /// Lowest column, inclusive.
        min_x: f64,
        /// Lowest row, inclusive.
        min_y: f64,
        /// Highest column, inclusive.
        max_x: f64,
        /// Highest row, inclusive.
        max_y: f64,
        /// Slice index. Both zero bit patterns select slice 0.
        z: f64,
    },
    /// The F-080 circle, writing 0.
    Circle {
        /// Column of the centre.
        center_x: f64,
        /// Row of the centre.
        center_y: f64,
        /// Slice of the centre.
        center_z: f64,
        /// Radius in index units.
        radius: f64,
    },
    /// The F-080 sphere, writing 0.
    Sphere {
        /// Column of the centre.
        center_x: f64,
        /// Row of the centre.
        center_y: f64,
        /// Slice of the centre.
        center_z: f64,
        /// Radius in index units.
        radius: f64,
    },
}

/// How many samples the scissor covered and how many of those changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScissorCoverage {
    /// Samples inside the scissor, including ones that were already 0.
    pub covered: u32,
    /// Samples whose stored number was not already 0.
    pub changed: u32,
}

/// Why a scissor wrote nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScissorError {
    /// A bound, a centre, a radius or a slice is not finite.
    Frame,
    /// The span is inverted, the radius is negative, or the grids are not one stack.
    Degenerate,
}

/// Erase every sample inside `shape` to segment 0.
///
/// A refusal leaves every grid unchanged.
///
/// # Errors
///
/// [`ScissorError::Frame`] when a bound, centre, radius or slice is not finite.
/// [`ScissorError::Degenerate`] when the span is inverted, the radius is
/// negative, or the grids do not form one stack of equal non-empty slices.
pub fn cut_scissor(
    grids: &mut [LabelGrid],
    shape: ScissorShape,
) -> Result<ScissorCoverage, ScissorError> {
    match shape {
        ScissorShape::Circle {
            center_x,
            center_y,
            center_z,
            radius,
        } => from_brush(stamp_brush(
            grids,
            Brush {
                shape: BrushShape::Circle,
                center_x,
                center_y,
                center_z,
                radius,
                segment: 0,
            },
        )),
        ScissorShape::Sphere {
            center_x,
            center_y,
            center_z,
            radius,
        } => from_brush(stamp_brush(
            grids,
            Brush {
                shape: BrushShape::Sphere,
                center_x,
                center_y,
                center_z,
                radius,
                segment: 0,
            },
        )),
        ScissorShape::Rectangle {
            min_x,
            min_y,
            max_x,
            max_y,
            z,
        } => cut_rectangle(grids, min_x, min_y, max_x, max_y, z),
    }
}

fn from_brush(
    result: Result<crate::brush::BrushCoverage, BrushError>,
) -> Result<ScissorCoverage, ScissorError> {
    match result {
        Ok(coverage) => Ok(ScissorCoverage {
            covered: coverage.covered,
            changed: coverage.changed,
        }),
        Err(BrushError::Frame) => Err(ScissorError::Frame),
        Err(BrushError::Degenerate) => Err(ScissorError::Degenerate),
    }
}

fn cut_rectangle(
    grids: &mut [LabelGrid],
    min_x: f64,
    min_y: f64,
    max_x: f64,
    max_y: f64,
    z: f64,
) -> Result<ScissorCoverage, ScissorError> {
    if !min_x.is_finite()
        || !min_y.is_finite()
        || !max_x.is_finite()
        || !max_y.is_finite()
        || !z.is_finite()
    {
        return Err(ScissorError::Frame);
    }
    if span_inverted(min_x, max_x) || span_inverted(min_y, max_y) {
        return Err(ScissorError::Degenerate);
    }
    let (width, height) = require_stack(grids).map_err(|_| ScissorError::Degenerate)?;
    let mut staged = Vec::new();
    for grid in grids.iter() {
        staged.push(grid.indices.clone());
    }
    let mut covered = 0_u32;
    let mut changed = 0_u32;
    for (slice, samples) in staged.iter_mut().enumerate() {
        let Ok(z_index) = u32::try_from(slice) else {
            return Err(ScissorError::Degenerate);
        };
        if !same_number(f64::from(z_index), z) {
            continue;
        }
        for y in 0..height {
            for x in 0..width {
                if !index_in_span(x, min_x, max_x) || !index_in_span(y, min_y, max_y) {
                    continue;
                }
                let Some(row) = y.checked_mul(width) else {
                    return Err(ScissorError::Degenerate);
                };
                let Some(offset) = row.checked_add(x) else {
                    return Err(ScissorError::Degenerate);
                };
                let Ok(index) = usize::try_from(offset) else {
                    return Err(ScissorError::Degenerate);
                };
                let Some(slot) = samples.get_mut(index) else {
                    return Err(ScissorError::Degenerate);
                };
                covered = covered.saturating_add(1);
                if *slot != 0 {
                    changed = changed.saturating_add(1);
                    *slot = 0;
                }
            }
        }
    }
    for (grid, samples) in grids.iter_mut().zip(staged) {
        grid.indices = samples;
    }
    Ok(ScissorCoverage { covered, changed })
}
