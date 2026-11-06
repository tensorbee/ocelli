//! Lengths, angles and region statistics for one held frame.
//!
//! The arithmetic is `ocelli-geom`: `length`, `angle_at`, `cobb`,
//! `bidirectional`, `rectangle`, `ellipse`, `polygon_area`,
//! `region_bounds`, `region_contains`, `polygon_crosses_itself`,
//! `roi_statistics` and `select_calibration`. The modality value of each
//! sample is the held frame's stage 1, `ModalityTransform::apply`, read
//! through F-210's value facts, so the window never enters a statistic.
//! This module owns the page-to-index shift, the millimetre points, the
//! units, the rounding of the text and the refusals. The local rules are
//! `.claude/plans/F-211-design.md`.
//!
//! Page marks are in image-pixel coordinates, where cell `c` covers
//! `[c, c + 1)` and has its centre at `c + 0.5`. Index space puts that
//! centre at `c`, so `index = page - 0.5` on both axes, here and nowhere
//! else. PS3.3 C.7.6.2: Pixel Spacing is the adjacent row spacing, then the
//! adjacent column spacing, in mm. PS3.3 C.7.6.2.1.1 scales the column index
//! `i` by the column spacing and the row index `j` by the row spacing, so a
//! column step is `PixelSpacing[1]` and a row step is `PixelSpacing[0]`.

use ocelli_core::{Index, Pt, World};
use ocelli_geom::{
    CalibrationRequest, CalibrationSource, GeomError, RegionShape, SpacingMm, angle_at,
    bidirectional, cobb, ellipse, length, polygon_area, polygon_crosses_itself, rectangle,
    region_bounds, region_contains, roi_statistics, select_calibration,
};
use ocelli_viewport::view::f32_from_f64;

use crate::preview::{HeldFrame, PreviewError, decode_held, json_string, push_field};
use crate::value::{ImagePoint, stored_at, stored_integer};

/// Most samples a region's clipped bounds may hold, `4096 * 4096`. One
/// request then keeps at most 128 MiB of `f64` samples.
const REGION_SAMPLE_CAP: u64 = 4096 * 4096;
/// Most bounding-box samples times vertices for a freeform, `2^28`, because
/// `region_contains` is evaluated per sample and is linear in the vertices.
const FREEFORM_WORK_CAP: u64 = 1 << 28;
/// Most vertices a freeform may have.
pub const FREEFORM_POINT_CAP: usize = 4000;

/// Which measurement a request asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeasureShape {
    /// Two points.
    Length,
    /// Arm, vertex, arm.
    Angle,
    /// Two lines of two points each.
    Cobb,
    /// The long axis then the short axis, two points each.
    Bidirectional,
    /// Two opposite corners.
    Rectangle,
    /// Two opposite corners of the ellipse's box.
    Ellipse,
    /// Three to 4000 vertices, closed from the last to the first.
    Freeform,
}

impl MeasureShape {
    /// The shape named by an `X-Ocelli-Shape` token.
    #[must_use]
    pub fn from_token(text: &str) -> Option<Self> {
        Some(match text {
            "length" => Self::Length,
            "angle" => Self::Angle,
            "cobb" => Self::Cobb,
            "bidirectional" => Self::Bidirectional,
            "rectangle" => Self::Rectangle,
            "ellipse" => Self::Ellipse,
            "freeform" => Self::Freeform,
            _ => return None,
        })
    }

    /// The token, as the reply's `shape` field.
    #[must_use]
    pub const fn token(self) -> &'static str {
        match self {
            Self::Length => "length",
            Self::Angle => "angle",
            Self::Cobb => "cobb",
            Self::Bidirectional => "bidirectional",
            Self::Rectangle => "rectangle",
            Self::Ellipse => "ellipse",
            Self::Freeform => "freeform",
        }
    }

    /// Whether `count` points is the count this shape takes.
    #[must_use]
    pub const fn accepts(self, count: usize) -> bool {
        match self {
            Self::Length | Self::Rectangle | Self::Ellipse => count == 2,
            Self::Angle => count == 3,
            Self::Cobb | Self::Bidirectional => count == 4,
            Self::Freeform => count >= 3 && count <= FREEFORM_POINT_CAP,
        }
    }
}

/// One spacing pair as the file stores it, row then column.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum SpacingInput {
    /// The attribute is absent.
    Absent,
    /// The attribute is present and is not two decimals.
    Unusable,
    /// The stored numbers, nothing derived from them.
    Pair {
        /// Adjacent row spacing.
        row: f64,
        /// Adjacent column spacing.
        column: f64,
    },
}

/// One decimal as the file stores it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum FactorInput {
    /// The attribute is absent or has zero length.
    Absent,
    /// The attribute is present and is not one decimal.
    Unusable,
    /// The stored number.
    Value(f64),
}

/// The spacing attributes of one frame, read by `decode_held`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct CalibrationInputs {
    /// Pixel Spacing (0028,0030), from the Pixel Measures Sequence or the top level.
    pub(crate) pixel_spacing: SpacingInput,
    /// Imager Pixel Spacing (0018,1164), PS3.3 C.8.11.4.
    pub(crate) imager_spacing: SpacingInput,
    /// Estimated Radiographic Magnification Factor (0018,1114), PS3.3 C.8.7.5.
    pub(crate) magnification: FactorInput,
}

/// The spacing a request uses and how it is labelled.
#[derive(Debug, Clone, Copy)]
struct Spacing {
    /// `pixel-spacing`, `imager-spacing`, `detector` or `uncalibrated`.
    source: &'static str,
    /// Millimetres per row and per column, absent when uncalibrated.
    mm: Option<SpacingMm>,
    /// The length unit, `mm`, `mm at the detector` or `px`.
    length_unit: &'static str,
    /// The area unit.
    area_unit: &'static str,
}

/// Choose the spacing, the F-211 calibration table.
///
/// `select_calibration` decides whenever Pixel Spacing is present, and
/// whenever Imager Pixel Spacing comes with the factor. It also decides the
/// PS3.3 10.7.1.1 case, a Pixel Spacing equal to the Imager Pixel Spacing,
/// which takes the imager path: `detector` with no factor, and refused when
/// a present factor cannot be used, as without Pixel Spacing.
/// Imager Pixel Spacing alone, with no Pixel Spacing and no factor, is this
/// example's own `detector` row, which keeps the raw pair. No spacing at all
/// is `uncalibrated`. A present source that is chosen and cannot be used
/// refuses.
///
/// F-195 takes the first present source, so the inputs are read in that
/// order and only as far as the source that is chosen. A present Pixel
/// Spacing that differs from the imager pair decides alone, and a later
/// Imager Pixel Spacing or factor that cannot be used is never read.
fn choose_spacing(inputs: &CalibrationInputs) -> Result<Spacing, PreviewError> {
    let pair = |input: SpacingInput| match input {
        SpacingInput::Absent => Ok(None),
        SpacingInput::Unusable => Err(PreviewError::Calibration),
        SpacingInput::Pair { row, column } => Ok(Some(SpacingMm { row, column })),
    };
    let pixel_spacing = pair(inputs.pixel_spacing)?;
    let (imager_spacing, factor) = if pixel_spacing.is_some() {
        // Only an equal imager pair can change the choice, and only then is
        // the factor read. An unparseable factor goes in as a present
        // non-finite one, which `select_calibration` refuses on the imager
        // path and never reads beside a Pixel Spacing that differs.
        let imager = match inputs.imager_spacing {
            SpacingInput::Pair { row, column } => Some(SpacingMm { row, column }),
            SpacingInput::Absent | SpacingInput::Unusable => None,
        };
        let factor = match inputs.magnification {
            FactorInput::Value(value) => Some(value),
            FactorInput::Unusable => Some(f64::NAN),
            FactorInput::Absent => None,
        };
        (imager, factor)
    } else {
        let factor = match inputs.magnification {
            FactorInput::Absent => None,
            FactorInput::Unusable => return Err(PreviewError::Calibration),
            FactorInput::Value(value) => Some(value),
        };
        (pair(inputs.imager_spacing)?, factor)
    };
    if pixel_spacing.is_none()
        && factor.is_none()
        && let Some(raw) = imager_spacing
    {
        if !raw.row.is_finite() || !raw.column.is_finite() || raw.row <= 0.0 || raw.column <= 0.0 {
            return Err(PreviewError::Calibration);
        }
        return Ok(detector(raw));
    }
    if pixel_spacing.is_none() && imager_spacing.is_none() && factor.is_none() {
        return Ok(Spacing {
            source: "uncalibrated",
            mm: None,
            length_unit: "px",
            area_unit: "px²",
        });
    }
    let chosen = select_calibration(CalibrationRequest {
        pixel_spacing,
        imager_spacing,
        estimated_radiographic_magnification_factor: factor,
        ultrasound_delta: None,
        manual: None,
    })
    .map_err(|_| PreviewError::Calibration)?;
    match chosen.source {
        CalibrationSource::PixelSpacing | CalibrationSource::ImagerSpacing => Ok(Spacing {
            source: chosen.source.name(),
            mm: Some(chosen.spacing),
            length_unit: "mm",
            area_unit: "mm²",
        }),
        CalibrationSource::Detector => Ok(detector(chosen.spacing)),
        CalibrationSource::Ultrasound | CalibrationSource::Manual => Err(PreviewError::Calibration),
    }
}

/// Detector-plane spacing, not corrected for magnification.
fn detector(raw: SpacingMm) -> Spacing {
    Spacing {
        source: CalibrationSource::Detector.name(),
        mm: Some(raw),
        length_unit: "mm at the detector",
        area_unit: "mm² at the detector",
    }
}

/// Decode one frame and measure one shape on it.
///
/// This is the composition the `/measure` route runs on a frame it does not
/// hold yet: the same decode as `preview_frame`, then the same measurement
/// of the held frame.
///
/// # Errors
///
/// Returns [`PreviewError`] when the file or the frame cannot be read, the
/// spacing cannot be used, the shape is degenerate, crosses itself or is too
/// large, or a sample has no modality value. A refusal returns no number.
pub fn measure_frame(
    bytes: &[u8],
    frame: u32,
    shape: MeasureShape,
    points: &[ImagePoint],
) -> Result<String, PreviewError> {
    let mut native = Vec::new();
    let mut samples = Vec::new();
    let held = decode_held(
        bytes,
        frame,
        &mut native,
        &mut samples,
        &mut |_| {},
        &mut |_| Vec::new(),
    )?;
    measure_json(&held, frame, shape, points)
}

/// One measurement of one held frame, as one JSON object.
///
/// # Errors
///
/// In order: [`PreviewError::Degenerate`] for a wrong point count, the
/// held calibration refusal, then [`PreviewError::Calibration`] for a
/// present spacing that cannot be used. A line shape is then
/// [`PreviewError::Degenerate`] for a zero-length angle arm or Cobb line. A
/// region is [`PreviewError::Degenerate`] for a zero radius or no sample in
/// the image, [`PreviewError::TooLarge`] past the sample caps,
/// [`PreviewError::SelfIntersecting`] for a crossing freeform, then the held
/// value refusal, and [`PreviewError::Degenerate`] when every sample is
/// padding. [`PreviewError::Point`] when a coordinate is too large to measure.
pub(crate) fn measure_json(
    held: &HeldFrame,
    frame: u32,
    shape: MeasureShape,
    points: &[ImagePoint],
) -> Result<String, PreviewError> {
    if !shape.accepts(points.len()) {
        return Err(PreviewError::Degenerate);
    }
    let inputs = held.calibration.as_ref().map_err(|error| *error)?;
    let spacing = choose_spacing(inputs)?;
    // The half-pixel shift, once. Every later point is an index point.
    let index: Vec<Pt<Index>> = points
        .iter()
        .map(|point| Pt::new(point.x - 0.5, point.y - 0.5, 0.0))
        .collect();
    let mut out = String::from("{");
    push_field(&mut out, "shape", shape.token(), true);
    out.push_str(",\"frame\":");
    out.push_str(&frame.to_string());
    out.push_str(",\"calibration\":{");
    push_field(&mut out, "source", spacing.source, true);
    if let Some(mm) = spacing.mm {
        push_number(&mut out, "row_mm", mm.row);
        push_number(&mut out, "column_mm", mm.column);
    }
    out.push('}');
    match shape {
        MeasureShape::Length
        | MeasureShape::Angle
        | MeasureShape::Cobb
        | MeasureShape::Bidirectional => {
            let lines = match spacing.mm {
                Some(mm) => line_numbers(shape, &millimetres(&index, mm))?,
                None => line_numbers(shape, &index)?,
            };
            push_lines(&mut out, &lines, &spacing);
        }
        MeasureShape::Rectangle | MeasureShape::Ellipse | MeasureShape::Freeform => {
            let area = match spacing.mm {
                Some(mm) => shape_area(shape, &millimetres(&index, mm))?,
                None => shape_area(shape, &index)?,
            };
            let region = region_of(shape, &index)?;
            let stats = region_statistics(held, &region, area)?;
            push_measure(
                &mut out,
                "area",
                area,
                spacing.area_unit,
                &format!("{area:.2} {}", spacing.area_unit),
            );
            push_stats(&mut out, &stats);
        }
    }
    out.push('}');
    Ok(out)
}

/// Plane-local millimetre points, `(c * column_mm, r * row_mm, 0)`.
///
/// This is not LPS. By PS3.3 C.7.6.2.1.1, for orthonormal row and column
/// directions, the plane scaled this way is a rigid motion away from LPS,
/// so a distance, an angle or an area between these points is the patient one.
fn millimetres(index: &[Pt<Index>], mm: SpacingMm) -> Vec<Pt<World>> {
    index
        .iter()
        .map(|point| Pt::new(point.x * mm.column, point.y * mm.row, 0.0))
        .collect()
}

/// The numbers of a line shape, in the units of `S`.
enum LineNumbers {
    /// One length.
    Length(f64),
    /// One angle in radians.
    Angle(f64),
    /// The longer and the shorter of two lengths.
    Bidirectional { length: f64, width: f64 },
}

fn line_numbers<S>(shape: MeasureShape, points: &[Pt<S>]) -> Result<LineNumbers, PreviewError> {
    let at = |index: usize| points.get(index).copied().ok_or(PreviewError::Degenerate);
    Ok(match shape {
        MeasureShape::Length => LineNumbers::Length(length(points).map_err(geom)?),
        MeasureShape::Angle => LineNumbers::Angle(angle_at(at(0)?, at(1)?, at(2)?).map_err(geom)?),
        MeasureShape::Cobb => {
            LineNumbers::Angle(cobb(at(0)?, at(1)?, at(2)?, at(3)?).map_err(geom)?)
        }
        MeasureShape::Bidirectional => {
            let pair = bidirectional(at(0)?, at(1)?, at(2)?, at(3)?).map_err(geom)?;
            LineNumbers::Bidirectional {
                length: pair.length,
                width: pair.width,
            }
        }
        MeasureShape::Rectangle | MeasureShape::Ellipse | MeasureShape::Freeform => {
            return Err(PreviewError::Degenerate);
        }
    })
}

fn push_lines(out: &mut String, lines: &LineNumbers, spacing: &Spacing) {
    let unit = spacing.length_unit;
    match *lines {
        LineNumbers::Length(value) => {
            push_measure(out, "length", value, unit, &format!("{value:.1} {unit}"));
        }
        LineNumbers::Angle(radians) => {
            let degrees = radians.to_degrees();
            let text = if spacing.mm.is_some() {
                format!("{degrees:.1}°")
            } else {
                format!("{degrees:.1}°, square pixels assumed")
            };
            push_measure(out, "angle", degrees, "°", &text);
        }
        LineNumbers::Bidirectional { length, width } => {
            push_measure(out, "length", length, unit, &format!("{length:.1} {unit}"));
            push_measure(out, "width", width, unit, &format!("{width:.1} {unit}"));
        }
    }
}

/// The area of the drawn shape, from the `ocelli-geom` area of that shape.
fn shape_area<S>(shape: MeasureShape, points: &[Pt<S>]) -> Result<f64, PreviewError> {
    let at = |index: usize| points.get(index).copied().ok_or(PreviewError::Degenerate);
    match shape {
        MeasureShape::Rectangle => Ok(rectangle(at(0)?, at(1)?).map_err(geom)?.area),
        MeasureShape::Ellipse => {
            let (a, b) = (at(0)?, at(1)?);
            ellipse((b.x - a.x) / 2.0, (b.y - a.y) / 2.0).map_err(geom)
        }
        MeasureShape::Freeform => polygon_area(points).map_err(geom),
        MeasureShape::Length
        | MeasureShape::Angle
        | MeasureShape::Cobb
        | MeasureShape::Bidirectional => Err(PreviewError::Degenerate),
    }
}

/// The inclusion shape on index points.
fn region_of(shape: MeasureShape, index: &[Pt<Index>]) -> Result<RegionShape<'_>, PreviewError> {
    let at = |at: usize| index.get(at).copied().ok_or(PreviewError::Degenerate);
    match shape {
        MeasureShape::Rectangle => Ok(RegionShape::Rectangle {
            corner: at(0)?,
            opposite: at(1)?,
        }),
        MeasureShape::Ellipse => {
            let (a, b) = (at(0)?, at(1)?);
            Ok(RegionShape::Ellipse {
                center: Pt::new((a.x + b.x) / 2.0, (a.y + b.y) / 2.0, 0.0),
                rx: (b.x - a.x) / 2.0,
                ry: (b.y - a.y) / 2.0,
            })
        }
        MeasureShape::Freeform => Ok(RegionShape::Polygon { points: index }),
        MeasureShape::Length
        | MeasureShape::Angle
        | MeasureShape::Cobb
        | MeasureShape::Bidirectional => Err(PreviewError::Degenerate),
    }
}

/// The F-075 summary of a region's non-padding modality values.
struct RegionStats {
    count: u32,
    padding_excluded: u32,
    unit: String,
    mean: f64,
    sd: f64,
    min: f32,
    max: f32,
}

/// Walk the clipped bounds once, keep each contained non-padding sample.
///
/// PS3.3 C.7.5.1.1.2: a padding sample is compared in stored space and is
/// not a modality value, so it is excluded and counted. PS3.3 C.11.1: each
/// kept sample is `ModalityTransform::apply` of the stored value, the held
/// frame's stage 1, widened from `f32` to `f64` without loss.
fn region_statistics(
    held: &HeldFrame,
    region: &RegionShape<'_>,
    area: f64,
) -> Result<RegionStats, PreviewError> {
    let Some(bounds) = region_bounds(region, held.width, held.height).map_err(geom)? else {
        return Err(PreviewError::Degenerate);
    };
    let columns = u64::from(bounds.last_column - bounds.first_column) + 1;
    let rows = u64::from(bounds.last_row - bounds.first_row) + 1;
    let box_samples = columns.saturating_mul(rows);
    if box_samples > REGION_SAMPLE_CAP {
        return Err(PreviewError::TooLarge);
    }
    if let RegionShape::Polygon { points } = region {
        let vertices = u64::try_from(points.len()).map_err(|_| PreviewError::TooLarge)?;
        if box_samples.saturating_mul(vertices) > FREEFORM_WORK_CAP {
            return Err(PreviewError::TooLarge);
        }
        // The shoelace area and the even-odd samples disagree on a ring
        // that crosses itself, so it is refused before any sample is read.
        if polygon_crosses_itself(points).map_err(geom)? {
            return Err(PreviewError::SelfIntersecting);
        }
    }
    let facts = held.value.as_ref().map_err(|error| *error)?;
    let capacity = usize::try_from(box_samples).map_err(|_| PreviewError::TooLarge)?;
    let mut samples: Vec<f64> = Vec::with_capacity(capacity);
    let mut padding_excluded = 0_u32;
    for row in bounds.first_row..=bounds.last_row {
        for column in bounds.first_column..=bounds.last_column {
            if !region_contains(region, column, row).map_err(geom)? {
                continue;
            }
            let stored = stored_at(held, column, row)?;
            if let Some(range) = facts.padding()
                && range.contains(stored_integer(stored)?)
            {
                padding_excluded = padding_excluded.saturating_add(1);
                continue;
            }
            let value = facts.modality.apply(stored).0;
            if !value.is_finite() {
                return Err(PreviewError::Attribute);
            }
            samples.push(f64::from(value));
        }
    }
    let stats = roi_statistics(&samples, area).map_err(|error| match error {
        GeomError::Degenerate => PreviewError::Degenerate,
        GeomError::Frame => PreviewError::Attribute,
    })?;
    Ok(RegionStats {
        count: stats.count,
        padding_excluded,
        unit: facts.unit().to_owned(),
        mean: stats.mean,
        sd: stats.sd,
        // Each extreme is one sample widened from `f32`, so narrowing is exact.
        min: f32_from_f64(stats.min),
        max: f32_from_f64(stats.max),
    })
}

fn push_stats(out: &mut String, stats: &RegionStats) {
    let unit = stats.unit.as_str();
    out.push_str(",\"stats\":{\"count\":");
    out.push_str(&stats.count.to_string());
    out.push_str(",\"padding_excluded\":");
    out.push_str(&stats.padding_excluded.to_string());
    push_field(out, "unit", unit, false);
    push_number(out, "mean", stats.mean);
    push_number(out, "sd", stats.sd);
    // Rust's `Display` for `f32` is the shortest text that reads back as
    // the same value, and it never uses an exponent.
    out.push_str(",\"min\":");
    out.push_str(&stats.min.to_string());
    out.push_str(",\"max\":");
    out.push_str(&stats.max.to_string());
    let text = format!(
        "mean {mean:.1} {unit}, SD {sd:.1} {unit}, min {min} {unit}, max {max} {unit}",
        mean = stats.mean,
        sd = stats.sd,
        min = stats.min,
        max = stats.max,
    );
    out.push_str(",\"text\":");
    json_string(out, &text);
    out.push('}');
}

/// `,"name":{"value":..,"unit":..,"text":..}`.
fn push_measure(out: &mut String, name: &str, value: f64, unit: &str, text: &str) {
    out.push_str(",\"");
    out.push_str(name);
    out.push_str("\":{\"value\":");
    out.push_str(&value.to_string());
    push_field(out, "unit", unit, false);
    push_field(out, "text", text, false);
    out.push('}');
}

/// `,"name":value` for a finite `f64`. `Display` never uses an exponent.
fn push_number(out: &mut String, name: &str, value: f64) {
    out.push_str(",\"");
    out.push_str(name);
    out.push_str("\":");
    out.push_str(&value.to_string());
}

/// A geometry refusal on finite page points.
///
/// `Degenerate` is a zero length, radius or region. `Frame` can only be an
/// overflow, since the route parsed finite numbers, so it is the point.
fn geom(error: GeomError) -> PreviewError {
    match error {
        GeomError::Degenerate => PreviewError::Degenerate,
        GeomError::Frame => PreviewError::Point,
    }
}
