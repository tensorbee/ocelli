//! The LINEAR against LINEAR_EXACT divergence, quantised to eight bits, and
//! what HLD 25.1 can and cannot say about it.
//!
//! **This file computes no VOI function.** HLD section 18 says the LUT chain
//! is implemented once, in `ocelli-pixel`, and the comparator contains no copy
//! of it. Every display value below is a hand-computed literal, worked out
//! from the formulas transcribed in HLD 18.2 and reproduced in the comment
//! that carries it. The only arithmetic this file performs on them is
//! `f64::round`, which is the declared quantiser and not a window function.
//!
//! The formulas, verbatim from HLD 18.2:
//!
//! ```text
//! // PS3.3 C.11.2.1.2 -- LINEAR. Requires w >= 1.
//! // c' = c - 0.5 ; w' = w - 1
//! // x <= c' - w'/2 -> ymin
//! // x > c' + w'/2 -> ymax
//! // else y = ((x - c') / w' + 0.5) * (ymax - ymin) + ymin
//! // PS3.3 C.11.2.1.3.2 -- LINEAR_EXACT. Requires w > 0.
//! // x <= c - w/2 -> ymin
//! // x > c + w/2 -> ymax
//! // else y = ((x - c) / w + 0.5) * (ymax - ymin) + ymin
//! ```
//!
//! Throughout: the soft-tissue CT window of HLD 18.3, centre 40, width 400,
//! output range 0 to 255, so `c' = 39.5` and `w' = 399`. The row's rescale
//! slope is 1 and its intercept 0, so a stored value equals its HU value.
//!
//! **Rounding rule, declared here because neither 25.1 nor section 18 states
//! one** (HLD 27.3 makes every rounding decision a human review item): a
//! display value becomes an 8-bit code by rounding half away from zero, which
//! is `f64::round`. Sixteen of the seventeen values below are chosen so that
//! none lands on a half and the table is therefore independent of which of the
//! three common rules is used. The seventeenth, `LINEAR_EXACT(40) = 127.500`,
//! lands exactly on one and is asserted separately for that reason.

use std::error::Error;

use ocelli_oracle::frame::{ChannelSet, ChannelStats, Frame, Rect, difference};
use ocelli_oracle::tolerance;

type Outcome = Result<(), Box<dyn Error>>;

/// HU 100 to 115, quantised LINEAR codes. Hand-computed, one row per HU:
///
/// ```text
/// LINEAR(x) = ((x - 39.5) / 399 + 0.5) * 255
///
/// 100 -> 166.165 -> 166      108 -> 171.278 -> 171
/// 101 -> 166.805 -> 167      109 -> 171.917 -> 172
/// 102 -> 167.444 -> 167      110 -> 172.556 -> 173
/// 103 -> 168.083 -> 168      111 -> 173.195 -> 173
/// 104 -> 168.722 -> 169      112 -> 173.835 -> 174
/// 105 -> 169.361 -> 169      113 -> 174.474 -> 174
/// 106 -> 170.000 -> 170      114 -> 175.113 -> 175
/// 107 -> 170.639 -> 171      115 -> 175.752 -> 176
/// ```
const LINEAR_CODES: [u8; 16] = [
    166, 167, 167, 168, 169, 169, 170, 171, 171, 172, 173, 173, 174, 174, 175, 176,
];

/// The same sixteen inputs through LINEAR_EXACT. Hand-computed:
///
/// ```text
/// LINEAR_EXACT(x) = ((x - 40) / 400 + 0.5) * 255
///
/// 100 -> 165.750 -> 166      108 -> 170.850 -> 171
/// 101 -> 166.388 -> 166      109 -> 171.488 -> 171
/// 102 -> 167.025 -> 167      110 -> 172.125 -> 172
/// 103 -> 167.663 -> 168      111 -> 172.763 -> 173
/// 104 -> 168.300 -> 168      112 -> 173.400 -> 173
/// 105 -> 168.938 -> 169      113 -> 174.038 -> 174
/// 106 -> 169.575 -> 170      114 -> 174.675 -> 175
/// 107 -> 170.213 -> 170      115 -> 175.313 -> 175
/// ```
const LINEAR_EXACT_CODES: [u8; 16] = [
    166, 166, 167, 168, 168, 169, 170, 170, 171, 171, 172, 173, 173, 174, 175, 175,
];

/// Seven of the sixteen differ, every one of them by exactly one code, every
/// one of them in the same direction. That one-sidedness is the signature the
/// bias bullet exists to see.
const DIFFERING: u64 = 7;

/// Four by four is the sixteen inputs above, and the whole frame is image, so
/// the image rectangle the bias bullet names is the whole of it.
const SIDE: u32 = 4;

fn divergence_pair() -> Result<
    (
        tolerance::MonochromeVerdict,
        tolerance::BiasVerdict,
        ChannelStats,
    ),
    Box<dyn Error>,
> {
    let reference = Frame::from_monochrome(SIDE, SIDE, &LINEAR_EXACT_CODES)?;
    let candidate = Frame::from_monochrome(SIDE, SIDE, &LINEAR_CODES)?;
    let diff = difference(
        &reference,
        &candidate,
        Rect::full(SIDE, SIDE),
        ChannelSet::Monochrome,
    )?;
    let stats = diff
        .full
        .channel(0)
        .ok_or("no channel 0 in the full-frame statistics")?
        .clone();
    let image = diff
        .image
        .channel(0)
        .ok_or("no channel 0 in the image statistics")?;
    Ok((
        tolerance::monochrome_predicate(&stats)?,
        tolerance::bias_bound(image)?,
        stats,
    ))
}

/// **This test asserts that the written maximum-difference tolerance PASSES a
/// real window-function divergence.** That is the finding rather than a bug in
/// the test, and it is the whole reason the operator added a bias bullet to
/// 25.1 in S03.
///
/// The arithmetic behind it, from the two formulas above:
///
/// ```text
/// LINEAR(x) - LINEAR_EXACT(x)
///   = 255 * [ (x - 39.5)/399 - (x - 40)/400 ]
///   = 255 * (x + 160) / 159600
/// ```
///
/// which is 0 at LINEAR's lower clamp of -160 and 0.6375 at its upper clamp of
/// 239, and never exceeds 255 * 400 / 159600 = 0.639 over the whole window. A
/// difference bounded by 0.639 of a display code cannot survive quantisation
/// to eight bits as more than one code under any monotone quantiser, so the
/// swap always yields `maxAbsDiff <= 1` and `countOverTwo == 0`.
#[test]
fn the_written_maximum_difference_rule_passes_a_linear_against_linear_exact_swap() -> Outcome {
    let (monochrome, _, stats) = divergence_pair()?;
    assert_eq!(monochrome.max_abs_diff, 1, "no pixel moves by two codes");
    assert_eq!(monochrome.count_over_max, 0);
    assert_eq!(
        monochrome.within_one_lsb_fraction.to_bits(),
        1.0_f64.to_bits(),
        "every pixel is within one LSB, so the fraction is 1.0 and not merely \
         above 0.999"
    );
    assert!(
        monochrome.passes,
        "25.1's first two clauses pass the project's own headline defect"
    );
    assert_eq!(stats.count_at(1), DIFFERING);
    Ok(())
}

/// The bias bullet fails the same pair. Seven of sixteen pixels one code
/// brighter is a signed mean of 7 / 16 = 0.4375, which is four times the
/// declared bound of 0.1 and is exactly representable in binary.
#[test]
fn the_bias_bullet_fails_the_same_pair() -> Outcome {
    let (_, bias, _) = divergence_pair()?;
    assert_eq!(bias.signed_mean_diff.to_bits(), 0.4375_f64.to_bits());
    assert!(
        !bias.passes,
        "0.4375 is over 25.1's 0.1, which is what makes the divergence \
         detectable at all"
    );
    Ok(())
}

/// Swapping the two sides negates the bias exactly and leaves the
/// maximum-difference statistics alone. A comparator that reported the
/// magnitude would say the same thing about the two orders, and the sign is
/// what says which side is brighter.
#[test]
fn the_bias_negates_exactly_when_the_sides_are_swapped() -> Outcome {
    let reference = Frame::from_monochrome(SIDE, SIDE, &LINEAR_CODES)?;
    let candidate = Frame::from_monochrome(SIDE, SIDE, &LINEAR_EXACT_CODES)?;
    let diff = difference(
        &reference,
        &candidate,
        Rect::full(SIDE, SIDE),
        ChannelSet::Monochrome,
    )?;
    let image = diff
        .image
        .channel(0)
        .ok_or("no channel 0 in the image statistics")?;
    let bias = tolerance::bias_bound(image)?;
    assert_eq!(bias.signed_mean_diff.to_bits(), (-0.4375_f64).to_bits());
    assert!(!bias.passes);
    Ok(())
}

/// HLD 18.3's four sample points, quantised, showing that ALL FOUR collapse to
/// identical display codes. The table exists to show a 0.32 divergence at the
/// window centre, and at eight bits none of its four rows can show it.
///
/// The four rows, with deviation **D-13** applied to the first:
///
/// | Input (HU) | LINEAR | LINEAR_EXACT | codes |
/// |---|---|---|---|
/// | -160 | 0.000 | 0.000 | 0, 0 |
/// | 40 | 127.819 | 127.500 | 128, 128 |
/// | 240 | 255.000 | 255.000 | 255, 255 |
/// | -60 | 63.910 | 63.750 | 64, 64 |
///
/// **D-13**: 18.3's table prints `LINEAR_EXACT(-160) = 1.594`. 18.2's
/// LINEAR_EXACT clamps at `x <= c - w/2`, which is `40 - 200 = -160`, and
/// `-160 <= -160` holds, so the value is `ymin`. The formula body evaluates to
/// `((-160 - 40) / 400 + 0.5) * 255 = (-0.5 + 0.5) * 255 = 0.000` there in any
/// case, so no boundary convention produces 1.594. 1.594 is the value at
/// `x = -157.5`. Where 18.2's formula and 18.3's worked value disagree, the
/// formula is the specification.
#[test]
fn all_four_rows_of_the_18_3_table_collapse_to_identical_codes() {
    let rows: [(f64, f64); 4] = [
        (0.000, 0.000),
        (127.819, 127.500),
        (255.000, 255.000),
        (63.910, 63.750),
    ];
    let expected: [f64; 4] = [0.0, 128.0, 255.0, 64.0];
    for (index, (linear, linear_exact)) in rows.iter().enumerate() {
        let code = linear.round();
        let exact_code = linear_exact.round();
        assert_eq!(
            code.to_bits(),
            exact_code.to_bits(),
            "18.3 row {index} does not distinguish the two functions at 8 bits"
        );
        let Some(want) = expected.get(index) else {
            continue;
        };
        assert_eq!(code.to_bits(), want.to_bits());
    }
}

/// The window centre is the one place in 18.3's table where the rounding rule
/// decides. `LINEAR_EXACT(40)` is exactly 127.500, so half away from zero and
/// half to even both give 128 while truncation gives 127. The declared rule is
/// half away from zero, which is `f64::round`, and it is asserted here so the
/// choice is a line a reviewer reads rather than an implicit default.
#[test]
fn the_window_centre_is_the_one_half_value_and_the_rule_is_declared() {
    assert_eq!(127.500_f64.round().to_bits(), 128.0_f64.to_bits());
    assert_eq!(127.500_f64.trunc().to_bits(), 127.0_f64.to_bits());
    // `black_box` keeps these runtime values rather than constants clippy can
    // fold, which is what makes this an assertion rather than a comment.
    let linear = core::hint::black_box(127.819_f64);
    let linear_exact = core::hint::black_box(127.500_f64);
    assert!(
        (linear - linear_exact) < 0.5,
        "the 0.32 divergence 18.3 calls its headline is less than half a code, \
         so no quantiser can turn it into a code difference at this input"
    );
}
