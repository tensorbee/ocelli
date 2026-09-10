# F-025 review, pass 5

**Reviewed**: fully staged working tree against
`fdd15fe717af1ccfb284dd03916e3b7eb5f5d5f1`
**Result**: 0 defects, 0 smells, 0 nitpicks

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

- Pass-four D1 is closed. Direct registered Explicit VR Big Endian OW decode
  rejects an odd logical source length after format and exact-length checks and
  before its first output write. The successful direct route uses
  `chunks_exact(2)`, so every normalized physical word is complete.
- `bin/ocelli.sh test ocelli-codec --test native
  direct_registered_big_endian_ow_refuses_an_odd_logical_slice_atomically --
  --exact` exited 0. Its permanent two-frame fixture proves the direct
  three-byte physical slice is refused with sentinel output unchanged, then
  proves the same complete Value decodes frame zero to `[1, 2, 3]` through
  `NativeFrameIndex`.
- Complete-Value validation and logical-frame decode remain separate.
  `NativeFrameIndex` requires a positive frame count, retains exact UID byte
  order and typed OB or OW evidence, and uses checked multiplication, rounded
  division, conversion, frame-start, frame-end, and physical-bound arithmetic.
  Short, long, overflowed, out-of-range, and wrong-output cases are refused
  before output mutation.
- OB applies its required final NULL only to the complete Value when the packed
  byte count is odd. OW requires complete physical words and ignores bits or
  bytes outside the last Pixel Cell. Little-endian and Big Endian OW mapping
  operate on physical 16-bit words for 8-bit, 16-bit, and 32-bit containers.
- Native one-bit Pixel Cells are read and repacked least-significant bit first.
  Permanent tests cover first and later mid-byte OB frames. A temporary
  independent fixture covered two nine-bit Big Endian OW frames where frame
  one began inside a word. It decoded the hand-computed outputs `[0x01, 0x01]`
  and `[0xaa, 0x00]` while accepting a nonzero insignificant final byte. The
  same probe required `usize::MAX` frames of a 24-bit frame to return
  `FrameMismatch`. Its exact command exited 0 with one test passing, and the
  probe was removed.
- A fresh arithmetic mutation distinct from the author and prior reviewer
  mutations changed one-bit output packing from `1_u8 << local_bit` to
  `1_u8 << (7 - local_bit)`. `bin/ocelli.sh test ocelli-codec --test native
  native_value_index_extracts_first_and_mid_byte_one_bit_frames -- --exact`
  exited 101 with actual `[176]` against expected `[13]`. The mutation was
  reverted and no unstaged change remained.
- Earlier exact-UID and VR repairs remain correct. Implicit VR Little Endian
  requires OW. Explicit VR native routes retain legal eight-bit OB. Native and
  RLE registration preflight all four target UIDs before insertion. Raw output
  reports preserved sample layout, while RLE and JPEG report interleaved
  layout.
- RLE retains allocation-free two-pass atomic decode. The first pass validates
  the 64-byte header, exact segment count, first offset 64, strictly increasing
  even offsets, nonempty even physical segments, zero unused offsets, every
  row, literal or repeat run, legal `0x80` no-op, and exact necessary zero pad.
  The second pass writes the same validated traversal.
- RLE format validation matches PS3.5 Table 8.2.2-1 for MONOCHROME1,
  MONOCHROME2, PALETTE COLOR, RGB, and YBR_FULL. The RGB16 and YBR_FULL16
  fixtures prove six MSB-first planes produce interleaved canonical
  little-endian samples. Invalid photometric, sample-count, signed-colour,
  one-bit, and 32-bit combinations remain atomic refusals.
- Deflated Explicit VR Little Endian remains unavailable in the frame registry
  and owned by the strict F-016 whole-data-set ingest route. Corpus evidence
  compares native LE, native BE, RLE, and Deflate with one synthetic native
  truth, and checks `DispatchPath::DeflatedExplicitVrLittleEndian`.
- The corrected plan and LLD agree with the implemented direct Big Endian OW
  refusal, complete-Value indexing, OB and OW distinction, frame arithmetic,
  sample-layout evidence, RLE table and segment rules, and Deflate ownership.
  No new deviation or false completion claim was found.
- `bin/ocelli.sh test ocelli-codec` exited 0 with 47 tests passing. This was 2
  unit tests, 10 JPEG tests, 11 native tests, 14 registry tests, and 10 RLE
  tests.
- `bin/ocelli.sh test ocelli-dicom` exited 0 with 112 tests passing and two
  corpus tests ignored outside their gate.
- `bin/ocelli.sh gate corpus` exited 0 with all 92 manifest rows verified and
  both corpus integration tests passing.
- `bin/ocelli.sh gate native` exited 0. Native entry points linked, shared
  crates compiled for `wasm32-unknown-unknown` and native all-targets, and 17
  direct dependency feature sets agreed across targets.
- `bin/ocelli.sh gate test` and `bin/ocelli.sh gate clippy` each exited 0.
  The complete workspace tests and lint were green on the restored tree.
- The full staged implementation contains no Rust `as` cast, adds no
  repository `unsafe`, imports no `wasm-bindgen` outside `ocelli-wasm`, adds no
  Deflate frame decoder, and allocates no storage in native or RLE decode.
