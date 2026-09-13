//! Hand-computed encapsulated frame boundary fixtures for F-019.
//!
//! DICOM PS3.5 Annex A.4 measures Basic Offset Table entries from the first
//! byte of the first Fragment Item Tag. Each Fragment contributes its 8-byte
//! Item Tag and Length header plus its even Value length. PS3.3 C.7.6.3.1.8
//! gives the Extended Offset Table the same origin and restricts each indexed
//! frame to one Fragment.

use ocelli_dicom::{EncapsulatedFrameIndex, FrameIndexError};

const FIRST: &[u8] = &[1, 2, 3, 4];
const SECOND: &[u8] = &[5, 6, 7, 8, 9, 10];
const THIRD: &[u8] = &[11, 12, 13, 14, 15, 16, 17, 18];

fn fragments() -> [&'static [u8]; 3] {
    [FIRST, SECOND, THIRD]
}

#[test]
fn basic_offsets_select_one_frame_spanning_two_fragments() {
    // Fragment Item Tag offsets are 0, 8 + 4 = 12, and
    // 12 + 8 + 6 = 26 bytes. Frame two starts at the third Item Tag.
    let fragments = fragments();
    let index = EncapsulatedFrameIndex::from_basic(2, &[0, 26], &fragments);
    assert!(index.is_ok());
    let index = index.unwrap_or_else(|_| unreachable!());
    assert_eq!(index.frame_fragments(0), Ok(&fragments[..2]));
    assert_eq!(index.frame_fragments(1), Ok(&fragments[2..]));
}

#[test]
fn empty_basic_table_uses_only_provable_boundaries() {
    let fragments = fragments();
    let single =
        EncapsulatedFrameIndex::from_basic(1, &[], &fragments).unwrap_or_else(|_| unreachable!());
    assert_eq!(single.frame_fragments(0), Ok(&fragments[..]));

    let one_per_frame =
        EncapsulatedFrameIndex::from_basic(3, &[], &fragments).unwrap_or_else(|_| unreachable!());
    assert_eq!(one_per_frame.frame_fragments(1), Ok(&fragments[1..2]));

    assert_eq!(
        EncapsulatedFrameIndex::from_basic(2, &[], &fragments).err(),
        Some(FrameIndexError::BoundaryUnavailable {
            declared: 2,
            fragments: 3,
        })
    );
}

#[test]
fn extended_offsets_and_lengths_select_exact_unpadded_frame_bytes() {
    // Each frame is one Fragment under PS3.3 C.7.6.3.1.8. The third encoded
    // frame is seven bytes plus one trailing Item pad byte, so its table length
    // is 7 while the Fragment Value remains even-length.
    let fragments = fragments();
    let index = EncapsulatedFrameIndex::from_extended(3, &[0, 12, 26], &[4, 6, 7], &fragments);
    assert!(index.is_ok());
    let index = index.unwrap_or_else(|_| unreachable!());
    assert_eq!(index.frame_fragments(0), Ok(&[FIRST][..]));
    assert_eq!(index.frame_fragments(1), Ok(&[SECOND][..]));
    let Some(third_without_pad) = THIRD.get(..7) else {
        unreachable!();
    };
    assert_eq!(
        index.frame_fragments(2),
        Ok(core::slice::from_ref(&third_without_pad))
    );
}

#[test]
fn offset_tables_refuse_truncation_non_monotonicity_and_non_boundaries() {
    let fragments = fragments();
    assert_eq!(
        EncapsulatedFrameIndex::from_basic(2, &[12, 26], &fragments).err(),
        Some(FrameIndexError::FirstOffsetNotZero(12))
    );
    assert_eq!(
        EncapsulatedFrameIndex::from_basic(2, &[0], &fragments).err(),
        Some(FrameIndexError::OffsetCount {
            declared: 2,
            actual: 1,
        })
    );
    assert_eq!(
        EncapsulatedFrameIndex::from_basic(2, &[0, 0], &fragments).err(),
        Some(FrameIndexError::NonMonotonicOffsets)
    );
    assert_eq!(
        EncapsulatedFrameIndex::from_basic(2, &[0, 25], &fragments).err(),
        Some(FrameIndexError::OffsetNotFragmentBoundary(25))
    );
    assert_eq!(
        EncapsulatedFrameIndex::from_basic(2, &[0, 40], &fragments).err(),
        Some(FrameIndexError::OffsetNotFragmentBoundary(40))
    );
    assert_eq!(
        EncapsulatedFrameIndex::from_extended(3, &[0, 12], &[4, 6, 7], &fragments).err(),
        Some(FrameIndexError::OffsetCount {
            declared: 3,
            actual: 2,
        })
    );
    assert_eq!(
        EncapsulatedFrameIndex::from_extended(3, &[0, 12, 28], &[4, 6, 7], &fragments).err(),
        Some(FrameIndexError::OffsetNotFragmentBoundary(28))
    );
}

#[test]
fn extended_lengths_and_final_bound_are_checked() {
    let fragments = fragments();
    assert_eq!(
        EncapsulatedFrameIndex::from_extended(3, &[0, 12, 26], &[4, 6], &fragments).err(),
        Some(FrameIndexError::LengthCount {
            declared: 3,
            actual: 2,
        })
    );
    assert_eq!(
        EncapsulatedFrameIndex::from_extended(3, &[0, 12, 26], &[4, 6, 9], &fragments).err(),
        Some(FrameIndexError::InvalidExtendedLength {
            frame: 2,
            declared: 9,
            fragment: 8,
        })
    );
    assert_eq!(
        EncapsulatedFrameIndex::from_extended(3, &[0, 12, 26], &[4, 6, 6], &fragments).err(),
        Some(FrameIndexError::InvalidExtendedLength {
            frame: 2,
            declared: 6,
            fragment: 8,
        })
    );
    assert_eq!(
        EncapsulatedFrameIndex::from_extended(2, &[0, 12], &[4, 6], &fragments).err(),
        Some(FrameIndexError::ExtendedFrameFragmentCount {
            declared: 2,
            fragments: 3,
        })
    );
}

#[test]
fn zero_frames_and_missing_fragments_are_refused() {
    let fragments = fragments();
    assert_eq!(
        EncapsulatedFrameIndex::from_basic(0, &[], &fragments).err(),
        Some(FrameIndexError::ZeroFrameCount)
    );
    assert_eq!(
        EncapsulatedFrameIndex::from_basic(1, &[], &[]).err(),
        Some(FrameIndexError::NoFragments)
    );
}

#[test]
fn odd_physical_fragment_values_are_refused_on_both_table_paths() {
    let odd_fragment: &[u8] = &[1, 2, 3];
    let fragments = [odd_fragment];
    assert_eq!(
        EncapsulatedFrameIndex::from_basic(1, &[], &fragments).err(),
        Some(FrameIndexError::OddFragmentLength {
            fragment: 0,
            length: 3,
        })
    );
    assert_eq!(
        EncapsulatedFrameIndex::from_extended(1, &[0], &[3], &fragments).err(),
        Some(FrameIndexError::OddFragmentLength {
            fragment: 0,
            length: 3,
        })
    );
}

#[test]
fn odd_extended_length_requires_exactly_one_physical_pad_byte() {
    let physical_fragment: &[u8] = &[1, 2, 3, 0];
    let fragments = [physical_fragment];
    let index = EncapsulatedFrameIndex::from_extended(1, &[0], &[3], &fragments)
        .unwrap_or_else(|_| unreachable!());
    let Some(encoded_without_pad) = physical_fragment.get(..3) else {
        unreachable!();
    };
    assert_eq!(
        index.frame_fragments(0),
        Ok(core::slice::from_ref(&encoded_without_pad))
    );

    let too_long: &[u8] = &[1, 2, 3, 4, 5, 0, 0, 0];
    assert_eq!(
        EncapsulatedFrameIndex::from_extended(1, &[0], &[5], &[too_long]).err(),
        Some(FrameIndexError::InvalidExtendedLength {
            frame: 0,
            declared: 5,
            fragment: 8,
        })
    );
}

#[test]
fn out_of_range_frame_identifies_the_declared_count() {
    let fragments = fragments();
    let index = EncapsulatedFrameIndex::from_basic(2, &[0, 26], &fragments)
        .unwrap_or_else(|_| unreachable!());
    assert_eq!(
        index.frame_fragments(2),
        Err(FrameIndexError::FrameOutOfRange {
            requested: 2,
            declared: 2,
        })
    );
}
