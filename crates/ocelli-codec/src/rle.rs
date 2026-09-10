//! Allocation-free DICOM RLE Lossless frame decoding.

use crate::{CodecError, DecodeSampleLayout, Decoder, FrameDesc, PixelDataVr, PixelRepresentation};

const RLE_LOSSLESS: &str = "1.2.840.10008.1.2.5";
const RLE_SYNTAXES: &[&str] = &[RLE_LOSSLESS];
const HEADER_LEN: usize = 64;
const MAX_SEGMENTS: usize = 15;

/// Stateless DICOM RLE Lossless decoder.
pub struct RleDecoder;

impl Decoder for RleDecoder {
    fn transfer_syntaxes(&self) -> &'static [&'static str] {
        RLE_SYNTAXES
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
        if desc.pixel_data_vr() != PixelDataVr::Ob || !is_supported_descriptor(desc) {
            return Err(CodecError::UnsupportedPixelFormat);
        }

        let bytes_per_sample = usize::from(desc.bits_allocated() / 8);
        let samples_per_pixel = usize::from(desc.samples_per_pixel());
        let expected_segments = samples_per_pixel
            .checked_mul(bytes_per_sample)
            .ok_or(CodecError::FrameMismatch)?;
        let segments = parse_header(src, expected_segments)?;
        let rows = usize::from(desc.rows());
        let columns = usize::from(desc.columns());

        for bounds in segments.iter().take(expected_segments) {
            let segment = src
                .get(bounds.start..bounds.end)
                .ok_or(CodecError::InvalidCodestream)?;
            walk_segment(segment, rows, columns, |_, _| {})?;
        }

        for (plane, bounds) in segments.iter().take(expected_segments).enumerate() {
            let sample = plane / bytes_per_sample;
            let byte_in_sample = bytes_per_sample - 1 - plane % bytes_per_sample;
            let segment = src
                .get(bounds.start..bounds.end)
                .ok_or(CodecError::InvalidCodestream)?;
            walk_segment(segment, rows, columns, |pixel, value| {
                let output_index =
                    (pixel * samples_per_pixel + sample) * bytes_per_sample + byte_in_sample;
                if let Some(target) = out.get_mut(output_index) {
                    *target = value;
                }
            })?;
        }
        Ok(())
    }
}

fn is_supported_descriptor(desc: &FrameDesc) -> bool {
    matches!(
        (
            desc.photometric_interpretation(),
            desc.samples_per_pixel(),
            desc.bits_allocated(),
            desc.pixel_representation(),
        ),
        ("MONOCHROME1" | "MONOCHROME2", 1, 8 | 16, _)
            | ("PALETTE COLOR", 1, 8 | 16, PixelRepresentation::Unsigned)
            | ("RGB" | "YBR_FULL", 3, 8 | 16, PixelRepresentation::Unsigned)
    )
}

#[derive(Clone, Copy)]
struct SegmentBounds {
    start: usize,
    end: usize,
}

fn parse_header(
    src: &[u8],
    expected_segments: usize,
) -> Result<[SegmentBounds; MAX_SEGMENTS], CodecError> {
    if src.len() < HEADER_LEN {
        return Err(CodecError::InvalidCodestream);
    }
    let declared = read_u32_le(src.get(0..4).ok_or(CodecError::InvalidCodestream)?)?;
    let declared = usize::try_from(declared).map_err(|_| CodecError::InvalidCodestream)?;
    if declared != expected_segments {
        return Err(CodecError::FrameMismatch);
    }
    if declared == 0 || declared > MAX_SEGMENTS {
        return Err(CodecError::InvalidCodestream);
    }

    let empty = SegmentBounds { start: 0, end: 0 };
    let mut segments = [empty; MAX_SEGMENTS];
    let mut previous = HEADER_LEN;
    for index in 0..declared {
        let field = 4 + index * 4;
        let offset = read_u32_le(
            src.get(field..field + 4)
                .ok_or(CodecError::InvalidCodestream)?,
        )?;
        let offset = usize::try_from(offset).map_err(|_| CodecError::InvalidCodestream)?;
        if (index == 0 && offset != HEADER_LEN)
            || offset < HEADER_LEN
            || offset >= src.len()
            || (index > 0 && offset <= previous)
            || !offset.is_multiple_of(2)
        {
            return Err(CodecError::InvalidCodestream);
        }
        if index > 0 {
            let previous_segment = segments
                .get_mut(index - 1)
                .ok_or(CodecError::InvalidCodestream)?;
            previous_segment.end = offset;
        }
        let segment = segments
            .get_mut(index)
            .ok_or(CodecError::InvalidCodestream)?;
        segment.start = offset;
        previous = offset;
    }
    let final_segment = segments
        .get_mut(declared - 1)
        .ok_or(CodecError::InvalidCodestream)?;
    final_segment.end = src.len();

    for index in declared..MAX_SEGMENTS {
        let field = 4 + index * 4;
        if read_u32_le(
            src.get(field..field + 4)
                .ok_or(CodecError::InvalidCodestream)?,
        )? != 0
        {
            return Err(CodecError::InvalidCodestream);
        }
    }
    if segments
        .iter()
        .take(declared)
        .any(|bounds| bounds.end <= bounds.start || !(bounds.end - bounds.start).is_multiple_of(2))
    {
        return Err(CodecError::InvalidCodestream);
    }
    Ok(segments)
}

fn read_u32_le(bytes: &[u8]) -> Result<u32, CodecError> {
    let array = <[u8; 4]>::try_from(bytes).map_err(|_| CodecError::InvalidCodestream)?;
    Ok(u32::from_le_bytes(array))
}

fn walk_segment(
    segment: &[u8],
    rows: usize,
    columns: usize,
    mut write: impl FnMut(usize, u8),
) -> Result<(), CodecError> {
    let mut source_index = 0_usize;
    for row in 0..rows {
        let mut column = 0_usize;
        while column < columns {
            let control = *segment
                .get(source_index)
                .ok_or(CodecError::InvalidCodestream)?;
            source_index += 1;
            match control {
                0..=127 => {
                    let count = usize::from(control) + 1;
                    let end = source_index
                        .checked_add(count)
                        .ok_or(CodecError::InvalidCodestream)?;
                    let values = segment
                        .get(source_index..end)
                        .ok_or(CodecError::InvalidCodestream)?;
                    if column + count > columns {
                        return Err(CodecError::InvalidCodestream);
                    }
                    for (offset, value) in values.iter().copied().enumerate() {
                        write(row * columns + column + offset, value);
                    }
                    column += count;
                    source_index = end;
                }
                129..=255 => {
                    let count = 257_usize - usize::from(control);
                    let value = *segment
                        .get(source_index)
                        .ok_or(CodecError::InvalidCodestream)?;
                    source_index += 1;
                    if column + count > columns {
                        return Err(CodecError::InvalidCodestream);
                    }
                    for offset in 0..count {
                        write(row * columns + column + offset, value);
                    }
                    column += count;
                }
                128 => {}
            }
        }
    }

    match segment.get(source_index..) {
        Some([]) if source_index.is_multiple_of(2) => Ok(()),
        Some([0]) if !source_index.is_multiple_of(2) => Ok(()),
        Some(_) => Err(CodecError::TrailingData),
        None => Err(CodecError::InvalidCodestream),
    }
}
