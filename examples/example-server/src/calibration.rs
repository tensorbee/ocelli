//! JSON for `POST /calibration`.
//!
//! The body is the token `synthetic`. The numbers are the F-195 hand
//! fixtures. Imager spacing `0.2` mm divided by magnification factor `2`
//! is `0.1` mm. SUVlbm is `350`. SUVbsa is `90`. The length ratio is `2`.
//! The contour volume is `150`.

use ocelli_geom::{
    CalibrationRequest, SpacingMm, SuvActivity, contour_volume, length_ratio, select_calibration,
    suv_bsa, suv_lbm,
};

/// Answer the calibration fixture.
#[must_use]
pub fn calibration_reply(body: &[u8]) -> (u16, String) {
    if body != b"synthetic" {
        return (400, "{\"error\":\"fixture\"}".to_string());
    }
    let selected = select_calibration(CalibrationRequest {
        pixel_spacing: None,
        imager_spacing: Some(SpacingMm {
            row: 0.2,
            column: 0.2,
        }),
        estimated_radiographic_magnification_factor: Some(2.0),
        ultrasound_delta: Some(SpacingMm {
            row: 1.0,
            column: 1.0,
        }),
        manual: Some(SpacingMm {
            row: 9.0,
            column: 9.0,
        }),
    });
    let Ok(selected) = selected else {
        return (500, "{\"error\":\"fixture\"}".to_string());
    };
    let lbm = suv_lbm(activity(), 70.0);
    let Ok(lbm) = lbm else {
        return (500, "{\"error\":\"fixture\"}".to_string());
    };
    let bsa = suv_bsa(activity(), 1.8);
    let Ok(bsa) = bsa else {
        return (500, "{\"error\":\"fixture\"}".to_string());
    };
    let ratio = length_ratio(4.0, 2.0);
    let Ok(ratio) = ratio else {
        return (500, "{\"error\":\"fixture\"}".to_string());
    };
    let volume = contour_volume(&[10.0, 20.0], 5.0);
    let Ok(volume) = volume else {
        return (500, "{\"error\":\"fixture\"}".to_string());
    };
    let Some(row) = known(selected.spacing.row) else {
        return (500, "{\"error\":\"fixture\"}".to_string());
    };
    let Some(column) = known(selected.spacing.column) else {
        return (500, "{\"error\":\"fixture\"}".to_string());
    };
    let Some(lbm) = known(lbm) else {
        return (500, "{\"error\":\"fixture\"}".to_string());
    };
    let Some(bsa) = known(bsa) else {
        return (500, "{\"error\":\"fixture\"}".to_string());
    };
    let Some(ratio) = known(ratio) else {
        return (500, "{\"error\":\"fixture\"}".to_string());
    };
    let Some(volume) = known(volume) else {
        return (500, "{\"error\":\"fixture\"}".to_string());
    };
    (
        200,
        format!(
            "{{\"source\":\"{source}\",\"rowMm\":{row},\"columnMm\":{column},\"suvLbm\":{lbm},\"suvBsa\":{bsa},\"lengthRatio\":{ratio},\"contourVolume\":{volume}}}",
            source = selected.source.name(),
        ),
    )
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

fn known(value: f64) -> Option<&'static str> {
    let bits = value.to_bits();
    if bits == 0.1_f64.to_bits() {
        return Some("0.1");
    }
    if bits == 2.0_f64.to_bits() {
        return Some("2");
    }
    if bits == 90.0_f64.to_bits() {
        return Some("90");
    }
    if bits == 150.0_f64.to_bits() {
        return Some("150");
    }
    if bits == 350.0_f64.to_bits() {
        return Some("350");
    }
    None
}
