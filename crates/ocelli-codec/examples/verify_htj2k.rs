//! Cross-target execution proof for the production HTJ2K adapter.
//!
//! F-X013's central claim was that `openjph-core` 0.1.0 produces identical
//! samples on native, plain wasm and `+simd128` wasm, which is what makes one
//! implementation across browser, desktop and server credible. That claim was
//! measured once in a throwaway spike harness that no longer exists. This file
//! is the standing version of it, over the production adapter, and it is the
//! same source the `native` gate runs on all three targets.
//!
//! All three transfer syntaxes are checked. `.201` and `.202` must reproduce
//! the uncompressed reference exactly. `.203` is irreversible, so it is checked
//! against the exact output F-X013 recorded for the candidate rather than
//! against the reference, using a checksum rather than a digest because this
//! example has no hashing dependency on the wasm target.

use ocelli_codec::{
    CodecError, Decoder, FrameDesc, FrameDescInput, Htj2kDecoder, PixelDataVr, PixelRepresentation,
};

/// The sum of `.203`'s decoded bytes, from the run that produced F-X013's
/// digest `ce4a2bb9d75b897292a4e9e9e7455447e976f5ff1e18b4ecb3219bb17d4d062c`.
/// `crates/ocelli-codec/tests/htj2k.rs` pins the digest itself.
const LOSSY_CHECKSUM: u64 = 1_559_962;

fn main() {
    if verify().is_err() {
        std::process::abort();
    }
}

fn verify() -> Result<(), CodecError> {
    let desc = FrameDesc::new(FrameDescInput {
        rows: 64,
        columns: 96,
        samples_per_pixel: 1,
        bits_allocated: 16,
        bits_stored: 16,
        high_bit: 15,
        pixel_representation: PixelRepresentation::Unsigned,
        photometric_interpretation: "MONOCHROME2".to_owned(),
        pixel_data_vr: PixelDataVr::Ob,
    })
    .map_err(|_| CodecError::FrameMismatch)?;
    let expected = include_bytes!("../tests/fixtures/jpegls_corpus_reference_u16le.raw");

    let mut out = vec![0xa5; desc.output_len()];
    Htj2kDecoder::lossless().decode(
        include_bytes!("../tests/fixtures/htj2k_corpus_lossless.j2c"),
        &desc,
        &mut out,
    )?;
    if out.as_slice() != expected {
        return Err(CodecError::FrameMismatch);
    }

    out.fill(0xa5);
    Htj2kDecoder::lossless_rpcl().decode(
        include_bytes!("../tests/fixtures/htj2k_corpus_lossless_rpcl.j2c"),
        &desc,
        &mut out,
    )?;
    if out.as_slice() != expected {
        return Err(CodecError::FrameMismatch);
    }

    out.fill(0xa5);
    Htj2kDecoder::general().decode(
        include_bytes!("../tests/fixtures/htj2k_corpus_lossy.j2c"),
        &desc,
        &mut out,
    )?;
    let checksum = out
        .iter()
        .fold(0_u64, |sum, byte| sum.wrapping_add(u64::from(*byte)));
    if checksum != LOSSY_CHECKSUM {
        return Err(CodecError::FrameMismatch);
    }
    // The irreversible row must also still BE irreversible. A fixture that
    // quietly became lossless would satisfy the checksum only by coincidence,
    // and this says so directly.
    if out.as_slice() == expected {
        return Err(CodecError::FrameMismatch);
    }
    Ok(())
}
