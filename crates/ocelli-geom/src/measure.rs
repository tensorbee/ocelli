//! Hit-testing, projection and measurement mathematics.
//!
//! The HLD names this crate and does not give these formulas. F-070 rules
//! are the local decisions in `.claude/plans/F-070-design.md`. F-073 rules
//! are the local decisions in `.claude/plans/F-073-design.md`. F-074 rules
//! are the local decisions in `.claude/plans/F-074-design.md`. `length` of
//! fewer than two points is degenerate. `polyline_length` of one finite
//! point stays 0. Height is the absolute caller-space `z` difference, not
//! the Euclidean length. Cobb uses the absolute dot and stays in `0..=PI/2`.
//! A rectangle uses absolute `x` and `y` only. An ellipse area is `PI`
//! times the absolute radii, and a zero radius is degenerate. A circle
//! radius is the Euclidean distance, and identical points are degenerate.
//! F-076 rules are the local decisions in `.claude/plans/F-076-design.md`.
//! A polygon area is half the absolute shoelace sum, and the function
//! closes the ring. Fewer than three points is degenerate. A spline point
//! uses the uniform Catmull-Rom basis. `t` outside `0..=1` is degenerate.
//! Points stay in the space the caller already chose. This module does not
//! convert a canvas point into a world point.

use glam::{DVec2, DVec3};
use ocelli_core::Pt;

/// A refused measurement.
///
/// `Frame` is 400 and `Degenerate` is 401. Those numbers sit in the
/// `ocelli-geom` range. They are not boundary [`ocelli_core::ErrorCode`]
/// values, because nothing here writes the event ring.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GeomError {
    /// A coordinate, a tolerance or a result is not finite, or the tolerance
    /// is negative.
    Frame = 400,
    /// A segment, a plane normal or an angle arm has length zero.
    Degenerate = 401,
}

impl GeomError {
    /// The stable number. A match, not a cast.
    #[must_use]
    pub const fn number(self) -> u16 {
        match self {
            Self::Frame => 400,
            Self::Degenerate => 401,
        }
    }
}

/// The closest point on a finite segment, the clamped parameter, and the
/// distance from the query point to that foot.
#[derive(Debug)]
pub struct SegmentProjection<S> {
    /// `start + t * (end - start)` with `t` in `0..=1`.
    pub point: Pt<S>,
    /// Zero at `start` and one at `end`.
    pub t: f64,
    /// Euclidean distance from the query point to `point`, in the units of `S`.
    pub distance: f64,
}

impl<S> Clone for SegmentProjection<S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S> Copy for SegmentProjection<S> {}

fn finite_point<S>(point: Pt<S>) -> Result<(), GeomError> {
    if point.x.is_finite() && point.y.is_finite() && point.z.is_finite() {
        Ok(())
    } else {
        Err(GeomError::Frame)
    }
}

fn exact_zero(value: f64) -> bool {
    value.to_bits() == 0.0_f64.to_bits() || value.to_bits() == (-0.0_f64).to_bits()
}

fn sub<S>(end: Pt<S>, start: Pt<S>) -> (f64, f64, f64) {
    (end.x - start.x, end.y - start.y, end.z - start.z)
}

fn dot(left: (f64, f64, f64), right: (f64, f64, f64)) -> f64 {
    left.0 * right.0 + left.1 * right.1 + left.2 * right.2
}

fn span_length(value: (f64, f64, f64)) -> f64 {
    // `f64::sqrt` is not in `core` for this `no_std` crate. glam's libm
    // backend is the same policy the sigmoid uses in `ocelli-pixel`.
    DVec3::new(value.0, value.1, value.2).length()
}

/// Euclidean distance. A point and itself have distance 0.
///
/// # Errors
///
/// [`GeomError::Frame`] when either point is non-finite, or when the
/// difference overflows.
pub fn distance<S>(start: Pt<S>, end: Pt<S>) -> Result<f64, GeomError> {
    finite_point(start)?;
    finite_point(end)?;
    let span = span_length(sub(end, start));
    if span.is_finite() {
        Ok(span)
    } else {
        Err(GeomError::Frame)
    }
}

/// Sum of the segment lengths, in order.
///
/// Zero points or one finite point have length 0. A non-finite coordinate
/// refuses the whole sum.
///
/// # Errors
///
/// [`GeomError::Frame`] when any point is non-finite, or when a running
/// total overflows.
pub fn polyline_length<S>(points: &[Pt<S>]) -> Result<f64, GeomError> {
    for point in points {
        finite_point(*point)?;
    }
    let mut total = 0.0;
    for pair in points.windows(2) {
        let Some(start) = pair.first().copied() else {
            continue;
        };
        let Some(end) = pair.get(1).copied() else {
            continue;
        };
        let span = span_length(sub(end, start));
        if !span.is_finite() {
            return Err(GeomError::Frame);
        }
        total += span;
        if !total.is_finite() {
            return Err(GeomError::Frame);
        }
    }
    Ok(total)
}

/// Unsigned angle at `vertex`, in radians, in `0..=PI`.
///
/// The value is `atan2` of the cross-product length and the dot product of
/// the two arms. A zero-length arm is degenerate.
///
/// # Errors
///
/// [`GeomError::Frame`] when a point or the angle is non-finite.
/// [`GeomError::Degenerate`] when either arm has length zero.
pub fn angle_at<S>(arm_start: Pt<S>, vertex: Pt<S>, arm_end: Pt<S>) -> Result<f64, GeomError> {
    finite_point(arm_start)?;
    finite_point(vertex)?;
    finite_point(arm_end)?;
    let u = sub(arm_start, vertex);
    let v = sub(arm_end, vertex);
    let u_len = span_length(u);
    let v_len = span_length(v);
    if !u_len.is_finite() || !v_len.is_finite() {
        return Err(GeomError::Frame);
    }
    if exact_zero(u_len) || exact_zero(v_len) {
        return Err(GeomError::Degenerate);
    }
    let cross = (
        u.1 * v.2 - u.2 * v.1,
        u.2 * v.0 - u.0 * v.2,
        u.0 * v.1 - u.1 * v.0,
    );
    let cross_len = span_length(cross);
    let dotted = dot(u, v);
    if !cross_len.is_finite() || !dotted.is_finite() {
        return Err(GeomError::Frame);
    }
    // `to_angle` is `atan2(y, x)`. `y` is the cross length, so the result
    // stays in `0..=PI` for a non-negative cross length.
    let angle = DVec2::new(dotted, cross_len).to_angle();
    if angle.is_finite() {
        Ok(angle)
    } else {
        Err(GeomError::Frame)
    }
}

/// Orthogonal foot on the finite segment from `start` to `end`.
///
/// `t = dot(point - start, end - start) / dot(end - start, end - start)`,
/// then clamped into `0..=1`. An exact zero-length segment is degenerate.
///
/// # Errors
///
/// [`GeomError::Frame`] when an input or the foot is non-finite.
/// [`GeomError::Degenerate`] when `start` and `end` are the same point.
pub fn project_onto_segment<S>(
    point: Pt<S>,
    start: Pt<S>,
    end: Pt<S>,
) -> Result<SegmentProjection<S>, GeomError> {
    finite_point(point)?;
    finite_point(start)?;
    finite_point(end)?;
    let direction = sub(end, start);
    let len2 = dot(direction, direction);
    if !len2.is_finite() {
        return Err(GeomError::Frame);
    }
    if exact_zero(len2) {
        return Err(GeomError::Degenerate);
    }
    let raw = dot(sub(point, start), direction) / len2;
    let t = if raw < 0.0 {
        0.0
    } else if raw > 1.0 {
        1.0
    } else if raw.is_finite() {
        raw
    } else {
        return Err(GeomError::Frame);
    };
    let foot = Pt::new(
        start.x + t * direction.0,
        start.y + t * direction.1,
        start.z + t * direction.2,
    );
    finite_point(foot)?;
    let span = span_length(sub(point, foot));
    if !span.is_finite() {
        return Err(GeomError::Frame);
    }
    Ok(SegmentProjection {
        point: foot,
        t,
        distance: span,
    })
}

/// Whether the distance from `point` to the finite segment is at most
/// `tolerance`. The comparison includes the boundary.
///
/// A miss is `Ok(false)`. A negative tolerance is [`GeomError::Frame`] and
/// is reported before a zero-length segment is reported as degenerate.
///
/// # Errors
///
/// [`GeomError::Frame`] when an input is non-finite or `tolerance` is
/// negative. [`GeomError::Degenerate`] when the segment has length zero and
/// the tolerance is finite and non-negative.
pub fn hit_segment<S>(
    point: Pt<S>,
    start: Pt<S>,
    end: Pt<S>,
    tolerance: f64,
) -> Result<bool, GeomError> {
    finite_point(point)?;
    finite_point(start)?;
    finite_point(end)?;
    if !tolerance.is_finite() || tolerance < 0.0 {
        return Err(GeomError::Frame);
    }
    let projection = project_onto_segment(point, start, end)?;
    Ok(projection.distance <= tolerance)
}

/// Orthogonal projection of `point` onto the plane through `origin`.
///
/// `normal` is divided by its length. It does not have to be a unit vector.
/// An exact zero normal is degenerate.
///
/// # Errors
///
/// [`GeomError::Frame`] when an input or the foot is non-finite.
/// [`GeomError::Degenerate`] when `normal` has length zero.
pub fn project_onto_plane<S>(
    point: Pt<S>,
    origin: Pt<S>,
    normal: Pt<S>,
) -> Result<Pt<S>, GeomError> {
    finite_point(point)?;
    finite_point(origin)?;
    finite_point(normal)?;
    let n = DVec3::new(normal.x, normal.y, normal.z);
    let len = n.length();
    if !len.is_finite() {
        return Err(GeomError::Frame);
    }
    if exact_zero(len) {
        return Err(GeomError::Degenerate);
    }
    let unit = n / len;
    let signed = DVec3::new(point.x - origin.x, point.y - origin.y, point.z - origin.z).dot(unit);
    if !signed.is_finite() {
        return Err(GeomError::Frame);
    }
    let foot = Pt::new(
        point.x - signed * unit.x,
        point.y - signed * unit.y,
        point.z - signed * unit.z,
    );
    finite_point(foot)?;
    Ok(foot)
}

/// The longer and shorter of two segment lengths.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bidirectional {
    /// The larger Euclidean length. Equal lengths use that shared value.
    pub length: f64,
    /// The smaller Euclidean length.
    pub width: f64,
}

/// Polyline length. Fewer than two points is degenerate.
///
/// This is the F-073 length tool. It calls [`polyline_length`] after the
/// count check, so a one-point list is not reported as 0.
///
/// # Errors
///
/// [`GeomError::Degenerate`] when `points` has fewer than two elements.
/// [`GeomError::Frame`] when [`polyline_length`] refuses.
pub fn length<S>(points: &[Pt<S>]) -> Result<f64, GeomError> {
    if points.len() < 2 {
        return Err(GeomError::Degenerate);
    }
    polyline_length(points)
}

/// Absolute difference of the caller-space `z` coordinates.
///
/// This is the F-073 height tool. It does not convert spaces and it does
/// not pick an image axis.
///
/// # Errors
///
/// [`GeomError::Frame`] when either point or the difference is non-finite.
pub fn height<S>(start: Pt<S>, end: Pt<S>) -> Result<f64, GeomError> {
    finite_point(start)?;
    finite_point(end)?;
    let delta = (end.z - start.z).abs();
    if delta.is_finite() {
        Ok(delta)
    } else {
        Err(GeomError::Frame)
    }
}

/// Acute angle between two undirected segments, in radians, in `0..=PI/2`.
///
/// Reversing either segment does not change the value. A zero-length
/// segment is degenerate.
///
/// # Errors
///
/// [`GeomError::Frame`] when a point or the angle is non-finite.
/// [`GeomError::Degenerate`] when either segment has length zero.
pub fn cobb<S>(
    first_start: Pt<S>,
    first_end: Pt<S>,
    second_start: Pt<S>,
    second_end: Pt<S>,
) -> Result<f64, GeomError> {
    finite_point(first_start)?;
    finite_point(first_end)?;
    finite_point(second_start)?;
    finite_point(second_end)?;
    let first = sub(first_end, first_start);
    let second = sub(second_end, second_start);
    let first_len = span_length(first);
    let second_len = span_length(second);
    if !first_len.is_finite() || !second_len.is_finite() {
        return Err(GeomError::Frame);
    }
    if exact_zero(first_len) || exact_zero(second_len) {
        return Err(GeomError::Degenerate);
    }
    let cross = (
        first.1 * second.2 - first.2 * second.1,
        first.2 * second.0 - first.0 * second.2,
        first.0 * second.1 - first.1 * second.0,
    );
    let cross_len = span_length(cross);
    let dotted = dot(first, second).abs();
    if !cross_len.is_finite() || !dotted.is_finite() {
        return Err(GeomError::Frame);
    }
    let angle = DVec2::new(dotted, cross_len).to_angle();
    if angle.is_finite() {
        Ok(angle)
    } else {
        Err(GeomError::Frame)
    }
}

/// Longer and shorter Euclidean lengths of two segments.
///
/// Identical endpoints stay length 0, because that distance is exact.
///
/// # Errors
///
/// [`GeomError::Frame`] when [`distance`] refuses either segment.
pub fn bidirectional<S>(
    first_start: Pt<S>,
    first_end: Pt<S>,
    second_start: Pt<S>,
    second_end: Pt<S>,
) -> Result<Bidirectional, GeomError> {
    let first = distance(first_start, first_end)?;
    let second = distance(second_start, second_end)?;
    // `float_cmp` is denied. Both lengths are finite, so `total_cmp` orders
    // them without treating an equal pair as unordered.
    let (longer, shorter) = match first.total_cmp(&second) {
        core::cmp::Ordering::Less => (second, first),
        core::cmp::Ordering::Equal | core::cmp::Ordering::Greater => (first, second),
    };
    Ok(Bidirectional {
        length: longer,
        width: shorter,
    })
}

/// Width, height and area of a rectangle in the caller plane.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rectangle {
    /// Absolute difference of the `x` coordinates.
    pub width: f64,
    /// Absolute difference of the `y` coordinates.
    pub height: f64,
    /// `width * height`. `z` is not a factor.
    pub area: f64,
}

/// Radius and area of a circle in the caller's space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Circle {
    /// Euclidean distance from the centre to the rim.
    pub radius: f64,
    /// `PI * radius * radius`.
    pub area: f64,
}

/// Rectangle from two opposite corners. `z` is ignored.
///
/// # Errors
///
/// [`GeomError::Frame`] when a point or the area is non-finite.
pub fn rectangle<S>(corner: Pt<S>, opposite: Pt<S>) -> Result<Rectangle, GeomError> {
    finite_point(corner)?;
    finite_point(opposite)?;
    let width = (opposite.x - corner.x).abs();
    let height = (opposite.y - corner.y).abs();
    let area = width * height;
    if width.is_finite() && height.is_finite() && area.is_finite() {
        Ok(Rectangle {
            width,
            height,
            area,
        })
    } else {
        Err(GeomError::Frame)
    }
}

/// Ellipse area, `PI * abs(rx) * abs(ry)`.
///
/// The product of the absolute radii is taken first, so radii 2 and 3 are
/// `PI * 6`. A zero radius is degenerate rather than area 0.
///
/// # Errors
///
/// [`GeomError::Frame`] when a radius or the area is non-finite.
/// [`GeomError::Degenerate`] when either radius is exact zero.
pub fn ellipse(rx: f64, ry: f64) -> Result<f64, GeomError> {
    if !rx.is_finite() || !ry.is_finite() {
        return Err(GeomError::Frame);
    }
    if exact_zero(rx) || exact_zero(ry) {
        return Err(GeomError::Degenerate);
    }
    let area = core::f64::consts::PI * (rx.abs() * ry.abs());
    if area.is_finite() {
        Ok(area)
    } else {
        Err(GeomError::Frame)
    }
}

/// Circle from a centre and a rim point.
///
/// Identical points are degenerate. The radius of those points is 0, and
/// area 0 is not a circle.
///
/// # Errors
///
/// [`GeomError::Frame`] when [`distance`] refuses, or when the area is
/// non-finite.
/// [`GeomError::Degenerate`] when the two points are identical.
pub fn circle<S>(centre: Pt<S>, rim: Pt<S>) -> Result<Circle, GeomError> {
    let radius = distance(centre, rim)?;
    if exact_zero(radius) {
        return Err(GeomError::Degenerate);
    }
    let area = core::f64::consts::PI * (radius * radius);
    if area.is_finite() {
        Ok(Circle { radius, area })
    } else {
        Err(GeomError::Frame)
    }
}

/// Absolute shoelace area in the caller plane. The ring is closed here.
///
/// Fewer than three points is degenerate. `z` is not a factor.
///
/// # Errors
///
/// [`GeomError::Degenerate`] when `points` has fewer than three elements.
/// [`GeomError::Frame`] when a point or the running sum is non-finite.
pub fn polygon_area<S>(points: &[Pt<S>]) -> Result<f64, GeomError> {
    if points.len() < 3 {
        return Err(GeomError::Degenerate);
    }
    for point in points {
        finite_point(*point)?;
    }
    let mut sum = 0.0;
    for index in 0..points.len() {
        let Some(current) = points.get(index).copied() else {
            continue;
        };
        let next_index = if index + 1 == points.len() {
            0
        } else {
            let Some(index) = index.checked_add(1) else {
                return Err(GeomError::Frame);
            };
            index
        };
        let Some(next) = points.get(next_index).copied() else {
            continue;
        };
        let cross = current.x * next.y - next.x * current.y;
        if !cross.is_finite() {
            return Err(GeomError::Frame);
        }
        sum += cross;
        if !sum.is_finite() {
            return Err(GeomError::Frame);
        }
    }
    let area = sum.abs() * 0.5;
    if area.is_finite() {
        Ok(area)
    } else {
        Err(GeomError::Frame)
    }
}

/// One point on a uniform Catmull-Rom segment.
///
/// The curve passes through the input points. Segment 0 repeats the first
/// point as the previous ghost. The last segment repeats the last point as
/// the next ghost. `t` is in `0..=1`.
///
/// # Errors
///
/// [`GeomError::Degenerate`] when `t` is outside `0..=1`, or when `segment`
/// has no following point.
/// [`GeomError::Frame`] when a used point or the result is non-finite.
pub fn spline_point<S>(points: &[Pt<S>], segment: usize, t: f64) -> Result<Pt<S>, GeomError> {
    if !(0.0..=1.0).contains(&t) {
        return Err(GeomError::Degenerate);
    }
    let Some(next_index) = segment.checked_add(1) else {
        return Err(GeomError::Degenerate);
    };
    let Some(start) = points.get(segment).copied() else {
        return Err(GeomError::Degenerate);
    };
    let Some(end) = points.get(next_index).copied() else {
        return Err(GeomError::Degenerate);
    };
    let previous = if segment == 0 {
        start
    } else {
        let Some(point) = segment
            .checked_sub(1)
            .and_then(|index| points.get(index))
            .copied()
        else {
            return Err(GeomError::Degenerate);
        };
        point
    };
    let following = match next_index.checked_add(1) {
        Some(index) => points.get(index).copied().unwrap_or(end),
        None => end,
    };
    finite_point(previous)?;
    finite_point(start)?;
    finite_point(end)?;
    finite_point(following)?;
    let pm = DVec3::new(previous.x, previous.y, previous.z);
    let p0 = DVec3::new(start.x, start.y, start.z);
    let p1 = DVec3::new(end.x, end.y, end.z);
    let pn = DVec3::new(following.x, following.y, following.z);
    let t2 = t * t;
    let t3 = t2 * t;
    let combined = p0 * 2.0
        + (p1 - pm) * t
        + (pm * 2.0 - p0 * 5.0 + p1 * 4.0 - pn) * t2
        + (p0 * 3.0 - pm - p1 * 3.0 + pn) * t3;
    let q = combined * 0.5;
    if !q.x.is_finite() || !q.y.is_finite() || !q.z.is_finite() {
        return Err(GeomError::Frame);
    }
    Ok(Pt::new(q.x, q.y, q.z))
}
