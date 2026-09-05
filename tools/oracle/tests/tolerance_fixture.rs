//! HLD 25.1's class-one predicate, at each of its three boundaries, plus the
//! bias bullet added in S03.
//!
//! Every expected number here is hand-computed from the transcribed text of
//! `docs/hld/22-testing-and-tolerance.md` section 25.1 and NOT from running
//! the comparator. HLD 27.2 R2: an agent asked to test a function will assert
//! what it does rather than what it should do, so the counts below are worked
//! out from the quoted rule, and the frame size is chosen so that 0.1% of the
//! frame is an integer.
//!
//! The rule, verbatim from 25.1:
//!
//! > **Monochrome 16-bit (CT, MR, CR, DR):** maximum absolute difference <= 1
//! > LSB on at least 99.9% of pixels, zero pixels differing by more than 2.
//!
//! and the bullet the operator added through this story's design plan:
//!
//! > **Systematic bias, monochrome:** signed mean difference over the
//! > informative region within 0.1 of one display code, evaluated only where
//! > input identity, declared parameters and geometry already agree.
//!
//! **"informative region" and not "image rectangle".** The bullet said the
//! latter when it was written and the sprint review's second pass changed it,
//! because a pixel clipped to the same extreme on both sides cannot express a
//! divergence and counting it in the denominator hides one. This header quoted
//! the superseded wording until the fourth pass. The two regions coincide in
//! the fixtures below, which is stated at `verdict`, so nothing here can
//! measure the difference. The test that does is
//! `the_bias_bound_is_fed_the_informative_region_and_not_the_rectangle` in
//! `tools/oracle/src/attribution.rs`.
//!
//! "1 LSB" is one 8-bit display code. The reference emits RGBA8 canvas frames
//! and nothing else, so the 16-bit reading is not evaluable against this
//! instrument at all. That is decision 1 of this story's design round.
//!
//! **No `unwrap`, no `expect` and no `panic!`.** Each test returns a `Result`
//! and the harness fails it on `Err`, which is the shape HLD section 23's
//! denials leave available.

use std::error::Error;

use ocelli_oracle::frame::{ChannelSet, ChannelStats, Frame, Rect, difference};
use ocelli_oracle::tolerance;

type Outcome = Result<(), Box<dyn Error>>;

/// 100 by 100 is 10000 pixels, so 0.1% is exactly 10 pixels and 99.9% is
/// exactly 9990. Every count below is an integer for that reason, and a frame
/// size where 0.1% was fractional would have hidden which way the boundary
/// rounds.
const SIDE: u32 = 100;
const PIXELS: usize = 10_000;

/// A flat mid-grey reference, and a candidate that is the same frame with a
/// declared list of per-pixel deltas applied in raster order from the top
/// left. Mid-grey so that a delta of plus or minus three cannot clip at either
/// end of the 8-bit range and quietly become a smaller delta.
const BASE: u8 = 128;

fn pair(deltas: &[(usize, i16)]) -> Result<(Frame, Frame), Box<dyn Error>> {
    let reference = vec![BASE; PIXELS];
    let mut candidate = reference.clone();
    for (index, delta) in deltas {
        let value = u8::try_from(i16::from(BASE) + delta)?;
        let slot = candidate
            .get_mut(*index)
            .ok_or("the fixture placed a delta outside the frame")?;
        *slot = value;
    }
    Ok((
        Frame::from_monochrome(SIDE, SIDE, &reference)?,
        Frame::from_monochrome(SIDE, SIDE, &candidate)?,
    ))
}

/// **All three regions coincide in these fixtures, and that is deliberate.**
/// The whole frame is image, so there is no letterbox, and `BASE` is 128 with
/// deltas no larger than three, so no pixel reaches 0 or 255 on either side
/// and every image pixel is informative. The full-frame, image-rectangle and
/// informative statistics are therefore the same numbers, which is what makes
/// each boundary below a statement about the BOUND rather than about the
/// region.
///
/// **Which region the comparator picks is therefore not decided here, and
/// until the sprint review's fifth pass it was not decided anywhere a
/// `cargo test` could reach.** It is decided by
/// `the_bias_bound_is_fed_the_informative_region_and_not_the_rectangle` in
/// `tools/oracle/src/attribution.rs`, which builds a frame whose two regions
/// disagree about the verdict. Over the corpus it is measured by
/// `ocelli-compare census`.
///
/// The bound is fed the INFORMATIVE channel below, which is the region
/// `build_statistics` feeds it, so this file asserts a bound over the region
/// production evaluates rather than one beside it.
fn verdict(
    deltas: &[(usize, i16)],
) -> Result<(tolerance::MonochromeVerdict, tolerance::BiasVerdict), Box<dyn Error>> {
    let (reference, candidate) = pair(deltas)?;
    let rect = Rect::full(SIDE, SIDE);
    let diff = difference(&reference, &candidate, rect, ChannelSet::Monochrome)?;
    let full: &ChannelStats = diff
        .full
        .channel(0)
        .ok_or("no channel 0 in the full-frame statistics")?;
    let informative: &ChannelStats = diff
        .informative
        .channel(0)
        .ok_or("no channel 0 in the informative statistics")?;
    Ok((
        tolerance::monochrome_predicate(full)?,
        tolerance::bias_bound(informative)?,
    ))
}

fn spread(count: usize, delta: i16) -> Vec<(usize, i16)> {
    (0..count).map(|index| (index, delta)).collect()
}

/// 25.1 boundary one. Exactly 99.9% of pixels within 1 LSB passes.
///
/// 10 of 10000 pixels differ by 2, so 9990 are within 1 and the fraction is
/// 9990 / 10000 = 0.999 exactly, which satisfies "on at least 99.9%". No pixel
/// differs by more than 2, so the second clause holds at a count of ten.
/// A pixel differing by exactly 2 is PERMITTED, for up to 0.1% of the frame.
#[test]
fn exactly_999_permille_within_one_lsb_passes() -> Outcome {
    let (monochrome, _) = verdict(&spread(10, 2))?;
    assert_eq!(
        monochrome.within_one_lsb_fraction.to_bits(),
        0.999_f64.to_bits()
    );
    assert_eq!(monochrome.count_over_max, 0);
    assert_eq!(monochrome.max_abs_diff, 2);
    assert!(
        monochrome.passes,
        "9990 of 10000 within 1 LSB is exactly 99.9%"
    );
    Ok(())
}

/// 25.1 boundary one, one pixel the wrong side of it.
///
/// 11 of 10000 differ by 2, so 9989 are within 1 and the fraction is 0.9989,
/// which is below 0.999. One pixel is the whole difference between this case
/// and the one above.
#[test]
fn one_pixel_below_999_permille_fails() -> Outcome {
    let (monochrome, _) = verdict(&spread(11, 2))?;
    assert_eq!(
        monochrome.within_one_lsb_fraction.to_bits(),
        0.9989_f64.to_bits()
    );
    assert_eq!(monochrome.count_over_max, 0);
    assert!(!monochrome.passes, "0.9989 is below 25.1's 0.999");
    Ok(())
}

/// 25.1 boundary two. "zero pixels differing by more than 2", so a single
/// pixel at 3 fails at any count, even though 9999 of 10000 pixels are
/// identical and the within-one-LSB fraction is 0.9999.
#[test]
fn one_pixel_at_difference_three_fails_at_any_count() -> Outcome {
    let (monochrome, _) = verdict(&[(0, 3)])?;
    assert_eq!(
        monochrome.within_one_lsb_fraction.to_bits(),
        0.9999_f64.to_bits()
    );
    assert_eq!(monochrome.count_over_max, 1);
    assert_eq!(monochrome.max_abs_diff, 3);
    assert!(!monochrome.passes, "one pixel over 2 fails, at any count");
    Ok(())
}

/// The sign of the difference does not enter 25.1's first two clauses. A frame
/// three codes DARKER at one pixel fails for the same reason as one three
/// codes brighter, which is what "absolute difference" means and is the clause
/// an implementation that subtracted without taking an absolute value would
/// pass.
#[test]
fn a_negative_difference_of_three_fails_the_same_way() -> Outcome {
    let (monochrome, _) = verdict(&[(0, -3)])?;
    assert_eq!(monochrome.count_over_max, 1);
    assert_eq!(monochrome.max_abs_diff, 3);
    assert!(!monochrome.passes);
    Ok(())
}

/// An identical frame passes both bounds, with a signed mean of exactly zero.
/// The degenerate case is asserted rather than assumed, because a comparator
/// whose statistics were empty would also report zero.
#[test]
fn an_identical_frame_passes_with_zero_bias() -> Outcome {
    let (monochrome, bias) = verdict(&[])?;
    assert_eq!(
        monochrome.within_one_lsb_fraction.to_bits(),
        1.0_f64.to_bits()
    );
    assert_eq!(monochrome.max_abs_diff, 0);
    assert_eq!(bias.signed_mean_diff.to_bits(), 0.0_f64.to_bits());
    assert!(monochrome.passes);
    assert!(bias.passes);
    Ok(())
}

/// The bias bullet, at the bound. 1000 of 10000 pixels one code brighter is a
/// signed mean of exactly 1000 / 10000 = 0.1, and the bullet says "within
/// 0.1", so the bound is satisfied AT it.
///
/// 1000.0 / 10000.0 and the literal 0.1 are the same double, both being the
/// correctly rounded nearest double to the rational one tenth. That is
/// asserted rather than assumed, because if it were ever untrue this test
/// would be checking a boundary one ulp away from the one 25.1 states.
#[test]
fn a_signed_mean_of_exactly_one_tenth_is_within_the_bound() -> Outcome {
    let (monochrome, bias) = verdict(&spread(1000, 1))?;
    assert_eq!(bias.signed_mean_diff.to_bits(), 0.1_f64.to_bits());
    assert!(bias.passes, "\"within 0.1\" includes 0.1");
    assert!(
        monochrome.passes,
        "and 25.1's maximum-difference rule passes it, which is why the bias \
         bullet exists at all"
    );
    Ok(())
}

/// The bias bullet, one pixel over. 1001 of 10000 is 0.1001.
#[test]
fn a_signed_mean_one_pixel_over_the_bound_fails() -> Outcome {
    let (monochrome, bias) = verdict(&spread(1001, 1))?;
    assert_eq!(bias.signed_mean_diff.to_bits(), 0.1001_f64.to_bits());
    assert!(!bias.passes);
    assert!(
        monochrome.passes,
        "a frame every pixel of which is within one code passes 25.1's first \
         two clauses no matter how many pixels differ"
    );
    Ok(())
}

/// Bias is SIGNED, so a difference that is one code brighter on half the
/// pixels it touches and one code darker on the other half cancels to zero.
/// That is the property separating a systematic window-function divergence
/// from rounding noise, and it is the reason the bullet does not say "mean
/// absolute difference".
#[test]
fn a_balanced_difference_cancels_to_zero_bias() -> Outcome {
    let mut deltas = spread(2000, 1);
    deltas.extend((2000..4000).map(|index| (index, -1)));
    let (monochrome, bias) = verdict(&deltas)?;
    assert_eq!(bias.signed_mean_diff.to_bits(), 0.0_f64.to_bits());
    assert!(bias.passes, "4000 differing pixels, no bias");
    assert!(monochrome.passes);
    Ok(())
}

/// A negative bias fails at the same magnitude as a positive one. The bullet
/// says "within 0.1", which is a two-sided bound, and a comparator that tested
/// only the upper side would pass a candidate systematically darker than the
/// reference.
#[test]
fn a_negative_bias_fails_at_the_same_magnitude() -> Outcome {
    let (_, bias) = verdict(&spread(1001, -1))?;
    assert_eq!(bias.signed_mean_diff.to_bits(), (-0.1001_f64).to_bits());
    assert!(!bias.passes);
    Ok(())
}

/// The constants are asserted against the quoted text of 25.1, so a silent
/// widening changes a line a reviewer reads. This is the check that makes
/// `tolerance.rs` a transcription rather than a set of numbers.
#[test]
fn the_constants_match_the_quoted_section_25_1_text() {
    assert!(
        tolerance::SECTION_25_1_MONOCHROME.contains("≤ 1 LSB on at least 99.9% of pixels"),
        "the quoted rule must carry the fraction the constant claims"
    );
    assert!(
        tolerance::SECTION_25_1_MONOCHROME.contains("zero pixels differing by more than 2"),
        "the quoted rule must carry the maximum the constant claims"
    );
    assert!(
        tolerance::SECTION_25_1_BIAS.contains("within 0.1 of one display code"),
        "the quoted bullet must carry the bound the constant claims"
    );
    assert!(
        tolerance::SECTION_25_1_GEOMETRY.contains("within 1e-6 mm"),
        "the quoted bullet must carry the world bound the constant claims"
    );
    assert!(
        tolerance::SECTION_25_1_GEOMETRY.contains("within a quarter pixel"),
        "the quoted bullet must carry the canvas bound the constant claims"
    );
    assert_eq!(
        tolerance::MONOCHROME_WITHIN_ONE_LSB_FRACTION.to_bits(),
        0.999_f64.to_bits()
    );
    assert_eq!(tolerance::MONOCHROME_ONE_LSB, 1);
    assert_eq!(tolerance::MONOCHROME_MAX_ABS_DIFF, 2);
    assert_eq!(
        tolerance::MONOCHROME_SIGNED_MEAN_BIAS.to_bits(),
        0.1_f64.to_bits()
    );
    assert_eq!(tolerance::WORLD_TOLERANCE_MM.to_bits(), 1e-6_f64.to_bits());
    assert_eq!(
        tolerance::CANVAS_TOLERANCE_PIXELS.to_bits(),
        0.25_f64.to_bits()
    );
    // NOT from 25.1, and pinned here for that reason rather than in spite of
    // it. 25.1 states no informative floor, the comparator needs one, and the
    // design round put it beside the 25.1 constants so the reference half's
    // configuration cannot decide a comparator verdict. It is a declared
    // judgement, so it gets the same "changing it is a line a reviewer reads"
    // treatment as the numbers that are 25.1's.
    assert_eq!(
        tolerance::INFORMATIVE_FRACTION_FLOOR.to_bits(),
        0.10_f64.to_bits()
    );
}
