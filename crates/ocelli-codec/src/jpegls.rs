//! JPEG-LS lossless and near-lossless transfer-syntax adapters.
//!
//! Appendix A gate A2 resolved the JPEG-LS route as pure Rust. The S09 design
//! round selected the already-vendored `ritk-codecs` 0.6.0 `jpeg_ls` module over
//! `pure_jpegls` 2.0.0, because gate A2's one advantage for the latter was
//! multi-component support and the vendored source refuses it too, leaving a
//! second vendored package as the only difference.
//!
//! The dependency's `PixelLayout` applies a modality rescale. Slope is pinned to
//! `1.0` and intercept to `0.0` at the one call site, so HLD section 18's
//! arithmetic still exists exactly once, in `ocelli-pixel`. The dependency
//! refuses a multi-component scan. One shape is decoded here instead: 8-bit
//! RGB, `NEAR` zero, sample interleaved. Every other multi-component codestream
//! is still refused.

mod sample_interleaved;

use std::sync::Arc;

use ritk_codecs::{PixelLayout, PixelSignedness, decode_jpeg_ls_fragment};

use crate::{
    Capability, CodecError, DecodePhotometricInterpretation, DecodeSampleLayout, Decoder,
    FrameDesc, PixelDataVr, PixelRepresentation, Registry, RegistryError,
    sample_convert::convert_samples,
};

const JPEGLS_LOSSLESS_UID: &str = "1.2.840.10008.1.2.4.80";
const JPEGLS_NEAR_LOSSLESS_UID: &str = "1.2.840.10008.1.2.4.81";
const LOSSLESS_UIDS: &[&str] = &[JPEGLS_LOSSLESS_UID];
const NEAR_LOSSLESS_UIDS: &[&str] = &[JPEGLS_NEAR_LOSSLESS_UID];

/// ISO/IEC 14495-1 marker codes, second byte after `0xff`.
const SOI: u8 = 0xd8;
const EOI: u8 = 0xd9;
const SOF55: u8 = 0xf7;
const SOS: u8 = 0xda;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Lossless,
    NearLossless,
}

/// One exact DICOM JPEG-LS transfer-syntax adapter.
pub struct JpegLsDecoder {
    mode: Mode,
}

impl JpegLsDecoder {
    /// Decoder for lossless JPEG-LS UID `.80`, where `NEAR` must be zero.
    #[must_use]
    pub const fn lossless() -> Self {
        Self {
            mode: Mode::Lossless,
        }
    }

    /// Decoder for near-lossless JPEG-LS UID `.81`, where `NEAR` must be
    /// positive.
    #[must_use]
    pub const fn near_lossless() -> Self {
        Self {
            mode: Mode::NearLossless,
        }
    }
}

/// Register both JPEG-LS adapters atomically.
///
/// # Errors
///
/// Returns the first registry collision or declaration error. Preflight occurs
/// before either registration, so an error leaves both capabilities unchanged.
pub fn register_jpegls_decoders(registry: &mut Registry) -> Result<(), RegistryError> {
    for uid in [JPEGLS_LOSSLESS_UID, JPEGLS_NEAR_LOSSLESS_UID] {
        match registry.capability(uid) {
            Capability::Available => return Err(RegistryError::AlreadyRegistered { uid }),
            Capability::KnownUnavailable => {}
            Capability::Unknown => return Err(RegistryError::UnknownTransferSyntax { uid }),
        }
    }
    registry.register(Arc::new(JpegLsDecoder::lossless()))?;
    registry.register(Arc::new(JpegLsDecoder::near_lossless()))
}

impl Decoder for JpegLsDecoder {
    fn transfer_syntaxes(&self) -> &'static [&'static str] {
        match self.mode {
            Mode::Lossless => LOSSLESS_UIDS,
            Mode::NearLossless => NEAR_LOSSLESS_UIDS,
        }
    }

    fn decode_photometric_interpretation(
        &self,
        desc: &FrameDesc,
    ) -> DecodePhotometricInterpretation {
        if self.mode == Mode::Lossless && rgb8_frame(desc) {
            DecodePhotometricInterpretation::Rgb
        } else {
            DecodePhotometricInterpretation::Preserved
        }
    }

    fn decode_sample_layout(&self, desc: &FrameDesc) -> DecodeSampleLayout {
        if self.mode == Mode::Lossless && rgb8_frame(desc) {
            DecodeSampleLayout::Interleaved
        } else {
            DecodeSampleLayout::Preserved
        }
    }

    fn decode(&self, src: &[u8], desc: &FrameDesc, out: &mut [u8]) -> Result<(), CodecError> {
        if out.len() != desc.output_len() {
            return Err(CodecError::OutputLength {
                expected: desc.output_len(),
                actual: out.len(),
            });
        }
        validate_descriptor(desc)?;
        let codestream = logical_codestream(src)?;
        let header = inspect_headers(codestream)?;
        validate_header(self.mode, header, desc)?;
        if rgb8_frame(desc) {
            // The sample-interleaved decoder is the lossless 8-bit scan only.
            // A near-lossless UID, including one whose NEAR is above zero,
            // stays refused rather than being read with the lossless thresholds.
            if self.mode != Mode::Lossless {
                return Err(CodecError::UnsupportedPixelFormat);
            }
            let decoded =
                sample_interleaved::decode(colour_scan(codestream)?, desc.rows(), desc.columns())?;
            if decoded.len() != out.len() {
                return Err(CodecError::InvalidCodestream);
            }
            out.copy_from_slice(&decoded);
            return Ok(());
        }

        let signedness = match desc.pixel_representation() {
            PixelRepresentation::Unsigned => PixelSignedness::Unsigned,
            PixelRepresentation::Signed => PixelSignedness::Signed,
        };
        let layout = PixelLayout {
            rows: usize::from(desc.rows()),
            cols: usize::from(desc.columns()),
            samples_per_pixel: 1,
            bits_allocated: desc.bits_allocated(),
            pixel_representation: signedness,
            // HLD section 18 puts the modality LUT in `ocelli-pixel`. Pinning
            // the identity here keeps that true, and a fixture fails if either
            // value ever moves.
            rescale_slope: 1.0,
            rescale_intercept: 0.0,
        };
        let decoded =
            decode_jpeg_ls_fragment(codestream, layout).map_err(|_| CodecError::DecoderFailure)?;
        convert_samples(&decoded, desc, out)
    }
}

/// 8-bit interleaved RGB. This is the one multi-component shape decoded here.
fn rgb8_frame(desc: &FrameDesc) -> bool {
    desc.pixel_data_vr() == PixelDataVr::Ob
        && desc.samples_per_pixel() == 3
        && desc.bits_allocated() == 8
        && desc.bits_stored() == 8
        && desc.pixel_representation() == PixelRepresentation::Unsigned
        && desc.photometric_interpretation() == "RGB"
}

fn validate_descriptor(desc: &FrameDesc) -> Result<(), CodecError> {
    if rgb8_frame(desc) {
        return Ok(());
    }
    // Every other multi-component or colour shape is refused before a sample
    // is written. A monochrome descriptor paired with a colour codestream is
    // caught later, because this check cannot see the codestream.
    if desc.pixel_data_vr() != PixelDataVr::Ob
        || desc.samples_per_pixel() != 1
        || !matches!(desc.bits_allocated(), 8 | 16)
        || !matches!(
            desc.photometric_interpretation(),
            "MONOCHROME1" | "MONOCHROME2"
        )
    {
        return Err(CodecError::UnsupportedPixelFormat);
    }
    Ok(())
}

#[derive(Clone, Copy)]
struct Headers {
    /// Sample precision `P` from SOF55, ISO/IEC 14495-1 C.2.2.
    precision: u8,
    /// Number of lines `Y`.
    rows: u16,
    /// Number of samples per line `X`.
    columns: u16,
    /// Number of components `Nf`.
    components: u8,
    /// `NEAR` from SOS, ISO/IEC 14495-1 C.2.3.
    near: u8,
    /// Interleave mode `ILV`.
    interleave: u8,
}

/// Bound the codestream to SOI through EOI, PS3.5 A.4.
fn logical_codestream(src: &[u8]) -> Result<&[u8], CodecError> {
    if src.len() < 4 || !src.starts_with(&[0xff, SOI]) {
        return Err(CodecError::InvalidCodestream);
    }
    if src.ends_with(&[0xff, EOI]) {
        return Ok(src);
    }
    // PS3.5 A.4 pads an odd fragment to an even length with a single byte.
    if src.ends_with(&[0xff, EOI, 0]) && !(src.len() - 1).is_multiple_of(2) {
        return src
            .get(..src.len() - 1)
            .ok_or(CodecError::InvalidCodestream);
    }
    if src.windows(2).any(|window| window == [0xff, EOI]) {
        return Err(CodecError::TrailingData);
    }
    Err(CodecError::InvalidCodestream)
}

/// Read SOF55 and SOS without decoding the entropy-coded scan.
///
/// The walk stops at SOS, because everything after its segment is scan data in
/// which `0xff` is byte-stuffed and marker parsing no longer applies.
fn inspect_headers(src: &[u8]) -> Result<Headers, CodecError> {
    let mut offset = 2usize;
    let mut frame = None;
    while offset.checked_add(4).is_some_and(|end| end <= src.len()) {
        if src.get(offset) != Some(&0xff) {
            return Err(CodecError::InvalidCodestream);
        }
        let marker = *src.get(offset + 1).ok_or(CodecError::InvalidCodestream)?;
        if matches!(marker, SOI | EOI) {
            return Err(CodecError::InvalidCodestream);
        }
        let length = usize::from(read_u16(src, offset + 2)?);
        if length < 2 {
            return Err(CodecError::InvalidCodestream);
        }
        let end = offset
            .checked_add(2)
            .and_then(|value| value.checked_add(length))
            .ok_or(CodecError::InvalidCodestream)?;
        let payload = src
            .get(offset + 4..end)
            .ok_or(CodecError::InvalidCodestream)?;
        match marker {
            SOF55 => {
                if frame.is_some() {
                    return Err(CodecError::InvalidCodestream);
                }
                frame = Some(parse_sof55(payload)?);
            }
            SOS => {
                let (near, interleave) = parse_sos(payload)?;
                let (precision, rows, columns, components) =
                    frame.ok_or(CodecError::InvalidCodestream)?;
                return Ok(Headers {
                    precision,
                    rows,
                    columns,
                    components,
                    near,
                    interleave,
                });
            }
            _ => {}
        }
        offset = end;
    }
    Err(CodecError::InvalidCodestream)
}

/// SOF55, ISO/IEC 14495-1 C.2.2: `P Y X Nf (Ci Hi/Vi Tqi)*Nf`.
fn parse_sof55(payload: &[u8]) -> Result<(u8, u16, u16, u8), CodecError> {
    let precision = *payload.first().ok_or(CodecError::InvalidCodestream)?;
    let rows = read_u16(payload, 1)?;
    let columns = read_u16(payload, 3)?;
    let components = *payload.get(5).ok_or(CodecError::InvalidCodestream)?;
    if components == 0 {
        return Err(CodecError::InvalidCodestream);
    }
    let expected = 6usize
        .checked_add(
            usize::from(components)
                .checked_mul(3)
                .ok_or(CodecError::InvalidCodestream)?,
        )
        .ok_or(CodecError::InvalidCodestream)?;
    if payload.len() != expected {
        return Err(CodecError::InvalidCodestream);
    }
    // ISO/IEC 14495-1 A.1 permits precision 2 through 16.
    if !(2..=16).contains(&precision) || rows == 0 || columns == 0 {
        return Err(CodecError::InvalidCodestream);
    }
    Ok((precision, rows, columns, components))
}

/// SOS, ISO/IEC 14495-1 C.2.3: `Ns (Cj Tmj)*Ns NEAR ILV Al/Ah`.
///
/// NEAR sits two bytes per component past `Ns`. Reading it from the end of the
/// segment instead lands on `ILV`, which is zero for every non-interleaved
/// frame and therefore looks like a lossless NEAR on a near-lossless
/// codestream.
fn parse_sos(payload: &[u8]) -> Result<(u8, u8), CodecError> {
    let scan_components = *payload.first().ok_or(CodecError::InvalidCodestream)?;
    let expected = 4usize
        .checked_add(
            usize::from(scan_components)
                .checked_mul(2)
                .ok_or(CodecError::InvalidCodestream)?,
        )
        .ok_or(CodecError::InvalidCodestream)?;
    if payload.len() != expected {
        return Err(CodecError::InvalidCodestream);
    }
    let near_index = 1usize
        .checked_add(
            usize::from(scan_components)
                .checked_mul(2)
                .ok_or(CodecError::InvalidCodestream)?,
        )
        .ok_or(CodecError::InvalidCodestream)?;
    let near = *payload
        .get(near_index)
        .ok_or(CodecError::InvalidCodestream)?;
    let interleave = *payload
        .get(near_index + 1)
        .ok_or(CodecError::InvalidCodestream)?;
    Ok((near, interleave))
}

fn validate_header(mode: Mode, header: Headers, desc: &FrameDesc) -> Result<(), CodecError> {
    // Two accepted shapes. Anything else, including line interleave, is a
    // refusal rather than a wrong pixel. The vendored decoder still accepts
    // only the single-component arm.
    let sample_interleaved = header.components == 3 && header.interleave == 2;
    let single = header.components == 1 && header.interleave == 0;
    if !sample_interleaved && !single {
        return Err(CodecError::UnsupportedPixelFormat);
    }
    if sample_interleaved && !rgb8_frame(desc) {
        return Err(CodecError::UnsupportedPixelFormat);
    }
    if header.rows != desc.rows()
        || header.columns != desc.columns()
        || u16::from(header.precision) != desc.bits_stored()
    {
        return Err(CodecError::FrameMismatch);
    }
    // PS3.5 A.4.3 and A.4.4. `.80` is lossless, which ISO/IEC 14495-1 defines
    // as NEAR zero, and `.81` is near-lossless, which is NEAR above zero.
    // Without this the two UIDs are interchangeable and the lossless claim is
    // unfalsifiable.
    let near_is_valid = match mode {
        Mode::Lossless => header.near == 0,
        Mode::NearLossless => header.near > 0,
    };
    if !near_is_valid {
        return Err(CodecError::FrameMismatch);
    }
    if single
        && (desc.samples_per_pixel() != 1
            || !matches!(
                desc.photometric_interpretation(),
                "MONOCHROME1" | "MONOCHROME2"
            ))
    {
        return Err(CodecError::FrameMismatch);
    }
    Ok(())
}

/// Bytes of one scan, from the end of SOS through the byte before EOI.
///
/// The colour decoder accepts SOF55 and SOS only. An LSE preset, a restart
/// interval, or any other marker would change the coding parameters, and
/// ignoring one would decode a different image.
fn colour_scan(src: &[u8]) -> Result<&[u8], CodecError> {
    let mut offset = 2usize;
    let mut saw_frame = false;
    while offset.checked_add(4).is_some_and(|end| end <= src.len()) {
        if src.get(offset) != Some(&0xff) {
            return Err(CodecError::InvalidCodestream);
        }
        let marker = *src.get(offset + 1).ok_or(CodecError::InvalidCodestream)?;
        if matches!(marker, SOI | EOI) {
            return Err(CodecError::InvalidCodestream);
        }
        let length = usize::from(read_u16(src, offset + 2)?);
        if length < 2 {
            return Err(CodecError::InvalidCodestream);
        }
        let end = offset
            .checked_add(2)
            .and_then(|value| value.checked_add(length))
            .ok_or(CodecError::InvalidCodestream)?;
        if end > src.len() {
            return Err(CodecError::InvalidCodestream);
        }
        match marker {
            SOF55 => {
                if saw_frame {
                    return Err(CodecError::InvalidCodestream);
                }
                saw_frame = true;
            }
            SOS => {
                if !saw_frame {
                    return Err(CodecError::InvalidCodestream);
                }
                let scan_end = src
                    .len()
                    .checked_sub(2)
                    .ok_or(CodecError::InvalidCodestream)?;
                if end > scan_end || src.get(scan_end..scan_end + 2) != Some(&[0xff, EOI]) {
                    return Err(CodecError::InvalidCodestream);
                }
                return src.get(end..scan_end).ok_or(CodecError::InvalidCodestream);
            }
            _ => return Err(CodecError::UnsupportedPixelFormat),
        }
        offset = end;
    }
    Err(CodecError::InvalidCodestream)
}

fn read_u16(src: &[u8], offset: usize) -> Result<u16, CodecError> {
    let bytes = src
        .get(offset..offset + 2)
        .ok_or(CodecError::InvalidCodestream)?;
    Ok(u16::from_be_bytes(
        <[u8; 2]>::try_from(bytes).map_err(|_| CodecError::InvalidCodestream)?,
    ))
}

#[cfg(test)]
mod tests {
    use super::{parse_sof55, parse_sos};
    use crate::CodecError;

    #[test]
    fn sos_near_is_read_past_the_component_specifiers() {
        // The exact SOS payload pyjpegls 1.5.1 emits for one component at
        // NEAR 3: Ns=01, (Cj=01, Tmj=00), NEAR=03, ILV=00, Al/Ah=00.
        assert_eq!(parse_sos(&[0x01, 0x01, 0x00, 0x03, 0x00, 0x00]), Ok((3, 0)));
        // The same shape at NEAR 0, which is what `.80` requires.
        assert_eq!(parse_sos(&[0x01, 0x01, 0x00, 0x00, 0x00, 0x00]), Ok((0, 0)));
        // Reading from the end of the segment would return ILV here, so this
        // row separates the correct index from the plausible one.
        assert_eq!(parse_sos(&[0x01, 0x01, 0x00, 0x05, 0x02, 0x00]), Ok((5, 2)));
        // Two components: NEAR moves two bytes further along.
        assert_eq!(
            parse_sos(&[0x02, 0x01, 0x00, 0x02, 0x00, 0x07, 0x01, 0x00]),
            Ok((7, 1))
        );
    }

    #[test]
    fn malformed_marker_segments_are_refused() {
        assert_eq!(parse_sos(&[]), Err(CodecError::InvalidCodestream));
        // Length disagrees with the declared component count.
        assert_eq!(
            parse_sos(&[0x02, 0x01, 0x00, 0x03, 0x00, 0x00]),
            Err(CodecError::InvalidCodestream)
        );
        assert_eq!(parse_sof55(&[]), Err(CodecError::InvalidCodestream));
        // Zero components.
        assert_eq!(
            parse_sof55(&[0x0c, 0x00, 0x40, 0x00, 0x40, 0x00]),
            Err(CodecError::InvalidCodestream)
        );
        // Precision 17, outside ISO/IEC 14495-1 A.1's 2 to 16.
        assert_eq!(
            parse_sof55(&[0x11, 0x00, 0x40, 0x00, 0x40, 0x01, 0x01, 0x11, 0x00]),
            Err(CodecError::InvalidCodestream)
        );
        // Zero rows.
        assert_eq!(
            parse_sof55(&[0x0c, 0x00, 0x00, 0x00, 0x40, 0x01, 0x01, 0x11, 0x00]),
            Err(CodecError::InvalidCodestream)
        );
    }

    #[test]
    fn sof55_reads_the_pyjpegls_payload() {
        // The exact SOF55 payload pyjpegls 1.5.1 emits for a 64 by 64 12-bit
        // single-component frame.
        assert_eq!(
            parse_sof55(&[0x0c, 0x00, 0x40, 0x00, 0x40, 0x01, 0x01, 0x11, 0x00]),
            Ok((12, 64, 64, 1))
        );
    }
}
