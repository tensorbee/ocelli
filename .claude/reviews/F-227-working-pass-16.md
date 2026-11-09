# F-227 review, pass 16

**Reviewed**: `git diff dd585041..112aa5f9` on branch `work/f-227-xai`, HEAD `112aa5f9`, tracked tree clean (untracked `.venv` and `tools/oracle/out` only). Folder viewer half: 19 files, 2165 insertions, 232 deletions. The library half (`crates/ocelli-pixel`) was reviewed clean at pass 15 and is read here only as a provider.
**Author / reviewer**: the viewer half is by a Claude implementer, per the commit messages and `.claude/scratch/F-227-progress.md`. This pass is by an independent Claude reviewer who authored none of the reviewed material and none of the earlier remediation.
**Result**: 4 defects, 0 smells, 3 nitpicks

## Acceptance challenged

- **The pooled arithmetic, derived independently.** A Python harness (`numpy.float32`, written from the design's items 3, 5, 6 and 7 and PS3.3 C.11.1, C.7.5.1.1.2 and C.11.2.1.3.2, not from the source) posted synthetic Part 10 files to a release viewer and compared `/series-window` and `/preview`.
  - PT, 20 signed files of 8 by 8, half stored 0, slopes `0.0731 + 0.0113 i` (not exact in binary), so `series_sample(20)` takes 16. Hand: n = 512, ranks 6 and 507, low 5.922, high 759.41486. Route: counted 512, centre `382.66794`, width `753.4919`, edges 6 and 16 f32 units inside the hand percentiles, the library's margin. Four probe samples on files 0, 7, 12 and 19 gave the hand grey under the series window (30, 88, 0, 183) and a different grey under each file's own window (111, 147, 0, 175), so per-image and one-slope pools are rejected.
  - CT, 5 unsigned files, intercept -1024, slope 1 or 0.5 by file, stored 0 air, stored 4000 to 4009 on every file and a padding range 4000 to 4095 on file 1 only. Hand: n = 260 (5 times 54 less file 1's 10), ranks 3 and 258, low -520, high 2985. Route counted 260, centre `1232.4985`, width `3504.995`. File 1's stored 4002 (977 HU) draws grey 109 under the series window as by hand, and 255 under its own window. Excluding modality 0 instead of stored 0, or applying padding to every file, gives a different n.
  - AC11 by hand: values 1 to 100 and 2 to 200 by 2, n = 200, ranks 2 and 198 are 2 and 196, centre 99, width 194. Frame B stored 50 is `((100 - 99) / 194 + 0.5) * 255 = 128.81`, grey 129, frame A stored 50 is 63.09, grey 63. The test asserts both.
  - AC13 by hand: mixed list 1 to 100 and 101 to 200, ranks 2 and 198, centre 100, width 196, stored 150 grey 193, sampled 3, skipped 1. Padded file: 51 to 100 counted, n = 50, ranks 1 and 50, centre 75.5, width 49, stored 75 grey 125. Without padding n = 70, centre -950, width 2100, stored 75 grey 252. My harness reproduced the mixed case on the live route (`99.99988`, `195.99976`, sampled 3, skipped 1, stored 150 grey 193) and the windowed middle file kept its own grey 64 with `none`.
- **Eligibility.** Colour is refused at decode (`NotMonochrome`), so a colour sample is `Refused` and skipped. `contribute` requires `voi_lut == Ok(false)` and `FileWindow::Auto` with readable padding. The page's `autoEligible` reads the image's window list and VOI LUT list and `monochrome`, and the process decides the final label.
- **F-213.** A file window is never replaced: mutant MD (series window applied before the file window branch) goes red. A preset request sends no series header and its cache key is the preset token alone. The Auto window row is index 0, the file window state.
- **F-209.** A linked Reset copies index 0 and no window, so each target sends its own series entry. auto.mjs `observeLinked` observed it.
- **Cache key.** Mutant PA (cache key without `seriesHeader`) built in the scratch copy and run through auto.mjs went red: `PT slice 11 reads "WW 88.99986  WL 54.499947  Auto window, this image"`, a thumbnail's per-image grey shown in the main view. The key as written separates them.
- **410 once and retry once, F-216, F-224 and F-226.** Driven in Playwright against restart, eviction and a changed file, which is where D1 and D2 come from. A new open clears all three maps through `dropCache`, which `beginOpen`'s caller runs first.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo test -p ocelli-folder-viewer -p ocelli-pixel` | worktree HEAD, worktree target | now | 0. viewer lib 91, preview 116, value 24, measure 16, pixel auto_window 32, all others green |
| `bin/ocelli.sh gate viewer-browser` | worktree HEAD, release | now | 0, `ALL GREEN`. auto.mjs: PT `WW 116.49986  WL 64.24994  Auto window`, 1 request for PT, 2 in all, refused reads `Auto window, this image`, linked Reset returns the MR cell to its own window |
| `cargo clippy -p ocelli-folder-viewer --all-targets -D warnings`, `cargo fmt --all --check` | worktree | now | 0 and 0 |
| `scripts/prose_check.py`, `scripts/source_provenance_check.py` | worktree | now | clean over 986 and 1870 files |
| `node --test tools/bench/tests/viewer_work_test.mjs` | worktree | now | 15 pass |
| `arith.py` PT, CT, MIXED against `/series-window` and `/preview` | release binary, synthetic files, ephemeral port | now | every route value within the margin of the hand percentiles, every probed grey equal to the hand grey |
| `mono1.py`, 4 by 4 all stored 0, MONOCHROME2 then MONOCHROME1 | release, synthetic | now | basis `background`, centre 0.5, width 1, greys `[0, 0, 0, 0]` then `[255, 255, 255, 255]` |
| `p16.mjs restart`, two PT series of 20, restart on the same port after series 1, then series 2 | release, Playwright, synthetic | now | `/series-window` 200 then 410, 410. Series 2 slices 1 to 4 read WW 24.999924, 37.499924, 49.999847, 62.499817, each `Auto window, this image` |
| `p16.mjs evict`, `OCELLI_CACHE_MIB=4`, 40 files of 256 by 256 | release, Playwright, synthetic | now | 410, 410. Slices 1 to 4 read WW 44.49993, 66.74987, 88.99986, 111.24986, each `Auto window, this image` |
| `p16.mjs changed`, `s2-04.dcm` (index 4, outside `series_sample(20)`) given a new modification time, `probeFiles` over series 2, then series 2 shown | release, Playwright, synthetic | now | no `/series-window` request for series 2. Slices 1 to 4 read WW 24.999924 to 62.499817, each `Auto window, this image`. Without the explicit probe first, the request won the race and returned 200 |
| Mutant MA, `series_window` called on every held `/preview` carrying a series header | scratch copy, own target | now | `the_series_window_is_not_recomputed_per_preview` **ok**, survives |
| Mutants MB (series branch draws the image's own window), MC (pool drops padding), MD (series window over a file window), ME (windowed image contributes) | scratch copy | now | each 101, red in the series, padded, windowed or mixed tests |
| Mutant PA, cache key without the series header, auto.mjs | scratch copy, release | now | exit 1, slice 11 per-image grey |
| Guard probe `viewer-browser.auto-per-slice`, run by hand both ways | scratch copy | now | 0 unmutated, 1 with `got.line !== line` replaced by `false`, printing `auto check accepted a per-slice window` |
| AC17, `tools/bench/out/run.json` | ignored file, started 2026-11-07T19:14:35Z | prior receipt | viewer.first_image CT 11.7, DX 29.8, pet_no_window 15.0 ms, as the progress note records. Load average 8.3, 9.5, 11.6 |

## Defects

### D1, two or more gone files of a series turn the series window into a window per slice

**Where**: `examples/folder-viewer/src/page.html` `askSeriesWindow`, the `response.status === 410 && !reposted` branch, with `serve.rs` `series_window_route` naming only the first gone id.
**What**: the route stops at the first sampled id that is not held and names it. The page re-posts that one file, sends the list once more, and on the next 410 returns `null`, which `ensureSeriesWindow` records in `seriesWindowRefused` for the rest of the open. After a process restart every id is gone, and under F-216's budget any files posted at open can be. Every image of the series is then drawn at its own window, and the corner says `Auto window, this image` with a different WW and WL on each slice.
**Why it is wrong**: design item 11 (the operator's answer, one window for every eligible image so scrolling keeps relative brightness) and item 15 (no per-image window while a series window can be computed). Item 18 names restart and eviction as the 410 path that keeps the series window, and the re-post once rule it states cannot deliver that when more than one sampled file is gone, so this is also a plan statement found false that the implementer's list does not report. AC16 runs on the operator's study, where a folder of 26 series can exceed the 256 MiB default and evict the PT files posted at open.
**Evidence**: `p16.mjs restart` and `p16.mjs evict` above. Restart log `200, 410, 410`, then WW 25, 37.5, 50, 62.5 over slices 1 to 4. Eviction log `410, 410`, then WW 44.5, 66.7, 89.0, 111.2.
**Scope and regression**: present on HEAD. Affects every no-window series first shown after a restart or after eviction of two or more of its sampled files. Smallest regression: a browser or HTTP case that makes two sampled ids gone (restart, or `OCELLI_CACHE_MIB` small) and requires one series window and the same corner on two slices. The repair is a design call for the lead: re-post every gone id until none is gone, bounded by the series length, or have the route name every gone sampled id at once.

### D2, a changed file anywhere in a series prevents its series window

**Where**: `page.html` `askSeriesWindow`, `const id = await ensureFile(image.file)` over every image, and `ensureSeriesWindow`'s `.catch(() => {})`.
**What**: `ensureFile` throws F-224's `FileChanged` for a file marked changed. `askSeriesWindow` posts every image of the series, sampled or not, so one changed file rejects it before any request is sent. The catch stores neither an entry nor a refusal, so each slice asks again, fails again and is drawn at its own window.
**Why it is wrong**: design item 12 says a sampled image that cannot contribute is skipped and the others still pool, and the window is a function of the sampled images alone. Here a file outside `series_sample` decides whether the series has a window at all.
**Evidence**: `p16.mjs changed` with `s2-04.dcm`, index 4, which `series_sample(20)` does not pick. No `/series-window` request for series 2, slices 1 to 4 at WW 25, 37.5, 50 and 62.5, each `Auto window, this image`.
**Scope and regression**: present on HEAD whenever F-224's probe marks a file of a series before that series' window is asked for. Smallest regression: changed.mjs's mechanism on an unsampled file of a no-window series, then one series window on the other slices.

### D3, the AC14 counter cannot see a pool made per preview

**Where**: `serve.rs` `series_window_route`, `viewer.note_series_pool()`, and the test `the_series_window_is_not_recomputed_per_preview`.
**What**: the counter is incremented by the `/series-window` handler before it calls `series_window`, not where a pool is made. A pool built on the `/preview` path never touches it, so "none per `/preview`" holds whatever the preview path does.
**Why it is wrong**: AC14 requires a counter that shows one pool per `/series-window` request and none per `/preview`, and the design's mutation list names "pool per preview (AC14)" as seen red. Microscope section 2: a test that passes both ways is a defect.
**Evidence**: mutant MA called `series_window(&[(id, frame)], ..)`, which makes an `AutoWindowPool`, on every held `/preview` with a series header. The test passed.
**Scope and regression**: test only, present on HEAD. The fix is to count where the pool is made, for example inside `series_window`, and to replay MA.

### D4, "drawn black" is false for MONOCHROME1

**Where**: `examples/folder-viewer/src/more.html`, `#auto-window`, "a frame of padding and zero only is drawn black", and `docs/lld/pixel-pipeline.md`, "`Background` ... draws its largest at the output minimum, black".
**What**: the Background window maps the largest left-out value to the VOI output minimum. On MONOCHROME1 the chain's presentation stage inverts, so the frame is drawn white.
**Why it is wrong**: design item 9 says "black on `MONOCHROME2`", PS3.3 C.7.6.3.1.2 makes the minimum white in MONOCHROME1, and microscope section 3 makes a false sentence in more.html a defect.
**Evidence**: `mono1.py`, the same all-zero frame drew `[0, 0, 0, 0]` as MONOCHROME2 and `[255, 255, 255, 255]` as MONOCHROME1, both basis `background`.
**Scope and regression**: prose only, present on HEAD. Qualify both sentences with MONOCHROME2, or say "at the output minimum".

## Smells

None.

## Nitpicks

- **N1** `page.html` `seriesHeader` does not check `autoEligible`, so a windowed image at index 0 in a mixed series sends a series header the process ignores and keeps its main-view grey under a different cache key from its thumbnail. Correct, one extra request.
- **N2** `validate_frame_request` accepts a series window with a function sent alone, which draws the series centre and width under LINEAR or SIGMOID and still reports `series`. No page path sends it, and the design's refusal list does not name it.
- **N3** `use.html`'s new example pools any frame whose window is `Absent` without checking `opened.photometric()`, while the next sentence says it is what the folder viewer does, and the viewer refuses colour first.
- **Carried**: pass 15 N1 and the earlier library nitpicks, unchanged.

## Verified clean

- Every hand value above, plus AC10's one-sample grey 128, the RLE and JPEG lossless comment rewrites (four values give ranks 1 and 4, and 0, 2047, 2048, 4095 counts three values from 2047), and `span_frame`'s 27, 199, 200 ranks 1 and 3.
- No `as` cast and no `#[allow]` added. `exact_bound` takes i16 then u16, so a US or SS padding bound is exact in `f32`.
- `value.rs` moves the padding reader without changing it, and the 24 value tests stay green.
- The route parses digits only for the frame, caps 65,536 lines, and checks the body cap at compile time. A busy reply is retried, and only a departed peer gives it.
- No recompute per preview in the tree as written: the series pool is made only in `series_window_route`, and the per-image window is computed once per decode in `prepare_held`. D3 is about the guard, not the tree.
- `tools/bench/src/viewer_work.mjs` `restartAndDraw`: `#wrap .cell.active .fault` matches the node `showFault` writes. Cells come from `cell-template` inserted into `#wrap`, and `.active` is toggled on `vp.dom.cell`. My Playwright probes read the fault through the same selector.
- README, use.html, more.html, pixel-pipeline.md and benchmarks.md sentences executed: the row after the presets on a CT, the corner words, Reset and the linked Reset, the tile words, the thumbnail fallback, `open_frame` still inventing nothing (`WindowInput::Absent`), the API names in both examples (`parse_part10(&[u8])`, `open_frame(&ParsedDicom, u32)`, `window()`, `padding()`, `modality()`, `PaddingRange::value` and `limit`), the workload description against `petSlope` and `petPixels`, and the cornerstone3D 5.8.2 sentence against `StackViewport.js` `_getInitialVOIRange`, which falls back to the image's range unless a PT image is pre-scaled with SUV. Only D4's sentences failed.
- The oracle is untouched (AC18). No file under `tools/oracle` is in the range.

## Limitations and handoff

- **AC17**: the first image figures were measured as recorded. `tools/bench/out/run.json` started at 19:14:35Z on the source of `d0a1fc21`, and the later commits change only tests. The load average there was 8.3 to 11.6, not the 4 to 9 the progress note gives, and the design asks for F-216's host otherwise idle. The `viewer.open` figures (965 ms and 107 ms) have no surviving record, because `run.json` was overwritten, and the note says they came from a tree before the prefetch guard. Both are implementer runs. AC17 stays with the sprint lead on an idle host and was not re-run here.
- AC16 needs the operator's study and was not run. D1 makes it likely to fail where the study exceeds the cache budget.
- Not exercised: the four-image tile mode before and after the series window arrives, a multiframe with per-frame windows in a no-window series, a refused decode inside a sampled list over HTTP, and a network failure of `/series-window`.
- No GPU, wasm or oracle job, by instruction. The scratch copy and its target under `scratchpad/f227-p16` are deleted with Python `shutil` after this file is written, and every viewer this pass started was stopped.
- Next owner: the implementer for D3 and D4. D1 and D2 change the design's item 18 and the page's posting rule, so the lead rules on them first. Then a fresh independent pass.
