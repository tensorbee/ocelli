# F-085, labelmap edit with contour

**Status**: approved
**Epic ref**: E13.6
**Sprint**: S31
**Estimate**: 3w

## Normative source, transcribed

`docs/hld/B-parity-surface.md` Appendix B counts the classes and the three
representations. It does not define an edit of a labelmap from a polygon.
The quotation is the whole of the relevant rows.

```text
| Tool classes | ~63 | 26 annotation, 12 segmentation, 25 manipulation and utility |
| Segmentation representations | 3 | Labelmap, contour, surface |
```

`docs/hld/03-architecture-and-crates.md` names `ocelli-seg`. It does not
give this function.

F-064 stores a contour in patient space and draws it. F-066 stores which
representation a viewport shows. This story does not edit either. The grid
it writes is the F-063 label grid. DICOM PS3.3 C.8.20.2 still says a
segmentation pixel is a segment number and 0 is background.

## What the specification does not cover

The HLD does not say which label samples a polygon includes. This plan
decides that, and it is not an HLD formula and not the F-064 contour.

The caller supplies one grid and a list of index points. Fewer than 3
points is `Degenerate`. The function closes the ring from the last point
to the first. The caller does not have to repeat the first point.

A sample at integer column `x` and row `y` is tested at
`(f64::from(x), f64::from(y))`. It is not tested at a half-pixel centre.

A sample on any closed edge is inside. An edge contains the sample when
the cross product of the edge and the sample offset is a zero bit pattern,
either `0` or `-0`, and the dot of that offset with the edge is at or
above 0 and at or below the edge length squared. Equality uses the F-080
same-number rule. A zero-length edge contains only that same point.

A sample that is not on an edge is inside when a horizontal ray from the
sample toward increasing `x` crosses an odd number of edges. An edge
counts when one endpoint is at or below the sample `y` and the other is
strictly above, and the intersection `x` is strictly above the sample `x`.
A horizontal edge does not count. An intersection `x` that is the same
number as the sample `x` means the sample is on the boundary, so it is
inside. A non-finite intersection is `Frame` and writes nothing.

A non-finite vertex is `Frame`. A zero side or a length that is not
`width * height` is `Degenerate`. A refusal writes nothing.

Inside samples become the caller segment, including segment 0. Outside
samples stay. `covered` and `changed` follow the F-081 meaning. Counts use
`saturating_add`.

There is no TypeScript copy. This story does not change the representation
enum and does not call `present_contour` or `intersect_contour`.

## Problem and reproducer

No function paints label samples from a polygon. On a grid of width 5 and
height 3, the rectangle `(1, 0), (3, 0), (3, 2), (1, 2)` must paint the
three middle columns and leave both outer columns at 0. Today there is no
such entry point.

```text
0 1 1 1 0
0 1 1 1 0
0 1 1 1 0
```

## Scope, API and compatibility

In scope: `edit_labelmap_with_contour` in `ocelli-seg`.

Out of scope: patient-space contours, the F-064 geometric type, surface
intersection, scissors, fill, a threshold, interpolation, a GPU path, a
new presenter, the representation enum, and the SR content tree.

```rust
pub struct IndexPoint {
    pub x: f64,
    pub y: f64,
}

pub struct ContourEditCoverage {
    pub covered: u32,
    pub changed: u32,
}

pub enum ContourEditError {
    Frame,
    Degenerate,
}

pub fn edit_labelmap_with_contour(
    grid: &mut LabelGrid,
    polygon: &[IndexPoint],
    segment: u16,
) -> Result<ContourEditCoverage, ContourEditError>
```

Do not derive `PartialEq` on `IndexPoint`.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| `edit_labelmap_with_contour` | one `LabelGrid` | index units, even-odd plus boundary | inside samples become `segment` |
| F-064 `present_contour` | unchanged | not called | contour fixtures stay |
| `Representation` | unchanged | not written | a viewport's choice stays |
| existing `present_labelmap` | unchanged | F-063 rules | RGBA8 after the edit |

## State transitions and invariants

| Entry point / starting state | Input or event | Resulting state and output | Refusal and preserved state |
|------------------------------|----------------|----------------------------|-----------------------------|
| three or more finite points | a sample inside or on the boundary | that sample becomes `segment` | an outside sample stays |
| fewer than 3 points, zero side, bad length | any segment | `Degenerate` | every index unchanged |
| a non-finite vertex, or a non-finite intersection | any segment | `Frame` | every index unchanged |
| inside samples already hold `segment` | the same polygon again | `changed` is 0 | indices stay |

The scan stages a copy. The grid is assigned only after every sample has
been classified. A `Frame` during the scan returns before that assignment.

## Ownership, commit and recovery

The caller owns the grid and the point list. The function allocates one
staged copy of the indices. It does not retain the polygon. A refusal
drops the copy. A repeated call reports `changed` 0 when the inside
samples already hold the segment.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | width 5, height 3, vertices (1, 0), (3, 0), (3, 2), (1, 2), segment 4 | both outer columns stay 0, the middle three columns become 4, covered 9, changed 9 | the x = 3 column excluded, or x = 0 painted | `crates/ocelli-seg/tests/contour_edit.rs` |
| AC2 | 3x3, triangle (2, 0), (2, 2), (0, 1), segment 5 | (0, 1) and (1, 1) and the right column's on-edge samples follow the rule below | (1, 0) painted, or (1, 1) left 0 | same test file |
| AC3 | two points, and a 2x2 with 3 indices | `Degenerate` both times | any index written | same test file |
| AC4 | one vertex with y = NaN, on a grid of 7s | `Frame` | any 7 replaced | same test file |
| AC5 | vertices (1e308, -1), (-1e308, 1), (0, 2) on a 1x1 grid of 7 | `Frame` | the sample written, or the overflow ignored | same test file |
| AC6 | AC1 with the first vertex repeated at the end | the same nine samples, covered 9 | the repeated point opening a gap | same test file |
| AC7 | AC1 run twice | second changed is 0 | a second edit reported as a change | same test file |
| AC8 | present the AC1 grid, fill on, outline off, colour of segment 4 | an inside pixel is the LUT colour, an outer pixel alpha is 0 | presenter formula edited, or contour.rs edited | same test file |

Hand-computed AC2 on a 3x3, segment 5, starting from zeros. The right edge
from (2, 0) to (2, 2) contains (2, 0), (2, 1) and (2, 2). The apex (0, 1)
is a vertex. (1, 1) has one crossing, on the right edge, so it is inside.
(1, 0) has two crossings and is outside. (0, 0) and (0, 2) are outside.
(1, 2) is outside. The written grid is:

```text
0 0 5
5 5 5
0 0 5
```

Covered 5. Changed 5.

AC5. The edge from (1e308, -1) to (-1e308, 1) straddles y = 0, and the
intersection overflows to a non-finite `x`. That is `Frame`. The sample
stays 7.

## Approach

Add `crates/ocelli-seg/src/contour_edit.rs` and export it from `lib.rs`.
Use the F-081 same-number helper for the zero tests. Do not edit
`contour.rs`, `surface.rs`, `labelmap.rs` or the representation enum. Do
not call `sqrt`. Do not add glam.

## Dependencies, write ownership and resources

Depends on F-080, which is done, and uses the same-number helper F-081
exports. Implement F-081 first. This story writes `contour_edit.rs`,
`lib.rs`, `tests/contour_edit.rs`, the plan, the LLD paragraph and the
delivery rows. It does not write a shader and it does not take the GPU.

## Verification and evidence reuse

Focused check: `bin/ocelli.sh test ocelli-seg --test contour_edit`. Then
clippy on `ocelli-seg`. Completion uses the feature profile, 27 gates,
with corpus pass and no comparison report. The sprint profile is a later
commit. CPU candidate bytes stay as they are.

Mutate the crossing test from odd to even, and confirm AC1 goes red
because the interior sample at (2, 1) is not on a boundary. Restore it.
The boundary columns would still pass that mutation, which is why the
interior sample is the witness.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none
- unsafe: none
- Tier A (WebGPU): n/a
- Tier B (WebGL2): n/a
- Tier C (CPU): n/a

The edit does not open a device.

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| fixture | AC1 through AC8, hand-computed in this plan. PS3.3 C.8.20.2 is the background rule only. The even-odd rule is this plan. | `crates/ocelli-seg/tests/contour_edit.rs` |

## LLD impact

- `docs/lld/public-api.md`
- the `public-api.md` row in `docs/lld/README.md`

## Parity surface covered

Appendix B counts the contour representation and 12 segmentation classes.
This story edits the labelmap from an index polygon. It does not add a
contour representation and it does not establish a rendered parity view.

## Deviations

None. The HLD does not define this edit, so the even-odd rule is not a
departure from a written formula.

## Open questions

None. The operator declined a design-question round. The gaps above are
decided in this plan.
