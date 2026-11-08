# F-223 review, pass 5

**Reviewed**: `git diff c73dc8b 12539b7` on `work/f-223-xai`, the pure-function slice only (AC4 to AC10, AC12 to AC14). HEAD `12539b774de48117e9632b7020293eb15883d9f0`. One commit, `12539b7`, touching `tools/bench/src/comparison.mjs`, `tools/bench/tests/comparison_test.mjs`, `tools/oracle/src/bin/ocelli-compare.rs` (inside `#[cfg(test)] mod argument_tests` only), `tools/oracle/tests/pair_fixture.rs` (doc comment only) and `ci/guard-probe-budget.json` (one line). The worktree was clean apart from the untracked pass 1 to pass 4 reports, before and after this pass.
**Author / reviewer**: GLM-5.3 in opencode wrote the change (its log is `glm-F-223-fix4b.log`). The commit carries the operator's name as git author, so the log is the evidence of who wrote it. Claude Opus 5.5 reviewed it as an independent subagent. I wrote none of the reviewed material and none of passes 1 to 4.
**Result**: 1 defect, 1 smell, 6 nitpicks

## Acceptance challenged

- **D1 of pass 4 (a number at fraction 0).** I probed `publishedRecord` on a scratch copy of HEAD with a matrix I chose, not GLM's:
  - bias 0 at fraction 0, `low_information` true: throws "a number beside an informative fraction of exactly 0".
  - bias 0.5 at fraction `-0`: throws the same.
  - null at fraction `-0` with `low_information` true: publishes. The summary prints "not measured" beside "informative fraction: 0.0000".
  - bias NaN at fraction 0: throws "not a finite number". The finite check runs first.
  - bias 0.1 at fraction `"0"`: throws "informative_fraction is not a finite number".
  - Changing the new branch to `< 0` or to `Object.is(.., -0)` turned exactly `the pair verdict refuses a number beside a zero informative fraction` red.
  - **Closed.** The new doc sentence goes further than the code, though (D1 here).
- **D2 of pass 4 ("in advance").** `pair_fixture.rs:282` now reads "which the ruling refused". `grep -rn "in advance"` over `tools/bench` and `tools/oracle` leaves three hits, `comparison.mjs:438`, `:1347` and `:1379`. All three say the plan refused a statistic over a partial sample in advance. `git show 83bd184` holds the plan's "A metric with fewer valid trials than N is `invalid` and has no median" at 10:53 on 2026-11-05. That is before `bdec2c9` (15:07 on 2026-11-06), so those three are true. **Closed.**
- **S1 of pass 4 (the `low_information` clause).** Null at fraction 0 with `low_information` false throws. Deleting the clause turned exactly `a null bias beside a zero fraction needs low_information true` red. **Closed.**
- **S2 of pass 4 (undriven refusals).**
  - JS: `node --test --experimental-test-coverage` over `comparison.mjs` reports 99.42 percent of lines. The uncovered lines are 467-468, 955-956, 1011-1012, 1455-1456 and 1729-1730. None of them is a throw.
  - I then rewrote each of the 92 `throw new Error` sites in turn to `void new Error` in a scratch copy and ran the suite under the TAP reporter. Every site turns at least one test red. 90 sites turn exactly one test red. Two turn two tests red: `1301` (the null test and the `low_information` test) and `1336` (the closed-schema test and the boolean test).
  - The inverse map confirms the comment above the new block: each of its 14 tests goes red for exactly one site.
  - The five loose regexes now fail at their own site: `773`, `1090`, `1166`, `1200` and `1209` each turn exactly one test red. So no later throw satisfies them.
  - Rust: I deleted `pair`'s `--candidate` refusal, replaced each `--width` and `--height` parse refusal with `unwrap_or(0)`, and deleted the `--reference` refusal. Each turned exactly one test red, with 20 passing. **Closed for the ruled sites.** Adjacent Rust refusals are S1 here.
- **S3 of pass 4 (reason lines).** I deleted each of the three pushes in turn: the unavailable-cell reason, the invalid note (its `if` changed to `if (false)`) and the workload reason. Each turned exactly `the reports render the reason lines an unmeasured cell carries` red. The test loops over `renderSummary(published)` and `renderReport(run)`, and both go through `sharedBody`. **Closed.**
- **N3 of pass 4.** A test named `memory counts every process in the inventory` now exists (`comparison_test.mjs:139`). It is the renamed tree-walk test. See N2 here.
- **N1 reasoning (`-0`).** `-0 === 0` holds, `JSON.stringify(-0)` is `"0"`, and the renderer prints `0.0000`. The probes above show `-0` treated as 0 in both directions. The ruling says "an informative fraction of exactly 0", and `-0` is that value. Switching to `Object.is` would refuse a null beside `-0`, which the ruling admits. **I agree with GLM's reasoning.**
- **Census re-record (92 to 93).** I ran `discover()` and `match_sites()` against HEAD. `bench.comparison` claims 93 sites, all in `comparison.mjs`:
  - 90 distinct `js-throw` messages from the 92 raw throws. `773` and `781` share the message "`${where} is missing ${field}`". `958` and `1014` share "the deny set's `${kind}` is not a list".
  - 3 `py-return-list` matches at `1499`, `1587` and `1608`. These are renderer `return [ "## ..."` blocks, not refusals. See N5.
  - At `c73dc8b` the raw count was 91 and the recorded count 92. The difference between the two commits is the one D1 throw at `1312`. **One real new site, confirmed.**
- **AC map.** It is unchanged from pass 4 except AC8's named test, which now exists by name.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `cargo fmt --all --check` | worktree at 12539b7 | now | 0 |
| `cargo clippy -p ocelli-oracle --all-targets -- -D warnings` | same, `CARGO_TARGET_DIR=<wt>/target`, the two touched Rust files `touch`ed to force a re-check | now | 0 |
| `cargo test -p ocelli-oracle --test pair_fixture --test tolerance_fixture` | same | now, and again after the scratch builds | 0, 10 + 10 pass |
| `cargo test -p ocelli-oracle --bin ocelli-compare` | same | now, and again after the scratch builds | 0, 21 pass |
| `node --test tools/bench/tests/comparison_test.mjs` | same | now, and again at the end | 0, 54 pass |
| `npm run lint` | worktree | now | 0 |
| `guard_census.py`, `--check-runbook` | same | now | 0, 0, "0 watched by nothing" |
| `prose_check.py`, `source_provenance_check.py` | same | now | 0 (864 files), 0 (1710 files) |
| Coverage of `comparison.mjs` | `git archive HEAD tools/bench scripts` under `target/mp5`, deleted | now | 99.42 percent of lines, no throw line uncovered |
| 92 single-site neutralisations, TAP reporter | same copy, file `cmp`'d against the worktree after the run | now | 0 sites green, 90 with one red test, 2 with two |
| Targeted JS mutations: S1 clause, the D1 branch two ways, the three reason pushes, bias `isFinite` to `typeof`, fraction `isFinite` to `isNaN` | same copy | now | all red except fraction `isFinite` to `isNaN` (N3) |
| Probe matrix through `publishedRecord` and `renderSummary` | same copy | now | as listed above, plus D1 |
| Census site list via `discover()` and `match_sites()` | worktree | now | 93 sites, composition above |
| The three `py-return-list` headings each renamed | same copy | now | `## The machine` 0 red, `## The pixel agreement check` 0 red, `## What the numbers do not show` 1 red |
| Rust mutations (6) plus the noun mutation | `git archive HEAD` under `target/mp5rs`, deleted, shared `CARGO_TARGET_DIR` | now | ruled sites red, missing-value refusals green (S1), noun green (N4) |
| Out of scope | `git diff --quiet c73dc8b HEAD` over `comparison.json`, `run.mjs`, `package.json`, `bin`, `README.md`, `examples`, `docs`, `scripts`, `tolerance.rs`, `lib.rs`, `tools/oracle/page`, `.claude/plans`, `AGENTS.md`, `CHANGELOG.md` | now | exit 0. `ci/` changed only the `bench.comparison` count |
| Added lines scanned for an em-dash, an en-dash and a prose `;` in comments and strings | `git diff c73dc8b HEAD` | now | none |
| GLM's round report | tail of `glm-F-223-fix4b.log` | prior receipt | its table matches my sweep site for site. It says "16 new tests" and the diff adds 17 (37 to 54) |

## Defects

### D1, the new doc says the bias is a number "exactly when the informative fraction is above zero", and a negative fraction publishes a number
**Where**: `tools/bench/src/comparison.mjs:1281-1283`, in the `validatePairVerdict` doc. The fraction checks are at `:1318-1322`.
**What**: The rewritten doc states an equivalence: the bias is "a finite number exactly when the informative fraction is above zero, and null exactly when the fraction is 0". The code refuses a number only at `=== 0`. The fraction check is only `Number.isFinite`. So a verdict with `informative_fraction: -0.5` and bias `0.1` passes `publishedRecord`, and the summary prints "signed mean bias over the informative region: 0.1000" beside "informative fraction: -0.5000". A fraction of `1.5` publishes too, and so does a `within_one_lsb_fraction` of `1.5`.
**Why it is wrong**: microscope section 3, a doc sentence describing a constraint the schema does not apply. It is new in this diff. Before, the doc said only "a finite number when it was measured", which made no claim about the fraction's sign. A fraction is a ratio of pixel counts, so it lies in [0, 1] (HLD 25.1's informative region over the image). The closed schema's purpose, AC13, is that the published record carries only what the run could have measured.
**Evidence**: the probes `bias 0.1 at f=-0.5` and `bias 0.1 at f=1.5`, run through `publishedRecord` and `renderSummary` on a copy of HEAD. Both published.
**Scope and regression**: This affects the published schema's doc and, if the lead takes the second option below, the schema. Rust cannot produce a fraction outside [0, 1], so only a hand-edited or corrupted record reaches this. That is the class passes 3 and 4 treated as defects. There are two smallest fixes:
- Reword the sentence to "a finite number when the fraction is not 0". This deletes the false half and needs no test.
- Refuse either fraction outside [0, 1], with one assertion per fraction. That adds sites, so the census would move again.
The lead should choose.

## Smells

### S1, `pair`'s missing-value refusals for `--width` and `--height` are undriven
**Where**: `tools/oracle/src/bin/ocelli-compare.rs:180-182` and `:190-192`, `.ok_or_else(|| "--width wants a pixel count" ..)` and its `--height` twin. F-223 added both.
**What**: I replaced each refusal with a default value of `"0"` in a scratch workspace. All 21 bin tests passed each time. No test passes `--width` or `--height` as the last argument with no value.
**Why it is wrong**: microscope section 4. This is the class pass 4's S2 named for the two Rust sites beside these. The lead ruled the argument-contract group in scope for the JS file. The Rust ruling named only the candidate refusal and the non-numeric parse. These two sit beside the parse refusals in the same match arms. One test can drive each: `parsed(["pair", "--width"])` returns `Err` containing "wants a pixel count".
**Scope and regression**: No present behaviour is wrong. It affects `ocelli-compare pair`'s argument parsing only. The Rust bin is outside the census's shapes, so nothing else counts these. The lead may rule them out of scope instead.

## Nitpicks

- N1. Census message collapse. `773` and `781` share one census identity, and so do `958` and `1014`, so the census cannot see either second site removed. Both are driven by a test today, so nothing is lost now. The fix belongs to the census owner or to a reword that gives each site its own words.
- N2. The renamed test `memory counts every process in the inventory` calls only `parseProcessTree` and no memory function. The ruling allowed a rename and the inventory is what memory is counted over. Still, `parseFootprint` and `memoryFigures` stay in two other tests, so the plan's AC8 row still maps to three tests.
- N3. Changing the fraction loop's `!Number.isFinite(pair[key])` to `Number.isNaN(pair[key])` leaves all 54 green. The only test, `the pair verdict refuses a fraction that is not finite`, sends `NaN`, so an `Infinity` fraction is never sent. This predates the diff (`4b263ba`), but the line changed here (N2 of pass 4).
- N4. The `--reference` and `--candidate` missing-value messages choose "frame file" for `pair` and "directory" otherwise. Collapsing both arms to `"directory"` leaves all 21 green. It is message wording and not a refusal.
- N5. The census applies `py-return-list` (`return\s*\[\s*f?["']`) to this `.mjs` file. So 3 of `bench.comparison`'s 93 recorded sites are renderer headings (`1499`, `1587`, `1608`) and not refusals. Renaming `## The machine` or `## The pixel agreement check` leaves every test green. This is outside F-223's write set (`scripts/guards/discover.py`). It means the per-entry count moves when a section is added, and that the census's "covered" for those three is vacuous.
- N6. An `invalid` cell may carry a `reason` under the schema (`validateMetricCell`), and the JSON publishes it. `measurementNotes` renders only the fixed "fewer valid trials" line, so the recorded reason never reaches either renderer. This predates the diff. I have not confirmed it is unintended.
- Carried: pass 4 N4 (two harmless non-throw arms, still uncovered at `467` and `1455`) and N5 (pass 3 N3, pass 2 N5 to N7, for the run round).

## Verified clean

- Every sentence changed in `12539b7` was checked. That covers the `validatePairVerdict` doc and inline comment, the D1 error message, the `pair_fixture.rs` doc and every new test's name and assertion message. They are true, except the doc's "exactly when the informative fraction is above zero" (D1). Among the messages:
  - "a block without the mode is not the block fact 15 records" matches `tierSentence`'s `headed` check.
  - "the seven keys" matches `PAIR_VERDICT_KEYS`.
  - "65536 does not fit" matches `MAX_US` and PS3.5's US.
  - "a state the renderers do not know would publish as its raw word" matches `workloadsSection`.
  - The comment "Each test drives exactly one site ..." matches the inverse sweep map.
- The comment at `comparison_test.mjs:914` is now true for the schema. A number at fraction 0 cannot reach a renderer through `publishedRecord`.
- No `as` cast and no pixel arithmetic changed. The Rust change is in the test module only.
- No added line carries a prose `;`, an em-dash or an en-dash.
- The census budget moved by exactly one real refusal (`1312`). `uncovered` is 0 and `sweep_complete` is true at both commits.
- The out-of-scope files are untouched since `c73dc8b`.

## Limitations and handoff

- No browser, Playwright, oracle or GPU job was run. AC7's flip, AC11 and AC15 to AC20 remain with the run round.
- The Rust scratch builds used `CARGO_TARGET_DIR=<wt>/target`. The worktree's own tests were re-run afterwards, and were green.
- I deleted `target/mp5` and `target/mp5rs`. HEAD is still `12539b7`, `git diff --quiet HEAD` exits 0, and this file is the only addition. The sweep script and its JSON stay in the session scratchpad, outside the tree.
- Next owner: the lead chooses D1's form (reword or range check) and rules S1 in or out. The implementer then fixes them, and a fresh independent pass 6 reviews the changed material.
