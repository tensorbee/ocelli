//! Hand-computed fixtures for the local rules in `.claude/plans/F-070-design.md`,
//! `.claude/plans/F-073-design.md`, `.claude/plans/F-074-design.md` and
//! `.claude/plans/F-076-design.md`.
//!
//! The F-070 numbers are the Euclidean results of that plan. The F-073
//! numbers are that plan's own rules. One finite point is degenerate for
//! `length` and is 0 for `polyline_length`. Height of `(0,0,1)` and `(5,5,4)` is 3,
//! not `sqrt(59)`. Cobb of the x axis and `(1,1,0)` is `PI/4` because the
//! absolute dot is used. They are not a second call into the function under
//! test. A constant segment parameter of 0.5 fails the `(1, 3, 0)` projection.

use ocelli_core::{Canvas, Pt, World};
use ocelli_geom::{
    GeomError, angle_at, bidirectional, circle, cobb, distance, ellipse, height, hit_segment,
    length, polygon_area, polyline_length, project_onto_plane, project_onto_segment, rectangle,
    spline_point,
};

fn world(x: f64, y: f64, z: f64) -> Pt<World> {
    Pt::new(x, y, z)
}

fn bits(value: f64) -> u64 {
    value.to_bits()
}

#[test]
fn distance_is_euclidean() {
    assert_eq!(
        distance(world(0.0, 0.0, 0.0), world(3.0, 4.0, 0.0)).map(bits),
        Ok(bits(5.0))
    );
    assert_eq!(
        distance(world(1.0, 2.0, 3.0), world(1.0, 2.0, 3.0)).map(bits),
        Ok(bits(0.0))
    );
}

#[test]
fn the_same_function_is_instantiated_for_canvas() {
    let start = Pt::<Canvas>::new(0.0, 0.0, 0.0);
    let end = Pt::<Canvas>::new(3.0, 4.0, 0.0);
    assert_eq!(distance(start, end).map(bits), Ok(bits(5.0)));
}

#[test]
fn polyline_length_sums_segments() {
    let points = [
        world(0.0, 0.0, 0.0),
        world(3.0, 4.0, 0.0),
        world(3.0, 4.0, 12.0),
    ];
    assert_eq!(polyline_length(&points).map(bits), Ok(bits(17.0)));
    assert_eq!(
        polyline_length(&[world(1.0, 2.0, 3.0)]).map(bits),
        Ok(bits(0.0))
    );
    assert_eq!(polyline_length::<World>(&[]).map(bits), Ok(bits(0.0)));
    assert_eq!(
        polyline_length(&[
            world(0.0, 0.0, 0.0),
            world(3.0, 4.0, 0.0),
            world(f64::NAN, 0.0, 0.0)
        ]),
        Err(GeomError::Frame)
    );
}

#[test]
fn angle_is_unsigned_and_uses_the_cross_and_the_dot() {
    let origin = world(0.0, 0.0, 0.0);
    assert_eq!(
        angle_at(world(1.0, 0.0, 0.0), origin, world(0.0, 1.0, 0.0)).map(bits),
        Ok(bits(std::f64::consts::FRAC_PI_2))
    );
    assert_eq!(
        angle_at(world(1.0, 0.0, 0.0), origin, world(-1.0, 0.0, 0.0)).map(bits),
        Ok(bits(std::f64::consts::PI))
    );
    assert_eq!(
        angle_at(world(1.0, 0.0, 0.0), origin, world(1.0, 0.0, 0.0)).map(bits),
        Ok(bits(0.0))
    );
    assert_eq!(
        angle_at(origin, origin, world(1.0, 0.0, 0.0)),
        Err(GeomError::Degenerate)
    );
}

#[test]
fn a_forty_five_degree_arm_is_pi_over_four() {
    assert_eq!(
        angle_at(
            world(1.0, 0.0, 0.0),
            world(0.0, 0.0, 0.0),
            world(1.0, 1.0, 0.0)
        )
        .map(bits),
        Ok(bits(std::f64::consts::FRAC_PI_4))
    );
}

#[test]
fn projection_uses_t_and_clamps() {
    let start = world(0.0, 0.0, 0.0);
    let end = world(4.0, 0.0, 0.0);
    let got = project_onto_segment(world(1.0, 3.0, 0.0), start, end);
    assert_eq!(
        got.map(|item| (
            bits(item.t),
            bits(item.point.x),
            bits(item.point.y),
            bits(item.point.z),
            bits(item.distance),
        )),
        Ok((bits(0.25), bits(1.0), bits(0.0), bits(0.0), bits(3.0)))
    );

    let before = project_onto_segment(world(-2.0, 0.0, 0.0), start, end);
    assert_eq!(
        before.map(|item| (bits(item.t), bits(item.point.x), bits(item.distance))),
        Ok((bits(0.0), bits(0.0), bits(2.0)))
    );
    let after = project_onto_segment(world(6.0, 0.0, 0.0), start, end);
    assert_eq!(
        after.map(|item| (bits(item.t), bits(item.point.x), bits(item.distance))),
        Ok((bits(1.0), bits(4.0), bits(2.0)))
    );
}

#[test]
fn only_an_exact_zero_segment_is_degenerate() {
    let start = world(0.0, 0.0, 0.0);
    assert_eq!(
        project_onto_segment(start, start, start).map(|_| ()),
        Err(GeomError::Degenerate)
    );
    let short = project_onto_segment(world(0.0, 1.0, 0.0), start, world(1e-12, 0.0, 0.0));
    assert_eq!(
        short.map(|item| (bits(item.t), bits(item.distance))),
        Ok((bits(0.0), bits(1.0)))
    );
}

#[test]
fn a_hit_includes_the_tolerance_boundary() {
    let start = world(0.0, 0.0, 0.0);
    let end = world(4.0, 0.0, 0.0);
    let off = world(1.0, 3.0, 0.0);
    assert_eq!(hit_segment(off, start, end, 3.0), Ok(true));
    assert_eq!(hit_segment(off, start, end, 2.0), Ok(false));
    assert_eq!(hit_segment(world(2.0, 0.0, 0.0), start, end, 0.0), Ok(true));
    assert_eq!(hit_segment(off, start, end, 0.0), Ok(false));
}

#[test]
fn the_tolerance_sign_is_checked_first() {
    let origin = world(0.0, 0.0, 0.0);
    assert_eq!(
        hit_segment(origin, origin, origin, -1.0),
        Err(GeomError::Frame)
    );
    assert_eq!(
        hit_segment(origin, origin, world(1.0, 0.0, 0.0), f64::NAN),
        Err(GeomError::Frame)
    );
}

#[test]
fn non_finite_inputs_and_overflow_do_not_become_a_length() {
    assert_eq!(
        distance(world(f64::NAN, 0.0, 0.0), world(1.0, 0.0, 0.0)),
        Err(GeomError::Frame)
    );
    assert_eq!(
        distance(world(f64::INFINITY, 0.0, 0.0), world(0.0, 0.0, 0.0)),
        Err(GeomError::Frame)
    );
    assert_eq!(
        distance(world(f64::MAX, 0.0, 0.0), world(f64::MIN, 0.0, 0.0)),
        Err(GeomError::Frame)
    );
}

#[test]
fn the_plane_normal_is_scaled_by_its_length() {
    let origin = world(0.0, 0.0, 0.0);
    let axial = project_onto_plane(world(1.0, 2.0, 3.0), origin, world(0.0, 0.0, 2.0));
    assert_eq!(
        axial.map(|point| (bits(point.x), bits(point.y), bits(point.z))),
        Ok((bits(1.0), bits(2.0), bits(0.0)))
    );

    let oblique = project_onto_plane(world(1.0, 0.0, 5.0), origin, world(1.0, 1.0, 0.0));
    let reported = oblique.map(|foot| (foot.x, foot.y, foot.z));
    // Assertion bound for this irrational foot. The un-normalised subtraction
    // misses by half a unit, so 1e-12 does not hide that mistake. This is
    // not a section 25.1 tolerance.
    const BOUND: f64 = 1e-12;
    assert!(
        matches!(
            reported,
            Ok((x, y, z))
                if (x - 0.5).abs() < BOUND
                    && (y + 0.5).abs() < BOUND
                    && (z - 5.0).abs() < BOUND
        ),
        "{reported:?}"
    );
}

#[test]
fn a_zero_normal_is_not_a_plane() {
    let origin = world(0.0, 0.0, 0.0);
    assert_eq!(
        project_onto_plane(world(1.0, 2.0, 3.0), origin, origin),
        Err(GeomError::Degenerate)
    );
    assert_eq!(
        project_onto_plane(world(1.0, 2.0, 3.0), origin, world(f64::NAN, 0.0, 0.0)),
        Err(GeomError::Frame)
    );
}

#[test]
fn the_geom_numbers_stay_in_range() {
    assert_eq!(GeomError::Frame.number(), 400);
    assert_eq!(GeomError::Degenerate.number(), 401);
}

// F-073 fixtures from `.claude/plans/F-073-design.md`.
// Length of (0, 0, 0) to (3, 4, 0) is 5 because 3*3 + 4*4 = 25 and its
// square root is 5. Height of z = 1 and z = 4 is abs(4 - 1) = 3. Cobb of
// (1, 0, 0) and (1, 1, 0) has dot 1 and cross length 1, so atan2(1, 1) is
// PI/4. Reversing the second segment makes the signed dot -1. The absolute
// dot keeps the same acute angle. Lengths 3 and 4 keep 4 as the longer one.

#[test]
fn length_of_a_three_four_five_segment_is_five() {
    let points = [world(0.0, 0.0, 0.0), world(3.0, 4.0, 0.0)];
    assert_eq!(length(&points).map(bits), Ok(bits(5.0)));
}

#[test]
fn a_single_point_is_not_a_length() {
    assert_eq!(length(&[world(0.0, 0.0, 0.0)]), Err(GeomError::Degenerate));
    assert_eq!(length::<World>(&[]), Err(GeomError::Degenerate));
}

#[test]
fn height_is_the_absolute_z_difference() {
    // The Euclidean length of this pair is not 3. Height ignores x and y.
    assert_eq!(
        height(world(0.0, 0.0, 1.0), world(5.0, 5.0, 4.0)).map(bits),
        Ok(bits(3.0))
    );
    assert_eq!(
        height(world(0.0, 0.0, 4.0), world(0.0, 0.0, 1.0)).map(bits),
        Ok(bits(3.0))
    );
    assert_eq!(
        height(world(f64::NAN, 0.0, 1.0), world(0.0, 0.0, 4.0)),
        Err(GeomError::Frame)
    );
}

#[test]
fn a_right_angle_is_half_pi() {
    assert_eq!(
        angle_at(
            world(1.0, 0.0, 0.0),
            world(0.0, 0.0, 0.0),
            world(0.0, 1.0, 0.0)
        )
        .map(bits),
        Ok(bits(std::f64::consts::FRAC_PI_2))
    );
}

#[test]
fn a_zero_arm_is_degenerate() {
    let origin = world(0.0, 0.0, 0.0);
    assert_eq!(
        angle_at(origin, origin, world(1.0, 0.0, 0.0)),
        Err(GeomError::Degenerate)
    );
}

#[test]
fn cobb_is_acute_and_ignores_direction() {
    let forward = cobb(
        world(0.0, 0.0, 0.0),
        world(1.0, 0.0, 0.0),
        world(0.0, 0.0, 0.0),
        world(1.0, 1.0, 0.0),
    )
    .map(bits);
    assert_eq!(forward, Ok(bits(std::f64::consts::FRAC_PI_4)));
    let reversed = cobb(
        world(0.0, 0.0, 0.0),
        world(1.0, 0.0, 0.0),
        world(1.0, 1.0, 0.0),
        world(0.0, 0.0, 0.0),
    )
    .map(bits);
    assert_eq!(reversed, Ok(bits(std::f64::consts::FRAC_PI_4)));
    assert_eq!(
        cobb(
            world(0.0, 0.0, 0.0),
            world(0.0, 0.0, 0.0),
            world(0.0, 0.0, 0.0),
            world(1.0, 0.0, 0.0),
        ),
        Err(GeomError::Degenerate)
    );
}

#[test]
fn bidirectional_keeps_the_longer_length() {
    let pair = bidirectional(
        world(0.0, 0.0, 0.0),
        world(3.0, 0.0, 0.0),
        world(0.0, 0.0, 0.0),
        world(0.0, 4.0, 0.0),
    );
    assert_eq!(
        pair.map(|item| (bits(item.length), bits(item.width))),
        Ok((bits(4.0), bits(3.0)))
    );
    let swapped = bidirectional(
        world(0.0, 0.0, 0.0),
        world(0.0, 4.0, 0.0),
        world(0.0, 0.0, 0.0),
        world(3.0, 0.0, 0.0),
    );
    assert_eq!(
        swapped.map(|item| (bits(item.length), bits(item.width))),
        Ok((bits(4.0), bits(3.0)))
    );
    let same = bidirectional(
        world(0.0, 0.0, 0.0),
        world(0.0, 0.0, 0.0),
        world(1.0, 0.0, 0.0),
        world(1.0, 0.0, 0.0),
    );
    assert_eq!(
        same.map(|item| (bits(item.length), bits(item.width))),
        Ok((bits(0.0), bits(0.0)))
    );
}

// F-074 fixtures from `.claude/plans/F-074-design.md`.
// Width is abs(2 - 0) = 2. Height is abs(3 - 0) = 3. Area is 2 * 3 = 6.
// The z difference 9 is not a factor. Ellipse radii 2 and 3 have absolute
// product 6, so the area is PI * 6. A zero radius is not area 0. Circle
// radius is sqrt(9 + 16) = 5, and the area is PI * 25.

#[test]
fn rectangle_ignores_z() {
    let shape = rectangle(world(0.0, 0.0, 0.0), world(2.0, 3.0, 9.0));
    assert_eq!(
        shape.map(|item| (bits(item.width), bits(item.height), bits(item.area))),
        Ok((bits(2.0), bits(3.0), bits(6.0)))
    );
    let reversed = rectangle(world(2.0, 3.0, 9.0), world(0.0, 0.0, 0.0));
    assert_eq!(
        reversed.map(|item| (bits(item.width), bits(item.height), bits(item.area))),
        Ok((bits(2.0), bits(3.0), bits(6.0)))
    );
    assert_eq!(
        rectangle(world(f64::NAN, 0.0, 0.0), world(2.0, 3.0, 9.0)),
        Err(GeomError::Frame)
    );
}

#[test]
fn ellipse_area_is_pi_times_the_radii() {
    let area = std::f64::consts::PI * 6.0;
    assert_eq!(ellipse(2.0, 3.0).map(bits), Ok(bits(area)));
    // Opposite signs. A signed product is the negation of PI * 6.
    assert_eq!(ellipse(-2.0, 3.0).map(bits), Ok(bits(area)));
    assert_eq!(ellipse(2.0, -3.0).map(bits), Ok(bits(area)));
    assert_eq!(ellipse(f64::NAN, 3.0), Err(GeomError::Frame));
}

#[test]
fn a_zero_radius_is_degenerate() {
    assert_eq!(ellipse(0.0, 3.0), Err(GeomError::Degenerate));
    assert_eq!(ellipse(2.0, 0.0), Err(GeomError::Degenerate));
    assert_eq!(ellipse(-0.0, 3.0), Err(GeomError::Degenerate));
}

#[test]
fn circle_radius_is_the_euclidean_length() {
    let shape = circle(world(0.0, 0.0, 0.0), world(3.0, 4.0, 0.0));
    assert_eq!(
        shape.map(|item| (bits(item.radius), bits(item.area))),
        Ok((bits(5.0), bits(std::f64::consts::PI * 25.0)))
    );
    assert_eq!(
        circle(world(1.0, 2.0, 3.0), world(1.0, 2.0, 3.0)),
        Err(GeomError::Degenerate)
    );
    assert_eq!(
        circle(world(f64::NAN, 0.0, 0.0), world(1.0, 0.0, 0.0)),
        Err(GeomError::Frame)
    );
}

// F-076 fixtures from `.claude/plans/F-076-design.md`.
// Unit square cross terms are 0 + 1 + 1 + 0 = 2, and half of that is 1.
// The same square shifted to (1, 1) still has area 1, and dropping the
// closing edge leaves 1.5. Spline points (0, 0, 0), (1, 0, 0), (2, 1, 0),
// segment 0, t = 0.5. Previous ghost is the first point and the next point
// is the third. The basis sum is (0.875, -0.125, 0), and half of that is
// (0.4375, -0.0625, 0). A polyline midpoint would be (0.5, 0, 0).

#[test]
fn the_unit_square_has_area_one() {
    let square = [
        world(0.0, 0.0, 0.0),
        world(1.0, 0.0, 0.0),
        world(1.0, 1.0, 0.0),
        world(0.0, 1.0, 0.0),
    ];
    assert_eq!(polygon_area(&square).map(bits), Ok(bits(1.0)));
    let reversed = [
        world(0.0, 0.0, 0.0),
        world(0.0, 1.0, 0.0),
        world(1.0, 1.0, 0.0),
        world(1.0, 0.0, 0.0),
    ];
    assert_eq!(polygon_area(&reversed).map(bits), Ok(bits(1.0)));
    let shifted = [
        world(1.0, 1.0, 9.0),
        world(2.0, 1.0, 9.0),
        world(2.0, 2.0, 9.0),
        world(1.0, 2.0, 9.0),
    ];
    assert_eq!(polygon_area(&shifted).map(bits), Ok(bits(1.0)));
    assert_eq!(
        polygon_area(&[
            world(0.0, 0.0, 0.0),
            world(1.0, 0.0, 0.0),
            world(f64::NAN, 1.0, 0.0)
        ]),
        Err(GeomError::Frame)
    );
}

#[test]
fn two_points_have_no_area() {
    assert_eq!(
        polygon_area(&[world(0.0, 0.0, 0.0), world(1.0, 0.0, 0.0)]),
        Err(GeomError::Degenerate)
    );
    assert_eq!(polygon_area::<World>(&[]), Err(GeomError::Degenerate));
}

#[test]
fn the_first_spline_midpoint_is_hand_computed() {
    let points = [
        world(0.0, 0.0, 0.0),
        world(1.0, 0.0, 0.0),
        world(2.0, 1.0, 0.0),
    ];
    let got = spline_point(&points, 0, 0.5);
    assert_eq!(
        got.map(|point| (bits(point.x), bits(point.y), bits(point.z))),
        Ok((bits(0.4375), bits(-0.0625), bits(0.0)))
    );
}

#[test]
fn the_spline_passes_through_the_knots() {
    let points = [
        world(0.0, 0.0, 0.0),
        world(1.0, 0.0, 0.0),
        world(2.0, 1.0, 0.0),
    ];
    let start = spline_point(&points, 0, 0.0);
    assert_eq!(
        start.map(|point| (bits(point.x), bits(point.y), bits(point.z))),
        Ok((bits(0.0), bits(0.0), bits(0.0)))
    );
    let end = spline_point(&points, 0, 1.0);
    assert_eq!(
        end.map(|point| (bits(point.x), bits(point.y), bits(point.z))),
        Ok((bits(1.0), bits(0.0), bits(0.0)))
    );
}

#[test]
fn a_parameter_outside_the_segment_is_degenerate() {
    let points = [
        world(0.0, 0.0, 0.0),
        world(1.0, 0.0, 0.0),
        world(2.0, 1.0, 0.0),
    ];
    assert_eq!(spline_point(&points, 0, 1.5), Err(GeomError::Degenerate));
    assert_eq!(spline_point(&points, 2, 0.5), Err(GeomError::Degenerate));
    assert_eq!(
        spline_point(&points, 0, f64::NAN),
        Err(GeomError::Degenerate)
    );
}
