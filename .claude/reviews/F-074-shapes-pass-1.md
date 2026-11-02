# F-074 review, pass 1

**Reviewed**: Unstaged working tree on `sprint/s28` at HEAD `1b0d74590e1699f3bbb05ce6b00c574bfa1d3432` (`F-073, length, height, Cobb and bidirectional`). The index was empty and was not reviewed. Paths and sha256 before any review edit: `crates/ocelli-geom/src/measure.rs` `9c0be8d45fee8291914590f7ebb2296a499d3213f572613bde01df7f7a1eac52`, `crates/ocelli-geom/src/lib.rs` `01be826a8ef29781d29ea88afecadeeff88354a7130f5730ae0c107290eb7f62`, `crates/ocelli-geom/tests/measure.rs` `60c3c8becac58925014783aa6f64e029e539109529dedd8088fff499922bdf8e`, `.claude/plans/F-074-design.md` `195cfe3e33dfcfdf389361871fed27890a0fb378e5fda28964faba06a25bff76`. Those four hashes match again after the mutations and the temporary probe were removed. `rustc 1.97.1 (8bab26f4f 2026-07-14)`. Diff against HEAD is 162 insertions and 11 deletions across the three geom files. `crates/ocelli-volume` has an empty diff. `measure.rs` again contains `core::f64::consts::PI * (rx.abs() * ry.abs())`.
**Author / reviewer**: The author is the implementation agent of this uncommitted tree. This reviewer is a fresh subagent, did not write the functions, and did not resume subagents `01a19b80-c0d5-79c3-bec5-886896b6d697`, `01a19b8d-2454-7961-9d2e-63dc8eafd649`, or `01a19b98-8766-7ea0-85cc-65a4e5a4587b`.
**Result**: 1 defect, 0 smells, 0 nitpicks

## Acceptance challenged

The contract is `.claude/plans/F-074-design.md`. HLD `docs/hld/B-parity-surface.md` counts tool classes and does not define a probe, a rectangle, an ellipse or a circle. The hand values were recomputed with Python `float` before the suite was treated as evidence. `abs(2 - 0) = 2`, `abs(3 - 0) = 3`, and the product is 6. The `z` difference 9 is not a factor. Reversed corners stay 2, 3 and 6. Radii 2 and 3 have absolute product 6, so the area is `PI * 6`. `sqrt(9 + 16) = 5`, and the circle area is `PI * 25`.

AC1. Corners `(0, 0, 0)` and `(2, 3, 9)` are width 2, height 3 and area 6. The reversed pair is the same. Including 9 in the product cannot equal 6. A separate finite pair, `(-1, 5, 100)` and `(4, 1, 100.5)`, is width 5, height 4 and area 20. The `z` step of 0.5 does not enter. That probe was removed.

AC2. Radii 2 and 3 are the bits of `PI * 6`. Diameters 4 and 6 would be `PI * 24`. Replacing `PI` with `FRAC_PI_2` makes the positive assertion fail. For this pair, `(PI * 2) * 3` has the same bits as `PI * 6`, so the parentheses are not what that assertion rejects. Both-negative radii also match `PI * 6` with no `abs`. Opposite signs do not. That is D1.

AC3. A zero radius is `Degenerate`, not area 0. The permanent test expects `Err` for `0` and for `-0` on the first radius. `Ok(0)` would fail that assertion. `exact_zero` checks both zero signs and is called for both radii.

AC4. Rim `(3, 4, 0)` is radius 5 and area `PI * 25`. An x-only radius would be 3 and area `PI * 9`. Identical points are `Degenerate`, not area 0. `NaN` is `Frame`. A pure `z` segment from `(0, 0, 0)` to `(0, 0, 5)` was radius 5 on the restored `circle`, so this body includes `z`. The permanent fixture has `dz` 0, so it does not reject a planar radius. That gap is a limitation. The named rejection in the plan is using only `x`, and the permanent assertion rejects that.

`circle` calls `distance`. `distance` was not edited. There is no probe wrapper and no drag-probe type. `rg` over `crates/ocelli-geom` found no `sample_world`. `Volume::sample_world` is still `crates/ocelli-volume/src/ortho.rs:102` and the volume tree is unchanged. No mean, standard deviation, SUV or histogram was added.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `bin/ocelli.sh test ocelli-geom` | Working tree before either mutation. Hashes above. | Executed now | Exit 0. 1 unit test, 24 integration tests, 0 doc tests. The four new tests passed with the older geom tests. |
| `ellipse` area constant `PI` replaced by `FRAC_PI_2` | `measure.rs` only. Restored after. | Executed now | `bin/ocelli.sh test ocelli-geom -- ellipse_area_is_pi_times_the_radii` exit 101. Failed at `crates/ocelli-geom/tests/measure.rs:377`. Left bits `4621495296368779730` are `(PI/2)*6`. Right bits `4625998895996150226` are `PI*6`. |
| `rx.abs() * ry.abs()` replaced by `rx * ry` | `measure.rs` only. Restored after. | Executed now | Same ellipse filter, exit 0. `ellipse(-2, -3)` stays the bits of `PI*6`. |
| Same ellipse filter after both restores | `PI * (rx.abs() * ry.abs())` present. Hashes match the pre-mutation tree. | Executed now | Exit 0. |
| `bin/ocelli.sh clippy ocelli-geom` | Restored tree. `cargo clippy -p ocelli-geom --all-targets -- -D warnings`. | Executed now | Exit 0. |
| Python `float` hand values | Not the implementation. | Executed now | Rectangle 2, 3, 6 in both orders. `PI*6` hex `0x1.2d97c7f3321d2p+4`. Both-negative product matches it. One-negative product is the negation and does not match. Radius 5. `PI*25` matches `PI*(5*5)` and `(PI*5)*5` for this radius. |
| Temporary `review_probe_opposite_sign_and_z`, then deleted | Restored formula. Not left in the tree. | Executed now | Exit 0. `ellipse(-2, 3)` and `ellipse(2, -3)` matched `PI*6`. `NaN` `z` was `Frame`. The 5 by 4 rectangle was area 20. `circle` along `z` to 5 was radius 5. Infinity was `Frame`. `ellipse(-0, -0)` was `Degenerate`. |
| `git diff --stat -- crates/ocelli-volume` and `git grep fn sample_world HEAD` | Volume tree, not the geom diff. | Executed now | Empty diff. `sample_world` remains line 102. |

## Defects

### D1, Both-negative ellipse radii do not reject a missing absolute value

**Where**: `crates/ocelli-geom/tests/measure.rs:378`. The formula under test is `crates/ocelli-geom/src/measure.rs:477`.

**What**: `ellipse(-2.0, -3.0)` expects the bits of `PI * 6`. Those are also the bits of `PI * ((-2) * (-3))`. Deleting both `.abs()` calls leaves `ellipse_area_is_pi_times_the_radii` green. The positive pair and a `NaN` radius do not use the absolute value either.

**Why it is wrong**: The F-074 plan says the area is `PI * abs(rx) * abs(ry)`. A test that stays green when that absolute value is removed will be counted as coverage of negative radii. Opposite signs are the case it changes. `PI * ((-2) * 3)` is the negation of `PI * 6`. Python `float` gives `-0x1.2d97c7f3321d2p+4` against `0x1.2d97c7f3321d2p+4`. The HLD does not define this area. The plan is the contract. HLD 27.2 R2 is the rule that a test must fail when the arithmetic is wrong.

**Evidence**: The `rx * ry` mutation and the exit 0 run are in the evidence table. The `FRAC_PI_2` mutation shows the positive assertion does fail when the factor changes. It stops at line 377, so the both-negative line was not reached in that run. On the restored formula, a removed probe observed that `ellipse(-2, 3)` and `ellipse(2, -3)` match `PI * 6`. The implementation currently applies `abs`. The test does not lock it.

**Scope and regression**: Present in this working tree. No other crate calls `ellipse`. The current caller result is still a positive area. The smallest regression is one opposite-sign assertion, `ellipse(-2.0, 3.0)` or `ellipse(2.0, -3.0)`, expecting the bits of `PI * 6`. Do not change the formula to satisfy the existing assertion. Uncertainty is limited to the deleted probe not being part of the suite. The green mutation run was on the permanent test.

## Smells

None.

## Nitpicks

None.

## Verified clean

`rectangle` subtracts `x` and `y`, takes `abs` of each side, and multiplies those two values. Reversed corners of the acceptance pair stay 2, 3 and 6 because each side is absolute. The expected bits are `+2` and `+3`, so a signed width of `-2` cannot pass the reversed assertion. Area 6 rejects a product that includes the `z` difference 9. A non-finite coordinate returns `Frame` through `finite_point`, including `z`. The summary says `z` is ignored, and the error paragraph says a non-finite point is `Frame`. That is the same shape as the height rule, which ignores `x` and `y` in the difference and still refuses a non-finite point. The area field says `z` is not a factor. A removed probe saw `Frame` for `NaN` `z` and area 20 for a pair whose `z` values differ. Zero width is `Ok` with area 0. The plan does not call that `Degenerate`.

`ellipse` refuses a non-finite radius as `Frame` before the zero check. `exact_zero` rejects both zero signs. The area is `core::f64::consts::PI * (rx.abs() * ry.abs())`. For radii 2 and 3 the product is taken before the multiply by `PI`, and those bits are `PI * 6`. `core::f64::consts::PI` compiled in the `no_std` library. Integration tests build that library with `cfg(test)` off. A non-finite product returns `Frame`.

`circle` calls `distance` and then refuses an exact zero radius as `Degenerate`. Identical points therefore do not return area 0. The area is `PI * (radius * radius)`. For radius 5 that is `PI * 25`. `NaN` is `Frame` because `distance` refuses it. A removed probe showed a pure `z` segment uses the third component.

The new functions have no `as` cast, no `unsafe`, no `wasm-bindgen`, no `unwrap`, no `expect`, no `panic` and no slice index. They do not compare floats with `==`. `exact_zero` uses `to_bits`. Clippy on all targets accepted the restored tree. Workspace `Cargo.toml` denies `float_cmp`, `unwrap_used`, `expect_used` and `panic`. `Rectangle` and `Circle` are concrete structs. They are not traits, not new generic parameters with one implementer, and not wrappers. There is no sampler that forwards to `sample_world`. The functions allocate nothing and store nothing. A refusal returns `Err`.

Module headers cite F-074 and state the new rules. `measure.rs` says a rectangle uses absolute `x` and `y` only, an ellipse area is `PI` times the absolute radii, a zero radius is degenerate, a circle radius is the Euclidean distance, and identical points are degenerate. Those sentences match the functions. `lib.rs` says F-074 adds rectangle, ellipse and circle, and that a probe is not a second sampler in this crate. The test header cites the F-074 plan and does not describe the new areas as F-070 Euclidean results. The comment above the new tests states 2, 3, 6, `PI * 6` and `sqrt(9 + 16) = 5`.

The plan's tier rows are all n/a. The functions do not open a device and do not cross the wasm boundary. `docs/lld/geometry.md` and `docs/lld/public-api.md` do not yet name the three shapes. The plan defers that edit to completion. That is not a false claim that those files were already updated. No patient data was added.

## Limitations and handoff

Not left in the suite: an opposite-sign ellipse, a rectangle with `NaN` `z`, a circle whose only separation is `z`, `Canvas` points for the three new functions, and an overflowing area. The generic does not branch on the marker. `distance` still has a canvas instantiation test. The permanent circle fixture has `dz` 0. A planar radius would pass it. The body calls `distance`, and the removed probe included `z`. That is not filed as a defect because the plan's named wrong implementation is using only `x`, which the bits of 5 reject.

`GeomError::Degenerate` still says a segment, a plane normal or an angle arm has length zero. `length` already returned that variant for a short list, and the new functions return it for a zero radius. The function docs state those refusals. The variant line was not treated as a new false sentence.

Clippy was run on the restored tree. No GPU and no oracle comparison. These functions have no rendered pixels. This pass does not complete the story, does not edit the LLD, and does not record a sprint review. The next change is the opposite-sign assertion. A later independent pass has to review that test.
