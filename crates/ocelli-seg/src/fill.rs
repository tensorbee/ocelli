//! Flood fill and paint fill on one label grid.
//!
//! F-082. The 4-connected rule is the approved plan. It is not an HLD
//! formula. PS3.3 C.8.20.2 keeps segment 0 as background, and writing 0
//! erases.

use crate::labelmap::LabelGrid;
use alloc::collections::VecDeque;
use alloc::vec::Vec;

/// How many samples the fill covered and how many of those changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FillCoverage {
    /// Samples selected by the seed value.
    pub covered: u32,
    /// Samples whose stored number differed from the written segment.
    pub changed: u32,
}

/// Why a fill wrote nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FillError {
    /// The seed is outside, or the grid is not one dense rectangle.
    Degenerate,
}

/// Replace the 4-connected region of the seed value.
///
/// A diagonal neighbour is not connected. A refusal leaves the grid unchanged.
///
/// # Errors
///
/// [`FillError::Degenerate`] when the seed is outside the grid, a side is 0,
/// the length is not `width * height`, or the pixel count does not fit in `u32`.
pub fn flood_fill(
    grid: &mut LabelGrid,
    x: u32,
    y: u32,
    segment: u16,
) -> Result<FillCoverage, FillError> {
    let (width, height, seed) = prepare(grid, x, y)?;
    let mut staged = grid.indices.clone();
    let mut seen = Vec::new();
    seen.resize(staged.len(), false);
    let mut queue = VecDeque::new();
    mark_seed(&mut seen, &staged, width, x, y)?;
    queue.push_back((x, y));
    let coverage = walk_flood(
        &mut staged,
        &mut seen,
        &mut queue,
        width,
        height,
        seed,
        segment,
    )?;
    grid.indices = staged;
    Ok(coverage)
}

/// Replace every sample on this grid that holds the seed value.
///
/// Connection does not matter. A refusal leaves the grid unchanged.
///
/// # Errors
///
/// [`FillError::Degenerate`] when the seed is outside the grid, a side is 0,
/// the length is not `width * height`, or the pixel count does not fit in `u32`.
pub fn paint_fill(
    grid: &mut LabelGrid,
    x: u32,
    y: u32,
    segment: u16,
) -> Result<FillCoverage, FillError> {
    let (_width, _height, seed) = prepare(grid, x, y)?;
    let mut staged = grid.indices.clone();
    let mut covered = 0_u32;
    let mut changed = 0_u32;
    for slot in staged.iter_mut() {
        if *slot != seed {
            continue;
        }
        covered = covered.saturating_add(1);
        if *slot != segment {
            changed = changed.saturating_add(1);
            *slot = segment;
        }
    }
    grid.indices = staged;
    Ok(FillCoverage { covered, changed })
}

fn prepare(grid: &LabelGrid, x: u32, y: u32) -> Result<(u32, u32, u16), FillError> {
    if grid.width == 0 || grid.height == 0 {
        return Err(FillError::Degenerate);
    }
    let width_usize = usize::try_from(grid.width).map_err(|_| FillError::Degenerate)?;
    let height_usize = usize::try_from(grid.height).map_err(|_| FillError::Degenerate)?;
    let pixels = width_usize
        .checked_mul(height_usize)
        .ok_or(FillError::Degenerate)?;
    if u32::try_from(pixels).is_err() || grid.indices.len() != pixels {
        return Err(FillError::Degenerate);
    }
    if x >= grid.width || y >= grid.height {
        return Err(FillError::Degenerate);
    }
    let offset = offset_of(grid.width, x, y)?;
    let seed = grid
        .indices
        .get(offset)
        .copied()
        .ok_or(FillError::Degenerate)?;
    Ok((grid.width, grid.height, seed))
}

fn mark_seed(
    seen: &mut [bool],
    samples: &[u16],
    width: u32,
    x: u32,
    y: u32,
) -> Result<(), FillError> {
    let offset = offset_of(width, x, y)?;
    let Some(flag) = seen.get_mut(offset) else {
        return Err(FillError::Degenerate);
    };
    *flag = true;
    if samples.get(offset).is_none() {
        return Err(FillError::Degenerate);
    }
    Ok(())
}

fn walk_flood(
    samples: &mut [u16],
    seen: &mut [bool],
    queue: &mut VecDeque<(u32, u32)>,
    width: u32,
    height: u32,
    seed: u16,
    segment: u16,
) -> Result<FillCoverage, FillError> {
    let mut covered = 0_u32;
    let mut changed = 0_u32;
    while let Some((x, y)) = queue.pop_front() {
        let offset = offset_of(width, x, y)?;
        let Some(slot) = samples.get_mut(offset) else {
            return Err(FillError::Degenerate);
        };
        if *slot != seed {
            return Err(FillError::Degenerate);
        }
        covered = covered.saturating_add(1);
        if *slot != segment {
            changed = changed.saturating_add(1);
            *slot = segment;
        }
        if let Some(right) = x.checked_add(1) {
            consider(samples, seen, queue, width, height, (right, y), seed)?;
        }
        if let Some(left) = x.checked_sub(1) {
            consider(samples, seen, queue, width, height, (left, y), seed)?;
        }
        if let Some(down) = y.checked_add(1) {
            consider(samples, seen, queue, width, height, (x, down), seed)?;
        }
        if let Some(up) = y.checked_sub(1) {
            consider(samples, seen, queue, width, height, (x, up), seed)?;
        }
    }
    Ok(FillCoverage { covered, changed })
}

fn consider(
    samples: &[u16],
    seen: &mut [bool],
    queue: &mut VecDeque<(u32, u32)>,
    width: u32,
    height: u32,
    at: (u32, u32),
    seed: u16,
) -> Result<(), FillError> {
    let (x, y) = at;
    if x >= width || y >= height {
        return Ok(());
    }
    let offset = offset_of(width, x, y)?;
    let Some(flag) = seen.get_mut(offset) else {
        return Err(FillError::Degenerate);
    };
    if *flag {
        return Ok(());
    }
    let Some(value) = samples.get(offset).copied() else {
        return Err(FillError::Degenerate);
    };
    if value != seed {
        return Ok(());
    }
    *flag = true;
    queue.push_back((x, y));
    Ok(())
}

fn offset_of(width: u32, x: u32, y: u32) -> Result<usize, FillError> {
    let row = y.checked_mul(width).ok_or(FillError::Degenerate)?;
    let offset = row.checked_add(x).ok_or(FillError::Degenerate)?;
    usize::try_from(offset).map_err(|_| FillError::Degenerate)
}
