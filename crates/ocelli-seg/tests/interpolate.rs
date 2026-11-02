//! F-086 interpolation fixtures. The expected indices are hand-computed in
//! `.claude/plans/F-086-design.md`. PS3.3 C.8.20.2 keeps segment 0 as
//! background. The halfway rule is the plan, not an HLD formula.

use ocelli_seg::{
    InterpolateCoverage, InterpolateError, LabelColourLut, LabelGrid, LabelStyle, Rgba8,
    interpolate_labelmap, present_labelmap,
};

fn label(width: u32, height: u32, indices: Vec<u16>) -> LabelGrid {
    LabelGrid {
        width,
        height,
        indices,
    }
}

fn at(grids: &[LabelGrid], z: usize) -> &[u16] {
    match grids.get(z) {
        Some(grid) => grid.indices.as_slice(),
        None => unreachable!("the fixture stack has this slice"),
    }
}

fn must_fill(grids: &mut [LabelGrid], segment: u16) -> InterpolateCoverage {
    match interpolate_labelmap(grids, segment) {
        Ok(coverage) => coverage,
        Err(_) => unreachable!("the plan's rectangular stack"),
    }
}

#[test]
fn a_middle_slice_takes_the_shared_end_segment() {
    let mut grids = [
        label(1, 1, vec![4]),
        label(1, 1, vec![0]),
        label(1, 1, vec![4]),
    ];
    let coverage = must_fill(&mut grids, 4);
    assert_eq!(coverage.covered, 1);
    assert_eq!(coverage.changed, 1);
    assert_eq!(at(&grids, 0), [4].as_slice());
    assert_eq!(at(&grids, 1), [4].as_slice());
    assert_eq!(at(&grids, 2), [4].as_slice());
}

#[test]
fn the_halfway_slice_takes_the_union() {
    let mut grids = [
        label(3, 1, vec![4, 0, 0]),
        label(3, 1, vec![0, 0, 0]),
        label(3, 1, vec![0, 0, 4]),
    ];
    let coverage = must_fill(&mut grids, 4);
    assert_eq!(coverage.covered, 2);
    assert_eq!(coverage.changed, 2);
    assert_eq!(at(&grids, 1), [4, 0, 4].as_slice());
    assert_eq!(at(&grids, 0), [4, 0, 0].as_slice());
    assert_eq!(at(&grids, 2), [0, 0, 4].as_slice());
}

#[test]
fn a_near_slice_takes_only_the_closer_mask() {
    let mut grids = [
        label(2, 1, vec![4, 0]),
        label(2, 1, vec![0, 0]),
        label(2, 1, vec![0, 0]),
        label(2, 1, vec![0, 4]),
    ];
    let coverage = must_fill(&mut grids, 4);
    assert_eq!(coverage.covered, 2);
    assert_eq!(coverage.changed, 2);
    assert_eq!(at(&grids, 1), [4, 0].as_slice());
    assert_eq!(at(&grids, 2), [0, 4].as_slice());
    assert_eq!(at(&grids, 0), [4, 0].as_slice());
    assert_eq!(at(&grids, 3), [0, 4].as_slice());
}

#[test]
fn one_key_and_adjacent_keys_write_nothing() {
    let mut one = [label(1, 1, vec![4]), label(1, 1, vec![7])];
    let coverage = must_fill(&mut one, 4);
    assert_eq!(coverage.covered, 0);
    assert_eq!(coverage.changed, 0);
    assert_eq!(at(&one, 0), [4].as_slice());
    assert_eq!(at(&one, 1), [7].as_slice());
    let mut adjacent = [label(1, 1, vec![4]), label(1, 1, vec![4])];
    let coverage = must_fill(&mut adjacent, 4);
    assert_eq!(coverage.covered, 0);
    assert_eq!(at(&adjacent, 0), [4].as_slice());
    assert_eq!(at(&adjacent, 1), [4].as_slice());
}

#[test]
fn an_empty_stack_and_a_width_mismatch_write_nothing() {
    let mut empty: [LabelGrid; 0] = [];
    assert!(matches!(
        interpolate_labelmap(&mut empty, 4),
        Err(InterpolateError::Degenerate)
    ));
    let mut mismatched = [label(2, 1, vec![4, 0]), label(1, 1, vec![4])];
    assert!(matches!(
        interpolate_labelmap(&mut mismatched, 4),
        Err(InterpolateError::Degenerate)
    ));
    assert_eq!(at(&mismatched, 0), [4, 0].as_slice());
    assert_eq!(at(&mismatched, 1), [4].as_slice());
    let mut zero_width = [label(0, 1, vec![])];
    assert!(matches!(
        interpolate_labelmap(&mut zero_width, 4),
        Err(InterpolateError::Degenerate)
    ));
    let mut zero_height = [label(1, 0, vec![])];
    assert!(matches!(
        interpolate_labelmap(&mut zero_height, 4),
        Err(InterpolateError::Degenerate)
    ));
    let mut bad_length = [label(2, 1, vec![4, 0]), label(2, 1, vec![4])];
    assert!(matches!(
        interpolate_labelmap(&mut bad_length, 4),
        Err(InterpolateError::Degenerate)
    ));
    assert_eq!(at(&bad_length, 0), [4, 0].as_slice());
    assert_eq!(at(&bad_length, 1), [4].as_slice());
}

#[test]
fn a_second_interpolation_changes_nothing() {
    let mut grids = [
        label(1, 1, vec![4]),
        label(1, 1, vec![0]),
        label(1, 1, vec![4]),
    ];
    must_fill(&mut grids, 4);
    let coverage = must_fill(&mut grids, 4);
    // The filled middle holds the segment, so it is a key. No slice sits
    // strictly between the keys, and the second call writes nothing.
    assert_eq!(coverage.covered, 0);
    assert_eq!(coverage.changed, 0);
    assert_eq!(at(&grids, 0), [4].as_slice());
    assert_eq!(at(&grids, 1), [4].as_slice());
    assert_eq!(at(&grids, 2), [4].as_slice());
}

#[test]
fn interpolating_background_clears_the_presenter() {
    let mut grids = [
        label(1, 1, vec![0]),
        label(1, 1, vec![5]),
        label(1, 1, vec![0]),
    ];
    let coverage = must_fill(&mut grids, 0);
    assert_eq!(coverage.covered, 1);
    assert_eq!(coverage.changed, 1);
    assert_eq!(at(&grids, 1), [0].as_slice());
    let lut = LabelColourLut::from_segment_one(vec![Rgba8 {
        r: 10,
        g: 20,
        b: 30,
        a: 40,
    }]);
    let mut out = vec![7_u8; 4];
    let Some(middle) = grids.get(1) else {
        unreachable!("the fixture has a middle slice");
    };
    match present_labelmap(
        middle,
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
    assert_eq!(out.as_slice(), [0, 0, 0, 0].as_slice());
}

#[test]
fn a_slice_past_the_upper_key_stays() {
    let mut grids = [
        label(1, 1, vec![4]),
        label(1, 1, vec![0]),
        label(1, 1, vec![4]),
        label(1, 1, vec![9]),
    ];
    let coverage = must_fill(&mut grids, 4);
    assert_eq!(coverage.covered, 1);
    assert_eq!(at(&grids, 1), [4].as_slice());
    assert_eq!(at(&grids, 3), [9].as_slice());
    assert_eq!(at(&grids, 2), [4].as_slice());
}

#[test]
fn consecutive_pairs_do_not_use_the_outer_keys() {
    // Pair (0, 2) is the union of [4, 0] and [0, 4]. Pair (2, 4) is the
    // union of [0, 4] and [4, 0]. The outer pair alone would write [4, 0]
    // on both between-slices and would rewrite the middle key.
    let mut grids = [
        label(2, 1, vec![4, 0]),
        label(2, 1, vec![0, 0]),
        label(2, 1, vec![0, 4]),
        label(2, 1, vec![0, 0]),
        label(2, 1, vec![4, 0]),
    ];
    let coverage = must_fill(&mut grids, 4);
    assert_eq!(coverage.covered, 4);
    assert_eq!(coverage.changed, 4);
    assert_eq!(at(&grids, 1), [4, 4].as_slice());
    assert_eq!(at(&grids, 3), [4, 4].as_slice());
    assert_eq!(at(&grids, 2), [0, 4].as_slice());
    assert_eq!(at(&grids, 0), [4, 0].as_slice());
    assert_eq!(at(&grids, 4), [4, 0].as_slice());
}
