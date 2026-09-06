#!/usr/bin/env python3
"""The benchmark harness's integrity. HLD section 26, made mechanical.

Section 26's last rule is:

    "Measure with the benchmark harness before optimising anything. The
     intuitions that work in JavaScript do not transfer."

The rule names an instrument and the HLD gives it no shape, so `tools/bench` is
the shape and this is the gate over it. It asserts the INSTRUMENT and never a
duration, which is what lets it sit in the CI floor: it costs no GPU, it is
deterministic, and it cannot be satisfied by intention.

## The defect this exists for

Most of the rows in `tools/bench/subjects.json` have nothing to measure in this
tree. There is no decoder, no renderer, no worker and no boundary, which is
decision D7 holding: the oracle and the instruments exist before the port code.
No count is written here, because a written count goes stale the first time a
story lands and one already did. `node tools/bench/run.mjs --list` prints the
split and reads `docs/sprints/BACKLOG.md` to do it.

**A benchmark harness under time pressure invents a workload, produces a
plausible number, and that number then sits in a tracked file describing
nothing.** That is the defect, it is this project's "quietly wrong" shape
applied to cost rather than to pixels, and discipline is not a mechanism against
it. So two things are refused mechanically:

1. a runner file for a subject whose story has not landed, because a runner for
   a subject that does not exist can only be timing a stub, and
2. an entry in `ci/bench-baseline.json` for such a subject, because that is
   where an invented number would come to rest.

`unavailable` is the correct output for those rows and it is a useful one: the
record names the F-ID a reader should go and read.

## What it deliberately does not assert

**Any duration.** `bin/ocelli.sh bench --compare` compares a figure against the
baseline on the machine that recorded it. A duration comparison on a shared CI
runner is either noise or a skip, and a skipped gate is not a pass here, so
putting it in the floor would produce a permanently amber gate and an amber gate
is a gate that gets disabled.

**That a runner measures the right thing.** No script can. What stops a later
story redefining a measurement into something easier to take is that
`subjects.json` fixes the definition NOW, from the specification, for every row
including the ones with no subject.

Usage: python3 scripts/bench_check.py
"""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
BENCH = ROOT / "tools" / "bench"
SUBJECTS = BENCH / "subjects.json"
RUNNERS = BENCH / "src" / "runners"
BENCH_PKG = BENCH / "package.json"
ORACLE_PKG = ROOT / "tools" / "oracle" / "package.json"
BASELINE = ROOT / "ci" / "bench-baseline.json"
ALLOCATION = ROOT / "docs" / "sprints" / "allocation.json"
BACKLOG = ROOT / "docs" / "sprints" / "BACKLOG.md"

REQUIRED_FIELDS = ("id", "title", "definition_hld", "definition", "unit",
                   "dimensions", "tiers", "subject_story", "feeds")

# `n/a` is a legitimate answer and an omitted row is not. Deviation D-07.
VALID_TIERS = ("A", "B", "C", "n/a")

# Mirrors scripts/backlog_check.py.
VALID_STATUS = ("pending", "in-progress", "done", "archived", "superseded")
BACKLOG_ROW = re.compile(r"^\|\s*(F-X?\d{3}[a-z]?)\s*\|(.*)$")

# The fields ci/bench-baseline.json's host_class block must carry. Mirrors
# HOST_CLASS_FIELDS in tools/bench/src/hostclass.mjs. The oracle's run.json
# carries the first three and none of the last three, which is enough to
# identify a reference environment for pixels and not enough to normalise a
# duration.
HOST_CLASS_FIELDS = ("platform", "release", "arch", "cpu_model", "cpu_count",
                     "memory_bytes")

BASELINE_ENTRY_FIELDS = ("value", "unit", "tolerance", "tolerance_provenance",
                         "provenance", "recorded", "story", "why",
                         "host_class", "instrument")


def runner_basename(subject_id: str) -> str:
    """The runner file's basename for a subject id.

    The same rule `tools/bench/src/paths.mjs` applies, and it has to be, because
    a second spelling would let a runner exist that this gate could not see, and
    an unseen runner is exactly the file the anti-fabrication rule is about.
    """
    return subject_id.replace(".", "_")


def backlog_statuses(text: str) -> dict[str, str]:
    """Story status by F-ID, from the STATUS tables only.

    `BACKLOG.md` also carries a "Recorded defects" table whose rows begin with
    an F-ID and carry no status. Reading those as status rows is a guard failing
    on its own document, which is why `scripts/backlog_check.py` restricts the
    same way.
    """
    statuses: dict[str, str] = {}
    in_status_table = False
    for line in text.splitlines():
        if line.startswith("### "):
            heading = line[4:].strip()
            in_status_table = (re.match(r"^M\d+,", heading) is not None
                               or heading.startswith("Roadmap"))
            continue
        if line.startswith("## "):
            in_status_table = False
            continue
        if not in_status_table:
            continue
        match = BACKLOG_ROW.match(line)
        if not match:
            continue
        cells = [c.strip() for c in match.group(2).split("|")]
        statuses[match.group(1)] = cells[-2] if len(cells) >= 2 else ""
    return statuses


def check_registry(registry: dict, fids: set[str],
                   statuses: dict[str, str]) -> tuple[list[str], dict]:
    """Shape, and that every subject_story resolves.

    Returns the problems and the rows keyed by id, so the callers below can go
    on working with what parsed.
    """
    problems: list[str] = []
    rows: dict[str, dict] = {}
    subjects = registry.get("subjects")
    if not isinstance(subjects, list) or not subjects:
        return (["tools/bench/subjects.json has no `subjects` array"], {})

    for subject in subjects:
        sid = subject.get("id")
        if not isinstance(sid, str) or not sid:
            problems.append("a subject has a missing or non-string id")
            continue
        if sid in rows:
            problems.append(f"subject id {sid} appears twice")
            continue
        rows[sid] = subject
        for field in REQUIRED_FIELDS:
            if field not in subject:
                problems.append(
                    f"{sid} is missing the {field} field. Every field is "
                    f"required, because a row with no definition is a row a "
                    f"later story is free to redefine into something easier "
                    f"to measure.")
        for field in ("dimensions", "feeds"):
            if field in subject and not isinstance(subject[field], list):
                problems.append(
                    f"{sid} has a non-list {field}. A figure with one "
                    f"dimension spelled as a bare string reads as a list of "
                    f"characters to anything that iterates it.")
        tiers = subject.get("tiers")
        if not isinstance(tiers, list) or not tiers:
            problems.append(
                f"{sid} declares no tiers. Deviation D-07 makes \"n/a\" a "
                f"legitimate answer and an omitted one not.")
        else:
            for tier in tiers:
                if tier not in VALID_TIERS:
                    problems.append(
                        f"{sid} declares tier {tier!r}, not one of "
                        f"{', '.join(VALID_TIERS)}")
        story = subject.get("subject_story")
        if story is None:
            continue
        if not isinstance(story, str):
            problems.append(f"{sid} has a subject_story that is neither null "
                            f"nor an F-ID")
            continue
        if story not in fids:
            problems.append(
                f"{sid} names {story}, which is not an F-ID in "
                f"docs/sprints/allocation.json. Every subject_story resolves "
                f"against the allocation rather than being copied from a "
                f"comment, because comments in this tree once named F-096 for "
                f"work that is F-101's and were corrected in S03.")
            continue
        status = statuses.get(story)
        if status is None:
            problems.append(f"{sid} names {story}, which has no status row in "
                            f"docs/sprints/BACKLOG.md")
        elif status not in VALID_STATUS:
            problems.append(f"{sid} names {story}, whose BACKLOG.md status is "
                            f"{status!r}")
    return problems, rows


def check_runners(rows: dict, statuses: dict[str, str],
                  runner_names: set[str]) -> list[str]:
    """The anti-fabrication rule, and the point of this gate."""
    problems: list[str] = []
    expected: dict[str, str] = {}
    for sid, subject in rows.items():
        expected[runner_basename(sid)] = sid
        story = subject.get("subject_story")
        has_runner = runner_basename(sid) in runner_names
        if story is None:
            if not has_runner:
                problems.append(
                    f"{sid} has a subject today and no runner at "
                    f"tools/bench/src/runners/{runner_basename(sid)}.mjs. A "
                    f"subject that can be measured and is not is a harness "
                    f"that has stopped measuring it.")
            continue
        status = statuses.get(story, "")
        if status != "done" and has_runner:
            problems.append(
                f"{sid} has a runner at "
                f"tools/bench/src/runners/{runner_basename(sid)}.mjs and its "
                f"subject story {story} is {status!r} rather than done. A "
                f"runner for a subject that does not exist can only be timing "
                f"a stub, and a plausible number for a thing that does not "
                f"exist describes nothing. Report unavailable and name the "
                f"story.")
    for name in sorted(runner_names - set(expected)):
        problems.append(
            f"tools/bench/src/runners/{name}.mjs matches no row in "
            f"subjects.json. A runner the registry does not know about is a "
            f"measurement with no definition.")
    return problems


def check_baseline(baseline: dict, rows: dict,
                   statuses: dict[str, str]) -> list[str]:
    """No recorded number for a subject that does not exist."""
    problems: list[str] = []
    host_classes = baseline.get("host_classes")
    if not isinstance(host_classes, dict):
        return ["ci/bench-baseline.json has no `host_classes` map. It is a map "
                "keyed on host class from the start, because migrating a "
                "scalar to a map later rewrites every recorded entry."]

    for key, block in host_classes.items():
        host_class = block.get("host_class")
        if not isinstance(host_class, dict):
            problems.append(f"host class {key} records no host_class block")
            host_class = {}
        for field in HOST_CLASS_FIELDS:
            if field not in host_class:
                problems.append(
                    f"host class {key} records no {field}. A duration belongs "
                    f"to a machine, and a baseline that cannot say which "
                    f"machine cannot refuse a comparison against another.")
        subjects = block.get("subjects")
        if not isinstance(subjects, dict):
            problems.append(f"host class {key} records no `subjects` map")
            continue
        for sid, entry in subjects.items():
            if sid not in rows:
                problems.append(
                    f"ci/bench-baseline.json records {sid} on host class "
                    f"{key}, and subjects.json has no such row")
                continue
            story = rows[sid].get("subject_story")
            if story is not None and statuses.get(story, "") != "done":
                problems.append(
                    f"ci/bench-baseline.json records a number for {sid} on "
                    f"host class {key}, and its subject story {story} is "
                    f"{statuses.get(story, 'unknown')!r} rather than done. "
                    f"This is the entry an invented number would come to rest "
                    f"in, and it is refused.")
            for field in BASELINE_ENTRY_FIELDS:
                if field not in entry:
                    problems.append(
                        f"the {sid} entry on host class {key} records no "
                        f"{field}")
            # Presence is not enough for these two. A `null` passes a `not in`
            # test and names no machine and no tools, and the comparison then
            # has nothing to establish the figure was taken here. It is
            # reported `incomparable` rather than compared, which is safe, but
            # an entry that can never be compared is a recorded measurement
            # that has quietly stopped being one, and the gate is where that
            # should be caught rather than discovered.
            for field in ("host_class", "instrument"):
                if field in entry and not isinstance(entry[field], dict):
                    problems.append(
                        f"the {sid} entry on host class {key} records "
                        f"{field} as {entry[field]!r} rather than a block. A "
                        f"duration belongs to a machine and to the tools that "
                        f"took it, and an entry naming neither can never be "
                        f"compared against anything.")
            tolerance = entry.get("tolerance")
            if not isinstance(tolerance, (int, float)) or isinstance(
                    tolerance, bool) or not 0 < tolerance <= 1:
                problems.append(
                    f"the {sid} entry on host class {key} has tolerance "
                    f"{tolerance!r}. Spike gate A7.3 names "
                    f"ci/wasm-size-budget.json's mechanism, which is a "
                    f"fraction above 0 and at most 1.")
            elif not entry.get("tolerance_provenance"):
                problems.append(
                    f"the {sid} entry on host class {key} carries a tolerance "
                    f"and nothing saying where it came from. "
                    f"ci/tier-thresholds.json states the provenance of each "
                    f"figure separately, and a tolerance is a derived figure "
                    f"like any other.")
            value = entry.get("value")
            if not isinstance(value, (int, float)) or isinstance(value, bool):
                problems.append(
                    f"the {sid} entry on host class {key} has value {value!r}")
            if entry.get("provenance") != "measured":
                problems.append(
                    f"the {sid} entry on host class {key} has provenance "
                    f"{entry.get('provenance')!r}. Only a figure this harness "
                    f"took belongs here.")
            recorded_host = entry.get("host_class")
            if isinstance(recorded_host, dict) and isinstance(host_class, dict):
                if recorded_host != host_class:
                    problems.append(
                        f"the {sid} entry on host class {key} records a "
                        f"host_class that differs from its block's. The entry "
                        f"would then be permanently incomparable with the "
                        f"machine it is filed under.")
    return problems


def check_pins(bench_pkg: dict, oracle_pkg: dict) -> list[str]:
    """One playwright pin, in two harnesses, and they have to be equal.

    `tools/bench` gets its own install rather than sharing the oracle's, so that
    the benchmark's story is not coupled to the comparator's. The cost of two
    installs is two pins that can drift, and a drifting browser build between
    them would silently make the two harnesses report different environments.
    That is exactly the kind of difference a host class exists to catch, so the
    equality is a mechanism rather than something to remember.
    """
    problems: list[str] = []
    bench_pin = bench_pkg.get("devDependencies", {}).get("playwright")
    oracle_pin = oracle_pkg.get("devDependencies", {}).get("playwright")
    if bench_pin is None:
        problems.append("tools/bench/package.json pins no playwright")
    if oracle_pin is None:
        problems.append("tools/oracle/package.json pins no playwright")
    if bench_pin is None or oracle_pin is None:
        return problems
    for label, pin in (("tools/bench", bench_pin), ("tools/oracle",
                                                    oracle_pin)):
        if not re.fullmatch(r"\d+\.\d+\.\d+", pin):
            problems.append(
                f"{label}/package.json pins playwright as {pin!r}. Exact, with "
                f"no caret and no tilde, for HLD 15.2's reason applied to the "
                f"instrument.")
    if bench_pin != oracle_pin:
        problems.append(
            f"tools/bench pins playwright {bench_pin} and tools/oracle pins "
            f"{oracle_pin}. The two harnesses run their own installs on "
            f"purpose and the pin is held equal on purpose, so that a browser "
            f"build cannot differ between the thing that measures cost and the "
            f"thing that measures correctness.")
    return problems


def check(registry: dict, allocation: dict, backlog_text: str,
          runner_names: set[str], baseline: dict, bench_pkg: dict,
          oracle_pkg: dict) -> list[str]:
    """Every assertion, over inputs the caller supplies."""
    fids = {story["fid"] for story in allocation.get("stories", [])}
    statuses = backlog_statuses(backlog_text)
    problems, rows = check_registry(registry, fids, statuses)
    problems += check_runners(rows, statuses, runner_names)
    problems += check_baseline(baseline, rows, statuses)
    problems += check_pins(bench_pkg, oracle_pkg)
    return problems


def installed_runners() -> set[str]:
    if not RUNNERS.is_dir():
        return set()
    return {path.stem for path in RUNNERS.glob("*.mjs")}


def main() -> int:
    try:
        registry = json.loads(SUBJECTS.read_text(encoding="utf-8"))
        allocation = json.loads(ALLOCATION.read_text(encoding="utf-8"))
        backlog_text = BACKLOG.read_text(encoding="utf-8")
        baseline = (json.loads(BASELINE.read_text(encoding="utf-8"))
                    if BASELINE.exists() else {"host_classes": {}})
        bench_pkg = json.loads(BENCH_PKG.read_text(encoding="utf-8"))
        oracle_pkg = json.loads(ORACLE_PKG.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as exc:
        print("FAIL: the benchmark harness could not be read")
        print(f"  {exc}")
        return 1

    problems = check(registry, allocation, backlog_text, installed_runners(),
                     baseline, bench_pkg, oracle_pkg)
    if problems:
        print("FAIL: the benchmark harness and the delivery record disagree")
        for problem in problems:
            print(f"  {problem}")
        print()
        print("This gate asserts the instrument and never a duration. A "
              "subject whose story has not landed is reported `unavailable` "
              "and names that story. It is never given a runner, and it is "
              "never given a number.")
        return 1

    subjects = registry.get("subjects", [])
    # Counted and LABELLED as what it is. A row naming no blocking story is not
    # the same set as the rows with a subject in this tree: a row whose story
    # has since landed also has one, which is what F-004 did to
    # `tier.startup_microbenchmark`. `node tools/bench/run.mjs --list` is the
    # authority on that split, because it resolves every story against
    # docs/sprints/BACKLOG.md.
    unblocked = [s for s in subjects if s.get("subject_story") is None]
    recorded = sum(len(block.get("subjects", {}))
                   for block in baseline.get("host_classes", {}).values())
    print(f"OK: {len(subjects)} benchmark subject(s), {len(unblocked)} naming "
          f"no blocking story, {recorded} recorded baseline entr(ies) "
          f"across {len(baseline.get('host_classes', {}))} host class(es)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
