//! The LINEAR against LINEAR_EXACT divergence, quantised to eight bits, and
//! what HLD 25.1 can and cannot say about it.
//!
//! **This file carries no LUT chain.** HLD section 18 says the chain is
//! implemented once, in `ocelli-pixel`, and neither the comparator nor this
//! fixture holds a copy of it. What it does hold is HLD 18.2's two window
//! formulas transcribed into EXACT INTEGER RATIONAL arithmetic, over the one
//! window HLD 18.3 works, for one purpose: to check this file's own
//! hand-computed literals against the specification they were computed from.
//! There is no `f64` anywhere in that transcription, no eight-bit pipeline,
//! and nothing outside these tests reads it.
//!
//! **That transcription is what makes the numbers below assertable.** Until
//! the S03 sprint review's second pass this fixture asserted only that HLD
//! 18.3's four rows ROUND to equal display codes, so a mistyped display value
//! survived anywhere inside a half-code band. HLD 27.2 R3 asks for
//! hand-computed values and a rounding equivalence is not one. Every display
//! value is now written as the exact rational its formula produces and
//! compared to the formula by cross-multiplication, which no wrong literal
//! survives at any size.
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
//! display value becomes an 8-bit code by rounding half away from zero. Every
//! display value here is non-negative, so on integer rationals that is
//! `(2n + d) / (2d)` under Rust's truncating division, stated once in
//! `Exact::code` and used everywhere.
//!
//! This file asserts forty display values: sixteen inputs through each
//! function in the tables below, and HLD 18.3's four rows through each. Of
//! those forty, **exactly one lands on a half**, `LINEAR_EXACT(40) = 255/2`,
//! and it is asserted separately for that reason. The other thirty-nine are
//! independent of which of the three common rounding rules is used, which is
//! checked rather than claimed by `exactly_one_asserted_value_lands_on_a_half`.

use std::error::Error;

use ocelli_oracle::frame::{ChannelSet, ChannelStats, Frame, Rect, difference};
use ocelli_oracle::tolerance;

type Outcome = Result<(), Box<dyn Error>>;

/// The window HLD 18.3 works, as integers. `c' = c - 0.5` never appears as a
/// number here, because every comparison and every quotient below is written
/// over `2x` instead, and that is what removes the last float from the file.
const CENTRE: i128 = 40;
const WIDTH: i128 = 400;
const Y_MIN: i128 = 0;
const Y_MAX: i128 = 255;

/// A display value as an exact rational, `numerator / denominator`.
///
/// Both formulas of HLD 18.2 are ratios of integers at every input this file
/// uses, so nothing here is ever an approximation and no comparison needs a
/// tolerance. Denominators are `2(w - 1)` and `2w`, which is 798 and 800 at
/// the tables' own window and up to 8190 across the twelve widths the
/// white-exclusion section varies over. The band endpoints at the foot of the
/// file carry a further factor of 510, which is the widest thing here: the
/// largest denominator is then about 4.2e6, the largest numerator about 1.1e9,
/// and the cross products about 2.2e9. `i128` holds around 1.7e38 and cannot
/// overflow on any of it.
#[derive(Clone, Copy, Debug)]
struct Exact {
    numerator: i128,
    denominator: i128,
}

impl Exact {
    /// A whole number of display codes.
    const fn whole(value: i128) -> Self {
        Self {
            numerator: value,
            denominator: 1,
        }
    }

    /// Exact equality, by cross-multiplication rather than by comparing the
    /// two pairs, so `102000/798` and `51000/399` are the same value. This is
    /// the comparison a mistyped literal cannot survive: it is exact, and a
    /// wrong numerator is wrong by at least one part in 800 of a display code.
    fn equals(self, other: Self) -> bool {
        self.numerator * other.denominator == other.numerator * self.denominator
    }

    /// Rounded half away from zero to an 8-bit display code. Non-negative
    /// here, so `(2n + d) / (2d)` under truncating division IS half away from
    /// zero and there is no rounding decision left implicit.
    fn code(self) -> i128 {
        (2 * self.numerator + self.denominator) / (2 * self.denominator)
    }

    /// The same rounding, to three decimal places, so the file can state the
    /// decimals HLD 18.3 prints and check them without a float.
    fn thousandths(self) -> i128 {
        (2 * 1000 * self.numerator + self.denominator) / (2 * self.denominator)
    }

    /// Whether the value is exactly a half, which is the one input the
    /// rounding rule decides.
    fn is_a_half(self) -> bool {
        (2 * self.numerator) % (2 * self.denominator) == self.denominator
    }

    /// `self * numerator / denominator`, exact.
    ///
    /// Every denominator in this file is positive, which is what makes
    /// `is_below` a cross-multiplication with no sign case, so this is only
    /// ever called with a positive `denominator`.
    fn scaled(self, numerator: i128, denominator: i128) -> Self {
        Self {
            numerator: self.numerator * numerator,
            denominator: self.denominator * denominator,
        }
    }

    /// Strict `<`, by cross-multiplication. Both denominators are positive, so
    /// the inequality does not flip.
    fn is_below(self, other: Self) -> bool {
        self.numerator * other.denominator < other.numerator * self.denominator
    }

    /// `self + other`, exact.
    fn plus(self, other: Self) -> Self {
        Self {
            numerator: self.numerator * other.denominator + other.numerator * self.denominator,
            denominator: self.denominator * other.denominator,
        }
    }

    /// `self - other`, exact.
    fn minus(self, other: Self) -> Self {
        Self {
            numerator: self.numerator * other.denominator - other.numerator * self.denominator,
            denominator: self.denominator * other.denominator,
        }
    }
}

/// PS3.3 C.11.2.1.2, LINEAR, at this file's centre and an arbitrary width, in
/// exact rationals.
///
/// Transcribed from the block quoted in the module header and from nothing
/// else. `c' = c - 0.5` and `w' = w - 1`, so `2c' = 2c - 1`, and every test
/// below is written over `2x`:
///
/// ```text
/// x <= c' - w'/2   becomes   2x <= 2c' - w'
/// x >  c' + w'/2   becomes   2x >  2c' + w'
/// y = ((x - c')/w' + 1/2) * (ymax - ymin) + ymin
///   = [ (2x - 2c') + w' ] * (ymax - ymin) / (2w') + ymin
/// ```
///
/// The width is a parameter because the white-exclusion section at the foot of
/// this file has to vary it. `w >= 2`, so `w'` is at least 1 and nothing here
/// divides by zero, which is HLD 18.2's own `w >= 1` for LINEAR tightened by
/// one because `w' = 0` is the division it warns about.
///
/// **The stored value is an exact rational rather than an integer**, and that
/// is not generality for its own sake. The movable band's two endpoints fall
/// BETWEEN integers at every width, so a section that could only evaluate this
/// transcription at integers had to write its endpoints down as literals
/// derived from neither formula, which is what the sprint review's seventh
/// pass found. `linear_at` below is the integer entry point and calls this, so
/// there is still exactly one transcription in the file.
fn linear_at_rational(x: Exact, w: i128) -> Exact {
    let two_x = x.scaled(2, 1);
    let two_c_prime = Exact::whole(2 * CENTRE - 1);
    let w_prime = Exact::whole(w - 1);
    if !two_c_prime.minus(w_prime).is_below(two_x) {
        return Exact::whole(Y_MIN);
    }
    if two_c_prime.plus(w_prime).is_below(two_x) {
        return Exact::whole(Y_MAX);
    }
    two_x
        .minus(two_c_prime)
        .plus(w_prime)
        .scaled(Y_MAX - Y_MIN, 2 * (w - 1))
        .plus(Exact::whole(Y_MIN))
}

/// The same, at an integer stored value, which is what every table in this
/// file is written over.
fn linear_at(x: i128, w: i128) -> Exact {
    linear_at_rational(Exact::whole(x), w)
}

/// PS3.3 C.11.2.1.3.2, LINEAR_EXACT, the same way.
///
/// ```text
/// x <= c - w/2   becomes   2x <= 2c - w
/// x >  c + w/2   becomes   2x >  2c + w
/// y = ((x - c)/w + 1/2) * (ymax - ymin) + ymin
///   = [ (2x - 2c) + w ] * (ymax - ymin) / (2w) + ymin
/// ```
fn linear_exact_at_rational(x: Exact, w: i128) -> Exact {
    let two_x = x.scaled(2, 1);
    let two_c = Exact::whole(2 * CENTRE);
    let width = Exact::whole(w);
    if !two_c.minus(width).is_below(two_x) {
        return Exact::whole(Y_MIN);
    }
    if two_c.plus(width).is_below(two_x) {
        return Exact::whole(Y_MAX);
    }
    two_x
        .minus(two_c)
        .plus(width)
        .scaled(Y_MAX - Y_MIN, 2 * w)
        .plus(Exact::whole(Y_MIN))
}

/// The same, at an integer stored value.
fn linear_exact_at(x: i128, w: i128) -> Exact {
    linear_exact_at_rational(Exact::whole(x), w)
}

/// A stored value `CENTRE + u`, which is the coordinate the band endpoints at
/// the foot of this file are solved in. `u = x - c` and nothing else.
fn stored_at(u: Exact) -> Exact {
    Exact::whole(CENTRE).plus(u)
}

/// LINEAR at the one window HLD 18.3 works, which is what everything above the
/// white-exclusion section uses. One transcription, two call sites.
fn linear(x: i128) -> Exact {
    linear_at(x, WIDTH)
}

/// LINEAR_EXACT at the same window.
fn linear_exact(x: i128) -> Exact {
    linear_exact_at(x, WIDTH)
}

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

/// Four by four is the sixteen inputs above, and the whole frame is image. No
/// code in either table is 0 or 255, so no pixel is clipped and the informative
/// region the bias bullet names is the whole frame too. The two regions
/// coincide here, so nothing in this file distinguishes them, and the test
/// that does is
/// `the_bias_bound_is_fed_the_informative_region_and_not_the_rectangle` in
/// `tools/oracle/src/attribution.rs`.
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
    // The INFORMATIVE region, which is what `build_statistics` feeds the
    // bound. It is the whole frame here, because no code in either table is 0
    // or 255, but naming the region the comparator uses keeps this file from
    // asserting a bound over a region production does not evaluate.
    let informative = diff
        .informative
        .channel(0)
        .ok_or("no channel 0 in the informative statistics")?;
    Ok((
        tolerance::monochrome_predicate(&stats)?,
        tolerance::bias_bound(informative)?,
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
    let informative = diff
        .informative
        .channel(0)
        .ok_or("no channel 0 in the informative statistics")?;
    let bias = tolerance::bias_bound(informative)?;
    assert_eq!(bias.signed_mean_diff.to_bits(), (-0.4375_f64).to_bits());
    assert!(!bias.passes);
    Ok(())
}

/// One row of HLD 18.3's worked table, hand-computed.
///
/// Each display value is the exact rational the formula produces, written out
/// as a numerator over a denominator, and each is checked against the formula
/// by cross-multiplication in `the_four_rows_of_18_3_are_the_hand_computed_values`.
struct WorkedRow {
    /// The stored value, which is the HU value: this row's rescale slope is 1
    /// and its intercept 0.
    hu: i128,
    linear: Exact,
    linear_exact: Exact,
    /// The display value as HLD 18.3 prints it, in thousandths, so the
    /// decimals a reader compares against the HLD are asserted too.
    linear_thousandths: i128,
    linear_exact_thousandths: i128,
    /// The 8-bit code both functions collapse to.
    code: i128,
}

/// HLD 18.3's four sample points, hand-computed, with deviation **D-13**
/// applied to the first.
///
/// The workings, at `c = 40`, `w = 400`, `ymin = 0`, `ymax = 255`, so
/// `2c' = 79`, `w' = 399`, `c' - w'/2 = -160` and `c' + w'/2 = 239`:
///
/// ```text
/// x = -160  LINEAR       2x = -320 <= 79 - 399, so ymin        0/1
///           LINEAR_EXACT 2x = -320 <= 80 - 400, so ymin        0/1
/// x =   40  LINEAR       (80 - 79 + 399) * 255 / 798      102000/798
///           LINEAR_EXACT (80 - 80 + 400) * 255 / 800      102000/800
/// x =  240  LINEAR       2x = 480 > 79 + 399, so ymax          255/1
///           LINEAR_EXACT 2x = 480 is NOT > 480, so the body,
///                        (480 - 80 + 400) * 255 / 800     204000/800
/// x =  -60  LINEAR       (-120 - 79 + 399) * 255 / 798     51000/798
///           LINEAR_EXACT (-120 - 80 + 400) * 255 / 800     51000/800
/// ```
///
/// In decimal those are `0`, `127.8195488...`, `255`, `63.9097744...` for
/// LINEAR and `0`, `127.5`, `255`, `63.75` for LINEAR_EXACT. **HLD 18.3 prints
/// the second as `127.819`, which is the exact value truncated rather than
/// rounded**, and its fourth as `63.910`, which is rounded. The rationals are
/// the values and the printed decimals are a rendering of them, so the
/// thousandths asserted here are the correctly rounded ones.
///
/// **D-13**: 18.3's table prints `LINEAR_EXACT(-160) = 1.594`. 18.2's
/// LINEAR_EXACT clamps at `x <= c - w/2`, which is `40 - 200 = -160`, and
/// `-160 <= -160` holds, so the value is `ymin`. The formula body evaluates to
/// `((-160 - 40) / 400 + 0.5) * 255 = (-0.5 + 0.5) * 255 = 0.000` there in any
/// case, so no boundary convention produces 1.594. 1.594 is the value at
/// `x = -157.5`. Where 18.2's formula and 18.3's worked value disagree, the
/// formula is the specification.
const ROWS_18_3: [WorkedRow; 4] = [
    WorkedRow {
        hu: -160,
        linear: Exact::whole(0),
        linear_exact: Exact::whole(0),
        linear_thousandths: 0,
        linear_exact_thousandths: 0,
        code: 0,
    },
    WorkedRow {
        hu: 40,
        linear: Exact {
            numerator: 102_000,
            denominator: 798,
        },
        linear_exact: Exact {
            numerator: 102_000,
            denominator: 800,
        },
        linear_thousandths: 127_820,
        linear_exact_thousandths: 127_500,
        code: 128,
    },
    WorkedRow {
        hu: 240,
        linear: Exact::whole(255),
        linear_exact: Exact {
            numerator: 204_000,
            denominator: 800,
        },
        linear_thousandths: 255_000,
        linear_exact_thousandths: 255_000,
        code: 255,
    },
    WorkedRow {
        hu: -60,
        linear: Exact {
            numerator: 51_000,
            denominator: 798,
        },
        linear_exact: Exact {
            numerator: 51_000,
            denominator: 800,
        },
        linear_thousandths: 63_910,
        linear_exact_thousandths: 63_750,
        code: 64,
    },
];

/// **The assertion S15 asked for.** Every hand-computed value in `ROWS_18_3`
/// is compared to what HLD 18.2's formula produces, exactly, by
/// cross-multiplication.
///
/// The old version of this test asserted only that the four rows ROUND to
/// equal display codes, so a mistyped literal survived anywhere inside a
/// half-code band. Nothing survives this: a numerator wrong by one is a
/// display value wrong by one part in 800 of a code, and the comparison has no
/// tolerance at all.
#[test]
fn the_four_rows_of_18_3_are_the_hand_computed_values() {
    for row in &ROWS_18_3 {
        let hu = row.hu;
        assert!(
            linear(hu).equals(row.linear),
            "LINEAR({hu}): HLD 18.2's formula gives {:?} and this file writes \
             {:?}",
            linear(hu),
            row.linear
        );
        assert!(
            linear_exact(hu).equals(row.linear_exact),
            "LINEAR_EXACT({hu}): HLD 18.2's formula gives {:?} and this file \
             writes {:?}",
            linear_exact(hu),
            row.linear_exact
        );
        assert_eq!(
            row.linear.thousandths(),
            row.linear_thousandths,
            "LINEAR({hu}) is not the decimal this file prints for it"
        );
        assert_eq!(
            row.linear_exact.thousandths(),
            row.linear_exact_thousandths,
            "LINEAR_EXACT({hu}) is not the decimal this file prints for it"
        );
    }
}

/// The point HLD 18.3's table exists to make, and the point it cannot make at
/// eight bits: all four rows collapse to one code per row.
#[test]
fn all_four_rows_of_the_18_3_table_collapse_to_identical_codes() {
    for row in &ROWS_18_3 {
        let hu = row.hu;
        assert_eq!(
            row.linear.code(),
            row.code,
            "LINEAR({hu}) does not quantise to the hand-computed code"
        );
        assert_eq!(
            row.linear_exact.code(),
            row.code,
            "LINEAR_EXACT({hu}) does not quantise to the hand-computed code"
        );
    }
}

/// The divergence itself, exactly, against the closed form the tolerance
/// constant and `docs/lld/comparator.md` both quote:
///
/// ```text
/// LINEAR(x) - LINEAR_EXACT(x) = 255 * (x + 160) / 159600
/// ```
///
/// It holds only where NEITHER function clamps, which is the interval
/// `(-160, 239]`. Of 18.3's four rows two are inside it and two are not, and
/// the two that are not are the reason the closed form is stated with its
/// interval rather than as an identity. At `x = 240` LINEAR has clamped to
/// `ymax` while LINEAR_EXACT's body reaches `ymax` on its own, so the
/// divergence there is exactly zero and the closed form would say 0.639.
#[test]
fn the_divergence_matches_the_closed_form_wherever_neither_function_clamps() {
    for row in &ROWS_18_3 {
        let hu = row.hu;
        let measured = row.linear.minus(row.linear_exact);
        let closed_form = Exact {
            numerator: 255 * (hu + 160),
            denominator: 159_600,
        };
        if hu > -160 && hu <= 239 {
            assert!(
                measured.equals(closed_form),
                "at {hu} the two hand-computed values differ by {measured:?} \
                 and the closed form gives {closed_form:?}"
            );
        } else {
            assert!(
                measured.equals(Exact::whole(0)),
                "at {hu} both functions clamp, so the divergence is zero and \
                 not {measured:?}"
            );
        }
    }
    // The window centre, which is the 0.32 HLD 18.3 calls its headline.
    // 102000/798 - 102000/800 is 255 * 200 / 159600, which is 51000/159600,
    // which is 0.3195488... of a display code.
    let centre = linear(40).minus(linear_exact(40));
    assert!(
        centre.equals(Exact {
            numerator: 51_000,
            denominator: 159_600,
        }),
        "the headline divergence is {centre:?}"
    );
    // Less than half a code, so no quantiser can turn it into a code
    // difference at this input. `2 * numerator < denominator` is that
    // comparison in integers.
    assert!(2 * centre.numerator < centre.denominator);
}

/// The window centre is the one place in this file where the rounding rule
/// decides, and that is checked rather than asserted from memory: of the forty
/// display values the file asserts, `LINEAR_EXACT(40) = 255/2` is the only one
/// that lands on a half. Half away from zero and half to even both give 128
/// there while truncation gives 127, and half away from zero is the declared
/// rule.
#[test]
fn exactly_one_asserted_value_lands_on_a_half() {
    let mut halves: Vec<String> = Vec::new();
    for offset in 0..16_i128 {
        let hu = 100 + offset;
        if linear(hu).is_a_half() {
            halves.push(format!("LINEAR({hu})"));
        }
        if linear_exact(hu).is_a_half() {
            halves.push(format!("LINEAR_EXACT({hu})"));
        }
    }
    for row in &ROWS_18_3 {
        if row.linear.is_a_half() {
            halves.push(format!("LINEAR({})", row.hu));
        }
        if row.linear_exact.is_a_half() {
            halves.push(format!("LINEAR_EXACT({})", row.hu));
        }
    }
    assert_eq!(halves, vec!["LINEAR_EXACT(40)".to_owned()]);
    // And the declared rule takes it up. `2n == 255d` says the value is
    // exactly 127.5 without dividing.
    let centre = linear_exact(40);
    assert_eq!(centre.numerator * 2, centre.denominator * 255);
    assert_eq!(centre.code(), 128);
}

/// The thirty-two codes in the two tables above, against HLD 18.2's formulas.
///
/// The tables are the fixture's pixel data: `divergence_pair` builds both
/// frames from them, so every statistic this file asserts rests on them, and
/// until now nothing checked a single one of the thirty-two against the
/// specification they were hand-computed from.
#[test]
fn every_code_in_the_two_tables_follows_from_18_2s_formulas() -> Outcome {
    for (index, code) in LINEAR_CODES.iter().enumerate() {
        let hu = 100 + i128::try_from(index)?;
        assert_eq!(
            linear(hu).code(),
            i128::from(*code),
            "LINEAR({hu}) quantises to {} and the table says {code}",
            linear(hu).code()
        );
    }
    for (index, code) in LINEAR_EXACT_CODES.iter().enumerate() {
        let hu = 100 + i128::try_from(index)?;
        assert_eq!(
            linear_exact(hu).code(),
            i128::from(*code),
            "LINEAR_EXACT({hu}) quantises to {} and the table says {code}",
            linear_exact(hu).code()
        );
    }
    // The seven differing rows `DIFFERING` names, counted from the formulas
    // rather than from the tables, so the constant is checked too.
    let differing = (0..16)
        .filter(|index| {
            let hu = 100 + i128::from(*index);
            linear(hu).code() != linear_exact(hu).code()
        })
        .count();
    assert_eq!(u64::try_from(differing)?, DIFFERING);
    Ok(())
}

// ---------------------------------------------------------------------------
// The two display-extreme exclusions of `Effect::VoiLinearExactSwap`.
//
// **This section exists because the same block carried a wrong consequence of
// PS3.3 through four consecutive sprint review passes**, each time in a
// comment that HLD 27.3 tells a human to check against the cited section "not
// against the comment above it". Four comments and one LLD paragraph said the
// exclusion of the display value 255 was "exact at `w >= 510`". It is exact at
// no width, and nothing in the suite went red at any point.
//
// The mutation in `tools/oracle/src/mutations.rs` skips every pixel the
// reference rendered 0 or 255, because an 8-bit frame does not carry the
// stored value behind either. Both exclusions are safe in direction, since
// skipping a pixel that could have moved under-damages the frame and never
// over-damages it, so this was a false derivation and never a wrong pixel.
// What is asserted below is which of the two is exact and which is merely
// safe, from HLD 18.2's transcribed formulas and from nothing else.
//
// The derivation, in one line. Where NEITHER function clamps,
//
//     y_L - y_E = 255 * [ (x - c')/w' - (x - c)/w ]
//               = 255 * (x - c + w/2) / (w * w')
//               = y_L / w
//
// because `y_L = 255 * (x - c + w/2) / w'` by the same algebra. Everything
// else follows from that identity and from the rounding rule this file
// declares in its header.
// ---------------------------------------------------------------------------

/// One width and the integer stored values that move at it.
///
/// `movers` is every integer `x` at `CENTRE` that LINEAR quantises to 255 and
/// LINEAR_EXACT does not. Reproduce the whole table with
/// `cargo test -p ocelli-oracle --test voi_divergence_fixture`.
struct WhiteBand {
    width: i128,
    movers: &'static [i128],
}

/// The twelve widths this section varies over, with the movers hand-computed
/// at each.
///
/// 100 and 256 bracket 255, and 509, 510, 511 and 512 bracket 510, because
/// those are the two numbers the old derivation treated as boundaries. Two
/// rows worked by hand, at `c = 40`, `ymin = 0`, `ymax = 255`:
///
/// ```text
/// w = 400   c' = 39.5   w' = 399
///   LINEAR(239)       = ((239 - 39.5) / 399 + 0.5) * 255
///                     = (0.5 + 0.5) * 255 = 255.0000  -> 255
///   LINEAR_EXACT(239) = ((239 - 40) / 400 + 0.5) * 255
///                     = (0.4975 + 0.5) * 255 = 254.3625 -> 254   MOVES
///
/// w = 512   c' = 39.5   w' = 511
///   LINEAR(294)       = ((294 - 39.5) / 511 + 0.5) * 255
///                     = (0.4980431 + 0.5) * 255 = 254.5010 -> 255
///   LINEAR_EXACT(294) = ((294 - 40) / 512 + 0.5) * 255
///                     = (0.4960938 + 0.5) * 255 = 253.9539 -> 254   MOVES
/// ```
///
/// **The second of those is the whole finding.** 512 is above 510, where four
/// review passes said a 255 "cannot move for any stored value whatever", and a
/// stored value moves there.
///
/// The two empty rows, 255 and 510, are where the integers happen to fall and
/// not a property of either width. They sit on opposite sides of 510, which is
/// the shortest statement that 510 is not a threshold separating two regimes.
const WHITE_BANDS: [WhiteBand; 12] = [
    WhiteBand {
        width: 100,
        movers: &[89],
    },
    WhiteBand {
        width: 255,
        movers: &[],
    },
    WhiteBand {
        width: 256,
        movers: &[167],
    },
    WhiteBand {
        width: 400,
        movers: &[239],
    },
    WhiteBand {
        width: 509,
        movers: &[293],
    },
    WhiteBand {
        width: 510,
        movers: &[],
    },
    WhiteBand {
        width: 511,
        movers: &[294],
    },
    WhiteBand {
        width: 512,
        movers: &[294],
    },
    WhiteBand {
        width: 600,
        movers: &[338],
    },
    WhiteBand {
        width: 1000,
        movers: &[538],
    },
    WhiteBand {
        width: 2048,
        movers: &[1059],
    },
    WhiteBand {
        width: 4096,
        movers: &[2079],
    },
];

/// 254.5, the lowest display value that quantises to 255 under this file's
/// declared rounding rule, as an exact rational.
const HALF_ABOVE_254: Exact = Exact {
    numerator: 509,
    denominator: 2,
};

/// Every integer stored value from `CENTRE - w` to `CENTRE + w`, which covers
/// the whole window and a half-window of margin at each end, so no boundary
/// case can fall outside the scan.
fn scanned(width: i128) -> impl Iterator<Item = i128> {
    (CENTRE - width)..=(CENTRE + width)
}

/// The stored values LINEAR renders 255 and LINEAR_EXACT does not.
fn movers_at_255(width: i128) -> Vec<i128> {
    scanned(width)
        .filter(|x| linear_at(*x, width).code() == 255 && linear_exact_at(*x, width).code() != 255)
        .collect()
}

/// **The identity the whole section rests on**, over every width and every
/// stored value where neither function clamps.
///
/// `LINEAR(x) - LINEAR_EXACT(x) = y_L / w` exactly. The closed form asserted
/// further up this file, `255 * (x + 160) / 159600`, is this same identity at
/// one window, and this generalises it so the exclusion arguments below can be
/// stated at any width.
///
/// The unclamped region in integers: LINEAR_EXACT's lower clamp is
/// `2x <= 2c - w`, LINEAR's is the same number, and LINEAR's upper clamp is
/// `2x > 2c' + w' = 2c + w - 2`, which is the earlier of the two upper ones.
#[test]
fn the_divergence_is_the_display_value_over_the_width() {
    let mut checked = 0_u32;
    for band in &WHITE_BANDS {
        let w = band.width;
        for x in scanned(w) {
            if 2 * x <= 2 * CENTRE - w || 2 * x > 2 * CENTRE + w - 2 {
                continue;
            }
            let y_l = linear_at(x, w);
            let measured = y_l.minus(linear_exact_at(x, w));
            assert!(
                measured.equals(y_l.scaled(1, w)),
                "at w = {w}, x = {x} the divergence is {measured:?} and \
                 y_L / w is {:?}",
                y_l.scaled(1, w)
            );
            checked += 1;
        }
    }
    assert!(checked > 10_000, "only {checked} inputs were checked");
}

/// **The exclusion at 255 is conservative at every width and exact at none.**
///
/// A pixel the reference rendered 255 has `round(y_L) = 255`, so
/// `y_L >= 254.5`, and it moves when `round(y_E) != 255`, that is when
/// `y_L - y_L / w < 254.5`, that is when `y_L < 254.5 * w / (w - 1)`. That
/// band is non-empty at every width, so some `y_L` always moves, and whether
/// an INTEGER stored value lands in it is a separate question this table
/// answers width by width.
///
/// The old comment said a 255 "cannot move for any stored value whatever" at
/// `w >= 510`. Six of the rows below are above 510, being 511, 512, 600, 1000,
/// 2048 and 4096, and every one of the six carries a mover. Both counts are
/// asserted at the foot of this test rather than only written here, because
/// the sentence that stood before this one said five and four and the
/// assertions twenty lines below it already said six and six.
#[test]
fn the_white_exclusion_is_conservative_at_every_width_and_exact_at_none() {
    for band in &WHITE_BANDS {
        let w = band.width;
        assert_eq!(
            movers_at_255(w),
            band.movers,
            "at w = {w} the stored values LINEAR renders 255 and LINEAR_EXACT \
             does not are not the hand-computed ones"
        );
        for x in band.movers {
            assert_eq!(linear_at(*x, w).code(), 255);
            assert!(
                linear_exact_at(*x, w).code() < 255,
                "the swap can only ever DARKEN a pixel, so excluding this one \
                 under-damages the frame and cannot over-damage it"
            );
        }
    }
    // 510 is not a threshold, and this is the assertion that says so. Every
    // width above it carries a mover, and one width below it carries none, so
    // the presence of a mover does not sort the widths by 510 in either
    // direction.
    let above = WHITE_BANDS.iter().filter(|band| band.width > 510).count();
    let above_with_movers = WHITE_BANDS
        .iter()
        .filter(|band| band.width > 510 && !band.movers.is_empty())
        .count();
    let below_without = WHITE_BANDS
        .iter()
        .filter(|band| band.width < 510 && band.movers.is_empty())
        .count();
    assert_eq!(above, 6, "511, 512, 600, 1000, 2048 and 4096");
    assert_eq!(
        above_with_movers, above,
        "every one of the six widths above 510 carries a stored value that \
         moves, where the old derivation said none of them could"
    );
    assert_eq!(below_without, 1, "255 is a width below 510 with no mover");
}

/// **The movable band, pinned at both endpoints, in display-value space and
/// then in stored-value space.**
///
/// `y_L` is the display value LINEAR produces, so `0 <= y_L <= 255`. Where
/// neither function clamps, `y_E = y_L * (w - 1) / w`, so a pixel the
/// reference rendered 255 has `y_L >= 254.5` and moves exactly when
/// `y_L < 254.5 * w / (w - 1)`. The movable set is therefore
///
/// ```text
/// y_L in [254.5, 254.5 * w / (w - 1))   intersected with   [0, 255]
/// ```
///
/// which is the CLOSED `[254.5, 255]` for `w < 510`, `[254.5, 255)` at
/// `w = 510`, and strictly inside `[254.5, 255)` above it.
///
/// **The intersection is what an earlier `min(255, ...)` with an open top got
/// wrong, and it excluded three of the twelve rows this file tabulates.** The
/// formula's top is open, but below 510 it lies ABOVE 255, so no `y_L` ever
/// reaches it and every display value up to and INCLUDING 255 moves. `y_L` is
/// exactly 255 at `x = c + w/2 - 1`, the last stored value LINEAR does not
/// clamp, and at `w = 100`, 256 and 400 that stored value is the only mover
/// the table above carries, `w = 400, x = 239` being the row the table's own
/// header hand-works and marks MOVES. Writing the top as `min(255, ...)` and
/// leaving the bracket open put that stored value in neither the movable band
/// nor the clamped interval `(c + w/2 - 1, c + w/2]`, which is open at its
/// left.
///
/// Above `c + w/2 - 1` LINEAR has clamped, the identity `y_E = y_L (w - 1)/w`
/// no longer holds, and display-value space can no longer say which of those
/// pixels move. **Stored-value space can, with no case split at all**, which
/// is the second half of this test and the reason the two endpoints below are
/// solved rather than written down.
#[test]
fn the_movable_band_is_non_empty_at_every_width() {
    assert_eq!(
        HALF_ABOVE_254.code(),
        255,
        "254.5 rounds half away from zero"
    );
    for band in &WHITE_BANDS {
        let w = band.width;

        // The lower boundary moves, at every width.
        let low_moved = HALF_ABOVE_254.scaled(w - 1, w);
        assert!(low_moved.is_below(HALF_ABOVE_254));
        assert!(
            low_moved.code() < 255,
            "at w = {w} the bottom of the band does not move, so there is no \
             band at all"
        );

        // The upper boundary does not, at every width. `254.5 * w / (w - 1)`
        // times `(w - 1) / w` is 254.5 by construction, which is the exact
        // statement that the band is half open.
        let upper = HALF_ABOVE_254.scaled(w, w - 1);
        assert!(
            HALF_ABOVE_254.is_below(upper),
            "at w = {w} the band is empty, and it is empty at no width"
        );
        assert!(upper.scaled(w - 1, w).equals(HALF_ABOVE_254));
        assert_eq!(upper.scaled(w - 1, w).code(), 255);

        // The width of the band, `254.5 / (w - 1)` capped at 0.5. The cap
        // binds exactly when `254.5 * w / (w - 1) >= 255`, which is `w <= 510`,
        // and that inequality is the ONLY place 510 comes from.
        let uncapped = Exact {
            numerator: 509,
            denominator: 2 * (w - 1),
        };
        let half = Exact {
            numerator: 1,
            denominator: 2,
        };
        let capped = if half.is_below(uncapped) {
            half
        } else {
            uncapped
        };
        // **`top` is where the band's MEASURE stops and is NOT its membership
        // test.** A display value cannot exceed 255, so the part of
        // `[254.5, upper)` that any `y_L` can occupy stops at 255 and the
        // measure is `min(upper, 255) - 254.5`. Membership is `y_L < upper`
        // and nothing else, so below 510, where `upper` is above 255, the
        // display value 255 IS in the band. Reading this `min` as the
        // membership test is what excluded it, and with it three of the twelve
        // rows above.
        let top = if upper.is_below(Exact::whole(255)) {
            upper
        } else {
            Exact::whole(255)
        };
        assert!(
            top.minus(HALF_ABOVE_254).equals(capped),
            "at w = {w} the band width is {:?} and 254.5 / (w - 1) capped at \
             0.5 is {capped:?}",
            top.minus(HALF_ABOVE_254)
        );
        assert_eq!(
            w <= 510,
            !upper.is_below(Exact::whole(255)),
            "254.5 * w / (w - 1) >= 255 exactly when w <= 510"
        );

        // The endpoint the open bracket excluded. `y_L = 255` moves exactly
        // when `w < 510`, one width tighter than the measure's cap, because at
        // `w = 510` the top is 255 exactly and the top is open. So the band is
        // `[254.5, 255]` below 510 and `[254.5, 255)` at it.
        let white_moves = Exact::whole(255).scaled(w - 1, w).code() < 255;
        assert_eq!(
            white_moves,
            w < 510,
            "at w = {w} the display value 255 is the endpoint that decides \
             whether the band is closed, and it moves exactly below 510"
        );

        // And at an even width `c + w/2 - 1` is the integer stored value where
        // `y_L` is exactly 255, so the table above must carry it as a mover
        // exactly when the band is closed. At 100, 256 and 400 it is the only
        // mover in the row.
        if w % 2 == 0 {
            let at_255 = CENTRE + w / 2 - 1;
            assert!(linear_at(at_255, w).equals(Exact::whole(255)));
            assert_eq!(
                band.movers.contains(&at_255),
                w < 510,
                "at w = {w} the stored value {at_255} renders 255 under \
                 LINEAR, and whether it moves is exactly whether the band is \
                 closed at 255"
            );
        }

        // **The same band in STORED-VALUE units, where it is ONE contiguous
        // interval `509/510` of an input unit wide at every width, with no
        // case split anywhere in it.** Write `u = x - c`. Both endpoints are
        // solved from HLD 18.2's formulas and then CHECKED BY EVALUATING THEM
        // THERE, which is the whole point of this half: the previous version
        // asserted a free-standing `(510 - w)/510` that neither formula
        // produces, and mutating the LINEAR transcription left this test
        // green while six others went red.
        //
        //   y_L(x) = 254.5   is   255 (u + w/2) / (w - 1) = 509/2
        //                    so   u = 509(w - 1)/510 - w/2 = (254w - 509)/510
        //   y_E(x) = 254.5   is   255 (u + w/2) / w = 509/2
        //                    so   u = 509w/510 - w/2 = 254w/510
        //
        // `y_L` is non-decreasing in `x` and is at or above 254.5 exactly for
        // `u >= u_start`, INCLUDING where it clamps to 255, and `y_E` is
        // non-decreasing and under 254.5 exactly for `u < u_end`. A stored
        // value moves exactly when both hold, so the movable set is
        // `[u_start, u_end)` at every width, of measure
        // `254w/510 - (254w - 509)/510 = 509/510`. That single interval
        // subsumes the pixels LINEAR ROUNDED up to 255 and the pixels it
        // CLAMPED to 255, so 510 sorts nothing here: it moves the clamp point
        // `w/2 - 1` across the interval and changes neither endpoint.
        let u_start = Exact {
            numerator: 254 * w - 509,
            denominator: 510,
        };
        let u_end = Exact {
            numerator: 254 * w,
            denominator: 510,
        };
        assert!(
            linear_at_rational(stored_at(u_start), w).equals(HALF_ABOVE_254),
            "at w = {w} LINEAR at u_start is {:?} and not 254.5",
            linear_at_rational(stored_at(u_start), w)
        );
        assert!(
            linear_exact_at_rational(stored_at(u_end), w).equals(HALF_ABOVE_254),
            "at w = {w} LINEAR_EXACT at u_end is {:?} and not 254.5",
            linear_exact_at_rational(stored_at(u_end), w)
        );
        assert!(
            u_end.minus(u_start).equals(Exact {
                numerator: 509,
                denominator: 510,
            }),
            "at w = {w} the movable band is {:?} input units wide and it is \
             509/510 at every width",
            u_end.minus(u_start)
        );

        // And the integers inside that interval are the table's movers, which
        // is what ties the closed form to the twelve hand-computed rows. The
        // half-open bracket is load-bearing here: at `w = 255` the only
        // candidate stored value sits exactly ON `u_end`, which is why that
        // row is empty.
        let inside: Vec<i128> = scanned(w)
            .filter(|x| {
                let u = Exact::whole(*x - CENTRE);
                !u.is_below(u_start) && u.is_below(u_end)
            })
            .collect();
        assert_eq!(
            inside, band.movers,
            "at w = {w} the integers in [u_start, u_end) are not the stored \
             values the two formulas move"
        );
    }
}

/// **What `w >= 510` actually buys is only that a CLAMPED pixel cannot move**,
/// and reading that as the whole statement is the defect this section exists
/// for.
///
/// On `(c + w/2 - 1, c + w/2]` LINEAR has clamped to `ymax` and LINEAR_EXACT
/// has not, and the lowest `y_E` anywhere `y_L` is 255 is `255 - 255/w`, taken
/// at `x = c + w/2 - 1`. It quantises to 255 exactly when `255/w <= 0.5`,
/// which is `w >= 510`.
///
/// So above 510 no pixel that is 255 BY CLAMPING can move. The pixels LINEAR
/// ROUNDED up to 255 from below are a different population, the band above
/// covers them, and it never closes. Both halves are asserted here, because
/// the true half on its own is what four passes mistook for the whole.
#[test]
fn a_width_of_510_buys_only_that_a_clamped_pixel_cannot_move() {
    for band in &WHITE_BANDS {
        let w = band.width;
        // `255 - 255/w`, the worst LINEAR_EXACT value under a LINEAR 255.
        let worst = Exact {
            numerator: 255 * (w - 1),
            denominator: w,
        };
        assert_eq!(
            worst.code() == 255,
            w >= 510,
            "at w = {w} the lowest y_E under a display value of 255 is \
             {worst:?}, which quantises to {}",
            worst.code()
        );
        if w % 2 == 0 {
            // `c + w/2 - 1` is an integer at an even width, it is the last
            // stored value LINEAR does not clamp, and LINEAR is exactly 255
            // there. This is where `255 - 255/w` is taken.
            let top = CENTRE + w / 2 - 1;
            assert!(linear_at(top, w).equals(Exact::whole(255)));
            assert!(linear_exact_at(top, w).equals(worst));
        }
    }
    // The punchline. At 512 and at 4096 a 255 reached by clamping cannot move,
    // and a stored value moves anyway.
    for width in [512_i128, 4096] {
        let worst = Exact {
            numerator: 255 * (width - 1),
            denominator: width,
        };
        assert_eq!(worst.code(), 255, "no clamped pixel moves at w = {width}");
        assert!(
            !movers_at_255(width).is_empty(),
            "and yet a stored value moves at w = {width}, which is why the \
             exclusion is conservative here and not exact"
        );
    }
}

/// **The exclusion at 0 IS exact at every width, and coincident clamps are
/// only half the reason.**
///
/// The lower clamps coincide, `c' - w'/2 = (c - 0.5) - (w - 1)/2 = c - w/2`,
/// so a stored value clamping to black under one function clamps under the
/// other. That alone says nothing about the unclamped values just above them,
/// and the comment in `mutations.rs` used to stop there. What carries the rest
/// is that `y_E = y_L * (w - 1) / w` lies in `[0, y_L]` wherever neither
/// function clamps, so `round(y_L) = 0` forces `round(y_E) = 0`. The upper
/// bracket is CLOSED: `y_E = y_L` at `y_L = 0`, where the half-open form this
/// used to be written in is the empty interval and states nothing at the one
/// value the paragraph is about.
///
/// Both are asserted: the ordering `y_E <= y_L` at every scanned input, and
/// the consequence, that no stored value at any width renders 0 under LINEAR
/// and something else under LINEAR_EXACT. The exhaustive half is what the
/// white table above cannot say.
#[test]
fn the_black_exclusion_is_exact_at_every_width() {
    for band in &WHITE_BANDS {
        let w = band.width;
        let mut blacks = 0_u32;
        for x in scanned(w) {
            let y_l = linear_at(x, w);
            let y_e = linear_exact_at(x, w);
            assert!(
                !y_l.is_below(y_e),
                "at w = {w}, x = {x} LINEAR_EXACT is ABOVE LINEAR, which the \
                 swap's direction depends on being impossible"
            );
            if y_l.code() == 0 {
                blacks += 1;
                assert_eq!(
                    y_e.code(),
                    0,
                    "at w = {w} the stored value {x} renders 0 under LINEAR \
                     and {} under LINEAR_EXACT, so excluding 0 is not exact",
                    y_e.code()
                );
            }
        }
        assert!(
            blacks > 0,
            "no input rendered 0 at w = {w}, so the width \
             proves nothing"
        );
    }
}
