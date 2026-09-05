//! The mutation catalogue: declared damage with the verdict it must produce.
//!
//! **A guard nobody has watched fail is not a guard.** F-010 learned that the
//! hard way, and `docs/sprints/CURRENT_SPRINT.md` states the rule this file
//! exists to satisfy: every new guard is observed red before it is claimed,
//! and the mutation that proves it must not be run in the same command that
//! adds it. So this catalogue is production data in `src/`, exactly as
//! `tools/oracle/src/faults.mjs` is, the runner is separate, and
//! `bin/ocelli.sh compare` re-runs every entry on every oracle gate rather
//! than trusting a note saying somebody once watched them fail.
//!
//! An identity comparison of the reference against itself proves the loader,
//! the identifier mapping, the class resolution, the sidecar contract and the
//! report shape over all ninety-eight views. **It proves nothing about
//! detection**, which is why it is never allowed to be the only corpus-scale
//! exercise. This file is the other half.
//!
//! Every mutation is applied IN MEMORY. Nothing under `tools/oracle/out/` is
//! written to, because a difference image of a real corpus row is a rendered
//! picture of patient data.

use serde_json::Value;
use thiserror::Error;

use crate::frame::{Frame, FrameError, Rect};
use crate::report::{Outcome, Qualifier, Side, ViewRecord};
use crate::sidecar::{Run, ViewKind};
use crate::tolerance::ToleranceClass;

#[derive(Debug, Error)]
pub enum MutationError {
    #[error("{0}: no view in the identity run matches this mutation's target")]
    NoTarget(&'static str),
    #[error("{0}: {1}")]
    Apply(&'static str, String),
    #[error("{0}")]
    Frame(#[from] FrameError),
}

/// Which side of the comparison the damage is applied to.
///
/// Two entries damage the REFERENCE on purpose. They are what show rung 2
/// naming a side rather than only saying that two readings disagree.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MutatedSide {
    Reference,
    Candidate,
}

/// Which view a mutation lands on. Resolved against the identity run's own
/// records, so a mutation always lands on a view whose undamaged outcome is
/// known rather than on whichever identifier sorts first.
#[derive(Clone, Copy, Debug)]
pub enum Target {
    /// The first class-one stack view that passes cleanly, so the mutation's
    /// effect on the outcome is unambiguous.
    MeasuredStack,
    /// The first volume reformat that passes cleanly. Present so the
    /// catalogue reaches the nine views `rows[]` does not name.
    MeasuredReformat,
    /// The first class-two view. Its outcome is `unmeasured` before and after,
    /// so what a mutation must show there is DETECTION and not a verdict.
    ///
    /// On today's corpus this resolves to `real/us_cmb_crc/00000001.dcm`,
    /// which is the 8-bit MONOCHROME2 ultrasound HLD 25.1 has no class for at
    /// all, so a mutation aimed here also exercises the one row that carries
    /// two qualifiers and no evaluable bound.
    ClassTwo,
    /// The first class-two view whose frame is actually colour.
    ///
    /// The two are separate because measuring caught a real thing: the first
    /// class-two view in identifier order is greyscale, so a red-and-blue swap
    /// on it changes NOTHING and a catalogue that aimed a channel swap at
    /// `ClassTwo` would report a guard as watched while measuring a frame the
    /// mutation could not damage.
    ColourClassTwo,
}

/// How many pixels a delta touches.
#[derive(Clone, Copy, Debug)]
pub enum PixelCount {
    One,
    /// `numerator / denominator` of the image rectangle.
    FractionOfImage {
        numerator: u64,
        denominator: u64,
    },
    /// The largest number of pixels that can differ by more than one LSB while
    /// 25.1's fraction still holds over the whole frame.
    ///
    /// Integer arithmetic, exactly: the rule needs
    /// `count_within(1) >= 0.999 * pixels`, so the budget is
    /// `pixels - ceil(999 * pixels / 1000)`. On the declared 512 by 512 canvas
    /// that is `262144 - 261882 = 262`, and 262.144 being fractional is
    /// exactly why this is computed rather than written down.
    WithinTheFractionBudget,
    /// One pixel more than that.
    OneOverTheFractionBudget,
}

impl PixelCount {
    /// # Errors
    /// Never. The signature is a `Result` so a future count that can fail to
    /// resolve does not change every call site.
    pub fn resolve(self, frame_pixels: u64, image_pixels: u64) -> Result<u64, MutationError> {
        Ok(match self {
            Self::One => 1,
            Self::FractionOfImage {
                numerator,
                denominator,
            } => image_pixels
                .saturating_mul(numerator)
                .checked_div(denominator)
                .unwrap_or(0),
            Self::WithinTheFractionBudget => fraction_budget(frame_pixels),
            Self::OneOverTheFractionBudget => fraction_budget(frame_pixels).saturating_add(1),
        })
    }
}

/// `pixels - ceil(999 * pixels / 1000)`, in integers.
fn fraction_budget(frame_pixels: u64) -> u64 {
    let numerator = frame_pixels.saturating_mul(999);
    let ceiling = numerator.div_ceil(1000);
    frame_pixels.saturating_sub(ceiling)
}

/// What the mutation does.
#[derive(Clone, Copy, Debug)]
pub enum Effect {
    /// Add a signed delta to a count of image-rectangle pixels, skipping any
    /// pixel where the result would clip, so the delta the frame carries is
    /// the delta the catalogue declared.
    AddDelta { count: PixelCount, delta: i16 },
    /// Apply the actual LINEAR to LINEAR_EXACT swap to every pixel.
    ///
    /// The per-pixel divergence is exactly `u / w`, where `u` is the LINEAR
    /// display value and `w` the window width, so the swapped value is
    /// `round(u - u / w)`. Derived in exact rational arithmetic over five
    /// windows in the S03 sprint review, not fitted.
    ///
    /// **This exists because `AddDelta` could not represent the real thing.**
    /// A flat delta over a declared fraction of the image is a caricature: it
    /// moves every chosen pixel by a whole code, where the real divergence
    /// moves each pixel by a sub-code amount that only sometimes crosses a
    /// rounding boundary, and it moves them uniformly where the real one is
    /// proportional to the display value. A mutation that clears the bound by
    /// four times proves the bound catches THAT mutation and nothing else.
    VoiLinearExactSwap,
    /// Shift the frame one canvas pixel to the right.
    TranslateOnePixel,
    /// Swap the red and blue lanes.
    SwapRedAndBlue,
    /// Set one pixel's alpha.
    SetAlpha(u8),
    /// Overwrite a numeric sidecar field.
    SidecarNumber { pointer: &'static str, value: f64 },
    /// Overwrite a string sidecar field.
    SidecarString {
        pointer: &'static str,
        value: &'static str,
    },
    /// Add a delta to a numeric sidecar field, for the geometry boundary
    /// cases where the interesting quantity is the SIZE of the change.
    SidecarDelta { pointer: &'static str, delta: f64 },
    /// Replace the sidecar's declared frame digest, leaving the pixels alone.
    CorruptDeclaredDigest,
    /// Drop the view from one side entirely.
    RemoveView,
    /// Put a `.raw` in the directory listing that no declared list names.
    OrphanRaw,
    /// Change one of `run.json`'s input digests.
    RunDigest { key: &'static str },
}

/// What the run must then say.
#[derive(Clone, Copy, Debug)]
pub enum Expectation {
    /// The structural verification refuses, and its message carries this.
    StructuralRefusal(&'static str),
    /// Reading or comparing the targeted view refuses, and its message carries
    /// this.
    ComparisonRefusal(&'static str),
    /// The run-level check reports a problem carrying this.
    RunProblem(&'static str),
    /// The targeted view reaches this outcome, carrying every qualifier
    /// listed, attributed to this side.
    View {
        outcome: Outcome,
        qualifiers: &'static [Qualifier],
        side: Side,
    },
    /// The targeted view keeps its `unmeasured` outcome and the statistics
    /// nonetheless show the damage. This is what a class-two mutation can
    /// prove, because deviation D-16 gives class two no verdict to flip.
    UnmeasuredAndDetected,
}

/// One declared mutation.
#[derive(Clone, Copy, Debug)]
pub struct Mutation {
    pub name: &'static str,
    pub why: &'static str,
    pub side: MutatedSide,
    pub target: Target,
    pub effect: Effect,
    pub expect: Expectation,
}

/// The catalogue. Order is the order they run in and is not otherwise
/// meaningful.
pub const CATALOGUE: &[Mutation] = &[
    Mutation {
        name: "plus-one-on-a-twentieth-of-a-percent",
        why: "A difference of one display code on a small fraction of the image \
              passes 25.1's first two clauses by construction, because every \
              pixel is still within one LSB. It is here to show what the \
              written maximum-difference rule cannot see, and it is the small \
              end of the same thing the bias bullet catches at scale.",
        side: MutatedSide::Candidate,
        target: Target::MeasuredStack,
        effect: Effect::AddDelta {
            count: PixelCount::FractionOfImage {
                numerator: 5,
                denominator: 10_000,
            },
            delta: 1,
        },
        expect: Expectation::View {
            outcome: Outcome::Pass,
            qualifiers: &[],
            side: Side::None,
        },
    },
    Mutation {
        name: "the-actual-linear-exact-swap",
        why: "The real thing rather than a caricature of it. Every pixel moves \
              by `u / w`, which is the exact divergence between LINEAR and \
              LINEAR_EXACT derived in rational arithmetic, so most pixels do \
              not cross a rounding boundary at all and the ones that do move \
              by a single code. `plus-one-on-two-fifths-of-the-image` moves 40 \
              per cent of the image by a whole code and clears the bias bound \
              several times over, which proves the bound catches THAT and says \
              nothing about the divergence HLD 18.3 is about. This one is the \
              divergence. It is also why the bound is evaluated over the \
              informative region: over the whole image rectangle this mutation \
              is NOT detected on any view in the corpus, because the clipped \
              pixels that cannot show it outnumber the ones that can.",
        side: MutatedSide::Candidate,
        target: Target::MeasuredStack,
        effect: Effect::VoiLinearExactSwap,
        expect: Expectation::View {
            outcome: Outcome::Fail,
            qualifiers: &[Qualifier::Bias],
            side: Side::Ours,
        },
    },
    Mutation {
        name: "plus-one-on-two-fifths-of-the-image",
        why: "The LINEAR against LINEAR_EXACT signature at corpus scale: a \
              one-sided difference of one code over a large fraction of the \
              image. 25.1's maximum-difference rule passes it and the bias \
              bullet the operator added in S03 does not. This is the mutation \
              that proves the new bound detects the divergence HLD 18.3 calls \
              the entire argument for building the oracle first.",
        side: MutatedSide::Candidate,
        target: Target::MeasuredStack,
        effect: Effect::AddDelta {
            count: PixelCount::FractionOfImage {
                numerator: 2,
                denominator: 5,
            },
            delta: 1,
        },
        expect: Expectation::View {
            outcome: Outcome::Fail,
            qualifiers: &[Qualifier::Bias],
            side: Side::Ours,
        },
    },
    Mutation {
        name: "plus-two-at-the-fraction-budget",
        why: "25.1 permits a pixel differing by exactly 2, for up to 0.1% of \
              the frame. This is that boundary from the passing side, and it is \
              the case a comparator that read `more than 2` as `2 or more` \
              would fail.",
        side: MutatedSide::Candidate,
        target: Target::MeasuredStack,
        effect: Effect::AddDelta {
            count: PixelCount::WithinTheFractionBudget,
            delta: 2,
        },
        expect: Expectation::View {
            outcome: Outcome::Pass,
            qualifiers: &[],
            side: Side::None,
        },
    },
    Mutation {
        name: "plus-two-one-pixel-over-the-budget",
        why: "The same boundary from the failing side. One pixel is the whole \
              difference between this and the entry above, which is what makes \
              the pair a boundary test rather than two unrelated cases.",
        side: MutatedSide::Candidate,
        target: Target::MeasuredStack,
        effect: Effect::AddDelta {
            count: PixelCount::OneOverTheFractionBudget,
            delta: 2,
        },
        expect: Expectation::View {
            outcome: Outcome::Fail,
            qualifiers: &[],
            side: Side::Ours,
        },
    },
    Mutation {
        name: "plus-three-on-one-pixel",
        why: "25.1's second clause is `zero pixels differing by more than 2`, \
              so one pixel at 3 fails at any count. A comparator that gated \
              only on the fraction would pass this, because 262143 of 262144 \
              pixels are identical.",
        side: MutatedSide::Candidate,
        target: Target::MeasuredStack,
        effect: Effect::AddDelta {
            count: PixelCount::One,
            delta: 3,
        },
        expect: Expectation::View {
            outcome: Outcome::Fail,
            qualifiers: &[],
            side: Side::Ours,
        },
    },
    Mutation {
        name: "plus-three-on-one-pixel-of-a-reformat",
        why: "The same damage on a volume reformat. It is here because \
              run.json's rows[] is stack-only, so a comparator that read that \
              list alone would compare eighty-nine of ninety-eight views and \
              report success, and a catalogue that only ever damaged a stack \
              row would not notice.",
        side: MutatedSide::Candidate,
        target: Target::MeasuredReformat,
        effect: Effect::AddDelta {
            count: PixelCount::One,
            delta: 3,
        },
        expect: Expectation::View {
            outcome: Outcome::Fail,
            qualifiers: &[],
            side: Side::Ours,
        },
    },
    Mutation {
        name: "one-pixel-canvas-translation",
        why: "The frame shifted one canvas pixel right, with the camera left \
              alone. Geometry agrees because the declared camera is unchanged, \
              so this is rung 5: a pixel divergence with parameters and \
              geometry agreeing, attributed to ours. rowsTouched and \
              columnsTouched are what tell a reader it is a fit error rather \
              than a LUT error.",
        side: MutatedSide::Candidate,
        target: Target::MeasuredStack,
        effect: Effect::TranslateOnePixel,
        expect: Expectation::View {
            outcome: Outcome::Fail,
            qualifiers: &[],
            side: Side::Ours,
        },
    },
    Mutation {
        name: "red-and-blue-swapped-on-a-colour-view",
        why: "Deviation D-16 gives class two no verdict to flip, so what this \
              has to show is DETECTION: the outcome stays `unmeasured` and the \
              per-channel statistics carry the damage. A comparator that \
              returned `unmeasured` without measuring would pass the identity \
              run and fail this one.",
        side: MutatedSide::Candidate,
        target: Target::ColourClassTwo,
        effect: Effect::SwapRedAndBlue,
        expect: Expectation::UnmeasuredAndDetected,
    },
    Mutation {
        name: "plus-three-on-the-eight-bit-greyscale-row",
        why: "The one corpus row HLD 25.1 has no class for at all: an 8-bit \
              MONOCHROME2 ultrasound, absorbed into class two by modality per \
              docs/lld/corpus.md, and decimated as well, so it carries two \
              qualifiers and no evaluable bound. It can never pass or fail, so \
              the only thing that can be asked of it is that the instrument \
              still MEASURES it. A view with no bound is exactly where a \
              comparator would stop looking.",
        side: MutatedSide::Candidate,
        target: Target::ClassTwo,
        effect: Effect::AddDelta {
            count: PixelCount::One,
            delta: 3,
        },
        expect: Expectation::UnmeasuredAndDetected,
    },
    Mutation {
        name: "alpha-254-on-one-pixel",
        why: "Alpha never enters a difference, so a frame that is not fully \
              opaque has to be refused rather than compared. Without the \
              refusal this mutation would be invisible.",
        side: MutatedSide::Candidate,
        target: Target::MeasuredStack,
        effect: Effect::SetAlpha(254),
        expect: Expectation::ComparisonRefusal("not fully opaque"),
    },
    Mutation {
        name: "candidate-image-slope-changed",
        why: "HLD section 11: metadata is diffed alongside pixels because a \
              wrong rescale slope can still produce a plausible image. The \
              pixels are untouched here and the declared slope is not, so the \
              only thing that can catch it is rung 2. The candidate's \
              /image/slope no longer agrees with its own /attributes/ \
              rescaleSlope, which dicom-parser read straight from the bytes, \
              and the reference's still does, so the divergence is attributed \
              to ours.",
        side: MutatedSide::Candidate,
        target: Target::MeasuredStack,
        effect: Effect::SidecarNumber {
            pointer: "/image/slope",
            value: 2.0,
        },
        expect: Expectation::View {
            outcome: Outcome::Fail,
            qualifiers: &[Qualifier::ParameterDivergence],
            side: Side::Ours,
        },
    },
    Mutation {
        name: "reference-image-slope-changed",
        why: "The same damage on the other side, and it is the entry that \
              proves the attribution is a measurement rather than a default. \
              Now the REFERENCE's /image/slope disagrees with its own \
              independent reading and the candidate's agrees, so rung 2 names \
              the reference. Without this pair, `attributed to ours` would be \
              indistinguishable from a constant.",
        side: MutatedSide::Reference,
        target: Target::MeasuredStack,
        effect: Effect::SidecarNumber {
            pointer: "/image/slope",
            value: 2.0,
        },
        expect: Expectation::View {
            outcome: Outcome::Fail,
            qualifiers: &[Qualifier::ParameterDivergence],
            side: Side::Reference,
        },
    },
    Mutation {
        name: "parallel-scale-inside-the-world-bound",
        why: "5e-7 mm is inside 25.1's 1e-6 mm, so the verdict does not move. \
              A geometry check that compared for exact equality would fail this \
              and a tolerance that had been widened would pass the entry below \
              as well.",
        side: MutatedSide::Candidate,
        target: Target::MeasuredStack,
        effect: Effect::SidecarDelta {
            pointer: "/camera/parallelScale",
            delta: 5e-7,
        },
        expect: Expectation::View {
            outcome: Outcome::Pass,
            qualifiers: &[],
            side: Side::None,
        },
    },
    Mutation {
        name: "parallel-scale-outside-the-world-bound",
        why: "2e-6 mm is over it. The same perturbation the reference half's \
              own `volume-geometry-drift` fault uses against the same bullet, \
              so the two halves of the harness are held to one number.",
        side: MutatedSide::Candidate,
        target: Target::MeasuredStack,
        effect: Effect::SidecarDelta {
            pointer: "/camera/parallelScale",
            delta: 2e-6,
        },
        expect: Expectation::View {
            outcome: Outcome::Fail,
            qualifiers: &[Qualifier::GeometryDivergence],
            side: Side::Fit,
        },
    },
    Mutation {
        name: "declared-frame-digest-corrupted",
        why: "The reference half hashes every frame twice already. This is the \
              third hash, taken at read, and it is what stops a file edited \
              after the run being compared as though somebody had rendered it.",
        side: MutatedSide::Candidate,
        target: Target::MeasuredStack,
        effect: Effect::CorruptDeclaredDigest,
        expect: Expectation::ComparisonRefusal("hashes to"),
    },
    Mutation {
        name: "an-undeclared-raw-in-the-directory",
        why: "THE guard. run.json's rows[] is stack-only and the reformats are \
              in volumes[].frames[], so a comparator reading one list would \
              compare part of the run and report success. Any `.raw` no \
              declared list names fails the run, and that keeps working when a \
              later story adds a third list.",
        side: MutatedSide::Candidate,
        target: Target::MeasuredStack,
        effect: Effect::OrphanRaw,
        expect: Expectation::StructuralRefusal("no declared frame list names it"),
    },
    Mutation {
        name: "a-view-missing-from-the-candidate",
        why: "An identifier on one side only is `absent` and a run failure, \
              never a skip. A comparator that iterated the intersection would \
              report a green run over ninety-seven views.",
        side: MutatedSide::Candidate,
        target: Target::MeasuredStack,
        effect: Effect::RemoveView,
        expect: Expectation::RunProblem("is declared on one side and not the other"),
    },
    Mutation {
        name: "the-manifest-digest-disagrees",
        why: "Two frames produced from different corpora are two correct frames \
              that differ. Every *Sha256 key present on either side must be \
              present on both and equal, and the check is by suffix rather than \
              from a fixed list, so F-X007 adding volumeParamsSha256 did not \
              silently stop covering an input that decides the frames.",
        side: MutatedSide::Candidate,
        target: Target::MeasuredStack,
        effect: Effect::RunDigest {
            key: "manifestSha256",
        },
        expect: Expectation::RunProblem("manifestSha256"),
    },
    Mutation {
        name: "the-render-params-digest-disagrees",
        why: "render-params.json's own note says a change there changes every \
              reference frame. The same refusal as above, on the input that \
              decides the window, the camera and the canvas.",
        side: MutatedSide::Candidate,
        target: Target::MeasuredStack,
        effect: Effect::RunDigest {
            key: "renderParamsSha256",
        },
        expect: Expectation::RunProblem("renderParamsSha256"),
    },
    Mutation {
        name: "a-sidecar-carrying-an-unknown-kind",
        why: "An unknown `kind` is refused rather than defaulted to `stack`, \
              because defaulting is how a new view kind gets compared under the \
              wrong rules. F-X007 was the first story to add one and it will \
              not be the last.",
        side: MutatedSide::Candidate,
        target: Target::MeasuredStack,
        effect: Effect::SidecarString {
            pointer: "/kind",
            value: "volume",
        },
        expect: Expectation::StructuralRefusal("is not one this comparator knows"),
    },
];

/// Which view this mutation lands on, resolved against the identity run.
///
/// # Errors
/// When no view in the identity run matches the target, which means the corpus
/// changed shape under the catalogue and the catalogue has to be looked at
/// rather than the corpus.
pub fn resolve_target(
    mutation: &Mutation,
    records: &[ViewRecord],
) -> Result<String, MutationError> {
    let found = records.iter().find(|record| match mutation.target {
        Target::MeasuredStack => {
            record.kind == ViewKind::Stack
                && record.class == ToleranceClass::MonochromeSixteenBit
                && record.outcome == Outcome::Pass
        }
        Target::MeasuredReformat => {
            record.kind == ViewKind::VolumeReformat && record.outcome == Outcome::Pass
        }
        Target::ClassTwo => record.class == ToleranceClass::ColourOrUltrasound,
        Target::ColourClassTwo => {
            record.class == ToleranceClass::ColourOrUltrasound && !record.monochrome_frame
        }
    });
    found
        .map(|record| record.id.clone())
        .ok_or(MutationError::NoTarget(mutation.name))
}

/// Whether this mutation damages the frame rather than the run record.
#[must_use]
pub fn touches_the_frame(mutation: &Mutation) -> bool {
    matches!(
        mutation.effect,
        Effect::AddDelta { .. }
            | Effect::VoiLinearExactSwap
            | Effect::TranslateOnePixel
            | Effect::SwapRedAndBlue
            | Effect::SetAlpha(_)
    )
}

/// Apply the run-record half of a mutation, in memory.
///
/// # Errors
/// When the mutation names a field the sidecar does not carry, which means the
/// catalogue and the contract have parted company.
pub fn apply_to_run(mutation: &Mutation, run: &mut Run, target: &str) -> Result<(), MutationError> {
    match mutation.effect {
        Effect::SidecarNumber { pointer, value } => {
            set_sidecar(run, target, pointer, Value::from(value), mutation.name)
        }
        Effect::SidecarString { pointer, value } => {
            set_sidecar(run, target, pointer, Value::from(value), mutation.name)
        }
        Effect::SidecarDelta { pointer, delta } => {
            let current = run
                .sidecars
                .get(target)
                .and_then(|sidecar| sidecar.json.pointer(pointer))
                .and_then(Value::as_f64)
                .ok_or_else(|| {
                    MutationError::Apply(
                        mutation.name,
                        format!("{target} carries no number at {pointer}"),
                    )
                })?;
            set_sidecar(
                run,
                target,
                pointer,
                Value::from(current + delta),
                mutation.name,
            )
        }
        Effect::CorruptDeclaredDigest => set_sidecar(
            run,
            target,
            "/frame/sha256",
            Value::from("0000000000000000000000000000000000000000000000000000000000000000"),
            mutation.name,
        ),
        Effect::RemoveView => {
            run.views.remove(target);
            run.sidecars.remove(target);
            run.raw_present.remove(target);
            run.categories.remove(target);
            for ids in run.by_path.values_mut() {
                ids.remove(target);
            }
            Ok(())
        }
        Effect::OrphanRaw => {
            run.raw_present
                .insert("nobody__declared_this_frame".to_owned());
            Ok(())
        }
        Effect::RunDigest { key } => {
            let Some(object) = run.json.as_object_mut() else {
                return Err(MutationError::Apply(
                    mutation.name,
                    "run.json is not an object".to_owned(),
                ));
            };
            object.insert(
                key.to_owned(),
                Value::from("1111111111111111111111111111111111111111111111111111111111111111"),
            );
            Ok(())
        }
        Effect::AddDelta { .. }
        | Effect::VoiLinearExactSwap
        | Effect::TranslateOnePixel
        | Effect::SwapRedAndBlue
        | Effect::SetAlpha(_) => Ok(()),
    }
}

fn set_sidecar(
    run: &mut Run,
    target: &str,
    pointer: &str,
    value: Value,
    name: &'static str,
) -> Result<(), MutationError> {
    let sidecar = run
        .sidecars
        .get_mut(target)
        .ok_or_else(|| MutationError::Apply(name, format!("{target} is not in this run")))?;
    let slot = sidecar
        .json
        .pointer_mut(pointer)
        .ok_or_else(|| MutationError::Apply(name, format!("{target} carries no {pointer}")))?;
    *slot = value;
    Ok(())
}

/// Apply the pixel half of a mutation to one frame, in memory.
///
/// # Errors
/// When the frame cannot carry the declared damage, for instance when the
/// image rectangle holds fewer unclipped pixels than the mutation needs. That
/// is an error rather than a smaller mutation, because a catalogue entry that
/// quietly did less than it declared would report a guard as watched when it
/// was not.
pub fn apply_to_frame(
    mutation: &Mutation,
    frame: &mut Frame,
    image: &Rect,
    window_width: Option<u32>,
) -> Result<(), MutationError> {
    match mutation.effect {
        Effect::AddDelta { count, delta } => {
            let frame_pixels = u64::from(frame.width()) * u64::from(frame.height());
            let wanted = count.resolve(frame_pixels, image.pixels())?;
            let mut applied = 0_u64;
            for y in image.y0..image.y0.saturating_add(image.height) {
                for x in image.x0..image.x0.saturating_add(image.width) {
                    if applied >= wanted {
                        break;
                    }
                    let pixel = frame.pixel(x, y)?;
                    let Some(grey) = pixel.first() else { continue };
                    let moved = i16::from(*grey) + delta;
                    let Ok(byte) = u8::try_from(moved) else {
                        continue;
                    };
                    frame.set_pixel(x, y, [byte, byte, byte, u8::MAX])?;
                    applied = applied.saturating_add(1);
                }
            }
            if applied < wanted {
                return Err(MutationError::Apply(
                    mutation.name,
                    format!(
                        "the image rectangle holds only {applied} pixels that can \
                         carry a delta of {delta} without clipping, and {wanted} \
                         were declared"
                    ),
                ));
            }
            Ok(())
        }
        Effect::VoiLinearExactSwap => {
            // The view's OWN window, read from its sidecar by the caller. Not a
            // constant in the catalogue, because a divergence of `u / w` is a
            // statement about the window the frame was actually rendered with,
            // and a catalogue that hardcoded one would silently stop describing
            // the view the day the target resolved elsewhere.
            let Some(window_width) = window_width.filter(|w| *w > 0) else {
                return Err(MutationError::Apply(
                    mutation.name,
                    "this view's sidecar carries no positive window width, so \
                     the LINEAR to LINEAR_EXACT divergence is not defined for it"
                        .to_owned(),
                ));
            };
            // `round(u * (w - 1) / w)` in integers, which is exactly
            // `round(u - u / w)`. `(2a + b) / (2b)` is round-half-up for
            // non-negative integers, so there is no float, no `as` cast and no
            // rounding decision left implicit. u is at most 255 and w fits a
            // u32, so u64 cannot overflow here.
            let w = u64::from(window_width);
            let mut moved = 0_u64;
            for y in image.y0..image.y0.saturating_add(image.height) {
                for x in image.x0..image.x0.saturating_add(image.width) {
                    let pixel = frame.pixel(x, y)?;
                    let Some(grey) = pixel.first() else { continue };
                    let u = u64::from(*grey);
                    let swapped = (2 * u * (w - 1) + w) / (2 * w);
                    let byte = u8::try_from(swapped).unwrap_or(*grey);
                    if byte != *grey {
                        moved = moved.saturating_add(1);
                    }
                    frame.set_pixel(x, y, [byte, byte, byte, u8::MAX])?;
                }
            }
            if moved == 0 {
                return Err(MutationError::Apply(
                    mutation.name,
                    format!(
                        "the swap moved no pixel at window width {window_width}, \
                         so this view cannot show the divergence and declaring it \
                         detectable would be false"
                    ),
                ));
            }
            Ok(())
        }
        Effect::TranslateOnePixel => {
            let source = frame.clone();
            for y in 0..frame.height() {
                for x in (1..frame.width()).rev() {
                    let pixel = source.pixel(x.saturating_sub(1), y)?;
                    frame.set_pixel(x, y, pixel)?;
                }
            }
            Ok(())
        }
        Effect::SwapRedAndBlue => {
            for y in 0..frame.height() {
                for x in 0..frame.width() {
                    let pixel = frame.pixel(x, y)?;
                    let (Some(r), Some(g), Some(b), Some(a)) =
                        (pixel.first(), pixel.get(1), pixel.get(2), pixel.get(3))
                    else {
                        continue;
                    };
                    frame.set_pixel(x, y, [*b, *g, *r, *a])?;
                }
            }
            Ok(())
        }
        Effect::SetAlpha(alpha) => {
            let pixel = frame.pixel(image.x0, image.y0)?;
            let (Some(r), Some(g), Some(b)) = (pixel.first(), pixel.get(1), pixel.get(2)) else {
                return Ok(());
            };
            frame.set_pixel(image.x0, image.y0, [*r, *g, *b, alpha])?;
            Ok(())
        }
        Effect::SidecarNumber { .. }
        | Effect::SidecarString { .. }
        | Effect::SidecarDelta { .. }
        | Effect::CorruptDeclaredDigest
        | Effect::RemoveView
        | Effect::OrphanRaw
        | Effect::RunDigest { .. } => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::{CATALOGUE, PixelCount, fraction_budget};
    use crate::tolerance::MONOCHROME_WITHIN_ONE_LSB_FRACTION;

    /// The budget on the declared 512 by 512 canvas, hand-computed:
    /// 512 * 512 = 262144 pixels, and 25.1 needs at least 99.9% of them within
    /// one LSB, so `ceil(999 * 262144 / 1000) = ceil(261881.856) = 261882`
    /// pixels must be within 1 and at most `262144 - 261882 = 262` may not be.
    ///
    /// 262.144 is not an integer, which is exactly why the catalogue computes
    /// this rather than writing 262 down beside a canvas size that a later
    /// story could change.
    #[test]
    fn the_fraction_budget_is_computed_and_not_written_down() {
        assert_eq!(fraction_budget(262_144), 262);
        assert_eq!(fraction_budget(10_000), 10, "0.1% of 10000 is exactly 10");
        assert_eq!(fraction_budget(1), 0, "a one-pixel frame has no budget");
    }

    /// The budget is the largest count that still satisfies the written
    /// fraction, and one more does not. Asserted against the constant rather
    /// than against the number above, so a widened tolerance moves both.
    #[test]
    fn the_budget_is_the_boundary_of_the_written_fraction() {
        let pixels = 262_144_u64;
        let budget = fraction_budget(pixels);
        let within = pixels - budget;
        let fraction = f64::from(u32::try_from(within).unwrap_or(0))
            / f64::from(u32::try_from(pixels).unwrap_or(1));
        assert!(fraction >= MONOCHROME_WITHIN_ONE_LSB_FRACTION);
        let over = f64::from(u32::try_from(within - 1).unwrap_or(0))
            / f64::from(u32::try_from(pixels).unwrap_or(1));
        assert!(over < MONOCHROME_WITHIN_ONE_LSB_FRACTION);
    }

    #[test]
    fn a_fraction_of_the_image_resolves_by_integer_arithmetic() {
        let count = PixelCount::FractionOfImage {
            numerator: 2,
            denominator: 5,
        };
        assert_eq!(count.resolve(262_144, 196_608).ok(), Some(78_643));
    }

    /// Every catalogue entry has a distinct name, because the runner reports
    /// by name and two entries sharing one would report as a single guard.
    #[test]
    fn every_catalogue_entry_has_a_distinct_name() {
        let mut names: Vec<&str> = CATALOGUE.iter().map(|entry| entry.name).collect();
        let total = names.len();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), total);
        assert!(total >= 18, "the declared catalogue is {total} entries");
    }

    /// Every entry says why it exists. A mutation with no rationale is a
    /// number nobody can review.
    #[test]
    fn every_catalogue_entry_explains_itself() {
        for entry in CATALOGUE {
            assert!(
                entry.why.len() > 80,
                "{}: the rationale is too short to be one",
                entry.name
            );
        }
    }
}
