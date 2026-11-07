# F-226 review, pass 5

**Reviewed**: range `d9c2d477..8f55b899` on `work/f-226-xai`, the folder viewer half. HEAD `8f55b899`, `git write-tree` `94d9cb78`, matching HEAD's tree. No unstaged tracked paths. Untracked at start and end: `.venv` and `tools/oracle/out`, neither touched.
**Author / reviewer**: the folder viewer rounds are a Claude implementer's. This pass is an independent Claude reviewer that wrote none of the reviewed material. The reviewer only added this report and scratch files under ignored `target/`.
**Result**: 2 defects, 1 smell, 5 nitpicks

## Acceptance challenged

- **Cache consistency, derived independently.** After every racing thread joins, these must all hold. `files` and `frames` equal the live `HeldKey::File` and `HeldKey::Frame` keys. `frame_order` holds each live frame key exactly once and nothing else. `free_bytes` equals the parked capacities. `used + free <= limit`. `in_flight` and `doomed` are empty. No id that a forget has returned for holds anything afterwards, whatever posts, decodes, stores and evictions were running. After `forget_page` for every token, the cache is empty, the page record is empty, and the summed replies equal the census taken before. I wrote `review5_forget_is_final_under_races` in a scratch copy to check this. One thread posts 3,000 synthetic slices under two tokens into a budget of about six frames, so evictions run all the time. Four threads take `preview_source`, keep the flight, decode, `store_frame` and drop the flight. One thread forgets random subsets with repeats in them. 2,873 to 2,902 decodes got shared bytes per run, and 920 ids were forgotten. It passed 3 of 3 runs, and it goes red under mutations M5, M6, M9 and M13 below. So it does discriminate.
- **AC10 to AC12**: each named wrong implementation was mutated, plus some the plan does not name. See the evidence table. AC10's fixture misses one, see D2.
- **AC12 on the production path.** `Viewer::server` always schedules, so production previews go through `preview_held_on_worker`. `a_forget_during_a_pooled_decode_is_not_undone_by_its_store` covers that path, and M2 (flight dropped at once there) turns it red. I also derived one case of my own: a forget during a decode whose render then waits for grey room. That request answers 410, not its frame. See D1.
- **AC13 to AC15 and the ensureFile guard**: every page mutation turns `forget.mjs` red for the right reason. That includes removing the `ensureFile` guard (P1), which I had suspected the abort-on-open controllers might mask.
- **Page token**: I checked the format, that the token changes across reloads, that a real reload and a real navigation away both empty the census, and Chromium's keepalive quota with the bodies the LLD quotes.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo test -p ocelli-folder-viewer` | worktree, HEAD `8f55b899`, default target | now | 0, every binary ok |
| `bin/ocelli.sh gate viewer-browser` | same, release build | now | ALL GREEN. aspect 9 canvases, readout 5 changes, link reference line, forget: `left` 0 files 3 frames and A 410 x4, `late` 410 and 0 posts, `hide` one request of 37 bytes then 3 held, `leave` 0 held |
| `prose_check.py`, `source_provenance_check.py` | worktree | now | 0 over 910 files, 0 over 1778 files |
| `guard_census.py`, `--check-runbook` | worktree | now | "0 watched by nothing", runbook exit 0 |
| `guard_probe.py --only viewer-browser.forget-left-folder` and `--only viewer-browser.forget-late-post` | worktree | now | both OK, the probe goes red for its declared reason and the control is green |
| `bin/ocelli.sh gate --list` | worktree | now | viewer-browser row names `forget.mjs` and F-226 |
| Rust mutations, `cargo test -p ocelli-folder-viewer --no-fail-fast` | `git archive HEAD` into `target/f226-review5-mut/src`, own `CARGO_TARGET_DIR` `target/f226-review5-mut/target`. Each mutation applied alone and the file restored and compared after | now | see the mutation table |
| Browser mutations of `page.html`, `node examples/folder-viewer/tests/browser/forget.mjs` | same copy, release build in its own target, P0 an unmutated control | now, one browser job at a time | P0 exit 0. P1 to P4 exit 1 |
| `review5-beacon.mjs` (Playwright, synthetic study only) | worktree release binary at HEAD, port 0 | now | tokens `^[0-9a-f]{32}$`, two loads gave two different tokens. Census 8 files 4 frames, then 0 and 0 after `page.reload()`, then 0 and 0 after `goto("about:blank")`. `sendBeacon` 35,699 bytes, 35,699 bytes, 37 bytes returned true, false, true |
| `review5_forget_during_a_decode_that_then_waits_for_grey` | scratch `serve.rs` test: grey cap filled by two stalls, both permits held, preview sent, forget sent, permits released, stalls dropped | now | without the forget `200 OK`. With it, the forget answers `{"files":1,"frames":0,"bytes":524758,"unknown":0}` and the preview `410 Gone` |
| use.html section 12 sample | compiled as `crates/ocelli-volume/tests/review5_use_sample.rs` in the scratch copy | now | compiles and passes, with only unused-import warnings |

Mutation table, each against the unmodified copy:

| ID | Mutation | Red tests |
|----|----------|-----------|
| M1 | `store_frame_with` ignores `doomed` | held AC12, pooled AC12 |
| M2 | pooled path: `let _ = found.take_flight()` | pooled AC12 |
| M3 | `decode_for_value`: `let _ = bytes.take_flight()` | **none**, exit 0 |
| M4 | direct `preview_held`: `let _ = bytes.take_flight()` | **none**, exit 0 |
| M5 | `forget_ids` skips `forget_frame` | `a_forgotten_file_leaves_the_cache_as_if_never_held`, review stress |
| M6 | `forget_ids` skips the `frames` decrement | as M5, plus AC10 and the page forget test |
| M7 | frame range `Frame(id, 0)..=Frame(id, 0)` | **none**, exit 0 |
| M8 | `forget_page` ignores the token | `forget_by_page_drops_only_that_pages_files` |
| M9 | `land` never clears `doomed` | held AC12, review stress |
| M10 | `sweep_pages` never sweeps | `page_records_of_evicted_files_are_swept` |
| M11 | sweep keeps only ids whose bytes are held | **none**, exit 0 |
| M12 | forget never dooms an in-flight id | held AC12, pooled AC12 |
| M13 | an id forget leaves its page record | page forget test, review stress |
| P1 | `ensureFile` guard removed | `late`: "A's file was posted again 1 times" |
| P2 | `pagehide` sends one beacon per id | `hide`: 3,400 requests, 2,645 still held |
| P3 | `pagehide` sends nothing | `hide` 3,403 held, `leave` 3 held |
| P4 | an open does not forget the previous ids | `left` 1 file 6 frames and A 200, `late` 200 |

## Defects

### D1, the LLD says a forgotten in-flight decode is still served, and on the grey-wait path it answers 410

**Where**: `docs/lld/cache.md`, the paragraph opening "A forget wins against a decode already in flight", the words "returns the frame to its own request, which is still served". `README.md`'s new paragraph says "A decode already running for a forgotten file answers its own request and stores nothing", which reads the same way.
**What**: in `preview_held_on_worker`, a decode whose render meets a full grey cap comes back as `PooledOutcome::Congested`. Its `store_frame_with` is then skipped for the doomed id, and the request waits for room and `continue`s. The second round calls `preview_source`, finds the file forgotten, and returns `gone()`. So the request gets a 410, not the frame.
**Why it is wrong**: a false claim in prose, under microscope section 3. The behaviour is defensible. F-216's `a_frame_gone_after_the_grey_wait_releases_its_room` already answers 410 for an id forgotten during the wait. The sentence is the false part. The plan's state table row ("the decode returns its reply") holds only on the paths without a grey wait.
**Evidence**: `review5_forget_during_a_decode_that_then_waits_for_grey` in the scratch copy. Without the forget the reply is `HTTP/1.1 200 OK`. With the forget it is `HTTP/1.1 410 Gone`.
**Scope and regression**: present on HEAD, production path only when the 17,400,000-byte grey cap is full. It does no user harm, because the page does not post a forgotten open's file again. Smallest fix: say the decode stores nothing and its request is answered, with 410 when the render had to wait for grey room, or delete "which is still served". Adjust the README sentence to match.

### D2, AC10 passes when a forget removes only frame 0

**Where**: `examples/folder-viewer/tests/preview.rs` `forget_drops_files_and_frames`, and `held.rs` `forget_ids`'s `remove_range`.
**What**: the only frames any forget test holds are frame 0. M7 limits the range to `Frame(id, 0)..=Frame(id, 0)`, so frames 1 and up of a multi-frame object survive the forget, and the whole suite still passes, exit 0.
**Why it is wrong**: decision 5 says the route removes "the range `HeldKey::Frame(id, 0)..=HeldKey::Frame(id, u32::MAX)`", and AC10 names "Removing the file but not its frames" as the implementation it rejects. A test that passes under a forget that keeps frames is the "would this test fail" defect of microscope section 2. The review stress test uses one-frame slices and does not catch it either.
**Evidence**: M7 in the mutation table.
**Scope and regression**: the code on HEAD is correct. The cover is missing. Smallest regression: in AC10, preview frame 1 of `two` as well as frame 0, so the reply expects two frames and the hand-summed bytes include both charges. Confirm M7 goes red.

## Smells

### S1, the flights on `/value`, `/measure` and the direct preview path are not executed by any forget test

**Where**: `serve.rs` `decode_for_value` (`let _flight = bytes.take_flight();`) and the `PreviewSource::Shared` arm of `preview_held`.
**What**: dropping either flight at once (M3, M4) leaves the whole suite green. The `/value` and `/measure` path runs in production, because the readout and measurements call it on a held file that is not yet decoded. If that flight regressed, a forget during a `/value` decode would be undone by its store, and the forgotten id's frame would sit in the cache until eviction. `docs/lld/cache.md` names "the direct path, the decode workers and `/value` and `/measure`" as kept until after the store, and only the decode workers are tested.
**Maintenance hazard**: the flight is a line any refactor of `decode_for_value` can move or drop, and nothing goes red. The held.rs AC12 test copies the direct route's steps by hand rather than calling `preview_held`, so it does not cover the route either. Tests for one entry point do not prove another.
**Smallest regression**: a `/value` variant of `a_forget_during_a_pooled_decode_is_not_undone_by_its_store`. Hold both permits, send `/value` for a posted id, forget it, release the permits, then assert 0 files, 0 frames and 0 used. Confirm M3 goes red.

## Nitpicks

- **N1.** M11 survives. No test covers the sweep keeping an id whose posted bytes have left but whose frames are held, which is the `framed` half of the `retain`. If it regressed, a page forget after a sweep would miss fully decoded files.
- **N2.** `forget.mjs` `hide` compares the census after the beacon to the constant `B_FILES` (`other: B_FILES`), not to the second page's files and frames measured before the beacon. The catalogue's `refuses` text says "does not count exactly the second page's files and frames". It works because each B file counts as exactly one entry after its draw, but the check does not measure the other page.
- **N3.** The `hide:` console line joins every beacon size, so under a regression like P2 it prints 3,400 numbers on one line. Printing the count and the largest size would be enough.
- **N4.** README's new paragraph ends "the 256 MiB budget still bounds what the process holds", but `OCELLI_CACHE_MIB` sets that budget and 256 is only the default. "the cache budget" would be exact.
- **N5.** Not a finding against the plan. `/forget` by id accepts any id, and ids are `<generation>.<serial>` with sequential serials, so one page can forget another page's files by id. Decision 5 accepts this and the lead's ruling limits only the token. No prose claims otherwise, since the README's "no other page's" is about the beacon, which is true. I record it so the boundary stays visible.

## Verified clean

- **`Viewer::forget`, `forget_page`, `CacheState::forget_ids`**. One path for both routes, under one lock. Files go through `Lru::remove` with `files` decremented. Frames go through `remove_range`, `forget_frame`, the `frames` decrement and `reclaim_held`, which is the bookkeeping `absorb` uses for eviction. Posted bytes are dropped after the lock is released. A repeat, an unknown id and another generation's id count as `unknown`. The review stress test, M5, M6 and M13 back this.
- **In-flight and doomed**. The count is taken under the cache lock in `preview_source`, only for `Shared`. `Hit` takes none and stores nothing. On the pooled path the flight is taken on the request thread before the `PreviewSource` moves into the job, so `pooled_preview` drops `SharedBytes` with no flight, and the flight lives across both `store_frame_with` calls. A `Flight` is never dropped under the cache lock: `forget_ids` returns only `Held::File` values, and no non-test site drops a flight-bearing `SharedBytes` while holding the lock. `land` clears `doomed` only at zero, so both sets are bounded by live decodes.
- **Races**: forget then store (doomed), store then forget (the frame is removed), forget during eviction (serialised by the mutex, and a frame a reply is reading is removed and its buffer comes back through the recycle channel as on eviction), and a decode taking the bytes after a forget (`Gone`).
- **Page token**. `PageToken::parse` takes exactly 32 lowercase hex digits. A malformed `X-Ocelli-Page` on `/file` is `400 header`. A `page` body with a short, long or uppercase token, with a trailing id, or with no space is `400 forget` and leaves the census alone. An unknown token gives four zeros. A repeat gives four zeros. Another page's file survives. The token comes from `crypto.getRandomValues` over 16 bytes, once per page load, and is never sent anywhere but this origin's `/file` and `/forget`. The 37-byte beacon arrives on a real `reload` and on a real navigation away, measured from the process side. A beacon of 35,699 bytes after another is refused while 37 bytes is accepted, which matches the LLD's measurement.
- **`ensureFile` guard at every 410 re-post site**: `postPreview`, `postHeaders`, `fetchValue` and `askMeasure` each `filePosts.delete` and then go through `ensureFile`, which now refuses a file outside `openFiles`. Every caller catches the `AbortError`. P1 shows the guard is what stops the late post. `postFile` and `readFolder` forget at once an id that arrives after another open began.
- **Route limits**: 65,536 lines are accepted and 65,537 refused before anything is removed. A declared length of 4 MiB plus one is refused before the body is read. `FORGET_LINES * FORGET_LINE_MAX <= FORGET_BODY_CAP` is a const assert. An `X-Ocelli-File` header is `400 header`. The reply is `application/json; charset=utf-8`.
- **Prose executed**: README's new paragraph apart from D1 and N4. `use.html` section 12's sample compiles, and its sentences match `release_series`, `release_volume` and the folder viewer's forget by id. `more.html` `#worker`: both new sentences match the page. `docs/lld/cache.md`'s four new paragraphs apart from D1. The sweep threshold sentence matches `max(4096, 2 * held)`. "contiguous because `HeldKey` orders by id before frame" holds under the derived `Ord`, where `Frame(id, n)` orders by id first. `forget_file` is gone, and its test now calls `viewer.forget`. Runbook rows 582 and 583 and the renumbered 584 to 586 pass `--check-runbook`. The catalogue entry's probes run green. The `RingEvent::kind` doc now names kind 6's owner, which is true.
- **No regression in F-208, F-209, F-210, F-211 or F-216**: the folder viewer suite passes, and the viewer-browser gate's aspect, readout and link checks pass beside forget.
- No `unsafe`, no `as` cast, no `wasm-bindgen` and no pixel arithmetic in the diff.

## Limitations and handoff

- Not exercised: AC16, the bench rerun at sprint verification on the F-216 host. Not exercised either: bfcache restore, which headless Playwright does not give, and a real forget racing a `/value` decode, which is S1's missing test. No GPU or oracle job ran.
- Scratch: `target/f226-review5-mut`, which holds the archive copy, the scratch tests, `mut.py`, `batch1.py`, `batch1b.py`, `batch2.py` and `review5-beacon.mjs`. It is deleted with Python `shutil` after this report. The implementer's `target/f226-mut-8f55b899`, `target/f226-debug.mjs` and `target/f226-debug2.mjs` are still in ignored `target/`, and I did not touch them.
- This pass is blocking: D1, D2 and S1 need remediation, then a fresh independent pass on the changed material. It is a bounded review of the folder viewer half. It is not completed acceptance or sprint readiness.
