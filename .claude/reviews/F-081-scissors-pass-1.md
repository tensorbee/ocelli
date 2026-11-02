# F-081 review, pass 1

**Reviewed**: branch `sprint/s31`, HEAD `f7e3bb0a9875025b0f616223abcc7ba97e572a07` (`S31, approve sprint designs`). `git write-tree` is `8c2bc566e0d9eb9261a18a604be91edc4cb92721`, the same tree as `HEAD^{tree}`, because the F-081 edits are unstaged and the two new Rust files are untracked. This pass reviews that working tree, not the index. Paths and SHA-256: `crates/ocelli-seg/src/scissors.rs` `107eacc47f64bdfd742c6eb4b387317d924caf56cfb9d2a5e49229cdd9b45bde`, `crates/ocelli-seg/src/brush.rs` `ea1b0b8f83d5eee8a3d45241fd599516e4868acb7b0bd375e3948c038ce3cc55`, `crates/ocelli-seg/src/lib.rs` `6304a75b9a465504488548b66134e3170deac6a8c2b3918da1becc6d8d3cee16`, `crates/ocelli-seg/tests/scissors.rs` `d4634167f228eeb9e9cf111acceabc7d35239602daadb420a6bdebc6cee9662e`, `CHANGELOG.md` `fc2aca5eb6cafa7463eb2c841f8d06cda5d23db4242bad5c1dc68770cb3cbe54`, `docs/lld/README.md` `b9ad638f24d07e61019d90427a728341f4d8c1bdd86f98f38c12b9abf1d83fb7`, `docs/lld/public-api.md` `aff3f57af1ea5daa316857d525ecd77e38d954cc7d3eef76f1b69dc82da69324`.
**Author / reviewer**: the product diff has no commit, so this pass does not take an author from a trailer. This reviewer did not write the scissor, the brush helper move, the tests, or the LLD lines. No earlier F-081 review was resumed. This is not a self-review and it is not a sprint review.
**Result**: 1 defects, 0 smells, 0 nitpicks

## Acceptance challenged

The contract is `.claude/plans/F-081-design.md`. `docs/hld/B-parity-surface.md` lines 25 and 29 match the plan's quotation. They count tool classes and the three representations. They do not define a scissor. The span and the radius test are the plan.

AC1 through AC9 are the ten permanent tests in order, with `a_negative_zero_bound_includes_index_zero` after AC9. The permanent rectangle is the symmetric block from `(0, 0)` through `(1, 1)`, so those fixtures cannot reject a column and row swap. This pass ran a temporary probe, then removed it. The test file hash above is the restored file.

The probe expectations were computed from the plan, not from the implementation output. A rectangle on slice 0 of two 3 by 3 grids of 1s must leave slice 1 as 1s. That rejects an erase of every slice. A rectangle with `max_x` 0 and `max_y` 1 on slice 1 must write `[0, 1, 1, 0, 1, 1, 1, 1, 1]` and leave slice 0 as 1s. A swapped axis would write `[0, 0, 1, 1, 1, 1, 1, 1, 1]`. The same column on slice 0 must cover 2 and change 2. A circle at `(1, 1, 0)` with radius 1 on two slices must leave slice 1 as 1s and cover 5. That rejects a circle implemented as a sphere. A rectangle from `(0.5, 0)` through `(1, 0.5)` must clear only index 1. That rejects flooring the minimum. An inverted `y` with a valid `x` must be `Degenerate` and write nothing. `min` of `+0` and `max` of `-0` must not be `Degenerate`, and it must clear only index 0. That case is not the same as the permanent all-`-0` bound, because `+0` compares greater than `-0` under `total_cmp`. A grid `[4, 0, 2, 0, 4, 2, 7, 7, 7]` cut by the AC1 rectangle must cover 4, change 2, and become `[0, 0, 2, 0, 0, 2, 7, 7, 7]`. A second cut must cover 4 and change 0. A `NaN` minimum must be `Frame` with the grid unchanged, and a later rectangle from `(0, 0)` through `(0, 0)` must still clear index 0. An empty stack, a zero width, and a 3 by 3 with 10 indices must be `Degenerate` with no write. A sphere radius of `-1` must be `Degenerate`. A sphere `center_z` of `NaN` must be `Frame`. A circle with `center_z` `-0` and radius `-0` must clear only the centre of slice 0 and cover 1. A circle at `(0, 0)` with radius 1 must cover 3 and write `[0, 0, 1, 0, 1, 1, 1, 1, 1]`. A span from `0.5` through `0.5` must be `Ok` with covered 0. A 3 by 3 beside a 2 by 2 must refuse a rectangle. A rectangle `z` of `0.5` on slices filled with 3 and 4 must cover 0 and keep both patterns. A sphere at `z` `0.5` with radius `0.5` must clear both centres only and cover 2. A minimum one ULP above its maximum must be `Degenerate`. An infinite `z` must be `Frame`.

The probe passed. Those observations reject a multi-slice rectangle, a transposed rectangle, a fractional floor, an x-only inversion check, a `+0`/`-0` false inversion, a changed count that ignores an existing 0, a refusal that blocks a later cut, a circle that writes another slice, and a sphere that ignores a half-slice centre.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `bin/ocelli.sh test ocelli-seg --test scissors --test brush` | Working tree hashes above, before the probe and before the mutation | Executed now | Exit 0. Brush 18 passed. Scissors 10 passed. No ignored tests |
| Temporary `review_probe_discriminators`, then removed | Added only in `tests/scissors.rs`, then deleted. Restored hash `d4634167f228eeb9e9cf111acceabc7d35239602daadb420a6bdebc6cee9662e` | Executed now | `bin/ocelli.sh test ocelli-seg --test scissors review_probe_discriminators` exit 0. 1 passed. 10 filtered out |
| `disk_contains` `is_le` changed to `is_lt`, then restored | `crates/ocelli-seg/src/brush.rs` squared-distance comparison. Ball comparison left as `is_le` | Executed now | `bin/ocelli.sh test ocelli-seg --test scissors --test brush` exit 101. Brush 11 passed and 7 failed. `a_circle_of_radius_one_paints_the_plus_and_not_the_corners` reported covered left 1 right 5. Cargo stopped before the scissors binary. `bin/ocelli.sh test ocelli-seg --test scissors` exit 101. `a_circle_erases_the_plus_and_leaves_the_corners` reported covered left 1 right 5. `a_second_cut_of_an_already_blank_plus_changes_nothing` reported covered left 1 right 5. Rectangle tests stayed green |
| SHA-256 of the seven reviewed paths after restore | Same paths as the Reviewed line | Executed now | Each digest matches the green tree. `review_probe` is absent. `disk_contains` is `is_le` again |
| Python set of `F-` ids in `docs/lld/public-api.md` line 3 against the `public-api.md` row in `docs/lld/README.md` | Working tree | Executed now | Header minus index is `F-081` only. Index minus header is empty |
| `git diff --stat HEAD` and `git status --short` | Unstaged F-081 paths. New files untracked | Executed now | Five tracked files, 90 insertions and 33 deletions. Untracked `scissors.rs` and `tests/scissors.rs` only, before this review file |
| `git diff HEAD` of `brush.rs` | Working tree against the design commit | Executed now | Stack check, same-number test, and negative test moved with the same expressions. Circle distance no longer adds a zero `z` term. `is_le` remains |

Environment: macOS, zsh, workspace `/Users/atulsharma/Documents/projects/tensorbee/ocelli`, date context 2026-11-02. No second cargo ran beside these test commands. No wasm-pack ran. No patient data was used. dwv, Horos, and the medical viewer Grok were not opened.

## Defects

### D1, The public-api index omits F-081

**Where**: `docs/lld/README.md:38`
**What**: The F-IDs cell for `public-api.md` ends at `F-080, F-084`. The contributed line in `docs/lld/public-api.md:3` is that same list plus `F-081`.
**Why it is wrong**: `docs/lld/README.md` lines 10 to 18 say the F-IDs column carries the contributed set from the file header, and that an edit of the file must update the header and the row together. The approved plan names this row under LLD impact. The HLD does not define the index. The false claim is the drift the README already records as a review failure. This diff adds `F-081` to the header and to the area text, and it does not add `F-081` to the F-IDs cell.
**Evidence**: The Python set difference on this working tree is `F-081` only. `git diff HEAD` shows the header gaining `F-081` and the index cell staying at `F-084`. At HEAD the two lists both ended at `F-084`.
**Scope and regression**: Documentation only. It is not present on HEAD. No runtime consumer changes. The smallest fix is to append `F-081` to that F-IDs cell. No product test covers the cell.

## Smells

None.

## Nitpicks

None.

## Verified clean

`cut_scissor` exports `ScissorShape`, `ScissorCoverage`, `ScissorError`, and the function from `lib.rs`. `ScissorShape` derives `Debug`, `Clone`, and `Copy`. It does not derive `PartialEq`. Circle and sphere build a `Brush` with segment 0 and map `BrushError::Frame` and `BrushError::Degenerate` onto the scissor errors. The rectangle writes 0 only. `covered` counts samples inside the span, including a stored 0. `changed` counts the others. Both counts use `saturating_add`.

Finite checks run before the inverted-span check and before `require_stack`. Non-finite wins over an inverted span. The rectangle clones every slice, writes only the staged copy, and assigns those copies back only after the walk. A `u32` offset that does not resolve returns `Degenerate` before that assignment. The probed refusals left every index unchanged, and a later valid rectangle still cut.

A column is inside when `f64::from(x)` is at or above `min_x` and at or below `max_x` under `total_cmp`, with `same_number` for both zero bit patterns. The row uses the same test. `span_inverted` is `total_cmp` `is_gt` after `same_number` says the ends differ. Slice selection uses `same_number` against `f64::from` of the slice index. `u32` to `f64` is exact. There is no `as` cast in `scissors.rs` or `brush.rs`. There is no `unsafe`, no `wasm-bindgen`, no `sqrt`, and no glam. `labelmap.rs`, `contour.rs`, `surface.rs`, `segment_tools.rs`, and `DEVIATIONS.md` are not in the diff.

`require_stack`, `strictly_negative`, and `same_number` are the old expressions with `pub(crate)` added. `disk_contains` and `ball_contains` keep `total_cmp` `is_le`. The circle helper does not add the old zero `z` square. A square of a finite value is not negative zero, so that dropped `+ 0.0` does not change the bits the comparison sees. The 18 brush fixtures passed on the moved helpers, including the radius boundary, negative zero radius, one slice, and a non-zero segment. The `is_lt` mutation made the permanent circle stamp and the permanent circle scissor report covered 1 instead of 5. Those tests do not pass both ways.

AC1 writes `[0, 0, 1, 0, 0, 1, 1, 1, 1]` with covered 4 and changed 4. AC2 writes the plus `[1, 0, 1, 0, 0, 0, 1, 0, 1]` with covered 5. Squared distances from `(1, 1)` are 2 at the corners, 1 at the edges, and 0 at the centre. Radius squared is 1. AC3 adds the next centre only and covers 6. AC4, AC5, and AC8 refuse and keep the ones. AC6 erases on `z` of `-0` and covers 0 on `z` of `0.5`. AC7 covers 5 and changes 0. AC9 asks the unchanged `present_labelmap` for the AC2 grid with fill on and outline off. Segment 0 is transparent. Segment 1 is the first LUT entry. The corner is bytes 0 through 3 and the centre is bytes 16 through 19. The test supplies `(10, 20, 30, 40)` and expects that colour at the corner and alpha 0 at the centre. `labelmap.rs` is not in the diff.

The changelog sentence and the `public-api.md` F-081 paragraph match this behaviour. Both say the presenter is unchanged and that there is no TypeScript copy. A search of the diff found no TypeScript edit. The cut does not open a device. Tiers A, B, and C stay not applicable, as the plan says. The crate stays `no_std` outside tests and uses `alloc` for the staged copies.

## Limitations and handoff

This pass is not feature completion and it is not a sprint review. Clippy, the feature profile, wasm, and the corpus gate did not run. No GPU adapter was opened. A grid whose `width * height` exceeds `u32::MAX` was not allocated, so the offset-overflow refusal was read and not executed. The DICOM PDF was not opened. Segment 0 as background is the plan and the existing labelmap contract.

The asymmetric rectangle, the mixed segment count, the refusal followed by a valid cut, and the half-slice sphere were a temporary probe. They are not permanent tests. The probe was removed. Do not treat this pass as coverage of those cases in the suite.

The product bytes were restored to the hashes in the Reviewed line before this review file was added. The next owner adds `F-081` to the `public-api.md` F-IDs cell. A later independent pass reviews that change. Do not attach this result to a later tree.

F-081 pass 1: D=1 S=0 N=0
