//! F-099 RTSTRUCT import.
//!
//! Hand-stated from `.claude/plans/F-099-design.md`. PS3.3 C.8.8.6 names the
//! geometric type. The half-spacing slab is the local tolerance. Points stay
//! in patient millimetres.

use ocelli_core::{Pt, World};
use ocelli_seg::{Contour, ContourError, ContourGeometricType, import_rtstruct_contour};

fn at(x: f64, y: f64, z: f64) -> Pt<World> {
    Pt::new(x, y, z)
}

fn triangle(z: f64) -> Vec<Pt<World>> {
    vec![at(0.0, 0.0, z), at(1.0, 0.0, z), at(0.0, 1.0, z)]
}

fn must_import(points: &[Pt<World>], slice_z: f64, spacing: f64) -> Contour {
    match import_rtstruct_contour(
        1,
        ContourGeometricType::ClosedPlanar,
        points,
        slice_z,
        spacing,
    ) {
        Ok(contour) => contour,
        Err(_) => unreachable!("the plan's in-slab contour"),
    }
}

#[test]
fn a_closed_triangle_on_the_slice_is_copied() {
    let points = triangle(10.0);
    let before = points.clone();
    let contour = must_import(&points, 10.0, 2.0);
    assert_eq!(contour.segment, 1);
    assert_eq!(contour.kind, ContourGeometricType::ClosedPlanar);
    assert_eq!(contour.points, points);
    assert_eq!(points, before);
}

#[test]
fn half_spacing_keeps_10_9_and_refuses_11_1() {
    let inside_points = triangle(10.9);
    let inside = must_import(&inside_points, 10.0, 2.0);
    assert_eq!(inside.points, inside_points);
    let edge_points = triangle(11.0);
    let edge = must_import(&edge_points, 10.0, 2.0);
    assert_eq!(edge.points, edge_points);
    assert_eq!(
        import_rtstruct_contour(
            1,
            ContourGeometricType::ClosedPlanar,
            &triangle(11.1),
            10.0,
            2.0,
        ),
        Err(ContourError::OutOfPlane)
    );
}

#[test]
fn spacing_zero_and_a_non_finite_point_are_refused() {
    assert_eq!(
        import_rtstruct_contour(
            1,
            ContourGeometricType::ClosedPlanar,
            &triangle(10.0),
            10.0,
            0.0,
        ),
        Err(ContourError::OutOfPlane)
    );
    let points = vec![
        at(f64::NAN, 0.0, 10.0),
        at(1.0, 0.0, 10.0),
        at(0.0, 1.0, 10.0),
    ];
    assert_eq!(
        import_rtstruct_contour(1, ContourGeometricType::ClosedPlanar, &points, 10.0, 2.0),
        Err(ContourError::Frame)
    );
}

#[test]
fn a_short_contour_and_segment_zero_are_refused() {
    let one = vec![at(0.0, 0.0, 10.0)];
    assert_eq!(
        import_rtstruct_contour(1, ContourGeometricType::ClosedPlanar, &one, 10.0, 2.0),
        Err(ContourError::Length)
    );
    assert_eq!(
        import_rtstruct_contour(
            0,
            ContourGeometricType::ClosedPlanar,
            &triangle(10.0),
            10.0,
            2.0,
        ),
        Err(ContourError::MissingColour)
    );
}

#[test]
fn the_stored_z_is_the_caller_z() {
    let points = triangle(10.9);
    let before = points.clone();
    let contour = must_import(&points, 10.0, 2.0);
    assert_eq!(contour.points, points);
    assert_ne!(contour.points, triangle(10.0));
    assert_eq!(points, before);
}
