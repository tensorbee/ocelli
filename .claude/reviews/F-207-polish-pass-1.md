# F-207 polish review, pass 1

**Reviewed**: working tree on `sprint/s46`, HEAD `d48c46853660672230d4540e07f8865742e27856`, unstaged paths only.
`examples/folder-viewer/src/page.html` blob `ee4eecb0e711a91d1c6e73c0e6ed1535fffbd7af` (byte-identical to the page served at `http://127.0.0.1:8742/` during the pass).
`docs/sprints/BACKLOG.md` blob `91c41ce675ee25a2d44de40fdf7e6831e2f2452f` and `docs/sprints/allocation.json` blob `d1b503edf9b49c43fc851276c86d69d0b460068f`, both pinned at the start of the pass.
Not reviewed: `more.html`, `use.html`, `serve.rs` (`OCELLI_PORT`), and the later edits to `BACKLOG.md` (now `04d30e6`), `allocation.json` (now `acc0a27`), `CURRENT_SPRINT.md` and `SPRINT_PLAN.md` that schedule F-208 to F-215 into S46. Those changes landed during the pass.
**Author / reviewer**: change authored by another participant. Independent reviewer is a Claude subagent that did not write any of the reviewed material. No files edited except this report.
**Result**: 5 defects, 1 smell, 3 nitpicks

## Acceptance challenged

- **Pixel aspect (claim 3)**. Derived independently from PS3.3 10.7.1.3. Pixel Spacing is row spacing (vertical) then column spacing (horizontal). The synthetic `0.7\0.5` with 128 columns and 96 rows is 64 mm by 67.2 mm, so width over height must be 0.95238. Observed canvas 708.56 by 744 px, ratio 0.952369. Tiles 353.81 by 371.5, ratio 0.95238. This rejects the plausible wrong form (spacing swapped, ratio 1.05 with 128 by 96 at 0.5 by 0.7).
- **Image mapping under rotation and flip (claims 3, 12)**. The page's `clientToImage` and `imageToClient` were checked against the browser's own `ink.getScreenCTM()`, which is independent of the page's arithmetic. Checked at 0°, 90°+flipH and 180°+flipH+flipV. Agreement to 0.005 px forward and 0.0007 image px back. A rectangle dragged 40 screen px right at 90° moved by exactly dy = -40/(scale*aspectY) = -4.916 image px with dx = 0.
- **Measurement (claim 3)**. A length drawn under rot180+flips from image (10,20) to (100,80) is stored at those image points and labelled `61.6 mm`. The independent value is hypot(90*0.5, 60*0.7) = 61.6. The display aspect does not enter `lengthText`.
- **Upright text**. Under rotation, flip and aspect, the screen CTM of the note and of the length label is diag(5.5357, 5.5357) = scale with no shear. Font size 2.71 user units * 5.5357 = 15 px.
- Live W/L, selection through invert, CT-only presets, Escape in the note field, empty-note drop, wiggle and small-rectangle drop, nav-click select and empty-click clear, wheel, ctrl+wheel, modifier keys, 2x2 dim-but-clickable, and the magnifier were all driven in headless Chromium. Results are in Verified clean.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `cargo test -p ocelli-folder-viewer` | working tree, including the out-of-scope `serve.rs` | now | 0, 31 passed |
| `python3 scripts/backlog_check.py`, `python3 scripts/gen_sprint_plan.py --check` | pinned BACKLOG and allocation | now, at pass start | 0, `OK: 238 F-IDs` and `OK: 193 planned F-IDs` |
| same two, re-run | BACKLOG and allocation after the mid-pass scheduling edit | now | 0, `OK: 201 planned F-IDs` |
| `bin/ocelli.sh gate prose`, `bin/ocelli.sh gate backlog` | working tree | now | ALL GREEN. `prose_check.py` scans `.md` only, so it does not cover `page.html` |
| grep for em-dash and prose semicolons in the added doc lines and the page's UI strings | pinned blobs | now | none |
| `node fn.js` extracting `personName`, `dicomDate`, `greyAspect` | pinned page.html | now | see D4 and Verified clean |
| Playwright scripts `b1.js` to `b6.js` under the scratchpad `review/` folder, Chromium headless, DPR 1 | served page = pinned blob, synthetic study, plus a 3-file copy of synthetic slices for the folder path | now | outputs quoted per finding |

## Defects

### D1, a changed or unreadable local file is reported as the viewer process having died
**Where**: `examples/folder-viewer/src/page.html:1626-1631` (`requestGrey` catch), `:1082-1085` (`processGone`), `:773` (`GONE`)
**What**: every rejected `fetch("/preview")` is treated as the process being gone. Chrome also rejects a fetch whose `File` body changed or vanished on disk after it was picked (`ERR_UPLOAD_FILE_CHANGED`). The page then claims the process stopped answering while the process is healthy. The status fault also never clears, because nothing on the preview path removes it again.
**Why it is wrong**: it is a false sentence to the operator, and it sends them to restart a server that is fine. The `/inspect` path classifies the same rejection correctly as "files could not be read" (`:1234-1236`).
**Evidence**: `b3.js` opened a 3-file folder, appended 16 bytes to `s0.dcm`, then pressed `i`. The status became `The viewer process stopped answering. Images already shown stay. Run cargo run -p ocelli-folder-viewer again.` and the view fault read `the viewer process at this address is not answering...`. ArrowDown then showed image 2 normally (`w: 128`) while the status still claimed the process was gone.
**Scope and regression**: new in this change, since HEAD surfaced the raw error. It also fires from `prefetch` and `loadThumbs`, because both reach `requestGrey`. Smallest fix: on rejection, probe the server with a cheap GET (for example `fetch("/synthetic")`). Call `processGone()` only if the probe also rejects, and otherwise throw "this file could not be read again, it may have changed or moved". Clear `status.fault` on the next successful preview. Regression: the same file-change script must leave the status free of "stopped answering".

### D2, the nine tool buttons have no help sentence, in the new tooltip or in the sentence bar
**Where**: `page.html:740-748` (keys `scroll` to `length`) against the lookups at `:1022`, `:1101` and `:1118`, which all use the button id `tool-<name>`
**What**: `SENTENCE["tool-rect"]` and the other eight are `undefined`, so hovering a tool shows nothing and the selected tool's sentence never appears. For example, "It is not an area and it is not saved" for Rectangle and the mm or px rule for Length are never shown.
**Why it is wrong**: claim 2 says the tooltip shows "name + key + help sentence". For the tools it shows only the name and key. The protocol items "no fill or area" and "length in mm with pixel spacing" depend on these sentences being visible.
**Evidence**: `b3.js` in single view gave `tool-rect {"bar":"","tipHelp":""}`, `tool-length {"bar":"","tipHelp":""}` and `invert {"bar":"Reverse the grey...","tipHelp":"Reverse the grey..."}`. With the rect tool selected and nothing hovered, the bar was `""`. In 2x2, only the `liveSentence` override text appears.
**Scope and regression**: the sentence-bar half is already on HEAD (`toolId` and bare keys at HEAD `:459` and `:734`). The tooltip half is new. Smallest fix: rename the nine keys to `"tool-scroll"` to `"tool-length"`. Regression: hover `#tool-rect` and assert that `#sentence` and the `#tip span` are not empty.

### D3, Headers, Fit, Reset and Full screen are clipped off a 1280 px window with no scroll affordance
**Where**: `page.html:178-190` (`.toolbar` `overflow-x: auto` with `scrollbar-width: none` and the hidden webkit scrollbar)
**What**: with both side panels open, the toolbar content is 911 px wide. At a 1280 px viewport the bar is 784 px, so `headers`, `fit`, `reset` and `fullscreen` are off the right edge. At 1400 px `fullscreen` is off. At 1024 px ten buttons are off. There is no visible scrollbar, and a vertical wheel does not scroll the bar.
**Why it is wrong**: the protocol requires a reachable Headers sheet, Fit, Reset and Full screen. At HEAD, 1280 px shows every toolbar button (`b6.js`: `HEAD 1280 {"offscreen":[]}`), so this is a regression on a common laptop width.
**Evidence**: `b5.js` gave `1280 {"client":784,"scroll":911,"offscreen":["headers","fit","reset","fullscreen"]}` and `1400 {"client":904,"scroll":911,"offscreen":["fullscreen"]}`.
**Scope and regression**: keyboard shortcuts and Tab still reach the buttons. Mouse users lose them. Smallest fix: let `.toolbar` use `flex-wrap: wrap` with an auto row height, or narrow `--left` and `--right` below about 1500 px, or keep a visible thin scrollbar. Regression: at 1280 by 800, assert that no toolbar button's right edge exceeds the toolbar's.

### D4, a Person Name with an empty alphabetic group now shows no name at all
**Where**: `page.html:1858-1868` (`personName`)
**What**: the function keeps only `split("=")[0]`. PS3.5 6.2 allows the alphabetic group to be empty while the ideographic or phonetic group is present, for example `=山田^太郎=やまだ^たろう`. Such a name is formatted to `""`, so the patient's name silently disappears from the corner.
**Why it is wrong**: PS3.5 6.2 and 6.2.1 define three component groups, and the first is not mandatory. HEAD printed the raw value, so this is a new loss of displayed patient identity.
**Evidence**: `node fn.js` gave `"=山田^太郎=やまだ^たろう" -> ""`, `"Yamada^Tarou=山田^太郎=やまだ^たろう" -> "Yamada, Tarou"` and `"Adams^John Robert Quincy^^Rev.^B.A. M.Div." -> "Adams, Rev. John Robert Quincy, B.A. M.Div."`.
**Scope and regression**: affects only the top-left corner. Smallest fix: use the first non-empty group, `String(text||"").split("=").map(s=>s.trim()).find(Boolean) || ""`. Regression: a fixture with an ideographic-only name must render non-empty. Unconfirmed: whether `text_or_empty` decodes the non-ASCII Specific Character Set used in such files. Either way the page drops the name.

### D5, tiles (and the single view) switch to smoothed resampling when minified, against the protocol's "pixelated" 2x2
**Where**: `page.html:1775` (`sizeTile`) and `:1798` (`layout`)
**What**: `image-rendering` becomes `auto` when display px per image px times devicePixelRatio is below 1. At DPR 1, a 512² CT in a 1400 px window has tiles about 452 px wide, so the default 2x2 of a real CT is smoothed.
**Why it is wrong**: the acceptance protocol says "2x2 four neighbouring slices pixelated". HEAD was always `pixelated`. Smoothing on minification is a defensible choice, but the change contradicts the acceptance contract without amending it.
**Evidence**: `b2.js` at 620 by 500 gave tiles `61.5px by 64.575px, 'auto'`. Back in single view the canvas was `'auto'` at scale 0.92. At 1400 by 900 the 128-column synthetic stays `pixelated`, so the synthetic study does not exercise this at normal sizes.
**Scope and regression**: this needs an operator decision rather than a code guess. Smallest fix: either pin `pixelated` for tiles (and the single view) or amend the protocol and the `grid-2` and `toggle-mag` sentences to say minified images are smoothed. Regression: a tile whose fit is below 1 asserts whichever rendering is chosen.

## Smells

### S1, the new display geometry from Pixel Spacing has no test, so a row and column swap passes the suite
**Where**: `page.html:1409-1413` (`greyAspect`) and its consumers. The only page tests are substring pins in `examples/folder-viewer/tests/preview.rs:172-214`.
**Mechanism**: `synthetic.rs:4` says the `0.7\0.5` spacing exists "so a swap is visible". No test reads `greyAspect`, `aspectX` or `aspectY` (`grep -rn aspect examples/folder-viewer/tests src/*.rs` finds nothing), so `{ x: facts.row / least, y: facts.column / least }` would pass `cargo test`. The mutation was not applied because this pass may not edit `page.html`, and it follows from the grep. CLAUDE.md requires geometry arithmetic to be checked against the cited section, and here only a human looking at the screen can catch a swap.
**Smallest fix**: a Playwright or node check that opens the synthetic study and asserts the `#view` width/height ratio of 64/67.2 to 1e-3. The `b1.js` first block is a usable start.

## Nitpicks

- N1. `page.html:2933`. The meta/ctrl/alt filter also drops AltGr characters, which arrive as ctrl+alt, so `[` and `]` cannot toggle the panels on German or Nordic layouts. The buttons still work.
- N2. A collapsed panel (`body.no-series` or `no-inspector`) keeps its buttons in the tab order inside a 0 px column. Focus lands on invisible controls. `inert` on the collapsed aside would fix it.
- N3. `loadThumbs` calls `renderSeries()` after each thumbnail and rebuilds every series button. A press that straddles a rebuild loses its click. This was found by reading and was not reproduced.

## Verified clean

- **Claim 1**. HEAD has `.inspector ...` selectors against `<aside id="inspector">` (HEAD `:50-67`, `:371`), `.top` on both the header and `.edge.top` (`:305`, `:354`), and `grid-template-rows: 1fr 1fr` for tiles. All four are fixed. The `[hidden]` override is global with `!important`, and the hidden `#folder` input still opens.
- **Claim 3**. The aspect ratio, mapping, measurement and upright text are covered under Acceptance challenged. Thumbnails use the same aspect. Handles are circular on screen (`rx = 3.5/(scale*aspectX)`). The ink SVG has `preserveAspectRatio="none"`.
- **Claim 5**. The magnifier canvas is 160 device px at DPR 1 with `imageSmoothingEnabled` false. The canvas transform order (translate, rotate, flip, scale) matches the CSS order, and the source is the image-resolution canvas, so the "does not blend them" sentence holds.
- **Claim 6**. During a live W/L drag, one `/preview` was in flight per move. The last request sent was `160/680`, and the overlay read `WW 680  WL 160` during and after release. `overrideWindow` equals the server echo. `liveToken` is bumped in `showIndex`, and the result is discarded when the image changes.
- **Claim 7**. Selection survived `i` (invert): `selected` stayed 2 and the chrome was still drawn.
- **Claim 8**. With `modality: 'MR'` the presets were `[true,true,true]` and the sentence named MR. The synthetic CT left them enabled.
- **Claim 9**. The PN component order is right for PS3.5 6.2 except D4. DA `20261105` becomes `2026-11-05`, and other forms pass through unchanged.
- **Claim 10**. In 2x2 the draw tools are `icon dim` and not disabled. Labels read `Image 1` to `Image 4`, and the corner reads `Images 1–4 / 12`. Clicking Length returned to `gridMode 1` with the stage shown. Hovering a dim tool in the grid gave the tooltip "Drawing works in the single view." (the bar is empty, see D2).
- **Claim 11**. Mark rows are a `div` holding two buttons, and the list read `Length 61.6 mm×`. The `>Remove<`, `>Freeform<`, `>Text<`, `>Rectangle<`, `>Ellipse<` and `>Headers<` pins still hold.
- **Claim 12**. The edit field's top-left sat at (495.8, 297.5) against an anchor of (495.8, 320.5), which is the note size plus 8 above the note.
- **Claim 13**. In fullscreen, Escape in the note field kept the draft visible inside `#wrap`. After `exitFullscreen()`, Enter committed `draft`. This exercised only the page logic. In headless mode the browser does not itself leave fullscreen on Escape.
- **Claim 15**. `Control+=` and `Meta+0` were ignored by the page. Ctrl+wheel zoomed to 111% with `visualViewport.scale` at 1. Wheel 3×30 px moved one image. The context menu is prevented. The two key maps do not overlap.
- **Protocol samples**. The status starts with `Synthetic study, not a patient.`. One CT series of 12. Edge letters R/L/A/P are shown and hidden when turned (code path). An empty new note is dropped. A tiny freeform wiggle and a 3 px rectangle are dropped. Under Escape the note was restored and stayed selected. A pan click on a note selected it, and an empty click cleared the selection. No page errors appeared in any run.
- **Docs (pinned blobs)**. Eight rows F-208 to F-215 with E38.18 to E38.25. Every `depends_eids` maps to the `depends_fids` given (E38.1 F-191, E38.5 F-195, E38.6 F-196, E38.9 F-199, E38.10 F-200, E38.11 F-201, E38.16 F-206, E38.17 F-207, E38.18 F-208, E38.20 F-210). These note claims were checked against code: "The pointer cross shows column and row today", "Number keys currently choose the layout", "The three CT presets are offered on CT only", and "Projection images ... show no edge letters today" (`orientation_letters` reads only 0020,0037 and 0020,9116). No em-dash and no prose semicolon. Both checks are green.

## Limitations and handoff

- Only Chromium headless at DPR 1 was exercised. DPR 2, Firefox, Safari, touch and pinch, and real fullscreen Escape handling were not exercised.
- No non-CT or non-square real corpus file was opened. The non-CT preset path was driven through `syncPresetAvailability` with a synthetic `info`.
- More.html, use.html, serve.rs and the S46 scheduling edits to the docs are not covered by this pass. The current docs also pass both checks.
- Next: the author remediates D1 to D5 and S1, and the operator decides D5. A fresh independent pass must then review the changed material and replay the D1 file-change script and the D3 width sweep.
