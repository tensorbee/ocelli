# F-073 review, pass 2

**Reviewed**: Unstaged working tree on `sprint/s28` at HEAD `8ebd9765371d20641cc9959301846fe1f4b6d39e` (`S28, approve the annotation designs`). The index was not reviewed. sha256 after the mutation and the temporary assertions were removed: `crates/ocelli-geom/src/measure.rs` `a32fbe33ac7e3408e769e9d0c1bcc85f23bab837e6b1bc75f20b5510d491f20c`, `crates/ocelli-geom/src/lib.rs` `b73d3c0f5837122ff8ee39a47eb9ba219e1278051e1c3a83f348fc495eeaf464`, `crates/ocelli-geom/tests/measure.rs` `8e00b67bd79a37db816f2de20622ae39fe3e0d06d61702cd9c0f1b82053cdb2b`, `.claude/plans/F-073-design.md` `401ec5139eed182379c2d6cb35511a2b85e2f92b3e7dec712a68e378eeaed298`. Those four hashes match the tree before the mutation. `lib.rs` is unchanged from the pass 1 hash. The other three files changed after pass 1. `rustc 1.97.1 (8bab26f4f 2026-07-14)`. Diff against HEAD is 281 insertions and 30 deletions across those four files. `measure.rs` again contains `points.len() < 2` and `dot(first, second).abs()`.
**Author / reviewer**: The author is the implementation agent of this uncommitted tree. This reviewer is a fresh subagent, did not write the functions, and did not resume pass 1 or subagent `01a19b80-c0d5-79c3-bec5-886896b6d697`.
**Result**: 1 defect, 0 smells, 0 nitpicks

## Acceptance challenged

The contract is the working `.claude/plans/F-073-design.md`. HLD `docs/hld/03-architecture-and-crates.md` names `ocelli-geom` and does not give these formulas. The hand values were recomputed with Python `math` before the suite was treated as evidence. `sqrt(9 + 16) = 5`. `sqrt(25 + 25 + 9) = sqrt(59)`, about 7.681, not 3. `abs(4 - 1) = 3`. `atan2(1, 0) = PI/2`. `atan2(1, 1) = PI/4`. `atan2(1, -1) = 3*PI/4`.

AC1. `(0,0,0)` to `(3,4,0)` is 5. The test compares bits of 5. It stayed green when the length count gate was widened, so that gate is not what produces 5.

AC2. One point and an empty list are `Degenerate` for `length`. Widening the gate from `< 2` to `< 1` made `a_single_point_is_not_a_length` return `Ok(0.0)`. That is the rejected implementation.

AC3. Height of `(0,0,1)` and `(5,5,4)` is 3, including the reversed pair. The code subtracts `z` and takes `abs`. It does not use x or y.

AC4. `angle_at((1,0,0), (0,0,0), (0,1,0))` is `PI/2` through the existing function. There is no `angle` wrapper.

AC5. A zero-length arm is `Degenerate` on `angle_at`. The new zero-arm test calls that function.

AC6. Cobb of the x axis and `(1,1,0)` has signed dot `+1` and cross length 1, so `atan2(1, 1)` is `PI/4` with or without `abs`. Reversing the second segment makes the signed dot `-1`. `abs` keeps `PI/4`. A signed dot would be `3*PI/4`. Pass 1 executed that mutation. This pass did not repeat it.

AC7. Lengths 3 and 4 report length 4 and width 3 in either argument order. Identical points stay 0. `total_cmp` orders the two finite distances.

Discriminating case that is not an acceptance row. One `NaN` point is `Frame` from `polyline_length` and `Degenerate` from `length`, because `length` checks the count before the finite check. A header that says every one-point polyline length is 0 is the rejected reading. The temporary assertions that showed this were removed.

Dropping `angle` stays closed. There is no `fn angle`. `measure.rs` defines `angle_at` and `length`. `lib.rs` re-exports `angle_at` and does not re-export `angle`. The working plan's API list, consumer map, approach and LLD note name `angle_at` and not a second function. `AGENTS.md` still refuses a wrapper that only forwards. `length` is not that shape. It refuses fewer than two points before `polyline_length`.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `bin/ocelli.sh test ocelli-geom` | Working tree before the mutation. Hashes above. | Executed now | Exit 0. 1 unit test, 20 integration tests, 0 doc tests. |
| `length` gate `< 2` changed to `< 1` | `measure.rs` only. Restored after. | Executed now | `bin/ocelli.sh test ocelli-geom -- a_single_point_is_not_a_length length_of_a_three_four_five_segment_is_five` exit 101. Failed at `crates/ocelli-geom/tests/measure.rs:244`. Left `Ok(0.0)`. Right `Err(Degenerate)`. The two-point test stayed green. |
| Same length filter after restore | `points.len() < 2` present. Hashes match the pre-mutation tree. | Executed now | Exit 0. `a_single_point_is_not_a_length`, `length_of_a_three_four_five_segment_is_five` and `polyline_length_sums_segments` passed. |
| One `NaN` point, temporary assertions, then removed | Added to `polyline_length_sums_segments`, then deleted. Test-file hash restored. | Executed now | Exit 0 while the assertions were present. `polyline_length` of one `NaN` point was `Frame`. `length` of that point was `Degenerate`. |
| glam 0.30.10 `DVec3::length` and `DVec2::to_angle` | Registry source, not a second formula. | Read now | `length` is `sqrt` of the dot product. `to_angle` is `atan2(y, x)`. `DVec2::new(abs(dot), cross_len)` is `atan2(cross length, abs(dot))`. |
| Cobb without `.abs()` | Pass 1 mutated this and restored it. | Prior receipt, not re-run | Pass 1 exit 101 at the reversed assertion. Forward `PI/4` did not fail. Left bits were `3*PI/4`. |

## Defects

Prior D1 is closed. Both module headers now cite F-070 and F-073. `measure.rs:5` through `measure.rs:8` state that `length` of fewer than two points is degenerate, that `polyline_length` of one finite point stays 0, that height is the absolute caller-space `z` difference, and that Cobb uses the absolute dot and stays in `0..=PI/2`. Those four sentences match the code and the plan. The test header no longer tells a reader to treat the new `length` as 0 or height as a Euclidean length. The new defect is a false sentence added in that test header.

### D1, Test header drops finite from the one-point polyline rule

**Where**: `crates/ocelli-geom/tests/measure.rs:5`

**What**: The new header says "One point is degenerate for `length` and is 0 for `polyline_length`." The `length` half is true, including a non-finite point, because the count check runs first. The `polyline_length` half is not true for every one-point list. One non-finite point is `GeomError::Frame`, not 0.

**Why it is wrong**: A false sentence in the remediation is a defect. The header cites `.claude/plans/F-070-design.md`, and that plan says one finite point has length 0 and a non-finite coordinate refuses the whole sum. `docs/lld/geometry.md` lines 36 to 37 say the same thing. `measure.rs:6` in this diff states the finite qualifier. The test header does not. A reader who fixes `polyline_length` to match this header would return 0 for `NaN`, which is the result F-070 refuses.

**Evidence**: Temporary assertions in `polyline_length_sums_segments`, removed afterwards. `polyline_length` of one `NaN` point was `Err(Frame)`. `length` of that point was `Err(Degenerate)`. The filtered test exited 0. The test file hash after deletion matches the hash taken before the probe. The permanent tests do not assert this one-point `NaN` case. The false universal claim is only in the header.

**Scope and regression**: Present in this working tree. The behavior is the pre-existing F-070 behavior. No production caller imports `ocelli_geom`. The smallest change is the word finite in that one sentence. Uncertainty is limited to wording. The numeric tests are not the regression.

## Smells

None.

## Nitpicks

None.

## Verified clean

`length` returns `Degenerate` for 0 or 1 points and then calls `polyline_length`. Two identical finite points stay 0. One finite point through `polyline_length` stays 0, which the widened-gate failure showed as `Ok(0.0)`. `height` uses `(end.z - start.z).abs()` and returns `Frame` when the difference is non-finite. A non-finite coordinate is `Frame` before the subtraction. The height test expects bits of 3 for both orders, so a signed difference or `sqrt(59)` would not match.

`cobb` checks finite points, refuses a non-finite span as `Frame`, and refuses an exact zero span as `Degenerate`. The cross product matches `angle_at`. The dot is passed through `.abs()`. `DVec2::new(abs(dot), cross_len).to_angle()` is the plan's `atan2`. The forward pair's signed dot is already `+1`, so `PI/4` for that pair does not depend on `abs`. The header sentence that says the pair is `PI/4` because the absolute dot is used does not say the signed dot would differ on that pair. The absolute dot is what keeps the reversed segment at `PI/4`. The comment above the tests says that, and `cobb_is_acute_and_ignores_direction` asserts both orientations. That header sentence is not a second defect.

`bidirectional` calls `distance` twice. A non-finite point is `Frame`. A zero span stays 0. `total_cmp` `Less` swaps. `Equal` and `Greater` keep the first value as `length`. There is no `as` cast, no `unsafe`, no `wasm-bindgen`, no `unwrap`, no `expect` and no `panic` in the new functions. `float_cmp` is not used.

`Bidirectional` is a concrete struct. It is not a trait, not a generic with one implementer and not a forwarding wrapper. `span_length` is the old private vector length. Call sites in `distance`, `polyline_length`, `angle_at` and `project_onto_segment` changed only that name. Those formulas were not edited. Their existing tests passed in the full run.

The plan no longer lists `angle`. The post-pass-1 plan text also names `docs/lld/geometry.md` beside `docs/lld/public-api.md`, and it says one finite polyline point stays 0, which is not `length`. Current `docs/lld/geometry.md` still says that about `polyline_length` and does not yet name the new functions. The plan defers that edit to completion. That is not a false claim that the files were already updated.

The plan's tier rows are all n/a. The functions do not open a device, do not allocate and do not cross the wasm boundary. `tools.ts`, shaders and `ocelli-volume` are not in the diff. No patient data was added.

The only consumer of the crate today is `crates/ocelli-geom/tests/measure.rs`. A refusal returns `Err` and writes no stored state. The functions are stateless.

## Limitations and handoff

Not executed as permanent fixtures: a polyline of more than one segment through `length`, a non-zero equal bidirectional pair, opposite parallel segments, a perpendicular Cobb angle, height overflow of `f64::MAX` against `f64::MIN`, and `Canvas` points for the four new functions. The generic does not branch on the marker. `distance` still has a canvas instantiation test.

Clippy was not re-run. Pass 1 ran `bin/ocelli.sh clippy ocelli-geom` on the same function bodies. This pass changed comments and plan sentences, not those bodies.

The Cobb `.abs()` mutation was not repeated. Pass 1's receipt is cited above and is not this pass's execution.

No GPU and no oracle comparison. These functions have no rendered pixels. The next owner is the implementation agent for D1. This pass does not complete the story and does not record a sprint review.
