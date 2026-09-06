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

## Where an arm ENDS, and the worst thing the fifth pass found

That sentence was false for `panic` and the way it was false is the reason
this section exists. `gate_commands` ended an arm at the next case LABEL rather
than at its `;;`, so every arm swallowed the comment block introducing the
following arm and the command regex read commands out of prose. `arms['panic']`
came out as `['npm run test']`, a command that appears nowhere in the `panic`
arm, taken from the `bench` comment block's sentence about
`npm run test:browser`.

Measured: delete the `bin/ocelli.sh gate panic` step from `ci.yml` and this
check refused at exit 1. Then also add a legitimate `- run: npm run test` step
to the frontend job and it returned to exit 0 with "all 25 floor gate(s) are
invoked by CI". `panic` is the wasm panic-hook proof of HLD section 23, the one
property no native test can observe, and it could be deleted from CI by a
change that reads as adding a test.

So an arm now ends at its `;;`, `#` comments are stripped from the body before
extraction, and a backslash line continuation is joined first.

**And "ends at its `;;`" was itself false three more ways, all measured in the
S03 review's ninth pass**, each on the one-line `fmt` arm with a real command
CI does not run appended after the shape, each leaving this check at exit 0
with one extracted command and `unseen` `None`, and each accepted by `sh -n`:
a `;;` inside a quoted string, a `while case ... esac`, and an `if case ...
esac`. The first is why the arm's end is found by a scan that steps over quoted
spans rather than by a regex, and why the comment strip and the statement split
use that same scan. The other two are why `NESTED_CASE`'s alternation is DERIVED
from `SHELL_INTRODUCERS` instead of being written out beside it: this file knew
`if`, `while` and `until` introduce a command in one function and not in the
other, and two lists that must agree are one list.

The continuation join closed a loss of its
own: the extraction pattern stops at the backslash, so `-p <suite>` fell
off the end of `errors`, `bench` and `guards`, and `runs_command` uses
`search`, which means any unittest step satisfied any of them. Replacing the
`guards` step with its five arm commands but with
`-p test_nothing_at_all.py`, which discovers zero tests, left this check at
exit 0.

**The extractor's limit was a live hole and the OK line asserted the opposite.**
It recognises four command prefixes, `python3 `, `npm run `, `cargo ` and
`ci/`. `node`, `wasm-pack` and `"$0"` are invisible to it, so the three
`node --test` suites in `bench`, the wasm-pack build in `panic` and the
`"$0" wasm` and `"$0" native` self-calls were not demanded of CI by a check
printing "every command in each gate's arm". The declaration that stood here
said nothing was lost, because "bench, wasm, panic and native are each invoked
by NAME in ci.yml", and **nothing enforced that sentence**. Measured in the S03
review's sixth pass: replacing the `- run: bin/ocelli.sh gate bench` step with
its two extractable commands left this check at exit 0 with that same OK line,
and five node test files left CI in a two-line edit that reads as expanding the
step. It is the same shape as the `arms['panic']` bug the fifth pass fixed, one
level out.

So the sentence is a rule now. `unseen_commands` reads each arm's statements
and reports the ones the extractor cannot see, and **a gate whose arm holds one
of those is refused unless a step invokes the gate by name**, per event, exactly
as the per-command rule is. That is strictly stronger than the declaration it
replaces, and it leaves the `all([])` argument standing: `panic`, `native` and
`oracle` still yield no extractable command, `covers` still refuses an empty arm
through `bool(arm)`, and widening `COMMAND_PREFIXES` is still not what happened
here. Only the sentence that was taken on trust is now mechanical.

A statement whose first word is a shell builtin is not a command for this
purpose. `[ -d node_modules ]`, `command -v wasm-pack`, `skip`, `echo` and
`return` decide whether the real work runs and are not the work, so counting
them would demand a gate-name step for `lint`, `types` and `packages`, which
run their whole arm as steps today and are not weaker for it.

**A control KEYWORD is different, and treating it as the same thing was a
fail-open.** `STATEMENT_BREAK` splits on newline, semicolon, brace, bracket and
the two boolean operators, so `if node --test x; then true; fi` is ONE statement
whose head is `if`, and `if` sat in the builtin list. The S03 review's seventh
pass measured the outcome:
wrapping the `bench` arm's `node --test` line that way and replacing the
`gate bench` step with its two `python3` commands gave `unseen bench: None` and
exit 0, five node suites out of CI, which is byte for byte the outcome the
sixth pass measured and made a rule. So the head is dropped and the REST OF THE
STATEMENT IS RE-SCANNED. `SHELL_INTRODUCERS` holds the keywords that introduce
a statement and `SHELL_NOISE` holds the heads whose remainder is arguments, and
the split matters: re-scanning past `[` would report `-d node_modules ]` as an
unseen command and refuse `lint`, `types` and `packages`.

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
match is on the command. Making CI call `bin/ocelli.sh gate --floor` as one step
would close that, and it is not this script's call to make: it would collapse
the job matrix that gives CI its useful per-area failure names.

**The example this paragraph used to give was false**, and it is worth naming
because it was cited as the reason for a design decision. It said "a step that
ran `cargo clippy` without `-D warnings` would satisfy this check and be
wrong". MEASURED in the S03 review's seventh pass: dropping `-- -D warnings`
from the CI step gives exit 1, "the `clippy` gate is in the CI floor and
nothing in ci.yml runs it", because the arm command is the whole string
including the flags. The real hole in that class was the direction the
comparison was made in, and it is closed below.

`runs_command` compares the argument VECTORS for equality, and it used to test
for a substring. MEASURED the same pass: changing a CI step to
`python3 scripts/prose_check.py --only-this-one-file README.md` left this check
at exit 0 with `prose` reported as invoked, while CI checked one file.
`ci-floor.narrowed-arm-command` probes the opposite direction, an argument the
arm already carried being narrowed, and could not see an argument being added.

The one normalisation is the Python interpreter, and the one permitted addition
is `PERMITTED_ADDITIONS`. `corpus-tests` runs `python3 scripts/corpus_tests.py`
through the gate runner and `uv run scripts/corpus_tests.py
--require-prerequisites` in CI, which is deliberate and documented at that step:
`uv` supplies pydicom, and `--require-prerequisites` makes a skip red rather
than green, which is STRICTLY STRONGER than the arm. That exception is declared
with its reason in one place rather than granted by a loose comparison
everywhere.

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
# no corpus.
#
# `guards-deep` is excluded for a different reason, and the reason has been
# written three ways and was false twice. It said "cargo, npm or wasm-pack" and
# "minutes rather than seconds" until the eighth pass. It said the cost was a
# toolchain a runner has to install until the S03 review's NINTH pass, and the
# `guards` job installs the pinned toolchain for `gate guards` anyway, so the
# runner that would run the deep probes already had one. That job runs
# `bin/ocelli.sh gate guards-deep` on every event now, so the TRIGGER is no
# longer a difference between the two.
#
# What is left is duplication and nothing else: `--profile deep` is a strict
# superset of `--profile floor`, so a `gate --floor` including this gate would
# run every floor probe twice. MEASURED on the development machine, deep 23.8s
# against floor 15.9s. `python3 scripts/guard_probe.py --list` prints each
# probe's profile and what it needs, and reading that beats reading this.
# Kept here so this script fails if the runner's exclusion list changes without
# anyone thinking about CI.
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


# The interpreter, normalised away before two argv are compared. `uv run` and
# `python3` are the same run of the same script and only one of them supplies
# pydicom, which is why the `corpus-tests` step uses it.
INTERPRETER = re.compile(r"^(?:python3|uv\s+run)\s+")

# The ONE place a CI step may run more than a gate arm's command, with the
# reason it may. Everything else added to a step changes what the gate means,
# and the S03 review's seventh pass measured the cost of a substring match:
# changing a CI step to `python3 scripts/prose_check.py --only-this-one-file
# README.md` left this check at exit 0 with `prose` reported as invoked, while
# CI checked one file. `ci-floor.narrowed-arm-command` probes the same class
# from the other side and could not see this one, because it narrowed an
# argument the arm already carried rather than adding one.
#
# The addition here is strictly STRONGER than the arm: the gate runner reports
# a skipped prerequisite as a pass and `--require-prerequisites` makes it red,
# which is why the CI step is written that way and why the workflow says so at
# the step.
PERMITTED_ADDITIONS: dict[str, tuple[frozenset[str], str]] = {
    "scripts/corpus_tests.py": (
        frozenset({"--require-prerequisites"}),
        "it makes an absent prerequisite RED rather than a green skip, which "
        "is strictly stronger than the gate arm's run"),
}


def _argv(text: str) -> list[str]:
    """One command as its argument vector, with the interpreter normalised."""
    return INTERPRETER.sub("python3 ", text.strip()).split()


def _permitted_extension(wanted: list[str], got: list[str]) -> bool:
    """Is `got` `wanted` plus only additions declared for the script it runs."""
    if len(got) <= len(wanted) or got[:len(wanted)] != wanted:
        return False
    for name, (additions, _) in PERMITTED_ADDITIONS.items():
        if name in wanted:
            return all(token in additions for token in got[len(wanted):])
    return False


def runs_command(command: str, commands: list[Command]) -> bool:
    """Whether CI runs a gate arm's command, interpreter aside.

    EQUALITY over the argument vector, not a substring. A substring match said
    yes to a step running the arm's command plus arguments that narrowed it,
    and this file's own docstring named the equivalence it does not check while
    leaving this the way in. The one declared exception is
    `PERMITTED_ADDITIONS`, which is a superset the workflow documents at its
    step and which is strictly stronger than the arm.
    """
    wanted = _argv(command)
    if not wanted:
        return False
    for line in commands:
        got = _argv(line.text)
        if got == wanted or _permitted_extension(wanted, got):
            return True
    return False


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


# The four command prefixes this file can see. Declared as a constant so the
# limit is one thing to read rather than a regex to re-derive. `node`,
# `wasm-pack` and `"$0"` are deliberately absent, for the reason the docstring
# gives at length: adding them would make `panic` a gate with an extractable
# command and would break the claim that a gate whose arm yields none can only
# be satisfied by a step naming it.
COMMAND_PREFIXES = ("python3 ", "npm run ", "cargo ", "ci/")

# A line continuation. Joined BEFORE anything else, because the extraction
# class stops at the backslash and `-p <suite>` then falls off the end of every
# multi-line arm.
CONTINUATION = re.compile(r"\\\n\s*")

# A `run_gate` case LABEL at the start of a line. The arm's END is found by
# `_arm_end` below and not by this pattern, which is the S03 review's ninth
# pass. `ARM` was `^[ \t]*([a-z-]+)\)(.*?);;`, and a `;;` inside a QUOTED STRING
# is not the end of an arm. MEASURED on the one-line `fmt` arm with a real
# command CI does not run appended after it: `fmt) cargo fmt --all --check &&
# echo "a ;; b" && python3 scripts/prose_check.py --extra ;;` left
# `scripts/ci_floor_check.py` at exit 0, `arms['fmt']` holding one command and
# `unseen['fmt']` `None`, with `sh -n` accepting the file. That is the eighth
# pass's comment-holding-a-terminator defect one lexer rule along: the comment
# route was closed by stripping comments first, and a string cannot be stripped.
ARM_LABEL = re.compile(r"^[ \t]*([a-z-]+)\)", re.M)


def _quote_spans(text: str, index: int) -> tuple[int, str]:
    """Step over one character of shell text, tracking the quote it opens.

    Returns the next index and the quote that is now open, "" for none. One
    reader for two callers, `_strip_shell_comments` and `_arm_end`, because
    they are the same lexer rule and written twice they would drift, which is
    exactly what `NESTED_CASE` and `SHELL_INTRODUCERS` did.

    Single quotes take no escape, double quotes take a backslash escape, and a
    backslash outside quotes escapes the next character. That is POSIX shell
    quoting and it is all this file needs: the `run_gate` arms hold no
    here-document and no `$'...'`.
    """
    char = text[index]
    if char == "\\":
        return index + 2, ""
    if char in "\"'":
        return index + 1, char
    return index + 1, ""


def _strip_shell_comments(text: str) -> str:
    """`text` with every `#` comment removed, QUOTES RESPECTED.

    `SHELL_COMMENT` was `(?<!\\S)#.*$`, which cannot tell a comment from a `#`
    inside a string. It ran over the whole `run_gate` region, so a legitimate
    `echo "issue #12"` in an arm would have been truncated mid-string and the
    quote left open. Nothing in the region carries one today, and the whole
    point of the ninth pass's arm work is that the parser must not depend on
    that staying true: the scan below is the same one `_arm_end` uses, so a `#`
    inside a string is text and a `;;` inside a string is text, by one rule
    rather than by two that agree today.
    """
    kept: list[str] = []
    quote = ""
    index = 0
    while index < len(text):
        char = text[index]
        if quote:
            if quote == '"' and char == "\\":
                kept.append(text[index:index + 2])
                index += 2
                continue
            if char == quote:
                quote = ""
            kept.append(char)
            index += 1
            continue
        if char == "#" and (not kept or kept[-1][-1:] in ("", " ", "\t", "\n")):
            end = text.find("\n", index)
            index = len(text) if end == -1 else end
            continue
        step, opened = _quote_spans(text, index)
        kept.append(text[index:step])
        quote = opened
        index = step
    return "".join(kept)


def _arm_end(text: str, start: int) -> tuple[int, str]:
    """Where the arm beginning at `start` ends, or why this parser cannot say.

    The first `;;` OUTSIDE a quoted string. Returns its index and an empty
    reason, or `-1` and the reason to refuse. Two reasons, both fail-closed: a
    quote that is never closed, which is not a shell script this parser should
    guess at, and an arm with no terminator at all.
    """
    quote = ""
    index = start
    while index < len(text):
        char = text[index]
        if quote:
            if quote == '"' and char == "\\":
                index += 2
                continue
            if char == quote:
                quote = ""
            index += 1
            continue
        if text.startswith(";;", index):
            return index, ""
        step, opened = _quote_spans(text, index)
        quote = opened
        index = step
    if quote:
        return -1, (f"a {quote} quote is opened and never closed, so this "
                    f"parser cannot tell which `;;` ends the arm")
    return -1, "the arm reaches the end of the region with no `;;` terminator"


# Keywords that INTRODUCE a statement rather than being one. What follows one
# of these is the work, so the head is dropped and the REST OF THE STATEMENT IS
# RE-SCANNED rather than discarded with it.
#
# Discarding it was a fail-open, and the S03 review's seventh pass measured the
# outcome exactly. `STATEMENT_BREAK` splits on `[\n;{}()]|&&|\|\||\|`, so
# `if node --test ...; then true; fi` is one statement whose head is `if`,
# and `if` sat in the noise list below. Wrapping the `bench` arm's `node --test`
# line that way and replacing the `gate bench` step with its two `python3`
# commands gave `unseen bench: None` and exit 0, which is five node suites out
# of CI. That is byte for byte the outcome the sixth pass measured and made a
# rule.
#
# `eval` is here and not below, because `eval python3 x` runs the command.
#
# **`NESTED_CASE` is DERIVED from this set since the S03 review's ninth pass**,
# and it is declared here rather than below so the derivation can read it. The
# two lists were written out separately and had already drifted: this one held
# ten names and the alternation held five, so `while case` and `if case` were
# statement positions this file knew about in one function and not in the
# other. MEASURED on the one-line `fmt` arm, each shape appending a real command
# CI does not run and each leaving `scripts/ci_floor_check.py` at exit 0 with
# `arms['fmt']` holding one command and `unseen['fmt']` `None`, with `sh -n`
# accepting the file:
#
#     ... && while case x in *) false ;; esac; do :; done && python3 ... ;;
#     ... && if case x in *) true ;; esac; then :; fi && python3 ... ;;
#
# Two lists that must agree are one list, which is the same repair
# `bin/ocelli.sh`'s `--floor` arm and `NOT_IN_FLOOR` got in the fourth pass.
SHELL_INTRODUCERS = frozenset({
    "if", "then", "elif", "else", "while", "until", "do", "!", "time", "eval",
})

# A `case` keyword at a statement position inside an arm body.
#
# A KEYWORD is a statement position too, and the S03 review's seventh pass
# measured what leaving it out cost. The previous pattern required `case` to
# follow the start of the body or one of `[\n;{}()&|]`, so `then case` and
# `do case` slipped past it while `ARM` still truncated the body at the inner
# `;;`. Measured on a synthetic `prose` arm reading
# `python3 scripts/prose_check.py && if true; then case "$OSTYPE" in *) : ;;
# esac; fi && python3 scripts/prose_check.py --extra`: the body was kept only
# as far as the inner case, `arms['prose']` held one command, `unseen` was
# `None`, no refusal fired and the real second `python3` command after the
# inner case was dropped at exit 0.
#
# The keyword list is spelled out rather than replaced by a bare `\bcase\b`,
# because a word boundary sits inside `--lower-case` too and an option is not
# a nested statement. `case` must also be FOLLOWED by whitespace, which the
# keyword always is and an option ending in `-case` is not.
#
# The `\b` in front of the alternation was a fail-open of the same shape one
# member further along, and the S03 review's eighth pass measured it. `\b`
# needs a WORD character immediately before the token it precedes, and `!` is
# not one, so `&& ! case ...` matched nothing while `ARM` went on truncating
# the body at the inner `;;`. MEASURED on a synthetic `prose` arm reading
# `python3 scripts/prose_check.py && ! case "$OSTYPE" in *) : ;; esac &&
# python3 scripts/prose_check.py --extra`: `arms['prose']` held one command,
# `unseen['prose']` was `None`, no refusal fired and the real trailing command
# was dropped at exit 0. The `\b` is gone and the keywords carry their own
# boundary, which they need and `!` does not: `!` cannot be the tail of a word.
#
# The alternation is BUILT from `SHELL_INTRODUCERS` above rather than written
# out a second time, which is the ninth pass's fix. A word introducer needs a
# `\b` in front of it and a punctuation one must not have it, so the set is
# split on that property here rather than hand-sorted into two literals.
_WORD_INTRODUCERS = sorted(w for w in SHELL_INTRODUCERS if w[:1].isalpha())
_PUNCT_INTRODUCERS = sorted(w for w in SHELL_INTRODUCERS if not w[:1].isalpha())
NESTED_CASE = re.compile(
    r"(?:^|[\n;{}()&|]|(?:"
    + "|".join([r"\b(?:" + "|".join(re.escape(w) for w in _WORD_INTRODUCERS)
                + r")"]
               + [re.escape(w) for w in _PUNCT_INTRODUCERS])
    + r")[ \t])[ \t]*case[ \t]")

# Where one shell statement ends and the next begins, for the unseen-command
# scan. `{` and `}` are separators here and never statement heads.
#
# Applied by `_split_statements` below rather than by `re.split`, because a
# separator inside a QUOTED STRING does not separate anything. `echo "a ;; b"`
# is one statement whose head is a builtin, and split blindly it became `echo
# "a`, ` b"` and a refusal naming `b"` as a command CI does not run. That is a
# guard refusing a legitimate state, which is the runbook's own sentence, and
# it is the same defect as the `;;` this pass fixed in the arm parser: shell
# read with a regex that does not know about quotes.
STATEMENT_BREAK = re.compile(r"[\n;{}()]|&&|\|\||\|")


def _split_statements(text: str) -> list[str]:
    """`text` split on `STATEMENT_BREAK`, quoted spans left whole."""
    pieces: list[str] = []
    current: list[str] = []
    quote = ""
    index = 0
    while index < len(text):
        if quote:
            char = text[index]
            if quote == '"' and char == "\\":
                current.append(text[index:index + 2])
                index += 2
                continue
            if char == quote:
                quote = ""
            current.append(char)
            index += 1
            continue
        separator = STATEMENT_BREAK.match(text, index)
        if separator is not None:
            pieces.append("".join(current))
            current = []
            index = separator.end()
            continue
        step, opened = _quote_spans(text, index)
        current.append(text[index:step])
        quote = opened
        index = step
    pieces.append("".join(current))
    return pieces

# Statement heads that are not the work a gate does, and whose REMAINDER is
# arguments rather than a command. A builtin decides whether the real command
# runs and is not the work, and demanding a gate-name step for one of them
# would refuse `lint`, `types` and `packages`, which run their whole arm as CI
# steps today and lose nothing by it. `[ -d node_modules ]` is the shape: its
# head is a builtin and `-d node_modules ]` is not a command, so re-scanning
# past THESE heads would report an argument list as an unseen command.
#
# `for` is here rather than above for the same reason: what follows it is a
# variable and a word list, and the command lives after the `do`.
#
# `command` is NOT here since the S03 review's eighth pass, and it was here for
# exactly the reason `eval` was: `command foo args` runs foo. It has its own
# reader below rather than a place in either set, because its options decide
# which of the two it is.
SHELL_NOISE = frozenset({
    "[", "[[", "test", "echo", "printf", "return", "exit", "skip",
    "local", "fi", "for", "done", "case", "esac", "true", "false", ":", "set",
    "shift", "read", "cd", "export", "unset",
})

# `command -v x` and `command -V x` LOOK a command up and run nothing, which is
# what `bin/ocelli.sh`'s `panic` arm uses. `command -p x` and a bare
# `command x` RUN it, which is the property that moved `eval` into
# `SHELL_INTRODUCERS`, and this file's own declared limit named `command`
# beside `eval` as the residue. MEASURED in the S03 review's eighth pass:
# rewriting the `bench` arm's `node --test` line as `command node --test ...`
# gave `unseen['bench'] == None` and exit 0 with the `gate bench` step replaced
# by its two extractable commands, so the five node suites stopped being
# demanded of CI. That is byte for byte the outcome the sixth and seventh
# passes each measured and made a rule.
COMMAND_LOOKUP_OPTIONS = frozenset({"-v", "-V", "-pv", "-pV", "-vp", "-Vp"})
COMMAND_PATH_OPTIONS = frozenset({"-p", "--"})


def command_builtin_runs(statement: str) -> str:
    """What `command ...` actually runs, or "" when it only looks one up.

    Fails CLOSED on an option this function does not model: the remainder is
    returned unchanged and the caller then reports it as a command it cannot
    see, which refuses. An option is not a command and the refusal will name a
    leading `-`, which is a message a maintainer can act on, and the
    alternative is a statement dropped in silence.
    """
    while statement.split(" ", 1)[0] == "command":
        rest = statement.partition(" ")[2].strip()
        while True:
            head = rest.split(" ", 1)[0]
            if head in COMMAND_LOOKUP_OPTIONS:
                return ""
            if head not in COMMAND_PATH_OPTIONS:
                break
            rest = rest.partition(" ")[2].strip()
        statement = rest
        if not statement:
            return ""
    return statement


def arm_bodies(runner: str) -> dict[str, str]:
    """Each gate's `run_gate` arm body, comments stripped, continuations joined.

    An arm ends at its `;;`. Nothing here can read a command out of prose. See
    the docstring's "Where an arm ENDS" section for what that cost.

    **The stripping happens BEFORE any `;;` is looked for, and doing it after
    was a fail-open the S03 review's eighth pass measured.** The arm ends at the
    first `;;`, and a `;;` inside a shell comment is not the end of an arm, so
    the body was truncated at the comment and only then were comments removed
    from what survived. MEASURED on a synthetic `prose` arm whose comment line
    read `# the voice rules, then the second pass ;; see the LLD` with a real
    `python3 scripts/prose_check.py --extra` after it: `arms['prose']` held one
    command, `unseen['prose']` was `None`, no refusal fired and the trailing
    command was dropped at exit 0. Over the whole region the comment is gone
    before any `;;` is looked for, and the comment blocks BETWEEN arms go with
    it, which the previous order also had to handle one arm at a time.

    **A `;;` inside a QUOTED STRING is not the end of an arm either, and that
    is the ninth pass.** Stripping comments first cannot help there, because a
    string is not strippable. MEASURED on the one-line `fmt` arm rewritten as
    `fmt) cargo fmt --all --check && echo "a ;; b" && python3
    scripts/prose_check.py --extra ;;`, which `sh -n` accepts: this check
    exited 0 with `arms['fmt']` holding one command, `unseen['fmt']` `None` and
    the real trailing command dropped. `_arm_end` scans for the first `;;`
    outside a quoted span now, and REFUSES when a quote is never closed rather
    than guessing which `;;` was meant.
    """
    try:
        region = runner[runner.index("run_gate() {"):runner.index("skip() {")]
    except ValueError as error:
        raise RuntimeError(
            "bin/ocelli.sh carries no `run_gate() {` ... `skip() {` region "
            "where this parser looks for the gate arms. Either the runner was "
            "restructured, in which case this parser has to be restructured "
            "with it, or the arms are gone. Both need a person, and neither "
            "may be read as agreement.") from error
    body = _strip_shell_comments(CONTINUATION.sub(" ", region))
    bodies: dict[str, str] = {}
    index = 0
    while True:
        match = ARM_LABEL.search(body, index)
        if match is None:
            break
        end, unreadable = _arm_end(body, match.end())
        if unreadable:
            raise RuntimeError(
                f"the `{match.group(1)}` arm in bin/ocelli.sh's `run_gate` "
                f"cannot be read to its end: {unreadable}. An arm this parser "
                f"cannot delimit is an arm whose commands it would report as "
                f"whatever it happened to stop at, and a `;;` inside a quoted "
                f"string was MEASURED to leave this check at exit 0 with the "
                f"rest of the arm dropped. Both need a person, and neither may "
                f"be read as agreement.")
        text = body[match.end():end]
        index = end + 2
        if NESTED_CASE.search(text):
            raise RuntimeError(
                f"the `{match.group(1)}` arm in bin/ocelli.sh's `run_gate` "
                f"holds a nested `case`. An arm ends at its `;;` here and a "
                f"nested case ends its own branches the same way, so this "
                f"parser would keep only what came before the first inner "
                f"`;;` and drop the rest of the arm without saying so. Either "
                f"move the nested case into a function the arm calls, or "
                f"teach this parser to balance `case`/`esac`. Both need a "
                f"person, and neither may be read as agreement.")
        bodies[match.group(1)] = text
    if not bodies:
        raise RuntimeError(
            "bin/ocelli.sh's `run_gate` declares no case arm this parser can "
            "read. Every gate would then have an empty arm, `covers` would "
            "fall back to the gate-name route for all of them, and the "
            "per-command rule would hold over nothing.")
    return bodies


def gate_commands(runner: str) -> dict[str, list[str]]:
    """The commands each gate's `run_gate` arm runs and this file can see."""
    arms: dict[str, list[str]] = {}
    for gate, text in arm_bodies(runner).items():
        found = re.findall(
            "(?:" + "|".join(re.escape(p) for p in COMMAND_PREFIXES) +
            r")[\w./ -]+", text)
        # Collapsed to single spaces. A joined continuation leaves the space
        # that sat before the backslash beside the one that replaced it, and
        # `runs_command` compares the text, so a doubled space would make an
        # arm command that CI runs verbatim look absent.
        arms[gate] = [re.sub(r"\s+", " ", c).strip() for c in found]
    return arms


def unseen_commands(runner: str) -> dict[str, list[str]]:
    """The statements in each arm that `gate_commands` cannot see.

    The extractor's vocabulary is `COMMAND_PREFIXES` and the arms run more than
    that: `node --test`, `wasm-pack build` and the `"$0"` self-calls. A gate
    holding one of these can only be run whole by a step that names the gate,
    and until the sixth pass that was a sentence in this file's docstring
    rather than a rule. It is a rule now, and `main` applies it per event.

    Shell builtins are not commands here, for the reason `SHELL_NOISE` gives.
    A control KEYWORD is not a command either, and dropping it takes the rest
    of the statement with it unless the rest is re-scanned, which is the
    fail-open `SHELL_INTRODUCERS` records and closes.
    """
    unseen: dict[str, list[str]] = {}
    for gate, text in arm_bodies(runner).items():
        found: list[str] = []
        for raw in _split_statements(text):
            statement = re.sub(r"\s+", " ", raw).strip()
            # `if node --test x` is one statement and `node --test x` is the
            # work in it. Looped, because `while ! python3 x` is two heads.
            while statement.split(" ", 1)[0] in SHELL_INTRODUCERS:
                head, _, rest = statement.partition(" ")
                statement = rest.strip()
                if not statement:
                    break
            if not statement:
                continue
            # `command x` runs x. Re-scanned rather than dropped, which is the
            # same rule `SHELL_INTRODUCERS` carries one line up.
            if statement.split(" ", 1)[0] == "command":
                statement = command_builtin_runs(statement)
            if not statement:
                continue
            if statement.startswith(COMMAND_PREFIXES):
                continue
            if statement.split(" ", 1)[0] in SHELL_NOISE:
                continue
            found.append(statement)
        if found:
            unseen[gate] = found
    return unseen


def main() -> int:
    runner = RUNNER.read_text()
    workflow = WORKFLOW.read_text()
    # Every parser here refuses rather than returning an empty result, and
    # until the fifth pass those refusals reached the terminal as a bare
    # traceback. A cosmetic reformat of the `--floor` case line into three arms
    # exited 1 with `RuntimeError:` and no `FAIL:` header, which is fail-closed
    # and is still the wrong way to tell a maintainer what to do. The refusals
    # are unchanged. Only how they are printed is.
    try:
        arms = gate_commands(runner)
        unseen = unseen_commands(runner)
        declared = declared_gates(runner)
        excluded = runner_excluded(runner)
    except RuntimeError as error:
        print("FAIL: CI does not run the whole floor")
        print(f"  {error}")
        return 1
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

    for gate in [g for g in declared if g not in NOT_IN_FLOOR]:
        running = steps_running(gate, arms, commands)
        # Per event, not per step. Two steps with complementary conditions
        # cover the floor between them, and asking one step to cover every
        # event refuses that arrangement while naming no missing event, which
        # is a message that cannot be acted on.
        #
        # Per event AND over the whole arm. `blocked` is the events no step
        # touching the gate reaches at all, `partial` is the events a step
        # reaches while leaving a command in the arm unrun.
        #
        # `unnamed` is the third: the events on which a gate whose arm holds a
        # command this file CANNOT see is not invoked by name. `covers` is
        # blind to those commands by construction, so it can answer yes over a
        # smaller arm, and a step naming the gate is the only thing that runs
        # them. Asked per event for the same reason as the other two.
        blocked: list[str] = []
        partial: dict[str, list[str]] = {}
        unnamed: list[str] = []
        for event in sorted(events):
            reachable = [c for c in commands if c.runs_on({event})]
            if unseen.get(gate) and gate not in invoked_gates(reachable):
                unnamed.append(event)
            if covers(gate, arms, reachable):
                continue
            if not any(c in running for c in reachable):
                blocked.append(event)
            else:
                partial[event] = missing_arm_commands(gate, arms, reachable)
        if events and running and not blocked and not partial and not unnamed:
            continue
        if running and events and (blocked or partial or unnamed):
            if unnamed:
                problems.append(
                    f"the `{gate}` gate is in the CI floor, its arm in "
                    f"bin/ocelli.sh runs "
                    f"{', '.join(repr(c) for c in unseen[gate])}, and no step "
                    f"in {WORKFLOW.relative_to(ROOT)} invokes "
                    f"`bin/ocelli.sh gate {gate}` on "
                    f"{', '.join(unnamed)}. This file's command extractor "
                    f"recognises {', '.join(repr(p) for p in COMMAND_PREFIXES)}"
                    f" and nothing else, so it cannot demand those commands "
                    f"step by step and must not report the arm covered "
                    f"without them. A step naming the gate runs the arm "
                    f"entire by definition, and that is the only form this "
                    f"check can accept here. Either restore the gate-name "
                    f"step, or exclude the gate from the floor in "
                    f"bin/ocelli.sh and say why.")
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
    for gate in sorted(NOT_IN_FLOOR & set(declared)):
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

    count = len([g for g in declared if g not in NOT_IN_FLOOR])
    print(f"OK: all {count} floor gate(s) are invoked by CI on "
          f"{', '.join(sorted(events))}, every command in each gate's arm")
    print(f"  the runner's --floor exclusion list and NOT_IN_FLOOR agree on "
          f"{', '.join(sorted(NOT_IN_FLOOR))}")
    # Named rather than left to the docstring, because the previous version of
    # this claim was a sentence in the docstring saying these gates were
    # invoked by name and nothing read it. The gates are derived from the arms
    # and the list is printed, so a gate joining or leaving it is visible in a
    # diff of this output rather than in a paragraph nobody re-reads.
    named_only = sorted(g for g in unseen
                        if g in declared and g not in NOT_IN_FLOOR)
    if named_only:
        print(f"  {', '.join(named_only)} hold arm command(s) this file "
              f"cannot extract, so each is required to be invoked by name and "
              f"each is. That was a declared limit until the sixth pass and "
              f"is a rule now.")
    for gate, reachable in sorted(reached_outside.items()):
        print(f"  outside the floor, CI runs `{gate}` on "
              f"{', '.join(reachable)}. That is what is PROVABLE from the "
              f"conditions. A condition naming github.ref is not evaluated "
              f"here, so an event missing from this list may still reach the "
              f"step on some branch and this check does not claim it does.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
