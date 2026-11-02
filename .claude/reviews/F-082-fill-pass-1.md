# F-082 review, pass 1

**Reviewed**: branch `sprint/s31`, HEAD `e2fe31ec491261173ba6a7b0fca7550f50661825` (`F-081, rectangle circle and sphere scissors`). `git write-tree` is `677c6e45bacee7e7af3feb70c3dbef404a52b8d5`, the same tree as `HEAD^{tree}`, because the F-082 edits are unstaged and the two new Rust files are untracked. This pass reviews that working tree, not the index. Paths and SHA-256: `crates/ocelli-seg/src/fill.rs` `ba88ad6353797a694e735327d6c03f11819858089459328482eab1e85f4d98f4`, `crates/ocelli-seg/tests/fill.rs` `7ce9038ca1f38884fecb45ae034c31edfb7712bd4baf69dc3d55b89d13de8982`, `crates/ocelli-seg/src/lib.rs` `30b893dc367561e104ca936948865276afe1fbd9443f35ed6c4127bdaf643c83`, `CHANGELOG.md` `313c24279648a8bbfd272a7d8730d0340007fcdbff639661e6827c3eab200f55`, `docs/lld/README.md` `3f8e2f79af5908a0266e3dfcf758efdee60ff58318e49ea8e7bfc050f5b11ea8`, `docs/lld/public-api.md` `e52792a9436b222852f056054c08b7844338e994d925ac470bc957a2571b289d`.
**Author / reviewer**: the product diff has no commit, so this pass does not take an author from a trailer. This reviewer did not write the fill, the tests, or the LLD lines. Reviewers `01a19d2a-4ab3-74f3-93b5-f282472ae39a` and `01a19d37-53e6-7ac0-8aef-0f5953c4195b` were not resumed. This is not a self-review and it is not a sprint review.
**Result**: 1 defect, 0 smells, 0 nitpicks

## Acceptance challenged

The contract is `.claude/plans/F-082-design.md`. `docs/hld/B-parity-surface.md` lines 25 and 29 match the plan's quotation. They count tool classes and the three representations. They do not define a fill. `docs/hld/03-architecture-and-crates.md` names `ocelli-seg` and does not give a fill function. The 4-connected rule is the plan. It is not an HLD formula.

AC1 through AC8 are the eight permanent tests, in that order. The expectations below were computed from the plan, not copied from a test failure.

A diagonal line of three 5s must flood to one 9 at the seed and must paint all three 5s. An 8-connected flood would cover 3. A ring of twelve 1s around a hole, with a blank row and then four more 1s, must flood to covered 12 and must leave the hole and the bottom row. Paint from the same seed must cover 16 and must still leave the hole. A row `2 1 2` painted from the 1 to 9 must become `2 9 2` with covered 1. Replacing every sample would not leave the 2s. Two 4s with a gap, painted with segment 4, must cover 2 and change 0. A one-column run of three 1s, flooded from the bottom, must cover 3. A 3 by 2 block flooded from the last index must cover 6. A vertical wall of 7s must stop a flood at the left column and must not stop a paint of the 1s. A seed on row 2 of a 2 by 2 must be `Degenerate` with the 1s unchanged, and a later flood from `(1, 1)` must still write four 3s. Paint at x equal to the width of `4 5 6` must refuse. Flood of a 2 by 2 with three indices must refuse. A 2 by 2 with a fifth index 9 must refuse both functions and keep the 9. Width 0 and height 0 must refuse and keep a stale 8. Flood of the AC1 plus writing segment 1 must cover 5 and change 0. The unchanged presenter must then write the LUT colour at the top edge and at the centre, and transparent bytes at the corner.

The temporary probe passed those cases and was removed. The test file hash above is the restored file.

The right-hand step was then changed from 1 to 2. That is not the removed diagonal-neighbour edit. `a_gap_stops_a_flood_and_not_a_paint` went red at covered left 2 and right 1. `a_flood_follows_a_corner_and_leaves_the_off_snake_sample` went red at covered left 1 and right 5. The source was restored.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `bin/ocelli.sh test ocelli-seg --test fill` | Working tree hashes above, before the mutation and before the probe | Executed now | Exit 0. 8 passed, 0 failed, 0 ignored |
| Right step `checked_add(1)` changed to `checked_add(2)`, then restored | `crates/ocelli-seg/src/fill.rs` line 159. Up, down, and left steps left at 1 | Executed now | Same test command, exit 101. 6 passed and 2 failed. Gap covered left 2 right 1 at `tests/fill.rs:60`. Snake covered left 1 right 5 at `tests/fill.rs:142` |
| SHA-256 of the six reviewed paths after restore | Same paths as the Reviewed line | Executed now | Each digest matches the green tree. `review_probe_discriminators` is absent. The right step is `checked_add(1)` again |
| Temporary `review_probe_discriminators`, then removed | Added only in `tests/fill.rs`, then deleted. Restored hash `7ce9038ca1f38884fecb45ae034c31edfb7712bd4baf69dc3d55b89d13de8982` | Executed now | `bin/ocelli.sh test ocelli-seg --test fill review_probe_discriminators` exit 0. 1 passed. 8 filtered out |
| Python list of `F-` ids with a digit in `docs/lld/public-api.md` line 3 and the F-IDs cell of `docs/lld/README.md` line 38 | Working tree | Executed now | Both lists have the same 22 ids, in the same order, ending `F-080, F-084, F-081, F-082`. Both differences are empty. No duplicate |
| `git diff --stat HEAD` and `git status --short` | Unstaged F-082 paths. New Rust files untracked | Executed now | Four tracked files, 9 insertions and 3 deletions. Product untracked files are `fill.rs` and `tests/fill.rs` |
| `git diff --name-only HEAD` for brush, scissors, labelmap, contour, surface, segment tools, and `DEVIATIONS.md` | Working tree against HEAD | Executed now | Empty |

Environment: macOS, zsh, workspace `/Users/atulsharma/Documents/projects/tensorbee/ocelli`, date context 2026-11-02. The three cargo test commands ran one after another. No wasm-pack ran. No patient data was used. dwv, Horos, and the medical viewer Grok were not opened.

## Defects

### D1, The public-api paint sentence replaces every sample

**Where**: `docs/lld/public-api.md:209`
**What**: The sentence says a paint replaces every sample on that grid with the same value. `paint_fill` replaces only samples that still hold the seed value, and it writes the caller segment. On the permanent row `1 0 1`, paint from x 0 writing 2 leaves the middle 0.
**Why it is wrong**: `docs/lld/README.md` line 7 says an LLD file describes what the code does today. The approved plan says paint replaces every sample whose stored value equals the seed value, and that other values stay. The HLD does not define this fill, so this is not a departure from an HLD formula. It is a false description of the function. The ordinary reading of "replaces every sample with the same value" is the full-grid write that row rejects. The rustdoc at `fill.rs` lines 61 to 63 says the sample must hold the seed value. The sprint done line is outside this diff and is not charged. It uses "paint-fills" and its next sentence says the caller chooses the written segment. This paragraph uses "replaces" and does not say that.
**Evidence**: `a_gap_stops_a_flood_and_not_a_paint` expects `[2, 0, 2]` and passed on the restored tree. The loop at `fill.rs` lines 80 to 82 skips a sample that is not the seed. The temporary probe painted `[2, 1, 2]` to `[2, 9, 2]` with covered 1. `git diff HEAD` adds this paragraph as a new line.
**Scope and regression**: Documentation only. The sentence is not on HEAD. No runtime consumer reads it. The smallest fix is to say that paint replaces each sample whose stored value equals the seed, that the written number is the caller segment, and that a different value stays. The permanent gap fixture already locks the row.

## Smells

None.

## Nitpicks

None.

## Verified clean

`flood_fill` and `paint_fill` are exported with `FillCoverage` and `FillError` from `lib.rs` line 30. `FillError` has one variant, `Degenerate`, and no `Frame` variant. The seed column and row are `u32`. `covered` counts selected samples, including ones that already hold the written segment. `changed` counts the ones whose stored number differed. Both counts use `saturating_add`. AC5 covers 5 and changes 0. AC6 covers 1 and changes 0. The probe's two stored 4s cover 2 and change 0.

`prepare` returns before any clone when a side is 0, the pixel count does not fit in `u32`, the length is not `width * height`, or the seed is outside. `flood_fill` clones, builds the visit buffer, and assigns `grid.indices` only after `walk_flood` returns. A `?` from `prepare`, `mark_seed`, or `walk_flood` returns before that assignment. `paint_fill` clones only after `prepare` accepts the grid, then assigns the staged copy. The probe's long tail stayed 9. Its stale 8 stayed on a zero side. Its outside row stayed all 1s, and the next flood still wrote four 3s.

The four flood steps are one column left or right and one row up or down, through `checked_add` and `checked_sub`. There is no diagonal step. `consider` does not mark a sample outside the grid, and it queues a sample only when the stored value equals the seed. A different value stays. The ring probe left the four hole samples at 0 and left the separated bottom row at 1 on flood. Paint of that grid changed the bottom row and still left the hole. The wall probe changed the left column only on flood and changed every 1 on paint.

`LabelGrid` is row-major. Width is the column count. The fill offset is `y * width + x`, the same formula as `labelmap.rs` lines 174 to 177. There is no `as` cast in `fill.rs`. The only use of the word "as" is in the module comment. There is no `unsafe`, no `wasm-bindgen`, no glam, and no slice index. Reads and writes use `get` and `get_mut`. The crate stays `no_std` outside tests. `fill.rs` uses `alloc` for `VecDeque` and `Vec`. The fill does not call `stamp_brush` or `cut_scissor`. It does not open a device. Tiers A, B, and C stay not applicable, as the plan says, and there is no tier branch that could write a different result.

AC1 writes the plus `[0, 2, 0, 2, 2, 2, 0, 2, 0]` with covered 5 and changed 5. AC2 writes only the centre. AC3 flood writes `[2, 0, 1]` and paint writes `[2, 0, 2]`. AC4 refuses and keeps the stored ones. AC8 writes `[2, 2, 0, 0, 2, 0, 0, 2, 2]`. Those indices match the plan's hand count. PS3.3 C.8.20.2 is the background rule only. Segment 0 is a legal written value.

AC7 paints the plus to 0 and asks the unchanged `present_labelmap` for fill on and outline off. The centre sample is index 4, so its bytes are 16 through 19. Segment 0 is transparent at `labelmap.rs` lines 130 to 136. The same-segment probe kept the plus as 1s and the presenter wrote `(10, 20, 30, 40)` at bytes 4 through 7 and at bytes 16 through 19, with transparent bytes at the corner. A presenter that zeroes every pixel would fail that probe. `labelmap.rs` is not in the diff.

The changelog line says paint-fills every sample with that value, after naming the seed value. That noun phrase is not charged. The `lib.rs` lines say every copy of one value. The function docs state the diagonal rule. Both F-ID lists include F-082. A search of `*.ts`, `*.tsx`, `*.js`, and `*.mjs` found no `flood_fill` and no `paint_fill`. The new sentences have no em dash and no prose semicolon. The semicolon characters already in `public-api.md` are the pre-existing array lengths in code fences at lines 49 and 55.

The step mutation did not leave the suite green. The plus fixture stayed green for that one change because a width of 3 lets the left sample step onto the right sample. The gap fixture and the snake fixture did not stay green. The diagonal fixture stayed green because that grid has no orthogonal neighbour of the seed. The paint fixture stayed green because paint does not take that step.

## Limitations and handoff

This pass is not feature completion and it is not a sprint review. Clippy, the feature profile, wasm, and the corpus gate did not run. No GPU adapter was opened. A grid whose `width * height` exceeds `u32::MAX` was not allocated, so the `u32::try_from` refusal was read and not executed. The DICOM PDF was not opened. Segment 0 as background is the plan and the existing labelmap contract.

The ring, the wall, the long tail, the zero side, the crossed refusals, and the refusal followed by a valid flood were a temporary probe. They are not permanent tests. The probe was removed. Do not treat this pass as coverage of those cases in the suite. The permanent AC4 test checks flood for an outside column and paint for a short buffer. Both functions call `prepare`.

The product bytes were restored to the hashes in the Reviewed line before this review file was added. This pass adds only `.claude/reviews/F-082-fill-pass-1.md`. The next owner rewrites the paint sentence in `docs/lld/public-api.md`. A later independent pass reviews that change. Do not attach this result to a later tree.

F-082 pass 1: D=1 S=0 N=0
