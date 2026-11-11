//! Bit and byte readers shared by the parsers of this module.
//!
//! [`Bits`] reads the RBSP of one NAL unit, dropping each
//! emulation_prevention_three_byte as it goes, Rec. ITU-T H.264 (08/2024)
//! 7.3.1 and Rec. ITU-T H.265 (07/2024) 7.3.1.1, so a slice header is read
//! without copying the slice. For MPEG-2, which has no emulation prevention,
//! it reads bytes as they are.

use super::VideoError;

/// A bounds-checked reader of big-endian bits.
pub(crate) struct Bits<'a> {
    data: &'a [u8],
    /// The next byte of `data` to load.
    next: usize,
    /// Zero bytes immediately before `next`, for emulation prevention.
    zeros: u8,
    /// Whether 0x000003 drops the 03.
    emulation_prevention: bool,
    /// The loaded byte and how many of its bits are still unread.
    current: u8,
    left: u8,
    structure: &'static str,
}

impl<'a> Bits<'a> {
    /// A reader of an H.264 or HEVC RBSP inside the NAL unit bytes `data`.
    pub(crate) fn rbsp(data: &'a [u8], structure: &'static str) -> Self {
        Self::new(data, structure, true)
    }

    /// A reader of plain bytes.
    pub(crate) fn plain(data: &'a [u8], structure: &'static str) -> Self {
        Self::new(data, structure, false)
    }

    fn new(data: &'a [u8], structure: &'static str, emulation_prevention: bool) -> Self {
        Self {
            data,
            next: 0,
            zeros: 0,
            emulation_prevention,
            current: 0,
            left: 0,
            structure,
        }
    }

    fn truncated(&self) -> VideoError {
        VideoError::Truncated {
            structure: self.structure,
        }
    }

    pub(crate) fn malformed(&self, reason: &'static str) -> VideoError {
        VideoError::Malformed {
            structure: self.structure,
            reason,
        }
    }

    fn load(&mut self) -> Result<(), VideoError> {
        let mut byte = *self.data.get(self.next).ok_or_else(|| self.truncated())?;
        self.next += 1;
        if self.emulation_prevention && self.zeros >= 2 && byte == 3 {
            byte = *self.data.get(self.next).ok_or_else(|| self.truncated())?;
            self.next += 1;
            self.zeros = 0;
        }
        self.zeros = if byte == 0 {
            self.zeros.saturating_add(1)
        } else {
            0
        };
        self.current = byte;
        self.left = 8;
        Ok(())
    }

    /// u(1), one flag.
    pub(crate) fn flag(&mut self) -> Result<bool, VideoError> {
        if self.left == 0 {
            self.load()?;
        }
        self.left -= 1;
        Ok((self.current >> self.left) & 1 == 1)
    }

    /// u(n) for n of 0 to 32.
    pub(crate) fn bits(&mut self, count: u8) -> Result<u32, VideoError> {
        if count > 32 {
            return Err(self.malformed("a field wider than 32 bits"));
        }
        let mut value = 0u32;
        for _ in 0..count {
            value = value << 1 | u32::from(self.flag()?);
        }
        Ok(value)
    }

    /// u(8).
    pub(crate) fn byte(&mut self) -> Result<u8, VideoError> {
        let value = self.bits(8)?;
        // Eight bits fit a u8.
        u8::try_from(value).map_err(|_| self.malformed("an eight bit field over 255"))
    }

    /// Skips `count` bits.
    pub(crate) fn skip(&mut self, count: u32) -> Result<(), VideoError> {
        for _ in 0..count {
            self.flag()?;
        }
        Ok(())
    }

    /// ue(v), Exp-Golomb, Rec. ITU-T H.264 (08/2024) 9.1 and Rec. ITU-T
    /// H.265 (07/2024) 9.2. A code longer than 32 leading zeros cannot
    /// encode a value of 32 bits, so it is malformed. Private, so every parser
    /// reads through [`Bits::ue_max`] with the range its clause gives.
    fn ue(&mut self) -> Result<u32, VideoError> {
        let mut leading = 0u8;
        while !self.flag()? {
            leading += 1;
            if leading > 31 {
                return Err(self.malformed("an Exp-Golomb code longer than 32 bits"));
            }
        }
        let suffix = self.bits(leading)?;
        // codeNum = 2^leadingZeroBits - 1 + read_bits(leadingZeroBits), and
        // with at most 31 leading zeros it fits u32.
        let base = (1u32 << leading) - 1;
        base.checked_add(suffix)
            .ok_or_else(|| self.malformed("an Exp-Golomb value over 32 bits"))
    }

    /// se(v), the signed mapping of Rec. ITU-T H.264 (08/2024) Table 9-3:
    /// codeNum k gives (-1)^(k+1) * Ceil(k / 2). Private, so every parser
    /// reads through [`Bits::se_range`] with the range its clause gives.
    fn se(&mut self) -> Result<i32, VideoError> {
        let code = self.ue()?;
        let magnitude = i32::try_from(code / 2 + code % 2)
            .map_err(|_| self.malformed("a signed Exp-Golomb value out of range"))?;
        Ok(if code % 2 == 1 { magnitude } else { -magnitude })
    }

    /// ue(v) bounded by an inclusive maximum from the specification. ue(v)
    /// itself never exceeds 2^32 - 2, so a maximum of `u32::MAX - 1` states a
    /// clause's range that the reader has already enforced.
    pub(crate) fn ue_max(&mut self, max: u32, reason: &'static str) -> Result<u32, VideoError> {
        let value = self.ue()?;
        if value > max {
            return Err(self.malformed(reason));
        }
        Ok(value)
    }

    /// se(v) bounded by an inclusive range from the specification.
    pub(crate) fn se_range(
        &mut self,
        min: i32,
        max: i32,
        reason: &'static str,
    ) -> Result<i32, VideoError> {
        let value = self.se()?;
        if !(min..=max).contains(&value) {
            return Err(self.malformed(reason));
        }
        Ok(value)
    }
}

/// A big-endian field of `N` bytes at `offset`.
pub(crate) fn be<const N: usize>(
    data: &[u8],
    offset: usize,
    structure: &'static str,
) -> Result<[u8; N], VideoError> {
    let end = offset
        .checked_add(N)
        .ok_or(VideoError::Truncated { structure })?;
    data.get(offset..end)
        .and_then(|field| <[u8; N]>::try_from(field).ok())
        .ok_or(VideoError::Truncated { structure })
}

pub(crate) fn be_u8(data: &[u8], offset: usize, structure: &'static str) -> Result<u8, VideoError> {
    data.get(offset)
        .copied()
        .ok_or(VideoError::Truncated { structure })
}

pub(crate) fn be_u16(
    data: &[u8],
    offset: usize,
    structure: &'static str,
) -> Result<u16, VideoError> {
    be::<2>(data, offset, structure).map(u16::from_be_bytes)
}

pub(crate) fn be_u32(
    data: &[u8],
    offset: usize,
    structure: &'static str,
) -> Result<u32, VideoError> {
    be::<4>(data, offset, structure).map(u32::from_be_bytes)
}

pub(crate) fn be_u64(
    data: &[u8],
    offset: usize,
    structure: &'static str,
) -> Result<u64, VideoError> {
    be::<8>(data, offset, structure).map(u64::from_be_bytes)
}

/// A u32 as an index. Every target this crate builds for has a usize of at
/// least 32 bits, and the conversion is checked rather than assumed.
pub(crate) fn to_usize(value: u32, structure: &'static str) -> Result<usize, VideoError> {
    usize::try_from(value).map_err(|_| VideoError::Malformed {
        structure,
        reason: "a count larger than this target addresses",
    })
}

/// One start code delimited unit of an Annex B or MPEG-2 byte stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Unit {
    /// The first byte of the start code, including a leading zero_byte.
    pub(crate) start: usize,
    /// The first byte after the 0x000001 prefix.
    pub(crate) payload: usize,
    /// One past the last byte of the unit, before any trailing zero bytes.
    pub(crate) end: usize,
}

/// Start code delimited units in byte stream order.
///
/// Rec. ITU-T H.264 (08/2024) B.2 and Rec. ITU-T H.265 (07/2024) B.3: a NAL
/// unit follows the three byte prefix 0x000001 and ends where the next
/// three bytes are 0x000000 or 0x000001. A zero byte before the prefix is a
/// zero_byte of the next start code, so it starts the next unit. MPEG-2
/// start codes are byte aligned behind the same prefix, Rec. ITU-T H.262
/// (02/2000) 5.2.3, and zero bytes before them are stuffing.
pub(crate) fn units(data: &[u8]) -> impl Iterator<Item = Unit> + '_ {
    let mut pending: Option<(usize, usize)> = next_prefix(data, 0);
    std::iter::from_fn(move || {
        let (start, payload) = pending?;
        let following = next_prefix(data, payload);
        let mut end = following.map_or(data.len(), |(next_start, _)| next_start);
        while end > payload && data.get(end - 1) == Some(&0) {
            end -= 1;
        }
        pending = following;
        Some(Unit {
            start,
            payload,
            end,
        })
    })
}

/// The next 0x000001 at or after `from`, as (first byte including leading
/// zero bytes, first byte after the prefix).
fn next_prefix(data: &[u8], from: usize) -> Option<(usize, usize)> {
    let rest = data.get(from..)?;
    let found = rest.windows(3).position(|window| window == [0, 0, 1])?;
    let prefix = from + found;
    let mut start = prefix;
    while start > from && data.get(start - 1) == Some(&0) {
        start -= 1;
    }
    Some((start, prefix + 3))
}

/// A bit writer for crafting syntax structures in unit tests, the inverse of
/// [`Bits`]: u(n), ue(v) by Rec. ITU-T H.264 (08/2024) 9.1, se(v) by Table
/// 9-3, then rbsp_trailing_bits and emulation prevention by 7.3.1.
#[cfg(test)]
#[derive(Default)]
pub(crate) struct Writer {
    bits: Vec<bool>,
}

#[cfg(test)]
impl Writer {
    pub(crate) fn u(&mut self, width: u32, value: u64) -> &mut Self {
        for bit in (0..width).rev() {
            self.bits.push((value >> bit) & 1 == 1);
        }
        self
    }

    pub(crate) fn flag(&mut self, value: bool) -> &mut Self {
        self.bits.push(value);
        self
    }

    /// byte_alignment( ) of Rec. ITU-T H.265 (07/2024) 7.3.2.12: a 1, then
    /// 0 to the next byte boundary.
    pub(crate) fn align(&mut self) -> &mut Self {
        self.bits.push(true);
        while !self.bits.len().is_multiple_of(8) {
            self.bits.push(false);
        }
        self
    }

    /// codeNum k is written as M zeros, a one and the M low bits of k + 1,
    /// with M = Floor( Log2( k + 1 ) ).
    pub(crate) fn ue(&mut self, value: u64) -> &mut Self {
        let code = value + 1;
        let width = 64 - code.leading_zeros();
        self.u(width - 1, 0).u(width, code)
    }

    /// Table 9-3: a positive value v is codeNum 2v - 1 and a value v of 0 or
    /// less is codeNum -2v.
    pub(crate) fn se(&mut self, value: i64) -> &mut Self {
        let code = if value > 0 { 2 * value - 1 } else { -2 * value };
        self.ue(code.unsigned_abs())
    }

    /// The bits as bytes, with no trailing bits, for a byte stream that is
    /// not an RBSP.
    pub(crate) fn bytes(&self) -> Vec<u8> {
        self.bits
            .chunks(8)
            .map(|chunk| {
                chunk
                    .iter()
                    .chain(std::iter::repeat(&false))
                    .take(8)
                    .fold(0u8, |byte, bit| byte << 1 | u8::from(*bit))
            })
            .collect()
    }

    /// A NAL unit: the header bytes, then the RBSP with rbsp_trailing_bits
    /// and an emulation_prevention_three_byte before any 00 00 0x with x of 3
    /// or less.
    pub(crate) fn nal(&self, header: &[u8]) -> Vec<u8> {
        let mut rbsp = self.bits.clone();
        rbsp.push(true);
        while !rbsp.len().is_multiple_of(8) {
            rbsp.push(false);
        }
        let raw = Writer { bits: rbsp }.bytes();
        let mut out = header.to_vec();
        let mut zeros = 0;
        for byte in raw {
            if zeros >= 2 && byte <= 3 {
                out.push(3);
                zeros = 0;
            }
            out.push(byte);
            zeros = if byte == 0 { zeros + 1 } else { 0 };
        }
        out
    }
}

/// Writes a crafted row stream to `OCELLI_F241_ROW_DUMP` when that variable
/// names a directory, so an independent reader, ffmpeg's trace_headers run as
/// a host tool, can check the builders. Without the variable it does nothing.
#[cfg(test)]
pub(crate) fn dump_row(prefix: &str, name: &str, stream: &[u8]) {
    if let Some(directory) = std::env::var_os("OCELLI_F241_ROW_DUMP") {
        let file: String = name
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
            .collect();
        let path = std::path::Path::new(&directory).join(format!("{prefix}_{file}.{prefix}"));
        // Review pass 4 N1: a dump that cannot be written fails the run and
        // names the path, so an empty dump is never read as a clean one.
        let written = std::fs::write(&path, stream);
        assert!(
            written.is_ok(),
            "OCELLI_F241_ROW_DUMP: cannot write {}: {:?}",
            path.display(),
            written.err()
        );
    }
}

/// An Annex B byte stream of NAL units, each behind a four byte start code.
#[cfg(test)]
pub(crate) fn annex_b(units: &[Vec<u8>]) -> Vec<u8> {
    units
        .iter()
        .flat_map(|unit| [0u8, 0, 0, 1].into_iter().chain(unit.iter().copied()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{Bits, Unit, Writer, units};

    /// The writer and reader agree on every form, so a crafted structure
    /// reads back as written.
    #[test]
    fn writer_round_trips_through_the_reader() -> Result<(), super::VideoError> {
        let mut writer = Writer::default();
        writer
            .ue(0)
            .ue(7)
            .ue(65_535)
            .se(-128)
            .se(127)
            .se(0)
            .u(5, 0b10110)
            .flag(true);
        let nal = writer.nal(&[]);
        let mut bits = Bits::rbsp(&nal, "test");
        assert_eq!(bits.ue()?, 0);
        assert_eq!(bits.ue()?, 7);
        assert_eq!(bits.ue()?, 65_535);
        assert_eq!(bits.se()?, -128);
        assert_eq!(bits.se()?, 127);
        assert_eq!(bits.se()?, 0);
        assert_eq!(bits.bits(5)?, 0b10110);
        assert!(bits.flag()?);
        Ok(())
    }

    /// The range checked reads refuse a value outside their bounds.
    #[test]
    fn range_checked_reads_refuse_out_of_range_values() {
        let mut writer = Writer::default();
        writer.ue(16).se(-129).se(128);
        let nal = writer.nal(&[]);
        let mut bits = Bits::rbsp(&nal, "test");
        assert!(bits.ue_max(15, "over 15").is_err());
        assert!(bits.se_range(-128, 127, "outside").is_err());
        assert!(bits.se_range(-128, 127, "outside").is_err());
    }

    /// Rec. ITU-T H.264 (08/2024) Table 9-2: bit strings 1, 010, 011, 00100
    /// are codeNum 0, 1, 2, 3, and Table 9-3 maps codeNum 1, 2, 3 to
    /// se(v) 1, -1, 2.
    #[test]
    fn exp_golomb_follows_tables_9_2_and_9_3() -> Result<(), super::VideoError> {
        // 1 010 011 00100 00100, padded with ones: 1010 0110 0100 0010 0111 1111.
        let data = [0b1010_0110, 0b0100_0010, 0b0111_1111];
        let mut bits = Bits::plain(&data, "test");
        assert_eq!(bits.ue()?, 0);
        assert_eq!(bits.ue()?, 1);
        assert_eq!(bits.ue()?, 2);
        assert_eq!(bits.ue()?, 3);
        // The second 00100 read as se(v): codeNum 3 is +2.
        assert_eq!(bits.se()?, 2);
        Ok(())
    }

    /// Rec. ITU-T H.264 (08/2024) 7.3.1: 0x000003 drops the 03, so the bytes
    /// 00 00 03 01 read as 00 00 01.
    #[test]
    fn emulation_prevention_byte_is_dropped() -> Result<(), super::VideoError> {
        let data = [0x00, 0x00, 0x03, 0x01];
        let mut bits = Bits::rbsp(&data, "test");
        assert_eq!(bits.bits(24)?, 0x00_0001);
        let mut plain = Bits::plain(&data, "test");
        assert_eq!(plain.bits(24)?, 0x00_0003);
        Ok(())
    }

    /// Annex B B.2: the zero byte before a four byte start code belongs to
    /// that start code, and trailing zeros do not belong to the unit before.
    #[test]
    fn start_code_units_follow_annex_b() {
        let data = [
            0, 0, 0, 1, 0x67, 0xAA, 0, 0, 0, 1, 0x68, 0xBB, 0, 0, 1, 0x65,
        ];
        let found: Vec<Unit> = units(&data).collect();
        assert_eq!(
            found,
            [
                Unit {
                    start: 0,
                    payload: 4,
                    end: 6
                },
                Unit {
                    start: 6,
                    payload: 10,
                    end: 12
                },
                Unit {
                    start: 12,
                    payload: 15,
                    end: 16
                },
            ]
        );
    }
}
