//! Cross-target execution proof for the production JPEG 2000 adapter.

use ocelli_codec::{
    Decoder, FrameDesc, FrameDescInput, Jpeg2000Decoder, PixelDataVr, PixelRepresentation,
};

fn main() {
    if verify().is_err() {
        std::process::abort();
    }
}

fn verify() -> Result<(), ocelli_codec::CodecError> {
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
    .map_err(|_| ocelli_codec::CodecError::FrameMismatch)?;
    let expected = include_bytes!("../tests/fixtures/jpeg2000_corpus_reference_u16le.raw");
    let mut out = vec![0xa5; desc.output_len()];
    Jpeg2000Decoder::lossless_only().decode(
        include_bytes!("../tests/fixtures/jpeg2000_corpus_j2k_lossless.j2k"),
        &desc,
        &mut out,
    )?;
    if out.as_slice() != expected {
        return Err(ocelli_codec::CodecError::FrameMismatch);
    }
    out.fill(0xa5);
    Jpeg2000Decoder::general().decode(
        include_bytes!("../tests/fixtures/jpeg2000_corpus_j2k_lossy.j2k"),
        &desc,
        &mut out,
    )?;
    let mut differing = 0_u64;
    let mut signed_sum = 0_i64;
    let mut max_abs = 0_u16;
    for (actual, wanted) in out.chunks_exact(2).zip(expected.chunks_exact(2)) {
        let actual = u16::from_le_bytes(
            <[u8; 2]>::try_from(actual).map_err(|_| ocelli_codec::CodecError::FrameMismatch)?,
        );
        let wanted = u16::from_le_bytes(
            <[u8; 2]>::try_from(wanted).map_err(|_| ocelli_codec::CodecError::FrameMismatch)?,
        );
        differing += u64::from(actual != wanted);
        signed_sum += i64::from(actual) - i64::from(wanted);
        max_abs = max_abs.max(actual.abs_diff(wanted));
    }
    if (differing, signed_sum, max_abs) != (418, 30, 1) {
        return Err(ocelli_codec::CodecError::FrameMismatch);
    }
    Ok(())
}
