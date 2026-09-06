//! Committed metadata truth for synthetic oracle rows.
//!
//! The truth is read from `metadata-truth.json`. It is independent of both
//! sidecars, so agreement between two producers cannot hide a shared error.

use std::collections::BTreeMap;

use serde_json::Value;
use thiserror::Error;

use crate::report::{Outcome, ParameterDivergence, Qualifier, Rung, Side, ViewRecord};
use crate::sidecar::Sidecar;
use crate::tolerance::ToleranceClass;

#[derive(Debug, Error)]
pub enum MetadataError {
    #[error("metadata-truth.json: {0}")]
    Schema(String),
    #[error("{view}: {side} carries a non-finite number at {pointer}")]
    NonFinite {
        view: String,
        side: &'static str,
        pointer: String,
    },
}

#[derive(Clone, Debug)]
struct FieldTruth {
    scope: String,
    source_pointer: Option<String>,
    value: Value,
    tolerance: Option<f64>,
}

#[derive(Clone, Debug)]
struct Entry {
    citation: String,
    fields: BTreeMap<String, FieldTruth>,
}

/// The one committed set of expected metadata values.
#[derive(Clone, Debug)]
pub struct MetadataTruth {
    entries: BTreeMap<String, Entry>,
    volume_entries: BTreeMap<String, Entry>,
}

/// Result of comparing both producers with the independent truth.
#[derive(Clone, Debug, Default)]
pub struct TruthComparison {
    pub divergences: Vec<ParameterDivergence>,
    pub shared_problem: bool,
}

impl TruthComparison {
    #[must_use]
    pub fn into_failure_record(
        self,
        id: &str,
        reference: &Sidecar,
        candidate: &Sidecar,
        class: ToleranceClass,
    ) -> Option<ViewRecord> {
        if self.divergences.is_empty() {
            return None;
        }
        let declared_hash = |sidecar: &Sidecar| {
            sidecar
                .json
                .pointer("/frame/sha256")
                .and_then(Value::as_str)
                .unwrap_or("<frame not read because metadata truth failed>")
                .to_owned()
        };
        let mut record = ViewRecord {
            id: id.to_owned(),
            kind: reference.kind,
            class,
            outcome: Outcome::Fail,
            qualifiers: std::collections::BTreeSet::new(),
            side: Side::Unattributed,
            rung: Rung::MetadataTruth,
            notes: vec![
                "frame bytes were not read because committed metadata truth answered first"
                    .to_owned(),
            ],
            parameter_divergences: Vec::new(),
            parameter_values_withheld: false,
            geometry_divergences: Vec::new(),
            register_entry: None,
            reference_render_hash: declared_hash(reference),
            candidate_render_hash: declared_hash(candidate),
            statistics: None,
            monochrome_frame: false,
            photometric_interpretation: reference
                .json
                .pointer("/attributes/photometricInterpretation")
                .and_then(Value::as_str)
                .map(str::to_owned),
        };
        self.apply_to(&mut record);
        Some(record)
    }

    /// Put committed truth ahead of every pixel-based attribution in a view
    /// record. A clean comparison leaves the existing verdict untouched.
    pub fn apply_to(self, record: &mut ViewRecord) {
        if self.divergences.is_empty() {
            return;
        }
        record.qualifiers.insert(Qualifier::MetadataTruth);
        record.outcome = Outcome::Fail;
        let first_side = self.divergences.first().map(|divergence| divergence.side);
        let one_side = first_side.is_some_and(|side| {
            self.divergences
                .iter()
                .all(|divergence| divergence.side == side)
        });
        record.side = if !self.shared_problem && one_side {
            first_side.unwrap_or(Side::Unattributed)
        } else {
            Side::Unattributed
        };
        record.rung = Rung::MetadataTruth;
        if self.shared_problem {
            record.notes.push(
                "both producers disagree with committed synthetic truth. This is an instrument or shared-reference problem, not agreement"
                    .to_owned(),
            );
        }
        record.parameter_divergences.extend(self.divergences);
    }
}

impl MetadataTruth {
    /// Parse and validate the committed truth document.
    ///
    /// # Errors
    /// When its schema is incomplete or a field lacks its DICOM scope.
    pub fn from_json(root: &Value, volume_root: &Value) -> Result<Self, MetadataError> {
        let version = root.pointer("/schemaVersion").and_then(Value::as_u64);
        if version != Some(1) {
            return Err(MetadataError::Schema(
                "schemaVersion must be the integer 1".to_owned(),
            ));
        }
        validate_display_fixtures(root)?;
        let raw_entries = root
            .pointer("/entries")
            .and_then(Value::as_object)
            .ok_or_else(|| MetadataError::Schema("entries must be an object".to_owned()))?;
        let mut entries = BTreeMap::new();
        for (path, raw) in raw_entries {
            if !path.starts_with("synthetic/") && !path.starts_with("syntax/") {
                return Err(MetadataError::Schema(format!(
                    "{path} is not a generated corpus row"
                )));
            }
            let citation = raw
                .pointer("/citation")
                .and_then(Value::as_str)
                .filter(|text| text.starts_with("PS3.3 "))
                .ok_or_else(|| {
                    MetadataError::Schema(format!("{path} needs a citation beginning with PS3.3"))
                })?
                .to_owned();
            let fields = parse_fields(path, raw)?;
            entries.insert(path.clone(), Entry { citation, fields });
        }
        let mut volume_entries = BTreeMap::new();
        let raw_volumes = volume_root
            .pointer("/subjects")
            .and_then(Value::as_object)
            .ok_or_else(|| {
                MetadataError::Schema("volume-truth.json subjects must be an object".to_owned())
            })?;
        let geometry_tolerance = volume_root
            .pointer("/toleranceMm")
            .and_then(Value::as_f64)
            .ok_or_else(|| {
                MetadataError::Schema("volume-truth.json needs toleranceMm".to_owned())
            })?;
        for (subject, raw) in raw_volumes {
            let Some(series) = raw.pointer("/referenceGeometryTruth") else {
                continue;
            };
            let citation = raw
                .pointer("/citation")
                .and_then(Value::as_str)
                .filter(|text| text.starts_with("PS3.3 "))
                .ok_or_else(|| {
                    MetadataError::Schema(format!(
                        "volume subject {subject} needs a citation beginning with PS3.3"
                    ))
                })?
                .to_owned();
            let series_directory = raw
                .pointer("/seriesDirectory")
                .and_then(Value::as_str)
                .filter(|value| value.starts_with("synthetic/"))
                .ok_or_else(|| {
                    MetadataError::Schema(format!(
                        "volume subject {subject} needs a synthetic seriesDirectory"
                    ))
                })?
                .to_owned();
            let mut fields = BTreeMap::new();
            for (name, tolerance) in [
                ("dimensions", None),
                ("origin", Some(geometry_tolerance)),
                ("direction", Some(geometry_tolerance)),
            ] {
                let value = series.get(name).cloned().ok_or_else(|| {
                    MetadataError::Schema(format!(
                        "volume subject {subject} referenceGeometryTruth needs {name}"
                    ))
                })?;
                fields.insert(
                    format!("/volume/referenceGeometry/{name}"),
                    FieldTruth {
                        scope: "derived-volume".to_owned(),
                        source_pointer: None,
                        value,
                        tolerance,
                    },
                );
            }
            volume_entries.insert(series_directory, Entry { citation, fields });
        }
        Ok(Self {
            entries,
            volume_entries,
        })
    }

    /// Compare one stack view with its synthetic truth. Rows absent from the
    /// truth file are real or deliberately outside the current surface.
    ///
    /// # Errors
    /// On a non-finite number. Such a value is refused instead of being
    /// allowed to compare unequal to everything.
    pub fn compare(
        &self,
        view: &str,
        reference: &Sidecar,
        candidate: &Sidecar,
    ) -> Result<TruthComparison, MetadataError> {
        let (path, entry) =
            if let Some(path) = reference.json.pointer("/row/path").and_then(Value::as_str) {
                let Some(entry) = self.entries.get(path) else {
                    return Ok(TruthComparison::default());
                };
                (path, entry)
            } else if let Some(series) = reference
                .json
                .pointer("/volume/seriesDirectory")
                .and_then(Value::as_str)
            {
                let Some(entry) = self.volume_entries.get(series) else {
                    return Ok(TruthComparison::default());
                };
                (series, entry)
            } else {
                return Ok(TruthComparison::default());
            };
        let mut result = TruthComparison::default();
        for (pointer, field) in &entry.fields {
            let left = Observed::at(&reference.json, pointer);
            let right = Observed::at(&candidate.json, pointer);
            ensure_finite(left.value(), view, "reference", pointer)?;
            ensure_finite(right.value(), view, "candidate", pointer)?;
            ensure_finite(&field.value, view, "truth", pointer)?;
            let left_ok = field.matches(&left) && field.source_matches(&reference.json);
            let right_ok = field.matches(&right) && field.source_matches(&candidate.json);
            if left_ok && right_ok {
                continue;
            }
            let side = match (left_ok, right_ok) {
                (false, true) => Side::Reference,
                (true, false) => Side::Ours,
                (false, false) => {
                    result.shared_problem = true;
                    Side::Unattributed
                }
                (true, true) => Side::None,
            };
            result.divergences.push(ParameterDivergence {
                pointer: pointer.clone(),
                reference: left.report_value(),
                candidate: right.report_value(),
                side,
                why: format!(
                    "metadata truth for {path} at {} ({}, {}) is {}",
                    field.scope, entry.citation, pointer, field.value
                ),
            });
        }
        Ok(result)
    }
}

impl FieldTruth {
    fn matches(&self, observed: &Observed<'_>) -> bool {
        let Observed::Present(value) = observed else {
            return false;
        };
        self.tolerance.map_or_else(
            || exact_equal(value, &self.value),
            |tolerance| within_tolerance(value, &self.value, tolerance),
        )
    }

    fn source_matches(&self, sidecar: &Value) -> bool {
        self.source_pointer.as_ref().is_none_or(|pointer| {
            sidecar.pointer(pointer).and_then(Value::as_str) == Some(self.scope.as_str())
        })
    }
}

enum Observed<'a> {
    Missing,
    Present(&'a Value),
}

impl<'a> Observed<'a> {
    fn at(root: &'a Value, pointer: &str) -> Self {
        root.pointer(pointer).map_or(Self::Missing, Self::Present)
    }

    fn value(&self) -> &Value {
        static NULL: Value = Value::Null;
        match self {
            Self::Missing => &NULL,
            Self::Present(value) => value,
        }
    }

    fn report_value(&self) -> Value {
        match self {
            Self::Missing => Value::String("<missing JSON member>".to_owned()),
            Self::Present(value) => (*value).clone(),
        }
    }
}

fn validate_display_fixtures(root: &Value) -> Result<(), MetadataError> {
    let samples = root
        .pointer("/displayFixtures/soft-tissue-ct/samples")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            MetadataError::Schema("soft-tissue-ct samples must be an array".to_owned())
        })?;
    if samples.len() != 4 {
        return Err(MetadataError::Schema(format!(
            "soft-tissue-ct must contain all four reviewed C.11 samples, found {}",
            samples.len()
        )));
    }
    for (index, sample) in samples.iter().enumerate() {
        for field in ["input", "linear", "linearExact"] {
            if sample.get(field).and_then(Value::as_f64).is_none() {
                return Err(MetadataError::Schema(format!(
                    "soft-tissue-ct sample {index} needs numeric {field}"
                )));
            }
        }
    }
    Ok(())
}

fn parse_fields(owner: &str, raw: &Value) -> Result<BTreeMap<String, FieldTruth>, MetadataError> {
    let raw_fields = raw
        .pointer("/fields")
        .and_then(Value::as_object)
        .ok_or_else(|| MetadataError::Schema(format!("{owner} needs fields")))?;
    if raw_fields.is_empty() {
        return Err(MetadataError::Schema(format!("{owner} has no fields")));
    }
    let mut fields = BTreeMap::new();
    for (pointer, raw_field) in raw_fields {
        if !pointer.starts_with('/') {
            return Err(MetadataError::Schema(format!(
                "{owner} field {pointer:?} is not a JSON pointer"
            )));
        }
        let scope = raw_field
            .pointer("/scope")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                MetadataError::Schema(format!("{owner} field {pointer} needs a scope"))
            })?;
        let valid_scope = matches!(scope, "top-level" | "shared-functional-group" | "per-frame");
        if !valid_scope {
            return Err(MetadataError::Schema(format!(
                "{owner} field {pointer} has unknown scope {scope:?}"
            )));
        }
        let source_pointer = raw_field.get("sourcePointer").and_then(Value::as_str);
        if pointer.starts_with("/attributes/") {
            if scope != "top-level" || source_pointer.is_some() {
                return Err(MetadataError::Schema(format!(
                    "{owner} field {pointer} is a direct top-level attribute"
                )));
            }
        } else if pointer.starts_with("/cornerstoneMetadata/") {
            let expected = pointer.replacen(
                "/cornerstoneMetadata/",
                "/metadataSources/cornerstoneMetadata/",
                1,
            );
            if source_pointer != Some(expected.as_str()) {
                return Err(MetadataError::Schema(format!(
                    "{owner} field {pointer} must bind sourcePointer {expected}"
                )));
            }
        } else {
            return Err(MetadataError::Schema(format!(
                "{owner} field {pointer} is not an allowed metadata surface"
            )));
        }
        let value = raw_field.get("value").cloned().ok_or_else(|| {
            MetadataError::Schema(format!("{owner} field {pointer} needs a value"))
        })?;
        let tolerance = match raw_field.get("toleranceMm") {
            Some(value) => Some(value.as_f64().ok_or_else(|| {
                MetadataError::Schema(format!(
                    "{owner} field {pointer} toleranceMm must be a number"
                ))
            })?),
            None => None,
        };
        if tolerance.is_some_and(|value| !value.is_finite() || value <= 0.0) {
            return Err(MetadataError::Schema(format!(
                "{owner} field {pointer} toleranceMm must be finite and positive"
            )));
        }
        fields.insert(
            pointer.clone(),
            FieldTruth {
                scope: scope.to_owned(),
                source_pointer: source_pointer.map(str::to_owned),
                value,
                tolerance,
            },
        );
    }
    Ok(fields)
}

fn ensure_finite(
    value: &Value,
    view: &str,
    side: &'static str,
    pointer: &str,
) -> Result<(), MetadataError> {
    match value {
        Value::Number(number) => {
            if number.as_f64().is_some_and(|number| !number.is_finite()) {
                return Err(MetadataError::NonFinite {
                    view: view.to_owned(),
                    side,
                    pointer: pointer.to_owned(),
                });
            }
        }
        Value::Array(values) => {
            for item in values {
                ensure_finite(item, view, side, pointer)?;
            }
        }
        Value::Object(values) => {
            for item in values.values() {
                ensure_finite(item, view, side, pointer)?;
            }
        }
        Value::Null | Value::Bool(_) | Value::String(_) => {}
    }
    Ok(())
}

fn exact_equal(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Number(a), Value::Number(b)) => match (a.as_f64(), b.as_f64()) {
            (Some(a), Some(b)) => a.to_bits() == b.to_bits(),
            _ => false,
        },
        (Value::Array(a), Value::Array(b)) => {
            a.len() == b.len() && a.iter().zip(b).all(|(a, b)| exact_equal(a, b))
        }
        (Value::Object(a), Value::Object(b)) => {
            a.len() == b.len()
                && a.iter()
                    .all(|(key, value)| b.get(key).is_some_and(|other| exact_equal(value, other)))
        }
        _ => left == right,
    }
}

fn within_tolerance(left: &Value, right: &Value, tolerance: f64) -> bool {
    match (left, right) {
        (Value::Number(a), Value::Number(b)) => match (a.as_f64(), b.as_f64()) {
            (Some(a), Some(b)) => (a - b).abs() <= tolerance,
            _ => false,
        },
        (Value::Array(a), Value::Array(b)) => {
            a.len() == b.len()
                && a.iter()
                    .zip(b)
                    .all(|(a, b)| within_tolerance(a, b, tolerance))
        }
        _ => exact_equal(left, right),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::{MetadataTruth, exact_equal};
    use crate::report::{Outcome, Qualifier, Rung, Side, ViewRecord};
    use crate::sidecar::{Sidecar, ViewKind};
    use crate::tolerance::ToleranceClass;
    use serde_json::json;

    #[test]
    fn absence_empty_order_and_signed_zero_are_deliberate() {
        assert!(!exact_equal(&json!(null), &json!([])));
        assert!(!exact_equal(&json!([]), &json!([null])));
        assert!(exact_equal(&json!([0.5, 0.25]), &json!([0.5, 0.25])));
        assert!(!exact_equal(&json!([0.5, 0.25]), &json!([0.25, 0.5])));
        assert!(!exact_equal(&json!(0.0), &json!(-0.0)));
    }

    #[test]
    fn json_refuses_nan_before_metadata_can_compare_it() {
        assert!(serde_json::from_str::<serde_json::Value>("NaN").is_err());
    }

    fn sidecar(value: serde_json::Value) -> Sidecar {
        Sidecar {
            id: "synthetic__case".to_owned(),
            kind: ViewKind::Stack,
            json: value,
        }
    }

    fn document(entries: serde_json::Value) -> serde_json::Value {
        json!({
            "schemaVersion": 1,
            "displayFixtures": {
                "soft-tissue-ct": {
                    "samples": [
                        { "input": 0.0, "linear": 0.0, "linearExact": 0.0 },
                        { "input": 1.0, "linear": 1.0, "linearExact": 1.0 },
                        { "input": 2.0, "linear": 2.0, "linearExact": 2.0 },
                        { "input": 3.0, "linear": 3.0, "linearExact": 3.0 }
                    ]
                }
            },
            "entries": entries
        })
    }

    fn empty_volume_document() -> serde_json::Value {
        json!({ "toleranceMm": 0.000001, "subjects": {} })
    }

    fn pixel_record() -> ViewRecord {
        ViewRecord {
            id: "synthetic__case".to_owned(),
            kind: ViewKind::Stack,
            class: ToleranceClass::MonochromeSixteenBit,
            outcome: Outcome::Pass,
            qualifiers: BTreeSet::new(),
            side: Side::None,
            rung: Rung::Pixels,
            notes: Vec::new(),
            parameter_divergences: Vec::new(),
            parameter_values_withheld: false,
            geometry_divergences: Vec::new(),
            register_entry: None,
            reference_render_hash: "reference".to_owned(),
            candidate_render_hash: "candidate".to_owned(),
            statistics: None,
            monochrome_frame: true,
            photometric_interpretation: Some("MONOCHROME2".to_owned()),
        }
    }

    #[test]
    fn candidate_and_shared_truth_failures_are_distinguished()
    -> Result<(), Box<dyn std::error::Error>> {
        let truth = MetadataTruth::from_json(
            &document(json!({
                "synthetic/case.dcm": {
                    "citation": "PS3.3 C.11.1",
                    "fields": {
                        "/attributes/rescaleIntercept": {
                            "scope": "top-level",
                            "value": -1024.0
                        }
                    }
                }
            })),
            &empty_volume_document(),
        )?;
        let correct = sidecar(json!({
            "row": { "path": "synthetic/case.dcm" },
            "attributes": { "rescaleIntercept": -1024.0 }
        }));
        let wrong = sidecar(json!({
            "row": { "path": "synthetic/case.dcm" },
            "attributes": { "rescaleIntercept": 0.0 }
        }));
        let candidate = truth.compare("synthetic__case", &correct, &wrong)?;
        assert_eq!(candidate.divergences.len(), 1);
        assert_eq!(
            candidate.divergences.first().map(|entry| entry.side),
            Some(crate::report::Side::Ours)
        );
        assert!(!candidate.shared_problem);
        let shared = truth.compare("synthetic__case", &wrong, &wrong)?;
        assert_eq!(shared.divergences.len(), 1);
        assert_eq!(
            shared.divergences.first().map(|entry| entry.side),
            Some(crate::report::Side::Unattributed)
        );
        assert!(shared.shared_problem);

        let mut record = pixel_record();
        candidate.apply_to(&mut record);
        assert_eq!(record.outcome, Outcome::Fail);
        assert_eq!(record.rung, Rung::MetadataTruth);
        assert_eq!(record.side, Side::Ours);
        assert!(record.qualifiers.contains(&Qualifier::MetadataTruth));
        Ok(())
    }

    #[test]
    fn missing_member_is_not_explicit_null() -> Result<(), Box<dyn std::error::Error>> {
        let truth = MetadataTruth::from_json(
            &document(json!({
                "synthetic/case.dcm": {
                    "citation": "PS3.3 C.11.1",
                    "fields": {
                        "/attributes/rescaleIntercept": {
                            "scope": "top-level",
                            "value": null
                        }
                    }
                }
            })),
            &empty_volume_document(),
        )?;
        let explicit = sidecar(json!({
            "row": { "path": "synthetic/case.dcm" },
            "attributes": { "rescaleIntercept": null }
        }));
        let missing = sidecar(json!({
            "row": { "path": "synthetic/case.dcm" },
            "attributes": {}
        }));
        let result = truth.compare("synthetic__case", &explicit, &missing)?;
        assert_eq!(result.divergences.len(), 1);
        let divergence = result.divergences.first().ok_or("divergence is absent")?;
        assert_eq!(divergence.side, Side::Ours);
        assert_eq!(divergence.candidate, json!("<missing JSON member>"));
        let record = result
            .into_failure_record(
                "synthetic__case",
                &explicit,
                &missing,
                ToleranceClass::MonochromeSixteenBit,
            )
            .ok_or("metadata failure did not produce a record")?;
        assert_eq!(record.rung, Rung::MetadataTruth);
        assert!(record.statistics.is_none());
        assert_eq!(
            record.reference_render_hash,
            "<frame not read because metadata truth failed>"
        );
        Ok(())
    }

    #[test]
    fn errors_on_both_sides_or_shared_with_one_side_are_unattributed() {
        let mut opposite = pixel_record();
        super::TruthComparison {
            divergences: vec![
                crate::report::ParameterDivergence {
                    pointer: "/a".to_owned(),
                    reference: json!(0),
                    candidate: json!(1),
                    side: Side::Reference,
                    why: "fixture".to_owned(),
                },
                crate::report::ParameterDivergence {
                    pointer: "/b".to_owned(),
                    reference: json!(1),
                    candidate: json!(0),
                    side: Side::Ours,
                    why: "fixture".to_owned(),
                },
            ],
            shared_problem: false,
        }
        .apply_to(&mut opposite);
        assert_eq!(opposite.side, Side::Unattributed);

        let mut mixed = pixel_record();
        super::TruthComparison {
            divergences: vec![crate::report::ParameterDivergence {
                pointer: "/a".to_owned(),
                reference: json!(0),
                candidate: json!(1),
                side: Side::Ours,
                why: "fixture".to_owned(),
            }],
            shared_problem: true,
        }
        .apply_to(&mut mixed);
        assert_eq!(mixed.side, Side::Unattributed);
    }

    #[test]
    fn changed_scope_label_is_detected_against_sidecar_provenance()
    -> Result<(), Box<dyn std::error::Error>> {
        let truth = MetadataTruth::from_json(
            &document(json!({
                "synthetic/case.dcm": {
                    "citation": "PS3.3 C.7.6.16",
                    "fields": {
                        "/cornerstoneMetadata/modalityLutModule/rescaleIntercept": {
                            "scope": "top-level",
                            "sourcePointer": "/metadataSources/cornerstoneMetadata/modalityLutModule/rescaleIntercept",
                            "value": -1024.0
                        }
                    }
                }
            })),
            &empty_volume_document(),
        )?;
        let actual = sidecar(json!({
            "row": { "path": "synthetic/case.dcm" },
            "cornerstoneMetadata": { "modalityLutModule": { "rescaleIntercept": -1024.0 } },
            "metadataSources": { "cornerstoneMetadata": { "modalityLutModule": {
                "rescaleIntercept": "per-frame"
            } } }
        }));
        let result = truth.compare("synthetic__case", &actual, &actual)?;
        assert_eq!(result.divergences.len(), 1);
        assert!(result.shared_problem);
        Ok(())
    }

    #[test]
    fn truncating_the_c11_truth_is_refused() -> Result<(), Box<dyn std::error::Error>> {
        let mut document: serde_json::Value =
            serde_json::from_str(include_str!("../metadata-truth.json"))?;
        let samples = document
            .pointer_mut("/displayFixtures/soft-tissue-ct/samples")
            .and_then(serde_json::Value::as_array_mut)
            .ok_or("display samples are absent")?;
        samples.pop();
        assert!(MetadataTruth::from_json(&document, &empty_volume_document()).is_err());
        Ok(())
    }

    #[test]
    fn derived_volume_geometry_uses_the_written_world_bound()
    -> Result<(), Box<dyn std::error::Error>> {
        let truth = MetadataTruth::from_json(
            &document(json!({})),
            &json!({
                "toleranceMm": 0.000001,
                "subjects": {
                    "volume__synthetic__series": {
                        "citation": "PS3.3 C.7.6.2.1.1",
                        "seriesDirectory": "synthetic/series",
                        "referenceGeometryTruth": {
                            "dimensions": [1, 1, 1],
                            "origin": [0.0, 0.0, 0.0],
                            "direction": [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]
                        }
                    }
                }
            }),
        )?;
        let reference = sidecar(json!({
            "volume": {
                "seriesDirectory": "synthetic/series",
                "referenceGeometry": {
                    "dimensions": [1, 1, 1],
                    "origin": [0.0, 0.0, 0.0],
                    "direction": [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]
                }
            }
        }));
        let inside = sidecar(json!({
            "volume": {
                "seriesDirectory": "synthetic/series",
                "referenceGeometry": {
                    "dimensions": [1, 1, 1],
                    "origin": [0.000001, 0.0, 0.0],
                    "direction": [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]
                }
            }
        }));
        assert!(
            truth
                .compare("inside", &reference, &inside)?
                .divergences
                .is_empty()
        );
        let outside = sidecar(json!({
            "volume": {
                "seriesDirectory": "synthetic/series",
                "referenceGeometry": {
                    "dimensions": [1, 1, 1],
                    "origin": [0.000002, 0.0, 0.0],
                    "direction": [1.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0]
                }
            }
        }));
        let comparison = truth.compare("outside", &reference, &outside)?;
        assert_eq!(comparison.divergences.len(), 1);
        assert_eq!(
            comparison.divergences.first().map(|item| item.side),
            Some(Side::Ours)
        );
        Ok(())
    }
}
