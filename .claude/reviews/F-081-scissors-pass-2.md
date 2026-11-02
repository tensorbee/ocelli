# F-081 review, pass 2

**Reviewed**: branch `sprint/s31`, HEAD `f7e3bb0a9875025b0f616223abcc7ba97e572a07` (`S31, approve sprint designs`). `git write-tree` is `8c2bc566e0d9eb9261a18a604be91edc4cb92721`, the same tree as `HEAD^{tree}`, because the F-081 edits are unstaged and the two new Rust files are untracked. This pass reviews that working tree, not the index. Paths and SHA-256: `crates/ocelli-seg/src/scissors.rs` `107eacc47f64bdfd742c6eb4b387317d924caf56cfb9d2a5e49229cdd9b45bde`, `crates/ocelli-seg/src/brush.rs` `ea1b0b8f83d5eee8a3d45241fd599516e4868acb7b0bd375e3948c038ce3cc55`, `crates/ocelli-seg/src/lib.rs` `6304a75b9a465504488548b66134e3170deac6a8c2b3918da1becc6d8d3cee16`, `crates/ocelli-seg/tests/scissors.rs` `c799f6e41d2265e39d4fdbaaff4d5d2ba7f495eba926b2e16d910a2c03b00b6e`, `CHANGELOG.md` `fc2aca5eb6cafa7463eb2c841f8d06cda5d23db4242bad5c1dc68770cb3cbe54`, `docs/lld/README.md` `b234b1386816fb92d51a4ee2853a50fb12ff658a5b1473fa8f1d295500becc31`, `docs/lld/public-api.md` `aff3f57af1ea5daa316857d525ecd77e38d954cc7d3eef76f1b69dc82da69324`. Compared with pass 1, only the README digest and the scissor test digest moved. The other five product digests are unchanged.
**Author / reviewer**: the product diff has no commit, so this pass does not take an author from a trailer. This reviewer did not write the scissor, the brush helper move, the tests, the LLD lines, or the F-081 index cell. Pass 1 and reviewer `01a19d2a-4ab3-74f3-93b5-f282472ae39a` were not resumed. This is not a self-review and it is not a sprint review.
**Result**: 0 defects, 0 smells, 0 nitpicks

## Acceptance challenged

The contract is `.claude/plans/F-081-design.md`. `docs/hld/B-parity-surface.md` lines 25 and 29 match the plan's quotation. They count tool classes and the three representations. They do not define a scissor. The span and the radius test are the plan.

Pass 1 D1 is closed. The `public-api.md` header and the F-IDs cell on `docs/lld/README.md` line 38 are the same 21-id list, in the same order, ending `F-080, F-084, F-081`. Header minus index is empty. Index minus header is empty. There is no duplicate id. README lines 10 to 18 require those two lists to move together. They do.

AC1 through AC9 remain the original ten tests, with `a_negative_zero_bound_includes_index_zero` after AC9. Nine fixtures that pass 1 had only probed are now permanent. The expectations below were computed from the plan, not copied from a test failure.

A rectangle on slice 0 of two 3 by 3 grids of 1s covers 4 and must leave slice 1 as 1s. That rejects an erase of every slice. A rectangle with `max_x` 0 and `max_y` 1 on slice 1 must write `[0, 1, 1, 0, 1, 1, 1, 1, 1]` and leave slice 0 as 1s. A swapped axis would write `[0, 0, 1, 1, 1, 1, 1, 1, 1]`. A rectangle from `(0.5, 0)` through `(1, 0.5)` must clear only index 1. Flooring the minimum would also clear index 0. `min` of `+0` and `max` of `-0` is not inverted, because the ends are the same number, and the cut must clear only index 0. `total_cmp` ranks `+0` above `-0`, so this is not the all-`-0` bound. A grid `[4, 0, 2, 0, 4, 2, 7, 7, 7]` cut by the AC1 rectangle must cover 4, change 2, and become `[0, 0, 2, 0, 0, 2, 7, 7, 7]`. A second cut must cover 4 and change 0. A `NaN` minimum must be `Frame` with the grid unchanged, and a later rectangle from `(0, 0)` through `(0, 0)` must still clear index 0. A circle at `(1, 1, 0)` with radius 1 must cover 5 and leave the next slice as 1s. Squared distance from that centre to the next centre is 1, so a sphere would erase it. An empty stack, a zero width, and a 3 by 3 with 10 indices must be `Degenerate`. A sphere at `z` `0.5` with radius `0.5` must clear both centres only and cover 2. Both `0.5` squares are the exact value `0.25`. An orthogonal edge is `1.25` and is outside.

The green run matched those results. A separate `rustc` program, written under `/tmp` and not in this repo, printed `+0` bits `0`, `-0` bits `8000000000000000`, `total_cmp` `Greater`, and showed that passing `-0.0` keeps the high bit. The `+0` to `-0` fixture is therefore not a folded pair of positive zeros.

The upper side of `index_in_span` was then changed so a value equal to the maximum is outside. AC1 went red at `crates/ocelli-seg/tests/scissors.rs:61` with covered left 1 and right 4. The 2 by 2 block loses every sample on the excluded edge, which leaves the single open corner. The fractional fixture went red with covered left 0 and right 1, because its only sample sits on `max_x`. The column fixture went red with covered left 0 and right 2, because `max_x` is the column itself. Six further rectangle fixtures went red. All circle and sphere fixtures stayed green, and all 18 brush fixtures stayed green. The source was restored. The brush digest matches the green tree.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `bin/ocelli.sh test ocelli-seg --test scissors --test brush` | Working tree hashes above, before the mutation | Executed now | Exit 0. Brush 18 passed. Scissors 19 passed. No ignored tests |
| `index_in_span` upper side without `same_number`, then restored | `crates/ocelli-seg/src/brush.rs` line 204. Disk and ball comparisons left as `is_le` | Executed now | `bin/ocelli.sh test ocelli-seg --test scissors --test brush` exit 101. Brush 18 passed. Scissors 10 passed and 9 failed. AC1 covered left 1 right 4. Fractional covered left 0 right 1. Column covered left 0 right 2 |
| SHA-256 of the seven reviewed paths after restore | Same paths as the Reviewed line | Executed now | Each digest matches the green tree. The upper side is `same_number` or `is_lt` again |
| Python lists of `F-` ids in `docs/lld/public-api.md` line 3 and the `public-api.md` row in `docs/lld/README.md` | Working tree | Executed now | Both lists have the same 21 ids. Both differences are empty. No duplicate |
| `git diff --stat HEAD` and `git status --short` | Unstaged F-081 paths. New Rust files untracked | Executed now | Five tracked files, 90 insertions and 33 deletions. Product untracked files are `scissors.rs` and `tests/scissors.rs` |
| `git diff HEAD` of `brush.rs` | Working tree against the design commit | Executed now | Stack check, same-number test, and negative test moved with the same expressions. Circle distance no longer adds a zero `z` term. `is_le` remains |
| `/tmp` `rustc` negative-zero literal | Not a repo path and not cargo | Executed now | Exit 0. Positive bits `0`. Negative bits `8000000000000000`. Comparison `Greater` |

Environment: macOS, zsh, workspace `/Users/atulsharma/Documents/projects/tensorbee/ocelli`, date context 2026-11-02. The two cargo test commands ran one after the other. No wasm-pack ran. No patient data was used. dwv, Horos, and the medical viewer Grok were not opened.

## Defects

None. Pass 1 D1 does not survive on this tree.

## Smells

None.

## Nitpicks

None.

## Verified clean

`cut_scissor` exports `ScissorShape`, `ScissorCoverage`, `ScissorError`, and the function from `lib.rs`. `ScissorShape` derives `Debug`, `Clone`, and `Copy`. It does not derive `PartialEq`. Circle and sphere build a `Brush` with segment 0 and map `BrushError::Frame` and `BrushError::Degenerate` onto the scissor errors. The rectangle writes 0 only. `covered` counts samples inside the span, including a stored 0. `changed` counts the others. Both counts use `saturating_add`. The permanent mixed grid covers 4 and changes 2, then covers 4 and changes 0.

Finite checks run before the inverted-span check and before `require_stack`. Non-finite wins over an inverted span. The rectangle clones every slice, writes only the staged copy, and assigns those copies back only after the walk. A `u32` offset that does not resolve returns `Degenerate` before that assignment. The `NaN` rectangle leaves every 1 in place, and the following valid rectangle clears index 0.

A column is inside when `f64::from(x)` is at or above `min_x` and at or below `max_x` under `total_cmp`, with `same_number` for both zero bit patterns. The row uses the same test. `span_inverted` is `total_cmp` `is_gt` after `same_number` says the ends differ. The `+0` to `-0` fixture takes the not-inverted branch and clears index 0. The AC4 fixture takes the inverted branch on `x` and leaves the 1s. Slice selection uses `same_number` against `f64::from` of the slice index. `u32` to `f64` is exact for every `u32`. There is no `as` cast in `scissors.rs` or `brush.rs`. There is no `unsafe`, no `wasm-bindgen`, no `sqrt`, and no glam. `labelmap.rs`, `contour.rs`, `surface.rs`, `segment_tools.rs`, and `DEVIATIONS.md` are not in the diff.

`require_stack`, `strictly_negative`, and `same_number` are the old expressions with `pub(crate)` added. `disk_contains` and `ball_contains` keep `total_cmp` `is_le`. The circle helper does not add the old zero `z` square. A square of a finite value is `+0` or positive, so that dropped `+ 0.0` does not change the bits the comparison sees. The 18 brush fixtures passed on the moved helpers, including radius 1, radius just under 1, negative zero radius, one slice, a halfway sphere, and non-zero segments 1, 2, 3, 4, and 5. The rectangle mutation left those 18 green.

AC1 writes `[0, 0, 1, 0, 0, 1, 1, 1, 1]` with covered 4 and changed 4. AC2 writes the plus `[1, 0, 1, 0, 0, 0, 1, 0, 1]` with covered 5. Squared distances from `(1, 1)` are 2 at the corners, 1 at the edges, and 0 at the centre. Radius squared is 1. AC3 adds the next centre only and covers 6. AC4, AC5, and AC8 refuse and keep the stored samples. AC6 erases on `z` of `-0` and covers 0 on `z` of `0.5`. AC7 covers 5 and changes 0. AC9 asks the unchanged `present_labelmap` for the AC2 grid with fill on and outline off. Segment 0 is transparent at `labelmap.rs` lines 130 to 136. Segment 1 is the first LUT entry, because `colour` subtracts 1. The corner is bytes 0 through 3 and the centre is bytes 16 through 19. The test supplies `(10, 20, 30, 40)` and expects that colour at the corner and alpha 0 at the centre. `labelmap.rs` is not in the diff.

The changelog sentence and the `public-api.md` F-081 paragraph match this behaviour. Both say the presenter is unchanged and that there is no TypeScript copy. A search of `*.ts`, `*.tsx`, `*.js`, and `*.mjs` found no `cut_scissor`. The cut does not open a device. Tiers A, B, and C stay not applicable, as the plan says. The crate stays `no_std` outside tests and uses `alloc` for the staged copies.

## Limitations and handoff

This pass is not feature completion and it is not a sprint review. Clippy, the feature profile, wasm, and the corpus gate did not run. No GPU adapter was opened. A grid whose `width * height` exceeds `u32::MAX` was not allocated, so the offset-overflow refusal was read and not executed. The DICOM PDF was not opened. Segment 0 as background is the plan and the existing labelmap contract.

The long-buffer fixture checks `Degenerate` and length 10. It does not check that the stored 9s remain. The short-buffer fixture does check that its 8 ones remain, and `require_stack` rejects a bad length before the rectangle stages a copy. A y-only inversion is not a separate fixture. The x inversion executes the true branch of `span_inverted`, and the `+0` to `-0` span executes the false branch. An infinite `z`, a one-ULP inversion, a span from `0.5` through `0.5`, and a circle radius of `-0` were read and were not given new scissor fixtures. `NaN` already exercises `is_finite`. The brush suite already exercises a negative-zero radius.

The two-slice rectangle fixture locks covered 4 and an untouched second slice. It does not print slice 0. AC1 prints that pattern on one slice, and the column fixture prints an asymmetric pattern on slice 1. Those three observations together are what this pass relies on.

The product bytes were restored to the hashes in the Reviewed line before this review file was added. This pass adds only `.claude/reviews/F-081-scissors-pass-2.md`. Do not attach this result to a later tree.

F-081 pass 2: D=0 S=0 N=0
