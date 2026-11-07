# F-213 review, working pass 1

**Reviewed**: `work/f-213-xai`, range `a2b3098b..76e961be`, HEAD `76e961be09b4234779f413541025d4d516904b3d`. No unstaged tracked paths when the pass began. Untracked `.venv` and `tools/oracle/out` exist and are outside the diff.
**Author / reviewer**: a Claude implementer wrote the range. This pass is an independent Claude reviewer, launched by the sprint lead. It did not write or edit any of the reviewed code.
**Result**: 2 defects, 0 smells, 4 nitpicks

## Acceptance challenged

- AC1, AC2, AC3, the index selection. I worked out the greys by hand from PS3.3 C.11.2.1.2.1 LINEAR (`c' = c - 0.5`, `w' = w - 1`). Index 0 of `40\-600` and `400\1500` at stored 50 gives `((50 - 39.5) / 399 + 0.5) * 255 = 134.21`, so grey 134. Index 1 gives `((50 + 600.5) / 1499 + 0.5) * 255 = 238.16`, so grey 238. LINEAR_EXACT at index 1 gives `((50 + 600) / 1500 + 0.5) * 255 = 237.99`, so grey 238. AC3's LINEAR_EXACT centre 50 width 1 gives 127.5, so grey 128. All four match the fixtures. `render_held` passes the whole list and `window.selected` to `VoiTransform::new` and reads the pair back through `voi.window()`, so the folder viewer does not re-select. `VoiTransform::new` checks only the selected pair (`crates/ocelli-pixel/src/lut.rs` around line 343), so the NaN filler in the unselected empty slots is never read.
- Refusal order. A preset meets `voi_present` before `!recipe.ct`. An index meets `validate_frame_request` (with a preset or a numeric window), then the numeric override path, which still draws on a VOI LUT Sequence as F-206 does, then `voi_present`, then `FileWindow::Full`, then the empty slot. Mutation M5, which swaps the two preset checks, turns `a_preset_on_mr_is_the_modality_error` red.
- Lead ruling on the empty first slot. With no index the server opens at the first slot that holds both a centre and a width. An explicit empty index is refused. The page's `effectiveIndex` follows the same rule, so the key `file<n>` and the header always name the same pair. In the browser, MR centres `\40` and widths `\400` opened as `SECOND` with grey 128, and row 1 was listed as `empty` and disabled. Keys 1 and 2 both left it on `SECOND`.
- Presets by modality. The numbers come from `WindowPreset::center` and `width`, and the page holds no `-600`, `1500`, `soft_tissue` or `"CT"` (the AC12 check). The modality test reads Modality (0008,0060) through `optional_text`.
- AC9, AC10, AC11, the region window. `region_extent` runs on F-211's `region_samples` (the pixel-centre rule through `ImagePoint::to_index`, padding exclusion, size cap) and on `roi_statistics` for the extremes. The test also measures the same rectangle through `/measure` and gets count 6, min -1024 and max -1018. In the browser, a drag over columns 8 to 24 of the HU -800 and HU 40 slice read `WW 840  WL -380  region`. Under LINEAR, `c' = -380.5` and `w' = 839`, so HU 40 lies above 39 and gives 255, and HU -800 gives 0. The page showed 255 and 0.
- Cross-story, F-209 linked window. On a 2 by 1 layout with window link on, CT A (`A1\A2`) as the source and CT B (`B1\B2`, 100/200 and -600/1500) as the target: key 5 gave A2 and B2 at -600/1500, key 2 gave Lung on both, key 4 gave A1 and B1 at WW 200 WL 100 with grey 51, a region drag copied `region` to both, and Reset returned both to their first windows.
- AC16, AC17, the allowlist. The 47 names in `LIBRARY_FILES` are exactly the `.js` files in the built `packages/core/dist`. The only bare import in that build is `dcmjs`, and `dcmjs.es.js` has no import statement. Raw HTTP to the release binary served the three listed paths. It returned 404 for `..`, `%2F`, `%2e%2e`, `./`, `//lib`, upper case, `.map`, `.d.ts`, `.test.js`, `%00`, a trailing slash, a fragment and the bare `/lib/`. A query string is stripped before routing, so `index.js?v=1` serves the listed file, which is safe.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo test -p ocelli-folder-viewer -p ocelli-pixel` | worktree at 76e961be | now | 0, every suite ok, preview 103 passed, lib 83 passed |
| `bin/ocelli.sh gate viewer-browser` | worktree, release build, `packages/core/dist` present | now | 0, ALL GREEN. window.mjs printed every step, plus `region window: "WW 840  WL -380  region"` |
| `python3 scripts/guard_probe.py --only viewer-browser.window-stale-grey` | worktree | now | red for its declared reason, control green |
| `python3 -B -m unittest ... test_guard_catalogue.py`, `python3 scripts/guard_census.py` | worktree | now | 57 ok, census lists the new LIMIT |
| Mutations M1 to M13, scratch copy with its own `CARGO_TARGET_DIR`, deleted with `shutil.rmtree` | scratch copy of 76e961be | now | every one red. M1 `selected` to 0: AC1. M2 centre shift removed: AC9. M3 default index 0: empty first slot test. M4 modality check off: AC6. M5 order swapped: AC6. M6 empty slots dropped: AC4. M7 allowlist bypassed: AC16. M8 count 0: AC1 and AC2. M9 per-frame false: AC8. M10 index with a preset allowed: AC2. M11 index on full range allowed: AC2. M12 padding kept: AC9. M13 empty index skipped: AC2 and the empty-slot test |
| Playwright exploration, synthetic folder of 5 files in 4 series, no patient attributes, viewer on `OCELLI_PORT=0` | `scratchpad/explore.mjs` | now | the observations above, plus D1 and D2 |
| `scratchpad/repro410.mjs`, raw fetch to the release binary | synthetic 3-frame enhanced CT with per-frame Frame VOI LUT items | now | `inspect f1 before decode: 200`, then `/preview` for frames 0, 1 and 2, then `inspect f1 after every frame is held: 410` |
| `scratchpad/lib.mjs`, raw socket GETs | release binary | now | the allowlist table above |
| Scratch release binary built without `node_modules` | scratch copy | now | `/lib/dcmjs.js` 404, the tool reads unavailable and gives the library sentence (N2) |
| dicom.nema.org current edition, PS3.3 C.11.2 | fetched now | now | C.11.2.1.2.1 is "Default LINEAR Function". C.11.2.1.2.2 holds the "considered as pairs" and alternative views text. VOI LUT Sequence takes one or more items. Amendment 7's citations are right |

## Defects

### D1, a per-frame window list is re-asked by id after its file's bytes are dropped, and the page then posts the whole file again

**Where**: `examples/folder-viewer/src/serve.rs` `inspect_route` (line 1699) reads `viewer.file_bytes(id)`. `examples/folder-viewer/src/held.rs` `take_encoded_if_ready` (line 2047) drops the posted bytes once every frame of the file is held. `examples/folder-viewer/src/page.html` `refreshFrameWindows` (line 3030) handles the 410 at line 3038 by deleting the post and calling `ensureFile` again.
**What**: on a multiframe with `windowsPerFrame` true, the neighbour prefetch soon holds every frame. A frame change then sends `POST /inspect` by id, gets 410 `the file is no longer held by this process`, and posts the whole file to get a new id. Frames already held under the old id stop being used for that page, so they decode again under the new id, and the cycle can repeat.
**Why it is wrong**: the design's AC17 names "a per-frame repost of the whole file" as the wrong implementation it rejects. The design text says "a frame change never posts the file again". README step 6 says the shown frame's windows are "asked for by file id". The page comment says "so the file is not posted again".
**Evidence**: `repro410.mjs` prints 200 before the decodes and 410 after them. In the browser, one ArrowDown on the enhanced CT logged the requests `/inspect` for frame 1, `/preview` with index 1 for frame 1, `/file`, then `/inspect` for frame 1.
**Scope and regression**: affects every enhanced multiframe whose Frame VOI LUT is per-frame, on HEAD. The cost grows with file size. `inspect_by_id_matches_the_posted_body` posts and inspects without holding every frame first, so it does not see this. The smallest regression is that test with every frame previewed by id before the id-form inspect. Fix options for the author: answer the id form from data that outlives the encoded bytes, or return every frame's window list in the scan reply so nothing has to be asked again.

### D2, a frame whose per-frame list is shorter shows a fault before its own list arrives

**Where**: `examples/folder-viewer/src/page.html` `showIndex` (line 2084) starts `refreshFrameWindows` without waiting for it. `requestGrey` then computes `effectiveIndex` from the scan's frame 0 list. The neighbour prefetch does the same.
**What**: on frame 0 with two windows, key 5 picks index 1. ArrowDown to frame 1, whose own list holds one window, sends `X-Ocelli-Window-Index: 1` for frame 1. The local process correctly refuses with the window error, and the cell shows the fault `the window or presentation could not be applied` over frame 0's corner `WW 1500  WL -600  F0B` until the inspect reply comes back and a second request draws `F1ONLY`. The prefetch made the same refused request as soon as key 5 was pressed.
**Why it is wrong**: the design's state table gives "File index 1, next slice has one window | Arrow key | Index 0 requested, label follows | No fault". The README says "a file window past the next image's list shows that image's first usable one". For a per-frame multiframe the page cannot know the list before the request goes out, so it sends a stale index.
**Evidence**: sampling every 5 ms after ArrowDown gave `{"line":"WW 1500  WL -600  F0B","fault":"the window or presentation could not be applied"} -> {"line":"WW 1500  WL 300  F1ONLY","fault":""}`. Two `400 /preview` responses with index 1 and frame 1 were logged. With D1 in the path the fault lasts for a whole file post.
**Scope and regression**: affects a per-frame multiframe whenever the chosen index is past a later frame's own count. No wrong grey was shown, because the fault covers the previous picture. window.mjs uses classic single-frame slices only, so it does not reach this path. The smallest regression is one window.mjs step on a synthetic enhanced file whose frame 1 lists fewer windows, asserting no fault was ever seen. A fix could wait for the frame's list before sending an index above 0 on a `windowsPerFrame` image, and skip such indices in the prefetch.

## Smells

None.

## Nitpicks

- N1. README step 5 and the `link-window` sentence say a viewport whose image lists fewer windows "shows its first". The code, and step 6's own later sentence, show the first usable one. The wording should agree.
- N2. When `packages/core/dist` is built but `node_modules/dcmjs` is missing, the page and the 404 body both say to run `npm run build -w @ocelli/core`, which does not install dcmjs. Observed on a scratch binary with no `node_modules`. The state is unusual, because the build itself needs `npm ci`.
- N3. `window_rows` returns `None` for the whole list when any one of Window Center, Window Width or the explanation cannot be read. A file the preview still draws with a file window would then list no rows, and on a non-CT the note would say "This file stores no window, so the full range of its values is shown". This is a plausible path that I did not reproduce, and it needs an explanation element that is not text.
- N4. The region tool reads unavailable only after it is first selected, because `loadLibrary` runs from `setTool`. The README's "reads unavailable and says so" holds from that point on.

## Verified clean

- The arithmetic exists once. Grey comes from `ocelli-pixel`'s `VoiTransform` and `LutChain`, and the region centre and width come from `@ocelli/core`'s `windowRegion`. The page computes no window and holds no window number.
- `as` casts: none were added. The index and count conversions use `try_from`, and the extremes narrow through `f32_from_f64` from values that were widened from `f32`.
- Cache key and state: `cacheKey` and the index header are both computed synchronously before the first await in `requestGrey`, so they cannot disagree. The thumbnail `PLAIN_VIEW` key `file0` and the no-header server default name the same pair, because `effectiveIndex` returns 0 only when slot 0 is usable or no row is usable. Per-frame keys include the frame, and after a list is refreshed the key moves with the effective index, so I found no stale grey. `chooseWindow` commits only after a successful reply, and a refused region or preset left the image and its corner unchanged with the reason in the sentence line. Observed: "This region holds one value, so it gives no window." and "The region holds no pixel with a value to window."
- Keys: `onKey` returns early on meta, ctrl or alt, and while typing, so Cmd+1 is not taken. Disabled rows do nothing.
- Docs executed: README step 5, step 6 and the refusal paragraph, the `use.html` sentence, the page sentences, the `bin/ocelli.sh` gate list, the runbook row 597 and its limit, and the catalogue probe. Apart from N1, N2 and N4 they match the tree. CI's viewer-browser job runs `npm ci` only in `tools/bench`, so CI takes the library-sentence leg as the limit says. `more.html` needs no change.
- No regression across F-208 to F-212, F-224 or F-226: the viewer-browser gate is green for aspect, readout, link, forget and window, and the folder viewer and pixel crates are green.

## Limitations and handoff

- Not exercised: the AC13 click-through at sprint close as the operator would do it, a real enhanced multiframe from the corpus (synthetic files only, by instruction), tile mode with number keys, and a Modality LUT Sequence file under the region tool.
- `bin/ocelli.sh gate --sprint` was not run, per the verify-once-at-sprint-end rule.
- Next owner: the implementer remediates D1 and D2. After that, a fresh independent pass is needed on the changed material, replaying both triggers through the page.
