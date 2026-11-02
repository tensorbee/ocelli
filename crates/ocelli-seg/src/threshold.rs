//! Rectangle and circle start-end threshold.
//!
//! F-083. The value span is the approved plan. It is not an HLD formula.
//! The circle uses the F-080 disk helper. This function does not read DICOM.
//! PS3.3 C.8.20.2 keeps segment 0 as background, and writing 0 erases.

use crate::brush::{
    disk_contains, index_in_span, number_in_span, require_stack, span_inverted, strictly_negative,
};
use crate::labelmap::LabelGrid;

/// Which samples are eligible for the value test.
#[derive(Debug, Clone, Copy)]
pub enum ThresholdShape {
    /// Inclusive index bounds on this grid.
    Rectangle {
        /// Lowest column, inclusive.
        min_x: f64,
        /// Lowest row, inclusive.
        min_y: f64,
        /// Highest column, inclusive.
        max_x: f64,
        /// Highest row, inclusive.
        max_y: f64,
    },
    /// The F-080 disk.
    Circle {
        /// Column of the centre.
        center_x: f64,
        /// Row of the centre.
        center_y: f64,
        /// Radius in index units.
        radius: f64,
    },
}

/// One start-end paint. The caller owns the value buffer.
#[derive(Debug, Clone, Copy)]
pub struct ThresholdRoi {
    /// Rectangle or circle.
    pub shape: ThresholdShape,
    /// Lowest included value. Both zero bit patterns match.
    pub start: f64,
    /// Highest included value. Both zero bit patterns match.
    pub end: f64,
    /// Segment number to write. 0 erases.
    pub segment: u16,
}

/// How many samples the threshold covered and how many of those changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThresholdCoverage {
    /// Samples inside the shape and inside the value span.
    pub covered: u32,
    /// Samples whose stored number differed from `segment`.
    pub changed: u32,
}

/// Why a threshold wrote nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThresholdError {
    /// A parameter is not finite, or a value inside the shape is not finite.
    Frame,
    /// The span is inverted, the radius is negative, or the buffers disagree.
    Degenerate,
}

/// Paint `roi.segment` where a value is inside the span and the shape.
///
/// A value outside the shape is not a reason to refuse. A refusal leaves
/// the grid unchanged.
///
/// # Errors
///
/// [`ThresholdError::Frame`] when a parameter is not finite, or a value
/// inside the shape is not finite.
/// [`ThresholdError::Degenerate`] when the span is inverted, the radius is
/// negative, or the grid and the value buffer are not one rectangle.
pub fn threshold_roi(
    grid: &mut LabelGrid,
    values: &[f64],
    roi: ThresholdRoi,
) -> Result<ThresholdCoverage, ThresholdError> {
    match roi.shape {
        ThresholdShape::Rectangle {
            min_x,
            min_y,
            max_x,
            max_y,
        } => {
            if !min_x.is_finite() || !min_y.is_finite() || !max_x.is_finite() || !max_y.is_finite()
            {
                return Err(ThresholdError::Frame);
            }
            if span_inverted(min_x, max_x) || span_inverted(min_y, max_y) {
                return Err(ThresholdError::Degenerate);
            }
            paint(grid, values, roi.start, roi.end, roi.segment, |x, y| {
                index_in_span(x, min_x, max_x) && index_in_span(y, min_y, max_y)
            })
        }
        ThresholdShape::Circle {
            center_x,
            center_y,
            radius,
        } => {
            if !center_x.is_finite() || !center_y.is_finite() || !radius.is_finite() {
                return Err(ThresholdError::Frame);
            }
            if strictly_negative(radius) {
                return Err(ThresholdError::Degenerate);
            }
            let radius_sq = radius * radius;
            paint(grid, values, roi.start, roi.end, roi.segment, |x, y| {
                disk_contains(center_x, center_y, radius_sq, x, y)
            })
        }
    }
}

fn paint(
    grid: &mut LabelGrid,
    values: &[f64],
    start: f64,
    end: f64,
    segment: u16,
    inside: impl Fn(u32, u32) -> bool,
) -> Result<ThresholdCoverage, ThresholdError> {
    if !start.is_finite() || !end.is_finite() {
        return Err(ThresholdError::Frame);
    }
    if span_inverted(start, end) {
        return Err(ThresholdError::Degenerate);
    }
    let (width, height) =
        require_stack(core::slice::from_ref(grid)).map_err(|_| ThresholdError::Degenerate)?;
    let Some(expected) = usize::try_from(width).ok().and_then(|side| {
        usize::try_from(height)
            .ok()
            .and_then(|rows| side.checked_mul(rows))
    }) else {
        return Err(ThresholdError::Degenerate);
    };
    if values.len() != expected || grid.indices.len() != expected {
        return Err(ThresholdError::Degenerate);
    }
    let mut staged = grid.indices.clone();
    let mut covered = 0_u32;
    let mut changed = 0_u32;
    for y in 0..height {
        for x in 0..width {
            if !inside(x, y) {
                continue;
            }
            let Some(row) = y.checked_mul(width) else {
                return Err(ThresholdError::Degenerate);
            };
            let Some(offset) = row.checked_add(x) else {
                return Err(ThresholdError::Degenerate);
            };
            let Ok(index) = usize::try_from(offset) else {
                return Err(ThresholdError::Degenerate);
            };
            let Some(value) = values.get(index).copied() else {
                return Err(ThresholdError::Degenerate);
            };
            if !value.is_finite() {
                return Err(ThresholdError::Frame);
            }
            if !number_in_span(value, start, end) {
                continue;
            }
            let Some(slot) = staged.get_mut(index) else {
                return Err(ThresholdError::Degenerate);
            };
            covered = covered.saturating_add(1);
            if *slot != segment {
                changed = changed.saturating_add(1);
                *slot = segment;
            }
        }
    }
    grid.indices = staged;
    Ok(ThresholdCoverage { covered, changed })
}
