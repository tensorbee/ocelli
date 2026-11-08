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
fn an_rgb_descriptor_does_not_decode_the_monochrome_codestream() -> TestResult {
    // The lossless monochrome fixture is 16-bit and single-component. An 8-bit
    // RGB descriptor names a different frame, so the answer is a mismatch and
    // not a decoded image. The check still happens before the vendored decoder.
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
        Err(CodecError::FrameMismatch)
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

/// The manifest-backed multi-component row, which is the one gate A2 asked for.
///
/// `docs/spikes/A2-jpeg-ls.md` measured `Nf != 1` as "REFUSED by the crate, and
/// NOT MEASURED here" and recorded that the corpus could not close the gap
/// because no such row existed. F-030 added `syntax/jpegls_lossless_rgb8.dcm`,
/// so the refusal now has a real encoder's three-component output behind it.
///
/// **This is not a duplicate of the test above.** That one is a repository
/// local 16 by 16 fixture encoded LINE-interleaved, `ILV = 1`, and every
/// descriptor refuses it. This one is a corpus row encoded SAMPLE-interleaved,
/// `ILV = 2`. An RGB descriptor decodes that row. A monochrome descriptor does
/// not, so a test at one interleave cannot speak for the other, and only one
/// of the two is manifest backed.
///
/// The descriptor is single-sample monochrome at the frame's real dimensions.
/// The codestream is three-component sample-interleaved RGB. A monochrome
/// description of that codestream is still refused, and nothing is written.
/// The RGB descriptor is a different test, and it must reproduce the ramp.
#[test]
fn the_manifest_backed_sample_interleaved_colour_row_is_refused() -> TestResult {
    let desc = frame(64, 96, 8, 8, PixelRepresentation::Unsigned, "MONOCHROME2")?;
    let mut out = vec![0xa5; desc.output_len()];
    assert_eq!(
        JpegLsDecoder::lossless().decode(
            include_bytes!("fixtures/jpegls_corpus_rgb8.jls"),
            &desc,
            &mut out,
        ),
        Err(CodecError::UnsupportedPixelFormat)
    );
    // No partial output. A refusal that had already written some of the frame
    // would leave a caller holding half a decoded image and an error.
    assert!(out.iter().all(|byte| *byte == 0xa5));
    Ok(())
}

#[test]
fn sample_interleaved_rgb_reproduces_the_uncompressed_ramp() -> TestResult {
    // `syntax/jpegls_lossless_rgb8.dcm` is the RGB reference encoded losslessly,
    // so the decode must equal `scripts/corpus_synth.py`'s `rgb_ramp` at
    // `SYNTAX_ROWS` by `SYNTAX_COLS`. That ramp is red along the columns, green
    // along the rows, and blue the integer mean of the two. It comes from no
    // codec. `.81` is the wrong UID for a NEAR of zero.
    let desc = FrameDesc::new(FrameDescInput {
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
    assert_eq!(
        JpegLsDecoder::lossless().decode_photometric_interpretation(&desc),
        DecodePhotometricInterpretation::Rgb
    );
    assert_eq!(
        JpegLsDecoder::lossless().decode_sample_layout(&desc),
        DecodeSampleLayout::Interleaved
    );
    assert_eq!(
        JpegLsDecoder::near_lossless().decode_photometric_interpretation(&desc),
        DecodePhotometricInterpretation::Preserved
    );

    let source = include_bytes!("fixtures/jpegls_corpus_rgb8.jls");
    let mut out = vec![0xa5; desc.output_len()];
    assert_eq!(
        JpegLsDecoder::near_lossless().decode(source, &desc, &mut out),
        Err(CodecError::FrameMismatch)
    );
    assert!(out.iter().all(|byte| *byte == 0xa5));

    JpegLsDecoder::lossless().decode(source, &desc, &mut out)?;
    let expected = rgb_ramp(64, 96);
    assert_eq!(out.len(), expected.len());
    let mut mismatches = 0usize;
    let mut first = None;
    for (index, (got, want)) in out.iter().zip(expected.iter()).enumerate() {
        if got != want {
            mismatches += 1;
            if first.is_none() {
                first = Some(index);
            }
        }
    }
    assert_eq!(mismatches, 0, "first sample index {first:?}");
    Ok(())
}

fn decode_constructed_rgb(
    rows: u16,
    columns: u16,
    source: &[u8],
    pixels: &[[u8; 3]],
) -> TestResult {
    assert_eq!(pixels.len(), usize::from(rows) * usize::from(columns));
    let desc = FrameDesc::new(FrameDescInput {
        rows,
        columns,
        samples_per_pixel: 3,
        bits_allocated: 8,
        bits_stored: 8,
        high_bit: 7,
        pixel_representation: PixelRepresentation::Unsigned,
        photometric_interpretation: "RGB".to_owned(),
        pixel_data_vr: PixelDataVr::Ob,
    })?;
    let mut expected = Vec::with_capacity(pixels.len() * 3);
    for pixel in pixels {
        expected.extend_from_slice(pixel);
    }
    let mut out = vec![0xa5; desc.output_len()];
    JpegLsDecoder::lossless().decode(source, &desc, &mut out)?;
    assert_eq!(out, expected);
    Ok(())
}

fn solid(count: usize, fill: [u8; 3]) -> Vec<[u8; 3]> {
    vec![fill; count]
}

#[test]
fn a_flat_rgb_run_interruption_keeps_the_constructed_sample() -> TestResult {
    // The field is flat, so the scan enters run mode, and one sample is the
    // interruption. A second frame puts that interruption on a row whose value
    // differs from the row above. Each codestream is a lossless 8-bit ILV=2
    // encoding. The expected pixels are the arrays built here.
    let columns = 8usize;
    let mut flat = solid(4 * columns, [10, 20, 30]);
    let slot = flat
        .get_mut(columns + 3)
        .ok_or("interruption outside the frame")?;
    *slot = [10, 20, 29];
    let lowered: &[u8] = &[
        0xff, 0xd8, 0xff, 0xf7, 0x00, 0x11, 0x08, 0x00, 0x04, 0x00, 0x08, 0x03, 0x01, 0x11, 0x00,
        0x02, 0x11, 0x00, 0x03, 0x11, 0x00, 0xff, 0xda, 0x00, 0x0c, 0x03, 0x01, 0x00, 0x02, 0x00,
        0x03, 0x00, 0x00, 0x02, 0x00, 0x02, 0x02, 0x07, 0x25, 0x2d, 0xdf, 0xff, 0x14, 0xd0, 0x84,
        0x65, 0xbf, 0xe0, 0xff, 0xd9,
    ];
    decode_constructed_rgb(4, 8, lowered, &flat)?;

    let mut banded = solid(columns, [10, 20, 30]);
    banded.extend(solid(columns, [40, 50, 60]));
    let slot = banded
        .get_mut(columns + 3)
        .ok_or("interruption outside the frame")?;
    *slot = [40, 50, 59];
    let interrupted: &[u8] = &[
        0xff, 0xd8, 0xff, 0xf7, 0x00, 0x11, 0x08, 0x00, 0x02, 0x00, 0x08, 0x03, 0x01, 0x11, 0x00,
        0x02, 0x11, 0x00, 0x03, 0x11, 0x00, 0xff, 0xda, 0x00, 0x0c, 0x03, 0x01, 0x00, 0x02, 0x00,
        0x03, 0x00, 0x00, 0x02, 0x00, 0x02, 0x02, 0x07, 0x25, 0x2d, 0xdf, 0xfe, 0x00, 0x02, 0x3a,
        0x00, 0x01, 0x3f, 0xce, 0x7f, 0xe0, 0xff, 0xd9,
    ];
    decode_constructed_rgb(2, 8, interrupted, &banded)?;
    Ok(())
}

#[test]
fn a_near_lossless_rgb_codestream_is_refused() -> TestResult {
    // The lossless fixture's NEAR byte is zero. Raising it to one makes a
    // legal `.81` header. That scan is still refused. The lossless thresholds
    // must not decode it, and the output buffer stays untouched.
    let desc = FrameDesc::new(FrameDescInput {
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
    let mut source = include_bytes!("fixtures/jpegls_corpus_rgb8.jls").to_vec();
    assert_eq!(source.get(32).copied(), Some(0));
    assert_eq!(source.get(33).copied(), Some(2));
    let Some(near) = source.get_mut(32) else {
        return Err("NEAR byte missing".into());
    };
    *near = 1;
    let mut out = vec![0xa5; desc.output_len()];
    assert_eq!(
        JpegLsDecoder::near_lossless().decode(&source, &desc, &mut out),
        Err(CodecError::UnsupportedPixelFormat)
    );
    assert!(out.iter().all(|byte| *byte == 0xa5));
    assert_eq!(
        JpegLsDecoder::lossless().decode(&source, &desc, &mut out),
        Err(CodecError::FrameMismatch)
    );
    assert!(out.iter().all(|byte| *byte == 0xa5));
    Ok(())
}

/// `scripts/corpus_synth.py` `rgb_ramp` for the syntax-sized colour reference.
fn rgb_ramp(rows: u32, columns: u32) -> Vec<u8> {
    let mut out = Vec::new();
    let row_span = rows.saturating_sub(1).max(1);
    let column_span = columns.saturating_sub(1).max(1);
    for row in 0..rows {
        let green = (row * 255) / row_span;
        for column in 0..columns {
            let red = (column * 255) / column_span;
            let blue = (red + green) / 2;
            out.push(u8::try_from(red).unwrap_or(255));
            out.push(u8::try_from(green).unwrap_or(255));
            out.push(u8::try_from(blue).unwrap_or(255));
        }
    }
    out
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

    // F-243's refusals, each through its own descriptor: a scan that runs
    // out, two whole unread bytes before EOI, a run miss past the line, and an
    // LSE preset the decoder does not apply.
    let full_range = include_bytes!("fixtures/jpegls_full_range_u16.jls");
    let start = scan_start(full_range)?;
    let mut truncated = full_range
        .get(..start + (full_range.len() - 2 - start) / 2)
        .ok_or("scan outside the fixture")?
        .to_vec();
    truncated.extend_from_slice(&[0xff, 0xd9]);
    let full_desc = frame(
        256,
        256,
        16,
        16,
        PixelRepresentation::Unsigned,
        "MONOCHROME2",
    )?;
    assert_atomic_error(
        Some(truncated.as_slice()),
        &full_desc,
        CodecError::DecoderFailure,
    );
    let reported_desc = frame(32, 32, 16, 16, PixelRepresentation::Unsigned, "MONOCHROME2")?;
    let unread = insert_before_eoi(
        include_bytes!("fixtures/jpegls_two_band_u16_32x32.jls"),
        &[0x00, 0x00],
    );
    assert_atomic_error(
        Some(unread.as_slice()),
        &reported_desc,
        CodecError::DecoderFailure,
    );
    let line = frame(1, 13, 8, 8, PixelRepresentation::Unsigned, "MONOCHROME2")?;
    assert_atomic_error(
        Some(include_bytes!("fixtures/jpegls_run_miss_past_line_u8.jls").as_slice()),
        &line,
        CodecError::DecoderFailure,
    );
    let preset = with_preset(65535, 18, 67, 276, 32)?;
    assert_atomic_error(
        Some(preset.as_slice()),
        &desc,
        CodecError::UnsupportedPreset(ocelli_codec::JpegLsPreset::Reset),
    );
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

// ---------------------------------------------------------------------------
// F-243, run mode per ITU-T T.87 F.1 step 18, the end of the scan, and LSE presets.
//
// Every expected frame here is the two-band formula of the design plan,
// `s(r, c) = A if c < cols / 2 else B`, built by `two_band` below and by
// `generate_jpegls.py`, never by a codec. The codestreams are CharLS encodes,
// and each test states what the vendored decoder returned before F-243.
// ---------------------------------------------------------------------------

/// The reported file's two stored values.
const BAND_LOW: u16 = 224;
const BAND_HIGH: u16 = 1064;

/// `s(r, c) = low if c < cols / 2 else high`, little-endian 16-bit words.
fn two_band(rows: usize, cols: usize, low: u16, high: u16) -> Vec<u8> {
    let mut out = Vec::with_capacity(rows * cols * 2);
    for _ in 0..rows {
        for column in 0..cols {
            let value = if column < cols / 2 { low } else { high };
            out.extend_from_slice(&value.to_le_bytes());
        }
    }
    out
}

/// The number of 16-bit samples that differ, and the first one's index.
fn mono16_mismatches(actual: &[u8], expected: &[u8]) -> (usize, Option<usize>) {
    let mut count = 0;
    let mut first = None;
    for (index, (got, want)) in actual
        .chunks_exact(2)
        .zip(expected.chunks_exact(2))
        .enumerate()
    {
        if got != want {
            count += 1;
            first.get_or_insert(index);
        }
    }
    (count, first)
}

#[test]
fn two_band_eight_by_four_decodes_exactly() -> TestResult {
    // T.87 F.1 step 18.b. The smallest two-band frame measured to reach a run that
    // ends its line with fewer than 2^J[RUNindex] samples left, whose one bit
    // must not raise RUNindex. Before F-243 the vendored loop raised it
    // anyway, and 2 of the 32 samples came back wrong with `Ok`, the first at
    // row 7 column 2.
    let expected = two_band(8, 4, BAND_LOW, BAND_HIGH);
    assert_eq!(
        include_bytes!("fixtures/jpegls_two_band_u16_8x4.raw").as_slice(),
        expected.as_slice()
    );
    let desc = frame(8, 4, 16, 16, PixelRepresentation::Unsigned, "MONOCHROME2")?;
    let mut out = vec![0xa5; desc.output_len()];
    JpegLsDecoder::lossless().decode(
        include_bytes!("fixtures/jpegls_two_band_u16_8x4.jls"),
        &desc,
        &mut out,
    )?;
    assert_eq!(mono16_mismatches(&out, &expected), (0, None));
    Ok(())
}

#[test]
fn two_band_thirty_two_reproduces_the_reported_frame() -> TestResult {
    // The reported Enhanced CT frame's shape and values, P 16, NEAR 0, with
    // CharLS's default LSE. Before F-243, 916 of 1,024 samples came back
    // wrong, the first at row 2 column 16, and the decode returned `Ok`.
    let expected = two_band(32, 32, BAND_LOW, BAND_HIGH);
    assert_eq!(
        include_bytes!("fixtures/jpegls_two_band_u16_32x32.raw").as_slice(),
        expected.as_slice()
    );
    let desc = frame(32, 32, 16, 16, PixelRepresentation::Unsigned, "MONOCHROME2")?;
    let mut out = vec![0xa5; desc.output_len()];
    JpegLsDecoder::lossless().decode(
        include_bytes!("fixtures/jpegls_two_band_u16_32x32.jls"),
        &desc,
        &mut out,
    )?;
    assert_eq!(mono16_mismatches(&out, &expected), (0, None));
    Ok(())
}

#[test]
fn two_band_near_three_is_inside_the_bound_and_equals_charls() -> TestResult {
    // T.87 A.4.4: every reconstructed sample is within NEAR of its source.
    // That is the specification's anchor. CharLS 2.4.4's own decode of the
    // same stream is the cornerstone3D parity reference, and it is exact.
    // On this content CharLS reconstructs the formula exactly, so the
    // maximum error is 0, recorded rather than assumed.
    let formula = two_band(32, 32, BAND_LOW, BAND_HIGH);
    let charls = include_bytes!("fixtures/jpegls_two_band_u16_32x32_near3_charls.raw");
    let desc = frame(32, 32, 16, 16, PixelRepresentation::Unsigned, "MONOCHROME2")?;
    let mut out = vec![0xa5; desc.output_len()];
    JpegLsDecoder::near_lossless().decode(
        include_bytes!("fixtures/jpegls_two_band_u16_32x32_near3.jls"),
        &desc,
        &mut out,
    )?;
    let (max_error, over_bound) = mono16_error(&out, &formula)?;
    assert_eq!(over_bound, 0, "samples outside NEAR {NEAR}");
    assert_eq!(max_error, 0);
    assert_eq!(mono16_mismatches(&out, charls), (0, None));
    Ok(())
}

#[test]
fn twelve_bit_two_band_without_lse_decodes_exactly() -> TestResult {
    // The second reviewer's frame: P 12 in a 16-bit container, NEAR 0, and no
    // LSE marker at all, so the defect is the run-mode rule and not preset
    // handling. Before F-243, 915 of 1,024 samples were wrong from sample 80.
    let codestream = include_bytes!("fixtures/jpegls_two_band_u12_32x32_no_lse.jls");
    assert!(
        !codestream.windows(2).any(|pair| pair == [0xff, 0xf8]),
        "the frame must carry no LSE"
    );
    let expected = two_band(32, 32, BAND_LOW, BAND_HIGH);
    assert_eq!(
        include_bytes!("fixtures/jpegls_two_band_u12_32x32_no_lse.raw").as_slice(),
        expected.as_slice()
    );
    let desc = frame(32, 32, 16, 12, PixelRepresentation::Unsigned, "MONOCHROME2")?;
    let mut out = vec![0xa5; desc.output_len()];
    JpegLsDecoder::lossless().decode(codestream, &desc, &mut out)?;
    assert_eq!(mono16_mismatches(&out, &expected), (0, None));
    Ok(())
}

/// Offset of the first entropy-coded byte, just past the SOS segment.
fn scan_start(codestream: &[u8]) -> Result<usize, Box<dyn Error>> {
    let mut offset = 2usize;
    loop {
        let Some(&[prefix, code, high, low]) = codestream.get(offset..offset + 4) else {
            return Err("SOS not found".into());
        };
        let length = usize::from(u16::from_be_bytes([high, low]));
        if [prefix, code] == [0xff, 0xda] {
            return Ok(offset + 2 + length);
        }
        offset += 2 + length;
    }
}

/// The codestream with `inserted` placed just before its final EOI.
fn insert_before_eoi(codestream: &[u8], inserted: &[u8]) -> Vec<u8> {
    let (body, eoi) = codestream.split_at(codestream.len().saturating_sub(2));
    assert_eq!(eoi, [0xff, 0xd9], "the codestream must end at EOI");
    let mut out = body.to_vec();
    out.extend_from_slice(inserted);
    out.extend_from_slice(eoi);
    out
}

#[test]
fn a_truncated_scan_is_refused() -> TestResult {
    // A scan that runs out of entropy-coded data before the last sample is
    // refused. The vendored bit reader returned zero bits past the end, and
    // before F-243 each of these three returned `Ok` with wrong samples.
    let source = include_bytes!("fixtures/jpegls_full_range_u16.jls");
    let desc = frame(
        256,
        256,
        16,
        16,
        PixelRepresentation::Unsigned,
        "MONOCHROME2",
    )?;
    let start = scan_start(source)?;
    let scan_length = source.len() - 2 - start;
    for kept_percent in [90usize, 50, 10] {
        let kept = scan_length * kept_percent / 100;
        let mut truncated = source
            .get(..start + kept)
            .ok_or("scan outside the fixture")?
            .to_vec();
        truncated.extend_from_slice(&[0xff, 0xd9]);
        let mut out = vec![0xa5; desc.output_len()];
        assert_eq!(
            JpegLsDecoder::lossless().decode(&truncated, &desc, &mut out),
            Err(CodecError::DecoderFailure),
            "{kept_percent} per cent of the scan kept"
        );
        assert!(out.iter().all(|byte| *byte == 0xa5));
    }

    // The positive control: the whole scan still decodes exactly.
    let mut out = vec![0xa5; desc.output_len()];
    JpegLsDecoder::lossless().decode(source, &desc, &mut out)?;
    assert_eq!(
        out.as_slice(),
        include_bytes!("fixtures/jpegls_full_range_u16.raw").as_slice()
    );
    Ok(())
}

/// Decode `hex` to bytes.
fn unhex(hex: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    (0..hex.len())
        .step_by(2)
        .map(|at| {
            let pair = hex.get(at..at + 2).ok_or("odd hex")?;
            Ok(u8::from_str_radix(pair, 16)?)
        })
        .collect()
}

/// One named mutation, exactly as `generate_jpegls.py`'s `mutate` applies it.
fn mutate(codestream: &[u8], mutation: &str) -> Result<Option<Vec<u8>>, Box<dyn Error>> {
    let start = scan_start(codestream)?;
    let scan = codestream.len() - 2 - start;
    let (body, eoi) = codestream.split_at(codestream.len() - 2);
    let (kind, argument) = mutation.split_once(':').unwrap_or((mutation, ""));
    let mutated = match kind {
        "none" => codestream.to_vec(),
        "drop" => {
            let count: usize = argument.parse()?;
            if count >= scan {
                return Ok(None);
            }
            [body.get(..body.len() - count).ok_or("drop")?, eoi].concat()
        }
        "ins" => [body, unhex(argument)?.as_slice(), eoi].concat(),
        "at" => {
            let (place, inserted) = argument.split_once(':').ok_or("at")?;
            let index = start
                + match place {
                    "start" => 0,
                    "third" => scan / 3,
                    "last" => scan - 1,
                    other => return Err(format!("unknown place {other}").into()),
                };
            let (head, tail) = codestream.split_at(index);
            [head, unhex(inserted)?.as_slice(), tail].concat()
        }
        "lead" => {
            let bit: u32 = argument.parse()?;
            let mut flipped = codestream.to_vec();
            let byte = flipped.get_mut(start).ok_or("lead")?;
            *byte ^= 1 << bit;
            flipped
        }
        "flip" => {
            let (back, bit) = argument.split_once(':').ok_or("flip")?;
            let (back, bit): (usize, u32) = (back.parse()?, bit.parse()?);
            if back > scan {
                return Ok(None);
            }
            let mut flipped = codestream.to_vec();
            let byte = flipped.get_mut(codestream.len() - 2 - back).ok_or("flip")?;
            *byte ^= 1 << bit;
            flipped
        }
        "cut" => {
            let percent: usize = argument.parse()?;
            [
                codestream
                    .get(..start + scan * percent / 100)
                    .ok_or("cut")?,
                eoi,
            ]
            .concat()
        }
        other => return Err(format!("unknown mutation {other}").into()),
    };
    Ok(Some(mutated))
}

/// Ocelli's verdict on a corpus case: `refuse`, or the digest of its samples
/// in the form the generator records CharLS's.
fn ocelli_digest(codestream: &[u8], base: &[u8]) -> Result<String, Box<dyn Error>> {
    use sha2::{Digest, Sha256};
    // SOF55 follows SOI directly in every base stream: P, Y, X.
    let Some(&[precision, y_high, y_low, x_high, x_low]) = base.get(6..11) else {
        return Err("no SOF55".into());
    };
    let start = scan_start(base)?;
    let near = *base.get(start - 3).ok_or("no NEAR")?;
    let desc = frame(
        u16::from_be_bytes([y_high, y_low]),
        u16::from_be_bytes([x_high, x_low]),
        if precision > 8 { 16 } else { 8 },
        u16::from(precision),
        PixelRepresentation::Unsigned,
        "MONOCHROME2",
    )?;
    let decoder = if near == 0 {
        JpegLsDecoder::lossless()
    } else {
        JpegLsDecoder::near_lossless()
    };
    let mut out = vec![0xa5; desc.output_len()];
    match decoder.decode(codestream, &desc, &mut out) {
        Ok(()) => Ok(Sha256::digest(&out)
            .iter()
            .take(8)
            .map(|byte| format!("{byte:02x}"))
            .collect()),
        Err(CodecError::DecoderFailure) => {
            assert!(out.iter().all(|byte| *byte == 0xa5));
            Ok("refuse".to_owned())
        }
        Err(other) => Err(format!("unexpected refusal {other:?}").into()),
    }
}

/// How each case of the generated corpus came out, by damage class.
///
/// The outcomes are `agree`, the same verdict and samples, `refused, CharLS
/// draws other samples`, a damaged stream CharLS opens to samples that are not
/// the stream's own, `refused, CharLS draws its own samples`, and `opened,
/// CharLS refuses`, where Ocelli draws the stream's own samples. A fifth,
/// Ocelli drawing samples that are neither CharLS's nor the stream's own, is a
/// wrong pixel and is never allowed. Every count was measured on this tree,
/// and the test fails if any moves, in either direction.
const CORPUS_OUTCOMES: &[(&str, &str, usize)] = &[
    ("a byte inserted inside the scan", "agree", 435),
    (
        "a byte inserted inside the scan",
        "refused, CharLS draws other samples",
        15,
    ),
    ("dropped final bytes", "agree", 1018),
    (
        "dropped final bytes",
        "refused, CharLS draws its own samples",
        8,
    ),
    (
        "dropped final bytes",
        "refused, CharLS draws other samples",
        7,
    ),
    ("early EOI", "agree", 435),
    ("early EOI", "refused, CharLS draws its own samples", 8),
    ("early EOI", "refused, CharLS draws other samples", 7),
    ("fill bytes before EOI", "agree", 300),
    ("flipped bit", "agree", 4713),
    ("flipped bit", "refused, CharLS draws other samples", 63),
    ("other bytes before EOI", "agree", 900),
    ("undamaged", "agree", 150),
    ("zero bytes before EOI", "agree", 164),
    (
        "zero bytes before EOI",
        "refused, CharLS draws its own samples",
        136,
    ),
    ("zero bytes inside the scan", "agree", 789),
    (
        "zero bytes inside the scan",
        "refused, CharLS draws its own samples",
        79,
    ),
    (
        "zero bytes inside the scan",
        "refused, CharLS draws other samples",
        32,
    ),
];

#[test]
fn ocelli_agrees_with_charls_on_the_generated_corpus() -> TestResult {
    // The lead's rulings D3 after review pass 1 and D1 and D2 after pass 2.
    // `generate_jpegls.py` encodes the streams with CharLS 2.4.4, applies each
    // mutation and records the damage class, CharLS's verdict and its sample
    // digest. Every undamaged stream that CharLS opens must decode to CharLS's
    // samples, no case may decode to samples that are neither CharLS's nor the
    // stream's own, and the count of each damaged outcome is pinned in
    // `CORPUS_OUTCOMES`.
    use std::collections::{BTreeMap, HashMap};
    let mut streams = HashMap::new();
    for line in include_str!("fixtures/jpegls_charls_streams.tsv")
        .lines()
        .filter(|line| !line.starts_with('#'))
    {
        let (name, hex) = line.split_once('\t').ok_or("stream line")?;
        streams.insert(name, unhex(hex)?);
    }
    let verdicts: Vec<[&str; 4]> = include_str!("fixtures/jpegls_charls_verdicts.tsv")
        .lines()
        .filter(|line| !line.starts_with('#'))
        .map(|line| {
            let fields: Vec<&str> = line.split('\t').collect();
            <[&str; 4]>::try_from(fields).map_err(|_| "verdict line")
        })
        .collect::<Result<_, _>>()?;
    let undamaged: HashMap<&str, &str> = verdicts
        .iter()
        .filter(|[_, mutation, _, _]| *mutation == "none")
        .map(|[name, _, _, charls]| (*name, *charls))
        .collect();

    let mut outcomes: BTreeMap<(String, String), usize> = BTreeMap::new();
    for [name, mutation, class, charls] in &verdicts {
        let base = streams.get(name).ok_or("unknown stream")?;
        let mutated = mutate(base, mutation)?.ok_or("mutation not applicable")?;
        let ocelli = ocelli_digest(&mutated, base)?;
        let own = undamaged.get(name).copied().ok_or("no undamaged row")?;
        let outcome = if ocelli == *charls {
            "agree"
        } else if ocelli == "refuse" && *charls != own {
            "refused, CharLS draws other samples"
        } else if ocelli == "refuse" {
            "refused, CharLS draws its own samples"
        } else if ocelli == own && *charls == "refuse" {
            "opened, CharLS refuses"
        } else {
            return Err(format!("{name}:{mutation}: Ocelli drew a wrong sample").into());
        };
        if *class == "undamaged" {
            assert!(
                outcome == "agree",
                "{name}: an undamaged stream did not decode as CharLS decodes it"
            );
        }
        *outcomes
            .entry(((*class).to_owned(), outcome.to_owned()))
            .or_default() += 1;
    }
    let pinned: BTreeMap<(String, String), usize> = CORPUS_OUTCOMES
        .iter()
        .map(|(class, outcome, count)| (((*class).to_owned(), (*outcome).to_owned()), *count))
        .collect();
    assert_eq!(outcomes, pinned);
    // The only undamaged stream CharLS refuses is the hand-built one-padded
    // line, and Ocelli refuses it too.
    for (name, charls) in &undamaged {
        assert_eq!(
            *charls == "refuse",
            *name == "jpegls_final_ff_one_padded_u8",
            "{name}"
        );
    }
    Ok(())
}

#[test]
fn a_damaged_scan_that_reads_as_a_shorter_valid_scan_is_refused() -> TestResult {
    // Review pass 2's two smallest cases. Damage turned each scan into a valid
    // scan of another image that ends one byte early, with the original final
    // zero byte left over before EOI. CharLS 2.4.4 refuses both, recorded by
    // the generator. Before the end-of-scan allowance was narrowed to 7 bits
    // Ocelli opened both with `Ok`: the P 8 frame with its last sample 113
    // where the stream encodes 114, and the P 16 NEAR 2 ramp with 8 of its 9
    // samples wrong.
    for (source, decoder, bits) in [
        (
            include_bytes!("fixtures/jpegls_damaged_shorter_u8_3x3.jls").as_slice(),
            JpegLsDecoder::lossless(),
            8,
        ),
        (
            include_bytes!("fixtures/jpegls_damaged_shorter_u16_near2_3x3.jls").as_slice(),
            JpegLsDecoder::near_lossless(),
            16,
        ),
    ] {
        let desc = frame(
            3,
            3,
            bits,
            bits,
            PixelRepresentation::Unsigned,
            "MONOCHROME2",
        )?;
        let mut out = vec![0xa5; desc.output_len()];
        assert_eq!(
            decoder.decode(source, &desc, &mut out),
            Err(CodecError::DecoderFailure),
            "P {bits}"
        );
        assert!(out.iter().all(|byte| *byte == 0xa5));
    }
    Ok(())
}

#[test]
fn fill_bytes_before_end_of_image_are_accepted() -> TestResult {
    // ITU-T T.87 D.2: optional 0xff fill bytes may precede a marker
    // appended to the compressed image data. They sit outside the entropy-coded data, so
    // the reported frame with one or two fill bytes before EOI decodes to the
    // same samples. CharLS 2.4.4 was measured to open both to the frame's own
    // samples.
    let reported = include_bytes!("fixtures/jpegls_two_band_u16_32x32.jls");
    let desc = frame(32, 32, 16, 16, PixelRepresentation::Unsigned, "MONOCHROME2")?;
    for fill in [[0xff].as_slice(), [0xff, 0xff].as_slice()] {
        let filled = insert_before_eoi(reported, fill);
        let mut out = vec![0xa5; desc.output_len()];
        JpegLsDecoder::lossless().decode(&filled, &desc, &mut out)?;
        assert_eq!(
            out,
            two_band(32, 32, BAND_LOW, BAND_HIGH),
            "{} fill bytes",
            fill.len()
        );
    }
    Ok(())
}

#[test]
fn a_context_reset_floors_a_negative_bias() -> TestResult {
    // At a context RESET, A, B and N are halved. CharLS 2.4.4 floors B, an
    // arithmetic shift, and the published vendored code divided, which rounds
    // an odd negative B toward zero and moves the bias correction. Frames
    // encoded from real corpus CT slices desynchronised on it, which review
    // pass 1 measured as up to 114,504 of 115,600 samples wrong with `Ok`
    // before F-243 and refused after the run-mode patch alone. This is a synthetic CT-like 64 by 64 slice at
    // P 12, from a fixed seed in the generator, which reaches the same reset.
    // Measured on the original tree `aba1cb09`: both decode with `Ok`,
    // lossless with 1,596 of 4,096 samples wrong and NEAR 2 with 2,035 wrong
    // against CharLS's decode.
    let desc = frame(64, 64, 16, 12, PixelRepresentation::Unsigned, "MONOCHROME2")?;
    let image = include_bytes!("fixtures/jpegls_ct_like_u12_64.raw");
    let mut out = vec![0xa5; desc.output_len()];
    JpegLsDecoder::lossless().decode(
        include_bytes!("fixtures/jpegls_ct_like_u12_64.jls"),
        &desc,
        &mut out,
    )?;
    assert_eq!(mono16_mismatches(&out, image), (0, None));

    let mut out = vec![0xa5; desc.output_len()];
    JpegLsDecoder::near_lossless().decode(
        include_bytes!("fixtures/jpegls_ct_like_u12_64_near2.jls"),
        &desc,
        &mut out,
    )?;
    assert_eq!(
        mono16_mismatches(
            &out,
            include_bytes!("fixtures/jpegls_ct_like_u12_64_near2_charls.raw")
        ),
        (0, None)
    );
    for (got, want) in out.chunks_exact(2).zip(image.chunks_exact(2)) {
        let got = u16::from_le_bytes(<[u8; 2]>::try_from(got)?);
        let want = u16::from_le_bytes(<[u8; 2]>::try_from(want)?);
        assert!(got.abs_diff(want) <= 2);
    }
    Ok(())
}

#[test]
fn the_vendored_encoder_follows_the_same_run_rule() -> TestResult {
    // F-243 fixes the vendored encoder's mirror of the run-index defect, T.87
    // A.7.1 code segment A.16, although no shipped Ocelli path calls it. This
    // test is the only caller. Its upstream round trips pair it with the
    // decoder, so without this check an encoder still raising RUNindex on the
    // hit that ends a line would agree with nothing and go unnoticed. The
    // 8 by 4 two-band frame reaches that hit.
    let samples: Vec<u16> = (0..32)
        .map(|index| if index % 4 < 2 { BAND_LOW } else { BAND_HIGH })
        .collect();
    let encoded = ritk_codecs::jpeg_ls::encoder::encode_grayscale_jpeg_ls(&samples, 8, 4, 16, 0)?;
    let desc = frame(8, 4, 16, 16, PixelRepresentation::Unsigned, "MONOCHROME2")?;
    let mut out = vec![0xa5; desc.output_len()];
    JpegLsDecoder::lossless().decode(&encoded, &desc, &mut out)?;
    assert_eq!(out, two_band(8, 4, BAND_LOW, BAND_HIGH));
    // And it writes the scan CharLS wrote for the same frame, byte for byte.
    let charls = include_bytes!("fixtures/jpegls_two_band_u16_8x4.jls");
    assert_eq!(
        encoded.get(scan_start(&encoded)?..),
        charls.get(scan_start(charls)?..)
    );
    Ok(())
}

#[test]
fn a_run_miss_past_the_line_end_is_refused() -> TestResult {
    // T.87 F.1 step 18, J from A.2.1 step 3. P 8, one line of 13 zeros, all in run mode.
    // The 0xff byte is eight hits, J = 0 0 0 0 1 1 1 1, so the run reaches
    // 1 + 1 + 1 + 1 + 2 + 2 + 2 + 2 = 12 at RUNindex 8, where J = 2. The next
    // byte carries a stuffed zero bit and seven data bits.
    //
    // `FF 30`: data bits 0 1 1 0 0 0 0, a miss whose two remainder bits 11
    // give 3, so the run would be 15, past the 13 samples on the line. CharLS
    // refuses it, recorded by the generator. Before F-243 the vendored loop
    // clamped it to 13 and returned 13 zeros with `Ok`.
    let line = frame(1, 13, 8, 8, PixelRepresentation::Unsigned, "MONOCHROME2")?;
    let mut out = vec![0xa5; line.output_len()];
    assert_eq!(
        JpegLsDecoder::lossless().decode(
            include_bytes!("fixtures/jpegls_run_miss_past_line_u8.jls"),
            &line,
            &mut out,
        ),
        Err(CodecError::DecoderFailure)
    );
    assert!(out.iter().all(|byte| *byte == 0xa5));

    // `FF 40`: data bits 1 0 0 0 0 0 0, one hit whose count is
    // min(2^2, 13 - 12) = 1, which fills the line. CharLS decodes 13 zeros.
    let mut out = vec![0xa5; line.output_len()];
    JpegLsDecoder::lossless().decode(
        include_bytes!("fixtures/jpegls_run_partial_hit_u8.jls"),
        &line,
        &mut out,
    )?;
    assert_eq!(out, vec![0u8; 13]);
    Ok(())
}

#[test]
fn default_thresholds_clamp_to_the_lower_bound() -> TestResult {
    // ISO/IEC 14495-1 C.2.4.1.1.1, MAXVAL >= 128:
    //   FACTOR = floor((min(MAXVAL, 4095) + 128) / 256)
    //   T1 = CLAMP(FACTOR * (3 - 2) + 2 + 3 * NEAR, NEAR + 1, MAXVAL)
    //   T2 = CLAMP(FACTOR * (7 - 3) + 3 + 5 * NEAR, T1, MAXVAL)
    //   T3 = CLAMP(FACTOR * (21 - 4) + 4 + 7 * NEAR, T2, MAXVAL)
    //   CLAMP(i, j, MAXVAL) = j if i > MAXVAL or i < j, otherwise i
    //
    // P 8, NEAR 34: FACTOR = floor(383 / 256) = 1,
    //   T1 = CLAMP(105, 35, 255) = 105, T2 = CLAMP(177, 105, 255) = 177,
    //   T3 = CLAMP(259, 177, 255) = 177, because 259 is above MAXVAL.
    // A saturating clamp gives T3 = 255 instead.
    // P 8, NEAR 0: FACTOR 1, so 3, 7, 21.
    // P 16, NEAR 0: FACTOR = floor(4223 / 256) = 16, so 18, 67, 276, the LSE
    // CharLS writes. P 16, NEAR 3: 27, 82, 297, measured in CharLS's LSE too.
    //
    // CharLS writes no LSE for this 8-bit frame, so the decoder's defaults are
    // the thresholds. The frame is 4 by 4 blocks at 10, 140 and 250, whose
    // source edges are 110, 130 and 240, and a gradient from 177 upward
    // quantizes to 4 under T3 = 177 and to 3 or 4 under T3 = 255. With only the
    // CLAMP reverted this stream is refused as desynchronised. Every two-band
    // 8-bit frame tried at NEAR 34 decoded identically under both readings, so
    // the two-band formula cannot reach this row, measured on 2026-11-07.
    let codestream = include_bytes!("fixtures/jpegls_banded_u8_near34.jls");
    assert!(!codestream.windows(2).any(|pair| pair == [0xff, 0xf8]));
    let formula = include_bytes!("fixtures/jpegls_banded_u8_near34.raw");
    let charls = include_bytes!("fixtures/jpegls_banded_u8_near34_charls.raw");
    let desc = frame(32, 32, 8, 8, PixelRepresentation::Unsigned, "MONOCHROME2")?;
    let mut out = vec![0xa5; desc.output_len()];
    JpegLsDecoder::near_lossless().decode(codestream, &desc, &mut out)?;
    let differing = out
        .iter()
        .zip(charls.iter())
        .filter(|(got, want)| got != want)
        .count();
    assert_eq!(differing, 0, "samples that differ from CharLS's decode");
    assert_eq!(out.len(), charls.len());
    for (index, (got, want)) in out.iter().zip(formula.iter()).enumerate() {
        assert!(
            got.abs_diff(*want) <= 34,
            "sample {index}: {got} against {want}"
        );
    }
    for (index, want) in formula.iter().enumerate() {
        let (row, column) = (index / 32, index % 32);
        let expected = [10, 140, 250].get((row / 4 + column / 4) % 3).copied();
        assert_eq!(Some(*want), expected);
    }
    Ok(())
}

/// The lossless corpus fixture with its LSE ID 1 parameters replaced.
///
/// ISO/IEC 14495-1 C.2.4.1.1: `LSE Ll=13 ID=1 MAXVAL T1 T2 T3 RESET`, each
/// parameter 16 bits.
fn with_preset(
    maxval: u16,
    t1: u16,
    t2: u16,
    t3: u16,
    reset: u16,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut source = include_bytes!("fixtures/jpegls_corpus_lossless.jls").to_vec();
    let offset = source
        .windows(5)
        .position(|window| window == [0xff, 0xf8, 0x00, 0x0d, 0x01])
        .ok_or("no LSE ID 1 segment")?;
    // The defaults CharLS wrote at P 16 and NEAR 0, checked before editing.
    let parameters = source
        .get_mut(offset + 5..offset + 15)
        .ok_or("LSE segment truncated")?;
    assert_eq!(
        parameters,
        &[0xff, 0xff, 0x00, 0x12, 0x00, 0x43, 0x01, 0x14, 0x00, 0x40]
    );
    for (slot, value) in parameters
        .chunks_exact_mut(2)
        .zip([maxval, t1, t2, t3, reset])
    {
        slot.copy_from_slice(&value.to_be_bytes());
    }
    Ok(source)
}

#[test]
fn lse_presets_not_applied_are_refused_by_name() -> TestResult {
    // C.2.4.1.1. Each row edits one thing in CharLS's default LSE at P 16,
    // MAXVAL 65535, T1 18, T2 67, T3 276, RESET 64. Before F-243 the vendored
    // decoder ignored MAXVAL and RESET and returned `Ok`.
    use ocelli_codec::JpegLsPreset;
    let desc = corpus_desc()?;
    let refused = [
        ((4095, 18, 67, 276, 64), JpegLsPreset::MaxVal),
        ((65535, 18, 67, 276, 32), JpegLsPreset::Reset),
        (
            (65535, 30, 0, 0, 64),
            JpegLsPreset::PartialDefaultThresholds,
        ),
        (
            (65535, 0, 100, 0, 64),
            JpegLsPreset::PartialDefaultThresholds,
        ),
        ((65535, 67, 18, 276, 64), JpegLsPreset::ThresholdOrder),
    ];
    for ((maxval, t1, t2, t3, reset), preset) in refused {
        let source = with_preset(maxval, t1, t2, t3, reset)?;
        let mut out = vec![0xa5; desc.output_len()];
        assert_eq!(
            JpegLsDecoder::lossless().decode(&source, &desc, &mut out),
            Err(CodecError::UnsupportedPreset(preset)),
            "{maxval} {t1} {t2} {t3} {reset}"
        );
        assert!(out.iter().all(|byte| *byte == 0xa5));
    }

    // Zero selects the default, and at P 16 and NEAR 0 each default is the
    // value the stream was encoded with, so each of these decodes exactly.
    let reference = include_bytes!("fixtures/jpegls_corpus_reference_u16le.raw");
    for (maxval, t1, t2, t3, reset) in [
        (0, 18, 67, 276, 64),
        (65535, 18, 67, 276, 0),
        (65535, 0, 0, 0, 64),
    ] {
        let source = with_preset(maxval, t1, t2, t3, reset)?;
        let mut out = vec![0xa5; desc.output_len()];
        JpegLsDecoder::lossless().decode(&source, &desc, &mut out)?;
        assert_eq!(
            out.as_slice(),
            reference.as_slice(),
            "{maxval} {t1} {t2} {t3} {reset}"
        );
    }
    // The two bounds no P 16 case reaches. At P 12 MAXVAL is 4095, so a T3 of
    // 5000 is above it, and at NEAR 3 a T1 of 3 equals NEAR where C.2.4.1.1
    // asks for at least NEAR + 1. Each segment is inserted before SOS in a
    // 12-bit fixture that carries none, with MAXVAL and RESET zero.
    let twelve = frame(64, 64, 16, 12, PixelRepresentation::Unsigned, "MONOCHROME2")?;
    for (source, decoder, thresholds) in [
        (
            include_bytes!("fixtures/jpegls_ramp_u12.jls").as_slice(),
            JpegLsDecoder::lossless(),
            [18u16, 67, 5000],
        ),
        (
            include_bytes!("fixtures/jpegls_ramp_u12_near3.jls").as_slice(),
            JpegLsDecoder::near_lossless(),
            [3, 82, 297],
        ),
    ] {
        let sos = source
            .windows(2)
            .position(|pair| pair == [0xff, 0xda])
            .ok_or("no SOS")?;
        let (head, tail) = source.split_at(sos);
        let mut lse = vec![0xff, 0xf8, 0x00, 0x0d, 0x01, 0x00, 0x00];
        for value in thresholds {
            lse.extend_from_slice(&value.to_be_bytes());
        }
        lse.extend_from_slice(&[0x00, 0x00]);
        let edited = [head, lse.as_slice(), tail].concat();
        let mut out = vec![0xa5; twelve.output_len()];
        assert_eq!(
            decoder.decode(&edited, &twelve, &mut out),
            Err(CodecError::UnsupportedPreset(JpegLsPreset::ThresholdOrder)),
            "{thresholds:?}"
        );
        assert!(out.iter().all(|byte| *byte == 0xa5));
    }
    assert_eq!(
        CodecError::UnsupportedPreset(JpegLsPreset::Reset).to_string(),
        "JPEG-LS preset parameter is not applied by this decoder: RESET"
    );
    Ok(())
}

#[test]
fn manifest_backed_banded_rows_reach_the_run_mode_trigger() -> TestResult {
    // `syntax/jpegls_banded_lossless.dcm` and
    // `syntax/jpegls_banded_near_lossless.dcm`, encoded by
    // `scripts/corpus_synth.py` through pyjpegls from
    // `syntax/reference_banded16.dcm`, the two-band formula written
    // uncompressed. Before F-243 the lossless row decoded with 916 wrong
    // samples and the near-lossless row with 906 outside its NEAR bound, both
    // with `Ok`, which no earlier corpus row could show.
    let reference = include_bytes!("fixtures/jpegls_corpus_banded_reference_u16le.raw");
    assert_eq!(
        reference.as_slice(),
        two_band(32, 32, BAND_LOW, BAND_HIGH).as_slice()
    );
    let desc = frame(32, 32, 16, 16, PixelRepresentation::Unsigned, "MONOCHROME2")?;

    let mut out = vec![0xa5; desc.output_len()];
    JpegLsDecoder::lossless().decode(
        include_bytes!("fixtures/jpegls_corpus_banded_lossless.jls"),
        &desc,
        &mut out,
    )?;
    assert_eq!(mono16_mismatches(&out, reference), (0, None));

    let mut out = vec![0xa5; desc.output_len()];
    JpegLsDecoder::near_lossless().decode(
        include_bytes!("fixtures/jpegls_corpus_banded_near3.jls"),
        &desc,
        &mut out,
    )?;
    let (_, over_bound) = mono16_error(&out, reference)?;
    assert_eq!(over_bound, 0, "samples outside NEAR {NEAR}");
    assert_eq!(
        mono16_mismatches(
            &out,
            include_bytes!("fixtures/jpegls_corpus_banded_near3_charls.raw")
        ),
        (0, None)
    );
    Ok(())
}
