# F-216 review, pass 11

**Reviewed**: `work/f-216-xai`, `git diff d9e68c8 051a5c4`, HEAD `051a5c4` ("F-216, main refuses a per-subject tolerance before any run"). Three paths: `tools/bench/run.mjs`, `tools/bench/tests/run_test.mjs` and `ci/guard-probe-budget.json`. The tree was clean apart from the untracked `.claude/reviews/` passes 1 to 10 and `tools/oracle/out`, which were not reviewed. This report is the only file this pass adds.

**Author / reviewer**: a Claude implementer agent wrote 051a5c4 to close pass 10's S1, N1, N2 and N3. The reviewer is a separate Claude agent invoked by the lead. It wrote none of the reviewed material and none of passes 1 to 10.

**Result**: 0 defects, 1 smell, 3 nitpicks

## Acceptance challenged

Pass 10 S1 (no floor test that `main` reaches the per-subject refusal), N1 (a registry fault exiting 2), N2 (the `acceptRecord` why-check untested at one named tolerance) and N3 (one message for two causes). The wrong behaviours tested were:

- a new `main` test that stays green when `main` stops refusing
- a `main` test that goes red, but only after it has started a build or a runner
- stderr capture that survives a throw and swallows later output
- a test that creates files outside a temp dir
- a registry fault still exiting 2, or exiting 1 without its stack
- the two N3 messages unreachable, or not distinct
- help text that contradicts the refusals
- a probe-budget count that does not match a real new site

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `node --test tools/bench/tests/run_test.mjs tools/bench/tests/record_test.mjs` | HEAD | now | exit 0, 56 tests, 55 pass, 1 skipped (worker instrument, environment) |
| `bin/ocelli.sh gate bench` | HEAD | now | ALL GREEN, 1 gate |
| `python3 scripts/bench_check.py` | HEAD | now | exit 0, "17 benchmark subject(s), 1 naming no blocking story, 10 recorded baseline entr(ies) across 3 host class(es)" |
| `python3 scripts/guard_census.py`, `--check-runbook` | HEAD | now | both exit 0 |
| `python3 scripts/prose_check.py`, `scripts/source_provenance_check.py` | HEAD | now | exit 0 over 840 files, exit 0 over 1677 files |
| `_sites_in("tools/bench/run.mjs", ...)` from `scripts/guards/discover.py` at both commits | `git show d9e68c8:` and `051a5c4:` | now | 14 sites, then 15. The one new site is `run.mjs:224`, "which is outside --subject", which both new test cases drive |
| Mutations in `target/p11-review/tree`, a `git archive` of 051a5c4 with no `node_modules`, deleted after | HEAD | now | see the mutation table below |
| N1 through the scratch copy's `bin/ocelli.sh bench --list`, with `subjects.json[subjects][0]` appended a second time | scratch copy only | now | exit 1, stderr "Error: subject id wasm.cold_start appears twice" and a stack through `parseRegistry`, `loadRegistry` and `main (run.mjs:331)`. The same with `--list viewer.open --tolerance viewer.open=0.1 --tolerance-why x` |
| Seven refusals through the worktree's `bin/ocelli.sh bench ... --out <scratchpad>` under a 15 s alarm | HEAD, unmutated | now | all exit 2, and `--out` was never created. See N3 below. `ci/*.json` and `subjects.json` checksums unchanged |

### Mutations

| Mutation | Result |
|----------|--------|
| none | 55 pass, 0 fail |
| pass 10's S1: `main` passes `resolveRun` `{ ...options, tolerances: {} }` | the `main` test went red, **but the test process never exited**. It built `target/release/ocelli-folder-viewer` in the scratch tree, launched it listening on 127.0.0.1:50971, and stayed alive after the alarm. I stopped that viewer and its test process by PID |
| the `--subject` check in `resolveRun` replaced by `if (false)` | the `resolveRun` test went red, and the run **hung again**. The `--accept --subject` case has no positional ids, so `main` went on to every subject's runner. When I stopped it, it had reached `bin/ocelli.sh test ocelli-render --release -- --ignored --nocapture measures_a_fill_r...` and was still compiling. No GPU test binary had started |
| `inRun` back to the pre-051a5c4 filtered form | 2 fail, `resolveRun` and `main` |
| the "outside --subject" text changed to "not in this run" | 2 fail |
| exit 1 instead of 2 on a `resolveRun` refusal in `main` | 1 fail, `main` |
| `acceptRecord` why-check at `> 1` (pass 10 N2) | 1 fail |
| `loadRegistry()` moved back inside the `try` (reverts N1) | 0 fail, see N1 below |
| the registry made to throw (duplicate id) while the `main` test runs | the `main` test fails and stderr is restored, so the 26 later tests report normally. The temp dir leaked, see N2 below |
| `"--list"` added to the test's `main` argv, then the S1 and `if (false)` mutations again | each red within 7 ms, no build, no runner, no hang. Unmutated, it stays green |

The bench was not run. No oracle job ran. No GPU test binary ran, as the second mutation row says.

## Defects

None.

## Smells

### S1, the new `main` test fails by running the benchmark

**Where**: `tools/bench/tests/run_test.mjs:105-139`, `mainRefusal` and "main refuses a per-subject tolerance before any run".
**What**: the test calls the real `main` with argv that, once past `resolveRun`, is a full benchmark invocation. It has no `--no-build` and no `--list`, and two of its three cases carry `--accept --story F-000 --why probe`. So when `main` stops refusing, the test does not fail fast. It goes on into `runSubjects`. I replayed pass 10's S1 mutation in a scratch copy without `node_modules`, which matches CI. The test reported red, then hung. Before hanging it had built the release folder viewer and left it listening on a port. The adjacent regression, dropping the `--subject` check, hung the same way. Its case names no positional subject, so it started every runner in the registry, including the `ocelli-render --release -- --ignored` GPU microbenchmark build.

On a machine with `tools/bench/node_modules` installed, the case-1 runner can succeed. `acceptRecord` ignores a tolerance for a subject outside the record, so `main` would then write the tracked `ci/bench-baseline.json` with `story: "F-000"` and `why: "probe"`. This last step is from reading `run.mjs:414-430` and `record.mjs:276-350`. It was not executed, because that would run the bench.

**Why it is a smell**: the test is correct today. It goes red under every mutation tried, and on the green path it creates only its `mkdtemp` directory. The hazard is how it fails. Microscope section 2 asks that a test fail when the code is wrong. Here a regression in exactly the code under test turns a floor test into a build, a server launch, a hang and, with an installed browser, a re-baseline of a tracked file. `.github/workflows/ci.yml:134` runs `bin/ocelli.sh gate bench`, so in CI that regression shows up as a job timeout after a release build, not as a red test. The lead asked for this test to create no files outside a temp dir, and it meets that only while the code is right. pass 10 S1 named F-223's planned second mode as the next change to `main`'s early path, and that change is what would trip this.
**Evidence**: the S1 and `if (false)` mutation rows above, each needing a kill by PID. `ps` showed `target/p11-review/tree/target/release/ocelli-folder-viewer` with `LISTEN 127.0.0.1:50971` as a child of the test process.
**Scope and regression**: present on HEAD, in the test only. The production refusal is correct. The smallest change is to add `"--list"` to the argv `mainRefusal` passes. `resolveRun` runs before the `--list` return at `run.mjs:340`, so the same refusal is still reached through `main`, and a regression then returns 0 with a listing instead of starting a runner. I tried exactly this in the scratch copy. It stayed green unmutated, and both mutations went red in under 7 ms with nothing built. If the test should also hold the non-list route, a second safety would be needed, for example asserting that no runner module was imported. That is the implementer's choice.

## Nitpicks

### N1, the registry-fault exit code has no test

Closed in behaviour. Through `bin/ocelli.sh bench --list` with a duplicate id in a scratch `subjects.json`, the command exits 1 with the stack, as it did before d9e68c8. The comment at `run.mjs:329-330` is true. Moving `loadRegistry()` back inside the `try` stays green, so nothing holds this. No caller distinguishes 1 from 2 today, which is why this stays a nitpick.

### N2, `mainRefusal` leaks its temp dir when `main` throws

The `finally` restores `process.stderr.write`, which I confirmed. After a throw from `main`, the 26 later tests reported normally. But `rm(parent)` is after the `finally`, so a throw leaves an empty `ocelli-bench-main-*` directory in `os.tmpdir()`. I reproduced this once and removed the directories my runs left. Two more such directories, from 15:36:52 and 15:37:01, already existed before this pass began. That is consistent with the implementer's own runs and they were left in place. Moving the `rm` into the `finally` closes this. The directory is inside the temp dir and empty, so this does not block.

The capture cannot leak into other tests. Top-level `node:test` tests in one file run one at a time, `--test` isolates each file in its own process, and the runner's report does not travel over the child's stderr. The capture drops `write`'s encoding and callback arguments, and no refusal path uses them.

### N3, carried from pass 10 N4: a named subject the run cannot measure is dropped silently

Unchanged by this diff and still unreachable. `loadRegistry()` gives 17 rows, each with a runner and an existing subject. It is still a nitpick, not a smell. When it does fire, nothing false lands in the baseline: that subject's previous entry is kept and no figure is written. The cost is an ignored flag, and it needs a pending registry row before any run reaches it. If one lands, the refusal belongs beside "not in this run", using `runSubjects`' own `hasRunner && subjectExists` test.

## Verified clean

- **Pass 10 S1 closed in substance.** `main` now has a floor test, which goes red under the S1 mutation and under four adjacent mutations. It asserts the exit code, the exact stderr anchored with `^...\n$`, and that `--out` was not created. The new S1 above is about how it fails, not whether.
- **Pass 10 N1 closed** in behaviour, through the real entry: exit 1 with its stack.
- **Pass 10 N2 closed.** The `> 1` mutation of the `acceptRecord` why-check now goes red.
- **Pass 10 N3 closed.** Both messages are reachable through `bin/ocelli.sh bench`, each exits 2, and they are distinct:
  - "not in this run", for a named subject outside the positional ids, with or without `--accept`, and also under `--list`
  - "outside --subject", for a subject in the run but not the `--subject`, whether the run comes from positional ids or from the whole registry
  - When both apply, "not in this run" wins, since the membership check comes first. Its text is true either way.
  - `--accept --subject viewer.open` with tolerances naming `viewer.open` and `viewer.first_image` refuses `viewer.first_image` only.
- **Help text true.** "The subject must be in this run and, with --subject, be that subject" matches every case above. The `resolveRun` doc comment still matches.
- **Census.** `bench.runner` 14 to 15 matches exactly one new real `js-throw` site, `run.mjs:224`, which both new tests drive. Both census commands exit 0.
- **No stale references.** `git grep` finds the two messages only in `run.mjs` and `run_test.mjs`. README.md, `use.html`, `more.html` and `docs/lld/benchmarks.md` do not mention these refusals. The commit message has no em-dash or semicolon.
- **Tree unchanged.** `git status` and the checksums of `ci/*.json` and `tools/bench/subjects.json` are identical at the start and end of the pass. The scratch copy, including the 380 MB its red runs built, was deleted.

## Limitations and handoff

- The bench was not run. That an installed machine would re-baseline `ci/bench-baseline.json` under the S1 regression comes from reading the code, not from a run.
- No GPU job ran. The second mutation's build reached the GPU microbenchmark's compile step before I stopped it, and no GPU test binary started.
- Implementation should address S1, most simply by adding `--list` to the test's argv. N1 to N3 do not block.
- This is a bounded review of one commit. It is not AC17 acceptance and not sprint readiness.
