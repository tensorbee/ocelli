//! The verdict vocabulary, the per-view record, and the run-level rule.
//!
//! HLD 25.1 gives a predicate. It does not say what a comparator reports, what
//! a run-level result is, or what happens to a view that satisfies the
//! predicate while carrying no information. This module is that construction,
//! and every choice in it is named in F-011's design plan under `## What the
//! specification does not cover`.
//!
//! **`unmeasured` is a third outcome and not a synonym for `pass`.** A run of
//! ninety-nine views that reports "71 pass, 0 fail, 28 unmeasured" is saying
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
use crate::render_hash::{ALGORITHM as RENDER_HASH_ALGORITHM, RenderHash, run_render_hash};
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
    /// At least one side disagrees with the committed synthetic metadata
    /// truth. This rung precedes the pixel and side-to-side parameter rungs.
    MetadataTruth,
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
            Self::MetadataTruth => "metadata-truth",
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
            Self::MetadataTruth,
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

macro_rules! define_rungs {
    ($($variant:ident => $label:literal),+ $(,)?) => {
        /// Which rung of the attribution ladder answered for a view.
        #[derive(Clone, Copy, Debug, Eq, PartialEq)]
        pub enum Rung {
            $($variant),+
        }

        impl Rung {
            /// Every rung the production comparator can emit, in ladder order.
            pub const ALL: &[Self] = &[$(Self::$variant),+];

            #[must_use]
            pub const fn label(self) -> &'static str {
                match self {
                    $(Self::$variant => $label),+
                }
            }
        }
    };
}

define_rungs! {
    MetadataTruth => "metadata-truth",
    Parameters => "parameters",
    Register => "register",
    VolumeDivergence => "volume-divergence",
    Geometry => "geometry",
    ClassTwo => "class-two",
    Letterbox => "letterbox",
    Pixels => "pixels",
    Decimated => "decimated",
    Weak => "weak",
}

/// One qualifier and rung combination that can be green while unmeasured.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GreenUnmeasuredState {
    pub class: ToleranceClass,
    pub qualifiers: &'static [Qualifier],
    pub rung: Rung,
}

impl GreenUnmeasuredState {
    /// Every green unmeasured state production attribution can emit.
    pub const ALL: &[Self] = &[
        Self {
            class: ToleranceClass::MonochromeSixteenBit,
            qualifiers: &[Qualifier::Weak],
            rung: Rung::Weak,
        },
        Self {
            class: ToleranceClass::MonochromeSixteenBit,
            qualifiers: &[Qualifier::Decimated],
            rung: Rung::Decimated,
        },
        Self {
            class: ToleranceClass::MonochromeSixteenBit,
            qualifiers: &[Qualifier::Weak, Qualifier::Decimated],
            rung: Rung::Decimated,
        },
        Self {
            class: ToleranceClass::ColourOrUltrasound,
            qualifiers: &[Qualifier::UnstatedThreshold],
            rung: Rung::ClassTwo,
        },
        Self {
            class: ToleranceClass::ColourOrUltrasound,
            qualifiers: &[Qualifier::Weak, Qualifier::UnstatedThreshold],
            rung: Rung::ClassTwo,
        },
        Self {
            class: ToleranceClass::ColourOrUltrasound,
            qualifiers: &[Qualifier::Decimated, Qualifier::UnstatedThreshold],
            rung: Rung::ClassTwo,
        },
        Self {
            class: ToleranceClass::ColourOrUltrasound,
            qualifiers: &[
                Qualifier::Weak,
                Qualifier::Decimated,
                Qualifier::UnstatedThreshold,
            ],
            rung: Rung::ClassTwo,
        },
    ];

    fn permits(record: &ViewRecord) -> bool {
        Self::ALL.iter().any(|state| {
            state.class == record.class
                && state.rung == record.rung
                && state.qualifiers.len() == record.qualifiers.len()
                && state
                    .qualifiers
                    .iter()
                    .all(|qualifier| record.qualifiers.contains(qualifier))
        })
    }
}

/// Which side a divergence is attributed to.
///
/// Rung 6's default is `Ours`, and that direction is the conservative one:
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
    pub signed_histogram: Vec<(i16, u64)>,
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
            signed_histogram: (-255_i16..=255_i16)
                .filter_map(|difference| {
                    let count = stats.signed_count_at(difference);
                    (count > 0).then_some((difference, count))
                })
                .collect(),
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
            "signedHistogram": self.signed_histogram,
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
    pub rung: Rung,
    pub notes: Vec<String>,
    pub parameter_divergences: Vec<ParameterDivergence>,
    /// Real-row parameter values are compared in memory but never serialized.
    pub parameter_values_withheld: bool,
    pub geometry_divergences: Vec<Divergence>,
    pub register_entry: Option<String>,
    /// Exact, shape-aware identity of the already validated reference frame.
    pub reference_render_hash: String,
    /// Exact, shape-aware identity of the already validated candidate frame.
    pub candidate_render_hash: String,
    pub statistics: Option<ViewStatistics>,
    /// Whether the frame is grey on every pixel, red equal to green equal to
    /// blue. Recorded for EVERY view rather than only for class one, because
    /// a class-two view whose frame is monochrome is the known gap decision 4
    /// of F-011's design round left open: HLD 25.1 has no class for 8-bit
    /// monochrome at all, and `docs/lld/corpus.md` absorbs the one such corpus
    /// row into class two by modality. Publishing the flag is what stops the
    /// next 8-bit greyscale row re-deriving that from scratch.
    pub monochrome_frame: bool,
    /// The reference sidecar's `/attributes/photometricInterpretation`, or
    /// `None` where the sidecar carries no `attributes` block at all, which is
    /// every volume reformat.
    ///
    /// **Not published in `to_json`, because it is an input to target
    /// resolution rather than a fact about the comparison.** It exists so
    /// `mutations::resolve_target` can keep `Effect::VoiLinearExactSwap` off
    /// an inverted frame, where the divergence it models runs the other way.
    /// `monochrome_frame` above is published because it answers a question the
    /// report is asked. This one answers a question the catalogue is asked.
    pub photometric_interpretation: Option<String>,
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
            "rung": self.rung.label(),
            "notes": self.notes,
            "parameterDivergences": Value::Array(
                self.parameter_divergences
                    .iter()
                    .map(|divergence| {
                        let reference = if self.parameter_values_withheld {
                            json!("<withheld, real corpus row>")
                        } else {
                            divergence.reference.clone()
                        };
                        let candidate = if self.parameter_values_withheld {
                            json!("<withheld, real corpus row>")
                        } else {
                            divergence.candidate.clone()
                        };
                        json!({
                            "field": divergence.pointer,
                            "reference": reference,
                            "candidate": candidate,
                            "attributedTo": divergence.side.label(),
                            "why": divergence.why,
                        })
                    })
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
            "renderHashes": {
                "algorithm": RENDER_HASH_ALGORITHM,
                "reference": self.reference_render_hash,
                "candidate": self.candidate_render_hash,
            },
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

/// Machine-readable classification of a whole comparison run.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GateVerdict {
    Pass,
    ComparisonFailure,
    CoverageLoss,
    Refusal,
}

impl GateVerdict {
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::ComparisonFailure => "comparison-failure",
            Self::CoverageLoss => "coverage-loss",
            Self::Refusal => "refusal",
        }
    }
}

/// The whole run's result.
#[derive(Clone, Debug)]
pub struct RunReport {
    pub records: Vec<ViewRecord>,
    pub problems: Vec<String>,
    pub coverage_problems: Vec<String>,
    pub absent_views: usize,
    pub unsupported_source_rows: usize,
    pub declared_volume_refusals: usize,
    pub reference_directory: String,
    pub candidate_directory: String,
}

impl RunReport {
    fn hashes(&self, reference: bool) -> Vec<RenderHash> {
        self.records
            .iter()
            .map(|record| RenderHash {
                kind: record.kind,
                id: record.id.clone(),
                sha256: if reference {
                    record.reference_render_hash.clone()
                } else {
                    record.candidate_render_hash.clone()
                },
            })
            .collect()
    }

    /// Stable identity of all declared reference views, in canonical order.
    #[must_use]
    pub fn reference_render_hash(&self) -> String {
        run_render_hash(&self.hashes(true))
    }

    /// Stable identity of all declared candidate views, in canonical order.
    #[must_use]
    pub fn candidate_render_hash(&self) -> String {
        run_render_hash(&self.hashes(false))
    }

    #[must_use]
    pub fn count(&self, outcome: Outcome) -> usize {
        self.records
            .iter()
            .filter(|record| record.outcome == outcome)
            .count()
    }

    /// Views for which the written pixel predicate produced a verdict.
    /// `unmeasured`, `absent` and source-level refusals are named separately.
    #[must_use]
    pub fn claimed_verdict_views(&self) -> usize {
        self.count(Outcome::Pass) + self.count(Outcome::Fail)
    }

    #[must_use]
    pub fn absent_count(&self) -> usize {
        self.absent_views + self.count(Outcome::Absent)
    }

    #[must_use]
    pub fn gate_verdict(&self) -> GateVerdict {
        if !self.problems.is_empty() {
            GateVerdict::Refusal
        } else if self.count(Outcome::Fail) > 0
            || !self.absorbed_divergences().is_empty()
            || self.records.iter().any(|record| {
                record.outcome == Outcome::Unmeasured && !GreenUnmeasuredState::permits(record)
            })
        {
            GateVerdict::ComparisonFailure
        } else if self.claimed_verdict_views() == 0
            || self.absent_count() > 0
            || !self.coverage_problems.is_empty()
        {
            GateVerdict::CoverageLoss
        } else {
            GateVerdict::Pass
        }
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
    /// Structural and input refusals are `problems`. Census changes are
    /// `coverage_problems`. Outcomes and absorbed divergences are read from
    /// the records here, so one category cannot be reported as another.
    #[must_use]
    pub fn green(&self) -> bool {
        self.gate_verdict() == GateVerdict::Pass
    }

    #[must_use]
    pub fn to_json(&self) -> Value {
        json!({
            "story": "F-011, F-012, F-015",
            "reference": self.reference_directory,
            "candidate": self.candidate_directory,
            "views": self.records.len(),
            "pass": self.count(Outcome::Pass),
            "fail": self.count(Outcome::Fail),
            "unmeasured": self.count(Outcome::Unmeasured),
            "absent": self.absent_count(),
            "claimedVerdictViews": self.claimed_verdict_views(),
            "coverage": {
                "unmeasured": self.count(Outcome::Unmeasured),
                "absent": self.absent_count(),
                "unsupportedSourceRows": self.unsupported_source_rows,
                "declaredVolumeRefusals": self.declared_volume_refusals,
            },
            "gateVerdict": self.gate_verdict().label(),
            "qualifiers": Value::Object(
                self.qualifier_counts()
                    .into_iter()
                    .map(|(label, count)| (label.to_owned(), json!(count)))
                    .collect()
            ),
            "problems": self.problems,
            "coverageProblems": self.coverage_problems,
            "absorbedDivergences": self.absorbed_divergences(),
            "green": self.green(),
            "renderHashes": {
                "algorithm": RENDER_HASH_ALGORITHM,
                "reference": self.reference_render_hash(),
                "candidate": self.candidate_render_hash(),
            },
            "records": Value::Array(self.records.iter().map(ViewRecord::to_json).collect()),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::fmt;

    use serde::de::{DeserializeSeed, MapAccess, SeqAccess, Visitor};
    use serde_json::json;

    use super::{
        Census, ChannelReport, GreenUnmeasuredState, Outcome, ParameterDivergence, Qualifier,
        RunReport, Rung, Side, ViewRecord, ViewStatistics,
    };
    use crate::geometry::Divergence;
    use crate::render_hash::{ALGORITHM as RENDER_HASH_ALGORITHM, RUN_ALGORITHM};
    use crate::sidecar::ViewKind;
    use crate::tolerance::{
        INFORMATIVE_FRACTION_FLOOR, MONOCHROME_MAX_ABS_DIFF, MONOCHROME_SIGNED_MEAN_BIAS,
        MONOCHROME_WITHIN_ONE_LSB_FRACTION, ToleranceClass,
    };

    fn record(id: &str, outcome: Outcome, qualifiers: &[Qualifier]) -> ViewRecord {
        ViewRecord {
            id: id.to_owned(),
            kind: ViewKind::Stack,
            class: ToleranceClass::MonochromeSixteenBit,
            outcome,
            qualifiers: qualifiers.iter().copied().collect::<BTreeSet<_>>(),
            side: Side::None,
            rung: Rung::Pixels,
            notes: Vec::new(),
            parameter_divergences: Vec::new(),
            parameter_values_withheld: false,
            geometry_divergences: Vec::new(),
            register_entry: None,
            reference_render_hash: "reference-hash".to_owned(),
            candidate_render_hash: "candidate-hash".to_owned(),
            statistics: None,
            monochrome_frame: true,
            photometric_interpretation: Some("MONOCHROME2".to_owned()),
        }
    }

    fn zero_channel() -> ChannelReport {
        ChannelReport {
            pixels: 1,
            signed_histogram: vec![(0, 1)],
            max_abs_diff: 0,
            count_at_zero: 1,
            count_at_one: 0,
            count_at_two: 0,
            count_over_two: 0,
            fraction_within_one_lsb: 1.0,
            differing_fraction: 0.0,
            signed_mean_diff: 0.0,
            percentile_999_abs_diff: 0,
        }
    }

    fn object_keys(value: &serde_json::Value) -> BTreeSet<String> {
        value
            .as_object()
            .map(|object| object.keys().cloned().collect())
            .unwrap_or_default()
    }

    fn object_keys_at(value: &serde_json::Value, pointer: &str) -> BTreeSet<String> {
        value.pointer(pointer).map(object_keys).unwrap_or_default()
    }

    fn expected_keys(names: &[&str]) -> BTreeSet<String> {
        names.iter().map(|name| (*name).to_owned()).collect()
    }

    struct UniqueValue;

    impl<'de> DeserializeSeed<'de> for UniqueValue {
        type Value = serde_json::Value;

        fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            deserializer.deserialize_any(UniqueValueVisitor)
        }
    }

    struct UniqueValueVisitor;

    impl<'de> Visitor<'de> for UniqueValueVisitor {
        type Value = serde_json::Value;

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("a JSON value without duplicate object keys")
        }

        fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E> {
            Ok(serde_json::Value::Bool(value))
        }

        fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E> {
            Ok(serde_json::Value::Number(value.into()))
        }

        fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> {
            Ok(serde_json::Value::Number(value.into()))
        }

        fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            serde_json::Number::from_f64(value)
                .map(serde_json::Value::Number)
                .ok_or_else(|| E::custom("a JSON number must be finite"))
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            self.visit_string(value.to_owned())
        }

        fn visit_string<E>(self, value: String) -> Result<Self::Value, E> {
            Ok(serde_json::Value::String(value))
        }

        fn visit_none<E>(self) -> Result<Self::Value, E> {
            Ok(serde_json::Value::Null)
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E> {
            Ok(serde_json::Value::Null)
        }

        fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
        where
            A: SeqAccess<'de>,
        {
            let mut values = Vec::new();
            while let Some(value) = sequence.next_element_seed(UniqueValue)? {
                values.push(value);
            }
            Ok(serde_json::Value::Array(values))
        }

        fn visit_map<A>(self, mut entries: A) -> Result<Self::Value, A::Error>
        where
            A: MapAccess<'de>,
        {
            let mut object = serde_json::Map::new();
            while let Some(key) = entries.next_key::<String>()? {
                if object.contains_key(&key) {
                    return Err(<A::Error as serde::de::Error>::custom(format!(
                        "duplicate JSON key {key:?}"
                    )));
                }
                object.insert(key, entries.next_value_seed(UniqueValue)?);
            }
            Ok(serde_json::Value::Object(object))
        }
    }

    fn parse_unique_json(source: &str) -> Result<serde_json::Value, serde_json::Error> {
        let mut deserializer = serde_json::Deserializer::from_str(source);
        let value = UniqueValue.deserialize(&mut deserializer)?;
        deserializer.end()?;
        Ok(value)
    }

    #[test]
    fn contract_reader_refuses_duplicate_keys_at_any_depth() {
        assert!(parse_unique_json(r#"{"version":1,"version":1}"#).is_err());
        assert!(parse_unique_json(r#"{"schemas":{"report":[],"report":[]}}"#).is_err());
    }

    fn declared_keys(contract: &serde_json::Value, name: &str) -> BTreeSet<String> {
        contract
            .pointer(&format!("/schemas/{name}"))
            .and_then(serde_json::Value::as_array)
            .map(|entries| {
                entries
                    .iter()
                    .filter_map(serde_json::Value::as_str)
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// The tracked contract is the only cross-language schema and fixture.
    /// Rust proves it is an exact serializer product, and the Python ledger
    /// and its refusal probes consume the same file.
    #[test]
    fn tracked_report_contract_matches_the_serializer() -> Result<(), Box<dyn std::error::Error>> {
        let contract = parse_unique_json(include_str!("../report-contract.json"))?;
        assert_eq!(
            object_keys(&contract),
            expected_keys(&[
                "version",
                "schemas",
                "vocabularies",
                "semantics",
                "hashAlgorithms",
                "greenReport",
            ])
        );
        assert_eq!(
            object_keys_at(&contract, "/schemas"),
            expected_keys(&[
                "report",
                "coverage",
                "record",
                "statistics",
                "channel",
                "parameterDivergence",
                "geometryDivergence",
                "renderHashes",
                "greenUnmeasuredState",
            ])
        );
        assert_eq!(
            object_keys_at(&contract, "/vocabularies"),
            expected_keys(&[
                "kinds",
                "toleranceClasses",
                "outcomes",
                "sides",
                "rungs",
                "qualifiers",
            ])
        );
        assert_eq!(
            object_keys_at(&contract, "/semantics"),
            expected_keys(&[
                "channelCountByClass",
                "greenUnmeasuredQualifiers",
                "greenUnmeasuredStates",
                "monochromeWithinOneLsbFraction",
                "monochromeMaxAbsDiff",
                "monochromeSignedMeanBias",
                "informativeFractionFloor",
            ])
        );
        assert_eq!(
            object_keys_at(&contract, "/semantics/channelCountByClass"),
            expected_keys(&["mono16", "colour-or-us"])
        );
        assert_eq!(
            object_keys_at(&contract, "/hashAlgorithms"),
            expected_keys(&["view", "run"])
        );
        for (index, _) in GreenUnmeasuredState::ALL.iter().enumerate() {
            assert_eq!(
                object_keys_at(
                    &contract,
                    &format!("/semantics/greenUnmeasuredStates/{index}")
                ),
                declared_keys(&contract, "greenUnmeasuredState")
            );
        }
        let mut green = record("probe-view", Outcome::Pass, &[]);
        green.reference_render_hash = "0".repeat(64);
        green.candidate_render_hash = "0".repeat(64);
        green.statistics = Some(ViewStatistics {
            channels: 1,
            full: vec![zero_channel()],
            image: vec![zero_channel()],
            background: Vec::new(),
            informative: vec![zero_channel()],
            rows_touched: 0,
            columns_touched: 0,
            image_pixels: 1,
            informative_pixels: 1,
            informative_fraction: 1.0,
            predicate_passes: true,
            bias_passes: true,
            signed_mean_diff: 0.0,
        });
        let mut serialized = report(vec![green], Vec::new()).to_json();
        if let Some(object) = serialized.as_object_mut() {
            object.insert("operation".to_owned(), json!("gate"));
        }
        assert_eq!(contract.pointer("/greenReport"), Some(&serialized));
        assert_eq!(contract.pointer("/version"), Some(&json!(1)));
        assert_eq!(object_keys(&serialized), declared_keys(&contract, "report"));
        assert_eq!(
            object_keys_at(&serialized, "/coverage"),
            declared_keys(&contract, "coverage")
        );
        assert_eq!(
            object_keys_at(&serialized, "/records/0"),
            declared_keys(&contract, "record")
        );
        assert_eq!(
            object_keys_at(&serialized, "/records/0/statistics"),
            declared_keys(&contract, "statistics")
        );
        assert_eq!(
            object_keys_at(&serialized, "/records/0/statistics/full/0"),
            declared_keys(&contract, "channel")
        );
        assert_eq!(
            object_keys_at(&serialized, "/renderHashes"),
            declared_keys(&contract, "renderHashes")
        );

        let mut divergence = record("divergence", Outcome::Fail, &[]);
        divergence.parameter_divergences.push(ParameterDivergence {
            pointer: "/field".to_owned(),
            reference: json!(1),
            candidate: json!(2),
            side: Side::Ours,
            why: "controlled".to_owned(),
        });
        divergence.geometry_divergences.push(Divergence {
            field: "camera.position[0]".to_owned(),
            reference: 0.0,
            candidate: 1.0,
            difference: 1.0,
            bound: 0.25,
        });
        let divergence = divergence.to_json();
        assert_eq!(
            object_keys_at(&divergence, "/parameterDivergences/0"),
            declared_keys(&contract, "parameterDivergence")
        );
        assert_eq!(
            object_keys_at(&divergence, "/geometryDivergences/0"),
            declared_keys(&contract, "geometryDivergence")
        );

        assert_eq!(
            contract.pointer("/vocabularies/kinds"),
            Some(&json!([
                ViewKind::Stack.label(),
                ViewKind::VolumeReformat.label(),
            ]))
        );
        assert_eq!(
            contract.pointer("/vocabularies/toleranceClasses"),
            Some(&json!([
                ToleranceClass::MonochromeSixteenBit.label(),
                ToleranceClass::ColourOrUltrasound.label(),
            ]))
        );
        assert_eq!(
            contract.pointer("/vocabularies/outcomes"),
            Some(&json!([
                Outcome::Pass.label(),
                Outcome::Fail.label(),
                Outcome::Unmeasured.label(),
                Outcome::Absent.label(),
            ]))
        );
        assert_eq!(
            contract.pointer("/vocabularies/sides"),
            Some(&json!([
                Side::Inputs.label(),
                Side::Reference.label(),
                Side::Ours.label(),
                Side::Fit.label(),
                Side::None.label(),
                Side::Unattributed.label(),
            ]))
        );
        assert_eq!(
            contract.pointer("/vocabularies/qualifiers"),
            Some(&json!([
                Qualifier::Weak.label(),
                Qualifier::Decimated.label(),
                Qualifier::UnstatedThreshold.label(),
                Qualifier::ParameterDivergence.label(),
                Qualifier::MetadataTruth.label(),
                Qualifier::GeometryDivergence.label(),
                Qualifier::ReferenceDivergence.label(),
                Qualifier::Bias.label(),
                Qualifier::LetterboxOnly.label(),
                Qualifier::DivergentWhileUnmeasured.label(),
            ]))
        );
        assert_eq!(
            contract.pointer("/vocabularies/rungs"),
            Some(&json!(
                Rung::ALL
                    .iter()
                    .copied()
                    .map(Rung::label)
                    .collect::<Vec<_>>()
            ))
        );
        assert_eq!(
            contract.pointer("/semantics/channelCountByClass"),
            Some(&json!({
                ToleranceClass::MonochromeSixteenBit.label(): 1,
                ToleranceClass::ColourOrUltrasound.label(): 3,
            }))
        );
        assert_eq!(
            contract.pointer("/semantics/greenUnmeasuredQualifiers"),
            Some(&json!([
                Qualifier::Weak.label(),
                Qualifier::Decimated.label(),
                Qualifier::UnstatedThreshold.label(),
            ]))
        );
        let green_unmeasured_states = GreenUnmeasuredState::ALL
            .iter()
            .map(|state| {
                json!({
                    "toleranceClass": state.class.label(),
                    "qualifiers": state
                        .qualifiers
                        .iter()
                        .copied()
                        .map(Qualifier::label)
                        .collect::<Vec<_>>(),
                    "rung": state.rung.label(),
                })
            })
            .collect::<Vec<_>>();
        assert_eq!(
            contract.pointer("/semantics/greenUnmeasuredStates"),
            Some(&json!(green_unmeasured_states))
        );
        assert_eq!(
            contract.pointer("/hashAlgorithms"),
            Some(&json!({
                "view": RENDER_HASH_ALGORITHM,
                "run": RUN_ALGORITHM,
            }))
        );
        assert_eq!(
            contract.pointer("/semantics/monochromeWithinOneLsbFraction"),
            Some(&json!(MONOCHROME_WITHIN_ONE_LSB_FRACTION))
        );
        assert_eq!(
            contract.pointer("/semantics/monochromeMaxAbsDiff"),
            Some(&json!(MONOCHROME_MAX_ABS_DIFF))
        );
        assert_eq!(
            contract.pointer("/semantics/monochromeSignedMeanBias"),
            Some(&json!(MONOCHROME_SIGNED_MEAN_BIAS))
        );
        assert_eq!(
            contract.pointer("/semantics/informativeFractionFloor"),
            Some(&json!(INFORMATIVE_FRACTION_FLOOR))
        );
        Ok(())
    }

    #[test]
    fn real_parameter_values_do_not_survive_report_serialization() {
        let secrets = ["PATIENT-VALUE-A", "PATIENT-VALUE-B"];
        let mut real = record(
            "opaque-id",
            Outcome::Fail,
            &[Qualifier::ParameterDivergence],
        );
        real.parameter_values_withheld = true;
        real.parameter_divergences.push(super::ParameterDivergence {
            pointer: "/attributes/private".to_owned(),
            reference: json!({ "nested": [secrets[0], 12.25] }),
            candidate: json!([secrets[1], -0.0]),
            side: Side::Unattributed,
            why: "independent readers disagree".to_owned(),
        });
        let serialized = serde_json::to_string(&real.to_json()).unwrap_or_default();
        for secret in secrets {
            assert!(!serialized.contains(secret));
        }
        assert!(!serialized.contains("12.25"));
        assert!(serialized.contains("<withheld, real corpus row>"));
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
            coverage_problems: Vec::new(),
            absent_views: 0,
            unsupported_source_rows: 0,
            declared_volume_refusals: 0,
            reference_directory: "reference".to_owned(),
            candidate_directory: "candidate".to_owned(),
        }
    }

    /// A run with nothing wrong is green, which is what makes the two tests
    /// below about the qualifier rather than about the constructor.
    #[test]
    fn a_clean_run_is_green() {
        let mut unmeasured = record("a", Outcome::Unmeasured, &[Qualifier::Weak]);
        unmeasured.rung = Rung::Weak;
        let clean = report(
            vec![record("judged", Outcome::Pass, &[]), unmeasured],
            Vec::new(),
        );
        assert!(clean.green());
        assert!(clean.absorbed_divergences().is_empty());
        let json = clean.to_json();
        assert_eq!(
            json.pointer("/renderHashes/algorithm"),
            Some(&json!("sha256-rgba8-v1"))
        );
        assert_eq!(
            json.pointer("/records/0/renderHashes/reference"),
            Some(&json!("reference-hash"))
        );
    }

    #[test]
    fn every_declared_green_unmeasured_state_reaches_the_gate_verdict() {
        for state in GreenUnmeasuredState::ALL {
            let mut unmeasured = record("unmeasured", Outcome::Unmeasured, state.qualifiers);
            unmeasured.class = state.class;
            unmeasured.rung = state.rung;
            let run = report(
                vec![record("judged", Outcome::Pass, &[]), unmeasured],
                Vec::new(),
            );
            assert!(run.green(), "declared state {state:?} was not green");
        }

        let mut undeclared = record("unmeasured", Outcome::Unmeasured, &[Qualifier::Weak]);
        undeclared.rung = Rung::Decimated;
        let run = report(
            vec![record("judged", Outcome::Pass, &[]), undeclared],
            Vec::new(),
        );
        assert!(!run.green(), "an undeclared state reached a green verdict");
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
