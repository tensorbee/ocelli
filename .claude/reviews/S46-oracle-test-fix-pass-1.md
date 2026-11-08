# S46 oracle test fix review, pass 1

**Reviewed**: commit b61cfe282bd1dd8bcea71d02456c9d60bdd94741 on sprint/s46, `tools/oracle/src/produce.rs` only, canonical worktree clean at review time.
**Author / reviewer**: authored by the sprint lead (a Claude session). Reviewed by an independent Claude subagent that wrote none of it. No sub-agents were used.
**Result**: 1 defect, 0 smells, 2 nitpicks

## Acceptance challenged

- With the reference present, both tests still compare frames. Rejected wrong behaviour: the new `None` branch firing on the operator's machine and turning both tests into constant passes.
- In a clean `git archive` copy, both pass by asserting `reference-run-absent`. Rejected wrong behaviour: the absent branch passing whatever the producer says.
- CI's `cargo test --workspace` in that clean copy. Rejected wrong behaviour: another untracked-resource test still panicking.
- Partial resource state, reference present and corpus absent, was derived independently as the case that separates "the producer says the resource is absent" from "the test asserts a constant".

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `cargo test -p ocelli-oracle --lib` on the two tests, `--exact` | archive of b61cfe28, `tools/oracle/out` and `corpus/data` symlinked to the canonical tree | now | 2 passed |
| Mutant A: `apply_reference_jpeg_level` uses `residual = 0.0_f32` | same | now | jpeg test FAILED at produce.rs:3294 "fraction 0.956787109375 over 11264 worst 10", dx ok |
| Mutant B: `WINDOW_SUBPIXELS` 16.0 to 1.0 (produce.rs:965) | same | now | dx test FAILED at produce.rs:3294 "fraction 0.87200927734375 over 23522 worst 192", jpeg ok |
| Rebuild unmutated and rerun | same | now | 2 passed |
| The two tests in a clean copy, own `CARGO_TARGET_DIR` | archive of b61cfe28, no `out`, no `corpus/data` | now | 2 passed, and the run wrote `tools/oracle/compare-out/dispositions.json` with `resources` `reference-run-absent` |
| Mutant C: produce.rs:310 `bare("reference-run-absent")` to `bare("reference-run-mutant-c")` | clean copy | now | both FAILED at produce.rs:3226, left "reference-run-mutant-c" right "reference-run-absent" |
| Parent produce.rs (`b61cfe28^`) in the clean copy | clean copy | now | both FAILED at produce.rs:3213 "entered unreachable code: the reference run loads, ... tools/oracle/out/run.json: No such file or directory" |
| `cargo test --workspace --no-fail-fast` | clean copy | now | exit 0, 155 test binaries, 1762 passed, 0 failed, 82 ignored |
| `cargo fmt --all --check` | clean copy | now | exit 0 |
| `cargo clippy -p ocelli-oracle --all-targets -- -D warnings` | clean copy | now | clean |
| Reference present, corpus symlink removed | present copy | now | both pass, `None` returned, dispositions record `reference-run-absent` although `tools/oracle/out/run.json` exists |

Environment: macOS, stable toolchain from the workspace, no GPU job, no oracle run, no browser.

## Defects

### D1, the doc comment says the producer reports the absence, and for an absent corpus it reports the wrong one

**Where**: tools/oracle/src/produce.rs:3211-3213, the doc comment on `rendered_pair`, and the branch at produce.rs:3218-3229.
**What**: the comment says "`None` when the reference run or the corpus is absent. Absent, the producer must say so". The branch never shows the producer the real state. It always passes the fixed path `tools/oracle/missing-run` as `reference_run`, so the producer reports `reference-run-absent` whatever is actually missing. When the reference is present and the corpus is absent, the producer is never asked about the corpus. With the real paths it would say `corpus-absent` (produce.rs:343-346). The test passes anyway and writes a dispositions record naming the wrong resource.
**Why it is wrong**: the microscope's section 3 counts a false claim in prose as a defect. This claim is a guard description, and it is false in one of the two states it names.
**Evidence**: present copy with only the `corpus/data` symlink removed. Both tests passed, and `tools/oracle/compare-out/dispositions.json` held `"resources": "reference-run-absent"` while `tools/oracle/out/run.json` existed.
**Scope and regression**: the same shape is already in `uniform_slice_edge_matches_the_reference_within_one_lsb` (produce.rs:3139-3152) and `applicable_monochrome_rows_reach_the_comparator` (produce.rs:2315-2333). This commit copies it and adds a comment that makes the claim explicitly. The assertion is a constant. It is fixed input to a fixed branch, so it duplicates `a_missing_reference_run_is_unrun` (produce.rs:2288) and adds nothing about the environment. Mutant C turns it red, but it turns that earlier test red too.
**Fix**: the smallest one is to make the comment true. Something like "`None` when the reference run or the corpus is absent, after checking that the producer refuses a reference run that is never present." That avoids "the producer must say so". The stronger option is to hand the producer the real `reference` and `corpus` paths and an output under `std::env::temp_dir()`. Then assert `reference-run-absent` when `run.json` is missing, and `corpus-absent` otherwise. That would also clear N1.

## Smells

None.

## Nitpicks

### N1, the absent branch writes into the checkout's `tools/oracle/compare-out`

**Where**: produce.rs:3222-3226, which goes through `write_dispositions` (produce.rs:2149) via produce.rs:309-311.
**What**: in a clean tree, `cargo test` creates `tools/oracle/compare-out/dispositions.json`, and it was observed. The path is gitignored (`.gitignore:62`), so git is unaffected. But a tree with a real `compare-out/dispositions.json` and no reference has it overwritten with a `reference-run-absent` record. Worktree `ocelli-wt-f-223` is in that state today: no `out`, no corpus, a `compare-out/dispositions.json` present. Two neighbouring tests already carry this, and this commit adds two more writers. It does not block.
**Fix**: point `reference_subset`, `candidate_run` and `dispositions` at a per-process `temp_dir()` path, as `a_missing_reference_run_is_unrun` does.

### N2, outside the diff: the jpeg test's comment names the mean bias, and the mutant fails on the fraction instead

**Where**: produce.rs:3182-3184.
**What**: "A residual of 0 leaves the informative mean about nine display counts dark". Under mutant A, the test failed on the first assertion, within-one fraction 0.957 and 11264 pixels over the maximum. The mean assertion was never reached, so the "nine counts" figure went unobserved. The test does go red, which is the claim that matters. Not counted against this commit.

## Verified clean

1. Present behaviour. Both tests take the comparison path and each guards its own constant. Mutant A reddens only the jpeg test and mutant B only the dx test. The unmutated rebuild is green.
2. Absent behaviour. Both pass in a clean archive and assert `reference-run-absent`. The leading `jpeg_extended_12_level_residual` bit-exact assertion still runs before the absent return, so the jpeg test guards the constant in CI too. Mutant C reddens both.
3. The commit's premise. The parent's two tests panic with `unreachable` in a clean archive, so the fix addresses a reproduced failure. CI runs `cargo test --workspace` at `.github/workflows/*.yml:61`.
4. The whole workspace in a clean archive passes: 1762 passed, 0 failed, 82 ignored. No other test breaks CI for lack of `tools/oracle/out` or `corpus/data`. Other readers of those paths, and how each guards absence:
   - `crates/ocelli-dicom/tests/corpus.rs:31,81,140`: `#[ignore]`.
   - `tools/oracle/src/produce_current.rs:2524` `equipped()`: returns `None`.
   - `tools/oracle/tests/geometry_fixture.rs:116`: returns early on a missing `run.json`.
   - `produce.rs:2315` and `produce.rs:3139`: the same branch as this commit.
5. Not silent on the operator's machine. With `run.json` and the corpus both present, the branch is not taken, and a missing sidecar or `.raw` still panics through `unreachable!`. The branch is reached only when the reference is genuinely absent. A failed `bin/ocelli.sh oracle` run empties `tools/oracle/out` (run.mjs:1675-1683), and the `oracle` gate (bin/ocelli.sh:417) then fails loudly in the same `gate --sprint`. So the skip does not hide a frame failure where the reference exists. It still reports `ok` rather than `ignored`, which is the neighbouring tests' convention.
6. Commit message, every claim executed. "panicked in a clean clone because tools/oracle/out is untracked" is true (parent replay, and `.gitignore:51`). "CI's cargo test --workspace failed" was inferred from the reproduced panic plus the CI command. No CI log was read. "takes the absent branch its neighbouring tests use" is true, a line diff against produce.rs:3139-3151 differs only in `return` against `return None`. "asserting the producer reports reference-run-absent, and both tests return" is true. "With the reference present both still compare the rendered frame" is true (mutants A and B).
7. Doc comment, first sentence ("The candidate and reference frames of one row, or `None` when the reference run or the corpus is absent"): true for both absent states. The second sentence is D1.
8. fmt and clippy at `-D warnings` are clean for the crate. No `as` cast, no new arithmetic and no prose em-dash or semicolon in the diff. `README.md`, `use.html` and `more.html` describe nothing this test helper touches.

## Limitations and handoff

- The clean copy is a `git archive` with no `.git`, which CI does have. A test that needs git metadata could behave differently in CI. None failed here.
- My first clean-copy run shared a `CARGO_TARGET_DIR` with the present copy. Cargo reused the present copy's binary, which had `CARGO_MANIFEST_DIR` baked in, and it reported a misleading dx failure. I discarded it and repeated every clean-copy result with a separate target directory. This was my method error, not a finding.
- No GPU, oracle or browser job was run. Tier A candidate tests stayed ignored.
- Clippy was run for `ocelli-oracle` only, not the whole workspace.
- Scratch copies `oracle-fix-p1-*` were removed with shutil. The canonical tree still reports a clean `git status`.
- Next owner: the sprint lead fixes D1 (and N1 optionally), then a fresh independent pass reviews the remediation.
