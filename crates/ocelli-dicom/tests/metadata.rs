//! Lossless metadata projection fixtures for F-017.
//!
//! PS3.5 sections 6.2 and 7.1 define VR spelling, multiplicity, even-length
//! padding, signed binary values, and SQ item nesting. These hand-encoded,
//! non-identifying elements are independent inputs rather than Ocelli output.

use dicom_core::{PrimitiveValue, VR};
use ocelli_dicom::{
    MetadataElement, MetadataError, MetadataRequest, MetadataSet, MetadataValue, ProviderId,
    ProviderRegistry, ProviderRegistryError, Tag, data_set_provider, file_meta_provider,
};
use proptest::prelude::*;

const EXPLICIT_VR_LE_PADDED: &[u8] = b"1.2.840.10008.1.2.1\0";
const JPEG_BASELINE_PADDED: &[u8] = b"1.2.840.10008.1.2.4.50\0";
const SOP_CLASS_UID: &[u8] = b"1.2.840.10008.5.1.4.1.1.7\0";
const SOP_INSTANCE_UID: &[u8] = b"2.25.1";
const IMPLEMENTATION_UID: &[u8] = b"2.25.2";

fn fixture_u16(value: usize) -> u16 {
    u16::try_from(value).unwrap_or(u16::MAX)
}

fn fixture_u32(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

fn explicit_element(tag: (u16, u16), vr: [u8; 2], value: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&tag.0.to_le_bytes());
    bytes.extend_from_slice(&tag.1.to_le_bytes());
    bytes.extend_from_slice(&vr);
    bytes.extend_from_slice(&fixture_u16(value.len()).to_le_bytes());
    bytes.extend_from_slice(value);
    bytes
}

fn long_explicit_element(tag: (u16, u16), vr: [u8; 2], value: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&tag.0.to_le_bytes());
    bytes.extend_from_slice(&tag.1.to_le_bytes());
    bytes.extend_from_slice(&vr);
    bytes.extend_from_slice(&[0, 0]);
    bytes.extend_from_slice(&fixture_u32(value.len()).to_le_bytes());
    bytes.extend_from_slice(value);
    bytes
}

fn item(value: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&0xfffe_u16.to_le_bytes());
    bytes.extend_from_slice(&0xe000_u16.to_le_bytes());
    bytes.extend_from_slice(&fixture_u32(value.len()).to_le_bytes());
    bytes.extend_from_slice(value);
    bytes
}

fn file_meta(transfer_syntax: &[u8]) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend(long_explicit_element((0x0002, 0x0001), *b"OB", &[0, 1]));
    body.extend(explicit_element((0x0002, 0x0002), *b"UI", SOP_CLASS_UID));
    body.extend(explicit_element((0x0002, 0x0003), *b"UI", SOP_INSTANCE_UID));
    body.extend(explicit_element((0x0002, 0x0010), *b"UI", transfer_syntax));
    body.extend(explicit_element(
        (0x0002, 0x0012),
        *b"UI",
        IMPLEMENTATION_UID,
    ));

    let mut meta = explicit_element(
        (0x0002, 0x0000),
        *b"UL",
        &fixture_u32(body.len()).to_le_bytes(),
    );
    meta.extend(body);
    meta
}

fn metadata_data_set() -> Vec<u8> {
    let mut nested = Vec::new();
    nested.extend(explicit_element(
        (0x0008, 0x1150),
        *b"UI",
        b"1.2.840.10008.5.1.4.1.1.7\0",
    ));

    let mut bytes = Vec::new();
    bytes.extend(explicit_element((0x0010, 0x0010), *b"PN", b""));
    bytes.extend(explicit_element((0x0008, 0x0016), *b"UI", b"1.2.3\0"));
    bytes.extend(explicit_element((0x0008, 0x103e), *b"LO", b"SYNTHETIC "));
    bytes.extend(explicit_element((0x0008, 0x0081), *b"ST", b"  SYNTHETIC "));
    bytes.extend(explicit_element((0x0028, 0x0030), *b"DS", b" 0.5\\1.25 "));
    bytes.extend(explicit_element(
        (0x0011, 0x1001),
        *b"SS",
        &(-7_i16).to_le_bytes(),
    ));
    bytes.extend(explicit_element(
        (0x0011, 0x1002),
        *b"SL",
        &(-70_000_i32).to_le_bytes(),
    ));
    bytes.extend(long_explicit_element(
        (0x0008, 0x1115),
        *b"SQ",
        &item(&nested),
    ));
    bytes
}

fn part10_with_syntax(transfer_syntax: &[u8], data_set: &[u8]) -> Vec<u8> {
    let mut bytes = vec![0; 128];
    bytes.extend_from_slice(b"DICM");
    bytes.extend(file_meta(transfer_syntax));
    bytes.extend_from_slice(data_set);
    bytes
}

fn part10(data_set: &[u8]) -> Vec<u8> {
    part10_with_syntax(EXPLICIT_VR_LE_PADDED, data_set)
}

fn encapsulated_pixel_data_set() -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&0x7fe0_u16.to_le_bytes());
    bytes.extend_from_slice(&0x0010_u16.to_le_bytes());
    bytes.extend_from_slice(b"OB");
    bytes.extend_from_slice(&[0, 0]);
    bytes.extend_from_slice(&u32::MAX.to_le_bytes());
    bytes.extend(item(&[]));
    bytes.extend(item(&[1, 2, 3, 4]));
    bytes.extend_from_slice(&0xfffe_u16.to_le_bytes());
    bytes.extend_from_slice(&0xe0dd_u16.to_le_bytes());
    bytes.extend_from_slice(&0_u32.to_le_bytes());
    bytes
}

fn parsed_fixture() -> ocelli_dicom::ParsedDicom {
    let parsed = ocelli_dicom::parse_part10(&part10(&metadata_data_set()));
    assert!(parsed.is_ok(), "the independently encoded fixture parses");
    parsed.unwrap_or_else(|_| unreachable!())
}

#[test]
fn projection_distinguishes_missing_empty_and_preserves_wire_spelling() {
    let parsed = parsed_fixture();
    let metadata = MetadataSet::from_object(parsed.object());
    assert!(metadata.is_ok());
    let metadata = metadata.unwrap_or_else(|_| unreachable!());

    assert!(metadata.get(Tag(0x0010, 0x0020)).is_none());
    assert_eq!(
        metadata
            .get(Tag(0x0010, 0x0010))
            .map(MetadataElement::value),
        Some(&MetadataValue::Empty)
    );
    assert_eq!(
        metadata
            .get(Tag(0x0008, 0x0016))
            .map(MetadataElement::value),
        Some(&MetadataValue::Text(vec!["1.2.3\0".to_owned()]))
    );
    assert_eq!(
        metadata
            .get(Tag(0x0008, 0x103e))
            .map(MetadataElement::value),
        Some(&MetadataValue::Text(vec!["SYNTHETIC ".to_owned()]))
    );
    assert_eq!(
        metadata
            .get(Tag(0x0028, 0x0030))
            .and_then(MetadataElement::semantic_text),
        Some(vec!["0.5", "1.25"])
    );
    assert_eq!(
        metadata
            .get(Tag(0x0008, 0x0081))
            .and_then(MetadataElement::semantic_text),
        Some(vec!["  SYNTHETIC"])
    );
}

#[test]
fn projection_preserves_signed_width_multiplicity_and_nested_items() {
    let parsed = parsed_fixture();
    let metadata = MetadataSet::from_object(parsed.object());
    assert!(metadata.is_ok());
    let metadata = metadata.unwrap_or_else(|_| unreachable!());

    assert_eq!(
        metadata
            .get(Tag(0x0011, 0x1001))
            .map(MetadataElement::value),
        Some(&MetadataValue::Signed16(vec![-7]))
    );
    assert_eq!(
        metadata
            .get(Tag(0x0011, 0x1002))
            .map(MetadataElement::value),
        Some(&MetadataValue::Signed32(vec![-70_000]))
    );

    let sequence = metadata
        .get(Tag(0x0008, 0x1115))
        .map(MetadataElement::value);
    assert_eq!(
        sequence.and_then(|value| match value {
            MetadataValue::Sequence(items) => Some(items.len()),
            _ => None,
        }),
        Some(1)
    );
    let item = match sequence {
        Some(MetadataValue::Sequence(items)) => items.first(),
        _ => None,
    };
    assert_eq!(
        item.and_then(|nested| nested.get(Tag(0x0008, 0x1150)))
            .map(MetadataElement::value),
        Some(&MetadataValue::Text(vec![
            "1.2.840.10008.5.1.4.1.1.7\0".to_owned()
        ]))
    );
}

#[test]
fn projection_refuses_to_flatten_encapsulated_pixel_fragments() {
    let parsed = ocelli_dicom::parse_part10(&part10_with_syntax(
        JPEG_BASELINE_PADDED,
        &encapsulated_pixel_data_set(),
    ));
    assert!(parsed.is_ok(), "the encapsulated fixture parses");
    let parsed = parsed.unwrap_or_else(|_| unreachable!());
    assert_eq!(
        MetadataSet::from_object(parsed.object()).err(),
        Some(MetadataError::UnsupportedPixelFragments(Tag(
            0x7fe0, 0x0010
        )))
    );
}

#[test]
fn json_person_names_remain_typed() {
    let person = ocelli_dicom::PersonName::new(
        Some("SYNTHETIC^ALPHA".to_owned()),
        Some("SYNTHETIC=IDEO".to_owned()),
        None,
    );
    let person_element = MetadataElement::person_names(vec![person.clone()]);
    assert_eq!(person_element.vr(), VR::PN);
    assert_eq!(
        person_element.value(),
        &MetadataValue::PersonNames(vec![person])
    );
}

#[test]
fn bulk_data_uri_accepts_every_ps318_f22_vr() {
    let allowed = [
        VR::DS,
        VR::FL,
        VR::FD,
        VR::IS,
        VR::LT,
        VR::OB,
        VR::OD,
        VR::OF,
        VR::OL,
        VR::OV,
        VR::OW,
        VR::SL,
        VR::SS,
        VR::ST,
        VR::SV,
        VR::UC,
        VR::UL,
        VR::UN,
        VR::US,
        VR::UT,
        VR::UV,
    ];
    for vr in allowed {
        let element = MetadataElement::bulk_data_uri(vr, "/bulk/synthetic".to_owned());
        assert!(element.is_ok(), "{}", vr.to_string());
        let uri = element.ok().and_then(|element| match element.value() {
            MetadataValue::BulkDataUri(uri) => Some(uri.as_str().to_owned()),
            _ => None,
        });
        assert_eq!(uri.as_deref(), Some("/bulk/synthetic"));
    }
}

#[test]
fn bulk_data_uri_rejects_vrs_outside_ps318_f22() {
    for vr in [VR::AE, VR::PN, VR::SQ, VR::UI] {
        assert_eq!(
            MetadataElement::bulk_data_uri(vr, "/bulk/synthetic".to_owned()).err(),
            Some(MetadataError::InvalidBulkDataUriVr(vr))
        );
    }
}

#[test]
fn inline_binary_accepts_every_ps318_f22_vr() {
    for vr in [VR::OB, VR::OD, VR::OF, VR::OL, VR::OV, VR::OW, VR::UN] {
        let element = MetadataElement::inline_binary(vr, "AAE=".to_owned());
        assert!(element.is_ok(), "{}", vr.to_string());
        let encoded = element.ok().and_then(|element| match element.value() {
            MetadataValue::InlineBinary(value) => Some(value.as_str().to_owned()),
            _ => None,
        });
        assert_eq!(encoded.as_deref(), Some("AAE="));
    }
}

#[test]
fn inline_binary_rejects_vrs_outside_ps318_f22() {
    for vr in [VR::DS, VR::PN, VR::SQ, VR::UT] {
        assert_eq!(
            MetadataElement::inline_binary(vr, "AA==".to_owned()).err(),
            Some(MetadataError::InvalidInlineBinaryVr(vr))
        );
    }
}

#[test]
fn inline_binary_rejects_empty_and_invalid_base64() {
    assert_eq!(
        MetadataElement::inline_binary(VR::OB, String::new()).err(),
        Some(MetadataError::EmptyInlineBinary)
    );

    assert_eq!(
        MetadataElement::inline_binary(VR::OB, "A===".to_owned()).err(),
        Some(MetadataError::InvalidInlineBinary)
    );
}

fn empty_provider(_request: MetadataRequest<'_>) -> Result<Option<MetadataElement>, MetadataError> {
    Ok(Some(MetadataElement::empty(VR::PN)))
}

fn absent_provider(
    _request: MetadataRequest<'_>,
) -> Result<Option<MetadataElement>, MetadataError> {
    Ok(None)
}

#[test]
fn ordered_provider_lookup_stops_on_present_empty_and_returns_identity() {
    let parsed = parsed_fixture();
    let mut registry = ProviderRegistry::new();
    assert!(
        registry
            .register(ProviderId::new("absent"), absent_provider)
            .is_ok()
    );
    assert!(
        registry
            .register(ProviderId::new("empty"), empty_provider)
            .is_ok()
    );
    assert!(
        registry
            .register(ProviderId::new("data-set"), data_set_provider)
            .is_ok()
    );

    let found = registry.lookup(MetadataRequest::new(&parsed, Tag(0x0010, 0x0010)));
    assert!(found.is_ok());
    let found = found.ok().flatten();
    assert_eq!(
        found.as_ref().map(|answer| answer.provider().as_str()),
        Some("empty")
    );
    assert_eq!(
        found.as_ref().map(|answer| answer.element().value()),
        Some(&MetadataValue::Empty)
    );
}

#[test]
fn registry_uses_provider_id_as_the_only_registration_identity() {
    let mut registry = ProviderRegistry::new();
    assert!(
        registry
            .register(ProviderId::new("main"), data_set_provider)
            .is_ok()
    );
    assert_eq!(
        registry
            .register(ProviderId::new("main"), file_meta_provider)
            .err(),
        Some(ProviderRegistryError::DuplicateProviderId(ProviderId::new(
            "main"
        )))
    );
    assert!(
        registry
            .register(ProviderId::new("alias"), data_set_provider)
            .is_ok()
    );
    assert_eq!(registry.len(), 2);
}

#[test]
fn production_providers_keep_data_set_and_file_meta_distinct() {
    let parsed = parsed_fixture();
    let mut registry = ProviderRegistry::new();
    assert!(
        registry
            .register(ProviderId::new("file-meta"), file_meta_provider)
            .is_ok()
    );
    assert!(
        registry
            .register(ProviderId::new("data-set"), data_set_provider)
            .is_ok()
    );

    let meta = registry.lookup(MetadataRequest::new(&parsed, Tag(0x0002, 0x0010)));
    assert!(meta.is_ok());
    assert_eq!(
        meta.ok()
            .flatten()
            .as_ref()
            .map(|answer| answer.provider().as_str()),
        Some("file-meta")
    );
    let data = registry.lookup(MetadataRequest::new(&parsed, Tag(0x0008, 0x0016)));
    assert!(data.is_ok());
    assert_eq!(
        data.ok()
            .flatten()
            .as_ref()
            .map(|answer| answer.provider().as_str()),
        Some("data-set")
    );
}

proptest! {
    #[test]
    fn primitive_projection_preserves_i16(values in proptest::collection::vec(any::<i16>(), 0..8)) {
        let projected = MetadataValue::from_primitive(&PrimitiveValue::I16(values.clone().into()));
        prop_assert_eq!(projected, MetadataValue::Signed16(values));
    }

    #[test]
    fn primitive_projection_preserves_i32(values in proptest::collection::vec(any::<i32>(), 0..8)) {
        let projected = MetadataValue::from_primitive(&PrimitiveValue::I32(values.clone().into()));
        prop_assert_eq!(projected, MetadataValue::Signed32(values));
    }

    #[test]
    fn primitive_projection_preserves_u16(values in proptest::collection::vec(any::<u16>(), 0..8)) {
        let projected = MetadataValue::from_primitive(&PrimitiveValue::U16(values.clone().into()));
        prop_assert_eq!(projected, MetadataValue::Unsigned16(values));
    }

    #[test]
    fn primitive_projection_preserves_u32(values in proptest::collection::vec(any::<u32>(), 0..8)) {
        let projected = MetadataValue::from_primitive(&PrimitiveValue::U32(values.clone().into()));
        prop_assert_eq!(projected, MetadataValue::Unsigned32(values));
    }
}
