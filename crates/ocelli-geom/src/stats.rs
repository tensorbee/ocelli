//! ROI statistics.
//!
//! The HLD does not give these formulas. The local rules for statistics and
//! SUVbw are in `.claude/plans/F-075-design.md`. SUVlbm and SUVbsa follow
//! `.claude/plans/F-195-design.md` and share this decay. SUV is a separate
//! path and this module does not call the modality LUT. An empty sample list
//! is degenerate. The standard deviation divides by the count. The area is
//! the caller's number.
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

/// PET activity inputs shared by the SUV methods.
///
/// Weight, lean body mass and body surface area are not fields. Each method
/// receives its own body-size magnitude.
#[derive(Debug, Clone, Copy)]
pub struct SuvActivity {
    /// Stored pixel value.
    pub stored: f64,
    /// Rescale slope supplied by the caller.
    pub slope: f64,
    /// Rescale intercept supplied by the caller.
    pub intercept: f64,
    /// Radionuclide total dose in becquerels.
    pub total_dose_bq: f64,
    /// Radionuclide half-life in seconds.
    pub half_life_seconds: f64,
    /// Seconds from injection to series time.
    pub elapsed_seconds: f64,
    /// True only when Units is `BQML`.
    pub units_bqml: bool,
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

/// A refused SUV or calibration.
///
/// `Frame` is 402 and `Unavailable` is 403. Those numbers sit beside the
/// `ocelli-geom` measurement range and are not [`GeomError`] values.
/// Calibration uses the same codes. A non-finite present value is `Frame`.
/// A missing source or a magnitude that is not strictly positive is
/// `Unavailable`.
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

fn edge_at(min: f64, width: f64, bin: usize) -> Option<f64> {
    let index = u32::try_from(bin).ok()?;
    Some(min + width * f64::from(index))
}

/// Highest bin whose edge still moves. A later multiple is not accepted.
fn first_stall(min: f64, width: f64, last: usize) -> usize {
    let mut stall = 0_usize;
    while stall < last {
        let Some(next) = stall.checked_add(1) else {
            break;
        };
        let (Some(edge), Some(prev)) = (edge_at(min, width, next), edge_at(min, width, stall))
        else {
            break;
        };
        if !edge.total_cmp(&prev).is_gt() {
            break;
        }
        stall = next;
    }
    stall
}

/// Largest bin at or before `stall` whose edge is still at or below `sample`.
fn bin_before_stall(sample: f64, min: f64, width: f64, stall: usize) -> usize {
    let mut lo = 0_usize;
    let mut hi = stall;
    while lo < hi {
        let Some(step) = (hi - lo).checked_add(1) else {
            break;
        };
        let mid = lo + step / 2;
        let Some(edge) = edge_at(min, width, mid) else {
            hi = mid.saturating_sub(1);
            continue;
        };
        if sample.total_cmp(&edge).is_ge() {
            lo = mid;
        } else {
            hi = mid.saturating_sub(1);
        }
    }
    lo
}

/// Fill `counts` from `samples`.
///
/// A value on an interior edge belongs to the higher bin. The edge is the
/// minimum plus the width times the bin. Negative zero is that same value.
/// Edges count only up to the first one that does not rise above the edge
/// before it. A sample above that edge, other than the maximum, is in the
/// bin of the edge before it. The maximum belongs to the last bin, including
/// when that edge does not advance. Equal samples all land in bin 0.
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
    // An exact zero span, including both signs of zero, is one value. A span
    // that underflows when divided by the bin count is not one value. The
    // maximum still belongs in the last bin. Negative zero matches zero.
    let min_edge = min + 0.0;
    let max_edge = max + 0.0;
    let span = max_edge - min_edge;
    let width = if exact_zero(span) {
        0.0
    } else {
        span / f64::from(bin_count)
    };
    if !width.is_finite() {
        return Err(GeomError::Frame);
    }
    for slot in counts.iter_mut() {
        *slot = 0;
    }
    if exact_zero(span) {
        if let Some(slot) = counts.first_mut() {
            *slot = sample_count;
        }
        return Ok(Histogram { min, max, width });
    }
    let last = counts.len() - 1;
    let stall = first_stall(min_edge, width, last);
    for &sample in samples {
        let sample = sample + 0.0;
        let index = if sample.total_cmp(&max_edge).is_eq() {
            last
        } else {
            bin_before_stall(sample, min_edge, width, stall)
        };
        if let Some(slot) = counts.get_mut(index) {
            *slot = slot.saturating_add(1);
        }
    }
    Ok(Histogram { min, max, width })
}

/// Body-weight SUV from caller-supplied inputs.
///
/// This is not the modality LUT. The activity line is the separate PET path.
/// The decay is the same function `suv_lbm` and `suv_bsa` call.
///
/// # Errors
///
/// [`SuvError::Frame`] when an input or the result is non-finite.
/// [`SuvError::Unavailable`] when units are not BQML, or weight, dose or
/// half-life is not strictly positive, or elapsed time is negative.
pub fn suv_bw(inputs: SuvInputs) -> Result<f64, SuvError> {
    scaled_suv(activity_from(inputs), inputs.patient_weight_kg, 1000.0)
}

/// SUV by lean body mass.
///
/// The caller supplies lean body mass in kilograms. This function does not
/// derive it from height and weight. The scale is 1000, the same grams
/// conversion as [`suv_bw`]. The HLD gives no formula. The local rule is
/// `.claude/plans/F-195-design.md`.
///
/// # Errors
///
/// [`SuvError::Frame`] when an input or the result is non-finite.
/// [`SuvError::Unavailable`] when units are not BQML, or lean body mass,
/// dose or half-life is not strictly positive, or elapsed time is negative.
pub fn suv_lbm(activity: SuvActivity, lbm_kg: f64) -> Result<f64, SuvError> {
    scaled_suv(activity, lbm_kg, 1000.0)
}

/// SUV by body surface area.
///
/// The caller supplies body surface area in square metres. The scale is
/// 10000 so square metres become square centimetres beside the grams
/// conversion in [`suv_bw`]. The HLD gives no formula. The local rule is
/// `.claude/plans/F-195-design.md`.
///
/// # Errors
///
/// [`SuvError::Frame`] when an input or the result is non-finite.
/// [`SuvError::Unavailable`] when units are not BQML, or body surface area,
/// dose or half-life is not strictly positive, or elapsed time is negative.
pub fn suv_bsa(activity: SuvActivity, bsa_m2: f64) -> Result<f64, SuvError> {
    scaled_suv(activity, bsa_m2, 10_000.0)
}

fn activity_from(inputs: SuvInputs) -> SuvActivity {
    SuvActivity {
        stored: inputs.stored,
        slope: inputs.slope,
        intercept: inputs.intercept,
        total_dose_bq: inputs.total_dose_bq,
        half_life_seconds: inputs.half_life_seconds,
        elapsed_seconds: inputs.elapsed_seconds,
        units_bqml: inputs.units_bqml,
    }
}

fn scaled_suv(activity: SuvActivity, magnitude: f64, scale: f64) -> Result<f64, SuvError> {
    let fields = [
        activity.stored,
        activity.slope,
        activity.intercept,
        activity.total_dose_bq,
        activity.half_life_seconds,
        activity.elapsed_seconds,
        magnitude,
        scale,
    ];
    for value in fields {
        if !value.is_finite() {
            return Err(SuvError::Frame);
        }
    }
    if !activity.units_bqml
        || magnitude.total_cmp(&0.0).is_le()
        || activity.total_dose_bq.total_cmp(&0.0).is_le()
        || activity.half_life_seconds.total_cmp(&0.0).is_le()
        || activity.elapsed_seconds < 0.0
        || scale.total_cmp(&0.0).is_le()
    {
        return Err(SuvError::Unavailable);
    }
    let concentration = activity.stored * activity.slope + activity.intercept;
    let exponent = -activity.elapsed_seconds / activity.half_life_seconds;
    let decay = DVec2::new(2.0, 0.0).powf(exponent).x;
    let decayed = activity.total_dose_bq * decay;
    let suv = concentration * magnitude * scale / decayed;
    if !suv.is_finite() {
        return Err(SuvError::Frame);
    }
    Ok(suv)
}
