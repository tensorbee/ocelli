# F-025 review, pass 1

**Reviewed**: staged working tree against `fdd15fe717af1ccfb284dd03916e3b7eb5f5d5f1`
**Result**: 3 defects, 0 smells, 0 nitpicks

## Defects

### D1, native decoding accepts a missing mandatory Value pad

**Where**: `crates/ocelli-codec/src/native.rs:88-98,117-123` and
`crates/ocelli-codec/tests/native.rs:87-98`

**What**: `validate_native_length` accepts `src.len() == logical_len` even when
the logical byte count is odd. The little-endian and big-endian OB paths then
copy that odd physical Value successfully. The permanent test asserts that the
three-byte unpadded input succeeds before also checking the valid four-byte
padded form.

**Why it is wrong**: DICOM PS3.5 section 6.2 requires an OB octet stream to be
padded with one trailing NULL when needed to reach even length. OW is a stream
of complete 16-bit words. The approved plan also says at lines 45-49 that a
native Pixel Data Value has even physical length, and `docs/lld/codecs.md`
lines 129-132 claims the decoder validates that rule. Accepting an unpadded odd
physical Value makes those claims false and accepts a malformed Data Element.

**Evidence**: a temporary test required
`RawDecoder::little_endian().decode(&[1, 2, 3], ...)` to return
`InvalidCodestream` without changing its sentinel output.
`bin/ocelli.sh test ocelli-codec --test native
review_probe_requires_physical_even_length_and_implicit_ow -- --exact` exited
101. The assertion observed `Ok(())` instead of `Err(InvalidCodestream)`. The
temporary probe was removed.

### D2, Implicit VR Little Endian accepts an impossible OB Pixel Data VR

**Where**: `crates/ocelli-codec/src/native.rs:10-17,69-98`

**What**: one `RawDecoder::little_endian` value claims both Implicit VR Little
Endian and Explicit VR Little Endian. Its decode method has no selected UID,
so it accepts an eight-bit `PixelDataVr::Ob` descriptor on the implicit route.

**Why it is wrong**: DICOM PS3.5 A.1 requires Pixel Data under Implicit VR
Little Endian to have VR OW. OB is an option for eight-bit native Pixel Data
only when an Explicit VR carries that evidence. The plan says the decoder
validates Pixel Data VR and preserves exact UID dispatch. A shared decoder that
cannot observe which of its two UIDs was selected cannot enforce that contract.

**Evidence**: a temporary test registered the production adapters and called
`Registry::decode("1.2.840.10008.1.2", ...)` with an even two-byte OB
descriptor. `bin/ocelli.sh test ocelli-codec --test native
review_probe_implicit_vr_requires_ow -- --exact` exited 101. The assertion
observed `Ok(())` instead of `Err(UnsupportedPixelFormat)`, and the output was
therefore not left at its sentinel value. The temporary probe was removed.

### D3, RLE accepts Pixel Module combinations forbidden by the standard

**Where**: `crates/ocelli-codec/src/rle.rs:22-38`

**What**: RLE preflight checks only that the VR is OB and Bits Allocated is
eight or sixteen. It derives a segment count from any Samples per Pixel value
and never consults the retained Photometric Interpretation or Pixel
Representation. For example it accepts RGB with two samples per pixel and
decodes two planes successfully.

**Why it is wrong**: DICOM PS3.5 Table 8.2.2-1 fixes the allowed combinations.
RGB requires three samples per pixel and unsigned representation. YBR_FULL
also requires three unsigned eight-bit samples. Palette Color requires one
unsigned sample. MONOCHROME1 and MONOCHROME2 require one sample. Accepting
combinations outside that table makes a malformed descriptor look like a
successfully decoded standard RLE frame and leaves a downstream consumer with
no defined pixel meaning.

**Evidence**: a temporary fixture supplied two valid one-byte RLE segments and
a descriptor with Photometric Interpretation RGB, Samples per Pixel 2, and
Bits Allocated 8. `bin/ocelli.sh test ocelli-codec --test rle
review_probe_rejects_nonstandard_rgb_sample_count -- --exact` exited 101. The
assertion observed `Ok(())` instead of `Err(UnsupportedPixelFormat)`. The
temporary probe was removed.

## Smells

None.

## Nitpicks

None.

## Verified clean

- Raw Explicit VR word normalization matches PS3.5 Annex D. The fixtures prove
  identical canonical little-endian bytes for signed twelve-bit values in
  sixteen-bit containers, distinct eight-bit big-endian OB and OW ordering,
  and word-wise rather than whole-sample swapping for a 32-bit OW sample.
- RLE header fields are read as sixteen little-endian 32-bit values, used
  offsets begin at 64, used offsets are strictly increasing and even, and
  unused offsets must be zero. Segment lengths are checked as even.
- RLE PackBits traversal is row scoped. Literal and replicate runs cannot cross
  a row boundary, `0x80` is accepted while the normative decoder loop still
  needs output, truncated runs and surplus output fail, and exact necessary
  zero segment padding is distinguished from another run. PS3.5 G.3.2 stops
  the decoder loop when the uncompressed segment size is reached, so rejecting
  another `0x80` after that point as trailing data is correct.
- RLE byte planes are consumed most-significant first and written as
  interleaved canonical little-endian samples. Changing
  `bytes_per_sample - 1 - plane % bytes_per_sample` to
  `plane % bytes_per_sample` made
  `two_rows_and_two_byte_planes_reconstruct_exact_little_endian_pixels` fail
  with byte-swapped samples. The mutation command exited 101 and the mutation
  was reverted.
- RLE validation is a complete first pass before any output write. All later
  indexes are bounded by the validated frame dimensions and segment count.
  Malformed header, offset, run, padding, source length, and output length
  cases retain the caller's sentinel bytes.
- One-bit RLE and 32-bit RLE return `UnsupportedPixelFormat` before source
  parsing or output mutation. The one-bit limitation is explicit in the
  approved plan. Thirty-two-bit RLE is outside PS3.5 Table 8.2.2-1.
- `DecodeSampleLayout` keeps layout separate from Photometric Interpretation.
  Raw reports `Preserved`. RLE and JPEG report `Interleaved`. The RGB RLE
  fixture proves plane-to-pixel interleaving.
- Native plus RLE registration preflights all four UIDs before insertion and
  leaves Deflated Explicit VR Little Endian `KnownUnavailable`. The corpus
  integration proves Deflate remains an ingest-owned whole-data-set route and
  selects `DispatchPath::DeflatedExplicitVrLittleEndian`.
- The ignored corpus integration compared Implicit VR LE, Explicit VR LE,
  Explicit VR BE, RLE, and Deflate against the synthetic mono16 reference.
  `bin/ocelli.sh gate corpus` exited 0 with 92 verified rows and both ignored
  Rust corpus tests passing.
- F-024 JPEG compatibility remains intact after adding Pixel Data VR and sample
  layout evidence. The full codec suite exercised all four JPEG UIDs. The
  workspace suite also exercised F-018 stored-value and LUT tests.
- `bin/ocelli.sh test ocelli-codec` exited 0 with 39 tests passing.
- `bin/ocelli.sh test ocelli-dicom` exited 0 with 112 tests passing and two
  corpus tests ignored outside their gate.
- `bin/ocelli.sh gate test clippy` exited 0. The complete workspace test and
  clippy gates were green.
- `bin/ocelli.sh gate native` exited 0. Native entry points linked, all shared
  crates including `ocelli-codec` and `ocelli-dicom` compiled for
  `wasm32-unknown-unknown`, native all-target checks passed, and 17 direct
  dependency feature sets agreed across targets. This is compile evidence, as
  the plan and LLD state.
- `bin/ocelli.sh gate unsafe prose backlog deviations content provenance`
  exited 0. No new unsafe, patient data, build artefact, provenance violation,
  voice violation, backlog inconsistency, or undeclared deviation was found.
- The staged additions contain no `as` cast or rounding decision. Raw and RLE
  decode allocate no per-call storage.
