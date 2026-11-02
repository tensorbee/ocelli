# S32 review, pass 1

**Reviewed**: `a7828da2dd3876142f015a1cb57c6f55e70f8d24..c0eac3d938d7be01d925738b7089691cf9af4480` on `sprint/s32`. The base is `Merge sprint/s31, M9 is not complete`. HEAD is `F-086, between-slice labelmap interpolation`. `git rev-parse HEAD^{tree}` is `e9f504c7d13e261e4c793f6c13292aee7137456b`. That is the index of HEAD. `git status --short` was empty at the start and after every temporary mutation was restored. This review file is not part of that tree.
**Author / reviewer**: `git log` names Atul Sharma for the three commits. The author of the implementation is the parent implementation agent. The reviewer is independent subagent `01a19dd3-11dd-7421-ba7c-10684a280058` and did not write the open, the design, the feature diff, or `.claude/reviews/F-086-interpolate-pass-1.md`. Reviewer `01a19dc1-b14f-7ea0-b9a3-678dba7b8102` was not resumed. That feature review was read only as an artefact, and its code claims were checked against this tree. This pass is not that review and it is not a self-review.
**Result**: 0 defects, 0 smells, 0 nitpicks

## Acceptance challenged

The local plan is `.claude/plans/F-086-design.md`. `docs/hld/B-parity-surface.md` counts the tool classes and the three segmentation representations. It does not define an interpolation. `docs/hld/03-architecture-and-crates.md` names `ocelli-seg` and does not give this function. The halfway fraction is the approved plan. It is not an HLD formula and it is not a PS3.3 formula. PS3.3 C.8.20.2 is only the background rule for segment 0.

AC1 to AC8, the five-slice consecutive-pair case, and the zero-side and short-buffer refusals were checked. The consumer called beside the new function is the unchanged `present_labelmap`. `stamp_brush` is not called.

AC2 was challenged by editing only the low-side compare from `is_le` to `is_lt` and running `the_halfway_slice_takes_the_union`. The test went red at `crates/ocelli-seg/tests/interpolate.rs:55`. `coverage.covered` was 1 against 2. At span 2, `t` is exactly one half, so column 0 is the low key's contribution. Dropping `Equal` from the low side leaves column 2. The pixel assertion on line 57 did not run. The source was restored from the saved blob and the nine tests passed again.

The plan's AC2 rejects a dropped union and a painted centre. It does not name a clear of unselected samples to 0. The fixture middle starts at `[0, 0, 0]`, so column 1 is already 0. A temporary edit that stored 0 in every unselected between-slice sample left all nine permanent tests green. That result is an accepted limit of the plan's fixture. It is not a defect and not a smell. The shipped function does not clear those samples. It leaves the staged clone. A removed probe started the middle at `[9, 7, 1]` and expected `[4, 7, 4]`. On the restored function the probe passed and the 7 stayed. The same clear then failed that probe with left `[4, 0, 4]` and right `[4, 7, 4]`.

AC1 is the column where both keys hold the segment. Ends `[4]` and middle `[0]` become middle `[4]`, with covered 1 and changed 1. An exclusive-or of the two masks would leave that middle at 0, so the permanent test rejects it. A removed probe also used low `[4, 4]`, high `[4, 0]` and middle `[1, 2]`. The halfway union is `[4, 4]`. The outer keys stayed. The probe passed.

A span of 4 was computed before the probe ran. Low is `[4, 0]` and high is `[0, 4]`. The three between-slices start at `[3, 3]`. Step 1 is one quarter and takes only the low mask, so the slice becomes `[4, 3]`. Step 2 is one half and becomes `[4, 4]`. Step 3 is three quarters and becomes `[3, 4]`. Covered is 4 and changed is 4. The keys stay. The probe passed.

Changing `1..span` to `0..span` includes the lower key. AC1 covered was 2 against 1 at `crates/ocelli-seg/tests/interpolate.rs:40`. The permanent test rejects that endpoint write. The source was restored.

Fewer than two keys returns covered 0 and changed 0 before any staged copy is assigned. The permanent one-key fixture locks that return, and the non-key sample stays 7. The probe ran `[1]` and `[2]` with segment 4. Both counts were 0 and both values stayed. Adjacent keys take the `span < 2` continue. The probe observed changed 0 there. The permanent adjacent fixture already locks covered 0 and both indices.

A height mismatch returned `Degenerate` and left `[4, 9]` and `[4]`. A width mismatch on another stack returned `Degenerate` and stayed mismatched. A later valid stack still wrote 4 into its middle. The refusal did not stick, and the refused stack was not repaired by the later call.

The five-slice case was checked from the plan, not from the function's output. Pair `(0, 2)` has span 2, so `t` is one half and the union of `[4, 0]` and `[0, 4]` is `[4, 4]`. Pair `(2, 4)` is the union of `[0, 4]` and `[4, 0]`. Slice 2 is a key and stays `[0, 4]`. Four samples are selected and none already hold 4, so covered is 4 and changed is 4. An outer span of 4 alone would write `[4, 0]` on the between-slices and would rewrite the middle key. The permanent test rejects that.

AC6 covered 0 follows the key rule. After AC1 the middle holds 4, so the second call has a key on every slice. Nothing lies strictly between consecutive keys. The plan's AC6 row requires the second changed to be 0. Covered 0 is the same rule. It is not a missing count of the new key.

The false arm of `if *slot != request.segment` is unreachable on a fill of a strict gap. A slice that already stores the segment is a key, and only a strict gap is written. Each gap slice is visited once. The arm is the plan's split between `covered` and `changed`. It is not a guard that claims to reject an input. The second-call test and the adjacent-key path execute the rule that makes the arm unreachable. That is not a defect and not a smell.

`f64::from` of a `u32` is exact. One half is exact when `2 * step == span`. Python floats, which are IEEE doubles, were compared with the integer side test on every step of each span from 2 through 3999, and on the end, half and third steps of spans through 7999 and of `2**16`, `2**20`, `2**24`, `2**31 - 1`, `2**31`, `2**32 - 2`, `2**32 - 1`, `10**9` and `3 * 10**9`. That was 8022054 ratios, 0 side changes, and 0 exact halves that were not 0.5. A non-equal `u32` ratio is at least `1 / (2 * span)` away from one half. For every `u32` span that distance is far larger than one `f64` unit in the last place at one half.

An empty stack, a zero side, a short buffer and mismatched widths hit `require_stack` and return `Degenerate` before assignment. `require_stack` in `crates/ocelli-seg/src/brush.rs` returns only `BrushError::Degenerate`. The mapping drops no `Frame`. The plan has no `Frame` variant. The mismatch and short-buffer fixtures keep the caller's indices.

AC7 writes segment 0 through the same halfway rule. The middle becomes 0. `present_labelmap` is unchanged. At `crates/ocelli-seg/src/labelmap.rs` the segment 0 branch writes RGBA 0 before any LUT lookup. The fixture buffer starts at 7 and the assertion expects `[0, 0, 0, 0]`. `labelmap.rs` is not in the diff.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `git rev-parse HEAD`, `git rev-parse HEAD^{tree}`, `git log --oneline a7828da2..HEAD`, `git diff --name-only a7828da2..HEAD`, `git status --short` | `sprint/s32` at review start | Executed now | HEAD `c0eac3d938d7be01d925738b7089691cf9af4480`. Tree `e9f504c7d13e261e4c793f6c13292aee7137456b`. Commits `0ca7a8d` open, `2a33170` design, `c0eac3d` feature. 12 paths. Status empty |
| `git diff --stat` on `brush.rs`, `labelmap.rs`, `contour.rs`, `surface.rs`, `scissors.rs`, `fill.rs`, `threshold.rs`, `contour_edit.rs`, `crates/ocelli-render/shaders`, `annotation.ts`, `tools.ts`, `pointer.ts`, `DEVIATIONS.md`, `ci/wasm-size-budget.json`, `Cargo.toml` | Same range | Executed now | Empty. Those paths are not in the sprint diff |
| `bin/ocelli.sh test ocelli-seg --test interpolate` | HEAD `interpolate.rs` and `tests/interpolate.rs`, before any mutation | Executed now | Exit 0. 9 passed, 0 failed, 0 ignored. Names: `a_middle_slice_takes_the_shared_end_segment`, `the_halfway_slice_takes_the_union`, `a_near_slice_takes_only_the_closer_mask`, `one_key_and_adjacent_keys_write_nothing`, `an_empty_stack_and_a_width_mismatch_write_nothing`, `a_second_interpolation_changes_nothing`, `interpolating_background_clears_the_presenter`, `a_slice_past_the_upper_key_stays`, `consecutive_pairs_do_not_use_the_outer_keys` |
| Low side `is_le` changed to `is_lt`, then `bin/ocelli.sh test ocelli-seg --test interpolate the_halfway_slice_takes_the_union` | Working copy only. Saved blob `/tmp/f086-src.rs`. High side stayed `is_ge` | Executed now | Exit 101. Failed at `tests/interpolate.rs:55`, covered left 1 and right 2. 8 filtered out |
| Unselected between-slice samples assigned 0, then the same nine-test command | Working copy only. The `continue` arm called `staged_slot` and stored 0 | Executed now | Exit 0. 9 passed, 0 failed. The plan fixture does not see this edit |
| `for step in 1..span` changed to `0..span`, then `a_middle_slice_takes_the_shared_end_segment` | Working copy only | Executed now | Exit 101. Failed at `tests/interpolate.rs:40`, covered left 2 and right 1 |
| Removed `review_probe_sprint` on the restored function | Overlap row, foreign centre `[9, 7, 1]`, span 4 with stored 3, zero keys, adjacent changed, height mismatch, then a valid stack after a width mismatch | Executed now | Exit 0. 1 passed. 9 filtered out. Foreign middle stayed `[4, 7, 4]`. Span 4 became `[4, 3]`, `[4, 4]`, `[3, 4]` |
| Same probe after the clear-to-zero edit | Probe file still present. Product source still the clear edit | Executed now | Exit 101. Failed at the foreign-centre assertion, left `[4, 0, 4]` and right `[4, 7, 4]` |
| `cmp` of both saved blobs, `shasum -a 256` of `interpolate.rs`, restored `bin/ocelli.sh test ocelli-seg --test interpolate`, `git status --short`, `git rev-parse HEAD^{tree}` | Restored HEAD bytes | Executed now | Both `cmp` checks matched. SHA-256 `e0853470e923b75f30fd8b2fbd6c1e0e8439219e1f242cb562f87c6223915982`. Tests exit 0, 9 passed. Status empty. Tree still `e9f504c7d13e261e4c793f6c13292aee7137456b` |
| Python ratio check described above | Not the crate. Python `float` | Executed now | Exit 0. 8022054 checks, 0 disagreements |
| Added-line scan for em-dash and semicolon, and the same scan of `git log --format=%B a7828da2..HEAD` | Markdown paths in the range, plus the three commit messages. Rust punctuation was not treated as prose | Executed now | No em-dash and no prose semicolon in the added markdown or the commit messages |
| F-ID list compare | `docs/lld/public-api.md` contributing line and the last cell of the `public-api.md` row in `docs/lld/README.md` | Executed now | Equal. 25 ids, both ending in F-086, same order |
| `git rev-parse s31`, `s31^{commit}`, `a7828da2^2`, and those trees | Tag and S31 merge | Executed now | Tag object `f894be05fde3d974146d59fa62226adbf3b67145` peels to `a7828da2dd3876142f015a1cb57c6f55e70f8d24`. Sprint tip `9ab950933c1ca6f39720a242dbee961c32cec39c`, tree `fb86414d342f875f6186b7cd36bb5d9565007639` |
| Ledger key `e9f504c7d13e261e4c793f6c13292aee7137456b` and the `c0eac3d` trailer | `.claude/verify-ledger.json` and `git log -1 --format=%B c0eac3d` | Prior receipt, read now, gates not re-run | `profile=feature`, `corpus=pass`, the same 27 gate names, no comparison object. Trailer tree prefix `e9f504c7d13e` |
| `int(2151196 * (1 + 0.05))` against `scripts/pin_and_size_check.py` | `ci/wasm-size-budget.json` bytes 2151196 and `TOLERANCE = 0.05` | Executed now for the formula only | The integer is 2258755. The wasm gate was not run |

Environment: macOS, zsh, workspace `/Users/atulsharma/Documents/projects/tensorbee/ocelli`, shell date 2026-11-02. `rustc` 1.97.1 and `cargo` 1.97.1. The cargo test commands ran one after another. No wasm-pack ran. No `bin/ocelli.sh gate` ran. No patient data was used. dwv, Horos, and the medical viewer Grok were not opened.

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

`interpolate_labelmap` is the only new entry point. A slice is a key when `indices.contains` finds the caller segment. Consecutive keys come from `windows(2)` in slice order. `step` runs through `1..span`, so endpoints are not written. `t` is `f64::from(step) / f64::from(span)`. The low mask uses `total_cmp` `is_le` against one half and the high mask uses `is_ge`, so the exact half is the union and a nearer slice takes one mask. Samples outside both masks stay on the staged clone. Counts use `saturating_add`. Fewer than two keys returns covered 0 and changed 0 without assigning. `require_stack` still rejects an empty stack, a zero side, a bad length and a mismatched rectangle, and every such error is `InterpolateError::Degenerate`. The grids are replaced only after the walk returns. Endpoint masks are read from the caller's original indices. There is no `Frame` variant.

`fill_slice` has seven arguments. `FillRequest` holds `t: f64` and does not derive `PartialEq`. `InterpolateCoverage` is two `u32` fields and derives `PartialEq` and `Eq`. No new trait, generic or `Box<dyn>`. The new Rust has no `unsafe`, no `wasm_bindgen`, no `sqrt`, no glam, no `unwrap` and no `expect`. The only `as` hits are the English word in two doc comments. Tests use `unreachable!` for fixture shape. The crate stays `no_std` outside tests and the new module uses `alloc::vec::Vec`.

The new changelog bullet, the F-086 paragraph in `docs/lld/public-api.md`, and the `lib.rs` crate sentence match the nearer-mask and halfway-union rule. The changelog still says nothing is published. The F-ID header and the README row match. There is no TypeScript file in the diff and no TypeScript, JavaScript or shader name of `interpolate_labelmap`. The function does not open a device and does not mention a tier. Plan tiers A, B and C stay not applicable, and the `AS_BUILT` row says the same. `brush.rs`, `labelmap.rs`, `contour.rs`, `surface.rs`, `annotation.ts`, `tools.ts` and `pointer.ts` are absent from the range. `present_labelmap` was only called.

`BACKLOG.md` marks F-086 `done`. `CURRENT_SPRINT.md` marks that same row `done`. The open says M9 is not complete until this sprint closes, and that closing completes M9. It does not say M9 is already complete and it does not say this feature commit publishes. The version files are not in the diff. `Cargo.toml`, `packages/core/package.json` and `packages/react/package.json` still say `0.1.0`. D-22 remains in `docs/hld/DEVIATIONS.md`, which is not in the diff, and the carried sentence still says that block stops publication and does not stop this sprint. `F-X003` stays `in-progress`. `F-X011` stays pending.

The M9 count in the open is the seven backlog rows F-080 through F-086. Their estimates are 4, 4, 3, 4, 3, 3 and 3 weeks, which sum to 24. That is the planned total already written in `docs/sprints/SPRINT_PLAN.md`. It is not a measured duration and it is not a ratio. The F-086 tracker cell begins `not measured` and gives no day count and no ratio.

`AS_BUILT.md` calls the `c0eac3d` record a feature profile, names floor plus the corpus gate, and says the sprint push needs a later sprint-profile record with comparison evidence. The commit trailer is `profile=feature`, 27 gates, `corpus=pass`, tree prefix `e9f504c7d13e`, and it has no comparison field. Those 27 names are every gate in `bin/ocelli.sh` except `oracle`, `gpu`, `guards-deep` and `quirk-mutations`. The local ledger entry for this tree matches the trailer and has no comparison object. That receipt was not re-run. It is not the sprint verification.

The feature review's `interpolate.rs` digest matches this HEAD blob. Its reported low side is still `is_le`. Its reported test names are the nine that passed here. Its write-tree `a43234b928af504b2f0d1e3c956d03a6b6110c3c` is the index it reviewed before the sprint rows were committed. This pass reviewed the committed range, not that earlier index.

The checked states are one key, zero keys, adjacent keys, an empty stack, a width mismatch, a height mismatch, zero width, zero height, a short buffer, a repeat after AC1, segment 0 followed by the existing presenter, a slice past the upper key, exact half, span 3, span 4, an overlapping halfway column, a foreign unselected centre, two consecutive pairs around a middle key, and a refusal followed by a valid call.

## Limitations and handoff

This is a bounded code and prose review of the named range. It is not a sprint-profile record and it is not the close. `python3 scripts/sprint_workflow.py record-sprint-review` was not run. Closing S32 would complete M9 and would not publish. This pass does not close it. Version stays 0.1.0. D-22 still blocks publication.

The clear-to-zero edit is an accepted limit of the plan's AC2 fixture, as decided above. The shipped function keeps the foreign centre. No permanent test starts an unselected between-slice sample at a value other than 0.

Checked: the plan rules in `interpolate_labelmap`, the nine permanent tests and the mutations above, the doc rows named in the assignment, the feature trailer against the ledger entry, and the frozen path list. Unchecked: `bin/ocelli.sh gate`, the rest of `ocelli-seg`, wasm, native, the oracle, the corpus, and a GPU. The ignored `tools/oracle/compare-out/compare.json` was not rewritten by this pass. Its mtime is before the S32 open. It is an identity report, not a file this pass hashed back to the S31 gate digest. No comparison trailer was added by `c0eac3d`.

A grid whose `width * height` exceeds `u32::MAX` was not allocated, so the checked-arithmetic `Degenerate` arms were read and not forced. The historical feature-review index `a43234b928af504b2f0d1e3c956d03a6b6110c3c` was not reconstructed. The removed probe was not left in the tree.

No blocking finding. The product tree was left as found. This file is a new unstaged review and was not staged. A clean bounded review is not sprint verification and it is not sprint readiness.
