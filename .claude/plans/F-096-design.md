# F-096, measurement persistence and serialisation format

**Status**: approved
**Epic ref**: E15.3
**Sprint**: S36
**Estimate**: 3w

## Normative source, transcribed

`docs/hld/26-differentiating-capabilities.md` section 34 says the
in-memory type is a DICOM SR content-item tree. The drawing layer renders
from it and does not own it. Concepts use SNOMED CT and UCUM. There are
two tiers. SR is for semantics. A bulk binary representation is for object
counts past a few thousand. Provenance on every annotation names the tool,
the engine version, the algorithm and version where one was involved, and
the operator.

Decision D13 says the annotation type is that tree. The rejected
alternative is an internal model with adapters.

Decision D10 keeps dcmjs for SR generation. TID 1500 bytes are F-097.

F-094 already defined the content-item fields. This story persists that
tree. It does not define a second one.

## What the specification does not cover

Section 34 does not give a byte layout for the SR tier or for the bulk
tier. It does not give the count in "a few thousand".

This plan decides both gaps. They are not HLD formulas.

The document is JSON text. The object has `format` equal to
`ocelli-measurement`, `version` equal to 1, and `items` as an array of
content trees. `persistMeasurements` runs `contentTree` on every root
before it writes. One refusal returns null. The caller array and the
caller nodes are not written.

The item count is every node, roots and nested children. The local bulk
threshold is 4000. A count of 4000 or more returns null from both
functions. That is unavailable. This story does not define a bulk binary
layout.

`loadMeasurements` parses the text and runs `contentTree` on every item.
A wrong format, a wrong version, a non-object, or one bad node returns
null. An empty `items` array is valid and loads as a new empty array.

Order is the caller order. There is no Rust twin.

## Problem and reproducer

A content tree that exists only in memory cannot be stored. There is no
persistence function in `packages/core` today.

## Scope, API and compatibility

`packages/core/src/measurement-persist.ts` exports `persistMeasurements`
and `loadMeasurements`. `index.ts` exports those two values. The limit
4000 is not a published runtime name.

No command kind. No edit of `annotation.ts`. No dcmjs import. No
measurement arithmetic.

## Consumer and provider map

| Function | Input | Rule | Output |
|----------|-------|------|--------|
| `persistMeasurements` | root items | every root passes `contentTree`, count below 4000 | JSON text, or null |
| `loadMeasurements` | JSON text | format 1 and every item passes `contentTree` | copied roots, or null |

## State transitions and invariants

The functions are stateless. A refusal writes nothing and returns null.
A second call with the same items returns a document that loads to the
same copied trees.

## Ownership, commit and recovery

Not applicable. There is no device, no file handle, and no cache. The
returned string is the whole document.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test | Wrong implementation rejected |
|----|-------------------------------|------|-------------------------------|
| AC1 | Two valid roots load in order. A nested text is not the caller string. | `measurement-persist.test.ts` | a shallow copy, or a sort |
| AC2 | One bad node returns null. | the same file | a document that drops the bad node |
| AC3 | 3999 nodes persist. 4000 nodes return null. | the same file | a second binary format |
| AC4 | Wrong format, wrong version, and a non-object return null. | the same file | a partial load |
| AC5 | An empty list loads as a new empty array. | the same file | returning the caller array |

## Approach

Copy through `contentTree`, count the copied nodes, then
`JSON.stringify` the document. Load parses, checks the header, and copies
again through `contentTree`.

## Dependencies, write ownership and resources

Depends on F-094, which is done. It writes `measurement-persist.ts`, its
test, the plan, and the LLD paragraph. It does not take the GPU.

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
| fixture | AC1 through AC5, hand-stated in this plan. | `packages/core/src/measurement-persist.test.ts` |

## LLD impact

- `docs/lld/public-api.md`
- the `public-api.md` row in `docs/lld/README.md`

## Parity surface covered

Appendix B names SR TID 1500 and does not define this JSON document.
This story adds the content-tree document. It does not establish a
rendered parity view. TID 1500 bytes are F-097.

## Open questions

None. The JSON document and the count of 4000 are the local decision and
they are approved with this plan.
