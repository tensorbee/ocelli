//! Triangle coverage for one segmentation surface.
//!
//! The caller supplies the mesh. This presenter does not build it. A pixel
//! is covered when the barycentric weights of its centre are all at least
//! zero. The edge is included. A zero-area triangle is skipped.

use alloc::vec::Vec;

use ocelli_core::{Pt, World};

use crate::contour::PlaneFrame;
use crate::labelmap::LabelColourLut;

/// One triangle mesh for a single segment.
#[derive(Debug, Clone, PartialEq)]
pub struct SurfaceMesh {
    /// Segment number. Numbers start at 1.
    pub segment: u16,
    /// Patient millimetres.
    pub vertices: Vec<Pt<World>>,
    /// Three indexes into `vertices` for each triangle.
    pub triangles: Vec<[u32; 3]>,
}

/// Why a surface was not written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceError {
    /// Width or height is 0, or no pixel centre was covered.
    Empty,
    /// A triangle index does not name a vertex.
    Index,
    /// The segment has no colour. Nothing was written.
    MissingColour,
    /// A vertex is not on the frame's Z plane.
    OutOfPlane,
    /// The frame or a projected coordinate cannot be drawn.
    Frame,
    /// The destination is not four bytes per pixel.
    OutputLength,
}

/// Write covered pixel centres into `out`.
///
/// A refusal leaves `out` unchanged. Vertices are not rounded. The texel
/// uses the LUT alpha.
///
/// # Errors
///
/// [`SurfaceError`] when the mesh, the frame or the destination cannot be
/// drawn.
pub fn present_surface(
    mesh: &SurfaceMesh,
    frame: &PlaneFrame,
    lut: &LabelColourLut,
    width: u32,
    height: u32,
    out: &mut [u8],
) -> Result<(), SurfaceError> {
    check_frame(frame)?;
    if width == 0 || height == 0 {
        return Err(SurfaceError::Empty);
    }
    let width_us = usize::try_from(width).map_err(|_| SurfaceError::Empty)?;
    let height_us = usize::try_from(height).map_err(|_| SurfaceError::Empty)?;
    let pixels = width_us.checked_mul(height_us).ok_or(SurfaceError::Empty)?;
    let bytes = pixels.checked_mul(4).ok_or(SurfaceError::Empty)?;
    if out.len() != bytes {
        return Err(SurfaceError::OutputLength);
    }
    let colour = lut
        .colour_bytes(mesh.segment)
        .ok_or(SurfaceError::MissingColour)?;
    for triangle in &mesh.triangles {
        let [i0, i1, i2] = *triangle;
        for index in [i0, i1, i2] {
            require_index(mesh.vertices.len(), index)?;
        }
    }
    let mut plane = Vec::new();
    for vertex in &mesh.vertices {
        plane.push(project(frame, *vertex)?);
    }
    let mut pending = alloc::vec![0_u8; bytes];
    let mut painted = false;
    for triangle in &mesh.triangles {
        let [i0, i1, i2] = *triangle;
        let a = vertex_at(&plane, i0)?;
        let b = vertex_at(&plane, i1)?;
        let c = vertex_at(&plane, i2)?;
        let abx = b.x - a.x;
        let aby = b.y - a.y;
        let acx = c.x - a.x;
        let acy = c.y - a.y;
        let den = abx * acy - acx * aby;
        if cross_is_zero(den) {
            continue;
        }
        if !den.is_finite() {
            return Err(SurfaceError::Frame);
        }
        let mut row = 0_u32;
        while row < height {
            let mut column = 0_u32;
            while column < width {
                let centre = PlanePoint {
                    x: f64::from(column) + 0.5,
                    y: f64::from(row) + 0.5,
                };
                if covered(a, abx, aby, acx, acy, den, centre)? {
                    paint(&mut pending, width, column, row, colour)?;
                    painted = true;
                }
                column = column.checked_add(1).ok_or(SurfaceError::Frame)?;
            }
            row = row.checked_add(1).ok_or(SurfaceError::Frame)?;
        }
    }
    if !painted {
        return Err(SurfaceError::Empty);
    }
    out.copy_from_slice(&pending);
    Ok(())
}

#[derive(Clone, Copy)]
struct PlanePoint {
    x: f64,
    y: f64,
}

fn require_index(len: usize, index: u32) -> Result<(), SurfaceError> {
    let slot = usize::try_from(index).map_err(|_| SurfaceError::Index)?;
    if slot >= len {
        return Err(SurfaceError::Index);
    }
    Ok(())
}

fn vertex_at(items: &[PlanePoint], index: u32) -> Result<PlanePoint, SurfaceError> {
    let slot = usize::try_from(index).map_err(|_| SurfaceError::Index)?;
    items.get(slot).copied().ok_or(SurfaceError::Index)
}

fn check_frame(frame: &PlaneFrame) -> Result<(), SurfaceError> {
    if !frame.origin.x.is_finite() || !frame.origin.y.is_finite() || !frame.origin.z.is_finite() {
        return Err(SurfaceError::Frame);
    }
    if !frame.spacing_x_mm.is_finite() || !frame.spacing_y_mm.is_finite() {
        return Err(SurfaceError::Frame);
    }
    if frame.spacing_x_mm <= 0.0 || frame.spacing_y_mm <= 0.0 {
        return Err(SurfaceError::Frame);
    }
    Ok(())
}

fn project(frame: &PlaneFrame, point: Pt<World>) -> Result<PlanePoint, SurfaceError> {
    if !point.x.is_finite() || !point.y.is_finite() || !point.z.is_finite() {
        return Err(SurfaceError::Frame);
    }
    if point.z.to_bits() != frame.origin.z.to_bits() {
        return Err(SurfaceError::OutOfPlane);
    }
    let x = (point.x - frame.origin.x) / frame.spacing_x_mm;
    let y = (point.y - frame.origin.y) / frame.spacing_y_mm;
    if !x.is_finite() || !y.is_finite() {
        return Err(SurfaceError::Frame);
    }
    Ok(PlanePoint { x, y })
}

fn cross_is_zero(value: f64) -> bool {
    value.to_bits() == 0 || value.to_bits() == (-0.0_f64).to_bits()
}

fn covered(
    a: PlanePoint,
    abx: f64,
    aby: f64,
    acx: f64,
    acy: f64,
    den: f64,
    p: PlanePoint,
) -> Result<bool, SurfaceError> {
    let apx = p.x - a.x;
    let apy = p.y - a.y;
    let weight_b = (apx * acy - acx * apy) / den;
    let weight_c = (abx * apy - apx * aby) / den;
    let weight_a = 1.0 - weight_b - weight_c;
    if !weight_a.is_finite() || !weight_b.is_finite() || !weight_c.is_finite() {
        return Err(SurfaceError::Frame);
    }
    Ok(weight_a >= 0.0 && weight_b >= 0.0 && weight_c >= 0.0)
}

fn paint(
    pending: &mut [u8],
    width: u32,
    column: u32,
    row: u32,
    colour: [u8; 4],
) -> Result<(), SurfaceError> {
    let offset = row
        .checked_mul(width)
        .ok_or(SurfaceError::Frame)?
        .checked_add(column)
        .ok_or(SurfaceError::Frame)?;
    let start = offset.checked_mul(4).ok_or(SurfaceError::Frame)?;
    let start_us = usize::try_from(start).map_err(|_| SurfaceError::Frame)?;
    let end = start_us.checked_add(4).ok_or(SurfaceError::Frame)?;
    let Some(slot) = pending.get_mut(start_us..end) else {
        return Err(SurfaceError::OutputLength);
    };
    slot.copy_from_slice(&colour);
    Ok(())
}
