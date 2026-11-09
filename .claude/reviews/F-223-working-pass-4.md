# F-223 review, pass 4

**Reviewed**: `git diff 4b263ba c73dc8b` on `work/f-223-xai`, the pure-function slice only (AC4 to AC10, AC12 to AC14), plus one sweep of the whole slice, `git diff 73b68ad c73dc8b -- tools scripts ci docs`. HEAD `c73dc8b20074d006a695559b2fd9882ec4d36c23`. The range holds two commits. `694a19c` is the lead's plan edit (two lines of `.claude/plans/F-223-design.md`). `c73dc8b` is GLM's fix round and touches `comparison.mjs`, `comparison_test.mjs` and `pair_fixture.rs`. The worktree was clean apart from the untracked pass 1 to pass 3 reports, before and after this pass.
**Author / reviewer**: GLM-5.3 in opencode wrote `c73dc8b` (its log is `glm-F-223-fix3.log`). The sprint lead wrote `694a19c`. Claude Opus 5.5 reviewed both as an independent subagent. I wrote none of the reviewed material and none of passes 1 to 3.
**Result**: 2 defects, 3 smells, 5 nitpicks

## Acceptance challenged

- **D1 of pass 3 (the pipe prose).** I ran `renderSummary(publishedRecord(..))` on a copy of HEAD. With `A|B` in a version cell the summary holds `A\|B` and not `A|B`, and `scanForDenied` returns `[{"kind":"physician","line":23}]`. With `A|B` in an unavailable workload's reason, the line is `- pseudo CT: skipped at A|B`. It is raw, and the scan hits line 43. With `ANGLE A|B` as the renderer, the text is raw and the scan hits line 9. `escapeCell` has one call site, `versionsSection` at `comparison.mjs:1517`. The other tables carry only closed IDs, enums and numbers. Both rewritten sentences now match this. **Closed.**
- **D2 of pass 3 (null beyond the ruling).** The null matrix through `publishedRecord`:
  - fraction 0.05, `low_information` true: throws "admits only when the informative fraction is exactly 0".
  - fraction 0, `low_information` true: publishes. Both renderers print "not measured, because no pixel was informative".
  - fraction 0, `low_information` false: throws.
  - fraction 0.621, `low_information` false: throws.
  - Deleting `pair.informative_fraction !== 0 ||` turned exactly `a bias that was not measured is null and the reports say so` red.
  - Deleting `|| pair.low_information !== true` left all 37 green (S1 here).
  - The three sentences pass 3 named: the inline comment and the doc now state the fraction condition and do not credit `low_information` to the ruling. The pair_fixture doc's "the plan refused in advance" became "the ruling refused in advance", and that is still false (D2 here).
  - **Closed for null at a non-zero fraction.** The converse, a number at fraction 0, is open (D1 here).
- **S1 of pass 3 (the finite refusal).** Infinity, -Infinity, NaN and the string `"0.1"` each throw "signed_mean_bias_informative is not a finite number". Deleting the whole `else if` turned exactly `a bias that was measured is a finite number` red. So did deleting only its `!Number.isFinite` half. Deleting only the `typeof` half left everything green, because `Number.isFinite` already refuses a non-number (N2). **Closed.**
- **Pass 3 N1 and N2.** `ratio(999.6, 1)` is now asserted to include `| 1000 |`. The fixture doc reads "in between the verdict carried a placeholder zero". `git show` confirms that order: `bdec2c9` gave the empty-statistic error, `120228e` the placeholder and `4b263ba` the null. **Closed.**
- **The plan edit `694a19c`.** The struct now reads `Option<f64>`, which matches `tolerance.rs:320`. The ruling now names "an informative fraction of exactly 0", which matches the Rust `None` condition `informative.pixels() == 0`.
- **The whole-slice sweep.** I ran node's line coverage over `comparison.mjs` under `comparison_test.mjs`. I also neutralised each of the 91 `throw new Error` sites in turn, rewriting it to `void new Error`, and re-ran the suite. 39 sites leave every test green. 34 of those are never executed by any test. The other 5 are reached, but a later throw satisfies the test's loose regex. Two output branches that render a reason are never executed either. In Rust, deleting `pair`'s candidate refusal and its `--width` parse refusal each left the 19 bin tests green. See S2 and S3.
- **AC map.**

| AC | Named test |
|---|---|
| AC4 | `workload selection is by attribute` (`comparison_test.mjs:390`) |
| AC5 | `quartiles are linear between order statistics` (`:54`), plus `a metric with fewer valid trials than N is invalid and has no median` (`:77`) |
| AC6 | `a difference is stated only when spreads separate` (`:96`) |
| AC7 | `trials alternate sides` (`:115`). The per-workload flip is the run round's, as pass 3 said |
| AC8 | covered by `the process tree is the root and every descendant` (`:139`), `the footprint parse reads the recorded shape` (`:296`) and `the two memory figures are the lower and the upper bound` (`:344`). The plan's named test, `memory counts every process in the inventory`, does not exist (N3) |
| AC9 | `cpu refuses a changed inventory` (`:359`) |
| AC10 | `tools/oracle/tests/pair_fixture.rs`, 11 tests |
| AC12 | `outputs refuse an identifying value` (`:538`) |
| AC13 | `the published record has a closed schema` (`:670`) |
| AC14 | `the summary is measured and acknowledging` (`:936`), plus `the summary and the report pass prose_check over the rendered text` (`:1042`) |

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `cargo fmt --all --check` | worktree at c73dc8b | now | 0 |
| `cargo clippy -p ocelli-oracle --all-targets -- -D warnings` | same, `CARGO_TARGET_DIR=<wt>/target`, `pair_fixture.rs` and `tolerance.rs` touched to force a re-check | now | 0 |
| `cargo test -p ocelli-oracle --test pair_fixture --test tolerance_fixture` | same | now, and again after the scratch builds | 0, 10 + 10 pass |
| `cargo test -p ocelli-oracle --bin ocelli-compare` | same | now, and again after the scratch builds | 0, 19 pass |
| `node --test tools/bench/tests/comparison_test.mjs` | same | now | 0, 37 pass |
| `npm run lint` | worktree | now | 0 |
| `guard_census.py`, `--check-runbook` | same | now | 0, 0, "0 watched by nothing" |
| `prose_check.py`, `source_provenance_check.py` | same | now | 0 (864 files), 0 (1710 files) |
| Replay probes: the pipe in a version, a reason and a renderer, the null matrix, non-finite and string biases, a number at fraction 0 | `git archive HEAD` of `tools/bench` and `scripts` under `target/mp4`, deleted | now | as listed above and in D1 |
| JS mutations: the fraction clause, the `low_information` clause, the whole finite check, each half of it, the null branch never taken, the two reason renderers | same copy, file restored after each run and `cmp`'d against the original | now | red: fraction clause, finite check, `isFinite` half, null branch. Green: the `low_information` clause (S1), the `typeof` half (N2), both reason renderers (S3) |
| Every `throw new Error` site neutralised in turn (91 runs) | same copy | now | 39 green (S2) |
| `node --test --experimental-test-coverage` | same copy | now | throw lines never executed are listed in S2 |
| Rust mutations: delete `pair`'s candidate refusal, make `--width` parse to 0 instead of refusing | `git archive HEAD` under `target/mp4rs`, deleted, then the worktree tests re-run | now | both green, 19 of 19 |
| `git log` timestamps of `120228e` and `44398d5`, and `git show` of `tolerance.rs` at three commits | repository | now | the placeholder at 16:38, the ruling at 16:53 |
| Added lines scanned for `;` and an em-dash | `git diff 4b263ba c73dc8b` | now | none in prose |
| Out of scope | `git diff --quiet 4b263ba HEAD` over `comparison.json`, `run.mjs`, `package.json`, `bin/ocelli.sh`, `README.md`, `examples`, `docs`, `scripts`, `ci`, `tools/oracle/src` | now | exit 0 |
| GLM's round report | tail of `glm-F-223-fix3.log` | prior receipt | its command table agrees with mine. Its red evidence for D2 and S1 matches what I reproduced |

## Defects

### D1, the schema publishes a measured bias at an informative fraction of 0, which is the placeholder the ruling removed
**Where**: `tools/bench/src/comparison.mjs:1292-1305`. The claims are in the doc at `:1280-1282`, "a finite number when it was measured and null when it was not", and in the test comment at `comparison_test.mjs:869`, "does not print a placeholder zero beside a zero fraction". The pair_fixture doc at `:281-283` says the ruling refused "the best possible bias beside an informative fraction of zero".
**What**: The bias check refuses null except at fraction 0. It never refuses a number at fraction 0. A verdict with `signed_mean_bias_informative: 0`, `informative_fraction: 0`, `low_information: true` and `passes: false` passes `publishedRecord`. Both renderers then print `- signed mean bias over the informative region: 0.0000` above `- informative fraction: 0.0000`. That is the output the null ruling exists to prevent.
**Why it is wrong**: The ruling at plan line 409 is an equivalence. When the fraction is 0 the bias "is absent from the measurement". The closed schema now enforces only one direction of it. This is D14 (claim measured divergence) and microscope section 3, because the doc sentence describes a constraint the schema does not apply.
**Evidence**: the probe `bias 0 at f=0` above, run against a copy of HEAD.
**Scope and regression**: This affects the published schema and both renderers. Rust cannot produce this record, since a fraction of 0 always yields `None`, so only a hand-edited or corrupted record reaches it. That is the case pass 3's D2 treated as a defect, and this is its mirror. The smallest fix refuses a numeric bias when `informative_fraction === 0`, with one assertion beside the 0.05 case.

### D2, "which the ruling refused in advance" is still false
**Where**: `tools/oracle/tests/pair_fixture.rs:282`
**What**: Pass 3's D2 named "the number the plan refused in advance" as false, because nothing refused the placeholder before the post-review ruling. The fix changed "plan" to "ruling" and kept "in advance". The placeholder landed in `120228e` at 16:38. The ruling landed in `44398d5` at 16:53, after pass 2 had reviewed that placeholder. The ruling refused it afterwards, not in advance.
**Why it is wrong**: microscope section 3. It is the same finding, one word changed, now surviving its second pass.
**Evidence**: `git log --format="%h %ad %s" --date=iso 73b68ad..HEAD`, and `git show 120228e:tools/oracle/src/tolerance.rs`, which holds `signed_mean_diff: 0.0` for the empty region.
**Scope and regression**: prose only. Delete "in advance", or the whole clause. No test is needed.

## Smells

### S1, the `low_information` half of the new null refusal is undriven
**Where**: `comparison.mjs:1293`, `|| pair.low_information !== true`, and the clause of the message that states it
**What**: Deleting the clause leaves all 37 tests green. Every test that sends a null bias either has a non-zero fraction, so the fraction clause throws first, or has fraction 0 with `low_information` true. Nothing sends fraction 0 with `low_information` false. With the clause gone, that record publishes and renders "low information: no" beside "not measured".
**Why it is wrong**: microscope section 4. It is the class that pass 2's S1 and pass 3's S1 closed, reintroduced by the remediation of pass 3's D2.
**Scope and regression**: This affects the published schema. One option is a single assertion at fraction 0 with `low_information` false. The other is to delete the clause and its half of the message, because the fraction clause alone carries the ruling. The lead should choose, since pass 3 offered the clause as optional.

### S2, the sweep finds 34 refusal sites that no test executes, and 5 more whose test a later throw satisfies
**Where**: `tools/bench/src/comparison.mjs`, against the `bench.comparison` entry in `scripts/guards/catalogue.py`, which claims every site (`"*"`) as covered by `comparison_test.mjs`. Also `tools/oracle/src/bin/ocelli-compare.rs:447-449` and `:183-187`.
**What**: Each site below was neutralised to `void new Error` on its own, and all 37 tests stayed green. Line coverage confirms that the first group is never reached.
- Never executed, in slice ACs:
  - AC13, the published schema: `1049` (not an object), `1129` (no renderer string), `1157` (a non-finite default), `1173` (a workload state that is not a state), `1253` (a pair verdict missing a key).
  - AC12, the deny scan: `946`, `949`, `1006` (input refusals).
  - AC4, selection: `769`, `777`, `781`, `787` (US range), `874`.
  - AC8 and AC9: `599` (a pid printed twice), `659`, `665`, `704`.
  - AC5: `445`.
- Never executed, in `loadComparison`'s vocabulary check: `292`, `311`, `321`, `324`, `334`, `355`, `390`, `396`. The test `loadComparison refuses a file that disagrees` drives other branches.
- Never executed, argument contracts: `179`, `522`, `525`, `584`, `916`, `1682`, `1710`, `1713`.
- Reached, but the test passes whichever site throws: `773`, `1090`, `1166`, `1200`, `1209`. For example, `/bogus/` matches both the unknown-workload refusal and the count refusal behind it.
- Rust: `pair_requires_both_frames_explicitly` drives only the missing `--reference`. With the `--candidate` refusal deleted, all 19 bin tests pass. With `--width abc` parsed to 0 instead of refused, all 19 pass too.
**Why it is wrong**: microscope section 4, a guard that is present and never reached. The census reports "0 watched by nothing" because it checks file-level `covered_by` and does not run each site. Passes 1 to 3 mutated chosen sites and never measured coverage, so this went unseen. Pass 2's S1 treated seven such sites as a smell, and these are the same class.
**Scope and regression**: No present behaviour is wrong. Each site is a refusal that a future edit can drop with nothing going red. The smallest meaningful fix is one negative assertion per slice-AC site, with a regex specific enough that only that site satisfies it. The lead may rule the argument-contract group out of scope. In Rust, either make the test drive the missing candidate or rename it.

### S3, the summary's lines for an unavailable or invalid reason are never rendered by any test
**Where**: `comparison.mjs:1410-1418` (`measurementNotes`) and `:1542-1547` (`workloadsSection`)
**What**: Replacing either `notes.push` of a reason with `void 0` leaves all 37 tests green. Coverage marks both blocks, and the invalid-trials note at `:1414-1418`, as never executed. No fixture renders a metric cell or workload whose state is `unavailable` or `invalid`.
**Why it is wrong**: The plan's state table makes "`unavailable` with reason" one of the three outcomes a published metric can have. AC18 requires it of the equipped run. D14 makes the reason the measured statement in place of a number. A renderer that drops the reason would publish a bare "unavailable" with nothing going red.
**Scope and regression**: This affects the tracked summary and the plain report. The smallest fix is one fixture with one unavailable workload, one unavailable cell and one invalid cell, asserting each reason line.

## Nitpicks

- N1. `informative_fraction !== 0` admits `-0`, because `-0 !== 0` is false in JS. Rust divides a `u64` count, so it cannot produce `-0`.
- N2. In the bias check, the `typeof ... !== "number"` half is redundant beside `!Number.isFinite(..)`. The fraction loop has the same shape. Deleting it changes nothing.
- N3. The plan's AC8 row names a test, `memory counts every process in the inventory`, that does not exist. The coverage is real but split across three tests with other names. Rename one, or amend the row at completion.
- N4. Never executed and harmless: `summarise`'s single-element quantile arm (`:467`) and `ratioText`'s `rounded === 0` arm (`:1445`).
- N5. Carried: pass 3 N3 (`passes: true` beside `low_information: true` publishes) and pass 2 N5, N6 and N7. N6 is the summary's unconditional "Every timing figure above was taken in a run whose pixel check passed". The new null test renders it beside "the verdict passed: no". I confirmed that sentence on the null, fraction-0 published record. It stays carried for the run round's `--publish` refusal.

## Verified clean

- Every sentence changed in `c73dc8b` and `694a19c` was checked. The `scanForDenied` doc and the pipe test comment are true, as executed above. The `validatePairVerdict` doc and inline comment, the error message, the null test comment, the new assertions and their messages, the finite-test comment and the `pair_fixture.rs` doc are true, except "in advance" (D2) and the measured-bias sentence that D1 makes false. The plan struct matches `tolerance.rs`.
- The new regex `/informative fraction is exactly 0/` matches the only message that can throw there. Both null assertions go red when their own clause is deleted, except as S1 records.
- No `as` cast and no pixel arithmetic changed in this range. `pair_fixture.rs` changed only in doc comments.
- No added line carries a prose `;` or an em-dash. The census budget and runbook did not change, and both checks pass. The throw count is 91 at both `4b263ba` and `c73dc8b`.
- The out-of-scope files are untouched since `4b263ba`.

## Limitations and handoff

- No browser, Playwright, oracle or GPU job was run. AC7's flip, AC11 and AC15 to AC20 remain with the run round.
- The two Rust scratch builds used `CARGO_TARGET_DIR=<wt>/target`. The worktree's own tests were re-run afterwards, rebuilt and green.
- I deleted `target/mp4` and `target/mp4rs`. HEAD is still `c73dc8b`, `git diff --quiet HEAD` exits 0, and this file is the only addition. Scratch notes stay in the session scratchpad, outside the tree.
- Next owner: the implementer fixes D1, D2, S1 and S3. For S2, the lead rules on the argument-contract group, then the implementer adds the slice-AC assertions. A fresh independent pass 5 then reviews the changed material.
