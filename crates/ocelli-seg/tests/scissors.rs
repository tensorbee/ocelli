//! F-081 scissor fixtures. The expected indices are hand-computed in
//! `.claude/plans/F-081-design.md`. PS3.3 C.8.20.2 keeps segment 0 as
//! background. The span is the plan. The circle reuses the F-080 radius test.

use ocelli_seg::{
    LabelColourLut, LabelGrid, LabelStyle, Rgba8, ScissorCoverage, ScissorError, ScissorShape,
    cut_scissor, present_labelmap,
};

fn grid(width: u32, height: u32, indices: Vec<u16>) -> LabelGrid {
    LabelGrid {
        width,
        height,
        indices,
    }
}

fn ones(count: usize) -> Vec<u16> {
    vec![1; count]
}

fn rectangle(min_x: f64, min_y: f64, max_x: f64, max_y: f64, z: f64) -> ScissorShape {
    ScissorShape::Rectangle {
        min_x,
        min_y,
        max_x,
        max_y,
        z,
    }
}

fn circle(x: f64, y: f64, z: f64, radius: f64) -> ScissorShape {
    ScissorShape::Circle {
        center_x: x,
        center_y: y,
        center_z: z,
        radius,
    }
}

fn sphere(x: f64, y: f64, z: f64, radius: f64) -> ScissorShape {
    ScissorShape::Sphere {
        center_x: x,
        center_y: y,
        center_z: z,
        radius,
    }
}

fn must_cut(grids: &mut [LabelGrid], shape: ScissorShape) -> ScissorCoverage {
    match cut_scissor(grids, shape) {
        Ok(coverage) => coverage,
        Err(_) => unreachable!("the plan's finite scissor"),
    }
}

#[test]
fn a_rectangle_erases_the_inclusive_top_left_block() {
    let mut grids = vec![grid(3, 3, ones(9))];
    let coverage = must_cut(&mut grids, rectangle(0.0, 0.0, 1.0, 1.0, 0.0));
    assert_eq!(coverage.covered, 4);
    assert_eq!(coverage.changed, 4);
    assert_eq!(
        grids.first().map(|item| item.indices.as_slice()),
        Some([0, 0, 1, 0, 0, 1, 1, 1, 1].as_slice())
    );
}

#[test]
fn a_circle_erases_the_plus_and_leaves_the_corners() {
    let mut grids = vec![grid(3, 3, ones(9))];
    let coverage = must_cut(&mut grids, circle(1.0, 1.0, 0.0, 1.0));
    assert_eq!(coverage.covered, 5);
    assert_eq!(coverage.changed, 5);
    assert_eq!(
        grids.first().map(|item| item.indices.as_slice()),
        Some([1, 0, 1, 0, 0, 0, 1, 0, 1].as_slice())
    );
}

#[test]
fn a_sphere_erases_the_next_slice_only_at_the_centre() {
    let mut grids = vec![grid(3, 3, ones(9)), grid(3, 3, ones(9))];
    let coverage = must_cut(&mut grids, sphere(1.0, 1.0, 0.0, 1.0));
    assert_eq!(coverage.covered, 6);
    assert_eq!(coverage.changed, 6);
    assert_eq!(
        grids.first().map(|item| item.indices.as_slice()),
        Some([1, 0, 1, 0, 0, 0, 1, 0, 1].as_slice())
    );
    assert_eq!(
        grids.get(1).map(|item| item.indices.as_slice()),
        Some([1, 1, 1, 1, 0, 1, 1, 1, 1].as_slice())
    );
}

#[test]
fn an_inverted_span_and_a_negative_radius_write_nothing() {
    let mut inverted = vec![grid(3, 3, ones(9))];
    assert!(matches!(
        cut_scissor(&mut inverted, rectangle(2.0, 0.0, 0.0, 2.0, 0.0)),
        Err(ScissorError::Degenerate)
    ));
    assert_eq!(
        inverted.first().map(|item| item.indices.as_slice()),
        Some([1, 1, 1, 1, 1, 1, 1, 1, 1].as_slice())
    );
    let mut negative = vec![grid(3, 3, ones(9))];
    assert!(matches!(
        cut_scissor(&mut negative, circle(1.0, 1.0, 0.0, -1.0)),
        Err(ScissorError::Degenerate)
    ));
    assert_eq!(
        negative.first().map(|item| item.indices.as_slice()),
        Some([1, 1, 1, 1, 1, 1, 1, 1, 1].as_slice())
    );
}

#[test]
fn a_non_finite_z_and_a_short_buffer_write_nothing() {
    let mut non_finite = vec![grid(3, 3, ones(9))];
    assert!(matches!(
        cut_scissor(&mut non_finite, rectangle(0.0, 0.0, 1.0, 1.0, f64::NAN)),
        Err(ScissorError::Frame)
    ));
    assert_eq!(
        non_finite.first().map(|item| item.indices.as_slice()),
        Some([1, 1, 1, 1, 1, 1, 1, 1, 1].as_slice())
    );
    let mut short = vec![grid(3, 3, vec![1; 8])];
    assert!(matches!(
        cut_scissor(&mut short, rectangle(0.0, 0.0, 1.0, 1.0, 0.0)),
        Err(ScissorError::Degenerate)
    ));
    assert_eq!(
        short.first().map(|item| item.indices.as_slice()),
        Some([1, 1, 1, 1, 1, 1, 1, 1].as_slice())
    );
}

#[test]
fn negative_zero_selects_slice_zero_and_a_fraction_selects_none() {
    let mut grids = vec![grid(3, 3, ones(9))];
    let coverage = must_cut(&mut grids, rectangle(0.0, 0.0, 1.0, 1.0, -0.0));
    assert_eq!(coverage.covered, 4);
    assert_eq!(coverage.changed, 4);
    assert_eq!(
        grids.first().map(|item| item.indices.as_slice()),
        Some([0, 0, 1, 0, 0, 1, 1, 1, 1].as_slice())
    );
    let mut missed = vec![grid(3, 3, ones(9))];
    let coverage = must_cut(&mut missed, rectangle(0.0, 0.0, 2.0, 2.0, 0.5));
    assert_eq!(coverage.covered, 0);
    assert_eq!(coverage.changed, 0);
    assert_eq!(
        missed.first().map(|item| item.indices.as_slice()),
        Some([1, 1, 1, 1, 1, 1, 1, 1, 1].as_slice())
    );
}

#[test]
fn a_second_cut_of_an_already_blank_plus_changes_nothing() {
    let mut grids = vec![grid(3, 3, vec![1, 0, 1, 0, 0, 0, 1, 0, 1])];
    let coverage = must_cut(&mut grids, circle(1.0, 1.0, 0.0, 1.0));
    assert_eq!(coverage.covered, 5);
    assert_eq!(coverage.changed, 0);
    assert_eq!(
        grids.first().map(|item| item.indices.as_slice()),
        Some([1, 0, 1, 0, 0, 0, 1, 0, 1].as_slice())
    );
}

#[test]
fn mismatched_slices_are_not_cut() {
    let mut grids = vec![grid(3, 3, ones(9)), grid(2, 2, ones(4))];
    assert!(matches!(
        cut_scissor(&mut grids, sphere(0.0, 0.0, 0.0, 2.0)),
        Err(ScissorError::Degenerate)
    ));
    assert_eq!(
        grids.first().map(|item| item.indices.as_slice()),
        Some([1, 1, 1, 1, 1, 1, 1, 1, 1].as_slice())
    );
    assert_eq!(
        grids.get(1).map(|item| item.indices.as_slice()),
        Some([1, 1, 1, 1].as_slice())
    );
}

#[test]
fn the_presenter_keeps_a_corner_and_clears_the_cut_centre() {
    let mut grids = vec![grid(3, 3, ones(9))];
    must_cut(&mut grids, circle(1.0, 1.0, 0.0, 1.0));
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
    assert_eq!(out.get(0..4), Some([10, 20, 30, 40].as_slice()));
    assert_eq!(out.get(16..20), Some([0, 0, 0, 0].as_slice()));
}

#[test]
fn a_rectangle_does_not_erase_another_slice() {
    let mut grids = vec![grid(3, 3, ones(9)), grid(3, 3, ones(9))];
    let coverage = must_cut(&mut grids, rectangle(0.0, 0.0, 1.0, 1.0, 0.0));
    assert_eq!(coverage.covered, 4);
    assert_eq!(
        grids.get(1).map(|item| item.indices.as_slice()),
        Some([1, 1, 1, 1, 1, 1, 1, 1, 1].as_slice())
    );
}

#[test]
fn a_rectangle_uses_x_as_the_column() {
    let mut grids = vec![grid(3, 3, ones(9)), grid(3, 3, ones(9))];
    let coverage = must_cut(&mut grids, rectangle(0.0, 0.0, 0.0, 1.0, 1.0));
    assert_eq!(coverage.covered, 2);
    assert_eq!(coverage.changed, 2);
    assert_eq!(
        grids.first().map(|item| item.indices.as_slice()),
        Some([1, 1, 1, 1, 1, 1, 1, 1, 1].as_slice())
    );
    assert_eq!(
        grids.get(1).map(|item| item.indices.as_slice()),
        Some([0, 1, 1, 0, 1, 1, 1, 1, 1].as_slice())
    );
}

#[test]
fn a_fractional_bound_does_not_floor() {
    let mut grids = vec![grid(3, 3, ones(9))];
    let coverage = must_cut(&mut grids, rectangle(0.5, 0.0, 1.0, 0.5, 0.0));
    assert_eq!(coverage.covered, 1);
    assert_eq!(
        grids.first().map(|item| item.indices.as_slice()),
        Some([1, 0, 1, 1, 1, 1, 1, 1, 1].as_slice())
    );
}

#[test]
fn a_positive_zero_to_negative_zero_span_clears_index_zero() {
    let mut grids = vec![grid(3, 3, ones(9))];
    let coverage = must_cut(&mut grids, rectangle(0.0, 0.0, -0.0, -0.0, 0.0));
    assert_eq!(coverage.covered, 1);
    assert_eq!(coverage.changed, 1);
    assert_eq!(
        grids.first().map(|item| item.indices.as_slice()),
        Some([0, 1, 1, 1, 1, 1, 1, 1, 1].as_slice())
    );
}

#[test]
fn an_existing_zero_is_covered_and_not_changed() {
    let mut grids = vec![grid(3, 3, vec![4, 0, 2, 0, 4, 2, 7, 7, 7])];
    let coverage = must_cut(&mut grids, rectangle(0.0, 0.0, 1.0, 1.0, 0.0));
    assert_eq!(coverage.covered, 4);
    assert_eq!(coverage.changed, 2);
    assert_eq!(
        grids.first().map(|item| item.indices.as_slice()),
        Some([0, 0, 2, 0, 0, 2, 7, 7, 7].as_slice())
    );
    let coverage = must_cut(&mut grids, rectangle(0.0, 0.0, 1.0, 1.0, 0.0));
    assert_eq!(coverage.covered, 4);
    assert_eq!(coverage.changed, 0);
}

#[test]
fn a_refused_cut_does_not_block_a_later_one() {
    let mut grids = vec![grid(3, 3, ones(9))];
    assert!(matches!(
        cut_scissor(&mut grids, rectangle(f64::NAN, 0.0, 1.0, 1.0, 0.0)),
        Err(ScissorError::Frame)
    ));
    assert_eq!(
        grids.first().map(|item| item.indices.as_slice()),
        Some([1, 1, 1, 1, 1, 1, 1, 1, 1].as_slice())
    );
    let coverage = must_cut(&mut grids, rectangle(0.0, 0.0, 0.0, 0.0, 0.0));
    assert_eq!(coverage.covered, 1);
    assert_eq!(
        grids.first().map(|item| item.indices.as_slice()),
        Some([0, 1, 1, 1, 1, 1, 1, 1, 1].as_slice())
    );
}

#[test]
fn a_circle_does_not_erase_the_next_slice() {
    let mut grids = vec![grid(3, 3, ones(9)), grid(3, 3, ones(9))];
    let coverage = must_cut(&mut grids, circle(1.0, 1.0, 0.0, 1.0));
    assert_eq!(coverage.covered, 5);
    assert_eq!(
        grids.get(1).map(|item| item.indices.as_slice()),
        Some([1, 1, 1, 1, 1, 1, 1, 1, 1].as_slice())
    );
}

#[test]
fn an_empty_stack_a_zero_side_and_a_long_buffer_are_degenerate() {
    let mut empty: Vec<LabelGrid> = Vec::new();
    assert!(matches!(
        cut_scissor(&mut empty, rectangle(0.0, 0.0, 1.0, 1.0, 0.0)),
        Err(ScissorError::Degenerate)
    ));
    let mut zero_side = vec![grid(0, 1, Vec::new())];
    assert!(matches!(
        cut_scissor(&mut zero_side, rectangle(0.0, 0.0, 0.0, 0.0, 0.0)),
        Err(ScissorError::Degenerate)
    ));
    assert_eq!(zero_side.first().map(|item| item.indices.len()), Some(0));
    let mut long = vec![grid(3, 3, vec![9; 10])];
    assert!(matches!(
        cut_scissor(&mut long, rectangle(0.0, 0.0, 2.0, 2.0, 0.0)),
        Err(ScissorError::Degenerate)
    ));
    assert_eq!(long.first().map(|item| item.indices.len()), Some(10));
}

#[test]
fn a_sphere_halfway_between_slices_erases_both_centres() {
    let mut grids = vec![grid(3, 3, ones(9)), grid(3, 3, ones(9))];
    let coverage = must_cut(&mut grids, sphere(1.0, 1.0, 0.5, 0.5));
    assert_eq!(coverage.covered, 2);
    assert_eq!(coverage.changed, 2);
    assert_eq!(
        grids.first().map(|item| item.indices.as_slice()),
        Some([1, 1, 1, 1, 0, 1, 1, 1, 1].as_slice())
    );
    assert_eq!(
        grids.get(1).map(|item| item.indices.as_slice()),
        Some([1, 1, 1, 1, 0, 1, 1, 1, 1].as_slice())
    );
}

#[test]
fn a_negative_zero_bound_includes_index_zero() {
    let mut grids = vec![grid(3, 3, ones(9))];
    let coverage = must_cut(&mut grids, rectangle(-0.0, -0.0, -0.0, -0.0, 0.0));
    assert_eq!(coverage.covered, 1);
    assert_eq!(coverage.changed, 1);
    assert_eq!(
        grids.first().map(|item| item.indices.as_slice()),
        Some([0, 1, 1, 1, 1, 1, 1, 1, 1].as_slice())
    );
}
