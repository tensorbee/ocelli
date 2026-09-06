//! `ocelli-compare`, the comparator's runner.
//!
//! Four commands, and none of them is a `cargo test`. All four need a
//! rendered run under `tools/oracle/out/`, and an `#[ignore]` test that needs
//! a directory reads as a pass on the day it did not run, which is a shape
//! this repository refuses.
//!
//! ```text
//! ocelli-compare gate      --reference DIR --candidate DIR [--out DIR]
//! ocelli-compare identity  [--reference DIR] [--candidate DIR] [--out DIR]
//! ocelli-compare mutations [--reference DIR] [--candidate DIR]
//! ocelli-compare census    [--reference DIR] [--candidate DIR]
//! ```
//!
//! `--candidate` defaults to `--reference` for every command including
//! `census`, whose baseline is a full comparison of the two directories, so
//! passing it there changes which views the census finds gating.
//!
//! `identity` compares a directory against itself, which proves the plumbing
//! over every view. `mutations` replays the declared catalogue and requires
//! each entry to produce the verdict written beside it, which is what proves
//! detection. `bin/ocelli.sh compare` runs those two.
//!
//! `gate` is the production candidate contract. Both directories are explicit
//! and must resolve to different locations. It never turns identity evidence
//! into a candidate claim.
//!
//! `census` reports nothing about correctness and gates nothing. It exists
//! because four tracked files carry counts of which corpus views the bias
//! bound can and cannot detect the LINEAR to LINEAR_EXACT divergence on, and
//! every one of those counts was wrong at least once. It applies the
//! catalogue's own swap to every gating class-one view and prints what it
//! measures, so a number in prose has a command beside it rather than a
//! provenance.

use std::collections::BTreeSet;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use ocelli_oracle::attribution::{
    Context, Register, compare_view, image_rect_for, unreachable_entries_that_fired,
};
use ocelli_oracle::frame::{ChannelSet, Frame, difference};
use ocelli_oracle::metadata::{MetadataTruth, TruthComparison};
use ocelli_oracle::mutations::{
    CATALOGUE, Expectation, MutatedSide, Mutation, apply_to_frame, apply_to_run,
    frame_read_refusal, resolve_target, touches_the_frame,
};
use ocelli_oracle::render_hash::ALGORITHM as RENDER_HASH_ALGORITHM;
use ocelli_oracle::report::{Census, Outcome, RunReport, Side, ViewRecord};
use ocelli_oracle::sidecar::Run;
use ocelli_oracle::tolerance::{
    MONOCHROME_SIGNED_MEAN_BIAS, ToleranceClass, class_from_categories,
};
use serde_json::Value;

/// Where the comparator writes. **Ignored and refused by
/// `scripts/staged_content_check.py`**, because a difference image of a real
/// corpus row is a rendered picture of patient data exactly as a reference
/// frame is, and every real corpus row carries `burned-in-unchecked`.
const DEFAULT_OUT: &str = "tools/oracle/compare-out";
const DEFAULT_REFERENCE: &str = "tools/oracle/out";
const REGISTER: &str = "tools/oracle/reference-divergence.json";
const EXPECTATIONS: &str = "tools/oracle/compare-expectations.json";
const METADATA_TRUTH: &str = "tools/oracle/metadata-truth.json";
const VOLUME_TRUTH: &str = "tools/oracle/volume-truth.json";

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
    reference_explicit: bool,
    candidate_explicit: bool,
}

fn parse_arguments() -> Result<Arguments, String> {
    parse_arguments_from(std::env::args_os().skip(1))
}

fn parse_arguments_from<I, S>(input: I) -> Result<Arguments, String>
where
    I: IntoIterator<Item = S>,
    S: Into<OsString>,
{
    let mut command = String::new();
    let mut reference = PathBuf::from(DEFAULT_REFERENCE);
    let mut reference_explicit = false;
    let mut candidate: Option<PathBuf> = None;
    let mut candidate_explicit = false;
    let mut out = PathBuf::from(DEFAULT_OUT);
    let mut arguments = input.into_iter().map(Into::into);
    while let Some(argument) = arguments.next() {
        match argument.to_string_lossy().as_ref() {
            "--reference" => {
                reference = PathBuf::from(
                    arguments
                        .next()
                        .ok_or_else(|| "--reference wants a directory".to_owned())?,
                );
                reference_explicit = true;
            }
            "--candidate" => {
                candidate = Some(PathBuf::from(
                    arguments
                        .next()
                        .ok_or_else(|| "--candidate wants a directory".to_owned())?,
                ));
                candidate_explicit = true;
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
        return Err("a command is required: `gate`, `identity`, `mutations` or \
             `census`. See docs/lld/comparator.md"
            .to_owned());
    }
    Ok(Arguments {
        candidate: candidate.unwrap_or_else(|| reference.clone()),
        command,
        reference,
        out,
        reference_explicit,
        candidate_explicit,
    })
}

fn validate_gate_directories(arguments: &Arguments) -> Result<(), String> {
    if !arguments.reference_explicit {
        return Err("`gate` requires an explicit --reference directory".to_owned());
    }
    if !arguments.candidate_explicit {
        return Err("`gate` requires an explicit --candidate directory".to_owned());
    }
    let reference = arguments.reference.canonicalize().map_err(|error| {
        format!(
            "{}: cannot resolve reference directory: {error}",
            arguments.reference.display()
        )
    })?;
    let candidate = arguments.candidate.canonicalize().map_err(|error| {
        format!(
            "{}: cannot resolve candidate directory: {error}",
            arguments.candidate.display()
        )
    })?;
    if reference == candidate {
        return Err(
            "`gate` reference and candidate must resolve to different directories".to_owned(),
        );
    }
    validate_output_directory(&arguments.out, &reference, &candidate)?;
    Ok(())
}

/// Resolve an output path even when its final components do not exist yet.
///
/// The nearest existing ancestor is canonicalized so symlinks in the part the
/// filesystem can resolve do not hide a relationship with either input.
fn resolve_output_directory(path: &Path) -> Result<PathBuf, String> {
    if path.exists() {
        return path.canonicalize().map_err(|error| {
            format!(
                "{}: cannot resolve output directory: {error}",
                path.display()
            )
        });
    }

    let mut ancestor = path.to_path_buf();
    let mut missing = Vec::new();
    while !ancestor.exists() {
        if ancestor.as_os_str().is_empty() {
            ancestor.push(".");
            continue;
        }
        let name = ancestor.file_name().ok_or_else(|| {
            format!(
                "{}: cannot resolve output directory through a missing parent",
                path.display()
            )
        })?;
        missing.push(name.to_os_string());
        if !ancestor.pop() {
            return Err(format!(
                "{}: output directory has no existing ancestor",
                path.display()
            ));
        }
    }

    let mut resolved = ancestor.canonicalize().map_err(|error| {
        format!(
            "{}: cannot resolve output directory: {error}",
            path.display()
        )
    })?;
    for component in missing.iter().rev() {
        resolved.push(component);
    }
    Ok(resolved)
}

fn validate_output_directory(out: &Path, reference: &Path, candidate: &Path) -> Result<(), String> {
    let out = resolve_output_directory(out)?;
    for (role, input) in [("reference", reference), ("candidate", candidate)] {
        let input = input.canonicalize().map_err(|error| {
            format!(
                "{}: cannot resolve {role} directory: {error}",
                input.display()
            )
        })?;
        if out == input || out.starts_with(&input) || input.starts_with(&out) {
            return Err(format!(
                "`--out` must be separate from the {role} directory and neither may contain the other"
            ));
        }
    }
    Ok(())
}

fn run() -> Result<bool, String> {
    let arguments = parse_arguments()?;
    if arguments.command == "gate" {
        validate_gate_directories(&arguments)?;
    }
    let register = load_register(Path::new(REGISTER))?;
    let census = load_census(Path::new(EXPECTATIONS))?;
    let metadata_truth = load_metadata_truth(Path::new(METADATA_TRUTH), Path::new(VOLUME_TRUTH))?;

    let reference = Run::load(&arguments.reference).map_err(|error| error.to_string())?;
    let candidate = Run::load(&arguments.candidate).map_err(|error| error.to_string())?;

    match arguments.command.as_str() {
        "gate" => gate(
            &reference,
            &candidate,
            &register,
            &metadata_truth,
            &census,
            &arguments.out,
        ),
        "identity" => identity(
            &reference,
            &candidate,
            &register,
            &metadata_truth,
            &census,
            &arguments.out,
        ),
        "mutations" => mutations(&reference, &candidate, &register, &metadata_truth, &census),
        "census" => detectability(&reference, &candidate, &register, &metadata_truth, &census),
        other => Err(format!(
            "{other} is not a command. `gate`, `identity`, `mutations` or `census`"
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

fn load_metadata_truth(path: &Path, volume_path: &Path) -> Result<MetadataTruth, String> {
    let text =
        std::fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let json: Value =
        serde_json::from_str(&text).map_err(|error| format!("{}: {error}", path.display()))?;
    let volume_text = std::fs::read_to_string(volume_path)
        .map_err(|error| format!("{}: {error}", volume_path.display()))?;
    let volume_json: Value = serde_json::from_str(&volume_text)
        .map_err(|error| format!("{}: {error}", volume_path.display()))?;
    MetadataTruth::from_json(&json, &volume_json).map_err(|error| error.to_string())
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
    metadata_truth: &MetadataTruth,
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
    let mut coverage_problems = Vec::new();
    let absent_views = reference_ids.symmetric_difference(&candidate_ids).count();
    for id in reference_ids.symmetric_difference(&candidate_ids) {
        coverage_problems.push(format!(
            "{id} is declared on one side and not the other. An identifier on \
             one side only is `absent` and a run failure, never a skip"
        ));
    }

    let unsupported_source_rows = reference.unsupported_source_rows();
    let candidate_unsupported_source_rows = candidate.unsupported_source_rows();
    if unsupported_source_rows != candidate_unsupported_source_rows {
        coverage_problems.push(format!(
            "the reference declares {unsupported_source_rows} unsupported source rows and the \
             candidate declares {candidate_unsupported_source_rows}"
        ));
    }
    let declared_volume_refusals = reference.declared_volume_refusals();
    let candidate_volume_refusals = candidate.declared_volume_refusals();
    if declared_volume_refusals != candidate_volume_refusals {
        coverage_problems.push(format!(
            "the reference declares {declared_volume_refusals} volume refusals and the candidate \
             declares {candidate_volume_refusals}"
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
        let truth = metadata_truth
            .compare(
                &identifier,
                reference
                    .sidecars
                    .get(&identifier)
                    .ok_or_else(|| format!("{identifier} has no reference sidecar"))?,
                candidate
                    .sidecars
                    .get(&identifier)
                    .ok_or_else(|| format!("{identifier} has no candidate sidecar"))?,
            )
            .map_err(|error| error.to_string())?;
        let reference_sidecar = reference
            .sidecars
            .get(&identifier)
            .ok_or_else(|| format!("{identifier} has no reference sidecar"))?;
        let candidate_sidecar = candidate
            .sidecars
            .get(&identifier)
            .ok_or_else(|| format!("{identifier} has no candidate sidecar"))?;
        let class = class_from_categories(
            reference
                .categories
                .get(&identifier)
                .map(Vec::as_slice)
                .unwrap_or_default(),
        )
        .map_err(|error| error.to_string())?;
        let record = metadata_before_frames(
            truth,
            &identifier,
            reference_sidecar,
            candidate_sidecar,
            class,
            || {
                let reference_frame =
                    read_side(reference, &identifier, damage, MutatedSide::Reference)?;
                let candidate_frame =
                    read_side(candidate, &identifier, damage, MutatedSide::Candidate)?;
                compare_view(&context, &identifier, &reference_frame, &candidate_frame)
                    .map_err(|error| error.to_string())
            },
        )?;
        records.push(record);
    }

    coverage_problems.extend(census.differences(&records));

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

    Ok(RunReport {
        records,
        problems,
        coverage_problems,
        absent_views,
        unsupported_source_rows,
        declared_volume_refusals,
        reference_directory: reference.directory.display().to_string(),
        candidate_directory: candidate.directory.display().to_string(),
    })
}

/// Resolve committed metadata truth before invoking any frame reader.
///
/// Keeping frame I/O behind the closure makes the precedence structural. The
/// metadata-failure branch cannot accidentally inspect a malformed frame.
fn metadata_before_frames<F>(
    truth: TruthComparison,
    id: &str,
    reference: &ocelli_oracle::sidecar::Sidecar,
    candidate: &ocelli_oracle::sidecar::Sidecar,
    class: ToleranceClass,
    read_and_compare_frames: F,
) -> Result<ViewRecord, String>
where
    F: FnOnce() -> Result<ViewRecord, String>,
{
    truth
        .into_failure_record(id, reference, candidate, class)
        .map_or_else(read_and_compare_frames, Ok)
}

fn read_side(
    run: &Run,
    id: &str,
    damage: Option<(&str, &Mutation)>,
    side: MutatedSide,
) -> Result<Frame, String> {
    if let Some((target, mutation)) = damage
        && target == id
        && let Some(message) = frame_read_refusal(mutation)
    {
        return Err(message.to_owned());
    }
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
        report.absent_count()
    );
    println!(
        "  claimed verdict views: {}, unsupported source rows: {}, declared volume refusals: {}",
        report.claimed_verdict_views(),
        report.unsupported_source_rows,
        report.declared_volume_refusals
    );
    for (qualifier, count) in report.qualifier_counts() {
        println!("  {qualifier}: {count}");
    }
    for problem in &report.problems {
        println!("  PROBLEM {problem}");
    }
    for problem in &report.coverage_problems {
        println!("  COVERAGE {problem}");
    }
    // Printed from the records rather than from `problems`, because that is
    // where `green()` reads it and a summary that read a different source
    // could report a red run with nothing said about why.
    for absorbed in report.absorbed_divergences() {
        println!("  PROBLEM {absorbed}");
    }
    println!(
        "render hashes ({RENDER_HASH_ALGORITHM}): reference {}, candidate {}",
        report.reference_render_hash(),
        report.candidate_render_hash()
    );
    println!("gate verdict: {}", report.gate_verdict().label());
}

fn gate(
    reference: &Run,
    candidate: &Run,
    register: &Register,
    metadata_truth: &MetadataTruth,
    census: &Census,
    out: &Path,
) -> Result<bool, String> {
    let report = compare_runs(reference, candidate, register, metadata_truth, census, None)?;
    summarise(&report);
    write_output(&report, reference, candidate, out, "gate")?;
    if report.green() {
        println!(
            "compare: candidate gate over {} judged views is green",
            report.claimed_verdict_views()
        );
        Ok(true)
    } else {
        println!(
            "compare: candidate gate is RED ({})",
            report.gate_verdict().label()
        );
        Ok(false)
    }
}

fn identity(
    reference: &Run,
    candidate: &Run,
    register: &Register,
    metadata_truth: &MetadataTruth,
    census: &Census,
    out: &Path,
) -> Result<bool, String> {
    let report = compare_runs(reference, candidate, register, metadata_truth, census, None)?;
    summarise(&report);
    write_output(&report, reference, candidate, out, "identity")?;
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
    operation: &str,
) -> Result<(), String> {
    validate_output_directory(out, &reference.directory, &candidate.directory)?;
    if out.exists() {
        std::fs::remove_dir_all(out).map_err(|error| format!("{}: {error}", out.display()))?;
    }
    std::fs::create_dir_all(out).map_err(|error| format!("{}: {error}", out.display()))?;
    let mut report_json = report.to_json();
    let object = report_json
        .as_object_mut()
        .ok_or_else(|| "compare.json: run report is not an object".to_owned())?;
    object.insert("operation".to_owned(), Value::String(operation.to_owned()));
    let text = serde_json::to_string_pretty(&report_json)
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
    metadata_truth: &MetadataTruth,
    census: &Census,
) -> Result<bool, String> {
    let baseline = compare_runs(reference, candidate, register, metadata_truth, census, None)?;
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
            metadata_truth,
            census,
            Some((&target, mutation)),
        );
        match check(mutation, &target, &outcome)
            .and_then(|()| check_render_hash_changed(mutation, &target, &baseline, &outcome))
        {
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

/// Every mutation that changes frame bytes must also change the stable hash
/// for the mutated side. The catalogue already proves the comparator sees the
/// damage. This makes it prove the F-015 hook sees the same damage too.
fn check_render_hash_changed(
    mutation: &Mutation,
    target: &str,
    baseline: &RunReport,
    outcome: &Result<RunReport, String>,
) -> Result<(), String> {
    if !touches_the_frame(mutation) {
        return Ok(());
    }
    let Ok(mutated) = outcome else {
        return Ok(());
    };
    let before = baseline
        .records
        .iter()
        .find(|record| record.id == target)
        .ok_or_else(|| format!("{target} is not in the baseline hash report"))?;
    let after = mutated
        .records
        .iter()
        .find(|record| record.id == target)
        .ok_or_else(|| format!("{target} is not in the mutated hash report"))?;
    let (before_hash, after_hash) = match mutation.side {
        MutatedSide::Reference => (&before.reference_render_hash, &after.reference_render_hash),
        MutatedSide::Candidate => (&before.candidate_render_hash, &after.candidate_render_hash),
    };
    if before_hash == after_hash {
        return Err(format!(
            "{target}: frame bytes changed and the {RENDER_HASH_ALGORITHM} hash did not"
        ));
    }
    Ok(())
}

/// The catalogue entry that carries the real divergence. Looked up by name
/// rather than rebuilt here, so the census measures the mutation the oracle
/// runs and not a second copy of it.
const SWAP: &str = "the-actual-linear-exact-swap";

/// One gating view's answer to "can the bias bound see the divergence here?".
struct Detectability {
    id: String,
    window_width: u32,
    informative_bias: f64,
    image_bias: f64,
}

/// Apply the real swap to every gating class-one view and print what it moves.
///
/// **This is a measurement and not a gate**, and it exits 0 whatever it finds.
/// It exists because the counts in `tolerance.rs`, `attribution.rs` and
/// `docs/lld/comparator.md` describing which views the bound can reach were
/// written from a model rather than from the mutation, and the model was wrong
/// three times running.
///
/// Two regions are reported per view because both appear in tracked prose. The
/// informative-region figure is what the comparator actually gates on. The
/// image-rectangle figure is what the same swap would produce over the region
/// the bullet named before the sprint review's second pass, and it is the
/// evidence that the region choice is load-bearing.
fn detectability(
    reference: &Run,
    candidate: &Run,
    register: &Register,
    metadata_truth: &MetadataTruth,
    census: &Census,
) -> Result<bool, String> {
    let baseline = compare_runs(reference, candidate, register, metadata_truth, census, None)?;
    let swap = CATALOGUE
        .iter()
        .find(|mutation| mutation.name == SWAP)
        .ok_or_else(|| format!("{SWAP} is not in the catalogue"))?;

    let mut measured: Vec<Detectability> = Vec::new();
    let mut declined: Vec<String> = Vec::new();
    for record in &baseline.records {
        if record.class != ToleranceClass::MonochromeSixteenBit || record.outcome != Outcome::Pass {
            continue;
        }
        let sidecar = reference
            .sidecars
            .get(&record.id)
            .ok_or_else(|| format!("{}: no sidecar", record.id))?;
        let original = reference
            .read_frame(&record.id)
            .map_err(|error| error.to_string())?;
        let rect = image_rect_for(sidecar.kind, sidecar, original.width(), original.height())
            .map_err(|error| error.to_string())?;
        // **`as_u64` is `None` for a JSON number written with a decimal
        // point**, so a reference half that emitted `741.0`, or a DICOM `DS`
        // window width of `741.5`, lands its view in `declined` rather than
        // `measured` and the census reports one fewer view than the corpus
        // has. Today every sidecar's `windowWidth` is integral, so nothing is
        // lost, and the totals printed below make a change visible rather than
        // silent. Widening this to `as_f64` is a rounding decision on a
        // window width, which HLD 27.3 makes a human review item, so it is
        // stated here rather than taken.
        let window_width = sidecar
            .json
            .pointer("/voi/windowWidth")
            .and_then(Value::as_u64)
            .and_then(|width| u32::try_from(width).ok());
        let mut swapped = original.clone();
        if let Err(error) = apply_to_frame(swap, &mut swapped, &rect, window_width) {
            declined.push(format!("{}: {error}", record.id));
            continue;
        }
        let diff = difference(&original, &swapped, rect, ChannelSet::Monochrome)
            .map_err(|error| error.to_string())?;
        let informative = diff
            .informative
            .channel(0)
            .ok_or_else(|| format!("{}: no informative channel", record.id))?;
        let image = diff
            .image
            .channel(0)
            .ok_or_else(|| format!("{}: no image-rectangle channel", record.id))?;
        measured.push(Detectability {
            id: record.id.clone(),
            window_width: window_width.unwrap_or(0),
            informative_bias: informative
                .signed_mean_diff()
                .map_err(|error| error.to_string())?,
            image_bias: image
                .signed_mean_diff()
                .map_err(|error| error.to_string())?,
        });
    }

    let bound = MONOCHROME_SIGNED_MEAN_BIAS;
    println!("id\twindowWidth\tbiasInformative\tbiasImageRect");
    for row in &measured {
        println!(
            "{}\t{}\t{:.4}\t{:.4}",
            row.id, row.window_width, row.informative_bias, row.image_bias
        );
    }
    for message in &declined {
        println!("DECLINED {message}");
    }

    let can_fail = measured
        .iter()
        .filter(|row| row.informative_bias.abs() > bound)
        .count();
    let blind: Vec<&Detectability> = measured
        .iter()
        .filter(|row| row.informative_bias.abs() <= bound)
        .collect();
    let over_the_rectangle = measured
        .iter()
        .filter(|row| row.image_bias.abs() > bound)
        .count();
    let worst_rectangle = measured
        .iter()
        .max_by(|a, b| a.image_bias.abs().total_cmp(&b.image_bias.abs()));

    // **The totals add up, and until the sprint review's fifth pass they only
    // appeared to.** `gating` counted the declined views as well, while
    // `can_fail` and `cannot` partition the MEASURED ones alone, so the moment
    // any view declined the swap the three printed numbers stopped summing and
    // said nothing about it. Both counts are named.
    let gating = measured.len().saturating_add(declined.len());
    println!(
        "gating class-one views: {gating} ({} measured, {} declined)",
        measured.len(),
        declined.len()
    );
    println!("can fail the bias bound over the informative region: {can_fail}");
    println!(
        "cannot: {} (can-fail plus cannot is the {} measured)",
        blind.len(),
        measured.len()
    );
    if let Some(smallest) = blind.iter().map(|row| row.window_width).min() {
        println!("smallest blind window: {smallest}");
    }
    for row in &blind {
        println!(
            "  BLIND {} w={} bias={:.4}",
            row.id, row.window_width, row.informative_bias
        );
    }
    println!("exceed the bound over the IMAGE RECTANGLE instead: {over_the_rectangle}");
    if let Some(worst) = worst_rectangle {
        println!(
            "largest image-rectangle bias: {:.4} on {}",
            worst.image_bias, worst.id
        );
    }
    Ok(true)
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
                .chain(&report.coverage_problems)
                .any(|problem| problem.contains(fragment))
            {
                Ok(())
            } else {
                Err(format!(
                    "expected a run problem carrying {fragment:?}, got problems {:?} and coverage problems {:?}",
                    report.problems, report.coverage_problems
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

#[cfg(test)]
mod argument_tests {
    use std::path::PathBuf;

    use ocelli_oracle::{mutations::CATALOGUE, report::RunReport};

    use super::{check, parse_arguments_from, validate_gate_directories};

    fn parsed<const N: usize>(arguments: [&str; N]) -> Result<super::Arguments, String> {
        parse_arguments_from(arguments)
    }

    #[test]
    fn gate_requires_an_explicit_reference() -> Result<(), String> {
        let arguments = parsed(["gate", "--candidate", "candidate"])?;
        let Err(error) = validate_gate_directories(&arguments) else {
            return Err("an implicit reference was accepted".to_owned());
        };
        assert!(error.contains("explicit --reference"));
        Ok(())
    }

    #[test]
    fn gate_requires_an_explicit_candidate() -> Result<(), String> {
        let arguments = parsed(["gate", "--reference", "reference"])?;
        let Err(error) = validate_gate_directories(&arguments) else {
            return Err("an implicit candidate was accepted".to_owned());
        };
        assert!(error.contains("explicit --candidate"));
        Ok(())
    }

    #[test]
    fn gate_refuses_two_spellings_of_the_same_directory() -> Result<(), String> {
        let root = std::env::temp_dir().join(format!("ocelli-f012-gate-{}", std::process::id()));
        std::fs::create_dir_all(&root).map_err(|error| error.to_string())?;
        let dotted = root.join(".");
        let arguments = parse_arguments_from([
            "gate".into(),
            "--reference".into(),
            root.as_os_str().to_owned(),
            "--candidate".into(),
            dotted.as_os_str().to_owned(),
        ])?;
        let Err(error) = validate_gate_directories(&arguments) else {
            return Err("two spellings of one directory were accepted".to_owned());
        };
        assert!(error.contains("resolve to different directories"));
        std::fs::remove_dir(&root).map_err(|error| error.to_string())?;
        Ok(())
    }

    #[test]
    fn gate_accepts_two_distinct_existing_directories() -> Result<(), String> {
        let root =
            std::env::temp_dir().join(format!("ocelli-f012-gate-distinct-{}", std::process::id()));
        let reference = root.join("reference");
        let candidate = root.join("candidate");
        std::fs::create_dir_all(&reference).map_err(|error| error.to_string())?;
        std::fs::create_dir_all(&candidate).map_err(|error| error.to_string())?;
        let arguments = parse_arguments_from([
            "gate".into(),
            "--reference".into(),
            reference.as_os_str().to_owned(),
            "--candidate".into(),
            candidate.as_os_str().to_owned(),
        ])?;
        validate_gate_directories(&arguments)?;
        std::fs::remove_dir(&reference).map_err(|error| error.to_string())?;
        std::fs::remove_dir(&candidate).map_err(|error| error.to_string())?;
        std::fs::remove_dir(&root).map_err(|error| error.to_string())?;
        Ok(())
    }

    #[test]
    fn gate_refuses_output_equal_to_an_input_directory() -> Result<(), String> {
        let root = std::env::temp_dir().join(format!(
            "ocelli-f012-gate-output-equal-{}",
            std::process::id()
        ));
        let reference = root.join("reference");
        let candidate = root.join("candidate");
        std::fs::create_dir_all(&reference).map_err(|error| error.to_string())?;
        std::fs::create_dir_all(&candidate).map_err(|error| error.to_string())?;
        let arguments = parse_arguments_from([
            "gate".into(),
            "--reference".into(),
            reference.as_os_str().to_owned(),
            "--candidate".into(),
            candidate.as_os_str().to_owned(),
            "--out".into(),
            reference.as_os_str().to_owned(),
        ])?;
        let Err(error) = validate_gate_directories(&arguments) else {
            return Err("an output equal to the reference was accepted".to_owned());
        };
        assert!(error.contains("`--out` must be separate from the reference"));
        std::fs::remove_dir_all(&root).map_err(|error| error.to_string())?;
        Ok(())
    }

    #[test]
    fn gate_refuses_output_inside_an_input_directory() -> Result<(), String> {
        let root = std::env::temp_dir().join(format!(
            "ocelli-f012-gate-output-inside-{}",
            std::process::id()
        ));
        let reference = root.join("reference");
        let candidate = root.join("candidate");
        std::fs::create_dir_all(&reference).map_err(|error| error.to_string())?;
        std::fs::create_dir_all(&candidate).map_err(|error| error.to_string())?;
        let out = candidate.join("reports").join("latest");
        let arguments = parse_arguments_from([
            "gate".into(),
            "--reference".into(),
            reference.as_os_str().to_owned(),
            "--candidate".into(),
            candidate.as_os_str().to_owned(),
            "--out".into(),
            out.as_os_str().to_owned(),
        ])?;
        let Err(error) = validate_gate_directories(&arguments) else {
            return Err("an output inside the candidate was accepted".to_owned());
        };
        assert!(error.contains("`--out` must be separate from the candidate"));
        std::fs::remove_dir_all(&root).map_err(|error| error.to_string())?;
        Ok(())
    }

    #[test]
    fn gate_refuses_output_containing_an_input_directory() -> Result<(), String> {
        let root = std::env::temp_dir().join(format!(
            "ocelli-f012-gate-output-ancestor-{}",
            std::process::id()
        ));
        let reference = root.join("reference");
        let candidate = root.join("candidate");
        std::fs::create_dir_all(&reference).map_err(|error| error.to_string())?;
        std::fs::create_dir_all(&candidate).map_err(|error| error.to_string())?;
        let arguments = parse_arguments_from([
            "gate".into(),
            "--reference".into(),
            reference.as_os_str().to_owned(),
            "--candidate".into(),
            candidate.as_os_str().to_owned(),
            "--out".into(),
            root.as_os_str().to_owned(),
        ])?;
        let Err(error) = validate_gate_directories(&arguments) else {
            return Err("an output containing both inputs was accepted".to_owned());
        };
        assert!(error.contains("neither may contain the other"));
        std::fs::remove_dir_all(&root).map_err(|error| error.to_string())?;
        Ok(())
    }

    #[test]
    fn a_non_gate_command_keeps_the_identity_default() -> Result<(), String> {
        let arguments = parsed(["identity", "--reference", "reference"])?;
        assert_eq!(arguments.reference, PathBuf::from("reference"));
        assert_eq!(arguments.candidate, PathBuf::from("reference"));
        Ok(())
    }

    #[test]
    fn a_coverage_problem_satisfies_a_run_problem_mutation() -> Result<(), String> {
        let mutation = CATALOGUE
            .iter()
            .find(|mutation| mutation.name == "a-view-missing-from-the-candidate")
            .ok_or_else(|| "the missing-view mutation is absent".to_owned())?;
        let report = RunReport {
            records: Vec::new(),
            problems: Vec::new(),
            coverage_problems: vec!["a is declared on one side and not the other".to_owned()],
            absent_views: 1,
            unsupported_source_rows: 0,
            declared_volume_refusals: 0,
            reference_directory: "reference".to_owned(),
            candidate_directory: "candidate".to_owned(),
        };

        check(mutation, "a", &Ok(report))
    }
}
