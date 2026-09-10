# Codec registry

**F-IDs that contributed:** F-023
**Last updated:** 2026-09-09

`ocelli-codec` owns decoder capability, registration, and exact Transfer
Syntax UID dispatch. It contains no concrete codec yet. The crate builds for
native and `wasm32-unknown-unknown` from the same Rust implementation.

## Ownership and extension point

HLD section 21 defines `Decoder` as a `Send + Sync` runtime extension point.
Each decoder declares a static list of exact Transfer Syntax UIDs and decodes
one frame into a caller-provided output slice. The registry stores shared
decoders as `Arc<dyn Decoder>` so one adapter can serve every syntax it owns.

Registration is setup work. It may allocate the hash tables and clone `Arc`
handles. Capability lookup and one-frame dispatch borrow existing state and do
not allocate. A decoder remains responsible for meeting section 21's rule that
one `decode` call does not allocate.

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
or common syntax.

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
This is size validation, not pixel arithmetic. Stored-bit unpacking and colour
conversion remain owned by later codec and pixel stories.

The checked byte-length calculation first multiplies the conforming DICOM
fields in `u64`, then converts the result to the target length type. Production
instantiates it as `usize`. A host unit test instantiates the identical helper
as `u32` and proves that maximum conforming `US` dimensions overflow a 32-bit
output length while the one-byte form remains representable.

## Dispatch failures

`CodecError::UnknownTransferSyntax` and `KnownUnavailable` preserve the
capability distinction at dispatch. `OutputLength` is returned before a
decoder sees a wrongly sized destination. A concrete adapter can report
`DecoderFailure`, and dispatch propagates it unchanged.

Error text contains no source bytes, metadata values, paths, or identifiers.

## Worker and tier boundary

The registry is used by decode workers. HLD section 24 says those workers never
touch the GPU, so rendering tiers A, B, and C do not select different registry
behavior. `ocelli-codec` imports neither `wgpu` nor `wasm-bindgen`, and it uses
no unsafe code.

The resolved codec routes remain future adapter work:

- JPEG-LS uses `pure_jpegls` 2.0.0 subject to the production story's coverage
  and conformance gates.
- HTJ2K uses `openjph-core` 0.1.0 or a reviewed successor subject to the
  production gates in `docs/spikes/A1-htj2k-route.md`.

Until those adapters land, their known UIDs remain `KnownUnavailable`.

## Measurement boundary

The `decode.frame` benchmark begins at one real `Decoder::decode` call and ends
when that call returns. F-023 supplies dispatch but no production decoder, so
there is no honest frame-decode runner or number yet. The benchmark reports
`unavailable` with reason `no_runner` after the story is marked done.
