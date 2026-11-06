# F-210, folder viewer value at the pointer

**Status**: approved
**Epic ref**: E38.20
**Sprint**: S46
**Estimate**: 3w

## Normative source, transcribed

`docs/hld/15-lut-chain.md` section 18, opening paragraph:

```text
This is the highest-risk arithmetic in the project. It is specified in DICOM PS3.3 C.11 and the stages apply strictly in order. Implement it once, in ocelli-pixel, and let the shader read the parameters — do not let a second copy of this logic appear anywhere.
```

Section 18 stage table, row 1:

| **Stage** | **From → To** | **Source** |
|----|----|----|
| 1\. Modality LUT | Stored → Modality | Rescale slope/intercept, or a Modality LUT Sequence which takes precedence |

Section 18.1, character by character:

```rust
pub fn modality(sv: Stored, slope: f32, intercept: f32) -> Modality {
    Modality(sv.0 * slope + intercept)
}
```

```text
*If a Modality LUT Sequence is present it wins over slope and intercept. PET SUV is a separate path and needs the radiopharmaceutical sequence — do not fold it in here.*
```

The implementation of that section is `ocelli_pixel::modality` at `crates/ocelli-pixel/src/lut.rs:199`, `Modality(stored.0 * slope + intercept)`, and `ModalityTransform::new` and `ModalityTransform::apply` at `lut.rs:222` and `lut.rs:240`. `apply` uses the LUT when one was selected and `modality` otherwise. `ModalityTransform::map_into` at `lut.rs:277` maps a slice without allocating.

`docs/hld/07-concurrency-and-typescript.md` section 10, which keeps the pointer in the shell and the arithmetic out of it:

```text
- DOM, pointer, touch and wheel events; canvas lifecycle; ResizeObserver
- Tool interaction state machines; the geometry and hit-testing behind them is Rust, in ocelli-geom
```

`docs/hld/11-decision-log.md` decision D3: pixels never cross the boundary. That decision is about the wasm boundary. This page is not on it. The spirit is kept anyway: the page receives one number per request and never a buffer of modality values.

From the DICOM standard. There is no local copy of the standard in this repository, so these lines are transcribed from the published text without a copy at hand, which is a transcription from memory. The flag stays. AC20 makes the reviewer confirm them against the current edition at dicom.nema.org before the unit rule and the fixtures are implemented (open question Q5).

- PS3.3 C.11.1, Modality LUT Module. Rescale Intercept (0028,1052), Rescale Slope (0028,1053) and Rescale Type (0028,1054). The output units are `m*SV + b`, where `m` is the slope and `b` the intercept.
- PS3.3 C.11.1.1.2, Rescale Type. It "specifies the output units of Rescale Slope and Rescale Intercept". Defined terms: `OD` optical density, `HU` Hounsfield Units, `US` unspecified, `MGML` mg/ml, `Z_EFF` effective atomic number, `ED` electron density, `EDW` electron density normalised to water, `HU_MOD` modified Hounsfield unit, `PCT` percentage.
- PS3.3 C.8.2.1, CT Image Module. Rescale Type (0028,1054) is Type 1C, "Required if the Rescale Type is not HU (Hounsfield Units). May be present otherwise." So on CT an absent Rescale Type means HU.
- PS3.3 C.7.6.3.1, Pixel Representation and Bits Stored. A stored sample is masked to Bits Stored and a signed sample is sign extended from bit `BitsStored - 1`. `StoredPixelDescription::unpack` at `crates/ocelli-pixel/src/stored_pixel.rs:271` implements it and cites the section.
- PS3.3 C.7.5.1.1.2, Pixel Padding Value (0028,0120) and Pixel Padding Range Limit (0028,0121). `OpenedFrame::padding` and `PaddingRange::contains` in `crates/ocelli-dicom/src/frame_open.rs` implement the inclusive range in stored space, before the modality stage.

The library's own frame-open reader, F-191, is `ocelli_dicom::open_frame(parsed, frame)` at `frame_open.rs:336`. It returns `OpenedFrame`, whose `modality()` is the constructed stage 1 and whose `padding()` is the stored padding range. Its private `rescale_or_identity` makes an absent slope and intercept the identity and refuses one half of the pair without the other. It does not read Modality (0008,0060) or Rescale Type (0028,1054).

## What the specification does not cover

The HLD does not name this page, a value route, a unit label, or a pixel-under-pointer rule. The decisions below are this plan's.

The unit rule. The HLD does not give one and `open_frame` does not read the two attributes it needs. The rule, in this order, from the attribute resolved for the requested frame:

| Rescale Type for this frame | Modality (0008,0060) | Unit token | Unit source |
|---|---|---|---|
| absent or zero length | `CT` | `HU` | `ct-default` |
| `HU` | any | `HU` | `rescale-type` |
| `US` | any | `unspecified` | `rescale-type` |
| any other non-empty code | any | that code verbatim | `rescale-type` |
| absent or zero length | `PT`, with Units (0054,1001) present and non-empty | that `Units` code verbatim, such as `BQML` | `units` |
| absent or zero length | not `CT`, and not a `PT` with Units | `unspecified` | `none` |

Rescale Type is resolved through the Pixel Value Transformation Sequence (0028,9145) first and the top level second, the same `TopLevelFallback::Allowed` resolution the preview uses for slope and intercept. A Modality LUT Sequence is refused before the unit is read, the same as the preview, so Modality LUT Type (0028,3004) is not consulted. PET `Units` (0054,1001) is read for Modality `PT` only, under the operator's answer to Q3, and no SUV is computed.

The pixel under the pointer. Page marks and the pointer are in image-pixel coordinates where cell `c` covers `[c, c + 1)` and its centre is `c + 0.5`. This is the coordinate the page already uses for `C` and `R` in `pointReadout`. The server, not the page, chooses the cell. For `0 <= x <= width` the column index is `floor(x)`, and `x == width` is the last column `width - 1`. Rows are the same. A point outside that closed range is refused. The reply carries the zero-based indices it used and the one-based column and row the page prints.

Padding. A sample inside the padding range is reported as padding with no number. The page prints `padding`. This plan does not invent a modality value for a sample the standard says is not part of the image. The stored value is compared as an integer through a checked conversion. `Stored` is `f32`, which is exact for integers up to `2^24`, so a padded object with Bits Stored above 24 is refused for the value rather than compared inexactly.

Number formatting. The reply carries the modality value as a JSON number written with Rust's shortest round-trip `Display` for `f32`, and a `text` field of that number, a space, and the unit token. The page prints `text` and does no arithmetic on it.

Transport. `/value` uses F-216's file id protocol and nothing else. The page posts each file once to `POST /file`, keeps the `X-Ocelli-File` id, and every `/value` request carries that id with `Content-Length: 0`. The process reads the frame through `Viewer::frame(id, frame)`, which returns the held frame or decodes it from the held bytes once and inserts it. A pointer move never posts the file. This is the operator's answer to Q1.

## Problem and reproducer

Run `cargo run -p ocelli-folder-viewer`, open Synthetic study, turn on the pointer cross with `C`, and hover the body at column 65, row 49. The readout is `C 65  R 49`. There is no value. The sentence for the cross says `The numbers are the column and row under it, counted from 1.` A reader cannot tell air from bone without the window. Every viewer in the professional survey, section 10 row 5, shows the modality value under the cursor as a baseline feature.

The strings that change, and what replaces them:

| Where | Current text | After F-210 |
|---|---|---|
| `page.html` `SENTENCES["toggle-cross"]` | `Two lines meet at the pointer. The numbers are the column and row under it, counted from 1.` | `Two lines meet at the pointer. The numbers are the column and row under it, counted from 1, and the modality value of that pixel, which is Hounsfield units on a CT. The local process computes the value.` |
| `page.html` `pointReadout` | `read.textContent = "C " + column + "  R " + row;` | the same `C` and `R`, then the reply's `text`, or `padding`, or nothing while no reply for that cell has arrived |
| `.claude/plans/F-207-design.md` table row `Pointer cross` | `The numbers are the column and row under it, counted from 1.` | not edited. F-207 is approved history. This plan supersedes that row. |
| root `README.md` step 7 | no sentence about the cross | one sentence: the cross shows the column, the row and the modality value, HU on CT |
| Operator protocol, pointer cross line | `shows a column and a row, not a Hounsfield value` | `shows a column, a row and the modality value with its unit, HU on CT`. This is a deliberate protocol change, decided by the operator on 2026-11-05, and the operator's protocol is updated with it |

## Scope, API and compatibility

In scope:

- The held frame of F-216 gains the request-independent value inputs. `HeldFrame` gets one field, `value: Result<ValueFacts, PreviewError>`, filled by `decode_held` from the parsed object: the `ModalityTransform` from `open_frame(..).modality()`, the stored padding range from `padding()`, and the unit and its source from `modality_unit`. A Modality LUT Sequence is stored as the `window` refusal, in the same precedence as the preview's. `render_held` does not read the field, so `preview_frame`'s output does not change.
- `fn value_json(held: &HeldFrame, points: &[ImagePoint]) -> Result<String, PreviewError>` in a new module `examples/folder-viewer/src/value.rs`, used by the route. `pub fn value_frame(bytes: &[u8], frame: u32, points: &[ImagePoint]) -> Result<String, PreviewError>` is its composition with `decode_held`, exactly as `preview_frame` composes `decode_held` and `render_held`, and is re-exported from `lib.rs` so every fixture runs through the function the route uses. `ImagePoint` is `{ x: f64, y: f64 }` in page image-pixel coordinates.
- The unit rule above as a private `modality_unit` in the same module.
- `POST /value` in `serve.rs`.
- The page cross readout and its sentence.

Route shape. The existing request header parser stays, with F-216's `X-Ocelli-File` rule. One header is added.

```text
POST /value
X-Ocelli-File: <id from POST /file>
X-Ocelli-Frame: 0
X-Ocelli-Points: 64.5,48.5 0.25,0.25
Content-Length: 0
```

`/value` has no body form. A request without `X-Ocelli-File`, or with an id and a non-empty body, is 400 with code `header`. An id that is unknown, evicted or from another process is F-216's 410 `file-gone`, and the page re-posts the file once through the shared `ensureFile` and retries once. A file the process did not hold (`X-Ocelli-Held: 0`) gets no `/value` request, and the readout prints `C` and `R` and the words `not held`. The reply carries `X-Ocelli-Source: held` or `decoded`, as `/preview` does under F-216.

`X-Ocelli-Points` is a space-separated list of `x,y` pairs, at most 64 pairs. Each number must parse as a finite decimal. A second `X-Ocelli-Points` header, an empty list, a malformed pair, or more than 64 pairs is a 400 with error code `points`. The header is ignored on every other route.

Reply, 200, `application/json`:

```json
{"frame":0,"unit":"HU","unit_source":"ct-default","points":[
  {"column":65,"row":49,"index_column":64,"index_row":48,"padding":false,"value":40,"text":"40 HU"},
  {"column":1,"row":1,"index_column":0,"index_row":0,"padding":false,"value":-1000,"text":"-1000 HU"}]}
```

A padded sample is `{"column":..,"row":..,"index_column":..,"index_row":..,"padding":true,"text":"padding"}` with no `value`.

Refusals use the existing `text_reply` with an `X-Ocelli-Error` code and a sentence. They reuse `PreviewError` codes where one fits: `not-part10`, `attribute`, `not-monochrome`, `too-large`, `decode`, `frame`, and `window` for a Modality LUT Sequence. One new variant, `PreviewError::Point`, code `point`, sentence `the point is outside this image`, covers a point outside `0..=width` or `0..=height`. A refusal returns no partial list.

Non-goals: SUV, a value for colour frames, a value in a 2 by 2 tile, a Modality LUT Sequence, the VOI or display value, a value cache, and any change to `/preview`, `/inspect` or `/headers` output.

Compatibility. The page keeps working against a process without `/value`. A 404 from `/value` leaves the readout as `C` and `R` only.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| Pointer cross, single view | `POST /value` by `X-Ocelli-File` id, then `Viewer::frame` and `value_json` | One request in flight, no body. On completion the latest pointer cell is sent if it changed. A 410 re-posts once | `C n  R n  <text>` |
| `value_json` stage 1 | `HeldFrame::value`, filled once by `decode_held` from `ocelli_dicom::open_frame(..).modality()`, then `ModalityTransform::apply` | Modality LUT Sequence refused before `open_frame` is consulted, same rule as `lut_chain` in `preview.rs` | `Modality(f32)` |
| `value_json` stored sample | `HeldFrame::samples` through the same `StoredPixelDescription::unpack` that `render_held` uses, on the requested samples only | The same decode as `preview_frame`, held by F-216 | `Stored(f32)` |
| `value_json` padding | `OpenedFrame::padding()` and `PaddingRange::contains` | Stored-space comparison through a checked integer conversion | `padding: true` |
| `value_json` unit | `modality_unit` in `value.rs` | The table above | `unit`, `unit_source` |
| `preview_frame` | F-216's `decode_held` then `render_held` | unchanged output | the same grey bytes |
| F-211 probe tool | `POST /value` by id | One request per probe set on a frame | the same reply |

The owner of the unit, the rounding and the cell selection is `value.rs`. The page owns none of them.

## State transitions and invariants

| Entry point / starting state | Input or event | Resulting state and output | Refusal and preserved state |
|------------------------------|----------------|----------------------------|-----------------------------|
| Cross on, single view, no request in flight | Pointer enters cell (64, 48) | One `/value` request. On reply the readout gains `40 HU`. | A refusal leaves `C` and `R` and shows no value |
| Request in flight for cell A | Pointer moves to B, then C | No second request. On A's reply, A's value is dropped because the cell changed, and one request for C is sent. B is never requested. | A reply whose `index_column` and `index_row` differ from the current cell is not shown |
| Cross on | Slice change | The pending reply is dropped by a generation counter, the readout clears, and the next move requests the new frame | A reply for the old frame is never shown on the new one |
| Cross on | Preset, W/L drag, invert | No new request. The modality value does not depend on the window. | The shown value stays |
| Cross on | 2 by 2 view | No request. The cross is hidden in the grid already. | |
| Cross on, id evicted or process restarted | Next `/value` | 410 `file-gone`, one re-post through `ensureFile`, one retry | A second 410 or a failed re-post leaves `C` and `R` and shows no value |

Invariant across consumers: the value for a cell is a function of the file, the frame and the cell only. The window, invert, rotation, flips, zoom and pan never change it.

## Ownership, commit and recovery

The server holds no state of its own for this route. It reads F-216's `Viewer`, and a frame miss decodes and inserts through the same path `/preview` uses, so a later `/preview` or `/value` on that frame is served from the held frame. The page holds one in-flight promise and one generation number, and reuses F-216's id map. A failed request does not alter the grey image or the marks. A duplicate request for the same cell returns the same reply.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | Signed 12-bit in 16 (BitsStored 12, HighBit 11, PixelRepresentation 1), Modality CT, slope 1, intercept -1024, raw container `0x0FFF` at (0, 0) gives stored -1 and value -1025, text `-1025 HU`. PS3.3 C.7.6.3.1 and C.11.1. | `value_signed_12_bit_ct_is_minus_1025` | `as i16` without sign extension from bit 11, which gives 4095 and 3071 | implementation |
| AC2 | Same object, raw `0xF7FF` at (1, 0). The mask keeps `0x7FF`, stored 2047, value 1023. PS3.3 C.7.6.3.1. | `value_masks_bits_above_bits_stored` | No mask, which reads the high nibble | implementation |
| AC3 | Modality MR, slope 2, intercept -1000, unsigned stored 300 gives -400, unit `unspecified`, source `none`. PS3.3 C.11.1. | `value_mr_slope_two_is_minus_400_unspecified` | Labelling every rescaled value HU | implementation |
| AC4 | Modality MR, slope 0.5, intercept 0.25, stored 3 gives 1.75, text `1.75 unspecified`. Exact in `f32`. | `value_fractional_rescale_keeps_two_decimals` | Integer rounding of the value | implementation |
| AC5 | No slope and no intercept, Modality OT, stored 7 gives 7, unit `unspecified` | `value_without_rescale_is_the_stored_value` | Refusing an absent pair, or inventing an intercept | implementation |
| AC6 | CT with Rescale Type `HU_MOD` reports `HU_MOD` and source `rescale-type`. CT with Rescale Type `US` reports `unspecified`. CT with Rescale Type absent reports `HU` and `ct-default`. PT with no Rescale Type and Units `BQML` reports `BQML` and source `units`. PT with neither reports `unspecified` and `none`. | `value_unit_follows_rescale_type` | Forcing HU on every CT, or hiding a unit the PET file states | implementation |
| AC7 | Enhanced CT whose Pixel Value Transformation Sequence for frame 1 has slope 1, intercept -1000 while the top level has intercept -1024 reports the functional group value for frame 1 | `value_uses_the_per_frame_rescale` | Reading the top-level intercept only | implementation |
| AC8 | Pixel Padding Value -2000 on a signed CT, stored -2000 at (0, 0) reports `padding: true` and no `value` | `value_reports_padding_without_a_number` | Reporting -3024 HU for a padded sample | implementation |
| AC9 | A Modality LUT Sequence returns 400 code `window`, the same refusal as `/preview` | `value_refuses_a_modality_lut_sequence` | Applying rescale while a sequence is present | implementation |
| AC10 | Point x = width maps to the last column. Point x = width + 0.01 is 400 code `point`. Point x = -0.01 is 400 code `point`. | `value_closed_right_edge_and_outside_refusal` | Clamping an outside point silently, or `floor(width)` reading past the row | implementation |
| AC11 | Synthetic slice 0 through the HTTP route, posted once to `POST /file` and then read by id: (0.5, 0.5) is -1000 HU, (64.5, 48.5) is 40 HU, (46.5, 48.5) is -800 HU, (72.5, 48.5) is 700 HU. Stored is HU + 1024 with intercept -1024. | `value_route_reads_the_synthetic_named_pixels` | A value computed from the grey byte | integrated |
| AC12 | For every synthetic slice, and for each test object in `tests/preview.rs` that previews, the stage 1 transform in `HeldFrame::value` equals the one `render_held` uses (`ModalityTransform` is `PartialEq`) | `value_and_preview_agree_on_stage_one`, a unit test in `src/value.rs` because `HeldFrame` is not exported | Two rescale readers that disagree on a functional group | integrated |
| AC13 | 65 points, an empty list, a pair `1,`, or a non-finite number is 400 code `points` | `value_points_header_is_bounded` | Accepting an unbounded list | implementation |
| AC14 | Window override headers on a `/value` request do not change the value | `value_ignores_window_headers` | Mapping through VOI | implementation |
| AC15 | The page contains `fetch("/value"` with an `X-Ocelli-File` header and no `body:` in that call, the new cross sentence, and a stale-cell check on the reply before it writes the readout | `the_page_offers_a_folder_picker` extended | A page that posts `image.file` to `/value`, computes `stored * slope + intercept`, or drops the stale check | implementation |
| AC16 | Hover the synthetic body in Chrome with the cross on and read `C 65  R 49  40 HU` | Manual observation at sprint close | none claimed by Rust tests | external, sprint close |
| AC17 | Optional independent check: for three corpus CT rows, pydicom's `apply_modality_lut` at three pixels each equals the route value | Run by hand with the `dicom-tooling` skill | Shared wrong reader on both sides | external, open until run |
| AC18 | `/value` without `X-Ocelli-File` is 400 `header`. `/value` with an id and a non-empty body is 400 `header`. An evicted id and an id from another `Viewer` are 410 `file-gone` | `value_route_takes_only_a_held_id` | A route that still accepts the whole file per pointer move | implementation |
| AC19 | Two `/value` requests on one id and frame report `X-Ocelli-Source` `decoded` then `held`, and a `/preview` by id on that frame afterwards reports `held` | `value_on_a_held_frame_does_not_decode_again` | A route that decodes per pointer move, or a second cache beside F-216's | implementation |
| AC20 | Before the unit rule and the fixtures are implemented, the reviewer checks every transcription above against the DICOM standard text at dicom.nema.org, current edition: PS3.3 C.11.1 (Modality LUT Module), C.11.1.1.2 (Rescale Type defined terms), C.8.2.1 (the CT Image Module Type 1C condition on Rescale Type), C.7.6.3.1 (Pixel Representation and Bits Stored), C.7.6.16.2.9 (Pixel Value Transformation), C.7.5.1.1.2 (Pixel Padding Value and Range Limit) and C.8.9.1 (PET Image Module, Units). Any correction lands in this plan before the code | Review note naming the edition read and each section | Fixtures that encode a condition nobody checked | reviewer, before implementation |

AC1 to AC10 and AC13 to AC14 are hand-computed fixtures. Each test comment cites the PS3.3 section named in its row.

## Approach

1. In F-216's `decode_held`, fill `HeldFrame::value`: refuse a Modality LUT Sequence with the same `group_has` check `lut_chain` uses, call `open_frame` for `modality()` and `padding()`, read Modality and Rescale Type, and resolve the unit with `modality_unit`. `render_held` and the 2 by 2 and thumbnail paths do not change.
2. Add `value.rs`. `value_json` validates the points, reads `HeldFrame::value` and returns its refusal if it holds one, then for each point chooses the cell, unpacks that one stored sample from `HeldFrame::samples` with the same `unpack`, checks padding, and calls `ModalityTransform::apply`. `value_frame` composes `decode_held` and `value_json` for the fixtures. It formats the JSON with the existing `json_string` and `push_field` helpers.
3. Add `PreviewError::Point`.
4. Add the header and the route in `serve.rs`, reading the id through F-216's `parse_header` and `Viewer::frame`, and writing `X-Ocelli-Source`.
5. Page. `pointReadout` keeps its `C` and `R`. It calls `requestValue(cell)`, which takes the id from F-216's `ensureFile` and follows the state table. The reply is shown only when its indices equal the current cell and its generation equals the current one.

Rejected: shipping a `Float32Array` of modality values with `/preview` and reading it in the page. It moves the chain's output into the page and invites page arithmetic. Rejected: computing the value from the grey byte, which is the display value and not the modality value. Rejected: using the preview's local `rescale` for the value. `open_frame` is the library reader F-191 built for this. AC12 checks that the two agree.

## Dependencies, write ownership and resources

Declared: F-207 and F-191, both done. Source-level: `ocelli-dicom::open_frame`, `ocelli-pixel::ModalityTransform`, `StoredPixelDescription`, and F-216's `Viewer`, `HeldFrame`, `decode_held` and file id protocol, which must merge first.

**Sprint build order, decided in the S46 design round on 2026-11-05.** F-217, then F-216, then this story, then F-211, F-208, F-209, F-212, F-213, F-214 and F-215. F-119 is independent and may run in parallel.

Writes `examples/folder-viewer/src/preview.rs`, `src/held.rs` (the `value` field of `HeldFrame`), `src/value.rs` (new), `src/lib.rs`, `src/serve.rs`, `src/page.html`, `tests/value.rs` (new), `tests/preview.rs` (page contract test only), and the root `README.md` step 7. No library crate is written.

`page.html` and `serve.rs` are also written by F-208, F-211, F-213, F-214 and F-215 this sprint. F-210 runs before F-211, which extends `serve.rs` and the same readout. No GPU and no browser exclusivity.

## Verification and evidence reuse

`bin/ocelli.sh check ocelli-folder-viewer` in the inner loop. Then `cargo test -p ocelli-folder-viewer`. Mutation check for the fixture rows: change `stored.0 * slope + intercept` to `stored.0 * slope - intercept` in `ocelli_pixel::modality` locally and confirm AC1, AC3 and AC11 go red, then restore. The sprint profile is the later whole-sprint run. A change to `preview.rs` decode, `held.rs`, `open_frame`, or `modality` invalidates this evidence.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no wasm crossing. The page receives one number per point and never a modality buffer.
- Render-loop allocation: none. A `/value` request reads the held frame and allocates only its reply. A frame miss decodes once through F-216's permit scratch.
- unsafe: none
- Tier A (WebGPU): n/a. The value is host arithmetic. No device opens.
- Tier B (WebGL2): n/a
- Tier C (CPU): n/a. The value is not drawn. The grey image keeps F-206's CPU LUT.

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| fixture | AC1, AC2 sign extension and mask, PS3.3 C.7.6.3.1 then C.11.1 | `examples/folder-viewer/tests/value.rs` |
| fixture | AC3, AC4, AC5 rescale arithmetic, PS3.3 C.11.1 | same |
| fixture | AC6 unit rule, PS3.3 C.11.1.1.2 and C.8.2.1 | same |
| fixture | AC7 per-frame rescale, PS3.3 C.7.6.16.2.9 Pixel Value Transformation | same |
| fixture | AC8 padding, PS3.3 C.7.5.1.1.2 | same |
| refusal | AC9, AC10, AC13 | same |
| integrated | AC11, AC12, AC14 through the HTTP route | same |
| contract | AC15 page strings | `examples/folder-viewer/tests/preview.rs` |
| refusal | AC18 id-only route | `examples/folder-viewer/tests/value.rs` |
| integrated | AC19 held reuse | same |
| review | AC20 transcription check | review note, before implementation |

Existing coverage stays: `crates/ocelli-pixel/tests/modality.rs` `rescale_uses_the_hand_computed_slope_and_intercept` is the library fixture for `modality`.

## Parity surface covered

none. Appendix B counts tool classes and has no per-tool row. cornerstone3D's `Probe` tool is the nearest name and this story does not measure parity against it.

## Deviations

none

## LLD impact

none. The folder viewer is a local example and not the published API. `docs/lld/frame-open.md` is unchanged because `open_frame` is unchanged.

## User-facing documentation impact

- `README.md`, "Start the local series viewer", step 7 (already in the write set): one sentence that the pointer cross shows the column, the row and the modality value with its unit, HU on CT, `padding` for a padded sample, and `not held` for a file the process could not hold.
- `examples/folder-viewer/src/use.html`: no change. No library export is added, and the unit rule stays in the example under Q4.
- `examples/folder-viewer/src/more.html`: no change. No feature card covers a value readout, and no card's sentence becomes false.

## Open questions

Q1. Transport cost. Affects AC16 and the page state table. The plan posts the whole file for every pointer cell. On the synthetic study that is 24 KiB. On a 1 GiB multi-frame object it is not usable. Options: (a) stateless as planned, with one request in flight, (b) a bounded process cache keyed by a ticket that `/preview` returns, so `/value` posts no body, (c) stateless now and measure the round trip on the largest corpus row before deciding. Owner: operator. Recommendation (c), with a threshold of 50 ms per reply on a release build. Deferral cost: large multi-frame files have a slow readout.

Q2. Stage 1 provider. Affects AC12. The plan takes stage 1 from `open_frame` and keeps the preview on its local `rescale`. If AC12 finds a disagreement, options are (a) switch the preview to `open_frame` in this story, which changes its refusal surface, or (b) keep both and refuse `/value` when they differ. Owner: operator. Recommendation (a) if AC12 fails, otherwise no change.

Q3. PET units. Affects AC6. A PT object usually has no Rescale Type and its modality values are in `Units` (0054,1001), often `BQML`. Options: (a) `unspecified`, as the brief states, (b) read `Units` for PT and report it, with no SUV. Owner: operator. Recommendation (b), because `unspecified` on a BQML image hides a unit the file states. Deferral cost: PET reads `unspecified`.

Q4. Unit rule placement. The rule lives in the example. F-201's measurement document and a later `@ocelli/core` probe will want the same rule. Options: (a) the example now, (b) a `modality_unit` accessor on `OpenedFrame` in `ocelli-dicom`, which writes a library crate and `docs/lld/frame-open.md`. Owner: operator. Recommendation (a) for this sprint and a backlog row for (b).

Q5. Standard transcription. The PS3.3 C.11.1.1.2 defined terms, the C.8.2.1 Type 1C condition, and the section numbers C.7.6.3.1 and C.7.6.16.2.9 cited by the fixtures were transcribed without a local copy of the standard. The reviewer confirms both against the current edition before approval. Owner: reviewer. Deferral cost: AC6 could encode a wrong condition.

### Answers

Given by the operator on 2026-11-05 in the S46 consolidated design round and relayed by the sprint lead.

- Q1. Answered by the operator, by the cross-story decision: option (b) in F-216's form, not a ticket of this story's own. `/value` uses F-216's file id protocol, upload once to `POST /file` and read by `X-Ocelli-File` with no body, and F-216 lands before this story. The route spec, the consumer map, the state table, AC11, AC12, AC15, AC18 and AC19 say so. The arithmetic and the fixtures AC1 to AC10 are unchanged.
- Q2. Answered: recommended (a) if AC12 fails, switch the preview to `open_frame` in this story, otherwise no change.
- Q3. Answered: recommended (b), read `Units` (0054,1001) for PT and report it, with no SUV. The unit rule table gains a PT row reading `Units` verbatim, source `units`, before the `none` row. AC6 gains a PT `BQML` case. PS3.3 C.8.9.1, the PET Image Module row that defines `Units`, is in AC20's list.
- Q4. Answered: recommended (a), the unit rule stays in the example this sprint, and a backlog row for an `OpenedFrame::modality_unit` accessor in `ocelli-dicom` is filed by the sprint lead at `/complete-feature`.
- Q5. Answered: the flag stays, and AC20 makes the reviewer check the transcription against dicom.nema.org, current edition, before implementation of the unit rule and the fixtures.

Protocol change from the same round: the pointer cross now shows the modality value and its unit, which deliberately changes the operator protocol line "shows a column and a row, not a Hounsfield value". It is named in the table under Problem and reproducer. No question is left open.

### Lead rulings and corrections after review pass 1

Made by the sprint lead on 2026-11-06 under the operator's delegation of the quality call, after `.claude/reviews/F-210-working-pass-1.md`, which checked the transcriptions above against PS3.3 2026d as AC20 asks. Where this section and the text above differ, this section holds.

- **Corrected transcriptions.** PS3.3 C.8.2.1 makes Rescale Type required if the Rescale Type is not HU, or Multi-energy CT Acquisition (0018,9361) is YES. Units (0054,1001) for PET is described in C.8.9.4, the PET Image Module, and C.8.9.1 is the PET Series Module. The sentence quoted for Rescale Type is the C.11.1 table's, and C.11.1.1.2 says "Specifies the units of the output of the Modality LUT or rescale operation." Masking above Bits Stored and sign extension from Bits Stored minus 1 are cited to PS3.5 8.1.1 and PS3.3 C.7.6.3.3, not C.7.6.3.1.
- **A multi-energy CT without Rescale Type has no assumed unit.** When Multi-energy CT Acquisition is YES and Rescale Type is absent, the unit is reported as unspecified, not HU.
- **On PET, Units decides the unit.** For PT, Units (0054,1001) is reported when present, whatever Rescale Type says, per C.8.9.4.
- **The value depends only on the file, the frame and the cell.** Stage 1 is taken from the held frame's modality recipe, which AC12 proves equal to the preview's, so a window, presentation, lossy or burned-in attribute the value never reads cannot refuse it.
- **The value's facts are charged to the cache budget.** Their heap bytes count in the held frame's bytes like every other owned buffer.
