//! F-064 contour stroke.
//!
//! Hand pixels use spacing 1 mm and origin 0. `f64::round` sends 0.5 to 1
//! and -0.5 to -1. The stroke keeps the LUT alpha. DICOM PS3.3 C.8.8.6
//! names the geometric type. PS3.3 C.8.20.2 keeps segment 0 out of the table.

use ocelli_core::{Pt, World};
use ocelli_seg::{
    Contour, ContourError, ContourGeometricType, LabelColourLut, LabelGrid, LabelStyle, PlaneFrame,
    Rgba8, intersect_contour, present_contour, present_labelmap,
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

fn at_z(x: f64, y: f64, z: f64) -> Pt<World> {
    Pt::new(x, y, z)
}

#[test]
fn a_cut_off_the_midpoint_uses_t() {
    // t = (1 - 0) / (4 - 0) = 0.25. x = 0.25 * 10. y = 0.25 * 4.
    // A constant midpoint would be (5, 2, 1).
    let rising = contour(
        ContourGeometricType::OpenPlanar,
        vec![at_z(0.0, 0.0, 0.0), at_z(10.0, 4.0, 4.0)],
    );
    assert_eq!(
        intersect_contour(&rising, 1.0),
        Ok(contour(
            ContourGeometricType::Point,
            vec![at_z(2.5, 1.0, 1.0)]
        ))
    );
    // t = (1 - 4) / (0 - 4) = 0.75. x = 0.75 * 8. y = 0.75 * 2.
    // A constant 0.25 would be (2, 0.5, 1).
    let falling = contour(
        ContourGeometricType::OpenPlanar,
        vec![at_z(0.0, 0.0, 4.0), at_z(8.0, 2.0, 0.0)],
    );
    assert_eq!(
        intersect_contour(&falling, 1.0),
        Ok(contour(
            ContourGeometricType::Point,
            vec![at_z(6.0, 1.5, 1.0)]
        ))
    );
}

#[test]
fn a_segment_crosses_the_plane_at_its_midpoint() {
    let line = contour(
        ContourGeometricType::OpenPlanar,
        vec![at_z(0.0, 0.0, 0.0), at_z(0.0, 0.0, 4.0)],
    );
    let cut = intersect_contour(&line, 2.0);
    assert_eq!(
        cut,
        Ok(contour(
            ContourGeometricType::Point,
            vec![at_z(0.0, 0.0, 2.0)]
        ))
    );
}

#[test]
fn an_open_polyline_crosses_the_plane_twice() {
    let line = contour(
        ContourGeometricType::OpenPlanar,
        vec![
            at_z(0.0, 0.0, 0.0),
            at_z(2.0, 0.0, 2.0),
            at_z(4.0, 0.0, 0.0),
        ],
    );
    assert_eq!(
        intersect_contour(&line, 1.0),
        Ok(contour(
            ContourGeometricType::OpenPlanar,
            vec![at_z(1.0, 0.0, 1.0), at_z(3.0, 0.0, 1.0)]
        ))
    );
}

#[test]
fn a_contour_already_on_the_plane_keeps_its_kind() {
    let closed = contour(
        ContourGeometricType::ClosedPlanar,
        vec![
            at_z(0.0, 0.0, 1.0),
            at_z(2.0, 0.0, 1.0),
            at_z(1.0, 2.0, 1.0),
        ],
    );
    assert_eq!(intersect_contour(&closed, 1.0), Ok(closed));
}

#[test]
fn negative_zero_is_not_the_positive_plane() {
    let point = contour(ContourGeometricType::Point, vec![at_z(0.0, 0.0, -0.0)]);
    assert_eq!(intersect_contour(&point, 0.0), Err(ContourError::Empty));
    let rising = contour(
        ContourGeometricType::OpenPlanar,
        vec![at_z(0.0, 0.0, -0.0), at_z(4.0, 0.0, 4.0)],
    );
    assert_eq!(intersect_contour(&rising, 0.0), Err(ContourError::Empty));
    let falling = contour(
        ContourGeometricType::OpenPlanar,
        vec![at_z(0.0, 0.0, -0.0), at_z(4.0, 0.0, -4.0)],
    );
    assert_eq!(intersect_contour(&falling, 0.0), Err(ContourError::Empty));
    let ring = contour(
        ContourGeometricType::ClosedPlanar,
        vec![
            at_z(0.0, 0.0, -0.0),
            at_z(4.0, 0.0, -4.0),
            at_z(0.0, 4.0, -4.0),
        ],
    );
    assert_eq!(intersect_contour(&ring, 0.0), Err(ContourError::Empty));
}

#[test]
fn a_coplanar_ring_with_one_negative_zero_vertex_is_refused() {
    // The square lies in the cut plane. The vertex (4, 4) is -0, so the
    // walk joins (4, 0) to (0, 4) across the interior.
    let closed = contour(
        ContourGeometricType::ClosedPlanar,
        vec![
            at_z(0.0, 0.0, 0.0),
            at_z(4.0, 0.0, 0.0),
            at_z(4.0, 4.0, -0.0),
            at_z(0.0, 4.0, 0.0),
        ],
    );
    assert_eq!(intersect_contour(&closed, 0.0), Err(ContourError::Length));
}

#[test]
fn a_coplanar_edge_with_one_negative_zero_vertex_stays() {
    // Only the bottom edge has both ends on the positive plane.
    let closed = contour(
        ContourGeometricType::ClosedPlanar,
        vec![
            at_z(0.0, 0.0, 0.0),
            at_z(4.0, 0.0, 0.0),
            at_z(0.0, 4.0, -0.0),
        ],
    );
    assert_eq!(
        intersect_contour(&closed, 0.0),
        Ok(contour(
            ContourGeometricType::OpenPlanar,
            vec![at_z(0.0, 0.0, 0.0), at_z(4.0, 0.0, 0.0)]
        ))
    );
}

#[test]
fn a_negative_zero_vertex_does_not_hide_a_gap() {
    // Two legs joined above the plane. The vertex at x = 4 has z = -0,
    // so it is not emitted. The span from x = 2 to x = 10 still crosses
    // the gap, and the midpoint at x = 6 is inside the far leg.
    let closed = contour(
        ContourGeometricType::ClosedPlanar,
        vec![
            at_z(0.0, 0.0, 4.0),
            at_z(0.0, 0.0, -2.0),
            at_z(2.0, 0.0, -2.0),
            at_z(2.0, 0.0, 0.0),
            at_z(2.0, 0.0, 2.0),
            at_z(4.0, 0.0, 2.0),
            at_z(4.0, 0.0, -0.0),
            at_z(4.0, 0.0, -2.0),
            at_z(10.0, 0.0, -2.0),
            at_z(10.0, 0.0, 4.0),
        ],
    );
    assert_eq!(intersect_contour(&closed, 0.0), Err(ContourError::Length));
}

#[test]
fn a_negative_zero_touch_on_a_chord_stays() {
    // The touch at x = 3 lies on the chord from x = 0 to x = 6.
    let closed = contour(
        ContourGeometricType::ClosedPlanar,
        vec![
            at_z(0.0, 0.0, -2.0),
            at_z(0.0, 0.0, 2.0),
            at_z(3.0, 0.0, -0.0),
            at_z(6.0, 0.0, 2.0),
            at_z(6.0, 0.0, -2.0),
        ],
    );
    assert_eq!(
        intersect_contour(&closed, 0.0),
        Ok(contour(
            ContourGeometricType::OpenPlanar,
            vec![at_z(0.0, 0.0, 0.0), at_z(6.0, 0.0, 0.0)]
        ))
    );
}

#[test]
fn a_zero_sign_on_y_does_not_refuse_the_chord() {
    // The notch touches y = 0. One touch uses +0 and the others use -0.
    // Those are the same place, and the chord stays one piece.
    let closed = contour(
        ContourGeometricType::ClosedPlanar,
        vec![
            at_z(0.0, 2.0, 2.0),
            at_z(0.0, -0.0, 1.0),
            at_z(0.0, -2.0, 0.0),
            at_z(4.0, -2.0, 0.0),
            at_z(4.0, 0.0, 1.0),
            at_z(4.0, 2.0, 2.0),
            at_z(3.0, 2.0, 2.0),
            at_z(2.0, -0.0, 1.0),
            at_z(1.0, 2.0, 2.0),
        ],
    );
    assert_eq!(
        intersect_contour(&closed, 1.0),
        Ok(contour(
            ContourGeometricType::OpenPlanar,
            vec![
                at_z(0.0, -0.0, 1.0),
                at_z(2.0, -0.0, 1.0),
                at_z(4.0, 0.0, 1.0)
            ]
        ))
    );
}

#[test]
fn a_closed_ring_emits_its_shared_vertex_once() {
    // Vertex 0 lies on the plane and closes the ring. The cut is the vertex
    // and the crossing of the opposite edge, not the vertex a second time.
    // t = (1 - 0) / (2 - 0) = 0.5, so the crossing is (2, 1, 1).
    let closed = contour(
        ContourGeometricType::ClosedPlanar,
        vec![
            at_z(0.0, 0.0, 1.0),
            at_z(2.0, 0.0, 0.0),
            at_z(2.0, 2.0, 2.0),
        ],
    );
    assert_eq!(
        intersect_contour(&closed, 1.0),
        Ok(contour(
            ContourGeometricType::OpenPlanar,
            vec![at_z(0.0, 0.0, 1.0), at_z(2.0, 1.0, 1.0)]
        ))
    );
}

#[test]
fn a_closed_contour_cut_stays_open() {
    let closed = contour(
        ContourGeometricType::ClosedPlanar,
        vec![
            at_z(0.0, 0.0, 0.0),
            at_z(2.0, 0.0, 0.0),
            at_z(1.0, 1.0, 2.0),
        ],
    );
    assert_eq!(
        intersect_contour(&closed, 1.0),
        Ok(contour(
            ContourGeometricType::OpenPlanar,
            vec![at_z(1.5, 0.5, 1.0), at_z(0.5, 0.5, 1.0)]
        ))
    );
}

#[test]
fn a_closed_ring_with_more_than_two_crossings_is_refused() {
    // The ring is a U on the plane z = y. The cut z = 1 meets four edges
    // at (3, 1), (2, 1), (1, 1) and (0, 1). The span from x = 2 to x = 1
    // lies in the notch. One contour cannot hold both arms.
    let closed = contour(
        ContourGeometricType::ClosedPlanar,
        vec![
            at_z(0.0, -2.0, -2.0),
            at_z(3.0, -2.0, -2.0),
            at_z(3.0, 2.0, 2.0),
            at_z(2.0, 2.0, 2.0),
            at_z(2.0, 0.0, 0.0),
            at_z(1.0, 0.0, 0.0),
            at_z(1.0, 2.0, 2.0),
            at_z(0.0, 2.0, 2.0),
        ],
    );
    assert_eq!(intersect_contour(&closed, 1.0), Err(ContourError::Length));
}

#[test]
fn a_closed_ring_touched_only_at_vertices_is_refused() {
    // The same U cut at z = 2 meets the four top vertices and no edge
    // interior. The span from (2, 2) to (1, 2) is the open mouth.
    let closed = contour(
        ContourGeometricType::ClosedPlanar,
        vec![
            at_z(0.0, -2.0, -2.0),
            at_z(3.0, -2.0, -2.0),
            at_z(3.0, 2.0, 2.0),
            at_z(2.0, 2.0, 2.0),
            at_z(2.0, 0.0, 0.0),
            at_z(1.0, 0.0, 0.0),
            at_z(1.0, 2.0, 2.0),
            at_z(0.0, 2.0, 2.0),
        ],
    );
    assert_eq!(intersect_contour(&closed, 2.0), Err(ContourError::Length));
}

#[test]
fn a_closed_cut_through_one_on_plane_vertex_stays_open() {
    // The vertex (2, 2, 2) lies on the chord from (4, 2, 2) to (0, 2, 2).
    // That is one piece, so the cut stays the chord.
    let closed = contour(
        ContourGeometricType::ClosedPlanar,
        vec![
            at_z(0.0, 0.0, 0.0),
            at_z(4.0, 0.0, 0.0),
            at_z(4.0, 4.0, 4.0),
            at_z(2.0, 2.0, 2.0),
            at_z(0.0, 4.0, 4.0),
        ],
    );
    assert_eq!(
        intersect_contour(&closed, 2.0),
        Ok(contour(
            ContourGeometricType::OpenPlanar,
            vec![
                at_z(4.0, 2.0, 2.0),
                at_z(2.0, 2.0, 2.0),
                at_z(0.0, 2.0, 2.0)
            ]
        ))
    );
}

#[test]
fn a_closed_cut_started_on_the_chord_stays_in_line_order() {
    // The same pentagon, started at the on-plane vertex. The chord runs
    // from x = 0 through x = 2 to x = 4. Walking the ring first would
    // emit the middle, then one end, then the other.
    let closed = contour(
        ContourGeometricType::ClosedPlanar,
        vec![
            at_z(2.0, 2.0, 2.0),
            at_z(0.0, 4.0, 4.0),
            at_z(0.0, 0.0, 0.0),
            at_z(4.0, 0.0, 0.0),
            at_z(4.0, 4.0, 4.0),
        ],
    );
    assert_eq!(
        intersect_contour(&closed, 2.0),
        Ok(contour(
            ContourGeometricType::OpenPlanar,
            vec![
                at_z(0.0, 2.0, 2.0),
                at_z(2.0, 2.0, 2.0),
                at_z(4.0, 2.0, 2.0)
            ]
        ))
    );
}

#[test]
fn a_notch_that_touches_the_chord_stays_in_line_order() {
    // The tip (2, 2, 2) lies on the chord from x = 0 to x = 6.
    let closed = contour(
        ContourGeometricType::ClosedPlanar,
        vec![
            at_z(0.0, 0.0, 0.0),
            at_z(1.0, 0.0, 0.0),
            at_z(2.0, 2.0, 2.0),
            at_z(3.0, 0.0, 0.0),
            at_z(6.0, 0.0, 0.0),
            at_z(6.0, 4.0, 4.0),
            at_z(0.0, 4.0, 4.0),
        ],
    );
    assert_eq!(
        intersect_contour(&closed, 2.0),
        Ok(contour(
            ContourGeometricType::OpenPlanar,
            vec![
                at_z(6.0, 2.0, 2.0),
                at_z(2.0, 2.0, 2.0),
                at_z(0.0, 2.0, 2.0)
            ]
        ))
    );
}

#[test]
fn a_non_finite_plane_is_refused() {
    let line = contour(
        ContourGeometricType::OpenPlanar,
        vec![at_z(0.0, 0.0, 0.0), at_z(0.0, 0.0, 4.0)],
    );
    assert_eq!(intersect_contour(&line, f64::NAN), Err(ContourError::Frame));
    let broken = contour(
        ContourGeometricType::OpenPlanar,
        vec![at_z(0.0, 0.0, f64::NAN), at_z(0.0, 0.0, 4.0)],
    );
    assert_eq!(intersect_contour(&broken, 1.0), Err(ContourError::Frame));
}

#[test]
fn a_crossing_that_rounds_onto_the_endpoint_is_kept() {
    // t = (1e-30 + 1) / (1e-20 + 1) rounds to 1. The plane still separates
    // the first edge, so the cut keeps that end and the point already on it.
    let line = contour(
        ContourGeometricType::OpenPlanar,
        vec![
            at_z(0.0, 0.0, -1.0),
            at_z(4.0, 0.0, 1.0e-20),
            at_z(8.0, 0.0, 1.0e-30),
        ],
    );
    assert_eq!(
        intersect_contour(&line, 1.0e-30),
        Ok(contour(
            ContourGeometricType::OpenPlanar,
            vec![at_z(4.0, 0.0, 1.0e-30), at_z(8.0, 0.0, 1.0e-30)]
        ))
    );
}

#[test]
fn a_non_finite_denominator_is_refused() {
    let line = contour(
        ContourGeometricType::OpenPlanar,
        vec![at_z(0.0, 0.0, -1.0e308), at_z(4.0, 0.0, 1.0e308)],
    );
    assert_eq!(intersect_contour(&line, 0.0), Err(ContourError::Frame));
}

#[test]
fn a_parallel_segment_off_the_plane_is_empty() {
    let line = contour(
        ContourGeometricType::OpenPlanar,
        vec![at_z(0.0, 0.0, 0.0), at_z(4.0, 0.0, 0.0)],
    );
    assert_eq!(intersect_contour(&line, 1.0), Err(ContourError::Empty));
}

#[test]
fn a_shared_vertex_on_the_plane_is_emitted_once() {
    let line = contour(
        ContourGeometricType::OpenPlanar,
        vec![
            at_z(0.0, 0.0, 0.0),
            at_z(0.0, 0.0, 2.0),
            at_z(0.0, 0.0, 0.0),
        ],
    );
    assert_eq!(
        intersect_contour(&line, 2.0),
        Ok(contour(
            ContourGeometricType::Point,
            vec![at_z(0.0, 0.0, 2.0)]
        ))
    );
}

#[test]
fn an_open_nonplanar_cut_is_open_planar() {
    let line = contour(
        ContourGeometricType::OpenNonplanar,
        vec![
            at_z(0.0, 0.0, 0.0),
            at_z(2.0, 0.0, 2.0),
            at_z(4.0, 1.0, 0.0),
        ],
    );
    assert_eq!(
        intersect_contour(&line, 1.0),
        Ok(contour(
            ContourGeometricType::OpenPlanar,
            vec![at_z(1.0, 0.0, 1.0), at_z(3.0, 0.5, 1.0)]
        ))
    );
    let on_plane = contour(
        ContourGeometricType::OpenNonplanar,
        vec![at_z(0.0, 0.0, 1.0), at_z(2.0, 0.0, 1.0)],
    );
    assert_eq!(intersect_contour(&on_plane, 1.0), Ok(on_plane));
}

#[test]
fn a_contour_with_the_wrong_point_count_is_refused() {
    let digon = contour(
        ContourGeometricType::ClosedPlanar,
        vec![at_z(0.0, 0.0, 0.0), at_z(1.0, 0.0, 2.0)],
    );
    assert_eq!(intersect_contour(&digon, 1.0), Err(ContourError::Length));
}
