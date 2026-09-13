//! Checked frame-to-fragment indexing for encapsulated Pixel Data.
//!
//! DICOM PS3.5 Annex A.4 defines Basic Offset Table offsets from the first
//! Fragment Item Tag. DICOM PS3.3 C.7.6.3.1.8 applies the same origin to the
//! Extended Offset Table and requires one Fragment per indexed frame.

use core::ops::Range;

const ITEM_HEADER_BYTES: u64 = 8;

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
    /// A requested zero-based frame is not present.
    #[error("frame index is outside the declared frame count")]
    FrameOutOfRange {
        /// The requested zero-based frame.
        requested: usize,
        /// The positive declared frame count.
        declared: usize,
    },
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
    /// An empty table maps one frame to all Fragments. For multiple frames it
    /// maps one Fragment per frame only when the two counts are equal. It
    /// never guesses a boundary among multiple Fragments.
    ///
    /// # Errors
    ///
    /// Returns a structural error for impossible counts, non-increasing
    /// offsets, or offsets that do not point at Fragment Item Tags.
    pub fn from_basic(
        frame_count: usize,
        offsets: &[u32],
        fragments: &'a [&'a [u8]],
    ) -> Result<Self, FrameIndexError> {
        validate_nonempty(frame_count, fragments)?;
        if offsets.is_empty() {
            let ranges = if frame_count == 1 {
                core::iter::once(0..fragments.len()).collect()
            } else if frame_count == fragments.len() {
                (0..frame_count).map(|frame| frame..frame + 1).collect()
            } else {
                return Err(FrameIndexError::BoundaryUnavailable {
                    declared: frame_count,
                    fragments: fragments.len(),
                });
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
        fragments: &'a [&'a [u8]],
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
        fragments: &'a [&'a [u8]],
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
