//! HTJ2K conformance fixtures for F-027.
//!
//! Built by `tests/fixtures/generate_htj2k.py`, which records where each
//! codestream came from and asserts its CAP marker, progression order and
//! wavelet kernel before writing it.

use std::error::Error;

use ocelli_codec::{
    Capability, CodecError, DecodePhotometricInterpretation, DecodeSampleLayout, Decoder,
    FrameDesc, FrameDescInput, Htj2kDecoder, PixelDataVr, PixelRepresentation, Registry,
    RegistryError, register_htj2k_decoders,
};

type TestResult = Result<(), Box<dyn Error>>;

/// One differing sample as `(index, actual, expected)`.
type Difference = (usize, u16, u16);

const HTJ2K_LOSSLESS: &str = "1.2.840.10008.1.2.4.201";
const HTJ2K_LOSSLESS_RPCL: &str = "1.2.840.10008.1.2.4.202";
const HTJ2K: &str = "1.2.840.10008.1.2.4.203";

#[test]
fn both_lossless_syntaxes_reproduce_the_uncompressed_reference_exactly() -> TestResult {
    // `corpus/manifest.tsv` rows 35 and 36. Both are lossless, so both must
    // reproduce `syntax/explicit_vr_le.dcm`'s Pixel Data byte for byte. That
    // reference is the synthetic ramp from scripts/corpus_synth.py and comes
    // from no codec, so it is an encoder-independent anchor.
    let reference = include_bytes!("fixtures/jpegls_corpus_reference_u16le.raw");
    let desc = corpus_desc()?;

    let mut out = vec![0xa5; desc.output_len()];
    Htj2kDecoder::lossless().decode(
        include_bytes!("fixtures/htj2k_corpus_lossless.j2c"),
        &desc,
        &mut out,
    )?;
    assert_eq!(out.as_slice(), reference.as_slice());

    let mut out = vec![0xa5; desc.output_len()];
    Htj2kDecoder::lossless_rpcl().decode(
        include_bytes!("fixtures/htj2k_corpus_lossless_rpcl.j2c"),
        &desc,
        &mut out,
    )?;
    assert_eq!(out.as_slice(), reference.as_slice());
    Ok(())
}

/// The `.203` decode F-X013 measured, over the canonical 12,288 bytes.
///
/// F-X013 recorded this for `D_native`, `D_wasm` and `D_simd` alike, so pinning
/// it here asserts that the production adapter reproduces the spike's
/// measurement through an entirely different code path.
const LOSSY_SHA256: &str = "ce4a2bb9d75b897292a4e9e9e7455447e976f5ff1e18b4ecb3219bb17d4d062c";

#[test]
fn the_irreversible_syntax_reproduces_the_measurement_f_x013_recorded() -> TestResult {
    // `corpus/manifest.tsv` row 37, transfer syntax `.203`.
    //
    // **The reference ramp is NOT an anchor for this row and the story's design
    // plan was wrong about how to assert it.** The plan said to pin F-X013's
    // "41 of 6144" divergence. That figure is the candidate measured against
    // OpenJPH 0.31.0's own output, which is not in this tree: `ojph_expand` is
    // an external tool the spike ran once. Against the uncompressed ramp this
    // irreversible decode differs at 5,377 of 6,144 samples, which is what a
    // lossy codec does and is not a defect.
    //
    // What CAN be asserted here is stronger than a tolerance and is what
    // decision D14 means by a measured divergence: F-X013 recorded the
    // candidate's own output digest, identical across its native, plain wasm
    // and SIMD wasm builds, and the production adapter reproduces it exactly.
    // If the decode ever changes, this fails, and the OpenJPH comparison the
    // spike recorded would have to be re-measured rather than assumed.
    let desc = corpus_desc()?;
    let mut out = vec![0xa5; desc.output_len()];
    Htj2kDecoder::general().decode(
        include_bytes!("fixtures/htj2k_corpus_lossy.j2c"),
        &desc,
        &mut out,
    )?;
    assert_eq!(out.len(), 12_288);
    assert_eq!(hex_sha256(&out), LOSSY_SHA256);

    // And it really is lossy, so a `.203` fixture cannot silently be a lossless
    // one. The count is recorded rather than bounded.
    let differing = differences(
        &out,
        include_bytes!("fixtures/jpegls_corpus_reference_u16le.raw"),
    )?;
    assert_eq!(differing.len(), 5_377);
    Ok(())
}

#[test]
fn a_part_one_codestream_is_refused_because_it_carries_no_cap_marker() -> TestResult {
    // CAP `(0xff50)` is what makes a codestream HTJ2K rather than JPEG 2000
    // Part 1. `fixtures/jpeg2000_corpus_j2k_lossless.j2k` is the same synthetic
    // ramp encoded as Part 1, so its dimensions, precision and reversibility
    // all match the descriptor and every other check passes. Without the CAP
    // check it would decode through an HTJ2K transfer syntax, which is a file
    // being read as something it does not claim to be.
    let desc = corpus_desc()?;
    for decoder in [
        Htj2kDecoder::lossless(),
        Htj2kDecoder::lossless_rpcl(),
        Htj2kDecoder::general(),
    ] {
        let mut out = vec![0xa5; desc.output_len()];
        assert_eq!(
            decoder.decode(
                include_bytes!("fixtures/jpeg2000_corpus_j2k_lossless.j2k"),
                &desc,
                &mut out,
            ),
            Err(CodecError::FrameMismatch)
        );
        assert!(out.iter().all(|byte| *byte == 0xa5));
    }
    Ok(())
}

#[test]
fn the_rpcl_constraint_is_one_directional_because_ps3_5_makes_it_so() -> TestResult {
    // PS3.5 A.4.10 requires RPCL progression for `.202`. A.4.9 constrains
    // nothing about progression for `.201`.
    //
    // So `.202` refuses the LRCP row, and `.201` ACCEPTS the RPCL row. The
    // second half is the one worth asserting: a `.202` codestream is also a
    // valid `.201` one, and inventing a non-RPCL constraint for `.201` to make
    // the two mutually exclusive would refuse conformant files.
    let reference = include_bytes!("fixtures/jpegls_corpus_reference_u16le.raw");
    let desc = corpus_desc()?;

    let mut out = vec![0xa5; desc.output_len()];
    assert_eq!(
        Htj2kDecoder::lossless_rpcl().decode(
            include_bytes!("fixtures/htj2k_corpus_lossless.j2c"),
            &desc,
            &mut out,
        ),
        Err(CodecError::FrameMismatch)
    );
    assert!(out.iter().all(|byte| *byte == 0xa5));

    let mut out = vec![0xa5; desc.output_len()];
    Htj2kDecoder::lossless().decode(
        include_bytes!("fixtures/htj2k_corpus_lossless_rpcl.j2c"),
        &desc,
        &mut out,
    )?;
    assert_eq!(out.as_slice(), reference.as_slice());
    Ok(())
}

#[test]
fn the_lossless_syntaxes_refuse_an_irreversible_codestream() -> TestResult {
    // PS3.5 A.4.9 and A.4.10 are lossless only, so the wavelet transform must
    // be the reversible 5/3 kernel. The general form accepts either, which is
    // the same asymmetry `.90` and `.91` already have.
    let desc = corpus_desc()?;
    for decoder in [Htj2kDecoder::lossless(), Htj2kDecoder::lossless_rpcl()] {
        let mut out = vec![0xa5; desc.output_len()];
        assert_eq!(
            decoder.decode(
                include_bytes!("fixtures/htj2k_corpus_lossy.j2c"),
                &desc,
                &mut out,
            ),
            Err(CodecError::FrameMismatch)
        );
        assert!(out.iter().all(|byte| *byte == 0xa5));
    }

    // And the general form accepts a reversible codestream, so the constraint
    // refuses the wrong pairing rather than everything.
    let mut out = vec![0xa5; desc.output_len()];
    Htj2kDecoder::general().decode(
        include_bytes!("fixtures/htj2k_corpus_lossless.j2c"),
        &desc,
        &mut out,
    )?;
    assert_eq!(
        out.as_slice(),
        include_bytes!("fixtures/jpegls_corpus_reference_u16le.raw").as_slice()
    );
    Ok(())
}

#[test]
fn a_multi_component_codestream_is_refused_by_the_header_check() -> TestResult {
    // No multi-component HTJ2K corpus row exists, and the descriptor check
    // cannot reach this condition: a three-sample descriptor is refused before
    // the codestream is read. So the SIZ of the `.201` fixture is rewritten
    // here to declare `Csiz = 3` with three component specifiers, which is the
    // smallest edit that makes a structurally valid multi-component main
    // header out of a single-component one.
    //
    // Everything before the component check passes, including CAP, so the
    // refusal that fires is the one this test is for.
    let source = include_bytes!("fixtures/htj2k_corpus_lossless.j2c");
    // ISO/IEC 15444-1 A.5.1. `Rsiz Xsiz Ysiz XOsiz YOsiz XTsiz YTsiz XTOsiz
    // YTOsiz Csiz (Ssiz XRsiz YRsiz)*Csiz`, 96 by 64, one tile, 16-bit
    // unsigned, with Csiz raised from 1 to 3 and its specifier repeated.
    const THREE_COMPONENT_SIZ: &[u8] = &[
        0xff, 0x51, 0x00, 0x2f, 0x40, 0x00, 0x00, 0x00, 0x00, 0x60, 0x00, 0x00, 0x00, 0x40, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x60, 0x00, 0x00, 0x00, 0x40,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x03, 0x0f, 0x01, 0x01, 0x0f, 0x01,
        0x01, 0x0f, 0x01, 0x01,
    ];
    // The original SIZ is 43 bytes and begins at offset 2, right after SOC.
    let head = source.get(..2).ok_or("fixture is shorter than SOC")?;
    let tail = source.get(45..).ok_or("fixture is shorter than its SIZ")?;
    assert_eq!(source.get(2..4), Some(&[0xff, 0x51][..]));
    let mut spliced = Vec::with_capacity(head.len() + THREE_COMPONENT_SIZ.len() + tail.len());
    spliced.extend_from_slice(head);
    spliced.extend_from_slice(THREE_COMPONENT_SIZ);
    spliced.extend_from_slice(tail);

    let desc = corpus_desc()?;
    let mut out = vec![0xa5; desc.output_len()];
    assert_eq!(
        Htj2kDecoder::lossless().decode(&spliced, &desc, &mut out),
        Err(CodecError::UnsupportedPixelFormat)
    );
    assert!(out.iter().all(|byte| *byte == 0xa5));
    Ok(())
}

#[test]
fn descriptor_and_geometry_mismatches_preserve_caller_output() -> TestResult {
    let source = include_bytes!("fixtures/htj2k_corpus_lossless.j2c");

    // Dimensions disagreeing with SIZ.
    let wrong = frame(32, 96, 16, 16, PixelRepresentation::Unsigned, "MONOCHROME2")?;
    assert_atomic_error(source, &wrong, CodecError::FrameMismatch);

    // Bits Stored disagreeing with SIZ's precision.
    let narrow = frame(64, 96, 16, 12, PixelRepresentation::Unsigned, "MONOCHROME2")?;
    assert_atomic_error(source, &narrow, CodecError::FrameMismatch);

    // Signedness disagreeing with SIZ.
    let signed = frame(64, 96, 16, 16, PixelRepresentation::Signed, "MONOCHROME2")?;
    assert_atomic_error(source, &signed, CodecError::FrameMismatch);

    // A colour descriptor, refused before the codestream is read.
    let colour = FrameDesc::new(FrameDescInput {
        rows: 64,
        columns: 96,
        samples_per_pixel: 3,
        bits_allocated: 8,
        bits_stored: 8,
        high_bit: 7,
        pixel_representation: PixelRepresentation::Unsigned,
        photometric_interpretation: "RGB".to_owned(),
        pixel_data_vr: PixelDataVr::Ob,
    })?;
    assert_atomic_error(source, &colour, CodecError::UnsupportedPixelFormat);

    // Other Word Pixel Data. PS3.5 A.4 encapsulates with OB.
    let word = frame_with_vr(
        64,
        96,
        16,
        16,
        PixelRepresentation::Unsigned,
        "MONOCHROME2",
        PixelDataVr::Ow,
    )?;
    assert_atomic_error(source, &word, CodecError::UnsupportedPixelFormat);
    Ok(())
}

#[test]
fn structural_failures_are_distinguished_from_each_other() -> TestResult {
    let source = include_bytes!("fixtures/htj2k_corpus_lossless.j2c");
    let desc = corpus_desc()?;

    // Not a codestream at all.
    let head = source
        .get(..3)
        .ok_or("fixture is shorter than its own header")?;
    assert_atomic_error(head, &desc, CodecError::InvalidCodestream);
    // Truncated before EOC.
    let truncated = source
        .get(..source.len() - 8)
        .ok_or("fixture is shorter than the truncation")?;
    assert_atomic_error(truncated, &desc, CodecError::InvalidCodestream);

    // PS3.5 A.4's single pad byte is not trailing data and must still decode.
    let mut padded = source.to_vec();
    padded.push(0);
    let mut out = vec![0xa5; desc.output_len()];
    Htj2kDecoder::lossless().decode(&padded, &desc, &mut out)?;
    assert_eq!(
        out.as_slice(),
        include_bytes!("fixtures/jpegls_corpus_reference_u16le.raw").as_slice()
    );

    // Two extra bytes is a complete image followed by something else.
    let mut trailing = source.to_vec();
    trailing.extend_from_slice(&[0x12, 0x34]);
    assert_atomic_error(&trailing, &desc, CodecError::TrailingData);

    // The caller's buffer is the wrong length.
    let mut short = vec![0xa5; desc.output_len() - 2];
    assert_eq!(
        Htj2kDecoder::lossless().decode(source, &desc, &mut short),
        Err(CodecError::OutputLength {
            expected: desc.output_len(),
            actual: desc.output_len() - 2,
        })
    );
    assert!(short.iter().all(|byte| *byte == 0xa5));
    Ok(())
}

#[test]
fn registration_covers_three_uids_atomically() -> TestResult {
    let mut registry = Registry::new();
    for uid in [HTJ2K_LOSSLESS, HTJ2K_LOSSLESS_RPCL, HTJ2K] {
        assert_eq!(registry.capability(uid), Capability::KnownUnavailable);
    }

    register_htj2k_decoders(&mut registry)?;
    for uid in [HTJ2K_LOSSLESS, HTJ2K_LOSSLESS_RPCL, HTJ2K] {
        assert_eq!(registry.capability(uid), Capability::Available);
    }

    assert_eq!(
        register_htj2k_decoders(&mut registry),
        Err(RegistryError::AlreadyRegistered {
            uid: HTJ2K_LOSSLESS
        })
    );

    let desc = corpus_desc()?;
    assert_eq!(
        registry.decode_photometric_interpretation(HTJ2K_LOSSLESS, &desc)?,
        DecodePhotometricInterpretation::Preserved
    );
    assert_eq!(
        registry.decode_sample_layout(HTJ2K_LOSSLESS, &desc)?,
        DecodeSampleLayout::Preserved
    );

    let mut out = vec![0xa5; desc.output_len()];
    registry.decode(
        HTJ2K_LOSSLESS,
        include_bytes!("fixtures/htj2k_corpus_lossless.j2c"),
        &desc,
        &mut out,
    )?;
    assert_eq!(
        out.as_slice(),
        include_bytes!("fixtures/jpegls_corpus_reference_u16le.raw").as_slice()
    );
    Ok(())
}

#[test]
fn a_collision_on_the_third_uid_leaves_the_first_two_unclaimed() -> TestResult {
    // The preflight covers all three before any registration, so a collision on
    // the last leaves the earlier two `KnownUnavailable`. Deviation D-19.
    let mut registry = Registry::new();
    registry.register(std::sync::Arc::new(Htj2kDecoder::general()))?;
    assert_eq!(
        register_htj2k_decoders(&mut registry),
        Err(RegistryError::AlreadyRegistered { uid: HTJ2K })
    );
    assert_eq!(
        registry.capability(HTJ2K_LOSSLESS),
        Capability::KnownUnavailable
    );
    assert_eq!(
        registry.capability(HTJ2K_LOSSLESS_RPCL),
        Capability::KnownUnavailable
    );
    Ok(())
}

#[track_caller]
fn assert_atomic_error(source: &[u8], desc: &FrameDesc, expected: CodecError) {
    let mut out = vec![0xa5; desc.output_len()];
    assert_eq!(
        Htj2kDecoder::lossless().decode(source, desc, &mut out),
        Err(expected)
    );
    assert!(out.iter().all(|byte| *byte == 0xa5));
}

/// The corpus rows are 64 by 96 unsigned 16-bit, from `scripts/corpus_synth.py`.
fn corpus_desc() -> Result<FrameDesc, Box<dyn Error>> {
    frame(64, 96, 16, 16, PixelRepresentation::Unsigned, "MONOCHROME2")
}

fn frame(
    rows: u16,
    columns: u16,
    bits_allocated: u16,
    bits_stored: u16,
    pixel_representation: PixelRepresentation,
    photometric_interpretation: &str,
) -> Result<FrameDesc, Box<dyn Error>> {
    frame_with_vr(
        rows,
        columns,
        bits_allocated,
        bits_stored,
        pixel_representation,
        photometric_interpretation,
        PixelDataVr::Ob,
    )
}

fn frame_with_vr(
    rows: u16,
    columns: u16,
    bits_allocated: u16,
    bits_stored: u16,
    pixel_representation: PixelRepresentation,
    photometric_interpretation: &str,
    pixel_data_vr: PixelDataVr,
) -> Result<FrameDesc, Box<dyn Error>> {
    Ok(FrameDesc::new(FrameDescInput {
        rows,
        columns,
        samples_per_pixel: 1,
        bits_allocated,
        bits_stored,
        high_bit: bits_stored - 1,
        pixel_representation,
        photometric_interpretation: photometric_interpretation.to_owned(),
        pixel_data_vr,
    })?)
}

/// Lowercase hexadecimal SHA-256, so a digest recorded in a spike answer file
/// can be compared with a decode without a second spelling of the same bytes.
fn hex_sha256(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// Every differing sample as `(index, actual, expected)`, in index order.
fn differences(actual: &[u8], expected: &[u8]) -> Result<Vec<Difference>, Box<dyn Error>> {
    if actual.len() != expected.len() {
        return Err("length mismatch".into());
    }
    let mut found = Vec::new();
    for (index, (got, want)) in actual
        .chunks_exact(2)
        .zip(expected.chunks_exact(2))
        .enumerate()
    {
        let got = u16::from_le_bytes(<[u8; 2]>::try_from(got)?);
        let want = u16::from_le_bytes(<[u8; 2]>::try_from(want)?);
        if got != want {
            found.push((index, got, want));
        }
    }
    Ok(found)
}
