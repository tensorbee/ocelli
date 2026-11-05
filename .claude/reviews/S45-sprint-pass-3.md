# S45 review, pass 3

**Reviewed**: sprint/s45 after the tile-fault fix, on top of the pass 2 sentence corrections
**Author / reviewer**: implementation by the sprint host. Review by Claude Code, read only, with no shell.
**Result**: 0 defects, 0 smells, 0 nitpicks

## Acceptance challenged

- **The claim under test.** In the two-by-two grid, a refused preset cannot leave the previous window's grey on screen.
- **Plan sentence.** `.claude/plans/F-207-design.md:126` reads "A failed preview hides the previous grey image and shows the fault sentence."
- **As-built sentence.** `docs/sprints/AS_BUILT.md:5065` reads "A failed preview hides the previous grey image and shows the fault, so the viewport does not keep the previous slice under the fault text."
- **Pass 2's scenario, traced again.** Take a VOI LUT Sequence series in grid mode 2 and press Lung.
  - `cacheKey` includes the preset (`page.html:1089-1094`), so nothing comes from the cache. `showIndex` starts `paintTiles` (`page.html:1045`), which takes a fresh `tileToken` (`page.html:1239`). It then awaits the main fetch (`page.html:1061`).
  - Tile 0 and the main image use the same key, so they share one inflight job (`page.html:1134-1135`) and get the same rejection.
  - If the tile catch runs first, the token is still current, so tile 0 becomes 1 by 1 (`page.html:1268-1270`). The loop then moves on to tile 1. Next the `showIndex` catch passes its `loadToken` check (`page.html:1065`) and calls `showFault` (`page.html:1069`).
  - If the `showIndex` catch runs first, `showFault` bumps `tileToken` (`page.html:1364`). The tile catch then sees a stale token and returns (`page.html:1268`).
  - Either way, `showFault` hides `tiles` (`page.html:1365`) and sets every tile canvas to 1 by 1 (`page.html:1366-1369`). Setting a canvas width clears its bitmap, so no earlier pixels are left.
  - Any later tile fetch, whether it succeeds or fails, meets a stale token at `page.html:1252` or `page.html:1268` and returns without drawing.
- **Result.** In this scenario the grid ends hidden and blank, with the fault text showing. Both sentences are now true in grid mode 2 as well as one-up.

## Evidence

Everything below comes from reading files. I ran no commands. I read:

- the pass 2 review, and the pass 1 Nitpicks section
- `page.html` lines 1000-1194 and 1195-1394. These cover `setGrid`'s tail, `showIndex`, `applyGrey`, `cacheKey`, `takeGrey`, `rememberGrey`, `fetchGrey`, `requestGrey`, `drawGrey`, `syncStage`, `paintTiles`, `showFault`, `hideFault` and `clearView`.
- grep results for every `syncStage(`, `applyGrey(`, `paintTiles(`, `tiles.hidden` and `setGrid(` call in `page.html`
- the two sentences, found by grep

The author reports that the folder-viewer tests passed. That is not evidence here, and I could not run them.

## Defects

None.

## Smells

None. Pass 2's smell 1 is resolved by `page.html:1364-1369` and `page.html:1267-1271`.

## Nitpicks

None new. Pass 1's two nitpicks are about `README.md:219-221` and the Length sentence at `page.html:467`. This edit touches neither, so both are still as stated and I did not reopen them.

## Verified clean

- **Nothing un-hides stale tiles after a fault.**
  - `syncStage` sets `tiles.hidden = gridMode !== 2` without checking for a fault (`page.html:1234`). It is called only from `drawGrey` (`page.html:1224`), which is on the success path after `hideFault` (`page.html:1074`), and from `setGrid` with `silent` (`page.html:1007`).
  - The only `silent` caller switches to mode 1 (`page.html:751`), and `setGrid(2)` always goes through `showIndex` (`page.html:659`, `page.html:1006`).
  - Even if tiles were shown again under a fault, the canvases would already be 1 by 1.
- **Overlapping loads.** A late failure from an earlier `showIndex` stops at the `loadToken` check (`page.html:1065`), so it cannot call `showFault` over a newer load. A newer failure bumps `tileToken`, which stops the older `paintTiles` loop.
- **Single failed tile.** When the main image succeeds and one tile fails, that tile is now blanked in place (`page.html:1269-1270`) and does not keep old pixels. This is the second fix pass 2 suggested.
- **Non-monochrome path.** `showIndex` still hides `stage` and `tiles` before `showFault` (`page.html:1038-1042`). `showFault` now also blanks the canvases, which does no harm there.
- **`drawGrey` with no context.** That `showFault` call (`page.html:1210`) now blanks the tiles too. That is consistent with the sentence.
- **Recovery.** A later success in grid mode 2 always runs `paintTiles` first (`page.html:1045`) with a new token, so the blank canvases are repainted with current pixels.

## Limitations and handoff

- I did not run the page. The microtask ordering above is argued from the code, and I showed the result is the same under both orderings.
- I did not see the diff. I relied on the brief that only `showFault` and the `paintTiles` catch changed.
- I did not recheck the CSS for how a 1 by 1 transparent canvas is drawn in a tile. It holds no earlier pixels either way.
- **Observation, not a finding.** A single failed tile next to a successful main image is blank, and no fault text is shown for it. The plan sentence names the fault sentence for a failed preview. A reader could take that to cover a lone tile, and nothing on screen says why the tile is empty. This is not stale pixels, and it is the fix pass 2 proposed, so I am not counting it. The author can decide whether the sentence should say that it is about the main image.
- **Next step:** this pass found nothing, so the microscope loop can stop on this change.

COUNTS defects=0 smells=0 nitpicks=0
