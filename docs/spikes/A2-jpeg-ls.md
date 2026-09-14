# A2, what is the JPEG-LS answer?

**Gate**: `docs/hld/A-spike-gates.md`, Appendix A, A2.
**Answered by**: F-X006, from the approved plan `.claude/plans/F-X006-design.md`.
**Status**: **RESOLVED. Outcome `Pure Rust`.**

**Every "E2.6" below means E4.6, F-028**, which is the JPEG-LS production
story. E2.6 is F-014, the quirk-capture workflow. F-028 adopted the named
fallback rather than the recommendation, on evidence this file marks
`NOT MEASURED`, and its measurements are unchanged. See `docs/lld/codecs.md`.

**2026-09-14, F-030: the multi-component corpus row this file recorded as owed
now exists.** Condition 2 of the recommendation said "A multi-component JPEG-LS
corpus row is owed, and it is a corpus story with a manifest row rather than
something this gate adds", and the coverage section said the gap was real and
the corpus could not close it. `syntax/jpegls_lossless_rgb8.dcm` closes the
corpus half: `Nf = 3`, `ILV = 2`, encoded by the same `pyjpegls` 1.5.1 this gate
used, and `crates/ocelli-codec/tests/jpegls.rs` asserts the adapter refuses it
cleanly. **The measurements below are not revised and the `NOT MEASURED` rows
stay as written**, because they record what was known when the gate ran and a
spike record that is edited to match later work stops being evidence. What
changed is coverage, not a measurement: multi-component **decoding** is still
unmeasured and is still a codec story. See `docs/lld/codecs.md`.

`docs/hld/A-spike-gates.md`, Appendix A, the row, transcribed character for
character, em-dash included. It sits in a table because
`scripts/prose_check.py` relaxes the voice rules inside a table row, which is
what lets an author's text be quoted exactly.

| **Gate** | **Question** | **Consequence if it fails** |
|----|----|----|
| A2 | What is the JPEG-LS answer — CharLS bridge, self-compiled CharLS, or a young pure-Rust crate? | Changes the architecture, not just a dependency line. Decide before anything else |

**`pure_jpegls` 2.0.0.** It decodes both JPEG-LS corpus rows byte-identically to
DCMTK 3.7.0 and to the uncompressed reference, stays inside ISO/IEC 14495-1's
`NEAR` bound, builds and runs on `wasm32-unknown-unknown` with and without
SIMD128 and on the native host, is MIT OR Apache-2.0, has exactly one
dependency, and is not a port of a read-blocked source. **One implementation on
every target.** The architecture change the gate warned about does not happen.

It carries one real limit and it is not hidden: it is **single-component only**,
so a multi-component JPEG-LS frame is refused. See **Coverage**.

---

## What the outcomes mean

Written before the work, per `/spike` step 1. Transcribed from
`.claude/plans/F-X006-design.md`, approved and committed before any measurement
in this file was taken. A2 is a decision rather than a pass or a fail, so its
table names what gets built.

| Outcome | Meaning |
|---------|---------|
| **Pure Rust** | A pure-Rust crate decodes both `.80` and `.81` bit-exact against `dcmdjpls`, reproduces `R` exactly for `.80`, builds for `wasm32-unknown-unknown` and for native, is permissively licensed and is not a port of a read-blocked source. One implementation on every target. Recommend it by name and version, record the maintenance risk in dates and download counts, and record the size delta |
| **CharLS, one route** | No pure-Rust crate qualifies and exactly one CharLS route serves every target. Register that one, record the toolchain it costs, and record what it does to the R5 audit surface |
| **CharLS, split** | No pure-Rust crate qualifies and no single route serves every target, so the browser gets R1 and native gets R2. **This is the outcome the gate means by "changes the architecture, not just a dependency line."** Allowed only with both decodes measured byte-identical on both rows and a standing test that keeps them so, and it stops for the operator per `/spike` step 5 |
| **Blocked** | No route decodes both rows correctly on any target. JPEG-LS reports UNAVAILABLE, which is section 31's rule generalised and is what deviation D-07 already requires of a feature that cannot run. Appendix B loses a transfer syntax from the parity surface, and that is a product decision for the operator, not an engineering one |

**The first row is satisfied in full**, with the coverage caveat recorded below
rather than folded into the verdict.

---

## The candidates, and their licences

`docs/SOURCE-POLICY.md`'s three questions were answered for every candidate.
Question 1 was checked against the repository rather than the published crate,
because a `.crate` archive frequently omits a licence file that the repository
carries. **None is copyleft, so none is read-blocked**, and the read-blocked
projects the policy names are not candidates and were not opened.

| Route | Crate or package | Version | Licence, metadata | LICENCE file at the repository root | Read-blocked |
|-------|------------------|---------|-------------------|-------------------------------------|--------------|
| **R1** | `@cornerstonejs/codec-charls` | 1.2.5 | MIT (package), BSD-3 CharLS inside | `chafey/charls-js`, `master/LICENSE`, MIT | no |
| **R2** | `charls` over `charls-sys` | 0.4.2 / 2.4.5 | MIT, BSD-3 CharLS vendored | `coradoya/charls-rs` and `coradoya/charls-sys`, `main/LICENSE.md` | no |
| **R3** | `pure_jpegls` | 2.0.0 | MIT OR Apache-2.0 | `jpfielding/dicos.rs`, `main/LICENSE-MIT` and `main/LICENSE-APACHE` | no |
| **R3b** | `dicom-toolkit-codec` | 0.5.0 | MIT OR Apache-2.0 | `knopkem/dicom-toolkit-rs`, `main/LICENSE-MIT`, `main/LICENSE-APACHE`, `main/NOTICE` | no |
| **R3b** | `ritk-codecs` | 0.6.0 | MIT OR Apache-2.0 | `ryancinsight/ritk`, `main/LICENSE-MIT` and `main/LICENSE-APACHE` | no |
| **R4** | `dicom-transfer-syntax-registry` | 0.10.0 | MIT OR Apache-2.0 | `Enet4/dicom-rs`, already in the policy table | no |

**R4 collapses into R2 at the bottom, and that is the finding rather than a
footnote.** `dicom-transfer-syntax-registry` 0.10.0 declares, verbatim:

```toml
# jpeg LS support via charls bindings
charls = ["dep:charls"]

# use vcpkg to build CharLS
charls-vcpkg = ["charls?/vcpkg"]
```

`dep:charls` is `charls` 0.4.2, which is `charls-sys` 2.4.5, which is the C++
library. **dicom-rs has no pure-Rust JPEG-LS route**, which is section 21's
sentence "JPEG-LS has no credible pure-Rust path" still being true inside the
library the HLD selects. Section 21's note is a fact about when the HLD was
written and this gate is the re-check, and outside dicom-rs the sentence is no
longer true.

### The provenance question this project needs specifically

Not the licence field, which proves nothing about a derivative of blocked
source, but what each candidate claims to be a port of. Each answer is from the
candidate's own README, NOTICE or module header.

| Candidate | Claims to be | In `docs/SOURCE-POLICY.md`'s yes column |
|-----------|-------------|----------------------------------------|
| `pure_jpegls` 2.0.0 | "Pure Rust implementation of JPEG-LS (ITU-T T.87 / ISO/IEC 14495-1)", "Verified against CharLS fixtures". A standard implementation, tested against CharLS output, not a port of it | CharLS, yes. The standard is not a source |
| `dicom-toolkit-codec` 0.5.0 | Its NOTICE: "an independent, clean-room Rust port inspired by" DCMTK and CharLS, "implemented by studying the ISO/IEC 14495-1 standard and referencing CharLS's algorithmic approach", "no C++ source was translated line-by-line" | DCMTK and CharLS, both yes |
| `ritk-codecs` 0.6.0 | "RITK-native", "JPEG-LS (none - RITK-native since Sprint 127)", and its crate header states `openjpeg-sys` / `openjp2` / `jpeg2k` / `charls` are no longer dependencies | Names no blocked source |
| `openjph-core` 0.1.0, a transitive dependency of `dicom-toolkit-codec` through `dicom-toolkit-jpeg2000` | "a faithful port of the OpenJPH C++ library (v0.26.3)" | OpenJPH, yes. **But crates.io reports no repository URL for it, so question 1 cannot be answered from a repository and only question 2 can.** Flagged, and it does not reach the recommended route |

**No candidate names a read-blocked project as a source.** Every claim above
was read from the candidate's own text and not inferred from a `license` field.

---

## Method

Same rig as A1, same canonical form, same comparator.

**The canonical form**: 12288 bytes, little-endian `u16`, 6144 samples, 64 rows
by 96 columns, from `scripts/corpus_synth.py`. Both JPEG-LS rows were encoded
from `syntax/explicit_vr_le.dcm` by `pyjpegls` 1.5.1, with
`JPEG_LS_NEAR = 3` for `.81`.

**The comparator** is `tools/spikes/common/compare.mjs`, exact equality with no
tolerance, and it was observed red by two separate mutations before any digest
in this file was believed. The mutation runs and their exit codes are recorded
in `docs/spikes/A1-htj2k-openjp2.md`, which uses the same comparator.

### The anchors, and which claim rests on which

**The DCMTK anchor is CharLS on both sides and this is stated rather than
implied.** `pyjpegls` 1.5.1 encoded these two rows and it wraps CharLS.
`dcmdjpls` is DCMTK 3.7.0 and it also uses CharLS. **So a candidate agreeing
with `dcmdjpls` has agreed with a second reading of the same implementation and
not with an independent one.** That comparison is still worth running, because
a disagreement would be decisive, and an agreement is weaker evidence than it
looks.

The two anchors that are independent:

| Row | Independent anchor | Why it is independent |
|-----|-------------------|----------------------|
| `1.2.840.10008.1.2.4.80` | `R`, the uncompressed `syntax/explicit_vr_le.dcm` PixelData | The syntax is lossless so `R` is exact, and `R` comes from `scripts/corpus_synth.py`'s hand-predictable ramp rather than from any codec. `scripts/tests/test_corpus_synth.py::test_lossless_syntaxes_round_trip_exactly` already asserts the round trip through a third decoder |
| `1.2.840.10008.1.2.4.81` | ISO/IEC 14495-1's guarantee that the maximum absolute error is at most `NEAR` | It is the standard rather than an implementation. `scripts/corpus_synth.py` sets `JPEG_LS_NEAR = 3` and `scripts/tests/test_corpus_synth.py::test_jpeg_ls_near_lossless_holds_its_declared_near_bound` already asserts it against the reference |

**A candidate that satisfies the NEAR bound but disagrees with `dcmdjpls` on
`.81` has produced a legal but different image, and for us that is a defect
rather than a pass**, because near-lossless decoding is deterministic given the
codestream. Both criteria are therefore reported separately below rather than
merged.

### The harness

`tools/spikes/a2-jpeg-ls/`, the same shape as A1's: a `cdylib` with a
hand-written integer ABI, no `wasm-bindgen`, no `unsafe`, `edition = "2021"`
for the `#[no_mangle]` reason, its own empty `[workspace]` table, and errors
leaving as raw integers because `CodecError` is F-005's to shape.

**One feature per candidate**, so axis 1 is measurable per candidate rather
than as one all-or-nothing answer. A candidate whose dependency tree refuses
`wasm32-unknown-unknown` would otherwise make the whole harness unbuildable and
hide the other two, which is exactly what happened.

**R2 is not a dependency of the harness.** It builds a vendored C++ CharLS
through `cmake`, and depending on it would have made every other candidate
unmeasurable on this machine. Its build is measured separately below.

---

## The six axes

### 1. Does it build for `wasm32-unknown-unknown`?

| Route | Builds | The exact result |
|-------|--------|------------------|
| **R1** `@cornerstonejs/codec-charls` 1.2.5 | n/a | It ships a prebuilt emscripten module, `dist/charlswasm_decode.wasm`, 145306 bytes. There is nothing to compile |
| **R2** `charls` 0.4.2 | **no**, and it does not build natively here either | Native: `is `cmake` not installed?` from `cmake-0.1.58`, build script failed. wasm32: `link-cplusplus@1.0.12: error: unable to create target: 'No available targets are compatible with triple "wasm32-unknown-unknown"'` |
| **R3** `pure_jpegls` 2.0.0 | **yes**, plain and `+simd128` | exit 0 both ways |
| **R3b** `dicom-toolkit-codec` 0.5.0 | **no** | `error: to use `uuid` on `wasm32-unknown-unknown`, specify a source of randomness using one of the `js`, `rng-getrandom`, or `rng-rand` features`, from `uuid-1.26.0/src/rng.rs:104`, reached through `dicom-toolkit-core` |
| **R3b** `ritk-codecs` 0.6.0 | **yes**, plain and `+simd128` | exit 0 both ways |
| **R4** | see R2 | `dicom-transfer-syntax-registry`'s `charls` feature is `dep:charls`, so its answer is R2's answer |

**`dicom-toolkit-codec` has a second wasm problem that would be a D2 violation
and is worth naming, because it is invisible until someone tries the obvious
fix.** The `uuid` error above has a documented remedy, which is to enable one of
`js`, `rng-getrandom` or `rng-rand`. **The `js` feature pulls `wasm-bindgen`
into the dependency graph**, and `wasm-bindgen` in a crate that is not
`ocelli-wasm` is exactly what decision D2 and deviation D-12 forbid. It is
already visible as an unactivated optional entry in this harness's own
`tools/spikes/a2-jpeg-ls/Cargo.lock`, at **0.2.128**, which is also not the
`=0.2.127` the workspace pins. Nothing is violated today, because the spike is
its own workspace and `ci/check-bindgen-isolation.sh` scopes to `crates/*/`,
and it would be violated the moment this candidate was adopted and made to
build. It is out on correctness anyway.

**R2's toolchain absence is a fact about the route and not a local
inconvenience.** `cmake` and `emcc` are both absent from this machine, and
`link-cplusplus` reports that the host C++ compiler has no
`wasm32-unknown-unknown` target at all, which is a property of the toolchain
rather than of what is installed. Building CharLS for the browser needs
emscripten, which is what `@cornerstonejs/codec-charls` already did.

### 2. Correctness

Every digest below is over the canonical 12288 bytes.

```
R (syntax/explicit_vr_le.dcm PixelData): b20a1ef346d9742bcbd38db9174ae5a4775531df434e50d7e209c11b67a27609

--- jpegls_lossless, 1.2.840.10008.1.2.4.80, lossless ---
  R1 frame info: {"width":96,"height":64,"bitsPerSample":16,"componentCount":1}
  dicom_toolkit/native      UNAVAILABLE: DecompressionError { reason: "JPEG-LS: unexpected end of bitstream" }
  dicom_toolkit/wasm        NOT BUILT: does not compile for wasm32-unknown-unknown
  A_dcmtk                  b20a1ef346d9742bcbd38db9174ae5a4775531df434e50d7e209c11b67a27609
  R1_charls_js             b20a1ef346d9742bcbd38db9174ae5a4775531df434e50d7e209c11b67a27609
  pure_jpegls/native       b20a1ef346d9742bcbd38db9174ae5a4775531df434e50d7e209c11b67a27609
  pure_jpegls/wasm         b20a1ef346d9742bcbd38db9174ae5a4775531df434e50d7e209c11b67a27609
  pure_jpegls/simd         b20a1ef346d9742bcbd38db9174ae5a4775531df434e50d7e209c11b67a27609
  ritk_codecs/native       b20a1ef346d9742bcbd38db9174ae5a4775531df434e50d7e209c11b67a27609
  ritk_codecs/wasm         b20a1ef346d9742bcbd38db9174ae5a4775531df434e50d7e209c11b67a27609
  ritk_codecs/simd         b20a1ef346d9742bcbd38db9174ae5a4775531df434e50d7e209c11b67a27609
  every route EQUAL to A_dcmtk, and every route EQUAL to R

--- jpegls_near_lossless, 1.2.840.10008.1.2.4.81, near-lossless NEAR 3 ---
  dicom_toolkit/native      UNAVAILABLE: DecompressionError { reason: "JPEG-LS: unexpected end of bitstream" }
  dicom_toolkit/wasm        NOT BUILT: does not compile for wasm32-unknown-unknown
  A_dcmtk                  1353c4956c8907e084eaa1c4c546f794acbc5cbc1c4b65c945298a5ac4a5b2e5
  R1_charls_js             1353c4956c8907e084eaa1c4c546f794acbc5cbc1c4b65c945298a5ac4a5b2e5
  pure_jpegls/native       1353c4956c8907e084eaa1c4c546f794acbc5cbc1c4b65c945298a5ac4a5b2e5
  pure_jpegls/wasm         1353c4956c8907e084eaa1c4c546f794acbc5cbc1c4b65c945298a5ac4a5b2e5
  pure_jpegls/simd         1353c4956c8907e084eaa1c4c546f794acbc5cbc1c4b65c945298a5ac4a5b2e5
  ritk_codecs/native       1353c4956c8907e084eaa1c4c546f794acbc5cbc1c4b65c945298a5ac4a5b2e5
  ritk_codecs/wasm         1353c4956c8907e084eaa1c4c546f794acbc5cbc1c4b65c945298a5ac4a5b2e5
  ritk_codecs/simd         1353c4956c8907e084eaa1c4c546f794acbc5cbc1c4b65c945298a5ac4a5b2e5
  every route EQUAL to A_dcmtk

  -- against ISO/IEC 14495-1's NEAR bound, max abs error at most 3 --
  WITHIN   A_dcmtk                  max abs error 3, 0 sample(s) over 3
  WITHIN   R1_charls_js             max abs error 3, 0 sample(s) over 3
  WITHIN   pure_jpegls/native       max abs error 3, 0 sample(s) over 3
  WITHIN   pure_jpegls/wasm         max abs error 3, 0 sample(s) over 3
  WITHIN   pure_jpegls/simd         max abs error 3, 0 sample(s) over 3
  WITHIN   ritk_codecs/native       max abs error 3, 0 sample(s) over 3
  WITHIN   ritk_codecs/wasm         max abs error 3, 0 sample(s) over 3
  WITHIN   ritk_codecs/simd         max abs error 3, 0 sample(s) over 3

0 failing comparison(s), 2 route(s) unavailable
```

Three things worth reading twice.

**`pure_jpegls` and `ritk-codecs` reproduce `R` exactly for `.80`.** That is
the encoder-independent claim, and it is the strongest correctness statement in
this record. It does not depend on CharLS at all.

**Every surviving route lands on the same digest for `.81` and every one is
inside the NEAR bound.** Maximum absolute error is exactly 3 against `R`, with
zero samples over 3, which is the bound holding at its limit rather than
comfortably inside it. Since near-lossless decode is deterministic given the
codestream, agreement here is the expected result and a disagreement would have
been a defect.

**wasm and native produce identical digests for both surviving Rust
candidates, in both SIMD configurations.** That is the property A1 could not
test at all, and it is the one that makes a single implementation across targets
credible.

**`dicom-toolkit-codec` 0.5.0 decodes neither row.** It reports "JPEG-LS:
unexpected end of bitstream" on both, on a codestream that four other decoders
read without complaint. It is out on correctness before its wasm failure is
even reached.

### 3. Coverage

| Requirement | `pure_jpegls` 2.0.0 | `ritk-codecs` 0.6.0 | R1 charls-js |
|-------------|--------------------|--------------------|--------------|
| `1.2.840.10008.1.2.4.80` | **measured, exact** | **measured, exact** | **measured, exact** |
| `1.2.840.10008.1.2.4.81`, `NEAR > 0` | **measured, exact and within bound** | **measured, exact and within bound** | **measured, exact and within bound** |
| 16-bit samples | **measured**, `bitsPerSample` 16 on every route | **measured** | **measured** |
| Multi-component, `Nf != 1` | **REFUSED by the crate, and NOT MEASURED here.** Its own header: conformant "for **single-component** images (`Nf = 1`, `ILV = 0`)", and `Nf != 1` and `ILV != 0` are "rejected as `Unsupported`" | Its module has `ComponentInfo` and `InterleaveMode` and parses `SOF55`, `LSE`, `DRI`, `DNL`. **NOT MEASURED here** | CharLS handles it. **NOT MEASURED here** |
| `DRI` and restart markers | **REFUSED by the crate**, "rejected as `Unsupported`". **NOT MEASURED** | parses `DRI`. **NOT MEASURED** | **NOT MEASURED** |

**The multi-component gap is real and the corpus cannot close it.** There is no
multi-component JPEG-LS row in `corpus/manifest.tsv`, so nothing here measures
it, and every entry above that is not measured says so rather than being
inferred from a README. **A DICOM JPEG-LS frame can be RGB**, so this is a
coverage hole and not a curiosity, and it is the single reason the
recommendation below carries a condition.

### 4. Binary size

Release wasm, `opt-level = "z"`, `lto = "fat"`, `codegen-units = 1`,
`strip = true`, which is HLD 15.2's profile. The baseline is the same harness
with no candidate compiled in, so the delta is the codec and nothing else. The
baseline includes both embedded codestreams, 2406 bytes.

| Build | Bytes | Delta over baseline |
|-------|-------|--------------------|
| baseline, no candidate | 18531 | 0 |
| `pure_jpegls`, plain | 59072 | **40541** |
| `pure_jpegls`, `+simd128` | 58649 | **40118** |
| `ritk-codecs`, plain | 142963 | **124432** |
| `ritk-codecs`, `+simd128` | 142716 | **124185** |
| R1, `@cornerstonejs/codec-charls` `charlswasm_decode.wasm` | 145306 | n/a, a separate module |

`ci/wasm-size-budget.json` is the authority on the recorded module size and its
0.05 tolerance, which is gate A4's currency. **Read the file rather than a
number quoted here.** It read 16388 bytes at commit `739ba11`, where F-005
rebaselined it from 14104. **`pure_jpegls` is roughly 40 KB, and
`ritk-codecs` is roughly three times that**, partly because it also carries a
JPEG 2000 decoder and pulls `jpeg-decoder` with `rayon`. R1 is a separate 145 KB
module with a separate linear memory and is not comparable to a delta.

**Nothing in this story changes `ci/wasm-size-budget.json`.** The budget moves
when E2.6 registers a decoder in `crates/ocelli-codec`, and this figure is what
that story should expect.

### 5. Provenance

Answered in **The candidates, and their licences** above. Every repository
resolves and every one carries a licence file at its root. The one flag is
`openjph-core` 0.1.0, which reports no repository URL, and it is a transitive
dependency of a candidate that is out on other grounds.

**A `docs/SOURCE-POLICY.md` "Extensions to the table" row is owed for
`pure_jpegls` 2.0.0 with a decided date. Editing that normative file is the
operator's**, per the design round's decision 5, and this record is the input
to it.

### 6. Maintenance risk, in dates and counts rather than adjectives

| Crate | First published | Latest | Total downloads | Recent | Direct dependencies |
|-------|----------------|--------|-----------------|--------|--------------------|
| `pure_jpegls` | **2026-06-01** | 2026-07-20 (2.0.0) | 407 | 393 | **one**, `thiserror` |
| `ritk-codecs` | 2026-08-03 | 2026-08-11 (0.6.0) | 112 | 112 | `anyhow`, `jpeg-decoder` (which pulls `rayon`) |
| `dicom-toolkit-codec` | 2026-03-15 | 2026-03-20 (0.5.0) | 760 | 349 | 9 direct, including `inventory`, which HLD 15.2 states does not work on wasm at all. `uuid`, `sha2`, `encoding_rs` and `openjph-core` arrive transitively |
| `charls` | 2024-07-29 | 2025-04-24 (0.4.2) | 22370 | 6215 | `charls-sys` and a C++ toolchain |
| `@cornerstonejs/codec-charls` | n/a | 1.2.5 | n/a | n/a | a prebuilt emscripten module |

**`pure_jpegls` is the "young pure-Rust crate" A2 names, literally.** Three
months old at the date of this record, two releases, 407 downloads. That is a
real risk and it is stated in dates rather than softened.

Three things reduce it and they are facts rather than reassurance. It has
**one** dependency, and that dependency is `thiserror`, which the workspace
already declares. Its entire subject matter is one standard, ISO/IEC 14495-1,
which does not move. And its correctness on the two rows that matter is
measured above against an encoder-independent anchor, so adopting it is not a
bet on the maintainer, it is a bet that a vendored or forked copy would remain
correct if the crate stopped moving.

---

## The architecture consequences

These are reasoning, and they are labelled as such, sitting beside the
measurements rather than mixed into them. They are why the outcome is `Pure
Rust` and not `CharLS, split`, given that R1 also decoded both rows exactly.

**R1's real problem is not a measurement.** There is no JavaScript on the
native desktop and server targets, so R1 means **those targets have no JPEG-LS
decoder at all**. D2's stated consequence is "Phases 2 and 3 are entry points,
not rewrites", and a transfer syntax that exists only in the browser is a hole
in exactly that claim. This is the strongest single argument in A2 and it is an
architecture argument.

**R1 would also put a second wasm module and a second linear memory inside the
worker**, with decoded pixels copied out of that module's memory into ours once
per frame. **That is not a D3 violation**, because D3 governs the worker to
main-thread boundary and both modules live worker-side. It is a per-frame copy
the pure-Rust routes do not have, and it is a cost rather than a defect. The
harness performs exactly that copy in `charlsDecode`, so the cost is real and
was exercised.

**R2 is the reverse hole**, the natural native answer and the difficult browser
one, and on this machine the impossible browser one.

**A split answer would have been the outcome that costs the most under D14**,
because two implementations of one decode on two targets is precisely the
measured-divergence surface D14 commits to publishing. **The measurements would
have permitted it**, since R1 and R2's shared CharLS lineage and `pure_jpegls`
all land on the same digest. It is not recommended, because one implementation
that is measured exact on both targets is strictly better than two that are
measured equal today, and because a split would have required a standing test
keeping them equal forever.

**Tier A, B and C are all n/a here, and that is load-bearing rather than
empty.** Decode is CPU work in a worker that completes before anything reaches
a device, and the resolved tier does not select a decoder. **There must never
be a tier-gated codec path**, which is section 18's rule for the LUT chain
generalised, with the added property that a second decode behind a tier check
would only ever run on hardware nobody develops on. The axis that matters in
this story is the target, browser against native, and that is why the three
tier rows are n/a.

**Section 21's "Must not allocate", read narrowly** per the design round's
decision 2: no allocation per decode call, output buffer caller-provided,
internal state at registration or first use outside the rule. `pure_jpegls`'s
current API is `decode(data, width, height) -> Result<(Vec<u16>, u32, u32)>`,
which **returns a `Vec` per call and therefore does not meet section 21's
signature as written.** The adapter E2.6 writes has to decode into the
caller-provided buffer, and with this crate that means either an upstream API
addition or one copy per frame out of the returned `Vec`. **That is the one
concrete integration cost of this recommendation and it is named here so E2.6
budgets it.**

**`ritk-codecs` has a second and larger integration cost**, which is why it is
the runner-up rather than the recommendation despite being equally correct. Its
API is `decode_jpeg_ls_fragment(fragment, layout) -> Result<Vec<f32>>` and the
layout carries `rescale_slope` and `rescale_intercept`, so **the codec applies
the modality rescale**. HLD section 18 requires that arithmetic to exist exactly
once, in `ocelli-pixel`. Using this crate means either passing slope 1 and
intercept 0 and converting `f32` back to integers, which is what the harness
does, or accepting a second place where the LUT chain lives. The first is
wasteful and the second is the defect section 18 exists to prevent.

**The one `as` cast in this harness, named so a reviewer knows where to look
under HLD 27.3.** `tools/spikes/a2-jpeg-ls/src/lib.rs`, `f32_sample_to_u16`,
`Ok(truncated as u16)`. It exists only because `ritk-codecs` returns `f32`. The
value is proved on the two preceding lines to lie in `0.0..=65535.0` and to
equal its own `trunc()`, so the conversion is exact rather than rounding or
saturating. The equality against `trunc()` is an integrality test and not an
approximate float comparison, which is the one shape where comparing floats is
the correct tool. **The recommended route has no cast at all**, because
`pure_jpegls` returns `u16` samples directly, and the A1 harness has none
either.

**What R5's audit-surface claim covers.** `scripts/unsafe_allowlist_check.py`
reads `git ls-files` and never sees a dependency. **The recommended route is
the only one where that does not matter**: `pure_jpegls` is pure Rust with one
pure-Rust dependency, so adopting it adds no `extern "C"` surface at all. R2
and any `charls-sys` route are `extern "C"` and unsafe by construction, and
choosing one would leave R5 passing mechanically while its stated purpose was
weakened. That sentence is here because it would otherwise be discovered by a
device-submission reviewer.

---

## Recommendation

**Adopt `pure_jpegls` 2.0.0 for `1.2.840.10008.1.2.4.80` and
`1.2.840.10008.1.2.4.81`, on every target, with three conditions.**

1. **E2.6 pins the exact version** and carries a standing conformance test over
   both corpus rows against `R` and against the NEAR bound, not against
   `dcmdjpls`, because the DCMTK anchor is a second CharLS reading rather than
   an independent one.
2. **Multi-component JPEG-LS reports unavailable until it is measured.** The
   crate refuses `Nf != 1` explicitly, so the failure is a clean refusal rather
   than a wrong pixel, which is section 31's rule and deviation D-07's rule
   already satisfied by the crate's own behaviour. **A multi-component JPEG-LS
   corpus row is owed**, and it is a corpus story with a manifest row rather
   than something this gate adds.
3. **The `Vec`-returning API is adapted, not adopted.** Section 21's `decode`
   takes a caller-provided `out`, and the cost of bridging is one copy per
   frame or an upstream API addition. E2.6 decides which and records it.

**Keep `ritk-codecs` 0.6.0 as the named fallback.** It is equally correct on
both rows, builds for both targets, and appears to handle multi-component. Its
costs are three times the binary size and a rescale-applying API that section 18
does not want. If `pure_jpegls`'s multi-component gap turns out to block a real
case before the crate closes it, this is where to look next.

**Do not adopt `dicom-toolkit-codec` 0.5.0.** It decodes neither row and does
not build for the target that ships.

**Do not adopt R1 or R2.** Both are correct and neither serves every target,
and one implementation everywhere is available.

---

## What changes in the plan

**Nothing stops for the operator on A2.** The architecture change the gate
warned about does not happen, and the answer costs a dependency line rather
than a redesign.

| Item | Before | After |
|------|--------|-------|
| JPEG-LS route | open, three candidates, possibly a split | **`pure_jpegls` 2.0.0, one implementation on every target** |
| `docs/hld/12-workspace-and-build.md` 15.2's dependency list | names no JPEG-LS feature, which was the specification's own statement of A2 | A pure-Rust crate is added by E2.6. The absence in 15.2 is now explained rather than open |
| Section 21's note, "JPEG-LS has no credible pure-Rust path" | true when written | **Still true inside dicom-rs**, whose `charls` feature is `dep:charls` over the C++ library. No longer true outside it |
| `ci/wasm-size-budget.json` | the file is the authority, 16388 bytes read at commit `739ba11` | Unchanged by this story. E2.6 should expect roughly 40 KB |
| `docs/SOURCE-POLICY.md` "Extensions to the table" | no codec rows | **A `pure_jpegls` 2.0.0 row is owed, and that edit is the operator's** |
| Multi-component JPEG-LS | not considered | **A corpus gap, and a story** |

---

## Reproducing this

| Thing | Version |
|-------|---------|
| rustc, cargo | 1.97.1 (`8bab26f4f 2026-07-14`), cargo 1.97.1 |
| target | `wasm32-unknown-unknown` |
| node | v24.16.0 |
| `pure_jpegls` | 2.0.0, MIT OR Apache-2.0 |
| `ritk-codecs` | 0.6.0, MIT OR Apache-2.0 |
| `dicom-toolkit-codec` | 0.5.0, MIT OR Apache-2.0 |
| `@cornerstonejs/codec-charls` | 1.2.5, MIT, the same pin as `tools/oracle/package.json` |
| DCMTK | `dcmdjpls` v3.7.0 2025-12-15 |
| encoder of both rows | `pyjpegls` 1.5.1, `scripts/corpus_synth.py`, `JPEG_LS_NEAR = 3` |
| corpus manifest | sha256 `a6151d3c289a6938dec98f6cd8f42231e1daeb7fefb72fb7455e68e77618bf49` |
| corpus rows | manifest lines 44 and 45, and 33 for `R` |

```bash
uv run tools/spikes/common/extract.py
uv run tools/spikes/a2-jpeg-ls/anchors.py
node tools/spikes/common/tests/compare_test.mjs
cd tools/spikes/a2-jpeg-ls
npm install
cargo build --release
for f in pure-jpegls ritk; do
  cargo build --release --lib --target wasm32-unknown-unknown \
    --no-default-features --features "$f" --target-dir "target/wasm-$f"
  RUSTFLAGS="-C target-feature=+simd128" cargo build --release --lib \
    --target wasm32-unknown-unknown --no-default-features --features "$f" \
    --target-dir "target/wasm-$f-simd"
done
# The size baseline: the same harness with no candidate compiled in.
cargo build --release --lib --target wasm32-unknown-unknown \
  --no-default-features --target-dir target/wasm-baseline
node run.mjs
```

R2 is measured outside this harness, because depending on it would make the
others unbuildable here. In a scratch crate with the same
`rust-toolchain.toml`:

```bash
cargo add charls@=0.4.2
cargo build                                  # the cmake failure
cargo build --target wasm32-unknown-unknown  # the link-cplusplus failure
```

**The harness is throwaway and is not held to the gate set**, per `/spike`
step 2, and it is deleted when this gate and A1 are closed. **F-028 closed A2's
production question and removed `tools/spikes/a2-jpeg-ls/`**, so the commands
above describe a rig that existed rather than one a reader will find. Its
measurements are unaffected and stay here, which is the reason this file
survives the harness. The paragraph below is left as it was written, describing
what the harness was held to while it was present. The gates that do
reach it are `unsafe`, `provenance` and `content`, which read `git ls-files`,
and `lint`, which reaches `run.mjs` through the `tools/spikes/**/*.mjs` block
in `eslint.config.js`. It passes all four. `clippy`, `test`, `bindgen` and
`nostd` are scoped to the workspace or to `crates/` and never see it.

**`prose` does not reach this directory**, which earlier revisions of this file
claimed it did. `scripts/prose_check.py` includes a path only if it is in
`INCLUDE_EXACT` or it ends in `.md` and starts with one of `INCLUDE_PREFIXES`.
`tools/spikes/` is on neither list and holds no `.md`. **This file is a
different matter**: `docs/spikes/` is a prefix, so the answer files are covered
and the harness sources are not. Nothing under
`tools/spikes/out/` and nothing under `tools/spikes/a2-jpeg-ls/node_modules/`
is committed.
