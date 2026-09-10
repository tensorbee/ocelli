//! DICOMweb response fixtures for F-021.
//!
//! The JSON shapes come from DICOM PS3.18 2026c Annex F. Multipart framing
//! comes from PS3.18 sections 8.6 and 8.7. Every identifier and payload here
//! is synthetic and hand encoded.

use std::collections::BTreeSet;

use ocelli_codec::{Capability, KNOWN_TRANSFER_SYNTAXES, Registry};
use ocelli_dicom::{
    DicomwebSource, MetadataSet, MetadataValue, SeriesSource, SourceBatch, SourceError,
    SourceResponse, SourceResponseKind, Tag, parse_part10,
};

const EXPLICIT_VR_LE: &str = "1.2.840.10008.1.2.1";
const SOP_CLASS_UID: &[u8] = b"1.2.840.10008.5.1.4.1.1.2\0";
const SOP_INSTANCE_UID: &[u8] = b"2.25.1";
const IMPLEMENTATION_UID: &[u8] = b"2.25.2";

fn fixture_u16(length: usize) -> u16 {
    assert!(u16::try_from(length).is_ok());
    u16::try_from(length).unwrap_or(u16::MAX)
}

fn fixture_u32(length: usize) -> u32 {
    assert!(u32::try_from(length).is_ok());
    u32::try_from(length).unwrap_or(u32::MAX)
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

fn part10(rows: u16) -> Vec<u8> {
    let mut data_set = Vec::new();
    data_set.extend(explicit_element((0x0008, 0x0016), *b"UI", SOP_CLASS_UID));
    data_set.extend(explicit_element(
        (0x0028, 0x0010),
        *b"US",
        &rows.to_le_bytes(),
    ));
    part10_with_syntax(EXPLICIT_VR_LE, &data_set)
}

fn part10_with_syntax(transfer_syntax: &str, data_set: &[u8]) -> Vec<u8> {
    let mut meta_body = Vec::new();
    meta_body.extend(long_explicit_element((0x0002, 0x0001), *b"OB", &[0, 1]));
    meta_body.extend(explicit_element((0x0002, 0x0002), *b"UI", SOP_CLASS_UID));
    meta_body.extend(explicit_element((0x0002, 0x0003), *b"UI", SOP_INSTANCE_UID));
    let mut syntax = transfer_syntax.as_bytes().to_vec();
    if !syntax.len().is_multiple_of(2) {
        syntax.push(0);
    }
    meta_body.extend(explicit_element((0x0002, 0x0010), *b"UI", &syntax));
    meta_body.extend(explicit_element(
        (0x0002, 0x0012),
        *b"UI",
        IMPLEMENTATION_UID,
    ));

    let mut bytes = vec![0; 128];
    bytes.extend_from_slice(b"DICM");
    bytes.extend(explicit_element(
        (0x0002, 0x0000),
        *b"UL",
        &fixture_u32(meta_body.len()).to_le_bytes(),
    ));
    bytes.extend(meta_body);
    bytes.extend_from_slice(data_set);
    bytes
}

fn corpus_transfer_syntaxes() -> BTreeSet<&'static str> {
    include_str!("../../../corpus/manifest.tsv")
        .lines()
        .skip(1)
        .filter_map(|row| row.split('\t').nth(2))
        .collect()
}

fn multipart(boundary: &str, parts: &[(&str, &[u8])]) -> Vec<u8> {
    let mut body = Vec::new();
    for (index, (media_type, payload)) in parts.iter().enumerate() {
        body.extend_from_slice(b"--");
        body.extend_from_slice(boundary.as_bytes());
        body.extend_from_slice(b"\r\nContent-Type: ");
        body.extend_from_slice(media_type.as_bytes());
        body.extend_from_slice(b"\r\nContent-Location: /synthetic/");
        body.extend_from_slice(index.to_string().as_bytes());
        body.extend_from_slice(b"\r\nContent-Length: ");
        body.extend_from_slice(payload.len().to_string().as_bytes());
        body.extend_from_slice(b"\r\n\r\n");
        body.extend_from_slice(payload);
        body.extend_from_slice(b"\r\n");
    }
    body.extend_from_slice(b"--");
    body.extend_from_slice(boundary.as_bytes());
    body.extend_from_slice(b"--");
    body
}

fn source(kind: SourceResponseKind, bytes: Vec<u8>) -> Result<SourceBatch, SourceError> {
    DicomwebSource.consume(SourceResponse::new(kind, bytes))
}

#[test]
fn qido_json_preserves_annex_f_metadata_semantics() {
    // PS3.18 F.2.2 to F.2.7: one top-level array, uppercase tags, explicit VR,
    // ordered Value arrays, carrier objects, absent tags, and carrier-free
    // present empty attributes.
    let json = br#"[{
        "00080005":{"vr":"CS"},
        "00080001":{"vr":"AT","Value":["00080016",null,"00080018"]},
        "00080008":{"vr":"CS","Value":["ORIGINAL",null,"PRIMARY"]},
        "00080060":{"vr":"CS","Value":["CT","MR"]},
        "00081115":{"vr":"SQ","Value":[{"00081150":{"vr":"UI","Value":["2.25.3"]}},{}]},
        "00100010":{"vr":"PN","Value":[{"Alphabetic":"SYNTHETIC^ALPHA","Ideographic":"SYNTHETIC=IDEOGRAPHIC","Phonetic":"SYNTHETIC=PHONETIC"},null]},
        "00111001":{"vr":"US","Value":[1,null,2]},
        "00111002":{"vr":"SV","Value":["-9223372036854775808"]},
        "00111003":{"vr":"UV","Value":["18446744073709551615"]},
        "00111004":{"vr":"US","Value":[null]},
        "00280103":{"vr":"SS","Value":[-1]},
        "00281052":{"vr":"DS","Value":["-1024","1.5"]},
        "7FE00008":{"vr":"OF","InlineBinary":"AAE="},
        "7FE00010":{"vr":"OB","BulkDataURI":"https://example.test/bulk/synthetic"}
    }]"#;

    let result = source(SourceResponseKind::QidoJson, json.to_vec());
    assert!(result.is_ok(), "the Annex F fixture should parse");
    assert!(matches!(&result, Ok(SourceBatch::Query(_))));
    let Ok(SourceBatch::Query(data_sets)) = result else {
        return;
    };
    assert_eq!(data_sets.len(), 1);
    let Some(data_set) = data_sets.first() else {
        return;
    };
    assert!(data_set.get(Tag(0x0008, 0x0018)).is_none());
    assert_eq!(
        data_set
            .get(Tag(0x0008, 0x0005))
            .map(|element| element.value()),
        Some(&MetadataValue::Empty)
    );
    assert!(matches!(
        data_set
            .get(Tag(0x0008, 0x0008))
            .map(|element| element.value()),
        Some(MetadataValue::WithNullSlots(_))
    ));
    assert_eq!(
        data_set
            .get(Tag(0x0008, 0x0060))
            .map(|element| element.value()),
        Some(&MetadataValue::Text(vec!["CT".to_owned(), "MR".to_owned()]))
    );
    assert_eq!(
        data_set
            .get(Tag(0x0028, 0x0103))
            .map(|element| element.value()),
        Some(&MetadataValue::Signed16(vec![-1]))
    );
    let nullable_text =
        data_set
            .get(Tag(0x0008, 0x0008))
            .and_then(|element| match element.value() {
                MetadataValue::WithNullSlots(slots) => Some(slots),
                _ => None,
            });
    assert_eq!(nullable_text.map(|slots| slots.value_count()), Some(3));
    assert_eq!(
        nullable_text.map(|slots| slots.empty_slots()),
        Some(&[1][..])
    );
    assert_eq!(
        nullable_text.map(|slots| slots.present_values()),
        Some(&MetadataValue::Text(vec![
            "ORIGINAL".to_owned(),
            "PRIMARY".to_owned()
        ]))
    );
    assert_eq!(
        nullable_text.and_then(|slots| slots.present_index(2)),
        Some(1)
    );

    let nullable_us = data_set
        .get(Tag(0x0011, 0x1001))
        .and_then(|element| match element.value() {
            MetadataValue::WithNullSlots(slots) => Some(slots),
            _ => None,
        });
    assert_eq!(nullable_us.map(|slots| slots.value_count()), Some(3));
    assert_eq!(nullable_us.map(|slots| slots.empty_slots()), Some(&[1][..]));
    assert_eq!(
        nullable_us.map(|slots| slots.present_values()),
        Some(&MetadataValue::Unsigned16(vec![1, 2]))
    );
    assert_eq!(
        data_set
            .get(Tag(0x0011, 0x1002))
            .map(|element| element.value()),
        Some(&MetadataValue::Signed64(vec![i64::MIN]))
    );
    assert_eq!(
        data_set
            .get(Tag(0x0011, 0x1003))
            .map(|element| element.value()),
        Some(&MetadataValue::Unsigned64(vec![u64::MAX]))
    );
    let all_null = data_set
        .get(Tag(0x0011, 0x1004))
        .and_then(|element| match element.value() {
            MetadataValue::WithNullSlots(slots) => Some(slots),
            _ => None,
        });
    assert_eq!(all_null.map(|slots| slots.value_count()), Some(1));
    assert_eq!(all_null.map(|slots| slots.empty_slots()), Some(&[0][..]));
    assert_eq!(
        all_null.map(|slots| slots.present_values()),
        Some(&MetadataValue::Unsigned16(Vec::new()))
    );
    assert_eq!(
        data_set
            .get(Tag(0x0028, 0x1052))
            .map(|element| element.value()),
        Some(&MetadataValue::Text(vec![
            "-1024".to_owned(),
            "1.5".to_owned()
        ]))
    );

    let names = data_set
        .get(Tag(0x0010, 0x0010))
        .map(|element| element.value())
        .and_then(|value| match value {
            MetadataValue::WithNullSlots(slots) => match slots.present_values() {
                MetadataValue::PersonNames(names) => Some((slots, names)),
                _ => None,
            },
            _ => None,
        });
    assert!(names.is_some(), "PN remains a person-name carrier");
    assert_eq!(names.map(|(slots, _)| slots.value_count()), Some(2));
    assert_eq!(names.map(|(slots, _)| slots.empty_slots()), Some(&[1][..]));
    assert_eq!(
        names
            .and_then(|(_, values)| values.first())
            .and_then(|name| name.alphabetic()),
        Some("SYNTHETIC^ALPHA")
    );
    assert_eq!(
        names
            .and_then(|(_, values)| values.first())
            .and_then(|name| name.ideographic()),
        Some("SYNTHETIC=IDEOGRAPHIC")
    );
    assert_eq!(
        names
            .and_then(|(_, values)| values.first())
            .and_then(|name| name.phonetic()),
        Some("SYNTHETIC=PHONETIC")
    );

    let nullable_tags =
        data_set
            .get(Tag(0x0008, 0x0001))
            .and_then(|element| match element.value() {
                MetadataValue::WithNullSlots(slots) => Some(slots),
                _ => None,
            });
    assert_eq!(nullable_tags.map(|slots| slots.value_count()), Some(3));
    assert_eq!(
        nullable_tags.map(|slots| slots.empty_slots()),
        Some(&[1][..])
    );
    assert_eq!(
        nullable_tags.map(|slots| slots.present_values()),
        Some(&MetadataValue::Tags(vec![
            Tag(0x0008, 0x0016),
            Tag(0x0008, 0x0018)
        ]))
    );

    let items = data_set
        .get(Tag(0x0008, 0x1115))
        .map(|element| element.value())
        .and_then(|value| match value {
            MetadataValue::Sequence(items) => Some(items),
            _ => None,
        });
    assert!(items.is_some(), "SQ remains an ordered sequence carrier");
    let Some(items) = items else {
        return;
    };
    assert_eq!(items.len(), 2);
    assert_eq!(items.first().map(MetadataSet::len), Some(1));
    assert_eq!(items.get(1).map(MetadataSet::is_empty), Some(true));

    let uri = data_set
        .get(Tag(0x7fe0, 0x0010))
        .map(|element| element.value())
        .and_then(|value| match value {
            MetadataValue::BulkDataUri(uri) => Some(uri),
            _ => None,
        });
    assert_eq!(
        uri.map(|value| value.as_str()),
        Some("https://example.test/bulk/synthetic")
    );
    let encoded = data_set
        .get(Tag(0x7fe0, 0x0008))
        .map(|element| element.value())
        .and_then(|value| match value {
            MetadataValue::InlineBinary(encoded) => Some(encoded),
            _ => None,
        });
    assert_eq!(encoded.map(|value| value.as_str()), Some("AAE="));
}

#[test]
fn qido_json_refuses_duplicate_attributes_before_map_collapse() {
    // PS3.18 F.2.2 and F.3.1: a DICOM JSON object has one Attribute object
    // per Tag key and cannot represent duplicate Tag values.
    let json =
        br#"[{"00080060":{"vr":"CS","Value":["CT"]},"00080060":{"vr":"CS","Value":["MR"]}}]"#;
    assert_eq!(
        source(SourceResponseKind::QidoJson, json.to_vec()).err(),
        Some(SourceError::InvalidJson)
    );
}

#[test]
fn qido_json_refuses_duplicate_attributes_inside_sequence_items() {
    // PS3.18 F.2.2 applies recursively because each SQ item is a DICOM JSON
    // Model object representing one nested data set.
    let json = br#"[{"00081115":{"vr":"SQ","Value":[{"00081150":{"vr":"UI","Value":["2.25.1"]},"00081150":{"vr":"UI","Value":["2.25.2"]}}]}}]"#;
    assert_eq!(
        source(SourceResponseKind::QidoJson, json.to_vec()).err(),
        Some(SourceError::InvalidJson)
    );
}

#[test]
fn qido_json_refuses_invalid_structure_and_carriers() {
    let cases: &[(&[u8], SourceError)] = &[
        (br#"{}"#, SourceError::InvalidJsonRoot),
        (
            br#"[{"00080060":{"vr":"ZZ","Value":["CT"]}}]"#,
            SourceError::InvalidJsonVr { attribute: 0 },
        ),
        (
            br#"[{"0008006a":{"vr":"CS","Value":["CT"]}}]"#,
            SourceError::InvalidJsonTag { attribute: 0 },
        ),
        (
            br#"[{"00080060":{"vr":"CS","Value":["CT"],"BulkDataURI":"https://example.test/bulk"}}]"#,
            SourceError::InvalidJsonCarrier { attribute: 0 },
        ),
        (
            br#"[{"7FE00010":{"vr":"OB","InlineBinary":"AAF="}}]"#,
            SourceError::InvalidJsonCarrier { attribute: 0 },
        ),
        (
            br#"[{"7FE00010":{"vr":"OB","Value":[1]}}]"#,
            SourceError::InvalidJsonValue { attribute: 0 },
        ),
        (
            br#"[{"00080000":{"vr":"UL","Value":[1]}}]"#,
            SourceError::InvalidJsonTag { attribute: 0 },
        ),
        (
            br#"[{"00081115":{"vr":"SQ","Value":[null]}}]"#,
            SourceError::InvalidJsonValue { attribute: 0 },
        ),
    ];
    for (json, expected) in cases {
        assert_eq!(
            source(SourceResponseKind::QidoJson, json.to_vec()).err(),
            Some(*expected)
        );
    }
}

#[test]
fn wado_rs_instance_multipart_parses_one_and_many_part10_objects() {
    let first = part10(2);
    let second = part10(3);
    for expected in [
        vec![first.as_slice()],
        vec![first.as_slice(), second.as_slice()],
    ] {
        let body = multipart(
            "synthetic-boundary",
            &expected
                .iter()
                .map(|bytes| ("application/dicom", *bytes))
                .collect::<Vec<_>>(),
        );
        let kind = SourceResponseKind::WadoRsInstances {
            boundary: "synthetic-boundary".to_owned(),
        };
        let result = source(kind, body);
        assert!(result.is_ok(), "the multipart fixture should parse");
        assert!(matches!(&result, Ok(SourceBatch::Instances(_))));
        let Ok(SourceBatch::Instances(instances)) = result else {
            return;
        };
        assert_eq!(instances.len(), expected.len());
        for instance in instances {
            assert_eq!(instance.transfer_syntax().uid(), EXPLICIT_VR_LE);
        }
    }
}

#[test]
fn wado_uri_parses_one_part10_object_without_a_multipart_wrapper() {
    let result = source(SourceResponseKind::WadoUriPart10, part10(7));
    assert!(result.is_ok(), "the WADO-URI fixture should parse");
    assert!(matches!(&result, Ok(SourceBatch::Instances(_))));
    let Ok(SourceBatch::Instances(instances)) = result else {
        return;
    };
    assert_eq!(instances.len(), 1);
    assert_eq!(
        instances
            .first()
            .map(|instance| instance.transfer_syntax().uid()),
        Some(EXPLICIT_VR_LE)
    );
}

#[test]
fn selected_codec_catalogue_matches_corpus_and_f016_dispatch() {
    let selected: BTreeSet<_> = KNOWN_TRANSFER_SYNTAXES.iter().copied().collect();
    assert_eq!(selected.len(), 16);
    assert_eq!(selected, corpus_transfer_syntaxes());

    for uid in KNOWN_TRANSFER_SYNTAXES {
        // PS3.5 A.5 carries the data set as a raw Deflate stream. 03 00 is the
        // complete empty fixed-Huffman block. Other dispatch paths accept an
        // empty data set and append their own completion sentinel internally.
        let data_set: &[u8] = if *uid == "1.2.840.10008.1.2.1.99" {
            &[0x03, 0x00]
        } else {
            &[]
        };
        let parsed = parse_part10(&part10_with_syntax(uid, data_set));
        assert!(
            parsed.is_ok(),
            "F-016 dispatch must accept selected Transfer Syntax UID {uid}: {parsed:?}"
        );
        assert_eq!(
            parsed.map(|instance| instance.transfer_syntax().uid()),
            Ok(*uid)
        );
    }
}

#[test]
fn frame_multipart_returns_ordered_ranges_without_splitting_payload_lookalikes() {
    // PS3.18 8.6.1.2.1: only CRLF + DASH + the complete boundary + a legal
    // delimiter suffix separates parts. The first payload contains a prefix
    // lookalike and must remain one byte range.
    let first = b"frame-a\r\n--synthetic-boundaryX\r\nstill-a";
    let second = b"frame-b";
    let body = multipart(
        "synthetic-boundary",
        &[
            ("application/octet-stream", first.as_slice()),
            ("application/octet-stream", second.as_slice()),
        ],
    );
    let kind = SourceResponseKind::WadoRsFrames {
        boundary: "synthetic-boundary".to_owned(),
        media_type: "application/octet-stream".to_owned(),
    };
    let result = source(kind, body);
    assert!(result.is_ok(), "the frame multipart fixture should parse");
    assert!(matches!(&result, Ok(SourceBatch::Frames(_))));
    let Ok(SourceBatch::Frames(frames)) = result else {
        return;
    };
    assert_eq!(frames.parts().len(), 2);
    assert_eq!(frames.part_bytes(0), Some(first.as_slice()));
    assert_eq!(frames.part_bytes(1), Some(second.as_slice()));
    assert_eq!(
        frames.parts().first().map(|part| part.media_type()),
        Some("application/octet-stream")
    );
    assert!(
        frames
            .parts()
            .first()
            .is_some_and(|part| part.range().end <= frames.bytes().len())
    );
}

#[test]
fn frame_parts_retain_parameterized_media_type_evidence() {
    const FRAME_TRANSFER_SYNTAX: &str = "1.2.840.10008.1.2.4.80";
    let media_type = format!("image/jls; transfer-syntax={FRAME_TRANSFER_SYNTAX}");
    let body = multipart("frame-edge", &[(media_type.as_str(), b"encoded")]);
    let result = source(
        SourceResponseKind::WadoRsFrames {
            boundary: "frame-edge".to_owned(),
            media_type: media_type.clone(),
        },
        body,
    );
    assert!(result.is_ok());
    assert!(matches!(&result, Ok(SourceBatch::Frames(_))));
    let Ok(SourceBatch::Frames(frames)) = result else {
        return;
    };
    assert_eq!(
        frames.parts().first().map(|part| part.media_type()),
        Some(media_type.as_str())
    );
    assert_eq!(
        frames
            .parts()
            .first()
            .and_then(|part| part.transfer_syntax()),
        Some(FRAME_TRANSFER_SYNTAX)
    );
    assert_eq!(
        Registry::new().capability(FRAME_TRANSFER_SYNTAX),
        Capability::KnownUnavailable
    );
}

#[test]
fn multipart_refuses_invalid_boundaries_framing_and_media_types() {
    let valid = multipart("edge", &[("application/dicom", part10(1).as_slice())]);
    assert_eq!(
        source(
            SourceResponseKind::WadoRsInstances {
                boundary: "bad boundary ".to_owned(),
            },
            valid.clone(),
        )
        .err(),
        Some(SourceError::InvalidMultipartBoundary)
    );
    assert_eq!(
        source(
            SourceResponseKind::WadoRsInstances {
                boundary: "edge".to_owned(),
            },
            b"--edge\nContent-Type: application/dicom\n\nbytes--edge--".to_vec(),
        )
        .err(),
        Some(SourceError::InvalidMultipart)
    );
    let wrong = multipart("edge", &[("application/octet-stream", b"bytes")]);
    assert_eq!(
        source(
            SourceResponseKind::WadoRsInstances {
                boundary: "edge".to_owned(),
            },
            wrong,
        )
        .err(),
        Some(SourceError::InvalidPartMediaType { part: 0 })
    );
}

#[test]
fn multipart_requires_resource_headers_and_matching_content_length() {
    let missing_location =
        b"--edge\r\nContent-Type: application/dicom\r\nContent-Length: 5\r\n\r\nbytes\r\n--edge--"
            .to_vec();
    let wrong_length = b"--edge\r\nContent-Type: application/dicom\r\nContent-Location: /synthetic/0\r\nContent-Length: 4\r\n\r\nbytes\r\n--edge--".to_vec();
    let signed_length = b"--edge\r\nContent-Type: application/dicom\r\nContent-Location: /synthetic/0\r\nContent-Length: +5\r\n\r\nbytes\r\n--edge--".to_vec();
    let conflicting_transfer = b"--edge\r\nContent-Type: application/dicom\r\nContent-Location: /synthetic/0\r\nContent-Length: 5\r\nTransfer-Encoding: identity\r\n\r\nbytes\r\n--edge--".to_vec();
    for body in [
        missing_location,
        wrong_length,
        signed_length,
        conflicting_transfer,
    ] {
        assert_eq!(
            source(
                SourceResponseKind::WadoRsInstances {
                    boundary: "edge".to_owned(),
                },
                body,
            )
            .err(),
            Some(SourceError::InvalidMultipart)
        );
    }

    let transfer_encoded = b"--edge\r\nContent-Type: application/octet-stream\r\nContent-Location: /synthetic/0\r\nTransfer-Encoding: identity\r\n\r\nbytes\r\n--edge--".to_vec();
    assert!(matches!(
        source(
            SourceResponseKind::WadoRsFrames {
                boundary: "edge".to_owned(),
                media_type: "application/octet-stream".to_owned(),
            },
            transfer_encoded,
        ),
        Ok(SourceBatch::Frames(_))
    ));
}

#[test]
fn multipart_requires_exact_header_names_and_uri_reference_locations() {
    let whitespace_before_colon = b"--edge\r\nContent-Type : application/octet-stream\r\nContent-Location: /synthetic/0\r\nContent-Length: 5\r\n\r\nbytes\r\n--edge--".to_vec();
    let invalid_space = b"--edge\r\nContent-Type: application/octet-stream\r\nContent-Location: not a URI\r\nContent-Length: 5\r\n\r\nbytes\r\n--edge--".to_vec();
    let invalid_percent = b"--edge\r\nContent-Type: application/octet-stream\r\nContent-Location: /synthetic/%ZZ\r\nContent-Length: 5\r\n\r\nbytes\r\n--edge--".to_vec();
    for body in [whitespace_before_colon, invalid_space, invalid_percent] {
        assert_eq!(
            source(
                SourceResponseKind::WadoRsFrames {
                    boundary: "edge".to_owned(),
                    media_type: "application/octet-stream".to_owned(),
                },
                body,
            )
            .err(),
            Some(SourceError::InvalidMultipart)
        );
    }

    let relative = b"--edge\r\nContent-Type: application/octet-stream\r\nContent-Location: ../frames/1?synthetic=true\r\nContent-Length: 5\r\n\r\nbytes\r\n--edge--".to_vec();
    assert!(matches!(
        source(
            SourceResponseKind::WadoRsFrames {
                boundary: "edge".to_owned(),
                media_type: "application/octet-stream".to_owned(),
            },
            relative,
        ),
        Ok(SourceBatch::Frames(_))
    ));
}

#[test]
fn source_errors_never_retain_response_values_or_parser_text() {
    let marker = "SYNTHETIC_SECRET_MARKER";
    let malformed = format!(r#"[{{"00080060":{{"vr":"CS","Value":{{"marker":"{marker}"}}}}}}]"#);
    let error = source(SourceResponseKind::QidoJson, malformed.into_bytes())
        .err()
        .unwrap_or(SourceError::InvalidJson);
    assert!(!format!("{error}").contains(marker));
    assert!(!format!("{error:?}").contains(marker));

    let duplicate = format!(
        r#"[{{"00080060":{{"vr":"CS","Value":["{marker}"]}},"00080060":{{"vr":"CS","Value":["MR"]}}}}]"#
    );
    let duplicate_error = source(SourceResponseKind::QidoJson, duplicate.into_bytes())
        .err()
        .unwrap_or(SourceError::InvalidJsonRoot);
    assert_eq!(duplicate_error, SourceError::InvalidJson);
    assert!(!format!("{duplicate_error}").contains(marker));
    assert!(!format!("{duplicate_error:?}").contains(marker));

    let parse_error = source(
        SourceResponseKind::WadoUriPart10,
        marker.as_bytes().to_vec(),
    )
    .err()
    .unwrap_or(SourceError::InvalidJson);
    assert!(!format!("{parse_error}").contains(marker));
    assert!(!format!("{parse_error:?}").contains(marker));
}

#[test]
fn dicomweb_source_satisfies_the_transport_neutral_series_source_contract() {
    fn consume(
        source: &impl SeriesSource,
        response: SourceResponse,
    ) -> Result<SourceBatch, SourceError> {
        source.consume(response)
    }

    let result = consume(
        &DicomwebSource,
        SourceResponse::new(SourceResponseKind::QidoJson, b"[]".to_vec()),
    );
    assert!(matches!(result, Ok(SourceBatch::Query(data_sets)) if data_sets.is_empty()));
}
