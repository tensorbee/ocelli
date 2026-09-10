//! JPEG 2000 lossless-only and general transfer-syntax adapters.

use std::sync::Arc;

use ritk_codecs::{PixelLayout, PixelSignedness, decode_jpeg2000_fragment};

use crate::{
    Capability, CodecError, DecodeSampleLayout, Decoder, FrameDesc, PixelDataVr,
    PixelRepresentation, Registry, RegistryError,
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
        validate_descriptor(desc)?;
        let codestream = logical_codestream(src)?;
        let header = inspect_main_header(codestream)?;
        validate_header(self.mode, header, desc)?;

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
            rescale_slope: 1.0,
            rescale_intercept: 0.0,
        };
        let decoded =
            decode_jpeg2000_fragment(codestream, layout).map_err(|_| CodecError::DecoderFailure)?;
        convert_samples(&decoded, desc, out)
    }
}

fn validate_descriptor(desc: &FrameDesc) -> Result<(), CodecError> {
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
struct MainHeader {
    width: u32,
    height: u32,
    components: u16,
    precision: u8,
    signed: bool,
    mct: u8,
    transform: u8,
    quantization_style: u8,
}

fn logical_codestream(src: &[u8]) -> Result<&[u8], CodecError> {
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

fn inspect_main_header(src: &[u8]) -> Result<MainHeader, CodecError> {
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
            0x5d => return Err(CodecError::UnsupportedPixelFormat),
            _ => {}
        }
        offset = end;
    }
    let (width, height, components, precision, signed) =
        siz.ok_or(CodecError::InvalidCodestream)?;
    let (mct, decomposition_levels, transform) = cod.ok_or(CodecError::InvalidCodestream)?;
    let (quantization_style, quantization_entries) = qcd.ok_or(CodecError::InvalidCodestream)?;
    validate_qcd(
        quantization_style,
        quantization_entries,
        decomposition_levels,
    )?;
    Ok(MainHeader {
        width,
        height,
        components,
        precision,
        signed,
        mct,
        transform,
        quantization_style,
    })
}

fn parse_qcd(payload: &[u8]) -> Result<(u8, usize), CodecError> {
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

fn parse_siz(payload: &[u8]) -> Result<(u32, u32, u16, u8, bool), CodecError> {
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
    let ssiz = *payload.get(36).ok_or(CodecError::InvalidCodestream)?;
    Ok((
        xsiz - xosiz,
        ysiz - yosiz,
        components,
        (ssiz & 0x7f) + 1,
        ssiz & 0x80 != 0,
    ))
}

fn validate_header(mode: Mode, header: MainHeader, desc: &FrameDesc) -> Result<(), CodecError> {
    if header.components != 1 || header.mct != 0 {
        return Err(CodecError::UnsupportedPixelFormat);
    }
    let signed = desc.pixel_representation() == PixelRepresentation::Signed;
    if header.width != u32::from(desc.columns())
        || header.height != u32::from(desc.rows())
        || u16::from(header.precision) != desc.bits_stored()
        || header.signed != signed
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

fn convert_samples(decoded: &[f32], desc: &FrameDesc, out: &mut [u8]) -> Result<(), CodecError> {
    let expected_samples = usize::from(desc.rows())
        .checked_mul(usize::from(desc.columns()))
        .ok_or(CodecError::FrameMismatch)?;
    if decoded.len() != expected_samples {
        return Err(CodecError::FrameMismatch);
    }
    if out.len() != desc.output_len() {
        return Err(CodecError::OutputLength {
            expected: desc.output_len(),
            actual: out.len(),
        });
    }

    let mut bytes = Vec::with_capacity(desc.output_len());
    for &value in decoded {
        append_sample(&mut bytes, value, desc)?;
    }
    if bytes.len() != out.len() {
        return Err(CodecError::FrameMismatch);
    }
    out.copy_from_slice(&bytes);
    Ok(())
}

fn append_sample(bytes: &mut Vec<u8>, value: f32, desc: &FrameDesc) -> Result<(), CodecError> {
    let sample = exact_i64(value).ok_or(CodecError::UnsupportedPixelFormat)?;
    let bits = desc.bits_stored();
    match (desc.bits_allocated(), desc.pixel_representation()) {
        (8, PixelRepresentation::Unsigned) => {
            let max = (1_i64 << bits) - 1;
            if !(0..=max).contains(&sample) {
                return Err(CodecError::UnsupportedPixelFormat);
            }
            bytes.push(u8::try_from(sample).map_err(|_| CodecError::UnsupportedPixelFormat)?);
        }
        (8, PixelRepresentation::Signed) => {
            let limit = 1_i64 << (bits - 1);
            if !(-limit..limit).contains(&sample) {
                return Err(CodecError::UnsupportedPixelFormat);
            }
            bytes.extend_from_slice(
                &i8::try_from(sample)
                    .map_err(|_| CodecError::UnsupportedPixelFormat)?
                    .to_le_bytes(),
            );
        }
        (16, PixelRepresentation::Unsigned) => {
            let max = (1_i64 << bits) - 1;
            if !(0..=max).contains(&sample) {
                return Err(CodecError::UnsupportedPixelFormat);
            }
            bytes.extend_from_slice(
                &u16::try_from(sample)
                    .map_err(|_| CodecError::UnsupportedPixelFormat)?
                    .to_le_bytes(),
            );
        }
        (16, PixelRepresentation::Signed) => {
            let limit = 1_i64 << (bits - 1);
            if !(-limit..limit).contains(&sample) {
                return Err(CodecError::UnsupportedPixelFormat);
            }
            bytes.extend_from_slice(
                &i16::try_from(sample)
                    .map_err(|_| CodecError::UnsupportedPixelFormat)?
                    .to_le_bytes(),
            );
        }
        _ => return Err(CodecError::UnsupportedPixelFormat),
    }
    Ok(())
}

fn exact_i64(value: f32) -> Option<i64> {
    if !value.is_finite() || value.fract() != 0.0 {
        return None;
    }
    if value == 0.0 {
        return Some(0);
    }
    let bits = value.to_bits();
    let exponent = i32::try_from((bits >> 23) & 0xff).ok()?.checked_sub(127)?;
    if !(0..=62).contains(&exponent) {
        return None;
    }
    let mantissa = u64::from((bits & 0x7f_ffff) | 0x80_0000);
    let magnitude = if exponent >= 23 {
        mantissa.checked_shl(u32::try_from(exponent - 23).ok()?)?
    } else {
        let shift = u32::try_from(23 - exponent).ok()?;
        let mask = (1_u64.checked_shl(shift)?).checked_sub(1)?;
        if mantissa & mask != 0 {
            return None;
        }
        mantissa >> shift
    };
    let signed = i64::try_from(magnitude).ok()?;
    if bits >> 31 == 0 {
        Some(signed)
    } else {
        signed.checked_neg()
    }
}

fn read_u16(src: &[u8], offset: usize) -> Result<u16, CodecError> {
    let bytes = src
        .get(offset..offset + 2)
        .ok_or(CodecError::InvalidCodestream)?;
    Ok(u16::from_be_bytes(
        <[u8; 2]>::try_from(bytes).map_err(|_| CodecError::InvalidCodestream)?,
    ))
}

fn read_u32(src: &[u8], offset: usize) -> Result<u32, CodecError> {
    let bytes = src
        .get(offset..offset + 4)
        .ok_or(CodecError::InvalidCodestream)?;
    Ok(u32::from_be_bytes(
        <[u8; 4]>::try_from(bytes).map_err(|_| CodecError::InvalidCodestream)?,
    ))
}

#[cfg(test)]
mod tests {
    use super::{convert_samples, exact_i64};
    use crate::{
        CodecError, FrameDesc, FrameDescError, FrameDescInput, PixelDataVr, PixelRepresentation,
    };

    #[test]
    fn exact_integer_conversion_refuses_fractional_and_nonfinite_values() {
        assert_eq!(exact_i64(4_095.0), Some(4_095));
        assert_eq!(exact_i64(-2_048.0), Some(-2_048));
        assert_eq!(exact_i64(0.5), None);
        assert_eq!(exact_i64(f32::NAN), None);
        assert_eq!(exact_i64(f32::INFINITY), None);
    }

    #[test]
    fn conversion_errors_preserve_caller_output() -> Result<(), FrameDescError> {
        let desc = FrameDesc::new(FrameDescInput {
            rows: 1,
            columns: 1,
            samples_per_pixel: 1,
            bits_allocated: 8,
            bits_stored: 8,
            high_bit: 7,
            pixel_representation: PixelRepresentation::Unsigned,
            photometric_interpretation: "MONOCHROME2".to_owned(),
            pixel_data_vr: PixelDataVr::Ob,
        })?;

        for decoded in [&[0.5_f32][..], &[256.0_f32][..]] {
            let mut out = [0xa5];
            assert_eq!(
                convert_samples(decoded, &desc, &mut out),
                Err(CodecError::UnsupportedPixelFormat)
            );
            assert_eq!(out, [0xa5]);
        }

        let mut out = [0xa5];
        assert_eq!(
            convert_samples(&[0.0, 1.0], &desc, &mut out),
            Err(CodecError::FrameMismatch)
        );
        assert_eq!(out, [0xa5]);
        Ok(())
    }
}
