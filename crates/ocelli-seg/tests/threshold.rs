//! F-083 threshold fixtures. The expected indices are hand-computed in
//! `.claude/plans/F-083-design.md`. PS3.3 C.8.20.2 keeps segment 0 as
//! background. The value span is the plan, not an HLD formula.

use ocelli_seg::{
    LabelColourLut, LabelGrid, LabelStyle, Rgba8, ThresholdCoverage, ThresholdError, ThresholdRoi,
    ThresholdShape, present_labelmap, threshold_roi,
};

fn grid(width: u32, height: u32, indices: Vec<u16>) -> LabelGrid {
    LabelGrid {
        width,
        height,
        indices,
    }
}

fn zeros(count: usize) -> Vec<u16> {
    vec![0; count]
}

fn values_zero_through_eight() -> Vec<f64> {
    vec![0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0]
}

fn rectangle(min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> ThresholdShape {
    ThresholdShape::Rectangle {
        min_x,
        min_y,
        max_x,
        max_y,
    }
}

fn circle(x: f64, y: f64, radius: f64) -> ThresholdShape {
    ThresholdShape::Circle {
        center_x: x,
        center_y: y,
        radius,
    }
}

fn roi(shape: ThresholdShape, start: f64, end: f64, segment: u16) -> ThresholdRoi {
    ThresholdRoi {
        shape,
        start,
        end,
        segment,
    }
}

fn must_paint(grid: &mut LabelGrid, values: &[f64], roi: ThresholdRoi) -> ThresholdCoverage {
    match threshold_roi(grid, values, roi) {
        Ok(coverage) => coverage,
        Err(_) => unreachable!("the plan's finite threshold"),
    }
}

#[test]
fn a_rectangle_paints_values_from_three_through_five() {
    let mut grid = grid(3, 3, zeros(9));
    let coverage = must_paint(
        &mut grid,
        &values_zero_through_eight(),
        roi(rectangle(0.0, 0.0, 2.0, 2.0), 3.0, 5.0, 7),
    );
    assert_eq!(coverage.covered, 3);
    assert_eq!(coverage.changed, 3);
    assert_eq!(
        grid.indices.as_slice(),
        [0, 0, 0, 7, 7, 7, 0, 0, 0].as_slice()
    );
}

#[test]
fn a_circle_paints_the_plus_and_leaves_the_corners() {
    let mut grid = grid(3, 3, zeros(9));
    let coverage = must_paint(
        &mut grid,
        &values_zero_through_eight(),
        roi(circle(1.0, 1.0, 1.0), 0.0, 8.0, 4),
    );
    assert_eq!(coverage.covered, 5);
    assert_eq!(coverage.changed, 5);
    assert_eq!(
        grid.indices.as_slice(),
        [0, 4, 0, 4, 4, 4, 0, 4, 0].as_slice()
    );
}

#[test]
fn a_non_finite_value_outside_the_shape_does_not_refuse() {
    let values = vec![4.0, 0.0, f64::NAN, 4.0, 0.0, 0.0, 4.0, 0.0, 0.0];
    let mut grid = grid(3, 3, zeros(9));
    let coverage = must_paint(
        &mut grid,
        &values,
        roi(rectangle(0.0, 0.0, 0.0, 2.0), 4.0, 4.0, 7),
    );
    assert_eq!(coverage.covered, 3);
    assert_eq!(
        grid.indices.as_slice(),
        [7, 0, 0, 7, 0, 0, 7, 0, 0].as_slice()
    );
}

#[test]
fn a_non_finite_value_inside_the_shape_writes_nothing() {
    let values = vec![0.0, 1.0, 2.0, 3.0, f64::NAN, 5.0, 6.0, 7.0, 8.0];
    let mut grid = grid(3, 3, vec![1; 9]);
    assert!(matches!(
        threshold_roi(
            &mut grid,
            &values,
            roi(rectangle(0.0, 0.0, 2.0, 2.0), 3.0, 5.0, 7),
        ),
        Err(ThresholdError::Frame)
    ));
    assert_eq!(grid.indices.as_slice(), [1; 9].as_slice());
}

#[test]
fn an_inverted_span_and_a_negative_radius_write_nothing() {
    let mut grid = grid(3, 3, vec![1; 9]);
    let values = values_zero_through_eight();
    assert!(matches!(
        threshold_roi(
            &mut grid,
            &values,
            roi(rectangle(0.0, 0.0, 2.0, 2.0), 5.0, 3.0, 7),
        ),
        Err(ThresholdError::Degenerate)
    ));
    assert!(matches!(
        threshold_roi(&mut grid, &values, roi(circle(1.0, 1.0, -1.0), 0.0, 8.0, 7),),
        Err(ThresholdError::Degenerate)
    ));
    assert_eq!(grid.indices.as_slice(), [1; 9].as_slice());
}

#[test]
fn a_short_value_buffer_and_a_zero_side_write_nothing() {
    let mut full = grid(3, 3, vec![1; 9]);
    assert!(matches!(
        threshold_roi(
            &mut full,
            &[0.0; 8],
            roi(rectangle(0.0, 0.0, 2.0, 2.0), 0.0, 1.0, 7),
        ),
        Err(ThresholdError::Degenerate)
    ));
    assert_eq!(full.indices.as_slice(), [1; 9].as_slice());
    let mut empty = grid(0, 1, Vec::new());
    assert!(matches!(
        threshold_roi(
            &mut empty,
            &[],
            roi(rectangle(0.0, 0.0, 0.0, 0.0), 0.0, 1.0, 7),
        ),
        Err(ThresholdError::Degenerate)
    ));
}

#[test]
fn a_second_paint_changes_nothing() {
    let mut grid = grid(3, 3, zeros(9));
    let values = values_zero_through_eight();
    let request = roi(rectangle(0.0, 0.0, 2.0, 2.0), 3.0, 5.0, 7);
    must_paint(&mut grid, &values, request);
    let coverage = must_paint(&mut grid, &values, request);
    assert_eq!(coverage.covered, 3);
    assert_eq!(coverage.changed, 0);
    assert_eq!(
        grid.indices.as_slice(),
        [0, 0, 0, 7, 7, 7, 0, 0, 0].as_slice()
    );
}

#[test]
fn an_exact_value_paints_only_that_sample() {
    let mut grid = grid(3, 3, zeros(9));
    let coverage = must_paint(
        &mut grid,
        &values_zero_through_eight(),
        roi(rectangle(0.0, 0.0, 2.0, 2.0), 4.0, 4.0, 9),
    );
    assert_eq!(coverage.covered, 1);
    assert_eq!(
        grid.indices.as_slice(),
        [0, 0, 0, 0, 9, 0, 0, 0, 0].as_slice()
    );
}

#[test]
fn the_presenter_colours_a_painted_edge_and_clears_a_corner() {
    let mut grid = grid(3, 3, zeros(9));
    must_paint(
        &mut grid,
        &values_zero_through_eight(),
        roi(circle(1.0, 1.0, 1.0), 0.0, 8.0, 4),
    );
    let lut = LabelColourLut::from_segment_one(vec![
        Rgba8 {
            r: 0,
            g: 0,
            b: 0,
            a: 0,
        },
        Rgba8 {
            r: 0,
            g: 0,
            b: 0,
            a: 0,
        },
        Rgba8 {
            r: 0,
            g: 0,
            b: 0,
            a: 0,
        },
        Rgba8 {
            r: 10,
            g: 20,
            b: 30,
            a: 40,
        },
    ]);
    let mut out = vec![7_u8; 36];
    match present_labelmap(
        &grid,
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
    assert_eq!(out.get(4..8), Some([10, 20, 30, 40].as_slice()));
    assert_eq!(out.get(0..4), Some([0, 0, 0, 0].as_slice()));
}
