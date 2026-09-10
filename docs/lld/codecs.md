# Codec registry

**F-IDs that contributed:** F-023, F-024
**Last updated:** 2026-09-10

`ocelli-codec` owns decoder capability, registration, and exact Transfer
Syntax UID dispatch. Its first concrete adapter covers JPEG Baseline `.50`,
JPEG Extended `.51`, JPEG Lossless `.57`, and JPEG Lossless SV1 `.70`. The
crate builds for native and `wasm32-unknown-unknown` from the same Rust
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

## Frame description and output ownership

`FrameDescInput` names every field before validation so adjacent DICOM integer
fields cannot be swapped as positional constructor arguments. `FrameDesc`
retains rows, columns, samples per pixel, Bits Allocated, Bits Stored, High Bit,
Pixel Representation, and Photometric Interpretation.

Rows and Columns are `u16`, matching their DICOM PS3.6 `US` value
representations. They are not widened to manufacture host-only test inputs.

Construction refuses zero dimensions, sample containers other than 8, 16, or
32 bits, Bits Stored outside its container, High Bit other than Bits Stored
minus one, and an output byte length that cannot be represented on the target.
The High Bit equality is required by DICOM PS3.3 C.7.6.3.3. The validated byte
length is compared with the caller's output slice before the decoder is called.
This is size validation, not pixel arithmetic. Stored-bit unpacking remains
owned by later codec and pixel stories.

The immutable `FrameDesc` and `Result<(), CodecError>` return cannot report a
colour layout changed by a decoder. `DecodePhotometricInterpretation` is the
typed output description for that fact. Its default trait implementation is
`Preserved`, so raw, RLE, and JPEG 2000 adapters keep the input description
without new code. JPEG reports `Rgb` after its dependency converts a
three-sample colour codestream. Registry forwards the same query by exact UID.
This reports the conversion for a future pixel-stage consumer but does not
enforce its use. No production consumer is wired in F-024.

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
decoder sees a wrongly sized destination. JPEG additionally distinguishes an
invalid codestream, excess or non-padding trailing data, frame metadata
mismatch, unsupported pixel format, and dependency decode failure. Dispatch
propagates these results unchanged.

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

The `decode.frame` benchmark begins at one real `Decoder::decode` call and ends
when that call returns. Its first runner uses the release `.51` adapter over
the 96 by 64 synthetic corpus frame. Process startup, fixture setup, decoder
construction, and caller output allocation are excluded. Allocation performed
inside `Decoder::decode` remains included because removing it would move the
specified boundary.
