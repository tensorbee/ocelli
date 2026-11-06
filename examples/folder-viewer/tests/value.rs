//! Modality value at a point. Every expected value is computed by hand from
//! PS3.5 8.1.1 (unused bits above Bits Stored are not assumed zero, and the
//! sign bit is the High Bit), PS3.3 C.7.6.3.3 (High Bit is Bits Stored minus
//! one) and PS3.3 C.11.1 (output units = m * SV + b), not from a previous
//! run of this route.

use std::io::{Read, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
use std::sync::Arc;
use std::time::Duration;

use ocelli_folder_viewer::{
    FrameRequest, ImagePoint, PreviewError, Viewer, ViewerConfig, preview_first_frame,
    preview_frame, serve_on, slice, value_frame,
};

// ---------------------------------------------------------------------------
// Builders. Elements are sorted by tag before they are written.

struct Spec {
    rows: u16,
    columns: u16,
    frames: Option<u16>,
    bits: (u16, u16, u16, u16),
    modality: Option<&'static str>,
    rescale: Option<(&'static str, &'static str)>,
    extra: Vec<((u16, u16), Vec<u8>)>,
    pixels: Vec<u8>,
}

impl Spec {
    fn mono(columns: u16, pixels: &[u16]) -> Self {
        let mut bytes = Vec::new();
        for value in pixels {
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        Self {
            rows: 1,
            columns,
            frames: None,
            bits: (16, 16, 15, 0),
            modality: None,
            rescale: None,
            extra: Vec::new(),
            pixels: bytes,
        }
    }
}

fn build(spec: Spec) -> Result<Vec<u8>, String> {
    let mut elements: Vec<((u16, u16), Vec<u8>)> = Vec::new();
    elements.push((
        (0x0008, 0x0016),
        short_le((0x0008, 0x0016), *b"UI", &ui(b"1.2.840.10008.5.1.4.1.1.2"))?,
    ));
    if let Some(modality) = spec.modality {
        elements.push((
            (0x0008, 0x0060),
            short_le((0x0008, 0x0060), *b"CS", &text_pad(modality))?,
        ));
    }
    elements.push((
        (0x0028, 0x0002),
        short_le((0x0028, 0x0002), *b"US", &1_u16.to_le_bytes())?,
    ));
    elements.push((
        (0x0028, 0x0004),
        short_le((0x0028, 0x0004), *b"CS", &text_pad("MONOCHROME2"))?,
    ));
    if let Some(frames) = spec.frames {
        elements.push((
            (0x0028, 0x0008),
            short_le((0x0028, 0x0008), *b"IS", &text_pad(&frames.to_string()))?,
        ));
    }
    elements.push((
        (0x0028, 0x0010),
        short_le((0x0028, 0x0010), *b"US", &spec.rows.to_le_bytes())?,
    ));
    elements.push((
        (0x0028, 0x0011),
        short_le((0x0028, 0x0011), *b"US", &spec.columns.to_le_bytes())?,
    ));
    let (allocated, stored, high, representation) = spec.bits;
    for (tag, value) in [
        ((0x0028, 0x0100), allocated),
        ((0x0028, 0x0101), stored),
        ((0x0028, 0x0102), high),
        ((0x0028, 0x0103), representation),
    ] {
        elements.push((tag, short_le(tag, *b"US", &value.to_le_bytes())?));
    }
    if let Some((slope, intercept)) = spec.rescale {
        elements.push((
            (0x0028, 0x1052),
            short_le((0x0028, 0x1052), *b"DS", &text_pad(intercept))?,
        ));
        elements.push((
            (0x0028, 0x1053),
            short_le((0x0028, 0x1053), *b"DS", &text_pad(slope))?,
        ));
    }
    for (tag, bytes) in spec.extra {
        elements.push((tag, bytes));
    }
    elements.push((
        (0x7FE0, 0x0010),
        long_le((0x7FE0, 0x0010), *b"OW", &spec.pixels)?,
    ));
    elements.sort_by_key(|(tag, _)| *tag);
    let mut bytes = vec![0_u8; 128];
    bytes.extend_from_slice(b"DICM");
    bytes.extend(file_meta()?);
    for (_, element) in elements {
        bytes.extend(element);
    }
    Ok(bytes)
}

fn text_element(tag: (u16, u16), vr: [u8; 2], text: &str) -> Result<((u16, u16), Vec<u8>), String> {
    Ok((tag, short_le(tag, vr, &text_pad(text))?))
}

fn item(content: &[u8]) -> Result<Vec<u8>, String> {
    let length = u32::try_from(content.len()).map_err(|_| "item length".to_owned())?;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&0xFFFE_u16.to_le_bytes());
    bytes.extend_from_slice(&0xE000_u16.to_le_bytes());
    bytes.extend_from_slice(&length.to_le_bytes());
    bytes.extend_from_slice(content);
    Ok(bytes)
}

fn file_meta() -> Result<Vec<u8>, String> {
    let mut body = Vec::new();
    body.extend(long_le((0x0002, 0x0001), *b"OB", &[0, 1])?);
    body.extend(short_le(
        (0x0002, 0x0002),
        *b"UI",
        &ui(b"1.2.840.10008.5.1.4.1.1.2"),
    )?);
    body.extend(short_le((0x0002, 0x0003), *b"UI", &ui(b"2.25.1"))?);
    body.extend(short_le(
        (0x0002, 0x0010),
        *b"UI",
        &ui(b"1.2.840.10008.1.2.1"),
    )?);
    body.extend(short_le((0x0002, 0x0012), *b"UI", &ui(b"2.25.2"))?);
    let length = u32::try_from(body.len()).map_err(|_| "meta length".to_owned())?;
    let mut meta = short_le((0x0002, 0x0000), *b"UL", &length.to_le_bytes())?;
    meta.extend(body);
    Ok(meta)
}

fn ui(text: &[u8]) -> Vec<u8> {
    let mut value = text.to_vec();
    if !value.len().is_multiple_of(2) {
        value.push(0);
    }
    value
}

fn text_pad(text: &str) -> Vec<u8> {
    let mut value = text.as_bytes().to_vec();
    if !value.len().is_multiple_of(2) {
        value.push(b' ');
    }
    value
}

fn short_le(tag: (u16, u16), vr: [u8; 2], value: &[u8]) -> Result<Vec<u8>, String> {
    if !value.len().is_multiple_of(2) {
        return Err("odd element".to_owned());
    }
    let length = u16::try_from(value.len()).map_err(|_| "element length".to_owned())?;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&tag.0.to_le_bytes());
    bytes.extend_from_slice(&tag.1.to_le_bytes());
    bytes.extend_from_slice(&vr);
    bytes.extend_from_slice(&length.to_le_bytes());
    bytes.extend_from_slice(value);
    Ok(bytes)
}

fn long_le(tag: (u16, u16), vr: [u8; 2], value: &[u8]) -> Result<Vec<u8>, String> {
    if !value.len().is_multiple_of(2) {
        return Err("odd element".to_owned());
    }
    let length = u32::try_from(value.len()).map_err(|_| "element length".to_owned())?;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&tag.0.to_le_bytes());
    bytes.extend_from_slice(&tag.1.to_le_bytes());
    bytes.extend_from_slice(&vr);
    bytes.extend_from_slice(&[0, 0]);
    bytes.extend_from_slice(&length.to_le_bytes());
    bytes.extend_from_slice(value);
    Ok(bytes)
}

fn at(x: f64, y: f64) -> ImagePoint {
    ImagePoint { x, y }
}

fn expect(got: Result<String, PreviewError>, want: &str, name: &str) -> Result<(), String> {
    match got {
        Ok(json) if json == want => Ok(()),
        Ok(json) => Err(format!("{name}: got {json}, wanted {want}")),
        Err(error) => Err(format!("{name}: refused with {}", error.code())),
    }
}

fn expect_refusal(
    got: Result<String, PreviewError>,
    want: PreviewError,
    name: &str,
) -> Result<(), String> {
    match got {
        Err(error) if error == want => Ok(()),
        Err(error) => Err(format!("{name}: refused with {}", error.code())),
        Ok(json) => Err(format!("{name}: answered {json}")),
    }
}

// ---------------------------------------------------------------------------
// Hand-computed fixtures through `value_frame`, the composition the route uses.

/// Bits Allocated 16, Bits Stored 12, High Bit 11, Pixel Representation 1.
fn signed_12_in_16() -> Result<Vec<u8>, String> {
    let mut spec = Spec::mono(2, &[0x0FFF, 0xF7FF]);
    spec.bits = (16, 12, 11, 1);
    spec.modality = Some("CT");
    spec.rescale = Some(("1", "-1024"));
    build(spec)
}

#[test]
fn value_signed_12_bit_ct_is_minus_1025() -> Result<(), String> {
    // PS3.5 8.1.1 and PS3.3 C.7.6.3.3. The container 0x0FFF masked to Bits
    // Stored 12 is 0xFFF. Bit 11, the High Bit, is set and Pixel
    // Representation is 1, so the stored value is 0xFFF - 0x1000 = -1.
    // PS3.3 C.11.1: -1 * 1 + (-1024) = -1025. Modality CT with no Rescale
    // Type and no Multi-energy CT Acquisition, so the unit is HU (C.8.2.1).
    let file = signed_12_in_16()?;
    expect(
        value_frame(&file, 0, &[at(0.5, 0.5)]),
        "{\"frame\":0,\"unit\":\"HU\",\"unit_source\":\"ct-default\",\"points\":[\
         {\"column\":1,\"row\":1,\"index_column\":0,\"index_row\":0,\"padding\":false,\"value\":-1025,\"text\":\"-1025 HU\"}]}",
        "signed 12 bit",
    )
}

#[test]
fn value_masks_bits_above_bits_stored() -> Result<(), String> {
    // PS3.5 8.1.1: the bits above Bits Stored are not assumed to be zero.
    // 0xF7FF & 0x0FFF = 0x07FF. Bit 11 is clear, so the stored value is
    // 2047. PS3.3 C.11.1: 2047 * 1 + (-1024) = 1023.
    let file = signed_12_in_16()?;
    expect(
        value_frame(&file, 0, &[at(1.5, 0.5)]),
        "{\"frame\":0,\"unit\":\"HU\",\"unit_source\":\"ct-default\",\"points\":[\
         {\"column\":2,\"row\":1,\"index_column\":1,\"index_row\":0,\"padding\":false,\"value\":1023,\"text\":\"1023 HU\"}]}",
        "masked bits",
    )
}

#[test]
fn value_mr_slope_two_is_minus_400_unspecified() -> Result<(), String> {
    // PS3.3 C.11.1: 300 * 2 + (-1000) = -400. MR with no Rescale Type has
    // no unit this rule can name, so it is `unspecified` from `none`.
    let mut spec = Spec::mono(1, &[300]);
    spec.modality = Some("MR");
    spec.rescale = Some(("2", "-1000"));
    let file = build(spec)?;
    expect(
        value_frame(&file, 0, &[at(0.5, 0.5)]),
        "{\"frame\":0,\"unit\":\"unspecified\",\"unit_source\":\"none\",\"points\":[\
         {\"column\":1,\"row\":1,\"index_column\":0,\"index_row\":0,\"padding\":false,\"value\":-400,\"text\":\"-400 unspecified\"}]}",
        "MR slope 2",
    )
}

#[test]
fn value_fractional_rescale_keeps_two_decimals() -> Result<(), String> {
    // PS3.3 C.11.1: 3 * 0.5 + 0.25 = 1.75. Every term is a short binary
    // fraction, so 1.75 is exact in f32 and prints as 1.75.
    let mut spec = Spec::mono(1, &[3]);
    spec.modality = Some("MR");
    spec.rescale = Some(("0.5", "0.25"));
    let file = build(spec)?;
    expect(
        value_frame(&file, 0, &[at(0.5, 0.5)]),
        "{\"frame\":0,\"unit\":\"unspecified\",\"unit_source\":\"none\",\"points\":[\
         {\"column\":1,\"row\":1,\"index_column\":0,\"index_row\":0,\"padding\":false,\"value\":1.75,\"text\":\"1.75 unspecified\"}]}",
        "fractional rescale",
    )
}

#[test]
fn value_without_rescale_is_the_stored_value() -> Result<(), String> {
    // PS3.3 C.11.1. With neither Rescale Slope nor Rescale Intercept the
    // modality stage is the identity, 7 * 1 + 0 = 7.
    let mut spec = Spec::mono(1, &[7]);
    spec.modality = Some("OT");
    let file = build(spec)?;
    expect(
        value_frame(&file, 0, &[at(0.5, 0.5)]),
        "{\"frame\":0,\"unit\":\"unspecified\",\"unit_source\":\"none\",\"points\":[\
         {\"column\":1,\"row\":1,\"index_column\":0,\"index_row\":0,\"padding\":false,\"value\":7,\"text\":\"7 unspecified\"}]}",
        "identity",
    )
}

fn unit_case(
    modality: &'static str,
    rescale_type: Option<&str>,
    units: Option<&str>,
) -> Result<String, String> {
    let mut spec = Spec::mono(1, &[5]);
    spec.modality = Some(modality);
    spec.rescale = Some(("1", "0"));
    if let Some(code) = rescale_type {
        spec.extra
            .push(text_element((0x0028, 0x1054), *b"LO", code)?);
    }
    if let Some(code) = units {
        spec.extra
            .push(text_element((0x0054, 0x1001), *b"CS", code)?);
    }
    let file = build(spec)?;
    value_frame(&file, 0, &[at(0.5, 0.5)]).map_err(|error| error.code().to_owned())
}

/// Name, Modality, Rescale Type, Units, expected unit, expected source.
type UnitCase = (
    &'static str,
    &'static str,
    Option<&'static str>,
    Option<&'static str>,
    &'static str,
    &'static str,
);

#[test]
fn value_unit_follows_rescale_type() -> Result<(), String> {
    // PS3.3 C.11.1.1.2 defined terms. C.8.2.1 makes Rescale Type required on
    // CT if it is not HU or Multi-energy CT Acquisition is YES, so an absent
    // one on a CT that is not multi-energy is HU. C.8.9.4 states the PET
    // unit in Units (0054,1001). Stored 5, slope 1, intercept 0, so every
    // value is 5 (C.11.1).
    let cases: [UnitCase; 7] = [
        (
            "CT HU_MOD",
            "CT",
            Some("HU_MOD"),
            None,
            "HU_MOD",
            "rescale-type",
        ),
        (
            "CT US",
            "CT",
            Some("US"),
            None,
            "unspecified",
            "rescale-type",
        ),
        ("CT HU", "CT", Some("HU"), None, "HU", "rescale-type"),
        ("CT absent", "CT", None, None, "HU", "ct-default"),
        ("PT BQML", "PT", None, Some("BQML"), "BQML", "units"),
        ("PT neither", "PT", None, None, "unspecified", "none"),
        (
            "MR Units ignored",
            "MR",
            None,
            Some("BQML"),
            "unspecified",
            "none",
        ),
    ];
    for (name, modality, rescale_type, units, unit, source) in cases {
        let got = unit_case(modality, rescale_type, units)?;
        let want = five_with(unit, source);
        if got != want {
            return Err(format!("{name}: got {got}, wanted {want}"));
        }
    }
    Ok(())
}

/// The reply for stored 5, slope 1, intercept 0 at the one pixel.
fn five_with(unit: &str, source: &str) -> String {
    format!(
        "{{\"frame\":0,\"unit\":\"{unit}\",\"unit_source\":\"{source}\",\"points\":[\
         {{\"column\":1,\"row\":1,\"index_column\":0,\"index_row\":0,\"padding\":false,\"value\":5,\"text\":\"5 {unit}\"}}]}}"
    )
}

#[test]
fn value_pet_units_decide_over_rescale_type() -> Result<(), String> {
    // PS3.3 C.8.9.4, PET Image Module, Rescale Intercept: "U = m*SV+b" with
    // U the pixel value units "defined in Units (0054,1001)". On PT, Units
    // names the unit whenever it is present, whatever Rescale Type says.
    // Stored 5, slope 1, intercept 0, so 5 * 1 + 0 = 5 (PS3.3 C.11.1).
    let cases: [UnitCase; 4] = [
        (
            "PT US and BQML",
            "PT",
            Some("US"),
            Some("BQML"),
            "BQML",
            "units",
        ),
        (
            "PT CNTS and BQML",
            "PT",
            Some("CNTS"),
            Some("BQML"),
            "BQML",
            "units",
        ),
        (
            "PT US alone",
            "PT",
            Some("US"),
            None,
            "unspecified",
            "rescale-type",
        ),
        (
            "PT CNTS alone",
            "PT",
            Some("CNTS"),
            None,
            "CNTS",
            "rescale-type",
        ),
    ];
    for (name, modality, rescale_type, units, unit, source) in cases {
        let got = unit_case(modality, rescale_type, units)?;
        let want = five_with(unit, source);
        if got != want {
            return Err(format!("{name}: got {got}, wanted {want}"));
        }
    }
    Ok(())
}

/// Name, Multi-energy CT Acquisition, Rescale Type, expected unit, source.
type MultiEnergyCase = (
    &'static str,
    Option<&'static str>,
    Option<&'static str>,
    &'static str,
    &'static str,
);

#[test]
fn value_unit_is_bounded_by_the_lo_length() -> Result<(), String> {
    // PS3.5 Table 6.2-1: LO, the VR of Rescale Type (0028,1054), is "64
    // chars maximum". A 64-letter Rescale Type is answered with that unit.
    // A 65-letter one is longer than any Rescale Type or Units value can be,
    // so the route refuses it `attribute`. Stored 5, slope 1, intercept 0.
    let legal = "A".repeat(64);
    let got = unit_case("MR", Some(&legal), None)?;
    let want = five_with(&legal, "rescale-type");
    if got != want {
        return Err(format!("64 letters: got {got}, wanted {want}"));
    }
    let long = "A".repeat(65);
    match unit_case("MR", Some(&long), None) {
        Err(code) if code == "attribute" => Ok(()),
        other => Err(format!("65 letters gave {other:?}")),
    }
}

#[test]
fn value_multi_energy_ct_has_no_assumed_unit() -> Result<(), String> {
    // PS3.3 C.8.2.1, Table C.8-3, CT Image Module. Rescale Type is "Required
    // if the Rescale Type is not HU (Hounsfield Units), or Multi-energy CT
    // Acquisition (0018,9361) is YES", and Rescale Intercept says the output
    // is HU only when "Multi-energy CT Acquisition (0018,9361) is either
    // absent or NO". So an absent Rescale Type means HU only when the
    // attribute is absent or NO. Multi-energy CT Acquisition is a top-level
    // Type 3 attribute in Table C.8-3 and in Table C.8-114 (C.8.15.2,
    // Enhanced CT Image Module). Stored 5, slope 1, intercept 0, so 5.
    let cases: [MultiEnergyCase; 4] = [
        (
            "YES and no Rescale Type",
            Some("YES"),
            None,
            "unspecified",
            "none",
        ),
        (
            "NO and no Rescale Type",
            Some("NO"),
            None,
            "HU",
            "ct-default",
        ),
        ("YES and HU", Some("YES"), Some("HU"), "HU", "rescale-type"),
        (
            "YES and Z_EFF",
            Some("YES"),
            Some("Z_EFF"),
            "Z_EFF",
            "rescale-type",
        ),
    ];
    for (name, multi_energy, rescale_type, unit, source) in cases {
        let mut spec = Spec::mono(1, &[5]);
        spec.modality = Some("CT");
        spec.rescale = Some(("1", "0"));
        if let Some(code) = multi_energy {
            spec.extra
                .push(text_element((0x0018, 0x9361), *b"CS", code)?);
        }
        if let Some(code) = rescale_type {
            spec.extra
                .push(text_element((0x0028, 0x1054), *b"LO", code)?);
        }
        let file = build(spec)?;
        expect(
            value_frame(&file, 0, &[at(0.5, 0.5)]),
            &five_with(unit, source),
            name,
        )?;
    }
    // Enhanced CT. The attribute is at the top level and the frame's Pixel
    // Value Transformation Sequence carries no Rescale Type. Stored 1040,
    // slope 1, intercept -1024 on frame 0, so 1040 * 1 + (-1024) = 16.
    let file = enhanced(&[], Some("YES"), &[None, None])?;
    expect(
        value_frame(&file, 0, &[at(0.5, 0.5)]),
        "{\"frame\":0,\"unit\":\"unspecified\",\"unit_source\":\"none\",\"points\":[\
         {\"column\":1,\"row\":1,\"index_column\":0,\"index_row\":0,\"padding\":false,\"value\":16,\"text\":\"16 unspecified\"}]}",
        "enhanced multi-energy",
    )
}

/// Enhanced CT, two frames of one pixel, both stored 1040, slope 1 and
/// intercept -1024 at the top level and in each frame's Pixel Value
/// Transformation Sequence. `top` adds top-level elements, `multi_energy`
/// adds Multi-energy CT Acquisition, and `types` is each frame's Rescale Type.
fn enhanced(
    top: &[((u16, u16), Vec<u8>)],
    multi_energy: Option<&str>,
    types: &[Option<&str>; 2],
) -> Result<Vec<u8>, String> {
    let mut per_frame = Vec::new();
    for rescale_type in types {
        let mut body = [
            short_le((0x0028, 0x1052), *b"DS", &text_pad("-1024"))?,
            short_le((0x0028, 0x1053), *b"DS", &text_pad("1"))?,
        ]
        .concat();
        if let Some(code) = rescale_type {
            body.extend(short_le((0x0028, 0x1054), *b"LO", &text_pad(code))?);
        }
        per_frame.extend(item(&long_le((0x0028, 0x9145), *b"SQ", &item(&body)?)?)?);
    }
    let mut spec = Spec::mono(1, &[1040, 1040]);
    spec.frames = Some(2);
    spec.modality = Some("CT");
    spec.rescale = Some(("1", "-1024"));
    spec.extra.extend(top.iter().cloned());
    if let Some(code) = multi_energy {
        spec.extra
            .push(text_element((0x0018, 0x9361), *b"CS", code)?);
    }
    spec.extra.push((
        (0x5200, 0x9230),
        long_le((0x5200, 0x9230), *b"SQ", &per_frame)?,
    ));
    build(spec)
}

#[test]
fn value_reads_the_per_frame_rescale_type() -> Result<(), String> {
    // PS3.3 C.7.6.16.2.9, Table C.7.6.16-10. In an Enhanced CT, Rescale Type
    // is Type 1 inside each frame's Pixel Value Transformation Sequence and
    // the frame's value wins over the top level. Frame 0 is
    // 1040 * 1 + (-1024) = 16 (PS3.3 C.11.1), and its group names HU_MOD.
    // With no top-level Rescale Type, a top-level read would say HU from
    // ct-default. With a top-level HU, it would say HU from rescale-type.
    let want = "{\"frame\":0,\"unit\":\"HU_MOD\",\"unit_source\":\"rescale-type\",\"points\":[\
         {\"column\":1,\"row\":1,\"index_column\":0,\"index_row\":0,\"padding\":false,\"value\":16,\"text\":\"16 HU_MOD\"}]}";
    let types = [Some("HU_MOD"), Some("HU_MOD")];
    let bare = enhanced(&[], None, &types)?;
    expect(value_frame(&bare, 0, &[at(0.5, 0.5)]), want, "no top level")?;
    let top_hu = [text_element((0x0028, 0x1054), *b"LO", "HU")?];
    let shadowed = enhanced(&top_hu, None, &types)?;
    expect(
        value_frame(&shadowed, 0, &[at(0.5, 0.5)]),
        want,
        "top level HU",
    )
}

/// One stored sample of 5 in a 32-bit container, MR, slope 1, intercept 0.
fn wide(bits_stored: u16, padding: bool) -> Result<Vec<u8>, String> {
    let mut spec = Spec::mono(1, &[]);
    spec.pixels = 5_u32.to_le_bytes().to_vec();
    spec.bits = (32, bits_stored, bits_stored - 1, 0);
    spec.modality = Some("MR");
    spec.rescale = Some(("1", "0"));
    if padding {
        spec.extra.push((
            (0x0028, 0x0120),
            short_le((0x0028, 0x0120), *b"US", &0_u16.to_le_bytes())?,
        ));
    }
    build(spec)
}

#[test]
fn value_refuses_padding_above_24_stored_bits() -> Result<(), String> {
    // `Stored` holds an `f32`, whose 24-bit significand makes every integer
    // up to 2^24 exact (IEEE 754 binary32). Above Bits Stored 24 the padding
    // comparison of PS3.3 C.7.5.1.1.2 could see a rounded neighbour, so a
    // frame with Pixel Padding Value and Bits Stored 32 is refused
    // `attribute`. Without padding there is no comparison and the value is
    // 5 * 1 + 0 = 5 (PS3.3 C.11.1). Bits Stored 24 is still exact and is
    // answered with padding present, because stored 5 is not padding 0.
    expect_refusal(
        value_frame(&wide(32, true)?, 0, &[at(0.5, 0.5)]),
        PreviewError::Attribute,
        "32 bits with padding",
    )?;
    let want = five_with("unspecified", "none");
    expect(
        value_frame(&wide(32, false)?, 0, &[at(0.5, 0.5)]),
        &want,
        "32 bits without padding",
    )?;
    expect(
        value_frame(&wide(24, true)?, 0, &[at(0.5, 0.5)]),
        &want,
        "24 bits with padding",
    )
}

#[test]
fn value_ignores_attributes_stage_one_never_reads() -> Result<(), String> {
    // PS3.3 C.11.1. The value is m * SV + b and reads no window, no
    // presentation shape and no lossy or burned-in flag. A one-pixel CT,
    // stored 1064, slope 1, intercept -1024, is 1064 * 1 + (-1024) = 40 in
    // HU (C.8.2.1, no Rescale Type and no Multi-energy CT Acquisition).
    // Each case below is non-conformant in an attribute outside stage 1.
    // The preview, given a window override, still draws the first four.
    let want = "{\"frame\":0,\"unit\":\"HU\",\"unit_source\":\"ct-default\",\"points\":[\
         {\"column\":1,\"row\":1,\"index_column\":0,\"index_row\":0,\"padding\":false,\"value\":40,\"text\":\"40 HU\"}]}";
    let center = text_element((0x0028, 0x1050), *b"DS", "40")?;
    let cases = [
        (
            "Window Width 0",
            vec![center.clone(), text_element((0x0028, 0x1051), *b"DS", "0")?],
        ),
        (
            "Window Width abc",
            vec![center, text_element((0x0028, 0x1051), *b"DS", "abc")?],
        ),
        (
            "Lossy Image Compression 1",
            vec![text_element((0x0028, 0x2110), *b"CS", "1")?],
        ),
        (
            "Burned In Annotation MAYBE",
            vec![text_element((0x0028, 0x0301), *b"CS", "MAYBE")?],
        ),
        (
            "Presentation LUT Shape BOGUS",
            vec![text_element((0x2050, 0x0020), *b"CS", "BOGUS")?],
        ),
    ];
    let mut failed = Vec::new();
    for (name, extra) in cases {
        let mut spec = Spec::mono(1, &[1064]);
        spec.modality = Some("CT");
        spec.rescale = Some(("1", "-1024"));
        spec.extra = extra;
        let file = build(spec)?;
        if let Err(error) = expect(value_frame(&file, 0, &[at(0.5, 0.5)]), want, name) {
            failed.push(error);
        }
        let drawn = preview_frame(
            &file,
            FrameRequest {
                window_center: Some(40.0),
                window_width: Some(400.0),
                ..FrameRequest::default()
            },
        );
        // PS3.3 C.11.2.1.2.1, LINEAR, centre 40, width 400, value 40:
        // ((40 - 39.5) / 399 + 0.5) * 255 = 127.82, nearest grey 128. The
        // Presentation LUT Shape case is refused by the preview, which reads
        // that shape, and not by the value, which does not.
        let drawn = drawn
            .map(|preview| preview.grey)
            .map_err(|error| error.code());
        let shape = name.starts_with("Presentation");
        let as_expected = if shape {
            drawn == Err("window")
        } else {
            drawn == Ok(vec![128])
        };
        if !as_expected {
            failed.push(format!("{name}: the preview gave {drawn:?}"));
        }
    }
    if failed.is_empty() {
        Ok(())
    } else {
        Err(failed.join("\n"))
    }
}

/// Enhanced CT, two frames of one pixel, both stored 1040. The top level says
/// intercept -1024. The per-frame Pixel Value Transformation Sequence says
/// -1024 for frame 0 and -1000 for frame 1.
fn enhanced_rescale() -> Result<Vec<u8>, String> {
    let group = |intercept: &str| -> Result<Vec<u8>, String> {
        let body = [
            short_le((0x0028, 0x1052), *b"DS", &text_pad(intercept))?,
            short_le((0x0028, 0x1053), *b"DS", &text_pad("1"))?,
        ]
        .concat();
        item(&long_le((0x0028, 0x9145), *b"SQ", &item(&body)?)?)
    };
    let per_frame = [group("-1024")?, group("-1000")?].concat();
    let mut spec = Spec::mono(1, &[1040, 1040]);
    spec.frames = Some(2);
    spec.modality = Some("CT");
    spec.rescale = Some(("1", "-1024"));
    spec.extra.push((
        (0x5200, 0x9230),
        long_le((0x5200, 0x9230), *b"SQ", &per_frame)?,
    ));
    build(spec)
}

#[test]
fn value_uses_the_per_frame_rescale() -> Result<(), String> {
    // PS3.3 C.7.6.16.2.9. Frame 1's functional group gives 1040 * 1 + (-1000)
    // = 40. Frame 0's gives 1040 * 1 + (-1024) = 16. The top-level intercept
    // alone would give 16 on both frames.
    let file = enhanced_rescale()?;
    expect(
        value_frame(&file, 1, &[at(0.5, 0.5)]),
        "{\"frame\":1,\"unit\":\"HU\",\"unit_source\":\"ct-default\",\"points\":[\
         {\"column\":1,\"row\":1,\"index_column\":0,\"index_row\":0,\"padding\":false,\"value\":40,\"text\":\"40 HU\"}]}",
        "frame 1",
    )?;
    expect(
        value_frame(&file, 0, &[at(0.5, 0.5)]),
        "{\"frame\":0,\"unit\":\"HU\",\"unit_source\":\"ct-default\",\"points\":[\
         {\"column\":1,\"row\":1,\"index_column\":0,\"index_row\":0,\"padding\":false,\"value\":16,\"text\":\"16 HU\"}]}",
        "frame 0",
    )
}

#[test]
fn value_reports_padding_without_a_number() -> Result<(), String> {
    // PS3.3 C.7.5.1.1.2. Pixel Padding Value -2000 in stored space. Stored
    // -2000 at column 1 is padding and has no modality value. Stored -1000
    // at column 2 is not padding: -1000 * 1 + (-1024) = -2024.
    let padded = i16::to_le_bytes(-2000);
    let kept = i16::to_le_bytes(-1000);
    let mut spec = Spec::mono(2, &[]);
    spec.pixels = [padded, kept].concat();
    spec.bits = (16, 16, 15, 1);
    spec.modality = Some("CT");
    spec.rescale = Some(("1", "-1024"));
    spec.extra.push((
        (0x0028, 0x0120),
        short_le((0x0028, 0x0120), *b"SS", &(-2000_i16).to_le_bytes())?,
    ));
    let file = build(spec)?;
    expect(
        value_frame(&file, 0, &[at(0.5, 0.5), at(1.5, 0.5)]),
        "{\"frame\":0,\"unit\":\"HU\",\"unit_source\":\"ct-default\",\"points\":[\
         {\"column\":1,\"row\":1,\"index_column\":0,\"index_row\":0,\"padding\":true,\"text\":\"padding\"},\
         {\"column\":2,\"row\":1,\"index_column\":1,\"index_row\":0,\"padding\":false,\"value\":-2024,\"text\":\"-2024 HU\"}]}",
        "padding",
    )
}

#[test]
fn value_refuses_a_malformed_padding_range() -> Result<(), String> {
    // PS3.3 C.7.5.1.1.2. Pixel Padding Range Limit (0028,0121) is used only
    // with a Pixel Padding Value (0028,0120), and in MONOCHROME2 the value is
    // the low end of the inclusive range. A limit alone, or value 10 above
    // limit 0, names no range, so the value cannot say which samples are
    // padding and is refused `attribute`. Stored 5 is otherwise 5 (C.11.1).
    let us = |tag: (u16, u16), value: u16| -> Result<((u16, u16), Vec<u8>), String> {
        Ok((tag, short_le(tag, *b"US", &value.to_le_bytes())?))
    };
    let cases = [
        ("a limit without a value", vec![us((0x0028, 0x0121), 10)?]),
        (
            "a reversed MONOCHROME2 range",
            vec![us((0x0028, 0x0120), 10)?, us((0x0028, 0x0121), 0)?],
        ),
    ];
    for (name, extra) in cases {
        let mut spec = Spec::mono(1, &[5]);
        spec.modality = Some("MR");
        spec.rescale = Some(("1", "0"));
        spec.extra = extra;
        expect_refusal(
            value_frame(&build(spec)?, 0, &[at(0.5, 0.5)]),
            PreviewError::Attribute,
            name,
        )?;
    }
    Ok(())
}

#[test]
fn value_refuses_a_value_outside_f32() -> Result<(), String> {
    // PS3.3 C.11.1: 65535 * 1e38 + 0 = 6.5535e42, above the largest finite
    // f32, 3.4028235e38 (IEEE 754 binary32), so the modality value is not a
    // number this route can print. It is refused `attribute`, because the
    // rescale attributes are the cause and no window is involved.
    let mut spec = Spec::mono(1, &[65_535]);
    spec.modality = Some("MR");
    spec.rescale = Some(("1e38", "0"));
    expect_refusal(
        value_frame(&build(spec)?, 0, &[at(0.5, 0.5)]),
        PreviewError::Attribute,
        "slope 1e38",
    )
}

#[test]
fn value_refuses_a_modality_lut_sequence() -> Result<(), String> {
    // PS3.3 C.11.1. A Modality LUT Sequence replaces rescale. This route does
    // not apply one, so it refuses with the preview's `window` code.
    let mut spec = Spec::mono(1, &[50]);
    spec.rescale = Some(("1", "0"));
    spec.extra
        .push(((0x0028, 0x3000), long_le((0x0028, 0x3000), *b"SQ", &[])?));
    let file = build(spec)?;
    expect_refusal(
        value_frame(&file, 0, &[at(0.5, 0.5)]),
        PreviewError::Window,
        "value",
    )?;
    match preview_first_frame(&file) {
        Err(PreviewError::Window) => Ok(()),
        Err(error) => Err(format!("the preview refused with {}", error.code())),
        Ok(_) => Err("the preview drew a Modality LUT Sequence".to_owned()),
    }
}

#[test]
fn value_closed_right_edge_and_outside_refusal() -> Result<(), String> {
    // Columns 2, Rows 1. Cell c covers [c, c + 1). x = 2 is the right edge
    // of the last cell, index 1. Stored 10 and 20, identity rescale.
    let mut spec = Spec::mono(2, &[10, 20]);
    spec.modality = Some("OT");
    let file = build(spec)?;
    expect(
        value_frame(&file, 0, &[at(2.0, 1.0)]),
        "{\"frame\":0,\"unit\":\"unspecified\",\"unit_source\":\"none\",\"points\":[\
         {\"column\":2,\"row\":1,\"index_column\":1,\"index_row\":0,\"padding\":false,\"value\":20,\"text\":\"20 unspecified\"}]}",
        "right edge",
    )?;
    expect(
        value_frame(&file, 0, &[at(0.0, 0.0)]),
        "{\"frame\":0,\"unit\":\"unspecified\",\"unit_source\":\"none\",\"points\":[\
         {\"column\":1,\"row\":1,\"index_column\":0,\"index_row\":0,\"padding\":false,\"value\":10,\"text\":\"10 unspecified\"}]}",
        "left edge",
    )?;
    for (name, point) in [
        ("past the right edge", at(2.01, 0.5)),
        ("left of the image", at(-0.01, 0.5)),
        ("below the image", at(0.5, 1.01)),
        ("above the image", at(0.5, -0.01)),
    ] {
        // A refusal returns no partial list, so a good point beside it fails too.
        expect_refusal(
            value_frame(&file, 0, &[at(0.5, 0.5), point]),
            PreviewError::Point,
            name,
        )?;
    }
    if PreviewError::Point.code() != "point"
        || PreviewError::Point.to_string() != "the point is outside this image"
    {
        return Err("the point refusal has the wrong code or sentence".to_owned());
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// The HTTP route.

fn exchange(addr: SocketAddr, request: &[u8]) -> Result<Vec<u8>, String> {
    let mut stream = TcpStream::connect(addr).map_err(|error| error.to_string())?;
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .map_err(|error| error.to_string())?;
    stream
        .write_all(request)
        .map_err(|error| error.to_string())?;
    stream
        .shutdown(Shutdown::Write)
        .map_err(|error| error.to_string())?;
    let mut response = Vec::new();
    stream
        .read_to_end(&mut response)
        .map_err(|error| error.to_string())?;
    Ok(response)
}

fn split_http(response: &[u8]) -> Result<(String, String), String> {
    let end = response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|start| start + 4)
        .ok_or_else(|| "the response has no header".to_owned())?;
    let header = response
        .get(..end)
        .ok_or_else(|| "the response header is short".to_owned())?;
    let body = response
        .get(end..)
        .ok_or_else(|| "the response body is short".to_owned())?;
    Ok((
        String::from_utf8_lossy(header).into_owned(),
        String::from_utf8_lossy(body).into_owned(),
    ))
}

fn header_line(header: &str, name: &str) -> Option<String> {
    header.split("\r\n").find_map(|line| {
        let (key, value) = line.split_once(':')?;
        if key.eq_ignore_ascii_case(name) {
            Some(value.trim().to_owned())
        } else {
            None
        }
    })
}

fn start(viewer: Viewer) -> Result<SocketAddr, String> {
    let listener = TcpListener::bind("127.0.0.1:0").map_err(|error| error.to_string())?;
    let addr = listener.local_addr().map_err(|error| error.to_string())?;
    let viewer = Arc::new(viewer);
    std::thread::spawn(move || {
        let _ = serve_on(listener, viewer);
    });
    Ok(addr)
}

/// The direct viewer and the process server, which decodes on its workers.
fn both_viewers() -> Result<[(&'static str, SocketAddr); 2], String> {
    let config = || ViewerConfig {
        cache_bytes: 8 * 1024 * 1024,
    };
    Ok([
        ("direct", start(Viewer::new(config()))?),
        ("server", start(Viewer::server(config()))?),
    ])
}

fn post_file(addr: SocketAddr, file: &[u8]) -> Result<String, String> {
    let mut request = format!(
        "POST /file HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: {len}\r\n\r\n",
        len = file.len()
    )
    .into_bytes();
    request.extend_from_slice(file);
    let (header, _) = split_http(&exchange(addr, &request)?)?;
    header_line(&header, "X-Ocelli-File").ok_or_else(|| format!("no file id in {header}"))
}

fn value_request(id: &str, extra: &str) -> Vec<u8> {
    format!(
        "POST /value HTTP/1.1\r\nHost: 127.0.0.1\r\nX-Ocelli-File: {id}\r\n{extra}Content-Length: 0\r\n\r\n"
    )
    .into_bytes()
}

fn ask(addr: SocketAddr, id: &str, extra: &str) -> Result<(String, String), String> {
    split_http(&exchange(addr, &value_request(id, extra))?)
}

fn refused_with(reply: &(String, String), status: &str, code: &str) -> bool {
    reply.0.starts_with(&format!("HTTP/1.1 {status} "))
        && header_line(&reply.0, "X-Ocelli-Error").as_deref() == Some(code)
}

#[test]
fn value_route_reads_the_synthetic_named_pixels() -> Result<(), String> {
    // Synthetic slice 0: stored is HU + 1024 and the intercept is -1024, so
    // PS3.3 C.11.1 gives the HU back. Air -1000 at (0, 0). The body ellipse
    // centre (64, 48) is 40. (46, 48) is inside the left lung, -800. (72, 48)
    // is 8 from the centre, 64 squared, inside the 36 to 144 ring, 700.
    let file = slice(0).ok_or_else(|| "no synthetic slice".to_owned())?;
    let want = "{\"frame\":0,\"unit\":\"HU\",\"unit_source\":\"ct-default\",\"points\":[\
        {\"column\":1,\"row\":1,\"index_column\":0,\"index_row\":0,\"padding\":false,\"value\":-1000,\"text\":\"-1000 HU\"},\
        {\"column\":65,\"row\":49,\"index_column\":64,\"index_row\":48,\"padding\":false,\"value\":40,\"text\":\"40 HU\"},\
        {\"column\":47,\"row\":49,\"index_column\":46,\"index_row\":48,\"padding\":false,\"value\":-800,\"text\":\"-800 HU\"},\
        {\"column\":73,\"row\":49,\"index_column\":72,\"index_row\":48,\"padding\":false,\"value\":700,\"text\":\"700 HU\"}]}";
    for (name, addr) in both_viewers()? {
        let id = post_file(addr, &file)?;
        let reply = ask(
            addr,
            &id,
            "X-Ocelli-Frame: 0\r\nX-Ocelli-Points: 0.5,0.5 64.5,48.5 46.5,48.5 72.5,48.5\r\n",
        )?;
        if !reply.0.starts_with("HTTP/1.1 200 ") || reply.1 != want {
            return Err(format!("{name}: {} {}", reply.0, reply.1));
        }
    }
    Ok(())
}

#[test]
fn value_points_header_is_bounded() -> Result<(), String> {
    let file = slice(0).ok_or_else(|| "no synthetic slice".to_owned())?;
    let many = vec!["1,1"; 65].join(" ");
    let most = vec!["1,1"; 64].join(" ");
    for (name, addr) in both_viewers()? {
        let id = post_file(addr, &file)?;
        let cases = [
            ("65 pairs", format!("X-Ocelli-Points: {many}\r\n")),
            ("an empty list", "X-Ocelli-Points: \r\n".to_owned()),
            ("no header", String::new()),
            ("a half pair", "X-Ocelli-Points: 1,\r\n".to_owned()),
            ("one number", "X-Ocelli-Points: 1\r\n".to_owned()),
            ("three numbers", "X-Ocelli-Points: 1,2,3\r\n".to_owned()),
            ("infinity", "X-Ocelli-Points: inf,1\r\n".to_owned()),
            ("not a number", "X-Ocelli-Points: NaN,1\r\n".to_owned()),
            ("a word", "X-Ocelli-Points: a,1\r\n".to_owned()),
            (
                "two headers",
                "X-Ocelli-Points: 1,1\r\nX-Ocelli-Points: 2,2\r\n".to_owned(),
            ),
        ];
        for (case, extra) in cases {
            let reply = ask(addr, &id, &extra)?;
            if !refused_with(&reply, "400", "points") {
                return Err(format!("{name} {case}: {}", reply.0));
            }
        }
        let reply = ask(addr, &id, &format!("X-Ocelli-Points: {most}\r\n"))?;
        if !reply.0.starts_with("HTTP/1.1 200 ")
            || reply.1.matches("\"index_column\"").count() != 64
        {
            return Err(format!("{name}: 64 pairs were not answered: {}", reply.0));
        }
        // The header is ignored on every other route.
        let preview = split_http(&exchange(
            addr,
            &format!(
                "POST /preview HTTP/1.1\r\nHost: 127.0.0.1\r\nX-Ocelli-File: {id}\r\nX-Ocelli-Points: a,\r\nContent-Length: 0\r\n\r\n"
            )
            .into_bytes(),
        )?)?;
        if !preview.0.starts_with("HTTP/1.1 200 ") {
            return Err(format!(
                "{name}: a preview read the points header: {}",
                preview.0
            ));
        }
    }
    Ok(())
}

#[test]
fn value_ignores_window_headers() -> Result<(), String> {
    let file = slice(0).ok_or_else(|| "no synthetic slice".to_owned())?;
    for (name, addr) in both_viewers()? {
        let id = post_file(addr, &file)?;
        let plain = ask(addr, &id, "X-Ocelli-Points: 64.5,48.5\r\n")?;
        let windowed = ask(
            addr,
            &id,
            "X-Ocelli-Points: 64.5,48.5\r\nX-Ocelli-Window-Center: 500\r\nX-Ocelli-Window-Width: 10\r\nX-Ocelli-Window-Function: SIGMOID\r\nX-Ocelli-Invert: 1\r\n",
        )?;
        let preset = ask(
            addr,
            &id,
            "X-Ocelli-Points: 64.5,48.5\r\nX-Ocelli-Window-Preset: bone\r\n",
        )?;
        if !plain.1.contains("\"value\":40,\"text\":\"40 HU\"")
            || windowed.1 != plain.1
            || preset.1 != plain.1
        {
            return Err(format!(
                "{name}: a window changed the value: {} {} {}",
                plain.1, windowed.1, preset.1
            ));
        }
    }
    Ok(())
}

#[test]
fn value_route_takes_only_a_held_id() -> Result<(), String> {
    let file = slice(0).ok_or_else(|| "no synthetic slice".to_owned())?;
    let viewers = both_viewers()?;
    for (name, addr) in viewers {
        let id = post_file(addr, &file)?;
        let mut no_id =
            b"POST /value HTTP/1.1\r\nHost: 127.0.0.1\r\nX-Ocelli-Points: 1,1\r\n".to_vec();
        no_id.extend_from_slice(format!("Content-Length: {}\r\n\r\n", file.len()).as_bytes());
        no_id.extend_from_slice(&file);
        let reply = split_http(&exchange(addr, &no_id)?)?;
        if !refused_with(&reply, "400", "header") {
            return Err(format!(
                "{name}: a body without an id was not refused: {}",
                reply.0
            ));
        }
        let bare = split_http(&exchange(
            addr,
            b"POST /value HTTP/1.1\r\nHost: 127.0.0.1\r\nX-Ocelli-Points: 1,1\r\nContent-Length: 0\r\n\r\n",
        )?)?;
        if !refused_with(&bare, "400", "header") {
            return Err(format!(
                "{name}: a request without an id was not refused: {}",
                bare.0
            ));
        }
        let mut with_body = format!(
            "POST /value HTTP/1.1\r\nHost: 127.0.0.1\r\nX-Ocelli-File: {id}\r\nX-Ocelli-Points: 1,1\r\nContent-Length: 2\r\n\r\n"
        )
        .into_bytes();
        with_body.extend_from_slice(b"ab");
        let reply = split_http(&exchange(addr, &with_body)?)?;
        if !refused_with(&reply, "400", "header") {
            return Err(format!(
                "{name}: an id with a body was not refused: {}",
                reply.0
            ));
        }
    }
    // An id from another viewer.
    let id = post_file(viewers[0].1, &file)?;
    let reply = ask(viewers[1].1, &id, "X-Ocelli-Points: 1,1\r\n")?;
    if !refused_with(&reply, "410", "file-gone") {
        return Err(format!("another viewer's id was served: {}", reply.0));
    }
    // An evicted id. The budget holds one decoded synthetic frame at most,
    // so posting more files pushes the first one out.
    let small = start(Viewer::new(ViewerConfig {
        cache_bytes: file.len() + 1024,
    }))?;
    let first = post_file(small, &file)?;
    for _ in 0..3 {
        let _ = post_file(small, &file)?;
    }
    let reply = ask(small, &first, "X-Ocelli-Points: 1,1\r\n")?;
    if !refused_with(&reply, "410", "file-gone") {
        return Err(format!("an evicted id was served: {}", reply.0));
    }
    Ok(())
}

#[test]
fn value_on_a_held_frame_does_not_decode_again() -> Result<(), String> {
    let file = slice(3).ok_or_else(|| "no synthetic slice".to_owned())?;
    for (name, addr) in both_viewers()? {
        let id = post_file(addr, &file)?;
        let first = ask(addr, &id, "X-Ocelli-Points: 64.5,48.5\r\n")?;
        let second = ask(addr, &id, "X-Ocelli-Points: 64.5,48.5\r\n")?;
        let preview = split_http(&exchange(
            addr,
            &format!(
                "POST /preview HTTP/1.1\r\nHost: 127.0.0.1\r\nX-Ocelli-File: {id}\r\nContent-Length: 0\r\n\r\n"
            )
            .into_bytes(),
        )?)?;
        let sources = [
            header_line(&first.0, "X-Ocelli-Source"),
            header_line(&second.0, "X-Ocelli-Source"),
            header_line(&preview.0, "X-Ocelli-Source"),
        ];
        if sources
            != [
                Some("decoded".to_owned()),
                Some("held".to_owned()),
                Some("held".to_owned()),
            ]
        {
            return Err(format!("{name}: sources were {sources:?}"));
        }
        // A duplicate request for the same cell returns the same reply.
        if first.1 != second.1 {
            return Err(format!("{name}: a repeated value differed"));
        }
    }
    Ok(())
}
