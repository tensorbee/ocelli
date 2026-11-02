# F-080 review, pass 3

**Reviewed**: branch `sprint/s30`, HEAD `ba6f3957770ddefaa029b9d82ef9d3fbec87ab8c`. The index tree is `3ee487bfa4975a20f1a99c8373e3921f2643a8be`, the same tree as HEAD, because the F-080 edits are unstaged. This pass reviews the new ledger text only. That text is the appended `## F-080` section in `docs/sprints/AS_BUILT.md`, the new F-080 row in `docs/sprints/SPRINT_TRACKER.md`, the F-080 status cell in `docs/sprints/BACKLOG.md`, and the F-080 status cell in `docs/sprints/CURRENT_SPRINT.md`. `crates/ocelli-seg/src/brush.rs` SHA-256 is `a8756735471dd7787d2538daf1e61e12529576ab0425571984be0a76d27f96dc`. `crates/ocelli-seg/tests/brush.rs` SHA-256 is `b95e96c182086722db3e229453d7c47bfac9a6a7251f1165fce8f2b5a7e33a3e`.
**Author / reviewer**: the earlier reports name the implementation author as grok. This reviewer did not write the brush, the tests, the ledgers, or either earlier report. This pass did not resume reviewer `01a19cb6-214d-7b10-89ff-19537da639e5` or reviewer `01a19cc7-d0c1-78d1-8392-feb38c898891`. There is no F-080 commit, so this pass does not take an author from a trailer. This is not a self-review and it is not a sprint review.
**Result**: 0 defects, 0 smells, 0 nitpicks

## Acceptance challenged

The new claims are the ledger sentences. The brush behaviour they restate was already reviewed on the same product bytes. This pass checks that those sentences match the files.

`crates/ocelli-seg/tests/brush.rs` has eighteen `#[test]` functions. Four other `fn` items are helpers and are not tests. The first nine tests are AC1 through AC9 in `.claude/plans/F-080-design.md`. The next nine are the locks named in the AS_BUILT sentence. That sentence says nine more. Its comma list is nine items. "an empty stack and a zero side" is one item and one test. The last "and" joins "a halfway sphere" to "a radius just under one", and those are two tests. That is nine locks for nine tests, not ten names for nine tests.

The verification sentence is the completion formula. `.claude/commands/verify.md` defines the feature profile as `bin/ocelli.sh gate --floor` plus the corpus gate. The sentence names that set. It does not name a tree, a date, or a gate count. `.claude/verify-ledger.json` has no F-080 record and no brush record. `python3 scripts/verify_ledger.py assert` exits 1 on the index, and that index is still the design-commit tree. The sentence becomes the commit's ledger claim when that profile is recorded and the commit is made. This pass accepts that timing. It is not a false claim on this uncommitted text. The tracker clause "staged feature verification" is the same house formula in the days-actual cell. It is not a receipt on this tree, and it is not charged as one.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `shasum -a 256` of `brush.rs` and `tests/brush.rs` | Untracked product files on this working tree | Executed now | `a8756735471dd7787d2538daf1e61e12529576ab0425571984be0a76d27f96dc` and `b95e96c182086722db3e229453d7c47bfac9a6a7251f1165fce8f2b5a7e33a3e` |
| Count of `#[test]` and of other `fn` in `tests/brush.rs` | Same test file | Executed now | 18 tests. 4 helpers. First 9 match AC1 to AC9. Next 9 match the lock list |
| `git diff` of `AS_BUILT.md`, `SPRINT_TRACKER.md`, `BACKLOG.md`, `CURRENT_SPRINT.md` | Working tree against HEAD `ba6f3957770ddefaa029b9d82ef9d3fbec87ab8c` | Executed now | AS_BUILT appends only the F-080 section. Tracker adds only the F-080 row. Backlog and current sprint change only the F-080 status word from pending to done |
| `git diff` of `labelmap.rs`, `contour.rs`, `surface.rs`, and `docs/hld/DEVIATIONS.md` | Same HEAD | Executed now | Empty. No deviation row was added |
| `git show HEAD` tails of `AS_BUILT.md` and `SPRINT_TRACKER.md` | Design commit | Executed now | F-079 notes and the F-079 tracker row match the working-tree lines above the new F-080 text |
| `rg` for `stamp_brush` and `BrushShape` under `packages` | Working tree | Executed now | No hit |
| `python3 scripts/verify_ledger.py assert` and a read of `.claude/verify-ledger.json` | Index tree `3ee487bfa4975a20f1a99c8373e3921f2643a8be`, which is `HEAD^{tree}` | Executed now | Assert exits 1. The ledger text does not mention F-080 or brush |
| Pass 1 and pass 2 report headers | `.claude/reviews/F-080-brush-pass-1.md` and `F-080-brush-pass-2.md` | Read now. Mutations not repeated | Both Result lines are 0 defects, 0 smells, 0 nitpicks. Pass 1 changed `is_le` to `is_lt` and says the compare was restored. Pass 2 removed the negative-zero bit clause and says the source was restored |
| Appendix B and the crate table | `docs/hld/B-parity-surface.md` lines 25 and 29, `docs/hld/03-architecture-and-crates.md` line 40 | Read now | Tool classes are a count. Representations are labelmap, contour, and surface. `ocelli-seg` is named. Neither file defines a brush or a stamp |

Environment: macOS, zsh, date context 2026-11-02. No cargo process ran. No mutation ran. No patient data was used. dwv, Horos, and the medical viewer Grok were not opened.

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

The eighteen tests, in order, are `a_circle_of_radius_one_paints_the_plus_and_not_the_corners`, `a_sphere_reaches_the_next_slice_only_at_the_centre`, `a_negative_radius_and_a_non_finite_radius_write_nothing`, `a_short_buffer_is_not_stamped`, `radius_zero_paints_only_an_integer_centre`, `a_circle_paints_only_the_matching_slice`, `erasing_the_centre_then_stamping_it_again_changes_once`, `mismatched_slices_are_not_stamped`, and `the_presenter_colours_the_stamped_segment`, then `a_negative_zero_radius_erases_only_the_matching_slice`, `a_fractional_circle_slice_paints_nothing`, `an_empty_stack_and_a_zero_side_write_nothing`, `a_non_finite_centre_writes_nothing_and_a_later_stamp_works`, `a_long_buffer_is_not_stamped`, `radius_zero_follows_column_then_row`, `an_already_painted_outside_corner_is_not_a_change`, `a_sphere_halfway_between_slices_paints_both_centres`, and `a_radius_just_under_one_misses_the_edges`. The first nine are the plan rows AC1 through AC9. The second nine are the locks the AS_BUILT sentence names. The helpers `grid`, `brush`, `zeros`, and `must_stamp` are not tests.

`brush.rs` still compares squared distance with `total_cmp` and `is_le` at line 154. `strictly_negative` still excludes `0x8000_0000_0000_0000` at line 158. `same_number` still treats both zero bit patterns as one slice, so a circle at either zero selects slice 0. A sphere does not take that skip. A non-finite centre or radius returns `Frame` before a write. A negative radius returns `Degenerate`. Segment 0 is a legal written value. The file does not name a device. The tier sentence that A, B, and C are n/a matches the plan and that file. The stamp writes segment numbers. AC9 calls the existing presenter. The fixture sentence that the stamp does no pixel arithmetic holds on that split.

`present_labelmap` lives in `labelmap.rs`. That file, `contour.rs`, and `surface.rs` have no diff. No TypeScript file is in the diff, and `stamp_brush` has no hit under `packages`. The crates sentence names `ocelli-seg`, `docs/lld/public-api.md`, and the public-api row in `docs/lld/README.md`. Those three are in the diff. The public-api contributor line and the README row both gained F-080.

Appendix B counts tool classes and names the three representations. It does not define a brush. The architecture table names `ocelli-seg` and does not give a stamp function. The squared-distance rule is the approved plan. `docs/hld/DEVIATIONS.md` is not in the diff. No deviation row was asked for.

The backlog status cell and the current-sprint status cell for F-080 are `done`. F-084 stays `pending` in both files. The current-sprint diff does not touch the F-084 row. The backlog diff does not touch it either.

The tracker days-actual cell starts with `not measured`. The estimate cell stays `4w`. No duration was invented. The same cell says two review passes at 0 defects and 0 smells, a radius-boundary mutation, and a negative-zero mutation. Pass 1 is the `is_le` to `is_lt` edit and says the source was restored. Pass 2 is the removed negative-zero bit clause and says the source was restored. The recomputed `brush.rs` hash matches the AS_BUILT note and both earlier pins. The recomputed test hash matches the pass 2 pin.

The Result line in each earlier report is 0 defects, 0 smells, and 0 nitpicks. The reviewer id strings in the AS_BUILT note are the ids this assignment names for those passes. Those strings are not written inside either review file. Neither report names a different reviewer. The committed F-079 note has the same shape. The result counts are what the headers say.

The verification sentence and the corpus sentence that points at the commit trailer are the completion formula used by the unchanged F-079 entry and by earlier entries. This pass accepts them as that formula. They do not claim that a receipt already exists for this unstaged tree. The separate corpus clause, that this story changes no codec path and no CPU candidate bytes, matches the diff file list. The F-079 AS_BUILT section and the F-079 tracker row are unchanged.

## Limitations and handoff

This pass is not feature completion and it is not a sprint review. It covers the new F-080 ledger claims. It does not re-review the brush implementation, and it does not repeat either mutation.

No test command ran. No feature profile ran. The verify ledger has no F-080 receipt. The index does not contain the F-080 edits, so the assert refusal is about the design-commit tree. The completion formula is accepted here and becomes a receipt only after a green profile is recorded on the staged feature tree and the commit is made. Do not attach this result to a later tree.

The review file is untracked.
