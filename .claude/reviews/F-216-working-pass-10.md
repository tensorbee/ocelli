# F-216 review, pass 10

**Reviewed**: `work/f-216-xai`, `git diff 2e3b42c d9e68c8`, HEAD `d9e68c8` ("F-216, per-subject bench tolerance and the baseline re-accepted"). Five paths: `tools/bench/run.mjs`, `tools/bench/src/record.mjs`, `tools/bench/tests/run_test.mjs`, `ci/guard-probe-budget.json` and `ci/bench-baseline.json`. The implementer's artefacts were read from the session scratchpad `f216-impl/`: `run1.log`, `run1/run.json`, `baseline-before.json`, `red.txt`, `mutate.py` and `gate.txt`. The implementer's own transcript was read for the exact command line of the run. The tree was clean apart from the untracked `.claude/reviews/` passes 1 to 9 and `tools/oracle/out`, which were not reviewed. This report is the only file this pass adds.

**Author / reviewer**: a Claude implementer agent wrote d9e68c8 under the lead's ruling on pass 9 D1. The reviewer is a separate Claude agent invoked by the lead. It wrote none of the reviewed material and none of passes 1 to 9.

**Result**: 0 defects, 1 smell, 4 nitpicks

## Acceptance challenged

Pass 9 D1 (the baseline's numbers written outside `--accept`) and the lead's ruling: a per-subject `--tolerance <subject>=<fraction>`, then one real `--accept` run. The wrong behaviours tested were:

- a refusal that fires only after the build or the run, or only in the unit and not through `bin/ocelli.sh bench`
- a changed bare form for an existing caller
- a `resolveRun` that is not a pure move
- the bare form overriding a per-subject one
- an unnamed subject losing its previous tolerance or provenance
- a why-refusal that misses the per-subject form
- baseline figures not from this run, or written by a second process
- a band that crosses the Open question A budget
- false `why` or `tolerance_provenance` prose
- probe sites counted that are not real or not watched

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `node --test tools/bench/tests/run_test.mjs tools/bench/tests/record_test.mjs` | HEAD | now | exit 0, 55 tests, 54 pass, 1 skipped (worker instrument, environment) |
| `bin/ocelli.sh gate bench` | HEAD | now | exit 0, ALL GREEN |
| `python3 scripts/bench_check.py` | HEAD | now | exit 0, "17 benchmark subject(s), 1 naming no blocking story, 10 recorded baseline entr(ies) across 3 host class(es)" |
| `python3 scripts/guard_census.py`, `--check-runbook` | HEAD | now | both exit 0, "0 watched by nothing" |
| `python3 scripts/prose_check.py`, `scripts/source_provenance_check.py` | HEAD | now | exit 0 over 840 files, exit 0 over 1677 files |
| 15 refusals through `bin/ocelli.sh bench ... --out <scratch>`, each under a 15 s alarm | HEAD | now | each exits 2 within the second with its message, and the `--out` directory was never created. Cases: not in run (positional), not the `--subject` under `--accept`, `viewer.absent` under `--accept`, `=0.1`, the same subject twice, two bare forms, per-subject with no why, `=zero`, `=1.5`, `=` with no fraction, why alone, bare with no why, `0`, unknown subject, `--list` with an absent named subject. `--list --tolerance 0.25 --tolerance-why x` exits 0. `ci/bench-baseline.json` and `ci/guard-probe-budget.json` checksums unchanged afterwards |
| Bare form, old against new | `git archive` of 2e3b42c and d9e68c8 under `target/`, `node run.mjs` | now | same exit and message for seven bare-form and unknown-subject cases. `parseArgs` gives the same `tolerance` and `toleranceWhy`, plus `tolerances: {}` |
| Registry fault, old against new | duplicate id appended to the copied `subjects.json`, `--list` | now | old exit 1 with a stack, new exit 2 with the message only (N1) |
| Replay of `acceptRecord(baseline-before, run1/run.json, {tolerance: 0.25, tolerances: {open 0.06, resident 0.12}, ...})` | scratchpad, HEAD `record.mjs` | now | byte identical to `git show d9e68c8:ci/bench-baseline.json`. With `tolerance: null` it differs, so the bare 0.25 was passed |
| `baseline-before.json` against `git show 2e3b42c:ci/bench-baseline.json` | scratchpad | now | `cmp` identical |
| Implementer's command line | session transcript | prior record | one `OCELLI_PORT=0 bin/ocelli.sh bench <five viewer ids> --accept --story F-216 --why ... --tolerance viewer.open=0.06 --tolerance viewer.resident_memory=0.12 --tolerance 0.25 --tolerance-why ... --out .../run1`, with no `--no-build` |
| `run1.log`, mtimes | scratchpad, worktree | prior receipt, now | HEAD 2e3b42c, three modified `tools/bench` files, START 15:22:26Z, END 15:24:07Z, EXIT 0, "re-baselined into ci/bench-baseline.json". `run.json` `started` is 15:24:07.193Z and `ci/bench-baseline.json`'s mtime is 15:24:07. `run.mjs` and `record.mjs` were last modified at 15:21:20, before the run, and committed at 15:25:47 |
| Mutations of my own, in a scratch copy under `target/`, since deleted | HEAD | now | red: provenance keyed on the bare form, bare wins over per-subject, `inRun` from the whole registry, the `parseArgs` why-check at `> 1`, unnamed subject losing its previous tolerance. **Green**: `main` passing `resolveRun` an empty `tolerances` (S1), and the `acceptRecord` why-check at `> 1` (N2) |
| Discovered sites in `run.mjs` | `scripts/guards/discover.py` | now | 14 `js-throw` sites. The five new ones are "names no subject", "one bare fraction", "names ... twice", "unknown subject" (an inline `stderr` write and `return 2` before, so not counted) and "not in this run". Each has a `run_test.mjs` case that drives it |

The bench itself was not run. Load average was 2.0 at the start of the pass.

## Defects

None.

## Smells

### S1, nothing in the floor tests that `main` reaches the new refusal

**Where**: `tools/bench/run.mjs:323-329` (`main` calls `resolveRun`) and `tools/bench/tests/run_test.mjs:76-96`.
**What**: the "not in this run" refusal is tested only by calling `command.resolveRun` directly. Nothing calls `command.main`. I changed `main` to pass `resolveRun` the options with `tolerances: {}`, and the full `run_test.mjs` and `record_test.mjs` stayed green, 0 failing. The same is true of any change to `main` that goes back to the inline only-filter it had one commit ago. In both cases `bin/ocelli.sh bench viewer.first_image --accept ... --tolerance viewer.open=0.06` stops refusing before the run. It then runs, records `viewer.first_image`, prints "re-baselined" and exits 0, with the named tolerance applied to nothing. That is the hazard `resolveRun`'s doc comment says it closes. The test's own message, "resolveRun must be the exported production seam main calls before a run", states the requirement, but the assertion only checks `typeof command.resolveRun === "function"`.
**Why it is a smell**: the refusal is correct today. I drove it through the real entry, and every case exits 2 before any build. But the census counts the site as watched by `run_test.mjs`, and it is watched only at function level. Microscope sections 4 and "Composition challenge": tests for one entry point do not prove another. The failure mechanism is concrete. F-223's design (`.claude/plans/F-223-design.md`, rows for `--compare-cornerstone` and line 234) adds a second mode branching in `main`, refused before any launch, so `main`'s early path is the next thing scheduled to change.
**Evidence**: the mutation above, 0 failing. Unmutated, the real entry refuses all 15 cases with exit 2.
**Scope and regression**: present on HEAD. Affects every `--tolerance <subject>=` caller through `bin/ocelli.sh bench`. The smallest meaningful regression is one floor test calling `await command.main([...])` for the positional case and the `--accept --subject` case, asserting the return 2 and the stderr text. `main` returns before `runSubjects`, the build and any runner import, so this is safe in the floor. The same test would also hold the unknown-subject refusal, which was never tested through `main` before this commit either.

## Nitpicks

### N1, `resolveRun` is not quite a pure move, because a registry fault now exits 2

`loadRegistry()` moved inside the new `try`. Before the change a `subjects.json` the registry refuses (for example a duplicate id) rejected `main`. The top-level handler then printed the stack with exit code 1. Now the error is caught, printed without the stack, and exits 2, which is the exit code for an argument refusal. I reproduced this on archived copies of both commits. No caller or doc distinguishes 1 from 2 today, and `bench_check.py` refuses a bad registry in CI either way. Calling `loadRegistry()` before the `try` restores the old behaviour.

### N2, the `acceptRecord` why-refusal is not tested for a single named tolerance

The test's refusal case passes two named tolerances, so changing `Object.keys(tolerances).length > 0` to `> 1` in `record.mjs` stays green. `parseArgs` refuses the single case in production, and removing the per-subject clause entirely (the implementer's M8) does go red. So the gap is only the boundary of the second layer. One more `assert.throws` with one named tolerance closes it.

### N3, the refusal text says "not in this run" for a subject that did run

`bin/ocelli.sh bench viewer.open viewer.first_image --accept --subject viewer.first_image --tolerance viewer.open=0.06 --tolerance-why x` is refused with "--tolerance names viewer.open, which is not in this run". `viewer.open` is in the run. It is outside `--subject`. The refusal is right, and `resolveRun`'s doc comment says "or outside `--subject`". The message and the help line ("the subject must be in this run") do not say so.

### N4, a named subject the run cannot measure is dropped silently, and today's registry has none

`resolveRun` checks that a named subject is in the resolved rows. A row with no runner, or whose story is not in progress or done, passes that check and is then recorded `unavailable`. `acceptRecord` then drops its tolerance and keeps the previous entry, still exiting 0, whereas `--subject` on the same row refuses with "this run has no measured figure". I reproduced this with a synthetic record. All 17 rows today have a runner and an existing subject, so no real call reaches it. The registry header names pending rows as a supported state, so the gap opens the first time one lands.

## Verified clean

- **Ordering and the real entry.** `parseArgs` is `main`'s first act and `resolveRun` its second, both before `runSubjects`, the build and the baseline read. All 15 refusals exit 2 through `bin/ocelli.sh bench` and create no `--out` directory. The `node_modules` precondition in `bin/ocelli.sh` comes earlier still and is unchanged.
- **The bare form is unchanged** for every existing caller. The only callers or documents of `--tolerance` are the help text, `run_test.mjs`, the baseline's prose, `docs/lld/benchmarks.md:328-332` and past review and plan text. None passes two bare forms, and that is now the only bare-form change. README.md, `use.html` and `more.html` do not mention the flag.
- **`resolveRun`** is otherwise a pure move. It has the same `unknown subject <ids>` message, the same exit 2 and the same filter order. The added check uses the `--subject` restriction exactly as `acceptRecord`'s `chosen` does.
- **`acceptRecord`.**
  - Per-subject wins over bare, and bare covers the rest.
  - With no form naming a subject, its previous tolerance and `tolerance_provenance` both carry over.
  - With a form, the new `toleranceWhy` is written.
  - The why-refusal covers both forms, apart from N2.
  - Mutations of each of these went red, apart from N2.
- **The baseline came from one real run.**
  - There was a single `bin/ocelli.sh bench --accept` process with a build. `artefact_provenance` is "built by this run" and every `detail.profile` is `release`.
  - The replay is byte identical, and the mtimes tie the write to that process.
  - `value`, `unit`, `detail`, `instrument`, `conditions` and `host_class` equal `run1/run.json` for all five entries. `replaced` equals each previous `value`.
  - Nothing else moved: other host classes, other subjects, subject order and `$about` are all unchanged.
- **Every band is within budget.**

  | Subject | Edge | Budget |
  |---------|------|--------|
  | open | 1018 x 1.06 = 1,079.08 ms | at most 1,100 |
  | resident | 445,956,096 x 1.12 = 499,470,828 | at most 528,194,192 |
  | first image | 9.7 x 1.25 = 12.12 ms | at most 20 |
  | scroll, low edge | 109.08 x 0.75 = 81.8 fps | at least 55 |
  | window step | 2.4 x 1.25 = 3.0 ms | at most 5 |

  The band for open, 956.9 to 1,079.1 ms, contains every same-day release reading that pass 9 listed except the 1,133 ms reading at load 41.
- **The prose is true.**
  - `$about[0]` ("Written by `bin/ocelli.sh bench --accept`, which is the only thing that writes a number here") is now true for these five entries.
  - Each part of the `why` checks out: one run, this host, release build, HEAD 2e3b42c with the two files uncommitted, and the replaced entries written by a script.
  - "Which no runner imports" holds. Nothing under `tools/bench/src` (runners, `browser_session.mjs`, `viewer_work.mjs`), `tools/bench/page` or `subjects.json` imports `run.mjs` or `record.mjs`. `run.mjs` imports `record.mjs` and loads the runners.
  - The uncommitted `run_test.mjs` at run time goes unmentioned and is imported by nothing the run executes.
  - The `tolerance_provenance` arithmetic is true. "Keep the 0.25 carried from the 2026-11-05 first recording" is true in value: ab5e576 recorded 0.25 "first recording headroom" for all three.
- **The probe budget.** `bench.runner` going from 9 to 14 counts five real new `js-throw` sites, each driven by a `run_test.mjs` case. The census a2 equality check passes, and "0 watched by nothing" is unchanged.
- **Help text** for both forms and `--tolerance-why` is accurate, apart from N3. `docs/lld/benchmarks.md` has no false sentence. It does not describe the per-subject form, which is an omission rather than a false claim.
- The implementer's own red-first receipt (`red.txt`, 7 failing before) and nine-mutation script were read. They are separate from this pass's mutations, which were chosen to differ.

## Limitations and handoff

- The bench was not run. That the run's working tree matched the committed `run.mjs` and `record.mjs` rests on mtimes, not on a recorded tree hash, and `run.json` still records no commit or tree.
- `--compare` against the new entries was not exercised.
- S1 needs a floor test through `main`. N1 to N4 do not block. This is a bounded review of one commit. It is not AC17 acceptance and not sprint readiness.
- Scratch copies under `target/p10-review` were deleted. The worktree's `git status` is identical to its state at the start of the pass, apart from this report.
