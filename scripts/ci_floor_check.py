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
esac`. The first is why the arm's end is found by a scan rather than by a
regex. The other two are why `NESTED_CASE`'s alternation is DERIVED
from `SHELL_INTRODUCERS` instead of being written out beside it: this file knew
`if`, `while` and `until` introduce a command in one function and not in the
other, and two lists that must agree are one list.

**"That same scan" was half true until the S03 review's eleventh pass, and the
other half was a live regression.** The arm's end, the comment strip and the
statement split shared the rule for OPENING a span and the set of delimiters.
The rule for CLOSING one was written out three times and agreed only because
all three were edited in one commit, which is the condition that sentence
claimed had been removed. The scan also had no `$( ... )` span, so when the
tenth pass made `)` a word start, which is bash's own rule, it could not tell
an operator `)` from the one closing a substitution: `echo $(printf x)#no &&`
in an arm dropped the rest of the line at exit 0, and the SAME input one commit
earlier exited 1. There is one tokenizer now, `shell_pieces`, with a span
stack, and its callers consume pieces. It reads the `GATES` array too, which is
what removed the second copy of the gate-row regex from
`scripts/guards/census.py`.

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

The four excluded gates are not excluded for the same reason, and the runner
says which is which without a second literal here. `oracle` is the one gate
`bin/ocelli.sh` marks `YES` in its GPU column, and deviation D-04 is that CI
has no GPU, so nothing in CI may run it. The other three are excluded for cost
or for the corpus, and CI runs each through the corpus-tooling or guards jobs:
`corpus`, `guards-deep` and `quirk-mutations`.

So the rule is: **a gate outside the floor that does not need a GPU must still
be run by some CI step, provably reachable on at least one event the workflow
declares.** State the claim exactly and no more, and print the events proved
rather than a sentence somebody has to keep current.

**Four sentences here described a job that no longer exists, and the S03
review's tenth pass found them still standing.** The `guards-deep` JOB was
gated to

    github.event_name == 'workflow_dispatch'
      || (github.event_name == 'push' && github.ref == 'refs/heads/main')

and the ninth pass deleted it, moving its one step into the `guards` job, which
is behind no condition at all. So the claim written here, "CI runs
`guards-deep` on `workflow_dispatch`", was narrower than what the check itself
now prints, which is `pull_request, push, workflow_dispatch`. It was stale
prose rather than a hole in the logic, and it sat in a DECLARED LIMIT in
`scripts/guards/catalogue.py` as well, where this project treats a declared
limit as load-bearing.

What is still true, and it is a property of this file rather than of one job:
`_permits` cannot evaluate `github.ref`, so an event is counted only where the
governing conditions are provably true on the event name alone. No step this
check reads sits behind a `github.ref` condition today, so nothing is currently
being dropped by that rule, and the OK line prints the event list it proved so
a reader compares it with the workflow rather than with a sentence. Deleting
the step, or putting it behind a condition that is false on every declared
event, is refused.

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

## The workflow is YAML, and it was read line by line until the twelfth pass

The eleventh pass's remediation predicted that the next reviewer would plant
inputs at the hand-rolled YAML reader, and the twelfth pass did exactly that
and found FOUR fail-open routes and FIVE refusals of legitimate workflows. Each
was measured at exit 0 where the check should refuse, or at exit 1 on a file
GitHub Actions runs correctly.

1. **Key order, and it is the one no spelling rule reaches.** `run_commands`
   attached whatever `if:` it had seen SO FAR to a `run:`, so

       - run: bin/ocelli.sh gate guards
         if: github.event_name == 'workflow_dispatch'

   exited 0 and the identical two lines in the other order exited 1. A YAML
   mapping has no key order, so those two files are one workflow and the check
   gave two answers. Two more spellings of the same thing, both measured at 0
   against a reversed control at 1: a quoted `"if":` key before the `run:`, and
   a multi-line flow mapping with `run` before `if`. All three land on
   `guards`, the gate that watches every other gate, in an edit that reads as
   tidying.
2. **`run: >` folding.** A folded block scalar's body was split into one
   `Command` per LINE, so `python3 scripts/prose_check.py` and
   `--only-this-one-file README.md` became two commands and the argv equality
   of `runs_command` matched the first. Exit 0, where the same narrowing
   written on one plain line exits 1. That is the seventh pass's hole reached
   by changing one character.
3. **A quoted key in `on:`.** `"pull_request":` narrowed the event set the
   floor is checked against, and with the `guards` step gated to push the check
   printed "all 25 floor gate(s) are invoked by CI on push" at exit 0. The head
   regex already spelled `(?:on|"on"|'on'|true)`, so the author knew keys may
   be quoted, and the rule was not carried into the body scan.
4. **A gate name in any `run:` TEXT, and a `run:` key at any depth.**
   `echo "if the size budget moves, run bin/ocelli.sh gate panic locally"`
   satisfied `panic` at exit 0, and so did a `run:` written as an action input
   under `with:`. `panic` is HLD section 23's wasm panic-hook proof, the one
   property no native test can observe.

A fifth, found while building the probe for the parse refusal and measured the
same way: `- run: [bin/ocelli.sh gate guards`, an unterminated flow sequence
that no YAML parser accepts and that GitHub Actions therefore cannot run at
all, left the line reader at exit **0** reporting the whole floor covered. The
text after `run:` still held the gate name, and a scanner that does not parse
cannot tell a workflow from a file that merely looks like one.

And the five it refused, each a workflow GitHub runs: a quoted `run:` scalar, a
block scalar with an explicit indentation indicator, a single-line flow-mapping
step, a trailing YAML comment on an `if:`, and `on:` written as a block
sequence. The comment one is the worst of them, because the refusal quoted a
condition that plainly does run on the events it said it did not.

### So this file parses the workflow, and takes a dependency to do it

`yaml.load(..., Loader=yaml.BaseLoader)` for the structure, and `shell_pieces`,
which this file already owns, for the `run:` body. **The dependency is the
cost and it is recorded as deviation D-17 rather than left quiet**, because a
FLOOR gate acquiring a third-party import is exactly the kind of thing this
project makes visible.

The alternative was considered and rejected, and the reason is not taste.
Dependency-free, the only move available is to REFUSE every spelling the reader
does not model. That closes routes 2, 3 and 4, and it does not close route 1,
because key order is not a spelling to refuse: it is what reading a tree line
by line produces. Worse, three of the five legitimate spellings above ARE the
spellings such a rule would have to refuse, so the fail-closed repair makes a
guard that refuses a legitimate state permanent by design at the same moment it
leaves the widest route open. A parser satisfies both halves at once, because
it accepts every legal spelling and yields ONE tree, and key order stops
existing as a concept rather than being defended against.

That is also the argument this file has already made three times. The shell
arms and the `GATES` array stopped being read with a regex in the ninth and
eleventh passes, and the declared-constant ratchet reads HLD 27.1's lint table
with `tomllib` for the same reason. YAML was the fourth foreign grammar here
and the only one still read by hand. `tomllib` was free because it is stdlib
and PyYAML is not, and that difference is a cost, not a different argument.

What the cost actually is, stated exactly. `pyproject.toml` pins
`pyyaml==6.0.3` beside its eight other pins. `.github/workflows/ci.yml`'s
`guards` job installs it, reading the version out of `pyproject.toml` rather
than carrying a second copy of it, and that job is the only one that runs the
`ci` gate or the probes that run this file inside a disposable clone. An absent
PyYAML is a loud refusal at import with the install command in it, never a
skipped check: the import guard below is the whole of that mechanism.

`BaseLoader` and not `SafeLoader`, deliberately, and it is the more restrictive
of the two rather than the looser. It constructs `str`, `list` and `dict` and
nothing else, resolving no implicit tag and no `!!python/` tag, so it cannot do
what `yaml.load` with the default loader can. What that buys here is that `on`
stays the STRING `'on'` instead of becoming YAML 1.1's boolean true, every
scalar arrives as `str`, and `(?:on|"on"|'on'|true)` goes away rather than
being carried forward into a tree reader. It also means a workflow whose
event block is written as the YAML 1.1 boolean spelling `true:` is read as
having no `on` key at all, which refuses with the top-level keys named. That is
fail-closed and it is declared here rather than found.

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

# The one third-party import in the CI floor, and the reason it is here is in
# the docstring's "So this file parses the workflow" section. It is deviation
# D-17.
#
# The failure it can produce is an ABSENCE, so it is caught here and turned
# into the same `FAIL:` header every other refusal in this file prints, with
# the install command in it. A floor gate whose dependency is missing must say
# so in one readable line rather than as an import traceback, and it must never
# be possible for it to say nothing: there is no fallback reader and no
# `except ImportError: pass` anywhere below, which is what makes the absence
# fail CLOSED.
try:
    import yaml
except ModuleNotFoundError as _error:  # pragma: no cover, environment
    raise SystemExit(
        "FAIL: CI does not run the whole floor\n"
        "  scripts/ci_floor_check.py reads .github/workflows/ci.yml with "
        "PyYAML and PyYAML is not installed. It is pinned in pyproject.toml, "
        "and `python3 -m pip install \"$(grep -om1 'pyyaml==[0-9.]*' "
        "pyproject.toml)\"` is what .github/workflows/ci.yml's `guards` job "
        "runs. It is `pip` rather than this repository's usual `uv sync "
        "--locked` on purpose: `bin/ocelli.sh` invokes this gate as a bare "
        "`python3`, which does not resolve imports from `.venv`, so syncing "
        "the project environment would install PyYAML where this gate never "
        "looks. This check reads YAML with a YAML parser since the S03 "
        "review's twelfth pass, which measured four fail-open routes and five "
        "false refusals at the hand-rolled reader it replaced, so there is "
        "deliberately no fallback to that reader here.") from _error

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
# run every floor probe twice.
#
# **THE TIMINGS ARE NOT WRITTEN HERE.** They are in
# `ci/guard-probe-budget.json` under `wall_clock_seconds`, and `--record-budget`
# is what writes them. This comment carried deep 23.8s against floor 15.9s
# while the recorded pair said otherwise, which the tenth pass fixed by copying
# the recorded pair into four files, and the eleventh pass found all four
# saying deep 27.2 and floor 18.3 while the file said 28.4 and 18.6, because
# the recording run moved them in the same commit that quoted them. A quoted
# number is a copy, and a copy of a measurement goes stale the next time the
# measurement is taken. So read the file, which is the rule the same commit
# already applied to the probe count in `.github/workflows/ci.yml`.
# `python3 scripts/guard_probe.py --list --profile deep` prints each
# probe's profile and what it needs, and reading that beats reading this. The
# profile filters the listing to the probes this paragraph is about, and it was
# load-bearing until the tenth pass, when bare `--list` printed the floor set
# alone and showed none of them.
#
# `quirk-mutations` is excluded because its fixed generator boundaries need
# the locked DICOM environment and its attribution boundary needs cargo. The
# corpus-tooling job installs both and invokes it on every event. The separate
# stdlib-only `quirks` gate remains in the floor.
# Kept here so this script fails if the runner's exclusion list changes without
# anyone thinking about CI.
NOT_IN_FLOOR = {"oracle", "corpus", "guards-deep", "quirk-mutations"}


# The `GATES=( ... )` array literal. The array is read as SHELL WORDS by
# `shell_words` below and not matched row by row, which is the S03 review's
# eleventh pass and is the third foreign grammar this file stopped reading with
# a regex.
#
# What that fixes, MEASURED at HEAD before the change. `bin/ocelli.sh` reads
# each entry as `IFS='|' read -r name gpu desc`, which imposes no character
# class on the name at all, and the two Python copies of the row regex, here
# and in `scripts/guards/census.py`, both spelled it `[a-z-]+`. So a gate named
# `prose2`, with a real arm beside it, gave bash 29 gates and Python 28,
# `scripts/ci_floor_check.py` exit 0, the census exit 0, `gates_declared`
# unmoved because the Python side never counted it, and `gate --floor`
# selecting it while no CI step ran it and no catalogue entry claimed it. An
# entry the reader cannot parse became an OMISSION rather than a refusal, and
# the check's own sentence, that `--floor` is what CI runs, was false in a
# direction with no detection anywhere.
#
# Latent today because no gate name carries a digit. It is a mechanism now
# rather than a coincidence: the name class is `GATE_NAME` below, an entry
# outside it is REFUSED by name, and `scripts/guards/census.py` calls this
# function rather than carrying a second copy of it.
GATES_ARRAY = re.compile(r"^GATES=\(\n(.*?)^\)[ \t]*$", re.M | re.S)

# What a gate name may be. Wider than the `[a-z-]+` two copies of a regex used
# to allow, because bash allows anything, and BOUNDED, because a name reaches
# `re.escape`-free patterns elsewhere in this file and in the catalogue's probe
# builders. A name outside it is a refusal here, which is the point: the reader
# does not get to drop an entry it cannot use.
GATE_NAME = re.compile(r"^[A-Za-z0-9_-]+$")

# The GPU column's two values. `bin/ocelli.sh`'s own comment above the array
# says `name|needs_gpu|description`, and `gpu_gates` reads `YES` to decide
# which excluded gate CI may not run at all under deviation D-04. A third
# spelling would be read as `no` by that function and as nothing by a reader.
GPU_COLUMN = ("no", "YES")


def gate_entries(runner: str) -> list[str]:
    """Every element of `bin/ocelli.sh`'s GATES array, as bash sees it.

    Scanned with `shell_words`, so an entry is a word rather than a line: two
    entries on one line, a comment between them and a blank line are all
    exactly what bash makes of them, and none of them can silently drop one.
    """
    block = GATES_ARRAY.search(runner)
    if block is None:
        raise RuntimeError(
            "bin/ocelli.sh carries no `GATES=(` ... `)` array where this "
            "parser looks for it. Every gate would then be undeclared, this "
            "check would have no floor to compare against and the census "
            "would have no gate list. Both need a person, and neither may be "
            "read as agreement.")
    return shell_words(block.group(1))


def gate_rows(runner: str) -> list[tuple[str, str, str]]:
    """Each GATES entry as `(name, gpu, description)`.

    Split exactly as `IFS='|' read -r name gpu desc` splits it, so a `|` in a
    description stays in the description.
    """
    rows = []
    for entry in gate_entries(runner):
        fields = entry.split("|", 2)
        rows.append(tuple(fields + [""] * (3 - len(fields)))[:3])
    return rows


def gate_row_problems(runner: str) -> list[str]:
    """Every GATES entry this reader cannot use, as a refusal.

    An entry that cannot be read is a REFUSAL and not an omission, which is
    the whole finding: a row regex that skips what it cannot match leaves the
    gate running in bash, selected by `--floor`, and invisible to every Python
    check that is supposed to demand a CI step and a catalogue entry for it.
    """
    problems = []
    for entry in gate_entries(runner):
        fields = entry.split("|", 2)
        if len(fields) < 3:
            problems.append(
                f"bin/ocelli.sh's GATES array carries the entry {entry!r}, "
                f"which is not `name|needs_gpu|description`. The runner reads "
                f"each entry with `IFS='|' read -r name gpu desc`, so a "
                f"missing field is an empty variable there and a row this "
                f"check has to guess at here. Write all three fields.")
            continue
        name, gpu, _ = fields
        if not GATE_NAME.match(name):
            problems.append(
                f"bin/ocelli.sh's GATES array declares the gate {name!r}, "
                f"whose name is outside `{GATE_NAME.pattern}`. bash imposes "
                f"no class on it and this check, the census and the "
                f"catalogue's probe builders all match gate names by pattern, "
                f"so a name outside the class runs in `gate --floor` and is "
                f"invisible to every one of them. That is refused rather than "
                f"skipped, because skipping it is the defect: it was measured "
                f"at exit 0 here and in the census with the gate running and "
                f"nothing in CI invoking it.")
        if gpu not in GPU_COLUMN:
            problems.append(
                f"bin/ocelli.sh's GATES array declares the gate `{name}` with "
                f"{gpu!r} in the GPU column, which is neither "
                f"{' nor '.join(repr(v) for v in GPU_COLUMN)}. `gpu_gates` "
                f"reads that column to decide which gate CI may not run at "
                f"all under deviation D-04, and an unrecognised value reads "
                f"there as `no`, which is the permissive answer.")
    return problems


def declared_gates(runner: str) -> list[str]:
    """Every gate name `bin/ocelli.sh` declares.

    THE reader. `scripts/guards/census.py` calls this rather than carrying a
    second copy, because two copies of a regex over a foreign grammar agreeing
    with each other is not either of them agreeing with the grammar.
    """
    declared = [name for name, _, _ in gate_rows(runner)]
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
    return {name for name, gpu, _ in gate_rows(runner) if gpu == "YES"}


# `case "$name" in oracle|corpus|guards-deep) continue ;; esac`, inside the
# `--floor` arm. Anchored on `"$name"` and on `continue`, which together occur
# once in the file, so this cannot drift onto some other case statement.
RUNNER_EXCLUSION = re.compile(
    r'case\s+"\$name"\s+in\s+([A-Za-z0-9_|-]+)\)\s*continue\s*;;\s*esac')


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


def workflow_tree(workflow: str) -> dict:
    """`.github/workflows/ci.yml` as the tree GitHub Actions reads.

    THE reader. Every question this file asks of the workflow is asked of this
    tree, so there is one grammar and one answer, and the four fail-open routes
    the twelfth pass measured at the line-by-line reader are gone as a class
    rather than one spelling at a time.

    Refuses on a file it cannot parse and on a top level that is not a mapping.
    A workflow this parser cannot read is not a workflow whose coverage it may
    report, and returning an empty tree would say "nothing in ci.yml runs the
    floor" about a file that runs it, which is a refusal naming the wrong
    thing.
    """
    try:
        tree = yaml.load(workflow, Loader=yaml.BaseLoader)
    except yaml.YAMLError as error:
        raise RuntimeError(
            f"{WORKFLOW.relative_to(ROOT)} cannot be parsed as YAML: "
            f"{str(error).strip()}. GitHub Actions reads this file with a "
            f"YAML parser and so does this check, so a file neither can read "
            f"is a workflow that does not run at all. That needs a person and "
            f"may not be read as agreement.") from error
    if not isinstance(tree, dict):
        raise RuntimeError(
            f"{WORKFLOW.relative_to(ROOT)} parses as "
            f"{type(tree).__name__} rather than as a YAML mapping, so it "
            f"declares neither `on:` nor `jobs:` and nothing in it could run "
            f"a floor gate.")
    return tree


def _shaped(value: object, kind: type, where: str) -> object:
    """`value` when it is `kind`, or the one refusal for a shape not modelled.

    ONE message for every shape this reader cannot use, so a workflow written
    in a form it does not model is a named refusal rather than a silently
    smaller answer. Under `BaseLoader` every scalar is a `str`, so `str` here
    means "a scalar" and nothing narrower.
    """
    if isinstance(value, kind):
        return value
    names = {dict: "a mapping", list: "a sequence", str: "a scalar"}
    raise RuntimeError(
        f"{WORKFLOW.relative_to(ROOT)} carries {where}, which this reader "
        f"cannot use: it is {type(value).__name__} where GitHub Actions and "
        f"this check both expect {names[kind]}. A shape this file does not "
        f"model is refused rather than skipped, because a skipped step is a "
        f"step whose gate then reads as uninvoked or, worse, as covered by "
        f"whatever was read instead.")


def workflow_events(workflow: str) -> set[str]:
    """The events `on:` declares, in every shape GitHub accepts.

    A mapping, an inline sequence, a block sequence and a bare scalar, which is
    four shapes where the sentence here named two and the reader handled two.
    The block sequence was the fifth of the twelfth pass's false refusals: it
    is what GitHub's own documentation writes as `on: [push, pull_request]`
    laid out over lines, and this file read it as declaring no automatic event
    at all and refused the whole floor.

    A workflow that declares none is refused by the caller rather than treated
    as covering everything. `on` is the string key here and not YAML 1.1's
    boolean, which is what `BaseLoader` buys and what removed the
    `(?:on|"on"|'on'|true)` alternation this function used to open with.
    """
    declared = workflow_tree(workflow).get("on")
    if declared is None or declared == "":
        return set()
    if isinstance(declared, str):
        return {declared}
    if isinstance(declared, list):
        return {str(_shaped(event, str, "an entry of `on:`"))
                for event in declared}
    return set(_shaped(declared, dict, "`on:`"))


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


# `continue-on-error` written so that it does NOT tolerate a failure. GitHub
# reads the key as a boolean or as an expression, `yaml.BaseLoader` hands every
# scalar over as a string, and YAML 1.1 spells false five ways. Anything not in
# this set tolerates, an unevaluated `${{ ... }}` expression included, which is
# the same direction `_permits` takes on an `if:` it cannot prove: a value this
# file cannot read as harmless is not read as harmless.
NOT_TOLERATING = frozenset({"", "false", "no", "off", "n", "0"})


def _tolerates_failure(value: object) -> bool:
    """Does this `continue-on-error:` value take a failure away."""
    return str(value).strip().lower() not in NOT_TOLERATING


@dataclass(frozen=True)
class Command:
    """One statement CI may execute, and whether it proves a gate runs.

    `tolerated` is the reason this command cannot supply that proof. Its failure
    may be discarded, or an earlier condition may skip it while a later command
    leaves the step green. It is "" when nothing takes the proof away.
    **It exists because `continue-on-error`
    was in the parsed tree and nothing read it, which the S03 review's
    thirteenth pass measured three ways**, each valid YAML, each leaving this
    check at exit 0 with the `guards` gate covered: the key on the `gate
    guards` STEP, the same key on the `guards` JOB, and `|| true` appended to
    the run. `--floor` claims to be what CI runs, and a step whose failure
    cannot fail the run does not run the gate in the sense that claim means, on
    the gate that watches every other gate, in one line that reads as
    tolerating flakiness.

    A tolerated command is kept in the list rather than dropped, so that `main`
    can say which shell shape took the gate away. Every consumer asks
    `runs_on` first and a tolerated command answers no on every event.
    """

    text: str
    conditions: tuple[str, ...]
    tolerated: str = ""

    def runs_on(self, events: set[str]) -> bool:
        if self.tolerated:
            return False
        return all(_permits(condition, event)
                   for condition in self.conditions
                   for event in events)


# The `shell:` values whose semantics this file has MEASURED, and the only
# ones under which it will read a `run:` body as bash.
#
# GitHub's default on Linux with no `shell:` is `bash -e {0}`, which is what
# `_tolerated_statements` models. `shell: bash` is `bash --noprofile --norc
# -eo pipefail {0}`, which is STRICTER: it adds pipefail, so this file's
# pipeline rule over-tolerates under it, and over-tolerating is a refusal.
# `shell: sh` is `sh -e {0}`, and it is NOT in the set. Every citation in this
# block is a `bash -ec` or `bash -e <file>` measurement, and on GitHub's ubuntu
# runners `/bin/sh` is dash rather than bash, so none of them transfers to it.
# The rules involved are POSIX and are very likely to hold, but this constant
# is named for what has been MEASURED and `sh` had not been, which the S03
# review's fifteenth pass called the one asserted row in the file. Refusing it
# costs nothing today: no step in `.github/workflows/ci.yml` sets `shell:` at
# all, and the refusal names the key and asks for the measurement.
#
# Everything else is refused BY NAME rather than read, which is what makes the
# set closed against values GitHub adds later. The dangerous value is not
# `python` or `pwsh`: neither is bash, and a Python program cannot carry the
# runner at a bash statement head outside a string, so both fail closed. It is
# a CUSTOM TEMPLATE. `shell: bash {0}` is still bash and has NO `-e`, so every
# statement in the body is swallowed. MEASURED in the S03 review's fourteenth
# pass, with the real `bin/ocelli.sh gate guards` step replaced by one under
# `shell: bash {0}`: this check exited 0. The same template written once at
# workflow level under `defaults:` took every `run:` in the file with it, also
# at exit 0. `guards` is the gate that watches every other gate.
MEASURED_SHELLS = frozenset({"bash"})


def _defaults_shell(node: dict, where: str) -> tuple[str, str]:
    """A `defaults.run.shell` and where it was written, ("", "") when absent.

    Settable at workflow level and at job level, the job overriding the
    workflow, and a step's own `shell:` overriding both. All three are read,
    because the one that matters is the one NO step mentions.
    """
    defaults = _shaped(node.get("defaults") or {}, dict,
                       f"a `defaults:` on {where}")
    run = _shaped(defaults.get("run") or {}, dict,
                  f"a `defaults.run:` on {where}")
    value = str(_shaped(run.get("shell", ""), str,
                        f"a `defaults.run.shell:` on {where}")).strip()
    return (value, f"`defaults.run.shell:` on {where}") if value else ("", "")


def _measured_shell(shell: str, where: str) -> None:
    """Refuse a `shell:` whose errexit this file has not measured."""
    if not shell or shell in MEASURED_SHELLS:
        return
    raise RuntimeError(
        f"{WORKFLOW.relative_to(ROOT)} sets {where} to `{shell}`, and this "
        f"check reads a `run:` body as `bash -e`, which is what GitHub runs "
        f"when no `shell:` is set. `{shell}` is refused rather than read "
        f"under that model, because a custom template such as `bash {{0}}` is "
        f"still bash with NO `-e`: every command in the body would be "
        f"swallowed and this file would go on reporting the gates as run. "
        f"Add it to `MEASURED_SHELLS` only together with the `bash -ec` "
        f"measurements that show what its errexit actually does.")


def run_commands(workflow: str) -> list[Command]:
    """Every line `.github/workflows/ci.yml` executes, and when.

    A `run:` is a STEP KEY, reached as `jobs.<id>.steps[n].run` and nowhere
    else, and its conditions are that step's `if:` and its job's. Both facts
    are read off the parsed tree, which is what closed the twelfth pass's four
    routes at once:

    - **Key order stops existing.** The previous reader walked lines and
      attached whatever `if:` it had seen SO FAR, so a step with `run:` written
      above `if:` exited 0 and the same two keys in the other order exited 1.
      A mapping has no order and this function no longer has one either.
    - **`run: >` folds.** The previous reader split a block scalar's body into
      one command per LINE, which turned one narrowed command into two and let
      the argv comparison in `runs_command` match the first half. The parser
      folds `>` and keeps `|`, exactly as GitHub does, and the split here is on
      the FOLDED value.
    - **A `run:` under `with:` is an action input**, not a step's shell, and a
      `run:` under `defaults:` is a shell setting. Neither is reached from
      `steps[n]`, so neither is a command any more. It was one at any depth.
    - **A quoted key is the same key.** `"if":` and `if:` are one key after a
      parse, where the reader's `^\\s*(?:-\\s+)?if:` matched only the second.

    A `#` in the value is a shell comment and is removed by `shell_pieces`, the
    scanner this file already reads `bin/ocelli.sh` with, so `echo "issue #12"`
    keeps its `#` and `true;#x` loses everything after the `;`. The YAML-level
    comment is gone before this function ever sees the value, which is the
    parser's job and used to be a second `#` rule here that could not tell the
    two apart.

    **The whole body is scanned ONCE since the S03 review's thirteenth pass,
    and it was scanned line by line before that.** One line of code produced a
    fail-open and a false refusal together, because cross-line shell state was
    discarded between the lines that establish it and the lines it governs.

    - Fail-open. A step whose body is `cat <<'EOF'` / `bin/ocelli.sh gate
      panic` / `EOF` satisfied the `panic` gate at exit 0 with the real step
      gone, and bash confirms the runner is never called: the body is DATA.
      That is the twelfth pass's route 4a, a gate name that is a mention rather
      than an invocation, in the spelling that pass did not close, on HLD
      section 23's wasm panic-hook proof. `echo not \\` then `bin/ocelli.sh
      gate panic` reaches the same place through the continuation, and bash
      prints `not bin/ocelli.sh gate panic`.
    - False refusal. `python3 scripts/staged_content_check.py \\` continued
      onto the next line made the `content` gate read as uninvoked at exit 1,
      while bash runs it as one command.

    Both were measured in a real clone at 0 and 1 respectively. The body now
    goes through `shell_source` and `_split_statements` exactly as a `run_gate`
    arm does, so the `BODY` piece kind drops a here-document here for the same
    reason it drops one there, `JOIN` joins a continuation, and a statement
    rather than a line is the unit. That also makes `a && b` written on one
    `run:` line two commands, so a step chaining two of a gate's arm commands
    now covers both, which the line reader could match against neither.

    A span the scanner cannot close REFUSES here, on `_arm_end`'s argument.
    Everything after an unclosed quote is swallowed into one span, which can
    only hide commands and therefore only cause refusals, but the refusal it
    causes names a gate rather than the quote, and a message that sends its
    reader to the wrong file is the shape this whole check has been rewritten
    for. bash will not run such a body either.
    """
    tree = workflow_tree(workflow)
    commands: list[Command] = []
    workflow_shell = _defaults_shell(tree, "the workflow")
    jobs = _shaped(tree.get("jobs") or {}, dict, "`jobs:`")
    for job_id, raw_job in jobs.items():
        job = _shaped(raw_job, dict, f"the job `{job_id}`")
        # NOT `... or workflow_shell`: `("", "")` is a non-empty tuple and is
        # TRUE, so the workflow-level default would never be reached. The
        # value is what decides, not the pair.
        job_shell = _defaults_shell(job, f"the job `{job_id}`")
        if not job_shell[0]:
            job_shell = workflow_shell
        job_if = str(_shaped(job.get("if", ""), str,
                             f"an `if:` on the job `{job_id}`")).strip()
        job_tolerates = _tolerates_failure(
            _shaped(job.get("continue-on-error", ""), str,
                    f"a `continue-on-error:` on the job `{job_id}`"))
        steps = _shaped(job.get("steps") or [], list,
                        f"a `steps:` on the job `{job_id}`")
        for index, raw_step in enumerate(steps):
            where = f"step {index + 1} of the job `{job_id}`"
            step = _shaped(raw_step, dict, where)
            if "run" not in step:
                continue
            body = str(_shaped(step["run"], str, f"a `run:` on {where}"))
            step_shell = str(_shaped(step.get("shell", ""), str,
                                     f"a `shell:` on {where}")).strip()
            _measured_shell(*((step_shell, f"a `shell:` on {where}")
                              if step_shell else job_shell))
            step_if = str(_shaped(step.get("if", ""), str,
                                  f"an `if:` on {where}")).strip()
            step_tolerates = _tolerates_failure(
                _shaped(step.get("continue-on-error", ""), str,
                        f"a `continue-on-error:` on {where}"))
            conditions = tuple(c for c in (job_if, step_if) if c)
            source = shell_source(body)
            _, unclosed = shell_pieces(source)
            if unclosed:
                raise RuntimeError(
                    f"the `run:` on {where} carries a {unclosed} span that is "
                    f"opened and never closed, so this parser cannot tell "
                    f"where one command in it ends and the next begins. bash "
                    f"will not run that body either. Fix the quoting, and do "
                    f"not read this refusal as a statement about the gates: "
                    f"the swallowed text can only HIDE commands from this "
                    f"file, so the message you would otherwise get names a "
                    f"gate rather than the quote.")
            _refuse_unmodelled_shell(source, where)
            tolerated = _tolerated_statements(source)
            for position, statement in enumerate(_split_statements(source)):
                text = statement.strip()
                if not text:
                    continue
                if step_tolerates:
                    reason = (f"`continue-on-error` on {where}")
                elif job_tolerates:
                    reason = (f"`continue-on-error` on the job `{job_id}`")
                elif position in tolerated:
                    reason = f"on {where}, {tolerated[position]}"
                else:
                    reason = ""
                commands.append(Command(text, conditions, reason))
    return commands


# `bin/ocelli.sh gate a b c` at the head of a statement. `a b c` runs three
# gates, so the names are split rather than kept as one string, and a name
# matches only whole: `guards` and `guards-deep` are two gates and neither
# satisfies the other.
GATE_INVOCATION = re.compile(
    r"bin/ocelli\.sh\s+gate\s+((?:[A-Za-z0-9_-]+[ \t]+)*[A-Za-z0-9_-]+)")


def invoked_gates(commands: list[Command]) -> set[str]:
    """The gate NAMES CI invokes through the runner.

    An invocation is the runner at a STATEMENT HEAD, which is what "CI runs the
    gate" means. A gate name inside a string, inside a comment or as an
    argument to some other command is a MENTION, and the floor's claim is about
    what runs. Route 4a of the twelfth pass measured the difference: the name
    was matched ANYWHERE in a `run:` line, and a string is anywhere, so
    `echo "... run bin/ocelli.sh gate panic locally"` satisfied `panic` at exit
    0 with the real step deleted. `panic` is HLD section 23's wasm panic-hook
    proof, the one property no native test can observe. It is the comment-only
    lesson one language along: a comment runs nothing and neither does a
    sentence in a string.

    This is where `shell_pieces` reaches a `run:` body, which it never did
    before, and it reaches it through `_split_statements`. That matters for
    more than the head: `echo "a; bin/ocelli.sh gate panic"` is ONE statement,
    because the `;` is inside a span and separates nothing, so the runner never
    appears at a head at all. Heads are then stripped with
    `SHELL_INTRODUCERS`, so this file asks the question about a CI step exactly
    as `unseen_commands` asks it about a gate arm, and `cd x && bin/ocelli.sh
    gate y` is two statements of which the second is an invocation.

    A separate span MASK over the statement was written first and removed, and
    the reason is worth keeping: with the match anchored at the head, a span
    can only sit at index 0, where the match already fails on the quote itself.
    Probe `ci-floor.gate-name-inside-a-run-string` stayed red with the mask
    made an identity function and reports HARNESS when the anchor is taken
    away, which is what says which half of the pair was carrying the weight.
    A defence no probe can reach is a comment claiming a property, and this
    file has been rewritten twice for exactly that.

    What the anchor costs, declared rather than left to be found: a genuine
    invocation this file cannot see at a statement head is not counted, so
    `sh -c 'bin/ocelli.sh gate x'` and `FOO=1 bin/ocelli.sh gate x` are
    refusals rather than passes. Both fail CLOSED and the refusal names the
    gate, which is a message a maintainer can act on.
    """
    names: set[str] = set()
    for command in commands:
        for statement in _split_statements(command.text):
            head = statement.strip()
            while head.split(" ", 1)[0] in SHELL_INTRODUCERS:
                head = head.partition(" ")[2].strip()
            match = GATE_INVOCATION.match(head)
            if match:
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
    of that, exact command equivalence is available only to an arm with one
    visible command. Several direct steps do not preserve an arm's order, job
    boundary or `&&` exit semantics, even if every argv is present.

    `bool(arm)` and not a bare `all()`. `native`, `panic` and `oracle` yield no
    extractable command, and `all([])` is true, so without this a gate whose
    arm this file cannot read would pass over an empty set and say so in the
    language of success.
    """
    if gate in invoked_gates(commands):
        return True
    arm = arms.get(gate, [])
    return len(arm) == 1 and not missing_arm_commands(gate, arms, commands)


# The four command prefixes this file can see. Declared as a constant so the
# limit is one thing to read rather than a regex to re-derive. `node`,
# `wasm-pack` and `"$0"` are deliberately absent, for the reason the docstring
# gives at length: adding them would make `panic` a gate with an extractable
# command and would break the claim that a gate whose arm yields none can only
# be satisfied by a step naming it.
COMMAND_PREFIXES = ("python3 ", "npm run ", "cargo ", "ci/")

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
ARM_LABEL = re.compile(r"^[ \t]*([A-Za-z0-9_-]+)\)", re.M)


# ---------------------------------------------------------------------------
# One shell tokenizer, with a span stack
# ---------------------------------------------------------------------------
#
# **This replaced `_quote_spans` and three hand-written copies of its CLOSE
# rule in the S03 review's eleventh pass, and the reviewer's sentence is the
# reason.** `_quote_spans` shared the rule for OPENING a span and the set of
# delimiters. Closing one was written out three times, in
# `shell_source`, in `_arm_end` and in `_split_statements`, and the
# three agreed only because all three were edited in one commit. The docstring
# claimed the drift `NESTED_CASE` and `SHELL_INTRODUCERS` suffered had been
# removed, and it had been removed from half the rule.
#
# It also could not tell an operator `)` from the `)` that closes a `$( ... )`,
# because it had no `$(` span at all. That was measured as a live regression:
# see `WORD_BREAK` below.
#
# So there is one scanner. It walks the text once with a STACK, and yields
# pieces its three callers consume: a piece is either one plain character
# outside every span, or a whole span from its opener to its closer. A caller
# never inspects the inside of a span and never has to know how one ends.
#
# **Was bash asked instead, and it can be asked. It is not, and the reason is
# three measurements rather than a preference.** The S03 review's twelfth pass
# put the question exactly right: `bin/ocelli.sh` is bash, `declare -f run_gate`
# is bash's own reparse of the function, and `test_the_names_match_bash` already
# asks bash about the `GATES` array while nothing asked it about the arms.
#
# 1. `declare -f` needs the function DEFINED, and defining it needs the file
#    EXECUTED. `set -n` is the only way to ask bash to parse without running,
#    and it does not define functions: MEASURED, `bash -c 'set -n; source
#    f.sh; set +n; declare -f run_gate'` prints nothing. So the reader would
#    source the file. This check exists to read a `bin/ocelli.sh` that somebody
#    has changed, and `scripts/guards/catalogue.py` plants adversarial shell
#    into that very file inside a disposable clone and runs this check over it.
#    A reader that sources turns "this file is misparsed" into "this file is
#    run", which is a strictly worse failure than the one it repairs.
# 2. `declare -f`'s output is a PRETTY-PRINTED form with no stability contract,
#    and it differs between the two bash versions on this machine. MEASURED on
#    the same input: bash 5.3.15 prints `cat <<'EOF'` and `esac`, and
#    /bin/bash 3.2.57 prints `cat  <<'EOF'`, two spaces, and `esac;`. Consuming
#    that as text is a hand-written model of an undocumented printer, which is
#    the defect class one layer along rather than an escape from it.
# 3. What it would have bought is smaller than it looks. `declare -f` strips
#    comments and joins continuations, both of which the scanner below now does
#    from the grammar. It does NOT normalise the two spellings the twelfth pass
#    measured as fail-open: `<<\EOF` comes back as `<<'EOF'` and `<<'EOF-1'`
#    comes back unchanged, so the delimiter production had to be fixed either
#    way.
#
# So bash IS asked, in `scripts/tests/test_guard_readers.py`, where it is safe
# and where a divergence is a red test rather than an executed plant: each arm
# shape below is run as a synthetic `case` of `echo` markers and what bash
# PRINTS is compared with what this scanner attributes to the arm. That is
# asking bash for the production. It is not asking bash to be the reader.
#
# WHAT STAYS HAND-ROLLED, then, stated as what it cannot see rather than as a
# list of constructs it can.
#
# **This section stated the residue as ONE thing until the S03 review's
# thirteenth pass, and one thing was not the count.** It said the scanner does
# not model compound commands and stopped there. That is accurate about
# `shell_pieces`, and `shell_pieces` is not the whole shell reader: it is ONE
# tokenizer with FOUR hand-written productions sitting on top of it, and the
# fourteenth route lived in the first of them. "One tokenizer" was achieved and
# "one reader" was not, and the limit read as if it had been. So the residue is
# stated per production.
#
# THE TOKENIZER, `shell_pieces`. It models spans and words. It does not model
# bash's COMPOUND COMMANDS, so it cannot tell a `(` that opens a subshell from
# one that ends a `case` pattern, and it cannot tell `((` arithmetic from
# `( (` nested subshells the way bash does, which is by trying the arithmetic
# parse and backtracking. Every consequence is fail-closed and asserted rather
# than assumed: `$(case y in *) ... esac)` closes at the pattern's `)` and the
# arm then ends at the inner `;;`, which `NESTED_CASE` refuses, and
# `scripts/tests/test_guard_readers.py` asserts that it is the refusal and not
# the scan carrying the weight. `(( a << b ))` reads the `<<` as a
# here-document whose body then runs to the end of the region, which refuses.
# `$'...'` is a span, so its EXTENT is right and its C escapes are not decoded,
# so `shell_words` yields `a\'b` where bash yields `a'b`, which in the `GATES`
# array is a name outside `GATE_NAME` and a refusal. A here-document written
# INSIDE a command substitution is not queued, because the scanner does not
# look inside a span. Doing better needs a compound-command parser, which is a
# second grammar, and the point of one tokenizer is that there is not one.
#
# `STATEMENT_BREAK` and `_split_statements`, where one statement ends. It sees
# the control operators and nothing else about a list. It cannot see that a
# `coproc` names its own compound command, and `coproc case x in *) : ;; esac`
# is refused only because `unseen_commands` reports `coproc case x in *` as a
# command CI does not run, which is a real refusal and not the one the compound
# limit above claims carries the weight: `NESTED_CASE` misses that shape.
# `_tolerated_statements` reads the SAME separators to decide whose failure
# `bash -e` discards, so it inherits every one of these blind spots and is
# conservative where it cannot see: a shape it cannot resolve leaves a command
# uncounted rather than counted.
#
# `NESTED_CASE`, where an arm holds a nested `case`. Its alternation is DERIVED
# from `SHELL_INTRODUCERS`, so it sees a `case` after a keyword, after a
# control operator and after a backtick. It does not see one introduced by a
# word `SHELL_INTRODUCERS` does not list. MEASURED: `time case x in *) : ;;
# esac` matches, because `time` is in that set, and `coproc case x in *) : ;;
# esac` does NOT, and `bash -n` accepts both. The `coproc` shape fails closed
# anyway, through the statement scanner rather than through this pattern, which
# is the row above. That is worth the line: the refusal carrying the weight is
# not the one the compound-command limit names.
#
# `SHELL_INTRODUCERS` against `SHELL_NOISE`, whether a head is the work or a
# decision about the work. `eval` and `command` were each measured on the wrong
# side of that line and each has a reader now. What remains is any OTHER head
# whose remainder is really a command, and the split is a list rather than a
# grammar, so a builtin bash adds is a builtin this file does not know.
#
# `ARM_LABEL` and `GATE_INVOCATION`, where an arm and an invocation begin. Both
# anchor at a statement head, so both inherit the statement scanner's residue
# rather than adding one, and `invoked_gates` declares what its anchor costs at
# its own docstring: `sh -c 'bin/ocelli.sh gate x'` and `FOO=1 bin/ocelli.sh
# gate x` are refusals rather than passes.
#
# MEASURED over both regions this scanner is used on, in the S03 review's
# eleventh pass and re-measured in the twelfth: after comments are stripped,
# the `run_gate` region carries 0 `$'`, 0 `$(`, 0 `${`, 0 `<<` and 0 backticks,
# against 48 backticks before stripping, and the `GATES` array carries 0 of all
# five. The `$(` count was 0 before the eleventh pass's fix as well, which is
# the point: the shape that walked past this parser was planted, and a parser
# that only handles what the file happens to contain today is the class these
# passes are about.


@dataclass(frozen=True)
class Span:
    """One kind of shell span, and how the scanner leaves it.

    **Every field below is a measurement against bash, not a reading of the
    manual.** The script each row was measured with is in
    `scripts/tests/test_guard_readers.py`, which re-runs it, because a table
    of shell facts nothing re-derives is the shape four review passes have
    already gone wrong on.
    """

    close: str
    # Does a backslash escape the next character inside.
    #
    # MEASURED. `echo 'a\'` prints `a\`, so a single quote takes none. `echo
    # "a\" ;; b"` prints `a" ;; b`, so a double quote does. `x=`printf "%s"
    # a\`b`` is an unterminated substitution to bash, so a backtick does: the
    # `` \` `` did not close it and bash ran to end of file looking for one.
    escapes: bool
    # The character that DEEPENS this span, so `$( (printf x) )` closes at the
    # outer `)` and not at the inner one. "" for a span that does not nest.
    #
    # MEASURED. `echo A$( (printf x) )#b && echo RAN_SECOND` prints `Ax#b` and
    # `RAN_SECOND`, so bash balanced the bare parentheses. `${u:-'}'}` prints
    # `}`, so the brace inside the quote did not close the expansion, and
    # `${u:-a\}b}` prints `a}b`, so a backslash does not either.
    nests: str
    # Which openers may open a span INSIDE this one, and it is a SET rather
    # than the boolean it was until the S03 review's twelfth pass, because
    # bash's answer is per pair and the boolean forced two of the pairs wrong.
    #
    # MEASURED, and the backtick was a fail-open. `x=`printf '%s' 'a`b'`` is
    # an error to bash, "unexpected EOF while looking for matching `''", so a
    # quote inside a backtick does NOT hide the closing backtick: the backtick
    # runs to the next unescaped one and nothing opens inside it. The scanner
    # said the whole thing was one closed span and accepted a file bash
    # refuses.
    #
    # MEASURED the other way for the double quote, which was the declared
    # limit until the same pass. `"a$(printf "%s" X)b"` prints `aXb`,
    # `"a`printf b`c"` prints `abc` and `"${u:-"}"}"` prints `}`, so all three
    # substitutions open inside a double quote and carry their own nesting.
    # `"a'b"` prints `a'b` and `"$'a'"` prints `$'a'`, so the two quote forms
    # do not.
    inner: tuple[str, ...]
    # Does quote removal drop this span's delimiters when a WORD is read.
    # `shell_words` decided this with a literal `{"'", '"'}` set, which is a
    # second table beside this one, and a here-document delimiter needs the
    # same answer.
    strips: bool


# Every opener, longest first, so `$(` is seen as itself rather than as a `$`
# followed by a `(`. Declared before the table because the two substitutions
# admit every opener and say so by naming this rather than by repeating it.
SPAN_OPENERS = ("$(", "${", "$'", "'", '"', "`")

SPANS: dict[str, Span] = {
    "'": Span("'", False, "", (), True),
    '"': Span('"', True, "", ("$(", "${", "`"), True),
    "`": Span("`", True, "", (), False),
    "$(": Span(")", True, "(", SPAN_OPENERS, False),
    "${": Span("}", True, "{", SPAN_OPENERS, False),
    "$'": Span("'", True, "", (), True),
}

# A here-document redirection OPERATOR, and nothing about its delimiter.
#
# **The delimiter used to be modelled here, as `(['\"]?)([A-Za-z_]\w*)\2`, and
# that was the S03 review's twelfth fail-open.** bash takes a WORD, and a word
# is not a character class. MEASURED, each shape planted in the `prose` arm
# with a real `python3 scripts/prose_check.py --probe-extra` after it, each
# accepted by `bash -n`, each leaving this check at exit 0 with that command
# dropped and bash really running it:
#
#     : <<\EOF        the third quoting mechanism, beside `'` and `"`
#     : <<'EOF-1'     `\w*` stops at the hyphen, so `\2` then fails to match
#
# When the redirection is not recognised the body is scanned as CODE, so a `;;`
# in it ends the arm and everything after it leaves in silence.
#
# So the delimiter is read as a word by the scanner itself, which is the one
# word production this file has, and `shell_words` reads the `GATES` array with
# the same one. What that production is, MEASURED rather than recalled:
#
# - It is not expanded. `cat <<EOF$X` wants the literal `EOF$X`, which bash
#   says in as many words when it hits end of file.
# - It is subject to quote removal only. `cat <<a"b"c` is terminated by `abc`,
#   and `cat <<a$(b)c` by the literal `a$(b)c`, so `$( )` delimits the word
#   without being expanded in it.
# - It ends at a blank, a newline or the first character of an operator.
#   `cat <<EOF; echo AFTER`, `cat <<EOF|cat`, `cat <<EOF>/dev/null` and
#   `cat <<EOF & wait` all terminate on `EOF` and run what follows.
#
# `<<<` is a here-STRING, three characters of one operator that takes a word
# rather than a body, so the negative lookahead is load-bearing.
HEREDOC_OPERATOR = re.compile(r"<<(-?)(?!<)[ \t]*")

# Where a shell WORD ends when it is not quoted, and equally what may sit
# before a `#` for that `#` to begin a comment. POSIX and bash end a word at a
# blank, at a newline or at the first character of an operator, and begin a
# word in exactly those places, so these are one fact and were two constants
# until the S03 review's twelfth pass. Two lists that must agree are one
# list, which is the repair `NESTED_CASE` and `SHELL_INTRODUCERS` got in the
# ninth pass and the runner's exclusion list got in the fourth.
#
# The comment half is the tenth pass's finding: `_strip_shell_comments`
# accepted the start of input, a blank and a newline only, so `true;#;;`
# planted in the one-line `fmt` arm dropped the trailing command.
#
# **`)` here was a REGRESSION for as long as the scanner had no `$(` span, and
# the eleventh pass measured it.** The rule is right and the implementation
# could not tell an operator `)` from the `)` closing a `$( ... )`. MEASURED:
# `bash -c 'echo A$(printf x)#no && echo RAN_SECOND'` prints both lines, so
# bash does not begin a comment there, and with `echo $(printf x)#no && cargo
# fmt --all --check --probe-extra &&` planted in the `fmt` arm this check
# exited 0 with the trailing command dropped, while the SAME input at
# `e2b11d8`, before `)` joined the set, exited 1. Failing closed there was
# luck: the `#no` survived as a token whose head is not a builtin.
#
# The scanner answers it now instead of a tuple doing so. A `)` the scanner
# opened is inside a span and is never a piece a caller sees, and a `)` that is
# really an operator, which is what a `case` pattern ends with, still is. A `#`
# straight after a closing quote or substitution is NOT a comment in bash,
# `echo "a"#b` prints `a#b`, and a span therefore leaves the scanner not at a
# word start, which is the direction this must not widen into.
WORD_BREAK = " \t\n;&|<>()"


def _opener_at(text: str, index: int,
               allowed: tuple[str, ...] = SPAN_OPENERS) -> str:
    """The span opener starting at `index`, or "" for none.

    `allowed` is the current span's `inner`, so an opener bash does not honour
    inside a backtick or a quote is not honoured here either. It defaulted to
    everything and was called unconditionally from inside a span, which is why
    `test_a_nested_substitution_closes_at_the_outer_paren` never reached the
    `nests` counter: the inner `$(` was re-opened as a span whatever the
    counter said.
    """
    for opener in SPAN_OPENERS:
        if opener in allowed and text.startswith(opener, index):
            return opener
    return ""


def _heredoc_end(text: str, start: int,
                 pending: list[tuple[str, bool]]) -> tuple[int, str]:
    """Where the bodies of the here-documents `pending` declares end.

    Several may be queued on one line, `cmd <<A <<B`, and their bodies follow
    in order. Returns the end and the delimiter of the FIRST body that reaches
    the end of the text without meeting its terminator, "" when every body
    closed.

    **That second value is new in the S03 review's twelfth pass and the
    docstring here claimed it for two passes without it existing.** It said an
    unterminated body "reports the whole remainder so the caller's
    unclosed-span refusal is what fires", and the caller cleared `pending`
    before its own `if pending:` test, so nothing fired. MEASURED with
    `: <<EOF-1` planted in the `prose` arm, the near miss of the second
    fail-open above: the check refused, which is the right direction, saying
    "the arm reaches the end of the region with no `;;` terminator" about an
    arm whose `;;` is present and whose here-document is the actual problem.
    """
    index = start
    for delimiter, stripped in pending:
        while index < len(text):
            end = text.find("\n", index)
            line = text[index:len(text) if end == -1 else end]
            index = len(text) if end == -1 else end + 1
            if (line.lstrip("\t") if stripped else line) == delimiter:
                break
        else:
            return len(text), delimiter
    return index, ""


# What a piece of a scanned shell text is. `TEXT` is one character outside
# every span, `SPAN` is a whole quoted or substituted region including its
# delimiters, `COMMENT` is a `#` that begins a word and everything after it on
# its line, `JOIN` is a line continuation, a backslash and the newline it eats,
# and `BODY` is a here-document body together with the line that terminates it.
#
# BODY is its own kind rather than a `SPAN` because it is DATA and a span is
# not. `_split_statements` and `shell_words` drop it, which is the fix for a
# false refusal both the eleventh pass's scanner and its predecessor had:
# MEASURED with a legitimate `: <<'EOF-1'` note in the `prose` arm, this check
# exited 1 reporting that the arm "runs 'a note', 'EOF-1'", two lines of
# English demanded of CI as commands. `shell_source` KEEPS it, because the text
# it returns is re-scanned by `_arm_end` and a source with the body cut out
# declares a here-document whose terminator is then missing.
#
# COMMENT is a piece of this scanner rather than a pass before it, and that is
# not a preference. A `#` opens a comment only outside a span, and a span opens
# only outside a comment, so the two rules are one rule and a scanner that ran
# them in sequence would be wrong in whichever order it chose: strip comments
# with a scanner that knows spans and an apostrophe in `# don't` opens a span
# that runs to the next quote in the file, and scan spans first and a `;;`
# inside a comment ends an arm, which is the eighth pass's measured fail-open.
#
# **JOIN is a piece here for the same reason, and it was a regex pre-pass over
# shell until the S03 review's twelfth pass.** `arm_bodies` ran
# `CONTINUATION.sub(" ", region)` BEFORE the tokenizer, which is exactly what
# this scanner's header argues no pass may do. MEASURED: a backslash ending a
# COMMENT line continues nothing in bash, `echo A # c \` then `echo B` prints
# both, and the pre-pass joined the next line INTO the comment before the
# tokenizer could see either. With `# the voice rules, and the flag below \`
# planted above a real `python3 scripts/prose_check.py --probe-extra ;;` in the
# `prose` arm, that command vanished entirely, `arms['prose']` came back
# holding the `content` gate's command instead, and the check failed closed
# only because the swallowed `content` arm then had no body, refusing while
# naming two gates neither of which was the one edited.
TEXT = "text"
SPAN = "span"
COMMENT = "comment"
JOIN = "join"
BODY = "body"


def shell_pieces(text: str) -> tuple[list[tuple[int, int, str]], str]:
    """`text` as `(start, end, kind)` pieces, plus an unclosed opener.

    A `TEXT` piece is one character, except that a backslash escape outside
    every span is one piece of two and a here-document redirection OPERATOR is
    one piece, neither of which can hold anything a caller looks for. A `SPAN`,
    a `COMMENT` or a `JOIN` piece is the whole region.

    The second return value is the opener of a span that is never closed, or
    `<<` and the delimiter of a here-document whose body never meets it, and ""
    when everything closed. `_arm_end` refuses on it rather than guessing.
    """
    pieces: list[tuple[int, int, str]] = []
    stack: list[list] = []
    pending: list[tuple[str, bool]] = []
    # The here-document delimiter being read, as the index of the first piece
    # of its word and whether the `<<-` form stripped tabs. The word is read by
    # this same loop rather than by a pattern of its own, which is what makes
    # the delimiter production and `shell_words`'s word production one
    # production. See `HEREDOC_OPERATOR` for what that production is and how
    # each half of it was measured.
    delimiter: tuple[int, bool] | None = None
    word_start = True
    span_start = 0
    index = 0
    while index < len(text):
        char = text[index]
        if stack:
            opener, span, depth = stack[-1]
            if span.escapes and char == "\\" and index + 1 < len(text):
                index += 2
                continue
            if span.nests and char == span.nests:
                stack[-1][2] = depth + 1
                index += 1
                continue
            if text.startswith(span.close, index):
                if depth:
                    stack[-1][2] = depth - 1
                else:
                    stack.pop()
                index += len(span.close)
                if not stack:
                    pieces.append((span_start, index, SPAN))
                    # A `#` straight after a closing quote or substitution is
                    # NOT a comment in bash: `echo "a"#b` prints `a#b`. This is
                    # the line that answers the tenth pass's `)` regression,
                    # and it answers it in the scanner rather than by taking
                    # `)` back out of `WORD_BREAK`, which a `case` pattern
                    # needs.
                    word_start = False
                continue
            nested = _opener_at(text, index, span.inner)
            if nested:
                stack.append([nested, SPANS[nested], 0])
                index += len(nested)
                continue
            index += 1
            continue
        # The delimiter word ends here, and the check comes before every rule
        # below it because the newline that ends the word is also the newline
        # that starts the body.
        if delimiter is not None and char in WORD_BREAK:
            word = _word_value(text, pieces[delimiter[0]:])
            if not word:
                # `<<` with no word, which bash refuses to parse at all.
                return pieces, "<<"
            pending.append((word, delimiter[1]))
            delimiter = None
        if char == "#" and word_start:
            newline = text.find("\n", index)
            end = len(text) if newline == -1 else newline
            pieces.append((index, end, COMMENT))
            index = end
            word_start = False
            continue
        if char == "\\" and index + 1 < len(text):
            kind = JOIN if text[index + 1] == "\n" else TEXT
            pieces.append((index, index + 2, kind))
            index += 2
            # A JOIN leaves the scanner exactly where the backslash was, which
            # for a word start is the character before it. Nothing here needs
            # that character back, because a continuation inside a word is
            # removed and a continuation between words sits after a blank, so
            # False is the answer in the direction that cannot widen.
            word_start = False
            continue
        if char == "\n" and pending:
            pieces.append((index, index + 1, TEXT))
            body, unterminated = _heredoc_end(text, index + 1, pending)
            pieces.append((index + 1, body, BODY))
            pending = []
            index = body
            word_start = True
            if unterminated:
                return pieces, f"<<{unterminated}"
            continue
        redirect = (HEREDOC_OPERATOR.match(text, index)
                    if delimiter is None else None)
        if redirect is not None:
            pieces.append((index, redirect.end(), TEXT))
            index = redirect.end()
            delimiter = (len(pieces), redirect.group(1) == "-")
            word_start = False
            continue
        opener = _opener_at(text, index)
        if opener:
            span_start = index
            stack.append([opener, SPANS[opener], 0])
            index += len(opener)
            continue
        pieces.append((index, index + 1, TEXT))
        word_start = char in WORD_BREAK
        index += 1
    if stack:
        # The unclosed span is still a PIECE, so `shell_source` keeps its text
        # and the refusal comes from `_arm_end`, which is the caller that can
        # say what it cost. Dropping it here instead would delete the rest of
        # the region from the arm parser in silence, which is the failure shape
        # this whole file is about.
        pieces.append((span_start, len(text), SPAN))
        return pieces, stack[0][0]
    if delimiter is not None:
        # The delimiter word runs to the end of the text, so the body is
        # absent rather than unterminated. Named the same way, because the
        # thing a maintainer has to look at is the same redirection.
        word = _word_value(text, pieces[delimiter[0]:])
        return pieces, f"<<{word}" if word else "<<"
    if pending:
        return pieces, f"<<{pending[0][0]}"
    return pieces, ""


def _word_value(text: str, pieces: list[tuple[int, int, str]]) -> str:
    """The pieces of ONE shell word, after quote removal.

    The whole of what "a word" means here, in one place, so the `GATES` array
    and a here-document delimiter get the same answer. A span contributes its
    contents when `Span.strips` says bash removes its delimiters and itself
    otherwise, a backslash escape contributes the character it escapes, a line
    continuation contributes nothing, and everything else contributes itself.

    A substitution and a here-document body are contributed VERBATIM, because
    this scanner does not run the shell and a word it cannot resolve has to
    stay visible to whoever reads it. In the `GATES` array that lands outside
    `GATE_NAME` and refuses.
    """
    out: list[str] = []
    for start, end, kind in pieces:
        if kind in (COMMENT, JOIN, BODY):
            continue
        if kind == SPAN:
            opener = _opener_at(text, start)
            span = SPANS.get(opener)
            # Both delimiters have to be THERE. A `SPAN` piece is also how an
            # unclosed span and a here-document BODY reach a caller, and a body
            # that happens to begin with a quote would otherwise lose its first
            # and last character to a closer that was never scanned.
            closed = (span is not None
                      and end - start >= len(opener) + len(span.close)
                      and text.endswith(span.close, start, end))
            if closed and span.strips:
                out.append(text[start + len(opener):end - len(span.close)])
            else:
                out.append(text[start:end])
            continue
        if text[start] == "\\" and end - start == 2:
            out.append(text[start + 1])
            continue
        out.append(text[start:end])
    return "".join(out)


def shell_source(text: str) -> str:
    """`text` as the shell reads it: comments gone, continuations joined.

    `SHELL_COMMENT` was `(?<!\\S)#.*$`, which cannot tell a comment from a `#`
    inside a string. It ran over the whole `run_gate` region, so a legitimate
    `echo "issue #12"` in an arm would have been truncated mid-string and the
    quote left open. Nothing in the region carries one today, and the whole
    point of the ninth pass's arm work is that the parser must not depend on
    that staying true.

    **The first sentence said "every `#` comment" and that was wider than the
    test under it until the S03 review's tenth pass.** POSIX and bash begin a
    comment at any `#` that starts a word, and a word starts after an operator
    as well as after a blank, so `;#`, `&#`, `|#`, `(#`, `)#`, `<#` and `>#`
    all open one. The test accepted start-of-input, space, tab and newline
    only. MEASURED on the one-line `fmt` arm with `true;#;;` planted in it: the
    trailing command was dropped, and it failed CLOSED only by accident,
    because the residue's head was `#` and the refusal named `#` as a command
    CI does not run.

    **Then `)` in that set was itself a fail-open until the eleventh pass**,
    because the scanner had no `$( ... )` span and could not tell an operator
    `)` from the one closing a substitution. Where a comment BEGINS is
    `shell_pieces`'s answer now, not this function's, so the three callers
    cannot come to disagree about it.

    **This was `_strip_shell_comments` and the continuation was a regex
    pre-pass beside it until the twelfth pass**, which is the fail-open
    `JOIN` records above. Both are the scanner's answer now, for the same
    reason the comment already was: a backslash is a continuation only outside
    a comment, and a comment ends at a newline a continuation would otherwise
    eat, so the two rules are one rule.

    What this does NOT remove, stated exactly: a continuation INSIDE a double
    quote or a substitution, which bash also removes and which this leaves in
    place because a span is opaque to every caller here. It reaches only
    `runs_command`'s text comparison, where it would make an arm command CI
    runs verbatim look absent, which is a refusal.
    """
    pieces, _ = shell_pieces(text)
    return "".join(text[start:end] for start, end, kind in pieces
                   if kind not in (COMMENT, JOIN))


def _arm_end(text: str, start: int) -> tuple[int, str]:
    """Where the arm beginning at `start` ends, or why this parser cannot say.

    The first `;;` OUTSIDE every span and every comment. Returns its index and
    an empty reason, or `-1` and the reason to refuse. Three reasons, all
    fail-closed: a span that is never closed, a here-document whose body never
    meets its delimiter, and an arm with no terminator at all.

    The here-document reason is separate from the span one since the S03
    review's twelfth pass, and it is not cosmetic. `: <<EOF-1` in an arm
    used to reach the third reason, which says the arm has no `;;` about an
    arm whose `;;` is right there, and sends a maintainer to the wrong line.
    """
    pieces, unclosed = shell_pieces(text[start:])
    for piece_start, _end, kind in pieces:
        if kind != TEXT:
            continue
        if text.startswith(";;", start + piece_start):
            return start + piece_start, ""
    if unclosed.startswith("<<"):
        return -1, (f"the here-document `{unclosed}` opens a body that never "
                    f"meets its delimiter, so everything after it is read as "
                    f"that body and this parser cannot tell which `;;` ends "
                    f"the arm")
    if unclosed:
        return -1, (f"a {unclosed} span is opened and never closed, so this "
                    f"parser cannot tell which `;;` ends the arm")
    return -1, "the arm reaches the end of the region with no `;;` terminator"


def shell_words(text: str) -> list[str]:
    """`text` split into shell WORDS, with quotes removed.

    The reader `bin/ocelli.sh`'s `GATES=( ... )` array needs, and it is the
    same scanner rather than a fourth grammar. A word ends where `WORD_BREAK`
    says a word ends, which is at a blank, a newline or the first character of
    an operator, and its value is `_word_value`'s, which a here-document
    delimiter also uses.

    The operator half of that break arrived in the S03 review's twelfth
    pass with the delimiter production, and it cannot change what this reads
    out of the array today: MEASURED, `GATES=( a;b )`, `GATES=( a|b )` and
    `GATES=( a>b )` are each a SYNTAX ERROR to bash, so an unquoted operator in
    an entry is a file bash will not run, and every entry in the runner's array
    carries its own operators inside a quote.

    **That sentence quoted a COUNT until the S03 review's thirteenth pass, and
    the count was wrong.** It said "the runner's twenty-nine entries" where
    `declared_gates` returns 28 and `ci/guard-probe-budget.json` records
    `gates_declared` 28, in a file that legislates against exactly this: a
    quoted number is a copy, and a copy of a measurement goes stale. The count
    is not restated here because nothing here needs one. The number a reader
    wants is printed by the check and recorded in the budget, and the census
    compares those two.
    """
    words: list[str] = []
    current: list[tuple[int, int, str]] = []
    pieces, _ = shell_pieces(text)
    for piece in pieces:
        start, end, kind = piece
        if kind in (COMMENT, BODY):
            continue
        if kind == JOIN:
            # A continuation is removed and neither breaks a word nor begins
            # one, so it joins a word already open and is dropped otherwise.
            if current:
                current.append(piece)
            continue
        if kind == TEXT and end - start == 1 and text[start] in WORD_BREAK:
            if current:
                words.append(_word_value(text, current))
                current = []
            continue
        current.append(piece)
    if current:
        words.append(_word_value(text, current))
    return words


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
#
# A BACKTICK is in the character class since the S03 review's tenth pass, and
# it was the fourth fully fail-open terminator shape. `$(case ...)` was caught
# only because `(` happens to be in the class, and `` `case ...` `` was caught
# by nothing: a backtick is not `^`, not one of `[\n;{}()&|]` and not a
# `SHELL_INTRODUCERS` word. MEASURED on the one-line `fmt` arm rewritten as
# `fmt) cargo fmt --all --check && test -n `case x in *) echo y ;; esac` &&
# python3 scripts/prose_check.py --extra ;;`, which `bash -n` accepts: this
# check exited 0, the arm came out as one command, `unseen['fmt']` was `None`
# and bash really runs the dropped command. `shell_pieces` treats the backtick
# as a span as well, so the two halves of the fix agree: the substitution's
# body cannot end the arm and cannot hide a `case` from this pattern.
_WORD_INTRODUCERS = sorted(w for w in SHELL_INTRODUCERS if w[:1].isalpha())
_PUNCT_INTRODUCERS = sorted(w for w in SHELL_INTRODUCERS if not w[:1].isalpha())
NESTED_CASE = re.compile(
    r"(?:^|[\n;{}()&|`]|(?:"
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
#
# **`&` was ABSENT until the S03 review's thirteenth pass, and it is passes 6,
# 7 and 8 with a different operator.** bash's `&` terminates a list exactly as
# `;` does, this pattern carried `&&` and no `&`, so `A & B` was ONE statement
# whose head is `A`, and a head that is a `COMMAND_PREFIXES` prefix or a
# `SHELL_NOISE` builtin took `B` out of `unseen_commands` with it. MEASURED in
# a real clone: the `&&` before `node --test` in the `bench` arm rewritten as
# `&` on one line, and the `- run: bin/ocelli.sh gate bench` step replaced by
# the arm's two extractable `python3` commands, gave `bash -n` 0 and this check
# exit 0 printing "every command in each gate's arm", with `bench` gone from
# the named-only list and six node suites out of CI. The bash oracle disagrees
# on the same body: `case g in g) echo M1 & echo M2 ;; esac` prints both
# markers, and `scripts/tests/test_guard_readers.py` runs that comparison now.
#
# **The `&` is NOT a bare alternative, and the naive spelling was measured to
# refuse a legitimate state.** `&` is also the second character of `>&` and
# `<&` and the first of `&>` and `&>>`, none of which separates anything, and
# the `panic` arm really contains `echo "wasm-pack is not installed. See
# docs/DEVELOPER_SETUP.md" >&2`. MEASURED with `r"[\n;{}()]|&&|\|\||\||&"`:
# `unseen_commands` grew the entry `unseen['panic'] = ['2', ...]`, a file
# descriptor reported as a command CI does not run. It cost no exit code today
# only because `panic` already holds unseen commands and CI names the gate, so
# an arm whose only unextractable text was a redirection would have refused.
# The lookbehind and the lookahead are what a redirection operator is, and
# `&&` stays ahead of `&` so the two-character control operator still wins.
STATEMENT_BREAK = re.compile(r"[\n;{}()]|&&|\|\||\||(?<![<>])&(?!>)")

# The separators that end an AND-OR LIST rather than continuing one, which is
# the distinction `_statement_separators`' callers need and `STATEMENT_BREAK`
# does not draw. Everything not here, so `&&`, `||` and `|`, joins the
# statement before it to the statement after it into one list whose exit
# status is decided by ONE of its members. See `_tolerated_statements`.
LIST_CONTINUATIONS = frozenset({"&&", "||", "|"})


def _statement_separators(text: str) -> list[tuple[str, str]]:
    """`text` as (statement, the separator that FOLLOWED it) pairs.

    The separator is "" for the last statement, which nothing followed. It is
    kept because two callers need it and neither may re-scan the text to find
    it: `_split_statements` throws it away, and `_tolerated_statements` asks
    which statements' failures a following `||` or `|` swallows.

    A separator inside a span does not separate anything, and it is the same
    scanner that says so here, in `_arm_end` and in `shell_source`.
    A `$( ... )` is one piece, so the `(` and `)` that delimit it are not the
    `(` and `)` of `STATEMENT_BREAK`, which is what a subshell writes.
    """
    statements: list[tuple[str, str]] = []
    current: list[str] = []
    pieces, _ = shell_pieces(text)
    index = 0
    for start, end, kind in pieces:
        if start < index:
            continue
        if kind in (COMMENT, JOIN, BODY):
            continue
        if kind == SPAN:
            current.append(text[start:end])
            continue
        separator = STATEMENT_BREAK.match(text, start)
        if separator is not None:
            statements.append(("".join(current), separator.group(0)))
            current = []
            index = separator.end()
            continue
        current.append(text[start:end])
    statements.append(("".join(current), ""))
    return statements


def _split_statements(text: str) -> list[str]:
    """`text` split on `STATEMENT_BREAK`, spans left whole."""
    return [statement for statement, _ in _statement_separators(text)]


# The heads of an errexit-EXEMPT context, taken from `SHELL_INTRODUCERS`
# rather than written out a second time beside it, which is the repair the
# `case` alternation got in the ninth pass and this file's standing answer to
# one list with two spellings.
#
# MEASURED with `bash -ec <body>`, exit status read from bash. Every row here
# is a 0 where the obvious reading of the errexit paragraph says 1:
#
#   set +e / false / echo AFTER                                       -> 0
#   set +o errexit / false / echo AFTER                               -> 0
#   if false; then echo T; fi / echo AFTER                            -> 0
#   if false; then echo A; elif false; then echo T; fi / echo AFTER   -> 0
#   if true && false; then echo T; fi / echo AFTER                    -> 0
#   while false; do echo T; done / echo AFTER                         -> 0
#   until true; do echo T; done / echo AFTER                          -> 0
#   ! false / echo AFTER                                              -> 0
#
# and these are the CONTROLS, which must stay 1 or this would be over-refusing
# rather than reading the shell:
#
#   false / echo AFTER                                                -> 1
#   if true; then false; fi / echo AFTER                              -> 1
#   while [ -z "$X" ]; do false; break; done / echo AFTER             -> 1
#
# So a `then`, an `else` and a `do` END the exemption and the BODY after one
# is not exempt. That is the whole reason this is an extent and not a head
# test, and the third row above is the other half of it: `if true &&
# bin/ocelli.sh gate x` puts the runner in the SECOND statement, whose head is
# the runner rather than `if`.
# The shell this file MODELS, stated as what it refuses rather than as what it
# handles.
#
# **This block replaced a model of bash's compound-command grammar, and the
# reason is four review passes long.** Pass 14 replaced a table of ten measured
# examples with a generated oracle. Pass 15 found the oracle's GRAMMAR was
# missing six productions, three of them total bypasses where bash never runs
# the gate, and added a nesting rule. Pass 16 found the nesting rule was a HEAD
# test, and the repair for that was a keyword regex, and the regex was missing
# the most basic fact about bash's grammar: **a reserved word is reserved only
# in command-word position.** `echo done` matched it. One ordinary line of
# output turned a correct refusal into a pass, on `guards`, the gate that
# watches every other gate.
#
# Three readers, three passes, each closing the previous spelling and opening a
# new one at the same layer. So this file stopped modelling. A `run:` body that
# carries a construct below is REFUSED BY NAME, which makes an unmodelled
# construct a named refusal a maintainer can act on rather than a silent pass.
# It is a whitelist, so the next construct nobody thought of fails CLOSED
# instead of being counted.
#
# MEASURED cost, which is why this is affordable: `.github/workflows/ci.yml`
# carries 0 compound statements, 0 banned heads and 0 grouping separators in
# its `run:` bodies today. The `run_gate` arms in `bin/ocelli.sh` are NOT
# affected, since `unseen_commands` reads those and is a different consumer.
#
# `in` is deliberately absent: every construct that uses it is already refused
# through `for`, `case` or `select`, so listing it would add false refusals on
# an extremely common English word and no coverage. `time` is absent for the
# same reason, `time <compound>` being refused through the compound itself,
# and `time <command>` running the command normally.
#
# **The declared cost, which is the mirror of the defect this replaced.** A
# reserved word is matched ANYWHERE outside a span, including in argument
# position, so `echo done` is refused even though bash reads it as two
# ordinary words. That is deliberate. Deciding whether a word sits in
# command-word position is precisely the judgement three successive readers
# got wrong, and a whitelist that guesses it would be the fourth. Over
# refusing is a named refusal a maintainer fixes by quoting the word, and
# `echo "done"` passes, because a span is masked before the scan. Under
# refusing is a gate reported as run that never ran. MEASURED: the real
# `.github/workflows/ci.yml` carries 0 bare reserved words in its `run:`
# bodies.
_RESERVED_WORDS = (
    "case", "coproc", "do", "done", "elif", "else", "esac", "fi", "for",
    "function", "if", "select", "then", "until", "while",
)
_RESERVED_WORD = re.compile(
    r"(?<![\w-])(" + "|".join(_RESERVED_WORDS) + r")(?![\w-])")

# Heads whose effect on the rest of the body this file does not model. `set`
# and `shopt` both write the `set -o` option set, so both can turn errexit off,
# and their SCOPE depends on whether a subshell or a function frame lies
# between. `exit` and `exec` end the script. Each was a measured fail-open.
_UNMODELLED_HEADS = frozenset({"exec", "exit", "set", "shopt"})

# Grouping separators. A `(` subshell or a `{` group is a compound command,
# and `f() {` is a function definition, which is how a gate hides in a body
# that never calls it.
_GROUPING = frozenset({"{", "}", "(", ")"})


def _refuse_unmodelled_shell(source: str, where: str) -> None:
    """Refuse a `run:` body carrying shell this file does not model.

    Named by construct, so a probe can tell one refusal from another and a
    maintainer is told which word to remove rather than which gate went
    missing.
    """
    pieces, _ = shell_pieces(source)
    outside = "".join(
        source[start:end] if kind == TEXT else " " * (end - start)
        for start, end, kind in pieces)
    found = _RESERVED_WORD.search(outside)
    carried = f"the shell keyword `{found.group(1)}`" if found else ""
    if not carried:
        for statement, separator in _statement_separators(source):
            head = statement.strip().split(" ", 1)[0]
            if head in _UNMODELLED_HEADS:
                carried = f"a `{head}`"
                break
            if head == "!":
                carried = "a `!` negation"
                break
            if separator in _GROUPING:
                carried = f"a `{separator}`"
                break
    if not carried:
        return
    raise RuntimeError(
        f"the `run:` on {where} carries {carried}, and this check does not "
        f"model it. It answers whether CI is GUARANTEED to run each floor "
        f"gate and report its failure, and every construct listed in "
        f"`_RESERVED_WORDS`, `_UNMODELLED_HEADS` and `_GROUPING` can break "
        f"that guarantee in a way three successive readers of bash's grammar "
        f"each got wrong. A gate inside a body the shell never reaches, or "
        f"after a `set +e`, or after an `exit`, still READS as an invocation. "
        f"So it is refused by name rather than modelled: write the gate as one "
        f"command per statement at the top level of the body, or add the "
        f"construct here together with the `bash -ec` measurements that say "
        f"what it does.")


def _tolerated_statements(text: str) -> dict[int, str]:
    """Which statements' FAILURE the shell discards, and why, by index.

    Keyed by the index in `_statement_separators(text)`, valued with the reason
    a refusal can print. A reason rather than a bare set, because the four
    shapes below are four different edits to undo and "swallowed" names none of
    them.

    GitHub Actions runs a `run:` body as `bash -e {0}`, and every rule below is
    a MEASUREMENT of that shell rather than a reading of the errexit paragraph,
    because the obvious reading of that paragraph is wrong. The scripts are run
    as `bash -ec <body>` and the exit status is read from bash itself.

    - `false\\necho AFTER` exits 1. A simple command's failure fires errexit.
    - `false && true\\necho AFTER` exits 0, and so does `false && false\\necho
      AFTER`. errexit is suppressed for a command in an AND-OR list, and the
      short circuit means the command after the final `&&` never runs, so
      nothing fires it and the list's status is discarded. **A failing left
      side of `&&` is therefore swallowed. The right side is skipped, so it
      cannot prove a gate ran either. Both lose their status when a later list
      succeeds.**
    - `false && true` ALONE exits 1. The last list in the body decides the
      script's status, so the same statement is not swallowed there.
    - `false || false\\necho AFTER` exits 1 and `true | false\\necho AFTER`
      exits 1. The last member of a list or pipeline runs, and errexit fires
      on it.
    - `false &\\necho AFTER\\nwait` exits 0. An asynchronous command reports 0
      at once, so `&` swallows unconditionally.

    So a failure reaches the step exactly when the statement is the last member
    of its AND-OR list or pipeline and is not backgrounded, or when its list is
    the LAST one in the body and nothing to its right can overwrite the status,
    which a `||` after it and a `|` immediately after it both can.

    `.github/workflows/ci.yml` runs `sudo apt-get update && sudo apt-get
    install -y dcmtk` today, and this reports the left side as swallowed. That
    is not a false refusal, it is bash: if the update fails, the step is green.
    It costs no verdict because neither half is a gate command, and the day one
    is, the refusal will be right.
    """
    pairs = _statement_separators(text)
    segments: list[list[int]] = []
    current: list[int] = []
    for index, (_, separator) in enumerate(pairs):
        current.append(index)
        if separator not in LIST_CONTINUATIONS:
            segments.append(current)
            current = []
    if current:
        segments.append(current)
    # The last list that carries a command. A body normally ends in a newline,
    # so the last pair is an empty statement and its own segment, and reading
    # THAT as the final list would swallow the real final list with it.
    last = max((index for index, (statement, _) in enumerate(pairs)
                if statement.strip()), default=-1)
    tolerated: dict[int, str] = {}
    for segment in segments:
        final = last in segment
        for position, index in enumerate(segment):
            separator = pairs[index][1]
            if separator == "&":
                tolerated[index] = ("it is backgrounded with `&`, so the shell "
                                    "reports 0 for it at once")
                continue
            if separator == "|":
                tolerated[index] = ("it is a non-final member of a pipeline, "
                                    "whose status is its last member's")
                continue
            if position > 0 and pairs[segment[position - 1]][1] == "||":
                # It runs ONLY when the left side FAILED, so its presence is
                # not evidence that CI runs this gate. MEASURED: `true ||
                # false` exits 0 because the right side never ran, so `true ||
                # bin/ocelli.sh gate guards` satisfied `guards` at exit 0 with
                # the gate never executed. Found by the generated-input bash
                # oracle. The right side of `&&` is handled below because its
                # skipped status is safe only when the AND-list ends the body.
                tolerated[index] = ("a `||` before it means it runs only when "
                                    "the left-hand side failed, so nothing "
                                    "here says the shell runs it at all")
                continue
            if (not final and any(pairs[i][1] == "&&"
                                  for i in segment[:position])):
                tolerated[index] = (
                    "an earlier `&&` means it runs only when the left-hand "
                    "side succeeds, and a later statement can leave the step "
                    "green when it does not run")
                continue
            if position == len(segment) - 1:
                continue
            after = [pairs[i][1] for i in segment[position:]]
            if "||" in after:
                tolerated[index] = ("a `||` after it runs the right-hand side "
                                    "instead, and that side's status is what "
                                    "the shell reports")
            elif not final:
                tolerated[index] = ("it is a non-final command of an AND-OR "
                                    "list that is not the last list in the "
                                    "body, and `bash -e` discards that")
    # No context rules any more. Everything a context rule used to answer is
    # refused outright by `_refuse_unmodelled_shell`, so what remains here is
    # the SEPARATOR reading, which is about a flat list of commands and is the
    # only shape this file now accepts.
    return tolerated


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
    """Each gate's `run_gate` arm body, as `shell_source` leaves the region.

    An arm ends at its `;;`. Nothing here can read a command out of prose. See
    the docstring's "Where an arm ENDS" section for what that cost.

    **Comments and continuations are ONE call since the S03 review's
    twelfth pass, and the continuation was a regex pre-pass over shell
    before it.** `CONTINUATION.sub(" ", region)` ran before the tokenizer,
    which is the one thing the tokenizer's own header argues no pass may do,
    and it joined the line after a COMMENT ending in a backslash into that
    comment. bash joins nothing there. What that cost is measured at `JOIN`.

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
    body = shell_source(region)
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
        # that sat before the backslash beside the whole of the next line's
        # indentation, and `runs_command` compares the text, so the run of
        # blanks would make an arm command CI runs verbatim look absent.
        #
        # A here-document BODY is still in `text` here, because `shell_source`
        # has to return something `_arm_end` can re-scan and a source with the
        # body cut out declares a here-document whose terminator is missing.
        # So a body carrying one of `COMMAND_PREFIXES` yields a command this
        # file then demands of CI. That is fail-CLOSED, it can only ADD a
        # demand, and the refusal names the text, which is a message a
        # maintainer can act on. `unseen_commands` does not see it: the body
        # is its own piece kind and `_split_statements` drops it.
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
    #
    # The workflow's readers are inside this block since the S03 review's
    # twelfth pass, and they were outside it while they could not fail. They
    # can now: `.github/workflows/ci.yml` is PARSED, so a file that is not
    # YAML, or whose shape this reader does not model, is a refusal here
    # exactly as an unreadable `run_gate` region already was, and it arrives
    # under the same header rather than as a traceback.
    try:
        arms = gate_commands(runner)
        unseen = unseen_commands(runner)
        declared = declared_gates(runner)
        excluded = runner_excluded(runner)
        entry_problems = gate_row_problems(runner)
        commands = run_commands(workflow)
        every_event = workflow_events(workflow)
    except RuntimeError as error:
        print("FAIL: CI does not run the whole floor")
        print(f"  {error}")
        return 1
    events = every_event - MANUAL_EVENTS

    # An entry the gate reader cannot use, FIRST, because every rule below is
    # about the gates it did read and none of them can say anything about one
    # it dropped. That was measured at exit 0 with a gate running.
    problems = list(entry_problems)
    if not events:
        # The declared keys are named because `on` is read as a string key
        # here, so a workflow whose event block is written as YAML 1.1's
        # boolean spelling `true:` reaches this branch, and a message saying
        # only "no automatic event" would send its author looking at the
        # events rather than at the key. That is the declared cost of
        # `BaseLoader` and this is where it is paid.
        top_level = sorted(workflow_tree(workflow))
        problems.append(
            f"{WORKFLOW.relative_to(ROOT)} declares no automatic event, so "
            f"there is no event on which the floor could be said to run. "
            f"`--floor` claims to be what CI runs, and a workflow only a "
            f"person can start does not run on a pull request. The top-level "
            f"keys this parser read are "
            f"{', '.join(repr(key) for key in top_level) or 'none'}, and the "
            f"event block is the one spelled `on`.")

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
        touching = steps_running(gate, arms, commands)
        running = [c for c in touching if not c.tolerated]
        # The step is in the file and mentions the gate, but does not prove the
        # gate runs and reports failure. Without this the refusal below fires on
        # `blocked` and quote an empty condition, sending its reader to look
        # for an `if:` that is not there. `Command.tolerated` records why, and
        # the three shapes it distinguishes are the thirteenth pass's three
        # measured plants.
        tolerated = sorted({command.tolerated for command in touching
                            if command.tolerated})
        note = ""
        if tolerated:
            note = (f" A step in {WORKFLOW.relative_to(ROOT)} mentions this "
                    f"gate and is not counted, because it is not guaranteed "
                    f"to run it and report its failure: "
                    f"{'; '.join(tolerated)}. `--floor` claims to be what CI "
                    f"runs, and a skipped check or a discarded red is not a "
                    f"check CI runs.")
        # Per event, not per step. Two steps with complementary conditions
        # cover the floor between them, and asking one step to cover every
        # event refuses that arrangement while naming no missing event, which
        # is a message that cannot be acted on.
        #
        # Per event AND over the whole arm. `blocked` is the events no step
        # touching the gate reaches at all, `partial` is the events a step
        # reaches while leaving a command in the arm unrun.
        #
        # `unnamed` is the third: the events on which a gate that cannot be
        # reconstructed command by command is not invoked by name. That is an
        # arm with an unextractable command, or an arm with several visible
        # commands whose ordering and job boundary direct argv equality does
        # not prove. Asked per event for the same reason as the other two.
        blocked: list[str] = []
        partial: dict[str, list[str]] = {}
        unnamed: list[str] = []
        for event in sorted(events):
            reachable = [c for c in commands if c.runs_on({event})]
            if not any(c in running for c in reachable):
                blocked.append(event)
                continue
            name_required = (bool(unseen.get(gate))
                             or len(arms.get(gate, [])) > 1)
            if name_required and gate not in invoked_gates(reachable):
                unnamed.append(event)
                continue
            if covers(gate, arms, reachable):
                continue
            partial[event] = missing_arm_commands(gate, arms, reachable)
        if events and running and not blocked and not partial and not unnamed:
            continue
        if running and events and (blocked or partial or unnamed):
            if unnamed:
                if len(arms.get(gate, [])) > 1:
                    message = (
                        f"the `{gate}` gate is in the CI floor, its arm in "
                        f"bin/ocelli.sh runs several executable commands, and "
                        f"no step in {WORKFLOW.relative_to(ROOT)} invokes "
                        f"`bin/ocelli.sh gate {gate}` on "
                        f"{', '.join(unnamed)}. Exact direct argv matches can "
                        f"show that each command exists, but cannot show they "
                        f"remain in the arm's order, in one job, with its "
                        f"`&&` exit semantics. A visible multi-command floor "
                        f"arm must be invoked by name. Either restore that "
                        f"gate-name step, or exclude the gate from the floor "
                        f"in bin/ocelli.sh and say why.")
                else:
                    message = (
                        f"the `{gate}` gate is in the CI floor, its arm in "
                        f"bin/ocelli.sh runs "
                        f"{', '.join(repr(c) for c in unseen[gate])}, and no "
                        f"step in {WORKFLOW.relative_to(ROOT)} invokes "
                        f"`bin/ocelli.sh gate {gate}` on "
                        f"{', '.join(unnamed)}. This file's command extractor "
                        f"recognises "
                        f"{', '.join(repr(p) for p in COMMAND_PREFIXES)} and "
                        f"nothing else, so it cannot demand those commands "
                        f"step by step and must not report the arm covered "
                        f"without them. A step naming the gate runs the arm "
                        f"entire by definition, and that is the only form "
                        f"this check can accept here. Either restore the "
                        f"gate-name step, or exclude the gate from the floor "
                        f"in bin/ocelli.sh and say why.")
                problems.append(message + note)
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
                    f"the arm entire." + note)
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
                    f"to be true." + note)
            continue
        problems.append(
            f"the `{gate}` gate is in the CI floor and nothing in "
            f"{WORKFLOW.relative_to(ROOT)} runs it. Either add a step, or "
            f"exclude it from the floor in bin/ocelli.sh and say why. "
            f"`--floor` claims to be what CI runs, and that claim has to be "
            f"true." + note)

    # The gates outside the floor. `oracle` needs a GPU and D-04 says CI has
    # none, so it is the one gate CI may not run. Every other excluded gate is
    # excluded for cost or for the corpus, and CI is still supposed to run it
    # somewhere. Nothing asserted that until the S03 review's fourth pass
    # deleted the whole `guards-deep` job and watched this exit 0.
    #
    # `every_event` is read above, MANUAL ONES INCLUDED, because a manual
    # dispatch is exactly what `guards-deep` is on. The floor's own claim is
    # about a change and stays narrower.
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
        tolerated = sorted({command.tolerated for command in running
                            if command.tolerated})
        note = ""
        if tolerated:
            note = (f" A step mentions it and is not counted, because it is "
                    f"not guaranteed to run and report its failure: "
                    f"{'; '.join(tolerated)}.")
        problems.append(
            f"the `{gate}` gate is excluded from the floor and needs no GPU, "
            f"so CI is still supposed to run it, and no step in "
            f"{WORKFLOW.relative_to(ROOT)} runs it on any event the workflow "
            f"declares. Being out of the floor means it does not run on every "
            f"pull request. It does not mean it runs nowhere, and a gate that "
            f"runs nowhere is a gate whose refusals nobody has watched. Add a "
            f"step, or mark the gate as needing a GPU in bin/ocelli.sh's GATES "
            f"table and say why in a deviation." + note)

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
