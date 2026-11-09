# F-240, Matrix and tone-curve ICC profiles in ocelli-pixel stage 4

**Status**: approved
**Epic ref**: E38.38
**Sprint**: S46
**Estimate**: 5w

Story row, `docs/sprints/BACKLOG.md`: "Matrix and tone-curve ICC profiles applied in ocelli-pixel stage 4 for colour frames, lookup-table profiles refused by name until F-145". Layer All, depends on F-239, milestone M20.

`allocation.json` notes: "Filed by the operator on 2026-11-07 with the answers to F-239's questions. F-239 draws a frame carrying ICC Profile (0028,2000) with a sentence that the profile was not applied, and this story applies the matrix and tone-curve subset. Scheduled right after F-239. A subset of F-145, which keeps its own row and its recorded E4.9 defect."

The operator answered questions 1 to 5 on 2026-11-07, recorded under "Decided by the operator on 2026-11-07", and the plan is approved. The estimate rose from 4w to 5w for the independent comparison of AC12 and the measurements of AC13 to AC16. Every path and line below was read on `sprint/s46` at 5451cb7d.

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

1. **The subset.** Profile class `mntr` or `scnr`, data colour space `RGB `, PCS `XYZ `, with all six of `rXYZ`, `gXYZ`, `bXYZ`, `rTRC`, `gTRC` and `bTRC`, each TRC `curv` or `para` of types 0 to 4. Every other shape is not applied, and is named by its kind: another class (`prtr`, `link`, `spac`, `abst`, `nmcl`), a `Lab ` PCS, a `GRAY` profile, a missing tag, and any profile carrying `A2B0`, `A2B1`, `A2B2`, `D2B0` or another lookup-table tag, even when the matrix tags are also present, because ICC.1 has a profile with a lookup-table tag use it in preference to the matrix and using the matrix would be a different answer from the one the profile asks for. By the operator's answer to questions 2 and 3, a frame whose profile is not applied is still drawn in device RGB, as F-239 draws it, and the page names the profile kind, for example "This image's lookup-table ICC profile was not applied." These cases stay under D-30 until F-145.
2. **The rendering intent.** The matrix/TRC model has one transform, so the header's intent is read and recorded and does not change the arithmetic.
3. **Media white and adaptation.** The colourants are taken as already adapted to D50, which ICC.1 v4 requires and which `chad` records. `wtpt` is read and not applied, so the conversion is media-relative colorimetric. Absolute colorimetric is a non-goal.
4. **Where it sits.** After the colour transform of stage 4 and in place of F-239's display scaling, before F-239's colour window. The device values are normalised by `2^n - 1`, with `n` Bits Stored for samples and the descriptor's third value for a palette, because ICC.1 normalises device values by the encoding range. The output is linear sRGB encoded and scaled by 255 as `f32`, so the window and the tier C byte rule are unchanged. A frame without a profile is untouched and keeps F-239's route byte for byte.
5. **One implementation.** The parser and the transform live in `ocelli-pixel`, which owns all pixel interpretation (`docs/lld/pixel-pipeline.md`). `ocelli-dicom` returns the element's bytes and nothing else.
6. **Size.** A profile above a named byte constant is refused by name. The implementer sets the constant from the largest profile in the corpus and real files it can read, and records it.
7. **Build, not a crate.** By the operator's answer to question 1, the subset is built in `ocelli-pixel` with no new dependency.

## Problem and reproducer

After F-239, `synthetic/sc_rgb_icc.dcm` draws in device RGB with "This image's ICC profile was not applied." A secondary capture or ultrasound file whose profile is not sRGB is shown with the wrong colours, and the sentence is the only sign. `ColourLut::icc` in `crates/ocelli-render/src/colour.rs:146` to `:155` always returns `Unavailable`. No ICC parser exists in the tree. `git grep -n -i 'icc_profile\|IccProfile\|2028,2000\|0028,2000' -- crates examples tools` finds none.

## Scope, API and compatibility

In scope:

- `ocelli-pixel` gains `IccProfile::parse(&[u8]) -> Result<IccProfile, IccError>` for the subset of item 1, and `IccTransform`, which maps `Rgb` device values to `Rgb` display values per item 4.
- `ColorTransform` accepts an optional `IccTransform` at resolve time, so the profile is decided once per held frame.
- `ocelli-dicom` gains `icc_profile(metadata: &MetadataSet) -> Option<&[u8]>`, beside F-239's `has_icc_profile`.
- The folder viewer applies the profile. F-239's sentence is removed for a profile this story applies, and a profile this story does not apply is drawn in device RGB with a sentence naming its kind, by the operator's answer to question 3.
- `IccError` names each refusal: `UnsupportedClass`, `UnsupportedColourSpace`, `UnsupportedPcs`, `LookupTableProfile`, `MissingTag`, `UnsupportedCurve`, `Malformed`, `TooLarge`.
- `inspect_json` gains `icc_applied` and `icc_kind`, the kind of a profile not applied, from a closed list matching `IccError`.
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
| Held colour frame | The profile is not in the subset | Drawn in device RGB, the page names the profile kind not applied | The refusal is held in the recipe, so a retry gives the same answer without a second parse |
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
| AC6 | Not applied, by kind: classes `prtr`, `link`, `spac`, `abst`, `nmcl`, PCS `Lab `, colour space `GRAY`, a missing `bTRC`, a `curv` table with a truncated entry, a profile above the size constant, a profile with `A2B0` alone and one with `A2B0` beside all six matrix tags. Each parse returns its named `IccError`, each frame draws F-239's device RGB bytes, and `icc_kind` names the kind on the page | `icc_profiles_outside_the_subset_are_named_and_not_applied` | Drawing with the matrix of a lookup-table profile, or refusing the frame | implementation |
| AC7 | A frame without `(0028,2000)` produces F-239's bytes for every F-239 fixture | `no_profile_is_byte_identical_to_f_239` | A profile path that perturbs untagged frames | implementation |
| AC8 | 12-bit RGB with the AC1 profile, stored `4095 2048 0` in each channel of three pixels, normalised by 4095. `255`, `187.5365` for `2048 / 4095 = 0.50012`, and `0`, reply `255 188 0` per channel | `icc_normalises_by_bits_stored` | Applying the 8-bit display scaling before the profile | implementation |
| AC9 | The folder viewer draws `synthetic/sc_rgb_icc_gamma1.dcm` with AC1's bytes and no ICC sentence. The lookup-table row draws the device RGB bytes F-239 draws and shows "This image's lookup-table ICC profile was not applied." | `examples/folder-viewer/tests/preview.rs` and `tests/browser/colour.mjs` | The F-239 sentence left showing on a converted image | implementation, integrated |
| AC10 | Tier rows. `ColourLut::icc` still returns `Unavailable` and a GPU caller of a profiled frame reports unavailable rather than drawing device RGB | `crates/ocelli-render` tests | A GPU path that silently drops the profile | implementation |
| AC11 | Mutation. Dropping the sRGB encoding, transposing the colourant matrix, and treating `para` type 3's `c` as type 4's `e` turn AC1, AC5 and AC3 red | Recorded in the review | A test that passes on the wrong code | implementation, reviewer |
| AC12 | Two checks before any fixture is accepted, by the operator's answer to question 5. First, the reviewer checks every transcription in this plan, the header offsets, the tag table, `XYZType`, `curveType`, `parametricCurveType`, the matrix/TRC model, the lookup-table precedence rule and the Bradford annex, against the published ICC.1:2022, and the sRGB encoding against the published IEC 61966-2-1, and names the editions read. Second, an independent comparison: Little CMS 2's `transicc`, a host-side tool installed on the developer host at 2.18 by Homebrew, MIT licensed, not shipped and not a dependency, which `docs/SOURCE-POLICY.md`'s "What a dependency is" places outside the policy, converts the AC1 to AC5 and AC8 sample pixels with each synthetic profile as input, its built-in sRGB as output and media-relative colorimetric intent. The largest per-channel difference from Ocelli's `f32` output is measured and recorded with the tool's version and its exact command line, and a difference above one code is investigated before acceptance rather than tolerated. No Little CMS source is read or copied | Review note and the recorded comparison in `docs/lld/pixel-pipeline.md` | Fixtures and code that share one wrong transcription | reviewer and implementer, before fixtures are accepted |
| AC13 | Wasm size. `bin/ocelli.sh gate wasm` passes on the story's tip, building with `bin/ocelli.sh wasm` and `bin/ocelli.sh wasm-decode` and checking `ci/wasm-size-budget.json` and `ci/wasm-decode-size.json` through `scripts/pin_and_size_check.py --with-size`, with neither budget file edited. Both module sizes are measured on the story's base and tip with the same commands and the deltas recorded. `cargo tree -p ocelli-wasm -e normal --target wasm32-unknown-unknown --prefix none` and the same with `--no-default-features --features decode-worker`, run on 5451cb7d for this plan, show both modules reach `ocelli-pixel`, where `icc.rs` lands, and neither reaches `ocelli-dicom` or the folder viewer. No `ocelli-wasm` export calls `IccProfile::parse`, so fat LTO is expected to remove it. The implementer reruns both commands on the tip, and any non-zero delta is explained by symbol | Recorded in `docs/lld/build-targets.md` | An ICC parser reached from a wasm export without a decision, or a re-baselined budget | implementation |
| AC14 | No monochrome or untagged colour regression. `bin/ocelli.sh bench viewer.open viewer.first_image viewer.window_step viewer.scroll_rate viewer.resident_memory` on the F-216 host, on the story's base and tip, stays within F-216's thresholds with both medians recorded. `bin/ocelli.sh bench viewer.colour_frame`, F-239's subject, is within its noise of F-239's recorded figures for its untagged cases | Recorded in `docs/lld/benchmarks.md` | A profile lookup on every frame, or a parse on a frame that has no profile | external, sprint verification |
| AC15 | The ICC path measured once. F-239's `viewer.colour_frame` subject gains an 8192 by 8192 8-bit RGB case carrying the AC2 profile and a 16-bit case carrying a `curv` table profile. It records the first `/preview`, the median of 9 window steps and peak `ps -o rss=`, through `bin/ocelli.sh bench viewer.colour_frame`. Work is done once per held frame: the parse, the colourant and PCS to sRGB matrix product, and a table of each TRC over the `2^n` device codes, at most `3 * 65536` `f32` or 768 KiB, charged to the held frame. Per draw, each pixel costs three table reads, one 3 by 3 product and three sRGB encodings, and the encoding is not tabled because its input is continuous and a table would change values. The numbers are recorded | Recorded in `docs/lld/benchmarks.md` | A parse or a TRC evaluation per draw | external, sprint verification |
| AC16 | Not slower than cornerstone3D. cornerstone3D 5.8.2 applies no profile, so it draws the same profiled frame as device RGB. `bin/ocelli.sh bench --compare-cornerstone --pseudo <dir>`, F-223's harness, on the F-216 host with `<dir>` holding the AC15 frames as series, records open, first image, window step and peak memory for both viewers. The target is that Ocelli with the profile applied is not slower than cornerstone without it, and a miss is recorded with its cause | Recorded in `docs/lld/benchmarks.md` | An ICC path measured against nothing | external, sprint verification, needs F-223 |

## Approach

1. `ocelli-pixel/src/icc.rs`: a bounds-checked big-endian reader over the profile bytes, the header and tag table, the three tag types, and `IccTransform` holding the TRCs and one `f32` 3 by 3 matrix, the product of the PCS to sRGB inverse and the colourant matrix, computed once at parse.
2. `ColorTransform::resolve` takes `Option<IccTransform>`. `map_into` applies the TRCs, the matrix and the sRGB encoding per pixel when present, and F-239's `DisplayScale` otherwise.
3. `ocelli-dicom`: `icc_profile` returns the top-level element's bytes.
4. Folder viewer, with `tools/bench`'s `viewer.colour_frame` runner gaining the AC15 cases: the colour recipe parses the profile once, `inspect_json` writes `icc_applied`, a profile outside the subset sets `icc_kind` and the page names that kind.
5. `scripts/corpus_synth.py` writes the synthetic profiles and rows.

Rejected: baking the profile into a 256-entry table per channel, because the matrix mixes channels. A second sRGB encoder in `ocelli-render`, which would be a second copy.

## Dependencies, write ownership and resources

Depends on F-239, which adds the colour recipe, `has_icc_profile`, `DisplayScale`, `ColourWindow` and the ICC sentence this story replaces. Writes `crates/ocelli-pixel/src/icc.rs`, `color.rs`, `lib.rs` and tests, `crates/ocelli-dicom/src/frame_open.rs`, `examples/folder-viewer/src/preview.rs`, `page.html`, `use.html` and tests, `scripts/corpus_synth.py`, `corpus/manifest.tsv`, `tools/bench/src/runners/viewer_colour_frame.mjs` and `tools/bench/subjects.json`, `README.md` and the LLD files below. `docs/hld/DEVIATIONS.md` is not written, because D-30 already covers the profiles this story does not apply. AC16 needs F-223 integrated, which it is scheduled to be within S46. Every folder viewer file is shared with F-212 to F-215, so this story integrates in the folder viewer chain's order. No GPU is needed.

Relationship to F-145. F-145, "ICC colour management pipeline", E35.3, S72, 10w, depends on `E4.9 (UNRESOLVED)`, which `docs/sprints/BACKLOG.md`'s "Recorded defects" table and `scripts/backlog_check.py` keep declared on purpose. This story is the matrix and tone-curve subset of F-145, brought forward for radiology. F-145 keeps the lookup-table profiles, `Lab ` PCS, the whole-slide Optical Path Sequence, a GPU form and absolute intent. This filing does not touch F-145's row, its estimate or its defect. Whether F-145 should depend on F-240 and shrink is open question 4.

## Verification and evidence reuse

`bin/ocelli.sh check ocelli-pixel`, `bin/ocelli.sh check ocelli-dicom` and `bin/ocelli.sh check ocelli-folder-viewer` while iterating, then the feature profile once at the integration tree. The sprint profile runs once for the sprint. No oracle pixel comparison is claimed for a profiled frame, because cornerstone3D 5.8.2 applies no ICC profile. The independent observation is AC12's `transicc` comparison. AC13 runs `bin/ocelli.sh gate wasm` on base and tip, AC14 runs `bin/ocelli.sh bench viewer.open viewer.first_image viewer.window_step viewer.scroll_rate viewer.resident_memory viewer.colour_frame` on base and tip, AC15 runs `bin/ocelli.sh bench viewer.colour_frame` on the tip, and AC16 runs `bin/ocelli.sh bench --compare-cornerstone --pseudo <dir>` once F-223 has integrated, all on the F-216 host with the release build.

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

## cornerstone3D 5.8.2 is the floor

By the operator's clarification of 2026-11-07: every behaviour cornerstone3D 5.8.2 has for these images Ocelli has too, every departure is better on a measured axis and recorded as a measured divergence under D14, and where cornerstone can be timed on the same frame Ocelli is not slower.

| Behaviour | cornerstone3D 5.8.2 | Ocelli after F-240 | Axis and measurement |
|-----------|---------------------|--------------------|----------------------|
| A frame with a matrix and tone-curve profile | Drawn in device RGB, profile ignored | Drawn through the profile into sRGB | Better, closer to ICC.1 and to what the file asks. AC1 to AC5, AC8, checked independently by AC12 |
| A frame with a lookup-table or other profile | Drawn in device RGB, profile ignored | Drawn in device RGB, the kind named | Better, the omission is visible. AC6 |
| A frame without a profile | Drawn | Drawn, F-239's bytes | Same. AC7 |
| Window, invert, statistics, readout on a profiled frame | As F-239's table | F-239's rules on the converted values | Same rule. The values differ by the profile, which is the point of the story |
| Speed and memory | Measured by F-223's harness on the same frames | Measured | Target not slower. AC16 |

There is no oracle pixel row for a profiled frame, because the reference does not apply the profile and agreement with it would be agreement with the device RGB this story replaces.

## Parity surface covered

None. `docs/hld/B-parity-surface.md` has no ICC row, and cornerstone3D 5.8.2 does not apply ICC profiles, so this story goes beyond the reference rather than matching it.

## Deviations

None of its own. D-30, raised by F-239, already records that a profile outside this story's subset is drawn without being applied and named on the page, which is the operator's answer to question 3. This story narrows D-30 in practice and does not need to edit its text.

## LLD impact

- `docs/lld/pixel-pipeline.md`: stage 4's ICC half for the subset, its place and its refusals.
- `docs/lld/frame-open.md`: `icc_profile`.
- `docs/lld/corpus.md`: the profile rows.
- `docs/lld/feature-availability.md`: ICC on tier C for the subset, unavailable on A and B.
- `docs/lld/build-targets.md`: AC13's deltas and reachability.
- `docs/lld/benchmarks.md`: AC14 to AC16's figures.

## User-facing documentation impact

- `README.md`, the folder viewer's colour paragraph F-239 writes: a matrix profile is applied, and which profiles are not.
- `examples/folder-viewer/src/use.html:638`: "ICC profiles are not implemented" becomes the subset that is, with `IccProfile::parse` in the crate table at `use.html:215`.
- `examples/folder-viewer/src/more.html`: no change, it has no colour card.

## Decided by the operator on 2026-11-07

1. **Build or a crate.** Build the matrix and tone-curve subset in `ocelli-pixel`, no dependency. Applied in item 7 and the Approach.
2. **A profile with both lookup-table and matrix tags.** Drawn without the profile and named, as any lookup-table profile is. Applied in item 1 and AC6.
3. **What a profile outside the subset does to the frame.** Drawn in device RGB, and the page names the profile kind not applied. Covered by D-30. Applied in item 1, the state table, AC6 and AC9.
4. **F-145's row** (lead default). Left untouched, with its estimate and its recorded E4.9 defect.
5. **An independent check.** The reviewer checks every transcription against the published ICC.1 and IEC sRGB text before fixtures are accepted, and a host-side colour tool, not shipped and not a dependency, converts a few sample pixels as an independent comparison with the measured difference recorded. The lead names Little CMS 2's `transicc`, present on the developer host. Applied in AC12.

Also given the same day, relayed by the lead: the features must not grow the wasm build beyond budget or make the system slower, applied in AC13 to AC15, and cornerstone3D 5.8.2 is the floor and Ocelli should be better, applied in "cornerstone3D 5.8.2 is the floor" and AC16.

## Open questions

None. Questions 1 to 5 are answered above.

### Lead rulings after review pass 1

Made by the sprint lead on 2026-11-07 after `.claude/reviews/F-240-working-pass-1.md`, which checked the transcriptions against ICC.1:2022-05 and the ICC's sRGB note and compared Ocelli with Little CMS 2.18.

- **D1, out-of-gamut colour is clipped** to 0..1 in linear sRGB before encoding, per the ICC sRGB note A.7 and ICC.1 F.8 to F.16, so F-239's colour window never sees a value outside 0..255. A fixture with Adobe RGB 1998 green asserts the clipped bytes.
- **D2, the PCS-to-sRGB matrix comes from one consistent source.** Use the inverse of the D50 matrix published in the ICC's sRGB note, computed in the test from the printed values, so D50 maps to sRGB white within the matrix's own precision, and the comment's numbers are reproducible from what it cites.
- **D3, a GPU route given a profiled frame reports unavailable,** in code, with a render test, and `ColourLut::icc`'s documentation says what is now true. A profiled palette is not baked as device RGB on any route that cannot apply the profile.
- **D4, the LLDs describe this branch only.** Sentences about callers the viewer part will add are removed or marked as the viewer part's.
- **D5 and D6, expected values come from the formula.** The palette fixture's 187.5147 and 136.9611 are computed by hand and asserted within 1e-3, and AC5's wrong-answer bytes are what the transposed matrix actually gives.
- **D7 and D8, every guard and route has a test:** a negative s15Fixed16 parameter, the 10.6 and 10.18 range clip, the `XYZ ` type check, the table depth check, and each of the three YBR routes with ICC attached (the profile's input is RGB after the YBR conversion, PS3.3 C.11.15.1.1).
- **S1, a profile is classified before its size is checked,** so a large lookup-table profile is named as lookup-table, and `D2B3` joins the list (N1).

### Lead rulings after pass 2

- **D1.** `build-targets.md` records the measured tip, 2,182,144 bytes, with its delta and percentage, and attributes the size it reports.
- **D2.** The ruled 1e-3 stands. The palette fixture asserts the hand value computed from the profile's own s15Fixed16 colourant words, ICC.1 Table 4, `(254.9997, 187.5169, 136.9621)`, within 1e-3, with that derivation in its comment. It must go red under pass 2's mutant M23.
- **D3.** The three places attribute the PCS-to-device constant to ICC.1 Annex F.3 and say the note's printed matrix is the device-to-PCS one.
- **D4.** Each residue figure is one stated quantity with its definition and measured value, or it goes.
- **D5 and D6.** The "one code apart" line is reworded. No added comment line carries a prose semicolon, whatever `prose_check.py` scans.
- **Nitpicks taken.** N1 derives the two fixture figures. N2 scopes or corrects the 0.0027 figure. N4 removes the viewer-part claim from `color.rs:660`. N3, N5 and N6 stay carried.
- **AC12.** The transicc comparison is run again against the new matrix and recorded, running Little CMS as a tool only.

### Lead rulings after pass 3

- **D1.** The AC12 record states the Adobe red row as a measured D14 divergence from Little CMS 2.18. It names the cause, the sRGB note's printed matrix against Little CMS's built-in sRGB, and gives the colorimetric answer, green 0, because the two spaces share the red chromaticity. Both values are in one unit, and the same row's blue difference of 0.102 is given too. It says no stored byte differs at the identity window, and gives one measured non-identity colour window that shows it on tier C. "Nothing needed investigation" becomes "below AC12's one-code threshold at the identity window".
- **The matrix choice stays for this story.** The plan rules the note's printed matrix, and pass 3 measured Ocelli further from the colorimetric answer than Little CMS on this row, by about 1.0e-4 linear. A backlog story is filed at sprint close to measure an inverse derived from the sRGB primaries with Bradford adaptation against Little CMS across every AC12 row, and to adopt it only if it is closer on every row.
- **D2.** The palette comment prints the linear values as reproduced, and says which gap its figure measures.
- **N1 taken.** `build-targets.md` names the commit by its title. N3, N5 and N6 stay carried.


### Viewer implementation and fixture facts, 2026-11-08

The held colour recipe parses the profile before buffer admission, attaches the
accepted transform once and charges its retained palette and ICC table storage.
Scan JSON classifies the same bytes with icc_applied and a closed icc_kind. It
does not build device-code tables. Missing or unsupported profiles retain the
F-239 device route. No profile is parsed during a held window redraw.

The corpus adds sc_rgb_icc_gamma1, sc_rgb_icc_gamma22, sc_rgb_icc_srgb_para,
sc_rgb_icc_swap and sc_rgb_icc_lut. All five profiles are generated here from
ICC.1:2022 header, tag and type definitions, with the common description,
copyright and media-white tags. Little CMS 2.18 accepts and converts all five
with transicc as an external host tool, including the actual lookup profile.
No Little CMS source or third-party profile was read or copied.

ICC.1:2022 7.2.4 encodes version 4.4.0.0 as 04400000, confirmed directly in the
published document. Two inherited Rust fixture builders used 04440000, which
names bug-fix version 4. The builders now use 04400000. No parser arithmetic,
expected colour value or tolerance changes. The corpus fixture tests pin the
standard version and the independent fixed-point words.

### Lead clarification, shared regression and stacked size evidence, 2026-11-08

AC14 uses the same corrected helper on base and tip under F-239's shared
regression instrument clarification. The untagged colour cases keep their
original registered protocol. Actual measurements remain open until the
final composed instrument and release products are run on the named host.
AC13 records the original, colour and combined stacks separately, names
other stories in each interval and attributes retained code by executable
identity. Combined growth is never labelled an isolated ICC cost.

### Implementation amendment, AC15 held budget, 2026-11-08

The new 8192 by 8192 native RGB16 ICC workload has 384 MiB of encoded
samples and 384 MiB of decoded samples. It cannot be held under the 256 MiB
default. The lead authorizes an explicit 512 MiB held budget for this new
workload only. Its benchmark detail records both requested and actual budget,
with a small fake-child control for their agreement. The two existing F239
workloads retain their default budget and input bytes. The RGB8 ICC workload
also retains the default budget. If 512 MiB does not admit the intended held
frame, investigate and report the admission cause rather than increasing it
again. This is a bounded benchmark setup exception, with no threshold or
allocator-policy change.

### Supplementary normalized diagnostic scope, 2026-11-08

Root authorized a separate control based on the untouched pre-ICC tree
12797792, applying only the independently reviewed palette-validation hoist
and exact final-byte store. It adds no ICC implementation. The resulting
c4d6e4b3 source passed focused tests and a release build, then the unchanged
two-case native instrument completed with permitted process observation.
The initial sandbox RSS refusal is retained separately. The current evidence
and exact identities are in docs/lld/benchmarks.md.

This adds diagnostic evidence and does not replace the literal historical
comparison, amend AC14, change a tolerance or call improvements within noise.
It removes two known differences but does not isolate every remaining product
change as ICC cost. Any substitution for AC14 remains a separate explicit
plan decision.


### Lead ruling, explicit colour census declarations, 2026-11-09

The first post-citation oracle gate reached the reference identity comparison and failed its coverage census. Eleven newly added synthetic colour views were correctly unmeasured under existing D-16 but absent from the declared inventory. The four compressed YBR views and saturated RGB reference belong to F-239. The six RGB ICC views belong to F-239 and F-240. Every one is absent from the canonical pre-integration manifest and has three components with eight stored bits in the actual reference sidecar. No previously scored view is demoted.

The lead adopts an explicit addition of those eleven identifiers with only `unstated-threshold` to `tools/oracle/compare-expectations.json`, subject to independent review like a tolerance change. All 48 prior entries and qualifiers remain unchanged. This applies the approved unscored-colour scope and existing HLD 25.1/D-16, with no new metric, numerical tolerance, class rule, pixel gold, gate bypass or passed-colour claim. It does not absorb an unexpected scored failure. The exact identifiers, provenance and review are recorded in the existing feature review series. The failed gate remains failed. The corrected tree must pass the actual identity and mutation consumers and a fresh named oracle gate before a passing receipt is recorded. Native candidate parity remains a separate production comparison.


### Lead ruling, untagged reductions satisfy non-regression, 2026-11-09

The original literal phrase "within its noise" is not met by the large observed reductions. Under the operator's direction to avoid making the system slower and the normal design authority of this run, the lead adopts the following independently reviewed clarification. The literal story base and all observations remain intact. The required first-image repair is complete within every existing monochrome band and absolute limit, with no exception.

> AC14's untagged colour non-regression clause accepts the measured reductions against the literal F-239 base as non-regression. Record the actual observations as reductions, without calling them within noise. Preserve all original and normalized-control data and their distinct roles. All monochrome limits, registered baseline bands, budgets, sample counts, gates and pixel tolerances remain unchanged. This disposition supplies no colour pixel parity, isolated ICC cost, other acceptance row, operator check or sprint closure.

Current RGB8 first request and window median are 2171.469/2699.735 ms against literal-base 4955.386/4728.307 ms. Palette16 is 2503.991/3087.126 ms against 19287.031/19257.557 ms. These combined-stack observations include the palette-validation and exact-byte-store changes and do not isolate ICC cost. Their shared fixture and measurement helper hashes agree. The original raw SHA-256 is `61c638f921c418c6a19dbe3207c2930ec28ad6f7d81e6b7c33d21897c576f4fd`, current `6f32db8e4cc0b525a28c95bb973747973ae9bed1a774440300d13bcfaeabd3c4`. Independent factual and authority review is retained with the feature evidence. The earlier proposal's extra operator-condition wrapper was not adopted. This ruling applies only to untagged reductions against the literal base, with no substitution by the normalized control. Genuine Cornerstone target misses and the separate incompatible JPEG-LS host remain separately qualified.
