Result: 0 defects, 0 smells, 0 nitpicks

# F-070 review, pass 1

**Reviewed**: uncommitted working tree on `sprint/s25` at `06768e19630e6336bc7824ff16eb81168ffb7db7`. The review set is `.claude/plans/F-070-design.md`, `crates/ocelli-geom/src/lib.rs`, `crates/ocelli-geom/src/measure.rs`, `crates/ocelli-geom/tests/measure.rs`, `crates/ocelli-geom/Cargo.toml`, `docs/lld/geometry.md`, and the geometry row in `docs/lld/README.md`. `measure.rs`, the integration test, and `geometry.md` are untracked. Nothing was staged or committed. `rustc 1.97.1 (8bab26f4f 2026-07-14)`.

**Author / reviewer**: this reviewer did not write the implementation. No earlier F-070 review was present.

## Acceptance challenged

The local rules in the approved plan are the formula spec. HLD section 4 names the crate and gives no distance, angle, hit, or projection formula.

Distance from `(0,0,0)` to `(3,4,0)` is 5. A point and itself is 0, not `Degenerate`. The polyline `(0,0,0)`, `(3,4,0)`, `(3,4,12)` is `5 + 12 = 17`, not the chord 13. Empty and one finite point are 0. A `NaN` on a later point refuses the whole sum.

The unsigned angle is `atan2(cross length, dot)` in `[0, pi]`. glam 0.30.10 `DVec2::to_angle` is `atan2(y, x)`, so `DVec2::new(dotted, cross_len)` is that order. Arms `(1,0,0)` and `(0,1,0)` are `FRAC_PI_2`. Opposite arms are `PI`. The same arm is 0. A zero arm is `Degenerate`.

Segment `(0,0,0)` to `(4,0,0)` and point `(1,3,0)` give `t = 4/16 = 0.25`, foot `(1,0,0)`, distance 3. Before the start, `t` is 0 and the distance is 2. Past the end, `t` is 1, the foot x is 4, and the distance is 2. Exact zero length is `Degenerate`. Length `1e-12` still projects, with squared length `1e-24`.

A hit is distance `<=` tolerance. Tolerance 3 hits `(1,3,0)`. A negative tolerance on a zero-length segment is `Frame` before `Degenerate`.

Plane origin, normal `(0,0,2)`, point `(1,2,3)` is `(1,2,0)` after dividing the normal by 2. Normal `(1,1,0)` and point `(1,0,5)` land on `(0.5, -0.5, 5)`. The un-normalised subtraction is `(0, -1, 5)`.

`GeomError::number` is 400 and 401 by match. Those numbers are not `ErrorCode` values.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `git rev-parse HEAD`, status, sha256 | working tree before edits | Executed now | HEAD `06768e19630e6336bc7824ff16eb81168ffb7db7`. `measure.rs` sha256 `2b00b1406e5b07cebeda9fc863bee927f9615df65b68299fde1d6f179b3902e8`. |
| `let unit = n` in `project_onto_plane` | division by length removed, then restored from `/tmp/f070-measure.rs.orig` | Executed now | `the_plane_normal_is_scaled_by_its_length` failed. z bits were `13844628204490326016`, which is `-9`. Expected z bits were `0`. That is `3 - 6 * 2`. |
| `len2 < 1e-20` added to the zero test | restored before the next edit | Executed now | `only_an_exact_zero_segment_is_degenerate` failed. Left was `Err(Degenerate)`. Right was `Ok` with `t` bits 0 and distance bits of `1`. |
| tolerance check moved below `project_onto_segment` | restored before the next edit | Executed now | `the_tolerance_sign_is_checked_first` failed. Left was `Err(Degenerate)`. Right was `Err(Frame)`. |
| `DVec2::new(cross_len, dotted)` | restored after both angle filters | Executed now | `angle_is_unsigned_and_uses_the_cross_and_the_dot` failed at the right-angle assertion. Left bits were `0`. Right bits were `4609753056924675352`. `a_forty_five_degree_arm_is_pi_over_four` stayed green, because `atan2(1, 1)` is unchanged when the arguments swap. |
| `bin/ocelli.sh test ocelli-geom` and `bin/ocelli.sh clippy ocelli-geom` | restored source, same sha256 as the start | Executed now | Test exit 0. Scaffold test passed. 13 integration tests passed. Clippy exit 0. |
| Prior `t = 0.5` and `<` mutations | not repeated | Prior receipt in the review request | Not re-run. The restored source still divides by `len2` and still uses `<=`. |

The final sha256 of `measure.rs` matches the pre-edit hash. `cmp` against `/tmp/f070-measure.rs.orig` was silent after the last restore.

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

`distance` is `DVec3::length` of the difference. Identical points return 0. `f64::MAX` to `f64::MIN` overflows and is `Frame`. `NaN` and infinity are `Frame` before a length is returned.

`polyline_length` checks every point, then adds `windows(2)` in order, and returns `Frame` if a span or the running total is not finite. The third-point `NaN` fixture expects `Frame`, so a returned partial 5 would fail. The `Some` continues on those windows are unreachable. `windows(2)` always yields two points, so they do not drop a segment.

`angle_at` builds the cross product on all three components, takes its length, and calls `DVec2::new(dotted, cross_len).to_angle()`. A non-negative cross length keeps the result in `[0, pi]`. A zero arm is `exact_zero` on that length, using both zero bit patterns, and is `Degenerate`. The right-angle fixture is the one that rejects a swapped `atan2`. The forty-five degree fixture locks `FRAC_PI_4` bits and does not also reject that swap.

`project_onto_segment` uses the written division and clamps with `< 0` and `> 1`. Exact zero `len2` is `Degenerate`. There is no epsilon. The `1e-12` segment is accepted, and adding a `1e-20` cutoff makes that fixture `Degenerate`.

`hit_segment` checks finite coordinates, then a non-finite or negative tolerance, then the projection. The comparison is `<=`. A miss is `Ok(false)`.

`project_onto_plane` divides the normal by `n.length()` and subtracts the component of `point - origin`. A non-unit normal is legal. Exact zero length is `Degenerate`. A `NaN` normal is `Frame`. Dropping the division fails the axial bit test. An independent evaluation of the oblique foot sits about `1.11e-16` from `(0.5, -0.5, 5)`, inside the `1e-12` assertion bound. That bound is not a section 25.1 tolerance. The un-normalised point is half a unit away, so the bound does not hide it.

Expected fixture numbers are literals or `f64` constants. They are not a second call to the function under test.

`GeomError::number` matches 400 and 401. `ErrorCode` still has only `Panicked` 1, `Unavailable` 700, and `Workgroup` 701. `ci/error-codes.json` still reserves 400 to 499 for the crate and does not list 400 or 401 as codes. `packages/core/src/errors.ts` has neither number.

The functions are generic over `Pt<S>`. The canvas fixture is also 5. Nothing in the crate calls a space transform. `SegmentProjection` is `Copy` by a hand impl with no marker bound. The crate is `no_std` outside tests. Integration tests compile that form and passed. Dependencies are `ocelli-core` and workspace `glam` with the libm backend. There is no `unsafe`, no `wasm-bindgen`, and no `wgpu`. No tier branch exists. `docs/lld/geometry.md` matches these rules, and `docs/lld/README.md` names F-070 on the geometry row.

`packages/core/src/tools.ts` stores tool modes and bindings. It does not compute a hit, a length, an angle, or a projection. No shell file calls `Math.hypot`, `Math.atan2`, or `Math.sqrt`.

## Limitations and handoff

The constant `t = 0.5` mutation and the `<` mutation were not repeated. A plane whose origin is not zero was read and not executed. Angle fixtures stay in the xy plane. `Index` was not instantiated. An infinite segment parameter that the clamp turns into 0 or 1 was traced to the later finite check and was not run. `bin/ocelli.sh gate`, wasm-pack, and `prose_check.py` were not run. `tools/oracle/src/geometry.mjs` has a pre-existing `norm` for section 25.1 and is outside this diff. It is not a shell copy of these functions. F-068 was not reviewed beyond that formula search.

The temporary edits were restored before the green suite. Product hashes match the start of the pass. This file is the only review artefact added. The result is a bounded review of this worktree, not sprint acceptance and not a verify receipt.
