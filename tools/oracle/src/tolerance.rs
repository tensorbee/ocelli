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

/// 25.1's first bullet, verbatim except for the two normalisations this
/// repository's prose checker requires outside `docs/hld/`: the source's
/// prose semicolon is written as a comma and its `≤` as `<=`. No word is
/// changed.
pub const SECTION_25_1_MONOCHROME: &str = "Monochrome 16-bit (CT, MR, CR, DR): \
maximum absolute difference <= 1 LSB on at least 99.9% of pixels, zero pixels \
differing by more than 2.";

/// 25.1's bias bullet, added in S03 by operator decision through F-011's
/// design plan, transcribed under the same two normalisations.
pub const SECTION_25_1_BIAS: &str = "Systematic bias, monochrome: signed mean \
difference over the image rectangle within 0.1 of one display code, evaluated \
only where input identity, declared parameters and geometry already agree.";

/// 25.1's colour bullet.
pub const SECTION_25_1_COLOUR: &str = "Colour and ultrasound: perceptual \
difference below a stated threshold, because chroma subsampling and YBR \
conversion legitimately differ.";

/// 25.1's geometry bullet.
pub const SECTION_25_1_GEOMETRY: &str = "Geometry: world coordinates within \
1e-6 mm, canvas coordinates within a quarter pixel.";

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
/// functions everywhere. 0.1 sits roughly three times below the 0.32 mean that
/// swap produces at the window centre and far above the zero expected when two
/// implementations agree.
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
/// choice was the difference between detecting 0 of 71 gating class-one
/// views and detecting the soft-tissue CT rows where HLD 18.3's worked
/// example lives.
///
/// **A structural limit worth knowing before trusting this.** The
/// per-pixel divergence between LINEAR and LINEAR_EXACT is exactly
/// `u / w`, where `u` is the LINEAR display value in 0 to 255 and `w`
/// is the window width. So the largest divergence any view can show is
/// `255 / w`, and for `w > 2550` this bound is UNREACHABLE no matter
/// which region it is taken over. The corpus already carries rows at
/// `w = 4096`. Those views are not protected by this bound and nothing
/// pretends otherwise.
#[derive(Clone, Copy, Debug)]
pub struct BiasVerdict {
    pub signed_mean_diff: f64,
    pub passes: bool,
}

/// # Errors
/// When the region is empty.
pub fn bias_bound(image_stats: &ChannelStats) -> Result<BiasVerdict, ToleranceError> {
    let signed_mean_diff = image_stats.signed_mean_diff()?;
    Ok(BiasVerdict {
        signed_mean_diff,
        passes: signed_mean_diff.abs() <= MONOCHROME_SIGNED_MEAN_BIAS,
    })
}

#[cfg(test)]
mod tests {
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
