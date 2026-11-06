//! Pixel-centre inclusion fixtures for the local rule in
//! `.claude/plans/F-211-design.md`, with the F-075 statistics on top.
//!
//! The grid is 5 by 5 with `v(c, r) = c + 10 * r`, column `c` and row `r`
//! as zero-based index points, the sample centres. The statistics are the
//! F-075 population form, so the standard deviation divides by the count.
//! Every expected number below is worked by hand in the test comment.

use ocelli_core::{Index, Pt};
use ocelli_geom::{
    GeomError, IndexBounds, RegionShape, RoiStats, ellipse, polygon_area, polygon_crosses_itself,
    rectangle, region_bounds, region_contains, roi_statistics,
};

const SIDE: u32 = 5;

fn at(x: f64, y: f64) -> Pt<Index> {
    Pt::new(x, y, 0.0)
}

fn value(column: u32, row: u32) -> f64 {
    f64::from(column) + 10.0 * f64::from(row)
}

/// The values of every grid sample the shape contains, walking only its bounds.
fn inside(shape: &RegionShape<'_>) -> Result<Vec<f64>, String> {
    let bounds = region_bounds(shape, SIDE, SIDE)
        .map_err(|error| format!("bounds refused: {error:?}"))?
        .ok_or_else(|| "the shape missed the grid".to_owned())?;
    let mut samples = Vec::new();
    for row in bounds.first_row..=bounds.last_row {
        for column in bounds.first_column..=bounds.last_column {
            if region_contains(shape, column, row)
                .map_err(|error| format!("contains refused: {error:?}"))?
            {
                samples.push(value(column, row));
            }
        }
    }
    Ok(samples)
}

/// Every sample of the whole grid, to show the bounds walk loses none.
fn inside_whole_grid(shape: &RegionShape<'_>) -> Result<Vec<f64>, String> {
    let mut samples = Vec::new();
    for row in 0..SIDE {
        for column in 0..SIDE {
            if region_contains(shape, column, row)
                .map_err(|error| format!("contains refused: {error:?}"))?
            {
                samples.push(value(column, row));
            }
        }
    }
    Ok(samples)
}

fn stats(samples: &[f64], area: f64) -> Result<RoiStats, String> {
    roi_statistics(samples, area).map_err(|error| format!("statistics refused: {error:?}"))
}

#[test]
fn rectangle_uses_pixel_centres() -> Result<(), String> {
    // AC1. Page corners (1, 1) and (3, 3) minus the half pixel are index
    // (0.5, 0.5) and (2.5, 2.5). Centres 1 and 2 lie between them on both
    // axes, so the samples are 11, 12, 21 and 22. Sum 66, mean 16.5.
    // Deviations -5.5, -4.5, 4.5, 5.5, squares 30.25 + 20.25 + 20.25 +
    // 30.25 = 101, divided by 4 is 25.25, so SD is sqrt(25.25). The area of
    // the drawn shape is 2 * 2 = 4 px². Without the shift, (1, 1) to (3, 3)
    // holds nine samples 11..33 with mean 22.
    let corner = at(0.5, 0.5);
    let opposite = at(2.5, 2.5);
    let shape = RegionShape::Rectangle { corner, opposite };
    let samples = inside(&shape)?;
    if samples != [11.0, 12.0, 21.0, 22.0] {
        return Err(format!("the rectangle held {samples:?}"));
    }
    if samples != inside_whole_grid(&shape)? {
        return Err("the bounds lost a sample".to_owned());
    }
    let area = rectangle(corner, opposite)
        .map_err(|error| format!("area refused: {error:?}"))?
        .area;
    let got = stats(&samples, area)?;
    let want = RoiStats {
        count: 4,
        mean: 16.5,
        sd: 25.25_f64.sqrt(),
        min: 11.0,
        max: 22.0,
        area: 4.0,
    };
    if got != want || got.sd.to_bits() != 5.024_937_810_560_445_f64.to_bits() {
        return Err(format!("statistics were {got:?}"));
    }
    Ok(())
}

#[test]
fn rectangle_includes_a_centre_on_its_edge() -> Result<(), String> {
    // A box with integer index bounds (1, 1) to (2, 3) has centres on its
    // edges. Inclusive bounds keep columns 1 and 2 of rows 1, 2 and 3:
    // 11, 12, 21, 22, 31, 32. A strict rule would keep none.
    let shape = RegionShape::Rectangle {
        corner: at(2.0, 3.0),
        opposite: at(1.0, 1.0),
    };
    let samples = inside(&shape)?;
    if samples != [11.0, 12.0, 21.0, 22.0, 31.0, 32.0] {
        return Err(format!("the edge box held {samples:?}"));
    }
    Ok(())
}

#[test]
fn ellipse_excludes_the_four_corners() -> Result<(), String> {
    // AC2. Page box (0, 0) to (5, 5) is index (-0.5, -0.5) to (4.5, 4.5),
    // centre (2, 2), radii 2.5. A sample is inside when dc² + dr² <= 6.25.
    // The four corners have 4 + 4 = 8 > 6.25 and every other sample has at
    // most 4 + 1 = 5, so 21 of 25 stay. The whole grid sums to 550
    // (columns 25 * 2 = 50, rows 10 * 25 * 2 = 500). The corners 0, 4, 40
    // and 44 sum to 88, so the sum is 462 and the mean 462 / 21 = 22. The
    // squared deviations from 22 sum to 3434, so SD is sqrt(3434 / 21).
    // Without the corners the minimum is v(1, 0) = 1 and the maximum
    // v(3, 4) = 43. The area is PI * 2.5 * 2.5 = PI * 6.25 px².
    let shape = RegionShape::Ellipse {
        center: at(2.0, 2.0),
        rx: 2.5,
        ry: 2.5,
    };
    let samples = inside(&shape)?;
    if samples.len() != 21 || samples.contains(&0.0) || samples.contains(&44.0) {
        return Err(format!("the ellipse held {samples:?}"));
    }
    if samples != inside_whole_grid(&shape)? {
        return Err("the bounds lost a sample".to_owned());
    }
    let area = ellipse(2.5, 2.5).map_err(|error| format!("area refused: {error:?}"))?;
    let got = stats(&samples, area)?;
    if got.count != 21
        || got.mean.to_bits() != 22.0_f64.to_bits()
        || got.sd.to_bits() != 12.787_642_844_707_914_f64.to_bits()
        || got.min.to_bits() != 1.0_f64.to_bits()
        || got.max.to_bits() != 43.0_f64.to_bits()
        || got.area.to_bits() != 19.634_954_084_936_208_f64.to_bits()
    {
        return Err(format!("statistics were {got:?}"));
    }
    Ok(())
}

#[test]
fn ellipse_includes_its_boundary() -> Result<(), String> {
    // Centre (2, 2), radii 2. dc² + dr² <= 4 holds for the centre, the four
    // neighbours at 1, the four diagonals at 2, and the four samples at
    // distance exactly 2: (0, 2), (4, 2), (2, 0) and (2, 4). That is 13.
    // A strict rule drops those four and keeps 9.
    let shape = RegionShape::Ellipse {
        center: at(2.0, 2.0),
        rx: 2.0,
        ry: 2.0,
    };
    let samples = inside(&shape)?;
    for edge in [20.0, 24.0, 2.0, 42.0] {
        if !samples.contains(&edge) {
            return Err(format!("the boundary sample {edge} was dropped"));
        }
    }
    if samples.len() != 13 {
        return Err(format!("the circle held {} samples", samples.len()));
    }
    // Radii 2 across and 1 down, centre (2, 2). (c - 2)² / 4 + (r - 2)² <= 1.
    // Row 2 keeps columns 0 to 4. Rows 1 and 3 keep column 2 only, because
    // column 1 gives 1 / 4 + 1 > 1. That is 5 + 1 + 1 = 7, values 20 to 24,
    // 12 and 32.
    let flat = RegionShape::Ellipse {
        center: at(2.0, 2.0),
        rx: 2.0,
        ry: 1.0,
    };
    let samples = inside(&flat)?;
    if samples != [12.0, 20.0, 21.0, 22.0, 23.0, 24.0, 32.0] {
        return Err(format!("the flat ellipse held {samples:?}"));
    }
    Ok(())
}

#[test]
fn polygon_includes_its_edges() -> Result<(), String> {
    // AC3. Triangle (0, 0), (4, 0), (0, 4). A centre is inside when
    // c + r <= 4, which is 5 + 4 + 3 + 2 + 1 = 15 samples. Their values sum
    // to 220: the columns contribute sum over r of sum_{c=0}^{4-r} c
    // = 10 + 6 + 3 + 1 + 0 = 20, and the rows 10 * (0*5 + 1*4 + 2*3 + 3*2
    // + 4*1) = 200. Mean 220 / 15. Minimum v(0, 0) = 0, maximum
    // v(0, 4) = 40. The 12 samples on the three edges are inside, and a
    // strict interior rule keeps only (1, 1), (1, 2) and (2, 1). The
    // shoelace area is 4 * 4 / 2 = 8 px².
    let points = [at(0.0, 0.0), at(4.0, 0.0), at(0.0, 4.0)];
    let shape = RegionShape::Polygon { points: &points };
    let samples = inside(&shape)?;
    if samples.len() != 15 {
        return Err(format!("the triangle held {samples:?}"));
    }
    if samples != inside_whole_grid(&shape)? {
        return Err("the bounds lost a sample".to_owned());
    }
    let area = polygon_area(&points).map_err(|error| format!("area refused: {error:?}"))?;
    let got = stats(&samples, area)?;
    let sum: f64 = samples.iter().sum();
    if sum.to_bits() != 220.0_f64.to_bits()
        || got.count != 15
        || got.mean.to_bits() != 14.666_666_666_666_666_f64.to_bits()
        || got.sd.to_bits() != 11.897_712_198_383_164_f64.to_bits()
        || got.min.to_bits() != 0.0_f64.to_bits()
        || got.max.to_bits() != 40.0_f64.to_bits()
        || got.area.to_bits() != 8.0_f64.to_bits()
    {
        return Err(format!("statistics were {got:?}, sum {sum}"));
    }
    Ok(())
}

#[test]
fn bounds_are_clipped_to_the_grid() -> Result<(), String> {
    // A rectangle from index (-3, 1.5) to (2.5, 9) covers columns 0 to 2
    // and rows 2 to 4 of the 5 by 5 grid once clipped.
    let shape = RegionShape::Rectangle {
        corner: at(-3.0, 1.5),
        opposite: at(2.5, 9.0),
    };
    let got = region_bounds(&shape, SIDE, SIDE).map_err(|error| format!("{error:?}"))?;
    let want = IndexBounds {
        first_column: 0,
        last_column: 2,
        first_row: 2,
        last_row: 4,
    };
    if got != Some(want) {
        return Err(format!("bounds were {got:?}"));
    }
    Ok(())
}

#[test]
fn region_refusals() -> Result<(), String> {
    // AC5. Wholly outside the grid is no bounds, never an empty mean.
    let outside = RegionShape::Rectangle {
        corner: at(5.5, 0.0),
        opposite: at(9.0, 4.0),
    };
    if region_bounds(&outside, SIDE, SIDE) != Ok(None) {
        return Err("a rectangle right of the grid had bounds".to_owned());
    }
    let between = RegionShape::Rectangle {
        corner: at(1.2, 1.2),
        opposite: at(1.8, 1.8),
    };
    if region_bounds(&between, SIDE, SIDE) != Ok(None) {
        return Err("a box between centres had bounds".to_owned());
    }
    let far = [at(-9.0, -9.0), at(-8.0, -9.0), at(-9.0, -8.0)];
    if region_bounds(&RegionShape::Polygon { points: &far }, SIDE, SIDE) != Ok(None) {
        return Err("a triangle above the grid had bounds".to_owned());
    }
    let far_ellipse = RegionShape::Ellipse {
        center: at(-20.0, 2.0),
        rx: 2.0,
        ry: 2.0,
    };
    if region_bounds(&far_ellipse, SIDE, SIDE) != Ok(None) {
        return Err("an ellipse left of the grid had bounds".to_owned());
    }
    if region_bounds(&between, 0, SIDE) != Ok(None) {
        return Err("an empty grid had bounds".to_owned());
    }
    // A zero radius is degenerate, either sign.
    for (rx, ry) in [(0.0, 2.0), (2.0, -0.0)] {
        let flat = RegionShape::Ellipse {
            center: at(2.0, 2.0),
            rx,
            ry,
        };
        if region_contains(&flat, 2, 2) != Err(GeomError::Degenerate)
            || region_bounds(&flat, SIDE, SIDE) != Err(GeomError::Degenerate)
        {
            return Err(format!("radii {rx} and {ry} were not degenerate"));
        }
    }
    // Fewer than three polygon points is degenerate.
    let two = [at(0.0, 0.0), at(4.0, 0.0)];
    let short = RegionShape::Polygon { points: &two };
    if region_contains(&short, 1, 0) != Err(GeomError::Degenerate)
        || region_bounds(&short, SIDE, SIDE) != Err(GeomError::Degenerate)
    {
        return Err("a two-point polygon was not degenerate".to_owned());
    }
    // A NaN or infinite input is `Frame`.
    let nan = [at(0.0, 0.0), at(f64::NAN, 0.0), at(0.0, 4.0)];
    let bad_polygon = RegionShape::Polygon { points: &nan };
    let bad_rectangle = RegionShape::Rectangle {
        corner: at(0.0, f64::INFINITY),
        opposite: at(1.0, 1.0),
    };
    let bad_ellipse = RegionShape::Ellipse {
        center: at(f64::NAN, 2.0),
        rx: 1.0,
        ry: 1.0,
    };
    let bad_radius = RegionShape::Ellipse {
        center: at(2.0, 2.0),
        rx: f64::INFINITY,
        ry: 1.0,
    };
    for (name, shape) in [
        ("polygon", bad_polygon),
        ("rectangle", bad_rectangle),
        ("ellipse centre", bad_ellipse),
        ("ellipse radius", bad_radius),
    ] {
        if region_contains(&shape, 1, 1) != Err(GeomError::Frame)
            || region_bounds(&shape, SIDE, SIDE) != Err(GeomError::Frame)
        {
            return Err(format!("a non-finite {name} was not Frame"));
        }
    }
    Ok(())
}

#[test]
fn a_figure_of_eight_crosses_itself() -> Result<(), String> {
    // AC13. The ring (0, 0), (4, 4), (4, 0), (0, 4) has edges (0, 0) to
    // (4, 4) and (4, 0) to (0, 4), which are not adjacent and cross at
    // (2, 2), strictly inside both. Its shoelace terms x_i * y_(i+1) -
    // x_(i+1) * y_i are 0, 0 - 16, 16 - 0 and 0, so the area is 0 while the
    // even-odd rule still keeps samples, the disagreement the plan refuses.
    let eight = [at(0.0, 0.0), at(4.0, 4.0), at(4.0, 0.0), at(0.0, 4.0)];
    if polygon_crosses_itself(&eight) != Ok(true) {
        return Err("the figure of eight was not refused".to_owned());
    }
    // A convex square and the AC3 triangle do not cross.
    let square = [at(0.0, 0.0), at(4.0, 0.0), at(4.0, 4.0), at(0.0, 4.0)];
    let triangle = [at(0.0, 0.0), at(4.0, 0.0), at(0.0, 4.0)];
    for (name, ring) in [("square", &square[..]), ("triangle", &triangle[..])] {
        if polygon_crosses_itself(ring) != Ok(false) {
            return Err(format!("the {name} was called crossing"));
        }
    }
    // F-211 review pass 1, D1. Any contact between two edges that are not
    // neighbours is a crossing, and so is a pair of neighbours that fold
    // back over each other. Only the shared vertex of two neighbours may
    // touch.
    let refused: [(&str, &[Pt<Index>]); 5] = [
        // Edge (4, 0) to (0, 4) passes through vertex (2, 2), so one
        // orientation is 0 and the ring is still a bow-tie.
        (
            "an edge through a vertex",
            &[
                at(0.0, 0.0),
                at(2.0, 2.0),
                at(4.0, 4.0),
                at(4.0, 0.0),
                at(0.0, 4.0),
            ],
        ),
        // The pinched bow meets itself at (2, 2), where edges (0, 0) to
        // (2, 2) and (4, 4) to (2, 2) touch and are not neighbours.
        (
            "a touch at a repeated vertex",
            &[
                at(0.0, 0.0),
                at(2.0, 2.0),
                at(4.0, 0.0),
                at(4.0, 4.0),
                at(2.0, 2.0),
                at(0.0, 4.0),
            ],
        ),
        // Edge (4, 0) to (2, 0) lies along edge (0, 0) to (6, 0) and the two
        // are not neighbours: a collinear overlap.
        (
            "a collinear overlap",
            &[
                at(0.0, 0.0),
                at(6.0, 0.0),
                at(6.0, 2.0),
                at(4.0, 0.0),
                at(2.0, 0.0),
                at(2.0, -2.0),
            ],
        ),
        // Neighbours (0, 0) to (4, 0) and (4, 0) to (2, 0) fold back over
        // each other beyond their shared vertex. Edge (2, 0) to (2, 3) also
        // starts on edge (0, 0) to (4, 0), a non-neighbour touch.
        (
            "neighbours that fold back",
            &[at(0.0, 0.0), at(4.0, 0.0), at(2.0, 0.0), at(2.0, 3.0)],
        ),
        // A flat ring of three collinear points has three edges and every
        // pair is neighbours, so only the fold rule can refuse it. Its
        // shoelace area is 0 while the even-odd rule keeps the samples on
        // the line from (0, 0) to (4, 0).
        (
            "a flat ring that folds back",
            &[at(0.0, 0.0), at(4.0, 0.0), at(2.0, 0.0)],
        ),
    ];
    for (name, ring) in refused {
        if polygon_crosses_itself(ring) != Ok(true) {
            return Err(format!("{name} was not refused"));
        }
    }
    // Neighbours that continue in a straight line touch only at the shared
    // vertex: (0, 0), (2, 0), (4, 0), (4, 4), (0, 4) is a simple square.
    let straight = [
        at(0.0, 0.0),
        at(2.0, 0.0),
        at(4.0, 0.0),
        at(4.0, 4.0),
        at(0.0, 4.0),
    ];
    if polygon_crosses_itself(&straight) != Ok(false) {
        return Err("a straight run of neighbours was called crossing".to_owned());
    }
    let two = [at(0.0, 0.0), at(1.0, 0.0)];
    if polygon_crosses_itself(&two) != Err(GeomError::Degenerate) {
        return Err("a two-point ring was not degenerate".to_owned());
    }
    let nan = [at(0.0, 0.0), at(f64::NAN, 0.0), at(0.0, 1.0)];
    if polygon_crosses_itself(&nan) != Err(GeomError::Frame) {
        return Err("a NaN ring was not Frame".to_owned());
    }
    Ok(())
}
