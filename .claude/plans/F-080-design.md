# F-080, brush with sphere and circle geometry

**Status**: approved
**Epic ref**: E13.1
**Sprint**: S30
**Estimate**: 4w

## Normative source, transcribed

`docs/hld/B-parity-surface.md` Appendix B counts the classes. It does not
define a brush. The quotation is the whole of the relevant rows.

```text
| Tool classes | ~63 | 26 annotation, 12 segmentation, 25 manipulation and utility |
| Segmentation representations | 3 | Labelmap, contour, surface |
```

`docs/hld/03-architecture-and-crates.md` names `ocelli-seg` as the
segmentation crate. It does not give a stamp function.

The label grid this stamp writes is the one F-063 already built. That story
reads DICOM PS3.3 C.8.20.2 as follows, and this story does not reopen it.
A segmentation pixel is a segment number. Pixel value 0 is background and
is not a segment number. The colour table is addressed from segment 1.

## What the specification does not cover

The HLD does not say which samples a circle or a sphere includes. This
plan decides that, and it is not an HLD formula.

A sample at integer column `x`, row `y` and slice `z` is inside when the
squared distance from the centre is less than or equal to the squared
radius. The centre and the radius are caller `f64` values in index units.
The caller owns millimetres and spacing. This function does not read them.

A circle uses `x` and `y` only, and only on the slice whose index is the
same number as `center_z`, including both zero bit patterns. Any other
slice is left untouched. A non-integer `center_z` selects no slice.

A sphere uses `x`, `y` and `z` on every supplied slice.

Radius 0 includes only a sample whose index is exactly the centre. A
negative radius is refused. Both zero bit patterns are a zero radius, not
a negative radius.

`covered` counts samples inside the brush. `changed` counts those whose
stored segment number differs from the written value. A second stamp of
the same value keeps `covered` and sets `changed` to 0.

Segment 0 is a legal written value. It erases. The presenter still treats
0 as background.

A refusal is `Frame` for a non-finite centre or radius, and `Degenerate`
for a negative radius, an empty stack, a zero side, a length that is not
`width * height`, or slices that do not share one width and height. A
refusal writes nothing.

There is no TypeScript copy. The segment numbers stay in `ocelli-seg`.
Pixels do not cross the boundary. The existing presenter is not edited.

## Problem and reproducer

No function writes a brush into `LabelGrid`. A 3 by 3 grid stamped at
centre `(1, 1, 0)` with radius 1 and segment 2 must become the plus below
and must not paint the corners. Today the grid has no stamp entry point.

```text
0 2 0
2 2 2
0 2 0
```

## Scope, API and compatibility

In scope: `stamp_brush` in `ocelli-seg`.

Out of scope: scissors, flood fill, contour editing, interpolation, a GPU
brush, a new presenter, the representation enum, and the SR content tree.

```rust
pub enum BrushShape {
    Circle,
    Sphere,
}

pub struct Brush {
    pub shape: BrushShape,
    pub center_x: f64,
    pub center_y: f64,
    pub center_z: f64,
    pub radius: f64,
    pub segment: u16,
}

pub struct BrushCoverage {
    pub covered: u32,
    pub changed: u32,
}

pub enum BrushError {
    Frame,
    Degenerate,
}

pub fn stamp_brush(grids: &mut [LabelGrid], brush: Brush) -> Result<BrushCoverage, BrushError>
```

`present_labelmap`, `intersect_contour` and `present_surface` stay as they
are. `LabelGrid` fields stay as they are.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| `stamp_brush` | `LabelGrid::indices` | caller owns spacing and the segment number | segment numbers replaced inside the brush |
| existing `present_labelmap` | unchanged | fill and outline stay the F-063 rules | RGBA8 of the grid after the stamp |

## State transitions and invariants

| Entry point / starting state | Input or event | Resulting state and output | Refusal and preserved state |
|------------------------------|----------------|----------------------------|-----------------------------|
| any grids | finite centre, radius >= 0, matching slices | indices inside the brush become `segment` | none |
| any grids | non-finite centre or radius | `Frame` | every index unchanged |
| any grids | negative radius, empty stack, bad length, mismatched slices | `Degenerate` | every index unchanged |
| already stamped | the same brush again | `changed` is 0 | indices stay |

Validation finishes before the first write. The staged indices replace the
grids only after every offset resolves.

## Ownership, commit and recovery

The caller owns the grids. The function allocates one staged copy per
slice and assigns it back only after the stamp is complete. A refusal
does not assign. A repeated call with the same arguments writes the same
numbers and reports `changed` 0.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | 3x3 circle, centre (1, 1, 0), radius 1, segment 2 | plus of five 2s, corners stay 0, covered 5, changed 5 | a corner painted, or an edge pixel left 0 | `crates/ocelli-seg/tests/brush.rs` |
| AC2 | two 3x3 slices, sphere, same centre and radius, segment 1 | slice 0 is the plus, slice 1 is only the centre | the next slice left blank, or a corner of slice 1 painted | same test file |
| AC3 | negative radius, and a non-finite radius, on a grid of 9s | `Degenerate`, then `Frame` | any 9 replaced | same test file |
| AC4 | 3x3 declared with 8 indices | `Degenerate` | a short buffer written | same test file |
| AC5 | radius 0 at (1, 1, 0), and radius 0 at (1.5, 1, 0) | only the integer centre, then covered 0 | a neighbour painted | same test file |
| AC6 | circle at z = 1 on two slices, and circle at z = -0 on slice 0 | only slice 1, then slice 0 | every slice painted | same test file |
| AC7 | erase the centre of a grid of 1s, then stamp that centre again | changed 1, then changed 0 | erase reported as a miss | same test file |
| AC8 | a 3x3 slice beside a 2x2 slice | `Degenerate` | one slice written | same test file |
| AC9 | present the AC1 grid after a segment 1 stamp, fill on, outline off | centre bytes are the LUT colour, corner alpha is 0 | presenter formula edited | same test file |

## Approach

Add `crates/ocelli-seg/src/brush.rs` and export it from `lib.rs`. Compare
squared distances with `total_cmp`. Do not call `sqrt`. Do not add glam.
Do not edit `labelmap.rs`, `contour.rs` or `surface.rs`.

## Dependencies, write ownership and resources

Depends on F-063, which is done. This story writes `brush.rs`, `lib.rs`,
`tests/brush.rs`, the plan, the LLD paragraph and the delivery rows. It
does not write a shader and it does not take the GPU.

## Verification and evidence reuse

Focused check: `bin/ocelli.sh test ocelli-seg --test brush`. Then clippy
on `ocelli-seg`. Completion uses the feature profile, 27 gates, with
corpus pass and no comparison report. The sprint profile is a later
commit. CPU candidate bytes stay as they are.

Mutate the comparison from `is_le` to `is_lt` and confirm AC1 goes red,
because the four edge pixels sit on the radius. Restore it.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none
- unsafe: none
- Tier A (WebGPU): n/a
- Tier B (WebGL2): n/a
- Tier C (CPU): n/a

The stamp does not open a device. The presenter that draws the grid is
unchanged and already has its own tier rows.

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| fixture | AC1 through AC9, hand-computed in this plan. PS3.3 C.8.20.2 is the background rule only. The radius test is this plan. | `crates/ocelli-seg/tests/brush.rs` |

## LLD impact

- `docs/lld/public-api.md`
- the `public-api.md` row in `docs/lld/README.md`
