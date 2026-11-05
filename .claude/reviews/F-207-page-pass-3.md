# F-207 review, pass 3

**Reviewed**: working tree on sprint/s45 at 7f4a5ce after the pass 2 remediation
**Author / reviewer**: implementation by the sprint host. Review by Claude Code, read only, with no shell.
**Result**: 0 defects, 0 smells, 3 nitpicks

## Acceptance challenged

- AC1 holds. The page test checks for Freeform, Text, Rectangle, Ellipse, and Remove at `examples/folder-viewer/tests/preview.rs:178` and `:181`.
- AC2 holds. The test checks for `system-ui` and `#141414`. It fails if Georgia or `#16130f` is present.
- AC3 holds. `more_and_use_are_static_pages` fails if either route contains a canvas.
- AC4 holds. The test needs `preventDefault` as the first statement of the ctrlKey branch. This matches `page.html:1445` to `:1446`.
- AC5 holds. README step 3 gives the 40 pixel threshold. Step 6 covers pinch, Ctrl with the wheel, and the sideways swipe. Step 8 now says that a drag moves a mark only while a drawing tool is active.

## Evidence

- No command was run. The author says the folder-viewer tests pass and clippy is clean. That claim is not repeated here as evidence.
- I read the design plan, the pass 2 note, all of `page.html`, the page contract test, and README lines 169 to 228.
- The D1 trace starts at the `f` key and at `=`. It goes through `onKey`, `setTool`, `setGrid`, `showIndex`, `syncStage`, `zoomCentre`, `zoomAt`, `layout`, `onPointerDown`, and `clientToImage`. Nothing was run in a browser.

## Defects

None.

## Smells

None.

## Nitpicks

### N1, the drawing-tool dead zone does not latch

**Where**: `onPointerMove` at `page.html:1550` to `:1554`.

**What**: Each move during a mark move checks travel against the press point. The check is never latched. Suppose the pointer goes 10 pixels out and then comes back within 5 pixels of the press point. Those later moves are ignored. The mark stays at its last position outside the circle and does not sit under the release point. The navigation path does latch, because `drag.markId` is cleared at 5 pixels (`:1559`). A `started` flag on `move`, set the first time travel reaches 5, would make both paths behave the same. S1's stated requirement still holds.

### N2, a frame showing the colour fault still accepts a drawing

**Where**: The non-monochrome branch of `showIndex` at `page.html:1035` to `:1043`, and `clientToImage` at `:1682`.

**What**: That branch hides the stage but does not clear `imageWidth`, `imageHeight`, or `scale`. With a drawing tool in the single view, a press on the fault sentence passes `onPointerDown`. `clientToImage` then returns a point computed from the previous image's size and scale. A Freeform with three or more points is stored under the colour frame's key. It shows up in the marks list but is drawn nowhere. The mark is on the right image, so pass 2 D1's wrong-image case does not apply. The coordinates are still meaningless. This branch was not touched by the remediation and probably predates pass 2. It belongs to the same family as D1, since `zoomAt` is guarded by `stage.hidden` but `clientToImage` is not.

### N3, the drawing buttons stay disabled after the automatic exit from 2 by 2

**Where**: `showIndex` at `page.html:1028` to `:1033`.

**What**: Moving to a slice with a neighbour over 32 MiB drops the page to the single view. That path changes `gridMode`, the two grid buttons, and `tiles`. It does not re-enable the five drawing buttons, which `setGrid` would do at `:1002` to `:1004`. The page is in the single view, yet Freeform, Rectangle, Ellipse, Text, and Length cannot be clicked. The keys and any navigation tool click bring them back, because `setTool` sets `disabled` again from `gridMode`.

Pass 2 N2 to N6 are still accurate as written and are not repeated here.

## Verified clean

- **Pass 2 D1 is closed, on both paths.**
  - Going from 2 by 2 to a drawing tool now sets `leavingGrid`. `setTool` then calls `setGrid(1, false)`, which calls `showIndex(viewIndex)` instead of `syncStage` (`page.html:743`, `:751`, `:1006`).
  - When the frame is not cached, `showIndex` sets `imageWidth` to 0 and hides `stage` and `frame` before it awaits (`:1053` to `:1056`). The awaited job is the same promise from `inflight`, so the request is not sent twice.
  - The stage is shown again only by `drawGrey` through `applyGrey`, once that slice's grey has arrived.
  - In the single view, `setGrid(1, true)` calls `syncStage`. `syncStage` needs `imageWidth > 0` and no fault, so it cannot show the stage while a slice is loading (`:1232`).
  - `=` reaches `zoomAt` through `zoomCentre`. `zoomAt` returns when `imageWidth` is 0 or the stage is hidden (`:1489`). `0`, rotate, and both flips reach `layout`, which returns on the same condition (`:1283`).
- **No mark lands on the wrong image while a slice loads.** With `imageWidth` at 0, `clientToImage` returns null. So the Freeform, Rectangle, Ellipse, and Length drafts are never created, and a Text click opens no field. A draft that was already in progress is cancelled by `showIndex` (`:1025`). The arrow keys and the wheel are refused while a draft exists.
- **Pass 2 S1 is closed.**
  - A drawing-tool press on a mark records the press point in `move` (`:1997` to `:2004`).
  - `onPointerMove` ignores any move under 5 CSS pixels (`:1551` to `:1552`).
  - On release with no travel, the points match the origin. `pointsMoved` returns false and only `paintSelection` runs, so the SVG node survives and a double-click can still open the editor.
- **Pass 2 S2 is closed.** The plan's state table now has separate rows for navigation tools and drawing tools (`F-207-design.md:117` to `:118`).
  - Navigation tool: a click selects the mark, and a drag past 5 pixels runs the tool without moving the mark. This matches `drag.markId` being cleared at `:1559` and the selection on release at `:1616`.
  - Drawing tool: a press selects the mark, which then moves with the pointer, and a drag under 5 pixels only selects. This matches `bindShape`.
  - README step 8 agrees with both rows.
- **Pass 2 N1 is closed.** The test now needs one string containing the threshold check, the accumulator reset, and `queueSlice(sign)` (`preview.rs:193`). This matches `page.html:1471` to `:1473`. Moving `queueSlice` out of the threshold block would turn the test red. The `queueSlice` call in the playing branch is intended, because a wheel event stops playback first.
- The remediation leaves the earlier contracts intact: presets send only a name, the wheel accumulator still works, Ctrl zoom still calls `preventDefault`, the length label uses the spacing source, and Escape in the note field still stops before the window handler.

## Limitations and handoff

There was no shell and no browser. The D1 and S1 behaviour comes from reading the code, not from running the page. The Rust test does not exercise gestures, as the plan says. A live click-through at sprint close should cover:

- pressing `f` in 2 by 2 on a slice that is not cached
- pressing `=` during a single-view load
- a drawing-tool tap on a mark followed by a double-click

None of the three nitpicks blocks closing F-207. N2 is the one worth fixing first, because it is the same guard gap D1 had.

COUNTS defects=0 smells=0 nitpicks=3
