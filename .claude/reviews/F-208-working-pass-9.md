# F-208 review, pass 9

**Reviewed**: `git diff 5c8b28f7 HEAD` on `work/f-208-xai` in `ocelli-wt-f-208`, HEAD `a8d0ccd9f2d7ca11449a9ca10873baf08a239c4c`, tree `502f2b6cd8fe8c58b269200a637e1f7d7886a86d`. Nothing staged or unstaged before or after the pass. The untracked paths are the pass 1 to 8 reports, present before this pass, and this report.
**Author / reviewer**: the commit under review is by a Claude implementer agent. This reviewer is a separate Claude agent (`/microscope F-208 --working`, pass 9) and wrote none of the diff.
**Scope**: this diff only. Pass 8 checked the page fix and every earlier item.
**Result**: 0 defects, 0 smells, 0 nitpicks

## Pass 8 items

- **page.html unchanged.** `git diff 5c8b28f7 HEAD --name-only` lists `examples/folder-viewer/tests/browser/readout.mjs` alone.
- **D1, the header count. Closed.** Line 23 now says "Four more cases, each on a fresh page:", and the list under it is `empty`, `throw`, `fault` and `gap`. `observeReadout` opens `fresh`, `thrownPage`, `faultPage` and `gapPage` for them, so "fresh page" holds for all four. The `gap` entry now says the magnifier first shows over tile 1's picture, which is what `tileGapCase` does.
- **S1, the gap case passed with the magnifier never on. Closed.** `tileGapCase` now moves to the centre of the first `.tile`'s canvas (`canvas[data-tile="0"]`, page.html:700) and records `overPicture` as `!mag.hidden` after 400 ms, before it moves into the strip. `noPictureProblems` checks `overPicture` first and refuses with "the magnifier did not show over tile 1's picture first, so the case shows nothing". So a hidden magnifier over the gap counts only after the magnifier was seen on.
  - **Replay of the m-press-removed mutation (M1).** I made a scratch `git archive HEAD` with `git show b6d2fcca:` page.html (checked with `cmp`), gave it its own `CARGO_TARGET_DIR`, and removed the `m` press from `tileGapCase` alone. Run against b6d2fcca's page it exits 1. The gap line reads `over tile 1's picture first false, in the gap true, magnifier hidden` and the refusal is the `overPicture` one, not the strip one. At pass 8 the same mutation stayed green.
  - **The unmutated HEAD check still discriminates.** On the same scratch build it exits 1. The gap line reads `over tile 1's picture first true, in the gap true, magnifier shown` and the refusal is "the magnifier showed over the strip between tiles". The faulted case is red too, which is the expected result on b6d2fcca. So the positive control passes on a page whose magnifier works, and the negative half still sees pass 7 D2.
- **N1, the moved doc comment. Closed.** The `empty`/`thrown` JSDoc now sits directly above `faultProblems`, which takes `(empty, thrown)`. The block above `noPictureProblems` names `faulted` and `gap`, which are its parameters, and the `gap` shape it gives, `{ name, overPicture, inGap, magHidden }`, is the object `tileGapCase` returns.

## Other text in the diff

- The comments inside `tileGapCase` ("First over the middle of tile 1's picture", "Then over the strip between that picture and the tile's edge") match the coordinates: `pictureX` is the canvas centre, `x` is midway between `picture.right` and `box.right`.
- The new log line in `main` prints `overPicture`, and the gate shows it.
- The catalogue and runbook LIMIT ("tile-gap branches have no probe") is still true, because the new branch is in the same case and nothing probes it. README step 8, use.html and more.html say nothing this diff touches.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `npm run lint` | tree 502f2b6 | now | 0, no output from eslint |
| `bin/ocelli.sh gate viewer-browser` | tree 502f2b6, OCELLI_PORT=0, CARGO_TARGET_DIR=wt/target, viewers 91183 and 91224, Chromium 151.0.7922.34 | now | 0, ALL GREEN. Gap `over tile 1's picture first true, in the gap true, magnifier hidden`. Faulted `hidden readout true cross true magnifier true, 0 POST /value` |
| `python3 scripts/prose_check.py` | same | now | 0, "voice rules clean over 895 file(s)" |
| `python3 scripts/source_provenance_check.py` | same | now | 0, "clean over 1758 files" |
| HEAD readout.mjs against b6d2fcca's page | scratch archive of HEAD, own CARGO_TARGET_DIR, viewer 94300 | now | 1, gap "magnifier showed over the strip", faulted red |
| M1, no `m` press in `tileGapCase`, against b6d2fcca's page | same scratch, viewer 95153 | now | 1, gap refused for "did not show over tile 1's picture first" |

## Limitations and handoff

- One synthetic CT series and device pixel ratio 1, as in pass 8. M1 was not run against HEAD's page, because with the magnifier never on `overPicture` is false whatever the page does.
- No oracle, GPU or `--sprint` profile. Ports 8742 and 8765 not used, PID 98891 not touched, nothing killed by name. Viewers 91183, 91224, 94300 and 95153 were stopped by their PIDs through `stopByPid`, and `ps -p` confirms all four are gone. I deleted the scratch copy and its target directory. The run logs are in this session's scratchpad.
- Tree unchanged: HEAD `a8d0ccd9`, tree `502f2b6`, nothing staged or unstaged.
- This is a bounded review of one diff, not sprint readiness.

F-208 pass 9: 0 defects, 0 smells, 0 nitpicks.
