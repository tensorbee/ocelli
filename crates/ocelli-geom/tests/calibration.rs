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
fn equal_pixel_and_imager_spacing_is_the_detector_plane() {
    // F-211, the operator answer after review pass 1. PS3.3 10.7.1.1: when
    // Pixel Spacing is present and the image has not been calibrated for
    // geometric magnification, its values are the same as Imager Pixel
    // Spacing. So an equal pair is detector-plane spacing and the imager
    // path decides. With a usable factor 2 the patient spacing is
    // 0.2 / 2 = 0.1 (C.8.7.5). With no factor, the raw 0.2 is reported as
    // the detector plane and is not corrected. A present factor that cannot
    // be used refuses, as it does on the imager path without Pixel Spacing
    // (the lead ruling after F-211 review pass 1).
    let equal = Some(pair(0.2, 0.2));
    let corrected = select_calibration(CalibrationRequest {
        pixel_spacing: equal,
        imager_spacing: equal,
        estimated_radiographic_magnification_factor: Some(2.0),
        ..empty()
    });
    assert!(corrected.is_ok());
    let Ok(corrected) = corrected else {
        unreachable!("asserted immediately above")
    };
    assert_eq!(corrected.source, CalibrationSource::ImagerSpacing);
    assert_eq!(bits(corrected.spacing.row), bits(0.1));
    assert_eq!(bits(corrected.spacing.column), bits(0.1));
    // No factor leaves the detector plane, uncorrected.
    let detector = select_calibration(CalibrationRequest {
        pixel_spacing: equal,
        imager_spacing: equal,
        ..empty()
    });
    assert!(detector.is_ok());
    let Ok(detector) = detector else {
        unreachable!("asserted immediately above")
    };
    assert_eq!(detector.source, CalibrationSource::Detector);
    assert_eq!(detector.source.name(), "detector");
    assert_eq!(bits(detector.spacing.row), bits(0.2));
    assert_eq!(bits(detector.spacing.column), bits(0.2));
    // A present factor that cannot be used refuses with the F-195 codes: a
    // non-finite factor is `Frame`, and one that is not strictly positive is
    // `Unavailable`. The same factors refuse without Pixel Spacing.
    for (factor, want) in [
        (f64::NAN, SuvError::Frame),
        (0.0, SuvError::Unavailable),
        (-2.0, SuvError::Unavailable),
    ] {
        for pixel_spacing in [equal, None] {
            let refused = select_calibration(CalibrationRequest {
                pixel_spacing,
                imager_spacing: equal,
                estimated_radiographic_magnification_factor: Some(factor),
                ..empty()
            });
            assert!(
                matches!(refused, Err(error) if error == want),
                "factor {factor}, pixel spacing {pixel_spacing:?}: {refused:?}"
            );
        }
    }
    // A pair that differs in one half is a corrected Pixel Spacing and
    // decides on its own, whatever the factor.
    let differs = select_calibration(CalibrationRequest {
        pixel_spacing: Some(pair(0.2, 0.18)),
        imager_spacing: equal,
        estimated_radiographic_magnification_factor: Some(f64::NAN),
        ..empty()
    });
    assert!(differs.is_ok());
    let Ok(differs) = differs else {
        unreachable!("asserted immediately above")
    };
    assert_eq!(differs.source, CalibrationSource::PixelSpacing);
    assert_eq!(bits(differs.spacing.column), bits(0.18));
    // F-211 review pass 2, D1. A pair that differs only in the row half is
    // corrected too: 0.18\0.2 beside 0.2\0.2 is Pixel Spacing 0.18 and 0.2.
    let row_differs = select_calibration(CalibrationRequest {
        pixel_spacing: Some(pair(0.18, 0.2)),
        imager_spacing: equal,
        ..empty()
    });
    assert!(row_differs.is_ok());
    let Ok(row_differs) = row_differs else {
        unreachable!("asserted immediately above")
    };
    assert_eq!(row_differs.source, CalibrationSource::PixelSpacing);
    assert_eq!(bits(row_differs.spacing.row), bits(0.18));
    assert_eq!(bits(row_differs.spacing.column), bits(0.2));
}

#[test]
fn an_invalid_pixel_spacing_is_refused_beside_an_agreeing_imager_pair() {
    // F-211 review pass 3, D1. A present Pixel Spacing that is not strictly
    // positive is refused `Unavailable` and does not fall through (F-195).
    // An Imager Pixel Spacing within the 1e-9 mm tolerance of it does not
    // change that: 0\0 beside 5e-10\5e-10, with or without a factor, and
    // -5e-10\-5e-10 beside 5e-10\5e-10, each differ by 5e-10 or 1e-9 mm and
    // agree, and each is still refused.
    //
    // F-211 review pass 4, S1. The order is Pixel Spacing first, then the
    // imager path, and both halves of each pair are checked:
    // - 0\0 with a NaN factor is Pixel Spacing's `Unavailable`, not the
    //   factor's `Frame`, because Pixel Spacing is checked first.
    // - 0\0.2 beside 5e-10\0.2, and 0.2\0 beside 0.2\5e-10, agree within
    //   the tolerance and have one invalid half, which refuses.
    // - A valid 5e-10\5e-10 beside an agreeing imager pair of 0\0 takes the
    //   imager path, whose pair is not strictly positive, so it refuses and
    //   does not fall back to Pixel Spacing.
    let near = pair(5.0e-10, 5.0e-10);
    for (pixel, imager, factor) in [
        (pair(0.0, 0.0), near, None),
        (pair(0.0, 0.0), near, Some(2.0)),
        (pair(-5.0e-10, -5.0e-10), near, None),
        (pair(0.0, 0.0), near, Some(f64::NAN)),
        (pair(0.0, 0.2), pair(5.0e-10, 0.2), None),
        (pair(0.2, 0.0), pair(0.2, 5.0e-10), None),
        (near, pair(0.0, 0.0), None),
    ] {
        let refused = select_calibration(CalibrationRequest {
            pixel_spacing: Some(pixel),
            imager_spacing: Some(imager),
            estimated_radiographic_magnification_factor: factor,
            ..empty()
        });
        assert!(
            matches!(refused, Err(SuvError::Unavailable)),
            "{pixel:?} beside {imager:?} with factor {factor:?}: {refused:?}"
        );
    }
}

#[test]
fn an_equal_pair_is_equal_within_the_one_spacing_tolerance() {
    // F-211 review pass 2, S1. One definition of an equal pair,
    // `ocelli_core::spacing_pair_agrees`, the one `ocelli-dicom`'s
    // `ImagerAgrees` uses. 0.1430000001 and 0.143 are both legal DS text and
    // differ by about 1e-10 mm, inside the 1e-9 mm tolerance, so the pair is
    // equal under PS3.3 10.7.1.1 and with no factor it is the detector plane.
    // A difference of 1e-6 mm is outside it and Pixel Spacing decides.
    let near = select_calibration(CalibrationRequest {
        pixel_spacing: Some(pair(0.143_000_000_1, 0.143_000_000_1)),
        imager_spacing: Some(pair(0.143, 0.143)),
        ..empty()
    });
    assert!(matches!(near, Ok(found) if found.source == CalibrationSource::Detector));
    let apart = select_calibration(CalibrationRequest {
        pixel_spacing: Some(pair(0.143_001, 0.143)),
        imager_spacing: Some(pair(0.143, 0.143)),
        ..empty()
    });
    assert!(matches!(apart, Ok(found) if found.source == CalibrationSource::PixelSpacing));
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
