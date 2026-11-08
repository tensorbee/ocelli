# F-223 review, pass 3

**Reviewed**: `git diff 44398d5 4b263ba` on `work/f-223-xai`, the pure-function slice only (AC4 to AC10, AC12 to AC14). HEAD `4b263ba7b48fbed59c6e48291aae079719649faa`. The worktree was clean apart from the untracked pass 1 and pass 2 reports, before and after this pass. The commit changes six files: `comparison.mjs`, `comparison_test.mjs`, `tolerance.rs`, `pair_fixture.rs`, `ocelli-compare.rs` and `ci/guard-probe-budget.json`.
**Author / reviewer**: GLM-5.3 in opencode wrote `4b263ba`. Claude Opus 5.5 reviewed it as an independent subagent. I wrote none of the reviewed material and none of passes 1 and 2.
**Result**: 2 defects, 1 smell, 4 nitpicks

## Acceptance challenged

- **D1 of pass 2 (ratioText).** I pulled `ratioText` out of the file at HEAD and ran it directly. 5/0 and 0/0 gave `denominator is zero`. 9.996 gave `10.0`, 0.99951 gave `1.00`, 0.0009996 gave `0.00100`, 999.6 gave `1000`, 99.96 gave `100` and 0.09996 gave `0.100`. A sweep over every three-figure mantissa from 1e-6 to 1e5 (10,800 values) matched the expected text everywhere. Three mutations each turned exactly the ratio test red: the exponent taken from the unrounded value, the zero case returning `Infinity`, and `toPrecision(4)`. **Closed.**
- **D2 of pass 2 (the pipe).** Replayed with `A|B` in `versions.playwright`, through `publishedRecord`, `renderSummary` and `scanForDenied`. It now returns `[{"kind":"physician","line":22}]`, and `renderReport` gives the same hit. A value that carries both a backslash and a pipe is caught too. Removing the `cellEscaped` clause turned exactly the new pipe test red. **Closed for behaviour.** The new prose overstates where the transform applies (D1 here).
- **D3 of pass 2 (the boundary doc).** The fixture asserts within-one at 1.0 and `max_abs_diff` at 1. The constants are 0.999 and 2, at `tolerance.rs:58` and `:72`. `exactly_999_permille_within_one_lsb_passes` is at `tolerance_fixture.rs:147`. I read 25.1 at `docs/hld/22-testing-and-tolerance.md:42-44`, and "within 0.1" is inclusive. **Closed.**
- **S1 of pass 2 (the seven refusals).** I deleted each refusal on its own in a scratch copy and restored the file after each run. Each turned exactly one test red, and it was the test named for it: `max_abs_diff > 255`, the fraction loop's finite check, `low_information` dropped from the boolean loop, selected-with-reason, the reason type, the `n` check, and the machine string types. When I deleted the whole boolean loop, two tests went red, because it also drops `passes`, which the closed-schema test drives. Narrowing the mutation to `low_information` alone gave the one-test result that GLM reports. **Closed.** The commit adds a new undriven refusal of the same class (S1 here).
- **S2 of pass 2 (the placeholder bias).**
  - The real `ocelli-compare pair` binary, run on RGBA frames I wrote, printed `"signed_mean_bias_informative": null` for black on black and for white on white. Both runs exited 1 with `low_information` true. Black on white printed 255.0. Ten informative pixels of a hundred printed 0.0 and passed with exit 0. Five of a hundred printed 0.0, with `low_information` true and exit 1.
  - I mutated the Rust back to `Some(map_or(0.0, ..))`. Both all-clipped tests went red, at `pair_fixture.rs:255` and `:292`, and the restore was green.
  - In JS, a null bias with `low_information` false throws the new message. A null with true is admitted. Deleting that refusal turned exactly the null test red.
  - The plain report and the summary both print "not measured, because no pixel was informative", and no `0.0000`. Deleting the render branch turned exactly the null test red.
  - **Closed for the produced case.** The schema admits null beyond the ruling, and the rendered sentence is then false (D2 here).
- **N4 of pass 2 (footprint reset).** Deleting `current = null` after the `Summary Footprint` line turned exactly `the footprint parse reads the recorded shape` red. **Closed.**
- **Census re-record.** I cloned the worktree into the scratchpad. At `44398d5`, `--record` left no diff, so 90 was correct there. At `4b263ba`, `--record` and `--render-runbook` both left no diff, so 92 and the runbook are what the commands produce. `git diff 44398d5 HEAD -- ci docs scripts` touches one line of `ci/guard-probe-budget.json`. The count of `throw new Error` sites went from 89 to 91. The two new sites are the null refusal and the split-out bias finite check.
- **Out of scope.** `git diff --quiet 44398d5 HEAD` over `comparison_run.mjs`, `run.mjs`, `package.json`, `bin/ocelli.sh`, `scripts`, `README.md`, `examples`, `docs` and `tools/bench/comparison.json` exits 0.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `cargo fmt --all --check` | worktree at 4b263ba | now | 0 |
| `cargo clippy -p ocelli-oracle --all-targets -- -D warnings` | same, `CARGO_TARGET_DIR=<wt>/target`, `tolerance.rs` touched to force a re-check | now | 0 |
| `cargo test -p ocelli-oracle --test pair_fixture --test tolerance_fixture` | same | now | 0, 10 + 10 pass |
| `cargo test -p ocelli-oracle --bin ocelli-compare` | same | now | 0, 19 pass |
| `node --test tools/bench/tests/comparison_test.mjs` | same | now | 0, 36 pass |
| `npm run lint` | worktree. `node_modules` exists now because GLM ran `npm ci`, and it is ignored | now | 0 |
| `guard_census.py`, `--check-runbook` | same | now | 0, 0, "0 watched by nothing" |
| `--record`, `--render-runbook` in a clone at 44398d5 and 4b263ba | scratchpad clone, deleted | now | 0, no diff at either commit |
| `prose_check.py`, `source_provenance_check.py` | same | now | 0 (864 files), 0 (1710 files) |
| `ocelli-compare pair` on six synthetic 10x10 RGBA pairs | scratchpad `.raw` files, deleted | now | as listed above |
| JS mutations, 17 single deletions or edits | `git archive HEAD` copy in the scratchpad, file restored after each run, baseline re-run green | now | 15 red as intended. Green: the bias finite check, deleted whole or only its `isFinite` half (S1) |
| Rust mutation, the null replaced by a placeholder zero | scratchpad clone, own target dir, `touch` after restore | now | 2 red, restore green |
| Probes: ratioText sweep, pipe in a version, a reason and the renderer string, null-bias admission matrix | inline node scripts in the scratchpad | now | see findings |
| Added comment lines scanned for `;` and an em-dash | `git diff 44398d5 HEAD` | now | none |
| GLM's round report | tail of `glm-F-223-fix2.log` | prior receipt | its command table agrees with mine. "The new refusal is watched by the S2 test" holds for one of the two new sites (S1) |

## Defects

### D1, the deny scan's new prose says a piped value reaches the summary only in its escaped form
**Where**: `tools/bench/src/comparison.mjs:933-936` ("a value such as `A|B` arrives in the tracked summary only as `A\|B`"), test comment `tools/bench/tests/comparison_test.mjs:604-608` ("reaches the rendered summary as neither its raw form nor its JSON-escaped one")
**What**: `escapeCell` is called in one place only, `versionsSection` at `comparison.mjs:1514`. A workload or metric-cell reason is rendered raw in a bullet, at `:1408` and `:1545`, and so is the renderer string in the first paragraph and the tier sentence. A pipe in any of them reaches the tracked summary unchanged.
**Why it is wrong**: microscope section 3, a claim in prose that is false. This is the same sentence pass 2's D2 asked to correct, and the remediation generalises it past the code. A maintainer who trusts "only as `A\|B`" could treat the raw match as redundant for the summary.
**Evidence**: A published record with `pseudo-ct` unavailable and the reason "skipped at A|B" renders `- pseudo CT: skipped at A|B`. In it, `summary.includes("A|B")` is true and `includes("A\\|B")` is false. A renderer string `ANGLE A|B` also renders raw. The scan still catches both, through the raw match.
**Scope and regression**: prose only, and the behaviour is correct. Narrow both sentences to a version value, or say that the table cells escape a pipe and the bullets do not. No new test is needed.

### D2, the schema admits a null bias beyond the ruling, and the report then renders a false sentence
**Where**: `comparison.mjs:1292-1299` (the check is `low_information !== true`), its doc at `:1276-1283` and inline comment at `:1288-1291`, the `pixelSection` doc at `:1556-1559` and its bullet at `:1583`, and `pair_fixture.rs:277-282`
**What**: The ruling reads: "When no pixel is informative, `signed_mean_bias_informative` is absent ... and serialises as `null`" (plan line 409). Rust produces `None` exactly when `informative.pixels() == 0`, that is, when the informative fraction is 0. The JS admits null whenever `low_information` is true, and that is true for any fraction below 0.10. The inline comment says the schema admits null "only when the run itself said no pixel was informative", and that is false. The doc attributes the `low_information` condition to "the ruling after review pass 2", which does not state it. The pair_fixture doc calls the placeholder "the number the plan refused in advance", but the plan never mentions it before the post-review ruling.
**Why it is wrong**: D14, claim measured divergence. The closed schema exists to refuse a record the producer cannot emit, and this one renders a self-contradiction. It is also microscope section 3, three false sentences.
**Evidence**: A run record with bias null, `informative_fraction` 0.05, `low_information` true and state `refused_pixels` passes `publishedRecord` and renders this:
```
- signed mean bias over the informative region: not measured, because no pixel was informative
- informative fraction: 0.0500
```
**Scope and regression**: this affects the published schema and both renderers. No Rust output reaches it, so only a hand-edited or corrupted record does, and that is the case the schema guards. The smallest fix admits null only when `informative_fraction === 0`, keeping `low_information === true` if the lead wants both. Correct the three sentences, and add one assertion that null at 0.05 throws.

## Smells

### S1, the split-out bias finite check is a new refusal that no test drives
**Where**: `comparison.mjs:1300-1304`
**What**: The commit moves `signed_mean_bias_informative` out of the fraction loop into its own `else if` throw. That throw is one of the two sites that took the census from 90 to 92, and the census counts it as watched by `comparison_test.mjs`. Deleting it, or only its `!Number.isFinite` half, left all 36 tests green. With it gone, a bias of `Infinity` publishes and renders `Infinity`, and a string bias publishes and then throws a TypeError inside `pixelSection`.
**Why it is wrong**: microscope section 4. It is the class pass 2's S1 closed, reintroduced by the same remediation. GLM's report says the new refusal is watched, and that is true of the null site only.
**Scope and regression**: this affects the published schema. The smallest fix is one test that drives a non-finite or string bias through `publishedRecord` and expects `signed_mean_bias_informative is not a finite number`.

## Nitpicks

- N1. The ratio test's comment says "999.6 renders `1000`", but nothing asserts 999.6. It is true, since I probed it.
- N2. The all-clipped fixture's doc, at `pair_fixture.rs:244-248`, says "after it the verdict carried a placeholder zero". That reads as current behaviour, but it is history.
- N3. The schema admits `passes: true` beside `low_information: true`, and now beside a null bias, which renders "not measured" with "the verdict passed: yes". Rust cannot produce this. It is pre-existing as a cross-field gap, and the run round may want it.
- N4. Carried from pass 2 and not taken: N5 (the `--reference` noun is untested), N6 (run round) and N7 (the catalogue's `refuses` sentence).

## Verified clean

- No `as` cast was added. `Option<f64>` serialises as `null` through the existing derive, as the real binary shows. `passes` uses `is_some_and`, which cannot be observed apart from `map_or(true, ..)`, because `None` implies a fraction of 0, which implies `low_information`. No sentence claims otherwise.
- `measured()` uses no `unwrap`, `expect` or `panic!`, and every caller has an informative region, since the suite is green.
- The rewritten `ratioText` doc holds: three significant figures across decades, words for a zero denominator, and `toFixed` exponent notation only from 1e21.
- The new `scanRecordForDenied` sentence ("the renderer rewrites only a pipe") is true, because `escapeCell` is the renderer's only string transform. I grepped for `replace`, case changes, `trim` and `slice` from `:1380` onwards and found none.
- The new pass 2 N1, N2 and N3 edits hold. The stray blank line is gone, no added comment line carries `;` or an em-dash, and both footprint sentences say "throwaway processes".
- The Rust consumers of `PairVerdict` compile. `attribution.rs` keeps its own separate placeholder, outside this story. The only JS consumer of the field is `comparison.mjs`.
- The out-of-scope files are untouched.

## Limitations and handoff

- No browser, Playwright, oracle or GPU job was run.
- For the plan owner: plan line 188 still shows `pub signed_mean_bias_informative: f64`, which the ruling at line 409 superseded.
- The run-round items are still open: AC7's per-workload flip, AC11, AC15 to AC20 and N6.
- I deleted the scratch copies (`mut`, `clone`, `clone-target`, the `.raw` frames). HEAD is still `4b263ba`, and `git diff --quiet HEAD` exits 0. This file is the only addition.
- Next owner: the implementer fixes D1, D2 and S1, then a fresh independent pass 4 reviews the changed material.
