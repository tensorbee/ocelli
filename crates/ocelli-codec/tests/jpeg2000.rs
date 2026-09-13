use std::error::Error;

use ocelli_codec::{
    Capability, CodecError, DecodePhotometricInterpretation, DecodeSampleLayout, Decoder,
    FrameDesc, FrameDescInput, Jpeg2000Decoder, PixelDataVr, PixelRepresentation, Registry,
    RegistryError, register_jpeg2000_decoders,
};

type TestResult = Result<(), Box<dyn Error>>;

const JPEG2000_LOSSLESS: &str = "1.2.840.10008.1.2.4.90";
const JPEG2000: &str = "1.2.840.10008.1.2.4.91";

#[test]
fn lossless_decodes_hand_computed_eight_and_sixteen_bit_signed_domains() -> TestResult {
    // DICOM PS3.5 A.4.4 reconstructs JPEG 2000 samples into the Image Pixel
    // module's native containers. Each OpenJPEG fixture repeats four boundary
    // values across a 64 by 64 frame. The bytes below are their independently
    // computed DICOM little-endian representations.
    assert_repeated_decode(
        include_bytes!("fixtures/jpeg2000_lossless_u8.j2k"),
        frame(64, 64, 8, 8, PixelRepresentation::Unsigned, "MONOCHROME2")?,
        &[0x00, 0x7f, 0x80, 0xff],
    )?;
    assert_repeated_decode(
        include_bytes!("fixtures/jpeg2000_lossless_s8.j2k"),
        frame(64, 64, 8, 8, PixelRepresentation::Signed, "MONOCHROME1")?,
        &[0x80, 0xff, 0x00, 0x7f],
    )?;
    assert_repeated_decode(
        include_bytes!("fixtures/jpeg2000_lossless_u12.j2k"),
        frame(64, 64, 16, 12, PixelRepresentation::Unsigned, "MONOCHROME2")?,
        &[0x00, 0x00, 0xff, 0x07, 0x00, 0x08, 0xff, 0x0f],
    )?;
    assert_repeated_decode(
        include_bytes!("fixtures/jpeg2000_lossless_s12.j2k"),
        frame(64, 64, 16, 12, PixelRepresentation::Signed, "MONOCHROME2")?,
        &[0x00, 0xf8, 0xff, 0xff, 0x00, 0x00, 0xff, 0x07],
    )?;
    Ok(())
}

#[test]
fn general_mode_accepts_reversible_but_lossless_mode_refuses_irreversible() -> TestResult {
    let reversible = include_bytes!("fixtures/jpeg2000_lossless_u8.j2k");
    let desc = frame(64, 64, 8, 8, PixelRepresentation::Unsigned, "MONOCHROME2")?;
    let mut out = vec![0xa5; desc.output_len()];
    Jpeg2000Decoder::general().decode(reversible, &desc, &mut out)?;

    let irreversible = include_bytes!("fixtures/jpeg2000_corpus_j2k_lossy.j2k");
    let desc = corpus_desc()?;
    assert_atomic_error(
        &Jpeg2000Decoder::lossless_only(),
        irreversible,
        &desc,
        CodecError::FrameMismatch,
    );
    Ok(())
}

#[test]
fn general_accepts_scalar_derived_qcd_but_lossless_refuses_it() -> TestResult {
    // DICOM PS3.5 A.4.4 permits general `.91` JPEG 2000 Part 1 streams while
    // `.90` requires no quantization. The adjacent generator deterministically
    // derives this style-1 QCD stream and its OpenJPEG 2.5.4 raw reference.
    let source = include_bytes!("fixtures/jpeg2000_scalar_derived_u8.j2k");
    let reference = include_bytes!("fixtures/jpeg2000_scalar_derived_u8_openjpeg.raw");
    let qcd = marker_offset(source, [0xff, 0x5c])?;
    assert_eq!(source.get(qcd + 2..qcd + 4), Some(&[0, 5][..]));
    assert_eq!(source.get(qcd + 4).map(|value| value & 0x1f), Some(1));

    let desc = frame(64, 64, 8, 8, PixelRepresentation::Unsigned, "MONOCHROME2")?;
    let mut out = vec![0xa5; desc.output_len()];
    Jpeg2000Decoder::general().decode(source, &desc, &mut out)?;
    assert_eq!(out.as_slice(), reference);

    assert_atomic_error(
        &Jpeg2000Decoder::lossless_only(),
        source,
        &desc,
        CodecError::FrameMismatch,
    );
    Ok(())
}

#[test]
fn lossless_requires_reversible_transform_and_no_quantization() -> TestResult {
    // DICOM PS3.5 A.4.4 binds `.90` to the reversible JPEG 2000 Part 1
    // process with no quantization. The vendored public encoder made this
    // 64 by 64 unsigned 8-bit constant-128 stream with five decomposition
    // levels and scalar step 4, then only its COD transform byte changed from
    // irreversible to reversible. That pairs scalar QCD with reversible COD
    // and holds both conditions independently.
    let source = include_bytes!("fixtures/jpeg2000_quantized_reversible_invalid.j2k");
    let desc = frame(64, 64, 8, 8, PixelRepresentation::Unsigned, "MONOCHROME2")?;
    assert_atomic_error(
        &Jpeg2000Decoder::lossless_only(),
        source,
        &desc,
        CodecError::FrameMismatch,
    );

    // The general `.91` syntax retains its existing policy and accepts either
    // JPEG 2000 Part 1 process.
    let mut out = vec![0xa5; desc.output_len()];
    Jpeg2000Decoder::general().decode(source, &desc, &mut out)?;
    assert!(out.iter().all(|sample| *sample == 128));

    let qcd = marker_offset(source, [0xff, 0x5c])?;
    let qcd_length = usize::from(u16::from_be_bytes(
        source
            .get(qcd + 2..qcd + 4)
            .ok_or("QCD length is truncated")?
            .try_into()?,
    ));

    let mut missing = source.to_vec();
    missing.drain(qcd..qcd + 2 + qcd_length);
    assert_atomic_error(
        &Jpeg2000Decoder::lossless_only(),
        &missing,
        &desc,
        CodecError::InvalidCodestream,
    );

    let mut duplicate = source.to_vec();
    let qcd_segment = source
        .get(qcd..qcd + 2 + qcd_length)
        .ok_or("QCD segment is truncated")?;
    duplicate.splice(qcd..qcd, qcd_segment.iter().copied());
    assert_atomic_error(
        &Jpeg2000Decoder::lossless_only(),
        &duplicate,
        &desc,
        CodecError::InvalidCodestream,
    );

    let mut override_qcc = source.to_vec();
    *override_qcc
        .get_mut(qcd + 1)
        .ok_or("QCD marker is truncated")? = 0x5d;
    assert_atomic_error(
        &Jpeg2000Decoder::lossless_only(),
        &override_qcc,
        &desc,
        CodecError::UnsupportedPixelFormat,
    );

    let mut invalid_style = source.to_vec();
    let style = invalid_style
        .get_mut(qcd + 4)
        .ok_or("QCD payload is truncated")?;
    *style = (*style & 0xe0) | 3;
    assert_atomic_error(
        &Jpeg2000Decoder::lossless_only(),
        &invalid_style,
        &desc,
        CodecError::InvalidCodestream,
    );
    Ok(())
}

#[test]
fn registration_is_atomic_and_reports_preserved_monochrome_output() -> TestResult {
    let mut registry = Registry::new();
    register_jpeg2000_decoders(&mut registry)?;
    for uid in [JPEG2000_LOSSLESS, JPEG2000] {
        assert_eq!(registry.capability(uid), Capability::Available);
        assert_eq!(
            registry.decode_photometric_interpretation(uid, &corpus_desc()?)?,
            DecodePhotometricInterpretation::Preserved
        );
        assert_eq!(
            registry.decode_sample_layout(uid, &corpus_desc()?)?,
            DecodeSampleLayout::Interleaved
        );
    }

    let mut collision = Registry::new();
    collision.register(std::sync::Arc::new(Jpeg2000Decoder::general()))?;
    assert_eq!(
        register_jpeg2000_decoders(&mut collision),
        Err(RegistryError::AlreadyRegistered { uid: JPEG2000 })
    );
    assert_eq!(
        collision.capability(JPEG2000_LOSSLESS),
        Capability::KnownUnavailable
    );
    Ok(())
}

#[test]
fn marker_and_descriptor_mismatches_preserve_caller_output() -> TestResult {
    let source = include_bytes!("fixtures/jpeg2000_lossless_u12.j2k");
    let desc = frame(64, 64, 16, 12, PixelRepresentation::Unsigned, "MONOCHROME2")?;

    assert_atomic_error(
        &Jpeg2000Decoder::lossless_only(),
        source.get(..source.len() - 1).ok_or("fixture too short")?,
        &desc,
        CodecError::InvalidCodestream,
    );

    let mut unnecessary_pad = source.to_vec();
    unnecessary_pad.push(0);
    assert_atomic_error(
        &Jpeg2000Decoder::lossless_only(),
        &unnecessary_pad,
        &desc,
        CodecError::TrailingData,
    );

    let mut mct = source.to_vec();
    let cod = marker_offset(&mct, [0xff, 0x52])?;
    let mct_offset = cod.checked_add(8).ok_or("COD offset overflow")?;
    *mct.get_mut(mct_offset).ok_or("COD marker is truncated")? = 1;
    assert_atomic_error(
        &Jpeg2000Decoder::lossless_only(),
        &mct,
        &desc,
        CodecError::UnsupportedPixelFormat,
    );

    let mut invalid_transform = source.to_vec();
    let cod = marker_offset(&invalid_transform, [0xff, 0x52])?;
    let transform_offset = cod.checked_add(13).ok_or("COD offset overflow")?;
    *invalid_transform
        .get_mut(transform_offset)
        .ok_or("COD marker is truncated")? = 2;
    assert_atomic_error(
        &Jpeg2000Decoder::lossless_only(),
        &invalid_transform,
        &desc,
        CodecError::InvalidCodestream,
    );

    let mut two_components = source.to_vec();
    let siz = marker_offset(&two_components, [0xff, 0x51])?;
    let siz_length = usize::from(u16::from_be_bytes(
        two_components
            .get(siz + 2..siz + 4)
            .ok_or("SIZ length is truncated")?
            .try_into()?,
    ));
    let component = two_components
        .get(siz + 40..siz + 43)
        .ok_or("SIZ component is truncated")?
        .to_vec();
    two_components.splice(siz + 2 + siz_length..siz + 2 + siz_length, component);
    two_components
        .get_mut(siz + 2..siz + 4)
        .ok_or("SIZ length is truncated")?
        .copy_from_slice(&u16::try_from(siz_length + 3)?.to_be_bytes());
    two_components
        .get_mut(siz + 38..siz + 40)
        .ok_or("SIZ component count is truncated")?
        .copy_from_slice(&2_u16.to_be_bytes());
    assert_atomic_error(
        &Jpeg2000Decoder::lossless_only(),
        &two_components,
        &desc,
        CodecError::UnsupportedPixelFormat,
    );

    assert_atomic_error(
        &Jpeg2000Decoder::lossless_only(),
        source,
        &frame(63, 64, 16, 12, PixelRepresentation::Unsigned, "MONOCHROME2")?,
        CodecError::FrameMismatch,
    );
    assert_atomic_error(
        &Jpeg2000Decoder::lossless_only(),
        source,
        &frame(64, 64, 16, 11, PixelRepresentation::Unsigned, "MONOCHROME2")?,
        CodecError::FrameMismatch,
    );
    assert_atomic_error(
        &Jpeg2000Decoder::lossless_only(),
        source,
        &frame(64, 64, 16, 12, PixelRepresentation::Signed, "MONOCHROME2")?,
        CodecError::FrameMismatch,
    );
    assert_atomic_error(
        &Jpeg2000Decoder::lossless_only(),
        source,
        &frame(64, 64, 16, 12, PixelRepresentation::Unsigned, "RGB")?,
        CodecError::UnsupportedPixelFormat,
    );

    assert_atomic_error(
        &Jpeg2000Decoder::lossless_only(),
        source,
        &frame_with_vr(
            64,
            64,
            16,
            12,
            PixelRepresentation::Unsigned,
            "MONOCHROME2",
            PixelDataVr::Ow,
        )?,
        CodecError::UnsupportedPixelFormat,
    );

    let mut wrong_length = vec![0xa5; desc.output_len() - 1];
    assert_eq!(
        Jpeg2000Decoder::lossless_only().decode(source, &desc, &mut wrong_length),
        Err(CodecError::OutputLength {
            expected: desc.output_len(),
            actual: desc.output_len() - 1,
        })
    );
    assert!(wrong_length.iter().all(|byte| *byte == 0xa5));
    Ok(())
}

#[test]
fn manifest_backed_lossless_is_exact_and_lossy_keeps_the_fixed_distribution() -> TestResult {
    // These codestreams come from the manifest-verified synthetic `.90` and
    // `.91` rows generated from `explicit_vr_le.dcm`. The native reference is
    // therefore independent of this decoder. DICOM PS3.5 A.4.4 requires `.90`
    // to reproduce it exactly. HLD 25.1 fixes the mono16 lossy predicate.
    let reference = include_bytes!("fixtures/jpeg2000_corpus_reference_u16le.raw");
    let independent_lossless =
        include_bytes!("fixtures/jpeg2000_corpus_j2k_lossless_pydicom_u16le.raw");
    let independent_lossy = include_bytes!("fixtures/jpeg2000_corpus_j2k_lossy_pydicom_u16le.raw");
    let desc = corpus_desc()?;
    let mut out = vec![0xa5; desc.output_len()];

    Jpeg2000Decoder::lossless_only().decode(
        include_bytes!("fixtures/jpeg2000_corpus_j2k_lossless.j2k"),
        &desc,
        &mut out,
    )?;
    assert_eq!(out.as_slice(), reference);
    assert_eq!(out.as_slice(), independent_lossless);

    out.fill(0xa5);
    Jpeg2000Decoder::general().decode(
        include_bytes!("fixtures/jpeg2000_corpus_j2k_lossy.j2k"),
        &desc,
        &mut out,
    )?;
    assert_eq!(
        mono16_measurement(&out, reference)?,
        Mono16Measurement {
            samples: 6_144,
            differing: 418,
            signed_sum: 30,
            max_abs_diff: 1,
            over_one: 0,
            over_two: 0,
        }
    );
    assert_eq!(
        mono16_measurement(&out, independent_lossy)?,
        Mono16Measurement {
            samples: 6_144,
            differing: 1_232,
            signed_sum: 730,
            max_abs_diff: 1,
            over_one: 0,
            over_two: 0,
        }
    );
    Ok(())
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

#[allow(clippy::too_many_arguments)]
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

fn corpus_desc() -> Result<FrameDesc, Box<dyn Error>> {
    frame(64, 96, 16, 16, PixelRepresentation::Unsigned, "MONOCHROME2")
}

fn assert_repeated_decode(source: &[u8], desc: FrameDesc, repeated: &[u8]) -> TestResult {
    let mut expected = Vec::with_capacity(desc.output_len());
    while expected.len() < desc.output_len() {
        expected.extend_from_slice(repeated);
    }
    let mut out = vec![0xa5; desc.output_len()];
    Jpeg2000Decoder::lossless_only().decode(source, &desc, &mut out)?;
    assert_eq!(out, expected);
    Ok(())
}

fn assert_atomic_error(
    decoder: &dyn Decoder,
    source: &[u8],
    desc: &FrameDesc,
    expected: CodecError,
) {
    let mut out = vec![0xa5; desc.output_len()];
    assert_eq!(decoder.decode(source, desc, &mut out), Err(expected));
    assert!(out.iter().all(|byte| *byte == 0xa5));
}

fn marker_offset(source: &[u8], marker: [u8; 2]) -> Result<usize, Box<dyn Error>> {
    source
        .windows(2)
        .position(|window| window == marker)
        .ok_or_else(|| "fixture marker is absent".into())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Mono16Measurement {
    samples: u64,
    differing: u64,
    signed_sum: i64,
    max_abs_diff: u16,
    over_one: u64,
    over_two: u64,
}

fn mono16_measurement(actual: &[u8], expected: &[u8]) -> Result<Mono16Measurement, Box<dyn Error>> {
    if actual.len() != expected.len() || !actual.len().is_multiple_of(2) {
        return Err("mono16 buffers have different or invalid lengths".into());
    }
    let mut measurement = Mono16Measurement {
        samples: u64::try_from(actual.len() / 2)?,
        differing: 0,
        signed_sum: 0,
        max_abs_diff: 0,
        over_one: 0,
        over_two: 0,
    };
    for (actual, expected) in actual.chunks_exact(2).zip(expected.chunks_exact(2)) {
        let actual = u16::from_le_bytes(<[u8; 2]>::try_from(actual)?);
        let expected = u16::from_le_bytes(<[u8; 2]>::try_from(expected)?);
        let difference = actual.abs_diff(expected);
        measurement.differing += u64::from(difference != 0);
        measurement.signed_sum += i64::from(actual) - i64::from(expected);
        measurement.max_abs_diff = measurement.max_abs_diff.max(difference);
        measurement.over_one += u64::from(difference > 1);
        measurement.over_two += u64::from(difference > 2);
    }
    assert!((measurement.samples - measurement.over_one) * 1_000 >= measurement.samples * 999);
    assert_eq!(measurement.over_two, 0);
    Ok(measurement)
}
