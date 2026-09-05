//! `ocelli-compare`, the comparator's runner.
//!
//! Two exercises, and neither is a `cargo test`. They need
//! `tools/oracle/out/`, and an `#[ignore]` test that needs a directory reads
//! as a pass on the day it did not run, which is a shape this repository
//! refuses.
//!
//! ```text
//! ocelli-compare identity  [--reference DIR] [--candidate DIR] [--out DIR]
//! ocelli-compare mutations [--reference DIR] [--candidate DIR]
//! ```
//!
//! `identity` compares a directory against itself, which proves the plumbing
//! over every view. `mutations` replays the declared catalogue and requires
//! each entry to produce the verdict written beside it, which is what proves
//! detection. `bin/ocelli.sh compare` runs both.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use ocelli_oracle::attribution::{
    Context, Register, compare_view, image_rect_for, unreachable_entries_that_fired,
};
use ocelli_oracle::frame::Frame;
use ocelli_oracle::mutations::{
    CATALOGUE, Expectation, MutatedSide, Mutation, apply_to_frame, apply_to_run, resolve_target,
    touches_the_frame,
};
use ocelli_oracle::report::{Census, Outcome, RunReport, Side, ViewRecord};
use ocelli_oracle::sidecar::Run;
use serde_json::Value;

/// Where the comparator writes. **Ignored and refused by
/// `scripts/staged_content_check.py`**, because a difference image of a real
/// corpus row is a rendered picture of patient data exactly as a reference
/// frame is, and every real corpus row carries `burned-in-unchecked`.
const DEFAULT_OUT: &str = "tools/oracle/compare-out";
const DEFAULT_REFERENCE: &str = "tools/oracle/out";
const REGISTER: &str = "tools/oracle/reference-divergence.json";
const EXPECTATIONS: &str = "tools/oracle/compare-expectations.json";

fn main() -> ExitCode {
    match run() {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(message) => {
            eprintln!("ocelli-compare: {message}");
            ExitCode::FAILURE
        }
    }
}

struct Arguments {
    command: String,
    reference: PathBuf,
    candidate: PathBuf,
    out: PathBuf,
}

fn parse_arguments() -> Result<Arguments, String> {
    let mut command = String::new();
    let mut reference = PathBuf::from(DEFAULT_REFERENCE);
    let mut candidate: Option<PathBuf> = None;
    let mut out = PathBuf::from(DEFAULT_OUT);
    let mut arguments = std::env::args().skip(1);
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--reference" => {
                reference = PathBuf::from(
                    arguments
                        .next()
                        .ok_or_else(|| "--reference wants a directory".to_owned())?,
                );
            }
            "--candidate" => {
                candidate = Some(PathBuf::from(
                    arguments
                        .next()
                        .ok_or_else(|| "--candidate wants a directory".to_owned())?,
                ));
            }
            "--out" => {
                out = PathBuf::from(
                    arguments
                        .next()
                        .ok_or_else(|| "--out wants a directory".to_owned())?,
                );
            }
            other if other.starts_with("--") => {
                return Err(format!("{other} is not an argument this binary takes"));
            }
            other if command.is_empty() => command = other.to_owned(),
            other => return Err(format!("{other} is a second command")),
        }
    }
    if command.is_empty() {
        return Err("a command is required: `identity` or `mutations`. See \
             docs/lld/comparator.md"
            .to_owned());
    }
    Ok(Arguments {
        candidate: candidate.unwrap_or_else(|| reference.clone()),
        command,
        reference,
        out,
    })
}

fn run() -> Result<bool, String> {
    let arguments = parse_arguments()?;
    let register = load_register(Path::new(REGISTER))?;
    let census = load_census(Path::new(EXPECTATIONS))?;

    let reference = Run::load(&arguments.reference).map_err(|error| error.to_string())?;
    let candidate = Run::load(&arguments.candidate).map_err(|error| error.to_string())?;

    match arguments.command.as_str() {
        "identity" => identity(&reference, &candidate, &register, &census, &arguments.out),
        "mutations" => mutations(&reference, &candidate, &register, &census),
        other => Err(format!(
            "{other} is not a command. `identity` or `mutations`"
        )),
    }
}

fn load_register(path: &Path) -> Result<Register, String> {
    let text =
        std::fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let json: Value =
        serde_json::from_str(&text).map_err(|error| format!("{}: {error}", path.display()))?;
    Register::from_json(&json).map_err(|error| error.to_string())
}

fn load_census(path: &Path) -> Result<Census, String> {
    let text =
        std::fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let json: Value =
        serde_json::from_str(&text).map_err(|error| format!("{}: {error}", path.display()))?;
    Census::from_json(&json)
}

/// One whole comparison of two loaded runs.
///
/// `damage` names a view and a mutation, and is `None` for the identity run.
/// A structural refusal or a read refusal comes back as an `Err`, which is
/// what makes the catalogue's `StructuralRefusal` and `ComparisonRefusal`
/// expectations checkable.
fn compare_runs(
    reference: &Run,
    candidate: &Run,
    register: &Register,
    census: &Census,
    damage: Option<(&str, &Mutation)>,
) -> Result<RunReport, String> {
    let mut problems = Vec::new();

    reference.verify_structure().map_err(|e| e.to_string())?;
    candidate.verify_structure().map_err(|e| e.to_string())?;

    // Rung 1. Every `*Sha256` key present on either side is present on both
    // and equal. Collected by suffix rather than from a fixed list, because a
    // fixed list would silently stop covering an input a later story adds.
    let reference_digests = reference.digests();
    let candidate_digests = candidate.digests();
    let keys: BTreeSet<String> = reference_digests
        .keys()
        .chain(candidate_digests.keys())
        .cloned()
        .collect();
    for key in keys {
        match (reference_digests.get(&key), candidate_digests.get(&key)) {
            (Some(a), Some(b)) if a == b => {}
            (Some(a), Some(b)) => problems.push(format!(
                "{key} disagrees: the reference declares {a} and the candidate \
                 {b}. Two frames produced under different inputs are two \
                 correct frames that differ, so this is refused before a pixel \
                 is read"
            )),
            (Some(_), None) => problems.push(format!(
                "{key} is declared by the reference and not by the candidate"
            )),
            (None, Some(_)) => problems.push(format!(
                "{key} is declared by the candidate and not by the reference"
            )),
            (None, None) => {}
        }
    }

    let reference_ids: BTreeSet<&String> = reference.views.keys().collect();
    let candidate_ids: BTreeSet<&String> = candidate.views.keys().collect();
    let mut records: Vec<ViewRecord> = Vec::new();
    for id in reference_ids.symmetric_difference(&candidate_ids) {
        problems.push(format!(
            "{id} is declared on one side and not the other. An identifier on \
             one side only is `absent` and a run failure, never a skip"
        ));
    }

    let low_information = reference.low_information().map_err(|e| e.to_string())?;
    let downsampled = reference.downsampled().map_err(|e| e.to_string())?;
    let context = Context {
        reference,
        candidate,
        register,
        low_information: &low_information,
        downsampled: &downsampled,
    };

    for id in reference_ids.intersection(&candidate_ids) {
        let identifier = (*id).clone();
        let reference_frame = read_side(reference, &identifier, damage, MutatedSide::Reference)?;
        let candidate_frame = read_side(candidate, &identifier, damage, MutatedSide::Candidate)?;
        let record = compare_view(&context, &identifier, &reference_frame, &candidate_frame)
            .map_err(|error| error.to_string())?;
        records.push(record);
    }

    problems.extend(census.differences(&records));

    for (entry, views) in unreachable_entries_that_fired(register, reference) {
        problems.push(format!(
            "register entry {entry} is marked `reachable: false` and it fired \
             on {} view(s), first {}. An unreachable entry that fires means the \
             claim was wrong, and the entry is a reviewed change exactly as a \
             tolerance is",
            views.len(),
            views.first().map_or("none", String::as_str)
        ));
    }

    for record in &records {
        if record.outcome == Outcome::Fail {
            problems.push(format!(
                "{}: {} at rung {}, attributed to {}",
                record.id,
                record.outcome,
                record.rung,
                record.side.label()
            ));
        }
    }

    Ok(RunReport {
        records,
        problems,
        reference_directory: reference.directory.display().to_string(),
        candidate_directory: candidate.directory.display().to_string(),
    })
}

fn read_side(
    run: &Run,
    id: &str,
    damage: Option<(&str, &Mutation)>,
    side: MutatedSide,
) -> Result<Frame, String> {
    let mut frame = run.read_frame(id).map_err(|error| error.to_string())?;
    if let Some((target, mutation)) = damage
        && target == id
        && mutation.side == side
        && touches_the_frame(mutation)
    {
        let sidecar = run
            .sidecars
            .get(id)
            .ok_or_else(|| format!("{id} has no sidecar"))?;
        let rect = image_rect_for(sidecar.kind, sidecar, frame.width(), frame.height())
            .map_err(|error| error.to_string())?;
        // The view's own declared window, as an integer. `as_u64` rather
        // than a rounded `as_f64`, because `as` casts are denied here and a
        // non-integral width is a case this mutation genuinely cannot express
        // rather than one to round into shape. Declining is reported by the
        // effect with its own message.
        let window_width = sidecar
            .json
            .pointer("/voi/windowWidth")
            .and_then(serde_json::Value::as_u64)
            .and_then(|w| u32::try_from(w).ok());
        apply_to_frame(mutation, &mut frame, &rect, window_width)
            .map_err(|error| error.to_string())?;
    }
    Ok(frame)
}

fn summarise(report: &RunReport) {
    println!(
        "{} views: {} pass, {} fail, {} unmeasured, {} absent",
        report.records.len(),
        report.count(Outcome::Pass),
        report.count(Outcome::Fail),
        report.count(Outcome::Unmeasured),
        report.count(Outcome::Absent)
    );
    for (qualifier, count) in report.qualifier_counts() {
        println!("  {qualifier}: {count}");
    }
    for problem in &report.problems {
        println!("  PROBLEM {problem}");
    }
}

fn identity(
    reference: &Run,
    candidate: &Run,
    register: &Register,
    census: &Census,
    out: &Path,
) -> Result<bool, String> {
    let report = compare_runs(reference, candidate, register, census, None)?;
    summarise(&report);
    write_output(&report, reference, candidate, out)?;
    if report.green() {
        println!(
            "compare: identity over {} views is green",
            report.records.len()
        );
        Ok(true)
    } else {
        println!("compare: identity is RED");
        Ok(false)
    }
}

/// Write the report and one `<id>.diff.raw` per view carrying a difference.
///
/// The difference image is the per-lane ABSOLUTE difference, unamplified, in
/// the same RGBA8 shape as the frames it came from. Unamplified deliberately:
/// a scale factor is a number nobody stated, and the statistics in
/// `compare.json` are the evidence. The image is for locating a difference,
/// not for judging one.
fn write_output(
    report: &RunReport,
    reference: &Run,
    candidate: &Run,
    out: &Path,
) -> Result<(), String> {
    if out.exists() {
        std::fs::remove_dir_all(out).map_err(|error| format!("{}: {error}", out.display()))?;
    }
    std::fs::create_dir_all(out).map_err(|error| format!("{}: {error}", out.display()))?;
    let text = serde_json::to_string_pretty(&report.to_json())
        .map_err(|error| format!("compare.json: {error}"))?;
    std::fs::write(out.join("compare.json"), text)
        .map_err(|error| format!("compare.json: {error}"))?;

    for record in &report.records {
        let differs = record
            .statistics
            .as_ref()
            .is_some_and(|stats| stats.full.iter().any(|channel| channel.max_abs_diff > 0));
        if !differs {
            continue;
        }
        let (Ok(left), Ok(right)) = (
            reference.read_frame(&record.id),
            candidate.read_frame(&record.id),
        ) else {
            continue;
        };
        let mut bytes = Vec::with_capacity(left.bytes().len());
        for (a, b) in left
            .bytes()
            .chunks_exact(4)
            .zip(right.bytes().chunks_exact(4))
        {
            let lane = |index: usize| -> u8 {
                a.get(index)
                    .copied()
                    .unwrap_or(0)
                    .abs_diff(b.get(index).copied().unwrap_or(0))
            };
            bytes.extend_from_slice(&[lane(0), lane(1), lane(2), u8::MAX]);
        }
        if let Ok(image) = Frame::new(left.width(), left.height(), bytes) {
            let _ = std::fs::write(out.join(format!("{}.diff.raw", record.id)), image.bytes());
        }
    }
    Ok(())
}

fn mutations(
    reference: &Run,
    candidate: &Run,
    register: &Register,
    census: &Census,
) -> Result<bool, String> {
    let baseline = compare_runs(reference, candidate, register, census, None)?;
    if !baseline.green() {
        println!(
            "compare: the mutation baseline is RED before any mutation is \
             applied, so nothing below would prove anything. A broken \
             baseline is how a mutation harness reports guards as watched \
             when they are not"
        );
        summarise(&baseline);
        return Ok(false);
    }

    let mut failures = 0_usize;
    for mutation in CATALOGUE {
        let target = resolve_target(mutation, &baseline.records).map_err(|e| e.to_string())?;
        let mut damaged_reference = reference.clone();
        let mut damaged_candidate = candidate.clone();
        match mutation.side {
            MutatedSide::Reference => {
                apply_to_run(mutation, &mut damaged_reference, &target)
                    .map_err(|e| e.to_string())?;
            }
            MutatedSide::Candidate => {
                apply_to_run(mutation, &mut damaged_candidate, &target)
                    .map_err(|e| e.to_string())?;
            }
        }
        let outcome = compare_runs(
            &damaged_reference,
            &damaged_candidate,
            register,
            census,
            Some((&target, mutation)),
        );
        match check(mutation, &target, &outcome) {
            Ok(()) => println!("  {:<44} ok  ({target})", mutation.name),
            Err(reason) => {
                failures = failures.saturating_add(1);
                println!("  {:<44} NOT DETECTED  {reason}", mutation.name);
            }
        }
    }

    println!(
        "compare: {} mutations, {} not detected",
        CATALOGUE.len(),
        failures
    );
    Ok(failures == 0)
}

fn check(
    mutation: &Mutation,
    target: &str,
    outcome: &Result<RunReport, String>,
) -> Result<(), String> {
    match (&mutation.expect, outcome) {
        (Expectation::StructuralRefusal(fragment), Err(message))
        | (Expectation::ComparisonRefusal(fragment), Err(message)) => {
            if message.contains(fragment) {
                Ok(())
            } else {
                Err(format!(
                    "expected a refusal carrying {fragment:?}, got {message:?}"
                ))
            }
        }
        (
            Expectation::StructuralRefusal(fragment) | Expectation::ComparisonRefusal(fragment),
            Ok(_),
        ) => Err(format!(
            "expected a refusal carrying {fragment:?} and the run completed"
        )),
        (Expectation::RunProblem(fragment), Ok(report)) => {
            if report
                .problems
                .iter()
                .any(|problem| problem.contains(fragment))
            {
                Ok(())
            } else {
                Err(format!(
                    "expected a run problem carrying {fragment:?}, got {:?}",
                    report.problems
                ))
            }
        }
        (Expectation::RunProblem(fragment), Err(message)) => Err(format!(
            "expected a run problem carrying {fragment:?} and the run refused with {message:?}"
        )),
        (Expectation::View { .. } | Expectation::UnmeasuredAndDetected, Err(message)) => {
            Err(format!("the run refused with {message:?}"))
        }
        (
            Expectation::View {
                outcome: want,
                qualifiers,
                side,
            },
            Ok(report),
        ) => {
            let record = report
                .records
                .iter()
                .find(|record| record.id == target)
                .ok_or_else(|| format!("{target} is not in the report"))?;
            if record.outcome != *want {
                return Err(format!(
                    "{target} is {} and {want} was declared",
                    record.outcome
                ));
            }
            for qualifier in *qualifiers {
                if !record.qualifiers.contains(qualifier) {
                    return Err(format!(
                        "{target} does not carry {}, it carries {:?}",
                        qualifier.label(),
                        record.qualifier_labels()
                    ));
                }
            }
            if *side != Side::None && record.side != *side {
                return Err(format!(
                    "{target} is attributed to {} and {} was declared",
                    record.side.label(),
                    side.label()
                ));
            }
            Ok(())
        }
        (Expectation::UnmeasuredAndDetected, Ok(report)) => {
            let record = report
                .records
                .iter()
                .find(|record| record.id == target)
                .ok_or_else(|| format!("{target} is not in the report"))?;
            if record.outcome != Outcome::Unmeasured {
                return Err(format!(
                    "{target} is {} and unmeasured was declared",
                    record.outcome
                ));
            }
            let detected = record
                .statistics
                .as_ref()
                .is_some_and(|stats| stats.full.iter().any(|channel| channel.max_abs_diff > 0));
            if detected {
                Ok(())
            } else {
                Err(format!(
                    "{target} stayed unmeasured and the statistics show no \
                     difference, so the damage was not measured at all"
                ))
            }
        }
    }
}
