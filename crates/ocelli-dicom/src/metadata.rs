//! Lossless DICOM metadata values shared by Part 10 and DICOM JSON ingest.
//!
//! DICOM PS3.5 sections 6.2 and 7.1 define value representation, value
//! multiplicity, padding, and nested sequence items. Projection keeps those
//! distinctions observable instead of normalising them during ingest.

use std::collections::BTreeMap;

use dicom_core::{
    DicomValue, PrimitiveValue, Tag, VR,
    value::{DicomDate, DicomDateTime, DicomTime},
};
use dicom_object::{DefaultDicomObject, StandardDataDictionary, mem::InMemDicomObject};

/// A failure to construct or project lossless metadata.
///
/// Variants contain only structural information. They never retain attribute
/// values, object identifiers, or input bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum MetadataError {
    /// The set already contains this tag.
    #[error("metadata contains a duplicate tag")]
    DuplicateTag(Tag),
    /// Encapsulated Pixel Data is owned by the codec path, not metadata.
    #[error("encapsulated pixel fragments cannot be projected as metadata")]
    UnsupportedPixelFragments(Tag),
    /// The DICOM JSON InlineBinary carrier is not canonical base64.
    #[error("DICOM JSON InlineBinary is not valid canonical base64")]
    InvalidInlineBinary,
    /// DICOM JSON represents an empty element without an InlineBinary carrier.
    #[error("DICOM JSON InlineBinary cannot be empty")]
    EmptyInlineBinary,
    /// PS3.18 F.2.2 does not permit BulkDataURI for this VR.
    #[error("DICOM JSON BulkDataURI is not permitted for this VR")]
    InvalidBulkDataUriVr(VR),
    /// PS3.18 F.2.2 does not permit InlineBinary for this VR.
    #[error("DICOM JSON InlineBinary is not permitted for this VR")]
    InvalidInlineBinaryVr(VR),
    /// DICOM JSON null positions do not match the compact typed value.
    #[error("DICOM JSON null-slot metadata is inconsistent")]
    InvalidNullSlots,
}

/// One lossless metadata collection indexed by DICOM tag.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MetadataSet {
    elements: BTreeMap<Tag, MetadataElement>,
}

impl MetadataSet {
    /// Create an empty metadata set.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            elements: BTreeMap::new(),
        }
    }

    /// Project the main data set of a parsed Part 10 object.
    ///
    /// File Meta Information is intentionally separate and is exposed by the
    /// file-meta provider.
    ///
    /// # Errors
    ///
    /// Returns [`MetadataError::UnsupportedPixelFragments`] if an element is
    /// encapsulated Pixel Data. Those fragments stay on the codec path.
    pub fn from_object(object: &DefaultDicomObject) -> Result<Self, MetadataError> {
        Self::from_data_set(object)
    }

    /// Insert a tag exactly once.
    ///
    /// # Errors
    ///
    /// Returns [`MetadataError::DuplicateTag`] if the tag is already present.
    pub fn insert(&mut self, tag: Tag, element: MetadataElement) -> Result<(), MetadataError> {
        if self.elements.contains_key(&tag) {
            return Err(MetadataError::DuplicateTag(tag));
        }
        self.elements.insert(tag, element);
        Ok(())
    }

    /// Look up an element without inventing a default for absence.
    #[must_use]
    pub fn get(&self, tag: Tag) -> Option<&MetadataElement> {
        self.elements.get(&tag)
    }

    /// Iterate in ascending tag order.
    pub fn iter(&self) -> impl Iterator<Item = (Tag, &MetadataElement)> {
        self.elements.iter().map(|(tag, element)| (*tag, element))
    }

    /// Number of present tags, including present empty elements.
    #[must_use]
    pub fn len(&self) -> usize {
        self.elements.len()
    }

    /// Whether this set contains no tags.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }

    fn from_data_set(
        object: &InMemDicomObject<StandardDataDictionary>,
    ) -> Result<Self, MetadataError> {
        let mut metadata = Self::new();
        for element in object {
            let tag = element.header().tag;
            metadata.insert(tag, MetadataElement::from_dicom_element(tag, element)?)?;
        }
        Ok(metadata)
    }
}

/// One present DICOM element, including its declared VR.
#[derive(Debug, Clone, PartialEq)]
pub struct MetadataElement {
    vr: VR,
    value: MetadataValue,
}

impl MetadataElement {
    /// Construct a present element from an explicit VR and value inside the
    /// module after its carrier invariants have been checked.
    #[must_use]
    const fn new(vr: VR, value: MetadataValue) -> Self {
        Self { vr, value }
    }

    /// Construct a present zero-length element.
    #[must_use]
    pub const fn empty(vr: VR) -> Self {
        Self::new(vr, MetadataValue::Empty)
    }

    /// Construct the typed DICOM JSON Person Name carrier.
    #[must_use]
    pub fn person_names(values: Vec<PersonName>) -> Self {
        Self::new(VR::PN, MetadataValue::PersonNames(values))
    }

    /// Construct an ordered DICOM JSON sequence value.
    #[must_use]
    pub fn sequence(items: Vec<MetadataSet>) -> Self {
        Self::new(VR::SQ, MetadataValue::Sequence(items))
    }

    /// Retain null positions around an existing compact typed Value array.
    ///
    /// The empty indices must be strictly increasing and in range. Their
    /// complement must have exactly the multiplicity of the existing value.
    /// SQ, binary carriers, nested null-slot wrappers, and no-slot wrappers
    /// are refused.
    ///
    /// # Errors
    ///
    /// Returns [`MetadataError::InvalidNullSlots`] for inconsistent metadata
    /// or a value kind that DICOM JSON cannot wrap with null slots.
    pub fn with_null_slots(
        self,
        value_count: usize,
        empty_slots: Vec<usize>,
    ) -> Result<Self, MetadataError> {
        let present_count = self.value.value_count_for_null_slots();
        let indices_are_valid = !empty_slots.is_empty()
            && value_count > 0
            && empty_slots.iter().enumerate().all(|(position, index)| {
                *index < value_count
                    && (position == 0 || empty_slots.get(position - 1) < Some(index))
            });
        if !indices_are_valid
            || present_count != Some(value_count.saturating_sub(empty_slots.len()))
        {
            return Err(MetadataError::InvalidNullSlots);
        }
        Ok(Self::new(
            self.vr,
            MetadataValue::WithNullSlots(NullSlots {
                value_count,
                empty_slots,
                present_values: Box::new(self.value),
            }),
        ))
    }

    /// Construct the typed DICOM JSON BulkDataURI carrier.
    ///
    /// # Errors
    ///
    /// Returns [`MetadataError::InvalidBulkDataUriVr`] unless PS3.18 F.2.2
    /// permits BulkDataURI for the supplied VR.
    pub fn bulk_data_uri(vr: VR, uri: String) -> Result<Self, MetadataError> {
        if !bulk_data_uri_vr_is_allowed(vr) {
            return Err(MetadataError::InvalidBulkDataUriVr(vr));
        }
        Ok(Self::new(
            vr,
            MetadataValue::BulkDataUri(BulkDataUri::new(uri)),
        ))
    }

    /// Construct and validate the typed DICOM JSON InlineBinary carrier.
    ///
    /// # Errors
    ///
    /// Returns a VR error unless PS3.18 F.2.2 permits InlineBinary for the
    /// supplied VR. An empty carrier is refused because PS3.18 F.2.5
    /// represents a present empty attribute with no carrier.
    pub fn inline_binary(vr: VR, encoded: String) -> Result<Self, MetadataError> {
        if !inline_binary_vr_is_allowed(vr) {
            return Err(MetadataError::InvalidInlineBinaryVr(vr));
        }
        Ok(Self::new(
            vr,
            MetadataValue::InlineBinary(InlineBinary::new(encoded)?),
        ))
    }

    /// Construct an element from a preserved dicom-rs primitive value.
    #[must_use]
    pub fn from_primitive(vr: VR, value: &PrimitiveValue) -> Self {
        Self::new(vr, MetadataValue::from_primitive(value))
    }

    /// The declared DICOM value representation.
    #[must_use]
    pub const fn vr(&self) -> VR {
        self.vr
    }

    /// The lossless value, including an explicit empty state.
    #[must_use]
    pub const fn value(&self) -> &MetadataValue {
        &self.value
    }

    /// Produce the PS3.5 section 6.2 semantic view of retained text.
    ///
    /// The source spelling remains unchanged. VRs that declare leading space
    /// insignificant trim it. ST, LT, UT, and other trailing-pad-only VRs
    /// retain leading spaces while dropping only their legal trailing pad.
    #[must_use]
    pub fn semantic_text(&self) -> Option<Vec<&str>> {
        let MetadataValue::Text(values) = &self.value else {
            return None;
        };
        Some(
            values
                .iter()
                .map(|value| match self.vr {
                    VR::AE | VR::CS | VR::DS | VR::IS | VR::LO | VR::PN | VR::SH => {
                        value.trim_matches(' ')
                    }
                    VR::UI => value.trim_end_matches('\0'),
                    _ => value.trim_end_matches(' '),
                })
                .collect(),
        )
    }

    pub(crate) fn from_dicom_element(
        tag: Tag,
        element: &dicom_object::mem::InMemElement<StandardDataDictionary>,
    ) -> Result<Self, MetadataError> {
        let value = match element.value() {
            DicomValue::Primitive(value) => MetadataValue::from_primitive(value),
            DicomValue::Sequence(sequence) => {
                let items = sequence
                    .items()
                    .iter()
                    .map(MetadataSet::from_data_set)
                    .collect::<Result<Vec<_>, _>>()?;
                MetadataValue::Sequence(items)
            }
            DicomValue::PixelSequence(_) => {
                return Err(MetadataError::UnsupportedPixelFragments(tag));
            }
        };
        Ok(Self::new(element.vr(), value))
    }
}

/// A lossless DICOM element value.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum MetadataValue {
    /// A present zero-length element.
    Empty,
    /// Preserved text components, including legal leading and trailing pad.
    Text(Vec<String>),
    /// Attribute tag values.
    Tags(Vec<Tag>),
    /// OB or UN bytes.
    Bytes(Vec<u8>),
    /// Signed 16-bit values.
    Signed16(Vec<i16>),
    /// Unsigned 16-bit values, including OW payloads.
    Unsigned16(Vec<u16>),
    /// Signed 32-bit values.
    Signed32(Vec<i32>),
    /// Unsigned 32-bit values, including OL payloads.
    Unsigned32(Vec<u32>),
    /// Signed 64-bit values.
    Signed64(Vec<i64>),
    /// Unsigned 64-bit values, including OV payloads.
    Unsigned64(Vec<u64>),
    /// 32-bit floating-point values.
    Float32(Vec<f32>),
    /// 64-bit floating-point values.
    Float64(Vec<f64>),
    /// Date values with their source precision.
    Dates(Vec<DicomDate>),
    /// Date-time values with their source precision and offset.
    DateTimes(Vec<DicomDateTime>),
    /// Time values with their source precision.
    Times(Vec<DicomTime>),
    /// Ordered nested data-set items.
    Sequence(Vec<MetadataSet>),
    /// DICOM JSON Person Name component objects.
    PersonNames(Vec<PersonName>),
    /// DICOM JSON BulkDataURI, retained separately from text values.
    BulkDataUri(BulkDataUri),
    /// Validated DICOM JSON InlineBinary, retained separately from text.
    InlineBinary(InlineBinary),
    /// Typed DICOM JSON values plus their ordered null positions.
    WithNullSlots(NullSlots),
}

impl MetadataValue {
    /// Project a preserved dicom-rs primitive without numeric coercion.
    #[must_use]
    pub fn from_primitive(value: &PrimitiveValue) -> Self {
        match value {
            PrimitiveValue::Empty => Self::Empty,
            PrimitiveValue::Strs(values) => Self::Text(values.iter().cloned().collect()),
            PrimitiveValue::Str(value) => Self::Text(vec![value.clone()]),
            PrimitiveValue::Tags(values) => Self::Tags(values.iter().copied().collect()),
            PrimitiveValue::U8(values) => Self::Bytes(values.to_vec()),
            PrimitiveValue::I16(values) => Self::Signed16(values.to_vec()),
            PrimitiveValue::U16(values) => Self::Unsigned16(values.to_vec()),
            PrimitiveValue::I32(values) => Self::Signed32(values.to_vec()),
            PrimitiveValue::U32(values) => Self::Unsigned32(values.to_vec()),
            PrimitiveValue::I64(values) => Self::Signed64(values.to_vec()),
            PrimitiveValue::U64(values) => Self::Unsigned64(values.to_vec()),
            PrimitiveValue::F32(values) => Self::Float32(values.to_vec()),
            PrimitiveValue::F64(values) => Self::Float64(values.to_vec()),
            PrimitiveValue::Date(values) => Self::Dates(values.iter().copied().collect()),
            PrimitiveValue::DateTime(values) => Self::DateTimes(values.iter().copied().collect()),
            PrimitiveValue::Time(values) => Self::Times(values.iter().copied().collect()),
        }
    }

    fn value_count_for_null_slots(&self) -> Option<usize> {
        match self {
            Self::Text(values) => Some(values.len()),
            Self::Tags(values) => Some(values.len()),
            Self::Signed16(values) => Some(values.len()),
            Self::Unsigned16(values) => Some(values.len()),
            Self::Signed32(values) => Some(values.len()),
            Self::Unsigned32(values) => Some(values.len()),
            Self::Signed64(values) => Some(values.len()),
            Self::Unsigned64(values) => Some(values.len()),
            Self::Float32(values) => Some(values.len()),
            Self::Float64(values) => Some(values.len()),
            Self::Dates(values) => Some(values.len()),
            Self::DateTimes(values) => Some(values.len()),
            Self::Times(values) => Some(values.len()),
            Self::PersonNames(values) => Some(values.len()),
            Self::Empty
            | Self::Bytes(_)
            | Self::Sequence(_)
            | Self::BulkDataUri(_)
            | Self::InlineBinary(_)
            | Self::WithNullSlots(_) => None,
        }
    }
}

/// The ordered null positions around a compact typed DICOM JSON value.
#[derive(Debug, Clone, PartialEq)]
pub struct NullSlots {
    value_count: usize,
    empty_slots: Vec<usize>,
    present_values: Box<MetadataValue>,
}

impl NullSlots {
    /// Total multiplicity, including null positions.
    #[must_use]
    pub const fn value_count(&self) -> usize {
        self.value_count
    }

    /// Strictly increasing positions whose JSON Value was null.
    #[must_use]
    pub fn empty_slots(&self) -> &[usize] {
        &self.empty_slots
    }

    /// Compact typed values in their original relative order.
    #[must_use]
    pub const fn present_values(&self) -> &MetadataValue {
        &self.present_values
    }

    /// Whether one original in-range position was null.
    #[must_use]
    pub fn is_empty_slot(&self, slot: usize) -> Option<bool> {
        (slot < self.value_count).then(|| self.empty_slots.binary_search(&slot).is_ok())
    }

    /// Map an original slot to its compact typed-value index.
    #[must_use]
    pub fn present_index(&self, slot: usize) -> Option<usize> {
        if slot >= self.value_count || self.empty_slots.binary_search(&slot).is_ok() {
            return None;
        }
        Some(slot - self.empty_slots.partition_point(|empty| *empty < slot))
    }
}

/// One DICOM JSON Person Name component object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersonName {
    alphabetic: Option<String>,
    ideographic: Option<String>,
    phonetic: Option<String>,
}

impl PersonName {
    /// Construct a Person Name without collapsing its component groups.
    #[must_use]
    pub const fn new(
        alphabetic: Option<String>,
        ideographic: Option<String>,
        phonetic: Option<String>,
    ) -> Self {
        Self {
            alphabetic,
            ideographic,
            phonetic,
        }
    }

    /// The alphabetic component group, if supplied.
    #[must_use]
    pub fn alphabetic(&self) -> Option<&str> {
        self.alphabetic.as_deref()
    }

    /// The ideographic component group, if supplied.
    #[must_use]
    pub fn ideographic(&self) -> Option<&str> {
        self.ideographic.as_deref()
    }

    /// The phonetic component group, if supplied.
    #[must_use]
    pub fn phonetic(&self) -> Option<&str> {
        self.phonetic.as_deref()
    }
}

/// A DICOM JSON BulkDataURI carrier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BulkDataUri(String);

impl BulkDataUri {
    /// Retain the URI spelling supplied by DICOM JSON.
    #[must_use]
    const fn new(uri: String) -> Self {
        Self(uri)
    }

    /// The retained URI spelling.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A canonical padded-base64 DICOM JSON InlineBinary carrier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InlineBinary(String);

impl InlineBinary {
    /// Validate and retain an InlineBinary spelling.
    ///
    /// # Errors
    ///
    /// Returns [`MetadataError::InvalidInlineBinary`] for a non-base64 byte,
    /// misplaced padding, or non-zero unused bits.
    fn new(encoded: String) -> Result<Self, MetadataError> {
        if encoded.is_empty() {
            return Err(MetadataError::EmptyInlineBinary);
        }
        if is_canonical_base64(encoded.as_bytes()) {
            Ok(Self(encoded))
        } else {
            Err(MetadataError::InvalidInlineBinary)
        }
    }

    /// The validated base64 spelling.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

const fn bulk_data_uri_vr_is_allowed(vr: VR) -> bool {
    matches!(
        vr,
        VR::DS
            | VR::FL
            | VR::FD
            | VR::IS
            | VR::LT
            | VR::OB
            | VR::OD
            | VR::OF
            | VR::OL
            | VR::OV
            | VR::OW
            | VR::SL
            | VR::SS
            | VR::ST
            | VR::SV
            | VR::UC
            | VR::UL
            | VR::UN
            | VR::US
            | VR::UT
            | VR::UV
    )
}

const fn inline_binary_vr_is_allowed(vr: VR) -> bool {
    matches!(
        vr,
        VR::OB | VR::OD | VR::OF | VR::OL | VR::OV | VR::OW | VR::UN
    )
}

fn is_canonical_base64(bytes: &[u8]) -> bool {
    if !bytes.len().is_multiple_of(4) {
        return false;
    }
    if bytes.is_empty() {
        return true;
    }

    let padding = bytes.iter().rev().take_while(|byte| **byte == b'=').count();
    if padding > 2 {
        return false;
    }
    let data_length = bytes.len() - padding;
    let Some(data) = bytes.get(..data_length) else {
        return false;
    };
    let Some(padding_bytes) = bytes.get(data_length..) else {
        return false;
    };
    if data.iter().any(|byte| base64_value(*byte).is_none())
        || padding_bytes.iter().any(|byte| *byte != b'=')
    {
        return false;
    }

    match padding {
        0 => true,
        1 => bytes
            .get(data_length - 1)
            .and_then(|byte| base64_value(*byte))
            .is_some_and(|value| value & 0b0000_0011 == 0),
        2 => bytes
            .get(data_length - 1)
            .and_then(|byte| base64_value(*byte))
            .is_some_and(|value| value & 0b0000_1111 == 0),
        _ => false,
    }
}

fn base64_value(byte: u8) -> Option<u8> {
    match byte {
        b'A'..=b'Z' => Some(byte - b'A'),
        b'a'..=b'z' => Some(byte - b'a' + 26),
        b'0'..=b'9' => Some(byte - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::InlineBinary;

    #[test]
    fn inline_binary_requires_canonical_padding_and_unused_bits() {
        for valid in ["AA==", "AAE=", "AAEC", "////"] {
            assert!(InlineBinary::new(valid.to_owned()).is_ok(), "{valid}");
        }
        for invalid in ["", "A", "AAA", "A===", "AA=A", "A/==", "AAF="] {
            assert!(InlineBinary::new(invalid.to_owned()).is_err(), "{invalid}");
        }
    }
}
