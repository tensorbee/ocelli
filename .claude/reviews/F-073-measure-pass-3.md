# F-073 review, pass 3

**Reviewed**: Unstaged working tree on `sprint/s28` at HEAD `8ebd9765371d20641cc9959301846fe1f4b6d39e` (`S28, approve the annotation designs`). The index was not reviewed. Status showed those four paths as unstaged. sha256 after the height mutation was restored: `crates/ocelli-geom/src/measure.rs` `a32fbe33ac7e3408e769e9d0c1bcc85f23bab837e6b1bc75f20b5510d491f20c`, `crates/ocelli-geom/src/lib.rs` `b73d3c0f5837122ff8ee39a47eb9ba219e1278051e1c3a83f348fc495eeaf464`, `crates/ocelli-geom/tests/measure.rs` `39e082a7e15928ee58760d3b9e6113f22ea05f6ce415ba214aeba105c731ac25`, `.claude/plans/F-073-design.md` `401ec5139eed182379c2d6cb35511a2b85e2f92b3e7dec712a68e378eeaed298`. Those four hashes match the tree before the mutation. `measure.rs`, `lib.rs` and the plan match the pass 2 hashes. The test file does not. Pass 2 recorded `8e00b67bd79a37db816f2de20622ae39fe3e0d06d61702cd9c0f1b82053cdb2b`. `rustc 1.97.1 (8bab26f4f 2026-07-14)`. Diff against HEAD is 281 insertions and 30 deletions across those four files. `measure.rs` again contains `(end.z - start.z).abs()` and `points.len() < 2`.
**Author / reviewer**: The author is the implementation agent of this uncommitted tree. This reviewer is a fresh subagent, did not write the functions, and did not resume pass 1, pass 2, or subagents `01a19b80-c0d5-79c3-bec5-886896b6d697` and `01a19b8d-2454-7961-9d2e-63dc8eafd649`.
**Result**: 0 defects, 0 smells, 0 nitpicks

## Acceptance challenged

The contract is the working `.claude/plans/F-073-design.md`. HLD `docs/hld/03-architecture-and-crates.md` names `ocelli-geom` and does not give these formulas. The hand values were recomputed with Python `math` before the suite was treated as evidence. `sqrt(9 + 16) = 5`. `sqrt(25 + 25 + 9) = sqrt(59)`, about 7.681, not 3. `abs(4 - 1) = 3` and `abs(1 - 4) = 3`. `atan2(1, 1) = PI/4`. `atan2(1, -1) = 3*PI/4`. The reversed Cobb pair has signed dot `-1` and cross length 1.

AC1. `(0,0,0)` to `(3,4,0)` is 5. The test compares bits of 5.

AC2. One point and an empty list are `Degenerate` for `length`. The count check is `points.len() < 2`, before `polyline_length`.

AC3. Height of `(0,0,1)` and `(5,5,4)` is 3. The reversed pair is also 3. Removing `.abs()` left the forward pair at 3 and made the reversed pair `-3`. That is the rejected signed difference. A Euclidean result would be `sqrt(59)`, whose bits are not the bits of 3.

AC4. `angle_at((1,0,0), (0,0,0), (0,1,0))` is `PI/2` through the existing function. There is no `angle` wrapper. `rg` found no `fn angle`.

AC5. A zero-length arm is `Degenerate` on `angle_at`. The new zero-arm test calls that function.

AC6. Cobb of the x axis and `(1,1,0)` is `atan2(1, 1) = PI/4`. Reversal stays `PI/4` because `dot` is passed through `.abs()`. A signed dot of the reversal is `3*PI/4`. Pass 1 executed that mutation. This pass did not repeat it. The forward pair's signed dot is already `+1`, so `PI/4` for that pair does not depend on `abs`.

AC7. Lengths 3 and 4 report length 4 and width 3 in either argument order. Identical points stay 0. `total_cmp` orders the two finite distances.

Pass 2 D1's sentence is now true for the case it states. `tests/measure.rs` lines 5 to 6 say one finite point is degenerate for `length` and is 0 for `polyline_length`. A finite one-point list is `Degenerate` from `length` and `Ok(0)` from `polyline_length`. The existing tests `a_single_point_is_not_a_length` and `polyline_length_sums_segments` execute those two results. The sentence does not claim the non-finite one-point list. That list is `Frame` from `polyline_length` and `Degenerate` from `length`, because `length` checks the count first. Pass 2 executed that probe on the same `measure.rs` bytes. This pass did not repeat it and does not treat the omitted case as a defect.

Dropping `angle` stays closed. There is no new reason to reopen it. `length` is not a forwarding wrapper. It refuses fewer than two points before `polyline_length`.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `bin/ocelli.sh test ocelli-geom` | Working tree before the mutation. Hashes above. 1 unit test, 20 integration tests, 0 doc tests. | Executed now | Exit 0. The seven acceptance tests passed, including the pre-existing F-070 cases. |
| `height` without `.abs()` | `measure.rs` only. `end.z - start.z`. Restored after. | Executed now | `bin/ocelli.sh test ocelli-geom -- height_is_the_absolute_z_difference` exit 101. Failed at `crates/ocelli-geom/tests/measure.rs:255`. Left bits `13837309855095848960` are `-3`. Right bits `4613937818241073152` are `3`. The forward assertion did not fail. |
| Same height filter, plus the other acceptance filters, after restore | `(end.z - start.z).abs()` present. Hashes match the pre-mutation tree. | Executed now | Exit 0. `height_is_the_absolute_z_difference`, `length_of_a_three_four_five_segment_is_five`, `a_single_point_is_not_a_length`, `cobb_is_acute_and_ignores_direction`, `bidirectional_keeps_the_longer_length`, `a_right_angle_is_half_pi` and `polyline_length_sums_segments` passed. |
| Python `math` hand values | Not the implementation. | Executed now | `sqrt(25) = 5`. `sqrt(59)` is not 3. `abs(4 - 1) = 3`. `atan2(1, 1) = PI/4`. `atan2(1, -1) = 3*PI/4`. Reversed Cobb dot is `-1` and cross length is 1. |
| glam 0.30.10 `DVec3::length` and `DVec2::to_angle` | Registry source, not a second formula. | Read now | `length` is `sqrt` of the dot product. `to_angle` is `atan2(y, x)`. `DVec2::new(abs(dot), cross_len)` is `atan2(cross length, abs(dot))`. |
| `length` gate `< 2` changed to `< 1`, and one `NaN` point | Pass 2 mutated these and restored them. `measure.rs` hash is unchanged since that pass. | Prior receipt, not re-run | Pass 2 exit 101 at `tests/measure.rs:244` for the widened gate. Left `Ok(0.0)`. Right `Err(Degenerate)`. One `NaN` point was `Frame` from `polyline_length` and `Degenerate` from `length`. |
| Cobb without `.abs()` | Pass 1 mutated this and restored it. The `.abs()` call is still present. | Prior receipt, not re-run | Pass 1 exit 101 at the reversed assertion. Left bits were `3*PI/4`. Forward `PI/4` did not fail. |

## Defects

Pass 1 D1 is closed. Both module headers cite F-070 and F-073. `measure.rs` lines 3 to 8 state that `length` of fewer than two points is degenerate, that `polyline_length` of one finite point stays 0, that height is the absolute caller-space `z` difference, and that Cobb uses the absolute dot and stays in `0..=PI/2`. Those sentences match the code and the plan.

Pass 2 D1 is closed. The test header now says "One finite point". That sentence is true. One finite point is `Degenerate` for `length` and `Ok(0)` for `polyline_length`. It does not say a non-finite one-point list is 0. No new false sentence was found in the header, the function docs, or the plan edits.

No new defect.

## Smells

None.

## Nitpicks

None.

## Verified clean

`length` returns `Degenerate` for 0 or 1 points and then calls `polyline_length`. The count check runs before the finite check, so a non-finite one-point list is also `Degenerate` from `length`. Two identical finite points stay 0, because that is the F-070 distance and the plan keeps two or more points on `polyline_length`. `height` uses `(end.z - start.z).abs()` and returns `Frame` when the difference is non-finite. A non-finite coordinate is `Frame` before the subtraction. The height test expects bits of 3 for both orders. The mutation showed the reversed order is what rejects a signed difference. The expected constant 3 rejects `sqrt(59)`.

`cobb` checks finite points, refuses a non-finite span as `Frame`, and refuses an exact zero span as `Degenerate`. The cross product matches `angle_at`. The dot is passed through `.abs()`. `DVec2::new(abs(dot), cross_len).to_angle()` is the plan's `atan2`. Reversing either segment negates the dot and the cross. `abs` and the cross length keep the same angle. The header sentence that the forward pair is `PI/4` because the absolute dot is used was already judged in pass 2. It does not say the signed dot would differ on that pair. It is not reopened.

`bidirectional` calls `distance` twice. A non-finite point is `Frame`. A zero span stays 0. `total_cmp` `Less` swaps. `Equal` and `Greater` keep the first value as `length`. There is no `as` cast, no `unsafe`, no `wasm-bindgen`, no `unwrap`, no `expect` and no `panic` in the new functions. `float_cmp` is not used. Workspace `Cargo.toml` denies `float_cmp`, `unwrap_used`, `expect_used` and `panic`.

`Bidirectional` is a concrete struct. It is not a trait, not a generic with one implementer and not a forwarding wrapper. `span_length` is the old private vector length. Call sites in `distance`, `polyline_length`, `angle_at` and `project_onto_segment` changed only that name. Those formulas were not edited. Their existing tests passed in the full run. `lib.rs` is `no_std` outside tests. The new functions use `core::cmp::Ordering` and do not allocate.

The plan no longer lists `angle`. It says one finite polyline point stays 0, which is not `length`. Current `docs/lld/geometry.md` still says that about `polyline_length` and does not yet name the new functions. The plan defers that edit to completion. That is not a false claim that the files were already updated.

The plan's tier rows are all n/a. The functions do not open a device, do not allocate and do not cross the wasm boundary. `tools.ts`, shaders and `ocelli-volume` are not in the diff. `packages/core/src/tools.ts` still says a length is not computed there. No patient data was added.

The only consumer of the new functions today is `crates/ocelli-geom/tests/measure.rs`. A refusal returns `Err` and writes no stored state. The functions are stateless.

## Limitations and handoff

Not executed as permanent fixtures: a polyline of more than one segment through `length`, a non-zero equal bidirectional pair, opposite parallel segments, a perpendicular Cobb angle, height overflow of `f64::MAX` against `f64::MIN`, and `Canvas` points for the four new functions. The generic does not branch on the marker. `distance` still has a canvas instantiation test.

Clippy was not re-run. Pass 1 ran `bin/ocelli.sh clippy ocelli-geom` on these function bodies. Pass 2 changed comments, not those bodies. This pass changed the test header's finite qualifier and did not change the function bodies.

The Cobb `.abs()` mutation and the one-point `NaN` probe were not repeated. Pass 1 and pass 2 receipts are cited above. `measure.rs` is byte-identical to the pass 2 hash.

No GPU and no oracle comparison. These functions have no rendered pixels. Pass 1 D1 and pass 2 D1 are closed. This pass found no new blocking finding. It does not complete the story, does not edit the LLD, and does not record a sprint review.
