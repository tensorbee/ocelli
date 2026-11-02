# F-083, rectangle and circle ROI threshold

**Status**: approved
**Epic ref**: E13.4
**Sprint**: S31
**Estimate**: 4w

## Normative source, transcribed

`docs/hld/B-parity-surface.md` Appendix B counts the classes. It does not
define a threshold. The quotation is the whole of the relevant rows.

```text
| Tool classes | ~63 | 26 annotation, 12 segmentation, 25 manipulation and utility |
| Segmentation representations | 3 | Labelmap, contour, surface |
```

`docs/hld/03-architecture-and-crates.md` names `ocelli-seg`. It does not
give a threshold function.

The label grid this tool writes is the one F-063 already built. That story
reads DICOM PS3.3 C.8.20.2 as follows, and this story does not reopen it.
A segmentation pixel is a segment number. Pixel value 0 is background and
is not a segment number. Writing 0 remains a legal erase.

F-080 compares a sample with a circle by squared index distance and
`total_cmp` `is_le`. This story uses that helper for the circle. It does
not restate a second comparison.

## What the specification does not cover

The HLD does not say which samples a start-end threshold paints. This plan
decides that, and it is not an HLD formula.

The caller supplies one grid and one row-major `f64` buffer of the same
length. This function does not read DICOM. The caller owns modality, VOI
and spacing. The numbers are compared as given, in index units for the
shape.

Inside a rectangle or a circle, a sample is painted with the caller segment
when its value is at or above `start` and at or below `end`. Equality uses
the F-080 same-number rule, so both zero bit patterns match. Outside the
shape, the sample is left unchanged and its value is not a reason to
refuse.

The rectangle is the F-081 inclusive span on `x` and `y`. There is no `z`.
The grid is the slice. `min` above `max`, and not the same number, is
`Degenerate`.

The circle is the F-080 disk. Radius 0 includes only a sample whose index
is exactly the centre. A negative radius is `Degenerate`. Both zero bit
patterns are a zero radius.

A non-finite `start`, `end`, bound, centre or radius is `Frame`. A
non-finite value inside the shape is `Frame` and writes nothing. A
non-finite value outside the shape is ignored.

`covered` counts samples inside the shape and inside the value span.
`changed` counts those whose stored segment differed. Counts use
`saturating_add`.

A zero side, a length that is not `width * height`, or a value buffer of
a different length, is `Degenerate`.

There is no TypeScript copy. There is no sphere in this story.

## Problem and reproducer

No function paints a segment from a value range inside a shape. On a 3 by
3 grid, values 0 through 8 in row-major order, a rectangle covering the
whole grid with start 3 and end 5 must paint only the three middle
samples. Today there is no threshold entry point.

```text
values
0 1 2
3 4 5
6 7 8
```

## Scope, API and compatibility

In scope: `threshold_roi` in `ocelli-seg`.

Out of scope: a sphere threshold, scissors, flood fill, a polygon edit,
interpolation, reading a DICOM pixel, a GPU path, a new presenter, the
representation enum, and the SR content tree.

```rust
pub enum ThresholdShape {
    Rectangle { min_x: f64, min_y: f64, max_x: f64, max_y: f64 },
    Circle { center_x: f64, center_y: f64, radius: f64 },
}

pub struct ThresholdRoi {
    pub shape: ThresholdShape,
    pub start: f64,
    pub end: f64,
    pub segment: u16,
}

pub struct ThresholdCoverage {
    pub covered: u32,
    pub changed: u32,
}

pub enum ThresholdError {
    Frame,
    Degenerate,
}

pub fn threshold_roi(
    grid: &mut LabelGrid,
    values: &[f64],
    roi: ThresholdRoi,
) -> Result<ThresholdCoverage, ThresholdError>
```

Do not derive `PartialEq` on `ThresholdShape` or `ThresholdRoi`.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| `threshold_roi` | one `LabelGrid` plus the caller `f64` buffer | shape from F-080 and F-081, range inclusive | matching samples become `segment` |
| `disk_contains` in `brush.rs` | F-080 squared distance | not reimplemented here | the circle matches a stamp of the same centre and radius |
| existing `present_labelmap` | unchanged | F-063 rules | RGBA8 after the paint |

## State transitions and invariants

| Entry point / starting state | Input or event | Resulting state and output | Refusal and preserved state |
|------------------------------|----------------|----------------------------|-----------------------------|
| values match the grid | inside the shape and inside the span | those samples become `segment` | outside the shape stays, including a non-finite value there |
| values match the grid | inside the shape, value outside the span | that sample stays | it is not covered |
| any grid | non-finite parameter, or a non-finite value inside the shape | `Frame` | every index unchanged |
| any grid | inverted span, negative radius, bad length, value length mismatch | `Degenerate` | every index unchanged |
| matching samples already hold `segment` | the same call again | `changed` is 0 | indices stay |

The scan stages a copy. The grid is assigned only after every inside value
has been read. A non-finite inside value returns before that assignment.

## Ownership, commit and recovery

The caller owns the grid and the value buffer. The function allocates one
staged copy of the indices. It does not retain the values. A refusal drops
the copy. A repeated call reports `changed` 0 when the painted samples
already hold the segment.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | 3x3 values 0 through 8, rectangle over the whole grid, start 3, end 5, segment 7 | indices 7 at offsets 3, 4 and 5 only, covered 3, changed 3 | 2 or 6 painted, or 4 left alone | `crates/ocelli-seg/tests/threshold.rs` |
| AC2 | same values, circle centre (1, 1), radius 1, start 0, end 8, segment 4 | the plus becomes 4, corners stay 0 | a corner painted, or an edge left 0 | same test file |
| AC3 | rectangle (0, 0) through (0, 2), values with a non-finite number at (2, 0) and a finite 4 at (0, 0) | the first column is painted and the call is `Ok` | the outside non-finite value refused | same test file |
| AC4 | the AC1 rectangle with a non-finite value at offset 4 | `Frame` | any index written | same test file |
| AC5 | start 5 and end 3, and a circle radius -1 | `Degenerate` both times | any index written | same test file |
| AC6 | value length 8 on a 3x3, and a zero width | `Degenerate` both times | a short buffer read | same test file |
| AC7 | AC1 run twice | second changed is 0, indices stay 7 | a second paint reported as a change | same test file |
| AC8 | start and end are 4, rectangle over the grid | only offset 4 is painted | the neighbours included | same test file |
| AC9 | present the AC2 grid, fill on, outline off | an edge pixel is the LUT colour, a corner alpha is 0 | presenter formula edited | same test file |

Hand-computed AC2. The circle is the F-080 plus. Every value on that plus
is between 0 and 8 inclusive. The four corners are outside the circle, so
their values are not painted even though they are inside the numeric span.

## Approach

Add `crates/ocelli-seg/src/threshold.rs` and export it from `lib.rs`.
Call the F-081 `pub(crate)` disk helper and the same-number span helper.
Do not copy the squared-distance expression into this file. Do not call
`sqrt`. Do not add glam. Do not edit `labelmap.rs`, `contour.rs` or
`surface.rs`. Do not change `stamp_brush`'s result.

## Dependencies, write ownership and resources

Depends on F-080, which is done, and uses the `pub(crate)` helpers F-081
adds. Implement F-081 first. This story writes `threshold.rs`, `lib.rs`,
`tests/threshold.rs`, the plan, the LLD paragraph and the delivery rows.
It does not write a shader and it does not take the GPU.

## Verification and evidence reuse

Focused check: `bin/ocelli.sh test ocelli-seg --test threshold`. Then
clippy on `ocelli-seg`. Completion uses the feature profile, 27 gates,
with corpus pass and no comparison report. The sprint profile is a later
commit. CPU candidate bytes stay as they are.

Mutate the value test so `end` is excluded, and confirm AC1 goes red on
the sample whose value is 5. Restore it.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none
- unsafe: none
- Tier A (WebGPU): n/a
- Tier B (WebGL2): n/a
- Tier C (CPU): n/a

The threshold does not open a device. It does not read a pixel from a
decoder.

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| fixture | AC1 through AC9, hand-computed in this plan. PS3.3 C.8.20.2 is the background rule only. The value span is this plan. | `crates/ocelli-seg/tests/threshold.rs` |

## LLD impact

- `docs/lld/public-api.md`
- the `public-api.md` row in `docs/lld/README.md`

## Parity surface covered

Appendix B counts 12 segmentation classes and does not name a threshold
row. This story adds the rectangle and circle start-end tools. It does not
establish a rendered parity view.

## Deviations

None. The HLD does not define this tool, so the inclusive value span is
not a departure from a written formula.

## Open questions

None. The operator declined a design-question round. The gaps above are
decided in this plan.
