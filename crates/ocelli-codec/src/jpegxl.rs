//! JPEG XL transfer-syntax adapters, DICOM PS3.5 A.4.12.
//!
//! `.110` is lossless only. `.112` renders the codestream. `.111` is a known
//! syntax and no decoder is registered for it. `jxl-jbr` 0.2.1
//! `src/reconstruct.rs:457` panics with `expect("is_last not found")` in the
//! DHT branch of JPEG reconstruction, so no path here calls `reconstruct_jpeg`
//! or any other `jxl-jbr` entry point. Integer grids are read sample by
//! sample. A float grid is quantised by [`quantise_lossy_sample`].
//! `ImageStream::write_to_buffer` is not used, because a 12-bit stored value
//! would be rescaled into the full `u16` range.
//!
//! Allocation is bounded by [`ALLOC_LIMIT_NUMERATOR`] times the descriptor's
//! output length. The numerator is the measured peak ratio, doubled. The
//! measurement is recorded on that constant. The encoded ICC profile size a
//! codestream may declare is bounded separately, before any parsing, by
//! [`crate::jpegxl_icc`].

use std::sync::Arc;

use jxl_oxide::frame::Encoding;
use jxl_oxide::image::BitDepth;
use jxl_oxide::image::color::{ColourEncoding, ColourSpace, TransferFunction};
use jxl_oxide::{AllocTracker, InitializeResult, JxlImage};
use jxl_render::ImageBuffer;

use crate::{
    Capability, CodecError, DecodePhotometricInterpretation, DecodeSampleLayout, Decoder,
    FrameDesc, PixelDataVr, PixelRepresentation, Registry, RegistryError,
};

const JPEGXL_LOSSLESS_UID: &str = "1.2.840.10008.1.2.4.110";
/// JPEG XL JPEG Recompression. Known, and left unregistered.
///
/// `jxl-jbr` 0.2.1 `src/reconstruct.rs:457` is an unconditional
/// `expect("is_last not found")` in the DHT branch. Under `panic = "abort"`
/// that ends the process. [`register_jpegxl_decoders`] does not claim this UID.
const JPEGXL_RECOMPRESSION_UID: &str = "1.2.840.10008.1.2.4.111";
const JPEGXL_UID: &str = "1.2.840.10008.1.2.4.112";
const LOSSLESS_UIDS: &[&str] = &[JPEGXL_LOSSLESS_UID];
const GENERAL_UIDS: &[&str] = &[JPEGXL_UID];

/// Worst tracked allocation, in bytes, divided by [`FrameDesc::output_len`],
/// over the tracked fixtures, then doubled.
///
/// `AllocTracker` keeps `bytes_left` private, so the peak is the smallest
/// limit for which the real decode path stops returning
/// [`CodecError::DecoderFailure`]. Measured on 2026-11-05 over the tracked
/// fixtures: the suite fails up to a numerator of 12 and passes from 13, and
/// the binding case is the 64 by 96 lossy 12-bit `jpegxl_lossy_12.jxl`. The 8
/// by 16 container fails up to 5 and passes from 6. A decode that exceeds the
/// doubled limit returns [`CodecError::DecoderFailure`] and does not write
/// `out`.
const ALLOC_LIMIT_NUMERATOR: usize = 26;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    /// `.110`, lossless only.
    Lossless,
    /// `.112`, the general form.
    General,
}

/// One exact DICOM JPEG XL transfer-syntax adapter.
pub struct JpegXlDecoder {
    mode: Mode,
}

impl JpegXlDecoder {
    /// Decoder for JPEG XL Lossless UID `.110`.
    #[must_use]
    pub const fn lossless() -> Self {
        Self {
            mode: Mode::Lossless,
        }
    }

    /// Decoder for general JPEG XL UID `.112`.
    #[must_use]
    pub const fn general() -> Self {
        Self {
            mode: Mode::General,
        }
    }
}

/// Register the JPEG XL Lossless and JPEG XL adapters atomically.
///
/// JPEG XL JPEG Recompression stays [`Capability::KnownUnavailable`]. The
/// reason is `jxl-jbr` 0.2.1 `src/reconstruct.rs:457`, the same shape as
/// Deflated Explicit VR Little Endian staying unclaimed in
/// [`crate::register_native_and_rle_decoders`]. Preflight still requires the
/// UID to be in the catalogue, and it runs before either adapter is
/// registered. No byte of a `.111` frame is passed to `jxl-oxide`.
///
/// # Errors
///
/// Returns the first registry collision or declaration error. An error leaves
/// every capability unchanged.
pub fn register_jpegxl_decoders(registry: &mut Registry) -> Result<(), RegistryError> {
    for uid in [JPEGXL_LOSSLESS_UID, JPEGXL_RECOMPRESSION_UID, JPEGXL_UID] {
        match registry.capability(uid) {
            Capability::Available => return Err(RegistryError::AlreadyRegistered { uid }),
            Capability::KnownUnavailable => {}
            Capability::Unknown => return Err(RegistryError::UnknownTransferSyntax { uid }),
        }
    }
    registry.register(Arc::new(JpegXlDecoder::lossless()))?;
    registry.register(Arc::new(JpegXlDecoder::general()))
}

/// Quantise one lossy float sample into the stored domain.
///
/// `q = floor(v * (2^BitsStored - 1) + 0.5)`, then clamped to
/// `[0, 2^BitsStored - 1]`. The product is computed in `f64` from the `f32`,
/// so the multiplier for Bits Stored at most 16 is exact. Half rounds away
/// from zero on a non-negative operand. A non-finite sample refuses.
///
/// This is the decoder output contract. PS3.5 does not assign a DICOM stored
/// value to a float render, and PS3.5 8.2.15 says the codestream's bit depth
/// is the stored precision.
///
/// # Errors
///
/// [`CodecError::DecoderFailure`] when `sample` is not finite.
/// [`CodecError::UnsupportedPixelFormat`] when `bits_stored` is 0 or above 16.
pub fn quantise_lossy_sample(sample: f32, bits_stored: u16) -> Result<u32, CodecError> {
    let bits = u32::from(bits_stored);
    if bits == 0 || bits > 16 {
        return Err(CodecError::UnsupportedPixelFormat);
    }
    if !sample.is_finite() {
        return Err(CodecError::DecoderFailure);
    }
    let max = 1u32
        .checked_shl(bits)
        .ok_or(CodecError::DecoderFailure)?
        .checked_sub(1)
        .ok_or(CodecError::DecoderFailure)?;
    let scaled = f64::from(sample) * f64::from(max) + 0.5;
    if !scaled.is_finite() {
        return Err(CodecError::DecoderFailure);
    }
    if scaled <= 0.0 {
        return Ok(0);
    }
    let max_f = f64::from(max);
    if scaled >= max_f + 1.0 {
        return Ok(max);
    }
    // Greatest integer `n` with `n <= scaled`. `scaled` is in `(0, max + 1)`.
    let mut lo = 0u32;
    let mut hi = max;
    while lo < hi {
        let mid = lo + (hi - lo) / 2 + 1;
        if f64::from(mid) <= scaled {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    Ok(lo)
}

impl Decoder for JpegXlDecoder {
    fn transfer_syntaxes(&self) -> &'static [&'static str] {
        match self.mode {
            Mode::Lossless => LOSSLESS_UIDS,
            Mode::General => GENERAL_UIDS,
        }
    }

    fn decode_photometric_interpretation(
        &self,
        desc: &FrameDesc,
    ) -> DecodePhotometricInterpretation {
        if desc.samples_per_pixel() == 3 {
            DecodePhotometricInterpretation::Rgb
        } else {
            DecodePhotometricInterpretation::Preserved
        }
    }

    fn decode_sample_layout(&self, _desc: &FrameDesc) -> DecodeSampleLayout {
        DecodeSampleLayout::Interleaved
    }

    fn decode(&self, src: &[u8], desc: &FrameDesc, out: &mut [u8]) -> Result<(), CodecError> {
        if out.len() != desc.output_len() {
            return Err(CodecError::OutputLength {
                expected: desc.output_len(),
                actual: out.len(),
            });
        }
        validate_descriptor(desc)?;
        let limit = allocation_limit(desc)?;
        self.decode_limited(src, desc, out, limit)
    }
}

fn allocation_limit(desc: &FrameDesc) -> Result<usize, CodecError> {
    desc.output_len()
        .checked_mul(ALLOC_LIMIT_NUMERATOR)
        .ok_or(CodecError::DecoderFailure)
}

impl JpegXlDecoder {
    fn decode_limited(
        &self,
        src: &[u8],
        desc: &FrameDesc,
        out: &mut [u8],
        limit: usize,
    ) -> Result<(), CodecError> {
        // Before `jxl-oxide` parses anything: a header declaring an encoded
        // ICC profile the codestream cannot supply would have the dependency
        // size its buffer from the declaration alone.
        if crate::jpegxl_icc::declared_icc_exceeds_codestream(src) {
            return Err(CodecError::InvalidCodestream);
        }
        let (mut image, header_end) = read_header(src, limit)?;
        validate_header(self.mode, &image, desc)?;
        let consumed = load_rest(&mut image, src, header_end)?;
        let trailing = src.get(consumed..).ok_or(CodecError::DecoderFailure)?;
        refuse_trailing(trailing, consumed)?;
        if image.num_loaded_keyframes() != 1 {
            return Err(CodecError::FrameMismatch);
        }
        if self.mode == Mode::Lossless {
            let frame = image.frame_header(0).ok_or(CodecError::FrameMismatch)?;
            if frame.encoding == Encoding::VarDct {
                return Err(CodecError::FrameMismatch);
            }
        }
        let rendered = image.render_frame(0).map_err(map_jxl)?;
        write_rendered(rendered.color_channels(), desc, out)
    }
}

fn validate_descriptor(desc: &FrameDesc) -> Result<(), CodecError> {
    if desc.pixel_data_vr() != PixelDataVr::Ob {
        return Err(CodecError::UnsupportedPixelFormat);
    }
    if desc.pixel_representation() != PixelRepresentation::Unsigned {
        return Err(CodecError::UnsupportedPixelFormat);
    }
    if !matches!(desc.bits_allocated(), 8 | 16) {
        return Err(CodecError::UnsupportedPixelFormat);
    }
    Ok(())
}

/// Parse the image header without feeding the frame bytes that follow it.
///
/// A declared size is refused before `render_frame`, and before the frame
/// grids are allocated. The returned offset is the number of source bytes the
/// container parser consumed to reach a header.
///
/// The parser keeps a short signature until the next byte shows whether the
/// stream is a bare codestream or a box. A feed that consumes nothing is
/// presented again with one more byte, which is what `JxlImage::read` does
/// with its own buffer. Bytes past the header are not fed, so `try_init` does
/// not load a frame grid.
fn read_header(src: &[u8], limit: usize) -> Result<(JxlImage, usize), CodecError> {
    let tracker = AllocTracker::with_limit(limit);
    let mut uninit = JxlImage::builder().alloc_tracker(tracker).build_uninit();
    let mut start = 0usize;
    let mut end = 0usize;
    loop {
        if end == start {
            if end >= src.len() {
                return Err(CodecError::InvalidCodestream);
            }
            end = end.checked_add(1).ok_or(CodecError::DecoderFailure)?;
        }
        let window = src.get(start..end).ok_or(CodecError::InvalidCodestream)?;
        let consumed = uninit.feed_bytes(window).map_err(map_jxl)?;
        if consumed > window.len() {
            return Err(CodecError::DecoderFailure);
        }
        start = start
            .checked_add(consumed)
            .ok_or(CodecError::DecoderFailure)?;
        match uninit.try_init().map_err(map_jxl)? {
            InitializeResult::NeedMoreData(next) => {
                uninit = next;
                if consumed == 0 {
                    if end >= src.len() {
                        return Err(CodecError::InvalidCodestream);
                    }
                    end = end.checked_add(1).ok_or(CodecError::DecoderFailure)?;
                }
            }
            InitializeResult::Initialized(image) => return Ok((image, start)),
        }
    }
}

fn validate_header(mode: Mode, image: &JxlImage, desc: &FrameDesc) -> Result<(), CodecError> {
    let header = image.image_header();
    if header.metadata.orientation != 1 {
        return Err(CodecError::UnsupportedPixelFormat);
    }
    match header.metadata.bit_depth {
        BitDepth::FloatSample { .. } => return Err(CodecError::UnsupportedPixelFormat),
        BitDepth::IntegerSample { bits_per_sample } => {
            if bits_per_sample != u32::from(desc.bits_stored()) {
                return Err(CodecError::FrameMismatch);
            }
        }
    }
    if !header.metadata.ec_info.is_empty() {
        return Err(CodecError::UnsupportedPixelFormat);
    }
    match &header.metadata.colour_encoding {
        ColourEncoding::IccProfile(_) => return Err(CodecError::UnsupportedPixelFormat),
        ColourEncoding::Enum(encoding) => {
            // PS3.3 C.7.6.3.1.2 gives XYB no conversion matrix, and an unknown
            // colour space has no definition to convert with. The adapter's
            // output contract is RGB or preserved greyscale, so anything else
            // refuses before a sample is written.
            if !matches!(encoding.colour_space, ColourSpace::Rgb | ColourSpace::Grey) {
                return Err(CodecError::UnsupportedPixelFormat);
            }
            match encoding.tf {
                TransferFunction::Pq | TransferFunction::Hlg => {
                    return Err(CodecError::UnsupportedPixelFormat);
                }
                _ => {}
            }
        }
    }
    if header.size.width != u32::from(desc.columns())
        || header.size.height != u32::from(desc.rows())
    {
        return Err(CodecError::FrameMismatch);
    }
    let channels: u16 = if header.metadata.grayscale() { 1 } else { 3 };
    if channels != desc.samples_per_pixel() {
        return Err(CodecError::FrameMismatch);
    }
    if mode == Mode::Lossless && header.metadata.xyb_encoded {
        return Err(CodecError::FrameMismatch);
    }
    Ok(())
}

fn load_rest(image: &mut JxlImage, src: &[u8], mut offset: usize) -> Result<usize, CodecError> {
    loop {
        if image.is_loading_done() {
            return Ok(offset);
        }
        if offset >= src.len() {
            image.finalize().map_err(map_jxl)?;
            if image.is_loading_done() {
                return Ok(offset);
            }
            return Err(CodecError::InvalidCodestream);
        }
        let rest = src.get(offset..).ok_or(CodecError::InvalidCodestream)?;
        let consumed = image.feed_bytes(rest).map_err(map_jxl)?;
        if consumed == 0 {
            image.finalize().map_err(map_jxl)?;
            if image.is_loading_done() {
                return Ok(offset);
            }
            return Err(CodecError::InvalidCodestream);
        }
        offset = offset
            .checked_add(consumed)
            .ok_or(CodecError::DecoderFailure)?;
    }
}

/// One trailing `0x00` is the DICOM even-length pad when the consumed length
/// is odd. Anything else is trailing data. A bare codestream parser may
/// consume bytes past the codestream, and those bytes are not detectable here.
fn refuse_trailing(trailing: &[u8], consumed: usize) -> Result<(), CodecError> {
    if trailing.is_empty() || (consumed % 2 == 1 && trailing == [0]) {
        Ok(())
    } else {
        Err(CodecError::TrailingData)
    }
}

fn write_rendered(
    channels: &[ImageBuffer],
    desc: &FrameDesc,
    out: &mut [u8],
) -> Result<(), CodecError> {
    let samples = usize::from(desc.samples_per_pixel());
    if channels.len() != samples {
        return Err(CodecError::FrameMismatch);
    }
    let rows = usize::from(desc.rows());
    let columns = usize::from(desc.columns());
    let mut bytes = Vec::new();
    bytes
        .try_reserve(desc.output_len())
        .map_err(|_| CodecError::DecoderFailure)?;
    let float_grid = channels
        .first()
        .is_some_and(|channel| matches!(channel, ImageBuffer::F32(_)));
    for row in 0..rows {
        for column in 0..columns {
            for channel in channels {
                let sample = read_sample(channel, row, column, columns, rows, float_grid, desc)?;
                push_sample(&mut bytes, sample, desc.bits_allocated())?;
            }
        }
    }
    if bytes.len() != out.len() {
        return Err(CodecError::FrameMismatch);
    }
    out.copy_from_slice(&bytes);
    Ok(())
}

fn read_sample(
    channel: &ImageBuffer,
    row: usize,
    column: usize,
    columns: usize,
    rows: usize,
    float_grid: bool,
    desc: &FrameDesc,
) -> Result<u32, CodecError> {
    if channel.width() < columns || channel.height() < rows {
        return Err(CodecError::FrameMismatch);
    }
    match channel {
        ImageBuffer::F32(grid) => {
            if !float_grid {
                return Err(CodecError::UnsupportedPixelFormat);
            }
            let line = grid.try_get_row(row).ok_or(CodecError::FrameMismatch)?;
            let sample = line.get(column).copied().ok_or(CodecError::FrameMismatch)?;
            quantise_lossy_sample(sample, desc.bits_stored())
        }
        ImageBuffer::I16(grid) => {
            if float_grid {
                return Err(CodecError::UnsupportedPixelFormat);
            }
            let line = grid.try_get_row(row).ok_or(CodecError::FrameMismatch)?;
            let sample = line.get(column).copied().ok_or(CodecError::FrameMismatch)?;
            stored_integer(i32::from(sample), desc)
        }
        ImageBuffer::I32(grid) => {
            if float_grid {
                return Err(CodecError::UnsupportedPixelFormat);
            }
            let line = grid.try_get_row(row).ok_or(CodecError::FrameMismatch)?;
            let sample = line.get(column).copied().ok_or(CodecError::FrameMismatch)?;
            stored_integer(sample, desc)
        }
    }
}

fn stored_integer(sample: i32, desc: &FrameDesc) -> Result<u32, CodecError> {
    if sample < 0 {
        return Err(CodecError::UnsupportedPixelFormat);
    }
    let value = u32::try_from(sample).map_err(|_| CodecError::UnsupportedPixelFormat)?;
    let max = stored_max(desc.bits_stored())?;
    if value > max {
        return Err(CodecError::UnsupportedPixelFormat);
    }
    Ok(value)
}

fn stored_max(bits_stored: u16) -> Result<u32, CodecError> {
    let bits = u32::from(bits_stored);
    if bits == 0 || bits > 16 {
        return Err(CodecError::UnsupportedPixelFormat);
    }
    1u32.checked_shl(bits)
        .ok_or(CodecError::DecoderFailure)?
        .checked_sub(1)
        .ok_or(CodecError::DecoderFailure)
}

fn push_sample(bytes: &mut Vec<u8>, sample: u32, bits_allocated: u16) -> Result<(), CodecError> {
    match bits_allocated {
        8 => {
            let byte = u8::try_from(sample).map_err(|_| CodecError::UnsupportedPixelFormat)?;
            bytes.push(byte);
        }
        16 => {
            let word = u16::try_from(sample).map_err(|_| CodecError::UnsupportedPixelFormat)?;
            bytes.extend_from_slice(&word.to_le_bytes());
        }
        _ => return Err(CodecError::UnsupportedPixelFormat),
    }
    Ok(())
}

fn map_jxl(err: Box<dyn std::error::Error + Send + Sync>) -> CodecError {
    if allocation_limited(err.as_ref()) {
        CodecError::DecoderFailure
    } else {
        CodecError::InvalidCodestream
    }
}

fn allocation_limited(err: &(dyn std::error::Error + 'static)) -> bool {
    if err.to_string().contains("failed to allocate") {
        return true;
    }
    match err.source() {
        Some(source) => allocation_limited(source),
        None => false,
    }
}
