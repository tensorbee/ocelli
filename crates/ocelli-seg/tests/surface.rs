//! F-065 surface coverage.
//!
//! Hand pixels use spacing 1 mm and origin 0. A centre is covered when every
//! barycentric weight is at least zero. DICOM PS3.3 C.8.20.2 keeps segment 0
//! out of the table. The geometric type names used by the contour comparison
//! are PS3.3 C.8.8.6.

use ocelli_core::{Pt, World};
use ocelli_seg::{
    Contour, ContourGeometricType, LabelColourLut, PlaneFrame, Rgba8, SurfaceError, SurfaceMesh,
    present_contour, present_surface,
};

fn colour() -> Rgba8 {
    Rgba8 {
        r: 10,
        g: 20,
        b: 30,
        a: 128,
    }
}

fn lut() -> LabelColourLut {
    LabelColourLut::from_segment_one(vec![colour()])
}

fn frame() -> PlaneFrame {
    PlaneFrame {
        origin: Pt::new(0.0, 0.0, 0.0),
        spacing_x_mm: 1.0,
        spacing_y_mm: 1.0,
    }
}

fn at(x: f64, y: f64) -> Pt<World> {
    Pt::new(x, y, 0.0)
}

fn hand() -> SurfaceMesh {
    SurfaceMesh {
        segment: 1,
        vertices: vec![at(0.0, 0.0), at(4.0, 0.0), at(0.0, 4.0)],
        triangles: vec![[0, 1, 2]],
    }
}

fn pixel(buf: &[u8], index: usize) -> Option<[u8; 4]> {
    let start = index.checked_mul(4)?;
    let end = start.checked_add(4)?;
    let slot = buf.get(start..end)?;
    let mut out = [0_u8; 4];
    out.copy_from_slice(slot);
    Some(out)
}

fn index(column: u32, row: u32) -> usize {
    let row_us = usize::try_from(row).unwrap_or(0);
    let column_us = usize::try_from(column).unwrap_or(0);
    row_us.saturating_mul(5).saturating_add(column_us)
}

#[test]
fn triangle_covers_centres_inside_the_edge() {
    let mut out = vec![0xA5_u8; 100];
    assert!(present_surface(&hand(), &frame(), &lut(), 5, 5, &mut out).is_ok());
    let ink = [10_u8, 20, 30, 128];
    let covered = [
        (0, 0),
        (1, 0),
        (2, 0),
        (3, 0),
        (0, 1),
        (1, 1),
        (2, 1),
        (0, 2),
        (1, 2),
        (0, 3),
    ];
    for (column, row) in covered {
        assert_eq!(pixel(&out, index(column, row)), Some(ink));
    }
    for (column, row) in [(4, 0), (0, 4), (3, 1), (2, 2), (4, 4)] {
        assert_eq!(pixel(&out, index(column, row)), Some([0, 0, 0, 0]));
    }
}

#[test]
fn clockwise_triangle_uses_the_same_centres() {
    let mesh = SurfaceMesh {
        segment: 1,
        vertices: vec![at(0.0, 0.0), at(0.0, 4.0), at(4.0, 0.0)],
        triangles: vec![[0, 1, 2]],
    };
    let mut out = vec![0_u8; 100];
    assert!(present_surface(&mesh, &frame(), &lut(), 5, 5, &mut out).is_ok());
    assert_eq!(pixel(&out, index(1, 1)), Some([10, 20, 30, 128]));
    assert_eq!(pixel(&out, index(4, 0)), Some([0, 0, 0, 0]));
    assert_eq!(pixel(&out, index(0, 3)), Some([10, 20, 30, 128]));
}

#[test]
fn contour_and_surface_disagree_on_the_hand_triangle() {
    let mut surface = vec![0_u8; 100];
    assert!(present_surface(&hand(), &frame(), &lut(), 5, 5, &mut surface).is_ok());
    let contour = Contour {
        segment: 1,
        kind: ContourGeometricType::ClosedPlanar,
        points: vec![at(0.0, 0.0), at(4.0, 0.0), at(0.0, 4.0)],
    };
    let mut stroke = vec![0_u8; 100];
    assert!(present_contour(&contour, &frame(), &lut(), 5, 5, &mut stroke).is_ok());
    assert_eq!(pixel(&surface, index(1, 1)), Some([10, 20, 30, 128]));
    assert_eq!(pixel(&stroke, index(1, 1)), Some([0, 0, 0, 0]));
    assert_eq!(pixel(&stroke, index(4, 0)), Some([10, 20, 30, 128]));
    assert_eq!(pixel(&surface, index(4, 0)), Some([0, 0, 0, 0]));
    assert_eq!(pixel(&stroke, index(0, 4)), Some([10, 20, 30, 128]));
    assert_eq!(pixel(&surface, index(0, 4)), Some([0, 0, 0, 0]));
}

#[test]
fn a_degenerate_triangle_is_skipped() {
    let flat = SurfaceMesh {
        segment: 1,
        vertices: vec![at(0.0, 0.0), at(1.0, 0.0), at(2.0, 0.0)],
        triangles: vec![[0, 1, 2]],
    };
    let mut sentinels = vec![0xA5_u8; 100];
    assert_eq!(
        present_surface(&flat, &frame(), &lut(), 5, 5, &mut sentinels),
        Err(SurfaceError::Empty)
    );
    assert!(sentinels.iter().all(|byte| *byte == 0xA5));
    let mixed = SurfaceMesh {
        segment: 1,
        vertices: vec![
            at(0.0, 0.0),
            at(1.0, 0.0),
            at(2.0, 0.0),
            at(4.0, 0.0),
            at(0.0, 4.0),
        ],
        triangles: vec![[0, 1, 2], [0, 3, 4]],
    };
    let mut out = vec![0_u8; 100];
    assert!(present_surface(&mixed, &frame(), &lut(), 5, 5, &mut out).is_ok());
    assert_eq!(pixel(&out, index(1, 1)), Some([10, 20, 30, 128]));
}

#[test]
fn a_surface_refusal_writes_nothing() {
    let mut sentinels = vec![0xA5_u8; 100];
    let mut bad_index = hand();
    bad_index.triangles = vec![[0, 1, 9]];
    assert_eq!(
        present_surface(&bad_index, &frame(), &lut(), 5, 5, &mut sentinels),
        Err(SurfaceError::Index)
    );
    let mut off_plane = at(0.0, 4.0);
    off_plane.z = 1.0;
    let raised = SurfaceMesh {
        segment: 1,
        vertices: vec![at(0.0, 0.0), at(4.0, 0.0), off_plane],
        triangles: vec![[0, 1, 2]],
    };
    assert_eq!(
        present_surface(&raised, &frame(), &lut(), 5, 5, &mut sentinels),
        Err(SurfaceError::OutOfPlane)
    );
    let mut missing = hand();
    missing.segment = 2;
    assert_eq!(
        present_surface(&missing, &frame(), &lut(), 5, 5, &mut sentinels),
        Err(SurfaceError::MissingColour)
    );
    let mut short = vec![0xA5_u8; 4];
    assert_eq!(
        present_surface(&hand(), &frame(), &lut(), 5, 5, &mut short),
        Err(SurfaceError::OutputLength)
    );
    let mut bad = frame();
    bad.spacing_x_mm = 0.0;
    assert_eq!(
        present_surface(&hand(), &bad, &lut(), 5, 5, &mut sentinels),
        Err(SurfaceError::Frame)
    );
    assert!(sentinels.iter().all(|byte| *byte == 0xA5));
    assert!(short.iter().all(|byte| *byte == 0xA5));
}
