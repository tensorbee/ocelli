//! Contour stroke.
//!
//! DICOM PS3.3 C.8.8.6 names the geometric types. The points are patient
//! millimetres. F-064 projects them onto the world XY plane and strokes the
//! polyline. It does not fill and it does not force outline alpha.

use alloc::vec::Vec;

use ocelli_core::{Pt, World};

use crate::labelmap::LabelColourLut;

/// DICOM PS3.3 C.8.8.6 Contour Geometric Type, as a value rather than a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContourGeometricType {
    /// One point.
    Point,
    /// An open polyline on the draw plane.
    OpenPlanar,
    /// An open polyline. This presenter still requires the draw plane.
    OpenNonplanar,
    /// A closed polyline. The last point connects to the first.
    ClosedPlanar,
}

/// The world XY plane a contour is drawn on.
///
/// Column `c` is half-away rounding of `(x - origin.x) / spacing_x_mm`.
/// Row `r` is the same formula on `y`. 0.5 becomes 1 and -0.5 becomes -1.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PlaneFrame {
    /// Patient millimetres of pixel `(0, 0)`.
    pub origin: Pt<World>,
    /// Millimetres per column. Must be finite and greater than zero.
    pub spacing_x_mm: f64,
    /// Millimetres per row. Must be finite and greater than zero.
    pub spacing_y_mm: f64,
}

/// One contour. Segment 0 is background and has no colour.
#[derive(Debug, Clone, PartialEq)]
pub struct Contour {
    /// Segment number. Numbers start at 1.
    pub segment: u16,
    /// How the points connect.
    pub kind: ContourGeometricType,
    /// Patient millimetres, in order.
    pub points: Vec<Pt<World>>,
}

/// Why a contour was not written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContourError {
    /// Width or height is 0, or the stroke painted nothing inside the grid.
    Empty,
    /// The point list does not match the geometric type.
    Length,
    /// The segment has no colour. Nothing was written.
    MissingColour,
    /// A point is not on the frame's Z plane.
    OutOfPlane,
    /// The frame or a projected coordinate cannot be drawn.
    Frame,
    /// The destination is not four bytes per pixel.
    OutputLength,
}

/// Write the contour stroke into `out`.
///
/// A refusal leaves `out` unchanged. Pixels outside the grid are skipped.
/// The stroke uses the LUT alpha.
///
/// # Errors
///
/// [`ContourError`] when the contour, the frame or the destination cannot
/// be drawn.
pub fn present_contour(
    contour: &Contour,
    frame: &PlaneFrame,
    lut: &LabelColourLut,
    width: u32,
    height: u32,
    out: &mut [u8],
) -> Result<(), ContourError> {
    check_frame(frame)?;
    if width == 0 || height == 0 {
        return Err(ContourError::Empty);
    }
    if !kind_len(contour.kind, contour.points.len()) {
        return Err(ContourError::Length);
    }
    let width_us = usize::try_from(width).map_err(|_| ContourError::Empty)?;
    let height_us = usize::try_from(height).map_err(|_| ContourError::Empty)?;
    let pixels = width_us.checked_mul(height_us).ok_or(ContourError::Empty)?;
    let bytes = pixels.checked_mul(4).ok_or(ContourError::Empty)?;
    if out.len() != bytes {
        return Err(ContourError::OutputLength);
    }
    let colour = lut
        .colour_bytes(contour.segment)
        .ok_or(ContourError::MissingColour)?;
    let mut projected = Vec::new();
    for point in &contour.points {
        projected.push(project(frame, *point)?);
    }
    let width_i = i32::try_from(width).map_err(|_| ContourError::Frame)?;
    let height_i = i32::try_from(height).map_err(|_| ContourError::Frame)?;
    let mut pending = alloc::vec![0_u8; bytes];
    let painted = {
        let mut raster = Raster {
            pending: &mut pending,
            width: width_i,
            height: height_i,
            colour,
        };
        let mut painted = false;
        paint_projected(&mut raster, contour.kind, &projected, &mut painted)?;
        painted
    };
    if !painted {
        return Err(ContourError::Empty);
    }
    out.copy_from_slice(&pending);
    Ok(())
}

/// Cut `contour` by the patient plane `z = plane_z`.
///
/// A point is on the plane when its Z bits match `plane_z`, so `-0.0` and
/// `+0.0` stay different. A crossing uses `t = (plane_z - z0) / (z1 - z0)`
/// with `0 < t < 1` and stores `plane_z` itself. The whole contour is
/// returned unchanged when every point is already on the plane.
///
/// # Errors
///
/// [`ContourError::Frame`] for a non-finite plane or coordinate.
/// [`ContourError::Length`] when the geometric type does not match the
/// point count. [`ContourError::Empty`] when the plane misses the contour.
pub fn intersect_contour(contour: &Contour, plane_z: f64) -> Result<Contour, ContourError> {
    if !plane_z.is_finite() {
        return Err(ContourError::Frame);
    }
    if !kind_len(contour.kind, contour.points.len()) {
        return Err(ContourError::Length);
    }
    for point in &contour.points {
        if !point.x.is_finite() || !point.y.is_finite() || !point.z.is_finite() {
            return Err(ContourError::Frame);
        }
    }
    let mut entirely_on_plane = true;
    for point in &contour.points {
        if point.z.to_bits() != plane_z.to_bits() {
            entirely_on_plane = false;
            break;
        }
    }
    if entirely_on_plane {
        return Ok(Contour {
            segment: contour.segment,
            kind: contour.kind,
            points: contour.points.clone(),
        });
    }
    if contour.kind == ContourGeometricType::Point {
        return Err(ContourError::Empty);
    }
    let mut emitted = Vec::new();
    let count = contour.points.len();
    let segments = if contour.kind == ContourGeometricType::ClosedPlanar {
        count
    } else {
        count - 1
    };
    for index in 0..segments {
        let Some(start) = contour.points.get(index) else {
            return Err(ContourError::Length);
        };
        let next = if index + 1 == count { 0 } else { index + 1 };
        let Some(end) = contour.points.get(next) else {
            return Err(ContourError::Length);
        };
        emit_segment(&mut emitted, *start, *end, plane_z)?;
    }
    if emitted.is_empty() {
        return Err(ContourError::Empty);
    }
    let kind = if emitted.len() == 1 {
        ContourGeometricType::Point
    } else {
        ContourGeometricType::OpenPlanar
    };
    Ok(Contour {
        segment: contour.segment,
        kind,
        points: emitted,
    })
}

fn emit_segment(
    out: &mut Vec<Pt<World>>,
    start: Pt<World>,
    end: Pt<World>,
    plane_z: f64,
) -> Result<(), ContourError> {
    let start_on = start.z.to_bits() == plane_z.to_bits();
    let end_on = end.z.to_bits() == plane_z.to_bits();
    if start_on && end_on {
        push_unique(out, start);
        push_unique(out, end);
        return Ok(());
    }
    if start_on {
        push_unique(out, start);
        return Ok(());
    }
    if end_on {
        push_unique(out, end);
        return Ok(());
    }
    let start_below = start.z < plane_z;
    let end_below = end.z < plane_z;
    if start_below == end_below {
        return Ok(());
    }
    let dz = end.z - start.z;
    let t = (plane_z - start.z) / dz;
    if !t.is_finite() || t <= 0.0 || t >= 1.0 {
        return Ok(());
    }
    let x = start.x + t * (end.x - start.x);
    let y = start.y + t * (end.y - start.y);
    if !x.is_finite() || !y.is_finite() {
        return Err(ContourError::Frame);
    }
    push_unique(out, Pt::new(x, y, plane_z));
    Ok(())
}

fn push_unique(out: &mut Vec<Pt<World>>, point: Pt<World>) {
    if let Some(last) = out.last()
        && last.x.to_bits() == point.x.to_bits()
        && last.y.to_bits() == point.y.to_bits()
        && last.z.to_bits() == point.z.to_bits()
    {
        return;
    }
    out.push(point);
}

fn paint_projected(
    raster: &mut Raster<'_>,
    kind: ContourGeometricType,
    projected: &[(i32, i32)],
    painted: &mut bool,
) -> Result<(), ContourError> {
    if kind == ContourGeometricType::Point {
        let Some(&(x, y)) = projected.first() else {
            return Err(ContourError::Length);
        };
        *painted = plot(raster, x, y)?;
    } else {
        for index in 1..projected.len() {
            let Some(&(x0, y0)) = projected.get(index - 1) else {
                return Err(ContourError::Length);
            };
            let Some(&(x1, y1)) = projected.get(index) else {
                return Err(ContourError::Length);
            };
            if stroke(raster, x0, y0, x1, y1)? {
                *painted = true;
            }
        }
        if kind == ContourGeometricType::ClosedPlanar {
            let Some(&(x0, y0)) = projected.last() else {
                return Err(ContourError::Length);
            };
            let Some(&(x1, y1)) = projected.first() else {
                return Err(ContourError::Length);
            };
            if stroke(raster, x0, y0, x1, y1)? {
                *painted = true;
            }
        }
    }
    Ok(())
}

fn kind_len(kind: ContourGeometricType, count: usize) -> bool {
    match kind {
        ContourGeometricType::Point => count == 1,
        ContourGeometricType::OpenPlanar | ContourGeometricType::OpenNonplanar => count >= 2,
        ContourGeometricType::ClosedPlanar => count >= 3,
    }
}

fn check_frame(frame: &PlaneFrame) -> Result<(), ContourError> {
    if !frame.origin.x.is_finite() || !frame.origin.y.is_finite() || !frame.origin.z.is_finite() {
        return Err(ContourError::Frame);
    }
    if !frame.spacing_x_mm.is_finite() || !frame.spacing_y_mm.is_finite() {
        return Err(ContourError::Frame);
    }
    if frame.spacing_x_mm <= 0.0 || frame.spacing_y_mm <= 0.0 {
        return Err(ContourError::Frame);
    }
    Ok(())
}

fn project(frame: &PlaneFrame, point: Pt<World>) -> Result<(i32, i32), ContourError> {
    if !point.x.is_finite() || !point.y.is_finite() || !point.z.is_finite() {
        return Err(ContourError::Frame);
    }
    if point.z.to_bits() != frame.origin.z.to_bits() {
        return Err(ContourError::OutOfPlane);
    }
    let column = round_half_away((point.x - frame.origin.x) / frame.spacing_x_mm);
    let row = round_half_away((point.y - frame.origin.y) / frame.spacing_y_mm);
    Ok((rounded_i32(column)?, rounded_i32(row)?))
}

/// `f64::round` is not available in this `no_std` crate. Half steps go away
/// from zero, which is that function's rule. The added magnitude is just
/// under one half. Adding a full half turns the float below `0.5` into `1`.
fn round_half_away(value: f64) -> f64 {
    if !value.is_finite() {
        return value;
    }
    const DELTA: f64 = 0.5 - 0.25 * f64::EPSILON;
    let shifted = if value.is_sign_negative() {
        value - DELTA
    } else {
        value + DELTA
    };
    trunc_toward_zero(shifted)
}

fn trunc_toward_zero(value: f64) -> f64 {
    if !value.is_finite() {
        return value;
    }
    let bits = value.to_bits();
    let biased = (bits >> 52) & 0x7ff;
    let Ok(biased) = u32::try_from(biased) else {
        return value;
    };
    if biased < 1023 {
        return 0.0;
    }
    let unbiased = biased - 1023;
    if unbiased >= 52 {
        return value;
    }
    let drop = 52 - unbiased;
    let cleared = bits & !((1_u64 << drop) - 1);
    f64::from_bits(cleared)
}

fn rounded_i32(rounded: f64) -> Result<i32, ContourError> {
    if !rounded.is_finite() {
        return Err(ContourError::Frame);
    }
    if rounded > -1.0 && rounded < 1.0 {
        return Ok(0);
    }
    let negative = rounded < 0.0;
    let magnitude = if negative { -rounded } else { rounded };
    let whole = magnitude_u32(magnitude).ok_or(ContourError::Frame)?;
    if negative {
        if whole == 1_u32 << 31 {
            return Ok(i32::MIN);
        }
        let positive = i32::try_from(whole).map_err(|_| ContourError::Frame)?;
        return positive.checked_neg().ok_or(ContourError::Frame);
    }
    i32::try_from(whole).map_err(|_| ContourError::Frame)
}

/// Build one contour from patient-space RTSTRUCT points.
///
/// PS3.3 C.8.8.6 names the geometric type. The points stay in patient
/// millimetres. A z value is on the slice when its absolute distance from
/// `slice_z` is at most half of `spacing`. The stored z is the caller z.
/// Segment 0 is refused. This function does not project and it does not
/// call [`present_contour`].
///
/// # Errors
///
/// [`ContourError::MissingColour`] for segment 0.
/// [`ContourError::Length`] when the existing geometric-type length rule refuses the point count.
/// [`ContourError::OutOfPlane`] for a spacing that is not finite and
/// greater than 0, or for a z outside the slab.
/// [`ContourError::Frame`] for a non-finite point.
pub fn import_rtstruct_contour(
    segment: u16,
    kind: ContourGeometricType,
    points: &[Pt<World>],
    slice_z: f64,
    spacing: f64,
) -> Result<Contour, ContourError> {
    if segment == 0 {
        return Err(ContourError::MissingColour);
    }
    if !kind_len(kind, points.len()) {
        return Err(ContourError::Length);
    }
    if !spacing.is_finite() || spacing <= 0.0 || !slice_z.is_finite() {
        return Err(ContourError::OutOfPlane);
    }
    let half = spacing / 2.0;
    let mut stored = Vec::with_capacity(points.len());
    for point in points {
        if !point.x.is_finite() || !point.y.is_finite() || !point.z.is_finite() {
            return Err(ContourError::Frame);
        }
        let distance = (point.z - slice_z).abs();
        if distance > half {
            return Err(ContourError::OutOfPlane);
        }
        stored.push(Pt::new(point.x, point.y, point.z));
    }
    Ok(Contour {
        segment,
        kind,
        points: stored,
    })
}

fn magnitude_u32(value: f64) -> Option<u32> {
    if value > f64::from(u32::MAX) {
        return None;
    }
    let bits = value.to_bits();
    let biased = u32::try_from((bits >> 52) & 0x7ff).ok()?;
    if biased < 1023 {
        return Some(0);
    }
    let unbiased = biased - 1023;
    if unbiased >= 32 {
        return None;
    }
    let mantissa = (bits & ((1_u64 << 52) - 1)) | (1_u64 << 52);
    let shift = 52 - unbiased;
    u32::try_from(mantissa >> shift).ok()
}

struct Raster<'a> {
    pending: &'a mut [u8],
    width: i32,
    height: i32,
    colour: [u8; 4],
}

fn stroke(
    raster: &mut Raster<'_>,
    mut x0: i32,
    mut y0: i32,
    x1: i32,
    y1: i32,
) -> Result<bool, ContourError> {
    let dx = i64::from(x1) - i64::from(x0);
    let dy = i64::from(y1) - i64::from(y0);
    let adx = dx.checked_abs().ok_or(ContourError::Frame)?;
    let ady = dy.checked_abs().ok_or(ContourError::Frame)?;
    let sx: i32 = if dx >= 0 { 1 } else { -1 };
    let sy: i32 = if dy >= 0 { 1 } else { -1 };
    let mut err = adx.checked_sub(ady).ok_or(ContourError::Frame)?;
    let limit = u64::try_from(adx)
        .map_err(|_| ContourError::Frame)?
        .saturating_add(u64::try_from(ady).map_err(|_| ContourError::Frame)?)
        .saturating_add(1);
    let mut steps = 0_u64;
    let mut painted = false;
    loop {
        steps = steps.saturating_add(1);
        if steps > limit {
            return Err(ContourError::Frame);
        }
        if plot(raster, x0, y0)? {
            painted = true;
        }
        if x0 == x1 && y0 == y1 {
            break;
        }
        let e2 = err.checked_mul(2).ok_or(ContourError::Frame)?;
        let neg_dy = ady.checked_neg().ok_or(ContourError::Frame)?;
        if e2 > neg_dy {
            err = err.checked_sub(ady).ok_or(ContourError::Frame)?;
            x0 = x0.checked_add(sx).ok_or(ContourError::Frame)?;
        }
        if e2 < adx {
            err = err.checked_add(adx).ok_or(ContourError::Frame)?;
            y0 = y0.checked_add(sy).ok_or(ContourError::Frame)?;
        }
    }
    Ok(painted)
}

fn plot(raster: &mut Raster<'_>, x: i32, y: i32) -> Result<bool, ContourError> {
    if x < 0 || y < 0 || x >= raster.width || y >= raster.height {
        return Ok(false);
    }
    let x_u = u32::try_from(x).map_err(|_| ContourError::Frame)?;
    let y_u = u32::try_from(y).map_err(|_| ContourError::Frame)?;
    let width_u = u32::try_from(raster.width).map_err(|_| ContourError::Frame)?;
    let offset = y_u
        .checked_mul(width_u)
        .ok_or(ContourError::Frame)?
        .checked_add(x_u)
        .ok_or(ContourError::Frame)?;
    let start = offset.checked_mul(4).ok_or(ContourError::Frame)?;
    let start_us = usize::try_from(start).map_err(|_| ContourError::Frame)?;
    let end = start_us.checked_add(4).ok_or(ContourError::Frame)?;
    let Some(slot) = raster.pending.get_mut(start_us..end) else {
        return Err(ContourError::OutputLength);
    };
    slot.copy_from_slice(&raster.colour);
    Ok(true)
}
