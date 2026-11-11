//! Bounded identity projection for native QIDO identifier replies.
//!
//! Scope, error, and UID syntax helpers live here together with the
//! envelope composer. Parsing goes through the bounded borrowed-source
//! reader; no route wiring or server integration is claimed here.

use dicom_core::{Tag, VR};
use ocelli_dicom::{MetadataSet, MetadataValue};

/// Restricts which QIDO queries the caller may build.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum QidoScope {
    /// Query at the study level.
    Study,
    /// Query inside one study.
    Series {
        /// Study instance UID for the query.
        study_uid: String,
    },
    /// Query inside one study and series.
    Instance {
        /// Study instance UID for the query.
        study_uid: String,
        /// Series instance UID for the query.
        series_uid: String,
    },
}

/// Errors raised while reading QIDO identifier attributes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum QidoError {
    /// Requested identifier cannot be answered for the current scope.
    InvalidRequest,
    /// An attribute is absent, malformed, or has wrong VR/value form.
    InvalidValue,
    /// Memory for returned text could not be reserved.
    AllocationRefused,
}

use ocelli_dicom::{
    DicomwebSource, SourceBatch, SourceError, SourceResponseKind, SourceResponseLimits,
};
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};

pub(crate) fn is_valid_uid(uid: &str) -> bool {
    if uid.is_empty() || uid.len() > 64 {
        return false;
    }
    uid.split('.').all(|component| {
        if component.is_empty() {
            return false;
        }
        let mut chars = component.chars();
        let first = match chars.next() {
            Some(c) => c,
            None => return false,
        };
        if !first.is_ascii_digit() {
            return false;
        }
        if first == '0' && chars.next().is_some() {
            return false;
        }
        component.chars().all(|c| c.is_ascii_digit())
    })
}

fn required_uid(set: &MetadataSet, tag: Tag) -> Result<String, QidoError> {
    let element = match set.get(tag) {
        Some(e) => e,
        None => return Err(QidoError::InvalidValue),
    };
    if element.vr() != VR::UI {
        return Err(QidoError::InvalidValue);
    }
    let text: &Vec<String> = match element.value() {
        MetadataValue::Text(v) => v,
        _ => return Err(QidoError::InvalidValue),
    };
    if text.len() != 1 {
        return Err(QidoError::InvalidValue);
    }
    let semantic = match element.semantic_text() {
        Some(s) => s,
        None => return Err(QidoError::InvalidValue),
    };
    let uid = match semantic.first() {
        Some(s) => s,
        None => return Err(QidoError::InvalidValue),
    };
    if !is_valid_uid(uid) {
        return Err(QidoError::InvalidValue);
    }
    Ok(uid.to_string())
}

fn optional_uid(set: &MetadataSet, tag: Tag) -> Result<Option<String>, QidoError> {
    match set.get(tag) {
        None => Ok(None),
        Some(_) => required_uid(set, tag).map(Some),
    }
}

const QIDO_NONCE_LEN: usize = 32;
pub(crate) const QIDO_MAX_GENERATION: u64 = 9_007_199_254_740_991;
const TAG_STUDY_INSTANCE_UID: Tag = Tag(0x0020, 0x000D);
const TAG_SERIES_INSTANCE_UID: Tag = Tag(0x0020, 0x000E);
const TAG_SOP_INSTANCE_UID: Tag = Tag(0x0008, 0x0018);

pub(crate) fn qido_nonce_is_valid(nonce: &str) -> bool {
    nonce.len() == QIDO_NONCE_LEN
        && nonce
            .bytes()
            .all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

fn qido_scope_is_valid(scope: &QidoScope) -> bool {
    match scope {
        QidoScope::Study => true,
        QidoScope::Series { study_uid } => is_valid_uid(study_uid),
        QidoScope::Instance {
            study_uid,
            series_uid,
        } => is_valid_uid(study_uid) && is_valid_uid(series_uid),
    }
}

fn qido_captured_scope(scope: &QidoScope) -> Value {
    match scope {
        QidoScope::Study => json!({}),
        QidoScope::Series { study_uid } => json!({ "studyUid": study_uid }),
        QidoScope::Instance {
            study_uid,
            series_uid,
        } => json!({
            "studyUid": study_uid,
            "seriesUid": series_uid,
        }),
    }
}

fn qido_parent_matches(optional_parent: Option<&str>, expected: &str) -> Result<(), QidoError> {
    match optional_parent {
        None => Ok(()),
        Some(parent) => {
            if parent == expected {
                Ok(())
            } else {
                Err(QidoError::InvalidValue)
            }
        }
    }
}

fn qido_project_row(scope: &QidoScope, set: &MetadataSet) -> Result<Value, QidoError> {
    match scope {
        QidoScope::Study => {
            let study_uid = required_uid(set, TAG_STUDY_INSTANCE_UID)?;
            Ok(json!({ "studyUid": study_uid }))
        }
        QidoScope::Series { study_uid } => {
            let series_uid = required_uid(set, TAG_SERIES_INSTANCE_UID)?;
            let returned_study = optional_uid(set, TAG_STUDY_INSTANCE_UID)?;
            qido_parent_matches(returned_study.as_deref(), study_uid)?;
            let mut row = Map::new();
            row.insert("seriesUid".to_string(), Value::String(series_uid));
            if let Some(study) = returned_study {
                row.insert("returnedStudyUid".to_string(), Value::String(study));
            }
            Ok(Value::Object(row))
        }
        QidoScope::Instance {
            study_uid,
            series_uid,
        } => {
            let instance_uid = required_uid(set, TAG_SOP_INSTANCE_UID)?;
            let returned_study = optional_uid(set, TAG_STUDY_INSTANCE_UID)?;
            let returned_series = optional_uid(set, TAG_SERIES_INSTANCE_UID)?;
            qido_parent_matches(returned_study.as_deref(), study_uid)?;
            qido_parent_matches(returned_series.as_deref(), series_uid)?;
            let mut row = Map::new();
            row.insert("instanceUid".to_string(), Value::String(instance_uid));
            if let Some(study) = returned_study {
                row.insert("returnedStudyUid".to_string(), Value::String(study));
            }
            if let Some(series) = returned_series {
                row.insert("returnedSeriesUid".to_string(), Value::String(series));
            }
            Ok(Value::Object(row))
        }
    }
}

fn qido_level(scope: &QidoScope) -> &'static str {
    match scope {
        QidoScope::Study => "study",
        QidoScope::Series { .. } => "series",
        QidoScope::Instance { .. } => "instance",
    }
}

pub(crate) fn qido_reply(
    raw: &[u8],
    scope: &QidoScope,
    source_nonce: &str,
    generation: u64,
) -> Result<Value, QidoError> {
    if !qido_scope_is_valid(scope) {
        return Err(QidoError::InvalidRequest);
    }
    if !qido_nonce_is_valid(source_nonce) {
        return Err(QidoError::InvalidRequest);
    }
    if generation > QIDO_MAX_GENERATION {
        return Err(QidoError::InvalidRequest);
    }
    let batch = DicomwebSource
        .consume_borrowed(
            raw,
            &SourceResponseKind::QidoJson,
            &SourceResponseLimits {
                encoded_body_max: 2 * 1024 * 1024,
                decoded_max: 128 * 1024 * 1024,
                max_parts: 1,
                max_rows: 20000,
            },
        )
        .map_err(|err| match err {
            SourceError::SizeLimit => QidoError::AllocationRefused,
            _ => QidoError::InvalidValue,
        })?;
    let rows = match batch {
        SourceBatch::Query(rows) => rows,
        _ => return Err(QidoError::InvalidValue),
    };
    let mut projected = Vec::with_capacity(rows.len());
    for set in &rows {
        projected.push(qido_project_row(scope, set)?);
    }
    let mut hasher = Sha256::new();
    hasher.update(raw);
    let raw_body_sha256 = format!("{:x}", hasher.finalize());
    Ok(json!({
        "level": qido_level(scope),
        "rows": projected,
        "capturedScope": qido_captured_scope(scope),
        "sourceNonce": source_nonce,
        "generation": generation,
        "rawBodySha256": raw_body_sha256,
    }))
}

// appended to examples/folder-viewer/src/dicomweb.rs

use dicom_core::{DicomValue, PrimitiveValue};
use dicom_object::DefaultDicomObject;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct InstanceIdentity {
    pub(crate) study_uid: String,
    pub(crate) series_uid: String,
    pub(crate) instance_uid: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum InstanceError {
    /// The caller-supplied identity, nonce, or generation is not well-formed.
    InvalidRequest,
    /// A parsed attribute is absent, malformed, or disagrees with the caller.
    InvalidValue,
    /// The body exceeded the bounded limits for this representation.
    AllocationRefused,
    /// The multipart boundary or a part media type is unacceptable.
    ContentType,
}

const WADO_INSTANCES_FIELD: &str = "wado-rs-instances";
const WADO_INSTANCES_ENCODED_BODY_MAX: usize = 32 * 1024 * 1024;
const WADO_INSTANCES_DECODED_MAX: usize = 128 * 1024 * 1024;
const WADO_INSTANCES_BOUNDARY_MAX: usize = 70;

fn instance_uid_text(object: &DefaultDicomObject, tag: Tag) -> Result<String, InstanceError> {
    let element = object
        .element(tag)
        .map_err(|_| InstanceError::InvalidValue)?;
    if element.vr() != VR::UI {
        return Err(InstanceError::InvalidValue);
    }
    let text: &str = match element.value() {
        DicomValue::Primitive(PrimitiveValue::Str(value)) => value.as_str(),
        DicomValue::Primitive(PrimitiveValue::Strs(values)) => {
            if values.len() != 1 {
                return Err(InstanceError::InvalidValue);
            }
            match values.first() {
                Some(value) => value.as_str(),
                None => return Err(InstanceError::InvalidValue),
            }
        }
        _ => return Err(InstanceError::InvalidValue),
    };
    let trimmed = match text.strip_suffix('\0') {
        Some(stripped) => stripped,
        None => text,
    };
    if !is_valid_uid(trimmed) {
        return Err(InstanceError::InvalidValue);
    }
    Ok(trimmed.to_string())
}

pub(crate) fn instance_reply(
    raw: &[u8],
    boundary: &str,
    expected: &InstanceIdentity,
    source_nonce: &str,
    generation: u64,
) -> Result<Value, InstanceError> {
    if !is_valid_uid(&expected.study_uid)
        || !is_valid_uid(&expected.series_uid)
        || !is_valid_uid(&expected.instance_uid)
    {
        return Err(InstanceError::InvalidRequest);
    }
    if !qido_nonce_is_valid(source_nonce) {
        return Err(InstanceError::InvalidRequest);
    }
    if generation > QIDO_MAX_GENERATION {
        return Err(InstanceError::InvalidRequest);
    }
    if boundary.is_empty() || boundary.len() > WADO_INSTANCES_BOUNDARY_MAX {
        return Err(InstanceError::ContentType);
    }
    let kind = SourceResponseKind::WadoRsInstances {
        boundary: boundary.to_owned(),
    };
    let limits = SourceResponseLimits {
        encoded_body_max: WADO_INSTANCES_ENCODED_BODY_MAX,
        decoded_max: WADO_INSTANCES_DECODED_MAX,
        max_parts: 1,
        max_rows: 20000,
    };
    let batch = DicomwebSource
        .consume_borrowed(raw, &kind, &limits)
        .map_err(|err| match err {
            SourceError::SizeLimit => InstanceError::AllocationRefused,
            SourceError::InvalidMultipartBoundary | SourceError::InvalidPartMediaType { .. } => {
                InstanceError::ContentType
            }
            _ => InstanceError::InvalidValue,
        })?;
    let mut instances = match batch {
        SourceBatch::Instances(instances) => instances,
        _ => return Err(InstanceError::InvalidValue),
    };
    let parsed = match instances.pop() {
        Some(parsed) => parsed,
        None => return Err(InstanceError::InvalidValue),
    };
    if !instances.is_empty() {
        return Err(InstanceError::InvalidValue);
    }
    let object = parsed.object();
    let study_uid = instance_uid_text(object, TAG_STUDY_INSTANCE_UID)?;
    let series_uid = instance_uid_text(object, TAG_SERIES_INSTANCE_UID)?;
    let instance_uid = instance_uid_text(object, TAG_SOP_INSTANCE_UID)?;
    if study_uid != expected.study_uid
        || series_uid != expected.series_uid
        || instance_uid != expected.instance_uid
    {
        return Err(InstanceError::InvalidValue);
    }
    let mut hasher = Sha256::new();
    hasher.update(raw);
    let raw_body_sha256 = format!("{:x}", hasher.finalize());
    Ok(json!({
        "identity": {
            "studyUid": study_uid,
            "seriesUid": series_uid,
            "instanceUid": instance_uid,
        },
        "rawBodySha256": raw_body_sha256,
        "sourceNonce": source_nonce,
        "generation": generation,
        "byteLength": raw.len(),
        "representation": {
            "field": WADO_INSTANCES_FIELD,
            "boundary": boundary,
        },
    }))
}

use std::io;

/// The fixed generic rejection message used for every invalid viewer dicomweb URL pair.
/// No input values or parser errors are ever echoed into this message.
pub(crate) const VIEWER_DICOMWEB_INVALID_PAIR_MESSAGE: &str = "viewer dicomweb configuration requires exactly two valid full http or https absolute URLs with no user info, query, or fragment";

/// Owned viewer dicomweb endpoint configuration derived from a validated URL pair.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ViewerDicomwebConfig {
    pub(crate) studies_service_url: String,
    pub(crate) wado_uri_url: String,
    pub(crate) connect_origin: String,
}

/// Restricts the host so the derived connect origin is safe for a single CSP
/// connect-src directive. Only IPv4 literals and registered names composed
/// solely of ASCII letters, ASCII digits, hyphens, and dots with no empty
/// label are permitted. Everything else (wildcards, sub-delims,
/// percent-encoding, and other RFC 3986 reg-name characters) is rejected so
/// wildcard sources, directive terminators, and header splitters can never
/// reach the policy. The refusal of a numeric last label lives in
/// `validate_full_http_url`.
fn is_allowed_host(host: &uriparse::Host<'_>) -> bool {
    match host {
        uriparse::Host::IPv4Address(_) => true,
        uriparse::Host::IPv6Address(_) => false,
        uriparse::Host::RegisteredName(name) => {
            let text = name.as_str();
            if text.is_empty() {
                return false;
            }
            if !text
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'.')
            {
                return false;
            }
            text.split('.').all(|label| !label.is_empty())
        }
    }
}

/// Returns the scheme default port for the browser origin serialization:
/// 80 for http and 443 for https. Any other scheme is already refused.
fn scheme_default_port(scheme_str: &str) -> Option<u16> {
    if scheme_str.eq_ignore_ascii_case("http") {
        Some(80)
    } else if scheme_str.eq_ignore_ascii_case("https") {
        Some(443)
    } else {
        None
    }
}

/// Validates a single full http or https absolute URL and returns the owned
/// connect origin derived from its scheme and authority, serialized the way
/// a browser serializes an origin: lowercase scheme, ASCII-lowercase
/// registered-name host, and no port when the port equals the scheme
/// default. The URL is parsed
/// exactly once. No parser errors are reflected to the caller.
fn validate_full_http_url(value: &str) -> Result<String, ()> {
    if value.is_empty() {
        return Err(());
    }

    let reference = uriparse::URIReference::try_from(value).map_err(|_| ())?;

    let scheme = reference.scheme().ok_or(())?;
    let scheme_str = scheme.as_str();
    if !scheme_str.eq_ignore_ascii_case("http") && !scheme_str.eq_ignore_ascii_case("https") {
        return Err(());
    }

    if reference.username().is_some() || reference.password().is_some() {
        return Err(());
    }

    let authority = reference.authority().ok_or(())?;

    let host = authority.host();
    if !is_allowed_host(host) {
        return Err(());
    }

    // A registered name whose last label is all ASCII digits, or is 0x or
    // 0X followed by zero or more ASCII hex digits, is refused.
    // WHATWG URL host parsing fails such a host because it ends in a number
    // but is not a valid IPv4 address, so the browser could never fetch it.
    // This also refuses leading-zero dotted forms such as 012.0.0.1, because
    // std rejects them as IPv4 so they arrive as registered names.
    if let uriparse::Host::RegisteredName(name) = host {
        let text = name.as_str();
        let is_number_label = |label: &str| {
            label.bytes().all(|byte| byte.is_ascii_digit())
                || ((label.starts_with("0x") || label.starts_with("0X"))
                    && label[2..].bytes().all(|byte| byte.is_ascii_hexdigit()))
        };
        if let Some(last_label) = text.rsplit('.').next()
            && is_number_label(last_label)
        {
            return Err(());
        }
    }

    // Port 0 is refused for the same reason as a numeric last label: the OS
    // hands it to a listener that asked to bind ephemeral, so a browser
    // connection to a configured service on it always fails.
    if reference.port() == Some(0) {
        return Err(());
    }

    if reference.query().is_some() || reference.fragment().is_some() {
        return Err(());
    }

    let mut origin = String::new();
    origin.push_str(scheme_str);
    origin.push_str("://");
    match host {
        uriparse::Host::IPv4Address(address) => origin.push_str(&address.to_string()),
        uriparse::Host::IPv6Address(_) => return Err(()),
        uriparse::Host::RegisteredName(name) => {
            origin.push_str(&name.as_str().to_ascii_lowercase());
        }
    }
    let default_port = scheme_default_port(scheme_str);
    if let Some(port) = reference.port()
        && Some(port) != default_port
    {
        origin.push(':');
        origin.push_str(&port.to_string());
    }

    Ok(origin)
}

/// Validates an optional pair of viewer dicomweb URLs.
///
/// Both `None` yields `Ok(None)` for local-only operation. Any partial pair, empty
/// value, malformed URL, or host outside the allowed host rule yields a fixed generic
/// `std::io::ErrorKind::InvalidInput` error carrying no original values and no
/// parser errors. Both full original strings are stored unchanged; the connect
/// origin derives only from the validated studies scheme and authority.
pub(crate) fn viewer_dicomweb_config_from_pair(
    studies: Option<&str>,
    wado: Option<&str>,
) -> io::Result<Option<ViewerDicomwebConfig>> {
    match (studies, wado) {
        (None, None) => Ok(None),
        (Some(studies), Some(wado)) => {
            let studies_origin = validate_full_http_url(studies).map_err(|_| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    VIEWER_DICOMWEB_INVALID_PAIR_MESSAGE,
                )
            })?;
            validate_full_http_url(wado).map_err(|_| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    VIEWER_DICOMWEB_INVALID_PAIR_MESSAGE,
                )
            })?;

            Ok(Some(ViewerDicomwebConfig {
                studies_service_url: studies.to_owned(),
                wado_uri_url: wado.to_owned(),
                connect_origin: studies_origin,
            }))
        }
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            VIEWER_DICOMWEB_INVALID_PAIR_MESSAGE,
        )),
    }
}

/// Converts optional OsStr endpoint values to optional strings, treating a
/// present value that is not valid Unicode as a present invalid value rather
/// than absence, then delegates to `viewer_dicomweb_config_from_pair`.
pub(crate) fn viewer_dicomweb_config_from_os(
    studies: Option<&std::ffi::OsStr>,
    wado: Option<&std::ffi::OsStr>,
) -> io::Result<Option<ViewerDicomwebConfig>> {
    let invalid = || {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            VIEWER_DICOMWEB_INVALID_PAIR_MESSAGE,
        )
    };
    let studies = match studies {
        None => None,
        Some(value) => match value.to_str() {
            Some(value) => Some(value),
            None => return Err(invalid()),
        },
    };
    let wado = match wado {
        None => None,
        Some(value) => match value.to_str() {
            Some(value) => Some(value),
            None => return Err(invalid()),
        },
    };
    viewer_dicomweb_config_from_pair(studies, wado)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_rows_bind_exact_envelope() {
        let result = qido_reply(
            b"[]",
            &QidoScope::Study,
            "00000000000000000000000000000000",
            0,
        );
        assert_eq!(
            result,
            Ok(json!({
                "level": "study",
                "rows": [],
                "capturedScope": {},
                "sourceNonce": "00000000000000000000000000000000",
                "generation": 0,
                "rawBodySha256": "4f53cda18c2baa0c0354bb5f9a3ecbe5ed12ab4d8e11ba873c2f11161202b945"
            }))
        );
    }

    #[test]
    fn qido_reply_valid_series_row_preserves_returned_parent() {
        let raw = br#"[{"0020000D":{"vr":"UI","Value":["2.25.1"]},"0020000E":{"vr":"UI","Value":["2.25.2"]}}]"#;
        let scope = QidoScope::Series {
            study_uid: "2.25.1".to_string(),
        };
        let reply = qido_reply(raw, &scope, "00000000000000000000000000000000", 0)
            .map(|reply| reply.get("rows").cloned());
        assert_eq!(
            reply,
            Ok(Some(json!([
                {
                    "seriesUid": "2.25.2",
                    "returnedStudyUid": "2.25.1"
                }
            ])))
        );
    }

    #[test]
    fn qido_reply_missing_parent_row_and_wrong_parent_uid() {
        let scope = QidoScope::Series {
            study_uid: "2.25.1".to_string(),
        };
        let raw_missing = br#"[{"0020000E":{"vr":"UI","Value":["2.25.2"]}}]"#;
        assert_eq!(
            qido_reply(raw_missing, &scope, "00000000000000000000000000000000", 0)
                .map(|reply| reply.get("rows").cloned()),
            Ok(Some(json!([{ "seriesUid": "2.25.2" }])))
        );
        let raw_wrong = br#"[{"0020000D":{"vr":"UI","Value":["9.9"]},"0020000E":{"vr":"UI","Value":["2.25.2"]}}]"#;
        assert!(matches!(
            qido_reply(raw_wrong, &scope, "00000000000000000000000000000000", 0),
            Err(QidoError::InvalidValue)
        ));
    }

    #[test]
    fn qido_instance_reply_single_row_returns_parsed_parents() {
        let scope = QidoScope::Instance {
            study_uid: "2.25.1".to_string(),
            series_uid: "2.25.2".to_string(),
        };
        let raw = br#"[{"00080018":{"vr":"UI","Value":["2.25.3"]},"0020000D":{"vr":"UI","Value":["2.25.1"]},"0020000E":{"vr":"UI","Value":["2.25.2"]}}]"#;
        let expected_rows = json!([{"instanceUid":"2.25.3","returnedStudyUid":"2.25.1","returnedSeriesUid":"2.25.2"}]);
        let result = qido_reply(raw, &scope, "00000000000000000000000000000000", 0)
            .map(|reply| reply.get("rows").cloned());
        assert_eq!(result, Ok(Some(expected_rows)));
    }

    #[test]
    fn qido_instance_reply_parent_mismatch_rejected() {
        let scope = QidoScope::Instance {
            study_uid: "2.25.1".to_string(),
            series_uid: "2.25.2".to_string(),
        };
        let valid_raw = br#"[{"00080018":{"vr":"UI","Value":["2.25.3"]}}]"#;
        let expected_rows = json!([{"instanceUid":"2.25.3"}]);
        let result = qido_reply(valid_raw, &scope, "00000000000000000000000000000000", 0)
            .map(|reply| reply.get("rows").cloned());
        assert_eq!(result, Ok(Some(expected_rows)));

        let mismatch_raw = br#"[{"00080018":{"vr":"UI","Value":["2.25.3"]},"0020000E":{"vr":"UI","Value":["9.9"]}}]"#;
        let mismatch = qido_reply(mismatch_raw, &scope, "00000000000000000000000000000000", 0);
        assert_eq!(mismatch, Err(QidoError::InvalidValue));
    }

    #[test]
    fn qido_request_context_faults_are_rejected() {
        let invalid_series_scope = QidoScope::Series {
            study_uid: "2.25.01".to_string(),
        };
        assert!(matches!(
            qido_reply(
                b"[]",
                &invalid_series_scope,
                "00000000000000000000000000000000",
                0
            ),
            Err(QidoError::InvalidRequest)
        ));

        let uppercase_nonce = "A".repeat(32);
        assert!(matches!(
            qido_reply(b"[]", &QidoScope::Study, &uppercase_nonce, 0),
            Err(QidoError::InvalidRequest)
        ));

        let short_nonce = "0".repeat(31);
        assert!(matches!(
            qido_reply(b"[]", &QidoScope::Study, &short_nonce, 0),
            Err(QidoError::InvalidRequest)
        ));

        let oversized_generation: u64 = 9_007_199_254_740_992;
        assert!(matches!(
            qido_reply(
                b"[]",
                &QidoScope::Study,
                "00000000000000000000000000000000",
                oversized_generation
            ),
            Err(QidoError::InvalidRequest)
        ));
    }

    #[test]
    fn qido_payload_limit_boundary_is_enforced() {
        let mut body = b"[]".to_vec();
        body.resize(2 * 1024 * 1024, b' ');
        let boundary_ok = qido_reply(
            &body,
            &QidoScope::Study,
            "00000000000000000000000000000000",
            0,
        )
        .map(|reply| reply.get("rows").cloned());
        assert_eq!(boundary_ok, Ok(Some(json!([]))));

        body.push(b' ');
        assert!(matches!(
            qido_reply(
                &body,
                &QidoScope::Study,
                "00000000000000000000000000000000",
                0
            ),
            Err(QidoError::AllocationRefused)
        ));
    }

    #[test]
    fn qido_reply_rejects_invalid_study_json() {
        let nonce = "00000000000000000000000000000000";
        let cases: [&str; 7] = [
            "[{}]",
            r#"[{"0020000D":{"vr":"LO","Value":["2.25.1"]}}]"#,
            r#"[{"0020000D":{"vr":"UI","Value":["2.25.1","2.25.2"]}}]"#,
            r#"[{"0020000D":{"vr":"UI","Value":[]}}]"#,
            r#"[{"0020000D":{"vr":"UI","Value":["2.25.01"]}}]"#,
            r#"[{"0020000D":{"vr":"UI","Value":["2..25.1"]}}]"#,
            r#"[{"0020000D":{"vr":"UI","Value":["2.25.a"]}}]"#,
        ];
        for raw in cases {
            let result = qido_reply(raw.as_bytes(), &QidoScope::Study, nonce, 0);
            assert_eq!(result, Err(QidoError::InvalidValue));
        }
    }

    // Wholly synthetic fixtures: PS3.5 Part10 and PS3.18 multipart gold.
    // Generator SHA256 84600a3d2f3160a06fa2b3b51d2593c15986b306ab42be9bfbe5a12c1542f3b3, pydicom 3.0.2.
    // Source fixture data only; no HTTP or base64 transport layer implied.
    pub const F245_A_MULTIPART_HEX: &str = "2D2D663234352D7761646F2D626F756E646172790D0A436F6E74656E742D547970653A206170706C69636174696F6E2F6469636F6D0D0A436F6E74656E742D4C6F636174696F6E3A202F73796E7468657469632F696E7374616E63650D0A436F6E74656E742D4C656E6774683A203435380D0A0D0A00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000004449434D02000000554C04007C000000020001004F4200000200000000010200020055491A00312E322E3834302E31303030382E352E312E342E312E312E32000200030055490600322E32352E330200100055491400312E322E3834302E31303030382E312E322E31000200120055490800322E32352E3939000200130053480A004632343553594E5448200800160055491A00312E322E3834302E31303030382E352E312E342E312E312E32000800180055490600322E32352E330800600043530200435420000D0055490600322E32352E3120000E0055490600322E32352E32280002005553020001002800040043530C004D4F4E4F4348524F4D4532202800100055530200010028001100555302000100280000015553020010002800010155530200100028000201555302000F0028000301555302000000E07F10004F5700000200000000000D0A2D2D663234352D7761646F2D626F756E646172792D2D0D0A";
    pub const F245_A_MULTIPART_SHA256: &str =
        "06364c3195af83700d11dd3f3c6768d5aa5827b9cbc4d4cd466752ce0f77c04b";
    pub const F245_A_MULTIPART_LEN: usize = 601;
    pub const F245_B_MULTIPART_HEX: &str = "2D2D663234352D7761646F2D626F756E646172790D0A436F6E74656E742D547970653A206170706C69636174696F6E2F6469636F6D0D0A436F6E74656E742D4C6F636174696F6E3A202F73796E7468657469632F696E7374616E63650D0A436F6E74656E742D4C656E6774683A203435380D0A0D0A00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000004449434D02000000554C04007C000000020001004F4200000200000000010200020055491A00312E322E3834302E31303030382E352E312E342E312E312E32000200030055490600322E32352E330200100055491400312E322E3834302E31303030382E312E322E31000200120055490800322E32352E3939000200130053480A004632343553594E5448200800160055491A00312E322E3834302E31303030382E352E312E342E312E312E32000800180055490600322E32352E330800600043530200435420000D0055490600322E32352E3120000E0055490600322E32352E32280002005553020001002800040043530C004D4F4E4F4348524F4D4532202800100055530200010028001100555302000100280000015553020010002800010155530200100028000201555302000F0028000301555302000000E07F10004F5700000200000001000D0A2D2D663234352D7761646F2D626F756E646172792D2D0D0A";
    pub const F245_B_MULTIPART_SHA256: &str =
        "0a0e85f758f770f03f204b222105601dc4bf70ef57bfeb133355422f30ca975d";
    pub const F245_B_MULTIPART_LEN: usize = 601;

    #[test]
    fn instance_reply_wado_identity_hash_pair() -> Result<(), &'static str> {
        const A_HEX: &str = F245_A_MULTIPART_HEX;
        const A_LEN: usize = F245_A_MULTIPART_LEN;
        const A_SHA: &str = F245_A_MULTIPART_SHA256;
        const B_HEX: &str = F245_B_MULTIPART_HEX;
        const B_LEN: usize = F245_B_MULTIPART_LEN;
        const B_SHA: &str = F245_B_MULTIPART_SHA256;

        let expected = InstanceIdentity {
            study_uid: "2.25.1".to_string(),
            series_uid: "2.25.2".to_string(),
            instance_uid: "2.25.3".to_string(),
        };

        assert_eq!(A_LEN, B_LEN, "fixture lengths must be equal");
        assert_ne!(A_SHA, B_SHA, "fixture digests must differ");

        let fixtures = [(A_HEX, A_LEN, A_SHA), (B_HEX, B_LEN, B_SHA)];

        for (hex, len, digest) in fixtures {
            let body = decode_hex_fixture(hex)?;
            assert_eq!(body.len(), len, "decoded length mismatch");
            let mut hasher = Sha256::new();
            Digest::update(&mut hasher, &body);
            let actual = hasher.finalize();
            let actual_hex: String = actual.iter().map(|byte| format!("{:02x}", byte)).collect();
            assert_eq!(actual_hex, digest, "decoded sha256 mismatch");

            let reply = instance_reply(
                &body,
                "f245-wado-boundary",
                &expected,
                "00000000000000000000000000000000",
                0,
            );
            let want = json!({
                "identity": {
                    "studyUid": "2.25.1",
                    "seriesUid": "2.25.2",
                    "instanceUid": "2.25.3"
                },
                "rawBodySha256": digest,
                "sourceNonce": "00000000000000000000000000000000",
                "generation": 0,
                "byteLength": len,
                "representation": {
                    "field": "wado-rs-instances",
                    "boundary": "f245-wado-boundary"
                }
            });
            assert_eq!(reply, Ok(want), "full reply mismatch");
        }
        Ok(())
    }

    fn decode_hex_fixture(hex: &str) -> Result<Vec<u8>, &'static str> {
        let mut out = Vec::new();
        let iter = hex.as_bytes().chunks_exact(2);
        if !iter.remainder().is_empty() {
            return Err("odd hex length");
        }
        for pair in iter {
            let text = std::str::from_utf8(pair).map_err(|_| "invalid utf8")?;
            let byte = u8::from_str_radix(text, 16).map_err(|_| "invalid hex digit")?;
            out.push(byte);
        }
        Ok(out)
    }

    pub const F245_NEG_MISSING_SOP_MULTIPART_HEX: &str = "2D2D663234352D7761646F2D626F756E646172790D0A436F6E74656E742D547970653A206170706C69636174696F6E2F6469636F6D0D0A436F6E74656E742D4C6F636174696F6E3A202F73796E7468657469632F696E7374616E63650D0A436F6E74656E742D4C656E6774683A203434340D0A0D0A00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000004449434D02000000554C04007C000000020001004F4200000200000000010200020055491A00312E322E3834302E31303030382E352E312E342E312E312E32000200030055490600322E32352E330200100055491400312E322E3834302E31303030382E312E322E31000200120055490800322E32352E3939000200130053480A004632343553594E5448200800160055491A00312E322E3834302E31303030382E352E312E342E312E312E32000800600043530200435420000D0055490600322E32352E3120000E0055490600322E32352E32280002005553020001002800040043530C004D4F4E4F4348524F4D4532202800100055530200010028001100555302000100280000015553020010002800010155530200100028000201555302000F0028000301555302000000E07F10004F5700000200000000000D0A2D2D663234352D7761646F2D626F756E646172792D2D0D0A";
    pub const F245_NEG_MISSING_SOP_MULTIPART_SHA256: &str =
        "d260208b3e747aafcd76a5a14d0f257475d87789e4de8eeb6840a5e8cf87c056";
    pub const F245_NEG_MISSING_SOP_MULTIPART_LEN: usize = 587;
    pub const F245_NEG_LO_SERIES_MULTIPART_HEX: &str = "2D2D663234352D7761646F2D626F756E646172790D0A436F6E74656E742D547970653A206170706C69636174696F6E2F6469636F6D0D0A436F6E74656E742D4C6F636174696F6E3A202F73796E7468657469632F696E7374616E63650D0A436F6E74656E742D4C656E6774683A203435380D0A0D0A00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000004449434D02000000554C04007C000000020001004F4200000200000000010200020055491A00312E322E3834302E31303030382E352E312E342E312E312E32000200030055490600322E32352E330200100055491400312E322E3834302E31303030382E312E322E31000200120055490800322E32352E3939000200130053480A004632343553594E5448200800160055491A00312E322E3834302E31303030382E352E312E342E312E312E32000800180055490600322E32352E330800600043530200435420000D0055490600322E32352E3120000E004C4F0600322E32352E32280002005553020001002800040043530C004D4F4E4F4348524F4D4532202800100055530200010028001100555302000100280000015553020010002800010155530200100028000201555302000F0028000301555302000000E07F10004F5700000200000000000D0A2D2D663234352D7761646F2D626F756E646172792D2D0D0A";
    pub const F245_NEG_LO_SERIES_MULTIPART_SHA256: &str =
        "89616fb3f58a644a909242f1ec76272342dfc9870c513a063340c191f672f73f";
    pub const F245_NEG_LO_SERIES_MULTIPART_LEN: usize = 601;
    pub const F245_NEG_VM2_STUDY_MULTIPART_HEX: &str = "2D2D663234352D7761646F2D626F756E646172790D0A436F6E74656E742D547970653A206170706C69636174696F6E2F6469636F6D0D0A436F6E74656E742D4C6F636174696F6E3A202F73796E7468657469632F696E7374616E63650D0A436F6E74656E742D4C656E6774683A203436360D0A0D0A00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000004449434D02000000554C04007C000000020001004F4200000200000000010200020055491A00312E322E3834302E31303030382E352E312E342E312E312E32000200030055490600322E32352E330200100055491400312E322E3834302E31303030382E312E322E31000200120055490800322E32352E3939000200130053480A004632343553594E5448200800160055491A00312E322E3834302E31303030382E352E312E342E312E312E32000800180055490600322E32352E330800600043530200435420000D0055490E00322E32352E315C322E32352E390020000E0055490600322E32352E32280002005553020001002800040043530C004D4F4E4F4348524F4D4532202800100055530200010028001100555302000100280000015553020010002800010155530200100028000201555302000F0028000301555302000000E07F10004F5700000200000000000D0A2D2D663234352D7761646F2D626F756E646172792D2D0D0A";
    pub const F245_NEG_VM2_STUDY_MULTIPART_SHA256: &str =
        "e36e4ebffd00afe0ad33fcaf8e3a80e6177afc95da68c1f7210424a6b0b2d60a";
    pub const F245_NEG_VM2_STUDY_MULTIPART_LEN: usize = 609;

    #[test]
    fn f245_negative_identity_bodies() -> Result<(), &'static str> {
        let nonce = "00000000000000000000000000000000";
        let boundary = "f245-wado-boundary";
        let expected = InstanceIdentity {
            study_uid: "2.25.1".to_string(),
            series_uid: "2.25.2".to_string(),
            instance_uid: "2.25.3".to_string(),
        };
        let goldens: [(&str, usize, &str); 3] = [
            (
                F245_NEG_MISSING_SOP_MULTIPART_HEX,
                F245_NEG_MISSING_SOP_MULTIPART_LEN,
                F245_NEG_MISSING_SOP_MULTIPART_SHA256,
            ),
            (
                F245_NEG_LO_SERIES_MULTIPART_HEX,
                F245_NEG_LO_SERIES_MULTIPART_LEN,
                F245_NEG_LO_SERIES_MULTIPART_SHA256,
            ),
            (
                F245_NEG_VM2_STUDY_MULTIPART_HEX,
                F245_NEG_VM2_STUDY_MULTIPART_LEN,
                F245_NEG_VM2_STUDY_MULTIPART_SHA256,
            ),
        ];
        for (hex, len, sha) in goldens {
            let body = decode_hex_fixture(hex)?;
            let digest = format!("{:x}", Sha256::digest(&body));
            assert_eq!(body.len(), len);
            assert_eq!(digest, sha);
            let outcome = instance_reply(&body, boundary, &expected, nonce, 1);
            assert_eq!(outcome, Err(InstanceError::InvalidValue));
        }
        Ok(())
    }

    pub const F245_A_DCM_HEX: &str = "00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000004449434D02000000554C04007C000000020001004F4200000200000000010200020055491A00312E322E3834302E31303030382E352E312E342E312E312E32000200030055490600322E32352E330200100055491400312E322E3834302E31303030382E312E322E31000200120055490800322E32352E3939000200130053480A004632343553594E5448200800160055491A00312E322E3834302E31303030382E352E312E342E312E312E32000800180055490600322E32352E330800600043530200435420000D0055490600322E32352E3120000E0055490600322E32352E32280002005553020001002800040043530C004D4F4E4F4348524F4D4532202800100055530200010028001100555302000100280000015553020010002800010155530200100028000201555302000F0028000301555302000000E07F10004F570000020000000000";

    // Synthetic multipart fixtures for F245 WADO-RS instance reply framing controls.

    const F245_TEST_BOUNDARY: &str = "f245-wado-boundary";

    fn f245_multipart(parts: &[(&str, &[u8])]) -> Vec<u8> {
        let mut body: Vec<u8> = Vec::new();
        for (media_type, payload) in parts {
            body.extend_from_slice(format!("--{}\r\n", F245_TEST_BOUNDARY).as_bytes());
            body.extend_from_slice(format!("Content-Type: {}\r\n", media_type).as_bytes());
            body.extend_from_slice(
                format!(
                    "Content-Location: /synthetic/instance\r\nContent-Length: {}\r\n\r\n",
                    payload.len()
                )
                .as_bytes(),
            );
            body.extend_from_slice(payload);
            body.extend_from_slice(b"\r\n");
        }
        body.extend_from_slice(format!("--{}--\r\n", F245_TEST_BOUNDARY).as_bytes());
        body
    }

    #[test]
    fn f245_second_part_is_refused_before_invalid_part10_parse() -> Result<(), &'static str> {
        let bytes_a = decode_hex_fixture(F245_A_DCM_HEX)?;
        let expected = InstanceIdentity {
            study_uid: "2.25.1".to_string(),
            series_uid: "2.25.2".to_string(),
            instance_uid: "2.25.3".to_string(),
        };
        let parts: Vec<(&str, &[u8])> = vec![
            ("application/dicom", bytes_a.as_slice()),
            ("application/dicom", b"not-a-part10"),
        ];
        let mime = f245_multipart(parts.as_slice());
        let err = instance_reply(
            &mime,
            F245_TEST_BOUNDARY,
            &expected,
            "00000000000000000000000000000000",
            1,
        )
        .err()
        .ok_or("expected AllocationRefused")?;
        assert!(err == InstanceError::AllocationRefused);
        Ok(())
    }

    #[test]
    fn f245_unsupported_part_media_is_content_type() -> Result<(), &'static str> {
        let bytes_a = decode_hex_fixture(F245_A_DCM_HEX)?;
        let expected = InstanceIdentity {
            study_uid: "2.25.1".to_string(),
            series_uid: "2.25.2".to_string(),
            instance_uid: "2.25.3".to_string(),
        };
        let parts: Vec<(&str, &[u8])> = vec![("application/octet-stream", bytes_a.as_slice())];
        let mime = f245_multipart(parts.as_slice());
        let err = instance_reply(
            &mime,
            F245_TEST_BOUNDARY,
            &expected,
            "00000000000000000000000000000000",
            1,
        )
        .err()
        .ok_or("expected ContentType")?;
        assert!(err == InstanceError::ContentType);
        Ok(())
    }

    #[test]
    fn f245_truncated_multipart_is_invalid_value() -> Result<(), &'static str> {
        let bytes_a = decode_hex_fixture(F245_A_DCM_HEX)?;
        let expected = InstanceIdentity {
            study_uid: "2.25.1".to_string(),
            series_uid: "2.25.2".to_string(),
            instance_uid: "2.25.3".to_string(),
        };
        let parts: Vec<(&str, &[u8])> = vec![("application/dicom", bytes_a.as_slice())];
        let full_mime = f245_multipart(parts.as_slice());
        let prefix = full_mime
            .strip_suffix(b"--f245-wado-boundary--\r\n")
            .ok_or("missing fixture close")?;
        let framing_err = instance_reply(
            prefix,
            F245_TEST_BOUNDARY,
            &expected,
            "00000000000000000000000000000000",
            1,
        )
        .err()
        .ok_or("expected InvalidValue on truncated framing")?;
        assert!(framing_err == InstanceError::InvalidValue);
        let retry = instance_reply(
            &full_mime,
            F245_TEST_BOUNDARY,
            &expected,
            "00000000000000000000000000000000",
            1,
        );
        assert!(retry.is_ok());
        Ok(())
    }

    #[test]
    fn f245_encoded_body_fixed_cap_exact_and_over() -> Result<(), &'static str> {
        let expected = InstanceIdentity {
            study_uid: "2.25.1".to_string(),
            series_uid: "2.25.2".to_string(),
            instance_uid: "2.25.3".to_string(),
        };
        let nonce = "00000000000000000000000000000000";
        for (pad_u32, pad_usize, length, over) in [
            (33553814u32, 33553814usize, 33554432usize, true),
            (33553816u32, 33553816usize, 33554434usize, false),
        ] {
            let mut dcm = decode_hex_fixture(F245_A_DCM_HEX)?;
            assert_eq!(dcm.len(), 458);
            let mut body = Vec::new();
            body.extend_from_slice(&[0xFC, 0xFF, 0xFC, 0xFF, b'O', b'B', 0, 0]);
            body.extend_from_slice(&pad_u32.to_le_bytes());
            dcm.extend_from_slice(&body);
            dcm.resize(dcm.len() + pad_usize, 0);
            let parts = f245_multipart(&[("application/dicom", dcm.as_slice())]);
            assert_eq!(parts.len(), length);
            // Independent gold: pydicom re-encode + hashlib.sha256 over the exact byte stream.
            let expected_sha = if over {
                "cbc48589a7b46fc80b9300ca3415ad29ffb78ff5b74b22ea00870b1a346db902"
            } else {
                "a92b99c5dd4ca34ac68d00f312c52420c5ba0ef88b465e0f0bd76e1eb2ea2e1e"
            };
            let digest = Sha256::digest(&parts);
            let actual_hex: String = digest.iter().map(|b| format!("{:02x}", b)).collect();
            assert_eq!(actual_hex, expected_sha);
            let result = instance_reply(&parts, F245_TEST_BOUNDARY, &expected, nonce, 1);
            if over {
                assert!(result.is_ok());
                // Encoded length must win before multipart parsing: one raw trailing zero byte
                // makes the body 33554433 bytes even though it is not valid DICOM or MIME.
                let mut parts_plus_one = parts;
                parts_plus_one.push(0u8);
                assert_eq!(parts_plus_one.len(), 33554433);
                let result2 =
                    instance_reply(&parts_plus_one, F245_TEST_BOUNDARY, &expected, nonce, 1);
                assert!(matches!(result2, Err(InstanceError::AllocationRefused)));
            } else {
                assert_eq!(result, Err(InstanceError::AllocationRefused));
            }
        }
        Ok(())
    }

    // independent pydicom/hashlib gold
    pub const F245_DEFLATED_PREFIX_HEX: &str = "00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000004449434D02000000554C04007E000000020001004F4200000200000000010200020055491A00312E322E3834302E31303030382E352E312E342E312E312E32000200030055490600322E32352E330200100055491600312E322E3834302E31303030382E312E322E312E39390200120055490800322E32352E3939000200130053480A004632343553594E544820";
    pub const F245_DEFLATED_DATASET_HEX: &str = "0800160055491A00312E322E3834302E31303030382E352E312E342E312E312E32000800180055490600322E32352E330800600043530200435420000D0055490600322E32352E3120000E0055490600322E32352E32280002005553020001002800040043530C004D4F4E4F4348524F4D4532202800100055530200010028001100555302000100280000015553020010002800010155530200100028000201555302000F0028000301555302000000E07F10004F570000020000000000";
    pub const F245_DEFLATED_DATASET_LEN: usize = 190;

    // NOTE: gold constants are copied mechanically from pydicom fixtures
    // (independent verification via SHA256 below); not defined here.
    use flate2::Compression;
    use flate2::write::DeflateEncoder;
    use std::io::Write;

    #[test]
    fn f245_decoded_dataset_fixed_cap_deflate() -> Result<(), &'static str> {
        // (pad_u32, pad_usize, expected_decoded_len, should_pass, extra_byte)
        let cases: [(u32, usize, usize, bool, bool); 3] = [
            (134217526u32, 134217526usize, 134217728usize, true, false),
            (134217528u32, 134217528usize, 134217730usize, false, false),
            (134217526u32, 134217526usize, 134217729usize, false, true),
        ];
        for (pad_u32, pad_usize, expected_decoded_len, should_pass, extra_byte) in cases {
            // --- verify gold constants before construction (independent pydicom fixtures) ---
            let prefix = decode_hex_fixture(F245_DEFLATED_PREFIX_HEX)?;
            let dataset = decode_hex_fixture(F245_DEFLATED_DATASET_HEX)?;
            assert_eq!(prefix.len(), 270);
            assert_eq!(dataset.len(), F245_DEFLATED_DATASET_LEN);
            assert_eq!(dataset.len(), 190);
            let mut h = Sha256::new();
            h.update(&prefix);
            assert_eq!(
                format!("{:x}", h.finalize()),
                "167e6ac285f3464cd97ae1789ca5dbb0a168fc9c50755f6b6da05c5eb49f0f48"
            );
            let mut h = Sha256::new();
            h.update(&dataset);
            assert_eq!(
                format!("{:x}", h.finalize()),
                "ba279c285a689337a3150155a84384e72229e7ce0ee8c2535e61770c6720c9ac"
            );

            // --- deflate dataset + DataSetTrailingPadding element ---
            let mut enc = DeflateEncoder::new(Vec::new(), Compression::default());
            enc.write_all(&dataset)
                .map_err(|_| "deflate write failed")?;
            // header: tag (FFFC,FFFC), VR 'OB', reserved 0000, then u32 length
            enc.write_all(&[0xFC, 0xFF, 0xFC, 0xFF, b'O', b'B', 0, 0])
                .map_err(|_| "deflate write failed")?;
            enc.write_all(&pad_u32.to_le_bytes())
                .map_err(|_| "deflate write failed")?;
            // stream exactly pad_usize zero bytes via fixed scratch buffer
            let mut remaining: usize = pad_usize;
            let scratch: [u8; 65536] = [0u8; 65536];
            while remaining > 0 {
                let count = remaining.min(scratch.len());
                let chunk = scratch.get(..count).ok_or("fixture range")?;
                enc.write_all(chunk).map_err(|_| "deflate write failed")?;
                remaining -= count;
            }
            if extra_byte {
                // Not valid DICOM padding; deliberately pushes decoded size past
                // the budget before parsing so the cap check rejects it.
                enc.write_all(&[0]).map_err(|_| "deflate write failed")?;
            }
            let mut compressed = enc.finish().map_err(|_| "deflate finish failed")?;
            if !compressed.len().is_multiple_of(2) {
                compressed.push(0);
            }

            // --- assemble Part 10 + multipart ---
            let mut body = prefix.clone();
            body.extend_from_slice(&compressed);
            let mime = f245_multipart(&[("application/dicom", &body)]);
            // production limit: 32 MiB
            let limit_32mib: usize = 32 * 1024 * 1024;
            assert!(mime.len() < limit_32mib, "encoded body exceeds 32 MiB");
            // literal decoded length: dataset + 12-byte header + padding + extra byte
            let decoded_len: usize = 190 + 12 + pad_usize + usize::from(extra_byte);
            assert_eq!(decoded_len, expected_decoded_len);

            let identity = InstanceIdentity {
                study_uid: "2.25.1".to_string(),
                series_uid: "2.25.2".to_string(),
                instance_uid: "2.25.3".to_string(),
            };
            let result = instance_reply(
                &mime,
                F245_TEST_BOUNDARY,
                &identity,
                "00000000000000000000000000000000",
                1,
            );
            if should_pass {
                let _v: Value = result.map_err(|_| "expected success")?;
            } else {
                assert_eq!(result, Err(InstanceError::AllocationRefused));
            }
        }
        Ok(())
    }

    #[test]
    fn f245_requested_tuple_mismatches_then_valid_retry() -> Result<(), &'static str> {
        let raw = decode_hex_fixture(F245_A_MULTIPART_HEX)?;
        let valid = InstanceIdentity {
            study_uid: "2.25.1".to_string(),
            series_uid: "2.25.2".to_string(),
            instance_uid: "2.25.3".to_string(),
        };
        let nonce = "00000000000000000000000000000000";
        let boundary = "f245-wado-boundary";

        let mut study_swap = valid.clone();
        study_swap.study_uid = "2.25.9".to_string();
        assert_eq!(
            instance_reply(&raw, boundary, &study_swap, nonce, 1),
            Err(InstanceError::InvalidValue)
        );

        let mut series_swap = valid.clone();
        series_swap.series_uid = "2.25.9".to_string();
        assert_eq!(
            instance_reply(&raw, boundary, &series_swap, nonce, 1),
            Err(InstanceError::InvalidValue)
        );

        let mut instance_swap = valid.clone();
        instance_swap.instance_uid = "2.25.9".to_string();
        assert_eq!(
            instance_reply(&raw, boundary, &instance_swap, nonce, 1),
            Err(InstanceError::InvalidValue)
        );

        let ok = instance_reply(&raw, boundary, &valid, nonce, 1)
            .map_err(|_| "valid tuple must succeed after refusals")?;
        assert!(ok.is_object(), "valid reply must be a JSON object");
        Ok(())
    }

    #[test]
    fn f245_instance_context_single_faults() -> Result<(), &'static str> {
        let raw = decode_hex_fixture(F245_A_MULTIPART_HEX)?;
        let valid = InstanceIdentity {
            study_uid: "2.25.1".to_string(),
            series_uid: "2.25.2".to_string(),
            instance_uid: "2.25.3".to_string(),
        };
        let nonce = "00000000000000000000000000000000";
        let boundary = "f245-wado-boundary";

        let mut bad_study = valid.clone();
        bad_study.study_uid = "2..25.1".to_string();
        assert_eq!(
            instance_reply(&raw, boundary, &bad_study, nonce, 1),
            Err(InstanceError::InvalidRequest)
        );

        let nonce_32a = "A".repeat(32);
        assert_eq!(
            instance_reply(&raw, boundary, &valid, nonce_32a.as_str(), 1),
            Err(InstanceError::InvalidRequest)
        );

        let nonce_31z = "0".repeat(31);
        assert_eq!(
            instance_reply(&raw, boundary, &valid, nonce_31z.as_str(), 1),
            Err(InstanceError::InvalidRequest)
        );

        let ok = instance_reply(&raw, boundary, &valid, nonce, 9007199254740991)
            .map_err(|_| "max valid generation must succeed")?;
        assert!(
            ok.is_object(),
            "max valid generation reply must be a JSON object"
        );
        assert_eq!(
            instance_reply(&raw, boundary, &valid, nonce, 9007199254740992),
            Err(InstanceError::InvalidRequest)
        );

        assert_eq!(
            instance_reply(&raw, "", &valid, nonce, 1),
            Err(InstanceError::ContentType)
        );
        let long_boundary = "a".repeat(71);
        assert_eq!(
            instance_reply(&raw, long_boundary.as_str(), &valid, nonce, 1),
            Err(InstanceError::ContentType)
        );

        Ok(())
    }

    #[test]
    fn viewer_dicomweb_pair_absent_and_partial_cases() -> Result<(), String> {
        let absent = viewer_dicomweb_config_from_pair(None, None)
            .map_err(|error| format!("absent pair must not error: {}", error))?;
        if absent.is_some() {
            return Err("absent pair must yield None".to_string());
        }

        let partial_cases = [
            (Some("http://example.com/studies"), None),
            (None, Some("http://example.com/wado")),
            (Some(""), Some("http://example.com/wado")),
            (Some("http://example.com/studies"), Some("")),
            (Some(""), None),
            (None, Some("")),
        ];
        for (studies, wado) in partial_cases {
            let error = viewer_dicomweb_config_from_pair(studies, wado)
                .err()
                .ok_or_else(|| "partial or empty pair must be rejected".to_string())?;
            if error.kind() != std::io::ErrorKind::InvalidInput {
                return Err("partial or empty pair must yield InvalidInput".to_string());
            }
            if error.to_string() != VIEWER_DICOMWEB_INVALID_PAIR_MESSAGE {
                return Err("partial or empty pair must carry the fixed message".to_string());
            }
        }
        Ok(())
    }

    #[test]
    fn viewer_dicomweb_pair_accepts_full_http_and_https_urls() -> Result<(), String> {
        let config = viewer_dicomweb_config_from_pair(
            Some("http://pacs.example.com:8080/dicom-web/studies"),
            Some("https://viewer.example.org:8443/wado"),
        )
        .map_err(|error| format!("valid full pair must be accepted: {}", error))?
        .ok_or_else(|| "valid full pair must yield a config".to_string())?;
        if config.studies_service_url != "http://pacs.example.com:8080/dicom-web/studies" {
            return Err("studies URL must be stored unchanged".to_string());
        }
        if config.wado_uri_url != "https://viewer.example.org:8443/wado" {
            return Err("wado URL must be stored unchanged".to_string());
        }
        if config.connect_origin != "http://pacs.example.com:8080" {
            return Err("connect origin must derive from the studies URL".to_string());
        }

        let config = viewer_dicomweb_config_from_pair(
            Some("http://127.0.0.1:8042/studies"),
            Some("http://localhost/wado"),
        )
        .map_err(|error| format!("valid full pair must be accepted: {}", error))?
        .ok_or_else(|| "valid full pair must yield a config".to_string())?;
        if config.studies_service_url != "http://127.0.0.1:8042/studies" {
            return Err("IPv4 studies URL must be stored unchanged".to_string());
        }
        if config.wado_uri_url != "http://localhost/wado" {
            return Err("DNS name wado URL must be stored unchanged".to_string());
        }
        if config.connect_origin != "http://127.0.0.1:8042" {
            return Err("IPv4 connect origin must derive from the studies URL".to_string());
        }

        // The connect origin must match the browser's own origin serialization for
        // the same URL: lowercase scheme, ASCII-lowercase registered-name host,
        // and no port when the port
        // equals the scheme default (80 for http, 443 for https).
        let exact_cases = [
            ("HTTP://EXAMPLE.COM:8080/studies", "http://example.com:8080"),
            ("http://h.example:80/studies", "http://h.example"),
            ("https://h.example:443/studies", "https://h.example"),
            ("http://127.0.0.1:8042/studies", "http://127.0.0.1:8042"),
        ];
        for (url, expected_origin) in exact_cases {
            let config =
                viewer_dicomweb_config_from_pair(Some(url), Some("http://example.com/wado"))
                    .map_err(|error| format!("exact origin case must be accepted: {}", error))?
                    .ok_or_else(|| "exact origin case must yield a config".to_string())?;
            if config.connect_origin != expected_origin {
                return Err(
                    "connect origin must match the browser origin serialization".to_string()
                );
            }
            if config.studies_service_url != url {
                return Err("studies URL must be stored unchanged".to_string());
            }
            if config.wado_uri_url != "http://example.com/wado" {
                return Err("wado URL must be stored unchanged".to_string());
            }
        }
        Ok(())
    }

    #[test]
    fn viewer_dicomweb_pair_rejects_faulty_urls_identically() -> Result<(), String> {
        // The digit-ending host cases are refused because WHATWG URL host parsing
        // fails any host whose last label is all ASCII digits but that is not a
        // valid IPv4 address, so the browser could never fetch it. The
        // leading-zero IPv4 cases are refused because WHATWG reads leading-zero
        // parts as octal and would reach a different address than the canonical
        // dotted-decimal text written in the input bytes after "//" and before
        // the next "/" with any ":port" suffix removed, for example the bytes
        // "012.0.0.1" versus the canonical "12.0.0.1". Port 0 is refused because
        // the OS hands it to a listener that asked to bind ephemeral, so a
        // browser connection to it always fails.
        let faulty_urls = [
            "/relative/path",
            "example.com/studies",
            "ftp://example.com/studies",
            "http://user@example.com/studies",
            "http://user:pass@example.com/studies",
            "http://:@example.com/studies",
            "http://example.com/studies?query=1",
            "http://example.com/studies?",
            "http://example.com/studies#fragment",
            "http:///no-host/studies",
            "http://example.com:99999/studies",
            "http://example.com/%zz/studies",
            "http://*.example.com/studies",
            "http://a;b/studies",
            "http://a,b/studies",
            "http://a'b/studies",
            "http://ex%61mple.com/studies",
            "http://a..b/studies",
            "http://999.999.999.999/studies",
            "http://1.2.3.4.5/studies",
            "http://0127.0.0.1/studies",
            "http://012.0.0.1/studies",
            "http://example.com:0/studies",
            "http://127.0.0.1:0/s",
            "https://[2001:db8::1]:8042/studies",
            "http://[2001:db8::2]/wado",
            "http://0X7F000001:8042/s",
            "http://0x/s",
            "http://a.0x10/s",
        ];

        for url in faulty_urls {
            for (studies, wado) in [
                (Some(url), Some("http://example.com/wado")),
                (Some("http://example.com/studies"), Some(url)),
            ] {
                let error = viewer_dicomweb_config_from_pair(studies, wado)
                    .err()
                    .ok_or_else(|| "faulty URL must be rejected".to_string())?;
                if error.kind() != std::io::ErrorKind::InvalidInput {
                    return Err("faulty URL must yield InvalidInput".to_string());
                }
                if error.to_string() != VIEWER_DICOMWEB_INVALID_PAIR_MESSAGE {
                    return Err("faulty URL must carry the fixed message".to_string());
                }
            }
        }
        Ok(())
    }

    #[test]
    #[cfg(unix)]
    fn viewer_dicomweb_config_from_os_refuses_non_unicode_studies() -> Result<(), String> {
        use std::os::unix::ffi::OsStrExt;
        let invalid = std::ffi::OsStr::from_bytes(&[0xFF]);
        let wado = std::ffi::OsStr::new("https://pacs.example:8443/wado");
        match viewer_dicomweb_config_from_os(Some(invalid), Some(wado)) {
            Err(err) => {
                if err.kind() != io::ErrorKind::InvalidInput {
                    return Err(format!("unexpected error kind: {:?}", err.kind()));
                }
                if err.to_string() != VIEWER_DICOMWEB_INVALID_PAIR_MESSAGE {
                    return Err(format!("unexpected message: {}", err));
                }
                Ok(())
            }
            Ok(config) => Err(format!("unexpected config: {:?}", config)),
        }
    }

    #[test]
    #[cfg(unix)]
    fn viewer_dicomweb_config_from_os_refuses_non_unicode_wado() -> Result<(), String> {
        use std::os::unix::ffi::OsStrExt;
        let studies = std::ffi::OsStr::new("https://pacs.example:8443/dicom-web");
        let invalid = std::ffi::OsStr::from_bytes(&[0xFF]);
        match viewer_dicomweb_config_from_os(Some(studies), Some(invalid)) {
            Err(err) => {
                if err.kind() != io::ErrorKind::InvalidInput {
                    return Err(format!("unexpected error kind: {:?}", err.kind()));
                }
                if err.to_string() != VIEWER_DICOMWEB_INVALID_PAIR_MESSAGE {
                    return Err(format!("unexpected message: {}", err));
                }
                Ok(())
            }
            Ok(config) => Err(format!("unexpected config: {:?}", config)),
        }
    }

    #[test]
    #[cfg(unix)]
    fn viewer_dicomweb_config_from_os_refuses_two_non_unicode_values() -> Result<(), String> {
        use std::os::unix::ffi::OsStrExt;
        let invalid = std::ffi::OsStr::from_bytes(&[0xFF]);
        match viewer_dicomweb_config_from_os(Some(invalid), Some(invalid)) {
            Err(err) => {
                if err.kind() != io::ErrorKind::InvalidInput {
                    return Err(format!("unexpected error kind: {:?}", err.kind()));
                }
                if err.to_string() != VIEWER_DICOMWEB_INVALID_PAIR_MESSAGE {
                    return Err(format!("unexpected message: {}", err));
                }
                Ok(())
            }
            Ok(config) => Err(format!("unexpected config: {:?}", config)),
        }
    }

    #[test]
    #[cfg(unix)]
    fn viewer_dicomweb_config_from_os_refuses_non_unicode_studies_with_absent_wado()
    -> Result<(), String> {
        use std::os::unix::ffi::OsStrExt;
        let invalid = std::ffi::OsStr::from_bytes(&[0xFF]);
        match viewer_dicomweb_config_from_os(Some(invalid), None) {
            Err(err) => {
                if err.kind() != io::ErrorKind::InvalidInput {
                    return Err(format!("unexpected error kind: {:?}", err.kind()));
                }
                if err.to_string() != VIEWER_DICOMWEB_INVALID_PAIR_MESSAGE {
                    return Err(format!("unexpected message: {}", err));
                }
                Ok(())
            }
            Ok(config) => Err(format!("unexpected config: {:?}", config)),
        }
    }

    #[test]
    #[cfg(unix)]
    fn viewer_dicomweb_config_from_os_refuses_absent_studies_with_non_unicode_wado()
    -> Result<(), String> {
        use std::os::unix::ffi::OsStrExt;
        let invalid = std::ffi::OsStr::from_bytes(&[0xFF]);
        match viewer_dicomweb_config_from_os(None, Some(invalid)) {
            Err(err) => {
                if err.kind() != io::ErrorKind::InvalidInput {
                    return Err(format!("unexpected error kind: {:?}", err.kind()));
                }
                if err.to_string() != VIEWER_DICOMWEB_INVALID_PAIR_MESSAGE {
                    return Err(format!("unexpected message: {}", err));
                }
                Ok(())
            }
            Ok(config) => Err(format!("unexpected config: {:?}", config)),
        }
    }

    #[test]
    fn viewer_dicomweb_config_from_os_all_absent_gives_none() -> Result<(), String> {
        match viewer_dicomweb_config_from_os(None, None) {
            Ok(None) => Ok(()),
            Ok(config) => Err(format!("unexpected config: {:?}", config)),
            Err(err) => Err(format!("unexpected error: {}", err)),
        }
    }

    #[test]
    fn viewer_dicomweb_config_from_os_valid_pair_gives_connect_origin() -> Result<(), String> {
        let studies = std::ffi::OsStr::new("https://pacs.example:8443/dicom-web");
        let wado = std::ffi::OsStr::new("https://pacs.example:8443/wado");
        match viewer_dicomweb_config_from_os(Some(studies), Some(wado)) {
            Ok(Some(config)) => {
                if config.connect_origin != "https://pacs.example:8443" {
                    return Err(format!("unexpected origin: {}", config.connect_origin));
                }
                Ok(())
            }
            Ok(None) => Err(String::from("expected a config")),
            Err(err) => Err(format!("unexpected error: {}", err)),
        }
    }
}
