# F-239 review, pass 4, codec part

**Reviewed**: `work/f-239-codec`, range `2eaec747..7b51bdb6` (one commit, 9 files), in the worktree `ocelli-wt-f-239-codec`, tip `7b51bdb6ea2464ea2bf3fc5cb844db48305b9f92`. No staged or unstaged tracked change at review time. Untracked: the `.venv` symlink and `tools/oracle/out` only.
**Author / reviewer**: remediation by GLM in opencode under the lead ruling after pass 3 (`.claude/plans/F-239-design.md:564` to `:572`), with GLM's self-review cycles 11 to 13 (`.claude/scratch/F-239-self-review-11.md` to `-13.md`, ignored files). Reviewed by Claude as an independent `/microscope --working` pass 4. The reviewer wrote no part of this range. OpenJPEG source and ISO/IEC 15444-1 were not read at source, so the derived coefficients below come from the forward ICT the plan transcribes.
**Result**: 0 defects, 1 smell, 1 nitpick

## Acceptance challenged

- **The gap between G-6's literals and the derived coefficients, re-measured.** Exact rational arithmetic on the forward matrix gives blue-to-green `2 * 0.114 * 0.886 / 0.587 = 0.3441362862` and red-to-green `2 * 0.299 * 0.701 / 0.587 = 0.7141362862`. Gaps from the literals: `6.286e-6` and `3.714e-6`. Through the published f32 expressions (`ALPHA_*` as f32, arithmetic in f64, cast to f32) the derived values are `0.34413627` and `0.71413624`, gaps `6.268e-6` and `3.755e-6`. The red and blue diagonal coefficients `1.402` and `1.772` differ from their f32 derived forms by under `5e-8`. The commit message's "the blue-to-green gap measured 6.27e-6 in f32" holds. Neither `colour.rs` nor `PATCH-PROVENANCE.md` states a number now, which the ruling allows, and "the same values" is gone from both.
- **AC25's coefficient contract against the gates (point 4).** In a scratch copy with its own target directory, `GAMMA_CB2G` moved from `0.34413` to `0.35413`, a 1e-2 step, against a fresh build. Result: `cargo test -p ocelli-codec --test htj2k` 18 passed including `inverse_ict_uses_g6_literal_coefficients`, and `cargo test -p ocelli-dicom --test corpus -- --ignored` 7 passed including both lossy colour rows. Only the vendored suite went red: `cargo test --offline --locked --manifest-path vendor/openjph-core-0.1.0/Cargo.toml --lib` failed 5 of 154, `ict_backward_is_the_annex_g_transform` and four published `ict_forward_backward_*` tests. GLM's observation reproduces. Then whether any gate runs that suite. It does not, see S1.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo test -p ocelli-codec -p ocelli-dicom` | worktree tip 7b51bdb6, `CARGO_TARGET_DIR=$PWD/target` | now | 0, 327 passed, 0 failed, 7 ignored (the corpus suite) |
| `python3 scripts/pin_and_size_check.py` | worktree | now | 0, `OK: wasm-bindgen, wgpu pinned exactly` |
| SHA-256 of `colour.rs`, `tile.rs`, `tile_comp.rs`, `PATCH-PROVENANCE.md` | worktree | now | `8d13e0db...`, `551f0f6f...`, `0585e142...`, `8619d0c0...`, each equal to `OPENJPH_PATCHED_HASHES`, `OPENJPH_PROVENANCE_SHA256` and the provenance digest lines |
| pins gate after a one-byte edit to each of the four files | scratch `git archive` of the tip | now | each exit 1 naming the file ("has undeclared changes beyond the exact D-31 colour-transform patch", or "does not match the exact reviewed patch record"), each back to exit 0 after restore. The `colour.rs` edit was the `0.34413` to `0.35413` digit |
| `guard_probe.py --only pins.openjph-inventory` | worktree | now | OK, red for its declared reason, control green |
| `GAMMA_CB2G` 1e-2 mutation through the codec test, the corpus rows and the vendored suite | scratch, own target | now | 18 and 7 green, vendored 149 passed 5 failed. Restored, hash back to `8d13e0db...`, vendored 154 passed, ritk 321 passed, pins OK |
| `cargo test -p openjph-core --lib` from the workspace | scratch | now | 101, "cannot be tested because it requires dev-dependencies and is not a member of the workspace" |
| `cargo metadata --no-deps` workspace members, `Cargo.toml` `exclude`, `bin/ocelli.sh gate --list`, `bin/ocelli.sh:108`, `ci.yml:61` | worktree | now | neither vendored crate is a member (both under `exclude`), the `test` gate and CI run `cargo test --workspace`, and no script, gate, guard probe or workflow names `--manifest-path vendor` or either vendored test name |
| `guard_census.py`, `guard_census.py --check-runbook` | worktree | now | 0 and 0, 0 watched by nothing |
| `prose_check.py`, `source_provenance_check.py`, `deviation_check.py`, `unsafe_allowlist_check.py` | worktree | now | 0, voice clean over 911 files. 0, clean over 1780 files. 0, 31 deviations. 0, openjph 104 and ritk 0 unchanged |
| `git grep` for the old test name, "exactly two decode-path", "6e-6", "the same values" | worktree | now | old name only in `.claude/reviews/F-239-codec-working-pass-1.md` to `-3.md`, which are historical records. No other hit in scope |
| `git status --porcelain [--ignored]`, `git diff --quiet HEAD`, `git reflog`, `git ls-files -m -d` | worktree | now | see Verified clean |
| GLM's self-review receipts | `.claude/scratch/F-239-self-review-11.md` to `-13.md` | prior | read, not relied on |

## Defects

None.

## Smells

### S1, AC25's 1e-4 coefficient contract is held by tests no gate runs
**Where**: `Cargo.toml:4` (`exclude = ["vendor/ritk-codecs-0.6.0", "vendor/openjph-core-0.1.0"]`), `bin/ocelli.sh:108` (`test) cargo test --workspace`), `.github/workflows/ci.yml:61`. The tests are `vendor/openjph-core-0.1.0/src/codestream/tile.rs` `rct_backward_bounds::ict_backward_is_the_annex_g_transform` and `vendor/ritk-codecs-0.6.0/src/jpeg_2000/packet/reader.rs` `inverse_colour_transform::inverse_ict_9_7_is_the_annex_g_transform`. The prose that relies on them: `crates/ocelli-codec/tests/htj2k.rs:349` to `:352` ("The exact coefficients are held by the vendored patch's unit test"), `crates/ocelli-codec/tests/jpeg2000.rs:471` to `:472`, `crates/ocelli-dicom/tests/corpus.rs:383` to `:386` ("pinned at a 1e-4 tolerance by the vendored patch's unit test"), `vendor/openjph-core-0.1.0/PATCH-PROVENANCE.md:49` to `:53`.
**What**: the pass 2 ruling's contract is that unit tests assert AC25's hand values within 1e-4 and go red under any one coefficient changed by more. They do go red, but only when someone runs them by hand. Both vendored crates are excluded from the workspace, `cargo test --workspace` does not reach them, `cargo test -p openjph-core` from the workspace refuses, and no gate, guard probe or CI step runs `--manifest-path vendor/...`. The tests that do run, the two `inverse_ict_uses_g6_literal_coefficients` tests and the seven corpus rows, stay green under a 1e-2 change to `GAMMA_CB2G`, so they cannot carry the contract, and say so in their own comments.
**Why it is wrong**: `/microscope` section 4, a test that looks authoritative and that nothing executes. What holds the reviewed coefficient bytes today is the pins gate's hash of `colour.rs` and `reader.rs`, which refuses any edit whatever its value. That guards the bytes and not the arithmetic. The first time those files are re-pinned, as this very range re-pinned `colour.rs`, every gate is green whether or not the coefficients are still G-6's. A re-vendor, an upstream bump or a typo landed with its re-pin passes `gate --sprint` with a coefficient up to at least 1e-2 wrong.
**Evidence**: the mutation above. `GAMMA_CB2G = 0.35413`: codec suite 18 green, corpus 7 green, vendored 5 red. `cargo metadata` lists neither crate as a member. `grep -rn manifest-path scripts bin .github` returns nothing for either vendored crate.
**Scope and regression**: present on the tip since the pass 2 remediation `7fcdd8f1`, not introduced by this range, and missed by pass 3, which ran both suites by hand and recorded them as manual evidence. No pixel is wrong today: the coefficients at the tip are G-6's, verified by reading `colour.rs:25` to `:28` and by the vendored test passing. Smallest repair, for the lead to place: have a floor gate run `cargo test --offline --locked --manifest-path vendor/openjph-core-0.1.0/Cargo.toml --lib` and the ritk equivalent with `CARGO_TARGET_DIR` outside `vendor/`, and add it to `ci.yml`, which the `ci` gate requires. Measured in scratch: `--locked` resolves against each crate's tracked published `Cargo.lock`, leaves it byte-identical, and the pins gate stays OK afterwards. A guard probe that applies the 1e-2 mutation and expects that step red would close the loop. Narrower alternative: run only the two named tests. The cheaper wrong fix is a corpus-level pin, which would contradict the bound design pass 2 D2 established.

## Nitpicks

- N1. `scripts/guards/catalogue.py:8346` to `:8347`, "The recorded D-31 patch covers the decode-path files." The count is gone as the ruling asked, and the sentence is true, but "the decode-path files" reads as every decode-path file. "covers the files its refusal names" would say exactly what it means.

## Pass 3 findings replayed

- **D1, closed.** The `pins.openjph-inventory` note has no count. `git grep -n "exactly two decode-path"` returns nothing, and the probe is still red for its declared reason.
- **D2, closed.** `colour.rs:22` to `:24` and `PATCH-PROVENANCE.md:35` to `:38` give no number and no "the same values". The comment change is the only change to `colour.rs`, and both openjph hashes were re-pinned and match the files.
- **N1, the provenance purpose clause, closed.** `PATCH-PROVENANCE.md:15` to `:21` now names both purposes, the inverse before the shift and the G-6 literals, so the opening sentence is true for all three files.
- **N2, the D-31 row, closed.** `DEVIATIONS.md:48` adds one clause: the four ICT coefficients pinned to G-6's literals in place of the forward matrix's derived ones. Four is right, the four `GAMMA_*` constants, and `deviation_check.py` is green.
- **N3, the rename, closed.** Both functions are `inverse_ict_uses_g6_literal_coefficients` (`htj2k.rs:323`, `jpeg2000.rs:459`), the plan's AC25 Tests cell at `F-239-design.md:407` follows, and the three `corpus.rs` references are updated. The old name survives only in the pass 1 to 3 review records, which `/microscope` says to preserve, and in no source, plan, LLD or doc.

## Verified clean

- **No arithmetic changed.** Every changed `.rs` and `.py` line in the range is one of: two `fn` renames, three comment renames in `corpus.rs`, the guard note, two hash constants, and the three-line comment in `colour.rs`. `GAMMA_*`, `ALPHA_*` and `BETA_*` are byte-identical to `2eaec747`. The 327 workspace tests and both vendored suites pass at the tip.
- **Every changed sentence executed.** The gap figure in the commit message, the provenance's purpose clause and its "OpenJPEG applies the same literals, and the forward path is untouched" (forward constants untouched, OpenJPEG agreement carried from pass 3's measurement), the D-31 clause, the guard note, the plan row. No added line holds an em-dash, an en-dash or a prose semicolon, and `prose_check.py` is green.
- **Pins exact and live.** All four openjph files hash to their records, and a one-byte edit to each turns the pins gate red naming the file.
- **Tree intact after GLM's `rm -rf`.** `git status --porcelain` shows only `.venv` and `tools/oracle/out`. `git diff --quiet HEAD` is true. `git ls-files -m -d` is empty. `--ignored` shows only `.claude/scratch/`, `corpus/data`, the two wasm pkg directories, `__pycache__` directories and `target/`. The reflog after pass 3's base `bc50dcda` holds exactly the pass 3 review commit `ee66eed3` at 20:03:16, the lead ruling `2eaec747` at 20:03:29 and GLM's `7b51bdb6` at 21:41:20, with no reset, checkout or amend. The one stash is from `sprint/s17`. The `rm` named a path outside the worktree, and the hash checks above cover every vendored patched file.
- **Unchanged consumers.** No signature changed. `README.md`, `use.html` and `more.html` describe nothing this range touched, and no LLD carries the old test name or the deleted bound.

## Limitations and handoff

- The ritk 1e-2 mutation was not re-run this pass. GLM's cycle 11 reports the same shape for `reader.rs:468` (the `.91` row green, the vendored test red), and S1's reasoning applies to ritk on the exclusion alone.
- Not exercised: a real CI run, and any oracle, GPU or browser job, by instruction.
- GLM's cycle 11 note labels the blue-to-green value `0.3441362679` as "cr2g". That file is ignored scratch and out of scope, and the tracked commit message names the coefficient correctly.
- This pass's scratch copy and target directory were deleted with Python `shutil`.
- Next owner: the lead rules on S1, then implementation repairs it and a fresh independent pass 5 reviews the changed material.

## Self-review catch figure

GLM's cycles 11 to 13 should have caught **1 of 1** blocking findings. Cycle 11 reproduced S1's symptom, the corpus rows blind to a 1e-2 step, and flagged it, but rested on "the 1e-4 coefficient contract belongs to the vendored unit tests" without checking whether any gate runs them, which `gate --list` and `Cargo.toml`'s `exclude` answer in one step. Counted as surfaced but not caught.
