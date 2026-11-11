//! JPEG 2000 lossless-only and general transfer-syntax adapters.

use std::sync::Arc;

use ritk_codecs::{
    PixelLayout, PixelSignedness, decode_jpeg2000_fragment, decode_jpeg2000_fragment_colour,
};

use crate::{
    Capability, CodecError, DecodePhotometricInterpretation, DecodeSampleLayout, Decoder,
    FrameDesc, PixelDataVr, PixelRepresentation, Registry, RegistryError,
    sample_convert::convert_samples,
};

const JPEG2000_LOSSLESS_UID: &str = "1.2.840.10008.1.2.4.90";
const JPEG2000_UID: &str = "1.2.840.10008.1.2.4.91";
const LOSSLESS_UIDS: &[&str] = &[JPEG2000_LOSSLESS_UID];
const GENERAL_UIDS: &[&str] = &[JPEG2000_UID];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    LosslessOnly,
    General,
}

/// One exact DICOM JPEG 2000 transfer-syntax adapter.
pub struct Jpeg2000Decoder {
    mode: Mode,
}

impl Jpeg2000Decoder {
    /// Decoder for lossless-only JPEG 2000 UID `.90`.
    #[must_use]
    pub const fn lossless_only() -> Self {
        Self {
            mode: Mode::LosslessOnly,
        }
    }

    /// Decoder for general JPEG 2000 UID `.91`.
    #[must_use]
    pub const fn general() -> Self {
        Self {
            mode: Mode::General,
        }
    }
}

/// Register both JPEG 2000 adapters atomically.
///
/// # Errors
///
/// Returns the first registry collision or declaration error. Preflight occurs
/// before either registration, so an error leaves both capabilities unchanged.
pub fn register_jpeg2000_decoders(registry: &mut Registry) -> Result<(), RegistryError> {
    for uid in [JPEG2000_LOSSLESS_UID, JPEG2000_UID] {
        match registry.capability(uid) {
            Capability::Available => return Err(RegistryError::AlreadyRegistered { uid }),
            Capability::KnownUnavailable => {}
            Capability::Unknown => return Err(RegistryError::UnknownTransferSyntax { uid }),
        }
    }
    registry.register(Arc::new(Jpeg2000Decoder::lossless_only()))?;
    registry.register(Arc::new(Jpeg2000Decoder::general()))
}

impl Decoder for Jpeg2000Decoder {
    fn transfer_syntaxes(&self) -> &'static [&'static str] {
        match self.mode {
            Mode::LosslessOnly => LOSSLESS_UIDS,
            Mode::General => GENERAL_UIDS,
        }
    }

    fn decode_photometric_interpretation(
        &self,
        desc: &FrameDesc,
    ) -> DecodePhotometricInterpretation {
        decoded_photometric(desc)
    }

    fn decode_sample_layout(&self, _desc: &FrameDesc) -> DecodeSampleLayout {
        DecodeSampleLayout::Interleaved
    }

    fn decode(&self, src: &[u8], desc: &FrameDesc, out: &mut [u8]) -> Result<(), CodecError> {
        if out.len() != desc.output_len() {
            return Err(CodecError::OutputLength {
                expected: desc.output_len(),
                actual: out.len(),
            });
        }
        validate_pixel_descriptor(desc)?;
        let codestream = logical_codestream(src)?;
        let (header, tile_at) = inspect_main_header(codestream)?;
        if self.mode == Mode::LosslessOnly {
            refuse_tile_coding_overrides(codestream, tile_at, false)?;
        }
        validate_header(self.mode, header, desc)?;

        let signedness = match desc.pixel_representation() {
            PixelRepresentation::Unsigned => PixelSignedness::Unsigned,
            PixelRepresentation::Signed => PixelSignedness::Signed,
        };
        let decoded = if desc.samples_per_pixel() == 3 {
            // Three-component output is interleaved RGB or, with the
            // transform off, the codestream's own three components.
            let layout = PixelLayout {
                rows: usize::from(desc.rows()),
                cols: usize::from(desc.columns()),
                samples_per_pixel: 3,
                bits_allocated: desc.bits_allocated(),
                pixel_representation: signedness,
                rescale_slope: 1.0,
                rescale_intercept: 0.0,
            };
            decode_jpeg2000_fragment_colour(codestream, layout)
                .map_err(|_| CodecError::DecoderFailure)?
        } else {
            let layout = PixelLayout {
                rows: usize::from(desc.rows()),
                cols: usize::from(desc.columns()),
                samples_per_pixel: 1,
                bits_allocated: desc.bits_allocated(),
                pixel_representation: signedness,
                rescale_slope: 1.0,
                rescale_intercept: 0.0,
            };
            decode_jpeg2000_fragment(codestream, layout).map_err(|_| CodecError::DecoderFailure)?
        };
        convert_samples(&decoded, desc, out)
    }
}

#[derive(Clone)]
struct MainHeader {
    width: u32,
    height: u32,
    components: Vec<ComponentParams>,
    mct: u8,
    transform: u8,
    quantization_style: u8,
}

pub(crate) fn logical_codestream(src: &[u8]) -> Result<&[u8], CodecError> {
    if src.len() < 4 || !src.starts_with(&[0xff, 0x4f]) {
        return Err(CodecError::InvalidCodestream);
    }
    if src.ends_with(&[0xff, 0xd9]) {
        return Ok(src);
    }
    if src.ends_with(&[0xff, 0xd9, 0]) && !(src.len() - 1).is_multiple_of(2) {
        return src
            .get(..src.len() - 1)
            .ok_or(CodecError::InvalidCodestream);
    }
    if src.windows(2).any(|window| window == [0xff, 0xd9]) {
        return Err(CodecError::TrailingData);
    }
    Err(CodecError::InvalidCodestream)
}

fn inspect_main_header(src: &[u8]) -> Result<(MainHeader, usize), CodecError> {
    let mut offset = 2usize;
    let mut siz = None;
    let mut cod = None;
    let mut qcd = None;
    while offset.checked_add(2).is_some_and(|end| end <= src.len()) {
        if src.get(offset) != Some(&0xff) {
            return Err(CodecError::InvalidCodestream);
        }
        let marker = *src.get(offset + 1).ok_or(CodecError::InvalidCodestream)?;
        if marker == 0x90 {
            break;
        }
        if marker == 0xd9 || marker == 0x4f {
            return Err(CodecError::InvalidCodestream);
        }
        let length = usize::from(read_u16(src, offset + 2)?);
        if length < 2 {
            return Err(CodecError::InvalidCodestream);
        }
        let end = offset
            .checked_add(2)
            .and_then(|v| v.checked_add(length))
            .ok_or(CodecError::InvalidCodestream)?;
        let payload = src
            .get(offset + 4..end)
            .ok_or(CodecError::InvalidCodestream)?;
        match marker {
            0x51 => {
                if siz.is_some() {
                    return Err(CodecError::InvalidCodestream);
                }
                siz = Some(parse_siz(payload)?);
            }
            0x52 => {
                if cod.is_some() || payload.len() < 10 {
                    return Err(CodecError::InvalidCodestream);
                }
                cod = Some((
                    *payload.get(4).ok_or(CodecError::InvalidCodestream)?,
                    *payload.get(5).ok_or(CodecError::InvalidCodestream)?,
                    *payload.get(9).ok_or(CodecError::InvalidCodestream)?,
                ));
            }
            0x5c => {
                if qcd.is_some() {
                    return Err(CodecError::InvalidCodestream);
                }
                qcd = Some(parse_qcd(payload)?);
            }
            // COC and QCC override the main-header wavelet or quantizer for one
            // component. A lossless UID cannot accept either.
            0x53 | 0x5d => return Err(CodecError::UnsupportedPixelFormat),
            _ => {}
        }
        offset = end;
    }
    let (width, height, components) = siz.ok_or(CodecError::InvalidCodestream)?;
    let (mct, decomposition_levels, transform) = cod.ok_or(CodecError::InvalidCodestream)?;
    let (quantization_style, quantization_entries) = qcd.ok_or(CodecError::InvalidCodestream)?;
    validate_qcd(
        quantization_style,
        quantization_entries,
        decomposition_levels,
    )?;
    Ok((
        MainHeader {
            width,
            height,
            components,
            mct,
            transform,
            quantization_style,
        },
        offset,
    ))
}

/// Refuse a tile-part COD, COC, QCD or QCC.
///
/// `offset` is the first SOT. ISO/IEC 15444-1 lets that tile-part header
/// replace the main-header wavelet and quantizer. Lossless-only syntaxes
/// cannot accept the replacement. `Psot` skips the packet bytes, which are
/// not a marker list.
pub(crate) fn refuse_tile_coding_overrides(
    src: &[u8],
    mut offset: usize,
    refuse_progression_change: bool,
) -> Result<(), CodecError> {
    while offset.checked_add(2).is_some_and(|end| end <= src.len()) {
        if src.get(offset) != Some(&0xff) {
            return Err(CodecError::InvalidCodestream);
        }
        let marker = *src.get(offset + 1).ok_or(CodecError::InvalidCodestream)?;
        if marker == 0xd9 {
            return Ok(());
        }
        if marker != 0x90 {
            return Err(CodecError::InvalidCodestream);
        }
        let length = usize::from(read_u16(src, offset + 2)?);
        if length < 10 {
            return Err(CodecError::InvalidCodestream);
        }
        let tile_bytes = usize::try_from(read_u32(src, offset + 6)?)
            .map_err(|_| CodecError::InvalidCodestream)?;
        let header_at = offset
            .checked_add(2)
            .and_then(|start| start.checked_add(length))
            .ok_or(CodecError::InvalidCodestream)?;
        walk_tile_header(src, header_at, refuse_progression_change)?;
        if tile_bytes == 0 {
            return Ok(());
        }
        let next = offset
            .checked_add(tile_bytes)
            .ok_or(CodecError::InvalidCodestream)?;
        if next <= header_at || next > src.len() {
            return Err(CodecError::InvalidCodestream);
        }
        offset = next;
    }
    Ok(())
}

fn walk_tile_header(
    src: &[u8],
    mut offset: usize,
    refuse_progression_change: bool,
) -> Result<(), CodecError> {
    loop {
        if src.get(offset) != Some(&0xff) {
            return Err(CodecError::InvalidCodestream);
        }
        let marker = *src.get(offset + 1).ok_or(CodecError::InvalidCodestream)?;
        if marker == 0x93 {
            return Ok(());
        }
        if matches!(marker, 0x52 | 0x53 | 0x5c | 0x5d) {
            return Err(CodecError::UnsupportedPixelFormat);
        }
        // POC replaces the COD progression order. Only a caller that requires
        // one progression, currently HTJ2K `.202`, asks for this refusal.
        if marker == 0x5f && refuse_progression_change {
            return Err(CodecError::FrameMismatch);
        }
        if (0x30..=0x3f).contains(&marker) || matches!(marker, 0x4f | 0x90 | 0xd9) {
            return Err(CodecError::InvalidCodestream);
        }
        let length = usize::from(read_u16(src, offset + 2)?);
        if length < 2 {
            return Err(CodecError::InvalidCodestream);
        }
        offset = offset
            .checked_add(2)
            .and_then(|start| start.checked_add(length))
            .ok_or(CodecError::InvalidCodestream)?;
    }
}

pub(crate) fn parse_qcd(payload: &[u8]) -> Result<(u8, usize), CodecError> {
    let sqcd = *payload.first().ok_or(CodecError::InvalidCodestream)?;
    let style = sqcd & 0x1f;
    if !matches!(style, 0..=2) {
        return Err(CodecError::InvalidCodestream);
    }
    Ok((style, payload.len() - 1))
}

fn validate_qcd(style: u8, entries: usize, decomposition_levels: u8) -> Result<(), CodecError> {
    let subbands = usize::from(decomposition_levels)
        .checked_mul(3)
        .and_then(|count| count.checked_add(1))
        .ok_or(CodecError::InvalidCodestream)?;
    let expected = match style {
        0 => subbands,
        1 => 2,
        2 => subbands
            .checked_mul(2)
            .ok_or(CodecError::InvalidCodestream)?,
        _ => return Err(CodecError::InvalidCodestream),
    };
    if entries != expected {
        return Err(CodecError::InvalidCodestream);
    }
    Ok(())
}

/// One component's parameters from SIZ, ISO/IEC 15444-1 A.5.1.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ComponentParams {
    /// `(Ssiz & 0x7f) + 1`, the component's bit depth.
    pub(crate) precision: u8,
    /// Whether Ssiz's sign bit is set.
    pub(crate) signed: bool,
    /// XRsiz, the horizontal sub-sampling factor.
    pub(crate) x_sampling: u8,
    /// YRsiz, the vertical sub-sampling factor.
    pub(crate) y_sampling: u8,
}

pub(crate) fn parse_siz(payload: &[u8]) -> Result<(u32, u32, Vec<ComponentParams>), CodecError> {
    if payload.len() < 39 {
        return Err(CodecError::InvalidCodestream);
    }
    let xsiz = read_u32(payload, 2)?;
    let ysiz = read_u32(payload, 6)?;
    let xosiz = read_u32(payload, 10)?;
    let yosiz = read_u32(payload, 14)?;
    let components = read_u16(payload, 34)?;
    let expected = 36usize
        .checked_add(
            usize::from(components)
                .checked_mul(3)
                .ok_or(CodecError::InvalidCodestream)?,
        )
        .ok_or(CodecError::InvalidCodestream)?;
    if payload.len() != expected || components == 0 || xsiz <= xosiz || ysiz <= yosiz {
        return Err(CodecError::InvalidCodestream);
    }
    let mut component_params = Vec::with_capacity(usize::from(components));
    for index in 0..usize::from(components) {
        let ssiz = *payload
            .get(36 + index * 3)
            .ok_or(CodecError::InvalidCodestream)?;
        let x_sampling = *payload
            .get(37 + index * 3)
            .ok_or(CodecError::InvalidCodestream)?;
        let y_sampling = *payload
            .get(38 + index * 3)
            .ok_or(CodecError::InvalidCodestream)?;
        component_params.push(ComponentParams {
            precision: (ssiz & 0x7f) + 1,
            signed: ssiz & 0x80 != 0,
            x_sampling,
            y_sampling,
        });
    }
    Ok((xsiz - xosiz, ysiz - yosiz, component_params))
}

fn validate_header(mode: Mode, header: MainHeader, desc: &FrameDesc) -> Result<(), CodecError> {
    validate_colour_shape(&header.components, header.mct, desc)?;
    if header.components.len() == 3 {
        validate_colour_components(&header.components)?;
    }
    let first = first_component(&header.components)?;
    let signed = desc.pixel_representation() == PixelRepresentation::Signed;
    if header.width != u32::from(desc.columns())
        || header.height != u32::from(desc.rows())
        || u16::from(first.precision) != desc.bits_stored()
        || first.signed != signed
    {
        return Err(CodecError::FrameMismatch);
    }
    if !matches!(header.transform, 0 | 1) {
        return Err(CodecError::InvalidCodestream);
    }
    if mode == Mode::LosslessOnly && header.transform != 1 {
        return Err(CodecError::FrameMismatch);
    }
    if mode == Mode::LosslessOnly && header.quantization_style != 0 {
        return Err(CodecError::FrameMismatch);
    }
    Ok(())
}

/// The photometric and component-count matrix both JPEG 2000 adapters apply,
/// PS3.5 8.2.4 Table 8.2.4-1 and its notes.
pub(crate) fn validate_colour_shape(
    components: &[ComponentParams],
    multiple_component_transform: u8,
    desc: &FrameDesc,
) -> Result<(), CodecError> {
    if multiple_component_transform > 1 {
        // ISO/IEC 15444-1 A.6.1 Table A.17 defines the values 0 and 1.
        return Err(CodecError::InvalidCodestream);
    }
    let samples = desc.samples_per_pixel();
    match components.len() {
        1 => {
            if samples != 1 {
                return Err(CodecError::FrameMismatch);
            }
            if multiple_component_transform != 0 {
                // The colour transform is defined over three components.
                return Err(CodecError::UnsupportedPixelFormat);
            }
            Ok(())
        }
        3 => {
            if samples != 3 {
                return Err(CodecError::FrameMismatch);
            }
            match multiple_component_transform {
                0 => {
                    if matches!(desc.photometric_interpretation(), "YBR_RCT" | "YBR_ICT") {
                        // PS3.5 8.2.4 note 2: those two codes name the
                        // codestream's own transform, which this codestream
                        // says it did not apply.
                        return Err(CodecError::FrameMismatch);
                    }
                }
                // PS3.5 8.2.4: a codestream that applied the transform hands
                // back RGB samples, so the photometric must name it. `RGB`
                // or `YBR_FULL` over a transformed codestream would draw
                // already-RGB samples through a YBR matrix.
                1 if !matches!(desc.photometric_interpretation(), "YBR_RCT" | "YBR_ICT") => {
                    return Err(CodecError::FrameMismatch);
                }
                _ => {}
            }
            Ok(())
        }
        _ => Err(CodecError::UnsupportedPixelFormat),
    }
}

/// The three components of a colour codestream must be sub-sample free and
/// share depth and signedness, as openjph-core's own validity check requires
/// of the transform, because one DICOM descriptor describes all of them.
pub(crate) fn validate_colour_components(components: &[ComponentParams]) -> Result<(), CodecError> {
    let first = first_component(components)?;
    if components.iter().any(|component| {
        component.x_sampling != 1
            || component.y_sampling != 1
            || component.precision != first.precision
            || component.signed != first.signed
    }) {
        return Err(CodecError::UnsupportedPixelFormat);
    }
    Ok(())
}

pub(crate) fn first_component(
    components: &[ComponentParams],
) -> Result<ComponentParams, CodecError> {
    components
        .first()
        .copied()
        .ok_or(CodecError::InvalidCodestream)
}

/// What a decode of `desc` hands back, PS3.5 8.2.4. `YBR_RCT` and `YBR_ICT`
/// name the codestream's own component transform, so a decode of either is
/// packed RGB, and everything else is preserved.
pub(crate) fn decoded_photometric(desc: &FrameDesc) -> DecodePhotometricInterpretation {
    if desc.samples_per_pixel() == 3
        && matches!(desc.photometric_interpretation(), "YBR_RCT" | "YBR_ICT")
    {
        DecodePhotometricInterpretation::Rgb
    } else {
        DecodePhotometricInterpretation::Preserved
    }
}

/// The photometric and sample-count descriptor matrix shared by the two JPEG
/// 2000 adapters, PS3.5 8.2.4 Table 8.2.4-1. A colour frame is three samples
/// in an eight or sixteen bit container. The two 4:2:2 photometric values are
/// not valid for these syntaxes and a palette stays an index stream the
/// colour stage owns.
pub(crate) fn validate_pixel_descriptor(desc: &FrameDesc) -> Result<(), CodecError> {
    if desc.pixel_data_vr() != PixelDataVr::Ob {
        return Err(CodecError::UnsupportedPixelFormat);
    }
    match (desc.samples_per_pixel(), desc.photometric_interpretation()) {
        (1, "MONOCHROME1" | "MONOCHROME2") => {}
        (3, "RGB" | "YBR_FULL" | "YBR_RCT" | "YBR_ICT") => {}
        _ => return Err(CodecError::UnsupportedPixelFormat),
    }
    if !matches!(desc.bits_allocated(), 8 | 16) {
        return Err(CodecError::UnsupportedPixelFormat);
    }
    Ok(())
}

pub(crate) fn read_u16(src: &[u8], offset: usize) -> Result<u16, CodecError> {
    let bytes = src
        .get(offset..offset + 2)
        .ok_or(CodecError::InvalidCodestream)?;
    Ok(u16::from_be_bytes(
        <[u8; 2]>::try_from(bytes).map_err(|_| CodecError::InvalidCodestream)?,
    ))
}

pub(crate) fn read_u32(src: &[u8], offset: usize) -> Result<u32, CodecError> {
    let bytes = src
        .get(offset..offset + 4)
        .ok_or(CodecError::InvalidCodestream)?;
    Ok(u32::from_be_bytes(
        <[u8; 4]>::try_from(bytes).map_err(|_| CodecError::InvalidCodestream)?,
    ))
}
