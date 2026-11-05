# F-207 review, pass 2

**Reviewed**: working tree on sprint/s45 at 7f4a5ce after the pass 1 remediation
**Author / reviewer**: implementation by the sprint host. Review by Claude Code, read only, with no shell.
**Result**: 1 defect, 2 smells, 6 nitpicks

## Acceptance challenged

- AC1 holds. The page test requires Freeform, Rectangle, Ellipse, Text, and Remove.
- AC2 holds. The test requires `system-ui` and `#141414`, and it fails if Georgia or `#16130f` is present.
- AC3 holds. `/more` and `/use` are static HTML and the test rejects a canvas.
- AC4 holds. The test requires `preventDefault` as the first statement of the ctrlKey branch.
- AC5 holds. The README describes the 40 pixel threshold, pinch zoom, and the sideways swipe.

## Evidence

- No command was run. The author's report of 31 passing tests and clean clippy was not repeated.
- The design, the pass 1 note, the three pages, the page contract test, the routes, and the README folder-viewer steps were read.
- The preset numbers are not in the pages. `font-weight: 600` and the words TID 1500 are not window values.

## Defects

### D1, forcing the single view can show the previous slice

**Where**: `setTool` calls `setGrid(1, true)`. `syncStage` unhides the stage whenever `imageWidth` is set and there is no fault.

**What**: Open 2 by 2 and move to a slice that is not cached. `showIndex` hides the stage and waits on the preview. Press `f` before that preview returns. The silent grid change unhides the stage. The canvas still holds the previous slice. The corners and the marks belong to the new slice. A freeform stroke is stored on the new slice while the user is looking at the previous pixels. When the preview returns, those pixels appear under the mark. The same unhide is in `zoomAt`, so `=` during a single-view load can show a stale frame. This path also skips `paintEdges`, so orientation letters stay hidden until the next slice change.

**Why it is wrong**: In the single view the stage stays hidden until the grey for the shown index arrives. The forced path skips that. The image is quietly not the slice the label names. CLAUDE.md treats a quietly wrong image as the defect that matters.

**Evidence**: Traced from the `f` key through `setTool`, `setGrid`, `syncStage`, and `layout`. Not run in a browser.

**Scope and regression**: New with the forced single view. Leaving the grid through `showIndex` waits for the shown slice. Clearing `imageWidth` while a preview is in flight stops `zoomAt` from revealing the previous canvas.

## Smells

### S1, a drawing-tool click has no dead zone

**Where**: `bindShape` starts a move on pointer down. `moveNote` applies every later move. `pointsMoved` treats a shift above 0.1 image pixels as a real move and then `renderInk` replaces the node.

**What**: A navigation tool ignores the first 5 CSS pixels. A drawing tool moves the mark on a 1 pixel drift, which also replaces the node the double-click edit needs. A still pointer keeps the node. A trackpad tap may not.

### S2, the plan's state row does not match the drawing-tool drag

**Where**: The design state table says that for any tool a longer drag runs the tool and does not move the mark. The page moves the mark while a drawing tool is active. README step 8 says the same thing as the page.

**What**: The approved row is now false. The plan should say that a navigation drag does not move the mark, and that a drawing tool does. Leaving the row unchanged keeps a false contract in the plan.

## Nitpicks

### N1

The test accepts `wheelAcc` and `>= 40` anywhere. Moving `queueSlice(sign)` outside the threshold check would stay green. One string that includes the threshold, the reset, and `queueSlice(sign)` would tie them together.

### N2

The length sentence on the page still says millimetres or pixels. That is the plan's sentence. The README and the label also say millimetres at the detector.

### N3

In 2 by 2 the drawing buttons are disabled and their sentence says drawing works in the single view. The keys `f`, `r`, `e`, `t`, and `l` switch to the single view.

### N4

Edit text still opens at the top left of the viewport plus 24 pixels.

### N5

`use.html` imports `DicomwebClient` and the sample does not call it.

### N6

The headers sheet still uses the black scrim.

## Verified clean

- Pass 1 D1 is closed. The marks section says the drawings are in this browser, are not DICOM SR, are not written to the file, and are gone when you reload.
- Pass 1 D2 is closed. Text chrome uses the same upright transform as the glyphs. Rotate and both flips rebuild the ink after layout.
- Pass 1 S1 is closed. The draft has a black underlay at 4 pixels and opacity 0.8, then the colour.
- Pass 1 S2 is closed. The note field is inside `#wrap`. Its pointer down does not start a drag.
- Pass 1 S3 is closed for Georgia, `#16130f`, and a wheel handler with no accumulator. N1 is the remaining gap.
- Pass 1 N1 is closed in the README. Imager Pixel Spacing is millimetres at the detector.
- Pass 1 N5 is closed. The grid checks the same four images the tiles paint, and a large neighbour leaves the single view.
- A click that does not move the mark does not replace the SVG node. Escape in the note field stops before the window handler. The preset, wheel, zoom, length, and window-drag contracts still hold.

## Limitations and handoff

No shell and no browser. The next pass should see D1 fixed, including the `=` path, a 5 pixel dead zone on a drawing-tool move, and a plan row that matches that move.

COUNTS defects=1 smells=2 nitpicks=6
