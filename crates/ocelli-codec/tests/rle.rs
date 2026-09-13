//! DICOM RLE fixtures derived from PS3.5 Annex G.

use std::error::Error;

use ocelli_codec::{
    CodecError, DecodeSampleLayout, Decoder, FrameDesc, FrameDescInput, PixelDataVr,
    PixelRepresentation, RleDecoder,
};

type TestResult = Result<(), Box<dyn Error>>;

fn frame(
    rows: u16,
    columns: u16,
    samples_per_pixel: u16,
    bits_allocated: u16,
) -> Result<FrameDesc, Box<dyn Error>> {
    Ok(FrameDesc::new(FrameDescInput {
        rows,
        columns,
        samples_per_pixel,
        bits_allocated,
        bits_stored: bits_allocated,
        high_bit: bits_allocated - 1,
        pixel_representation: PixelRepresentation::Unsigned,
        photometric_interpretation: if samples_per_pixel == 1 {
            "MONOCHROME2".to_owned()
        } else {
            "RGB".to_owned()
        },
        pixel_data_vr: PixelDataVr::Ob,
    })?)
}

fn encoded_frame(segments: &[&[u8]]) -> Vec<u8> {
    assert!(!segments.is_empty());
    assert!(segments.len() <= 15);
    let mut bytes = vec![0_u8; 64];
    assert!(set_u32_le(
        &mut bytes,
        0,
        u32::try_from(segments.len()).unwrap_or(0)
    ));
    let mut offset = 64_u32;
    for (index, segment) in segments.iter().enumerate() {
        let start = 4 + index * 4;
        assert!(set_u32_le(&mut bytes, start, offset));
        offset = offset
            .checked_add(u32::try_from(segment.len()).unwrap_or(0))
            .unwrap_or(0);
    }
    for segment in segments {
        bytes.extend_from_slice(segment);
    }
    bytes
}

fn set_u32_le(bytes: &mut [u8], start: usize, value: u32) -> bool {
    let Some(field) = bytes.get_mut(start..start + 4) else {
        return false;
    };
    field.copy_from_slice(&value.to_le_bytes());
    true
}

fn assert_atomic_error(source: &[u8], desc: &FrameDesc, expected: CodecError) {
    let mut out = vec![0xa5; desc.output_len()];
    assert_eq!(RleDecoder.decode(source, desc, &mut out), Err(expected));
    assert!(out.iter().all(|byte| *byte == 0xa5));
}

#[test]
fn two_rows_and_two_byte_planes_reconstruct_exact_little_endian_pixels() -> TestResult {
    // PS3.5 G.2 through G.5. Each row is encoded separately. Segment zero is
    // the MSB plane and segment one is the LSB plane. Row zero is one literal
    // run of four values. Row one is a repeat run of four values. Each encoded
    // segment is seven semantic bytes and therefore has one required zero pad.
    let msb = [3, 1, 3, 5, 7, 0xfd, 9, 0];
    let lsb = [3, 2, 4, 6, 8, 0xfd, 10, 0];
    let source = encoded_frame(&[&msb, &lsb]);
    let desc = frame(2, 4, 1, 16)?;
    let mut out = [0xa5; 16];

    RleDecoder.decode(&source, &desc, &mut out)?;

    assert_eq!(out, [2, 1, 4, 3, 6, 5, 8, 7, 10, 9, 10, 9, 10, 9, 10, 9,]);
    assert_eq!(
        RleDecoder.decode_sample_layout(&desc),
        DecodeSampleLayout::Interleaved
    );
    Ok(())
}

#[test]
fn legal_noop_is_accepted_without_becoming_padding() -> TestResult {
    // PS3.5 G.3.2 defines -128, encoded as 0x80, as a zero-output operation.
    // It remains part of the run stream. These even-length segments need no
    // pad and still produce two rows of four values each.
    let msb = [0x80, 3, 1, 3, 5, 7, 0xfd, 9];
    let lsb = [0x80, 3, 2, 4, 6, 8, 0xfd, 10];
    let desc = frame(2, 4, 1, 16)?;
    let mut out = [0xa5; 16];
    RleDecoder.decode(&encoded_frame(&[&msb, &lsb]), &desc, &mut out)?;
    assert_eq!(out[0], 2);
    assert_eq!(out[15], 9);
    Ok(())
}

#[test]
fn three_sample_planes_become_interleaved_rgb_pixels() -> TestResult {
    // PS3.5 G.2 orders one-byte RGB segments R, G, then B. Each segment is a
    // three-byte literal run plus the necessary zero pad to an even length.
    let red = [1, 10, 11, 0];
    let green = [1, 20, 21, 0];
    let blue = [1, 30, 31, 0];
    let desc = frame(1, 2, 3, 8)?;
    let mut out = [0xa5; 6];

    RleDecoder.decode(&encoded_frame(&[&red, &green, &blue]), &desc, &mut out)?;

    assert_eq!(out, [10, 20, 30, 11, 21, 31]);
    Ok(())
}

#[test]
fn sixteen_bit_rgb_uses_six_msb_first_planes_and_interleaves_little_endian() -> TestResult {
    // PS3.5 Table 8.2.2-1 and G.2. Sixteen-bit RGB has two byte planes for
    // each of R, G, and B. Within each sample, the MSB segment comes first.
    let source = six_plane_two_pixel_frame([
        [0x11, 0x77],
        [0x22, 0x88],
        [0x33, 0x99],
        [0x44, 0xaa],
        [0x55, 0xbb],
        [0x66, 0xcc],
    ]);
    let desc = frame_with("RGB", 3, 16, PixelRepresentation::Unsigned)?;
    let mut out = [0xa5; 12];

    RleDecoder.decode(&source, &desc, &mut out)?;

    assert_eq!(
        out,
        [
            0x22, 0x11, 0x44, 0x33, 0x66, 0x55, 0x88, 0x77, 0xaa, 0x99, 0xcc, 0xbb,
        ]
    );
    Ok(())
}

#[test]
fn sixteen_bit_ybr_full_uses_the_same_six_plane_container_contract() -> TestResult {
    // Table 8.2.2-1 permits YBR_FULL at sixteen bits. RLE retains its three
    // sample values and interleaves Y, Cb, and Cr without a colour transform.
    let source = six_plane_two_pixel_frame([
        [0x01, 0x07],
        [0x02, 0x08],
        [0x03, 0x09],
        [0x04, 0x0a],
        [0x05, 0x0b],
        [0x06, 0x0c],
    ]);
    let desc = frame_with("YBR_FULL", 3, 16, PixelRepresentation::Unsigned)?;
    let mut out = [0xa5; 12];

    RleDecoder.decode(&source, &desc, &mut out)?;

    assert_eq!(
        out,
        [
            0x02, 0x01, 0x04, 0x03, 0x06, 0x05, 0x08, 0x07, 0x0a, 0x09, 0x0c, 0x0b,
        ]
    );
    Ok(())
}

fn six_plane_two_pixel_frame(planes: [[u8; 2]; 6]) -> Vec<u8> {
    let encoded = planes.map(|[first, second]| [1, first, second, 0]);
    let [first, second, third, fourth, fifth, sixth] = encoded;
    encoded_frame(&[&first, &second, &third, &fourth, &fifth, &sixth])
}

#[test]
fn one_run_may_not_cross_an_image_row_even_when_total_output_is_exact() -> TestResult {
    // A single eight-byte literal expands to the correct total plane length,
    // but PS3.5 G.3.1 requires rows to be encoded separately.
    let cross_row = [7, 1, 3, 5, 7, 9, 9, 9, 9, 0];
    let valid = [3, 2, 4, 6, 8, 0xfd, 10, 0];
    assert_atomic_error(
        &encoded_frame(&[&cross_row, &valid]),
        &frame(2, 4, 1, 16)?,
        CodecError::InvalidCodestream,
    );
    Ok(())
}

#[test]
fn segment_padding_is_exactly_one_zero_only_when_semantic_length_is_odd() -> TestResult {
    let padded = [3, 1, 3, 5, 7, 0xfd, 9, 0];
    let other = [3, 2, 4, 6, 8, 0xfd, 10, 0];
    let desc = frame(2, 4, 1, 16)?;

    let mut nonzero_pad = padded;
    nonzero_pad[7] = 1;
    assert_atomic_error(
        &encoded_frame(&[&nonzero_pad, &other]),
        &desc,
        CodecError::TrailingData,
    );

    let even_stream = [0x80, 3, 1, 3, 5, 7, 0xfd, 9];
    let unnecessary = [0x80, 3, 1, 3, 5, 7, 0xfd, 9, 0, 0];
    assert_atomic_error(
        &encoded_frame(&[&unnecessary, &even_stream]),
        &desc,
        CodecError::TrailingData,
    );
    Ok(())
}

#[test]
fn unsupported_one_bit_and_thirty_two_bit_rle_are_permanent_refusals() -> TestResult {
    for bits in [1, 32] {
        let desc = frame(1, 8, 1, bits)?;
        assert_atomic_error(&[], &desc, CodecError::UnsupportedPixelFormat);
    }
    Ok(())
}

#[test]
fn table_822_1_rejects_invalid_photometric_sample_and_container_combinations() -> TestResult {
    let invalid = [
        frame_with("RGB", 2, 8, PixelRepresentation::Unsigned)?,
        frame_with("RGB", 3, 8, PixelRepresentation::Signed)?,
        frame_with("PALETTE COLOR", 1, 8, PixelRepresentation::Signed)?,
        frame_with("MONOCHROME2", 2, 8, PixelRepresentation::Unsigned)?,
        frame_with("YBR_FULL_422", 3, 8, PixelRepresentation::Unsigned)?,
    ];
    for desc in invalid {
        assert_atomic_error(&[], &desc, CodecError::UnsupportedPixelFormat);
    }
    Ok(())
}

fn frame_with(
    photometric_interpretation: &str,
    samples_per_pixel: u16,
    bits_allocated: u16,
    pixel_representation: PixelRepresentation,
) -> Result<FrameDesc, Box<dyn Error>> {
    Ok(FrameDesc::new(FrameDescInput {
        rows: 1,
        columns: 2,
        samples_per_pixel,
        bits_allocated,
        bits_stored: bits_allocated,
        high_bit: bits_allocated - 1,
        pixel_representation,
        photometric_interpretation: photometric_interpretation.to_owned(),
        pixel_data_vr: PixelDataVr::Ob,
    })?)
}

#[test]
fn malformed_headers_and_runs_leave_output_unchanged() -> TestResult {
    let desc = frame(2, 4, 1, 16)?;
    let valid = [3, 1, 3, 5, 7, 0xfd, 9, 0];

    assert_atomic_error(&[0; 63], &desc, CodecError::InvalidCodestream);

    let mut wrong_count = encoded_frame(&[&valid, &valid]);
    assert!(set_u32_le(&mut wrong_count, 0, 1));
    assert_atomic_error(&wrong_count, &desc, CodecError::FrameMismatch);

    let mut unused_offset = encoded_frame(&[&valid, &valid]);
    assert!(set_u32_le(&mut unused_offset, 12, 80));
    assert_atomic_error(&unused_offset, &desc, CodecError::InvalidCodestream);

    let mut wrong_first_offset = encoded_frame(&[&valid, &valid]);
    assert!(set_u32_le(&mut wrong_first_offset, 4, 66));
    assert_atomic_error(&wrong_first_offset, &desc, CodecError::InvalidCodestream);

    let mut non_monotonic = encoded_frame(&[&valid, &valid]);
    assert!(set_u32_le(&mut non_monotonic, 8, 62));
    assert_atomic_error(&non_monotonic, &desc, CodecError::InvalidCodestream);

    let mut odd_offset = encoded_frame(&[&valid, &valid]);
    assert!(set_u32_le(&mut odd_offset, 8, 71));
    assert_atomic_error(&odd_offset, &desc, CodecError::InvalidCodestream);

    let truncated_literal = [3, 1, 3];
    assert_atomic_error(
        &encoded_frame(&[&truncated_literal, &valid]),
        &desc,
        CodecError::InvalidCodestream,
    );

    let truncated_repeat = [0xfd];
    assert_atomic_error(
        &encoded_frame(&[&truncated_repeat, &valid]),
        &desc,
        CodecError::InvalidCodestream,
    );

    let source = encoded_frame(&[&valid, &valid]);
    let mut short_out = [0xa5; 15];
    assert_eq!(
        RleDecoder.decode(&source, &desc, &mut short_out),
        Err(CodecError::OutputLength {
            expected: 16,
            actual: 15,
        })
    );
    assert!(short_out.iter().all(|byte| *byte == 0xa5));
    Ok(())
}
