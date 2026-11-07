# F-216 review, pass 9

**Reviewed**: `work/f-216-xai`, `git diff 171804b 2e3b42c`, HEAD `2e3b42c` ("F-216, the viewer baseline re-recorded at the reviewed tree"). One path, `ci/bench-baseline.json`, 157 insertions and 113 deletions, all inside the five `viewer.*` entries of host class `darwin|27.0.0|arm64|Apple_M4_Max|16|51539607552`. Records read: the session scratchpad `accept.mjs`, `bench-out/run.json`, `bench.log`, and every `run.json` under `target/f216-runs` and the scratchpad dated 2026-11-06. The tree was clean apart from the untracked `.claude/reviews/` passes 1 to 8 and `tools/oracle/out`, which were not reviewed. This report is the only file this pass adds. Replay outputs are in the scratchpad under `review-p9/`.

**Author / reviewer**: a Claude agent wrote the commit and the accept script. The reviewer is a separate Claude agent invoked by the lead. It wrote none of the reviewed material and none of passes 1 to 8.

**Result**: 1 defect, 0 smells, 3 nitpicks

## Acceptance challenged

AC17 (budgets in Open question A met on the named host, release build, with a run record). Plausible wrong behaviours tested: figures typed by hand rather than taken from a run, a debug build recorded as release, a tolerance whose band crosses the budget, a provenance sentence whose arithmetic is wrong, the script writing fields `acceptRecord` would not, and collateral changes to other subjects or host classes.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `git diff 171804b 2e3b42c --stat` | the reviewed range | now | one file, `ci/bench-baseline.json` |
| Replay: `accept.mjs` with `BASELINE_PATH` pointed at a scratch copy of `git show 171804b:ci/bench-baseline.json` and `today` fixed to `2026-11-06`, then `cmp` against `git show 2e3b42c:ci/bench-baseline.json` | scratchpad `bench-out/run.json`, `tools/bench/src/record.mjs` at HEAD | now | byte identical |
| `bench.log` against `run.json` | scratchpad | prior receipt | log brackets the run 15:09 to 15:11 GMT, `EXIT=0`, five measured, figures match `run.json` |
| Commit times | `git log`, reflog | now | 171804b at 14:52:19, run 15:09 to 15:11, 2e3b42c at 15:12:55. HEAD was 171804b during the run |
| JSON diff excluding the five entries | base and after | now | rest equal, subject order kept, host class block equal, instrument equal, `replaced` equals each old `value` |
| Tolerance arithmetic | python | now | 1020 x 1.0784 = 1099.968, 1.0785 gives 1100.07. 462,241,792 x 1.1426 = 528,157,472, 1.1427 gives 528,203,696 over 528,194,192. 8.8 x 1.25 = 11.0 at most 20. 109.03 x 0.75 = 81.77 at least 55. 2.5 x 1.25 = 3.125 at most 5. 1020 x 1.25 = 1,275 and 462,241,792 x 1.25 = 577,802,240 |
| `python3 scripts/bench_check.py` | HEAD | now | exit 0, "17 benchmark subject(s), 1 naming no blocking story, 10 recorded baseline entr(ies) across 3 host class(es)" |
| `check_baseline` mutation: delete `viewer.open` `tolerance_provenance`, set resident tolerance 1.2 | in memory copy | now | three problems reported, clean copy reports none |
| `bin/ocelli.sh gate bench` | HEAD | now | ALL GREEN, 146 tests, 144 pass, 2 skipped (worker instrument, environment) |
| viewer figures across 2026-11-06 runs | 40 records | prior receipts | open 1019, 1027, 1133 (load 41), 1037, 1036, 1026, 1037, 1020. resident 430.9 to 463.9 MB. window_step 1.9 to 2.7 ms. first_image 8.8 to 10.4 ms. scroll 108.6 to 109.2 |

The bench itself was not run. Load average was 13.3 at the start of the pass.

## Defects

### D1, the baseline file's own prose says only `--accept` writes its numbers

**Where**: `ci/bench-baseline.json`, `$about[0]`, and the five entries the commit writes.
**What**: `$about[0]` says the file is "Written by `bin/ocelli.sh bench --accept`, which is the only thing that writes a number here." The five entries in this commit were written by a script outside the tree, `accept.mjs` in the session scratchpad, which imports `acceptRecord` and applies it five times to a saved `run.json`. Neither the entries nor the commit message say so. The commit body is the subject line alone.
**Why it is wrong**: a factual sentence in the changed file is false after the change (microscope section 3). The script also bypasses two of `run.mjs`'s refusals. It does not check `reason === "runner_failed"`, and the run and the accept are no longer one process, so nothing in the tree ties the accepted `run.json` to the run that produced it. Neither refusal would have fired here: all five subjects are `measured`, and `bench.log` shows the same five figures. The next re-baseline can copy the pattern without either.
**Evidence**: the replay above is byte identical, so the content is what `acceptRecord` produces. The script exists only under `/private/tmp`, and that directory is cleaned.
**Scope and regression**: affects the provenance of these five entries only. The figures are correct. Ways to close it, for the lead to choose: re-record through `bin/ocelli.sh bench --accept` (one run with the carried tolerances, then one `--subject --tolerance` run each for `viewer.open` and `viewer.resident_memory`, which takes those two from separate runs as the earlier `viewer.window_step` entry was), or give `run.mjs` a way to pass a tolerance per subject, or amend `$about[0]` and the record to name the path that was used. The third keeps the figures and weakens the sentence, so it needs a ruling and not just an edit.

## Smells

None.

## Nitpicks

### N1, `viewer.open` at 0.0784 is a reasonable bound and flaps only under heavy contention

The band is 940.0 to 1099.97 ms around 1,020. Same-day release runs at load average 7 to 38 gave 1,019 to 1,037 ms, at most +1.7 per cent, which is a little over a fifth of the band. The pass reports' 1,013 to 1,037 ms sits inside it. The one run outside was 1,133 ms (+11.1 per cent) at load 41, and it would read `moved`. Because the upper edge is the 1,100 ms budget, an upward `moved` also means the budget was missed, so that verdict is not noise. On the low side, older trees gave 935 to 976 ms, so a real speed-up of 8 per cent or more will flag, which the two-sided rule intends. `--compare` is not a gate. The sentence saying the value is not a calibrated spread is accurate.

### N2, the carried `viewer.window_step` tolerance sits close to its low edge

The band is 1.875 to 3.125 ms around 2.5 ms. Readings come in 0.1 ms steps, and same-day runs gave 1.9 to 2.7 ms. The 1.9 reading is at -24 per cent, just inside, so a slightly faster run will read `moved`. This is the carried 0.25 and not something this commit changed. The sentence calling it headroom, "not a calibrated spread", is accurate.

### N3, "at commit 171804b" is inferred, not recorded

`run.json` records no commit or tree. The claim holds on timestamps: HEAD was 171804b from 14:52:19 to 15:12:55 and the run was 15:09 to 15:11. Nothing in the record would catch a dirty tree.

## Verified clean

- The values come from a real harness run. `bench-out/run.json` (`artefact_provenance: "built by this run"`, every `detail.profile` = `release`) matches `bench.log` and matches every `value`, `detail`, `conditions`, `instrument` and `host_class` in the five entries.
- The script calls the harness's own `acceptRecord`, with the run's record before any compare, `story: "F-216"`, a `why`, an explicit tolerance and `toleranceWhy` per subject, and today's date. It writes only `BASELINE_PATH`, in the `JSON.stringify(_, null, 2)` plus newline format `run.mjs` uses. The replay is byte identical.
- Every field in `BASELINE_ENTRY_FIELDS` is present. Tolerances are in (0, 1], `provenance` is `measured`, and each entry's `host_class` equals its block's. `bench_check.py` passes and refuses the mutated copy.
- Each band stays within the Open question A budget: open at most 1,100 ms, first image (CT) at most 20 ms, scroll at least 55 fps on the low side, window step (512) at most 5 ms. Resident x (1 + t) stays at or below 528,194,192, and 0.1426 and 0.0784 are the largest four-place values that do. Each run's own `thresholds_met` is true, including the DX legs (median 30.4 of 250 ms and 19.5 of 70 ms), heap 98,123,775 of 118,686,080, and zero blank samples on release and debug.
- The `tolerance_provenance` sentences are true: the arithmetic above, the 1,275 ms and 577,802,240 byte figures, "carried from the 2026-11-05 first recording" (the old tolerance was 0.25 with "first recording headroom"), and the design states the 528,194,192 byte bound. The `why` sentence is true apart from the inference in N3. The old entries did say they came from the uncommitted working tree.
- Nothing outside the five entries moved: other host classes, `$about`, other subjects, subject order and the host class block are all unchanged. `replaced` carries each previous `value`.
- No stale quote of the old figures in README.md, `use.html`, `more.html`, `docs/lld/benchmarks.md` or the design. No em dash or semicolon in the new strings.
- `bin/ocelli.sh gate bench` is green.

## Limitations and handoff

- The bench was not run (load 13). Flap estimates come from recorded runs on trees from pass 6 to HEAD, not all of them 171804b.
- The pass cannot prove the scratchpad `run.json` was not edited between the run and the accept. `bench.log` matches it on the five headline figures only, not on the samples.
- D1 needs a lead ruling on which way to close it. This is a bounded review of one commit. It is not AC17 acceptance and not sprint readiness.
