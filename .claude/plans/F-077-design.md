# F-077, LivewireContour and LivewireContourSegmentation

**Status**: approved
**Epic ref**: E12.5
**Sprint**: S29
**Estimate**: 5w

## Normative source, transcribed

HLD `docs/hld/26-differentiating-capabilities.md` names interactive segmentation as a future compute kernel. It does not define a livewire cost, a neighbourhood, or a fill rule.

```text
Initial kernel set: region growing and interactive segmentation, denoise and gradient filtering, per-frame histogram and ROI statistics, full-resolution MPR and oblique resampling, and registration refinement.
```

```text
A kernel with no fallback marks its feature unavailable; it never silently produces a different answer.
```

`docs/hld/B-parity-surface.md` counts 12 segmentation tool classes. It does not name livewire.

`polygon_area` and `spline_point` stay the F-076 functions. This story does not change them and does not call them.

## What the specification does not cover

The HLD does not define this tool. The decisions below are local. They are not a formula the HLD forgot to write.

- The caller supplies one finite cost per pixel, row-major. This story does not compute a gradient from stored pixels. A path that ignored those costs would be a different contour, and AC2 rejects it.
- The neighbourhood is 4-connected. Diagonals are not edges.
- The path cost is the sum of the costs of the pixels entered. The start pixel adds nothing.
- Search is Dijkstra. A candidate replaces a settled cost only when it is strictly smaller. Equal costs keep the parent already stored. The next unsettled pixel is the lowest index among the lowest costs, so a tie has one path.
- A cost that is negative or non-finite is refused before any path is built. Negative is `Degenerate`. Non-finite is `Frame`.
- `width` or `height` of 0, a cost buffer whose length is not `width * height`, or a start or goal outside the grid is `Degenerate`.
- Start equal to goal is a one-pixel path. It is not `Degenerate`.
- `livewire_mask` marks the pixels the path visits. It does not fill the interior. The HLD does not define a fill rule, and a filled mask that is quietly wrong is the defect this sprint refuses. An empty path is `Degenerate`, not a mask of zeros. A path pixel outside the grid is `Degenerate` and no mask is returned.
- The path and the mask are owned `Vec` values. `ocelli-geom` gains `extern crate alloc` for this story. The search scratch is allocated inside the call and dropped before return.
- No TypeScript copy. No SVG. No SR content item. No GPU dispatch.

## Problem and reproducer

A caller can evaluate a spline point and cannot ask for a cheap path through a cost grid. A straight index-space line would ignore the gradient the caller encoded as costs.

On a 2 by 2 grid with costs `1`, `100`, `1`, `1`, start `(0, 0)` and goal `(1, 1)`, the path is `(0, 0)`, `(0, 1)`, `(1, 1)`. The mask is `1`, `0`, `1`, `1`. The other 4-connected route costs 101 and must not win.

## Scope, API and compatibility

In scope:

- `livewire_path` and `livewire_mask` in `crates/ocelli-geom/src/livewire.rs`
- `extern crate alloc` in `ocelli-geom`
- Unit tests with the grid above

Out of scope:

- A gradient filter, a DICOM reader, and a filled region
- A change to `polygon_area` or `spline_point`
- A TypeScript copy and a `wasm-bindgen` export

```text
livewire_path(costs, width, height, start, goal) -> Result<Vec<Pt<S>>, GeomError>
livewire_mask(width, height, path) -> Result<Vec<u8>, GeomError>
```

`start` and `goal` are `Pt<S>` whose `x` and `y` are pixel indices. `z` is ignored. A non-integer `x` or `y` is `Degenerate`. The integer test is exact, with no epsilon.

## Consumer and provider map

| Consumer | Provider | Policy | Output |
|----------|----------|--------|--------|
| A later livewire tool | `livewire_path` | a rectangular non-negative cost grid | one 4-connected path |
| A later segmentation tool | `livewire_mask` | a non-empty in-range path | one byte per pixel, 1 on the path |
| F-076 | unchanged | this story does not call it | polygon area and spline point |

## State transitions and invariants

Both functions are stateless apart from the values they return. A refusal returns `Err` and no `Vec` the caller can observe.

| Start | Input | Result | Preserved on refusal |
|-------|-------|--------|----------------------|
| none | the 2 by 2 cheap path | `(0,0)`, `(0,1)`, `(1,1)` | no stored state |
| none | start equal to goal | one point | no stored state |
| none | a negative cost | `Degenerate` | no stored state |
| none | the path above | mask `1, 0, 1, 1` | no stored state |
| none | an empty path | `Degenerate` | no stored state |

## Ownership, commit and recovery

The returned `Vec` is the commit point. Scratch for distances and parents is dropped on both success and refusal. A second call with the same arguments returns an equal path. It does not append to a previous mask.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test | Wrong implementation rejected | Stage |
|----|-------------------------------|------|-------------------------------|-------|
| AC1 | The cheap 2 by 2 path detours through `(0, 1)` | `the_cheap_path_follows_the_gradient` | a straight line through the cost-100 pixel | implementation |
| AC2 | That mask is `1, 0, 1, 1` | `the_mask_is_the_visited_pixels` | filling the interior or the whole grid | implementation |
| AC3 | Start equal to goal is one point | `a_seed_is_a_one_pixel_path` | refusing a closed seed | implementation |
| AC4 | A negative cost is `Degenerate` | `a_negative_cost_is_degenerate` | searching anyway | implementation |
| AC5 | An empty path is `Degenerate` | `an_empty_path_is_not_a_blank_mask` | returning a zero mask | implementation |
| AC6 | A non-integer index is `Degenerate` | `a_fractional_index_is_degenerate` | truncating toward a pixel | implementation |

## Approach

Implement Dijkstra with a linear scan for the next unsettled pixel. The grid in the fixture has four pixels, so the scan is the algorithm rather than a stand-in. Parent pointers reconstruct the path from the goal back to the start, then the function reverses that list.

Rejected: an 8-connected neighbourhood, a filled polygon, and a call to `polygon_area`.

## Dependencies, write ownership and resources

Depends on F-076, which is done. This story writes `crates/ocelli-geom` and its tests. It runs after F-075 because both write that crate. It does not write shaders or the TypeScript package.

## Verification and evidence reuse

`bin/ocelli.sh test ocelli-geom` for the focused loop. `/complete-feature` runs the feature profile.

The path and the mask are hand-computed from the costs in this plan.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none. The `Vec` is the tool result, not a frame buffer.
- unsafe: none
- Tier A (WebGPU): n/a
- Tier B (WebGL2): n/a
- Tier C (CPU): n/a

The functions do not open a device. The HLD segmentation kernel is not this story.

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| unit | the acceptance rows above | `crates/ocelli-geom/tests/livewire.rs` |
| fixture | the 2 by 2 path and mask | the same tests |

## Parity surface covered

None. Appendix B counts the segmentation classes and does not define livewire.

## Deviations

none

## LLD impact

`docs/lld/geometry.md` and `docs/lld/public-api.md` gain `livewire_path` and `livewire_mask` when the story is completed. They say the mask is the visited pixels and that `polygon_area` is unchanged.

## Open questions

None. The operator declined a design-question round on 2026-10-31. The 4-connected cost sum and the visited-pixel mask are the local decisions.
