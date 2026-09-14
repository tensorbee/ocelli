//! Derived per-frame geometry over the F-019 functional-group projection.
//!
//! F-019 retained plane position, plane orientation and pixel measures without
//! interpreting them. This module is the first in the programme that computes a
//! coordinate from a tag rather than retaining one, so every derivation cites
//! the DICOM PS3.3 section it implements and a value that cannot be derived is
//! refused rather than defaulted.
//!
//! The arithmetic itself is not here. `ocelli-pixel` owns `ImagePlane` and
//! DICOM PS3.3 C.7.6.2.1.1's index-to-world transform, and this module assembles
//! validated attributes into those existing types.

use dicom_core::{Tag, VR};
use ocelli_core::{Index, Transform, World};
use ocelli_pixel::{
    ImageDimensions, ImageOrientationPatient, ImagePlane, ImagePositionPatient, PixelError,
    PixelSpacing,
};

use crate::{
    DuplicateSources, FunctionalGroupSource, MetadataElement, MetadataSet, MetadataValue,
    MultiframeError, MultiframeMetadata, TopLevelFallback,
};

const ROWS: Tag = Tag(0x0028, 0x0010);
const COLUMNS: Tag = Tag(0x0028, 0x0011);
const GANTRY_DETECTOR_TILT: Tag = Tag(0x0018, 0x1120);
const SLICE_THICKNESS: Tag = Tag(0x0018, 0x0050);
const SPACING_BETWEEN_SLICES: Tag = Tag(0x0018, 0x0088);
const IMAGER_PIXEL_SPACING: Tag = Tag(0x0018, 0x1164);
const PIXEL_SPACING: Tag = Tag(0x0028, 0x0030);
const IMAGE_POSITION_PATIENT: Tag = Tag(0x0020, 0x0032);
const IMAGE_ORIENTATION_PATIENT: Tag = Tag(0x0020, 0x0037);
const PIXEL_MEASURES_SEQUENCE: Tag = Tag(0x0028, 0x9110);
const PLANE_POSITION_SEQUENCE: Tag = Tag(0x0020, 0x9113);
const PLANE_ORIENTATION_SEQUENCE: Tag = Tag(0x0020, 0x9116);

/// DICOM PS3.5 section 6.2 caps a Decimal String at sixteen bytes, padding
/// included.
const DECIMAL_STRING_MAX_BYTES: usize = 16;

/// Millimetres. The off-axis component of an inter-frame step must exceed this
/// before a stack is called sheared, and two projected steps must agree within
/// it before a stack is called uniformly spaced.
///
/// The figure is below the precision the source attribute can express, which is
/// the property wanted: it must absorb `f64` arithmetic noise and never mask a
/// difference a file could actually state. Image Position Patient arrives as a
/// Decimal String, which DICOM PS3.5 section 6.2 caps at sixteen characters. A
/// patient coordinate routinely carries three or four digits before the point,
/// so the attribute can express about eleven decimal places at the very most and
/// real files carry four to six.
///
/// It is chosen once, here. Changing it is a design-plan decision under
/// `.claude/WORKFLOW.md`'s tolerance policy, not a fix for a failing case.
const GEOMETRY_TOLERANCE_MM: f64 = 1e-6;

/// Millimetres. Two spacing values that arrive as decimal text agree exactly as
/// text in the ordinary case, so this only absorbs a differing number of
/// trailing zeros. It is not a physical tolerance.
const SPACING_EQUALITY_MM: f64 = 1e-9;

/// A derivation that cannot be performed from the evidence present.
///
/// Variants retain tags and counts only. No attribute value is ever copied into
/// an error, which is the constraint [`MultiframeError`] already holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum FrameGeometryError {
    /// Image Position Patient resolves to nothing for this frame.
    #[error("Image Position Patient is absent")]
    MissingImagePosition,
    /// Image Orientation Patient resolves to nothing for this frame.
    #[error("Image Orientation Patient is absent")]
    MissingImageOrientation,
    /// Neither Pixel Spacing nor Imager Pixel Spacing is present.
    #[error("Pixel Spacing is absent")]
    MissingPixelSpacing,
    /// Only Imager Pixel Spacing is present, which is detector-plane spacing.
    #[error("only Imager Pixel Spacing is present, which is not image spacing")]
    UncalibratedSpacingOnly,
    /// Rows or Columns is absent or not a single US value.
    #[error("Rows or Columns is absent or malformed")]
    InvalidDimensionsAttribute,
    /// A Decimal String does not follow DICOM PS3.5 section 6.2.
    #[error("attribute is not a conforming Decimal String")]
    InvalidDecimalString {
        /// The attribute that failed to parse.
        tag: Tag,
    },
    /// A Decimal String has the wrong number of values.
    #[error("attribute has the wrong value multiplicity")]
    DecimalStringMultiplicity {
        /// The attribute whose multiplicity is wrong.
        tag: Tag,
        /// The multiplicity DICOM PS3.6 assigns.
        expected: usize,
        /// The multiplicity present.
        actual: usize,
    },
    /// An attribute's Value Representation is not the one PS3.6 assigns.
    #[error("attribute does not use the Value Representation PS3.6 assigns")]
    InvalidVr {
        /// The attribute whose VR is wrong.
        tag: Tag,
        /// The VR found.
        found: VR,
    },
    /// `ocelli-pixel` refused the assembled plane.
    #[error("assembled image plane is invalid: {0:?}")]
    Plane(PixelError),
    /// Projected inter-frame steps are not equal within tolerance.
    #[error("inter-frame spacing is not uniform")]
    NonUniformFrameSpacing,
    /// Functional-group resolution itself failed.
    #[error("multiframe resolution failed: {0}")]
    Multiframe(#[from] MultiframeError),
}

/// How Pixel Spacing relates to Imager Pixel Spacing, DICOM PS3.3 C.7.6.1.1.5.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpacingRelationship {
    /// Only Pixel Spacing is present.
    ImageOnly,
    /// Both are present and equal, so no calibration is implied.
    ImagerAgrees,
    /// Both are present and differ, so Pixel Spacing has been calibrated.
    Calibrated,
}

/// Calibrated and uncalibrated spacing, both retained.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SpacingEvidence {
    image_mm: PixelSpacing,
    imager_mm: Option<PixelSpacing>,
    relationship: SpacingRelationship,
}

impl SpacingEvidence {
    /// Pixel Spacing `(0028,0030)`, the spacing that applies to the image.
    #[must_use]
    pub const fn image_mm(&self) -> PixelSpacing {
        self.image_mm
    }

    /// Imager Pixel Spacing `(0018,1164)`, detector-plane spacing.
    ///
    /// Retained as evidence. It is never substituted for [`Self::image_mm`],
    /// because on a projection radiograph the two differ by the
    /// source-to-image magnification factor.
    #[must_use]
    pub const fn imager_mm(&self) -> Option<PixelSpacing> {
        self.imager_mm
    }

    /// Which of PS3.3 C.7.6.1.1.5's cases this frame is in.
    #[must_use]
    pub const fn relationship(&self) -> SpacingRelationship {
        self.relationship
    }
}

/// Which functional-group source answered each derived attribute.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameGeometrySources {
    position: FunctionalGroupSource,
    orientation: FunctionalGroupSource,
    pixel_spacing: FunctionalGroupSource,
    duplicates: DuplicateSources,
}

impl FrameGeometrySources {
    /// The source that answered Image Position Patient.
    #[must_use]
    pub const fn position(&self) -> FunctionalGroupSource {
        self.position
    }

    /// The source that answered Image Orientation Patient.
    #[must_use]
    pub const fn orientation(&self) -> FunctionalGroupSource {
        self.orientation
    }

    /// The source that answered Pixel Spacing.
    #[must_use]
    pub const fn pixel_spacing(&self) -> FunctionalGroupSource {
        self.pixel_spacing
    }

    /// Lower-precedence sources that also declared one of the three.
    ///
    /// DICOM PS3.3 C.7.6.16.1.1 makes a duplicate declaration malformed. F-019
    /// chose to retain it observably rather than refuse, and this follows that
    /// choice so one instance cannot parse under one module and fail under the
    /// other.
    #[must_use]
    pub const fn duplicates(&self) -> DuplicateSources {
        self.duplicates
    }
}

/// Derived, validated geometry for one zero-based frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrameGeometry {
    plane: ImagePlane,
    spacing: SpacingEvidence,
    sources: FrameGeometrySources,
    slice_thickness_mm: Option<f64>,
    spacing_between_slices_mm: Option<f64>,
}

impl FrameGeometry {
    /// Derive one frame's plane from the functional-group projection.
    ///
    /// Image Position Patient, Image Orientation Patient and Pixel Spacing are
    /// resolved per-frame first, then shared, then at the top level, which is
    /// DICOM PS3.3 C.7.6.16.1.1's order. The top-level fallback is permitted
    /// because a legacy single-frame instance carries all three there and has
    /// no functional groups at all.
    ///
    /// Imager Pixel Spacing and Gantry/Detector Tilt are read from the top
    /// level only. Neither belongs to a functional group macro.
    ///
    /// # Errors
    ///
    /// Returns [`FrameGeometryError`] for an absent or malformed attribute, a
    /// frame outside the declared count, or a plane `ocelli-pixel` refuses. No
    /// route substitutes a default.
    pub fn derive(
        metadata: &MultiframeMetadata<'_>,
        frame: usize,
    ) -> Result<Self, FrameGeometryError> {
        let dimensions = image_dimensions(metadata.metadata())?;

        let position = resolve(
            metadata,
            frame,
            PLANE_POSITION_SEQUENCE,
            IMAGE_POSITION_PATIENT,
        )?
        .ok_or(FrameGeometryError::MissingImagePosition)?;
        let orientation = resolve(
            metadata,
            frame,
            PLANE_ORIENTATION_SEQUENCE,
            IMAGE_ORIENTATION_PATIENT,
        )?
        .ok_or(FrameGeometryError::MissingImageOrientation)?;
        let pixel_spacing = resolve(metadata, frame, PIXEL_MEASURES_SEQUENCE, PIXEL_SPACING)?;

        let imager_mm = imager_pixel_spacing(metadata.metadata())?;
        // DICOM PS3.3 C.7.6.1.1.5. Imager Pixel Spacing is spacing at the
        // detector plane. Substituting it for image spacing scales every
        // measurement by the source-to-image magnification factor on an image
        // that still renders correctly, so the caller decides rather than us.
        let Some(pixel_spacing) = pixel_spacing else {
            return Err(if imager_mm.is_some() {
                FrameGeometryError::UncalibratedSpacingOnly
            } else {
                FrameGeometryError::MissingPixelSpacing
            });
        };
        let spacing = spacing_evidence(pixel_spacing.element(), imager_mm)?;

        let ipp = decimal_values::<3>(position.element(), IMAGE_POSITION_PATIENT)?;
        let iop = decimal_values::<6>(orientation.element(), IMAGE_ORIENTATION_PATIENT)?;

        let plane = ImagePlane::new(
            ImagePositionPatient::new(ipp).map_err(FrameGeometryError::Plane)?,
            ImageOrientationPatient::new([iop[0], iop[1], iop[2]], [iop[3], iop[4], iop[5]])
                .map_err(FrameGeometryError::Plane)?,
            spacing.image_mm,
            dimensions,
        )
        .map_err(FrameGeometryError::Plane)?;

        let duplicates = position
            .duplicates()
            .union(orientation.duplicates())
            .union(pixel_spacing.duplicates());

        Ok(Self {
            plane,
            spacing,
            sources: FrameGeometrySources {
                position: position.source(),
                orientation: orientation.source(),
                pixel_spacing: pixel_spacing.source(),
                duplicates,
            },
            slice_thickness_mm: optional_single_decimal(
                metadata,
                frame,
                PIXEL_MEASURES_SEQUENCE,
                SLICE_THICKNESS,
            )?,
            spacing_between_slices_mm: optional_single_decimal(
                metadata,
                frame,
                PIXEL_MEASURES_SEQUENCE,
                SPACING_BETWEEN_SLICES,
            )?,
        })
    }

    /// The validated plane, owned by `ocelli-pixel`.
    #[must_use]
    pub const fn plane(&self) -> ImagePlane {
        self.plane
    }

    /// DICOM PS3.3 C.7.6.2.1.1's index-to-patient transform.
    ///
    /// Implemented once, in `ocelli-pixel`. This forwards rather than
    /// re-deriving it.
    #[must_use]
    pub fn index_to_world(&self) -> Transform<Index, World> {
        self.plane.index_to_world()
    }

    /// Calibrated and uncalibrated spacing evidence.
    #[must_use]
    pub const fn spacing(&self) -> SpacingEvidence {
        self.spacing
    }

    /// Which source answered each derived attribute.
    #[must_use]
    pub const fn sources(&self) -> FrameGeometrySources {
        self.sources
    }

    /// Slice Thickness `(0018,0050)`, retained as evidence.
    ///
    /// DICOM PS3.3 C.7.6.2 makes this the reconstructed slab thickness, which
    /// may overlap or gap. It is never used as inter-frame spacing.
    #[must_use]
    pub const fn slice_thickness_mm(&self) -> Option<f64> {
        self.slice_thickness_mm
    }

    /// Spacing Between Slices `(0018,0088)`, retained as evidence.
    ///
    /// Frequently absent and frequently wrong. The projected Image Position
    /// Patient difference is the ground truth, and [`StackGeometry`] derives it.
    #[must_use]
    pub const fn spacing_between_slices_mm(&self) -> Option<f64> {
        self.spacing_between_slices_mm
    }
}

/// Whether a multiframe instance's frames form an axis-aligned stack.
///
/// Measured from DICOM PS3.3 C.7.6.2.1.1's geometry, never from
/// Gantry/Detector Tilt `(0018,1120)`, which is Type 3 and nominal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StackShear {
    /// Every inter-frame step is parallel to the slice normal.
    AxisAligned {
        /// The signed projected step in millimetres, so a descending stack is
        /// negative.
        step_mm: f64,
    },
    /// At least one step is not parallel, so the volume is a sheared
    /// parallelepiped rather than a box.
    Sheared {
        /// The largest off-axis component in millimetres.
        max_offaxis_mm: f64,
    },
    /// Fewer than two frames, so there is no inter-frame vector at all.
    NotApplicable,
}

/// One instance's cross-frame geometry.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StackGeometry {
    shear: StackShear,
    nominal_tilt_degrees: Option<f64>,
}

impl StackGeometry {
    /// Measure the stack's shear from every frame's derived plane.
    ///
    /// # Errors
    ///
    /// Propagates [`FrameGeometry::derive`]'s refusals, and returns
    /// [`FrameGeometryError::NonUniformFrameSpacing`] when the projected steps
    /// of an otherwise axis-aligned stack are not equal within tolerance.
    /// Non-uniform spacing is real in dose-modulated and multi-slab
    /// acquisitions, and rendering it as uniform distorts geometry in a way no
    /// measurement tool flags, so it is refused rather than averaged.
    pub fn derive(metadata: &MultiframeMetadata<'_>) -> Result<Self, FrameGeometryError> {
        let nominal_tilt_degrees =
            top_level_single_decimal(metadata.metadata(), GANTRY_DETECTOR_TILT)?;
        let frame_count = metadata.frame_count();
        if frame_count < 2 {
            // Derive frame zero anyway, so a malformed single-frame instance is
            // still refused here rather than reported as having no shear.
            FrameGeometry::derive(metadata, 0)?;
            return Ok(Self {
                shear: StackShear::NotApplicable,
                nominal_tilt_degrees,
            });
        }

        let first = FrameGeometry::derive(metadata, 0)?;
        let normal = first.plane.slice_normal();
        let mut previous = first.plane.position_vector();
        let mut first_step_mm: Option<f64> = None;
        let mut max_offaxis_mm = 0.0_f64;
        let mut uniform = true;

        for frame in 1..frame_count {
            let current = FrameGeometry::derive(metadata, frame)?
                .plane
                .position_vector();
            let step = current - previous;
            let projected = step.dot(normal);
            let offaxis = (step - normal * projected).length();
            if offaxis > max_offaxis_mm {
                max_offaxis_mm = offaxis;
            }
            match first_step_mm {
                None => first_step_mm = Some(projected),
                Some(first) if (projected - first).abs() > GEOMETRY_TOLERANCE_MM => {
                    uniform = false;
                }
                Some(_) => {}
            }
            previous = current;
        }

        if max_offaxis_mm > GEOMETRY_TOLERANCE_MM {
            return Ok(Self {
                shear: StackShear::Sheared { max_offaxis_mm },
                nominal_tilt_degrees,
            });
        }

        // No step at all means no inter-frame vector, which is exactly what
        // `NotApplicable` says. It is unreachable below a frame count of two,
        // and naming it here keeps every arm a true statement.
        let Some(step_mm) = first_step_mm else {
            return Ok(Self {
                shear: StackShear::NotApplicable,
                nominal_tilt_degrees,
            });
        };
        if !uniform {
            return Err(FrameGeometryError::NonUniformFrameSpacing);
        }

        Ok(Self {
            shear: StackShear::AxisAligned { step_mm },
            nominal_tilt_degrees,
        })
    }

    /// The measured verdict.
    #[must_use]
    pub const fn shear(&self) -> StackShear {
        self.shear
    }

    /// Gantry/Detector Tilt `(0018,1120)`, retained and never used.
    ///
    /// DICOM PS3.3 C.8.7.3.1.1 calls it the nominal angle of tilt, and it is
    /// Type 3. The geometry is the measurement and the tag is evidence.
    #[must_use]
    pub const fn nominal_tilt_degrees(&self) -> Option<f64> {
        self.nominal_tilt_degrees
    }
}

/// One resolved attribute plus the sources behind it.
#[derive(Clone, Copy)]
struct Resolved<'a> {
    element: &'a MetadataElement,
    source: FunctionalGroupSource,
    duplicates: DuplicateSources,
}

impl<'a> Resolved<'a> {
    const fn element(self) -> &'a MetadataElement {
        self.element
    }

    const fn source(self) -> FunctionalGroupSource {
        self.source
    }

    const fn duplicates(self) -> DuplicateSources {
        self.duplicates
    }
}

fn resolve<'a>(
    metadata: &MultiframeMetadata<'a>,
    frame: usize,
    group_tag: Tag,
    attribute_tag: Tag,
) -> Result<Option<Resolved<'a>>, FrameGeometryError> {
    Ok(metadata
        .resolve(frame, group_tag, attribute_tag, TopLevelFallback::Allowed)?
        .map(|resolved| Resolved {
            element: resolved.element(),
            source: resolved.source(),
            duplicates: resolved.duplicate_sources(),
        }))
}

fn image_dimensions(metadata: &MetadataSet) -> Result<ImageDimensions, FrameGeometryError> {
    let rows = unsigned_short(metadata, ROWS)?;
    let columns = unsigned_short(metadata, COLUMNS)?;
    ImageDimensions::new(u32::from(rows), u32::from(columns)).map_err(FrameGeometryError::Plane)
}

fn unsigned_short(metadata: &MetadataSet, tag: Tag) -> Result<u16, FrameGeometryError> {
    let element = metadata
        .get(tag)
        .ok_or(FrameGeometryError::InvalidDimensionsAttribute)?;
    if element.vr() != VR::US {
        return Err(FrameGeometryError::InvalidVr {
            tag,
            found: element.vr(),
        });
    }
    let MetadataValue::Unsigned16(values) = element.value() else {
        return Err(FrameGeometryError::InvalidDimensionsAttribute);
    };
    match values.as_slice() {
        [value] => Ok(*value),
        _ => Err(FrameGeometryError::InvalidDimensionsAttribute),
    }
}

/// Imager Pixel Spacing `(0018,1164)`, which belongs to no functional group
/// macro and is therefore read from the main data set only.
fn imager_pixel_spacing(
    metadata: &MetadataSet,
) -> Result<Option<PixelSpacing>, FrameGeometryError> {
    match metadata.get(IMAGER_PIXEL_SPACING) {
        Some(element) => Ok(Some(pixel_spacing_pair(element, IMAGER_PIXEL_SPACING)?)),
        None => Ok(None),
    }
}

fn spacing_evidence(
    element: &MetadataElement,
    imager: Option<PixelSpacing>,
) -> Result<SpacingEvidence, FrameGeometryError> {
    let image_mm = pixel_spacing_pair(element, PIXEL_SPACING)?;

    let relationship = match imager {
        None => SpacingRelationship::ImageOnly,
        Some(imager)
            if (imager.row_mm() - image_mm.row_mm()).abs() <= SPACING_EQUALITY_MM
                && (imager.column_mm() - image_mm.column_mm()).abs() <= SPACING_EQUALITY_MM =>
        {
            SpacingRelationship::ImagerAgrees
        }
        Some(_) => SpacingRelationship::Calibrated,
    };

    Ok(SpacingEvidence {
        image_mm,
        imager_mm: imager,
        relationship,
    })
}

fn pixel_spacing_pair(
    element: &MetadataElement,
    tag: Tag,
) -> Result<PixelSpacing, FrameGeometryError> {
    // DICOM PS3.3 C.7.6.2: Pixel Spacing is [between ROWS, between COLUMNS].
    let values = decimal_values::<2>(element, tag)?;
    PixelSpacing::new(values[0], values[1]).map_err(FrameGeometryError::Plane)
}

fn optional_single_decimal(
    metadata: &MultiframeMetadata<'_>,
    frame: usize,
    group_tag: Tag,
    attribute_tag: Tag,
) -> Result<Option<f64>, FrameGeometryError> {
    let Some(resolved) = resolve(metadata, frame, group_tag, attribute_tag)? else {
        return Ok(None);
    };
    Ok(Some(
        decimal_values::<1>(resolved.element(), attribute_tag)?[0],
    ))
}

fn top_level_single_decimal(
    metadata: &MetadataSet,
    tag: Tag,
) -> Result<Option<f64>, FrameGeometryError> {
    let Some(element) = metadata.get(tag) else {
        return Ok(None);
    };
    Ok(Some(decimal_values::<1>(element, tag)?[0]))
}

/// Read exactly `N` Decimal String values from one attribute.
fn decimal_values<const N: usize>(
    element: &MetadataElement,
    tag: Tag,
) -> Result<[f64; N], FrameGeometryError> {
    if element.vr() != VR::DS {
        return Err(FrameGeometryError::InvalidVr {
            tag,
            found: element.vr(),
        });
    }
    let MetadataValue::Text(values) = element.value() else {
        return Err(FrameGeometryError::InvalidDecimalString { tag });
    };
    if values.len() != N {
        return Err(FrameGeometryError::DecimalStringMultiplicity {
            tag,
            expected: N,
            actual: values.len(),
        });
    }
    let mut parsed = [0.0_f64; N];
    for (slot, raw) in parsed.iter_mut().zip(values.iter()) {
        *slot =
            parse_decimal_string(raw).ok_or(FrameGeometryError::InvalidDecimalString { tag })?;
    }
    Ok(parsed)
}

/// Parse one DICOM PS3.5 section 6.2 Decimal String.
///
/// The grammar is an optionally signed decimal with an optional fraction and an
/// optional exponent, at most sixteen bytes including padding, which is a
/// space for a string Value Representation.
///
/// `f64::from_str` is used for the conversion itself, and it accepts three
/// spellings the grammar does not. `inf` and `nan` are refused by the
/// finiteness check. Rust's `1_0` digit separator is refused by the character
/// filter, which is why the filter runs before the parse rather than relying on
/// the parser. A leading `+` is legal DS and both accept it.
fn parse_decimal_string(raw: &str) -> Option<f64> {
    if raw.len() > DECIMAL_STRING_MAX_BYTES {
        return None;
    }
    let text = raw.trim_matches(' ');
    if text.is_empty() {
        return None;
    }
    if !text
        .bytes()
        .all(|byte| byte.is_ascii_digit() || matches!(byte, b'+' | b'-' | b'.' | b'e' | b'E'))
    {
        return None;
    }
    let value = text.parse::<f64>().ok()?;
    value.is_finite().then_some(value)
}

#[cfg(test)]
mod tests {
    use super::{DECIMAL_STRING_MAX_BYTES, parse_decimal_string};

    #[test]
    fn the_decimal_string_grammar_of_ps3_5_6_2_is_enforced() {
        // Legal spellings.
        assert_eq!(parse_decimal_string("1.5"), Some(1.5));
        assert_eq!(parse_decimal_string("  1.5  "), Some(1.5));
        assert_eq!(parse_decimal_string("+1.5"), Some(1.5));
        assert_eq!(parse_decimal_string("-1.5"), Some(-1.5));
        assert_eq!(parse_decimal_string("1.0e-3"), Some(0.001));
        assert_eq!(parse_decimal_string("2.0E1"), Some(20.0));
        assert_eq!(parse_decimal_string("0"), Some(0.0));

        // Spellings `f64::from_str` accepts and DICOM PS3.5 6.2 does not.
        assert_eq!(parse_decimal_string("inf"), None);
        assert_eq!(parse_decimal_string("-inf"), None);
        assert_eq!(parse_decimal_string("nan"), None);
        assert_eq!(parse_decimal_string("1_0"), None);

        // Not a decimal at all.
        assert_eq!(parse_decimal_string(""), None);
        assert_eq!(parse_decimal_string("   "), None);
        assert_eq!(parse_decimal_string("1.2.3"), None);
        assert_eq!(parse_decimal_string("abc"), None);
    }

    #[test]
    fn the_sixteen_byte_limit_counts_retained_padding() {
        // PS3.5 6.2 caps DS at sixteen bytes and the padding is part of the
        // value, which is why the limit is checked before trimming.
        let sixteen = "1".repeat(DECIMAL_STRING_MAX_BYTES);
        assert_eq!(sixteen.len(), 16);
        assert!(parse_decimal_string(&sixteen).is_some());

        let seventeen = "1".repeat(DECIMAL_STRING_MAX_BYTES + 1);
        assert_eq!(parse_decimal_string(&seventeen), None);

        // Fifteen digits plus one pad byte is sixteen and is legal.
        let padded = format!("{} ", "1".repeat(15));
        assert_eq!(padded.len(), 16);
        assert!(parse_decimal_string(&padded).is_some());

        // Fifteen digits plus two pad bytes is seventeen and is not.
        let over_padded = format!("{}  ", "1".repeat(15));
        assert_eq!(over_padded.len(), 17);
        assert_eq!(parse_decimal_string(&over_padded), None);
    }
}
