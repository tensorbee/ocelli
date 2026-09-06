# F-X013, Price the HTJ2K decoder route after gate A1 failed

**Status**: approved
**Epic ref**: Y1.8
**Sprint**: S04
**Estimate**: 3w

## Normative source, transcribed

### `docs/hld/A-spike-gates.md`, Appendix A

```text
Each of these can end or reshape the programme, and each is cheap to answer. They belong in the first six weeks, with the authority to stop.

| **Gate** | **Question** | **Consequence if it fails** |
|----|----|----|
| A1 | Does HTJ2K decode correctly in openjp2 under wasm32, bit-exact against OpenJPH? | You maintain C codec builds regardless, and one of the four arguments for the rewrite weakens |
```

### `docs/hld/18-codec-registry.md`, section 21

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

```text
| **TWO OPEN GATES** HTJ2K through openjp2 is registered in dicom-rs but unverified under wasm32 — test it bit-exact against OpenJPH output in week one. JPEG-LS has no credible pure-Rust path; the registry design deliberately allows a JS-side bridge to @cornerstonejs/codec-charls as a registered decoder, so choosing that route costs an adapter rather than a redesign. |
```

### `docs/hld/11-decision-log.md`, decisions D2, D3, D7, and D14

```text
| D2 | wasm-bindgen in one crate only | Bindings wherever convenient | Phases 2 and 3 are entry points, not rewrites |
| D3 | Pixels never cross the boundary | Decode in wasm, render in JS | The main architectural gain |
| D7 | Validation oracle before port code | Validation as a tail phase | Makes generated Rust safe to merge at volume |
| D14 | Attestation claims measured divergence | Claim bit-exact reproducibility | Honest, publishable, and actually achievable |
```

### `docs/hld/22-testing-and-tolerance.md`, section 25

```text
| **Layer** | **What it proves** | **Where it comes from** |
|----|----|----|
| Conformance | Each transfer syntax decodes correctly | Published DICOM test corpora |
```

### `docs/spikes/A1-htj2k-openjp2.md`, recommendation and routes

```text
**Do not adopt `openjp2` for the browser target. Do not adopt it for the native
targets either without addressing the null-`dealloc` defect, which is undefined
behaviour on every target and not only on wasm.**

Three routes exist and this gate does not choose between them, because
**decision 6 of the design round says the fallback is priced in an S04 story
and not here.**
```

```text
| **F1** | Upstream fix to `openjp2`, a null check at `src/tcd.rs` 1432 and 2510, plus a `cfg` for the C allocator | Both changes are small and are in a BSD-2 crate. It is a third-party release cycle, and until it lands the wasm route also needs the `--allow-undefined` link argument and an allocator shim, which is a bigger commitment than a patch |
| **F2** | `@cornerstonejs/codec-openjph` 2.4.9, which is what the reference itself uses | It is a JS-side wasm module, so it has R1's shape and R1's hole: no JavaScript on the native desktop and server targets. `docs/SOURCE-POLICY.md` already permits OpenJPH |
| **F3** | `openjph-core` 0.1.0, pure Rust, BSD-2-Clause | Declares itself "a faithful port of the OpenJPH C++ library (v0.26.3)". OpenJPH is in the policy's yes column. First and only publish 2026-03-20, 3643 downloads, and **crates.io reports no repository URL for it**, so `docs/SOURCE-POLICY.md` question 1 cannot be answered from a repository and only question 2 can. Not measured by this gate |
```

```text
**F3 is the one to measure first in S04**, because it is the only route that is
one implementation on every target, which is the property that decided A2.
```

### `docs/SOURCE-POLICY.md`, the rule for a new source

```text
Before taking anything from a source not in the table above, check three
things and record the answer:

1. Is there a LICENSE, LICENCE, COPYING or NOTICE file at the repository root?
2. Does the hosting platform's metadata report a licence?
3. Does the licence permit the specific use, which for source is usually
   **derivative works**, not merely use?

If 1 and 2 are both absent, the answer is no.
```

## What the specification does not cover

The HLD names `openjp2` and says what happens if A1 fails. It does not select a
replacement, define how a young port is accepted, or price the cost of a split
browser and native decoder.

This plan uses five axes for the price: correctness, target coverage, source
provenance and audit surface, binary and toolchain cost, and maintenance risk.
It recommends a route only when all three HTJ2K transfer syntaxes have an
answer. A lossless route must reproduce the uncompressed synthetic reference
exactly. The irreversible `.203` route is compared against the OpenJPH CLI and
reported as a D14 measured divergence, since it has no independent lossless
anchor.

## Approach

1. Resolve the source-policy question for `openjph-core` 0.1.0 before reading
   its implementation. Inspect the packaged licence material, crates.io
   metadata, publisher identity, and stated OpenJPH provenance. Record an
   extension row in `docs/SOURCE-POLICY.md` if dependence is approved.
2. Measure F3 first in a throwaway spike crate. Build it for the pinned native
   target and `wasm32-unknown-unknown`, with and without SIMD128 where the crate
   permits that distinction. Decode `.201`, `.202`, and `.203` codestreams
   extracted by the existing common spike tooling.
3. Canonicalise decoded samples with the existing
   `tools/spikes/common/compare.mjs` contract. Compare `.201` and `.202`
   exactly against `syntax/reference_mono12.dcm`. Compare all three against
   `ojph_expand`, while stating that F3 is itself an OpenJPH port and therefore
   is not an independent algorithmic implementation.
4. Record wasm release size, native dependency and toolchain surface,
   allocations required by one decode, any `unsafe` inside the dependency,
   API completeness for caller-owned output, publication history, repository
   and issue-tracker availability, and the cost of maintaining a fork.
5. If F3 passes correctness and both targets, recommend it with its risks. If
   it fails, price F1 as the upstream patch plus release-cycle dependency, then
   F2 as a browser-only adapter plus a distinct native route. Do not implement
   any production decoder in this story.
6. Write `docs/spikes/A1-htj2k-route.md` with the evidence matrix, raw results,
   recommendation, rejected routes, and the exact follow-up owed by E2.6.
   Update the existing A1 answer and LLD only with pointers and the resolved
   replacement route.

### Decision matrix

| Outcome | Decision |
|---------|----------|
| F3 passes all three syntaxes on wasm and native | Recommend the pinned F3 version as one decoder route, subject to source-policy approval and a production E2.6 design |
| F3 fails but F1 has a bounded patch and credible release route | Recommend F1 only with the upstream fix landed and released. Do not ship the local allocator shim |
| Only F2 works in the browser | Record a split architecture and stop for operator approval, because native desktop and server still need another decoder |
| No route meets correctness on both targets | HTJ2K remains unavailable through the stable `Unavailable` contract |

### Anticipated implementation write set

- `docs/SOURCE-POLICY.md`, only if the operator approves `openjph-core`
- `tools/spikes/x013-htj2k-route/Cargo.toml`
- `tools/spikes/x013-htj2k-route/Cargo.lock`
- `tools/spikes/x013-htj2k-route/src/lib.rs`
- `tools/spikes/x013-htj2k-route/src/main.rs`
- `tools/spikes/x013-htj2k-route/run.mjs`
- `docs/spikes/A1-htj2k-route.md`
- `docs/spikes/A1-htj2k-openjp2.md`
- `docs/lld/oracle.md`
- `docs/lld/corpus.md`

The existing `tools/spikes/common/` comparator and extraction code are reused,
not forked. No production crate or workspace manifest changes.

## Boundary and tier

- wasm-bindgen: not touched by repository source. A candidate's transitive dependencies are recorded
- Pixels across the boundary: no production boundary change. F2's required codec-byte copy is priced as a rejected or conditional architecture cost
- Render-loop allocation: none. Decode allocation is measured separately and production acceptance remains section 21's caller-owned output contract
- unsafe: none in repository code. Dependency `unsafe` is measured as audit surface
- Tier A (WebGPU): n/a
- Tier B (WebGL2): n/a
- Tier C (CPU): n/a. Decode is CPU work on every rendering tier, and unavailable is identical on all three

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| `conformance` | F3 decodes `.201` and `.202` exactly to the synthetic uncompressed reference, and produces a measured `.203` result against OpenJPH | `tools/spikes/x013-htj2k-route/run.mjs` |
| `unit` | The shared comparator still observes one changed sample red before candidate measurements are trusted | existing `tools/spikes/common/tests/compare_test.mjs` |
| `build` | The exact candidate builds for native and `wasm32-unknown-unknown`, including the declared SIMD configurations | spike commands recorded in `docs/spikes/A1-htj2k-route.md` |
| `fixture` | No new pixel formula is implemented. The existing 64 by 96 unsigned 16-bit ramp is the hand-computable anchor for the two reversible syntaxes, citing PS3.3 C.7.6.3.1.1 and C.7.6.3.1.2 | existing corpus generator and `scripts/tests/test_corpus_synth.py` |

## Parity surface covered

`docs/hld/B-parity-surface.md` contains this adjacent row:

| Transfer syntaxes | ~13 | Two of them, JPEG-LS and HTJ2K, are the open gates in Appendix A |

This story prices and selects the HTJ2K route. It does not register a decoder,
so the parity count does not change. Appendix B has no `Covered by` column in
this repository's copy and no row keyed by Y1.8.

## Deviations

None in this story. The production codec story that activates a route must
raise a deviation from section 15.2's `openjp2` selection if it chooses F2 or
F3. This evidence-only story does not activate a codec feature.

## LLD impact

`docs/lld/oracle.md` will point to the selected route while continuing to state
that the reference currently has no usable HTJ2K decoder. `docs/lld/corpus.md`
will record which comparisons are independent and which share OpenJPH lineage.

## Open questions

None. The operator approved `openjph-core` 0.1.0 for the spike after packaged
provenance is checked. Production registration, standing codec tests, and any
section 15.2 deviation remain with E2.6.
