# F-202 design review, pass 5

**Reviewed**: design round 5 of .claude/plans/F-202-design.md on sprint/s47 8949c801.
**Author / reviewer**: GLM through opencode. Review by Claude, a read-only subagent, composed by the Claude coordinator for S47.
**Result**: 6 defects, 12 smells, 10 nitpicks

The full findings follow as the reviewer wrote them. The lead amendment after design round 5 in the plan applies each fix exactly as stated here.

F-202 design review, round 5. Plan: /private/tmp/ocelli-s47-f202-design/.claude/plans/F-202-design.md (working copy, 2620 lines), base sprint/s47 8949c801. This was a read-only review. The tree is unchanged apart from the plan's existing modification, and the scratch directory has been removed by its exact path. I did not open F-273-design.md, corpus/data, dwv, Horos or Grok.

**Verdict:** not approvable yet. It can be approved after a coordinator lead amendment. Every fix below is a text change and none needs the operator. The blockers are D1 to D6. The smells and nitpicks are cheap corrections that can go in the same amendment.

## 1. Round 4 rulings, one by one

- **D1, AC16: resolved.** I recomputed it in emulated f32 (`Math.fround`) from auto_window.rs:273, :410-411, :537-550 and :623-653, from lut.rs `apply_window` LINEAR_EXACT and from colour.rs `reference_ramp_index` / `round_half_even`.
  - Correct case: low 1, high 99, moved to 1.0000019073486328 and 98.9998779296875. Width 97.9998779296875, centre 49.99993896484375, accepted at search attempt 0. Displays 0, 39.0307, 124.8981, 210.7656, 252.3983. Bytes 0, 39, 125, 211, 253.
  - Zero-counting mutant: low 0, high 98, low moved to a subnormal. Width 97.9998779296875, centre 48.99993896484375. Bytes 0, 42, 128, 213, 255.
  - All values match the plan (L246-257, L760-767, L1755). The `KIND_SET_WINDOW` path parses f64 and converts with `finite_f32` (commands.rs:507-516), so the record's f32 values survive the command exactly.
- **D2, parse_count: partly resolved.** The helper increment and the three-call fixture are present (L1171-1178, L2355). The stray `1.` marker is still there, at L1170-1171 ("`window_source`" then a new line "1. It then writes"). Under CommonMark an ordered list starting at 1 interrupts a paragraph, so it still renders as a list (see N1).
- **D3, AC12: the transfer set matches the ruling** (L264-269, L1751). The observation cannot see the worker-to-page direction (D5 below).
- **D4, early returns: applied** (L270-281, L1391-1399). The mechanism is incomplete (D2 and D3 below). Stale "painter gates" wording remains at L1811 and L1957-1958.
- **D5, routing table: present** (L1343-1359) but incomplete (D1 below).
- **D6, wrapper: resolved** (L1318-1325, L1773). Its stacking against `#cross-*` and `#mag` is unspecified (D2).
- **D7, eslint: resolved** (L1827-1862). Verified against eslint.config.js: block at :245-264, `assertTheBanIsIntact` at :150-203, allowance at :282, browser-suite block at :533. There is no observation that the typed rule actually fires on `.js` files (S12).
- **D8, census record: mostly resolved.** It is in the write set as item 28, and AC27 cites it. AC11 does not cite it as ruled, and the new pins refusals are not enumerated (S6).
- **D9, gzip rule: the ceiling rule is resolved** (TOLERANCE at :241 confirmed). The definition of "stale" contradicts itself (S6).
- **D10, the one-to-one seam: mostly resolved.**
  - Confirmed: `fitView` at :3865, `blankViewport` at :1362, `remoteRelease` at :1467, `#remote-state` at :753, `section#remote` at :751, Show local, AC35.
  - `remoteRelease` has three callers, not two (N3).
  - The Fit dispatch and the activation's `fitView` call can collide (S1).
- **S1, existing per-frame resolvers: resolved.** It recommends against reuse and adds an agreement test. Citations verified: frame_open.rs:365, :636, :702, :740-800, :802-818, :1030, :1087.
- **S2, trap probe builds in CI: resolved for CI.** The local gate arm still does not build them (S7).
- **S3, armed page CSP: resolved.** Verified csp.rs:35 and serve.rs:298-299.
- **S4, AC31 visibility: text added, but it conflicts with the D4 mechanism** (D3).
- **Smell 5 and smell 6, and the seven nitpicks:** applied, except that the cornerstone sentences are still marked as carried (N6).

## 2. Citations executed on 8949c801 (about 140, all holding except where noted)

- **page.html:** 751, 753, 763-765, 781-782, 819, 820, 823-824, 827, 833, 834-842, 843, 855-858, 867, 1362, 1407, 1428, 1467, 1493, 1592, 1638, 1639, 1641, 1647, 1654, 1671-1672, 1750, 1753, 2623, 2639, 2776, 3492, 3589, 3647, 3650, 3662, 3728, 3732, 3748, 3752, 3865, 3871, 4035-4037, 4044, 4243, 4265-4271, 4283, 4294, 4298, 4361, 4700, 4705, 4721, 4738, 4856, 5129, 5142, 6036-6042, 6050-6055, 6059-6061, 6075, 6103, 6969, 6983.
- **Tooling and config:** eslint.config.js 150-203, 245-264, 282, 533. scripts/guard_census.py `--record` and `--render-runbook` exist (lines 19, 212, 214). ci/guard-probe-budget.json exists and pins `DECODE_EXPORTS`, `DECODE_ABSENT` and `TOLERANCE`. pin_and_size_check.py :93-98, :241, and `check_decode_size`. decode_graph_check.py :32, :68-69, :74-77, :220-234. catalogue.py :10157, :10172, :10255. bin/ocelli.sh :109, :110, :161-167, :219-237, :296-301, :650. ci.yml :409-427. .gitignore:30.
- **Rust:** session.rs 23, 158-160, 227, 238, 472, 681, 689, 702, 806, 1380, 1385, 1417, 1546. lib.rs 47-48, 70, 128. canvas.rs 634, 666, 718. Cargo.toml 102, 105. dicomweb.rs 204, 221-223. parse.rs 154. multiframe.rs 234. preview.rs 74, 145, 155, 1191, 1228-1242, 1287, 1291, 1693, 2131-2287, 3851-3854. value.rs 25, 190. lut.rs 305, 503, 572, 596. colour.rs 424-430, 446-481, 518. cpu.rs 457, 591, 966, 1169. commands.rs 135, 427, 507-516. boundary.rs 128.
- **TypeScript and scripts:** bulk.ts 343-369, 401-410, 424, 546, 592. dicomweb-viewer.ts 372, 375, 384, 418, 1090, 1194, 1233, 1312, 1320, 1325, 1331. load.ts 19, panic.ts 133, panic_probe.mjs 130 and 172, csp-probe.mjs 723-729, dicomweb.mjs 515 and 573.
- **Cornerstone 5.8.2**, read at the main checkout's tools/oracle/node_modules/@cornerstonejs/: init.js `maxWebWorkers || getReasonableWorkerCount()` = `max(1, floor(hardwareConcurrency/2))`, and `VOILUTFunctionType` is LINEAR, SAMPLED_SIGMOID="SIGMOID", LINEAR_EXACT. Both facts hold.
- **Wrong:** preview.rs:713-721 (N5). The remoteRelease caller count (N3).

## Defects

**D1. The routing table misses inputs that step, window or reload the remote viewport (L1343-1372, L1385-1389).**
- Only `applyGrey`, `scheduleLive` and `paintTiles` return early. `showIndex` and `loadSeries` still run and mutate the remote viewport's local state.
- Release then repaints the labels but redraws no pixels (L1387-1389: "re-runs no showIndex").
- Inputs the table misses:
  - Series list click and dblclick, `loadSeries` at :2377-2378.
  - Cell drop at :6648, and `fillCells` / `startViewports` at :3844 and :3855.
  - The stack-scroll drag tool, `tool-scroll` / key `s` (:762), whose mode `scroll` calls `showIndex` at :4477-4481.
  - Window list rows of kind `file` and `auto`, and number keys 1 to 9, through `chooseRow` (:4031-4038, :6056-6058). These rows are built from the local image by `windowRows(currentImage(vp))`.
  - The frame-window refresh re-show at :4115-4117.
  - Draw and measure tools, region window `y`, and key image `k`. These go through `onPointerDown` (:4361) for modes other than wl, pan and zoom.
  - Link fan-out from another viewport into the remote one: `copyWindow` at :6996-7017, `copyZoom` at :7022-7036, link scroll at :6950.
  - Shift+R calls `resetView` directly (:6066-6067). Only `#reset` is listed.
  - Tile mode: Shift+G and the layout picker `g` (:6068-6073), then `setTileMode` calls `showIndex` (:2601) and the layout loop runs (:6699).
  - The changed-file re-show from `probeActive` (:3091-3094).
  - The `setActive` draft cancel (:6750).
- **Failure scenario:** a remote frame is shown in cell 0. The user clicks series B in the list. `loadSeries` sets `vp.seriesKey` to B and fit mode, `showIndex` runs, and `applyGrey` returns early. The user presses Show local. `remoteRelease` repaints `commitLabels`, `paintOverlays` and `paintEdges` for series B over series A's last pixels. That is a quietly wrong label.
- **Second scenario:** a length drawn over the remote frame becomes a mark on the local image, at local geometry.
- **Fix:**
  - Add every input above to the table, either dispatching or refusing.
  - Put the field check in `loadSeries`, `showIndex`, `chooseRow`/`chooseWindow`, `resetView` and the drag modes in `onPointerMove` and `onPointerDown`. In the pointer handlers the check must come before the `!currentImage(vp) || vp.tileMode` bail.
  - Make `copyWindow`, `copyZoom` and the link scroll skip a viewport whose field is true.
  - Have `remoteRelease` redraw with `showIndex(vp, vp.viewIndex)` from the cache. That is not a re-open.

**D2. Hover readout, crosshair and magnifier are not gated (L1394-1396, L1418-1431, L1770).**
- `pointReadout` (:4700) runs from `onPointerMove` (:4424) for every move. It resolves the viewport geometrically with `viewportAt` (:6567). `vp.dom.stage` is not hidden while the remote frame shows, so it maps the point through local geometry.
- It then calls `requestValue` and `paintValue`, which write the local pixel value into `#cross-read` (:4779-4781), and it draws the local magnifier.
- `pointReadout` is not one of the D4 early-return entries. The claim at L1394-1396, "no local-derived label can be produced... whatever the corners, cross and magnifier toggles say", is false.
- `#cross-*` and `#mag` are children of `#wrap` at z-index 2 and 3, not of `.cell`. The wrapper's z-index is never stated relative to them.
- **Failure scenario:** with the cross toggled on, hovering the remote CT frame shows the local image's pixel value at that position. AC31's hover case is red on a plan-correct implementation.
- **Fix:** make `pointReadout` (and `updateMag`) return early, after `hideCross()` and `mag.hidden = true`, for a viewport whose field is true. Name the wrapper's z-index as above 3.

**D3. AC31's assertion contradicts the D4 mechanism (L1770 against L963-965 and L1391-1399).**
- D4 hides nothing. The plan relies on the opaque wrapper covering the in-cell elements ("the opaque wrapper covers whatever they last drew").
- Some writers bypass the entries entirely:
  - The `toggle-corners` handler calls `paintOverlays` directly (:1670-1673). It unhides the corners and writes local text.
  - The activation's `fitView` calls `layoutViewport`, which calls `paintOverlays` (:3625).
- AC31 asserts `display: none` from `getComputedStyle` on the eleven elements. A correct implementation fails that. Hit-testing cannot fix it either: `.overlay`, `.edge`, `.scale`, `.fault`, `.cell-empty`, `#cross-*` and `#mag` are all `pointer-events: none` (:250, :323-394), so `elementFromPoint` and `elementsFromPoint` skip them.
- **Fix:** for the in-cell elements, assert occlusion. Take a page screenshot of each element's box and compare it to the expected remote canvas bytes, and assert the wrapper's computed z-index is above each sibling's. Keep computed `display: none` for `#cross-*` and `#mag` after D2's gate. Apply the same to AC34's "visible" wording.

**D4. Canvas respawn is infeasible as written (L951, L1416, L1673, L1701, AC25, AC26).**
- The plan says the canvas is "created once", describes "One OffscreenCanvas owned for the page lifetime", and says the canvas is "rebound" after a trap.
- `transferControlToOffscreen` is one-shot (the plan says so itself at L957). The placeholder canvas of a terminated worker cannot be transferred again, and the call throws InvalidStateError.
- **Failure scenario:** AC25's and AC26's "next Display respawns both workers" throws, and no frame draws.
- **Fix:** every render-worker spawn replaces the wrapper's canvas element with a fresh one and transfers that. AC12's count of one canvas per spawn already allows this. Reword L951, L1673 and AC26's "rebound".

**D5. AC12 cannot reject its named wrong implementations (L1751).**
- Wrapping `Worker.prototype.postMessage` and `MessagePort.prototype.postMessage` in the page init script covers page-to-worker only. Init scripts do not run in dedicated workers, and the worker's `self.postMessage` is unwrapped. "Records every transfer list in both directions" is not achievable this way.
- **Failure scenario:** a facts record posted to the page as an ArrayBuffer passes.
- Per-event command posting is only rejected if the case drives several input events inside one animation frame.
- **Fix:**
  - Add a page-side `message` listener on each Worker that deep-walks `event.data` for ArrayBuffer, ArrayBufferView, MessagePort and ImageBitmap, and asserts none.
  - Add a driver that sends five wheel or pointer events synchronously inside one frame and asserts exactly one command buffer, present in the transfer list.

**D6. The F-241 ordering claim is false (L2140-2154, L2148-2149, against L839-847).**
- decode_graph_check.py:74-77 is an exact-set pin (`set(render) != needed`). csp-policy.test.ts:211-259 refuses any `ocelli-codec` edge under `--all-features`.
- F-241's approved plan names neither file in its write set (grep of F-241-design.md finds neither). Yet its AC21 (L261) requires `gate wasm` to be green on its tip.
- **Failure scenario:** F-241's own landing turns `gate wasm` and `npm run test` red. Its implementer then improvises a guard edit. Meanwhile F-202 states "its AC21 needs no amendment" and "No guard refuses ocelli-codec in the render module at any point in the sequence".
- **Fix:** the coordinator amends F-241 to own the render pin edit with its probe and census re-record, and the csp-policy codec walk (narrowed or replaced, citing F-202). F-202's L839-847 and L2140-2154 are restated to match.

## Smells

- **S1, Fit routing against the activation drop (L1330-1335, L1350).** If the field check sits inside `fitView`, then the activation's own `fitView` call, made after the field is set, dispatches `KIND_FIT` and never runs F-204's fit path. AC35's held 1:1 survives. State the order: call `fitView` before setting the field, or check at the handlers rather than inside `fitView`.
- **S2, the Reset row (L1355) re-sends the window only.** Local `resetView` (:3871-3887) also restores invert, rotation, flips, pan and fit. After Reset, a remote frame stays rotated, flipped, zoomed or inverted. Dispatch `KIND_FIT`, `KIND_SET_INVERT` with the record's `invert` (not false, because of MONOCHROME1), and the rotation and flip inverses (`KIND_ROTATE` is relative).
- **S3, the W/L drag origin (L1352).**
  - `onPointerDown` takes its origin from `dragWindow(vp)` and checks the local `vp.shownWindow` (:4375-4376). A remote drag would jump from the local window. Name the remote setter's latest choice as the origin.
  - The pointer handlers bail on `!currentImage(vp)`, so in AC34's empty-cell case nothing dispatches.
  - Presets come from `windowRows` over the local image.
- **S4, AC24 and the Rust test cannot reject a hard-coded `parse_count = 1`.** Add a positive control that drives the helper twice and reads 2, as AC14 does for its counter.
- **S5, AC14's "counted wrapper" (L1236-1238, L1753).** It must patch `navigator.gpu.requestAdapter`, and `OffscreenCanvas.prototype.getContext` for webgl2 and webgpu, before the glue is imported. A JS wrapper function does not count wasm-initiated web-sys calls, so the `resolvedTier` wrong implementation it names would pass.
- **S6, AC11 (L1750).**
  - "Stale" is defined as recorded bytes not equal to the built size, but the text then says the accept path refuses the opposite case. Read literally, accept refuses exactly the case it exists for. Replace with: "gate wasm applies the two ceilings only. Accept refuses a no-op, where built bytes equal recorded bytes."
  - AC11 does not cite the census record or the pins probes, as the D8 ruling requires.
  - The new pins refusals are not enumerated: gzip over its ceiling, a missing gzip field, a no-op accept, and a missing new decode export.
- **S7, local builds.** The `viewer-browser` gate arm (bin/ocelli.sh:219-237) builds neither pkg, pkg-decode, packages/core/dist nor the two trap probes. Only CI yaml does. A local `gate --sprint` fails by name. Put the builds in the arm on the panic arm's pattern and have CI call the arm.
- **S8, documents and tests left stale.**
  - more.html:177, the F-245 row, says "remote image display is not available yet" and is asserted by dicomweb.mjs:769.
  - README.md:325 says "Remote images are not displayed yet".
  - Neither AC9's named edit nor the documentation impact section names them.
- **S9, F-241's folder viewer page worker.** F-241-design.md L196, L206 and L280 add a page worker to the folder viewer, and it lands first. Scope items 3 and 4 (wasm routes and `worker_policy()`) should state that they extend F-241's serving and policy rather than add a second copy. The overlaps section is silent on this.
- **S10, AC35's no-re-open check.** Comparing series structures does not catch a `loadSeries` or `showIndex` re-run. Assert that `loadToken` is unchanged and that no new `/preview` request appears.
- **S11, `#remote` forced visible (L1340, L1774).** The `source-dicomweb` toggle (:1597-1601) can still hide the panel. Name it as refused or ignored while the field is true.
- **S12, eslint coverage is unobserved.** No AC proves the typed rule fires on the three `.js` files under `allowDefaultProject`. Add a mutation: a cached `wasm.memory.buffer` view in a worker should turn `npm run lint` red.

## Nitpicks

- **N1.** The stray `1.` marker is still at L1170-1171 (round-4 D2, unapplied).
- **N2.** Prose semicolons at L1726, L1751 (two) and L1755.
- **N3.** `remoteRelease` has three callers, :1407, :1428 and :2154 (`openSynthetic`). L1385 says two.
- **N4.** Stale wording: "painter gates" at L1811 and L1957-1958, and "two catalogue probes" at L2293 (there are four).
- **N5.** preview.rs:713-721 is `HeldFrame::channels`. `FileWindow` is at :769 and its Auto arm at :1653 (L752).
- **N6.** The cornerstone dist is readable at the main checkout and both facts verify (init.js :8-22). L32-35, L851-856 and AC13 still say "carried, not re-verified", and L541-542 says "read this session", which contradicts them.
- **N7.** The `decodeInstanceMetadata` identity check is cited as :418-424 (L88, L1742) and as :418-421 (L664). Line 424 is `rawBodySha256`, so :421 is right.
- **N8.** The open option (L2516) says the record supplies modality. The 76-byte layout has no modality field.
- **N9.** The status sentence set is called "seven" (L2556). It is now nine, plus Scope item 8's refusal lines, which are never written out. AC13's gap list omits the cine and 1:1 refusals.
- **N10.** series_open.rs:1103 `resolved_text` reads the Frame Content Sequence stack id with a VR check. It is not a presentation resolver, so "a third private set" (L1597) overstates it. Also, value.rs:742 is the `#[test]` attribute and the function is at :743.

## 4. Tiers and deviation drafts

- All three tier rows are answered: CPU raster on every tier, no device opened. Confirmed that the CPU raster honours `View` (cpu.rs:15, :994), so the zoom, rotate and flip dispatches are real.
- D-34 is correct.
- D-35 quotes the F-110 guard at :68-69, as ruled.
- `deviation_check.py` refuses D-34 and D-35 until the rows are written. That is the expected state for a draft.

## 5. The open option (corners filled from the remote record)

Defer it, with a follow-up F-ID filed at approval.
- The record has no patient, orientation, exposure or modality fields, so filling the corners needs a record extension and a second corner producer.
- AC13 states the gap, and the opaque wrapper covers the corners meanwhile.
- The cost of deferring is no numeric window feedback during a remote W/L drag. That is acceptable under Partial.
- AC31 and AC13 are unaffected.

## 6. Readiness

Approvable after one coordinator lead amendment that rules D1 to D6 exactly as fixed above. The smells and nitpicks can ride the same amendment as text corrections. No operator question is needed. Blockers are D1 to D6. Everything else is polish.

RESULT: 6 defects, 12 smells, 10 nitpicks