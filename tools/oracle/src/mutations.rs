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
//! report shape over all ninety-nine views. **It proves nothing about
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
    /// One exact declared view. Metadata truth mutations use named synthetic
    /// fixtures because their expected values are committed independently.
    View(&'static str),
    /// The first class-one stack view that passes cleanly, so the mutation's
    /// effect on the outcome is unambiguous.
    ///
    /// **Deliberately not narrowed by ramp direction.** The seventeen entries
    /// that land here move a declared number of codes, or a sidecar field, or
    /// the frame's position on the canvas, and none of those means anything
    /// different on an inverted frame. The eighteenth stack entry is the one
    /// whose arithmetic does depend on the ramp, and it takes
    /// `MeasuredMonochrome2Stack` below instead.
    /// The eighth review pass caught this the right way round: while the
    /// photometric predicate sat here, an unrelated `AddDelta` entry was the
    /// first thing to refuse when the sidecar pointer it reads was mutated.
    MeasuredStack,
    /// The same, and positively `MONOCHROME2`.
    ///
    /// **The photometric interpretation is part of THIS target and not a
    /// description of what today's corpus happens to hold.**
    /// `Effect::VoiLinearExactSwap` only ever DARKENS, and `apply_to_frame`'s
    /// derivation says why that is the LINEAR to LINEAR_EXACT divergence:
    /// `y_E` sits `y_L / w` BELOW `y_L`. Under `MONOCHROME1` the ramp is
    /// inverted, PS3.3 C.7.6.3.1.2, so the rendered byte is `255 - y`, the
    /// drop direction reverses and the two display-extreme exclusions swap
    /// ends. The same accumulator applied there would still be detected, and
    /// would still not be the divergence, which is the exact shape of the
    /// defect the review's third pass found in `round(u - u / w)`.
    ///
    /// Until this narrowing, only `real` sorting before `synthetic` in the
    /// `BTreeSet` kept the swap off `synthetic__cr_monochrome1`, which is a
    /// `mono16` stack view that passes. That is the same accident smell S4
    /// named on `MeasuredReformat`, and here it guarded arithmetic rather than
    /// a ladder rung, so it is closed by the predicate rather than described.
    /// `ColourClassTwo` below is the precedent this follows exactly: a
    /// SEPARATE target narrowed by the frame property one mutation's meaning
    /// depends on, leaving the shared one unnarrowed.
    MeasuredMonochrome2Stack,
    /// The first volume reformat that passes cleanly. Present so the
    /// catalogue reaches the nine views `rows[]` does not name.
    ///
    /// **It is not pinned to a subject carrying a declared reference
    /// divergence and nothing here should assume it is.** On today's corpus it
    /// resolves to `volume__real__mr_eay131__AXIAL`, which does carry one, but
    /// only because records are iterated from a `BTreeSet` and `real` sorts
    /// before `synthetic`. The S03 sprint review's smell S4 is that the
    /// `!geometry.is_empty()` narrowing on the attribution ladder's rung 3 was
    /// watched by that accident alone. It is now watched by three unit tests
    /// in `attribution.rs`, which build both sides themselves and cannot be
    /// satisfied by an iteration order.
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
    /// LINEAR_EXACT sits `u / w` below LINEAR before the renderer quantises,
    /// where `u` is the LINEAR display value and `w` the window width, so a
    /// pixel drops one display code with probability `u / w` and the rest do
    /// not, and at `u > w` it drops more than one. An accumulator over the
    /// image rectangle reproduces that exactly and deterministically.
    ///
    /// **Both display extremes are excluded, and the two exclusions are not
    /// equally tight.** A pixel at 0 cannot move at any width, so excluding it
    /// is EXACT. A pixel at 255 can move at EVERY width, over the band where
    /// LINEAR rounded up to 255 and LINEAR_EXACT falls under 254.5, so
    /// excluding it is CONSERVATIVE at every width and exact at none. An
    /// 8-bit frame does not carry the stored value behind a 255, so excluding
    /// the whole population under-damages the frame rather than stating that
    /// nothing could have moved. `apply_to_frame` carries the derivation at
    /// the site, and it is the only statement of this arithmetic in the file.
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
    /// Change committed metadata and make either frame reader fail if the
    /// comparator reaches it. This watches metadata-before-frame precedence
    /// at the production `compare_runs` call site.
    SidecarStringAndFrameRefusal {
        pointer: &'static str,
        value: &'static str,
        message: &'static str,
    },
    /// Add a delta to a numeric sidecar field, for the geometry boundary
    /// cases where the interesting quantity is the SIZE of the change.
    SidecarDelta { pointer: &'static str, delta: f64 },
    /// Swap two entries in an array-valued sidecar field.
    SidecarArraySwap {
        pointer: &'static str,
        first: usize,
        second: usize,
    },
    /// Exchange the row and column direction triples of an IOP value.
    SidecarDirectionSwap { pointer: &'static str },
    /// Replace a resolved field with JSON null. Used to model a reader taking
    /// an absent top-level value instead of the per-frame functional group.
    SidecarNull { pointer: &'static str },
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
        name: "metadata-pixel-spacing-transposed",
        why: "PS3.3 C.7.6.2.1.1 stores row spacing before column spacing. The committed non-square truth makes reversing them attributable to the candidate before pixels are compared.",
        side: MutatedSide::Candidate,
        target: Target::View("synthetic__mr_nonsquare_spacing"),
        effect: Effect::SidecarArraySwap {
            pointer: "/attributes/pixelSpacing",
            first: 0,
            second: 1,
        },
        expect: Expectation::View {
            outcome: Outcome::Fail,
            qualifiers: &[Qualifier::MetadataTruth],
            side: Side::Ours,
        },
    },
    Mutation {
        name: "metadata-iop-vectors-reversed",
        why: "PS3.3 C.7.6.2.1.1 gives the row direction before the column direction. The oblique geometry fixture makes exchanging those triples visible.",
        side: MutatedSide::Candidate,
        target: Target::View("synthetic__mr_nonsquare_spacing"),
        effect: Effect::SidecarDirectionSwap {
            pointer: "/attributes/imageOrientationPatient",
        },
        expect: Expectation::View {
            outcome: Outcome::Fail,
            qualifiers: &[Qualifier::MetadataTruth],
            side: Side::Ours,
        },
    },
    Mutation {
        name: "metadata-rescale-intercept-wrong",
        why: "HLD section 11 names a wrong rescale value as metadata damage that can still produce a plausible image. The committed C.11 truth attributes it without consulting pixels.",
        side: MutatedSide::Candidate,
        target: Target::View("synthetic__ct_unsigned_16"),
        effect: Effect::SidecarNumber {
            pointer: "/attributes/rescaleIntercept",
            value: -1024.0,
        },
        expect: Expectation::View {
            outcome: Outcome::Fail,
            qualifiers: &[Qualifier::MetadataTruth],
            side: Side::Ours,
        },
    },
    Mutation {
        name: "metadata-window-function-wrong",
        why: "PS3.3 C.11.2 gives LINEAR, LINEAR_EXACT and SIGMOID different arithmetic. A changed declaration is a truth failure before any pixel tolerance applies.",
        side: MutatedSide::Candidate,
        target: Target::View("synthetic__ct_unsigned_16"),
        effect: Effect::SidecarString {
            pointer: "/attributes/voiLutFunction",
            value: "LINEAR_EXACT",
        },
        expect: Expectation::View {
            outcome: Outcome::Fail,
            qualifiers: &[Qualifier::MetadataTruth],
            side: Side::Ours,
        },
    },
    Mutation {
        name: "metadata-top-level-used-instead-of-per-frame",
        why: "PS3.3 C.7.6.16 resolves the per-frame functional group before shared or top-level values. Replacing the resolved intercept with the absent top-level value must fail the committed per-frame truth.",
        side: MutatedSide::Candidate,
        target: Target::View("synthetic__ct_multiframe_perframe"),
        effect: Effect::SidecarNull {
            pointer: "/cornerstoneMetadata/modalityLutModule/rescaleIntercept",
        },
        expect: Expectation::View {
            outcome: Outcome::Fail,
            qualifiers: &[Qualifier::MetadataTruth],
            side: Side::Ours,
        },
    },
    Mutation {
        name: "metadata-per-frame-scope-labelled-top-level",
        why: "PS3.3 C.7.6.16.2.2.1 gives per-frame values precedence. The scope label is compared with provenance derived from the raw functional-group sequence and cannot be descriptive text only.",
        side: MutatedSide::Candidate,
        target: Target::View("synthetic__ct_multiframe_perframe"),
        effect: Effect::SidecarString {
            pointer: "/metadataSources/cornerstoneMetadata/modalityLutModule/rescaleIntercept",
            value: "top-level",
        },
        expect: Expectation::View {
            outcome: Outcome::Fail,
            qualifiers: &[Qualifier::MetadataTruth],
            side: Side::Ours,
        },
    },
    Mutation {
        name: "metadata-presentation-inversion-wrong",
        why: "PS3.3 C.11.6 gives INVERSE different presentation semantics from IDENTITY. The generated positive declaration must survive the independent parser and metadata truth comparison.",
        side: MutatedSide::Candidate,
        target: Target::View("synthetic__ct_unsigned_16"),
        effect: Effect::SidecarString {
            pointer: "/attributes/presentationLutShape",
            value: "IDENTITY",
        },
        expect: Expectation::View {
            outcome: Outcome::Fail,
            qualifiers: &[Qualifier::MetadataTruth],
            side: Side::Ours,
        },
    },
    Mutation {
        name: "metadata-failure-precedes-frame-refusal",
        why: "A committed metadata failure must remain reportable when either input frame is malformed. The combined mutation fails the production comparator if reference or candidate frame I/O moves ahead of metadata truth.",
        side: MutatedSide::Candidate,
        target: Target::View("synthetic__ct_unsigned_16"),
        effect: Effect::SidecarStringAndFrameRefusal {
            pointer: "/attributes/presentationLutShape",
            value: "IDENTITY",
            message: "the combined metadata mutation reached malformed frame I/O",
        },
        expect: Expectation::View {
            outcome: Outcome::Fail,
            qualifiers: &[Qualifier::MetadataTruth],
            side: Side::Ours,
        },
    },
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
        why: "The real thing rather than a caricature of it. LINEAR_EXACT sits \
              `u / w` below LINEAR before the renderer quantises, so a pixel \
              drops one code with probability `u / w` and the rest do not, \
              which an accumulator reproduces exactly and deterministically. \
              `plus-one-on-two-fifths-of-the-image` moves 40 \
              per cent of the image by a whole code and clears the bias bound \
              several times over, which proves the bound catches THAT and says \
              nothing about the divergence HLD 18.3 is about. This one is the \
              divergence. It is also why the bound is evaluated over the \
              informative region, and that is measured rather than asserted: \
              applied to all 70 gating class-one views and averaged over the \
              whole image rectangle instead, it exceeds the bound on 0 of \
              them, because the clipped pixels that cannot show it outnumber \
              the ones that can. Run `ocelli-compare census` for the table. \
              Until the sprint review's fourth pass that claim was true of one \
              view only and false of 44, because the accumulator dropped white \
              pixels, which an 8-bit frame gives no way to tell apart from the \
              255s PS3.3 C.11.2.1.2 and C.11.2.1.3.2 hold still.",
        side: MutatedSide::Candidate,
        target: Target::MeasuredMonochrome2Stack,
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
              list alone would compare ninety of ninety-nine views and \
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
        Target::View(id) => record.id == id,
        Target::MeasuredStack => {
            record.kind == ViewKind::Stack
                && record.class == ToleranceClass::MonochromeSixteenBit
                && record.outcome == Outcome::Pass
        }
        Target::MeasuredMonochrome2Stack => {
            record.kind == ViewKind::Stack
                && record.class == ToleranceClass::MonochromeSixteenBit
                && record.outcome == Outcome::Pass
                // Positively `MONOCHROME2`, not merely "not `MONOCHROME1`". A
                // stack sidecar that carries no photometric interpretation is
                // one whose ramp direction is unknown, and the swap's
                // direction is only derivable when it is known.
                && record.photometric_interpretation.as_deref() == Some("MONOCHROME2")
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

/// A declared frame-read refusal carried by a combined metadata mutation.
///
/// The comparator asks this before opening the real frame. A metadata failure
/// never gets here. Moving production frame I/O above metadata truth does,
/// which makes the standing mutation fail at the original defect boundary.
#[must_use]
pub fn frame_read_refusal(mutation: &Mutation) -> Option<&'static str> {
    match mutation.effect {
        Effect::SidecarStringAndFrameRefusal { message, .. } => Some(message),
        _ => None,
    }
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
        Effect::SidecarStringAndFrameRefusal { pointer, value, .. } => {
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
        Effect::SidecarArraySwap {
            pointer,
            first,
            second,
        } => {
            let mut value = run
                .sidecars
                .get(target)
                .and_then(|sidecar| sidecar.json.pointer(pointer))
                .and_then(Value::as_array)
                .cloned()
                .ok_or_else(|| {
                    MutationError::Apply(
                        mutation.name,
                        format!("{target} carries no array at {pointer}"),
                    )
                })?;
            if first >= value.len() || second >= value.len() {
                return Err(MutationError::Apply(
                    mutation.name,
                    format!("{target} array at {pointer} is too short"),
                ));
            }
            value.swap(first, second);
            set_sidecar(run, target, pointer, Value::Array(value), mutation.name)
        }
        Effect::SidecarDirectionSwap { pointer } => {
            let mut value = run
                .sidecars
                .get(target)
                .and_then(|sidecar| sidecar.json.pointer(pointer))
                .and_then(Value::as_array)
                .cloned()
                .ok_or_else(|| {
                    MutationError::Apply(
                        mutation.name,
                        format!("{target} carries no direction array at {pointer}"),
                    )
                })?;
            if value.len() != 6 {
                return Err(MutationError::Apply(
                    mutation.name,
                    format!("{target} direction at {pointer} does not have six values"),
                ));
            }
            for axis in 0..3 {
                value.swap(axis, axis + 3);
            }
            set_sidecar(run, target, pointer, Value::Array(value), mutation.name)
        }
        Effect::SidecarNull { pointer } => {
            set_sidecar(run, target, pointer, Value::Null, mutation.name)
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
/// image rectangle holds fewer unclipped pixels than the mutation needs, or
/// when it carries under one drop's worth of the LINEAR to LINEAR_EXACT
/// divergence. And when the damage delivered is not the damage declared: the
/// swap counts its drops and refuses if the total is not `floor(sum(u) / w)`.
/// All of these are errors rather than smaller mutations, because a catalogue
/// entry that quietly did less than it declared would report a guard as
/// watched when it was not.
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
            // **The renderer quantises a CONTINUOUS value, and that is the
            // whole of this.** LINEAR_EXACT sits `u / w` below LINEAR before
            // quantisation, so a pixel whose continuous value is within `u / w`
            // of a rounding boundary drops one code and every other pixel does
            // not. Over a region the effect is a drop rate of `u / w`,
            // proportional to the display value.
            //
            // An earlier version of this applied `round(u - u / w)` to the
            // already-quantised byte, and the sprint review's third pass caught
            // it. That is not the divergence, it is a threshold: it changes a
            // pixel only when `u >= w / 2`, so at width 400 it moved 55 of the
            // 256 codes and at any width from 510 upward **it moved nothing at
            // all** and then refused, claiming the view could not show the
            // divergence, which was false. Four tracked files called it the
            // real thing.
            //
            // The accumulator below is the real thing. `acc += u` each pixel,
            // and a drop is taken whenever it crosses `w`, which yields exactly
            // `floor(sum(u) / w)` drops placed in proportion to `u`. It is
            // deterministic, so the mutation is reproducible, and it needs no
            // float, no cast and no rounding decision.
            //
            // **EVERYTHING BELOW ASSUMES `MONOCHROME2`, AND UNDER
            // `MONOCHROME1` EVERY DIRECTION IN IT REVERSES.** PS3.3
            // C.7.6.3.1.2 defines `MONOCHROME1` so that the MINIMUM value is
            // displayed as white, which is the greyscale ramp of
            // `MONOCHROME2` inverted, so the byte in the rendered frame is
            // `255 - y` and not `y`: a LOWER display value from C.11.2 is a
            // BRIGHTER code here. The accumulated `u` would have to be
            // `255 - byte` rather than `byte`, the `saturating_sub` would have
            // to be an add, and the two exclusions would swap ends, the EXACT
            // one moving to the byte 255 and the CONSERVATIVE one to the byte
            // 0. Applying this arm unchanged to an inverted frame still
            // produces a one-sided difference the bound detects, so nothing
            // goes red, and that is precisely the failure this file already
            // repudiated once: a mutation that is detected but is not the
            // thing it says it is.
            //
            // This entry's own `Target::MeasuredMonochrome2Stack` is narrowed
            // to a `MONOCHROME2` view so the swap cannot reach an inverted
            // one. The shared `Target::MeasuredStack` the other seventeen
            // stack entries use is NOT narrowed, because a delta of a display
            // code means the same thing on either ramp. `ocelli-compare
            // census` is NOT narrowed, because its argument is about the whole
            // gating population, and today that population carries exactly one
            // inverted row, `synthetic__cr_monochrome1`. Its reported
            // `w=4096 bias=-0.0311` therefore has the WRONG SIGN and a
            // magnitude that is near-right only because `mean(u)` and
            // `255 - mean(u)` are close on that frame. It moves no verdict,
            // since the bound is two-sided and 0.0311 is far under 0.1, and
            // the census paragraph in `docs/lld/comparator.md` says so rather
            // than leaving the number to be read as measured.
            //
            // **BOTH display extremes are excluded, and the reason is not the
            // same at each end.** Derived from PS3.3 rather than from the
            // shape of the code below.
            //
            // C.11.2.1.2 gives LINEAR on `c' = c - 0.5` and `w' = w - 1`:
            // `x <= c' - w'/2` yields ymin, `x > c' + w'/2` yields ymax.
            // C.11.2.1.3.2 gives LINEAR_EXACT on `c` and `w` themselves:
            // `x <= c - w/2` yields ymin, `x > c + w/2` yields ymax.
            //
            // Write `y_L` and `y_E` for the two display values at the same
            // stored value `x`. Where NEITHER function clamps, the entire
            // divergence is one line:
            //
            //     y_L - y_E = 255 * [ (x - c')/w' - (x - c)/w ]
            //               = 255 * (x - c + w/2) / (w * w')
            //               = y_L / w
            //
            // the last step because `y_L = 255 * (x - c + w/2) / w'` by the
            // same algebra. That identity is the whole of this block. It is
            // asserted over a table of widths by
            // `the_divergence_is_the_display_value_over_the_width` in
            // `tools/oracle/tests/voi_divergence_fixture.rs`.
            //
            // **The exclusion at 0 is EXACT at every width, and coincident
            // clamps are only half the reason.** The lower clamps do coincide,
            // `c' - w'/2 = (c - 0.5) - (w - 1)/2 = c - w/2`, so a stored value
            // clamping to black under one function clamps under the other.
            // What carries the rest is that `y_E = y_L * (w - 1) / w` lies in
            // `[0, y_L]` everywhere the two functions are unclamped, so
            // `round(y_L) = 0` forces `round(y_E) = 0`. The bracket is CLOSED
            // at the top, because `y_E = y_L` at `y_L = 0`, and the half-open
            // form this used to be written in is the empty interval at exactly
            // the value the sentence is about. Coincident clamps alone say
            // nothing about the unclamped values just above them, and this
            // comment used to stop there.
            //
            // **The exclusion at 255 is CONSERVATIVE at every width and exact
            // at none.** A pixel the reference rendered 255 has
            // `round(y_L) = 255`, so `y_L >= 254.5`, and it moves when
            // `round(y_E) != 255`, that is when `y_L - y_L/w < 254.5`, that is
            // when `y_L < 254.5 * w / (w - 1)`. A display value cannot exceed
            // 255, so the movable set in display-value space is
            //
            //     y_L in [254.5, 254.5 * w / (w - 1))   intersected   [0, 255]
            //
            // which is the CLOSED `[254.5, 255]` for `w < 510`, `[254.5, 255)`
            // at `w = 510`, and strictly inside `[254.5, 255)` above it. It is
            // NON-EMPTY at every finite `w >= 2`, because 254.5 is strictly
            // below `254.5 * w / (w - 1)` there.
            //
            // **The top is open in the FORMULA and that is not a `min` with
            // 255.** Below 510 the formula's top lies ABOVE 255, so no `y_L`
            // reaches it and every display value up to and INCLUDING 255
            // moves. `y_L` is exactly 255 at `x = c + w/2 - 1`, the last
            // stored value LINEAR does not clamp, and at `w = 100`, 256 and
            // 400 that stored value is the ONLY mover the fixture's table
            // carries, `w = 400, x = 239` being the row that table hand-works.
            // Writing the top as `min(255, ...)` with an open bracket excluded
            // it, and the clamped interval `(c + w/2 - 1, c + w/2]` below is
            // open at its left, so it fell into neither stated region while
            // the fixture's own table moved it. The measure of the band is
            // `254.5 / (w - 1)` capped at `0.5`, and that cap is where the
            // `min` belongs and the only thing it means.
            //
            // **What `w >= 510` buys is only that a CLAMPED pixel cannot
            // move**, and four review passes in a row read that as the whole
            // statement. `254.5 * w / (w - 1) >= 255` exactly when `w <= 510`,
            // which is the only place 510 comes from. On
            // `(c + w/2 - 1, c + w/2]`, where LINEAR clamps to 255 and
            // LINEAR_EXACT does not, the lowest `y_E` is `255 - 255/w`, which
            // rounds back to 255 exactly when `w >= 510`. That says nothing
            // about the pixels LINEAR ROUNDED up to 255 from below, and that
            // band never closes. 510 is therefore not a threshold separating
            // two regimes and must not be written as one.
            //
            // Measured over integer stored values at centre 40, counting `x`
            // where LINEAR rounds to 255 and LINEAR_EXACT does not. The
            // fixture tabulates TWELVE widths and these are seven of them,
            // named because they bracket the two numbers the old derivation
            // treated as boundaries: `w = 400` gives 1, `w = 510` gives 0, and
            // 512, 600, 1000, 2048 and 4096 each give 1. The other five rows
            // are 100, 255, 256, 509 and 511, and 255 is the second and last
            // width in the table with no mover.
            //
            // **The two zeroes, at 255 and at 510, are where a half rounds up
            // and not where the integers happen to fall**, which is what this
            // comment said until the eighth pass. The movable interval below
            // is `509/510` of an input unit wide, so it holds no integer
            // exactly when its open endpoint `u_end = 254w/510` IS one, which
            // is exactly when 255 divides `w`. At those two widths the sole
            // candidate stored value sits ON `u_end`, where `y_E` is exactly
            // 254.5 and `y_L` is exactly 255, and the fixture's declared
            // rounding rule takes 254.5 up. Truncate instead and both rows
            // carry a mover. That is a property of the width and of the
            // rounding rule together, and 510 still sorts nothing: 255 is
            // below it and 510 is not.
            //
            // **In stored-value units the movable set is ONE contiguous
            // interval at every width, with no case split at all.** Write
            // `u = x - c`. `y_L` reaches 254.5 at `u = (254w - 509)/510` and
            // `y_E` reaches it at `u = 254w/510`, both solved from the two
            // formulas above, and a stored value moves exactly on
            //
            //     u in [ (254w - 509)/510 , 254w/510 )
            //
            // which is `509/510` of one input unit wide at EVERY width and
            // holds at most one integer and sometimes none. That one interval
            // covers the pixels LINEAR ROUNDED up to 255 and the pixels it
            // CLAMPED to 255 together, so 510 sorts nothing here either: it
            // moves the clamp point `u = w/2 - 1` across the interval and
            // changes neither endpoint.
            // `the_white_exclusion_is_conservative_at_every_width_and_exact_at_none`
            // and `the_movable_band_is_non_empty_at_every_width` in the
            // fixture named above pin all of that, the second by evaluating
            // both formulas at both endpoints rather than by a literal.
            //
            // An 8-bit frame does not carry the stored value behind a 255, so
            // there is no way to tell a pixel inside the band from one outside
            // it, and excluding the whole population is the conservative
            // reading at every width. It UNDER-damages the frame by whatever
            // share of the 255s fell in the band and never over-damages it, so
            // no pixel is wrongly moved.
            //
            // The accumulator's drop rate is `u / w`, which is HIGHEST at
            // `u = 255`, while 255 is the display value whose stored values
            // can move over the NARROWEST band. Excluding it is also what
            // stops the mutation perturbing the informative region, since a
            // pixel that stays at an extreme on both sides stays
            // uninformative, so the measured bias keeps the numerator and the
            // denominator the divergence actually has.
            //
            // **The residue is discarded, and the bound on it is now
            // enforced rather than asserted.** Each pixel takes
            // `accumulator / w` drops and keeps `accumulator % w`, so the
            // accumulator is below `w` after every pixel BY CONSTRUCTION, and
            // the scan applies exactly `floor(sum(u) / w)` drops and never
            // `round`. The most a frame loses to the residue is under one
            // drop, and `drops_applied` is compared against `sum(u) / w`
            // below so the delivered quantity is the declared one rather than
            // a quantity nobody counted.
            //
            // **A single `if` was wrong below `w = 255`, and the sprint
            // review's fifth pass measured it.** One subtraction per pixel
            // leaves the accumulator at or above `w` whenever `u >= 2w`, and
            // `u` runs to 254, so `acc < w` held only while every
            // participating `u` was smaller than `w`. Frame
            // `[200, 200, 200]` at `w = 100` gave `[199, 199, 199]`: three
            // drops where `floor(600 / 100)` is six, and a residue of 300.
            //
            // **A drop of more than one code is the divergence and not an
            // artefact of the accumulator.** `LINEAR(x) - LINEAR_EXACT(x)` is
            // `u / w` display codes exactly, so at `w = 100` a pixel at 200
            // diverges by two whole codes and a mutation applying one
            // under-damages the frame. Refusing narrow windows instead would
            // give the catalogue nothing to say exactly where the divergence
            // is largest, so the loop is corrected rather than the domain
            // narrowed. The corpus's narrowest window today is 256, which is
            // why nothing measured this, and a CT brain window of 80 or any
            // MR window below 255 reaches it.
            //
            // **The per-pixel drop cannot take a pixel below zero.** The
            // accumulator is under `w` before the add, so
            // `drops <= floor((w - 1 + u) / w) = floor(1 + (u - 1) / w)`,
            // which at the `w >= 2` the first statement of this arm enforces
            // is at most `1 + (u - 1) = u`. It said `w >= 1` until the eighth
            // review pass, one bound looser than the code. A participating
            // `u` is at least 1, so `grey - drops` is never negative and the
            // `saturating_sub` below never saturates.
            //
            // **`w >= 2`, not `w >= 1`.** C.11.2.1.2 needs `w >= 1`, but at
            // `w = 1` LINEAR's `w' = w - 1` is 0, which is the division that
            // section warns about, so at that one width there is no `y_L` and
            // no divergence to show rather than a small one. The fixture's
            // transcription documents the same bound for the same reason. The
            // corpus's narrowest window is 256, so nothing reaches this, which
            // is exactly why it is written down.
            let Some(window_width) = window_width.filter(|w| *w >= 2) else {
                return Err(MutationError::Apply(
                    mutation.name,
                    "this view's sidecar carries no window width of 2 or more, \
                     so the LINEAR to LINEAR_EXACT divergence is not defined \
                     for it: LINEAR divides by `w - 1`"
                        .to_owned(),
                ));
            };
            let w = u64::from(window_width);
            let mut accumulator = 0_u64;
            let mut participating_sum = 0_u64;
            let mut drops_applied = 0_u64;
            for y in image.y0..image.y0.saturating_add(image.height) {
                for x in image.x0..image.x0.saturating_add(image.width) {
                    let pixel = frame.pixel(x, y)?;
                    let Some(grey) = pixel.first() else { continue };
                    if *grey == 0 || *grey == u8::MAX {
                        continue;
                    }
                    participating_sum = participating_sum.saturating_add(u64::from(*grey));
                    accumulator = accumulator.saturating_add(u64::from(*grey));
                    let drops = accumulator / w;
                    accumulator %= w;
                    if drops > 0 {
                        let byte = grey.saturating_sub(u8::try_from(drops).unwrap_or(u8::MAX));
                        frame.set_pixel(x, y, [byte, byte, byte, u8::MAX])?;
                        drops_applied = drops_applied.saturating_add(drops);
                    }
                }
            }
            let declared = participating_sum / w;
            if drops_applied == 0 {
                return Err(MutationError::Apply(
                    mutation.name,
                    format!(
                        "the swap moved no pixel at window width {window_width}. \
                         The display values in the image rectangle that are \
                         neither 0 nor 255 sum to {participating_sum}, which is \
                         less than {window_width}, so there is under one drop's \
                         worth of divergence to show rather than a defect here"
                    ),
                ));
            }
            // **A tripwire against a future edit, not a measurement.** Until
            // the fifth pass the only thing asked of the scan was that it
            // moved SOMETHING, so one drop on a frame owing 262144 of them
            // passed, and this replaced that. What it can prove is bounded,
            // and saying so is the point: the residue telescopes, so the loop
            // above applies `floor(participating_sum / w)` drops BY
            // CONSTRUCTION and the two sides of this comparison cannot differ
            // while the loop is the loop. It fires only if a later edit breaks
            // that, which is exactly what the fifth pass's single `if` did.
            if drops_applied != declared {
                return Err(MutationError::Apply(
                    mutation.name,
                    format!(
                        "the swap applied {drops_applied} drops at window width \
                         {window_width} and the display values it accumulated \
                         sum to {participating_sum}, which declares \
                         {declared}. The accumulator and the quantity this \
                         mutation claims to apply have parted company"
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
        | Effect::SidecarStringAndFrameRefusal { .. }
        | Effect::SidecarDelta { .. }
        | Effect::SidecarArraySwap { .. }
        | Effect::SidecarDirectionSwap { .. }
        | Effect::SidecarNull { .. }
        | Effect::CorruptDeclaredDigest
        | Effect::RemoveView
        | Effect::OrphanRaw
        | Effect::RunDigest { .. } => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::error::Error;

    use super::{
        CATALOGUE, Mutation, PixelCount, Target, apply_to_frame, fraction_budget, resolve_target,
    };
    use crate::frame::{Frame, Rect};
    use crate::report::{Outcome as ViewOutcome, Side, ViewRecord};
    use crate::sidecar::ViewKind;
    use crate::tolerance::{MONOCHROME_WITHIN_ONE_LSB_FRACTION, ToleranceClass};

    type Outcome = Result<(), Box<dyn Error>>;

    /// The catalogue's own swap entry, so these tests exercise the shipped
    /// mutation and not a second copy of it.
    fn the_swap() -> Result<&'static Mutation, Box<dyn Error>> {
        CATALOGUE
            .iter()
            .find(|entry| entry.name == "the-actual-linear-exact-swap")
            .ok_or_else(|| "the-actual-linear-exact-swap is not in the catalogue".into())
    }

    /// **`apply_to_frame` had no unit test at all until the sprint review's
    /// fourth pass**, and that is how `round(u - u / w)` shipped as "the real
    /// thing" through two passes and how the missing `grey == 255` exclusion
    /// survived a third. `cargo test -p ocelli-oracle` never executed a line of
    /// the accumulator: the catalogue was exercised only by
    /// `bin/ocelli.sh compare`, which needs the rendered corpus and no GPU-less
    /// machine can run.
    ///
    /// Eight pixels, hand-computed, with a 0 and a 255 in them.
    ///
    /// **The exclusions are PS3.3's, not the accumulator's, and they are not
    /// equally tight.** `apply_to_frame` carries the derivation. At 0 the
    /// exclusion is exact at every width: the lower clamps coincide, and above
    /// them `y_E` lies in `[0, y_L]`, so `round(y_L) = 0` forces
    /// `round(y_E) = 0`. At 255 it is conservative at every width and exact at
    /// none, because a pixel LINEAR rounded or clamped to 255 moves whenever
    /// `y_L < 254.5 * w / (w - 1)`, a band that is non-empty at every width
    /// and that includes the display value 255 itself below `w = 510`.
    /// An 8-bit frame cannot tell a 255 inside the band from one outside it,
    /// so the accumulator skips the whole population, which under-damages the
    /// frame and never over-damages it.
    /// `tools/oracle/tests/voi_divergence_fixture.rs` pins the band.
    ///
    /// Values `[0, 255, 100, 200, 150, 255, 0, 90]` at `w = 400`. The two
    /// zeroes and the two 255s take no part, so the accumulator sees
    /// 100, 200, 150, 90 in that order:
    ///
    /// | pixel | u | accumulator after | drop |
    /// |-------|---|-------------------|------|
    /// | 2 | 100 | 100 | no |
    /// | 3 | 200 | 300 | no |
    /// | 4 | 150 | 450, then 50 | **yes**, 150 becomes 149 |
    /// | 7 | 90 | 140 | no |
    ///
    /// One drop, on pixel 4, and `floor(540 / 400) = 1` confirms the count.
    /// The residue of 140 is discarded, which is what makes the count a floor.
    ///
    /// Under the accumulator as it stood before this pass, the 255s were added
    /// and only their drop was blocked, so the run was
    /// `255, 355, 555 -> drop at 3, 305, 560 -> drop at 5`: two drops instead
    /// of one, on the wrong pixels, and one of them on a 255.
    #[test]
    fn the_swap_drops_by_the_accumulator_and_never_touches_a_display_extreme() -> Outcome {
        let swap = the_swap()?;
        let mut frame = Frame::from_monochrome(8, 1, &[0, 255, 100, 200, 150, 255, 0, 90])?;
        apply_to_frame(swap, &mut frame, &Rect::full(8, 1), Some(400))?;
        assert_eq!(
            frame,
            Frame::from_monochrome(8, 1, &[0, 255, 100, 200, 149, 255, 0, 90])?,
            "one drop, on the pixel where the accumulator crossed 400"
        );
        Ok(())
    }

    /// The same eight pixels at `w = 540`, which is exactly the sum of the four
    /// values that take part. The accumulator reaches 540 only on the LAST of
    /// them, so the single drop lands on pixel 7 and the residue is zero.
    ///
    /// This is the case that says the drop is placed by the accumulator rather
    /// than by position, because the same frame moves a different pixel.
    #[test]
    fn a_wider_window_moves_the_drop_to_the_pixel_that_crosses_it() -> Outcome {
        let swap = the_swap()?;
        let mut frame = Frame::from_monochrome(8, 1, &[0, 255, 100, 200, 150, 255, 0, 90])?;
        apply_to_frame(swap, &mut frame, &Rect::full(8, 1), Some(540))?;
        assert_eq!(
            frame,
            Frame::from_monochrome(8, 1, &[0, 255, 100, 200, 150, 255, 0, 89])?
        );
        Ok(())
    }

    /// One wider still. 540 is under 541, so the accumulator never crosses and
    /// the frame carries under one drop's worth of divergence. The mutation
    /// REFUSES rather than applying nothing, because a catalogue entry that
    /// quietly did less than it declared would report a guard as watched when
    /// it was not.
    ///
    /// The message names what was measured. It used to say "every pixel in the
    /// image rectangle is black", which is an inference the condition does not
    /// support: this frame has a 255 and four mid-greys in it.
    #[test]
    fn the_swap_refuses_a_frame_with_under_one_drop_in_it() -> Outcome {
        let swap = the_swap()?;
        let mut frame = Frame::from_monochrome(8, 1, &[0, 255, 100, 200, 150, 255, 0, 90])?;
        let Err(error) = apply_to_frame(swap, &mut frame, &Rect::full(8, 1), Some(541)) else {
            return Err("a frame carrying under one drop was mutated anyway".into());
        };
        let message = error.to_string();
        assert!(
            message.contains("under one drop's worth"),
            "the refusal must say what was measured: {message}"
        );
        assert!(!message.contains("black"), "and must not infer: {message}");
        Ok(())
    }

    /// **The narrow-window case, which is DEFECT 1 of the sprint review's
    /// fifth pass.** The accumulator took one drop per pixel, so its residue
    /// was bounded by `w` only while every participating `u` was smaller than
    /// `w`. `u` runs to 254, so any window under 255 broke that, and the
    /// doc comment claimed the bound held "by construction".
    ///
    /// Hand-computed at `w = 100` on `[200, 200, 200]`, per pixel:
    ///
    /// | pixel | u | accumulator after add | drops | residue | result |
    /// |-------|---|-----------------------|-------|---------|--------|
    /// | 0 | 200 | 200 | **2** | 0 | 198 |
    /// | 1 | 200 | 200 | **2** | 0 | 198 |
    /// | 2 | 200 | 200 | **2** | 0 | 198 |
    ///
    /// Six drops, and `floor(sum(u) / w) = floor(600 / 100) = 6` confirms the
    /// count. Two codes per pixel is the divergence itself and not an
    /// artefact: `LINEAR(x) - LINEAR_EXACT(x)` is `u / w` display codes, which
    /// is `200 / 100 = 2` here.
    ///
    /// Under the single `if` this replaces the frame came back
    /// `[199, 199, 199]`: three drops instead of six, and a residue of 300
    /// left in an accumulator the comment said could not exceed 100.
    #[test]
    fn a_window_narrower_than_the_display_range_takes_every_drop_it_owes() -> Outcome {
        let swap = the_swap()?;
        let mut frame = Frame::from_monochrome(3, 1, &[200, 200, 200])?;
        apply_to_frame(swap, &mut frame, &Rect::full(3, 1), Some(100))?;
        assert_eq!(
            frame,
            Frame::from_monochrome(3, 1, &[198, 198, 198])?,
            "floor(600 / 100) is six drops, two on each pixel"
        );
        Ok(())
    }

    /// The same defect one step further in, where a single pixel owes more
    /// drops than the residue of the pixels before it.
    ///
    /// `[254, 3, 3]` at `w = 5`, hand-computed:
    ///
    /// | pixel | u | accumulator after add | drops | residue | result |
    /// |-------|---|-----------------------|-------|---------|--------|
    /// | 0 | 254 | 254 | **50** | 4 | 204 |
    /// | 1 | 3 | 7 | **1** | 2 | 2 |
    /// | 2 | 3 | 5 | **1** | 0 | 2 |
    ///
    /// 52 drops, and `floor(260 / 5) = 52`. The first pixel alone owes fifty,
    /// which no per-pixel `if` can deliver, and the largest drop a pixel can
    /// owe is its own value, so 204 is nowhere near the floor of zero.
    #[test]
    fn one_pixel_can_owe_many_drops_and_takes_them_all() -> Outcome {
        let swap = the_swap()?;
        let mut frame = Frame::from_monochrome(3, 1, &[254, 3, 3])?;
        apply_to_frame(swap, &mut frame, &Rect::full(3, 1), Some(5))?;
        assert_eq!(frame, Frame::from_monochrome(3, 1, &[204, 2, 2])?);
        Ok(())
    }

    /// A frame of nothing but display extremes carries no divergence at all,
    /// whatever the window, and is refused for the same reason. Both extremes,
    /// so this fails if either exclusion is dropped.
    #[test]
    fn a_frame_of_only_extremes_carries_no_divergence() -> Outcome {
        let swap = the_swap()?;
        let mut frame = Frame::from_monochrome(4, 1, &[0, 255, 255, 0])?;
        assert!(apply_to_frame(swap, &mut frame, &Rect::full(4, 1), Some(2)).is_err());
        assert_eq!(frame, Frame::from_monochrome(4, 1, &[0, 255, 255, 0])?);
        Ok(())
    }

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

    /// A passing class-one stack record, shaped only as far as
    /// `resolve_target` reads it.
    fn stack_record(id: &str, photometric: Option<&str>) -> ViewRecord {
        ViewRecord {
            id: id.to_owned(),
            kind: ViewKind::Stack,
            class: ToleranceClass::MonochromeSixteenBit,
            outcome: ViewOutcome::Pass,
            qualifiers: BTreeSet::new(),
            side: Side::None,
            rung: "pixels",
            notes: Vec::new(),
            parameter_divergences: Vec::new(),
            parameter_values_withheld: false,
            geometry_divergences: Vec::new(),
            register_entry: None,
            reference_render_hash: "reference-hash".to_owned(),
            candidate_render_hash: "candidate-hash".to_owned(),
            statistics: None,
            monochrome_frame: true,
            photometric_interpretation: photometric.map(str::to_owned),
        }
    }

    /// The catalogue's `plus-one-on-a-twentieth-of-a-percent`, which is an
    /// `AddDelta` on the SHARED stack target, so these tests exercise a
    /// shipped entry of each kind rather than a second copy.
    fn a_delta_entry() -> Result<&'static Mutation, Box<dyn Error>> {
        CATALOGUE
            .iter()
            .find(|entry| entry.name == "plus-one-on-a-twentieth-of-a-percent")
            .ok_or_else(|| "plus-one-on-a-twentieth-of-a-percent is not in the catalogue".into())
    }

    /// **`Target::MeasuredMonochrome2Stack` must skip an inverted view, and the
    /// order of the records must not be what decides it.**
    ///
    /// `synthetic__cr_monochrome1` is a `mono16` stack view that passes, so
    /// before the target carried a photometric interpretation the only thing
    /// keeping the swap off it was `real` sorting before `synthetic` in the
    /// `BTreeSet` the runner iterates. This puts the inverted record FIRST,
    /// which is the order that accident does not survive.
    ///
    /// Under `MONOCHROME1` the rendered byte is `255 - y`, PS3.3 C.7.6.3.1.2,
    /// so the swap's drop direction reverses. The mutation would still be
    /// detected there and would still not be the divergence it declares.
    #[test]
    fn the_monochrome2_target_skips_an_inverted_view() -> Outcome {
        let swap = the_swap()?;
        let records = vec![
            stack_record("synthetic__cr_monochrome1", Some("MONOCHROME1")),
            stack_record("real__ct_cmb_mml__00000001", Some("MONOCHROME2")),
        ];
        let resolved = resolve_target(swap, &records).map_err(|error| error.to_string())?;
        assert_eq!(resolved, "real__ct_cmb_mml__00000001");
        Ok(())
    }

    /// A stack whose sidecar declares no photometric interpretation is not a
    /// target for the SWAP either, and a run holding nothing but those refuses.
    ///
    /// Refusing is the right answer rather than falling back to the first
    /// passing stack: `MutationError::NoTarget` says the corpus changed shape
    /// under the catalogue, which is a thing to look at, and a silent fallback
    /// would put the swap back on a frame whose ramp direction is unknown.
    #[test]
    fn a_stack_with_no_declared_ramp_is_not_a_monochrome2_stack() -> Outcome {
        let swap = the_swap()?;
        let records = vec![
            stack_record("synthetic__cr_monochrome1", Some("MONOCHROME1")),
            stack_record("a__view__with__no__attributes", None),
        ];
        assert!(
            resolve_target(swap, &records).is_err(),
            "an inverted view and a view with no declared ramp are not targets"
        );
        Ok(())
    }

    /// **The two targets are separate, and this is what says so.** The shared
    /// `MeasuredStack` takes the same two records the swap refuses and lands
    /// on the FIRST of them, inverted ramp and all, because a delta of one
    /// display code means the same thing on an inverted frame.
    ///
    /// Put the photometric predicate back on `MeasuredStack` and this test
    /// fails on the first record and then on `NoTarget` for the second, which
    /// is the coupling the eighth review pass named: eighteen entries narrowed
    /// by a property one of them depends on. `ColourClassTwo` is the precedent
    /// and it was created as a separate target for exactly this reason.
    #[test]
    fn the_shared_stack_target_is_not_narrowed_by_ramp_direction() -> Outcome {
        let delta = a_delta_entry()?;
        let inverted_first = vec![
            stack_record("synthetic__cr_monochrome1", Some("MONOCHROME1")),
            stack_record("real__ct_cmb_mml__00000001", Some("MONOCHROME2")),
        ];
        assert_eq!(
            resolve_target(delta, &inverted_first).map_err(|error| error.to_string())?,
            "synthetic__cr_monochrome1"
        );
        let undeclared_only = vec![stack_record("a__view__with__no__attributes", None)];
        assert_eq!(
            resolve_target(delta, &undeclared_only).map_err(|error| error.to_string())?,
            "a__view__with__no__attributes",
            "a stack whose ramp direction is undeclared still carries a delta"
        );
        Ok(())
    }

    /// And exactly one catalogue entry takes the narrowed target, so the
    /// narrowing cannot spread back over the catalogue without this going red.
    #[test]
    fn one_entry_takes_the_monochrome2_target_and_the_rest_share_the_stack() {
        let narrowed: Vec<&str> = CATALOGUE
            .iter()
            .filter(|entry| matches!(entry.target, Target::MeasuredMonochrome2Stack))
            .map(|entry| entry.name)
            .collect();
        assert_eq!(narrowed, vec!["the-actual-linear-exact-swap"]);
        let shared = CATALOGUE
            .iter()
            .filter(|entry| matches!(entry.target, Target::MeasuredStack))
            .count();
        assert_eq!(shared, 17, "the other seventeen stack entries");
    }

    /// **A window of 1 is refused, because LINEAR has no value at it.**
    /// C.11.2.1.2 puts `w' = w - 1` in the denominator, so at `w = 1` there is
    /// no `y_L` and therefore no divergence to show rather than a small one.
    /// The corpus's narrowest window is 256, so nothing reaches this, which is
    /// why it is a test and not a measurement.
    #[test]
    fn the_swap_refuses_a_window_of_one() -> Outcome {
        let swap = the_swap()?;
        let mut frame = Frame::from_monochrome(3, 1, &[200, 200, 200])?;
        let Err(error) = apply_to_frame(swap, &mut frame, &Rect::full(3, 1), Some(1)) else {
            return Err("a window of 1 was accepted, where LINEAR divides by w - 1 = 0".into());
        };
        let message = error.to_string();
        assert!(
            message.contains("2 or more"),
            "the refusal must say what it needed: {message}"
        );
        assert_eq!(
            frame,
            Frame::from_monochrome(3, 1, &[200, 200, 200])?,
            "and the frame is untouched"
        );
        Ok(())
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
        // The exact count, not a floor. It stood at `>= 18` while twenty-one
        // entries were declared, so the three newest were pinned by nothing
        // and one could have been deleted in silence. Change this number
        // deliberately when the catalogue changes, which is the point of it.
        assert_eq!(total, 29, "the declared catalogue is {total} entries");
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
