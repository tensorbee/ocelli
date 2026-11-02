# F-084 review, pass 1

**Reviewed**: branch `sprint/s30`, HEAD `91b7af1b4c9ae9698a54fb71591dd2b1e20feb81` (`F-080, circle and sphere brush`). The index tree is `dd592d802409b4358acf68df9a21ccde6705b29c`, the same tree as `HEAD^{tree}`, because the F-084 edits are unstaged and the two new Rust files are untracked. This pass reviews the working tree. Unstaged: `CHANGELOG.md`, `crates/ocelli-seg/src/lib.rs`, `docs/lld/README.md`, `docs/lld/public-api.md`. Untracked: `crates/ocelli-seg/src/segment_tools.rs`, `crates/ocelli-seg/tests/segment_tools.rs`. The approved contract `.claude/plans/F-084-design.md` is already committed and is not in the diff.
**Author / reviewer**: the assigned author is grok. This reviewer did not write the segment tools, the fixtures, the changelog bullet, or the LLD sentences. This pass did not resume reviewer `01a19cb6-214d-7b10-89ff-19537da639e5`, reviewer `01a19cc7-d0c1-78d1-8392-feb38c898891`, or reviewer `01a19cd3-d87c-7531-99a0-c421f131e996`. There is no F-084 commit, so this pass does not take an author from a trailer. This is not a self-review and it is not a sprint review.
**Result**: 0 defects, 0 smells, 0 nitpicks

## Acceptance challenged

The contract is the approved plan. Appendix B only counts tool classes and names the three representations. It does not define select, label, or a bidirectional run. No deviation was filed. `ocelli-geom` `bidirectional` was not used as a formula.

AC1 through AC9 are the nine permanent tests, in source order. The expected runs were recomputed from the plan grids, not taken from the implementation's output. On the 4 by 3 grid `0 1 1 0 / 0 1 0 0 / 1 1 1 1`, `(1, 0)` is 1, `(0, 0)` is 0, and `(4, 0)` is outside. The bottom row is `(0, 2)` through `(3, 2)`, length 4. Column 1 is `(1, 0)` through `(1, 2)`, length 3. The AC6 grid `1 1 0 / 0 1 1` has two horizontal runs of length 2. The top run is the one the plan keeps. The AC7 grid is two full columns, so the left column is the one the plan keeps.

The permanent AC6 assertion was mutated. Only the horizontal replacement at `segment_tools.rs:192` was changed from `length > current.length` to `length >= current.length`. The vertical site at line 233 stayed `>`. `an_equal_horizontal_run_keeps_the_upper_one` failed at `tests/segment_tools.rs:122`, which is `runs.horizontal.start_x`. The assertion was left 1, right 0. That left value is the start column of the lower run `(1, 1)` through `(2, 1)`. The test stopped there, so this run did not also observe `start_y` for that case. The same binary failed `an_equal_vertical_run_keeps_the_left_one` at line 137, which is `runs.horizontal.start_y`, left 1, right 0. The vertical assertions above that line did not fail, so the left column still won. The horizontal `>=` is what selected the lower row of the all-ones 2 by 2 grid. The comparison was restored to `>` before any later run. Both length sites are `length > current.length`. The only `>=` left in `segment_tools.rs` is the outside check `x >= grid.width || y >= grid.height` at line 144.

One temporary test, `a_refused_zero_label_leaves_an_existing_label`, was added after the restore and deleted before this report. It is not in the tree. It rejected four wrong behaviours the permanent nine tests do not name:

- A label of segment 0 on a list that already holds segment 3 `liver` returns `Background` and leaves that entry in place. A later label of segment 4 `spleen` still appends. That rejects a refusal that clears the list, overwrites the text, inserts segment 0, or blocks the next accepted label.
- The row `1 0 1` keeps the left sample, start 0, end 0, length 1. That rejects a run of length 3 across the gap and rejects the later equal sample.
- A 2 by 2 grid with five indices, the last of them 9, is `Degenerate` from both `segment_at` and `segment_bidirectional`. That rejects reading the first stored 1, which is what a short-only length check would do.
- Width 0 and row 3 on the 4 by 3 grid are `Degenerate` from `segment_at`. That rejects `Ok(0)` for a zero side and for an outside row.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `bin/ocelli.sh test ocelli-seg --test segment_tools` | Working tree before the mutation. `segment_tools.rs` SHA-256 `f087933c6e2e0216d8f26d4e33909682d812fbaf8374c776994cc844b145898d`. Test file SHA-256 `5feefd8e29e44697fff40a0d7043a50df1cb6a3b5aa07bc5fcd803976cc1519d` | Executed now | `code=0`. 9 passed, 0 failed, 0 ignored |
| Same command after changing only line 192 to `length >= current.length` | Recompiled `ocelli-seg` in 0.28s. Line 233 still `length > current.length` | Executed now | `code=101`. 7 passed, 2 failed. Horizontal tie failed at line 122, left 1, right 0. The all-ones tie failed at line 137, left 1, right 0 |
| Restore of line 192, then `shasum -a 256` | Same path | Executed now | Both length sites are `length > current.length`. `segment_tools.rs` hash returned to `f087933c6e2e0216d8f26d4e33909682d812fbaf8374c776994cc844b145898d` |
| Same test command with the temporary tenth test | Restored library source. Temporary test only | Executed now, then the test was deleted | `code=0`. 10 passed, 0 failed. The four extra cases above passed |
| `shasum -a 256` after deleting the temporary test | Working tree at the end of the pass | Executed now | Test file hash returned to `5feefd8e29e44697fff40a0d7043a50df1cb6a3b5aa07bc5fcd803976cc1519d`. Library hash unchanged. `brush.rs` is `a8756735471dd7787d2538daf1e61e12529576ab0425571984be0a76d27f96dc` |
| `git diff --stat` and `git diff` of the forbidden paths | HEAD `91b7af1b4c9ae9698a54fb71591dd2b1e20feb81` | Executed now | Four tracked files, 12 insertions and 3 deletions. `labelmap.rs`, `contour.rs`, `surface.rs`, `measure.rs`, `annotation.ts`, and `brush.rs` have an empty diff |
| `rg` for `stamp_brush`, `present_labelmap`, `ocelli_geom`, `glam`, `wasm_bindgen`, `unsafe`, `unwrap`, and `expect` in the two new Rust files | Working tree after restore | Executed now | No call and no import. The only `as` hits are the words "as background" and "as" in comments |
| `rg` for the three new function names under `*.ts`, `*.tsx`, and `*.js` | Working tree | Executed now | No hit |
| Appendix B rows | `docs/hld/B-parity-surface.md` lines 25 and 29 | Read now | Tool classes are a count. Representations are labelmap, contour, and surface |

Environment: macOS, zsh, rustc 1.97.1 (`8bab26f4f`, 2026-07-14), date context 2026-11-02. No patient data was used. dwv, Horos, and the medical viewer Grok were not opened. The three test commands were sequential. No other cargo process was running.

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

`segment_at` calls `pixel_count` before `offset`. A zero side, a product that does not fit, and a length other than `width * height` are `Degenerate`. An outside column or row is `Degenerate` from `x >= grid.width || y >= grid.height`, and `indices.get` is a second refusal rather than a background read. An in-range sample returns the stored `u16`, including 0. The 4 by 3 reads match the plan.

`label_segment` returns `Background` for segment 0 and `Blank` for trim-empty text before it writes. `" liver "` is stored as `liver`. An existing segment is updated through `iter_mut` and returned, so its place does not change. A new segment is appended. The temporary case showed that a segment 0 refusal on a non-empty list leaves the stored label, and that a later label still appends.

`segment_bidirectional` refuses segment 0 before it reads the grid. An absent segment is `Degenerate` because neither scan finds a run. Length is the sample count. Ends are inclusive. One sample is length 1 on both axes at that sample. Index 5 of a 3 by 3 grid is `(2, 1)`. A gap does not join two samples. Equal length does not replace, because both sites use `>` rather than `>=`. Rows are scanned from the top and columns from the left. The grid argument is a shared reference. The function does not write it.

The new files use `alloc::string::String` and `alloc::vec::Vec`. There is no `as` cast, no `unsafe`, no `unwrap`, no `expect`, no `sqrt`, no glam, and no `wasm-bindgen`. The crate is `no_std` outside tests. The integration-test build compiles the library without `cfg(test)`, and that build succeeded. The tests use `unreachable!` and `assert`. They do not use `unwrap` or `expect`.

`lib.rs` adds `mod segment_tools`, the public export of the three functions and four types, and one crate-doc sentence. That sentence matches the three operations. `stamp_brush` and `present_labelmap` are not called. `ocelli-geom` `bidirectional` is not called, and `measure.rs` has no diff.

The changelog bullet is directly above the F-080 bullet. It matches the code. A read returns one stored number. A label is trimmed and is only for a segment above 0. The reported runs are the longest horizontal run and the longest vertical run. An equal length keeps the earlier run. Segment 0 is not stored as a label and is not returned as a run. A short buffer is `Degenerate`, not background. The brush file and the presenter file are unchanged.

The public-api paragraph matches the same code. An in-range read includes 0. An outside column, a zero side, and a short buffer are `Degenerate`. The zero-side half of that sentence was executed by the temporary test, not only read. A label stores trimmed text and keeps an existing segment in place. Segment 0 is the label error `Background`. Blank text is `Blank`. The longest run wins, and an equal length keeps the first run found. One sample has length 1 on both axes. `bidirectional` in `ocelli-geom` is unchanged. There is no TypeScript copy. The F-ID header and the `public-api.md` row in `docs/lld/README.md` both add F-084. The README phrase is segment select, label, and bidirectional runs. The new sentences have no em-dash and no prose semicolon. The two semicolons in `public-api.md` are the pre-existing array lengths in code fences.

Tiers A, B, and C are not applicable. These functions do not open a device, do not submit a queue, and do not move pixels across the wasm boundary.

## Limitations and handoff

This pass is not feature completion and it is not a sprint review. It did not run clippy, `bin/ocelli.sh wasm`, `bin/ocelli.sh gate`, the oracle, the corpus, the benchmark harness, or the other `ocelli-seg` tests. No adapter was opened. A no-adapter result was not required, because the plan marks every tier not applicable.

The permanent suite does not retain the temporary cases. Those are a segment 0 refusal on a non-empty list, a gapped row, a long buffer, a zero width, and an outside row. This pass executed them on the restored source and then deleted the test. The product hashes match the hashes taken after the `>` restore. Do not treat the deleted test as coverage that remains in the tree.

The index does not contain this working tree. `git write-tree` still names the F-080 commit tree. Do not attach this result to a later staged tree or to a feature-profile receipt. The review file is new and is not part of the reviewed product diff. No commit was made and nothing was pushed.

F-084 pass 1: D=0 S=0 N=0
