# F-207 review, pass 1

**Reviewed**: working tree on sprint/s45 at 7f4a5ce, plus uncommitted page.html, more.html, use.html, serve.rs, tests/preview.rs, and README.md
**Author / reviewer**: implementation by the sprint host. Review by Claude Code, read only, with no shell.
**Result**: 2 defects, 3 smells, 5 nitpicks

## Acceptance challenged

- AC1 holds. `tests/preview.rs` checks Freeform, Rectangle, Ellipse, and Remove.
- AC2 holds for the page. The test only checks that `system-ui` and `#141414` are present. A page that still had Georgia or `#16130f` beside them would pass. See S3.
- AC3 holds. `/more` and `/use` return HTML through `html_page` and the test rejects a canvas.
- AC4 holds for the page. The ctrlKey branch calls `preventDefault`. The test matches those two strings anywhere. See S3.
- AC5 holds. The README describes the 40 pixel threshold, pinch zoom, and the sideways swipe.

## Evidence

- No command was run. The author's report of 31 passing folder-viewer tests and a clean clippy was not repeated.
- The design, the three pages, `serve.rs`, the page contract tests, and the README folder-viewer steps were read.
- `DicomwebClient` takes `studiesServiceUrl`, `wadoUriUrl`, `fetch`, `wasm`, and `sink`. It has no `baseUrl`.
- `packCommands` takes a viewport id and a command list. `OcelliViewport` accepts a null session.
- `LutChain::map_into` and `parse_part10` exist at the paths the Use page names.
- The folder-viewer crate has no `unsafe` and no `wasm-bindgen`.
- The three pages do not contain the preset numbers 400, 600, 1500, or 300.

## Defects

### D1, the page does not say what the marks are

**Where**: `examples/folder-viewer/src/page.html` mark sentences, and `more.html` documents section.

**What**: Hovering a drawing tool says only "It is not saved." A search of the three pages finds no `reload` and no `SR`.

**Why it is wrong**: The plan says the page says the marks are drawings in the browser and that they are gone on reload. The review contract also requires the page to say they are not DICOM SR and are not written to the file. HLD decision D13 is that an annotation type is a DICOM SR content tree. These marks are not that type. "Not saved" does not say so.

**Evidence**: Read of the sentences and of `more.html`. The README has the reload and file sentences. The README is not the page.

**Scope and regression**: Anyone using only the image page. Present on this working tree.

### D2, the text selection box ignores flip and rotate

**Where**: `examples/folder-viewer/src/page.html` `noteBounds` and `paintSelection`.

**What**: Place a text note, flip horizontally, and select it. The glyphs are counter-transformed so they stay upright. The dashed box is still built from the anchor toward image +x and image -y. Under a horizontal flip the box sits on the other side of the glyphs. A vertical flip puts it below the baseline. A 90 degree turn moves it off the glyphs.

**Why it is wrong**: The layout spec draws the dashed box 4 CSS pixels outside the mark. The glyphs already follow the upright rule. The chrome has to follow the same counter-transform.

**Evidence**: Read of `placeText` against `noteBounds`. No browser click.

**Scope and regression**: Text selection after flip or rotate. Rectangle, ellipse, freeform, and length boxes stay in image space, which is what the spec asks for those marks.

## Smells

### S1, the draft stroke has no black underlay

**Where**: `paintDraft` strokes one node in the ink colour at 2px. A committed mark draws a 4px black underlay at opacity 0.8 and then the colour.

**What**: White or lemon ink on a bright grey is nearly invisible until the pointer is released. The layout spec gives that as the reason for the underlay.

### S2, the note field is outside the full-screen element

**Where**: Full screen is requested on `#wrap`. The note field is a child of `body`, outside `#wrap`.

**What**: In full screen a text click focuses a field the browser does not draw. Enter or blur can commit text the user never saw. Edit text has the same path.

### S3, the page-contract tests do not reject the named wrong pages

**Where**: `tests/preview.rs` `the_page_offers_a_folder_picker`.

**What**: Presence of `system-ui` and `#141414` does not fail if Georgia or `#16130f` remains. Presence of `if (event.ctrlKey)` and any `preventDefault` does not fail a handler that still steps once per wheel event. The plan names both of those as the wrong implementations. A test that stays green for the wrong page will be counted as coverage.

## Nitpicks

### N1

`README.md` says length shows millimetres when the file gives Pixel Spacing, and pixels when it does not. A file with only Imager Pixel Spacing shows "mm at the detector".

### N2

Edit text opens the field at the top left of the viewport plus 24 pixels. The layout spec opens it at the note. The plan is silent. This does not block.

### N3

`use.html` imports `DicomwebClient` and the sample does not call it. The prose says "wasm memory" and "a sink" for the fields `wasm` and `sink`. The statement is accurate.

### N4

The headers sheet uses a black scrim at 0.72 opacity. The layout spec refuses translucent panels over the image. The scrim is equal-channel and is only the modal backdrop.

### N5

`gridAllowed` checks only the current file. The other three tiles can come from a larger file in the same series.

## Verified clean

- The page sends `soft_tissue`, `lung`, or `bone` and does not send a preset together with a centre, a width, or a function. It does not store the preset numbers.
- The sans stack, the 13px page, and the 15px static pages match the plan. Chrome hex values have equal channels. Fault text is the only use of `#f2786d`.
- Ink is the five named colours. Committed strokes are an underlay, then the colour, then a transparent hit stroke. Shapes use no fill.
- The wheel normalises `deltaMode`, steps after 40 pixels, resets on a direction change or 150ms, and applies the index once per frame. Ctrl zoom calls `preventDefault` before the image check. A horizontal wheel returns after `preventDefault`.
- Anchored zoom matches `pan + (q - C) * (1 - s' / s)` and clamps to 0.05 times fit through 40 times fit.
- Right drag and Ctrl plus left drag pan. Middle drag zooms from the press. There is no Safari gesture event.
- The inverse matches CSS flip-then-rotate. Freeform needs at least three points. A horizontal length is kept. Text sizes are 12, 15, 20, and 28. Rendered size is `note.size / scale`.
- Length is the hypotenuse of column delta times column spacing and row delta times row spacing, from the preview headers.
- A slice change clears the selection. Remove, Delete, Backspace, and the row control delete it. Empty new text is dropped. Escape in full screen returns without cancelling a draft.
- 2 by 2 uses the same preview. Drawing is refused there with the sentence "Drawing works in the single view."
- `#stage` stays `flex: none`. The main canvas is not stretched with `width: 100%`.
- `/more` names the refused behaviour with no picture and no result number. `/use` names the real calls and says this folder viewer is not the wasm path and that nothing is published.
- Window centre follows the vertical drag and width follows the horizontal drag.

## Limitations and handoff

No shell, so the tests were not repeated. No browser, so gestures were read from source. Double-click edit was not executed. The next pass should see the fixes for D1, D2, S1, S2, and S3.

COUNTS defects=2 smells=3 nitpicks=5
