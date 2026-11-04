# F-201, example measurement document, annotation list, undo, TID 1500, key image mark, shortcut and touch

**Status**: approved
**Epic ref**: E38.11
**Sprint**: S44
**Estimate**: 4w

## Normative source, transcribed

`docs/hld/07-concurrency-and-typescript.md` section 10 keeps the SVG annotation layer in the DOM, tool state machines in TypeScript, and the dcmjs bridge for TID 1500 in TypeScript. Pointer, touch and wheel stay in TypeScript.

Decision D13 says the annotation type is a DICOM SR content tree. `docs/lld/public-api.md` records `persistMeasurements`, `loadMeasurements`, `putAnnotation`, `annotationsForFrame`, `undoAnnotation`, `writeTid1500`, `readTid1500`, `keyImageMark` and `keyAction`. `createPointerDispatch` normalises a touch pointer down to `touch.start`. This story calls those exports.

`docs/hld/B-parity-surface.md` lists 26 annotation tool classes and the SR TID 1500 adapter among 4 adapters. It has no per-tool row.

## What the specification does not cover

The HLD does not name this page section or a preferences file. A metadata browser with compare and copy has no shell function this page can call without parsing DICOM tags in TypeScript, which the F-198 plan already refuses. Those lines read unavailable. The work is unscheduled F-205.

Key object and secondary capture export stay on the existing document panel from F-196. This story adds `keyImageMark`. It does not add a second writer.

## Problem and reproducer

The example page does not round-trip a measurement document, an annotation undo stack, a TID 1500 number, a key image mark, a shortcut, or a touch start.

## Scope, API and compatibility

In scope: one measurement section. The synthetic item is one NUM, 12.5 mm, concept scheme SCT, value 410668003, meaning Length. The study, series and instance identifiers are synthetic 2.25 values. The frame of reference on the annotation is a synthetic 2.25 value. The shortcut default is ArrowUp with shift, which `keyAction` names `scroll-previous`. The touch control ingests one pointer down with pointer type touch and shows the event name.

Out of scope: SCOORD mapping, a patient module, a second key-object writer, and parsing tags in the page. A non-finite number is refused by the content tree and the lines read refused.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| Measurement button | persist, load, annotation state, TID 1500, `keyImageMark`, `keyAction`, `createPointerDispatch` | One synthetic NUM and one touch down | The number, the list count, undo count, the TID number, KEY, the shortcut, `touch.start` |
| Metadata browser and preferences | none | No core call | the word unavailable |
| Document panel | unchanged F-196 panel | Already on the page | not replaced |

## State transitions and invariants

Each button run starts from `createAnnotationState`. Undo of that one put returns an empty list. A refused document does not leave the previous TID bytes on the page, because the whole report is replaced. The shortcut does not change the measurement number.

## Ownership, commit and recovery

The page holds the report in React state. TID bytes are counted by the read-back and then dropped. No wasm session is created. A second click builds a new annotation state. It does not append to the previous one.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | 12.5 mm persists and loads as 12.5 | `measurement-report.test.ts` | Parsing a DICOM tag in the page | implementation |
| AC2 | A non-finite number is refused | same | Storing a quiet zero | implementation |
| AC3 | One put lists 1 annotation and undo lists 0 | same | Undo that keeps the put | implementation |
| AC4 | TID 1500 reads the same number back | same | An operator name shown as the measurement | implementation |
| AC5 | The key image text is KEY, ArrowUp with shift is scroll-previous, and the touch event is touch.start | same | A mouse down named as touch | implementation |
| AC6 | The page shows the report and the unavailable lines | `measurement-panel.test.tsx` | A second secondary-capture writer | integrated |

## Approach

Add `measurement-report.ts` and a measurement section. Reuse the existing document writers only by leaving their panel in place.

## Dependencies, write ownership and resources

Depends on F-198, F-195 and F-196, all done. Writes the example page and the ledgers. No GPU.

## Verification and evidence reuse

Example vitest files. Feature profile at story close. dcmjs stays the TID 1500 writer inside `@ocelli/core`.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none
- unsafe: none
- Tier A (WebGPU): n/a
- Tier B (WebGL2): n/a
- Tier C (CPU): n/a

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| contract | the document, the list, undo, TID 1500, the mark, the shortcut and the touch name | `examples/viewer-react/src/measurement-report.test.ts` |
| contract | the page shows them and does not add a second export writer | `examples/viewer-react/src/measurement-panel.test.tsx` |

No new pixel arithmetic.

## Parity surface covered

none. Appendix B is not marked measured.

## Deviations

none

## LLD impact

One paragraph in `docs/lld/public-api.md` for the measurement section. F-205 owns the metadata browser and preferences.

## Open questions

none
