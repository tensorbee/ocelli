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
        validate_column_pairs(desc)?;
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
        // The per-frame output is the stored frame's own length, which equals
        // `desc.output_len()` for every photometric whose stored sample count
        // is Samples per Pixel and is the two-sample 4:2:2 length otherwise.
        let output_len =
            usize::try_from(frame_bits.div_ceil(8)).map_err(|_| CodecError::FrameMismatch)?;
        Ok(NativeFrameIndex {
            value,
            frame_count,
            frame_bits,
            output_len,
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
        if self.copy_aligned(frame_start, out)? {
            return Ok(());
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

    /// Byte copy for a frame that starts on a byte and fills whole bytes.
    ///
    /// Big-endian OW still exchanges each pair. A one-bit frame that starts
    /// mid-byte returns false and keeps the bit walk.
    fn copy_aligned(&self, frame_start: u64, out: &mut [u8]) -> Result<bool, CodecError> {
        if !frame_start.is_multiple_of(8) || !self.frame_bits.is_multiple_of(8) {
            return Ok(false);
        }
        let byte_len = self.frame_bits / 8;
        if byte_len != u64::try_from(out.len()).map_err(|_| CodecError::FrameMismatch)? {
            return Ok(false);
        }
        let start = usize::try_from(frame_start / 8).map_err(|_| CodecError::FrameMismatch)?;
        match (self.byte_order, self.pixel_data_vr) {
            (NativeByteOrder::BigEndian, PixelDataVr::Ow) => {
                // `decode_frame` already proved the last swapped byte exists.
                // Every index in this frame is at or below that byte.
                for (index, target) in out.iter_mut().enumerate() {
                    let canonical = start.checked_add(index).ok_or(CodecError::FrameMismatch)?;
                    let Some(byte) = self.value.get(canonical ^ 1) else {
                        return Err(CodecError::InvalidCodestream);
                    };
                    *target = *byte;
                }
            }
            _ => {
                let end = start
                    .checked_add(out.len())
                    .ok_or(CodecError::FrameMismatch)?;
                let Some(bytes) = self.value.get(start..end) else {
                    return Err(CodecError::InvalidCodestream);
                };
                out.copy_from_slice(bytes);
            }
        }
        Ok(true)
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

    fn decoded_output_len(&self, desc: &FrameDesc) -> usize {
        let stored = stored_samples_per_pixel(desc);
        if stored == desc.samples_per_pixel() {
            return desc.output_len();
        }
        // PS3.3 C.7.6.3.1.2: two of the frame's three samples are stored for
        // the two 4:2:2 codes it defines over three samples per pixel. This
        // arm runs only when `stored_samples_per_pixel` answered 2 against
        // Samples per Pixel 3. The product below is therefore bounded by
        // 65535 * 65535 * 2 * 32 bits, far below `u64::MAX`, and it cannot
        // exceed the container length `FrameDesc::new` proved representable
        // on this target.
        let bits = u64::from(desc.rows())
            * u64::from(desc.columns())
            * u64::from(stored)
            * u64::from(desc.bits_allocated());
        usize::try_from(bits.div_ceil(8)).unwrap_or(desc.output_len())
    }

    fn decode(&self, src: &[u8], desc: &FrameDesc, out: &mut [u8]) -> Result<(), CodecError> {
        // A 4:2:2 photometric over any other sample count contradicts the
        // section that defines it, and the refusal must not depend on the
        // buffer the caller happened to bring.
        validate_stored_samples(desc)?;
        let output_len = self.decoded_output_len(desc);
        if out.len() != output_len {
            return Err(CodecError::OutputLength {
                expected: output_len,
                actual: out.len(),
            });
        }
        validate_pixel_format(self.requires_ow, desc)?;
        validate_column_pairs(desc)?;

        let logical_len = output_len;
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

/// Whether the photometric names one of the two 4:2:2 codes PS3.3
/// C.7.6.3.1.2 defines. The one copy of that code list, shared by the
/// stored-sample answer and the refusal below it.
fn photometric_names_a_422_code(photometric_interpretation: &str) -> bool {
    matches!(
        photometric_interpretation,
        "YBR_FULL_422" | "YBR_PARTIAL_422"
    )
}

/// Stored samples on the wire for one pixel of a native frame.
///
/// PS3.3 C.7.6.3.1.2 subsamples the chroma of `YBR_FULL_422` and
/// `YBR_PARTIAL_422` two to one horizontally and stores each pair of pixels
/// as `Y1 Y2 Cb Cr`, so two samples are stored where Samples per Pixel says
/// three. This is the same fact `ocelli_pixel`'s sample layout encodes as
/// its stored-samples-per-pixel answer, which a [`FrameDesc`] cannot reach:
/// the frame carries the photometric as text and carries no Planar
/// Configuration, so the two codes are named here against that section
/// rather than re-derived. `ocelli_pixel` reaches its copy only through
/// `SampleLayout::new`, which refuses a sample count other than the
/// photometric's expected three, so the answer 2 here carries the same
/// condition and `decode` refuses the contradiction rather than re-counting.
fn stored_samples_per_pixel(desc: &FrameDesc) -> u16 {
    if photometric_names_a_422_code(desc.photometric_interpretation())
        && desc.samples_per_pixel() == 3
    {
        2
    } else {
        desc.samples_per_pixel()
    }
}

/// PS3.3 C.7.6.3.1.2 defines the two 4:2:2 codes over three samples per
/// pixel, the rule `ocelli_pixel`'s `SampleLayout::new` applies before its
/// own stored-sample answer is reachable. A header naming one of the two
/// codes with any other Samples per Pixel contradicts the section and is
/// refused rather than re-counted, so the codec's answer and `ocelli_pixel`'s
/// cannot diverge on the input the two copies used to disagree on.
fn validate_stored_samples(desc: &FrameDesc) -> Result<(), CodecError> {
    if photometric_names_a_422_code(desc.photometric_interpretation())
        && desc.samples_per_pixel() != 3
    {
        return Err(CodecError::FrameMismatch);
    }
    Ok(())
}

/// PS3.3 C.7.6.3.1.2 stores a pair of horizontally adjacent pixels as one
/// chroma unit. An odd Columns leaves the last pixel of a row without a
/// partner, and PS3.3 names no rule that joins it to the next row, so the
/// frame is refused rather than silently re-paired.
fn validate_column_pairs(desc: &FrameDesc) -> Result<(), CodecError> {
    if stored_samples_per_pixel(desc) != desc.samples_per_pixel() && desc.columns() % 2 == 1 {
        return Err(CodecError::FrameMismatch);
    }
    Ok(())
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
    // Sized from the stored sample count, not Samples per Pixel, so a native
    // 4:2:2 frame is two samples per pixel (PS3.3 C.7.6.3.1.2). The two are
    // equal for every other photometric.
    u64::from(desc.rows())
        .checked_mul(u64::from(desc.columns()))
        .and_then(|bits| bits.checked_mul(u64::from(stored_samples_per_pixel(desc))))
        .and_then(|bits| bits.checked_mul(u64::from(desc.bits_allocated())))
        .ok_or(CodecError::FrameMismatch)
}
