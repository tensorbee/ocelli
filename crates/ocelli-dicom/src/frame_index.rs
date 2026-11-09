//! Checked frame-to-fragment indexing for encapsulated Pixel Data.
//!
//! DICOM PS3.5 Annex A.4 defines Basic Offset Table offsets from the first
//! Fragment Item Tag. DICOM PS3.3 C.7.6.3.1.8 applies the same origin to the
//! Extended Offset Table and requires one Fragment per indexed frame.
//!
//! An empty Basic Offset Table gives no boundary. PS3.5 8.2 still requires
//! frames that span Fragments to be read, and A.4.1, A.4.3 and A.4.4 permit
//! the span. F-244 proves each boundary from the codestream's own start and
//! end markers, and refuses a file whose markers do not prove it.

use core::ops::Range;

use dicom_core::Tag;
use dicom_core::value::{PrimitiveValue, Value};

use crate::parse::ParsedDicom;

const ITEM_HEADER_BYTES: u64 = 8;
const PIXEL_DATA: Tag = Tag(0x7FE0, 0x0010);
const EXTENDED_OFFSET_TABLE: Tag = Tag(0x7FE0, 0x0001);
const EXTENDED_OFFSET_TABLE_LENGTHS: Tag = Tag(0x7FE0, 0x0002);
// ISO/IEC 10918-1 and 14495-1 end of image, and ISO/IEC 15444-1 end of
// codestream. The two markers share their bytes.
const END_MARKER: [u8; 2] = [0xFF, 0xD9];
// ISO/IEC 10918-1 and 14495-1 start of image, then the first byte of the
// marker every JPEG and JPEG-LS codestream carries next.
const JPEG_START: [u8; 3] = [0xFF, 0xD8, 0xFF];
// ISO/IEC 15444-1 start of codestream, then the SIZ marker A.5.1 requires
// immediately after it.
const JPEG2000_START: [u8; 4] = [0xFF, 0x4F, 0xFF, 0x51];

/// How a transfer syntax lets one frame's encoded data span Fragments,
/// DICOM PS3.5 A.4.1 to A.4.12.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum FrameDelimiter {
    /// No in-band end of frame. RLE A.4.2 and JPEG XL A.4.12.
    Unmarked,
    /// SOI `FF D8 FF` starts a frame and EOI `FF D9` ends it. JPEG A.4.1 and
    /// JPEG-LS A.4.3.
    JpegMarkers,
    /// SOC and SIZ `FF 4F FF 51` start a frame and EOC `FF D9` ends it.
    /// JPEG 2000 Part 1 and HTJ2K, A.4.4.
    Jpeg2000Markers,
}

impl FrameDelimiter {
    /// The delimiter for an encapsulated transfer syntax UID. A UID with no
    /// row here is `Unmarked`, which never infers a boundary.
    #[must_use]
    pub fn for_transfer_syntax(uid: &str) -> Self {
        match uid {
            // A.4.1 JPEG and A.4.3 JPEG-LS.
            "1.2.840.10008.1.2.4.50"
            | "1.2.840.10008.1.2.4.51"
            | "1.2.840.10008.1.2.4.57"
            | "1.2.840.10008.1.2.4.70"
            | "1.2.840.10008.1.2.4.80"
            | "1.2.840.10008.1.2.4.81" => Self::JpegMarkers,
            // A.4.4 JPEG 2000 Part 1 and HTJ2K. HTJ2K may not span Fragments
            // under A.4.4, and is read leniently by the same markers.
            "1.2.840.10008.1.2.4.90"
            | "1.2.840.10008.1.2.4.91"
            | "1.2.840.10008.1.2.4.201"
            | "1.2.840.10008.1.2.4.202"
            | "1.2.840.10008.1.2.4.203" => Self::Jpeg2000Markers,
            _ => Self::Unmarked,
        }
    }

    /// The bytes a Fragment opening a frame begins with, if the codestream
    /// has an in-band start of frame.
    const fn start(self) -> Option<&'static [u8]> {
        match self {
            Self::Unmarked => None,
            Self::JpegMarkers => Some(&JPEG_START),
            Self::Jpeg2000Markers => Some(&JPEG2000_START),
        }
    }
}

/// Whether the logical suffix through a Fragment closes a frame: the end marker, or
/// with the end marker and the one pad byte PS3.5 A.4 allows for the last
/// Fragment of a frame.
fn ends_frame(fragment: &[u8]) -> bool {
    fragment.ends_with(&END_MARKER)
        || fragment
            .strip_suffix(&[0])
            .is_some_and(|unpadded| unpadded.ends_with(&END_MARKER))
}

/// A structural encapsulated-frame indexing failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum FrameIndexError {
    /// A frame index cannot be built for zero frames.
    #[error("encapsulated frame count is zero")]
    ZeroFrameCount,
    /// No Fragment Values were supplied.
    #[error("encapsulated Pixel Data contains no fragments")]
    NoFragments,
    /// A populated offset table does not have one value per frame.
    #[error("offset table count does not match the declared frame count")]
    OffsetCount {
        /// The positive declared frame count.
        declared: usize,
        /// The table's actual value count.
        actual: usize,
    },
    /// Extended Offset Table Lengths does not have one value per frame.
    #[error("extended length count does not match the declared frame count")]
    LengthCount {
        /// The positive declared frame count.
        declared: usize,
        /// The table's actual value count.
        actual: usize,
    },
    /// The first populated offset is not zero.
    #[error("the first frame offset is not zero")]
    FirstOffsetNotZero(u64),
    /// Frame offsets do not increase strictly.
    #[error("frame offsets are not strictly increasing")]
    NonMonotonicOffsets,
    /// An offset does not point at a supplied Fragment Item Tag.
    #[error("frame offset does not point at a Fragment Item Tag")]
    OffsetNotFragmentBoundary(u64),
    /// Empty BOT evidence cannot prove a multi-frame boundary mapping.
    #[error("an empty Basic Offset Table does not prove frame boundaries")]
    BoundaryUnavailable {
        /// The positive declared frame count.
        declared: usize,
        /// The number of supplied Fragments.
        fragments: usize,
    },
    /// Extended offset tables require exactly one Fragment per frame.
    #[error("Extended Offset Table requires one Fragment per frame")]
    ExtendedFrameFragmentCount {
        /// The positive declared frame count.
        declared: usize,
        /// The number of supplied Fragments.
        fragments: usize,
    },
    /// An extended frame length does not match its Fragment Value and pad.
    #[error("extended frame length does not match its Fragment Value")]
    InvalidExtendedLength {
        /// The zero-based frame carrying the invalid length.
        frame: usize,
        /// The declared encoded frame byte length.
        declared: u64,
        /// The supplied Fragment Value length including any Item pad byte.
        fragment: usize,
    },
    /// A physical Fragment Item Value has no required even-length padding.
    #[error("encapsulated Fragment Value length is odd")]
    OddFragmentLength {
        /// The zero-based Fragment.
        fragment: usize,
        /// The odd physical Item Value length.
        length: usize,
    },
    /// A Fragment length or cumulative Item offset exceeds 64-bit indexing.
    #[error("encapsulated Fragment offsets overflow")]
    FragmentOffsetOverflow,
    /// The checked frame-range reservation could not be allocated.
    #[error("encapsulated frame index allocation failed")]
    AllocationFailed,
    /// A requested zero-based frame is not present.
    #[error("frame index is outside the declared frame count")]
    FrameOutOfRange {
        /// The requested zero-based frame.
        requested: usize,
        /// The positive declared frame count.
        declared: usize,
    },
    /// Pixel Data is absent or is not encapsulated.
    #[error("Pixel Data is absent or not encapsulated")]
    NotEncapsulated,
    /// An Extended Offset Table is present beside a populated Basic Offset
    /// Table, which PS3.5 A.4 does not permit.
    #[error("an Extended Offset Table is present beside a populated Basic Offset Table")]
    ExtendedWithBasicOffsets,
    /// Extended Offset Table or its Lengths are absent, malformed or not
    /// 64-bit unsigned values.
    #[error("the Extended Offset Table or its Lengths cannot be read")]
    ExtendedOffsetTableUnreadable,
}

/// Borrowed Fragment Values grouped into checked frame ranges.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncapsulatedFrameIndex<'a> {
    fragments: Vec<&'a [u8]>,
    ranges: Vec<Range<usize>>,
}

impl<'a> EncapsulatedFrameIndex<'a> {
    /// Build an index from a Basic Offset Table.
    ///
    /// An empty table maps one frame to every Fragment, and equal counts to
    /// one Fragment per frame, with no marker read. With more Fragments than
    /// frames it splits at the start and end markers `delimiter` names, and
    /// only where those markers and the frame count prove every boundary.
    ///
    /// # Errors
    ///
    /// Returns a structural error for impossible counts, non-increasing
    /// offsets, offsets that do not point at Fragment Item Tags, or an empty
    /// table whose boundaries the markers do not prove.
    pub fn from_basic(
        frame_count: usize,
        offsets: &[u32],
        fragments: &[&'a [u8]],
        delimiter: FrameDelimiter,
    ) -> Result<Self, FrameIndexError> {
        validate_nonempty(frame_count, fragments)?;
        if offsets.is_empty() {
            if frame_count > fragments.len() {
                return Err(FrameIndexError::BoundaryUnavailable {
                    declared: frame_count,
                    fragments: fragments.len(),
                });
            }
            let ranges = if frame_count == 1 {
                core::iter::once(0..fragments.len()).collect()
            } else if frame_count == fragments.len() {
                (0..frame_count).map(|frame| frame..frame + 1).collect()
            } else {
                split_at_markers(frame_count, fragments, delimiter)?
            };
            return Ok(Self {
                fragments: fragments.to_vec(),
                ranges,
            });
        }
        if offsets.len() != frame_count {
            return Err(FrameIndexError::OffsetCount {
                declared: frame_count,
                actual: offsets.len(),
            });
        }
        let offsets = offsets
            .iter()
            .map(|offset| u64::from(*offset))
            .collect::<Vec<_>>();
        Self::from_offsets(frame_count, &offsets, fragments)
    }

    /// Build the index for a parsed object's top-level Pixel Data: the
    /// Extended Offset Table when present, the Basic Offset Table otherwise,
    /// and the delimiter of the object's transfer syntax.
    ///
    /// # Errors
    ///
    /// Returns [`FrameIndexError::NotEncapsulated`] when Pixel Data is absent
    /// or native, [`FrameIndexError::ExtendedWithBasicOffsets`] when both
    /// tables carry values, [`FrameIndexError::ExtendedOffsetTableUnreadable`]
    /// when the Extended table or its Lengths cannot be read as 64-bit
    /// unsigned values, and otherwise the refusal of the table it selects.
    pub fn from_parsed(
        parsed: &'a ParsedDicom,
        frame_count: usize,
    ) -> Result<Self, FrameIndexError> {
        // `get` and not `element`: an absent element is an ordinary answer
        // here, and dicom-object's `element` error captures a backtrace,
        // which costs about 4 microseconds per absent tag, measured.
        let object = parsed.object();
        let value = object
            .get(PIXEL_DATA)
            .ok_or(FrameIndexError::NotEncapsulated)?
            .value();
        let (Some(offsets), Some(values)) = (value.offset_table(), value.fragments()) else {
            return Err(FrameIndexError::NotEncapsulated);
        };
        let fragments: Vec<&'a [u8]> = values.iter().map(Vec::as_slice).collect();
        let Some(extended) = object.get(EXTENDED_OFFSET_TABLE) else {
            let delimiter = FrameDelimiter::for_transfer_syntax(parsed.transfer_syntax().uid());
            return Self::from_basic(frame_count, offsets, &fragments, delimiter);
        };
        if !offsets.is_empty() {
            return Err(FrameIndexError::ExtendedWithBasicOffsets);
        }
        let lengths = object
            .get(EXTENDED_OFFSET_TABLE_LENGTHS)
            .ok_or(FrameIndexError::ExtendedOffsetTableUnreadable)?;
        Self::from_extended(
            frame_count,
            unsigned_64(extended.value())?,
            unsigned_64(lengths.value())?,
            &fragments,
        )
    }

    /// Build an index from Extended Offset Table values and lengths.
    ///
    /// DICOM PS3.3 C.7.6.3.1.8 permits this table only when every frame is in
    /// one Fragment. The returned Fragment slice excludes the optional Item
    /// pad byte when the declared compressed frame length is odd.
    ///
    /// # Errors
    ///
    /// Returns a structural error unless offsets, lengths, frame count, and
    /// Fragment boundaries agree exactly.
    pub fn from_extended(
        frame_count: usize,
        offsets: &[u64],
        lengths: &[u64],
        fragments: &[&'a [u8]],
    ) -> Result<Self, FrameIndexError> {
        validate_nonempty(frame_count, fragments)?;
        if offsets.len() != frame_count {
            return Err(FrameIndexError::OffsetCount {
                declared: frame_count,
                actual: offsets.len(),
            });
        }
        if lengths.len() != frame_count {
            return Err(FrameIndexError::LengthCount {
                declared: frame_count,
                actual: lengths.len(),
            });
        }
        if fragments.len() != frame_count {
            return Err(FrameIndexError::ExtendedFrameFragmentCount {
                declared: frame_count,
                fragments: fragments.len(),
            });
        }
        validate_offsets(offsets)?;
        let fragment_offsets = fragment_offsets(fragments)?;
        for (offset, expected) in offsets.iter().zip(fragment_offsets.iter()) {
            if offset != expected {
                return Err(FrameIndexError::OffsetNotFragmentBoundary(*offset));
            }
        }

        let mut selected = Vec::with_capacity(frame_count);
        for (frame, (fragment, declared)) in fragments.iter().zip(lengths.iter()).enumerate() {
            let declared_usize =
                usize::try_from(*declared).map_err(|_| FrameIndexError::InvalidExtendedLength {
                    frame,
                    declared: *declared,
                    fragment: fragment.len(),
                })?;
            let includes_one_pad = !declared.is_multiple_of(2)
                && declared_usize.checked_add(1) == Some(fragment.len());
            if declared_usize != fragment.len() && !includes_one_pad {
                return Err(FrameIndexError::InvalidExtendedLength {
                    frame,
                    declared: *declared,
                    fragment: fragment.len(),
                });
            }
            let Some(encoded) = fragment.get(..declared_usize) else {
                return Err(FrameIndexError::InvalidExtendedLength {
                    frame,
                    declared: *declared,
                    fragment: fragment.len(),
                });
            };
            selected.push(encoded);
        }
        Ok(Self {
            fragments: selected,
            ranges: (0..frame_count).map(|frame| frame..frame + 1).collect(),
        })
    }

    /// Borrow the ordered Fragment Values for one zero-based frame.
    ///
    /// # Errors
    ///
    /// Returns [`FrameIndexError::FrameOutOfRange`] without clamping.
    pub fn frame_fragments(&self, frame: usize) -> Result<&[&'a [u8]], FrameIndexError> {
        let Some(range) = self.ranges.get(frame).cloned() else {
            return Err(FrameIndexError::FrameOutOfRange {
                requested: frame,
                declared: self.ranges.len(),
            });
        };
        self.fragments
            .get(range)
            .ok_or(FrameIndexError::FragmentOffsetOverflow)
    }

    fn from_offsets(
        frame_count: usize,
        offsets: &[u64],
        fragments: &[&'a [u8]],
    ) -> Result<Self, FrameIndexError> {
        validate_offsets(offsets)?;
        let fragment_offsets = fragment_offsets(fragments)?;
        let mut starts = Vec::with_capacity(frame_count);
        for offset in offsets {
            let start = fragment_offsets
                .binary_search(offset)
                .map_err(|_| FrameIndexError::OffsetNotFragmentBoundary(*offset))?;
            starts.push(start);
        }
        let mut ranges = Vec::with_capacity(frame_count);
        for frame in 0..frame_count {
            let Some(start) = starts.get(frame).copied() else {
                return Err(FrameIndexError::FragmentOffsetOverflow);
            };
            let end = starts.get(frame + 1).copied().unwrap_or(fragments.len());
            ranges.push(start..end);
        }
        Ok(Self {
            fragments: fragments.to_vec(),
            ranges,
        })
    }
}

/// Group Fragments into frames by their codestream markers, the plan's rule
/// for an empty Basic Offset Table with more Fragments than frames. Frame 0
/// starts at Fragment 0. Each frame ends at the first Fragment at or after its
/// start that closes a frame. The next frame starts at the following Fragment,
/// which must open one. Exactly `frame_count` frames must be found, the last
/// ending at the last Fragment.
fn split_at_markers(
    frame_count: usize,
    fragments: &[&[u8]],
    delimiter: FrameDelimiter,
) -> Result<Vec<Range<usize>>, FrameIndexError> {
    let unavailable = FrameIndexError::BoundaryUnavailable {
        declared: frame_count,
        fragments: fragments.len(),
    };
    let Some(start_marker) = delimiter.start() else {
        return Err(unavailable);
    };
    let mut ranges = Vec::new();
    ranges
        .try_reserve_exact(frame_count)
        .map_err(|_| FrameIndexError::AllocationFailed)?;
    let mut start = 0;
    while let Some(rest) = fragments.get(start..).filter(|rest| !rest.is_empty()) {
        if ranges.len() == frame_count {
            return Err(unavailable);
        }
        let opens = rest
            .iter()
            .flat_map(|fragment| fragment.iter().copied())
            .take(start_marker.len())
            .eq(start_marker.iter().copied());
        if start > 0 && !opens {
            return Err(unavailable);
        }
        // PS3.5 A.4 allows a two-byte Fragment to divide a codec marker.
        // Keep only the last three logical bytes, including an optional pad,
        // and inspect at most three bytes per Fragment without joining it.
        let mut suffix = [0_u8; 3];
        let Some(last) = rest.iter().position(|fragment| {
            for byte in fragment.iter().rev().take(3).rev() {
                suffix.rotate_left(1);
                suffix[2] = *byte;
            }
            ends_frame(&suffix)
        }) else {
            return Err(unavailable);
        };
        let end = start + last + 1;
        ranges.push(start..end);
        start = end;
    }
    if ranges.len() == frame_count {
        Ok(ranges)
    } else {
        Err(unavailable)
    }
}

/// Values of an `OV` element, PS3.5 6.2, which dicom-parser reads as 64-bit
/// unsigned integers.
fn unsigned_64<I, P>(value: &Value<I, P>) -> Result<&[u64], FrameIndexError> {
    match value {
        Value::Primitive(PrimitiveValue::U64(values)) => Ok(values.as_slice()),
        _ => Err(FrameIndexError::ExtendedOffsetTableUnreadable),
    }
}

fn validate_nonempty(frame_count: usize, fragments: &[&[u8]]) -> Result<(), FrameIndexError> {
    if frame_count == 0 {
        return Err(FrameIndexError::ZeroFrameCount);
    }
    if fragments.is_empty() {
        return Err(FrameIndexError::NoFragments);
    }
    for (fragment, value) in fragments.iter().enumerate() {
        if !value.len().is_multiple_of(2) {
            return Err(FrameIndexError::OddFragmentLength {
                fragment,
                length: value.len(),
            });
        }
    }
    Ok(())
}

fn validate_offsets(offsets: &[u64]) -> Result<(), FrameIndexError> {
    let mut previous = None;
    for offset in offsets {
        if previous.is_none() && *offset != 0 {
            return Err(FrameIndexError::FirstOffsetNotZero(*offset));
        }
        if previous.is_some_and(|prior| *offset <= prior) {
            return Err(FrameIndexError::NonMonotonicOffsets);
        }
        previous = Some(*offset);
    }
    Ok(())
}

fn fragment_offsets(fragments: &[&[u8]]) -> Result<Vec<u64>, FrameIndexError> {
    let mut offsets = Vec::with_capacity(fragments.len());
    let mut next = 0_u64;
    for fragment in fragments {
        offsets.push(next);
        let length =
            u64::try_from(fragment.len()).map_err(|_| FrameIndexError::FragmentOffsetOverflow)?;
        next = next
            .checked_add(ITEM_HEADER_BYTES)
            .and_then(|offset| offset.checked_add(length))
            .ok_or(FrameIndexError::FragmentOffsetOverflow)?;
    }
    Ok(offsets)
}
