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

## And the step has to run on the events the floor covers

The review's third pass measured the equivalent bypass that survived that
change: a step is a real `run:` line, it names the gate, and it is behind

    - if: github.event_name == 'workflow_dispatch'
      run: bin/ocelli.sh gate guards

That is the same OUTCOME as the deleted step, reached differently. The gate is
in the file, this check passes, and the gate does not run on a pull request.
It matters most for `guards`, which is the gate that watches every other gate.

So a floor gate has to be invoked on the events the floor covers, and those
are read from the workflow's own `on:` block rather than assumed: every event
it declares except the manual ones, which today is `push` and `pull_request`.
For each such event the governing conditions, the job's `if:` and the step's,
must be PROVABLY true. `_permits` evaluates the subset of the expression
language that turns on `github.event_name`, three-valued, and anything it
cannot prove true, `github.ref` included, does not count. That direction is
deliberate: `if: github.ref == 'refs/heads/main'` on a floor step means the
gate does not run on a pull request, and this check may not read "possibly" as
"yes".

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
from dataclasses import dataclass
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
RUNNER = ROOT / "bin" / "ocelli.sh"
WORKFLOW = ROOT / ".github" / "workflows" / "ci.yml"

# Events a person triggers by hand or a clock triggers on its own. The floor's
# claim is about what CI does to a change, so these are not part of it, and a
# step excluded from one of them is not a step the floor has lost.
MANUAL_EVENTS = {"workflow_dispatch", "repository_dispatch", "schedule"}

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


def workflow_events(workflow: str) -> set[str]:
    """The events `on:` declares.

    Both the block form this workflow uses and the inline list form. A
    workflow that declares none is refused by the caller rather than treated
    as covering everything.
    """
    head = re.search(r"^(?:on|\"on\"|'on'|true):[^\S\n]*(.*)$",
                     workflow, re.M)
    if head is None:
        return set()
    inline = head.group(1).strip()
    if inline:
        return set(re.findall(r"[a-z_]+", inline))
    events: set[str] = set()
    body = workflow[head.end():].splitlines()
    for line in body:
        if not line.strip() or line.lstrip().startswith("#"):
            continue
        indent = len(line) - len(line.lstrip())
        if indent == 0:
            break
        name = re.match(r"^\s{2}([a-z_]+):", line)
        if name:
            events.add(name.group(1))
    return events


# The atoms `_permits` can resolve. `github.event_name` is the one that
# carries the answer. The four status functions are about the steps before
# this one rather than about the event, so a step behind one of them runs on
# every event its job runs on, and they resolve to a constant here.
_STATUS = {"success()": True, "always()": True,
           "failure()": False, "cancelled()": False}

_TOKEN = re.compile(
    r"'[^']*'|\"[^\"]*\"|&&|\|\||==|!=|[\w.$-]+\(\)|[()!]|[\w.$-]+|\S")


class _Unprovable(Exception):
    """The expression is outside the subset this file evaluates."""


@dataclass
class _Cursor:
    tokens: list[str]
    at: int = 0

    def peek(self) -> str | None:
        return self.tokens[self.at] if self.at < len(self.tokens) else None

    def take(self) -> str:
        token = self.peek()
        if token is None:
            raise _Unprovable("expression ends early")
        self.at += 1
        return token


def _atom(cursor: _Cursor, event: str) -> tuple[str, object]:
    """One operand, as ("str", text), ("bool", flag) or ("unknown", None)."""
    token = cursor.take()
    if token[:1] in {"'", '"'}:
        return "str", token[1:-1]
    if token == "github.event_name":
        return "str", event
    if token in _STATUS:
        return "bool", _STATUS[token]
    if token in {"true", "false"}:
        return "bool", token == "true"
    if re.fullmatch(r"[\w.$-]+", token):
        return "unknown", None
    raise _Unprovable(f"unreadable token {token!r}")


def _truth(operand: tuple[str, object]) -> bool | None:
    kind, value = operand
    if kind == "bool":
        return bool(value)
    if kind == "str":
        return value != ""
    return None


def _comparison(cursor: _Cursor, event: str) -> bool | None:
    if cursor.peek() == "!":
        cursor.take()
        inner = _comparison(cursor, event)
        return None if inner is None else not inner
    if cursor.peek() == "(":
        cursor.take()
        value = _disjunction(cursor, event)
        if cursor.take() != ")":
            raise _Unprovable("unbalanced parentheses")
        left: tuple[str, object] = (
            "bool", value) if value is not None else ("unknown", None)
    else:
        left = _atom(cursor, event)
    if cursor.peek() not in {"==", "!="}:
        return _truth(left)
    operator = cursor.take()
    right = _atom(cursor, event)
    if left[0] == "unknown" or right[0] == "unknown" or left[0] != right[0]:
        return None
    same = left[1] == right[1]
    return same if operator == "==" else not same


def _conjunction(cursor: _Cursor, event: str) -> bool | None:
    values = [_comparison(cursor, event)]
    while cursor.peek() == "&&":
        cursor.take()
        values.append(_comparison(cursor, event))
    if False in values:
        return False
    return None if None in values else True


def _disjunction(cursor: _Cursor, event: str) -> bool | None:
    values = [_conjunction(cursor, event)]
    while cursor.peek() == "||":
        cursor.take()
        values.append(_conjunction(cursor, event))
    if True in values:
        return True
    return None if None in values else False


def _permits(condition: str, event: str) -> bool:
    """Is this `if:` condition PROVABLY true on `event`.

    Three-valued underneath, and only a proven true counts. A condition that
    turns on anything but the event name, `github.ref` above all, is unknown
    and does not count. "Possibly reachable" is not the claim `--floor` makes.
    """
    expression = condition.strip()
    wrapper = re.fullmatch(r"\$\{\{(.*)\}\}", expression, re.S)
    if wrapper:
        expression = wrapper.group(1).strip()
    if not expression:
        return True
    cursor = _Cursor(_TOKEN.findall(expression))
    try:
        value = _disjunction(cursor, event)
    except _Unprovable:
        return False
    return value is True and cursor.peek() is None


@dataclass(frozen=True)
class Command:
    """One line CI executes, and the conditions that decide whether it runs."""

    text: str
    conditions: tuple[str, ...]

    def runs_on(self, events: set[str]) -> bool:
        return all(_permits(condition, event)
                   for condition in self.conditions
                   for event in events)


def run_commands(workflow: str) -> list[Command]:
    """Every line `.github/workflows/ci.yml` executes, and when.

    Both YAML forms, `run: <command>` and a `run: |` block, with any trailing
    comment removed. A gate named anywhere else in the file, in a comment, in
    a `name:` field or in an `if:` expression, is not an invocation.

    Each command carries the `if:` conditions above it, the job's and the
    step's. A step's keys sit deeper than the `- ` that opens it and a job's
    sit shallower, which is what tells the two apart.
    """
    lines = workflow.splitlines()
    commands: list[Command] = []
    job_if = ""
    step_if = ""
    step_indent: int | None = None
    in_jobs = False
    index = 0
    while index < len(lines):
        line = lines[index]
        index += 1
        if not line.strip() or line.lstrip().startswith("#"):
            continue
        indent = len(line) - len(line.lstrip())
        if indent == 0:
            in_jobs = line.startswith("jobs:")
            job_if, step_if, step_indent = "", "", None
            continue
        if not in_jobs:
            continue
        if indent == 2 and re.match(r"^\s{2}[\w.-]+:\s*$", line):
            job_if, step_if, step_indent = "", "", None
            continue
        item = re.match(r"^(\s*)-\s", line)
        if item:
            step_if, step_indent = "", len(item.group(1))
        condition = re.match(r"^\s*(?:-\s+)?if:\s*(.*)$", line)
        if condition:
            if step_indent is not None and indent >= step_indent:
                step_if = condition.group(1).strip()
            else:
                job_if = condition.group(1).strip()
            continue
        head = re.match(r"^(\s*)(?:-\s+)?run:\s*(.*)$", line)
        if head is None:
            continue
        conditions = tuple(c for c in (job_if, step_if) if c)
        rest = head.group(2).strip()
        if rest in {"|", ">", "|-", ">-", "|+", ">+"}:
            block = len(head.group(1))
            while index < len(lines):
                body = lines[index]
                if body.strip() and len(body) - len(body.lstrip()) <= block:
                    break
                commands.append(Command(body, conditions))
                index += 1
        else:
            commands.append(Command(rest, conditions))
    # A comment is what the `#` opens in both languages here, YAML for the
    # single-line form and the shell for the block form.
    return [Command(re.sub(r"(?<!\S)#.*$", "", command.text).strip(),
                    command.conditions)
            for command in commands]


def invoked_gates(commands: list[Command]) -> set[str]:
    """The gate NAMES CI invokes through the runner.

    `bin/ocelli.sh gate a b c` runs three gates, so the names are split rather
    than kept as one string, and a name matches only whole. `guards` and
    `guards-deep` are two gates and neither satisfies the other.
    """
    names: set[str] = set()
    for command in commands:
        for match in re.finditer(
                r"\bbin/ocelli\.sh\s+gate\s+((?:[a-z][a-z-]*\s+)*[a-z][a-z-]*)",
                command.text):
            names.update(match.group(1).split())
    return names


def runs_command(command: str, commands: list[Command]) -> bool:
    """Whether CI runs a gate arm's command, interpreter aside."""
    command = command.strip()
    if not command:
        return False
    python = re.match(r"^python3\s+(.*)$", command)
    if python:
        pattern = re.compile(
            r"(?:python3|uv run)\s+" + re.escape(python.group(1)))
        return any(pattern.search(line.text) for line in commands)
    return any(command in line.text for line in commands)


def steps_running(gate: str, arms: dict[str, list[str]],
                  commands: list[Command]) -> list[Command]:
    """Every command that runs this gate, reachable or not.

    Either by naming the gate or by being the command the gate's own arm
    runs. Both are legitimate and ci.yml uses both.
    """
    return [command for command in commands
            if gate in invoked_gates([command])
            or any(runs_command(arm, [command])
                   for arm in arms.get(gate, []))]


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
    events = workflow_events(workflow) - MANUAL_EVENTS

    problems = []
    if not events:
        problems.append(
            f"{WORKFLOW.relative_to(ROOT)} declares no automatic event, so "
            f"there is no event on which the floor could be said to run. "
            f"`--floor` claims to be what CI runs, and a workflow only a "
            f"person can start does not run on a pull request.")

    for gate in floor_gates(runner):
        running = steps_running(gate, arms, commands)
        # Per event, not per step. Two steps with complementary conditions
        # cover the floor between them, and asking one step to cover every
        # event refuses that arrangement while naming no missing event, which
        # is a message that cannot be acted on.
        missing = sorted(
            event for event in events
            if not any(command.runs_on({event}) for command in running))
        if events and running and not missing:
            continue
        if running and events:
            gating = " and ".join(sorted(
                {condition for command in running
                 for condition in command.conditions}))
            problems.append(
                f"the `{gate}` gate is in the CI floor and every step in "
                f"{WORKFLOW.relative_to(ROOT)} that runs it is behind a "
                f"condition that does not run it on "
                f"{', '.join(missing)}: `{gating}`. A step that never "
                f"executes on a pull request is the same outcome as no step "
                f"at all. Either drop the condition, or exclude the gate "
                f"from the floor in bin/ocelli.sh and say why. `--floor` "
                f"claims to be what CI runs, and that claim has to be true.")
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
    print(f"OK: all {count} floor gate(s) are invoked by CI on "
          f"{', '.join(sorted(events))}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
