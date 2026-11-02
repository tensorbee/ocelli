# F-084 review, pass 2

**Reviewed**: branch `sprint/s30`, HEAD `91b7af1b4c9ae9698a54fb71591dd2b1e20feb81` (`F-080, circle and sphere brush`). The index tree is `dd592d802409b4358acf68df9a21ccde6705b29c`, the same tree as `HEAD^{tree}`, because the F-084 edits are unstaged and the two new Rust files are untracked. This pass reviews the working tree. Unstaged: `CHANGELOG.md`, `crates/ocelli-seg/src/lib.rs`, `docs/lld/README.md`, `docs/lld/public-api.md`. Untracked product files: `crates/ocelli-seg/src/segment_tools.rs`, `crates/ocelli-seg/tests/segment_tools.rs`. The pass 1 report is untracked and was not edited. The approved contract `.claude/plans/F-084-design.md` is already committed and is not in the diff.
**Author / reviewer**: the assigned author is grok. This reviewer did not write the segment tools, the fixtures, the changelog bullet, or the LLD sentences. This pass did not resume reviewer `01a19ce1-769c-7be2-8cb7-54f1da121f02` or any earlier reviewer, including the pass 1 reviewer. There is no F-084 commit, so this pass does not take an author from a trailer. This is not a self-review and it is not a sprint review.
**Result**: 0 defects, 0 smells, 0 nitpicks

## Acceptance challenged

The contract is the approved plan. Appendix B only counts tool classes and names the three representations. It does not define select, label, or a bidirectional run. No deviation was filed. `ocelli-geom` `bidirectional` was not used as a formula. A missing HLD formula is not a defect. The plan is the contract.

Pass 1 reviewed the nine permanent tests and then deleted a temporary tenth test. That tenth test is now four permanent tests. The library file was not part of that addition. Its SHA-256 is still `f087933c6e2e0216d8f26d4e33909682d812fbaf8374c776994cc844b145898d`. The test file is now `5010c4aca40e8ade0f4412abe00623ba9cbb8addde8bce483b1795bf02af9b99`.

The four new expectations were derived from the plan, not from a passing run.

`a_refused_label_leaves_an_existing_one`. Segment 3 with text `liver` is one entry. A blank on that same segment is `Blank` and must not replace the text. Segment 0 with text `nope` is `Background` and must not insert a row. The list is still one entry whose text is `liver`. Segment 4 with text `spleen` then appends, so the order is segment 3 then segment 4. That rejects a refusal that clears the list, overwrites the stored text, inserts segment 0, or blocks the next accepted label. The blank check sits before the in-place write. Deleting it would store an empty string and return `Ok`, so the `Blank` match would fail. This pass did not delete that check. It deleted only the segment 0 return in `label_segment`. The new test failed at the `Background` match. The older empty-list test failed at its own `Background` match. The other eleven tests stayed green, including the three new tests that do not call `label_segment`.

`a_gap_keeps_the_first_sample`. The row is `1 0 1` on a 3 by 1 grid. The first horizontal run is column 0 through column 0, length 1. The later sample is another run of length 1. Equal length keeps the earlier run, so the horizontal start is 0 and the end is 0. The vertical runs are the two columns that hold segment 1. Equal length keeps the left column, start 0 and length 1. A scan that did not stop on the 0 would report end 2 and length 3. Replacing on `>=` in the horizontal site would report start 2. Replacing on `>=` in the vertical site would report vertical start 2. Each of those results fails an assertion in this test. This pass did not edit either comparison. Both sites still read `length > current.length`.

`a_long_buffer_is_not_read_as_a_segment`. A 2 by 2 grid with five indices, the last of them 9, has length 5 against `width * height` of 4. `pixel_count` returns `Degenerate` before `segment_at` reads index 0 and before either scan. Both public calls are `Degenerate`, and the five indices stay, including the 9. A length check that rejected only a short buffer would accept 5, `segment_at(0, 0)` would return 1, and `segment_bidirectional` for segment 1 would return runs. Both `Degenerate` assertions would fail. The short-buffer test remains the other half. Together they lock inequality rather than a one-sided length check.

`a_zero_side_and_an_outside_row_are_degenerate`. Width 0 and height 1 with an empty buffer, read at `(0, 0)`, is `Degenerate`. Row 3 on the 4 by 3 sample grid, read at `(0, 3)`, is `Degenerate`. The wrong plan result is a successful read, including 0. Either `Ok` fails the match. This pass did not delete those two comparisons. The trace is as follows. Deleting `width == 0 || height == 0` does not by itself make this width 0 call return `Ok`. `offset` still treats `0 >= width` as outside. If that bound were absent too, `get` on the empty buffer would still be `Degenerate`. Deleting `y >= grid.height` does not by itself make `(0, 3)` return `Ok`. The index is 12 on a buffer of length 12, and `get` maps a miss to `Degenerate`. When `pixel_count` has already required `width * height` samples, an outside row is past that length if the product fits, and checked arithmetic refuses if it does not. The outside column in the older test is the case that can read a stored 0: `(4, 0)` is index 4, and that sample is 0. These two new assertions stay green if only their early comparisons are deleted, because the Result does not change. They do not stay green if the call returns `Ok`. The plan requires the Result. This overlap is not a defect.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `bin/ocelli.sh test ocelli-seg --test segment_tools` | Working tree before the mutation. Library SHA-256 `f087933c6e2e0216d8f26d4e33909682d812fbaf8374c776994cc844b145898d`. Test SHA-256 `5010c4aca40e8ade0f4412abe00623ba9cbb8addde8bce483b1795bf02af9b99`. `brush.rs` SHA-256 `a8756735471dd7787d2538daf1e61e12529576ab0425571984be0a76d27f96dc` | Executed now | `code=0`. 13 passed, 0 failed, 0 ignored |
| Same command after deleting only the `segment == 0` return in `label_segment` | Recompiled `ocelli-seg` in 0.77s. That return was the only edit | Executed now | `code=101`. 11 passed, 2 failed. `a_refused_label_leaves_an_existing_one` panicked at `tests/segment_tools.rs:187`, the `Background` match on segment 0 text `nope`. `a_background_segment_and_a_blank_label_are_refused` panicked at line 59, the empty-list `Background` match. The length and text assertions below line 187 were not reached |
| Restore of that return, then `shasum -a 256` | Same three paths | Executed now | The `label_segment` segment 0 return is present again. Library hash returned to `f087933c6e2e0216d8f26d4e33909682d812fbaf8374c776994cc844b145898d`. Test hash unchanged. `brush.rs` unchanged |
| `git diff --stat` and `git diff` of the forbidden paths | HEAD `91b7af1b4c9ae9698a54fb71591dd2b1e20feb81` | Executed now | Four tracked files, 12 insertions and 3 deletions. `labelmap.rs`, `contour.rs`, `surface.rs`, `measure.rs`, `annotation.ts`, and `brush.rs` have an empty diff |
| `rg` for `length > current.length` and `length >= current.length` | Restored `segment_tools.rs` | Executed now | Line 192 and line 233 are `length > current.length`. No `>=` replacement. The only `>=` in the file is the outside check in `offset` |
| `rg` for `stamp_brush`, `present_labelmap`, `ocelli_geom`, `glam`, `wasm_bindgen`, `unsafe`, `unwrap`, and `expect` in the two Rust files | Restored tree | Executed now | No call and no import. The only `as` hits are the words in the two file comments |
| `rg` for the three new function names under `*.ts`, `*.tsx`, and `*.js` | Working tree | Executed now | No hit |

Environment: macOS, zsh, rustc 1.97.1 (`8bab26f4f`, 2026-07-14), date context 2026-11-02. No patient data was used. dwv, Horos, and the medical viewer Grok were not opened. The two test commands were sequential. No other cargo process was running. The guard was restored before this report was written. Leaving it out would have been a defect introduced by this pass.

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

`segment_at` calls `pixel_count` before `offset`. A zero side, a product that does not fit, and a length other than `width * height` are `Degenerate`. An outside column or row is `Degenerate` from `x >= grid.width || y >= grid.height`, and `indices.get` is a second refusal rather than a background read. An in-range sample returns the stored `u16`, including 0. The 4 by 3 reads match the plan. The new outside-row call is `(0, 3)` on that grid. The new zero-side call is width 0. Both are `Degenerate` on the restored source, which the green run executed.

`label_segment` returns `Background` for segment 0 and `Blank` for trim-empty text before it writes. `" liver "` is stored as `liver`. An existing segment is updated through `iter_mut` and returned, so its place does not change. A new segment is appended. The green run of `a_refused_label_leaves_an_existing_one` shows that a blank and a segment 0 call leave segment 3 `liver` in place, and that segment 4 `spleen` then appends. The red run shows that the segment 0 return is why the second refusal is `Background` rather than an inserted row.

`segment_bidirectional` refuses segment 0 before it reads the grid. An absent segment is `Degenerate` because neither scan finds a run. Length is the sample count. Ends are inclusive. One sample is length 1 on both axes at that sample. A gap does not join two samples. The new 3 by 1 row keeps the first sample and the left column. Equal length does not replace, because both sites use `>` rather than `>=`. Rows are scanned from the top and columns from the left. The grid argument is a shared reference. The function does not write it. The long-buffer test shows that five indices on a 2 by 2 grid are `Degenerate` from both entry points and that the 9 stays in the buffer.

The new files use `alloc::string::String` and `alloc::vec::Vec`. There is no `as` cast, no `unsafe`, no `unwrap`, no `expect`, no `sqrt`, no glam, and no `wasm-bindgen`. The crate is `no_std` outside tests. The integration-test build compiles the library without `cfg(test)`, and that build succeeded. The tests use `unreachable!` and `assert`. They do not use `unwrap` or `expect`.

`lib.rs` adds `mod segment_tools`, the public export of the three functions and four types, and one crate-doc sentence. That sentence matches the three operations. `stamp_brush` and `present_labelmap` are not called. `ocelli-geom` `bidirectional` is not called, and `measure.rs` has no diff.

The changelog bullet is directly above the F-080 bullet. It matches the code. A read returns one stored number. A label is trimmed and is only for a segment above 0. The reported runs are the longest horizontal run and the longest vertical run. An equal length keeps the earlier run, which is the two `>` sites. Segment 0 is not stored as a label and is not returned as a run. A short buffer is `Degenerate`, not background. A long buffer is also `Degenerate`. The bullet does not claim otherwise. The brush file hash is the pin above, and the presenter file has an empty diff.

The public-api paragraph matches the same code. An in-range read includes 0. An outside column, a zero side, and a short buffer are `Degenerate`. The zero-side sentence is the Result, which the new test executes. A label stores trimmed text and keeps an existing segment in place. Segment 0 is the label error `Background`. Blank text is `Blank`. The longest run wins, and an equal length keeps the first run found. One sample has length 1 on both axes. `bidirectional` in `ocelli-geom` is unchanged. There is no TypeScript copy. The F-ID header and the `public-api.md` row in `docs/lld/README.md` both add F-084. The README phrase is segment select, label, and bidirectional runs. The new sentences have no em-dash and no prose semicolon. The two semicolons in `public-api.md` are the pre-existing array lengths in code fences.

Tiers A, B, and C are not applicable. These functions do not open a device, do not submit a queue, and do not move pixels across the wasm boundary.

## Limitations and handoff

This pass is not feature completion and it is not a sprint review. It did not run clippy, `bin/ocelli.sh wasm`, `bin/ocelli.sh gate`, the oracle, the corpus, the benchmark harness, or the other `ocelli-seg` tests. No adapter was opened. A no-adapter result was not required, because the plan marks every tier not applicable.

The pass 1 mutation of horizontal `>` to `>=` was not repeated. The library hash is the pin taken after that restore, and both sites still read `length > current.length`. The blank return inside `label_segment` was not deleted. The gap comparison, the length check, the zero-side return, and the row comparison were not edited. Those conclusions are from reading the restored source against the assertions. Only the segment 0 return in `label_segment` was deleted, and only for the red run recorded above.

Height 0 was not given its own call. It shares the zero-side check with the width 0 case that the green run executed. A zero side with a non-empty buffer was not given its own call. The length check already refuses that buffer because `width * height` is 0.

The index does not contain this working tree. `git write-tree` still names the F-080 commit tree. Do not attach this result to a later staged tree or to a feature-profile receipt. The review file is new and is not part of the reviewed product diff. No commit was made and nothing was pushed.

F-084 pass 2: D=0 S=0 N=0
