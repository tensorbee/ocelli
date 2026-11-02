//! Ultrasound direction and a rectangular redaction.
//!
//! The HLD does not give these formulas. The local rules are in
//! `.claude/plans/F-079-design.md`. The angle is the image plane. The
//! length includes `z`. A missed rectangle sets `applied` false.

use glam::{DVec2, DVec3};
use ocelli_core::Pt;

use crate::GeomError;

/// Image-plane angle and the Euclidean length of the same span.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Direction {
    /// `atan2(dy, dx)`. Zero is positive `x`. `z` is not a factor.
    pub radians: f64,
    /// Euclidean length. `z` is a factor.
    pub length: f64,
}

/// What one rectangle did to a sample buffer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Redaction {
    /// Samples changed to the fill.
    pub samples_written: u32,
    /// Samples in the intersection that already equalled the fill.
    pub samples_already_fill: u32,
    /// The intersection was non-empty.
    pub applied: bool,
}

fn finite_point<S>(point: Pt<S>) -> bool {
    point.x.is_finite() && point.y.is_finite() && point.z.is_finite()
}

fn exact_zero(value: f64) -> bool {
    value.to_bits() == 0.0_f64.to_bits() || value.to_bits() == (-0.0_f64).to_bits()
}

fn same_finite(left: f64, right: f64) -> bool {
    if exact_zero(left) && exact_zero(right) {
        return true;
    }
    left.total_cmp(&right).is_eq()
}

fn usize_from_u32(value: u32) -> Result<usize, GeomError> {
    usize::try_from(value).map_err(|_| GeomError::Degenerate)
}

fn flat_index(x: u32, y: u32, width: u32) -> Result<usize, GeomError> {
    let row = y.checked_mul(width).ok_or(GeomError::Degenerate)?;
    let index = row.checked_add(x).ok_or(GeomError::Degenerate)?;
    usize_from_u32(index)
}

fn axis_span(origin: u32, side: u32, limit: u32) -> Option<(u32, u32)> {
    if side == 0 || origin >= limit {
        return None;
    }
    let end = match origin.checked_add(side) {
        Some(sum) => sum.min(limit),
        None => limit,
    };
    if end <= origin {
        None
    } else {
        Some((origin, end))
    }
}

fn miss() -> Redaction {
    Redaction {
        samples_written: 0,
        samples_already_fill: 0,
        applied: false,
    }
}

/// Image-plane direction of two points.
///
/// # Errors
///
/// [`GeomError::Frame`] when a point or a result is non-finite.
/// [`GeomError::Degenerate`] when the span length is zero.
pub fn ultrasound_direction<S>(start: Pt<S>, end: Pt<S>) -> Result<Direction, GeomError> {
    if !finite_point(start) || !finite_point(end) {
        return Err(GeomError::Frame);
    }
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    let dz = end.z - start.z;
    if !dx.is_finite() || !dy.is_finite() || !dz.is_finite() {
        return Err(GeomError::Frame);
    }
    let length = DVec3::new(dx, dy, dz).length();
    if !length.is_finite() {
        return Err(GeomError::Frame);
    }
    if exact_zero(length) {
        return Err(GeomError::Degenerate);
    }
    let radians = DVec2::new(dx, dy).to_angle();
    if !radians.is_finite() {
        return Err(GeomError::Frame);
    }
    Ok(Direction { radians, length })
}

/// Write `fill` into the intersection of a rectangle and a row-major buffer.
///
/// # Errors
///
/// [`GeomError::Degenerate`] when the frame size is zero or the buffer length
/// disagrees. [`GeomError::Frame`] when the fill or a sample in the
/// intersection is non-finite. A refusal writes nothing.
// The approved plan names these eight arguments.
#[allow(clippy::too_many_arguments)]
pub fn redact_rect(
    samples: &mut [f64],
    width: u32,
    height: u32,
    x: u32,
    y: u32,
    rect_w: u32,
    rect_h: u32,
    fill: f64,
) -> Result<Redaction, GeomError> {
    if width == 0 || height == 0 {
        return Err(GeomError::Degenerate);
    }
    let cells = width.checked_mul(height).ok_or(GeomError::Degenerate)?;
    let needed = usize_from_u32(cells)?;
    if samples.len() != needed {
        return Err(GeomError::Degenerate);
    }
    if !fill.is_finite() {
        return Err(GeomError::Frame);
    }
    let Some((x0, x1)) = axis_span(x, rect_w, width) else {
        return Ok(miss());
    };
    let Some((y0, y1)) = axis_span(y, rect_h, height) else {
        return Ok(miss());
    };

    let mut samples_written = 0u32;
    let mut samples_already_fill = 0u32;
    for row in y0..y1 {
        for column in x0..x1 {
            let index = flat_index(column, row, width)?;
            let Some(sample) = samples.get(index).copied() else {
                return Err(GeomError::Degenerate);
            };
            if !sample.is_finite() {
                return Err(GeomError::Frame);
            }
            if same_finite(sample, fill) {
                samples_already_fill = match samples_already_fill.checked_add(1) {
                    Some(next) => next,
                    None => return Err(GeomError::Degenerate),
                };
            } else {
                samples_written = match samples_written.checked_add(1) {
                    Some(next) => next,
                    None => return Err(GeomError::Degenerate),
                };
            }
        }
    }

    for row in y0..y1 {
        for column in x0..x1 {
            let index = flat_index(column, row, width)?;
            let Some(slot) = samples.get_mut(index) else {
                return Err(GeomError::Degenerate);
            };
            if !same_finite(*slot, fill) {
                *slot = fill;
            }
        }
    }

    Ok(Redaction {
        samples_written,
        samples_already_fill,
        applied: true,
    })
}
