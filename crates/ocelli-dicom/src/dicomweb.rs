//! Transport-neutral DICOMweb response parsing.
//!
//! TypeScript owns HTTP. This module consumes validated response bytes and an
//! explicit representation kind, then projects them into the same metadata
//! and Part 10 types used by the rest of the crate.

use std::{fmt, ops::Range, str::FromStr};

use dicom_core::{PrimitiveValue, VR};
use serde::{
    Deserialize,
    de::{self, MapAccess, SeqAccess, Visitor},
};
use serde_json::{Map, Number, Value};
use uriparse::URIReference;

use crate::{MetadataElement, MetadataSet, ParseError, ParsedDicom, PersonName, Tag, parse_part10};

/// The representation selected and validated by the transport layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceResponseKind {
    /// A QIDO-RS `application/dicom+json` response.
    QidoJson,
    /// One WADO-URI `application/dicom` Part 10 object.
    WadoUriPart10,
    /// A WADO-RS multipart response containing Part 10 instances.
    WadoRsInstances {
        /// The declared outer multipart boundary.
        boundary: String,
    },
    /// A WADO-RS multipart response containing encoded frames.
    WadoRsFrames {
        /// The declared outer multipart boundary.
        boundary: String,
        /// The requested and validated part media type.
        media_type: String,
    },
}

/// Complete bytes for one transport response.
#[derive(Debug)]
pub struct SourceResponse {
    kind: SourceResponseKind,
    bytes: Vec<u8>,
}

impl SourceResponse {
    /// Pair response bytes with their transport-validated representation.
    #[must_use]
    pub const fn new(kind: SourceResponseKind, bytes: Vec<u8>) -> Self {
        Self { kind, bytes }
    }
}

/// One source operation result.
#[derive(Debug)]
pub enum SourceBatch {
    /// Ordered QIDO result data sets.
    Query(Vec<MetadataSet>),
    /// One or more parsed DICOM Part 10 instances.
    Instances(Vec<ParsedDicom>),
    /// Encoded frame payloads retained as ranges into one owned response.
    Frames(EncodedFrames),
}

/// A validated encoded frame part.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EncodedFramePart {
    media_type: String,
    range: Range<usize>,
}

impl EncodedFramePart {
    /// The complete validated per-part media type, including parameters.
    #[must_use]
    pub fn media_type(&self) -> &str {
        &self.media_type
    }

    /// The per-part transfer-syntax parameter, when declared.
    #[must_use]
    pub fn transfer_syntax(&self) -> Option<&str> {
        media_type_parameter(&self.media_type, "transfer-syntax")
    }

    /// The payload range in [`EncodedFrames::bytes`].
    #[must_use]
    pub const fn range(&self) -> &Range<usize> {
        &self.range
    }
}

/// Ordered zero-copy views over one owned frame response.
#[derive(Debug)]
pub struct EncodedFrames {
    bytes: Vec<u8>,
    parts: Vec<EncodedFramePart>,
}

impl EncodedFrames {
    /// The complete owned multipart response.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Ordered frame-part metadata.
    #[must_use]
    pub fn parts(&self) -> &[EncodedFramePart] {
        &self.parts
    }

    /// Borrow one encoded frame payload without copying it.
    #[must_use]
    pub fn part_bytes(&self, index: usize) -> Option<&[u8]> {
        let range = self.parts.get(index)?.range.clone();
        self.bytes.get(range)
    }
}

/// A patient-safe response parsing failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum SourceError {
    /// JSON text is not syntactically valid.
    #[error("DICOM JSON is invalid")]
    InvalidJson,
    /// A QIDO response is not a top-level array of objects.
    #[error("DICOM JSON has an invalid root")]
    InvalidJsonRoot,
    /// An attribute key is not an uppercase eight-digit hexadecimal tag.
    #[error("DICOM JSON contains an invalid attribute tag")]
    InvalidJsonTag { attribute: usize },
    /// An attribute has no valid explicit VR.
    #[error("DICOM JSON contains an invalid attribute VR")]
    InvalidJsonVr { attribute: usize },
    /// An attribute has an invalid or conflicting value carrier.
    #[error("DICOM JSON contains an invalid value carrier")]
    InvalidJsonCarrier { attribute: usize },
    /// An attribute Value does not match its declared VR.
    #[error("DICOM JSON contains an invalid attribute value")]
    InvalidJsonValue { attribute: usize },
    /// A multipart boundary is absent or syntactically invalid.
    #[error("DICOM multipart boundary is invalid")]
    InvalidMultipartBoundary,
    /// Multipart delimiter lines or part headers are malformed.
    #[error("DICOM multipart framing is invalid")]
    InvalidMultipart,
    /// A part does not use the required media type.
    #[error("DICOM multipart part has an incompatible media type")]
    InvalidPartMediaType { part: usize },
    /// One DICOM Part 10 payload is invalid.
    #[error("DICOM Part 10 response payload is invalid")]
    InvalidPart10 { part: usize, error: ParseError },
}

/// A transport-neutral source of query metadata, instances, and frame bytes.
pub trait SeriesSource {
    /// Consume one already-fetched response.
    ///
    /// # Errors
    ///
    /// Returns a structural [`SourceError`] without retaining response values.
    fn consume(&self, response: SourceResponse) -> Result<SourceBatch, SourceError>;
}

/// The pure response parser used by the TypeScript DICOMweb transport.
#[derive(Debug, Clone, Copy, Default)]
pub struct DicomwebSource;

impl SeriesSource for DicomwebSource {
    fn consume(&self, response: SourceResponse) -> Result<SourceBatch, SourceError> {
        match response.kind {
            SourceResponseKind::QidoJson => {
                parse_qido_json(&response.bytes).map(SourceBatch::Query)
            }
            SourceResponseKind::WadoUriPart10 => parse_part10(&response.bytes)
                .map(|object| SourceBatch::Instances(vec![object]))
                .map_err(|error| SourceError::InvalidPart10 { part: 0, error }),
            SourceResponseKind::WadoRsInstances { boundary } => {
                let parts = parse_multipart(&response.bytes, &boundary)?;
                let mut instances = Vec::with_capacity(parts.len());
                for (index, part) in parts.iter().enumerate() {
                    if !media_type_is(&part.media_type, "application/dicom") {
                        return Err(SourceError::InvalidPartMediaType { part: index });
                    }
                    let bytes = response
                        .bytes
                        .get(part.range.clone())
                        .ok_or(SourceError::InvalidMultipart)?;
                    instances.push(
                        parse_part10(bytes)
                            .map_err(|error| SourceError::InvalidPart10 { part: index, error })?,
                    );
                }
                Ok(SourceBatch::Instances(instances))
            }
            SourceResponseKind::WadoRsFrames {
                boundary,
                media_type,
            } => {
                let parts = parse_multipart(&response.bytes, &boundary)?;
                let mut frames = Vec::with_capacity(parts.len());
                for (index, part) in parts.into_iter().enumerate() {
                    if !media_type_is(&part.media_type, &media_type) {
                        return Err(SourceError::InvalidPartMediaType { part: index });
                    }
                    frames.push(EncodedFramePart {
                        media_type: part.media_type,
                        range: part.range,
                    });
                }
                Ok(SourceBatch::Frames(EncodedFrames {
                    bytes: response.bytes,
                    parts: frames,
                }))
            }
        }
    }
}

#[derive(Debug)]
struct MultipartPart {
    media_type: String,
    range: Range<usize>,
}

struct PartHeaders {
    media_type: String,
    content_length: Option<usize>,
}

fn parse_multipart(bytes: &[u8], boundary: &str) -> Result<Vec<MultipartPart>, SourceError> {
    if !valid_boundary(boundary) {
        return Err(SourceError::InvalidMultipartBoundary);
    }
    let marker = [b"--".as_slice(), boundary.as_bytes()].concat();
    if !bytes.starts_with(&marker) {
        return Err(SourceError::InvalidMultipart);
    }
    let mut cursor = marker.len();
    let mut parts = Vec::new();
    loop {
        if bytes.get(cursor..cursor + 2) == Some(b"--") {
            cursor += 2;
            if bytes
                .get(cursor..)
                .is_some_and(|tail| tail.is_empty() || tail == b"\r\n")
            {
                return if parts.is_empty() {
                    Err(SourceError::InvalidMultipart)
                } else {
                    Ok(parts)
                };
            }
            return Err(SourceError::InvalidMultipart);
        }
        if bytes.get(cursor..cursor + 2) != Some(b"\r\n") {
            return Err(SourceError::InvalidMultipart);
        }
        cursor += 2;
        let header_end =
            find_from(bytes, b"\r\n\r\n", cursor).ok_or(SourceError::InvalidMultipart)?;
        let headers = parse_part_headers(
            bytes
                .get(cursor..header_end)
                .ok_or(SourceError::InvalidMultipart)?,
        )?;
        let payload_start = header_end + 4;
        let delimiter = [b"\r\n--".as_slice(), boundary.as_bytes()].concat();
        let mut search = payload_start;
        let (payload_end, after_marker) = loop {
            let candidate =
                find_from(bytes, &delimiter, search).ok_or(SourceError::InvalidMultipart)?;
            let after = candidate + delimiter.len();
            let valid_suffix = matches!(bytes.get(after..after + 2), Some(b"\r\n" | b"--"));
            if valid_suffix {
                break (candidate, after);
            }
            search = candidate + 1;
        };
        if headers
            .content_length
            .is_some_and(|length| length != payload_end - payload_start)
        {
            return Err(SourceError::InvalidMultipart);
        }
        parts.push(MultipartPart {
            media_type: headers.media_type,
            range: payload_start..payload_end,
        });
        cursor = after_marker;
    }
}

fn find_from(haystack: &[u8], needle: &[u8], start: usize) -> Option<usize> {
    let tail = haystack.get(start..)?;
    tail.windows(needle.len())
        .position(|window| window == needle)
        .map(|position| start + position)
}

fn parse_part_headers(bytes: &[u8]) -> Result<PartHeaders, SourceError> {
    let text = std::str::from_utf8(bytes).map_err(|_| SourceError::InvalidMultipart)?;
    let headers = text
        .split("\r\n")
        .map(|line| {
            line.split_once(':')
                .map(|(name, value)| (name, value.trim()))
                .ok_or(SourceError::InvalidMultipart)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let required = headers.get(..3).ok_or(SourceError::InvalidMultipart)?;
    let [content_type, content_location, length_or_encoding] = required else {
        return Err(SourceError::InvalidMultipart);
    };
    if !content_type.0.eq_ignore_ascii_case("content-type")
        || !content_location.0.eq_ignore_ascii_case("content-location")
        || !(length_or_encoding.0.eq_ignore_ascii_case("content-length")
            || length_or_encoding
                .0
                .eq_ignore_ascii_case("transfer-encoding"))
        || required.iter().any(|(_, value)| value.is_empty())
        || URIReference::try_from(content_location.1).is_err()
    {
        return Err(SourceError::InvalidMultipart);
    }
    for (index, (name, _value)) in headers.iter().enumerate() {
        if (index >= 3 && matches_required_header(name))
            || headers
                .iter()
                .take(index)
                .any(|(previous, _)| previous.eq_ignore_ascii_case(name))
        {
            return Err(SourceError::InvalidMultipart);
        }
    }
    let content_length = if length_or_encoding.0.eq_ignore_ascii_case("content-length") {
        if !length_or_encoding
            .1
            .bytes()
            .all(|byte| byte.is_ascii_digit())
        {
            return Err(SourceError::InvalidMultipart);
        }
        Some(
            length_or_encoding
                .1
                .parse::<usize>()
                .map_err(|_| SourceError::InvalidMultipart)?,
        )
    } else {
        None
    };
    Ok(PartHeaders {
        media_type: content_type.1.to_owned(),
        content_length,
    })
}

fn matches_required_header(name: &str) -> bool {
    [
        "content-type",
        "content-location",
        "content-length",
        "transfer-encoding",
    ]
    .iter()
    .any(|required| name.eq_ignore_ascii_case(required))
}

fn valid_boundary(boundary: &str) -> bool {
    let bytes = boundary.as_bytes();
    !bytes.is_empty()
        && bytes.len() <= 70
        && bytes.last() != Some(&b' ')
        && bytes.iter().all(|byte| {
            byte.is_ascii_alphanumeric()
                || matches!(
                    byte,
                    b'\''
                        | b'('
                        | b')'
                        | b'+'
                        | b'_'
                        | b','
                        | b'-'
                        | b'.'
                        | b'/'
                        | b':'
                        | b'='
                        | b'?'
                        | b' '
                )
        })
}

fn media_type_is(actual: &str, expected: &str) -> bool {
    normalized_media_type(actual) == normalized_media_type(expected)
}

fn normalized_media_type(media_type: &str) -> Option<(String, Vec<(String, String)>)> {
    let mut parts = media_type.split(';');
    let base = parts.next()?.trim().to_ascii_lowercase();
    if !base.contains('/') {
        return None;
    }
    let mut parameters = parts
        .map(|part| {
            let (name, value) = part.split_once('=')?;
            let name = name.trim().to_ascii_lowercase();
            let value = value.trim().trim_matches('"').to_ascii_lowercase();
            (!name.is_empty() && !value.is_empty()).then_some((name, value))
        })
        .collect::<Option<Vec<_>>>()?;
    parameters.sort();
    Some((base, parameters))
}

fn media_type_parameter<'a>(media_type: &'a str, requested: &str) -> Option<&'a str> {
    media_type.split(';').skip(1).find_map(|part| {
        let (name, value) = part.split_once('=')?;
        name.trim()
            .eq_ignore_ascii_case(requested)
            .then_some(value.trim().trim_matches('"'))
    })
}

fn parse_qido_json(bytes: &[u8]) -> Result<Vec<MetadataSet>, SourceError> {
    let DuplicateCheckedValue(value) =
        serde_json::from_slice(bytes).map_err(|_| SourceError::InvalidJson)?;
    let rows = value.as_array().ok_or(SourceError::InvalidJsonRoot)?;
    rows.iter()
        .map(|row| {
            row.as_object()
                .ok_or(SourceError::InvalidJsonRoot)
                .and_then(parse_data_set)
        })
        .collect()
}

struct DuplicateCheckedValue(Value);

impl<'de> Deserialize<'de> for DuplicateCheckedValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(DuplicateCheckedVisitor)
    }
}

struct DuplicateCheckedVisitor;

impl<'de> Visitor<'de> for DuplicateCheckedVisitor {
    type Value = DuplicateCheckedValue;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a JSON value without duplicate object members")
    }

    fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E> {
        Ok(DuplicateCheckedValue(Value::Bool(value)))
    }

    fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E> {
        Ok(DuplicateCheckedValue(Value::Number(value.into())))
    }

    fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E> {
        Ok(DuplicateCheckedValue(Value::Number(value.into())))
    }

    fn visit_f64<E>(self, value: f64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Number::from_f64(value)
            .map(Value::Number)
            .map(DuplicateCheckedValue)
            .ok_or_else(|| E::custom("JSON number is not finite"))
    }

    fn visit_str<E>(self, value: &str) -> Result<Self::Value, E> {
        Ok(DuplicateCheckedValue(Value::String(value.to_owned())))
    }

    fn visit_string<E>(self, value: String) -> Result<Self::Value, E> {
        Ok(DuplicateCheckedValue(Value::String(value)))
    }

    fn visit_none<E>(self) -> Result<Self::Value, E> {
        Ok(DuplicateCheckedValue(Value::Null))
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E> {
        Ok(DuplicateCheckedValue(Value::Null))
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut values = Vec::new();
        while let Some(DuplicateCheckedValue(value)) = sequence.next_element()? {
            values.push(value);
        }
        Ok(DuplicateCheckedValue(Value::Array(values)))
    }

    fn visit_map<A>(self, mut entries: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut object = Map::new();
        while let Some(key) = entries.next_key::<String>()? {
            if object.contains_key(&key) {
                return Err(de::Error::custom("duplicate JSON object member"));
            }
            let DuplicateCheckedValue(value) = entries.next_value()?;
            object.insert(key, value);
        }
        Ok(DuplicateCheckedValue(Value::Object(object)))
    }
}

fn parse_data_set(object: &Map<String, Value>) -> Result<MetadataSet, SourceError> {
    let mut data_set = MetadataSet::new();
    for (attribute, (key, value)) in object.iter().enumerate() {
        let tag = parse_attribute_tag(key).ok_or(SourceError::InvalidJsonTag { attribute })?;
        let element = parse_element(value, attribute)?;
        data_set
            .insert(tag, element)
            .map_err(|_| SourceError::InvalidJsonCarrier { attribute })?;
    }
    Ok(data_set)
}

fn parse_tag(value: &str) -> Option<Tag> {
    let bytes = value.as_bytes();
    if bytes.len() != 8
        || bytes
            .iter()
            .any(|byte| !byte.is_ascii_digit() && !(b'A'..=b'F').contains(byte))
    {
        return None;
    }
    let group = u16::from_str_radix(value.get(..4)?, 16).ok()?;
    let element = u16::from_str_radix(value.get(4..)?, 16).ok()?;
    Some(Tag(group, element))
}

fn parse_attribute_tag(value: &str) -> Option<Tag> {
    parse_tag(value).filter(|tag| tag.1 != 0)
}

fn parse_element(value: &Value, attribute: usize) -> Result<MetadataElement, SourceError> {
    let object = value
        .as_object()
        .ok_or(SourceError::InvalidJsonCarrier { attribute })?;
    if object.keys().any(|key| {
        !matches!(
            key.as_str(),
            "vr" | "Value" | "BulkDataURI" | "InlineBinary"
        )
    }) {
        return Err(SourceError::InvalidJsonCarrier { attribute });
    }
    let vr = object
        .get("vr")
        .and_then(Value::as_str)
        .and_then(|text| VR::from_str(text).ok())
        .ok_or(SourceError::InvalidJsonVr { attribute })?;
    let carriers = ["Value", "BulkDataURI", "InlineBinary"]
        .iter()
        .filter(|name| object.contains_key(**name))
        .count();
    if carriers > 1 {
        return Err(SourceError::InvalidJsonCarrier { attribute });
    }
    if let Some(uri) = object.get("BulkDataURI") {
        let uri = uri
            .as_str()
            .ok_or(SourceError::InvalidJsonCarrier { attribute })?;
        return MetadataElement::bulk_data_uri(vr, uri.to_owned())
            .map_err(|_| SourceError::InvalidJsonCarrier { attribute });
    }
    if let Some(encoded) = object.get("InlineBinary") {
        let encoded = encoded
            .as_str()
            .ok_or(SourceError::InvalidJsonCarrier { attribute })?;
        return MetadataElement::inline_binary(vr, encoded.to_owned())
            .map_err(|_| SourceError::InvalidJsonCarrier { attribute });
    }
    let Some(values) = object.get("Value") else {
        return Ok(MetadataElement::empty(vr));
    };
    let values = values
        .as_array()
        .filter(|values| !values.is_empty())
        .ok_or(SourceError::InvalidJsonValue { attribute })?;
    value_element(vr, values, attribute)
}

fn value_element(
    vr: VR,
    values: &[Value],
    attribute: usize,
) -> Result<MetadataElement, SourceError> {
    let empty_slots = values
        .iter()
        .enumerate()
        .filter_map(|(index, value)| value.is_null().then_some(index))
        .collect::<Vec<_>>();
    if !empty_slots.is_empty() {
        if vr == VR::SQ {
            return Err(SourceError::InvalidJsonValue { attribute });
        }
        let compact = values
            .iter()
            .filter(|value| !value.is_null())
            .cloned()
            .collect::<Vec<_>>();
        return value_element(vr, &compact, attribute)?
            .with_null_slots(values.len(), empty_slots)
            .map_err(|_| SourceError::InvalidJsonValue { attribute });
    }
    let invalid = || SourceError::InvalidJsonValue { attribute };
    let primitive = match vr {
        VR::PN => {
            let names = values
                .iter()
                .map(|value| {
                    let object = value.as_object().ok_or_else(invalid)?;
                    if object.keys().any(|key| {
                        !matches!(key.as_str(), "Alphabetic" | "Ideographic" | "Phonetic")
                    }) {
                        return Err(invalid());
                    }
                    Ok(PersonName::new(
                        optional_string(object, "Alphabetic", &invalid)?,
                        optional_string(object, "Ideographic", &invalid)?,
                        optional_string(object, "Phonetic", &invalid)?,
                    ))
                })
                .collect::<Result<Vec<_>, _>>()?;
            return Ok(MetadataElement::person_names(names));
        }
        VR::SQ => {
            let items = values
                .iter()
                .map(|value| {
                    value
                        .as_object()
                        .ok_or_else(invalid)
                        .and_then(parse_data_set)
                })
                .collect::<Result<Vec<_>, _>>()?;
            return Ok(MetadataElement::sequence(items));
        }
        VR::AT => PrimitiveValue::Tags(
            values
                .iter()
                .map(|value| value.as_str().and_then(parse_tag).ok_or_else(invalid))
                .collect::<Result<Vec<_>, _>>()?
                .into(),
        ),
        VR::SS => PrimitiveValue::I16(
            values
                .iter()
                .map(|value| {
                    value
                        .as_i64()
                        .and_then(|number| i16::try_from(number).ok())
                        .ok_or_else(invalid)
                })
                .collect::<Result<Vec<_>, _>>()?
                .into(),
        ),
        VR::SL => PrimitiveValue::I32(
            values
                .iter()
                .map(|value| {
                    value
                        .as_i64()
                        .and_then(|number| i32::try_from(number).ok())
                        .ok_or_else(invalid)
                })
                .collect::<Result<Vec<_>, _>>()?
                .into(),
        ),
        VR::SV => PrimitiveValue::I64(
            values
                .iter()
                .map(|value| parse_i64(value).ok_or_else(invalid))
                .collect::<Result<Vec<_>, _>>()?
                .into(),
        ),
        VR::US => PrimitiveValue::U16(
            values
                .iter()
                .map(|value| {
                    value
                        .as_u64()
                        .and_then(|number| u16::try_from(number).ok())
                        .ok_or_else(invalid)
                })
                .collect::<Result<Vec<_>, _>>()?
                .into(),
        ),
        VR::UL => PrimitiveValue::U32(
            values
                .iter()
                .map(|value| {
                    value
                        .as_u64()
                        .and_then(|number| u32::try_from(number).ok())
                        .ok_or_else(invalid)
                })
                .collect::<Result<Vec<_>, _>>()?
                .into(),
        ),
        VR::UV => PrimitiveValue::U64(
            values
                .iter()
                .map(|value| parse_u64(value).ok_or_else(invalid))
                .collect::<Result<Vec<_>, _>>()?
                .into(),
        ),
        VR::FL => PrimitiveValue::F32(
            values
                .iter()
                .map(|value| {
                    value
                        .as_number()
                        .and_then(|number| number.to_string().parse::<f32>().ok())
                        .filter(|number| number.is_finite())
                        .ok_or_else(invalid)
                })
                .collect::<Result<Vec<_>, _>>()?
                .into(),
        ),
        VR::FD => PrimitiveValue::F64(
            values
                .iter()
                .map(|value| {
                    value
                        .as_f64()
                        .filter(|number| number.is_finite())
                        .ok_or_else(invalid)
                })
                .collect::<Result<Vec<_>, _>>()?
                .into(),
        ),
        VR::OB | VR::OD | VR::OF | VR::OL | VR::OV | VR::OW | VR::UN => {
            return Err(invalid());
        }
        _ => PrimitiveValue::Strs(
            values
                .iter()
                .map(|value| match value {
                    Value::String(text) => Ok(text.clone()),
                    Value::Number(number) if matches!(vr, VR::DS | VR::IS) => {
                        Ok(number.to_string())
                    }
                    _ => Err(invalid()),
                })
                .collect::<Result<Vec<_>, _>>()?
                .into(),
        ),
    };
    Ok(MetadataElement::from_primitive(vr, &primitive))
}

fn parse_i64(value: &Value) -> Option<i64> {
    value
        .as_i64()
        .or_else(|| value.as_str().and_then(|text| text.parse().ok()))
}

fn parse_u64(value: &Value) -> Option<u64> {
    value
        .as_u64()
        .or_else(|| value.as_str().and_then(|text| text.parse().ok()))
}

fn optional_string(
    object: &Map<String, Value>,
    key: &str,
    invalid: &impl Fn() -> SourceError,
) -> Result<Option<String>, SourceError> {
    object
        .get(key)
        .map(|value| value.as_str().map(str::to_owned).ok_or_else(invalid))
        .transpose()
}
