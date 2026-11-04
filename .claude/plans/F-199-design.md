# F-199, example layout grid, stack drag, full screen, reference lines and reference cursor

**Status**: approved
**Epic ref**: E38.9
**Sprint**: S44
**Estimate**: 4w

## Normative source, transcribed

`docs/hld/01-purpose-and-scope.md` section 1 puts stack and volume viewports, MPR and 3D in Phase 1. The study database is Phase 2. The study library UI is Phase 3.

`docs/hld/05-rendering.md` section 7 names tier A as WebGPU and tier B as WebGL2 through the downlevel profile. Volume rendering runs on both, because a 3D texture ray cast in a fragment shader is legal on tier B. Compute stays on tier A and degrades rather than failing. One submit per frame. Deviation D-07 adds tier C. A feature that cannot run on the resolved tier reports unavailable and does not invent another answer.

`docs/hld/07-concurrency-and-typescript.md` section 9 keeps one wasm instance per worker. The main thread does DOM, events and tool UI. There is no SharedArrayBuffer. Section 10 keeps DOM, pointer, touch, wheel and canvas lifecycle in TypeScript. Geometry stays in Rust.

`docs/hld/B-parity-surface.md` is a summary table. It lists 12 viewport types, about 63 tool classes, 5 blend modes, 3 VOI LUT functions, about 13 transfer syntaxes, 3 segmentation representations, 50 core events plus 53 tool events, and 4 adapters. It has no per-capability row and no Covered by column.

`docs/lld/public-api.md` already records `overlayGrid`, `stackDrag`, `referenceLines` and `referenceCursor`. This story calls those functions. It does not add a second formula.

## What the specification does not cover

The HLD does not name the example page layout, a drag protocol, or the Fullscreen API. This plan puts one layout section on `examples/viewer-react`. The grid, the drag step, the reference lines and the reference cursor are the return values of the functions named above. Full screen uses `document.fullscreenEnabled` on that section and reads unavailable when the flag is not true.

Prior studies, linked scroll, synced window and zoom, a frame-of-reference 3D cursor, an MPR draw, a slab draw, a volume draw and fusion are not given a new formula here. Each has a line that reads unavailable. The work is unscheduled F-203. The existing series panel stays the series panel. This story does not add a second one.

## Problem and reproducer

The example page has no control that calls `overlayGrid`, `stackDrag`, `referenceLines` or `referenceCursor`.

## Scope, API and compatibility

In scope: one layout section, the four calls, the full screen word, and the unavailable lines.

Out of scope: a study database, a second series panel, a renderer, F-202, and F-119. No new wasm export. No pixel getter.

A refused call leaves the line on refused. It does not substitute a zero.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| Layout button | `overlayGrid`, `stackDrag`, `referenceLines`, `referenceCursor` | Fixed synthetic viewport numbers | Line count, drag step, two segments, one cursor |
| Full screen button | `fullscreenWord` | `document.fullscreenEnabled` | unavailable, on, or off |
| Prior studies, linked scroll, synced window, MPR, slab, volume, fusion | none | No core call | the word unavailable |

## State transitions and invariants

The report is computed when the button runs. A second run with the same inputs replaces the text with the same text. Full screen does not change the grid numbers. An unavailable line never becomes a coordinate.

## Ownership, commit and recovery

The page owns the section element. There is no wasm session in this story. A refused fullscreen request leaves the word unavailable and leaves the last report in place.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | 2 by 2 on a 100 by 80 viewport reports 2 grid lines | `layout-report.test.ts` | A constant line count | implementation |
| AC2 | 1 by 1 reports 0 grid lines | same | Treating one cell as a drawn line | implementation |
| AC3 | A drag of 256 px upward reports step 1, and a drag of 0 reports refused | same | Using the first lateral step of a volume | implementation |
| AC4 | The cursor and both reference segments are the caller point on the caller viewport | same | Reading a frame of reference | implementation |
| AC5 | Full screen is unavailable when the browser flag is not true | `fullscreenWord` test | Showing on when the API is absent | implementation |
| AC6 | The page shows the report and the unavailable lines | `layout-panel.test.tsx` | A control whose label is a different function | integrated |

## Approach

Add `layout-report.ts` and a layout section on the example page. Call the existing exports. Do not import `ocelli-volume`.

## Dependencies, write ownership and resources

Depends on F-198, F-197 and F-088, all done. Writes the example page, the sprint ledgers, and `docs/lld/public-api.md`. No second worker. No GPU.

## Verification and evidence reuse

Example vitest files. Feature profile at story close. No wasm size change.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none
- unsafe: none
- Tier A (WebGPU): n/a for these shell calls. This story does not draw a volume
- Tier B (WebGL2): n/a for these shell calls. This story does not draw a volume
- Tier C (CPU): n/a for these shell calls. This story does not draw a volume

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| contract | grid, drag, lines and cursor follow the caller | `examples/viewer-react/src/layout-report.test.ts` |
| contract | the page shows those words and the unavailable lines | `examples/viewer-react/src/layout-panel.test.tsx` |

No new pixel arithmetic. The called functions keep the fixtures they already have.

## Parity surface covered

none. The summary rows in Appendix B are not marked measured.

## Deviations

none

## LLD impact

One paragraph in `docs/lld/public-api.md` naming the example layout section and the four calls. F-203 stays the owner of the unavailable clauses.

## Open questions

none
