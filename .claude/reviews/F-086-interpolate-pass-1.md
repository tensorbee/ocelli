# F-086 review, pass 1

**Reviewed**: staged tree `a43234b928af504b2f0d1e3c956d03a6b6110c3c` (`git write-tree` of the index). HEAD is `2a33170e56b16748e23e4084ace710ed89e41224` on `sprint/s32`. No unstaged or untracked product path was present at the start. The same write-tree was still the index after the halfway mutation was restored. This review file is not part of that tree.
**Author / reviewer**: Author is the parent implementation agent. Reviewer is independent subagent `01a19dc1-b14f-7ea0-b9a3-678dba7b8102` and did not write the diff. No earlier review was consulted.
**Result**: 0 defects, 0 smells, 0 nitpicks

## Acceptance challenged

AC1 to AC8, the five-slice consecutive-pair case, and the zero-side and bad-length refusals. The consumer checked with the new function is the unchanged `present_labelmap`. `stamp_brush` is not called.

AC2 was challenged by editing only the low-side compare from `is_le` to `is_lt` and running `the_halfway_slice_takes_the_union`. The test went red. At span 2, `t` is exactly one half, so the low mask is the exact-half contribution. Column 0 is on the low key only. Dropping `Equal` from the low side leaves the high mask, column 2, and the covered count falls from 2 to 1. The failing assertion was `coverage.covered`, left 1 and right 2, at `crates/ocelli-seg/tests/interpolate.rs:55`. The pixel assertion on line 57 did not run. The source was restored from the staged blob and the nine tests passed again.

AC6 covered 0 is correct. After AC1 the middle sample is 4, so the second call has keys at slices 0, 1 and 2. Consecutive spans are 1. Nothing lies strictly between them, and the call writes nothing. `covered` counts samples the halfway rule selects on a between-slice, including a selected sample that already stores the segment. It does not count samples on a key. The plan's AC6 row only requires the second `changed` to be 0. The ownership sentence requires the same. Covered 0 is the key rule, not a missing count of the new key. That is not a defect and not a smell.

The false arm of `if *slot != request.segment` is unreachable, and that is an honest counter rather than a microscope section 4 defect. A slice that already stores the segment is a key, and only a strict gap is filled. Each gap slice is visited once. The staged value starts as a clone of a slice that does not contain the segment, so the compare is always true when a sample is selected. The arm is the plan's split between `covered` and `changed`. It is not a guard that claims to reject an input. The key rule that makes it unreachable is what the second-call test and the adjacent-key test execute. The arm was not deleted.

The five-slice case was checked from the plan, not from the function's output. Pair (0, 2) has span 2, so `t` is 1/2 and the union of `[4, 0]` and `[0, 4]` is `[4, 4]`. Pair (2, 4) is the same union of `[0, 4]` and `[4, 0]`. Slice 2 is a key and stays `[0, 4]`. Four samples are selected and none already hold 4, so covered is 4 and changed is 4. An outer span of 4 alone would write `[4, 0]` on the between-slices and would rewrite the middle key. The test rejects that.

AC2 at exact half keeps column 0 and leaves column 1, which is on neither key, at 0. The union is `[4, 0, 4]`. AC3 uses span 3. Slice 1 has `t` of 1/3 and takes only `[4, 0]`. Slice 2 has `t` of 2/3 and takes only `[0, 4]`. Those ratios are not close enough to one half for an `f64` rounding to change the side. A `u32` ratio is exactly one half only when `2 * step == span`, and both integers fit in `f64`.

AC7 writes segment 0 through the same halfway union. The middle becomes 0. `present_labelmap` is the existing function. Its segment 0 branch writes RGBA 0 before any LUT lookup. The fixture buffer starts at 7, and the assertion expects `[0, 0, 0, 0]`. A presenter edit would have been a diff in `labelmap.rs`, and there is none.

Empty stack, zero width, zero height, a short buffer and mismatched widths all hit `require_stack` and return `Degenerate` before the staged copy is assigned. The mismatch and short-buffer fixtures keep the caller's indices.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `git write-tree`, `git diff --cached --stat`, `git status --short` | Index at review start. HEAD `2a33170e56b16748e23e4084ace710ed89e41224` | Executed now | Exit 0. Write-tree `a43234b928af504b2f0d1e3c956d03a6b6110c3c`. Six staged paths, 396 insertions. No unstaged path |
| `bin/ocelli.sh test ocelli-seg --test interpolate` | Staged `interpolate.rs` and `tests/interpolate.rs` | Executed now | Exit 0. 9 passed, 0 failed. Names: `a_middle_slice_takes_the_shared_end_segment`, `the_halfway_slice_takes_the_union`, `a_near_slice_takes_only_the_closer_mask`, `one_key_and_adjacent_keys_write_nothing`, `an_empty_stack_and_a_width_mismatch_write_nothing`, `a_second_interpolation_changes_nothing`, `interpolating_background_clears_the_presenter`, `a_slice_past_the_upper_key_stays`, `consecutive_pairs_do_not_use_the_outer_keys` |
| `bin/ocelli.sh clippy ocelli-seg` | Same tree. Wrapper is `cargo clippy -p ocelli-seg --all-targets -- -D warnings` | Executed now | Exit 0 |
| Low-side `is_le` changed to `is_lt` in the working copy only, then `bin/ocelli.sh test ocelli-seg --test interpolate the_halfway_slice_takes_the_union` | Staged blob saved at `/tmp/f086-interpolate-staged.rs` before the edit. High side stayed `is_ge` | Executed now | Exit 101. `the_halfway_slice_takes_the_union` failed at `tests/interpolate.rs:55`, `coverage.covered` left 1 and right 2. Line 57 did not run |
| `cp` of the staged blob back, then `cmp` to `crates/ocelli-seg/src/interpolate.rs` | Working file against `git show :crates/ocelli-seg/src/interpolate.rs` | Executed now | `cmp` exit 0. Low side is `is_le` again. `git write-tree` still `a43234b928af504b2f0d1e3c956d03a6b6110c3c`. No unstaged diff. Status matches the six staged paths |
| Restored `bin/ocelli.sh test ocelli-seg --test interpolate` | Restored working file, same staged blob | Executed now | Exit 0. 9 passed, 0 failed |
| `git diff --name-only HEAD` | Whole tree versus HEAD, after restore | Executed now | Only `CHANGELOG.md`, `crates/ocelli-seg/src/interpolate.rs`, `crates/ocelli-seg/src/lib.rs`, `crates/ocelli-seg/tests/interpolate.rs`, `docs/lld/README.md`, `docs/lld/public-api.md` |
| Search of the new Rust for `as `, `unsafe`, `unwrap`, `expect`, `sqrt`, `glam`, `wasm_bindgen` | `interpolate.rs` and `tests/interpolate.rs` | Executed now | No cast, no `unsafe`, no `unwrap(`, no `expect(`, no `sqrt`, no `glam`, no `wasm_bindgen`. The only `as ` hit is the English word in a doc comment. Tests use `unreachable!` for fixture shape |
| Python compare of F-ID lists | Header line of `docs/lld/public-api.md` and the last cell of the `public-api.md` row in `docs/lld/README.md` | Executed now | Equal. 25 ids, both ending in F-086, same order |

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

`interpolate_labelmap` is the only new entry point. A slice is a key when `indices.contains` finds the caller segment. Consecutive keys come from `windows(2)` in slice order. `step` runs through `1..span`, so endpoints are not written. `t` is `f64::from(step) / f64::from(span)`. The low mask uses `total_cmp` `is_le` against one half and the high mask uses `is_ge`, so the exact half is the union and a nearer slice takes one mask. Samples outside both masks are left on the staged clone. Counts use `saturating_add`. Fewer than two keys returns covered 0 and changed 0 without assigning. `require_stack` still rejects an empty stack, a zero side, a bad length and a mismatched rectangle, and every such error is `InterpolateError::Degenerate`. The grids are replaced only after the walk returns. There is no `Frame` variant.

`fill_slice` has seven arguments. `FillRequest` holds `t: f64` and does not derive `PartialEq`. `InterpolateCoverage` is two `u32` fields and derives `PartialEq` and `Eq`. No new trait, generic or `Box<dyn>`.

The new changelog bullet and the F-086 paragraph in `docs/lld/public-api.md` match those rules. Neither sentence contains an em-dash or a prose semicolon. The `lib.rs` crate sentence matches the nearer-mask and halfway-union rule and has no semicolon. The F-ID header and the README row match. There is no TypeScript file in the diff. The function does not open a device, does not mention a tier and does not call `wasm_bindgen`. Plan tiers A, B and C stay not applicable. `brush.rs`, `labelmap.rs`, `contour.rs`, `surface.rs`, `annotation.ts`, `tools.ts`, `pointer.ts`, `graph.rs`, `cpu.rs`, `voi.wgsl`, `fusion.wgsl`, `raycast.wgsl` and the other shader paths are absent from the HEAD diff. `present_labelmap` was only called.

The checked states are one key, adjacent keys, an empty stack, a width mismatch, zero width, zero height, a short buffer, a repeat after AC1, segment 0 followed by the existing presenter, a slice past the upper key, exact half, span 3, and two consecutive pairs around a middle key.

## Limitations and handoff

This pass did not run `bin/ocelli.sh gate`, the rest of `ocelli-seg`, wasm, native, the oracle, the corpus or a GPU. It did not delete the equal-slot arm. It did not execute a between-slice whose unselected sample starts as a foreign label. AC2's unselected centre starts at 0, so a clear-to-zero of unselected samples would still pass that fixture. The code leaves those samples on the clone. A span other than 2 or 3 was not fixture-tested. A non-empty rectangle with no key uses the same early return as the one-key fixture and was not separate. Checked-arithmetic `Degenerate` arms were not forced with an overflowing size. No patient data was read. The halfway fraction is the approved plan, not a PS3.3 formula. PS3.3 C.8.20.2 is only the background rule for segment 0.

No blocking finding. The product index was left as found. This file is a new unstaged review and was not committed. A clean bounded review is not the feature profile and is not sprint readiness.
