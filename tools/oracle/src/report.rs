//! The verdict vocabulary, the per-view record, and the run-level rule.
//!
//! HLD 25.1 gives a predicate. It does not say what a comparator reports, what
//! a run-level result is, or what happens to a view that satisfies the
//! predicate while carrying no information. This module is that construction,
//! and every choice in it is named in F-011's design plan under `## What the
//! specification does not cover`.
//!
//! **`unmeasured` is a third outcome and not a synonym for `pass`.** A run of
//! ninety-eight views that reports "70 pass, 0 fail, 28 unmeasured" is saying
//! something a run that reported "98 compared" would not. The number is
//! uncomfortable on purpose: better than a quarter of the corpus is covered
//! and not measured, and that was true before this story and invisible.
//!
//! **`unmeasured` is not a place things go to be forgotten**, and there are
//! two mechanisms rather than one. The census pins which views are unmeasured
//! and with which qualifiers, in both directions, so a view entering or
//! leaving it fails the run. And a view whose pixel predicate actually FAILED
//! while its outcome is `unmeasured` carries `divergent-while-unmeasured` and
//! fails the run on its own, because absorbing a measured divergence into an
//! "I cannot tell" is the exact defect this sprint names.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use serde_json::{Value, json};

use crate::frame::StatsError;
use crate::geometry::Divergence;
use crate::sidecar::ViewKind;
use crate::tolerance::ToleranceClass;

/// What a view's comparison concluded.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Outcome {
    /// The written tolerance was evaluated and held.
    Pass,
    /// The written tolerance was evaluated and did not hold.
    Fail,
    /// The instrument cannot answer. Never a synonym for `pass`.
    Unmeasured,
    /// The view exists on one side and not the other. A run failure, never a
    /// skip.
    Absent,
}

impl Outcome {
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Fail => "fail",
            Self::Unmeasured => "unmeasured",
            Self::Absent => "absent",
        }
    }
}

impl fmt::Display for Outcome {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.label())
    }
}

/// Why a view is not a plain pass. A view may carry several: the ultrasound
/// row is class two, decimated and 8-bit greyscale all at once.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Qualifier {
    /// The informative fraction is below the declared floor, so the frame has
    /// too little evidence in it for a verdict to mean anything.
    Weak,
    /// The frame was fitted DOWN into the canvas, so a sub-source-pixel
    /// difference in the fit selects a different source pixel under `NEAREST`
    /// and the pixel difference is unbounded for a reason that has nothing to
    /// do with the LUT chain.
    Decimated,
    /// Class two. 25.1 requires "perceptual difference below a stated
    /// threshold" and states none, so a pass would be a claim against a bound
    /// nobody wrote. Deviation D-16.
    UnstatedThreshold,
    /// The two sides declare different values for something that decides the
    /// compared pixels.
    ParameterDivergence,
    /// The two sides' cameras or image extents are outside 25.1's geometry
    /// bound.
    GeometryDivergence,
    /// A committed register entry, or F-X007's own per-subject measurement,
    /// explains the divergence and attributes it to the reference.
    ReferenceDivergence,
    /// The signed mean over the INFORMATIVE region is outside 25.1's bias
    /// bound. Not over the image rectangle, which is what the bullet said
    /// before the S03 sprint review's second pass and what this line said
    /// until its fourth.
    Bias,
    /// The difference is confined to the letterbox, so it is a difference in
    /// the fit rather than in the picture.
    LetterboxOnly,
    /// The outcome is `unmeasured` and the pixel predicate nonetheless failed.
    /// A run failure on its own.
    DivergentWhileUnmeasured,
}

impl Qualifier {
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Weak => "weak",
            Self::Decimated => "decimated",
            Self::UnstatedThreshold => "unstated-threshold",
            Self::ParameterDivergence => "parameter-divergence",
            Self::GeometryDivergence => "geometry-divergence",
            Self::ReferenceDivergence => "reference-divergence",
            Self::Bias => "bias",
            Self::LetterboxOnly => "letterbox-only",
            Self::DivergentWhileUnmeasured => "divergent-while-unmeasured",
        }
    }

    /// # Errors
    /// On a label this vocabulary does not carry. A census naming a qualifier
    /// nobody emits is a stale census.
    pub fn parse(raw: &str) -> Result<Self, String> {
        for candidate in [
            Self::Weak,
            Self::Decimated,
            Self::UnstatedThreshold,
            Self::ParameterDivergence,
            Self::GeometryDivergence,
            Self::ReferenceDivergence,
            Self::Bias,
            Self::LetterboxOnly,
            Self::DivergentWhileUnmeasured,
        ] {
            if candidate.label() == raw {
                return Ok(candidate);
            }
        }
        Err(format!(
            "{raw:?} is not a qualifier this comparator emits. The vocabulary \
             is in tools/oracle/src/report.rs"
        ))
    }
}

/// Which side a divergence is attributed to.
///
/// Rung 5's default is `Ours`, and that direction is the conservative one:
/// HLD section 11 makes cornerstone3D the reference and deviation D-11 makes
/// the pin the definition of correct, so the burden is on us to show the
/// reference is wrong rather than on the reference to show it is right.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Side {
    /// The two runs were produced from different inputs. Neither renderer.
    Inputs,
    /// The reference. Named only by a committed register entry or by F-X007's
    /// own measurement against a known truth.
    Reference,
    /// The candidate. The default for an unexplained pixel divergence.
    Ours,
    /// The fit rather than the LUT chain: the letterbox, or whole rows and
    /// columns.
    Fit,
    /// Nothing to attribute.
    None,
    /// Something diverged and nothing here decides which side owns it.
    Unattributed,
}

impl Side {
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Inputs => "inputs",
            Self::Reference => "reference",
            Self::Ours => "ours",
            Self::Fit => "fit",
            Self::None => "none",
            Self::Unattributed => "unattributed",
        }
    }
}

/// One declared field on which the two sides disagree.
#[derive(Clone, Debug)]
pub struct ParameterDivergence {
    pub pointer: String,
    pub reference: Value,
    pub candidate: Value,
    /// Which side disagrees with the independent reading of the bytes, when
    /// there is one to disagree with.
    pub side: Side,
    pub why: String,
}

/// Everything measured over one view's pixels.
#[derive(Clone, Debug)]
pub struct ViewStatistics {
    pub channels: usize,
    pub full: Vec<ChannelReport>,
    pub image: Vec<ChannelReport>,
    pub background: Vec<ChannelReport>,
    pub informative: Vec<ChannelReport>,
    pub rows_touched: u32,
    pub columns_touched: u32,
    pub image_pixels: u64,
    pub informative_pixels: u64,
    pub informative_fraction: f64,
    pub predicate_passes: bool,
    pub bias_passes: bool,
    pub signed_mean_diff: f64,
}

/// One lane's numbers, in the shape a reader of the report needs them.
#[derive(Clone, Debug)]
pub struct ChannelReport {
    pub pixels: u64,
    pub max_abs_diff: u8,
    pub count_at_zero: u64,
    pub count_at_one: u64,
    pub count_at_two: u64,
    pub count_over_two: u64,
    pub fraction_within_one_lsb: f64,
    pub differing_fraction: f64,
    pub signed_mean_diff: f64,
    pub percentile_999_abs_diff: u8,
}

impl ChannelReport {
    /// # Errors
    /// When the region is empty, which is an error rather than a zero because
    /// reporting zero over no pixels would invent evidence.
    pub fn of(stats: &crate::frame::ChannelStats) -> Result<Self, StatsError> {
        Ok(Self {
            pixels: stats.pixels(),
            max_abs_diff: stats.max_abs_diff(),
            count_at_zero: stats.count_at(0),
            count_at_one: stats.count_at(1),
            count_at_two: stats.count_at(2),
            count_over_two: stats.count_over(2),
            fraction_within_one_lsb: stats.fraction_within(1)?,
            differing_fraction: stats.differing_fraction()?,
            signed_mean_diff: stats.signed_mean_diff()?,
            percentile_999_abs_diff: stats
                .percentile_abs_diff(crate::tolerance::CLASS_TWO_REPORTED_PERCENTILE)?,
        })
    }

    fn to_json(&self) -> Value {
        json!({
            "pixels": self.pixels,
            "maxAbsDiff": self.max_abs_diff,
            "countAtZero": self.count_at_zero,
            "countAtOne": self.count_at_one,
            "countAtTwo": self.count_at_two,
            "countOverTwo": self.count_over_two,
            "fractionWithinOneLsb": self.fraction_within_one_lsb,
            "differingFraction": self.differing_fraction,
            "signedMeanDiff": self.signed_mean_diff,
            "percentile999AbsDiff": self.percentile_999_abs_diff,
        })
    }
}

fn channels_to_json(list: &[ChannelReport]) -> Value {
    Value::Array(list.iter().map(ChannelReport::to_json).collect())
}

/// One view's whole record.
#[derive(Clone, Debug)]
pub struct ViewRecord {
    pub id: String,
    pub kind: ViewKind,
    pub class: ToleranceClass,
    pub outcome: Outcome,
    pub qualifiers: BTreeSet<Qualifier>,
    pub side: Side,
    /// Which rung of the attribution ladder answered.
    pub rung: &'static str,
    pub notes: Vec<String>,
    pub parameter_divergences: Vec<ParameterDivergence>,
    pub geometry_divergences: Vec<Divergence>,
    pub register_entry: Option<String>,
    pub statistics: Option<ViewStatistics>,
    /// Whether the frame is grey on every pixel, red equal to green equal to
    /// blue. Recorded for EVERY view rather than only for class one, because
    /// a class-two view whose frame is monochrome is the known gap decision 4
    /// of F-011's design round left open: HLD 25.1 has no class for 8-bit
    /// monochrome at all, and `docs/lld/corpus.md` absorbs the one such corpus
    /// row into class two by modality. Publishing the flag is what stops the
    /// next 8-bit greyscale row re-deriving that from scratch.
    pub monochrome_frame: bool,
}

impl ViewRecord {
    #[must_use]
    pub fn qualifier_labels(&self) -> Vec<String> {
        self.qualifiers
            .iter()
            .map(|qualifier| qualifier.label().to_owned())
            .collect()
    }

    #[must_use]
    pub fn to_json(&self) -> Value {
        let statistics = self.statistics.as_ref().map_or(Value::Null, |stats| {
            json!({
                "channels": stats.channels,
                "full": channels_to_json(&stats.full),
                "image": channels_to_json(&stats.image),
                "background": channels_to_json(&stats.background),
                "informative": channels_to_json(&stats.informative),
                "rowsTouched": stats.rows_touched,
                "columnsTouched": stats.columns_touched,
                "imagePixels": stats.image_pixels,
                "informativePixels": stats.informative_pixels,
                "informativeFraction": stats.informative_fraction,
                "predicatePasses": stats.predicate_passes,
                "biasPasses": stats.bias_passes,
                "signedMeanDiff": stats.signed_mean_diff,
            })
        });
        json!({
            "id": self.id,
            "kind": self.kind.label(),
            "toleranceClass": self.class.label(),
            "outcome": self.outcome.label(),
            "qualifiers": self.qualifier_labels(),
            "attributedTo": self.side.label(),
            "rung": self.rung,
            "notes": self.notes,
            "parameterDivergences": Value::Array(
                self.parameter_divergences
                    .iter()
                    .map(|divergence| json!({
                        "field": divergence.pointer,
                        "reference": divergence.reference,
                        "candidate": divergence.candidate,
                        "attributedTo": divergence.side.label(),
                        "why": divergence.why,
                    }))
                    .collect()
            ),
            "geometryDivergences": Value::Array(
                self.geometry_divergences
                    .iter()
                    .map(|divergence| json!({
                        "field": divergence.field,
                        "reference": divergence.reference,
                        "candidate": divergence.candidate,
                        "difference": divergence.difference,
                        "bound": divergence.bound,
                    }))
                    .collect()
            ),
            "referenceDivergenceEntry": self.register_entry,
            "monochromeFrame": self.monochrome_frame,
            "statistics": statistics,
        })
    }
}

/// The committed census of which views are `unmeasured` and why.
///
/// Strict in both directions, which is `unsupported.json`'s discipline applied
/// to the comparator: a view that LEAVES the census is a coverage gain that
/// has to be recorded, and a view that JOINS it is a coverage loss that has to
/// be explained.
#[derive(Clone, Debug)]
pub struct Census {
    pub expected: BTreeMap<String, BTreeSet<Qualifier>>,
}

impl Census {
    /// # Errors
    /// When the file is not the declared shape, or names a qualifier this
    /// comparator does not emit.
    pub fn from_json(root: &Value) -> Result<Self, String> {
        let map = root
            .pointer("/unmeasured")
            .and_then(Value::as_object)
            .ok_or_else(|| "compare-expectations.json has no `unmeasured` object".to_owned())?;
        let mut expected = BTreeMap::new();
        for (id, qualifiers) in map {
            let list = qualifiers
                .as_array()
                .ok_or_else(|| format!("{id}: the qualifier list is not an array"))?;
            let mut set = BTreeSet::new();
            for entry in list {
                let label = entry
                    .as_str()
                    .ok_or_else(|| format!("{id}: a qualifier is not a string"))?;
                set.insert(Qualifier::parse(label)?);
            }
            expected.insert(id.clone(), set);
        }
        Ok(Self { expected })
    }

    /// Every way the measured census departs from the committed one.
    #[must_use]
    pub fn differences(&self, records: &[ViewRecord]) -> Vec<String> {
        let mut problems = Vec::new();
        let mut measured: BTreeMap<String, BTreeSet<Qualifier>> = BTreeMap::new();
        for record in records {
            if record.outcome == Outcome::Unmeasured {
                measured.insert(record.id.clone(), record.qualifiers.clone());
            }
        }
        for (id, qualifiers) in &measured {
            match self.expected.get(id) {
                None => problems.push(format!(
                    "{id} is unmeasured and compare-expectations.json does not \
                     list it. A view joining the census is a coverage LOSS and \
                     has to be explained, never absorbed. Qualifiers: {}",
                    labels(qualifiers)
                )),
                Some(want) if want != qualifiers => problems.push(format!(
                    "{id} is unmeasured for {} and the census expects {}",
                    labels(qualifiers),
                    labels(want)
                )),
                Some(_) => {}
            }
        }
        for id in self.expected.keys() {
            if !measured.contains_key(id) {
                problems.push(format!(
                    "{id} is in compare-expectations.json and is not unmeasured \
                     in this run. A view leaving the census is a coverage GAIN \
                     and has to be RECORDED, so update the file in the same \
                     change"
                ));
            }
        }
        problems
    }
}

fn labels(set: &BTreeSet<Qualifier>) -> String {
    set.iter()
        .map(|qualifier| qualifier.label().to_owned())
        .collect::<Vec<_>>()
        .join(", ")
}

/// The whole run's result.
#[derive(Clone, Debug)]
pub struct RunReport {
    pub records: Vec<ViewRecord>,
    pub problems: Vec<String>,
    pub reference_directory: String,
    pub candidate_directory: String,
}

impl RunReport {
    #[must_use]
    pub fn count(&self, outcome: Outcome) -> usize {
        self.records
            .iter()
            .filter(|record| record.outcome == outcome)
            .count()
    }

    /// The qualifier histogram over EVERY record, which is the number the
    /// run-level summary line carries.
    ///
    /// Not over the unmeasured views alone. A qualifier is not the property of
    /// an outcome: `parameter-divergence`, `geometry-divergence` and `bias` all
    /// arrive on views that FAILED, and a histogram that dropped those would
    /// under-report the reasons a run went red. The counts therefore total more
    /// than the unmeasured count, and `real/us_cmb_crc/00000001.dcm` alone
    /// contributes to two of them.
    #[must_use]
    pub fn qualifier_counts(&self) -> BTreeMap<&'static str, usize> {
        let mut counts: BTreeMap<&'static str, usize> = BTreeMap::new();
        for record in &self.records {
            for qualifier in &record.qualifiers {
                *counts.entry(qualifier.label()).or_insert(0) += 1;
            }
        }
        counts
    }

    /// Every view whose gate failed while its outcome stayed `unmeasured`,
    /// phrased as run problems.
    ///
    /// **Read off the records here rather than collected by the caller**, and
    /// the difference is the whole point. Until the sprint review's fourth pass
    /// four tracked places said this qualifier failed a run on its own and
    /// nothing pushed a problem for it, so it failed only through the census in
    /// `compare-expectations.json`, and `Qualifier::parse` accepts the label.
    /// Adding `"divergent-while-unmeasured"` to a census entry, which reads
    /// like documenting a known-unmeasured view, therefore turned a measured
    /// divergence back into a green run. A rule that any other file can switch
    /// off is not the rule the record claims it is.
    #[must_use]
    pub fn absorbed_divergences(&self) -> Vec<String> {
        self.records
            .iter()
            .filter(|record| {
                record
                    .qualifiers
                    .contains(&Qualifier::DivergentWhileUnmeasured)
            })
            .map(|record| {
                format!(
                    "{}: the pixel predicate failed and the outcome is {}. A \
                     measured divergence absorbed into \"cannot tell\" fails \
                     the run on its own, and no census entry excuses it",
                    record.id, record.outcome
                )
            })
            .collect()
    }

    /// A comparison run is green when every view has outcome `pass` or
    /// `unmeasured`, there are zero `absent` views, no view carries
    /// `divergent-while-unmeasured`, the census matches exactly in both
    /// directions, and no register entry marked unreachable fired.
    ///
    /// Three of those five are `problems`, which the caller collects. The
    /// other two are counted and read here, so the whole rule holds whatever a
    /// caller does or forgets.
    #[must_use]
    pub fn green(&self) -> bool {
        self.problems.is_empty()
            && self.absorbed_divergences().is_empty()
            && self.count(Outcome::Fail) == 0
            && self.count(Outcome::Absent) == 0
    }

    #[must_use]
    pub fn to_json(&self) -> Value {
        json!({
            "story": "F-011",
            "reference": self.reference_directory,
            "candidate": self.candidate_directory,
            "views": self.records.len(),
            "pass": self.count(Outcome::Pass),
            "fail": self.count(Outcome::Fail),
            "unmeasured": self.count(Outcome::Unmeasured),
            "absent": self.count(Outcome::Absent),
            "qualifiers": Value::Object(
                self.qualifier_counts()
                    .into_iter()
                    .map(|(label, count)| (label.to_owned(), json!(count)))
                    .collect()
            ),
            "problems": self.problems,
            "absorbedDivergences": self.absorbed_divergences(),
            "green": self.green(),
            "records": Value::Array(self.records.iter().map(ViewRecord::to_json).collect()),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use serde_json::json;

    use super::{Census, Outcome, Qualifier, RunReport, Side, ViewRecord};
    use crate::sidecar::ViewKind;
    use crate::tolerance::ToleranceClass;

    fn record(id: &str, outcome: Outcome, qualifiers: &[Qualifier]) -> ViewRecord {
        ViewRecord {
            id: id.to_owned(),
            kind: ViewKind::Stack,
            class: ToleranceClass::MonochromeSixteenBit,
            outcome,
            qualifiers: qualifiers.iter().copied().collect::<BTreeSet<_>>(),
            side: Side::None,
            rung: "pixels",
            notes: Vec::new(),
            parameter_divergences: Vec::new(),
            geometry_divergences: Vec::new(),
            register_entry: None,
            statistics: None,
            monochrome_frame: true,
        }
    }

    fn census(entries: &[(&str, &[&str])]) -> Census {
        let map: serde_json::Map<String, serde_json::Value> = entries
            .iter()
            .map(|(id, qualifiers)| ((*id).to_owned(), json!(qualifiers.to_vec())))
            .collect();
        let Ok(built) = Census::from_json(&json!({ "unmeasured": map })) else {
            return Census {
                expected: std::collections::BTreeMap::new(),
            };
        };
        built
    }

    /// A view JOINING the census fails the run. That is a coverage loss.
    #[test]
    fn a_view_joining_the_census_fails_the_run() {
        let expected = census(&[]);
        let problems =
            expected.differences(&[record("a", Outcome::Unmeasured, &[Qualifier::Weak])]);
        assert_eq!(problems.len(), 1);
        let Some(first) = problems.first() else {
            return;
        };
        assert!(first.contains("coverage LOSS"));
    }

    /// A view LEAVING the census fails the run too. That is a coverage gain
    /// and it has to be recorded rather than silently enjoyed.
    #[test]
    fn a_view_leaving_the_census_fails_the_run() {
        let expected = census(&[("a", &["weak"])]);
        let problems = expected.differences(&[record("a", Outcome::Pass, &[])]);
        assert_eq!(problems.len(), 1);
        let Some(first) = problems.first() else {
            return;
        };
        assert!(first.contains("coverage GAIN"));
    }

    /// A view whose qualifiers change fails the run even though it is still
    /// unmeasured, because the REASON is what the census records.
    #[test]
    fn a_changed_qualifier_set_fails_the_run() {
        let expected = census(&[("a", &["weak"])]);
        let problems =
            expected.differences(&[record("a", Outcome::Unmeasured, &[Qualifier::Decimated])]);
        assert_eq!(problems.len(), 1);
    }

    #[test]
    fn a_matching_census_reports_nothing() {
        let expected = census(&[("a", &["weak", "decimated"])]);
        let problems = expected.differences(&[record(
            "a",
            Outcome::Unmeasured,
            &[Qualifier::Weak, Qualifier::Decimated],
        )]);
        assert!(problems.is_empty());
    }

    /// A census naming a qualifier this comparator does not emit is a stale
    /// census and is refused rather than read as an empty set.
    #[test]
    fn an_unknown_qualifier_in_the_census_is_refused() {
        let outcome = Census::from_json(&json!({ "unmeasured": { "a": ["invented"] } }));
        assert!(outcome.is_err());
    }

    fn report(records: Vec<ViewRecord>, problems: Vec<String>) -> RunReport {
        RunReport {
            records,
            problems,
            reference_directory: "reference".to_owned(),
            candidate_directory: "candidate".to_owned(),
        }
    }

    /// A run with nothing wrong is green, which is what makes the two tests
    /// below about the qualifier rather than about the constructor.
    #[test]
    fn a_clean_run_is_green() {
        let clean = report(
            vec![record("a", Outcome::Unmeasured, &[Qualifier::Weak])],
            Vec::new(),
        );
        assert!(clean.green());
        assert!(clean.absorbed_divergences().is_empty());
    }

    /// `divergent-while-unmeasured` fails the run BY ITSELF. Zero fails, zero
    /// absents and an empty problem list, and the run is still red.
    ///
    /// Until the sprint review's fourth pass this was false. `green()` read
    /// `problems`, the qualifier put nothing there, and four tracked places
    /// said it failed a run on its own.
    #[test]
    fn an_absorbed_divergence_fails_the_run_on_its_own() {
        let absorbed = report(
            vec![record(
                "a",
                Outcome::Unmeasured,
                &[Qualifier::Weak, Qualifier::DivergentWhileUnmeasured],
            )],
            Vec::new(),
        );
        assert_eq!(absorbed.count(Outcome::Fail), 0);
        assert_eq!(absorbed.count(Outcome::Absent), 0);
        assert!(absorbed.problems.is_empty());
        assert_eq!(absorbed.absorbed_divergences().len(), 1);
        assert!(!absorbed.green(), "a measured divergence was absorbed");
    }

    /// **The bypass, refused.** The census is the only thing that used to fail
    /// a run for this qualifier, and `Qualifier::parse` accepts the label, so
    /// a census entry naming it silenced the census check as well. Here the
    /// census agrees with the record exactly and reports nothing, which is the
    /// state that used to be green.
    #[test]
    fn a_census_entry_naming_the_qualifier_does_not_buy_a_green_run() {
        let qualifiers = &[Qualifier::Weak, Qualifier::DivergentWhileUnmeasured];
        let records = vec![record("a", Outcome::Unmeasured, qualifiers)];
        let expected = census(&[("a", &["weak", "divergent-while-unmeasured"])]);
        let problems = expected.differences(&records);
        assert!(
            problems.is_empty(),
            "the census is satisfied, which is the bypass: {problems:?}"
        );
        assert!(!report(records, problems).green());
    }
}
