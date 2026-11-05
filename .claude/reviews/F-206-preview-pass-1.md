# F-206 review, pass 1

**Reviewed**: working tree on sprint/s45 at cac65d0, including untracked synthetic.rs
**Author / reviewer**: implementation by the sprint host. Review by Claude Code, read only, with no shell.
**Result**: 2 defects, 2 smells, 3 nitpicks

## Acceptance challenged

- AC1 holds. `synthetic.rs` writes no group 0010 element. The test searches the raw bytes for both tags and also reads `patientName` and `patientId` back through `inspect_json`.
- AC2 holds as written. The test checks the same two pixels on every slice. It never checks that slices differ, so a builder that ignored `slice_index` would still pass. The design does not ask for more, so this is not a finding.
- AC3 and AC4 hold. The greys were recomputed from PS3.3 C.11.2.1.2 rather than read back from the code. The working is under Verified clean.
- AC5 is not observable from a read. The test comment states the mutation. Only the author's report says it went red.
- AC6 and AC7 hold for the synthetic slice. AC6 does not cover a legal DS spelling that `clean_decimal` mishandles. See D1.
- AC8 holds for both the VOI LUT Sequence and the Modality LUT Sequence.
- The state table in the design says a bad token is the window error. The implementation and its test both say `header`. See D2.

## Evidence

- Read in full: the design plan, `synthetic.rs`, `preview.rs`, `serve.rs`, `lib.rs`, `tests/preview.rs` and `presentation.rs`.
- Read in part: `apply_window` in `crates/ocelli-pixel/src/lut.rs`, `display_index` in `crates/ocelli-render/src/colour.rs`, and `parse_decimal_string` in `crates/ocelli-dicom/src/frame_geometry.rs`. Also `PRESET_BYTES` in `crates/ocelli-viewport/src/boundary.rs`, and the inspect consumer in `examples/folder-viewer/src/page.html`.
- No test, clippy or diff was run in this pass. The author's report on tests, clippy and the intercept mutation is unverified here.

## Defects

### D1, spacing text is not a safe JSON number, and a refused Pixel Spacing falls through to the imager pair

**Where**: `examples/folder-viewer/src/preview.rs` `clean_decimal`, used by `spacing_pair` and emitted raw by `push_json_token`.

**What**: Two triggers.

Trigger A. Pixel Spacing `00.5\00.5` or `-.5\-.5`. `parse_decimal_string` accepts both. The character filter allows them, and the leading-dot check does not catch `-.5`. `inspect_json` then writes `"rowSpacing":00.5`, which is not a JSON number. `response.json()` throws and the file disappears from the series list.

Trigger B. Pixel Spacing `5E-1\5E-1` with Imager Pixel Spacing `0.6\0.6` also present. That is legal DS under PS3.5 6.2. `clean_decimal` refuses `E`, so `spacing_pair` returns nothing and `frame_facts` uses the imager pair. The response reports `0.6` as `ImagerPixelSpacing`. A Pixel Spacing with three values falls through the same way.

**Why it is wrong**: The design precedence is Pixel Spacing, else Imager Pixel Spacing, else empty. A present but unusable Pixel Spacing is not an absent one. PS3.3 C.7.6.2.1.1. Detector-plane spacing must not be presented as the patient-plane spacing.

**Evidence**: Read of `clean_decimal`, `spacing_pair`, `frame_facts` and `push_json_token`. No shell in this pass.

**Scope and regression**: Inspect and the preview spacing headers. Smallest check is those two fixtures, asserting a JSON number with source `PixelSpacing`, or an explicit empty result, and never the imager pair while Pixel Spacing is present.

### D2, a bad preset token is a header error

**Where**: `examples/folder-viewer/src/serve.rs` maps `window_preset_from_token` failure through `header_unusable`.

**What**: `POST /preview` with `X-Ocelli-Window-Preset: soft` returns `X-Ocelli-Error: header`.

**Why it is wrong**: The state table in `.claude/plans/F-206-design.md` says a bad token is the window error. `parse_function` already uses that error for a bad window function. The test asserts `header`, so it records the code rather than the design.

**Evidence**: Read of the design state table, `serve.rs` and the test.

**Scope and regression**: The preset header only. Change the code and the test to `window`, or amend the design before merge.

## Smells

### S1, a preset discards a window function

**Where**: `preview.rs` preset branch forces `VoiFunction::Linear` and never reads `request.window_function`.

**What**: A request with `lung` and `LINEAR_EXACT` returns 200 and LINEAR. The header reports LINEAR, so the pixels are not mislabelled. A preset with a numeric window refuses, while a preset with a function is ignored.

**Why it will break later**: The three window inputs do not behave alike. Refusing the combination next to the numeric check would close it.

### S2, preview builds the multiframe read twice

**Where**: `lut_chain` and `frame_facts` each call `project_metadata` and build a `MultiframeMetadata`.

**What**: The design's consumer map says one geometry helper over the same multiframe read as the window. Every preview walks the data set twice.

**Why it will break later**: The two reads can drift. Passing one `MultiframeMetadata` into both keeps the plan.

## Nitpicks

### N1

`synthetic.rs` writes Study ID (0020,0010) with VR `IS`. The data dictionary gives `SH`. The value `1` is legal under both.

### N2

`element_text` falls back to `CS` for any tag missing from the match. A tag added later without a match arm gets a wrong VR and no error.

### N3

The zero-vector branch in `direction_letter` cannot be reached. `unit_cosine` has already rejected that vector.

## Verified clean

- LINEAR arithmetic matches PS3.3 C.11.2.1.2. Low side is `<=`. High side is `>`. `c' = c - 0.5` and `w' = w - 1`.
- Hand-computed greys agree with the tests. Soft tissue body is 127.8196 and grey 128. Lung air, body and lung are grey 60, 236 and 94. Bone body and bone are grey 83 and 196. Air and the out-of-window tissues are 0 or 255 as the bounds require. `display_index` rounds by adding 0.5 in f32, ties up.
- Named pixels match the phantom. Stored values are HU + 1024.
- Preset centre and width come from `WindowPreset`. The folder viewer has no second window formula. The preset branch uses `LutChain::new`.
- A Modality LUT Sequence is refused before every branch. A preset refuses a VOI LUT Sequence. A numeric override still replaces one.
- Pixel Spacing element 0 is the row and element 1 is the column. The phantom writes `0.7` then `0.5`.
- Edge letters for `1\0\0\0\1\0` are left R, right L, top A, bottom P. A missing or non-unit orientation yields four empty strings.
- The synthetic study has no patient name or id. UIDs use `2.25.`. Routes return 12 slices, and index 12 or a word is 404.
- Spacing header tokens that pass the current filter cannot inject a new header line.

## Limitations and handoff

- No shell, so the diff against cac65d0 was not seen. The named files were read in full, which includes code from earlier stories. Nothing was raised that belongs only to that earlier code.
- Tests, clippy and the intercept mutation were not re-run here.
- Next is to fix D1 and D2, then close S1 and S2, then a fresh pass.

COUNTS defects=2 smells=2 nitpicks=3
