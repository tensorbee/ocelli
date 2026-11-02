//! ROI statistics.
//!
//! The HLD does not give these formulas. The local rules are in
//! `.claude/plans/F-075-design.md`. SUV is a separate path and this module
//! does not call the modality LUT. An empty sample list is degenerate. The
//! standard deviation divides by the count. The area is the caller's number.
//! A histogram writes the caller's bins only after the samples validate.

use glam::DVec2;

use crate::GeomError;

/// Mean, population standard deviation, extrema and a caller-supplied area.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RoiStats {
    /// How many finite samples were summarised.
    pub count: u32,
    /// Sum divided by [`Self::count`].
    pub mean: f64,
    /// Population standard deviation. One sample is 0.
    pub sd: f64,
    /// Smallest sample.
    pub min: f64,
    /// Largest sample.
    pub max: f64,
    /// The area the caller already measured. This function does not recompute it.
    pub area: f64,
}

/// The range a histogram used. Counts live in the caller's buffer.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Histogram {
    /// Smallest sample.
    pub min: f64,
    /// Largest sample.
    pub max: f64,
    /// Equal bin width. Zero when every sample is equal.
    pub width: f64,
}

/// Inputs for one SUVbw. The caller has already read the tags.
#[derive(Debug, Clone, Copy)]
pub struct SuvInputs {
    /// Stored pixel value.
    pub stored: f64,
    /// Rescale slope supplied by the caller.
    pub slope: f64,
    /// Rescale intercept supplied by the caller.
    pub intercept: f64,
    /// Patient weight in kilograms.
    pub patient_weight_kg: f64,
    /// Radionuclide total dose in becquerels.
    pub total_dose_bq: f64,
    /// Radionuclide half-life in seconds.
    pub half_life_seconds: f64,
    /// Seconds from injection to series time.
    pub elapsed_seconds: f64,
    /// True only when Units is `BQML`.
    pub units_bqml: bool,
}

/// A refused SUV.
///
/// `Frame` is 402 and `Unavailable` is 403. Those numbers sit beside the
/// `ocelli-geom` measurement range and are not [`GeomError`] values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuvError {
    /// An input or the result is not finite.
    Frame = 402,
    /// Units is not BQML, or a required magnitude is missing.
    Unavailable = 403,
}

impl SuvError {
    /// The stable number. A match, not a cast.
    #[must_use]
    pub const fn number(self) -> u16 {
        match self {
            Self::Frame => 402,
            Self::Unavailable => 403,
        }
    }
}

fn exact_zero(value: f64) -> bool {
    value.to_bits() == 0.0_f64.to_bits() || value.to_bits() == (-0.0_f64).to_bits()
}

fn square_root(value: f64) -> f64 {
    DVec2::new(value, 0.0).powf(0.5).x
}

/// Summarise finite samples.
///
/// # Errors
///
/// [`GeomError::Degenerate`] when `samples` is empty.
/// [`GeomError::Frame`] when a sample, the area, or a total is non-finite.
pub fn roi_statistics(samples: &[f64], area: f64) -> Result<RoiStats, GeomError> {
    let Some(first) = samples.first().copied() else {
        return Err(GeomError::Degenerate);
    };
    if !area.is_finite() {
        return Err(GeomError::Frame);
    }
    let Ok(count) = u32::try_from(samples.len()) else {
        return Err(GeomError::Frame);
    };
    if !first.is_finite() {
        return Err(GeomError::Frame);
    }
    let mut sum = first;
    let mut min = first;
    let mut max = first;
    for &sample in samples.iter().skip(1) {
        if !sample.is_finite() {
            return Err(GeomError::Frame);
        }
        sum += sample;
        if sample.total_cmp(&min).is_lt() {
            min = sample;
        }
        if sample.total_cmp(&max).is_gt() {
            max = sample;
        }
    }
    let mean = sum / f64::from(count);
    let mut acc = 0.0;
    for &sample in samples {
        let delta = sample - mean;
        acc += delta * delta;
    }
    let variance = acc / f64::from(count);
    if !mean.is_finite() || (!variance.is_finite()) {
        return Err(GeomError::Frame);
    }
    let sd = if exact_zero(variance) || variance.total_cmp(&0.0).is_lt() {
        if variance.total_cmp(&0.0).is_lt() && !exact_zero(variance) {
            return Err(GeomError::Frame);
        }
        0.0
    } else {
        square_root(variance)
    };
    if !sd.is_finite() {
        return Err(GeomError::Frame);
    }
    Ok(RoiStats {
        count,
        mean,
        sd,
        min,
        max,
        area,
    })
}

/// Fill `counts` from `samples`.
///
/// A value on an interior edge belongs to the higher bin. The maximum belongs
/// to the last bin. Equal samples all land in bin 0.
///
/// # Errors
///
/// [`GeomError::Degenerate`] when either slice is empty. The buffer is unchanged.
/// [`GeomError::Frame`] when a sample or the width is non-finite. The buffer
/// is unchanged.
pub fn histogram(samples: &[f64], counts: &mut [u32]) -> Result<Histogram, GeomError> {
    if samples.is_empty() || counts.is_empty() {
        return Err(GeomError::Degenerate);
    }
    let Some(first) = samples.first().copied() else {
        return Err(GeomError::Degenerate);
    };
    if !first.is_finite() {
        return Err(GeomError::Frame);
    }
    let mut min = first;
    let mut max = first;
    for &sample in samples.iter().skip(1) {
        if !sample.is_finite() {
            return Err(GeomError::Frame);
        }
        if sample.total_cmp(&min).is_lt() {
            min = sample;
        }
        if sample.total_cmp(&max).is_gt() {
            max = sample;
        }
    }
    let Ok(bin_count) = u32::try_from(counts.len()) else {
        return Err(GeomError::Frame);
    };
    let Ok(sample_count) = u32::try_from(samples.len()) else {
        return Err(GeomError::Frame);
    };
    let width = if min.total_cmp(&max).is_eq() {
        0.0
    } else {
        (max - min) / f64::from(bin_count)
    };
    if !width.is_finite() {
        return Err(GeomError::Frame);
    }
    for slot in counts.iter_mut() {
        *slot = 0;
    }
    if exact_zero(width) {
        if let Some(slot) = counts.first_mut() {
            *slot = sample_count;
        }
        return Ok(Histogram { min, max, width });
    }
    let last = counts.len() - 1;
    for &sample in samples {
        let mut index = 0;
        let mut edge = min + width;
        while index < last && sample.total_cmp(&edge).is_ge() {
            index += 1;
            edge += width;
        }
        if let Some(slot) = counts.get_mut(index) {
            *slot = slot.saturating_add(1);
        }
    }
    Ok(Histogram { min, max, width })
}

/// Body-weight SUV from caller-supplied inputs.
///
/// This is not the modality LUT. The activity line is the separate PET path.
///
/// # Errors
///
/// [`SuvError::Frame`] when an input or the result is non-finite.
/// [`SuvError::Unavailable`] when units are not BQML, or weight, dose or
/// half-life is not strictly positive, or elapsed time is negative.
pub fn suv_bw(inputs: SuvInputs) -> Result<f64, SuvError> {
    let fields = [
        inputs.stored,
        inputs.slope,
        inputs.intercept,
        inputs.patient_weight_kg,
        inputs.total_dose_bq,
        inputs.half_life_seconds,
        inputs.elapsed_seconds,
    ];
    for value in fields {
        if !value.is_finite() {
            return Err(SuvError::Frame);
        }
    }
    if !inputs.units_bqml
        || inputs.patient_weight_kg.total_cmp(&0.0).is_le()
        || inputs.total_dose_bq.total_cmp(&0.0).is_le()
        || inputs.half_life_seconds.total_cmp(&0.0).is_le()
        || inputs.elapsed_seconds.total_cmp(&0.0).is_lt()
    {
        return Err(SuvError::Unavailable);
    }
    let activity = inputs.stored * inputs.slope + inputs.intercept;
    let exponent = -inputs.elapsed_seconds / inputs.half_life_seconds;
    let decay = DVec2::new(2.0, 0.0).powf(exponent).x;
    let decayed = inputs.total_dose_bq * decay;
    if !activity.is_finite() || !decayed.is_finite() || decayed.total_cmp(&0.0).is_le() {
        return Err(SuvError::Unavailable);
    }
    let suv = activity * inputs.patient_weight_kg * 1000.0 / decayed;
    if !suv.is_finite() {
        return Err(SuvError::Frame);
    }
    Ok(suv)
}
