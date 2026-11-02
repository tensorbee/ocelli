# F-082 review, pass 2

**Reviewed**: branch `sprint/s31`, HEAD `e2fe31ec491261173ba6a7b0fca7550f50661825` (`F-081, rectangle circle and sphere scissors`). `git write-tree` is `677c6e45bacee7e7af3feb70c3dbef404a52b8d5`, the same tree as `HEAD^{tree}`, because the F-082 edits are unstaged and the two new Rust files are untracked. This pass reviews that working tree, not the index. Paths and SHA-256 after the mutation was restored and the probe was removed: `crates/ocelli-seg/src/fill.rs` `ba88ad6353797a694e735327d6c03f11819858089459328482eab1e85f4d98f4`, `crates/ocelli-seg/tests/fill.rs` `7ce9038ca1f38884fecb45ae034c31edfb7712bd4baf69dc3d55b89d13de8982`, `crates/ocelli-seg/src/lib.rs` `30b893dc367561e104ca936948865276afe1fbd9443f35ed6c4127bdaf643c83`, `CHANGELOG.md` `313c24279648a8bbfd272a7d8730d0340007fcdbff639661e6827c3eab200f55`, `docs/lld/README.md` `3f8e2f79af5908a0266e3dfcf758efdee60ff58318e49ea8e7bfc050f5b11ea8`, `docs/lld/public-api.md` `fc0c71515d094331e618a6737d20444d9232392dc0ec80226eb0c163f5299a59`. The public-api digest differs from pass 1. The other five product digests match pass 1.
**Author / reviewer**: the product diff has no commit, so this pass does not take an author from a trailer. This reviewer did not write the fill, the tests, the LLD rewrite, or pass 1. Reviewers `01a19d46-f6d6-7080-a3cd-fbdb19b10e6d`, `01a19d2a-4ab3-74f3-93b5-f282472ae39a`, and `01a19d37-53e6-7ac0-8aef-0f5953c4195b` were not resumed. This is not a self-review and it is not a sprint review.
**Result**: 0 defects, 0 smells, 0 nitpicks

## Acceptance challenged

The contract is `.claude/plans/F-082-design.md`. `docs/hld/B-parity-surface.md` lines 25 and 29 match the plan quotation. They count tool classes and the three representations. They do not define a fill. A search of `docs/hld/` for a flood fill or a 4-connected rule found neither. The 4-connected rule is the plan.

Pass 1 D1 said `docs/lld/public-api.md` described paint as replacing every sample on the grid. That sentence was rewritten. This pass treats the finding as closed only where the new sentence matches `paint_fill` and the row `1 0 1`.

Line 209 now says a paint replaces each sample whose stored value equals the seed, the written number is the caller segment, and a different value stays. `prepare` reads the seed. The paint loop skips a sample that is not that value and writes the caller segment. On `[1, 0, 1]`, paint from x 0 writing 2 leaves the middle 0 and writes 2 into both 1s. Covered is 2 and changed is 2. A full-grid write would be `[2, 2, 2]`. Writing the seed value would leave `[1, 0, 1]`. A flood would leave `[2, 0, 1]`. The permanent test expects `[2, 0, 2]` and passed before the mutation.

The adjacent probe, removed after the run, painted that row from x 2 and got `[9, 0, 9]` with covered 2 and changed 2. A second paint changed 0 and left the 0 in place. Paint from the middle 0, writing 7, got `[1, 7, 1]`. Paint of the AC2 diagonal grid changed all five 1s and left the 0s. An outside flood left `[1, 0, 1]` unchanged, and the following paint wrote `[4, 0, 4]`. A 2 by 2 with a fifth index 9 refused both functions and kept the 9. Those results reject a full-grid replacement, a paint that writes the seed value, and a paint that stops at a gap or a diagonal.

AC1 through AC8 remain the eight permanent tests. The hand counts in the plan match those assertions. This pass does not reopen them as new failures.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `bin/ocelli.sh test ocelli-seg --test fill` | Product hashes above, before the mutation and before the probe | Executed now | Exit 0. 8 passed, 0 failed, 0 ignored |
| Right step `checked_add(1)` changed to `checked_add(2)`, then restored | `crates/ocelli-seg/src/fill.rs` line 159. Up, down, and left steps left at 1 | Executed now | Same test command, exit 101. 6 passed and 2 failed. Gap covered left 2 and right 1 at `tests/fill.rs:60`. Snake covered left 1 and right 5 at `tests/fill.rs:142` |
| SHA-256 of the six product paths after restore | Same paths as the Reviewed line | Executed now | Each digest matches. `review_probe_adjacent` is absent. The right step is `checked_add(1)`. No `(1, 1)` neighbour step |
| Temporary `review_probe_adjacent`, then removed | Added only in `tests/fill.rs`, then deleted. Restored hash `7ce9038ca1f38884fecb45ae034c31edfb7712bd4baf69dc3d55b89d13de8982`. `fill.rs` stayed on its digest during the probe | Executed now | `bin/ocelli.sh test ocelli-seg --test fill review_probe_adjacent` exit 0. 1 passed. 8 filtered out |
| Python compare of the F-ID list on `docs/lld/public-api.md` line 3 and the F-ID cell of `docs/lld/README.md` line 38 | Working tree | Executed now | Both lists have the same 22 ids, in the same order, ending F-080, F-084, F-081, F-082. No duplicate |
| `git diff --name-only HEAD` for brush, scissors, labelmap, contour, surface, segment tools, and `DEVIATIONS.md` | Working tree against HEAD, after restore | Executed now | Empty |
| `git status --short` and `git diff --stat HEAD` | Unstaged F-082 paths. New Rust files untracked | Executed now | Four tracked files, 9 insertions and 3 deletions. `git write-tree` stayed `677c6e45bacee7e7af3feb70c3dbef404a52b8d5` |

Environment: macOS, zsh, workspace `/Users/atulsharma/Documents/projects/tensorbee/ocelli`, date context 2026-11-02. The three cargo test commands ran one after another. No wasm-pack ran. No patient data was used. dwv, Horos, and the medical viewer Grok were not opened.

## Defects

None. Pass 1 D1 is closed. The rewritten paint sentence matches `paint_fill` and the row fixture. It is not refiled.

## Smells

None.

## Nitpicks

None.

## Verified clean

`flood_fill` and `paint_fill` are exported with `FillCoverage` and `FillError` from `lib.rs` line 30. `FillError` has one variant, `Degenerate`, and no `Frame` variant. The seed column and row are `u32`. `covered` counts selected samples, including ones that already hold the written segment. `changed` counts the ones whose stored number differed. Both counts use `saturating_add`. AC5 covers 5 and changes 0. The removed probe's second paint covers 2 and changes 0.

`prepare` returns before any clone when a side is 0, the pixel count does not fit in `u32`, the length is not `width * height`, or the seed is outside. `flood_fill` assigns `grid.indices` only after `walk_flood` returns. A `?` from `prepare`, `mark_seed`, or `walk_flood` returns before that assignment. `paint_fill` clones only after `prepare` accepts the grid. The probe's long tail stayed 9.

The four flood steps are one column left or right and one row up or down, through `checked_add` and `checked_sub`. There is no diagonal step. `consider` does not mark a sample outside the grid, and it queues a sample only when the stored value equals the seed. The permanent diagonal test writes only the centre. The removed probe's paint of that same grid writes all five 1s.

`LabelGrid` is row-major. Width is the column count. The fill offset is `y * width + x`, the same formula as `labelmap.rs` lines 174 to 177. There is no `as` cast in `fill.rs`. The only use of the word "as" is in the module comment. There is no `unsafe`, no `wasm-bindgen`, and no glam. Reads and writes use `get` and `get_mut`. The crate stays `no_std` outside tests. `fill.rs` uses `alloc` for `VecDeque` and `Vec`. The fill does not call `stamp_brush` or `cut_scissor`. It does not open a device. Tiers A, B, and C stay not applicable, as the plan says, and there is no tier branch that could write a different result.

AC1 writes the plus `[0, 2, 0, 2, 2, 2, 0, 2, 0]` with covered 5 and changed 5. AC2 writes only the centre. AC3 flood writes `[2, 0, 1]` and paint writes `[2, 0, 2]`. AC4 refuses and keeps the stored ones. AC8 writes `[2, 2, 0, 0, 2, 0, 0, 2, 2]`. Those indices match a fresh hand count of the plan grids. PS3.3 C.8.20.2 is the background rule only. Segment 0 is a legal written value. AC6 and AC7 both passed on the unmutated tree.

AC7 paints the plus to 0 and asks the unchanged `present_labelmap` for fill on and outline off. The centre sample is index 4, so its bytes are 16 through 19. Segment 0 is transparent at `labelmap.rs` lines 130 to 136. `labelmap.rs` is not in the diff.

The changelog line says paint-fills every sample with that value, after naming the seed value. That noun phrase was not charged in pass 1. This pass does not open it. On the row, the samples that hold the seed value change, and the middle value stays. The written number is the caller segment, which the LLD sentence now states. The `lib.rs` lines say every copy of one value. The function docs state the diagonal rule and the four refusal triggers. Both F-ID lists include F-082. A search of `*.rs`, `*.ts`, `*.tsx`, `*.js`, `*.mjs`, and `*.md` outside hidden directories found `flood_fill` and `paint_fill` only in `fill.rs`, `lib.rs`, `tests/fill.rs`, and `docs/lld/public-api.md`. The new sentences have no em dash and no prose semicolon. The semicolon characters already in `public-api.md` are the pre-existing array lengths in code fences at lines 49 and 55.

The step mutation did not leave the suite green. The plus fixture stayed green for that one change because a width of 3 lets the left sample step onto the right sample. The gap fixture and the snake fixture did not stay green. The diagonal fixture stayed green on that mutation because that grid has no orthogonal neighbour of the seed. Its green result on the restored tree is the diagonal evidence, because a diagonal step would paint a corner.

## Limitations and handoff

This pass is not feature completion and it is not a sprint review. Clippy, the feature profile, wasm, and the corpus gate did not run. No GPU adapter was opened. A grid whose `width * height` exceeds `u32::MAX` was not allocated, so the `u32::try_from` refusal was read and not executed. The DICOM PDF was not opened. Other `ocelli-seg` tests were not run. The fill test does compile the library.

The right-hand paint, the duplicate paint, the diagonal paint, the refusal followed by a valid paint, and the long tail were a temporary probe. They are not permanent tests. The probe was removed. Do not treat this pass as coverage of those cases in the suite. The permanent AC4 test checks flood for an outside column and paint for a short buffer. Both functions call `prepare`.

AC7 checks a centre that is background after the paint. It does not reject a presenter that zeroes every pixel. The presenter file is unchanged, and this pass did not rerun the labelmap suite.

`docs/sprints/CURRENT_SPRINT.md` lines 106 to 108 are unchanged from HEAD and are not in this diff. Pass 1 read them and did not charge them. This pass does not reopen that choice. The code is judged against the approved plan.

The product bytes were restored to the hashes in the Reviewed line before this review file was added. This pass adds only `.claude/reviews/F-082-fill-pass-2.md`. Do not attach this result to a later tree.

F-082 pass 2: D=0 S=0 N=0
