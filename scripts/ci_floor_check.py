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

## What counts as CI running a gate, and what used to

The first version of this script tested `f"gate {name}"` as a plain substring
over the whole workflow file, and that was fail-open in two directions the S03
sprint review measured:

- **A YAML comment naming the gate satisfied it** with the real step deleted.
  A comment runs nothing, and the floor's claim is that CI RUNS the gate.
- **A LONGER gate name satisfied it.** F-X009 added `guards-deep`, so the
  string `gate guards` occurs inside `gate guards-deep`. The `guards` step
  could be deleted from every pull request and this check stayed green,
  because `guards-deep` runs only on a push to `main` or on a dispatch.
  `guards` is the gate that watches every other gate.

So the match is on a whole gate NAME, taken from an actual `run:` command.
`run_commands` extracts what GitHub Actions executes, both the single-line
`run:` form and the `run: |` block, and drops trailing comments from each. A
gate name that appears in a comment, in a `name:` field or in an `if:`
expression is not an invocation and no longer counts.

## What it still does not check

That the CI step is EQUIVALENT to the gate. `ci.yml` deliberately runs several
gates as their underlying command rather than through the gate runner, so the
match is on the command. A step that ran `cargo clippy` without
`-D warnings` would satisfy this check and be wrong. Making CI call
`bin/ocelli.sh gate --floor` as one step would close that too, and it is not
this script's call to make: it would collapse the job matrix that gives CI its
useful per-area failure names.

The one normalisation is the Python interpreter. `corpus-tests` runs
`python3 scripts/corpus_tests.py` through the gate runner and
`uv run scripts/corpus_tests.py --require-prerequisites` in CI, which is
deliberate and documented at that step: `uv` supplies pydicom, and
`--require-prerequisites` makes a skip red rather than green. The script path
and everything after it must still match, so only the interpreter may differ.

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
# no corpus. `guards-deep` is excluded for a different reason, cost: its probes
# each need cargo, npm or wasm-pack, so it runs on a push to `main` and on
# workflow_dispatch rather than on every pull request. Kept here so this script
# fails if the runner's exclusion list changes without anyone thinking about CI.
NOT_IN_FLOOR = {"oracle", "corpus", "guards-deep"}


def floor_gates(runner: str) -> list[str]:
    declared = re.findall(r'^\s*"([a-z-]+)\|(?:no|YES)\|', runner, re.M)
    if not declared:
        raise RuntimeError("bin/ocelli.sh declares no GATES entries")
    return [g for g in declared if g not in NOT_IN_FLOOR]


def run_commands(workflow: str) -> list[str]:
    """Every line `.github/workflows/ci.yml` actually executes.

    Both YAML forms, `run: <command>` and a `run: |` block, with any trailing
    comment removed. A gate named anywhere else in the file, in a comment, in
    a `name:` field or in an `if:` expression, is not an invocation.
    """
    lines = workflow.splitlines()
    commands: list[str] = []
    index = 0
    while index < len(lines):
        head = re.match(r"^(\s*)(?:-\s+)?run:\s*(.*)$", lines[index])
        index += 1
        if head is None:
            continue
        indent, rest = len(head.group(1)), head.group(2).strip()
        if rest in {"|", ">", "|-", ">-", "|+", ">+"}:
            while index < len(lines):
                body = lines[index]
                if body.strip() and len(body) - len(body.lstrip()) <= indent:
                    break
                commands.append(body)
                index += 1
        else:
            commands.append(rest)
    # A comment is what the `#` opens in both languages here, YAML for the
    # single-line form and the shell for the block form.
    return [re.sub(r"(?<!\S)#.*$", "", command).strip()
            for command in commands]


def invoked_gates(commands: list[str]) -> set[str]:
    """The gate NAMES CI invokes through the runner.

    `bin/ocelli.sh gate a b c` runs three gates, so the names are split rather
    than kept as one string, and a name matches only whole. `guards` and
    `guards-deep` are two gates and neither satisfies the other.
    """
    names: set[str] = set()
    for command in commands:
        for match in re.finditer(
                r"\bbin/ocelli\.sh\s+gate\s+((?:[a-z][a-z-]*\s+)*[a-z][a-z-]*)",
                command):
            names.update(match.group(1).split())
    return names


def runs_command(command: str, commands: list[str]) -> bool:
    """Whether CI runs a gate arm's command, interpreter aside."""
    command = command.strip()
    if not command:
        return False
    python = re.match(r"^python3\s+(.*)$", command)
    if python:
        pattern = re.compile(
            r"(?:python3|uv run)\s+" + re.escape(python.group(1)))
        return any(pattern.search(line) for line in commands)
    return any(command in line for line in commands)


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
    commands = run_commands(workflow)
    invoked = invoked_gates(commands)

    problems = []
    for gate in floor_gates(runner):
        # Either CI invokes the gate by name, or it runs the same command the
        # gate runs. Both are legitimate and ci.yml uses both.
        if gate in invoked:
            continue
        if any(runs_command(command, commands)
               for command in arms.get(gate, [])):
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
