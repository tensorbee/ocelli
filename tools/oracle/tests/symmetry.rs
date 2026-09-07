//! The difference statistics are symmetric under swapping the two sides,
//! except the signed mean, which negates exactly.
//!
//! This is the property that says the comparator has no preferred side. It
//! matters because the attribution ladder DOES have a preferred side, by
//! decision: rung 5 attributes an unexplained pixel divergence to ours. That
//! asymmetry belongs to the ladder and not to the arithmetic, and a
//! measurement that quietly favoured one direction would make the ladder's
//! default look like evidence.
//!
//! HLD 25.1's own listing in section 25 is a `proptest!` round-trip, and
//! `proptest` is already a workspace dev-dependency for that reason.

use ocelli_oracle::frame::{ChannelSet, Frame, Rect, difference};
use proptest::prelude::*;

/// Small enough that a shrunk counterexample is readable, large enough to
/// carry a letterbox and several differing rows and columns.
const SIDE: u32 = 8;
const PIXELS: usize = 64;

proptest! {
    #[test]
    fn the_statistics_are_symmetric_except_the_signed_mean(
        left in prop::collection::vec(any::<u8>(), PIXELS),
        right in prop::collection::vec(any::<u8>(), PIXELS),
    ) {
        let Ok(a) = Frame::from_monochrome(SIDE, SIDE, &left) else {
            return Err(TestCaseError::fail("the left frame did not build"));
        };
        let Ok(b) = Frame::from_monochrome(SIDE, SIDE, &right) else {
            return Err(TestCaseError::fail("the right frame did not build"));
        };
        let rect = Rect::full(SIDE, SIDE);

        let Ok(forward) = difference(&a, &b, rect.clone(), ChannelSet::Monochrome) else {
            return Err(TestCaseError::fail("the forward difference did not compute"));
        };
        let Ok(backward) = difference(&b, &a, rect, ChannelSet::Monochrome) else {
            return Err(TestCaseError::fail("the backward difference did not compute"));
        };

        let (Some(f), Some(r)) = (forward.full.channel(0), backward.full.channel(0)) else {
            return Err(TestCaseError::fail("a channel is missing"));
        };

        prop_assert_eq!(f.histogram(), r.histogram(), "the absolute histogram is the same both ways");
        prop_assert_eq!(f.pixels(), r.pixels());
        prop_assert_eq!(f.max_abs_diff(), r.max_abs_diff());
        prop_assert_eq!(forward.rows_touched, backward.rows_touched);
        prop_assert_eq!(forward.columns_touched, backward.columns_touched);
        prop_assert_eq!(&forward.image_rows_touched, &backward.image_rows_touched);
        prop_assert_eq!(&forward.image_columns_touched, &backward.image_columns_touched);
        prop_assert_eq!(&forward.background_rows_touched, &backward.background_rows_touched);
        prop_assert_eq!(&forward.background_columns_touched, &backward.background_columns_touched);
        prop_assert_eq!(forward.informative_pixels, backward.informative_pixels);

        let (Ok(fm), Ok(rm)) = (f.signed_mean_diff(), r.signed_mean_diff()) else {
            return Err(TestCaseError::fail("a signed mean did not compute"));
        };
        // Bit-for-bit, with the ONE exception IEEE 754 requires: negating zero
        // gives negative zero, whose sign bit differs, so `0.0f64.to_bits()`
        // is 0 and `(-0.0f64).to_bits()` is 1 << 63. Two frames that differ
        // nowhere have a signed mean of exactly zero, and that is the most
        // ordinary input there is rather than an edge case.
        //
        // **Found by proptest failing intermittently**, during the S03 sprint
        // review's remediation and not before, because the generator has to
        // land on two identical frames to reach it. A test that fails on one
        // run in many is worse than one that fails always, so this is written
        // down rather than reseeded away. The property meant is that the two
        // directions are exact negations, and `0.0 == -0.0` is true in IEEE
        // 754, so comparing the values expresses it and comparing the bits
        // overstates it.
        // `+ 0.0` is the normalisation and not padding: it maps negative zero
        // to positive zero and leaves every other value alone, so the
        // comparison stays a bit comparison on integers. `float_cmp` is denied
        // at the workspace and writing `fm == -rm` here would need an
        // `#[allow]`, which this project does not grant to move on.
        prop_assert_eq!(
            (fm + 0.0).to_bits(),
            (-rm + 0.0).to_bits(),
            "the signed mean negates exactly, bit for bit once signed zero is \
             normalised, and does not merely agree to an epsilon"
        );
    }
}

proptest! {
    /// A frame compared against itself has no difference at all, whatever it
    /// contains. That is the identity exercise stated as a property, and it is
    /// the case the corpus-scale identity run cannot distinguish from a
    /// comparator that always answers zero. This one can, because the property
    /// above shows the same code reporting non-zero on other input.
    #[test]
    fn a_frame_against_itself_has_no_difference(
        bytes in prop::collection::vec(any::<u8>(), PIXELS),
    ) {
        let Ok(frame) = Frame::from_monochrome(SIDE, SIDE, &bytes) else {
            return Err(TestCaseError::fail("the frame did not build"));
        };
        let Ok(diff) = difference(&frame, &frame, Rect::full(SIDE, SIDE), ChannelSet::Monochrome)
        else {
            return Err(TestCaseError::fail("the difference did not compute"));
        };
        let Some(stats) = diff.full.channel(0) else {
            return Err(TestCaseError::fail("a channel is missing"));
        };
        prop_assert_eq!(stats.max_abs_diff(), 0);
        prop_assert_eq!(stats.count_at(0), u64::from(SIDE) * u64::from(SIDE));
        prop_assert_eq!(diff.rows_touched, 0);
        prop_assert_eq!(diff.columns_touched, 0);
        prop_assert_eq!((diff.image_x, diff.image_y), (0, 0));
        prop_assert!(diff.image_rows_touched.is_empty());
        prop_assert!(diff.image_columns_touched.is_empty());
        prop_assert!(diff.background_rows_touched.is_empty());
        prop_assert!(diff.background_columns_touched.is_empty());
    }
}
