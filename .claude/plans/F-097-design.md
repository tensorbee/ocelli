# F-097, DICOM SR TID 1500 round-trip

**Status**: approved
**Epic ref**: E15.4
**Sprint**: S36
**Estimate**: 3w

## Normative source, transcribed

Decision D10 in `docs/hld/11-decision-log.md` says keep dcmjs for SR
generation. The rejected alternative is reimplementing TID 1500 in Rust.

`docs/hld/07-concurrency-and-typescript.md` section 10 places the dcmjs
bridge for TID 1500 structured reports in TypeScript.

Decision D13 says the annotation type is a DICOM SR content tree.

PS3.16 TID 1500 is the measurement report template. The root concept used
by that template is DCM code 126000, Imaging Measurement Report. The
template identifier is 1500. DCM code 126010 is Imaging Measurements.
DCM code 125007 is Measurement Group.

The HLD does not transcribe those rows. They are the standard, not an
HLD formula. This story does not invent a different template number.

## What the specification does not cover

The HLD does not say which TID 1500 rows a measurement fills, which SOP
class carries the document, or how a content-tree `NUM` maps onto
`MeasuredValueSequence`.

This plan decides that mapping. It is not an HLD formula.

`writeTid1500` accepts content trees and one source identity. The source
has four non-empty strings: study, series, sop class, and sop instance.
Each root must pass `contentTree` and must contain one `NUM` node, the
root or a descendant. A tree with no `NUM`, or with two, returns null.
The count of every node, using the same local threshold as F-096, refuses
at 4000.

The writer calls `dcmjs` `TID1500MeasurementReport` for the template
shell. Each measurement becomes one measurement group whose content item
is a `NUM`. The concept is `ConceptNameCodeSequence`. The number is
`NumericValue`. The unit is the UCUM code value. The referenced SOP is
the source sop class and sop instance. `DicomDict.write` produces the
bytes. A dcmjs throw returns null.

The document SOP class is Comprehensive 3D SR,
`1.2.840.10008.5.1.4.1.1.88.34`. The transfer syntax is Explicit VR
Little Endian, `1.2.840.10008.1.2.1`. The template identifier is the
string `1500`. These three are the local document choices.

`readTid1500` uses `DicomDict.readFile`. A template other than `1500`
returns null. Each measurement group must contain one `NUM`. The reader
returns those `NUM` items as roots, in group order, with a null
relationship and no children. The number, the unit, and the three concept
strings survive. Language, procedure, and the observer are template shell
and are not returned as measurements.

An operator TEXT whose concept is scheme `OCELLI` and value `operator`
sets the dcmjs person name. The reader does not return that name.

There is no Rust twin. Part 10 is the byte buffer dcmjs writes. This
story does not add a second encoder. It does not import cornerstone.

`dcmjs` at 0.52.2 is a dependency of `@ocelli/core`. It is not in the
tree today. D10 names it.

## Problem and reproducer

The content tree cannot leave the process as a TID 1500 document. No
module imports dcmjs.

## Scope, API and compatibility

`packages/core/src/tid1500.ts` exports `writeTid1500` and `readTid1500`.
`index.ts` exports those two values.

No edit of `annotation.ts`. No measurement arithmetic. No change to the
F-096 JSON document.

## Consumer and provider map

| Function | Input | Rule | Output |
|----------|-------|------|--------|
| `writeTid1500` | roots and a source | one `NUM` per root, dcmjs accepts the dataset | bytes, or null |
| `readTid1500` | bytes | template `1500`, one `NUM` per group | copied `NUM` roots, or null |

## State transitions and invariants

The functions are stateless. A refusal returns null. Two writes of the
same measurement may differ in generated UIDs. The read measurement does
not differ.

## Ownership, commit and recovery

Not applicable. dcmjs holds the bytes only for the call. There is no
device and no file in the repository.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test | Wrong implementation rejected |
|----|-------------------------------|------|-------------------------------|
| AC1 | `NUM` 12.5 with unit `mm` reads back as 12.5 and `mm`, with the same concept. | `tid1500.test.ts` | a writer that drops the unit |
| AC2 | The bytes name template `1500` and SOP class `1.2.840.10008.5.1.4.1.1.88.34`. | the same file | a private JSON document |
| AC3 | A tree `contentTree` refuses, a root with no `NUM`, and an empty source string return null. | the same file | a document written anyway |
| AC4 | Template `1499` reads as null. | the same file | accepting any SR |
| AC5 | Two measurements read back in caller order. | the same file | a sort |

## Approach

Build the shell with `TID1500MeasurementReport`. Append one measurement
group per root. Write and read the buffer with `DicomDict`. Map the
`NUM` fields in one function used by the writer and checked by the reader
through `contentTree`.

## Dependencies, write ownership and resources

Depends on F-095, which is done, and uses the F-094 tree. It writes
`tid1500.ts`, its test, `packages/core/package.json`, the lockfile, the
plan, and the LLD paragraph. It does not take the GPU.

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
| fixture | AC1 through AC5, hand-stated in this plan. | `packages/core/src/tid1500.test.ts` |

## LLD impact

- `docs/lld/public-api.md`
- the `public-api.md` row in `docs/lld/README.md`

## Parity surface covered

Appendix B names SR TID 1500. This story writes and reads that template
through dcmjs. It does not establish a rendered parity view. It does not
claim the document is every optional TID 1500 row.

## Open questions

None. The `NUM` mapping, the SOP class, and the dcmjs package version are
the local decision and they are approved with this plan.
