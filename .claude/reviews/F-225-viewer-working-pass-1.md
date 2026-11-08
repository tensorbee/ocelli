# F-225 review, viewer half, pass 1

**Reviewed**: `09f55038..4c583d1b` on `work/f-225-xai`, tip `4c583d1b`, the three commits after the stacking merge (`6943ceaf`, `7ea188a3`, `4c583d1b`). Worktree clean at the start apart from untracked `.venv` and `tools/oracle/out`, which were ignored.
**Author / reviewer**: the three commits are by a separate Claude implementer. The review is by a Claude subagent that wrote none of the design, the library half, the viewer half or any earlier pass.
**Result**: 1 defect, 1 smell, 5 nitpicks

## Acceptance challenged

AC32, the four expected findings, derived from the standard as fetched from dicom.nema.org (current edition) before reading the test's expectations:

- (0008,0018) SOP Instance UID is Type 1 in PS3.3 Table C.12-1. PS3.5 7.4.1: "The Length of the Value Field shall not be zero." Zero length, so TYPE1-EMPTY.
- (0008,0060) Modality is Type 1 in PS3.3 Table C.7-5a. Zero length, so TYPE1-EMPTY.
- (0008,0080) Institution Name is LO, PS3.5 Table 6.2-1 "64 chars maximum". 170 is even, so no pad, so VALUE-LENGTH value 1, 170 characters against 64.
- (0010,1010) Patient's Age is AS, Table 6.2-1 "nnnD, nnnW, nnnM, nnnY", "4 bytes fixed", repertoire "0"-"9", "D", "W", "M", "Y". `040 ` is four bytes, already even, so the SPACE is not 6.2 padding. It breaks both the repertoire and the format. The plan's decision 7 and AC2 report one finding and name it AS-FORMAT, which is a labelling choice the library half settled, not a viewer question.
- No fifth. Patient Position is present, Patient Orientation is not required for CT (C.7-9 2C), Laterality and Content Date and Time are in the not-checked list, every other Type 1 and 2 attribute of the seven modules is present and valid. Specific Character Set is not owed because every value is ASCII.

The test's four JSON lines and the browser check's four rows match this derivation exactly.

AC33, the schedule, against a discriminating case the browser check does not run: a new open while the previous open's request is still in flight. Expected by the plan's state table ("Any, new folder or synthetic open, cache cleared, token advanced") and decision 14: the new file is asked for after its first paint and its badge appears. Observed: no request is ever sent for the new open and its badge stays hidden (D1).

Plan adaptations, judged against the plan's intent:

| Adaptation | Ruling |
|------------|--------|
| A 410 falls back to the body form instead of re-posting | Sound. Measured: on every drawn monochrome file the id form answers 410 and the body form 200 (`normal`, `multi` runs below), which confirms the implementer's reason. A re-post would carry the same bytes and leave a new unheld id behind. The plan row "re-posts per F-216's rule" needs a lead amendment line |
| Cache keyed by the `File` object | Sound. An unheld file has id "" and an id changes on re-post, so the File is the stable key for the open |
| Replies for a file no longer shown are cached | Sound, the report is a function of the bytes. It does not cause D1, which is a generation break |
| One failure sentence | Sound. The page never shows a server message, which also keeps any refusal text off the sheet |
| The sheet asks immediately | Acceptable in intent, opening the sheet is a request for that file. It does break the plan's invariant "No request is issued while the page is in `POST /file` or thumbnail work" and AC33's "never requests during /file or thumbnails" for that one path, because `checksNow` skips the busy wait. Owed: a lead amendment naming the exception (N2) |
| First paint as `vp.paintedFile`, colour images get no badge | The `paintedFile` part is sound. The colour part is S1 |
| Supporting edits (`held_or_body_json`, `countedFilePost`, `loadThumbs` wrapper, eslint globals, gate line) | Sound. `/headers` keeps its error forms, proved by the AC32 test comparing replies byte for byte |
| The LLD left to completion | Acceptable under `/complete-feature`. `docs/lld/dicom-ingest.md` and `public-api.md` are still owed |

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `pgrep -fl 'chromium\|playwright\|oracle'` | host | now | three idle `@playwright/mcp` servers, no Chromium and no oracle process, so no browser job was running |
| `bin/ocelli.sh gate viewer-browser`, own `CARGO_TARGET_DIR` | worktree at `4c583d1b` | now | 0, ALL GREEN. conformance.mjs: library served, "open: asked 335 ms after the first paint", 4 rows, "scroll: the resting file was asked for 303 ms after its paint" |
| `f225-p1v-explore.mjs`, own Playwright script over the gate's release binary | synthetic 32 by 32 CT files built in the script, a colour RGB pair, a 3-frame CT, a 3-file series, a second folder | now | colour: fault "this preview shows monochrome frames only", badge hidden, no request. After opening the sheet: one id request 200, badge "4". Multiframe: one request (id 410, body 200), badge 4 on frame 1 and still 4 at "3 / 3". Normal: id 410 then body 200. Wakeup: see D1 |
| `f225-p1v-privacy.mjs`, `/conformance` by body over the eight files above | same files, values `KOWALSKI^JANINA`, `ZQXJ` x40, `077 `, `PIDZQ99`, `19620203`, `19991231`, `153045`, `HFS`, `9.8.7`, `2.25.` | now | 0. No reply contains any of them. The page's Checks section text in every run also contains none |
| mutations, `f225-p1v-mut.py` on an rsync copy of the worktree with its own target directory. page.html and serve.rs touched before each build, each browser mutant asserted present in the release binary's bytes, each source asserted restored by sha256 | copy of `4c583d1b` | now | table below. Copy baseline green |
| `python3 scripts/prose_check.py`, `npx eslint` on the new browser file and config, `cargo fmt --all --check` | worktree | now | 0, 0, 0 |
| cornerstone floor | `tools/oracle/node_modules` in the main checkout, `@cornerstonejs/*` 5.8.2 MIT, `dicom-parser` | now | no conformance or VR check anywhere. `dicom-image-loader` `wadouri/metaData/metaDataProvider.js:80` reads `patientAge: dataSet.intString('x00101010')`, so `040 ` becomes 40 and the missing unit is dropped silently. The plan's "reports no conformance" holds, and the report is additive over the floor |

| Mutation | Guard | Result |
|----------|-------|--------|
| R1 `conformance_json` answers `headers_json` | AC32 Rust test | red, "the report does not open with its scope" |
| R2 the id form reads the request body | AC32 | red, "the id form was not JSON ... not-part10" |
| R3 `LIBRARY_FILES` loses `conformance` | `library_files_match_the_index_closure` | red |
| R4, mine. `/conformance` ignores the id | AC32 | red |
| B1 `CHECKS_DELAY_MS` 20 | conformance.mjs | red, "scroll: 5 reports were asked for after the steps" |
| B2 the busy wait removed | conformance.mjs | red, "busy: 2 requests were sent while thumbRuns was above zero", and the same for filePostsOpen |
| B3 the 410 fallback removed | conformance.mjs | red, "gone: the requests were id 410", and "back: the badge reads null" |
| B4 the badge shown at zero | conformance.mjs | red, "scroll: the badge reads 0 on a valid file" |
| M1, mine. `checksShown` ignores the cache | conformance.mjs | green. Equivalent at the wire: `pumpChecks` checks the cache again before sending, so only a timer is wasted |
| M2, mine. `applyGrey` no longer calls `checksShown` | conformance.mjs | red, "busy: 0 reports were asked for" and "gone: the requests were ," |
| M3, mine. The library sentence always the dcmjs one | conformance.mjs | red, "library: the section reads ... dcmjs" |
| M4, mine. `resetChecks` removed from `readFolder` | conformance.mjs | green. Nearly equivalent, because the cache is keyed by `File` and a new open has new `File` objects (N4) |
| D1 fix candidate, generation mismatch `continue` | own wakeup scenario | badge 4, the new open asked at 5385 ms right after the stale reply |

## Defects

### D1, a new open while a report is in flight never asks for the new open's report

**Where**: `examples/folder-viewer/src/page.html:5809`, in `pumpChecks`.
**What**: `if (generation !== currentGeneration || !entry) break;` leaves the loop when the reply belongs to a left open. While that request was out, the new open's `checksDue` set `checks.due` and called `pumpChecks`, which returned at once because `checks.busy` was true. The old loop then breaks without reading `due` again, nothing re-arms a timer, and the new file's report is never asked for. The badge stays hidden on a file with findings until the user moves to another file and back, or opens the sheet.
**Why it is wrong**: the plan's state table row "Any, new folder or synthetic open, cache cleared, token advanced" and decision 14, "The page requests it for the current image 300 ms after that image's first paint, if the image is still current". The README also says "A count on the Headers button is the number of findings in the shown file's conformance report".
**Evidence**: own script, `wakeup` case. The first `/conformance` reply was held 5 s by `page.route`, a second folder whose instance 1 is the reproducer was opened meanwhile, and the page was watched 9 s. Requests: `id 410` at 376 ms and its `body 200` at 5386 ms, both for the old open, then nothing. Badge `null`. The sheet then showed "4 findings." for the same file. With line 5809 changed to `if (generation !== currentGeneration) continue;` and `if (!entry) break;`, on the copy, the same run sent `id 410, body 200` for the new file at 5385 ms and the badge read 4.
**Scope and regression**: present at `4c583d1b`. Reachable whenever a report request outlives the next open's posts, thumbnails and 300 ms delay. The sheet's immediate request makes it likelier, because it is sent even during `/file` posts, and a large DX goes by body after its 410. Fix: on a generation mismatch `continue` so the loop reads `due` again (keep `break` for a null entry, which would otherwise loop on an abort), or re-pump in `finally` when `due` is set and not cached. Smallest regression: a step in `conformance.mjs` that holds the first `/conformance` reply with `page.route`, opens a second folder, and requires one report for the new reproducer and the badge 4.

## Smells

### S1, a colour or refused image never gets a badge, then gets one once the sheet is opened

**Where**: `page.html:2442` (`showIndex` returns through `showFault` before any paint, so `applyGrey` never sets `paintedFile` and `checksShown` is never called), and `page.html:5778` (`checksNow` calls `checksShown(file)` for the sheet's file whatever `paintedFile` says).
**What**: on a colour file the badge is hidden and no report is asked for. Opening the sheet asks, and from then on the badge reads the count on that colour image (observed: hidden, then "4" after the sheet). The same holds for any monochrome file whose preview fails, which is the class of file the operator's team hit. Two states for the same image depending on history is the maintenance hazard, and the badge is missing exactly where a refused file would most want it.
**Why it matters**: the plan covers colour only implicitly, through "after that image's first paint", and does not decide either state. README step 8 says "A count on the Headers button is the number of findings in the shown file's conformance report", which a user reads as covering every shown file.
**Fix**: a lead ruling. Either treat a fault shown for an image as its first paint for the checks (call `checksShown(image.file)` beside `showFault` in `showIndex` for the active cell and record `paintedFile`), which gives colour and refused files a badge, or keep the badge to drawn images, make `checksNow` leave `checks.shown` alone, and say in the README that the count appears for images the viewer draws. Regression: open the colour pair and assert the badge is the same before and after the sheet.

## Nitpicks

- N1. In normal use every report costs an id request answered 410 before the body form, because the held bytes are dropped once every frame is held. Harmless and measured, one wasted round trip per file.
- N2. The sheet's immediate request skips the busy wait. Owed as a lead amendment beside the 410 row, so AC33's wording matches the page.
- N3. `the_page_shows_the_conformance_checks` pins `"if (filePostsOpen > 0 || thumbRuns > 0) {"` and the CHECKS constants as source text. The browser half is what discriminates (B1, B2 red), so this is fine, but a reformatted line fails the source test without a behaviour change.
- N4. `resetChecks` in `readFolder` is unobserved by every test (M4 green). Its effect is memory and the badge between the open and the first paint. A one-line assertion in the browser check that the cache is empty after a second open would pin it.
- N5. The AC32 test's "/headers is byte-identical to before" is checked against `headers_json` on the same tree, not against the pre-story reply. It holds because `headers_json` is not in the diff, which is the evidence the plan accepts.

## Verified clean

- AC32 by derivation (above), the by-id and by-body replies byte-identical, the non-Part-10, id-with-body and unknown-id refusals byte-identical to `/headers`.
- AC33 in Chromium 151 through the gate: badge 4 after one request, sent with no file post and no thumbnail run open, 335 ms after the first paint. The sheet's four rows, scope sentence, module line and length detail. Five 60 ms steps send nothing, the resting file is asked once 303 ms after its paint, back to the reproducer reads 4 from the cache. Busy wait for both counters. Library 404 sentence. Failed request sentence with the headers table still filled. The 410 path asks id then body, posts no file, and never asks again.
- The multiframe file: one report for the file, the badge stays across frames, and frame scrolling sends nothing.
- Privacy: no file value in any `/conformance` reply or in the Checks section across eight files, and the page renders only the tag, keyword, rule, catalogue section, value number, measure and the reader-validated module fields.
- No change to `open_frame`, `/preview`, `/file` or `parse_part10` in the range. `conformance_json` is called only from the new route.
- Every new test goes red under its guarded mutation (R1 to R4, B1 to B4) and under three of my four own (M2, M3, R4). M1 and M4 are equivalent or near-equivalent and explained.
- README step 8, use.html 7.1 and more.html "Conformance report" against the tree: the route, the error forms, the reader's reasons, the not-checked list (it equals the served `notChecked` exactly), the four SOP classes and seven modules, the scope sentence and the library sentence are true. The one overstated sentence is S1's.
- Cornerstone 5.8.2 floor: no conformance surface to match, and `intString` on Patient's Age shows the silent acceptance the story answers.
- Boundary and tier: no wasm, no pixels, no GPU, nothing on the render path. Tier rows n/a as the plan declares.

## AC34, the bench driver

Confirmed. `tools/bench/src/viewer_work.mjs:753` in `restartAndDraw` reads `page.locator("#fault").textContent()`, and the page has had no `#fault` element since `8b8bcada` ("F-208, folder viewer viewports"), which replaced `<p id="fault" hidden>` with one `<p class="fault" hidden>` per viewport cell. The same commit migrated the scroll sampler at `viewer_work.mjs:600` to `#wrap .cell.active .fault` and missed this line. `locator.textContent()` waits for the element, so with the page's 180 s default timeout the step throws, and `measureFirstImage` fails after its 18 timed opens, before it returns a value. Every run above found `document.querySelectorAll("#fault").length === 0`. The failure predates F-225.

The smallest correct fix, not made here: read the active cell's fault as line 600 does, an empty string when it is hidden.

```js
const fault = await page.evaluate(() => {
  const node = document.querySelector("#wrap .cell.active .fault");
  return node && !node.hidden ? (node.textContent || "").trim() : "";
});
```

It belongs to F-208's area, so it needs its own owner or a lead decision to carry it in F-225, with `bench_check` and the bench unit tests re-run.

## Limitations and handoff

- AC34 not run. It needs F-216's host alone, the release build and the bench fix above. `viewer.open`, `viewer.first_image`, `viewer.scroll_rate` with the badge on, and the `/conformance` latency for the held 512 by 512 CT and the 17 MB DX, median of 9, are owed. The 17 MB DX reply now includes a body upload after the 410, which that measurement should show.
- Not run this pass: the full `cargo test -p ocelli-folder-viewer`, `gate --floor`, npm tests and the wasm gate. The implementer reports them green. Only the new Rust tests, the library closure test and the viewer-browser gate were run.
- Not exercised: a multi-viewport layout switching active cells, the synthetic open path, the dcmjs-missing branch, and a real DX. The CI branch without a library build was not run.
- Owed acceptance rows: AC34 (integrated, F-216 host), the LLD sections in `docs/lld/dicom-ingest.md` and `docs/lld/public-api.md`, lead amendment lines for the 410 body fallback and the sheet's immediate request, a lead ruling on S1, and the D1 remediation with its regression. AC32 and AC33 are evidenced at `4c583d1b` apart from D1.
- Scratch under the session scratchpad `f225-p1v/`. The workspace copy, its target directory, the gate's target directory and the data folders were deleted with `shutil.rmtree`. No tracked file was edited apart from this report, which is uncommitted.
- Next owner: the implementer for D1 and S1, then a fresh independent pass 2 over the changed material.
