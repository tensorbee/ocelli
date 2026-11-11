//! HTJ2K transfer-syntax adapters, JPEG 2000 Part 15.
//!
//! Appendix A gate A1 measured `openjp2` 0.6.1 as a failure on
//! `wasm32-unknown-unknown`, and F-X013 priced `openjph-core` 0.1.0 as the one
//! candidate that uses the same Rust implementation on browser, desktop and
//! server. Deviation D-22 records the departure from HLD section 15.2, which
//! names `openjp2`.
//!
//! The dependency has no API that decodes into a caller-provided slice: each
//! `pull` clones one image row into a fresh `Vec<i32>`. That is deviation
//! D-21's largest instance, the rows are validated completely before the
//! caller's output is touched, and the write is one `copy_from_slice`.

use std::sync::Arc;

use openjph_core::{codestream::Codestream, file::MemInfile};

use crate::jpeg2000::{
    ComponentParams, logical_codestream, parse_siz, read_u16, refuse_tile_coding_overrides,
    validate_colour_components, validate_colour_shape, validate_pixel_descriptor,
};
use crate::{
    Capability, CodecError, DecodePhotometricInterpretation, DecodeSampleLayout, Decoder,
    FrameDesc, PixelRepresentation, Registry, RegistryError, sample_convert::convert_samples,
};

const HTJ2K_LOSSLESS_UID: &str = "1.2.840.10008.1.2.4.201";
const HTJ2K_LOSSLESS_RPCL_UID: &str = "1.2.840.10008.1.2.4.202";
const HTJ2K_UID: &str = "1.2.840.10008.1.2.4.203";
const LOSSLESS_UIDS: &[&str] = &[HTJ2K_LOSSLESS_UID];
const LOSSLESS_RPCL_UIDS: &[&str] = &[HTJ2K_LOSSLESS_RPCL_UID];
const GENERAL_UIDS: &[&str] = &[HTJ2K_UID];

/// ISO/IEC 15444-1 marker codes, second byte after `0xff`.
const SOC: u8 = 0x4f;
const EOC: u8 = 0xd9;
const SIZ: u8 = 0x51;
const COD: u8 = 0x52;
const COC: u8 = 0x53;
const QCD: u8 = 0x5c;
const QCC: u8 = 0x5d;
const POC: u8 = 0x5f;
const CAP: u8 = 0x50;
const SOT: u8 = 0x90;

/// SGcod's progression order value for RPCL, ISO/IEC 15444-1 A.6.1 table A.16.
const PROGRESSION_RPCL: u8 = 2;

/// SPcod's wavelet transform value for the reversible 5/3 kernel, table A.20.
const WAVELET_REVERSIBLE_5_3: u8 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// `.201`, lossless only, no progression constraint.
    Lossless,
    /// `.202`, lossless only with RPCL progression.
    LosslessRpcl,
    /// `.203`, the general form.
    General,
}

/// One exact DICOM HTJ2K transfer-syntax adapter.
pub struct Htj2kDecoder {
    mode: Mode,
}

impl Htj2kDecoder {
    /// Decoder for HTJ2K Lossless Only UID `.201`.
    #[must_use]
    pub const fn lossless() -> Self {
        Self {
            mode: Mode::Lossless,
        }
    }

    /// Decoder for HTJ2K Lossless Only RPCL UID `.202`.
    #[must_use]
    pub const fn lossless_rpcl() -> Self {
        Self {
            mode: Mode::LosslessRpcl,
        }
    }

    /// Decoder for general HTJ2K UID `.203`.
    #[must_use]
    pub const fn general() -> Self {
        Self {
            mode: Mode::General,
        }
    }
}

/// Register all three HTJ2K adapters atomically.
///
/// # Errors
///
/// Returns the first registry collision or declaration error. Preflight covers
/// all three UIDs before any registration, so an error leaves every capability
/// unchanged.
pub fn register_htj2k_decoders(registry: &mut Registry) -> Result<(), RegistryError> {
    for uid in [HTJ2K_LOSSLESS_UID, HTJ2K_LOSSLESS_RPCL_UID, HTJ2K_UID] {
        match registry.capability(uid) {
            Capability::Available => return Err(RegistryError::AlreadyRegistered { uid }),
            Capability::KnownUnavailable => {}
            Capability::Unknown => return Err(RegistryError::UnknownTransferSyntax { uid }),
        }
    }
    registry.register(Arc::new(Htj2kDecoder::lossless()))?;
    registry.register(Arc::new(Htj2kDecoder::lossless_rpcl()))?;
    registry.register(Arc::new(Htj2kDecoder::general()))
}

impl Decoder for Htj2kDecoder {
    fn transfer_syntaxes(&self) -> &'static [&'static str] {
        match self.mode {
            Mode::Lossless => LOSSLESS_UIDS,
            Mode::LosslessRpcl => LOSSLESS_RPCL_UIDS,
            Mode::General => GENERAL_UIDS,
        }
    }

    fn decode_photometric_interpretation(
        &self,
        desc: &FrameDesc,
    ) -> DecodePhotometricInterpretation {
        crate::jpeg2000::decoded_photometric(desc)
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
        if self.mode != Mode::General {
            refuse_tile_coding_overrides(codestream, tile_at, self.mode == Mode::LosslessRpcl)?;
        }
        validate_header(self.mode, header, desc)?;

        let decoded = decode_rows(codestream, desc)?;
        convert_samples(&decoded, desc, out)
    }
}

#[derive(Clone)]
struct MainHeader {
    width: u32,
    height: u32,
    components: Vec<ComponentParams>,
    /// Whether CAP `(0xff50)` is present, which is what makes a codestream
    /// HTJ2K rather than JPEG 2000 Part 1.
    high_throughput: bool,
    progression: u8,
    reversible: bool,
    multiple_component_transform: u8,
    /// QCD `Sqcd` style, when a main-header QCD is present.
    quantization_style: Option<u8>,
    /// A POC marker was present. It can replace the COD progression order.
    progression_order_change: bool,
}

/// Read SIZ, COD and CAP from the main header, stopping at SOT.
fn inspect_main_header(src: &[u8]) -> Result<(MainHeader, usize), CodecError> {
    let mut offset = 2usize;
    let mut siz = None;
    let mut cod = None;
    let mut high_throughput = false;
    let mut quantization_style = None;
    let mut progression_order_change = false;
    while offset.checked_add(2).is_some_and(|end| end <= src.len()) {
        if src.get(offset) != Some(&0xff) {
            return Err(CodecError::InvalidCodestream);
        }
        let marker = *src.get(offset + 1).ok_or(CodecError::InvalidCodestream)?;
        if marker == SOT {
            break;
        }
        if matches!(marker, EOC | SOC) {
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
            SIZ => {
                if siz.is_some() {
                    return Err(CodecError::InvalidCodestream);
                }
                siz = Some(parse_siz(payload)?);
            }
            COD => {
                if cod.is_some() {
                    return Err(CodecError::InvalidCodestream);
                }
                cod = Some(parse_cod(payload)?);
            }
            CAP => {
                if high_throughput {
                    return Err(CodecError::InvalidCodestream);
                }
                high_throughput = true;
            }
            // A component override can replace the reversible wavelet or add
            // quantization after the main-header COD has been accepted.
            COC | QCC => return Err(CodecError::UnsupportedPixelFormat),
            QCD => {
                if quantization_style.is_some() {
                    return Err(CodecError::InvalidCodestream);
                }
                let (style, _) = crate::jpeg2000::parse_qcd(payload)?;
                quantization_style = Some(style);
            }
            POC => progression_order_change = true,
            _ => {}
        }
        offset = end;
    }
    let (width, height, components) = siz.ok_or(CodecError::InvalidCodestream)?;
    let (progression, multiple_component_transform, reversible) =
        cod.ok_or(CodecError::InvalidCodestream)?;
    Ok((
        MainHeader {
            width,
            height,
            components,
            high_throughput,
            progression,
            reversible,
            multiple_component_transform,
            quantization_style,
            progression_order_change,
        },
        offset,
    ))
}

/// COD, ISO/IEC 15444-1 A.6.1.
///
/// `Scod SGcod{progression, layers, mct} SPcod{levels, xcb, ycb, style,
/// transform, ...}`, so the progression order is payload byte 1, the multiple
/// component transform is byte 4, and the wavelet transform is byte 9.
fn parse_cod(payload: &[u8]) -> Result<(u8, u8, bool), CodecError> {
    if payload.len() < 10 {
        return Err(CodecError::InvalidCodestream);
    }
    let progression = *payload.get(1).ok_or(CodecError::InvalidCodestream)?;
    let multiple_component_transform = *payload.get(4).ok_or(CodecError::InvalidCodestream)?;
    let transform = *payload.get(9).ok_or(CodecError::InvalidCodestream)?;
    if !matches!(transform, 0 | 1) {
        return Err(CodecError::InvalidCodestream);
    }
    Ok((
        progression,
        multiple_component_transform,
        transform == WAVELET_REVERSIBLE_5_3,
    ))
}

fn validate_header(mode: Mode, header: MainHeader, desc: &FrameDesc) -> Result<(), CodecError> {
    // CAP is what separates HTJ2K from JPEG 2000 Part 1. Without this check a
    // Part 1 codestream would decode through an HTJ2K transfer syntax, which is
    // a file being read as something it does not claim to be.
    if !header.high_throughput {
        return Err(CodecError::FrameMismatch);
    }
    validate_colour_shape(
        &header.components,
        header.multiple_component_transform,
        desc,
    )?;
    if header.components.len() == 3 {
        validate_colour_components(&header.components)?;
    }
    let first = header
        .components
        .first()
        .copied()
        .ok_or(CodecError::InvalidCodestream)?;
    let signed = desc.pixel_representation() == PixelRepresentation::Signed;
    if header.width != u32::from(desc.columns())
        || header.height != u32::from(desc.rows())
        || u16::from(first.precision) != desc.bits_stored()
        || first.signed != signed
    {
        return Err(CodecError::FrameMismatch);
    }
    // PS3.5 A.4.9, A.4.10 and A.4.11.
    //
    // `.201` and `.202` are lossless only, so the wavelet transform must be the
    // reversible 5/3 kernel. `.203` is the general form and constrains neither
    // the transform nor the progression order.
    //
    // **`.202` additionally requires RPCL and `.201` does not forbid it**, so a
    // `.202` codestream is also a valid `.201` one. That asymmetry is the
    // standard's and is not smoothed over here: inventing a non-RPCL constraint
    // for `.201` would refuse conformant files.
    match mode {
        Mode::Lossless => {
            if !header.reversible {
                return Err(CodecError::FrameMismatch);
            }
        }
        Mode::LosslessRpcl => {
            if !header.reversible
                || header.progression != PROGRESSION_RPCL
                || header.progression_order_change
            {
                return Err(CodecError::FrameMismatch);
            }
        }
        Mode::General => {}
    }
    // PS3.5 A.4.4's no-quantization rule, applied to the HTJ2K lossless UIDs.
    // A reversible COD with a scalar or expounded QCD is still lossy.
    if matches!(mode, Mode::Lossless | Mode::LosslessRpcl) {
        match header.quantization_style {
            Some(0) => {}
            Some(_) => return Err(CodecError::FrameMismatch),
            None => return Err(CodecError::InvalidCodestream),
        }
    }
    Ok(())
}

/// Decode every image row and flatten it into one owned sample buffer.
///
/// Deviation D-21. `openjph-core` 0.1.0 has no API that decodes into a
/// caller-provided slice, and each `pull` returns a fresh `Vec<i32>` for one
/// row, so a frame costs at least one allocation per row plus this one. The
/// cost is bounded by the descriptor, which was already checked against the
/// codestream's own SIZ, and the caller's output is not touched until every
/// sample has been validated by `convert_samples`.
///
/// A colour frame's samples are interleaved, so the three components are
/// pulled row by row into one sample stream. The codestream's own inverse
/// component transform has already run inside the vendored decode.
fn decode_rows(codestream: &[u8], desc: &FrameDesc) -> Result<Vec<f32>, CodecError> {
    let rows = usize::from(desc.rows());
    let columns = usize::from(desc.columns());
    let components = usize::from(desc.samples_per_pixel());
    let samples = rows
        .checked_mul(columns)
        .and_then(|pixels| pixels.checked_mul(components))
        .ok_or(CodecError::FrameMismatch)?;

    let mut input = MemInfile::new(codestream);
    let mut decoder = Codestream::new();
    decoder
        .read_headers(&mut input)
        .map_err(|_| CodecError::InvalidCodestream)?;
    decoder
        .create(&mut input)
        .map_err(|_| CodecError::DecoderFailure)?;

    let mut decoded = Vec::with_capacity(samples);
    for _ in 0..rows {
        let mut lines: Vec<Vec<i32>> = Vec::with_capacity(components);
        for component in 0..components {
            let component =
                u32::try_from(component).map_err(|_| CodecError::UnsupportedPixelFormat)?;
            let line = decoder.pull(component).ok_or(CodecError::FrameMismatch)?;
            if line.len() != columns {
                return Err(CodecError::FrameMismatch);
            }
            lines.push(line);
        }
        // Sample-interleaved output: the three components' samples for one
        // pixel are adjacent, so the rows are zipped and not concatenated.
        for column in 0..columns {
            for line in &lines {
                // Every line was checked to be `columns` long, so the index
                // below is in range, and `get` says so to the borrow checker.
                let value = line.get(column).copied().ok_or(CodecError::FrameMismatch)?;
                // `Stored`'s representation is `f32`, and `sample_convert` refuses
                // anything that is not exactly an integer inside the descriptor's
                // stored range. Every `i32` through 24 significant bits converts
                // exactly, and a wider one is caught there rather than here.
                decoded.push(exact_f32(value).ok_or(CodecError::UnsupportedPixelFormat)?);
            }
        }
    }
    // A frame that still has rows is not the frame the descriptor described.
    for component in 0..components {
        let component = u32::try_from(component).map_err(|_| CodecError::UnsupportedPixelFormat)?;
        if decoder.pull(component).is_some() {
            return Err(CodecError::FrameMismatch);
        }
    }
    if decoded.len() != samples {
        return Err(CodecError::FrameMismatch);
    }
    Ok(decoded)
}

/// Convert one decoded `i32` sample to `f32` without losing a bit.
///
/// `f32` carries 24 significant bits, so an arbitrary `i32` can round. This
/// adapter accepts Bits Allocated 8 or 16 only, so every legitimate sample lies
/// in `-32_768 ..= 65_535`, which is the union of the signed and unsigned
/// 16-bit stored domains and is well inside 24 bits. Both `From` conversions
/// below are lossless by construction, and a value outside that union is a
/// refusal rather than a rounded sample.
///
/// There is no `as` cast here deliberately. `i32 as f32` would round silently
/// on exactly the values this function exists to catch.
fn exact_f32(value: i32) -> Option<f32> {
    if let Ok(signed) = i16::try_from(value) {
        return Some(f32::from(signed));
    }
    if let Ok(unsigned) = u16::try_from(value) {
        return Some(f32::from(unsigned));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::exact_f32;

    /// The `f32` bit pattern of an integer known to be exactly
    /// representable, built without going through the conversion under
    /// test. `i16` and `u16` both have lossless `From` impls for `f32`.
    fn f64_exact_bits(value: i32) -> u32 {
        i16::try_from(value).map_or_else(
            |_| f32::from(u16::try_from(value).unwrap_or(0)).to_bits(),
            |narrow| f32::from(narrow).to_bits(),
        )
    }

    #[test]
    fn sample_conversion_covers_both_sixteen_bit_domains_and_refuses_outside() {
        // The union of the signed and unsigned 16-bit stored domains.
        for value in [-32_768_i32, -1, 0, 32_767, 32_768, 65_535] {
            let converted = exact_f32(value);
            assert!(converted.is_some(), "{value} must convert");
            if let Some(converted) = converted {
                assert_eq!(converted.to_bits(), f64_exact_bits(value));
            }
        }

        // Outside it, refused rather than rounded.
        assert_eq!(exact_f32(-32_769), None);
        assert_eq!(exact_f32(65_536), None);
        assert_eq!(exact_f32(i32::MIN), None);
        assert_eq!(exact_f32(i32::MAX), None);
    }

    #[test]
    fn every_value_in_the_union_round_trips_exactly() {
        // Not a sample. Every one of the 98,304 values, because "exact except
        // near the top of the range" is the defect this check exists for.
        for value in -32_768..=65_535_i32 {
            let converted = exact_f32(value);
            assert!(converted.is_some());
            if let Some(converted) = converted {
                assert_eq!(f64::from(converted).to_bits(), f64::from(value).to_bits());
            }
        }
    }
}
