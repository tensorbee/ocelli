//! A bound on the encoded ICC profile size a JPEG XL codestream may declare.
//!
//! `jxl-color` 0.11.0 `icc/decode.rs` sizes its encoded ICC buffer from the
//! codestream's own field, up to `1 << 28` bytes, before it reads any of
//! them, and that buffer is not seen by `jxl-grid`'s `AllocTracker`. A 26
//! byte codestream declaring `1 << 28` bytes measured a peak resident set of
//! 270974976 bytes on 2026-11-05, about ten million times its own length,
//! while the decode refused. [`declared_icc_exceeds_codestream`] walks the
//! header with a private bit reader and reports a declaration the remaining
//! codestream cannot supply, so [`super::jpegxl`] can refuse it before
//! `jxl-oxide` parses anything.
//!
//! The walk is conservative in both directions. A header it cannot walk is
//! reported as no finding and left to `jxl-oxide`, and a finding is only
//! raised for a stream whose colour encoding already refuses in
//! [`super::jpegxl`]'s `validate_header`, so the check changes when the
//! refusal happens, never whether. The bound is on the codestream's bit
//! supply, because the encoded stream is entropy decoded, and LZ77
//! compression inside that stream is not modelled.

use std::borrow::Cow;

/// The 12 byte signature of a JPEG XL container, ISO/IEC 18181-2.
const CONTAINER_SIGNATURE: [u8; 12] = *b"\x00\x00\x00\x0cJXL \x0d\x0a\x87\x0a";

/// Returns `true` when `src` declares an encoded ICC profile that the
/// remaining codestream cannot supply.
///
/// The encoded ICC stream is entropy decoded, and each of its `enc_size`
/// output bytes consumes at least one bit, so a declaration larger than the
/// bit count that follows it cannot be honoured. A byte count would be
/// wrong here: the tracked ICC fixture declares 390 encoded bytes into 2040
/// remaining bits, and must stay decodable so that `validate_header` can
/// refuse it as an unsupported pixel format. A stream this function cannot
/// walk, or one that declares no ICC profile, returns `false`.
pub fn declared_icc_exceeds_codestream(src: &[u8]) -> bool {
    let Some(codestream) = codestream_bytes(src) else {
        return false;
    };
    let Ok(Some((enc_size, bits_consumed))) = declared_icc_size(&codestream) else {
        return false;
    };
    let total_bits = codestream.len().saturating_mul(8);
    let Ok(remaining_bits) = u64::try_from(total_bits.saturating_sub(bits_consumed)) else {
        return false;
    };
    enc_size > remaining_bits
}

/// Extracts the codestream from a bare stream or a container.
///
/// A container's `jxlc` and `jxlp` boxes carry the codestream, a `jxlp`
/// payload after its four byte index. Anything else, including a box whose
/// size outruns the input, is left to `jxl-oxide`.
fn codestream_bytes(src: &[u8]) -> Option<Cow<'_, [u8]>> {
    if src.starts_with(&[0xff, 0x0a]) {
        return Some(Cow::Borrowed(src));
    }
    if !src.starts_with(&CONTAINER_SIGNATURE) {
        return None;
    }
    let mut rest = src.get(CONTAINER_SIGNATURE.len()..)?;
    let mut codestream = Vec::new();
    while !rest.is_empty() {
        // A fragment shorter than a box header, such as a DICOM even-length
        // pad after the last box, ends the walk on what was collected.
        let Some((payload, box_type, final_box, consumed)) = split_box(rest) else {
            if rest.len() < 8 {
                break;
            }
            return None;
        };
        if box_type == *b"jxlc" {
            codestream.extend_from_slice(payload);
        } else if box_type == *b"jxlp" {
            let indexed = payload.get(4..)?;
            codestream.extend_from_slice(indexed);
        }
        if final_box {
            break;
        }
        rest = rest.get(consumed..)?;
    }
    Some(Cow::Owned(codestream))
}

/// Splits the next container box into its payload and type.
///
/// Returns the payload, the box type, whether the box runs to the end of the
/// input, which is the meaning of a declared size of zero, and the total box
/// length the caller must consume. `None` is a header that does not fit or a
/// size field that lies.
fn split_box(rest: &[u8]) -> Option<(&[u8], [u8; 4], bool, usize)> {
    let header: [u8; 8] = rest.get(..8)?.try_into().ok()?;
    let declared = u32::from_be_bytes([header[0], header[1], header[2], header[3]]);
    let box_type = [header[4], header[5], header[6], header[7]];
    if declared == 1 {
        let extended: [u8; 8] = rest.get(8..16)?.try_into().ok()?;
        let declared = u64::from_be_bytes(extended);
        let payload_len = usize::try_from(declared.checked_sub(16)?).ok()?;
        let payload = rest.get(16..)?.get(..payload_len)?;
        return Some((payload, box_type, false, 16 + payload_len));
    }
    if declared == 0 {
        return Some((rest.get(8..)?, box_type, true, rest.len()));
    }
    let payload_len = usize::try_from(declared.checked_sub(8)?).ok()?;
    let payload = rest.get(8..)?.get(..payload_len)?;
    Some((payload, box_type, false, 8 + payload_len))
}

/// An LSB-first bit reader over a fixed byte slice, in the order
/// `jxl-bitstream` reads.
struct Bits<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Bits<'a> {
    fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    fn read_bit(&mut self) -> Result<bool, WalkError> {
        let byte = self.data.get(self.pos / 8).ok_or(WalkError)?;
        let bit = (byte >> (self.pos % 8)) & 1 == 1;
        self.pos += 1;
        Ok(bit)
    }

    fn read_bits(&mut self, count: u32) -> Result<u32, WalkError> {
        let mut value = 0u32;
        for index in 0..count {
            if self.read_bit()? {
                value |= 1 << index;
            }
        }
        Ok(value)
    }

    /// Reads a `U32` in the `(offset, extra bits)` distribution form that
    /// `jxl-bitstream`'s `read_u32` takes. A zero bit count is a constant.
    fn read_u32_coded(&mut self, distributions: [(u32, u32); 4]) -> Result<u32, WalkError> {
        let selector = self.read_bits(2)?;
        let index = usize::try_from(selector).map_err(|_| WalkError)?;
        let Some(&(offset, count)) = distributions.get(index) else {
            return Err(WalkError);
        };
        offset.checked_add(self.read_bits(count)?).ok_or(WalkError)
    }

    /// Reads the enum coding, which is `U32(0, 1, 2 + U(4), 18 + U(6))`.
    fn read_enum(&mut self) -> Result<u32, WalkError> {
        self.read_u32_coded([(0, 0), (1, 0), (2, 4), (18, 6)])
    }

    /// Reads the `U64` coding of `jxl-bitstream`.
    fn read_u64(&mut self) -> Result<u64, WalkError> {
        match self.read_bits(2)? {
            0 => Ok(0),
            1 => Ok(u64::from(self.read_bits(4)?) + 1),
            2 => Ok(u64::from(self.read_bits(8)?) + 17),
            _ => {
                let mut value = u64::from(self.read_bits(12)?);
                let mut shift = 12u32;
                loop {
                    if !self.read_bit()? {
                        break;
                    }
                    if shift == 60 {
                        value |= u64::from(self.read_bits(4)?) << shift;
                        break;
                    }
                    value |= u64::from(self.read_bits(8)?) << shift;
                    shift += 8;
                }
                Ok(value)
            }
        }
    }

    /// Advances past `count` bits without reading them.
    fn skip_bits(&mut self, count: u64) -> Result<(), WalkError> {
        let count = usize::try_from(count).map_err(|_| WalkError)?;
        let new_pos = self.pos.checked_add(count).ok_or(WalkError)?;
        if new_pos > self.data.len() * 8 {
            return Err(WalkError);
        }
        self.pos = new_pos;
        Ok(())
    }
}

/// The header walk cannot continue, which is not a finding.
struct WalkError;

/// Walks the codestream header to the declared encoded ICC size.
///
/// Returns `Ok(None)` when the header declares no ICC profile, and
/// `Ok(Some((enc_size, bits consumed)))` when it does, with the bit position
/// just past the declaration. The position is after the whole image metadata,
/// because that is where `jxl-oxide` reads the size from.
fn declared_icc_size(codestream: &[u8]) -> Result<Option<(u64, usize)>, WalkError> {
    let mut bits = Bits::new(codestream);
    if bits.read_bits(16)? != 0xaff {
        return Err(WalkError);
    }
    size_header(&mut bits)?;
    if bits.read_bit()? {
        // The metadata is all default, so its colour encoding is sRGB.
        return Ok(None);
    }
    let extra_fields = bits.read_bit()?;
    if extra_fields {
        bits.read_bits(3)?;
        if bits.read_bit()? {
            size_header(&mut bits)?;
        }
        if bits.read_bit()? {
            preview_header(&mut bits)?;
        }
        if bits.read_bit()? {
            animation_header(&mut bits)?;
        }
    }
    bit_depth(&mut bits)?;
    bits.read_bit()?;
    let num_extra = bits.read_u32_coded([(0, 0), (1, 0), (2, 4), (1, 12)])?;
    for _ in 0..num_extra {
        extra_channel(&mut bits)?;
    }
    let xyb_encoded = bits.read_bit()?;
    if bits.read_bit()? {
        // The colour encoding is all default.
        return Ok(None);
    }
    if !bits.read_bit()? {
        // want_icc is clear, so no size is declared.
        bits.read_enum()?;
        return Ok(None);
    }
    bits.read_enum()?;
    tone_mapping(&mut bits, extra_fields)?;
    extensions(&mut bits)?;
    let default_m = bits.read_bit()?;
    if !default_m && xyb_encoded {
        opsin_inverse_matrix(&mut bits)?;
    }
    if !default_m {
        bits.read_bits(3)?;
    }
    let enc_size = bits.read_u64()?;
    Ok(Some((enc_size, bits.pos)))
}

fn size_header(bits: &mut Bits) -> Result<(), WalkError> {
    let div8 = bits.read_bit()?;
    if div8 {
        bits.read_bits(5)?;
    } else {
        bits.read_u32_coded([(1, 9), (1, 13), (1, 18), (1, 30)])?;
    }
    let ratio = bits.read_bits(3)?;
    if div8 && ratio == 0 {
        bits.read_bits(5)?;
    }
    if !div8 && ratio == 0 {
        bits.read_u32_coded([(1, 9), (1, 13), (1, 18), (1, 30)])?;
    }
    Ok(())
}

fn preview_header(bits: &mut Bits) -> Result<(), WalkError> {
    let div8 = bits.read_bit()?;
    let small = [(16u32, 0u32), (32, 0), (1, 5), (33, 9)];
    let large = [(1u32, 6u32), (65, 8), (321, 10), (1345, 12)];
    if div8 {
        bits.read_u32_coded(small)?;
    } else {
        bits.read_u32_coded(large)?;
    }
    let ratio = bits.read_bits(3)?;
    if div8 {
        bits.read_u32_coded(small)?;
    }
    if !div8 && ratio == 0 {
        bits.read_u32_coded(large)?;
    }
    Ok(())
}

fn animation_header(bits: &mut Bits) -> Result<(), WalkError> {
    bits.read_u32_coded([(100, 0), (1000, 0), (1, 10), (1, 30)])?;
    bits.read_u32_coded([(1, 0), (1001, 0), (1, 8), (1, 10)])?;
    bits.read_u32_coded([(0, 0), (0, 3), (0, 16), (0, 32)])?;
    bits.read_bit()?;
    Ok(())
}

fn bit_depth(bits: &mut Bits) -> Result<(), WalkError> {
    if bits.read_bit()? {
        bits.read_u32_coded([(32, 0), (16, 0), (24, 0), (1, 6)])?;
        bits.read_bits(4)?;
    } else {
        bits.read_u32_coded([(8, 0), (10, 0), (12, 0), (1, 6)])?;
    }
    Ok(())
}

fn extra_channel(bits: &mut Bits) -> Result<(), WalkError> {
    if bits.read_bit()? {
        // A default alpha channel carries no further fields.
        return Ok(());
    }
    let channel_type = bits.read_enum()?;
    bit_depth(bits)?;
    bits.read_u32_coded([(0, 0), (3, 0), (4, 0), (1, 3)])?;
    let name_len = bits.read_u32_coded([(0, 0), (0, 4), (16, 5), (48, 10)])?;
    for _ in 0..name_len {
        bits.read_bits(8)?;
    }
    match channel_type {
        0 => {
            bits.read_bit()?;
        }
        2 => {
            for _ in 0..4 {
                bits.read_bits(16)?;
            }
        }
        5 => {
            bits.read_u32_coded([(1, 0), (0, 2), (3, 4), (19, 8)])?;
        }
        _ => {}
    }
    Ok(())
}

fn tone_mapping(bits: &mut Bits, extra_fields: bool) -> Result<(), WalkError> {
    if !extra_fields {
        return Ok(());
    }
    if !bits.read_bit()? {
        bits.read_bits(16)?;
        bits.read_bits(16)?;
        bits.read_bit()?;
        bits.read_bits(16)?;
    }
    Ok(())
}

fn extensions(bits: &mut Bits) -> Result<(), WalkError> {
    let present = bits.read_u64()?;
    let mut lengths = Vec::new();
    let mut remaining = present;
    for _ in 0..64 {
        if remaining & 1 == 1 {
            lengths.push(bits.read_u64()?);
        }
        remaining >>= 1;
    }
    for length in lengths {
        bits.skip_bits(length)?;
    }
    Ok(())
}

fn opsin_inverse_matrix(bits: &mut Bits) -> Result<(), WalkError> {
    if !bits.read_bit()? {
        for _ in 0..16 {
            bits.read_bits(16)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::declared_icc_exceeds_codestream;

    #[test]
    fn non_jxl_input_is_no_finding() {
        assert!(!declared_icc_exceeds_codestream(b""));
        assert!(!declared_icc_exceeds_codestream(b"not a jxl stream at all"));
    }

    struct Writer {
        bytes: Vec<u8>,
        bit_len: usize,
    }

    impl Writer {
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

        fn finish(mut self) -> Vec<u8> {
            while !self.bit_len.is_multiple_of(8) {
                self.push_bit(false);
            }
            self.bytes
        }
    }

    /// A 26 byte bare codestream whose header declares an encoded ICC
    /// profile of `1 << 28` bytes. The 18 entropy decoded bytes name that
    /// output size, so an unguarded decode reaches `jxl-color`'s resize
    /// before it runs out of input.
    fn crafted_oversized_declaration() -> Vec<u8> {
        let mut w = Writer {
            bytes: Vec::new(),
            bit_len: 0,
        };
        w.push_bits(0x0aff, 16);
        w.push_bit(true);
        w.push_bits(7, 5);
        w.push_bits(4, 3);
        w.push_bit(false);
        w.push_bit(false);
        w.push_bit(false);
        w.push_bits(0, 2);
        w.push_bit(true);
        w.push_bits(0, 2);
        w.push_bit(false);
        w.push_bit(false);
        w.push_bit(true);
        w.push_bits(0, 2);
        w.push_bits(0, 2);
        w.push_bit(true);
        w.push_bits(3, 2);
        w.push_bits(0, 12);
        for chunk in [0u32, 0, 1] {
            w.push_bit(true);
            w.push_bits(chunk, 8);
        }
        w.push_bit(false);
        w.push_bit(false);
        w.push_bit(true);
        w.push_bits(0, 2);
        w.push_bit(true);
        w.push_bits(0, 4);
        w.push_bit(true);
        w.push_bits(7, 4);
        w.push_bits(127, 7);
        w.push_bits(1, 2);
        w.push_bits(3, 2);
        for sym in [0u32, 1, 2, 8] {
            w.push_bits(sym, 8);
        }
        w.push_bit(false);
        for _ in 0..4 {
            w.push_bits(3, 2);
            w.push_bits(0, 7);
        }
        for _ in 0..2 {
            w.push_bits(2, 2);
        }
        for _ in 0..12 {
            w.push_bits(0, 2);
        }
        w.finish()
    }

    #[test]
    fn oversized_declared_icc_is_a_finding() {
        let crafted = crafted_oversized_declaration();
        assert_eq!(crafted.len(), 26);
        assert!(declared_icc_exceeds_codestream(&crafted));
    }

    #[test]
    fn container_wrapped_oversized_declared_icc_is_a_finding() {
        let crafted = crafted_oversized_declaration();
        let mut container = Vec::new();
        container.extend_from_slice(super::CONTAINER_SIGNATURE.as_slice());
        let box_len = u32::try_from(crafted.len() + 8).unwrap_or(u32::MAX);
        container.extend_from_slice(&box_len.to_be_bytes());
        container.extend_from_slice(b"jxlc");
        container.extend_from_slice(&crafted);
        assert!(declared_icc_exceeds_codestream(&container));
    }
}
