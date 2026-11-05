# S45 review, pass 2

**Reviewed**: sprint/s45 after the pass 1 recovery-sentence fix, on top of d242505
**Author / reviewer**: implementation by the sprint host. Review by Claude Code, read only, with no shell.
**Result**: 0 defects, 1 smell, 0 nitpicks

## Acceptance challenged

- **Plan sentence.** `.claude/plans/F-207-design.md:126` now reads "A failed preview hides the previous grey image and shows the fault sentence."
- **As-built sentence.** `docs/sprints/AS_BUILT.md:5065` now reads "The recovery sentence was corrected. A failed preview hides the previous grey image and shows the fault, so the viewport does not keep the previous slice under the fault text."
- **One-up path.** On a cache miss, `showIndex` hides `stage` and `frame` before the fetch (`page.html:1055-1056`). The catch calls `showFault` (`page.html:1069`), which hides both again (`page.html:1360-1361`). In one-up mode both sentences match the code.
- **Non-monochrome path.** `showIndex` hides `stage` and also `tiles` before it calls `showFault` (`page.html:1038-1042`). This matches both sentences.
- **Two-by-two path.** This one does not match. See smell 1.

## Evidence

Everything below comes from reading files. I ran no commands. I read:

- the pass 1 review
- `F-207-design.md` lines 113-126, by grep
- the F-207 section of `AS_BUILT.md`, lines 5043-5065
- `page.html`: `showIndex`, `applyGrey`, `syncStage`, `paintTiles`, `showFault`, `hideFault`, `cacheKey`, `fetchGrey`, `applyPreset`, the `#frame` and `#tiles` markup and their CSS

I did not see the diff. I am relying on the brief that only the two sentences changed.

## Defects

None.

## Smells

1. **The corrected recovery sentence is true in one-up mode and false in two-by-two mode.**
   - `#tiles` is a sibling of `#frame` and is not inside it (`page.html:336-347`). `showFault` hides only `stage` and `frame` (`page.html:1360-1361`). In grid mode 2 `tiles.hidden` stays false.
   - On a cache miss in grid mode 2, `showIndex` calls `paintTiles` (`page.html:1045`). A tile whose fetch fails reaches the catch at `page.html:1267-1269`, which does nothing. The canvas therefore keeps the pixels it drew last.
   - `cacheKey` includes the preset (`page.html:1089-1094`), so pressing a preset always fetches again for every tile.
   - Failure scenario: open a series whose frames carry a VOI LUT Sequence. Switch to the two-by-two grid and press Lung. The server refuses every request, as pass 1 traced (`preview.rs:555-557`). The fault sentence appears, but all four tiles still show the previous window's grey images, with the Lung chip active. The plan says the previous image is hidden, and the as-built says the viewport does not keep the previous slice under the fault text. Neither is true here.
   - A related case reaches the same catch. If the main image succeeds and one tile fails, that tile keeps its old pixels and no fault is shown.
   - Pass 1 called this kind of result the quietly wrong picture, so I recommend fixing the code rather than narrowing the sentences. Two small fixes would work. One is to hide `tiles` in the `showIndex` catch the way the non-monochrome path does at `page.html:1039`. The other is to blank the tile canvas in the `paintTiles` catch, which also covers a single failed tile. The other choice is to limit both sentences to the one-up view, but that leaves stale tiles on screen.

## Nitpicks

None new. The edits did not make either pass 1 nitpick false, so I did not reopen them.

## Verified clean

- **One-up agreement.** On the paths the brief named, the plan sentence, the as-built sentence and `showIndex` with `showFault` now agree.
- **Cache-hit path.** `applyGrey` calls `hideFault` first (`page.html:1074`), so a cached success clears an earlier fault. This is not a failure path, so the sentence does not apply to it.
- **Stale replies.** The `loadToken` check in the catch (`page.html:1065`) stops a late failure from an earlier slice from hiding a newer image.
- **Second as-built sentence.** "A drawing-tool drag moves the mark and a navigation drag does not" matches state-table rows 117 and 118 of the plan. I checked the text only, not the drag code.

## Limitations and handoff

- I did not run the page. The two-by-two behaviour comes from reading the source only.
- Without a shell I could not check whether two-by-two mode or `paintTiles` predates S45. The smell stands either way, because the S45 sentences make the claim.
- I did not check whether `#fault` is drawn over `#tiles` or under it. The tiles stay visible in both cases.
- **Next step:** fix the tile path in `page.html`, or narrow the two sentences to the one-up view. Then run a pass 3 on that change.

COUNTS defects=0 smells=1 nitpicks=0
