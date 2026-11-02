//! Hit-testing, projection and measurement mathematics.
//!
//! The HLD names this crate and does not give these formulas. The rules are
//! the local decisions in `.claude/plans/F-070-design.md`. Points stay in
//! the space the caller already chose. This module does not convert a
//! canvas point into a world point.

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

fn length(value: (f64, f64, f64)) -> f64 {
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
    let span = length(sub(end, start));
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
        let span = length(sub(end, start));
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
    let u_len = length(u);
    let v_len = length(v);
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
    let cross_len = length(cross);
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
    let span = length(sub(point, foot));
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
