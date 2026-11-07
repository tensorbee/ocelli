# F-208 review, pass 2

**Reviewed**: `git diff 8b8bcada 919e61ab` on `work/f-208-xai` in `ocelli-wt-f-208`, HEAD `919e61ab443fe194d73fbdd861234949c3b00e99`, tree `dd5a3944c20b7c03bc8273c0dcfd0b1c7483d2df`. Nothing unstaged before or after the pass. The only untracked paths are `.claude/reviews/F-208-working-pass-1.md`, present before this pass, and this report.
**Author / reviewer**: the remediation commit is by a Claude implementer agent. This reviewer is a separate Claude agent (`/microscope F-208 --working`, pass 2). It wrote none of the diff and did not write pass 1.
**Result**: 1 defect, 1 smell, 1 nitpick

## Acceptance challenged

- **Pass 1 D1, readout erased by another cell. Closed.** At 2 by 1, cell 0 active, cross on, pointer resting on cell 1: `C 65  R 49  40 HU` before, the same after ArrowDown in cell 0 (cell 0 at Image 2) and after 1.5 s of Play in cell 0. When cell 1 steps itself the readout does reset: a 40 px wheel over cell 1 gives `C 65  R 49` with the pointer still, a move restores `40 HU`, and ArrowDown with cell 1 active gives `C 65  R 49` again. That is F-210's rule kept per cell.
- **Pass 1 D2, a click on the open row. Closed, matching the lead ruling.** 1 by 1: Lung and `=` read `WW 1500  WL -600 / Zoom 111%`, preset `lung`. A row click reads `WW 400  WL 40 / Zoom 100%`, `activePreset` null, `fitMode` true. 2 by 2 with cell 0 active, the same pair. Cell 1 with Lung, zoom and two image steps, then a drop of the series it already shows: `WW 1500  WL -600 / Zoom 111%` kept. A row click while cell 1 is active then resets it to `WW 400  WL 40 / Zoom 100%`. A **different-series drop**, replayed on a real two-series folder (a scratch copy of `corpus/data/synthetic/ct_series_uniform` and `ct_series_nonuniform` under `target/`, deleted after): cell 1 with Lung and zoom, then the row of cell 0's series dropped on it, reads `WW 400  WL 40 / Zoom 100%`, its key equals cell 0's and `fitMode` is true.
- **Row click in tile mode.** Now that `loadSeries` no longer returns early, a click on the open row runs the token, abort and reset path in a tile-mode cell. Cell 0 in tile mode at 2 by 1, then a row click: `tileMode` stays true, all four tile canvases are drawn, the window resets. The abort controllers are replaced at their next use (`page.html:2130`, `2283`), so the abort does not poison later requests.
- **The readout's cell when the layout shrinks.** Challenged as asked, and it fails (D1 below).
- **S1 pins, every form a digit could be bound in.** Read `onKey` (`page.html:4038`) and the two maps. The maps are looked up by `event.key.toLowerCase()`, and the branches compare `event.key`. Fourteen mutations, results under the smell.
- **N1 pin.** Removing `if (!carriesSeries(event)) return;` from `onCellDragOver` goes red with "a cell accepts a drag without the series type", and so does `carriesSeries` accepting any typed drag. N1 is closed.
- **`viewer-browser.aspect-no-canvas`.** The probe script exits 0 on the unmutated `aspect.mjs`. With the probe's own substitution (`if (false) {`) it exits 1 with `aspect check accepted a run with no canvas`. Changing only the message `no canvas was measured` also turns it red, so it keys on that branch's text. Inside `gate guards` the probe reads `red` as expected.
- **Pass 1's verified-clean items.** Pass 1's `replay.mjs` and `replay2.mjs` re-run on this tree give logs identical to pass 1's (ports aside): B1 to B5, digits change neither layout nor tile mode, `g` opens the picker, Escape closes it, Shift G per cell, Shift G refused under Length, foreign drag refused, the 32 MiB rule, the abort-share case (cell 1 `Image 7`, cell 0 `Image 10`, 0 previews in the idle second), no paint into hidden cell 1, regrow keeps view state. Aspect is in the gate row below. The diff touches none of `pumpMain`, tiles, prefetch or `scheduleLive`.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo fmt --all --check` | tree dd5a394, CARGO_TARGET_DIR=wt/target | now | 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | same | now | 0 |
| `cargo test -p ocelli-folder-viewer` | same | now | 0, every suite ok (73, 1, 14, 60, 1, 1, 24) |
| `npm run lint`, `npm run typecheck` | same | now | 0, 0 |
| `bin/ocelli.sh gate guards` | same | now | ALL GREEN, `viewer-browser.aspect-swapped` and `viewer-browser.aspect-no-canvas` both `red` |
| `bin/ocelli.sh gate viewer-browser` | release, OCELLI_PORT=0, Chromium 151.0.7922.34 | now | ALL GREEN. Single view 0.952365, tiles 0.952346, 2 by 2 cells 0.952363, all 9 within 0.001 of 0.952381 |
| `guard_census.py`, `--check-runbook` | same | now | 0, 0. "0 watched by nothing". Runbook rows 579 to 582 agree |
| `prose_check.py`, `source_provenance_check.py` | same | now | 0 ("895 file(s)"), 0 ("1757 files") |
| `target/f208-review2/replay.mjs` (D1, D2, shrink, different-series drop) | release viewer, stopped by PID | now | `replay.log`, no page errors |
| `target/f208-review2/replay-shrink.mjs` (keyboard-only shrink, held `/value`) | same | now | `replay-shrink.log`, `shrink-keyboard.png` |
| `target/f208-review2/replay-tile.mjs` | same | now | `replay-tile.log` |
| `target/f208-review2/p1/replay.mjs`, `replay2.mjs` | pass 1's scripts, paths adjusted | now | logs identical to pass 1's |
| Page-test mutations M1 to M14, N1a, N1b | `git archive HEAD` scratch copy in the scratchpad, own target dir, deleted after | now | table under S1, restored copy green |
| Aspect probe script against mutated `aspect.mjs` | same scratch copy | now | 0 unmutated, 1 mutated, 1 message changed |
| `target/f208/replay2/log.txt` | implementer's | prior receipt | read, agrees with the replays above |

## Defects

### D1, a layout shrink leaves the hidden cell's value in the readout, and a late reply paints it
**Where**: `examples/folder-viewer/src/page.html:4375` (`setLayout`, which never looks at `valueState` or the cross) with `sendValue`'s acceptance at `page.html:3153`
**What**: when the cell the readout belongs to is hidden by a smaller layout, the cross, its readout text and `valueState.cell` stay as they were. A `/value` reply in flight for that cell is still accepted, because its generation, column and row all still match, and it is painted over the cell that now fills that area. Replay through the real caller, keyboard only and the pointer still: 2 by 1, cross on, pointer on cell 1, `/value` held 1.5 s, then `G`, Shift Tab to "1 by 1", Enter. Just after the shrink the readout is `C 56  R 49` with `valueState.cell` 1 and one cell shown. Once the reply lands it is `C 56  R 49  700 HU`. One pointer move later the same spot reads `C 103  R 49  40 HU`, which is the image actually shown there. `shrink-keyboard.png` shows `700 HU` printed beside the grey ring of the single viewport, and the horizontal cross line still spans only cell 1's old box. Without a held reply, the old cell's full readout (`C 65  R 49  40 HU`) simply stays on screen after the shrink.
**Why it is wrong**: F-210's comment on `valueState` says a reply is shown only for the cell and the image it was asked for, and the remediation added `cell` so that holds per viewport. Here the cell is no longer shown and its value is drawn over another image. A modality value printed at a point whose value it is not is a quietly wrong value, which is CLAUDE.md's dangerous class. The plan's invariant for a hidden cell is "its tokens move on so no late frame paints into it" (`setLayout`'s own comment), and the readout is the one per-cell output that does not follow it.
**Evidence**: `target/f208-review2/replay-shrink.log` and `replay.log` ("S" lines, the same through `setLayout(1, 1)` directly).
**Scope and regression**: present on HEAD and already present at `8b8bcada`. The remediation did not introduce it, and pass 1 did not exercise it. A mouse click on the picker hides the cross first through `wrap`'s `pointerleave`, so the trigger is the keyboard picker path or any other non-pointer layout change. The smallest fix is in `setLayout`: when `valueState.cell` is not below the new count, call `resetValue()` (which moves the generation, so the late reply is dropped) and `hideCross()`. The magnifier follows the same pointer and was not checked after a shrink (unconfirmed). The regression is the keyboard replay above, asserting the readout is hidden or carries no value after the shrink and after the held reply lands.

## Smells

### S1 (carried from pass 1, narrowed), the digit pins catch the canonical spellings only
**Where**: `examples/folder-viewer/tests/preview.rs:417-455`
**What**: pass 1's mutation now goes red, and so do several neighbours. Other spellings of the same binding stay green, while the test's comments claim the general property: "No bare digit may be a key in either" and "No branch of `onKey` tests a digit from 1 to 9".

| Mutation in the scratch copy | Result |
|------------------------------|--------|
| M1 `clicks`: `"1": "layout",` | red |
| M2 `clicks`: ` 1: "layout",` | red |
| M3 `clicks`: `'1': "layout",` | red |
| M4 `clicks`: `{1: "layout",` | **green** |
| M5 `clicks`: `"invert",1: "layout",` | **green** |
| M6 `map`: `"2": "scroll",` | red |
| M7 `clicks`: `"1" : "layout",` | **green** |
| M8 `onKey`: `event.key === "1"` | red |
| M9 `onKey`: `event.key === '1'` | **green** |
| M10 `onKey`: `"1" === event.key` | **green** |
| M11 `onKey`: `/^[1-9]$/.test(event.key)` | **green** |
| M12 `onKey`: `event.code === "Digit1"` | **green** |
| M13 `onKey`: `event.key == "1"` | **green** |
| M14 `clicks`: `"0": "layout",` | red |

**Mechanism**: the next story to edit this handler is F-213, which binds `1` to `9` to window rows, and the natural way to bind a range is M11 or a lookup on `event.code` (M12), not nine `===` branches. The pin would stay green through that change, so F-213 is not forced to revisit it, and the tree would carry a test whose comment says no digit is bound while digits are bound. That is the same failure as pass 1's S1, a check counted as "digits are free" that does not observe the form the binding will take. **Smallest fix**: in the `onKey` body, refuse any string or regex literal that contains a digit from 1 to 9, and any `Digit` token. Today the only digits in that body are `"0"` and `0.1` in `Math.exp`, so this does not misfire. For the maps, strip whitespace and quotes from the literal before looking for `{N:` or `,N:`. Or narrow the comments to the spellings actually checked. The plan's AC13 itself only asks for the `grid-1`/`grid-2` strings, so this is a smell and not an AC failure.

## Nitpicks

- **N1.** `more.html:119`, the rewritten lede says "up to nine viewports, each with one slice of one series". A viewport in tile mode shows four slices and an empty one shows none. The base lede was already loose about the four-image view, and the `#series` paragraph at line 234 states the specifics correctly, so this does not block. "each showing one series" would be exact.

## Verified clean

- Pass 1 D1, D2 and D3 are closed. D3: `grep "The image page shows one slice"` finds nothing, and the new lede's "up to nine viewports" holds (picker reaches 3 by 3, replayed in pass 1 and rechecked in the picker DOM).
- README step 4's new sentence, executed: "A click on the series already open there starts its window and fit again" (`WW 400  WL 40 / Zoom 100%`, preset cleared, at 1 by 1 and 2 by 2) and "a drop of that series keeps its view" (`WW 1500  WL -600 / Zoom 111%` kept, `onCellDrop` returns before `loadSeries`, so the image index is kept too).
- The `loadSeries` comment matches the code. `onCellDrop` checks `cellShown` and the key before the same-series return, and still makes the cell active.
- `valueState.cell` is never cleared by `resetValue`, so after a reset in a cell's own `showIndex` the readout stays tied to that cell, and `requestValue` resets when the pointer enters another cell (`C 103 ... cell 0` after a move).
- The catalogue's new probe, its runbook row 579 and the renumbered 580 to 582 agree with `--check-runbook`. The limit text now names both probes, which closes pass 1's N3.
- Boundary and tier: the diff adds no wasm, no `unsafe`, no cast and no pixel arithmetic.

## Limitations and handoff

- The magnifier after a shrink was not checked. Only one CT modality was exercised. The `shown` row class with three or more series was read, not replayed.
- No oracle, GPU or `--sprint` profile was run, as instructed. Ports 8742 and 8765 were not used and PID 98891 was not touched. Every viewer I started was stopped by its PID (17755, 18480, 24873, 25969, 26745, and 26434 by the gate), and none is still running. The scratch copy, its target dir and the two-series study copy were deleted. Replay scripts and logs stay under `target/f208-review2/`, which is ignored.
- Next owner: the implementer fixes D1 and S1, then a fresh independent pass 3 reviews.

F-208 pass 2: 1 defect, 1 smell, 1 nitpick.
