# F-208, Folder viewer viewports, layouts from 1 by 1 to 3 by 3, per-viewport series and view state, an active viewport, a series dragged onto a cell, and the four-image view kept as a tile mode

**Status**: approved
**Epic ref**: E38.18
**Sprint**: S46
**Estimate**: 6w

## Normative source, transcribed

The HLD does not specify this page. The binding rules it does specify, and this plan keeps, are these.

`docs/hld/07-concurrency-and-typescript.md` section 10, "What stays TypeScript":

```text
- DOM, pointer, touch and wheel events; canvas lifecycle; ResizeObserver
- The SVG annotation drawing layer — DOM-native, and faster left there
- Tool interaction state machines; the geometry and hit-testing behind them is Rust, in ocelli-geom
```

This story is all of the first bullet and the state-machine half of the third. It adds no DICOM geometry. The page's own display transform (`imageToClient`, `clientToImage`, `fitScale`) is display state, already in the page, and becomes per viewport.

`docs/hld/04-boundary-and-data-path.md` section 5.1:

> Small, infrequent, one call per user intent: set camera, set VOI, activate tool, load series. **Never one call per pointer move.**

The page is not the wasm path. The rule still shapes it: each viewport issues one `POST /preview` per shown image and per committed window, through the existing cache, and a live W/L drag keeps the existing one-in-flight coalescing per viewport.

`docs/hld/15-lut-chain.md` section 18 keeps the LUT chain in one place. Each viewport sends its own window command (a preset name, or centre, width and function, or nothing for the file window) to F-206's `/preview`. The page stores no preset numbers and maps no grey value.

`docs/hld/01-purpose-and-scope.md` section 1 puts "the study library UI" in Phase 3. The series panel stays the list of series in the opened folder. It is not a study library.

Decision D13 makes the annotation type an SR content tree. These marks are not that type. The page keeps the sentence that they are drawings in this browser and gone on reload.

`docs/hld/B-parity-surface.md` is a summary table. It counts 12 viewport types and about 63 tools. It has no per-capability row and no Covered by column.

From the approved `.claude/plans/F-207-design.md`, the accepted rules this story must keep:

| Input | Result |
|---|---|
| Wheel without Ctrl | Change slice. Normalise `deltaMode`. Step when the accumulator reaches 40px. At most one step per event. Reset on a direction change or after 150ms. |
| Wheel with Ctrl, including pinch | Zoom at the pointer. `preventDefault` even when no image is loaded. |
| Horizontal wheel | `preventDefault` and do nothing else |
| Left drag | The active tool |
| Middle drag | Zoom, anchored at the press |
| Right drag, or Ctrl plus left drag | Pan |
| `=` `-` `0` | Zoom in, zoom out, and fit |

> Anchored zoom is `pan' = pan + (q - C) * (1 - s' / s)` with `C` the wrap centre plus the current pan.

In this story `C` becomes the cell centre plus that viewport's pan. Nothing else in the formula changes.

## What the specification does not cover

Claude read the working-tree `page.html` (3134 lines, uncommitted polish on `sprint/s46`) and the professional-viewer research note kept in the session scratchpad, which is not tracked, sections 1, 2, 9 and 10. Every choice below is local to the example.

**State model.** Today the view state is one set of globals: `activeIndex`, `viewIndex`, `fitMode`, `scale`, `panX`, `panY`, `rotation`, `flipH`, `flipV`, `gridMode`, `imageWidth`, `imageHeight`, `aspectX`, `aspectY`, `overrideWindow`, `draftWindow`, `activePreset`, `invert`, `shownWindow`, `shownFacts`, `zoomLabel`, `loadToken`, `tileToken`, `liveBusy`, `liveDirty`, `liveToken`. They all move into one record per viewport.

```js
const MAX_SIDE = 3;               // columns and rows each run 1..3
let layout = { columns: 1, rows: 1 };
const viewports = [];             // cell number 0..8, row-major, created on first use
let activeCell = 0;

function blankViewport(cell) {
  return {
    cell,
    seriesKey: "",                // series.key, stable across a re-sort, "" when empty
    viewIndex: 0,
    fitMode: true, scale: 1, panX: 0, panY: 0,
    rotation: 0, flipH: false, flipV: false,
    overrideWindow: null, draftWindow: null, activePreset: null, invert: false,
    tileMode: false,
    imageWidth: 0, imageHeight: 0, aspectX: 1, aspectY: 1,
    shownWindow: null, shownFacts: null, zoomLabel: "100%",
    loadToken: 0, tileToken: 0, liveToken: 0, liveBusy: false, liveDirty: false,
    dom: null                     // the cell's nodes, from the template
  };
}
```

What stays page-global: `tool`, `showCorners`, `showCross`, `showMag`, `showLetters`, `currentColor`, `currentSize`, `notes`, `keyImages`, `selected`, `draft` (gains `draft.cell`), `move` (gains `move.cell`), `drag` (gains `drag.cell`), `playing`, the wheel accumulators (gain `wheelCell`), `cache`, `inflight`, `thumbs`, and `positions` (the last index per series key, written by whichever viewport scrolled that series).

**Function signatures.** Each display function takes the viewport it acts on. `currentImage(vp)`, `showIndex(vp, next)`, `applyGrey(vp, grey, image)`, `cacheKey(image, vp)`, `requestGrey(image, options)` where `options.view` carries that viewport's `overrideWindow`, `activePreset` and `invert`, `drawGrey(vp, grey)`, `syncStage(vp)`, `paintTiles(vp)`, `fitScale(vp)`, `layoutViewport(vp)` (renamed from `layout` so the name does not collide with the grid), `clientToImage(vp, x, y)`, `imageToClient(vp, point)`, `paintOverlays(vp)`, `paintEdges(vp)`, `renderInk(vp)`, `zoomAt(vp, x, y, factor)`, `fitView(vp)`, `resetView(vp)`, `applyPreset(vp, name)`, `scheduleLive(vp)`, `prefetch(vp, index)`. `fitScale`, `clientToImage` and `imageToClient` read `vp.dom.cell.getBoundingClientRect()` instead of `wrap`. `queueSlice(sign)` keeps its signature and reads `wheelCell`, because the F-207 test pins that call text.

**DOM.** `#wrap` keeps its id and becomes a CSS grid of `layout.columns` by `layout.rows` cells with a 2px `#2e2e2e` gap. Each cell is cloned from `<template id="cell-template">`. The single-view ids `frame`, `stage`, `view`, `ink`, `tiles`, `ov-tl`, `ov-tr`, `ov-bl`, `ov-br`, `edge-*` and `fault` become classes inside the cell. `#empty`, `#note`, `#mag`, `#cross-h`, `#cross-v` and `#cross-read` stay single nodes under `#wrap`, positioned from the cell under the pointer. The ink draft nodes keep the id `draft-under`, because only the active cell ever holds a draft.

**Layout picker.** A toolbar button `id="layout"` with `aria-label="Layout"` replaces `grid-1`. It opens `#layout-picker`, a 3 by 3 grid of buttons with `data-columns` and `data-rows`. Hover highlights the rectangle from the top-left to the hovered button. A click applies it and closes the picker. Escape and an outside click close it unchanged. The sentence is "Choose 1 by 1 up to 3 by 3 viewports. Drag a series onto a viewport, or click a series to open it in the outlined one."

**Tile mode.** The four-image view becomes a mode of one viewport. The button `grid-2` becomes `id="tile-mode"`, `aria-label="Four images"`, acting on the active viewport. Its sentence stays "Four neighbouring images from the open series, each one decoded by the same preview. Double-click one to open it alone." `liveSentence` keeps "Drawing works in the single view." for the tile button while a drawing tool is chosen and for the drawing tools while the active viewport is in tile mode. It keeps "Four images are not shown for a file larger than 32 MiB." under the existing `GRID_CAP`. Tiles keep `sizeTile` and the aspect ratio from `greyAspect`. Tiles are always drawn with `image-rendering: pixelated`, at every fit and every device pixel ratio, because the operator protocol reads "2x2 four neighbouring slices pixelated". The F-207 rule that switched on one image pixel per device pixel no longer applies to a tile.

**Active viewport.** `pointerdown` and `wheel` on a cell make it active before the existing handler runs. The press is not swallowed, so a drag with the active tool works on the first press. With more than one cell, the active cell has a 2px `#dedede` inner outline. That is the F-207 active-tool grey, so the chrome stays equal-channel. With one cell there is no outline. Keys, the scrubber, the counter, Play, presets, Invert, Rotate, Flip, Fit, Reset, Key image, Headers and the marks list act on the active viewport. Changing the active cell stops Play, cancels a draft and a W/L drag, and resyncs every toolbar and inspector control from that viewport's record.

**Series into a cell.** Each thumbnail button gets `draggable = true`. `dragstart` writes the series key under the type `application/x-ocelli-series` with `effectAllowed = "copy"`. A cell accepts `dragover` only when that type is present, adds the class `drop-target`, and on `drop` loads the series into that cell and makes it active. A drag without that type is not accepted, and the browser's own handling is unchanged. A click on a thumbnail loads the series into the active viewport, which is today's behaviour with one cell. A double-click does the same, so the "double-click loads into the active viewport" convention also holds. The series row of the active viewport keeps the `active` class. A row whose series is shown in another visible cell gets the class `shown`.

**Starting state.** Opening a study or a folder resets every viewport to `blankViewport` and keeps the layout. Cell 0 becomes active and receives the series with the most images, which is today's rule. Every other visible cell receives the next series in list order not already shown, and stays empty when there is none. An empty cell shows "Drag a series here." in the muted grey. Growing the layout fills the new cells by the same rule. Shrinking it keeps the hidden viewports' records, increments their `loadToken` and `tileToken` so no late frame paints into a hidden cell, and moves the active cell to 0 when it is hidden. Growing again shows those records as they were.

**Overlays.** Every cell shows its own four corners and edge letters from its own image and record. The corner, cross, magnifier and letters toggles stay global. Zoom in the bottom-right corner is that viewport's scale over its own `fitScale`.

**Marks.** Marks stay keyed by `noteKey(image)`, the file identity and frame. A mark therefore shows in every cell that shows that image. Selection, the draft, the note field and a mark move belong to the active cell. This is the default in Open question 2.

**Full screen** still calls `wrap.requestFullscreen()`, so the whole grid fills the screen.

**Out of scope.** Hanging protocols, a saved layout, maximising one cell by double-click, dropping files from the desktop, linked scroll, synced window or zoom, and reference lines. The last three are F-209.

## Problem and reproducer

Open `cargo run -p ocelli-folder-viewer`, choose Synthetic study, then 2 by 2. Today the four cells are four neighbouring images of the one open series, drawing is disabled, and each cell has no zoom, pan, window or series of its own. There is no way to show two series side by side, no active viewport, and a thumbnail cannot be dragged. Research section 10 rows 10 and 11 tag "Thumbnail/series panel with drag or double-click into a viewport" and "Multi-viewport grid layouts (1x1 up to at least 2x2/3x3)" as MUST.

## Scope, API and compatibility

In scope: `page.html` and its Rust page test. No route changes. No new header. `/preview`, `/inspect`, `/headers` and `/synthetic` are unchanged.

Compatibility with the shipped page, kept on purpose:

- With layout 1 by 1 and tile mode off, every F-207 behaviour reads exactly as today, including the status line, the counter, the four corners, edge letters, the wheel rule, Ctrl-wheel zoom at the pointer, and the marks sentences.
- "Synthetic study, not a patient. 1 series." stays the status after opening the synthetic study. The twelve CT slices, `0.7\0.5` spacing and the 40 / 400 file window are F-206 and are untouched.
- `id="wrap"`, `id="note"`, `id="inspector"` and their order stay, because `the_page_offers_a_folder_picker` checks it.
- The two pinned wheel strings stay byte for byte.

Changed on purpose: the `grid-1` and `grid-2` buttons become `layout` and `tile-mode`. The keys follow the operator's answer to Open question 1. `G` opens the layout picker, `Shift G` toggles tile mode on the active viewport, and bare `1` and `2` no longer choose a layout. This story frees `1` to `9` and binds nothing to them. F-213 binds them to the window rows. `0` stays Fit.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| A cell showing an image | `POST /preview` in F-216's id form, `X-Ocelli-File` with `Content-Length: 0`, and that viewport's window headers | Cache key includes that viewport's window and invert. A 410 re-posts once through F-216's shared `ensureFile` | That cell's grey image, drawn into that cell's one reused `ImageData` |
| Tile mode in one cell | Same `/preview` through `fetchGrey` | `GRID_CAP` per file, `tileToken` per viewport | Four tiles in that cell only |
| Layout picker | `setLayout(columns, rows)` in the page | 1 to 3 on each side | The grid and the filled cells |
| Thumbnail drag | HTML drag and drop, type `application/x-ocelli-series` | Only that type is accepted | The series in the drop cell |
| Thumbnail click | `loadSeries(activeViewport, key)` | Active cell | The series in the outlined cell |
| Wheel, pointer, keys | The F-207 handlers with a viewport argument | Cell under the pointer for wheel and press, active cell for keys | That cell's slice, zoom, pan or window |
| Marks list, Remove, colours | `notes` keyed by image | Active cell's image | The list and the selection |
| Corners and letters | `paintOverlays(vp)`, `paintEdges(vp)` from preview headers | Per cell | Four corners per cell |

Units, rounding and selection stay where F-206 and F-207 put them. The window numbers a W/L drag produces are rounded to 0.01 in the page, as today, and become that viewport's command.

## State transitions and invariants

| Entry point / starting state | Input or event | Resulting state and output | Refusal and preserved state |
|------------------------------|----------------|----------------------------|-----------------------------|
| 1 by 1, synthetic open | Layout 2 by 2 | Cells 1 to 3 show "Drag a series here." Cell 0 keeps its index, zoom and window | No series is duplicated by the fill rule |
| 2 by 2, cell 0 active | Drag the series onto cell 2 | Cell 2 shows the series at its remembered index and becomes active | Cell 0 record unchanged |
| Cell 2 active | Wheel over cell 0, one 40px step | Cell 0 becomes active and steps one image | Cell 2 index unchanged |
| Cell 0 at Zoom 200% | Ctrl-wheel over cell 2 | Cell 2 zooms at the pointer | Cell 0 still reads Zoom 200% |
| Cell 2 active | Lung preset | Cell 2 reads `WW 1500  WL -600` | Cell 0 still reads `WW 400  WL 40` |
| Cell 0 active | Four images | Cell 0 shows four tiles, drawing tools dim while cell 0 is active | Other cells stay single view |
| Cell 0 in tile mode | Double-click tile 3 | Cell 0 single view at that image | Tile mode of other cells unchanged |
| 3 by 3 | Layout 1 by 1 | Only cell 0 shows. Hidden records kept, their tokens incremented | A late frame for cell 5 does not paint |
| 1 by 1 after 3 by 3 | Layout 3 by 3 | Cells 1 to 8 show their kept records | No refetch beyond the cache |
| Slow frame requested in cell 1 | Drop a different series on cell 1 | The old response is ignored by `loadToken` | Cell 1 shows only the new series |
| Drawing press in cell 0 | Wheel over cell 1 | Ignored, as F-207 ignores a slice change during a press | Draft unchanged |
| Playing in cell 0 | Press in cell 1 | Play stops, cell 1 active | Cell 0 index stays where Play left it |
| New folder opened | Read completes | Every record blank, layout kept, cell 0 the largest series | Marks and key images cleared, as today |

Invariant across consumers: a viewport's grey image, corners, edge letters, ink and zoom label always derive from that viewport's record and image. No function reads another cell's state, except `positions`, which is per series by design.

## Ownership, commit and recovery

| Resource or visible state | Acquired or staged by | Lifetime identity | Observable commit point | Failure and duplicate-operation behavior |
|---------------------------|-----------------------|-------------------|-------------------------|------------------------------------------|
| A cell's grey image | `showIndex(vp, n)` | `vp.loadToken` | `applyGrey` paints the canvas | A failed preview shows that cell's fault sentence and leaves other cells alone. A repeat of the same index hits the cache. |
| A cell's tiles | `paintTiles(vp)` | `vp.tileToken` | Each tile's `putImageData` | Leaving tile mode or hiding the cell increments the token |
| A live W/L draft | `scheduleLive(vp)` | `vp.liveToken` | Release commits `overrideWindow` | One request in flight per viewport, the last draft wins |
| A dropped series | `drop` handler | series key | `loadSeries` sets `vp.seriesKey` | Dropping the series a cell already shows keeps its index and view |
| A mark | `addNote` | `note.id` and `noteKey(image)` | Added to `notes` | Shown wherever that image is shown |

The shared grey cache keeps F-216's 64 MiB page budget across all cells, and each viewport canvas keeps one reused `ImageData`, as F-216 does for the single view. With nine cells in tile mode that is up to 36 frames wanted at once. The cache evicts oldest first, as today.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | The page offers a layout picker from 1 by 1 to 3 by 3 | `the_page_has_viewport_cells` finds `id="layout"`, `id="layout-picker"`, `data-columns="3"` and `data-rows="3"` | A two-state 1 or 2 by 2 toggle | implementation |
| AC2 | Cells come from one template | Same test finds `id="cell-template"` and `class="cell"` | Nine hand-copied cell blocks that drift | implementation |
| AC3 | View state is per viewport, not global | Same test asserts the page has none of `let scale =`, `let panX =`, `let rotation =`, `let gridMode =`, `let overrideWindow =`, `let activePreset =`, `let activeIndex =`, `let viewIndex =` at top level, and has `function blankViewport(` | A second set of globals beside the records, so one cell's zoom leaks into another | implementation |
| AC4 | A thumbnail drags onto a cell | Same test finds `application/x-ocelli-series`, `draggable = true`, `"dragover"` and `"drop"` | A drop that accepts any drag, including desktop files | implementation |
| AC5 | Tile mode is kept with its sentences | Same test finds `id="tile-mode"`, the four-image sentence, "Drawing works in the single view." and "Four images are not shown for a file larger than 32 MiB." | Tile mode deleted, or drawing allowed on tiles | implementation |
| AC6 | F-207 page rules hold | `the_page_offers_a_folder_picker` unchanged and green, including both wheel strings and the wrap, note, inspector order | A wheel handler rewritten without the 40px accumulator | implementation |
| AC7 | Empty cells say what to do | `the_page_has_viewport_cells` finds "Drag a series here." | A blank black cell | implementation |
| AC8 | Two cells keep independent zoom, window and slice | Browser observation B3 to B5 below | A shared `scale` or window | integrated / sprint close |
| AC9 | A drag of a thumbnail onto cell 3 loads it there and outlines cell 3 | Browser observation B2 | Drop ignored, or loaded into the active cell instead | integrated / sprint close |
| AC10 | Tile mode in one cell leaves the others single view, double-click opens one tile alone | Browser observation B6 | A global tile mode | integrated / sprint close |
| AC11 | Shrinking and growing the layout keeps each record and paints no stale frame | Browser observation B7 | Records rebuilt on every layout change | integrated / sprint close |
| AC12 | 1 by 1 is the F-207 page | Browser observation B1 | A visible outline or changed corners in 1 by 1 | integrated / sprint close |
| AC13 | `G` opens the layout picker, `Shift G` toggles tile mode, and bare `1` and `2` no longer appear in the `clicks` key map | `the_page_has_viewport_cells` finds the `G` and `Shift G` branches and no `"1": "grid-1"` or `"2": "grid-2"` | Digits left bound to layouts, which F-213 then cannot use | implementation |
| AC14 | On the synthetic study (128 columns, 96 rows, Pixel Spacing `0.7\0.5`) the displayed width over height is `128 * 0.5 : 96 * 0.7` = `64 : 67.2` = 0.952381 within 1e-3 for the single view canvas at 1 by 1, for each of the four tile canvases in tile mode, and for each of the four cell canvases at 2 by 2. Carried from the F-207 polish review, smell S1, which survived three passes. | Headless Chromium through `tools/bench`'s pinned Playwright, reading `getBoundingClientRect()` of each canvas, in a committed script `examples/folder-viewer/tests/browser/aspect.mjs` that starts the viewer with `OCELLI_PORT=0`, reads the printed URL and stops it by PID. A new `bin/ocelli.sh gate viewer-browser` runs the script, sits outside the floor beside `bench` because it needs Chromium, is selected by the `--sprint` and `--all` profiles, and is registered with `scripts/guard_census.py` with a refuse probe. The measured ratios go in the review evidence table. Mutation control: swap `aspectX` and `aspectY` at their assignment and confirm the single-view ratio reads 1.866667 and the check goes red | Swapping `aspectX` and `aspectY` at assignment, which string pins in `tests/preview.rs` cannot catch | integrated, implementation and review |

Browser procedure for AC8 to AC12 and AC14. Host: the operator's Mac, Chrome driven by the Playwright MCP tools, or headless Chromium for AC14. Start `OCELLI_PORT=0 cargo run --release -p ocelli-folder-viewer`, read the one URL the process prints, and open that URL. Port 8765 is never used, because an operator file server holds it.

- B1. Synthetic study. Status reads "Synthetic study, not a patient. 1 series." The counter reads `1 / 12` at the remembered index, no cell outline.
- B2. Layout 2 by 2. Drag the series thumbnail onto cell 3 (`browser_drag`). Cell 3 shows the image and has the outline.
- B3. One wheel event of `deltaY = 40` over cell 3. Cell 3 counter moves by one. Cell 1 does not.
- B4. Ctrl-wheel over cell 1. Cell 1 bottom-right zoom changes. Cell 3 still reads `Zoom 100%`.
- B5. With cell 3 active, Lung. Cell 3 reads `WW 1500  WL -600`. Cell 1 reads `WW 400  WL 40`.
- B6. Cell 1 active, Four images. Cell 1 shows four tiles, Length is dim, hovering it shows "Drawing works in the single view." Double-click tile 3 returns cell 1 to one image, two after the first tile.
- B7. Layout 3 by 3, then 1 by 1, then 2 by 2. Cell 3 shows the same image, zoom and window as at B5.

The Rust tests read the page text and do not execute it. B1 to B7 are a separate observation and are not claimed by a Rust test.

## Approach

1. Add the template, the picker, `blankViewport`, `viewports`, `activeCell` and `setLayout`. Build cells on first use and keep them.
2. Convert the globals in one pass, function by function, with the signatures above. Keep `onWheel` text intact apart from resolving `wheelCell` before the existing branches.
3. Replace `selectSeries(index)` with `loadSeries(vp, key)`. It keeps the reset of fit, window and preset, and reads `positions` for the start index.
4. Rename `gridMode === 2` checks to `vp.tileMode`. `gridAllowed(vp)` keeps the `GRID_CAP` rule.
5. Add drag and drop on thumbnails and cells.
6. Add the outline CSS and the empty-cell sentence.

Rejected: one `<canvas>` for the whole grid. It would move every mark and hit test into a second coordinate system for no gain. Rejected: an iframe per cell. Five shared caches and no shared marks.

## Dependencies, write ownership and resources

Depends on F-207 and F-199, both done. Writes `examples/folder-viewer/src/page.html` and `examples/folder-viewer/tests/preview.rs`. No GPU, no wasm.

**Sprint build order, decided in the S46 design round on 2026-11-05.** F-217 first, then F-216, then F-210, F-211, F-208, F-209, F-212, F-213, F-214 and F-215. F-119 is independent and may run in parallel. F-208 therefore builds on F-216's request layer (`ensureFile`, the id map, the coalesced main view and the reused buffers) and on the pointer readout and `/measure` page code of F-210 and F-211, and converts all of it to per-viewport records in the same pass. F-209, F-212, F-213, F-214 and F-215 come after F-208 and extend the viewport records, so a worker on any of them before F-208 merges would write against globals that no longer exist.

## Verification and evidence reuse

Focused: `bin/ocelli.sh check ocelli-folder-viewer` and `cargo test -p ocelli-folder-viewer`. Final: the sprint profile over the staged union. A page edit after a recorded verification invalidates it. Browser observations B1 to B7 are recorded once against the final tree, with the URL, the Chrome version and a screenshot per step.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no wasm crossing. The page still receives grey bytes from the local process.
- Render-loop allocation: none. Each cell's canvas is sized when its frame arrives, as today.
- unsafe: none
- Tier A (WebGPU): n/a. This page does not open a device.
- Tier B (WebGL2): n/a
- Tier C (CPU): full for each cell's grey image, because F-206's preview is the CPU LUT. Layout, cells and drag are page state on top of those bytes.

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| contract | Picker, template, drag type, tile mode sentences, empty-cell sentence, no global view state | `examples/folder-viewer/tests/preview.rs`, new `the_page_has_viewport_cells` |
| contract | F-207 rules unchanged | existing `the_page_offers_a_folder_picker` |
| observation | Independent cells, drag, tile mode, layout round trip | B1 to B7 above |

No pixel or geometry arithmetic is added. The preview fixtures of F-206 stay the evidence for grey values and spacing.

The AC3 negative check needs a mutation control: add `let scale = 1;` back at top level and confirm `the_page_has_viewport_cells` goes red.

## Parity surface covered

None. Appendix B has no viewport-grid row, and the oracle does not render this page.

## Deviations

none

## LLD impact

none. The local example is not the published API.

## User-facing documentation impact

- `README.md`, "Start the local series viewer". The opening sentence "shows one image at a time" becomes up to nine viewports. Step 3 says the arrow keys act on the outlined viewport and the wheel on the viewport under the pointer. Step 5's layout group "(1 by 1, 2 by 2)" becomes the Layout picker (`G`) and Four images (`Shift G`). A new step after step 3 says a series row can be dragged onto a viewport or clicked into the outlined one, and that an empty viewport reads "Drag a series here."
- `examples/folder-viewer/src/more.html`, `#series` "Several series at once". The status line and the closing tiers paragraph say the image page now shows several series side by side in its own grid, and keep linked scroll and synced window as not built until F-209.
- `examples/folder-viewer/src/use.html`, the opening note, "draws one image through the same `ocelli-pixel` code", becomes "draws each viewport's image". No chapter changes, because no library API changes.

## Open questions

1. **Keys for layout and tile mode.** Affects AC1, AC5 and F-213. Today `1` is 1 by 1 and `2` is the four-image view. F-213's backlog note says "Number keys currently choose the layout, so the binding needs one decision." Options: (a) F-208 frees `1` to `9` for F-213 presets, opens the layout picker on `G` and toggles tile mode on `Shift G`, (b) keep `1` and `2` and give F-213 another key set. Recommended (a), because research section 10 row 3 names number keys for presets. Owner: operator. Deferral leaves F-213 without a key plan.
2. **How marks follow a viewport.** Affects the Marks paragraph and B-series observations. Options: (a) marks belong to the image, so the same image in two cells shows the same marks, (b) marks belong to the cell. Recommended (a), which is today's keying and what F-215 will write as SR content per image. Owner: operator.
3. **Tile mode per viewport or global.** Affects AC10. Options: (a) per viewport, the button acts on the active cell, (b) one global mode for every cell. Recommended (a). Owner: operator.
4. **Click on a thumbnail.** Options: (a) a click loads into the active cell, as today, and a double-click does the same, (b) a click only selects and a double-click loads. Recommended (a), because it keeps 1 by 1 unchanged. Owner: operator.
5. **Filling new cells.** Options: (a) next series not already shown, else empty, (b) always empty. Recommended (a). Owner: operator.

### Answers

Given by the operator on 2026-11-05 in the S46 consolidated design round and relayed by the sprint lead.

1. Answered by the operator, shared with F-213 Q1: (a). Bare `1` to `9` select window rows (on CT, 1 soft tissue, 2 lung, 3 bone, then the file's windows, and on other modalities the file windows start at 1), which F-213 binds. `0` stays Fit. `G` opens the layout picker. `Shift G` toggles tile mode. AC13 pins it.
2. Answered: recommended (a), marks belong to the image, so the same image in two cells shows the same marks.
3. Answered: recommended (a), tile mode per viewport, and the button acts on the active cell.
4. Answered: recommended (a), a click loads into the active cell and a double-click does the same.
5. Answered: recommended (a), new cells take the next series not already shown, else they stay empty.

Constraints carried into this plan from the same round: browser checks use `OCELLI_PORT=0` and the printed URL, never port 8765. AC14 is the aspect measurement carried from the F-207 polish review. Tiles stay `pixelated` always. No question is left open.
