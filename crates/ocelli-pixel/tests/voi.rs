//! DICOM PS3.3 C.11.2.1 VOI fixtures.

use ocelli_core::Modality;
use ocelli_pixel::{LutDescriptor, VoiFunction, VoiTransform};

const TOLERANCE: f32 = 0.001;

macro_rules! require_ok {
    ($value:expr) => {{
        let result = $value;
        assert!(result.is_ok());
        let Ok(value) = result else { return };
        value
    }};
}

#[track_caller]
fn assert_close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < TOLERANCE,
        "was {actual}, wanted {expected}"
    );
}

#[test]
fn hld_section_18_3_linear_and_linear_exact_rows_apply_d_13() {
    let linear = require_ok!(VoiTransform::new(
        None,
        &[40.0],
        &[400.0],
        0,
        VoiFunction::Linear,
        0.0,
        255.0
    ));
    let exact = require_ok!(VoiTransform::new(
        None,
        &[40.0],
        &[400.0],
        0,
        VoiFunction::LinearExact,
        0.0,
        255.0
    ));

    // PS3.3 C.11.2.1.2 and C.11.2.1.3.2, with D-13 correcting the
    // LINEAR_EXACT value at -160 to the formula's clamped 0.
    for (input, linear_expected, exact_expected) in [
        (-160.0, 0.0, 0.0),
        (40.0, 127.819_55, 127.5),
        (240.0, 255.0, 255.0),
        (-60.0, 63.909_775, 63.75),
    ] {
        assert_close(linear.apply(Modality(input)).0, linear_expected);
        assert_close(exact.apply(Modality(input)).0, exact_expected);
    }
}

#[test]
fn voi_bounds_use_lower_less_equal_and_upper_strict_greater() {
    let linear = require_ok!(VoiTransform::new(
        None,
        &[40.0],
        &[400.0],
        0,
        VoiFunction::Linear,
        0.0,
        255.0
    ));
    let exact = require_ok!(VoiTransform::new(
        None,
        &[40.0],
        &[400.0],
        0,
        VoiFunction::LinearExact,
        0.0,
        255.0
    ));

    assert_close(linear.apply(Modality(-160.0)).0, 0.0);
    assert_close(linear.apply(Modality(-159.0)).0, 255.0 / 399.0);
    assert_close(linear.apply(Modality(238.0)).0, 255.0 * 398.0 / 399.0);
    assert_close(linear.apply(Modality(239.0)).0, 255.0);
    assert_close(exact.apply(Modality(-160.0)).0, 0.0);
    assert_close(exact.apply(Modality(-159.0)).0, 255.0 / 400.0);
    assert_close(exact.apply(Modality(239.0)).0, 255.0 * 399.0 / 400.0);
    assert_close(exact.apply(Modality(240.0)).0, 255.0);

    let width_one = require_ok!(VoiTransform::new(
        None,
        &[0.0],
        &[1.0],
        0,
        VoiFunction::Linear,
        0.0,
        255.0
    ));
    assert_close(width_one.apply(Modality(-0.5)).0, 0.0);
    assert_close(width_one.apply(Modality(-0.499)).0, 255.0);
}

#[test]
fn sigmoid_minus_sixty_is_255_over_one_plus_e() {
    let sigmoid = require_ok!(VoiTransform::new(
        None,
        &[40.0],
        &[400.0],
        0,
        VoiFunction::Sigmoid,
        0.0,
        255.0
    ));

    // PS3.3 C.11.2.1.3.1: at x=-60, c=40, w=400, the exponent is +1.
    assert_close(
        sigmoid.apply(Modality(-60.0)).0,
        255.0 / (1.0 + core::f32::consts::E),
    );
}

#[test]
fn each_function_refuses_its_invalid_width() {
    assert!(VoiTransform::new(None, &[0.0], &[0.5], 0, VoiFunction::Linear, 0.0, 255.0).is_err());
    assert!(
        VoiTransform::new(
            None,
            &[0.0],
            &[0.0],
            0,
            VoiFunction::LinearExact,
            0.0,
            255.0
        )
        .is_err()
    );
    assert!(VoiTransform::new(None, &[0.0], &[-1.0], 0, VoiFunction::Sigmoid, 0.0, 255.0).is_err());
}

#[test]
fn voi_lut_sequence_takes_precedence_over_a_malformed_window_pair() {
    let lut = require_ok!(LutDescriptor::new(3, -1, 16, vec![5.0, 15.0, 25.0]));
    let transform = require_ok!(VoiTransform::new(
        Some(lut),
        &[40.0, 80.0],
        &[0.0],
        99,
        VoiFunction::Linear,
        0.0,
        255.0
    ));

    assert_eq!(
        transform.apply(Modality(0.0)).0.to_bits(),
        15.0_f32.to_bits()
    );
}
