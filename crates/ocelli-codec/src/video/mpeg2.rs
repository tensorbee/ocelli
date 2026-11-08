//! MPEG-2 video headers and access units.
//!
//! Every syntax table and field width here is from Rec. ITU-T H.262 (02/2000),
//! which is ISO/IEC 13818-2:2000, the clause named beside each parser. The
//! 2012 edition asks for an ITU account to download, so a change it made to
//! these headers would not be seen here.

use super::{
    ColourDescription, ColourSource, FrameSize, VideoError,
    bits::{Bits, Unit, units},
    table::{AccessUnit, StreamInfo},
};

/// Table 6-1 start code values.
mod code {
    pub(super) const PICTURE: u8 = 0x00;
    pub(super) const SLICE_LAST: u8 = 0xAF;
    pub(super) const SEQUENCE_HEADER: u8 = 0xB3;
    pub(super) const EXTENSION: u8 = 0xB5;
    pub(super) const GROUP: u8 = 0xB8;
    pub(super) const SYSTEM_FIRST: u8 = 0xB9;
}

/// Table 6-2 extension_start_code_identifier values.
mod extension {
    pub(super) const SEQUENCE: u32 = 0b0001;
    pub(super) const SEQUENCE_DISPLAY: u32 = 0b0010;
    pub(super) const PICTURE_CODING: u32 = 0b1000;
}

/// Table 6-12, picture_coding_type 001 is an intra-coded picture.
const INTRA: u32 = 0b001;
/// Table 6-14, picture_structure 11 is a frame picture.
const FRAME_PICTURE: u32 = 0b11;
/// 6.3.9, temporal_reference is a 10-bit count modulo 1024.
const TEMPORAL_REFERENCE_MODULUS: i64 = 1024;

const STRUCTURE: &str = "MPEG-2 video";

fn payload<'a>(es: &'a [u8], unit: &Unit) -> &'a [u8] {
    es.get(unit.payload..unit.end).unwrap_or_default()
}

/// The sequence header and its extensions, 6.2.2.1, 6.2.2.3 and 6.2.2.4.
#[derive(Debug, Default)]
struct Sequence {
    horizontal: u32,
    vertical: u32,
    extension: Option<(bool, u32, u32)>,
    colour: Option<(u8, u8, u8)>,
}

impl Sequence {
    fn info(&self) -> Result<StreamInfo, VideoError> {
        let (progressive_sequence, horizontal_extension, vertical_extension) =
            self.extension.ok_or(VideoError::Unsupported {
                feature: "an ISO/IEC 11172-2 stream with no sequence_extension",
            })?;
        // 6.3.3: horizontal_size and vertical_size take their two high bits
        // from the extension. mb_width is ( horizontal_size + 15 ) / 16, and
        // mb_height is ( vertical_size + 15 ) / 16 for a progressive sequence
        // and 2 * ( ( vertical_size + 31 ) / 32 ) for frame pictures of an
        // interlaced one.
        let width = horizontal_extension << 12 | self.horizontal;
        let height = vertical_extension << 12 | self.vertical;
        let mb_height = if progressive_sequence {
            height.div_ceil(16)
        } else {
            2 * height.div_ceil(32)
        };
        Ok(StreamInfo {
            codec_string: None,
            coded_size: FrameSize {
                width: width.div_ceil(16) * 16,
                height: mb_height * 16,
            },
            visible_size: FrameSize { width, height },
            colour: self
                .colour
                .map(|(primaries, transfer, matrix)| ColourDescription {
                    code_point_source: ColourSource::SequenceDisplayExtension,
                    range_source: ColourSource::SequenceDisplayExtension,
                    colour_primaries: primaries.into(),
                    transfer_characteristics: transfer.into(),
                    matrix_coefficients: matrix.into(),
                    // H.262 has no range flag. Table 6-9 relates the samples to
                    // the analogue signals by Y = 219 * E'Y + 16 and
                    // Cb = 224 * E'PB + 128, the studio range.
                    full_range: false,
                }),
            code_points_signalled: self.colour.is_some(),
            interlaced: !progressive_sequence,
        })
    }
}

/// 6.2.2.1, sequence_header after its start code.
fn sequence_header(data: &[u8], sequence: &mut Sequence) -> Result<(), VideoError> {
    let mut bits = Bits::plain(data, "MPEG-2 sequence header");
    sequence.horizontal = bits.bits(12)?;
    sequence.vertical = bits.bits(12)?;
    if sequence.horizontal == 0 || sequence.vertical == 0 {
        // 6.3.3: neither size value shall be zero.
        return Err(bits.malformed("a zero horizontal_size_value or vertical_size_value"));
    }
    Ok(())
}

/// 6.2.2.3 and 6.2.2.4, the two sequence level extensions this module
/// reads. Any other extension is left alone.
fn sequence_extension(data: &[u8], sequence: &mut Sequence) -> Result<(), VideoError> {
    let mut bits = Bits::plain(data, "MPEG-2 extension");
    match bits.bits(4)? {
        extension::SEQUENCE => {
            bits.byte()?; // profile_and_level_indication
            let progressive_sequence = bits.flag()?;
            bits.bits(2)?; // chroma_format
            let horizontal = bits.bits(2)?;
            let vertical = bits.bits(2)?;
            sequence.extension = Some((progressive_sequence, horizontal, vertical));
        }
        extension::SEQUENCE_DISPLAY => {
            bits.bits(3)?; // video_format
            if bits.flag()? {
                sequence.colour = Some((bits.byte()?, bits.byte()?, bits.byte()?));
            }
        }
        _ => {}
    }
    Ok(())
}

/// One coded picture of the current access unit.
struct Picture {
    temporal_reference: i64,
    intra: bool,
    /// Whether the picture coding extension has been read.
    coding_extension: bool,
}

/// Splits an MPEG-2 video elementary stream into access units with their
/// display order, and reads the stream description from its first sequence
/// header and extensions.
///
/// An access unit runs from a sequence header, a group of pictures header or
/// a picture header that follows a picture's slices, to the next one. Its
/// display position is its group, counted from the group of pictures headers
/// of 6.2.2.6, and its temporal_reference within the group, which 6.3.9 says
/// counts up by one per frame in display order modulo 1024.
pub(crate) fn access_units(es: &[u8]) -> Result<(Vec<AccessUnit>, StreamInfo), VideoError> {
    let mut sequence = Sequence::default();
    // The description is the first sequence header's, 6.1.1.6 requiring every
    // repeat to carry the same values.
    let mut first_picture_seen = false;
    let mut found = Vec::new();
    let mut open: Option<usize> = None;
    let mut anchor: Option<usize> = None;
    let mut picture: Option<Picture> = None;
    let mut slices_seen = false;
    let mut group = 0u32;
    // The previous picture's temporal_reference in this group and its
    // unwrapped display count.
    let mut previous: Option<(i64, i64)> = None;
    let mut last_end = 0usize;
    // Which header the next extension follows.
    let mut after_sequence_header = false;
    for unit in units(es) {
        let Some(value) = es.get(unit.payload).copied() else {
            continue;
        };
        let body = payload(es, &unit).get(1..).unwrap_or_default();
        let starts_unit = matches!(value, code::SEQUENCE_HEADER | code::GROUP | code::PICTURE);
        if starts_unit && slices_seen {
            close(
                &mut found,
                &mut open,
                &mut anchor,
                &mut picture,
                &mut previous,
                group,
                last_end,
            )?;
            slices_seen = false;
        }
        if open.is_none() {
            open = Some(unit.start);
        }
        match value {
            code::SEQUENCE_HEADER => {
                if !first_picture_seen {
                    sequence_header(body, &mut sequence)?;
                }
                after_sequence_header = true;
            }
            code::EXTENSION => {
                if after_sequence_header {
                    if !first_picture_seen {
                        sequence_extension(body, &mut sequence)?;
                    }
                } else if let Some(current) = picture.as_mut() {
                    let mut bits = Bits::plain(body, "MPEG-2 picture coding extension");
                    if bits.bits(4)? == extension::PICTURE_CODING {
                        bits.bits(16)?; // f_code[0][0] to f_code[1][1]
                        bits.bits(2)?; // intra_dc_precision
                        if bits.bits(2)? != FRAME_PICTURE {
                            // A field picture is half a displayed frame, and
                            // pairing two fields is not done here.
                            return Err(VideoError::Unsupported {
                                feature: "an MPEG-2 stream of field pictures outside MP4",
                            });
                        }
                        current.coding_extension = true;
                    }
                }
            }
            code::GROUP => {
                // A group header before the first picture opens group 0.
                if picture.is_some() || !found.is_empty() {
                    group = group.checked_add(1).ok_or(VideoError::Malformed {
                        structure: STRUCTURE,
                        reason: "too many groups of pictures",
                    })?;
                }
                previous = None;
                after_sequence_header = false;
            }
            code::PICTURE => {
                if sequence.horizontal == 0 {
                    return Err(VideoError::Malformed {
                        structure: STRUCTURE,
                        reason: "a picture before any sequence header",
                    });
                }
                first_picture_seen = true;
                after_sequence_header = false;
                // 6.2.3: temporal_reference u(10), picture_coding_type u(3).
                let mut bits = Bits::plain(body, "MPEG-2 picture header");
                let temporal_reference = i64::from(bits.bits(10)?);
                let intra = bits.bits(3)? == INTRA;
                picture = Some(Picture {
                    temporal_reference,
                    intra,
                    coding_extension: false,
                });
                anchor = Some(unit.payload - 3);
            }
            0x01..=code::SLICE_LAST => {
                if picture
                    .as_ref()
                    .is_none_or(|current| !current.coding_extension)
                {
                    // 6.3.1: in a stream with a sequence_extension every
                    // picture_header is followed immediately by a
                    // picture_coding_extension.
                    return Err(VideoError::Malformed {
                        structure: STRUCTURE,
                        reason: "a slice with no picture header and picture coding extension",
                    });
                }
                slices_seen = true;
            }
            code::SYSTEM_FIRST..=0xFF => {
                return Err(VideoError::Malformed {
                    structure: STRUCTURE,
                    reason: "a system start code inside a video elementary stream",
                });
            }
            _ => {}
        }
        last_end = unit.end;
    }
    if picture.is_some() && slices_seen {
        close(
            &mut found,
            &mut open,
            &mut anchor,
            &mut picture,
            &mut previous,
            group,
            es.len(),
        )?;
    }
    Ok((found, sequence.info()?))
}

fn close(
    found: &mut Vec<AccessUnit>,
    open: &mut Option<usize>,
    anchor: &mut Option<usize>,
    picture: &mut Option<Picture>,
    previous: &mut Option<(i64, i64)>,
    group: u32,
    end: usize,
) -> Result<(), VideoError> {
    let missing = VideoError::Malformed {
        structure: STRUCTURE,
        reason: "an access unit with no picture",
    };
    let start = open.take().ok_or(missing)?;
    let current = picture.take().ok_or(missing)?;
    let anchor = anchor.take().ok_or(missing)?;
    // 6.3.9: the display count advances by temporal_reference modulo 1024.
    // Decode order is never more than half the modulus from display order,
    // so the difference is taken in -512 to 511.
    let count = match *previous {
        None => current.temporal_reference,
        Some((reference, count)) => {
            let half = TEMPORAL_REFERENCE_MODULUS / 2;
            let step = (current.temporal_reference - reference + half)
                .rem_euclid(TEMPORAL_REFERENCE_MODULUS)
                - half;
            count + step
        }
    };
    *previous = Some((current.temporal_reference, count));
    found.push(AccessUnit {
        range: start..end,
        anchor,
        order: (group, count),
        // An intra-coded picture needs no other picture to decode.
        random_access: current.intra,
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{Sequence, access_units};
    use crate::video::{FrameSize, VideoError, bits::Writer};

    /// 6.3.3: for an interlaced sequence, progressive_sequence 0, mb_height
    /// for frame pictures is 2 * ( ( vertical_size + 31 ) / 32 ). A 64 by 72
    /// sequence is 2 * 3 = 6 macroblock rows, coded 96 high, where a
    /// progressive one would be ( 72 + 15 ) / 16 = 5 rows and 80.
    #[test]
    fn interlaced_mb_height_follows_6_3_3() -> Result<(), VideoError> {
        let mut sequence = Sequence {
            horizontal: 64,
            vertical: 72,
            extension: Some((false, 0, 0)),
            colour: None,
        };
        let interlaced = sequence.info()?;
        assert_eq!(
            interlaced.coded_size,
            FrameSize {
                width: 64,
                height: 96
            }
        );
        assert!(interlaced.interlaced);
        sequence.extension = Some((true, 0, 0));
        assert_eq!(
            sequence.info()?.coded_size,
            FrameSize {
                width: 64,
                height: 80
            }
        );
        Ok(())
    }

    /// A start code, Table 6-1, then the bits.
    fn unit(code: u8, writer: &Writer) -> Vec<u8> {
        let mut out = vec![0, 0, 1, code];
        out.extend(writer.bytes());
        out
    }

    /// One picture: sequence_header 6.2.2.1 for 16 by 16 at frame_rate_code 3,
    /// sequence_extension 6.2.2.3, picture_header 6.2.3 of an I picture,
    /// picture_coding_extension 6.2.3.1 with the given picture_structure, and
    /// one slice.
    fn picture(picture_structure: u64) -> Vec<u8> {
        let mut header = Writer::default();
        header
            .u(12, 16)
            .u(12, 16)
            .u(4, 1)
            .u(4, 3)
            .u(18, 1)
            .flag(true)
            .u(10, 1);
        header.flag(false).flag(false).flag(false);
        let mut extension = Writer::default();
        extension
            .u(4, 0b0001)
            .u(8, 0x48)
            .flag(true)
            .u(2, 1)
            .u(2, 0)
            .u(2, 0);
        extension
            .u(12, 0)
            .flag(true)
            .u(8, 0)
            .flag(false)
            .u(2, 0)
            .u(5, 0);
        let mut picture = Writer::default();
        picture.u(10, 0).u(3, 0b001).u(16, 0xFFFF).flag(false);
        let mut coding = Writer::default();
        coding
            .u(4, 0b1000)
            .u(16, 0xFFFF)
            .u(2, 0)
            .u(2, picture_structure);
        // top_field_first through chroma_420_type clear, progressive_frame
        // set, composite_display_flag clear.
        coding.u(8, 0).flag(true).flag(false);
        let mut slice = Writer::default();
        slice.u(5, 1).flag(false).u(8, 0x80);
        [
            unit(0xB3, &header),
            unit(0xB5, &extension),
            unit(0x00, &picture),
            unit(0xB5, &coding),
            unit(0x01, &slice),
        ]
        .concat()
    }

    /// Table 6-14: picture_structure 11 is a frame picture and reads as one
    /// access unit. 01, a top field, is half a frame, and is refused by name
    /// rather than listed as a frame.
    #[test]
    fn a_field_picture_is_refused_by_name() -> Result<(), VideoError> {
        let (found, _) = access_units(&picture(0b11))?;
        assert_eq!(found.len(), 1);
        assert_eq!(
            access_units(&picture(0b01)).err(),
            Some(VideoError::Unsupported {
                feature: "an MPEG-2 stream of field pictures outside MP4",
            })
        );
        Ok(())
    }
}
