# F-X006, Answer Appendix A gates A1 (HTJ2K) and A2 (JPEG-LS) against our own decoders

**Status**: approved
**Epic ref**: Y1.1
**Sprint**: S03
**Estimate**: 3w

## Normative source, transcribed

_Transcriptions are verbatim. Two mechanical notes. First, `scripts/prose_check.py`
skips fenced code blocks and Markdown table rows, so a transcribed table cell and
a transcribed code listing keep the author's em-dashes and semicolons exactly as
written. Second, where a quotation outside a table or a fence carries an em-dash
or a prose semicolon, it is normalised to a hyphen or a comma and the normalisation
is flagged at that quotation. No word is changed anywhere. Where exact bytes
matter, the tracked Markdown under `docs/hld/` wins._

### `docs/hld/A-spike-gates.md`, Appendix A, the framing sentence and the two rows

> Each of these can end or reshape the programme, and each is cheap to answer. They belong in the first six weeks, with the authority to stop.

| **Gate** | **Question** | **Consequence if it fails** |
|----|----|----|
| A1 | Does HTJ2K decode correctly in openjp2 under wasm32, bit-exact against OpenJPH? | You maintain C codec builds regardless, and one of the four arguments for the rewrite weakens |
| A2 | What is the JPEG-LS answer — CharLS bridge, self-compiled CharLS, or a young pure-Rust crate? | Changes the architecture, not just a dependency line. Decide before anything else |

Both cells above are reproduced character for character, em-dash included, because
they sit inside a table row and the prose checker relaxes both rules there.

### `docs/hld/18-codec-registry.md`, section 21, in full

_The opening sentence's em-dash is normalised to a comma, per the note at the
top of this section. No word is changed._

> Explicit runtime registration, not the inventory crate, inventory does not work on WebAssembly, which is precisely why dicom-rs's own plugin registry is unavailable there. Explicit registration is also what lets a native build link C codecs the browser build cannot.

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

|  |
|----|
| **TWO OPEN GATES** HTJ2K through openjp2 is registered in dicom-rs but unverified under wasm32 — test it bit-exact against OpenJPH output in week one. JPEG-LS has no credible pure-Rust path; the registry design deliberately allows a JS-side bridge to @cornerstonejs/codec-charls as a registered decoder, so choosing that route costs an adapter rather than a redesign. |

Three things in that listing are load-bearing for this story and are quoted here
rather than paraphrased later.

- `fn transfer_syntaxes(&self) -> &'static [&'static str]` keys the registry on
  the UID string, so the five UIDs this story measures are the five keys the
  eventual adapters will claim.
- The doc comment on `decode` reads, exactly, `/// Decode one frame into `out`. Must not allocate.`
  Every candidate library in this story allocates internally. See
  `## Open questions`, item 2.
- `pub struct Registry { by_ts: HashMap<&'static str, Arc<dyn Decoder>> }` is an
  `Arc<dyn Decoder>`, which the structural rules would otherwise refuse.
  `AGENTS.md` names this as one of two deliberate exceptions.

### `AGENTS.md`, structural rules, the exception that covers this story

> Two deliberate exceptions, both from the HLD: the `Decoder` trait
> (section 21) and the `SeriesSource` and render-target traits (section 13) exist
> with one implementer each, because they are the declared extension points that
> make Phases 2 and 3 entry points rather than rewrites.

So the answer this story recommends is allowed to be "one registered decoder",
and the trait is not held to the two-implementers rule while that is true.

### `docs/hld/12-workspace-and-build.md`, section 15.2, the dicom-rs features cell

| **DICOM-RS FEATURES** Disable default features. The defaults are rayon and simd, and the gdcm feature is native-only. On wasm you want: default-features = false, then jpeg, rle, deflate and openjp2 selected explicitly. Note also that the inventory-based transfer-syntax plugin registry does not work on wasm at all — which is why §21 specifies an explicit runtime registry instead. |

That cell names `openjp2` as the wasm feature to select and names no JPEG-LS
feature at all. The absence is the specification's own statement of A2.

### `.claude/commands/spike.md`, the steps, quoted

> 1. State the question and what a pass and a fail each look like, **before**
>    doing the work. A spike whose success criterion is written afterwards
>    always passes.
> 2. Build the smallest thing that answers it. A spike is throwaway code and is
>    not held to the gate set. Say so in the record.
> 3. **Measure. Do not reason.** A4 asks for a number. A1 and A2 ask for a
>    bit-exact comparison against an independent decoder, which means actually
>    decoding the same file two ways and comparing bytes, not reading a
>    changelog that says a format is supported.
> 4. Record the answer in `docs/spikes/A{N}-<slug>.md`: question, method, raw
>    result, interpretation, recommendation, and what changes in the plan.
> 5. If the answer changes the plan, say what changes and stop for the operator.
>    A spike that reshapes a milestone is a decision, not an implementation.

and, on ordering:

> **A2 before the codec work is designed.** The HLD is explicit that it changes
> the architecture rather than a dependency line, because a JS-side bridge to a
> CharLS build is a registered decoder in the registry design and a native
> self-compiled CharLS is not available to the browser at all.

### `docs/spikes/GATES.md`, the form a recorded gate takes

> `docs/hld/A-spike-gates.md` carries A1 to A6 and is cut from the authored
> document, so it cannot be hand-edited. Gates decided after the HLD was written
> live here, in the same form and with the same authority: **each can end or
> reshape the work it gates.**

and A7's own criteria table, which is the worked example this story copies:

> ### What a pass and a fail look like
>
> Written before the work, per `/spike` step 1.

| Outcome | Meaning |
|---------|---------|
| **Pass** | A7.1 passes on its own, and A7.3 shows interactive stack rendering is achievable inside a defensible CPU budget. Build F-X001 to F-X004 as planned |
| **Constrained** | A7.1 passes but A7.3 shows CPU stack rendering costs too much per session for the estate. Tier C still ships, but the default becomes reduced: lower interactive resolution during a drag, full resolution on settle. **A degraded mode is still tier C, and it is still declared** |
| **Fail** | CPU stack rendering cannot be made viable at any quality. Drop F-X003 and F-X004, keep F-X001 and F-X002 so a no-GPU session at least reports UNAVAILABLE honestly instead of showing a blank canvas |

Three properties of that table are the ones to copy. It has a middle outcome that
is neither pass nor fail. Each cell says what is built as a consequence, not only
what is concluded. And the whole table sits above the measurement in the file.

### `docs/spikes/A7-tier-c.md`, the sentence that sets the standard of evidence

> **No layer is an oracle.** The reference is cornerstone3D through the harness,
> per §11. A software adapter proves code runs. It does not prove a pixel is
> right.

### `docs/lld/oracle.md`, the paragraph this story exists because of

> **This does not answer Appendix A gates A1 or A2, and it is worth being exact
> about why.** A1 asks whether HTJ2K decodes correctly "in openjp2 under wasm32,
> bit-exact against OpenJPH", and A2 asks what the JPEG-LS answer is, "CharLS
> bridge, self-compiled CharLS, or a young pure-Rust crate". Both are questions
> about Ocelli's own codec build, which does not exist yet. What this story gives
> them is the other half of the measurement: all three HTJ2K rows and both
> JPEG-LS rows decode, present and read back through the reference, so there are
> reference frames for those five rows to be compared against once there is
> something to compare.

### `docs/lld/corpus.md`, the caveat that shapes the method

> - **The `j2k_*` and `jpegls_*` cases are encoded and decoded by the same
>   library**, so the conformance check on them is weaker than on the rest.

and, on what a tool is for:

> **A tool used to build a case is not a tool used to check it.** DCMTK, OpenJPH
> and OpenJPEG produce codestreams here. Whether Ocelli decodes them correctly is
> decided by the oracle against cornerstone3D, per HLD section 11.

### `docs/hld/11-decision-log.md`, section 14, the four rows this story is bounded by

| **\#** | **Decision** | **Rejected alternative** | **Consequence** |
|----|----|----|----|
| D2 | wasm-bindgen in one crate only | Bindings wherever convenient | Phases 2 and 3 are entry points, not rewrites |
| D3 | Pixels never cross the boundary | Decode in wasm, render in JS | The main architectural gain |
| D5 | Single-threaded, one instance per worker | wasm-bindgen-rayon from the start | Stays on stable Rust |
| D14 | Attestation claims measured divergence | Claim bit-exact reproducibility | Honest, publishable, and actually achievable |

### `docs/hld/24-agent-code-standards.md`, section 27.2, rules R2, R3 and R5

| R2 | Tests derive from the spec or the oracle, never from reading the implementation | An agent asked to test a function will assert what it does, not what it should do |
| R3 | Every function doing pixel arithmetic needs a fixture test with hand-computed values, citing the DICOM section | This is the defect class that reaches patients |
| R5 | No unsafe outside the allow-list (ocelli-wasm/src/ring.rs, ocelli-core/src/cast.rs) | Keeps the audit surface to two files |

### `docs/SOURCE-POLICY.md`, the rows and the rule that decide A2's candidate list

| Project | Licence | Read? | Depend? |
|---------|---------|-------|---------|
| DCMTK | BSD-style (OFFIS) | yes | yes |
| OpenJPEG, CharLS, OpenJPH | BSD-2 / BSD-3 | yes | yes |
| **dwv** | GPL-3.0 | **NO** | no |
| **Horos** | LGPL-3 with a linked AGPL-3 component (Grok) | **NO** | no |
| **Grok JPEG 2000** | AGPL-3 | **NO** | no |

> Bridging to `@cornerstonejs/codec-charls` is fine and
> so are OpenJPEG, CharLS and OpenJPH, all permissive. Grok is not.

and the rule that governs every crate this story assesses:

> **No licence is not the same as permissive.** A repository with no LICENSE
> file, and no `license` field in its metadata, is **all rights reserved** by
> default under the Berne Convention.
>
> Before taking anything from a source not in the table above, check three
> things and record the answer:
>
> 1. Is there a LICENSE, LICENCE, COPYING or NOTICE file at the repository root?
> 2. Does the hosting platform's metadata report a licence?
> 3. Does the licence permit the specific use, which for source is usually
>    **derivative works**, not merely use?
>
> If 1 and 2 are both absent, the answer is no.

### `docs/sprints/CURRENT_SPRINT.md`, what done means

> - **F-X006** produces a written answer per gate in `docs/spikes/`, not a
>   passing test. A1 is bit-exactness of HTJ2K through openjp2 under wasm32
>   against OpenJPH. A2 is the JPEG-LS architecture decision.

### `docs/hld/B-parity-surface.md`, Appendix B, the row this story reports against

| Transfer syntaxes | ~13 | Two of them, JPEG-LS and HTJ2K, are the open gates in Appendix A |

## What the specification does not cover

This section is not empty and each item below is a decision this plan makes that
the HLD does not make for it.

1. **How a wasm32 decode is actually executed.** A1 says "under wasm32" and
   names no runtime, no target triple and no way of getting bytes in or results
   out. This plan chooses `wasm32-unknown-unknown` under node, and the reasoning
   is in `## Approach`.
2. **What "bit-exact against OpenJPH" is measured over.** A1 names neither a
   canonical byte representation nor what to do with a difference. A raw decode
   is a sample array, and OpenJPH's own tool writes big-endian PGM while a DICOM
   frame is little-endian, so "the bytes" is under-specified until somebody says
   which bytes.
3. **Whether the lossy HTJ2K row is held to the same bar as the lossless ones.**
   A1 says "bit-exact" without distinguishing `1.2.840.10008.1.2.4.203`, which
   is irreversible, from `.201` and `.202`, which are not. This plan separates
   them and says why.
4. **What the A2 decision matrix is.** A2 names three candidate routes and no
   axes to choose between them. This plan names six axes and makes each one a
   recorded measurement or a recorded fact rather than a judgement.
5. **Where the A1 and A2 pass criteria live.** `A-spike-gates.md` is cut from
   the authored document and cannot be hand-edited, and `GATES.md` is for gates
   decided after the HLD was written, which A1 and A2 were not. See
   `## Open questions`, item 1.
6. **What the audit-surface claim in R5 means once a codec is a dependency.**
   R5's stated payoff is that a reviewer reads two files to audit every
   `unsafe` line. `scripts/unsafe_allowlist_check.py` reads `git ls-files`, so
   it never sees a dependency. This plan does not change R5 and does require
   the answer files to say plainly what R5 does and does not cover once a
   c2rust port or a C bridge is in the tree.
7. **Whether the A2 answer may differ per target.** The HLD writes one codec
   registry and does not say whether one transfer syntax may resolve to
   different decoders on wasm32 and on native. This plan treats a split answer
   as available, and as the outcome that costs the most under D14.

## Approach

The deliverable is two written answers under `docs/spikes/`, each with its
pass criteria above its measurements, plus the throwaway harness that produced
the numbers. **No port code is written. `crates/ocelli-codec/src/lib.rs` is
unchanged by this story and stays the scaffold it is today.** The registry is
designed by the E2.6 codec story, and this story is what tells that story which
decoders to register.

### The premise check, run before the plan was written

Recorded here because a plan that assumes an installed tool and is wrong wastes
the sprint, and because `allocation.json`'s note asserts several of these.

| Premise | Checked | Result |
|---------|---------|--------|
| OpenJPH installed | `which ojph_compress ojph_expand`, `brew list --versions openjph` | present, `openjph 0.31.0`, both binaries at `/opt/homebrew/bin` |
| DCMTK installed, `dcmdjpls` present | `which dcmdjpls dcmcjpls`, `brew list --versions dcmtk` | present, `dcmtk 3.7.0` |
| Five corpus rows in the manifest | `grep` over `corpus/manifest.tsv` | all five present, lines 34, 35, 36, 44, 45 |
| Five corpus files present | `ls corpus/data/syntax/` | all five present, unread |
| An openjp2 or JPEG-LS crate already resolved | `grep` over `Cargo.lock` and `Cargo.toml` | none. `Cargo.lock` has no `dicom`, no `openjp2`, no `charls`. The `dicom` workspace dependency is declared and unresolved, and section 15.2's comment says it activates at F-016 |
| wasm32 targets installed | `rustup target list --installed` | `aarch64-apple-darwin` and `wasm32-unknown-unknown` only. No `wasm32-wasip1` |
| A standalone wasm runtime | `which wasmtime wasmer` | neither. `node` v24.16.0 is present and is the runtime this plan uses |
| A C or C++ toolchain for wasm | `which cmake emcc` | neither is installed |

Two of those results shape the design directly. There is no `wasm32-wasip1`
target and no standalone runtime, so the wasm32 measurement is taken on
`wasm32-unknown-unknown` under node, which is the target that ships anyway.
And there is no `cmake` and no `emcc`, so the self-compiled CharLS route cannot
be built for wasm32 on this machine without a toolchain the repository does not
have, which is a fact about that route and is recorded as one rather than
treated as a local inconvenience.

### The shared measurement rig, and the one comparator

Both gates ask the same question in the same shape: decode one frame two ways
and compare bytes. So there is **one** comparator, in
`tools/spikes/common/compare.mjs`, used by both. Writing a second would be the
defect this repository is built to distrust, and a comparator that disagrees
with itself between two answer files is worse than no answer.

**The canonical form.** Every decode of every one of the five rows is reduced to
the same thing before comparison: **12288 bytes, little-endian `u16`, 6144
samples, row-major, 64 rows by 96 columns.** That shape is not chosen, it is
what `scripts/corpus_synth.py` produced. `SYNTAX_ROWS, SYNTAX_COLS = 64, 96`,
`syntax_base(..., "mono16")` writes `ramp(SYNTAX_ROWS, SYNTAX_COLS, 65535, np.uint16)`
with `BitsAllocated` 16, `BitsStored` 16, `HighBit` 15, `PixelRepresentation` 0,
and every one of the five compressed cases is encoded from
`syntax/explicit_vr_le.dcm`, which is `REFERENCE_MONO16`.

Conversions into canonical form, each one named because a mistake here presents
as a total mismatch and reads as a decoder defect:

- `ojph_expand` writes a PGM. `P5` with maxval 65535 is **big-endian** by the
  PGM specification, so the conversion is a header skip and a byte swap. The
  swap is done in the comparator, never in a decoder, so no decode is touched
  by our arithmetic.
- `dcmdjpls` writes a DICOM file in an uncompressed transfer syntax. The
  canonical bytes are its `PixelData` element value, read by a small pydicom
  step in `tools/spikes/common/extract.py`, which reads only Rows, Columns,
  BitsAllocated, PixelRepresentation and PixelData.
- The Rust decodes return native-endian `u16` samples on a little-endian
  target. The comparator asserts the length is exactly 12288 before comparing
  anything, so a truncation cannot pass as an equality over a shorter buffer.

**The comparison itself.** `sha256` over the 12288 canonical bytes for the
headline result, and, whenever two digests differ, an index-level report:
the number of differing samples, the first differing sample index with both
values, the maximum absolute difference, and a histogram of differences by
magnitude. The digest says equal or not equal. The index report says what to do
next, and both go into the answer file. **A difference is never absorbed. It is
either explained by a named property of the format, which makes the outcome
`Constrained` and publishes the bound, or it is a defect and the outcome is
`Fail`.** There is no third move, and in particular there is no tolerance,
because none of these comparisons is a rendering comparison and HLD section 25.1
does not reach them.

**The comparator is observed red before any digest it prints is believed.**
`tools/spikes/common/tests/compare_test.mjs` feeds it two buffers differing in
one sample and asserts it reports exactly that sample. This is S03's stated gate
defect class applied to a spike, from `CURRENT_SPRINT.md`: a guard mutated in the
same command that adds it has been observed to fire once and has nothing watching
it afterwards. The mutation runs as its own command.

**Inputs.** `tools/spikes/common/extract.py` reads the five corpus rows with
pydicom, decapsulates the single fragment of each `PixelData` item, and writes
the raw codestreams plus the uncompressed reference into ignored
`tools/spikes/out/`. **Nothing under `tools/spikes/out/` is tracked**, for the
same reason nothing under `tools/oracle/out/` is: `.gitignore` covers it and
`scripts/staged_content_check.py` already refuses staged DICOM by magic bytes.
See `## Open questions`, item 10, on whether the path guard needs extending.

### A1, the method

**Three decodes and one anchor**, per row.

| Symbol | What it is |
|--------|-----------|
| `D_wasm` | `openjp2` compiled to `wasm32-unknown-unknown`, executed under node |
| `D_native` | the same `openjp2` version compiled to `aarch64-apple-darwin` |
| `D_ojph` | `ojph_expand` 0.31.0, the independent decoder A1 names |
| `R` | `syntax/explicit_vr_le.dcm`'s pixel data, the uncompressed reference every case was encoded from |

**Three questions hide inside A1 and they must not be collapsed.** A1's literal
wording asks the third. The second is the one that would be a wasm-specific
defect, and it is what the gate's consequence column is really about.

1. Does `openjp2` build for `wasm32-unknown-unknown` and accept an HTJ2K
   codestream at all?
2. Does `openjp2` under wasm32 agree with `openjp2` native, byte for byte?
3. Does `openjp2` agree with OpenJPH, byte for byte?

Question 1 is answered first and on its own, because there is a real chance it
ends the gate. What is already known from the manifest, and is a signal rather
than the measurement: the `openjp2` crate is described as a "Rust port of
Openjpeg", the upstream README describes a C2Rust port of `src/lib/openjp2`,
"Part 1 & 2", and HTJ2K is Part 15. Against that, the published crate's source
listing for 0.6.1 contains `ht_dec.rs` and `t1_ht_luts.rs`, and OpenJPEG 2.5.2
in this repository's own `.venv` decodes `corpus/data/syntax/htj2k_lossless.dcm`
today. So the question is genuinely open and it is answered by building and
running, not by reading either of those facts.

**How the wasm32 measurement is actually taken.** This is the part A1 leaves
entirely unspecified and it is where the constraints bite.

- The harness crate is `tools/spikes/a1-htj2k/`, a `cdylib` for
  `wasm32-unknown-unknown`. It is **not** a workspace member. The root
  `Cargo.toml` has `members = ["crates/*", "tools/oracle"]`, so the spike
  directory is already outside it, and the spike's own `Cargo.toml` carries an
  empty `[workspace]` table so cargo treats it as its own workspace and the root
  manifest is not edited.
- **No `wasm-bindgen`.** The module is a plain cdylib with a hand-written integer
  ABI. `ci/check-bindgen-isolation.sh` scopes all three of its passes to
  `crates/*/`, so it never looks here, and even if it did there is nothing to
  find. D2 and D-12 are untouched.
- **No `unsafe`, and the reason is not obvious.** Rust 2024 requires
  `#[unsafe(no_mangle)]` on an exported symbol, and
  `scripts/unsafe_allowlist_check.py` matches the bare token `unsafe` over every
  tracked `.rs` file from `git ls-files`, with `crates/ocelli-wasm/src/ring.rs`
  and `crates/ocelli-core/src/cast.rs` the only exceptions. **A tracked spike
  file containing no unsafe code at all would fail `gate unsafe` on an
  attribute.** So the spike crate is pinned to `edition = "2021"` deliberately,
  where `#[no_mangle]` is spelled without the token. The edition is a decision,
  not an inheritance, and the crate says so in a comment.
- **Input reaches the module without a pointer.** The three codestreams are
  embedded with `include_bytes!` from ignored `tools/spikes/out/`, selected by an
  integer argument. Nothing is written into linear memory from outside, so
  nothing needs a raw pointer inbound.
- **Output leaves the module without a pointer dereference in Rust.** The module
  exports `decode(case: u32) -> u32` returning zero or an error code,
  `out_len() -> u32`, and `out_ptr() -> u32`. The decoded buffer is kept alive in
  a `thread_local!` `RefCell<Vec<u8>>`, so there is no `static mut`. The driver
  reads `new Uint8Array(instance.exports.memory.buffer, out_ptr(), out_len())`,
  which is ordinary JavaScript. **The one `as` cast in the whole harness is the
  `usize` to `u32` in `out_ptr`**, it is named here so a reviewer knows exactly
  where to look under 27.3, and the driver asserts `out_len()` is 12288 before
  reading anything.
- **`wasm32-unknown-unknown` and not `wasm32-wasip1`**, for two reasons. It is
  the target that ships, because the core runs in a browser worker. And
  `rust-toolchain.toml` pins `targets = ["wasm32-unknown-unknown"]`, so
  measuring on wasip1 would mean changing the toolchain pin to measure a target
  nothing ships on. wasip1 would give a simpler harness, a `fn main()` reading a
  file with no exports and no casts, and it would answer a question A1 did not
  ask.
- **Two wasm builds, not one.** With and without `-C target-feature=+simd128`.
  `docs/spikes/A7-tier-c.md` makes SIMD128 a requirement rather than a detection
  detail, `scripts/target_feature_check.py` exists, and F-004 detects it at
  runtime this same sprint. Two builds, two digests. A difference between them is
  a finding in its own right and it is exactly the kind that only appears on
  hardware nobody develops on.

**A1's pass criteria, written before the measurement.** Per `/spike` step 1, and
these are what the answer file transcribes above its results.

| Outcome | Meaning |
|---------|---------|
| **Pass** | `openjp2` builds for `wasm32-unknown-unknown` in both SIMD configurations. For `.201` and `.202`, all of `D_wasm`, `D_native`, `D_ojph` and `R` have the same sha256. For `.203`, `D_wasm`, `D_native` and `D_ojph` have the same sha256. Then `openjp2` is the HTJ2K decoder, no C codec build is needed for HTJ2K, and the rewrite argument the gate names holds |
| **Constrained** | `.201` and `.202` pass in full, and `.203` differs from `D_ojph` while matching `D_native` exactly. The irreversible 9/7 path is where two conforming implementations may legitimately round differently, so this is an explained difference rather than a defect. `openjp2` is still the decoder, the record publishes the max absolute difference and its distribution as a D14 measured divergence, and the E2.6 codec story carries a standing test pinning that bound |
| **Fail** | Any one of: the crate does not build for wasm32, the module refuses an HTJ2K codestream, `D_wasm` differs from `D_native` for the same library, or a lossless row does not reproduce `R` exactly. Then Appendix A's consequence applies verbatim, C codec builds are maintained regardless and one of the four arguments for the rewrite weakens. The record prices the fallback rather than leaving it as a gap, and the two candidates to price are an OpenJPH build, which is what the reference itself uses at `@cornerstonejs/codec-openjph` 2.4.9, and the pure-Rust `openjph-core` |

Two notes on that table. **`D_wasm` differing from `D_native` is a `Fail` and not
a `Constrained`, whatever the size of the difference**, because a library
disagreeing with itself across targets is a wasm defect and the whole gate is
about wasm. And **the anchor `R` is what makes the lossless rows encoder-independent**.
The HTJ2K rows were encoded by `ojph_compress` with `-reversible true`, so
`D_ojph` and the encoder are the same build, and the `docs/lld/corpus.md` caveat
about a case encoded and decoded by one library applies. `R` does not come from
OpenJPH at all. It comes from `scripts/corpus_synth.py`'s hand-predictable ramp,
and `scripts/tests/test_corpus_synth.py::test_lossless_syntaxes_round_trip_exactly`
already asserts the round trip through a third decoder, pydicom's. So for `.201`
and `.202` there are three independent parties and the circularity is broken.
For `.203` there is no such anchor and the record must say so rather than imply
one. See `## Open questions`, item 8.

### A2, the method

A2 is a decision, not a pass or a fail, so its table is a set of outcomes and
each one names what gets built. The candidates below all had their licence
checked against `docs/SOURCE-POLICY.md`'s three questions, and **none is
copyleft, so none is read-blocked.** The read-blocked projects are named in the
policy and are not candidates and must not be opened, and that includes the AGPL-3
JPEG 2000 library the policy lists, which is not a candidate here for exactly
that reason.

| Route | Concretely | Licence, checked | Read-blocked |
|-------|-----------|------------------|--------------|
| **R1**, JS-side bridge | `@cornerstonejs/codec-charls` 1.2.5, already pinned in `tools/oracle/package.json`, instantiated as a second wasm module inside the same worker | package MIT, CharLS inside it BSD-3 | no |
| **R2**, self-compiled CharLS | `charls` 0.4.2 over `charls-sys` 2.4.5 over the CharLS C++ library | crates MIT, CharLS BSD-3 | no |
| **R3**, young pure-Rust crate | `pure_jpegls` 2.0.0, first published 2026-06-01, 407 downloads | MIT OR Apache-2.0 | no |
| **R3b**, pure-Rust alternates | `dicom-toolkit-codec` 0.5.0, `ritk-codecs` 0.6.0 | MIT OR Apache-2.0, both | no |
| **R4**, dicom-rs's own answer | `dicom-transfer-syntax-registry` 0.10's `charls` feature | MIT OR Apache-2.0 | no |

**R4 collapses into R2 at the bottom, and that is the finding, not a footnote.**
`dicom-transfer-syntax-registry` 0.10's `charls` feature resolves to
`dep:charls`, which is `charls` 0.4.2, which is `charls-sys`, which is the C++
library. dicom-rs has no pure-Rust JPEG-LS route, which is section 21's sentence
"JPEG-LS has no credible pure-Rust path" still being true inside the library the
HLD selects. The corresponding `openjp2` feature resolves to `dep:jpeg2k` plus
`jpeg2k/openjp2`, which is the pure-Rust `openjp2` crate A1 measures, so the two
gates are asymmetric in the dependency graph and not only in the prose.

**Six axes, each a recorded measurement or a recorded fact.**

1. **Does it build for `wasm32-unknown-unknown`?** Yes or no, with the exact
   command and, if no, the verbatim error. R2, R4 and anything else over
   `charls-sys` need a C++ toolchain targeting wasm plus `cmake`, and neither
   `cmake` nor `emcc` is installed here. That is not a local inconvenience, it is
   the shape of the route.
2. **Correctness**, on both JPEG-LS rows, through the same comparator and the
   same canonical 12288 bytes as A1. Anchors are `dcmdjpls` 3.7.0 for both rows,
   and `R` for `.80` only.
3. **Coverage.** Does the candidate handle `.80` **and** `.81`, 16-bit samples,
   and `NEAR > 0`. `pure_jpegls` describes itself as a lossless codec, so `.81`
   is the first thing to try and it may end that route on its own.
4. **Binary size.** The release wasm delta, against `ci/wasm-size-budget.json`'s
   recorded 14104 bytes and 0.05 tolerance. That file is gate A4's currency and
   A2's answer will move it.
5. **Provenance**, against `docs/SOURCE-POLICY.md`'s three questions, plus one
   this project needs specifically: **what does the candidate claim to be a port
   of?** A port of CharLS or of OpenJPH is fine and both are in the policy's yes
   column. A port of a read-blocked source is not, and the check is reading the
   candidate's own README and NOTICE rather than inferring from a `license`
   field, because a permissive field on a derivative of blocked source proves
   nothing. Each candidate that survives gets a row in the policy's "Extensions
   to the table" with a decided date. See `## Open questions`, item 5.
6. **Maintenance risk**, stated as facts and not as a feeling: first publish
   date, latest publish date, download count, and whether the repository
   resolves. `pure_jpegls` 2.0.0 is the "young pure-Rust crate" A2 names,
   literally, and the record says how young in dates rather than in adjectives.

**The architecture consequences, which are what actually decide it.** These are
reasoning and are labelled as such, sitting beside the measurements rather than
mixed into them.

- **R1 puts a second wasm module and a second linear memory inside the worker.**
  Decoded pixels are then copied out of that module's memory into ours, once per
  frame. **That is not a D3 violation**, because D3 is about the worker to
  main-thread boundary and both modules live worker-side, and the plan says so
  explicitly rather than leaving a reviewer to wonder. It is a per-frame copy the
  other routes do not have, and it is stated as a cost.
- **R1's real problem is a different one.** There is no JavaScript on the native
  desktop and server targets, so R1 means **those targets have no JPEG-LS
  decoder at all.** D2's stated consequence is "Phases 2 and 3 are entry points,
  not rewrites", and a transfer syntax that exists only in the browser is a hole
  in exactly that claim. **This is the strongest single argument in A2 and it is
  an architecture argument rather than a measurement**, which is why it is here
  and not in the axes above.
- **R2 is the reverse hole.** It is the natural native answer and the difficult
  browser one, and on this machine it is currently the impossible browser one.
- **A split answer, R2 native and R1 browser, is available and is the outcome
  that costs the most under D14**, because two implementations of one decode on
  two targets is precisely the measured-divergence surface D14 commits to
  publishing. It is checkable, which is what makes it defensible at all: `.80`
  is lossless so a difference is a defect, and `.81` is deterministic given
  `NEAR`, so a difference is also a defect. A split answer is allowed only if
  that divergence is measured to be zero and a standing test keeps it zero.
- **R3, if it decodes both rows bit-exact against `dcmdjpls` and builds for
  wasm32, is the only route that is one implementation on every target.**
  Section 21's note says that route did not exist. That sentence is a fact about
  when the HLD was written, and this gate is the re-check.

**A2's outcomes, written before the measurement.**

| Outcome | Meaning |
|---------|---------|
| **Pure Rust** | A pure-Rust crate decodes both `.80` and `.81` bit-exact against `dcmdjpls`, reproduces `R` exactly for `.80`, builds for `wasm32-unknown-unknown` and for native, is permissively licensed and is not a port of a read-blocked source. One implementation on every target. Recommend it by name and version, record the maintenance risk in dates and download counts, and record the size delta |
| **CharLS, one route** | No pure-Rust crate qualifies and exactly one CharLS route serves every target. Register that one, record the toolchain it costs, and record what it does to the R5 audit surface |
| **CharLS, split** | No pure-Rust crate qualifies and no single route serves every target, so the browser gets R1 and native gets R2. **This is the outcome the gate means by "changes the architecture, not just a dependency line."** Allowed only with both decodes measured byte-identical on both rows and a standing test that keeps them so, and it stops for the operator per `/spike` step 5 |
| **Blocked** | No route decodes both rows correctly on any target. JPEG-LS reports UNAVAILABLE, which is section 31's rule generalised and is what deviation D-07 already requires of a feature that cannot run. Appendix B loses a transfer syntax from the parity surface, and that is a product decision for the operator, not an engineering one |

The fourth outcome is deliberately not called a fail. A2 is a choice, and the
honest fourth answer is that none of the choices work.

### What the two answer files contain

`docs/spikes/A1-htj2k-openjp2.md` and `docs/spikes/A2-jpeg-ls.md`, each in
`A7-tier-c.md`'s shape and each following `/spike` step 4: question, the criteria
table above the results, method, raw result, interpretation, recommendation, and
what changes in the plan. Three things every `/spike` record here must carry and
which are easy to leave out:

- **That the harness is throwaway and is not held to the gate set**, per step 2,
  together with the precise statement of which gates do reach it anyway.
  `unsafe`, `prose`, `provenance` and `content` all read `git ls-files` and
  therefore see `tools/spikes/`. `clippy`, `test`, `bindgen` and `nostd` are
  scoped to the workspace or to `crates/` and do not.
- **The exact versions of everything**, in the manner `docs/lld/oracle.md` pins
  the reference: crate versions, `ojph_expand` 0.31.0, DCMTK 3.7.0, the rustc
  pin 1.97.1, node v24.16.0, and the corpus manifest digest the codestreams came
  from. An answer that cannot be re-run is an opinion.
- **What R5's audit-surface claim covers once a codec is a dependency.**
  `openjp2` is a C2Rust port and is substantially `unsafe` internally, and any
  `charls-sys` route is `extern "C"` and unsafe by construction.
  `scripts/unsafe_allowlist_check.py` reads `git ls-files` and never sees either.
  **This is not a reason to reject either route.** It is a reason the record must
  not claim an audit surface the project does not have, and it is the kind of
  thing that is discovered by a device-submission reviewer rather than by us if
  nobody writes it down now.

## Boundary and tier

- **wasm-bindgen**: not touched. The A1 harness is a plain cdylib with an integer
  ABI and declares no `wasm-bindgen`. `ci/check-bindgen-isolation.sh` scopes its
  source grep, its manifest pass and its `cargo tree` loop to `crates/*/`, so it
  does not reach `tools/spikes/` and there is nothing there for it to find. D2
  and D-12 are unchanged.
- **Pixels across the boundary**: no. Everything in this story runs on the host
  or under node. There is no viewport, no worker, no ring buffer and no main
  thread. The one place the question arises is A2's route R1, and the plan
  answers it above: a second wasm module instantiated **inside the worker** is
  not a D3 crossing, because D3 governs the worker to main-thread boundary, and
  the per-frame copy it costs is recorded as a cost rather than waved at.
- **Render-loop allocation**: n/a, this story has no render loop. Worth stating
  once rather than omitting, because section 21's `decode` doc comment says
  "Must not allocate" and every candidate library allocates internally. That is
  a constraint on the eventual adapter, the answer files record it as a
  requirement the recommended route has to meet, and the ambiguity in the
  comment itself is `## Open questions` item 2.
- **unsafe**: none in this story's tracked code, and three named pressure points
  rather than a bare assertion.
  1. Rust 2024's `#[unsafe(no_mangle)]` would fail `gate unsafe` on a file
     containing no unsafe code. Avoided by pinning the spike crates to
     `edition = "2021"`, which is a decision and is commented as one.
  2. Getting decoded bytes out of a wasm module is the obvious place a raw
     pointer dereference appears. Avoided by doing the read in JavaScript from
     `instance.exports.memory.buffer` and returning only integers from Rust.
  3. **A C codec bridge is exactly where the unsafe pressure appears**, and this
     story does not build one. `charls-sys` and `openjpeg-sys` are `extern "C"`
     and unsafe by construction, and a dependency is invisible to
     `scripts/unsafe_allowlist_check.py` because it reads `git ls-files`. If A2
     recommends R2 or a split, **no `unsafe` enters this repository and R5 still
     passes mechanically, while R5's stated purpose is weakened.** The answer
     file says that in those words. It is not a deviation, because R5 is a rule
     about this repository's files and it continues to hold. It is a claim that
     would be false if restated more broadly, and the record is where it gets
     stated correctly.
- **Tier A (WebGPU)**: n/a. Decode is CPU work in a worker and completes before
  anything reaches a device. The resolved tier does not select a decoder.
- **Tier B (WebGL2)**: n/a, for the same reason.
- **Tier C (CPU)**: n/a as a tier decision, and load-bearing as a consequence.
  Deviation D-07 makes tier C a primary clinical path, and a session on tier C
  runs **exactly the same decoder** as one on tier A. **There must never be a
  tier-gated codec path.** A second decode behind a tier check is the same defect
  section 18 forbids for the LUT chain, with the added property that it would
  only ever run on hardware nobody develops on. So the answer is n/a and the
  sentence after it is the reason.

**The axis that does matter here is not the tier, it is the target.** Browser
against native. A2's answer is a matrix over targets, and the three tier rows are
n/a specifically because the interesting availability question in this story is a
different one. Stating that is the point of requiring the rows.

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| `conformance` | Each of the five transfer syntaxes decodes to the bytes an independent decoder produces, and each lossless one to the uncompressed reference. This is the deliverable, expressed as digests in the answer files rather than as a suite | `tools/spikes/a1-htj2k/run.mjs`, `tools/spikes/a2-jpeg-ls/run.mjs`, recorded in `docs/spikes/A1-htj2k-openjp2.md` and `docs/spikes/A2-jpeg-ls.md` |
| `unit` | The comparator reports a difference when given one, at the right index, and refuses a buffer that is not exactly 12288 bytes. Observed red by mutation, in its own command | `tools/spikes/common/tests/compare_test.mjs` |
| `fixture` | **Not applicable, and stated rather than omitted.** This story computes no pixel arithmetic of its own. Its hand-computed anchor already exists and is cited rather than duplicated: `syntax/explicit_vr_le.dcm` is a 64 by 96 `uint16` ramp with `BitsStored` 16 and `HighBit` 15 per PS3.3 C.7.6.3.1.1 and C.7.6.3.1.2, written by `scripts/corpus_synth.py`, and the two lossless HTJ2K rows and the lossless JPEG-LS row must reproduce it exactly by construction | the existing `scripts/tests/test_corpus_synth.py::test_lossless_syntaxes_round_trip_exactly` and `::test_jpeg_ls_near_lossless_holds_its_declared_near_bound` |

Two notes on that table, both from 27.2.

**R3 says a fixture is mandatory for pixel arithmetic and this story has none.**
The row is present and says so, because an omitted row and a deliberate "no pixel
arithmetic here" read identically six months later.

**R2 says tests derive from the specification or the oracle, never from reading
the implementation.** The `.81` criterion is the clearest case: the bound is not
"whatever `dcmdjpls` produced", it is ISO/IEC 14495-1's guarantee that the
maximum absolute error is at most `NEAR`, which `scripts/corpus_synth.py` sets to
`JPEG_LS_NEAR = 3` and `scripts/tests/test_corpus_synth.py` already asserts against
the reference. So a candidate decoder that satisfies the NEAR bound but disagrees
with `dcmdjpls` has produced a legal but different image, and for us that is a
defect rather than a pass, because near-lossless decoding is deterministic given
the codestream.

**None of this story's output goes into the gate set.** No new gate, no change to
`bin/ocelli.sh`, no change to `.github/workflows/ci.yml`. A spike is throwaway
per `/spike` step 2, and any standing test that the answers turn out to justify
belongs to the E2.6 codec story that registers the decoder, not to this one.

## Parity surface covered

`docs/hld/B-parity-surface.md`, the row:

| Transfer syntaxes | ~13 | Two of them, JPEG-LS and HTJ2K, are the open gates in Appendix A |

**This story closes the two gates named in that row and implements none of the
thirteen syntaxes.** The row moves from "open gate" to "decided route", and the
count itself is unchanged until E2.6 registers a decoder. Appendix B has no
`Covered by` column in this repository's copy, so nothing there is edited.

## Deviations

**None.** This plan proposes no new `D-NN` row and cites none, so
`scripts/deviation_check.py` has nothing to assert for it.

Two deviation-shaped consequences follow from the answers rather than from this
story, and both are recorded in the answer files as things the E2.6 codec story
will have to raise. Naming them here so they are not discovered later as
surprises.

- **`openjp2` needs `std`.** `crates/ocelli-codec/src/lib.rs` today carries
  `#![cfg_attr(not(test), no_std)]`, and activating a decoder that needs `std`
  drops it. That is the same shape as D-10, where `wgpu` cost `ocelli-render`
  and `ocelli-compute` a `no_std` posture the HLD neither requires nor forbids,
  and `scripts/no_std_check.py` is the gate that will notice.
- **A split A2 answer is an architecture change**, and the gate says so. If the
  measurements land there, `/spike` step 5 applies and the work stops for the
  operator rather than proceeding into a design.

## LLD impact

`/complete-feature` step 9 updates:

- **`docs/lld/corpus.md`.** Its caveat "The `j2k_*` and `jpegls_*` cases are
  encoded and decoded by the same library, so the conformance check on them is
  weaker than on the rest" is directly addressed by this story for the JPEG-LS
  rows, which gain an independent `dcmdjpls` decode, and partly for the HTJ2K
  rows, which gain an `openjp2` decode alongside the OpenJPH one. The file
  should say what is now checked and by what, and should keep saying what is
  still not.
- **`docs/lld/oracle.md`.** Its paragraph "This does not answer Appendix A gates
  A1 or A2" should point at the two answer files rather than only at their
  absence, because a reader arriving at that paragraph after this story is
  looking for exactly that link.

No new LLD file is proposed. The spike answers are the deliverable and they live
in `docs/spikes/`, which is not the LLD. Whether the codec area gets a
`docs/lld/codecs.md` is a question for E2.6, when there is a registry to describe.
See `## Open questions`, item 7.

## Open questions

Numbered, and each one names the decision it blocks.

1. **Where do the A1 and A2 pass criteria live so that `/spike` step 1 is
   checkable?** `docs/hld/A-spike-gates.md` is cut from the authored document and
   cannot be hand-edited. `docs/spikes/GATES.md` states it carries gates
   "decided after the HLD was written", which A1 and A2 were not. Proposal: this
   plan carries the criteria, the answer files transcribe them above their
   results the way `A7-tier-c.md` does, and `GATES.md` gains a short pointer
   saying that the criteria for A1 and A2 live in their answer files and were
   written before the measurement. **That `GATES.md` edit is the operator's, not
   this agent's.** _Blocks: whether the answer files can satisfy step 1 in a way
   a reviewer can verify from `GATES.md` alone._
2. **What does section 21's `/// Decode one frame into `out`. Must not allocate.`
   mean?** Two readings. Narrow: the decoder must not allocate the output,
   because `out: &mut [u8]` is caller-provided, and internal scratch is fine.
   Broad: no allocation at all. Every candidate in this story allocates
   internally, `openjp2` conspicuously so, since it is a C2Rust port carrying its
   own `malloc.rs`. Under the broad reading no route passes and the answer files
   must say the registry contract as written cannot be met. **F-006's plan
   transcribes the same doc comment and builds its definition of "decode" on it,
   so this needs one answer for both stories rather than two.** _Blocks: whether
   A1's and A2's recommendations can claim to satisfy the registry contract,
   whether E2.6 needs a deviation, and what F-006 is timing._
3. **Confirm the wasm32 measurement is taken on `wasm32-unknown-unknown` under
   node, and that `rust-toolchain.toml` is not touched.** `wasm32-wasip1` would
   give a simpler and cast-free harness but would measure a target nothing ships
   on and would need a toolchain pin change. Recommendation is
   `wasm32-unknown-unknown`. _Blocks: the harness shape, and whether this story
   edits `rust-toolchain.toml`._
4. **Confirm the spike crates carry their own empty `[workspace]` table rather
   than being added to a root `exclude` list**, so the root `Cargo.toml` is not
   edited. The consequence is that `gate clippy` and `gate test`, which run
   `--workspace`, never see the spike code, while `unsafe`, `prose`, `provenance`
   and `content`, which read `git ls-files`, do. That asymmetry is intended and
   is stated in the answer files, and it should be confirmed rather than assumed.
   _Blocks: whether F-X006 touches the root manifest._
5. **Each surviving A2 candidate needs a `docs/SOURCE-POLICY.md` "Extensions to
   the table" row with a decided date**, in the same form as the existing TCIA
   and pydicom rows. Candidates to assess are `pure_jpegls` 2.0.0, `charls` 0.4.2,
   `charls-sys` 2.4.5, `@cornerstonejs/codec-charls` 1.2.5, and, if A1 fails,
   `openjph-core` 0.1.0. All report permissive licences in crate and package
   metadata, checked, and none is copyleft. **Editing that normative file is the
   operator's, not this agent's.** _Blocks: depending on any candidate, and
   arguably blocks reading any candidate's source closely enough to assess it._
6. **If A1 fails, is the fallback priced inside this story or deferred?**
   Appendix A's consequence column says C codec builds are maintained regardless,
   and pricing an OpenJPH route, whether the C library or the `openjph-core`
   crate, is a separate piece of work. Recommendation is that the answer file
   names the fallback and its two candidates and does not price it, and that
   `/spike` step 5 stops for the operator. _Blocks: the 3w estimate, and whether
   a follow-up story is opened in S04._
7. **Does F-X006 create a `docs/lld/` file, or do the spike answers stand
   alone?** Recommendation is that they stand alone and that `docs/lld/codecs.md`
   is created by E2.6 when there is a registry to describe. _Blocks:
   `/complete-feature` step 9's file list._
8. **Is the `.203` lossy HTJ2K row's lack of an independent anchor acceptable?**
   For `.201` and `.202` there are three independent parties: OpenJPH encoded,
   `openjp2` decodes, and the uncompressed reference `R` comes from
   `scripts/corpus_synth.py`'s ramp with a third decode already asserted by
   `scripts/tests/test_corpus_synth.py`. For `.203` there is no `R`, so the only
   comparison is `openjp2` against OpenJPH, and OpenJPH also encoded it. The
   comparison is still cross-implementation on the decode side and is what A1
   literally asks for. _Blocks: whether a `.203` case from a second encoder needs
   adding to the corpus, which would be a corpus change and a new manifest row._
9. **A parallel question for JPEG-LS, and it is sharper.** The `.80` and `.81`
   rows were encoded by `pyjpegls` 1.5.1, which wraps CharLS, and the independent
   decode is `dcmdjpls` from DCMTK 3.7.0, which also uses CharLS. **So the A2
   anchor is CharLS on both sides.** The strong anchors are therefore `R` for
   `.80`, which is encoder-independent and exact, and ISO/IEC 14495-1's NEAR
   bound for `.81`, which is the standard rather than an implementation.
   Recommendation is to say this plainly in the answer file and treat `dcmdjpls`
   as a second CharLS reading rather than as an independent implementation.
   _Blocks: how strongly A2's correctness axis can be worded, and whether a
   non-CharLS JPEG-LS decode needs sourcing._
10. **Does `scripts/staged_content_check.py` need `tools/spikes/out/` added by
    path, the way `tools/oracle/out/` already is?** `.gitignore` will cover it and
    the pre-commit hook refuses staged DICOM by magic bytes, but a raw `.j2c`
    codestream extracted from a synthetic corpus row is neither a DICOM by magic
    bytes nor an oracle output. The corpus rows in question are synthetic and
    carry no patient data, so the risk is tracked build artefacts rather than
    disclosure, but the parallel is close enough to ask. **That script edit is the
    operator's.** _Blocks: whether ignoring the directory is sufficient._
11. **Confirm the S03 boundary with F-011.** F-011 builds a pixel-diff comparator
    with a per-modality tolerance policy. This story builds an exact-equality byte
    comparator with no tolerance. **They must not become two general comparators.**
    Recommendation is that this one stays throwaway inside `tools/spikes/common/`,
    is never imported by anything under `crates/` or `tools/oracle/`, and says so
    in its own header. _Blocks: whether F-011 and F-X006 can run concurrently
    without converging on one shared utility that serves neither well._
12. **Two further S03 boundaries, both narrow and both worth confirming.**
    **F-005** owns the error model, and section 21's signature returns
    `Result<(), CodecError>`. This story names decoder error codes as raw
    numbers out of the harness and **does not define `CodecError`**, which is
    F-005's to shape and E2.6's to populate. **F-004** owns SIMD128 detection at
    runtime, including a `WebAssembly.validate` probe over a committed module.
    This story compiles two wasm builds, with and without
    `-C target-feature=+simd128`, and reports two digests. Those are the
    build-side and the runtime-side halves of one question and neither should
    restate the other's conclusion. _Blocks: nothing on its own, and left open
    so the consolidated design round can confirm the split rather than three
    plans each assuming it._

---

## Decisions taken in the design round

Answers to `## Open questions`, taken in the S03 consolidated round.

**1. The pass and fail criteria live in this plan and are transcribed into the
answer files.** `docs/hld/A-spike-gates.md` is the author's text and is not
hand-edited. `docs/spikes/GATES.md` gains a pointer to the two answer files,
which this story may add, because that file exists for exactly this and already
carries A7 in the same shape. `/spike` step 1 is then checkable against a
committed artefact written before the measurement.

**2. Section 21's "Must not allocate" is read narrowly: no allocation per
decode call.** The output buffer is caller-provided, which is what the signature
is for, and a decoder allocating internal state at registration or first use is
outside the rule. Every candidate allocates internally and `openjp2` carries its
own `malloc.rs`, so the broad reading would fail every route including the one
the HLD names. **This answer is shared with F-006**, which transcribes the same
comment and times `Decoder::decode`, so both stories carry one reading rather
than two.

**3. The wasm32 measurement is taken on `wasm32-unknown-unknown` under node.**
It is the target that ships and the only one `rust-toolchain.toml` pins. A
`wasm32-wasip1` harness would be simpler and would measure a target nothing
ships on, and it would need a toolchain pin change. `rust-toolchain.toml` is not
touched.

**4. Spike crates carry their own empty `[workspace]` table** rather than a root
`exclude`, so the root manifest is untouched and the spike stays throwaway code
outside the gate set, which `/spike` step 2 permits and this story records.

**5. A `docs/SOURCE-POLICY.md` row is added for each surviving candidate at
completion, not now.** The row records a decision, and the decision is what the
spike produces. The plan's candidate table with its verified licences is the
input to that row.

**6. If A1 fails, the OpenJPH fallback is priced in an S04 story, not here.**
The gate's job is the answer. Appendix A already states the consequence, "you
maintain C codec builds regardless", and turning that into a plan is a story
with its own estimate rather than three unbudgeted weeks bolted onto this one.

**7. No `docs/lld/` file.** `codecs.md` belongs to E2.6, which is the story that
builds a codec rather than the one that decides which. `/complete-feature` step
9 updates `docs/lld/corpus.md` and `docs/lld/oracle.md` only.

**8. The `.203` anchor limitation is recorded, and no corpus row is added.**
`.201` and `.202` have three independent parties through the reference ramp,
`.203` has only openjp2 against OpenJPH and OpenJPH also encoded it. Say so in
the answer file as a stated limit on the strength of that row's evidence. Adding
a case from a second encoder is a corpus story with a licence and a manifest row.

**9. The JPEG-LS correctness axis is worded to state its anchor.** `pyjpegls`
encoded and `dcmdjpls` decodes, and both wrap CharLS, so a CharLS-versus-CharLS
agreement is not independent. The strong anchors are the uncompressed reference
for `.80` and ISO/IEC 14495-1's NEAR bound for `.81`, and the answer file says
which claim rests on which.

**10. `tools/spikes/out/` is refused by path in
`scripts/staged_content_check.py`, and `.gitignore` alone is not enough**,
because `git add -f` exists and a raw codestream extracted from a corpus row is
derived from patient data. **F-X009 lands last and absorbs this prefix** into
its declared-constant ratchet, exactly as it absorbs F-011's.

**11. The A1 and A2 comparator is throwaway and stays in
`tools/spikes/common/`.** It is never imported from `crates/` or
`tools/oracle/`, and its header says so. F-011's comparator carries HLD 25.1's
tolerance policy over rendered frames. This one is exact equality over a decoded
buffer. The sprint must not end with two general comparators, and the boundary
is that one of them is deleted when the gates are answered.

**12. The split with F-004 and F-005 is confirmed.** F-005 owns `CodecError`,
F-004 owns runtime SIMD128 detection, and this story names raw decoder error
codes and reports two build-side SIMD digests without defining either. F-004's
`WebAssembly.validate` probe and this story's two builds are the runtime and
build halves of one question, and the answer file cross-references F-004's
record rather than restating it.
