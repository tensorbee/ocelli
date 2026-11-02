# F-071, SVG annotation drawing layer

**Status**: approved
**Epic ref**: E11.4
**Sprint**: S27
**Estimate**: 3w

## Normative source, transcribed

HLD `docs/hld/03-architecture-and-crates.md` section 3:

```text
The shell is TypeScript on the main thread: DOM and pointer events, the SVG annotation layer, tool interaction state, framework bindings, and DICOMweb fetch and authentication.
```

HLD `docs/hld/07-concurrency-and-typescript.md` section 10:

```text
The SVG annotation drawing layer — DOM-native, and faster left there
```

HLD `docs/hld/26-differentiating-capabilities.md` section 34:

```text
The in-memory type is a DICOM SR content-item tree. The drawing layer renders from it; it does not own it.
```

Decision D13 says the annotation type is a DICOM SR content tree. The backlog places that type at F-094, which is not this sprint. F-103 names `annotation.added`, `annotation.completed`, `annotation.modified`, `annotation.removed`, `annotation.selectionChanged`, `annotation.locked`, `annotation.visibilityChanged` and `annotation.rendered`. Delivery is `shell`. Producer is `shell annotation layer`. This story is that producer. The events are not ring records. The catalogue file stays unchanged.

The allocation note is `TS — DOM-native, stays out of wasm`. The epic layer cell says Rust+TS. The note places the drawing in TypeScript. No Rust file is added. Geometry and hit-testing stay in `ocelli-geom`. This story does not project world points.

## What the specification does not cover

The HLD does not name the SVG elements, the display-list fields, or the order of the annotation events. The SR content tree is F-094, so this story has no content-item schema to render. The decisions below are local. They are not a formula the HLD forgot to write.

- The input is a display list of canvas-pixel marks. It is the render input for one call. It is not the annotation type, it is not persisted, and it is not an SR content item. F-094 owns that type.
- `createAnnotationLayer(svg, listener)` binds one SVG element and one listener. There is no module singleton.
- `sync(marks)` validates the whole list, then writes the SVG, then emits. A refusal writes nothing and emits nothing.
- Removals come first, in the previous list's order. Then each mark in the next list's order. A new id emits `annotation.added`. If that new mark is `completed`, `annotation.completed` follows `annotation.added`. The initial `visible`, `locked` and `selected` flags do not emit change events.
- An id that remains emits, in this order, only the differences: `annotation.modified` when `points` or `text` differ, `annotation.completed` when `completed` goes from false to true, `annotation.visibilityChanged`, `annotation.locked`, and `annotation.selectionChanged`. A `completed` change from true to false emits none of those on its own.
- `annotation.rendered` is last, once, and its `ids` are the accepted list in order. An identical list still emits `annotation.rendered` and still rewrites the SVG. It emits no other name.
- Points compare with `===`. There is no epsilon. Text compares with `===`.
- A blank id is `trim().length === 0`. Stored ids are not trimmed. The first bad mark in list order refuses the call. The checks for one mark are structure, then a blank id, then a non-finite coordinate, then a duplicate of an earlier id in that call.
- The SVG uses the SVG namespace. One `g` per mark, in list order. `data-ocelli-id` is the id. `data-selected` and `data-locked` are `true` or `false`. A hidden mark stays in the SVG with `display="none"`. A visible mark does not set `display`. A non-empty point list is one `polyline` whose `points` attribute is `x,y` pairs separated by one space, using `String` on each finite number. A non-null `text` is one `text` element at the last point. Null text adds no `text` element.
- The layer copies the accepted list. A later edit of the caller's array does not change the next diff.
- A throw from the listener is not caught. The SVG write and the stored list already happened.
- This module does not listen and does not import `pointer.ts` or `pointer.js`.

## Problem and reproducer

Nothing in the shell draws an annotation, so the F-103 annotation names have no producer. The observation is a layer call, not a rendered image. `sync` one mark id `a`, points `(1, 2)` and `(3, 4)`, text null, visible, not locked, not selected, not completed. The listener receives `annotation.added` for `a`, then `annotation.rendered` with ids `["a"]`. The SVG has one `g` and one `polyline` whose points are `1,2 3,4`. A second `sync` of that mark with `visible` false receives `annotation.visibilityChanged` then `annotation.rendered`, and the same `g` has `display="none"`.

## Scope, API and compatibility

In scope:

- `packages/core/src/annotation.ts`
- `packages/core/src/annotation.test.ts`
- Re-exports from `packages/core/src/index.ts` and the published-name list
- `docs/lld/public-api.md` gains the drawing-layer section when the story is completed

Out of scope:

- The SR content tree, SNOMED, UCUM, and TID 1500. Those are F-094 and later.
- The 26 annotation tool classes. Those are M8.
- World-to-canvas projection and hit-testing. Those stay in `ocelli-geom`.
- DOM listeners. F-069 owns those.
- Editing `events.ts`, `pointer.ts`, `cursor.ts`, or `tools.ts`.

`createAnnotationLayer(svg, listener): AnnotationLayer`

`AnnotationMark` fields are `id`, `points`, `text`, `visible`, `locked`, `selected` and `completed`. `points` are `{ x, y }` canvas pixels. `text` is a string or null.

`sync` returns `{ ok: true }` or `{ ok: false, refusal }`. Refusals are `blank-id`, `duplicate-id`, `non-finite` and `bad-mark`.

Event fields are `name` and `id` for every name except `annotation.rendered`, which carries `ids`. `annotation.visibilityChanged` also carries `visible`. `annotation.locked` also carries `locked`. `annotation.selectionChanged` also carries `selected`.

## Consumer and provider map

| Consumer | Provider | Policy | Output |
|----------|----------|--------|--------|
| Shell listener | `createAnnotationLayer` | emit only after a successful write | the eight annotation names |
| One SVG element | `sync` | replace its children from the accepted list | one `g` per mark |
| Caller-owned model | the display list | not stored as the annotation type | the next diff |

## State transitions and invariants

| Start | Input | Result | Preserved on refusal |
|-------|-------|--------|----------------------|
| empty | one visible incomplete mark | `added`, then `rendered` | SVG empty, no event |
| that mark | same mark, `visible` false | `visibilityChanged`, then `rendered` | previous SVG, no event |
| that mark | same mark, `completed` true | `completed`, then `rendered` | previous SVG, no event |
| that mark | points change | `modified`, then `rendered` | previous SVG, no event |
| that mark | empty list | `removed`, then `rendered` | previous SVG, no event |
| any | identical list | `rendered` only, SVG rewritten | previous SVG, no event |
| any | duplicate id | refusal `duplicate-id` | previous SVG, no event |

The layer keeps the last accepted list only so the next diff has a previous side. That copy is not the SR tree.

## Ownership, commit and recovery

Validation finishes before any DOM write. The SVG commit is the child replacement. The stored list is replaced in the same successful call, before the listener runs. A throw from the listener does not roll the SVG back. An identical `sync` writes the children again and emits `annotation.rendered` only.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test | Wrong implementation rejected | Stage |
|----|-------------------------------|------|-------------------------------|-------|
| AC1 | A new mark emits `added` then `rendered` | `a new mark is added and drawn` | drawing without `added`, or `rendered` first | implementation |
| AC2 | A new completed mark emits `completed` after `added` | `a new completed mark completes` | skipping `completed`, or emitting it first | implementation |
| AC3 | Initial flags do not emit change events | `the initial flags are not changes` | a visibility event on insert | implementation |
| AC4 | A point or text change emits `modified` | `a point change is modified` | treating a move as a new id | implementation |
| AC5 | False to true `completed` emits `completed` | `completion is a rising edge` | emitting it again on the next sync | implementation |
| AC6 | A visibility change keeps the group and sets `display` | `a hidden mark stays in the svg` | removing the group | implementation |
| AC7 | Lock and selection changes emit those names | `lock and selection are separate` | folding them into `modified` | implementation |
| AC8 | Removal follows the previous order | `removal keeps the previous order` | sorting the ids | implementation |
| AC9 | An identical sync emits only `rendered` and rewrites | `an identical sync still draws` | skipping the write | implementation |
| AC10 | A refusal emits nothing and keeps the SVG | `a refusal is silent` | drawing the bad list | implementation |
| AC11 | The first bad mark in order wins | `the first bad mark wins` | scanning for a later duplicate first | implementation |
| AC12 | A caller edit after `sync` does not affect the next diff | `the layer keeps its own copy` | diffing the caller's array | implementation |
| AC13 | The source does not call `addEventListener` | `the annotation module does not listen` | a pointer listener in this file | implementation |

## Approach

`packages/core/src/annotation.ts` validates, diffs, writes SVG nodes with `createElementNS`, and emits. It imports nothing from `pointer.ts`, `cursor.ts`, or `tools.ts`.

Rejected: an SR content-item type in this story, a module-singleton layer, removing hidden marks from the DOM, and listening for pointer events inside this file.

## Dependencies, write ownership and resources

Depends on F-069, which is done. This story writes `annotation.ts`, `annotation.test.ts`, `index.ts` and `index.test.ts`. No shader and no GPU. The feature profile still runs later because the ledgers change.

## Verification and evidence reuse

`npm test`, `npm run typecheck` and `npm run lint` for the focused loop. No wasm rebuild is required for this story alone. `/complete-feature` still runs the feature profile. A later edit to `annotation.ts` invalidates that receipt.

No pixel fixture and no geometry fixture. Points are not projected here.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none. This module is not on the frame path
- unsafe: none
- Tier A (WebGPU): n/a
- Tier B (WebGL2): n/a
- Tier C (CPU): n/a

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| unit | the acceptance rows above | `packages/core/src/annotation.test.ts` |
| fixture | none. No pixel arithmetic | |

## Parity surface covered

None of the tool-class rows. The shell annotation producer for the eight annotation names is this module. The catalogue file is not edited.

## Deviations

none

## LLD impact

`docs/lld/public-api.md` gains a drawing-layer section. No new LLD file.

## Open questions

None. The operator declined a design-question round on 2026-10-31. The display list and the emission order are decided in this plan.
