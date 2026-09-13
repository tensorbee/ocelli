//! Native Pixel Data fixtures derived from DICOM PS3.5 Annex A and Annex D.

use std::{error::Error, sync::Arc};

use ocelli_codec::{
    Capability, CodecError, DecodeSampleLayout, Decoder, FrameDesc, FrameDescInput, PixelDataVr,
    PixelRepresentation, RawDecoder, Registry, register_native_and_rle_decoders,
};

type TestResult = Result<(), Box<dyn Error>>;

const IMPLICIT_VR_LE: &str = "1.2.840.10008.1.2";
const EXPLICIT_VR_LE: &str = "1.2.840.10008.1.2.1";
const DEFLATED_EXPLICIT_VR_LE: &str = "1.2.840.10008.1.2.1.99";
const EXPLICIT_VR_BE: &str = "1.2.840.10008.1.2.2";
const RLE_LOSSLESS: &str = "1.2.840.10008.1.2.5";

fn frame(
    columns: u16,
    bits_allocated: u16,
    bits_stored: u16,
    pixel_data_vr: PixelDataVr,
) -> Result<FrameDesc, Box<dyn Error>> {
    Ok(FrameDesc::new(FrameDescInput {
        rows: 1,
        columns,
        samples_per_pixel: 1,
        bits_allocated,
        bits_stored,
        high_bit: bits_stored - 1,
        pixel_representation: PixelRepresentation::Signed,
        photometric_interpretation: "MONOCHROME2".to_owned(),
        pixel_data_vr,
    })?)
}

#[test]
fn sixteen_bit_endian_routes_produce_identical_little_endian_containers() -> TestResult {
    // PS3.5 Annex A and Annex D. The stored 12-bit values are -2048 and 2047.
    // Their container words are 0xF800 and 0x07FF. Explicit Big Endian OW
    // transmits each word most-significant byte first, while canonical output
    // is the same little-endian bytes as both little-endian transfer syntaxes.
    let desc = frame(2, 16, 12, PixelDataVr::Ow)?;
    let expected = [0x00, 0xf8, 0xff, 0x07];
    let mut little = [0xa5; 4];
    let mut big = [0xa5; 4];

    RawDecoder::explicit_vr_little_endian().decode(&expected, &desc, &mut little)?;
    RawDecoder::big_endian().decode(&[0xf8, 0x00, 0x07, 0xff], &desc, &mut big)?;

    assert_eq!(little, expected);
    assert_eq!(big, expected);
    Ok(())
}

#[test]
fn eight_bit_big_endian_ob_and_ow_have_distinct_physical_order() -> TestResult {
    // PS3.5 Annex D Figure D-7 and D-8. OB is byte-order insensitive. OW is
    // a stream of 16-bit words, so the bytes for the first two 8-bit pixels
    // are reversed on the big-endian wire. The third pixel shares its final
    // word with the insignificant unused byte of the final OW word.
    let ob_desc = frame(3, 8, 8, PixelDataVr::Ob)?;
    let ow_desc = frame(3, 8, 8, PixelDataVr::Ow)?;
    let mut ob = [0xa5; 3];
    let mut ow = [0xa5; 3];

    RawDecoder::big_endian()
        .index_native_value(&[0x11, 0x22, 0x33, 0x00], 1, &ob_desc)?
        .decode_frame(0, &mut ob)?;
    RawDecoder::big_endian()
        .index_native_value(&[0x22, 0x11, 0x00, 0x33], 1, &ow_desc)?
        .decode_frame(0, &mut ow)?;

    assert_eq!(ob, [0x11, 0x22, 0x33]);
    assert_eq!(ow, [0x11, 0x22, 0x33]);
    Ok(())
}

#[test]
fn thirty_two_bit_ow_swaps_physical_words_not_whole_samples() -> TestResult {
    // Pixel Data remains OW for a 32-bit integer cell. Annex D first packs
    // the low 16-bit word, then the high word. Big Endian changes byte order
    // within each OW word, giving 33 44 11 22 for sample 0x11223344.
    let desc = frame(1, 32, 32, PixelDataVr::Ow)?;
    let mut out = [0xa5; 4];
    RawDecoder::big_endian().decode(&[0x33, 0x44, 0x11, 0x22], &desc, &mut out)?;
    assert_eq!(out, [0x44, 0x33, 0x22, 0x11]);
    Ok(())
}

#[test]
fn logical_frame_decode_does_not_guess_whole_value_padding() -> TestResult {
    let odd = frame(3, 8, 8, PixelDataVr::Ob)?;
    let even = frame(2, 8, 8, PixelDataVr::Ob)?;
    let decoder = RawDecoder::explicit_vr_little_endian();

    let mut out = [0xa5; 3];
    decoder.decode(&[1, 2, 3], &odd, &mut out)?;
    assert_eq!(out, [1, 2, 3]);

    let mut unchanged = [0xa5; 3];
    assert_eq!(
        decoder.decode(&[1, 2, 3, 0], &odd, &mut unchanged),
        Err(CodecError::TrailingData)
    );
    assert_eq!(unchanged, [0xa5; 3]);

    let mut unchanged = [0xa5; 3];
    assert_eq!(
        decoder.decode(&[1, 2], &odd, &mut unchanged),
        Err(CodecError::InvalidCodestream)
    );
    assert_eq!(unchanged, [0xa5; 3]);

    let mut unchanged = [0xa5; 2];
    assert_eq!(
        decoder.decode(&[1, 2, 0], &even, &mut unchanged),
        Err(CodecError::TrailingData)
    );
    assert_eq!(unchanged, [0xa5; 2]);

    let invalid_ob = frame(1, 16, 16, PixelDataVr::Ob)?;
    let mut unchanged = [0xa5; 2];
    assert_eq!(
        decoder.decode(&[1, 2], &invalid_ob, &mut unchanged),
        Err(CodecError::UnsupportedPixelFormat)
    );
    assert_eq!(unchanged, [0xa5; 2]);

    Ok(())
}

#[test]
fn native_value_index_extracts_first_and_mid_byte_one_bit_frames() -> TestResult {
    // PS3.5 8.1.1 and 8.2. Two five-bit Frames are concatenated without
    // padding. Frame one starts at bit five of the first byte. Pixel Cells are
    // packed least-significant bit first, so the two repacked frames are 0x0d
    // and 0x16 respectively.
    let desc = frame(5, 1, 1, PixelDataVr::Ob)?;
    let decoder = RawDecoder::explicit_vr_little_endian();
    let index = decoder.index_native_value(&[0xcd, 0x02], 2, &desc)?;
    let mut first = [0xa5; 1];
    let mut second = [0xa5; 1];

    index.decode_frame(0, &mut first)?;
    index.decode_frame(1, &mut second)?;

    assert_eq!(first, [0x0d]);
    assert_eq!(second, [0x16]);
    Ok(())
}

#[test]
fn native_value_index_owns_ob_padding_and_frame_bounds_atomically() -> TestResult {
    let odd_frame = frame(3, 8, 8, PixelDataVr::Ob)?;
    let decoder = RawDecoder::explicit_vr_little_endian();
    assert_eq!(
        decoder.index_native_value(&[], 0, &odd_frame),
        Err(CodecError::FrameMismatch)
    );
    let index = decoder.index_native_value(&[1, 2, 3, 0], 1, &odd_frame)?;
    let mut decoded = [0xa5; 3];
    index.decode_frame(0, &mut decoded)?;
    assert_eq!(decoded, [1, 2, 3]);

    assert_eq!(
        decoder.index_native_value(&[1, 2, 3], 1, &odd_frame),
        Err(CodecError::InvalidCodestream)
    );
    assert_eq!(
        decoder.index_native_value(&[1, 2, 3, 7], 1, &odd_frame),
        Err(CodecError::TrailingData)
    );

    let two_frames = decoder.index_native_value(&[1, 2, 3, 4, 5, 6], 2, &odd_frame)?;
    assert_eq!(
        decoder.index_native_value(&[1, 2, 3, 4, 5, 6, 0, 0], 2, &odd_frame),
        Err(CodecError::TrailingData)
    );
    let mut unchanged = [0xa5; 3];
    assert_eq!(
        two_frames.decode_frame(2, &mut unchanged),
        Err(CodecError::FrameMismatch)
    );
    assert_eq!(unchanged, [0xa5; 3]);
    let mut wrong_length = [0xa5; 2];
    assert_eq!(
        two_frames.decode_frame(0, &mut wrong_length),
        Err(CodecError::OutputLength {
            expected: 3,
            actual: 2,
        })
    );
    assert_eq!(wrong_length, [0xa5; 2]);
    Ok(())
}

#[test]
fn ow_value_index_ignores_final_bits_and_uses_global_big_endian_words() -> TestResult {
    let desc = frame(3, 8, 8, PixelDataVr::Ow)?;
    let little = RawDecoder::explicit_vr_little_endian();
    let little_index = little.index_native_value(&[1, 2, 3, 0x7e], 1, &desc)?;
    let mut out = [0xa5; 3];
    little_index.decode_frame(0, &mut out)?;
    assert_eq!(out, [1, 2, 3]);

    // The complete canonical stream is 01 02 03 04 05 06. Big-endian OW
    // orders each Value-level word as 02 01, 04 03, 06 05. The frame boundary
    // after byte three falls inside the second word.
    let big = RawDecoder::big_endian();
    let big_index = big.index_native_value(&[2, 1, 4, 3, 6, 5], 2, &desc)?;
    let mut first = [0xa5; 3];
    let mut second = [0xa5; 3];
    big_index.decode_frame(0, &mut first)?;
    big_index.decode_frame(1, &mut second)?;
    assert_eq!(first, [1, 2, 3]);
    assert_eq!(second, [4, 5, 6]);

    let big_unused = big.index_native_value(&[2, 1, 0x7e, 3], 1, &desc)?;
    out.fill(0xa5);
    big_unused.decode_frame(0, &mut out)?;
    assert_eq!(out, [1, 2, 3]);
    Ok(())
}

#[test]
fn direct_registered_big_endian_ow_refuses_an_odd_logical_slice_atomically() -> TestResult {
    // The canonical two-frame Value is 01 02 03 04 05 06. Its physical
    // big-endian OW bytes are 02 01 04 03 06 05. The first three physical
    // bytes are not a complete representation of frame zero because its final
    // canonical byte appears after frame one's first canonical byte.
    let desc = frame(3, 8, 8, PixelDataVr::Ow)?;
    let value = [2, 1, 4, 3, 6, 5];
    let mut registry = Registry::new();
    register_native_and_rle_decoders(&mut registry)?;
    let mut unchanged = [0xa5; 3];

    assert_eq!(
        registry.decode(EXPLICIT_VR_BE, &value[..3], &desc, &mut unchanged),
        Err(CodecError::InvalidCodestream)
    );
    assert_eq!(unchanged, [0xa5; 3]);

    let index = RawDecoder::big_endian().index_native_value(&value, 2, &desc)?;
    let mut first = [0xa5; 3];
    index.decode_frame(0, &mut first)?;
    assert_eq!(first, [1, 2, 3]);
    Ok(())
}

#[test]
fn implicit_vr_requires_ow_while_explicit_little_endian_accepts_ob() -> TestResult {
    let desc = frame(2, 8, 8, PixelDataVr::Ob)?;
    let mut registry = Registry::new();
    register_native_and_rle_decoders(&mut registry)?;

    let mut implicit = [0xa5; 2];
    assert_eq!(
        registry.decode(IMPLICIT_VR_LE, &[1, 2], &desc, &mut implicit),
        Err(CodecError::UnsupportedPixelFormat)
    );
    assert_eq!(implicit, [0xa5; 2]);

    let mut explicit = [0xa5; 2];
    registry.decode(EXPLICIT_VR_LE, &[1, 2], &desc, &mut explicit)?;
    assert_eq!(explicit, [1, 2]);
    Ok(())
}

#[test]
fn native_registration_preserves_layout_and_leaves_deflate_at_ingest() -> TestResult {
    let mut registry = Registry::new();
    register_native_and_rle_decoders(&mut registry)?;
    for uid in [IMPLICIT_VR_LE, EXPLICIT_VR_LE, EXPLICIT_VR_BE, RLE_LOSSLESS] {
        assert_eq!(registry.capability(uid), Capability::Available);
    }
    assert_eq!(
        registry.capability(DEFLATED_EXPLICIT_VR_LE),
        Capability::KnownUnavailable
    );
    assert_eq!(
        registry.decode_sample_layout(EXPLICIT_VR_LE, &frame(1, 8, 8, PixelDataVr::Ob)?)?,
        DecodeSampleLayout::Preserved
    );
    Ok(())
}

#[test]
fn native_and_rle_registration_preflights_every_uid_before_insertion() -> TestResult {
    let mut registry = Registry::new();
    registry.register(Arc::new(RawDecoder::big_endian()))?;

    assert_eq!(
        register_native_and_rle_decoders(&mut registry),
        Err(ocelli_codec::RegistryError::AlreadyRegistered {
            uid: EXPLICIT_VR_BE,
        })
    );
    assert_eq!(registry.capability(EXPLICIT_VR_BE), Capability::Available);
    for uid in [IMPLICIT_VR_LE, EXPLICIT_VR_LE, RLE_LOSSLESS] {
        assert_eq!(registry.capability(uid), Capability::KnownUnavailable);
    }
    Ok(())
}
