# F-240, Matrix and tone-curve ICC profiles in ocelli-pixel stage 4

**Status**: draft
**Epic ref**: E38.38
**Sprint**: S46
**Estimate**: 4w

Story row, `docs/sprints/BACKLOG.md`: "Matrix and tone-curve ICC profiles applied in ocelli-pixel stage 4 for colour frames, lookup-table profiles refused by name until F-145". Layer All, depends on F-239, milestone M20.

`allocation.json` notes: "Filed by the operator on 2026-11-07 with the answers to F-239's questions. F-239 draws a frame carrying ICC Profile (0028,2000) with a sentence that the profile was not applied, and this story applies the matrix and tone-curve subset. Scheduled right after F-239. A subset of F-145, which keeps its own row and its recorded E4.9 defect."

This plan is a draft with open questions for the operator. Every path and line below was read on `sprint/s46` at 5451cb7d.

## Normative source, transcribed

### HLD section 18, `docs/hld/15-lut-chain.md:20` and `:27`

```text
It is specified in DICOM PS3.3 C.11 and the stages apply strictly in order. Implement it once, in ocelli-pixel, and let the shader read the parameters — do not let a second copy of this logic appear anywhere.

| 4\. Palette / ICC | Display → RGB | Palette colour LUT, or the display colour pipeline |
```

### HLD section 35, `docs/hld/26-differentiating-capabilities.md:136`

```text
- **ICC colour management is a correctness requirement.** A real pipeline stage after the LUT chain: scanner RGB → CIEXYZ → sRGB via the embedded profile. Radiology has windowing; pathology has colour management, and getting it wrong changes diagnoses.
```

### D-23, `docs/hld/DEVIATIONS.md:40`

"ICC is not implemented by F-030 and its route reports unavailable, so this row makes no claim about the `ICC` half of the stage name beyond the input type."

### What the HLD does not give

No section transcribes the ICC specification, names a profile subset, names the target colour space beyond "sRGB", or gives a fixture. `grep -n -i 'icc' docs/hld/*.md` finds only the two lines above and D-23. Everything below about the profile format is the lead's transcription of ICC.1 from memory of its structure, **not checked against the document**, which is not in the repository. AC12 makes the reviewer check every byte offset, tag type and formula against ICC.1:2022 (profile version 4.4) at color.org, and the v2 profile layout in ICC.1:2001-04, before a fixture is accepted. The same holds for IEC 61966-2-1's sRGB encoding.

### The profile subset, as the lead transcribes it

```text
header           128 bytes. Profile/device class at bytes 12..16, data colour
                 space at 16..20, PCS at 20..24, version at 8.
tag table        at 128: a u32 count, then per tag a signature, an offset and
                 a size, each u32 big-endian.
XYZType  'XYZ '  4 reserved bytes, then XYZ triples as s15Fixed16Number.
curveType 'curv' 4 reserved, u32 count n, then n u16 entries.
                 n = 0: identity.  n = 1: one u8Fixed8Number gamma, Y = X^g.
                 n >= 2: table over [0, 1], entries / 65535, linear
                 interpolation between neighbours.
parametricCurveType 'para'  4 reserved, u16 function type, 2 reserved, then
                 s15Fixed16 parameters:
                 0  g              Y = X^g
                 1  g a b          Y = (aX + b)^g  for X >= -b/a, else 0
                 2  g a b c        Y = (aX + b)^g + c  for X >= -b/a, else c
                 3  g a b c d      Y = (aX + b)^g  for X >= d, else cX
                 4  g a b c d e f  Y = (aX + b)^g + e  for X >= d, else cX + f
matrix/TRC model, three-component RGB to PCS XYZ:
                 [X Y Z]^T = [rXYZ gXYZ bXYZ] * [rTRC(R) gTRC(G) bTRC(B)]^T
                 with R, G and B the device values normalised to [0, 1].
PCS              CIE XYZ with the D50 illuminant. wtpt is the media white.
```

### The target, sRGB, as the lead transcribes IEC 61966-2-1

```text
encoding  V = 12.92 L                 for L <= 0.0031308
          V = 1.055 L^(1/2.4) - 0.055 otherwise
```

The PCS to linear sRGB matrix is the inverse of IEC 61966-2-1's RGB to XYZ matrix adapted from D65 to D50 by the linear Bradford transform of ICC.1's chromatic adaptation annex, computed once in `f64` from the published constants and rounded once to `f32`, as `color.rs` does for its YCbCr inverse (`crates/ocelli-pixel/src/color.rs:400` to `:446`).

## What the specification does not cover

1. **The subset.** Profile class `mntr` or `scnr`, data colour space `RGB `, PCS `XYZ `, with all six of `rXYZ`, `gXYZ`, `bXYZ`, `rTRC`, `gTRC` and `bTRC`, each TRC `curv` or `para` of types 0 to 4. Every other shape is refused by name: another class (`prtr`, `link`, `spac`, `abst`, `nmcl`), a `Lab ` PCS, a `GRAY` profile, a missing tag, and any profile carrying `A2B0`, `A2B1`, `A2B2`, `D2B0` or another lookup-table tag. That last one is open question 2, because ICC.1 has a profile with a lookup-table tag use it in preference to the matrix, so using the matrix would be a different answer from the one the profile asks for.
2. **The rendering intent.** The matrix/TRC model has one transform, so the header's intent is read and recorded and does not change the arithmetic.
3. **Media white and adaptation.** The colourants are taken as already adapted to D50, which ICC.1 v4 requires and which `chad` records. `wtpt` is read and not applied, so the conversion is media-relative colorimetric. Absolute colorimetric is a non-goal.
4. **Where it sits.** After the colour transform of stage 4 and in place of F-239's display scaling, before F-239's colour window. The device values are normalised by `2^n - 1`, with `n` Bits Stored for samples and the descriptor's third value for a palette, because ICC.1 normalises device values by the encoding range. The output is linear sRGB encoded and scaled by 255 as `f32`, so the window and the tier C byte rule are unchanged. A frame without a profile is untouched and keeps F-239's route byte for byte.
5. **One implementation.** The parser and the transform live in `ocelli-pixel`, which owns all pixel interpretation (`docs/lld/pixel-pipeline.md`). `ocelli-dicom` returns the element's bytes and nothing else.
6. **Size.** A profile above a named byte constant is refused by name. The implementer sets the constant from the largest profile in the corpus and real files it can read, and records it.
7. **Build or a crate.** Open question 1.

## Problem and reproducer

After F-239, `synthetic/sc_rgb_icc.dcm` draws in device RGB with "This image's ICC profile was not applied." A secondary capture or ultrasound file whose profile is not sRGB is shown with the wrong colours, and the sentence is the only sign. `ColourLut::icc` in `crates/ocelli-render/src/colour.rs:146` to `:155` always returns `Unavailable`. No ICC parser exists in the tree. `git grep -n -i 'icc_profile\|IccProfile\|2028,2000\|0028,2000' -- crates examples tools` finds none.

## Scope, API and compatibility

In scope:

- `ocelli-pixel` gains `IccProfile::parse(&[u8]) -> Result<IccProfile, IccError>` for the subset of item 1, and `IccTransform`, which maps `Rgb` device values to `Rgb` display values per item 4.
- `ColorTransform` accepts an optional `IccTransform` at resolve time, so the profile is decided once per held frame.
- `ocelli-dicom` gains `icc_profile(metadata: &MetadataSet) -> Option<&[u8]>`, beside F-239's `has_icc_profile`.
- The folder viewer applies the profile. F-239's sentence is removed for a profile this story applies, and a refused profile is handled as open question 3 decides.
- `IccError` names each refusal: `UnsupportedClass`, `UnsupportedColourSpace`, `UnsupportedPcs`, `LookupTableProfile`, `MissingTag`, `UnsupportedCurve`, `Malformed`, `TooLarge`.
- `scripts/corpus_synth.py` writes synthetic profiles of its own, never a copied third-party profile, into new rows of `corpus/manifest.tsv`: a gamma 1.0 matrix profile, a `curv` gamma `0x0233` profile, a `para` type 3 profile with the sRGB curve, a colourant-swap profile, and one lookup-table profile for the refusal.

Non-goals: lookup-table profiles, which are F-145's, `Lab ` PCS, absolute colorimetric intent, a GPU form of the transform, the Optical Path Sequence of whole-slide imaging, Color Space `(0028,2002)`, and a display profile for the viewer's own monitor.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| Folder viewer colour recipe | `ocelli_dicom::icc_profile`, then `IccProfile::parse` | Once per held frame | `IccTransform` or a named `IccError` |
| Stage 4 | `ColorTransform` with `IccTransform` | In place of the display scaling | sRGB-encoded `Rgb` at 0 to 255 |
| Colour window and bytes | F-239's `ColourWindow` and `colour_bytes_into` | Unchanged | Bytes |
| inspect JSON | `icc` and a new `icc_applied` | Folder scan | The page's sentence choice |
| GPU colour path | `ColourLut::icc` | Unchanged | `Unavailable` |
| Oracle candidate producer | `produce_current.rs` | Unchanged in this story | cornerstone3D 5.8.2 applies no profile, so no comparison is claimed |

## State transitions and invariants

| Entry point / starting state | Input or event | Resulting state and output | Refusal and preserved state |
|------------------------------|----------------|----------------------------|-----------------------------|
| Folder scan | A file with a matrix profile | Listed, `icc_applied` true | None |
| Held colour frame | The profile parses | Drawn through `IccTransform`, no sentence | None |
| Held colour frame | The profile is refused | As open question 3 decides | The refusal is held in the recipe, so a retry gives the same answer without a second parse |
| Held colour frame | A window drag | The window acts on the converted values | The profile is not parsed again |

Invariant: a frame without `(0028,2000)` produces the bytes F-239 produces.

## Ownership, commit and recovery

Stateless arithmetic over a held frame. The parsed transform is held in F-239's colour recipe under F-216's cache and charged with the recipe's heap bytes. A refused parse holds the refusal. There is no external resource.

## Numbered acceptance inventory

Hand-computed values use device bytes normalised by 255, the TRC, colourants whose columns sum to D50 so a neutral input stays neutral, and the sRGB encoding above, then the tier C byte rule. Neutral rows therefore do not depend on the exact PCS to sRGB matrix beyond its mapping of D50 to `1 1 1`, and the implementer measures and records the largest deviation the s15Fixed16 quantisation of the colourants causes.

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | sRGB colourants, `curv` with `n = 1` and gamma `0x0100`, grey device `64 128 192 255`. Linear equals device, so the sRGB encoding gives `137.2066 187.8450 224.9999 255`, reply `137 188 225 255` | `gamma_one_profile_encodes_linear_as_srgb` | Skipping the encoding, which gives `64 128 192 255` | implementation |
| AC2 | Same colourants, `curv` gamma `0x0233` = 2.19921875, grey `64 128 192`. `61.7730 129.0303 193.4013`, reply `62 129 193` | `u8_fixed8_gamma_is_read_as_two_point_two` | Reading only the integer byte, gamma 2, which gives `71 137 198`. Gamma 2.2 exactly gives the same bytes as 2.19921875 on these rows, so this row does not tell them apart | implementation |
| AC3 | `para` type 3 with the sRGB parameters in s15Fixed16, `g = 2.399994`, `a = 0.947861`, `b = 0.052139`, `c = 0.077393`, `d = 0.040451`, grey `64 128 192`. The round trip is the identity within the quantisation, reply `64 128 192` | `parametric_type_three_srgb_is_the_identity` | Using the `X >= d` branch below `d` | implementation |
| AC4 | `curv` with `n = 2`, entries `0 65535`. Same as AC1 | `a_two_entry_table_is_linear` | Indexing the table without interpolation | implementation |
| AC5 | Colourant swap: `rXYZ` and `gXYZ` exchanged, gamma 1.0 curves, device `255 0 0`. Reply `0 255 0` within the quantisation, and `0 255 0` becomes `255 0 0` | `colourants_are_columns_not_rows` | Transposing the colourant matrix, whose bytes on this row the implementer computes by hand and records | implementation |
| AC6 | Refusals by name: classes `prtr`, `link`, `spac`, `abst`, `nmcl`, PCS `Lab `, colour space `GRAY`, a missing `bTRC`, a `curv` table with a truncated entry, a profile above the size constant, and a profile with `A2B0` | `icc_refusals_are_named` | Drawing with the matrix of a lookup-table profile | implementation |
| AC7 | A frame without `(0028,2000)` produces F-239's bytes for every F-239 fixture | `no_profile_is_byte_identical_to_f_239` | A profile path that perturbs untagged frames | implementation |
| AC8 | 12-bit RGB with the AC1 profile, stored `4095 2048 0` in each channel of three pixels, normalised by 4095. `255`, `187.5365` for `2048 / 4095 = 0.50012`, and `0`, reply `255 188 0` per channel | `icc_normalises_by_bits_stored` | Applying the 8-bit display scaling before the profile | implementation |
| AC9 | The folder viewer draws `synthetic/sc_rgb_icc_gamma1.dcm` with AC1's bytes and no ICC sentence. The lookup-table row behaves as question 3 decides | `examples/folder-viewer/tests/preview.rs` and `tests/browser/colour.mjs` | The F-239 sentence left showing on a converted image | implementation, integrated |
| AC10 | Tier rows. `ColourLut::icc` still returns `Unavailable` and a GPU caller of a profiled frame reports unavailable rather than drawing device RGB | `crates/ocelli-render` tests | A GPU path that silently drops the profile | implementation |
| AC11 | Mutation. Dropping the sRGB encoding, transposing the colourant matrix, and treating `para` type 3's `c` as type 4's `e` turn AC1, AC5 and AC3 red | Recorded in the review | A test that passes on the wrong code | implementation, reviewer |
| AC12 | Before any fixture is accepted, the reviewer checks the header offsets, the tag table, `XYZType`, `curveType`, `parametricCurveType`, the matrix/TRC model, the lookup-table precedence rule and the Bradford annex against ICC.1:2022, and the sRGB encoding against IEC 61966-2-1, and names the editions read | Review note | Fixtures citing a transcription nobody checked | reviewer, before implementation |

## Approach

1. `ocelli-pixel/src/icc.rs`: a bounds-checked big-endian reader over the profile bytes, the header and tag table, the three tag types, and `IccTransform` holding the TRCs and one `f32` 3 by 3 matrix, the product of the PCS to sRGB inverse and the colourant matrix, computed once at parse.
2. `ColorTransform::resolve` takes `Option<IccTransform>`. `map_into` applies the TRCs, the matrix and the sRGB encoding per pixel when present, and F-239's `DisplayScale` otherwise.
3. `ocelli-dicom`: `icc_profile` returns the top-level element's bytes.
4. Folder viewer: the colour recipe parses the profile once, `inspect_json` writes `icc_applied`, the sentence follows question 3.
5. `scripts/corpus_synth.py` writes the synthetic profiles and rows.

Rejected: baking the profile into a 256-entry table per channel, because the matrix mixes channels. A second sRGB encoder in `ocelli-render`, which would be a second copy.

## Dependencies, write ownership and resources

Depends on F-239, which adds the colour recipe, `has_icc_profile`, `DisplayScale`, `ColourWindow` and the ICC sentence this story replaces. Writes `crates/ocelli-pixel/src/icc.rs`, `color.rs`, `lib.rs` and tests, `crates/ocelli-dicom/src/frame_open.rs`, `examples/folder-viewer/src/preview.rs`, `page.html`, `use.html` and tests, `scripts/corpus_synth.py`, `corpus/manifest.tsv`, `README.md`, `docs/hld/DEVIATIONS.md` and the LLD files below. Every folder viewer file is shared with F-212 to F-215, so this story integrates in the folder viewer chain's order. No GPU is needed.

Relationship to F-145. F-145, "ICC colour management pipeline", E35.3, S72, 10w, depends on `E4.9 (UNRESOLVED)`, which `docs/sprints/BACKLOG.md`'s "Recorded defects" table and `scripts/backlog_check.py` keep declared on purpose. This story is the matrix and tone-curve subset of F-145, brought forward for radiology. F-145 keeps the lookup-table profiles, `Lab ` PCS, the whole-slide Optical Path Sequence, a GPU form and absolute intent. This filing does not touch F-145's row, its estimate or its defect. Whether F-145 should depend on F-240 and shrink is open question 4.

## Verification and evidence reuse

`bin/ocelli.sh check ocelli-pixel`, `bin/ocelli.sh check ocelli-dicom` and `bin/ocelli.sh check ocelli-folder-viewer` while iterating, then the feature profile once at the integration tree. The sprint profile runs once for the sprint. No oracle comparison is claimed, because cornerstone3D 5.8.2 applies no ICC profile. An independent observation for the arithmetic, if the operator wants one beyond the hand fixtures, is open question 5.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no wasm crossing. The folder viewer reply carries display bytes as F-239 makes it.
- Render-loop allocation: none. The transform is built at hold time and `map_into` writes into F-239's band scratch.
- unsafe: none
- Tier A (WebGPU): unavailable. `ColourLut::icc` keeps returning `Unavailable`, and a GPU route given a profiled frame reports unavailable rather than drawing device RGB.
- Tier B (WebGL2): unavailable, for the same reason.
- Tier C (CPU): full for the subset, through `ocelli-pixel`'s `IccTransform` and tier C's byte rule. Every other profile is refused by name.

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| fixture | Tone curves `curv` and `para`, ICC.1 tag types, and the sRGB encoding, IEC 61966-2-1 | `crates/ocelli-pixel/tests/icc.rs`, AC1 to AC4, AC8 |
| fixture | Colourant matrix orientation, ICC.1 matrix/TRC model | same, AC5 |
| unit | Named refusals and bounds | same, AC6 |
| integration | Untagged frames unchanged, folder viewer route | `examples/folder-viewer/tests/preview.rs`, AC7, AC9 |
| unit | GPU route unavailable | `crates/ocelli-render` tests, AC10 |
| browser | The profiled image drawn without the sentence | `tests/browser/colour.mjs`, AC9 |

## Parity surface covered

None. `docs/hld/B-parity-surface.md` has no ICC row, and cornerstone3D 5.8.2 does not apply ICC profiles, so this story goes beyond the reference rather than matching it.

## Deviations

None of its own. F-239's deviation for drawing a profiled frame without its profile is narrowed by this story to the profiles it refuses, or retired if question 3 makes every refused profile refuse the frame. The register row is edited in the change that approves this plan.

## LLD impact

- `docs/lld/pixel-pipeline.md`: stage 4's ICC half for the subset, its place and its refusals.
- `docs/lld/frame-open.md`: `icc_profile`.
- `docs/lld/corpus.md`: the profile rows.
- `docs/lld/feature-availability.md`: ICC on tier C for the subset, unavailable on A and B.

## User-facing documentation impact

- `README.md`, the folder viewer's colour paragraph F-239 writes: a matrix profile is applied, and which profiles are not.
- `examples/folder-viewer/src/use.html:638`: "ICC profiles are not implemented" becomes the subset that is, with `IccProfile::parse` in the crate table at `use.html:215`.
- `examples/folder-viewer/src/more.html`: no change, it has no colour card.

## Open questions

1. **Build or a crate.** Affects the write set, the wasm size budgets and every fixture's provenance. Options: A, build the subset in `ocelli-pixel`, a few hundred lines with every value hand-checkable and no new dependency. B, a pure-Rust colour management crate. `docs/SOURCE-POLICY.md` would require, before it is added: its three questions answered and recorded as a dated row in "Extensions to the table" (`docs/SOURCE-POLICY.md:80` to `:88`, `:99` to `:100`), its licence in `deny.toml`'s allow list, an exact `=` pin checked by `scripts/pin_and_size_check.py`, its licence text in the notices F-220 generates, its unsafe sites counted in the external audit, a dependency graph with no Rayon, both wasm size budgets re-measured, and a deviation row, as D-26 and D-28 record for dependencies §15.2 does not list. No crate was assessed for this plan and none is named, because naming one would be a claim this plan has not measured. A C library such as Little CMS is option C and is not recommended, because it adds a C build to the wasm32 target. Owner: operator. Lead recommendation: A, because the subset is small and a crate would still need the hand fixtures. Deferral blocks implementation.
2. **A profile with both a lookup-table tag and the matrix tags.** Options: A, refuse it by name until F-145, because ICC.1 has the lookup table take precedence. B, use the matrix. Owner: operator. Lead recommendation: A.
3. **What a refused profile does to the frame.** Options: A, draw in device RGB with a sentence naming the profile kind not applied, so F-239's deviation narrows to refused profiles. B, refuse the frame by name, which retires that deviation and leaves those files undrawn. Owner: operator. Lead recommendation: A, because it keeps what F-239 draws.
4. **F-145's row.** Options: A, leave it untouched, as this filing does. B, F-145 depends on F-240 and is re-estimated for the remainder, which edits a story carrying a recorded import defect. Owner: operator. Lead recommendation: A until F-145 is designed.
5. **An independent check of the arithmetic.** The hand fixtures share the lead's transcription. Options: A, the fixtures and AC12's review only. B, also compare against a reference colour management tool run on the developer host over the synthetic profiles, tooling only and never redistributed, recorded with its version. Owner: operator. Lead recommendation: B, with the tool named by the operator.
