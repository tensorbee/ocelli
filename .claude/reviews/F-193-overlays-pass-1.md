# F-193 review, pass 1

**Reviewed**: uncommitted working tree on sprint/s42 against 98fd152
**Author / reviewer**: implementation by the sprint host. Review by Claude Code, read only, with no shell.
**Result**: 2 defects, 3 smells, 4 nitpicks

## Acceptance challenged

- **AC1, origin.** `overlay_index` (`crates/ocelli-dicom/src/overlay.rs:173`) computes overlay row as image row minus origin row plus 1. With origin `1\1`, image `(1,1)` is overlay `(1,1)`, index 0, which is byte `0x01` bit 0, so it is set. With origin `0\0`, image `(1,1)` is overlay `(2,2)`, index 5, which is clear. In `outside_count`, overlay `(1,1)` lands on image `(0,0)`, so the count is 1. Both match PS3.3 C.9.2's note that `0\0` is one row above and one column left. Confirmed.
- **AC2, rectangle.** `column < *left` with left 2 hides column 1 and leaves column 2 open. The implementer's mutation to `<=` turned the test red. Confirmed.
- **Circle.** Centre `2\2`, radius 1. At `(2,3)` the squared distance is 1.0, and `1.0 > 1.0` is false, so the pixel is open. At `(2,4)` it is 4.0, so the pixel is hidden. Every operand is an integer-valued f64, so the equality is exact and no epsilon is needed. Confirmed.
- **Polygon.** The boundary is tested before the ray cast, so it counts as inside. `shares_vertex` skips adjacent pairs, including the wrap-around pair, and the bow-tie fixture crosses on edges 1 and 3. The ray-crossing cross-multiplication flips correctly on the sign of `dy`. Confirmed, with the triangle gap in Defect 2.
- **AC3, bitmap.** `bitmap_shutter` removes the group from `planes` and keeps a clone on `Shutter.bitmap`. `hides` follows `plane.bit`, where value 1 hides. Confirmed.
- **AC4, presentation.** An absent or empty value maps to `None`, and a present 0 maps to `Some(0)`. The server prints `null` and the page prints `The neutral value is absent.` Confirmed.
- **AC5.** Bits allocated 8 reaches the `_` arm and returns `BitsAllocated`. Confirmed.
- **AC7, the synthetic object.** In `examples/example-server/src/bytes.rs` the mask is built from `set.planes()`, so it holds graphic overlay bits only, never shutter bits. It is capped at 256 entries. Row-major output gives `[1,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0]`. `hit` is true, `hidden` is true and `presentation` is null. Confirmed.

## Evidence

- I read `/tmp/ocelli-f193.diff` in full, the approved plan, and `docs/lld/overlays.md` inside the diff.
- I read `crates/ocelli-dicom/src/frame_open.rs:973` to `:1008` (`project_metadata`). Pixel Data and pixel sequences are skipped, and every other element is projected.
- I read `crates/ocelli-dicom/src/metadata.rs:338` to `:357`. `PrimitiveValue::U16` becomes `MetadataValue::Unsigned16` and `U8` becomes `Bytes`, so an OW element arrives as words. The passing 16-bit fixture, which is OW with `Unsigned16`, confirms that path.
- I grepped `image_rows` and `image_columns` in `overlay.rs`. The `OverlayPlane` fields are written at line 469 and never read.
- I relied on the implementer's recorded test, clippy, mutation, tsc and eslint results. I ran nothing.

## Defects

**1. A conforming 1-bit overlay stored as OW is refused as `BitsAllocated`.**
- **Where:** `crates/ocelli-dicom/src/overlay.rs:586`, plus the same OB-only assumption in the bitmap check at `:818`.
- **Trigger:** An Explicit VR Little Endian object with `(6000,0100)` = 1, `(6000,0102)` = 0 and `(6000,3000)` encoded as OW.
- **What happens:** The 1-bit arm requires `VR::OB` and `MetadataValue::Bytes`. It gets `VR::OW` and `Unsigned16`, so `read_overlays` returns `BitsAllocated`. Because a refusal returns no planes, every other group and the shutter on that frame are lost too.
- **Expected:** The plane is read. Little-endian OW words carry the same packed bits as OB bytes, low bit first.
- **Standard:** PS3.6 gives `(60xx,3000)` the VR "OB or OW". PS3.5 8.1.2 permits either in explicit VR, and PS3.5 A.1 makes OW mandatory under Implicit VR Little Endian, which is the common legacy encoding. A bitmap shutter in OW, PS3.3 C.7.6.15, fails the same way.
- **Test gap:** No fixture encodes a 1-bit overlay as OW.

**2. A three-vertex polygon with a zero-length edge is accepted and then hides the whole image. The LLD says it is refused.**
- **Where:** `crates/ocelli-dicom/src/overlay.rs:861` and `:899`. The prose is `docs/lld/overlays.md:38`.
- **Trigger:** Shutter Shape `POLYGONAL` with `(0018,1620)` = `1\1\1\1\3\3`.
- **What happens:** With three vertices every edge pair is adjacent, so `polygon_crosses` reaches `continue` for every pair and never calls `edge`, the only place a zero-length edge is detected. The read succeeds. Then `polygon_contains` gets `None` from `edge(vertices, 0)` and returns false, so `hides` is true for every pixel.
- **Expected:** `OverlayReason::Polygon`. The LLD says "a zero-length edge ... is refused".
- **Standard:** PS3.3 C.7.6.11 requires a closed polygon whose edges do not otherwise intersect, and a repeated consecutive vertex is not a polygon edge. With four or more vertices every edge has a non-adjacent partner, so the gap is three vertices only.
- **Test gap:** No fixture covers it.

## Smells

**1. The fixtures cannot detect a row and column transposition, and multi-frame bit selection has no fixture at all.**
- **Where:** `crates/ocelli-dicom/tests/overlay.rs:21`, `:79`, `:89` and the `hit()` fixture at `:240`.
- **The symmetric inputs:** The image is 4 by 4. The origins are `1\1` and `0\0`, the circle centre is `2\2`, and the polygon is a square symmetric under transpose. The only set bit is index 0.
- **What survives:** Each of these mutations stays green:
  - swapping the components of `(60xx,0050)` or `(0018,1610)`
  - dropping the `xy` swap in polygon vertices
  - indexing overlay bits column-major instead of row-major
- **Multi-frame:** `plane_index` and `offset = pixels * plane_index` are never exercised with Number of Frames in Overlay above 1 or Image Frame Origin above 1.
- **Standard:** PS3.3 C.9.2 and C.7.6.11 fix the row then column order, and CLAUDE.md requires a hand-computed fixture for this arithmetic.
- **Suggested fixtures:** A non-square image, origin `1\2`, a circle at `1\3`, an asymmetric polygon, a bit at index 1, and a two-frame overlay with origin 2.

**2. `OverlayPlane` stores the image bounds but `bit` never uses them.**
- **Where:** `crates/ocelli-dicom/src/overlay.rs:115` and `:116` are dead fields. `bit` is at `:160`.
- **Behaviour:** `bit` returns false for an image row or column below 1, but it returns the overlay's bit for a row or column past the image's last.
- **Trigger:** An 8 by 8 overlay at origin `1\1` on a 4 by 4 image. `bit(5,1)` can be true, while the rustdoc promises an answer about an image pixel.
- **Why it matters:** Nothing in F-193 queries past the image, but a later compositor that trusts `bit` gets asymmetric clipping.

**3. Adjacent edges that fold back along their shared line are not checked.**
- **Where:** `crates/ocelli-dicom/src/overlay.rs:878`.
- **Trigger:** Vertices `1\1\1\4\1\2\3\2`. Edges 0 and 1 overlap on columns 2 to 4, yet `shares_vertex` skips the pair.
- **Consequence:** The polygon is accepted rather than refused. The pixel impact is small because the overlap lies on the boundary, which counts as inside.
- **Standard:** PS3.3 C.7.6.11 allows edges to meet only at their shared vertex, and this pair meets along a segment.
- **Fix:** Before skipping an adjacent pair, check whether the two edges are collinear and overlap.

## Nitpicks

1. **`crates/ocelli-dicom/src/overlay.rs:235`.** The `Shutter` rustdoc says "true outside every geometric opening", which reads as union semantics. The next sentence and the code say intersection. "outside any geometric opening" would match both.
2. **`crates/ocelli-dicom/src/overlay.rs:283` and `:288`.** `OverlaySet::shutter` and `OverlaySet::presentation_value` lack `#[must_use]`, which every sibling accessor carries.
3. **`CHANGELOG.md:15`.** It says shutters "are read from the top-level overlay groups". Geometric shutters live in group `0018`. "Overlay planes are read from the top-level overlay groups, and display shutters from the shutter attributes" is accurate.
4. **`examples/viewer-react/src/App.tsx:564`.** The plan describes a small mask preview that uses the presentation value when present. The page prints the mask as digits and prints the value separately. That is acceptable for S42, but the plan and the LLD should say the same thing.

## Verified clean

- The group walk covers even groups `6000` to `601E`, skips a missing group without stopping, and ignores odd groups.
- Required C.9.2 Type 1 attributes are enforced once Overlay Rows is present. Overlay Type is `G` or `R` only. A 1-bit overlay requires bit position 0.
- 1-bit packing is low bit first, and the length check uses `div_ceil(8)` across all frames. Every product is `checked_mul` on u32.
- `plane_index` is correct for a 0-based frame against the 1-based Image Frame Origin, and an uncovered frame contributes nothing.
- The per-frame `(5200,9230)` item is searched recursively and refused as `FrameOverlayUnsupported`.
- `(0018,1610)` is read as one IS with two components, `(0018,1612)` as the radius and `(0018,1620)` as the vertices. This is the recorded dictionary departure and correct.
- A rectangle with left greater than right, or upper greater than lower, is refused.
- `(0018,1623)` is read as US, must be an even overlay group, and the bitmap is checked for G, packed bits, origin `1\1` and image size.
- Orientation and crossing arithmetic is done in i64, so it cannot overflow i32 inputs.
- On the server, the `/overlay` OPTIONS, POST, size-cap and no-length messages are routed correctly. `frame` flows through to `frame_out_of_range`, and the error JSON carries the index.
- The vite proxy entry, the React parsing of `presentation` as `null` or a number, and the mask validation to 0 or 1 entries are all correct.
- The page sentence is exactly `The neutral value is absent.`
- HLD section 19 `SegOverlay` is untouched. No `unsafe`, no wasm-bindgen and no renderer change.
- I found no em dash or prose semicolon in the changed tracked prose.
