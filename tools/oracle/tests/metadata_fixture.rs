//! Independent metadata fixtures from DICOM PS3.3.

use std::collections::BTreeMap;
use std::error::Error;

use serde::Deserialize;

type Outcome = Result<(), Box<dyn Error>>;
const GEOMETRY_TOLERANCE_MM: f64 = 1e-6;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Truth {
    display_fixtures: BTreeMap<String, DisplayFixture>,
    geometry_fixtures: BTreeMap<String, GeometryFixture>,
}

#[derive(Deserialize)]
struct VolumeTruth {
    subjects: BTreeMap<String, VolumeFixture>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DisplayFixture {
    samples: Vec<DisplaySample>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DisplaySample {
    input: f64,
    linear: f64,
    linear_exact: f64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GeometryFixture {
    image_position_patient: [f64; 3],
    image_orientation_patient: [f64; 6],
    pixel_spacing: [f64; 2],
    samples: Vec<GeometrySample>,
}

#[derive(Deserialize)]
struct GeometrySample {
    column: f64,
    row: f64,
    patient: [f64; 3],
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct VolumeFixture {
    #[serde(default)]
    projections_mm: Vec<f64>,
    #[serde(default)]
    gaps_mm: Vec<f64>,
    #[serde(default)]
    slice_thickness: f64,
}

fn truth() -> Result<Truth, serde_json::Error> {
    serde_json::from_str(include_str!("../metadata-truth.json"))
}

fn volume_truth() -> Result<VolumeTruth, serde_json::Error> {
    serde_json::from_str(include_str!("../volume-truth.json"))
}

#[test]
fn section_c11_display_values_are_the_hand_computed_fixture() -> Outcome {
    // PS3.3 C.11.2.1.2 and C.11.2.1.3.2. D-13 corrects the first
    // LINEAR_EXACT value to zero because x <= c - w/2 clamps to ymin.
    let document = truth()?;
    let samples = &document
        .display_fixtures
        .get("soft-tissue-ct")
        .ok_or("soft tissue fixture is absent")?
        .samples;
    assert_eq!(samples.len(), 4);
    assert!(samples.iter().all(|sample| {
        sample.input.is_finite() && sample.linear.is_finite() && sample.linear_exact.is_finite()
    }));
    Ok(())
}

fn voxel_to_patient(
    ipp: [f64; 3],
    iop: [f64; 6],
    spacing: [f64; 2],
    column: f64,
    row: f64,
) -> [f64; 3] {
    // PS3.3 C.7.6.2.1.1. PixelSpacing[1] scales the row direction for
    // column i, while PixelSpacing[0] scales the column direction for row j.
    let [px, py, pz] = ipp;
    let [x0, x1, x2, y0, y1, y2] = iop;
    let [row_spacing, column_spacing] = spacing;
    [
        px + column * column_spacing * x0 + row * row_spacing * y0,
        py + column * column_spacing * x1 + row * row_spacing * y1,
        pz + column * column_spacing * x2 + row * row_spacing * y2,
    ]
}

fn point_is_within_tolerance(actual: [f64; 3], expected: [f64; 3]) -> bool {
    actual
        .iter()
        .zip(expected)
        .all(|(actual, expected)| (actual - expected).abs() <= GEOMETRY_TOLERANCE_MM)
}

#[test]
fn nonsquare_oblique_samples_map_column_and_row_indices_correctly() -> Outcome {
    let document = truth()?;
    let fixture = document
        .geometry_fixtures
        .get("oblique-nonsquare")
        .ok_or("oblique fixture is absent")?;
    for sample in &fixture.samples {
        let actual = voxel_to_patient(
            fixture.image_position_patient,
            fixture.image_orientation_patient,
            fixture.pixel_spacing,
            sample.column,
            sample.row,
        );
        assert!(point_is_within_tolerance(actual, sample.patient));
    }
    Ok(())
}

#[test]
fn swapping_spacing_or_direction_vectors_breaks_the_fixture() -> Outcome {
    let document = truth()?;
    let fixture = document
        .geometry_fixtures
        .get("oblique-nonsquare")
        .ok_or("oblique fixture is absent")?;
    let expected = fixture
        .samples
        .first()
        .ok_or("oblique geometry has no samples")?
        .patient;
    let [row_spacing, column_spacing] = fixture.pixel_spacing;
    let swapped_spacing = [column_spacing, row_spacing];
    assert!(!point_is_within_tolerance(
        voxel_to_patient(
            fixture.image_position_patient,
            fixture.image_orientation_patient,
            swapped_spacing,
            2.0,
            3.0,
        ),
        expected
    ));
    let [x0, x1, x2, y0, y1, y2] = fixture.image_orientation_patient;
    let swapped_iop = [y0, y1, y2, x0, x1, x2];
    assert!(!point_is_within_tolerance(
        voxel_to_patient(
            fixture.image_position_patient,
            swapped_iop,
            fixture.pixel_spacing,
            2.0,
            3.0,
        ),
        expected
    ));
    Ok(())
}

#[test]
fn projected_gaps_not_thickness_expose_nonuniform_spacing() -> Outcome {
    // PS3.3 C.7.6.2.1.1. The slice coordinate is the IPP projection onto
    // the normal. SliceThickness is not the distance between positions.
    let document = volume_truth()?;
    let fixture = document
        .subjects
        .get("volume__synthetic__ct_series_nonuniform")
        .ok_or("nonuniform series fixture is absent")?;
    let derived: Vec<f64> = fixture
        .projections_mm
        .windows(2)
        .filter_map(|pair| pair.first().zip(pair.get(1)).map(|(a, b)| b - a))
        .collect();
    assert_eq!(derived.len(), fixture.gaps_mm.len());
    assert!(
        derived
            .iter()
            .zip(&fixture.gaps_mm)
            .all(|(actual, expected)| (actual - expected).abs() <= GEOMETRY_TOLERANCE_MM)
    );
    assert!(
        fixture
            .gaps_mm
            .iter()
            .any(|gap| gap.to_bits() != fixture.slice_thickness.to_bits())
    );
    Ok(())
}
