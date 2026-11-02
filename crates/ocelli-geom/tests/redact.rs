//! Hand-computed fixtures for the local rules in `.claude/plans/F-079-design.md`.
//!
//! `(0, 0, 0)` to `(0, 2, 9)` has image-plane angle `PI / 2`.
//! The squared length is `0 + 4 + 81`.
//! A row-major 2 by 2 buffer `1, 2, 3, 4` with the left column set to 0
//! becomes `0, 2, 0, 4`.

use ocelli_core::{Pt, World};
use ocelli_geom::{Direction, GeomError, Redaction, redact_rect, ultrasound_direction};

fn world(x: f64, y: f64, z: f64) -> Pt<World> {
    Pt::new(x, y, z)
}

fn same_bits(left: &[f64], right: &[f64]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right.iter())
            .all(|(sample, expected)| sample.to_bits() == expected.to_bits())
}

fn direction(start: Pt<World>, end: Pt<World>) -> Direction {
    let result = ultrasound_direction(start, end);
    assert!(result.is_ok());
    let Ok(direction) = result else {
        unreachable!("asserted immediately above")
    };
    direction
}

#[test]
fn positive_y_is_half_pi() {
    let found = direction(world(0.0, 0.0, 0.0), world(0.0, 2.0, 9.0));
    let squared = 0.0_f64 + 2.0 * 2.0 + 9.0 * 9.0;
    assert!(
        found
            .radians
            .total_cmp(&std::f64::consts::FRAC_PI_2)
            .is_eq()
    );
    assert!(found.length.total_cmp(&squared.sqrt()).is_eq());
    assert!(found.length.total_cmp(&2.0).is_ne());
}

#[test]
fn a_zero_direction_is_degenerate() {
    assert_eq!(
        ultrasound_direction(world(1.0, 2.0, 3.0), world(1.0, 2.0, 3.0)),
        Err(GeomError::Degenerate)
    );
    assert_eq!(
        ultrasound_direction(world(0.0, 0.0, 0.0), world(-0.0, -0.0, -0.0)),
        Err(GeomError::Degenerate)
    );
}

#[test]
fn a_non_finite_point_is_frame() {
    assert_eq!(
        ultrasound_direction(world(0.0, 0.0, 0.0), world(1.0, f64::NAN, 0.0)),
        Err(GeomError::Frame)
    );
}

#[test]
fn the_left_column_is_redacted() {
    let mut samples = [1.0, 2.0, 3.0, 4.0];
    assert_eq!(
        redact_rect(&mut samples, 2, 2, 0, 0, 1, 2, 0.0),
        Ok(Redaction {
            samples_written: 2,
            samples_already_fill: 0,
            applied: true,
        })
    );
    assert!(same_bits(&samples, &[0.0, 2.0, 0.0, 4.0]));
}

#[test]
fn a_second_pass_says_the_pixels_were_unchanged() {
    let mut samples = [0.0, 2.0, 0.0, 4.0];
    assert_eq!(
        redact_rect(&mut samples, 2, 2, 0, 0, 1, 2, 0.0),
        Ok(Redaction {
            samples_written: 0,
            samples_already_fill: 2,
            applied: true,
        })
    );
    assert!(same_bits(&samples, &[0.0, 2.0, 0.0, 4.0]));
}

#[test]
fn a_missed_rectangle_is_not_applied() {
    let mut samples = [0.0, 2.0, 0.0, 4.0];
    assert_eq!(
        redact_rect(&mut samples, 2, 2, 5, 5, 1, 1, 0.0),
        Ok(Redaction {
            samples_written: 0,
            samples_already_fill: 0,
            applied: false,
        })
    );
    assert!(same_bits(&samples, &[0.0, 2.0, 0.0, 4.0]));
}

#[test]
fn a_non_finite_sample_blocks_the_write() {
    let mut samples = [1.0, 2.0, f64::NAN, 4.0];
    assert_eq!(
        redact_rect(&mut samples, 2, 2, 0, 0, 1, 2, 0.0),
        Err(GeomError::Frame)
    );
    let Some(first) = samples.first().copied() else {
        unreachable!("the buffer has four samples")
    };
    let Some(third) = samples.get(2).copied() else {
        unreachable!("the buffer has four samples")
    };
    assert!(first.total_cmp(&1.0).is_eq());
    assert!(third.is_nan());
}

#[test]
fn a_non_finite_fill_writes_nothing() {
    let mut samples = [1.0, 2.0, 3.0, 4.0];
    assert_eq!(
        redact_rect(&mut samples, 2, 2, 0, 0, 1, 2, f64::NAN),
        Err(GeomError::Frame)
    );
    assert!(same_bits(&samples, &[1.0, 2.0, 3.0, 4.0]));
}

#[test]
fn a_zero_side_is_not_applied() {
    let mut samples = [1.0, 2.0, 3.0, 4.0];
    assert_eq!(
        redact_rect(&mut samples, 2, 2, 0, 0, 0, 2, 0.0),
        Ok(Redaction {
            samples_written: 0,
            samples_already_fill: 0,
            applied: false,
        })
    );
    assert!(same_bits(&samples, &[1.0, 2.0, 3.0, 4.0]));
}

#[test]
fn a_short_buffer_is_degenerate() {
    let mut samples = [1.0, 2.0, 3.0];
    assert_eq!(
        redact_rect(&mut samples, 2, 2, 0, 0, 1, 1, 0.0),
        Err(GeomError::Degenerate)
    );
    assert!(same_bits(&samples, &[1.0, 2.0, 3.0]));
}

#[test]
fn a_negative_zero_already_matches_the_fill() {
    let mut samples = [-0.0, 1.0, 2.0, 3.0];
    assert_eq!(
        redact_rect(&mut samples, 2, 2, 0, 0, 1, 1, 0.0),
        Ok(Redaction {
            samples_written: 0,
            samples_already_fill: 1,
            applied: true,
        })
    );
    let Some(first) = samples.first().copied() else {
        unreachable!("the buffer has four samples")
    };
    assert_eq!(first.to_bits(), (-0.0_f64).to_bits());
}
