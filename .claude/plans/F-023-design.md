# F-023, Codec dispatch layer and capability registry

**Status**: approved
**Epic ref**: E4.1
**Sprint**: S07
**Estimate**: 2w

## Normative source, transcribed

_The quotations below normalise the source's em dash to a hyphen and its prose
semicolon to a full stop because `scripts/prose_check.py` covers this plan. No
technical text is changed. The tracked HLD wins where exact bytes matter._

### `docs/hld/03-architecture-and-crates.md`, section 4

| **Crate** | **Responsibility** | **wasm** | **native** |
|----|----|----|----|
| ocelli-codec | Decoder registry and codec adapters, registered at runtime | yes | yes |
| ocelli-wasm | The only crate that may import wasm-bindgen. Boundary, commands, event ring. | yes | no |

### `docs/hld/10-extension-points.md`, section 13

> - **A dynamic codec registry**, so a native build can link C codecs the
> browser build cannot.

### `docs/hld/12-workspace-and-build.md`, sections 15.1 and 15.2

> ocelli-codec/ \# decoder registry + adapters

> **DICOM-RS FEATURES** Disable default features. The defaults are rayon and
> simd, and the gdcm feature is native-only. On wasm you want:
> default-features = false, then jpeg, rle, deflate and openjp2 selected
> explicitly. Note also that the inventory-based transfer-syntax plugin
> registry does not work on wasm at all - which is why section 21 specifies an
> explicit runtime registry instead.

### `docs/hld/18-codec-registry.md`, section 21, in full

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

> **TWO OPEN GATES** HTJ2K through openjp2 is registered in dicom-rs but
> unverified under wasm32 - test it bit-exact against OpenJPH output in week
> one. JPEG-LS has no credible pure-Rust path. The registry design deliberately
> allows a JS-side bridge to @cornerstonejs/codec-charls as a registered
> decoder, so choosing that route costs an adapter rather than a redesign.

The two gates are now resolved. `docs/spikes/GATES.md` records A1 as `Fail`
for openjp2 under wasm32 and A2 as `Pure Rust` through `pure_jpegls` 2.0.0.
`docs/spikes/A1-htj2k-route.md` recommends `openjph-core` 0.1.0 subject to its
listed production gates. F-023 creates no concrete codec adapter and does not
activate any of those dependencies.

### `docs/hld/21-worker-protocol.md`, section 24

> Three roles: the main thread, N decode workers each with its own WebAssembly
> instance, and one render worker owning the GPUDevice and every
> OffscreenCanvas. Decode workers never touch the GPU. The render worker never
> decodes.

### `docs/hld/22-testing-and-tolerance.md`, section 25

| **Layer** | **What it proves** | **Where it comes from** |
|----|----|----|
| Conformance | Each transfer syntax decodes correctly | Published DICOM test corpora |

### `docs/hld/23-performance-rules.md`, section 26

> - No allocation in the render loop. Pre-size every buffer at viewport
> creation.
>
> - **Measure with the benchmark harness before optimising anything.** The
> intuitions that work in JavaScript do not transfer.

### `docs/hld/24-agent-code-standards.md`, sections 27.2 and 27.3

| **#** | **Rule** | **Why** |
|----|----|----|
| R2 | Tests derive from the spec or the oracle, never from reading the implementation | An agent asked to test a function will assert what it does, not what it should do |
| R3 | Every function doing pixel arithmetic needs a fixture test with hand-computed values, citing the DICOM section | This is the defect class that reaches patients |
| R5 | No unsafe outside the allow-list (ocelli-wasm/src/ring.rs, ocelli-core/src/cast.rs) | Keeps the audit surface to two files |

> - Every as cast and every rounding decision.
>
> - That a new test would actually fail if the code were wrong. Mutate one
>   constant, re-run, confirm it goes red.

### `docs/hld/25-first-ten-files.md`, section 28

| **#** | **File** | **Why here** |
|----|----|----|
| 5 | crates/ocelli-codec/src/registry.rs | The decoder trait and registry |

## What the specification does not cover

The HLD fixes the `Decoder` extension point, the explicit runtime map, shared
ownership, and caller-provided output. It does not define `FrameDesc`, error
variants, capability vocabulary, unknown-versus-known-unavailable semantics,
duplicate registration behavior, atomicity when one decoder claims several
UIDs, or buffer-size validation. The active sprint supplies the missing
requirements: unknown syntax and known syntax without a decoder are distinct,
unavailable capability is observable, and registration order must not silently
change which decoder runs.

This plan keeps the HLD's `Arc<dyn Decoder>` and its trait even before a
production adapter lands. `Decoder` is an explicit repository exception to the
one-implementer structural rule. The registry starts with an explicit set of
known Transfer Syntax UIDs. Registration is atomic: it validates every UID a
decoder declares before inserting any. A duplicate registered UID is refused
rather than replaced. A decoder may claim only a UID already declared known.

The HLD's shown `register` signature silently replaces an existing entry and
cannot report invalid declarations. The sprint acceptance explicitly forbids
that behavior. D-19 records the necessary result-returning, collision-refusing
registration contract without changing the prescribed map or decoder trait.

`tools/bench/subjects.json` assigns `decode.frame` to F-023, but its definition
times one real `Decoder::decode` over one corpus frame. No concrete decoder is
in F-023 scope. Recording a dispatch-only or test-decoder number as decode
performance would be a fabricated measurement. F-023 leaves the subject
`unavailable` with reason `no_runner`. The first concrete decoder story can
add the runner without redefining the subject.

## Approach

1. Create `registry.rs` and transcribe the `Decoder` trait and `Registry`
   storage shape. Use `std::collections::HashMap` and `std::sync::Arc` as the
   HLD specifies. Remove the scaffold's self-selected `no_std` declaration
   because the prescribed implementation requires `std`.
2. Define `FrameDesc` as validated descriptive input only: rows, columns,
   samples per pixel, bits allocated, bits stored, high bit, pixel
   representation, and photometric interpretation. Construction validates
   nonzero dimensions, supported container widths, stored-bit bounds, high-bit
   placement, and checked output length. It performs no pixel arithmetic.
3. Define `Capability` with `Available`, `KnownUnavailable`, and `Unknown`.
   `Registry::with_known` receives the build's explicit UID catalogue.
   Capability and lookup never infer a common transfer syntax.
4. Make `register` return `Result<(), RegistryError>`. Preflight a decoder's
   complete static UID list for empty strings, repetitions, unknown UIDs, and
   existing registrations. Insert only after the whole declaration passes.
5. Dispatch by exact UID. Unknown and known-unavailable return distinct error
   variants before invoking a decoder. Available dispatch calls `decode`
   directly with the caller's `out` slice and performs no registry allocation.
6. Use test decoders only to prove registry behavior and the trait's output
   contract. Concrete native, JPEG, RLE, JPEG-LS, JPEG 2000, and HTJ2K adapters
   remain in F-024 onward.
7. Mutate duplicate handling to last-registration-wins and mutate exact UID
   lookup to a common-syntax fallback. Each named registry test must fail, then
   the mutation is reverted.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none. Registration allocates during setup. Lookup
  and decode dispatch allocate nothing
- unsafe: none
- Tier A (WebGPU): n/a. Decode workers never touch the GPU
- Tier B (WebGL2): n/a. The same registry is used
- Tier C (CPU): n/a. The same registry is used

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| unit | Exact UID lookup distinguishes available, known unavailable, and unknown | `crates/ocelli-codec/src/registry.rs` |
| unit | Multi-UID registration is atomic and refuses empty, repeated, unknown, or already registered UIDs without replacing a decoder | `crates/ocelli-codec/src/registry.rs` |
| unit | Dispatch passes the exact source, descriptor, and caller buffer to the selected decoder and propagates its error without allocating an output vector | `crates/ocelli-codec/tests/registry.rs` |
| property | Registration order cannot change a successful mapping because every collision is refused before mutation | `crates/ocelli-codec/tests/registry.rs` |
| cross-target | Native and wasm32 compile the same explicit registry with no inventory registration or `wasm-bindgen` | `bin/ocelli.sh check ocelli-codec` and `bin/ocelli.sh wasm` |
| mutation | Last-registration-wins and common-syntax fallback mutations make their named tests fail | recorded in the clean feature review |

No concrete codec and no pixel arithmetic are introduced, so conformance
corpus output fixtures and HLD 27.2 R3 pixel fixtures begin with F-024.

## Parity surface covered

Appendix B's `Transfer syntaxes` row records about thirteen and names JPEG-LS
and HTJ2K as its open gates. F-023 supplies explicit dispatch and capability
states for that surface. It does not claim any syntax decodes until a concrete
adapter and its conformance evidence land.

## Deviations

- Add D-19. Section 21 shows `register(&mut self, d: Arc<dyn Decoder>)` and
  overwrites existing mappings with `HashMap::insert`. F-023 changes it to a
  result-returning operation that validates the decoder's complete UID set and
  refuses collisions atomically. The active sprint requires registration order
  not to silently change which decoder runs.

No other deviation is planned. The crate's old `no_std` attribute is a
self-selected scaffold posture, while the normative section 21 implementation
uses `HashMap` and `Arc` from `std`.

## LLD impact

- Create `docs/lld/codecs.md` for decoder ownership, frame description,
  capability states, atomic registration, dispatch errors, allocation
  boundaries, and resolved spike routes reserved for later adapters.
- Update `docs/lld/README.md` to index `codecs.md` and list F-023.
- Update `docs/lld/benchmarks.md` only to replace F-023's pending-story reason
  for `decode.frame` with the honest `no_runner` state. Do not record a number.
- Update `docs/lld/build-targets.md` with the prescribed `std` registry shape.

## Write set

**Created**

- `crates/ocelli-codec/src/registry.rs`
- `crates/ocelli-codec/tests/registry.rs`
- `docs/lld/codecs.md`

**Modified**

- `crates/ocelli-codec/src/lib.rs`
- `docs/hld/DEVIATIONS.md`
- `docs/lld/README.md`
- `docs/lld/benchmarks.md`
- `docs/lld/build-targets.md`
- `.claude/plans/F-023-design.md`
- `docs/sprints/CURRENT_SPRINT.md`
- `docs/sprints/BACKLOG.md`
- `docs/sprints/SPRINT_TRACKER.md`
- `docs/sprints/AS_BUILT.md`
- `CHANGELOG.md`

F-023 shares only LLD and sprint ledger files with the other S07 stories.
`docs/hld/DEVIATIONS.md` is exclusive to F-023 in this sprint. Sprint ledger
files remain integrator-owned for a parallel prepared feature.

## Open questions

None.
