# F-025 review, pass 4

**Reviewed**: fully staged working tree against
`fdd15fe717af1ccfb284dd03916e3b7eb5f5d5f1`
**Result**: 1 defect, 0 smells, 0 nitpicks

## Defects

### D1, direct Big Endian OW accepts an unrepresentable odd-byte frame slice

**Where**: `crates/ocelli-codec/src/native.rs:241-269`,
`crates/ocelli-codec/src/registry.rs:359-391`,
`.claude/plans/F-025-design.md:79-85,99-109`, and
`docs/lld/codecs.md:120-153`

**What**: the registered `RawDecoder::decode` path accepts an odd-length
logical Big Endian OW frame and reverses the final one-byte `chunks(2)` chunk
as though it were a complete physical word. Such a frame slice cannot carry
the physical bytes for that logical frame. When an odd-byte frame starts on a
word boundary, its final canonical byte is the first byte of a physical word
and appears after the next frame's first canonical byte in the Value. The
three bytes for the frame are therefore not contiguous in physical order.
The complete-Value `NativeFrameIndex` handles this correctly, but the public
registered direct path neither refuses the impossible slice nor routes through
that index.

**Why it is wrong**: DICOM PS3.5 section 8.1.1 requires native Frames to be
concatenated without per-frame padding, then encodes OW as complete 16-bit
words. The corrected plan and LLD consequently restrict direct Big Endian OW
normalization to frames already isolated on a Value word boundary and assign
odd-byte boundaries to `NativeFrameIndex`. Accepting a three-byte physical OW
slice contradicts that contract and silently changes a Pixel Cell.

**Evidence**: the complete canonical two-frame stream
`01 02 03 04 05 06` has Big Endian OW physical Value
`02 01 04 03 06 05`. A temporary probe passed the first contiguous three
physical bytes `02 01 04` to direct decode for a three-pixel frame. The command
`bin/ocelli.sh test ocelli-codec --test native
review_probe_direct_big_endian_ow_refuses_odd_frame_boundary -- --exact`
first exited 101 because the decoder returned `Ok(())` instead of
`InvalidCodestream`. A second form exited 101 with actual output `[1, 2, 4]`
instead of canonical `[1, 2, 3]`. The probe was removed.

**Required repair boundary**: reject odd logical Big Endian OW source lengths
in direct `Decoder::decode` before modifying output, and permanently test both
the refusal and atomicity. Keep `NativeFrameIndex` as the complete-Value route
for odd-byte and non-byte-aligned frame boundaries. Do not add per-frame
padding or change the complete-Value indexing rules.

## Smells

None.

## Nitpicks

None.

## Verified clean

- Both pass-three defects are otherwise closed at the complete native Value
  boundary. `NativeFrameIndex` validates the complete Value separately from
  logical-frame decode, owns positive Number of Frames evidence, and performs
  checked frame-bit, total-bit, rounded-byte, frame-offset, and frame-bound
  arithmetic. Construction rejects zero frames, multiplication overflow,
  short Values, and trailing bytes.
- Complete OB Values require exactly the necessary final NULL byte. Complete
  OW Values require whole physical words but correctly ignore nonzero bits and
  bytes outside the final Pixel Cell. Direct logical-frame decode no longer
  guesses whole-Value padding.
- Permanent fixtures cover explicit little-endian OB, little-endian OW,
  Big Endian OW global word mapping, first and mid-byte one-bit frame starts,
  byte-aligned odd-byte OW frame boundaries, wrong output lengths, invalid
  frame indices, missing or bad OB padding, and nonzero insignificant OW
  storage.
- An additional temporary proof covered two five-bit Big Endian OW frames,
  three byte-aligned eight-bit Big Endian OW frames, three byte-aligned one-bit
  little-endian OB frames, and `usize::MAX` frame-count multiplication. The
  exact temporary test exited 0 with one test passing and was removed.
- Frame-index failure paths validate the requested frame and output length
  before writing. Complete-Value construction retains the exact transfer
  syntax's byte order and the typed OB or OW evidence. Implicit VR Little
  Endian requires OW, explicit native syntaxes retain legal eight-bit OB, and
  exact UID registration remains atomic.
- A fresh arithmetic mutation distinct from the author's evidence changed the
  Big Endian OW physical mapping from `canonical_byte ^ 1` to
  `canonical_byte`. `bin/ocelli.sh test ocelli-codec --test native
  ow_value_index_ignores_final_bits_and_uses_global_big_endian_words --
  --exact` exited 101 with actual `[2, 1, 4]` against expected `[1, 2, 3]`.
  The mutation was reverted and no unstaged mutation remained.
- RLE still enforces the complete PS3.5 Table 8.2.2-1 combinations, including
  RGB16 and YBR_FULL16 six-segment MSB-first order. One-bit and thirty-two-bit
  RLE remain permanent refusals. Header, even offset, segment count, row
  termination, no-op, necessary zero pad, surplus data, and atomic failure
  contracts remain covered.
- Raw decoding preserves sample layout. RLE and JPEG report interleaved
  layout. Native and RLE registration preflight exact UIDs before insertion.
  Deflated Explicit VR Little Endian remains unavailable in the frame registry
  and remains owned by the F-016 whole-data-set ingest path.
- `bin/ocelli.sh test ocelli-codec` exited 0 with 46 tests passing. This was 2
  unit tests, 10 JPEG tests, 10 native tests, 14 registry tests, and 10 RLE
  tests.
- `bin/ocelli.sh test ocelli-dicom` exited 0 with 112 tests passing and two
  corpus tests ignored outside their gate.
- `bin/ocelli.sh gate corpus` exited 0 with 92 manifest rows verified and both
  corpus integration tests passing.
- `bin/ocelli.sh gate native` exited 0. Native entry points linked, shared
  crates compiled for `wasm32-unknown-unknown` and native all-targets, and 17
  direct dependency feature sets agreed across targets.
- `bin/ocelli.sh gate test` and `bin/ocelli.sh gate clippy` each exited 0.
  The full workspace test and lint gates were green on the restored tree.
- The full staged diff adds no repository `unsafe`, no `wasm-bindgen` outside
  `ocelli-wasm`, no Deflate frame decoder, and no allocation in native or RLE
  decode.
