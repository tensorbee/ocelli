//! F-047 stack scroll.
//!
//! Stored order is the order of the `FrameKey` list. Image position is not
//! an argument, so this file cannot sort by it. Prefetch radius is one.
//! A miss returns no samples. `scroll_by` clamps.

use ocelli_cache::{DecodedFrame, FrameKey, ImageCache, SeriesId};
use ocelli_pixel::{
    ImageDimensions, ImageOrientationPatient, ImagePlane, ImagePositionPatient,
    PhotometricInterpretation, PixelRepresentation, PixelSpacing, SampleLayout, StoredBits,
    StoredPixelDescription,
};
use ocelli_viewport::scroll::{ScrollError, StackSequence};

fn key(instance: u32) -> FrameKey {
    FrameKey::new(SeriesId(1), instance, 0)
}

fn frame(samples: Vec<u8>) -> Result<DecodedFrame, ocelli_pixel::PixelError> {
    let layout = SampleLayout::new(1, PhotometricInterpretation::Monochrome2, None)?;
    let bits = StoredBits::new(8, 8, 7, PixelRepresentation::Unsigned)?;
    let description = StoredPixelDescription::new(ImageDimensions::new(1, 1)?, layout, bits);
    let plane = ImagePlane::new(
        ImagePositionPatient::new([0.0, 0.0, 0.0])?,
        ImageOrientationPatient::new([1.0, 0.0, 0.0], [0.0, 1.0, 0.0])?,
        PixelSpacing::new(1.0, 1.0)?,
        ImageDimensions::new(1, 1)?,
    )?;
    Ok(DecodedFrame::new(description, plane, samples))
}

#[test]
fn stored_order_is_not_sorted_by_key() {
    // Instance numbers 2, 0, 1 would sort as 0, 1, 2. Scroll follows the
    // list. A position sort of z = 1, 2, 0 would be a different order, and
    // this type has no position to read.
    let keys = [key(2), key(0), key(1)];
    let mut sequence = StackSequence::new(&keys);
    assert_eq!(sequence.index(), Some(0));
    assert_eq!(sequence.displayed_key(), Some(key(2)));
    sequence.scroll_by(1);
    assert_eq!(sequence.index(), Some(1));
    assert_eq!(sequence.displayed_key(), Some(key(0)));
    sequence.scroll_by(1);
    assert_eq!(sequence.index(), Some(2));
    assert_eq!(sequence.displayed_key(), Some(key(1)));
}

#[test]
fn prefetch_radius_is_one() {
    let keys = [key(10), key(20), key(30)];
    let mut sequence = StackSequence::new(&keys);
    assert_eq!(sequence.displayed_key(), Some(key(10)));
    assert_eq!(sequence.prefetch_keys(), [None, Some(key(20))]);
    sequence.scroll_by(1);
    assert_eq!(sequence.displayed_key(), Some(key(20)));
    assert_eq!(sequence.prefetch_keys(), [Some(key(10)), Some(key(30))]);
    sequence.scroll_by(1);
    assert_eq!(sequence.displayed_key(), Some(key(30)));
    assert_eq!(sequence.prefetch_keys(), [Some(key(20)), None]);
}

#[test]
fn a_miss_returns_no_neighbour_bytes() -> Result<(), ocelli_pixel::PixelError> {
    let keys = [key(10), key(20), key(30)];
    let mut sequence = StackSequence::new(&keys);
    sequence.scroll_by(1);
    let mut cache = ImageCache::new(64);
    let admitted = cache.insert(keys[0], frame(vec![1, 2, 3])?);
    assert!(admitted.is_empty());
    let admitted = cache.insert(keys[2], frame(vec![4, 5, 6])?);
    assert!(admitted.is_empty());
    let samples = sequence.samples_for_upload(&mut cache);
    assert!(matches!(samples, Err(ScrollError::NotResident)));
    Ok(())
}

#[test]
fn a_resident_frame_returns_its_own_samples() -> Result<(), ocelli_pixel::PixelError> {
    let keys = [key(10), key(20), key(30)];
    let mut sequence = StackSequence::new(&keys);
    sequence.scroll_by(1);
    let mut cache = ImageCache::new(64);
    let admitted = cache.insert(keys[1], frame(vec![7, 8, 9])?);
    assert!(admitted.is_empty());
    let samples = sequence.samples_for_upload(&mut cache);
    assert!(samples.is_ok());
    match samples {
        Ok(bytes) => assert_eq!(bytes, &[7, 8, 9]),
        Err(_) => unreachable!("asserted immediately above"),
    }
    Ok(())
}

#[test]
fn scroll_by_clamps_and_an_empty_sequence_is_empty() -> Result<(), ocelli_pixel::PixelError> {
    let keys = [key(10), key(20), key(30)];
    let mut sequence = StackSequence::new(&keys);
    sequence.scroll_by(-1);
    assert_eq!(sequence.index(), Some(0));
    sequence.scroll_by(5);
    assert_eq!(sequence.index(), Some(2));
    sequence.scroll_by(1);
    assert_eq!(sequence.index(), Some(2));
    sequence.scroll_by(-10);
    assert_eq!(sequence.index(), Some(0));
    assert_eq!(sequence.displayed_key(), Some(key(10)));

    let empty = StackSequence::new(&[]);
    assert_eq!(empty.index(), None);
    assert_eq!(empty.displayed_key(), None);
    assert_eq!(empty.prefetch_keys(), [None, None]);
    let mut cache = ImageCache::new(64);
    assert!(matches!(
        empty.samples_for_upload(&mut cache),
        Err(ScrollError::Empty)
    ));
    let mut empty = empty;
    empty.scroll_by(3);
    assert!(matches!(
        empty.samples_for_upload(&mut cache),
        Err(ScrollError::Empty)
    ));
    Ok(())
}
