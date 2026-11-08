//! Colour frames through the folder viewer's colour stage. This is the
//! bounded additive test slice for F-239. Every expected value is
//! hand-computed from the approved `.claude/plans/F-239-design.md`
//! acceptance rows and the DICOM sections each test names, never from a
//! previous run of this route.
//!
//! The fixtures are synthetic Part 10 objects built here. No binary DICOM
//! is added to the repository. The one encoded input is the existing
//! tracked JPEG baseline fixture, read from the codec crate's fixture
//! directory the way `tests/preview.rs` reads its JPEG XL fixtures.
//!
//! Assertions use the pre-existing public surface only: `preview.grey`
//! bytes, the existing window fields, JSON substrings, and error code
//! tokens. No new `Preview` field is assumed.

use std::io::{Read, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
use std::sync::Arc;
use std::time::Duration;

use ocelli_codec::{
    FrameDesc, FrameDescInput, PixelDataVr, PixelRepresentation, Registry, register_jpeg_decoders,
};
use ocelli_folder_viewer::{
    FrameRequest, ImagePoint, MeasureShape, Preview, PreviewError, Viewer, ViewerConfig,
    inspect_json, measure_frame, preview_frame, serve_on, slice, value_frame,
};
use ocelli_pixel::VoiFunction;
use ocelli_viewport::presentation::WindowPreset;

// ---------------------------------------------------------------------------
// Builders, adapted from the `Spec` pattern of tests/value.rs. Elements are
// sorted by tag before they are written.

const SOP_SECONDARY_CAPTURE: &[u8] = b"1.2.840.10008.5.1.4.1.1.7";
const NATIVE_EXPLICIT_LE: &str = "1.2.840.10008.1.2.1";
const JPEG_BASELINE: &str = "1.2.840.10008.1.2.4.50";

type TaggedElement = ((u16, u16), Vec<u8>);

/// One synthetic frame description, cloned freely in the builders below.
#[derive(Clone)]
struct Spec {
    rows: u16,
    columns: u16,
    frames: Option<u16>,
    samples: u16,
    photometric: &'static str,
    /// Planar Configuration, omitted when `None`.
    planar: Option<u16>,
    /// (Bits Allocated, Bits Stored, High Bit, Pixel Representation).
    bits: (u16, u16, u16, u16),
    modality: Option<&'static str>,
    /// (Rescale Slope, Rescale Intercept), written as DS noise.
    rescale: Option<(&'static str, &'static str)>,
    /// (Window Center, Window Width), written as DS noise.
    window: Option<(&'static str, &'static str)>,
    /// (row spacing, column spacing) of Pixel Spacing.
    spacing: Option<(&'static str, &'static str)>,
    extra: Vec<Result<TaggedElement, String>>,
    pixels: Vec<u8>,
    /// The Pixel Data VR: OB for 8-bit native frames, OW otherwise.
    pixel_vr: [u8; 2],
}

impl Spec {
    /// An 8-bit native colour frame, one unsigned byte per stored sample.
    /// PS3.3 C.7.6.3.1.3 makes Planar Configuration required for three
    /// samples, so the constructors default it to interleaved and the
    /// planar cases override it.
    fn colour8(
        rows: u16,
        columns: u16,
        samples: u16,
        photometric: &'static str,
        pixels: &[u8],
    ) -> Self {
        Self {
            rows,
            columns,
            frames: None,
            samples,
            photometric,
            planar: (samples > 1).then_some(0),
            bits: (8, 8, 7, 0),
            modality: None,
            rescale: None,
            window: None,
            spacing: None,
            extra: Vec::new(),
            pixels: pixels.to_vec(),
            pixel_vr: *b"OB",
        }
    }

    /// A 16-bit container colour frame with an explicit stored shape.
    fn colour16(
        rows: u16,
        columns: u16,
        samples: u16,
        photometric: &'static str,
        bits: (u16, u16, u16, u16),
        words: &[u16],
    ) -> Self {
        let mut pixels = Vec::new();
        for value in words {
            pixels.extend_from_slice(&value.to_le_bytes());
        }
        Self {
            rows,
            columns,
            frames: None,
            samples,
            photometric,
            planar: (samples > 1).then_some(0),
            bits,
            modality: None,
            rescale: None,
            window: None,
            spacing: None,
            extra: Vec::new(),
            pixels,
            pixel_vr: *b"OW",
        }
    }

    /// A 32-bit container colour frame with an explicit stored shape.
    fn colour32(
        rows: u16,
        columns: u16,
        samples: u16,
        photometric: &'static str,
        bits: (u16, u16, u16, u16),
        words: &[u32],
    ) -> Self {
        let mut pixels = Vec::new();
        for value in words {
            pixels.extend_from_slice(&value.to_le_bytes());
        }
        Self {
            rows,
            columns,
            frames: None,
            samples,
            photometric,
            planar: (samples > 1).then_some(0),
            bits,
            modality: None,
            rescale: None,
            window: None,
            spacing: None,
            extra: Vec::new(),
            pixels,
            pixel_vr: *b"OW",
        }
    }

    fn with_planar(mut self, planar: u16) -> Self {
        self.planar = Some(planar);
        self
    }

    fn with_modality(mut self, modality: &'static str) -> Self {
        self.modality = Some(modality);
        self
    }

    fn with_rescale(mut self, slope: &'static str, intercept: &'static str) -> Self {
        self.rescale = Some((slope, intercept));
        self
    }

    fn with_window(mut self, center: &'static str, width: &'static str) -> Self {
        self.window = Some((center, width));
        self
    }

    fn with_spacing(mut self, row: &'static str, column: &'static str) -> Self {
        self.spacing = Some((row, column));
        self
    }

    fn with_bits(mut self, bits: (u16, u16, u16, u16)) -> Self {
        self.bits = bits;
        self
    }

    fn with_frames(mut self, frames: u16) -> Self {
        self.frames = Some(frames);
        self
    }

    /// One extra element whose value is padded already. The element is
    /// written short or long form by its VR, as PS3.5 7.1 asks.
    fn with_element(mut self, tag: (u16, u16), vr: [u8; 2], value: &[u8]) -> Self {
        let element = if vr == *b"OB" || vr == *b"OW" {
            long_le(tag, vr, value)
        } else {
            short_le(tag, vr, value)
        };
        self.extra.push(element.map(|element| (tag, element)));
        self
    }
}

fn build(spec: &Spec) -> Result<Vec<u8>, String> {
    let (mut elements, pixels) = dataset_and_pixels(spec)?;
    let tag = (0x7FE0, 0x0010);
    elements.push((tag, long_le(tag, spec.pixel_vr, &pixels)?));
    finish_native(elements)
}

/// Every element of the data set except Pixel Data, and the Pixel Data
/// value padded to even length as PS3.5 8.1.1 asks.
fn dataset_and_pixels(spec: &Spec) -> Result<(Vec<TaggedElement>, Vec<u8>), String> {
    let mut elements: Vec<TaggedElement> = Vec::new();
    elements.push((
        (0x0008, 0x0016),
        short_le((0x0008, 0x0016), *b"UI", &ui(SOP_SECONDARY_CAPTURE))?,
    ));
    let modality = spec.modality.unwrap_or("OT");
    elements.push((
        (0x0008, 0x0060),
        short_le((0x0008, 0x0060), *b"CS", &text_pad(modality))?,
    ));
    elements.push((
        (0x0028, 0x0002),
        short_le((0x0028, 0x0002), *b"US", &spec.samples.to_le_bytes())?,
    ));
    elements.push((
        (0x0028, 0x0004),
        short_le((0x0028, 0x0004), *b"CS", &text_pad(spec.photometric))?,
    ));
    if let Some(planar) = spec.planar {
        elements.push((
            (0x0028, 0x0006),
            short_le((0x0028, 0x0006), *b"US", &planar.to_le_bytes())?,
        ));
    }
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
    if let Some((row, column)) = spec.spacing {
        elements.push((
            (0x0028, 0x0030),
            short_le(
                (0x0028, 0x0030),
                *b"DS",
                &text_pad(&format!("{row}\\{column}")),
            )?,
        ));
    }
    let (allocated, stored, high, representation) = spec.bits;
    for (tag, value) in [
        ((0x0028, 0x0100), allocated),
        ((0x0028, 0x0101), stored),
        ((0x0028, 0x0102), high),
        ((0x0028, 0x0103), representation),
    ] {
        elements.push((tag, short_le(tag, *b"US", &value.to_le_bytes())?));
    }
    if let Some((center, width)) = spec.window {
        elements.push((
            (0x0028, 0x1050),
            short_le((0x0028, 0x1050), *b"DS", &text_pad(center))?,
        ));
        elements.push((
            (0x0028, 0x1051),
            short_le((0x0028, 0x1051), *b"DS", &text_pad(width))?,
        ));
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
    for element in &spec.extra {
        elements.push(element.clone()?);
    }
    let mut pixels = spec.pixels.clone();
    if !pixels.len().is_multiple_of(2) {
        pixels.push(0);
    }
    Ok((elements, pixels))
}

/// One encapsulated frame around `codestream`, in the wrapper shape
/// `tests/preview.rs` uses for its JPEG XL fixtures: Pixel Data OB of
/// undefined length, a zero-length Basic Offset Table, one fragment, and
/// the Sequence Delimitation Item (PS3.5 8.2).
fn encapsulated(spec: &Spec, transfer_syntax: &str, codestream: &[u8]) -> Result<Vec<u8>, String> {
    let (dataset, _) = dataset_and_pixels(spec)?;
    let mut fragment = codestream.to_vec();
    if fragment.len() % 2 == 1 {
        fragment.push(0);
    }
    let mut pixels = item(&0_u32.to_le_bytes())?;
    pixels.extend_from_slice(&item(&fragment)?);
    pixels.extend_from_slice(&0xFFFE_u16.to_le_bytes());
    pixels.extend_from_slice(&0xE0DD_u16.to_le_bytes());
    pixels.extend_from_slice(&0_u32.to_le_bytes());
    let mut element = Vec::new();
    element.extend_from_slice(&0x7FE0_u16.to_le_bytes());
    element.extend_from_slice(&0x0010_u16.to_le_bytes());
    element.extend_from_slice(b"OB");
    element.extend_from_slice(&[0, 0]);
    element.extend_from_slice(&0xFFFF_FFFF_u32.to_le_bytes());
    element.extend_from_slice(&pixels);
    let mut bytes = vec![0_u8; 128];
    bytes.extend_from_slice(b"DICM");
    bytes.extend(file_meta(transfer_syntax)?);
    for (_, dataset_element) in dataset {
        bytes.extend(dataset_element);
    }
    bytes.extend(element);
    Ok(bytes)
}

fn finish_native(mut elements: Vec<((u16, u16), Vec<u8>)>) -> Result<Vec<u8>, String> {
    elements.sort_by_key(|(tag, _)| *tag);
    let mut bytes = vec![0_u8; 128];
    bytes.extend_from_slice(b"DICM");
    bytes.extend(file_meta(NATIVE_EXPLICIT_LE)?);
    for (_, element) in elements {
        bytes.extend(element);
    }
    Ok(bytes)
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

fn file_meta(transfer_syntax: &str) -> Result<Vec<u8>, String> {
    // PS3.10 7.1: the file meta group is always Explicit VR Little Endian.
    let mut body = Vec::new();
    body.extend(long_le((0x0002, 0x0001), *b"OB", &[0, 1])?);
    body.extend(short_le(
        (0x0002, 0x0002),
        *b"UI",
        &ui(SOP_SECONDARY_CAPTURE),
    )?);
    body.extend(short_le((0x0002, 0x0003), *b"UI", &ui(b"2.25.1"))?);
    body.extend(short_le(
        (0x0002, 0x0010),
        *b"UI",
        &ui(transfer_syntax.as_bytes()),
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

/// The three words of one Palette Colour Lookup Table Descriptor
/// (0028,110x), US: entries, first stored value mapped, bits per entry.
fn lut_descriptor(entries: u16, first_mapped: u16, bits: u16) -> Vec<u8> {
    [entries, first_mapped, bits]
        .iter()
        .flat_map(|value| value.to_le_bytes())
        .collect()
}

/// The entries of one Palette Colour Lookup Table Data (0028,120x) as
/// 16-bit words, the OW form PS3.3 C.7.6.3.1.5 gives a descriptor above
/// 8 bits.
fn lut_words(entries: &[u16]) -> Vec<u8> {
    entries
        .iter()
        .flat_map(|entry| entry.to_le_bytes())
        .collect()
}

fn at(x: f64, y: f64) -> ImagePoint {
    ImagePoint { x, y }
}

/// One preview named for its failure message.
fn draw(file: &[u8], request: FrameRequest, name: &str) -> Result<Preview, String> {
    preview_frame(file, request).map_err(|error| format!("{name}: refused with {}", error.code()))
}

// ---------------------------------------------------------------------------
// Expectation helpers. Every refusal is asserted by its code token, which is
// the stable wire name, so the slice survives enum movement around it.

fn expect_bytes(got: Result<Preview, PreviewError>, want: &[u8], name: &str) -> Result<(), String> {
    let preview = got.map_err(|error| format!("{name}: refused with {}", error.code()))?;
    if preview.grey == want {
        Ok(())
    } else {
        Err(format!("{name}: got {:?}, wanted {:?}", preview.grey, want))
    }
}

fn expect_code(got: Result<Preview, PreviewError>, want: &str, name: &str) -> Result<(), String> {
    match got {
        Ok(preview) => Err(format!("{name}: answered {} bytes", preview.grey.len())),
        Err(error) if error.code() == want => Ok(()),
        Err(error) => Err(format!(
            "{name}: refused with {}, wanted {want}",
            error.code()
        )),
    }
}

fn expect_value_code(
    got: Result<String, PreviewError>,
    want: &str,
    name: &str,
) -> Result<(), String> {
    match got {
        Ok(json) => Err(format!("{name}: answered {json}")),
        Err(error) if error.code() == want => Ok(()),
        Err(error) => Err(format!(
            "{name}: refused with {}, wanted {want}",
            error.code()
        )),
    }
}

/// The reply of a frame drawn at the initial colour window: the contract's
/// center 128, width 256, LINEAR, no invert, no full range, no file window
/// index and window count 0.
fn expect_initial(
    got: Result<Preview, PreviewError>,
    want: &[u8],
    name: &str,
) -> Result<(), String> {
    let preview = got.map_err(|error| format!("{name}: refused with {}", error.code()))?;
    if preview.grey != want {
        return Err(format!("{name}: got {:?}, wanted {:?}", preview.grey, want));
    }
    if preview.window_center.to_bits() != 128.0_f32.to_bits()
        || preview.window_width.to_bits() != 256.0_f32.to_bits()
    {
        return Err(format!(
            "{name}: window {} by {}, wanted the initial 128 by 256",
            preview.window_center, preview.window_width
        ));
    }
    if preview.window_function != "LINEAR" {
        return Err(format!(
            "{name}: window function {}, wanted LINEAR",
            preview.window_function
        ));
    }
    if preview.inverted {
        return Err(format!("{name}: the initial window is not inverted"));
    }
    if preview.auto != ocelli_folder_viewer::AutoSource::None {
        return Err(format!("{name}: a colour frame has no automatic window"));
    }
    if preview.window_index.is_some() || preview.window_count != 0 {
        return Err(format!(
            "{name}: file window index {:?} and count {}, wanted none and 0",
            preview.window_index, preview.window_count
        ));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// 1. Native RGB, interleaved and planar. PS3.3 C.7.6.3.1.3: Planar
//    Configuration 0 is R G B interleaved, 1 is RRR GGG BBB by plane.

#[test]
fn native_interleaved_rgb_is_copied_per_pixel() -> Result<(), String> {
    // AC1. Native RGB 2 by 1, Planar Configuration 0, stored
    // `10 20 30 200 100 50`. The display bytes are the samples.
    let spec = Spec::colour8(1, 2, 3, "RGB", &[10, 20, 30, 200, 100, 50]).with_planar(0);
    expect_initial(
        preview_frame(&build(&spec)?, FrameRequest::default()),
        &[10, 20, 30, 200, 100, 50],
        "interleaved RGB",
    )
}

#[test]
fn native_planar_rgb_is_gathered_by_plane() -> Result<(), String> {
    // AC2. Native RGB 2 by 1, Planar Configuration 1, stored
    // `10 200 20 100 30 50`. Reading planar as interleaved would answer
    // `10 200 20 100 30 50`, which is not the reply.
    let spec = Spec::colour8(1, 2, 3, "RGB", &[10, 200, 20, 100, 30, 50]).with_planar(1);
    expect_initial(
        preview_frame(&build(&spec)?, FrameRequest::default()),
        &[10, 20, 30, 200, 100, 50],
        "planar RGB",
    )
}

#[test]
fn two_row_planar_rgb_rejects_plane_row_gathering_mistakes() -> Result<(), String> {
    // PS3.3 C.7.6.3.1.3. Two rows make each plane's row boundaries
    // observable. The planes are R `10 20 30 40 50 60`, G
    // `100 110 120 130 140 150` and B `1 2 3 4 5 6`. Row 0 is
    // `10 100 1`, `20 110 2`, `30 120 3` and row 1 is `40 130 4`,
    // `50 140 5`, `60 150 6`. Gathering both output rows from plane row 0,
    // or reading the data as interleaved, gives some other order.
    let mut pixels = Vec::new();
    pixels.extend_from_slice(&[10, 20, 30, 40, 50, 60]);
    pixels.extend_from_slice(&[100, 110, 120, 130, 140, 150]);
    pixels.extend_from_slice(&[1, 2, 3, 4, 5, 6]);
    let spec = Spec::colour8(2, 3, 3, "RGB", &pixels).with_planar(1);
    expect_initial(
        preview_frame(&build(&spec)?, FrameRequest::default()),
        &[
            10, 100, 1, 20, 110, 2, 30, 120, 3, 40, 130, 4, 50, 140, 5, 60, 150, 6,
        ],
        "two row planar RGB",
    )
}

#[test]
fn inspect_marks_three_rgb_frames_colour_and_not_monochrome() -> Result<(), String> {
    // AC14. The folder scan of a native RGB file of 3 frames lists every
    // frame: `colour` true, `monochrome` false and `frames` 3. A colour
    // cine listed as one image is the defect this rejects.
    let pixels: Vec<u8> = (0..3_u8)
        .flat_map(|frame| [10 + frame, 20, 30, 200, 100, 50])
        .collect();
    let spec = Spec::colour8(1, 2, 3, "RGB", &pixels)
        .with_planar(0)
        .with_frames(3);
    let json = inspect_json(&build(&spec)?, 0)
        .map_err(|error| format!("inspect: refused with {}", error.code()))?;
    for want in ["\"colour\":true", "\"monochrome\":false", "\"frames\":3"] {
        if !json.contains(want) {
            return Err(format!("inspect: {json} has no {want}"));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// 2. YBR_FULL, PS3.3 C.7.6.3.1.2, and the value readout at the pointer.

#[test]
fn ybr_full_uses_the_full_range_inverse() -> Result<(), String> {
    // AC3. Native YBR_FULL 1 by 1, `Y Cb Cr` `100 150 90`. Cb' 22 and
    // Cr' -38 give channels `46.72366 119.56544 138.98863`, which the tier
    // C byte rule rounds to `47 120 139`. The second row, `30 200 60`,
    // gives channels `-65.3378 53.7829 157.59158` and reply `0 54 158`,
    // so a wrapping store, which would keep -65 as 191, is rejected.
    let first = Spec::colour8(1, 1, 3, "YBR_FULL", &[100, 150, 90]);
    expect_bytes(
        preview_frame(&build(&first)?, FrameRequest::default()),
        &[47, 120, 139],
        "YBR_FULL 100 150 90",
    )?;
    let second = Spec::colour8(1, 1, 3, "YBR_FULL", &[30, 200, 60]);
    expect_bytes(
        preview_frame(&build(&second)?, FrameRequest::default()),
        &[0, 54, 158],
        "YBR_FULL 30 200 60",
    )
}

#[test]
fn ybr_full_value_readout_names_the_rgb_channels() -> Result<(), String> {
    // AC13 and the plan's protocol table. The pointer readout on a colour
    // frame is the R, G and B stage 4 output scaled and before the window,
    // printed to one decimal with a space after each label. A three-sample
    // frame has no scalar modality value (D-23), so there is no `value`,
    // no `modality` and no `unit_source`.
    let spec = Spec::colour8(1, 1, 3, "YBR_FULL", &[100, 150, 90]);
    let json = value_frame(&build(&spec)?, 0, &[at(0.5, 0.5)])
        .map_err(|error| format!("value: refused with {}", error.code()))?;
    for want in [
        "\"unit\":\"RGB\"",
        "\"text\":\"R 46.7 G 119.6 B 139.0\"",
        "\"padding\":false",
    ] {
        if !json.contains(want) {
            return Err(format!("value: {json} has no {want}"));
        }
    }
    // Lead ruling, 2026-11-08. Parse the one requested point's RGB
    // array and compare the AC13 hand values using the existing approved
    // PS3.3 YBR fixture tolerance in crates/ocelli-pixel/tests/color.rs.
    // Exact shortest-f32 spellings observed from this implementation are
    // not independent expected values. The one-decimal text stays exact.
    const YBR_TOLERANCE: f32 = 0.001;
    if json.matches("\"rgb\":[").count() != 1 || !json.ends_with("}]}") {
        return Err(format!("expected one closed RGB point: {json}"));
    }
    let (_, rgb) = json.split_once("\"rgb\":[").ok_or("no RGB array")?;
    let (rgb, _) = rgb.split_once(']').ok_or("RGB array does not close")?;
    let parsed = rgb
        .split(',')
        .map(|value| value.trim().parse::<f32>())
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("RGB is not numeric: {error}"))?;
    let actual: [f32; 3] = parsed
        .try_into()
        .map_err(|_| "RGB must have exactly three channels")?;
    for (actual, expected) in actual.into_iter().zip([46.72366_f32, 119.56544, 138.98863]) {
        if !actual.is_finite() || (actual - expected).abs() >= YBR_TOLERANCE {
            return Err(format!(
                "RGB channel was {actual}, expected hand value {expected}"
            ));
        }
    }
    for absent in ["\"value\"", "modality", "unit_source"] {
        if json.contains(absent) {
            return Err(format!("value: {json} carries {absent}"));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// 3. Native 4:2:2, PS3.3 C.7.6.3.1.2: stored Y1 Y2 Cb Cr, both pixels of a
//    pair sharing the one chroma sample, replicated without interpolation.

#[test]
fn native_ybr_full_422_shares_one_chroma_pair() -> Result<(), String> {
    // AC5. Native YBR_FULL_422 2 by 1 stored `Y1 Y2 Cb Cr` `50 200 100
    // 160` in a four byte Pixel Data. Cb' -28 and Cr' 32 give pixel 0
    // channels `94.86463 36.78385 0.38031`, reply `95 37 0`, and pixel 1
    // adds 150 to each channel, reply `245 187 150`. Sizing the value from
    // Samples per Pixel would refuse the four byte frame.
    let spec = Spec::colour8(1, 2, 3, "YBR_FULL_422", &[50, 200, 100, 160]);
    expect_bytes(
        preview_frame(&build(&spec)?, FrameRequest::default()),
        &[95, 37, 0, 245, 187, 150],
        "YBR_FULL_422 pair",
    )
}

#[test]
fn ybr_partial_422_applies_the_studio_offset() -> Result<(), String> {
    // AC6. Native YBR_PARTIAL_422 2 by 1 stored `16 235 128 128`. Y' is
    // Y - 16, so Y 16 gives 0 and Y 235 gives `1.1644155 * 219 = 255.007`,
    // clamped to 255. A full range matrix would give
    // `16 16 16 235 235 235`.
    let spec = Spec::colour8(1, 2, 3, "YBR_PARTIAL_422", &[16, 235, 128, 128]);
    expect_bytes(
        preview_frame(&build(&spec)?, FrameRequest::default()),
        &[0, 0, 0, 255, 255, 255],
        "YBR_PARTIAL_422",
    )
}

#[test]
fn odd_native_422_columns_are_refused_by_name() -> Result<(), String> {
    // AC5. A pixel pair with no partner has no rule PS3.3 names, so an odd
    // Columns on a native 4:2:2 frame is refused `colour-unavailable`,
    // never drawn by dropping a sample.
    let spec = Spec::colour8(1, 3, 3, "YBR_FULL_422", &[50, 200, 100, 160]);
    expect_code(
        preview_frame(&build(&spec)?, FrameRequest::default()),
        "colour-unavailable",
        "odd columns",
    )
}

// ---------------------------------------------------------------------------
// 4. PALETTE COLOR, PS3.3 C.7.6.3.1.5 and C.7.9. The descriptor is number
//    of entries, first stored value mapped, bits per entry, and inputs
//    below the first mapped value and above the last entry clamp to the
//    end entries. Ruling R1 reads the whole palette as 8-bit only when at
//    least one channel is declared above 8 bits and every entry of every
//    such channel fits in 8 bits, otherwise each channel is shifted by its
//    own declared depth minus 8.

#[test]
fn palette_maps_the_stored_value_from_the_first_mapped_input() -> Result<(), String> {
    // AC7. PALETTE COLOR 5 by 1, 8-bit stored `9 10 12 13 99`, descriptors
    // `4\10\16`, red `0 85 170 255`, green `255 170 85 0`, blue
    // `10 20 30 40`. Index 9 is below the first mapped input 10 and clamps
    // to entry 0, index 99 is above the last entry and clamps to entry 3.
    // Every declared 16-bit entry fits in 8 bits, so ruling R1 reads the
    // palette as 8-bit and the entries are taken as they are. Windowing
    // before the palette is forbidden by D-23.
    let mut spec = Spec::colour8(1, 5, 1, "PALETTE COLOR", &[9, 10, 12, 13, 99]);
    for tag in [(0x0028, 0x1101), (0x0028, 0x1102), (0x0028, 0x1103)] {
        spec = spec.with_element(tag, *b"US", &lut_descriptor(4, 10, 16));
    }
    spec = spec.with_element((0x0028, 0x1201), *b"OW", &lut_words(&[0, 85, 170, 255]));
    spec = spec.with_element((0x0028, 0x1202), *b"OW", &lut_words(&[255, 170, 85, 0]));
    spec = spec.with_element((0x0028, 0x1203), *b"OW", &lut_words(&[10, 20, 30, 40]));
    expect_initial(
        preview_frame(&build(&spec)?, FrameRequest::default()),
        &[0, 255, 10, 0, 255, 10, 170, 85, 30, 255, 0, 40, 255, 0, 40],
        "palette first mapped input",
    )
}

#[test]
fn conforming_16_bit_palette_entries_shift_by_eight() -> Result<(), String> {
    // AC22. PALETTE COLOR 5 by 1, 8-bit stored `0 1 2 3 4`, descriptors
    // `5\0\16` for all three, red `0 255 256 32768 65535`, green and blue
    // `65535 32768 256 255 0`. Entries exceed 8 bits, so each channel is
    // shifted right by its own declared depth minus 8, here 8.
    // Truncation, not rounding, gives 128 for 32768 and 1 for 256.
    let mut spec = Spec::colour8(1, 5, 1, "PALETTE COLOR", &[0, 1, 2, 3, 4]);
    for tag in [(0x0028, 0x1101), (0x0028, 0x1102), (0x0028, 0x1103)] {
        spec = spec.with_element(tag, *b"US", &lut_descriptor(5, 0, 16));
    }
    spec = spec.with_element(
        (0x0028, 0x1201),
        *b"OW",
        &lut_words(&[0, 255, 256, 32768, 65535]),
    );
    spec = spec.with_element(
        (0x0028, 0x1202),
        *b"OW",
        &lut_words(&[65535, 32768, 256, 255, 0]),
    );
    spec = spec.with_element(
        (0x0028, 0x1203),
        *b"OW",
        &lut_words(&[65535, 32768, 256, 255, 0]),
    );
    expect_initial(
        preview_frame(&build(&spec)?, FrameRequest::default()),
        &[0, 255, 255, 0, 128, 128, 1, 1, 1, 128, 0, 0, 255, 0, 0],
        "conforming 16 bit palette",
    )
}

#[test]
fn every_wide_channel_fitting_8_bits_reads_the_palette_as_8_bit() -> Result<(), String> {
    // AC22 under ruling R1. One entry per channel, red 0, green 100 and
    // blue 255, declared 16 bits and again declared 12 bits: at least one
    // channel is declared above 8 bits and every entry of every such
    // channel fits in 8 bits, so the whole palette is read as 8-bit and
    // the entry is taken as it is.
    for declared in [16_u16, 12] {
        let mut spec = Spec::colour8(1, 1, 1, "PALETTE COLOR", &[0]);
        for tag in [(0x0028, 0x1101), (0x0028, 0x1102), (0x0028, 0x1103)] {
            spec = spec.with_element(tag, *b"US", &lut_descriptor(1, 0, declared));
        }
        spec = spec.with_element((0x0028, 0x1201), *b"OW", &lut_words(&[0]));
        spec = spec.with_element((0x0028, 0x1202), *b"OW", &lut_words(&[100]));
        spec = spec.with_element((0x0028, 0x1203), *b"OW", &lut_words(&[255]));
        expect_initial(
            preview_frame(&build(&spec)?, FrameRequest::default()),
            &[0, 100, 255],
            &format!("8 bit fitting palette declared {declared}"),
        )?;
        // Retain the assigned three-entry ramp as well as the one-entry
        // channel example. Every channel is [0,100,255], so the three
        // selected pixels are grey 0,100,255 in packed RGB.
        let mut ramp = Spec::colour8(1, 3, 1, "PALETTE COLOR", &[0, 1, 2]);
        for tag in [(0x0028, 0x1101), (0x0028, 0x1102), (0x0028, 0x1103)] {
            ramp = ramp.with_element(tag, *b"US", &lut_descriptor(3, 0, declared));
        }
        for tag in [(0x0028, 0x1201), (0x0028, 0x1202), (0x0028, 0x1203)] {
            ramp = ramp.with_element(tag, *b"OW", &lut_words(&[0, 100, 255]));
        }
        expect_initial(
            preview_frame(&build(&ramp)?, FrameRequest::default()),
            &[0, 0, 0, 100, 100, 100, 255, 255, 255],
            &format!("three-entry fallback ramp declared {declared}"),
        )?;
    }
    Ok(())
}

#[test]
fn a_twelve_bit_palette_not_read_as_8_bit_shifts_by_four() -> Result<(), String> {
    // AC22. Descriptors `5\0\12` with entries above 255, so the palette is
    // not read as 8-bit and each 12-bit channel shifts right by 4. Red
    // `0 4095 2048 256 255` becomes `0 255 128 16 15`, green
    // `4095 2048 256 255 0` becomes `255 128 16 15 0` and blue
    // `300 100 4095 0 255` becomes `18 6 255 0 15`.
    let mut spec = Spec::colour8(1, 5, 1, "PALETTE COLOR", &[0, 1, 2, 3, 4]);
    for tag in [(0x0028, 0x1101), (0x0028, 0x1102), (0x0028, 0x1103)] {
        spec = spec.with_element(tag, *b"US", &lut_descriptor(5, 0, 12));
    }
    spec = spec.with_element(
        (0x0028, 0x1201),
        *b"OW",
        &lut_words(&[0, 4095, 2048, 256, 255]),
    );
    spec = spec.with_element(
        (0x0028, 0x1202),
        *b"OW",
        &lut_words(&[4095, 2048, 256, 255, 0]),
    );
    spec = spec.with_element(
        (0x0028, 0x1203),
        *b"OW",
        &lut_words(&[300, 100, 4095, 0, 255]),
    );
    expect_initial(
        preview_frame(&build(&spec)?, FrameRequest::default()),
        &[0, 255, 18, 255, 128, 6, 128, 16, 255, 16, 15, 0, 15, 0, 15],
        "12 bit palette shift by 4",
    )
}

#[test]
fn a_missing_palette_table_is_refused_by_name() -> Result<(), String> {
    // PS3.3 C.7.6.3.1.5. A PALETTE COLOR frame whose green and blue tables
    // are absent has no colour to draw, so it is refused
    // `colour-unavailable` rather than drawn with a partial palette.
    let mut spec = Spec::colour8(1, 5, 1, "PALETTE COLOR", &[0, 1, 2, 3, 4]);
    for tag in [(0x0028, 0x1101), (0x0028, 0x1102), (0x0028, 0x1103)] {
        spec = spec.with_element(tag, *b"US", &lut_descriptor(5, 0, 16));
    }
    spec = spec.with_element(
        (0x0028, 0x1201),
        *b"OW",
        &lut_words(&[0, 100, 200, 255, 255]),
    );
    expect_code(
        preview_frame(&build(&spec)?, FrameRequest::default()),
        "colour-unavailable",
        "missing table",
    )
}

#[test]
fn a_segmented_palette_is_refused_by_name() -> Result<(), String> {
    // PS3.3 C.7.9. Segmented Palette Colour Lookup Table Data (0028,1221)
    // is its own encoding this build does not expand, so a frame carrying
    // it is refused `colour-unavailable` by name.
    let mut spec = Spec::colour8(1, 5, 1, "PALETTE COLOR", &[0, 1, 2, 3, 4]);
    for tag in [(0x0028, 0x1101), (0x0028, 0x1102), (0x0028, 0x1103)] {
        spec = spec.with_element(tag, *b"US", &lut_descriptor(5, 0, 16));
    }
    for tag in [(0x0028, 0x1201), (0x0028, 0x1202), (0x0028, 0x1203)] {
        spec = spec.with_element(tag, *b"OW", &lut_words(&[0, 100, 200, 255, 255]));
    }
    spec = spec.with_element((0x0028, 0x1221), *b"OW", &[0, 0]);
    expect_code(
        preview_frame(&build(&spec)?, FrameRequest::default()),
        "colour-unavailable",
        "segmented palette",
    )
}

#[test]
fn a_palette_entry_depth_above_16_bits_is_refused_by_name() -> Result<(), String> {
    // The plan's display scaling rule. A palette declared outside 8 to 16
    // bits is refused `colour-unavailable`, never shifted by a guess.
    let mut spec = Spec::colour8(1, 4, 1, "PALETTE COLOR", &[0, 1, 2, 3]);
    for tag in [(0x0028, 0x1101), (0x0028, 0x1102), (0x0028, 0x1103)] {
        spec = spec.with_element(tag, *b"US", &lut_descriptor(4, 0, 17));
    }
    for tag in [(0x0028, 0x1201), (0x0028, 0x1202), (0x0028, 0x1203)] {
        spec = spec.with_element(tag, *b"OW", &lut_words(&[0, 100, 200, 255]));
    }
    expect_code(
        preview_frame(&build(&spec)?, FrameRequest::default()),
        "colour-unavailable",
        "17 bit descriptor",
    )
}

// ---------------------------------------------------------------------------
// 5. Colour above 8 bits, PS3.3 C.7.6.3.1 Bits Stored with the plan's one
//    display scaling rule: shift right by n - 8 for 9 <= n <= 16, identity
//    at 8, and refuse depths outside that range and signed samples.

#[test]
fn colour_samples_are_scaled_by_bits_stored() -> Result<(), String> {
    // AC11. Native RGB, Bits Allocated 16, Bits Stored 12, stored
    // `4095 2048 15`. Shifting right by 4 gives `255 128 0`. cornerstone's
    // saturation would give `255 255 15` and shifting by Bits Allocated is
    // the identity here, both rejected.
    let spec = Spec::colour16(1, 1, 3, "RGB", (16, 12, 11, 0), &[4095, 2048, 15]);
    expect_bytes(
        preview_frame(&build(&spec)?, FrameRequest::default()),
        &[255, 128, 0],
        "12 in 16 RGB",
    )
}

#[test]
fn the_same_samples_at_16_stored_bits_use_the_declared_shift() -> Result<(), String> {
    // AC11. The same stored words `4095 2048 15` at Bits Stored 16 shift
    // right by 16 - 8 = 8 and reply `15 8 0`, so the rule reads the
    // header and not the container.
    let spec = Spec::colour16(1, 1, 3, "RGB", (16, 16, 15, 0), &[4095, 2048, 15]);
    expect_bytes(
        preview_frame(&build(&spec)?, FrameRequest::default()),
        &[15, 8, 0],
        "16 stored bits",
    )
}

#[test]
fn a_12_bit_ybr_frame_is_scaled_before_the_matrix() -> Result<(), String> {
    // AC11. Native YBR_FULL at Bits Stored 12, `1600 2400 1440`, shifts to
    // `100 150 90` and then converts as AC3, replying `47 120 139`. The
    // shift happens before stage 4, so the 8-bit matrix and its + 128
    // offset apply to an 8-bit value.
    let spec = Spec::colour16(1, 1, 3, "YBR_FULL", (16, 12, 11, 0), &[1600, 2400, 1440]);
    expect_bytes(
        preview_frame(&build(&spec)?, FrameRequest::default()),
        &[47, 120, 139],
        "12 bit YBR",
    )
}

#[test]
fn a_seven_bit_colour_depth_is_refused_by_name() -> Result<(), String> {
    // The plan's display scaling rule. A colour sample depth below 8 is
    // refused `colour-unavailable` with the unsupported depth reason.
    let spec = Spec::colour8(1, 1, 3, "RGB", &[100, 150, 90]).with_bits((8, 7, 6, 0));
    expect_code(
        preview_frame(&build(&spec)?, FrameRequest::default()),
        "colour-unavailable",
        "7 bit depth",
    )
}

#[test]
fn a_17_bit_colour_depth_is_refused_by_name() -> Result<(), String> {
    // AC11. Bits Stored 17 in a 32-bit container is outside 8 to 16 and is
    // refused `colour-unavailable` with the unsupported depth reason.
    let spec = Spec::colour32(1, 1, 3, "RGB", (32, 17, 16, 0), &[131_071, 65_536, 1]);
    expect_code(
        preview_frame(&build(&spec)?, FrameRequest::default()),
        "colour-unavailable",
        "17 bit depth",
    )
}

#[test]
fn signed_colour_samples_are_refused_by_name() -> Result<(), String> {
    // The plan's display scaling rule. Pixel Representation 1 on a colour
    // frame is refused `colour-unavailable` with the signed reason.
    let spec = Spec::colour16(1, 1, 3, "RGB", (16, 16, 15, 1), &[100, 150, 90]);
    expect_code(
        preview_frame(&build(&spec)?, FrameRequest::default()),
        "colour-unavailable",
        "signed colour",
    )
}

// ---------------------------------------------------------------------------
// 6. The colour window and invert, the plan's D-29 rule following
//    cornerstone3D 5.8.2: the existing PS3.3 C.11.2.1.2 LINEAR per channel
//    over 0 to 255, initial width 256 and center 128, inversion after the
//    window, and the file's own windowing evidence never read on colour.

#[test]
fn the_initial_colour_window_ignores_file_windowing_evidence() -> Result<(), String> {
    // AC19 and the plan's item 9. Width 256 and center 128 is the identity
    // on 0 to 255, so stored `10 200 50` replies as it stands. The file's
    // Window Center and Width, Rescale and Modality are nonsense to the
    // colour route, which never builds stage 1, so they change nothing and
    // the reply carries no file window index and a window count of 0.
    let spec = Spec::colour8(1, 1, 3, "RGB", &[10, 200, 50])
        .with_modality("CT")
        .with_rescale("2", "-100")
        .with_window("40", "400");
    expect_initial(
        preview_frame(&build(&spec)?, FrameRequest::default()),
        &[10, 200, 50],
        "initial colour window",
    )
}

#[test]
fn a_colour_window_is_applied_per_channel() -> Result<(), String> {
    // AC19. Native RGB 1 by 1, stored `100 30 200`, width 100, center 128.
    // The LINEAR function is c' 127.5 and w' 99, so the channels are
    // `56.666664 0 255` and the reply is `57 0 255`. Stored
    // `125 128 130` at width 10 gives `56.666664 141.66667 198.33334` and
    // reply `57 142 198`, the row that rejects w in place of w - 1, which
    // would give `64 140 191`.
    let spec = Spec::colour8(1, 1, 3, "RGB", &[100, 30, 200]);
    let windowed = draw(
        &build(&spec)?,
        FrameRequest {
            window_center: Some(128.0),
            window_width: Some(100.0),
            ..FrameRequest::default()
        },
        "width 100",
    )?;
    if windowed.grey != [57, 0, 255] {
        return Err(format!("width 100: got {:?}", windowed.grey));
    }
    if windowed.window_center.to_bits() != 128.0_f32.to_bits()
        || windowed.window_width.to_bits() != 100.0_f32.to_bits()
    {
        return Err(format!(
            "width 100: the reply window is {} by {}",
            windowed.window_center, windowed.window_width
        ));
    }
    let wide = Spec::colour8(1, 1, 3, "RGB", &[125, 128, 130]);
    let narrow = draw(
        &build(&wide)?,
        FrameRequest {
            window_center: Some(128.0),
            window_width: Some(10.0),
            ..FrameRequest::default()
        },
        "width 10",
    )?;
    if narrow.grey != [57, 142, 198] {
        return Err(format!("width 10: got {:?}", narrow.grey));
    }
    if narrow.window_function != "LINEAR" {
        return Err(format!(
            "width 10: the window function is {}",
            narrow.window_function
        ));
    }
    Ok(())
}

#[test]
fn colour_invert_flips_each_channel_after_the_window() -> Result<(), String> {
    // AC19. The same windows with invert give 255 - y per channel after
    // the window: `198.33334 255 0` replies `198 255 0`, and the width 10
    // row replies `198 113 57`.
    let spec = Spec::colour8(1, 1, 3, "RGB", &[100, 30, 200]);
    let inverted = draw(
        &build(&spec)?,
        FrameRequest {
            window_center: Some(128.0),
            window_width: Some(100.0),
            invert: true,
            ..FrameRequest::default()
        },
        "width 100 inverted",
    )?;
    if inverted.grey != [198, 255, 0] {
        return Err(format!("width 100: got {:?}", inverted.grey));
    }
    if !inverted.inverted {
        return Err("width 100: the reply is not labelled inverted".to_owned());
    }
    let wide = Spec::colour8(1, 1, 3, "RGB", &[125, 128, 130]);
    let narrow = draw(
        &build(&wide)?,
        FrameRequest {
            window_center: Some(128.0),
            window_width: Some(10.0),
            invert: true,
            ..FrameRequest::default()
        },
        "width 10 inverted",
    )?;
    if narrow.grey != [198, 113, 57] {
        return Err(format!("width 10: got {:?}", narrow.grey));
    }
    Ok(())
}

#[test]
fn a_window_function_other_than_linear_is_refused_on_colour() -> Result<(), String> {
    // The plan's protocol table. A colour frame takes LINEAR only, so a
    // request naming another VOI LUT Function is `colour-not-applicable`.
    let spec = Spec::colour8(1, 1, 3, "RGB", &[100, 30, 200]);
    expect_code(
        preview_frame(
            &build(&spec)?,
            FrameRequest {
                window_function: Some(VoiFunction::Sigmoid),
                ..FrameRequest::default()
            },
        ),
        "colour-not-applicable",
        "sigmoid on colour",
    )
}

#[test]
fn a_window_preset_is_refused_on_colour() -> Result<(), String> {
    // The plan's protocol table. Presets are in modality units a colour
    // frame has none of, so a preset request is `colour-not-applicable`.
    let spec = Spec::colour8(1, 1, 3, "RGB", &[100, 30, 200]);
    expect_code(
        preview_frame(
            &build(&spec)?,
            FrameRequest {
                window_preset: Some(WindowPreset::Bone),
                ..FrameRequest::default()
            },
        ),
        "colour-not-applicable",
        "preset on colour",
    )
}

#[test]
fn a_file_window_index_is_refused_on_colour() -> Result<(), String> {
    // The plan's protocol table. The colour route never reads the file's
    // Window Center and Width pairs, so an explicit file window index is
    // `colour-not-applicable`.
    let spec = Spec::colour8(1, 1, 3, "RGB", &[100, 30, 200]).with_window("40", "400");
    expect_code(
        preview_frame(
            &build(&spec)?,
            FrameRequest {
                window_index: Some(0),
                ..FrameRequest::default()
            },
        ),
        "colour-not-applicable",
        "file window index on colour",
    )
}

#[test]
fn a_refused_colour_request_leaves_the_initial_image_drawable() -> Result<(), String> {
    // The plan's state table. A refused request never damages the held
    // frame, so after each refusal a fresh initial request draws the same
    // bytes as before.
    let file = build(&Spec::colour8(1, 1, 3, "RGB", &[100, 30, 200]))?;
    for (name, request) in [
        (
            "sigmoid",
            FrameRequest {
                window_function: Some(VoiFunction::Sigmoid),
                ..FrameRequest::default()
            },
        ),
        (
            "preset",
            FrameRequest {
                window_preset: Some(WindowPreset::Bone),
                ..FrameRequest::default()
            },
        ),
        (
            "file window index",
            FrameRequest {
                window_index: Some(0),
                ..FrameRequest::default()
            },
        ),
    ] {
        expect_code(preview_frame(&file, request), "colour-not-applicable", name)?;
        expect_bytes(
            preview_frame(&file, FrameRequest::default()),
            &[100, 30, 200],
            &format!("initial after {name}"),
        )?;
    }
    Ok(())
}

#[test]
fn a_colour_window_tie_rounds_half_to_even() -> Result<(), String> {
    // AC21. Native RGB 1 by 1, stored `130 130 130`, width 511, center
    // 128.5: the channel is exactly 128.5 and the reply is `128 128 128`,
    // the tie to even. Inverted, `126.5` replies `126 126 126`. Center
    // 130.5 and width 100 put stored 130 on exactly 127.5, which replies
    // 128, so both tie directions are asserted.
    let file = build(&Spec::colour8(1, 1, 3, "RGB", &[130, 130, 130]))?;
    let up = draw(
        &file,
        FrameRequest {
            window_center: Some(130.5),
            window_width: Some(100.0),
            ..FrameRequest::default()
        },
        "127.5",
    )?;
    if up.grey != [128, 128, 128] {
        return Err(format!("127.5: got {:?}", up.grey));
    }
    let down = draw(
        &file,
        FrameRequest {
            window_center: Some(128.5),
            window_width: Some(511.0),
            ..FrameRequest::default()
        },
        "128.5",
    )?;
    if down.grey != [128, 128, 128] {
        return Err(format!("128.5: got {:?}", down.grey));
    }
    let flipped = draw(
        &file,
        FrameRequest {
            window_center: Some(128.5),
            window_width: Some(511.0),
            invert: true,
            ..FrameRequest::default()
        },
        "inverted 126.5",
    )?;
    if flipped.grey != [126, 126, 126] {
        return Err(format!("inverted 126.5: got {:?}", flipped.grey));
    }
    Ok(())
}

#[test]
fn the_skipped_window_table_returns_the_raw_bytes() -> Result<(), String> {
    // AC19. Width 255 and center 127 with invert off is cornerstone's
    // skipped table condition, so the reply is the raw stage 4 bytes
    // `100 100 100`. Applying the formula instead would give 100.8957 and
    // reply 101.
    let spec = Spec::colour8(1, 1, 3, "RGB", &[100, 100, 100]);
    expect_bytes(
        preview_frame(
            &build(&spec)?,
            FrameRequest {
                window_center: Some(127.0),
                window_width: Some(255.0),
                ..FrameRequest::default()
            },
        ),
        &[100, 100, 100],
        "skipped table",
    )
}

// ---------------------------------------------------------------------------
// 7. The colour value at a point. D-23: a three-sample frame has no scalar
//    modality value, so the readout is the R, G and B stage 4 output. The
//    point edges keep the existing value policy.

#[test]
fn a_colour_value_at_a_point_reports_the_rgb_channels() -> Result<(), String> {
    // AC13. The second pixel of native RGB 2 by 1 `10 20 30 200 100 50` is
    // column 2, row 1, index column 1, index row 0, with rgb `200 100 50`
    // and the one decimal readout. The closed right and bottom edge of the
    // last cell still addresses it, and a point past the edge is refused
    // `point` with no partial list, the existing policy.
    let file = build(&Spec::colour8(1, 2, 3, "RGB", &[10, 20, 30, 200, 100, 50]).with_planar(0))?;
    let json = value_frame(&file, 0, &[at(1.5, 0.5)])
        .map_err(|error| format!("value: refused with {}", error.code()))?;
    for want in [
        "\"unit\":\"RGB\"",
        "\"rgb\":[200,100,50]",
        "\"text\":\"R 200.0 G 100.0 B 50.0\"",
        "\"column\":2,\"row\":1,\"index_column\":1,\"index_row\":0",
        "\"padding\":false",
    ] {
        if !json.contains(want) {
            return Err(format!("value: {json} has no {want}"));
        }
    }
    for absent in ["\"value\"", "modality", "unit_source"] {
        if json.contains(absent) {
            return Err(format!("value: {json} carries {absent}"));
        }
    }
    let edge = value_frame(&file, 0, &[at(2.0, 1.0)])
        .map_err(|error| format!("edge: refused with {}", error.code()))?;
    if !edge.contains("\"rgb\":[200,100,50]") {
        return Err(format!("edge: {edge} is not the last pixel"));
    }
    expect_value_code(
        value_frame(&file, 0, &[at(2.01, 0.5)]),
        "point",
        "past the right edge",
    )
}

// ---------------------------------------------------------------------------
// 8. Region statistics on a colour frame. The plan's item 12: one walk
//    gathering three channel vectors, the F-075 population statistics per
//    channel, values taken after the display scaling and before the window
//    and invert, no padding on colour, and calibration unchanged.

#[test]
fn a_colour_region_reports_per_channel_statistics() -> Result<(), String> {
    // AC13. A rectangle over both pixels of `10 20 30 200 100 50`. R has
    // count 2, mean 105, SD 95, min 10, max 200. G mean 60, SD 40, min 20,
    // max 100. B mean 40, SD 10, min 30, max 50, all population form. The
    // unit is RGB, colour has no padding to exclude, and the combined text
    // is present. Statistics of the windowed bytes would give other means,
    // because the API never consumes the display window.
    let file = build(&Spec::colour8(1, 2, 3, "RGB", &[10, 20, 30, 200, 100, 50]).with_planar(0))?;
    let json = measure_frame(
        &file,
        0,
        MeasureShape::Rectangle,
        &[at(0.0, 0.0), at(2.0, 1.0)],
    )
    .map_err(|error| format!("region: refused with {}", error.code()))?;
    let want = "\"stats\":{\"count\":2,\"padding_excluded\":0,\"unit\":\"RGB\",\"channels\":{\
                \"r\":{\"count\":2,\"mean\":105,\"sd\":95,\"min\":10,\"max\":200},\
                \"g\":{\"count\":2,\"mean\":60,\"sd\":40,\"min\":20,\"max\":100},\
                \"b\":{\"count\":2,\"mean\":40,\"sd\":10,\"min\":30,\"max\":50}}";
    if !json.contains(want) {
        return Err(format!("region: {json} has no {want}"));
    }
    if !json.contains("\"text\":") {
        return Err(format!("region: {json} has no combined text"));
    }
    Ok(())
}

#[test]
fn a_colour_length_stays_calibrated() -> Result<(), String> {
    // AC13. A length on a colour frame reports millimetres from Pixel
    // Spacing as before, PS3.3 C.7.6.2 row then column. One column step of
    // the `0.5\0.5` pair is 0.5 mm exactly, so the calibration is the
    // colour-independent part of the reply.
    let file = build(
        &Spec::colour8(1, 2, 3, "RGB", &[10, 20, 30, 200, 100, 50])
            .with_planar(0)
            .with_spacing("0.5", "0.5"),
    )?;
    let json = measure_frame(
        &file,
        0,
        MeasureShape::Length,
        &[at(1.0, 1.0), at(2.0, 1.0)],
    )
    .map_err(|error| format!("length: refused with {}", error.code()))?;
    for want in [
        "\"length\":{\"value\":0.5,\"unit\":\"mm\",\"text\":\"0.5 mm\"}",
        "\"source\":\"pixel-spacing\",\"row_mm\":0.5,\"column_mm\":0.5",
    ] {
        if !json.contains(want) {
            return Err(format!("length: {json} has no {want}"));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// 9. ICC Profile (0028,2000). The presence test is the element alone,
//    whatever its length, and the profile is named as not applied (D-30)
//    while the bytes are drawn unchanged.

#[test]
fn an_icc_profile_is_named_and_changes_no_bytes() -> Result<(), String> {
    // AC26. The same frame with and without an ICC Profile element of an
    // arbitrary length. The inspect reply names `icc` true only for the
    // file carrying the element, and both draw the same preview bytes.
    let plain = Spec::colour8(1, 1, 3, "RGB", &[100, 200, 50]);
    let with_profile = plain.clone().with_element(
        (0x0028, 0x2000),
        *b"OB",
        &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14],
    );
    let named = inspect_json(&build(&with_profile)?, 0)
        .map_err(|error| format!("inspect: refused with {}", error.code()))?;
    let bare = inspect_json(&build(&plain)?, 0)
        .map_err(|error| format!("inspect: refused with {}", error.code()))?;
    if !named.contains("\"icc\":true") {
        return Err(format!("inspect: {named} does not name the profile"));
    }
    if !bare.contains("\"icc\":false") {
        return Err(format!("inspect: {bare} names a profile it does not carry"));
    }
    let drawn = preview_frame(&build(&with_profile)?, FrameRequest::default())
        .map_err(|error| format!("icc preview: refused with {}", error.code()))?;
    let clean = preview_frame(&build(&plain)?, FrameRequest::default())
        .map_err(|error| format!("plain preview: refused with {}", error.code()))?;
    if drawn.grey != clean.grey || drawn.grey != [100, 200, 50] {
        return Err(format!("icc preview: {drawn:?} against {clean:?}"));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// 10. A decoder that already converted the colour space is not converted
//     again. PS3.3 C.7.6.3.1.2's trap: converting decoder RGB a second
//     time darkens and shifts hue.

#[test]
fn jpeg_ybr_full_422_is_not_converted_twice() -> Result<(), String> {
    // AC9. The existing JPEG baseline fixture wrapped with Photometric
    // Interpretation `YBR_FULL_422`. The decoder reports RGB for three
    // samples, so the colour stage copies its output and the preview is
    // the `Registry::decode` output byte for byte. A second YBR conversion
    // would give other bytes for most pixels.
    let codestream = std::fs::read(fixture("jpeg_baseline_rgb.jpg"))
        .map_err(|_| "jpeg_baseline_rgb.jpg is not readable".to_owned())?;
    let desc = FrameDesc::new(FrameDescInput {
        rows: 64,
        columns: 96,
        samples_per_pixel: 3,
        bits_allocated: 8,
        bits_stored: 8,
        high_bit: 7,
        pixel_representation: PixelRepresentation::Unsigned,
        photometric_interpretation: "YBR_FULL_422".to_owned(),
        pixel_data_vr: PixelDataVr::Ob,
    })
    .map_err(|error| error.to_string())?;
    let mut registry = Registry::new();
    register_jpeg_decoders(&mut registry).map_err(|error| error.to_string())?;
    let mut expected = vec![0_u8; 64 * 96 * 3];
    registry
        .decode(JPEG_BASELINE, &codestream, &desc, &mut expected)
        .map_err(|error| error.to_string())?;
    let spec = Spec::colour8(64, 96, 3, "YBR_FULL_422", &[]);
    let file = encapsulated(&spec, JPEG_BASELINE, &codestream)?;
    expect_bytes(
        preview_frame(&file, FrameRequest::default()),
        &expected,
        "jpeg YBR_FULL_422",
    )
}

fn fixture(name: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../crates/ocelli-codec/tests/fixtures")
        .join(name)
}

// ---------------------------------------------------------------------------
// The HTTP routes.

fn start_viewer() -> Result<SocketAddr, String> {
    let listener = TcpListener::bind("127.0.0.1:0").map_err(|error| error.to_string())?;
    let addr = listener.local_addr().map_err(|error| error.to_string())?;
    let viewer = Arc::new(Viewer::new(ViewerConfig {
        cache_bytes: 8 * 1024 * 1024,
    }));
    std::thread::spawn(move || {
        let _ = serve_on(listener, viewer);
    });
    Ok(addr)
}

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

fn split_http(response: &[u8]) -> Result<(String, Vec<u8>), String> {
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
    Ok((String::from_utf8_lossy(header).into_owned(), body.to_vec()))
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

#[test]
fn the_preview_route_names_the_channel_count() -> Result<(), String> {
    // The wire contract. Every successful preview reply carries
    // `X-Ocelli-Channels`, 3 for a colour frame whose body is row major
    // packed RGB, and 1 for a monochrome frame whose body is one grey byte
    // per pixel.
    let colour = build(&Spec::colour8(1, 2, 3, "RGB", &[10, 20, 30, 200, 100, 50]).with_planar(0))?;
    let (header, body) = split_http(&exchange(start_viewer()?, &post_body("/preview", &colour))?)?;
    if !header.starts_with("HTTP/1.1 200 ") {
        return Err(format!("colour preview: {header}"));
    }
    if header_line(&header, "X-Ocelli-Channels").as_deref() != Some("3") {
        return Err(format!("colour preview: {header}"));
    }
    if body != [10, 20, 30, 200, 100, 50] {
        return Err(format!("colour preview: body {:?}", body));
    }
    let mono = slice(0).ok_or_else(|| "no synthetic slice".to_owned())?;
    let (header, body) = split_http(&exchange(start_viewer()?, &post_body("/preview", &mono))?)?;
    if !header.starts_with("HTTP/1.1 200 ") {
        return Err(format!("monochrome preview: {header}"));
    }
    if header_line(&header, "X-Ocelli-Channels").as_deref() != Some("1") {
        return Err(format!("monochrome preview: {header}"));
    }
    if body.len() != 128 * 96 {
        // The synthetic slice is 128 columns by 96 rows.
        return Err(format!(
            "monochrome preview: {} bytes for a 128 by 96 frame",
            body.len()
        ));
    }
    Ok(())
}

fn post_body(route: &str, file: &[u8]) -> Vec<u8> {
    let mut request = format!(
        "POST {route} HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: {len}\r\n\r\n",
        len = file.len()
    )
    .into_bytes();
    request.extend_from_slice(file);
    request
}

#[test]
fn the_measure_route_ignores_display_window_headers_on_colour() -> Result<(), String> {
    // The plan's item 12. The statistics are the stage 4 output before the
    // window and invert, so display window headers on the route change
    // nothing, exactly as the direct API cannot take one.
    let file = build(&Spec::colour8(1, 2, 3, "RGB", &[10, 20, 30, 200, 100, 50]).with_planar(0))?;
    let addr = start_viewer()?;
    let id = post_file(addr, &file)?;
    let ask = |extra: &str| -> Result<String, String> {
        let mut request = format!(
            "POST /measure HTTP/1.1\r\nHost: 127.0.0.1\r\nX-Ocelli-File: {id}\r\n\
             X-Ocelli-Frame: 0\r\nX-Ocelli-Shape: rectangle\r\n{extra}\
             Content-Length: 7\r\n\r\n"
        )
        .into_bytes();
        request.extend_from_slice(b"0,0 2,1");
        let (header, body) = split_http(&exchange(addr, &request)?)?;
        if !header.starts_with("HTTP/1.1 200 ") {
            return Err(header);
        }
        String::from_utf8(body).map_err(|_| "the measure body is not text".to_owned())
    };
    let plain = ask("")?;
    let windowed =
        ask("X-Ocelli-Window-Center: 128\r\nX-Ocelli-Window-Width: 100\r\nX-Ocelli-Invert: 1\r\n")?;
    if plain != windowed {
        return Err(format!("plain {plain} against windowed {windowed}"));
    }
    let want = "\"channels\":{\"r\":{\"count\":2,\"mean\":105,\"sd\":95,\"min\":10,\"max\":200}";
    if !plain.contains(want) {
        return Err(format!("measure: {plain} has no {want}"));
    }
    Ok(())
}
