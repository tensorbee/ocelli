# F-223 review, pass 6

**Reviewed**: `git diff 12539b7 HEAD` on `work/f-223-xai`, the pure-function slice only (AC4 to AC10, AC12 to AC14). HEAD `653d03c4953895ea8609ba02a1d9ed44dc6ab03e`. Two commits, `5444bf6` and `653d03c`, touching `tools/bench/src/comparison.mjs` (the `validatePairVerdict` doc and two new refusals), `tools/bench/tests/comparison_test.mjs` (four new tests), `tools/oracle/src/bin/ocelli-compare.rs` (two new tests inside `#[cfg(test)] mod argument_tests` only) and `ci/guard-probe-budget.json` (`bench.comparison` 93 to 95). The worktree was clean apart from the untracked pass 1 to pass 5 reports, before and after this pass.
**Author / reviewer**: The lead reports that a Claude implementer wrote both commits. The commits carry the operator's name as git author and no trailer, so that attribution rests on the lead's statement. Claude Opus 5.5 reviewed them as an independent subagent. I wrote none of the reviewed material and none of passes 1 to 5.
**Result**: 0 defects, 0 smells, 1 nitpick

## Acceptance challenged

- **D1 of pass 5 (the bias doc overclaimed).** The lead ruled both fractions bounded to [0, 1]. I drove a matrix I chose through `publishedRecord` and `renderSummary` on a scratch copy of HEAD, each value for each fraction, each beside a measured bias (0.031) and beside a null bias (`low_information` true):
  - `-0.5` and `1.5`: refused with "outside [0, 1]" beside a measured bias. Beside null the informative case is refused earlier by the null rule, which is also correct.
  - `1 + Number.EPSILON`: refused. `-Infinity`, `Infinity` and `NaN`: refused as "not a finite number" because the finite check runs first.
  - `0` and `1`: publish. The informative fraction at 0 publishes only beside null, and a measured bias there is refused by the existing rule.
  - `-0`: publishes as `0` (JSON `"0"`, rendered `0.0000`), and is treated as 0 by the bias rule both ways, as pass 5 agreed it should be.
  - `Number.MIN_VALUE`: publishes beside a measured bias, as the doc says (above 0). See N1.
  - With the fractions bounded, the doc's equivalence ("a finite number exactly when the informative fraction is above 0, and null exactly when the fraction is 0") holds over every admitted record. **Closed.**
- **S1 of pass 5 (`--width` and `--height` with no value).** The real bin, `target/debug/ocelli-compare`, prints "--width wants a pixel count" for `pair --width` and "--height wants a pixel count" for `pair --height`, both exit 1. The same holds with the other size given first (`pair --width 4 --height`, `pair --height 4 --width`). The two new tests drive exactly these. **Closed.**
- **Pass 5 N3 (an `Infinity` fraction never sent).** It is now moot for the published outcome. With the finite check weakened to `isNaN`, an `Infinity` fraction is still refused by the new range check (`Infinity > 1`). Only the message would differ.
- **The new claim "a share of the frame's pixels".** I traced it to the producer. `pair_verdict` (`tools/oracle/src/tolerance.rs:342`) computes the difference over `Rect::full`, so the image rectangle is the whole frame. `informative_fraction` is informative pixels over `image_pixels` (`tools/oracle/src/frame.rs:423`), and `within_one_lsb_fraction` is `fraction_within` over the `full` region. Both are counts over counts with the numerator a subset, so both lie in [0, 1], and for this verdict both are shares of the frame's pixels. True as written.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `cargo fmt --all --check` | worktree at 653d03c | now | 0 |
| `cargo clippy -p ocelli-oracle --all-targets -- -D warnings` | same, `CARGO_TARGET_DIR=<wt>/target`, `ocelli-compare.rs` `touch`ed to force a re-check | now | 0 |
| `cargo test -p ocelli-oracle --bin ocelli-compare` | same | now | 0, 23 pass, including the two new tests |
| `cargo test -p ocelli-oracle --test pair_fixture --test tolerance_fixture` | same | now | 0, 10 + 10 pass |
| `node --test tools/bench/tests/comparison_test.mjs` | same | now | 0, 58 pass (54 at pass 5, plus 4 new) |
| `npm run lint` | worktree | now | 0 |
| `guard_census.py`, `--check-runbook` | same | now | 0, 0, "0 watched by nothing" |
| `prose_check.py`, `source_provenance_check.py` | same | now | 0 (864 files), 0 (1710 files) |
| Probe matrix, 10 values times 2 fractions times 2 bias states, through `publishedRecord` and `renderSummary` | `git archive HEAD tools/bench scripts ci package.json` under `target/mp6`, deleted | now | as listed above |
| 10 JS mutations: each new refusal replaced by `false`, each half of each disjunction dropped, each `< 0` to `<= 0`, each `> 1` to `>= 1` | same copy, file `cmp`'d against the worktree after the run | now | every mutation red. Each deletion turns exactly its own `refuses ... outside [0, 1]` test red and nothing else |
| Real bin, `pair --width`, `pair --height`, both with the other size first, `pair --width x`, `identity --width` | `cargo build -p ocelli-oracle --bin ocelli-compare` in the worktree | now | exit 1 each, the expected message each |
| 2 Rust mutations: each missing-value `ok_or_else(..)?` replaced by `unwrap_or_else(\|\| "0")` | `git archive HEAD` under `target/mp6rs` with its own `CARGO_TARGET_DIR=target/mp6rs/target` (an APFS clone of the worktree's `target/debug`), deleted | now | each exits 101 with exactly its own test red, 22 passing |
| Census sites in `comparison.mjs` via `_sites_in` at `12539b7` and HEAD, and `match_sites(discover())` | `git show` of each revision | now | 93 then 95. The two new keys are the `js-throw` at `1330` (within-one-LSB) and `1336` (informative). `bench.comparison` claims 95 |
| Added lines scanned for an em-dash, an en-dash and a prose `;` | `git diff 12539b7 HEAD` | now | none |
| Out of scope | `git diff --quiet 12539b7 HEAD` over `README.md`, `examples`, `docs`, `scripts`, `run.mjs`, `tolerance.rs`, `lib.rs`, `.claude/plans`, `AGENTS.md`, `CHANGELOG.md` | now | exit 0 |

Environment: macOS (Darwin 27.0.0), node v24.13.0, rustc 1.97.1.

## Defects

None.

## Smells

None.

## Nitpicks

- N1. The summary renders the informative fraction with `toFixed(4)`. A real one-pixel informative region on a 512 by 512 frame is 3.8e-6, which the Rust producer can emit, and it renders as "informative fraction: 0.0000" beside a measured bias. The record is correct and `low_information` reads "yes" beside it, so nothing is false. It predates this diff, but the new doc draws the line at exactly 0, and a reader of the summary sees 0.0000 on both sides of that line.
- Carried from pass 5: N1 (census message collapse at `773`/`781` and `958`/`1014`), N2 (the renamed memory test), N4 (the frame-file noun), N5 (`py-return-list` matching three renderer headings, which is still 3 of the 95) and N6 (an `invalid` cell's reason is never rendered). None of them is touched by this diff. Pass 5 N3 is moot, see above.

## Verified clean

- Every sentence changed in the diff, executed:
  - The `validatePairVerdict` doc: "each lies in [0, 1] because each is a share of the frame's pixels" (traced to `Rect::full` and the count ratios), and the bias equivalence (holds over the probe matrix).
  - The inline comment above the new refusals. "a bias beside a negative informative fraction would publish as measured": with the refusal deleted, `-0.5` beside 0.031 published and its test went red. "Each refusal has its own words so the census sees each one separately": the census keys the two sites separately.
  - Both new error messages match the regexes their tests assert, and each test fails when its own site is removed.
  - The test comment "A measured bias rides beside each refused value, so only the range check can refuse it": the fixture's default bias is 0.031, the finite check passes `-0.5` and `1.5`, and deleting the range check publishes both.
  - The test messages "the floor of the range" and "the top of the range" are pinned by the `<= 0` and `>= 1` mutations.
  - The Rust test failure messages and asserted substrings match the bin's actual output.
- No `as` cast and no pixel arithmetic changed. The Rust change is in the test module only.
- The census moved by exactly two real refusals. `uncovered` is 0.
- The out-of-scope files are untouched since `12539b7`.

## Limitations and handoff

- No browser, Playwright, oracle or GPU job was run. AC7's flip, AC11 and AC15 to AC20 remain with the run round.
- This pass swept only the diff and what it touches. Earlier passes swept the rest of the slice, and their findings are carried, not re-verified.
- I deleted `target/mp6` and `target/mp6rs`. HEAD is still `653d03c`, `git diff --quiet HEAD` exits 0, and this file is the only addition. The probe and census scripts stay in the session scratchpad, outside the tree.
- This is a clean bounded review of the pure-function slice. It is not completed acceptance for F-223 and not sprint readiness.
