# The guard harness

**F-IDs that contributed:** F-X009
**Last updated:** 2026-09-06

A guard is any refusal this repository can produce: a script that exits 1, a
hook that rejects a commit, a gate that fails.
`docs/runbooks/guard-verification.md` says why they need watching, in a
sentence this file exists to mechanise:

> A guard that exits zero on a clean tree looks identical to a guard whose
> detection is broken, and the two stay indistinguishable until the day the
> guard was supposed to catch something.

The runbook is a procedure a person follows. This is the standing version of
it. `docs/hld/24-agent-code-standards.md` section 27.3's third bullet asks a
human to "mutate one constant, re-run, confirm it goes red", and for repository
guards that bullet is now a gate rather than a habit. Section 27.3's other
bullets, and its application to LUT and geometry arithmetic, remain a human's
exactly as written.

**No count of a quantity this harness measures appears in this file.** A number
written into prose about the size of the harness goes stale inside a single
sprint, because the harness moves and the sentence beside it does not. Every
such quantity is named by the command that prints it instead. Counts of things
this harness does NOT measure are fine and there are about twenty of them
below, four cleanup layers and five tripwire reads among them. The earlier
wording said "no count appears in this file" and about twenty followed it, all
of them true, which made the rule read as broken when it was only worded too
widely.

## Four pieces that check each other

| Piece | File | Answers |
|-------|------|---------|
| Discovery | `scripts/guards/discover.py` | what refusals exist, found mechanically |
| Declaration | `scripts/guards/catalogue.py` | what each refusal is FOR, and how to drive it red |
| Probe | `scripts/guard_probe.py` | does it still refuse, in a disposable repository |
| Census | `scripts/guards/census.py`, run by `scripts/guard_census.py` | is the declaration complete, and has any guard been widened |

The pairing is the design. A probe run over a catalogue that has fallen behind
is a green answer to a question nobody asked, so the census refuses in both
directions. A census over a catalogue nobody executes is a list, so the probe
runner drives each entry red for its declared reason.

## Discovery, and what a refusal site is

`discover.py` finds refusal sites by shape rather than by a list somebody
maintains. The shapes are `problems.append(`, `problems += [`,
`problems.extend(`, `return ["..."]`, `print("FAIL...`,
`sys.exit("...")`, `raise SystemExit(` or `raise SomethingError(`,
`throw new Error(`, `echo "FAIL...` and a bare `exit 1`. The three list shapes
arrived in the S03 review's sixth pass, and the eight refusals they were
missing included `scripts/pin_and_size_check.py`'s wasm size ceiling, which is
HLD Appendix A gate A4's number. The scan roots are
`scripts/`, `ci/`, `.githooks/`, `bin/` and `tools/`, over `.py`, `.mjs`, `.js`
and `.sh`. Generated, vendored and test-fixture directories are excluded by
`SCAN_EXCLUDE`, because a test suite's own assertions are not guards.

**`crates/` is deliberately absent**, which is decision 7 of
`.claude/plans/F-X009-design.md`: a runtime refusal inside a crate is that
crate's story's test, not this harness's. The census prints that boundary on
every green run rather than leaving a reader to infer that the number covers
the whole repository.

**A site's identity is its file plus the normalised words of its message, and
deliberately not `file:line`.** Moving a refusal within a file must not churn
the catalogue, and rewording a message must force a re-read of what the refusal
is for. That is the trade `oracle.md` already records for the fault
catalogue's `expect` fragments.

That identity costs something and the cost is reported rather than assumed
away. Two refusals in one file whose messages normalise alike are one site, so
a probe on either reads as covering both. `sites_collapsed()` counts them and
the census prints the number.

Python docstrings and comments are blanked before the scan, preserving byte
offsets. The shape table in `discover.py`'s own docstring is markdown rows
quoting refusal shapes, and scanned as code they were refusal sites, so
deleting documentation turned the gate red.

## The catalogue

`scripts/guards/catalogue.py` is production data rather than test data. It is
Python and not JSON so an entry can carry a callable that builds a rejected
state, and it lives under `scripts/` and not under `docs/` because several
entries have to produce the exact text `prose_check.py` and `deviation_check.py`
refuse.

A `Guard` carries an id, the file it is about, the gate that runs it, and:

- **`spec`**, the normative citation, and **`refuses`**, one sentence written
  FROM it. **A probe's input is derived from `refuses`, never from the guard's
  source.** That is HLD 27.2 R2 applied to a guard: a probe written from the
  code asserts the current regex and passes forever once the regex has been
  weakened to match.
- **`claims`**, the regexes that bind the entry to discovered sites. `"*"` is a
  catch-all taking the sites in its file no explicit claim already owns, so two
  entries can split a file and every site still ends with exactly one owner.
- **`covered_by`**, for an entry with no probe here, naming a standing test
  elsewhere.
- **`kind`**, either `guard` or `not-a-guard`. The second is for a file whose
  refusals no gate runs. Counting those as guards would inflate the coverage
  number, and omitting them would leave sites no entry claims.
- **`limit`**, `reason` and `owner`, for what a probe does not reach.
- **`silent`**, for a refusal whose mechanism is the absence of output.

A `Probe` carries the mutation, the invoke, and:

- **`expect`**, which IS allowed to be the implementation's own words, and that
  is not a contradiction of the rule above. Its only job is to prove the run
  went red at the declared refusal rather than by accident.
- **`level`**. Level 3 runs the guard's real argv with the sandbox as cwd.
  Level 1 calls the detection function with an in-memory input, for a rule that
  has no argv of its own.
- **`polarity`**. `refuse` means the guard must exit non-zero. `accept` means
  it must exit zero, which is the false-positive direction: a guard that fails
  on everything is as useless as one that fails on nothing.
- **`profile`**, `floor` or `deep`, and **`needs`**, which the census cross
  checks.
- **`defect`**, naming a hole this repository has declared rather than fixed.
- **`control`**, **`control_status`** and **`control_expect`**.

`python3 scripts/guard_probe.py --list` prints every probe with its profile,
level, polarity, `needs`, guard and any declared defect, and the `note` field
carrying the rationale for its input. `needs` reached that output only in the
S03 review's fifth pass, and until then this file and
`.github/workflows/ci.yml` both sent a reader to the command for it and the
command did not answer.

### Known defects

`DEFECTS` at the foot of the catalogue holds each declared hole in full, so the
count lives in one place and no sentence can go stale against it. A
known-defect probe that FAILS is reported and does not fail the gate. **A
known-defect probe that PASSES fails the gate**, because the hole was fixed and
the declaration is now a lie. The ratchet points in the direction that matters.
`python3 scripts/guard_census.py` lists the open ones.

## The sandbox, and why it cannot write to your repository

`scripts/guards/sandbox.py` builds a disposable git repository per run.
**Nothing here may write inside the real repository**, and that is a guarantee
made by construction rather than by cleanup.

1. `tempfile.mkdtemp(prefix="ocelli-guard-")`
2. copy the WORKING TREE content of every path in `git ls-files`, mode bit kept
3. in the copy, `git init`, `git add -A`, `git commit`
4. every git call runs with a scrubbed environment

**A fresh repository, not a worktree.** The runbook's `git worktree add
--detach` is right for a person doing this once and wrong for a standing
harness, for two reasons the runbook itself records: a worktree is a full
checkout, so every path-scanning guard sees a second copy of the repository,
and `git worktree add` writes into the developer's `.git`.

**Working-tree content and not `HEAD`, deliberately.** A developer editing a
guard has to see their edit probed before they commit. Building from `HEAD`
would probe the previous version and report success.

Using `ls-files` rather than a directory walk also excludes `corpus/data`,
`node_modules`, `target`, `tools/oracle/out` and `.claude/verify-ledger.json`
by construction, so no patient data, no rendered reference frame and no local
evidence file is ever copied anywhere. A tracked path that is not a regular
file is refused rather than skipped, because a silent divergence between the
copy and the original is the one thing the control run assumes away.

### The safety argument, and where a reviewer checks it

**`repo_read` is the only function that runs git against the real repository.**
`cwd=REPO_ROOT` appears once in the file. Two other functions name `REPO_ROOT`
at all, and neither of them writes: the choke point names it to refuse a cwd
inside or above it, and `build` names it to read a tracked file out of it.
`repo_read` refuses every write verb by name, and it treats
`config` as a read only in its `--get` family of forms, because
`git config core.hooksPath X` in `REPO_ROOT` would rewrite the developer's
`.git/config` and the tripwire needs `config --get`.

`Sandbox.git` is the one path to a mutation and it carries four refusals:

- a cwd inside or above the real repository
- a directory this harness did not create, by the `ocelli-guard-` name
- a handle pointing at a directory with no `.git`
- `git rm --cached`, banned by name. Followed by `git checkout --` it leaves
  the path untracked with the broken content in place, so the very guard being
  probed then skips it and reports clean. That is a false green produced by the
  cleanup rather than by the guard. `reset()` uses `reset --hard` to the
  sandbox's own base commit and `clean -fdx`.

`scrubbed_env` removes every inherited `GIT_*` variable, points the global and
system config at `/dev/null`, and fixes the commit identity to
`Ocelli guard harness`, so a probe commit works on a machine with no git
identity and never carries the developer's name.

Cleanup has four layers: the `finally` in the context manager, SIGINT and
SIGTERM handlers that remove and re-raise, `sweep_stale` on the next run for a
`kill -9`, and the fact that no git call writes inside `REPO_ROOT`.

The fourth layer is narrower than it sounds, and the narrowing is why both
entry points set `sys.dont_write_bytecode` before their first import of
`guards.*`. `scrubbed_env` sets `PYTHONDONTWRITEBYTECODE` for CHILDREN, and the
PARENT is the process that imports those modules and builds every probe's
rejected state from them, so without the flag a run writes
`scripts/guards/__pycache__` inside the repository. `.gitignore` covers it, so
the tripwire cannot see it either, and a stale `.pyc` is capable of making the
harness build a rejected state that does not match its source. The ordering of
those two lines is the mechanism, not a style choice.

### The tripwire

`TRIPWIRE_READS` captures the real repository's `HEAD`, unstaged names, staged
names, untracked names and `core.hooksPath` before the run and again after it,
and a difference fails the run as a defect in the harness rather than in a
guard. The reads are content-level and not stat-level, because a stat-level
tripwire fires on `git status` legitimately refreshing the index and a tripwire
tuned away within a week is not a tripwire.

## The probe runner

`python3 scripts/guard_probe.py --profile floor` and `--profile deep`.

**Inverted success.** A probe whose guard exits 0 is a FAILURE OF THE HARNESS,
not a pass. A probe builder that silently stops mutating anything therefore
turns the run red rather than green, and `Sandbox.substitute` refuses a no-op
edit by name at the point of the edit.

**The mandatory control.** Every distinct control is run against the UNMUTATED
sandbox and must exit with the status its probe declares. That status is
usually 0, and for two probes it is deliberately not: `split_hld` and
`corpus-tests` have no healthy state a sandbox can build, so their control is
the DIFFERENT refusal a healthy repository gives, declared as
`control_status` and `control_expect`. The paragraph below says the same thing
and this one used to contradict it by claiming every invoke must exit 0.

The control's absence is what made round 12 of F-010's review worthless: the
harness was broken, so every earlier all-refusals-red result had a red baseline
and proved nothing. The control does two further jobs. It proves the sandbox is
a faithful copy, because a guard refusing an unmutated copy means the copy is
wrong. And when a control's declared status is non-zero it must refuse for a
DIFFERENT reason than the probe, so a probe over a guard that refuses
everything cannot read as a pass.

`_prepare_control` holds the minimum healthy state an invoke needs, written in
one place rather than hidden inside each probe, because a control that quietly
does the probe's job is the false green this file exists to refuse. A few
invokes have no healthy state a sandbox can build at all, because what they
need is per-clone and `git ls-files` never copies it. Those declare a non-zero
`control_status` and the different refusal a healthy repository gives, and
`python3 scripts/guard_probe.py --list` is where to read which they are. An
earlier version of this paragraph said three and the count was two, which is
the failure this file's no-counts rule exists to prevent.

The summary line reports refusal probes, the distinct guards those drove red,
accept probes, open known defects, controls green and the elapsed wall clock.
**Each is a different quantity and reporting one under another's name is the
failure this harness exists to refuse.** In particular a pass count is a PROBE
count and includes the accept probes, which were never red, so it is never the
number of guards observed red.

`python3 scripts/guard_probe.py --self-test` runs the harness's own refusals,
which no gate run otherwise produces. `SELF_TEST_PROPERTIES` names each
property and the printed count is derived from the blocks that actually ran,
because a hardcoded count survives the deletion of the blocks it stands for.
The faithful-copy property compares the sandbox against `git ls-files` by name.
It was a threshold, `copied > 100` over a walk that counts the sandbox's own
`.git`, and a sandbox with every tracked file deleted passed it.

## The census

`python3 scripts/guard_census.py`. The checks are lettered in
`scripts/guards/census.py`'s docstring, and none of them is sufficient alone.

**a. Refusal-site discovery, strict in both directions.** Every discovered site
must be claimed by exactly one entry, so the catalogue cannot fall behind.
Every entry must claim at least one site, so a deleted refusal cannot leave a
stale entry that reads as coverage.

**a2. The per-entry site count, for the gap check a leaves open.** The census
exists so that a guard added next month arrives with its test, and that rule
did not apply to a guard added to a file the catalogue ALREADY claims. Most
entries claim their file with `"*"`, so a new `problems.append` in a claimed
file lands in the probed bucket and no number anywhere moves. The S03 review's
fifth pass measured it from the other side, on this harness's own census
module: the fourth pass had added 224 lines to it, four of them new refusal
branches, and each could be deleted on its own with the census, the floor probe
profile and the unit suite all green.

So the site count of every catch-all entry is recorded in
`ci/guard-probe-budget.json` under `entry_sites` and compared for EQUALITY. A
refusal added to a claimed file, or removed from one, fails until the number is
re-recorded in the same change, which is check c's "put the widening in the
diff" applied to a different loss. It does not claim the new refusal is probed.
It claims a reviewer sees that the file grew one.

Equality and not a ratchet, because a refusal DELETED from a claimed file is
equally invisible: the entry goes on claiming the other sites in its file. The
catalogue was the alternative home for these numbers, one per entry, and it was
rejected. The budget already carries every other recorded value and `--record`
already writes them all in one command, where the same numbers spread through
the catalogue would be that many things to hand-edit. How many there are is
printed by `python3 scripts/guard_census.py --record`, and the two files that
carried the number in prose disagreed with each other and with the harness,
inside the paragraph that added the measurement.

**What that number can see is `scripts/guards/discover.py`'s shape table**, and
the sentence above stood ahead of the mechanism until the S03 review's sixth
pass. A refusal built as a list, `problems += [...]`, `problems.extend(...)` or
`return ["..."]`, was found by no shape, so eight refusals in scanned guard
files moved no number at all. One of them was `scripts/pin_and_size_check.py`'s
wasm size ceiling, which is HLD Appendix A gate A4's own number: deleting it
left the census headline byte-identical and exit 0, and only a probe caught it.
Those three shapes are scanned now. A refusal whose message is assembled into a
local variable before it is appended **is found**, and this sentence said it
was not until the S03 review's seventh pass. `problems.append(` carries no
requirement that its argument be a literal, so `scripts/guard_probe.py:260` and
`:273` and `tools/oracle/check_sidecars.py:558` were all found. What was true is
that each took its IDENTITY from the variable's name, which made the two
inverted-success refusals in `guard_probe.py` ONE site: deleting the first of
them, the refusal that makes "a probe whose guard exits 0 is a FAILURE OF THE
HARNESS" true, and returning `"pass"` instead, left `entry_sites` at 19, the
census at exit 0, `--self-test` at 10 properties, the floor profile at 106
probes red and the unit suite at 49, with the whole `guards` gate ALL GREEN and
the mechanism that gives every probe result its meaning removed. A message
carrying no string literal of its own now falls back to the enclosing function
plus an ordinal, and `discover.py` declares the remaining limits where the scan
is rather than here.

**b. Gate and hook coverage, in both directions.** Every name in
`bin/ocelli.sh`'s `GATES` array has an entry or an explicit `DELEGATED` reason,
every entry's `gate` names a gate that array declares, the number of rows in it
is a ratchet that may only grow, and every executable under `.githooks/` has
entries. A gate nobody declared is a gate nobody probed, and the three gate
rules are one rule seen three ways: the S03 review's fourth pass deleted the
`prose` row and got a green census, a green `ci_floor_check.py` over one gate
fewer, and probes still passing, because a probe invokes
`scripts/prose_check.py` directly rather than through the gate. `--floor`,
`--sprint` and `--all` all shrank and nothing said so. The `GATES` array is
parsed with a copy of `scripts/ci_floor_check.py`'s regex, and `bin/ocelli.sh`
carries a third. Nothing joins the three, which is a duplication this file does
not get to describe away.

**c. The declared-constant ratchet.** The class of weakening no probe can
reach. A probe proves a guard still refuses what it refuses and cannot notice
that the guard's configuration has been WIDENED, because after the widening the
guard is correct about its new, weaker rule. `CONSTANTS` names each
strictness-deciding value with the regex that reads it, and a change fails the
census until the recorded digest in `ci/guard-probe-budget.json` is updated in
the same change. That puts the widening in the diff and in front of a reviewer
rather than in a refactor nobody reads. A recorded value that no longer parses
also fails, because a constant the ratchet cannot read is a ratchet that has
quietly stopped holding.

**The ratchet can also be narrowed, and the number of declared constants is
recorded for exactly that.** Deleting a `Constant` and its recorded row is a
two-line edit that reads as a cleanup, and every other check here stays green
afterwards: the loop over `CONSTANTS` no longer visits it and the loop over the
recorded rows no longer sees it. The S03 review's fourth pass did it to
`pins:TOLERANCE` and the census exited 0. The one mechanism that notices a
guard being widened could therefore be disarmed in one green commit and the
widening land in the next, also green. The count may only grow, so retiring a
constant means saying which and why in the diff that re-records.

**d. The profile rule.** An entry whose probe needs a GPU, a browser or the
corpus may not be in the floor, and neither may one needing cargo, npm or
wasm-pack. `.claude/WORKFLOW.md`'s floor definition as a mechanism rather than
a convention. The rule is written over rows passed in as data so it has a
level-1 probe of its own.

**e. The uncovered ratchet.** The count of refusals belonging to a `guard`
entry with neither a probe nor a `covered_by` may only decrease. A new
uncovered refusal fails the floor, and once the sweep is recorded complete any
non-zero count fails `--profile deep`.

**f. `covered_by` names a FILE that reaches the file.** Each named path must
resolve to a file, and at least one must reach the guarded file: by naming it,
by importing it, in either direction, or by declaring identifiers the guarded
file implements by name. That last shape is the oracle's, where `faults.mjs`
declares the fault ids and the render page implements each one. The check is
cheap and blunt on purpose. It cannot show that the named test drives a
particular refusal red, and nothing claims it does. It exists because one entry
named a suite that never mentions the file it claimed to cover, and its
refusals were counted as watched.

**A directory claim is refused outright, and it took three attempts to get
there.** The first version accepted a directory for its own existence, with no
reachability check, so one line put the same nine refusals back into the
covered bucket. The second resolved a directory to the files in it and asked
whether one of them reached the guarded file, and the fourth review pass
measured that hatch reopened one indirection out: `scripts/guards/catalogue.py`
writes every guarded path as `file="tools/bench/run.mjs"` and so on, so any
directory whose walk reaches the catalogue reaches every entry. `scripts/`,
`scripts/guards/` and `docs/` each gave zero problems for every entry. No entry
claims a directory, so the route is gone rather than narrowed a third time.

### What the coverage number is, and what it is not

**It is an ENTRY-level claim summed over refusal sites.** It says which bucket
a refusal's entry is in. It does not say that this refusal has been driven red.
Only a probe in this harness has been watched fail. `report_lines` prints that
sentence beside the number, because a bare bucket count reads as a per-refusal
claim and is not one.

The green run also names, by entry, every uncovered refusal with its reason and
owner, every declared `limit`, and every open defect. Run
`python3 scripts/guard_census.py` for the current list. Those lines are the
authority. A reader who wants the shape of a limit rather than its text will
find the generated table in `docs/runbooks/guard-verification.md`.

### The oracle's faults are adopted, not re-declared

`tools/oracle/src/faults.mjs` already drives the oracle's own refusals, and a
second declaration of the same faults here would be the same defect as a second
copy of the LUT chain, except that it only runs where nobody looks. So
`oracle_adoption` verifies the real catalogue instead: that it carries faults,
that each names the specific message fragment proving its run went red at its
own boundary, that `tools/oracle/run.mjs` still reaches `tests/faults.mjs`, and
that the fault count has not shrunk below the recorded number. The count is a
ratchet because F-X007 grew it and a silent shrink is exactly what this census
exists to notice. `bin/ocelli.sh gate oracle` runs them for real and this
harness does not re-run them, because that needs a browser. So the census
proves the adoption is still wired, and not that the faults fired.

### The runbook's table is generated

`python3 scripts/guard_census.py --render-runbook` projects the probe table,
the defect list, the declared limits and the out-of-scope entries into
`docs/runbooks/guard-verification.md` between two markers. The prose around the
markers is hand-written and is not generated. A green census also verifies the
table has not drifted, so a guard added without re-rendering fails the gate. A
table of probes drifts the moment a guard is added, which is the defect this
harness exists to fix.

## The two gates

`bin/ocelli.sh gate --list` is the inventory. Two names are this harness's:

- **`guards`**, in the floor. It runs `lint_policy_check.py`, the census, the
  harness self test, the floor probe profile and the catalogue's unit suite.
  Every probe in it runs with no cargo, no npm, no wasm-pack, no browser, no
  corpus and no GPU, and check d refuses an entry that declares otherwise and
  sits in the floor anyway. **The GATE is not under that rule and the S03
  review's seventh pass made the difference visible.** `lint_policy_check.py`
  reads its member set from `cargo metadata --no-deps` now, because
  `[workspace] members` is not the member set and a crate reached as a path
  dependency was measured to be linted by cargo and never walked here. So that
  guard needs cargo, exactly as `no_std_check.py` has always needed it for
  `cargo tree`, and every `lint-policy` probe moved to `needs="cargo"` and the
  deep profile with it. The floor gate still runs the guard on every pull
  request. What moved to push-to-main is the harness watching the guard.
- **`guards-deep`**, not in the floor. `--profile deep` is every probe, so this
  re-runs the floor set and adds the ones that need a toolchain, plus the
  census at `--profile deep`. Every probe it adds declares `needs` `cargo`, all
  41 of them, and none needs npm or wasm-pack, which
  `python3 scripts/guard_probe.py --list` is the place to check. **It runs in
  the `guards` CI job on every event since the S03 review's ninth pass**, and
  it was a separate job gated to push-to-main and dispatch until then. The two
  reasons that gate carried were both false: the first named npm and wasm-pack
  and the eighth pass corrected it, and the second said a runner has to install
  the toolchain, while the `guards` job installs the pinned toolchain,
  `rust-cache`, python and node for `gate guards` itself. The deleted job added
  `targets: wasm32-unknown-unknown` and nothing else, and no deep probe needs
  it. What that cost bought was the wrong way round: every `lint-policy` probe
  is a deep probe, the review has found a route past that guard on every pass
  since the fifth, and those probes were unwatched on the pull request that
  would weaken them. **What is left of the floor exclusion is duplication and
  nothing else.** The deep profile is a strict superset of the floor one, so a
  `gate --floor` including it would run every floor probe twice. MEASURED on
  the development machine, deep 23.8s against floor 15.9s, so about eight
  seconds of the twenty-four is new coverage and about sixteen is the
  duplication CI now pays deliberately. Removing it would mean `gate guards`
  running a different probe set in CI from the one a developer gets, which is
  the failure this harness exists to catch.

`guards-deep` is excluded by name in two places, `bin/ocelli.sh`'s `--floor`
arm and `scripts/ci_floor_check.py`'s `NOT_IN_FLOOR`. **The mechanism that
joins them is set equality, in both directions**, and this paragraph said
something else until the S03 review's fifth pass. It said that missing either
list "makes the `ci` gate demand a CI step for a gate the floor never runs",
which is the claim `bin/ocelli.sh`'s own comment made and which the fourth pass
measured false: adding `prose` to the shell list alone left the `ci` gate at
exit 0 while `gate --floor` silently stopped running `prose`. A gate leaving
the floor removes work rather than adding a demand, so that direction had no
detection at all. `ci_floor_check.py` PARSES the runner's `case` line now and
refuses a set that differs from `NOT_IN_FLOOR` either way, both files record
the measurement, and this file was the last place still carrying the sentence
they were rewritten to remove.

`scripts/lint_policy_check.py` is in the `guards` gate rather than in `clippy`
because it is check c's class of problem rather than clippy's. The `clippy`
gate runs `-D warnings`, which turns whatever is enabled into an error and
asserts nothing about what is enabled. A lint moved from `deny` to `allow` in
`[workspace.lints]`, or a crate that stops carrying `lints.workspace = true`,
is invisible to it and passes over a smaller set of rules. HLD 27.1's table is
transcribed in `lint_policy_check.py`, and there is a second copy in
`Cargo.toml`'s `[workspace.lints.clippy]` carrying the same five rows at the
same levels. That is not an oversight, it is the point: comparing the two
copies is the whole job of the check, and a rule with one copy has nothing to
be compared against. `unsafe_code = "deny"` is the declared exception: R5 is enforced
by `scripts/unsafe_allowlist_check.py` over the whole tree instead, which is
stronger for what R5 asks, because a per-file `#![allow(unsafe_code)]` would
silence the lint and would not silence the script. The check requires one of
the two and names which it found.

**Where that check gets its file list from, and the honest name for it is a
RECONSTRUCTION.** The member set comes from `cargo metadata --no-deps`, because
`[workspace] members` is not the member set. The SOURCE list under each member
is rebuilt from four keys, and the guard does not know the set clippy compiles:

1. a glob of the member directory, `rglob("*.rs")`,
2. `targets[].src_path` from `cargo metadata`, seeded into the walk,
3. `#[path = "..."]` on a module item, followed transitively,
4. `include!("...")`, followed the same way, with rustc's own resolution rule,
   which is the directory of the file the macro is written in.

Each exists because the previous shape was measured open: `crates/*` missed
`tools/oracle` in the fifth pass, the globs missed a path dependency in the
seventh, the member directory missed a `[lib] path` in the eighth, and in the
ninth an `include!` reached a file none of the first three names. Every one of
those took cargo clippy from 101 to 0 with a group allow in the unread file
while the guard exited 0. The glob is kept beside cargo's answer because a
module reached by a plain `mod x;` is compiled and is named by no target.

**So the class is open and it is declared rather than claimed.** The class is
"a file clippy compiles that the guard does not read", four keys to it have been
found, and the guard's docstring, its OK line and this paragraph all say
"reconstructed" rather than "every `.rs` file a workspace member compiles",
which is what the first two said through four passes in which it was false.

**rustc's own answer was considered as the authority and rejected, with the
reason in `member_sources`.** `target/<profile>/deps/*.d` lists exactly the
files each compilation read, the `include!`d file and the `#[path]` module
among them. It cannot be the authority here because it exists only after a
build, a stale one narrows in silence, which was measured, freshness by mtime
would refuse after every keystroke, making it fresh means this guard runs
`cargo check --workspace --all-targets` at 10.8s in a clone with no `target/`
and again inside every `lint-policy` probe sandbox, and a workspace that does
not compile yields no dep-info at all.

`.cargo/config.toml` is read by the same check and it is PARSED, with `tomllib`,
since the ninth pass. It was matched with an `^`-anchored regex before, which
TOML's dotted key and quoted key both defeat: `build.rustflags =
["-Aclippy::pedantic"]` took cargo clippy from 101 to 0 while the guard printed
"1 cargo config(s) lower no denied lint through rustflags" at exit 0. Three key
paths are read by name, `build.rustflags`, `target.*.rustflags` and
`env.RUSTFLAGS`, and a config that does not parse is refused rather than read
as declaring nothing.

It refuses the FLAG rather than the file: a cargo config is the ordinary home
for an alias, a linker choice and a target runner. Five lint-naming flags are
refused, `-A`, `--allow`, `-W`, `--warn` and `--force-warn`, and that list is a
declared constant since the eighth pass because a probe can only ever write one
of them. That set is a SUPERSET of what is measured to weaken: `-W` and
`--warn` leave cargo clippy at 101 under the `clippy` gate's own `-D warnings`,
so refusing them is a decision rather than a measurement, and the ninth pass
corrected the guard's own claim that the set named "exactly" what weakens.
`--cap-lints` is refused separately at any level but `deny` or `forbid`,
because it names no lint and caps all of them, and both weakening levels have
a probe rather than a ratchet, there being only four levels in total.
`--force-warn` is in the first list against expectation: measured under the
pinned 1.97.1 toolchain it outranks the `-D warnings` the `clippy` gate
passes, so it silences a denied lint exactly as `-A` does. `env.RUSTFLAGS` is
read and refused and is NOT a route today, measured at 101, because `[env]`
sets a variable for the processes cargo spawns rather than for cargo's own flag
resolution.

## What is recorded, and where

`ci/guard-probe-budget.json` holds these keys, all measurements rather than
guesses, plus a `note` restating that:

- `constants`, the declared-constant ratchet's digests
- `constants_count`, so a constant removed together with its digest is noticed
- `gates_declared`, so a row deleted from `bin/ocelli.sh`'s `GATES` array is
  noticed
- `entry_sites`, the refusal count of every entry claiming its file with `"*"`,
  so a refusal added to an already-claimed file moves a number, for every
  refusal shape `scripts/guards/discover.py` scans and no others. It claimed
  this without the qualifier until the S03 review's sixth pass, and a refusal
  written as `problems += [...]` moved nothing
- `uncovered`, the uncovered-refusal ceiling and whether the sweep is complete,
  the second derived from the first rather than remembered
- `oracle_faults`, the fault count the adoption check ratchets against
- `wall_clock_seconds`, per profile, and the pair has to be POSSIBLE. Deep
  selects every probe and the floor's are a subset of them, so deep cannot be
  the faster of the two, and nothing compared the two until the S03 review's
  eighth pass found `deep: 17.2` recorded beside `floor: 18.1`. The seventh
  pass re-recorded floor while moving seventeen probes into deep and left deep
  at its pre-move value, which left the ceiling for the profile that now
  carries every `lint-policy` probe standing for a run that did not contain
  them. The gate was never red, because a ceiling that is too generous never
  is. `wall_clock_problems` in `scripts/guards/census.py` is what refuses it,
  it derives the superset relation from the catalogue rather than asserting
  it, and `census.impossible-wall-clock-pair` watches it

`python3 scripts/guard_census.py --record` writes every one of those except the
last, which `python3 scripts/guard_probe.py --record-budget` writes. Neither is
written on an ordinary gate run, because a gate that quietly rewrote a tracked
file would leave the tree dirty and make the number a record of the last
machine to run rather than a baseline anybody agreed to.

The wall-clock ceiling is deliberately generous, five times the recorded value
plus thirty seconds. It catches a change that made the harness ten times slower
and not a shared runner having a bad afternoon, because a timing gate tuned
tight is a timing gate that gets disabled.

## What this harness does not cover

- **Refusals under `crates/` and in files that are not `.py`, `.mjs`, `.js` or
  `.sh`.** Decision 7 of the design plan, printed on every green census.
- **Whether a `covered_by` test drives its refusal red.** Check f proves the
  test and the file are connected. Nothing more is claimed.
- **The declared limits and the open defects**, which
  `python3 scripts/guard_census.py` names in full with an owner.
- **That the hooks are enabled at all.** Every hook under `.githooks/` is inert
  until a clone runs `git config core.hooksPath .githooks`, which is per clone
  and untracked. `README.md`, `CONTRIBUTING.md` and `docs/DEVELOPER_SETUP.md`
  all instruct it
  and nothing verifies it. The harness enables hooks inside its own sandbox and
  the tripwire reads the real value, so both know about the setting and neither
  enforces it. Decision 11 of the design plan records the finding and says the
  enablement check belongs in `/verify` rather than in CI, because CI has no
  clone to check.
