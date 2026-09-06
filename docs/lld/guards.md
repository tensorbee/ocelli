# The guard harness

**F-IDs that contributed:** F-X008, F-X009, F-X010, F-X020
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

**No count of a quantity this harness measures appears in this file as a
statement about the harness TODAY.** A number written into prose about the
size of the harness goes stale inside a single sprint, because the harness
moves and the sentence beside it does not. Every such quantity is named by the
command that prints it instead.

**A number recording what a MEASUREMENT SAW, at the moment it was taken, is a
different thing and is kept**, because it is the evidence that a mutation was
actually run rather than described. Those are written as "at the time", and
they are not claims about the current harness. The S03 review's sixteenth pass
found two of them read as present tense, which made this rule look broken when
what was broken was the tense. Counts of things
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

**"Every probe" became true of the bare command in the tenth pass**, and the
same sentence was the same kind of wrong one field over until it did.
`--profile` defaulted to `floor` for listing as well as for running, so bare
`--list` printed the floor rows alone, all marked `floor`, while four files,
this one included, sent a reader to it to check something about the deep set.
The row count is not written here, and the tenth pass wrote one in the commit
that deleted a stale count from two other files: it said 112 and the set was
114 within a sprint. Run the command. A RUN
still defaults to the floor, which is what CI's `guards` gate is about. A LIST
is an inventory and takes every probe, and `--list --profile floor` still
prints the floor set for anyone who wants only that.

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
2. copy the WORKING TREE content and shape of every path in `git ls-files`
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
evidence file is ever copied anywhere. A tracked relative symlink is recreated
as the same relative symlink, including the package-local licence links used by
the wasm gate, only after its target resolves inside both the source repository
and the destination sandbox. Absolute links and relative links escaping either
root are refused. Regular files keep their executable bit. Any other tracked
file shape is refused rather than skipped, because a silent divergence between
the copy and the original is the one thing the control run assumes away.

F-X008 moves the sandbox catch-all census because the shape check now lives in
the copy helper, the caller adds path context, and escaping symlinks have their
own refusals. The focused catalogue suite watches all branches: a contained
relative symlink must remain a symlink, absolute and escaping relative links
must be refused, and an unrepresentable shape must raise the refusal. The two
relative-escape fixtures put their links at different depths under the source
and destination roots. One escapes only the source and the other escapes only
the destination, so either containment condition is independently watched.
The same story moves the pin catch-all for an absent repository grant, an
absent packaged grant, a packaged symlink and differing bytes. The
package-licence unit suite watches all four outcomes. Catalogue probes drive
the packaged-grant absence and a resolving packaged symlink through the real
`--with-size` command.

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
HARNESS" true, and returning `"pass"` instead, left `entry_sites` at 19 AT THE TIME, the
census at exit 0, `--self-test` at 10 properties, the floor profile at 106
probes red and the unit suite at 49, every one of those a reading taken during
that experiment and not a description of the harness now, with the whole
`guards` gate ALL GREEN and
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
`--sprint` and `--all` all shrank and nothing said so.

**The `GATES` array has ONE reader since the eleventh pass**, and this
paragraph used to record the duplication instead of removing it. It said the
census parsed the array with a copy of `scripts/ci_floor_check.py`'s regex and
that `bin/ocelli.sh` carried a third, which is the imprecision that hid the
defect: the runner carries no regex. It reads an entry with `IFS='|' read -r
name gpu desc`, so it imposes no character class on a gate name at all, while
both Python copies spelled it `[a-z-]+`. MEASURED at HEAD: a gate named
`prose2` with a real arm gave bash 29 gates and Python 28,
`scripts/ci_floor_check.py` at exit 0, the census at exit 0, `gates_declared`
unmoved because the Python side never counted the entry, and `gate --floor`
selecting a gate that no CI step ran and no catalogue entry claimed. An entry
the reader could not parse became an omission rather than a refusal.
`ci_floor_check.gate_entries` scans the array as shell WORDS with the same
tokenizer the gate arms are read with, the census calls that function, and an
entry outside `[A-Za-z0-9_-]+` or missing a field is refused by name in both
checks.

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

### The sprint-plan writer is bootstrap-only

`scripts/gen_sprint_plan.py --check` verifies the structured rows, milestone
summaries and goals in the hand-curated sprint plan. The bare command creates
that plan only when it is absent. Once the file exists, the command refuses to
replace it and names the two explicit modes: `--check` for verification and
`--force` for deliberate full regeneration.

The refusal protects prose the allocation cannot reconstruct. Its standing
probe inserts a hand-curated paragraph, runs the bare command and requires the
overwrite refusal. The control runs `--force` and requires success, so a writer
that refuses every mode cannot satisfy the probe.

**A regex reads a Python literal out of a Python file, and one entry is not
that.** `Cargo.toml:workspace.lints` records HLD 27.1's lint table, which is
TOML, and a regex over it is a second reader of a foreign grammar. The
eleventh pass measured the cost: six passes tuned that capture, and a QUOTED
key appended as the last line of the table was outside it and outside
`scripts/lint_policy_check.py`'s row regex at the same time, so cargo clippy
went 101 to 0 with the guard and the census both at exit 0. A `Constant` may
now carry a `read` callable instead of a `pattern`, and that entry calls
`lint_policy_check.workspace_lints_rows`, which is the guard's own `tomllib`
parse rendered as sorted rows. Two mechanisms reading one grammar two ways is
not two mechanisms.

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
  request, and since the ninth pass the `guards` CI job runs the deep harness
  beside it on every event, so nothing about these probes is behind
  push-to-main any more. The two sentences that said otherwise stood here and
  in `scripts/guards/catalogue.py` for a pass after the job they described was
  deleted.
- **`guards-deep`**, not in the floor. `--profile deep` is every probe, so this
  re-runs the floor set and adds the ones that need a toolchain, plus the
  census at `--profile deep`. Every probe it adds declares `needs` `cargo` and
  none needs npm or wasm-pack, which
  `python3 scripts/guard_probe.py --list --profile deep` is the place to
  check. **The count is deliberately not written here**, for the reason
  `Cargo.toml`'s dependency comment gives: it said 41, which was the number at
  `828037e`, written by the commit that took it to 50. `--profile deep` is
  written out because it filters the listing to the probes this bullet is
  about. It was load-bearing until the tenth pass, when bare `--list` printed
  the floor set alone and a reader checking this bullet against it would have
  seen none of them. **It runs in
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
  `gate --floor` including it would run every floor probe twice. **The timings
  are in `ci/guard-probe-budget.json` under `wall_clock_seconds` and are not
  written here**, and the arithmetic over them is not written here either. The
  pair went stale twice. This file, `ci.yml`, `bin/ocelli.sh` and
  `scripts/ci_floor_check.py` all carried deep 23.8s against floor 15.9s while
  the recorded budget disagreed, and the tenth pass answered that by copying
  the recorded pair into all four. The eleventh pass found all four saying deep
  27.2s against floor 18.3s while the file said 28.4 and 18.6, because the
  `--record-budget` run moved them in the same commit that quoted them, and the
  derived sentence about nine seconds of new coverage was arithmetic over the
  stale pair. A copy of a measurement goes stale the next time the measurement
  is taken, so this points at the file, which is the rule that same commit
  applied to the probe count in `ci.yml`. Removing the duplication would mean
  `gate guards` running a different probe set in CI from the one a developer
  gets, which is the failure this harness exists to catch.

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

### The shell half, and whether bash should be asked instead

The `run_gate` arms and the `GATES` array are read by one tokenizer,
`ci_floor_check.shell_pieces`. The S03 review's twelfth pass put the obvious
question to it: `bin/ocelli.sh` is bash, `declare -f run_gate` is bash's own
reparse of the function, and the unit suite already asks bash about the `GATES`
array while nothing asked it about the arms. So why is a shell production
written by hand at all.

**The answer is three measurements, and it splits the question in two.** bash
is not the READER, and bash IS the test oracle.

1. `declare -f` needs the function defined, and defining it needs the file
   EXECUTED. `set -n` is the only way to ask bash to parse without running and
   it does not define functions, so a `set -n` source followed by a
   `declare -f run_gate` prints nothing. A reader that asked bash would
   source `bin/ocelli.sh`. This check exists to read a runner somebody has
   changed, and `scripts/guards/catalogue.py` plants adversarial shell into
   that very file inside a disposable clone and runs the check over it. A
   reader that sources turns "this file is misparsed" into "this file is run".
2. `declare -f`'s output is a pretty-printed form with no stability contract,
   and it differs between the two bash versions on this machine. On one input,
   5.3.15 prints `cat <<'EOF'` and `esac`, and /bin/bash 3.2.57 prints
   `cat  <<'EOF'`, with two spaces, and `esac;`. Consuming that as text is a
   hand-written model of an undocumented printer, which is the same defect
   class one layer along rather than an escape from it.
3. What it would have bought is smaller than it looks. `declare -f` strips
   comments and joins continuations, which the scanner now does from the
   grammar, and it normalises NEITHER spelling the twelfth pass measured as
   fail-open: `<<\EOF` comes back as `<<'EOF'` and `<<'EOF-1'` comes back
   unchanged.

So bash is asked in `scripts/tests/test_guard_readers.py`, where it is safe and
where a divergence is a red test rather than an executed plant. Every row of
the span table and the here-document production is run as a synthetic `case`
arm of `echo` markers, and what bash PRINTS is compared with what the scanner
attributes to the arm. That found a fail-open nobody had suspected: the
backtick's "may a span open inside me" flag was true, and bash's answer is no.
``x=`printf '%s' 'a`b'` `` is an error to bash, "unexpected EOF while looking
for matching `''", so a quote inside a backtick does not hide the closing
backtick, and the scanner had been accepting a file bash refuses.

**The two measured fail-opens the twelfth pass closed**, both in the
here-document delimiter, which was the last hand-written production in the
tokenizer. It was spelled as a regex with an invented character class,
`(['"]?)([A-Za-z_]\w*)\2`, and bash takes a WORD quoted by any of three
mechanisms. When the redirection is not recognised the body is scanned as CODE,
so a `;;` in it ends the arm and everything after it leaves in silence. Planted
in the `prose` arm of a real clone with a real command after it, `bash -n`
green, `scripts/ci_floor_check.py` at exit 0 with that command dropped, and the
same shape under bash running it:

- `<<\EOF`, backslash-quoted, the third mechanism beside `'` and `"`.
- `<<'EOF-1'`, where `\w*` stops at the hyphen and the closing quote then fails
  to match.

The delimiter is read as a shell word by the scanner itself now, which is the
one word production this file has and the one `shell_words` reads the `GATES`
array with. Each half of it was measured: the word is not expanded, `cat
<<EOF$X` wanting the literal `EOF$X`; it is subject to quote removal only, `cat
<<a"b"c` terminating on `abc`. And it ends at a blank, a newline or the first
character of an operator, which `cat <<EOF; echo AFTER` and `cat <<EOF|cat`
both show by running what follows.

**Two smells went with them.** A regex pre-pass joined line continuations
BEFORE the tokenizer, which is the one thing the tokenizer's own header argues
no pass may do, and a backslash ending a COMMENT line continues nothing in
bash: `echo A # c \` then `echo B` prints both. The pre-pass joined the next
line into the comment, the planted command vanished entirely, the arm map came
back holding the NEXT gate's command, and the check refused while naming two
gates neither of which was the one edited. And a here-document BODY was being
read as commands: with a two-word English note in an arm, the check exited 1
reporting that the arm "runs 'a note', 'EOF-1'", which is a guard refusing a
legitimate state. A body is data, it is its own piece kind now, and the
statement split drops it.

**What stays hand-rolled, said as what it cannot see, and it was stated as ONE
thing until the thirteenth pass.** The one thing was that the scanner does not
model compound commands. That is accurate about `shell_pieces` and
`shell_pieces` is not the whole shell reader: it is one tokenizer with four
hand-written productions on top of it, and the fourteenth route lived in the
first of them. "One tokenizer" was achieved and "one reader" was not, and the
limit read as if it had been. Said per production:

- **The tokenizer, `shell_pieces`.** It models spans and words and does not
  model bash's compound commands, so it cannot tell a `(` that opens a subshell
  from one that ends a `case` pattern, and it cannot tell `((` arithmetic from
  `( (` nested subshells the way bash does, which is by attempting the
  arithmetic parse and backtracking. `$(case y in *) ... esac)` closes at the
  pattern's `)` and the arm ends at the inner `;;`, which `NESTED_CASE`
  refuses. `(( a << b ))` reads the `<<` as a here-document whose body runs to
  the end of the region, which refuses. `$'...'` has the right extent and its C
  escapes are not decoded. A here-document inside a substitution is not queued,
  its body being inside the same span. Every one is fail-closed and asserted
  where it is rather than assumed away.
- **`STATEMENT_BREAK` and `_split_statements`, where a statement ends.** They
  see the control operators and nothing else about a list. A `coproc` in front
  of a nested case is accepted by `bash -n`, is missed by `NESTED_CASE`, and
  fails closed only because the statement scanner reports `coproc case x in *`
  as a command CI does not run. That refusal is real and it is not the one the
  compound-command limit above claims carries the weight.
  `_tolerated_statements`, which decides whose failure `bash -e` discards,
  reads the same separators and inherits every one of these blind spots.
- **`NESTED_CASE`, where an arm holds a nested `case`.** Its alternation is
  derived from `SHELL_INTRODUCERS`, so `time case` matches and `coproc case`
  does not, measured both ways.
- **`SHELL_INTRODUCERS` against `SHELL_NOISE`,** whether a head is the work or
  a decision about the work. `eval` and `command` were each measured on the
  wrong side and each has a reader now. What remains is any other head whose
  remainder is really a command, and the split is a list rather than a grammar.
- **`ARM_LABEL` and `GATE_INVOCATION`, where an arm and an invocation begin.**
  Both anchor at a statement head, so both inherit the statement scanner's
  residue rather than adding one.

Doing better on the first of those needs a compound-command parser, which is a
second grammar, and the point of one tokenizer is that there is not one.

**The refusal for a gate name outside `[A-Za-z0-9_-]+` was watched by
nothing**, and the unit test carrying its name asserted the opposite. That test
planted `prose2`, which is INSIDE the class, and asserted the problem list was
empty, under a docstring reading "Refused, and not dropped". MEASURED with the
branch disabled: the census, every `ci-floor` probe there was at the time,
which was 54 of them, and both unit suites stayed at their unmutated status. It is probed now with a dotted name, which is
the shape the refusal's own sentence is about, because a gate name reaches
`re.escape`-free patterns in three files and a dot is a wildcard in every one
of them. The digit case keeps its probe and its note now says what it actually
watches, which is that the entry is COUNTED, and an accept probe beside it
watches the direction the class must not narrow back into.

`scripts/lint_policy_check.py` lost its last hand-rolled reader of `Cargo.toml`
in the same pass. `member_patterns` required a literal `[workspace]` header on
its own line and each member in double quotes, so `workspace.members =
["crates/*"]` and a single-quoted glob each read as no members at all, and both
are one workspace to cargo 1.97.1 at `cargo metadata` exit 0. It reads the
parsed document now, beside `workspace_lints` and `inherits_workspace_lints`,
so there is no regex over TOML left in that file. Its `main` also read the
manifest with an unguarded `read_text`, so a file that is not UTF-8 arrived as
a traceback rather than under the `FAIL:` header, which probe
`lint-policy.manifest-not-utf8` now watches.

### What it means for CI to run one gate

The floor retains per-area CI jobs because their names locate a failure, but a
job does not get to reconstruct a gate arm freely. A floor gate with one
visible executable command may be represented by that command's exact argument
vector. The existing declared additions remain limited to cases that are
strictly stronger than the arm. A narrowed command, a prefix match or an
unlisted addition is not equivalent.

A floor gate with several visible executable commands must be invoked as
`bin/ocelli.sh gate NAME`. The same exact commands split into YAML steps do not
preserve the arm's `&&` exit semantics. Reversing those steps keeps the command
set and loses the order. Moving them into separate jobs loses both order and a
shared failure boundary. The named invocation delegates all three properties
to `bin/ocelli.sh`, while the step remains in the useful per-area job.

The older named-only rule for an arm containing work the extractor cannot see
still applies independently. Its probe first reduces the synthetic arm to one
visible command, so deleting that rule cannot be hidden by the newer
multi-command rule. D-04's exclusions remain a separate set comparison:
`corpus`, `guards-deep` and `oracle` do not become floor gates through this
equivalence rule.

The per-event reader reports an unreachable gate before it asks how a reachable
arm is spelled. That keeps condition-reading probes attached to the missing
event they test. Parser probes that deliberately add a second visible command
expect the named-invocation refusal, because a broken parser drops that command
and makes the refusal disappear. Probes for invisible work instead reduce the
arm to one visible command first, which keeps the extractor-vocabulary rule
independently load-bearing.

`--sprint` and `--all` retain two public spellings and share one selector arm.
Both select every declared gate in `GATES` order. A structural reader test
requires the single `--sprint|--all)` implementation path, and a runtime test
stubs `run_gate` under bash and proves both spellings select the declared list.
The public names communicate sprint and release intent without maintaining two
copies of the complete set.

### The workflow is YAML, and the CI floor now has one dependency

`scripts/ci_floor_check.py` reads `.github/workflows/ci.yml` with
`yaml.load(..., Loader=yaml.BaseLoader)`. That is **deviation D-17**, the only
third-party import anywhere in the CI floor, and it is recorded rather than
quiet because a floor gate acquiring one is exactly the kind of change this
project puts in front of a reviewer.

**What it bought, measured.** The S03 review's twelfth pass planted inputs at
the hand-rolled reader, which is what the eleventh pass's remediation predicted
somebody would do, and found four fail-open routes and five refusals of
workflows GitHub Actions runs correctly. Each was measured against a real parse
and only reported where the parse is unambiguous.

The four, each at exit 0 where the check should refuse:

1. **Key order.** `run_commands` attached whatever `if:` it had seen so far, so
   `- run: bin/ocelli.sh gate guards` with its `if:` written underneath exited
   0, and the identical two lines in the other order exited 1. Two more
   spellings of the same mapping, a quoted `"if":` key and a multi-line flow
   mapping with `run` first, did the same. All three on `guards`, the gate that
   watches every other gate, in an edit that reads as tidying.
2. **`run: >` folding**, which split one narrowed command into two and let the
   argv comparison match the first half. The seventh pass's hole reached by
   changing one character.
3. **A quoted key in `on:`**, which narrowed the event set the floor is checked
   against and printed the narrowed list in the OK line.
4. **A gate name in any `run:` TEXT**, so an `echo` mentioning the gate
   satisfied it, and **a `run:` key at any depth**, so an action input under
   `with:` did too. The gate reached that way was `panic`, HLD section 23's
   wasm panic-hook proof.

A fifth turned up while building the probe for the parse refusal.
`- run: [bin/ocelli.sh gate guards` is an unterminated flow sequence that no
YAML parser accepts, so GitHub Actions cannot run that workflow at all, and the
line reader exited **0** reporting the whole floor covered because the text
after `run:` still held the gate name.

And the five refusals: a quoted `run:` scalar, a block scalar with an explicit
indentation indicator, a single-line flow-mapping step, a trailing YAML comment
on an `if:`, and `on:` written as a block sequence. The comment one quoted a
condition back at its author while saying the gate does not run on events that
condition plainly runs on.

**Why the dependency rather than a fail-closed reader.** Refusing every
spelling the reader does not model closes routes 2, 3 and 4 and is
dependency-free, and it provably does not close route 1: key order is not a
spelling, it is what reading a tree line by line produces. Worse, three of the
five legitimate spellings above are spellings such a rule would have to refuse,
so the fail-closed repair makes a guard that refuses a legitimate state
permanent by design at the moment it leaves the widest route open. A parser
satisfies both halves at once, because it accepts every legal spelling and
yields one tree.

It is also the argument this file has already recorded three times. The shell
arms and the `GATES` array stopped being read with a regex in the ninth and
eleventh passes, and check c's `Cargo.toml:workspace.lints` entry reads TOML
with `tomllib`. YAML was the fourth foreign grammar in that file and the only
one still read by hand. `tomllib` is stdlib and PyYAML is not, and that is the
whole of the difference.

**The cost, stated exactly.** `pyyaml==6.0.3` is pinned in `pyproject.toml`
beside the eight corpus-tooling pins, and `uv.lock` carries it. The `guards`
job in `.github/workflows/ci.yml` installs it, reading the version out of
`pyproject.toml` rather than copying it, and that job is the only one running
the `ci` gate, the census that imports the module, or the probes that run it
inside a disposable clone. An absent PyYAML is a refusal at import with the
install command in it, under the same `FAIL:` header as every other refusal in
that file. There is no `except ImportError` fallback anywhere, because a quiet
return to the reader with four measured fail-open routes would be a floor gate
that silently checks less wherever a dependency is missing. Probe
`ci-floor.pyyaml-absent` runs the guard under `python3 -S`, which takes
site-packages off the path, and reports HARNESS the moment that refusal becomes
a fallback.

`BaseLoader` and not `SafeLoader`, and it is the more restrictive of the two:
it constructs `str`, `list` and `dict` and resolves no implicit tag. What that
buys is that `on` stays the string `'on'` rather than becoming YAML 1.1's
boolean, so the reader's `(?:on|"on"|'on'|true)` alternation went away instead
of being carried into a tree. The cost of that choice is declared in the
catalogue's limit and paid in the refusal message: a workflow whose event block
is written `true:` reads as having no `on` key, and the refusal names the
top-level keys it did read.

**Three existing probes were planting a file GitHub Actions rejects**, and only
the parse could see it. `_put_gate_step_behind` turned a step's `run:` line into
a new `- ` list item, which is right when the `run:` is the step's first key and
invalid YAML when the step carries a `name:` as well. The line reader accepted
it and the probes passed, so `ci-floor.event-gated`, its accept twin and
`ci-floor.complementary-steps` were asserting this guard's behaviour on
something that is not a workflow. A probe input has to be a state the real
system can be in.

### The consumers of the grammar, which the thirteenth pass found unguarded

The twelfth pass closed the grammar and the thirteenth pass measured what sits
on top of it. The verdict is worth keeping in its own words: the tokenizer is
closed, and thirteen shapes planted at `shell_pieces`' span table all failed
closed, and the CONSUMERS of that grammar were not. Four routes, each measured
at exit 0 in a real clone against the code at `726f9c6` and at exit 1 against
the repair.

1. **`STATEMENT_BREAK` had `&&` and no `&`.** bash's `&` terminates a list
   exactly as `;` does, so `A & B` was one statement whose head is `A`, and a
   head that is a `COMMAND_PREFIXES` prefix or a `SHELL_NOISE` builtin took `B`
   out of `unseen_commands` with it. Measured: the `&&` before `node --test` in
   the `bench` arm rewritten as `&` on one line, with `- run: bin/ocelli.sh
   gate bench` replaced by the arm's two extractable commands, gave `bash -n`
   0 and the check exit 0 printing "every command in each gate's arm", `bench`
   gone from the named-only list and six node suites out of CI. This is the
   sixth, seventh and eighth passes with a different operator.

   The fix is not a bare `&` alternative, and the naive spelling was measured
   to refuse a legitimate state: `&` is also the second character of `>&` and
   `<&` and the first of `&>`, and with `r"[\n;{}()]|&&|\|\||\||&"` in place
   `unseen['panic']` grew the entry `'2'` out of the `panic` arm's own `echo
   "wasm-pack is not installed. ..." >&2`, a file descriptor reported as a
   command CI does not run. Both directions are probed.

2. **The workflow's `run:` bodies were scanned line by line** while the
   workflow itself was parsed, so cross-line shell state was discarded. One
   line of code, two defects. A step whose body is `cat <<'EOF'` then
   `bin/ocelli.sh gate panic` then `EOF` satisfied the `panic` gate at exit 0
   with bash never calling the runner, which is the twelfth pass's route 4a in
   a spelling that pass did not close, on HLD section 23's wasm panic-hook
   proof. `echo not \` followed by the invocation reaches the same place. And
   in the other direction a legitimate `python3 scripts/staged_content_check.py
   \` continued onto the next line made the `content` gate read as uninvoked at
   exit 1, while bash runs it as one command. The body goes through
   `shell_source` and `_split_statements` now, exactly as a `run_gate` arm
   does, so the `BODY` piece kind drops a here-document here for the same
   reason it drops one there.

3. **`scripts/lint_policy_check.py` read the `unsafe` arm with a regex, and a
   trailing comment satisfied it.** The pattern was
   `^\s*unsafe\)[^\n]*?python3 scripts/unsafe_allowlist_check\.py`, and
   `[^\n]*?` reaches a `#` as happily as a command. Measured with the arm
   rewritten to run another gate's command and the real one moved into a
   trailing comment, and the CI step deleted: `bash -n` 0, the guard exit 0
   PRINTING that `unsafe_code` is denied by the script in the `unsafe` gate,
   `scripts/ci_floor_check.py` exit 0, the census exit 0 and both unit suites
   exit 0, while the script ran nowhere and HLD 27.1's deny and 27.2 R5 were
   enforced by nothing. **The repair already existed two files over.**
   `scripts/guards/catalogue.py`'s `_ci_arm_commands` calls
   `ci_floor_check.gate_commands`, which is comment-stripped and span-aware by
   construction, and the guard is its third caller now. That is the
   propagation failure again, the answer present in the repository and not
   reaching the caller.

4. **`continue-on-error` was in the parsed tree and nothing read it.** The
   string occurred nowhere in `scripts/`, in this file or in the runbook.
   Three plants, each valid YAML, each at exit 0: the key on the `gate guards`
   step, the same key on the `guards` job, and `|| true` appended to the run.
   `--floor` claims to be what CI runs, and a step whose failure cannot fail
   the run does not run the gate in the sense that claim means, on the gate
   that watches every other gate, in one line that reads as tolerating
   flakiness. Both keys are read now, and `_tolerated_statements` answers the
   third.

**And the oracle stopped at the arm's extent, which is why route 1 survived.**
`scanner_keeps_in_the_arm` in `scripts/tests/test_guard_readers.py` ran
`shell_source` and `_arm_end` and nothing else, so it tested where the arm
ends. Every marker in every planted shape sits inside the extent either way, so
the suite stayed green while a statement was lost one function later.
`scanner_runs_in_the_arm` is the consumer oracle: a marker counts when it is
the only marker of a statement whose head, after the same `SHELL_INTRODUCERS`,
`command_builtin_runs` and `SHELL_NOISE` policy `unseen_commands` applies, is
the command that prints it. Two markers in one statement is a merge, and a
merge is the defect. Every existing shape is driven through it, and a
`WhatCiRunsInAStepBody` class does the same for `run_commands`, which had no
oracle at all.

`_tolerated_statements` is measured against `bash -e` rather than read out of
the errexit paragraph, and the obvious reading of that paragraph is wrong. The
first version of the function asserted that a failing left side of `&&` fails
the step. It does not: `false && true` followed by another line exits 0,
because the short circuit means the command after the final `&&` never runs, so
nothing fires errexit and the list's status is discarded. The same statement
alone in a body exits 1, because the last list decides the script's status. Ten
such measurements are a table in the suite and the reader is asserted against
that table row by row.

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
