//! Linked viewports: one frame-of-reference rule, the nearest coplanar
//! image and the reference line where one image plane cuts another.
//!
//! PS3.3 C.7.6.2.1.1 is implemented once, in [`ImagePlane::index_to_world`],
//! and its normal is [`ImagePlane::slice_normal`]. That section spans the
//! image plane with the row cosine `X` and the column cosine `Y`, which it
//! requires orthogonal and of unit length, so `X cross Y` is a unit normal
//! of the plane. It does not name the normal, and no rule here reads its
//! sign. PS3.3 10.7.1.3: Pixel Spacing is the row spacing, then the column
//! spacing. `ImagePlane` keeps that order.
//!
//! The behaviour is cornerstone3D 5.8.2's, transcribed in
//! `.claude/plans/F-209-design.md`: `areViewportsCoplanar`,
//! `imageSliceSyncCallback`'s `_getClosestImageIdIndex`, and
//! `ReferenceLinesTool` with `showFullDimension: false` and the whole source
//! image as its extent. Registration across frames of reference is not done.
//!
//! This module creates no device and holds no state.

use glam::DVec3;
use ocelli_core::{Index, Pt, World};
use ocelli_pixel::ImagePlane;
use ocelli_volume::FrameOfReferenceUid;

/// cornerstone3D 5.8.2 `areViewportsCoplanar`: `abs(dot) > 0.9`.
pub const COPLANAR_DOT: f64 = 0.9;

/// cornerstone3D 5.8.2 `EPSILON`, used by `isParallel` and `isPerpendicular`.
pub const PLANE_EPSILON: f64 = 1e-3;

/// Why a link moved nothing or drew nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinkError {
    /// The source and the shown target plane have `abs(dot) <= COPLANAR_DOT`.
    NotCoplanar,
    /// The two planes have `abs(dot) > 1 - PLANE_EPSILON`, so no line.
    Parallel,
    /// An empty stack, no finite distance, a non-finite intersection or
    /// index, or a target with a zero spacing on its singleton axis.
    Degenerate,
}

/// True only when both UIDs are present and equal as strings.
///
/// PS3.3 C.7.4.1.1.1: the Frame of Reference UID identifies a Frame of
/// Reference, and several series in a study may share one. A missing UID
/// identifies none, so it links nothing, not even to another missing one.
#[must_use]
pub fn same_frame_of_reference(
    a: Option<&FrameOfReferenceUid>,
    b: Option<&FrameOfReferenceUid>,
) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => a == b,
        _ => false,
    }
}

/// The index into `stack` whose Image Position (Patient) is nearest the
/// source's, by Euclidean distance in millimetres.
///
/// The comparison is strict, so a tie keeps the lower index. The caller
/// moves the target only when the answer differs from what it shows.
///
/// # Errors
///
/// [`LinkError::NotCoplanar`] when `abs(dot(source normal, shown normal))`
/// is not above [`COPLANAR_DOT`]. [`LinkError::Degenerate`] when no
/// distance is finite, which includes an empty stack.
pub fn closest_coplanar(
    source: &ImagePlane,
    shown: &ImagePlane,
    stack: &[ImagePlane],
) -> Result<usize, LinkError> {
    let dot = source.slice_normal().dot(shown.slice_normal()).abs();
    // `is_none_or` so a NaN dot, which compares as nothing, is refused too.
    if dot
        .partial_cmp(&COPLANAR_DOT)
        .is_none_or(|order| order.is_le())
    {
        return Err(LinkError::NotCoplanar);
    }
    let point = source.position_vector();
    let mut best: Option<(usize, f64)> = None;
    for (index, plane) in stack.iter().enumerate() {
        let distance = plane.position_vector().distance(point);
        let least = best.map_or(f64::INFINITY, |(_, least)| least);
        if distance < least {
            best = Some((index, distance));
        }
    }
    best.map(|(index, _)| index).ok_or(LinkError::Degenerate)
}

/// Where `source` cuts `target`, as two points in `target`'s index space.
///
/// The source corners are the centres of its four corner pixels. When the
/// source's top-bottom edge is perpendicular to the target normal, within
/// [`PLANE_EPSILON`], the edges cut are top and bottom, otherwise left and
/// right. Each edge is intersected with the target plane, and each point is
/// read back through the inverse of `target.index_to_world()`. Column is
/// `x` and row is `y`, both with the first pixel centre at `0`.
///
/// # Errors
///
/// [`LinkError::Parallel`] when `abs(dot)` of the two normals is above
/// `1 - PLANE_EPSILON`. [`LinkError::Degenerate`] when the target has a zero
/// spacing, which PS3.3 10.7.1.3 allows only on a single row or column and
/// where `index_to_world` substitutes a unit extension, or when a point is
/// not finite.
pub fn reference_segment(
    source: &ImagePlane,
    target: &ImagePlane,
) -> Result<(Pt<Index>, Pt<Index>), LinkError> {
    let normal = target.slice_normal();
    if source.slice_normal().dot(normal).abs() > 1.0 - PLANE_EPSILON {
        return Err(LinkError::Parallel);
    }
    let spacing = target.spacing();
    if spacing.row_mm() <= 0.0 || spacing.column_mm() <= 0.0 {
        return Err(LinkError::Degenerate);
    }
    let dimensions = source.dimensions();
    let last_column = f64::from(dimensions.columns().saturating_sub(1));
    let last_row = f64::from(dimensions.rows().saturating_sub(1));
    let to_world = source.index_to_world();
    let corner = |column: f64, row: f64| {
        let point = to_world.apply(Pt::<Index>::new(column, row, 0.0));
        DVec3::new(point.x, point.y, point.z)
    };
    let top_left = corner(0.0, 0.0);
    let top_right = corner(last_column, 0.0);
    let bottom_left = corner(0.0, last_row);
    let bottom_right = corner(last_column, last_row);
    // gl-matrix normalises a zero vector to zero, which then reads as
    // perpendicular, so `normalize_or_zero` keeps a one-row source on the
    // oracle's branch.
    let top_bottom = (top_left - bottom_left).normalize_or_zero();
    let [(start_a, start_b), (end_a, end_b)] = if top_bottom.dot(normal).abs() < PLANE_EPSILON {
        [(top_left, top_right), (bottom_left, bottom_right)]
    } else {
        [(top_left, bottom_left), (top_right, bottom_right)]
    };
    let offset = normal.dot(target.position_vector());
    let start = cut(start_a, start_b, normal, offset)?;
    let end = cut(end_a, end_b, normal, offset)?;
    let to_index = target.index_to_world().inverse();
    let start = to_index.apply(Pt::<World>::new(start.x, start.y, start.z));
    let end = to_index.apply(Pt::<World>::new(end.x, end.y, end.z));
    if finite(start) && finite(end) {
        Ok((start, end))
    } else {
        Err(LinkError::Degenerate)
    }
}

/// cornerstone3D 5.8.2 `linePlaneIntersection` for the plane
/// `normal . p = offset`. An edge parallel to the plane divides by zero and
/// is refused rather than drawn.
fn cut(from: DVec3, to: DVec3, normal: DVec3, offset: f64) -> Result<DVec3, LinkError> {
    let direction = to - from;
    let t = -(normal.dot(from) - offset) / normal.dot(direction);
    let point = from + direction * t;
    if point.is_finite() {
        Ok(point)
    } else {
        Err(LinkError::Degenerate)
    }
}

fn finite(point: Pt<Index>) -> bool {
    point.x.is_finite() && point.y.is_finite() && point.z.is_finite()
}
