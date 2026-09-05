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
        prop_assert_eq!(forward.informative_pixels, backward.informative_pixels);

        let (Ok(fm), Ok(rm)) = (f.signed_mean_diff(), r.signed_mean_diff()) else {
            return Err(TestCaseError::fail("a signed mean did not compute"));
        };
        prop_assert_eq!(
            fm.to_bits(),
            (-rm).to_bits(),
            "the signed mean negates exactly, bit for bit, and does not merely \
             agree to an epsilon"
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
    }
}
