//! Which samples a region contains.
//!
//! The HLD does not define it. The local rule is in
//! `.claude/plans/F-211-design.md`. A sample at column `c` and row `r` has
//! its centre at the index point `(c, r)`, and it belongs to a region when
//! that centre is inside the shape or on its boundary. The polygon rule is
//! the F-085 rule of `ocelli-seg`, even-odd plus the boundary.

use ocelli_core::{Index, Pt};

use crate::GeomError;

/// A region in index space. `z` is ignored.
#[derive(Debug, Clone, Copy)]
pub enum RegionShape<'a> {
    /// An axis-aligned box from two opposite corners.
    Rectangle {
        /// One corner.
        corner: Pt<Index>,
        /// The opposite corner.
        opposite: Pt<Index>,
    },
    /// An axis-aligned ellipse.
    Ellipse {
        /// The centre.
        center: Pt<Index>,
        /// Half the extent along columns.
        rx: f64,
        /// Half the extent along rows.
        ry: f64,
    },
    /// A ring of three or more vertices, closed from the last to the first.
    Polygon {
        /// The vertices in order.
        points: &'a [Pt<Index>],
    },
}

/// Inclusive sample bounds on a grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IndexBounds {
    /// First column, inclusive.
    pub first_column: u32,
    /// Last column, inclusive.
    pub last_column: u32,
    /// First row, inclusive.
    pub first_row: u32,
    /// Last row, inclusive.
    pub last_row: u32,
}

/// Whether the sample centre `(column, row)` is inside the shape or on its boundary.
///
/// Rectangle: `min(x0, x1) <= c <= max(x0, x1)` and the same for `r`.
/// Ellipse: `((c - cx) / rx)^2 + ((r - cy) / ry)^2 <= 1`, evaluated as
/// `((c - cx) * ry)^2 + ((r - cy) * rx)^2 <= (rx * ry)^2` so that half-pixel
/// inputs tie exactly. Polygon: even-odd, and a centre on an edge is inside.
///
/// # Errors
///
/// [`GeomError::Degenerate`] when an ellipse radius is zero or a polygon has
/// fewer than three points. [`GeomError::Frame`] when an input or an edge
/// product is not finite, or a product of two non-zero deltas flushed to zero.
pub fn region_contains(shape: &RegionShape<'_>, column: u32, row: u32) -> Result<bool, GeomError> {
    let c = f64::from(column);
    let r = f64::from(row);
    match *shape {
        RegionShape::Rectangle { corner, opposite } => {
            finite(corner)?;
            finite(opposite)?;
            let (low_x, high_x) = ordered(corner.x, opposite.x);
            let (low_y, high_y) = ordered(corner.y, opposite.y);
            Ok(low_x <= c && c <= high_x && low_y <= r && r <= high_y)
        }
        RegionShape::Ellipse { center, rx, ry } => {
            let (rx, ry) = radii(center, rx, ry)?;
            let across = (c - center.x) * ry;
            let down = (r - center.y) * rx;
            let reach = rx * ry;
            let left = across * across + down * down;
            let right = reach * reach;
            if !left.is_finite() || !right.is_finite() {
                return Err(GeomError::Frame);
            }
            Ok(left <= right)
        }
        RegionShape::Polygon { points } => {
            ring(points)?;
            inside_polygon(c, r, points)
        }
    }
}

/// Sample bounds that hold every sample the shape contains, clipped to the grid.
///
/// `None` when the bounds hold no sample of the grid. A rectangle and a
/// polygon use their exact extent. An ellipse uses `cx - rx ..= cx + rx`
/// widened by one sample on each side, so a rounded extent cannot drop a
/// boundary sample that [`region_contains`] keeps.
///
/// # Errors
///
/// The refusals of [`region_contains`] for the shape's own inputs.
pub fn region_bounds(
    shape: &RegionShape<'_>,
    width: u32,
    height: u32,
) -> Result<Option<IndexBounds>, GeomError> {
    let (low_x, high_x, low_y, high_y) = match *shape {
        RegionShape::Rectangle { corner, opposite } => {
            finite(corner)?;
            finite(opposite)?;
            let (low_x, high_x) = ordered(corner.x, opposite.x);
            let (low_y, high_y) = ordered(corner.y, opposite.y);
            (low_x, high_x, low_y, high_y)
        }
        RegionShape::Ellipse { center, rx, ry } => {
            let (rx, ry) = radii(center, rx, ry)?;
            let low_x = center.x - rx - 1.0;
            let high_x = center.x + rx + 1.0;
            let low_y = center.y - ry - 1.0;
            let high_y = center.y + ry + 1.0;
            if !low_x.is_finite()
                || !high_x.is_finite()
                || !low_y.is_finite()
                || !high_y.is_finite()
            {
                return Err(GeomError::Frame);
            }
            (low_x, high_x, low_y, high_y)
        }
        RegionShape::Polygon { points } => {
            ring(points)?;
            let mut low_x = f64::INFINITY;
            let mut high_x = f64::NEG_INFINITY;
            let mut low_y = f64::INFINITY;
            let mut high_y = f64::NEG_INFINITY;
            for point in points {
                low_x = low_x.min(point.x);
                high_x = high_x.max(point.x);
                low_y = low_y.min(point.y);
                high_y = high_y.max(point.y);
            }
            (low_x, high_x, low_y, high_y)
        }
    };
    let Some((first_column, last_column)) = span(low_x, high_x, width) else {
        return Ok(None);
    };
    let Some((first_row, last_row)) = span(low_y, high_y, height) else {
        return Ok(None);
    };
    Ok(Some(IndexBounds {
        first_column,
        last_column,
        first_row,
        last_row,
    }))
}

fn finite(point: Pt<Index>) -> Result<(), GeomError> {
    if point.x.is_finite() && point.y.is_finite() {
        Ok(())
    } else {
        Err(GeomError::Frame)
    }
}

fn ordered(left: f64, right: f64) -> (f64, f64) {
    if left <= right {
        (left, right)
    } else {
        (right, left)
    }
}

/// The absolute radii, the convention of [`crate::ellipse`].
fn radii(center: Pt<Index>, rx: f64, ry: f64) -> Result<(f64, f64), GeomError> {
    finite(center)?;
    if !rx.is_finite() || !ry.is_finite() {
        return Err(GeomError::Frame);
    }
    if same_number(rx, 0.0) || same_number(ry, 0.0) {
        return Err(GeomError::Degenerate);
    }
    Ok((rx.abs(), ry.abs()))
}

/// The F-085 order: the count first, then finiteness.
fn ring(points: &[Pt<Index>]) -> Result<(), GeomError> {
    if points.len() < 3 {
        return Err(GeomError::Degenerate);
    }
    for point in points {
        finite(*point)?;
    }
    Ok(())
}

/// The inclusive whole-number range `low..=high` clipped to `0..extent`.
///
/// `f64::from(u32)` is exact, so each comparison is on the true sample index.
fn span(low: f64, high: f64, extent: u32) -> Option<(u32, u32)> {
    let last_index = extent.checked_sub(1)?;
    // Smallest index whose value is at least `low`.
    let first = lower_bound(extent, |index| f64::from(index) >= low)?;
    // Largest index whose value is at most `high`: one before the first above it.
    let last = match lower_bound(extent, |index| f64::from(index) > high) {
        Some(0) => return None,
        Some(above) => above.checked_sub(1)?,
        None => last_index,
    };
    (first <= last).then_some((first, last))
}

/// The smallest index in `0..extent` where a monotone predicate turns true.
fn lower_bound(extent: u32, holds: impl Fn(u32) -> bool) -> Option<u32> {
    let mut low = 0_u32;
    let mut high = extent;
    while low < high {
        let mid = low + (high - low) / 2;
        if holds(mid) {
            high = mid;
        } else {
            low = mid.checked_add(1)?;
        }
    }
    (low < extent).then_some(low)
}

/// Whether two edges of the closed ring that share no vertex index cross.
///
/// Edge `i` runs from vertex `i` to vertex `i + 1`, and the last edge closes
/// the ring. A crossing is proper: each edge has the other's endpoints
/// strictly on opposite sides, so a touch at a vertex or a collinear overlap
/// is not one. The test is every non-adjacent pair, quadratic in the count.
///
/// # Errors
///
/// [`GeomError::Degenerate`] when `points` has fewer than three elements.
/// [`GeomError::Frame`] when a point or an orientation product is not finite.
pub fn polygon_crosses_itself(points: &[Pt<Index>]) -> Result<bool, GeomError> {
    ring(points)?;
    let count = points.len();
    for first in 0..count {
        let a = point_at(points, first)?;
        let b = point_at(points, first.saturating_add(1) % count)?;
        for second in first.saturating_add(2)..count {
            // The last edge shares vertex 0 with the first edge.
            if first == 0 && second.saturating_add(1) == count {
                continue;
            }
            let c = point_at(points, second)?;
            let d = point_at(points, second.saturating_add(1) % count)?;
            let abc = orientation(a, b, c)?;
            let abd = orientation(a, b, d)?;
            let cda = orientation(c, d, a)?;
            let cdb = orientation(c, d, b)?;
            if opposite(abc, abd) && opposite(cda, cdb) {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

/// The sign of the turn from `a` to `b` to `c`, as the cross product.
fn orientation(a: Pt<Index>, b: Pt<Index>, c: Pt<Index>) -> Result<f64, GeomError> {
    let turn = (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x);
    if turn.is_finite() {
        Ok(turn)
    } else {
        Err(GeomError::Frame)
    }
}

/// Strictly opposite signs. A zero is on the line, not across it.
fn opposite(left: f64, right: f64) -> bool {
    (left > 0.0 && right < 0.0) || (left < 0.0 && right > 0.0)
}

/// Both zeros compare equal, the F-085 `same_number`.
fn same_number(left: f64, right: f64) -> bool {
    let zero = |value: f64| {
        value.to_bits() == 0.0_f64.to_bits() || value.to_bits() == (-0.0_f64).to_bits()
    };
    if zero(left) && zero(right) {
        return true;
    }
    left.total_cmp(&right).is_eq()
}

/// The F-085 `inside_polygon` of `ocelli-seg`, on index points.
fn inside_polygon(px: f64, py: f64, polygon: &[Pt<Index>]) -> Result<bool, GeomError> {
    let count = polygon.len();
    for index in 0..count {
        let start = point_at(polygon, index)?;
        let end = point_at(polygon, index.saturating_add(1) % count)?;
        if on_segment(px, py, start, end)? {
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
        let rise = py - start.y;
        let run = end.x - start.x;
        let numerator = rise * run;
        let offset = numerator / t_den;
        if !same_number(numerator, 0.0) && same_number(offset, 0.0) {
            return Err(GeomError::Frame);
        }
        let x_hit = start.x + offset;
        if !x_hit.is_finite() {
            return Err(GeomError::Frame);
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

fn point_at(polygon: &[Pt<Index>], index: usize) -> Result<Pt<Index>, GeomError> {
    polygon.get(index).copied().ok_or(GeomError::Degenerate)
}

fn on_segment(px: f64, py: f64, start: Pt<Index>, end: Pt<Index>) -> Result<bool, GeomError> {
    let dx = end.x - start.x;
    let dy = end.y - start.y;
    if same_number(dx, 0.0) && same_number(dy, 0.0) {
        return Ok(same_number(px, start.x) && same_number(py, start.y));
    }
    let sx = px - start.x;
    let sy = py - start.y;
    let dx_sy = dx * sy;
    let dy_sx = dy * sx;
    let sx_dx = sx * dx;
    let sy_dy = sy * dy;
    let dx_dx = dx * dx;
    let dy_dy = dy * dy;
    if product_underflowed(dx, sy, dx_sy)
        || product_underflowed(dy, sx, dy_sx)
        || product_underflowed(sx, dx, sx_dx)
        || product_underflowed(sy, dy, sy_dy)
        || product_underflowed(dx, dx, dx_dx)
        || product_underflowed(dy, dy, dy_dy)
    {
        return Err(GeomError::Frame);
    }
    let cross = dx_sy - dy_sx;
    let dot = sx_dx + sy_dy;
    let len_sq = dx_dx + dy_dy;
    if !cross.is_finite() || !dot.is_finite() || !len_sq.is_finite() {
        return Err(GeomError::Frame);
    }
    if !same_number(cross, 0.0) {
        return Ok(false);
    }
    let above = same_number(dot, 0.0) || dot.total_cmp(&0.0).is_gt();
    let below = same_number(dot, len_sq) || dot.total_cmp(&len_sq).is_lt();
    Ok(above && below)
}

/// Two non-zero factors whose product flushed to zero.
fn product_underflowed(left: f64, right: f64, product: f64) -> bool {
    !same_number(left, 0.0) && !same_number(right, 0.0) && same_number(product, 0.0)
}

fn straddles(y1: f64, y2: f64, py: f64) -> bool {
    let y1_at_or_below = same_number(y1, py) || y1.total_cmp(&py).is_lt();
    let y2_above = !same_number(y2, py) && y2.total_cmp(&py).is_gt();
    let y2_at_or_below = same_number(y2, py) || y2.total_cmp(&py).is_lt();
    let y1_above = !same_number(y1, py) && y1.total_cmp(&py).is_gt();
    (y1_at_or_below && y2_above) || (y2_at_or_below && y1_above)
}
