# F-095, annotation state, frame-of-reference index, undo and redo

**Status**: approved
**Epic ref**: E15.2
**Sprint**: S35
**Estimate**: 4w

## Normative source, transcribed

HLD section 35 says a slide layer and a volume layer share the annotation
coordinate model through the frame of reference. DICOM `FrameOfReferenceUID`
(0020,0052) is that identifier. Two series share a spatial frame only when
they share it.

Decision D13 says the annotation type is the SR content tree from F-094.
This story stores those trees. It does not define a second annotation type.

The HLD does not define an undo stack.

## What the specification does not cover

The HLD does not say how undo is stored. This plan decides that, and it is
not an HLD formula.

`createAnnotationState` returns an empty state. `putAnnotation` appends one
record or replaces the same id. The record holds an id, a frame-of-reference
UID, and a content tree that has already passed `contentTree`. A blank id,
a blank UID, or a null tree returns null and does not change the returned
state. The function returns a new state.

`annotationsForFrame` returns the records whose UID is exactly the caller
UID, in insertion order. A different UID is absent. The list is a copy of
the record objects.

`undoAnnotation` returns the state from before the latest successful put.
The first state has no undo and returns null. `redoAnnotation` returns the
state that undo left. A successful put after undo drops the redo states.
Undo and redo do not compute a measurement.

## Problem and reproducer

Two series can share a frame of reference and still keep separate
annotation lists if the UID is ignored. A put that cannot be undone loses
the previous tree.

## Scope, API and compatibility

`packages/core/src/annotation-state.ts` exports `createAnnotationState`,
`putAnnotation`, `annotationsForFrame`, `undoAnnotation`, and
`redoAnnotation`. It calls `contentTree` from `sr.ts` and does not
reimplement the tree check.

No command kind. No registry row. `annotation.ts` is not edited.

## Consumer and provider map

| Function | Input | Rule | Output |
|----------|-------|------|--------|
| `putAnnotation` | id, UID, tree | tree already valid | a new state, or null |
| `annotationsForFrame` | UID | exact UID match | records in order |
| `undoAnnotation` | state | one step back | a state, or null |
| `redoAnnotation` | state | one step forward | a state, or null |

## State transitions and invariants

Each successful put is one history step. Undo and redo move one step. The
current records and the history do not share tree objects with the caller.

## Ownership, commit and recovery

A refused put returns null. The previous state object is unchanged because
the function never writes it.

## Numbered acceptance inventory

1. Two puts on UID `1.2.3` and one put on UID `1.2.4` list two records for
   `1.2.3` and one record for `1.2.4`.
2. Replacing id `a` keeps one record and the new number 4.
3. Undo after two puts restores the one-record state. A second undo
   restores empty. A third undo returns null.
4. Redo after one undo restores the latest put. A new put after undo makes
   redo return null.
5. A blank id returns null. A tree that `contentTree` refuses returns null.

## Approach

The state holds the current records and two stacks of record-list copies.
Put copies the list before appending.

## Dependencies, write ownership and resources

Depends on F-070, which is done, and uses the F-094 tree. It writes
`annotation-state.ts`, its test, the plan, and the LLD paragraph. It does
not take the GPU.

## Verification and evidence reuse

The feature profile is the completion gate. This story changes no CPU
candidate bytes.

## Boundary and tier

Pixels do not cross the boundary. No `wasm-bindgen`. No `unsafe`.

- Tier A (WebGPU): n/a
- Tier B (WebGL2): n/a
- Tier C (CPU): n/a

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| fixture | AC1 through AC5, hand-stated in this plan. | `packages/core/src/annotation-state.test.ts` |

## LLD impact

- `docs/lld/public-api.md`
- the `public-api.md` row in `docs/lld/README.md`

## Parity surface covered

Appendix B names the annotation classes and does not define this index.
This story adds the frame-of-reference list and the undo step. It does not
establish a rendered parity view.

## Open questions

None. The HLD does not specify the undo stack. The rules above are the
local decision and they are approved with this plan.
