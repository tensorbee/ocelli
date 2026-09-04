#!/usr/bin/env python3
"""Every gate in the CI floor is actually invoked by CI.

`bin/ocelli.sh gate --floor` is the definition of what CI runs, and
`.github/workflows/ci.yml` is what CI actually runs. Nothing made those two
agree, and they are maintained by different edits.

## The defect this exists for, measured

S02 added three gates to the floor: `native` in F-007, `device` in F-008 and
`packages` in F-003. Each needed a matching hand-written step in `ci.yml`, and
each got one. **Nothing would have noticed if one had been missed.** The gate
would have been green locally, absent in CI, and the floor's own claim, that it
is "what CI runs", would have been quietly false.

That is the same failure shape as a skipped gate reading as a passed one, which
this project already refuses everywhere else.

## What it does not check

That the CI step is EQUIVALENT to the gate. `ci.yml` deliberately runs several
gates as their underlying command rather than through the gate runner, so the
match is on the command. A step that ran `cargo clippy` without
`-D warnings` would satisfy this check and be wrong. Making CI call
`bin/ocelli.sh gate --floor` as one step would close that too, and it is not
this script's call to make: it would collapse the job matrix that gives CI its
useful per-area failure names.

Usage: python3 scripts/ci_floor_check.py
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
RUNNER = ROOT / "bin" / "ocelli.sh"
WORKFLOW = ROOT / ".github" / "workflows" / "ci.yml"

# Gates the floor deliberately excludes. `bin/ocelli.sh` excludes these by
# name in its --floor arm, and the reason is deviation D-04: CI has no GPU and
# no corpus. Kept here so this script fails if the runner's exclusion list
# changes without anyone thinking about CI.
NOT_IN_FLOOR = {"oracle", "corpus"}


def floor_gates(runner: str) -> list[str]:
    declared = re.findall(r'^\s*"([a-z-]+)\|(?:no|YES)\|', runner, re.M)
    if not declared:
        raise RuntimeError("bin/ocelli.sh declares no GATES entries")
    return [g for g in declared if g not in NOT_IN_FLOOR]


def gate_commands(runner: str) -> dict[str, list[str]]:
    """The commands each gate's `run_gate` arm actually runs."""
    body = runner[runner.index("run_gate() {"):runner.index("skip() {")]
    arms: dict[str, list[str]] = {}
    for match in re.finditer(
            r"^\s*([a-z-]+)\)\s*(.*?)(?=^\s*(?:[a-z-]+\)|\*\)))",
            body, re.M | re.S):
        arms[match.group(1)] = re.findall(
            r"(?:python3 |npm run |cargo |ci/)[\w./ -]+", match.group(2))
    return arms


def main() -> int:
    runner = RUNNER.read_text()
    workflow = WORKFLOW.read_text()
    arms = gate_commands(runner)

    problems = []
    for gate in floor_gates(runner):
        # Either CI invokes the gate by name, or it runs the same command the
        # gate runs. Both are legitimate and ci.yml uses both.
        if f"gate {gate}" in workflow:
            continue
        commands = arms.get(gate, [])
        if any(command.strip() in workflow for command in commands):
            continue
        problems.append(
            f"the `{gate}` gate is in the CI floor and nothing in "
            f"{WORKFLOW.relative_to(ROOT)} runs it. Either add a step, or "
            f"exclude it from the floor in bin/ocelli.sh and say why. "
            f"`--floor` claims to be what CI runs, and that claim has to be "
            f"true.")

    if problems:
        print("FAIL: CI does not run the whole floor")
        for problem in problems:
            print(f"  {problem}")
        return 1

    count = len(floor_gates(runner))
    print(f"OK: all {count} floor gate(s) are invoked by CI")
    return 0


if __name__ == "__main__":
    sys.exit(main())
