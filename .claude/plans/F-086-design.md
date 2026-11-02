# F-086, between-slice labelmap interpolation

**Status**: approved
**Epic ref**: E13.7
**Sprint**: S32
**Estimate**: 3w

## Normative source, transcribed

`docs/hld/B-parity-surface.md` Appendix B counts the classes. It does not
define an interpolation. The quotation is the whole of the relevant rows.

```text
| Tool classes | ~63 | 26 annotation, 12 segmentation, 25 manipulation and utility |
| Segmentation representations | 3 | Labelmap, contour, surface |
```

`docs/hld/03-architecture-and-crates.md` names `ocelli-seg`. It does not
give an interpolation function.

The label grid this tool writes is the one F-063 already built. That story
reads DICOM PS3.3 C.8.20.2 as follows, and this story does not reopen it.
A segmentation pixel is a segment number. Pixel value 0 is background and
is not a segment number. Writing 0 remains a legal erase.

## What the specification does not cover

The HLD does not say which samples a between-slice interpolation writes.
This plan decides that, and it is not an HLD formula.

The caller supplies a stack of label grids in slice order and one segment
number. Every grid must share width and height and a length of
`width * height`. This function does not read DICOM and it does not read
spacing. Slice distance is one index.

A slice is a key when any sample equals `segment`. Consecutive keys are
pairs in that list. Only slices strictly between a pair are eligible. An
endpoint key is not rewritten. A slice that is not between a pair stays.

For a slice `step` indices above the lower key, with `span` the distance
between the keys, `t` is `f64::from(step) / f64::from(span)`. A sample is
written when the lower key holds `segment` and `t` is at or below one
half, or when the upper key holds `segment` and `t` is at or above one
half. The comparison is `total_cmp`. One half is exact for an even span.
A slice closer to the lower key therefore takes the lower mask. A slice
closer to the upper key takes the upper mask. The exact halfway slice
takes the union.

`covered` counts written samples, including ones that already hold
`segment`. `changed` counts those whose stored number differed. Counts
use `saturating_add`. Fewer than two keys returns covered 0 and changed 0
and writes nothing. That is success.

An empty stack, a zero side, a length that is not `width * height`, or
mismatched slices, is `Degenerate`. A refusal writes nothing. There is
no `Frame` variant. The inputs are integers.

There is no TypeScript copy. This story does not call `stamp_brush` and
does not change its result.

## Problem and reproducer

No function fills the slice between two labelled slices. On three 1 by 1
grids, segment 4 on the ends and 0 in the middle, the middle must become
4. Today there is no interpolation entry point.

```text
slice 0: 4
slice 1: 0
slice 2: 4
```

## Scope, API and compatibility

In scope: `interpolate_labelmap` in `ocelli-seg`.

Out of scope: spacing in millimetres, a contour interpolation, a GPU
path, a new presenter, the representation enum, scissors, fill, a
threshold, and the SR content tree.

```rust
pub struct InterpolateCoverage {
    pub covered: u32,
    pub changed: u32,
}

pub enum InterpolateError {
    Degenerate,
}

pub fn interpolate_labelmap(
    grids: &mut [LabelGrid],
    segment: u16,
) -> Result<InterpolateCoverage, InterpolateError>
```

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| `interpolate_labelmap` | a stack of `LabelGrid` | halfway rule in this plan | between-slices gain `segment` |
| existing `present_labelmap` | unchanged | F-063 rules | RGBA8 after the write |
| `stamp_brush` | unchanged | not called | its fixtures stay |

## State transitions and invariants

| Entry point / starting state | Input or event | Resulting state and output | Refusal and preserved state |
|------------------------------|----------------|----------------------------|-----------------------------|
| two or more keys | a slice strictly between a consecutive pair | eligible samples become `segment` | the key slices stay |
| one key, or none | any segment | covered 0 and changed 0 | every index unchanged |
| empty stack, zero side, bad length, mismatched slices | any segment | `Degenerate` | every index unchanged |
| the same call again | the same segment | changed is 0 | indices stay |

The walk copies every slice first. The grids are assigned only after every
between-slice has been decided. A refusal returns before that assignment.
Endpoint masks are read from the caller's original indices.

## Ownership, commit and recovery

The caller owns the stack. The function allocates one staged copy of each
index buffer. A refusal drops the copies. A repeated call reports
`changed` 0 when the written samples already hold the segment.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | three 1x1 grids, ends are 4, middle is 0, segment 4 | middle becomes 4, ends stay 4, covered 1, changed 1 | the middle left at 0, or an end rewritten | `crates/ocelli-seg/tests/interpolate.rs` |
| AC2 | three rows, low `[4, 0, 0]`, high `[0, 0, 4]`, middle zeros | middle becomes `[4, 0, 4]` | the halfway union dropped, or the centre painted | same test file |
| AC3 | four slices, span 3, low `[4, 0]`, high `[0, 4]` | slice 1 is `[4, 0]`, slice 2 is `[0, 4]` | both between-slices take the same mask | same test file |
| AC4 | one key, and two adjacent keys | covered 0 both times, every index unchanged | a key slice rewritten | same test file |
| AC5 | an empty stack, and two grids whose widths differ | `Degenerate` both times | any index written | same test file |
| AC6 | AC1 run twice | second changed is 0 | a second fill reported as a change | same test file |
| AC7 | ends hold 0, middle holds 5, segment 0, then present | middle becomes 0 and its presenter alpha is 0 | background refused, or the presenter edited | same test file |
| AC8 | keys at slices 0 and 2 only, slice 3 holds 9 | slice 1 follows the pair, slice 3 stays 9 | the fill continued past the upper key | same test file |

Hand-computed AC2. The middle slice is one step of a span of two, so `t`
is one half. The low key contributes column 0. The high key contributes
column 2. The union is `[4, 0, 4]`. Column 1 is on neither key.

Hand-computed AC3. Span 3. Slice 1 has `t` of one third, which is below
one half, so only the low mask is written. Slice 2 has `t` of two thirds,
which is above one half, so only the high mask is written.

## Approach

Add `crates/ocelli-seg/src/interpolate.rs` and export it from `lib.rs`.
Use `require_stack` for the rectangle check. Offsets use checked `u32`
arithmetic and `usize::try_from`. Do not index a slice. Do not call
`sqrt`. Do not add glam. Do not edit `labelmap.rs`, `brush.rs`,
`contour.rs` or `surface.rs`.

## Dependencies, write ownership and resources

Depends on F-080, which is done. This story writes `interpolate.rs`,
`lib.rs`, `tests/interpolate.rs`, the plan, the LLD paragraph and the
delivery rows. It does not write a shader and it does not take the GPU.

## Verification and evidence reuse

Focused check: `bin/ocelli.sh test ocelli-seg --test interpolate`. Then
clippy on `ocelli-seg`. Completion uses the feature profile, 27 gates,
with corpus pass and no comparison report. The sprint profile is a later
commit. CPU candidate bytes stay as they are.

Mutate the halfway test from `is_le` to `is_lt` on the low side, and
confirm AC2 goes red because column 0 of the middle slice is the exact
half. Restore it.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none
- unsafe: none
- Tier A (WebGPU): n/a
- Tier B (WebGL2): n/a
- Tier C (CPU): n/a

The interpolation does not open a device.

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| fixture | AC1 through AC8, hand-computed in this plan. PS3.3 C.8.20.2 is the background rule only. The halfway rule is this plan. | `crates/ocelli-seg/tests/interpolate.rs` |

## LLD impact

- `docs/lld/public-api.md`
- the `public-api.md` row in `docs/lld/README.md`

## Parity surface covered

Appendix B counts 12 segmentation classes and does not name an
interpolation row. This story adds the between-slice labelmap tool. It
does not establish a rendered parity view.

## Open questions

None. The HLD does not specify the fraction. The halfway rule above is
the local decision and it is approved with this plan.
