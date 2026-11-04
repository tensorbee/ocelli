# F-196, Key Object Selection and Secondary Capture

**Status**: approved
**Epic ref**: E38.6
**Sprint**: S43
**Estimate**: 4w

## Normative source, transcribed

Decision D10 in `docs/hld/11-decision-log.md` keeps dcmjs for SR generation. The HLD names TID 1500 and does not name TID 2010 or a Secondary Capture writer.

F-097 already writes one TID 1500 document through dcmjs in `packages/core/src/tid1500.ts`. This story follows that bridge.

DICOM PS3.16 TID 2010 is the Key Object Selection template. The Key Object Selection Document SOP class is `1.2.840.10008.5.1.4.1.1.88.59`. PS3.3 A.8.1 is the Secondary Capture IOD. Its SOP class is `1.2.840.10008.5.1.4.1.1.7`.

## What the specification does not cover

The HLD does not map TID 2010 content items onto dcmjs sequences. This plan writes one document title and one referenced composite object, then reads those two fields back. It does not write the full observation context tree.

Secondary Capture here is an 8-bit MONOCHROME2 image whose pixel bytes are the caller's display bytes. It does not window, and it does not copy a patient module. Study, series and instance UIDs are synthetic `2.25` values the caller supplies.

## Problem and reproducer

TID 1500 round-trips. A key-object document and a secondary capture do not. The example page cannot show either.

## Scope, API and compatibility

In `packages/core`:

- `writeKeyObject` and `readKeyObject` for one title and one referenced SOP.
- `writeSecondaryCapture` for columns, rows and an 8-bit monochrome pixel buffer whose length is `columns * rows`.

A mismatched length, an empty title, or an empty UID returns nothing and writes no bytes. Another SR template returns nothing from the reader, as TID 1500 already does.

Out of scope: Rust SR, pixel mapping, and multi-frame secondary capture.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| Example key-object section | `writeKeyObject` then `readKeyObject` | Synthetic UIDs | The title and the referenced SOP |
| Example secondary-capture section | `writeSecondaryCapture` | A 2 by 2 ramp of bytes 0, 1, 2, 3 | A Part 10 blob whose pixel data is those four bytes |

## State transitions and invariants

Stateless. The same inputs produce the same document bytes aside from any dcmjs ordering that the reader treats as a set. The reader checks values, not byte identity of the whole file.

## Ownership, commit and recovery

The functions return a new `Uint8Array` or nothing. They do not replace a stored document in the page until the route returns Ok.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | A written key object reads back the title and the referenced SOP class and instance | `tid2010` test | Dropping the reference | implementation |
| AC2 | A TID 1500 document is not read as a key object | unit test | Accepting template 1500 | implementation |
| AC3 | Secondary capture of bytes 0, 1, 2, 3 reads those pixels back | unit test | Writing a windowed copy | implementation |
| AC4 | A short pixel buffer returns nothing | unit test | Padding with zeros | implementation |
| AC5 | The example page shows the title and the four pixel bytes | route or page test | No control on the page | integrated |

## Approach

Add `tid2010.ts` and `secondary-capture.ts` next to `tid1500.ts`. Reuse dcmjs. Do not add a second SR stack in Rust.

## Dependencies, write ownership and resources

Depends on F-097, done. Writes `packages/core`, the example page, the changelog and the sprint ledgers. No GPU.

## Verification and evidence reuse

`npm test` for the new package tests, or the package's existing test command. Feature profile at story close.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no. The secondary-capture bytes are the caller's already-display buffer, written as a file, not a render-loop transfer
- Render-loop allocation: none
- unsafe: none
- Tier A (WebGPU): n/a
- Tier B (WebGL2): n/a
- Tier C (CPU): n/a

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| fixture | TID 2010 title and reference. SC pixels 0, 1, 2, 3 | `packages/core` tests |

## Parity surface covered

The adapter row that names SR is unchanged. This story adds TID 2010 beside TID 1500. It does not claim SEG or RTSTRUCT.

## Deviations

none

## LLD impact

A short note in `docs/lld/` for the key-object and secondary-capture writers, linked from the LLD index.

## Open questions

none
