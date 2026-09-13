# Codec registry

**F-IDs that contributed:** F-023, F-024, F-025, F-026, F-027, F-028
**Last updated:** 2026-09-13

`ocelli-codec` owns decoder capability, registration, and exact Transfer
Syntax UID dispatch. Concrete adapters cover native Implicit and Explicit VR
Little Endian, native Explicit VR Big Endian, RLE Lossless, JPEG Baseline
`.50`, JPEG Extended `.51`, JPEG Lossless `.57`, and JPEG Lossless SV1 `.70`.
The crate builds for native and `wasm32-unknown-unknown` from the same Rust
implementation.

`bin/ocelli.sh gate native` proves both native and wasm compilation, and the
focused `cargo check -p ocelli-codec --all-targets --target
wasm32-unknown-unknown` isolates the codec half of that proof. This is compile
evidence. Adapter fixtures execute natively. `bin/ocelli.sh wasm` builds
`ocelli-wasm`, which does not depend on `ocelli-codec`, and no browser runner
executes the JPEG adapter in F-024.

## Ownership and extension point

HLD section 21 defines `Decoder` as a `Send + Sync` runtime extension point.
Each decoder declares a static list of exact Transfer Syntax UIDs and decodes
one frame into a caller-provided output slice. The registry stores shared
decoders as `Arc<dyn Decoder>` so one adapter can serve every syntax it owns.

The public `Decoder::decode` contract reflects D-21. Registry lookup and
dispatch add no allocation of their own. Raw plus RLE implementations remain
allocation-free per call. Concrete JPEG and JPEG 2000 adapters may allocate
bounded dependency-owned decoded storage and may copy encoded input when a
safe packet API requires owned bytes. Every adapter retains caller-owned
output and must not modify it before complete success.

Registration is setup work. It may allocate the hash tables and clone `Arc`
handles. Capability lookup and one-frame dispatch borrow existing state and do
not allocate. JPEG decode uses bounded dependency-owned frame storage before
one atomic copy into the caller's buffer. Deviation D-21 records why the safe
dependency APIs cannot meet section 21's no-allocation sentence.

## JPEG 2000 Part 1

F-026 registers separate adapters for `.90` and `.91`. A bounded main-header
inspector owns SIZ geometry, precision and signedness checks, COD transform and
MCT checks, QCD quantization-style and segment-length checks, and terminal EOC
plus necessary DICOM NULL padding. The supported domain is one MONOCHROME1 or
MONOCHROME2 component in an 8-bit or 16-bit signed or unsigned container with
identity rescale. Per DICOM PS3.5 A.4.4, `.90` requires reversible 5/3 and QCD
no-quantization style. `.91` accepts reversible 5/3 or irreversible 9/7 with
any valid Part 1 QCD style. Missing or duplicate QCD, invalid style lengths,
and QCC quantization override are refused before decode. Colour and MCT are
refused.

A deterministic fixture generator rewrites the tracked reversible fixture's
expounded QCD segment into a legal scalar-derived segment and asks OpenJPEG
2.5.4 for the independent reference bytes. The resulting `.91` fixture proves
QCD style 1 is decoded, while the `.90` mode refuses the same codestream before
decode because A.4.4 requires no quantization.

The vendored decoder returns owned `f32` samples. The adapter accepts only a
complete output of exact length whose samples are finite, integral and in the
declared stored-value range. It builds canonical little-endian bytes away from
the caller buffer and copies only after every sample passes. This is the
bounded allocation D-21 permits and preserves atomic caller output.

The manifest-backed `.90` frame equals the uncompressed synthetic truth. The
`.91` frame differs at 418 of 6,144 samples, with signed sum 30, maximum
absolute difference 1 and no difference over 1. Against the independent
pydicom and OpenJPEG decode it differs at 1,232 samples, with signed sum 730,
maximum absolute difference 1 and no difference over 1. Both satisfy the
unchanged HLD 25.1 mono16 predicate. The native gate executes these checks
natively and in Node from plain and `+simd128` wasm modules. The wasm gate does
not exercise this codec.

## JPEG-LS

`1.2.840.10008.1.2.4.80` lossless and `1.2.840.10008.1.2.4.81` near-lossless,
through the already-vendored `ritk-codecs` 0.6.0 `jpeg_ls` module under
deviation D-20. Appendix A gate A2 resolved the route as pure Rust and
recommended `pure_jpegls` 2.0.0. The S09 design round selected the vendored
package instead, because gate A2's one advantage for `pure_jpegls` was
multi-component support and
`vendor/ritk-codecs-0.6.0/src/jpeg_ls/decoder.rs` refuses `Nf != 1` explicitly
too, leaving a second vendored package as the only remaining difference.
**Both routes are single-component only, and both refuse rather than producing a
wrong pixel.**

### The two UIDs are not interchangeable

PS3.5 A.4.3 and A.4.4. `.80` is lossless, which ISO/IEC 14495-1 defines as
`NEAR = 0`, and `.81` is near-lossless, which is `NEAR > 0`. The adapter reads
`NEAR` from the SOS marker segment and refuses the wrong pairing as
`FrameMismatch` in both directions. Without that check either decoder would
accept either codestream and the lossless claim would be unfalsifiable.

**`NEAR` sits two bytes per component past `Ns`.** ISO/IEC 14495-1 C.2.3 gives
the segment as `Ls Ns (Cj Tmj)*Ns NEAR ILV Al/Ah`. Reading it from the end of
the segment instead lands on `ILV`, which is zero for every non-interleaved
frame and therefore looks like a lossless `NEAR` on a near-lossless codestream.
A unit test pins the exact payload `pyjpegls` 1.5.1 emits, including a row where
`ILV` is non-zero, so the correct index is separated from the plausible one.

### What is refused, and where

| Condition | Where | Error |
|-----------|-------|-------|
| Samples per Pixel other than one | descriptor | `UnsupportedPixelFormat` |
| Bits Allocated other than 8 or 16 | descriptor | `UnsupportedPixelFormat` |
| A colour or palette interpretation | descriptor | `UnsupportedPixelFormat` |
| Other Word Pixel Data | descriptor | `UnsupportedPixelFormat` |
| SOF55 `Nf != 1` or SOS `ILV != 0` | codestream | `UnsupportedPixelFormat` |
| Dimensions or precision disagreeing with the descriptor | codestream | `FrameMismatch` |
| `NEAR` disagreeing with the UID | codestream | `FrameMismatch` |
| A stray SOI or EOI where a marker segment belongs | codestream | `InvalidCodestream` |
| Bytes after a complete image | codestream | `TrailingData` |

**The descriptor and codestream checks are not redundant.** The descriptor check
catches a frame declared as colour. The codestream check catches a
**single-sample descriptor paired with a multi-component codestream**, which is
the file that claims to be monochrome and is not, and no descriptor check can
see it.

A multi-component JPEG-LS corpus row is still owed, which gate A2 recorded. The
adapter refuses one cleanly, so the gap is coverage rather than a wrong pixel.

### The modality rescale is pinned to the identity

`decode_jpeg_ls_fragment` takes a `PixelLayout` carrying `rescale_slope` and
`rescale_intercept`, so the dependency can apply the modality LUT. HLD section
18 requires that arithmetic to exist exactly once, in `ocelli-pixel`. The
adapter pins slope `1.0` and intercept `0.0` at its one call site, which is the
same pin `jpeg2000.rs` uses, and a fixture asserts that decoded sample `n` is
exactly `n` over a full ramp. Moving either value fails eight of the fifteen
JPEG-LS tests.

### The stored-domain boundary

The dependency returns `Vec<f32>` and the stored domain is integer. The
conversion back lives in `sample_convert`, shared with the JPEG 2000 adapter
rather than copied, and refuses any sample that is not exactly an integer inside
the descriptor's declared stored range.

**The round trip is proven over the full range rather than over a sample.** A
256 by 256 fixture carries all 65,536 unsigned 16-bit values exactly once, and
the test asserts both that the decode is byte-identical to the constructed ramp
and that every distinct value appeared exactly once. Every integer through 24
significant bits is exact in `f32`, so 16-bit stored values are exact by
construction, and the fixture asserts that rather than assuming it.

### Anchors, and which are independent of CharLS

`pyjpegls` 1.5.1 wraps CharLS and encoded both corpus rows, so agreeing with
another CharLS reading is not independent evidence. Two anchors are:

- **`.80`**: the uncompressed `syntax/explicit_vr_le.dcm` Pixel Data, which is
  the same synthetic ramp from `scripts/corpus_synth.py` and comes from no
  codec. The syntax is lossless, so the decode must reproduce it exactly.
- **`.81`**: ISO/IEC 14495-1's guarantee that the maximum absolute error is at
  most `NEAR`, which is the standard rather than an implementation. The test
  asserts zero samples over the bound **and** that the maximum error equals it,
  because a decode that came back bit-identical would satisfy the first half
  while meaning the near-lossless path had not run.

`.81` is lossy by design, so "the output differs from the source" is expected
there and is not evidence of a defect. The same statement about `.80` would be.

## HTJ2K

`1.2.840.10008.1.2.4.201`, `.202` and `.203`, JPEG 2000 Part 15, through
`openjph-core` 0.1.0 vendored under deviation **D-22**. Appendix A gate A1
measured `openjp2` 0.6.1 as a failure on `wasm32-unknown-unknown`, and F-X013
priced this candidate as the one route using the same Rust implementation on
browser, desktop and server. No package in the vendor tree implements HTJ2K
otherwise: `ritk-codecs`'s `jpeg_2000` module carries `mq_coder` and
`wavelet_9_7` with no HT block coder and no CAP handling.

### CAP is what makes a codestream HTJ2K

The CAP marker `(0xff50)` is the extended capability descriptor, and its
presence is what separates an HTJ2K codestream from JPEG 2000 Part 1. The
adapter parses it itself rather than asking the dependency, because
`openjph-core` keeps its `ParamCap` `pub(crate)` and because trusting a library
to have read the marker that decides what the file *is* would be trusting it
with the question. A codestream without CAP is `FrameMismatch`, asserted with
the Part 1 `.90` corpus fixture, which is the same synthetic ramp and passes
every other check.

### The three syntaxes, and the asymmetry between two of them

| UID | Transform | Progression |
|-----|-----------|-------------|
| `.201` | reversible 5/3 required | unconstrained |
| `.202` | reversible 5/3 required | **RPCL required** |
| `.203` | unconstrained | unconstrained |

PS3.5 A.4.10 requires RPCL for `.202`. A.4.9 constrains nothing about
progression for `.201`. **So a `.202` codestream is also a valid `.201` one,
and the two are not mutually exclusive.** That asymmetry is the standard's and
is not smoothed over: inventing a non-RPCL constraint for `.201` to make the
pair disjoint would refuse conformant files. The fixtures make both directions
visible, the `.201` corpus row being LRCP and the `.202` row RPCL, and a test
asserts that `.201` **accepts** the RPCL row.

What separates the two lossless syntaxes from `.203` is the wavelet transform,
not the progression order. The corpus `.203` row is RPCL as well, so progression
does not partition the three at all.

### `.203` and decision D14

`.203` is irreversible, so the uncompressed reference is not an anchor for it:
against the ramp this decode differs at 5,377 of 6,144 samples, which is what a
lossy codec does. What is pinned instead is the candidate's own output digest,
`ce4a2bb9d75b897292a4e9e9e7455447e976f5ff1e18b4ecb3219bb17d4d062c`, which
F-X013 recorded identically for its native, plain wasm and `+simd128` builds and
which the production adapter reproduces exactly.

**Pinned rather than bounded, and no tolerance is introduced.** A tolerance
would pass if the output moved within it. The digest fails if the decode changes
at all, which is the stronger statement and is what D14 means by claiming a
measured divergence. F-X013's further comparison against OpenJPH 0.31.0, 41 of
6,144 samples differing by one, used `ojph_expand`, an external tool this tree
does not carry, so that figure stays in the spike record and would have to be
re-measured rather than assumed if the digest ever moved.

### Cross-target identity is standing, not a spike result

`bin/ocelli.sh gate native` steps 9 to 11 run `examples/verify_htj2k.rs` over
all three syntaxes natively, as plain wasm and as `+simd128` wasm. That is the
standing version of F-X013's central claim, through the production adapter
rather than the throwaway harness, which is deleted.

### The allocation cost, which is D-21's largest instance

`openjph-core` 0.1.0 has no API that decodes into a caller-provided slice. Each
`pull` returns a fresh `Vec<i32>` for one image row, so a 64-row frame costs at
least 65 allocations. Every row is validated before the caller's output is
touched and the write is one `copy_from_slice`, so an error leaves the buffer
byte-unchanged. The `decode.transfer_syntax.htj2k` benchmark measures the cost
rather than describing it.

### Sample conversion, and why there is no cast

The dependency returns `i32`. `Stored`'s representation is `f32`, which carries
24 significant bits, so an arbitrary `i32` can round. The adapter accepts Bits
Allocated 8 or 16 only, so every legitimate sample lies in
`-32_768 ..= 65_535`, the union of the signed and unsigned 16-bit stored
domains. `exact_f32` converts through `i16::from` and `u16::from`, both
lossless, and refuses anything outside that union. `i32 as f32` would round
silently on exactly the values the function exists to catch, and a unit test
sweeps all 98,304 values of the union rather than sampling them.

### The redistribution condition, which is open

`openjph-core` 0.1.0's published archive carries no `LICENSE`, `LICENCE`,
`COPYING` or `NOTICE`, and crates.io reports no repository URL to obtain one
from. Its metadata declares BSD-2-Clause, which permits redistribution **with
its notice conditions retained**.

`python3 scripts/pin_and_size_check.py --require-redistribution` refuses while
that material is absent and `/release` step 5 runs it. **No development profile
does**, because `--floor`, `--sprint` and `--all` gate development and this
gates publication. The refusal names the file that would close it. Probe
`pins.openjph-notice` proves both directions: it fires today, and it stops
firing once a notice is placed.

The package's 104 `unsafe` constructs are a recorded number rather than an
invisible one. `scripts/unsafe_allowlist_check.py` refuses a vendored package
whose count moves, one with no record, and a record for a package not in the
tree. The per-file audit is in `docs/SOURCE-POLICY.md`.

## Known does not mean available

`KNOWN_TRANSFER_SYNTAXES` is the explicit sixteen-UID catalogue shared with the
corpus surface. A new `Registry` begins with every one of those UIDs in
`KnownUnavailable`. No native or compressed syntax is inferred to work merely
because its UID is common or because `ocelli-dicom` can parse its data set.

The three capability states are:

| State | Meaning |
|-------|---------|
| `Available` | The UID is known and a decoder is registered |
| `KnownUnavailable` | The UID is known and this build has no decoder |
| `Unknown` | The UID is outside the registry's explicit catalogue |

Lookup is an exact string match. It never falls back from one UID to a related
or common syntax. Four separately configured `JpegDecoder` values retain that
exact UID at decode time. This is required because `Decoder::decode` receives
no transfer-syntax argument and `.70` must enforce selection value 1 rather
than accept every process-14 codestream.

## Atomic registration

One decoder may declare several UIDs. `Registry::register` preflights the
complete declaration before changing the decoder map. It refuses:

- a decoder with no declared UID
- an empty UID
- a UID repeated by the same decoder
- a UID outside the known catalogue
- a UID already owned by another decoder

Every refusal leaves all mappings unchanged. Deviation D-19 records why this
differs from HLD section 21's example, whose `HashMap::insert` silently replaces
an existing decoder. Registration order therefore cannot change which decoder
runs for an already available UID.

The public `register_jpeg_decoders` helper preflights all four exact JPEG UIDs
before its first insertion. A collision on `.51`, `.57`, or `.70` therefore
cannot leave an earlier JPEG capability partially registered.

`register_native_and_rle_decoders` applies the same all-or-nothing preflight to
Implicit VR Little Endian, Explicit VR Little Endian, Explicit VR Big Endian,
and RLE Lossless. Deflated Explicit VR Little Endian is intentionally absent.
Its compressed stream wraps the data set rather than one frame and remains an
`ocelli-dicom` ingest responsibility. It therefore remains
`KnownUnavailable` in the frame registry even when all F-025 adapters are
registered.

## Frame description and output ownership

`FrameDescInput` names every field before validation so adjacent DICOM integer
fields cannot be swapped as positional constructor arguments. `FrameDesc`
retains rows, columns, samples per pixel, Bits Allocated, Bits Stored, High Bit,
Pixel Representation, and Photometric Interpretation.

Rows and Columns are `u16`, matching their DICOM PS3.6 `US` value
representations. They are not widened to manufacture host-only test inputs.

Construction refuses zero dimensions, sample containers other than 1, 8, 16,
or 32 bits, Bits Stored outside its container, High Bit other than Bits Stored
minus one, and an output byte length that cannot be represented on the target.
The High Bit equality is required by DICOM PS3.3 C.7.6.3.3. The validated byte
length is compared with the caller's output slice before the decoder is called.
This is size validation, not pixel arithmetic. Stored-bit unpacking remains
owned by later codec and pixel stories.

`FrameDesc` also retains typed `PixelDataVr::Ob` or `PixelDataVr::Ow` evidence.
The native adapter uses it to distinguish byte-order-insensitive OB bytes from
physical 16-bit OW words. Encapsulated RLE and JPEG accept OB only.

The immutable `FrameDesc` and `Result<(), CodecError>` return cannot report a
colour interpretation or sample layout changed by a decoder.
`DecodePhotometricInterpretation` and `DecodeSampleLayout` are the typed output
descriptions for those facts. Raw decoding reports `Preserved` layout. RLE and
JPEG report `Interleaved`. JPEG also reports `Rgb` after its dependency
converts a three-sample colour codestream. Registry forwards both queries by
exact UID. This reports conversions for a future pixel-stage consumer but
does not enforce their use. No production consumer is wired in F-025.

## Native Pixel Data normalization

Each native UID has a separate raw decoder value because the decode call does
not receive the selected UID. Implicit VR Little Endian requires Pixel Data VR
OW. Explicit VR Little Endian and Explicit VR Big Endian retain the legal
eight-bit OB option. OB bytes are copied in source order under explicit native
syntaxes. OW is a sequence of physical 16-bit words, so the big-endian decoder
swaps the two bytes of every complete word. This rule also applies when Bits
Allocated is 8 or 32. It does not swap a complete 32-bit sample as one integer.

`Decoder::decode` receives exactly one logical frame. It validates the exact
logical source and output lengths, but does not infer complete-Value padding
from that frame. Direct OB decode copies the logical frame bytes. Direct
big-endian OW decode normalizes complete physical words when the caller has
already isolated a frame on a word boundary. It refuses an odd logical source
length because those bytes cannot represent complete physical OW words. Such
a frame must use `NativeFrameIndex`. OB with more than eight allocated bits is
refused because it cannot carry the required word-order evidence.

`NativeFrameIndex` is the fallible owner for a complete native Pixel Data
Value. Construction through the exact-UID `RawDecoder` requires a positive
Number of Frames, validates the descriptor once, and records only checked
frame count, bit stride, Value Representation, and byte order evidence. The
complete OB Value is rounded to even length and its necessary trailing byte
must be NULL. The complete OW Value must contain whole physical 16-bit words,
but bits and bytes outside the final Pixel Cell are insignificant and need not
be zero.

Native frames are concatenated without per-frame padding. Frame extraction
therefore uses the checked global bit offset into the complete Value. Pixel
Cells are read least-significant bit first, including when a later one-bit
frame starts in the middle of a byte or word. Big-endian OW byte mapping is
applied at the Value word boundary before each requested frame is repacked as
canonical little-endian output. Construction and frame decode allocate
nothing. An out-of-range frame, wrong output length, or invalid complete Value
is refused before any caller output byte is changed.

## RLE validation and output

RLE Lossless parses the fixed 64-byte little-endian header and requires exactly
one segment for every sample byte plane. Used offsets begin at 64, are even,
strictly increase, and stay within source bounds. Unused offsets are zero.
Segments are ordered most-significant byte plane first for each sample.

The stateless decoder makes two allocation-free PackBits passes. The first
validates every row, run, decoded byte count, segment boundary, and physical
pad without writing. The second repeats the proven traversal and writes
interleaved pixels in canonical little-endian sample order. A run may not cross
an image row. Control byte `0x80` is a legal zero-output operation. A segment
whose semantic stream length is odd has exactly one trailing zero pad, while
an even semantic stream has none. Truncated runs, surplus output, bad padding,
and trailing compressed bytes are structural refusals that preserve caller
output.

The decoder also enforces PS3.5 Table 8.2.2-1 before parsing input.
MONOCHROME1 and MONOCHROME2 require one sample. PALETTE COLOR requires one
unsigned sample. RGB and YBR_FULL accept either eight-bit or sixteen-bit
samples, requiring three unsigned samples in both cases. Other Photometric
Interpretation, Samples per Pixel, Bits Allocated, and Pixel Representation
combinations are `UnsupportedPixelFormat`.

Eight-bit and sixteen-bit RLE are supported. One-bit RLE is explicitly
`UnsupportedPixelFormat` because the current RLE output contract reports
byte-interleaved samples and has no typed bit-packed layout. Thirty-two-bit RLE
is also explicitly unsupported, matching the standard photometric RLE
container scope.

## JPEG validation and output

The adapter parses enough JPEG marker structure to validate the boundary
before invoking either dependency. It requires one SOI, a complete SOF and
SOS, and one terminal EOI. DICOM PS3.5 A.4 requires even-length Fragment Item
Values and permits ISO 10918-1 FF fill before a marker so EOI ends on an even
boundary. It also permits the single trailing NULL needed after EOI when the
compressed stream is odd length. The adapter accepts both forms, removes an
external NULL before dependency decode, and rejects excess or non-padding
trailing data. SOF width, height, component count, sample precision, process
marker, and DICOM container width must match the validated `FrameDesc`. `.50`
accepts SOF0 at eight bits, `.51` accepts SOF1 at eight or twelve bits, `.57`
accepts lossless SOF3, and `.70` additionally requires predictor selection
value 1 in every scan.

`jpeg-decoder` 0.3.2 handles `.50`, eight-bit process 2 for `.51`, `.57`, and
`.70`. The adapter validates the dependency's dimensions, coding process,
pixel format, and exact output length. Sixteen-bit grayscale samples are
normalized to DICOM little-endian containers on a big-endian target before
the atomic copy.

`oxideav-mjpeg` 0.1.8 handles twelve-bit process 4 for `.51`. Version 0.1.8's documented
standalone entry point is not public in its published source, so the adapter
uses the operator-approved `registry` feature and its public
`registry::make_decoder` route. The lockfile resolves `oxideav-core` 0.1.35.
The twelve-bit `.51` route accepts only one-component frames in sixteen-bit
containers because that public result does not expose a pixel format and
returns colour as multiple YUV planes. Eight-bit process 2 supports both
grayscale and packed RGB through `jpeg-decoder`. Claiming twelve-bit colour
availability without a typed planar contract would be false.

The two dependencies allocate owned decode output. The `.51` route also must
copy the compressed input into `oxideav-core::Packet`. The adapter validates
the final plane count, stride, and exact byte length, moves the owned plane
storage out where possible, and copies into `out` only after all work succeeds.
Every refusal and dependency failure leaves every caller-buffer byte unchanged.

The published `oxideav-mjpeg` source contains no unsafe code. Its public route
expands the graph to `oxideav-core` 0.1.35, whose arena support contains eleven
audited unsafe sites, four unsafe implementations and seven unsafe blocks.
The audit matches whole-word `unsafe` over every `*.rs` file below the exact
published package's `src/` directory. That is external dependency code, not
repository unsafe. Both packages carry MIT licence files and metadata.

The checked byte-length calculation first multiplies the conforming DICOM
fields in `u64`, then converts the result to the target length type. Production
instantiates it as `usize`. A host unit test instantiates the identical helper
as `u32` and proves that maximum conforming `US` dimensions overflow a 32-bit
output length while the one-byte form remains representable.

## Dispatch failures

`CodecError::UnknownTransferSyntax` and `KnownUnavailable` preserve the
capability distinction at dispatch. `OutputLength` is returned before a
decoder sees a wrongly sized destination. Concrete adapters additionally
distinguish an invalid codestream, excess or non-padding trailing data, frame
metadata mismatch, unsupported pixel format, and dependency decode failure.
Dispatch propagates these results unchanged.

Error text contains no source bytes, metadata values, paths, or identifiers.

## Worker and tier boundary

The registry is used by decode workers. HLD section 24 says those workers never
touch the GPU, so rendering tiers A, B, and C do not select different registry
behavior. `ocelli-codec` imports neither `wgpu` nor `wasm-bindgen`, and it uses
no unsafe code.

The remaining resolved codec routes are future adapter work:

- JPEG-LS uses `pure_jpegls` 2.0.0 subject to the production story's coverage
  and conformance gates.
- HTJ2K uses `openjph-core` 0.1.0 or a reviewed successor subject to the
  production gates in `docs/spikes/A1-htj2k-route.md`.

Until those adapters land, their known UIDs remain `KnownUnavailable`.

## Evidence and measurement boundary

Synthetic fixtures contain no patient data. Lossless process 14 and SV1 decode
four hand-computed unsigned twelve-bit extrema exactly in DICOM little-endian
containers. The `.51` codestream is extracted from the deterministic synthetic
corpus row and compared with native samples independently decoded by DCMTK.
It satisfies HLD section 25.1's fixed mono16 predicate. The baseline fixture
is the deterministic synthetic corpus codestream and computes per-channel
class-two statistics against its native RGB reference as D-16 requires. An
eight-bit SOF1 process-2 fixture is independently decoded by DCMTK and likewise
fixes its class-two statistics as test evidence. The odd-length hand-computed
lossless fixture proves both legal A.4 padding forms. Refusal fixtures cover
truncation, excess or non-padding trailing data, dimension and precision
mismatch, wrong SV1 predictor, wrong output length, and dependency failure
with an unchanged caller buffer.

F-025 fixtures hand-compute native word order and RLE PackBits output. They
cover distinct 8-bit big-endian OB and OW Values, 32-bit OW word order, signed
containers retained as bytes, two-row literal and repeat runs, legal `0x80`,
six-plane RGB16 and YBR_FULL16 interleaving, row boundaries, exact segment
padding, malformed headers and runs, and permanent one-bit and 32-bit RLE
refusal. The ignored
corpus integration test compares native little endian, native big endian, RLE,
and Deflate rows against one synthetic native truth. It also proves Deflate
selects `DispatchPath::DeflatedExplicitVrLittleEndian` while the frame registry
reports `KnownUnavailable`.

The `decode.frame` benchmark begins at one real `Decoder::decode` call and ends
when that call returns. Its first runner uses the release `.51` adapter over
the 96 by 64 synthetic corpus frame. Process startup, fixture setup, decoder
construction, and caller output allocation are excluded. Allocation performed
inside `Decoder::decode` remains included because removing it would move the
specified boundary.
