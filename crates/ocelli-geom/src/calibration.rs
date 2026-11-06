//! Spacing provenance, a length ratio and a volume from contour areas.
//!
//! The HLD does not give these formulas. The local rules are in
//! `.claude/plans/F-195-design.md`. This module does not read DICOM tags
//! and does not call the modality LUT.
//!
//! Pixel spacing is the patient spacing of PS3.3 C.7.6.2, row then column,
//! except where PS3.3 10.7.1.1 says otherwise: an uncalibrated Pixel
//! Spacing has the same values as Imager Pixel Spacing, so an equal pair is
//! detector-plane spacing and takes the imager path. A
//! [`CalibrationSource::Detector`] result is detector-plane spacing, not a
//! patient spacing.
//! Imager pixel spacing is PS3.3 C.8.11.4, in millimetres at the detector,
//! and it is not already corrected for magnification. PS3.3 C.8.7.5 defines
//! the estimated radiographic magnification factor as source image receptor
//! distance divided by source object distance, so the patient spacing
//! derived from the detector is the imager spacing divided by that factor.
//! Ultrasound physical deltas are PS3.3 C.8.5.5.1.17. Their units are
//! PS3.3 C.8.5.5.1.15, where 0003H is centimetres, not millimetres. The
//! caller converts a length unit into positive millimetres and maps
//! Physical Delta Y onto the row and Physical Delta X onto the column.
//! This module does not read the unit code.

use super::measure::GeomError;
use super::stats::SuvError;

/// Which supplied pair was selected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CalibrationSource {
    /// DICOM Pixel Spacing.
    PixelSpacing,
    /// Imager Pixel Spacing divided by the magnification factor.
    ImagerSpacing,
    /// Imager Pixel Spacing at the detector plane, not corrected for
    /// magnification, and not a patient spacing. Chosen only when Pixel
    /// Spacing equals Imager Pixel Spacing (PS3.3 10.7.1.1) and no
    /// magnification factor is present. A present factor that cannot be used
    /// refuses instead.
    Detector,
    /// Ultrasound physical deltas supplied as a row and column pair.
    Ultrasound,
    /// A caller-entered pair.
    Manual,
}

impl CalibrationSource {
    /// Stable name for a page. Not a DICOM keyword.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::PixelSpacing => "pixel-spacing",
            Self::ImagerSpacing => "imager-spacing",
            Self::Detector => "detector",
            Self::Ultrasound => "ultrasound",
            Self::Manual => "manual",
        }
    }
}

/// One spacing pair in millimetres. `row` is the distance between rows.
#[derive(Debug, Clone, Copy)]
pub struct SpacingMm {
    /// Distance between adjacent rows, in millimetres.
    pub row: f64,
    /// Distance between adjacent columns, in millimetres.
    pub column: f64,
}

/// The sources a caller already read. Absent is `None`.
///
/// A `Some` value is present even when its numbers are invalid. An invalid
/// present source is an error and does not fall through to a later source.
#[derive(Debug, Clone, Copy)]
pub struct CalibrationRequest {
    /// Pixel Spacing, when the dataset has it.
    pub pixel_spacing: Option<SpacingMm>,
    /// Imager Pixel Spacing, when the dataset has it.
    pub imager_spacing: Option<SpacingMm>,
    /// Estimated Radiographic Magnification Factor, when the dataset has it.
    pub estimated_radiographic_magnification_factor: Option<f64>,
    /// Ultrasound physical deltas, when the dataset has them.
    pub ultrasound_delta: Option<SpacingMm>,
    /// A manual pair, when the caller entered one.
    pub manual: Option<SpacingMm>,
}

/// The selected source and its spacing in millimetres.
///
/// The spacing is a patient spacing unless the source is
/// [`CalibrationSource::Detector`], which is detector-plane spacing, not
/// corrected for magnification.
#[derive(Debug, Clone, Copy)]
pub struct Calibration {
    /// Which source won.
    pub source: CalibrationSource,
    /// The spacing of [`Self::source`]. After applying the magnification
    /// factor when the source is the imager, and raw at the detector plane
    /// when the source is [`CalibrationSource::Detector`].
    pub spacing: SpacingMm,
}

/// Choose the first present source.
///
/// The order is pixel spacing, imager spacing with the magnification
/// factor, ultrasound deltas, then manual.
///
/// One exception, PS3.3 10.7.1.1: a Pixel Spacing equal to the Imager
/// Pixel Spacing has not been calibrated for geometric magnification, so
/// the imager path decides once Pixel Spacing passes its own check. A
/// factor gives
/// [`CalibrationSource::ImagerSpacing`], the imager spacing divided by the
/// factor. No factor gives [`CalibrationSource::Detector`], the raw imager
/// spacing at the detector plane. A present factor that cannot be used
/// refuses with the codes below, as it does without Pixel Spacing. A Pixel
/// Spacing that differs in either half decides on its own, and the later
/// sources are not read.
///
/// # Errors
///
/// [`SuvError::Frame`] when a present number is not finite, including a
/// non-finite half of the imager pair.
/// [`SuvError::Unavailable`] when every source is absent, or a present
/// magnitude is not strictly positive, or the imager pair is incomplete.
/// On the equal branch both Pixel Spacing and the imager pair are checked,
/// in that order.
pub fn select_calibration(request: CalibrationRequest) -> Result<Calibration, SuvError> {
    if let Some(spacing) = request.pixel_spacing {
        if let Some(imager) = request.imager_spacing
            && same_pair(spacing, imager)
        {
            // Pixel Spacing is the earlier present source, so its own check
            // runs first and an invalid one is refused, not replaced.
            accept(CalibrationSource::PixelSpacing, spacing, None)?;
            return detector_plane(imager, request.estimated_radiographic_magnification_factor);
        }
        return accept(CalibrationSource::PixelSpacing, spacing, None);
    }
    if request.imager_spacing.is_some()
        || request
            .estimated_radiographic_magnification_factor
            .is_some()
    {
        // A present half is examined for finiteness before incompleteness.
        // Either half blocks ultrasound and manual.
        if let Some(spacing) = request.imager_spacing
            && (!spacing.row.is_finite() || !spacing.column.is_finite())
        {
            return Err(SuvError::Frame);
        }
        if let Some(factor) = request.estimated_radiographic_magnification_factor
            && !factor.is_finite()
        {
            return Err(SuvError::Frame);
        }
        let Some(spacing) = request.imager_spacing else {
            return Err(SuvError::Unavailable);
        };
        let Some(factor) = request.estimated_radiographic_magnification_factor else {
            return Err(SuvError::Unavailable);
        };
        return accept(CalibrationSource::ImagerSpacing, spacing, Some(factor));
    }
    if let Some(spacing) = request.ultrasound_delta {
        return accept(CalibrationSource::Ultrasound, spacing, None);
    }
    if let Some(spacing) = request.manual {
        return accept(CalibrationSource::Manual, spacing, None);
    }
    Err(SuvError::Unavailable)
}

/// Both halves agree under [`ocelli_core::spacing_pair_agrees`], the one
/// definition `ocelli-dicom`'s `ImagerAgrees` also uses. This is not a
/// validity check. On the equal branch [`accept`] checks Pixel Spacing and
/// then the imager pair.
fn same_pair(left: SpacingMm, right: SpacingMm) -> bool {
    ocelli_core::spacing_pair_agrees(left.row, left.column, right.row, right.column)
}

/// The imager path of an equal pair, PS3.3 10.7.1.1 and C.8.7.5.
///
/// [`accept`] refuses a present factor that is not finite or not strictly
/// positive, the same refusal the imager path gives without Pixel Spacing.
fn detector_plane(imager: SpacingMm, factor: Option<f64>) -> Result<Calibration, SuvError> {
    match factor {
        Some(factor) => accept(CalibrationSource::ImagerSpacing, imager, Some(factor)),
        None => accept(CalibrationSource::Detector, imager, None),
    }
}

/// `select_calibration` also rejects a non-finite imager half before calling
/// this function. The checks here stay so a later caller cannot skip them.
fn accept(
    source: CalibrationSource,
    spacing: SpacingMm,
    factor: Option<f64>,
) -> Result<Calibration, SuvError> {
    if !spacing.row.is_finite() || !spacing.column.is_finite() {
        return Err(SuvError::Frame);
    }
    if let Some(factor) = factor {
        if !factor.is_finite() {
            return Err(SuvError::Frame);
        }
        if spacing.row.total_cmp(&0.0).is_le()
            || spacing.column.total_cmp(&0.0).is_le()
            || factor.total_cmp(&0.0).is_le()
        {
            return Err(SuvError::Unavailable);
        }
        let row = spacing.row / factor;
        let column = spacing.column / factor;
        if !row.is_finite() || !column.is_finite() {
            return Err(SuvError::Frame);
        }
        if row.total_cmp(&0.0).is_le() || column.total_cmp(&0.0).is_le() {
            return Err(SuvError::Unavailable);
        }
        return Ok(Calibration {
            source,
            spacing: SpacingMm { row, column },
        });
    }
    if spacing.row.total_cmp(&0.0).is_le() || spacing.column.total_cmp(&0.0).is_le() {
        return Err(SuvError::Unavailable);
    }
    Ok(Calibration { source, spacing })
}

/// Divide two caller lengths.
///
/// # Errors
///
/// [`GeomError::Frame`] when an input or the quotient is not finite.
/// [`GeomError::Degenerate`] when the denominator is zero, including negative zero.
pub fn length_ratio(numerator: f64, denominator: f64) -> Result<f64, GeomError> {
    if !numerator.is_finite() || !denominator.is_finite() {
        return Err(GeomError::Frame);
    }
    // `total_cmp` orders negative zero below zero. Both signs are a zero length.
    if denominator.to_bits() == 0.0_f64.to_bits() || denominator.to_bits() == (-0.0_f64).to_bits() {
        return Err(GeomError::Degenerate);
    }
    let ratio = numerator / denominator;
    if !ratio.is_finite() {
        return Err(GeomError::Frame);
    }
    Ok(ratio)
}

/// Spacing times the sum of the caller areas.
///
/// This is not a trapezoidal integration. Areas are square millimetres and
/// spacing is millimetres, so the result is cubic millimetres.
///
/// # Errors
///
/// [`GeomError::Frame`] when spacing, an area or the product is not finite,
/// or an area is negative.
/// [`GeomError::Degenerate`] when the area list is empty or spacing is not
/// strictly positive.
pub fn contour_volume(areas_mm2: &[f64], spacing_mm: f64) -> Result<f64, GeomError> {
    if !spacing_mm.is_finite() {
        return Err(GeomError::Frame);
    }
    if spacing_mm.total_cmp(&0.0).is_le() {
        return Err(GeomError::Degenerate);
    }
    let Some(first) = areas_mm2.first() else {
        return Err(GeomError::Degenerate);
    };
    if !first.is_finite() {
        return Err(GeomError::Frame);
    }
    if first.total_cmp(&0.0).is_lt() {
        return Err(GeomError::Frame);
    }
    let mut sum = *first;
    for area in areas_mm2.iter().skip(1) {
        if !area.is_finite() {
            return Err(GeomError::Frame);
        }
        if area.total_cmp(&0.0).is_lt() {
            return Err(GeomError::Frame);
        }
        sum += *area;
    }
    let volume = spacing_mm * sum;
    if !volume.is_finite() {
        return Err(GeomError::Frame);
    }
    Ok(volume)
}
