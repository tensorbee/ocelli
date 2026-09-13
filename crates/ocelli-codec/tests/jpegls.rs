//! JPEG-LS conformance and stored-domain fixtures for F-028.
//!
//! The fixtures are built by `tests/fixtures/generate_jpegls.py`, which records
//! where each one came from and which anchors are independent of CharLS.

use std::error::Error;

use ocelli_codec::{
    Capability, CodecError, DecodePhotometricInterpretation, DecodeSampleLayout, Decoder,
    FrameDesc, FrameDescInput, JpegLsDecoder, PixelDataVr, PixelRepresentation, Registry,
    RegistryError, register_jpegls_decoders,
};

type TestResult = Result<(), Box<dyn Error>>;

const JPEGLS_LOSSLESS: &str = "1.2.840.10008.1.2.4.80";
const JPEGLS_NEAR_LOSSLESS: &str = "1.2.840.10008.1.2.4.81";

/// ISO/IEC 14495-1 NEAR for the near-lossless corpus row and the 12-bit ramp,
/// matching `scripts/corpus_synth.py`'s `JPEG_LS_NEAR`.
const NEAR: i32 = 3;

#[test]
fn manifest_backed_lossless_reproduces_the_uncompressed_reference_exactly() -> TestResult {
    // `corpus/manifest.tsv` row 45. The syntax is lossless, so the decoded
    // frame must equal `syntax/explicit_vr_le.dcm`'s Pixel Data, which is the
    // same synthetic ramp uncompressed and comes from scripts/corpus_synth.py
    // rather than from any codec. This is the encoder-independent anchor and it
    // does not depend on CharLS at all.
    let reference = include_bytes!("fixtures/jpegls_corpus_reference_u16le.raw");
    let desc = corpus_desc()?;
    let mut out = vec![0xa5; desc.output_len()];
    JpegLsDecoder::lossless().decode(
        include_bytes!("fixtures/jpegls_corpus_lossless.jls"),
        &desc,
        &mut out,
    )?;
    assert_eq!(out.as_slice(), reference.as_slice());
    Ok(())
}

#[test]
fn manifest_backed_near_lossless_stays_inside_the_iso_14495_1_bound() -> TestResult {
    // `corpus/manifest.tsv` row 46, encoded at NEAR 3. ISO/IEC 14495-1
    // guarantees the maximum absolute error is at most NEAR, and that guarantee
    // is the standard rather than an implementation, so it is an independent
    // anchor in a way agreement with another CharLS reading would not be.
    //
    // `.81` is lossy by design, so "the output differs from the source" is
    // expected here and is not evidence of a defect. The same statement about
    // `.80` would be.
    let reference = include_bytes!("fixtures/jpegls_corpus_reference_u16le.raw");
    let desc = corpus_desc()?;
    let mut out = vec![0xa5; desc.output_len()];
    JpegLsDecoder::near_lossless().decode(
        include_bytes!("fixtures/jpegls_corpus_near_lossless.jls"),
        &desc,
        &mut out,
    )?;

    let (max_error, over_bound) = mono16_error(&out, reference)?;
    assert!(
        over_bound == 0,
        "{over_bound} sample(s) exceed the declared NEAR bound of {NEAR}"
    );
    // The bound holding at its limit rather than comfortably inside it, which
    // is what gate A2 measured. A decode that came back bit-identical to the
    // reference would mean the near-lossless path had not run.
    assert_eq!(max_error, NEAR);
    Ok(())
}

#[test]
fn every_unsigned_sixteen_bit_stored_value_round_trips_exactly() -> TestResult {
    // `docs/sprints/CURRENT_SPRINT.md` requires the stored-domain round trip
    // proven over the full range rather than over a sample, because the
    // dependency returns `Vec<f32>` and an `f32` that carries a 16-bit value
    // exactly for everything except the top of the range is the quietly-wrong
    // pixel this repository exists to catch.
    //
    // This frame carries all 65,536 values exactly once. Every `f32` in
    // `0 ..= 65535` is exact, since 16 bits is well inside the 24 significant
    // bits an `f32` represents without loss, so this fixture asserts the
    // property rather than hoping for it.
    let reference = include_bytes!("fixtures/jpegls_full_range_u16.raw");
    let desc = frame(
        256,
        256,
        16,
        16,
        PixelRepresentation::Unsigned,
        "MONOCHROME2",
    )?;
    assert_eq!(desc.output_len(), 65_536 * 2);
    let mut out = vec![0xa5; desc.output_len()];
    JpegLsDecoder::lossless().decode(
        include_bytes!("fixtures/jpegls_full_range_u16.jls"),
        &desc,
        &mut out,
    )?;
    assert_eq!(out.as_slice(), reference.as_slice());

    // Every distinct value is present, so "exact over the full range" is a
    // statement about the whole domain rather than about 65,536 samples that
    // might repeat.
    let mut seen = vec![false; 65_536];
    for pair in out.chunks_exact(2) {
        let value = usize::from(u16::from_le_bytes(<[u8; 2]>::try_from(pair)?));
        let slot = seen
            .get_mut(value)
            .ok_or("sample outside the 16-bit domain")?;
        assert!(!*slot, "value {value} appeared twice");
        *slot = true;
    }
    assert!(seen.iter().all(|present| *present));
    Ok(())
}

#[test]
fn twelve_bit_stored_values_round_trip_in_a_sixteen_bit_container() -> TestResult {
    // Bits Stored 12 in a 16-bit container is the normal case for CT and CR,
    // not the exception. All 4,096 values, each exactly once.
    let reference = include_bytes!("fixtures/jpegls_ramp_u12.raw");
    let desc = frame(64, 64, 16, 12, PixelRepresentation::Unsigned, "MONOCHROME2")?;
    let mut out = vec![0xa5; desc.output_len()];
    JpegLsDecoder::lossless().decode(
        include_bytes!("fixtures/jpegls_ramp_u12.jls"),
        &desc,
        &mut out,
    )?;
    assert_eq!(out.as_slice(), reference.as_slice());
    Ok(())
}

#[test]
fn the_modality_rescale_is_pinned_to_the_identity() -> TestResult {
    // The dependency's `PixelLayout` carries `rescale_slope` and
    // `rescale_intercept`, so the codec can apply the modality LUT. HLD section
    // 18 requires that arithmetic to exist exactly once, in `ocelli-pixel`.
    //
    // The adapter pins slope 1 and intercept 0. If either moved, the decoded
    // bytes would no longer equal the ramp: with slope 2 and intercept 5 the
    // first sample would be 5 rather than 0, and the last would be far outside
    // the 12-bit stored range and refused. This fixture is what makes that
    // checkable from outside the crate.
    let reference = include_bytes!("fixtures/jpegls_ramp_u12.raw");
    let desc = frame(64, 64, 16, 12, PixelRepresentation::Unsigned, "MONOCHROME2")?;
    let mut out = vec![0xa5; desc.output_len()];
    JpegLsDecoder::lossless().decode(
        include_bytes!("fixtures/jpegls_ramp_u12.jls"),
        &desc,
        &mut out,
    )?;

    // Sample n is exactly n, which is only true under the identity rescale.
    for (index, pair) in out.chunks_exact(2).enumerate() {
        let value = u16::from_le_bytes(<[u8; 2]>::try_from(pair)?);
        assert_eq!(usize::from(value), index);
    }
    assert_eq!(out.as_slice(), reference.as_slice());
    Ok(())
}

#[test]
fn a_sample_outside_the_stored_range_is_refused_rather_than_truncated() -> TestResult {
    // The 16-bit full-range frame decoded against a descriptor claiming Bits
    // Stored 12. Every sample above 4,095 is outside the stored range the
    // descriptor declares, and the boundary refuses rather than masking.
    let desc = frame(
        256,
        256,
        16,
        12,
        PixelRepresentation::Unsigned,
        "MONOCHROME2",
    )?;
    let mut out = vec![0xa5; desc.output_len()];
    let error = JpegLsDecoder::lossless().decode(
        include_bytes!("fixtures/jpegls_full_range_u16.jls"),
        &desc,
        &mut out,
    );
    // The precision check catches it first, which is the stronger statement:
    // the codestream declares 16 and the descriptor declares 12, so the frame
    // does not match its DICOM description.
    assert_eq!(error, Err(CodecError::FrameMismatch));
    assert!(out.iter().all(|byte| *byte == 0xa5));
    Ok(())
}

#[test]
fn the_two_uids_are_not_interchangeable() -> TestResult {
    // PS3.5 A.4.3 and A.4.4. `.80` is lossless, which ISO/IEC 14495-1 defines
    // as NEAR zero, and `.81` is near-lossless, which is NEAR above zero.
    // Without this check either decoder would accept either codestream and the
    // lossless claim would be unfalsifiable.
    let desc = corpus_desc()?;
    let mut out = vec![0xa5; desc.output_len()];

    assert_eq!(
        JpegLsDecoder::lossless().decode(
            include_bytes!("fixtures/jpegls_corpus_near_lossless.jls"),
            &desc,
            &mut out,
        ),
        Err(CodecError::FrameMismatch)
    );
    assert!(out.iter().all(|byte| *byte == 0xa5));

    assert_eq!(
        JpegLsDecoder::near_lossless().decode(
            include_bytes!("fixtures/jpegls_corpus_lossless.jls"),
            &desc,
            &mut out,
        ),
        Err(CodecError::FrameMismatch)
    );
    assert!(out.iter().all(|byte| *byte == 0xa5));

    // And each accepts its own, so the check refuses the wrong pair rather
    // than everything.
    let twelve_bit = frame(64, 64, 16, 12, PixelRepresentation::Unsigned, "MONOCHROME2")?;
    let mut twelve_out = vec![0xa5; twelve_bit.output_len()];
    JpegLsDecoder::near_lossless().decode(
        include_bytes!("fixtures/jpegls_ramp_u12_near3.jls"),
        &twelve_bit,
        &mut twelve_out,
    )?;
    assert!(twelve_out.iter().any(|byte| *byte != 0xa5));
    Ok(())
}

#[test]
fn multi_component_and_colour_descriptors_are_refused_before_the_dependency() -> TestResult {
    // A DICOM JPEG-LS frame can be RGB and neither route gate A2 measured
    // decodes one. The refusal is ours and typed, and it happens before the
    // dependency is called, so the capability statement is honest rather than
    // an opaque dependency string.
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
    let mut out = vec![0xa5; colour.output_len()];
    assert_eq!(
        JpegLsDecoder::lossless().decode(
            include_bytes!("fixtures/jpegls_corpus_lossless.jls"),
            &colour,
            &mut out,
        ),
        Err(CodecError::UnsupportedPixelFormat)
    );
    assert!(out.iter().all(|byte| *byte == 0xa5));

    // Other Word Pixel Data is refused too. PS3.5 A.4 encapsulates with OB.
    let word = frame_with_vr(
        64,
        96,
        16,
        16,
        PixelRepresentation::Unsigned,
        "MONOCHROME2",
        PixelDataVr::Ow,
    )?;
    let mut out = vec![0xa5; word.output_len()];
    assert_eq!(
        JpegLsDecoder::lossless().decode(
            include_bytes!("fixtures/jpegls_corpus_lossless.jls"),
            &word,
            &mut out,
        ),
        Err(CodecError::UnsupportedPixelFormat)
    );
    assert!(out.iter().all(|byte| *byte == 0xa5));
    Ok(())
}

#[test]
fn a_multi_component_codestream_is_refused_by_the_header_check() -> TestResult {
    // The descriptor check cannot reach this. A colour descriptor is refused
    // before the codestream is read, so the codestream-side condition needs a
    // genuinely multi-component frame to have anything to refuse.
    //
    // `fixtures/jpegls_rgb8_ilv1.jls` is a 16 by 16 8-bit RGB frame encoded
    // line-interleaved, so its SOF55 declares Nf = 3 and its SOS declares
    // ILV = 1. The descriptor below is single-sample monochrome and matches the
    // frame's dimensions, so every earlier check passes and the header check is
    // the one that fires.
    let desc = frame(16, 16, 8, 8, PixelRepresentation::Unsigned, "MONOCHROME2")?;
    let mut out = vec![0xa5; desc.output_len()];
    assert_eq!(
        JpegLsDecoder::lossless().decode(
            include_bytes!("fixtures/jpegls_rgb8_ilv1.jls"),
            &desc,
            &mut out,
        ),
        Err(CodecError::UnsupportedPixelFormat)
    );
    assert!(out.iter().all(|byte| *byte == 0xa5));
    Ok(())
}

#[test]
fn each_descriptor_condition_is_refused_on_its_own() -> TestResult {
    // `validate_descriptor` is a disjunction, so a test whose descriptor
    // violates two conditions at once cannot say which one fired. Each row here
    // violates exactly one.
    let source = include_bytes!("fixtures/jpegls_corpus_lossless.jls");

    // Samples per Pixel 3 with a monochrome interpretation. Malformed DICOM,
    // and the point is that this check does not depend on the photometric one.
    let three_samples = FrameDesc::new(FrameDescInput {
        rows: 64,
        columns: 96,
        samples_per_pixel: 3,
        bits_allocated: 16,
        bits_stored: 16,
        high_bit: 15,
        pixel_representation: PixelRepresentation::Unsigned,
        photometric_interpretation: "MONOCHROME2".to_owned(),
        pixel_data_vr: PixelDataVr::Ob,
    })?;
    let mut out = vec![0xa5; three_samples.output_len()];
    assert_eq!(
        JpegLsDecoder::lossless().decode(source, &three_samples, &mut out),
        Err(CodecError::UnsupportedPixelFormat)
    );
    assert!(out.iter().all(|byte| *byte == 0xa5));

    // A 32-bit container. PS3.5 A.4 JPEG-LS carries 8 or 16.
    let wide = frame(64, 96, 32, 32, PixelRepresentation::Unsigned, "MONOCHROME2")?;
    let mut out = vec![0xa5; wide.output_len()];
    assert_eq!(
        JpegLsDecoder::lossless().decode(source, &wide, &mut out),
        Err(CodecError::UnsupportedPixelFormat)
    );
    assert!(out.iter().all(|byte| *byte == 0xa5));

    // A colour photometric interpretation with one sample, so only the
    // photometric condition is violated.
    let palette = frame(
        64,
        96,
        16,
        16,
        PixelRepresentation::Unsigned,
        "PALETTE COLOR",
    )?;
    let mut out = vec![0xa5; palette.output_len()];
    assert_eq!(
        JpegLsDecoder::lossless().decode(source, &palette, &mut out),
        Err(CodecError::UnsupportedPixelFormat)
    );
    assert!(out.iter().all(|byte| *byte == 0xa5));
    Ok(())
}

#[test]
fn structural_and_descriptor_failures_preserve_caller_output() -> TestResult {
    let source = include_bytes!("fixtures/jpegls_corpus_lossless.jls");
    let desc = corpus_desc()?;

    // Not a JPEG-LS codestream at all.
    assert_atomic_error(source.get(..3), &desc, CodecError::InvalidCodestream);
    // Truncated before EOI.
    assert_atomic_error(
        source.get(..source.len() - 8),
        &desc,
        CodecError::InvalidCodestream,
    );

    // Dimensions disagree with the descriptor.
    let wrong = frame(32, 96, 16, 16, PixelRepresentation::Unsigned, "MONOCHROME2")?;
    let mut out = vec![0xa5; wrong.output_len()];
    assert_eq!(
        JpegLsDecoder::lossless().decode(source, &wrong, &mut out),
        Err(CodecError::FrameMismatch)
    );
    assert!(out.iter().all(|byte| *byte == 0xa5));

    // The caller's buffer is the wrong length.
    let mut short = vec![0xa5; desc.output_len() - 2];
    assert_eq!(
        JpegLsDecoder::lossless().decode(source, &desc, &mut short),
        Err(CodecError::OutputLength {
            expected: desc.output_len(),
            actual: desc.output_len() - 2,
        })
    );
    assert!(short.iter().all(|byte| *byte == 0xa5));
    Ok(())
}

#[test]
fn trailing_data_after_the_end_marker_is_distinguished_from_padding() -> TestResult {
    let source = include_bytes!("fixtures/jpegls_corpus_lossless.jls");
    let desc = corpus_desc()?;

    // PS3.5 A.4 pads an odd fragment to an even length with one byte. That is
    // not trailing data and must still decode.
    let mut padded = source.to_vec();
    padded.push(0);
    let mut out = vec![0xa5; desc.output_len()];
    JpegLsDecoder::lossless().decode(&padded, &desc, &mut out)?;
    assert_eq!(
        out.as_slice(),
        include_bytes!("fixtures/jpegls_corpus_reference_u16le.raw").as_slice()
    );

    // Two extra bytes is a complete image followed by something else.
    let mut trailing = source.to_vec();
    trailing.extend_from_slice(&[0x12, 0x34]);
    assert_atomic_error(Some(trailing.as_slice()), &desc, CodecError::TrailingData);
    Ok(())
}

#[test]
fn a_stray_end_marker_inside_the_headers_is_refused_rather_than_walked_past() -> TestResult {
    // A hand-built malformed codestream. SOI, then a stray EOI whose two
    // length bytes claim a four-byte segment, then a SOF55 and SOS that are
    // individually well formed and agree with the descriptor, then EOI.
    //
    // Without the guard that refuses SOI and EOI where a marker segment is
    // expected, the walk skips the stray segment, reads the following headers
    // as if they were the frame's own, accepts them, and hands the dependency
    // bytes that are not a scan. The failure then arrives as a decoder failure
    // rather than as a structural one, which is a worse answer about a
    // structurally invalid stream.
    let mut source: Vec<u8> = vec![0xff, 0xd8];
    source.extend_from_slice(&[0xff, 0xd9, 0x00, 0x04, 0xaa, 0xbb]);
    // SOF55: P = 16, Y = 64, X = 96, Nf = 1, one component specifier.
    source.extend_from_slice(&[
        0xff, 0xf7, 0x00, 0x0b, 0x10, 0x00, 0x40, 0x00, 0x60, 0x01, 0x01, 0x11, 0x00,
    ]);
    // SOS: Ns = 1, one component specifier, NEAR = 0, ILV = 0, Al/Ah = 0.
    source.extend_from_slice(&[0xff, 0xda, 0x00, 0x08, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00]);
    source.extend_from_slice(&[0xff, 0xd9]);

    let desc = corpus_desc()?;
    let mut out = vec![0xa5; desc.output_len()];
    assert_eq!(
        JpegLsDecoder::lossless().decode(&source, &desc, &mut out),
        Err(CodecError::InvalidCodestream)
    );
    assert!(out.iter().all(|byte| *byte == 0xa5));
    Ok(())
}

#[test]
fn registration_is_atomic_and_reports_preserved_monochrome_output() -> TestResult {
    let mut registry = Registry::new();
    assert_eq!(
        registry.capability(JPEGLS_LOSSLESS),
        Capability::KnownUnavailable
    );
    assert_eq!(
        registry.capability(JPEGLS_NEAR_LOSSLESS),
        Capability::KnownUnavailable
    );

    register_jpegls_decoders(&mut registry)?;
    assert_eq!(registry.capability(JPEGLS_LOSSLESS), Capability::Available);
    assert_eq!(
        registry.capability(JPEGLS_NEAR_LOSSLESS),
        Capability::Available
    );

    // A second registration refuses and changes nothing.
    assert_eq!(
        register_jpegls_decoders(&mut registry),
        Err(RegistryError::AlreadyRegistered {
            uid: JPEGLS_LOSSLESS
        })
    );

    // The adapter performs no colour transform and no sample reordering, so
    // both queries report the frame's own description.
    let desc = corpus_desc()?;
    assert_eq!(
        registry.decode_photometric_interpretation(JPEGLS_LOSSLESS, &desc)?,
        DecodePhotometricInterpretation::Preserved
    );
    assert_eq!(
        registry.decode_sample_layout(JPEGLS_LOSSLESS, &desc)?,
        DecodeSampleLayout::Preserved
    );

    // Dispatch by exact UID reaches the same bytes as the direct call.
    let mut out = vec![0xa5; desc.output_len()];
    registry.decode(
        JPEGLS_LOSSLESS,
        include_bytes!("fixtures/jpegls_corpus_lossless.jls"),
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
fn a_partial_registration_cannot_leave_one_uid_claimed() -> TestResult {
    // The preflight runs over both UIDs before either is registered, so a
    // collision on the second leaves the first unclaimed. Deviation D-19.
    let mut registry = Registry::new();
    registry.register(std::sync::Arc::new(JpegLsDecoder::near_lossless()))?;
    assert_eq!(
        register_jpegls_decoders(&mut registry),
        Err(RegistryError::AlreadyRegistered {
            uid: JPEGLS_NEAR_LOSSLESS
        })
    );
    assert_eq!(
        registry.capability(JPEGLS_LOSSLESS),
        Capability::KnownUnavailable
    );
    Ok(())
}

#[track_caller]
fn assert_atomic_error(source: Option<&[u8]>, desc: &FrameDesc, expected: CodecError) {
    assert!(source.is_some());
    let Some(source) = source else { return };
    let mut out = vec![0xa5; desc.output_len()];
    assert_eq!(
        JpegLsDecoder::lossless().decode(source, desc, &mut out),
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

/// The maximum absolute difference and the count over the declared NEAR bound.
fn mono16_error(actual: &[u8], expected: &[u8]) -> Result<(i32, usize), Box<dyn Error>> {
    if actual.len() != expected.len() {
        return Err("length mismatch".into());
    }
    let mut max_error = 0_i32;
    let mut over_bound = 0_usize;
    for (got, want) in actual.chunks_exact(2).zip(expected.chunks_exact(2)) {
        let got = u16::from_le_bytes(<[u8; 2]>::try_from(got)?);
        let want = u16::from_le_bytes(<[u8; 2]>::try_from(want)?);
        let error = i32::from(got.abs_diff(want));
        if error > max_error {
            max_error = error;
        }
        if error > NEAR {
            over_bound += 1;
        }
    }
    Ok((max_error, over_bound))
}
