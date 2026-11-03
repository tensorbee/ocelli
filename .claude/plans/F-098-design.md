# F-098, DICOM SEG round trip

**Status**: approved
**Epic ref**: E15.5
**Sprint**: S35
**Estimate**: 4w

## Normative source, transcribed

DICOM PS3.3 C.8.20.2 says a segmentation pixel is a segment number and
that 0 is background. F-066 already presents that number through
`present_labelmap`. This story does not call that function and does not
change it.

The dicom-expert note on SEG is the trap this story locks. A SEG frame
carries its own referenced source instance. Frame *n* is not slice *n* of
the series. The mapping is the caller-supplied SOP instance UID on each
frame.

The HLD does not give a Part 10 SEG encoding. A byte file writer is not
this story. F-097 is the later document round trip for SR, not for SEG.

## What the specification does not cover

The HLD does not say which subset of the segmentation IOD is the round
trip. This plan decides the logical round trip, and it is not an HLD
formula.

`encodeSeg` accepts frames in caller order. Each frame has a non-empty
referenced SOP instance UID, integer rows and columns of at least 1, and
one sample per pixel. A sample is an integer of at least 0. Segment 0 is
legal. The function returns a copy. One bad frame refuses the whole call.

`decodeSeg` returns those frames in the same order. It does not sort by
UID and it does not sort by segment number. Two frames may name the same
SOP instance. Both stay, in the order given.

The samples are not a colour. The presenter stays in F-066.

## Problem and reproducer

Reading a SEG as if frame order were series order paints the overlay on
the wrong slice. The round trip must keep the referenced UID on the frame
that carried it.

## Scope, API and compatibility

`crates/ocelli-seg/src/seg_roundtrip.rs` exports `encode_seg` and
`decode_seg`. The module is private to the crate except for the two
functions, which are public. No TypeScript copy.

No change to `labelmap.rs`. No shader change. No new command.

## Consumer and provider map

| Function | Input | Rule | Output |
|----------|-------|------|--------|
| `encode_seg` | ordered frames | samples match rows times columns | a copy, or an error |
| `decode_seg` | that copy | order unchanged | the same frames |

## State transitions and invariants

The functions do not keep state. Decode of encode is the input, including
UID order and segment 0.

## Ownership, commit and recovery

The caller owns the input buffers. The functions return new frames. A
refusal does not write the caller samples.

## Numbered acceptance inventory

1. Two frames, SOP `1.2.9` then SOP `1.2.8`, each 2 by 1, samples `[0, 1]`
   and `[2, 0]`, round-trip in that order. The second frame is not sorted
   ahead of the first.
2. Two frames with the same SOP both remain, in the given order.
3. A sample of `-1`, a non-integer sample, a length other than
   rows times columns, a zero side, or an empty UID is an error.
4. Segment 0 is preserved.

## Approach

Check every frame, then copy the samples into a new `Vec`. Decode copies
again and does not look at a series list.

## Dependencies, write ownership and resources

Depends on F-066, which is done. This story does not call
`present_labelmap`. It writes `seg_roundtrip.rs`, its test, the plan, and
the LLD paragraph.

## Verification and evidence reuse

The feature profile is the completion gate. The functions do not render,
so they do not change CPU candidate bytes.

## Boundary and tier

Pixels do not cross the boundary as image pixels. The samples are segment
numbers. No `wasm-bindgen`. No `unsafe`.

- Tier A (WebGPU): n/a. The presenter is F-066.
- Tier B (WebGL2): n/a
- Tier C (CPU): n/a. The same copy runs with no device.

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| fixture | AC1 through AC4, hand-stated in this plan. No DICOM file bytes. | `crates/ocelli-seg/tests/seg_roundtrip.rs` |

## LLD impact

- `docs/lld/public-api.md` only if a shell name is exported. This plan
  exports no shell name, so the LLD sentence lives in
  `docs/lld/README.md` on the segmentation row if that row exists, and
  otherwise in `docs/lld/public-api.md` as one sentence that the shell
  does not export the round trip.

## Parity surface covered

Appendix B names SEG as an adapter. This story adds the frame-to-SOP
round trip. It does not write a Part 10 file and it does not establish a
rendered parity view.

## Open questions

None. The HLD does not specify the file bytes. The rules above are the
local decision and they are approved with this plan.
