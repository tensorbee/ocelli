//! The modality value under a point, stage 1 of the LUT chain.
//!
//! PS3.3 C.11.1: output units are `m * SV + b`. The arithmetic is
//! `ModalityTransform::apply` in `ocelli-pixel`, and the transform is the
//! held frame's own stage 1, the one the preview draws with. This module
//! chooses the cell, unpacks that one stored sample with
//! `StoredPixelDescription::unpack`, checks padding with
//! `ocelli_dicom::read_padding` and names the unit. It reads no window,
//! presentation shape, lossy flag or burned-in flag, so none of them can
//! refuse a value, and the value is the same for every window, invert,
//! rotation, flip, zoom and pan.

use dicom_core::Tag;
use ocelli_core::{Stored, exact_f32_integer};
use ocelli_dicom::{
    DefaultDicomObject, MetadataSet, MultiframeMetadata, PaddingRange, read_padding,
};
use ocelli_pixel::{
    ImageDimensions, ModalityTransform, PixelRepresentation, StoredPixelDescription,
};
use ocelli_viewport::view::f32_from_f64;

use crate::preview::{
    HeldFrame, MODALITY, PIXEL_VALUE_TRANSFORMATION, PreviewError, decode_held, json_string,
    optional_text, push_field, resolved_text,
};

const RESCALE_TYPE: Tag = Tag(0x0028, 0x1054);
const PET_UNITS: Tag = Tag(0x0054, 0x1001);
const MULTI_ENERGY: Tag = Tag(0x0018, 0x9361);
/// The longest unit a file can name, in characters. PS3.5 Table 6.2-1: LO,
/// the VR of Rescale Type, is "64 chars maximum", and CS, the VR of Units,
/// is "16 bytes maximum" of the Default Character Repertoire.
const UNIT_MAX_CHARS: usize = 64;
/// The most bytes a unit of [`UNIT_MAX_CHARS`] can own once decoded into
/// Rust's UTF-8, at most [`char::MAX_LEN_UTF8`] bytes per character. The
/// pre-decode claim includes it, so a stored frame never charges more than
/// its claim made room for.
pub(crate) const UNIT_MAX_BYTES: usize = UNIT_MAX_CHARS * char::MAX_LEN_UTF8;
/// Every integer up to `2^24` is exact in `f32`, which is what `Stored` holds.
const EXACT_STORED_BITS: u8 = 24;

/// A point in page image-pixel coordinates.
///
/// Cell `c` covers `[c, c + 1)` and its centre is `c + 0.5`. The right and
/// bottom edges, `x == width` and `y == height`, belong to the last cell.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImagePoint {
    /// Column coordinate, from 0 at the left edge to Columns at the right.
    pub x: f64,
    /// Row coordinate, from 0 at the top edge to Rows at the bottom.
    pub y: f64,
}

/// The request-independent inputs of a value, computed once per held frame.
pub(crate) struct ValueFacts {
    /// Stage 1, a copy of the held frame's recipe. A Modality LUT Sequence
    /// never reaches it.
    pub(crate) modality: ModalityTransform,
    /// Stored padding range, PS3.3 C.7.5.1.1.2.
    padding: Option<PaddingRange>,
    /// Unit token, such as `HU`.
    unit: UnitText,
    /// Which attribute named the unit.
    unit_source: &'static str,
}

impl ValueFacts {
    /// The heap bytes these facts own: the boxed struct and a file's unit text.
    pub(crate) fn heap_bytes(&self) -> usize {
        size_of::<Self>().saturating_add(self.unit_capacity())
    }

    /// The stored padding range, PS3.3 C.7.5.1.1.2. F-211 reads it for
    /// region statistics.
    pub(crate) fn padding(&self) -> Option<PaddingRange> {
        self.padding
    }

    /// The unit token, such as `HU`. F-211 names region statistics with it.
    pub(crate) fn unit(&self) -> &str {
        self.unit.as_str()
    }

    /// The bytes a file's unit text owns, and 0 for a rule's unit.
    pub(crate) fn unit_capacity(&self) -> usize {
        match &self.unit {
            UnitText::Rule(_) => 0,
            UnitText::File(text) => text.len(),
        }
    }
}

/// A unit named by this module's rule, or by the file.
///
/// A file's text is a `Box<str>`, which owns exactly its length, at most
/// [`UNIT_MAX_BYTES`].
enum UnitText {
    /// `HU` or `unspecified`, from the rule.
    Rule(&'static str),
    /// Rescale Type or Units as the file states it.
    File(Box<str>),
}

impl UnitText {
    fn as_str(&self) -> &str {
        match self {
            Self::Rule(text) => text,
            Self::File(text) => text,
        }
    }

    /// A file's unit, refused `attribute` past the longest LO value.
    fn from_file(code: &str) -> Result<Self, PreviewError> {
        if code.chars().count() > UNIT_MAX_CHARS {
            return Err(PreviewError::Attribute);
        }
        Ok(Self::File(Box::from(code)))
    }
}

/// Read the value inputs of one frame.
///
/// `modality` is the held frame's stage 1, built after the caller refused a
/// Modality LUT Sequence. Only padding and the unit are read here.
pub(crate) fn value_facts(
    metadata: &MetadataSet,
    object: &DefaultDicomObject,
    multi: &MultiframeMetadata<'_>,
    frame_index: usize,
    description: &StoredPixelDescription,
    modality: &ModalityTransform,
) -> Result<ValueFacts, PreviewError> {
    let signed = description.bits.pixel_representation() == PixelRepresentation::Signed;
    let padding = read_padding(
        metadata,
        description.layout.photometric_interpretation(),
        signed,
    )
    .map_err(|_| PreviewError::Attribute)?;
    // A padded stored value is compared as an integer. `Stored` is `f32`, so
    // above 24 bits the comparison could be inexact, and it is refused.
    if padding.is_some() && description.bits.bits_stored() > EXACT_STORED_BITS {
        return Err(PreviewError::Attribute);
    }
    let rescale_type = resolved_text(multi, frame_index, PIXEL_VALUE_TRANSFORMATION, RESCALE_TYPE)?;
    let modality_code = optional_text(object, MODALITY)?;
    let (units, multi_energy) = match modality_code.as_deref() {
        Some("PT") => (optional_text(object, PET_UNITS)?, None),
        Some("CT") => (None, optional_text(object, MULTI_ENERGY)?),
        _ => (None, None),
    };
    let (unit, unit_source) = modality_unit(
        modality_code.as_deref(),
        rescale_type.as_deref(),
        units.as_deref(),
        multi_energy.as_deref(),
    )?;
    Ok(ValueFacts {
        modality: modality.clone(),
        padding,
        unit,
        unit_source,
    })
}

/// The unit and the attribute it came from.
///
/// PS3.3 C.11.1.1.2 names the Rescale Type defined terms. On PT, Units
/// (0054,1001) names the unit whenever it is present, because PS3.3 C.8.9.4
/// defines the rescaled value in those units. PS3.3 C.8.2.1 makes Rescale
/// Type required on CT if it is not HU or Multi-energy CT Acquisition
/// (0018,9361) is YES, and gives HU only when that attribute is absent or
/// NO. So an absent Rescale Type on CT is HU only then. Multi-energy CT
/// Acquisition is a top-level attribute of the CT Image Module (C.8.2.1)
/// and of the Enhanced CT Image Module (C.8.15.2). Any other absent Rescale
/// Type names no unit.
fn modality_unit(
    modality: Option<&str>,
    rescale_type: Option<&str>,
    units: Option<&str>,
    multi_energy: Option<&str>,
) -> Result<(UnitText, &'static str), PreviewError> {
    Ok(match (rescale_type, modality, units) {
        (_, Some("PT"), Some(code)) => (UnitText::from_file(code)?, "units"),
        (Some("US"), _, _) => (UnitText::Rule("unspecified"), "rescale-type"),
        (Some(code), _, _) => (UnitText::from_file(code)?, "rescale-type"),
        (None, Some("CT"), _) if matches!(multi_energy, None | Some("NO")) => {
            (UnitText::Rule("HU"), "ct-default")
        }
        (None, _, _) => (UnitText::Rule("unspecified"), "none"),
    })
}

/// Decode one frame and read the value at each point.
///
/// This is the composition the `/value` route runs on a frame it does not
/// hold yet: the same decode as `preview_frame`, then the same reader of
/// the held frame.
///
/// # Errors
///
/// Returns [`PreviewError`] when the file or the frame cannot be read, when
/// the frame has no modality value this route can give, or when a point is
/// outside the image. A refusal returns no partial list.
pub fn value_frame(
    bytes: &[u8],
    frame: u32,
    points: &[ImagePoint],
) -> Result<String, PreviewError> {
    let mut native = Vec::new();
    let mut samples = Vec::new();
    let held = decode_held(
        bytes,
        frame,
        &mut native,
        &mut samples,
        &mut |_| {},
        &mut |_| Vec::new(),
    )?;
    value_json(&held, frame, points)
}

/// The value at each point of one held frame, as one JSON object.
///
/// # Errors
///
/// [`PreviewError::Point`] when any point is outside the image, then the
/// held value refusal, then a sample that cannot be read. Nothing is
/// returned for the other points.
pub(crate) fn value_json(
    held: &HeldFrame,
    frame: u32,
    points: &[ImagePoint],
) -> Result<String, PreviewError> {
    let mut cells = Vec::with_capacity(points.len());
    for point in points {
        let column = cell_index(point.x, held.width).ok_or(PreviewError::Point)?;
        let row = cell_index(point.y, held.height).ok_or(PreviewError::Point)?;
        cells.push((column, row));
    }
    let facts = held.value.as_ref().map_err(|error| *error)?;
    let mut out = String::from("{\"frame\":");
    out.push_str(&frame.to_string());
    out.push(',');
    push_field(&mut out, "unit", facts.unit.as_str(), true);
    push_field(&mut out, "unit_source", facts.unit_source, false);
    out.push_str(",\"points\":[");
    for (at, (column, row)) in cells.into_iter().enumerate() {
        if at > 0 {
            out.push(',');
        }
        let stored = stored_at(held, column, row)?;
        let padded = match facts.padding {
            Some(range) => range.contains(stored_integer(stored)?),
            None => false,
        };
        out.push_str("{\"column\":");
        out.push_str(&column.saturating_add(1).to_string());
        out.push_str(",\"row\":");
        out.push_str(&row.saturating_add(1).to_string());
        out.push_str(",\"index_column\":");
        out.push_str(&column.to_string());
        out.push_str(",\"index_row\":");
        out.push_str(&row.to_string());
        if padded {
            out.push_str(",\"padding\":true,\"text\":");
            json_string(&mut out, "padding");
        } else {
            let value = facts.modality.apply(stored).0;
            if !value.is_finite() {
                return Err(PreviewError::Attribute);
            }
            // Rust's `Display` for `f32` is the shortest text that reads
            // back as the same value, and it never uses an exponent.
            let number = format!("{value}");
            out.push_str(",\"padding\":false,\"value\":");
            out.push_str(&number);
            out.push_str(",\"text\":");
            json_string(
                &mut out,
                &format!("{number} {unit}", unit = facts.unit.as_str()),
            );
        }
        out.push('}');
    }
    out.push_str("]}");
    Ok(out)
}

/// The zero-based cell for one coordinate, or `None` outside `0..=extent`.
///
/// `floor(coordinate)`, except that `coordinate == extent` is the last cell.
fn cell_index(coordinate: f64, extent: u32) -> Option<u32> {
    if !coordinate.is_finite() || coordinate < 0.0 || coordinate > f64::from(extent) {
        return None;
    }
    // The floor is a whole number no larger than `extent`, which is at most
    // 65535 here, so narrowing it to `f32` is exact.
    let whole = exact_f32_integer(f32_from_f64(coordinate.floor())).ok()?;
    if whole.negative {
        return None;
    }
    let index = u32::try_from(whole.magnitude).ok()?;
    if index == extent {
        return extent.checked_sub(1);
    }
    Some(index)
}

/// Unpack the one stored sample at `(column, row)`.
///
/// PS3.5 8.1.1 and PS3.3 C.7.6.3.3. The same `unpack` the grey image uses,
/// over one sample: the bits above Bits Stored are masked, because PS3.5
/// 8.1.1 says nothing may be assumed about them, and a signed sample is
/// extended from its sign bit, the High Bit, which C.7.6.3.3 makes
/// `BitsStored - 1`.
pub(crate) fn stored_at(held: &HeldFrame, column: u32, row: u32) -> Result<Stored, PreviewError> {
    let width = usize::try_from(held.width).map_err(|_| PreviewError::Attribute)?;
    let column = usize::try_from(column).map_err(|_| PreviewError::Point)?;
    let row = usize::try_from(row).map_err(|_| PreviewError::Point)?;
    let bytes_per_sample = usize::from(held.description.bits.bits_allocated() / 8);
    if bytes_per_sample == 0 {
        return Err(PreviewError::Attribute);
    }
    let start = row
        .checked_mul(width)
        .and_then(|cells| cells.checked_add(column))
        .and_then(|cell| cell.checked_mul(bytes_per_sample))
        .ok_or(PreviewError::Decode)?;
    let end = start
        .checked_add(bytes_per_sample)
        .ok_or(PreviewError::Decode)?;
    let container = held.samples.get(start..end).ok_or(PreviewError::Decode)?;
    let one = ImageDimensions::new(1, 1).map_err(|_| PreviewError::Attribute)?;
    let single = StoredPixelDescription::new(one, held.description.layout, held.description.bits);
    let mut out = [Stored(0.0)];
    single
        .unpack(container, held.byte_order, &mut out)
        .map_err(|_| PreviewError::Decode)?;
    let [stored] = out;
    Ok(stored)
}

/// A stored sample as the integer it is, for the padding comparison.
pub(crate) fn stored_integer(stored: Stored) -> Result<i32, PreviewError> {
    let whole = exact_f32_integer(stored.0).map_err(|_| PreviewError::Decode)?;
    let magnitude = i64::try_from(whole.magnitude).map_err(|_| PreviewError::Decode)?;
    let signed = if whole.negative {
        magnitude.checked_neg().ok_or(PreviewError::Decode)?
    } else {
        magnitude
    };
    i32::try_from(signed).map_err(|_| PreviewError::Decode)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decoded(bytes: &[u8], frame: u32) -> Result<HeldFrame, String> {
        let mut native = Vec::new();
        let mut samples = Vec::new();
        decode_held(
            bytes,
            frame,
            &mut native,
            &mut samples,
            &mut |_| {},
            &mut |_| Vec::new(),
        )
        .map_err(|error| format!("frame {frame} was refused: {}", error.code()))
    }

    fn short_le(tag: (u16, u16), vr: [u8; 2], value: &[u8]) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&tag.0.to_le_bytes());
        bytes.extend_from_slice(&tag.1.to_le_bytes());
        bytes.extend_from_slice(&vr);
        let length = u16::try_from(value.len()).unwrap_or(0);
        bytes.extend_from_slice(&length.to_le_bytes());
        bytes.extend_from_slice(value);
        bytes
    }

    fn long_le(tag: (u16, u16), vr: [u8; 2], value: &[u8]) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&tag.0.to_le_bytes());
        bytes.extend_from_slice(&tag.1.to_le_bytes());
        bytes.extend_from_slice(&vr);
        bytes.extend_from_slice(&[0, 0]);
        let length = u32::try_from(value.len()).unwrap_or(0);
        bytes.extend_from_slice(&length.to_le_bytes());
        bytes.extend_from_slice(value);
        bytes
    }

    fn pad(text: &str) -> Vec<u8> {
        let mut value = text.as_bytes().to_vec();
        if !value.len().is_multiple_of(2) {
            value.push(b' ');
        }
        value
    }

    fn item(content: &[u8]) -> Vec<u8> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&0xFFFE_u16.to_le_bytes());
        bytes.extend_from_slice(&0xE000_u16.to_le_bytes());
        let length = u32::try_from(content.len()).unwrap_or(0);
        bytes.extend_from_slice(&length.to_le_bytes());
        bytes.extend_from_slice(content);
        bytes
    }

    /// One monochrome CT pixel per frame, explicit VR little endian.
    ///
    /// `rescale` is the top-level slope and intercept text, `""` for a
    /// zero-length element and `None` for absent ones. `groups` is the
    /// per-frame functional group sequence content, one item per frame.
    fn object(rescale: Option<(&str, &str)>, frames: u16, groups: Option<&[u8]>) -> Vec<u8> {
        let mut meta = Vec::new();
        meta.extend(long_le((0x0002, 0x0001), *b"OB", &[0, 1]));
        meta.extend(short_le(
            (0x0002, 0x0002),
            *b"UI",
            b"1.2.840.10008.5.1.4.1.1.2\0",
        ));
        meta.extend(short_le((0x0002, 0x0003), *b"UI", b"2.25.1"));
        meta.extend(short_le((0x0002, 0x0010), *b"UI", b"1.2.840.10008.1.2.1\0"));
        meta.extend(short_le((0x0002, 0x0012), *b"UI", b"2.25.2"));
        let mut data = Vec::new();
        data.extend(short_le(
            (0x0008, 0x0016),
            *b"UI",
            b"1.2.840.10008.5.1.4.1.1.2\0",
        ));
        data.extend(short_le((0x0008, 0x0060), *b"CS", b"CT"));
        data.extend(short_le((0x0028, 0x0002), *b"US", &1_u16.to_le_bytes()));
        data.extend(short_le((0x0028, 0x0004), *b"CS", &pad("MONOCHROME2")));
        if frames > 1 {
            data.extend(short_le(
                (0x0028, 0x0008),
                *b"IS",
                &pad(&frames.to_string()),
            ));
        }
        data.extend(short_le((0x0028, 0x0010), *b"US", &1_u16.to_le_bytes()));
        data.extend(short_le((0x0028, 0x0011), *b"US", &1_u16.to_le_bytes()));
        data.extend(short_le((0x0028, 0x0100), *b"US", &16_u16.to_le_bytes()));
        data.extend(short_le((0x0028, 0x0101), *b"US", &16_u16.to_le_bytes()));
        data.extend(short_le((0x0028, 0x0102), *b"US", &15_u16.to_le_bytes()));
        data.extend(short_le((0x0028, 0x0103), *b"US", &0_u16.to_le_bytes()));
        data.extend(short_le((0x0028, 0x1050), *b"DS", &pad("100")));
        data.extend(short_le((0x0028, 0x1051), *b"DS", &pad("200")));
        if let Some((slope, intercept)) = rescale {
            data.extend(short_le((0x0028, 0x1052), *b"DS", &pad(intercept)));
            data.extend(short_le((0x0028, 0x1053), *b"DS", &pad(slope)));
        }
        if let Some(groups) = groups {
            data.extend(long_le((0x5200, 0x9230), *b"SQ", groups));
        }
        let mut pixels = Vec::new();
        for _ in 0..frames {
            pixels.extend_from_slice(&50_u16.to_le_bytes());
        }
        data.extend(long_le((0x7FE0, 0x0010), *b"OW", &pixels));
        let mut bytes = vec![0_u8; 128];
        bytes.extend_from_slice(b"DICM");
        let length = u32::try_from(meta.len()).unwrap_or(0);
        bytes.extend(short_le((0x0002, 0x0000), *b"UL", &length.to_le_bytes()));
        bytes.extend(meta);
        bytes.extend(data);
        bytes
    }

    fn group(slope: &str, intercept: &str) -> Vec<u8> {
        let body = [
            short_le((0x0028, 0x1052), *b"DS", &pad(intercept)),
            short_le((0x0028, 0x1053), *b"DS", &pad(slope)),
        ]
        .concat();
        item(&long_le((0x0028, 0x9145), *b"SQ", &item(&body)))
    }

    /// AC12. The value's stage 1 and the preview's stage 1 are one transform.
    ///
    /// The objects are every synthetic slice, the top-level rescale shapes
    /// the `tests/preview.rs` objects use (`1` and `0`, zero length, absent),
    /// fractional decimals, and functional groups that override the top level.
    #[test]
    fn value_and_preview_agree_on_stage_one() -> Result<(), String> {
        let mut cases: Vec<(String, Vec<u8>, u32)> = Vec::new();
        for index in 0..crate::SLICES {
            let bytes = crate::slice(index).ok_or("a synthetic slice is missing")?;
            cases.push((format!("synthetic {index}"), bytes, 0));
        }
        for (name, rescale) in [
            ("slope 1 intercept 0", Some(("1", "0"))),
            ("zero length", Some(("", ""))),
            ("absent", None),
            ("fractional", Some(("0.3333", "-1024.5"))),
            ("small", Some(("0.00001", "0.1"))),
        ] {
            cases.push((name.to_owned(), object(rescale, 1, None), 0));
        }
        let groups = [group("1", "-1024"), group("2.5", "-1000")].concat();
        let enhanced = object(Some(("1", "-1024")), 2, Some(&groups));
        cases.push(("enhanced frame 0".to_owned(), enhanced.clone(), 0));
        cases.push(("enhanced frame 1".to_owned(), enhanced, 1));
        for (name, bytes, frame) in cases {
            let held = decoded(&bytes, frame).map_err(|error| format!("{name}: {error}"))?;
            let recipe = held
                .recipe
                .as_ref()
                .map_err(|error| format!("{name}: no recipe, {}", error.code()))?;
            let value = held
                .value
                .as_ref()
                .map_err(|error| format!("{name}: no value, {}", error.code()))?;
            if value.modality != recipe.modality {
                return Err(format!(
                    "{name}: value {:?} and preview {:?} differ",
                    value.modality.rescale(),
                    recipe.modality.rescale()
                ));
            }
        }
        Ok(())
    }

    #[test]
    fn an_owned_unit_is_charged_and_a_rule_unit_is_not() -> Result<(), String> {
        // A unit the file names is an owned byte run the cache must charge.
        // HU from the CT rule is a constant and owns no heap.
        let modality = ModalityTransform::new(None, Some(1.0), Some(0.0))
            .map_err(|_| "identity rescale was refused".to_owned())?;
        let facts = |unit: UnitText| ValueFacts {
            modality: modality.clone(),
            padding: None,
            unit,
            unit_source: "rescale-type",
        };
        let owned =
            facts(UnitText::from_file(&"A".repeat(40)).map_err(|error| error.code().to_owned())?);
        if owned.heap_bytes() != size_of::<ValueFacts>() + 40 {
            return Err(format!("an owned unit charged {}", owned.heap_bytes()));
        }
        // 64 characters is the LO limit of PS3.5 Table 6.2-1. U+10000 takes
        // 4 bytes in UTF-8 (RFC 3629), so 64 of them own 256 bytes, which the
        // claim's bound must still cover. A 65th character is refused.
        let widest = facts(
            UnitText::from_file(&"\u{10000}".repeat(64))
                .map_err(|error| error.code().to_owned())?,
        );
        if widest.unit_capacity() != 256 || widest.unit_capacity() > UNIT_MAX_BYTES {
            return Err(format!(
                "a 64-character unit owned {} bytes against a bound of {UNIT_MAX_BYTES}",
                widest.unit_capacity()
            ));
        }
        if UnitText::from_file(&"\u{10000}".repeat(65)).is_ok() {
            return Err("a 65-character unit was accepted".to_owned());
        }
        let borrowed = facts(UnitText::Rule("HU"));
        if borrowed.heap_bytes() != size_of::<ValueFacts>() {
            return Err(format!("a borrowed unit charged {}", borrowed.heap_bytes()));
        }
        Ok(())
    }

    #[test]
    fn a_cell_index_keeps_the_closed_right_edge() {
        assert_eq!(cell_index(0.0, 4), Some(0));
        assert_eq!(cell_index(-0.0, 4), Some(0));
        assert_eq!(cell_index(0.999, 4), Some(0));
        assert_eq!(cell_index(1.0, 4), Some(1));
        assert_eq!(cell_index(3.5, 4), Some(3));
        assert_eq!(cell_index(4.0, 4), Some(3));
        assert_eq!(cell_index(4.000_001, 4), None);
        assert_eq!(cell_index(-0.000_001, 4), None);
        assert_eq!(cell_index(f64::NAN, 4), None);
        assert_eq!(cell_index(65_535.0, 65_535), Some(65_534));
    }

    #[test]
    fn a_stored_integer_keeps_its_sign() {
        assert_eq!(stored_integer(Stored(-2000.0)), Ok(-2000));
        assert_eq!(stored_integer(Stored(65_535.0)), Ok(65_535));
        assert_eq!(stored_integer(Stored(0.5)), Err(PreviewError::Decode));
    }
}
