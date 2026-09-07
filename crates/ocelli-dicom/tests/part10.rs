//! Part 10 and transfer-syntax dispatch fixtures for F-016.
//!
//! The byte layouts here are independent inputs, not output from Ocelli.
//! PS3.10 sections 7 and 7.1 require the 128-byte preamble, `DICM` prefix,
//! Explicit VR Little Endian File Meta Information, and Transfer Syntax UID.
//! PS3.5 annex A defines each data-set encoding exercised below.

use ocelli_dicom::{DispatchPath, ParseError, Tag, parse_part10};

const IMPLICIT_VR_LE: &str = "1.2.840.10008.1.2";
const EXPLICIT_VR_LE: &str = "1.2.840.10008.1.2.1";
const DEFLATED_EXPLICIT_VR_LE: &str = "1.2.840.10008.1.2.1.99";
const EXPLICIT_VR_BE: &str = "1.2.840.10008.1.2.2";
const JPEG_BASELINE: &str = "1.2.840.10008.1.2.4.50";

const SOP_CLASS_UID: &[u8] = b"1.2.840.10008.5.1.4.1.1.2\0";
const SOP_INSTANCE_UID: &[u8] = b"2.25.1";
const IMPLEMENTATION_UID: &[u8] = b"2.25.2";

const DEFLATED_DATA_SET: &[u8] = &[
    // Python 3 compressobj(wbits=-15) over explicit_little_data_set(), fixed
    // at the default level. PS3.5 A.5 specifies an RFC 1951 Deflate stream.
    227, 96, 16, 99, 8, 245, 148, 98, 48, 212, 51, 210, 179, 48, 49, 208, 51, 52, 48, 48, 176, 208,
    51, 213, 51, 212, 51, 1, 98, 160, 40, 131, 6, 3, 7, 131, 103, 48, 19, 131, 145, 130, 6, 131, 0,
    67, 40, 144, 197, 4, 20, 51, 96, 112, 9, 230, 96, 48, 208, 51, 141, 49, 208, 51, 50, 5, 0,
];

fn explicit_element_le(tag: (u16, u16), vr: [u8; 2], value: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&tag.0.to_le_bytes());
    bytes.extend_from_slice(&tag.1.to_le_bytes());
    bytes.extend_from_slice(&vr);
    let length = fixture_u16(value.len());
    bytes.extend_from_slice(&length.to_le_bytes());
    bytes.extend_from_slice(value);
    bytes
}

fn explicit_element_be(tag: (u16, u16), vr: [u8; 2], value: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&tag.0.to_be_bytes());
    bytes.extend_from_slice(&tag.1.to_be_bytes());
    bytes.extend_from_slice(&vr);
    let length = fixture_u16(value.len());
    bytes.extend_from_slice(&length.to_be_bytes());
    bytes.extend_from_slice(value);
    bytes
}

fn implicit_element_le(tag: (u16, u16), value: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&tag.0.to_le_bytes());
    bytes.extend_from_slice(&tag.1.to_le_bytes());
    let length = fixture_u32(value.len());
    bytes.extend_from_slice(&length.to_le_bytes());
    bytes.extend_from_slice(value);
    bytes
}

fn fixture_u16(length: usize) -> u16 {
    assert_eq!(
        length % 2,
        0,
        "DICOM element fixture values have even length"
    );
    assert!(u16::try_from(length).is_ok(), "fixture length fits in u16");
    u16::try_from(length).unwrap_or(u16::MAX)
}

fn fixture_u32(length: usize) -> u32 {
    assert_eq!(
        length % 2,
        0,
        "DICOM element fixture values have even length"
    );
    assert!(u32::try_from(length).is_ok(), "fixture length fits in u32");
    u32::try_from(length).unwrap_or(u32::MAX)
}

fn long_explicit_element_le(tag: (u16, u16), vr: [u8; 2], value: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&tag.0.to_le_bytes());
    bytes.extend_from_slice(&tag.1.to_le_bytes());
    bytes.extend_from_slice(&vr);
    bytes.extend_from_slice(&[0, 0]);
    let length = fixture_u32(value.len());
    bytes.extend_from_slice(&length.to_le_bytes());
    bytes.extend_from_slice(value);
    bytes
}

fn file_meta(transfer_syntax: &str) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend(long_explicit_element_le((0x0002, 0x0001), *b"OB", &[0, 1]));
    body.extend(explicit_element_le((0x0002, 0x0002), *b"UI", SOP_CLASS_UID));
    body.extend(explicit_element_le(
        (0x0002, 0x0003),
        *b"UI",
        SOP_INSTANCE_UID,
    ));

    let mut syntax = transfer_syntax.as_bytes().to_vec();
    if !syntax.len().is_multiple_of(2) {
        syntax.push(0);
    }
    body.extend(explicit_element_le((0x0002, 0x0010), *b"UI", &syntax));
    body.extend(explicit_element_le(
        (0x0002, 0x0012),
        *b"UI",
        IMPLEMENTATION_UID,
    ));

    let mut meta = explicit_element_le(
        (0x0002, 0x0000),
        *b"UL",
        &fixture_u32(body.len()).to_le_bytes(),
    );
    meta.extend(body);
    meta
}

fn part10(transfer_syntax: &str, data_set: &[u8]) -> Vec<u8> {
    let mut bytes = vec![0; 128];
    bytes.extend_from_slice(b"DICM");
    bytes.extend(file_meta(transfer_syntax));
    bytes.extend_from_slice(data_set);
    bytes
}

fn explicit_little_data_set() -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend(explicit_element_le((0x0008, 0x0016), *b"UI", SOP_CLASS_UID));
    bytes.extend(explicit_element_le((0x0028, 0x0008), *b"IS", b"2 "));
    bytes.extend(explicit_element_le(
        (0x0028, 0x0010),
        *b"US",
        &2_u16.to_le_bytes(),
    ));
    bytes.extend(explicit_element_le((0x0028, 0x0030), *b"DS", b"0.5\\0.25"));
    bytes
}

fn implicit_little_data_set() -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend(implicit_element_le((0x0008, 0x0016), SOP_CLASS_UID));
    bytes.extend(implicit_element_le((0x0028, 0x0008), b"2 "));
    bytes.extend(implicit_element_le((0x0028, 0x0010), &2_u16.to_le_bytes()));
    bytes.extend(implicit_element_le((0x0028, 0x0030), b"0.5\\0.25"));
    bytes
}

fn explicit_big_data_set() -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend(explicit_element_be((0x0008, 0x0016), *b"UI", SOP_CLASS_UID));
    bytes.extend(explicit_element_be((0x0028, 0x0008), *b"IS", b"2 "));
    bytes.extend(explicit_element_be(
        (0x0028, 0x0010),
        *b"US",
        &2_u16.to_be_bytes(),
    ));
    bytes.extend(explicit_element_be((0x0028, 0x0030), *b"DS", b"0.5\\0.25"));
    bytes
}

fn encapsulated_data_set() -> Vec<u8> {
    let mut bytes = explicit_little_data_set();
    bytes.extend_from_slice(&0x7fe0_u16.to_le_bytes());
    bytes.extend_from_slice(&0x0010_u16.to_le_bytes());
    bytes.extend_from_slice(b"OB");
    bytes.extend_from_slice(&[0, 0]);
    bytes.extend_from_slice(&u32::MAX.to_le_bytes());
    bytes.extend_from_slice(&0xfffe_u16.to_le_bytes());
    bytes.extend_from_slice(&0xe000_u16.to_le_bytes());
    bytes.extend_from_slice(&0_u32.to_le_bytes());
    bytes.extend_from_slice(&0xfffe_u16.to_le_bytes());
    bytes.extend_from_slice(&0xe000_u16.to_le_bytes());
    bytes.extend_from_slice(&4_u32.to_le_bytes());
    bytes.extend_from_slice(&[1, 2, 3, 4]);
    bytes.extend_from_slice(&0xfffe_u16.to_le_bytes());
    bytes.extend_from_slice(&0xe0dd_u16.to_le_bytes());
    bytes.extend_from_slice(&0_u32.to_le_bytes());
    bytes
}

fn encapsulated_data_set_with_item_delimiter_first() -> Vec<u8> {
    let mut bytes = explicit_little_data_set();
    bytes.extend_from_slice(&0x7fe0_u16.to_le_bytes());
    bytes.extend_from_slice(&0x0010_u16.to_le_bytes());
    bytes.extend_from_slice(b"OB");
    bytes.extend_from_slice(&[0, 0]);
    bytes.extend_from_slice(&u32::MAX.to_le_bytes());
    bytes.extend_from_slice(&0xfffe_u16.to_le_bytes());
    bytes.extend_from_slice(&0xe00d_u16.to_le_bytes());
    bytes.extend_from_slice(&0_u32.to_le_bytes());
    bytes
}

fn odd_length_data_set() -> Vec<u8> {
    let mut bytes = explicit_little_data_set();
    bytes.extend_from_slice(&0x0028_u16.to_le_bytes());
    bytes.extend_from_slice(&0x0100_u16.to_le_bytes());
    bytes.extend_from_slice(b"US");
    bytes.extend_from_slice(&1_u16.to_le_bytes());
    bytes.push(8);
    bytes
}

fn forged_fixed_sentinel_before_truncated_value() -> Vec<u8> {
    let mut bytes = explicit_little_data_set();
    bytes.extend(explicit_element_le(
        (0x0002, 0x0000),
        *b"UL",
        &0_u32.to_le_bytes(),
    ));
    bytes.extend_from_slice(&0x0028_u16.to_le_bytes());
    bytes.extend_from_slice(&0x0101_u16.to_le_bytes());
    bytes.extend_from_slice(b"OB");
    bytes.extend_from_slice(&[0, 0]);
    bytes.extend_from_slice(&12_u32.to_le_bytes());
    bytes
}

fn header_with_missing_twelve_byte_value() -> Vec<u8> {
    let mut bytes = explicit_little_data_set();
    bytes.extend_from_slice(&0x0028_u16.to_le_bytes());
    bytes.extend_from_slice(&0x0101_u16.to_le_bytes());
    bytes.extend_from_slice(b"OB");
    bytes.extend_from_slice(&[0, 0]);
    bytes.extend_from_slice(&12_u32.to_le_bytes());
    bytes
}

fn assert_route(transfer_syntax: &str, data_set: &[u8], route: DispatchPath) {
    let result = parse_part10(&part10(transfer_syntax, data_set));
    assert!(result.is_ok(), "the independently encoded fixture parses");
    if let Ok(parsed) = result {
        assert_eq!(parsed.transfer_syntax().uid(), transfer_syntax);
        assert_eq!(parsed.transfer_syntax().dispatch_path(), route);
        assert_eq!(
            parsed
                .object()
                .element(Tag(0x0028, 0x0010))
                .ok()
                .and_then(|element| element.to_int::<u16>().ok()),
            Some(2)
        );
    }
}

#[test]
fn file_meta_selects_implicit_vr_little_endian() {
    assert_route(
        IMPLICIT_VR_LE,
        &implicit_little_data_set(),
        DispatchPath::ImplicitVrLittleEndian,
    );
}

#[test]
fn file_meta_selects_explicit_vr_little_endian() {
    assert_route(
        EXPLICIT_VR_LE,
        &explicit_little_data_set(),
        DispatchPath::ExplicitVrLittleEndian,
    );
}

#[test]
fn file_meta_selects_retired_explicit_vr_big_endian() {
    assert_route(
        EXPLICIT_VR_BE,
        &explicit_big_data_set(),
        DispatchPath::ExplicitVrBigEndian,
    );
}

#[test]
fn file_meta_selects_deflated_explicit_vr_little_endian() {
    assert_route(
        DEFLATED_EXPLICIT_VR_LE,
        DEFLATED_DATA_SET,
        DispatchPath::DeflatedExplicitVrLittleEndian,
    );
}

#[test]
fn encapsulated_pixel_data_stays_encapsulated() {
    let result = parse_part10(&part10(JPEG_BASELINE, &encapsulated_data_set()));
    assert!(
        result.is_ok(),
        "encapsulated data set parses without pixel decode"
    );
    if let Ok(parsed) = result {
        assert_eq!(
            parsed.transfer_syntax().dispatch_path(),
            DispatchPath::Encapsulated
        );
        assert!(
            parsed
                .object()
                .element(Tag(0x7fe0, 0x0010))
                .is_ok_and(|element| element.value().fragments().is_some())
        );
    }
}

#[test]
fn string_padding_and_multiplicity_remain_available() {
    let result = parse_part10(&part10(EXPLICIT_VR_LE, &explicit_little_data_set()));
    assert!(result.is_ok());
    if let Ok(parsed) = result {
        assert_eq!(
            parsed
                .object()
                .element(Tag(0x0028, 0x0008))
                .ok()
                .and_then(|element| element.to_str().ok())
                .as_deref(),
            Some("2")
        );
        assert_eq!(
            parsed
                .object()
                .element(Tag(0x0028, 0x0030))
                .ok()
                .map(|element| element.value().multiplicity()),
            Some(2)
        );
        assert_eq!(parsed.object().meta().transfer_syntax(), EXPLICIT_VR_LE);
    }
}

#[test]
fn short_part10_input_is_refused() {
    assert_eq!(
        parse_part10(&[0; 131]).err(),
        Some(ParseError::TruncatedPart10)
    );
}

#[test]
fn wrong_part10_prefix_is_refused() {
    let mut bytes = vec![0; 128];
    bytes.extend_from_slice(b"NOPE");
    assert_eq!(
        parse_part10(&bytes).err(),
        Some(ParseError::MissingDicmPrefix)
    );
}

#[test]
fn malformed_file_meta_is_refused() {
    let mut bytes = vec![0; 128];
    bytes.extend_from_slice(b"DICM");
    bytes.extend_from_slice(&[2, 0, 0, 0, b'U', b'L', 4, 0, 1]);
    assert_eq!(
        parse_part10(&bytes).err(),
        Some(ParseError::InvalidFileMeta)
    );
}

#[test]
fn missing_transfer_syntax_is_refused() {
    let mut bytes = vec![0; 128];
    bytes.extend_from_slice(b"DICM");
    bytes.extend(explicit_element_le(
        (0x0002, 0x0000),
        *b"UL",
        &0_u32.to_le_bytes(),
    ));
    assert_eq!(
        parse_part10(&bytes).err(),
        Some(ParseError::MissingTransferSyntax)
    );
}

#[test]
fn unknown_transfer_syntax_is_refused_without_fallback() {
    let bytes = part10("1.2.826.0.1.3680043.10.999", &explicit_little_data_set());
    assert_eq!(
        parse_part10(&bytes).err(),
        Some(ParseError::UnknownTransferSyntax)
    );
}

#[test]
fn truncated_data_set_is_refused_without_fallback() {
    let mut data_set = explicit_little_data_set();
    data_set.truncate(data_set.len() - 1);
    assert_eq!(
        parse_part10(&part10(EXPLICIT_VR_LE, &data_set)).err(),
        Some(ParseError::TruncatedDataSet)
    );
}

#[test]
fn partial_top_level_headers_are_refused() {
    for tail in [&[0x08_u8][..], &[0x08, 0x00][..], &[0x08, 0x00, 0x20][..]] {
        let mut data_set = explicit_little_data_set();
        data_set.extend_from_slice(tail);
        assert_eq!(
            parse_part10(&part10(EXPLICIT_VR_LE, &data_set)).err(),
            Some(ParseError::TruncatedDataSet)
        );
    }
}

#[test]
fn unterminated_encapsulated_pixel_data_is_refused() {
    let mut data_set = encapsulated_data_set();
    data_set.truncate(data_set.len() - 8);
    assert_eq!(
        parse_part10(&part10(JPEG_BASELINE, &data_set)).err(),
        Some(ParseError::TruncatedDataSet)
    );
}

#[test]
fn structurally_invalid_data_set_is_distinct_from_truncation() {
    assert_eq!(
        parse_part10(&part10(
            JPEG_BASELINE,
            &encapsulated_data_set_with_item_delimiter_first()
        ))
        .err(),
        Some(ParseError::InvalidDataSet)
    );
}

#[test]
fn odd_value_length_is_refused() {
    assert_eq!(
        parse_part10(&part10(EXPLICIT_VR_LE, &odd_length_data_set())).err(),
        Some(ParseError::InvalidDataSet)
    );
}

#[test]
fn input_tag_cannot_forge_completion_after_a_truncated_value() {
    assert_eq!(
        parse_part10(&part10(
            EXPLICIT_VR_LE,
            &forged_fixed_sentinel_before_truncated_value()
        ))
        .err(),
        Some(ParseError::InvalidDataSet)
    );
}

#[test]
fn file_meta_group_is_refused_in_the_main_data_set() {
    let mut data_set = explicit_little_data_set();
    data_set.extend(long_explicit_element_le((0x0002, 0x0001), *b"OB", &[0, 1]));
    assert_eq!(
        parse_part10(&part10(EXPLICIT_VR_LE, &data_set)).err(),
        Some(ParseError::InvalidDataSet)
    );
}

#[test]
fn completion_bytes_consumed_as_a_value_do_not_signal_success() {
    assert_eq!(
        parse_part10(&part10(
            EXPLICIT_VR_LE,
            &header_with_missing_twelve_byte_value()
        ))
        .err(),
        Some(ParseError::TruncatedDataSet)
    );
}
