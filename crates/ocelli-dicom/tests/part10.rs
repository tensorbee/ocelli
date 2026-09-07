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
    67, 40, 144, 197, 4, 20, 51, 96, 112, 9, 230, 96, 48, 208, 51, 141, 49, 208, 51, 50, 5, 0, 0,
];

// Python 3 compressobj(wbits=-15) over the Explicit VR Little Endian sequence
// fixtures below, fixed at the default level. An odd stream has its required
// PS3.5 A.5 NULL padding byte appended.
const DEFLATED_DEFINED_SEQUENCE_WITH_ITEM: &[u8] = &[
    227, 96, 16, 16, 12, 14, 100, 96, 224, 96, 96, 96, 248, 247, 159, 225, 1, 144, 98, 0, 0, 0,
];
const DEFLATED_DEFINED_SEQUENCE_WITH_ITEM_DELIMITER: &[u8] = &[
    227, 96, 16, 16, 12, 14, 100, 96, 224, 96, 96, 96, 248, 247, 159, 247, 1, 144, 98, 0, 0, 0,
];
const DEFLATED_DEFINED_SEQUENCE_WITH_SEQUENCE_DELIMITER: &[u8] = &[
    227, 96, 16, 16, 12, 14, 100, 96, 224, 96, 96, 96, 248, 247, 255, 238, 3, 32, 197, 0, 0, 0,
];
const DEFLATED_UNDEFINED_ITEM_WITH_NONZERO_DELIMITER: &[u8] = &[
    227, 96, 16, 16, 12, 14, 100, 96, 248, 15, 4, 255, 254, 51, 60, 128, 208, 188, 15, 88, 24, 24,
    24, 254, 253, 191, 251, 0, 72, 49, 0, 0,
];
const DEFLATED_UNDEFINED_SEQUENCE_WITH_NONZERO_DELIMITER: &[u8] = &[
    227, 96, 16, 16, 12, 14, 100, 96, 248, 15, 4, 255, 254, 223, 125, 192, 194, 192, 192, 0, 0, 0,
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

fn long_explicit_element_be(tag: (u16, u16), vr: [u8; 2], value: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&tag.0.to_be_bytes());
    bytes.extend_from_slice(&tag.1.to_be_bytes());
    bytes.extend_from_slice(&vr);
    bytes.extend_from_slice(&[0, 0]);
    let length = fixture_u32(value.len());
    bytes.extend_from_slice(&length.to_be_bytes());
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
    encapsulated_data_set_with_delimiter_length(0)
}

fn encapsulated_data_set_with_delimiter_length(delimiter_length: u32) -> Vec<u8> {
    let mut bytes = encapsulated_pixel_data_header();
    bytes.extend_from_slice(&0xfffe_u16.to_le_bytes());
    bytes.extend_from_slice(&0xe000_u16.to_le_bytes());
    bytes.extend_from_slice(&0_u32.to_le_bytes());
    bytes.extend_from_slice(&0xfffe_u16.to_le_bytes());
    bytes.extend_from_slice(&0xe000_u16.to_le_bytes());
    bytes.extend_from_slice(&4_u32.to_le_bytes());
    bytes.extend_from_slice(&[1, 2, 3, 4]);
    bytes.extend_from_slice(&0xfffe_u16.to_le_bytes());
    bytes.extend_from_slice(&0xe0dd_u16.to_le_bytes());
    bytes.extend_from_slice(&delimiter_length.to_le_bytes());
    bytes
}

fn encapsulated_pixel_data_header() -> Vec<u8> {
    encapsulated_pixel_data_header_with_vr(*b"OB")
}

fn encapsulated_pixel_data_header_with_vr(vr: [u8; 2]) -> Vec<u8> {
    let mut bytes = explicit_little_data_set();
    bytes.extend_from_slice(&0x7fe0_u16.to_le_bytes());
    bytes.extend_from_slice(&0x0010_u16.to_le_bytes());
    bytes.extend_from_slice(&vr);
    bytes.extend_from_slice(&[0, 0]);
    bytes.extend_from_slice(&u32::MAX.to_le_bytes());
    bytes
}

fn encapsulated_data_set_with_vr(vr: [u8; 2]) -> Vec<u8> {
    let mut bytes = encapsulated_pixel_data_header_with_vr(vr);
    bytes.extend(item_control_tag(0xe000));
    bytes.extend(item_control_tag_with_length(0xe000, 4));
    bytes.extend_from_slice(&[1, 2, 3, 4]);
    bytes.extend(item_control_tag(0xe0dd));
    bytes
}

fn encapsulated_data_set_without_fragment() -> Vec<u8> {
    let mut bytes = encapsulated_pixel_data_header();
    bytes.extend(item_control_tag(0xe000));
    bytes.extend(item_control_tag(0xe0dd));
    bytes
}

fn encapsulated_data_set_with_fragment(fragment: &[u8]) -> Vec<u8> {
    let mut bytes = encapsulated_pixel_data_header();
    bytes.extend(item_control_tag(0xe000));
    bytes.extend(item_control_tag_with_length(
        0xe000,
        fixture_u32(fragment.len()),
    ));
    bytes.extend_from_slice(fragment);
    bytes.extend(item_control_tag(0xe0dd));
    bytes
}

fn icon_image_sequence_with_native_pixel_data() -> Vec<u8> {
    icon_image_sequence_with_pixel_data_vr(*b"OB")
}

fn icon_image_sequence_with_pixel_data_vr(vr: [u8; 2]) -> Vec<u8> {
    let native_pixel_data = long_explicit_element_le((0x7fe0, 0x0010), vr, &[1, 2, 3, 4]);
    let mut item = item_control_tag_with_length(0xe000, fixture_u32(native_pixel_data.len()));
    item.extend(native_pixel_data);
    long_explicit_element_le((0x0088, 0x0200), *b"SQ", &item)
}

fn encapsulated_data_set_without_basic_offset_table() -> Vec<u8> {
    let mut bytes = encapsulated_pixel_data_header();
    bytes.extend(item_control_tag(0xe0dd));
    bytes
}

fn encapsulated_data_set_with_two_byte_basic_offset_table() -> Vec<u8> {
    let mut bytes = encapsulated_pixel_data_header();
    bytes.extend(item_control_tag_with_length(0xe000, 2));
    bytes.extend_from_slice(&[0, 0]);
    bytes.extend(item_control_tag(0xe0dd));
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

fn item_control_tag(element: u16) -> Vec<u8> {
    item_control_tag_with_length(element, 0)
}

fn item_control_tag_with_length(element: u16, length: u32) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&0xfffe_u16.to_le_bytes());
    bytes.extend_from_slice(&element.to_le_bytes());
    bytes.extend_from_slice(&length.to_le_bytes());
    bytes
}

fn item_control_tag_be(element: u16) -> Vec<u8> {
    item_control_tag_be_with_length(element, 0)
}

fn item_control_tag_be_with_length(element: u16, length: u32) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&0xfffe_u16.to_be_bytes());
    bytes.extend_from_slice(&element.to_be_bytes());
    bytes.extend_from_slice(&length.to_be_bytes());
    bytes
}

fn undefined_explicit_sequence_le(value: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&0x0008_u16.to_le_bytes());
    bytes.extend_from_slice(&0x1110_u16.to_le_bytes());
    bytes.extend_from_slice(b"SQ");
    bytes.extend_from_slice(&[0, 0]);
    bytes.extend_from_slice(&u32::MAX.to_le_bytes());
    bytes.extend_from_slice(value);
    bytes
}

fn undefined_implicit_sequence_le(value: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&0x0008_u16.to_le_bytes());
    bytes.extend_from_slice(&0x1110_u16.to_le_bytes());
    bytes.extend_from_slice(&u32::MAX.to_le_bytes());
    bytes.extend_from_slice(value);
    bytes
}

fn undefined_explicit_sequence_be(value: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&0x0008_u16.to_be_bytes());
    bytes.extend_from_slice(&0x1110_u16.to_be_bytes());
    bytes.extend_from_slice(b"SQ");
    bytes.extend_from_slice(&[0, 0]);
    bytes.extend_from_slice(&u32::MAX.to_be_bytes());
    bytes.extend_from_slice(value);
    bytes
}

fn assert_defined_sequence_route_results(
    element: u16,
    deflated_data_set: &[u8],
    expected: Result<(), ParseError>,
) {
    let item_le = item_control_tag(element);
    let item_be = item_control_tag_be(element);
    let explicit_le = long_explicit_element_le((0x0008, 0x1110), *b"SQ", &item_le);
    let cases = [
        (
            IMPLICIT_VR_LE,
            implicit_element_le((0x0008, 0x1110), &item_le),
        ),
        (EXPLICIT_VR_LE, explicit_le.clone()),
        (
            EXPLICIT_VR_BE,
            long_explicit_element_be((0x0008, 0x1110), *b"SQ", &item_be),
        ),
        (DEFLATED_EXPLICIT_VR_LE, deflated_data_set.to_vec()),
        (JPEG_BASELINE, explicit_le),
    ];

    for (transfer_syntax, data_set) in cases {
        let result = parse_part10(&part10(transfer_syntax, &data_set)).map(|_| ());
        assert_eq!(result, expected, "transfer syntax {transfer_syntax}");
    }
}

fn assert_undefined_sequence_refused_on_every_route(
    value_le: &[u8],
    value_be: &[u8],
    deflated_data_set: &[u8],
) {
    let explicit_le = undefined_explicit_sequence_le(value_le);
    let cases = [
        (IMPLICIT_VR_LE, undefined_implicit_sequence_le(value_le)),
        (EXPLICIT_VR_LE, explicit_le.clone()),
        (EXPLICIT_VR_BE, undefined_explicit_sequence_be(value_be)),
        (DEFLATED_EXPLICIT_VR_LE, deflated_data_set.to_vec()),
        (JPEG_BASELINE, explicit_le),
    ];

    for (transfer_syntax, data_set) in cases {
        assert_eq!(
            parse_part10(&part10(transfer_syntax, &data_set)).err(),
            Some(ParseError::InvalidDataSet),
            "transfer syntax {transfer_syntax}"
        );
    }
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
fn top_level_item_is_refused_as_invalid_structure() {
    let data_set = item_control_tag(0xe000);
    assert_eq!(
        parse_part10(&part10(EXPLICIT_VR_LE, &data_set)).err(),
        Some(ParseError::InvalidDataSet)
    );
}

#[test]
fn top_level_item_delimiter_is_refused_as_invalid_structure() {
    let data_set = item_control_tag(0xe00d);
    assert_eq!(
        parse_part10(&part10(EXPLICIT_VR_LE, &data_set)).err(),
        Some(ParseError::InvalidDataSet)
    );
}

#[test]
fn top_level_sequence_delimiter_is_refused_as_invalid_structure() {
    let data_set = item_control_tag(0xe0dd);
    assert_eq!(
        parse_part10(&part10(EXPLICIT_VR_LE, &data_set)).err(),
        Some(ParseError::InvalidDataSet)
    );
}

#[test]
fn defined_sequence_accepts_an_empty_item_on_every_route() {
    assert_defined_sequence_route_results(0xe000, DEFLATED_DEFINED_SEQUENCE_WITH_ITEM, Ok(()));
}

#[test]
fn defined_sequence_refuses_item_delimiter_without_an_item_on_every_route() {
    assert_defined_sequence_route_results(
        0xe00d,
        DEFLATED_DEFINED_SEQUENCE_WITH_ITEM_DELIMITER,
        Err(ParseError::InvalidDataSet),
    );
}

#[test]
fn defined_sequence_refuses_sequence_delimiter_on_every_route() {
    assert_defined_sequence_route_results(
        0xe0dd,
        DEFLATED_DEFINED_SEQUENCE_WITH_SEQUENCE_DELIMITER,
        Err(ParseError::InvalidDataSet),
    );
}

#[test]
fn undefined_item_refuses_a_nonzero_delimiter_length_on_every_route() {
    let mut value_le = item_control_tag_with_length(0xe000, u32::MAX);
    value_le.extend(item_control_tag_with_length(0xe00d, 4));
    value_le.extend(item_control_tag(0xe0dd));
    let mut value_be = item_control_tag_be_with_length(0xe000, u32::MAX);
    value_be.extend(item_control_tag_be_with_length(0xe00d, 4));
    value_be.extend(item_control_tag_be(0xe0dd));

    assert_undefined_sequence_refused_on_every_route(
        &value_le,
        &value_be,
        DEFLATED_UNDEFINED_ITEM_WITH_NONZERO_DELIMITER,
    );
}

#[test]
fn undefined_sequence_refuses_a_nonzero_delimiter_length_on_every_route() {
    assert_undefined_sequence_refused_on_every_route(
        &item_control_tag_with_length(0xe0dd, 4),
        &item_control_tag_be_with_length(0xe0dd, 4),
        DEFLATED_UNDEFINED_SEQUENCE_WITH_NONZERO_DELIMITER,
    );
}

#[test]
fn encapsulated_pixel_data_refuses_a_nonzero_delimiter_length() {
    assert_eq!(
        parse_part10(&part10(
            JPEG_BASELINE,
            &encapsulated_data_set_with_delimiter_length(4),
        ))
        .err(),
        Some(ParseError::InvalidDataSet)
    );
}

#[test]
fn native_syntax_refuses_encapsulated_pixel_data() {
    assert_eq!(
        parse_part10(&part10(EXPLICIT_VR_LE, &encapsulated_data_set())).err(),
        Some(ParseError::InvalidDataSet)
    );
}

#[test]
fn encapsulated_syntax_refuses_native_pixel_data() {
    let native_pixel_data = long_explicit_element_le((0x7fe0, 0x0010), *b"OB", &[1, 2, 3, 4]);
    assert_eq!(
        parse_part10(&part10(JPEG_BASELINE, &native_pixel_data)).err(),
        Some(ParseError::InvalidDataSet)
    );
}

#[test]
fn encapsulated_syntax_accepts_nested_native_pixel_data() {
    assert!(
        parse_part10(&part10(
            JPEG_BASELINE,
            &icon_image_sequence_with_native_pixel_data(),
        ))
        .is_ok()
    );
}

#[test]
fn encapsulated_pixel_data_requires_ob_vr() {
    for vr in [*b"OW", *b"SQ"] {
        assert_eq!(
            parse_part10(&part10(JPEG_BASELINE, &encapsulated_data_set_with_vr(vr))).err(),
            Some(ParseError::InvalidDataSet),
            "VR {}{}",
            char::from(vr[0]),
            char::from(vr[1]),
        );
    }
}

#[test]
fn pixel_data_cannot_be_encoded_as_a_defined_length_sequence() {
    let pixel_data_sequence = long_explicit_element_le((0x7fe0, 0x0010), *b"SQ", &[]);
    for transfer_syntax in [EXPLICIT_VR_LE, JPEG_BASELINE] {
        assert_eq!(
            parse_part10(&part10(transfer_syntax, &pixel_data_sequence)).err(),
            Some(ParseError::InvalidDataSet),
            "transfer syntax {transfer_syntax}",
        );
    }
}

#[test]
fn native_pixel_data_requires_ob_or_ow_vr() {
    let top_level = explicit_element_le((0x7fe0, 0x0010), *b"UI", b"1.2\0");
    let nested = icon_image_sequence_with_pixel_data_vr(*b"UI");
    for (transfer_syntax, data_set) in [(EXPLICIT_VR_LE, top_level), (JPEG_BASELINE, nested)] {
        assert_eq!(
            parse_part10(&part10(transfer_syntax, &data_set)).err(),
            Some(ParseError::InvalidDataSet),
            "transfer syntax {transfer_syntax}",
        );
    }
}

#[test]
fn encapsulated_pixel_data_requires_a_basic_offset_table_item() {
    assert_eq!(
        parse_part10(&part10(
            JPEG_BASELINE,
            &encapsulated_data_set_without_basic_offset_table(),
        ))
        .err(),
        Some(ParseError::InvalidDataSet)
    );
}

#[test]
fn basic_offset_table_length_must_be_a_multiple_of_four() {
    assert_eq!(
        parse_part10(&part10(
            JPEG_BASELINE,
            &encapsulated_data_set_with_two_byte_basic_offset_table(),
        ))
        .err(),
        Some(ParseError::InvalidDataSet)
    );
}

#[test]
fn encapsulated_pixel_data_requires_a_fragment_after_the_offset_table() {
    assert_eq!(
        parse_part10(&part10(
            JPEG_BASELINE,
            &encapsulated_data_set_without_fragment(),
        ))
        .err(),
        Some(ParseError::InvalidDataSet)
    );
}

#[test]
fn encapsulated_pixel_data_refuses_an_empty_fragment() {
    assert_eq!(
        parse_part10(&part10(
            JPEG_BASELINE,
            &encapsulated_data_set_with_fragment(&[]),
        ))
        .err(),
        Some(ParseError::InvalidDataSet)
    );
}

#[test]
fn encapsulated_pixel_data_accepts_a_two_byte_fragment() {
    assert!(
        parse_part10(&part10(
            JPEG_BASELINE,
            &encapsulated_data_set_with_fragment(&[1, 2]),
        ))
        .is_ok()
    );
}

#[test]
fn deflated_data_set_requires_null_padding_when_the_stream_length_is_odd() {
    let result = DEFLATED_DATA_SET
        .split_last()
        .and_then(|(_, unpadded)| parse_part10(&part10(DEFLATED_EXPLICIT_VR_LE, unpadded)).err());
    assert_eq!(result, Some(ParseError::InvalidDataSet));
}

#[test]
fn deflated_data_set_refuses_bytes_after_its_required_padding() {
    let mut data_set = DEFLATED_DATA_SET.to_vec();
    data_set.extend_from_slice(&[1, 2]);
    assert_eq!(
        parse_part10(&part10(DEFLATED_EXPLICIT_VR_LE, &data_set)).err(),
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
