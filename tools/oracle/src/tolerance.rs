//! HLD section 25.1's tolerance policy, transcribed, as constants.
//!
//! > Write it down once and hold it. Tuning tolerance per failure is how a
//! > suite stops meaning anything.
//!
//! **These numbers are not this file's to choose.** They live in source rather
//! than in a configuration file precisely so that changing one is a diff a
//! reviewer sees, and `tests/tolerance_fixture.rs` asserts each constant
//! against the quoted text below. A tolerance change is a pull request with a
//! rationale, reviewed like code.
//!
//! One number here is NOT from 25.1, and it is marked as such:
//! `INFORMATIVE_FRACTION_FLOOR`. 25.1 states no floor, this comparator needs
//! one, and the design round's decision 5 put it here rather than in
//! `render-params.json` so that the reference half's configuration cannot
//! decide a comparator verdict.

use crate::frame::{ChannelStats, StatsError};

/// 25.1's first bullet, verbatim. Every character, the `≤` and the prose
/// semicolon included.
///
/// **They were written as `<=` and `,` until the sprint review's fifth pass,
/// on the stated grounds that this repository's prose checker required it
/// outside `docs/hld/`.** It does not.
/// `scripts/prose_check.py`'s `INCLUDE_PREFIXES` and `INCLUDE_EXACT` cover
/// `.claude/`, `docs/sprints/`, `docs/lld/`, `docs/runbooks/`,
/// `docs/spikes/` and a named list of root markdown, and no Rust source is in
/// scope for any gate. Reproduce with
/// `python3 -c "import sys; sys.path.insert(0, 'scripts'); import
/// prose_check; print(prose_check.in_scope('tools/oracle/src/tolerance.rs'))"`,
/// which prints `False`. The substitution bought nothing and cost the
/// quotation check its teeth, because it had to be applied to the
/// specification side as well and a `.replace(';', ",")` over the whole file
/// weakens every comparison in it.
pub const SECTION_25_1_MONOCHROME: &str = "Monochrome 16-bit (CT, MR, CR, DR): \
maximum absolute difference ≤ 1 LSB on at least 99.9% of pixels; zero pixels \
differing by more than 2.";

/// 25.1's bias bullet, added in S03 by operator decision through F-011's
/// design plan. Verbatim, like the rest.
pub const SECTION_25_1_BIAS: &str = "Systematic bias, monochrome: signed mean \
difference over the informative region within 0.1 of one display code, \
evaluated only where input identity, declared parameters and geometry already \
agree.";

/// 25.1's colour bullet.
pub const SECTION_25_1_COLOUR: &str = "Colour and ultrasound: perceptual \
difference below a stated threshold, because chroma subsampling and YBR \
conversion legitimately differ.";

/// 25.1's geometry bullet.
pub const SECTION_25_1_GEOMETRY: &str = "Geometry: world coordinates within \
1e-6 mm; canvas coordinates within a quarter pixel.";

/// "on at least 99.9% of pixels". At least, so the comparison is `>=`.
pub const MONOCHROME_WITHIN_ONE_LSB_FRACTION: f64 = 0.999;

/// "1 LSB", read as **one 8-bit display code**.
///
/// The reference emits RGBA8 canvas frames and nothing else, so a 16-bit
/// stored value never reaches this comparison on either side and the 16-bit
/// reading of "LSB" is not evaluable against this instrument at all. That is
/// decision 1 of F-011's design round, and it is stated here at the site so a
/// later reader cannot assume the stricter reading was meant and believe the
/// comparator is 256 times tighter than it is.
pub const MONOCHROME_ONE_LSB: u8 = 1;

/// "zero pixels differing by more than 2". More than, so a pixel differing by
/// exactly 2 is permitted, and one differing by 3 is not, at any count.
pub const MONOCHROME_MAX_ABS_DIFF: u8 = 2;

/// "within 0.1 of one display code". Within, so the bound includes 0.1.
///
/// The derivation is on the record in `docs/hld/22-testing-and-tolerance.md`
/// and in `docs/lld/comparator.md`: `LINEAR(x) - LINEAR_EXACT(x)` at the
/// soft-tissue window is `255 * (x + 160) / 159600`, which peaks at 0.6375 of
/// a display code and therefore never exceeds one code after quantisation, so
/// the maximum-difference rule above passes a whole-frame swap between the two
/// functions everywhere.
///
/// **0.32 is a per-pixel divergence and not a mean, and this said "mean" until
/// the sprint review's fourth pass.** It is the value of that formula AT the
/// window centre, `255 * (40 + 160) / 159600 = 51000 / 159600 = 0.3195`, which
/// is one pixel's divergence and not an average over any region. What the
/// oracle measures over a region is `mean(u) / w`, and on this corpus's
/// synthetic soft-tissue rows, whose content is spread across the window, that
/// lands close to 0.32 for exactly that reason. **Close, and not within a
/// thousandth, which is what this said until the sprint review's fifth pass.**
/// Twenty-two of the twenty-three synthetic rows at `windowWidth` 400 measure
/// between -0.3193 and -0.3200, so those are within 0.0007 of 0.32.
/// `synthetic/ct_multiframe_perframe` is -0.3185, which is 0.0015 away and
/// over the claimed bound. On the real soft-tissue CT rows it runs from -0.269
/// to -0.284.
///
/// The signs are the measurement's own. The swap makes the candidate darker,
/// so every bias it produces is negative, and 25.1's bound is two-sided.
///
/// So 0.1 sits about three times below what the swap actually produces on a
/// soft-tissue row and far above the zero expected when two implementations
/// agree. Reproduce with `./target/release/ocelli-compare census`, the
/// `biasInformative` column, rows at `windowWidth` 400.
pub const MONOCHROME_SIGNED_MEAN_BIAS: f64 = 0.1;

/// "world coordinates within 1e-6 mm".
pub const WORLD_TOLERANCE_MM: f64 = 1e-6;

/// "canvas coordinates within a quarter pixel".
pub const CANVAS_TOLERANCE_PIXELS: f64 = 0.25;

/// The percentile class two publishes in place of a bound nobody wrote. It is
/// a REPORTED statistic and it gates nothing, per deviation D-16.
pub const CLASS_TWO_REPORTED_PERCENTILE: f64 = 0.999;

/// **Not from 25.1.** The fraction of the image rectangle that must carry
/// evidence before a class-one view's pixel verdict is allowed to mean
/// anything.
///
/// A pixel clipped to the same extreme on both sides agrees by construction,
/// so a defect in the value behind it cannot show. 25.1's monochrome rule
/// permits 0.1% of the frame to differ by more than one code, so a view whose
/// informative subset is smaller than that budget could not fail the predicate
/// even if every informative pixel were wrong. That is the necessary
/// condition, and 0.10 keeps roughly two orders of magnitude above it, so a
/// defect touching one percent of the informative pixels can still fail.
///
/// It is a declared judgement and not a derivation, and it is stated that way
/// on purpose. Measured on today's corpus it separates cleanly: every one of
/// the twenty-two views `run.json` lists under `lowInformation` has an
/// informative fraction at or below 0.0583, and the lowest fraction among the
/// seventy-six it does not list is 0.1007. That separation is an observation
/// recorded after the fact, not the reason for the number.
pub const INFORMATIVE_FRACTION_FLOOR: f64 = 0.10;

/// The two classes 25.1 names.
///
/// It names them by modality, and this comparator resolves them from the
/// manifest's category tokens instead. Modality does not resolve: four corpus
/// rows are `OT` and one is `DX`, and 25.1's first bullet names neither, while
/// `scripts/corpus_check.py --coverage` already fails a row that declares no
/// class token. So the token is a checked declaration and the modality is not.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ToleranceClass {
    /// 25.1 bullet one. `mono16`.
    MonochromeSixteenBit,
    /// 25.1 bullet three. `colour` or `us`.
    ColourOrUltrasound,
}

impl ToleranceClass {
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::MonochromeSixteenBit => "mono16",
            Self::ColourOrUltrasound => "colour-or-us",
        }
    }
}

/// The class-one token, matching `CLASS_TOKENS` in `scripts/corpus_check.py`.
pub const MONOCHROME_TOKEN: &str = "mono16";

/// The class-two tokens, matching the same constant.
pub const COLOUR_OR_ULTRASOUND_TOKENS: [&str; 2] = ["colour", "us"];

#[derive(Debug, thiserror::Error)]
pub enum ToleranceError {
    #[error(
        "categories {0:?} declare both tolerance classes. A row is in one class \
         or the other, and a row claiming both has no evaluable bound at all"
    )]
    TwoClasses(Vec<String>),
    #[error(
        "categories {0:?} declare no tolerance class. One of mono16, colour or \
         us is required by HLD 25.1, and scripts/corpus_check.py --coverage \
         already refuses a manifest row without one, so a view reaching here \
         without one means the sidecar and the manifest have parted company"
    )]
    NoClass(Vec<String>),
    #[error("{0}")]
    Stats(#[from] StatsError),
}

/// Resolve the tolerance class from a view's manifest category tokens.
///
/// `synthetic/us_ybr_full_422.dcm` carries both `us` and `colour`, and both are
/// class two, so that row is class two and not a conflict. A row carrying
/// `mono16` alongside either of them IS a conflict, because the two classes
/// have different rules and nothing decides which applies.
///
/// # Errors
/// When the tokens declare both classes or neither.
pub fn class_from_categories(categories: &[String]) -> Result<ToleranceClass, ToleranceError> {
    let monochrome = categories.iter().any(|token| token == MONOCHROME_TOKEN);
    let colour = categories
        .iter()
        .any(|token| COLOUR_OR_ULTRASOUND_TOKENS.contains(&token.as_str()));
    match (monochrome, colour) {
        (true, true) => Err(ToleranceError::TwoClasses(categories.to_vec())),
        (true, false) => Ok(ToleranceClass::MonochromeSixteenBit),
        (false, true) => Ok(ToleranceClass::ColourOrUltrasound),
        (false, false) => Err(ToleranceError::NoClass(categories.to_vec())),
    }
}

/// 25.1's monochrome rule, evaluated. Over the FULL frame: 25.1 says "of
/// pixels" and excludes nothing, and the letterbox agrees by construction on
/// both sides, so counting it inflates the pass rate by up to a quarter. The
/// same statistics restricted to the image rectangle are reported alongside so
/// the inflation is visible rather than absorbed.
#[derive(Clone, Copy, Debug)]
pub struct MonochromeVerdict {
    pub within_one_lsb_fraction: f64,
    pub count_over_max: u64,
    pub max_abs_diff: u8,
    pub passes: bool,
}

/// # Errors
/// When the region is empty.
pub fn monochrome_predicate(stats: &ChannelStats) -> Result<MonochromeVerdict, ToleranceError> {
    let within_one_lsb_fraction = stats.fraction_within(MONOCHROME_ONE_LSB)?;
    let count_over_max = stats.count_over(MONOCHROME_MAX_ABS_DIFF);
    Ok(MonochromeVerdict {
        within_one_lsb_fraction,
        count_over_max,
        max_abs_diff: stats.max_abs_diff(),
        passes: within_one_lsb_fraction >= MONOCHROME_WITHIN_ONE_LSB_FRACTION
            && count_over_max == 0,
    })
}

/// 25.1's bias bullet, evaluated over the INFORMATIVE region.
///
/// Not the image rectangle. A pixel clipped to black or white on both
/// sides differs by nothing whatever the arithmetic underneath says, so
/// including it in the denominator divides a real divergence by pixels
/// that structurally cannot show one. Measured on this corpus, that
/// choice is the difference between detecting **0 of the 70** gating
/// class-one views and detecting 51 of them, the soft-tissue CT rows
/// where HLD 18.3's worked example lives among them.
///
/// **What this bound cannot catch, measured rather than reasoned.** The
/// per-pixel divergence between LINEAR and LINEAR_EXACT is exactly `u / w`,
/// where `u` is the LINEAR display value in 0 to 255 and `w` is the window
/// width, so the mean over a region is `mean(u) / w` and the bound fires only
/// where the informative mean display code exceeds `0.1 * w`.
///
/// That is a statement about CONTENT and not only about width. The absolute
/// limit is `255 / w`, so `w > 2550` is unreachable for any content, but the
/// practical limit bites far sooner.
///
/// Measured across the **70** gating class-one views on this corpus, by
/// applying the catalogue's own swap to each and reading the signed mean back:
/// **51 can fail this bound under the real divergence and 19 cannot.** The 19
/// are the fifteen `real/mr_eay131` stack rows and two of that subject's three
/// reformats, at windows from 678 to 881 and informative means from 0.058 to
/// 0.087, plus `synthetic/mr_nonsquare_spacing` at 2048 and
/// `synthetic/cr_monochrome1` at 4096. **The smallest blind window is 678, not
/// 2550.**
///
/// Three details in that list were wrong before the sprint review's fourth
/// pass and each is worth naming. It is 70 gating views and not 71, because
/// `real/dx_varepop/00000001.dcm` is `mono16` and is `unmeasured` for
/// `decimated`, so it gates nothing. Its CORONAL reformat is at -0.114 and
/// CAN fail, so "its two reformats" was true only of the count. And
/// `real/dx_varepop/00000001.dcm` is not one of "the two rows at 4096",
/// because only `synthetic/cr_monochrome1.dcm` gates at that width at all.
///
/// Reproduce every number here with `./target/release/ocelli-compare census`.
///
/// So this bound protects the soft-tissue CT rows, which is where HLD 18.3's
/// worked example lives, and does not protect a wide-window MR. Widening it is
/// not the answer, because the bound's job is to sit below a real divergence
/// and above rounding noise. The answer for those views is a second statistic
/// or a corpus row whose content reaches the bound, and neither is this
/// story's to invent.
#[derive(Clone, Copy, Debug)]
pub struct BiasVerdict {
    pub signed_mean_diff: f64,
    pub passes: bool,
}

/// # Errors
/// When the region is empty.
pub fn bias_bound(region: &ChannelStats) -> Result<BiasVerdict, ToleranceError> {
    let signed_mean_diff = region.signed_mean_diff()?;
    Ok(BiasVerdict {
        signed_mean_diff,
        passes: signed_mean_diff.abs() <= MONOCHROME_SIGNED_MEAN_BIAS,
    })
}

#[cfg(test)]
mod tests {
    /// Section 25.1 of `docs/hld/22-testing-and-tolerance.md`, from its own
    /// heading to the next heading of any level or to the end of the file.
    ///
    /// The empty string when the heading is absent, which the caller asserts
    /// against rather than reading as a vacuous pass.
    fn section_25_1(hld: &str) -> &str {
        let heading = "### 25.1 ";
        let Some(start) = hld.find(heading) else {
            return "";
        };
        let body = hld.get(start..).unwrap_or_default();
        let after_heading = body.get(heading.len()..).unwrap_or_default();
        match after_heading.find("\n#") {
            Some(end) => body.get(..heading.len() + end).unwrap_or_default(),
            None => body,
        }
    }

    /// The slice really is the section and really does stop at its end. A
    /// helper that silently returned the whole file would restore exactly the
    /// weakness this replaced.
    #[test]
    fn the_section_slice_is_the_section() {
        let hld = include_str!("../../../docs/hld/22-testing-and-tolerance.md");
        let section = section_25_1(hld);
        assert!(section.starts_with("### 25.1 "));
        assert!(section.len() < hld.len(), "it must not be the whole file");
        assert!(
            section.contains("Tuning tolerance per failure"),
            "the section's own opening line"
        );
        assert!(
            !section.contains("## 25. Testing"),
            "the parent heading is above 25.1 and must not be inside the slice"
        );
        assert_eq!(section_25_1("nothing here"), "");
    }

    /// **The transcribed bullets are compared against the specification.**
    ///
    /// They are presented as verbatim quotations of HLD 25.1 and until the
    /// sprint review's third pass nothing compared them to it, so
    /// `SECTION_25_1_BIAS` still said "image rectangle" for a bullet the same
    /// sprint had changed to "informative region". A quotation nobody checks is
    /// a paraphrase that has stopped being true.
    ///
    /// The comparison is on the bullet's FIRST SENTENCE, because that is the
    /// normative statement and the rest of the bullet is the reasoning behind
    /// it, which moves as measurements are added.
    ///
    /// **All four bullets, and until the fourth pass it was two.** The loop
    /// covered the bias and colour bullets only, and the reason was not
    /// declared anywhere: the monochrome and geometry bullets each carry a
    /// prose semicolon and the monochrome one a `≤`, which the constants used
    /// to write as `,` and `<=`. The fourth pass added the missing two by
    /// undoing that substitution on the SPECIFICATION side, which made the
    /// check weaker rather than wider: `.replace(';', ",")` over the whole
    /// file means a bullet whose semicolon had become a comma still matched.
    ///
    /// **The fifth pass deleted both substitutions and the reason for them.**
    /// They were justified by this repository's prose checker, which does not
    /// cover Rust source at all, so nothing ever required them. The constants
    /// now carry `≤` and `;` and the comparison is byte for byte after
    /// whitespace collapse.
    ///
    /// **And the search is confined to section 25.1.** It used to run over the
    /// whole of `22-testing-and-tolerance.md`, so a bullet that moved out of
    /// the tolerance section into the prose above or below it would still have
    /// matched, and the constant would have gone on claiming to quote 25.1.
    #[test]
    fn the_transcribed_bullets_match_the_specification() {
        let hld = include_str!("../../../docs/hld/22-testing-and-tolerance.md");
        let spec: String = section_25_1(hld)
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        assert!(
            spec.contains("Tolerance policy"),
            "section 25.1 was not found in the HLD file, so the loop below \
             would be searching an empty string and reporting success"
        );
        for (label, quoted) in [
            (
                "Monochrome 16-bit (CT, MR, CR, DR)",
                super::SECTION_25_1_MONOCHROME,
            ),
            ("Systematic bias, monochrome", super::SECTION_25_1_BIAS),
            ("Colour and ultrasound", super::SECTION_25_1_COLOUR),
            ("Geometry", super::SECTION_25_1_GEOMETRY),
        ] {
            let first_sentence = quoted
                .split_once(". ")
                .map_or(quoted, |(head, _)| head)
                .replace(&format!("{label}: "), "");
            let normalised: String = first_sentence
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");
            assert!(
                spec.contains(&normalised),
                "{label} is transcribed here as {normalised:?} and \
                 docs/hld/22-testing-and-tolerance.md does not contain that. \
                 The constant claims to be a quotation, so either the \
                 specification moved and this did not, or this was never the \
                 quotation it says it is."
            );
        }
    }

    use super::{ToleranceClass, ToleranceError, class_from_categories};

    fn tokens(list: &[&str]) -> Vec<String> {
        list.iter().map(|token| (*token).to_owned()).collect()
    }

    #[test]
    fn mono16_is_class_one() {
        assert_eq!(
            class_from_categories(&tokens(&["synthetic", "mono16", "syntax-reference"])).ok(),
            Some(ToleranceClass::MonochromeSixteenBit)
        );
    }

    #[test]
    fn colour_and_us_are_both_class_two() {
        assert_eq!(
            class_from_categories(&tokens(&["colour", "synthetic"])).ok(),
            Some(ToleranceClass::ColourOrUltrasound)
        );
        assert_eq!(
            class_from_categories(&tokens(&["us", "real", "greyscale-8bit"])).ok(),
            Some(ToleranceClass::ColourOrUltrasound)
        );
    }

    /// The one corpus row carrying both class-two tokens,
    /// `synthetic/us_ybr_full_422.dcm`. Both are class two, so it resolves
    /// rather than conflicting.
    #[test]
    fn a_row_carrying_both_class_two_tokens_is_not_a_conflict() {
        assert_eq!(
            class_from_categories(&tokens(&["synthetic", "us", "colour"])).ok(),
            Some(ToleranceClass::ColourOrUltrasound)
        );
    }

    /// A row declaring both classes is refused. The two classes have different
    /// rules and nothing in 25.1 decides which applies.
    #[test]
    fn a_row_declaring_two_classes_is_refused() {
        assert!(matches!(
            class_from_categories(&tokens(&["mono16", "colour"])),
            Err(ToleranceError::TwoClasses(_))
        ));
    }

    #[test]
    fn a_row_declaring_no_class_is_refused() {
        assert!(matches!(
            class_from_categories(&tokens(&["synthetic", "series"])),
            Err(ToleranceError::NoClass(_))
        ));
    }
}
