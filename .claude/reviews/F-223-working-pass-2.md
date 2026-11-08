# F-223 review, pass 2

**Reviewed**: `git diff bdec2c9 120228e` on `work/f-223-xai`, the pure-function slice only (AC4 to AC10, AC12 to AC14 and the pure parts of the plan's facts). HEAD `120228ede18572dff6c7124ea2394a2c67bd07e0`. The worktree was clean apart from the untracked pass 1 report, before and after this pass. Out of scope: `comparison_run.mjs`, `run.mjs` wiring, `package.json`, `bin/ocelli.sh`, `bench_check.py`, the docs other than the generated runbook rows. I checked that `120228e` touches none of them. It changes eight files: the four code and test files, the catalogue, the budget, the runbook and `pair_fixture.rs`.
**Author / reviewer**: GLM-5.3 in opencode wrote `120228e`. Claude Opus 5.5 reviewed it as an independent subagent. I wrote none of the reviewed material and none of pass 1's.
**Result**: 3 defects, 2 smells, 7 nitpicks

## Acceptance challenged

- **D1 of pass 1 (AC8, parseFootprint).** I ran `/usr/bin/footprint -f bytes -p <pid> -p <pid>` myself, once on two `sleep 60` processes (73030 and 73031) and once on one sleep plus the shell (73028). I stopped both sleeps with `kill <pid>`. Both texts went through `parseFootprint` and gave the right per-pid figures, for example 73028 at 2638280 with a peak of 2671048. The `Shared with` blocks did not re-key the parse. A process name with spaces and brackets in it parsed too. The fixture is consistent with real output in two ways. First, process-b's category rows match my own `sleep` capture line for line, with the differences confined to rows that depend on which pages the other process shares. Second, the summary table is internally consistent: Malloc Metadata 606208 + 557056 = 1163264, page table 328136 + 180464 = 508600, and dirty total 1950152 + 1196272 + 32768 shared = 3179192. The `Shared Cache` UUID is the one my capture printed. My judgement is that the fixture is real output with only the names changed. I cannot reproduce the byte-identity claim, because the capture it was compared against was not kept. **Closed.**
- **D2 of pass 1 (AC13).** I replayed the probes. `machine.hostname`, `versions.series_description`, a string `passes`, a string `low_information`, `max_abs_diff` 256, an `informative_fraction` of NaN, a stray pair key, a stray workload key and a one-sided cell all throw. **Closed for the probes.** Seven of the new type refusals have no test that drives them (S1).
- **D3 of pass 1 (AC10).** I ran the real `ocelli-compare pair` binary on frame files I wrote, not on the test's own helper:
  - All black against all black, and all white against all white: exit 1, `informative_fraction` 0.0, `low_information` true, `passes` false, nothing on stderr.
  - Black against white: `count_over_max` 100, `max_abs_diff` 255, bias 255.0, false.
  - The test's +3 pair: `count_over_max` 1, bias 0.75, within-one 0.75. This confirms the test's comment.
  
  **Closed.** The placeholder bias is S2.
- **D4 of pass 1.** A refused-run report with `passes: false` now prints "No timing figure was recorded" and does not print the passing sentence. Forcing `refused` to false went red (J18). **Closed.** Under N6 I record that the sentence is keyed on the run state, not on the verdict's own `passes`.
- **D5 of pass 1.** `guard_census.py` and `--check-runbook` both exit 0. I cloned the worktree into `target/f223-clone` at `120228e`, then ran `--record` and `--render-runbook`. Neither left a diff, so the budget figure `bench.comparison: 90` and runbook rows 579 and 580 are exactly what the commands produce. The entry is `file="tools/bench/src/comparison.mjs"`, `claims=("*",)`, so it claims that file's refusal sites and no others. **Closed.**
- **D6 and S2 of pass 1.** `PairVerdict` derives `Serialize` and `pair_json` serialises it. The key-set test went red under a `#[serde(rename)]` (R7), and `Ok(true)` went red too (R8). The doc's claim that stdout carries JSON only on a verdict, and stderr only on an error, holds against the real binary: a size error and the `--out` refusal each printed 0 bytes on stdout. **Closed.**
- **D7 of pass 1.** `parseProcessTree("  200   100\n", 100)` is pinned, and the new sentence matches it. **Closed.**
- **S1 of pass 1, checked against 25.1's own wording.** I read `docs/hld/22-testing-and-tolerance.md` line 42, "≤ 1 LSB on at least 99.9% of pixels", which is `>=`. Line 44 says "within 0.1 of one display code", which is `<=`, and the constant's doc reads "within" as inclusive. The floor's `>=` comes from plan Approach item 4 and not from 25.1. Floor `<` changed to `<=` went red (R1). Bias `<=` changed to `<` went red (R2). The within-one bound `>=` changed to `>` went red, but only in `tolerance_fixture`'s `exactly_999_permille_within_one_lsb_passes` (R3), not in the new fixture. The new fixture's doc overstates what it pins (D3). **Closed for the boundary.**
- **S3 of pass 1.** The escaped forms are matched, and dropping that match went red (J4). The structured walk finds nested and array values, and flattening it went red (J5). Tab, control and non-ASCII values match through their escaped form. The walk does not scan keys. Its doc says "string values", so that is a scope statement, not a defect. The renderer's own transform is missed (D2).
- **S4 of pass 1, against fact 15.** The `browser` block is closed and typed. `tierSentence` names the reported renderer string and says "The browser ran headed." or "headless.". A SwiftShader renderer without `--software` is refused, case-insensitively, which went red under J9 and J10. With `--software`, the first paragraph of both the summary and the report names SwiftShader and still carries the fixed architecture text. Without the flag, the first paragraph is the architecture text alone. **Closed.**
- **N1 to N5 of pass 1.** N1 is closed (J26 red). N3 is closed: `pair --out x` gives "writes nothing to disk", `pair --reference` gives "wants a frame file" and `gate --reference` gives "wants a directory". The noun is untested (N5 here). N4 is closed and verified on the binary. N5 is closed, but it opened D2. N2 is not closed in its zero-denominator half (D1).

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `cargo fmt --all --check` | worktree at 120228e | now | 0 |
| `cargo clippy -p ocelli-oracle --all-targets -- -D warnings` | same, `CARGO_TARGET_DIR=<wt>/target`, sources touched to force a re-check | now | 0 |
| `cargo test -p ocelli-oracle --test pair_fixture --test tolerance_fixture` | same | now | 0, 9 + 10 pass |
| `cargo test -p ocelli-oracle --bin ocelli-compare` | same | now | 0, 19 pass |
| `node --test tools/bench/tests/comparison_test.mjs` | same | now | 0, 27 pass |
| `python3 scripts/guard_census.py`, `--check-runbook` | same | now | 0, 0 |
| `--record`, `--render-runbook` in a clone at 120228e | `target/f223-clone`, deleted | now | 0, 0, no diff |
| `python3 scripts/prose_check.py` | same | now | 0, 864 files |
| `python3 scripts/source_provenance_check.py` | same | now | 0, 1710 files |
| `npm run lint` | worktree | now | **not runnable**: the worktree has no `node_modules`, `sh: eslint: command not found` |
| eslint proxy: main checkout's `node_modules/.bin/eslint --config ../ocelli/eslint.config.js` on `comparison.mjs` and `comparison_test.mjs` | the main checkout's config differs from the worktree's only by F-216's `viewer_work.mjs` block, which does not match these files | now | 0 |
| `/usr/bin/footprint -f bytes -p 73030 -p 73031` and `-p 73030 -p 73028`, fed to `parseFootprint` | reviewer's own `sleep 60` processes and shell, stopped by `kill <pid>` | now | both parse, per-pid figures correct, `Shared with` not re-keyed. The summary is 2425312 against a per-pid sum of 2458080 |
| `ocelli-compare pair` on hand-written `.raw` files | scratchpad: grey/+3, black/black, white/white, black/white, 3x2 size error, `--out` | now | as listed under D3 and D6 above |
| Rust mutations R1 to R13 | scratch copy `target/f223-scratch`, deleted | now | R1 to R3, R5 to R8 and R10 to R12 red. R4 (placeholder `passes: true`) green, as the code documents. R9 (unwire `refuse_stray_pair_arguments` from `run`) green under test and red under clippy's `dead_code`. R13 (noun) green |
| JS mutations J1 to J27 | same scratch copy | now | 19 red. Green: J2 (summary reset), J12 (`max_abs_diff > 255`), J13 (finite check), J14 (`low_information` boolean), J15 (selected with reason), J16 (invalid reason type), J17 (`n` check), J27 (machine string types) |
| `publishedRecord`, `renderSummary`, `renderReport`, `scanForDenied`, `scanRecordForDenied` and `ratioText` probes | inline node scripts in the scratchpad | now | see the findings |
| GLM's round report | tail of `glm-F-223-fix1b.log` | prior receipt | the command table agrees with mine. "Dropping validatePairVerdict ... red" holds for the whole function and not for its clauses (S1) |

A note on method. Restoring a mutated file with `cp` and then `mv` gives it an mtime older than the mutated build, so cargo does not rebuild. My first Rust run was stale from R8 onwards for that reason. Every result above comes from a rerun that `touch`es each file after restoring it, and starts from a clean baseline.

## Defects

### D1, `ratioText`'s doc and test contradict its behaviour
**Where**: `tools/bench/src/comparison.mjs:1389-1408`, `tools/bench/tests/comparison_test.mjs:1034-1058`
**What**: The doc says a zero denominator "makes the ratio infinite, which is not a figure the table can carry at all". The function then returns `"Infinity"`, and the test asserts `| Infinity |` with the message "a zero denominator is stated, not rendered as NaN or an exponent". The old `(5/0).toPrecision(3)` also printed `Infinity`, so this half of pass 1's N2 is unchanged behaviour presented as a fix, and its assertion passes on the pre-fix code. Separately, "three significant figures" is false wherever rounding crosses a decade. The probes printed `10.00` for 9.996, `1.000` for 0.99951 and `1000` for 999.6.
**Why it is wrong**: microscope section 3, a claim in prose that is false. CLAUDE.md lists every rounding decision as something a human must check.
**Evidence**: the probe `ratio(5, 0)` gives `| Infinity |`. Running `node -e 'console.log((5/0).toPrecision(3))'` gives `Infinity`. The decade-crossing probes are listed above.
**Scope and regression**: this affects the rendered summary and report. Delete the "not a figure the table can carry" clause or render something else for a zero denominator. Take the decimals from the exponent of the rounded value. Then add one fixture at 9.996 that expects `10.0`.

### D2, the deny scan's doc names the wrong renderer transform, and the scan misses the real one
**Where**: `tools/bench/src/comparison.mjs:978-982`, test comment `tools/bench/tests/comparison_test.mjs:560-563`, against `escapeCell` at `comparison.mjs:1472`
**What**: Both comments say that a value carrying a backslash or a double quote "is changed by both" the serialisation and the renderer. The renderer changes neither. The only thing it rewrites is `|`, to `\|`, through `escapeCell`, which arrived in this same commit. `scanForDenied` matches the raw form and the JSON-escaped form, and neither of those is the Markdown form.
**Why it is wrong**: microscope section 3, and plan fact 13, "Every output ... is scanned against the deny set before it is written".
**Evidence**: `scanForDenied(renderSummary(publishedRecord(run with playwright "A|B")), { physician: ["A|B"] })` returns `[]`.
**Scope and regression**: the gap is reachable only through the four closed version values, and `scanRecordForDenied` over the record would catch it if the run round calls it. Correct both sentences. Then either match the `escapeCell` form as well, or state that the Markdown is covered by the record walk. Add one fixture with a pipe.

### D3, the boundary fixture's doc says every bound sits at its limit
**Where**: `tools/oracle/tests/pair_fixture.rs:270`
**What**: The doc says "every bound in the verdict is AT its own limit". The fixture's within-one fraction is 1.0 against a limit of 0.999, and its `max_abs_diff` is 1 against a limit of 2. Only the floor (0.10) and the bias (0.1) are at their limits, and the doc's own last sentence names only those two. The 99.9% boundary is pinned in `tolerance_fixture.rs` (R3), not here.
**Why it is wrong**: microscope section 3.
**Evidence**: the fixture's own assertions, `within_one_lsb_fraction == 1.0` and `max_abs_diff == 1`.
**Scope and regression**: this is a doc comment only. Narrow it to "the floor and the bias bound are at their limits".

## Smells

### S1, seven new type refusals are driven by no test, and the census counts them as covered
**Where**: `comparison.mjs:1098` (machine string types), `1177` (selected with reason), `1278-1298` (`validatePairVerdict`'s finite check, `max_abs_diff > 255` and `low_information` boolean), `1315` and `1355-1357` (`validateMetricCell`'s `n` check and its invalid reason type)
**What**: Each of the mutations J12, J13, J14, J15, J16, J17 and J27 deletes one of these refusals and leaves all 27 tests green. Pass 1's D2 fix asked for "one negative test for each". The new `bench.comparison` catalogue entry names `comparison_test.mjs` as `covered_by` for every site in the file, so the census reports these sites as watched.
**Why it is wrong**: microscope section 4, a refusal that is present and that nothing executes. CLAUDE.md says tests must fail if the code is wrong.
**Scope and regression**: this affects the published record's schema. The smallest regression is one assertion per refusal in "the published record has a closed schema".

### S2, an all-clipped pair publishes and renders a bias that was never measured
**Where**: `tools/oracle/src/tolerance.rs:364-376`, rendered by `pixelSection` at `comparison.mjs` ("signed mean bias over the informative region")
**What**: When the informative region is empty, the verdict writes `signed_mean_bias_informative: 0.0`. The real binary prints it into the JSON, and the plain report renders it as `0.0000`, which is the best possible value, beside `informative fraction: 0.0000`. The site's own comment says "inventing a mean for it would invent evidence". Plan step 5 says a refused verdict "writes the verdict numbers", and this zero is not one of them.
**Why it is wrong**: D14, which says to claim measured divergence. A reader of a `refused_pixels` report sees a perfect bias for a frame on which nothing was measured. `passes` is false, so nothing publishes, but the ignored record and the report do carry the number.
**Scope and regression**: this affects the ignored JSON and the plain report. The smallest fix is `Option<f64>` serialised as `null`, with the report rendering "none, no informative pixel" and the JS type check admitting `null` only when `informative_fraction` is 0. The other option is a report line conditioned on a fraction of 0. Add one assertion on the all-clipped fixture.

## Nitpicks

- N1. `pair_fixture.rs:320-322` gains a blank line between an unrelated test's doc comment and its `#[test]`.
- N2. Two prose semicolons in new doc comments, at `comparison.mjs:568` and `1303`. They are outside `prose_check`'s scope, but CLAUDE.md says "tracked prose" and the rest of `tools/bench/src` has none.
- N3. "Two of this repository's own processes" (`comparison.mjs:563`, test line 155) describes a shell and a `sleep`, and neither belongs to this repository.
- N4. Branches with no observable effect: `ratioText`'s NaN and `-Infinity` arms are unreachable, because `separated` is false at 0/0 and ratios are positive. The `Summary Footprint` reset changes nothing on real output (J2 green).
- N5. The `--reference` and `--candidate` noun follows the command, and nothing tests it (R13 green).
- N6. Two things for the run round. `renderSummary` passes `refused = false` unconditionally and `publishedRecord` admits `passes: false`, so the passing sentence in the summary relies only on `--publish`'s refusal (plan line 130). Keying it on `verdict.passes` would make it true by construction. `renderReport` drops the tier sentence when `browser` is absent, and throws when `browser` carries two strings, which is the shape a `prepared` refusal "both strings recorded" might take.
- N7. The catalogue's `refuses` sentence, copied into runbook row 579, does not mention the SwiftShader-without-flag refusal, the one-sided cell or the deny-scan input refusals.

## Verified clean

- No `as` cast was added. The Rust diff adds a `Serialize` derive, two booleans, one refusal and a placeholder branch.
- `pair_verdict` still reuses `difference`, `monochrome_predicate`, `bias_bound` and the existing constants, and defines nothing of its own. The `None` arms stay unreachable for `ChannelSet::Monochrome`.
- Removing the stray-argument refusal from `run` is caught by clippy's `dead_code` under `-D warnings`, so its wiring is watched by the lint gate if not by a test.
- On the real binary, stdout carries JSON only for a verdict, and errors go to stderr with exit 1.
- Every closed-schema refusal that a test drives went red under its mutation: J6 to J11, J24 and J25. So did J18 to J23 for D4, S4 and N5, J21 and J22 for the ratio, and J26 for N1.
- `parseFootprint` refuses an input with no process block, and refuses a block with no peak line.
- The JS key list for the pair verdict is still a hand-written copy. A field added in Rust and missing in JS fails closed at `publishedRecord`, as an unknown key or a missing key, so a drift would be loud, not silent.
- The out-of-scope files are untouched, and so are `tools/bench/comparison.json`, `README.md`, `use.html` and `more.html`.

## Limitations and handoff

- No browser, Playwright, oracle or GPU job was run. `npm run lint` cannot run in this worktree because it has no `node_modules`. The proxy lint is listed above.
- For the plan owner, not the implementer. My capture contradicts fact 9's "Summary Footprint ... for two unrelated processes was their sum": two sleeps gave 2425312 against a per-pid sum of 2458080, because they share pages. Plan row 143's published subset does not list the new `browser` block that the lead ruled in. Plan Approach item 4 lists six JSON keys and leaves out `low_information`.
- Reason strings and the renderer string are free text by necessity, and only the deny scan covers them.
- The run-round items from pass 1 are still open: AC7's per-workload flip, AC11 and AC15 to AC20. N6 belongs with them.
- I deleted the scratch copies `target/f223-scratch`, `target/f223-scratch-target` and `target/f223-clone`. HEAD is still `120228e`, and `git diff --quiet HEAD` exits 0. This file is the only addition.
- Next owner: the implementer fixes D1 to D3 and S1 and S2, then a fresh independent pass 3 reviews the changed material.
