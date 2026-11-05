# F-206, Folder viewer synthetic study, spacing, edge letters and preset window

**Status**: approved
**Epic ref**: E38.16
**Sprint**: S45
**Estimate**: 5w

## Normative source, transcribed

From `docs/hld/15-lut-chain.md` section 18. The chain exists once, in `ocelli-pixel`. Stages apply in order. A second copy is the defect.

```text
// PS3.3 C.11.2.1.2 -- LINEAR. Requires w >= 1.
// c' = c - 0.5 ; w' = w - 1
// x <= c' - w'/2 -> ymin
// x > c' + w'/2 -> ymax
// else y = ((x - c') / w' + 0.5) * (ymax - ymin) + ymin
```

Section 18.3 names the soft-tissue window as centre 40 and width 400. At that centre, LINEAR and LINEAR_EXACT differ by 0.32 of 255.

`WindowPreset` in `crates/ocelli-viewport/src/presentation.rs` already stores those numbers and lung `-600/1500` and bone `300/1500`. Each preset is `LINEAR`. `apply_preset` returns `PresentationError::Sequence` when stage 2 is a VOI LUT Sequence or stage 1 is a Modality LUT Sequence. `center` and `width` are private.

PS3.3 C.7.6.2.1.1 stores Pixel Spacing as adjacent row spacing, then adjacent column spacing. PS3.3 C.7.6.1.1.1 gives the patient direction of the row and of the column from Image Orientation (Patient). The positive X axis is the patient's left. Positive Y is posterior. Positive Z is superior.

## What the specification does not cover

The HLD does not describe this local page, a synthetic phantom, or edge-letter strings. Those choices are below. They do not add a LUT formula.

- The phantom is one CT series of 12 slices, 96 rows by 128 columns, explicit VR little endian, MONOCHROME2, 16-bit signed, slope 1, intercept -1024, file window 40/400 LINEAR, axial orientation `1\0\0\0\1\0`, Pixel Spacing `0.7\0.5`, Image Position z = `2.5 * k`.
- Pixel values are air -1000, body ellipse 40, two lung regions -800, and a bone ring 700. The ellipse radius changes with the slice index. There is no (0010,0010) and no (0010,0020). Series Description is `Synthetic phantom`. UIDs start with `2.25.`.
- Edge letters use the dominant axis of each cosine. The opposite edge is the opposite letter. A missing or non-unit orientation yields four empty strings.
- Spacing source is `PixelSpacing`, else `ImagerPixelSpacing`, else empty. The page must not parse the DICOM spacing string.

## Problem and reproducer

The folder viewer can open only a posted file. Nothing in the repository builds a Part 10 object that preview will decode and that carries spacing, orientation, and a window. `examples/example-server` builders have no Pixel Data, or they are `pub(crate)`. The test builders in `examples/folder-viewer/tests` are not reachable from `src`.

`inspect_json` does not report spacing or orientation. A numeric window override replaces a VOI LUT Sequence. A named preset must not.

## Scope, API and compatibility

In scope:

- `examples/folder-viewer/src/synthetic.rs` builds the 12 objects in memory.
- `GET /synthetic` returns `{"slices":12}`.
- `GET /synthetic/<n>` returns that slice as Part 10. Any other index is 404.
- `WindowPreset::center` and `WindowPreset::width` become public. No second table of numbers.
- `FrameRequest` gains `window_preset`. Header `X-Ocelli-Window-Preset` is `soft_tissue`, `lung`, or `bone`.
- A preset reruns `LutChain` with that centre, that width, and `LINEAR`. A VOI LUT Sequence or a Modality LUT Sequence refuses it with the existing window error. A preset together with `X-Ocelli-Window-Center` or `X-Ocelli-Window-Width` also refuses.
- `inspect_json` and the preview response gain `rowSpacing`, `columnSpacing`, `spacingSource`, and `edgeLeft`, `edgeRight`, `edgeTop`, `edgeBottom` for the requested frame. Preview sends them as `X-Ocelli-Row-Spacing`, `X-Ocelli-Column-Spacing`, `X-Ocelli-Spacing-Source`, and `X-Ocelli-Edge-Left`, `X-Ocelli-Edge-Right`, `X-Ocelli-Edge-Top`, `X-Ocelli-Edge-Bottom`.
- Inspect reads `X-Ocelli-Frame`, default 0.

Out of scope: the page chrome, gestures, marks, flip, rotate, `/more`, `/use`, overlay, shutter, and slice blending. F-207 draws those. The numeric window override keeps its current VOI LUT behaviour. This story does not retune it.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| `GET /synthetic/<n>` | `synthetic::slice` | Loopback only. No path is read. | Part 10 bytes |
| `POST /preview` with the preset header | `WindowPreset` then `LutChain::map_into` | Refuses a LUT sequence | Grey bytes and the preset centre and width |
| `POST /inspect` and `POST /preview` | One geometry helper over the same multiframe read as the window | Frame header, default 0 | Spacing numbers and four letters |

## State transitions and invariants

| Entry point / starting state | Input or event | Resulting state and output | Refusal and preserved state |
|------------------------------|----------------|----------------------------|-----------------------------|
| No study | `GET /synthetic/0` | One Part 10 object | Index 12 or a non-number is 404. No bytes. |
| File window 40/400 LINEAR | Preset `soft_tissue` | Same grey as the file window | A bad token is the window error. The body is not echoed. |
| VOI LUT Sequence present | Preset `lung` | Window error | Grey from a previous response is untouched because this response fails. |

The listener stays on `127.0.0.1`. Bodies and paths are not logged.

## Ownership, commit and recovery

Not applicable to a GPU resource. Each request is stateless. A repeated `GET /synthetic/0` returns the same bytes. A refused preset does not change the function that serves the next request.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | Slice bytes contain no (0010,0010) | `synthetic_study_has_no_patient_name` | A hard-coded name | implementation |
| AC2 | Every slice passes `preview_frame` | `every_synthetic_slice_previews` | A builder with no Pixel Data | implementation |
| AC3 | Soft tissue matches the file window on a named pixel | `soft_tissue_matches_the_file_window` | A second window formula | implementation |
| AC4 | Air, body, lung, and bone have hand-computed grey under each preset | `preset_grey_matches_ps3_3_c_11` | Reading the implementation back as the oracle | implementation |
| AC5 | Mutating the intercept expectation fails the test | The cited formula in the test | A tolerance edit | implementation |
| AC6 | Spacing `0.7` then `0.5` is row then column | `spacing_keeps_row_then_column` | A swapped pair | implementation |
| AC7 | Axial `1\0\0\0\1\0` letters are left R, right L, top A, bottom P | `axial_edge_letters` | Letters from the column cosine on the left edge | implementation |
| AC8 | A preset on a VOI LUT Sequence is the window error | `a_preset_refuses_a_voi_lut_sequence` | Drawing the preset anyway | implementation |

## Approach

Add `synthetic.rs` with local element helpers. Do not depend on `ocelli-example-server`. Publish `WindowPreset::center` and `width`. Thread the preset through `lut_chain` before the numeric override. Share one geometry read between inspect and preview.

## Dependencies, write ownership and resources

Depends on F-201, which is done. Writes `examples/folder-viewer` and `crates/ocelli-viewport/src/presentation.rs`. F-207 also writes the folder viewer and waits for this story. No GPU.

## Verification and evidence reuse

`bin/ocelli.sh test ocelli-folder-viewer` and `bin/ocelli.sh test ocelli-viewport` while iterating. The sprint gate is later, once, for the whole sprint. Evidence is invalidated by any later edit to these files.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no wasm crossing. Grey bytes stay an HTTP body of this local process, as they are today.
- Render-loop allocation: n/a. This crate does not render on a device.
- unsafe: none
- Tier A (WebGPU): n/a. This story does not open a device.
- Tier B (WebGL2): n/a
- Tier C (CPU): full. `preview_frame` is the CPU `LutChain`.

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| fixture | Hand-computed grey from PS3.3 C.11.2.1.2 | `examples/folder-viewer/tests/preview.rs` |
| fixture | Row spacing then column spacing, PS3.3 C.7.6.2.1.1 | same |
| fixture | Edge letters, PS3.3 C.7.6.1.1.1 | same |
| refusal | Preset plus a VOI LUT Sequence | same |

## Parity surface covered

None. Appendix B is the cornerstone parity of the library renderer. This story does not change that renderer.

## Deviations

none

## LLD impact

none. The local example is not the published API. `docs/lld/public-api.md` already names the preset tokens.

## Open questions

None. The phantom, the preset refusal, and the spacing order were fixed before implementation.
