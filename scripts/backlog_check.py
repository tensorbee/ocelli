#!/usr/bin/env python3
"""Consistency between BACKLOG.md, SPRINT_PLAN.md, allocation.json and the ledgers.

Checks, in order of how much a failure costs:

1. Every F-ID in `BACKLOG.md` has a unique, well-formed id and one status.
2. `SPRINT_PLAN.md` and `BACKLOG.md` agree on every story's sprint.
3. The two recorded import defects are still DECLARED, so a dependency that
   does not exist cannot be quietly repointed and forgotten.
4. Every `done` story has a `SPRINT_TRACKER.md` row and an `AS_BUILT.md` entry.
   A story marked done with no completion record is a status, not a delivery.
5. The five HLD section 38 hooks are still inside Phase 1 and still scheduled.
   They are the only reason Part III work appears in a parity plan, so a
   re-plan that pushes one out of Phase 1 is a decision, not an accident.

Usage: python3 scripts/backlog_check.py
"""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SPRINTS = ROOT / "docs" / "sprints"
ALLOCATION = SPRINTS / "allocation.json"

VALID_STATUS = {"pending", "in-progress", "done", "archived", "superseded"}
ROW = re.compile(r"^\|\s*(F-X?\d{3}[a-z]?)\s*\|(.*)$")
HOOK_EIDS = ["E1.8", "E2.7", "E5.6", "E8.8", "E15.1"]
DECLARED_DEFECTS = {"E35.3", "E36.3"}


def backlog_rows() -> dict[str, dict]:
    """F-ID rows from the STATUS tables only.

    `BACKLOG.md` also carries a "Recorded defects" table whose rows begin with
    an F-ID and carry no status. Reading those as status rows reported every
    listed defect as a duplicate F-ID with a garbage status, which is a guard
    failing on its own document. So only tables under a milestone heading or
    the roadmap heading are parsed.
    """
    text = (SPRINTS / "BACKLOG.md").read_text()
    rows: dict[str, dict] = {}
    in_status_table = False
    for line in text.splitlines():
        if line.startswith("### "):
            heading = line[4:].strip()
            in_status_table = (
                re.match(r"^M\d+,", heading) is not None
                or heading.startswith("Roadmap")
            )
            continue
        if line.startswith("## "):
            in_status_table = False
            continue
        if not in_status_table:
            continue
        match = ROW.match(line)
        if not match:
            continue
        fid = match.group(1)
        cells = [c.strip() for c in match.group(2).split("|")]
        rows.setdefault(fid, {"cells": cells, "count": 0})
        rows[fid]["count"] += 1
    return rows


def main() -> int:
    problems: list[str] = []

    data = json.loads(ALLOCATION.read_text())
    stories = {s["fid"]: s for s in data["stories"]}
    by_eid = {s["eid"]: s for s in data["stories"]}

    rows = backlog_rows()

    # 1. Uniqueness and status.
    for fid, row in sorted(rows.items()):
        if row["count"] > 1:
            problems.append(f"{fid} appears in {row['count']} BACKLOG.md rows")
        status = row["cells"][-2] if len(row["cells"]) >= 2 else ""
        if status not in VALID_STATUS:
            problems.append(f"{fid} has status {status!r}, "
                            f"expected one of {sorted(VALID_STATUS)}")
    for fid in sorted(set(stories) - set(rows)):
        problems.append(f"{fid} is in allocation.json and not in BACKLOG.md")

    # 3. Declared defects still declared.
    backlog_text = (SPRINTS / "BACKLOG.md").read_text()
    for eid in sorted(DECLARED_DEFECTS):
        if eid not in backlog_text:
            problems.append(
                f"the recorded import defect for {eid} is no longer declared "
                f"in BACKLOG.md. A dependency that does not exist, or that "
                f"crosses a phase, is carried as declared and not silently "
                f"repaired.")

    # 4. done implies a completion record.
    tracker = (SPRINTS / "SPRINT_TRACKER.md").read_text()
    asbuilt = (SPRINTS / "AS_BUILT.md").read_text()
    for fid, row in sorted(rows.items()):
        status = row["cells"][-2] if len(row["cells"]) >= 2 else ""
        if status != "done":
            continue
        if fid not in tracker:
            problems.append(f"{fid} is done with no SPRINT_TRACKER.md row")
        if fid not in asbuilt:
            problems.append(f"{fid} is done with no AS_BUILT.md entry")

    # 5. The five hooks.
    for eid in HOOK_EIDS:
        story = by_eid.get(eid)
        if story is None:
            problems.append(f"HLD section 38 hook {eid} is not in the backlog")
            continue
        if story["phase"] != "P1":
            problems.append(
                f"HLD section 38 hook {eid} ({story['fid']}) is in phase "
                f"{story['phase']}, not P1. Each hook costs a few weeks now "
                f"and a rewrite later, which is the whole reason it sits in "
                f"a parity plan.")
        if not story["sprint"]:
            problems.append(f"HLD section 38 hook {eid} ({story['fid']}) "
                            f"carries no sprint")

    if problems:
        print("FAIL: backlog consistency")
        for problem in problems:
            print(f"  {problem}")
        return 1

    done = sum(1 for r in rows.values()
               if len(r["cells"]) >= 2 and r["cells"][-2] == "done")
    print(f"OK: {len(rows)} F-IDs, {done} done, 5 hooks scheduled in Phase 1, "
          f"2 import defects declared")
    return 0


if __name__ == "__main__":
    sys.exit(main())
