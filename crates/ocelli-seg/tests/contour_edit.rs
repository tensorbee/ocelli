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
fn an_edge_product_that_underflows_writes_nothing() {
    // Sample (0, 0) is about 1e-170 from every vertex. It is not on an edge,
    // and a ray toward increasing x crosses none, so every stored 9 stays.
    let mut grid = grid(3, 3, vec![9; 9]);
    let polygon = [
        point(1.0e-170, 1.0e-170),
        point(2.0e-170, 3.0e-170),
        point(3.0e-170, 1.0e-170),
    ];
    assert!(matches!(
        edit_labelmap_with_contour(&mut grid, &polygon, 3),
        Err(ContourEditError::Frame)
    ));
    assert_eq!(grid.indices.as_slice(), [9; 9].as_slice());
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
fn an_infinite_squared_length_writes_nothing() {
    // Row 0 sits at x >= 0. The triangle lies at x <= -1, so every stored 0 stays.
    let mut grid = grid(4, 6, vec![0; 24]);
    let polygon = [point(-1.0e200, 0.0), point(-1.0, 0.0), point(-1.0, 5.0)];
    assert!(matches!(
        edit_labelmap_with_contour(&mut grid, &polygon, 3),
        Err(ContourEditError::Frame)
    ));
    assert_eq!(grid.indices.as_slice(), [0; 24].as_slice());
}

#[test]
fn a_flushed_intersection_writes_nothing() {
    // At y = 0 the left edge is just to the right of x = 0.
    // Sample (0, 0) is outside, so a success must not paint it.
    let mut grid = grid(2, 1, vec![0, 0]);
    let polygon = [
        point(0.0, -1.0e-200),
        point(1.0, 1.0e150),
        point(2.0, -1.0e-200),
    ];
    assert!(matches!(
        edit_labelmap_with_contour(&mut grid, &polygon, 3),
        Err(ContourEditError::Frame)
    ));
    assert_eq!(grid.indices.as_slice(), [0, 0].as_slice());
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

#[test]
fn region_matches_the_f085_rule() {
    // F-211 AC4. The triangle (0, 0), (4, 0), (0, 4) painted on a 5 by 5
    // grid covers the samples with c + r <= 4, 15 of them, and
    // `ocelli_geom::region_contains` must accept exactly those samples, so a
    // freeform measured and the same ring painted cover the same pixels.
    // The second ring has a vertex off the grid and a non-integer edge.
    use ocelli_core::{Index, Pt};
    use ocelli_geom::{RegionShape, region_contains};
    let rings: [&[(f64, f64)]; 2] = [
        &[(0.0, 0.0), (4.0, 0.0), (0.0, 4.0)],
        &[(0.5, -1.0), (4.0, 1.5), (3.0, 4.0), (-0.5, 2.5)],
    ];
    for ring in rings {
        let painted: Vec<IndexPoint> = ring.iter().map(|&(x, y)| point(x, y)).collect();
        let measured: Vec<Pt<Index>> = ring.iter().map(|&(x, y)| Pt::new(x, y, 0.0)).collect();
        let mut labels = grid(5, 5, vec![0; 25]);
        let coverage = must_edit(&mut labels, &painted, 1);
        let shape = RegionShape::Polygon { points: &measured };
        let mut accepted = 0_u32;
        for row in 0..5_u32 {
            for column in 0..5_u32 {
                let painted_here = labels
                    .indices
                    .get(usize::try_from(row * 5 + column).unwrap_or(usize::MAX))
                    .copied()
                    == Some(1);
                let measured_here = matches!(region_contains(&shape, column, row), Ok(true));
                assert_eq!(
                    painted_here, measured_here,
                    "sample ({column}, {row}) of {ring:?}"
                );
                accepted += u32::from(measured_here);
            }
        }
        assert_eq!(coverage.covered, accepted);
    }
    let mut triangle = grid(5, 5, vec![0; 25]);
    let coverage = must_edit(
        &mut triangle,
        &[point(0.0, 0.0), point(4.0, 0.0), point(0.0, 4.0)],
        1,
    );
    assert_eq!(coverage.covered, 15);
}
