# F-223 review, pass 1

**Reviewed**: `git diff 73b68ad bdec2c9` on `work/f-223-xai`, the pure-function slice only: Approach step 2 (AC10, `pair_verdict` and `ocelli-compare pair`) and the pure functions of step 3 (AC4 to AC9, AC12 to AC14, `tools/bench/src/comparison.mjs`, `tools/bench/tests/comparison_test.mjs`, `tools/bench/comparison.json`). Worktree clean at `bdec2c9` before and after the pass. Out of scope: `comparison_run.mjs`, `run.mjs` wiring, `package.json`, `bin/ocelli.sh`, `bench_check.py`, docs.
**Author / reviewer**: GLM-5.3 in opencode wrote `bdec2c9`. Claude Opus 5.5 reviewed it as an independent subagent and wrote none of the reviewed material.
**Result**: 7 defects, 4 smells, 5 nitpicks

## Acceptance challenged

- **AC10.** Worked the fixtures out again from 25.1's text. 9999 of 10000 within one code is 0.9999, which is at least 0.999. One pixel at +3 gives a count over 2 of 1, so the frame fails at any count. +1 everywhere gives a bias of 1.0, outside 0.1. The clipped background gives 201 / 2000 = 0.1005 informative against 201 / 10000 = 0.0201 over the rectangle. 5 / 100 = 0.05 is below the 0.10 floor. The f64 literals match by `to_bits` because each one is a single correctly rounded division. `pair_verdict` calls the existing `difference`, `monochrome_predicate`, `bias_bound` and `informative_fraction` and defines no constant, so it is not a second copy of 25.1. Mutations: bias over `image` went red, dropping the floor went red, dropping the bias clause went red, dropping the monochrome clause went red. `<` changed to `<=` at the floor stayed green (S1). A discriminating case I built myself, an all-black 10 by 10 pair, comes back as an error and not as a low-information verdict (D3).
- **AC5.** Re-derived `[10,20,30,40]`. The index is (4-1)q, so p25 = 10 + 0.75 * 10 = 17.5, median = 25 and p75 = 30 + 0.25 * 10 = 32.5. `[1..9]` gives 3, 5 and 7. Nearest-rank went red, an unsorted list went red, and accepting fewer than N trials went red.
- **AC6.** `p75 < p25` is strict, so ranges that touch are not separated. Median-only went red. `<=` went red.
- **AC7.** The alternation is correct and its mutation went red. **The per-workload flip is not in this slice.** `trialOrder` takes the start side from its caller, and the caller is `comparison_run.mjs`, so this half of AC7 is still open.
- **AC8.** The tree is root plus descendants to depth 3, with the sibling and the ancestor excluded. Direct-children-only went red. `memoryFigures` re-derived: the sample sums are 300, 310 and 280, so the lower figure is 310. The per-pid peaks are 160 and 240, so the upper figure is 400. Both mutations went red. **`parseFootprint` was tested against the real tool and fails** (D1).
- **AC9.** 0.5 * 3 = 1.5 re-derived. Exited, appeared and backwards each come back invalid, and ignoring a set change went red.
- **AC12.** The function returns kinds and line numbers only. I read every error message in `comparison.mjs` and none prints a deny value. Skipping UIDs went red. Values that JSON escapes are missed (S3).
- **AC13.** The schema is closed only in parts (D2).
- **AC14.** Checked the ten adjectives against the plan's list and they match. prose_check runs on the rendered text and passes. Dropping `not_shown` when the Ocelli figures are lower went red, and so did dropping the acknowledgement. An inverted ratio went red. The architecture text is checked.
- **AC4.** The largest CT series lacks a window and is not picked. First-found went red, ignoring the window went red, and the digest tie-break and DX pixel rule checked out.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `cargo fmt --all --check` | worktree at bdec2c9 | now | 0 |
| `cargo clippy -p ocelli-oracle --all-targets -- -D warnings` | same, `CARGO_TARGET_DIR=<wt>/target` | now | 0 |
| `cargo test -p ocelli-oracle --test pair_fixture --test tolerance_fixture` | same | now | 0, 7 + 10 pass |
| `cargo test -p ocelli-oracle --bin ocelli-compare` | same | now | 0, 16 pass |
| `node --test tools/bench/tests/comparison_test.mjs` | same | now | 0, 23 pass |
| `python3 scripts/prose_check.py` | same | now | 0, 864 files |
| `python3 scripts/source_provenance_check.py` | same | now | 0, 1710 files |
| `python3 scripts/guard_census.py` | same | now | **1, FAIL, 67 uncatalogued refusal sites, all in `tools/bench/src/comparison.mjs`** (D5) |
| `/usr/bin/footprint -f bytes -p $$ -p $PPID` then `parseFootprint` | this host, macOS 27 | now | the real output has no `Process N` line. `parseFootprint` returns a Map of size 0 and `memoryFigures` throws (D1) |
| Probe `pair_verdict` on an all-black pair and at fraction exactly 0.10 | scratch copy under `target/`, deleted | now | `Err("a statistic over zero pixels ...")`, then `0.1 false true` |
| Rust mutations M1 to M5, B1, B2 | scratch copy, deleted | now | M1, M2, M4 and M5 red. M3 (floor `<=`), B1 (`Ok(true)`) and B2 (renamed JSON key) green |
| JS mutation catalogue, 19 mutations | scratch copy, deleted | now | 18 red. The one green mutation reads `Summary Footprint`, which only an invented fixture can tell apart, and it is moot under D1 |
| `publishedRecord` and `renderReport` probes | inline node | now | machine stray key kept, free-text versions kept, string `passes` kept, a single-sided cell published and then a render TypeError, refused-run report says the pixel check passed |
| Implementer claim: two produce.rs failures | `produce.rs` unchanged between 73b68ad and bdec2c9. `tools/oracle/out/` is gitignored and absent | now | `dx_window_snap_matches_the_reference_bound` panics at produce.rs:3212 on the missing `run.json`. **Claim confirmed**, the failure predates this commit |
| Implementer claim: registration test red | `tools/bench/tests/paths_test.mjs` "every benchmark node suite is registered in both exact runner lists" | now | red, only for `comparison_test.mjs` missing from `package.json` and the bench arm. **Claim confirmed**, it is step 5's job |

## Defects

### D1, `parseFootprint` reads a shape the tool does not print
**Where**: `tools/bench/src/comparison.mjs:519-573`, fixture `tests/comparison_test.mjs:147-171`
**What**: `FOOTPRINT_PROCESS = /Process (\d+)/` keys each block on a `Process <pid>:` line. `/usr/bin/footprint -f bytes` prints `<name> [<pid>]: 64-bit    Footprint: <n> B` between `====` rules. `phys_footprint` and `phys_footprint_peak` sit under `Auxiliary data:`, and there are `Shared with <name> [<pid>]:` sub-blocks a naive `[pid]` regex would also catch. On real output the function returns an empty Map and raises no error.
**Why it is wrong**: the plan's fact 9 and AC8, "parseFootprint over the recorded shape in fact 9". The comments in the code and in the test both say "the shape the plan's fact 9 records". Fact 9 does not record a `Process N` header, so the fixture was invented, and both comments are false.
**Evidence**: real `footprint` output for two pids, fed to `parseFootprint`, gives `size 0`, and `memoryFigures([m])` then throws "each memory sample is a non-empty pid to figures map".
**Scope and regression**: on the equipped host, every `peak_memory` session fails. The fix is a fixture copied from real `footprint` output for two pids, including a `Shared with` block, plus a refusal when no process block is found.

### D2, the published schema is not closed
**Where**: `comparison.mjs:959-970`, `1046-1052`, `1059-1069`
**What**: `machine` checks that its required keys are present but copies every other key as well. `versions` accepts any key with any string value, which makes it a free-text map. Values in the pair verdict and the metric cells have no type check, so a string `passes` or `median` gets published. A metric cell with one side missing passes `publishedRecord`, and `renderSummary` then throws a TypeError.
**Why it is wrong**: AC13 says it "throws on an unknown key", and the wrong implementation it rejects is "A free-text field that could carry a header value". Plan section 13 says "a value cannot arrive through a field nobody expected".
**Evidence**: `machine.hostname = "STUDY-OF-DOE^JOHN"` was kept. `versions.series_description` was kept. `pair_verdict.passes = "DOE^JOHN"` was kept. The single-sided cell gave "Cannot read properties of undefined (reading 'state')" at render. The implementer's mutation "publishedRecord copies unknown keys" only exercised metric cells.
**Scope and regression**: this affects the tracked JSON and Markdown. The deny scan is the only remaining guard. The fix is a closed key set for `machine` and `versions`, typed values, both sides required per cell, and one negative test for each.

### D3, a pair with no informative pixel is an error and not a low-information verdict
**Where**: `tools/oracle/src/tolerance.rs:423-452`
**What**: when every pixel is clipped to the same extreme on both sides (all black, or all white), `bias_bound(informative)` returns `StatsError::NoPixels`, and `pair_verdict` returns `Err`. The informative fraction is 0, which is the clearest low-information case there is.
**Why it is wrong**: the `# Errors` contract, in the plan and at the site, is "When the frames differ in size or the image region is empty". The image region here is 10 by 10 and not empty. Plan step 5 says a low-information verdict "writes the verdict numbers", and this case writes none. The comment at line 430 says the only way to reach `NoPixels` is an unreachable `None` arm, but this is a reachable path to the same error.
**Evidence**: the probe `pair_verdict(all-black 10x10, same)` returned `Err("a statistic over zero pixels has no value ...")`.
**Scope and regression**: this case comes up when both sides render one window badly enough to clip everything. The harness will then see an argument-style error on exit 1, not a `refused_pixels` verdict. The fix is a fixture with 0 informative of 100 that expects `low_information` true and `passes` false, or a documented refusal.

### D4, a refused run's plain report says the pixel check passed
**Where**: `comparison.mjs:1202-1221`, called from `renderReport` by way of `sharedBody`
**What**: `pixelSection` always opens with "Every timing figure above was taken in a run whose pixel check passed", and `renderReport` renders it for a `refused_pixels` run too.
**Why it is wrong**: the sentence is false in exactly the state the plan says the report must describe (plan AC11 and state table: "refused_pixels, verdict written, no timing figure").
**Evidence**: `renderReport({ state: "refused_pixels", metrics: {}, pair_verdict: { passes: false, ... } })` prints that sentence, followed by "- the verdict passed: no". The test at `comparison_test.mjs:617-625` renders a refused run with `passes: true` in its fixture, so it never sees this.
**Scope and regression**: the plain report is affected. The tracked summary is not, because `--publish` refuses a failed check. The fix is a refused-run fixture with `passes: false` that asserts the sentence is absent.

### D5, the guard census goes red on this commit
**Where**: `tools/bench/src/comparison.mjs`, 67 throw sites, against `scripts/guards/catalogue.py`
**What**: `python3 scripts/guard_census.py` exits 1 with 67 "is a refusal no catalogue entry claims" lines, and all 67 are in `comparison.mjs`. The census runs in the `guards` arm of `bin/ocelli.sh` (line 237).
**Why it is wrong**: CLAUDE.md says the census bucket is a ratchet that may only decrease. The plan's write set and Approach step 5 never name `scripts/guards/catalogue.py`, so no later step owns this fix and it will not clear itself.
**Evidence**: the command above. Every failure line is in `comparison.mjs`.
**Scope and regression**: this fails the guards gate, the floor and the sprint verification. The fix is catalogue entries naming `comparison_test.mjs` as the covering suite, plus adding the catalogue file to the plan's write set.

### D6, `PairVerdict`'s doc claims the binary serialises the struct
**Where**: `tolerance.rs:390-392`, against `ocelli-compare.rs:197-205`
**What**: the doc says "the binary serialises exactly this struct" so that "the harness and the tool cannot drift". In fact the binary builds a `json!` literal that names each key by hand, and there is no `Serialize` derive. A field added to the struct would not reach the output, and a key renamed in the literal compiles and passes every test (mutation B2).
**Why it is wrong**: microscope section 3, a claim in prose that is false.
**Evidence**: mutation B2, `"signed_mean_bias_informative"` renamed to `"signed_mean_bias"`, left 16 of 16 green.
**Scope and regression**: the JS `PAIR_VERDICT_KEYS` is a third hand-written copy. The fix is either a derive or a corrected sentence, plus the test in S2.

### D7, `parseProcessTree`'s doc claim about an unlisted root is false
**Where**: `comparison.mjs:483-485`
**What**: the doc says "A pid the text does not list has no descendants to add and returns as a set of one". With `ps` text `200 100` and root 100 not listed, the function returns `{100, 200}`. That behaviour is the right one and the sentence is wrong.
**Evidence**: `parseProcessTree("  200   100\n", 100)` gives `[100, 200]`.
**Scope and regression**: this is a doc comment only. Delete the sentence or narrow it to "an empty text".

## Smells

### S1, the information floor's boundary is not pinned
`tolerance.rs:442` uses `<`, so a pair at exactly `INFORMATIVE_FRACTION_FLOOR` passes, which matches the plan's `>=`. Changing it to `<=` left every test green (M3). My probe at 10 informative of 100 printed `0.1 false true`. A fixture at exactly 10 / 100 would pin it. Microscope section 1, boundary comparisons.

### S2, the `pair` command's output contract is untested
The harness reads two things from `pair`: the exit status on a failing verdict and the seven JSON key names. Neither has a test. B1 (`Ok(verdict.passes)` changed to `Ok(true)`) and B2 (a renamed key) both stayed green, because the bin tests only run a passing pair and a refused pair. The smallest regression is a bin test on a failing pair that asserts `Ok(false)`, plus a key-set assertion shared with `PAIR_VERDICT_KEYS`.

### S3, `scanForDenied` over JSON text misses values JSON escapes
The scan is a raw substring match over the output text. A deny value containing `\` (a multi-valued DICOM string, or a path component) or `"` turns into `\\` or `\"` in `JSON.stringify` output and does not match. The probe confirmed it: the value `GENERAL\HOSPITAL` against `{"x":"GENERAL\\HOSPITAL"}` gives `[]`. The plan scans "the ignored JSON ... and both tracked files" as text. The fix is to scan each value's JSON-escaped form as well as its raw form, or to scan the parsed values, with one fixture value that contains a backslash.

### S4, no field in the published record can name the renderer, headed mode or SwiftShader
Plan fact 15 says that under `--software` "every output names SwiftShader in its first paragraph", and that the mode "records `headed` in the output". The tier sentence promises "cornerstone3D on the named WebGL renderer". `publishedRecord` has no field for any of these, and the first paragraph of `renderSummary` and `renderReport` is the fixed `ARCHITECTURE` text. The renderer string can only appear by accident, as a key in the free-text `versions` block (D2). When the run round lands, it will have to change these pure functions or publish a SwiftShader run without naming it.

## Nitpicks

- N1. `loadComparison` says "names a workload label twice" when `workloads` is not an array at all (`comparison.mjs:325-328`).
- N2. `(a.median / b.median).toPrecision(3)` renders `1.23e+3` once a ratio reaches 1000, and `Infinity` when the cornerstone3D median is 0.
- N3. `pair` silently ignores `--out` and `--corpus`, while `refuse_stray_frame_size` refuses `--width` on every other command for the very reason the code states. `--reference wants a directory` is the wrong noun for `pair`.
- N4. The `pair` doc says it exits 1 when the verdict does not pass. An argument or size error also exits 1, so the harness has to tell the two apart by whether stdout carries JSON. Say so at the site.
- N5. The Markdown table cells do not escape `|`, so a version string containing one breaks the table.

## Verified clean

- No `as` cast was added anywhere in the Rust diff, checked by grep over every added line. `--width` and `--height` parse as `u32`, and `Frame::new` checks `width * height * 4` in `u64`.
- `pair_verdict` reuses the existing predicate and defines no constant. `ToleranceError` gains exactly one variant, `Frame(#[from] FrameError)`.
- The size mismatch is refused through `FrameError::SizeDisagreement`, with the message "disagree about the frame size". A truncated `.raw` is refused with "is 15 bytes". A 0 by 0 pair is refused.
- The bias is taken over the informative region only, confirmed by M1 going red on the clipped-background fixture.
- `summarise`, `separated`, `trialOrder`, `parseProcessTree`, `memoryFigures`, `cpuFigure`, `selectWorkloads`, `inputIdentity`, `otherBrowserJobs` and `loadComparison` were each mutated, and each went red on its named test.
- The acknowledgement, the architecture text, the tier sentence and every `not_shown` item render whichever side is lower, with both directions mutated.
- The rendered summary and report contain no em-dash, en-dash, semicolon or listed adjective, and pass `prose_check.scan`.
- `comparison.json` matches the plan's metric table for N, workloads, mirrors, units and cold or warm states, and `loadComparison` refuses each drift its test plants.
- GLM's seven open decisions. Decisions 1, 2, 3, 5 and 6 are sound. Decision 4 is sound except for N3. Decision 7 is acceptable as written for the `None` arm, but its comment implies `NoPixels` cannot be reached, and D3 is a reachable path to it.

## Limitations and handoff

- The orchestration is out of scope: `comparison_run.mjs`, AC1 to AC3's run half, AC7's per-workload flip, AC11, AC15 to AC20 and the deny set's construction. Short path components such as `CT` or `1` in the deny set would refuse every publish, which is fail-safe. The run round should decide this.
- `tools/bench/subjects.json` on this base has no `viewer.*` subject, because F-216 is not integrated, so the `$about` sentence in `comparison.json`, "Every metric names the viewer.* subject in subjects.json it mirrors", is true only after F-216 lands. AC16's `bench_check.py` check should catch it.
- The comparison suite is not yet registered (`paths_test.mjs` is red until step 5), so the floor does not run it today.
- No browser, Playwright, oracle or GPU job was run. `footprint` was run once against two of the reviewer's own shell processes.
- Scratch copies under `target/f223-scratch` and `target/f223-base` were deleted. `git status` was clean at `bdec2c9` before this file was written.
- Next owner: the implementer fixes D1 to D7 and S1 to S4, then a fresh independent pass 2 reviews the changed material.
