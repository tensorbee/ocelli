# F-084, segment select, label and bidirectional

**Status**: approved
**Epic ref**: E13.5
**Sprint**: S30
**Estimate**: 3w

## Normative source, transcribed

`docs/hld/B-parity-surface.md` Appendix B counts the classes. It does not
define these three tools. The quotation is the whole of the relevant rows.

```text
| Tool classes | ~63 | 26 annotation, 12 segmentation, 25 manipulation and utility |
| Segmentation representations | 3 | Labelmap, contour, surface |
```

F-066 already stores which representation a viewport shows. This story
does not change that enum.

The segment number rule is the one F-063 reads from DICOM PS3.3 C.8.20.2.
A segmentation pixel is a segment number. Pixel value 0 is background and
is not a segment number.

F-073 `bidirectional` measures two caller distances. This story does not
call it and does not change it.

## What the specification does not cover

The HLD does not say how a segment is selected, how its label is stored,
or which run is the bidirectional pair. This plan decides that, and it is
not an HLD formula.

`segment_at` returns the stored `u16` at one column and row. A value of 0
is a successful read of background. A column or row outside the grid is
`Degenerate`, so a miss is not reported as background. A zero side or a
length that is not `width * height` is `Degenerate`.

`label_segment` stores trimmed text for a segment above 0. Segment 0 is
`Background` and the list is unchanged. Text whose trim is empty is
`Blank` and the list is unchanged. A segment already present keeps its
place and replaces the text. A new segment is appended.

`segment_bidirectional` finds one horizontal run and one vertical run of
one segment above 0. Length is the count of samples in the run. The ends
are inclusive. The longest run wins. An equal length keeps the run found
first while scanning rows from the top and columns from the left, and
inside a row or column from the low index. A missing segment is
`Degenerate`. Segment 0 is `Degenerate` even when the grid is all
background. One isolated sample is a run of length 1 on both axes.

There is no TypeScript copy. The readings stay in `ocelli-seg`. The SVG
layer and the SR content tree are not edited.

## Problem and reproducer

No function reads a segment number back from a label grid, names that
segment, or reports its longest row run and column run. On the grid below,
segment 1 must select at `(1, 0)`, must refuse a label, and must report
the bottom row and the middle column.

```text
0 1 1 0
0 1 0 0
1 1 1 1
```

Horizontal run: `(0, 2)` through `(3, 2)`, length 4.
Vertical run: `(1, 0)` through `(1, 2)`, length 3.

## Scope, API and compatibility

In scope: `segment_at`, `label_segment` and `segment_bidirectional` in
`ocelli-seg`.

Out of scope: the brush, scissors, flood fill, the F-073 measurement
functions, the representation enum, the presenter and the SR tree.

```rust
pub enum SegmentToolError {
    Degenerate,
    Background,
    Blank,
}

pub struct SegmentLabel {
    pub segment: u16,
    pub text: String,
}

pub struct SegmentRun {
    pub start_x: u32,
    pub start_y: u32,
    pub end_x: u32,
    pub end_y: u32,
    pub length: u32,
}

pub struct SegmentBidirectional {
    pub horizontal: SegmentRun,
    pub vertical: SegmentRun,
}

pub fn segment_at(grid: &LabelGrid, x: u32, y: u32) -> Result<u16, SegmentToolError>
pub fn label_segment(labels: &mut Vec<SegmentLabel>, segment: u16, text: &str) -> Result<(), SegmentToolError>
pub fn segment_bidirectional(grid: &LabelGrid, segment: u16) -> Result<SegmentBidirectional, SegmentToolError>
```

`Background` and `Blank` are label refusals. `segment_at` and
`segment_bidirectional` use `Degenerate` only.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| `segment_at` | `LabelGrid::indices` | caller supplies the column and row | one segment number, including 0 |
| `label_segment` | caller `Vec<SegmentLabel>` | trim, refuse 0 | the list, order preserved |
| `segment_bidirectional` | `LabelGrid::indices` | one segment above 0 | two inclusive runs |

`ocelli-geom::bidirectional` is not a provider and is not a caller.

## State transitions and invariants

| Entry point / starting state | Input or event | Resulting state and output | Refusal and preserved state |
|------------------------------|----------------|----------------------------|-----------------------------|
| grid | in-range column and row | `Ok(segment)` | grid unchanged |
| grid | out of range, empty, or bad length | `Degenerate` | grid unchanged |
| label list | segment above 0 and non-blank text | insert or replace | earlier order kept |
| label list | segment 0, or blank text | `Background` or `Blank` | list unchanged |
| grid | segment present | two runs | grid unchanged |
| grid | segment 0, or segment absent | `Degenerate` | grid unchanged |

## Ownership, commit and recovery

The caller owns the grid and the label list. `segment_at` and
`segment_bidirectional` allocate nothing durable and do not write the
grid. `label_segment` writes the list only after the segment and the text
are accepted. A repeated label of the same segment replaces the text once.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | read `(1, 0)`, `(0, 0)` and `(4, 0)` on the 4x3 grid | 1, then 0, then `Degenerate` | an outside pixel reported as background | `crates/ocelli-seg/tests/segment_tools.rs` |
| AC2 | a 2x2 grid with 3 indices | `Degenerate` | a short buffer read as 0 | same test file |
| AC3 | label segment 0, then a blank, then `" liver "` on segment 2 | empty, empty, then text `liver` | 0 stored, or the spaces kept | same test file |
| AC4 | label 1 then 2, then replace 1 | order 1, 2 and the new text | the replaced segment moved | same test file |
| AC5 | bidirectional on the 4x3 grid | length 4 on row 2, length 3 on column 1 | the shorter upper run chosen | same test file |
| AC6 | two horizontal runs of length 2, top row first | the top run | the lower run chosen | same test file |
| AC7 | two full columns of segment 1 | the left column | the right column chosen | same test file |
| AC8 | segment 2 absent, and segment 0 on a zero grid | `Degenerate` both times | a zero run returned | same test file |
| AC9 | one sample at `(2, 1)` | both runs length 1 at that sample | a neighbour included | same test file |

## Approach

Add `crates/ocelli-seg/src/segment_tools.rs` and export it from `lib.rs`.
Scan with `get`. Do not edit `measure.rs`, `annotation.ts`, `labelmap.rs`
or the representation enum.

## Dependencies, write ownership and resources

Depends on F-066, which is done. The representation enum is read-only for
this story. This story writes `segment_tools.rs`, `lib.rs`,
`tests/segment_tools.rs`, the plan, the LLD paragraph and the delivery
rows. It does not take the GPU. It does not write `brush.rs` unless that
file is still absent, and it does not change the brush rules.

## Verification and evidence reuse

Focused check: `bin/ocelli.sh test ocelli-seg --test segment_tools`. Then
clippy on `ocelli-seg`. Completion uses the feature profile, 27 gates,
with corpus pass and no comparison report.

Mutate the horizontal replacement from `>` to `>=` and confirm AC6 goes
red, because the later run of equal length would replace the earlier one.
Restore it.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none
- unsafe: none
- Tier A (WebGPU): n/a
- Tier B (WebGL2): n/a
- Tier C (CPU): n/a

These functions do not open a device.

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| fixture | AC1 through AC9, hand-computed in this plan. PS3.3 C.8.20.2 is the background rule only. The run rule is this plan. | `crates/ocelli-seg/tests/segment_tools.rs` |

## LLD impact

- `docs/lld/public-api.md`
- the `public-api.md` row in `docs/lld/README.md`
