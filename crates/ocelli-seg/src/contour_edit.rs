//! Labelmap edit from a closed index polygon.
//!
//! F-085. Even-odd fill plus the boundary is the approved plan. It is not
//! an HLD formula and it is not the F-064 contour. PS3.3 C.8.20.2 keeps
//! segment 0 as background, and writing 0 erases.

use crate::brush::{require_stack, same_number};
use crate::labelmap::LabelGrid;

/// One polygon vertex in index units.
#[derive(Debug, Clone, Copy)]
pub struct IndexPoint {
    /// Column.
    pub x: f64,
    /// Row.
    pub y: f64,
}

/// How many samples the edit covered and how many of those changed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContourEditCoverage {
    /// Samples inside the polygon, including the boundary.
    pub covered: u32,
    /// Samples whose stored number differed from the written segment.
    pub changed: u32,
}

/// Why an edit wrote nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContourEditError {
    /// A vertex is not finite, or an intersection is not finite.
    Frame,
    /// Fewer than three points, or the grid is not one rectangle.
    Degenerate,
}

/// Paint `segment` on samples inside the closed polygon.
///
/// The ring closes from the last point to the first. A refusal leaves the
/// grid unchanged.
///
/// # Errors
///
/// [`ContourEditError::Frame`] when a vertex or an intersection is not finite.
/// [`ContourEditError::Degenerate`] when fewer than three points are supplied
/// or the grid is empty, has a zero side, or has the wrong length.
pub fn edit_labelmap_with_contour(
    grid: &mut LabelGrid,
    polygon: &[IndexPoint],
    segment: u16,
) -> Result<ContourEditCoverage, ContourEditError> {
    if polygon.len() < 3 {
        return Err(ContourEditError::Degenerate);
    }
    for point in polygon {
        if !point.x.is_finite() || !point.y.is_finite() {
            return Err(ContourEditError::Frame);
        }
    }
    let (width, height) =
        require_stack(core::slice::from_ref(grid)).map_err(|_| ContourEditError::Degenerate)?;
    let mut staged = grid.indices.clone();
    let mut covered = 0_u32;
    let mut changed = 0_u32;
    for y in 0..height {
        for x in 0..width {
            if !inside_polygon(f64::from(x), f64::from(y), polygon)? {
                continue;
            }
            let Some(row) = y.checked_mul(width) else {
                return Err(ContourEditError::Degenerate);
            };
            let Some(offset) = row.checked_add(x) else {
                return Err(ContourEditError::Degenerate);
            };
            let Ok(index) = usize::try_from(offset) else {
                return Err(ContourEditError::Degenerate);
            };
            let Some(slot) = staged.get_mut(index) else {
                return Err(ContourEditError::Degenerate);
            };
            covered = covered.saturating_add(1);
            if *slot != segment {
                changed = changed.saturating_add(1);
                *slot = segment;
            }
        }
    }
    grid.indices = staged;
    Ok(ContourEditCoverage { covered, changed })
}

fn inside_polygon(px: f64, py: f64, polygon: &[IndexPoint]) -> Result<bool, ContourEditError> {
    let count = polygon.len();
    for index in 0..count {
        let start = point_at(polygon, index)?;
        let end = point_at(polygon, index.saturating_add(1) % count)?;
        if on_segment(px, py, start, end) {
            return Ok(true);
        }
    }
    let mut crossings = 0_u32;
    for index in 0..count {
        let start = point_at(polygon, index)?;
        let end = point_at(polygon, index.saturating_add(1) % count)?;
        if same_number(start.y, end.y) {
            continue;
        }
        if !straddles(start.y, end.y, py) {
            continue;
        }
        let t_den = end.y - start.y;
        let x_hit = start.x + (py - start.y) * (end.x - start.x) / t_den;
        if !x_hit.is_finite() {
            return Err(ContourEditError::Frame);
        }
        if same_number(x_hit, px) {
            return Ok(true);
        }
        if x_hit.total_cmp(&px).is_gt() {
            crossings = crossings.saturating_add(1);
        }
    }
    Ok(crossings % 2 == 1)
}

fn point_at(polygon: &[IndexPoint], index: usize) -> Result<IndexPoint, ContourEditError> {
    polygon
        .get(index)
        .copied()
        .ok_or(ContourEditError::Degenerate)
}

fn on_segment(px: f64, py: f64, start: IndexPoint, end: IndexPoint) -> bool {
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    if same_number(dx, 0.0) && same_number(dy, 0.0) {
        return same_number(px, start.x) && same_number(py, start.y);
    }
    let sx = px - start.x;
    let sy = py - start.y;
    let cross = dx * sy - dy * sx;
    if !same_number(cross, 0.0) {
        return false;
    }
    let dot = sx * dx + sy * dy;
    let len_sq = dx * dx + dy * dy;
    let above = same_number(dot, 0.0) || dot.total_cmp(&0.0).is_gt();
    let below = same_number(dot, len_sq) || dot.total_cmp(&len_sq).is_lt();
    above && below
}

fn straddles(y1: f64, y2: f64, py: f64) -> bool {
    let y1_at_or_below = same_number(y1, py) || y1.total_cmp(&py).is_lt();
    let y2_above = y2.total_cmp(&py).is_gt();
    let y2_at_or_below = same_number(y2, py) || y2.total_cmp(&py).is_lt();
    let y1_above = y1.total_cmp(&py).is_gt();
    (y1_at_or_below && y2_above) || (y2_at_or_below && y1_above)
}
