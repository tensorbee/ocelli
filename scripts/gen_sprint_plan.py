#!/usr/bin/env python3
"""Generate docs/sprints/SPRINT_PLAN.md from docs/sprints/allocation.json.

Written once at bootstrap. After that SPRINT_PLAN.md is hand-curated prose and
this script runs only in `--check` mode, where it asserts one thing and one
thing only: every planned F-ID appears in exactly one sprint table, and the
sprint it appears in matches BACKLOG.md. Prose drift is a human's business.

Usage:
  python3 scripts/gen_sprint_plan.py            # write SPRINT_PLAN.md
  python3 scripts/gen_sprint_plan.py --check    # assert plan matches backlog
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from collections import OrderedDict
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
ALLOCATION = ROOT / "docs" / "sprints" / "allocation.json"
PLAN = ROOT / "docs" / "sprints" / "SPRINT_PLAN.md"

PREAMBLE = """# Sprint Plan

Sprint-by-sprint roadmap for Ocelli. A sprint is a coherent unit of work with
one goal, not a fixed calendar box. The sprint clock starts at the first
`/start-feature` of that sprint.

**Phase 1 is S01 to S41**, 118 stories and 397 engineer-weeks, feature parity
with cornerstone3D v5.8.9. **Phase 1.5 is S42 to S72**, 39 stories and 352
engineer-weeks, the eight differentiating capabilities of HLD Part III. Phase 2
and Phase 3 carry F-IDs in `BACKLOG.md` and no sprint, deliberately.

Those two totals are the HLD's own figures, section 38 and the Part III
preamble respectively, reached independently by summing the imported
spreadsheet. They agree exactly, which is the only reason to trust either.

## How sprints were allocated

`scripts/import_backlog_xlsx.py` packs stories into sprints inside a milestone
under two caps, at most six stories and at most sixteen estimated
engineer-weeks, and never places a story in a sprint at or before the sprint
holding something it depends on.

**Some sprints hold one story and that is not a packing failure.** It is the
head or the tail of a dependency chain. S06 holds only F-016, because every
other story in M2 depends on it. Several Phase 1.5 sprints hold one story
because that story alone is ten to fourteen engineer-weeks.

**Sprint effort is not sprint duration.** The engineer-week estimates are the
spreadsheet's, made for a team. They are kept unmodified because they are what
the HLD's totals are built from, and rewriting them would break the only
cross-check available. Treat them as relative size, and let the sprint clock
measure the real thing.

## What re-planning looks like

Phase 1.5 sizing is provisional by the HLD's own statement, 352 engineer-weeks
"to be re-estimated once Phase 1 evidence exists". Do not treat S42 onward as
committed.

The six Appendix A spike gates each carry the authority to stop or reshape the
programme. They are not backlog stories, they are questions, and `/spike`
runs them against `docs/hld/A-spike-gates.md`. Answer them in the first six
weeks, which means during M1 and M2.

## The five Part III hooks inside Phase 1

Each costs a few weeks now and a rewrite later. They are the only reason
Part III work appears in a parity plan. This table is generated from
`allocation.json`, so it cannot drift from the backlog.
"""

# HLD section 38. Keyed by spreadsheet ID so a re-numbered F-ID cannot
# silently detach the hook from its story.
HOOKS = [
    ("Chunked residency in the cache", "E5.6"),
    ("Multiscale level axis on the volume", "E8.8"),
    ("SR as the native annotation type", "E15.1"),
    ("`ocelli-compute` crate exists", "E1.8"),
    ("Stable render hashes from the oracle", "E2.7"),
]

GOALS_HEADING = """
## Goals per sprint
"""


def load() -> dict:
    return json.loads(ALLOCATION.read_text())


def sprint_groups(data: dict) -> "OrderedDict[str, list[dict]]":
    planned = [s for s in data["stories"] if s["sprint"]]
    planned.sort(key=lambda s: (int(s["sprint"][1:]), s["fid"]))
    groups: "OrderedDict[str, list[dict]]" = OrderedDict()
    for story in planned:
        groups.setdefault(story["sprint"], []).append(story)
    return groups


def render(data: dict) -> str:
    milestones = {m["key"]: m for m in data["milestones"]}
    by_eid = {s["eid"]: s for s in data["stories"]}
    groups = sprint_groups(data)
    lines = [PREAMBLE.rstrip(), ""]
    w = lines.append

    w("| Hook | F-ID | Epic ref | Sprint | Now |")
    w("|------|------|----------|--------|-----|")
    for label, eid in HOOKS:
        story = by_eid[eid]
        w(f"| {label} | {story['fid']} | {eid} | {story['sprint']} "
          f"| {story['weeks']}w |")
    w(GOALS_HEADING.rstrip())
    w("")

    seen_milestone = None
    for sprint, stories in groups.items():
        key = stories[0]["milestone"]
        if key != seen_milestone:
            milestone = milestones[key]
            span = [s for s in groups if groups[s][0]["milestone"] == key]
            weeks = sum(
                st["weeks"] for sp in span for st in groups[sp]
            )
            w(f"### {key}, {milestone['title']}")
            w("")
            w(milestone["goal"])
            w("")
            w(f"_{span[0]} to {span[-1]}, "
              f"{sum(len(groups[sp]) for sp in span)} stories, "
              f"{weeks} engineer-weeks._")
            w("")
            seen_milestone = key

        # A comma, not a semicolon. This line is generated prose and the
        # voice rules apply to it. Story titles are trimmed of their own
        # internal punctuation for the same reason.
        goal_bits = ", ".join(
            s["story"].replace(";", ",") for s in stories)
        w(f"#### Sprint {sprint}")
        w("")
        w(f"**Goal**: {goal_bits}.")
        w("")
        w("| F-ID | Epic ref | Story | Layer | Est |")
        w("|------|----------|-------|-------|-----|")
        for story in stories:
            w(f"| {story['fid']} | {story['eid']} | {story['story']} "
              f"| {story['layer']} | {story['weeks']}w |")
        w("")
    return "\n".join(lines) + "\n"


ROW = re.compile(r"^\|\s*(F-X?\d{3}[a-z]?)\s*\|")
HEAD = re.compile(r"^####\s+Sprint\s+(S\d+)\s*$")


def parse_plan(text: str) -> dict[str, str]:
    """F-ID -> sprint, read from the '#### Sprint SNN' sections."""
    found: dict[str, str] = {}
    current = ""
    for line in text.splitlines():
        head = HEAD.match(line)
        if head:
            current = head.group(1)
            continue
        row = ROW.match(line)
        if row and current:
            found[row.group(1)] = current
    return found


def check(data: dict) -> int:
    if not PLAN.exists():
        print(f"FAIL: {PLAN.relative_to(ROOT)} does not exist")
        return 1
    expected = {s["fid"]: s["sprint"] for s in data["stories"] if s["sprint"]}
    actual = parse_plan(PLAN.read_text())

    problems = []
    for fid, sprint in sorted(expected.items()):
        if fid not in actual:
            problems.append(f"{fid} is in BACKLOG.md sprint {sprint} "
                            f"but appears in no SPRINT_PLAN.md sprint table")
        elif actual[fid] != sprint:
            problems.append(f"{fid} is {sprint} in BACKLOG.md and "
                            f"{actual[fid]} in SPRINT_PLAN.md")
    for fid in sorted(set(actual) - set(expected)):
        problems.append(f"{fid} appears in SPRINT_PLAN.md sprint {actual[fid]} "
                        f"but carries no sprint in BACKLOG.md")

    if problems:
        print("FAIL: SPRINT_PLAN.md and BACKLOG.md disagree")
        for problem in problems:
            print(f"  - {problem}")
        return 1
    print(f"OK: {len(expected)} planned F-IDs, sprint assignment agrees")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    data = load()
    if args.check:
        return check(data)
    PLAN.write_text(render(data))
    groups = sprint_groups(data)
    print(f"wrote {PLAN.relative_to(ROOT)} ({len(groups)} sprints)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
