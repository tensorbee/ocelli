# F-216 review, pass 4

**Reviewed**: `work/f-216-xai` HEAD `36d79975f80d8a9dab4371294080cdf4cc9c6807` plus the uncommitted worktree, reviewed as `git diff 36d7997` (21 tracked paths including the staged new `examples/folder-viewer/src/schedule.rs`). The `sha256` of that diff was `92ebb137e1c1f6ac8aad203ddc4c6e35c8ff8e78984d1a656e39b7c2fac9cf48` at the start of the pass and again at the end. Untracked and not reviewed: `.claude/reviews/F-216-working-pass-1.md` to `-3.md` and `tools/oracle/out`. `target/` was read as a record, not reviewed. This report is the only file this pass adds to the worktree. One side effect: `cargo test` reran `recycled_grey_matches_a_fresh_decode_for_every_accepted_corpus_file`, which rewrote `target/f216-runs/corpus-identity.txt` with the same five lines. Mutations, instrumented builds and probe tests ran in an `rsync` copy under the session scratchpad (`scratchpad/p4/tree`) with `CARGO_TARGET_DIR=<wt>/target/p4/target`. Each mutated file was restored from the worktree, `diff -r` on `tools/bench/src` was confirmed clean, and `<wt>/target/p4` was deleted at the end.
**Author / reviewer**: the remediation since pass 3 is by an external coding agent and then a Claude implementer (`.claude/scratch/F-216-progress.md`, records in `target/f216-runs/`). The lead wrote the plan amendments and relayed the operator's answer. Reviewed by a Claude agent the lead invoked, which wrote none of the reviewed material and none of passes 1 to 3.
**Result**: 4 defects, 2 smells, 11 nitpicks

## Acceptance challenged

- **Free list pixel identity, through the real release server, with geometries that share a byte length.** I built the fixtures by patching tags in the server's own `/synthetic` slices. A is 96 by 128 at 16 bits. B is the same 24,576 sample bytes read as 128 by 96. C is 64 by 96 with Bits Allocated 32, Bits Stored 32 and High Bit 31. T has its Pixel Data element shortened by 4,096 bytes, so it parses and then fails in `decode_frame`, after `prepare_samples` has claimed a buffer. U is the same file cut short, and it is refused as not Part 10 before any claim. The expected value for each file and each window is the body-form preview from a separate process at 64 MiB, which never touches a free list. I then ran `OCELLI_CACHE_MIB=1` with 8 concurrent clients over randomly chosen ids drawn from A, B, C and T, and re-posted on every 410. Runs were 4,000, 4,000 and 3,000 requests, the last on the debug binary, so `memory_census`'s `debug_assert` was live. **There were 0 mismatches** in status, the six window and geometry headers and the grey hash. T answered 400 every time. Every 200 matched its own variant. An instrumented build counted 428 steals and 794 parks over one run, so the cross-geometry reuse really ran.
- **The same buffer address across geometries.** A scratch unit test on `Viewer::server` at a 60,000-byte budget ran the sequence A, T, B, T, C, A, B, C through `preview_held`'s worker path. It saw one sample address reused for A, B (transposed) and C (32-bit), and T's failed decode claimed the parked buffer and handed it back. All 8 greys equalled `preview_frame` on the same bytes.
- **1024 by 1024 against 512 by 2048, then a folder change.** At 16 MiB: 200 small files, then 12 two-mebibyte files alternating the two geometries, three rounds. 0 mismatches, and no census line had `used + free > budget`.
- **A reply reading a frame while a steal wants its length.** Pixel safety here does not come from the `strong_count` check. `reclaim_owned` takes the buffer only through `Arc::try_unwrap`, which fails while any reply still holds a clone. So a buffer cannot be recycled while it is shared, with or without the check. I mutated the check to `if false && ...`. All 26 lib tests stayed green (S1). An instrumented run showed that the budget test reaches the held branch more than 30 times without asserting anything about it.
- **The corpus identity test with the poison fill**, run now on the worktree. Its output was `files 86`, `frames 88`, `differences 0`, `reused 90`, `poison 0xa5`. I also removed the zero fill in `prepare_samples` (mutation M2), and the test stayed green. Every accepted decoder overwrites every sample byte, so the poison test detects a decoder that skips bytes. It does not detect the zero fill (N11).
- **The scheduler under a client that stops reading.** One client asks for the 4 MiB grey of a 2048 by 2048 frame and never reads it. Six small previews then start, and three of them block until that socket closes: 3.01 s in one run and 70.01 s in another. The other worker sat idle the whole time (D1).
- **Body-form previews and the "two threads" claim.** At `OCELLI_CACHE_MIB=0`, `/file` answers `X-Ocelli-Held: 0`, and the page then sends the whole file to `/preview`. 400 such previews from 8 clients gave the census line `decode_threads 400`, `peak_decodes 3` (D2).

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo fmt --all --check` | worktree, `CARGO_TARGET_DIR=<wt>/target` | now | exit 0 |
| `cargo clippy -p ocelli-folder-viewer -p ocelli-pixel --all-targets -- -D warnings` | same | now | exit 0 |
| `cargo test -p ocelli-folder-viewer -p ocelli-pixel` | same | now | exit 0. viewer lib 26, main 0, preview 59, pixel lib 14, color 15, image_plane 4, lut_chain 18, modality 5, palette 7, stored_pixel 2, voi 6 |
| `bin/ocelli.sh gate licences` | same | now | exit 0, ALL GREEN |
| `bin/ocelli.sh gate bench` | same | now | exit 0, ALL GREEN. `node --test` in `tools/bench`: 146 tests, 144 pass, 0 fail, 2 skipped |
| `bin/ocelli.sh gate guards` | same | now | exit 0, ALL GREEN |
| `python3 scripts/guard_census.py`, then `--check-runbook` | same | now | exit 0 and exit 0, `0 watched by nothing` |
| `python3 scripts/prose_check.py` | same | now | exit 0, 840 files |
| `python3 scripts/source_provenance_check.py` | same | now | exit 0, 1674 files |
| `viewer.resident_memory` x3, `--no-build`, release, one process | `target/release/ocelli-folder-viewer` built 19:26, no source newer | now, loads 4.70 to 8.44, 8.44 to 12.33, 12.33 to 11.21 | **441,532,416**, **443,793,408**, **467,894,272** against 528,194,192, all `thresholds_met: true`, `profile: release`. Retained heap 98,123,859, 98,123,787 and 98,123,651, each agreed at collection 3, against 118,686,080. Before gc 125.01 to 125.03 MB. `held_peaks.free` was **0** in all three. Ending `/held` was `used 250948688, files 441, frames 5, inflight 0, workspace 17400000, free 0` |
| `viewer.open` | same | now, load 10.03 | 973 ms, met, release |
| `viewer.first_image` | same | now, load 9.64 | CT 8.3 ms, DX median 29.3 ms, met |
| `viewer.scroll_rate` (release, plus its debug half) | same | now, load 7.88 | 109.03 fps, 1,802 keys, `blank_samples 0`, `debug_blank_samples 0`, met |
| `viewer.window_step` | same | now, load 7.88 | CT 1.9 ms, DX 18.8 ms, `preview_bodies 0`, met |
| implementer's ten runs, `target/f216-runs/{quiet,load}-*/run.json` | records | prior | 417,054,720 to 456,949,760, all met, all release. Matches the progress note. The "quiet" runs started at load 22 to 28 (N10) |
| `fl.py 1 <seed> <files> <requests>`, A, B, C, T, 8 clients | release, `OCELLI_CACHE_MIB=1`, census log | now | seed 1: 0 bad over 4,000. seed 2: 0 bad over 4,000, max `used + free` 1,047,342 of 1,048,576, max free 24,576, decode threads 2 |
| same on the debug binary (`debug_assert` live) | `target/debug/ocelli-folder-viewer`, current | now | 0 bad over 3,000, 272 census lines, 0 over budget, no panic |
| same on an instrumented scratch debug build | scratch | now | `parked 794`, `stolen 428`, `dropped 157`, `park-refused 9` (D3) |
| `p4_server_reuse_across_geometry` (scratch unit test) | scratch, `Viewer::server`, 60,000 B | now | one address for A, B and C, T reused and returned it, 8 of 8 greys equal to `preview_frame` |
| `folder.py`, 1024x1024 and 512x2048 after 200 small files, 3 rounds | release, 16 MiB | now | 0 bad, 0 census lines over budget |
| `zero.py`, 400 body-form previews, 8 clients | release, `OCELLI_CACHE_MIB=0` | now | 400 of 200 status. Census `decode_threads 400`, `peak_decodes 3` (D2) |
| `slow2.py 3` and `slow2.py 70`, one stalled reader of a 4 MiB grey | release, 256 MiB | now | 3 of 6 small previews returned at 3.01 s and 70.01 s, the moment the stalled socket closed. The other 3 took at most 0.12 s (D1) |
| `slow3.py 130`, two stalled readers | same | now | 2 of 6 small previews still unanswered at the client's 90 s timeout, with `WRITE_TIMEOUT` at 60 s (D1) |

Mutations, each in the scratch copy, run and then restored:

1. M1, `take_oldest_frame_buffer`'s `strong_count > 1` branch disabled: all 26 lib tests **green** (S1).
2. M2, `prepare_samples` keeps old bytes when the length already matches: all lib tests, the corpus poison test included, **green** (N11).
3. M3, `park_if_fits` without the fit check: `cached_frames_and_free_buffers_share_one_budget` red, through `memory_census`'s `debug_assert`.
4. M4, `make_room_dropping` removed after a free-list pop: **green**. The pop has already taken the buffer's bytes out of `free`, so the call covers only `size_of::<HeldFrame>()` (N12).
5. `discard_busy_body` call removed, 3 runs: `a_busy_post_with_a_body_is_still_readable` red 3 of 3, `the 503 was reset: Connection reset by peer`. Pass 3 S1 is closed.
6. `ensureViewerBinary` picking debug when `build === false`: `a recorded release subject builds the release binary` red. Pass 3 S2 is closed.
7. `assertPageHooks`'s no-argument `requestGrey` lookup replaced by a function, which is pass 3's mutation 6: **green** (S2).
8. `foldHeapReadings` with the tolerance multiplied by 1000: 2 tests red. Returning the first sample instead of the agreeing one: 1 red. Pass 3 D1 is closed in behaviour.

Environment: Apple M4 Max, macOS 27.0.0 kernel, Node 24.13.0, Python 3.14.5. Load average ran from 4.7 to 23, with 9 users and the operator's debug viewer, PID 98891, on 8742. **The timings are indicative and are not a baseline.** No baseline was written. Every viewer I started ran with `OCELLI_PORT=0` and was stopped by `Popen.kill` on its own PID or by the bench runner. Ports 8742 and 8765 were never used, and PID 98891 was not touched. At the end, `ps` showed only PID 98891.

## Pass 3 findings replayed

| Pass 3 | Status | Replay and adjacent case |
|--------|--------|--------------------------|
| D1 heap readings | **closed** | The readings must agree within 64 KiB, capped at 8 collections. All three runs agreed at collection 3. Mutation 8 is red both ways. The 64 KiB is not in the amendment (N9) |
| D2 counter comments, double count | **closed for `/file`, open for body-form previews** | A body is now charged from allocation, and it is released before `insert_posted`. The `inflight` comment is still false for a body-form preview's sample buffer, which is never counted (D2) |
| D3 plan `/held` row | **open, changed** | The amendment names `pool`, which is gone, and omits `free`, which is reported (D4) |
| S1 drain untested | **closed** | Mutation 5 red 3 of 3 |
| S2 D4 regression path | **closed** | Mutation 6 red. `detail.profile` now comes from `/held` |
| S3 page-global branch | **narrowed, still open** | Mutation 7 green (S2) |
| S4 mimalloc version in prose | **closed** | The three documents name the allocator and the exact pin, with no version |
| S5 pool tweaks | **closed** | Pool and sample-class reservation removed. No `pool` text is left in `cache.md`, README or `use.html` |
| N1 to N6, N8 | carried | See Nitpicks. N7 is closed because the "spare posted buffers" text is gone |

## Defects

### D1, a client that stops reading holds a decode permit without bound, and previews homed to that worker wait while the other worker is idle

**Where**: `examples/folder-viewer/src/serve.rs:596` (the permit is returned with the reply), `:330-332` (the permit is released only after `write_reply`), `examples/folder-viewer/src/schedule.rs:152` and `:164`
**What**: `preview_held_on_worker` keeps its `DecodePermit` until `restore_grey` runs, and that runs after the whole reply is written to the socket. `acquire_decode(Some(home))` accepts only the home worker's slot, and never falls back. Its doc comment says "Prefer `home`", but the code requires it. So one client that sends a held preview of a large frame and never reads the reply blocks every preview whose bytes or frame live on that worker. That lasts until the client closes, well past the 60 s `WRITE_TIMEOUT`, and the other worker stays idle the whole time. With one stalled reader, 3 of 6 unrelated previews returned at 3.01 s and at 70.01 s, the moment the stalled socket closed. With two, 2 of 6 were still unanswered at 90 s. Before this change, the direct path, which is still `preview_held`'s non-scheduled branch, dropped its `WorkspaceGuard` before writing and recycled the grey afterwards.
**Why it is wrong**: the plan fixes two decode permits (`F-216-design.md:182`) and uses a permit for the render's scratch (`:188`), not for a network write. Its wait rule is "a thread waits for a permit only while that client is still there". Here the wait is caused by a different client, and it has no bound. Unrelated requests on a free worker are refused service, which is wrong behaviour, not a slow path.
**Evidence**: `slow2.py 3`, `slow2.py 70` and `slow3.py 130` in the evidence table.
**Scope and regression**: every held preview served by the process server. The page's Chromium client reads promptly, so the bench subjects do not see it. Any local HTTP client that stalls, and any very slow reader of an 8.7 MB DX grey, does. Smallest fix: return the permit before the write, letting the grey go back to the worker by message without the permit, and make `home` a preference with a fallback after a bounded wait, or state why it must be strict. Smallest regression: a serve test with a client that does not read a large reply, then a second held preview homed to the same worker, which must answer within a stated bound.

### D2, "decodes on two threads" and "a request body is freed on the thread that allocated it" are false for body-form previews

**Where**: `README.md:185`, `examples/folder-viewer/src/use.html:270`, `docs/lld/cache.md:168-173`, `examples/folder-viewer/src/schedule.rs:1-7` ("Two long-lived workers allocate every request body and every decoded frame"), `examples/folder-viewer/src/held.rs:448` ("Bodies and decodes run on `P` workers"), `held.rs:384` (`inflight`, "decode sample bytes not yet in the cache"). Code: `serve.rs:445` and `serve.rs:348-362`
**What**: a `/preview` that carries the file runs `preview_frame` on the connection thread. It takes no decode permit and charges no sample bytes to `inflight`, because `preview_frame` passes a no-op `on_samples`. The page sends that form whenever `/file` answers `X-Ocelli-Held: 0`: a budget of 0, or a file larger than the budget. At `OCELLI_CACHE_MIB=0`, 400 previews from 8 clients gave `decode_threads 400` and `peak_decodes 3` in the census. The concurrency is bounded only by the four body slots, not by `P`. Separately, a body that a worker allocated for `/preview`, `/headers` or `/inspect` is dropped when `route`'s `request` goes out of scope, which is on the connection thread. Only `/file` sends its body back to the worker.
**Why it is wrong**: microscope section 3. README and `use.html` are always in scope, and these sentences state an invariant the tree does not keep. The operator's answer after pass 3 asks the documents to say that a request body is freed on the thread that allocated it.
**Evidence**: `zero.py` above, and the `route` code path.
**Scope and regression**: the page's fallback path and every non-page client of the body forms. The resident bound's `P * (2 * S_max + 5 * N_max)` term assumes at most two decodes, and this path allows four. Smallest fix: run the body-form decode under a decode permit on a worker, and return the body to its worker, or narrow every sentence to the held path. Smallest regression: a census assertion that a body-form preview on `Viewer::server` raises neither `decode_threads` above 2 nor `peak_decodes` above 2.

### D3, the free-list sentences describe a buffer that always waits on the list, and a steal that skips held frames

**Where**: `README.md:185`, `examples/folder-viewer/src/use.html:270`, `docs/lld/cache.md:173-182`, `examples/folder-viewer/src/held.rs:5-6` (module doc) and `:214-215` (`CachedFrame`)
**What**: all five say that a decoded frame's sample buffer that leaves the cache waits on the free list. `CachedFrame`'s comment adds "instead of being freed". The code frees it in four places: `make_room_dropping` (`held.rs:1074`), `keep_buffer_if_room` when the buffer and the incoming entry do not both fit (`:1133`), `park_if_fits` when it does not fit (`:1059`), and `discard_held` on a refused reinsert. The tree's own `the_held_bytes_never_exceed_the_budget` requires `census.free == 0` after an eviction (`held.rs:1520`). An instrumented run counted 157 drops and 9 refused parks against 794 parks. `held_peaks.free` was 0 through all three resident runs, so in the workload the bound is measured on, reuse comes from the steal path and not from the list. `cache.md:180` also says a decode takes "the least recently used cached frame of that length that no reply is still reading". The code looks only at the least recently used frame of that length (`take_oldest_frame_buffer`, `held.rs:1174-1184`). If a reply holds that frame, it steals nothing, evicts with `make_room_dropping` and allocates fresh, even when an unheld frame of that length is next in line.
**Why it is wrong**: microscope section 3. A false sentence in README, `use.html` or the LLD is a defect.
**Evidence**: the counts above, the cited test, and the code path.
**Scope and regression**: the three user documents and the two doc comments. Smallest fix: say that the buffer is parked when it fits inside the budget and freed otherwise, and that the steal takes the least recently used frame of that length only when no reply holds it. Alternatively, make the steal walk on to the next frame of that length.

### D4, the plan's `/held` contract names a field the tree no longer has and omits one it reports

**Where**: `.claude/plans/F-216-design.md:110` and the first bullet of "Lead amendments after review pass 3"
**What**: the amendment says `/held` "also reports `pool`, `inflight` and `workspace`". The second bullet of the same section removes the pool, and the tree's reply (`serve.rs` `held_reply`) is `build, budget, used, files, frames, inflight, workspace, free`. `free` is the free-list byte count the operator's answer introduced. The runner records it as `held_peaks.free`, and `cache.md:182` documents it. No amendment names it.
**Why it is wrong**: the plan is the design contract for this route, and its row is false of the tree. This is pass 3's D3 in a new form, the second pass in which the row has not matched the reply.
**Evidence**: `/held` replies in every run above.
**Scope and regression**: the runner and any later consumer of the contract (F-210 and F-211 build on this protocol). Smallest fix: one lead amendment that lists the reply exactly, names `free` and drops `pool`.

## Smells

### S1, the steal's "no reply is reading" check runs and nothing tests its effect

**Where**: `examples/folder-viewer/src/held.rs:1184-1203`, test `cached_frames_and_free_buffers_share_one_budget`
**Failure mechanism**: the budget test reaches the branch more than 30 times (instrumented run), and disabling it (M1) leaves all 26 lib tests green. Pixel identity does not depend on it, because `reclaim_owned` cannot take a shared buffer. What it protects is retention. Without it, the frame a reply is reading right now is removed from the cache. For a single-frame object, whose bytes left when the frame was stored, the next preview of the image on screen then answers 410 and the page re-posts the file. The next change to the steal can drop the branch with the suite green.
**Smallest fix**: a unit case that holds the `Arc` from `preview_source` on the only frame of length L, claims L at a full budget, and requires that frame to be still cached and the claim to be fresh.

### S2, the page-globals refusal test still passes with either lookup replaced (pass 3 S3, narrowed)

**Where**: `tools/bench/tests/viewer_work_test.mjs:181-197`, `tools/bench/src/viewer_work.mjs:43-49`
**Failure mechanism**: the test deletes both globals at once, so when one lookup is replaced by a function that always exists, the other still throws. Mutation 7, which is pass 3's mutation 6 replayed, is green. Under that mutation, a page that lost only `requestGrey` passes the instrument's check.
**Smallest fix**: two refusal cases, each with exactly one hook missing.

## Nitpicks

- **N1** (carried) the lead amendment paragraph after the `viewer.scroll_rate` row still sits inside the subject table area of the plan (`F-216-design.md:328`).
- **N2** (carried) `held.rs:267`: a frame is charged its sample capacity plus `size_of::<HeldFrame>()` only.
- **N3** (carried) `preview.rs` `a_deflated_syntax_is_not_walked` substitutes RLE `.1.2.5` for deflated.
- **N4** (carried, pre-existing, not re-run) an out-of-range frame on a held single-frame id answers 410.
- **N5** (carried) `leave_inflight` and `release_count` saturate, which would hide an unpaired release.
- **N6** (carried) `held_peaks` is read on every tenth RSS tick and is a sampled maximum. `held_at_peak` is better and is read only when RSS rises.
- **N8** (carried) `/headers` after a decoded single-frame preview answers 410. The plan sanctions it, but the README does not mention it.
- **N9** `HEAP_AGREE_BYTES` (64 KiB) defines "agree", and the amendment says only "until two readings agree". The readings agree within about 1,140 bytes, so no verdict depends on it. State the tolerance in the amendment.
- **N10** the operator's fallback asks for five quiet runs and five beside a load above 10. The implementer's "quiet" runs started at load 22 to 28 (`target/f216-runs/quiet-*/uptime-start.txt`). My three runs started at 4.7, 8.4 and 12.3 and peaked at 441.5 to 467.9 MB, which covers part of the quiet half.
- **N11** the poison test detects a decoder that skips a byte, not the zero fill (M2 green). The zero fill is defence in depth, and the comments describe it correctly.
- **N12** `make_room_dropping` right after a free-list pop covers only `size_of::<HeldFrame>()` (M4 green).
- **N13** `drop_one_free` takes an arbitrary `HashMap` key, so after a folder change it can drop a buffer of the length in use before a stale one. This costs performance only, and the budget held in `folder.py`.

## Verified clean

- **Pixel correctness of the free list** in every shape the request named: a decode that failed after its claim (T) followed by previews of the same id and of other files, identical byte length with different rows and columns (128x96 against 96x128, 1024x1024 against 512x2048), 16-bit against 32-bit at one length, and 8-way concurrency with replies holding frames during steals. 0 mismatches over about 11,000 requests and in the scratch unit test, plus the corpus poison test at 0 differences. `prepare_samples` clears and zero-fills a claimed buffer. Its length key equals the decode's `output_len` and its capacity is at least that. A shared frame's buffer cannot be taken, because the only exit is `Arc::try_unwrap`. A frame dropped by a late reply reaches the list only through `drain_recycled`, under the lock, and through the fit check.
- **Budget**: `used + free <= budget` held on every census line in all runs, through steal, eviction under concurrent decodes and a folder change, and the debug binary's `debug_assert` never fired. M3 is red. Every mutation of `free` and `lru` happens under the one cache lock, and `reserve_for_insert` drops free buffers before it evicts.
- **Scheduler**, apart from D1: two threads, registered before `start` returns (the barrier). Four body slots, mapped two per worker. FIFO jobs. No `call_worker` from inside a worker job, so no self-wait. Shutdown sets `shut`, wakes both condvars and joins, and a job sent after a worker exits returns `None` and becomes a 503. A slow body holds a body slot on the connection thread, not a worker, so decodes do not wait on it. The 503 path keeps its drain (mutation 5 red).
- **Memory**: the three resident runs and the four other release subjects above, all met, all `profile: release`. The implementer's ten records were read and match the note.
- **Prose executed**: "at most four request bodies" (true of `BODY_SLOTS` for the server), "keyed by its exact byte length", "the next decode of that length takes the buffer", "its old bytes are overwritten before they are read", "`GET /held` reports `free`", "a decode copy is in flight until the frame is in the cache", the mimalloc sentences and the exact pin in `Cargo.toml`, and `serverRssBound`'s comment. The false ones are D2 and D3.
- No `as` cast, `#[allow]` or `unsafe` was added in the reviewed Rust. `cargo fmt`, `clippy -D warnings`, the tests and the seven gates and checks were all green.

## Limitations and handoff

- Not exercised: a non-loopback slow network, a big-endian host, Enhanced multi-frame files beyond the corpus in the reuse path through the real server, a stalled reader through Chromium (whose fetch reads promptly), and the 503 storm on the page. The corpus identity test drives `Viewer::new`'s direct path, not `Viewer::server`'s worker path. The worker path's identity rests on my black-box runs with synthetic fixtures and the scratch unit test.
- D1's block was measured to 90 s and was not followed to whatever eventually ends it.
- Clean scope: the bounded review above. It is not acceptance and not sprint readiness. AC17 and AC18 measured met on this tree in three resident runs and one run of each other subject, at loads up to about 12.
- Next owner: implementation, for D1, D2, D3, S1 and S2. The lead, for D4's amendment, and for whether D2 is fixed in code or by narrowing the claim. A fresh independent pass is required after remediation. It should replay the stalled-reader probe, `zero.py`, M1 and mutation 7.
