//! Synthetic enhanced multiframe metadata fixtures for F-019.
//!
//! DICOM PS3.3 C.7.6.6 and C.7.6.16 define Number of Frames and the
//! Shared and Per-frame Functional Groups Sequences. These non-identifying
//! values are assembled through the lossless metadata model before the
//! projection implementation exists.

use dicom_core::{PrimitiveValue, VR};
use ocelli_dicom::{
    FunctionalGroupSource, MetadataElement, MetadataSet, MultiframeError, MultiframeMetadata, Tag,
    TopLevelFallback,
};

const NUMBER_OF_FRAMES: Tag = Tag(0x0028, 0x0008);
const PIXEL_SPACING: Tag = Tag(0x0028, 0x0030);
const WINDOW_CENTER: Tag = Tag(0x0028, 0x1050);
const PIXEL_MEASURES_SEQUENCE: Tag = Tag(0x0028, 0x9110);
const FRAME_VOI_LUT_SEQUENCE: Tag = Tag(0x0028, 0x9132);
const SHARED_FUNCTIONAL_GROUPS: Tag = Tag(0x5200, 0x9229);
const PER_FRAME_FUNCTIONAL_GROUPS: Tag = Tag(0x5200, 0x9230);

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

fn group(group_tag: Tag, attribute_tag: Tag, value: &str) -> MetadataSet {
    group_element(group_tag, attribute_tag, text(VR::DS, value))
}

fn group_element(group_tag: Tag, attribute_tag: Tag, element: MetadataElement) -> MetadataSet {
    let mut item = MetadataSet::new();
    insert(&mut item, attribute_tag, element);
    let mut functional_groups = MetadataSet::new();
    insert(
        &mut functional_groups,
        group_tag,
        MetadataElement::sequence(vec![item]),
    );
    functional_groups
}

fn enhanced_fixture() -> MetadataSet {
    let mut metadata = MetadataSet::new();
    insert(&mut metadata, NUMBER_OF_FRAMES, text(VR::IS, "2"));
    insert(
        &mut metadata,
        SHARED_FUNCTIONAL_GROUPS,
        MetadataElement::sequence(vec![group_element(
            PIXEL_MEASURES_SEQUENCE,
            PIXEL_SPACING,
            texts(VR::DS, &["0.5", "0.25"]),
        )]),
    );
    insert(
        &mut metadata,
        PER_FRAME_FUNCTIONAL_GROUPS,
        MetadataElement::sequence(vec![
            group(FRAME_VOI_LUT_SEQUENCE, WINDOW_CENTER, "40"),
            group(FRAME_VOI_LUT_SEQUENCE, WINDOW_CENTER, "80"),
        ]),
    );
    metadata
}

#[test]
fn enhanced_projection_preserves_count_order_and_distinct_sources() {
    let metadata = enhanced_fixture();
    let projection = MultiframeMetadata::new(&metadata);
    assert!(projection.is_ok());
    let projection = projection.unwrap_or_else(|_| unreachable!());

    assert_eq!(projection.frame_count(), 2);
    let spacing = projection.resolve(
        1,
        PIXEL_MEASURES_SEQUENCE,
        PIXEL_SPACING,
        TopLevelFallback::Disallowed,
    );
    assert!(spacing.is_ok());
    assert_eq!(
        spacing
            .ok()
            .flatten()
            .map(|resolved| (resolved.source(), resolved.element().semantic_text())),
        Some((FunctionalGroupSource::Shared, Some(vec!["0.5", "0.25"]),))
    );

    let first_window = projection.resolve(
        0,
        FRAME_VOI_LUT_SEQUENCE,
        WINDOW_CENTER,
        TopLevelFallback::Disallowed,
    );
    let second_window = projection.resolve(
        1,
        FRAME_VOI_LUT_SEQUENCE,
        WINDOW_CENTER,
        TopLevelFallback::Disallowed,
    );
    assert_eq!(
        first_window
            .ok()
            .flatten()
            .map(|resolved| { (resolved.source(), resolved.element().semantic_text()) }),
        Some((FunctionalGroupSource::PerFrame(0), Some(vec!["40"]),))
    );
    assert_eq!(
        second_window
            .ok()
            .flatten()
            .map(|resolved| { (resolved.source(), resolved.element().semantic_text()) }),
        Some((FunctionalGroupSource::PerFrame(1), Some(vec!["80"]),))
    );
}

#[test]
fn per_frame_wins_while_duplicate_sources_remain_observable() {
    let mut metadata = MetadataSet::new();
    insert(&mut metadata, NUMBER_OF_FRAMES, text(VR::IS, "1"));
    insert(&mut metadata, WINDOW_CENTER, text(VR::DS, "20"));
    insert(
        &mut metadata,
        SHARED_FUNCTIONAL_GROUPS,
        MetadataElement::sequence(vec![group(FRAME_VOI_LUT_SEQUENCE, WINDOW_CENTER, "40")]),
    );
    insert(
        &mut metadata,
        PER_FRAME_FUNCTIONAL_GROUPS,
        MetadataElement::sequence(vec![group(FRAME_VOI_LUT_SEQUENCE, WINDOW_CENTER, "80")]),
    );

    let projection = MultiframeMetadata::new(&metadata).unwrap_or_else(|_| unreachable!());
    let resolved = projection.resolve(
        0,
        FRAME_VOI_LUT_SEQUENCE,
        WINDOW_CENTER,
        TopLevelFallback::Allowed,
    );
    assert!(resolved.is_ok());
    let resolved = resolved.ok().flatten().unwrap_or_else(|| unreachable!());
    assert_eq!(resolved.source(), FunctionalGroupSource::PerFrame(0));
    assert_eq!(resolved.element().semantic_text(), Some(vec!["80"]));
    assert!(
        resolved
            .duplicate_sources()
            .contains(FunctionalGroupSource::Shared)
    );
    assert!(
        resolved
            .duplicate_sources()
            .contains(FunctionalGroupSource::TopLevel)
    );
    assert!(!resolved.duplicate_sources().is_empty());
}

#[test]
fn top_level_is_consulted_only_when_the_callers_module_permits_it() {
    let mut metadata = MetadataSet::new();
    insert(&mut metadata, WINDOW_CENTER, text(VR::DS, "20"));
    let projection = MultiframeMetadata::new(&metadata).unwrap_or_else(|_| unreachable!());

    assert_eq!(
        projection
            .resolve(
                0,
                FRAME_VOI_LUT_SEQUENCE,
                WINDOW_CENTER,
                TopLevelFallback::Disallowed,
            )
            .ok()
            .flatten(),
        None
    );
    assert_eq!(
        projection
            .resolve(
                0,
                FRAME_VOI_LUT_SEQUENCE,
                WINDOW_CENTER,
                TopLevelFallback::Allowed,
            )
            .ok()
            .flatten()
            .map(|resolved| resolved.source()),
        Some(FunctionalGroupSource::TopLevel)
    );
}

#[test]
fn number_of_frames_defaults_only_when_absent() {
    let absent = MetadataSet::new();
    assert_eq!(
        MultiframeMetadata::new(&absent).map(|projection| projection.frame_count()),
        Ok(1)
    );

    let cases = [
        (
            MetadataElement::empty(VR::IS),
            MultiframeError::EmptyNumberOfFrames,
        ),
        (text(VR::IS, "0"), MultiframeError::ZeroNumberOfFrames),
        (text(VR::IS, "two"), MultiframeError::InvalidNumberOfFrames),
        (
            text(VR::IS, "1234567890123"),
            MultiframeError::InvalidNumberOfFrames,
        ),
        (
            text(VR::IS, "           1  "),
            MultiframeError::InvalidNumberOfFrames,
        ),
        (
            text(VR::IS, "2147483648"),
            MultiframeError::InvalidNumberOfFrames,
        ),
        (
            texts(VR::IS, &["1", "2"]),
            MultiframeError::MultipleNumberOfFrames(2),
        ),
        (
            text(VR::DS, "2"),
            MultiframeError::InvalidNumberOfFramesVr(VR::DS),
        ),
    ];

    for (element, expected) in cases {
        let mut metadata = MetadataSet::new();
        insert(&mut metadata, NUMBER_OF_FRAMES, element);
        assert_eq!(MultiframeMetadata::new(&metadata).err(), Some(expected));
    }
}

#[test]
fn malformed_functional_group_content_is_not_silently_skipped() {
    let mut shared_item = MetadataSet::new();
    insert(&mut shared_item, FRAME_VOI_LUT_SEQUENCE, text(VR::DS, "40"));
    let mut metadata = MetadataSet::new();
    insert(
        &mut metadata,
        SHARED_FUNCTIONAL_GROUPS,
        MetadataElement::sequence(vec![shared_item]),
    );
    let projection = MultiframeMetadata::new(&metadata).unwrap_or_else(|_| unreachable!());
    assert_eq!(
        projection
            .resolve(
                0,
                FRAME_VOI_LUT_SEQUENCE,
                WINDOW_CENTER,
                TopLevelFallback::Disallowed,
            )
            .err(),
        Some(MultiframeError::InvalidFunctionalGroup {
            location: FunctionalGroupSource::Shared,
            tag: FRAME_VOI_LUT_SEQUENCE,
        })
    );
}

#[test]
fn functional_group_item_counts_and_frame_selection_are_checked() {
    let mut too_many_shared = MetadataSet::new();
    insert(
        &mut too_many_shared,
        SHARED_FUNCTIONAL_GROUPS,
        MetadataElement::sequence(vec![MetadataSet::new(), MetadataSet::new()]),
    );
    assert_eq!(
        MultiframeMetadata::new(&too_many_shared).err(),
        Some(MultiframeError::SharedFunctionalGroupsCount(2))
    );

    let mut inconsistent = MetadataSet::new();
    insert(&mut inconsistent, NUMBER_OF_FRAMES, text(VR::IS, "2"));
    insert(
        &mut inconsistent,
        PER_FRAME_FUNCTIONAL_GROUPS,
        MetadataElement::sequence(vec![MetadataSet::new()]),
    );
    assert_eq!(
        MultiframeMetadata::new(&inconsistent).err(),
        Some(MultiframeError::PerFrameFunctionalGroupsCount {
            declared: 2,
            actual: 1,
        })
    );

    let enhanced = enhanced_fixture();
    let projection = MultiframeMetadata::new(&enhanced).unwrap_or_else(|_| unreachable!());
    assert_eq!(
        projection
            .resolve(
                2,
                FRAME_VOI_LUT_SEQUENCE,
                WINDOW_CENTER,
                TopLevelFallback::Disallowed,
            )
            .err(),
        Some(MultiframeError::FrameOutOfRange {
            requested: 2,
            declared: 2,
        })
    );
}
