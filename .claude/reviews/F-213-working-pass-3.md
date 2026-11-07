# F-213 review, working pass 3

**Reviewed**: `work/f-213-xai`, range `10caf04e..b8ec548f`, HEAD `b8ec548f19d038170b489bfb319e97969a3d5b16`. No unstaged tracked paths when the pass began. Untracked `.venv` and `tools/oracle/out` exist and are outside the diff.
**Author / reviewer**: a Claude implementer wrote the range, remediating `.claude/reviews/F-213-working-pass-2.md` under the lead's rulings (computed inspect replies kept only for a multiframe whose windows come from a per-frame functional group, charged at real length under a 16 MiB cap reported in `GET /held` as `kept`, tests executing the cap, sweep and counter, every tile following the single view's rule with its own label, the precondition catch removed, an evicted id's replies dropped). This pass is an independent Claude reviewer launched by the sprint lead. It did not write or edit any reviewed code.
**Result**: 4 defects, 0 smells, 5 nitpicks

## Acceptance challenged

- Pass 2 D1, replayed with files built for this pass. A two-frame enhanced CT (frame 0 `40\-600`, `400\1500`, `F0A\F0B`, frame 1 `300`, `1500`, `F1ONLY`, stored 224 and 1064, rescale -1024, Bits Stored 12) was written as explicit LE, then compressed by pydicom 3.0.2 to RLE Lossless and to JPEG-LS Lossless. The JPEG-LS frame is SOF55 with P 12 and no LSE marker. After both frames were held, `/inspect` by id for frame 1 answered 200 with `F1ONLY` for all three, with `held.files` 0. Through the page, window.mjs's `observeMultiframe` logged 0 `POST /file` after the first draw for all three, and the corner went `WW 1500  WL -600  F0B` then `WW 1500  WL 300  F1ONLY`. Closed.
- Pass 2 D2, replayed. A five-frame per-frame file whose frame 2 lists one window `300/1500 ONLY2`, key 5 on frame 0, then Shift G: the tiles settled at `B0 236/94`, `B1 236/94`, `ONLY2 83/0`, `B3 236/94`, so the short list falls back to its own first usable window and names it. ArrowDown in tile mode moved the corner to `B1` and the fifth tile to `B4 236/94`. Closed for the settled state. The interim label is D4.
- Linked F-209 layout. Source a one-frame series, target the five-frame series in Four images, window link on. Key 5 on the source moved the target tiles from `A0 128/0, A1 128/0, ONLY2 83/0, A3 128/0` to `B0 236/94, B1 236/94, ONLY2 83/0, B3 236/94` and its corner to `WW 1500  WL -600  B0`. Key 4 moved them back. With `/inspect` delayed 1.5 s the same final state was reached.
- Independently derived greys, PS3.3 C.11.2.1.2.1 LINEAR. HU 40 at centre -600 width 1500 is `((40 + 600.5) / 1499 + 0.5) * 255 = 236.46`, 236. HU -800 is `((-800 + 600.5) / 1499 + 0.5) * 255 = 93.56`, 94. At centre 40 width 400, HU 40 is `((40 - 39.5) / 399 + 0.5) * 255 = 127.82`, 128, and HU -800 is 0. At centre 300 width 1500, HU 40 is `((40 - 299.5) / 1499 + 0.5) * 255 = 83.36`, 83, and HU -800 is 0.
- The frame 0 decision. A three-frame file whose frame 0 per-frame item has no Frame VOI LUT, so frame 0 falls back to a top-level `40\-600`, `400\1500`, `TOPA\TOPB`, while frames 1 and 2 carry `300`, `1500`, `F1ONLY` per frame. See D2.
- The shared planes tail. On a 500-frame per-frame file with a plane position per frame, `/inspect` by id equalled `/inspect` by body byte for byte for frames 0, 1, 250 and 499, and both answered 400 for frame 500. The same held for the RLE and JPEG-LS files. M10 below (reply drops the tail) turns two tests red.
- The kept pool beside the F-216 cache budget. With `OCELLI_CACHE_MIB=1`, thirty posts of the 500-frame file (641,386 bytes of replies each) peaked at `kept` 16,676,036 under the 16,777,216 cap, the sweep at post 27 took it to 5,772,474, the newest id always answered 200, ids 1 to 21 answered 410 and ids 22 to 30 answered 200, and an id-list forget left `kept` 0. With the default budget, posts 27 to 30 were not kept and still answered 200 from their posted bytes.
- Cost of the computed replies. See D1.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo test -p ocelli-folder-viewer -p ocelli-pixel` | worktree at b8ec548f | now | 0. lib 86, preview 105, measure 16, value 24, every pixel suite ok |
| `bin/ocelli.sh gate viewer-browser` | worktree, release build | now | 0, ALL GREEN. `tiles: "Image 1  B0" 236/94, "Image 2  B1" 236/94, "Image 3  B2" 236/94, "Image 4  B3" 236/94` |
| `scripts/tests/test_guard_catalogue.py`, `guard_census.py`, `--check-runbook`, `prose_check.py` | worktree | now | 57 ok, census exit 0, runbook exit 0, prose clean over 952 files |
| `f213p2-raw.mjs disk` on `f213p3-files/{le,rle,jls}.dcm` | synthetic, pydicom 3.0.2 | now | after both frames held, `/inspect` by id frame 1: 200, 200, 200 |
| `f213p3-page.mjs syntax` (window.mjs `observeMultiframe`) | same three files | now | `POST /file` after first draw 0, 0, 0. JPEG-LS greys 255/255, see Limitations |
| `f213p3-tiles.mjs single`, `link`, each also with `/inspect` delayed 1500 ms | synthetic 5-frame per-frame LE, plus a 1-frame source | now | settled tiles and corners as above. Delayed: tiles 2 to 4 `128/0` with no window word for 1.5 s, see D4 |
| `f213p3-lateprobe.mjs`, `f213p3-latepage.mjs` | synthetic 3-frame file, frame 0 VOI top level only, LE and RLE | now | scan `windowsPerFrame` false, `kept` 0. Page frame 1 corner `WW 1500  WL 300  TOPA`, key 5 then ArrowDown fault `the window or presentation could not be applied` |
| `f213p3-bigpost.mjs`, `f213p3-bigpage.mjs` | synthetic 8 by 8 enhanced CT, 500, 1000, 2000 frames, VOI per frame or shared | now, HEAD binary and a binary built from 10caf04e's four source files | HEAD post 656, 2469, 9662 ms per frame count. Shared control 20, 9, 16 ms. 10caf04e post 30 ms at 1000 and 2000. Page first draw 2000 frames: shared 81 ms, per-frame 9640 ms |
| `f213p3-pool.mjs 1` and default | 500-frame per-frame file posted 30 times | now | as in the pool bullet above |
| `f213p3-bytes.mjs` | 500-frame, RLE, JPEG-LS files | now | id form equals body form for every frame asked |
| Rust mutations M1 to M14, `f213p3-mut.py`, scratch copy with its own `CARGO_TARGET_DIR`, deleted with `shutil.rmtree` | scratch copy of b8ec548f | now | see the mutation list below |
| Page mutations PM1 to PM6, `f213p3-pmut.py`, window.mjs from the scratch copy | scratch copy | now | PM1 to PM5 red, PM6 green, see D3 |
| `f213p3-wff.mjs` | Playwright from `tools/bench/node_modules` | now | an async predicate that is never true resolved in 9 ms. A sync `() => false` timed out at 2003 ms |

Rust mutations, each reverted after its run:

- M1 nothing kept: red in `a_compressed_per_frame_multiframe_is_inspected_by_id_after_every_frame_is_held` and `inspect_by_id_matches_the_posted_body`.
- M2 evicted id still answers, M5 forget leaves bytes, M6 keep for an id not held, M13 eviction removal not subtracted: red in `kept_replies_follow_the_id`.
- M3 sweep frees nothing: red in `a_full_pool_sweeps_ids_that_hold_nothing`. M4 cap `usize::MAX`: red in that and `kept_replies_stay_under_the_cap`.
- M7 charge omits the heads: red in the compressed test. M8 every file keeps: red in `a_file_whose_windows_do_not_change_by_frame_keeps_nothing`. M10 reply drops the planes tail: red in the compressed test and `inspect_by_id_matches_the_posted_body`.
- Green: M9 (a one-frame file keeps), M11 (a differing planes tail kept), M12 (a replaced entry not subtracted), M14 (the cap inside `per_frame_replies` ignored). See N4.

## Defects

### D1, computing every frame's reply at post time is quadratic in the frame count, so a 2000-frame per-frame multiframe takes 9.7 s to post and to first draw

**Where**: `examples/folder-viewer/src/preview.rs:2687`, `per_frame_replies` calls `inspect_object` once per frame. Each call runs `push_window_fields` at line 2839, which runs `project_metadata` over the whole object, and writes every frame's plane through `plane_tokens(object, frames)` at line 2841. `examples/folder-viewer/src/serve.rs` `file_route` runs it on every `POST /file` before the reply, whether or not the file is then held.
**What**: the post of an N-frame per-frame multiframe does O(N) work N times. Measured on the release binary: 500 frames 656 ms, 1000 frames 2469 ms, 2000 frames 9662 ms, about four times per doubling. The same files with the VOI in the shared group posted in 20, 9 and 16 ms, and the 2000-frame per-frame file posted in 30 ms on a binary built from 10caf04e's sources. Through the page, the 2000-frame per-frame file drew its first frame at 9640 ms against 81 ms for the shared control, because the page waits for the post before its first preview. A post that answers 410 is posted again by `refreshFrameWindows` and `postPreview`, so a repost pays it again.
**Why it is wrong**: the ruling keeps replies so that a frame change does not post the whole file again. Moving the cost to a ten second first draw makes opening the study worse than the repost it replaces, and it holds one of the four request body slots throughout. `FRAME_LIST_CAP` allows 65,536 frames. Unmeasured, but by the measured curve a file large enough to reach the 16 MiB cap, about 7000 frames here, would compute for minutes before `per_frame_replies` gives up.
**Evidence**: the `f213p3-bigpost.mjs` and `f213p3-bigpage.mjs` rows above.
**Scope and regression**: introduced by this range, every per-frame enhanced multiframe, all transfer syntaxes. The smallest regression is a test that counts how many times the frame-independent fields and the plane list are built for one post and requires one, rather than a timing bound. A fix could build the frame-independent head and the planes tail once and only the window fields per frame, from one `MultiframeMetadata`.

### D2, the per-frame decision is frame 0's, so a file whose later frames alone carry a per-frame VOI shows a label from another frame's list, and pass 1 D2's fault comes back

**Where**: `examples/folder-viewer/src/preview.rs` `window_rows` sets `per_frame` from the source of frame `frame`'s Window Center, and both the scan's `windowsPerFrame` and `per_frame_replies` (line 2679, `window_rows(&metadata, 0)`) read it for frame 0. `page.html` `windowsKnown` and `refreshFrameWindows` trust `image.info.windowsPerFrame`.
**What**: on a three-frame file whose frame 0 has no Frame VOI LUT and falls back to a top-level `TOPA\TOPB` list, while frames 1 and 2 carry `F1ONLY` per frame, the scan says `windowsPerFrame` false and `kept` stays 0. The page never asks frame 1's list. On frame 1 with key 4 the corner read `WW 1500  WL 300  TOPA` and the list marked `TOPA WW 400 WL 40` as chosen, while the picture was drawn at frame 1's own 300/1500, grey 83 and 0. Key 5 on frame 0 then ArrowDown sent index 1 for frame 1 and showed `the window or presentation could not be applied`. The process itself resolves frame 1 per frame, as `/inspect` by id for frame 1 answered `windowsPerFrame` true and `F1ONLY 300/1500`.
**Why it is wrong**: README step 6 says "A multiframe whose windows change per frame lists the shown frame's own", and this one lists frame 0's. The corner names `TOPA`, a window at 400/40, beside a picture at 300/1500, which is a quietly wrong label. Pass 1 D2 forbade a request naming an index the frame's own list has not confirmed. I believe PS3.3 C.7.6.16 does not let a macro sit per frame for some frames only, so the input is likely non-conformant, but that clause was not re-read in this pass and the process accepts and draws the file.
**Evidence**: `f213p3-lateprobe.mjs` and `f213p3-latepage.mjs`, explicit LE and RLE copies.
**Scope and regression**: present on HEAD. The flag came in with F-213's earlier commits and this range extended the same frame 0 decision to the pool. The smallest regression is this three-frame file in window.mjs, asserting frame 1's corner word and no fault. A fix could set the flag when any frame's VOI resolves from the per-frame group, or have the process refuse the file's window rows when frames disagree on the source.

### D3, window.mjs's "fully held" precondition never waits, because its predicate is async

**Where**: `examples/folder-viewer/tests/browser/window.mjs:324`, `page.waitForFunction(async () => { ... return held.files === 0 && held.frames === 2 }, ...)`.
**What**: an async function returns a Promise, which Playwright reads as a truthy result, so the wait resolves on its first poll whatever `/held` says. Removing the `.catch` therefore changed nothing. PM6, which changes the condition to `held.frames === 3`, which is never true, left window.mjs green with exit 0. A standalone check resolved a never-true async predicate in 9 ms and timed out a sync `() => false` at 2003 ms.
**Why it is wrong**: pass 2 S3 asked that the step fail or report not observed when the posted bytes have not been dropped before ArrowDown. The step still passes on whatever state the prefetch has reached. PM5 (the server ignores the kept replies) went red today, so on this host the state is reached by timing, not by the wait. The catalogue's "a frame change on a fully held multiframe" and the problem text "on a fully held multiframe" describe a precondition nothing checks.
**Evidence**: PM6 and `f213p3-wff.mjs` rows above.
**Scope and regression**: carried from pass 2 S3, not closed. Only window.mjs uses an async `waitForFunction` predicate in the tracked browser tests. A fix could poll `/held` from Node with `fetch` in a loop with a deadline, or use a sync predicate over state the page keeps, and fail the run when the deadline passes. PM6 is the check that it now goes red.

### D4, README says each tile's label names the window it is drawn at, and until its frame's list arrives the label names none

**Where**: `README.md` step 6, "Each tile of Four images follows the same rule and its label names the window it is drawn at". `page.html:2560` writes `word` only when `windowWord` finds one, and for a frame whose list is not known `effectiveIndex` is `null`, so `word` is empty.
**What**: with `/inspect` delayed 1.5 s, key 5 then Shift G drew tiles 2 to 4 as `Image 2 128/0`, `Image 3 83/0`, `Image 4 128/0` for 1.5 s, at each frame's first usable window and not the chosen second one, with no window word, then `B1 236/94`, `ONLY2 83/0`, `B3 236/94`. The single view's corner shows the WW and WL numbers during the same wait. A tile shows nothing about its window.
**Why it is wrong**: the sentence is false for that interval, and D1 makes the interval up to the cost of a repost. CLAUDE.md's rule is that a different result is never shown without saying so.
**Evidence**: `DELAY=1500 f213p3-tiles.mjs single`.
**Scope and regression**: present on HEAD, introduced by this range's tile change. The smallest fix is to narrow the sentence to the settled state, or to label an interim tile with its reply's WW and WL. window.mjs's `tileProblems` judges only the settled tiles, so a check of the interim label would need a delayed `/inspect` route.

## Smells

None.

## Nitpicks

- N1. Each arrival of a tile frame's list repaints every tile, which aborts and re-sends the others. One Shift G sent `preview f1 i1` three times.
- N2. After an id's posted bytes are dropped, `/inspect` by id for a frame past the file's count answers 410, where the body form and the held id answer 400. The page never asks past the count.
- N3. With a 1 MiB cache the pool held 16,034,650 bytes of replies while the cache held 8 files, because replies of evicted ids go only when asked for or under cap pressure. README states the "when it is next asked for" half.
- N4. Four mutations survive. M9, a one-frame per-frame file keeps a reply nobody asks for. M11, `plane_tokens` does not depend on the frame, so the tail mismatch branch is unreachable. M12, every post gets a fresh id, so the replace branch in `keep_inspect` is unreachable. M14, the cap inside `per_frame_replies` is redundant with `keep_inspect`'s check for the pool, but it is the only bound on D1's compute for a huge file, and no test reaches it.
- N5. `KeptReplies::bytes` counts the strings and the head list, not the `Arc` or the map entry. README's "charged at the bytes it holds" and the LLD's "the heap bytes it owns" read correctly for the struct.

## Verified clean

- Pass 2 D1 closed for explicit LE, RLE and JPEG-LS, by raw HTTP and through the page, as above. M1 and PM5 go red.
- Pass 2 D2 closed for the settled single view, tile mode, a frame with a shorter list and a linked layout. PM1 (no `refreshFrameWindows` in `paintTiles`), PM2 (no tile repaint on arrival), PM3 (no label word) and PM4 (the label read from the first tile's frame) each turn `tileProblems` red.
- Pass 2 S1 closed. Only a per-frame multiframe keeps replies (`kept` 0 for shared and top-level files, M8 red), charged at the length the test derives from the replies (M7 red). The 500-frame file was charged 641,386 bytes.
- Pass 2 S2 closed. `/held` reports `kept`, and the cap, sweep, forget and eviction paths are executed by tests that go red under M2 to M6 and M13.
- Pass 2 N1 closed. An evicted id answers 410 on `/inspect` and its replies leave the counter.
- The kept reply equals the body reply byte for byte, planes included.
- `/inspect` by id for a held id with kept replies never parses the posted bytes. The kept pool stayed under 16 MiB with the cache at 1 MiB and at the default.
- README's pool paragraph, its forget sentence, `docs/lld/cache.md`'s new paragraph (every named function exists and behaves as stated), use.html's new paragraph, the catalogue's tile text and the runbook's `tileProblems` line match the tree, apart from D2's "per-frame functional group" decision, D3's "fully held" and D4's label sentence.
- No new `as` cast, no pixel arithmetic changed. Greys still come from `ocelli-pixel`.

## Limitations and handoff

- External to F-213, not counted, and narrower than pass 2 said. The JPEG-LS decode fault is not limited to the preset-parameter path. `f213p3-files/jls.dcm` is SOF55 with P 12 and no LSE marker, and pydicom and imagecodecs decode it to the explicit LE pixels exactly. The folder viewer's preview of the same frame is right for pixels 0 to 79 and wrong from pixel 80, row 2 column 16, on: 915 of 1024 greys differ at window -600/1500, and row 16 reads `109,255,255,...` where explicit LE and RLE read 94 and 236. That is a quietly wrong pixel. Reproduce with `scratchpad/f213p3-jlsprobe.mjs` and `scratchpad/f213p3-files/jls.dcm`. The page's 255/255 JPEG-LS greys in this pass come from it and are not F-213 evidence.
- Synthetic files only, by instruction. No corpus multiframe, no JPEG 2000 or HTJ2K copy (they reach `per_frame_replies` through the same parse). The quadratic curve was measured to 2000 frames and the larger figures are extrapolated.
- `bin/ocelli.sh gate --sprint` was not run, per the verify-once-at-sprint-end rule.
- Next owner: the implementer remediates D1 to D4. A fresh independent pass is then needed on the changed material, replaying the 2000-frame post, the frame 0 decision file, PM6 and the delayed tile label through the page.
