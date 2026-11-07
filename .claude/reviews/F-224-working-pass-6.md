# F-224 review, pass 6

**Reviewed**: `git diff 48f861b8 afc6da53`, one commit afc6da53 "F-224, review pass 5 remediation for the folder viewer half", in worktree `ocelli-wt-f-224`, branch `work/f-224-xai`, HEAD `afc6da536f448c320e7dba636d6c1efab2cb165b`, tree `28786c6c6f621dd274865542ebc64683ad5b5897`. Three paths: `examples/folder-viewer/tests/browser/changed.mjs`, `README.md` line 191 and `examples/folder-viewer/src/use.html` line 771. No unstaged tracked paths. Untracked `.venv` and `tools/oracle/out` were present and are not part of the review. Normative source: `.claude/reviews/F-224-working-pass-5.md` S1 and N1 to N3, and `.claude/plans/F-224-design.md` decision 10.
**Author / reviewer**: remediation by the implementer named in the lead's brief. The commit author field is Atul Sharma, so authorship rests on the brief. Reviewer Claude Opus 5.5 as an independent `/microscope` subagent. It wrote none of the F-224 material.
**Result**: 0 defects, 0 smells, 1 nitpick

## Acceptance challenged

Every mutation ran in a scratch copy of HEAD made with `git archive` at `scratchpad/f224-p6`, with its own `CARGO_TARGET_DIR`, a release build, `OCELLI_PORT=0`, synthetic folders only and one browser at a time. Each mutation was applied by exact string match with a count of one, the page rebuilt, `changed.mjs` run and the page restored. The copy was compared with the worktree after the last run and then deleted with `shutil.rmtree`.

- **S1, the empty cell.** `observeCells` now picks layout 3 by 1, and after A's count clicks `.cell[data-cell="2"]`. On HEAD `emptyStatus` is `2 series.`. M5 (`noteChanged();` removed from `setActive`) and M5b (`if (seriesOf(activeViewport())) noteChanged();`) each turn `changed.mjs` red with the single line `after the empty cell became active the status read "2 series. 1 file of this series changed on disk after the folder was opened."`. Both now fail for the reason pass 5 named.
- **The click cannot pass vacuously.** A scratch edit printed the page state right after that click: `activeCell` 2, three `.cell` nodes, only cell 2 carries `active`, `seriesOf(viewports[2])` false and `seriesOf(viewports[1])` true. The mutation pair proves the same independently. A click landing on cell 0 leaves the count on HEAD, so HEAD would be red. A click landing on cell 1 runs a probe whose own `noteChanged()` drops the count, so M5 would be green. HEAD green and M5 red leave only cell 2. A missing `emptyStatus` defaults to `"changed"` and is refused.
- **N2, the changed style after a step.** M7 (`labels[index].classList.remove("changed");` removed from `paintTiles`) turns `changed.mjs` red with `after a step tile 2 read "Image 3" with the changed style on`. The step is real: after `ArrowDown` tile 1 holds file 1 and keeps the sentence, and tile 2, which held the changed file 1, holds file 2. The `some` precondition refuses a step that never moves a changed tile onto an unchanged image, and empty `labels` fail it, so the per-tile loop cannot pass over nothing.
- **N3, image 1's canvas in the thumb case.** The implementer reported no isolating mutation. Two exist and both are caught by the new check alone. M8 makes `markChanged` clear the view canvas of every shown viewport that does not show the changed file. M8b sets that canvas to 1 by 1 instead. In both, image 1's fault text stays `""`, so the pass 5 check passes, and the `changed`, `late`, `tiles` and `cells` cases stay green because none has a shown viewport off the changed file. The run goes red with 16 lines of the form `image 1, whose file did not change, at (0, 0) is 0, not 87` for M8, and `is undefined, not 96` and so on for M8b. This is the blank canvas with no fault that pass 5 N3 described.
- **What the thumb check cannot see.** Every synthetic file carries the same 64 stored values except the rewritten point (5, 2), so image 1 drawn from another unchanged file's grey or from the changed file's old grey is identical to image 1 drawn correctly. The header promises only that "image 1 must still draw", which the check now holds. No finding.
- **README line 191 and `use.html` line 771 executed.** "In Four images the sentence is in that file's tile and the other tiles still draw" and "its tile in Four images" name the control `#tile-mode`, whose `aria-label` and visible text are "Four images". A scratch run of `observeTiles` with `page.locator("#tile-mode").click()` in place of `Shift+G` gave `8 1 8 8` then `1 1 8 8`, tile 2 then tile 1 reading the sentence, and the run green. Neither file nor `more.html` still says "four-image view". The remaining phrase is a source comment at `page.html:2001`.
- **Comments and doc comments in `changed.mjs`.** The header's `tiles` and `cells` lines, the `thumb`, `tiles` and `cells` shapes in the `seen` doc comment, and the comments above the empty-cell click and the step match what the observers return and what `changedProblems` checks.

| Check | Source | When | Result |
|-------|--------|------|--------|
| `node changed.mjs` baseline | scratch copy of HEAD | now | 0. cells `empty "2 series."`, tiles `8 1 8 8` then `1 1 8 8`, thumb `2391296` then `0` |
| M5, `noteChanged();` removed from `setActive` | scratch copy | now | 1, the empty cell status line |
| M5b, `noteChanged` guarded on the active series | scratch copy | now | 1, the empty cell status line |
| M7, `classList.remove("changed")` removed | scratch copy | now | 1, tile 2 "Image 3" with the changed style on |
| M8, other viewports' canvas cleared in `markChanged` | scratch copy | now | 1, 16 points of image 1 read 0 |
| M8b, other viewports' canvas set to 1 by 1 in `markChanged` | scratch copy | now | 1, 16 points of image 1 read 0 or undefined |
| Click target and button-driven tiles | scratch copy, temporary edit to `changed.mjs` | now | 0, active cell 2 with no series, tiles as baseline |
| `bin/ocelli.sh gate viewer-browser` | worktree HEAD, node 24.13.0 | now | 0, ALL GREEN, the five scripts as in pass 5 plus `empty "2 series."` |
| `cargo test -p ocelli-folder-viewer` | worktree HEAD | now | 0. 202 passed, 0 failed |
| `npx eslint changed.mjs eslint.config.js` | worktree HEAD | now | 0 |
| `prose_check.py` | worktree HEAD | now | voice rules clean over 925 files |

## Defects

None.

## Smells

None.

## Nitpicks

- **N1.** The `viewer-browser.changed` entry's `refuses` text in `scripts/guards/catalogue.py` lists what `changedProblems` refuses but does not name the two refusals this commit added in substance: a tile that keeps the changed style after it shows an unchanged image, and image 1's canvas differing from the process grey. The thumb one is loosely covered by "another image of the series that does not draw", and the empty cell one by "a status that counts another series after a viewport becomes active". The list does not say it is complete, so no sentence is false.

## Verified clean

- **Pass 5 S1 closed.** The empty cell effect of `noteChanged` in `setActive` is watched in the browser, and both M5 and M5b go red there.
- **Pass 5 N1 closed.** README and `use.html` use the control's name.
- **Pass 5 N2 closed.** M7 goes red.
- **Pass 5 N3 closed.** A canvas check now backs "image 1 must still draw", and M8 and M8b show it isolates that claim.
- **Boundary and tier.** The commit touches no Rust, no page code and no arithmetic, so no PS3.3 fixture is owed.

## Limitations

- Mutations were run in `changed.mjs` only. The other four `viewer-browser` scripts ran once on HEAD through the gate.
- The thumb case cannot tell image 1 from another unchanged file of its folder, as above.
- Only Chromium headless from `tools/bench`'s pinned Playwright was exercised.
