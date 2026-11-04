//! F-080 brush fixtures. The expected indices are hand-computed in
//! `.claude/plans/F-080-design.md`. PS3.3 C.8.20.2 keeps segment 0 as
//! background. The radius test is the plan, not an HLD formula.

use ocelli_seg::{
    Brush, BrushCoverage, BrushError, BrushShape, LabelColourLut, LabelGrid, LabelStyle, Rgba8,
    present_labelmap, stamp_brush,
};

fn grid(width: u32, height: u32, indices: Vec<u16>) -> LabelGrid {
    LabelGrid {
        width,
        height,
        indices,
    }
}

fn brush(shape: BrushShape, x: f64, y: f64, z: f64, radius: f64, segment: u16) -> Brush {
    Brush {
        shape,
        center_x: x,
        center_y: y,
        center_z: z,
        radius,
        segment,
    }
}

fn zeros(count: usize) -> Vec<u16> {
    vec![0; count]
}

fn must_stamp(grids: &mut [LabelGrid], brush: Brush) -> BrushCoverage {
    match stamp_brush(grids, brush) {
        Ok(coverage) => coverage,
        Err(_) => unreachable!("the plan's finite stamp"),
    }
}

#[test]
fn a_circle_of_radius_one_paints_the_plus_and_not_the_corners() {
    let mut grids = vec![grid(3, 3, zeros(9))];
    let coverage = must_stamp(&mut grids, brush(BrushShape::Circle, 1.0, 1.0, 0.0, 1.0, 2));
    assert_eq!(coverage.covered, 5);
    assert_eq!(coverage.changed, 5);
    assert_eq!(
        grids.first().map(|item| item.indices.as_slice()),
        Some([0, 2, 0, 2, 2, 2, 0, 2, 0].as_slice())
    );
}

#[test]
fn a_sphere_reaches_the_next_slice_only_at_the_centre() {
    let mut grids = vec![grid(3, 3, zeros(9)), grid(3, 3, zeros(9))];
    let coverage = must_stamp(&mut grids, brush(BrushShape::Sphere, 1.0, 1.0, 0.0, 1.0, 1));
    assert_eq!(coverage.covered, 6);
    assert_eq!(coverage.changed, 6);
    assert_eq!(
        grids.first().map(|item| item.indices.as_slice()),
        Some([0, 1, 0, 1, 1, 1, 0, 1, 0].as_slice())
    );
    assert_eq!(
        grids.get(1).map(|item| item.indices.as_slice()),
        Some([0, 0, 0, 0, 1, 0, 0, 0, 0].as_slice())
    );
}

#[test]
fn a_radius_of_zero_does_not_paint_a_centre_that_is_not_an_integer() {
    // Sample (0, 0) is 1e-170 from the centre. Radius 0 includes only an
    // integer centre, so every stored 9 stays and the call still succeeds.
    let mut circle = vec![grid(3, 3, vec![9; 9])];
    let coverage = must_stamp(
        &mut circle,
        brush(BrushShape::Circle, 1.0e-170, 0.0, 0.0, 0.0, 3),
    );
    assert_eq!(coverage.covered, 0);
    assert_eq!(
        circle.first().map(|item| item.indices.as_slice()),
        Some([9, 9, 9, 9, 9, 9, 9, 9, 9].as_slice())
    );
    let mut sphere = vec![grid(3, 3, vec![9; 9])];
    let coverage = must_stamp(
        &mut sphere,
        brush(BrushShape::Sphere, 0.0, 0.0, 1.0e-170, 0.0, 3),
    );
    assert_eq!(coverage.covered, 0);
    assert_eq!(
        sphere.first().map(|item| item.indices.as_slice()),
        Some([9, 9, 9, 9, 9, 9, 9, 9, 9].as_slice())
    );
}

#[test]
fn a_non_zero_radius_whose_square_is_not_normal_writes_nothing() {
    // Sample (0, 0) is 1e-170 from the centre. The radius is 1e-200.
    // 1e-170 is not on or inside that radius, so every stored 9 stays.
    let mut circle = vec![grid(3, 3, vec![9; 9])];
    assert!(matches!(
        stamp_brush(
            &mut circle,
            brush(BrushShape::Circle, 1.0e-170, 0.0, 0.0, 1.0e-200, 3),
        ),
        Err(BrushError::Frame)
    ));
    assert_eq!(
        circle.first().map(|item| item.indices.as_slice()),
        Some([9, 9, 9, 9, 9, 9, 9, 9, 9].as_slice())
    );
    let mut sphere = vec![grid(3, 3, vec![9; 9])];
    assert!(matches!(
        stamp_brush(
            &mut sphere,
            brush(BrushShape::Sphere, 0.0, 0.0, 1.0e-170, 1.0e-200, 3),
        ),
        Err(BrushError::Frame)
    ));
    assert_eq!(
        sphere.first().map(|item| item.indices.as_slice()),
        Some([9, 9, 9, 9, 9, 9, 9, 9, 9].as_slice())
    );
}

#[test]
fn a_squared_radius_past_the_finite_range_writes_nothing() {
    // Sample (0, 0) is 1e200 from the centre. The radius is 1e160.
    // 1e200 is not on or inside that radius, so every stored 9 stays.
    let mut circle = vec![grid(3, 3, vec![9; 9])];
    assert!(matches!(
        stamp_brush(
            &mut circle,
            brush(BrushShape::Circle, 1.0e200, 0.0, 0.0, 1.0e160, 3),
        ),
        Err(BrushError::Frame)
    ));
    assert_eq!(
        circle.first().map(|item| item.indices.as_slice()),
        Some([9, 9, 9, 9, 9, 9, 9, 9, 9].as_slice())
    );
    let mut sphere = vec![grid(3, 3, vec![9; 9])];
    assert!(matches!(
        stamp_brush(
            &mut sphere,
            brush(BrushShape::Sphere, 1.0e200, 0.0, 0.0, 1.0e160, 3),
        ),
        Err(BrushError::Frame)
    ));
    assert_eq!(
        sphere.first().map(|item| item.indices.as_slice()),
        Some([9, 9, 9, 9, 9, 9, 9, 9, 9].as_slice())
    );
}

#[test]
fn a_negative_radius_and_a_non_finite_radius_write_nothing() {
    let mut negative = vec![grid(3, 3, vec![9; 9])];
    assert!(matches!(
        stamp_brush(
            &mut negative,
            brush(BrushShape::Circle, 1.0, 1.0, 0.0, -1.0, 1),
        ),
        Err(BrushError::Degenerate)
    ));
    assert_eq!(
        negative.first().map(|item| item.indices.as_slice()),
        Some([9, 9, 9, 9, 9, 9, 9, 9, 9].as_slice())
    );
    let mut non_finite = vec![grid(3, 3, vec![9; 9])];
    assert!(matches!(
        stamp_brush(
            &mut non_finite,
            brush(BrushShape::Sphere, 1.0, 1.0, 0.0, f64::NAN, 1),
        ),
        Err(BrushError::Frame)
    ));
    assert_eq!(
        non_finite.first().map(|item| item.indices.as_slice()),
        Some([9, 9, 9, 9, 9, 9, 9, 9, 9].as_slice())
    );
}

#[test]
fn a_short_buffer_is_not_stamped() {
    let mut grids = vec![grid(3, 3, vec![9; 8])];
    assert!(matches!(
        stamp_brush(&mut grids, brush(BrushShape::Circle, 1.0, 1.0, 0.0, 1.0, 1),),
        Err(BrushError::Degenerate)
    ));
    assert_eq!(grids.first().map(|item| item.indices.len()), Some(8));
    assert_eq!(
        grids.first().map(|item| item.indices.as_slice()),
        Some([9, 9, 9, 9, 9, 9, 9, 9].as_slice())
    );
}

#[test]
fn radius_zero_paints_only_an_integer_centre() {
    let mut grids = vec![grid(3, 3, zeros(9))];
    let coverage = must_stamp(&mut grids, brush(BrushShape::Circle, 1.0, 1.0, 0.0, 0.0, 4));
    assert_eq!(coverage.covered, 1);
    assert_eq!(coverage.changed, 1);
    assert_eq!(
        grids.first().map(|item| item.indices.as_slice()),
        Some([0, 0, 0, 0, 4, 0, 0, 0, 0].as_slice())
    );
    let mut missed = vec![grid(3, 3, zeros(9))];
    let coverage = must_stamp(
        &mut missed,
        brush(BrushShape::Circle, 1.5, 1.0, 0.0, 0.0, 4),
    );
    assert_eq!(coverage.covered, 0);
    assert_eq!(coverage.changed, 0);
    assert_eq!(
        missed.first().map(|item| item.indices.as_slice()),
        Some([0, 0, 0, 0, 0, 0, 0, 0, 0].as_slice())
    );
}

#[test]
fn a_circle_paints_only_the_matching_slice() {
    let mut grids = vec![grid(3, 3, zeros(9)), grid(3, 3, zeros(9))];
    let coverage = must_stamp(&mut grids, brush(BrushShape::Circle, 1.0, 1.0, 1.0, 1.0, 3));
    assert_eq!(coverage.covered, 5);
    assert_eq!(
        grids.first().map(|item| item.indices.as_slice()),
        Some([0, 0, 0, 0, 0, 0, 0, 0, 0].as_slice())
    );
    assert_eq!(
        grids.get(1).map(|item| item.indices.as_slice()),
        Some([0, 3, 0, 3, 3, 3, 0, 3, 0].as_slice())
    );
    let mut negative_zero = vec![grid(3, 3, zeros(9))];
    let coverage = must_stamp(
        &mut negative_zero,
        brush(BrushShape::Circle, 1.0, 1.0, -0.0, 1.0, 3),
    );
    assert_eq!(coverage.covered, 5);
    assert_eq!(
        negative_zero.first().map(|item| item.indices.as_slice()),
        Some([0, 3, 0, 3, 3, 3, 0, 3, 0].as_slice())
    );
}

#[test]
fn erasing_the_centre_then_stamping_it_again_changes_once() {
    let mut grids = vec![grid(3, 3, vec![1; 9])];
    let coverage = must_stamp(&mut grids, brush(BrushShape::Sphere, 1.0, 1.0, 0.0, 0.0, 0));
    assert_eq!(coverage.covered, 1);
    assert_eq!(coverage.changed, 1);
    assert_eq!(
        grids.first().and_then(|item| item.indices.get(4).copied()),
        Some(0)
    );
    let coverage = must_stamp(&mut grids, brush(BrushShape::Sphere, 1.0, 1.0, 0.0, 0.0, 0));
    assert_eq!(coverage.covered, 1);
    assert_eq!(coverage.changed, 0);
}

#[test]
fn mismatched_slices_are_not_stamped() {
    let mut grids = vec![grid(3, 3, vec![4; 9]), grid(2, 2, vec![5; 4])];
    assert!(matches!(
        stamp_brush(&mut grids, brush(BrushShape::Sphere, 0.0, 0.0, 0.0, 2.0, 1),),
        Err(BrushError::Degenerate)
    ));
    assert_eq!(
        grids.first().map(|item| item.indices.as_slice()),
        Some([4, 4, 4, 4, 4, 4, 4, 4, 4].as_slice())
    );
    assert_eq!(
        grids.get(1).map(|item| item.indices.as_slice()),
        Some([5, 5, 5, 5].as_slice())
    );
}

#[test]
fn the_presenter_colours_the_stamped_segment() {
    let mut grids = vec![grid(3, 3, zeros(9))];
    must_stamp(&mut grids, brush(BrushShape::Circle, 1.0, 1.0, 0.0, 1.0, 1));
    let lut = LabelColourLut::from_segment_one(vec![Rgba8 {
        r: 10,
        g: 20,
        b: 30,
        a: 40,
    }]);
    let mut out = vec![7_u8; 36];
    let Some(slice) = grids.first() else {
        unreachable!("one slice");
    };
    match present_labelmap(
        slice,
        &lut,
        LabelStyle {
            fill: true,
            outline: false,
        },
        &mut out,
    ) {
        Ok(()) => {}
        Err(_) => unreachable!("the existing presenter draws the grid"),
    }
    assert_eq!(out.get(16..20), Some([10, 20, 30, 40].as_slice()));
    assert_eq!(out.get(0..4), Some([0, 0, 0, 0].as_slice()));
}

#[test]
fn a_negative_zero_radius_erases_only_the_matching_slice() {
    let mut grids = vec![grid(3, 3, vec![1; 9]), grid(3, 3, vec![1; 9])];
    let coverage = must_stamp(
        &mut grids,
        brush(BrushShape::Circle, 1.0, 1.0, -0.0, -0.0, 0),
    );
    assert_eq!(coverage.covered, 1);
    assert_eq!(coverage.changed, 1);
    assert_eq!(
        grids.first().map(|item| item.indices.as_slice()),
        Some([1, 1, 1, 1, 0, 1, 1, 1, 1].as_slice())
    );
    assert_eq!(
        grids.get(1).map(|item| item.indices.as_slice()),
        Some([1, 1, 1, 1, 1, 1, 1, 1, 1].as_slice())
    );
}

#[test]
fn a_fractional_circle_slice_paints_nothing() {
    let mut grids = vec![grid(3, 3, vec![7; 9]), grid(3, 3, vec![7; 9])];
    let coverage = must_stamp(
        &mut grids,
        brush(BrushShape::Circle, 1.0, 1.0, 0.5, 10.0, 3),
    );
    assert_eq!(coverage.covered, 0);
    assert_eq!(coverage.changed, 0);
    assert_eq!(
        grids.first().map(|item| item.indices.as_slice()),
        Some([7, 7, 7, 7, 7, 7, 7, 7, 7].as_slice())
    );
    assert_eq!(
        grids.get(1).map(|item| item.indices.as_slice()),
        Some([7, 7, 7, 7, 7, 7, 7, 7, 7].as_slice())
    );
}

#[test]
fn an_empty_stack_and_a_zero_side_write_nothing() {
    let mut empty: Vec<LabelGrid> = Vec::new();
    assert!(matches!(
        stamp_brush(&mut empty, brush(BrushShape::Sphere, 0.0, 0.0, 0.0, 1.0, 1),),
        Err(BrushError::Degenerate)
    ));
    let mut zero_side = vec![grid(0, 4, Vec::new())];
    assert!(matches!(
        stamp_brush(
            &mut zero_side,
            brush(BrushShape::Circle, 0.0, 0.0, 0.0, 1.0, 1),
        ),
        Err(BrushError::Degenerate)
    ));
    assert_eq!(zero_side.first().map(|item| item.indices.len()), Some(0));
}

#[test]
fn a_non_finite_centre_writes_nothing_and_a_later_stamp_works() {
    let mut grids = vec![grid(3, 3, vec![9; 9])];
    for centre in [f64::INFINITY, f64::NEG_INFINITY, f64::NAN] {
        assert!(matches!(
            stamp_brush(
                &mut grids,
                brush(BrushShape::Sphere, centre, 1.0, 0.0, 1.0, 1),
            ),
            Err(BrushError::Frame)
        ));
    }
    assert!(matches!(
        stamp_brush(
            &mut grids,
            brush(BrushShape::Circle, 1.0, 1.0, 0.0, f64::INFINITY, 1),
        ),
        Err(BrushError::Frame)
    ));
    assert_eq!(
        grids.first().map(|item| item.indices.as_slice()),
        Some([9, 9, 9, 9, 9, 9, 9, 9, 9].as_slice())
    );
    let coverage = must_stamp(&mut grids, brush(BrushShape::Circle, 1.0, 1.0, 0.0, 0.0, 2));
    assert_eq!(coverage.covered, 1);
    assert_eq!(coverage.changed, 1);
    assert_eq!(
        grids.first().and_then(|item| item.indices.get(4).copied()),
        Some(2)
    );
}

#[test]
fn a_long_buffer_is_not_stamped() {
    let mut grids = vec![grid(3, 3, vec![9; 10])];
    assert!(matches!(
        stamp_brush(&mut grids, brush(BrushShape::Circle, 1.0, 1.0, 0.0, 1.0, 1),),
        Err(BrushError::Degenerate)
    ));
    assert_eq!(
        grids.first().map(|item| item.indices.as_slice()),
        Some([9, 9, 9, 9, 9, 9, 9, 9, 9, 9].as_slice())
    );
}

#[test]
fn radius_zero_follows_column_then_row() {
    let mut grids = vec![grid(3, 3, zeros(9))];
    let coverage = must_stamp(&mut grids, brush(BrushShape::Circle, 0.0, 1.0, 0.0, 0.0, 4));
    assert_eq!(coverage.covered, 1);
    assert_eq!(
        grids.first().map(|item| item.indices.as_slice()),
        Some([0, 0, 0, 4, 0, 0, 0, 0, 0].as_slice())
    );
}

#[test]
fn an_already_painted_outside_corner_is_not_a_change() {
    let mut grids = vec![grid(3, 3, vec![2, 0, 0, 0, 2, 0, 0, 0, 0])];
    let coverage = must_stamp(&mut grids, brush(BrushShape::Circle, 1.0, 1.0, 0.0, 1.0, 2));
    assert_eq!(coverage.covered, 5);
    assert_eq!(coverage.changed, 4);
    assert_eq!(
        grids.first().map(|item| item.indices.as_slice()),
        Some([2, 2, 0, 2, 2, 2, 0, 2, 0].as_slice())
    );
}

#[test]
fn a_sphere_halfway_between_slices_paints_both_centres() {
    let mut grids = vec![grid(3, 3, zeros(9)), grid(3, 3, zeros(9))];
    let coverage = must_stamp(&mut grids, brush(BrushShape::Sphere, 1.0, 1.0, 0.5, 0.5, 5));
    assert_eq!(coverage.covered, 2);
    assert_eq!(coverage.changed, 2);
    assert_eq!(
        grids.first().map(|item| item.indices.as_slice()),
        Some([0, 0, 0, 0, 5, 0, 0, 0, 0].as_slice())
    );
    assert_eq!(
        grids.get(1).map(|item| item.indices.as_slice()),
        Some([0, 0, 0, 0, 5, 0, 0, 0, 0].as_slice())
    );
}

#[test]
fn a_radius_just_under_one_misses_the_edges() {
    let mut grids = vec![grid(3, 3, zeros(9))];
    let coverage = must_stamp(
        &mut grids,
        brush(BrushShape::Circle, 1.0, 1.0, 0.0, 1.0_f64.next_down(), 2),
    );
    assert_eq!(coverage.covered, 1);
    assert_eq!(coverage.changed, 1);
    assert_eq!(
        grids.first().map(|item| item.indices.as_slice()),
        Some([0, 0, 0, 0, 2, 0, 0, 0, 0].as_slice())
    );
}
