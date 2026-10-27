# S18 review, pass 1

**Reviewed**: `git diff 7117d14a5961e653f6a924f0e78c07ba0e27578d..e8351b51ea35fa5a3e4fc967197d72e9ecb855c5` on `sprint/s18`. HEAD is `e8351b5`. Tree `55ab1948680c120dbf09acd2f996d956fd881ba6`, confirmed by `git rev-parse HEAD^{tree}` and `git write-tree`. Commits are `29ee462` (open), `a404a5e` (design approval) and `e8351b5` (F-051). The worktree was clean at the start. Mutations in this pass were restored. `git diff --stat` and `git status --porcelain` were empty before this file was added.
**Author / reviewer**: The implementation and the two feature reviews were written by other participants. This pass is an independent subagent and did not write the code.
**Result**: 0 defects, 0 smells, 0 nitpicks

## Acceptance challenged

AC1. Arrivals at z = 6, 0, 3 carry `[1,1,1,1]`, `[2,2,2,2]`, `[3,3,3,3]`. Projected order is z = 0, 3, 6, so slice 0 is `[2,2,2,2]`. Row spacing 0.5 mm and column spacing 1.0 mm are `PixelSpacing` order. The volume triple is column millimetres, row millimetres, then the gap, so `[1.0, 0.5, 3.0]`. Origin is the lowest Image Position Patient, `(0, 0, 0)`, not a half step. Placing at the arrival index made slice 0 `[1,1,1,1]` against `[2,2,2,2]`. Swapping the two in-plane spacing fields failed the same test. Adding `median / 2.0` to origin z failed it at `(0, 0, 1.5)`.

AC2. Row `(1, 0, 0)` crossed with column `(0, -0.6, -0.8)`, after the same unit Gram-Schmidt `ImageOrientationPatient::new` applies, is normal `(0, 0.8, -0.6)`. A 3 mm step is `(0, 2.4, -1.8)`. Python f64 gave projections 0, 3 and 6, and a lateral residual of about `4.97e-16` mm on each step, under the `1e-6` mm shear bound. Ascending z is k = 2, 1, 0, the reverse of the projection. Sorting the comparator by z, while leaving the projection in the shear and gap fields, put `[6,6,6,6]` at slice 0 against `[4,4,4,4]`. The approved coordinates at `a404a5e` are not that step. Against their stated normal `(0, -1, 0)`, the steps from `(0, 0, 10)` to `(0, -3, 0)` to `(0, -6, 5)` leave lateral lengths 10 mm and 5 mm. The landed fixture matches the F-033 oblique step in `crates/ocelli-volume/tests/assembly.rs`. The plan and `AS_BUILT` both say those earlier positions were shear. That disclosure is true.

AC10. Arrivals at z = 10, 2, 6 with samples `[9,8,7,6]`, `[1,2,3,4]`, `[5,6,7,8]`, row 0.7 mm and column 1.3 mm, give sorted slices `[1,2,3,4]`, `[5,6,7,8]`, `[9,8,7,6]`, origin `(0, 0, 2)` and spacing `[1.3, 0.7, 4.0]`. Both gaps are 4 mm. The median of that pair is 4. Forcing `volume.spacing[2] = 3.0` after `assemble_from` failed the literal zip. Repeating each slice's first byte failed slice 0 as `[1,1,1,1]` against `[1,2,3,4]`. A half step moved the origin to `(0, 0, 4)`.

AC4. Second IPP `(5, 0, 3)` against the origin is a 5 mm lateral step. Raising the shear bound from `1e-6` to `1e9` made only `construct_refuses_shear` fail. The unmutated test expects `ShearedVolume`.

AC5. Gaps 3, 3, 7, 3. Median 3, worst 7, index 2. The mean is 4. Absolute deviations from 4 are 1, 1, 3, 1, so the worst index stays 2. The test locks `median: 3.0`, which is what rejects a reported mean. The corrected sentence in the test and in the plan matches that arithmetic.

AC8. `assemble` on the AC1 candidate stayed cleared under the arrival-index mutation. The construct half of the same test went red. `assemble` is still the one-line forwarder to `assemble_from` and does not call `place`.

Unchanged consumers. The restored `ocelli-volume` suite passed the 14 assembly tests, including the oblique projection test and the cleared-store place test. `crates/ocelli-viewport` does not call `Volume::construct` or `Volume::assemble`. `VolumeCache::insert` is not on the construct path.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `git rev-parse HEAD` and `git write-tree` | `sprint/s18` | executed now | `e8351b51ea35fa5a3e4fc967197d72e9ecb855c5`, tree `55ab1948680c120dbf09acd2f996d956fd881ba6` |
| `bin/ocelli.sh test ocelli-volume --test construct` | unmutated HEAD | executed now | exit 0, 10 passed |
| Same command after `volume.spacing[2] = 3.0` following `assemble_from` | `lib.rs` construct, then restored | executed now | exit 101. Only `construct_spacing_follows_a_four_millimetre_gap` failed, at `construct.rs:224`, against `[1.3, 0.7, 4.0]` |
| Same command after a first-byte splat in `place` | `lib.rs` place, then restored | executed now | exit 101. Slice 0 `[1,1,1,1]` against `[1,2,3,4]` at `construct.rs:217`. The flat fixtures stayed green |
| Same command after `place` at the arrival index | `lib.rs` construct, then restored | executed now | exit 101. AC1 slice 0 `[1,1,1,1]` against `[2,2,2,2]` at `construct.rs:86`. AC2 slice 0 `[6,6,6,6]` against `[4,4,4,4]`. AC8 failed on the construct half at `construct.rs:368`. AC10 slice 0 `[9,8,7,6]` against `[1,2,3,4]`. 6 passed |
| Same command after the sort comparator used `position.z` | projection field left intact, then restored | executed now | exit 101. Only AC2 failed, slice 0 `[6,6,6,6]` against `[4,4,4,4]` at `construct.rs:145` |
| Same command after `spacing[0]` and `spacing[1]` swapped | `assemble_from`, then restored | executed now | exit 101. AC1 failed at `construct.rs:93`. AC10 failed at `construct.rs:224`. AC2 and AC3 stayed green because both in-plane inputs are 1 mm |
| Same command after origin z plus `median / 2.0` | `assemble_from`, then restored | executed now | exit 101. AC1 and AC2 origin `(0, 0, 1.5)` against `(0, 0, 0)`. AC10 origin `(0, 0, 4)` against `(0, 0, 2)`. AC3 stayed green |
| Same command after the shear bound `1e-6` changed to `1e9` | `assemble_from`, then restored | executed now | exit 101. Only AC4 failed at `construct.rs:262` |
| `bin/ocelli.sh test ocelli-volume` | restored tree | executed now | exit 0. lib 1, assembly 14, brick_width 2, construct 10, ready_bricks 4, doc-tests 0 |
| `git diff --stat` and `git status --porcelain` | restored tree, before this review file | executed now | both empty. `spacing[2] = 3.0` is absent. `place` copies `samples` at the sorted index. Shear bound is `1e-6` |
| Python f64 of the landed and approved AC2 positions | same literals as the fixture and as `a404a5e` | executed now | landed lateral about `4.97e-16` mm. Approved laterals 10 mm and 5 mm |
| Commit trailer and `.claude/verify-ledger.json` key `55ab1948680c120dbf09acd2f996d956fd881ba6` | this tree | prior receipt, read now | profile `sprint`, `corpus=pass`, 31 gates. Not re-run |
| `rustc --version` | host toolchain | executed now | `rustc 1.97.1 (8bab26f4f 2026-07-14)` |

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

- `Volume::construct` borrows the candidate through private `assemble_from`, clones `order`, and calls `place` once per sorted index with `evidence.frame().samples()`. `order[n]` is the arrival index. The destination is the sorted index, not the arrival index. The arrival mutation is what rejects the swap. There is no second sort, dot or cross in `construct`. `ImagePlane::slice_normal` remains the one cross product. `StackGeometry` and `ImagePlane::index_to_world` are not referenced from this crate.
- Spacing index 0 is `column_mm()` and index 1 is `row_mm()`. That is PS3.3 C.7.6.2.1.1: `PixelSpacing[0]` is the row spacing and multiplies the column cosine, and `PixelSpacing[1]` is the column spacing and multiplies the row cosine. Index 2 is `median` of the projected gaps. On an even count the median averages the two middle gaps. AC10's pair is 4 and 4. The swap mutation went red on AC1 and AC10. No `as` cast was added. The new `u32` conversions are `try_from`. The diff of `lib.rs` contains no `as` cast.
- Origin is the lowest projected position, copied into `Pt::new` with no half-spacing term. HLD section 19's comment names that point as the IPP of the first slice, and the done-means text names the first slice as the lowest projected one. The half-step mutation went red, including AC10's non-zero origin.
- Shear, duplicate position, one slice, non-uniform gaps and a multi-sample layout return from `assemble_from` before `place`. A 3-byte buffer on a 4-byte slice returns `SliceSizeMismatch` from `place` before `copy_from_slice`. `construct` uses `?`, so the local volume is dropped. There is no `VolumeCache::insert` on this path and no new `VolumeError` variant. `DEVIATIONS.md` is not in the diff.
- The two `SliceOutOfRange` arms in `construct` are not reached by any fixture. `assemble_from` builds `order` by enumerating the candidate, so `get(arrival)` does not fail for an order that function produced. A sorted index fits in `u32` whenever the slice count does. When the count does not, `dims[2]` is clamped to `u32::MAX` and `place(u32::MAX)` returns `SliceOutOfRange` one step before `try_from(sorted)` can fail. The arms keep the match total without `unwrap` or slice indexing. They return `Err` and drop the local store. They are not an acceptance claim.
- `Volume::assemble` still returns a zeroed `VoxelStore` and `present_count` 0. The restored assembly suite passed, so the borrow split did not change that result. `construct` does not insert into `VolumeCache`. Stack viewport placement does not call `construct`. The shell has no export of `construct`.
- Samples are `DecodedFrame::samples`, the stored buffer. `construct` does not call a modality or VOI LUT. The one voxel allocation remains `VoxelStore::new`. The cloned `order` vector is indexes. This is a load-path call, not a render-loop allocation. No `unsafe`. No `wasm-bindgen` in the diff. No pixels are handed across the wasm boundary.
- Tier A and tier B are not applicable. The function opens no device and uploads no texture. Tier C is this same result. D-07 is cited and not changed. There is no second LUT and no silent substitute.
- Ledger. `BACKLOG.md` marks F-051 `done` and F-033 `done`. `SPRINT_TRACKER.md` has an F-051 row whose actual cell starts `not measured`. `AS_BUILT.md` has the F-051 entry. `CURRENT_SPRINT.md` status is `done`. `CHANGELOG.md` has no F-051 line. F-052, F-057, F-058 and F-059 depend on F-051 and are S19. F-X021 is S20 and depends on F-012 and F-052. F-X004 is S21 and depends on F-052 and F-X003. The s17 merge is `7117d14` with tree `6fc40a9abcad`. Its message records 31 of 31 gates and a clean whole-sprint pass 2. The wasm budget file still records 2,151,196 bytes. The workspace version is 0.1.0. `max_review_passes` is stored in `scripts/sprint_workflow.py` and is not read by a later enforcement branch.
- The new construction section in `docs/lld/volumes.md`, the F-051 `AS_BUILT` entry, `CURRENT_SPRINT.md`, the design plan and the two feature reviews were read against the code and the mutations above. The AC2 and AC10 numbers in the plan match the fixture. The feature-review claims that this pass re-ran, including the pass 1 spacing hole and the pass 2 splat, match the exits above. New Markdown prose outside code fences and table rows, and the new Rust doc comments, have no em-dash and no prose semicolon.
- `assemble` forwarding to `assemble_from` is the approved way to keep the public by-value signature. It is not a second geometry.

## Limitations and handoff

This pass did not re-run `bin/ocelli.sh gate --sprint`, clippy, wasm, the oracle or a GPU. The sprint-profile receipt for tree `55ab1948680c120dbf09acd2f996d956fd881ba6` was read from the commit trailer and the verify ledger. It says `corpus=pass` and lists 31 gates. No adapter was opened. `construct` has no device path, so that is not missing arithmetic evidence.

The `SliceOutOfRange` arms were not executed. Reaching them needs an order `assemble_from` does not produce, or a slice count above `u32::MAX`. AC6 does not lock `DuplicatePosition.at`. AC4 does not lock the shear cosine. Both tests lock the refusal variant, and the shear-bound mutation shows the AC4 arm is reached. `construct` tests do not read `volume.order`. The slice bytes require projected placement. A wipe of `order` after a correct `place` would stay green. Mixed orientation, inconsistent dimensions and an unsupported voxel kind are covered on `assemble` through the same `assemble_from` error path. The restored assembly suite passed those tests. They are not a second `construct` fixture.

Replacing the projection key itself with z made AC2 return `Err` before the slice assertion, because shear then used z as the parallel component. That mutation was restored and is not the sort claim. The comparator mutation is the one that places `[6,6,6,6]` first.

This file is the pass record. It was not staged and `scripts/sprint_workflow.py record-sprint-review` was not run. The next owner can record it against this tree if the close path requires that ledger row. This pass does not merge and does not close the sprint.
