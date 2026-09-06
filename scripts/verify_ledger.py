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
import hashlib
import json
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


def comparison_evidence(path: str) -> dict:
    report_path = Path(path)
    try:
        encoded = report_path.read_bytes()
    except OSError as error:
        sys.exit(f"comparison report cannot be read: {report_path}: {error}")
    try:
        report = json.loads(encoded)
    except json.JSONDecodeError as error:
        sys.exit(f"comparison report is not valid JSON: {report_path}: {error}")
    if not isinstance(report, dict):
        sys.exit("comparison report is not a JSON object")
    if report.get("operation") != "gate":
        sys.exit("comparison report was not produced by the explicit candidate gate")
    if report.get("gateVerdict") != "pass" or report.get("green") is not True:
        sys.exit("comparison report is not green")

    claimed = report.get("claimedVerdictViews")
    if isinstance(claimed, bool) or not isinstance(claimed, int) or claimed <= 0:
        sys.exit("comparison report judged zero views or has an invalid count")
    passed = report.get("pass")
    failed = report.get("fail")
    if any(isinstance(value, bool) or not isinstance(value, int)
           or value < 0 for value in (passed, failed)):
        sys.exit("comparison report has invalid pass or fail counts")
    if passed + failed != claimed:
        sys.exit("comparison report claimed count is not pass plus fail")
    if failed != 0:
        sys.exit("comparison report is green but has failed views")

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

    top_level_absent = report.get("absent")
    if (isinstance(top_level_absent, bool)
            or not isinstance(top_level_absent, int)
            or top_level_absent != 0):
        sys.exit("comparison report has absent views")

    coverage = report.get("coverage")
    if not isinstance(coverage, dict):
        sys.exit("comparison report has no coverage object")
    absent = coverage.get("absent")
    if isinstance(absent, bool) or not isinstance(absent, int) or absent != 0:
        sys.exit("comparison report has absent views")

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
