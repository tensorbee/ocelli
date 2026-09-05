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

## A gate is several commands, and CI has to run all of them

The S03 review's fourth pass measured the next bypass. A gate whose
`bin/ocelli.sh` arm chains several commands was treated as invoked when CI ran
any ONE of them. `backlog` is `backlog_check.py && gen_sprint_plan.py --check`,
and deleting either step from `ci.yml` left this check green. So the estimate
comparison added in the same pass could be removed from every pull request by
deleting one line.

Two fixes were available and this file takes the first: **every command in a
gate's arm must be run by CI, or a step must invoke `bin/ocelli.sh gate
<name>`, which runs all of them by definition.** The second option, demanding
the gate runner for every multi-command gate, was rejected for the reason the
next section already gives: `ci.yml` deliberately runs most gates as their
underlying command so that a failure names the area rather than naming
`bin/ocelli.sh`, and forcing the runner would collapse that. Requiring every
command keeps both forms legal and still refuses a half-run gate, and the
refusal names the command that is missing rather than the gate.

A gate whose arm yields no extractable command, `native` and `panic` and
`oracle`, can only be satisfied by a step naming the gate. `all()` over an
empty list is true, and a vacuous pass here would be the widest hole in the
file.

## A gate outside the floor can still be a gate CI is supposed to run

`NOT_IN_FLOOR` takes a gate out of `floor_gates`, and until the fourth pass
nothing else asserted anything about it. The reviewer deleted the whole
`guards-deep` job from `.github/workflows/ci.yml` and this check, the census
and the floor probes all exited 0. `guards-deep` is what runs the cargo probes
and `census --profile deep`, which is the sweep-complete rule.

The three excluded gates are not excluded for the same reason, and the runner
says which is which without a second literal here. `oracle` is the one gate
`bin/ocelli.sh` marks `YES` in its GPU column, and deviation D-04 is that CI
has no GPU, so nothing in CI may run it. The other two are excluded for cost or
for the corpus, and CI does run part of each: `corpus_check.py --coverage` for
`corpus`, and the whole `guards-deep` job for `guards-deep`.

So the rule is: **a gate outside the floor that does not need a GPU must still
be run by some CI step, provably reachable on at least one event the workflow
declares.** State the claim exactly and no more. `_permits` cannot evaluate
`github.ref`, and `guards-deep`'s job condition is

    github.event_name == 'workflow_dispatch'
      || (github.event_name == 'push' && github.ref == 'refs/heads/main')

which is provably true on `workflow_dispatch`, provably FALSE on
`pull_request`, and unprovable on `push` because the branch decides. The claim
this check makes is therefore "CI runs `guards-deep` on `workflow_dispatch`",
and the OK line prints the event so the claim is read rather than assumed. A
push to `main` reaching it is the workflow's own comment and is not something
this file can prove. Deleting the job, or putting it behind a condition that is
false on every declared event, is refused.

## The two exclusion lists, joined

`bin/ocelli.sh`'s `--floor` arm excludes gates by name in a `case` statement
and `NOT_IN_FLOOR` above repeats the list. A comment in the runner claimed that
a name added to one and not the other "makes the `ci` gate demand a CI step for
a gate the floor never runs", and the reviewer measured that the net caught
nothing: `prose` added to the shell list alone left this check at 0 while
`gate --floor` silently stopped running `prose`. That direction has no
detection at all, because a gate leaving the floor removes work rather than
adding a demand.

So the runner's list is PARSED here and compared with `NOT_IN_FLOOR` for set
equality, in both directions. `NOT_IN_FLOOR` is also in the declared-constant
ratchet, so the pair is now watched by a mechanism rather than by a comment.

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


def declared_gates(runner: str) -> list[str]:
    declared = re.findall(r'^\s*"([a-z-]+)\|(?:no|YES)\|', runner, re.M)
    if not declared:
        raise RuntimeError("bin/ocelli.sh declares no GATES entries")
    return declared


def floor_gates(runner: str) -> list[str]:
    return [g for g in declared_gates(runner) if g not in NOT_IN_FLOOR]


def gpu_gates(runner: str) -> set[str]:
    """The gates `bin/ocelli.sh` marks YES in its GPU column.

    Read from the runner rather than listed again here. Deviation D-04 is that
    CI has no GPU, so this is what says which excluded gate CI is not supposed
    to run at all, and it is the runner's own declaration rather than a second
    copy of it.
    """
    return set(re.findall(r'^\s*"([a-z-]+)\|YES\|', runner, re.M))


# `case "$name" in oracle|corpus|guards-deep) continue ;; esac`, inside the
# `--floor` arm. Anchored on `"$name"` and on `continue`, which together occur
# once in the file, so this cannot drift onto some other case statement.
RUNNER_EXCLUSION = re.compile(
    r'case\s+"\$name"\s+in\s+([a-z|-]+)\)\s*continue\s*;;\s*esac')


def runner_excluded(runner: str) -> set[str]:
    """The gates `bin/ocelli.sh --floor` skips by name.

    Refuses rather than returning an empty set. A parse that finds nothing and
    reports agreement would say "the two lists agree" about a list it could not
    read, which is the shape this repository refuses everywhere else.
    """
    match = RUNNER_EXCLUSION.search(runner)
    if match is None:
        raise RuntimeError(
            "bin/ocelli.sh's --floor arm carries no "
            "`case \"$name\" in ...) continue ;; esac` exclusion list where "
            "this parser looks for it. Either the floor no longer excludes "
            "anything, in which case NOT_IN_FLOOR is wrong, or the arm was "
            "rewritten and this parser has to be rewritten with it. Both need "
            "a person, and neither may be read as agreement.")
    return {name for name in match.group(1).split("|") if name}


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
    """Every command that runs any PART of this gate, reachable or not.

    Either by naming the gate or by being one of the commands the gate's own
    arm runs. Both are legitimate and ci.yml uses both. This is deliberately
    the partial question. `covers` below asks the whole one.
    """
    return [command for command in commands
            if gate in invoked_gates([command])
            or any(runs_command(arm, [command])
                   for arm in arms.get(gate, []))]


def missing_arm_commands(gate: str, arms: dict[str, list[str]],
                         commands: list[Command]) -> list[str]:
    """The commands in this gate's arm that none of these steps runs."""
    return [arm.strip() for arm in arms.get(gate, [])
            if not runs_command(arm, commands)]


def covers(gate: str, arms: dict[str, list[str]],
           commands: list[Command]) -> bool:
    """Do these steps run the WHOLE gate.

    A step naming the gate runs every command in its arm by definition. Short
    of that, every command in the arm has to be run by some step: a gate whose
    arm is `a && b` is not invoked by a CI step that runs only `a`, and
    treating it as invoked is how the `backlog` gate's estimate check could be
    deleted from every pull request in one line.

    `bool(arm)` and not a bare `all()`. `native`, `panic` and `oracle` yield no
    extractable command, and `all([])` is true, so without this a gate whose
    arm this file cannot read would pass over an empty set and say so in the
    language of success.
    """
    if gate in invoked_gates(commands):
        return True
    arm = arms.get(gate, [])
    return bool(arm) and not missing_arm_commands(gate, arms, commands)


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

    # The two exclusion lists, joined. A comment used to say they had to
    # agree and nothing read either of them.
    excluded = runner_excluded(runner)
    if excluded != NOT_IN_FLOOR:
        only_runner = sorted(excluded - NOT_IN_FLOOR)
        only_here = sorted(NOT_IN_FLOOR - excluded)
        problems.append(
            f"bin/ocelli.sh's --floor arm and this file's NOT_IN_FLOOR "
            f"disagree about which gates the floor excludes. Only in the "
            f"runner: {', '.join(only_runner) or 'none'}. Only in "
            f"NOT_IN_FLOOR: {', '.join(only_here) or 'none'}. A name in the "
            f"runner alone silently removes the gate from `gate --floor` "
            f"while this check goes on demanding a CI step for it and "
            f"finding one, so the loss has no detection anywhere. A name here "
            f"alone demands a CI step for a gate the floor still runs. The "
            f"two lists are one decision written twice and they have to be "
            f"kept equal.")

    for gate in floor_gates(runner):
        running = steps_running(gate, arms, commands)
        # Per event, not per step. Two steps with complementary conditions
        # cover the floor between them, and asking one step to cover every
        # event refuses that arrangement while naming no missing event, which
        # is a message that cannot be acted on.
        #
        # Per event AND over the whole arm. `blocked` is the events no step
        # touching the gate reaches at all, `partial` is the events a step
        # reaches while leaving a command in the arm unrun.
        blocked: list[str] = []
        partial: dict[str, list[str]] = {}
        for event in sorted(events):
            reachable = [c for c in commands if c.runs_on({event})]
            if covers(gate, arms, reachable):
                continue
            if not any(c in running for c in reachable):
                blocked.append(event)
            else:
                partial[event] = missing_arm_commands(gate, arms, reachable)
        if events and running and not blocked and not partial:
            continue
        if running and events and (blocked or partial):
            if partial:
                absent = sorted({command for gaps in partial.values()
                                 for command in gaps})
                problems.append(
                    f"the `{gate}` gate is in the CI floor and "
                    f"{WORKFLOW.relative_to(ROOT)} runs only part of it on "
                    f"{', '.join(sorted(partial))}. Its arm in bin/ocelli.sh "
                    f"chains several commands and no step runs "
                    f"{', '.join(repr(c) for c in absent)}. A gate is every "
                    f"command in its arm, so a CI step that runs one of them "
                    f"leaves the rest deletable from every pull request in "
                    f"one line. Either add a step for each command, or have "
                    f"one step run `bin/ocelli.sh gate {gate}`, which runs "
                    f"the arm entire.")
            if blocked:
                gating = " and ".join(sorted(
                    {condition for command in running
                     for condition in command.conditions}))
                problems.append(
                    f"the `{gate}` gate is in the CI floor and every step in "
                    f"{WORKFLOW.relative_to(ROOT)} that runs it is behind a "
                    f"condition that does not run it on "
                    f"{', '.join(blocked)}: `{gating}`. A step that never "
                    f"executes on a pull request is the same outcome as no "
                    f"step at all. Either drop the condition, or exclude the "
                    f"gate from the floor in bin/ocelli.sh and say why. "
                    f"`--floor` claims to be what CI runs, and that claim has "
                    f"to be true.")
            continue
        problems.append(
            f"the `{gate}` gate is in the CI floor and nothing in "
            f"{WORKFLOW.relative_to(ROOT)} runs it. Either add a step, or "
            f"exclude it from the floor in bin/ocelli.sh and say why. "
            f"`--floor` claims to be what CI runs, and that claim has to be "
            f"true.")

    # The gates outside the floor. `oracle` needs a GPU and D-04 says CI has
    # none, so it is the one gate CI may not run. Every other excluded gate is
    # excluded for cost or for the corpus, and CI is still supposed to run it
    # somewhere. Nothing asserted that until the S03 review's fourth pass
    # deleted the whole `guards-deep` job and watched this exit 0.
    #
    # Every declared event, MANUAL ONES INCLUDED, because a manual dispatch is
    # exactly what `guards-deep` is on. The floor's own claim is about a change
    # and stays narrower.
    every_event = workflow_events(workflow)
    gpu = gpu_gates(runner)
    reached_outside: dict[str, list[str]] = {}
    for gate in sorted(NOT_IN_FLOOR & set(declared_gates(runner))):
        if gate in gpu:
            continue
        running = steps_running(gate, arms, commands)
        reachable = sorted(
            event for event in every_event
            if any(command.runs_on({event}) for command in running))
        if reachable:
            reached_outside[gate] = reachable
            continue
        problems.append(
            f"the `{gate}` gate is excluded from the floor and needs no GPU, "
            f"so CI is still supposed to run it, and no step in "
            f"{WORKFLOW.relative_to(ROOT)} runs it on any event the workflow "
            f"declares. Being out of the floor means it does not run on every "
            f"pull request. It does not mean it runs nowhere, and a gate that "
            f"runs nowhere is a gate whose refusals nobody has watched. Add a "
            f"step, or mark the gate as needing a GPU in bin/ocelli.sh's GATES "
            f"table and say why in a deviation.")

    if problems:
        print("FAIL: CI does not run the whole floor")
        for problem in problems:
            print(f"  {problem}")
        return 1

    count = len(floor_gates(runner))
    print(f"OK: all {count} floor gate(s) are invoked by CI on "
          f"{', '.join(sorted(events))}, every command in each gate's arm")
    print(f"  the runner's --floor exclusion list and NOT_IN_FLOOR agree on "
          f"{', '.join(sorted(NOT_IN_FLOOR))}")
    for gate, reachable in sorted(reached_outside.items()):
        print(f"  outside the floor, CI runs `{gate}` on "
              f"{', '.join(reachable)}. That is what is PROVABLE from the "
              f"conditions. A condition naming github.ref is not evaluated "
              f"here, so an event missing from this list may still reach the "
              f"step on some branch and this check does not claim it does.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
