# F-081, rectangle, circle and sphere scissors

**Status**: approved
**Epic ref**: E13.2
**Sprint**: S31
**Estimate**: 4w

## Normative source, transcribed

`docs/hld/B-parity-surface.md` Appendix B counts the classes. It does not
define a scissor. The quotation is the whole of the relevant rows.

```text
| Tool classes | ~63 | 26 annotation, 12 segmentation, 25 manipulation and utility |
| Segmentation representations | 3 | Labelmap, contour, surface |
```

`docs/hld/03-architecture-and-crates.md` names `ocelli-seg`. It does not
give a scissor function.

The label grid this tool writes is the one F-063 already built. That story
reads DICOM PS3.3 C.8.20.2 as follows, and this story does not reopen it.
A segmentation pixel is a segment number. Pixel value 0 is background and
is not a segment number.

F-080 already stamps a circle and a sphere. Its comparison is squared
distance with `total_cmp` `is_le`, and both zero bit patterns are one
slice. This story calls that stamp for the circle and the sphere. It does
not replace the comparison.

## What the specification does not cover

The HLD does not say which samples a scissor removes. This plan decides
that, and it is not an HLD formula.

A scissor writes segment 0 and nothing else. It erases. It cannot paint a
named segment.

A circle scissor and a sphere scissor call `stamp_brush` with segment 0
and the caller's centre and radius. The circle uses one slice. The sphere
uses every slice. Radius 0, a negative radius, a non-finite value, and the
stack checks stay the F-080 rules because that function still owns them.

A rectangle scissor uses inclusive index bounds on the one slice whose
index is the same number as `z`, including both zero bit patterns. A
non-integer `z` selects no slice and returns covered 0. A sample column
`x` is inside when `f64::from(x)` is at or above `min_x` and at or below
`max_x`. The same test applies to the row. Equality uses the F-080
same-number rule, so both zero bit patterns match. `min` above `max` by
`total_cmp`, and not the same number, is `Degenerate`.

A refusal writes nothing. `Frame` is a non-finite bound, centre, radius
or `z`. `Degenerate` is a negative radius, an inverted span, an empty
stack, a zero side, a length that is not `width * height`, or slices that
do not share one width and height.

`covered` counts samples inside the scissor. `changed` counts those whose
stored number was not already 0. Counts use `saturating_add`, the same
policy as `stamp_brush`.

There is no TypeScript copy. The segment numbers stay in `ocelli-seg`.

`stamp_brush` may gain `pub(crate)` helpers for the stack check, the
same-number test, the negative-radius test, and the disk and ball tests.
Those helpers are the existing expressions. The stamp's `is_le` comparison
and its negative-zero clause stay. This story does not change a brush
result.

## Problem and reproducer

No function erases a rectangle, a circle or a sphere to background. A 3 by
3 grid of segment 1, cut by a rectangle from `(0, 0)` to `(1, 1)` on slice
0, must become the grid below. Today there is no scissor entry point.

```text
0 0 1
0 0 1
1 1 1
```

## Scope, API and compatibility

In scope: `cut_scissor` in `ocelli-seg`.

Out of scope: flood fill, a threshold, a polygon edit, interpolation, a
GPU scissor, a new presenter, the representation enum, and the SR content
tree. `stamp_brush` still accepts a non-zero segment. Only the scissor
forces 0.

```rust
pub enum ScissorShape {
    Rectangle { min_x: f64, min_y: f64, max_x: f64, max_y: f64, z: f64 },
    Circle { center_x: f64, center_y: f64, center_z: f64, radius: f64 },
    Sphere { center_x: f64, center_y: f64, center_z: f64, radius: f64 },
}

pub struct ScissorCoverage {
    pub covered: u32,
    pub changed: u32,
}

pub enum ScissorError {
    Frame,
    Degenerate,
}

pub fn cut_scissor(
    grids: &mut [LabelGrid],
    shape: ScissorShape,
) -> Result<ScissorCoverage, ScissorError>
```

Do not derive `PartialEq` on `ScissorShape`. It holds `f64`.

`present_labelmap` stays as it is. `LabelGrid` fields stay as they are.
`brush.rs` keeps `stamp_brush`'s public signature.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| `cut_scissor` rectangle | staged `LabelGrid::indices` | caller owns spacing and the bounds | samples inside the span become 0 |
| `cut_scissor` circle or sphere | `stamp_brush` with segment 0 | F-080 distance rule | samples inside the radius become 0 |
| existing `present_labelmap` | unchanged | fill and outline stay the F-063 rules | RGBA8 of the grid after the cut |
| existing `stamp_brush` | unchanged public result | a non-zero segment still stamps | the F-080 fixtures still pass |

## State transitions and invariants

| Entry point / starting state | Input or event | Resulting state and output | Refusal and preserved state |
|------------------------------|----------------|----------------------------|-----------------------------|
| matching slices | finite rectangle, min at or below max | indices inside the span on that slice become 0 | other slices unchanged |
| matching slices | circle or sphere, finite centre, radius at or above 0 | `stamp_brush` writes 0 | F-080 refusal mapping |
| any grids | non-finite bound, centre, radius or z | `Frame` | every index unchanged |
| any grids | inverted span, negative radius, empty stack, bad length | `Degenerate` | every index unchanged |
| already 0 inside the shape | the same scissor again | `changed` is 0 | indices stay |
| circle or rectangle | `z` or `center_z` is not a slice index | `Ok` with covered 0 | every index unchanged |

Validation finishes before the first write. The rectangle stages one copy
per slice and assigns it back only after every offset resolves. The circle
and the sphere commit only inside `stamp_brush`, which already stages.

## Ownership, commit and recovery

The caller owns the grids. A refusal does not assign. A repeated call with
the same arguments writes 0 again and reports `changed` 0 when the samples
are already 0.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | 3x3 of 1s, rectangle (0, 0) through (1, 1), z 0 | four zeros at the top left, five 1s remain, covered 4, changed 4 | the far edge included, or (1, 1) left as 1 | `crates/ocelli-seg/tests/scissors.rs` |
| AC2 | the AC1 grid, circle centre (1, 1, 0), radius 1 | the plus becomes 0, corners stay 1, covered 5, changed 5 | a corner erased, or an edge left as 1 | same test file |
| AC3 | two 3x3 slices of 1s, sphere centre (1, 1, 0), radius 1 | slice 0 is the plus of zeros, slice 1 centre only | the next slice left full, or a corner of slice 1 erased | same test file |
| AC4 | rectangle with max_x below min_x, and a circle with radius -1, on a grid of 1s | `Degenerate` both times | any 1 replaced | same test file |
| AC5 | a non-finite z, and a 3x3 declared with 8 indices | `Frame`, then `Degenerate` | a short buffer written | same test file |
| AC6 | rectangle z = -0 on slice 0, and rectangle z = 0.5 | the span is erased, then covered 0 | slice 0 skipped for -0, or a fractional z writes | same test file |
| AC7 | erase a plus that is already 0 | covered 5, changed 0 | a second cut reported as a change | same test file |
| AC8 | a 3x3 slice beside a 2x2 slice | `Degenerate` | one slice written | same test file |
| AC9 | present the AC2 grid, fill on, outline off, colour of segment 1 | a corner that stayed 1 is the LUT colour, a cut centre is alpha 0 | presenter formula edited | same test file |

Hand-computed AC2. Squared distances from (1, 1): corners 2, edges 1,
centre 0. Radius squared is 1. Edges and the centre are inside. Corners
are not. That is the F-080 circle, writing 0.

## Approach

Add `crates/ocelli-seg/src/scissors.rs` and export it from `lib.rs`.
Circle and sphere build a `Brush` with segment 0 and call `stamp_brush`.
Map `BrushError::Frame` and `BrushError::Degenerate` onto `ScissorError`.
Rectangle walks the selected slice with the span test. Do not call `sqrt`.
Do not add glam. Do not edit `labelmap.rs`, `contour.rs` or `surface.rs`.

Move the existing stack check, same-number test, negative test, and disk
and ball tests to `pub(crate)` functions in `brush.rs` without changing
their expressions. `stamp_brush` calls them.

## Dependencies, write ownership and resources

Depends on F-080, which is done. This story writes `scissors.rs`,
`brush.rs` helpers, `lib.rs`, `tests/scissors.rs`, the plan, the LLD
paragraph and the delivery rows. It does not write a shader and it does
not take the GPU. The existing `tests/brush.rs` fixtures stay, and they
are re-run to show the helper move did not change a stamp.

## Verification and evidence reuse

Focused check: `bin/ocelli.sh test ocelli-seg --test scissors --test brush`.
Then clippy on `ocelli-seg`. Completion uses the feature profile, 27
gates, with corpus pass and no comparison report. The sprint profile is a
later commit. CPU candidate bytes stay as they are.

Mutate the rectangle's upper comparison so `max_x` is excluded, and
confirm AC1 goes red on the sample at x = 1. Restore it.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none
- unsafe: none
- Tier A (WebGPU): n/a
- Tier B (WebGL2): n/a
- Tier C (CPU): n/a

The cut does not open a device. The presenter that draws the grid is
unchanged and already has its own tier rows.

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| fixture | AC1 through AC9, hand-computed in this plan. PS3.3 C.8.20.2 is the background rule only. The span and the reused radius test are this plan. | `crates/ocelli-seg/tests/scissors.rs` |

## LLD impact

- `docs/lld/public-api.md`
- the `public-api.md` row in `docs/lld/README.md`

## Parity surface covered

Appendix B counts 12 segmentation classes and does not name a scissor row.
This story adds the rectangle, circle and sphere erase. It does not
establish a rendered parity view.

## Deviations

None. The HLD does not define this tool, so the span rule is not a
departure from a written formula.

## Open questions

None. The operator declined a design-question round. The gaps above are
decided in this plan.
