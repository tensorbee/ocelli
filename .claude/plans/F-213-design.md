# F-213, Folder viewer window presets by modality on number keys, every file window by its explanation, and a region window

**Status**: approved
**Epic ref**: E38.23
**Sprint**: S46
**Estimate**: 3w

## Normative source, transcribed

`docs/hld/15-lut-chain.md` section 18:

```text
This is the highest-risk arithmetic in the project. It is specified in DICOM
PS3.3 C.11 and the stages apply strictly in order. Implement it once, in
ocelli-pixel, and let the shader read the parameters -- do not let a second
copy of this logic appear anywhere.

| 2. VOI LUT | Modality -> Display | Window centre/width with a function, or a VOI LUT Sequence |

// PS3.3 C.11.2.1.2 -- LINEAR. Requires w >= 1.
// c' = c - 0.5 ; w' = w - 1
// x <= c' - w'/2 -> ymin
// x > c' + w'/2 -> ymax
// else y = ((x - c') / w' + 0.5) * (ymax - ymin) + ymin
```

Section 18.3 names the soft-tissue CT window, centre 40 and width 400.

The library already selects one pair from a multi-valued window. `crates/ocelli-pixel/src/lut.rs`:

```rust
pub fn new(
    lut: Option<LutDescriptor>,
    centers: &[f32],
    widths: &[f32],
    selected: usize,
    function: VoiFunction,
    ymin: f32,
    ymax: f32,
) -> Result<Self, PixelError>
// centers.is_empty() || centers.len() != widths.len() -> MismatchedWindowMultiplicity
// centers.get(selected) or widths.get(selected) is None -> WindowIndexOutOfRange
// Linear requires width >= 1.0. LinearExact and Sigmoid require width > 0.0.
```

Its accessor doc says the window pair returned is the selected one, resolved once by `VoiTransform::new` from the possibly multi-valued Window Center and Window Width of PS3.3 C.11.2.1.1, so a caller cannot re-select.

`crates/ocelli-viewport/src/presentation.rs` holds the only preset numbers. `WindowPreset::SoftTissue` is centre 40 and width 400, from HLD 18.3. `Lung` is -600 and 1500 and `Bone` is 300 and 1500, each documented as a plan number and not a clinical standard. Each is `LINEAR`. `apply_preset` refuses a Modality LUT Sequence and `set_window` refuses a VOI LUT Sequence with `PresentationError::Sequence`. `docs/lld/public-api.md` names the tokens `soft_tissue`, `lung` and `bone` and says the presets make no clinical-standard claim.

The library region window is `windowRegion` in `packages/core/src/manipulation.ts`, under the F-087 plan:

```text
window_region takes caller samples. The centre is (min + max) / 2.
The width is max - min. An empty list, a non-finite sample, or a span
of 0 is refused. The function does not read pixels and it does not apply
a VOI LUT.
```

There is no Rust copy of that rule.

PS3.3 C.11.2, VOI LUT Module, transcribed from the published standard by the author without a copy at hand, which is a transcription from memory. The flag stays. No copy is in the repository, and AC15 makes the reviewer check it against the current edition at dicom.nema.org before the window list and the index selection are implemented:

| Attribute | Tag | VR, VM | Type | Text |
|---|---|---|---|---|
| Window Center | (0028,1050) | DS, 1-n | 1C | Window Center for display. Required if VOI LUT Sequence is not present. May be present otherwise. |
| Window Width | (0028,1051) | DS, 1-n | 1C | Window Width for display. Required if Window Center is present. |
| Window Center & Width Explanation | (0028,1055) | LO, 1-n | 3 | Free form explanation of the meaning of the Window Center and Width. Multiple values correspond to multiple Window Center and Width values. |
| VOI LUT Function | (0028,1056) | CS, 1 | 3 | Defined Terms LINEAR, LINEAR_EXACT, SIGMOID. When absent the interpretation is linear as in C.11.2.1.2. |
| VOI LUT Sequence | (0028,3010) | SQ, 1-n | 1C | Required if Window Center is not present. May be present otherwise. |
| LUT Explanation, in each VOI LUT item | (0028,3003) | LO, 1 | 3 | Free form text explanation of the meaning of the LUT. |

PS3.3 C.11.2.1.2:

```text
If multiple values are present, both Attributes shall have the same number of
values and shall be considered as pairs. Multiple values indicate that
multiple alternative views may be presented.
```

`.claude/skills/dicom-expert/SKILL.md` section 5.2 adds the project reading: PS3.3 C.11.2.1.2.2 permits a VOI LUT or a window pair as alternative views, with one applied at a time. Ocelli's constructor selects the sequence when present, and preserving that selected chain through presets and manual window commands is a project policy, not a DICOM prohibition on explicit alternative selection.

The Frame VOI LUT Sequence (0028,9132) of an enhanced multiframe carries the same Window Center, Window Width, explanation and function per frame. `preview.rs` already resolves them with `MultiframeMetadata::resolve` and `TopLevelFallback::Allowed`, and `ResolvedFunctionalGroup::source` reports `PerFrame(n)`, `Shared` or `TopLevel`.

cornerstone3D 5.8.2 `WindowLevelRegionTool.applyWindowLevelRegion`, read at the oracle pin in `tools/oracle/node_modules/@cornerstonejs/tools/dist/esm/tools/WindowLevelRegionTool.js`:

```js
const minMaxMean = windowLevel.calculateMinMaxMean(pixelLuminanceData, imageData.minPixelValue, imageData.maxPixelValue)
const windowWidth = Math.max(Math.abs(minMaxMean.max - minMaxMean.min), this.configuration.minWindowWidth)  // default 10
const windowCenter = minMaxMean.mean
```

That is not the library rule. The centre is the mean, not the mid-span, and the width has a floor of 10 instead of a refusal at 0. Open question 3.

The research note `pro-viewer-research.md` section 3 records that RadiAnt binds number keys 0 to 9 to window presets, with CT presets on number keys. Section 5 records that MicroDicom offers a region window, and that InteleViewer keeps preset lists keyed by modality.

## What the specification does not cover

**Which presets each modality gets.** CT gets the three library presets in the order soft tissue, lung, bone. Every other modality gets none. The decision is made by the local process from Modality (0008,0060) equal to `CT`, and the server refuses a preset on any other modality. The page gate stays, but it is not the guard.

**Where the preset numbers are shown.** The inspect JSON for a CT carries `presets`, each with the token, a display name, and the centre and width from `WindowPreset::center` and `WindowPreset::width`. The page draws the rows from that list. It holds no token list, no name list and no number of its own.

**How file windows are listed.** One row per index of the Window Center and Window Width pair for the inspected frame, in file order. The row label is the explanation at the same index. An absent, empty or missing explanation reads `Window n`, counted from 1. Extra explanations are ignored. Each VOI LUT Sequence item is also listed, by its LUT Explanation or `Lookup table n`, as a disabled row whose sentence says this preview does not apply a VOI LUT Sequence. The centre and width shown in a row are the file's DS tokens through F-206's `clean_decimal`.

**Selecting a file window.** A new request header `X-Ocelli-Window-Index`. The index goes to `VoiTransform::new` as `selected` with the full centre and width lists and the file's VOI LUT Function. Nothing re-selects in the folder viewer. With no header the index is 0, which is today's behaviour. The reply reports the index used and the count.

**Refusals for the index**, each before a grey byte is written:

- with a preset, or with a numeric centre or width, the window error, the same rule as preset against numeric today
- out of range, or on a file with no window pair, the window error
- on a file with a VOI LUT Sequence, the window error, the same project policy as a preset (open question 2)

`X-Ocelli-Window-Function` with an index stays allowed, as it is with the file window today.

**Multiframe windows that change per frame.** The inspect JSON reports `windowsPerFrame` true when the Window Center resolved for frame 0 came from a per-frame item. The page then asks `/inspect` again with `X-Ocelli-Frame` when the shown frame changes, and replaces the list. That request uses F-216's id form, `X-Ocelli-File` with no body, which this story adds to `/inspect`, so a frame change never posts the file again. Otherwise the scan result stands.

**Number keys** (open question 1 holds the decision). The plan's recommendation: the window list is numbered, and bare `1` to `9` select row 1 to 9. On CT, rows 1, 2 and 3 are always soft tissue, lung and bone, and the file windows follow from 4. On every other modality the file windows start at 1. `0` stays Fit, as F-207 bound it. A disabled row is skipped by its key, and the key does nothing. The keys `1` and `2` leave the layout buttons, which F-208 rebinds.

**Region window.** A W/L region tool drags a rectangle in image pixels, using the F-207 inverse transform the marks use. On release, the page sends `POST /region` with F-216's `X-Ocelli-File` id, the rectangle and the frame, and no body. The local process returns the minimum, maximum and count of the modality values inside it, read from F-216's held frame through F-210's `HeldFrame::value`, which is the same library `open_frame` modality path, not through `preview.rs`'s local `rescale`, which F-210 rejects for values. Pixel `(i, j)` covers `[i, i + 1) x [j, j + 1)` and is inside when its centre `(i + 0.5, j + 0.5)` lies in the closed rectangle after the corners are sorted. The page passes `[min, max]` to the library `windowRegion`, whose result depends on the minimum and maximum alone, so the two-sample list gives the same centre and width as every sample. The result is sent as the existing numeric override, with the function currently shown, the same as a W/L drag. `windowRegion` runs in the page from the built `@ocelli/core`, served by the local process under `/lib/` from an allowlist, which is the operator's answer to open question 4. When the library build is absent the region tool reads unavailable and says "The region window needs the library build. Run npm run build in the repository, then start the viewer again." It does not fall back to anything.

**Padding.** F-210 reports a sample inside the padding range as padding with no number, from `OpenedFrame::padding_mask`. The region follows it. A padding sample is not counted and does not move the minimum or maximum. A rectangle that holds only padding is the `region` error. The full-range window of a file with no window is not changed by this story.

**State on the page.** One window state, one of: file window at index `i`, preset token, or numeric override from a drag or a region. Choosing one clears the others. Reset returns to file window index 0. A series change returns to index 0. A slice change keeps the state, except that a file index at or past the next image's count falls back to index 0 before the request is sent, so a shorter list does not show a fault. A refused region or preset leaves the previous state and image in place and puts the refusal in the sentence line.

**Corner text.** F-212 owns the corners. This story adds one word to the bottom right window line: the preset name, the file window label, `region`, or `manual`.

## Problem and reproducer

Open a CT that stores Window Center `40\-600` and Window Width `400\1500` with Window Center & Width Explanation `WINDOW1\WINDOW2`. The page draws window 1 and offers no way to reach window 2. Nothing shows `WINDOW1` or `WINDOW2`.

Open an MR. The three preset buttons are disabled by `syncPresetAvailability` in the page, but `POST /preview` with `X-Ocelli-Window-Preset: lung` still draws a Hounsfield window on MR signal.

Press `1`. The layout changes. A radiologist expects a window preset.

There is no region window in the folder viewer.

## Scope, API and compatibility

In scope:

- `FrameRequest` gains `window_index: Option<u32>`. `Preview` gains `window_index: Option<u32>` and `window_count: u32`.
- `lut_chain` passes `selected` and reports `centers[selected]` and `widths[selected]`, not `first()`.
- `PreviewError` gains `Modality`, code `modality`, sentence `these presets are in Hounsfield units and apply to CT only`. A preset on any modality other than `CT` returns it.
- `inspect_json` gains `presets`, `windows`, `voiLuts` and `windowsPerFrame`.
- `fn region_extent(held: &HeldFrame, rect: [f64; 4]) -> Result<RegionExtent, PreviewError>` in `preview.rs`, with `RegionExtent { min: f32, max: f32, count: u32 }`, used by the route, and `pub fn region_extent_frame(bytes: &[u8], frame: u32, rect: [f64; 4])`, its composition with F-216's `decode_held`, for the fixtures. The `POST /region` route in `serve.rs` takes only the id form, and a request without `X-Ocelli-File` is 400 `header`.
- `/inspect` accepts F-216's id form beside its body form, for the per-frame repost.
- The library routes in `serve.rs`, written here because F-215 lands after this story. `GET /lib/ocelli-core/<name>.js` serves `<CARGO_MANIFEST_DIR>/../../packages/core/dist/<name>.js` only when `<name>` is in a tracked `const LIBRARY_FILES: &[&str]`, the non-test modules of `packages/core/src/index.ts`'s relative import closure. `GET /lib/dcmjs.js` serves `<CARGO_MANIFEST_DIR>/../../node_modules/dcmjs/build/dcmjs.es.js`, because the `@ocelli/core` SR modules import it. Any other path under `/lib/` is 404, and the client never supplies a directory. A listed file that is missing is 404 with `X-Ocelli-Error: library`. Both roots are read at request time, so the crate builds and its tests run without `npm run build`. The served type is `text/javascript; charset=utf-8`. The page carries `<script type="importmap">{"imports":{"@ocelli/core":"/lib/ocelli-core/index.js","dcmjs":"/lib/dcmjs.js"}}</script>`. This is F-215 option A as F-215 specified it, with its name pattern replaced by the allowlist.
- `page.html`: the window list, the number keys, the region tool, removal of the hard-coded preset buttons and of the page-side CT test as the only guard.

Protocol strings that change:

| Where | Name | Value |
|---|---|---|
| preview request | `X-Ocelli-Window-Index` | Zero-based index, decimal |
| preview reply | `X-Ocelli-Window-Index` | Index used, or empty for a preset, an override or full range |
| preview reply | `X-Ocelli-Window-Count` | Number of file window pairs for the frame |
| error reply | `X-Ocelli-Error: modality` | Preset on a modality other than CT |
| error reply | `X-Ocelli-Error: region` | Empty rectangle after clamping, or an unusable `X-Ocelli-Region` |
| inspect JSON | `presets` | `[{"token":"soft_tissue","name":"Soft tissue","center":40,"width":400}, ...]` on CT, `[]` otherwise |
| inspect JSON | `windows` | `[{"center":40,"width":400,"explanation":"WINDOW1"}, ...]` |
| inspect JSON | `voiLuts` | `[{"explanation":"..."}]` |
| inspect JSON | `windowsPerFrame` | `true` or `false` |
| region request | `POST /region`, `X-Ocelli-File`, `X-Ocelli-Region`, no body | `x0,y0,x1,y1` in image pixels, finite decimals |
| inspect request | `POST /inspect` with `X-Ocelli-File` and `X-Ocelli-Frame`, no body | New id form, F-216's rules |
| library | `GET /lib/ocelli-core/<name>.js`, `GET /lib/dcmjs.js` | Allowlisted JavaScript, or 404 with `X-Ocelli-Error: library` |
| region reply | JSON | `{"min":-1000,"max":40,"count":6}` |

Existing fields keep their names. `windowCenter`, `windowWidth` and `windowFunction` stay the stored text. A request with none of the new headers behaves as today, except that a preset on a non-CT file is now refused.

Non-goals: new preset numbers such as brain or mediastinum (open question 5), user-editable presets, a VOI LUT Sequence drawn by this preview, and a histogram window.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| Window list rows | `inspect_json` `presets`, `windows`, `voiLuts` | Server numbers, server names | Rows and their numbers |
| Preset row or its key | `POST /preview` with `X-Ocelli-Window-Preset`, then `WindowPreset` | CT only, refused otherwise | Grey bytes, or the `modality` error |
| File window row or its key | `POST /preview` with `X-Ocelli-Window-Index`, then `VoiTransform::new(selected)` | Index refused out of range, with a preset, or on a VOI LUT Sequence | Grey bytes and the index used |
| W/L drag, unchanged | Numeric override | Shown function kept | Grey bytes |
| Region tool | `POST /region` by id through F-210's `HeldFrame::value` and `ModalityTransform`, then `windowRegion` from `/lib/ocelli-core/`, then the numeric override | Shown function kept. Unavailable when the library import fails | Grey bytes, the `region` sentence, or the library sentence |
| Bottom right corner | The page window state | F-212 owns the layout | The window word |
| `a_preset_refuses_a_voi_lut_sequence` and other preset tests | `preview_frame` | Builders gain Modality `CT`, so the LUT refusal is the one tested | Unchanged expectations |

Selection is owned by `VoiTransform::new`. Units are modality units from `ModalityTransform`. The page converts nothing.

## State transitions and invariants

| Entry point / starting state | Input or event | Resulting state and output | Refusal and preserved state |
|------------------------------|----------------|----------------------------|-----------------------------|
| CT, file window 0 | Key `2` | Preset lung, grey from -600 and 1500 | None |
| CT, preset lung | Key `5` with two file windows | File index 1, preset cleared | None |
| MR, file window 0 | Key `1` with one file window | File index 0 | Key `4` with one window does nothing |
| MR | Crafted preview with a preset | `modality` error | The page sends none, so the shown image is not touched |
| File index 1, next slice has one window | Arrow key | Index 0 requested, label follows | No fault |
| Any state | Region drag over a single value | `windowRegion` returns null, no request | State and image unchanged, sentence says the region has one value |
| Shown function LINEAR | Region span 0.5 | Server refuses the width under LINEAR | Previous state restored, sentence line shows the refusal |
| File with a VOI LUT Sequence | Index 0 | Window error, as with no index today | Rows listed disabled |
| Multiframe with per-frame windows | Frame change | Inspect reposted for that frame, list replaced | A refused inspect keeps the old list and disables its rows |

Invariant across consumers: the numbers the page shows for a preset, a file window or the applied window come from the local process. The page source contains no window number.

## Ownership, commit and recovery

No device and no file write. The window state is page memory until reload. The commit point for a window change is a successful preview reply. Until it arrives the previous image stays. An identical repeated request returns identical bytes.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | Stored 50, slope 1, intercept 0, LINEAR, centres `40\-600`, widths `400\1500`. Index 0 gives grey 134 and reports centre 40. Index 1 gives grey 238 and reports centre -600 and width 1500. | `a_file_window_index_selects_that_pair` | Drawing index 1 while reporting `first()`, or selecting in the folder viewer | implementation |
| AC2 | The AC1 file with index 2 is the window error. A file with no window and index 0 is the window error. Index with a preset, and index with a numeric centre, are the window error. | `a_file_window_index_is_refused_outside_the_pairs` | Clamping to the last pair, or falling to full range | implementation |
| AC3 | A file with a VOI LUT Sequence and a window pair refuses index 0. The same file with a numeric override still draws, as F-206. | `a_file_window_index_keeps_the_voi_lut_policy` | Letting an index silently drop a LUT | implementation |
| AC4 | Explanation `WINDOW1\WINDOW2` lists both labels with their DS tokens. Explanation `\WINDOW2` lists `Window 1` and `WINDOW2`. One explanation for two pairs lists `WINDOW1` and `Window 2`. Three explanations for two pairs list two rows. A VOI LUT item with LUT Explanation `Bone LUT` is listed in `voiLuts`. | `inspect_lists_every_window_by_its_explanation` | Shifting labels past an empty slot | implementation |
| AC5 | A CT inspect lists three presets whose numbers equal `WindowPreset::center` and `width`. An MR inspect lists `[]`. | `presets_are_offered_on_ct_only` | A second table of numbers | implementation |
| AC6 | An MR with the lung preset is `PreviewError::Modality` with code `modality`. The same builder with Modality `CT` draws, which is the positive control. | `a_preset_on_mr_is_the_modality_error` | A page-only gate | implementation |
| AC7 | `a_preset_refuses_a_voi_lut_sequence` builds a CT, so its refusal is the LUT refusal and not the modality refusal. | That test with its builder changed | A green test refused for the wrong reason | implementation |
| AC8 | An enhanced file with per-frame windows reports `windowsPerFrame` true. A single-frame file reports false. | `windows_per_frame_follows_the_functional_group_source` | Always reposting, or never | implementation |
| AC9 | A 4 by 4 frame with stored `row * 4 + column`, slope 1, intercept -1024. Rectangle `0.5,0.5,2.5,1.5` gives count 6, minimum -1024 and maximum -1018. Rectangle `2.5,1.5,0.5,0.5` gives the same. `-5,-5,-1,-1` is the `region` error. `0.6,0.6,0.9,0.9` holds no centre and is the `region` error. With Pixel Padding Value 0 the first rectangle gives count 5 and minimum -1023. | `region_extent_counts_pixel_centres_in_modality_units` | Stored values instead of modality values, or corners not sorted | implementation |
| AC10 | `POST /file` with the AC9 object, then `POST /region` by id with that rectangle, returns `{"min":-1024,"max":-1018,"count":6}`. A malformed `X-Ocelli-Region` is the header error. A `/region` without `X-Ocelli-File` is the header error. | `the_region_route_returns_the_extent` | Echoing the body, or a route that takes the whole file | implementation |
| AC11 | `windowRegion([-1024, -1018])` and `windowRegion` of the six values give centre -1021 and width 6. | `packages/core/src/manipulation.test.ts`, one added case | Assuming the equivalence without a test | implementation |
| AC12 | The page source contains no `data-preset="lung"`, no `-600`, no `1500`, binds `1` to `9` to the window rows, no longer binds `1` or `2` to the layout buttons, and imports `windowRegion`. | `the_page_offers_a_folder_picker` extended | Numbers or tokens left in the page | implementation |
| AC13 | In a browser on the synthetic CT, keys `1`, `2` and `3` change the corner word to Soft tissue, Lung and Bone. A region drag over lung changes the window. On an MR the keys skip the preset rows. | Click-through at sprint close | A binding that compiles and never fires | integrated, not claimed by Rust tests |
| AC14 | Mutating `selected` to 0 in `lut_chain` turns AC1 red. Mutating the centre test in the region inclusion turns AC9 red. | Recorded in the review | A test that passes on the wrong code | implementation, reviewer |
| AC15 | Before the window list and the index selection are implemented, the reviewer checks the transcriptions above against the DICOM standard text at dicom.nema.org, current edition: PS3.3 C.11.2 (VOI LUT Module attribute table), C.11.2.1.1, C.11.2.1.2 and C.11.2.1.2.2 (the window pair, multiple values as alternative views, VOI LUT against window) and C.11.1.1 (the modality values the region reads). Any correction lands in this plan before the code | Review note naming the edition read and each section | Fixtures citing text nobody checked | reviewer, before implementation |
| AC16 | `library_reply(root, path)` on a temporary root serves a listed `/lib/ocelli-core/index.js` and `/lib/dcmjs.js`, and refuses `/lib/ocelli-core/../x.js`, `/lib/ocelli-core/a.test.js`, an unlisted `/lib/ocelli-core/other.js` and `/lib/other.js` with 404. A listed file absent from the root is 404 `library`. `LIBRARY_FILES` equals the relative import closure of `packages/core/src/index.ts`, read from the tracked sources | `library_routes_serve_only_listed_modules`, `library_files_match_the_index_closure` | A path joined from the request, or a list that drifts from the package | implementation |
| AC17 | The page holds the import map, imports `windowRegion` from `@ocelli/core` dynamically, and contains the library sentence. `/inspect` by id with `X-Ocelli-Frame` returns the same body as the body form | `the_page_offers_a_folder_picker` extended, `inspect_by_id_matches_the_posted_body` | A bare specifier the browser cannot resolve, or a per-frame repost of the whole file | implementation |

AC1 worked, PS3.3 C.11.2.1.2 LINEAR with `ymin` 0 and `ymax` 255. Index 0: `c' = 39.5`, `w' = 399`, `50 > 39.5 - 199.5` and `50 <= 39.5 + 199.5`, so `y = ((50 - 39.5) / 399 + 0.5) * 255 = 134.21`, and `display_index` rounds to 134. Index 1: `c' = -600.5`, `w' = 1499`, `y = ((50 + 600.5) / 1499 + 0.5) * 255 = 238.16`, which is 238.

AC9 worked: the rectangle holds columns 0, 1, 2 and rows 0, 1. The stored values are 0, 1, 2, 4, 5, 6. Modality is `stored - 1024` under PS3.3 C.11.1.1, so the extremes are -1024 and -1018.

## Approach

1. `preview.rs`: add `window_index` to `FrameRequest`, the index refusals beside the existing preset refusals, the `selected` argument, the modality check for presets, and `Modality` on `PreviewError`. `region_extent` sits beside F-210's `value_json` and reads the same held frame, `HeldFrame::value` modality values and padding range. F-210 lands first under the build order and writes that shared read. This story calls it. There is one.
2. `preview.rs` `inspect_json`: `presets` from a local `const CT_PRESETS: [WindowPreset; 3]` and a token function that round-trips with `window_preset_from_token`. `windows` from the resolved centre, width and explanation for the frame. `voiLuts` from the resolved sequence items.
3. `serve.rs`: parse `X-Ocelli-Window-Index` and `X-Ocelli-Region` once each, write the two reply headers, add the `/region` route by id, the `/inspect` id form, `library_reply` with `LIBRARY_FILES`, and the two `/lib/` routes.
4. `page.html`: replace the three buttons with rows built from `info.presets`, `info.windows` and `info.voiLuts`, add the key binding, add the region tool, and import `windowRegion` through the import map, reporting the region tool unavailable when the import fails.
5. `packages/core/src/manipulation.test.ts`: the AC11 case. No change to `manipulation.ts`.

Rejected: a Rust `window_region`, which is a second copy of the F-087 rule. A page-side mid-span, which is a third. Selecting the pair in the folder viewer and passing one centre to `VoiTransform::new`, which is the re-selection the accessor doc forbids.

## Dependencies, write ownership and resources

Declared dependencies F-200 and F-207, both done. Source-level dependency on F-206's preset path, on F-216's held frame and id protocol, on F-210's `HeldFrame::value` in `value.rs`, on F-208's key handling, and on F-212's corner layout, which this story adds a word to.

**Sprint build order, decided in the S46 design round on 2026-11-05.** F-217, F-216, F-210, F-211, F-208, F-209, F-212, then this story, then F-214 and F-215. F-119 is independent. F-215 lands after this story and uses the `/lib/` routes and the import map written here.

Writes `examples/folder-viewer/src/preview.rs`, `src/serve.rs` (including the `/lib/` routes and `LIBRARY_FILES`), `src/lib.rs`, `src/page.html`, `tests/preview.rs`, and `packages/core/src/manipulation.test.ts`. `src/value.rs` is read, not written.

F-208 rebinds the layout keys and also writes `onKey`. F-214 writes the mouse and key preferences. Neither can write `page.html` at the same time as this story. The working tree's uncommitted `page.html` polish is the starting point. No GPU.

## Verification and evidence reuse

`bin/ocelli.sh test ocelli-folder-viewer` and `bin/ocelli.sh check ocelli-folder-viewer` while iterating, and the `@ocelli/core` vitest for AC11. The sprint gate runs once. A later edit to the write set invalidates the record. AC13 is a browser observation at sprint close and is open until someone performs it.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no wasm crossing. `/region` returns three numbers, not samples.
- Render-loop allocation: n/a. This crate does not render on a device.
- unsafe: none
- Tier A (WebGPU): n/a. This page does not open a device.
- Tier B (WebGL2): n/a
- Tier C (CPU): full. The selected window, the preset and the region override all run through `preview_frame`'s CPU `LutChain`.

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| fixture | Selected pair grey, PS3.3 C.11.2.1.2 and C.11.2.1.1 | `examples/folder-viewer/tests/preview.rs`, `a_file_window_index_selects_that_pair` |
| refusal | Index outside the pairs, with a preset, with a numeric window | same, `a_file_window_index_is_refused_outside_the_pairs` |
| refusal | Index on a VOI LUT Sequence, with the numeric override as control | same, `a_file_window_index_keeps_the_voi_lut_policy` |
| contract | Explanation per index, PS3.3 C.11.2 (0028,1055) and (0028,3003) | same, `inspect_lists_every_window_by_its_explanation` |
| contract | Presets on CT only, numbers from `WindowPreset` | same, `presets_are_offered_on_ct_only` |
| refusal | Preset on MR, with the CT control | same, `a_preset_on_mr_is_the_modality_error` |
| contract | Per-frame window source | same, `windows_per_frame_follows_the_functional_group_source` |
| fixture | Region extent in modality units, PS3.3 C.11.1.1 | same, `region_extent_counts_pixel_centres_in_modality_units` |
| integration | The region route by id | same, `the_region_route_returns_the_extent` |
| contract | Library routes, allowlist against the index closure, import map, `/inspect` by id | same, `library_routes_serve_only_listed_modules`, `library_files_match_the_index_closure`, `inspect_by_id_matches_the_posted_body` |
| review | AC15 transcription check | review note, before implementation |
| unit | Two-sample equivalence of `windowRegion` | `packages/core/src/manipulation.test.ts` |
| contract | Page binding and no page numbers | `the_page_offers_a_folder_picker` |

## Parity surface covered

None measured. Appendix B counts the VOI functions and the manipulation tools. The region window diverges from cornerstone3D 5.8.2 `WindowLevelRegionTool` as transcribed above, and this story does not claim that row.

## Deviations

none in this draft. Open question 3 may need one.

## LLD impact

`docs/lld/cache.md`, the kept inspect reply pool outside the budget that the lead ruling after review pass 1 added, and the forget paragraph's bound. Corrected at integration, because this said none for the folder viewer. If open question 3 changes the library rule, `docs/lld/public-api.md` changes with it, in that library story.

## User-facing documentation impact

- `README.md`, "Start the local series viewer". Step 5's "The right column holds the window presets" becomes the window list: on CT the three presets first, then the file's own windows by their explanation, numbered, with keys `1` to `9`, and `0` Fit. The drawing and navigation tools gain the region window. A new sentence says the region window needs `npm run build` before the viewer starts, because the process serves the built `@ocelli/core` under `/lib/`. The paragraph on refusals gains one sentence that a preset on a modality other than CT is refused.
- `examples/folder-viewer/src/use.html`, chapter 4 "Build from source": beside the `cargo run` line for this folder viewer, one sentence that its region window, and Save marks once F-215 lands, load the built `packages/core/dist` from the local process, so `npm run build` comes first.
- `examples/folder-viewer/src/more.html`: no change. No card covers window presets or a region window, and no card's sentence becomes false.

`use.html` joins the write set for this edit.

## Open questions

1. **Number keys.** Affects AC12 and AC13 and F-208. Options: (a) presets and file windows on `1` to `9` as described above, with layouts moved off bare digits, (b) presets on `Shift` and digits, layouts kept, (c) no keys. Owner: operator, answered once with F-208 open question 1, whose option (a) frees `1` to `9` and moves the layout picker to `G` and tile mode to `Shift G`. Recommendation: (a) in both plans. It is the radiology convention in research section 3, and it keeps the CT presets on fixed keys. Deferral blocks AC12 and leaves `1` choosing the layout.
2. **Index on a VOI LUT Sequence.** Affects AC3. Options: refuse, as presets and the library `set_window` do, or allow, as F-206's numeric override does, since PS3.3 C.11.2.1.2.2 treats them as alternative views. Owner: operator. Recommendation: refuse, because the skill names preserving the sequence as project policy and the folder viewer's override is the exception. Deferral leaves AC3 unwritten.
3. **Region rule against cornerstone 5.8.2.** Affects AC9 to AC11 and parity. Options: use the library rule and disclose the divergence, or change the F-087 rule to the mean centre and a 10 floor in a library story first. Owner: operator. Recommendation: use the library rule now, as the story says, and file a library story if parity is wanted. A change to the rule changes `@ocelli/core`'s public behaviour, which is not this story's to make. Deferral blocks nothing, because the wiring is the same.
4. **Where `windowRegion` runs.** Affects AC11 and AC12 and is shared with F-215 open question on how the page reaches `@ocelli/core`. The page has no build step and `packages/core/dist` is ignored by git. Options: (a) F-215's option A, the local process serves `/lib/ocelli-core/<name>.js` from `packages/core/dist` at request time with an import map, and the region tool imports `windowRegion` from `@ocelli/core` and reports unavailable when the import fails, (b) a `build.rs` embeds the built file when it exists, (c) a Rust copy of the rule. Owner: operator, decided once for F-213 and F-215. Recommendation: (a), so there is one route. It changes the crate's rule that it reads no path of its own, and F-215 already records that change. Deferral blocks the region tool.
5. **More presets.** Affects nothing above. Research section 5 lists brain, mediastinum, liver and others, and notes they vary by institution. Adding them changes `WindowPreset` and the preset bytes in the public API. Owner: operator. Recommendation: a library story with cited sources, not this one. Deferral keeps three CT presets.
6. **Transcription check.** The PS3.3 C.11.2 rows above are not in the repository. Affects AC1 to AC4. Owner: reviewer. Deferral leaves the explanation rule citing text nobody has checked.

### Answers

Given by the operator on 2026-11-05 in the S46 consolidated design round and relayed by the sprint lead.

1. Answered by the operator, shared with F-208 Q1: (a). Bare `1` to `9` select window rows. On CT, 1 is soft tissue, 2 lung, 3 bone, then the file's windows. On other modalities the file windows start at 1. `0` stays Fit. `G` opens the layout picker and `Shift G` toggles tile mode, which F-208 binds.
2. Answered: recommended, refuse an index on a file with a VOI LUT Sequence.
3. Answered: recommended, use the library `windowRegion` rule now and disclose the divergence from cornerstone3D 5.8.2. A library story is filed if parity is wanted.
4. Answered by the operator, shared with F-215 Q1: (a). The Rust process serves the built `packages/core/dist` and dcmjs's ES module under `/lib/` from an allowlist of file names, with an import map in the page. When the build is absent, the region tool says so and reads unavailable. This story writes the routes because it lands before F-215. AC16 and AC17 pin it.
5. Answered: recommended, more presets are a library story with cited sources, not this one.
6. Answered: the flag stays, and AC15 makes the reviewer check the C.11.2 rows against dicom.nema.org, current edition, before implementation of that code.

Derived from the build order and F-216: `/region` and the per-frame `/inspect` use F-216's file id protocol, so this story never posts a file after the first upload. No question is left open.

## Lead amendments before implementation, against F-209's tree

Made by the sprint lead on 2026-11-07 from read-only groundwork against `work/f-209-xai` at 3c206f67. Where these differ from the text above, these win. F-213 starts after F-212 lands and re-checks every path and line named here.

1. **Selection lives in `render_held`.** There is no `lut_chain`. The window chosen today is the first pair, passed as index `0`, in `render_held` in `examples/folder-viewer/src/preview.rs`. AC14's mutation target is that index.
2. **The file window index is part of every viewport's state and cache key.** It joins the override and preset in F-209's `cacheKey` and `copyWindow`, per viewport and not page-wide. A test renders index 0 then index 1 on the same frame and shows different grey, red if the index is left out of the key. A linked window copy carries the index.
3. **The region window reuses F-211's region walk.** No `region_extent`. The minimum and maximum come from the same walk F-211's region statistics use, with its pixel-centre rule, padding exclusion and size cap, factored if needed so the walk exists once, and not through `measure_json`, which refuses on spacing. `HeldFrame::value` is a field.
4. **A malformed region header is the `header` error,** as AC10 says. The protocol table is corrected to match. If a new error variant is needed it is added with its own test.
5. **Modality reaches the preset check.** `Recipe` gains Modality. Both `a_preset_refuses_a_voi_lut_sequence` and `a_voi_lut_preset_still_holds_the_frame` use builders without Modality and are updated. A VOI LUT refusal is reported before the modality refusal, and a test pins that order.
6. **Explanations keep their slots.** A new reader returns every Window Center, Window Width and Window Center & Width Explanation slot, empty ones included, so the labels pair with the windows by position per PS3.3 C.11.2.1.2.2. AC4 uses it, with a fixture whose first explanation is empty.
7. **Citations.** The LINEAR function is PS3.3 C.11.2.1.2.1 and the alternative views text is C.11.2.1.2.2. VOI LUT Sequence is VM 1 with one or more items. Output units are given in the C.11.1 attribute table.
8. **README** step numbers, including F-209's window-link sentence, are measured from the tree at implementation. `lib.rs`'s statement that the crate reads no path of its own is corrected if F-213 touches `/lib/`.
