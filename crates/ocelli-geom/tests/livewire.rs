//! Hand-computed fixtures for the local rules in `.claude/plans/F-077-design.md`.
//!
//! The 2 by 2 costs are 1, 100, 1, 1. The route through the cost-100 pixel
//! sums to 101. The route through `(0, 1)` sums to 2. The mask marks only
//! the pixels that route visits.

use ocelli_core::{Pt, World};
use ocelli_geom::{GeomError, livewire_mask, livewire_path};

fn world(x: f64, y: f64, z: f64) -> Pt<World> {
    Pt::new(x, y, z)
}

fn path(costs: &[f64], start: Pt<World>, goal: Pt<World>) -> Vec<Pt<World>> {
    let result = livewire_path(costs, 2, 2, start, goal);
    assert!(result.is_ok());
    let Ok(path) = result else {
        unreachable!("asserted immediately above")
    };
    path
}

#[test]
fn the_cheap_path_follows_the_gradient() {
    let found = path(
        &[1.0, 100.0, 1.0, 1.0],
        world(0.0, 0.0, 9.0),
        world(1.0, 1.0, 4.0),
    );
    assert_eq!(
        found,
        vec![
            world(0.0, 0.0, 0.0),
            world(0.0, 1.0, 0.0),
            world(1.0, 1.0, 0.0)
        ]
    );
}

#[test]
fn the_mask_is_the_visited_pixels() {
    let found = path(
        &[1.0, 100.0, 1.0, 1.0],
        world(0.0, 0.0, 0.0),
        world(1.0, 1.0, 0.0),
    );
    assert_eq!(livewire_mask(2, 2, &found), Ok(vec![1, 0, 1, 1]));
}

#[test]
fn a_seed_is_a_one_pixel_path() {
    let found = path(
        &[1.0, 2.0, 3.0, 4.0],
        world(1.0, 0.0, 0.0),
        world(1.0, 0.0, 0.0),
    );
    assert_eq!(found, vec![world(1.0, 0.0, 0.0)]);
}

#[test]
fn a_negative_zero_seed_is_the_origin() {
    let found = path(
        &[1.0, 1.0, 1.0, 1.0],
        world(-0.0, -0.0, 0.0),
        world(-0.0, -0.0, 0.0),
    );
    assert_eq!(found, vec![world(0.0, 0.0, 0.0)]);
}

#[test]
fn a_negative_zero_cost_is_not_negative() {
    let found = path(
        &[-0.0, 1.0, 1.0, 1.0],
        world(0.0, 0.0, 0.0),
        world(1.0, 0.0, 0.0),
    );
    assert_eq!(found, vec![world(0.0, 0.0, 0.0), world(1.0, 0.0, 0.0)]);
}

#[test]
fn a_negative_cost_is_degenerate() {
    assert_eq!(
        livewire_path::<World>(
            &[1.0, -1.0, 1.0, 1.0],
            2,
            2,
            world(0.0, 0.0, 0.0),
            world(1.0, 1.0, 0.0)
        ),
        Err(GeomError::Degenerate)
    );
    assert_eq!(
        livewire_path::<World>(
            &[1.0, f64::NAN, 1.0, 1.0],
            2,
            2,
            world(0.0, 0.0, 0.0),
            world(1.0, 1.0, 0.0)
        ),
        Err(GeomError::Frame)
    );
}

#[test]
fn an_empty_path_is_not_a_blank_mask() {
    assert_eq!(
        livewire_mask::<World>(2, 2, &[]),
        Err(GeomError::Degenerate)
    );
}

#[test]
fn a_fractional_index_is_degenerate() {
    assert_eq!(
        livewire_path::<World>(
            &[1.0, 1.0, 1.0, 1.0],
            2,
            2,
            world(0.5, 0.0, 0.0),
            world(1.0, 1.0, 0.0)
        ),
        Err(GeomError::Degenerate)
    );
}

#[test]
fn a_tie_keeps_the_lower_index() {
    let found = path(
        &[1.0, 1.0, 1.0, 1.0],
        world(0.0, 0.0, 0.0),
        world(1.0, 1.0, 0.0),
    );
    assert_eq!(
        found,
        vec![
            world(0.0, 0.0, 0.0),
            world(1.0, 0.0, 0.0),
            world(1.0, 1.0, 0.0)
        ]
    );
}
