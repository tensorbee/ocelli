# F-208 review, pass 8

**Reviewed**: `git diff b6d2fcca HEAD` on `work/f-208-xai` in `ocelli-wt-f-208`, HEAD `5c8b28f7d7560aa8deef8fd2e6c74e58b957d449`, tree `9bc0ba3ea1fe7fb5b85733414c5d6e101add2716`. Nothing staged or unstaged before or after the pass. The untracked paths are the pass 1 to 7 reports, present before this pass, and this report.
**Author / reviewer**: the commit under review is by a Claude implementer agent. This reviewer is a separate Claude agent (`/microscope F-208 --working`, pass 8) and wrote none of the diff.
**Result**: 1 defect, 1 smell, 1 nitpick

The code change is right. Every finding is in `readout.mjs`, the test file.

## Acceptance challenged

- **Pass 7 D1, a pointer over a faulted cell. Closed.** Single view, cross and magnifier on, the main preview made to fail, then `ArrowRight`. The cell shows "injected main failure". A move to the ring-edge point now shows no readout, no cross and no magnifier. The readout text stays at the bare `C 105  R 50` because no `POST /value` goes out. The same probe on b6d2fcca's page shows all three, with `40 HU`. The colour route reaches the same rule. The next image was marked non-monochrome and stepped to. The cell showed "this preview shows monochrome frames only", and a move showed nothing on HEAD. On b6d2fcca a move showed all three. In 2 by 1, a `showFault` on cell 1 hides all three on a move over cell 1, and cell 0 still shows `C 66  R 49  40 HU` with the cross and the magnifier.
- **Pass 7 D2, the strip between tiles. Closed.** In Four images the gap case reports the magnifier hidden on HEAD and shown on b6d2fcca. A third case of the same defect, not named in pass 7, is closed too. In Four images at index 9, tile 4 is emptied to a 1 px canvas and the pointer lands on `DIV.tile`. b6d2fcca showed the magnifier there at all three spots. HEAD hides it.
- **The fix cannot hide the readout over a visible picture.** The probe ran unchanged on HEAD and on b6d2fcca's page and its two logs were diffed. Every visible-picture line is identical in both logs. The lines differ only for no-picture states: the emptied tile 4, the faulted cell, the colour fault and cell 1's fault. The covered spots are below.
  - Single view, 128 by 96. Points 0.6 image pixels inside the left, right, top and bottom edges and the top-left and bottom-right corners show the readout, the cross and the magnifier, from `C 1  R 49` to `C 128  R 96`. The letterbox, a 150.6 px strip beside the image, shows none of them, the same as b6d2fcca, because `imagePoint` already returned null there.
  - Image wider than the cell, after four `=`. Points 3 px inside each cell edge are on the image, and all three show there (`C 6`, `C 123`, `R 17`, `R 80`).
  - Four images at index 0. Tiles 1 to 4 at the centre and 1 px inside the top-left and bottom-right corners: the magnifier shows at all twelve spots. The cross does not, which is the README rule.
  - Four images at index 9, the partial last page. Tiles 1 to 3 at the same three spots each: the magnifier shows. Tile 4 has no image: hidden.
  - After a fault clears. `requestGrey` was restored, then `ArrowRight` again. The fault hides and the stage shows at index 2. With the pointer still, nothing shows, which is the "next pointer move" rule. After a move, all three show with `40 HU`, and a second move gives the same magnifier hash.
  - After a colour fault, a step on to a monochrome image shows all three again after a move.
- **The moved tile-mode cases still test what they tested.** In Four images the old point, the cell centre, lands between the tiles' pictures. The old "tile mode on" re-show and "tile mode off" before-state needed the magnifier there, which is pass 7 D2. `quarterOf` lands on the single-view image at `C 37  R 33` and on tile 1's picture in Four images. The magnifier shows there on HEAD, which the fix allows only over a tile canvas. To check the cases still discriminate, the `leavePointerFor(vp)` on a tile-mode change was deleted (M2). Both cases went red with "the readout, cross or magnifier survived the change", after hidden `false/true` and `true/false`. So they still test the hide on the toggle and the re-show over a real picture.
- **Each half of the new condition, by mutation.** M3 dropped `vp.dom.stage.hidden`: the faulted case went red (shown, 2 POST /value) and the gap case stayed green. M4 dropped `vp.tileMode ||`: every case stayed green. In tile mode `syncStage` already hides the stage, so `stage.hidden` alone covers the strip. The `tileMode` term is redundant but harmless, and it mirrors `refreshMag`. It is not a finding.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo fmt --all --check` | tree 9bc0ba3, CARGO_TARGET_DIR=wt/target | now | 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | same | now | 0 |
| `cargo test -p ocelli-folder-viewer` | same | now | 0, 73 + 60 + 24 + 14 + four single tests passed |
| `npm run lint`, `npm run typecheck` | same | now | 0, 0 |
| `bin/ocelli.sh gate guards` | same, OCELLI_PORT=0 | now | 0, ALL GREEN, 434 refusal probes, `viewer-browser.readout-survives` `red` |
| `bin/ocelli.sh gate viewer-browser` | release build, Chromium 151.0.7922.34, viewers 55097 and 55151 | now | 0, ALL GREEN. Faulted `hidden readout true cross true magnifier true, 0 POST /value`. Gap `in the gap true, magnifier hidden` |
| `guard_census.py`, `--check-runbook` | same | now | 0, 0, "0 watched by nothing", the readout LIMIT names faulted-cell and tile-gap |
| `prose_check.py`, `source_provenance_check.py` | same | now | 0 ("895 file(s)"), 0 ("1758 files") |
| `target/f208-review8/probe.mjs` on HEAD | release viewer at HEAD, PIDs 56561 and 57518 stopped | now | `probe-head.log`, no page errors |
| HEAD's readout.mjs against b6d2fcca's page | scratch `git archive HEAD` with `git show b6d2fcca:` page.html, own CARGO_TARGET_DIR, PID 60175 stopped | now | exit 1. Only the faulted case ("showed the cross, readout or magnifier", "sent 2 POST /value") and the gap case ("the magnifier showed over the strip") are red. Every other case is green |
| probe.mjs on b6d2fcca's page | same scratch | now | `probe-old.log`. Its diff with `probe-head.log` is the no-picture lines only |
| M1, the gap case without its `m` press, on b6d2fcca's page | same scratch | now | the gap case is green (`magnifier hidden`) on the page that has D2. See S1 |
| M2, M3, M4 page mutations on HEAD's page | same scratch, rebuilt per mutation | now | M2 exit 1 (both tile-mode cases), M3 exit 1 (faulted case), M4 exit 0 |

## Defects

### D1, the readout.mjs header says two more cases and lists four
**Where**: `examples/folder-viewer/tests/browser/readout.mjs:23`, "Two more cases, each on a fresh page:". The list under it now has `empty`, `throw`, `fault` and `gap`.
**What**: the diff added `fault` and `gap` under the existing sentence and did not change the count. All four do run on fresh pages (`observeReadout` opens `faultPage` and `gapPage`), so only the number is false.
**Why it is wrong**: microscope section 3 says a false sentence in prose is a defect. This header is the file's own index of what the check covers.
**Evidence**: `grep -n 'Two more cases' readout.mjs` gives line 23, and lines 25 to 34 list four cases. At b6d2fcca the same sentence listed two.
**Scope and regression**: introduced by 5c8b28f7, in this file only. Fix: "Four more cases". No test is needed.

## Smells

### S1, the tile-gap case passes when the magnifier was never on
**Where**: `readout.mjs` `tileGapCase` (about line 290) and the gap half of `noPictureProblems`.
**What**: the case presses `m`, turns on Four images and moves straight into the strip. It never sees the magnifier shown first, so "hidden" there passes whether the fix worked or the magnifier was simply off. The other cases each have a positive control. The faulted case calls `pointAt(..., ["readout", "magnifier"])` before the change. `faultProblems` refuses "nothing was shown before the change", and `stepProblems` refuses identical images.
**Why it is wrong**: microscope section 2 says a test that passes both ways will be counted as coverage forever. Here is how it can happen: a later change to the `m` binding, to focus after `openSynthetic`, or to tile mode resetting `showMag` turns the case vacuous and it stays green.
**Evidence**: M1. The `m` press was removed from `tileGapCase` in the scratch copy and run against b6d2fcca's page, which has pass 7 D2. The case reported `in the gap true, magnifier hidden` and raised no problem.
**Scope and regression**: test-only, introduced by 5c8b28f7. The smallest fix is to `pointAt` tile 1's picture expecting the magnifier, record `shownBefore`, then move into the strip. `noPictureProblems` should then refuse `!gap.shownBefore` with a "so the case shows nothing" message, as it already does for `inGap`.

## Nitpicks

### N1, faultProblems lost its doc comment
At `readout.mjs` about line 124, the new `noPictureProblems` block was inserted between `faultProblems`' JSDoc ("`empty` is ..., `thrown` is ...") and `faultProblems` itself. Two JSDoc blocks now sit above `noPictureProblems`, and the first names parameters it does not take. Move the `empty`/`thrown` block back down to `faultProblems`.

## Verified clean

- **page.html.** One line changed in `pointReadout`. `tilePoint` still serves a tile canvas under the pointer. The single view maps through `imagePoint` only while `vp.dom.stage.hidden` is false. `syncStage` makes that false only for `!tileMode && imageWidth > 0 && !fault.textContent`, and the main canvas lives inside `.stage`. So "stage shows" is the same condition as "the single-view picture is on screen". With `point` null, the cross, `requestValue` and `updateMag` are all skipped, so no `POST /value` goes out (faulted case 0, M3 2). `lastPointer` is still recorded before the early null, so `refreshMag` after a fault clears has the pointer it needs. The comment above the line matches the observed behaviour.
- **readout.mjs.** The faulted case discriminates (red on b6d2fcca, red under M3) and guards against vacuity through `fault` and its `pointAt` positive control. The gap case discriminates against b6d2fcca, with the limit in S1. The `quarterOf` change keeps both tile-mode cases discriminating (M2). `observe`'s new `where` argument defaults to `centreOf`, so the other three observations are unchanged, and the gate shows the same text as at pass 7.
- **eslint.config.js.** "four of the page's own top-level functions": `imageToClient`, `activeViewport`, `paintSelection`, `dropCache`. `requestGrey` is replaced through `window.requestGrey`. The page is one classic `<script>` (page.html:811) and `requestGrey` is a top-level function declaration, so the page's calls resolve to the replacement. The injected fault appearing confirms it. Lint is 0.
- **catalogue.py and the runbook.** The `refuses` addition describes `noPictureProblems`. The LIMIT text matches the runbook (`--check-runbook` 0) and is true, because no probe drives the faulted or gap branches.
- **README step 8 and use.html / more.html.** "come back with the next pointer move" is about geometry changes over a picture, and it held after every such change in the gate. It says nothing about a fault, so the change leaves it true. use.html and more.html have no readout or magnifier sentence.
- **Pass 1 to 7 items** hold through the gate: five geometry changes hide and a move shows again, drags keep the readout 20 of 20, pan 0 `/value`, ArrowRight and Play equal fresh, the emptied tile hides, and the thrown layout clears.
- No wasm, no `unsafe`, no cast and no pixel arithmetic in the diff.

## Limitations and handoff

- One synthetic CT series, 128 by 96. The colour image was simulated by setting `info.monochrome = false` on the next image, which reaches the real `showFault` route. No real colour file or mixed-size series was available. Failures were injected by wrapping `requestGrey`.
- Device pixel ratio 1 only.
- No oracle, GPU or `--sprint` profile. Ports 8742 and 8765 not used, PID 98891 not touched, nothing killed by name. Every viewer started (55097, 55151, 56561, 57518, 60175 and the M1 to M4 runs) was stopped by its PID through `stopByPid`, and no folder viewer was left running. The scratch copy and its target directory are deleted. The probe and logs are under `target/f208-review8/`.
- Tree unchanged: HEAD `5c8b28f7`, tree `9bc0ba3`, nothing staged or unstaged.
- Next owner: the implementer fixes D1 and S1 (N1 optional), then a fresh independent pass 9 reviews that change. This is a bounded review, not sprint readiness.

F-208 pass 8: 1 defect, 1 smell, 1 nitpick.
