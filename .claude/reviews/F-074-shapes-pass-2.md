# F-074 review, pass 2

**Reviewed**: Unstaged working tree on `sprint/s28` at HEAD `1b0d74590e1699f3bbb05ce6b00c574bfa1d3432` (`F-073, length, height, Cobb and bidirectional`). The index was empty and was not reviewed. Paths and sha256 before the mutation, and again after `rx.abs() * ry.abs()` was restored: `crates/ocelli-geom/src/measure.rs` `9c0be8d45fee8291914590f7ebb2296a499d3213f572613bde01df7f7a1eac52`, `crates/ocelli-geom/src/lib.rs` `01be826a8ef29781d29ea88afecadeeff88354a7130f5730ae0c107290eb7f62`, `crates/ocelli-geom/tests/measure.rs` `3883e9e175190188bcaa34e9fa79e02986827c74c0ff1096f330bcf16d90f3c6`, `.claude/plans/F-074-design.md` `195cfe3e33dfcfdf389361871fed27890a0fb378e5fda28964faba06a25bff76`. The two library hashes and the plan hash match pass 1. The test hash does not. Pass 1 recorded `60c3c8becac58925014783aa6f64e029e539109529dedd8088fff499922bdf8e`. `rustc 1.97.1 (8bab26f4f 2026-07-14)`. Diff against HEAD is 164 insertions and 11 deletions across the three geom files. Pass 1 recorded 162 insertions and 11 deletions. `crates/ocelli-volume` has an empty diff. The restored area line is `core::f64::consts::PI * (rx.abs() * ry.abs())` at `crates/ocelli-geom/src/measure.rs:477`.
**Author / reviewer**: The author is the implementation agent of this uncommitted tree, including the opposite-sign remediation. This reviewer is a fresh subagent, did not write the functions or that test change, and did not resume `01a19bac-a17e-75a0-af4e-e2882ab4a654` or the reviewers named in `.claude/reviews/F-074-shapes-pass-1.md`.
**Result**: 0 defects, 0 smells, 0 nitpicks

## Closed prior findings

### Pass 1 D1, closed

Pass 1 D1 said `ellipse(-2, -3)` expects the bits of `PI * 6`, which are also the bits of `PI * ((-2) * (-3))`, so deleting `.abs()` left `ellipse_area_is_pi_times_the_radii` green.

The test now asserts `ellipse(-2.0, 3.0)` and `ellipse(2.0, -3.0)`, both against the bits of `PI * 6`. The both-negative call is gone. Replacing `rx.abs() * ry.abs()` with `rx * ry` made `ellipse_area_is_pi_times_the_radii` fail at `crates/ocelli-geom/tests/measure.rs:379`. Left bits `13849370932850926034` are `PI * ((-2) * 3)`. Right bits `4625998895996150226` are `PI * 6`. Those two integers differ only by the sign bit. The formula was restored before this report. The same four shape tests passed, and the four hashes above matched again.

No remaining implementation of D1. No new defect was found.

## Acceptance challenged

The contract is `.claude/plans/F-074-design.md`. HLD `docs/hld/B-parity-surface.md` counts about 63 tool classes, of which 26 are annotation, and a search of that file found no probe, rectangle, ellipse or circle. A search of `docs/hld/` for rectangle hit the image rectangle in `docs/hld/22-testing-and-tolerance.md` and not a shape area. The hand values were recomputed with Python `float` before the suite was treated as evidence.

AC1. Corners `(0, 0, 0)` and `(2, 3, 9)` are width 2, height 3 and area 6. Python gives `abs(2 - 0) = 2`, `abs(3 - 0) = 3` and product 6. The `z` difference 9 is not a factor. A product that included 9 would be 54. The reversed pair is width 2, height 3 and area 6. A signed reversed width of `-2` does not have the bits of `+2`. `rectangle_ignores_z` passed both orders. This pass did not mutate the rectangle formula. The assertions compare those bits, so the named wrong area is not a value the test can accept.

AC2. Radii 2 and 3 are the bits of `PI * 6`, integer `4625998895996150226`, hex `0x1.2d97c7f3321d2p+4`. Diameters 4 and 6 would be `PI * 24`, which is not that value. Opposite signs without `abs` are the negation, hex `-0x1.2d97c7f3321d2p+4`, integer `13849370932850926034`. Both-negative radii still match `PI * 6` with no `abs`. That is why pass 1 stayed green, and why the new pair is the one that rejects the deletion. The mutation failed on `ellipse(-2.0, 3.0)` after `ellipse(2.0, 3.0)` had already matched. The positive pair still does not use `abs`. The new assertion does.

AC3. A zero radius is `Degenerate`, not area 0. `a_zero_radius_is_degenerate` passed for `0` on each radius and for `-0` on the first radius. `Ok(0)` would not equal `Err(Degenerate)`. `exact_zero` checks both zero bit patterns and is called for both radii before the multiply. `-0` has bits `9223372036854775808`, not `0`. A zero test that missed the sign bit would return area 0 for `ellipse(-0.0, 3.0)`, and this assertion would fail.

AC4. Rim `(3, 4, 0)` is radius 5 and area `PI * 25`. Python `sqrt(9 + 16) = 5`. An x-only radius would be 3 and area `PI * 9`. Identical points are `Degenerate`, not area 0. `circle_radius_is_the_euclidean_length` passed all three assertions. `NaN` is `Frame`. The permanent fixture has `dz` 0, so it does not reject a radius that drops `z`. The body calls `distance`, and `distance` uses all three components. The plan's named wrong implementation is using only `x`, which the bits of 5 reject.

`circle` calls `distance`. `distance` was not edited. There is no probe wrapper and no drag-probe type. A search of `crates/ocelli-geom` found no `sample_world`. `Volume::sample_world` is still `crates/ocelli-volume/src/ortho.rs:102`. The volume diff is empty. No mean, standard deviation, SUV or histogram was added.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `bin/ocelli.sh test ocelli-geom -- ellipse_area_is_pi_times_the_radii rectangle_ignores_z a_zero_radius_is_degenerate circle_radius_is_the_euclidean_length` | Working tree before the mutation. Hashes above. | Executed now | Exit 0. 4 integration tests passed. 20 integration tests filtered out. 1 unit test filtered out. 0 doc tests. |
| `rx.abs() * ry.abs()` replaced by `rx * ry` | `measure.rs` only. Restored after. | Executed now | `bin/ocelli.sh test ocelli-geom -- ellipse_area_is_pi_times_the_radii` exit 101. Failed at `crates/ocelli-geom/tests/measure.rs:379`. Left bits `13849370932850926034`. Right bits `4625998895996150226`. Line 377 had already passed. |
| Same four shape tests after the restore | `PI * (rx.abs() * ry.abs())` present. Hashes match the pre-mutation tree. | Executed now | Exit 0. The same 4 tests passed. |
| `bin/ocelli.sh clippy ocelli-geom` | Restored tree. `cargo clippy -p ocelli-geom --all-targets -- -D warnings`. | Executed now | Exit 0. |
| Python `float` hand values | Not the implementation. | Executed now | Rectangle 2, 3, 6 in both orders. Including 9 would be 54. `PI * 6` bits `4625998895996150226`. Both-negative product matches it. One-negative product is the negation, bits `13849370932850926034`. Radius 5. `PI * 25` matches `PI * (5 * 5)` and `(PI * 5) * 5` for this radius. `PI * 9` and `PI * 24` do not match the circle and ellipse expectations. |
| `git diff --stat HEAD -- crates/ocelli-geom crates/ocelli-volume` and read of `sample_world` | Volume tree not in the geom diff. | Executed now | Geom diff only. Empty volume diff. `sample_world` remains `crates/ocelli-volume/src/ortho.rs:102`. |

## Defects

None. Pass 1 D1 is closed above and is not a current defect.

## Smells

None.

## Nitpicks

None.

## Verified clean

`rectangle` subtracts `x` and `y`, takes `abs` of each side, and multiplies those two values. `z` is not in the product. Reversed corners of the acceptance pair stay 2, 3 and 6 because each side is absolute. The expected bits are `+2` and `+3`, so a signed width of `-2` cannot pass the reversed assertion. Area 6 rejects a product that includes the `z` difference 9. A non-finite coordinate returns `Frame` through `finite_point`, including `z`. The summary says `z` is ignored, and the error paragraph says a non-finite point is `Frame`. That is the same shape as `height`, which uses only `z` in the difference and still refuses a non-finite point. The test locks `NaN` on `x`.

`ellipse` refuses a non-finite radius as `Frame` before the zero check. `exact_zero` rejects both zero signs. The area is `core::f64::consts::PI * (rx.abs() * ry.abs())`. For radii 2 and 3 the product is taken before the multiply by `PI`, and the passing assertion shows those bits are `PI * 6`. The opposite-sign assertions require the absolute values. A non-finite product returns `Frame`.

`circle` calls `distance` and then refuses an exact zero radius as `Degenerate`. Identical points therefore do not return area 0. `distance` of a point and itself is 0, from the existing distance contract, so the zero check is reached. The area is `PI * (radius * radius)`. For radius 5 that is `PI * 25`. `NaN` is `Frame` because `distance` refuses it.

The new functions have no `as` cast, no `unsafe`, no `wasm-bindgen`, no `unwrap`, no `expect`, no `panic` and no slice index. They do not compare floats with `==`. `exact_zero` uses `to_bits`. Clippy on all targets accepted the restored tree. `Rectangle` and `Circle` are concrete structs. They are not traits, not wrappers, and not a second sampler. The functions allocate nothing and store nothing. A refusal returns `Err`. No other crate calls the three functions. The only consumer in this tree is `crates/ocelli-geom/tests/measure.rs`.

Module headers cite F-074 and state the new rules. `measure.rs` says a rectangle uses absolute `x` and `y` only, an ellipse area is `PI` times the absolute radii, a zero radius is degenerate, a circle radius is the Euclidean distance, and identical points are degenerate. Those sentences match the functions. `lib.rs` says F-074 adds rectangle, ellipse and circle, and that a probe is not a second sampler in this crate. The test header cites the F-074 plan and does not describe the new areas as F-070 Euclidean results. The comment above the new tests states 2, 3, 6, `PI * 6` and `sqrt(9 + 16) = 5`. The comment above the opposite-sign assertions says a signed product is the negation of `PI * 6`. The failing bits differ from `PI * 6` only in the sign bit, so that sentence is true for these radii.

The plan's tier rows are all n/a. The functions do not open a device and do not cross the wasm boundary. `docs/lld/geometry.md` and `docs/lld/public-api.md` do not name the three shapes. The plan defers that edit to completion. That is not a false claim that those files were already updated. No patient data was added.

## Limitations and handoff

The red run stopped at line 379, so line 380 and the `NaN` assertion were not reached in that run. The restored green run executed the whole ellipse test, including `ellipse(2.0, -3.0)`. This pass did not delete only one `.abs()`. Line 379 rejects a missing absolute value on `rx` for `(-2, 3)`. Line 380 is the guard for a missing absolute value on `ry` for `(2, -3)`. It passed on the restored formula. Its red path was not executed. Python shows `PI * (2 * -3)` is the same negation as `PI * ((-2) * 3)`.

Not called in this pass: a rectangle with `NaN` `z`, a circle whose only separation is `z`, `Canvas` points for the three new functions, and an overflowing area. The generic does not branch on the marker. `distance` still has a canvas instantiation test. The permanent circle fixture has `dz` 0. A planar radius would pass it. The body calls `distance`, which keeps the third component. That is not filed as a defect because the plan's named wrong implementation is using only `x`, which the bits of 5 reject.

`GeomError::Degenerate` still says a segment, a plane normal or an angle arm has length zero. `length` already returned that variant for a short list, and the new functions return it for a zero radius. The function docs state those refusals. The variant line was not treated as a new false sentence.

`.claude/reviews/F-074-shapes-pass-1.md` was already untracked. This pass did not edit it. Clippy was run on the restored tree. No GPU and no oracle comparison. These functions have no rendered pixels. This pass does not complete the story, does not edit the LLD, and does not record a sprint review.
