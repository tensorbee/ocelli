//! Monochrome frames of one Part 10 object.
//!
//! Windowed samples go through `LutChain`. The grey byte is
//! `display_index`, the same address the colour table uses.
//! A window override replaces the file window with another call of that chain.
//! A Modality LUT Sequence is refused, because this preview does not apply one.
//! A VOI LUT Sequence is refused unless that override replaces it.
//! Enhanced multi-frame rescale and window values come from the functional
//! groups, with the top-level attributes as the fallback.
//! Invert flips the display byte after the chain.

use dicom_core::dictionary::{DataDictionary, DataDictionaryEntry};
use dicom_core::header::{DataElement, EmptyObject, Header};
use dicom_core::{DicomValue, PrimitiveValue, Tag, VR};
use dicom_object::mem::{InMemDicomObject, InMemElement};
use dicom_object::{DefaultDicomObject, StandardDataDictionary};
use ocelli_codec::{
    DecodePhotometricInterpretation, FrameDesc, FrameDescInput, PixelDataVr, PixelRepresentation,
    RawDecoder, Registry, register_htj2k_decoders, register_jpeg_decoders,
    register_jpeg2000_decoders, register_jpegls_decoders, register_native_and_rle_decoders,
};
use ocelli_core::{Display, Stored};
use ocelli_dicom::{
    DispatchPath, EncapsulatedFrameIndex, MetadataElement, MetadataSet, MetadataValue,
    MultiframeMetadata, ParseError, TopLevelFallback, parse_decimal_string, parse_part10,
};
use ocelli_pixel::{
    ByteOrder, ImageDimensions, LutChain, ModalityTransform, PhotometricInterpretation,
    PresentationLutEvidence, PresentationLutShape, SampleLayout, StoredBits,
    StoredPixelDescription, VoiFunction, VoiTransform,
};
use ocelli_render::display_index;
use ocelli_viewport::view::f32_from_f64;

const ROWS: Tag = Tag(0x0028, 0x0010);
const COLUMNS: Tag = Tag(0x0028, 0x0011);
const BITS_ALLOCATED: Tag = Tag(0x0028, 0x0100);
const BITS_STORED: Tag = Tag(0x0028, 0x0101);
const HIGH_BIT: Tag = Tag(0x0028, 0x0102);
const PIXEL_REPRESENTATION: Tag = Tag(0x0028, 0x0103);
const SAMPLES_PER_PIXEL: Tag = Tag(0x0028, 0x0002);
const PHOTOMETRIC: Tag = Tag(0x0028, 0x0004);
const NUMBER_OF_FRAMES: Tag = Tag(0x0028, 0x0008);
const RESCALE_SLOPE: Tag = Tag(0x0028, 0x1053);
const RESCALE_INTERCEPT: Tag = Tag(0x0028, 0x1052);
const WINDOW_CENTER: Tag = Tag(0x0028, 0x1050);
const WINDOW_WIDTH: Tag = Tag(0x0028, 0x1051);
const VOI_FUNCTION: Tag = Tag(0x0028, 0x1056);
const MODALITY_LUT_SEQUENCE: Tag = Tag(0x0028, 0x3000);
const VOI_LUT_SEQUENCE: Tag = Tag(0x0028, 0x3010);
const FRAME_VOI_LUT_SEQUENCE: Tag = Tag(0x0028, 0x9132);
const PIXEL_VALUE_TRANSFORMATION: Tag = Tag(0x0028, 0x9145);
const PRESENTATION_SHAPE: Tag = Tag(0x2050, 0x0020);
const PRESENTATION_SEQUENCE: Tag = Tag(0x2050, 0x0010);
const PIXEL_DATA: Tag = Tag(0x7FE0, 0x0010);
const SERIES_UID: Tag = Tag(0x0020, 0x000E);
const SERIES_NUMBER: Tag = Tag(0x0020, 0x0011);
const MODALITY: Tag = Tag(0x0008, 0x0060);
const SERIES_DESCRIPTION: Tag = Tag(0x0008, 0x103E);
const INSTANCE_NUMBER: Tag = Tag(0x0020, 0x0013);
const SOP_INSTANCE: Tag = Tag(0x0008, 0x0018);
const PATIENT_NAME: Tag = Tag(0x0010, 0x0010);
const PATIENT_ID: Tag = Tag(0x0010, 0x0020);
const STUDY_DATE: Tag = Tag(0x0008, 0x0020);
const STUDY_DESCRIPTION: Tag = Tag(0x0008, 0x1030);
const ACCESSION: Tag = Tag(0x0008, 0x0050);
const MAX_PIXELS: u32 = 8_192 * 8_192;
const MAX_HEADER_ROWS: usize = 4096;
const VALUE_CAP: usize = 1024;
const BULK_TEXT_CAP: usize = 64;

/// One grey frame ready to draw.
#[derive(Debug, Clone, PartialEq)]
pub struct Preview {
    /// Columns.
    pub width: u32,
    /// Rows.
    pub height: u32,
    /// Number of Frames, or 1 when that attribute is absent.
    pub frames: u32,
    /// The file had no window, so the grey bytes use the modality range.
    pub full_range: bool,
    /// One display byte per pixel, row major.
    pub grey: Vec<u8>,
    /// Zero-based frame that was decoded.
    pub frame: u32,
    /// Window center that was applied, in modality units.
    pub window_center: f32,
    /// Window width that was applied.
    pub window_width: f32,
    /// VOI LUT Function that was applied.
    pub window_function: &'static str,
    /// The display bytes were flipped after the LUT chain.
    pub inverted: bool,
}

/// Which frame to decode, and an optional replacement window.
///
/// Absent window fields keep the file window, including the full-range
/// fallback. Invert is applied to the grey byte after `display_index`.
#[derive(Debug, Clone, Copy, Default)]
pub struct FrameRequest {
    /// Zero-based frame index.
    pub frame: u32,
    /// Flip each grey byte as `255 - byte`.
    pub invert: bool,
    /// Replacement Window Center. Set together with [`Self::window_width`].
    pub window_center: Option<f32>,
    /// Replacement Window Width.
    pub window_width: Option<f32>,
    /// Replacement VOI LUT Function. Absent keeps the file function.
    pub window_function: Option<VoiFunction>,
}

impl PreviewError {
    /// A stable token for the page. The sentence in [`std::fmt::Display`] is the one a person reads.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::NotPart10 => "not-part10",
            Self::Attribute => "attribute",
            Self::NotMonochrome => "not-monochrome",
            Self::TooLarge => "too-large",
            Self::Decode => "decode",
            Self::Window => "window",
            Self::Frame => "frame",
        }
    }
}

/// Why a file could not be shown.
///
/// No variant carries bytes, names, or sample values.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum PreviewError {
    /// The bytes are not a Part 10 object this build can read.
    #[error("the file is not a DICOM Part 10 object this preview can read")]
    NotPart10,
    /// An image attribute is missing or not usable.
    #[error("an image attribute is missing or not usable")]
    Attribute,
    /// The frame is not monochrome.
    #[error("this preview shows monochrome frames only")]
    NotMonochrome,
    /// The frame is larger than the preview will allocate.
    #[error("the frame is larger than this preview will draw")]
    TooLarge,
    /// The codec registry or the frame decode refused the pixels.
    #[error("the frame could not be decoded")]
    Decode,
    /// The LUT chain refused the window, a LUT sequence this preview does not
    /// apply, or the presentation.
    #[error("the window or presentation could not be applied")]
    Window,
    /// The requested frame index is outside Number of Frames.
    #[error("the requested frame is not in this file")]
    Frame,
}

/// Decode frame 0 with the file window.
///
/// # Errors
///
/// Returns [`PreviewError`] before a grey byte is written when the file,
/// the frame, or the window cannot be shown.
pub fn preview_first_frame(bytes: &[u8]) -> Result<Preview, PreviewError> {
    preview_frame(bytes, FrameRequest::default())
}

/// Decode one frame and map it through the LUT chain.
///
/// # Errors
///
/// Returns [`PreviewError`] before a grey byte is written when the file,
/// the frame, or the window cannot be shown.
pub fn preview_frame(bytes: &[u8], request: FrameRequest) -> Result<Preview, PreviewError> {
    if request.window_center.is_some() != request.window_width.is_some() {
        return Err(PreviewError::Window);
    }
    if request
        .window_center
        .is_some_and(|center| !center.is_finite())
        || request.window_width.is_some_and(|width| !width.is_finite())
    {
        return Err(PreviewError::Window);
    }
    let parsed = parse_part10(bytes).map_err(part10_error)?;
    let object = parsed.object();
    let rows = required_u16(object, ROWS)?;
    let columns = required_u16(object, COLUMNS)?;
    let height = u32::from(rows);
    let width = u32::from(columns);
    let pixels = height.checked_mul(width).ok_or(PreviewError::TooLarge)?;
    if pixels == 0 || pixels > MAX_PIXELS {
        return Err(PreviewError::TooLarge);
    }
    let photometric = photometric(object)?;
    let samples = required_u16(object, SAMPLES_PER_PIXEL)?;
    if samples != 1 {
        return Err(PreviewError::NotMonochrome);
    }
    let bits_allocated = required_u16(object, BITS_ALLOCATED)?;
    let bits_stored = required_u16(object, BITS_STORED)?;
    let high_bit = required_u16(object, HIGH_BIT)?;
    let representation = pixel_representation(object)?;
    let frames = frame_count(object)?;
    if request.frame >= frames {
        return Err(PreviewError::Frame);
    }
    let frame_index = usize::try_from(request.frame).map_err(|_| PreviewError::Frame)?;
    let vr = pixel_vr(object)?;
    let desc = FrameDesc::new(FrameDescInput {
        rows,
        columns,
        samples_per_pixel: samples,
        bits_allocated,
        bits_stored,
        high_bit,
        pixel_representation: match representation {
            ocelli_pixel::PixelRepresentation::Unsigned => PixelRepresentation::Unsigned,
            ocelli_pixel::PixelRepresentation::Signed => PixelRepresentation::Signed,
        },
        photometric_interpretation: photometric_name(photometric).to_owned(),
        pixel_data_vr: vr,
    })
    .map_err(|_| PreviewError::Attribute)?;
    let decoded = decode_frame(&parsed, object, &desc, frames, frame_index)?;
    let stored_bits = StoredBits::new(
        u8::try_from(bits_allocated).map_err(|_| PreviewError::Attribute)?,
        u8::try_from(bits_stored).map_err(|_| PreviewError::Attribute)?,
        u8::try_from(high_bit).map_err(|_| PreviewError::Attribute)?,
        representation,
    )
    .map_err(|_| PreviewError::Attribute)?;
    let layout =
        SampleLayout::new(1, photometric, None).map_err(|_| PreviewError::NotMonochrome)?;
    let dimensions = ImageDimensions::new(height, width).map_err(|_| PreviewError::Attribute)?;
    let description = StoredPixelDescription::new(dimensions, layout, stored_bits);
    let count = description
        .sample_count()
        .map_err(|_| PreviewError::Attribute)?;
    let mut stored = vec![Stored(0.0); count];
    description
        .unpack(&decoded, ByteOrder::LittleEndian, &mut stored)
        .map_err(|_| PreviewError::Decode)?;
    let mapped = lut_chain(object, photometric, &stored, request, frame_index)?;
    let mut display = vec![Display(0.0); stored.len()];
    mapped
        .chain
        .map_into(&stored, &mut display)
        .map_err(|_| PreviewError::Window)?;
    let mut grey = vec![0_u8; display.len()];
    for (cell, sample) in grey.iter_mut().zip(display.iter()) {
        let index = display_index(sample.0, 255);
        *cell = u8::try_from(index).map_err(|_| PreviewError::Window)?;
    }
    if request.invert {
        for cell in &mut grey {
            *cell = 255 - *cell;
        }
    }
    Ok(Preview {
        width,
        height,
        frames,
        full_range: mapped.full_range,
        grey,
        frame: request.frame,
        window_center: mapped.center,
        window_width: mapped.width,
        window_function: function_label(mapped.function),
        inverted: request.invert,
    })
}

fn part10_error(_error: ParseError) -> PreviewError {
    // `ParseError` is non-exhaustive. Every refusal is the same preview answer.
    PreviewError::NotPart10
}

fn photometric(object: &DefaultDicomObject) -> Result<PhotometricInterpretation, PreviewError> {
    match required_text(object, PHOTOMETRIC)?.as_str() {
        "MONOCHROME1" => Ok(PhotometricInterpretation::Monochrome1),
        "MONOCHROME2" => Ok(PhotometricInterpretation::Monochrome2),
        _ => Err(PreviewError::NotMonochrome),
    }
}

fn photometric_name(photometric: PhotometricInterpretation) -> &'static str {
    match photometric {
        PhotometricInterpretation::Monochrome1 => "MONOCHROME1",
        PhotometricInterpretation::Monochrome2 => "MONOCHROME2",
        PhotometricInterpretation::PaletteColor => "PALETTE COLOR",
        PhotometricInterpretation::Rgb => "RGB",
        PhotometricInterpretation::YbrFull => "YBR_FULL",
        PhotometricInterpretation::YbrFull422 => "YBR_FULL_422",
        PhotometricInterpretation::YbrPartial422 => "YBR_PARTIAL_422",
        PhotometricInterpretation::YbrIct => "YBR_ICT",
        PhotometricInterpretation::YbrRct => "YBR_RCT",
    }
}

fn pixel_representation(
    object: &DefaultDicomObject,
) -> Result<ocelli_pixel::PixelRepresentation, PreviewError> {
    match required_u16(object, PIXEL_REPRESENTATION)? {
        0 => Ok(ocelli_pixel::PixelRepresentation::Unsigned),
        1 => Ok(ocelli_pixel::PixelRepresentation::Signed),
        _ => Err(PreviewError::Attribute),
    }
}

fn frame_count(object: &DefaultDicomObject) -> Result<u32, PreviewError> {
    let Some(raw) = optional_text(object, NUMBER_OF_FRAMES)? else {
        return Ok(1);
    };
    let value = parse_decimal_string(&raw).ok_or(PreviewError::Attribute)?;
    whole_u32(value).ok_or(PreviewError::Attribute)
}

/// A finite whole number in `1..=u32::MAX`, compared by bits so a fraction is refused.
fn whole_u32(value: f64) -> Option<u32> {
    if !value.is_finite() || value < 1.0 {
        return None;
    }
    if value.fract().to_bits() != 0.0_f64.to_bits() {
        return None;
    }
    let mut lo = 1_u32;
    let mut hi = u32::MAX;
    while hi - lo > 1 {
        let mid = lo + (hi - lo) / 2;
        if f64::from(mid) < value {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    if f64::from(lo).to_bits() == value.to_bits() {
        Some(lo)
    } else if f64::from(hi).to_bits() == value.to_bits() {
        Some(hi)
    } else {
        None
    }
}

fn pixel_vr(object: &DefaultDicomObject) -> Result<PixelDataVr, PreviewError> {
    let element = object
        .element(PIXEL_DATA)
        .map_err(|_| PreviewError::Attribute)?;
    match element.vr() {
        VR::OB => Ok(PixelDataVr::Ob),
        VR::OW => Ok(PixelDataVr::Ow),
        _ => Err(PreviewError::Attribute),
    }
}

fn decode_frame(
    parsed: &ocelli_dicom::ParsedDicom,
    object: &DefaultDicomObject,
    desc: &FrameDesc,
    frames: u32,
    frame_index: usize,
) -> Result<Vec<u8>, PreviewError> {
    let element = object
        .element(PIXEL_DATA)
        .map_err(|_| PreviewError::Attribute)?;
    let frame_count = usize::try_from(frames).map_err(|_| PreviewError::Attribute)?;
    let mut decoded = vec![0_u8; desc.output_len()];
    match parsed.transfer_syntax().dispatch_path() {
        DispatchPath::Encapsulated => {
            decode_encapsulated(
                parsed,
                element,
                desc,
                frame_count,
                frame_index,
                &mut decoded,
            )?;
        }
        path => {
            let native = native_bytes(element, path)?;
            let decoder = native_decoder(path)?;
            let index = decoder
                .index_native_value(&native, frame_count, desc)
                .map_err(|_| PreviewError::Decode)?;
            index
                .decode_frame(frame_index, &mut decoded)
                .map_err(|_| PreviewError::Decode)?;
        }
    }
    Ok(decoded)
}

fn native_decoder(path: DispatchPath) -> Result<RawDecoder, PreviewError> {
    match path {
        DispatchPath::ImplicitVrLittleEndian => Ok(RawDecoder::implicit_vr_little_endian()),
        DispatchPath::ExplicitVrLittleEndian | DispatchPath::DeflatedExplicitVrLittleEndian => {
            Ok(RawDecoder::explicit_vr_little_endian())
        }
        DispatchPath::ExplicitVrBigEndian => Ok(RawDecoder::big_endian()),
        DispatchPath::Encapsulated => Err(PreviewError::Decode),
    }
}

fn decode_encapsulated(
    parsed: &ocelli_dicom::ParsedDicom,
    element: &InMemElement<StandardDataDictionary>,
    desc: &FrameDesc,
    frame_count: usize,
    frame_index: usize,
    decoded: &mut [u8],
) -> Result<(), PreviewError> {
    let offsets = element.value().offset_table().ok_or(PreviewError::Decode)?;
    let fragments = element.value().fragments().ok_or(PreviewError::Decode)?;
    let borrowed: Vec<&[u8]> = fragments.iter().map(Vec::as_slice).collect();
    let index = EncapsulatedFrameIndex::from_basic(frame_count, offsets, &borrowed)
        .map_err(|_| PreviewError::Decode)?;
    let parts = index
        .frame_fragments(frame_index)
        .map_err(|_| PreviewError::Decode)?;
    let mut encoded = Vec::new();
    for part in parts {
        encoded.extend_from_slice(part);
    }
    let registry = codecs()?;
    let uid = parsed.transfer_syntax().uid();
    let produced = registry
        .decode_photometric_interpretation(uid, desc)
        .map_err(|_| PreviewError::Decode)?;
    if produced != DecodePhotometricInterpretation::Preserved {
        return Err(PreviewError::NotMonochrome);
    }
    registry
        .decode(uid, &encoded, desc, decoded)
        .map_err(|_| PreviewError::Decode)
}

fn native_bytes(
    element: &InMemElement<StandardDataDictionary>,
    path: DispatchPath,
) -> Result<Vec<u8>, PreviewError> {
    match element.value() {
        DicomValue::Primitive(PrimitiveValue::U8(bytes)) => Ok(bytes.to_vec()),
        DicomValue::Primitive(PrimitiveValue::U16(words)) => {
            let mut bytes = Vec::new();
            for word in words {
                let encoded = match path {
                    DispatchPath::ExplicitVrBigEndian => word.to_be_bytes(),
                    DispatchPath::ImplicitVrLittleEndian
                    | DispatchPath::ExplicitVrLittleEndian
                    | DispatchPath::DeflatedExplicitVrLittleEndian
                    | DispatchPath::Encapsulated => word.to_le_bytes(),
                };
                bytes.extend_from_slice(&encoded);
            }
            Ok(bytes)
        }
        DicomValue::Primitive(_) | DicomValue::Sequence(_) | DicomValue::PixelSequence(_) => {
            Err(PreviewError::Decode)
        }
    }
}

fn codecs() -> Result<Registry, PreviewError> {
    let mut registry = Registry::new();
    register_native_and_rle_decoders(&mut registry).map_err(|_| PreviewError::Decode)?;
    register_jpeg_decoders(&mut registry).map_err(|_| PreviewError::Decode)?;
    register_jpeg2000_decoders(&mut registry).map_err(|_| PreviewError::Decode)?;
    register_jpegls_decoders(&mut registry).map_err(|_| PreviewError::Decode)?;
    register_htj2k_decoders(&mut registry).map_err(|_| PreviewError::Decode)?;
    Ok(registry)
}

struct Mapped {
    chain: LutChain,
    full_range: bool,
    center: f32,
    width: f32,
    function: VoiFunction,
}

fn lut_chain(
    object: &DefaultDicomObject,
    photometric: PhotometricInterpretation,
    stored: &[Stored],
    request: FrameRequest,
    frame: usize,
) -> Result<Mapped, PreviewError> {
    let metadata = project_metadata(object)?;
    let multi = MultiframeMetadata::new(&metadata).map_err(|_| PreviewError::Attribute)?;
    // PS3.3 C.11.1: a Modality LUT Sequence replaces rescale. Passing `None`
    // into `ModalityTransform` would draw the identity slope instead.
    if group_has(
        &multi,
        frame,
        PIXEL_VALUE_TRANSFORMATION,
        MODALITY_LUT_SEQUENCE,
    )? {
        return Err(PreviewError::Window);
    }
    let (slope, intercept) = rescale(&multi, frame)?;
    let modality = ModalityTransform::new(None, Some(slope), Some(intercept))
        .map_err(|_| PreviewError::Window)?;
    // A request that names both center and width replaces the file window
    // and a VOI LUT Sequence. The file window is not read in that case.
    let (centers, widths, function, full_range) = if request.window_center.is_some()
        && request.window_width.is_some()
    {
        let center = request.window_center.ok_or(PreviewError::Window)?;
        let width = request.window_width.ok_or(PreviewError::Window)?;
        if !center.is_finite() || !width.is_finite() {
            return Err(PreviewError::Window);
        }
        let function = request.window_function.unwrap_or(VoiFunction::Linear);
        (vec![center], vec![width], function, false)
    } else {
        // PS3.3 C.11.2: a VOI LUT Sequence takes precedence over the file window.
        if group_has(&multi, frame, FRAME_VOI_LUT_SEQUENCE, VOI_LUT_SEQUENCE)? {
            return Err(PreviewError::Window);
        }
        let (centers, widths, mut function, full_range) = window(&multi, frame, &modality, stored)?;
        if let Some(choice) = request.window_function {
            function = choice;
        }
        (centers, widths, function, full_range)
    };
    let center = centers.first().copied().ok_or(PreviewError::Window)?;
    let width = widths.first().copied().ok_or(PreviewError::Window)?;
    let evidence = presentation(object)?;
    let voi = VoiTransform::new(None, &centers, &widths, 0, function, 0.0, 255.0)
        .map_err(|_| PreviewError::Window)?;
    let chain =
        LutChain::new(modality, voi, photometric, evidence).map_err(|_| PreviewError::Window)?;
    Ok(Mapped {
        chain,
        full_range,
        center,
        width,
        function,
    })
}

fn project_metadata(object: &DefaultDicomObject) -> Result<MetadataSet, PreviewError> {
    project_elements(object.iter())
}

fn project_item(
    object: &InMemDicomObject<StandardDataDictionary>,
) -> Result<MetadataSet, PreviewError> {
    project_elements(object.iter())
}

fn project_elements<'a>(
    elements: impl Iterator<Item = &'a InMemElement<StandardDataDictionary>>,
) -> Result<MetadataSet, PreviewError> {
    let mut metadata = MetadataSet::new();
    for element in elements {
        let tag = element.header().tag;
        if tag == PIXEL_DATA {
            continue;
        }
        let projected = match element.value() {
            DicomValue::Primitive(value) => MetadataElement::from_primitive(element.vr(), value),
            DicomValue::Sequence(sequence) => {
                let mut items = Vec::new();
                for item in sequence.items() {
                    items.push(project_item(item)?);
                }
                MetadataElement::sequence(items)
            }
            DicomValue::PixelSequence(_) => continue,
        };
        metadata
            .insert(tag, projected)
            .map_err(|_| PreviewError::Attribute)?;
    }
    Ok(metadata)
}

fn group_has(
    multi: &MultiframeMetadata<'_>,
    frame: usize,
    group: Tag,
    attribute: Tag,
) -> Result<bool, PreviewError> {
    Ok(multi
        .resolve(frame, group, attribute, TopLevelFallback::Allowed)
        .map_err(|_| PreviewError::Attribute)?
        .is_some())
}

fn resolved_decimals(
    multi: &MultiframeMetadata<'_>,
    frame: usize,
    group: Tag,
    attribute: Tag,
) -> Result<Vec<f32>, PreviewError> {
    let found = multi
        .resolve(frame, group, attribute, TopLevelFallback::Allowed)
        .map_err(|_| PreviewError::Attribute)?;
    let Some(found) = found else {
        return Ok(Vec::new());
    };
    // A zero-length element is present and carries no number. It means the
    // same as an absent attribute. Any other non-text value is refused.
    if matches!(found.element().value(), MetadataValue::Empty) {
        return Ok(Vec::new());
    }
    let Some(parts) = found.element().semantic_text() else {
        return Err(PreviewError::Attribute);
    };
    let mut values = Vec::new();
    for part in parts {
        if part.is_empty() {
            continue;
        }
        let parsed = parse_decimal_string(part).ok_or(PreviewError::Attribute)?;
        let narrow = f32_from_f64(parsed);
        if !narrow.is_finite() {
            return Err(PreviewError::Attribute);
        }
        values.push(narrow);
    }
    Ok(values)
}

fn resolved_text(
    multi: &MultiframeMetadata<'_>,
    frame: usize,
    group: Tag,
    attribute: Tag,
) -> Result<Option<String>, PreviewError> {
    let found = multi
        .resolve(frame, group, attribute, TopLevelFallback::Allowed)
        .map_err(|_| PreviewError::Attribute)?;
    let Some(found) = found else {
        return Ok(None);
    };
    // A zero-length element is present and carries no token. It means the
    // same as an absent attribute. Any other non-text value is refused.
    if matches!(found.element().value(), MetadataValue::Empty) {
        return Ok(None);
    }
    let Some(parts) = found.element().semantic_text() else {
        return Err(PreviewError::Attribute);
    };
    Ok(parts
        .into_iter()
        .find(|part| !part.is_empty())
        .map(str::to_owned))
}

fn function_label(function: VoiFunction) -> &'static str {
    match function {
        VoiFunction::Linear => "LINEAR",
        VoiFunction::LinearExact => "LINEAR_EXACT",
        VoiFunction::Sigmoid => "SIGMOID",
    }
}

fn rescale(multi: &MultiframeMetadata<'_>, frame: usize) -> Result<(f32, f32), PreviewError> {
    let slope = resolved_decimals(multi, frame, PIXEL_VALUE_TRANSFORMATION, RESCALE_SLOPE)?;
    let intercept = resolved_decimals(multi, frame, PIXEL_VALUE_TRANSFORMATION, RESCALE_INTERCEPT)?;
    match (slope.first().copied(), intercept.first().copied()) {
        (None, None) => Ok((1.0, 0.0)),
        (Some(slope), Some(intercept)) => {
            if slope.is_finite() && intercept.is_finite() {
                Ok((slope, intercept))
            } else {
                Err(PreviewError::Window)
            }
        }
        _ => Err(PreviewError::Window),
    }
}

fn window(
    multi: &MultiframeMetadata<'_>,
    frame: usize,
    modality: &ModalityTransform,
    stored: &[Stored],
) -> Result<(Vec<f32>, Vec<f32>, VoiFunction, bool), PreviewError> {
    let centers = resolved_decimals(multi, frame, FRAME_VOI_LUT_SEQUENCE, WINDOW_CENTER)?;
    let widths = resolved_decimals(multi, frame, FRAME_VOI_LUT_SEQUENCE, WINDOW_WIDTH)?;
    if centers.is_empty() && widths.is_empty() {
        let (center, width) = full_range(modality, stored)?;
        return Ok((vec![center], vec![width], VoiFunction::LinearExact, true));
    }
    if centers.is_empty() || centers.len() != widths.len() {
        return Err(PreviewError::Window);
    }
    let function = voi_function(multi, frame)?;
    Ok((centers, widths, function, false))
}

fn full_range(modality: &ModalityTransform, stored: &[Stored]) -> Result<(f32, f32), PreviewError> {
    let mut low: Option<f32> = None;
    let mut high: Option<f32> = None;
    for sample in stored {
        let value = modality.apply(*sample).0;
        if !value.is_finite() {
            return Err(PreviewError::Window);
        }
        low = Some(match low {
            Some(held) if held < value => held,
            Some(_) | None => value,
        });
        high = Some(match high {
            Some(held) if held > value => held,
            Some(_) | None => value,
        });
    }
    let (Some(low), Some(high)) = (low, high) else {
        return Err(PreviewError::Window);
    };
    let span = high - low;
    if !span.is_finite() {
        return Err(PreviewError::Window);
    }
    if span.to_bits() == 0.0_f32.to_bits() {
        return Ok((low, 1.0));
    }
    let center = (low + high) / 2.0;
    if !center.is_finite() {
        return Err(PreviewError::Window);
    }
    Ok((center, span))
}

fn voi_function(multi: &MultiframeMetadata<'_>, frame: usize) -> Result<VoiFunction, PreviewError> {
    match resolved_text(multi, frame, FRAME_VOI_LUT_SEQUENCE, VOI_FUNCTION)? {
        None => Ok(VoiFunction::Linear),
        Some(text) if text == "LINEAR" => Ok(VoiFunction::Linear),
        Some(text) if text == "LINEAR_EXACT" => Ok(VoiFunction::LinearExact),
        Some(text) if text == "SIGMOID" => Ok(VoiFunction::Sigmoid),
        Some(_) => Err(PreviewError::Window),
    }
}

fn presentation(object: &DefaultDicomObject) -> Result<PresentationLutEvidence, PreviewError> {
    if object.element(PRESENTATION_SEQUENCE).is_ok() {
        return Ok(PresentationLutEvidence::Sequence);
    }
    match optional_text(object, PRESENTATION_SHAPE)? {
        None => Ok(PresentationLutEvidence::Absent),
        Some(text) if text == "IDENTITY" => Ok(PresentationLutEvidence::Shape(
            PresentationLutShape::Identity,
        )),
        Some(text) if text == "INVERSE" => Ok(PresentationLutEvidence::Shape(
            PresentationLutShape::Inverse,
        )),
        Some(_) => Err(PreviewError::Window),
    }
}

fn required_u16(object: &DefaultDicomObject, tag: Tag) -> Result<u16, PreviewError> {
    object
        .element(tag)
        .map_err(|_| PreviewError::Attribute)?
        .to_int::<u16>()
        .map_err(|_| PreviewError::Attribute)
}

fn required_text(object: &DefaultDicomObject, tag: Tag) -> Result<String, PreviewError> {
    optional_text(object, tag)?.ok_or(PreviewError::Attribute)
}

fn optional_text(object: &DefaultDicomObject, tag: Tag) -> Result<Option<String>, PreviewError> {
    let Ok(element) = object.element(tag) else {
        return Ok(None);
    };
    let raw = element.to_str().map_err(|_| PreviewError::Attribute)?;
    let text = raw.trim_matches(' ').trim_end_matches('\0');
    if text.is_empty() {
        Ok(None)
    } else {
        Ok(Some(text.to_owned()))
    }
}

/// Read the VOI LUT Function token used by the page.
///
/// # Errors
///
/// Returns [`PreviewError::Window`] when the token is not one of the three
/// functions in PS3.3 C.11.2.
pub fn window_function_from_token(text: &str) -> Result<VoiFunction, PreviewError> {
    match text {
        "LINEAR" => Ok(VoiFunction::Linear),
        "LINEAR_EXACT" => Ok(VoiFunction::LinearExact),
        "SIGMOID" => Ok(VoiFunction::Sigmoid),
        _ => Err(PreviewError::Window),
    }
}

/// Series and image attributes for the left-hand list and the corner text.
///
/// The JSON holds the values stored in the file. The caller must not write it
/// to a log.
///
/// # Errors
///
/// Returns [`PreviewError::NotPart10`] when the bytes are not a Part 10 object.
pub fn inspect_json(bytes: &[u8]) -> Result<String, PreviewError> {
    let parsed = parse_part10(bytes).map_err(part10_error)?;
    let object = parsed.object();
    let mut sop = text_or_empty(object, SOP_INSTANCE)?;
    if sop.is_empty() {
        sop = object
            .meta()
            .media_storage_sop_instance_uid()
            .trim_end_matches('\0')
            .to_owned();
    }
    let photometric_text = text_or_empty(object, PHOTOMETRIC)?;
    let samples = optional_u32(object, SAMPLES_PER_PIXEL)?;
    let rows = optional_u32(object, ROWS)?;
    let columns = optional_u32(object, COLUMNS)?;
    let frames = frame_count(object).unwrap_or(0);
    let monochrome = (photometric_text == "MONOCHROME1" || photometric_text == "MONOCHROME2")
        && samples == 1
        && rows > 0
        && columns > 0
        && frames >= 1;
    let mut out = String::from("{");
    push_field(
        &mut out,
        "seriesUid",
        &text_or_empty(object, SERIES_UID)?,
        true,
    );
    push_field(
        &mut out,
        "seriesNumber",
        &text_or_empty(object, SERIES_NUMBER)?,
        false,
    );
    push_field(
        &mut out,
        "modality",
        &text_or_empty(object, MODALITY)?,
        false,
    );
    push_field(
        &mut out,
        "seriesDescription",
        &text_or_empty(object, SERIES_DESCRIPTION)?,
        false,
    );
    push_field(
        &mut out,
        "instanceNumber",
        &text_or_empty(object, INSTANCE_NUMBER)?,
        false,
    );
    push_field(&mut out, "sopInstanceUid", &sop, false);
    push_field(
        &mut out,
        "patientName",
        &text_or_empty(object, PATIENT_NAME)?,
        false,
    );
    push_field(
        &mut out,
        "patientId",
        &text_or_empty(object, PATIENT_ID)?,
        false,
    );
    push_field(
        &mut out,
        "studyDate",
        &text_or_empty(object, STUDY_DATE)?,
        false,
    );
    push_field(
        &mut out,
        "studyDescription",
        &text_or_empty(object, STUDY_DESCRIPTION)?,
        false,
    );
    push_field(
        &mut out,
        "accession",
        &text_or_empty(object, ACCESSION)?,
        false,
    );
    push_number(&mut out, "rows", rows);
    push_number(&mut out, "columns", columns);
    push_number(&mut out, "samples", samples);
    push_field(&mut out, "photometric", &photometric_text, false);
    push_number(&mut out, "frames", frames);
    push_field(
        &mut out,
        "windowCenter",
        &text_or_empty(object, WINDOW_CENTER)?,
        false,
    );
    push_field(
        &mut out,
        "windowWidth",
        &text_or_empty(object, WINDOW_WIDTH)?,
        false,
    );
    push_field(
        &mut out,
        "windowFunction",
        &text_or_empty(object, VOI_FUNCTION)?,
        false,
    );
    out.push_str(",\"monochrome\":");
    out.push_str(if monochrome { "true" } else { "false" });
    out.push('}');
    Ok(out)
}

/// File meta and data set rows for the headers table.
///
/// Pixel data is reported as a byte count. Other values are the stored text.
/// The caller must not write the JSON to a log.
///
/// # Errors
///
/// Returns [`PreviewError::NotPart10`] when the bytes are not a Part 10 object.
pub fn headers_json(bytes: &[u8]) -> Result<String, PreviewError> {
    let parsed = parse_part10(bytes).map_err(part10_error)?;
    let object = parsed.object();
    let dict = StandardDataDictionary;
    let mut rows = Vec::new();
    for element in object.meta().to_element_iter() {
        if capped(&mut rows) {
            break;
        }
        push_row(
            &mut rows,
            &dict,
            element.tag(),
            element.vr(),
            "",
            meta_text(&element),
        );
    }
    for element in object.iter() {
        push_dataset(&mut rows, &dict, element, "", 0);
    }
    Ok(rows_json(&rows))
}

struct HeaderRow {
    tag: String,
    vr: String,
    name: String,
    value: String,
}

fn text_or_empty(object: &DefaultDicomObject, tag: Tag) -> Result<String, PreviewError> {
    Ok(optional_text(object, tag)?.unwrap_or_default())
}

fn optional_u32(object: &DefaultDicomObject, tag: Tag) -> Result<u32, PreviewError> {
    let Ok(element) = object.element(tag) else {
        return Ok(0);
    };
    let value = element
        .to_int::<u16>()
        .map_err(|_| PreviewError::Attribute)?;
    Ok(u32::from(value))
}

fn push_field(out: &mut String, name: &str, value: &str, first: bool) {
    if !first {
        out.push(',');
    }
    out.push('"');
    out.push_str(name);
    out.push_str("\":");
    json_string(out, value);
}

fn push_number(out: &mut String, name: &str, value: u32) {
    out.push_str(",\"");
    out.push_str(name);
    out.push_str("\":");
    out.push_str(&value.to_string());
}

fn json_string(out: &mut String, text: &str) {
    out.push('"');
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            control if control.is_control() => {
                out.push_str(&format!("\\u{code:04x}", code = u32::from(control)));
            }
            other => out.push(other),
        }
    }
    out.push('"');
}

fn rows_json(rows: &[HeaderRow]) -> String {
    let mut out = String::from("[");
    for (index, row) in rows.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        out.push_str("{\"tag\":");
        json_string(&mut out, &row.tag);
        out.push_str(",\"vr\":");
        json_string(&mut out, &row.vr);
        out.push_str(",\"name\":");
        json_string(&mut out, &row.name);
        out.push_str(",\"value\":");
        json_string(&mut out, &row.value);
        out.push('}');
    }
    out.push(']');
    out
}

fn push_dataset(
    rows: &mut Vec<HeaderRow>,
    dict: &StandardDataDictionary,
    element: &InMemElement<StandardDataDictionary>,
    prefix: &str,
    depth: u32,
) {
    if capped(rows) {
        return;
    }
    let shown = format!("{prefix}{}", tag_text(element.tag()));
    push_row(
        rows,
        dict,
        element.tag(),
        element.vr(),
        &shown,
        dataset_text(element),
    );
    if depth >= 6 {
        return;
    }
    let Some(items) = element.value().items() else {
        return;
    };
    for (index, item) in items.iter().enumerate() {
        let next = format!("{shown}[{index}].");
        for child in item.iter() {
            push_dataset(rows, dict, child, &next, depth + 1);
        }
    }
}

fn push_row(
    rows: &mut Vec<HeaderRow>,
    dict: &StandardDataDictionary,
    tag: Tag,
    vr: VR,
    shown: &str,
    value: String,
) {
    if capped(rows) {
        return;
    }
    let label = if shown.is_empty() {
        tag_text(tag)
    } else {
        shown.to_owned()
    };
    rows.push(HeaderRow {
        tag: label,
        vr: VR::to_string(vr).to_owned(),
        name: dict
            .by_tag(tag)
            .map(|entry| entry.alias().to_owned())
            .unwrap_or_default(),
        value: clip_text(&value),
    });
}

fn capped(rows: &mut Vec<HeaderRow>) -> bool {
    if rows.len() < MAX_HEADER_ROWS {
        return false;
    }
    let already = rows
        .last()
        .is_some_and(|row| row.value == "further elements omitted");
    if !already {
        rows.push(HeaderRow {
            tag: String::new(),
            vr: String::new(),
            name: String::new(),
            value: "further elements omitted".to_owned(),
        });
    }
    true
}

fn tag_text(tag: Tag) -> String {
    format!(
        "({group:04X},{element:04X})",
        group = tag.group(),
        element = tag.element()
    )
}

fn clip_text(text: &str) -> String {
    if text.chars().count() <= VALUE_CAP {
        return text.to_owned();
    }
    let mut kept: String = text.chars().take(VALUE_CAP).collect();
    kept.push_str(" [truncated]");
    kept
}

fn meta_text(element: &DataElement<EmptyObject, [u8; 0]>) -> String {
    if element.tag() == PIXEL_DATA {
        return primitive_note(element.value(), true);
    }
    if let Some(note) = bulk_if_large(element.vr(), element.value()) {
        return note;
    }
    match element.to_str() {
        Ok(text) => text.into_owned(),
        Err(_) => primitive_note(element.value(), true),
    }
}

fn dataset_text(element: &InMemElement<StandardDataDictionary>) -> String {
    if element.tag() == PIXEL_DATA {
        return pixel_note(element);
    }
    if element.vr() == VR::SQ {
        let count = element
            .value()
            .items()
            .map(|items| items.len())
            .unwrap_or(0);
        return format!("{count} items");
    }
    if let DicomValue::Primitive(value) = element.value()
        && let Some(note) = bulk_if_large_primitive(element.vr(), value)
    {
        return note;
    }
    match element.to_str() {
        Ok(text) => text.into_owned(),
        Err(_) => pixel_note(element),
    }
}

fn bulk_if_large(vr: VR, value: &DicomValue<EmptyObject, [u8; 0]>) -> Option<String> {
    let DicomValue::Primitive(primitive) = value else {
        return None;
    };
    bulk_if_large_primitive(vr, primitive)
}

fn bulk_if_large_primitive(vr: VR, value: &PrimitiveValue) -> Option<String> {
    match vr {
        VR::OB | VR::OW | VR::OF | VR::OD | VR::OL | VR::OV | VR::UN => {
            let bytes = primitive_len(value)?;
            if bytes > BULK_TEXT_CAP {
                Some(format!("{bytes} bytes"))
            } else {
                None
            }
        }
        _ => None,
    }
}

fn primitive_note(value: &DicomValue<EmptyObject, [u8; 0]>, force: bool) -> String {
    let DicomValue::Primitive(primitive) = value else {
        return "0 bytes".to_owned();
    };
    match primitive_len(primitive) {
        Some(bytes) if force || bytes > BULK_TEXT_CAP => format!("{bytes} bytes"),
        Some(_) => primitive.to_str().into_owned(),
        None => "value omitted".to_owned(),
    }
}

fn pixel_note(element: &InMemElement<StandardDataDictionary>) -> String {
    if let Some(fragments) = element.value().fragments() {
        let mut total = 0_usize;
        for fragment in fragments {
            match total.checked_add(fragment.len()) {
                Some(sum) => total = sum,
                None => return "byte length overflowed".to_owned(),
            }
        }
        return format!("{total} bytes");
    }
    if let DicomValue::Primitive(value) = element.value() {
        match primitive_len(value) {
            Some(bytes) => format!("{bytes} bytes"),
            None => "value omitted".to_owned(),
        }
    } else {
        "0 bytes".to_owned()
    }
}

fn primitive_len(value: &PrimitiveValue) -> Option<usize> {
    match value {
        PrimitiveValue::Empty => Some(0),
        PrimitiveValue::U8(items) => Some(items.len()),
        PrimitiveValue::U16(items) => items.len().checked_mul(2),
        PrimitiveValue::I16(items) => items.len().checked_mul(2),
        PrimitiveValue::U32(items) => items.len().checked_mul(4),
        PrimitiveValue::I32(items) => items.len().checked_mul(4),
        PrimitiveValue::F32(items) => items.len().checked_mul(4),
        PrimitiveValue::U64(items) => items.len().checked_mul(8),
        PrimitiveValue::I64(items) => items.len().checked_mul(8),
        PrimitiveValue::F64(items) => items.len().checked_mul(8),
        PrimitiveValue::Str(_)
        | PrimitiveValue::Strs(_)
        | PrimitiveValue::Tags(_)
        | PrimitiveValue::Date(_)
        | PrimitiveValue::DateTime(_)
        | PrimitiveValue::Time(_) => None,
    }
}
