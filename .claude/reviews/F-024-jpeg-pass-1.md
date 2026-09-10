# F-024 JPEG review, pass 1

**Reviewed**: staged working tree on `work/f-024-codex` at base `586e503`
**Result**: 6 defects, 0 smells, 0 nitpicks

## Defects

### D1, JPEG Extended rejects the standard 8-bit process it claims to support

**Where**: `crates/ocelli-codec/src/jpeg.rs:212`

**What**: The `.51` adapter requires SOF1 precision 12 and
`decode_extended` requires 12-bit monochrome in a 16-bit container. It
therefore rejects every 8-bit process 2 codestream while registering `.51` as
available and documenting JPEG Extended support.

**Why it is wrong**: DICOM PS3.5 Table A.4-3 assigns both process 2 at 8 bits
and process 4 at 12 bits to UID `1.2.840.10008.1.2.4.51`. The approved plan's
parity claim says unsupported precision cannot be reported as available.

**Evidence**: Current DICOM PS3.5 A.4.1 lists `.51` as `2(8-bit),4(12-bit)`.
Inspection of `validate_header` found `(0xc1, 12)` for every Extended frame,
and inspection of `decode_extended` found an unconditional
`bits_stored() != 12` refusal.

### D2, the baseline conformance test has no truth comparison

**Where**: `crates/ocelli-codec/tests/jpeg.rs:82`

**What**: The only decoded-buffer assertion for `.50` is that six output bytes
are not all the sentinel value. An arbitrary non-sentinel buffer passes. No
expected pixels, independent decode, or class-two difference measurement is
present.

**Why it is wrong**: `CURRENT_SPRINT.md` says every supported JPEG UID is
compared with corpus truth. The approved F-024 plan says `.50`, `.51`, `.57`,
and `.70` buffers are compared with the uncompressed synthetic ramp or
independent truth and says the baseline row uses its declared class-two
measurement. The authoring diff weakens the conformance row from
manifest-backed evidence to any synthetic bytes while leaving the claimed
outcome unproved.

**Evidence**: Replacing the meaning of the decoded bytes with any value other
than `0xa5` would still satisfy `assert_ne!(out, [0xa5; 6])`. The corpus
generator already identifies `reference_rgb8.dcm` as the independent source
for `jpeg_baseline_rgb8.dcm`, but neither truth is exercised by this test.

### D3, the four-decoder registration helper is not atomic

**Where**: `crates/ocelli-codec/src/jpeg.rs:69`

**What**: `register_jpeg_decoders` mutates the registry four times. A collision
on `.51`, `.57`, or `.70` returns an error after earlier JPEG UIDs have already
become available. Its documentation acknowledges the partial mutation instead
of preserving the registry invariant.

**Why it is wrong**: Deviation D-19 requires complete declarations to be
preflighted so a later collision cannot leave partial registration. A public
helper that declares the complete JPEG set reintroduces that exact failure at
the next layer.

**Evidence**: The four sequential calls at lines 77 through 80 use `?` after
each mutation. With `.70` already registered, the first three calls succeed
before the fourth returns `AlreadyRegistered`.

### D4, D-21 does not authorize the encoded-input allocation used by `.51`

**Where**: `crates/ocelli-codec/src/jpeg.rs:173` and
`docs/hld/DEVIATIONS.md:38`

**What**: `.51` copies the complete compressed frame with `src.to_vec()` on
every decode. The LLD discloses this, but the normative deviation only permits
receiving library-owned decoded storage and justifies dependencies that return
owned output. The plan then states that D-21 records the bounded allocation,
which is false for this additional encoded-input copy.

**Why it is wrong**: HLD section 21 says one decode call must not allocate.
Every exception to that contract must be present in the deviation register,
not only in a lower-level implementation note.

**Evidence**: The exact dependency feature graph confirms that `.51` reaches
the `oxideav-core` packet API. The production call constructs that packet from
`src.to_vec()`, while D-21 names only owned decoded output.

### D5, the approved plan falsely says decoder scratch is pre-sized at construction

**Where**: `.claude/plans/F-024-design.md:109`

**What**: The plan claims decoder scratch is pre-sized at construction.
`JpegDecoder` construction stores only an enum. Both third-party decoder values
and all owned buffers are created inside each `decode` call.

**Why it is wrong**: `/microscope` treats a false factual statement in an
approved plan as a blocking defect. This sentence also misstates the measured
allocation boundary.

**Evidence**: The four constructors are `const` enum initializers. Decoder
construction occurs at `jpeg.rs:132` and `jpeg.rs:170`, inside `decode`.

### D6, the output-description query does not itself prevent double conversion

**Where**: `crates/ocelli-codec/src/registry.rs:59`,
`crates/ocelli-codec/tests/jpeg.rs:85`, and `docs/lld/codecs.md:84`

**What**: Source comments, the test comment, and the LLD say the new enum
prevents a later pixel stage from converting JPEG colour twice. The enum is an
optional query and there is no consumer or type-state connection that requires
a caller to use it. A caller can decode and ignore the query.

**Why it is wrong**: The API records or exposes colour ownership, which is the
F-024 scope, but it does not enforce the downstream behavior the prose claims.
False prevention claims obscure the remaining integration obligation.

**Evidence**: `Registry::decode` does not return the description, and
`decode_photometric_interpretation` is a separate method. Repository search
found no production call site of that query outside its definition.

## Smells

None.

## Nitpicks

None.

## Verified clean

- Exact UID dispatch remains case-sensitive and has no fallback.
- Caller output is checked before dependency decode and copied only after full
  validation, so inspected refusal paths are atomic.
- SOI, supported SOF, SOS, EOI, trailing-data, dimensions, components,
  precision, pixel format, stride, and output length are checked.
- `.70` checks selection value 1 in every scan, matching DICOM PS3.5 A.4.1 and
  section 10.2.
- `.57` and `.70` fixture expectations are hand-computed and byte-exact.
- `.51` uses the unchanged HLD mono16 predicate against an independent DCMTK
  result.
- The release benchmark clocks each real decode call and keeps process startup,
  fixture setup, decoder construction, and caller output allocation outside
  the timer. Allocations inside `decode` remain measured.
- Exact dependency versions, features, licence evidence, wasm compilation,
  and the eleven-site external `oxideav-core` unsafe audit are disclosed.
- No repository `unsafe`, `wasm-bindgen`, patient data, render-loop work, or
  network boundary was added.
- Staged diff whitespace checking passed.
