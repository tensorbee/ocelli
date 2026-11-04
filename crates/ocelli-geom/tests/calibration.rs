//! Hand fixtures for `.claude/plans/F-195-design.md`.
//!
//! PS3.3 C.8.11.4 gives imager pixel spacing at the detector in millimetres.
//! PS3.3 C.8.7.5 gives the estimated radiographic magnification factor as
//! source image receptor distance divided by source object distance. Patient
//! spacing is therefore imager spacing divided by that factor. Imager
//! `0.2` mm and factor `2` yield `0.1` mm. Multiplying by the factor would
//! yield `0.4` and this test would fail.
//!
//! PS3.3 C.7.6.2 orders Pixel Spacing as row then column. A pixel pair that
//! is present wins over a different imager pair.
//!
//! PS3.3 C.8.5.5.1.17 gives Physical Delta X and Physical Delta Y. PS3.3
//! C.8.5.5.1.15 names the units, and 0003H is centimetres. The caller
//! converts a length unit into positive millimetres and maps Y to the row
//! and X to the column before the call. This crate does not read the tags
//! or the unit code.
//!
//! SUVlbm for activity `5`, lean body mass `70` kg, dose `1000` Bq and
//! elapsed `0` is `5 * 70 * 1000 / 1000 = 350`. SUVbsa for body surface
//! area `1.8` m2 is `5 * 1.8 * 10000 / 1000 = 90`. The HLD gives neither
//! formula. The local rule is the design plan. `suv_bw` stays the F-075
//! value in `stats.rs`.
//!
//! Length ratio `4 / 2` is `2`. Areas `10` and `20` with spacing `5` mm
//! give `5 * (10 + 20) = 150`. Averaging the areas would be `75`.

use ocelli_geom::{
    CalibrationRequest, CalibrationSource, GeomError, SpacingMm, SuvActivity, SuvError,
    contour_volume, length_ratio, select_calibration, suv_bsa, suv_lbm,
};

fn bits(value: f64) -> u64 {
    value.to_bits()
}

fn pair(row: f64, column: f64) -> SpacingMm {
    SpacingMm { row, column }
}

fn empty() -> CalibrationRequest {
    CalibrationRequest {
        pixel_spacing: None,
        imager_spacing: None,
        estimated_radiographic_magnification_factor: None,
        ultrasound_delta: None,
        manual: None,
    }
}

fn activity() -> SuvActivity {
    SuvActivity {
        stored: 5.0,
        slope: 1.0,
        intercept: 0.0,
        total_dose_bq: 1000.0,
        half_life_seconds: 100.0,
        elapsed_seconds: 0.0,
        units_bqml: true,
    }
}

#[test]
fn pixel_spacing_wins_over_a_present_imager_pair() {
    let selected = select_calibration(CalibrationRequest {
        pixel_spacing: Some(pair(0.5, 0.7)),
        imager_spacing: Some(pair(0.2, 0.2)),
        estimated_radiographic_magnification_factor: Some(2.0),
        ultrasound_delta: Some(pair(1.0, 1.0)),
        manual: Some(pair(9.0, 9.0)),
    });
    assert!(selected.is_ok());
    let Ok(selected) = selected else {
        unreachable!("asserted immediately above")
    };
    assert_eq!(selected.source, CalibrationSource::PixelSpacing);
    assert_eq!(selected.source.name(), "pixel-spacing");
    assert_eq!(bits(selected.spacing.row), bits(0.5));
    assert_eq!(bits(selected.spacing.column), bits(0.7));
}

#[test]
fn imager_spacing_divided_by_the_magnification_factor_is_one_tenth() {
    // PS3.3 C.8.11.4 and C.8.7.5. 0.2 / 2 = 0.1. 0.4 / 2 = 0.2.
    let selected = select_calibration(CalibrationRequest {
        pixel_spacing: None,
        imager_spacing: Some(pair(0.2, 0.4)),
        estimated_radiographic_magnification_factor: Some(2.0),
        ultrasound_delta: Some(pair(1.0, 1.0)),
        manual: Some(pair(9.0, 9.0)),
    });
    assert!(selected.is_ok());
    let Ok(selected) = selected else {
        unreachable!("asserted immediately above")
    };
    assert_eq!(selected.source, CalibrationSource::ImagerSpacing);
    assert_eq!(bits(selected.spacing.row), bits(0.1));
    assert_eq!(bits(selected.spacing.column), bits(0.2));
}

#[test]
fn an_invalid_pixel_spacing_does_not_fall_through() {
    let zero = select_calibration(CalibrationRequest {
        pixel_spacing: Some(pair(0.0, 0.5)),
        imager_spacing: Some(pair(0.2, 0.2)),
        estimated_radiographic_magnification_factor: Some(2.0),
        ..empty()
    });
    assert!(matches!(zero, Err(SuvError::Unavailable)));
    let nan = select_calibration(CalibrationRequest {
        pixel_spacing: Some(pair(f64::NAN, 0.5)),
        imager_spacing: Some(pair(0.2, 0.2)),
        estimated_radiographic_magnification_factor: Some(2.0),
        ..empty()
    });
    assert!(matches!(nan, Err(SuvError::Frame)));
    let negative_zero = select_calibration(CalibrationRequest {
        pixel_spacing: Some(pair(-0.0, 0.5)),
        imager_spacing: Some(pair(0.2, 0.2)),
        estimated_radiographic_magnification_factor: Some(2.0),
        ..empty()
    });
    assert!(matches!(negative_zero, Err(SuvError::Unavailable)));
}

#[test]
fn ultrasound_is_used_only_when_the_earlier_sources_are_absent() {
    let selected = select_calibration(CalibrationRequest {
        ultrasound_delta: Some(pair(0.3, 0.4)),
        manual: Some(pair(9.0, 9.0)),
        ..empty()
    });
    assert!(selected.is_ok());
    let Ok(selected) = selected else {
        unreachable!("asserted immediately above")
    };
    assert_eq!(selected.source, CalibrationSource::Ultrasound);
    assert_eq!(bits(selected.spacing.row), bits(0.3));
    assert_eq!(bits(selected.spacing.column), bits(0.4));
}

#[test]
fn an_incomplete_imager_pair_does_not_fall_through_to_ultrasound() {
    let spacing_only = select_calibration(CalibrationRequest {
        imager_spacing: Some(pair(0.2, 0.2)),
        ultrasound_delta: Some(pair(0.3, 0.4)),
        ..empty()
    });
    assert!(matches!(spacing_only, Err(SuvError::Unavailable)));
    let factor_only = select_calibration(CalibrationRequest {
        estimated_radiographic_magnification_factor: Some(2.0),
        ultrasound_delta: Some(pair(0.3, 0.4)),
        ..empty()
    });
    assert!(matches!(factor_only, Err(SuvError::Unavailable)));
    let zero_factor = select_calibration(CalibrationRequest {
        imager_spacing: Some(pair(0.2, 0.2)),
        estimated_radiographic_magnification_factor: Some(0.0),
        ultrasound_delta: Some(pair(0.3, 0.4)),
        ..empty()
    });
    assert!(matches!(zero_factor, Err(SuvError::Unavailable)));
    let negative_factor = select_calibration(CalibrationRequest {
        imager_spacing: Some(pair(0.2, 0.2)),
        estimated_radiographic_magnification_factor: Some(-2.0),
        ..empty()
    });
    assert!(matches!(negative_factor, Err(SuvError::Unavailable)));
    let nan_factor = select_calibration(CalibrationRequest {
        imager_spacing: Some(pair(0.2, 0.2)),
        estimated_radiographic_magnification_factor: Some(f64::NAN),
        ultrasound_delta: Some(pair(0.3, 0.4)),
        ..empty()
    });
    assert!(matches!(nan_factor, Err(SuvError::Frame)));
    let nan_spacing = select_calibration(CalibrationRequest {
        imager_spacing: Some(pair(f64::NAN, 0.2)),
        ultrasound_delta: Some(pair(0.3, 0.4)),
        manual: Some(pair(9.0, 9.0)),
        ..empty()
    });
    assert!(matches!(nan_spacing, Err(SuvError::Frame)));
    let half_and_manual = select_calibration(CalibrationRequest {
        estimated_radiographic_magnification_factor: Some(2.0),
        ultrasound_delta: Some(pair(0.3, 0.4)),
        manual: Some(pair(9.0, 9.0)),
        ..empty()
    });
    assert!(matches!(half_and_manual, Err(SuvError::Unavailable)));
    let lone_nan_factor = select_calibration(CalibrationRequest {
        estimated_radiographic_magnification_factor: Some(f64::NAN),
        ultrasound_delta: Some(pair(0.3, 0.4)),
        manual: Some(pair(9.0, 9.0)),
        ..empty()
    });
    assert!(matches!(lone_nan_factor, Err(SuvError::Frame)));
}

#[test]
fn manual_is_last_and_absence_is_unavailable() {
    let selected = select_calibration(CalibrationRequest {
        manual: Some(pair(1.5, 2.5)),
        ..empty()
    });
    assert!(selected.is_ok());
    let Ok(selected) = selected else {
        unreachable!("asserted immediately above")
    };
    assert_eq!(selected.source, CalibrationSource::Manual);
    assert_eq!(selected.source.name(), "manual");
    assert_eq!(bits(selected.spacing.row), bits(1.5));
    assert_eq!(bits(selected.spacing.column), bits(2.5));
    assert!(matches!(
        select_calibration(empty()),
        Err(SuvError::Unavailable)
    ));
    assert!(matches!(
        select_calibration(CalibrationRequest {
            manual: Some(pair(-1.0, 1.0)),
            ..empty()
        }),
        Err(SuvError::Unavailable)
    ));
}

#[test]
fn suv_lbm_is_three_hundred_fifty_and_suv_bsa_is_ninety() {
    assert_eq!(suv_lbm(activity(), 70.0).map(bits), Ok(bits(350.0)));
    assert_eq!(suv_bsa(activity(), 1.8).map(bits), Ok(bits(90.0)));
    assert_eq!(suv_lbm(activity(), 0.0), Err(SuvError::Unavailable));
    assert_eq!(
        suv_lbm(
            SuvActivity {
                units_bqml: false,
                ..activity()
            },
            70.0
        ),
        Err(SuvError::Unavailable)
    );
    assert_eq!(suv_bsa(activity(), f64::NAN), Err(SuvError::Frame));
}

#[test]
fn length_ratio_four_over_two_is_two() {
    assert_eq!(length_ratio(4.0, 2.0).map(bits), Ok(bits(2.0)));
    assert_eq!(length_ratio(-4.0, 2.0).map(bits), Ok(bits(-2.0)));
    assert_eq!(length_ratio(4.0, 0.0), Err(GeomError::Degenerate));
    assert_eq!(length_ratio(4.0, -0.0), Err(GeomError::Degenerate));
    assert_eq!(length_ratio(f64::NAN, 2.0), Err(GeomError::Frame));
}

#[test]
fn contour_volume_is_spacing_times_the_sum_of_the_areas() {
    // 5 * (10 + 20) = 150. The mean of the areas times spacing would be 75.
    assert_eq!(
        contour_volume(&[10.0, 20.0], 5.0).map(bits),
        Ok(bits(150.0))
    );
    assert_eq!(contour_volume(&[], 5.0), Err(GeomError::Degenerate));
    assert_eq!(
        contour_volume(&[10.0, 20.0], 0.0),
        Err(GeomError::Degenerate)
    );
    assert_eq!(
        contour_volume(&[10.0, f64::NAN], 5.0),
        Err(GeomError::Frame)
    );
    assert_eq!(contour_volume(&[10.0, -1.0], 5.0), Err(GeomError::Frame));
}
