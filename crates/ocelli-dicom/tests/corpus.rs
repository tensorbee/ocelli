//! Local conformance evidence over the ignored DICOM corpus.
//!
//! The ordinary test suite remains self-contained. The `corpus` gate first
//! verifies every ignored file against `corpus/manifest.tsv`, then invokes
//! this ignored test. Failures report only row numbers and transfer-syntax
//! identifiers, never DICOM attribute values or instance identifiers.

use std::{collections::BTreeMap, fs, path::Path};

use ocelli_codec::{
    Capability, FrameDesc, FrameDescInput, PixelDataVr, PixelRepresentation, Registry,
    register_htj2k_decoders, register_jpeg_decoders, register_jpeg2000_decoders,
    register_jpegls_decoders, register_jpegxl_decoders, register_native_and_rle_decoders,
};
use ocelli_dicom::{
    ConformanceFinding, ConformanceRule, DispatchPath, EncapsulatedFrameIndex,
    FunctionalGroupSource, MetadataSet, MultiframeMetadata, ParsedDicom, Tag, TopLevelFallback, VR,
    check_conformance, parse_decimal_string, parse_part10,
};

const IMPLICIT_VR_LE: &str = "1.2.840.10008.1.2";
const EXPLICIT_VR_LE: &str = "1.2.840.10008.1.2.1";
const DEFLATED_EXPLICIT_VR_LE: &str = "1.2.840.10008.1.2.1.99";
const EXPLICIT_VR_BE: &str = "1.2.840.10008.1.2.2";
const RLE_LOSSLESS: &str = "1.2.840.10008.1.2.5";
const MULTIFRAME_ROW: &str = "synthetic/ct_multiframe_perframe.dcm";
const REFERENCE_ROW: &str = "syntax/reference_rgb8_saturated.dcm";
const J2K_COLOUR_LOSSLESS_ROW: &str = "syntax/j2k_lossless_ybr_rct.dcm";
const J2K_COLOUR_LOSSY_ROW: &str = "syntax/j2k_lossy_ybr_ict.dcm";
const HTJ2K_COLOUR_LOSSLESS_ROW: &str = "syntax/htj2k_lossless_ybr_rct.dcm";
const HTJ2K_COLOUR_LOSSY_ROW: &str = "syntax/htj2k_lossy_ybr_ict.dcm";
const ICC_ROW: &str = "synthetic/sc_rgb_icc.dcm";
const PIXEL_SPACING: Tag = Tag(0x0028, 0x0030);
const WINDOW_CENTER: Tag = Tag(0x0028, 0x1050);
const PIXEL_MEASURES_SEQUENCE: Tag = Tag(0x0028, 0x9110);
const FRAME_VOI_LUT_SEQUENCE: Tag = Tag(0x0028, 0x9132);

#[test]
#[ignore = "requires the verified local corpus through bin/ocelli.sh gate corpus"]
fn every_manifest_row_parses_under_its_declared_transfer_syntax() -> Result<(), String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let manifest = fs::read_to_string(root.join("corpus/manifest.tsv"))
        .map_err(|_| "tracked corpus manifest is unreadable".to_owned())?;
    let data = root.join("corpus/data");
    let mut parsed_count = 0_usize;

    for (row_index, line) in manifest.lines().skip(1).enumerate() {
        if line.trim().is_empty() {
            continue;
        }

        let row = row_index + 2;
        let mut columns = line.split('\t');
        let relative_path = columns
            .next()
            .ok_or_else(|| format!("manifest row {row} has no path"))?;
        let _modality = columns
            .next()
            .ok_or_else(|| format!("manifest row {row} has no modality"))?;
        let declared_transfer_syntax = columns
            .next()
            .ok_or_else(|| format!("manifest row {row} has no transfer syntax"))?
            .trim();
        if columns.count() != 6 {
            return Err(format!("manifest row {row} does not have nine columns"));
        }

        let bytes = fs::read(data.join(relative_path))
            .map_err(|_| format!("corpus file for manifest row {row} is unreadable"))?;
        let parsed = parse_part10(&bytes)
            .map_err(|_| format!("corpus manifest row {row} does not parse as Part 10"))?;
        let parsed_transfer_syntax = parsed.transfer_syntax().uid();
        if parsed_transfer_syntax != declared_transfer_syntax {
            return Err(format!(
                "manifest transfer syntax {declared_transfer_syntax} differs from parsed {parsed_transfer_syntax} at row {row}"
            ));
        }
        parsed_count += 1;
    }

    if parsed_count == 0 {
        return Err("the verified corpus contains no rows".to_owned());
    }

    Ok(())
}

#[test]
#[ignore = "requires the verified local corpus through bin/ocelli.sh gate corpus"]
fn manifest_backed_multiframe_row_preserves_functional_group_sources() -> Result<(), String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let manifest = fs::read_to_string(root.join("corpus/manifest.tsv"))
        .map_err(|_| "tracked corpus manifest is unreadable".to_owned())?;
    let row = manifest
        .lines()
        .skip(1)
        .find(|line| line.split('\t').next() == Some(MULTIFRAME_ROW))
        .ok_or_else(|| "tracked corpus manifest has no F-019 multiframe row".to_owned())?;
    let relative_path = row
        .split('\t')
        .next()
        .ok_or_else(|| "F-019 multiframe row has no path".to_owned())?;
    let bytes = fs::read(root.join("corpus/data").join(relative_path))
        .map_err(|_| "F-019 multiframe corpus file is unreadable".to_owned())?;
    let parsed = parse_part10(&bytes)
        .map_err(|_| "F-019 multiframe corpus file does not parse as Part 10".to_owned())?;
    let metadata = MetadataSet::from_object(parsed.object())
        .map_err(|_| "F-019 multiframe metadata projection failed".to_owned())?;
    let projection = MultiframeMetadata::new(&metadata)
        .map_err(|_| "F-019 multiframe structure is invalid".to_owned())?;

    if projection.frame_count() != 3 {
        return Err("F-019 multiframe row does not declare three frames".to_owned());
    }
    let shared = projection
        .resolve(
            0,
            PIXEL_MEASURES_SEQUENCE,
            PIXEL_SPACING,
            TopLevelFallback::Disallowed,
        )
        .map_err(|_| "F-019 shared functional-group lookup failed".to_owned())?
        .ok_or_else(|| "F-019 shared functional-group source is absent".to_owned())?;
    let per_frame = projection
        .resolve(
            1,
            FRAME_VOI_LUT_SEQUENCE,
            WINDOW_CENTER,
            TopLevelFallback::Disallowed,
        )
        .map_err(|_| "F-019 per-frame functional-group lookup failed".to_owned())?
        .ok_or_else(|| "F-019 per-frame functional-group source is absent".to_owned())?;

    if shared.source() != FunctionalGroupSource::Shared {
        return Err("F-019 Pixel Measures did not retain its shared source".to_owned());
    }
    if per_frame.source() != FunctionalGroupSource::PerFrame(1) {
        return Err("F-019 VOI did not retain its per-frame source".to_owned());
    }
    if shared.source() == per_frame.source() {
        return Err("F-019 functional-group sources were conflated".to_owned());
    }

    Ok(())
}

#[test]
#[ignore = "requires the verified local corpus through bin/ocelli.sh gate corpus"]
fn native_rle_and_deflate_syntax_rows_match_the_synthetic_reference() -> Result<(), String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let data = root.join("corpus/data/syntax");
    let reference_file = fs::read(data.join("explicit_vr_le.dcm"))
        .map_err(|_| "syntax reference is unreadable".to_owned())?;
    let expected = native_pixel_value(&reference_file, EXPLICIT_VR_LE)?.to_vec();
    let desc = mono16_frame(PixelDataVr::Ow)?;
    if expected.len() != desc.output_len() {
        return Err("syntax reference has the wrong decoded length".to_owned());
    }

    let mut registry = Registry::new();
    register_native_and_rle_decoders(&mut registry)
        .map_err(|_| "native and RLE registry setup failed".to_owned())?;

    for (filename, uid) in [
        ("implicit_vr_le.dcm", IMPLICIT_VR_LE),
        ("explicit_vr_le.dcm", EXPLICIT_VR_LE),
        ("explicit_vr_be.dcm", EXPLICIT_VR_BE),
    ] {
        let file = fs::read(data.join(filename))
            .map_err(|_| format!("syntax fixture {filename} is unreadable"))?;
        let source = native_pixel_value(&file, uid)?;
        let mut decoded = vec![0xa5; desc.output_len()];
        registry
            .decode(uid, source, &desc, &mut decoded)
            .map_err(|_| format!("native syntax {uid} did not decode"))?;
        if decoded != expected {
            return Err(format!("native syntax {uid} differs from its reference"));
        }
    }

    let rle_file = fs::read(data.join("rle_lossless.dcm"))
        .map_err(|_| "RLE syntax fixture is unreadable".to_owned())?;
    let rle =
        parse_part10(&rle_file).map_err(|_| "RLE syntax fixture does not parse".to_owned())?;
    let fragments = rle
        .object()
        .element(Tag(0x7fe0, 0x0010))
        .ok()
        .and_then(|element| element.value().fragments())
        .ok_or_else(|| "RLE syntax fixture has no encapsulated pixel fragments".to_owned())?;
    let [fragment] = fragments else {
        return Err("RLE syntax fixture does not contain exactly one frame fragment".to_owned());
    };
    let rle_desc = mono16_frame(PixelDataVr::Ob)?;
    let mut decoded = vec![0xa5; rle_desc.output_len()];
    registry
        .decode(RLE_LOSSLESS, fragment, &rle_desc, &mut decoded)
        .map_err(|_| "RLE syntax fixture did not decode".to_owned())?;
    if decoded != expected {
        return Err("RLE syntax fixture differs from its reference".to_owned());
    }

    let deflated_file = fs::read(data.join("deflated_explicit_vr_le.dcm"))
        .map_err(|_| "Deflate syntax fixture is unreadable".to_owned())?;
    let deflated = parse_part10(&deflated_file)
        .map_err(|_| "Deflate syntax fixture does not parse".to_owned())?;
    if deflated.transfer_syntax().dispatch_path() != DispatchPath::DeflatedExplicitVrLittleEndian {
        return Err("Deflate syntax fixture selected the wrong ingest path".to_owned());
    }
    if registry.capability(DEFLATED_EXPLICIT_VR_LE) != Capability::KnownUnavailable {
        return Err("Deflate syntax must remain unavailable to frame dispatch".to_owned());
    }
    let words = deflated
        .object()
        .element(Tag(0x7fe0, 0x0010))
        .map_err(|_| "Deflate syntax fixture has no Pixel Data".to_owned())?
        .to_multi_int::<u16>()
        .map_err(|_| "Deflate syntax Pixel Data is not a word sequence".to_owned())?;
    let deflated_pixels: Vec<u8> = words.into_iter().flat_map(u16::to_le_bytes).collect();
    if deflated_pixels != expected {
        return Err("Deflate ingest pixels differ from their reference".to_owned());
    }

    Ok(())
}

#[test]
#[ignore = "requires the verified local corpus through bin/ocelli.sh gate corpus"]
fn every_manifest_row_avoids_rules_a_well_formed_file_cannot_trip() -> Result<(), String> {
    // PS3.5 6.4. OB, OD, OF, OL, OV, OW and UN have one value. The one pad
    // byte on the last DA or DS value is not a format or length finding.
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let manifest = fs::read_to_string(root.join("corpus/manifest.tsv"))
        .map_err(|_| "tracked corpus manifest is unreadable".to_owned())?;
    let data = root.join("corpus/data");
    let mut parsed_count = 0_usize;
    let mut counts = BTreeMap::<&str, usize>::new();
    let mut violations = Vec::new();

    for (row_index, line) in manifest.lines().skip(1).enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let row = row_index + 2;
        let mut columns = line.split('\t');
        let relative_path = columns
            .next()
            .ok_or_else(|| format!("manifest row {row} has no path"))?;
        let _modality = columns
            .next()
            .ok_or_else(|| format!("manifest row {row} has no modality"))?;
        let declared_transfer_syntax = columns
            .next()
            .ok_or_else(|| format!("manifest row {row} has no transfer syntax"))?
            .trim();
        let bytes = fs::read(data.join(relative_path))
            .map_err(|_| format!("corpus file for manifest row {row} is unreadable"))?;
        let parsed = parse_part10(&bytes)
            .map_err(|_| format!("corpus manifest row {row} does not parse as Part 10"))?;
        let report = check_conformance(&parsed);
        for finding in report.findings() {
            *counts.entry(finding.rule.id()).or_default() += 1;
            if well_formed_cannot_trip(parsed.object(), finding) {
                violations.push(format!(
                    "manifest row {row} transfer syntax {declared_transfer_syntax} tag {} rule {}",
                    tag_text(finding.tag),
                    finding.rule.id()
                ));
            }
        }
        parsed_count += 1;
    }

    if parsed_count == 0 {
        return Err("the verified corpus contains no rows".to_owned());
    }
    println!("conformance findings by rule");
    for (rule, count) in &counts {
        println!("{rule}\t{count}");
    }
    println!("corpus rows\t{parsed_count}");
    if !violations.is_empty() {
        return Err(violations.join("\n"));
    }
    Ok(())
}

// F-244. Each empty-table row and the row whose codestream it carries, and
// for a multi-frame row `None`, because its three frames are the mono16
// reference, that reference with its columns reversed and with its rows
// reversed, and they are compared with the uncompressed reference directly.
const EMPTY_TABLE_ROWS: [(&str, Option<&str>); 19] = [
    ("rle_lossless_empty_bot.dcm", Some("rle_lossless.dcm")),
    (
        "jpeg_baseline_rgb8_empty_bot.dcm",
        Some("jpeg_baseline_rgb8.dcm"),
    ),
    (
        "jpeg_extended_12_empty_bot.dcm",
        Some("jpeg_extended_12.dcm"),
    ),
    (
        "jpeg_lossless_p14_empty_bot.dcm",
        Some("jpeg_lossless_p14.dcm"),
    ),
    (
        "jpeg_lossless_p14_sv1_empty_bot.dcm",
        Some("jpeg_lossless_p14_sv1.dcm"),
    ),
    ("jpegls_lossless_empty_bot.dcm", Some("jpegls_lossless.dcm")),
    (
        "jpegls_near_lossless_empty_bot.dcm",
        Some("jpegls_near_lossless.dcm"),
    ),
    ("j2k_lossless_empty_bot.dcm", Some("j2k_lossless.dcm")),
    ("j2k_lossy_empty_bot.dcm", Some("j2k_lossy.dcm")),
    ("htj2k_lossless_empty_bot.dcm", Some("htj2k_lossless.dcm")),
    (
        "htj2k_lossless_rpcl_empty_bot.dcm",
        Some("htj2k_lossless_rpcl.dcm"),
    ),
    ("htj2k_lossy_empty_bot.dcm", Some("htj2k_lossy.dcm")),
    ("jpegxl_lossless_empty_bot.dcm", Some("jpegxl_lossless.dcm")),
    ("jpegxl_lossy_empty_bot.dcm", Some("jpegxl_lossy.dcm")),
    (
        "jpegls_lossless_fragmented_empty_bot.dcm",
        Some("jpegls_lossless.dcm"),
    ),
    ("rle_multiframe_empty_bot.dcm", None),
    (
        "jpeg_lossless_p14_sv1_multiframe_fragmented_empty_bot.dcm",
        None,
    ),
    ("j2k_lossless_multiframe_fragmented_empty_bot.dcm", None),
    ("j2k_lossless_multiframe_eot.dcm", None),
];

#[test]
#[ignore = "requires the verified local corpus through bin/ocelli.sh gate corpus"]
fn manifest_backed_empty_table_rows_decode_as_their_sources() -> Result<(), String> {
    // PS3.5 A.4: decoders "need to accept both an empty Basic Offset Table
    // (zero length) and a Basic Offset Table filled with 32 bit offset
    // values". Each single-frame row decodes to its populated-table source's
    // bytes. Each multi-frame row decodes frame by frame to three distinct
    // frames, so a swapped or misplaced boundary is a different image.
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let manifest = fs::read_to_string(root.join("corpus/manifest.tsv"))
        .map_err(|_| "tracked corpus manifest is unreadable".to_owned())?;
    let data = root.join("corpus/data/syntax");
    let registry = every_decoder()?;

    let reference_file = fs::read(data.join("explicit_vr_le.dcm"))
        .map_err(|_| "syntax reference is unreadable".to_owned())?;
    let reference = native_pixel_value(&reference_file, EXPLICIT_VR_LE)?;
    let expected_frames = three_distinct_frames(reference)?;

    for (row, source) in EMPTY_TABLE_ROWS {
        if !manifest.contains(&format!("syntax/{row}\t")) {
            return Err(format!("{row} is not in the manifest"));
        }
        let file = fs::read(data.join(row)).map_err(|_| format!("{row} is unreadable"))?;
        let parsed = parse_part10(&file).map_err(|error| format!("{row}: {error}"))?;
        let frames =
            decode_every_frame(&registry, &parsed).map_err(|error| format!("{row}: {error}"))?;
        match source {
            Some(source) => {
                let source_file =
                    fs::read(data.join(source)).map_err(|_| format!("{source} is unreadable"))?;
                let source_parsed =
                    parse_part10(&source_file).map_err(|error| format!("{source}: {error}"))?;
                let want = decode_every_frame(&registry, &source_parsed)
                    .map_err(|error| format!("{source}: {error}"))?;
                if frames.len() != 1 || frames != want {
                    return Err(format!("{row} does not decode as {source}"));
                }
            }
            None => {
                if frames.len() != expected_frames.len() {
                    return Err(format!("{row} has {} frames, not 3", frames.len()));
                }
                for (frame, (got, want)) in frames.iter().zip(&expected_frames).enumerate() {
                    if got != want {
                        return Err(format!("{row} frame {frame} differs from its reference"));
                    }
                }
            }
        }
    }
    Ok(())
}

fn tag_text(tag: Tag) -> String {
    format!("({:04X},{:04X})", tag.group(), tag.element())
}

fn well_formed_cannot_trip(
    object: &ocelli_dicom::DefaultDicomObject,
    finding: &ConformanceFinding,
) -> bool {
    let Some((vr, parts)) = element_parts(object, finding) else {
        return false;
    };
    if finding.rule == ConformanceRule::Multiplicity && binary_vr(vr) {
        return true;
    }
    let Some(raw) = last_of_many(&parts, finding.value) else {
        return false;
    };
    if vr == VR::DA
        && matches!(
            finding.rule,
            ConformanceRule::DaFormat | ConformanceRule::FixedLength
        )
        && legal_da_pad(raw)
    {
        return true;
    }
    vr == VR::DS
        && matches!(
            finding.rule,
            ConformanceRule::DsFormat | ConformanceRule::ValueLength
        )
        && legal_ds_pad(raw)
}

fn binary_vr(vr: VR) -> bool {
    matches!(
        vr,
        VR::OB | VR::OD | VR::OF | VR::OL | VR::OV | VR::OW | VR::UN
    )
}

fn element_parts(
    object: &ocelli_dicom::DefaultDicomObject,
    finding: &ConformanceFinding,
) -> Option<(VR, Vec<String>)> {
    let mut current = &**object;
    for step in &finding.path {
        let element = current.element(step.sequence).ok()?;
        let items = element.value().items()?;
        let index = usize::try_from(step.item.checked_sub(1)?).ok()?;
        current = items.get(index)?;
    }
    let element = current.element(finding.tag).ok()?;
    let parts = element
        .value()
        .primitive()
        .and_then(|primitive| primitive.strings().ok())
        .map(|values| values.to_vec())
        .unwrap_or_default();
    Some((element.vr(), parts))
}

fn last_of_many(parts: &[String], value: Option<u32>) -> Option<&str> {
    if parts.len() < 2 {
        return None;
    }
    let number = value?;
    let index = usize::try_from(number.checked_sub(1)?).ok()?;
    if index + 1 != parts.len() {
        return None;
    }
    parts.get(index).map(String::as_str)
}

fn legal_da_pad(raw: &str) -> bool {
    let Some(body) = raw.strip_suffix(' ') else {
        return false;
    };
    body.len() == 8 && legal_date(body)
}

fn legal_ds_pad(raw: &str) -> bool {
    if raw.len() != 17 {
        return false;
    }
    let Some(body) = raw.strip_suffix(' ') else {
        return false;
    };
    body.len() == 16 && parse_decimal_string(body).is_some()
}

/// PS3.5 Table 6.2-1 DA is YYYYMMDD. 1900 is not a leap year. 2000 is.
fn legal_date(text: &str) -> bool {
    if text.len() != 8 || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return false;
    }
    let Ok(year) = text[0..4].parse::<u32>() else {
        return false;
    };
    let Ok(month) = text[4..6].parse::<u32>() else {
        return false;
    };
    let Ok(day) = text[6..8].parse::<u32>() else {
        return false;
    };
    let max = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400)) => {
            29
        }
        2 => 28,
        _ => return false,
    };
    (1..=max).contains(&day)
}

fn every_decoder() -> Result<Registry, String> {
    let mut registry = Registry::new();
    register_native_and_rle_decoders(&mut registry).map_err(|_| "native and RLE".to_owned())?;
    register_jpeg_decoders(&mut registry).map_err(|_| "JPEG".to_owned())?;
    register_jpeg2000_decoders(&mut registry).map_err(|_| "JPEG 2000".to_owned())?;
    register_jpegls_decoders(&mut registry).map_err(|_| "JPEG-LS".to_owned())?;
    register_htj2k_decoders(&mut registry).map_err(|_| "HTJ2K".to_owned())?;
    register_jpegxl_decoders(&mut registry).map_err(|_| "JPEG XL".to_owned())?;
    Ok(registry)
}

/// The mono16 reference, then with each row's samples reversed, then with
/// its rows in reverse order. 64 rows of 96 two-byte samples.
fn three_distinct_frames(reference: &[u8]) -> Result<Vec<Vec<u8>>, String> {
    let row_bytes = 96 * 2;
    if reference.len() != 64 * row_bytes {
        return Err("syntax reference has the wrong length".to_owned());
    }
    let rows: Vec<&[u8]> = reference.chunks_exact(row_bytes).collect();
    let columns_reversed = rows
        .iter()
        .flat_map(|row| row.chunks_exact(2).rev().flatten().copied())
        .collect();
    let rows_reversed = rows
        .iter()
        .rev()
        .flat_map(|row| row.iter().copied())
        .collect();
    Ok(vec![reference.to_vec(), columns_reversed, rows_reversed])
}

fn decode_every_frame(registry: &Registry, parsed: &ParsedDicom) -> Result<Vec<Vec<u8>>, String> {
    let object = parsed.object();
    let number = |tag: Tag| {
        object
            .element(tag)
            .ok()
            .and_then(|element| element.to_int::<u16>().ok())
    };
    let frame_count = object
        .element(Tag(0x0028, 0x0008))
        .ok()
        .and_then(|element| element.to_int::<usize>().ok())
        .unwrap_or(1);
    let desc = FrameDesc::new(FrameDescInput {
        rows: number(Tag(0x0028, 0x0010)).ok_or("no Rows")?,
        columns: number(Tag(0x0028, 0x0011)).ok_or("no Columns")?,
        samples_per_pixel: number(Tag(0x0028, 0x0002)).ok_or("no Samples per Pixel")?,
        bits_allocated: number(Tag(0x0028, 0x0100)).ok_or("no Bits Allocated")?,
        bits_stored: number(Tag(0x0028, 0x0101)).ok_or("no Bits Stored")?,
        high_bit: number(Tag(0x0028, 0x0102)).ok_or("no High Bit")?,
        pixel_representation: if number(Tag(0x0028, 0x0103)) == Some(1) {
            PixelRepresentation::Signed
        } else {
            PixelRepresentation::Unsigned
        },
        photometric_interpretation: object
            .element(Tag(0x0028, 0x0004))
            .ok()
            .and_then(|element| element.to_str().ok())
            .map(|text| text.trim().to_owned())
            .ok_or("no Photometric Interpretation")?,
        pixel_data_vr: PixelDataVr::Ob,
    })
    .map_err(|_| "frame description is invalid".to_owned())?;
    let index = EncapsulatedFrameIndex::from_parsed(parsed, frame_count)
        .map_err(|error| format!("frame index refused: {error:?}"))?;
    let uid = parsed.transfer_syntax().uid();
    let mut frames = Vec::with_capacity(frame_count);
    for frame in 0..frame_count {
        let encoded = index
            .frame_fragments(frame)
            .map_err(|error| format!("frame {frame}: {error:?}"))?
            .concat();
        let mut decoded = vec![0xa5; desc.output_len()];
        registry
            .decode(uid, &encoded, &desc, &mut decoded)
            .map_err(|error| format!("frame {frame} did not decode: {error}"))?;
        frames.push(decoded);
    }
    Ok(frames)
}

fn mono16_frame(pixel_data_vr: PixelDataVr) -> Result<FrameDesc, String> {
    FrameDesc::new(FrameDescInput {
        rows: 64,
        columns: 96,
        samples_per_pixel: 1,
        bits_allocated: 16,
        bits_stored: 16,
        high_bit: 15,
        pixel_representation: PixelRepresentation::Unsigned,
        photometric_interpretation: "MONOCHROME2".to_owned(),
        pixel_data_vr,
    })
    .map_err(|_| "synthetic syntax frame description is invalid".to_owned())
}

/// The colour corpus rows are 64 by 96, three samples at eight bits, from
/// `scripts/corpus_synth.py`'s saturated RGB reference.
fn colour_frame(photometric_interpretation: &str) -> Result<FrameDesc, String> {
    FrameDesc::new(FrameDescInput {
        rows: 64,
        columns: 96,
        samples_per_pixel: 3,
        bits_allocated: 8,
        bits_stored: 8,
        high_bit: 7,
        pixel_representation: PixelRepresentation::Unsigned,
        photometric_interpretation: photometric_interpretation.to_owned(),
        pixel_data_vr: PixelDataVr::Ob,
    })
    .map_err(|_| "colour corpus frame description is invalid".to_owned())
}

fn manifest_row(manifest: &str, path: &str, context: &str) -> Result<(String, String), String> {
    let line = manifest
        .lines()
        .skip(1)
        .find(|line| line.split('\t').next() == Some(path))
        .ok_or_else(|| format!("{context} has no manifest row"))?;
    let mut columns = line.split('\t');
    let _path = columns
        .next()
        .ok_or_else(|| format!("{context} row has no path"))?;
    let _modality = columns
        .next()
        .ok_or_else(|| format!("{context} row has no modality"))?;
    let transfer_syntax = columns
        .next()
        .ok_or_else(|| format!("{context} row has no transfer syntax"))?
        .trim()
        .to_owned();
    Ok((path.to_owned(), transfer_syntax))
}

/// Decode one encapsulated colour corpus row through the frame registry.
///
/// The row's own transfer syntax names the decoder, and the frame
/// description is the fixed geometry `scripts/corpus_synth.py` writes, so a
/// row whose header disagrees is refused rather than re-described.
fn decode_colour_row(
    root: &Path,
    path: &str,
    transfer_syntax: &str,
    photometric: &str,
) -> Result<Vec<u8>, String> {
    let bytes = fs::read(root.join("corpus/data").join(path))
        .map_err(|_| format!("{path} is unreadable"))?;
    let parsed = parse_part10(&bytes).map_err(|_| format!("{path} does not parse as Part 10"))?;
    let fragments = parsed
        .object()
        .element(Tag(0x7fe0, 0x0010))
        .ok()
        .and_then(|element| element.value().fragments())
        .ok_or_else(|| format!("{path} has no encapsulated Pixel Data"))?;
    let [fragment] = fragments else {
        return Err(format!("{path} does not hold exactly one frame fragment"));
    };
    let desc = colour_frame(photometric)?;
    let mut registry = Registry::new();
    register_native_and_rle_decoders(&mut registry)
        .map_err(|_| "registry setup failed".to_owned())?;
    register_jpeg2000_decoders(&mut registry)
        .map_err(|_| "JPEG 2000 registry setup failed".to_owned())?;
    register_htj2k_decoders(&mut registry).map_err(|_| "HTJ2K registry setup failed".to_owned())?;
    if registry.capability(transfer_syntax) != Capability::Available {
        return Err(format!("{path} declares an unavailable transfer syntax"));
    }
    let mut out = vec![0xa5; desc.output_len()];
    registry
        .decode(transfer_syntax, fragment, &desc, &mut out)
        .map_err(|_| format!("{path} did not decode"))?;
    Ok(out)
}

#[test]
#[ignore = "requires the verified local corpus through bin/ocelli.sh gate corpus"]
fn the_colour_jpeg_2000_rows_decode_to_the_saturated_reference() -> Result<(), String> {
    // ISO/IEC 15444-1 G.2, from the F-239 plan's working. The level-shifted
    // RGB `200 100 50` is `72 -28 -78`, whose forward RCT is
    // `Y0 = floor((72 + 2 * -28 + -78) / 4) = -16`, `Y1 = -78 - -28 = -50`,
    // `Y2 = 72 - -28 = 100`, the inverse gives `72 -28 -78` back, and
    // undoing the shift gives `200 100 50`. Pure red `255 0 0` gives
    // `Y0 = -65, Y1 = 0, Y2 = 255` and back. Both markers sit in the
    // saturated reference, whose first four pixels they are, and both
    // lossless rows must reproduce all of it byte for byte.
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let manifest = fs::read_to_string(root.join("corpus/manifest.tsv"))
        .map_err(|_| "tracked corpus manifest is unreadable".to_owned())?;

    let (path, syntax) = manifest_row(&manifest, J2K_COLOUR_LOSSLESS_ROW, "the .90 colour row")?;
    let j2k = decode_colour_row(&root, &path, &syntax, "YBR_RCT")?;
    let (path, syntax) = manifest_row(&manifest, HTJ2K_COLOUR_LOSSLESS_ROW, "the .201 colour row")?;
    let htj2k = decode_colour_row(&root, &path, &syntax, "YBR_RCT")?;

    // The rows' shared reference, read from its own manifest row's native
    // Pixel Data rather than from a fixture copy, so the anchor here is the
    // row the manifest attests and not an extracted copy of it.
    let (reference_path, reference_syntax) =
        manifest_row(&manifest, REFERENCE_ROW, "the saturated reference row")?;
    let reference_file = fs::read(root.join("corpus/data").join(&reference_path))
        .map_err(|_| "the saturated reference row is unreadable".to_owned())?;
    let reference = native_pixel_value(&reference_file, &reference_syntax)?.to_vec();
    let desc = colour_frame("YBR_RCT")?;
    if reference.len() != desc.output_len() {
        return Err("the saturated reference row has the wrong decoded length".to_owned());
    }

    // Marker pixels, three bytes per interleaved pixel: pure red, pure
    // green, pure blue, then the worked 200 100 50 triple.
    for (name, range, expected) in [
        ("pure red", 0..3, [255_u8, 0, 0]),
        ("pure green", 3..6, [0, 255, 0]),
        ("pure blue", 6..9, [0, 0, 255]),
        ("the worked RCT triple", 9..12, [200, 100, 50]),
    ] {
        for (decoder, decoded) in [(".90", &j2k), (".201", &htj2k)] {
            assert_eq!(
                decoded.get(range.clone()),
                Some(&expected[..]),
                "{decoder} does not reproduce {name} from the saturated reference"
            );
        }
    }
    // Two independent encoders against one reference row: each decode
    // reproduces it byte for byte, which is what makes the row an anchor
    // rather than a restatement of either implementation.
    assert_eq!(j2k, reference, "the .90 row differs from the reference row");
    assert_eq!(
        htj2k, reference,
        "the .201 row differs from the reference row"
    );
    Ok(())
}

#[test]
#[ignore = "requires the verified local corpus through bin/ocelli.sh gate corpus"]
fn the_colour_j2k_lossy_row_decodes_within_its_recorded_bound() -> Result<(), String> {
    // ISO/IEC 15444-1 G.3, from the F-239 plan's working: `Y0 = -28`,
    // `Y1 = 22`, `Y2 = -38` invert to `-81.276 -8.43354 10.984`, which the
    // decoder rounds to `47 120 139`. A whole frame of those coefficients is
    // not buildable by an available encoder, so the exact values are pinned
    // at a 1e-4 tolerance by the vendored patch's unit test
    // `inverse_ict_9_7_is_the_annex_g_transform`, and the codec test
    // `inverse_ict_uses_g6_literal_coefficients` records this row's bound
    // against OpenJPEG's own decode of the same codestream. This row's
    // evidence here is the decode through the production registry: the three
    // pure markers survive the lossy encode exactly, and the worked triple
    // arrives one code below its reference value at 199, which is also
    // OpenJPEG's own decode of this row and sits inside the recorded
    // one-code-per-channel bound.
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let manifest = fs::read_to_string(root.join("corpus/manifest.tsv"))
        .map_err(|_| "tracked corpus manifest is unreadable".to_owned())?;
    let (path, syntax) = manifest_row(&manifest, J2K_COLOUR_LOSSY_ROW, "the .91 colour row")?;
    let decoded = decode_colour_row(&root, &path, &syntax, "YBR_ICT")?;

    let expected: [(&str, usize, [u8; 3]); 4] = [
        ("pure red", 0, [255, 0, 0]),
        ("pure green", 1, [0, 255, 0]),
        ("pure blue", 2, [0, 0, 255]),
        (
            "the worked triple, one code inside the bound",
            3,
            [199, 100, 50],
        ),
    ];
    for (name, index, expected) in expected {
        let at = index * 3..index * 3 + 3;
        assert_eq!(
            decoded.get(at.clone()),
            Some(&expected[..]),
            "the .91 lossy row does not reproduce {name}"
        );
    }
    Ok(())
}

#[test]
#[ignore = "requires the verified local corpus through bin/ocelli.sh gate corpus"]
fn the_colour_htj2k_lossy_row_decodes_within_its_recorded_bound() -> Result<(), String> {
    // ISO/IEC 15444-1 G.3, from the F-239 plan's working: `Y0 = -28`,
    // `Y1 = 22`, `Y2 = -38` invert to `-81.276 -8.43354 10.984`, which the
    // decoder rounds to `47 120 139`. A whole frame of those coefficients is
    // not buildable by an available encoder, so the exact coefficients are
    // pinned in the vendored patches' own unit tests. This row's evidence
    // here is the marker pixels and the decode through the production
    // registry, which the nearly lossless quality keeps exact, and its
    // agreement with OpenJPEG's own decode of the same codestream, byte for
    // byte, is the codec test `inverse_ict_uses_g6_literal_coefficients`.
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let manifest = fs::read_to_string(root.join("corpus/manifest.tsv"))
        .map_err(|_| "tracked corpus manifest is unreadable".to_owned())?;
    let (path, syntax) = manifest_row(&manifest, HTJ2K_COLOUR_LOSSY_ROW, "the .203 colour row")?;
    let decoded = decode_colour_row(&root, &path, &syntax, "YBR_ICT")?;

    let expected: [(&str, usize, [u8; 3]); 4] = [
        ("pure red", 0, [255, 0, 0]),
        ("pure green", 1, [0, 255, 0]),
        ("pure blue", 2, [0, 0, 255]),
        ("the worked RCT triple", 3, [200, 100, 50]),
    ];
    for (name, index, expected) in expected {
        let at = index * 3..index * 3 + 3;
        assert_eq!(
            decoded.get(at.clone()),
            Some(&expected[..]),
            "the .203 lossy row does not reproduce {name}"
        );
    }

    // The comparison against OpenJPEG's own decode of this codestream is
    // the codec test `inverse_ict_uses_g6_literal_coefficients`, which records
    // the byte-for-byte bound against the extracted reference decode. This
    // row's evidence here is the marker pixels and the decode through the
    // production registry.
    Ok(())
}

#[test]
#[ignore = "requires the verified local corpus through bin/ocelli.sh gate corpus"]
fn the_icc_row_declares_colour_and_the_element_is_present() -> Result<(), String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let manifest = fs::read_to_string(root.join("corpus/manifest.tsv"))
        .map_err(|_| "tracked corpus manifest is unreadable".to_owned())?;
    let (path, syntax) = manifest_row(&manifest, ICC_ROW, "the ICC row")?;
    let bytes = fs::read(root.join("corpus/data").join(path))
        .map_err(|_| "the ICC row is unreadable".to_owned())?;
    let parsed =
        parse_part10(&bytes).map_err(|_| "the ICC row does not parse as Part 10".to_owned())?;
    if parsed.transfer_syntax().uid() != syntax {
        return Err("the ICC row's transfer syntax does not match its manifest row".to_owned());
    }
    let metadata = MetadataSet::from_object(parsed.object())
        .map_err(|_| "the ICC row's metadata projection failed".to_owned())?;
    if metadata.get(Tag(0x0028, 0x2000)).is_none() {
        return Err("the ICC row carries no ICC Profile element".to_owned());
    }
    Ok(())
}

fn native_pixel_value<'a>(file: &'a [u8], transfer_syntax: &str) -> Result<&'a [u8], String> {
    let (tag, header_len, byte_order) = match transfer_syntax {
        IMPLICIT_VR_LE => ([0xe0, 0x7f, 0x10, 0x00], 8, "little"),
        EXPLICIT_VR_LE => ([0xe0, 0x7f, 0x10, 0x00], 12, "little"),
        EXPLICIT_VR_BE => ([0x7f, 0xe0, 0x00, 0x10], 12, "big"),
        _ => return Err("native extraction received a non-native syntax".to_owned()),
    };
    let element = file
        .windows(tag.len())
        .rposition(|window| window == tag)
        .ok_or_else(|| "native syntax fixture has no Pixel Data tag".to_owned())?;
    let length_offset = if header_len == 8 {
        element + 4
    } else {
        element + 8
    };
    let length_bytes: [u8; 4] = file
        .get(length_offset..length_offset + 4)
        .and_then(|bytes| bytes.try_into().ok())
        .ok_or_else(|| "native Pixel Data header is truncated".to_owned())?;
    let value_len = match byte_order {
        "little" => u32::from_le_bytes(length_bytes),
        "big" => u32::from_be_bytes(length_bytes),
        _ => unreachable!(),
    };
    let value_start = element + header_len;
    let value_end = value_start
        .checked_add(usize::try_from(value_len).map_err(|_| "native Pixel Data is too large")?)
        .ok_or_else(|| "native Pixel Data length overflows".to_owned())?;
    file.get(value_start..value_end)
        .ok_or_else(|| "native Pixel Data value is truncated".to_owned())
}
