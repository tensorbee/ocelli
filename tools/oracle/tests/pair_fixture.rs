//! `pair_verdict`, the two-frame preconditions check that F-223's comparison
//! mode runs before it believes any timing figure.
//!
//! Every expected number here is hand-computed from the transcribed text of
//! `docs/hld/22-testing-and-tolerance.md` section 25.1 and NOT from running
//! the comparator. HLD 27.2 R2: an agent asked to test a function will assert
//! what it does rather than what it should do, so the counts below are worked
//! out from the quoted rules. The rule, verbatim from 25.1, the `≤` and the
//! semicolon included:
//!
//! > **Monochrome 16-bit (CT, MR, CR, DR):** maximum absolute difference ≤ 1
//! > LSB on at least 99.9% of pixels; zero pixels differing by more than 2.
//!
//! > **Systematic bias, monochrome:** signed mean difference over the
//! > informative region within 0.1 of one display code, evaluated only where
//! > input identity, declared parameters and geometry already agree.
//!
//! Two readings are stated at the site and repeated here because the fixtures
//! depend on them. "1 LSB" is one 8-bit display code, the reading
//! `MONOCHROME_ONE_LSB` documents. The informative region is the pixels not
//! clipped to the same extreme on both sides, per `frame.rs`: a pixel at 0 or
//! 255 on BOTH sides is not informative, and an identical mid-grey pixel is.
//!
//! The informative fraction's floor is NOT from 25.1. It is
//! `INFORMATIVE_FRACTION_FLOOR`, the declared judgement carried beside the
//! constants in `tolerance.rs`, and the low-information fixture below is the
//! frame a verdict without a floor would pass: every statistic over it says
//! the two sides agree, and they agree only because almost none of the frame
//! can express a disagreement.
//!
//! **No `unwrap`, no `expect` and no `panic!`.** Each test returns a `Result`
//! and the harness fails it on `Err`, which is the shape HLD section 23's
//! denials leave available.

use std::error::Error;

use ocelli_oracle::frame::Frame;
use ocelli_oracle::tolerance::pair_verdict;

type Outcome = Result<(), Box<dyn Error>>;

/// 100 by 100 is 10000 pixels, so every fraction below is an exact integer
/// count over 10000 and no boundary lands between two pixels. The one small
/// fixture says its own size, because the low-information case needs a frame
/// where five informative pixels are five per cent.
const SIDE: u32 = 100;
const PIXELS: usize = 10_000;

/// Mid-grey, far from both extremes. A pixel at 128 is informative whether it
/// differs or not, and a small delta at 128 cannot clip into a smaller one.
const BASE: u8 = 128;

/// The black that stands in for a clipped background. At 0 on BOTH sides a
/// pixel is clipped to the same extreme, so it is not informative, and the
/// bias bound never sees it.
const BLACK: u8 = 0;

/// Build a reference and a candidate over one 100 by 100 frame from two
/// per-index grey values. Both are written by the caller as literals so the
/// fixture stays hand-computed.
fn frames(
    reference: impl Fn(usize) -> u8,
    candidate: impl Fn(usize) -> u8,
) -> Result<(Frame, Frame), Box<dyn Error>> {
    let mut left = Vec::with_capacity(PIXELS);
    let mut right = Vec::with_capacity(PIXELS);
    for index in 0..PIXELS {
        left.push(reference(index));
        right.push(candidate(index));
    }
    Ok((
        Frame::from_monochrome(SIDE, SIDE, &left)?,
        Frame::from_monochrome(SIDE, SIDE, &right)?,
    ))
}

/// The measured bias as a plain `f64` for the `to_bits` comparisons. Every
/// caller here compares an informative region with at least one pixel, so
/// `None` is a fixture bug and not a value, and the test's own `Err` return
/// is how it fails: no `unwrap`, no `expect`, no `panic!`.
fn measured(bias: Option<f64>) -> Result<f64, Box<dyn Error>> {
    bias.ok_or_else(|| "the bias was not measured, so there is no figure to compare".into())
}

/// Two identical mid-grey frames. Every clause of 25.1 is at its best case:
/// the within-one fraction is 1.0, the over-maximum count is 0, the bias is
/// exactly zero, every pixel is informative, and the verdict passes.
#[test]
fn an_identical_pair_passes() -> Outcome {
    let (reference, candidate) = frames(|_| BASE, |_| BASE)?;
    let verdict = pair_verdict(&reference, &candidate)?;
    assert_eq!(verdict.within_one_lsb_fraction.to_bits(), 1.0_f64.to_bits());
    assert_eq!(verdict.count_over_max, 0);
    assert_eq!(verdict.max_abs_diff, 0);
    assert_eq!(
        measured(verdict.signed_mean_bias_informative)?.to_bits(),
        0.0_f64.to_bits()
    );
    assert_eq!(verdict.informative_fraction.to_bits(), 1.0_f64.to_bits());
    assert!(!verdict.low_information);
    assert!(verdict.passes, "an identical pair is the passing case");
    Ok(())
}

/// Boundary two of the monochrome rule. One pixel at +3 over a mid-grey
/// frame: 9999 of 10000 pixels are within 1 LSB, which satisfies "at least
/// 99.9%", and the bias is 3 / 10000 = 0.0003, which satisfies the bias
/// bullet. The verdict still refuses, because "zero pixels differing by more
/// than 2" admits no count of them at all.
#[test]
fn one_pixel_at_difference_three_fails_at_any_count() -> Outcome {
    // 128 + 3 is 131, inside the 8-bit range, so the delta cannot clip.
    let (reference, candidate) =
        frames(|_| BASE, |index| if index == 0 { BASE + 3 } else { BASE })?;
    let verdict = pair_verdict(&reference, &candidate)?;
    assert_eq!(
        verdict.within_one_lsb_fraction.to_bits(),
        0.9999_f64.to_bits()
    );
    assert_eq!(verdict.count_over_max, 1);
    assert_eq!(verdict.max_abs_diff, 3);
    assert_eq!(
        measured(verdict.signed_mean_bias_informative)?.to_bits(),
        0.0003_f64.to_bits()
    );
    assert!(!verdict.passes, "one pixel over 2 refuses, at any count");
    Ok(())
}

/// The bias bullet, far over the bound. Every one of the 10000 pixels is one
/// code brighter, so the informative region is the whole frame, the
/// informative fraction is 1.0, the within-one fraction is 1.0 and the
/// monochrome rule passes. The signed mean is 10000 / 10000 = 1.0 display
/// codes, and 1.0 is not within 0.1 of zero, so the verdict refuses on the
/// bias alone.
#[test]
fn every_informative_pixel_one_code_brighter_fails_the_bias() -> Outcome {
    let (reference, candidate) = frames(|_| BASE, |_| BASE + 1)?;
    let verdict = pair_verdict(&reference, &candidate)?;
    assert_eq!(verdict.within_one_lsb_fraction.to_bits(), 1.0_f64.to_bits());
    assert_eq!(verdict.count_over_max, 0);
    assert_eq!(verdict.max_abs_diff, 1);
    assert_eq!(
        measured(verdict.signed_mean_bias_informative)?.to_bits(),
        1.0_f64.to_bits()
    );
    assert_eq!(verdict.informative_fraction.to_bits(), 1.0_f64.to_bits());
    assert!(!verdict.low_information);
    assert!(
        !verdict.passes,
        "a bias of one display code is not within 0.1"
    );
    Ok(())
}

/// **The clipped-background fixture, the one the mutation catalogue names.**
///
/// Pixels 0 to 7999 are black on both sides, clipped to the same extreme and
/// so not informative. Pixels 8000 to 9999 are mid-grey on both sides and so
/// informative, and the first 201 of them are one code brighter in the
/// candidate. The working:
///
/// - informative pixels: 2000, so the informative fraction is 2000 / 10000
///   = 0.2, above the floor and not low-information.
/// - informative bias: 201 / 2000 = 0.1005 display codes, over the 0.1 bound.
/// - image-rectangle bias: 201 / 10000 = 0.0201, inside the bound, which is
///   the dilution a clipped background produces.
/// - monochrome rule: every difference is at most 1, so 1.0 within one LSB
///   and nothing over the maximum.
///
/// The verdict refuses because the bias over the INFORMATIVE region is over
/// the bound. A verdict that took the bias over the image rectangle instead
/// would see 0.0201, pass every clause, and answer true, which is why the
/// region is load-bearing and why this fixture exists.
#[test]
fn the_clipped_background_dilutes_the_image_bias_and_not_the_informative_bias() -> Outcome {
    let (reference, candidate) = frames(
        |index| if index < 8000 { BLACK } else { BASE },
        |index| {
            if index < 8000 {
                BLACK
            } else if index < 8201 {
                BASE + 1
            } else {
                BASE
            }
        },
    )?;
    let verdict = pair_verdict(&reference, &candidate)?;
    assert_eq!(
        measured(verdict.signed_mean_bias_informative)?.to_bits(),
        0.1005_f64.to_bits()
    );
    assert_eq!(verdict.informative_fraction.to_bits(), 0.2_f64.to_bits());
    assert!(!verdict.low_information);
    assert_eq!(verdict.within_one_lsb_fraction.to_bits(), 1.0_f64.to_bits());
    assert_eq!(verdict.count_over_max, 0);
    assert!(
        !verdict.passes,
        "201 codes over 2000 informative pixels is a bias of 0.1005, over the \
         bound the image rectangle would have diluted to 0.0201"
    );
    Ok(())
}

/// The floor, at a frame size where five informative pixels are five per
/// cent. 95 pixels are clipped black on both sides and 5 are mid-grey and
/// identical, so every statistic over the pair is the passing value: the
/// within-one fraction is 1.0, the over-maximum count is 0, the bias is
/// exactly zero. The informative fraction is 5 / 100 = 0.05, below the 0.10
/// floor, so the verdict is low-information and refuses anyway. This is the
/// frame a verdict without the floor would pass.
#[test]
fn a_low_information_frame_does_not_pass_even_when_everything_agrees() -> Outcome {
    let mut greys = vec![BLACK; 100];
    for slot in greys.iter_mut().skip(95) {
        *slot = BASE;
    }
    let reference = Frame::from_monochrome(10, 10, &greys)?;
    let candidate = Frame::from_monochrome(10, 10, &greys)?;
    let verdict = pair_verdict(&reference, &candidate)?;
    assert_eq!(verdict.informative_fraction.to_bits(), 0.05_f64.to_bits());
    assert!(verdict.low_information, "0.05 is below the 0.10 floor");
    assert_eq!(verdict.within_one_lsb_fraction.to_bits(), 1.0_f64.to_bits());
    assert_eq!(verdict.count_over_max, 0);
    assert_eq!(
        measured(verdict.signed_mean_bias_informative)?.to_bits(),
        0.0_f64.to_bits()
    );
    assert!(
        !verdict.passes,
        "every clause passes and the verdict still refuses, on information \
         alone"
    );
    Ok(())
}

/// An all-clipped pair is the clearest low-information case there is, and it
/// is a verdict and not an error. Every pixel of both frames is 0, so no
/// pixel is informative on either side: the informative fraction is 0, the
/// bias over an empty region was not measured and is carried as `None`,
/// which serialises as `null`, and the monochrome rule over the
/// full frame is at its best case because every pixel agrees by
/// construction. The verdict refuses on information alone. Before this
/// fixture the empty region reached `bias_bound` and came back as the
/// empty-statistic error, and in between the verdict carried a placeholder
/// zero, so the harness would have seen either an argument-style failure on
/// exit 1 or a perfect bias beside a zero fraction.
#[test]
fn an_all_clipped_pair_is_a_low_information_verdict_and_not_an_error() -> Outcome {
    let reference = Frame::from_monochrome(10, 10, &[BLACK; 100])?;
    let candidate = Frame::from_monochrome(10, 10, &[BLACK; 100])?;
    let verdict = pair_verdict(&reference, &candidate)?;
    assert_eq!(verdict.informative_fraction.to_bits(), 0.0_f64.to_bits());
    assert!(
        verdict.signed_mean_bias_informative.is_none(),
        "the bias has no value over an empty region and none is invented"
    );
    assert!(
        verdict.low_information,
        "an informative fraction of zero is below the floor"
    );
    assert_eq!(
        verdict.within_one_lsb_fraction.to_bits(),
        1.0_f64.to_bits(),
        "every pixel agrees, over the full frame"
    );
    assert_eq!(verdict.count_over_max, 0);
    assert_eq!(verdict.max_abs_diff, 0);
    assert!(
        !verdict.passes,
        "a pair with no informative pixel refuses on information alone"
    );
    Ok(())
}

/// A bias that was not measured is null, per the ruling after review pass 2.
/// The published schema admits `signed_mean_bias_informative` as null only
/// when the informative fraction is exactly 0, with `low_information` then
/// true, and the reports say the bias was not measured. A placeholder zero
/// would publish the best possible bias beside an informative fraction of
/// zero, which the ruling refused: nothing over an empty region was
/// measured, and the JSON carries that as null and not as a number.
#[test]
fn an_all_clipped_pair_serialises_the_bias_as_null() -> Outcome {
    let reference = Frame::from_monochrome(10, 10, &[BLACK; 100])?;
    let candidate = Frame::from_monochrome(10, 10, &[BLACK; 100])?;
    let verdict = pair_verdict(&reference, &candidate)?;
    let text = serde_json::to_string(&verdict).map_err(|error| error.to_string())?;
    let parsed: serde_json::Value =
        serde_json::from_str(&text).map_err(|error| error.to_string())?;
    assert!(
        parsed
            .get("signed_mean_bias_informative")
            .is_some_and(serde_json::Value::is_null),
        "the bias over an empty informative region serialises as null and \
         not as a number, got {text}"
    );
    Ok(())
}

/// The boundary fixture: the floor and the bias bound are AT their limits,
/// and the verdict passes. Both are inclusive where they bind: the bias
/// bound by 25.1's own text, "within 0.1 of one display code", and the
/// floor by the plan's declared judgement, required as
/// `informative_fraction >= INFORMATIVE_FRACTION_FLOOR`. Ten informative
/// pixels of a hundred put the fraction at exactly 0.10 and one brightened
/// pixel among the ten puts the signed mean at exactly 0.1, both computed as
/// exact integer ratios, so no boundary lands between two pixels. A
/// comparison turned strict in either place, `<` at the floor or `<` at the
/// bias bound, answers false here and goes red. No other bound sits at its
/// limit here: the within-one fraction is 1.0 against a limit of 0.999 and
/// the maximum absolute difference is 1 against a limit of 2, and the 99.9
/// per cent boundary itself is pinned in `tolerance_fixture.rs`.
#[test]
fn the_boundaries_are_inclusive_at_the_limit() -> Outcome {
    let mut reference_greys = vec![BLACK; 100];
    let mut candidate_greys = vec![BLACK; 100];
    for slot in reference_greys.iter_mut().skip(90) {
        *slot = BASE;
    }
    for (index, slot) in candidate_greys.iter_mut().enumerate() {
        if index == 90 {
            *slot = BASE + 1;
        } else if index > 90 {
            *slot = BASE;
        }
    }
    let reference = Frame::from_monochrome(10, 10, &reference_greys)?;
    let candidate = Frame::from_monochrome(10, 10, &candidate_greys)?;
    let verdict = pair_verdict(&reference, &candidate)?;
    assert_eq!(verdict.informative_fraction.to_bits(), 0.1_f64.to_bits());
    assert!(
        !verdict.low_information,
        "0.10 is the floor, and the floor includes itself"
    );
    assert_eq!(
        measured(verdict.signed_mean_bias_informative)?.to_bits(),
        0.1_f64.to_bits(),
        "one code over ten informative pixels is exactly the bound"
    );
    assert_eq!(verdict.within_one_lsb_fraction.to_bits(), 1.0_f64.to_bits());
    assert_eq!(verdict.count_over_max, 0);
    assert_eq!(verdict.max_abs_diff, 1);
    assert!(
        verdict.passes,
        "at the floor and at the bias bound, with every clause inclusive, \
         the verdict passes"
    );
    Ok(())
}

/// Two frames of different sizes are two different pictures and there is
/// nothing to compare. The refusal is the frame error through the tolerance
/// error, and it says the sizes rather than comparing the overlap.
#[test]
fn frames_of_different_sizes_are_refused() -> Outcome {
    let reference = Frame::from_monochrome(2, 2, &[BASE; 4])?;
    let candidate = Frame::from_monochrome(2, 3, &[BASE; 6])?;
    let outcome = pair_verdict(&reference, &candidate);
    let Err(error) = outcome else {
        return Err("two differently sized frames were compared".into());
    };
    assert!(
        format!("{error}").contains("disagree about the frame size"),
        "the refusal names the size disagreement, got {error}"
    );
    Ok(())
}

/// An empty image region has no informative fraction and no statistics, and
/// reporting zero for either would invent evidence. A 0 by 0 pair is refused
/// rather than answered.
#[test]
fn an_empty_image_region_is_refused() -> Outcome {
    let reference = Frame::from_monochrome(0, 0, &[])?;
    let candidate = Frame::from_monochrome(0, 0, &[])?;
    let outcome = pair_verdict(&reference, &candidate);
    let Err(error) = outcome else {
        return Err("an empty region produced a verdict".into());
    };
    assert!(
        format!("{error}").contains("zero pixels"),
        "the refusal says the statistic had no pixels, got {error}"
    );
    Ok(())
}
