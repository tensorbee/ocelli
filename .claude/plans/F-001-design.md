# F-001, Cargo workspace, crate skeleton, lint/CI baseline

**Status**: approved
**Epic ref**: E1.1
**Sprint**: S01
**Estimate**: 2w

## What is already present, and what this story still owes

The bootstrap commit created the workspace, the thirteen crates, the lint
baseline, the gate runner and the bindgen isolation check. `bin/ocelli.sh gate
--floor` is green on this tree with four gates skipped for reasons unrelated to
this story.

`docs/sprints/CURRENT_SPRINT.md` states what remains: entries 1 and 2 of
`docs/hld/25-first-ten-files.md`.

| # | File | Why here |
|---|------|----------|
| 1 | `crates/ocelli-core/src/space.rs` | Coordinate spaces and transforms, everything downstream depends on them |
| 2 | `crates/ocelli-core/src/value.rs` | Stored, Modality and Display newtypes |

Nothing else in this story touches build tooling. The workspace manifest is
edited in exactly one place, to make `glam` usable from a `no_std` crate.

## Normative source, transcribed

### `docs/hld/13-core-types.md`, section 16, verbatim

> Cornerstone represents canvas points, world points and voxel indices all as
> number\[\]. Mixing them is a silent, common and expensive bug. Rust can make
> the mistake impossible at compile time, and this is one of the clearest
> places the language actually earns its cost.

```rust
// ocelli-core/src/space.rs
use core::marker::PhantomData;
/// CSS pixels inside a viewport element. Origin top-left, y increases down.
pub enum Canvas {}
/// DICOM patient coordinate system (LPS), millimetres.
pub enum World {}
/// Voxel indices within a volume. Origin at voxel (0,0,0).
pub enum Index {}
#[derive(Debug, PartialEq)]
pub struct Pt<S> { pub x: f64, pub y: f64, pub z: f64, _s: PhantomData<S> }
// NOTE: derive(Clone, Copy) would add an S: Clone bound that the marker
// types do not satisfy. Implement by hand.
impl<S> Clone for Pt<S> { fn clone(&self) -> Self { *self } }
impl<S> Copy for Pt<S> {}
impl<S> Pt<S> {
    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z, _s: PhantomData }
    }
}
pub struct Transform<A, B> { m: glam::DMat4, _p: PhantomData<(A, B)> }
impl<A, B> Transform<A, B> {
    pub fn apply(&self, p: Pt<A>) -> Pt<B> { /* ... */ }
    pub fn inverse(&self) -> Transform<B, A> { /* ... */ }
    pub fn then<C>(&self, next: &Transform<B, C>) -> Transform<A, C> { /* ... */ }
}
```

> *The payoff: Transform\<Canvas, World\> composes with Transform\<World,
> Index\> and will not compose with anything else. A whole class of tool bugs
> stops compiling.*

### `docs/hld/13-core-types.md`, section 16.1, verbatim

> The same trick applies to pixel values, and for the same reason - the LUT
> chain is a sequence of transformations whose stages are easy to apply out of
> order.

```rust
// ocelli-core/src/value.rs
#[derive(Clone, Copy, Debug)] pub struct Stored(pub f32); // raw from pixel data
#[derive(Clone, Copy, Debug)] pub struct Modality(pub f32); // after rescale; HU for CT
#[derive(Clone, Copy, Debug)] pub struct Display(pub f32); // after VOI, in [ymin, ymax]
```

> *You cannot accidentally window a stored value, and a reviewer can see the
> stage from the type.*

### `docs/hld/22-testing-and-tolerance.md`, section 25, verbatim

```rust
proptest! {
    #[test]
    fn canvas_world_roundtrip(x in -1e4f64..1e4, y in -1e4f64..1e4) {
        let p = Pt::<Canvas>::new(x, y, 0.0);
        let t = viewport.canvas_to_world();
        let back = t.inverse().apply(t.apply(p));
        prop_assert!((back.x - p.x).abs() < 1e-6);
    }
}
```

### `docs/hld/22-testing-and-tolerance.md`, section 25.1, the geometry row, verbatim

> - **Geometry:** world coordinates within 1e-6 mm, canvas coordinates within
>   a quarter pixel.

### `docs/hld/12-workspace-and-build.md`, section 15.2, the line this story edits

```toml
glam = "0.30"
```

### DICOM PS3.3 C.7.6.2.1.1, the geometry the fixture is computed from

With `X = IOP[0..3]` the row direction cosine, `Y = IOP[3..6]` the column
direction cosine, `i` the **column** index and `j` the **row** index:

```
P = IPP + i * PixelSpacing[1] * X + j * PixelSpacing[0] * Y
```

`PixelSpacing` is `[between rows, between columns]`, so `PixelSpacing[0]`
multiplies the **column** direction cosine. `ImagePositionPatient` is the
centre of the first voxel, not its corner. The patient coordinate system is
LPS.

## What the specification does not cover

Six things, and each one is a decision this plan makes rather than finds.

1. **`#[derive(Debug, PartialEq)]` on `Pt<S>` does not compile at the use
   site.** The derive expands to `impl<S: Debug> Debug for Pt<S>`, so
   `Pt<Canvas>: Debug` requires `Canvas: Debug`, and `pub enum Canvas {}` as
   the HLD writes it implements neither `Debug` nor `PartialEq`. This is the
   same trap the HLD's own note identifies for `Clone` and `Copy`, applied to
   two traits the note does not mention. Verified against rustc, not reasoned
   about: `assert_eq!` on two `Pt<Canvas>` fails with E0369 and E0277.
   **Decision**: derive `Debug, Clone, Copy, PartialEq, Eq, Hash` on the three
   marker enums, which leaves the HLD's `Pt` block character for character as
   written. Raised as **D-08**.
2. **`Transform` has no constructor.** The HLD gives three methods and a
   private field. **Decision**: `Transform::from_mat4(glam::DMat4)` and
   `Transform::identity()`, nothing else, because nothing else has a caller
   today. A viewport camera constructor belongs to F-023, not here.
3. **Whether `apply` divides by w.** The parity surface lists a `PERSPECTIVE`
   viewport type, so a `Transform<World, Canvas>` will not always be affine.
   **Decision**: `glam::DMat4::project_point3`, which transforms the point as
   `(x, y, z, 1)` and divides by the resulting w. For an affine matrix w is
   exactly `1.0` and division by `1.0` is exact in IEEE 754, so this is the
   affine answer as well as the perspective one. `transform_point3` would be
   the affine-only choice and would be silently wrong under perspective, which
   is the failure mode this project exists to avoid.
4. **The composition order of `then`.** `a.then(b)` reads as "apply `a`, then
   `b`", so the matrix is `b.m * a.m`. The HLD's type signature does not pin
   the order and the reversed version type-checks identically. A `unit` test
   asserts the order against a hand-worked non-commuting pair.
5. **What `inverse` does for a singular transform.** `glam::DMat4::inverse`
   returns a matrix of non-finite values rather than failing. **Decision**:
   keep the HLD's signature, return the same thing, and do not invent a
   checked variant with no caller. Recorded here so the next reader does not
   have to discover it: the constructor that guarantees invertibility belongs
   to whoever builds a camera, and F-023 must not assume this one checked.
6. **`glam` in a `no_std` crate.** Every core crate carries
   `#![cfg_attr(not(test), no_std)]` from the bootstrap, which the HLD neither
   requires nor forbids. `glam = "0.30"` enables the `std` feature by default
   and glam needs either `std` or its optional `libm` dependency to compile at
   all. **Decision**: the workspace entry becomes `glam = { version = "0.30",
   default-features = false, features = ["libm"] }`. Raised as **D-09**.

Nothing else in this story is undetermined by the HLD.

## Approach

### `crates/ocelli-core/src/space.rs`

The HLD listing, transcribed, with the marker enums carrying derives (D-08) and
the three elided method bodies filled in:

- `apply` is `Pt::new` over `self.m.project_point3(DVec3::new(p.x, p.y, p.z))`.
- `inverse` is `Transform::from_mat4(self.m.inverse())`.
- `then` is `Transform::from_mat4(next.m * self.m)`.

`Clone` and `Copy` are hand-implemented on `Transform` too, for the reason the
HLD's note gives for `Pt`. `Debug` is derived on both, which works because the
markers derive it.

No `as` cast appears in this file. `Pt` is `f64` throughout and `glam::DVec3`
is `f64` throughout, so there is no conversion to review.

### `crates/ocelli-core/src/value.rs`

The three newtypes exactly as the HLD gives them, `f32`, tuple structs, public
field. No arithmetic and no conversions between them: `Stored` becomes
`Modality` in `ocelli-pixel` under the modality LUT, and inventing a `From`
here would be the second copy of an arithmetic stage that HLD section 18
requires to exist exactly once.

### `crates/ocelli-core/src/lib.rs`

Declare both modules and re-export their public items at the crate root. Keep
the existing `CRATE_NAME` scaffold constant and its test.

### `Cargo.toml`

Two edits. The `glam` workspace entry gains `default-features = false,
features = ["libm"]` (D-09), and `proptest` is added to
`[workspace.dependencies]` so the round-trip property test the HLD's section 25
specifies has a dependency to inherit. `proptest` is a dev-dependency of
`ocelli-core`, never a normal one.

### Lints this file has to live under

`float_cmp` is denied workspace-wide. The tests therefore compare with an
explicit tolerance rather than with `assert_eq!` on an `f64`, which is what
section 25.1 asks for anyway. `unwrap_used`, `expect_used` and `panic` are
denied, so no test uses them.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none. `Pt` and `Transform` are `Copy` and live on the
  stack, and no method in this story allocates.
- unsafe: none
- Tier A (WebGPU): n/a. This is CPU-side type and arithmetic code with no
  rendering path.
- Tier B (WebGL2): n/a, same reason.
- Tier C (CPU): n/a, same reason. Tier C consumes these types like every other
  tier does, it does not need a variant of them.

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| `unit` | `Pt::new` is `const` and stores its arguments unchanged | `crates/ocelli-core/src/space.rs` under `#[cfg(test)]` |
| `unit` | `then` composes in the stated order, against a hand-worked non-commuting pair of a translation and a scale | `crates/ocelli-core/src/space.rs` under `#[cfg(test)]` |
| `unit` | `identity().apply(p) == p` for all three spaces | `crates/ocelli-core/src/space.rs` under `#[cfg(test)]` |
| `unit` | The three value newtypes carry their field unchanged and are `Copy` | `crates/ocelli-core/src/value.rs` under `#[cfg(test)]` |
| `fixture` | `Transform<Index, World>` built from a hand-written IPP, IOP and non-square `PixelSpacing` maps four named voxel indices to patient coordinates computed by hand from **DICOM PS3.3 C.7.6.2.1.1**, within 1e-6 mm | `crates/ocelli-core/tests/geometry_ps3_3_c7_6_2.rs` |
| `property` | Round trip `Pt<Canvas>` to `World` and back is within 1e-6, per HLD 25.1, over the HLD's own `-1e4..1e4` range | `crates/ocelli-core/tests/roundtrip.rs` |
| `compile-fail` | `Transform<Canvas, World>::apply` refuses a `Pt<Index>`, and `Transform<Canvas, World>::then` refuses a `Transform<Index, World>` | `crates/ocelli-core/tests/` with `trybuild` |

### Why the fixture is not optional here

The fixture is the only test in this story that would go red if the arithmetic
were wrong rather than merely inconsistent. The specific defect it is built to
catch is the transposed spacing index: `PixelSpacing[0]` multiplies the
**column** direction cosine, and a fixture with square pixels cannot tell the
two apart. The chosen case therefore uses `PixelSpacing = [0.5, 0.25]` and an
oblique `ImageOrientationPatient`, so a transposition moves the answer by more
than the tolerance and an axis swap moves it further.

Expected values are computed in Python from the PS3.3 formula, before the Rust
is written, and the computation is recorded in the test file's header comment
so a reviewer can redo it without rerunning anything.

### The mutation check

`docs/hld/24-agent-code-standards.md` 27.3 requires that a new test would
actually fail if the code were wrong. For this story that means, one at a time
and reverted after each: swap `PixelSpacing[0]` and `PixelSpacing[1]` in
`apply`'s caller and confirm the fixture goes red, reverse the operand order in
`then` and confirm the composition test goes red, and replace `project_point3`
with `transform_point3` under a perspective matrix and confirm the round-trip
property goes red.

## Parity surface covered

None. `docs/hld/B-parity-surface.md` counts surfaces of cornerstone3D and
carries no row a build and core-types story covers.

## Deviations

Two, both new, both added to `docs/hld/DEVIATIONS.md` in the design commit.

- **D-08**, the marker enums derive `Debug, Clone, Copy, PartialEq, Eq, Hash`
  where HLD section 16 declares them bare, because the HLD's own
  `#[derive(Debug, PartialEq)]` on `Pt<S>` is otherwise unusable at every call
  site.
- **D-09**, the workspace `glam` entry disables default features and enables
  `libm`, because the core crates are `no_std` and glam's default `std`
  feature would defeat that.

Existing deviations relied on: D-01 (the workspace manifest already differs on
`rust-version` and `resolver`).

## LLD impact

`docs/lld/core-types.md`, created by `/complete-feature` step 9: the coordinate
and value spaces, the composition order of `then`, the perspective decision in
`apply`, and the singular-inverse behaviour that F-023 must not assume away.

## Open questions

None that block implementation. The six items under **What the specification
does not cover** are decided in this plan rather than deferred, and the two
that change tracked artefacts are raised as D-08 and D-09.
