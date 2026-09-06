#!/usr/bin/env python3
"""Generate docs/sprints/SPRINT_PLAN.md from docs/sprints/allocation.json.

Written once at bootstrap. After that SPRINT_PLAN.md is hand-curated prose and
this script runs only in `--check` mode, where it asserts the planning data
and nothing else: every planned F-ID appears in exactly one sprint table, and
the sprint and the estimate it appears with match the allocation the backlog is
rendered from. Prose drift is a human's business.

The estimate comparison was added by the S03 review's third pass. That pass
widened F-X014 from one week to two, updated `BACKLOG.md` and
`allocation.json`, and left this file's row at `1w`. **No committed state ever
held the disagreement**, and the fourth pass checked: at `fe18a91` all three
files read `1w` and at `4139a54` all three read `2w`. So this check has never
caught anything. It exists because nothing could have, and the drift it would
have caught was real for the length of one edit.

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

Those two totals agree with HLD section 38 and the Part III preamble. The
tracked backlog and allocation are now the authoritative planning data.

## How sprints were allocated

Stories were initially packed into sprints inside a milestone under two caps,
at most six stories and at most sixteen estimated engineer-weeks. The tracked
allocation never places a story in a sprint at or before the sprint holding
something it depends on.

**Some sprints hold one story and that is not a packing failure.** It is the
head or the tail of a dependency chain. S06 holds only F-016, because every
other story in M2 depends on it. Several Phase 1.5 sprints hold one story
because that story alone is ten to fourteen engineer-weeks.

**Sprint effort is not sprint duration.** The engineer-week estimates were made
for a team. Treat them as relative size, and let the sprint clock measure the
real thing.

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

# HLD section 38. Keyed by the retained planning ID so a re-numbered F-ID cannot
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


ROW = re.compile(r"^\|\s*(F-X?\d{3}[a-z]?)\s*\|(.*)$")
HEAD = re.compile(r"^####\s+Sprint\s+(S\d+)\s*$")

#: Which cell of a sprint table row carries the estimate, counting from the
#: F-ID as cell 0. `render` writes `| fid | eid | story | layer | Nw |`.
EST_CELL = 4


def parse_plan(text: str) -> dict[str, list[tuple[str, str]]]:
    """F-ID -> EVERY (sprint, estimate) it appears with, in document order.

    The sprint comes from the heading and the estimate from the row's fifth
    cell. A row too short to have one yields the empty string, which `check`
    reports rather than skipping.

    **Every occurrence and not the last one.** The docstring at the top of this
    file says every planned F-ID appears in "exactly one sprint table" and this
    function used to let the last occurrence win, so the rule was stated and
    not checked. Measured in the S03 review's fifth pass: copying S72's F-149
    row into S01's table left `--check` at exit 0, because the later row
    overwrote the earlier one and agreed with the allocation. A story planned
    into two sprints is a planning error that reads as a plan.

    **The story text is deliberately not returned.** After the bootstrap
    render this file is hand-curated prose and the wording is a human's
    business. An estimate is not wording, it is the planning number the
    backlog and the allocation both carry, and until the S03 review nothing
    compared the three. See `check` for what that review did and did not
    find, because the first account of it here claimed more than was true.
    """
    found: dict[str, list[tuple[str, str]]] = {}
    current = ""
    for line in text.splitlines():
        head = HEAD.match(line)
        if head:
            current = head.group(1)
            continue
        row = ROW.match(line)
        if row and current:
            cells = [c.strip() for c in row.group(2).split("|")]
            est = cells[EST_CELL - 1] if len(cells) >= EST_CELL else ""
            found.setdefault(row.group(1), []).append((current, est))
    return found


MILESTONE_LINE = re.compile(
    r"^_(S\d+) to (S\d+), (\d+) stories, (\d+) engineer-weeks\._$", re.M)
GOAL_LINE = re.compile(r"^\*\*Goal\*\*: (.+)$")


def rendered_milestones(data: dict) -> list[tuple[str, str, int, int]]:
    """The milestone summary lines `render` would write, as tuples.

    Built by the same walk `render` uses, so a disagreement is the plan
    lagging the allocation rather than two different derivations.
    """
    groups = sprint_groups(data)
    out: list[tuple[str, str, int, int]] = []
    seen = None
    for sprint, stories in groups.items():
        key = stories[0]["milestone"]
        if key == seen:
            continue
        span = [s for s in groups if groups[s][0]["milestone"] == key]
        out.append((span[0], span[-1],
                    sum(len(groups[sp]) for sp in span),
                    sum(st["weeks"] for sp in span for st in groups[sp])))
        seen = key
    return out


def rendered_goals(data: dict) -> dict[str, str]:
    """The `**Goal**:` line `render` would write, per sprint."""
    return {
        sprint: ", ".join(s["story"].replace(";", ",") for s in stories) + "."
        for sprint, stories in sprint_groups(data).items()
    }


def parse_goals(text: str) -> dict[str, list[str]]:
    """Sprint -> EVERY `**Goal**:` line under its heading, in document order.

    The first one only was kept until the S03 review's fifth pass, so a second
    goal paragraph under one sprint heading was invisible: the stale one wins
    and the corrected one below it is never read. `render` writes exactly one
    per sprint, so more than one is a hand edit that `check` reports.
    """
    found: dict[str, list[str]] = {}
    current = ""
    for line in text.splitlines():
        head = HEAD.match(line)
        if head:
            current = head.group(1)
            continue
        goal = GOAL_LINE.match(line)
        if goal and current:
            found.setdefault(current, []).append(goal.group(1).strip())
    return found


def check(data: dict) -> int:
    if not PLAN.exists():
        print(f"FAIL: {PLAN.relative_to(ROOT)} does not exist")
        return 1
    expected = {s["fid"]: (s["sprint"], f"{s['weeks']}w")
                for s in data["stories"] if s["sprint"]}
    actual = parse_plan(PLAN.read_text())

    problems = []

    # "Exactly one sprint table", which this file's own docstring has always
    # claimed and nothing checked. Refused BEFORE the comparisons below,
    # because with two rows for one F-ID there is no single row to compare.
    for fid, rows in sorted(actual.items()):
        if len(rows) > 1:
            problems.append(
                f"{fid} appears in {len(rows)} SPRINT_PLAN.md sprint tables, "
                f"under {', '.join(sprint for sprint, _ in rows)}. Every "
                f"planned F-ID appears in exactly one, and the parser used to "
                f"let the last occurrence win, so a row copied into an earlier "
                f"sprint agreed with the allocation and was invisible.")

    for fid, (sprint, est) in sorted(expected.items()):
        if fid not in actual:
            problems.append(f"{fid} is in BACKLOG.md sprint {sprint} "
                            f"but appears in no SPRINT_PLAN.md sprint table")
            continue
        found_sprint, found_est = actual[fid][0]
        if found_sprint != sprint:
            problems.append(f"{fid} is in sprint {sprint} in BACKLOG.md "
                            f"and in sprint {found_sprint} in "
                            f"SPRINT_PLAN.md")
        if found_est != est:
            problems.append(f"{fid} is estimated {est} in BACKLOG.md and "
                            f"{found_est or 'nothing'} in "
                            f"SPRINT_PLAN.md")
    for fid in sorted(set(actual) - set(expected)):
        problems.append(f"{fid} appears in SPRINT_PLAN.md sprint "
                        f"{actual[fid][0][0]} but carries no sprint in "
                        f"BACKLOG.md")

    # The milestone summary lines and the goal paragraphs. Both are written
    # by `render` from this same allocation, and until the S03 review's
    # fourth pass `check` read neither, which is how M1 carried 61
    # engineer-weeks against an allocation saying 62 and how S04's goal line
    # kept a story title the table row had already replaced. A generated line
    # nothing compares is a hand-maintained line that looks generated.
    plan_text = PLAN.read_text()
    expected_lines = rendered_milestones(data)
    actual_lines = [(a, b, int(n), int(w))
                    for a, b, n, w in MILESTONE_LINE.findall(plan_text)]
    # Matched on the SPAN a line names, not on its position in the file. A
    # positional `zip` reads the Nth expected line against the Nth found line,
    # so one absent summary shifts every line after it and the refusal names
    # the wrong milestone, which is a message a maintainer cannot act on. The
    # spans are the identity a reader has anyway: `_S01 to S05, ..._`.
    found_by_span = {(a, b): (a, b, n, w) for a, b, n, w in actual_lines}
    for wanted in expected_lines:
        span = (wanted[0], wanted[1])
        actual_line = found_by_span.get(span)
        if actual_line is None:
            problems.append(
                f"the allocation puts a milestone at {span[0]} to {span[1]} "
                f"and SPRINT_PLAN.md carries no summary line for it. The line "
                f"is generated, so an absent one is a deleted line rather than "
                f"a milestone nobody wrote up.")
            continue
        if actual_line != wanted:
            problems.append(
                f"the milestone summary line for {wanted[0]} to "
                f"{wanted[1]} should read {wanted[2]} stories and "
                f"{wanted[3]} engineer-weeks, and SPRINT_PLAN.md says "
                f"{actual_line[2]} stories and {actual_line[3]} "
                f"engineer-weeks")
    for span in sorted(set(found_by_span) - {(e[0], e[1])
                                             for e in expected_lines}):
        problems.append(
            f"SPRINT_PLAN.md carries a milestone summary line for {span[0]} "
            f"to {span[1]} and the allocation has no milestone spanning those "
            f"sprints")

    expected_goals = rendered_goals(data)
    actual_goals = parse_goals(plan_text)
    groups = sprint_groups(data)
    for sprint, goal in sorted(expected_goals.items()):
        if sprint not in actual_goals:
            problems.append(f"{sprint} carries no **Goal** line")
            continue
        if len(actual_goals[sprint]) > 1:
            problems.append(
                f"{sprint} carries {len(actual_goals[sprint])} **Goal** "
                f"lines. `render` writes one, and the parser used to keep the "
                f"first, so a stale line above a corrected one won and the "
                f"corrected one was never read.")
            continue
        found_goal = actual_goals[sprint][0]
        if found_goal == goal:
            continue
        # Name the title that drifted rather than reprinting the line. A
        # sprint holds up to six stories and quoting all of them buries the
        # one word that changed, which is how S04's stale title survived
        # being read.
        absent = [s["story"] for s in groups[sprint]
                  if s["story"].replace(";", ",") not in found_goal]
        if absent:
            for story in absent:
                problems.append(
                    f"{sprint}'s **Goal** line does not carry the story "
                    f"title it is built from: {story!r}")
        else:
            problems.append(
                f"{sprint}'s **Goal** line carries every story title and "
                f"still does not match, so its order or its punctuation has "
                f"drifted from what the generator writes")

    if problems:
        print("FAIL: SPRINT_PLAN.md and BACKLOG.md disagree")
        for problem in problems:
            print(f"  - {problem}")
        return 1
    print(f"OK: {len(expected)} planned F-IDs, sprint assignment and "
          f"estimate agree, {len(expected_lines)} milestone summary line(s) "
          f"and {len(expected_goals)} goal line(s) match the allocation")
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
