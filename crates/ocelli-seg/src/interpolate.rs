//! Between-slice labelmap interpolation.
//!
//! F-086. The halfway rule is the approved plan. It is not an HLD formula.
//! PS3.3 C.8.20.2 keeps segment 0 as background, and writing 0 erases.

use crate::brush::require_stack;
use crate::labelmap::LabelGrid;
use alloc::vec::Vec;

/// How many between-slice samples the interpolation covered and changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InterpolateCoverage {
    /// Samples selected by the halfway rule.
    pub covered: u32,
    /// Samples whose stored number differed from the written segment.
    pub changed: u32,
}

/// Why an interpolation wrote nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterpolateError {
    /// The stack is empty, a side is 0, or the grids are not one rectangle.
    Degenerate,
}

/// Write `segment` on slices strictly between consecutive key slices.
///
/// A key slice already holds `segment`. A slice closer to the lower key
/// takes that mask. A slice closer to the upper key takes that mask. The
/// exact halfway slice takes the union. Endpoint slices stay. A refusal
/// leaves every grid unchanged.
///
/// # Errors
///
/// [`InterpolateError::Degenerate`] when the stack is empty, a side is 0,
/// the length is not `width * height`, or two slices disagree.
pub fn interpolate_labelmap(
    grids: &mut [LabelGrid],
    segment: u16,
) -> Result<InterpolateCoverage, InterpolateError> {
    let (width, height) = require_stack(grids).map_err(|_| InterpolateError::Degenerate)?;
    let keys = key_slices(grids, segment)?;
    if keys.len() < 2 {
        return Ok(InterpolateCoverage {
            covered: 0,
            changed: 0,
        });
    }
    let mut staged: Vec<Vec<u16>> = grids.iter().map(|grid| grid.indices.clone()).collect();
    let mut covered = 0_u32;
    let mut changed = 0_u32;
    for window in keys.windows(2) {
        let Some(z0) = window.first().copied() else {
            return Err(InterpolateError::Degenerate);
        };
        let Some(z1) = window.get(1).copied() else {
            return Err(InterpolateError::Degenerate);
        };
        let Some(span) = z1.checked_sub(z0) else {
            return Err(InterpolateError::Degenerate);
        };
        if span < 2 {
            continue;
        }
        for step in 1..span {
            let Some(z) = z0.checked_add(step) else {
                return Err(InterpolateError::Degenerate);
            };
            fill_slice(
                grids,
                &mut staged,
                width,
                height,
                FillRequest {
                    z0,
                    z1,
                    z,
                    t: f64::from(step) / f64::from(span),
                    segment,
                },
                &mut covered,
                &mut changed,
            )?;
        }
    }
    for (grid, next) in grids.iter_mut().zip(staged) {
        grid.indices = next;
    }
    Ok(InterpolateCoverage { covered, changed })
}

fn key_slices(grids: &[LabelGrid], segment: u16) -> Result<Vec<u32>, InterpolateError> {
    let mut keys = Vec::new();
    for (index, grid) in grids.iter().enumerate() {
        if grid.indices.contains(&segment) {
            let z = u32::try_from(index).map_err(|_| InterpolateError::Degenerate)?;
            keys.push(z);
        }
    }
    Ok(keys)
}

struct FillRequest {
    z0: u32,
    z1: u32,
    z: u32,
    t: f64,
    segment: u16,
}

fn fill_slice(
    grids: &[LabelGrid],
    staged: &mut [Vec<u16>],
    width: u32,
    height: u32,
    request: FillRequest,
    covered: &mut u32,
    changed: &mut u32,
) -> Result<(), InterpolateError> {
    let near_low = request.t.total_cmp(&0.5).is_le();
    let near_high = request.t.total_cmp(&0.5).is_ge();
    for y in 0..height {
        for x in 0..width {
            let offset = offset_of(width, x, y)?;
            let low = sample_at(grids, request.z0, offset)?;
            let high = sample_at(grids, request.z1, offset)?;
            let inside =
                (low == request.segment && near_low) || (high == request.segment && near_high);
            if !inside {
                continue;
            }
            let slot = staged_slot(staged, request.z, offset)?;
            *covered = covered.saturating_add(1);
            if *slot != request.segment {
                *changed = changed.saturating_add(1);
                *slot = request.segment;
            }
        }
    }
    Ok(())
}

fn sample_at(grids: &[LabelGrid], z: u32, offset: usize) -> Result<u16, InterpolateError> {
    let index = usize::try_from(z).map_err(|_| InterpolateError::Degenerate)?;
    let Some(grid) = grids.get(index) else {
        return Err(InterpolateError::Degenerate);
    };
    grid.indices
        .get(offset)
        .copied()
        .ok_or(InterpolateError::Degenerate)
}

fn staged_slot(
    staged: &mut [Vec<u16>],
    z: u32,
    offset: usize,
) -> Result<&mut u16, InterpolateError> {
    let index = usize::try_from(z).map_err(|_| InterpolateError::Degenerate)?;
    let Some(samples) = staged.get_mut(index) else {
        return Err(InterpolateError::Degenerate);
    };
    samples.get_mut(offset).ok_or(InterpolateError::Degenerate)
}

fn offset_of(width: u32, x: u32, y: u32) -> Result<usize, InterpolateError> {
    let row = y.checked_mul(width).ok_or(InterpolateError::Degenerate)?;
    let offset = row.checked_add(x).ok_or(InterpolateError::Degenerate)?;
    usize::try_from(offset).map_err(|_| InterpolateError::Degenerate)
}
