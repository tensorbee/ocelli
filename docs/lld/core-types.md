# Core types, coordinate spaces and value spaces

**Area**: `crates/ocelli-core`
**Normative source**: `docs/hld/13-core-types.md` sections 16 and 16.1
**F-IDs that contributed:** F-001
**Last updated:** 2026-09-04

Living current-state document. It describes what the code does today.

## What is here

`ocelli-core` holds two modules, both re-exported at the crate root so callers
write `ocelli_core::Pt` rather than `ocelli_core::space::Pt`.

| Module | Contents |
|--------|----------|
| `space` | `Canvas`, `World`, `Index`, `Pt<S>`, `Transform<A, B>` |
| `value` | `Stored`, `Modality`, `Display` |

The crate is `#![cfg_attr(not(test), no_std)]` and has one dependency, `glam`,
for `DMat4`. No `unsafe`, no `wasm-bindgen`, no allocation, no I/O.

## Coordinate spaces

Three uninhabited marker enums name the spaces. `Pt<S>` carries three `f64`
components and a `PhantomData<S>`, so a canvas point and a world point are
different types and neither can be passed where the other is expected.

| Space | Units | `x`, `y`, `z` |
|-------|-------|---------------|
| `Canvas` | CSS pixels inside a viewport element | origin top left, y increases down |
| `World` | millimetres, DICOM patient coordinates, LPS | patient axes |
| `Index` | voxels | column index `i`, row index `j`, slice index |

`Pt::new` is `const`, so a point can be a constant.

`Index` orders `i` before `j` because DICOM PS3.3 C.7.6.2.1.1 does. The slice
index in `z` is this crate's own extension. C.7.6.2.1.1's column vector is
`(i, j, 0, 1)` with a literal zero, because the equation is in-plane and says
nothing about stepping between slices.

`Clone` and `Copy` on `Pt<S>` and on `Transform<A, B>` are hand-written rather
than derived. The reason is not the one HLD section 16's `NOTE` gives, which
stopped being true when D-08 gave the markers their derives. It is that the
hand-written impls are unconditional: `impl<S> Copy for Pt<S>` holds whatever a
future marker does or does not derive, where `#[derive(Copy)]` would make a
point's `Copy`-ness contingent on a marker nobody ever constructs a value of.

## `Transform<A, B>`

Two constructors and three methods. The matrix is a full 4x4 `glam::DMat4`,
not an affine 3x4, because a PERSPECTIVE viewport produces a projective
transform.

| Item | Behaviour |
|------|-----------|
| `from_mat4(DMat4)` | The caller owns the meaning of the matrix |
| `identity()` | On `impl<S> Transform<S, S>` only |
| `apply(Pt<A>) -> Pt<B>` | `project_point3`, which divides by the resulting `w` |
| `inverse() -> Transform<B, A>` | `self.m.inverse()`, unchecked |
| `then(&Transform<B, C>) -> Transform<A, C>` | `next.m * self.m` |

### `apply` divides by `w`, and that choice is free

`apply` uses `glam::DMat4::project_point3`, which transforms the point as
`(x, y, z, 1)` and divides the result by its `w`. The affine-only alternative
is `transform_point3`, which ignores the bottom row entirely.

The parity surface lists a PERSPECTIVE viewport type, so a
`Transform<World, Canvas>` will not always be affine, and under a projective
matrix `transform_point3` is silently wrong rather than loudly wrong. That is
the defect class this project exists to avoid.

**The choice costs nothing on an affine matrix.** For an affine transform `w`
is exactly `1.0`, and glam's `project_point3` performs a real per-component
division rather than a multiply by a reciprocal, so dividing by `1.0` is exact
in IEEE 754 and the result is bit identical to the affine form. There is no
accuracy argument and no performance argument for the narrower call.

`apply_divides_by_the_resulting_w` pins this with a hand-worked projective
case, and both perspective round trips in `tests/roundtrip.rs` go red if the
divide is dropped. The affine round trip and the whole geometry fixture stay
green under that change, correctly, which is why the perspective cases exist.

### `then` applies the receiver first

`a.then(b)` means apply `a`, then `b`, so the matrix is `b.m * a.m` and the
result carries `a`'s source space and `b`'s destination space. The reversed
composition type-checks identically and compiles, so the order is pinned by
tests against a non-commuting pair, a translation against a scale, which give
22 one way and 12 the other. Both orders have their own test.

### `inverse` does not check invertibility, and F-023 must not assume it does

`inverse` calls `glam::DMat4::inverse` and returns whatever comes back. For a
singular matrix that is non-finite components, not an error and not a panic.
Nothing in this crate checks the determinant first.

This is deliberate. A constructor that guarantees an invertible matrix belongs
to whoever builds a camera, and no caller today would have anything useful to
do with an error return. **Whoever writes that camera must not assume this
method checked.**

The case is realistic rather than theoretical: the PS3.3 C.7.6.2.1.1
image-plane matrix has a zero third column, so a single-slice
`Transform<Index, World>` is singular by construction.
`inverse_does_not_check_invertibility` executes this against exactly that
matrix, so the behaviour is a test rather than folklore, and a glam release
inside the `0.30` caret range that changed it would turn that test red.

One qualification. glam's `inverse` carries a `glam_assert!` on the
determinant, which expands to `assert!` under
`any(all(debug_assertions, feature = "debug-glam-assert"), feature = "glam-assert")`
and to nothing otherwise. Neither feature is enabled in this workspace, so the
non-finite behaviour above is what this workspace gets. If feature unification
ever turns one on, the path becomes a panicking one, which matters because a
panic poisons a wasm instance.

### `identity` is `Transform<S, S>`, not `Transform<A, B>`

On the wider impl, `Transform::<Canvas, World>::identity()` would compile and
turn a canvas point into a world point with no arithmetic, no cast and nothing
in a diff for a reviewer to stop on. Its own name would assert that no
conversion was needed. That is the `number[]` interchange section 16 exists to
prevent, in the form a caller reaches for when the real transform does not
exist yet.

`from_mat4` still accepts any matrix, so the narrowing closes no hole. It moves
the cost: `from_mat4(DMat4::IDENTITY)` makes the caller name a matrix, which is
a deliberate act that appears in a diff.

## Value spaces

`Stored`, `Modality` and `Display` are three distinct `#[derive(Clone, Copy,
Debug)]` newtypes over `f32` with public fields.

| Type | Stage |
|------|-------|
| `Stored` | raw from Pixel Data (7FE0,0010), after unpacking and signed interpretation |
| `Modality` | after the modality LUT, Hounsfield units for CT |
| `Display` | after the VOI LUT, in the output range `[ymin, ymax]` |

**There is deliberately no conversion between them.** No `From`, no `Into`, no
constructor that takes another of the three. `Stored` becomes `Modality` under
the modality LUT and `Modality` becomes `Display` under the VOI LUT, and HLD
section 18 requires that arithmetic to exist exactly once, in `ocelli-pixel`.
A `From` here would be a second copy of a LUT stage, which is the same defect
as a second copy anywhere else except that it would look like a convenience.

`value.rs` therefore contains no arithmetic at all. What it buys is that you
cannot accidentally window a stored value, and that a reviewer can see the
stage from the type.

## Tolerances used by this area's tests

HLD section 25.1's geometry bullet has two clauses, and they are easy to reach
for in the wrong order.

| Quantity | Bound | Authority |
|----------|-------|-----------|
| World coordinate difference | 1e-6 mm | 25.1's world clause, exactly |
| Canvas coordinate difference | 1e-6 CSS pixels | Section 25's listing. 25.1's canvas clause is a quarter pixel, so this is far tighter, deliberately |
| Voxel index difference | 1e-6 voxels | 25.1 has no clause for it |
| Dimensionless, such as a determinant or a dot product | 1e-6 | No unit, so it borrows none |

Every case in this area is exact arithmetic that should land on the nose rather
than a rendered result allowed to drift, so 1e-6 is a strictness floor and not
a tolerance to relax toward whichever 25.1 clause a reader finds first.
Widening any of them is a design-plan decision reviewed like code.

## What is guarded at compile time

Four `trybuild` cases under `crates/ocelli-core/tests/ui/`. They exist because
the claims they check are not observable from a test that runs.

| Case | Refuses |
|------|---------|
| `apply_refuses_a_point_from_another_space` | `Transform<Canvas, World>::apply` on a `Pt<Index>` |
| `then_refuses_a_transform_that_does_not_join` | Composing `Transform<Canvas, World>` with `Transform<Index, World>` |
| `identity_refuses_to_cast_between_spaces` | `Transform::<Canvas, World>::identity()` |
| `value_spaces_do_not_interconvert` | `let _: Modality = Stored(1.0)` |

Anyone adding a case adds its row here and in the design plan's test table in
the same change.

## Deviations this area rests on

**D-08.** The three marker enums derive `Debug, Clone, Copy, PartialEq, Eq,
Hash` where HLD section 16 writes them bare. Without it, section 16's own
`#[derive(Debug, PartialEq)]` on `Pt<S>` bounds `S`, and `Pt<Canvas>` satisfies
neither trait, so `assert_eq!` on two points fails to compile.

The deviation is held in place by `d_08_keeps_the_marker_derives_load_bearing`,
which reaches all six derives: `Debug` and `PartialEq` through a `{:?}` format
and an equality, and `Clone`, `Copy`, `Eq` and `Hash` through a bound assertion
that nothing else in the crate would need. Deleting any one of the six stops
the crate compiling. That test is the only thing standing between this
deviation and a silent revert, so do not delete it to make a build green.

**D-09.** The workspace `glam` entry disables default features and enables
`libm`, because the default `std` feature would defeat the `no_std` posture the
core crates declare. Note for whoever tightens this: **nothing in the
repository currently fails if D-09 is reverted.** `wasm32-unknown-unknown`
ships a `std` implementation, and a `no_std` crate may depend on a `std` crate
without error, so neither the floor gate nor a wasm32 build notices. The
cheapest close is an assertion on the resolved feature set.

## What this area does not have

No trait, no `Box<dyn>`, no feature flag, no wrapper that only forwards. No
rendering, so tiers A, B and C are all not applicable. Nothing crosses the
wasm boundary from here.
