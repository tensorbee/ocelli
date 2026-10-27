# F-051 review, pass 1

**Reviewed**: uncommitted work on `sprint/s18` at `a404a5e92ef6d2a6166901ca85dcccad422e002b`. Not a committed range. The behavior diff is unstaged `crates/ocelli-volume/src/lib.rs`, untracked `crates/ocelli-volume/tests/construct.rs`, and the AC2 row in unstaged `.claude/plans/F-051-design.md`. Unstaged `docs/sprints/BACKLOG.md` and `docs/sprints/CURRENT_SPRINT.md` only flip F-051 from `pending` to `in-progress`. Mutations below were reverted. `git status --short` after the last restore matches that set, and `review_probe` is not in the tree.
**Author / reviewer**: The parent-session implementation agent wrote the diff. This pass is a different agent and is not a self-review. No human name was established for either side.
**Result**: 1 defect, 0 smells, 2 nitpicks

## Acceptance challenged

AC1. Arrival order is z = 6, 0, 3 with samples `[1,1,1,1]`, `[2,2,2,2]`, `[3,3,3,3]`. Projected order is z = 0, 3, 6. Row spacing 0.5 mm and column spacing 1.0 mm must come back as `spacing` `[1.0, 0.5, 3.0]`. Origin is the lowest Image Position Patient, `(0, 0, 0)`, not a half-voxel shift. Placing by arrival index failed this test with slice 0 `[1,1,1,1]` against `[2,2,2,2]`. Swapping the spacing indexes failed it. Adding `median / 2.0` to origin z failed it with `(0, 0, 1.5)`.

AC2. Row `(1, 0, 0)` cross column `(0, -0.6, -0.8)` is `(0, 0.8, -0.6)`. A 3 mm step is `(0, 2.4, -1.8)`. Arrivals k = 2, 1, 0 put `[6,6,6,6]` first in both arrival order and ascending z. Projection must put `[4,4,4,4]` at slice 0 and keep origin `(0, 0, 0)`. The arrival-index mutation failed with slice 0 `[6,6,6,6]` against `[4,4,4,4]`. That is the same first slice a z sort would produce on this fixture, because ascending z is the arrival order. The replaced AC2 positions were shear. From `(0, 0, 10)` to `(0, -3, 0)` along normal `(0, -1, 0)` leaves a 10 mm lateral step. The current positions are collinear. An f64 check put the lateral residual near `5e-16` mm and `1e-15` mm, under the `1e-6` mm shear bound. The clean oblique test passed.

AC3. Positions 0, 3, 6 mm must report `spacing[2] == 3`. This does not reject a later assignment of that same 3. See D1. A swapped in-plane pair stays green here because the fixture uses 1 mm for both. AC1 is what rejects the swap.

AC4. Second IPP `(5, 0, 3)` against the origin is a 5 mm lateral step. Raising the shear bound from `1e-6` to `1e9` made `construct_refuses_shear` fail. The unmutated test expects `ShearedVolume` and passed.

AC5. Gaps 3, 3, 7, 3. Median 3, worst 7, index 2. The test locks those three fields. A mean of 4 does not move the index. See N2. The `median: 3.0` field would still reject a reported mean of 4.

AC6. A repeated projection returns `DuplicatePosition`. One slice returns `TooFewSlices`. Both passed. The duplicate arm does not lock the `at` field.

AC7. A 3-byte buffer on a 4-byte slice returns `SliceSizeMismatch` and no volume. Returning `Ok` of the local store instead failed the test. The `Ok` value still held slice 0 `[1,1,1,1]` and slice 1 zeros, with presence word 1. The unmutated path returns the error before that value can leave the function.

AC8. `assemble` on the AC1 candidate stays cleared, `present_count == 0`. The same bytes through `construct` fill slice 0 with `[2,2,2,2]`. The zero checks stayed green under the arrival-index mutation. The construct half went red. `assemble` does not call `place`.

AC9. Two RGB frames, three samples per pixel, 36 bytes, return `InconsistentSpacing` rather than `Ok` or `SliceSizeMismatch`. Passed. That refusal is the existing `samples_per_pixel != 1` check in `assemble_from`, reached through `construct`.

An independent probe, added only for this pass and then removed, used row 0.7 mm, column 1.3 mm, and arrival z = 10, 2, 6 with slice bytes `[9,8,7,6]`, `[1,2,3,4]`, `[5,6,7,8]`. Current `construct` returned order `[1, 2, 0]`, those bytes in that projected order, origin `(0, 0, 2)`, spacing `[1.3, 0.7, 4.0]`, `levels[0].spacing` equal to `volume.spacing`, and `is_complete`. A second `construct` of the axial stack compared equal and did not alias slice 0.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `bin/ocelli.sh test ocelli-volume --test construct` | unmutated `lib.rs` and `construct.rs` | executed now | exit 0, 9 passed |
| `bin/ocelli.sh clippy ocelli-volume` | unmutated crate, `-D warnings` | executed now | exit 0 |
| `bin/ocelli.sh test ocelli-volume` | full crate tests after restore | executed now | exit 0. lib 1, assembly 14, brick_width 2, construct 9, ready_bricks 4, doc-tests 0 |
| Arrival index used as the `place` index | same construct suite | executed now, then restored | exit 101. 6 passed, 3 failed. AC1 slice 0 `[1,1,1,1]` against `[2,2,2,2]`. AC2 slice 0 `[6,6,6,6]` against `[4,4,4,4]`. AC8 construct half matched AC1 |
| `spacing` indexes swapped in `assemble_from` | same suite | executed now, then restored | exit 101. only AC1 failed |
| Origin z plus `median / 2.0` | same suite | executed now, then restored | exit 101. AC1 and AC2 origin `(0, 0, 1.5)` against `(0, 0, 0)`. AC3 stayed green |
| Shear bound `1e-6` changed to `1e9` | same suite | executed now, then restored | exit 101. only AC4 failed |
| `place` error returned `Ok(volume)` | same suite | executed now, then restored | exit 101. AC7 `Ok` held `[1,1,1,1,0,0,0,0]` and presence word 1 |
| `SliceOutOfRange` arms replaced by index and `unwrap` | same suite, then clippy | executed now, then restored | tests exit 0, 9 passed. clippy exit 101 on `indexing_slicing` and `unwrap_used` |
| First sample splatted across the slice | same suite | executed now, then restored | exit 0, 9 passed |
| `volume.spacing[2] = 3.0` after `assemble_from` | same suite | executed now, then restored | exit 0, 9 passed |
| Temporary `review_probe_distinct_bytes_gap_four` | gap 4 mm, spacing `[1.3, 0.7]`, distinct bytes | executed now, then the test was removed | exit 0, 1 passed |
| Python f64 cross product of the AC2 cosines | literals `(1,0,0)` and `(0,-0.6,-0.8)` | executed now | normal delta 0. Lateral length about `4.97e-16` and `9.93e-16` |
| `rustc --version` | host toolchain | executed now | `rustc 1.97.1 (8bab26f4f 2026-07-14)` |

## Defects

### D1, Construct tests accept a forced 3 mm slice gap

**Where**: `crates/ocelli-volume/tests/construct.rs:93-99`, `construct.rs:152`, and `construct.rs:161-179`. The mutation was the statement after `Volume::construct`'s `assemble_from` call in `crates/ocelli-volume/src/lib.rs:554`.
**What**: Every successful `construct` fixture expects `spacing[2] == 3`. Assigning `volume.spacing[2] = 3.0` after a correct `assemble_from` left all 9 construct tests green. Level spacing was left at the real median and no acceptance test reads `levels`.
**Why it is wrong**: DICOM PS3.3 C.7.6.2.1.1 derives the inter-slice step from projected Image Position Patient, not from a constant. HLD section 19 stores that step as `spacing`. AC3 says a hard-coded stand-in must not pass. The suite can see a hard-coded 5.0 and cannot see a hard-coded 3.0, because 3 mm is the only success-path gap. HLD 27.3 says a new test that stays green when the code is wrong is not coverage. `tests/assembly.rs` varies the gap for `Volume::assemble` only. That property test does not call `construct`, so it does not see an assignment after `assemble_from`.
**Evidence**: `bin/ocelli.sh test ocelli-volume --test construct` after the assignment exited 0 with 9 passed. The source was restored. A removed probe with positions z = 2, 6, 10 and in-plane spacing 0.7 mm by 1.3 mm got `spacing` `[1.3, 0.7, 4.0]` from the unmutated function and exited 0. The algorithm is right today. The new tests do not lock it.
**Scope and regression**: Present in this working tree's new tests, not in the current `construct` body. Consumers are later callers of `Volume::construct`, starting with F-052, which will treat `spacing[2]` as millimetres. Smallest regression is one `construct` fixture whose projected gap is not 3 mm, asserting `spacing[2]` on the returned volume. The removed probe is a sufficient shape. Do not change the 0.5 percent tolerance. Uncertainty: this does not mean `assemble_from` ignores the gap. It means only the new entry point can overwrite the field without a red test.

## Smells

None.

## Nitpicks

### N1, Flat slice bytes hide a first-byte splat

**Where**: `crates/ocelli-volume/tests/construct.rs:59-74` and the other `vec![n, n, n, n]` fixtures.
**What**: Replacing the `place` buffer with `samples[0]` repeated `samples.len()` times left all 9 tests green.
**Why it does not block**: The approved expected slices are those repeated bytes, so the assertions match the written oracle. `Volume::place` copies the full slice with `copy_from_slice`. The removed probe copied `[1,2,3,4]`, `[5,6,7,8]`, and `[9,8,7,6]` in projected order. A distinct pattern in the suite would lock that, and the current code already does it.

### N2, A mean does not move the non-uniform index

**Where**: `crates/ocelli-volume/tests/construct.rs:222` and the AC5 cell in `.claude/plans/F-051-design.md`.
**What**: The comment says a mean would move the index. Gaps 3, 3, 7, 3 have mean 4. Absolute deviations from 4 are 1, 1, 3, 1. The worst gap stays at index 2, which is also the worst gap against median 3.
**Why it does not block**: The assertion still requires `median: 3.0`, `worst: 7.0`, and `index: 2`. A reported mean of 4 fails the median field. The sentence about the index is the inaccurate part, not the expected error.

## Verified clean

- `Volume::construct` calls private `assemble_from` and then `Volume::place`. It does not sort, dot, or cross on its own. `Volume::assemble` is only the by-value wrapper around `assemble_from`. There is one projection. `StackGeometry` and `ImagePlane::index_to_world` are not referenced from this crate. The in-plane equation stays in `ocelli-pixel`.
- Spacing index 0 is `column_mm()` and index 1 is `row_mm()`. `PixelSpacing::new` takes row millimetres, then column millimetres. AC1 passes `[0.5, 1.0]` and expects `[1.0, 0.5, 3.0]`. The swap mutation went red. That matches PS3.3 C.7.6.2.1.1, where `PixelSpacing[0]` multiplies the column direction.
- Origin is the lowest projected position vector, copied into `Pt::new` with no added half spacing. The half-voxel mutation went red. Direction is `DMat3::from_cols(row, column, normal)` with `normal` from `ImagePlane::slice_normal`.
- Shear, duplicate position, one slice, non-uniform gaps, and a multi-sample layout return before `place`. A short buffer fails inside `place` after the length check and before `copy_from_slice`. `construct` uses `?`. The local `Volume` is dropped. There is no `Drop` impl and no `VolumeCache::insert` on this path. The partial-`Ok` mutation is what publishes `[1,1,1,1,0,0,0,0]`, and AC7 rejects it. The unmutated function does not.
- A successful return places every `order` entry. `order` is the arrival index of each sorted position, and its length is the slice count whenever that count fits in `u32`. `Ok` then has `present_count == dims[2]`. The doc comment's completeness claim matches that path. Samples are `DecodedFrame::samples`, the stored buffer, with no LUT call.
- The two `SliceOutOfRange` arms in `construct` are unreached by every fixture, and deleting them in favour of `candidate.slices[arrival]` plus `u32::try_from(sorted).unwrap()` left the 9 tests green. They are still acceptable defensive conversions, not a smell. `assemble_from` builds `order` by enumerating the candidate, so `get(arrival)` cannot fail for an order that function produced. A sorted index fits in `u32` whenever the slice count does. When the count does not, `dims[2]` is clamped to `u32::MAX` and `place` returns `SliceOutOfRange` at sorted index `u32::MAX`, one step before `try_from` can fail. The `else` arm therefore does not run even for that overflow. Both arms exist so the match is total without `unwrap` or slice indexing. Workspace clippy denies `unwrap_used`, and `-D warnings` denies `indexing_slicing`. The replacement failed clippy at exit 101. The doc comment does not list these arms as caller-facing refusals. Either arm still returns `Err` and drops the local store, so neither can publish a partial volume. They are not an acceptance claim and they do not pretend to be one.
- The new doc comment has no em-dash and no prose semicolon. It says geometry comes from `assemble`, samples are written by `place` at `order`, a bad buffer is `SliceSizeMismatch`, and a failure returns no volume. Those reachable claims match the body. `assemble` and `assemble_from` share one implementation, so the `assemble` attribution is not a second geometry.
- No `as` cast was added. The new `u32` conversions are `try_from`. No `unsafe`. No `wasm-bindgen`. No pixels are handed across the wasm boundary. `construct` is not on a render loop. The cloned `order` vector is indexes. The store allocation remains `VoxelStore::new`. No new trait, generic, or `Box<dyn>`.
- Tier A and tier B are not applicable. The function opens no device and uploads no texture. Tier C is this same result. There is no second LUT and no silent substitute.
- F-033's assembly tests stayed green after the borrow split, including the oblique projection test, the spacing property test, and the cleared-store place test. `construct` is the only production call to `place`.
- AC2's current normal, step, origin, and projected sample order were checked against the cross product above, and the clean test passed. The plan row matches that fixture.

## Limitations and handoff

This pass did not run `bin/ocelli.sh gate --sprint`, wasm, the oracle, or a GPU. No adapter was opened. `construct` has no device path, so that is not missing arithmetic evidence.

Acceptance tests do not read `volume.order` or `levels`. The removed probe did, and both matched the geometry above. They also do not cover mixed orientation, inconsistent dimensions, or an unsupported voxel kind on the `construct` entry point. Those errors propagate through the same `?` on `assemble_from`, and the assembly suite still covers them for `assemble`.

AC6 does not lock `DuplicatePosition.at`. AC4 does not lock the shear cosine. Intra-slice byte order is N1. A second success-path gap is D1.

The sprint status edits were not treated as product behaviour. The word `in-progress` matches an open implementation. The plan's exclusive write set said those files stay untouched until completion. That process note is not a second defect.

The next owner is the implementation agent. Add a `construct` fixture whose projected gap is not 3 mm so D1 goes red if `spacing[2]` is forced to 3, keep the current mutations red, and restore nothing from this pass. A later independent pass reviews that test. This pass does not close the story.
