//! THROWAWAY SPIKE CODE. F-X006, Appendix A gate A2.
//!
//! Decodes the two JPEG-LS corpus codestreams through each pure-Rust candidate
//! and hands the canonical 12288 bytes back to a caller. Two callers exist:
//! `src/main.rs`, the native decode, and `run.mjs`, which instantiates the
//! `wasm32-unknown-unknown` build under node. Both reach the same functions.
//!
//! Not held to the gate set, per `/spike` step 2. It is not a workspace member,
//! so `gate clippy` and `gate test` never see it. `gate unsafe`, `gate prose`,
//! `gate provenance` and `gate content` read `git ls-files` and do see it.
//!
//! **No `unsafe`, no pointer dereference in Rust, and errors leave as raw
//! integers.** `CodecError` is F-005's to shape and E2.6's to populate.
//!
//! **Route R2, `charls` 0.4.2 over `charls-sys` 2.4.5, is not a dependency of
//! this crate.** It builds a vendored C++ CharLS through `cmake`, which is not
//! installed here, and making this harness depend on it would make every other
//! candidate unmeasurable on the same machine. Its build is measured on its own
//! and the verbatim error is recorded in the answer file.

use core::cell::RefCell;

/// `1.2.840.10008.1.2.4.80`, JPEG-LS lossless.
pub const CASE_LOSSLESS: u32 = 0;
/// `1.2.840.10008.1.2.4.81`, JPEG-LS near-lossless, `NEAR` 3.
pub const CASE_NEAR_LOSSLESS: u32 = 1;

/// R3, `pure_jpegls` 2.0.0.
pub const CANDIDATE_PURE_JPEGLS: u32 = 0;
/// R3b, `dicom-toolkit-codec` 0.5.0.
pub const CANDIDATE_DICOM_TOOLKIT: u32 = 1;
/// R3b, `ritk-codecs` 0.6.0.
pub const CANDIDATE_RITK: u32 = 2;

/// Raw decoder error codes. Not `CodecError`, deliberately.
pub const OK: u32 = 0;
pub const ERR_UNKNOWN_CASE: u32 = 1;
pub const ERR_UNKNOWN_CANDIDATE: u32 = 2;
pub const ERR_DECODE_REFUSED: u32 = 3;
pub const ERR_SAMPLE_COUNT: u32 = 4;
pub const ERR_SAMPLE_RANGE: u32 = 5;
pub const ERR_SAMPLE_NOT_INTEGRAL: u32 = 6;
pub const ERR_BYTE_COUNT: u32 = 7;

/// The canonical form, from `scripts/corpus_synth.py`.
pub const ROWS: u32 = 64;
pub const COLS: u32 = 96;
pub const SAMPLES: usize = 6144;
pub const CANONICAL_BYTES: usize = 12288;

// Embedded at compile time from ignored `tools/spikes/out/`.
const JPEGLS_LOSSLESS: &[u8] = include_bytes!("../../out/jpegls_lossless.jls");
const JPEGLS_NEAR_LOSSLESS: &[u8] =
    include_bytes!("../../out/jpegls_near_lossless.jls");

thread_local! {
    static OUT: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
}

fn codestream(case: u32) -> Option<&'static [u8]> {
    match case {
        CASE_LOSSLESS => Some(JPEGLS_LOSSLESS),
        CASE_NEAR_LOSSLESS => Some(JPEGLS_NEAR_LOSSLESS),
        _ => None,
    }
}

fn pack(samples: &[u16]) -> Result<Vec<u8>, u32> {
    if samples.len() != SAMPLES {
        return Err(ERR_SAMPLE_COUNT);
    }
    let mut out = Vec::with_capacity(CANONICAL_BYTES);
    for sample in samples {
        out.extend_from_slice(&sample.to_le_bytes());
    }
    Ok(out)
}

/// R3, `pure_jpegls` 2.0.0. Returns `u16` samples directly.
#[cfg(feature = "pure-jpegls")]
fn decode_pure_jpegls(src: &[u8]) -> Result<Vec<u8>, u32> {
    let (samples, _width, _height) =
        jpegls::decode(src, COLS, ROWS).map_err(|_| ERR_DECODE_REFUSED)?;
    pack(&samples)
}

/// R3b, `dicom-toolkit-codec` 0.5.0. Returns native pixel BYTES, already
/// little-endian for a multi-byte sample, which is the canonical form.
///
/// Its own module header says it is "a port of the CharLS algorithm - no C/C++
/// dependencies". CharLS is BSD-3 and is in `docs/SOURCE-POLICY.md`'s yes
/// column, so the provenance question that matters is answered and recorded.
#[cfg(feature = "dicom-toolkit")]
fn decode_dicom_toolkit(src: &[u8]) -> Result<Vec<u8>, u32> {
    let frame = dicom_toolkit_codec::JpegLsCodec::decode_frame(src)
        .map_err(|_| ERR_DECODE_REFUSED)?;
    if frame.width != COLS
        || frame.height != ROWS
        || frame.components != 1
        || frame.bits_per_sample != 16
    {
        return Err(ERR_DECODE_REFUSED);
    }
    if frame.pixels.len() != CANONICAL_BYTES {
        return Err(ERR_BYTE_COUNT);
    }
    Ok(frame.pixels)
}

/// Convert one `f32` that is required to be an exact integer sample.
///
/// `ritk-codecs` hands back `f32` with the modality rescale already applied.
/// The layout below sets slope 1 and intercept 0, so an unmodified sample is
/// expected and anything else is a refusal rather than a rounding. The equality
/// against `trunc()` is an INTEGRALITY TEST and not an approximate comparison,
/// which is the one shape where comparing floats is the correct tool. Every
/// `u16` is exactly representable in `f32`, so a legitimate sample passes it.
#[cfg(feature = "ritk")]
fn f32_sample_to_u16(value: f32) -> Result<u16, u32> {
    if !(0.0..=65535.0).contains(&value) {
        return Err(ERR_SAMPLE_RANGE);
    }
    let truncated = value.trunc();
    if truncated != value {
        return Err(ERR_SAMPLE_NOT_INTEGRAL);
    }
    // Proved above to be an integer in 0..=65535, so this conversion is exact.
    Ok(truncated as u16)
}

/// R3b, `ritk-codecs` 0.6.0. Returns `f32` with the rescale already applied.
///
/// **That API shape is itself a finding.** HLD section 18 requires the LUT
/// chain to exist exactly once, in `ocelli-pixel`. A codec that applies the
/// modality rescale is a second place where that arithmetic lives, and the
/// answer file records it as an integration cost rather than a preference.
#[cfg(feature = "ritk")]
fn decode_ritk(src: &[u8]) -> Result<Vec<u8>, u32> {
    let layout = ritk_codecs::PixelLayout {
        rows: 64,
        cols: 96,
        samples_per_pixel: 1,
        bits_allocated: 16,
        pixel_representation: ritk_codecs::PixelSignedness::Unsigned,
        rescale_slope: 1.0,
        rescale_intercept: 0.0,
    };
    let values = ritk_codecs::decode_jpeg_ls_fragment(src, layout)
        .map_err(|_| ERR_DECODE_REFUSED)?;
    if values.len() != SAMPLES {
        return Err(ERR_SAMPLE_COUNT);
    }
    let mut samples = Vec::with_capacity(SAMPLES);
    for value in &values {
        samples.push(f32_sample_to_u16(*value)?);
    }
    pack(&samples)
}

/// Decode one embedded codestream with one candidate.
pub fn decode_case(candidate: u32, case: u32) -> Result<Vec<u8>, u32> {
    let src = codestream(case).ok_or(ERR_UNKNOWN_CASE)?;
    match candidate {
        #[cfg(feature = "pure-jpegls")]
        CANDIDATE_PURE_JPEGLS => decode_pure_jpegls(src),
        #[cfg(feature = "dicom-toolkit")]
        CANDIDATE_DICOM_TOOLKIT => decode_dicom_toolkit(src),
        #[cfg(feature = "ritk")]
        CANDIDATE_RITK => decode_ritk(src),
        _ => Err(ERR_UNKNOWN_CANDIDATE),
    }
}

/// The candidate's own error text for one decode, for the record.
///
/// The exported `decode` reduces every failure to an integer, because an error
/// model is F-005's and a spike must not pre-empt it. A gate answer that says
/// only "raw error code 3" is not an answer anyone can act on, so the native
/// caller re-runs the decode through this and prints what the library said.
pub fn diagnose(candidate: u32, case: u32) -> String {
    let Some(src) = codestream(case) else {
        return "unknown case".to_string();
    };
    match candidate {
        #[cfg(feature = "pure-jpegls")]
        CANDIDATE_PURE_JPEGLS => match jpegls::decode(src, COLS, ROWS) {
            Ok((samples, width, height)) => {
                format!("ok, {} samples, {width}x{height}", samples.len())
            }
            Err(error) => format!("{error:?}"),
        },
        #[cfg(feature = "dicom-toolkit")]
        CANDIDATE_DICOM_TOOLKIT => {
            match dicom_toolkit_codec::JpegLsCodec::decode_frame(src) {
                Ok(frame) => format!(
                    "ok, {} bytes, {}x{}, {} bits, {} components",
                    frame.pixels.len(),
                    frame.width,
                    frame.height,
                    frame.bits_per_sample,
                    frame.components
                ),
                Err(error) => format!("{error:?}"),
            }
        }
        #[cfg(feature = "ritk")]
        CANDIDATE_RITK => {
            let layout = ritk_codecs::PixelLayout {
                rows: 64,
                cols: 96,
                samples_per_pixel: 1,
                bits_allocated: 16,
                pixel_representation: ritk_codecs::PixelSignedness::Unsigned,
                rescale_slope: 1.0,
                rescale_intercept: 0.0,
            };
            match ritk_codecs::decode_jpeg_ls_fragment(src, layout) {
                Ok(values) => format!("ok, {} values", values.len()),
                Err(error) => format!("{error:?}"),
            }
        }
        _ => "unknown candidate".to_string(),
    }
}

/// Decode into the module-owned buffer. Returns `OK` or an error code.
#[no_mangle]
pub extern "C" fn decode(candidate: u32, case: u32) -> u32 {
    OUT.with(|cell| {
        cell.borrow_mut().clear();
    });
    match decode_case(candidate, case) {
        Ok(bytes) => {
            OUT.with(|cell| {
                *cell.borrow_mut() = bytes;
            });
            OK
        }
        Err(code) => code,
    }
}

/// Length of the decoded buffer. The driver asserts this is 12288.
#[no_mangle]
pub extern "C" fn out_len() -> u32 {
    OUT.with(|cell| u32::try_from(cell.borrow().len()).unwrap_or(0))
}

/// Offset of the decoded buffer in linear memory. See the A1 harness for why
/// this is `addr()` and `try_from` rather than an `as` cast.
#[no_mangle]
pub extern "C" fn out_ptr() -> u32 {
    OUT.with(|cell| u32::try_from(cell.borrow().as_ptr().addr()).unwrap_or(0))
}
