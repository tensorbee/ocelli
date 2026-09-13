//! Native Pixel Data adapters for little-endian and retired big-endian syntax.

use std::sync::Arc;

use crate::{
    Capability, CodecError, Decoder, FrameDesc, PixelDataVr, Registry, RegistryError, RleDecoder,
};

const IMPLICIT_VR_LE: &str = "1.2.840.10008.1.2";
const EXPLICIT_VR_LE: &str = "1.2.840.10008.1.2.1";
const EXPLICIT_VR_BE: &str = "1.2.840.10008.1.2.2";
const RLE_LOSSLESS: &str = "1.2.840.10008.1.2.5";
const IMPLICIT_VR_LE_SYNTAXES: &[&str] = &[IMPLICIT_VR_LE];
const EXPLICIT_VR_LE_SYNTAXES: &[&str] = &[EXPLICIT_VR_LE];
const BIG_ENDIAN_SYNTAXES: &[&str] = &[EXPLICIT_VR_BE];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NativeByteOrder {
    LittleEndian,
    BigEndian,
}

/// Decoder for native DICOM Pixel Data.
pub struct RawDecoder {
    byte_order: NativeByteOrder,
    transfer_syntaxes: &'static [&'static str],
    requires_ow: bool,
}

/// Validated complete native Pixel Data Value with checked frame boundaries.
///
/// Construction owns whole-Value padding and physical-word validation because
/// [`Decoder::decode`] receives only one logical frame. Frame extraction uses
/// the global bit offset, so a one-bit frame may begin inside a byte or word.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeFrameIndex<'a> {
    value: &'a [u8],
    frame_count: usize,
    frame_bits: u64,
    output_len: usize,
    byte_order: NativeByteOrder,
    pixel_data_vr: PixelDataVr,
}

impl RawDecoder {
    /// Decoder for Implicit VR Little Endian, whose Pixel Data VR is OW.
    #[must_use]
    pub const fn implicit_vr_little_endian() -> Self {
        Self {
            byte_order: NativeByteOrder::LittleEndian,
            transfer_syntaxes: IMPLICIT_VR_LE_SYNTAXES,
            requires_ow: true,
        }
    }

    /// Decoder for Explicit VR Little Endian.
    #[must_use]
    pub const fn explicit_vr_little_endian() -> Self {
        Self {
            byte_order: NativeByteOrder::LittleEndian,
            transfer_syntaxes: EXPLICIT_VR_LE_SYNTAXES,
            requires_ow: false,
        }
    }

    /// Decoder for retired Explicit VR Big Endian.
    #[must_use]
    pub const fn big_endian() -> Self {
        Self {
            byte_order: NativeByteOrder::BigEndian,
            transfer_syntaxes: BIG_ENDIAN_SYNTAXES,
            requires_ow: false,
        }
    }

    /// Validate one complete native Pixel Data Value and retain its frame map.
    ///
    /// OB requires its whole-Value trailing NULL only when the packed image
    /// bytes have odd length. OW requires complete physical words and treats
    /// unused final-word bits as insignificant.
    ///
    /// # Errors
    ///
    /// Returns a pixel-format, frame-count, source-length, or OB-padding error
    /// without retaining partially validated evidence.
    pub fn index_native_value<'a>(
        &self,
        value: &'a [u8],
        frame_count: usize,
        desc: &FrameDesc,
    ) -> Result<NativeFrameIndex<'a>, CodecError> {
        validate_pixel_format(self.requires_ow, desc)?;
        if frame_count == 0 {
            return Err(CodecError::FrameMismatch);
        }
        let frame_bits = frame_bits(desc)?;
        let frame_count_u64 = u64::try_from(frame_count).map_err(|_| CodecError::FrameMismatch)?;
        let value_bits = frame_bits
            .checked_mul(frame_count_u64)
            .ok_or(CodecError::FrameMismatch)?;
        let packed_bytes = value_bits
            .checked_add(7)
            .and_then(|bits| bits.checked_div(8))
            .ok_or(CodecError::FrameMismatch)?;
        let physical_bytes = match desc.pixel_data_vr() {
            PixelDataVr::Ob => packed_bytes
                .checked_add(packed_bytes % 2)
                .ok_or(CodecError::FrameMismatch)?,
            PixelDataVr::Ow => value_bits
                .checked_add(15)
                .and_then(|bits| bits.checked_div(16))
                .and_then(|words| words.checked_mul(2))
                .ok_or(CodecError::FrameMismatch)?,
        };
        let physical_len =
            usize::try_from(physical_bytes).map_err(|_| CodecError::FrameMismatch)?;
        if value.len() < physical_len {
            return Err(CodecError::InvalidCodestream);
        }
        if value.len() > physical_len {
            return Err(CodecError::TrailingData);
        }
        if desc.pixel_data_vr() == PixelDataVr::Ob && packed_bytes % 2 == 1 {
            let pad_index = usize::try_from(packed_bytes).map_err(|_| CodecError::FrameMismatch)?;
            if value.get(pad_index).copied() != Some(0) {
                return Err(CodecError::TrailingData);
            }
        }
        Ok(NativeFrameIndex {
            value,
            frame_count,
            frame_bits,
            output_len: desc.output_len(),
            byte_order: self.byte_order,
            pixel_data_vr: desc.pixel_data_vr(),
        })
    }
}

impl NativeFrameIndex<'_> {
    /// Extract one frame into packed canonical little-endian bytes.
    ///
    /// One-bit output starts its first Pixel Cell at the least-significant bit
    /// of `out[0]`, irrespective of the frame's bit offset in the complete
    /// Value. Unused high output bits are zero.
    ///
    /// # Errors
    ///
    /// Returns a frame-bound or output-length error before modifying `out`.
    pub fn decode_frame(&self, frame: usize, out: &mut [u8]) -> Result<(), CodecError> {
        if frame >= self.frame_count {
            return Err(CodecError::FrameMismatch);
        }
        if out.len() != self.output_len {
            return Err(CodecError::OutputLength {
                expected: self.output_len,
                actual: out.len(),
            });
        }
        let frame_u64 = u64::try_from(frame).map_err(|_| CodecError::FrameMismatch)?;
        let frame_start = self
            .frame_bits
            .checked_mul(frame_u64)
            .ok_or(CodecError::FrameMismatch)?;
        let frame_end = frame_start
            .checked_add(self.frame_bits)
            .ok_or(CodecError::FrameMismatch)?;
        let final_canonical_byte = frame_end
            .checked_sub(1)
            .and_then(|bit| bit.checked_div(8))
            .ok_or(CodecError::FrameMismatch)?;
        let final_physical_byte = match (self.byte_order, self.pixel_data_vr) {
            (NativeByteOrder::BigEndian, PixelDataVr::Ow) => final_canonical_byte | 1,
            _ => final_canonical_byte,
        };
        let final_physical_byte =
            usize::try_from(final_physical_byte).map_err(|_| CodecError::FrameMismatch)?;
        if final_physical_byte >= self.value.len() {
            return Err(CodecError::InvalidCodestream);
        }

        for (output_byte, target) in (0_u64..).zip(out.iter_mut()) {
            let local_start = output_byte * 8;
            let bit_count = self.frame_bits.saturating_sub(local_start).min(8);
            let mut packed = 0_u8;
            for local_bit in 0..bit_count {
                let source_bit = frame_start + local_start + local_bit;
                if self.source_bit(source_bit) {
                    packed |= 1_u8 << local_bit;
                }
            }
            *target = packed;
        }
        Ok(())
    }

    fn source_bit(&self, bit: u64) -> bool {
        let canonical_byte = bit / 8;
        let physical_byte = self.physical_byte_index(canonical_byte);
        let Ok(physical_byte) = usize::try_from(physical_byte) else {
            return false;
        };
        self.value
            .get(physical_byte)
            .is_some_and(|byte| byte & (1_u8 << (bit % 8)) != 0)
    }

    const fn physical_byte_index(&self, canonical_byte: u64) -> u64 {
        match (self.byte_order, self.pixel_data_vr) {
            (NativeByteOrder::BigEndian, PixelDataVr::Ow) => canonical_byte ^ 1,
            _ => canonical_byte,
        }
    }
}

/// Register the native Pixel Data and RLE adapters after one complete preflight.
///
/// Deflated Explicit VR Little Endian is deliberately absent because its
/// Deflate stream wraps the complete data set and remains owned by
/// `ocelli-dicom`.
///
/// # Errors
///
/// Returns a registry collision or catalogue error without changing any of
/// the four target capabilities.
pub fn register_native_and_rle_decoders(registry: &mut Registry) -> Result<(), RegistryError> {
    for uid in [IMPLICIT_VR_LE, EXPLICIT_VR_LE, EXPLICIT_VR_BE, RLE_LOSSLESS] {
        match registry.capability(uid) {
            Capability::Available => return Err(RegistryError::AlreadyRegistered { uid }),
            Capability::KnownUnavailable => {}
            Capability::Unknown => return Err(RegistryError::UnknownTransferSyntax { uid }),
        }
    }

    registry.register(Arc::new(RawDecoder::implicit_vr_little_endian()))?;
    registry.register(Arc::new(RawDecoder::explicit_vr_little_endian()))?;
    registry.register(Arc::new(RawDecoder::big_endian()))?;
    registry.register(Arc::new(RleDecoder))
}

impl Decoder for RawDecoder {
    fn transfer_syntaxes(&self) -> &'static [&'static str] {
        self.transfer_syntaxes
    }

    fn decode(&self, src: &[u8], desc: &FrameDesc, out: &mut [u8]) -> Result<(), CodecError> {
        if out.len() != desc.output_len() {
            return Err(CodecError::OutputLength {
                expected: desc.output_len(),
                actual: out.len(),
            });
        }
        validate_pixel_format(self.requires_ow, desc)?;

        let logical_len = desc.output_len();
        validate_logical_frame_length(src, logical_len)?;
        if self.byte_order == NativeByteOrder::BigEndian
            && desc.pixel_data_vr() == PixelDataVr::Ow
            && !src.len().is_multiple_of(2)
        {
            return Err(CodecError::InvalidCodestream);
        }
        match (self.byte_order, desc.pixel_data_vr()) {
            (NativeByteOrder::LittleEndian, _) | (NativeByteOrder::BigEndian, PixelDataVr::Ob) => {
                out.copy_from_slice(src);
            }
            (NativeByteOrder::BigEndian, PixelDataVr::Ow) => {
                let normalized = src.chunks_exact(2).flat_map(|word| word.iter().rev());
                for (target, source) in out.iter_mut().zip(normalized) {
                    *target = *source;
                }
            }
        }
        Ok(())
    }
}

fn validate_pixel_format(requires_ow: bool, desc: &FrameDesc) -> Result<(), CodecError> {
    if (requires_ow && desc.pixel_data_vr() != PixelDataVr::Ow)
        || (desc.pixel_data_vr() == PixelDataVr::Ob && desc.bits_allocated() > 8)
    {
        Err(CodecError::UnsupportedPixelFormat)
    } else {
        Ok(())
    }
}

fn validate_logical_frame_length(src: &[u8], logical_len: usize) -> Result<(), CodecError> {
    if src.len() == logical_len {
        Ok(())
    } else if src.len() > logical_len {
        Err(CodecError::TrailingData)
    } else {
        Err(CodecError::InvalidCodestream)
    }
}

fn frame_bits(desc: &FrameDesc) -> Result<u64, CodecError> {
    u64::from(desc.rows())
        .checked_mul(u64::from(desc.columns()))
        .and_then(|bits| bits.checked_mul(u64::from(desc.samples_per_pixel())))
        .and_then(|bits| bits.checked_mul(u64::from(desc.bits_allocated())))
        .ok_or(CodecError::FrameMismatch)
}
