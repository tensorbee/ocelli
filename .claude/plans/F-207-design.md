# F-207, Folder viewer layout, gestures, marks and image controls

**Status**: approved
**Epic ref**: E38.17
**Sprint**: S45
**Estimate**: 8w

## Normative source, transcribed

The HLD does not specify this page. The binding rules it does specify, and this plan keeps, are these.

From `docs/hld/15-lut-chain.md` section 18, the LUT chain exists once. This page sends a preset name to F-206. It does not store 40, 400, -600, 1500, or 300.

From `docs/hld/11-decision-log.md`, pixels do not cross the wasm boundary, and events are not a callback per pointer event into the core. This page is not that path. Its grey bytes already come from the local process. F-207 does not move the page into `ocelli-wasm`.

DICOM SR as the annotation type is HLD decision D13. These marks are not that type. The page says they are drawings in the browser and that they are gone on reload.

## What the specification does not cover

Claude reviewed the current `page.html` handlers and the accepted three-route design. The choices below are that review. The HLD does not name a gray ramp or a trackpad mapping.

Font, one stack, for every control except the four corners, the header values, and the length number:

```text
system-ui, -apple-system, "Segoe UI", Roboto, "Noto Sans", "Liberation Sans", Arial, sans-serif
```

UI size is 13px on a 1.4 line height. No downloaded font.

Chrome colours have equal channels.

| Role | Hex | Use |
|---|---|---|
| Canvas surround | `#000000` | Behind the image |
| Page | `#141414` | Body, `/more`, `/use` |
| Panel | `#1c1c1c` | Bars, list, inspector |
| Raised | `#262626` | Hover and the selected row |
| Line | `#2e2e2e` | Dividers |
| Control border | `#3d3d3d` | Resting controls |
| Faint | `#5c5c5c` | Disabled |
| Muted | `#999999` | Status and sentences |
| Text | `#e6e6e6` | Labels |
| Active tool | `#dedede` fill, `#141414` label | The one active chip |
| Focus ring | `#ffffff` | Keyboard focus |
| Fault | `#f2786d` | Fault text only |

Ink on the image is a separate palette. Default is cyan `#00d1ff`. The others are green `#3ee06a`, lemon `#fff04d`, magenta `#ff4fd8`, and white `#ffffff`. Each stroke is a black underlay then the colour. Shapes use `fill: none`.

Layout is a grid. Columns are `248px minmax(0, 1fr) 272px`. The top bar is the name, the status, `Synthetic study | Folder`, and the links Images, More, and Use. The middle toolbar is Scroll, Pan, Zoom, W/L, Freeform, Rectangle, Ellipse, Text, Length, then Fit, Reset, and Full screen. The inspector holds presets, invert, rotate, flips, the display toggles, 1 by 1 or 2 by 2, key image, Headers, the colour swatches, text size, the marks list, and Remove. A 28px line under the scrubber shows the sentence for the active or hovered control.

Gestures, from the events this browser actually delivers:

| Input | Result |
|---|---|
| Wheel without Ctrl | Change slice. Normalise `deltaMode`. Step when the accumulator reaches 40px. At most one step per event. Reset on a direction change or after 150ms. |
| Wheel with Ctrl, including pinch | Zoom at the pointer. `preventDefault` even when no image is loaded. |
| Horizontal wheel | `preventDefault` and do nothing else |
| Left drag | The active tool |
| Middle drag | Zoom, anchored at the press |
| Right drag, or Ctrl plus left drag | Pan |
| Wheel during a drawing press or a move | Ctrl may zoom. A slice change is ignored. |
| `=` `-` `0` | Zoom in, zoom out, and fit |

Anchored zoom is `pan' = pan + (q - C) * (1 - s' / s)` with `C` the wrap centre plus the current pan.

Marks store image pixels. Freeform keeps a point list of at least three points. Rectangle and ellipse store two corners. Text stores one point, a string, and a size of 12, 15, 20, or 28 CSS pixels. Rendered text size is `note.size / scale`. Length keeps the accepted spacing formula and gains a colour. A mark can be selected from any tool and from the marks list. Remove, Delete, and Backspace delete the selection. Empty text is dropped. Escape cancels a draft. Text glyphs stay upright under flip and rotate.

## Problem and reproducer

`page.html` is Georgia on `#16130f` with a gold accent. `onWheel` steps one slice per event and ignores `ctrlKey`, so a trackpad swipe skips many slices and a pinch does nothing. Selection works only while the contour or text tool is active. Text size is rewritten to `15 / scale` on every layout. Rectangle and ellipse do not exist. There is no visible Remove control.

## Scope, API and compatibility

In scope is `page.html`, `more.html`, `use.html`, the GET routes for those pages, and the README steps that name Contour and the wheel.

The image controls and the sentence each one shows:

| Control | Sentence |
|---|---|
| Scroll, Pan, Zoom, Fit, Reset, Invert, W/L, Play, Headers | The sentences already implied by the control names, in plain words |
| Freeform | A drawing that follows the pointer. It is kept when it has at least three points. It is not saved. |
| Rectangle, Ellipse | A drawing around a region. It is not an area and it is not saved. |
| Text | A note on this image. Size and colour can be changed. It is not saved. |
| Length | The distance between the two points. It is in millimetres when the file gives Pixel Spacing, and in pixels when it gives no spacing. It is not saved. |
| Presets | The same LUT chain is run again with this preset's centre and width, in modality units. On a CT that is Hounsfield units. |
| Rotate, Flip | The displayed picture is turned or mirrored. The pixels are not changed and the marks turn with them. |
| Pointer cross | Two lines meet at the pointer. The numbers are the column and row under it, counted from 1. |
| Magnifier | This circle enlarges the grey pixels already drawn. It copies them and does not blend them. |
| Orientation letters | The patient direction at each edge, read from Image Orientation (Patient). The letters are hidden while the picture is turned or flipped. |
| Key image | Marks this image in the series list for this session. It is not saved and it does not write a Key Object Selection. |
| Full screen | The viewport fills the screen. Escape leaves full screen. |
| 2 by 2 | Four neighbouring images from the open series, each one decoded by the same preview. |

2 by 2 is disabled when the file is larger than 32 MiB or when a drawing tool is active. Drawing in the grid says `Drawing works in the single view.`

`/more` names, without a picture and without a result number, the library behaviours this image refuses: overlay, shutter, slice interpolation, reference lines, MPR, volume, fusion, TID 1500, measurement save, and key object. It names the function and that the React harness is where that function runs.

`/use` has three short instructions. JavaScript uses `@ocelli/core`. React uses `OcelliViewport`. Rust uses `parse_part10` and `LutChain` and points at the README example. It says this folder viewer is not the wasm path and that nothing is published.

Out of scope: changing W/L drag axes, overlay drawing, shutter drawing, blending two slices, and any second measurement formula. Imager Pixel Spacing is labelled `mm at the detector`.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| Source control | `GET /synthetic/<n>` or the folder input | Both call `readFolder` | The same series list |
| Preset button | `POST /preview` with `X-Ocelli-Window-Preset` | No numbers in the page | New grey bytes |
| Length | Preview spacing headers | Row then column | A label in mm, detector mm, or pixels |
| Marks | The `notes` map in the page | Never posted | SVG over the image |

## State transitions and invariants

| Entry point / starting state | Input or event | Resulting state and output | Refusal and preserved state |
|------------------------------|----------------|----------------------------|-----------------------------|
| Image shown, tool Scroll | Pinch | Scale changes, pan keeps the pointer still | No image: the page itself does not zoom |
| Drawing press in progress | Wheel without Ctrl | Draft unchanged, slice unchanged | Ctrl zoom rescales the draft with the stage |
| Navigation tool, mark under the pointer | Click with little movement | That mark is selected | A drag past 5 CSS pixels runs the tool and does not move the mark |
| Drawing tool, mark under the pointer | Press | That mark is selected and moves with the pointer | A drag under 5 CSS pixels only selects |
| Text selected | Size or swatch | `note.size` or `note.color` changes | A navigation drag does not recolour |
| Headers open, or the note field open | Wheel | Ignored | The sheet stays |

A slice change clears the selection. The colour row stays visible so the next mark still has a colour.

## Ownership, commit and recovery

Marks live in the page until reload. There is no commit to a file. Removing a mark removes it from the map for that file and frame. A failed preview hides the previous grey image and shows the fault sentence.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | The page names Freeform, Rectangle, Ellipse, and Remove | `the_page_offers_a_folder_picker` extended | A gold Contour-only toolbar | implementation |
| AC2 | The stylesheet uses the system sans stack and `#141414` | Same test reads those strings | Georgia or `#16130f` left in place | implementation |
| AC3 | `/more` and `/use` return HTML and do not contain a canvas viewer | `more_and_use_are_static_pages` | A second viewport | implementation |
| AC4 | Pinch is `ctrlKey` on the wheel handler and calls `preventDefault` | The page source contains that branch | A swipe that still steps once per event | implementation |
| AC5 | README no longer says the wheel always changes the image | README read in review | A stale step 8 | implementation |

Page gestures are not executed in a browser by the Rust tests. The sprint close starts the process and reports the URL. A click-through of the live page is a separate observation and is not claimed by the Rust test.

## Approach

Keep one `page.html` compiled with `include_str!`. Add `more.html` and `use.html` the same way. Restyle in place. Replace `onWheel`. Extend the note model. Map marks through the same stage transform as the image, with an inverse for new points. Length uses the spacing headers and the existing distance in row and column millimetres. It does not add a second formula.

## Dependencies, write ownership and resources

Depends on F-206. Writes `examples/folder-viewer/src/page.html`, `more.html`, `use.html`, `serve.rs`, and `README.md`. No GPU. Serial with F-206 because both edit the folder viewer.

## Verification and evidence reuse

`bin/ocelli.sh test ocelli-folder-viewer` after the page strings change. The sprint gate is the later whole-sprint run. A page edit after that record invalidates it.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no wasm crossing. The page still receives grey bytes from the local process.
- Render-loop allocation: none. The canvas is sized when the frame arrives.
- unsafe: none
- Tier A (WebGPU): n/a. This page does not open a device.
- Tier B (WebGL2): n/a
- Tier C (CPU): full for the grey image, because F-206's preview is the CPU LUT. Marks and gestures are page geometry on top of those bytes.

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| contract | The toolbar, the ramp, and the static routes exist | `examples/folder-viewer/tests/preview.rs` |
| fixture | Length inputs stay the F-206 spacing fields. This story does not recompute them. | F-206 tests |

## Parity surface covered

None. The React harness already carries the S44 controls. This story does not add a cornerstone observation.

## Deviations

none

## LLD impact

none. The local example is not the published API.

## Open questions

None. The ramp, the font, the gesture table, and the mark fields were fixed by the review before implementation.
