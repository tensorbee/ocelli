//! Hand-computed encapsulated frame boundary fixtures for F-019.
//!
//! DICOM PS3.5 Annex A.4 measures Basic Offset Table entries from the first
//! byte of the first Fragment Item Tag. Each Fragment contributes its 8-byte
//! Item Tag and Length header plus its even Value length. PS3.3 C.7.6.3.1.8
//! gives the Extended Offset Table the same origin and restricts each indexed
//! frame to one Fragment.

use ocelli_dicom::{EncapsulatedFrameIndex, FrameDelimiter, FrameIndexError, parse_part10};

const FIRST: &[u8] = &[1, 2, 3, 4];
const SECOND: &[u8] = &[5, 6, 7, 8, 9, 10];
const THIRD: &[u8] = &[11, 12, 13, 14, 15, 16, 17, 18];

fn fragments() -> [&'static [u8]; 3] {
    [FIRST, SECOND, THIRD]
}

#[test]
fn basic_offsets_select_one_frame_spanning_two_fragments() {
    // Fragment Item Tag offsets are 0, 8 + 4 = 12, and
    // 12 + 8 + 6 = 26 bytes. Frame two starts at the third Item Tag.
    let fragments = fragments();
    let index =
        EncapsulatedFrameIndex::from_basic(2, &[0, 26], &fragments, FrameDelimiter::Unmarked);
    assert!(index.is_ok());
    let index = index.unwrap_or_else(|_| unreachable!());
    assert_eq!(index.frame_fragments(0), Ok(&fragments[..2]));
    assert_eq!(index.frame_fragments(1), Ok(&fragments[2..]));
}

#[test]
fn empty_basic_table_uses_only_provable_boundaries() {
    let fragments = fragments();
    let single = EncapsulatedFrameIndex::from_basic(1, &[], &fragments, FrameDelimiter::Unmarked)
        .unwrap_or_else(|_| unreachable!());
    assert_eq!(single.frame_fragments(0), Ok(&fragments[..]));

    let one_per_frame =
        EncapsulatedFrameIndex::from_basic(3, &[], &fragments, FrameDelimiter::Unmarked)
            .unwrap_or_else(|_| unreachable!());
    assert_eq!(one_per_frame.frame_fragments(1), Ok(&fragments[1..2]));

    assert_eq!(
        EncapsulatedFrameIndex::from_basic(2, &[], &fragments, FrameDelimiter::Unmarked).err(),
        Some(FrameIndexError::BoundaryUnavailable {
            declared: 2,
            fragments: 3,
        })
    );
}

#[test]
fn extended_offsets_and_lengths_select_exact_unpadded_frame_bytes() {
    // Each frame is one Fragment under PS3.3 C.7.6.3.1.8. The third encoded
    // frame is seven bytes plus one trailing Item pad byte, so its table length
    // is 7 while the Fragment Value remains even-length.
    let fragments = fragments();
    let index = EncapsulatedFrameIndex::from_extended(3, &[0, 12, 26], &[4, 6, 7], &fragments);
    assert!(index.is_ok());
    let index = index.unwrap_or_else(|_| unreachable!());
    assert_eq!(index.frame_fragments(0), Ok(&[FIRST][..]));
    assert_eq!(index.frame_fragments(1), Ok(&[SECOND][..]));
    let Some(third_without_pad) = THIRD.get(..7) else {
        unreachable!();
    };
    assert_eq!(
        index.frame_fragments(2),
        Ok(core::slice::from_ref(&third_without_pad))
    );
}

#[test]
fn offset_tables_refuse_truncation_non_monotonicity_and_non_boundaries() {
    let fragments = fragments();
    assert_eq!(
        EncapsulatedFrameIndex::from_basic(2, &[12, 26], &fragments, FrameDelimiter::Unmarked)
            .err(),
        Some(FrameIndexError::FirstOffsetNotZero(12))
    );
    assert_eq!(
        EncapsulatedFrameIndex::from_basic(2, &[0], &fragments, FrameDelimiter::Unmarked).err(),
        Some(FrameIndexError::OffsetCount {
            declared: 2,
            actual: 1,
        })
    );
    assert_eq!(
        EncapsulatedFrameIndex::from_basic(2, &[0, 0], &fragments, FrameDelimiter::Unmarked).err(),
        Some(FrameIndexError::NonMonotonicOffsets)
    );
    assert_eq!(
        EncapsulatedFrameIndex::from_basic(2, &[0, 25], &fragments, FrameDelimiter::Unmarked).err(),
        Some(FrameIndexError::OffsetNotFragmentBoundary(25))
    );
    assert_eq!(
        EncapsulatedFrameIndex::from_basic(2, &[0, 40], &fragments, FrameDelimiter::Unmarked).err(),
        Some(FrameIndexError::OffsetNotFragmentBoundary(40))
    );
    assert_eq!(
        EncapsulatedFrameIndex::from_extended(3, &[0, 12], &[4, 6, 7], &fragments).err(),
        Some(FrameIndexError::OffsetCount {
            declared: 3,
            actual: 2,
        })
    );
    assert_eq!(
        EncapsulatedFrameIndex::from_extended(3, &[0, 12, 28], &[4, 6, 7], &fragments).err(),
        Some(FrameIndexError::OffsetNotFragmentBoundary(28))
    );
}

#[test]
fn extended_lengths_and_final_bound_are_checked() {
    let fragments = fragments();
    assert_eq!(
        EncapsulatedFrameIndex::from_extended(3, &[0, 12, 26], &[4, 6], &fragments).err(),
        Some(FrameIndexError::LengthCount {
            declared: 3,
            actual: 2,
        })
    );
    assert_eq!(
        EncapsulatedFrameIndex::from_extended(3, &[0, 12, 26], &[4, 6, 9], &fragments).err(),
        Some(FrameIndexError::InvalidExtendedLength {
            frame: 2,
            declared: 9,
            fragment: 8,
        })
    );
    assert_eq!(
        EncapsulatedFrameIndex::from_extended(3, &[0, 12, 26], &[4, 6, 6], &fragments).err(),
        Some(FrameIndexError::InvalidExtendedLength {
            frame: 2,
            declared: 6,
            fragment: 8,
        })
    );
    assert_eq!(
        EncapsulatedFrameIndex::from_extended(2, &[0, 12], &[4, 6], &fragments).err(),
        Some(FrameIndexError::ExtendedFrameFragmentCount {
            declared: 2,
            fragments: 3,
        })
    );
}

#[test]
fn zero_frames_and_missing_fragments_are_refused() {
    let fragments = fragments();
    assert_eq!(
        EncapsulatedFrameIndex::from_basic(0, &[], &fragments, FrameDelimiter::Unmarked).err(),
        Some(FrameIndexError::ZeroFrameCount)
    );
    assert_eq!(
        EncapsulatedFrameIndex::from_basic(1, &[], &[], FrameDelimiter::Unmarked).err(),
        Some(FrameIndexError::NoFragments)
    );
}

#[test]
fn odd_physical_fragment_values_are_refused_on_both_table_paths() {
    let odd_fragment: &[u8] = &[1, 2, 3];
    let fragments = [odd_fragment];
    assert_eq!(
        EncapsulatedFrameIndex::from_basic(1, &[], &fragments, FrameDelimiter::Unmarked).err(),
        Some(FrameIndexError::OddFragmentLength {
            fragment: 0,
            length: 3,
        })
    );
    assert_eq!(
        EncapsulatedFrameIndex::from_extended(1, &[0], &[3], &fragments).err(),
        Some(FrameIndexError::OddFragmentLength {
            fragment: 0,
            length: 3,
        })
    );
}

#[test]
fn odd_extended_length_requires_exactly_one_physical_pad_byte() {
    let physical_fragment: &[u8] = &[1, 2, 3, 0];
    let fragments = [physical_fragment];
    let index = EncapsulatedFrameIndex::from_extended(1, &[0], &[3], &fragments)
        .unwrap_or_else(|_| unreachable!());
    let Some(encoded_without_pad) = physical_fragment.get(..3) else {
        unreachable!();
    };
    assert_eq!(
        index.frame_fragments(0),
        Ok(core::slice::from_ref(&encoded_without_pad))
    );

    let too_long: &[u8] = &[1, 2, 3, 4, 5, 0, 0, 0];
    assert_eq!(
        EncapsulatedFrameIndex::from_extended(1, &[0], &[5], &[too_long]).err(),
        Some(FrameIndexError::InvalidExtendedLength {
            frame: 0,
            declared: 5,
            fragment: 8,
        })
    );
}

#[test]
fn out_of_range_frame_identifies_the_declared_count() {
    let fragments = fragments();
    let index =
        EncapsulatedFrameIndex::from_basic(2, &[0, 26], &fragments, FrameDelimiter::Unmarked)
            .unwrap_or_else(|_| unreachable!());
    assert_eq!(
        index.frame_fragments(2),
        Err(FrameIndexError::FrameOutOfRange {
            requested: 2,
            declared: 2,
        })
    );
}

// F-244. PS3.5 8.2: a frame "may span multiple Fragments" and support for it
// is required. A.4.1 (JPEG), A.4.3 (JPEG-LS) and A.4.4 (JPEG 2000) permit the
// span. With an empty Basic Offset Table nothing in the Items says where a
// frame ends, so the plan's rule reads the codestream markers: a later frame
// must begin with its start marker, every frame must end with its end marker
// or that marker and the one pad byte A.4 allows, and the count must come out
// exactly. ISO/IEC 10918-1 SOI is `FF D8` and EOI `FF D9`, and the marker
// after SOI begins `FF`. ISO/IEC 15444-1 SOC is `FF 4F`, SIZ `FF 51`, and
// EOC `FF D9`.

const JPEG_A: &[u8] = &[0xFF, 0xD8, 0xFF, 0xE0, 0x11, 0x22];
const JPEG_B: &[u8] = &[0x33, 0x44, 0xFF, 0xD9];
const JPEG_C: &[u8] = &[0xFF, 0xD8, 0xFF, 0xDB, 0x55, 0xFF, 0xD9, 0x00];

#[test]
fn jpeg_frames_spanning_fragments_split_at_their_markers() -> Result<(), String> {
    // Fragment 0 opens with SOI and has no EOI, Fragment 1 ends with EOI,
    // so frame 0 is Fragments 0 and 1. Fragment 2 opens with SOI and ends
    // with EOI then one pad byte, so frame 1 is Fragment 2 with its pad.
    let fragments = [JPEG_A, JPEG_B, JPEG_C];
    let index = EncapsulatedFrameIndex::from_basic(2, &[], &fragments, FrameDelimiter::JpegMarkers);
    let index = index.map_err(|error| format!("{error:?}"))?;
    assert_eq!(index.frame_fragments(0), Ok(&fragments[..2]));
    assert_eq!(index.frame_fragments(1), Ok(&fragments[2..]));
    for wrong in [FrameDelimiter::Jpeg2000Markers, FrameDelimiter::Unmarked] {
        assert_eq!(
            EncapsulatedFrameIndex::from_basic(2, &[], &fragments, wrong).err(),
            Some(FrameIndexError::BoundaryUnavailable {
                declared: 2,
                fragments: 3,
            }),
            "{wrong:?}"
        );
    }
    Ok(())
}

#[test]
fn jpeg2000_frames_spanning_fragments_split_at_their_markers() -> Result<(), String> {
    // SOC SIZ `FF 4F FF 51` opens frames 0 and 1. EOC `FF D9` closes
    // frame 0 at Fragment 1 and frame 1 at Fragment 2, before its pad.
    let fragments: [&[u8]; 3] = [
        &[0xFF, 0x4F, 0xFF, 0x51, 0x00, 0x29],
        &[0xAA, 0xBB, 0xFF, 0xD9],
        &[0xFF, 0x4F, 0xFF, 0x51, 0xCC, 0xFF, 0xD9, 0x00],
    ];
    let index =
        EncapsulatedFrameIndex::from_basic(2, &[], &fragments, FrameDelimiter::Jpeg2000Markers);
    let index = index.map_err(|error| format!("{error:?}"))?;
    assert_eq!(index.frame_fragments(0), Ok(&fragments[..2]));
    assert_eq!(index.frame_fragments(1), Ok(&fragments[2..]));
    assert_eq!(
        EncapsulatedFrameIndex::from_basic(2, &[], &fragments, FrameDelimiter::JpegMarkers).err(),
        Some(FrameIndexError::BoundaryUnavailable {
            declared: 2,
            fragments: 3,
        })
    );
    Ok(())
}

#[test]
fn marker_rule_refuses_unproven_boundaries() {
    let unavailable = |declared, fragments| {
        Some(FrameIndexError::BoundaryUnavailable {
            declared,
            fragments,
        })
    };
    // (i) Only the last Fragment ends a frame, so one frame for two declared.
    let only_last_ends: [&[u8]; 3] = [JPEG_A, &[0x11, 0x22], JPEG_B];
    // (ii) Fragment 2 should open frame 1 and begins `00 00`, not SOI.
    let no_second_start: [&[u8]; 3] = [JPEG_A, JPEG_B, &[0x00, 0x00, 0xFF, 0xD9]];
    // (iii) A false end. Fragment 0 ends `FF D9` but Fragment 1 does not open
    // a frame. An end-marker-only scan takes frame 0 as Fragment 0 and frame 1
    // as Fragments 1 and 2, which hands a decoder bytes that are not a frame.
    let false_end: [&[u8]; 3] = [
        &[0xFF, 0xD8, 0xFF, 0xE0, 0xFF, 0xD9],
        &[0x11, 0x22, 0x33, 0x44],
        &[0xFF, 0xD8, 0xFF, 0xDB, 0xFF, 0xD9],
    ];
    // (iv) The last Fragment does not end a frame.
    let open_last: [&[u8]; 3] = [JPEG_A, JPEG_B, &[0xFF, 0xD8, 0xFF, 0xDB]];
    // (v) Three frames found for two declared.
    let three_found: [&[u8]; 3] = [JPEG_C, JPEG_C, JPEG_C];
    for (case, fragments) in [
        ("only the last ends", only_last_ends),
        ("no second start", no_second_start),
        ("false end", false_end),
        ("open last", open_last),
        ("three found", three_found),
    ] {
        assert_eq!(
            EncapsulatedFrameIndex::from_basic(2, &[], &fragments, FrameDelimiter::JpegMarkers)
                .err(),
            unavailable(2, 3),
            "{case}"
        );
    }
}

#[test]
fn equal_counts_need_no_markers() -> Result<(), String> {
    // PS3.5 8.2 compares the Fragment count with the frame count. Equal
    // counts mean one Fragment per frame, and no marker is read, so these
    // marker-free Fragments still map one to each frame under every
    // delimiter.
    let fragments = fragments();
    for delimiter in [
        FrameDelimiter::Unmarked,
        FrameDelimiter::JpegMarkers,
        FrameDelimiter::Jpeg2000Markers,
    ] {
        let index = EncapsulatedFrameIndex::from_basic(3, &[], &fragments, delimiter);
        let index = index.map_err(|error| format!("{delimiter:?} {error:?}"))?;
        for frame in 0..3 {
            assert_eq!(
                index.frame_fragments(frame).ok(),
                fragments.get(frame..=frame),
                "{delimiter:?}"
            );
        }
    }
    Ok(())
}

#[test]
fn delimiter_follows_ps3_5_a_4() {
    for (uid, expected, section) in [
        (
            "1.2.840.10008.1.2.4.50",
            FrameDelimiter::JpegMarkers,
            "A.4.1",
        ),
        (
            "1.2.840.10008.1.2.4.51",
            FrameDelimiter::JpegMarkers,
            "A.4.1",
        ),
        (
            "1.2.840.10008.1.2.4.57",
            FrameDelimiter::JpegMarkers,
            "A.4.1",
        ),
        (
            "1.2.840.10008.1.2.4.70",
            FrameDelimiter::JpegMarkers,
            "A.4.1",
        ),
        (
            "1.2.840.10008.1.2.4.80",
            FrameDelimiter::JpegMarkers,
            "A.4.3",
        ),
        (
            "1.2.840.10008.1.2.4.81",
            FrameDelimiter::JpegMarkers,
            "A.4.3",
        ),
        (
            "1.2.840.10008.1.2.4.90",
            FrameDelimiter::Jpeg2000Markers,
            "A.4.4",
        ),
        (
            "1.2.840.10008.1.2.4.91",
            FrameDelimiter::Jpeg2000Markers,
            "A.4.4",
        ),
        // A.4.4 forbids an HTJ2K frame spanning Fragments. Its codestream
        // uses the Part 1 markers, so the plan reads it leniently, item 3.
        (
            "1.2.840.10008.1.2.4.201",
            FrameDelimiter::Jpeg2000Markers,
            "A.4.4",
        ),
        (
            "1.2.840.10008.1.2.4.202",
            FrameDelimiter::Jpeg2000Markers,
            "A.4.4",
        ),
        (
            "1.2.840.10008.1.2.4.203",
            FrameDelimiter::Jpeg2000Markers,
            "A.4.4",
        ),
        // A.4.2, one frame in one Fragment, and no in-band end marker.
        ("1.2.840.10008.1.2.5", FrameDelimiter::Unmarked, "A.4.2"),
        // A.4.12, one frame in one Fragment, and no end marker.
        (
            "1.2.840.10008.1.2.4.110",
            FrameDelimiter::Unmarked,
            "A.4.12",
        ),
        (
            "1.2.840.10008.1.2.4.111",
            FrameDelimiter::Unmarked,
            "A.4.12",
        ),
        (
            "1.2.840.10008.1.2.4.112",
            FrameDelimiter::Unmarked,
            "A.4.12",
        ),
        ("1.2.840.10008.1.2.1", FrameDelimiter::Unmarked, "native"),
        ("2.25.1", FrameDelimiter::Unmarked, "unknown"),
    ] {
        assert_eq!(
            FrameDelimiter::for_transfer_syntax(uid),
            expected,
            "{uid} {section}"
        );
    }
}

// PS3.10 7.1 Part 10 envelope and PS3.5 A.2 Explicit VR Little Endian
// elements, written by hand so the Extended Offset Table test reads a real
// parsed object.

fn element(tag: (u16, u16), vr: [u8; 2], value: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&tag.0.to_le_bytes());
    bytes.extend_from_slice(&tag.1.to_le_bytes());
    bytes.extend_from_slice(&vr);
    if matches!(&vr, b"OB" | b"OV") {
        bytes.extend_from_slice(&[0, 0]);
        bytes.extend_from_slice(&length(value.len()).to_le_bytes());
    } else {
        let short = u16::try_from(value.len()).unwrap_or(u16::MAX);
        bytes.extend_from_slice(&short.to_le_bytes());
    }
    bytes.extend_from_slice(value);
    bytes
}

fn length(value: usize) -> u32 {
    u32::try_from(value).unwrap_or(u32::MAX)
}

fn item(element: u16, value: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&0xFFFE_u16.to_le_bytes());
    bytes.extend_from_slice(&element.to_le_bytes());
    bytes.extend_from_slice(&length(value.len()).to_le_bytes());
    bytes.extend_from_slice(value);
    bytes
}

fn u64s(values: &[u64]) -> Vec<u8> {
    values
        .iter()
        .flat_map(|value| value.to_le_bytes())
        .collect()
}

const JPEG_BASELINE: &str = "1.2.840.10008.1.2.4.50\0";

fn part10(
    frame_count: &[u8],
    syntax: &str,
    basic: &[u8],
    extended: Option<&[u64]>,
    lengths: Option<&[u64]>,
    fragments: &[&[u8]],
) -> Vec<u8> {
    let mut meta = Vec::new();
    meta.extend(element((0x0002, 0x0001), *b"OB", &[0, 1]));
    meta.extend(element(
        (0x0002, 0x0002),
        *b"UI",
        b"1.2.840.10008.5.1.4.1.1.2\0",
    ));
    meta.extend(element((0x0002, 0x0003), *b"UI", b"2.25.1"));
    meta.extend(element((0x0002, 0x0010), *b"UI", syntax.as_bytes()));
    meta.extend(element((0x0002, 0x0012), *b"UI", b"2.25.2"));
    let mut bytes = vec![0; 128];
    bytes.extend_from_slice(b"DICM");
    bytes.extend(element(
        (0x0002, 0x0000),
        *b"UL",
        &length(meta.len()).to_le_bytes(),
    ));
    bytes.extend(meta);
    bytes.extend(element(
        (0x0008, 0x0016),
        *b"UI",
        b"1.2.840.10008.5.1.4.1.1.2\0",
    ));
    bytes.extend(element((0x0028, 0x0008), *b"IS", frame_count));
    if let Some(extended) = extended {
        bytes.extend(element((0x7FE0, 0x0001), *b"OV", &u64s(extended)));
    }
    if let Some(lengths) = lengths {
        bytes.extend(element((0x7FE0, 0x0002), *b"OV", &u64s(lengths)));
    }
    bytes.extend_from_slice(&0x7FE0_u16.to_le_bytes());
    bytes.extend_from_slice(&0x0010_u16.to_le_bytes());
    bytes.extend_from_slice(b"OB");
    bytes.extend_from_slice(&[0, 0]);
    bytes.extend_from_slice(&u32::MAX.to_le_bytes());
    bytes.extend(item(0xE000, basic));
    for fragment in fragments {
        bytes.extend(item(0xE000, fragment));
    }
    bytes.extend(item(0xE0DD, &[]));
    bytes
}

#[test]
fn from_parsed_prefers_the_extended_offset_table() -> Result<(), String> {
    // PS3.3 C.7.6.3.1.8: Extended Offset Table values are byte offsets to
    // each frame's one Fragment Item Tag, measured from the first Item Tag
    // after the empty Basic Offset Table. Fragments of 4, 6 and 2 bytes give
    // 0, 8 + 4 = 12 and 12 + 8 + 6 = 26. Extended Offset Table Lengths are
    // the frames' encoded lengths, and frame 0's 3 is its 4-byte Fragment
    // less one pad byte, so a correct Extended read yields three bytes where
    // the Basic path yields four.
    let fragments: [&[u8]; 3] = [&[1, 2, 3, 0], &[4, 5, 6, 7, 8, 9], &[10, 11]];
    let offsets: &[u64] = &[0, 12, 26];
    let lengths: &[u64] = &[3, 6, 2];
    let open = |basic: &[u8], extended: Option<&[u64]>, lengths: Option<&[u64]>| {
        parse_part10(&part10(
            b"3 ",
            JPEG_BASELINE,
            basic,
            extended,
            lengths,
            &fragments,
        ))
    };

    let parsed = open(&[], Some(offsets), Some(lengths));
    let parsed = parsed.map_err(|error| format!("{error:?}"))?;
    let index = EncapsulatedFrameIndex::from_parsed(&parsed, 3);
    let index = index.map_err(|error| format!("{error:?}"))?;
    assert_eq!(index.frame_fragments(0), Ok(&[&[1_u8, 2, 3][..]][..]));
    assert_eq!(index.frame_fragments(1), Ok(&fragments[1..2]));
    assert_eq!(index.frame_fragments(2), Ok(&fragments[2..3]));

    let refusal = |basic: &[u8], extended: Option<&[u64]>, lengths: Option<&[u64]>| {
        let parsed = open(basic, extended, lengths);
        let parsed = parsed.map_err(|error| format!("{error:?}"))?;
        Ok::<_, String>(EncapsulatedFrameIndex::from_parsed(&parsed, 3).err())
    };
    // An offset off an Item boundary, which the Basic path never reads.
    assert_eq!(
        refusal(&[], Some(&[0, 12, 27]), Some(lengths))?,
        Some(FrameIndexError::OffsetNotFragmentBoundary(27))
    );
    // A length that is neither the Fragment nor the Fragment less one pad.
    assert_eq!(
        refusal(&[], Some(offsets), Some(&[3, 6, 4]))?,
        Some(FrameIndexError::InvalidExtendedLength {
            frame: 2,
            declared: 4,
            fragment: 2,
        })
    );
    // PS3.5 A.4: a Basic Offset Table Item Value "is not permitted" beside an
    // Extended Offset Table. A one-frame-per-Fragment table, 0, 12 and 26.
    let mut basic = Vec::new();
    for offset in [0_u32, 12, 26] {
        basic.extend_from_slice(&offset.to_le_bytes());
    }
    assert_eq!(
        refusal(&basic, Some(offsets), Some(lengths))?,
        Some(FrameIndexError::ExtendedWithBasicOffsets)
    );
    // PS3.3 C.7.6.3: Lengths is Type 1C, required when the table is present.
    assert_eq!(
        refusal(&[], Some(offsets), None)?,
        Some(FrameIndexError::ExtendedOffsetTableUnreadable)
    );

    // No Extended table: the Basic path, equal counts, whole Fragments.
    let parsed = open(&[], None, None);
    let parsed = parsed.map_err(|error| format!("{error:?}"))?;
    let index = EncapsulatedFrameIndex::from_parsed(&parsed, 3);
    let index = index.map_err(|error| format!("{error:?}"))?;
    assert_eq!(index.frame_fragments(0), Ok(&fragments[0..1]));
    Ok(())
}

#[test]
fn split_markers_across_even_fragments() -> Result<(), String> {
    // PS3.5 A.4 permits two-byte Fragment Values, and 8.2 requires
    // fragmented frame support. Codec markers are not Item-aligned.
    let jpeg_start: [&[u8]; 3] = [JPEG_C, &[0xFF, 0xD8], &[0xFF, 0xDB, 0x11, 0x22, 0xFF, 0xD9]];
    let jpeg_end: [&[u8]; 3] = [&[0xFF, 0xD8, 0xFF, 0xDB, 0x11, 0xFF], &[0xD9, 0], JPEG_C];
    let j2k_frame: &[u8] = &[0xFF, 0x4F, 0xFF, 0x51, 0xFF, 0xD9];
    let j2k_start: [&[u8]; 3] = [j2k_frame, &[0xFF, 0x4F], &[0xFF, 0x51, 0xFF, 0xD9]];
    let j2k_end: [&[u8]; 3] = [&[0xFF, 0x4F, 0xFF, 0x51, 0x11, 0xFF], &[0xD9, 0], j2k_frame];
    for (label, delimiter, fragments, second) in [
        ("JPEG SOI", FrameDelimiter::JpegMarkers, jpeg_start, 1),
        ("JPEG EOI", FrameDelimiter::JpegMarkers, jpeg_end, 2),
        ("J2K SOC", FrameDelimiter::Jpeg2000Markers, j2k_start, 1),
        ("J2K EOC", FrameDelimiter::Jpeg2000Markers, j2k_end, 2),
    ] {
        // A populated table selects the same grouping independently of markers.
        let offset = length(
            fragments
                .iter()
                .take(second)
                .map(|fragment| 8 + fragment.len())
                .sum(),
        );
        let control = EncapsulatedFrameIndex::from_basic(2, &[0, offset], &fragments, delimiter)
            .map_err(|error| format!("{label} control: {error:?}"))?;
        let actual = EncapsulatedFrameIndex::from_basic(2, &[], &fragments, delimiter)
            .map_err(|error| format!("{label} empty: {error:?}"))?;
        for frame in 0..2 {
            assert_eq!(
                actual.frame_fragments(frame),
                control.frame_fragments(frame),
                "{label}"
            );
        }
        {
            let syntax = if delimiter == FrameDelimiter::JpegMarkers {
                JPEG_BASELINE
            } else {
                "1.2.840.10008.1.2.4.90\0"
            };
            let bytes = part10(b"2 ", syntax, &[], None, None, &fragments);
            let parsed =
                parse_part10(&bytes).map_err(|error| format!("{label} parse: {error:?}"))?;
            let parsed_index = EncapsulatedFrameIndex::from_parsed(&parsed, 2)
                .map_err(|error| format!("{label} parsed index: {error:?}"))?;
            for frame in 0..2 {
                assert_eq!(
                    parsed_index.frame_fragments(frame),
                    control.frame_fragments(frame),
                    "{label} parsed"
                );
            }
        }
    }
    Ok(())
}

#[test]
fn impossible_frame_count_is_refused_before_reservation() {
    // PS3.5 8.2 requires at least one Fragment per frame, so two Fragments
    // cannot carry three or usize::MAX frames. This must return, not panic.
    let fragments = [JPEG_C, JPEG_C];
    for count in [3, usize::MAX] {
        assert_eq!(
            EncapsulatedFrameIndex::from_basic(count, &[], &fragments, FrameDelimiter::JpegMarkers)
                .err(),
            Some(FrameIndexError::BoundaryUnavailable {
                declared: count,
                fragments: 2
            })
        );
    }
}

#[test]
fn split_end_without_a_valid_next_start_is_refused() {
    // An EOI-like suffix divided across even Items is not sufficient.
    // The next logical frame must also begin with its codec start bytes.
    let fragments: [&[u8]; 4] = [
        &[0xFF, 0xD8, 0xFF, 0xE0, 0x11, 0xFF],
        &[0xD9, 0],
        &[0, 0],
        &[0xFF, 0xDB, 0xFF, 0xD9],
    ];
    assert_eq!(
        EncapsulatedFrameIndex::from_basic(2, &[], &fragments, FrameDelimiter::JpegMarkers).err(),
        Some(FrameIndexError::BoundaryUnavailable {
            declared: 2,
            fragments: 4
        })
    );
}
