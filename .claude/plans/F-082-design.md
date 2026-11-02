# F-082, paint fill and flood fill

**Status**: approved
**Epic ref**: E13.3
**Sprint**: S31
**Estimate**: 3w

## Normative source, transcribed

`docs/hld/B-parity-surface.md` Appendix B counts the classes. It does not
define a fill. The quotation is the whole of the relevant rows.

```text
| Tool classes | ~63 | 26 annotation, 12 segmentation, 25 manipulation and utility |
| Segmentation representations | 3 | Labelmap, contour, surface |
```

`docs/hld/03-architecture-and-crates.md` names `ocelli-seg`. It does not
give a fill function.

The label grid this tool writes is the one F-063 already built. That story
reads DICOM PS3.3 C.8.20.2 as follows, and this story does not reopen it.
A segmentation pixel is a segment number. Pixel value 0 is background and
is not a segment number. Writing 0 remains a legal erase.

## What the specification does not cover

The HLD does not say which samples a flood or a paint fill replaces. This
plan decides that, and it is not an HLD formula.

Both functions take one grid, one seed column and row, and the segment to
write. The seed selects a value by reading it. The written segment may be
0.

`flood_fill` replaces the 4-connected region of samples that still hold
the seed's value. The four neighbours are the samples one column left or
right, and one row up or down, when they exist. A diagonal neighbour is
not connected. A sample with a different value stops the region.

`paint_fill` replaces every sample on that same grid whose stored value
equals the seed's value. Connection does not matter.

`covered` counts samples in the region, including ones that already hold
the written segment. `changed` counts those whose stored number differed.
A seed that already holds the written segment returns `changed` 0. Counts
use `saturating_add`.

A seed outside the grid, a zero side, a length that is not
`width * height`, or a pixel count that does not fit in `u32`, is
`Degenerate`. A refusal writes nothing. There is no `Frame` variant.
The seed is a `u32`, so it cannot be non-finite.

There is no TypeScript copy. This story does not call `stamp_brush` and
does not edit it.

## Problem and reproducer

No function replaces a connected region or every copy of one segment
number. On the grid below, a flood from `(0, 0)` writing segment 2 must
change only the first sample. A paint fill from the same seed must change
both 1s. Today there is no fill entry point.

```text
1 0 1
```

## Scope, API and compatibility

In scope: `flood_fill` and `paint_fill` in `ocelli-seg`.

Out of scope: scissors, a threshold, a polygon edit, interpolation, a GPU
fill, a new presenter, the representation enum, and the SR content tree.

```rust
pub struct FillCoverage {
    pub covered: u32,
    pub changed: u32,
}

pub enum FillError {
    Degenerate,
}

pub fn flood_fill(
    grid: &mut LabelGrid,
    x: u32,
    y: u32,
    segment: u16,
) -> Result<FillCoverage, FillError>

pub fn paint_fill(
    grid: &mut LabelGrid,
    x: u32,
    y: u32,
    segment: u16,
) -> Result<FillCoverage, FillError>
```

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| `flood_fill` | one `LabelGrid` | 4-connected, seed value | that region becomes `segment` |
| `paint_fill` | one `LabelGrid` | every sample equal to the seed value | those samples become `segment` |
| existing `present_labelmap` | unchanged | F-063 rules | RGBA8 after the fill |
| `stamp_brush` and `cut_scissor` | unchanged | not called | their fixtures stay |

## State transitions and invariants

| Entry point / starting state | Input or event | Resulting state and output | Refusal and preserved state |
|------------------------------|----------------|----------------------------|-----------------------------|
| seed in range | flood, neighbours share the seed value | the region becomes `segment` | a different value stays |
| seed in range | paint | every equal sample becomes `segment` | other values stay |
| seed already equal to `segment` | either function | `changed` is 0 | indices stay |
| seed outside, zero side, bad length, pixel count above `u32` | either function | `Degenerate` | every index unchanged |
| diagonal neighbour with the seed value | flood | that neighbour stays | paint would replace it |

The walk finishes on a staged copy. The grid is assigned only after the
region is complete. A refusal returns before that assignment.

## Ownership, commit and recovery

The caller owns the grid. The function allocates one staged copy and one
visit buffer. A refusal drops both. A repeated call with the same segment
reports `changed` 0.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | 3x3 plus of 1s, flood from (1, 1) to segment 2 | five 2s, corners stay 0, covered 5, changed 5 | a corner painted, or an edge left as 1 | `crates/ocelli-seg/tests/fill.rs` |
| AC2 | 3x3 with 1s on both diagonals, flood from the centre | only the centre becomes 2 | a diagonal neighbour painted | same test file |
| AC3 | row `1 0 1`, flood from x 0, then paint from a fresh copy | flood changes the first sample only, paint changes both | the gap treated as connected, or paint stopping at the gap | same test file |
| AC4 | seed x equal to the width, and a 2x2 with 3 indices | `Degenerate` both times | any sample written | same test file |
| AC5 | flood of a region that already holds segment 2 | covered is the region size, changed 0 | a second fill reported as a change | same test file |
| AC6 | one 0 among 1s, flood from that 0 writing 0 | covered 1, changed 0, the 1s stay | background refused, or a 1 erased | same test file |
| AC7 | paint every 1 on the AC1 plus to 0, then present with fill on | the centre alpha is 0, because 0 is background | presenter formula edited | same test file |
| AC8 | a 4-connected snake that turns a corner | the whole snake changes and the off-snake sample stays | the turn dropped | same test file |

Hand-computed AC1. The plus is the centre and the four edge neighbours.
Each edge shares a side with the centre. Each corner shares only a corner,
so the flood does not enter it. AC2 is the opposite pattern and covers 1.

AC8 grid, width 3:

```text
1 1 0
0 1 0
0 1 1
```

Flood from `(0, 0)` reaches `(1, 0)`, `(1, 1)`, `(1, 2)` and `(2, 2)`.
The sample at `(2, 0)` stays 0. Covered 5.

## Approach

Add `crates/ocelli-seg/src/fill.rs` and export it from `lib.rs`. Flood uses
`alloc::collections::VecDeque` and a visit buffer. Offsets use checked
`u32` arithmetic and `usize::try_from`. Do not index the slice. Do not add
glam. Do not edit `labelmap.rs`, `brush.rs`, `contour.rs` or `surface.rs`.

## Dependencies, write ownership and resources

Depends on F-080, which is done. The fill does not call the brush. This
story writes `fill.rs`, `lib.rs`, `tests/fill.rs`, the plan, the LLD
paragraph and the delivery rows. It does not write a shader and it does
not take the GPU.

## Verification and evidence reuse

Focused check: `bin/ocelli.sh test ocelli-seg --test fill`. Then clippy on
`ocelli-seg`. Completion uses the feature profile, 27 gates, with corpus
pass and no comparison report. The sprint profile is a later commit. CPU
candidate bytes stay as they are.

Mutate the flood so it also steps by `(1, 1)`, and confirm AC2 goes red.
Restore it.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none
- unsafe: none
- Tier A (WebGPU): n/a
- Tier B (WebGL2): n/a
- Tier C (CPU): n/a

The fill does not open a device.

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| fixture | AC1 through AC8, hand-computed in this plan. PS3.3 C.8.20.2 is the background rule only. | `crates/ocelli-seg/tests/fill.rs` |

## LLD impact

- `docs/lld/public-api.md`
- the `public-api.md` row in `docs/lld/README.md`

## Parity surface covered

Appendix B counts 12 segmentation classes and does not name a fill row.
This story adds flood fill and paint fill. It does not establish a
rendered parity view.

## Deviations

None. The HLD does not define this tool, so the 4-connected rule is not a
departure from a written formula.

## Open questions

None. The operator declined a design-question round. The gaps above are
decided in this plan.
