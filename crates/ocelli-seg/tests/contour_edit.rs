//! F-085 contour-edit fixtures. The expected indices are hand-computed in
//! `.claude/plans/F-085-design.md`. PS3.3 C.8.20.2 keeps segment 0 as
//! background. The even-odd rule is the plan, not an HLD formula.

use ocelli_seg::{
    ContourEditCoverage, ContourEditError, IndexPoint, LabelColourLut, LabelGrid, LabelStyle,
    Rgba8, edit_labelmap_with_contour, present_labelmap,
};

fn grid(width: u32, height: u32, indices: Vec<u16>) -> LabelGrid {
    LabelGrid {
        width,
        height,
        indices,
    }
}

fn point(x: f64, y: f64) -> IndexPoint {
    IndexPoint { x, y }
}

fn rectangle() -> Vec<IndexPoint> {
    vec![
        point(1.0, 0.0),
        point(3.0, 0.0),
        point(3.0, 2.0),
        point(1.0, 2.0),
    ]
}

fn must_edit(grid: &mut LabelGrid, polygon: &[IndexPoint], segment: u16) -> ContourEditCoverage {
    match edit_labelmap_with_contour(grid, polygon, segment) {
        Ok(coverage) => coverage,
        Err(_) => unreachable!("the plan's finite polygon"),
    }
}

#[test]
fn a_rectangle_paints_the_middle_columns_and_the_boundary() {
    let mut grid = grid(5, 3, vec![0; 15]);
    let coverage = must_edit(&mut grid, &rectangle(), 4);
    assert_eq!(coverage.covered, 9);
    assert_eq!(coverage.changed, 9);
    assert_eq!(
        grid.indices.as_slice(),
        [0, 4, 4, 4, 0, 0, 4, 4, 4, 0, 0, 4, 4, 4, 0].as_slice()
    );
}

#[test]
fn a_triangle_includes_the_apex_and_the_interior_sample() {
    let mut grid = grid(3, 3, vec![0; 9]);
    let polygon = [point(2.0, 0.0), point(2.0, 2.0), point(0.0, 1.0)];
    let coverage = must_edit(&mut grid, &polygon, 5);
    assert_eq!(coverage.covered, 5);
    assert_eq!(coverage.changed, 5);
    assert_eq!(
        grid.indices.as_slice(),
        [0, 0, 5, 5, 5, 5, 0, 0, 5].as_slice()
    );
}

#[test]
fn two_points_and_a_short_buffer_write_nothing() {
    let mut short_poly = grid(3, 3, vec![7; 9]);
    assert!(matches!(
        edit_labelmap_with_contour(&mut short_poly, &[point(0.0, 0.0), point(1.0, 0.0)], 4),
        Err(ContourEditError::Degenerate)
    ));
    assert_eq!(short_poly.indices.as_slice(), [7; 9].as_slice());
    let mut short_grid = grid(2, 2, vec![7, 7, 7]);
    assert!(matches!(
        edit_labelmap_with_contour(
            &mut short_grid,
            &[point(0.0, 0.0), point(1.0, 0.0), point(0.0, 1.0)],
            4
        ),
        Err(ContourEditError::Degenerate)
    ));
    assert_eq!(short_grid.indices.as_slice(), [7, 7, 7].as_slice());
}

#[test]
fn a_non_finite_vertex_writes_nothing() {
    let mut grid = grid(3, 3, vec![7; 9]);
    assert!(matches!(
        edit_labelmap_with_contour(
            &mut grid,
            &[point(0.0, f64::NAN), point(1.0, 0.0), point(0.0, 1.0)],
            4
        ),
        Err(ContourEditError::Frame)
    ));
    assert_eq!(grid.indices.as_slice(), [7; 9].as_slice());
}

#[test]
fn an_overflowing_edge_writes_nothing() {
    let mut grid = grid(1, 1, vec![7]);
    assert!(matches!(
        edit_labelmap_with_contour(
            &mut grid,
            &[point(1e308, -1.0), point(-1e308, 1.0), point(0.0, 2.0)],
            4
        ),
        Err(ContourEditError::Frame)
    ));
    assert_eq!(grid.indices.as_slice(), [7].as_slice());
}

#[test]
fn a_repeated_closing_point_paints_the_same_samples() {
    let mut polygon = rectangle();
    polygon.push(point(1.0, 0.0));
    let mut grid = grid(5, 3, vec![0; 15]);
    let coverage = must_edit(&mut grid, &polygon, 4);
    assert_eq!(coverage.covered, 9);
    assert_eq!(
        grid.indices.as_slice(),
        [0, 4, 4, 4, 0, 0, 4, 4, 4, 0, 0, 4, 4, 4, 0].as_slice()
    );
}

#[test]
fn a_second_edit_changes_nothing() {
    let mut grid = grid(5, 3, vec![0; 15]);
    must_edit(&mut grid, &rectangle(), 4);
    let coverage = must_edit(&mut grid, &rectangle(), 4);
    assert_eq!(coverage.covered, 9);
    assert_eq!(coverage.changed, 0);
}

#[test]
fn the_presenter_colours_an_inside_sample_and_clears_an_outer_one() {
    let mut grid = grid(5, 3, vec![0; 15]);
    must_edit(&mut grid, &rectangle(), 4);
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
    let mut out = vec![7_u8; 60];
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
