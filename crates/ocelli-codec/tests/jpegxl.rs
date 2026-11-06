//! JPEG XL conformance fixtures for F-229.
//!
//! Built by `tests/fixtures/generate_jpegxl.py`. Lossless rows are compared
//! with the synthetic references, never with a decode of the same file.

use std::error::Error;

use ocelli_codec::{
    Capability, CodecError, DecodePhotometricInterpretation, DecodeSampleLayout, Decoder,
    FrameDesc, FrameDescInput, JpegXlDecoder, PixelDataVr, PixelRepresentation, Registry,
    quantise_lossy_sample, register_jpegxl_decoders,
};

type TestResult = Result<(), Box<dyn Error>>;

const ROWS: u16 = 64;
const COLUMNS: u16 = 96;

/// imagecodecs `jpegxl_decode` of `jpegxl_lossy_12.jxl` against the 12-bit ramp.
/// The acceptance bound is one and a half times this error, which is 27.
const IMAGECODECS_LOSSY_ERROR: u32 = 18;

#[test]
fn jpegxl_lossless_16_matches_the_reference() -> TestResult {
    // PS3.5 8.2.15. The 64 by 96 ramp is the same bytes as
    // `jpegls_corpus_reference_u16le.raw`, which is not a JPEG XL decode.
    let reference = include_bytes!("fixtures/jpegls_corpus_reference_u16le.raw");
    let desc = mono(ROWS, COLUMNS, 16, 16)?;
    let mut out = vec![0xa5; desc.output_len()];
    JpegXlDecoder::lossless().decode(
        include_bytes!("fixtures/jpegxl_lossless_16.jxl"),
        &desc,
        &mut out,
    )?;
    assert_eq!(out.as_slice(), reference.as_slice());
    assert_eq!(
        JpegXlDecoder::lossless().decode_photometric_interpretation(&desc),
        DecodePhotometricInterpretation::Preserved
    );
    assert_eq!(
        JpegXlDecoder::lossless().decode_sample_layout(&desc),
        DecodeSampleLayout::Interleaved
    );
    Ok(())
}

#[test]
fn jpegxl_lossless_12_keeps_stored_values() -> TestResult {
    // PS3.5 8.2.15. Bits Stored is 12, so stored value 1 stays 1.
    // `1 * 65535 / 4095` is 16.0037, which is the rescale this test rejects.
    // The reference's first six stored values are 0, 0, 1, 1, 2, 3 and its
    // maximum is 4095.
    let reference = include_bytes!("fixtures/jpegxl_reference_mono12_u16le.raw");
    let desc = mono(ROWS, COLUMNS, 16, 12)?;
    let mut out = vec![0xa5; desc.output_len()];
    JpegXlDecoder::lossless().decode(
        include_bytes!("fixtures/jpegxl_lossless_12.jxl"),
        &desc,
        &mut out,
    )?;
    assert_eq!(read_le_u16(&out, 2)?, 1);
    assert_eq!(max_stored(&out)?, 4095);
    assert_eq!(out.as_slice(), reference.as_slice());
    Ok(())
}

#[test]
fn jpegxl_lossless_rgb8_matches_the_reference() -> TestResult {
    // PS3.5 8.2.15. Three channels are interleaved RGB in channel order.
    let reference = include_bytes!("fixtures/jpegxl_reference_rgb8.raw");
    let desc = colour(ROWS, COLUMNS, 8, 8, "RGB")?;
    let mut out = vec![0xa5; desc.output_len()];
    JpegXlDecoder::lossless().decode(
        include_bytes!("fixtures/jpegxl_lossless_rgb8.jxl"),
        &desc,
        &mut out,
    )?;
    assert_eq!(out.as_slice(), reference.as_slice());
    assert_eq!(
        JpegXlDecoder::lossless().decode_photometric_interpretation(&desc),
        DecodePhotometricInterpretation::Rgb
    );
    assert_eq!(
        JpegXlDecoder::lossless().decode_sample_layout(&desc),
        DecodeSampleLayout::Interleaved
    );
    Ok(())
}

const JPEGXL_RECOMPRESSION: &str = "1.2.840.10008.1.2.4.111";

#[test]
fn jpegxl_recompression_is_unavailable() -> TestResult {
    // AC7b. PS3.5 A.4.12 names `.111`. `jxl-jbr` 0.2.1 `src/reconstruct.rs:457`
    // panics with `expect("is_last not found")` in the DHT branch, so the UID
    // stays known and no decoder is registered. Intact bytes, the bit flip
    // that used to reach that expect, and a one-byte prefix all return
    // `KnownUnavailable` and leave `out` untouched. None of them is passed
    // to `jxl-oxide`.
    let mut registry = Registry::new();
    register_jpegxl_decoders(&mut registry)?;
    assert_eq!(
        registry.capability(JPEGXL_RECOMPRESSION),
        Capability::KnownUnavailable
    );
    let desc = colour(ROWS, COLUMNS, 8, 8, "YBR_FULL_422")?;
    let intact = include_bytes!("fixtures/jpegxl_recompression.jxl");
    assert_unavailable(&registry, intact, &desc)?;
    let mut flipped = intact.to_vec();
    let byte = flipped
        .get_mut(100)
        .ok_or("recompression fixture shorter than 101")?;
    *byte ^= 1 << 7;
    assert_unavailable(&registry, &flipped, &desc)?;
    let prefix = intact.get(..1).ok_or("empty recompression fixture")?;
    assert_unavailable(&registry, prefix, &desc)
}

#[test]
fn fixture_lossy_quantiser() -> TestResult {
    // PS3.5 8.2.15. The codestream's bit depth is the stored precision.
    // `q = floor(v * (2^BitsStored - 1) + 0.5)`, then clamped to
    // `[0, 2^BitsStored - 1]`. The product is taken in f64 from the f32.
    //
    // Bits Stored 8, maximum 255:
    // 0.0 * 255 + 0.5 = 0.5, floor 0
    // 1.0 * 255 + 0.5 = 255.5, floor 255
    // 0.5 * 255 + 0.5 = 128.0, floor 128
    // f32(0.498) * 255 + 0.5 = 127.48999, floor 127
    // -0.25 * 255 + 0.5 = -63.25, clamp 0
    // 1.25 * 255 + 0.5 = 319.25, clamp 255
    //
    // Bits Stored 12, maximum 4095:
    // 0.5 * 4095 + 0.5 = 2048.0, floor 2048
    // f32(1/4095) * 4095 + 0.5 = 1.50000006, floor 1
    assert_eq!(quantise_lossy_sample(0.0_f32, 8)?, 0);
    assert_eq!(quantise_lossy_sample(1.0_f32, 8)?, 255);
    assert_eq!(quantise_lossy_sample(0.5_f32, 8)?, 128);
    assert_eq!(quantise_lossy_sample(0.498_f32, 8)?, 127);
    assert_eq!(quantise_lossy_sample(-0.25_f32, 8)?, 0);
    assert_eq!(quantise_lossy_sample(1.25_f32, 8)?, 255);
    assert_eq!(quantise_lossy_sample(0.5_f32, 12)?, 2048);
    assert_eq!(quantise_lossy_sample(1.0_f32 / 4095.0_f32, 12)?, 1);
    assert_eq!(
        quantise_lossy_sample(f32::NAN, 8),
        Err(CodecError::DecoderFailure)
    );
    assert_eq!(
        quantise_lossy_sample(f32::INFINITY, 8),
        Err(CodecError::DecoderFailure)
    );
    assert_eq!(
        quantise_lossy_sample(f32::NEG_INFINITY, 8),
        Err(CodecError::DecoderFailure)
    );
    assert_eq!(
        quantise_lossy_sample(0.5_f32, 0),
        Err(CodecError::UnsupportedPixelFormat)
    );
    assert_eq!(
        quantise_lossy_sample(0.5_f32, 17),
        Err(CodecError::UnsupportedPixelFormat)
    );
    Ok(())
}

#[test]
fn jpegxl_lossy_stays_within_the_recorded_bound() -> TestResult {
    // `.112` of the 12-bit ramp. The bound is 27 stored units and is not raised.
    let reference = include_bytes!("fixtures/jpegxl_reference_mono12_u16le.raw");
    let desc = mono(ROWS, COLUMNS, 16, 12)?;
    let mut out = vec![0xa5; desc.output_len()];
    JpegXlDecoder::general().decode(
        include_bytes!("fixtures/jpegxl_lossy_12.jxl"),
        &desc,
        &mut out,
    )?;
    let difference = max_abs_stored(&out, reference)?;
    let bound = IMAGECODECS_LOSSY_ERROR
        .checked_mul(3)
        .ok_or("bound overflow")?;
    let doubled = difference.checked_mul(2).ok_or("difference overflow")?;
    assert!(
        difference > 0,
        "the lossy fixture decoded with no error, so it is not the lossy file"
    );
    assert!(
        doubled <= bound,
        "absolute error {difference} exceeds the bound of 27"
    );
    Ok(())
}

#[test]
fn lossless_only_refuses_vardct_and_xyb() -> TestResult {
    // PS3.5 A.4.12. `.110` is lossless. The AC10 fixture is VarDCT and
    // `xyb_encoded`, so either signal is `FrameMismatch`.
    let desc = mono(ROWS, COLUMNS, 16, 12)?;
    assert_refused(
        &JpegXlDecoder::lossless(),
        include_bytes!("fixtures/jpegxl_lossy_12.jxl"),
        &desc,
        CodecError::FrameMismatch,
    )?;

    // Each signal alone. The recompression file is VarDCT without XYB, so
    // only the frame encoding refuses it. The modular file is lossy modular
    // with XYB, so only the image header refuses it. Both decode under `.112`,
    // which shows the refusal is the lossless rule and not a broken file.
    let ybr = colour(ROWS, COLUMNS, 8, 8, "YBR_FULL_422")?;
    let vardct_only = include_bytes!("fixtures/jpegxl_recompression.jxl");
    assert_refused(
        &JpegXlDecoder::lossless(),
        vardct_only,
        &ybr,
        CodecError::FrameMismatch,
    )?;
    let mut out = vec![0xa5; ybr.output_len()];
    JpegXlDecoder::general().decode(vardct_only, &ybr, &mut out)?;

    let rgb_8x8 = colour(8, 8, 8, 8, "RGB")?;
    let xyb_only = include_bytes!("fixtures/jpegxl_modular_xyb.jxl");
    assert_refused(
        &JpegXlDecoder::lossless(),
        xyb_only,
        &rgb_8x8,
        CodecError::FrameMismatch,
    )?;
    let mut out = vec![0xa5; rgb_8x8.output_len()];
    JpegXlDecoder::general().decode(xyb_only, &rgb_8x8, &mut out)?;
    Ok(())
}

#[test]
fn descriptor_mismatches_refuse() -> TestResult {
    let lossless16 = include_bytes!("fixtures/jpegxl_lossless_16.jxl");
    let signed = frame(
        ROWS,
        COLUMNS,
        1,
        16,
        16,
        PixelRepresentation::Signed,
        "MONOCHROME2",
    )?;
    assert_refused(
        &JpegXlDecoder::lossless(),
        lossless16,
        &signed,
        CodecError::UnsupportedPixelFormat,
    )?;

    let one_bit = frame(
        ROWS,
        COLUMNS,
        1,
        1,
        1,
        PixelRepresentation::Unsigned,
        "MONOCHROME2",
    )?;
    assert_refused(
        &JpegXlDecoder::lossless(),
        lossless16,
        &one_bit,
        CodecError::UnsupportedPixelFormat,
    )?;

    let stored_12 = mono(ROWS, COLUMNS, 16, 12)?;
    assert_refused(
        &JpegXlDecoder::lossless(),
        lossless16,
        &stored_12,
        CodecError::FrameMismatch,
    )?;

    let short_rows = mono(32, COLUMNS, 16, 16)?;
    assert_refused(
        &JpegXlDecoder::lossless(),
        lossless16,
        &short_rows,
        CodecError::FrameMismatch,
    )?;

    let colour_header = colour(ROWS, COLUMNS, 16, 16, "RGB")?;
    assert_refused(
        &JpegXlDecoder::lossless(),
        lossless16,
        &colour_header,
        CodecError::FrameMismatch,
    )
}

#[test]
fn unsupported_codestreams_report_unavailable() -> TestResult {
    // Orientation, extra channels, ICC and PQ or HLG are unavailable.
    // Two keyframes disagree with A.4.12's one frame. A float sample is
    // not an integer stored value. Bits Allocated stays 8 here so the
    // float refusal is the codestream, not the descriptor.
    let gray_8x16 = mono(8, 16, 8, 8)?;
    assert_refused(
        &JpegXlDecoder::general(),
        include_bytes!("fixtures/jpegxl_orient6.jxl"),
        &gray_8x16,
        CodecError::UnsupportedPixelFormat,
    )?;
    assert_refused(
        &JpegXlDecoder::general(),
        include_bytes!("fixtures/jpegxl_two_frames.jxl"),
        &gray_8x16,
        CodecError::FrameMismatch,
    )?;
    assert_refused(
        &JpegXlDecoder::general(),
        include_bytes!("fixtures/jpegxl_pq.jxl"),
        &gray_8x16,
        CodecError::UnsupportedPixelFormat,
    )?;

    let gray_8x8 = mono(8, 8, 8, 8)?;
    assert_refused(
        &JpegXlDecoder::general(),
        include_bytes!("fixtures/jpegxl_alpha.jxl"),
        &gray_8x8,
        CodecError::UnsupportedPixelFormat,
    )?;

    let rgb_8x8 = colour(8, 8, 8, 8, "RGB")?;
    assert_refused(
        &JpegXlDecoder::general(),
        include_bytes!("fixtures/jpegxl_icc.jxl"),
        &rgb_8x8,
        CodecError::UnsupportedPixelFormat,
    )?;

    let gray_4x4 = mono(4, 4, 8, 8)?;
    assert_refused(
        &JpegXlDecoder::general(),
        include_bytes!("fixtures/jpegxl_float.jxl"),
        &gray_4x4,
        CodecError::UnsupportedPixelFormat,
    )
}

/// An LSB-first bit writer, matching `jxl-bitstream`'s bit order.
struct BitWriter {
    bytes: Vec<u8>,
    bit_len: usize,
}

impl BitWriter {
    fn new() -> Self {
        Self {
            bytes: Vec::new(),
            bit_len: 0,
        }
    }

    fn push_bit(&mut self, bit: bool) {
        if self.bit_len.is_multiple_of(8) {
            self.bytes.push(0);
        }
        if bit && let Some(last) = self.bytes.last_mut() {
            *last |= 1 << (self.bit_len % 8);
        }
        self.bit_len += 1;
    }

    fn push_bits(&mut self, value: u32, count: u32) {
        for index in 0..count {
            self.push_bit((value >> index) & 1 == 1);
        }
    }

    /// Appends `count` bits of `source` starting at bit `from`.
    fn append_bits_from(
        &mut self,
        source: &[u8],
        from: usize,
        count: usize,
    ) -> Result<(), Box<dyn Error>> {
        for index in from..from + count {
            let byte = source
                .get(index / 8)
                .copied()
                .ok_or_else(|| format!("bit index {index} outside the source"))?;
            self.push_bit((byte >> (index % 8)) & 1 == 1);
        }
        Ok(())
    }

    fn pad_to_byte(&mut self) {
        while !self.bit_len.is_multiple_of(8) {
            self.push_bit(false);
        }
    }

    fn finish(mut self) -> Vec<u8> {
        self.pad_to_byte();
        self.bytes
    }
}

fn fixture_bit(bytes: &[u8], index: usize) -> Result<bool, Box<dyn Error>> {
    let byte = bytes
        .get(index / 8)
        .copied()
        .ok_or_else(|| format!("bit index {index} outside the fixture"))?;
    Ok((byte >> (index % 8)) & 1 == 1)
}

/// Writes one enum value below 18 in the `U32(0, 1, 2 + U(4), 18 + U(6))` coding
/// that `jxl-bitstream`'s `read_enum` reads.
fn push_small_enum(writer: &mut BitWriter, value: u32) {
    match value {
        0 => writer.push_bits(0, 2),
        1 => writer.push_bits(1, 2),
        2..=17 => {
            writer.push_bits(2, 2);
            writer.push_bits(value - 2, 4);
        }
        _ => unreachable!("value needs the long enum coding"),
    }
}

/// The colour encoding an in-test rewrite of `jpegxl_lossless_rgb8.jxl` declares.
enum DeclaredColourEncoding {
    /// The fixture's own encoding, a single `all_default` bit.
    AllDefault,
    Xyb,
    Unknown,
}

/// Rewrites the declared colour encoding of `jpegxl_lossless_rgb8.jxl`.
///
/// The fixture's image metadata is present but its extra fields are absent, so
/// the colour encoding is one `all_default` bit at codestream bit 34, the
/// metadata ends at bit 38, and the frame starts byte aligned at byte 5. The
/// rewriter asserts that header shape first, so a changed fixture fails here
/// instead of producing a silently corrupt stream. The metadata tail is copied
/// from the fixture, the header is zero padded back to a byte boundary, and
/// the frame is copied from byte 5, which is where the original frame starts.
fn rewrite_colour_encoding(
    fixture: &[u8],
    declared: DeclaredColourEncoding,
) -> Result<Vec<u8>, Box<dyn Error>> {
    // Codestream bits 25 to 33: metadata all_default 0, extra_fields 0, float
    // sample 0 with the U32 selector 00 for 8 bits, modular 16-bit buffers on,
    // the num_extra selector 00, and no XYB. Bits 38 and 39 are the header's
    // own zero padding before the byte aligned frame.
    let expected_prefix = [false, false, false, false, false, true, false, false, false];
    for (index, bit) in expected_prefix.iter().enumerate() {
        assert_eq!(
            fixture_bit(fixture, 25 + index)?,
            *bit,
            "fixture header changed shape at bit {}",
            25 + index
        );
    }
    assert!(
        fixture_bit(fixture, 34)?,
        "bit 34 is not the colour encoding all_default bit"
    );
    assert!(
        !fixture_bit(fixture, 38)? && !fixture_bit(fixture, 39)?,
        "bits 38 and 39 are not zero header padding"
    );

    let mut writer = BitWriter::new();
    writer.append_bits_from(fixture, 0, 34)?;
    match declared {
        DeclaredColourEncoding::AllDefault => writer.push_bit(true),
        DeclaredColourEncoding::Xyb => {
            // all_default 0, want_icc 0, colour space XYB, gamma absent with
            // the sRGB transfer function, relative intent. PS3.3 C.7.6.3.1.2
            // gives XYB no conversion matrix.
            writer.push_bit(false);
            writer.push_bit(false);
            push_small_enum(&mut writer, 2);
            writer.push_bit(false);
            push_small_enum(&mut writer, 13);
            push_small_enum(&mut writer, 1);
        }
        DeclaredColourEncoding::Unknown => {
            writer.push_bit(false);
            writer.push_bit(false);
            push_small_enum(&mut writer, 3);
            push_small_enum(&mut writer, 1);
            push_small_enum(&mut writer, 1);
            writer.push_bit(false);
            push_small_enum(&mut writer, 13);
            push_small_enum(&mut writer, 1);
        }
    }
    // The metadata tail: extensions and default_m.
    writer.append_bits_from(fixture, 35, 3)?;
    writer.pad_to_byte();
    writer.append_bits_from(fixture, 40, fixture.len() * 8 - 40)?;
    Ok(writer.finish())
}

#[test]
fn codestream_colour_space_other_than_rgb_or_grey_refuses() -> TestResult {
    // PS3.5 8.2.15 and PS3.3 C.7.6.3.1.2. The codestream's own colour encoding
    // is what the decoder must produce, and XYB has no matrix to get there, so
    // anything but RGB or greyscale refuses before a sample is written. The
    // rewriter is proved against the original bytes first.
    let fixture = include_bytes!("fixtures/jpegxl_lossless_rgb8.jxl");
    let reproduced = rewrite_colour_encoding(fixture, DeclaredColourEncoding::AllDefault)?;
    assert_eq!(
        reproduced, fixture,
        "the all_default rewrite is not byte identical to the fixture"
    );
    let xyb = rewrite_colour_encoding(fixture, DeclaredColourEncoding::Xyb)?;
    let unknown = rewrite_colour_encoding(fixture, DeclaredColourEncoding::Unknown)?;
    let desc = colour(ROWS, COLUMNS, 8, 8, "RGB")?;
    for (name, src) in [("xyb", &xyb), ("unknown", &unknown)] {
        assert_refused(
            &JpegXlDecoder::lossless(),
            src,
            &desc,
            CodecError::UnsupportedPixelFormat,
        )
        .map_err(|e| format!("{name} under .110: {e}"))?;
        assert_refused(
            &JpegXlDecoder::general(),
            src,
            &desc,
            CodecError::UnsupportedPixelFormat,
        )
        .map_err(|e| format!("{name} under .112: {e}"))?;
    }
    Ok(())
}

#[test]
fn samples_above_declared_bits_stored_refuse() -> TestResult {
    // PS3.3 C.7.6.1.1 and PS3.5 8.2.15. Byte 52 bit 6 is the lowest bit of the
    // codestream's bits per sample, so the flip declares 15 while the modular
    // data stays a 16-bit ramp. Under Bits Stored 15 the range check refuses
    // before any sample is written.
    let mut flipped = include_bytes!("fixtures/jpegxl_lossless_16.jxl").to_vec();
    let byte = flipped.get_mut(52).ok_or("fixture shorter than 53")?;
    *byte ^= 1 << 6;
    let desc = mono(ROWS, COLUMNS, 16, 15)?;
    assert_refused(
        &JpegXlDecoder::lossless(),
        &flipped,
        &desc,
        CodecError::UnsupportedPixelFormat,
    )
}

/// Builds a 26 byte bare codestream whose header declares an encoded ICC
/// profile of `1 << 28` bytes. The 18 entropy decoded bytes name that output
/// size, so an unguarded decode reaches `jxl-color`'s resize before it runs
/// out of input.
fn crafted_oversized_icc_declaration() -> Result<Vec<u8>, Box<dyn Error>> {
    let mut writer = BitWriter::new();
    writer.push_bits(0x0aff, 16);
    writer.push_bit(true);
    writer.push_bits(7, 5);
    writer.push_bits(4, 3);
    writer.push_bit(false);
    writer.push_bit(false);
    writer.push_bit(false);
    writer.push_bits(0, 2);
    writer.push_bit(true);
    writer.push_bits(0, 2);
    writer.push_bit(false);
    writer.push_bit(false);
    writer.push_bit(true);
    writer.push_bits(0, 2);
    writer.push_bits(0, 2);
    writer.push_bit(true);
    writer.push_bits(3, 2);
    writer.push_bits(0, 12);
    for chunk in [0u32, 0, 1] {
        writer.push_bit(true);
        writer.push_bits(chunk, 8);
    }
    writer.push_bit(false);
    writer.push_bit(false);
    writer.push_bit(true);
    writer.push_bits(0, 2);
    writer.push_bit(true);
    writer.push_bits(0, 4);
    writer.push_bit(true);
    writer.push_bits(7, 4);
    writer.push_bits(127, 7);
    writer.push_bits(1, 2);
    writer.push_bits(3, 2);
    for sym in [0u32, 1, 2, 8] {
        writer.push_bits(sym, 8);
    }
    writer.push_bit(false);
    for _ in 0..4 {
        writer.push_bits(3, 2);
        writer.push_bits(0, 7);
    }
    for _ in 0..2 {
        writer.push_bits(2, 2);
    }
    for _ in 0..12 {
        writer.push_bits(0, 2);
    }
    Ok(writer.finish())
}

#[test]
fn declared_icc_refuses_unsupported_before_any_parse() -> TestResult {
    // AC13 and the pass 2 lead amendment. A header that declares an embedded
    // ICC profile refuses `UnsupportedPixelFormat` before `jxl-oxide` sees a
    // byte, whatever `enc_size` it declares and whatever follows it. The
    // crafted stream declares `1 << 28` encoded bytes into a 26 byte
    // codestream. Unguarded, `jxl-color` resizes its ICC buffer to that
    // declaration before reading it, a measured peak resident set of
    // 270811136 bytes on 2026-11-06, and then refuses `InvalidCodestream`
    // because the entropy data is zeros. The refusal kind is the detector:
    // removing the pre-check call from `decode_limited` turns this test red,
    // which is review pass 2 D4.
    let crafted = crafted_oversized_icc_declaration()?;
    let desc = colour(ROWS, COLUMNS, 8, 8, "RGB")?;
    for decoder in [&JpegXlDecoder::general(), &JpegXlDecoder::lossless()] {
        assert_refused(decoder, &crafted, &desc, CodecError::UnsupportedPixelFormat)?;
    }

    // The same stream in a jxlc box that declares 1000 bytes more than are
    // present, review pass 2 D2. The box lies about its size. The pre-check
    // reads the header from the bytes that exist and refuses, so `jxl-oxide`
    // never streams the partial box and never reaches the resize. Removing
    // the pre-check call turns this red too, through the same kind change.
    let mut container = Vec::new();
    container.extend_from_slice(b"\x00\x00\x00\x0cJXL \x0d\x0a\x87\x0a");
    let box_len = u32::try_from(crafted.len() + 8 + 1000).map_err(|_| "box length")?;
    container.extend_from_slice(&box_len.to_be_bytes());
    container.extend_from_slice(b"jxlc");
    container.extend_from_slice(&crafted);
    assert_eq!(container.len(), 46, "the review's 46 byte container");
    assert_refused(
        &JpegXlDecoder::general(),
        &container,
        &desc,
        CodecError::UnsupportedPixelFormat,
    )
}

#[test]
fn libjxl_icc_profile_refuses_unavailable() -> TestResult {
    // Review pass 2 D3, AC13. libjxl 0.12.0 embedded the Display P3 profile
    // with a zero filled private tag, so the codestream is 286 bytes and
    // declares 4417 encoded ICC bytes into the 2223 bits that follow the
    // declaration, and `jxl-oxide` reads it and renders. A size comparison
    // refuses such a stream as `InvalidCodestream`, which reports a valid
    // file as corrupt. The colour encoding is an ICC profile, so the refusal
    // is `UnsupportedPixelFormat` under both UIDs. Removing the pre-check
    // keeps this green, because `validate_header` refuses the same kind
    // later. The red conditions are the size comparison of review pass 2 and
    // any pre-check that refuses a different kind.
    let desc = colour(8, 8, 8, 8, "RGB")?;
    for decoder in [&JpegXlDecoder::general(), &JpegXlDecoder::lossless()] {
        assert_refused(
            decoder,
            include_bytes!("fixtures/jpegxl_icc_compressed.jxl"),
            &desc,
            CodecError::UnsupportedPixelFormat,
        )?;
    }
    Ok(())
}

#[test]
fn container_header_cut_short_refuses_before_jxl_oxide() -> TestResult {
    // The pass 2 lead amendment, fail closed. The box declares 1000 bytes
    // more than are present and the codestream stops inside the header, so
    // the pre-check cannot read to the ICC flag. It refuses before
    // `jxl-oxide` sees a byte, with the same `InvalidCodestream` the decode
    // returns for a truncated header, so this test does not detect the
    // pre-check's removal. What it locks is the refusal kind and that a
    // header no parser can read never reaches the dependency. The allocation
    // bound itself is the measured peak resident set recorded in the module
    // documentation of `jpegxl_icc`.
    let crafted = crafted_oversized_icc_declaration()?;
    let mut container = Vec::new();
    container.extend_from_slice(b"\x00\x00\x00\x0cJXL \x0d\x0a\x87\x0a");
    let box_len = u32::try_from(crafted.len() + 8 + 1000).map_err(|_| "box length")?;
    container.extend_from_slice(&box_len.to_be_bytes());
    container.extend_from_slice(b"jxlc");
    let partial = crafted.get(..5).ok_or("crafted shorter than 5")?;
    container.extend_from_slice(partial);
    let desc = colour(ROWS, COLUMNS, 8, 8, "RGB")?;
    assert_refused(
        &JpegXlDecoder::general(),
        &container,
        &desc,
        CodecError::InvalidCodestream,
    )
}

/// The twelve byte signature box of a JPEG XL container.
const CONTAINER_SIGNATURE: &[u8] = b"\x00\x00\x00\x0cJXL \x0d\x0a\x87\x0a";

/// One ISO BMFF style box, a 32 bit size followed by the type and payload.
fn container_box(box_type: &[u8; 4], payload: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
    let length = u32::try_from(payload.len() + 8).map_err(|_| "box length")?;
    let mut out = length.to_be_bytes().to_vec();
    out.extend_from_slice(box_type);
    out.extend_from_slice(payload);
    Ok(out)
}

/// A container of whole boxes after the signature box.
fn container(boxes: &[&[u8]]) -> Result<Vec<u8>, Box<dyn Error>> {
    let mut out = CONTAINER_SIGNATURE.to_vec();
    for box_bytes in boxes {
        out.extend_from_slice(box_bytes);
    }
    Ok(out)
}

/// Wraps a codestream in `jxlp` boxes carrying `chunk` bytes each.
///
/// Indexes count from zero and the last box sets the high flag bit, the
/// container format's own shape.
fn jxlp_container(codestream: &[u8], chunk: usize) -> Result<Vec<u8>, Box<dyn Error>> {
    let count = codestream.chunks(chunk).count();
    let mut boxes: Vec<Vec<u8>> = Vec::new();
    for (index, piece) in codestream.chunks(chunk).enumerate() {
        let last = index + 1 == count;
        let mut raw = u32::try_from(index).map_err(|_| "jxlp index")?;
        if last {
            raw |= 0x8000_0000;
        }
        let mut content = raw.to_be_bytes().to_vec();
        content.extend_from_slice(piece);
        boxes.push(container_box(b"jxlp", &content)?);
    }
    let refs: Vec<&[u8]> = boxes.iter().map(Vec::as_slice).collect();
    container(&refs)
}

#[test]
fn brob_box_beside_a_codestream_is_ignored() -> TestResult {
    // Review pass 3 D2. jxl-oxide 0.12.6 `aux_box.rs:72` inflates a brob box
    // into an unbounded Vec. The review's 1129 byte container, a brob box
    // carrying 841 bytes of Brotli naming 1 GiB of zeros beside the lossless
    // RGB codestream, decoded with a peak resident set of 1095647232 bytes
    // before the extraction. With the codestream extracted it decodes to the
    // reference and that container measured 2818048 bytes, so the payload is
    // never inflated. The committed form makes the brotli invalid: if the box
    // reached the dependency's auxiliary box reader, the feed would refuse
    // and this test would be red.
    let codestream = include_bytes!("fixtures/jpegxl_lossless_rgb8.jxl");
    let container = container(&[
        &container_box(b"brob", b"Exif this is not a brotli stream")?,
        &container_box(b"jxlc", codestream)?,
    ])?;
    let desc = colour(ROWS, COLUMNS, 8, 8, "RGB")?;
    let mut out = vec![0xa5; desc.output_len()];
    JpegXlDecoder::general().decode(&container, &desc, &mut out)?;
    assert_eq!(
        out.as_slice(),
        include_bytes!("fixtures/jpegxl_reference_rgb8.raw")
    );
    Ok(())
}

#[test]
fn jbrd_box_beside_a_codestream_is_ignored() -> TestResult {
    // Review pass 3 D2. jxl-jbr 0.2.1 `lib.rs:59` inflates a jbrd box's data
    // section through the same unbounded writer. A jbrd payload that parses
    // as neither header nor brotli refuses inside the dependency's auxiliary
    // box reader, so a reference match here proves the box was never parsed.
    // The review's bomb, a valid jbrd header naming 1 GiB of zeros, measured
    // 1095254016 bytes of resident set before the extraction while refusing,
    // and 2850816 bytes after it while decoding to the reference.
    let codestream = include_bytes!("fixtures/jpegxl_lossless_rgb8.jxl");
    let container = container(&[
        &container_box(b"jbrd", b"garbage that is neither header nor brotli")?,
        &container_box(b"jxlc", codestream)?,
    ])?;
    let desc = colour(ROWS, COLUMNS, 8, 8, "RGB")?;
    let mut out = vec![0xa5; desc.output_len()];
    JpegXlDecoder::general().decode(&container, &desc, &mut out)?;
    assert_eq!(
        out.as_slice(),
        include_bytes!("fixtures/jpegxl_reference_rgb8.raw")
    );
    Ok(())
}

#[test]
fn containers_that_break_the_extraction_rules_refuse() -> TestResult {
    // Review pass 3 D2. A container with no codestream, two jxlc boxes, a
    // jxlc beside jxlp boxes, jxlp boxes out of order, a jxlp after the final
    // one, and jxlp boxes without the last flag are refused before the
    // decode. The last one would decode if the rule were absent, because the
    // pieces carry a complete codestream, so it detects the rule's removal
    // and not only its kind.
    let codestream = include_bytes!("fixtures/jpegxl_lossless_rgb8.jxl");
    let desc = colour(ROWS, COLUMNS, 8, 8, "RGB")?;

    let no_codestream = container(&[&container_box(b"ftyp", b"\x00\x00\x00\x00jxlc jxlp")?])?;
    assert_refused(
        &JpegXlDecoder::general(),
        &no_codestream,
        &desc,
        CodecError::InvalidCodestream,
    )?;

    let duplicate = container(&[
        &container_box(b"jxlc", codestream)?,
        &container_box(b"jxlc", codestream)?,
    ])?;
    assert_refused(
        &JpegXlDecoder::general(),
        &duplicate,
        &desc,
        CodecError::InvalidCodestream,
    )?;

    let beside_later = container(&[
        &container_box(b"jxlc", codestream)?,
        &container_box(b"jxlp", &[0, 0, 0, 0])?,
    ])?;
    assert_refused(
        &JpegXlDecoder::general(),
        &beside_later,
        &desc,
        CodecError::InvalidCodestream,
    )?;

    let out_of_order = container(&[
        &container_box(b"jxlp", &[0, 0, 0, 0])?,
        &container_box(b"jxlp", &[0, 0, 0, 2])?,
    ])?;
    assert_refused(
        &JpegXlDecoder::general(),
        &out_of_order,
        &desc,
        CodecError::InvalidCodestream,
    )?;

    let after_final = container(&[
        &container_box(b"jxlp", &[0x80, 0, 0, 0])?,
        &container_box(b"jxlp", &[0x80, 0, 0, 1])?,
    ])?;
    assert_refused(
        &JpegXlDecoder::general(),
        &after_final,
        &desc,
        CodecError::InvalidCodestream,
    )?;

    let mut flagged = jxlp_container(codestream, 16)?;
    // The final jxlp box is the last 28 bytes, a size and type, a four byte
    // index and a 16 byte piece. Clear the flag bit at the top of its index.
    let index_at = flagged.len() - 28 + 8;
    let byte = flagged
        .get_mut(index_at)
        .ok_or("container shorter than the final jxlp box")?;
    assert_eq!(*byte & 0x80, 0x80, "the final jxlp index is not flagged");
    *byte &= 0x7f;
    assert_refused(
        &JpegXlDecoder::general(),
        &flagged,
        &desc,
        CodecError::InvalidCodestream,
    )
}

#[test]
fn container_trailing_byte_is_trailing_data() -> TestResult {
    // An ISO BMFF container stops at the box, so a following byte is
    // detectable. The container length is even, so one `0x00` is not the
    // odd-length DICOM pad.
    let desc = mono(8, 16, 8, 8)?;
    let container = include_bytes!("fixtures/jpegxl_container.jxl");
    let mut out = vec![0xa5; desc.output_len()];
    JpegXlDecoder::general().decode(container, &desc, &mut out)?;
    let mut expected = [0u8; 8 * 16];
    for (index, byte) in expected.iter_mut().enumerate() {
        *byte = u8::try_from(index).map_err(|_| "ramp")?;
    }
    assert_eq!(out.as_slice(), expected.as_slice());

    let mut padded = container.to_vec();
    padded.push(0);
    assert_refused(
        &JpegXlDecoder::general(),
        &padded,
        &desc,
        CodecError::TrailingData,
    )
}

/// Appends `value` in the `U64` coding `jxl_bitstream`'s `read_u64` reads.
fn push_u64(writer: &mut BitWriter, value: u64) {
    if value == 0 {
        writer.push_bits(0, 2);
    } else if value <= 16 {
        writer.push_bits(1, 2);
        writer.push_bits(u32::try_from(value - 1).unwrap_or(0), 4);
    } else if value <= 272 {
        writer.push_bits(2, 2);
        writer.push_bits(u32::try_from(value - 17).unwrap_or(0), 8);
    } else {
        writer.push_bits(3, 2);
        writer.push_bits((value & 0xfff) as u32, 12);
        let mut rest = value >> 12;
        let mut shift = 12u32;
        loop {
            if shift == 60 {
                writer.push_bits((rest & 0xf) as u32, 4);
                break;
            }
            let byte = (rest & 0xff) as u32;
            rest >>= 8;
            writer.push_bit(true);
            writer.push_bits(byte, 8);
            shift += 8;
            if rest == 0 {
                writer.push_bit(false);
                break;
            }
        }
    }
}

/// Rewrites the header of `jpegxl_lossless_rgb8.jxl` so that it is longer
/// than the first probe window: `default_m 0`, `cw_mask 7`, and 280 zero
/// upsampling weights. The weights are parsed and then unused, because the
/// frame does not upsample. The rewriter asserts the fixture's header shape
/// first, so a changed fixture fails here instead of producing a silently
/// corrupt stream.
fn rewrite_long_default_m_header(fixture: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
    let expected_prefix = [false, false, false, false, false, true, false, false, false];
    for (index, bit) in expected_prefix.iter().enumerate() {
        assert_eq!(
            fixture_bit(fixture, 25 + index)?,
            *bit,
            "fixture header changed shape at bit {}",
            25 + index
        );
    }
    assert!(
        fixture_bit(fixture, 34)?,
        "bit 34 is not the colour encoding all_default bit"
    );
    assert!(
        !fixture_bit(fixture, 35)? && !fixture_bit(fixture, 36)?,
        "bits 35 and 36 are not the zero extensions selector"
    );
    assert!(
        fixture_bit(fixture, 37)?,
        "bit 37 is not a set default_m bit"
    );
    assert!(
        !fixture_bit(fixture, 38)? && !fixture_bit(fixture, 39)?,
        "bits 38 and 39 are not zero header padding"
    );

    let mut writer = BitWriter::new();
    writer.append_bits_from(fixture, 0, 37)?;
    writer.push_bit(false);
    writer.push_bits(7, 3);
    for _ in 0..(15 + 55 + 210) {
        writer.push_bits(0, 16);
    }
    writer.pad_to_byte();
    writer.append_bits_from(fixture, 40, fixture.len() * 8 - 40)?;
    Ok(writer.finish())
}

#[test]
fn header_longer_than_the_first_probe_window_decodes() -> TestResult {
    // Review pass 3 S1. No fixture has a header longer than the first 64
    // byte probe window, so the window growth that keeps such a header
    // decodable was executed by no test. The rewrite is a valid stream whose
    // header is 566 bytes. It must decode to the reference, bare and split
    // over 16 byte jxlp boxes, so the pre-check's probe windows and
    // read_header's feed windows both grow past the first one. Replacing
    // either growth with a refusal turns this red.
    let fixture = include_bytes!("fixtures/jpegxl_lossless_rgb8.jxl");
    let long = rewrite_long_default_m_header(fixture)?;
    let header_bytes: usize = (37usize + 1 + 3 + 280 * 16).div_ceil(8);
    assert_eq!(
        long.len(),
        header_bytes + fixture.len() - 5,
        "the rewritten header is not the recorded length"
    );
    assert!(header_bytes > 64, "the rewritten header is not long");
    let reference = include_bytes!("fixtures/jpegxl_reference_rgb8.raw");
    let desc = colour(ROWS, COLUMNS, 8, 8, "RGB")?;
    let mut out = vec![0xa5; desc.output_len()];
    JpegXlDecoder::lossless().decode(&long, &desc, &mut out)?;
    assert_eq!(out.as_slice(), reference.as_slice());

    let wrapped = jxlp_container(&long, 16)?;
    let mut out = vec![0xa5; desc.output_len()];
    JpegXlDecoder::lossless().decode(&wrapped, &desc, &mut out)?;
    assert_eq!(out.as_slice(), reference.as_slice());
    Ok(())
}

/// Rewrites the header of `jpegxl_lossless_rgb8.jxl` so that one unknown
/// extension carries `skip_bits` skipped bits. The extension data is skipped
/// in constant time and no extra channel is declared, so the header is valid
/// and passes the pre-check, and its length comes only from the skip.
fn rewrite_extension_skip_header(
    fixture: &[u8],
    skip_bits: usize,
) -> Result<Vec<u8>, Box<dyn Error>> {
    let expected_prefix = [false, false, false, false, false, true, false, false, false];
    for (index, bit) in expected_prefix.iter().enumerate() {
        assert_eq!(
            fixture_bit(fixture, 25 + index)?,
            *bit,
            "fixture header changed shape at bit {}",
            25 + index
        );
    }
    assert!(
        fixture_bit(fixture, 34)?,
        "bit 34 is not the colour encoding all_default bit"
    );
    let mut writer = BitWriter::new();
    writer.append_bits_from(fixture, 0, 35)?;
    push_u64(&mut writer, 1);
    push_u64(
        &mut writer,
        u64::try_from(skip_bits).map_err(|_| "skip length")?,
    );
    for _ in 0..skip_bits {
        writer.push_bit(false);
    }
    writer.push_bit(true);
    writer.pad_to_byte();
    writer.append_bits_from(fixture, 40, fixture.len() * 8 - 40)?;
    Ok(writer.finish())
}

#[test]
fn header_with_a_long_extension_decodes() -> TestResult {
    // Review pass 3 D1. Extra channels are refused in the pre-check, so no
    // named header reaches read_header any more. A header can still be long
    // without them, and this one is 275010 bytes of extension skip. It must
    // decode to the reference, bare and over 4 KiB jxlp boxes, so read_header
    // feeds the codestream in doubling windows and the decode stays linear in
    // the header length.
    let fixture = include_bytes!("fixtures/jpegxl_lossless_rgb8.jxl");
    let long = rewrite_extension_skip_header(fixture, 2_200_000)?;
    let reference = include_bytes!("fixtures/jpegxl_reference_rgb8.raw");
    let desc = colour(ROWS, COLUMNS, 8, 8, "RGB")?;
    let mut out = vec![0xa5; desc.output_len()];
    JpegXlDecoder::lossless().decode(&long, &desc, &mut out)?;
    assert_eq!(out.as_slice(), reference.as_slice());

    let wrapped = jxlp_container(&long, 4096)?;
    let mut out = vec![0xa5; desc.output_len()];
    JpegXlDecoder::lossless().decode(&wrapped, &desc, &mut out)?;
    assert_eq!(out.as_slice(), reference.as_slice());
    Ok(())
}

/// Builds a header only bare codestream whose header carries `channels`
/// extra channels, each named with `name_len` bytes, the review's 69 KB and
/// 275 KB shapes. The names are the header's own length, and the stream has
/// no frame, because the pre-check refuses it before one is read.
fn extra_channel_header(channels: usize, name_len: usize) -> Result<Vec<u8>, Box<dyn Error>> {
    let fixture = include_bytes!("fixtures/jpegxl_lossless_rgb8.jxl");
    if channels < 2 {
        return Err("the num_extra coding needs at least two channels".into());
    }
    if !(48..=48 + 1023).contains(&name_len) {
        return Err("the name length needs the 48 + u(10) coding".into());
    }
    let mut writer = BitWriter::new();
    writer.append_bits_from(fixture, 0, 25)?;
    writer.push_bit(false);
    writer.push_bit(false);
    writer.push_bit(false);
    writer.push_bits(0, 2);
    writer.push_bit(true);
    if channels < 2 + 16 {
        writer.push_bits(2, 2);
        writer.push_bits(u32::try_from(channels - 2).map_err(|_| "channels")?, 4);
    } else {
        writer.push_bits(3, 2);
        writer.push_bits(u32::try_from(channels - 1).map_err(|_| "channels")?, 12);
    }
    for _ in 0..channels {
        writer.push_bit(false);
        push_small_enum(&mut writer, 1);
        writer.push_bit(false);
        writer.push_bits(0, 2);
        writer.push_bits(0, 2);
        writer.push_bits(3, 2);
        writer.push_bits(u32::try_from(name_len - 48).map_err(|_| "name length")?, 10);
        for _ in 0..name_len {
            writer.push_bits(u32::from(b'a'), 8);
        }
    }
    writer.push_bit(false);
    writer.push_bit(true);
    writer.push_bits(0, 2);
    writer.push_bit(true);
    Ok(writer.finish())
}

#[test]
fn extra_channel_header_refuses_before_the_reparse() -> TestResult {
    // Review pass 3 D1. A header whose length comes from extra channel names
    // is valid and has no ICC flag, so before the pre-check refusal it
    // reached read_header and was re-parsed once per fed byte. The review
    // measured 7.21 seconds for 64 channels and 108.4 seconds for 256. On
    // 2026-11-06 the same shapes measured 10.29 and 163.80 seconds here
    // before the pre-check refusal, and 882.125 microseconds and 3.564
    // milliseconds after it. The bound is far under the reparse cost and far
    // over the refusal cost, so removing the refusal turns this red through
    // time and not through kind.
    let crafted = extra_channel_header(256, 1071)?;
    let desc = colour(ROWS, COLUMNS, 8, 8, "RGB")?;
    let mut out = vec![0xa5; desc.output_len()];
    let start = std::time::Instant::now();
    let result = JpegXlDecoder::general().decode(&crafted, &desc, &mut out);
    let elapsed = start.elapsed();
    assert_eq!(result, Err(CodecError::UnsupportedPixelFormat));
    assert!(out.iter().all(|byte| *byte == 0xa5));
    assert!(
        elapsed < std::time::Duration::from_secs(10),
        "the extra channel header took {elapsed:?}, the pre-check refusal is not in front"
    );
    Ok(())
}

#[test]
fn oversized_header_refuses_before_render() -> TestResult {
    // A header-only codestream declaring 65535 by 65535. There is no frame
    // payload, so a decoder that rendered first could not return this error
    // from the size check.
    //
    // JPEG XL packs the header LSB first after the signature bytes FF 0A.
    // div8 is 0. Height uses U32 selector 2, which is 1 plus 18 bits.
    // 65535 is 1 + 65534, and 65534 in 18 bits follows the selector.
    // Ratio 0 repeats that coding for the width. Image metadata
    // all_default is 1, one further 1 bit follows, and the rest is zero.
    // Those bits are the eight bytes FF 0A F4 FF 07 FA FF 33.
    let header = [0xff, 0x0a, 0xf4, 0xff, 0x07, 0xfa, 0xff, 0x33];
    let desc = mono(ROWS, COLUMNS, 16, 16)?;
    assert_refused(
        &JpegXlDecoder::general(),
        &header,
        &desc,
        CodecError::FrameMismatch,
    )
}

#[test]
fn malformed_jpegxl_never_panics() -> TestResult {
    // Decision 12, as amended. Every truncation and every fixed-seed bit
    // flip of every `.110` and `.112` fixture must return `Ok` with a full
    // write or `Err` with `out` unchanged. A full write is proved with two
    // sentinel buffers: a decode that writes only part of the frame leaves
    // the rest at the sentinel, so two runs over different sentinels
    // disagree. Panics are collected and then reported together. `.111` is
    // not in this scan. It never reaches `jxl-oxide`. The recompression file
    // is scanned under `.112`.
    let cases = fixtures()?;
    let mut failures: Vec<String> = Vec::new();
    let panic_site = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
    let hook_site = std::sync::Arc::clone(&panic_site);
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let payload = info.payload();
        let text = payload
            .downcast_ref::<&str>()
            .copied()
            .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
            .unwrap_or("panic with a payload this build cannot render");
        let rendered = match info.location() {
            Some(location) => format!(
                "{text} at {}:{}:{}",
                location.file(),
                location.line(),
                location.column()
            ),
            None => text.to_owned(),
        };
        *hook_site
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = rendered;
    }));
    for case in &cases {
        for end in 0..case.bytes.len() {
            let Some(prefix) = case.bytes.get(..end) else {
                failures.push(format!(
                    "{} truncation {end} could not be sliced",
                    case.name
                ));
                continue;
            };
            if let Some(failure) = exercise(&case.decoder, prefix, &case.desc, &panic_site) {
                failures.push(format!("{} truncation {end}: {failure}", case.name));
            }
        }
        let mut state = 0x2290_f229_u64;
        for index in 0..2_000 {
            match flip_one(case.bytes, &mut state) {
                Ok(flipped) => {
                    if let Some(failure) =
                        exercise(&case.decoder, &flipped, &case.desc, &panic_site)
                    {
                        failures.push(format!("{} flip {index}: {failure}", case.name));
                    }
                }
                Err(error) => {
                    failures.push(format!("{} flip {index}: {error}", case.name));
                }
            }
        }
    }
    std::panic::set_hook(previous);
    if failures.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "{} panics or bad writes:\n{}",
            failures.len(),
            failures.join("\n")
        )
        .into())
    }
}

struct Case {
    name: &'static str,
    decoder: JpegXlDecoder,
    bytes: &'static [u8],
    desc: FrameDesc,
}

fn fixtures() -> Result<Vec<Case>, Box<dyn Error>> {
    Ok(vec![
        Case {
            name: "jpegxl_lossless_16.jxl",
            decoder: JpegXlDecoder::lossless(),
            bytes: include_bytes!("fixtures/jpegxl_lossless_16.jxl"),
            desc: mono(ROWS, COLUMNS, 16, 16)?,
        },
        Case {
            name: "jpegxl_lossless_12.jxl",
            decoder: JpegXlDecoder::lossless(),
            bytes: include_bytes!("fixtures/jpegxl_lossless_12.jxl"),
            desc: mono(ROWS, COLUMNS, 16, 12)?,
        },
        Case {
            name: "jpegxl_lossless_rgb8.jxl",
            decoder: JpegXlDecoder::lossless(),
            bytes: include_bytes!("fixtures/jpegxl_lossless_rgb8.jxl"),
            desc: colour(ROWS, COLUMNS, 8, 8, "RGB")?,
        },
        Case {
            name: "jpegxl_lossy_12.jxl",
            decoder: JpegXlDecoder::general(),
            bytes: include_bytes!("fixtures/jpegxl_lossy_12.jxl"),
            desc: mono(ROWS, COLUMNS, 16, 12)?,
        },
        Case {
            name: "jpegxl_orient6.jxl",
            decoder: JpegXlDecoder::general(),
            bytes: include_bytes!("fixtures/jpegxl_orient6.jxl"),
            desc: mono(8, 16, 8, 8)?,
        },
        Case {
            name: "jpegxl_two_frames.jxl",
            decoder: JpegXlDecoder::general(),
            bytes: include_bytes!("fixtures/jpegxl_two_frames.jxl"),
            desc: mono(8, 16, 8, 8)?,
        },
        Case {
            name: "jpegxl_alpha.jxl",
            decoder: JpegXlDecoder::general(),
            bytes: include_bytes!("fixtures/jpegxl_alpha.jxl"),
            desc: mono(8, 8, 8, 8)?,
        },
        Case {
            name: "jpegxl_icc.jxl",
            decoder: JpegXlDecoder::general(),
            bytes: include_bytes!("fixtures/jpegxl_icc.jxl"),
            desc: colour(8, 8, 8, 8, "RGB")?,
        },
        Case {
            name: "jpegxl_icc_compressed.jxl",
            decoder: JpegXlDecoder::general(),
            bytes: include_bytes!("fixtures/jpegxl_icc_compressed.jxl"),
            desc: colour(8, 8, 8, 8, "RGB")?,
        },
        Case {
            name: "jpegxl_pq.jxl",
            decoder: JpegXlDecoder::general(),
            bytes: include_bytes!("fixtures/jpegxl_pq.jxl"),
            desc: mono(8, 16, 8, 8)?,
        },
        Case {
            name: "jpegxl_float.jxl",
            decoder: JpegXlDecoder::general(),
            bytes: include_bytes!("fixtures/jpegxl_float.jxl"),
            desc: mono(4, 4, 8, 8)?,
        },
        Case {
            name: "jpegxl_container.jxl",
            decoder: JpegXlDecoder::general(),
            bytes: include_bytes!("fixtures/jpegxl_container.jxl"),
            desc: mono(8, 16, 8, 8)?,
        },
        Case {
            name: "jpegxl_modular_xyb.jxl",
            decoder: JpegXlDecoder::general(),
            bytes: include_bytes!("fixtures/jpegxl_modular_xyb.jxl"),
            desc: colour(8, 8, 8, 8, "RGB")?,
        },
        // The recompression file under `.112`. `.112` renders a frame that
        // carries reconstruction data. Since the pass 3 extraction, the
        // container is stripped to its codestream before the dependency sees
        // it, so the `jbrd` box is not fed to `jxl-jbr`'s parser and this
        // case scans the codestream parse alone.
        Case {
            name: "jpegxl_recompression.jxl as .112",
            decoder: JpegXlDecoder::general(),
            bytes: include_bytes!("fixtures/jpegxl_recompression.jxl"),
            desc: colour(ROWS, COLUMNS, 8, 8, "YBR_FULL_422")?,
        },
    ])
}

fn exercise(
    decoder: &JpegXlDecoder,
    src: &[u8],
    desc: &FrameDesc,
    panic_site: &std::sync::Arc<std::sync::Mutex<String>>,
) -> Option<String> {
    let mut first = vec![0xa5; desc.output_len()];
    let decoded = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        decoder.decode(src, desc, &mut first)
    }));
    match decoded {
        Ok(Ok(())) => {
            let mut second = vec![0x5a; desc.output_len()];
            let replayed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                decoder.decode(src, desc, &mut second)
            }));
            match replayed {
                Ok(Ok(())) => {
                    if first != second {
                        Some("decode wrote only part of the frame on success".to_owned())
                    } else {
                        None
                    }
                }
                Ok(Err(_)) => Some("decode is not deterministic: success then error".to_owned()),
                Err(_) => {
                    let site = panic_site
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner())
                        .clone();
                    Some(format!("panic on the second run: {site}"))
                }
            }
        }
        Ok(Err(_)) => {
            if first.iter().any(|byte| *byte != 0xa5) {
                Some("decode wrote the output before returning an error".to_owned())
            } else {
                None
            }
        }
        Err(_) => {
            let site = panic_site
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .clone();
            let site = if site.is_empty() {
                "panic with no recorded site".to_owned()
            } else {
                site
            };
            Some(format!("panic: {site}"))
        }
    }
}

fn assert_unavailable(registry: &Registry, src: &[u8], desc: &FrameDesc) -> TestResult {
    let mut out = vec![0xa5; desc.output_len()];
    let decoded = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        registry.decode(JPEGXL_RECOMPRESSION, src, desc, &mut out)
    }));
    match decoded {
        Ok(result) => assert_eq!(result, Err(CodecError::KnownUnavailable)),
        Err(_) => return Err("a .111 decode panicked".into()),
    }
    if out.iter().any(|byte| *byte != 0xa5) {
        return Err("unavailable decode changed the output".into());
    }
    Ok(())
}

fn flip_one(source: &[u8], state: &mut u64) -> Result<Vec<u8>, Box<dyn Error>> {
    let len = u64::try_from(source.len()).map_err(|_| "fixture length")?;
    if len == 0 {
        return Err("empty fixture".into());
    }
    let index = usize::try_from(next_lcg(state) % len).map_err(|_| "bit index")?;
    let bit = u32::try_from(next_lcg(state) % 8).map_err(|_| "bit")?;
    let mut flipped = source.to_vec();
    let byte = flipped.get_mut(index).ok_or("bit index")?;
    let mask = 1u8.checked_shl(bit).ok_or("bit shift")?;
    *byte ^= mask;
    Ok(flipped)
}

fn next_lcg(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1);
    *state
}

fn assert_refused(
    decoder: &JpegXlDecoder,
    src: &[u8],
    desc: &FrameDesc,
    expected: CodecError,
) -> TestResult {
    let mut out = vec![0xa5; desc.output_len()];
    assert_eq!(decoder.decode(src, desc, &mut out), Err(expected));
    if out.iter().any(|byte| *byte != 0xa5) {
        return Err("refused decode changed the output".into());
    }
    Ok(())
}

fn read_le_u16(bytes: &[u8], index: usize) -> Result<u16, Box<dyn Error>> {
    let start = index.checked_mul(2).ok_or("sample index")?;
    let lo = bytes.get(start).copied().ok_or("truncated sample")?;
    let hi = bytes
        .get(start.checked_add(1).ok_or("sample index")?)
        .copied()
        .ok_or("truncated sample")?;
    Ok(u16::from_le_bytes([lo, hi]))
}

fn max_stored(bytes: &[u8]) -> Result<u16, Box<dyn Error>> {
    if !bytes.len().is_multiple_of(2) {
        return Err("odd stored length".into());
    }
    let mut max = 0u16;
    for index in 0..bytes.len() / 2 {
        max = max.max(read_le_u16(bytes, index)?);
    }
    Ok(max)
}

fn max_abs_stored(actual: &[u8], expected: &[u8]) -> Result<u32, Box<dyn Error>> {
    if actual.len() != expected.len() || !actual.len().is_multiple_of(2) {
        return Err("stored length".into());
    }
    let mut max_diff = 0u32;
    for index in 0..actual.len() / 2 {
        let left = u32::from(read_le_u16(actual, index)?);
        let right = u32::from(read_le_u16(expected, index)?);
        max_diff = max_diff.max(left.abs_diff(right));
    }
    Ok(max_diff)
}

fn mono(
    rows: u16,
    columns: u16,
    bits_allocated: u16,
    bits_stored: u16,
) -> Result<FrameDesc, Box<dyn Error>> {
    frame(
        rows,
        columns,
        1,
        bits_allocated,
        bits_stored,
        PixelRepresentation::Unsigned,
        "MONOCHROME2",
    )
}

fn colour(
    rows: u16,
    columns: u16,
    bits_allocated: u16,
    bits_stored: u16,
    photometric: &str,
) -> Result<FrameDesc, Box<dyn Error>> {
    frame(
        rows,
        columns,
        3,
        bits_allocated,
        bits_stored,
        PixelRepresentation::Unsigned,
        photometric,
    )
}

fn frame(
    rows: u16,
    columns: u16,
    samples_per_pixel: u16,
    bits_allocated: u16,
    bits_stored: u16,
    pixel_representation: PixelRepresentation,
    photometric_interpretation: &str,
) -> Result<FrameDesc, Box<dyn Error>> {
    Ok(FrameDesc::new(FrameDescInput {
        rows,
        columns,
        samples_per_pixel,
        bits_allocated,
        bits_stored,
        high_bit: bits_stored.checked_sub(1).ok_or("bits stored is zero")?,
        pixel_representation,
        photometric_interpretation: photometric_interpretation.to_owned(),
        pixel_data_vr: PixelDataVr::Ob,
    })?)
}
