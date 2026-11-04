# F-193 review, pass 2

**Reviewed**: uncommitted working tree on sprint/s42 against 98fd152, after pass 1
**Author / reviewer**: implementation by the sprint host. Review by Claude Code, read only, with no shell.
**Result**: 0 defects, 1 smell, 5 nitpicks

## Acceptance challenged

- **Pass 1 defect 1, OW 1-bit overlay.** Closed. `packed_bytes` (`crates/ocelli-dicom/src/overlay.rs:863`) takes `(VR::OW, Unsigned16)` and expands each word with `to_le_bytes`. A word's low 8 bits become byte 0, so word bit k is pixel k. That matches the OB packing for any byte order the parser decodes into native words. `bitmap_shutter` only asks for `PlaneBits::Packed`, so an OW bitmap is accepted too. I traced `overlay_one_bit_ow_is_packed_low_bit_first`. Word `0x0002` becomes bytes `02 00`, and index 1 on a 2 by 2 plane is row-major `(1, 2)`. Under `to_be_bytes` the bytes are `00 02`, index 1 is clear, and the test goes red. The same test also catches column-major indexing, because column-major index 1 is `(2, 1)`. I also checked the Implicit VR case that pass 1 named. `dicom-dictionary-std` 0.10.0 gives `OverlayData` the VR `Ox`, and `dicom-core` resolves `Ox` to `VR::OW`, so implicit objects reach the same arm.
- **Pass 1 defect 2, zero-length edge.** Closed. `1\1\1\1\3\3` makes `edge(vertices, 0)` return `None`, and the pre-check at `:879` refuses it as `Polygon`. See nitpick 3 about what the fixture isolates.
- **Pass 1 smell 3, folded adjacent edges.** Closed. `adjacent_overlap` (`:906`) refuses a pair only when both far endpoints are collinear with the first edge and the shared-line projections overlap by more than zero. I traced three cases:
  - `1\1\1\4\1\2`: edge 0 spans x `[1,4]` and edge 1 spans `[2,4]`, an overlap of 2, so it is refused.
  - A straight collinear continuation such as `(1,1)` to `(1,2)` to `(1,4)` overlaps by `2 - 2 = 0`, so it is kept, which is correct.
  - Pass 1's four-vertex `1\1\1\4\1\2\3\2` is now refused on the pair (0, 1).

  A vertical pair projects to zero on x, so the y projection carries it, and a diagonal pair overlaps on both axes. The wrap-around pair reaches the check through `left == right_end`.
- **Pass 1 smell 2, `bit` past the image.** Closed. `bit` (`:165`) now clips to `image_rows` and `image_columns`, so the stored fields are read. `overlay_bit_past_the_image_stays_outside` would have returned true for `bit(1, 2)` before the fix. The bitmap shutter is unaffected because its plane must match the image size.
- **Pass 1 smell 1, transposition and multi-frame.** Partly closed. Origin, circle centre, row-major order and Image Frame Origin are now pinned. The polygon transposition and the multi-frame stride are not. See the smell.
- **Pass 1 nitpicks 1 to 3.** Closed. The `Shutter` rustdoc says "outside any geometric opening" (`:247`). `shutter` and `presentation_value` carry `#[must_use]` with a message (`:295`, `:301`). The CHANGELOG line now separates overlay groups from shutter attributes.
- **Pass 1 nitpick 4.** Half closed. The LLD (`docs/lld/overlays.md:46`) now describes the page as built. The approved plan still says otherwise (nitpick 1).

## Evidence

- I read `/tmp/ocelli-f193.diff` in full, the current `crates/ocelli-dicom/src/overlay.rs`, `crates/ocelli-dicom/tests/overlay.rs`, `docs/lld/overlays.md`, `examples/example-server/src/overlay.rs` inside the diff, and pass 1.
- I counted 25 `#[test]` functions in the fixture file, which matches the implementer's count.
- I grepped `.claude/plans/F-193-design.md` for the mask preview wording.
- I read the `OverlayData` entry in `dicom-dictionary-std-0.10.0/src/tags.rs:21386` and the `Ox` resolution in `dicom-core-0.10.0/src/dictionary/data_element.rs:212`.
- I traced every new fixture by hand against the code, under both the correct code and each mutation pass 1 listed.
- I did not verify the implementer's claim that a bare `#[must_use]` fails `clippy::double_must_use` on an `Option` return. The attribute with a message is valid either way. I relied on the implementer's test, clippy and HTTP results. I ran nothing.

## Defects

None.

## Smells

**1. Two of pass 1's surviving mutations still survive: the polygon transposition and the multi-frame stride.**
- **Polygon, where:** `crates/ocelli-dicom/tests/overlay.rs:175`, `overlay_asymmetric_polygon_keeps_one_side`.
  - **What survives:** dropping the `xy` swap in `edge` (`overlay.rs:938`), or building `point` as `(row, column)` in `polygon_contains` (`:942`). Either one reflects the triangle across the diagonal relative to the probe.
  - **Why it survives:** in the correct code the hypotenuse crosses row 2 at column 2.5. In the reflected triangle it crosses at column 2.33. So `(2, 2)` is inside both, and `(2, 3)` is outside both. `(1, 2)` lies on the top edge of the original and on the reflected left edge. All three asserted verdicts agree, and no other polygon fixture is asymmetric: the square and the bow-tie are both symmetric under transpose.
  - **A discriminating probe:** `hides(1, 4)` is false, because `(1, 4)` is a vertex. `hides(4, 1)` is true, because it is below the vertex at `(3, 1)`. Under the reflection both answers flip.
- **Multi-frame, where:** `crates/ocelli-dicom/tests/overlay.rs:199`, `overlay_second_frame_uses_its_own_bits`.
  - **What survives:** with a 1 by 1 overlay, `pixels` is 1, so `offset = pixels * plane_index` (`overlay.rs:591`) equals `plane_index`. The expected answer for frame 1 is "clear", which is also what any offset that overshoots gives. These mutations stay green:
    - `offset = plane_index`
    - a byte-aligned stride of 8 bits per frame
    - any offset past the data
  - **A pinning fixture:** a 1 by 2 overlay with two frames and data `04 00`. Frame 0 has both pixels clear. Frame 1 has `(1, 1)` set and `(1, 2)` clear.
- **Why it matters:** CLAUDE.md requires that a test would fail if the arithmetic were wrong. These are the two pieces of F-193 geometry that still have no such test.

## Nitpicks

1. **`.claude/plans/F-193-design.md:24`.** The plan still says "The preview uses the shutter presentation value when it is present." The LLD now says the presentation value is not painted into the digits. The as-built text is right, but the plan and the LLD still disagree, and the LLD does not record this as a departure from the plan.
2. **`crates/ocelli-dicom/src/overlay.rs:88`.** The `OverlayReason::Polygon` rustdoc says "too few vertices or a crossing edge". It now also covers an odd component count, a zero-length edge and a folded adjacent pair. A zero-length edge is not a crossing.
3. **`crates/ocelli-dicom/tests/overlay.rs:185`.** `overlay_zero_length_edge_is_an_error` does not isolate the new pre-check loop. Every edge is in at least one adjacent pair, and `adjacent_overlap` returns true when `edge` returns `None` (`overlay.rs:907`). So deleting the loop at `:879` leaves the fixture green. The behaviour is guarded twice, which is harmless. Only a mutation claim against that loop would be wrong.
4. **`crates/ocelli-dicom/src/overlay.rs:873` and `:613`.** A 1-bit Overlay Data with an unexpected VR, for example UN, is refused as `BitsAllocated`. The rustdoc on `BitsAllocated` (`:86`) says it means the allocation is not 1 or 16, or the bit position is bad. `Metadata` (`:80`) is the reason documented for "the wrong value representation". The refusal is correct and only the label is off.
5. **`crates/ocelli-dicom/src/overlay.rs:264` and `:854`.** This is older than the fix and is the same shape as pass 1 smell 2. `Shutter::hides` returns true for a row or column below 1. For a pixel past the image's last row or column, every geometric shape also hides, but a `BITMAP` shape reports it open, because the now-clipped `plane.bit` is false there. Nothing in F-193 queries past the image.

## Verified clean

- `packed_bytes` checks length after expansion. `need = total.div_ceil(8)` is compared against twice the word count, so an OW plane is not over-rejected.
- `adjacent_overlap` uses `i64` intervals and `orient` uses `i64` products, so `i32` vertices cannot overflow.
- After the pre-check, the `edge(...) else return true` arms in `polygon_crosses` and the `else return false` arms in `polygon_contains` are unreachable on the read path, and they fail closed.
- `overlay_origin_column_is_not_the_row` catches swapped `(60xx,0050)` components, because `(2, 1)` would put the bit at overlay column 0.
- `overlay_circle_centre_is_row_then_column` catches a swapped `(0018,1610)`, because a centre of `(3, 1)` puts `(1, 3)` at squared distance 8.
- `overlay_frame_before_the_overlay_origin_is_empty` catches a 0-based Image Frame Origin.
- `overlay_bitmap_ow_follows_the_low_bit` catches `to_be_bytes`, and the bitmap group is still removed from `planes`.
- The `bit` clip does not change the server: `mask_list` and `hit` only query inside the image.
- The LLD lines 14, 26 and 38 describe the code as it now behaves.
- I found no em dash or prose semicolon in the changed tracked prose.
