//! F-082 fill fixtures. The expected indices are hand-computed in
//! `.claude/plans/F-082-design.md`. PS3.3 C.8.20.2 keeps segment 0 as
//! background. The 4-connected rule is the plan, not an HLD formula.

use ocelli_seg::{
    FillCoverage, FillError, LabelColourLut, LabelGrid, LabelStyle, Rgba8, flood_fill, paint_fill,
    present_labelmap,
};

fn grid(width: u32, height: u32, indices: Vec<u16>) -> LabelGrid {
    LabelGrid {
        width,
        height,
        indices,
    }
}

fn must_flood(grid: &mut LabelGrid, x: u32, y: u32, segment: u16) -> FillCoverage {
    match flood_fill(grid, x, y, segment) {
        Ok(coverage) => coverage,
        Err(_) => unreachable!("the plan's in-range flood"),
    }
}

fn must_paint(grid: &mut LabelGrid, x: u32, y: u32, segment: u16) -> FillCoverage {
    match paint_fill(grid, x, y, segment) {
        Ok(coverage) => coverage,
        Err(_) => unreachable!("the plan's in-range paint"),
    }
}

#[test]
fn a_flood_of_the_plus_leaves_the_corners() {
    let mut grid = grid(3, 3, vec![0, 1, 0, 1, 1, 1, 0, 1, 0]);
    let coverage = must_flood(&mut grid, 1, 1, 2);
    assert_eq!(coverage.covered, 5);
    assert_eq!(coverage.changed, 5);
    assert_eq!(
        grid.indices.as_slice(),
        [0, 2, 0, 2, 2, 2, 0, 2, 0].as_slice()
    );
}

#[test]
fn a_flood_does_not_cross_a_diagonal() {
    let mut grid = grid(3, 3, vec![1, 0, 1, 0, 1, 0, 1, 0, 1]);
    let coverage = must_flood(&mut grid, 1, 1, 2);
    assert_eq!(coverage.covered, 1);
    assert_eq!(coverage.changed, 1);
    assert_eq!(
        grid.indices.as_slice(),
        [1, 0, 1, 0, 2, 0, 1, 0, 1].as_slice()
    );
}

#[test]
fn a_gap_stops_a_flood_and_not_a_paint() {
    let mut flooded = grid(3, 1, vec![1, 0, 1]);
    let coverage = must_flood(&mut flooded, 0, 0, 2);
    assert_eq!(coverage.covered, 1);
    assert_eq!(coverage.changed, 1);
    assert_eq!(flooded.indices.as_slice(), [2, 0, 1].as_slice());
    let mut painted = grid(3, 1, vec![1, 0, 1]);
    let coverage = must_paint(&mut painted, 0, 0, 2);
    assert_eq!(coverage.covered, 2);
    assert_eq!(coverage.changed, 2);
    assert_eq!(painted.indices.as_slice(), [2, 0, 2].as_slice());
}

#[test]
fn an_outside_seed_and_a_short_buffer_write_nothing() {
    let mut outside = grid(3, 3, vec![1; 9]);
    assert!(matches!(
        flood_fill(&mut outside, 3, 0, 2),
        Err(FillError::Degenerate)
    ));
    assert_eq!(
        outside.indices.as_slice(),
        [1, 1, 1, 1, 1, 1, 1, 1, 1].as_slice()
    );
    let mut short = grid(2, 2, vec![1, 1, 1]);
    assert!(matches!(
        paint_fill(&mut short, 0, 0, 2),
        Err(FillError::Degenerate)
    ));
    assert_eq!(short.indices.as_slice(), [1, 1, 1].as_slice());
}

#[test]
fn a_second_flood_of_the_same_segment_changes_nothing() {
    let mut grid = grid(3, 3, vec![0, 2, 0, 2, 2, 2, 0, 2, 0]);
    let coverage = must_flood(&mut grid, 1, 1, 2);
    assert_eq!(coverage.covered, 5);
    assert_eq!(coverage.changed, 0);
    assert_eq!(
        grid.indices.as_slice(),
        [0, 2, 0, 2, 2, 2, 0, 2, 0].as_slice()
    );
}

#[test]
fn flooding_background_with_background_leaves_the_ones() {
    let mut grid = grid(3, 1, vec![1, 0, 1]);
    let coverage = must_flood(&mut grid, 1, 0, 0);
    assert_eq!(coverage.covered, 1);
    assert_eq!(coverage.changed, 0);
    assert_eq!(grid.indices.as_slice(), [1, 0, 1].as_slice());
}

#[test]
fn painting_the_plus_to_background_clears_the_presenter_centre() {
    let mut grid = grid(3, 3, vec![0, 1, 0, 1, 1, 1, 0, 1, 0]);
    let coverage = must_paint(&mut grid, 1, 1, 0);
    assert_eq!(coverage.covered, 5);
    assert_eq!(coverage.changed, 5);
    let lut = LabelColourLut::from_segment_one(vec![Rgba8 {
        r: 10,
        g: 20,
        b: 30,
        a: 40,
    }]);
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
    assert_eq!(out.get(16..20), Some([0, 0, 0, 0].as_slice()));
}

#[test]
fn a_flood_follows_a_corner_and_leaves_the_off_snake_sample() {
    let mut grid = grid(3, 3, vec![1, 1, 0, 0, 1, 0, 0, 1, 1]);
    let coverage = must_flood(&mut grid, 0, 0, 2);
    assert_eq!(coverage.covered, 5);
    assert_eq!(coverage.changed, 5);
    assert_eq!(
        grid.indices.as_slice(),
        [2, 2, 0, 0, 2, 0, 0, 2, 2].as_slice()
    );
}
