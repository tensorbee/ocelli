# F-200, example presets, window drag, region window, rotate, flip, marker, magnifier, colour map, scale length, crop planes and error sentence

**Status**: approved
**Epic ref**: E38.10
**Sprint**: S44
**Estimate**: 4w

## Normative source, transcribed

`docs/hld/05-rendering.md` section 7 says a histogram pass that wants compute is tier A only and must degrade rather than fail. This story does not add that pass.

`docs/hld/07-concurrency-and-typescript.md` section 10 keeps tool interaction state on the TypeScript side. Geometry and hit-testing stay in Rust.

`docs/hld/B-parity-surface.md` lists about 63 tool classes, of which 25 are manipulation and utility, and 3 VOI LUT functions. It has no per-tool row.

`docs/lld/public-api.md` records `windowDrag`, `windowRegion`, `rotationDrag`, `orientationQuadrant`, `orientationMarker`, `magnifyCircle`, `scaleLength`, `cropPlanes`, `packCommands` and `describeError`. The preset names are `soft_tissue`, `lung` and `bone`. The colour maps are `grey` and `stops`. This story calls those exports. It does not add a second window formula.

## What the specification does not cover

The HLD does not name this page section. One to one, a histogram, and four corners have no function on `@ocelli/core` that this page can call without inventing a result. Each line reads unavailable. That work is unscheduled F-204.

Overlay planes and display shutters stay on the existing overlay panel from F-193. The crop line is `cropPlanes`. It is not labelled as a shutter.

## Problem and reproducer

The example page does not call the manipulation exports above. A preset control that changed zoom would be the defect this sprint names.

## Scope, API and compatibility

In scope: one manipulation section. Preset, flip and colour map go through `packCommands` for viewport 1 and the page shows the preset or map name and the packed length. Window, region, rotate, marker, magnifier, scale and crop show the function result. The warning line is `describeError` for `ERROR_CODE.Unavailable` on the page, and the helper accepts the code.

Out of scope: a new LUT, a shader, a wasm export, and a clinical-standard claim for the three presets. The public API already says those presets make no clinical-standard claim.

A refused call reads refused. It does not keep a stale number from a different control.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| Manipulation button | the functions named above | Caller preset, map, drag and samples | Names, packed length, and the numeric results |
| One to one, histogram, four corners | none | No core call | the word unavailable |
| Overlay panel | unchanged F-193 panel | Already on the page | not replaced |

## State transitions and invariants

The button reads the current preset and colour map. Changing the preset does not change the colour map name. A refused window does not clear the last successful scale line from a previous run. The page replaces the whole report together.

## Ownership, commit and recovery

No wasm write. `packCommands` returns bytes the page counts and drops. The error sentence is display text. Nothing matches on it.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | Preset lung and preset bone produce different names and the packed length of that command | `manipulation-report.test.ts` | A preset control that reports zoom | implementation |
| AC2 | A 256 px window drag from centre 40 and width 400 reports centre 41 and width 401 | same | Leaving the basis unchanged | implementation |
| AC3 | Samples 0 and 10 report region centre 5 and width 10 | same | A span of 0 accepted | implementation |
| AC4 | A 512 px rotation from 0 reports 2 radians and quadrant 1 | same | Wrapping quadrant 4 | implementation |
| AC5 | Marker quadrant 1 moves R to the end, magnify null scale is 1, and 10 mm at 2 px/mm is 20 | same | Reading pixels | implementation |
| AC6 | One finite crop plane reports count 1, and the warning text is `describeError` of the caller code | same | Matching on the sentence | implementation |
| AC7 | The page shows the report and the unavailable lines | `manipulation-panel.test.tsx` | Labelling `cropPlanes` as a display shutter | integrated |

## Approach

Add `manipulation-report.ts` and a manipulation section. Reuse `packCommands` as the preset, flip and colour map evidence. Do not submit the buffer to a device.

## Dependencies, write ownership and resources

Depends on F-198, F-193 and F-091, all done. Writes the example page and the ledgers. The overlay panel stays. No GPU.

## Verification and evidence reuse

Example vitest files. Feature profile at story close.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none
- unsafe: none
- Tier A (WebGPU): n/a for these shell calls. Histogram compute is not started
- Tier B (WebGL2): n/a for these shell calls. Histogram compute is not started
- Tier C (CPU): n/a for these shell calls. Histogram compute is not started

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| contract | each control follows its existing function | `examples/viewer-react/src/manipulation-report.test.ts` |
| contract | the page shows the names and the unavailable lines | `examples/viewer-react/src/manipulation-panel.test.tsx` |

No new pixel arithmetic.

## Parity surface covered

none. Appendix B is not marked measured.

## Deviations

none

## LLD impact

One paragraph in `docs/lld/public-api.md` for the manipulation section. F-204 owns one to one, histogram and four corners.

## Open questions

none
