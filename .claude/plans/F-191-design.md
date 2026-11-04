# F-191, frame open from a Part 10 object

**Status**: approved
**Epic ref**: E38.1
**Sprint**: S42
**Estimate**: 5w

## Normative source, transcribed

HLD `docs/hld/15-lut-chain.md` section 18:

```text
This is the highest-risk arithmetic in the project. It is specified in DICOM PS3.3 C.11 and the stages apply strictly in order. Implement it once, in ocelli-pixel, and let the shader read the parameters. Do not let a second copy of this logic appear anywhere.
```

Section 18 stage table, quoted as data:

| Stage | From to | Source |
|----|----|----|
| 1. Modality LUT | Stored to Modality | Rescale slope and intercept, or a Modality LUT Sequence which takes precedence |
| 2. VOI LUT | Modality to Display | Window centre and width with a function, or a VOI LUT Sequence |
| 3. Presentation LUT | Display to Display | Identity or INVERSE. Presentation state may override |
| 4. Palette or ICC | Display to RGB | Palette colour LUT, or the display colour pipeline |

Section 18.1:

```rust
pub fn modality(sv: Stored, slope: f32, intercept: f32) -> Modality {
    Modality(sv.0 * slope + intercept)
}
```

A Modality LUT Sequence wins over slope and intercept. PET SUV is a separate path.

Section 18.2 formulas, copied exactly:

```text
// PS3.3 C.11.2.1.2 -- LINEAR. Requires w >= 1.
// c' = c - 0.5 ; w' = w - 1
// x <= c' - w'/2 -> ymin
// x > c' + w'/2 -> ymax
// else y = ((x - c') / w' + 0.5) * (ymax - ymin) + ymin
// PS3.3 C.11.2.1.3.2 -- LINEAR_EXACT. Requires w > 0.
// x <= c - w/2 -> ymin
// x > c + w/2 -> ymax
// else y = ((x - c) / w + 0.5) * (ymax - ymin) + ymin
// PS3.3 C.11.2.1.3.1 -- SIGMOID. Requires w > 0.
// y = (ymax - ymin) / (1 + exp(-4 * (x - c) / w)) + ymin
```

Section 18.3 rows. Deviation D-13 corrects the first LINEAR_EXACT cell from 1.594 to 0. The formula in 18.2 wins. Centre 40, width 400, output 0 to 255:

| Input | LINEAR | LINEAR_EXACT |
|----|----|----|
| -160 | 0.000 | 0 |
| 40 | 127.819 | 127.500 |
| 240 | 255.000 | 255.000 |
| -60 | 63.910 | 63.750 |

HLD section 19 does not describe frame open. HLD section 21 owns the codec registry. This story does not add a decoder.

`docs/hld/B-parity-surface.md` has no row named for padding, lossy compression, burned-in annotation, or private creators. This story establishes none of those rows.

## What the specification does not cover

The HLD does not specify pixel padding, lossy flags, burned-in annotation, or private creators. Those behaviours come from the story text and the DICOM sections it names. They are not presented here as HLD quotations.

DICOM PS3.3 C.7.5.1.1.2, from the current standard text:

- Pixel Padding Value `(0028,0120)` is a stored pixel value, before the Modality LUT or any other transformation.
- With Pixel Padding Range Limit `(0028,0121)`, the two values are inclusive bounds. Every stored value between them is padding.
- For MONOCHROME2 and PALETTE COLOR, Pixel Padding Value shall be less than or equal to Pixel Padding Range Limit.
- The range limit shall not be present unless Pixel Padding Value is present.
- Padding values are suppressed. The standard does not give a second window formula for them.

DICOM PS3.3 C.7.6.1.1.1 and the image pixel module name Lossy Image Compression `(0028,2110)` as `00` or `01`. Burned In Annotation `(0028,0301)` is `YES` or `NO`.

DICOM PS3.5 7.8.1. An odd group is private. `(gggg,00xx)` holds the creator for elements `(gggg,xxee)`.

Colour selection uses the existing `ColorTransform` and the decoder's reported output space. This story does not convert samples.

Showcase decision, already made by the operator for this programme. The example viewer at `examples/viewer-react` is the page that must show the behaviour. `examples/folder-viewer` is not the showcase and is not changed. The measured `ocelli-wasm` render module does not gain `ocelli-dicom` or `ocelli-codec`. `wasm-bindgen` stays in `ocelli-wasm` only. F-198 still owns the wasm session, cache, tier resolution and DICOMweb source.

This story therefore adds `examples/example-server`, package `ocelli-example-server`, `publish = false`. `npm run dev` starts Vite and this helper. The helper binds `127.0.0.1`, prints only its base URL, and does not log names, paths, header values that carry names, or pixels. The page has a synthetic fixture button and a file control. A chosen file is posted and discarded. It is never written into the repository.

## Problem and reproducer

`parse_part10` returns a data set. `LutChain::new` already accepts a modality transform and a VOI transform. Nothing in `ocelli-dicom` turns one Part 10 object into those inputs, a padding range, the lossy and burned-in flags, a colour selection, and the private creators. The folder page still refuses a Modality LUT Sequence and refuses a VOI LUT Sequence unless the request replaces both window values. That refusal stays. This story does not weaken it.

Trigger: one synthetic Part 10 object with stored sample 50, slope 1, intercept 0, LINEAR_EXACT centre 100 and width 200, output 0 to 255, plus one padding sample whose stored value is the declared padding value. Expected: the existing chain maps 50 to display 63.75, the padding sample is reported in the mask, and the page shows both facts. Current behaviour: no such function and no such panel.

## Scope, API and compatibility

In scope:

- New module `crates/ocelli-dicom/src/frame_open.rs`, re-exported from the crate root.
- Read chain inputs through `MultiframeMetadata::resolve` with `TopLevelFallback::Allowed`. Do not copy pixel bytes into the metadata projection.
- Construct `ModalityTransform` and `VoiTransform` with the existing constructors. Do not call `map_into` inside the open type. The helper that serves the page calls `LutChain::map_into` for the synthetic grey value.
- Return presentation evidence for the caller that builds `LutChain`. Do not invent a window when centre, width and a VOI LUT Sequence are all absent. That case is `WindowAbsent`.
- Padding range and `padding_mask(stored_samples) -> Vec<bool>` of equal length. After a caller has mapped non-padding samples, padding positions are filled with the black display end. MONOCHROME2 black is `ymin`. MONOCHROME1 black is `ymax`. That fill is a constant. It is not a window function.
- Colour selection enum: monochrome, palette, or colour, plus whether the codec reports that it already converted to RGB. A second conversion is a refusal, not a second matrix in this crate.
- Lossy flag and burned-in flag.
- Private creators for odd groups. Unknown private elements are listed with their creator. Their values are not interpreted.
- Example server route `POST /frame-open` and a panel on `examples/viewer-react/src/App.tsx`.
- `npm run dev` starts the helper. The README start steps name that helper once it exists.

Out of scope:

- Decoding a new transfer syntax.
- SUV, measurements, KOS, Secondary Capture.
- Changing `examples/folder-viewer`.
- Linking this crate into the measured wasm render module.
- GSPS, which remains F-139.
- Applying padding by editing the stored samples before the chain.

Signatures, owned by this story:

```rust
pub struct OpenedFrame { /* fields below, private, with accessors */ }

pub enum WindowInput {
    Transform(VoiTransform),
    Absent,
}

pub enum ColourSelection {
    Monochrome,
    Palette,
    Colour { codec_already_rgb: bool },
}

pub enum LossyFlag {
    Absent,
    Lossless,
    Lossy,
}

pub enum BurnedInFlag {
    Absent,
    No,
    Yes,
}

pub struct PaddingRange {
    // stored integers after the existing signed interpretation, before the modality stage
}

pub struct PrivateCreator {
    pub group: u16,
    pub slot: u8,
    pub creator: String,
}

pub fn open_frame(parsed: &ParsedDicom, frame: u32) -> Result<OpenedFrame, FrameOpenError>;
```

`OpenedFrame` exposes `modality(&self) -> &ModalityTransform`, `window(&self) -> &WindowInput`, `photometric`, `presentation`, `colour`, `padding`, `lossy`, `burned_in`, and `private_creators`. `padding_mask` takes `&[i32]` and returns a `Vec<bool>`.

Refusal policy. A bad lossy code, a bad burned-in code, a padding range that violates the photometric order, a range limit without a padding value, or a colour photometric whose codec also reports a second conversion is `FrameOpenError`. The error carries a tag and a closed reason. It does not carry bytes, names, or pixel samples. An absent optional attribute stays absent. A zero-length value is absent, matching the folder page rule for window and rescale, and it is not an attribute error.

Compatibility. `parse_part10`, `LutChain`, and `StackGeometry::derive` keep their current results. Existing tests stay green without a tolerance edit.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| `open_frame` | `parse_part10`, `MultiframeMetadata`, existing modality and VOI constructors | One frame index inside one object | Opened frame or a tag-only error |
| Synthetic page button | `ocelli-example-server` `POST /frame-open` | Built-in bytes, no file from disk | Display 63.75 for stored 50, padding count 1 |
| Chosen file on the page | Same route | Browser posts bytes. Server discards them | Same JSON shape, or the tag-only error |
| Later F-198 wasm session | `OpenedFrame` accessors | Does not exist in this story | Unchanged absence of a wasm parse export |

Units. Stored padding values are integers in stored space. Display values stay `ocelli_core::Display`. The page prints the display number it received. It does not recompute it.

## State transitions and invariants

Stateless. `open_frame` does not cache and does not mutate the parsed object. A second call with the same bytes returns an equal value. A refusal leaves no partial server state. The page keeps the previous successful panel until the next successful response, and shows the error beside it.

Invariant. Non-padding samples are mapped only by `LutChain`. Padding samples are then filled with one constant display end. A test that changes one stored padding value to a non-padding neighbour must show the chain output, not the fill.

## Ownership, commit and recovery

| Resource or visible state | Acquired or staged by | Lifetime identity | Observable commit point | Failure and duplicate-operation behavior |
|---------------------------|-----------------------|-------------------|-------------------------|------------------------------------------|
| Posted body | Example server, capped at the existing 1 GiB folder-page cap | One request | JSON response completed | Over-cap request is refused before the body is read. A repeated post runs again. Nothing is stored. |
| Panel text | The React page | The latest response id | DOM text after a 200 | A failed request leaves the last success visible and adds the error sentence. |

No GPU resource. No render-loop allocation.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | Stored 50, slope 1, intercept 0, LINEAR_EXACT 100/200, ymin 0, ymax 255. Display is 63.75. | `frame_open` fixture citing PS3.3 C.11.2.1.3.2 | A second window formula, or LINEAR in place of LINEAR_EXACT | implementation |
| AC2 | Same object with a Modality LUT Sequence that maps 50 to 50. The sequence is selected and rescale 2/0 is not applied. | Fixture citing PS3.3 C.11.1 | Applying both | implementation |
| AC3 | VOI LUT Sequence present. It is selected even when centre and width are also present. | Fixture citing PS3.3 C.11.2 | Window applied as well as the sequence | implementation |
| AC4 | No window and no VOI LUT. `WindowInput::Absent`. No invented centre. | Unit | Full-range grey invented inside `open_frame` | implementation |
| AC5 | Padding value 0 and limit 0 on MONOCHROME2. Stored `[0, 50]`. Mask `[true, false]`. After map, index 0 is ymin and index 1 is 63.75. | Fixture citing PS3.3 C.7.5.1.1.2 | Padding run through the window and left there | implementation |
| AC6 | MONOCHROME2 with padding value greater than the limit. `FrameOpenError`. | Unit | Silent swap of the two ends | implementation |
| AC7 | Lossy `01` and burned-in `YES` are reported. `00` is lossless. Absent stays absent. `MAYBE` is an error. | Unit citing the CS definitions | Treating empty as YES | implementation |
| AC8 | Private tag `(0009,1001)` resolves to the creator in `(0009,0010)`. The value bytes are not copied into the error or the creator record. | Unit citing PS3.5 7.8.1 | Reading `(0009,0010)` as pixel data | implementation |
| AC9 | Palette photometric selects `ColourSelection::Palette` and does not build a monochrome chain. | Unit | Decoding palette as grey | implementation |
| AC10 | The example page synthetic button shows 63.75 and padding count 1. | `curl` against the helper, then the page source contains the panel | Result only in a crate test | integrated |
| AC11 | Mutating the LINEAR_EXACT centre constant in the fixture expected value makes AC1 red. | Recorded once during implementation | A test that restates the function body | implementation |

## Approach

Add `frame_open.rs` beside `multiframe.rs`. Resolve rescale, window, LUT sequences, photometric interpretation, and presentation evidence with the existing metadata and functional-group readers. Call `ModalityTransform::new` and `VoiTransform::new`. Return the pieces.

Padding compares stored integers inclusively. The mask function does not allocate per sample beyond the one result vector. The constant fill lives next to the mask function and takes the already mapped `&mut [Display]` plus the photometric interpretation. It writes only masked indices.

The example server is a new binary crate. Its first route calls `open_frame` and, for the synthetic fixture only, `LutChain::map_into` then the padding fill. JSON fields are numbers, closed strings, and counts. No sample array is returned for a posted file larger than the single-sample fixture. The posted-file response returns flags, colour, creator slots, and padding count. That keeps pixels off the wire except for the built-in one-sample demonstration, which is not patient data.

The React panel is a section under the existing header. It does not replace `OcelliViewport`.

Rejected. Putting the parser in `ocelli-wasm`. Reusing the folder viewer as the showcase. A second LUT.

## Dependencies, write ownership and resources

Depends on F-030 and F-020, both done. No story in S42 is a dependency. Files this story owns for the serial wave:

- `crates/ocelli-dicom/src/frame_open.rs` and `lib.rs`
- `crates/ocelli-dicom/tests/frame_open.rs`
- `examples/example-server/**`
- `examples/viewer-react/src/App.tsx`
- root `package.json` dev script
- `README.md` start paragraph for the helper
- `docs/lld/frame-open.md` at completion

F-192 and F-193 also write `ocelli-dicom` and `App.tsx`. F-194 writes `App.tsx`. The wave is serial, in the canonical worktree, in story order. No claim, no worker branch.

No GPU. No shader. No corpus row. No tolerance edit.

## Verification and evidence reuse

Focused:

```bash
bin/ocelli.sh test ocelli-dicom frame_open
bin/ocelli.sh clippy ocelli-dicom
bin/ocelli.sh check ocelli-example-server
```

The page check is `curl` of `POST /frame-open` with the synthetic body. Final evidence is the sprint gate, not this story's check. A later edit to `App.tsx` invalidates only the page observation, not the crate fixture, until the sprint gate reruns.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none
- unsafe: none
- Tier A (WebGPU): n/a. This story does not draw.
- Tier B (WebGL2): n/a
- Tier C (CPU): n/a

The helper is native. The page shows its JSON. Drawing remains the existing viewport.

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| fixture | AC1 to AC5, hand values, DICOM section in the test name comment | `crates/ocelli-dicom/tests/frame_open.rs` |
| unit | AC6 to AC9 refusals and flags | same file |
| integrated | AC10 helper JSON | `examples/example-server` test plus curl |

No oracle image. The chain fixture is the independent arithmetic check. AC1 expected 63.75 is the known value `((50 - 100) / 200 + 0.5) * 255`.

## Parity surface covered

None.

## Deviations

None.

## LLD impact

New `docs/lld/frame-open.md`. No rewrite of the pixel-pipeline LLD. That file stays the chain.

## Open questions

None. The showcase helper decision is recorded above from the operator instruction that the example viewer displays the functionality, together with the existing wasm-bindgen and measured-module constraints.
