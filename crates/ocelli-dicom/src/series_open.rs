//! Display order for a bag of frames.
//!
//! DICOM PS3.3 C.7.6.2.1.1 projects Image Position Patient onto the slice
//! normal. `Volume::assemble` and `StackGeometry::derive` refuse a mixed
//! orientation, a duplicate position, non-uniform spacing and shear. This
//! module orders frames a person can scroll and reports those conditions.
//! It does not build a volume and it does not change those refusals.
//!
//! Cine timing follows the cine module. The first present valid source wins.
//! An absent source is skipped. A non-finite, zero or negative interval is an
//! error and does not fall through. The first Frame Time Vector component may
//! be zero, because that component is not an interval. No default frame rate
//! is invented.

use std::cmp::Ordering;
use std::collections::HashMap;
use std::sync::Arc;

use dicom_core::{Tag, VR};
use glam::DVec3;
use ocelli_pixel::{ImageOrientationPatient, PixelSpacing};

use super::{
    FrameGeometry, FrameGeometryError, MetadataElement, MetadataSet, MetadataValue,
    MultiframeError, MultiframeMetadata, ParsedDicom, TopLevelFallback, parse_decimal_string,
};
use crate::frame_geometry::{
    cosines_agree, exceeds_geometry_tolerance, positions_coincide, spacing_agrees,
};
use crate::frame_open::project_metadata;

const SERIES_INSTANCE_UID: Tag = Tag(0x0020, 0x000E);
const FRAME_OF_REFERENCE_UID: Tag = Tag(0x0020, 0x0052);
const SOP_INSTANCE_UID: Tag = Tag(0x0008, 0x0018);
const INSTANCE_NUMBER: Tag = Tag(0x0020, 0x0013);
const FRAME_CONTENT_SEQUENCE: Tag = Tag(0x0020, 0x9111);
const STACK_ID: Tag = Tag(0x0020, 0x9056);
const IN_STACK_POSITION_NUMBER: Tag = Tag(0x0020, 0x9057);
const TEMPORAL_POSITION_INDEX: Tag = Tag(0x0020, 0x9128);
const FRAME_TIME_VECTOR: Tag = Tag(0x0018, 0x1065);
const FRAME_TIME: Tag = Tag(0x0018, 0x1063);
const RECOMMENDED_DISPLAY_FRAME_RATE: Tag = Tag(0x0008, 0x2144);
const CINE_RATE: Tag = Tag(0x0018, 0x0040);
const PREFERRED_PLAYBACK_SEQUENCING: Tag = Tag(0x0018, 0x1244);
const IMAGE_ORIENTATION_PATIENT: Tag = Tag(0x0020, 0x0037);
const PLANE_ORIENTATION_SEQUENCE: Tag = Tag(0x0020, 0x9116);
const PIXEL_SPACING: Tag = Tag(0x0028, 0x0030);
const PIXEL_MEASURES_SEQUENCE: Tag = Tag(0x0028, 0x9110);

/// One already parsed object and the frame index inside it.
///
/// The value does not store a path. Debug output names only the frame index.
#[derive(Clone, Copy)]
pub struct ParsedFrame<'a> {
    parsed: &'a ParsedDicom,
    frame_index: u32,
}

impl<'a> ParsedFrame<'a> {
    /// Borrow one parsed object at one zero-based frame.
    #[must_use]
    pub const fn new(parsed: &'a ParsedDicom, frame_index: u32) -> Self {
        Self {
            parsed,
            frame_index,
        }
    }

    /// The zero-based frame this entry names.
    #[must_use]
    pub const fn frame_index(self) -> u32 {
        self.frame_index
    }
}

impl std::fmt::Debug for ParsedFrame<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ParsedFrame")
            .field("frame_index", &self.frame_index)
            .finish()
    }
}

/// Where one input frame landed in display order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FramePlace {
    input_index: u32,
    frame_index: u32,
}

impl FramePlace {
    /// Index into the slice passed to [`assemble_display`].
    #[must_use]
    pub const fn input_index(self) -> u32 {
        self.input_index
    }

    /// Zero-based frame inside that input's object.
    #[must_use]
    pub const fn frame_index(self) -> u32 {
        self.frame_index
    }
}

/// Which key ordered a display group.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SpatialOrder {
    /// At least one frame had Image Position Patient.
    Projected,
    /// No frame in the group had Image Position Patient.
    InstanceNumber,
}

/// Preferred Playback Sequencing `(0018,1244)`.
///
/// PS3.6 defines the attribute as US. PS3.3 C.7.6.5 value 0 is looping and
/// value 1 is sweeping. `Sweep` is that sweeping value. A page may label it
/// Bounce.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Playback {
    /// Value 0, and the value used when the attribute is absent.
    Loop,
    /// Value 1.
    Sweep,
}

/// The first valid cine timing source, or the absence of every source.
#[derive(Debug, Clone, Copy, PartialEq)]
#[non_exhaustive]
pub enum CineTiming {
    /// No timing attribute was present.
    Unavailable,
    /// Frame Time Vector `(0018,1065)`.
    ///
    /// PS3.3 C.7.6.5.1.2 says the first component is zero. Each later component
    /// is a positive interval in milliseconds.
    FrameTimeVector,
    /// Frame Time `(0018,1063)`, one interval in milliseconds.
    FrameTime {
        /// The interval in milliseconds.
        milliseconds: f64,
    },
    /// Recommended Display Frame Rate `(0008,2144)`.
    RecommendedDisplayFrameRate {
        /// Frames per second.
        frames_per_second: f64,
    },
    /// Cine Rate `(0018,0040)`.
    CineRate {
        /// Frames per second.
        frames_per_second: f64,
    },
}

/// Why display assembly stopped.
///
/// The index is the caller's input index. The error does not carry a UID,
/// a path, or a sample.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
#[error("series assembly failed")]
pub struct SeriesOpenError {
    reason: SeriesOpenReason,
    index: u32,
}

impl SeriesOpenError {
    /// The closed reason.
    #[must_use]
    pub const fn reason(self) -> SeriesOpenReason {
        self.reason
    }

    /// The caller's input index.
    #[must_use]
    pub const fn index(self) -> u32 {
        self.index
    }

    /// Build an error from a closed reason and the caller's input index.
    #[must_use]
    pub const fn new(reason: SeriesOpenReason, index: u32) -> Self {
        Self { reason, index }
    }

    /// Replace the input index. The reason stays.
    #[must_use]
    pub const fn with_index(self, index: u32) -> Self {
        Self {
            reason: self.reason,
            index,
        }
    }
}

/// Closed reasons for [`SeriesOpenError`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SeriesOpenReason {
    /// The frame has a plane that geometry refused, other than a missing position.
    Geometry,
    /// The frame index is outside the declared frame count.
    FrameOutOfRange,
    /// A present cine timing source is not usable.
    Cine,
    /// Metadata could not be read.
    Metadata,
    /// Preferred Playback Sequencing is present with another value representation, or with a value other than 0 or 1.
    Playback,
}

/// One scrollable group.
///
/// Fields stay private so a UID used while grouping cannot leak through Debug.
#[derive(Debug, Clone, PartialEq)]
pub struct DisplaySeries {
    order: SpatialOrder,
    playback: Playback,
    timing: CineTiming,
    slots: Vec<Vec<FramePlace>>,
    sheared: bool,
    non_uniform_step: bool,
}

impl DisplaySeries {
    /// `Projected` when any frame has a position, otherwise `InstanceNumber`.
    #[must_use]
    pub const fn order(&self) -> SpatialOrder {
        self.order
    }

    /// Loop or Sweep from the first frame in display order. US 0 or US 1.
    #[must_use]
    pub const fn playback(&self) -> Playback {
        self.playback
    }

    /// Cine timing from the first frame in display order.
    #[must_use]
    pub const fn timing(&self) -> CineTiming {
        self.timing
    }

    /// Frames in this group, including every frame of a repeated position.
    #[must_use]
    pub fn frame_count(&self) -> usize {
        self.slots.iter().map(Vec::len).sum()
    }

    /// Spatial slots in display order.
    ///
    /// Frames that share a projected position stay in one slot. A slot length
    /// above one is a duplicate the volume builder would drop.
    #[must_use]
    pub fn slots(&self) -> &[Vec<FramePlace>] {
        &self.slots
    }

    /// An inter-slot step is not parallel to the first positioned frame's normal.
    #[must_use]
    pub const fn sheared(&self) -> bool {
        self.sheared
    }

    /// Through-plane steps differ by more than the geometry tolerance.
    #[must_use]
    pub const fn non_uniform_step(&self) -> bool {
        self.non_uniform_step
    }
}

/// How many frames [`MultiframeMetadata`] declares for one object.
///
/// The index on an error is zero. A caller that expands several objects can
/// replace it with the object's own index. Pixel Data is not copied.
///
/// # Errors
///
/// Returns [`SeriesOpenReason::Metadata`] when the frame declaration or the
/// functional-group sequences are malformed.
pub fn declared_frame_count(parsed: &ParsedDicom) -> Result<u32, SeriesOpenError> {
    let metadata = project(parsed, 0)?;
    let multi = MultiframeMetadata::new(&metadata).map_err(|error| map_multi(error, 0))?;
    u32::try_from(multi.frame_count())
        .map_err(|_| SeriesOpenError::new(SeriesOpenReason::Metadata, 0))
}

/// Order `frames` into display groups.
///
/// Grouping uses the Series Instance UID, the Frame of Reference UID, row and
/// column cosines, and in-plane spacing. A missing UID does not join a present
/// one. Frames of one parsed object share one missing UID and one metadata
/// projection. Two parsed objects that both omit a UID do not join each other.
/// A missing Image Position
/// Patient is unpositioned. A malformed cosine or spacing on that frame is
/// still an error. Any other geometry failure is an error.
///
/// # Errors
///
/// Returns [`SeriesOpenError`] with the input index of the frame that failed.
pub fn assemble_display(frames: &[ParsedFrame<'_>]) -> Result<Vec<DisplaySeries>, SeriesOpenError> {
    let mut prepared = Vec::with_capacity(frames.len());
    let mut shared = HashMap::new();
    for (index, frame) in frames.iter().enumerate() {
        let input_index = u32::try_from(index)
            .map_err(|_| SeriesOpenError::new(SeriesOpenReason::Metadata, 0))?;
        prepared.push(prepare(frame, input_index, &mut shared)?);
    }
    let groups = group_frames(prepared);
    let mut display = Vec::with_capacity(groups.len());
    for group in groups {
        display.push(finish(group)?);
    }
    Ok(display)
}

#[derive(Clone)]
enum UidIdentity {
    Present(String),
    /// Address of the parsed object. Frames of that object share one absence.
    Absent(usize),
}

fn same_uid(left: &UidIdentity, right: &UidIdentity) -> bool {
    match (left, right) {
        (UidIdentity::Present(left), UidIdentity::Present(right)) => left == right,
        (UidIdentity::Absent(left), UidIdentity::Absent(right)) => left == right,
        _ => false,
    }
}

struct Prepared {
    input_index: u32,
    frame_index: u32,
    series: UidIdentity,
    frame_of_reference: UidIdentity,
    sop: Vec<u8>,
    instance: Option<i64>,
    stack_id: Option<String>,
    in_stack: Option<u32>,
    temporal: Option<u32>,
    projection: Option<f64>,
    position: Option<DVec3>,
    row: Option<DVec3>,
    column: Option<DVec3>,
    spacing: Option<PixelSpacing>,
    own_normal: Option<DVec3>,
    metadata: Arc<MetadataSet>,
}

#[derive(Clone, Copy)]
struct Anchor {
    row: DVec3,
    column: DVec3,
    spacing: PixelSpacing,
    normal: DVec3,
}

struct GroupBuild {
    series: UidIdentity,
    frame_of_reference: UidIdentity,
    anchor: Option<Anchor>,
    positioned: Vec<Prepared>,
    unpositioned: Vec<Prepared>,
}

struct PlaneBits {
    row: DVec3,
    column: DVec3,
    spacing: PixelSpacing,
    normal: DVec3,
    position: DVec3,
}

fn plane_bits(frame: &Prepared) -> Option<PlaneBits> {
    Some(PlaneBits {
        row: frame.row?,
        column: frame.column?,
        spacing: frame.spacing?,
        normal: frame.own_normal?,
        position: frame.position?,
    })
}

fn project(parsed: &ParsedDicom, index: u32) -> Result<MetadataSet, SeriesOpenError> {
    // `from_object` copies an explicit OW payload and refuses an encapsulated
    // pixel sequence. Frame open already skips those elements. Reuse it.
    #[cfg(test)]
    PROJECTIONS.with(|count| count.set(count.get().saturating_add(1)));
    project_metadata(parsed).map_err(|_| SeriesOpenError::new(SeriesOpenReason::Metadata, index))
}

fn shared_metadata(
    parsed: &ParsedDicom,
    index: u32,
    shared: &mut HashMap<usize, Arc<MetadataSet>>,
) -> Result<Arc<MetadataSet>, SeriesOpenError> {
    let key = object_key(parsed);
    if let Some(existing) = shared.get(&key) {
        return Ok(Arc::clone(existing));
    }
    let projected = Arc::new(project(parsed, index)?);
    shared.insert(key, Arc::clone(&projected));
    Ok(projected)
}

fn prepare(
    frame: &ParsedFrame<'_>,
    input_index: u32,
    shared: &mut HashMap<usize, Arc<MetadataSet>>,
) -> Result<Prepared, SeriesOpenError> {
    let metadata = shared_metadata(frame.parsed, input_index, shared)?;
    let multi =
        MultiframeMetadata::new(&metadata).map_err(|error| map_multi(error, input_index))?;
    let frame_usize = usize::try_from(frame.frame_index)
        .map_err(|_| SeriesOpenError::new(SeriesOpenReason::FrameOutOfRange, input_index))?;
    if frame_usize >= multi.frame_count() {
        return Err(SeriesOpenError::new(
            SeriesOpenReason::FrameOutOfRange,
            input_index,
        ));
    }
    let series = uid_identity(&metadata, SERIES_INSTANCE_UID, frame.parsed, input_index)?;
    let frame_of_reference =
        uid_identity(&metadata, FRAME_OF_REFERENCE_UID, frame.parsed, input_index)?;
    let sop = sop_bytes(&metadata, input_index)?;
    let instance = instance_number(&metadata, input_index)?;
    let stack_id = resolved_text(&multi, frame_usize, STACK_ID, VR::SH, input_index)?;
    let in_stack = resolved_ul(&multi, frame_usize, IN_STACK_POSITION_NUMBER, input_index)?;
    let temporal = resolved_ul(&multi, frame_usize, TEMPORAL_POSITION_INDEX, input_index)?;
    let geometry = match FrameGeometry::derive(&multi, frame_usize) {
        Ok(geometry) => Some(geometry),
        Err(FrameGeometryError::MissingImagePosition) => {
            reject_malformed_unpositioned(&multi, frame_usize, input_index)?;
            None
        }
        Err(error) => return Err(map_geometry(error, input_index)),
    };
    let mut prepared = Prepared {
        input_index,
        frame_index: frame.frame_index,
        series,
        frame_of_reference,
        sop,
        instance,
        stack_id,
        in_stack,
        temporal,
        projection: None,
        position: None,
        row: None,
        column: None,
        spacing: None,
        own_normal: None,
        metadata,
    };
    if let Some(geometry) = geometry.as_ref() {
        adopt_plane(&mut prepared, geometry);
    }
    Ok(prepared)
}

fn adopt_plane(prepared: &mut Prepared, geometry: &FrameGeometry) {
    let plane = geometry.plane();
    let position = plane.position_vector();
    let normal = plane.slice_normal();
    prepared.position = Some(position);
    prepared.own_normal = Some(normal);
    prepared.row = Some(plane.orientation.row());
    prepared.column = Some(plane.orientation.column());
    prepared.spacing = Some(geometry.spacing().image_mm());
    prepared.projection = Some(position.dot(normal));
}

fn reject_malformed_unpositioned(
    multi: &MultiframeMetadata<'_>,
    frame: usize,
    index: u32,
) -> Result<(), SeriesOpenError> {
    if let Some(values) = plane_decimals(
        multi,
        frame,
        PLANE_ORIENTATION_SEQUENCE,
        IMAGE_ORIENTATION_PATIENT,
        6,
        index,
    )? {
        let mut row = [0.0_f64; 3];
        let mut column = [0.0_f64; 3];
        for (slot, number) in row.iter_mut().chain(column.iter_mut()).zip(values) {
            *slot = number;
        }
        ImageOrientationPatient::new(row, column)
            .map_err(|_| SeriesOpenError::new(SeriesOpenReason::Geometry, index))?;
    }
    if let Some(values) = plane_decimals(
        multi,
        frame,
        PIXEL_MEASURES_SEQUENCE,
        PIXEL_SPACING,
        2,
        index,
    )? {
        let mut pair = [0.0_f64; 2];
        for (slot, number) in pair.iter_mut().zip(values) {
            *slot = number;
        }
        let [row_mm, column_mm] = pair;
        PixelSpacing::new(row_mm, column_mm)
            .map_err(|_| SeriesOpenError::new(SeriesOpenReason::Geometry, index))?;
    }
    Ok(())
}

fn plane_decimals(
    multi: &MultiframeMetadata<'_>,
    frame: usize,
    group: Tag,
    attribute: Tag,
    count: usize,
    index: u32,
) -> Result<Option<Vec<f64>>, SeriesOpenError> {
    let resolved = multi
        .resolve(frame, group, attribute, TopLevelFallback::Allowed)
        .map_err(|error| map_multi(error, index))?;
    let Some(resolved) = resolved else {
        return Ok(None);
    };
    let element = resolved.element();
    if element.vr() != VR::DS {
        return Err(SeriesOpenError::new(SeriesOpenReason::Geometry, index));
    }
    let MetadataValue::Text(values) = element.value() else {
        return Err(SeriesOpenError::new(SeriesOpenReason::Geometry, index));
    };
    if values.len() != count {
        return Err(SeriesOpenError::new(SeriesOpenReason::Geometry, index));
    }
    let mut parsed = Vec::with_capacity(count);
    for raw in values {
        let Some(number) = parse_decimal_string(raw) else {
            return Err(SeriesOpenError::new(SeriesOpenReason::Geometry, index));
        };
        parsed.push(number);
    }
    Ok(Some(parsed))
}

fn map_geometry(error: FrameGeometryError, index: u32) -> SeriesOpenError {
    match error {
        FrameGeometryError::Multiframe(inner) => map_multi(inner, index),
        _ => SeriesOpenError::new(SeriesOpenReason::Geometry, index),
    }
}

fn map_multi(error: MultiframeError, index: u32) -> SeriesOpenError {
    match error {
        MultiframeError::FrameOutOfRange { .. } => {
            SeriesOpenError::new(SeriesOpenReason::FrameOutOfRange, index)
        }
        _ => SeriesOpenError::new(SeriesOpenReason::Metadata, index),
    }
}

fn group_frames(frames: Vec<Prepared>) -> Vec<GroupBuild> {
    let mut groups = Vec::new();
    let mut waiting = Vec::new();
    for frame in frames {
        if plane_bits(&frame).is_some() {
            place_positioned(&mut groups, frame);
        } else {
            waiting.push(frame);
        }
    }
    for frame in waiting {
        place_unpositioned(&mut groups, frame);
    }
    groups
}

fn place_positioned(groups: &mut Vec<GroupBuild>, mut frame: Prepared) {
    let Some(bits) = plane_bits(&frame) else {
        place_unpositioned(groups, frame);
        return;
    };
    let found = groups.iter().position(|group| {
        group.anchor.is_some()
            && same_uid(&group.series, &frame.series)
            && same_uid(&group.frame_of_reference, &frame.frame_of_reference)
            && orientations_agree(group, &frame)
            && spacing_matches(group, &frame)
    });
    if let Some(index) = found {
        if let Some(group) = groups.get_mut(index) {
            if let Some(anchor) = group.anchor {
                frame.projection = Some(bits.position.dot(anchor.normal));
            }
            group.positioned.push(frame);
        }
        return;
    }
    frame.projection = Some(bits.position.dot(bits.normal));
    groups.push(GroupBuild {
        series: frame.series.clone(),
        frame_of_reference: frame.frame_of_reference.clone(),
        anchor: Some(Anchor {
            row: bits.row,
            column: bits.column,
            spacing: bits.spacing,
            normal: bits.normal,
        }),
        positioned: vec![frame],
        unpositioned: Vec::new(),
    });
}

fn orientations_agree(group: &GroupBuild, frame: &Prepared) -> bool {
    let Some(anchor) = group.anchor else {
        return false;
    };
    let Some(row) = frame.row else {
        return false;
    };
    let Some(column) = frame.column else {
        return false;
    };
    cosines_agree(row, anchor.row) && cosines_agree(column, anchor.column)
}

fn spacing_matches(group: &GroupBuild, frame: &Prepared) -> bool {
    match (group.anchor, frame.spacing) {
        (Some(anchor), Some(spacing)) => spacing_agrees(spacing, anchor.spacing),
        _ => false,
    }
}

fn place_unpositioned(groups: &mut Vec<GroupBuild>, frame: Prepared) {
    let sharing: Vec<usize> = groups
        .iter()
        .enumerate()
        .filter_map(|(index, group)| {
            if group.anchor.is_some()
                && same_uid(&group.series, &frame.series)
                && same_uid(&group.frame_of_reference, &frame.frame_of_reference)
            {
                Some(index)
            } else {
                None
            }
        })
        .collect();
    if sharing.len() == 1 {
        if let Some(index) = sharing.first().copied()
            && let Some(group) = groups.get_mut(index)
        {
            group.unpositioned.push(frame);
        }
        return;
    }
    let companion = groups.iter().position(|group| {
        group.anchor.is_none()
            && same_uid(&group.series, &frame.series)
            && same_uid(&group.frame_of_reference, &frame.frame_of_reference)
    });
    if let Some(index) = companion {
        if let Some(group) = groups.get_mut(index) {
            group.unpositioned.push(frame);
        }
        return;
    }
    groups.push(GroupBuild {
        series: frame.series.clone(),
        frame_of_reference: frame.frame_of_reference.clone(),
        anchor: None,
        positioned: Vec::new(),
        unpositioned: vec![frame],
    });
}

fn finish(group: GroupBuild) -> Result<DisplaySeries, SeriesOpenError> {
    let GroupBuild {
        anchor,
        positioned,
        unpositioned,
        ..
    } = group;
    let order = if positioned.is_empty() {
        SpatialOrder::InstanceNumber
    } else {
        SpatialOrder::Projected
    };
    let normal = anchor.map(|anchor| anchor.normal);
    let slots = into_slots(positioned);
    let mut unpositioned = unpositioned;
    unpositioned.sort_by(cmp_unpositioned);
    let trailing: Vec<Vec<Prepared>> = unpositioned.into_iter().map(|frame| vec![frame]).collect();
    let frame_count = slots.iter().map(Vec::len).sum::<usize>() + trailing.len();
    let (timing, playback) = {
        let first = first_frame(&slots, &trailing)?;
        read_cine(&first.metadata, frame_count, first.input_index)?
    };
    let (sheared, non_uniform_step) = match normal {
        Some(normal) => report_steps(&slot_positions(&slots), normal),
        None => (false, false),
    };
    Ok(DisplaySeries {
        order,
        playback,
        timing,
        slots: slots_to_places(slots, trailing),
        sheared,
        non_uniform_step,
    })
}

fn into_slots(mut frames: Vec<Prepared>) -> Vec<Vec<Prepared>> {
    frames.sort_by(cmp_projection);
    let mut slots: Vec<Vec<Prepared>> = Vec::new();
    for frame in frames {
        if starts_new_slot(&slots, &frame) {
            slots.push(vec![frame]);
        } else if let Some(slot) = slots.last_mut() {
            slot.push(frame);
        }
    }
    for slot in &mut slots {
        slot.sort_by(cmp_within_slot);
    }
    slots
}

fn starts_new_slot(slots: &[Vec<Prepared>], frame: &Prepared) -> bool {
    let Some(anchor) = slots.last().and_then(|slot| slot.first()) else {
        return true;
    };
    match (anchor.projection, frame.projection) {
        (Some(anchor_mm), Some(frame_mm)) => !positions_coincide(frame_mm, anchor_mm),
        _ => true,
    }
}

fn first_frame<'a>(
    slots: &'a [Vec<Prepared>],
    trailing: &'a [Vec<Prepared>],
) -> Result<&'a Prepared, SeriesOpenError> {
    if let Some(frame) = slots.first().and_then(|slot| slot.first()) {
        return Ok(frame);
    }
    if let Some(frame) = trailing.first().and_then(|slot| slot.first()) {
        return Ok(frame);
    }
    Err(SeriesOpenError::new(SeriesOpenReason::Metadata, 0))
}

fn slot_positions(slots: &[Vec<Prepared>]) -> Vec<DVec3> {
    let mut positions = Vec::new();
    for slot in slots {
        if let Some(position) = slot.first().and_then(|frame| frame.position) {
            positions.push(position);
        }
    }
    positions
}

fn slots_to_places(
    slots: Vec<Vec<Prepared>>,
    trailing: Vec<Vec<Prepared>>,
) -> Vec<Vec<FramePlace>> {
    let mut places = Vec::with_capacity(slots.len().saturating_add(trailing.len()));
    for slot in slots.into_iter().chain(trailing) {
        places.push(slot.into_iter().map(place_of).collect());
    }
    places
}

fn place_of(frame: Prepared) -> FramePlace {
    FramePlace {
        input_index: frame.input_index,
        frame_index: frame.frame_index,
    }
}

fn report_steps(positions: &[DVec3], normal: DVec3) -> (bool, bool) {
    let mut sheared = false;
    let mut non_uniform = false;
    let mut previous: Option<DVec3> = None;
    let mut first_step: Option<f64> = None;
    for position in positions {
        let Some(prior) = previous else {
            previous = Some(*position);
            continue;
        };
        let step = *position - prior;
        let projected = step.dot(normal);
        let offaxis = (step - normal * projected).length();
        if exceeds_geometry_tolerance(offaxis) {
            sheared = true;
        }
        match first_step {
            None => first_step = Some(projected),
            Some(first) if exceeds_geometry_tolerance((projected - first).abs()) => {
                non_uniform = true;
            }
            Some(_) => {}
        }
        previous = Some(*position);
    }
    (sheared, non_uniform)
}

fn cmp_projection(left: &Prepared, right: &Prepared) -> Ordering {
    match (left.projection, right.projection) {
        (Some(left), Some(right)) => left.total_cmp(&right),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => left.input_index.cmp(&right.input_index),
    }
}

fn cmp_within_slot(left: &Prepared, right: &Prepared) -> Ordering {
    cmp_text(&left.stack_id, &right.stack_id)
        .then_with(|| cmp_option(left.in_stack, right.in_stack))
        .then_with(|| cmp_option(left.temporal, right.temporal))
        .then_with(|| cmp_option(left.instance, right.instance))
        .then_with(|| left.sop.cmp(&right.sop))
        .then_with(|| left.frame_index.cmp(&right.frame_index))
        .then_with(|| left.input_index.cmp(&right.input_index))
}

fn cmp_unpositioned(left: &Prepared, right: &Prepared) -> Ordering {
    cmp_option(left.instance, right.instance)
        .then_with(|| left.sop.cmp(&right.sop))
        .then_with(|| left.frame_index.cmp(&right.frame_index))
        .then_with(|| left.input_index.cmp(&right.input_index))
}

fn cmp_text(left: &Option<String>, right: &Option<String>) -> Ordering {
    match (left, right) {
        (Some(left), Some(right)) => left.cmp(right),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

fn cmp_option<T: Ord>(left: Option<T>, right: Option<T>) -> Ordering {
    match (left, right) {
        (Some(left), Some(right)) => left.cmp(&right),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

fn read_cine(
    metadata: &MetadataSet,
    frame_count: usize,
    index: u32,
) -> Result<(CineTiming, Playback), SeriesOpenError> {
    let playback = read_playback(metadata, index)?;
    let timing = read_timing(metadata, frame_count, index)?;
    Ok((timing, playback))
}

fn read_timing(
    metadata: &MetadataSet,
    frame_count: usize,
    index: u32,
) -> Result<CineTiming, SeriesOpenError> {
    if let Some(timing) = frame_time_vector(metadata, frame_count, index)? {
        return Ok(timing);
    }
    if let Some(milliseconds) = one_positive_ds(metadata, FRAME_TIME, index)? {
        return Ok(CineTiming::FrameTime { milliseconds });
    }
    if let Some(frames_per_second) =
        one_positive_is(metadata, RECOMMENDED_DISPLAY_FRAME_RATE, index)?
    {
        return Ok(CineTiming::RecommendedDisplayFrameRate { frames_per_second });
    }
    if let Some(frames_per_second) = one_positive_is(metadata, CINE_RATE, index)? {
        return Ok(CineTiming::CineRate { frames_per_second });
    }
    Ok(CineTiming::Unavailable)
}

fn frame_time_vector(
    metadata: &MetadataSet,
    frame_count: usize,
    index: u32,
) -> Result<Option<CineTiming>, SeriesOpenError> {
    let Some(element) = metadata.get(FRAME_TIME_VECTOR) else {
        return Ok(None);
    };
    if element.vr() != VR::DS {
        return Err(SeriesOpenError::new(SeriesOpenReason::Cine, index));
    }
    let Some(parts) = semantic_parts(element, index, SeriesOpenReason::Cine)? else {
        return Ok(None);
    };
    if parts.len() != frame_count {
        return Ok(None);
    }
    for (offset, value) in parts.iter().enumerate() {
        let Some(number) = parse_decimal_string(value) else {
            return Err(SeriesOpenError::new(SeriesOpenReason::Cine, index));
        };
        let allowed = if offset == 0 {
            matches!(
                number.partial_cmp(&0.0),
                Some(Ordering::Equal | Ordering::Greater)
            )
        } else {
            is_positive(number)
        };
        if !allowed {
            return Err(SeriesOpenError::new(SeriesOpenReason::Cine, index));
        }
    }
    Ok(Some(CineTiming::FrameTimeVector))
}

fn one_positive_ds(
    metadata: &MetadataSet,
    tag: Tag,
    index: u32,
) -> Result<Option<f64>, SeriesOpenError> {
    let Some(element) = metadata.get(tag) else {
        return Ok(None);
    };
    if element.vr() != VR::DS {
        return Err(SeriesOpenError::new(SeriesOpenReason::Cine, index));
    }
    let Some(parts) = present_text(element, index, SeriesOpenReason::Cine)? else {
        return Ok(None);
    };
    let [value] = parts.as_slice() else {
        return Err(SeriesOpenError::new(SeriesOpenReason::Cine, index));
    };
    let Some(number) = parse_decimal_string(value) else {
        return Err(SeriesOpenError::new(SeriesOpenReason::Cine, index));
    };
    if !is_positive(number) {
        return Err(SeriesOpenError::new(SeriesOpenReason::Cine, index));
    }
    Ok(Some(number))
}

fn one_positive_is(
    metadata: &MetadataSet,
    tag: Tag,
    index: u32,
) -> Result<Option<f64>, SeriesOpenError> {
    let Some(element) = metadata.get(tag) else {
        return Ok(None);
    };
    if element.vr() != VR::IS {
        return Err(SeriesOpenError::new(SeriesOpenReason::Cine, index));
    }
    let Some(text) = optional_single_text(element, index, VR::IS, SeriesOpenReason::Cine)? else {
        return Ok(None);
    };
    let parsed = text
        .parse::<i32>()
        .map_err(|_| SeriesOpenError::new(SeriesOpenReason::Cine, index))?;
    if parsed <= 0 {
        return Err(SeriesOpenError::new(SeriesOpenReason::Cine, index));
    }
    Ok(Some(f64::from(parsed)))
}

fn read_playback(metadata: &MetadataSet, index: u32) -> Result<Playback, SeriesOpenError> {
    let Some(element) = metadata.get(PREFERRED_PLAYBACK_SEQUENCING) else {
        return Ok(Playback::Loop);
    };
    if element.vr() != VR::US {
        return Err(SeriesOpenError::new(SeriesOpenReason::Playback, index));
    }
    match element.value() {
        MetadataValue::Empty => Ok(Playback::Loop),
        MetadataValue::Unsigned16(values) => match values.as_slice() {
            [] => Ok(Playback::Loop),
            [0] => Ok(Playback::Loop),
            [1] => Ok(Playback::Sweep),
            _ => Err(SeriesOpenError::new(SeriesOpenReason::Playback, index)),
        },
        _ => Err(SeriesOpenError::new(SeriesOpenReason::Playback, index)),
    }
}

fn object_key(parsed: &ParsedDicom) -> usize {
    std::ptr::from_ref(parsed).addr()
}

fn uid_identity(
    metadata: &MetadataSet,
    tag: Tag,
    parsed: &ParsedDicom,
    index: u32,
) -> Result<UidIdentity, SeriesOpenError> {
    let Some(element) = metadata.get(tag) else {
        return Ok(UidIdentity::Absent(object_key(parsed)));
    };
    match optional_single_text(element, index, VR::UI, SeriesOpenReason::Metadata)? {
        Some(value) => Ok(UidIdentity::Present(value)),
        None => Ok(UidIdentity::Absent(object_key(parsed))),
    }
}

fn sop_bytes(metadata: &MetadataSet, index: u32) -> Result<Vec<u8>, SeriesOpenError> {
    let Some(element) = metadata.get(SOP_INSTANCE_UID) else {
        return Ok(Vec::new());
    };
    match optional_single_text(element, index, VR::UI, SeriesOpenReason::Metadata)? {
        Some(value) => Ok(value.into_bytes()),
        None => Ok(Vec::new()),
    }
}

fn instance_number(metadata: &MetadataSet, index: u32) -> Result<Option<i64>, SeriesOpenError> {
    let Some(element) = metadata.get(INSTANCE_NUMBER) else {
        return Ok(None);
    };
    let Some(text) = optional_single_text(element, index, VR::IS, SeriesOpenReason::Metadata)?
    else {
        return Ok(None);
    };
    text.parse::<i64>()
        .map(Some)
        .map_err(|_| SeriesOpenError::new(SeriesOpenReason::Metadata, index))
}

fn resolved_ul(
    multi: &MultiframeMetadata<'_>,
    frame: usize,
    tag: Tag,
    index: u32,
) -> Result<Option<u32>, SeriesOpenError> {
    let resolved = multi
        .resolve(
            frame,
            FRAME_CONTENT_SEQUENCE,
            tag,
            TopLevelFallback::Allowed,
        )
        .map_err(|error| map_multi(error, index))?;
    let Some(resolved) = resolved else {
        return Ok(None);
    };
    one_ul(resolved.element(), index)
}

fn resolved_text(
    multi: &MultiframeMetadata<'_>,
    frame: usize,
    tag: Tag,
    vr: VR,
    index: u32,
) -> Result<Option<String>, SeriesOpenError> {
    let resolved = multi
        .resolve(
            frame,
            FRAME_CONTENT_SEQUENCE,
            tag,
            TopLevelFallback::Allowed,
        )
        .map_err(|error| map_multi(error, index))?;
    let Some(resolved) = resolved else {
        return Ok(None);
    };
    optional_single_text(resolved.element(), index, vr, SeriesOpenReason::Metadata)
}

fn one_ul(element: &MetadataElement, index: u32) -> Result<Option<u32>, SeriesOpenError> {
    if element.vr() != VR::UL {
        return Err(SeriesOpenError::new(SeriesOpenReason::Metadata, index));
    }
    match element.value() {
        MetadataValue::Empty => Ok(None),
        MetadataValue::Unsigned32(values) => match values.as_slice() {
            [] => Ok(None),
            [value] => Ok(Some(*value)),
            _ => Err(SeriesOpenError::new(SeriesOpenReason::Metadata, index)),
        },
        _ => Err(SeriesOpenError::new(SeriesOpenReason::Metadata, index)),
    }
}

fn optional_single_text(
    element: &MetadataElement,
    index: u32,
    vr: VR,
    reason: SeriesOpenReason,
) -> Result<Option<String>, SeriesOpenError> {
    if element.vr() != vr {
        return Err(SeriesOpenError::new(reason, index));
    }
    let Some(parts) = present_text(element, index, reason)? else {
        return Ok(None);
    };
    match parts.as_slice() {
        [one] => Ok(Some((*one).to_owned())),
        _ => Err(SeriesOpenError::new(reason, index)),
    }
}

fn is_positive(number: f64) -> bool {
    matches!(number.partial_cmp(&0.0), Some(Ordering::Greater))
}

fn semantic_parts(
    element: &MetadataElement,
    index: u32,
    reason: SeriesOpenReason,
) -> Result<Option<Vec<&str>>, SeriesOpenError> {
    match element.value() {
        MetadataValue::Empty => Ok(None),
        MetadataValue::Text(_) => {
            let Some(values) = element.semantic_text() else {
                return Err(SeriesOpenError::new(reason, index));
            };
            if values.is_empty() || values.iter().all(|value| value.is_empty()) {
                Ok(None)
            } else {
                Ok(Some(values))
            }
        }
        _ => Err(SeriesOpenError::new(reason, index)),
    }
}

fn present_text(
    element: &MetadataElement,
    index: u32,
    reason: SeriesOpenReason,
) -> Result<Option<Vec<&str>>, SeriesOpenError> {
    let Some(values) = semantic_parts(element, index, reason)? else {
        return Ok(None);
    };
    let parts: Vec<&str> = values
        .into_iter()
        .filter(|value| !value.is_empty())
        .collect();
    if parts.is_empty() {
        Ok(None)
    } else {
        Ok(Some(parts))
    }
}

#[cfg(test)]
thread_local! {
    static PROJECTIONS: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
fn projection_count() -> u32 {
    PROJECTIONS.with(std::cell::Cell::get)
}

#[cfg(test)]
fn reset_projections() {
    PROJECTIONS.with(|count| count.set(0));
}

#[cfg(test)]
mod projection_share {
    use super::{ParsedFrame, assemble_display, projection_count, reset_projections};
    use crate::parse_part10;

    #[test]
    fn two_frames_of_one_object_share_one_projection() {
        let built = one_slice();
        assert!(built.is_ok(), "fixture bytes");
        let Ok(built) = built else {
            unreachable!("asserted immediately above");
        };
        let parsed = parse_part10(&built);
        assert!(parsed.is_ok(), "{parsed:?}");
        let Ok(parsed) = parsed else {
            unreachable!("asserted immediately above");
        };
        reset_projections();
        let frames = [ParsedFrame::new(&parsed, 0), ParsedFrame::new(&parsed, 0)];
        let assembled = assemble_display(&frames);
        assert!(assembled.is_ok(), "{assembled:?}");
        let Ok(groups) = assembled else {
            unreachable!("asserted immediately above");
        };
        assert_eq!(groups.len(), 1);
        let Some(group) = groups.first() else {
            unreachable!("length is 1");
        };
        assert_eq!(group.frame_count(), 2);
        assert_eq!(projection_count(), 1);

        let again = parse_part10(&built);
        assert!(again.is_ok(), "{again:?}");
        let Ok(again) = again else {
            unreachable!("asserted immediately above");
        };
        reset_projections();
        let separate = [ParsedFrame::new(&parsed, 0), ParsedFrame::new(&again, 0)];
        let both = assemble_display(&separate);
        assert!(both.is_ok(), "{both:?}");
        assert_eq!(projection_count(), 2);
    }

    fn one_slice() -> Result<Vec<u8>, &'static str> {
        let mut data = Vec::new();
        data.extend(ui_element((0x0008, 0x0016), "1.2.840.10008.5.1.4.1.1.2")?);
        data.extend(ui_element((0x0008, 0x0018), "2.25.20")?);
        data.extend(ui_element((0x0020, 0x000E), "2.25.10")?);
        data.extend(ui_element((0x0020, 0x0052), "2.25.11")?);
        data.extend(text_element((0x0020, 0x0013), *b"IS", "1")?);
        data.extend(text_element((0x0020, 0x0032), *b"DS", "0\\0\\0")?);
        data.extend(text_element((0x0020, 0x0037), *b"DS", "1\\0\\0\\0\\1\\0")?);
        data.extend(text_element((0x0028, 0x0030), *b"DS", "1\\1")?);
        data.extend(short_le((0x0028, 0x0010), *b"US", &1_u16.to_le_bytes())?);
        data.extend(short_le((0x0028, 0x0011), *b"US", &1_u16.to_le_bytes())?);
        part10(data)
    }

    fn ui_element(tag: (u16, u16), text: &str) -> Result<Vec<u8>, &'static str> {
        short_le(tag, *b"UI", &ui(text.as_bytes()))
    }

    fn text_element(tag: (u16, u16), vr: [u8; 2], text: &str) -> Result<Vec<u8>, &'static str> {
        short_le(tag, vr, &text_pad(text))
    }

    fn ui(text: &[u8]) -> Vec<u8> {
        let mut value = text.to_vec();
        if !value.len().is_multiple_of(2) {
            value.push(0);
        }
        value
    }

    fn text_pad(text: &str) -> Vec<u8> {
        let mut value = text.as_bytes().to_vec();
        if !value.len().is_multiple_of(2) {
            value.push(b' ');
        }
        value
    }

    fn short_le(tag: (u16, u16), vr: [u8; 2], value: &[u8]) -> Result<Vec<u8>, &'static str> {
        if !value.len().is_multiple_of(2) {
            return Err("odd element");
        }
        let length = u16::try_from(value.len()).map_err(|_| "element length")?;
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&tag.0.to_le_bytes());
        bytes.extend_from_slice(&tag.1.to_le_bytes());
        bytes.extend_from_slice(&vr);
        bytes.extend_from_slice(&length.to_le_bytes());
        bytes.extend_from_slice(value);
        Ok(bytes)
    }

    fn long_le(tag: (u16, u16), vr: [u8; 2], value: &[u8]) -> Result<Vec<u8>, &'static str> {
        if !value.len().is_multiple_of(2) {
            return Err("odd element");
        }
        let length = u32::try_from(value.len()).map_err(|_| "element length")?;
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&tag.0.to_le_bytes());
        bytes.extend_from_slice(&tag.1.to_le_bytes());
        bytes.extend_from_slice(&vr);
        bytes.extend_from_slice(&[0, 0]);
        bytes.extend_from_slice(&length.to_le_bytes());
        bytes.extend_from_slice(value);
        Ok(bytes)
    }

    fn part10(data: Vec<u8>) -> Result<Vec<u8>, &'static str> {
        let mut bytes = vec![0_u8; 128];
        bytes.extend_from_slice(b"DICM");
        bytes.extend(file_meta()?);
        bytes.extend(data);
        Ok(bytes)
    }

    fn file_meta() -> Result<Vec<u8>, &'static str> {
        let mut body = Vec::new();
        body.extend(long_le((0x0002, 0x0001), *b"OB", &[0, 1])?);
        body.extend(ui_element((0x0002, 0x0002), "1.2.840.10008.5.1.4.1.1.2")?);
        body.extend(ui_element((0x0002, 0x0003), "2.25.1")?);
        body.extend(ui_element((0x0002, 0x0010), "1.2.840.10008.1.2.1")?);
        body.extend(ui_element((0x0002, 0x0012), "2.25.2")?);
        let length = u32::try_from(body.len()).map_err(|_| "meta length")?;
        let mut meta = short_le((0x0002, 0x0000), *b"UL", &length.to_le_bytes())?;
        meta.extend(body);
        Ok(meta)
    }
}
