# F-193 review, pass 3

**Reviewed**: uncommitted working tree on sprint/s42 against 98fd152, after pass 2
**Author / reviewer**: implementation by the sprint host. Review by Claude Code, read only, with no shell.
**Result**: 0 defects, 0 smells, 3 nitpicks

## Acceptance challenged

- **Pass 2 smell 1, polygon transposition.** Closed. I traced `overlay_asymmetric_polygon_keeps_one_side` (`crates/ocelli-dicom/tests/overlay.rs:177`) against `polygon_contains` (`crates/ocelli-dicom/src/overlay.rs:944`). The vertices `1\1\1\4\3\1` become the xy edges `(1,1)-(4,1)`, `(4,1)-(1,3)` and `(1,3)-(1,1)`.
  - Correct code, `hides(1, 4)`: the point is `(4, 1)`, which lies on edge 0. `on_segment` returns true, so the pixel is open and `!hides(1, 4)` holds.
  - Correct code, `hides(4, 1)`: the point is `(1, 4)`. It is not on any edge, because the only collinear edge, `(1,3)-(1,1)`, has y in `[1, 3]`. No edge straddles y 4, so the ray count is 0 and the pixel is hidden.
  - Mutation, point built as `(row, column)`: the two probes swap points. `hides(1, 4)` now tests `(1, 4)` and is hidden, so `!hides(1, 4)` goes red. That matches the implementer's report. `hides(4, 1)` now tests `(4, 1)`, which is on edge 0, so it goes red as well.
  - Mutation, `xy` swap dropped in `edge` (`:941`): this reflects the edges instead of the point. The two probes flip the same way.
  - I re-traced `(2, 2)` under the correct code. Edge 1 crosses y 2 at x 2.5, which is right of the point, and edge 2 crosses at x 1, which is left of it. That gives one crossing, so the pixel is inside, and the existing assertion still holds.
  - PS3.3 C.7.6.11 gives `(0018,1620)` as row then column pairs, so `(column, row)` is the right orientation for the point.
- **Pass 2 smell 1, multi-frame stride.** Closed. In `overlay_second_frame_uses_its_own_bits` (`tests/overlay.rs:203`), `pixels` is 2, `total` is 4, `need` is 1, and the data is 2 bytes. Byte 0 is `0b100`.
  - Frame 0: offset 0, so indices 0 and 1 read bits 0 and 1, and both are clear.
  - Frame 1: offset 2, so index 0 reads bit 2, which is set, and index 1 reads bit 3, which is clear.
  - Each mutation from pass 2 now goes red on `bit(1, 1)` for frame 1. `offset = plane_index` reads bit 1. A stride of 8 bits reads byte 1, which is 0. An offset past the data reads nothing. A constant offset of 0 reads bit 0. All four are clear.
- **Pass 2 nitpick 1, plan and LLD.** Closed. `docs/lld/overlays.md:46` now says that the design text wanted the value in the preview and that the page prints it as its own sentence. `examples/viewer-react/src/App.tsx:559` to `:563` prints "Neutral value N." or "The neutral value is absent." and never folds the value into the mask digits. `.claude/plans/F-193-design.md:24` is unchanged, which is what the implementer said.
- **Pass 2 nitpick 2, `Polygon` rustdoc.** Closed for the four causes pass 2 named (`overlay.rs:88`). See nitpick 2 for one cause it still leaves out.
- **Pass 2 nitpick 4, label for an unexpected VR.** Half closed. `packed_bytes` now falls through to `Metadata` (`:876`). The 16-bit arm still uses `BitsAllocated` for a non-OW VR. See nitpick 1.
- **Pass 2 nitpick 5, bitmap shutter past the image.** Closed. The `Bitmap` arm of `shape_hides` (`:858`) now hides a row or column past `image_rows` or `image_columns` before it consults the clipped `plane.bit`. Without that branch, `bit(1, 2)` on a 1 by 1 plane is false, so `hides(1, 2)` would be false and `overlay_bitmap_ow_follows_the_low_bit` (`tests/overlay.rs:162`) would go red. So the new assertion does test the new branch. A row or column below 1 is still caught first by `Shutter::hides` (`:265`). The example server only queries `hides(1, 1)` (`examples/example-server/src/overlay.rs:52`), so its output does not change.
- **Pass 2 nitpick 3.** Not addressed and not claimed. It carries over as nitpick 3.

## Evidence

- I read pass 2 and the current `crates/ocelli-dicom/src/overlay.rs` and `crates/ocelli-dicom/tests/overlay.rs` in full, plus `docs/lld/overlays.md`.
- I grepped `.claude/plans/F-193-design.md` for the presentation wording, `App.tsx` for how the neutral value is rendered, and the example server's `overlay.rs` for its shutter and presentation use.
- I did not re-read `/tmp/ocelli-f193.diff` in full. Every change claimed in this pass is in a file I read in its current state.
- I counted 25 `#[test]` functions in the fixture file, which matches the implementer's count.
- I traced every new or changed assertion by hand under the correct code and under each named mutation.
- I ran nothing. The test result and the clippy result are the implementer's. Clippy was run before the final assertion edits. Those edits are plain `assert!` calls and I expect no lint, but the verify gate has to run on the final tree anyway, because the provenance trailer is keyed to the tree hash.

## Defects

None.

## Smells

None.

## Nitpicks

1. **`crates/ocelli-dicom/src/overlay.rs:613` and `:617`.** A 16-bit Overlay Data whose VR is not OW, or whose value is not `Unsigned16`, is still refused as `BitsAllocated`. Under the rustdoc at `:80` and `:86` that is a `Metadata` case. The refusal is correct and only the label is wrong. This is the other half of pass 2 nitpick 4.
2. **`crates/ocelli-dicom/src/overlay.rs:88`.** `is_values` is called with `OverlayReason::Polygon` (`:783`). So a `(0018,1620)` that is not IS, or does not parse as integers, is also refused as `Polygon`, and the rustdoc does not name that cause. The same is true of `Shutter` for the other IS shutter attributes, but "not a usable shape" (`:90`) covers those loosely.
3. **`crates/ocelli-dicom/tests/overlay.rs:189`.** Carried from pass 2 nitpick 3. `overlay_zero_length_edge_is_an_error` stays green if the pre-check loop at `overlay.rs:882` is deleted, because `adjacent_overlap` returns true when `edge` is `None` (`:910`). The behaviour is guarded twice. Only a mutation claim against that loop would be false.

## Verified clean

- `packed_bytes` (`:866`) accepts only `(OB, Bytes)` and `(OW, Unsigned16)`, and everything else is `Metadata`.
- In the `Bitmap` arm of `shape_hides`, the `bitmap` that is `None` arm (`:855`) fails closed, and `read_shutter` always sets it whenever it pushes a `Bitmap` shape.
- The past-the-image check compares against the plane's own `image_rows` and `image_columns`. Those are the image's values, because `bitmap_shutter` requires the plane's size to equal the image's (`:827`).
- Both new polygon probes discriminate on their own, so dropping either one still leaves the transposition pinned.
- The multi-frame fixture declares Number of Frames 2 and Number of Frames in Overlay 2, and `plane_index` gives 0 and 1 with the default Image Frame Origin of 1.
- The LLD's polygon and bitmap paragraphs (`:38`, `:40`) still describe the code.
- I found no em dash and no prose semicolon in the changed rustdoc, the LLD, or the test comments.
