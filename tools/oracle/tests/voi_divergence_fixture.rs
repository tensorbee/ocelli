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
/// tolerance. Denominators are the constants `2(w - 1)` and `2w`, at most
/// 800, and numerators are at most `255 * 800`, so the cross products below
/// are around 1.6e8 and `i128` cannot overflow.
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

    /// `self - other`, exact.
    fn minus(self, other: Self) -> Self {
        Self {
            numerator: self.numerator * other.denominator - other.numerator * self.denominator,
            denominator: self.denominator * other.denominator,
        }
    }
}

/// PS3.3 C.11.2.1.2, LINEAR, at this file's window, in exact rationals.
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
fn linear(x: i128) -> Exact {
    let two_c_prime = 2 * CENTRE - 1;
    let w_prime = WIDTH - 1;
    if 2 * x <= two_c_prime - w_prime {
        return Exact::whole(Y_MIN);
    }
    if 2 * x > two_c_prime + w_prime {
        return Exact::whole(Y_MAX);
    }
    Exact {
        numerator: (2 * x - two_c_prime + w_prime) * (Y_MAX - Y_MIN) + Y_MIN * 2 * w_prime,
        denominator: 2 * w_prime,
    }
}

/// PS3.3 C.11.2.1.3.2, LINEAR_EXACT, the same way.
///
/// ```text
/// x <= c - w/2   becomes   2x <= 2c - w
/// x >  c + w/2   becomes   2x >  2c + w
/// y = ((x - c)/w + 1/2) * (ymax - ymin) + ymin
///   = [ (2x - 2c) + w ] * (ymax - ymin) / (2w) + ymin
/// ```
fn linear_exact(x: i128) -> Exact {
    if 2 * x <= 2 * CENTRE - WIDTH {
        return Exact::whole(Y_MIN);
    }
    if 2 * x > 2 * CENTRE + WIDTH {
        return Exact::whole(Y_MAX);
    }
    Exact {
        numerator: (2 * x - 2 * CENTRE + WIDTH) * (Y_MAX - Y_MIN) + Y_MIN * 2 * WIDTH,
        denominator: 2 * WIDTH,
    }
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
