# F-025 review, pass 2

**Reviewed**: fully staged working tree against
`fdd15fe717af1ccfb284dd03916e3b7eb5f5d5f1`
**Result**: 1 defect, 0 smells, 0 nitpicks

## Defects

### D1, standard-valid sixteen-bit colour RLE is refused

**Where**: `crates/ocelli-codec/src/rle.rs:67-78`,
`crates/ocelli-codec/tests/rle.rs:172-185`,
`.claude/plans/F-025-design.md:88-92`, `docs/lld/codecs.md:154-163`, and
`.claude/reviews/F-025-codecs-pass-1.md:65-68`

**What**: `is_supported_descriptor` accepts RGB and YBR_FULL only when Bits
Allocated is eight. The permanent invalid-combination fixture consequently
classifies unsigned sixteen-bit RGB and YBR_FULL as forbidden. The LLD says
both interpretations require eight-bit samples, and its next paragraph claims
that sixteen-bit RLE is supported. The pass-one review repeats the false
YBR_FULL restriction.

**Why it is wrong**: DICOM PS3.5 2026c Table 8.2.2-1 permits both RGB and
YBR_FULL with Samples per Pixel 3, Pixel Representation 0, and Bits Allocated
8 or 16. The approved plan requires validation of the complete table and does
not declare colour sixteen-bit RLE unavailable. Refusing valid descriptors is
wrong behaviour, while tests and prose calling them invalid are false claims.

**Evidence**: a temporary fixture encoded two unsigned sixteen-bit RGB pixels
as the six required even-length RLE byte-plane segments in R, G, B order and
required canonical interleaved little-endian output. `bin/ocelli.sh test
ocelli-codec --test rle review_probe_table_822_1_accepts_sixteen_bit_rgb --
--exact` exited 101 with `Error: UnsupportedPixelFormat` before reading the
valid source. The probe was removed. The normative table is published at
<https://dicom.nema.org/medical/dicom/current/output/chtml/part05/sect_8.2.2.html>.

## Smells

None.

## Nitpicks

None.

## Verified clean

- Pass-one D1 is remediated. Native decode now rejects every odd physical
  Value length before trimming and requires exactly one zero pad only for an
  odd logical length. The permanent
  `native_value_padding_is_only_accepted_when_needed_and_zero` test passed.
  Changing the physical even-length guard to false made that exact test fail
  with `TrailingData` where `InvalidCodestream` was required. The mutation
  command exited 101 and the mutation was reverted.
- Pass-one D2 is remediated. Separate decoder values retain Implicit VR Little
  Endian and Explicit VR Little Endian UID identity. The implicit value
  requires OW while the explicit value retains the legal eight-bit OB route.
  The permanent
  `implicit_vr_requires_ow_while_explicit_little_endian_accepts_ob` test
  passed. Changing the implicit decoder's `requires_ow` field to false made
  that exact test observe `Ok(())` instead of `UnsupportedPixelFormat`. The
  mutation command exited 101 and the mutation was reverted.
- Pass-one D3 is remediated for invalid combinations. RLE preflight now checks
  Photometric Interpretation, Samples per Pixel, Bits Allocated, Pixel
  Representation, and encapsulated OB before parsing the source. The permanent
  `table_822_1_rejects_invalid_photometric_sample_and_container_combinations`
  test passed for malformed sample counts, signed colour, Palette Color signed
  data, and unlisted YBR_FULL_422. Changing the accepted colour sample count
  from three to two made that exact test reach source parsing and fail. The
  mutation command exited 101 and the mutation was reverted. The sixteen-bit
  colour entries in that test remain the defect above.
- A temporary native matrix exercised one-bit and eight-bit odd logical Values
  through explicit little-endian OB and OW plus explicit big-endian OB and OW.
  It also exercised sixteen-bit and thirty-two-bit big-endian OW word
  normalization. Exact pads and canonical output passed in all cases. The
  command exited 0 and the temporary probe was removed.
- Native OB with more than eight allocated bits is refused before output
  mutation. OW normalization swaps physical sixteen-bit words, including for
  eight-bit and thirty-two-bit containers, and never swaps a whole
  thirty-two-bit sample. Signedness and Bits Stored remain byte-preserving
  metadata for the F-018 stored-pixel stage.
- RLE header validation still requires the 64-byte header, an exact nonzero
  segment count, first offset 64, strictly increasing even used offsets,
  in-bounds nonempty even physical segments, and zero unused offsets.
- The two-pass RLE traversal validates all rows, run lengths, segment
  termination, and exact padding before any write. Literal and repeat runs
  cannot cross a row. The `0x80` no-op remains legal while output is required.
  Every tested refusal preserves sentinel output.
- RLE plane order remains most-significant byte first for each sample and the
  output layout is typed as interleaved. Raw layout remains preserved. One-bit
  and thirty-two-bit RLE are intentionally refused before source parsing, as
  the approved plan records.
- Native and RLE registration preflights all four exact UIDs before insertion.
  Registry compatibility and the F-024 JPEG routes remained green. Deflated
  Explicit VR Little Endian remains `KnownUnavailable` in the frame registry
  and owned by the strict F-016 dataset ingest path.
- `bin/ocelli.sh test ocelli-codec` exited 0 with 41 tests passing. This was 2
  unit tests, 10 JPEG tests, 7 native tests, 14 registry tests, and 8 RLE tests.
- `bin/ocelli.sh test ocelli-dicom` exited 0 with 112 tests passing and two
  corpus tests ignored outside their gate.
- `bin/ocelli.sh gate corpus` exited 0. It verified all 92 manifest rows and
  both ignored corpus integration tests passed, including byte-equivalent
  native LE, native BE, RLE, and Deflate synthetic truth.
- `bin/ocelli.sh gate native` exited 0. Native entry points linked, shared
  crates including `ocelli-codec` and `ocelli-dicom` compiled for
  `wasm32-unknown-unknown`, native all-target checks passed, and 17 direct
  dependency feature sets agreed across targets.
- `bin/ocelli.sh gate test clippy` exited 0. The full workspace test and clippy
  gates remained green after the remediation.
- The complete staged implementation introduces no repository `unsafe`, no
  `wasm-bindgen` outside `ocelli-wasm`, and no per-call allocation in raw or RLE
  decode. Deflate remains dataset scoped, with no false frame decoder.
