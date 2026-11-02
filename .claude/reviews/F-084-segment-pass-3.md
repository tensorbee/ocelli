# F-084 review, pass 3

**Reviewed**: branch `sprint/s30`, HEAD `91b7af1b4c9ae9698a54fb71591dd2b1e20feb81` (`F-080, circle and sphere brush`). The index tree is `dd592d802409b4358acf68df9a21ccde6705b29c`, the same tree as `HEAD^{tree}`, because the F-084 edits are unstaged and the two new Rust files are untracked. This pass reviews the new ledger text only. That text is the appended `## F-084` section in `docs/sprints/AS_BUILT.md`, the new F-084 row in `docs/sprints/SPRINT_TRACKER.md`, the F-084 status cell in `docs/sprints/BACKLOG.md`, and the F-084 status cell in `docs/sprints/CURRENT_SPRINT.md`. `crates/ocelli-seg/src/segment_tools.rs` SHA-256 is `f087933c6e2e0216d8f26d4e33909682d812fbaf8374c776994cc844b145898d`. `crates/ocelli-seg/tests/segment_tools.rs` SHA-256 is `5010c4aca40e8ade0f4412abe00623ba9cbb8addde8bce483b1795bf02af9b99`.
**Author / reviewer**: the earlier reports name the implementation author as grok. This reviewer did not write the segment tools, the tests, the ledgers, or either earlier report. This pass did not resume reviewer `01a19ce1-769c-7be2-8cb7-54f1da121f02` or reviewer `01a19ced-603a-70b0-b242-e3b8343f19b6`. There is no F-084 commit, so this pass does not take an author from a trailer. This is not a self-review and it is not a sprint review.
**Result**: 0 defects, 0 smells, 0 nitpicks

## Acceptance challenged

The new claims are the ledger sentences. The segment-tool behaviour they restate was already reviewed on the same product bytes. This pass checks that those sentences match the files.

`crates/ocelli-seg/tests/segment_tools.rs` has thirteen `#[test]` functions. Four other `fn` items are helpers and are not tests. The first nine tests are AC1 through AC9 in `.claude/plans/F-084-design.md`, in that order. The next four are the locks named in the AS_BUILT sentence. That sentence says four more. Its comma list is four items. "a zero side together with an outside row" is one item and one test, `a_zero_side_and_an_outside_row_are_degenerate`. The last "and" joins "a long buffer" to that combined item. That is four locks for four tests, not five names for four tests.

The verification sentence is the completion formula. `.claude/commands/verify.md` defines the feature profile as `bin/ocelli.sh gate --floor` plus the corpus gate. The sentence names that set. It does not name a tree, a date, or a gate count. `.claude/verify-ledger.json` has no F-084 record and no `segment_tools` record. `python3 scripts/verify_ledger.py assert` exits 0 on the index, and that index is still the F-080 commit tree. The recorded profile on `dd592d802409b4358acf68df9a21ccde6705b29c` is `feature` with `corpus=pass`. That receipt is not an F-084 receipt. The sentence becomes the commit's ledger claim when that profile is recorded and the commit is made. This pass accepts that timing. It is not a false claim on this uncommitted text. The tracker clause "staged feature verification" is the same house formula in the days-actual cell. It is not a receipt on this tree, and it is not charged as one.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `shasum -a 256` of `src/segment_tools.rs` and `tests/segment_tools.rs` | Untracked product files on this working tree | Executed now | `f087933c6e2e0216d8f26d4e33909682d812fbaf8374c776994cc844b145898d` and `5010c4aca40e8ade0f4412abe00623ba9cbb8addde8bce483b1795bf02af9b99` |
| Count of `#[test]` and of other `fn` in `tests/segment_tools.rs` | Same test file | Executed now | 13 tests. 4 helpers. First 9 match AC1 to AC9. Next 4 match the lock list |
| `git diff` of `AS_BUILT.md`, `SPRINT_TRACKER.md`, `BACKLOG.md`, `CURRENT_SPRINT.md` | Working tree against HEAD `91b7af1b4c9ae9698a54fb71591dd2b1e20feb81` | Executed now | AS_BUILT appends only the F-084 section, 16 insertions and 0 deletions. Tracker adds only the F-084 row. Backlog and current sprint change only the F-084 status word from pending to done |
| `git diff` of `docs/hld/DEVIATIONS.md` | Same HEAD | Executed now | Empty. No deviation row was added |
| `git show` context at the end of the F-080 section | Design is already committed. The F-080 feature commit is HEAD | Executed now | The F-080 notes line is unchanged context above the new F-084 section |
| `python3 scripts/verify_ledger.py assert` and a read of `.claude/verify-ledger.json` | Index tree `dd592d802409b4358acf68df9a21ccde6705b29c`, which is `HEAD^{tree}` | Executed now | Assert exits 0. The message is `OK: tree dd592d802409 verified, corpus=pass`. The record profile is `feature`. The ledger text does not mention F-084 or `segment_tools` |
| Pass 1 and pass 2 report headers | `.claude/reviews/F-084-segment-pass-1.md` and `F-084-segment-pass-2.md` | Read now. Mutations not repeated | Both Result lines are 0 defects, 0 smells, 0 nitpicks. Pass 1 changed horizontal `>` to `>=` and says the comparison was restored. Pass 2 removed the segment 0 return in `label_segment` and says that return is present again |
| `length > current.length` and the segment 0 return | Restored `src/segment_tools.rs` | Read now. Not mutated | Lines 192 and 233 are `length > current.length`. `label_segment` still returns `Background` for segment 0 at lines 81 and 82 |

Environment: macOS, zsh, date context 2026-11-02. No cargo process ran. No mutation ran. No patient data was used. dwv, Horos, and the medical viewer Grok were not opened.

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

The thirteen tests, in order, are `an_in_range_read_returns_the_segment_and_an_outside_read_does_not`, `a_short_buffer_is_not_read_as_background`, `a_background_segment_and_a_blank_label_are_refused`, `replacing_a_label_keeps_its_place`, `the_longest_row_and_column_are_the_bidirectional_pair`, `an_equal_horizontal_run_keeps_the_upper_one`, `an_equal_vertical_run_keeps_the_left_one`, `a_missing_segment_and_background_are_not_runs`, and `one_sample_is_a_run_of_length_one_on_both_axes`, then `a_refused_label_leaves_an_existing_one`, `a_gap_keeps_the_first_sample`, `a_long_buffer_is_not_read_as_a_segment`, and `a_zero_side_and_an_outside_row_are_degenerate`. The first nine are the plan rows AC1 through AC9. The second four are the locks the AS_BUILT sentence names. The last of those four asserts a width of 0 and row 3 on the 4 by 3 grid in one function. The helpers `grid`, `sample_grid`, `must_at`, and `must_runs` are not tests.

The recomputed library hash matches the AS_BUILT note and both earlier pins. The recomputed test hash matches the pass 2 pin. The note names `segment_tools.rs` and records the library hash, which is the same shape as the F-080 `brush.rs` pin. It does not claim the test-file hash.

The backlog status cell and the current-sprint status cell for F-084 are `done`. F-080 stays `done` in both files. The backlog diff does not touch F-080, F-081, F-082, F-083, or F-085. Those four S31 rows stay `pending`. The current-sprint diff does not touch the F-080 row. The F-080 AS_BUILT section has no deletion and no rewritten line.

The tracker days-actual cell starts with `not measured`. The estimate cell stays `3w`. No duration was invented. The same cell says two review passes at 0 defects and 0 smells, a tie-direction mutation, and a background-refusal mutation. Pass 1 is the horizontal `>` to `>=` edit and says the source was restored. Pass 2 is the removed segment 0 return in `label_segment` and says the source was restored. The current library file still has both `length > current.length` sites and the `label_segment` background return.

The Result line in each earlier report is 0 defects, 0 smells, and 0 nitpicks. Pass 2 names `01a19ce1-769c-7be2-8cb7-54f1da121f02` as the pass 1 reviewer it did not resume. That is the pass 1 id in the AS_BUILT note. The pass 2 id in the note is `01a19ced-603a-70b0-b242-e3b8343f19b6`, which is the id this assignment names for that report. That second string is not written inside either review file. Neither report names a different reviewer for its own pass. The committed F-080 note has the same shape. The result counts are what the headers say.

The verification sentence and the corpus sentence that points at the commit trailer are the completion formula used by the unchanged F-080 entry. This pass accepts them as that formula. They do not claim that a receipt already exists for this unstaged tree. The assert success is the F-080 feature record on the index. The separate corpus clause, that this story changes no codec path and no CPU candidate bytes, matches the status file list. The changed paths are the changelog, `ocelli-seg` `lib.rs`, the two LLD files, the four sprint files, and the two new segment-tool files.

`docs/hld/DEVIATIONS.md` is not in the diff. The F-084 section says deviations none, and deviations from the design plan none. No deviation row was asked for. The new ledger sentences have no em-dash and no prose semicolon.

## Limitations and handoff

This pass is not feature completion and it is not a sprint review. It covers the new F-084 ledger claims. It does not re-review the segment-tool implementation, and it does not repeat either mutation.

No test command ran. No feature profile ran. The verify ledger has no F-084 receipt. The index does not contain the F-084 edits, so the assert success is about the F-080 commit tree. The completion formula is accepted here and becomes a receipt only after a green profile is recorded on the staged feature tree and the commit is made. Do not attach this result to a later tree.

The review file is untracked.

F-084 pass 3: D=0 S=0 N=0
