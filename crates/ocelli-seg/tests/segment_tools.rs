//! F-084 segment fixtures. The expected runs are hand-computed in
//! `.claude/plans/F-084-design.md`. PS3.3 C.8.20.2 keeps segment 0 as
//! background. The run rule is the plan, not an HLD formula.

use ocelli_seg::{
    LabelGrid, SegmentBidirectional, SegmentLabel, SegmentToolError, label_segment, segment_at,
    segment_bidirectional,
};

fn grid(width: u32, height: u32, indices: Vec<u16>) -> LabelGrid {
    LabelGrid {
        width,
        height,
        indices,
    }
}

fn sample_grid() -> LabelGrid {
    grid(4, 3, vec![0, 1, 1, 0, 0, 1, 0, 0, 1, 1, 1, 1])
}

fn must_at(grid: &LabelGrid, x: u32, y: u32) -> u16 {
    match segment_at(grid, x, y) {
        Ok(segment) => segment,
        Err(_) => unreachable!("the plan's in-range read"),
    }
}

fn must_runs(grid: &LabelGrid, segment: u16) -> SegmentBidirectional {
    match segment_bidirectional(grid, segment) {
        Ok(runs) => runs,
        Err(_) => unreachable!("the plan's present segment"),
    }
}

#[test]
fn an_in_range_read_returns_the_segment_and_an_outside_read_does_not() {
    let grid = sample_grid();
    assert_eq!(must_at(&grid, 1, 0), 1);
    assert_eq!(must_at(&grid, 0, 0), 0);
    assert!(matches!(
        segment_at(&grid, 4, 0),
        Err(SegmentToolError::Degenerate)
    ));
}

#[test]
fn a_short_buffer_is_not_read_as_background() {
    let grid = grid(2, 2, vec![1, 1, 1]);
    assert!(matches!(
        segment_at(&grid, 0, 0),
        Err(SegmentToolError::Degenerate)
    ));
}

#[test]
fn a_background_segment_and_a_blank_label_are_refused() {
    let mut labels = Vec::new();
    assert!(matches!(
        label_segment(&mut labels, 0, "liver"),
        Err(SegmentToolError::Background)
    ));
    assert!(labels.is_empty());
    assert!(matches!(
        label_segment(&mut labels, 2, "  "),
        Err(SegmentToolError::Blank)
    ));
    assert!(labels.is_empty());
    match label_segment(&mut labels, 2, " liver ") {
        Ok(()) => {}
        Err(_) => unreachable!("a trimmed label is stored"),
    }
    assert_eq!(labels.len(), 1);
    assert_eq!(labels.first().map(|label| label.segment), Some(2));
    assert_eq!(
        labels.first().map(|label| label.text.as_str()),
        Some("liver")
    );
}

#[test]
fn replacing_a_label_keeps_its_place() {
    let mut labels: Vec<SegmentLabel> = Vec::new();
    match label_segment(&mut labels, 1, "a") {
        Ok(()) => {}
        Err(_) => unreachable!("segment 1"),
    }
    match label_segment(&mut labels, 2, "b") {
        Ok(()) => {}
        Err(_) => unreachable!("segment 2"),
    }
    match label_segment(&mut labels, 1, "c") {
        Ok(()) => {}
        Err(_) => unreachable!("replacement"),
    }
    assert_eq!(labels.first().map(|label| label.segment), Some(1));
    assert_eq!(labels.first().map(|label| label.text.as_str()), Some("c"));
    assert_eq!(labels.get(1).map(|label| label.segment), Some(2));
    assert_eq!(labels.get(1).map(|label| label.text.as_str()), Some("b"));
}

#[test]
fn the_longest_row_and_column_are_the_bidirectional_pair() {
    let runs = must_runs(&sample_grid(), 1);
    assert_eq!(runs.horizontal.start_x, 0);
    assert_eq!(runs.horizontal.start_y, 2);
    assert_eq!(runs.horizontal.end_x, 3);
    assert_eq!(runs.horizontal.end_y, 2);
    assert_eq!(runs.horizontal.length, 4);
    assert_eq!(runs.vertical.start_x, 1);
    assert_eq!(runs.vertical.start_y, 0);
    assert_eq!(runs.vertical.end_x, 1);
    assert_eq!(runs.vertical.end_y, 2);
    assert_eq!(runs.vertical.length, 3);
}

#[test]
fn an_equal_horizontal_run_keeps_the_upper_one() {
    let grid = grid(3, 2, vec![1, 1, 0, 0, 1, 1]);
    let runs = must_runs(&grid, 1);
    assert_eq!(runs.horizontal.length, 2);
    assert_eq!(runs.horizontal.start_x, 0);
    assert_eq!(runs.horizontal.start_y, 0);
    assert_eq!(runs.horizontal.end_x, 1);
    assert_eq!(runs.horizontal.end_y, 0);
}

#[test]
fn an_equal_vertical_run_keeps_the_left_one() {
    let grid = grid(2, 2, vec![1, 1, 1, 1]);
    let runs = must_runs(&grid, 1);
    assert_eq!(runs.vertical.length, 2);
    assert_eq!(runs.vertical.start_x, 0);
    assert_eq!(runs.vertical.start_y, 0);
    assert_eq!(runs.vertical.end_x, 0);
    assert_eq!(runs.vertical.end_y, 1);
    assert_eq!(runs.horizontal.start_y, 0);
    assert_eq!(runs.horizontal.length, 2);
}

#[test]
fn a_missing_segment_and_background_are_not_runs() {
    let sample = sample_grid();
    assert!(matches!(
        segment_bidirectional(&sample, 2),
        Err(SegmentToolError::Degenerate)
    ));
    let zeros = grid(2, 2, vec![0, 0, 0, 0]);
    assert!(matches!(
        segment_bidirectional(&zeros, 0),
        Err(SegmentToolError::Degenerate)
    ));
}

#[test]
fn one_sample_is_a_run_of_length_one_on_both_axes() {
    let mut indices = vec![0, 0, 0, 0, 0, 0, 0, 0, 0];
    let Some(slot) = indices.get_mut(5) else {
        unreachable!("nine samples");
    };
    *slot = 1;
    let grid = grid(3, 3, indices);
    let runs = must_runs(&grid, 1);
    assert_eq!(runs.horizontal.start_x, 2);
    assert_eq!(runs.horizontal.start_y, 1);
    assert_eq!(runs.horizontal.end_x, 2);
    assert_eq!(runs.horizontal.end_y, 1);
    assert_eq!(runs.horizontal.length, 1);
    assert_eq!(runs.vertical.start_x, 2);
    assert_eq!(runs.vertical.start_y, 1);
    assert_eq!(runs.vertical.end_x, 2);
    assert_eq!(runs.vertical.end_y, 1);
    assert_eq!(runs.vertical.length, 1);
}

#[test]
fn a_refused_label_leaves_an_existing_one() {
    let mut labels = Vec::new();
    match label_segment(&mut labels, 3, "liver") {
        Ok(()) => {}
        Err(_) => unreachable!("the first label"),
    }
    assert!(matches!(
        label_segment(&mut labels, 3, "  "),
        Err(SegmentToolError::Blank)
    ));
    assert!(matches!(
        label_segment(&mut labels, 0, "nope"),
        Err(SegmentToolError::Background)
    ));
    assert_eq!(labels.len(), 1);
    assert_eq!(
        labels.first().map(|label| label.text.as_str()),
        Some("liver")
    );
    match label_segment(&mut labels, 4, "spleen") {
        Ok(()) => {}
        Err(_) => unreachable!("a later label"),
    }
    assert_eq!(labels.first().map(|label| label.segment), Some(3));
    assert_eq!(
        labels.first().map(|label| label.text.as_str()),
        Some("liver")
    );
    assert_eq!(labels.get(1).map(|label| label.segment), Some(4));
    assert_eq!(
        labels.get(1).map(|label| label.text.as_str()),
        Some("spleen")
    );
}

#[test]
fn a_gap_keeps_the_first_sample() {
    let grid = grid(3, 1, vec![1, 0, 1]);
    let runs = must_runs(&grid, 1);
    assert_eq!(runs.horizontal.start_x, 0);
    assert_eq!(runs.horizontal.end_x, 0);
    assert_eq!(runs.horizontal.length, 1);
    assert_eq!(runs.vertical.start_x, 0);
    assert_eq!(runs.vertical.length, 1);
}

#[test]
fn a_long_buffer_is_not_read_as_a_segment() {
    let grid = grid(2, 2, vec![1, 1, 1, 1, 9]);
    assert!(matches!(
        segment_at(&grid, 0, 0),
        Err(SegmentToolError::Degenerate)
    ));
    assert!(matches!(
        segment_bidirectional(&grid, 1),
        Err(SegmentToolError::Degenerate)
    ));
    assert_eq!(grid.indices.as_slice(), [1, 1, 1, 1, 9]);
}

#[test]
fn a_zero_side_and_an_outside_row_are_degenerate() {
    let zero = grid(0, 1, Vec::new());
    assert!(matches!(
        segment_at(&zero, 0, 0),
        Err(SegmentToolError::Degenerate)
    ));
    let sample = sample_grid();
    assert!(matches!(
        segment_at(&sample, 0, 3),
        Err(SegmentToolError::Degenerate)
    ));
    assert_eq!(sample.indices.len(), 12);
}
