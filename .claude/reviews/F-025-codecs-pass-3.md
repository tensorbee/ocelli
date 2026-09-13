# F-025 review, pass 3

**Reviewed**: fully staged working tree against
`fdd15fe717af1ccfb284dd03916e3b7eb5f5d5f1`
**Result**: 2 defects, 0 smells, 0 nitpicks

## Defects

### D1, native one-frame decode applies whole-Value padding to each frame

**Where**: `crates/ocelli-codec/src/registry.rs:380-391`,
`crates/ocelli-codec/src/native.rs:92-106,134-149`,
`.claude/plans/F-025-design.md:46-50,57-61,80-84`, and
`docs/lld/codecs.md:20-27,130-135`

**What**: the public decoder contract takes the encoded bytes for one frame,
but `RawDecoder` rejects any odd physical `src` length and requires a separate
pad for an odd logical frame. The LLD states this per-frame rule explicitly.
A valid native multi-frame Pixel Data Value may contain odd-length frames with
no bytes between them, so an exact frame slice cannot pass this decoder. With
one-bit native data, a later frame may begin in the middle of a byte or word,
which the current byte-slice frame contract also cannot represent without an
explicit extraction owner.

**Why it is wrong**: DICOM PS3.5 2026c section 8.1.1 requires individual
native Frames to be concatenated without padding. Padding is applied only to
the complete Pixel Data Value. Its second note expressly says a one-bit frame
may start in the middle of a byte or word. The HLD section 21 `Decoder`
contract and the staged registry API say `decode` consumes one frame, not the
whole Data Element. A frame decoder cannot infer whole-Value padding from
`FrameDesc::output_len` alone.

**Evidence**: a temporary fixture represented the first 1 by 3 eight-bit OB
frame of a standard-valid two-frame six-byte Pixel Data Value. It passed the
exact three frame bytes and required `[1, 2, 3]` output. `bin/ocelli.sh test
ocelli-codec --test native
review_probe_native_multiframe_member_has_no_per_frame_pad -- --exact` exited
101 with `Error: InvalidCodestream`. The probe was removed. The normative text
is at
<https://dicom.nema.org/medical/dicom/current/output/chtml/part05/chapter_8.html>.

**Required repair boundary**: validate physical even length and final Value
padding where the complete native Pixel Data Value and Number of Frames are
known. Give one-frame decode exact logical frame content, including explicit
bit-offset handling or a prior repack for non-byte-aligned one-bit frames. Do
not make the frame codec guess whole-Value padding from one frame's dimensions.

### D2, native OW wrongly requires insignificant padding bits to be zero

**Where**: `crates/ocelli-codec/src/native.rs:105-123` and
`crates/ocelli-codec/tests/native.rs:88-153`

**What**: odd logical native OW output is accepted only when the unused byte
of its final physical word is zero. The test suite treats a nonzero big-endian
OW padding byte as `TrailingData`. The same restriction is applied to
little-endian OW through the shared copy arm.

**Why it is wrong**: PS3.5 section 6.2 gives OB its trailing NULL rule. OW is
instead a stream of complete 16-bit words and has no NULL-byte padding rule.
PS3.5 section 8.1.1 calls Pixel Data padding data that is not part of the image
and shall not be considered significant. Requiring a value for those
insignificant OW padding bits rejects a legal Pixel Data Value.

**Evidence**: a temporary explicit little-endian OW fixture carried three
eight-bit pixels `[1, 2, 3]` followed by a nonzero unused byte `0x7e` in the
final word. `bin/ocelli.sh test ocelli-codec --test native
review_probe_ow_ignores_non_pixel_padding_bits -- --exact` exited 101 with
`Error: TrailingData` rather than returning the three pixels. The probe was
removed. PS3.5 sections 6.2 and 8.1.1 are the normative distinction.

**Required repair boundary**: keep exact physical word-length validation for
OW and ignore the non-pixel part of its final word. Retain the required NULL
check only for OB. Big-endian OW must still normalize every complete physical
word before excluding the insignificant byte from output.

## Smells

None.

## Nitpicks

None.

## Verified clean

- Pass-two D1 is closed. `is_supported_descriptor` accepts unsigned RGB and
  YBR_FULL with exactly three samples at either eight or sixteen Bits
  Allocated, matching PS3.5 Table 8.2.2-1. The approved plan and LLD now state
  that same table entry.
- The permanent RGB16 and YBR_FULL16 fixtures each construct six even-length
  RLE segments in sample order with the MSB plane before the LSB plane. Their
  hand-computed expected buffers prove canonical little-endian bytes
  interleaved R, G, B and Y, Cb, Cr per pixel. `bin/ocelli.sh test
  ocelli-codec --test rle sixteen_bit_` exited 0 with both tests passing.
- A fresh mutation distinct from the author's depth mutation changed the
  production sample mapping from `plane / bytes_per_sample` to
  `plane % samples_per_pixel`. The same focused command exited 101 with both
  positive colour fixtures reporting component-shuffled buffers. The mutation
  was reverted and both tests passed again.
- Invalid RLE combinations remain refused before source parsing and without
  caller-buffer mutation. The permanent table test covers RGB sample count
  two, signed RGB, signed Palette Color, monochrome sample count two, and
  unlisted YBR_FULL_422. One-bit and thirty-two-bit RLE remain the explicit
  plan-approved `UnsupportedPixelFormat` cases.
- The earlier exact-UID repair remains correct. Separate raw decoder values
  retain Implicit VR Little Endian and Explicit VR Little Endian identity.
  Implicit VR requires OW, explicit native syntax retains legal eight-bit OB,
  and registration dispatches by exact UID.
- For a single complete Value under the currently implemented assumption,
  native OB NULL validation is atomic. Big-endian OW swaps each physical
  sixteen-bit word, including for eight-bit and thirty-two-bit sample
  containers. It does not swap a complete thirty-two-bit sample. Signedness
  and Bits Stored remain untouched for the F-018 stored-pixel stage. D1 and D2
  identify the remaining native boundary errors.
- RLE header validation requires the complete 64-byte header, the exact
  nonzero segment count, first offset 64, strictly increasing even used
  offsets, in-bounds nonempty even physical segments, and zero unused offsets.
- RLE decoding remains allocation-free and atomic through its validation and
  write passes. Literal and repeat runs cannot cross a row, `0x80` remains a
  legal no-op while output is required, exact necessary segment padding is
  accepted, and malformed termination or surplus data preserves output.
- Raw reports preserved sample layout. RLE and JPEG report interleaved layout.
  The new YBR_FULL fixture preserves its sample values without introducing a
  second colour transform.
- Native and RLE registration preflight all four exact UIDs before insertion.
  Existing JPEG registration remains compatible. Deflated Explicit VR Little
  Endian stays unavailable in the frame registry and is handled only by the
  strict F-016 whole-dataset ingest path.
- `bin/ocelli.sh test ocelli-codec` exited 0 with 43 tests passing. This was 2
  unit tests, 10 JPEG tests, 7 native tests, 14 registry tests, and 10 RLE
  tests.
- `bin/ocelli.sh test ocelli-dicom` exited 0 with 112 tests passing and two
  corpus tests ignored outside their gate.
- `bin/ocelli.sh gate corpus` exited 0 with all 92 manifest rows verified and
  both corpus integration tests passing. The existing synthetic reference
  proves byte equivalence for its single-frame native LE, native BE, RLE, and
  Deflate rows. It does not exercise D1's native multi-frame boundary.
- `bin/ocelli.sh gate native` exited 0. Native entry points linked, shared
  crates including `ocelli-codec` and `ocelli-dicom` compiled for
  `wasm32-unknown-unknown`, native all-target checks passed, and 17 direct
  dependency feature sets agreed across targets.
- `bin/ocelli.sh gate test clippy` exited 0. The full workspace test and clippy
  gates were green after remediation.
- The staged source adds no repository `unsafe`, no `wasm-bindgen` outside
  `ocelli-wasm`, and no allocation in raw or RLE decode. No Deflate frame
  decoder or duplicate pixel arithmetic was introduced.
