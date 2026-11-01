# F-070, hit-testing, projection and measurement mathematics

**Status**: approved
**Epic ref**: E11.3
**Sprint**: S25
**Estimate**: 4w

## Normative source, transcribed

HLD `docs/hld/03-architecture-and-crates.md` section 4, the crate row:

```text
ocelli-geom | Hit-testing, projection, measurement mathematics | yes | yes
```

HLD `docs/hld/07-concurrency-and-typescript.md` says the geometry and hit-testing behind tool state machines is Rust, in `ocelli-geom`.

HLD `docs/hld/12-workspace-and-build.md` names the directory `ocelli-geom/` for hit-test and measurement math.

Deviation D-02 says the backlog name `tb-geom` is the spreadsheet spelling. The crate is `ocelli-geom`.

HLD `docs/hld/13-core-types.md` section 16 defines `Pt<S>` and `Transform<A, B>`. The markers are `Canvas`, `World` and `Index`. `apply` maps `(x, y, z, 1)` and divides by `w`. `then` applies the receiver first. That listing is already implemented in `ocelli-core`. This story consumes `Pt<S>`. It does not reimplement `Transform`.

The allocation note says the mathematics is the Rust part worth moving.

The HLD gives no distance formula, no angle formula, no hit tolerance and no projection formula. Nothing below is presented as a transcribed equation.

## What the specification does not cover

These are local decisions.

- The mathematics is generic over `Pt<S>`. The functions do not convert spaces. A canvas point reaches them only after the caller applies an F-048 transform. Camera projection stays in `ocelli-viewport`.
- The word "projection" in the crate row is taken as orthogonal projection onto a finite segment and onto a plane. It is not the camera matrix.
- Distance is the Euclidean length of the difference. A point and itself have distance 0. That is not a degeneracy.
- Polyline length is the sum of the segment distances in order. Zero points or one finite point have length 0. A non-finite coordinate refuses the whole sum. No partial length is returned.
- The angle at a vertex is the unsigned angle between the two arms, in radians, in `[0, pi]`. It is `atan2` of the cross-product length and the dot product. A zero-length arm is degenerate. Cobb angle, as a tool, is F-073. This function is the mathematics that tool will call.
- Segment projection uses `t = dot(point - start, end - start) / dot(end - start, end - start)`, then clamps `t` into `[0, 1]`. The closest point is `start + t * (end - start)`. An exact zero-length segment is degenerate. The zero test is the IEEE zero of that denominator, both signs, not an epsilon and not `==` written on an `f64`.
- A hit is that distance `<=` the caller tolerance, in the units of `S`. Tolerance 0 hits only a point whose distance is 0. A negative tolerance is `Frame`, checked before the zero-length test. A miss is `Ok(false)`, not an error.
- Plane projection divides the normal by its length. A non-unit normal is legal. An exact zero normal is degenerate. The plane is the set of points orthogonal to that direction through `origin`.
- Non-finite inputs, and a finite calculation that overflows to a non-finite result, are `GeomError::Frame`. No `NaN` is returned as a measurement.
- `GeomError::Frame` is number 400 and `GeomError::Degenerate` is number 401, in the `ocelli-geom` range. They are not added to `ErrorCode`. Nothing in this story writes the ring. Registering a code with no producer is the reservation `ocelli-core` already refuses to invent.
- There is no TypeScript copy of the arithmetic. The backlog layer cell says Rust+TS because the epic does. The story note assigns the mathematics to Rust. A second implementation in the shell is the defect HLD section 18 forbids for the LUT, applied here.
- No wasm-bindgen export. `ocelli-wasm` is the only bindgen crate, and F-069 has not asked for a measurement command.

Check order for the segment operations is fixed, so a test can see which refusal wins:

1. Any non-finite coordinate or tolerance returns `Frame`.
2. A negative tolerance returns `Frame`.
3. An exact zero-length segment or normal returns `Degenerate`.
4. Otherwise the value.

## Problem and reproducer

`ocelli-geom` is the F-001 scaffold. It exports `CRATE_NAME` and no measurement. A later length tool would have nowhere to put the arithmetic except the shell.

The observation is a Rust test, not a viewport. The segment from `(0, 0, 0)` to `(4, 0, 0)` and the point `(1, 3, 0)` project to `(1, 0, 0)` with `t = 0.25` and distance `3`. A constant `t` of `0.5` produces `(2, 0, 0)` and is the wrong implementation this fixture rejects.

The plane through the origin with normal `(0, 0, 2)` sends `(1, 2, 3)` to `(1, 2, 0)`. Treating that normal as already unit-length, or subtracting `dot * normal` without dividing by the length, does not produce that point.

## Scope, API and compatibility

In scope:

- `crates/ocelli-geom` gains `distance`, `polyline_length`, `angle_at`, `project_onto_segment`, `hit_segment` and `project_onto_plane`.
- `GeomError` and `SegmentProjection`.
- Integration tests in `crates/ocelli-geom/tests/measure.rs`.
- A short `docs/lld/geometry.md` when the story is completed.

Out of scope:

- Tool registration and modes. That is F-068.
- Event dispatch, SVG, cursors, and the length tool UI. Those are F-069, F-071, F-072 and F-073.
- Editing `Transform`, `ImagePlane`, or the F-048 viewport adapters.
- A tolerance change to section 25.1. These tests use exact values or the named constant's bits. They do not widen a render tolerance.
- Volume ray, fusion, VOI, and the oracle producer.

Signatures:

```rust
pub enum GeomError { Frame = 400, Degenerate = 401 }

pub struct SegmentProjection<S> { pub point: Pt<S>, pub t: f64, pub distance: f64 }

pub fn distance<S>(a: Pt<S>, b: Pt<S>) -> Result<f64, GeomError>
pub fn polyline_length<S>(points: &[Pt<S>]) -> Result<f64, GeomError>
pub fn angle_at<S>(arm_start: Pt<S>, vertex: Pt<S>, arm_end: Pt<S>) -> Result<f64, GeomError>
pub fn project_onto_segment<S>(point: Pt<S>, start: Pt<S>, end: Pt<S>) -> Result<SegmentProjection<S>, GeomError>
pub fn hit_segment<S>(point: Pt<S>, start: Pt<S>, end: Pt<S>, tolerance: f64) -> Result<bool, GeomError>
pub fn project_onto_plane<S>(point: Pt<S>, origin: Pt<S>, normal: Pt<S>) -> Result<Pt<S>, GeomError>
```

`GeomError::number` is a `match`, not a cast. The crate stays `no_std` outside tests. It depends on `ocelli-core` and does not depend on `wgpu` or `wasm-bindgen`.

`SegmentProjection` is `Copy` by a hand impl, for the same reason `Pt` is. The marker must not become a bound.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| Later measurement tools, F-073 and after | `distance`, `polyline_length`, `angle_at` | Points already in one space | A finite `f64`, or `GeomError` |
| Later hit tests | `hit_segment`, `project_onto_segment` | Tolerance in that space's units | `bool` or the clamped projection |
| A caller who has a plane normal | `project_onto_plane` | Normal need not be unit length | The orthogonal foot |
| F-048 `canvas_to_world` | not called from this crate | The caller converts, then measures | This crate never sees both spaces in one call |

No current render path changes.

## State transitions and invariants

Not applicable as stored session state. Each function is pure. A refusal returns `Err` and has no output struct to leave half-written. `hit_segment` does not change the segment.

The invariant that spans callers is the type parameter. `distance` of a `Pt<Canvas>` and a `Pt<World>` does not compile. Tests instantiate `World` for the fixtures and `Canvas` for one distance, so the generic is used two ways.

## Ownership, commit and recovery

Not applicable. The functions borrow their points and return a value. They allocate nothing. Repeating a call with the same points returns the same result. There is no cache.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | `(0,0,0)` to `(3,4,0)` is 5. A point to itself is 0. | `distance is euclidean` | Manhattan length 7, or a zero distance reported as degenerate | implementation |
| AC2 | Points `(0,0,0)`, `(3,4,0)`, `(3,4,12)` have length 17. One point has length 0. | `polyline length sums segments` | The chord from the first point to the last, which is 13 | implementation |
| AC3 | Arms `(1,0,0)` and `(0,1,0)` about the origin are `FRAC_PI_2` by bits. Opposite arms are `PI`. The same arm is 0. | `angle is unsigned and uses the cross and the dot` | A signed angle, or a constant `PI/2` for every input | implementation |
| AC4 | `(1,0,0)` to `(1,1,0)` about the origin is `FRAC_PI_4` by bits | `a forty-five degree arm is pi over four` | `atan2` of the dot and the cross, swapped, which is not that constant | implementation |
| AC5 | Segment `(0,0,0)` to `(4,0,0)`, point `(1,3,0)`, gives `t = 0.25`, point `(1,0,0)`, distance 3 | `projection uses t and clamps` | Constant `t = 0.5`, which lands on `(2,0,0)` | implementation |
| AC6 | Point `(-2,0,0)` clamps to `t = 0`. Point `(6,0,0)` clamps to `t = 1`. | `projection clamps to the segment` | The line is infinite and `t` is `-0.5` or `1.5` | implementation |
| AC7 | A zero-length segment is `Degenerate`. A segment of length `1e-12` still projects. | `only an exact zero segment is degenerate` | An epsilon rejects the short segment | implementation |
| AC8 | Tolerance 3 hits `(1,3,0)`. Tolerance 2 misses. Tolerance 0 hits the foot `(2,0,0)` and misses a point off the segment. | `a hit includes the tolerance boundary` | `<` instead of `<=`, so tolerance 3 misses | implementation |
| AC9 | A negative tolerance is `Frame` even when the segment is zero-length | `the tolerance sign is checked first` | `Degenerate` wins | implementation |
| AC10 | `NaN` and infinity are `Frame`. `f64::MAX` to `f64::MIN` on x is `Frame` because the difference overflows. | `non-finite inputs and overflow do not become a length` | `Ok(NaN)` or `Ok(inf)` | implementation |
| AC11 | Origin, normal `(0,0,2)`, point `(1,2,3)` projects to `(1,2,0)`. Normal `(1,1,0)` sends `(1,0,5)` to `(0.5, -0.5, 5)`. | `the plane normal is scaled by its length` | Un-normalised `p - dot(p, n) * n` | implementation |
| AC12 | A zero normal is `Degenerate`. A `NaN` normal is `Frame`. | `a zero normal is not a plane` | The point is returned unchanged | implementation |
| AC13 | `distance` on `Pt<Canvas>` of `(0,0,0)` and `(3,4,0)` is also 5 | `the same function is instantiated for canvas` | A world-only function, or a conversion hidden inside `distance` | implementation |
| AC14 | `GeomError::number` is 400 and 401 by match | `the geom numbers stay in range` | A cast, or 400 reused as `Degenerate` | implementation |

## Approach

`crates/ocelli-geom/src/measure.rs` holds the six functions. `lib.rs` re-exports them and keeps `CRATE_NAME`. Arithmetic uses `f64` in core. No `glam` call is added in this crate. `ocelli-core` already depends on `glam` for `Transform`, and this story does not need a second matrix type.

`len2` is an exact zero when its bits are the bits of `+0.0` or `-0.0`. Inequalities against `0` and `1` are the clamp. `float_cmp` denies `==` on `f64`, so the zero test does not use that operator.

Rejected alternatives:

- Implementing the formulas again in `packages/core`. Two copies drift.
- Exporting the functions through `ocelli-wasm` in this story. No command in the F-100 set is a measurement, and bindgen stays in one crate.
- Requiring a unit normal and refusing DICOM direction cosines that are not bit-exact. The plane formula normalises.
- An epsilon for "short" segments. The story would then own a tolerance the HLD did not write. Exact zero is the degeneracy. A short segment has a defined `t`.
- Calling `canvas_to_world` inside `distance`. That would hide the space change the type parameter exists to show.

## Dependencies, write ownership and resources

Depends on F-048, which is done. The source dependency is `ocelli-core`, which F-001 already built. F-048's adapters stay in `ocelli-viewport` and are not linked into this crate. This story does not wait on F-068's files.

Files: `crates/ocelli-geom/Cargo.toml`, `crates/ocelli-geom/src/lib.rs`, `crates/ocelli-geom/src/measure.rs`, `crates/ocelli-geom/tests/measure.rs`, and at completion `docs/lld/geometry.md`, `docs/lld/README.md`, and the sprint ledgers.

No shader, no GPU, no corpus row.

## Verification and evidence reuse

Focused check: `bin/ocelli.sh test ocelli-geom` and `bin/ocelli.sh clippy ocelli-geom`. Completion uses the feature profile with corpus. CPU candidate bytes do not change, so the stored comparison report stays the one S24 recorded. Do not regenerate it for this story.

The fixtures are hand-computed in the test from the local rules above. They are not copied from the function under a debugger. The `t = 0.25` case is the one a constant midpoint fails.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none
- unsafe: none
- Tier A (WebGPU): n/a. No device and no draw.
- Tier B (WebGL2): n/a. The same functions.
- Tier C (CPU): n/a. The same functions. The result does not change on CPU and does not report unavailable in place of a length.

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| fixture | AC1 to AC14, hand-computed from the local rules in this plan | `crates/ocelli-geom/tests/measure.rs` |

No DICOM section states the Euclidean length. The coordinates are chosen so the squares are integers. The angle cases cite `f64::consts::FRAC_PI_2`, `PI` and `FRAC_PI_4` by bits.

## Parity surface covered

None. No tool class is implemented. The mathematics is what those classes will call.

## Deviations

none. D-02 is already recorded and this story follows it by writing `ocelli-geom`.

## LLD impact

`docs/lld/geometry.md`, new, and its row in `docs/lld/README.md`.

## Open questions

None. The operator declined a design-question round on 2026-10-31. The missing formulas are decided above and labelled as local. The plan is approved.
