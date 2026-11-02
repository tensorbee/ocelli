# F-073 review, pass 1

**Reviewed**: Unstaged working tree on `sprint/s28` at HEAD `8ebd9765371d20641cc9959301846fe1f4b6d39e` (`S28, approve the annotation designs`). The index was not reviewed. Paths and sha256 before this review file: `crates/ocelli-geom/src/measure.rs` `47221d560e787d0776021191aaa11febecf9e726fa4317728d7cb8b64bbfd8ca`, `crates/ocelli-geom/src/lib.rs` `b73d3c0f5837122ff8ee39a47eb9ba219e1278051e1c3a83f348fc495eeaf464`, `crates/ocelli-geom/tests/measure.rs` `dd8a7fe05d4907c99f76730b639f70a826490c0ec42a508e6d88a6e75d8b131d`, `.claude/plans/F-073-design.md` `162a8e325f4c0fa46473f95ad7c5fcc19b746d0f46d93ee5490ac2b329189f97`. `rustc 1.97.1 (8bab26f4f 2026-07-14)`. Diff against HEAD is 265 insertions and 22 deletions across those four files. The mutation below was reverted. `git diff --stat` after the revert matches that count, and `measure.rs` again contains `dot(first, second).abs()`.
**Author / reviewer**: The author is the implementation agent of this uncommitted tree. This reviewer is a fresh subagent, did not write the functions, and did not resume an earlier reviewer.
**Result**: 1 defect, 0 smells, 0 nitpicks

## Acceptance challenged

The contract is the working `.claude/plans/F-073-design.md`. HLD `docs/hld/03-architecture-and-crates.md` names `ocelli-geom` as hit-testing, projection and measurement mathematics. It does not give a length, a height, an angle, a Cobb angle or a bidirectional pair. No DICOM section is the formula. The hand values below were recomputed before the suite was treated as evidence.

AC1. `(0,0,0)` to `(3,4,0)` is `sqrt(9 + 16) = 5`. `length_of_a_three_four_five_segment_is_five` compares bits of 5. A first-component-only result would be 3.

AC2. One point, and an empty list, return `GeomError::Degenerate`. `polyline_length` of one point stays 0. That split is the point of `length`. Returning 0 is the rejected implementation.

AC3. Height of `(0,0,1)` and `(5,5,4)` is `abs(4 - 1) = 3`. The Euclidean length is `sqrt(59)`, not 3. The reversed pair `(z = 4, z = 1)` is also 3, so a signed difference fails. `NaN` in x is `Frame` before the subtraction.

AC4. `angle_at((1,0,0), (0,0,0), (0,1,0))` has dot 0 and cross length 1. `atan2(1, 0)` is `PI/2`. The test compares bits of `FRAC_PI_2`. A degree result would not match.

AC5. A zero-length arm is `Degenerate`, not 0. The same refusal already exists on `angle_at`.

AC6. Cobb of `(0,0,0)-(1,0,0)` and `(0,0,0)-(1,1,0)` has dot 1 and cross length 1. `atan2(1, 1)` is `PI/4`. Reversing the second segment makes the signed dot -1 and the cross length still 1. `atan2(1, abs(-1))` stays `PI/4`. A signed dot is `atan2(1, -1) = 3*PI/4`. The range of the absolute form is `0..=PI/2` because both `atan2` arguments are non-negative. `angle_at` stays `0..=PI` because its dot is signed.

AC7. Lengths 3 and 4 report length 4 and width 3 in either argument order. Two identical-point segments report 0 and 0.

Dropping `angle` is not a defect. The committed plan at HEAD gave `angle(arm_start, vertex, arm_end)` and said that function calls `angle_at`. That signature matches `angle_at` exactly, so the body would only forward. `AGENTS.md` refuses a wrapper that only forwards. The same committed bullet said any other count is `Degenerate`, which a three-argument function cannot observe. The working plan removes that second name, keeps the angle tool as `angle_at`, and says the same thing in the API list, the consumer map, the approach and the LLD note. The HLD does not name `angle`. `length` is not the same shape. It refuses fewer than two points before calling `polyline_length`, which is a different result from Ok(0).

`cobb` does not call `angle_at`. F-070 forecast that the Cobb tool would call `angle_at`. The later F-073 plan is the contract, and `angle_at` cannot satisfy it. The segments need not share a vertex, and a signed dot fails AC6. The mutation below is that failure.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `bin/ocelli.sh test ocelli-geom` | Working tree before the mutation. Hashes above. | Executed now | Exit 0. 1 unit test, 20 integration tests, 0 doc tests. All seven acceptance tests passed, including the pre-existing F-070 cases. |
| `cobb` dot without `.abs()` | `measure.rs` only. Restored after. | Executed now | `bin/ocelli.sh test ocelli-geom -- cobb_is_acute_and_ignores_direction` exit 101. Failed at `crates/ocelli-geom/tests/measure.rs:300`. Left bits `4612488097114038738` are `3*PI/4`. Right bits `4605249457297304856` are `PI/4`. The forward assertion at line 292 did not fail. |
| Same Cobb filter after restore | `dot(first, second).abs()` present. Diff stat again 265 / 22. | Executed now | Exit 0. `cobb_is_acute_and_ignores_direction` passed. |
| `bin/ocelli.sh clippy ocelli-geom` | Restored tree. `--all-targets -D warnings`. | Executed now | Exit 0. |
| glam 0.30.10 `DVec3::length` and `DVec2::to_angle` | Registry source, not a second formula. | Read now | `length` is `sqrt` of the dot product. `to_angle` is `atan2(y, x)`. `DVec2::new(abs(dot), cross_len)` is `atan2(cross length, abs(dot))`. |

No prior F-073 review receipt was used.

## Defects

### D1, Module headers still say the rules are the F-070 Euclidean rules

**Where**: `crates/ocelli-geom/src/measure.rs:3` and `crates/ocelli-geom/tests/measure.rs:1`

**What**: The module header says the rules are the local decisions in `.claude/plans/F-070-design.md`. The test header says the fixtures are those rules and that the expected numbers are the Euclidean results of those rules. This tree adds `length`, `height`, `cobb` and `bidirectional` from the F-073 plan. One point is `Degenerate`, while F-070 gives one finite point a polyline length of 0. Height of `(0,0,1)` and `(5,5,4)` is 3, while the Euclidean length is `sqrt(59)`. Cobb uses the absolute dot and stays in `0..=PI/2`. None of those decisions are in the cited F-070 plan.

**Why it is wrong**: A false provenance sentence is a defect. HLD `03-architecture-and-crates.md` names the crate and does not give these formulas. The working F-073 plan is the contract. `lib.rs:5` was updated to cite both plans. These two headers were not. A reader who checks the new functions against the header's citation will apply the F-070 one-point rule, which is the AC2 failure, or will look for a Euclidean height, which is the AC3 failure.

**Evidence**: Both plans and both headers were read. No test fails on the comment, which is why the suite stayed green. The height test still expects bits of 3, so the arithmetic is not the fault. The comment block at `tests/measure.rs:225` does cite the F-073 plan and does say height is `abs(4 - 1)`. That lower comment does not make the module header true.

**Scope and regression**: Present in this working tree. The header text is older than the new functions and was true on HEAD before they existed. No production caller imports `ocelli_geom` except this test. The smallest change is the two header sentences. The function docs already state the F-073 refusals. Uncertainty is limited to wording. The numeric tests are not the regression.

## Smells

None.

## Nitpicks

None.

## Verified clean

`length` returns `Degenerate` for 0 or 1 points and then calls `polyline_length`. Two identical points stay 0, because that is the F-070 distance and the plan keeps it. `height` subtracts caller-space `z` and takes `abs`. It does not read x or y and it does not convert spaces. A non-finite coordinate is `Frame`. Two finite values can still overflow the difference, and that path returns `Frame`.

`cobb` checks finite points, refuses a non-finite span as `Frame`, and refuses an exact zero span as `Degenerate`, including both zero signs through `exact_zero`. The cross product matches `angle_at`. The length of that cross is non-negative. `DVec2::new(abs(dot), cross_len).to_angle()` is the plan's `atan2`. Reversing a segment cannot change the result while `.abs()` stays. A zero-length segment does not become 0.

`bidirectional` calls `distance` for each segment, so a non-finite point or an overflowing span is `Frame` and a zero span stays 0. `total_cmp` orders two finite lengths. The less arm swaps. The equal and greater arms keep the first value as `length`. Equal lengths therefore share one value. `float_cmp` is not used. There is no `as` cast, no `unsafe`, no `wasm-bindgen`, no `unwrap`, no `expect`, no `panic` and no slice index in the new functions. Clippy on all targets accepted that.

`Bidirectional` is a concrete struct with public `length` and `width`. It is not a trait, not a generic with one implementer and not a forwarding wrapper. The new functions use the existing `Pt<S>` parameter. `span_length` is the old private vector length under a new name, so the public `length` can exist. Call sites inside `distance`, `polyline_length`, `angle_at` and `project_onto_segment` changed only that name. Those formulas were not edited. Their existing tests passed in the full run.

The plan's tier rows are all n/a. The functions do not open a device, do not allocate and do not cross the wasm boundary. `tools.ts` is not in the diff. Shaders, pointer code, cursor code and `ocelli-volume` are not in the diff. `docs/lld/public-api.md` is unchanged, which matches the plan's "when the story is completed" line. No patient data was added.

The only consumer of the crate today is `crates/ocelli-geom/tests/measure.rs`. A refusal returns `Err` and writes no stored state. The functions are stateless, so a second identical call has nothing to preserve.

## Limitations and handoff

Not executed as fixtures: a polyline of more than one segment through `length`, a non-zero equal bidirectional pair, opposite parallel segments (`atan2(0, positive) = 0` by inspection), a perpendicular Cobb angle, height overflow of `f64::MAX` against `f64::MIN`, and `Canvas` points for the four new functions. The generic does not branch on the marker. `distance` still has a canvas instantiation test.

`docs/lld/geometry.md` still records only F-070. The plan defers the LLD sentence to completion and names `docs/lld/public-api.md`, not `geometry.md`. That file was not in this diff. Completion should not leave `geometry.md` saying that one finite point has length 0 once `length` is public.

No GPU and no oracle comparison. These functions have no rendered pixels. The next owner is the implementation agent for D1. This pass does not complete the story and does not record a sprint review.
