#!/usr/bin/env python3
"""Verification evidence, recorded against a TREE and carried in a trailer.

This file is the mechanism behind two things at once.

**HLD section 27.2 R6, the provenance trailer.** "Cheap now, a retrofit across
sixty thousand lines is not, and a device pathway may require it." Under
agent-assisted development this is what lets you answer a diligence question
about what was generated and how it was verified.

**Deviation D-04**, `docs/hld/DEVIATIONS.md`. CI runs no GPU, so the corpus
renders locally. That moves the gate onto a human remembering to run it, and
this is what stops it being a memory problem.

## Why the TREE hash and not the commit

A trailer cannot name the commit it is part of, and a ledger keyed on a commit
sha is written after the fact by definition. The tree hash is available BEFORE
the commit exists, is identical for an amend that changes only the message, and
changes the instant one byte of content changes.

So the chain is:

    /verify runs the gates  ->  records the result against `git write-tree`
    .githooks/pre-commit    ->  looks up the staged tree, refuses if absent,
                                appends the trailer it found there
    CI (no GPU)             ->  re-reads the trailer on the pushed head

The trailer cannot be hand-written to satisfy CI, because the hook only emits
one it found in a ledger entry that a real gate run wrote. That matters more
than it looks: a check the process CLAIMS to run and does not is worth LESS
than no check at all, because the records then read as though it ran and
nobody goes looking.

The ledger is per-clone evidence and is gitignored. The TRAILER is the shared,
committed artefact.

Usage:
  python3 scripts/verify_ledger.py record --gates fmt,clippy --corpus pass
  python3 scripts/verify_ledger.py assert                 # staged tree is green
  python3 scripts/verify_ledger.py trailer                # emit trailer lines
  python3 scripts/verify_ledger.py check-commit HEAD      # CI side, no GPU
"""

from __future__ import annotations

import argparse
import collections
import hashlib
import json
import math
import os
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
LEDGER = ROOT / ".claude" / "verify-ledger.json"

TRAILER_VERIFY = "Ocelli-Verify"
TRAILER_AGENT = "Ocelli-Generated-By"
CORPUS_STATES = {"pass", "fail", "absent", "skipped"}

REPORT_KEYS = {
    "story", "reference", "candidate", "views", "pass", "fail",
    "unmeasured", "absent", "claimedVerdictViews", "coverage",
    "gateVerdict", "qualifiers", "problems", "coverageProblems",
    "absorbedDivergences", "green", "renderHashes", "records", "operation",
}
COVERAGE_KEYS = {
    "unmeasured", "absent", "unsupportedSourceRows", "declaredVolumeRefusals",
}
RECORD_KEYS = {
    "id", "kind", "toleranceClass", "outcome", "qualifiers",
    "attributedTo", "rung", "notes", "parameterDivergences",
    "geometryDivergences", "referenceDivergenceEntry", "renderHashes",
    "monochromeFrame", "statistics",
}
STATISTICS_KEYS = {
    "channels", "full", "image", "background", "informative",
    "rowsTouched", "columnsTouched", "imagePixels", "informativePixels",
    "informativeFraction", "predicatePasses", "biasPasses", "signedMeanDiff",
}
CHANNEL_KEYS = {
    "pixels", "maxAbsDiff", "countAtZero", "countAtOne", "countAtTwo",
    "countOverTwo", "fractionWithinOneLsb", "differingFraction",
    "signedMeanDiff", "percentile999AbsDiff",
}
PARAMETER_DIVERGENCE_KEYS = {
    "field", "reference", "candidate", "attributedTo", "why",
}
GEOMETRY_DIVERGENCE_KEYS = {
    "field", "reference", "candidate", "difference", "bound",
}
RENDER_HASH_KEYS = {"algorithm", "reference", "candidate"}
HASH_ALGORITHM = "sha256-rgba8-v1"
HASH_PATTERN = re.compile(r"[0-9a-f]{64}")
KINDS = {"stack": 0, "volume-reformat": 1}
CLASSES = {"mono16": 1, "colour-or-us": 3}
OUTCOMES = {"pass", "fail", "unmeasured", "absent"}
SIDES = {"inputs", "reference", "ours", "fit", "none", "unattributed"}
RUNGS = {
    "metadata-truth", "parameters", "register", "volume-divergence",
    "geometry", "class-two", "letterbox", "pixels", "decimated", "weak",
}
QUALIFIERS = (
    "weak", "decimated", "unstated-threshold", "parameter-divergence",
    "metadata-truth", "geometry-divergence", "reference-divergence", "bias",
    "letterbox-only", "divergent-while-unmeasured",
)
GREEN_UNMEASURED_QUALIFIERS = {"weak", "decimated", "unstated-threshold"}
U64_MAX = (1 << 64) - 1
U32_MAX = (1 << 32) - 1


def git(*args: str) -> str:
    return subprocess.run(["git", *args], cwd=ROOT, capture_output=True,
                          text=True, check=True).stdout.strip()


def staged_tree() -> str:
    """Tree hash of the index. Available before the commit exists."""
    return git("write-tree")


def load() -> dict:
    if not LEDGER.exists():
        return {}
    try:
        return json.loads(LEDGER.read_text())
    except json.JSONDecodeError:
        return {}


def save(data: dict) -> None:
    LEDGER.parent.mkdir(parents=True, exist_ok=True)
    LEDGER.write_text(json.dumps(data, indent=1, sort_keys=True) + "\n")


class DuplicateJsonKey(ValueError):
    pass


def _unique_object(pairs: list[tuple[str, object]]) -> dict:
    result = {}
    for key, value in pairs:
        if key in result:
            raise DuplicateJsonKey(f"duplicate JSON key {key!r}")
        result[key] = value
    return result


def _invalid_constant(value: str) -> None:
    raise ValueError(f"non-finite JSON number {value}")


def _schema(value: object, label: str, keys: set[str]) -> dict:
    if not isinstance(value, dict):
        sys.exit(f"comparison report {label} is not an object")
    missing = sorted(keys - value.keys())
    unknown = sorted(value.keys() - keys)
    if missing or unknown:
        sys.exit(
            f"comparison report {label} has invalid keys, "
            f"missing={missing}, unknown={unknown}"
        )
    return value


def _integer(value: object, label: str, *, maximum: int = U64_MAX) -> int:
    if (isinstance(value, bool) or not isinstance(value, int)
            or value < 0 or value > maximum):
        sys.exit(f"comparison report has invalid {label}")
    return value


def _number(value: object, label: str) -> float:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        sys.exit(f"comparison report has invalid {label}")
    try:
        finite = math.isfinite(value)
    except OverflowError:
        finite = False
    if not finite:
        sys.exit(f"comparison report has invalid {label}")
    return float(value)


def _text(value: object, label: str) -> str:
    if not isinstance(value, str) or not value:
        sys.exit(f"comparison report has invalid {label}")
    return value


def _array(value: object, label: str) -> list:
    if not isinstance(value, list):
        sys.exit(f"comparison report has no {label} array")
    return value


def _hash(value: object, label: str) -> str:
    if not isinstance(value, str) or HASH_PATTERN.fullmatch(value) is None:
        sys.exit(f"comparison report has invalid {label} render hash")
    return value


def _channel_report(value: object, label: str) -> dict:
    channel = _schema(value, label, CHANNEL_KEYS)
    pixels = _integer(channel["pixels"], f"{label}.pixels")
    counts = [
        _integer(channel[key], f"{label}.{key}")
        for key in ("countAtZero", "countAtOne", "countAtTwo", "countOverTwo")
    ]
    if sum(counts) != pixels:
        sys.exit(f"comparison report {label} channel counts do not total pixels")
    for key in ("maxAbsDiff", "percentile999AbsDiff"):
        _integer(channel[key], f"{label}.{key}", maximum=255)
    within = _number(channel["fractionWithinOneLsb"],
                     f"{label}.fractionWithinOneLsb")
    differing = _number(channel["differingFraction"],
                        f"{label}.differingFraction")
    _number(channel["signedMeanDiff"], f"{label}.signedMeanDiff")
    if pixels == 0 or not 0.0 <= within <= 1.0 or not 0.0 <= differing <= 1.0:
        sys.exit(f"comparison report has invalid {label} channel fractions")
    expected_within = (counts[0] + counts[1]) / pixels
    expected_differing = (pixels - counts[0]) / pixels
    if within != expected_within or differing != expected_differing:
        sys.exit(f"comparison report {label} channel fractions contradict counts")
    return channel


def _statistics(value: object, record_id: str, tolerance_class: str) -> dict:
    label = f"record {record_id!r} statistics"
    statistics = _schema(value, label, STATISTICS_KEYS)
    channels = _integer(statistics["channels"], f"{label}.channels")
    if channels != CLASSES[tolerance_class]:
        sys.exit(f"comparison report {label} channels contradict tolerance class")

    regions = {}
    for region in ("full", "image", "background", "informative"):
        raw = _array(statistics[region], f"{label}.{region}")
        if region in ("full", "image") and len(raw) != channels:
            sys.exit(f"comparison report {label}.{region} has the wrong channel count")
        if region in ("background", "informative") and len(raw) not in (0, channels):
            sys.exit(f"comparison report {label}.{region} has the wrong channel count")
        regions[region] = [
            _channel_report(entry, f"{label}.{region}[{index}]")
            for index, entry in enumerate(raw)
        ]

    rows = _integer(statistics["rowsTouched"], f"{label}.rowsTouched",
                    maximum=U32_MAX)
    columns = _integer(statistics["columnsTouched"],
                       f"{label}.columnsTouched", maximum=U32_MAX)
    image_pixels = _integer(statistics["imagePixels"], f"{label}.imagePixels")
    informative_pixels = _integer(
        statistics["informativePixels"], f"{label}.informativePixels")
    informative_fraction = _number(
        statistics["informativeFraction"], f"{label}.informativeFraction")
    if image_pixels == 0 or informative_pixels > image_pixels:
        sys.exit(f"comparison report {label} has invalid region pixel counts")
    if informative_fraction != informative_pixels / image_pixels:
        sys.exit(f"comparison report {label} informative fraction contradicts counts")
    if not isinstance(statistics["predicatePasses"], bool):
        sys.exit(f"comparison report has invalid {label}.predicatePasses")
    if not isinstance(statistics["biasPasses"], bool):
        sys.exit(f"comparison report has invalid {label}.biasPasses")
    _number(statistics["signedMeanDiff"], f"{label}.signedMeanDiff")

    full_pixels = {entry["pixels"] for entry in regions["full"]}
    image_region_pixels = {entry["pixels"] for entry in regions["image"]}
    background_pixels = {entry["pixels"] for entry in regions["background"]}
    informative_region_pixels = {entry["pixels"] for entry in regions["informative"]}
    if len(full_pixels) != 1 or image_region_pixels != {image_pixels}:
        sys.exit(f"comparison report {label} region totals contradict channel reports")
    expected_background = next(iter(full_pixels)) - image_pixels
    if expected_background < 0:
        sys.exit(f"comparison report {label} image exceeds the full frame")
    if background_pixels not in (set(), {expected_background}):
        sys.exit(f"comparison report {label} background total is inconsistent")
    if expected_background > 0 and not background_pixels:
        sys.exit(f"comparison report {label} omits non-empty background statistics")
    expected_informative = set() if informative_pixels == 0 else {informative_pixels}
    if informative_region_pixels != expected_informative:
        sys.exit(f"comparison report {label} informative total is inconsistent")
    if rows > next(iter(full_pixels)) or columns > next(iter(full_pixels)):
        sys.exit(f"comparison report {label} touched counts exceed the frame")
    return statistics


def _parameter_divergence(value: object, label: str) -> None:
    divergence = _schema(value, label, PARAMETER_DIVERGENCE_KEYS)
    _text(divergence["field"], f"{label}.field")
    if (not isinstance(divergence["attributedTo"], str)
            or divergence["attributedTo"] not in SIDES):
        sys.exit(f"comparison report has invalid {label}.attributedTo")
    _text(divergence["why"], f"{label}.why")


def _geometry_divergence(value: object, label: str) -> None:
    divergence = _schema(value, label, GEOMETRY_DIVERGENCE_KEYS)
    _text(divergence["field"], f"{label}.field")
    for key in ("reference", "candidate", "difference", "bound"):
        number = _number(divergence[key], f"{label}.{key}")
        if key in ("difference", "bound") and number < 0:
            sys.exit(f"comparison report has invalid {label}.{key}")


def _record(value: object, index: int) -> dict:
    label = f"records[{index}]"
    record = _schema(value, label, RECORD_KEYS)
    record_id = _text(record["id"], f"{label}.id")
    kind = record["kind"]
    tolerance_class = record["toleranceClass"]
    outcome = record["outcome"]
    if not isinstance(kind, str) or kind not in KINDS:
        sys.exit(f"comparison report has invalid {label}.kind")
    if not isinstance(tolerance_class, str) or tolerance_class not in CLASSES:
        sys.exit(f"comparison report has invalid {label}.toleranceClass")
    if not isinstance(outcome, str) or outcome not in OUTCOMES:
        sys.exit(f"comparison report has invalid {label}.outcome")
    if (not isinstance(record["attributedTo"], str)
            or record["attributedTo"] not in SIDES):
        sys.exit(f"comparison report has invalid {label}.attributedTo")
    if not isinstance(record["rung"], str) or record["rung"] not in RUNGS:
        sys.exit(f"comparison report has invalid {label}.rung")
    if not isinstance(record["monochromeFrame"], bool):
        sys.exit(f"comparison report has invalid {label}.monochromeFrame")

    qualifiers = _array(record["qualifiers"], f"{label}.qualifiers")
    if (any(not isinstance(item, str) or item not in QUALIFIERS
            for item in qualifiers)
            or len(set(qualifiers)) != len(qualifiers)
            or qualifiers != sorted(qualifiers, key=QUALIFIERS.index)):
        sys.exit(f"comparison report has invalid {label}.qualifiers")
    notes = _array(record["notes"], f"{label}.notes")
    if any(not isinstance(note, str) or not note for note in notes):
        sys.exit(f"comparison report has invalid {label}.notes")

    parameters = _array(record["parameterDivergences"],
                        f"{label}.parameterDivergences")
    for item_index, item in enumerate(parameters):
        _parameter_divergence(item, f"{label}.parameterDivergences[{item_index}]")
    geometry = _array(record["geometryDivergences"],
                      f"{label}.geometryDivergences")
    for item_index, item in enumerate(geometry):
        _geometry_divergence(item, f"{label}.geometryDivergences[{item_index}]")
    register_entry = record["referenceDivergenceEntry"]
    if register_entry is not None:
        _text(register_entry, f"{label}.referenceDivergenceEntry")

    hashes = _schema(record["renderHashes"], f"{label}.renderHashes",
                     RENDER_HASH_KEYS)
    if hashes["algorithm"] != HASH_ALGORITHM:
        sys.exit(f"comparison report has invalid {label} render hash algorithm")
    _hash(hashes["reference"], f"{label} reference")
    _hash(hashes["candidate"], f"{label} candidate")
    _statistics(record["statistics"], record_id, tolerance_class)

    if not record["statistics"]["predicatePasses"] or not record["statistics"]["biasPasses"]:
        sys.exit(f"comparison report green record {record_id!r} has a failed predicate")
    if parameters or geometry or register_entry is not None:
        sys.exit(f"comparison report green record {record_id!r} has divergence details")
    if outcome == "pass":
        if qualifiers or record["attributedTo"] != "none" or record["rung"] != "pixels" or notes:
            sys.exit(f"comparison report pass record {record_id!r} is inconsistent")
    elif outcome == "unmeasured":
        if (not qualifiers or not set(qualifiers) <= GREEN_UNMEASURED_QUALIFIERS
                or record["attributedTo"] != "none" or not notes):
            sys.exit(f"comparison report unmeasured record {record_id!r} is inconsistent")
        expected_rung = "class-two" if "unstated-threshold" in qualifiers else None
        if expected_rung is not None and record["rung"] != expected_rung:
            sys.exit(f"comparison report unmeasured record {record_id!r} has the wrong rung")
        if expected_rung is None and record["rung"] not in ("weak", "decimated"):
            sys.exit(f"comparison report unmeasured record {record_id!r} has the wrong rung")
    else:
        sys.exit(f"comparison report green record {record_id!r} has outcome {outcome}")
    return record


def _run_hash(records: list[dict], side: str) -> str:
    entries = sorted(
        records,
        key=lambda record: (
            KINDS[record["kind"]], record["id"], record["renderHashes"][side]
        ),
    )
    digest = hashlib.sha256()
    digest.update(b"sha256-rgba8-run-v1\0")
    digest.update(len(entries).to_bytes(8, "little"))
    for record in entries:
        for field in (record["kind"], record["id"], record["renderHashes"][side]):
            encoded = field.encode()
            digest.update(len(encoded).to_bytes(8, "little"))
            digest.update(encoded)
    return digest.hexdigest()


def comparison_evidence(path: str) -> dict:
    report_path = Path(path)
    try:
        encoded = report_path.read_bytes()
    except OSError as error:
        sys.exit(f"comparison report cannot be read: {report_path}: {error}")
    try:
        report = json.loads(
            encoded,
            object_pairs_hook=_unique_object,
            parse_constant=_invalid_constant,
        )
    except (json.JSONDecodeError, DuplicateJsonKey, ValueError) as error:
        sys.exit(f"comparison report is not valid JSON: {report_path}: {error}")
    report = _schema(report, "root", REPORT_KEYS)
    if report.get("operation") != "gate":
        sys.exit("comparison report was not produced by the explicit candidate gate")
    if report.get("gateVerdict") != "pass" or report.get("green") is not True:
        sys.exit("comparison report is not green")

    if report["story"] != "F-011, F-012, F-015":
        sys.exit("comparison report has an invalid story set")
    reference = _text(report["reference"], "reference directory")
    candidate = _text(report["candidate"], "candidate directory")
    if reference == candidate:
        sys.exit("comparison report reference and candidate directories are equal")

    claimed = _integer(report["claimedVerdictViews"], "claimed verdict count")
    if claimed <= 0:
        sys.exit("comparison report judged zero views or has an invalid count")
    summaries = {
        outcome: _integer(report[outcome], f"{outcome} count")
        for outcome in ("pass", "fail", "unmeasured", "absent")
    }

    empty_arrays = {
        "problems": "problems",
        "coverageProblems": "coverage problems",
        "absorbedDivergences": "absorbed divergences",
    }
    for field, label in empty_arrays.items():
        value = report.get(field)
        if not isinstance(value, list):
            sys.exit(f"comparison report has no {label} array")
        if value:
            sys.exit(f"comparison report is green but has {label}")

    if summaries["absent"] != 0:
        sys.exit("comparison report has absent views")
    if summaries["fail"] != 0:
        sys.exit("comparison report is green but has failed views")

    coverage = _schema(report["coverage"], "coverage", COVERAGE_KEYS)
    coverage_counts = {
        field: _integer(coverage[field], f"coverage count for {field}")
        for field in COVERAGE_KEYS
    }
    if coverage_counts["absent"] != 0:
        sys.exit("comparison report has absent views")
    if summaries["unmeasured"] != coverage_counts["unmeasured"]:
        sys.exit("comparison report unmeasured count disagrees with coverage")

    records_raw = _array(report["records"], "records")
    if not records_raw:
        sys.exit("comparison report records array is empty")
    records = [_record(value, index) for index, value in enumerate(records_raw)]
    identifiers = [record["id"] for record in records]
    if len(set(identifiers)) != len(identifiers):
        sys.exit("comparison report has duplicate record identifiers")
    if identifiers != sorted(identifiers):
        sys.exit("comparison report records are not in serializer order")

    derived = collections.Counter(record["outcome"] for record in records)
    views = _integer(report["views"], "views count")
    if views != len(records):
        sys.exit("comparison report views count is not the record count")
    for outcome in ("pass", "fail", "unmeasured", "absent"):
        if summaries[outcome] != derived[outcome]:
            sys.exit(f"comparison report {outcome} count contradicts records")
    if summaries["pass"] + summaries["fail"] != claimed:
        sys.exit("comparison report claimed count is not pass plus fail")
    qualifiers = report["qualifiers"]
    if not isinstance(qualifiers, dict):
        sys.exit("comparison report qualifiers is not an object")
    if any(key not in QUALIFIERS for key in qualifiers):
        sys.exit("comparison report qualifiers has an unknown key")
    for key, value in qualifiers.items():
        if _integer(value, f"qualifier count for {key}") <= 0:
            sys.exit(f"comparison report has invalid qualifier count for {key}")
    derived_qualifiers = collections.Counter(
        qualifier for record in records for qualifier in record["qualifiers"]
    )
    if qualifiers != dict(derived_qualifiers):
        sys.exit("comparison report qualifier histogram contradicts records")

    hashes = _schema(report["renderHashes"], "renderHashes", RENDER_HASH_KEYS)
    if hashes["algorithm"] != HASH_ALGORITHM:
        sys.exit("comparison report has invalid aggregate render hash algorithm")
    for side in ("reference", "candidate"):
        aggregate = _hash(hashes[side], f"aggregate {side}")
        if aggregate != _run_hash(records, side):
            sys.exit(f"comparison report aggregate {side} render hash contradicts records")

    return {
        "reportSha256": hashlib.sha256(encoded).hexdigest(),
        "claimedVerdictViews": claimed,
        "verdict": "pass",
    }


def valid_comparison(entry: dict) -> bool:
    comparison = entry.get("comparison")
    if not isinstance(comparison, dict):
        return False
    digest = comparison.get("reportSha256")
    claimed = comparison.get("claimedVerdictViews")
    return (isinstance(digest, str)
            and re.fullmatch(r"[0-9a-f]{64}", digest) is not None
            and isinstance(claimed, int) and not isinstance(claimed, bool)
            and claimed > 0
            and comparison.get("verdict") == "pass")


def cmd_record(args: argparse.Namespace) -> int:
    if args.corpus not in CORPUS_STATES:
        sys.exit(f"--corpus must be one of {sorted(CORPUS_STATES)}")
    tree = args.tree or staged_tree()
    data = load()
    entry = {
        "gates": sorted(set(filter(None, args.gates.split(",")))),
        "corpus": args.corpus,
        "profile": args.profile,
        "agent": args.agent or os.environ.get("OCELLI_AGENT", "unknown"),
    }
    if args.comparison_report:
        entry["comparison"] = comparison_evidence(args.comparison_report)
    data[tree] = entry
    save(data)
    print(f"recorded tree {tree[:12]} corpus={args.corpus} "
          f"profile={args.profile}")
    return 0


def entry_for(tree: str) -> dict | None:
    return load().get(tree)


def cmd_assert(args: argparse.Namespace) -> int:
    tree = args.tree or staged_tree()
    entry = entry_for(tree)
    if entry is None:
        print(f"FAIL: no verification recorded for the staged tree "
              f"{tree[:12]}.")
        print("Run `/verify` (or `bin/ocelli.sh gate --all`) and commit the")
        print("same tree. A record for an ancestor commit does not count,")
        print("because it is a claim about different content.")
        return 1
    if entry["corpus"] == "fail":
        print(f"FAIL: the corpus is RED for tree {tree[:12]}.")
        return 1
    if args.require_corpus and entry["corpus"] != "pass":
        print(f"FAIL: corpus is '{entry['corpus']}' for tree {tree[:12]}, "
              f"and this gate requires 'pass'.")
        print("The corpus is the mechanism that makes generated Rust safe to")
        print("merge at volume (HLD decision D7). Acquire it, see")
        print("corpus/README.md.")
        return 1
    if args.require_comparison and not valid_comparison(entry):
        print(f"FAIL: comparison evidence is required for tree {tree[:12]}.")
        print("Run the explicit candidate gate and record its compare.json.")
        return 1
    print(f"OK: tree {tree[:12]} verified, corpus={entry['corpus']}")
    return 0


def cmd_trailer(args: argparse.Namespace) -> int:
    tree = args.tree or staged_tree()
    entry = entry_for(tree)
    if entry is None:
        return 1
    comparison = entry.get("comparison")
    suffix = ""
    if valid_comparison(entry):
        suffix = (f" comparison={comparison['reportSha256']}"
                  f" comparison-views={comparison['claimedVerdictViews']}"
                  f" comparison-verdict={comparison['verdict']}")
    print(f"{TRAILER_VERIFY}: profile={entry['profile']} "
          f"gates={','.join(entry['gates'])} corpus={entry['corpus']} "
          f"tree={tree[:12]}{suffix}")
    print(f"{TRAILER_AGENT}: {entry['agent']}")
    return 0


def cmd_check_commit(args: argparse.Namespace) -> int:
    """CI side. Needs no GPU, no corpus and no ledger."""
    message = git("log", "-1", "--format=%B", args.rev)
    tree = git("rev-parse", f"{args.rev}^{{tree}}")

    verify_line = next(
        (l for l in message.splitlines() if l.startswith(f"{TRAILER_VERIFY}:")),
        None)
    if verify_line is None:
        print(f"FAIL: {args.rev} carries no {TRAILER_VERIFY} trailer.")
        print("Every commit records how it was verified (HLD 27.2 R6), and")
        print("with no GPU in CI this trailer is the only evidence CI has")
        print("that the corpus ran at all (DEVIATIONS.md D-04).")
        return 1

    fields = dict(
        part.split("=", 1) for part in verify_line.split(":", 1)[1].split()
        if "=" in part)

    if fields.get("tree") != tree[:12]:
        print(f"FAIL: {args.rev} trailer names tree {fields.get('tree')} "
              f"but the commit's tree is {tree[:12]}.")
        print("The trailer was carried over from a different tree, so it is")
        print("evidence about content that is not in this commit.")
        return 1

    corpus = fields.get("corpus")
    if corpus == "fail":
        print(f"FAIL: {args.rev} records a RED corpus.")
        return 1
    if args.require_corpus and corpus != "pass":
        print(f"FAIL: {args.rev} records corpus={corpus}, 'pass' required.")
        return 1

    comparison_fields = {
        key: fields.get(key)
        for key in ("comparison", "comparison-views", "comparison-verdict")
    }
    has_comparison_field = any(value is not None
                               for value in comparison_fields.values())
    comparison_valid = (
        re.fullmatch(r"[0-9a-f]{64}", comparison_fields["comparison"] or "")
        is not None
        and re.fullmatch(r"[1-9][0-9]*",
                         comparison_fields["comparison-views"] or "")
        is not None
        and comparison_fields["comparison-verdict"] == "pass"
    )
    if has_comparison_field and not comparison_valid:
        print(f"FAIL: {args.rev} carries malformed comparison evidence.")
        return 1
    if args.require_comparison and not comparison_valid:
        print(f"FAIL: {args.rev} comparison evidence is required.")
        return 1

    print(f"OK: {args.rev} verified, corpus={corpus}, tree matches")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser()
    sub = parser.add_subparsers(dest="command", required=True)

    p = sub.add_parser("record")
    p.add_argument("--tree")
    p.add_argument("--gates", default="")
    p.add_argument("--corpus", default="absent")
    p.add_argument("--profile", default="feature")
    p.add_argument("--agent", default="")
    p.add_argument("--comparison-report", default="")
    p.set_defaults(func=cmd_record)

    p = sub.add_parser("assert")
    p.add_argument("--tree")
    p.add_argument("--require-corpus", action="store_true")
    p.add_argument("--require-comparison", action="store_true")
    p.set_defaults(func=cmd_assert)

    p = sub.add_parser("trailer")
    p.add_argument("--tree")
    p.set_defaults(func=cmd_trailer)

    p = sub.add_parser("check-commit")
    p.add_argument("rev", nargs="?", default="HEAD")
    p.add_argument("--require-corpus", action="store_true")
    p.add_argument("--require-comparison", action="store_true")
    p.set_defaults(func=cmd_check_commit)

    args = parser.parse_args()
    return args.func(args)


if __name__ == "__main__":
    sys.exit(main())
