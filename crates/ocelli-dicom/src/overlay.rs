//! Overlay planes and display shutters.
//!
//! DICOM PS3.3 C.9.2 is the overlay plane. C.7.6.11 is the geometric shutter.
//! C.7.6.15 is the bitmap shutter. This module reads those attributes. It does
//! not pass overlay bits through the LUT chain and it does not draw them.
//!
//! Circle centre is `(0018,1610)` with two integer components, and the radius
//! is `(0018,1612)`. Polygon vertices are `(0018,1620)`. The design text named
//! `(0018,1614)` as a third circle attribute. PS3.6 does not define that tag
//! for this module.
//!
//! A 1-bit overlay may be OB or OW. OW words are little-endian packed bits.

use dicom_core::{Tag, VR};

use super::{
    MetadataSet, MetadataValue, ParsedDicom, SeriesOpenError, SeriesOpenReason,
    declared_frame_count,
};
use crate::frame_open::project_metadata;

const ROWS: Tag = Tag(0x0028, 0x0010);
const COLUMNS: Tag = Tag(0x0028, 0x0011);
const SHUTTER_SHAPE: Tag = Tag(0x0018, 0x1600);
const SHUTTER_LEFT: Tag = Tag(0x0018, 0x1602);
const SHUTTER_RIGHT: Tag = Tag(0x0018, 0x1604);
const SHUTTER_UPPER: Tag = Tag(0x0018, 0x1606);
const SHUTTER_LOWER: Tag = Tag(0x0018, 0x1608);
const CIRCLE_CENTER: Tag = Tag(0x0018, 0x1610);
const CIRCLE_RADIUS: Tag = Tag(0x0018, 0x1612);
const POLYGON_VERTICES: Tag = Tag(0x0018, 0x1620);
const SHUTTER_PRESENTATION: Tag = Tag(0x0018, 0x1622);
const SHUTTER_OVERLAY_GROUP: Tag = Tag(0x0018, 0x1623);
const PER_FRAME_FUNCTIONAL_GROUPS: Tag = Tag(0x5200, 0x9230);

const OVERLAY_ROWS: u16 = 0x0010;
const OVERLAY_COLUMNS: u16 = 0x0011;
const OVERLAY_FRAMES: u16 = 0x0015;
const OVERLAY_TYPE: u16 = 0x0040;
const OVERLAY_ORIGIN: u16 = 0x0050;
const OVERLAY_FRAME_ORIGIN: u16 = 0x0051;
const OVERLAY_BITS_ALLOCATED: u16 = 0x0100;
const OVERLAY_BIT_POSITION: u16 = 0x0102;
const OVERLAY_DATA: u16 = 0x3000;

const FIRST_OVERLAY_GROUP: u16 = 0x6000;
const LAST_OVERLAY_GROUP: u16 = 0x601E;

/// Why an overlay or shutter could not be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
#[error("overlay read failed")]
pub struct OverlayError {
    reason: OverlayReason,
    index: u32,
}

impl OverlayError {
    /// The closed reason.
    #[must_use]
    pub const fn reason(self) -> OverlayReason {
        self.reason
    }

    /// The requested frame index.
    #[must_use]
    pub const fn index(self) -> u32 {
        self.index
    }

    const fn new(reason: OverlayReason, index: u32) -> Self {
        Self { reason, index }
    }
}

/// Closed reasons for [`OverlayError`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum OverlayReason {
    /// An attribute is missing, has the wrong value representation, or cannot be parsed.
    Metadata,
    /// The frame index is outside the declared frame count.
    FrameOutOfRange,
    /// A per-frame overlay group is present and is not read from the top-level groups.
    FrameOverlayUnsupported,
    /// Overlay Bits Allocated is not 1 or 16, or the bit position does not select one bit.
    BitsAllocated,
    /// The polygonal shutter has too few vertices, an odd count, a zero-length edge, or edges that cross or fold back.
    Polygon,
    /// A shutter attribute is present and is not a usable shape.
    Shutter,
}

/// Graphics or ROI. DICOM PS3.3 C.9.2.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum OverlayType {
    /// Overlay Type `G`.
    Graphics,
    /// Overlay Type `R`.
    Roi,
}

/// One overlay plane for one image frame.
///
/// Coordinates on [`Self::bit`] are 1-based image pixels. The stored bits stay
/// in overlay order. A set bit whose origin places it outside the image is
/// counted by [`Self::outside_count`] and is not dropped.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverlayPlane {
    group: u16,
    rows: u16,
    columns: u16,
    overlay_type: OverlayType,
    origin_row: i32,
    origin_column: i32,
    image_rows: u16,
    image_columns: u16,
    bits: PlaneBits,
    outside: u32,
}

impl OverlayPlane {
    /// Overlay group, for example `0x6000`.
    #[must_use]
    pub const fn group(&self) -> u16 {
        self.group
    }

    /// Overlay rows.
    #[must_use]
    pub const fn rows(&self) -> u16 {
        self.rows
    }

    /// Overlay columns.
    #[must_use]
    pub const fn columns(&self) -> u16 {
        self.columns
    }

    /// `G` or `R`.
    #[must_use]
    pub const fn overlay_type(&self) -> OverlayType {
        self.overlay_type
    }

    /// 1-based origin row. Values below 1 sit above the image.
    #[must_use]
    pub const fn origin_row(&self) -> i32 {
        self.origin_row
    }

    /// 1-based origin column. Values below 1 sit to the left of the image.
    #[must_use]
    pub const fn origin_column(&self) -> i32 {
        self.origin_column
    }

    /// Whether the overlay covers this 1-based image pixel.
    ///
    /// A row or column outside the image is clear. The bit is still counted
    /// by [`Self::outside_count`].
    #[must_use]
    pub fn bit(&self, row: i32, column: i32) -> bool {
        if row < 1
            || column < 1
            || row > i32::from(self.image_rows)
            || column > i32::from(self.image_columns)
        {
            return false;
        }
        let Some(index) = self.overlay_index(row, column) else {
            return false;
        };
        self.bits.set(index)
    }

    /// Set overlay bits that land outside the image.
    #[must_use]
    pub const fn outside_count(&self) -> u32 {
        self.outside
    }

    fn overlay_index(&self, row: i32, column: i32) -> Option<u32> {
        if row < 1 || column < 1 {
            return None;
        }
        let overlay_row = row.checked_sub(self.origin_row)?.checked_add(1)?;
        let overlay_column = column.checked_sub(self.origin_column)?.checked_add(1)?;
        if overlay_row < 1 || overlay_column < 1 {
            return None;
        }
        let overlay_row = u32::try_from(overlay_row).ok()?;
        let overlay_column = u32::try_from(overlay_column).ok()?;
        if overlay_row > u32::from(self.rows) || overlay_column > u32::from(self.columns) {
            return None;
        }
        let row_index = overlay_row.checked_sub(1)?;
        let column_index = overlay_column.checked_sub(1)?;
        row_index
            .checked_mul(u32::from(self.columns))?
            .checked_add(column_index)
    }
}

/// One geometric or bitmap shutter shape.
///
/// A polygonal vertex pair is row then column, matching PS3.3 C.7.6.11.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ShutterShape {
    /// Inclusive opening edges.
    Rectangular {
        /// Left vertical edge.
        left: i32,
        /// Right vertical edge.
        right: i32,
        /// Upper horizontal edge.
        upper: i32,
        /// Lower horizontal edge.
        lower: i32,
    },
    /// Inside when the squared distance is at most the squared radius.
    Circular {
        /// Centre row.
        center_row: f64,
        /// Centre column.
        center_column: f64,
        /// Radius in pixels.
        radius: f64,
    },
    /// A closed polygon. Vertices are row then column.
    Polygonal {
        /// At least three vertices.
        vertices: Vec<(i32, i32)>,
    },
    /// The overlay group used as the bitmap shutter.
    Bitmap {
        /// Overlay group number.
        group: u16,
    },
}

/// The display shutter for one frame.
///
/// [`Self::hides`] is true outside any geometric opening and where a bitmap
/// shutter bit is 1. Shapes combine by intersection of their openings.
#[derive(Debug, Clone, PartialEq)]
pub struct Shutter {
    shapes: Vec<ShutterShape>,
    bitmap: Option<OverlayPlane>,
}

impl Shutter {
    /// Shapes in the order the shutter attribute named them.
    #[must_use]
    pub fn shapes(&self) -> &[ShutterShape] {
        &self.shapes
    }

    /// Whether this 1-based image pixel is outside the displayed opening.
    #[must_use]
    pub fn hides(&self, row: i32, column: i32) -> bool {
        if row < 1 || column < 1 {
            return true;
        }
        for shape in &self.shapes {
            if shape_hides(shape, self.bitmap.as_ref(), row, column) {
                return true;
            }
        }
        false
    }
}

/// Overlay planes and the display shutter for one frame.
#[derive(Debug, Clone, PartialEq)]
pub struct OverlaySet {
    planes: Vec<OverlayPlane>,
    shutter: Option<Shutter>,
    presentation: Option<u16>,
    image_rows: u16,
    image_columns: u16,
}

impl OverlaySet {
    /// Graphic and ROI planes. A bitmap shutter group is not in this list.
    #[must_use]
    pub fn planes(&self) -> &[OverlayPlane] {
        &self.planes
    }

    /// The display shutter, when the shape attribute is present.
    #[must_use = "an absent shutter is meaningful"]
    pub const fn shutter(&self) -> Option<&Shutter> {
        self.shutter.as_ref()
    }

    /// Shutter Presentation Value. Absent and empty stay absent.
    #[must_use = "an absent presentation value is meaningful"]
    pub const fn presentation_value(&self) -> Option<u16> {
        self.presentation
    }

    /// Image rows from `(0028,0010)`.
    #[must_use]
    pub const fn image_rows(&self) -> u16 {
        self.image_rows
    }

    /// Image columns from `(0028,0011)`.
    #[must_use]
    pub const fn image_columns(&self) -> u16 {
        self.image_columns
    }
}

/// Read top-level overlay groups and the display shutter for one frame.
///
/// Pixel Data is not copied. An overlay group inside the requested frame's
/// per-frame functional groups item is refused.
///
/// # Errors
///
/// Returns [`OverlayError`] for a bad attribute, a frame outside the object,
/// a per-frame overlay, a bad bit allocation, a crossing polygon, or a shutter
/// that cannot be applied.
pub fn read_overlays(parsed: &ParsedDicom, frame: u32) -> Result<OverlaySet, OverlayError> {
    let metadata =
        project_metadata(parsed).map_err(|_| OverlayError::new(OverlayReason::Metadata, frame))?;
    let count = declared_frame_count(parsed).map_err(|error| map_series(error, frame))?;
    if frame >= count {
        return Err(OverlayError::new(OverlayReason::FrameOutOfRange, frame));
    }
    if frame_has_overlay_group(&metadata, frame)? {
        return Err(OverlayError::new(
            OverlayReason::FrameOverlayUnsupported,
            frame,
        ));
    }
    let image_rows = one_us(&metadata, ROWS, frame)?;
    let image_columns = one_us(&metadata, COLUMNS, frame)?;
    if image_rows == 0 || image_columns == 0 {
        return Err(OverlayError::new(OverlayReason::Metadata, frame));
    }
    let presentation = presentation_value(&metadata, frame)?;
    let mut planes = read_planes(&metadata, frame, image_rows, image_columns)?;
    let shutter = read_shutter(&metadata, &mut planes, frame, image_rows, image_columns)?;
    Ok(OverlaySet {
        planes,
        shutter,
        presentation,
        image_rows,
        image_columns,
    })
}

fn map_series(error: SeriesOpenError, frame: u32) -> OverlayError {
    let reason = match error.reason() {
        SeriesOpenReason::FrameOutOfRange => OverlayReason::FrameOutOfRange,
        _ => OverlayReason::Metadata,
    };
    OverlayError::new(reason, frame)
}

fn frame_has_overlay_group(metadata: &MetadataSet, frame: u32) -> Result<bool, OverlayError> {
    let Some(element) = metadata.get(PER_FRAME_FUNCTIONAL_GROUPS) else {
        return Ok(false);
    };
    if element.vr() != VR::SQ {
        return Err(OverlayError::new(OverlayReason::Metadata, frame));
    }
    let MetadataValue::Sequence(items) = element.value() else {
        return Err(OverlayError::new(OverlayReason::Metadata, frame));
    };
    let Some(index) = usize::try_from(frame).ok() else {
        return Err(OverlayError::new(OverlayReason::FrameOutOfRange, frame));
    };
    let Some(item) = items.get(index) else {
        return Ok(false);
    };
    Ok(set_has_overlay_group(item))
}

fn set_has_overlay_group(item: &MetadataSet) -> bool {
    for (tag, element) in item.iter() {
        if overlay_group(tag.group()) {
            return true;
        }
        if let MetadataValue::Sequence(nested) = element.value() {
            for child in nested {
                if set_has_overlay_group(child) {
                    return true;
                }
            }
        }
    }
    false
}

fn overlay_group(group: u16) -> bool {
    (FIRST_OVERLAY_GROUP..=LAST_OVERLAY_GROUP).contains(&group) && group.is_multiple_of(2)
}

fn read_planes(
    metadata: &MetadataSet,
    frame: u32,
    image_rows: u16,
    image_columns: u16,
) -> Result<Vec<OverlayPlane>, OverlayError> {
    let mut planes = Vec::new();
    let mut group = FIRST_OVERLAY_GROUP;
    loop {
        if let Some(plane) = read_group(metadata, group, frame, image_rows, image_columns)? {
            planes.push(plane);
        }
        if group == LAST_OVERLAY_GROUP {
            break;
        }
        group = group.saturating_add(2);
    }
    Ok(planes)
}

fn read_group(
    metadata: &MetadataSet,
    group: u16,
    frame: u32,
    image_rows: u16,
    image_columns: u16,
) -> Result<Option<OverlayPlane>, OverlayError> {
    let rows_tag = Tag(group, OVERLAY_ROWS);
    if metadata.get(rows_tag).is_none() {
        return Ok(None);
    }
    let rows = one_us(metadata, rows_tag, frame)?;
    let columns = one_us(metadata, Tag(group, OVERLAY_COLUMNS), frame)?;
    if rows == 0 || columns == 0 {
        return Err(OverlayError::new(OverlayReason::Metadata, frame));
    }
    let overlay_type = overlay_type(metadata, group, frame)?;
    let (origin_row, origin_column) = origin(metadata, group, frame)?;
    let allocated = one_us(metadata, Tag(group, OVERLAY_BITS_ALLOCATED), frame)?;
    let position = one_us(metadata, Tag(group, OVERLAY_BIT_POSITION), frame)?;
    let overlay_frames = optional_overlay_frames(metadata, group, frame)?;
    let frame_origin = optional_frame_origin(metadata, group, frame)?;
    let Some(plane_index) = plane_index(frame, frame_origin, overlay_frames) else {
        return Ok(None);
    };
    let bits = plane_bits(
        metadata,
        frame,
        BitSource {
            group,
            rows,
            columns,
            allocated,
            position,
            overlay_frames,
            plane_index,
        },
    )?;
    let outside = outside_count(
        Coverage {
            rows,
            columns,
            origin_row,
            origin_column,
            image_rows,
            image_columns,
        },
        &bits,
        frame,
    )?;
    Ok(Some(OverlayPlane {
        group,
        rows,
        columns,
        overlay_type,
        origin_row,
        origin_column,
        image_rows,
        image_columns,
        bits,
        outside,
    }))
}

fn plane_index(frame: u32, origin: u32, count: u32) -> Option<u32> {
    let first = origin.checked_sub(1)?;
    let end = first.checked_add(count)?;
    if frame < first || frame >= end {
        return None;
    }
    Some(frame - first)
}

fn overlay_type(
    metadata: &MetadataSet,
    group: u16,
    frame: u32,
) -> Result<OverlayType, OverlayError> {
    let values = cs_values(
        metadata,
        Tag(group, OVERLAY_TYPE),
        frame,
        OverlayReason::Metadata,
    )?;
    let Some(value) = values.first() else {
        return Err(OverlayError::new(OverlayReason::Metadata, frame));
    };
    if values.len() != 1 {
        return Err(OverlayError::new(OverlayReason::Metadata, frame));
    }
    match value.as_str() {
        "G" => Ok(OverlayType::Graphics),
        "R" => Ok(OverlayType::Roi),
        _ => Err(OverlayError::new(OverlayReason::Metadata, frame)),
    }
}

fn origin(metadata: &MetadataSet, group: u16, frame: u32) -> Result<(i32, i32), OverlayError> {
    let element = required(metadata, Tag(group, OVERLAY_ORIGIN), frame)?;
    if element.vr() != VR::SS {
        return Err(OverlayError::new(OverlayReason::Metadata, frame));
    }
    let MetadataValue::Signed16(values) = element.value() else {
        return Err(OverlayError::new(OverlayReason::Metadata, frame));
    };
    let [row, column] = values.as_slice() else {
        return Err(OverlayError::new(OverlayReason::Metadata, frame));
    };
    Ok((i32::from(*row), i32::from(*column)))
}

fn optional_overlay_frames(
    metadata: &MetadataSet,
    group: u16,
    frame: u32,
) -> Result<u32, OverlayError> {
    let Some(element) = metadata.get(Tag(group, OVERLAY_FRAMES)) else {
        return Ok(1);
    };
    let values = is_values(element, frame, OverlayReason::Metadata)?;
    let Some(value) = values.first() else {
        return Err(OverlayError::new(OverlayReason::Metadata, frame));
    };
    if values.len() != 1 || *value <= 0 {
        return Err(OverlayError::new(OverlayReason::Metadata, frame));
    }
    u32::try_from(*value).map_err(|_| OverlayError::new(OverlayReason::Metadata, frame))
}

fn optional_frame_origin(
    metadata: &MetadataSet,
    group: u16,
    frame: u32,
) -> Result<u32, OverlayError> {
    let Some(element) = metadata.get(Tag(group, OVERLAY_FRAME_ORIGIN)) else {
        return Ok(1);
    };
    let value = one_us_element(element, frame)?;
    if value == 0 {
        return Err(OverlayError::new(OverlayReason::Metadata, frame));
    }
    Ok(u32::from(value))
}

struct BitSource {
    group: u16,
    rows: u16,
    columns: u16,
    allocated: u16,
    position: u16,
    overlay_frames: u32,
    plane_index: u32,
}

fn plane_bits(
    metadata: &MetadataSet,
    frame: u32,
    source: BitSource,
) -> Result<PlaneBits, OverlayError> {
    let pixels = u32::from(source.rows)
        .checked_mul(u32::from(source.columns))
        .ok_or(OverlayError::new(OverlayReason::Metadata, frame))?;
    let total = pixels
        .checked_mul(source.overlay_frames)
        .ok_or(OverlayError::new(OverlayReason::Metadata, frame))?;
    let offset = pixels
        .checked_mul(source.plane_index)
        .ok_or(OverlayError::new(OverlayReason::Metadata, frame))?;
    let element = required(metadata, Tag(source.group, OVERLAY_DATA), frame)?;
    match source.allocated {
        1 => {
            if source.position != 0 {
                return Err(OverlayError::new(OverlayReason::BitsAllocated, frame));
            }
            let bytes = packed_bytes(element, frame)?;
            let need = total.div_ceil(8);
            let len = u32::try_from(bytes.len())
                .map_err(|_| OverlayError::new(OverlayReason::Metadata, frame))?;
            if len < need {
                return Err(OverlayError::new(OverlayReason::Metadata, frame));
            }
            Ok(PlaneBits::Packed { bytes, offset })
        }
        16 => {
            if source.position > 15 {
                return Err(OverlayError::new(OverlayReason::BitsAllocated, frame));
            }
            if element.vr() != VR::OW {
                return Err(OverlayError::new(OverlayReason::BitsAllocated, frame));
            }
            let MetadataValue::Unsigned16(words) = element.value() else {
                return Err(OverlayError::new(OverlayReason::BitsAllocated, frame));
            };
            let len = u32::try_from(words.len())
                .map_err(|_| OverlayError::new(OverlayReason::Metadata, frame))?;
            if len < total {
                return Err(OverlayError::new(OverlayReason::Metadata, frame));
            }
            Ok(PlaneBits::Words {
                words: words.clone(),
                position: source.position,
                offset,
            })
        }
        _ => Err(OverlayError::new(OverlayReason::BitsAllocated, frame)),
    }
}

struct Coverage {
    rows: u16,
    columns: u16,
    origin_row: i32,
    origin_column: i32,
    image_rows: u16,
    image_columns: u16,
}

fn outside_count(coverage: Coverage, bits: &PlaneBits, frame: u32) -> Result<u32, OverlayError> {
    let mut outside = 0_u32;
    let mut row = 1_i32;
    while row <= i32::from(coverage.rows) {
        let mut column = 1_i32;
        while column <= i32::from(coverage.columns) {
            let image_row = row
                .checked_add(coverage.origin_row)
                .and_then(|value| value.checked_sub(1));
            let image_column = column
                .checked_add(coverage.origin_column)
                .and_then(|value| value.checked_sub(1));
            let Some(image_row) = image_row else {
                return Err(OverlayError::new(OverlayReason::Metadata, frame));
            };
            let Some(image_column) = image_column else {
                return Err(OverlayError::new(OverlayReason::Metadata, frame));
            };
            let row_index = u32::try_from(
                row.checked_sub(1)
                    .ok_or(OverlayError::new(OverlayReason::Metadata, frame))?,
            )
            .map_err(|_| OverlayError::new(OverlayReason::Metadata, frame))?;
            let column_index = u32::try_from(
                column
                    .checked_sub(1)
                    .ok_or(OverlayError::new(OverlayReason::Metadata, frame))?,
            )
            .map_err(|_| OverlayError::new(OverlayReason::Metadata, frame))?;
            let index = row_index
                .checked_mul(u32::from(coverage.columns))
                .and_then(|value| value.checked_add(column_index))
                .ok_or(OverlayError::new(OverlayReason::Metadata, frame))?;
            if bits.set(index)
                && image_outside(
                    image_row,
                    image_column,
                    coverage.image_rows,
                    coverage.image_columns,
                )
            {
                outside = outside.saturating_add(1);
            }
            column = column.saturating_add(1);
        }
        row = row.saturating_add(1);
    }
    Ok(outside)
}

fn image_outside(row: i32, column: i32, rows: u16, columns: u16) -> bool {
    row < 1 || column < 1 || row > i32::from(rows) || column > i32::from(columns)
}

fn presentation_value(metadata: &MetadataSet, frame: u32) -> Result<Option<u16>, OverlayError> {
    let Some(element) = metadata.get(SHUTTER_PRESENTATION) else {
        return Ok(None);
    };
    if element.vr() != VR::US {
        return Err(OverlayError::new(OverlayReason::Shutter, frame));
    }
    match element.value() {
        MetadataValue::Empty => Ok(None),
        MetadataValue::Unsigned16(values) if values.is_empty() => Ok(None),
        MetadataValue::Unsigned16(values) => match values.as_slice() {
            [value] => Ok(Some(*value)),
            _ => Err(OverlayError::new(OverlayReason::Shutter, frame)),
        },
        _ => Err(OverlayError::new(OverlayReason::Shutter, frame)),
    }
}

fn read_shutter(
    metadata: &MetadataSet,
    planes: &mut Vec<OverlayPlane>,
    frame: u32,
    image_rows: u16,
    image_columns: u16,
) -> Result<Option<Shutter>, OverlayError> {
    let Some(element) = metadata.get(SHUTTER_SHAPE) else {
        return Ok(None);
    };
    let names = cs_element(element, frame, OverlayReason::Shutter)?;
    if names.is_empty() {
        return Err(OverlayError::new(OverlayReason::Shutter, frame));
    }
    let mut shapes = Vec::new();
    let mut bitmap = None;
    for name in &names {
        match name.as_str() {
            "RECTANGULAR" => shapes.push(rectangular(metadata, frame)?),
            "CIRCULAR" => shapes.push(circular(metadata, frame)?),
            "POLYGONAL" => shapes.push(polygonal(metadata, frame)?),
            "BITMAP" => {
                let (shape, plane) =
                    bitmap_shutter(metadata, planes, frame, image_rows, image_columns)?;
                shapes.push(shape);
                bitmap = Some(plane);
            }
            _ => return Err(OverlayError::new(OverlayReason::Shutter, frame)),
        }
    }
    Ok(Some(Shutter { shapes, bitmap }))
}

fn rectangular(metadata: &MetadataSet, frame: u32) -> Result<ShutterShape, OverlayError> {
    let left = one_is(metadata, SHUTTER_LEFT, frame)?;
    let right = one_is(metadata, SHUTTER_RIGHT, frame)?;
    let upper = one_is(metadata, SHUTTER_UPPER, frame)?;
    let lower = one_is(metadata, SHUTTER_LOWER, frame)?;
    if left > right || upper > lower {
        return Err(OverlayError::new(OverlayReason::Shutter, frame));
    }
    Ok(ShutterShape::Rectangular {
        left,
        right,
        upper,
        lower,
    })
}

fn circular(metadata: &MetadataSet, frame: u32) -> Result<ShutterShape, OverlayError> {
    let element = required(metadata, CIRCLE_CENTER, frame)?;
    let center = is_values(element, frame, OverlayReason::Shutter)?;
    let [center_row, center_column] = center.as_slice() else {
        return Err(OverlayError::new(OverlayReason::Shutter, frame));
    };
    let radius = one_is(metadata, CIRCLE_RADIUS, frame)?;
    if radius < 0 {
        return Err(OverlayError::new(OverlayReason::Shutter, frame));
    }
    Ok(ShutterShape::Circular {
        center_row: f64::from(*center_row),
        center_column: f64::from(*center_column),
        radius: f64::from(radius),
    })
}

fn polygonal(metadata: &MetadataSet, frame: u32) -> Result<ShutterShape, OverlayError> {
    let element = required(metadata, POLYGON_VERTICES, frame)?;
    let numbers = is_values(element, frame, OverlayReason::Polygon)?;
    if !numbers.len().is_multiple_of(2) || numbers.len() < 6 {
        return Err(OverlayError::new(OverlayReason::Polygon, frame));
    }
    let mut vertices = Vec::new();
    let mut index = 0;
    while index < numbers.len() {
        let Some(row) = numbers.get(index).copied() else {
            return Err(OverlayError::new(OverlayReason::Polygon, frame));
        };
        let Some(column) = numbers.get(index + 1).copied() else {
            return Err(OverlayError::new(OverlayReason::Polygon, frame));
        };
        vertices.push((row, column));
        index += 2;
    }
    if polygon_crosses(&vertices) {
        return Err(OverlayError::new(OverlayReason::Polygon, frame));
    }
    Ok(ShutterShape::Polygonal { vertices })
}

fn bitmap_shutter(
    metadata: &MetadataSet,
    planes: &mut Vec<OverlayPlane>,
    frame: u32,
    image_rows: u16,
    image_columns: u16,
) -> Result<(ShutterShape, OverlayPlane), OverlayError> {
    let group = one_us(metadata, SHUTTER_OVERLAY_GROUP, frame)?;
    if !overlay_group(group) {
        return Err(OverlayError::new(OverlayReason::Shutter, frame));
    }
    let Some(index) = planes.iter().position(|plane| plane.group == group) else {
        return Err(OverlayError::new(OverlayReason::Shutter, frame));
    };
    let plane = planes
        .get(index)
        .cloned()
        .ok_or(OverlayError::new(OverlayReason::Shutter, frame))?;
    if plane.overlay_type != OverlayType::Graphics
        || !matches!(plane.bits, PlaneBits::Packed { .. })
        || plane.origin_row != 1
        || plane.origin_column != 1
        || plane.rows != image_rows
        || plane.columns != image_columns
    {
        return Err(OverlayError::new(OverlayReason::Shutter, frame));
    }
    let _ = planes.remove(index);
    Ok((ShutterShape::Bitmap { group }, plane))
}

fn shape_hides(shape: &ShutterShape, bitmap: Option<&OverlayPlane>, row: i32, column: i32) -> bool {
    match shape {
        ShutterShape::Rectangular {
            left,
            right,
            upper,
            lower,
        } => column < *left || column > *right || row < *upper || row > *lower,
        ShutterShape::Circular {
            center_row,
            center_column,
            radius,
        } => {
            let row_delta = f64::from(row) - *center_row;
            let column_delta = f64::from(column) - *center_column;
            row_delta * row_delta + column_delta * column_delta > radius * radius
        }
        ShutterShape::Polygonal { vertices } => !polygon_contains(vertices, row, column),
        ShutterShape::Bitmap { .. } => {
            let Some(plane) = bitmap else {
                return true;
            };
            if row > i32::from(plane.image_rows) || column > i32::from(plane.image_columns) {
                return true;
            }
            plane.bit(row, column)
        }
    }
}

fn packed_bytes(element: &super::MetadataElement, frame: u32) -> Result<Vec<u8>, OverlayError> {
    match (element.vr(), element.value()) {
        (VR::OB, MetadataValue::Bytes(bytes)) => Ok(bytes.clone()),
        (VR::OW, MetadataValue::Unsigned16(words)) => {
            let mut bytes = Vec::new();
            for word in words {
                bytes.extend_from_slice(&word.to_le_bytes());
            }
            Ok(bytes)
        }
        _ => Err(OverlayError::new(OverlayReason::Metadata, frame)),
    }
}

fn polygon_crosses(vertices: &[(i32, i32)]) -> bool {
    let count = vertices.len();
    for index in 0..count {
        if edge(vertices, index).is_none() {
            return true;
        }
    }
    for left in 0..count {
        for right in (left + 1)..count {
            if shares_vertex(left, right, count) {
                if adjacent_overlap(vertices, left, right) {
                    return true;
                }
                continue;
            }
            let Some(a) = edge(vertices, left) else {
                return true;
            };
            let Some(b) = edge(vertices, right) else {
                return true;
            };
            if segments_cross(a.0, a.1, b.0, b.1) {
                return true;
            }
        }
    }
    false
}

fn adjacent_overlap(vertices: &[(i32, i32)], left: usize, right: usize) -> bool {
    let Some((a, b)) = edge(vertices, left) else {
        return true;
    };
    let Some((c, d)) = edge(vertices, right) else {
        return true;
    };
    if orient(a, b, c) != 0 || orient(a, b, d) != 0 {
        return false;
    }
    interval_overlap(a.0, b.0, c.0, d.0) > 0 || interval_overlap(a.1, b.1, c.1, d.1) > 0
}

fn interval_overlap(a0: i32, a1: i32, b0: i32, b1: i32) -> i64 {
    let left = i64::from(a0.min(a1)).max(i64::from(b0.min(b1)));
    let right = i64::from(a0.max(a1)).min(i64::from(b0.max(b1)));
    (right - left).max(0)
}

fn shares_vertex(left: usize, right: usize, count: usize) -> bool {
    let left_end = left.saturating_add(1) % count;
    let right_end = right.saturating_add(1) % count;
    left == right_end || left_end == right || left_end == right_end
}

fn edge(vertices: &[(i32, i32)], index: usize) -> Option<((i32, i32), (i32, i32))> {
    let count = vertices.len();
    let start = vertices.get(index).copied()?;
    let end = vertices.get(index.saturating_add(1) % count).copied()?;
    if start == end {
        return None;
    }
    Some((xy(start), xy(end)))
}

fn polygon_contains(vertices: &[(i32, i32)], row: i32, column: i32) -> bool {
    let point = (column, row);
    let count = vertices.len();
    for index in 0..count {
        let Some((start, end)) = edge(vertices, index) else {
            return false;
        };
        if on_segment(start, end, point) {
            return true;
        }
    }
    let mut inside = false;
    for index in 0..count {
        let Some((start, end)) = edge(vertices, index) else {
            return false;
        };
        if ray_crosses(start, end, point) {
            inside = !inside;
        }
    }
    inside
}

fn ray_crosses(start: (i32, i32), end: (i32, i32), point: (i32, i32)) -> bool {
    let (x1, y1) = start;
    let (x2, y2) = end;
    let (column, row) = point;
    if (y1 > row) == (y2 > row) {
        return false;
    }
    let dy = i64::from(y2) - i64::from(y1);
    let dx = i64::from(x2) - i64::from(x1);
    let left = i64::from(column) * dy;
    let right = i64::from(x1) * dy + dx * (i64::from(row) - i64::from(y1));
    if dy > 0 { left < right } else { left > right }
}

fn segments_cross(a: (i32, i32), b: (i32, i32), c: (i32, i32), d: (i32, i32)) -> bool {
    let o1 = orient(a, b, c);
    let o2 = orient(a, b, d);
    let o3 = orient(c, d, a);
    let o4 = orient(c, d, b);
    if o1 == 0 && o2 == 0 {
        return bbox_overlap(a, b, c, d);
    }
    o1.signum() != o2.signum() && o3.signum() != o4.signum()
}

fn orient(a: (i32, i32), b: (i32, i32), c: (i32, i32)) -> i64 {
    let (ax, ay) = a;
    let (bx, by) = b;
    let (cx, cy) = c;
    (i64::from(bx) - i64::from(ax)) * (i64::from(cy) - i64::from(ay))
        - (i64::from(by) - i64::from(ay)) * (i64::from(cx) - i64::from(ax))
}

fn on_segment(a: (i32, i32), b: (i32, i32), point: (i32, i32)) -> bool {
    if orient(a, b, point) != 0 {
        return false;
    }
    let (ax, ay) = a;
    let (bx, by) = b;
    let (px, py) = point;
    px >= ax.min(bx) && px <= ax.max(bx) && py >= ay.min(by) && py <= ay.max(by)
}

fn bbox_overlap(a: (i32, i32), b: (i32, i32), c: (i32, i32), d: (i32, i32)) -> bool {
    let (ax, ay) = a;
    let (bx, by) = b;
    let (cx, cy) = c;
    let (dx, dy) = d;
    ax.min(bx) <= cx.max(dx)
        && cx.min(dx) <= ax.max(bx)
        && ay.min(by) <= cy.max(dy)
        && cy.min(dy) <= ay.max(by)
}

fn xy(vertex: (i32, i32)) -> (i32, i32) {
    (vertex.1, vertex.0)
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PlaneBits {
    Packed {
        bytes: Vec<u8>,
        offset: u32,
    },
    Words {
        words: Vec<u16>,
        position: u16,
        offset: u32,
    },
}

impl PlaneBits {
    fn set(&self, index: u32) -> bool {
        match self {
            Self::Packed { bytes, offset } => {
                let Some(at) = offset.checked_add(index) else {
                    return false;
                };
                let Some(byte_index) = usize::try_from(at / 8).ok() else {
                    return false;
                };
                let Some(byte) = bytes.get(byte_index) else {
                    return false;
                };
                let shift = at % 8;
                (byte >> shift) & 1 == 1
            }
            Self::Words {
                words,
                position,
                offset,
            } => {
                let Some(at) = offset.checked_add(index) else {
                    return false;
                };
                let Some(word_index) = usize::try_from(at).ok() else {
                    return false;
                };
                let Some(word) = words.get(word_index) else {
                    return false;
                };
                (word >> u32::from(*position)) & 1 == 1
            }
        }
    }
}

fn required(
    metadata: &MetadataSet,
    tag: Tag,
    frame: u32,
) -> Result<&super::MetadataElement, OverlayError> {
    metadata
        .get(tag)
        .ok_or(OverlayError::new(OverlayReason::Metadata, frame))
}

fn one_us(metadata: &MetadataSet, tag: Tag, frame: u32) -> Result<u16, OverlayError> {
    one_us_element(required(metadata, tag, frame)?, frame)
}

fn one_us_element(element: &super::MetadataElement, frame: u32) -> Result<u16, OverlayError> {
    if element.vr() != VR::US {
        return Err(OverlayError::new(OverlayReason::Metadata, frame));
    }
    let MetadataValue::Unsigned16(values) = element.value() else {
        return Err(OverlayError::new(OverlayReason::Metadata, frame));
    };
    match values.as_slice() {
        [value] => Ok(*value),
        _ => Err(OverlayError::new(OverlayReason::Metadata, frame)),
    }
}

fn one_is(metadata: &MetadataSet, tag: Tag, frame: u32) -> Result<i32, OverlayError> {
    let values = is_values(
        required(metadata, tag, frame)?,
        frame,
        OverlayReason::Shutter,
    )?;
    match values.as_slice() {
        [value] => Ok(*value),
        _ => Err(OverlayError::new(OverlayReason::Shutter, frame)),
    }
}

fn is_values(
    element: &super::MetadataElement,
    frame: u32,
    reason: OverlayReason,
) -> Result<Vec<i32>, OverlayError> {
    if element.vr() != VR::IS {
        return Err(OverlayError::new(reason, frame));
    }
    let MetadataValue::Text(values) = element.value() else {
        return Err(OverlayError::new(reason, frame));
    };
    let mut numbers = Vec::new();
    for value in values {
        let text = value.trim_matches(' ');
        if text.is_empty() || text.len() > 12 {
            return Err(OverlayError::new(reason, frame));
        }
        for part in text.split('\\') {
            let part = part.trim_matches(' ');
            if part.is_empty() || part.len() > 12 {
                return Err(OverlayError::new(reason, frame));
            }
            let number = part
                .parse::<i32>()
                .map_err(|_| OverlayError::new(reason, frame))?;
            numbers.push(number);
        }
    }
    if numbers.is_empty() {
        return Err(OverlayError::new(reason, frame));
    }
    Ok(numbers)
}

fn cs_values(
    metadata: &MetadataSet,
    tag: Tag,
    frame: u32,
    reason: OverlayReason,
) -> Result<Vec<String>, OverlayError> {
    cs_element(required(metadata, tag, frame)?, frame, reason)
}

fn cs_element(
    element: &super::MetadataElement,
    frame: u32,
    reason: OverlayReason,
) -> Result<Vec<String>, OverlayError> {
    if element.vr() != VR::CS {
        return Err(OverlayError::new(reason, frame));
    }
    let MetadataValue::Text(values) = element.value() else {
        return Err(OverlayError::new(reason, frame));
    };
    let mut names = Vec::new();
    for value in values {
        for part in value.split('\\') {
            let part = part.trim_matches(' ');
            if !part.is_empty() {
                names.push(part.to_string());
            }
        }
    }
    Ok(names)
}
