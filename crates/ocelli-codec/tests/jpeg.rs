use std::error::Error;

use ocelli_codec::{
    Capability, CodecError, DecodePhotometricInterpretation, DecodeSampleLayout, Decoder,
    FrameDesc, FrameDescInput, JpegDecoder, PixelDataVr, PixelRepresentation, Registry,
    RegistryError, register_jpeg_decoders,
};

type TestResult = Result<(), Box<dyn Error>>;

const JPEG_BASELINE: &str = "1.2.840.10008.1.2.4.50";
const JPEG_EXTENDED: &str = "1.2.840.10008.1.2.4.51";
const JPEG_LOSSLESS: &str = "1.2.840.10008.1.2.4.57";
const JPEG_LOSSLESS_SV1: &str = "1.2.840.10008.1.2.4.70";

fn monochrome_12bit_desc() -> Result<FrameDesc, Box<dyn Error>> {
    Ok(FrameDesc::new(FrameDescInput {
        rows: 1,
        columns: 4,
        samples_per_pixel: 1,
        bits_allocated: 16,
        bits_stored: 12,
        high_bit: 11,
        pixel_representation: PixelRepresentation::Unsigned,
        photometric_interpretation: "MONOCHROME2".to_owned(),
        pixel_data_vr: PixelDataVr::Ob,
    })?)
}

fn baseline_colour_desc() -> Result<FrameDesc, Box<dyn Error>> {
    Ok(FrameDesc::new(FrameDescInput {
        rows: 64,
        columns: 96,
        samples_per_pixel: 3,
        bits_allocated: 8,
        bits_stored: 8,
        high_bit: 7,
        pixel_representation: PixelRepresentation::Unsigned,
        photometric_interpretation: "YBR_FULL_422".to_owned(),
        pixel_data_vr: PixelDataVr::Ob,
    })?)
}

fn extended_12bit_desc() -> Result<FrameDesc, Box<dyn Error>> {
    Ok(FrameDesc::new(FrameDescInput {
        rows: 64,
        columns: 96,
        samples_per_pixel: 1,
        bits_allocated: 16,
        bits_stored: 12,
        high_bit: 11,
        pixel_representation: PixelRepresentation::Unsigned,
        photometric_interpretation: "MONOCHROME2".to_owned(),
        pixel_data_vr: PixelDataVr::Ob,
    })?)
}

#[test]
fn process_14_preserves_hand_computed_12_bit_extrema_in_little_endian_containers() -> TestResult {
    // DICOM PS3.5 Annex F maps JPEG lossless process 14 decoded samples into
    // the DICOM native little-endian 16-bit containers described by the
    // Image Pixel module. These four unsigned 12-bit samples are exactly
    // 0, 2047, 2048, and 4095, so their container bytes are independent of
    // the production decoder.
    let expected = [0x00, 0x00, 0xff, 0x07, 0x00, 0x08, 0xff, 0x0f];
    let mut out = [0xa5; 8];
    JpegDecoder::lossless().decode(
        include_bytes!("fixtures/jpeg_lossless_process14_12bit.jpg"),
        &monochrome_12bit_desc()?,
        &mut out,
    )?;
    assert_eq!(out, expected);
    Ok(())
}

#[test]
fn process_14_selection_value_1_preserves_the_same_exact_samples() -> TestResult {
    // DICOM PS3.5 Annex F assigns selection value 1 to UID .70. Predictor
    // selection changes the coded differences, not the reconstructed sample
    // values or the DICOM native little-endian container order.
    let expected = [0x00, 0x00, 0xff, 0x07, 0x00, 0x08, 0xff, 0x0f];
    let mut registry = Registry::new();
    register_jpeg_decoders(&mut registry)?;
    let mut out = [0xa5; 8];
    registry.decode(
        JPEG_LOSSLESS_SV1,
        include_bytes!("fixtures/jpeg_lossless_sv1_12bit.jpg"),
        &monochrome_12bit_desc()?,
        &mut out,
    )?;
    assert_eq!(out, expected);
    assert_eq!(registry.capability(JPEG_LOSSLESS), Capability::Available);
    Ok(())
}

#[test]
fn dicom_even_item_padding_before_or_after_eoi_is_accepted() -> TestResult {
    // PS3.5 A.4 requires even Fragment Item Values. It permits ISO 10918-1
    // FF fill before EOI or one encapsulation pad after EOI. This fixture's
    // unpadded JPEG stream is deliberately odd length, so both forms are 94
    // bytes and represent the same four independently known samples.
    let source = include_bytes!("fixtures/jpeg_lossless_process14_12bit.jpg");
    assert_eq!(source.len(), 93);
    let expected = [0x00, 0x00, 0xff, 0x07, 0x00, 0x08, 0xff, 0x0f];
    let decoder = JpegDecoder::lossless();
    let desc = monochrome_12bit_desc()?;

    let mut after_eoi = source.to_vec();
    after_eoi.push(0);
    let mut out = [0xa5; 8];
    decoder.decode(&after_eoi, &desc, &mut out)?;
    assert_eq!(out, expected);

    let mut before_eoi = source.to_vec();
    before_eoi.insert(source.len() - 1, 0xff);
    out.fill(0xa5);
    decoder.decode(&before_eoi, &desc, &mut out)?;
    assert_eq!(out, expected);
    Ok(())
}

#[test]
fn unnecessary_null_after_even_length_codestream_is_trailing_data() -> TestResult {
    // PS3.5 A.4 permits only padding needed to make a Fragment Item Value even.
    // This baseline stream is already even, so another NULL is trailing data.
    let source = include_bytes!("fixtures/jpeg_baseline_rgb.jpg");
    assert_eq!(source.len(), 936);
    let mut unnecessary_padding = source.to_vec();
    unnecessary_padding.push(0);
    assert_atomic_error(
        &JpegDecoder::baseline(),
        &unnecessary_padding,
        &baseline_colour_desc()?,
        include_bytes!("fixtures/jpeg_baseline_rgb_reference.raw").len(),
        CodecError::TrailingData,
    );
    Ok(())
}

#[test]
fn baseline_colour_reports_rgb_and_class_two_measurement_against_corpus_truth() -> TestResult {
    // The codestream is extracted from the deterministic synthetic
    // `jpeg_baseline_rgb8.dcm` corpus row. Its independent source is the native
    // RGB Pixel Data from `reference_rgb8.dcm`. DICOM PS3.3 C.7.6.3.1.2
    // permits encapsulated JPEG to carry YBR while this decoder reports RGB.
    // HLD deviation D-16 defines class two as reported per-channel difference
    // statistics with no invented pass threshold.
    let desc = baseline_colour_desc()?;
    let expected = include_bytes!("fixtures/jpeg_baseline_rgb_reference.raw");
    let mut registry = Registry::new();
    register_jpeg_decoders(&mut registry)?;
    assert_eq!(
        registry.decode_photometric_interpretation(JPEG_BASELINE, &desc)?,
        DecodePhotometricInterpretation::Rgb
    );
    assert_eq!(
        registry.decode_sample_layout(JPEG_BASELINE, &desc)?,
        DecodeSampleLayout::Interleaved
    );
    let mut out = vec![0xa5; expected.len()];
    registry.decode(
        JPEG_BASELINE,
        include_bytes!("fixtures/jpeg_baseline_rgb.jpg"),
        &desc,
        &mut out,
    )?;
    let measurements = class_two_measurement(&out, expected)?;
    println!(
        "class-two observation, unmeasured ({}): {:?}",
        JPEG_BASELINE, measurements
    );
    for measurement in &measurements {
        assert_eq!(measurement.pixels, 6_144);
    }
    Ok(())
}

#[test]
fn extended_process_2_eight_bit_decodes_against_independent_dcmtk_truth() -> TestResult {
    // DICOM PS3.5 A.4.1 assigns 8-bit process 2 and 12-bit process 4 to `.51`.
    // This SOF1 fixture is derived from the synthetic baseline codestream by
    // changing only its process marker. The expected RGB was independently
    // decoded by DCMTK `dcmdjpeg`. D-16 requires reporting the class-two
    // measurement rather than inventing a threshold.
    let desc = baseline_colour_desc()?;
    let expected = include_bytes!("fixtures/jpeg_extended_process2_8bit_reference.raw");
    let mut out = vec![0xa5; expected.len()];
    let decoder = JpegDecoder::extended();
    assert_eq!(
        decoder.decode_photometric_interpretation(&desc),
        DecodePhotometricInterpretation::Rgb
    );
    assert_eq!(
        decoder.decode_sample_layout(&desc),
        DecodeSampleLayout::Interleaved
    );
    decoder.decode(
        include_bytes!("fixtures/jpeg_extended_process2_8bit.jpg"),
        &desc,
        &mut out,
    )?;
    let measurements = class_two_measurement(&out, expected)?;
    println!(
        "class-two observation, unmeasured ({}): {:?}",
        JPEG_EXTENDED, measurements
    );
    for measurement in &measurements {
        assert_eq!(measurement.pixels, 6_144);
    }
    Ok(())
}

#[test]
fn class_two_hand_metric_controls_match_counted_triplet_differences() -> TestResult {
    // Exact independent hand metric controls on fixed numeric RGB triplets
    // with no decoder, fixture or output-derived expectation. Every expected
    // value is derived directly by counting the per-channel differences of
    // the pixels below, not from any DICOM pixel formula. D-16 reports
    // measured statistics and never invents a pass threshold, so these only
    // pin the counting arithmetic itself.
    // R: differences 1 and 2 give pixels 2, differing 2, signed -1, max 2,
    // p999 2. G: differences 2 and 0 give differing 1, signed -2, max 2,
    // p999 2. B: differences 0 and 5 give differing 1, signed 5, max 5,
    // p999 5.
    let actual = [11_u8, 18, 30, 8, 20, 35];
    let expected = [10_u8, 20, 30, 10, 20, 30];
    assert_eq!(
        class_two_measurement(&actual, &expected)?,
        [
            ClassTwoMeasurement {
                pixels: 2,
                differing: 2,
                signed_sum: -1,
                max_abs_diff: 2,
                percentile_999_abs_diff: 2,
            },
            ClassTwoMeasurement {
                pixels: 2,
                differing: 1,
                signed_sum: -2,
                max_abs_diff: 2,
                percentile_999_abs_diff: 2,
            },
            ClassTwoMeasurement {
                pixels: 2,
                differing: 1,
                signed_sum: 5,
                max_abs_diff: 5,
                percentile_999_abs_diff: 5,
            },
        ]
    );
    Ok(())
}

#[test]
fn class_two_nearest_rank_boundary_control_separates_max_from_percentile() -> TestResult {
    // Nearest-rank boundary control: one thousand black expected RGB
    // triplets against black actual triplets where only the first R sample
    // is 1 and the last R sample is 255. Counting those differences gives R
    // pixels 1000, differing 2, signed 256, max 255 and p999 1, because the
    // 999th ordered difference is 1 under the existing 999 per 1000
    // nearest-rank rule. Max and percentile are distinct statistics and the
    // boundary sits exactly on the 999 sample threshold. G and B count all
    // zeros except pixels 1000.
    let expected = vec![0_u8; 3_000];
    let mut actual = vec![0_u8; 3_000];
    *actual
        .get_mut(0)
        .ok_or("hand fixture has no first red sample")? = 1;
    *actual
        .get_mut(2_997)
        .ok_or("hand fixture has no last red sample")? = 255;
    assert_eq!(
        class_two_measurement(&actual, &expected)?,
        [
            ClassTwoMeasurement {
                pixels: 1_000,
                differing: 2,
                signed_sum: 256,
                max_abs_diff: 255,
                percentile_999_abs_diff: 1,
            },
            ClassTwoMeasurement {
                pixels: 1_000,
                differing: 0,
                signed_sum: 0,
                max_abs_diff: 0,
                percentile_999_abs_diff: 0,
            },
            ClassTwoMeasurement {
                pixels: 1_000,
                differing: 0,
                signed_sum: 0,
                max_abs_diff: 0,
                percentile_999_abs_diff: 0,
            },
        ]
    );
    Ok(())
}

#[test]
fn class_two_measurement_rejects_unequal_and_non_triplet_buffers() -> TestResult {
    // The helper must keep refusing unequal-length buffers and buffers that
    // are not a whole number of RGB triplets, so mispaired pixel data can
    // never produce a statistic.
    let triplet = [11_u8, 18, 30];
    let mismatched = [10_u8, 20, 30, 10];
    assert!(class_two_measurement(&triplet, &mismatched).is_err());
    assert!(class_two_measurement(&mismatched, &triplet).is_err());
    let non_triplet = [0_u8; 4];
    assert!(class_two_measurement(&non_triplet, &non_triplet).is_err());
    Ok(())
}

#[test]
fn jpeg_registration_is_atomic_when_the_last_uid_collides() -> TestResult {
    let mut registry = Registry::new();
    registry.register(std::sync::Arc::new(JpegDecoder::lossless_sv1()))?;
    assert_eq!(
        register_jpeg_decoders(&mut registry),
        Err(RegistryError::AlreadyRegistered {
            uid: JPEG_LOSSLESS_SV1
        })
    );
    for uid in [JPEG_BASELINE, JPEG_EXTENDED, JPEG_LOSSLESS] {
        assert_eq!(registry.capability(uid), Capability::KnownUnavailable);
    }
    assert_eq!(
        registry.capability(JPEG_LOSSLESS_SV1),
        Capability::Available
    );
    Ok(())
}

#[test]
fn extended_12_bit_meets_the_fixed_mono16_tolerance_against_dcmtk() -> TestResult {
    // The codestream is extracted from the repository's deterministic
    // synthetic `.51` corpus row. The expected native samples were decoded
    // independently by DCMTK `dcmdjpeg`, then extracted without transformation.
    // DICOM PS3.5 Annex F defines the result as 12-bit samples in 16-bit
    // little-endian DICOM containers. JPEG Extended is lossy DCT, so HLD
    // section 25.1 supplies the fixed mono16 comparison policy: at least
    // 99.9 percent of samples differ by no more than one LSB and none differ
    // by more than two.
    let expected = include_bytes!("fixtures/jpeg_extended_12bit_reference.raw");
    let mut out = vec![0xa5; expected.len()];
    let mut registry = Registry::new();
    register_jpeg_decoders(&mut registry)?;
    registry.decode(
        JPEG_EXTENDED,
        include_bytes!("fixtures/jpeg_extended_12bit.jpg"),
        &extended_12bit_desc()?,
        &mut out,
    )?;
    let expected_samples = expected
        .chunks_exact(2)
        .map(le_u16)
        .collect::<Result<Vec<_>, _>>()?;
    let actual_samples = out
        .chunks_exact(2)
        .map(le_u16)
        .collect::<Result<Vec<_>, _>>()?;
    let differences: Vec<_> = actual_samples
        .into_iter()
        .zip(expected_samples)
        .map(|(actual, expected)| actual.abs_diff(expected))
        .collect();
    let within_one = differences
        .iter()
        .filter(|difference| **difference <= 1)
        .count();
    assert!(within_one * 1_000 >= differences.len() * 999);
    assert!(differences.iter().all(|difference| *difference <= 2));
    Ok(())
}

#[test]
fn validation_failures_leave_the_caller_buffer_unchanged() -> TestResult {
    let source = include_bytes!("fixtures/jpeg_lossless_sv1_12bit.jpg");
    let desc = monochrome_12bit_desc()?;

    let truncated = source.get(..source.len() - 1).ok_or("fixture too short")?;
    assert_atomic_error(
        &JpegDecoder::lossless_sv1(),
        truncated,
        &desc,
        8,
        CodecError::InvalidCodestream,
    );

    let mut trailing_non_pad = source.to_vec();
    trailing_non_pad.push(1);
    assert_atomic_error(
        &JpegDecoder::lossless_sv1(),
        &trailing_non_pad,
        &desc,
        8,
        CodecError::TrailingData,
    );

    let mut excess_padding = source.to_vec();
    excess_padding.extend_from_slice(&[0, 0]);
    assert_atomic_error(
        &JpegDecoder::lossless_sv1(),
        &excess_padding,
        &desc,
        8,
        CodecError::TrailingData,
    );

    let wrong_dimensions = FrameDesc::new(FrameDescInput {
        columns: 5,
        ..monochrome_12bit_input()
    })?;
    assert_atomic_error(
        &JpegDecoder::lossless_sv1(),
        source,
        &wrong_dimensions,
        10,
        CodecError::FrameMismatch,
    );

    let wrong_precision = FrameDesc::new(FrameDescInput {
        bits_stored: 11,
        high_bit: 10,
        ..monochrome_12bit_input()
    })?;
    assert_atomic_error(
        &JpegDecoder::lossless_sv1(),
        source,
        &wrong_precision,
        8,
        CodecError::FrameMismatch,
    );

    assert_atomic_error(
        &JpegDecoder::lossless_sv1(),
        include_bytes!("fixtures/jpeg_lossless_process14_12bit.jpg"),
        &desc,
        8,
        CodecError::FrameMismatch,
    );
    assert_atomic_error(
        &JpegDecoder::lossless_sv1(),
        source,
        &desc,
        7,
        CodecError::OutputLength {
            expected: 8,
            actual: 7,
        },
    );
    Ok(())
}

#[test]
fn dependency_decode_failure_leaves_the_caller_buffer_unchanged() -> TestResult {
    let mut source = include_bytes!("fixtures/jpeg_baseline_rgb.jpg").to_vec();
    let scan = source
        .windows(2)
        .position(|bytes| bytes == [0xff, 0xda])
        .ok_or("baseline fixture has no SOS marker")?;
    let table_selector = scan.checked_add(6).ok_or("SOS offset overflow")?;
    let byte = source
        .get_mut(table_selector)
        .ok_or("baseline fixture SOS is truncated")?;
    *byte = 0xff;
    assert_atomic_error(
        &JpegDecoder::baseline(),
        &source,
        &baseline_colour_desc()?,
        include_bytes!("fixtures/jpeg_baseline_rgb_reference.raw").len(),
        CodecError::DecoderFailure,
    );
    Ok(())
}

fn monochrome_12bit_input() -> FrameDescInput {
    FrameDescInput {
        rows: 1,
        columns: 4,
        samples_per_pixel: 1,
        bits_allocated: 16,
        bits_stored: 12,
        high_bit: 11,
        pixel_representation: PixelRepresentation::Unsigned,
        photometric_interpretation: "MONOCHROME2".to_owned(),
        pixel_data_vr: PixelDataVr::Ob,
    }
}

fn assert_atomic_error(
    decoder: &JpegDecoder,
    src: &[u8],
    desc: &FrameDesc,
    output_len: usize,
    expected: CodecError,
) {
    let mut out = vec![0xa5; output_len];
    assert_eq!(decoder.decode(src, desc, &mut out), Err(expected));
    assert!(out.iter().all(|byte| *byte == 0xa5));
}

fn le_u16(bytes: &[u8]) -> Result<u16, std::array::TryFromSliceError> {
    Ok(u16::from_le_bytes(<[u8; 2]>::try_from(bytes)?))
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct ClassTwoMeasurement {
    pixels: u64,
    differing: u64,
    signed_sum: i64,
    max_abs_diff: u8,
    percentile_999_abs_diff: u8,
}

fn class_two_measurement(
    actual: &[u8],
    expected: &[u8],
) -> Result<[ClassTwoMeasurement; 3], Box<dyn Error>> {
    if actual.len() != expected.len() || !actual.len().is_multiple_of(3) {
        return Err("class-two RGB buffers have different or invalid lengths".into());
    }
    let pixels = u64::try_from(actual.len() / 3)?;
    let mut measurements = [ClassTwoMeasurement {
        pixels,
        ..ClassTwoMeasurement::default()
    }; 3];
    let mut histograms = [[0_u64; 256]; 3];
    for (actual_pixel, expected_pixel) in actual.chunks_exact(3).zip(expected.chunks_exact(3)) {
        for (((actual_channel, expected_channel), measurement), histogram) in actual_pixel
            .iter()
            .zip(expected_pixel)
            .zip(&mut measurements)
            .zip(&mut histograms)
        {
            let difference = actual_channel.abs_diff(*expected_channel);
            let count = histogram
                .get_mut(usize::from(difference))
                .ok_or("class-two difference is outside its u8 histogram")?;
            *count += 1;
            measurement.differing += u64::from(difference != 0);
            measurement.signed_sum += i64::from(*actual_channel) - i64::from(*expected_channel);
            measurement.max_abs_diff = measurement.max_abs_diff.max(difference);
        }
    }
    for (measurement, histogram) in measurements.iter_mut().zip(histograms) {
        let mut running = 0_u64;
        for (difference, count) in histogram.into_iter().enumerate() {
            running += count;
            if running * 1_000 >= pixels * 999 {
                measurement.percentile_999_abs_diff = u8::try_from(difference)?;
                break;
            }
        }
    }
    Ok(measurements)
}
