//! Local conformance evidence over the ignored DICOM corpus.
//!
//! The ordinary test suite remains self-contained. The `corpus` gate first
//! verifies every ignored file against `corpus/manifest.tsv`, then invokes
//! this ignored test. Failures report only row numbers and transfer-syntax
//! identifiers, never DICOM attribute values or instance identifiers.

use std::{fs, path::Path};

use ocelli_codec::{
    Capability, FrameDesc, FrameDescInput, PixelDataVr, PixelRepresentation, Registry,
    register_native_and_rle_decoders,
};
use ocelli_dicom::{DispatchPath, Tag, parse_part10};

const IMPLICIT_VR_LE: &str = "1.2.840.10008.1.2";
const EXPLICIT_VR_LE: &str = "1.2.840.10008.1.2.1";
const DEFLATED_EXPLICIT_VR_LE: &str = "1.2.840.10008.1.2.1.99";
const EXPLICIT_VR_BE: &str = "1.2.840.10008.1.2.2";
const RLE_LOSSLESS: &str = "1.2.840.10008.1.2.5";

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
