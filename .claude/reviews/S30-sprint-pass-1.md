# S30 review, pass 1

**Reviewed**: `26f003c45bac07c07b15cad82079c355a66823a2...a3f1cf0b0ec1a759ef826114488ec5ec7deb9953` on `sprint/s30`. HEAD is `a3f1cf0b0ec1a759ef826114488ec5ec7deb9953`. HEAD tree is `09ffa29c8af30a59ffd6d5f00910f4bbb09dfac4`. `git write-tree` is that same tree. The worktree was clean. This pass edited no product file.
**Author / reviewer**: The feature trailers record `Ocelli-Generated-By: grok`. A generated trailer does not name a person. This reviewer did not write F-080, F-084, the plans, or the story reviews, and did not resume those reviewers.
**Result**: 0 defects, 0 smells, 0 nitpicks

## Acceptance challenged

The HLD Appendix B rows count tool classes and the three segmentation representations. They do not define a brush or a run. `docs/hld/03-architecture-and-crates.md` names `ocelli-seg` and does not give these functions. The approved plans are the contract. No deviation was added. `docs/hld/DEVIATIONS.md` is not in the diff.

F-080. A sample is inside when squared index distance is less than or equal to squared radius. On a 3 by 3 circle at `(1, 1, 0)` with radius 1, the four edges have distance squared 1 and the four corners have distance squared 2. The stamp is the plus, covered 5, corners left 0. Radius `next_down` from 1 has square `0.9999999999999998`, so those edges stay out and covered is 1. A sphere with the same centre and radius paints that plus on slice 0 and only the centre on slice 1, covered 6. A sphere at `z` 0.5 with radius 0.5 paints both centres and no neighbour, covered 2. A circle at `z` 0.5 paints nothing. Radius 0 paints only an integer centre. Both zero bit patterns are a zero radius and select slice 0. A negative radius is `Degenerate`. A non-finite centre or radius is `Frame`. Either refusal leaves the samples as they were. Segment 0 erases. A second erase of that centre reports `changed` 0.

F-084. On the plan grid, `(1, 0)` reads 1, `(0, 0)` reads 0, and `(4, 0)` is `Degenerate`. A short buffer is `Degenerate`, not a background read. Segment 0 is `Background`. Blank text is `Blank`. A refusal leaves an existing label in place. The longest runs are row 2 length 4 and column 1 length 3. Equal length keeps the earlier run, so the top length-2 row wins and the left full column wins. One sample is length 1 on both axes. `ocelli-geom` `bidirectional` is not called.

Cross-story. The stamp and the reader share row-major `y * width + x`. `present_labelmap` uses that same offset. After the radius-1 circle of segment 2, the longest runs are the middle row `(0, 1)` through `(2, 1)` and the middle column `(1, 0)` through `(1, 2)`, both length 3. Erasing the centre of a 3 by 3 grid of 1s leaves a 0 there. `segment_at` of that centre is a successful 0. `label_segment` of 0 is `Background` and does not store a name. `segment_bidirectional` of 0 is `Degenerate` before any run is built. The remaining 1s have two length-3 runs on each axis, and `length > current.length` keeps the top row and the left column. A later equal run does not replace the earlier one. The presenter test stamps segment 1, with fill on and outline off. The centre is linear index 4, so bytes 16 to 19 are the LUT colour. The corner is segment 0 and stays alpha 0. Outline alpha 255 is not used.

These observations reject an open ball, a stamp that paints a corner, a circle that paints every slice, a sphere that skips slice 1, a fractional circle that paints a slice, a short buffer reported as background, a segment 0 label, a later equal run replacing the earlier one, and a refusal that writes before it returns.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `git rev-parse HEAD`, `HEAD^{tree}`, `git write-tree`, `git status --porcelain` | `sprint/s30` before this file | Executed now | HEAD `a3f1cf0b0ec1a759ef826114488ec5ec7deb9953`. Both trees `09ffa29c8af30a59ffd6d5f00910f4bbb09dfac4`. Status empty |
| `git diff --name-status` and `git diff --stat` `26f003c...HEAD` | Four commits: `14e6dcf`, `ba6f395`, `91b7af1`, `a3f1cf0` | Executed now | 20 paths. 1971 insertions, 57 deletions. No `.ts` path |
| `git diff --stat` on the frozen paths | `graph.rs`, `voi.wgsl`, `fusion.wgsl`, `raycast.wgsl`, `cpu.rs`, `produce.rs`, `produce_current.rs`, `ocelli-compare.rs`, `labelmap.rs`, `contour.rs`, `surface.rs`, `measure.rs`, `annotation.ts`, `tools.ts`, `pointer.ts`, `cursor.ts`, `DEVIATIONS.md`, `ci/wasm-size-budget.json` | Executed now | Empty. `labelmap.rs` and `measure.rs` blob ids match the base |
| `shasum -a 256` of `brush.rs` and `segment_tools.rs` | HEAD | Executed now | `a8756735471dd7787d2538daf1e61e12529576ab0425571984be0a76d27f96dc` and `f087933c6e2e0216d8f26d4e33909682d812fbaf8374c776994cc844b145898d`. Both AS_BUILT notes match |
| `git show` of the two feature commits and `.claude/verify-ledger.json` | Trees `dd592d802409b4358acf68df9a21ccde6705b29c` and `09ffa29c8af30a59ffd6d5f00910f4bbb09dfac4` | Executed now | Both records are `profile=feature`, `corpus=pass`, 27 gate names, and no comparison object. The trailers name those tree prefixes. This is not comparison evidence |
| `python3 scripts/backlog_check.py` | HEAD `BACKLOG.md` | Executed now | Exit 0. `OK: 213 F-IDs, 107 done, 5 hooks scheduled in Phase 1, 2 import defects declared`. The parser status for F-080 and F-084 is `done`. Base done lines were 105 |
| Backlog status rows and the sprint table patterns | `BACKLOG.md` and `CURRENT_SPRINT.md` | Executed now | F-081, F-082, F-083, F-085 and F-086 are `pending`. F-X003 is `in-progress`. F-063 and F-066 are `done`. The two `^| F-[0-9]` rows and the two `^| F-[0-9X]` rows are F-080 and F-084, both `done`. S30 backlog rows are those two, both `done` |
| M9 estimate sum | F-080 through F-086 | Executed now | 7 stories. 4, 4, 3, 4, 3, 3 and 3 sum to 24. S30, S31 and S32 |
| Python float64 model of the stamp and the run scan | Plan numbers. `<=` stands in for `total_cmp` `is_le` on these finite values. Not a Rust call | Executed now | Plus covered 5. Sphere covered 6 with only the next centre. Under-radius covered 1. Halfway covered 2. Fractional circle covered 0. Erase covered 1. Plus runs are length 3 on the middle row and middle column. After the erase, segment 1 keeps the top row and the left column |
| `bin/ocelli.sh test ocelli-seg --test brush --test segment_tools` | HEAD sources above. One cargo process | Executed now | Exit 0. `brush.rs` 18 passed, 0 failed. `segment_tools.rs` 13 passed, 0 failed. No ignored tests |
| Story-review mutations | F-080 pass 1 `is_le` to `is_lt`. F-080 pass 2 negative-zero clause removed. F-084 pass 1 `>` to `>=`. F-084 pass 2 segment 0 return removed | Prior receipts. Not run again | Those reports say the named tests went red and the sources were restored to the hashes above |

Environment: macOS, zsh, rustc 1.97.1 (`8bab26f4f` 2026-07-14), Python 3.14.5. No pipe was used for the cargo status. The sprint gate was not run. wasm-pack was not run. `compare.json` was not regenerated. No patient data was used. dwv, Horos, and the medical viewer Grok were not opened.

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

`stamp_brush` refuses a non-finite centre or radius before a negative-radius check, and both return before a write. `strictly_negative` excludes the negative-zero bit pattern. `same_number` treats both zero bit patterns as one slice index before `total_cmp`. The distance test is `dist_sq.total_cmp(&radius_sq).is_le()` at `crates/ocelli-seg/src/brush.rs` line 154. There is no `sqrt`. A circle skips every slice whose index is not that number, including a fractional `center_z`. A sphere visits every supplied slice. Staged copies replace `indices` only after the loop. A bad length, a zero side, an empty stack, or a mismatched slice returns `Degenerate` before that assignment. Segment 0 is a legal written value.

`segment_at` returns the stored number, including 0, only after the length check and an in-range offset. An outside column, an outside row, a zero side, a short buffer, and a long buffer are `Degenerate`. `label_segment` returns `Background` or `Blank` before it changes the list, and a replacement keeps the existing place. Both run sites use `length > current.length`, at `crates/ocelli-seg/src/segment_tools.rs` lines 192 and 233. There is no `>=`. Segment 0 returns `Degenerate` before either scan. The new files have no `as` cast, no `unsafe`, and no `wasm_bindgen`.

`lib.rs` stays `no_std` outside tests. It re-exports the brush and the segment tools and still exports `present_labelmap`. The F-084 commit does not change `brush.rs`. The F-080 commit does not add `segment_tools.rs`. Neither commit changes the presenter, `measure.rs`, or a TypeScript tool. `packages/core` has no `stamp_brush`, `segment_at`, `label_segment`, or `segment_bidirectional`.

`AS_BUILT.md` gained 32 lines and lost none. HEAD starts with the base text. The F-075 and F-079 sections are that unchanged prefix. The new F-080 and F-084 notes carry the hashes above. `CHANGELOG.md` adds the two bullets and does not call either rule an HLD formula. `CURRENT_SPRINT.md` marks both stories done and says closing this sprint does not complete M9 and does not publish. The wasm budget file still records 2151196 bytes and tolerance 0.05. The carried ceiling 2258755 is the integer truncation of `2151196 * 1.05`. That file was not edited.

## Limitations and handoff

This pass is not sprint verification and not a close. M9 is not complete. Nothing was published. The sprint gate was not run. No adapter was opened. The feature receipts are feature-profile records with corpus pass and no comparison object. Do not describe them as comparison evidence. Do not push from this tree.

The cargo command was not repeated, and no constant was mutated in this pass. The prior mutation receipts were not re-run. The composed erase-then-run grid was executed as the same index arithmetic in Python, not as one Rust call. A grid whose linear index does not fit in `u32`, a stack of more than `u32::MAX` slices, and `covered` past `u32::MAX` were not allocated. Height 0 was not given its own call. It shares the zero-side check with the width 0 case that passed.

This file is a new untracked review. It was not staged and it was not passed to `scripts/sprint_workflow.py record-sprint-review`. A receipt on tree `09ffa29c8af30a59ffd6d5f00910f4bbb09dfac4` would not include this file. Do not record that tree as if it contained this review. Do not publish from this sprint.

S30 pass 1: D=0 S=0 N=0
