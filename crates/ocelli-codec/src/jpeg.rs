//! JPEG Baseline, Extended, and lossless adapters.

use std::{io::Cursor, sync::Arc};

use jpeg_decoder::{CodingProcess, Decoder as ImageJpegDecoder, PixelFormat};
use oxideav_core::{CodecId, CodecParameters, Frame, Packet, TimeBase};

use crate::{
    CodecError, DecodePhotometricInterpretation, Decoder, FrameDesc, Registry, RegistryError,
};

const JPEG_BASELINE_UID: &str = "1.2.840.10008.1.2.4.50";
const JPEG_EXTENDED_UID: &str = "1.2.840.10008.1.2.4.51";
const JPEG_LOSSLESS_UID: &str = "1.2.840.10008.1.2.4.57";
const JPEG_LOSSLESS_SV1_UID: &str = "1.2.840.10008.1.2.4.70";
const JPEG_BASELINE_UIDS: &[&str] = &[JPEG_BASELINE_UID];
const JPEG_EXTENDED_UIDS: &[&str] = &[JPEG_EXTENDED_UID];
const JPEG_LOSSLESS_UIDS: &[&str] = &[JPEG_LOSSLESS_UID];
const JPEG_LOSSLESS_SV1_UIDS: &[&str] = &[JPEG_LOSSLESS_SV1_UID];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum JpegSyntax {
    Baseline,
    Extended,
    Lossless,
    LosslessSv1,
}

/// One exact DICOM JPEG transfer-syntax adapter.
///
/// Separate configured values are necessary because the HLD decoder trait
/// does not pass the selected transfer-syntax UID into `decode`. Keeping the
/// UID in the registered decoder lets `.70` enforce selection value 1 rather
/// than treating every lossless process-14 stream as interchangeable.
pub struct JpegDecoder {
    syntax: JpegSyntax,
}

impl JpegDecoder {
    /// Decoder for JPEG Baseline process 1, UID `.50`.
    #[must_use]
    pub const fn baseline() -> Self {
        Self {
            syntax: JpegSyntax::Baseline,
        }
    }

    /// Decoder for JPEG Extended process 2 at 8 bits and process 4 at 12 bits,
    /// UID `.51`.
    #[must_use]
    pub const fn extended() -> Self {
        Self {
            syntax: JpegSyntax::Extended,
        }
    }

    /// Decoder for JPEG Lossless process 14, UID `.57`.
    #[must_use]
    pub const fn lossless() -> Self {
        Self {
            syntax: JpegSyntax::Lossless,
        }
    }

    /// Decoder for JPEG Lossless process 14 selection value 1, UID `.70`.
    #[must_use]
    pub const fn lossless_sv1() -> Self {
        Self {
            syntax: JpegSyntax::LosslessSv1,
        }
    }
}

/// Register the four JPEG adapters atomically.
///
/// # Errors
///
/// Returns a registry collision or declaration error. Every UID is preflighted
/// before registration, so any error leaves all four JPEG capabilities as they
/// were before this call.
pub fn register_jpeg_decoders(registry: &mut Registry) -> Result<(), RegistryError> {
    for uid in [
        JPEG_BASELINE_UID,
        JPEG_EXTENDED_UID,
        JPEG_LOSSLESS_UID,
        JPEG_LOSSLESS_SV1_UID,
    ] {
        match registry.capability(uid) {
            crate::Capability::Available => {
                return Err(RegistryError::AlreadyRegistered { uid });
            }
            crate::Capability::KnownUnavailable => {}
            crate::Capability::Unknown => {
                return Err(RegistryError::UnknownTransferSyntax { uid });
            }
        }
    }
    registry.register(Arc::new(JpegDecoder::baseline()))?;
    registry.register(Arc::new(JpegDecoder::extended()))?;
    registry.register(Arc::new(JpegDecoder::lossless()))?;
    registry.register(Arc::new(JpegDecoder::lossless_sv1()))
}

impl Decoder for JpegDecoder {
    fn transfer_syntaxes(&self) -> &'static [&'static str] {
        match self.syntax {
            JpegSyntax::Baseline => JPEG_BASELINE_UIDS,
            JpegSyntax::Extended => JPEG_EXTENDED_UIDS,
            JpegSyntax::Lossless => JPEG_LOSSLESS_UIDS,
            JpegSyntax::LosslessSv1 => JPEG_LOSSLESS_SV1_UIDS,
        }
    }

    fn decode_photometric_interpretation(
        &self,
        desc: &FrameDesc,
    ) -> DecodePhotometricInterpretation {
        if desc.samples_per_pixel() == 3
            && (self.syntax != JpegSyntax::Extended || desc.bits_stored() == 8)
        {
            DecodePhotometricInterpretation::Rgb
        } else {
            DecodePhotometricInterpretation::Preserved
        }
    }

    fn decode(&self, src: &[u8], desc: &FrameDesc, out: &mut [u8]) -> Result<(), CodecError> {
        if out.len() != desc.output_len() {
            return Err(CodecError::OutputLength {
                expected: desc.output_len(),
                actual: out.len(),
            });
        }
        let (header, codestream) = inspect_jpeg(src)?;
        validate_header(self.syntax, header, desc)?;
        let decoded = match self.syntax {
            JpegSyntax::Extended if header.precision == 12 => decode_extended(codestream, desc)?,
            JpegSyntax::Extended
            | JpegSyntax::Baseline
            | JpegSyntax::Lossless
            | JpegSyntax::LosslessSv1 => decode_image_jpeg(codestream, desc, header)?,
        };
        if decoded.len() != out.len() {
            return Err(CodecError::FrameMismatch);
        }
        out.copy_from_slice(&decoded);
        Ok(())
    }
}

fn decode_image_jpeg(
    src: &[u8],
    desc: &FrameDesc,
    header: JpegHeader,
) -> Result<Vec<u8>, CodecError> {
    let mut decoder = ImageJpegDecoder::new(Cursor::new(src));
    decoder.set_max_decoding_buffer_size(desc.output_len());
    let mut decoded = decoder.decode().map_err(|_| CodecError::DecoderFailure)?;
    let info = decoder.info().ok_or(CodecError::DecoderFailure)?;
    if info.width != desc.columns() || info.height != desc.rows() {
        return Err(CodecError::FrameMismatch);
    }
    let expected_process = if header.marker == 0xc3 {
        CodingProcess::Lossless
    } else {
        CodingProcess::DctSequential
    };
    if info.coding_process != expected_process {
        return Err(CodecError::FrameMismatch);
    }
    let expected_format = match (desc.samples_per_pixel(), desc.bits_allocated()) {
        (1, 8) => PixelFormat::L8,
        (1, 16) => PixelFormat::L16,
        (3, 8) => PixelFormat::RGB24,
        _ => return Err(CodecError::UnsupportedPixelFormat),
    };
    if info.pixel_format != expected_format {
        return Err(CodecError::FrameMismatch);
    }
    if expected_format == PixelFormat::L16 && cfg!(target_endian = "big") {
        for chunk in decoded.chunks_exact_mut(2) {
            let bytes = <&mut [u8; 2]>::try_from(chunk).map_err(|_| CodecError::DecoderFailure)?;
            let sample = u16::from_ne_bytes(*bytes);
            *bytes = sample.to_le_bytes();
        }
    }
    Ok(decoded)
}

fn decode_extended(src: &[u8], desc: &FrameDesc) -> Result<Vec<u8>, CodecError> {
    if desc.samples_per_pixel() != 1 || desc.bits_stored() != 12 || desc.bits_allocated() != 16 {
        return Err(CodecError::UnsupportedPixelFormat);
    }
    let params = CodecParameters::video(CodecId::new("mjpeg"));
    let mut decoder =
        oxideav_mjpeg::registry::make_decoder(&params).map_err(|_| CodecError::DecoderFailure)?;
    let packet = Packet::new(0, TimeBase::new(1, 1), src.to_vec());
    decoder
        .send_packet(&packet)
        .map_err(|_| CodecError::DecoderFailure)?;
    let frame = decoder
        .receive_frame()
        .map_err(|_| CodecError::DecoderFailure)?;
    let Frame::Video(video) = frame else {
        return Err(CodecError::UnsupportedPixelFormat);
    };
    let mut planes = video.planes.into_iter();
    let plane = planes.next().ok_or(CodecError::UnsupportedPixelFormat)?;
    if planes.next().is_some() {
        return Err(CodecError::UnsupportedPixelFormat);
    }
    let expected_stride = usize::from(desc.columns())
        .checked_mul(2)
        .ok_or(CodecError::FrameMismatch)?;
    if plane.stride != expected_stride || plane.data.len() != desc.output_len() {
        return Err(CodecError::FrameMismatch);
    }
    Ok(plane.data)
}

#[derive(Debug, Clone, Copy)]
struct JpegHeader {
    marker: u8,
    precision: u8,
    height: u16,
    width: u16,
    components: u8,
    every_predictor_is_one: bool,
}

fn validate_header(
    syntax: JpegSyntax,
    header: JpegHeader,
    desc: &FrameDesc,
) -> Result<(), CodecError> {
    let process_matches = match syntax {
        JpegSyntax::Baseline => header.marker == 0xc0 && header.precision == 8,
        JpegSyntax::Extended => header.marker == 0xc1 && matches!(header.precision, 8 | 12),
        JpegSyntax::Lossless | JpegSyntax::LosslessSv1 => header.marker == 0xc3,
    };
    if !process_matches
        || header.height != desc.rows()
        || header.width != desc.columns()
        || u16::from(header.components) != desc.samples_per_pixel()
        || u16::from(header.precision) != desc.bits_stored()
        || (header.precision <= 8 && desc.bits_allocated() != 8)
        || (header.precision > 8 && desc.bits_allocated() != 16)
        || (syntax == JpegSyntax::LosslessSv1 && !header.every_predictor_is_one)
    {
        return Err(CodecError::FrameMismatch);
    }
    Ok(())
}

fn inspect_jpeg(src: &[u8]) -> Result<(JpegHeader, &[u8]), CodecError> {
    if !src.starts_with(&[0xff, 0xd8]) {
        return Err(CodecError::InvalidCodestream);
    }
    let mut offset = 2;
    let mut in_entropy = false;
    let mut frame = None;
    let mut saw_scan = false;
    let mut every_predictor_is_one = true;
    let codestream_len = loop {
        let marker = next_marker(src, &mut offset, in_entropy)?;
        in_entropy = false;
        match marker {
            0xd9 => {
                let trailing = src.get(offset..).ok_or(CodecError::InvalidCodestream)?;
                if !(trailing.is_empty() || (offset % 2 == 1 && trailing == [0])) {
                    return Err(CodecError::TrailingData);
                }
                break offset;
            }
            0xc0 | 0xc1 | 0xc3 => {
                if frame.is_some() {
                    return Err(CodecError::InvalidCodestream);
                }
                let segment = take_segment(src, &mut offset)?;
                frame = Some(parse_frame(marker, segment)?);
            }
            0xda => {
                let segment = take_segment(src, &mut offset)?;
                let components =
                    usize::from(*segment.first().ok_or(CodecError::InvalidCodestream)?);
                let selection_offset = 1usize
                    .checked_add(
                        components
                            .checked_mul(2)
                            .ok_or(CodecError::InvalidCodestream)?,
                    )
                    .ok_or(CodecError::InvalidCodestream)?;
                let predictor = *segment
                    .get(selection_offset)
                    .ok_or(CodecError::InvalidCodestream)?;
                every_predictor_is_one &= predictor == 1;
                saw_scan = true;
                in_entropy = true;
            }
            0xd8 | 0xd0..=0xd7 | 0x01 => return Err(CodecError::InvalidCodestream),
            _ => {
                let _ = take_segment(src, &mut offset)?;
            }
        }
    };
    let mut header = frame.ok_or(CodecError::InvalidCodestream)?;
    if !saw_scan {
        return Err(CodecError::InvalidCodestream);
    }
    header.every_predictor_is_one = every_predictor_is_one;
    let codestream = src
        .get(..codestream_len)
        .ok_or(CodecError::InvalidCodestream)?;
    Ok((header, codestream))
}

fn parse_frame(marker: u8, segment: &[u8]) -> Result<JpegHeader, CodecError> {
    let precision = *segment.first().ok_or(CodecError::InvalidCodestream)?;
    let height = read_be_u16(segment, 1)?;
    let width = read_be_u16(segment, 3)?;
    let components = *segment.get(5).ok_or(CodecError::InvalidCodestream)?;
    let required = 6usize
        .checked_add(
            usize::from(components)
                .checked_mul(3)
                .ok_or(CodecError::InvalidCodestream)?,
        )
        .ok_or(CodecError::InvalidCodestream)?;
    if segment.len() != required || height == 0 || width == 0 || components == 0 {
        return Err(CodecError::InvalidCodestream);
    }
    Ok(JpegHeader {
        marker,
        precision,
        height,
        width,
        components,
        every_predictor_is_one: true,
    })
}

fn next_marker(src: &[u8], offset: &mut usize, in_entropy: bool) -> Result<u8, CodecError> {
    if in_entropy {
        while let Some(byte) = take_byte(src, offset) {
            if byte != 0xff {
                continue;
            }
            let mut code = take_byte(src, offset).ok_or(CodecError::InvalidCodestream)?;
            while code == 0xff {
                code = take_byte(src, offset).ok_or(CodecError::InvalidCodestream)?;
            }
            if code == 0x00 || (0xd0..=0xd7).contains(&code) {
                continue;
            }
            return Ok(code);
        }
        return Err(CodecError::InvalidCodestream);
    }

    if take_byte(src, offset) != Some(0xff) {
        return Err(CodecError::InvalidCodestream);
    }
    let mut code = take_byte(src, offset).ok_or(CodecError::InvalidCodestream)?;
    while code == 0xff {
        code = take_byte(src, offset).ok_or(CodecError::InvalidCodestream)?;
    }
    if code == 0x00 {
        return Err(CodecError::InvalidCodestream);
    }
    Ok(code)
}

fn take_segment<'a>(src: &'a [u8], offset: &mut usize) -> Result<&'a [u8], CodecError> {
    let length = usize::from(read_be_u16(src, *offset)?);
    if length < 2 {
        return Err(CodecError::InvalidCodestream);
    }
    let start = offset.checked_add(2).ok_or(CodecError::InvalidCodestream)?;
    let end = offset
        .checked_add(length)
        .ok_or(CodecError::InvalidCodestream)?;
    let segment = src.get(start..end).ok_or(CodecError::InvalidCodestream)?;
    *offset = end;
    Ok(segment)
}

fn read_be_u16(src: &[u8], offset: usize) -> Result<u16, CodecError> {
    let end = offset.checked_add(2).ok_or(CodecError::InvalidCodestream)?;
    let bytes = <&[u8; 2]>::try_from(src.get(offset..end).ok_or(CodecError::InvalidCodestream)?)
        .map_err(|_| CodecError::InvalidCodestream)?;
    Ok(u16::from_be_bytes(*bytes))
}

fn take_byte(src: &[u8], offset: &mut usize) -> Option<u8> {
    let byte = src.get(*offset).copied()?;
    *offset = offset.checked_add(1)?;
    Some(byte)
}
