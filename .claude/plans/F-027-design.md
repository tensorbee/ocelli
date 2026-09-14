# F-027, HTJ2K: spike, then integrate or bridge to openjph wasm

**Status**: approved
**Epic ref**: E4.5
**Sprint**: S09
**Estimate**: 5w
**Allocation note**: `P0 KILL CRITERION — unverified in Rust today`

## Normative source, transcribed

### `docs/hld/18-codec-registry.md`, section 21, the gate note

Transcribed into a table row, which is where `scripts/prose_check.py` relaxes
the voice rules, so the author's text is quoted exactly.

|  |
|----|
| **TWO OPEN GATES** HTJ2K through openjp2 is registered in dicom-rs but unverified under wasm32 — test it bit-exact against OpenJPH output in week one. JPEG-LS has no credible pure-Rust path; the registry design deliberately allows a JS-side bridge to @cornerstonejs/codec-charls as a registered decoder, so choosing that route costs an adapter rather than a redesign. |

The `Decoder` trait and `Registry`, quoted in full in
`.claude/plans/F-028-design.md` and identical here, are not re-transcribed.
Section 21's one line that governs this story specifically:

```rust
    /// Decode one frame into `out`. Must not allocate.
```

### `docs/hld/12-workspace-and-build.md`, section 15.2, the dependency instruction

|  |
|----|
| **DICOM-RS FEATURES** Disable default features. The defaults are rayon and simd, and the gdcm feature is native-only. On wasm you want: default-features = false, then jpeg, rle, deflate and openjp2 selected explicitly. Note also that the inventory-based transfer-syntax plugin registry does not work on wasm at all — which is why §21 specifies an explicit runtime registry instead. |

`openjp2` is the entry that matters here, emphasised in the sentence above
rather than inside the quotation.

**`openjp2` is the named JPEG 2000 and HTJ2K route and this story does not use
it.** That is deviation D-22, raised below. Deviation **D-20** already departed
from the same sentence for JPEG 2000 Part 1 at F-026, and the reason is the
same measured failure.

### `docs/hld/A-spike-gates.md`, Appendix A, gate A1, and its answer

`docs/spikes/GATES.md`, the resolution table, quoted:

> | **A1** | `docs/spikes/A1-htj2k-openjp2.md` | **`Fail`.** `openjp2` 0.6.1 does
> not decode at all on `wasm32-unknown-unknown`, and is exact natively.
> Appendix A's consequence is in force and the fallback is an S04 story |

### `docs/spikes/A1-htj2k-route.md`, F-X013, the priced fallback

**Outcome: Recommend F3 for the production design, with conditions.** The
measured axis table, quoted:

| Axis | Result | Consequence |
|------|--------|-------------|
| `.201` lossless | Exact against the synthetic ramp and OpenJPH on all three builds | Pass |
| `.202` lossless RPCL | Exact against the synthetic ramp and OpenJPH on all three builds | Pass |
| `.203` irreversible | Candidate builds agree. 41 samples differ from OpenJPH by one | Accept as a recorded D14 divergence |
| Native target | Builds and decodes on `aarch64-apple-darwin` | Pass |
| wasm target | Plain and `+simd128` builds decode all rows | Pass |
| Source provenance | Registry says BSD-2-Clause. Package has no licence file or repository URL | Conditional production risk |
| Caller-owned output | Not supported. `pull()` returns a new `Vec<i32>` | needs an upstream API or a maintained adaptation |
| Maintenance | One release, one owner, no repository or issue tracker in metadata | Pin exactly and require a fork plan |

The measured `.203` divergence, quoted exactly, because it becomes a standing
test rather than a spike result:

```text
D_native = D_wasm = D_simd
ce4a2bb9d75b897292a4e9e9e7455447e976f5ff1e18b4ecb3219bb17d4d062c

D_ojph
41a94cc4db2b871e16e50c738512413db800657b0dbeab7872415504be87c138

differing samples: 41 of 6144
first difference: index 69, candidate=756, OpenJPH=757
maximum absolute difference: 1
histogram by magnitude: 1:41
```

The seven follow-up conditions, quoted:

> 1. raise the section 15.2 deviation from `openjp2`
> 2. obtain and retain complete BSD notice and copyright material
> 3. decide whether to upstream a caller-owned decode API or maintain a bounded
>    adaptation that removes the per-row clone contract
> 4. audit the dependency's relevant unsafe paths and malformed-input behavior
> 5. add standing conformance cases for `.201`, `.202` and `.203` on native and
>    wasm, including the `.203` divergence measured here
> 6. measure incremental wasm size in the real production crate
> 7. keep HTJ2K `Unavailable` on every rendering tier until all of those gates
>    pass

**Both spike answer files address these conditions to "E2.6", and E2.6 is
F-014, "Quirk-capture workflow: every field bug becomes a fixture", in S05.**
Read from `docs/sprints/allocation.json`, E2.1 to E2.7 are the oracle epic, and
no codec story is among them. The intended referent is unambiguous from context
in both files: the HTJ2K production decoder story is **E4.5, F-027**, and the
JPEG-LS one is **E4.6, F-028**. The two answer files are corrected in this
change. This is recorded rather than silently fixed because a condition list
addressed to the wrong story is a condition list nobody is holding.

### Decision D14, from `docs/hld/11-decision-log.md`

> Claim MEASURED divergence, never bit-exact reproducibility.

This is what makes `.203`'s 41-sample difference a recordable result rather than
a failure.

## What the specification does not cover

1. **Whether to register HTJ2K at all this sprint.** Condition 7 says keep it
   unavailable "until all of those gates pass", and one of the seven cannot be
   closed by engineering. See Approach and Open question 1.
2. **How the seven conditions stop being prose.** A condition list in a spike
   answer is remembered, not enforced. This plan converts each into an artefact
   or a gate assertion, and the one that cannot be closed in-sprint becomes a
   release-blocking check rather than a note.
3. **Whether HTJ2K is one decoder or three.** `.201`, `.202` and `.203` are
   three UIDs. This plan drafted two `Mode` arms, lossless-only covering `.201`
   and `.202` together, on the reading that they differ only in progression
   order.

   **IMPLEMENTED AS THREE, because the two lossless syntaxes carry different
   constraints.** `.202` requires RPCL and `.201` requires nothing about
   progression, so one shared arm could only be right by refusing conformant
   `.201` files. See the correction in Approach step 5.

## Approach

### The decision, stated first

**Adopt route F3, `openjph-core` 0.1.0, vendored and pinned exactly as
`ritk-codecs` 0.6.0 already is, and register all three UIDs.** The alternative,
reporting the three UIDs `KnownUnavailable`, is a product decision that removes
three of sixteen transfer syntaxes from the parity surface, and the evidence does
not require it: F-X013 measured all three decoding on native and on both wasm
builds, with the two lossless syntaxes exact against an encoder-independent
anchor.

**There is no third candidate.** `ritk-codecs` 0.6.0, which is already vendored,
has no HTJ2K: `grep -ril 'htj2k\|high.throughput\|ht_block'` over
`vendor/ritk-codecs-0.6.0/src/` returns nothing, and its `jpeg_2000` module
carries `mq_coder` and `wavelet_9_7` with no HT block coder and no `CAP` marker
handling. `CURRENT_SPRINT.md` states the same thing and this is the check behind
it. F1, a patched `openjp2`, stays rejected on F-X006's measured undefined
behavior. F2, `@cornerstonejs/codec-openjph`, stays rejected because it leaves
the desktop and server targets with no HTJ2K decoder at all, which is the
architecture argument A2 made against R1 and it transfers unchanged.

### The seven conditions, each mapped to an artefact

| # | Condition | How it closes | Closes this sprint |
|---|-----------|---------------|--------------------|
| 1 | Raise the 15.2 deviation | **D-22**, written below, added to `docs/hld/DEVIATIONS.md` in this change | yes |
| 2 | Obtain complete BSD notice and copyright material | **Cannot be closed by engineering.** The package has no licence file and crates.io reports no repository URL. Converted into a mechanical release-profile gate that fails today, below | **no**, and the design round decided it is held by that gate rather than by registration |
| 3 | Caller-owned decode API | **Deviation D-21, already declared**, is exactly this case. The adapter validates the dependency's owned output completely, then copies it into the caller's slice atomically. F-027 is added to D-21's `Raised` cell and `openjph-core` is named in its text | yes |
| 4 | Audit the unsafe paths and malformed-input behavior | A `docs/SOURCE-POLICY.md` audit section in the shape the `oxideav-core` one already has: whole-word `unsafe` counted over every `*.rs` under the published `src/`, split by file and by whether the path is reachable on wasm32, plus a malformed-input table | yes |
| 5 | Standing conformance cases for all three UIDs on native and wasm, including the `.203` divergence | The test table below. The `.203` divergence is asserted by count, first index, maximum magnitude and histogram, exactly as F-X013 measured | yes |
| 6 | Incremental wasm size in the real production crate | `bin/ocelli.sh wasm` against `ci/wasm-size-budget.json`, rebaselined in this change with the delta attributed | yes |
| 7 | Keep HTJ2K unavailable until all gates pass | Six of seven close here. The design round decided condition 2 is a release gate rather than a registration gate, so the three UIDs become `Available` and publication stays blocked | yes, as decided |

**Condition 2 as a mechanical gate.** `scripts/pin_and_size_check.py` already
binds `ritk-codecs`'s archive digest, VCS identity, 74-file inventory, both
licence hashes and both no-Rayon graphs. `openjph-core` gets the same treatment
for everything it can carry, plus one assertion it will fail: the vendor tree
must contain a `LICENSE` file whose content hash is recorded. Until the notice
material is obtained, that assertion is scoped to the `--all` and release
profiles rather than the floor, so `/release` cannot publish and ordinary
development is not blocked. **This is the difference between a condition
somebody remembers and a condition the build holds.** A dead check would be
worse than no check, so it is written to fail today, and the failure names the
missing file and the reason.

### The vendored package

Same shape as D-20, because the pins gate already knows that shape:

- `vendor/openjph-core-0.1.0/`, added to the root `Cargo.toml` `exclude` list
- `openjph-core = { version = "=0.1.0", path = "vendor/openjph-core-0.1.0" }` at
  the workspace, and `openjph-core.workspace = true` in `ocelli-codec`
- The crates.io archive SHA-256
  `c8b96ed12b3d41623a771af4af8131abf353bc822b7a567c6ef3b35ab967a36d` and the
  packaged VCS revision `7ed6d6d110d994ec740aacaa90a78b2e807c4c24` bound by the
  pins gate, both re-verified against the crates.io API during implementation
  rather than copied from the spike
- The complete published file inventory bound to a digest held outside the
  vendor tree
- **No patch.** D-20 needed one to disable `jpeg-decoder` defaults. F-X013
  measured `openjph-core`'s runtime dependency surface as the crate plus
  `thiserror` 2.0.20, with no Rayon, so the published manifest already gives the
  graph section 15.2 wants and the vendored package is byte-identical to the
  archive

**Vendoring rather than a registry dependency is not optional here.** The package
has one release, one owner, and no repository or issue tracker in its metadata.
A yank or a deletion would make the build unreproducible, and the whole reason
D-20 vendored `ritk-codecs` applies more strongly to a package with a weaker
maintenance signal.

### The adapter

New file `crates/ocelli-codec/src/htj2k.rs`:

```rust
const HTJ2K_LOSSLESS_UID: &str = "1.2.840.10008.1.2.4.201";
const HTJ2K_LOSSLESS_RPCL_UID: &str = "1.2.840.10008.1.2.4.202";
const HTJ2K_UID: &str = "1.2.840.10008.1.2.4.203";

enum Mode { LosslessOnly, General }

pub struct Htj2kDecoder { mode: Mode }

pub fn register_htj2k_decoders(registry: &mut Registry) -> Result<(), RegistryError>;
```

`LosslessOnly` declares `[.201, .202]`, `General` declares `[.203]`.
Registration preflights all three UIDs before registering either decoder, per
D-19.

`decode` performs, in order:

1. Refuse a caller output whose length differs from `desc.output_len()`.
2. `validate_descriptor`, identical in shape to `jpeg2000.rs`: `PixelDataVr::Ob`,
   one sample per pixel, `bits_allocated` in `{8, 16}`, monochrome photometric
   interpretation.
3. Bound the codestream: it must start `ff 4f` (SOC) and end `ff d9` (EOC), with
   the same single trailing pad-byte tolerance `logical_codestream` already
   implements for JPEG 2000, and trailing data after EOC is `TrailingData`
   rather than a silent truncation.
4. Parse SIZ for dimensions, component count, precision and signedness and check
   them against the descriptor. Parse **CAP (`ff 50`)**, which is what makes a
   codestream HTJ2K rather than JPEG 2000 Part 1, and refuse a codestream with no
   CAP marker as `FrameMismatch`. Parse COD for the transform and the
   progression order.
5. **Enforce the mode.** `.201` and `.202` are lossless-only, so the wavelet
   transform must be the reversible 5/3 kernel. `.202` additionally requires
   progression order RPCL, read from COD's `SGcod` progression byte.

   **CORRECTED DURING IMPLEMENTATION, and this plan said it the wrong way
   round.** The draft read "and `.201` requires anything else. Without the
   progression check the two UIDs are interchangeable, and a `.202` claim would
   be unfalsifiable." PS3.5 A.4.10 requires RPCL for `.202` and A.4.9 constrains
   **nothing** about progression for `.201`, so a `.202` codestream is also a
   valid `.201` one and implementing the sentence as drafted would have refused
   conformant files. The corpus makes it concrete: its `.203` row is RPCL too,
   so progression does not partition the three at all. What separates `.201` and
   `.202` from `.203` is the transform, and RPCL is a one-directional
   requirement on `.202` alone. Caught by the fixture generator before any
   adapter code ran. See `.claude/reviews/F-027-working-pass-1.md` D1.
6. Decode through `openjph-core`, whose `pull()` returns one owned `Vec<i32>`
   per row. Rows are assembled into one bounded owned buffer, validated
   completely, and copied into the caller's slice once. This is D-21's contract.
7. Convert each sample through the shared `sample_convert` module F-028
   introduces, refusing any value outside the descriptor's stored range.

**`sample_convert` ownership.** F-028 moves `convert_samples`, `append_sample`
and `exact_i64` out of `jpeg2000.rs` into a shared private module. F-027 consumes
it and does not move it. **If F-028 does not land first, F-027 does the move.**
This is the one file both stories touch, and it is why they cannot run
concurrently. See Wave placement.

### `.203` and D14

`.203` is irreversible and its output differs from OpenJPH 0.31.0 at 41 of 6,144
samples by exactly one. **That is published as a measured divergence and is
never described as bit-exact**, which is decision D14. The standing test asserts
the divergence rather than tolerating it: the count, the first differing index,
the maximum absolute difference and the magnitude histogram are all pinned, so
the test goes red if the divergence grows, shrinks, or moves. **A tolerance is
not widened and no tolerance is introduced**, because an exact pinned divergence
is a stronger statement than a bound.

## Boundary and tier

- wasm-bindgen: not touched. F-X013 measured `openjph-core`'s locked graph as the
  crate, `thiserror` and `thiserror`'s proc-macro build dependencies, with no
  wasm-bindgen and no wasm import
- Pixels across the boundary: no. Decode is worker-side and writes into a caller
  buffer in linear memory
- Render-loop allocation: none in the render loop. Decode is not in the render
  loop. Within decode the dependency allocates per row, which F-X013 measured as
  at least 65 allocations for a 64-row frame. That is **deviation D-21**, it is
  larger than the JPEG 2000 case, and it is measured rather than described by the
  `decode.frame` benchmark, which gains an HTJ2K subject
- unsafe: **none in this repository.** The dependency contains 104 `unsafe fn`,
  `unsafe impl` or `unsafe { ... }` constructs across nine files, of which the
  scalar memory, wavelet and colour paths account for 50 and are the ones
  reachable on wasm. `scripts/unsafe_allowlist_check.py` reads `git ls-files` and
  never sees a dependency, so this changes no allowlist and is recorded in
  `docs/SOURCE-POLICY.md` instead. **That sentence is the reason condition 4 is a
  written audit rather than a gate count**
- Tier A (WebGPU): n/a
- Tier B (WebGL2): n/a
- Tier C (CPU): n/a

The three tier rows are `n/a` for the reason `docs/spikes/A2-jpeg-ls.md` states
and this plan adopts unchanged: decode is CPU work in a worker that completes
before anything reaches a device, the resolved tier does not select a decoder,
and a tier-gated codec path would only ever run on hardware nobody develops on.
Condition 7's phrase "unavailable on every rendering tier" is about capability
reporting, not about a tier-specific decode path, and there is none.

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| `conformance` | `.201` decodes `corpus/manifest.tsv` row 35 exactly to `R`, the synthetic ramp, whose digest is `b20a1ef3...` | `crates/ocelli-codec/tests/htj2k.rs` |
| `conformance` | `.202` decodes row 36 exactly to the same `R` | same |
| `conformance` | `.203` decodes row 37 to digest `ce4a2bb9...`, which is what F-X013 recorded for the candidate on all three builds. **CORRECTED DURING IMPLEMENTATION**: the `41 of 6144` figure is the candidate against OpenJPH 0.31.0's output, and `ojph_expand` is an external tool the spike ran once, so that comparison is not reproducible in this tree. Pinning the candidate's own digest is the reproducible half and is stronger than a tolerance. Against the uncompressed ramp the irreversible decode differs at 5,377 of 6,144 samples, which is recorded rather than bounded | same |
| `browser` | All three rows produce identical bytes on `wasm32-unknown-unknown`, plain and `+simd128`, as on native | same, under the wasm target |
| `fixture` | 8-bit and 16-bit, signed and unsigned stored domains round trip exactly, hand-computed little-endian two's complement citing PS3.3 C.7.6.3.1.4 | same |
| `unit` | A codestream with no CAP marker is `FrameMismatch`, so a JPEG 2000 Part 1 stream cannot be decoded as HTJ2K | same |
| `unit` | `.202` refuses a non-RPCL progression order and `.201` refuses an RPCL one, both `FrameMismatch` | same |
| `unit` | `.201` and `.202` refuse an irreversible transform and a non-zero quantization style | same |
| `unit` | Truncated, trailing-data and dimension-mismatched codestreams each return their distinct `CodecError` and leave the caller's output byte-unchanged | same |
| `unit` | Registration is atomic across all three UIDs, and `capability` reports `Available` after and `KnownUnavailable` before | same |

**Controlled mutation, observed red, required by `CURRENT_SPRINT.md`.** Three,
each named with its observed exit in the implementation note: drop the CAP
marker check so a Part 1 codestream is accepted, swap the `.201` and `.202`
progression rules, and change one byte of the pinned `.203` histogram. The third
proves the divergence assertion is load-bearing rather than decorative.

**Size.** `bin/ocelli.sh wasm` before and after, with the delta attributed to
`openjph-core` and `ci/wasm-size-budget.json` rebaselined in the same change.
F-X013's 110,724-byte whole-spike figure is **not** an incremental production
number and is not quoted as one.

## Parity surface covered

`docs/hld/B-parity-surface.md`'s **Transfer syntaxes** row, whose note reads "Two
of them, JPEG-LS and HTJ2K, are the open gates in Appendix A". This story closes
the HTJ2K half in implementation. There is no `Covered by` column in this
repository's copy to update.

## Deviations

**D-22, new. It is already in `docs/hld/DEVIATIONS.md`**, added with the
design-approval change so `scripts/deviation_check.py` resolves this plan's
citation. Read the row there rather than a copy here, because a plan carrying a
second copy of a register row is a second place for it to go stale.

- **D-19**, atomic registration. Already declared, reused unchanged
- **D-21**, bounded decoder-owned allocation. Already declared. `openjph-core`'s
  per-row `Vec<i32>` clone is its largest instance, so the row's text gains that
  package by name and its `Raised` cell gains F-027
- **D14** (decision, not deviation, and the hyphen matters): `.203` is published
  as a measured divergence

## LLD impact

- `docs/lld/codecs.md`, an HTJ2K section covering the three UIDs, the CAP marker
  requirement, the RPCL progression rule that separates `.201` from `.202`, the
  pinned `.203` divergence, and the row-clone allocation D-21 covers
- `docs/SOURCE-POLICY.md`, the `openjph-core` section updated from "F-X013 uses
  it only in a throwaway measurement crate" to production use, with the unsafe
  audit and the outstanding notice-material condition and its gate
- `docs/spikes/A1-htj2k-route.md` and `docs/spikes/A2-jpeg-ls.md`, the `E2.6`
  references corrected to F-027 and F-028. **Measurements untouched**
- `docs/spikes/GATES.md`, A1's row gains the production outcome
- `ci/wasm-size-budget.json`, rebaselined with the delta attributed

## Open questions

None. All three were answered in the S09 consolidated design round.

## Decisions from the S09 design round

**1. Register, and hold condition 2 with a release-profile gate.** HTJ2K's three
UIDs become `Available` this sprint. Six of F-X013's seven conditions close here.
Condition 2, the missing BSD notice and copyright material, becomes a
`scripts/pin_and_size_check.py` assertion scoped to the `--all` and release
profiles, which **fails today and names the missing file and the reason**, so
ordinary development proceeds and `/release` cannot publish.
`docs/SOURCE-POLICY.md` already decided `Depend? yes` on 2026-09-06 and its own
sentence says the material must be obtained "before shipping the dependency",
which is exactly what a release gate enforces and a note does not.

**The alternative was on the table and was rejected rather than overlooked.**
F-X013's condition 7 says keep HTJ2K unavailable until all seven pass, and one
will not pass this sprint. Reporting unavailable would have been a defensible
reading. It was declined because the evidence does not require it and the cost is
three of sixteen transfer syntaxes.

**The gate must be written to fail, and that is deliberate.** A check that
passes vacuously today would be worse than no check, because the next person
would read a green gate as the condition being met. Its failure message names
`vendor/openjph-core-0.1.0/LICENSE`, says the package ships no licence text, and
points at this row.

**2. The dependency's audit surface is accepted, with the audit written down.**
22,504 Rust code lines in 38 files, 104 `unsafe fn`, `unsafe impl` or
`unsafe { ... }` constructs across nine files, of which 50 are on paths reachable
on wasm32. There is no alternative candidate, so the real choice was this
dependency or no HTJ2K. Condition 4's audit is still done and still written into
`docs/SOURCE-POLICY.md` in the shape the `oxideav-core` audit already has,
because the number being accepted is not the same as the number being unmeasured.

**3. `tools/spikes/a1-htj2k/` and `tools/spikes/x013-htj2k-route/` are deleted by
this story.** Same answer as F-028's, one decision covering all three harnesses.
`docs/spikes/A1-htj2k-openjp2.md` and `docs/spikes/A1-htj2k-route.md` stay,
because they are the evidence, and each gains a note that the rig it names has
been removed and at which commit.
