# F-016, ocelli-dicom: parse and transfer-syntax dispatch over dicom-rs

**Status**: approved
**Epic ref**: E3.1
**Sprint**: S06
**Estimate**: 3w

## Normative source, transcribed

_The quotations below normalise the source's em dash to a hyphen because
`scripts/prose_check.py` covers this plan. No other word is changed. The
tracked HLD wins where exact bytes matter._

### `docs/hld/03-architecture-and-crates.md`, sections 3 and 4

> The shell is TypeScript on the main thread: DOM and pointer events, the SVG
> annotation layer, tool interaction state, framework bindings, and DICOMweb
> fetch and authentication. The core is Rust running in workers. Between them
> sits a deliberately narrow boundary carrying commands down, bulk bytes down,
> and events up.

| **Crate** | **Responsibility** | **wasm** | **native** |
|----|----|----|----|
| ocelli-dicom | Parsing, transfer-syntax dispatch, metadata model and providers | yes | yes |
| ocelli-codec | Decoder registry and codec adapters, registered at runtime | yes | yes |
| ocelli-wasm | The only crate that may import wasm-bindgen. Boundary, commands, event ring. | yes | no |

### `docs/hld/12-workspace-and-build.md`, Part II and sections 15.1 to 15.3

> This part is prescriptive. Where it gives a formula, a layout or a
> signature, that is the intended implementation and a deviation should be
> raised rather than improvised. It exists because the dangerous defect in
> medical imaging is not the crash - it is the pixel that is quietly wrong,
> and quietly wrong code is produced by reasonable people making locally
> reasonable choices.

> ocelli-dicom/ \# parse, metadata, providers

```toml
[workspace.dependencies]
wgpu = "=30.0.1" # pin EXACTLY - breaking changes ~quarterly
dicom = { version = "0.10", default-features = false }
glam = "0.30"
bytemuck = { version = "1", features = ["derive"] }
thiserror = "2"
```

> **DICOM-RS FEATURES** Disable default features. The defaults are rayon and
> simd, and the gdcm feature is native-only. On wasm you want:
> default-features = false, then jpeg, rle, deflate and openjp2 selected
> explicitly. Note also that the inventory-based transfer-syntax plugin
> registry does not work on wasm at all - which is why section 21 specifies an
> explicit runtime registry instead.

> Decision D2 is worthless unless it is enforced. This runs on every pull
> request.

The section 15.3 loop exempts only `ocelli-wasm` and fails if another crate
reaches `wasm-bindgen` on the host. Deviation D-12 keeps the source and direct
manifest checks unchanged, while allowing wgpu's target-specific transitive
route on wasm32.

### `docs/hld/18-codec-registry.md`, section 21

> Explicit runtime registration, not the inventory crate - inventory does not
> work on WebAssembly, which is precisely why dicom-rs's own plugin registry is
> unavailable there. Explicit registration is also what lets a native build
> link C codecs the browser build cannot.

```rust
pub trait Decoder: Send + Sync {
    fn transfer_syntaxes(&self) -> &'static [&'static str];
    /// Decode one frame into `out`. Must not allocate.
    fn decode(&self, src: &[u8], desc: &FrameDesc, out: &mut [u8])
    -> Result<(), CodecError>;
}
pub struct Registry { by_ts: HashMap<&'static str, Arc<dyn Decoder>> }
impl Registry {
    pub fn register(&mut self, d: Arc<dyn Decoder>) {
        for ts in d.transfer_syntaxes() { self.by_ts.insert(ts, d.clone()); }
    }
}
```

This story parses encapsulated pixel data into its retained fragment form. It
does not implement the `Decoder` trait or claim pixel decoding. F-023 owns that
runtime registry.

### `docs/hld/20-errors-and-panics.md`, section 23

> thiserror in the core crates, the boundary maps everything to a stable
> numeric code and a message. The important part is what happens when that is
> not enough.

> Error codes are stable and versioned. The shell switches on the code, the
> message is for humans and may change.

> A poisoned instance surfaces to the user as a viewport-level error state,
> never as a silent blank canvas.

F-016 has no boundary export and therefore adds no numeric boundary code. Its
Rust error variants distinguish invalid Part 10 structure, unknown transfer
syntax, unsupported data-set decoding, and invalid data set without carrying
attribute values or input bytes into errors. The boundary mapping arrives with
the worker protocol that can transport recoverable decode failures.

### `docs/hld/22-testing-and-tolerance.md`, section 25

| **Layer** | **What it proves** | **Where it comes from** |
|----|----|----|
| Unit and property | LUT arithmetic, geometry round-trips within epsilon | Hand-computed fixtures citing the DICOM section |
| Golden image | The rendered frame matches cornerstone3D | The oracle harness, over the corpus |
| Conformance | Each transfer syntax decodes correctly | Published DICOM test corpora |

F-016 claims data-set parsing and transfer-syntax dispatch, not pixel decoding.
Its conformance evidence therefore checks that native, deflated, and
encapsulated data sets reach the declared parser path and retain their encoded
pixel representation. Codec conformance remains with F-023 and its dependants.

### `docs/hld/24-agent-code-standards.md`, sections 27.2 and 27.3

| **#** | **Rule** | **Why** |
|----|----|----|
| R2 | Tests derive from the spec or the oracle, never from reading the implementation | An agent asked to test a function will assert what it does, not what it should do |
| R5 | No unsafe outside the allow-list (ocelli-wasm/src/ring.rs, ocelli-core/src/cast.rs) | Keeps the audit surface to two files |
| R6 | Provenance trailer on every commit | Cheap now, a retrofit across sixty thousand lines is not, and a device pathway may require it |

> That a new test would actually fail if the code were wrong. Mutate one
> constant, re-run, confirm it goes red.

### `docs/hld/25-first-ten-files.md`, section 28

> In this order. The goal of the first two weeks is to diff one windowed 2D
> image against cornerstone3D - everything below serves that.

| **#** | **File** | **Why here** |
|----|----|----|
| 5 | crates/ocelli-codec/src/registry.rs | The decoder trait and registry |
| 6 | crates/ocelli-dicom/src/parse.rs | Parse and dispatch over dicom-rs |

### DICOM encoding rules applied by this plan

- PS3.10 section 7 requires the 128-byte preamble and four-byte `DICM` prefix
  for the Part 10 file shape used here. It requires File Meta Information to be
  Explicit VR Little Endian regardless of the following data set.
- PS3.10 section 7.1 makes Transfer Syntax UID `(0002,0010)` a required File
  Meta Information element and says it identifies the transfer syntax used for
  the enclosed data set.
- PS3.5 annex A defines Implicit VR Little Endian, Explicit VR Little Endian,
  Explicit VR Big Endian, Deflated Explicit VR Little Endian, and the
  encapsulated transfer syntaxes used by the corpus.
- PS3.5 section A.5 applies deflate to the entire Explicit VR Little Endian data
  set. This is data-set decoding and belongs in F-016, unlike compressed pixel
  frame decoding.
- PS3.5 sections 6.2 and 6.4 retain UI NUL padding rules and the multi-value
  backslash convention. The parsed dicom-rs object remains available rather
  than flattening text values into one host scalar.

## What the specification does not cover

1. The HLD names `parse.rs` but gives no public Rust signature or returned
   type.
2. The HLD does not say whether parsing accepts a bare data set without Part 10
   File Meta Information. This story is explicitly Part 10, so the entry point
   refuses a missing preamble or `DICM` prefix instead of guessing an encoding.
3. The HLD does not define how the selected transfer syntax remains observable
   after dicom-rs parses the object.
4. The HLD does not define a Rust error taxonomy for parser failures, and the
   worker protocol has no recoverable decode-error route yet.
5. The HLD's `dicom = 0.10` dependency line and its feature paragraph do not
   describe the same Cargo package surface. The umbrella crate does not expose
   `jpeg`, `rle`, `deflate`, or `openjp2`, and it enables default features on
   `dicom-transfer-syntax-registry` through an unconditional dependency.
6. The HLD does not say how corpus-backed Rust conformance checks run while CI
   intentionally has no corpus under D-04.

## Approach

1. Add `crates/ocelli-dicom/src/parse.rs` with
   `parse_part10(bytes: &[u8]) -> Result<ParsedDicom, ParseError>` and re-export
   its public types from `lib.rs`.
2. Require a complete 128-byte preamble followed by `DICM`. Position a
   `Cursor<&[u8]>` at the prefix and call dicom-rs
   `FileMetaTable::from_reader`. This parser is fixed to Explicit VR Little
   Endian, so the data set's declared transfer syntax can never affect File
   Meta Information parsing.
3. Resolve the trimmed Transfer Syntax UID through the built-in static
   `TransferSyntaxRegistry`. Refuse an unknown UID. Refuse a known descriptor
   whose `can_decode_dataset()` is false. Do not guess or retry with Explicit VR
   Little Endian.
4. Classify the selected route as `ImplicitVrLittleEndian`,
   `ExplicitVrLittleEndian`, `ExplicitVrBigEndian`,
   `DeflatedExplicitVrLittleEndian`, or `Encapsulated`. Adapt the remaining
   bytes once. The selected dicom-rs codec supplies ordinary data-set adapters.
   Deflate uses the same flate2 implementation directly because the erased
   adapter does not expose its consumed input count, which is required to
   validate PS3.5 A.5 stream termination and NULL padding. Run a strict
   structural preflight and then collect once under the same resolved syntax.
   The preflight refuses odd lengths, malformed nesting, mismatched top-level
   Pixel Data representation, invalid encapsulation, and top-level group 0002
   elements. Native Format Pixel Data nested in a Sequence Item remains valid
   under an encapsulated transfer syntax.
   Append a fixed group 0002 completion marker after preflight, then require
   and remove it after collection so partial top-level headers remain
   observable without a collision or private-element allocation. Do not retry
   under an alternate syntax.
5. Return `ParsedDicom`, which couples the dicom-rs file object with immutable
   `TransferSyntaxInfo`. The info exposes the canonical UID, registry name, and
   route. This is not a forwarding wrapper. It preserves evidence about which
   parser path produced the object.
6. Keep the complete dicom-rs object so DS and IS multiplicity, UI padding,
   sequences, and encapsulated fragments remain available to F-017 and F-023.
   F-016 adds no metadata projection and performs no pixel decode.
7. Map upstream errors into patient-safe `ParseError` variants. No variant
   stores input bytes, attribute values, SOP identifiers, file paths, or the
   upstream token that failed. An unknown Transfer Syntax UID may be retained
   because it is an encoding identifier, not instance data.
8. Add table-driven synthetic tests whose byte layouts are transcribed from
   PS3.10 section 7 and PS3.5 annex A. Assert both a known tag value and the
   observed route for implicit little endian, explicit little endian, explicit
   big endian, deflated explicit little endian, and an encapsulated syntax.
9. Add controlled negative tests for a short preamble, wrong prefix, malformed
   File Meta Information, missing Transfer Syntax UID, unknown UID, a known
   transfer syntax with no data-set reader, and a truncated data set. None may
   retry under another syntax.
10. Add a corpus integration test over the manifest's available files. The
    ordinary Rust suite exercises self-contained synthetic byte fixtures. The
    existing local-only `corpus` gate requires the external corpus and invokes
    the ignored integration test, so CI does not gain a hidden corpus
    dependency and a missing local corpus cannot read as a pass.
11. Prove dispatch sensitivity by mutating the resolved route in the
    implementation to Explicit VR Little Endian. The implicit and big-endian
    fixtures must fail for their declared dispatch reason, then pass after the
    mutation is reverted.
12. Build the crate for the native host and wasm32. No wasm-only parsing branch
    or inventory registry is introduced. `ocelli-dicom` leaves the repository's
    self-selected `no_std` set because the required dicom-rs graph uses `std`.
    D-18 records the posture and the guard's explicit set is updated with it.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none. Parsing is outside the render loop
- unsafe: none
- no_std: no. D-18 records the required dicom-rs `std` dependency graph
- Tier A (WebGPU): n/a. Parsing is renderer-independent
- Tier B (WebGL2): n/a. Parsing is renderer-independent
- Tier C (CPU): n/a. The same parsed object and dispatch evidence are used on every tier

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| fixture | Part 10 File Meta Information is Explicit VR Little Endian and `(0002,0010)` selects the following data-set encoding, per PS3.10 sections 7 and 7.1 | `crates/ocelli-dicom/tests/part10.rs` |
| fixture | Implicit VR Little Endian, Explicit VR Little Endian, Explicit VR Big Endian, and Deflated Explicit VR Little Endian produce the independently encoded value and distinct route required by PS3.5 annex A | `crates/ocelli-dicom/tests/part10.rs` |
| fixture | Encapsulated pixel data remains an item sequence and is not decoded in F-016, per PS3.5 sections A.4 and 8.2 | `crates/ocelli-dicom/tests/part10.rs` |
| unit | Short input, wrong prefix, malformed meta, missing or unknown UID, unsupported data-set decoding, and truncated data set are distinct refusals with no fallback | `crates/ocelli-dicom/tests/part10.rs` and private tests in `parse.rs` |
| unit | UI NUL padding and multi-valued DS or IS text remain distinguishable in the dicom-rs object, per PS3.5 sections 6.2 and 6.4 | `crates/ocelli-dicom/tests/part10.rs` |
| conformance | Every present corpus row parses under its manifest-declared transfer syntax without pixel decode, including synthetic and licensed rows | `crates/ocelli-dicom/tests/corpus.rs`, invoked by the local `corpus` gate |
| cross-target | Native and wasm32 compile the same parser contract with no inventory registry or `wasm-bindgen` import | `bin/ocelli.sh check ocelli-dicom` and `bin/ocelli.sh wasm` |
| mutation | Replacing declared dispatch with Explicit VR Little Endian makes implicit and big-endian fixtures fail for the selected-path assertion | recorded in the clean feature review |

## Parity surface covered

Appendix B contains a `Transfer syntaxes` row, but the tracked file has no
`Covered by` column and no E3.1 mapping. F-016 establishes parsing for the
corpus transfer syntaxes without claiming codec decode parity. No Appendix B
count changes.

## Deviations

Existing D-02 applies the `ocelli-dicom` crate name. D-18 uses the dicom-rs
component crates directly because the 0.10 umbrella does not expose the HLD's
named codec features and enables transfer-syntax-registry defaults. It also
records that the required dicom-rs graph makes `ocelli-dicom` a `std` crate.
No other new deviation is anticipated.

## LLD impact

- Create `docs/lld/dicom-ingest.md` for the Part 10 parser, dispatch evidence,
  error taxonomy, supported data-set routes, and corpus verification path.
- Update `docs/lld/README.md` to index `dicom-ingest.md` and list F-016.
- Update `docs/lld/build-targets.md` with F-016's dicom-rs feature selection and
  the native and wasm parser proof.
- Update `docs/lld/corpus.md` with the Rust parser conformance invocation owned
  by the local corpus gate.

## Write set

**Created**

- `crates/ocelli-dicom/src/parse.rs`
- `crates/ocelli-dicom/tests/part10.rs`
- `crates/ocelli-dicom/tests/corpus.rs`
- `docs/lld/dicom-ingest.md`

**Modified**

- `Cargo.toml`
- `Cargo.lock`
- `crates/ocelli-dicom/Cargo.toml`
- `crates/ocelli-dicom/src/lib.rs`
- `bin/ocelli.sh`
- `scripts/no_std_check.py`
- `.claude/plans/F-016-design.md`
- `docs/lld/README.md`
- `docs/lld/build-targets.md`
- `docs/lld/corpus.md`
- `docs/sprints/CURRENT_SPRINT.md`
- `docs/sprints/BACKLOG.md`
- `docs/sprints/SPRINT_TRACKER.md`
- `docs/sprints/AS_BUILT.md`
- `CHANGELOG.md`

`docs/hld/DEVIATIONS.md` records D-18 for the component-crate dependency shape.

## Open questions

None.

## Design-round decision

The operator approved direct dicom-rs 0.10 component dependencies with default
features disabled and only `deflate` enabled for F-016. The selected components
are `dicom-object`, `dicom-encoding`, `dicom-parser`, and
`dicom-transfer-syntax-registry`. `flate2` is also direct so Ocelli can validate
the consumed Deflate stream boundary and required padding, which the erased
dicom-rs adapter cannot report. D-18 records the difference from section 15.2's
umbrella dependency line. Pixel codec features remain for F-023 and its
dependent codec stories.
