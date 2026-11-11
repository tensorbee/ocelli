# F-213 review, working pass 2

**Reviewed**: `work/f-213-xai`, range `cf943b94..24d27911`, HEAD `24d27911b08dba010f17a279f0fd7b469be4662e`. No unstaged tracked paths when the pass began. Untracked `.venv` and `tools/oracle/out` exist and are outside the diff.
**Author / reviewer**: a Claude implementer wrote the range, remediating `.claude/reviews/F-213-working-pass-1.md` under the lead's rulings D1 (kept data set outside the LRU, capped at 16 MiB, dropped by F-226's forget) and D2 (no window index for a frame whose own list is not known). This pass is an independent Claude reviewer launched by the sprint lead. It did not write or edit any reviewed code.
**Result**: 2 defects, 3 smells, 2 nitpicks

## Acceptance challenged

- Pass 1 D1, replayed. On an uncompressed enhanced CT the id form now answers after every frame is held: `/held` read files 0 and frames 2, then `/inspect` by id for frame 1 answered 200 with `F1ONLY`. Through the page, one ArrowDown made 0 `POST /file`. The same file encoded with pydicom as RLE Lossless and as JPEG-LS Lossless answered 410 after every frame was held, and through the page one ArrowDown made 1 `POST /file` for each. See D1.
- Pass 1 D2, replayed. On the window.mjs multiframe no fault was sampled and frame 1 showed `F1ONLY` at grey 83 and 0. With `/inspect` delayed by 1.5 s through a Playwright route, a linked target on frame 1 of a per-frame multiframe went `WW 1500  WL 300` (its own first window, no label) then `WW 1500  WL -600  T1B` at grey 236 and 94, with no fault, and its first request for frame 1 with index 1 left only after the list arrived. Tile mode does not follow this. See D2.
- Independently derived greys. Stored 224 and 1064 with rescale -1024 give HU -800 and 40. Under PS3.3 C.11.2.1.2.1 LINEAR at centre -600 width 1500, `((40 + 600.5) / 1499 + 0.5) * 255 = 236.46` gives 236 and `((-800 + 600.5) / 1499 + 0.5) * 255 = 93.56` gives 94. At centre 40 width 400, HU 40 gives `((40 - 39.5) / 399 + 0.5) * 255 = 127.82`, so 128, and HU -800 gives 0. These are the values D2's tiles are judged against.
- Cache key "default" against a real index 0. The page's "default" key is the request with no index header, which the process opens at the frame's first usable slot. `effectiveIndex` on a known list returns 0 only when slot 0 is usable or no slot is, and both of those also send no header. So a "default" grey and a `file0` grey are the same picture whenever both exist, and a collision could not show a wrong grey. The separate key only costs a duplicate request.
- `/inspect` by body. `inspect_json` already strips the pixel value itself, so the kept header and the posted body parse to the same JSON. `inspect_by_id_matches_the_posted_body` compares the id form with the body form, and the scan, which is the body form, drew every folder in the gate.
- The dcmjs leg. A scratch release binary with `packages/core/dist` present and an empty `node_modules` served `/lib/ocelli-core/index.js` 200 and `/lib/dcmjs.js` 404 with `dcmjs is not installed. Run npm ci in the repository`, and the region tool read `The region window needs dcmjs, which the library imports. Run npm ci in the repository, then start the viewer again.` with `aria-disabled` true. `secondary-capture.js`, `tid1500.js` and `tid2010.js` are the dist modules that import dcmjs, which matches "The library's SR modules import dcmjs".

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo test -p ocelli-folder-viewer -p ocelli-pixel` | worktree at 24d27911 | now | 0. lib 83, preview 103, measure 16, value 24, pixel suites all ok |
| `bin/ocelli.sh gate viewer-browser` | worktree, release build | now | 0, ALL GREEN. `multiframe: key 5 "WW 1500  WL -600  F0B" grey 236 and 94, ArrowDown "WW 1500  WL 300  F1ONLY" grey 83 and 0, 0 faults seen, 0 file posts` |
| `test_guard_catalogue.py`, `guard_census.py`, `--check-runbook`, `prose_check.py` | worktree | now | 57 ok, census and runbook clean, prose clean over 951 files |
| `scratchpad/f213p2-raw.mjs syntax` and `disk le rle jls` | synthetic 2-frame enhanced CT, per-frame Frame VOI LUT, explicit LE, then pydicom 3.0.2 RLE Lossless and JPEG-LS Lossless copies | now | after both frames held: LE 200, RLE 410, JPEG-LS 410 |
| `scratchpad/f213p2-page.mjs syntax` (window.mjs `observeMultiframe` on each file) | same three files | now | `POST /file` after the first draw: LE 0, RLE 1, JPEG-LS 1 |
| `f213p2-page.mjs tile`, at HEAD and with `if (!windowsKnown(image)) return null;` removed | synthetic 4-frame enhanced CT, every frame `40\-600`, `400\1500` | now | HEAD tiles `[236,94] [128,0] [128,0] [128,0]` under corner `WW 1500  WL -600  B0`. Without the line `[236,94]` four times |
| `f213p2-page.mjs link`, `/inspect` delayed 1.5 s | 1-frame source, 2-frame per-frame target on frame 1, window link on | now | target `WW 1500  WL 300 83/0` then `WW 1500  WL -600  T1B 236/94`, no fault |
| `f213p2-raw.mjs forget`, `big`, `fill` | synthetic, page token, a 17 MiB private OB, 300 one-frame files with 60 KiB private OB then 200 plain ones | now | page forget then 410. 17 MiB data set 410 after hold. After the fill, `used` 1,312,000 bytes, the multiframe 410 after hold |
| Rust mutations M1, M3, M4 to M8, scratch copy with its own `CARGO_TARGET_DIR`, deleted with `shutil.rmtree` | scratch copy of 24d27911 | now | M1 kept branch removed, M3 forget keeps the entry, M4 cap 0: red in `inspect_by_id_matches_the_posted_body`. M5 sweep frees nothing, M6 cap `usize::MAX`, M7 keep for an id not held, M8 `kept_bytes` not reduced on forget: every suite green |
| Page mutations PM1 (server ignores kept), PM2 (`windowsKnown` line removed), window.mjs from the scratch copy | scratch copy | now | PM1 red, `posted the file 1 times`. PM2 red, `showed the fault "the window or presentation could not be applied"` |
| Debug print in scratch `keep_inspect` | scratch copy | now | each 60 KiB file was charged 123,592 bytes, so the pool took 135 of them. Pool at 16,777,048 when the multiframe's 1,032 was refused |

## Defects

### D1, the kept data set exists only for the three uncompressed syntaxes, so a compressed per-frame multiframe still posts the whole file again

**Where**: `examples/folder-viewer/src/serve.rs` `file_route` calls `bytes_without_pixel_payload(&body)` and keeps nothing when it is `None`. `examples/folder-viewer/src/preview.rs` `dataset_origin` returns `None` for every transfer syntax other than `1.2.840.10008.1.2`, `.1.2.1` and `.1.2.2`.
**What**: for an RLE, JPEG, JPEG-LS, JPEG 2000, HTJ2K or deflated file, no header is kept. Once the prefetch holds every frame the posted bytes go, `/inspect` by id answers 410, and `refreshFrameWindows` posts the whole file again, which is pass 1's D1 unchanged for those files.
**Why it is wrong**: the design's AC17 rejects a per-frame repost of the whole file. README step 6 now says the id form is answered "from the data set it keeps for each held file, without the pixels, up to 16 MiB across every file, after the posted bytes are dropped", and for these files nothing is kept at all. The ruling bounds the store by size, not by transfer syntax.
**Evidence**: `f213p2-raw.mjs disk` gave 200 for LE and `410 the file is no longer held by this process` for RLE and JPEG-LS, each with files 0 after both previews. Through the page the RLE and JPEG-LS runs each logged 1 `POST /file` on one ArrowDown.
**Scope and regression**: every encapsulated enhanced multiframe whose Frame VOI LUT is per frame, on HEAD. Enhanced multiframes are commonly stored compressed. `inspect_by_id_matches_the_posted_body` and window.mjs use explicit LE only. The smallest regression is the same id-form step on an RLE copy, which the codec registry already decodes. A fix could keep the parsed object's elements without the pixel element whatever the syntax, or keep only the window list the id form returns.

### D2, tile mode on a per-frame multiframe draws three tiles at another window than the one the corner names, and never corrects them

**Where**: `examples/folder-viewer/src/page.html` `paintTiles` asks `requestGrey` for frames `viewIndex + 1` to `+ 3`. `effectiveIndex` returns `null` for them because `windowsKnown` is false, and `refreshFrameWindows` runs only from `showIndex` for the shown image. Its completion redraws only viewports whose `currentImage` is that image.
**What**: with the second file window chosen on frame 0 and tile mode on, tiles 2 to 4 are drawn at the first window (grey 128 and 0) while the cell's corner reads `WW 1500  WL -600  B0` (the second, grey 236 and 94). They stay that way, because nothing asks those frames' lists. Before this range the tiles sent index 1 and all four showed 236 and 94.
**Why it is wrong**: README step 6 says "Until that list arrives the frame is drawn at its own first usable window, then at the chosen one". The tiles are never drawn at the chosen one. CLAUDE.md's tier rule generalises HLD section 31: a feature that cannot produce the requested result reports unavailable and never quietly produces a different result. Here a window other than the one named is shown with nothing saying so.
**Evidence**: `f213p2-page.mjs tile` at HEAD read tiles `[236,94] [128,0] [128,0] [128,0]` 1.5 s after Shift G. With only `if (!windowsKnown(image)) return null;` removed it read `[236,94]` four times. In an earlier run in which frame 1 had been visited first, tile 2 was 236 and 94 and tiles 3 and 4 were 128 and 0.
**Scope and regression**: present on HEAD and introduced by this range's D2 fix. It affects tile mode on every per-frame multiframe with a file window other than the first chosen. The prefetch takes the same path, but its "default" greys are not shown under a chosen key, so only the tiles show the wrong window. The smallest regression is one window.mjs step on a 4-frame per-frame multiframe in tile mode, asserting each tile's grey. A fix could ask each tile frame's list before or after drawing it and repaint the tiles when the list arrives, or keep tile mode off for a per-frame multiframe until the lists are known.

## Smells

### S1, the 16 MiB pool is spent on files that never ask by id, at about twice their size

**Where**: `serve.rs` `file_route` keeps a header for every posted file. `held.rs` `keep_inspect` charges `header.capacity()`, and the header `Vec` is grown by `extend_from_slice` and never shrunk.
**Mechanism**: the only id-form caller is `refreshFrameWindows`, and only for a frame other than 0 of an image with `windowsPerFrame`. Every other posted file uses the pool and gives nothing back while its frames are held, because `sweep_kept` frees only ids that hold nothing. A 62 KiB header was charged 123,592 bytes. 300 one-frame files with 60 KiB of private data, then 200 plain one-frame files, used 1,312,000 bytes of the 256 MiB cache and left the pool too full for a 1,032 byte multiframe header, which then answered 410 after its frames were held, as in pass 1's D1. A series with large private headers, scrolled before a per-frame multiframe in the same folder, reaches the same state.
**Smallest change**: keep only for files whose scan reports `windowsPerFrame`, or keep only the per-frame window lists, and charge `len` after `shrink_to_fit`.

### S2, the pool's cap, sweep and byte counter are executed by no test and reported nowhere

**Where**: `held.rs` `KEPT_INSPECT_CAP`, `sweep_kept`, `kept_bytes`. `GET /held`.
**Mechanism**: M5 (sweep frees nothing), M6 (cap `usize::MAX`), M7 (keep for an id that is not held) and M8 (`kept_bytes` not reduced on forget) all left every suite green. Under M8 the counter only rises, so after enough forgets the pool refuses every file for the life of the process and D1's repost comes back, with no test or census to see it. `/held` does not report the pool, so the F-226 checks that read `/held` after a forget cannot see it either. README line 195 says the cache budget "still bounds what the process holds" after a forget that fails, and `docs/lld/cache.md` and `use.html` describe what the folder viewer holds without this 16 MiB outside `OCELLI_CACHE_MIB`.
**Smallest change**: report `kept` bytes in `/held`, add one unit test that fills the cap and forgets, and name the pool in the LLD's account of what the viewer holds.

### S3, window.mjs swallows the precondition its "no repost" step depends on

**Where**: `examples/folder-viewer/tests/browser/window.mjs` `observeMultiframe`, the wait for `held.files === 0 && held.frames === 2` ends in `.catch(() => {})`.
**Mechanism**: if the prefetch has not dropped the posted bytes when ArrowDown is pressed, the id form answers from those bytes and `posts` is 0 whatever the kept store does. PM1 went red today, so the state is reached on this host, but a slower prefetch or a cache change makes the step pass without testing D1, and the run gives no sign of it. The step should fail, or report not observed, when the wait times out.

## Nitpicks

- N1. An id whose frames are evicted keeps its entry until the pool is under pressure, so `/inspect` by id answers 200 while `/preview` by id answers 410. The page recovers through `postPreview`'s repost. README's "for each held file" is narrower than what happens.
- N2. The prefetch asks neighbour frames under the "default" key, and once the list arrives the shown frame is asked again under its index key, so each newly visited per-frame frame is rendered twice. No wrong grey was observed in 5 ms sampling.

## Verified clean

- Pass 1 D1 closed for uncompressed files, and pass 1 D2 closed for the single view and for a linked target, as replayed above. PM1 and PM2 turn window.mjs red, and M1, M3 and M4 turn `inspect_by_id_matches_the_posted_body` red, so the new checks would fail if those fixes were reverted.
- F-226 forget releases the entry by id and by page token. Page forget answered `{"files":0,"frames":2,...}` and the id then answered 410. `forget_ids` removes the entry whether or not the id still holds anything, so an evicted id is released by an id-list forget too. A page record swept by `sweep_pages` cannot be reached by the page token, which needs more than 4096 records and is bounded by the cap.
- A data set larger than the cap is not kept, and that id answers 410 once its bytes go, as `KEPT_INSPECT_CAP`'s comment says. Observed with a 17 MiB private element.
- The pool cannot grow per id. `insert` replaces and subtracts the old entry's charge. Every add is checked against the cap.
- No new `as` cast, no new pixel arithmetic. Greys still come from `ocelli-pixel`.
- `library_reply` names dcmjs only for `/lib/dcmjs.js` and the library build for every listed module, which the extended unit test asserts. README's changed library sentence, the `link-window` sentence and step 5's "first usable one" match the tree. The catalogue and runbook text matches window.mjs, apart from S3's unasserted "fully held".

## Limitations and handoff

- Not exercised: a real corpus multiframe (synthetic only, by instruction), JPEG 2000 and HTJ2K copies (the code path is the same `dataset_origin` refusal as RLE), memory measured from the process rather than from the charge.
- External to F-213, not counted: the JPEG-LS Lossless copy written by pydicom 3.0.2 (P 16, with an LSE preset parameters marker) drew grey 255 and 255 at window `-600/1500`, where the explicit LE and RLE copies of the same pixels drew 236 and 94, and CharLS through imagecodecs 2026.10.10 decodes it to 224 and 1064. This looks like a JPEG-LS decode fault in the preset-parameter path, which could be a quietly wrong pixel. Reproduce with `scratchpad/f213p2-files/jls.dcm` and `scratchpad/f213p2-jls.mjs`. It needs its own owner.
- `bin/ocelli.sh gate --sprint` was not run, per the verify-once-at-sprint-end rule.
- Next owner: the implementer remediates D1, D2 and S1 to S3. After that, a fresh independent pass is needed on the changed material, replaying the RLE and tile triggers through the page.
