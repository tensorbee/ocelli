//! The reference-divergence register, and the attribution ladder.
//!
//! # Why a register exists at all
//!
//! Deviation D-11 makes the pinned cornerstone3D 5.8.2 "the definition of
//! correct for this project". So a place where the pin is wrong about PS3.3
//! has to be written down, or a diff measures our correct arithmetic against
//! its incorrect arithmetic and reports the difference as ours.
//!
//! `tools/oracle/reference-divergence.json` is committed and it is the one
//! mechanism in this design that can turn a failure into a non-failure. **A
//! new entry is a reviewed change with a rationale, exactly like a tolerance
//! change**, and it gets that handling for exactly that reason.
//!
//! Strict in the direction that can be true today: **an entry marked
//! `reachable: false` that fires is a run failure**, because the claim was
//! wrong. The opposite direction, an entry no row exercises failing the run,
//! is `unsupported.json`'s other half and cannot be applied while the only
//! entry is unreachable by the corpus, since it would fail every run from the
//! first one. That asymmetry is decision 12 of F-011's design round.
//!
//! # The ladder, in order
//!
//! 1. Inputs disagree, by digest. Attributed to the inputs. A refusal, taken
//!    at the run level before a single pixel is read.
//! 2. Parameters disagree. Attributed to the side that disagrees with the
//!    INDEPENDENT reading of the bytes, which the sidecar already carries:
//!    `attributes` is read straight from the file by `dicom-parser` in the
//!    page, independently of the render path.
//! 3. A register entry matches, or F-X007's own per-subject
//!    `referenceDivergence` is set. Attributed to the reference.
//! 4. Geometry is outside 25.1's bound, or the difference is confined to the
//!    letterbox. Attributed to the fit rather than to the LUT chain.
//! 5. Pixels diverge with parameters and geometry agreeing. **Attributed to
//!    ours**, by default, because section 11 makes cornerstone3D the reference
//!    and D-11 makes the pin the definition of correct until an entry in the
//!    register says otherwise. That default direction is the conservative one:
//!    the burden is on us to show the reference is wrong.

use std::collections::{BTreeMap, BTreeSet};

use serde_json::Value;
use thiserror::Error;

use crate::frame::{ChannelSet, Frame, FrameError, Rect, difference};
use crate::geometry::{
    Divergence, canvas_divergences, reformat_scale_divergences, stack_extent, world_divergences,
};
use crate::report::{
    ChannelReport, Outcome, ParameterDivergence, Qualifier, Side, ViewRecord, ViewStatistics,
};
use crate::sidecar::{LoadError, Run, Sidecar, ViewKind};
use crate::tolerance::{
    self, INFORMATIVE_FRACTION_FLOOR, ToleranceClass, ToleranceError, class_from_categories,
};

/// The fields that decide the compared pixels and the compared camera, for a
/// stack view. HLD section 11's "metadata diffed alongside pixels because a
/// wrong rescale slope can still produce a plausible image", and it runs
/// BEFORE the pixels rather than beside them, because a parameter divergence
/// explains a pixel divergence and the reverse is not true.
///
/// This is not the full metadata harness. **That is F-013, E2.5, S04.** F-011
/// diffs only what decides the compared pixels and says so.
pub const STACK_PARAMETER_FIELDS: &[&str] = &[
    "/row/sha256",
    "/voi/source",
    "/voi/windowCenter",
    "/voi/windowWidth",
    "/voi/voiLutFunction",
    "/voi/origin",
    "/image/rows",
    "/image/columns",
    "/image/color",
    "/image/slope",
    "/image/intercept",
    "/image/minPixelValue",
    "/image/maxPixelValue",
    "/image/numberOfComponents",
    "/image/dataType",
    "/attributes/rescaleSlope",
    "/attributes/rescaleIntercept",
    "/attributes/windowCenter",
    "/attributes/windowWidth",
    "/attributes/voiLutFunction",
    "/attributes/photometricInterpretation",
    "/attributes/samplesPerPixel",
    "/attributes/planarConfiguration",
    "/attributes/rows",
    "/attributes/columns",
    "/attributes/bitsAllocated",
    "/attributes/bitsStored",
    "/attributes/highBit",
    "/attributes/pixelRepresentation",
];

/// The same, for a volume reformat.
///
/// **A reformat sidecar carries no `row`, no `image`, no `attributes` and no
/// `cornerstoneMetadata`.** F-011's design plan assumed a `row` block referring
/// to the series and there is none, so what decides a reformat's pixels is
/// read from the blocks that do exist: the window resolved from one member,
/// the reformat's own parameters, and the geometry the reference built the
/// volume with.
pub const VOLUME_PARAMETER_FIELDS: &[&str] = &[
    "/volume/id",
    "/volume/seriesDirectory",
    "/voi/source",
    "/voi/windowCenter",
    "/voi/windowWidth",
    "/voi/voiLutFunction",
    "/voi/member",
    "/reformat/orientation",
    "/reformat/viewportType",
    "/reformat/blendMode",
    "/reformat/slabThicknessMm",
    "/volume/referenceGeometry/dimensions",
    "/volume/referenceGeometry/spacing",
    "/volume/referenceGeometry/origin",
    "/volume/referenceGeometry/direction",
    "/volume/referenceGeometry/dataType",
];

/// A render-path field beside the independent reading of the same value.
///
/// This is what makes rung 2 name a side rather than say "the two disagree".
/// The sidecar's `attributes` block is read straight from the bytes by
/// `dicom-parser`, independently of anything cornerstone3D resolved, and
/// `check_sidecars.py` cross-reads it a third time under pydicom. So a side
/// that resolved a field differently from the bytes is attributable with
/// nothing new.
const INDEPENDENT_READINGS: &[(&str, &str)] = &[
    ("/voi/windowCenter", "/attributes/windowCenter/0"),
    ("/voi/windowWidth", "/attributes/windowWidth/0"),
    ("/voi/voiLutFunction", "/attributes/voiLutFunction"),
    ("/image/slope", "/attributes/rescaleSlope"),
    ("/image/intercept", "/attributes/rescaleIntercept"),
    ("/image/rows", "/attributes/rows"),
    ("/image/columns", "/attributes/columns"),
    ("/image/numberOfComponents", "/attributes/samplesPerPixel"),
];

#[derive(Debug, Error)]
pub enum CompareError {
    #[error("{0}")]
    Load(#[from] LoadError),
    #[error("{0}")]
    Frame(#[from] FrameError),
    #[error("{0}")]
    Tolerance(#[from] ToleranceError),
    #[error("{0}")]
    Stats(#[from] crate::frame::StatsError),
    #[error("{0} is declared on one side and not the other")]
    Absent(String),
    #[error(
        "{id}: {side} carries a NaN at {field}. A declared parameter that is \
         not a number is refused outright rather than compared, because every \
         comparison with a NaN is false and a comparator that tested one would \
         report agreement"
    )]
    NotANumber {
        id: String,
        side: &'static str,
        field: String,
    },
    #[error(
        "{id}: {side}'s frame is not fully opaque, first at pixel {x},{y}. The \
         reference records `opaque` per frame, and a difference in alpha is a \
         difference in the canvas rather than in the image"
    )]
    NotOpaque {
        id: String,
        side: &'static str,
        x: u32,
        y: u32,
    },
    #[error(
        "{id}: the row declares `mono16` and {side}'s frame is not monochrome, \
         first at pixel {x},{y} where red, green and blue are not all equal. A \
         class-one token over a colour frame means the token is lying"
    )]
    NotMonochrome {
        id: String,
        side: &'static str,
        x: u32,
        y: u32,
    },
    #[error("{id}: the two sides resolve different tolerance classes, {a} and {b}")]
    ClassDisagreement { id: String, a: String, b: String },
    #[error("{0}")]
    Register(String),
}

/// One declared place where the pinned reference departs from PS3.3.
#[derive(Clone, Debug)]
pub struct Entry {
    pub id: String,
    pub raised_by: String,
    pub reachable: bool,
    pub reachable_why: String,
    pub citation: String,
    pub reference_does: String,
    pub standard_requires: String,
    pub pixel_effect: String,
    pub conditions: Vec<Condition>,
}

#[derive(Clone, Debug)]
pub struct Condition {
    pub pointer: String,
    pub test: Test,
}

#[derive(Clone, Debug)]
pub enum Test {
    Equals(Value),
    LessThan(f64),
    GreaterThan(f64),
}

impl Condition {
    fn holds(&self, sidecar: &Value) -> bool {
        let Some(found) = sidecar.pointer(&self.pointer) else {
            return false;
        };
        match &self.test {
            Test::Equals(want) => found == want,
            Test::LessThan(bound) => found.as_f64().is_some_and(|value| value < *bound),
            Test::GreaterThan(bound) => found.as_f64().is_some_and(|value| value > *bound),
        }
    }
}

/// The committed register.
#[derive(Clone, Debug, Default)]
pub struct Register {
    pub entries: Vec<Entry>,
}

impl Register {
    /// # Errors
    /// On a malformed entry, or on a match condition this comparator cannot
    /// evaluate. An unknown condition is refused rather than treated as
    /// unsatisfied, because a condition nobody evaluates is an entry that
    /// silently never fires.
    pub fn from_json(root: &Value) -> Result<Self, CompareError> {
        let list = root
            .pointer("/entries")
            .and_then(Value::as_array)
            .ok_or_else(|| {
                CompareError::Register(
                    "reference-divergence.json has no `entries` array".to_owned(),
                )
            })?;
        let mut entries = Vec::new();
        for raw in list {
            entries.push(Self::entry(raw)?);
        }
        Ok(Self { entries })
    }

    fn entry(raw: &Value) -> Result<Entry, CompareError> {
        let text = |field: &str| -> Result<String, CompareError> {
            raw.pointer(field)
                .and_then(Value::as_str)
                .map(str::to_owned)
                .ok_or_else(|| {
                    CompareError::Register(format!(
                        "a register entry has no {field}, and every field of an \
                         entry is load bearing: the citation is what a reviewer \
                         checks the claim against"
                    ))
                })
        };
        let reachable = raw
            .pointer("/reachable")
            .and_then(Value::as_bool)
            .ok_or_else(|| {
                CompareError::Register(
                    "a register entry has no `reachable` boolean. An entry that \
                     does not say whether the corpus can reach it cannot be \
                     checked in either direction"
                        .to_owned(),
                )
            })?;
        let conditions_raw = raw
            .pointer("/match")
            .and_then(Value::as_object)
            .ok_or_else(|| {
                CompareError::Register(
                    "a register entry has no `match` object. An entry with no \
                     match condition would fire on every view or on none, and \
                     neither is a claim"
                        .to_owned(),
                )
            })?;
        if conditions_raw.is_empty() {
            return Err(CompareError::Register(
                "a register entry's `match` object is empty".to_owned(),
            ));
        }
        let mut conditions = Vec::new();
        for (pointer, condition) in conditions_raw {
            let object = condition.as_object().ok_or_else(|| {
                CompareError::Register(format!("{pointer}: a match condition is not an object"))
            })?;
            if object.len() != 1 {
                return Err(CompareError::Register(format!(
                    "{pointer}: a match condition carries {} tests and exactly \
                     one is expected",
                    object.len()
                )));
            }
            for (name, value) in object {
                let test = match name.as_str() {
                    "equals" => Test::Equals(value.clone()),
                    "lessThan" => Test::LessThan(value.as_f64().ok_or_else(|| {
                        CompareError::Register(format!("{pointer}: lessThan wants a number"))
                    })?),
                    "greaterThan" => Test::GreaterThan(value.as_f64().ok_or_else(|| {
                        CompareError::Register(format!("{pointer}: greaterThan wants a number"))
                    })?),
                    other => {
                        return Err(CompareError::Register(format!(
                            "{pointer}: {other:?} is not a match test this \
                             comparator evaluates. An unknown test is refused \
                             rather than read as unsatisfied, because a \
                             condition nobody evaluates is an entry that never \
                             fires and nobody notices"
                        )));
                    }
                };
                conditions.push(Condition {
                    pointer: pointer.clone(),
                    test,
                });
            }
        }
        Ok(Entry {
            id: text("/id")?,
            raised_by: text("/raisedBy")?,
            reachable,
            reachable_why: text("/reachableWhy")?,
            citation: text("/citation")?,
            reference_does: text("/referenceDoes")?,
            standard_requires: text("/standardRequires")?,
            pixel_effect: text("/pixelEffect")?,
            conditions,
        })
    }

    /// The entries whose every condition holds over this sidecar.
    ///
    /// Matched against the REFERENCE side's sidecar, because the register
    /// records where the reference departs from the standard.
    #[must_use]
    pub fn matching(&self, sidecar: &Value) -> Vec<&Entry> {
        self.entries
            .iter()
            .filter(|entry| {
                entry
                    .conditions
                    .iter()
                    .all(|condition| condition.holds(sidecar))
            })
            .collect()
    }
}

/// Everything a view comparison needs that is not the two frames.
pub struct Context<'a> {
    pub reference: &'a Run,
    pub candidate: &'a Run,
    pub register: &'a Register,
    pub low_information: &'a BTreeSet<String>,
    pub downsampled: &'a BTreeSet<String>,
}

/// Two JSON values are equal for this comparator's purposes.
///
/// Numbers are compared through `f64::to_bits`, which is exact equality with
/// the intent visible, and which satisfies the denied `float_cmp` lint
/// honestly rather than by an allow. These are DICOM-declared numbers read
/// from a file, not computed results, so two readers of one `DS` value that
/// disagree is a finding and not a tolerance.
///
/// Comparing through `as_f64` also makes JSON `1` and `1.0` equal, which they
/// should be: a representation difference between two writers of the same
/// number is not a divergence in what the file declared.
///
/// **`+0.0` and `-0.0` have different bit patterns and are therefore reported
/// as a divergence.** That is deliberate. A sign flip on a rescale intercept
/// is a finding, and a producer that emits a negative zero where the other
/// emits a positive one is worth a line in a report.
fn json_equal(a: &Value, b: &Value) -> bool {
    match (a.as_f64(), b.as_f64()) {
        (Some(left), Some(right)) => left.to_bits() == right.to_bits(),
        _ => a == b,
    }
}

fn any_nan(root: &Value, pointers: &[&str]) -> Option<String> {
    for pointer in pointers {
        if let Some(value) = root.pointer(pointer)
            && value.as_f64().is_some_and(f64::is_nan)
        {
            return Some((*pointer).to_owned());
        }
    }
    None
}

/// Rung 2. Every declared field on which the two sides disagree, with a side
/// attached wherever the sidecar's independent reading can attach one.
fn parameter_divergences(
    id: &str,
    reference: &Sidecar,
    candidate: &Sidecar,
    fields: &[&str],
) -> Result<Vec<ParameterDivergence>, CompareError> {
    if let Some(field) = any_nan(&reference.json, fields) {
        return Err(CompareError::NotANumber {
            id: id.to_owned(),
            side: "the reference",
            field,
        });
    }
    if let Some(field) = any_nan(&candidate.json, fields) {
        return Err(CompareError::NotANumber {
            id: id.to_owned(),
            side: "the candidate",
            field,
        });
    }

    let mut found = Vec::new();
    for pointer in fields {
        let left = reference
            .json
            .pointer(pointer)
            .cloned()
            .unwrap_or(Value::Null);
        let right = candidate
            .json
            .pointer(pointer)
            .cloned()
            .unwrap_or(Value::Null);
        if json_equal(&left, &right) {
            continue;
        }
        let (side, why) = attribute_parameter(pointer, reference, candidate);
        found.push(ParameterDivergence {
            pointer: (*pointer).to_owned(),
            reference: left,
            candidate: right,
            side,
            why,
        });
    }
    Ok(found)
}

/// The side that disagrees with the independent reading of the bytes is the
/// one at fault. Where the field has no independent counterpart, or where
/// neither side agrees with its own, the divergence is reported unattributed
/// rather than assigned by default, because rung 5's default belongs to the
/// pixels and not here.
fn attribute_parameter(pointer: &str, reference: &Sidecar, candidate: &Sidecar) -> (Side, String) {
    let Some((_, independent)) = INDEPENDENT_READINGS
        .iter()
        .find(|(field, _)| *field == pointer)
    else {
        return (
            Side::Unattributed,
            format!(
                "{pointer} has no independent reading in the sidecar, so which \
                 side is wrong is not decidable here. The full three-way \
                 metadata harness is F-013"
            ),
        );
    };
    let reference_agrees = reference
        .json
        .pointer(pointer)
        .zip(reference.json.pointer(independent))
        .is_some_and(|(a, b)| json_equal(a, b));
    let candidate_agrees = candidate
        .json
        .pointer(pointer)
        .zip(candidate.json.pointer(independent))
        .is_some_and(|(a, b)| json_equal(a, b));
    match (reference_agrees, candidate_agrees) {
        (true, false) => (
            Side::Ours,
            format!(
                "the reference's {pointer} agrees with its own {independent}, \
                 which is read straight from the bytes by dicom-parser, and the \
                 candidate's does not"
            ),
        ),
        (false, true) => (
            Side::Reference,
            format!(
                "the candidate's {pointer} agrees with its own {independent}, \
                 read straight from the bytes, and the reference's does not"
            ),
        ),
        _ => (
            Side::Unattributed,
            format!(
                "neither side's {pointer} agrees with its own {independent}, or \
                 both do, so the independent reading does not separate them"
            ),
        ),
    }
}

/// The image rectangle the bias bullet is averaged over, and the extents the
/// canvas bound is applied to.
///
/// **A volume reformat's rectangle is the whole frame**, and that is a
/// narrowing worth stating rather than leaving to be noticed. A reformat plane
/// is a cut through a volume and has no source pixel grid to be a
/// magnification of, so `docs/lld/oracle.md` deliberately does not publish
/// `canvasPixelsPerSourcePixel` for one. Deriving a rectangle from the volume
/// bounding box would be a second copy of a derivation the reference did not
/// publish, which is the thing decision 13 exists to avoid.
pub fn image_rect_for(
    kind: ViewKind,
    sidecar: &Sidecar,
    width: u32,
    height: u32,
) -> Result<Rect, CompareError> {
    match kind {
        ViewKind::Stack => {
            let (rows, columns) = sidecar.source_grid()?;
            let (vertical, horizontal) = sidecar.canvas_pixels_per_source_pixel()?;
            Ok(
                stack_extent(columns, rows, horizontal, vertical, width, height)?
                    .rect(width, height),
            )
        }
        ViewKind::VolumeReformat => Ok(Rect::full(width, height)),
    }
}

fn rectangles(
    kind: ViewKind,
    reference: &Sidecar,
    candidate: &Sidecar,
    width: u32,
    height: u32,
) -> Result<(Rect, Vec<Divergence>), CompareError> {
    match kind {
        ViewKind::Stack => {
            let (rows, columns) = reference.source_grid()?;
            let (vertical, horizontal) = reference.canvas_pixels_per_source_pixel()?;
            let reference_extent =
                stack_extent(columns, rows, horizontal, vertical, width, height)?;
            let (candidate_rows, candidate_columns) = candidate.source_grid()?;
            let (candidate_vertical, candidate_horizontal) =
                candidate.canvas_pixels_per_source_pixel()?;
            let candidate_extent = stack_extent(
                candidate_columns,
                candidate_rows,
                candidate_horizontal,
                candidate_vertical,
                width,
                height,
            )?;
            let divergences = canvas_divergences(&reference_extent, &candidate_extent);
            Ok((reference_extent.rect(width, height), divergences))
        }
        ViewKind::VolumeReformat => {
            let reference_scale = reference.millimetres_per_canvas_pixel()?;
            let candidate_scale = candidate.millimetres_per_canvas_pixel()?;
            let divergences = reformat_scale_divergences(reference_scale, candidate_scale, height);
            Ok((Rect::full(width, height), divergences))
        }
    }
}

/// Compare one view, and place the outcome on the ladder.
///
/// # Errors
/// On a refusal: a frame that is not opaque, a `mono16` token over a frame
/// that is not monochrome, a `NaN` in a declared parameter, or a tolerance
/// class the two sides resolve differently.
pub fn compare_view(
    context: &Context<'_>,
    id: &str,
    reference_frame: &Frame,
    candidate_frame: &Frame,
) -> Result<ViewRecord, CompareError> {
    let reference = context
        .reference
        .sidecars
        .get(id)
        .ok_or_else(|| CompareError::Absent(id.to_owned()))?;
    let candidate = context
        .candidate
        .sidecars
        .get(id)
        .ok_or_else(|| CompareError::Absent(id.to_owned()))?;
    let kind = reference.kind;

    let reference_class = class_from_categories(
        context
            .reference
            .categories
            .get(id)
            .map(Vec::as_slice)
            .unwrap_or_default(),
    )?;
    let candidate_class = class_from_categories(
        context
            .candidate
            .categories
            .get(id)
            .map(Vec::as_slice)
            .unwrap_or_default(),
    )?;
    if reference_class != candidate_class {
        return Err(CompareError::ClassDisagreement {
            id: id.to_owned(),
            a: reference_class.label().to_owned(),
            b: candidate_class.label().to_owned(),
        });
    }
    let class = reference_class;

    let monochrome_frame = reference_frame.first_non_monochrome().is_none()
        && candidate_frame.first_non_monochrome().is_none();

    for (frame, side) in [
        (reference_frame, "the reference"),
        (candidate_frame, "the candidate"),
    ] {
        if let Some((x, y)) = frame.first_non_opaque() {
            return Err(CompareError::NotOpaque {
                id: id.to_owned(),
                side,
                x,
                y,
            });
        }
        if class == ToleranceClass::MonochromeSixteenBit
            && let Some((x, y)) = frame.first_non_monochrome()
        {
            return Err(CompareError::NotMonochrome {
                id: id.to_owned(),
                side,
                x,
                y,
            });
        }
    }

    let channels = match class {
        ToleranceClass::MonochromeSixteenBit => ChannelSet::Monochrome,
        ToleranceClass::ColourOrUltrasound => ChannelSet::Rgb,
    };

    let fields = match kind {
        ViewKind::Stack => STACK_PARAMETER_FIELDS,
        ViewKind::VolumeReformat => VOLUME_PARAMETER_FIELDS,
    };
    let parameters = parameter_divergences(id, reference, candidate, fields)?;

    let (rect, mut geometry) = rectangles(
        kind,
        reference,
        candidate,
        reference_frame.width(),
        reference_frame.height(),
    )?;
    geometry.extend(world_divergences(
        &reference.camera()?,
        &candidate.camera()?,
    ));

    let diff = difference(reference_frame, candidate_frame, rect, channels)?;
    let statistics = build_statistics(&diff, class)?;

    let register_entries = context.register.matching(&reference.json);
    let volume_divergence = context.reference.reference_divergence_for(id);

    let mut qualifiers: BTreeSet<Qualifier> = BTreeSet::new();
    let mut notes: Vec<String> = Vec::new();
    let mut register_entry: Option<String> = None;
    let gate_failed = !statistics.predicate_passes || !statistics.bias_passes;

    // Rung 2, then rung 3, then rung 4, then rung 5. The first that answers
    // owns the outcome.
    let (mut outcome, mut side, mut rung) = if !parameters.is_empty() {
        qualifiers.insert(Qualifier::ParameterDivergence);
        if let Some(entry) = register_entries.first() {
            qualifiers.insert(Qualifier::ReferenceDivergence);
            register_entry = Some(entry.id.clone());
            notes.push(format!(
                "the parameter divergence is explained by register entry {} \
                 ({}): {}",
                entry.id, entry.citation, entry.reference_does
            ));
            (Outcome::Unmeasured, Side::Reference, "register")
        } else {
            let attributed = parameters
                .iter()
                .map(|divergence| divergence.side)
                .find(|side| *side != Side::Unattributed)
                .unwrap_or(Side::Unattributed);
            (Outcome::Fail, attributed, "parameters")
        }
    } else if let Some(entry) = register_entries.first() {
        qualifiers.insert(Qualifier::ReferenceDivergence);
        register_entry = Some(entry.id.clone());
        notes.push(format!(
            "register entry {} ({}) matches this view: {}",
            entry.id, entry.citation, entry.reference_does
        ));
        (Outcome::Unmeasured, Side::Reference, "register")
    // **`!geometry.is_empty()` is load-bearing and was added by the S03 sprint
    // review.** F-X007's `referenceDivergence` names a FIELD, and every
    // entry that exists names a geometry field, `spacing[2]`. A divergence in
    // the through-plane spacing explains a geometry difference. It does not
    // explain an arbitrary pixel difference on a view whose geometry agrees,
    // and attributing one to the other would be the comparator excusing our
    // own defect with somebody else's.
    //
    // This was latent until the review, because no volume subject carried a
    // declared divergence and the rung could never fire. Declaring the real
    // MR series' divergence, which is the fix for the defect one rung above
    // this one, made it fire and turned a three-code single-pixel mutation
    // from `fail` into `unmeasured`. The catalogue caught it immediately,
    // which is what the catalogue is for.
    } else if let Some(measured) = volume_divergence.filter(|_| gate_failed && !geometry.is_empty())
    {
        qualifiers.insert(Qualifier::ReferenceDivergence);
        notes.push(format!(
            "F-X007 measured a reference divergence on this subject and the \
             pixels diverge with parameters agreeing, so it is attributed to \
             the reference: {}",
            measured
                .pointer("/field")
                .and_then(Value::as_str)
                .unwrap_or("unnamed field")
        ));
        (Outcome::Unmeasured, Side::Reference, "volume-divergence")
    } else if !geometry.is_empty() {
        qualifiers.insert(Qualifier::GeometryDivergence);
        (Outcome::Fail, Side::Fit, "geometry")
    } else if class == ToleranceClass::ColourOrUltrasound {
        qualifiers.insert(Qualifier::UnstatedThreshold);
        notes.push(
            "HLD 25.1 states no perceptual metric and no threshold for class \
             two, so a pass would be a claim against a bound nobody wrote. \
             Deviation D-16. The per-channel statistics are published instead"
                .to_owned(),
        );
        if monochrome_frame {
            notes.push(
                "and this frame is monochrome, so it is the known gap: HLD \
                 25.1 has no class for 8-bit monochrome at all, and \
                 docs/lld/corpus.md absorbs the one such corpus row into class \
                 two by modality. The monochrome rule obviously wants to apply \
                 to it and no written bullet says so, which is decision 4 of \
                 F-011's design round"
                    .to_owned(),
            );
        }
        (Outcome::Unmeasured, Side::None, "class-two")
    } else if gate_failed {
        let letterbox_only = diff
            .image
            .channels()
            .iter()
            .all(|stats| stats.max_abs_diff() == 0)
            && diff
                .background
                .channels()
                .iter()
                .any(|stats| stats.max_abs_diff() > 0);
        if !statistics.bias_passes {
            qualifiers.insert(Qualifier::Bias);
        }
        if letterbox_only {
            qualifiers.insert(Qualifier::LetterboxOnly);
            (Outcome::Fail, Side::Fit, "letterbox")
        } else {
            (Outcome::Fail, Side::Ours, "pixels")
        }
    } else {
        (Outcome::Pass, Side::None, "pixels")
    };

    // The two overlays. They never turn a `pass` into a `fail`, and they never
    // let a measured divergence be absorbed: a view whose gate failed and
    // whose outcome is `unmeasured` carries `divergent-while-unmeasured`,
    // which fails the run on its own.
    if context.downsampled.contains(id) {
        qualifiers.insert(Qualifier::Decimated);
        notes.push(
            "fitted DOWN into the canvas under NEAREST, so a sub-source-pixel \
             difference in the fit selects a different source pixel and the \
             pixel difference is unbounded for a reason that is not the LUT \
             chain. No written tolerance covers a decimated frame, so the \
             pixel statistics are reported and do not gate. The geometry \
             component still gates, at 25.1's written bound"
                .to_owned(),
        );
        if outcome == Outcome::Pass || rung == "pixels" || rung == "letterbox" {
            outcome = Outcome::Unmeasured;
            side = Side::None;
            rung = "decimated";
        }
    }
    if context.low_information.contains(id)
        && statistics.informative_fraction < INFORMATIVE_FRACTION_FLOOR
    {
        qualifiers.insert(Qualifier::Weak);
        notes.push(format!(
            "run.json lists this view under lowInformation and its informative \
             fraction over the image rectangle is {:.6}, below the declared \
             floor of {INFORMATIVE_FRACTION_FLOOR}. A codec error in the \
             clipped values could not show in a pixel diff",
            statistics.informative_fraction
        ));
        if outcome == Outcome::Pass || rung == "pixels" || rung == "letterbox" {
            outcome = Outcome::Unmeasured;
            side = Side::None;
            rung = "weak";
        }
    }
    if outcome == Outcome::Unmeasured && gate_failed {
        qualifiers.insert(Qualifier::DivergentWhileUnmeasured);
        notes.push(
            "the pixel predicate FAILED on a view whose outcome is unmeasured. \
             The instrument found a divergence larger than the written \
             tolerance, so reporting it as \"cannot tell\" would absorb it. \
             This fails the run on its own"
                .to_owned(),
        );
    }

    Ok(ViewRecord {
        id: id.to_owned(),
        kind,
        class,
        outcome,
        qualifiers,
        side,
        rung,
        notes,
        parameter_divergences: parameters,
        geometry_divergences: geometry,
        register_entry,
        statistics: Some(statistics),
        monochrome_frame,
    })
}

fn build_statistics(
    diff: &crate::frame::FrameDifference,
    class: ToleranceClass,
) -> Result<ViewStatistics, CompareError> {
    let region = |stats: &crate::frame::RegionStats| -> Result<Vec<ChannelReport>, CompareError> {
        stats
            .channels()
            .iter()
            .map(|channel| ChannelReport::of(channel).map_err(|error| error.into()))
            .collect::<Result<Vec<_>, CompareError>>()
    };
    // `full` and `image` are never empty on a real view and an error there is
    // a genuine refusal. The other two legitimately can be, and an empty
    // region has NO statistics rather than zero ones: the informative subset
    // is empty on a fully saturated frame, which both AXIAL synthetic
    // reformats are at `extremeFraction` 1.0, and the background is empty on
    // every volume reformat, whose image rectangle is the whole canvas.
    // Reporting an empty list says the region had nothing in it. Reporting
    // zeroes would say it was measured and agreed.
    let full = region(&diff.full)?;
    let image = region(&diff.image)?;
    let background = region(&diff.background).unwrap_or_default();
    let informative = region(&diff.informative).unwrap_or_default();

    let informative_fraction = diff.informative_fraction().unwrap_or(0.0);

    // The gate is class one's only. Class two returns `unmeasured` under
    // deviation D-16 and never reaches a predicate, so it is reported as
    // passing both bounds rather than carrying a verdict nobody stated.
    let (predicate_passes, bias_passes, signed_mean_diff) =
        if class == ToleranceClass::MonochromeSixteenBit {
            let full_channel = diff
                .full
                .channel(0)
                .ok_or_else(|| CompareError::Register("no channel 0 to gate on".to_owned()))?;
            let image_channel = diff
                .image
                .channel(0)
                .ok_or_else(|| CompareError::Register("no image channel 0".to_owned()))?;
            let predicate = tolerance::monochrome_predicate(full_channel)?;
            // **The bias is evaluated over the INFORMATIVE region, not the
            // image rectangle, and the S03 sprint review's second pass is why.**
            //
            // A pixel clipped to black or white on both sides differs by
            // nothing, whatever the underlying arithmetic says, so it cannot
            // express a divergence. Averaging over the whole rectangle divides
            // the divergence the unclipped pixels DO show by a denominator that
            // includes every pixel that structurally cannot show one.
            //
            // Measured on this corpus, by applying the catalogue's own swap to
            // every gating class-one view and reading the signed mean back
            // over each region. The per-pixel divergence between LINEAR and
            // LINEAR_EXACT is `u / w`, where `u` is the LINEAR display value,
            // so the mean over a region is `mean(u) / w`.
            //
            // Over the image rectangle the largest bias across all **70**
            // gating class-one views is **0.0853**, on
            // `real/mr_eay131/00000008.dcm`, so a 0.1 bound detects NONE of
            // them. Over the informative region the same swap gives 0.269 to
            // 0.284 on the real soft-tissue CT rows and 0.32 on the synthetic
            // ones, which is where section 18.3's worked example lives, and
            // 51 of the 70 exceed the bound.
            //
            // It said 71 views and 0.0825 until the sprint review's fourth
            // pass. 71 was `93 - 22 weak` and forgot that
            // `real/dx_varepop/00000001.dcm` is `mono16` and `unmeasured` for
            // `decimated`, so it gates nothing either. 0.0825 is a real number
            // on this corpus and it is not this one: it is what two of the
            // `real/ct_cmb_mml` rows produce over the same region.
            //
            // Reproduce with `./target/release/ocelli-compare census`.
            //
            // A view whose informative region is empty is already `weak` and
            // already `unmeasured`, so it does not gate and the bias is not
            // evaluated for it rather than being invented.
            let bias = match diff.informative.channel(0) {
                Some(informative_channel) if informative_channel.pixels() > 0 => {
                    tolerance::bias_bound(informative_channel)?
                }
                _ => tolerance::BiasVerdict {
                    signed_mean_diff: 0.0,
                    passes: true,
                },
            };
            let _ = image_channel;
            (predicate.passes, bias.passes, bias.signed_mean_diff)
        } else {
            let mean = image
                .first()
                .map_or(0.0, |channel| channel.signed_mean_diff);
            (true, true, mean)
        };

    Ok(ViewStatistics {
        channels: diff.full.channels().len(),
        full,
        image,
        background,
        informative,
        rows_touched: diff.rows_touched,
        columns_touched: diff.columns_touched,
        image_pixels: diff.image_pixels,
        informative_pixels: diff.informative_pixels,
        informative_fraction,
        predicate_passes,
        bias_passes,
        signed_mean_diff,
    })
}

/// Every register entry marked unreachable that nonetheless fired, with the
/// views it fired on. **An unreachable entry that fires is a run failure**,
/// because the claim was wrong.
#[must_use]
pub fn unreachable_entries_that_fired(
    register: &Register,
    reference: &Run,
) -> BTreeMap<String, Vec<String>> {
    let mut fired: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (id, sidecar) in &reference.sidecars {
        for entry in register.matching(&sidecar.json) {
            if !entry.reachable {
                fired.entry(entry.id.clone()).or_default().push(id.clone());
            }
        }
    }
    fired
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{Register, json_equal};

    fn register() -> Register {
        let raw = json!({
            "entries": [{
                "id": "sigmoid-width-below-one",
                "raisedBy": "F-010",
                "reachable": false,
                "reachableWhy": "no corpus row resolves SIGMOID",
                "citation": "PS3.3 C.11.2.1.3.1",
                "referenceDoes": "applies LINEAR's (w - 1) / 2 to SAMPLED_SIGMOID",
                "standardRequires": "SIGMOID divides by w and requires only w > 0",
                "pixelEffect": "an inverted range",
                "match": {
                    "/voi/voiLutFunction": { "equals": "SIGMOID" },
                    "/voi/windowWidth": { "lessThan": 1 }
                }
            }]
        });
        Register::from_json(&raw).unwrap_or_default()
    }

    /// A matching entry fires. Both conditions have to hold.
    #[test]
    fn a_matching_entry_fires() {
        let register = register();
        let sidecar = json!({
            "voi": { "voiLutFunction": "SIGMOID", "windowWidth": 0.5 }
        });
        assert_eq!(register.matching(&sidecar).len(), 1);
    }

    /// A non-matching entry does not. The corpus's own rows resolve LINEAR, so
    /// this is the case every one of the ninety-eight views takes.
    #[test]
    fn a_non_matching_entry_does_not_fire() {
        let register = register();
        let linear = json!({ "voi": { "voiLutFunction": "LINEAR", "windowWidth": 400 } });
        assert!(register.matching(&linear).is_empty());
        let wide = json!({ "voi": { "voiLutFunction": "SIGMOID", "windowWidth": 400 } });
        assert!(
            register.matching(&wide).is_empty(),
            "one condition holding is not a match"
        );
    }

    /// A match test this comparator cannot evaluate is REFUSED rather than
    /// read as unsatisfied. An entry nobody can evaluate is an entry that
    /// never fires and nobody notices.
    #[test]
    fn an_unknown_match_test_is_refused() {
        let raw = json!({
            "entries": [{
                "id": "x", "raisedBy": "F-011", "reachable": true,
                "reachableWhy": "y", "citation": "PS3.3", "referenceDoes": "z",
                "standardRequires": "w", "pixelEffect": "v",
                "match": { "/voi/source": { "matchesRegex": "^f" } }
            }]
        });
        assert!(Register::from_json(&raw).is_err());
    }

    /// An entry with no `reachable` flag is refused, because the strictness
    /// this register carries is expressed entirely through that flag.
    #[test]
    fn an_entry_without_a_reachable_flag_is_refused() {
        let raw = json!({
            "entries": [{
                "id": "x", "raisedBy": "F-011",
                "reachableWhy": "y", "citation": "PS3.3", "referenceDoes": "z",
                "standardRequires": "w", "pixelEffect": "v",
                "match": { "/voi/source": { "equals": "file" } }
            }]
        });
        assert!(Register::from_json(&raw).is_err());
    }

    /// An entry with an empty match object would fire on every view, which is
    /// not a claim.
    #[test]
    fn an_entry_with_an_empty_match_is_refused() {
        let raw = json!({
            "entries": [{
                "id": "x", "raisedBy": "F-011", "reachable": true,
                "reachableWhy": "y", "citation": "PS3.3", "referenceDoes": "z",
                "standardRequires": "w", "pixelEffect": "v",
                "match": {}
            }]
        });
        assert!(Register::from_json(&raw).is_err());
    }

    /// JSON `1` and `1.0` are the same declared number. A representation
    /// difference between two writers is not a divergence in what the file
    /// declared.
    #[test]
    fn an_integer_and_its_float_spelling_are_equal() {
        assert!(json_equal(&json!(1), &json!(1.0)));
        assert!(!json_equal(&json!(1), &json!(1.0000001)));
        assert!(json_equal(&json!("LINEAR"), &json!("LINEAR")));
        assert!(!json_equal(&json!("LINEAR"), &json!("LINEAR_EXACT")));
        assert!(json_equal(&json!(null), &json!(null)));
    }

    /// Negative zero is reported as a divergence. A sign flip on a rescale
    /// intercept is a finding.
    #[test]
    fn negative_zero_is_a_divergence() {
        assert!(!json_equal(&json!(0.0), &json!(-0.0)));
    }

    // ---- Rung 3's narrowing, watched -----------------------------------
    //
    // **These three tests are the S03 sprint review's smell S4.** The
    // `!geometry.is_empty()` narrowing on the volume-divergence rung was
    // watched by exactly one thing: the mutation
    // `plus-three-on-one-pixel-of-a-reformat`, whose `Target::MeasuredReformat`
    // resolves through a `BTreeSet` and therefore landed on the one divergent
    // subject only because `real` sorts before `synthetic`. Reverting the
    // narrowing left `cargo test -p ocelli-oracle` green, and adding one
    // synthetic subject whose identifier sorted first would have left the
    // mutation green too.
    //
    // The pair below pins the behaviour to what it means rather than to an
    // iteration order: the SAME declared divergence explains a geometry
    // difference and does not explain a pixel difference on a view whose
    // geometry agrees. Neither test can be satisfied by the corpus changing
    // shape, because both sides are built here.

    use std::collections::{BTreeMap, BTreeSet};
    use std::path::PathBuf;

    use serde_json::Value;

    use super::{CompareError, Context, compare_view};
    use crate::frame::Frame;
    use crate::report::{Outcome, Qualifier, Side};
    use crate::sidecar::{DeclaredView, Run, Sidecar, ViewKind};

    const SUBJECT: &str = "subject-under-test";
    const VIEW: &str = "subject-under-test__AXIAL";
    const SIDE_PIXELS: u32 = 16;

    /// One side of a reformat comparison, built by hand.
    ///
    /// Everything a test does not vary is identical on the two sides, so a
    /// divergence in the record can only have come from what the test changed.
    fn reformat_run(parallel_scale: f64, declares_divergence: bool) -> Run {
        let divergence = if declares_divergence {
            json!({
                "field": "spacing[2]",
                "reference": 10.0,
                "truth": null,
                "attributedTo": "reference",
                "why": "the reference resolves one spacing for a series whose gaps differ"
            })
        } else {
            Value::Null
        };
        let sidecar = Sidecar {
            id: VIEW.to_owned(),
            kind: ViewKind::VolumeReformat,
            json: json!({
                "kind": "volume-reformat",
                "camera": {
                    "position": [0.0, 0.0, 100.0],
                    "focalPoint": [0.0, 0.0, 0.0],
                    "viewUp": [0.0, 1.0, 0.0],
                    "viewPlaneNormal": [0.0, 0.0, 1.0],
                    "parallelScale": parallel_scale
                },
                "reformat": { "millimetresPerCanvasPixel": 0.5 },
                "frame": { "width": SIDE_PIXELS, "height": SIDE_PIXELS }
            }),
        };
        let mut views = BTreeMap::new();
        views.insert(
            VIEW.to_owned(),
            DeclaredView {
                id: VIEW.to_owned(),
                kind: ViewKind::VolumeReformat,
                path: None,
                subject: Some(SUBJECT.to_owned()),
            },
        );
        let mut sidecars = BTreeMap::new();
        sidecars.insert(VIEW.to_owned(), sidecar);
        let mut categories = BTreeMap::new();
        categories.insert(VIEW.to_owned(), vec!["mono16".to_owned()]);
        Run {
            directory: PathBuf::from("built-in-memory"),
            json: json!({
                "volumes": [{ "id": SUBJECT, "referenceDivergence": divergence }]
            }),
            views,
            sidecars,
            raw_present: BTreeSet::new(),
            by_path: BTreeMap::new(),
            categories,
        }
    }

    /// The reformat pair, with one pixel three codes brighter on the
    /// candidate.
    ///
    /// Three codes is 25.1's "zero pixels differing by more than 2", so the
    /// predicate fails at any count and `gate_failed` is true. One pixel in
    /// 256 at three codes is a signed mean of 0.0117, well inside the 0.1 bias
    /// bound, so the bias is not what fails and `Qualifier::Bias` stays off.
    fn compare_reformat(
        candidate_parallel_scale: f64,
        declares_divergence: bool,
    ) -> Result<crate::report::ViewRecord, CompareError> {
        let reference_run = reformat_run(100.0, declares_divergence);
        let candidate_run = reformat_run(candidate_parallel_scale, declares_divergence);
        let register = Register::default();
        let empty: BTreeSet<String> = BTreeSet::new();
        let context = Context {
            reference: &reference_run,
            candidate: &candidate_run,
            register: &register,
            low_information: &empty,
            downsampled: &empty,
        };
        let mut greys = [100_u8; 256];
        let reference_frame = Frame::from_monochrome(SIDE_PIXELS, SIDE_PIXELS, &greys)?;
        if let Some(first) = greys.first_mut() {
            *first = 103;
        }
        let candidate_frame = Frame::from_monochrome(SIDE_PIXELS, SIDE_PIXELS, &greys)?;
        compare_view(&context, VIEW, &reference_frame, &candidate_frame)
    }

    /// **The narrowing.** A declared through-plane spacing divergence does not
    /// explain a pixel difference on a view whose geometry agrees, so the
    /// ladder falls through rung 3 to rung 5 and attributes it to us.
    ///
    /// Remove `&& !geometry.is_empty()` from the rung and this test reports
    /// `unmeasured` attributed to the reference, which is the comparator
    /// excusing our own defect with somebody else's.
    #[test]
    fn a_declared_divergence_does_not_excuse_a_pixel_difference_when_geometry_agrees()
    -> Result<(), CompareError> {
        let record = compare_reformat(100.0, true)?;
        assert!(
            record.geometry_divergences.is_empty(),
            "the two sides carry the same camera and the same scale, so \
             nothing should have been measured: {:?}",
            record.geometry_divergences
        );
        assert_eq!(record.rung, "pixels");
        assert_eq!(record.outcome, Outcome::Fail);
        assert_eq!(record.side, Side::Ours);
        assert!(!record.qualifiers.contains(&Qualifier::ReferenceDivergence));
        Ok(())
    }

    /// The same declared divergence, on a view whose geometry ALSO diverges.
    /// Here it is an explanation, rung 3 answers, and the outcome is
    /// `unmeasured` attributed to the reference.
    ///
    /// Without this half, the test above would be satisfied by deleting the
    /// rung altogether.
    #[test]
    fn the_same_divergence_does_explain_a_geometry_difference() -> Result<(), CompareError> {
        let record = compare_reformat(100.001, true)?;
        assert!(!record.geometry_divergences.is_empty());
        assert_eq!(record.rung, "volume-divergence");
        assert_eq!(record.outcome, Outcome::Unmeasured);
        assert_eq!(record.side, Side::Reference);
        assert!(record.qualifiers.contains(&Qualifier::ReferenceDivergence));
        assert!(
            record
                .qualifiers
                .contains(&Qualifier::DivergentWhileUnmeasured),
            "a gate failure absorbed into `unmeasured` still fails the run"
        );
        Ok(())
    }

    /// The same geometry difference with NO declared divergence is rung 4,
    /// attributed to the fit. This is what says the two tests above are about
    /// the divergence and not about the geometry.
    #[test]
    fn a_geometry_difference_with_no_declared_divergence_is_the_fit() -> Result<(), CompareError> {
        let record = compare_reformat(100.001, false)?;
        assert_eq!(record.rung, "geometry");
        assert_eq!(record.outcome, Outcome::Fail);
        assert_eq!(record.side, Side::Fit);
        assert!(record.qualifiers.contains(&Qualifier::GeometryDivergence));
        Ok(())
    }
}
