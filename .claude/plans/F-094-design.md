# F-094, DICOM SR as the native in-memory annotation model

**Status**: approved
**Epic ref**: E15.1
**Sprint**: S35
**Estimate**: 4w

## Normative source, transcribed

Decision D13 in `docs/hld/11-decision-log.md` says the annotation type is a
DICOM SR content tree. The rejected alternative is an internal model with
SR adapters.

HLD section 34 says the in-memory type is a DICOM SR content-item tree.
The drawing layer renders from it and does not own it. Concepts are coded
with SNOMED CT and UCUM. Provenance on every annotation names the tool,
the engine version, the algorithm and version where one was involved, and
the operator.

Decision D10 keeps dcmjs for SR generation. TID 1500 round trip is F-097
and is not this story.

The allocation note says dwv is GPL-3.0 and must not be read. The idea is
taken from DICOM PS3.3 and PS3.16. dwv, Horos, and the medical viewer Grok
must not be opened.

## What the specification does not cover

The HLD does not give a content-item field layout, a relationship list, or
a value-type list. This plan decides that layout. It is not an HLD formula
and it is not a TID 1500 template.

One content item has a relationship, a value type, a concept, one payload,
and an ordered child list. The root relationship is absent. A child
relationship is one of `CONTAINS`, `HAS_OBS_CONTEXT`, `HAS_PROPERTIES`,
`INFERRED_FROM`, and `SELECTED_FROM`. The value type is one of
`CONTAINER`, `TEXT`, `NUM`, `CODE`, `UIDREF`, `IMAGE`, `SCOORD`, and
`SCOORD3D`.

A concept is three non-empty strings: scheme, value, and meaning. A `NUM`
payload is a finite number and a non-empty UCUM unit string. The function
does not check the unit against a UCUM table. An `SCOORD` point is two
finite numbers. An `SCOORD3D` point is three finite numbers and the item
carries a non-empty frame-of-reference UID. `POINT` has one point.
`POLYLINE` has at least two points. `CIRCLE` has two points.

`contentTree` checks the whole tree and returns a deep copy. One bad node
returns nothing. The caller object is not written.

`provenanceItems` returns four `HAS_OBS_CONTEXT` text children, in this
order: tool, engine version, algorithm, operator. An empty string returns
nothing. Algorithm may be the empty string only when the caller passes
null, and that child is then omitted. The other three are required.

The drawing layer in `annotation.ts` is not edited. It already says it
does not own this tree.

There is no Rust twin. A second struct would be the internal model D13
rejects. F-097 owns the dcmjs bytes.

## Problem and reproducer

A display-list mark is canvas pixels. A measurement stored only as that
mark cannot leave the viewer. The content tree is the annotation.

## Scope, API and compatibility

`packages/core/src/sr.ts` exports `contentTree` and `provenanceItems`.
`index.ts` exports those two values. The concept type is not a runtime
value.

No command kind. No registry row. No change to `annotation.ts`.

## Consumer and provider map

| Function | Input | Rule | Output |
|----------|-------|------|--------|
| `contentTree` | one root item | whole tree valid | a deep copy, or null |
| `provenanceItems` | tool, engine, algorithm, operator | three required texts | child items, or null |

## State transitions and invariants

The functions do not keep state. A returned tree does not share objects
with the caller.

## Ownership, commit and recovery

The caller owns the input. The function owns the copy. A refusal returns
null and writes nothing.

## Numbered acceptance inventory

1. A container root with one `CONTAINS` `NUM` child of 12.5 and unit `mm`
   copies both nodes. The caller child is a different object.
2. A `POINT` `SCOORD` of `(1, 2)` stays those two numbers.
3. A `CIRCLE` with one point returns null. A non-finite number returns
   null. An empty concept value returns null.
4. Provenance with tool `Length`, engine `0.1.0`, algorithm null, and
   operator `reader` returns three text children. The algorithm child is
   absent. An empty tool returns null.

## Approach

Walk the tree once. Check the discriminant, then the payload, then the
children. Copy on the way out.

## Dependencies, write ownership and resources

Depends on F-070, which is done. This story does not call a measurement
function. It writes `sr.ts`, its test, the plan, and the LLD paragraph.

## Verification and evidence reuse

The feature profile is the completion gate. These functions do not change
CPU candidate bytes.

## Boundary and tier

Pixels do not cross the boundary. No `wasm-bindgen`. No `unsafe`.

- Tier A (WebGPU): n/a. The tree is not a texture.
- Tier B (WebGL2): n/a
- Tier C (CPU): n/a

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| fixture | AC1 through AC4, hand-stated in this plan. | `packages/core/src/sr.test.ts` |

## LLD impact

- `docs/lld/public-api.md`
- the `public-api.md` row in `docs/lld/README.md`

## Parity surface covered

Appendix B names SR TID 1500 as an adapter. This story adds the in-memory
tree. It does not write a TID 1500 document and it does not establish a
rendered parity view.

## Open questions

None. The HLD does not specify the node fields. The rules above are the
local decision and they are approved with this plan.
