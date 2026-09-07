use std::collections::BTreeSet;

use ocelli_oracle::report::{GateVerdict, Outcome, Qualifier, RunReport, Rung, Side, ViewRecord};
use ocelli_oracle::sidecar::ViewKind;
use ocelli_oracle::tolerance::ToleranceClass;

fn record(id: &str, outcome: Outcome) -> ViewRecord {
    let (qualifiers, rung) = if outcome == Outcome::Unmeasured {
        ([Qualifier::Weak].into_iter().collect(), Rung::Weak)
    } else {
        (BTreeSet::new(), Rung::Pixels)
    };
    ViewRecord {
        id: id.to_owned(),
        kind: ViewKind::Stack,
        class: ToleranceClass::MonochromeSixteenBit,
        outcome,
        qualifiers,
        side: Side::None,
        rung,
        notes: Vec::new(),
        parameter_divergences: Vec::new(),
        parameter_values_withheld: false,
        geometry_divergences: Vec::new(),
        register_entry: None,
        reference_render_hash: "reference-hash".to_owned(),
        candidate_render_hash: "candidate-hash".to_owned(),
        statistics: None,
        monochrome_frame: true,
        photometric_interpretation: Some("MONOCHROME2".to_owned()),
    }
}

fn report(records: Vec<ViewRecord>) -> RunReport {
    RunReport {
        records,
        problems: Vec::new(),
        coverage_problems: Vec::new(),
        absent_views: 0,
        unsupported_source_rows: 2,
        declared_volume_refusals: 1,
        reference_directory: "reference".to_owned(),
        candidate_directory: "candidate".to_owned(),
    }
}

#[test]
fn judged_views_are_only_pass_plus_fail() {
    let report = report(vec![
        record("pass", Outcome::Pass),
        record("fail", Outcome::Fail),
        record("unmeasured", Outcome::Unmeasured),
    ]);

    assert_eq!(report.claimed_verdict_views(), 2);
    assert_eq!(report.unsupported_source_rows, 2);
    assert_eq!(report.declared_volume_refusals, 1);
}

#[test]
fn a_pixel_failure_is_not_reported_as_coverage_loss() {
    let report = report(vec![record("failed", Outcome::Fail)]);

    assert_eq!(report.gate_verdict(), GateVerdict::ComparisonFailure);
}

#[test]
fn a_missing_view_is_coverage_loss() {
    let mut report = report(vec![record("present", Outcome::Pass)]);
    report.absent_views = 1;

    assert_eq!(report.gate_verdict(), GateVerdict::CoverageLoss);
    assert!(!report.green());
}

#[test]
fn a_new_unmeasured_view_is_coverage_loss() {
    let mut report = report(vec![record("new", Outcome::Unmeasured)]);
    report
        .coverage_problems
        .push("new is unmeasured and absent from the committed census".to_owned());

    assert_eq!(report.gate_verdict(), GateVerdict::CoverageLoss);
    assert!(!report.green());
}

#[test]
fn zero_judged_views_is_coverage_loss() {
    let report = report(vec![record("only", Outcome::Unmeasured)]);

    assert_eq!(report.claimed_verdict_views(), 0);
    assert_eq!(report.gate_verdict(), GateVerdict::CoverageLoss);
    assert!(!report.green());
}

#[test]
fn an_input_refusal_is_distinct_from_comparison_and_coverage() {
    let mut report = report(vec![record("present", Outcome::Pass)]);
    report.problems.push("input digest disagrees".to_owned());

    assert_eq!(report.gate_verdict(), GateVerdict::Refusal);
}

#[test]
fn an_input_refusal_precedes_a_pixel_failure() {
    let mut report = report(vec![record("failed", Outcome::Fail)]);
    report.problems.push("input digest disagrees".to_owned());

    assert_eq!(report.count(Outcome::Fail), 1);
    assert_eq!(report.gate_verdict(), GateVerdict::Refusal);
}

#[test]
fn the_json_names_every_coverage_class_and_the_gate_verdict() {
    let report = report(vec![
        record("pass", Outcome::Pass),
        record("unmeasured", Outcome::Unmeasured),
    ]);
    let json = report.to_json();

    assert_eq!(json.pointer("/claimedVerdictViews"), Some(&1.into()));
    assert_eq!(json.pointer("/coverage/unmeasured"), Some(&1.into()));
    assert_eq!(json.pointer("/coverage/absent"), Some(&0.into()));
    assert_eq!(
        json.pointer("/coverage/unsupportedSourceRows"),
        Some(&2.into())
    );
    assert_eq!(
        json.pointer("/coverage/declaredVolumeRefusals"),
        Some(&1.into())
    );
    assert_eq!(json.pointer("/gateVerdict"), Some(&"pass".into()));
}
