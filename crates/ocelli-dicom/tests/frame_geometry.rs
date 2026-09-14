//! Derived per-frame geometry fixtures for F-020.
//!
//! Every expected coordinate is hand-computed from the DICOM PS3.3 section
//! named beside it, never from reading `ocelli-dicom` or `ocelli-pixel`.
//! HLD 27.2 R2.

use dicom_core::{PrimitiveValue, VR};
use ocelli_core::{Index, Pt};
use ocelli_dicom::{
    FrameGeometry, FrameGeometryError, FunctionalGroupSource, MetadataElement, MetadataSet,
    MultiframeMetadata, SpacingRelationship, StackGeometry, StackShear, Tag,
};

const ROWS: Tag = Tag(0x0028, 0x0010);
const COLUMNS: Tag = Tag(0x0028, 0x0011);
const NUMBER_OF_FRAMES: Tag = Tag(0x0028, 0x0008);
const PIXEL_SPACING: Tag = Tag(0x0028, 0x0030);
const IMAGER_PIXEL_SPACING: Tag = Tag(0x0018, 0x1164);
const GANTRY_DETECTOR_TILT: Tag = Tag(0x0018, 0x1120);
const IMAGE_POSITION_PATIENT: Tag = Tag(0x0020, 0x0032);
const IMAGE_ORIENTATION_PATIENT: Tag = Tag(0x0020, 0x0037);
const PIXEL_MEASURES_SEQUENCE: Tag = Tag(0x0028, 0x9110);
const PLANE_POSITION_SEQUENCE: Tag = Tag(0x0020, 0x9113);
const PLANE_ORIENTATION_SEQUENCE: Tag = Tag(0x0020, 0x9116);
const SHARED_FUNCTIONAL_GROUPS: Tag = Tag(0x5200, 0x9229);
const PER_FRAME_FUNCTIONAL_GROUPS: Tag = Tag(0x5200, 0x9230);

/// Millimetres. Coordinates are exact in these fixtures, so this only absorbs
/// `f64` representation of the decimal strings.
const MM: f64 = 1e-9;

macro_rules! require_ok {
    ($value:expr) => {{
        let result = $value;
        assert!(
            result.is_ok(),
            "expected Ok, got {:?}",
            result.as_ref().err()
        );
        let Ok(value) = result else { return };
        value
    }};
}

fn insert(set: &mut MetadataSet, tag: Tag, element: MetadataElement) {
    assert!(set.insert(tag, element).is_ok());
}

fn text(vr: VR, value: &str) -> MetadataElement {
    MetadataElement::from_primitive(vr, &PrimitiveValue::Str(value.to_owned()))
}

fn texts(vr: VR, values: &[&str]) -> MetadataElement {
    let values = values
        .iter()
        .map(|value| (*value).to_owned())
        .collect::<Vec<_>>();
    MetadataElement::from_primitive(vr, &PrimitiveValue::Strs(values.into()))
}

fn unsigned_short(value: u16) -> MetadataElement {
    MetadataElement::from_primitive(VR::US, &PrimitiveValue::U16(vec![value].into()))
}

/// One functional-group sequence holding one item holding one attribute.
fn group(group_tag: Tag, attribute_tag: Tag, element: MetadataElement) -> MetadataSet {
    let mut item = MetadataSet::new();
    insert(&mut item, attribute_tag, element);
    let mut groups = MetadataSet::new();
    insert(
        &mut groups,
        group_tag,
        MetadataElement::sequence(vec![item]),
    );
    groups
}

/// Merge several single-attribute groups into one functional-groups item.
fn groups(parts: Vec<MetadataSet>) -> MetadataSet {
    let mut merged = MetadataSet::new();
    for part in parts {
        for (tag, element) in part.iter() {
            insert(&mut merged, tag, element.clone());
        }
    }
    merged
}

fn position(values: &[&str]) -> MetadataSet {
    group(
        PLANE_POSITION_SEQUENCE,
        IMAGE_POSITION_PATIENT,
        texts(VR::DS, values),
    )
}

fn orientation(values: &[&str]) -> MetadataSet {
    group(
        PLANE_ORIENTATION_SEQUENCE,
        IMAGE_ORIENTATION_PATIENT,
        texts(VR::DS, values),
    )
}

fn measures(values: &[&str]) -> MetadataSet {
    group(
        PIXEL_MEASURES_SEQUENCE,
        PIXEL_SPACING,
        texts(VR::DS, values),
    )
}

/// An axial identity orientation. Row direction is +x, column direction is +y,
/// so the slice normal from PS3.3 C.7.6.2.1.1 is +z.
const AXIAL: &[&str] = &["1", "0", "0", "0", "1", "0"];

/// A single-frame legacy instance carrying its plane attributes at the top
/// level, which is the case PS3.3 C.7.6.2 describes and which has no
/// functional groups at all.
fn legacy_instance(spacing: &[&str]) -> MetadataSet {
    legacy_instance_with(spacing, &["10", "20", "30"], AXIAL, None)
}

/// `MetadataSet::insert` refuses a duplicate tag, because the set is a lossless
/// record of what a file carried. A fixture therefore builds the value it wants
/// rather than inserting over one.
fn legacy_instance_with(
    spacing: &[&str],
    ipp: &[&str],
    iop: &[&str],
    omit: Option<Tag>,
) -> MetadataSet {
    let mut metadata = MetadataSet::new();
    insert(&mut metadata, ROWS, unsigned_short(8));
    insert(&mut metadata, COLUMNS, unsigned_short(8));
    if omit != Some(IMAGE_POSITION_PATIENT) {
        insert(&mut metadata, IMAGE_POSITION_PATIENT, texts(VR::DS, ipp));
    }
    if omit != Some(IMAGE_ORIENTATION_PATIENT) {
        insert(&mut metadata, IMAGE_ORIENTATION_PATIENT, texts(VR::DS, iop));
    }
    if omit != Some(PIXEL_SPACING) {
        insert(&mut metadata, PIXEL_SPACING, texts(VR::DS, spacing));
    }
    metadata
}

/// The same instance with one plane attribute left out, so an absent attribute
/// is built rather than removed. `MetadataSet` has no removal API, which is
/// deliberate: it is a lossless record of what a file carried.
fn legacy_instance_omitting(spacing: &[&str], omit: Option<Tag>) -> MetadataSet {
    legacy_instance_with(spacing, &["10", "20", "30"], AXIAL, omit)
}

/// The off-axis magnitude of a sheared verdict, or `None` for any other.
fn sheared_offaxis_mm(shear: StackShear) -> Option<f64> {
    match shear {
        StackShear::Sheared { max_offaxis_mm } => Some(max_offaxis_mm),
        StackShear::AxisAligned { .. } | StackShear::NotApplicable => None,
    }
}

/// The signed projected step of an axis-aligned verdict, or `None`.
fn axis_aligned_step_mm(shear: StackShear) -> Option<f64> {
    match shear {
        StackShear::AxisAligned { step_mm } => Some(step_mm),
        StackShear::Sheared { .. } | StackShear::NotApplicable => None,
    }
}

#[track_caller]
fn assert_mm(actual: Option<f64>, expected: f64, what: &str) {
    assert!(actual.is_some(), "expected {what}, got a different verdict");
    if let Some(actual) = actual {
        assert!(
            (actual - expected).abs() < MM,
            "was {actual}, wanted {expected}"
        );
    }
}

/// An enhanced multiframe instance with one plane position per frame.
fn enhanced_instance(frame_positions: &[&[&str]]) -> MetadataSet {
    enhanced_instance_with_shared_position(frame_positions, None)
}

/// The same instance, optionally also declaring a SHARED plane position that
/// the per-frame items already carry. PS3.3 C.7.6.16.1.1 makes that malformed.
fn enhanced_instance_with_shared_position(
    frame_positions: &[&[&str]],
    shared_position: Option<&[&str]>,
) -> MetadataSet {
    let mut metadata = MetadataSet::new();
    insert(&mut metadata, ROWS, unsigned_short(8));
    insert(&mut metadata, COLUMNS, unsigned_short(8));
    insert(
        &mut metadata,
        NUMBER_OF_FRAMES,
        text(VR::IS, &frame_positions.len().to_string()),
    );
    let mut shared = vec![orientation(AXIAL), measures(&["2.0", "0.5"])];
    if let Some(values) = shared_position {
        shared.push(position(values));
    }
    insert(
        &mut metadata,
        SHARED_FUNCTIONAL_GROUPS,
        MetadataElement::sequence(vec![groups(shared)]),
    );
    insert(
        &mut metadata,
        PER_FRAME_FUNCTIONAL_GROUPS,
        MetadataElement::sequence(
            frame_positions
                .iter()
                .map(|values| position(values))
                .collect(),
        ),
    );
    metadata
}

#[track_caller]
fn assert_point(actual: ocelli_core::Pt<ocelli_core::World>, expected: [f64; 3]) {
    for (got, want) in [actual.x, actual.y, actual.z].into_iter().zip(expected) {
        assert!((got - want).abs() < MM, "was {got}, wanted {want}");
    }
}

#[test]
fn pixel_spacing_row_value_scales_the_column_direction_cosine() {
    // DICOM PS3.3 C.7.6.2.1.1:
    //   P = IPP + i * PixelSpacing[1] * X + j * PixelSpacing[0] * Y
    // with i the COLUMN index, j the ROW index, X the row direction cosine and
    // Y the column direction cosine. PixelSpacing is [between rows, between
    // columns], so PixelSpacing[0] scales Y and PixelSpacing[1] scales X.
    //
    // IPP = (10, 20, 30), X = (1,0,0), Y = (0,1,0),
    // PixelSpacing = "2.0\0.5", so row spacing 2.0 and column spacing 0.5.
    // At column i = 4, row j = 3:
    //   P = (10,20,30) + 4 * 0.5 * (1,0,0) + 3 * 2.0 * (0,1,0)
    //     = (10 + 2, 20 + 6, 30) = (12, 26, 30)
    //
    // Swapping the two spacing values gives (10 + 8, 20 + 1.5, 30), which is a
    // different point. A square-pixel fixture could not tell them apart, which
    // is why this one is deliberately non-square.
    let metadata = legacy_instance(&["2.0", "0.5"]);
    let multiframe = require_ok!(MultiframeMetadata::new(&metadata));
    let geometry = require_ok!(FrameGeometry::derive(&multiframe, 0));

    let world = geometry
        .index_to_world()
        .apply(Pt::<Index>::new(4.0, 3.0, 0.0));
    assert_point(world, [12.0, 26.0, 30.0]);

    // PS3.3 C.7.6.2.1.1 makes IPP the centre of the first voxel, not a corner.
    assert_point(
        geometry
            .index_to_world()
            .apply(Pt::<Index>::new(0.0, 0.0, 0.0)),
        [10.0, 20.0, 30.0],
    );
}

#[test]
fn per_frame_position_wins_over_shared_and_the_shared_source_stays_visible() {
    // DICOM PS3.3 C.7.6.16.1.1: a per-frame value takes precedence over a
    // shared one. An attribute in both is malformed, and F-019 chose to retain
    // the losing source as observable evidence rather than refuse. F-020
    // follows that choice so one instance does not parse under one module and
    // fail under the other.
    let metadata = enhanced_instance_with_shared_position(
        &[&["0", "0", "0"], &["0", "0", "2"]],
        Some(&["99", "99", "99"]),
    );
    let multiframe = require_ok!(MultiframeMetadata::new(&metadata));
    let geometry = require_ok!(FrameGeometry::derive(&multiframe, 1));

    // The per-frame value is (0,0,2), not the shared (99,99,99).
    assert_point(
        geometry
            .index_to_world()
            .apply(Pt::<Index>::new(0.0, 0.0, 0.0)),
        [0.0, 0.0, 2.0],
    );
    // All three sources are asserted, not only the one that changed. An
    // accessor no fixture reads is an accessor that can report anything.
    let sources = geometry.sources();
    assert_eq!(sources.position(), FunctionalGroupSource::PerFrame(1));
    assert_eq!(sources.orientation(), FunctionalGroupSource::Shared);
    assert_eq!(sources.pixel_spacing(), FunctionalGroupSource::Shared);
    assert!(sources.duplicates().contains(FunctionalGroupSource::Shared));

    // A legacy instance answers all three from the main data set, which is the
    // other end of PS3.3 C.7.6.16.1.1's precedence order.
    let legacy = legacy_instance(&["2.0", "0.5"]);
    let legacy = require_ok!(MultiframeMetadata::new(&legacy));
    let legacy = require_ok!(FrameGeometry::derive(&legacy, 0));
    let sources = legacy.sources();
    assert_eq!(sources.position(), FunctionalGroupSource::TopLevel);
    assert_eq!(sources.orientation(), FunctionalGroupSource::TopLevel);
    assert_eq!(sources.pixel_spacing(), FunctionalGroupSource::TopLevel);
    assert!(sources.duplicates().is_empty());
}

#[test]
fn shear_is_measured_from_geometry_and_never_from_the_tilt_tag() {
    // DICOM PS3.3 C.7.6.2.1.1. The slice normal is N = X cross Y, which for the
    // axial identity orientation is (0,0,1). A stack is axis-aligned exactly
    // when every inter-frame step is parallel to N.
    //
    // Gantry/Detector Tilt (0018,1120) is Type 3 and NOMINAL, so it is retained
    // as evidence and decides nothing. Both directions are asserted here.

    // Sheared geometry, tilt tag says 0.
    //   IPP: (0,0,0), (0,1,2), (0,2,4). step = (0,1,2).
    //   step dot N = 2, so the off-axis part is (0,1,2) - 2*(0,0,1) = (0,1,0),
    //   whose length is 1.0 mm.
    let mut metadata = enhanced_instance(&[&["0", "0", "0"], &["0", "1", "2"], &["0", "2", "4"]]);
    insert(&mut metadata, GANTRY_DETECTOR_TILT, text(VR::DS, "0"));
    let multiframe = require_ok!(MultiframeMetadata::new(&metadata));
    let stack = require_ok!(StackGeometry::derive(&multiframe));
    assert_eq!(stack.nominal_tilt_degrees(), Some(0.0));
    assert_mm(sheared_offaxis_mm(stack.shear()), 1.0, "Sheared");

    // Axis-aligned geometry, tilt tag says 30.
    //   IPP: (0,0,0), (0,0,2), (0,0,4). step = (0,0,2), parallel to N.
    //   The projected step is +2 mm, signed, so a descending stack would be -2.
    let mut metadata = enhanced_instance(&[&["0", "0", "0"], &["0", "0", "2"], &["0", "0", "4"]]);
    insert(&mut metadata, GANTRY_DETECTOR_TILT, text(VR::DS, "30"));
    let multiframe = require_ok!(MultiframeMetadata::new(&metadata));
    let stack = require_ok!(StackGeometry::derive(&multiframe));
    assert_eq!(stack.nominal_tilt_degrees(), Some(30.0));
    assert_mm(axis_aligned_step_mm(stack.shear()), 2.0, "AxisAligned");
}

#[test]
fn a_descending_stack_reports_a_negative_step() {
    // The projected step is `step dot N`, signed. Taking its absolute value
    // would lose the acquisition direction, which a volume builder needs.
    let metadata = enhanced_instance(&[&["0", "0", "4"], &["0", "0", "2"], &["0", "0", "0"]]);
    let multiframe = require_ok!(MultiframeMetadata::new(&metadata));
    let stack = require_ok!(StackGeometry::derive(&multiframe));
    assert_mm(axis_aligned_step_mm(stack.shear()), -2.0, "AxisAligned");
}

#[test]
fn a_single_frame_instance_has_no_inter_frame_vector() {
    let metadata = legacy_instance(&["2.0", "0.5"]);
    let multiframe = require_ok!(MultiframeMetadata::new(&metadata));
    let stack = require_ok!(StackGeometry::derive(&multiframe));
    assert_eq!(stack.shear(), StackShear::NotApplicable);
    assert_eq!(stack.nominal_tilt_degrees(), None);
}

#[test]
fn non_uniform_frame_spacing_is_refused_rather_than_averaged() {
    // Projected positions 0, 2 and 5 give steps of 2 and 3. A volume model that
    // assumed uniform spacing would render this stretched along the normal,
    // which no measurement tool flags. The answer is a refusal.
    let metadata = enhanced_instance(&[&["0", "0", "0"], &["0", "0", "2"], &["0", "0", "5"]]);
    let multiframe = require_ok!(MultiframeMetadata::new(&metadata));
    assert_eq!(
        StackGeometry::derive(&multiframe),
        Err(FrameGeometryError::NonUniformFrameSpacing)
    );
}

#[test]
fn calibrated_pixel_spacing_wins_and_imager_spacing_is_retained() {
    // DICOM PS3.3 C.7.6.1.1.5. When both are present and differ, Pixel Spacing
    // has been calibrated to a plane other than the detector plane, and it is
    // the one that applies to measurements in the image. Imager Pixel Spacing
    // is detector spacing and is retained as evidence only.
    let mut metadata = legacy_instance(&["0.139", "0.139"]);
    insert(
        &mut metadata,
        IMAGER_PIXEL_SPACING,
        texts(VR::DS, &["0.175", "0.175"]),
    );
    let multiframe = require_ok!(MultiframeMetadata::new(&metadata));
    let geometry = require_ok!(FrameGeometry::derive(&multiframe, 0));

    let spacing = geometry.spacing();
    assert_eq!(spacing.relationship(), SpacingRelationship::Calibrated);
    assert!((spacing.image_mm().row_mm() - 0.139).abs() < MM);
    assert!((spacing.image_mm().column_mm() - 0.139).abs() < MM);
    let imager = spacing.imager_mm();
    assert!(imager.is_some());
    if let Some(imager) = imager {
        assert!((imager.row_mm() - 0.175).abs() < MM);
    }

    // The geometry uses the calibrated value. Substituting detector spacing
    // would scale every measurement by 0.175 / 0.139, about 26 per cent, on an
    // image that renders correctly.
    assert_point(
        geometry
            .index_to_world()
            .apply(Pt::<Index>::new(1.0, 0.0, 0.0)),
        [10.139, 20.0, 30.0],
    );
}

#[test]
fn agreeing_imager_spacing_is_reported_as_agreement_rather_than_calibration() {
    let mut metadata = legacy_instance(&["0.175", "0.175"]);
    insert(
        &mut metadata,
        IMAGER_PIXEL_SPACING,
        texts(VR::DS, &["0.175", "0.175"]),
    );
    let multiframe = require_ok!(MultiframeMetadata::new(&metadata));
    let geometry = require_ok!(FrameGeometry::derive(&multiframe, 0));
    assert_eq!(
        geometry.spacing().relationship(),
        SpacingRelationship::ImagerAgrees
    );
}

#[test]
fn pixel_spacing_alone_is_reported_as_image_only() {
    let metadata = legacy_instance(&["2.0", "0.5"]);
    let multiframe = require_ok!(MultiframeMetadata::new(&metadata));
    let geometry = require_ok!(FrameGeometry::derive(&multiframe, 0));
    assert_eq!(
        geometry.spacing().relationship(),
        SpacingRelationship::ImageOnly
    );
    assert_eq!(geometry.spacing().imager_mm(), None);
}

#[test]
fn imager_pixel_spacing_alone_is_refused_and_never_substituted() {
    // DICOM PS3.3 C.7.6.1.1.5. Imager Pixel Spacing is spacing at the DETECTOR
    // plane. On a projection radiograph it differs from spacing in the image by
    // the source-to-image magnification factor, typically 10 to 20 per cent.
    // Substituting it produces a geometry that renders and measures wrong, so
    // the answer is a refusal and the caller decides.
    let mut metadata = legacy_instance_omitting(&["2.0", "0.5"], Some(PIXEL_SPACING));
    insert(
        &mut metadata,
        IMAGER_PIXEL_SPACING,
        texts(VR::DS, &["0.175", "0.175"]),
    );
    let multiframe = require_ok!(MultiframeMetadata::new(&metadata));
    assert_eq!(
        FrameGeometry::derive(&multiframe, 0),
        Err(FrameGeometryError::UncalibratedSpacingOnly)
    );
}

#[test]
fn absent_spacing_position_and_orientation_each_refuse_distinctly() {
    for (tag, expected) in [
        (PIXEL_SPACING, FrameGeometryError::MissingPixelSpacing),
        (
            IMAGE_POSITION_PATIENT,
            FrameGeometryError::MissingImagePosition,
        ),
        (
            IMAGE_ORIENTATION_PATIENT,
            FrameGeometryError::MissingImageOrientation,
        ),
    ] {
        let metadata = legacy_instance_omitting(&["2.0", "0.5"], Some(tag));
        let multiframe = require_ok!(MultiframeMetadata::new(&metadata));
        assert_eq!(FrameGeometry::derive(&multiframe, 0), Err(expected));
    }
}

#[test]
fn a_decimal_string_with_the_wrong_multiplicity_is_refused() {
    let metadata = legacy_instance(&["2.0"]);
    let multiframe = require_ok!(MultiframeMetadata::new(&metadata));
    assert_eq!(
        FrameGeometry::derive(&multiframe, 0),
        Err(FrameGeometryError::DecimalStringMultiplicity {
            tag: PIXEL_SPACING,
            expected: 2,
            actual: 1,
        })
    );

    let metadata = legacy_instance_with(
        &["2.0", "0.5"],
        &["10", "20", "30"],
        &["1", "0", "0", "0", "1"],
        None,
    );
    let multiframe = require_ok!(MultiframeMetadata::new(&metadata));
    assert_eq!(
        FrameGeometry::derive(&multiframe, 0),
        Err(FrameGeometryError::DecimalStringMultiplicity {
            tag: IMAGE_ORIENTATION_PATIENT,
            expected: 6,
            actual: 5,
        })
    );
}

#[test]
fn a_decimal_string_outside_the_ps3_5_grammar_is_refused() {
    // DICOM PS3.5 section 6.2 defines DS as an optionally signed decimal with
    // an optional fraction and an optional exponent, at most 16 bytes including
    // padding. `f64::from_str` accepts three spellings DS does not.
    for value in [
        "inf",               // not a decimal
        "nan",               // not a decimal
        "1_0",               // Rust's digit separator, not DICOM's
        "1.2.3",             // two decimal points
        "",                  // empty after trimming
        "12345678901234567", // 17 bytes, over the PS3.5 6.2 limit
    ] {
        let metadata = legacy_instance_with(&["2.0", "0.5"], &[value, "20", "30"], AXIAL, None);
        let multiframe = require_ok!(MultiframeMetadata::new(&metadata));
        assert_eq!(
            FrameGeometry::derive(&multiframe, 0),
            Err(FrameGeometryError::InvalidDecimalString {
                tag: IMAGE_POSITION_PATIENT
            }),
            "value {value:?} should have been refused"
        );
    }
}

#[test]
fn a_decimal_string_inside_the_ps3_5_grammar_is_accepted() {
    // Legal DS spellings that must NOT be refused: retained padding, an
    // explicit plus sign, and an exponent.
    let metadata = legacy_instance_with(&["2.0", "0.5"], &[" +10.0 ", "2.0e1", "30"], AXIAL, None);
    let multiframe = require_ok!(MultiframeMetadata::new(&metadata));
    let geometry = require_ok!(FrameGeometry::derive(&multiframe, 0));
    assert_point(
        geometry
            .index_to_world()
            .apply(Pt::<Index>::new(0.0, 0.0, 0.0)),
        [10.0, 20.0, 30.0],
    );
}

#[test]
fn a_frame_outside_the_declared_count_is_refused() {
    let metadata = enhanced_instance(&[&["0", "0", "0"], &["0", "0", "2"]]);
    let multiframe = require_ok!(MultiframeMetadata::new(&metadata));
    assert!(FrameGeometry::derive(&multiframe, 2).is_err());
}

#[test]
fn index_to_world_round_trips_through_its_inverse() {
    // A property rather than a fixture: canvas to world to canvas is the
    // round trip HLD section 16's typed transforms exist to make checkable.
    let metadata = legacy_instance(&["2.0", "0.5"]);
    let multiframe = require_ok!(MultiframeMetadata::new(&metadata));
    let geometry = require_ok!(FrameGeometry::derive(&multiframe, 0));
    let forward = geometry.index_to_world();
    let inverse = forward.inverse();

    for (i, j) in [(0.0, 0.0), (7.0, 7.0), (3.0, 5.0), (-2.0, 11.0)] {
        let index = Pt::<Index>::new(i, j, 0.0);
        let back = inverse.apply(forward.apply(index));
        for (got, want) in [back.x, back.y, back.z].into_iter().zip([i, j, 0.0]) {
            assert!((got - want).abs() < 1e-9, "was {got}, wanted {want}");
        }
    }
}
