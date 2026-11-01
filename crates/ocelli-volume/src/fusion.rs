//! A shared patient plane for two volumes.
//!
//! Pixel `(column, row)` is PS3.3 C.7.6.2.1.1 in the plane's own axes.
//! `column_mm` multiplies the row direction. `row_mm` multiplies the column
//! direction. The directions are stored as given. This module does not blend
//! and does not know a colour.

use glam::DVec3;
use ocelli_core::{Pt, World};

use crate::{OrthoPlane, Volume};

/// One plane both volumes are sampled on.
///
/// Output pixel `(i, j)` is `origin + i * column_mm * row_dir + j * row_mm *
/// column_dir`. `i` is the column index and `j` is the row index.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FusionPlane {
    /// Centre of output pixel `(0, 0)`, in patient millimetres.
    pub origin: Pt<World>,
    /// Row direction. Multiplied by `column_mm`.
    pub row_dir: (f64, f64, f64),
    /// Column direction. Multiplied by `row_mm`.
    pub column_dir: (f64, f64, f64),
    /// Millimetres between output columns.
    pub column_mm: f64,
    /// Millimetres between output rows.
    pub row_mm: f64,
    /// Output columns.
    pub width: u32,
    /// Output rows.
    pub height: u32,
}

/// Build the plane an orthographic view already uses.
///
/// Axial steps on the row cosine and the column cosine. Sagittal holds the
/// column index and steps on the column cosine and the normal. Coronal holds
/// the row index and steps on the row cosine and the normal. An index outside
/// the volume returns [`None`] and allocates nothing.
pub fn plane_from_ortho(volume: &Volume, plane: OrthoPlane) -> Option<FusionPlane> {
    let (width, height, column_mm, row_mm, row_dir, column_dir, origin_index) = match plane {
        OrthoPlane::Axial { slice } => {
            volume.in_range(slice, volume.dims[2], 2).ok()?;
            (
                volume.dims[0],
                volume.dims[1],
                volume.spacing[0],
                volume.spacing[1],
                volume.direction.col(0),
                volume.direction.col(1),
                [0.0, 0.0, slice],
            )
        }
        OrthoPlane::Sagittal { column } => {
            volume.in_range(column, volume.dims[0], 0).ok()?;
            (
                volume.dims[1],
                volume.dims[2],
                volume.spacing[1],
                volume.spacing[2],
                volume.direction.col(1),
                volume.direction.col(2),
                [column, 0.0, 0.0],
            )
        }
        OrthoPlane::Coronal { row } => {
            volume.in_range(row, volume.dims[1], 1).ok()?;
            (
                volume.dims[0],
                volume.dims[2],
                volume.spacing[0],
                volume.spacing[2],
                volume.direction.col(0),
                volume.direction.col(2),
                [0.0, row, 0.0],
            )
        }
    };
    if width == 0 || height == 0 {
        return None;
    }
    let origin = volume.voxel_world(origin_index);
    Some(FusionPlane {
        origin,
        row_dir: (row_dir.x, row_dir.y, row_dir.z),
        column_dir: (column_dir.x, column_dir.y, column_dir.z),
        column_mm,
        row_mm,
        width,
        height,
    })
}

/// Patient point of one output pixel.
///
/// The directions are not renormalised here. A presenter that requires unit
/// directions refuses them before this point is sampled.
pub fn world_on_plane(plane: &FusionPlane, column: u32, row: u32) -> Pt<World> {
    let origin = DVec3::new(plane.origin.x, plane.origin.y, plane.origin.z);
    let row_dir = DVec3::new(plane.row_dir.0, plane.row_dir.1, plane.row_dir.2);
    let column_dir = DVec3::new(plane.column_dir.0, plane.column_dir.1, plane.column_dir.2);
    let along_column = f64_from_u32(column) * plane.column_mm;
    let along_row = f64_from_u32(row) * plane.row_mm;
    let point = origin + row_dir * along_column + column_dir * along_row;
    Pt::<World>::new(point.x, point.y, point.z)
}

fn f64_from_u32(value: u32) -> f64 {
    let high = u16::try_from(value >> 16).unwrap_or(0);
    let low = u16::try_from(value & 0xffff).unwrap_or(0);
    f64::from(high) * 65_536.0 + f64::from(low)
}
