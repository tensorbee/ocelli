//! F-064 contour stroke.
//!
//! Hand pixels use spacing 1 mm and origin 0. `f64::round` sends 0.5 to 1
//! and -0.5 to -1. The stroke keeps the LUT alpha. DICOM PS3.3 C.8.8.6
//! names the geometric type. PS3.3 C.8.20.2 keeps segment 0 out of the table.

use ocelli_core::{Pt, World};
use ocelli_seg::{
    Contour, ContourError, ContourGeometricType, LabelColourLut, LabelGrid, LabelStyle, PlaneFrame,
    Rgba8, present_contour, present_labelmap,
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

fn contour(kind: ContourGeometricType, points: Vec<Pt<World>>) -> Contour {
    Contour {
        segment: 1,
        kind,
        points,
    }
}

fn paint(contour: &Contour, width: u32, height: u32, out: &mut [u8]) -> Result<(), ContourError> {
    present_contour(contour, &frame(), &lut(), width, height, out)
}

fn pixel(buf: &[u8], index: usize) -> Option<[u8; 4]> {
    let start = index.checked_mul(4)?;
    let end = start.checked_add(4)?;
    let slot = buf.get(start..end)?;
    let mut out = [0_u8; 4];
    out.copy_from_slice(slot);
    Some(out)
}

#[test]
fn open_line_paints_the_stroke_only() {
    let line = contour(
        ContourGeometricType::OpenPlanar,
        vec![at(0.0, 0.0), at(3.0, 0.0)],
    );
    let mut out = vec![0xA5_u8; 32];
    assert!(paint(&line, 4, 2, &mut out).is_ok());
    assert_eq!(pixel(&out, 0), Some([10, 20, 30, 128]));
    assert_eq!(pixel(&out, 1), Some([10, 20, 30, 128]));
    assert_eq!(pixel(&out, 2), Some([10, 20, 30, 128]));
    assert_eq!(pixel(&out, 3), Some([10, 20, 30, 128]));
    assert_eq!(pixel(&out, 4), Some([0, 0, 0, 0]));
}

#[test]
fn open_nonplanar_uses_the_same_stroke_on_the_plane() {
    let line = contour(
        ContourGeometricType::OpenNonplanar,
        vec![at(0.0, 0.0), at(3.0, 0.0)],
    );
    let mut out = vec![0_u8; 32];
    assert!(paint(&line, 4, 2, &mut out).is_ok());
    assert_eq!(pixel(&out, 3), Some([10, 20, 30, 128]));
    assert_eq!(pixel(&out, 4), Some([0, 0, 0, 0]));
}

#[test]
fn closed_square_does_not_fill() {
    let square = contour(
        ContourGeometricType::ClosedPlanar,
        vec![at(0.0, 0.0), at(3.0, 0.0), at(3.0, 2.0), at(0.0, 2.0)],
    );
    let mut out = vec![0xA5_u8; 48];
    assert!(paint(&square, 4, 3, &mut out).is_ok());
    assert_eq!(pixel(&out, 0), Some([10, 20, 30, 128]));
    assert_eq!(pixel(&out, 3), Some([10, 20, 30, 128]));
    assert_eq!(pixel(&out, 5), Some([0, 0, 0, 0]));
    assert_eq!(pixel(&out, 11), Some([10, 20, 30, 128]));
    let grid = LabelGrid {
        width: 4,
        height: 3,
        indices: vec![1_u16; 12],
    };
    let mut label = vec![0_u8; 48];
    assert!(
        present_labelmap(
            &grid,
            &lut(),
            LabelStyle {
                fill: true,
                outline: true,
            },
            &mut label,
        )
        .is_ok()
    );
    assert_ne!(pixel(&out, 5), pixel(&label, 5));
}

#[test]
fn diagonal_bresenham_is_the_three_pixels() {
    let line = contour(
        ContourGeometricType::OpenPlanar,
        vec![at(0.0, 0.0), at(2.0, 2.0)],
    );
    let mut out = vec![0_u8; 36];
    assert!(paint(&line, 3, 3, &mut out).is_ok());
    assert_eq!(pixel(&out, 0), Some([10, 20, 30, 128]));
    assert_eq!(pixel(&out, 4), Some([10, 20, 30, 128]));
    assert_eq!(pixel(&out, 8), Some([10, 20, 30, 128]));
    assert_eq!(pixel(&out, 1), Some([0, 0, 0, 0]));
    assert_eq!(pixel(&out, 3), Some([0, 0, 0, 0]));
}

#[test]
fn half_rounds_away_from_zero() {
    let point = contour(ContourGeometricType::Point, vec![at(1.5, 0.5)]);
    let mut out = vec![0_u8; 24];
    assert!(paint(&point, 3, 2, &mut out).is_ok());
    assert_eq!(pixel(&out, 5), Some([10, 20, 30, 128]));
    assert_eq!(pixel(&out, 0), Some([0, 0, 0, 0]));
    assert_eq!(pixel(&out, 4), Some([0, 0, 0, 0]));
}

#[test]
fn a_float_just_below_one_half_stays_on_zero() {
    let below = f64::from_bits(0.5_f64.to_bits() - 1);
    let point = contour(ContourGeometricType::Point, vec![Pt::new(below, 0.0, 0.0)]);
    let mut out = vec![0_u8; 8];
    assert!(paint(&point, 2, 1, &mut out).is_ok());
    assert_eq!(pixel(&out, 0), Some([10, 20, 30, 128]));
    assert_eq!(pixel(&out, 1), Some([0, 0, 0, 0]));
    let above_neg = f64::from_bits((-0.5_f64).to_bits() - 1);
    let neg_point = contour(
        ContourGeometricType::Point,
        vec![Pt::new(above_neg, 0.0, 0.0)],
    );
    let mut neg = vec![0_u8; 8];
    assert!(paint(&neg_point, 2, 1, &mut neg).is_ok());
    assert_eq!(pixel(&neg, 0), Some([10, 20, 30, 128]));
    assert_eq!(pixel(&neg, 1), Some([0, 0, 0, 0]));
}

#[test]
fn a_contour_refusal_writes_nothing() {
    let mut sentinels = vec![0xA5_u8; 32];
    let mut raised = at(0.0, 0.0);
    raised.z = 1.0;
    let off = contour(ContourGeometricType::OpenPlanar, vec![at(0.0, 0.0), raised]);
    assert_eq!(
        paint(&off, 4, 2, &mut sentinels),
        Err(ContourError::OutOfPlane)
    );
    assert!(sentinels.iter().all(|byte| *byte == 0xA5));
    let short = contour(ContourGeometricType::Point, vec![]);
    assert_eq!(
        paint(&short, 4, 2, &mut sentinels),
        Err(ContourError::Length)
    );
    let one = contour(ContourGeometricType::OpenPlanar, vec![at(0.0, 0.0)]);
    assert_eq!(paint(&one, 4, 2, &mut sentinels), Err(ContourError::Length));
    let digon = contour(
        ContourGeometricType::ClosedPlanar,
        vec![at(0.0, 0.0), at(1.0, 0.0)],
    );
    assert_eq!(
        paint(&digon, 4, 2, &mut sentinels),
        Err(ContourError::Length)
    );
    let missing = Contour {
        segment: 2,
        kind: ContourGeometricType::Point,
        points: vec![at(0.0, 0.0)],
    };
    assert_eq!(
        paint(&missing, 4, 2, &mut sentinels),
        Err(ContourError::MissingColour)
    );
    let outside = contour(ContourGeometricType::Point, vec![Pt::new(-0.5, 0.0, 0.0)]);
    assert_eq!(
        paint(&outside, 4, 2, &mut sentinels),
        Err(ContourError::Empty)
    );
    let mut bad = frame();
    bad.spacing_x_mm = 0.0;
    let line = contour(
        ContourGeometricType::OpenPlanar,
        vec![at(0.0, 0.0), at(1.0, 0.0)],
    );
    assert_eq!(
        present_contour(&line, &bad, &lut(), 4, 2, &mut sentinels),
        Err(ContourError::Frame)
    );
    let mut short_out = vec![0xA5_u8; 4];
    assert_eq!(
        paint(&line, 4, 2, &mut short_out),
        Err(ContourError::OutputLength)
    );
    assert!(sentinels.iter().all(|byte| *byte == 0xA5));
    assert!(short_out.iter().all(|byte| *byte == 0xA5));
}
