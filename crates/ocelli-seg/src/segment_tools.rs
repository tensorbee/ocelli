//! Segment select, label and bidirectional runs.
//!
//! F-084. The run rule is the approved plan. It is not an HLD formula.
//! PS3.3 C.8.20.2 keeps segment 0 as background.

use crate::labelmap::LabelGrid;
use alloc::string::String;
use alloc::vec::Vec;

/// A stored name for one segment above 0.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SegmentLabel {
    /// Segment number. Never 0.
    pub segment: u16,
    /// Trimmed text.
    pub text: String,
}

/// One inclusive run of a single segment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SegmentRun {
    /// First column.
    pub start_x: u32,
    /// First row.
    pub start_y: u32,
    /// Last column.
    pub end_x: u32,
    /// Last row.
    pub end_y: u32,
    /// Count of samples in the run.
    pub length: u32,
}

/// The longest horizontal run and the longest vertical run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SegmentBidirectional {
    /// Longest run inside one row.
    pub horizontal: SegmentRun,
    /// Longest run inside one column.
    pub vertical: SegmentRun,
}

/// Why a segment reading was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SegmentToolError {
    /// The grid or the requested run cannot be read.
    Degenerate,
    /// Segment 0 is background and cannot be labelled.
    Background,
    /// The trimmed text is empty.
    Blank,
}

/// Read the segment number at one column and row.
///
/// # Errors
///
/// [`SegmentToolError::Degenerate`] when the grid is empty, the buffer
/// length disagrees, or the column or row is outside the grid.
pub fn segment_at(grid: &LabelGrid, x: u32, y: u32) -> Result<u16, SegmentToolError> {
    pixel_count(grid)?;
    let index = offset(grid, x, y)?;
    grid.indices
        .get(index)
        .copied()
        .ok_or(SegmentToolError::Degenerate)
}

/// Store trimmed text for `segment`, replacing a previous label in place.
///
/// # Errors
///
/// [`SegmentToolError::Background`] for segment 0.
/// [`SegmentToolError::Blank`] when the trimmed text is empty.
/// Either refusal leaves `labels` unchanged.
pub fn label_segment(
    labels: &mut Vec<SegmentLabel>,
    segment: u16,
    text: &str,
) -> Result<(), SegmentToolError> {
    if segment == 0 {
        return Err(SegmentToolError::Background);
    }
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err(SegmentToolError::Blank);
    }
    if let Some(existing) = labels.iter_mut().find(|label| label.segment == segment) {
        existing.text = String::from(trimmed);
        return Ok(());
    }
    labels.push(SegmentLabel {
        segment,
        text: String::from(trimmed),
    });
    Ok(())
}

/// Longest horizontal and vertical runs of `segment`.
///
/// An equal length keeps the run found first. Rows are scanned from the
/// top. Columns are scanned from the left.
///
/// # Errors
///
/// [`SegmentToolError::Degenerate`] for segment 0, a bad grid, or a
/// segment that does not appear.
pub fn segment_bidirectional(
    grid: &LabelGrid,
    segment: u16,
) -> Result<SegmentBidirectional, SegmentToolError> {
    if segment == 0 {
        return Err(SegmentToolError::Degenerate);
    }
    pixel_count(grid)?;
    let Some(horizontal) = longest_horizontal(grid, segment)? else {
        return Err(SegmentToolError::Degenerate);
    };
    let Some(vertical) = longest_vertical(grid, segment)? else {
        return Err(SegmentToolError::Degenerate);
    };
    Ok(SegmentBidirectional {
        horizontal,
        vertical,
    })
}

fn pixel_count(grid: &LabelGrid) -> Result<usize, SegmentToolError> {
    if grid.width == 0 || grid.height == 0 {
        return Err(SegmentToolError::Degenerate);
    }
    let width = usize::try_from(grid.width).map_err(|_| SegmentToolError::Degenerate)?;
    let height = usize::try_from(grid.height).map_err(|_| SegmentToolError::Degenerate)?;
    let pixels = width
        .checked_mul(height)
        .ok_or(SegmentToolError::Degenerate)?;
    if grid.indices.len() != pixels {
        return Err(SegmentToolError::Degenerate);
    }
    Ok(pixels)
}

fn offset(grid: &LabelGrid, x: u32, y: u32) -> Result<usize, SegmentToolError> {
    if x >= grid.width || y >= grid.height {
        return Err(SegmentToolError::Degenerate);
    }
    let row = y
        .checked_mul(grid.width)
        .ok_or(SegmentToolError::Degenerate)?;
    let offset = row.checked_add(x).ok_or(SegmentToolError::Degenerate)?;
    usize::try_from(offset).map_err(|_| SegmentToolError::Degenerate)
}

fn sample(grid: &LabelGrid, x: u32, y: u32) -> Result<u16, SegmentToolError> {
    let index = offset(grid, x, y)?;
    grid.indices
        .get(index)
        .copied()
        .ok_or(SegmentToolError::Degenerate)
}

fn longest_horizontal(
    grid: &LabelGrid,
    segment: u16,
) -> Result<Option<SegmentRun>, SegmentToolError> {
    let mut best: Option<SegmentRun> = None;
    for y in 0..grid.height {
        let mut x = 0_u32;
        while x < grid.width {
            if sample(grid, x, y)? != segment {
                x += 1;
                continue;
            }
            let start = x;
            while x < grid.width && sample(grid, x, y)? == segment {
                x += 1;
            }
            let Some(end) = x.checked_sub(1) else {
                return Err(SegmentToolError::Degenerate);
            };
            let Some(length) = x.checked_sub(start) else {
                return Err(SegmentToolError::Degenerate);
            };
            let run = SegmentRun {
                start_x: start,
                start_y: y,
                end_x: end,
                end_y: y,
                length,
            };
            let replace = match best {
                Some(current) => length > current.length,
                None => true,
            };
            if replace {
                best = Some(run);
            }
        }
    }
    Ok(best)
}

fn longest_vertical(
    grid: &LabelGrid,
    segment: u16,
) -> Result<Option<SegmentRun>, SegmentToolError> {
    let mut best: Option<SegmentRun> = None;
    for x in 0..grid.width {
        let mut y = 0_u32;
        while y < grid.height {
            if sample(grid, x, y)? != segment {
                y += 1;
                continue;
            }
            let start = y;
            while y < grid.height && sample(grid, x, y)? == segment {
                y += 1;
            }
            let Some(end) = y.checked_sub(1) else {
                return Err(SegmentToolError::Degenerate);
            };
            let Some(length) = y.checked_sub(start) else {
                return Err(SegmentToolError::Degenerate);
            };
            let run = SegmentRun {
                start_x: x,
                start_y: start,
                end_x: x,
                end_y: end,
                length,
            };
            let replace = match best {
                Some(current) => length > current.length,
                None => true,
            };
            if replace {
                best = Some(run);
            }
        }
    }
    Ok(best)
}
