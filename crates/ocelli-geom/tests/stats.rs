//! Hand-computed fixtures for the local rules in `.claude/plans/F-075-design.md`.
//!
//! Samples 1 and 3 have mean 2. The population variance is
//! `(1 + 1) / 2 = 1`, so the standard deviation is 1. Dividing by `n - 1`
//! would be `sqrt(2)` and this test would fail.
//!
//! Samples 0, 0.5 and 1 in two bins have width 0.5. The value 0.5 sits on
//! the interior edge and belongs to the higher bin, so the counts are 1 and 2.
//!
//! SUV with elapsed 0 is `2 * 70 * 1000 / 1000 = 140`. One half-life later
//! the decayed dose is half, so the result is 280. The formula is the plan,
//! which follows the project skill. It is not a PS3.3 equation.

use ocelli_geom::{GeomError, SuvError, SuvInputs, histogram, roi_statistics, suv_bw};

fn bits(value: f64) -> u64 {
    value.to_bits()
}

fn roi(samples: &[f64], area: f64) -> ocelli_geom::RoiStats {
    let result = roi_statistics(samples, area);
    assert!(result.is_ok());
    let Ok(stats) = result else {
        unreachable!("asserted immediately above")
    };
    stats
}

fn bins(samples: &[f64], counts: &mut [u32]) -> ocelli_geom::Histogram {
    let result = histogram(samples, counts);
    assert!(result.is_ok());
    let Ok(histogram) = result else {
        unreachable!("asserted immediately above")
    };
    histogram
}

fn pet(elapsed: f64, units_bqml: bool, weight: f64) -> SuvInputs {
    SuvInputs {
        stored: 2.0,
        slope: 1.0,
        intercept: 0.0,
        patient_weight_kg: weight,
        total_dose_bq: 1000.0,
        half_life_seconds: 100.0,
        elapsed_seconds: elapsed,
        units_bqml,
    }
}

#[test]
fn two_samples_have_population_sd_one() {
    let stats = roi(&[1.0, 3.0], 4.0);
    assert_eq!(stats.count, 2);
    assert_eq!(bits(stats.mean), bits(2.0));
    assert_eq!(bits(stats.sd), bits(1.0));
    assert_eq!(bits(stats.min), bits(1.0));
    assert_eq!(bits(stats.max), bits(3.0));
    assert_eq!(bits(stats.area), bits(4.0));
}

#[test]
fn an_empty_region_is_degenerate() {
    assert_eq!(roi_statistics(&[], 4.0), Err(GeomError::Degenerate));
    assert_eq!(roi_statistics(&[1.0], f64::NAN), Err(GeomError::Frame));
    assert_eq!(roi_statistics(&[f64::NAN], 4.0), Err(GeomError::Frame));
}

#[test]
fn one_sample_has_sd_zero() {
    let stats = roi(&[5.0], 1.0);
    assert_eq!(stats.count, 1);
    assert_eq!(bits(stats.mean), bits(5.0));
    assert_eq!(bits(stats.sd), bits(0.0));
    assert_eq!(bits(stats.min), bits(5.0));
    assert_eq!(bits(stats.max), bits(5.0));
}

#[test]
fn an_interior_edge_belongs_to_the_higher_bin() {
    let mut counts = [0, 7];
    let histogram = bins(&[0.0, 0.5, 1.0], &mut counts);
    assert_eq!(counts, [1, 2]);
    assert_eq!(bits(histogram.min), bits(0.0));
    assert_eq!(bits(histogram.max), bits(1.0));
    assert_eq!(bits(histogram.width), bits(0.5));
}

#[test]
fn the_maximum_of_a_narrow_range_is_the_last_bin() {
    let mut counts = [0, 0, 0, 0, 0, 0, 0, 0];
    let low = 10_000_000.0;
    let high = low + 1.0e-8;
    let histogram = bins(&[low, high], &mut counts);
    let Some(first) = counts.first().copied() else {
        unreachable!("eight bins")
    };
    let Some(last) = counts.last().copied() else {
        unreachable!("eight bins")
    };
    assert_eq!(first, 1);
    assert_eq!(last, 1);
    assert_eq!(counts.iter().sum::<u32>(), 2);
    assert!(histogram.width.total_cmp(&0.0).is_ne());
}

#[test]
fn equal_samples_fill_the_first_bin() {
    let mut counts = [9, 9, 9];
    let histogram = bins(&[5.0, 5.0], &mut counts);
    assert_eq!(counts, [2, 0, 0]);
    assert_eq!(bits(histogram.width), bits(0.0));
}

#[test]
fn a_non_finite_sample_does_not_write_bins() {
    let mut counts = [4, 5];
    assert_eq!(
        histogram(&[0.0, f64::NAN, 1.0], &mut counts),
        Err(GeomError::Frame)
    );
    assert_eq!(counts, [4, 5]);
    assert_eq!(histogram(&[], &mut counts), Err(GeomError::Degenerate));
    assert_eq!(counts, [4, 5]);
    let mut no_bins = [];
    assert_eq!(
        histogram(&[1.0, 2.0], &mut no_bins),
        Err(GeomError::Degenerate)
    );
}

#[test]
fn elapsed_zero_is_one_hundred_forty() {
    assert_eq!(suv_bw(pet(0.0, true, 70.0)).map(bits), Ok(bits(140.0)));
}

#[test]
fn one_half_life_is_two_hundred_eighty() {
    assert_eq!(suv_bw(pet(100.0, true, 70.0)).map(bits), Ok(bits(280.0)));
}

#[test]
fn counts_are_unavailable() {
    assert_eq!(suv_bw(pet(0.0, false, 70.0)), Err(SuvError::Unavailable));
}

#[test]
fn a_zero_weight_is_unavailable() {
    assert_eq!(suv_bw(pet(0.0, true, 0.0)), Err(SuvError::Unavailable));
    assert_eq!(suv_bw(pet(-1.0, true, 70.0)), Err(SuvError::Unavailable));
    assert_eq!(
        suv_bw(SuvInputs {
            stored: f64::NAN,
            ..pet(0.0, true, 70.0)
        }),
        Err(SuvError::Frame)
    );
}
