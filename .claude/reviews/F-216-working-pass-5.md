# F-216 review, pass 5

**Reviewed**: `work/f-216-xai` at `ae7c9c9ed8e8915efbaa335be1d2efbf5d35a403` (the pass 4 reviewed state) plus the staged, uncommitted round, reviewed as `git diff ae7c9c9`. That covers 9 paths: README.md, docs/lld/cache.md, held.rs, schedule.rs, serve.rs, use.html, the new tests/scheduled_decodes.rs and tests/stalled_reader.rs, and tools/bench/tests/viewer_work_test.mjs. The diff's `sha256` was `28d6de9f57839888ef16935baff8eed5ae7ac0be78fb67a7c371d9f65ac2f556` at the start of the pass and again at the end. `git write-tree` gave `97b3aecd29dcc62f2428bbf204cec3b73b0c6937`. Not reviewed: the untracked `.claude/reviews/` passes 1 to 4 and `tools/oracle/out`. `target/f216-runs/` was read as a record. This report is the only file this pass adds to the worktree. One side effect: `cargo test` reran the corpus identity test, which rewrote `target/f216-runs/corpus-identity.txt` with the same five lines.

Mutations, instrumented builds and probe tests ran in an `rsync` copy at `scratchpad/p5/tree`. A `git archive ae7c9c9` copy at `scratchpad/p5/base` served as the before-tree. Both were built with `CARGO_TARGET_DIR=<wt>/target/p5/t-*`. Every mutated file was restored and confirmed with `diff` against the worktree, and `<wt>/target/p5` was deleted at the end.

**Author / reviewer**: GLM-5.3 in opencode wrote the remediation (report at the end of `scratchpad/glm-F-216-fix7b.log`, records in `target/f216-runs/`). The sprint lead wrote the "Lead amendments after review pass 4". The reviewer is a Claude agent invoked by the lead. It wrote none of the reviewed material. It also wrote pass 4, so it is independent of this round but not of the findings it is replaying.

**Result**: 4 defects, 1 smell, 3 nitpicks

## Acceptance challenged

- **Pass 4 D1, a stalled reader, through the release server.** The six held previews answered in at most 4 ms in every case: one, two and three stalled readers of a 4 MiB grey by id, and one and three stalled readers of a body-form preview. Six stalled body-form readers hold no body slot, because a `/file` post, a body-form preview and an id preview each answered in 1 ms beside them.
- **Is the new regression test discriminating?** I changed `finish` so that `GreyHome` keeps the `DecodePermit` until after the write, which is the pass 4 behaviour. `stalled_reader` stayed **green** (D3). The same mutated binary answered **none** of six previews within 8 s once two readers stalled, one per worker.
- **Pass 4 D2, body-form previews at `OCELLI_CACHE_MIB=0`.** 400 previews from 8 clients all returned 200. The census reached at most `decode_threads 2`, `peak_decodes 2` and `body_threads 2`. A mutation that sends the body form back to the connection thread turns `the_census_stays_on_two_decode_threads` red ("12 decode threads").
- **The adjacent case, the body form at a non-zero budget.** At 16 MiB, six held frames were followed by one body-form preview of an unheld 17.4 MB file. That preview emptied the cache, and every held id then answered 410. The pass 4 tree keeps all six (D1). With an over-budget 9-frame file whose 512 KiB frames fit a 4 MiB budget, 3 of 150 held ids were lost on the current tree and 0 on the pass 4 tree.
- **"The body freed on the thread the doc says."** An instrumented build showed that a pooled body sent with a `GET` was dropped on the connection thread (`ThreadId(4)`), not on worker `ThreadId(2)` or `ThreadId(3)`. The body is also read from the socket on the connection thread. The worker only allocates it (D4).
- **"Either thread takes any job" and `HOME_WAIT`.** With the home worker held and the other worker free, the fallback took 51.9 to 60.0 ms over 10 trials, not 5 ms (D2).
- **Pixel identity with the body form on the free list.** The expected value for each variant is the body-form reply from the **pass 4 binary**, whose body form never touches a free list. The variants were A, B (A's bytes as 128 by 96), C (32-bit) and T (fails after its claim), each at the file window and at an override. The current release binary ran at `OCELLI_CACHE_MIB=1` with 8 clients, 40 % body-form previews, and a re-post on every 410. Seeds 11, 12 and 13 ran 4,000, 4,000 and 3,000 requests, and seed 14 ran 2,500 on the debug binary. **0 mismatches** in status, the seven headers or the grey hash. An instrumented build counted **503** body-path claims that received a parked buffer, so cross-geometry reuse really ran on the new path. No census line had `used + free` over budget, and the debug `debug_assert` never fired.
- **Corpus identity with the poison fill**, run now: `files 86`, `frames 88`, `differences 0`, `reused 90`, `poison 0xa5`.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo fmt --all --check` | worktree, `CARGO_TARGET_DIR=<wt>/target` | now | exit 0 |
| `cargo clippy -p ocelli-folder-viewer -p ocelli-pixel --all-targets -- -D warnings` | same | now | exit 0 |
| `cargo test -p ocelli-folder-viewer -p ocelli-pixel` | same | now | exit 0. viewer lib 28, main 0, preview 59, scheduled_decodes 1, stalled_reader 1, pixel lib 14, color 15, image_plane 4, lut_chain 18, modality 5, palette 7, stored_pixel 2, voi 6 |
| `bin/ocelli.sh gate licences` | same | now | exit 0, ALL GREEN |
| `bin/ocelli.sh gate bench` | same | now | exit 0, ALL GREEN, 146 tests, 144 pass, 0 fail, 2 skipped |
| `bin/ocelli.sh gate guards` | same | now | exit 0, ALL GREEN |
| `guard_census.py`, then `--check-runbook` | same | now | exit 0 and exit 0, `0 watched by nothing` |
| `prose_check.py` | same | now | exit 0, 840 files |
| `source_provenance_check.py` | same | now | exit 0, 1676 files |
| `slown.py N id 10`, N = 1, 2, 3 | release, built 23:22, newer than every source | now | worst 0.003, 0.004 and 0.002 s, all 200 |
| `slown.py N body 10`, N = 1, 3 | same | now | worst 0.002 and 0.003 s, all 200 |
| `many.py 20`, 20 stalled readers of one 4 MiB grey | same | now | `workspace` 83,886,080 (20.0 greys), RSS 30.1 MB to 112.9 MB, back to 8,388,608 after close (S1) |
| `slots.py`, 6 stalled body-form readers | same | now | post, body preview and id preview each 200 in 0.001 s, `workspace` 25,178,112 |
| `zero.py`, 400 body-form previews, 8 clients, budget 0 | same | now | 400 of 200. Census max `decode_threads 2`, `peak_decodes 2`, `body_threads 2`, `peak_bodies 4` |
| `evict.py 16 3000 2900` | current, then the ae7c9c9 binary | now | current: 6 frames to **0**, held ids `[410 x 6]`. ae7c9c9: 6 frames kept, `[200 x 6]` (D1) |
| `evict2.py 4`, 150 held, 9-frame 4.7 MB file | current, then ae7c9c9 | now | current: 150 frames to 147, `410: 3`. ae7c9c9: 150 kept (D1) |
| `fl5.py 1 {11,12}`, release | expected from the ae7c9c9 binary | now | 0 bad over 4,000 each, 0 census lines over budget, decode threads 2 |
| `fl5.py 1 13`, instrumented body-claim count | scratch release | now | 0 bad over 3,000, `BODYREUSE` 503 |
| `fl5.py 1 14`, debug binary | `target/debug`, built 23:19 | now | 0 bad over 2,500, 243 census lines, 0 over budget, empty stderr |
| `p5_fallback_latency` (scratch test) | scratch, `Scheduler::start` | now | 53.9, 60.0, 57.7, 55.3, 51.9, 54.9, 58.3, 59.1, 59.2, 55.7 ms, all on worker 1 (D2) |
| `p5_lost_wakeup` (scratch test) | scratch | now | the waiter that wanted the returned slot waited 50.5 to 60.0 ms in 20 of 20 trials (D2, mechanism predates this round) |
| GET with a 100,000-byte body, instrumented | scratch release | now | `GETBODY cap 100000 pooled true dropped on ThreadId(4)`, workers are `ThreadId(2)` and `(3)` (D4) |
| `viewer.resident_memory` x3, `--no-build`, release, one process | `target/release`, loads 6.2 to 5.4 | now | **430,899,200**, **468,041,728**, **444,383,232** against 528,194,192, all met. Heap 98,123,859, 98,123,787 and 98,123,747, each agreed at collection 3, against 118,686,080. `held_peaks.free` 0, `held_peaks.workspace` 17,400,000 |
| `viewer.open` / `first_image` / `scroll_rate` / `window_step` | same | now | 963, 976 and 947 ms. CT 7.5 to 9.3 ms, DX median 28.6 ms. 109.01 fps at 1,801 keys, `blank_samples 0`, `debug_blank_samples 0`. CT 2.4 to 6.2 ms, DX median 17.7 ms, `preview_bodies 0`. All met, all release |
| implementer records `post-pass4-r1..3`, `red-*.txt` | records | prior | consistent with the report. The red stall receipt is on a tree with both the permit and the strict home, see D3 |

Mutations, each in the scratch copy, run and then restored:

1. M1 (pass 4 S1), `take_oldest_frame_buffer`'s `strong_count > 1` branch changed to `if false && ...`: `a_reply_reading_a_frame_keeps_it_cached_for_the_next_preview` is **red** ("the frame being read answered 410"). Pass 4 S1 is closed.
2. S2, the `requestGrey` lookup alone replaced by `() => {}`: **red** (13 pass, 1 fail). The `drawGrey` lookup alone: **red**. Pass 4 S2 is closed.
3. `GreyHome` keeps the `DecodePermit` until `restore_grey`: `stalled_reader` is **green** (D3).
4. `preview_route`'s `!viewer.schedules()` changed to `true || ...`: `scheduled_decodes` is **red**.

Environment: Apple M4 Max, macOS 27.0.0 kernel, Node 24.13.0, Python 3.14.5. Load ran from 3.4 to 14.3, with 11 users and the operator's debug viewer, PID 98891, on 8742. The timings are indicative and are not a baseline. No baseline was written. Every viewer I started ran with `OCELLI_PORT=0` and was stopped by `Popen.kill` on its own PID or by the bench runner. Ports 8742 and 8765 were never used, and PID 98891 was not touched. At the end, `ps` showed only PID 98891.

## Pass 4 findings replayed

| Pass 4 | Status | Replay and adjacent case |
|--------|--------|--------------------------|
| D1 stalled reader holds the permit | **closed in behaviour** | 1, 2 and 3 stalled readers by id, and 1 and 3 by body, all answered in at most 4 ms. Its new test does not prove the permit change (D3), and the fallback's stated 5 ms is about 55 ms (D2) |
| D2 body-form decodes on connection threads | **closed for the census, new defect beside it** | `zero.py` stays at 2 threads and 2 decodes, and mutation 4 is red. The body form now evicts cache entries it never uses (D1). The "freed on the thread that allocated it" sentence is still false for a GET with a body (D4) |
| D3 free-list and steal sentences | **partly closed** | "Parked when it fits, freed otherwise" now matches `park_if_fits`, `keep_buffer_if_room` and `make_room_dropping`, given that the room is for the incoming frame. The steal sentence lost its "the cache is full" condition (D4) |
| D4 plan `/held` row | **closed** | The pass 3 bullet now names `inflight`, `workspace` and `free`. The reply is `build, budget, used, files, frames, inflight, workspace, free` |
| S1 steal check untested | **closed** | Mutation 1 is red |
| S2 page hooks | **closed** | Mutation 2 is red both ways |

## Defects

### D1, a body-form preview evicts cached entries for a sample buffer it never stores, then frees that buffer

**Where**: `examples/folder-viewer/src/serve.rs:551-587` (`pooled_body_preview`), the claim at `:569` and the recycle at `:581`. `held.rs:1003-1026` (`claim_buffer`: the steal at `:1016`, `make_room_dropping` at `:1008` and `:1023`). `preview.rs:664` (`std::mem::take(samples)`).
**What**: the scheduled body form now claims its sample buffer through `CacheHandle::claim_sample_buffer`. That claim was written for a decode that is about to be cached. At a full budget it steals an unheld cached frame of the same length, or it evicts least recently used files and frames until the frame's cost fits. A frame larger than the whole budget therefore empties the cache. The body form never stores the frame, so the room it made is not used. On success `finish_decode` moves the buffer into the `HeldFrame`, and that frame is dropped inside the `Ok` arm. `recycle_sample_buffer(samples)` then receives an empty `Vec` and parks nothing. The doc comment says "the buffer goes back to the free list because no frame is stored". That is false on success, because the buffer is freed.
**Why it is wrong**: the plan's state row for a file that is not held says only that the page uses the body form for that file from then on. AC13 says each file is posted once while it is held. Here, viewing one unheld file evicts unrelated held files, and the page has to re-post them. The pass 4 lead amendment asks for the body form to run under a permit and be charged in flight, not to take room in the cache.
**Evidence**: `evict.py 16 3000 2900`: the current tree went from 6 frames to 0, then `[410 x 6]`. ae7c9c9 kept 6 frames, then `[200 x 6]`. `evict2.py 4`: 3 of 150 held ids were lost on the current tree and 0 on ae7c9c9. `free` was 0 after both runs.
**Scope and regression**: any page that has open a file larger than `OCELLI_CACHE_MIB` (or one whose decoded frame is), next to held files. At the default 256 MiB that means files over 256 MiB. At a lowered budget it is common. Pixels are not affected (0 mismatches above). Smallest fix: the body form takes a buffer from the free list only (a pop, with no steal and no eviction), allocates otherwise, and returns `held.samples` to `park_if_fits` after rendering. Smallest regression: on `Viewer::server`, hold frames, send a body-form preview of a file over budget, and require the frame count unchanged and the held ids still 200.

### D2, the home-worker fallback takes about 55 ms, not the documented 5 ms, and the test checks only a lower bound

**Where**: `examples/folder-viewer/src/schedule.rs:27-31` (`HOME_WAIT`, "How long a decode waits for its home worker before any worker takes the job"), `:160-186` (`acquire_decode`), `:328-354` (`wait_slot`), `:356-359` (`return_slot`'s `notify_one`). Test `a_busy_home_worker_hands_the_job_to_the_other_worker` (`:559`).
**What**: the deadline is checked only inside the `accept` predicate, and that runs only when the condvar wakes. With the other worker idle there is no notify, so the waiter sleeps the full `WAIT` of 50 ms. Measured: 51.9 to 60.0 ms in 10 of 10 trials. A preview homed to a worker that stays busy therefore waits 10 times the stated bound while the other worker is idle. Adjacent mechanism, which predates this round because `return_slot` is unchanged since ae7c9c9: `notify_one` can wake a waiter whose predicate declines the returned slot. The waiter that wanted it then sleeps 50 ms. That happened in 20 of 20 trials in `p5_lost_wakeup`.
**Why it is wrong**: microscope section 3, a doc comment that is false when executed. The pass 4 amendment says "either decode thread takes the next job" so that a busy home worker does not hold a preview while the other is idle. The test asserts `elapsed >= HOME_WAIT` only, so a 60 ms or a 60 s fallback both pass.
**Evidence**: the two scratch tests in the evidence table.
**Scope and regression**: every held preview whose frame or bytes live on a busy worker. Smallest fix: wait no longer than `deadline - now` while `home` is set, and wake all decode waiters when a slot returns. Smallest regression: the existing test with an upper bound, for example under 25 ms.

### D3, `stalled_reader` stays green with the permit held through the write, which is the property its name states

**Where**: `examples/folder-viewer/tests/stalled_reader.rs:1-6` (module doc) and `a_reader_that_stops_does_not_hold_the_decode_permit`.
**What**: the test has one stalled reader. Once a fallback exists, the six previews run on the other worker even when the stalled reply keeps its permit. Mutation 3 restores the pass 4 permit lifetime, and the test stays green. The module doc says the six waited on the old tree because "the permit came back only after the reply was written". The implementer's red receipt is from a tree that had both the late permit and the strict home, so that receipt does not separate the two causes.
**Why it is wrong**: microscope section 2. A test that passes both ways is a defect, and it would be counted as coverage of the permit change forever.
**Evidence**: mutation 3 green. On the same mutated binary, `slown.py 2 id 8` answered none of six within 8 s, while the current tree answers all of them in 4 ms.
**Scope and regression**: the D1 closure rests on my probe, not on the suite. Smallest fix: two stalled readers, one homed to each worker, then a preview that must answer within the deadline. Mutation 3 must go red.

### D4, three body-thread sentences and the steal sentence are false of the code

**Where**: `README.md:185`, `examples/folder-viewer/src/use.html:270`, `docs/lld/cache.md:175` and `:181-187`, `examples/folder-viewer/src/schedule.rs:1-3` and `:7`, `examples/folder-viewer/src/held.rs:450-452`. Code: `serve.rs:363`, `serve.rs:1296-1320` (`read_pooled_body`).
**What**:
1. "every pooled request body is read and freed on these two threads" (schedule.rs), "Request bodies are read and freed on the `P` workers" (held.rs) and "A pooled body is freed by the worker that read it" (schedule.rs). A worker only allocates the body (`permit.call(move || exact_body(declared))`). `fill_prepared` reads it from the socket on the connection thread.
2. "A request body is freed on the thread that allocated it" (README, use.html, cache.md). The `("GET", _)` arm does not retire the body. A pooled GET body is dropped on the connection thread, as the instrumented run showed (`ThreadId(4)`, workers 2 and 3).
3. "When no free buffer has the length, the decode takes the samples of the oldest cached frame of that length only when no reply is still reading that frame, and allocates otherwise" (README, use.html, cache.md). `claim_buffer` steals only when `charged + cost > limit`. With room in the budget it allocates and keeps the older frame. `evict.py` held six same-length A frames at 16 MiB. The pass 4 text said "When the cache is full and no free buffer has the length", and this round dropped that condition.
4. "A decode copy is in flight until the frame is in the cache" (cache.md:187). A body-form decode's copy never reaches the cache. It is in flight until the job ends.
**Why it is wrong**: microscope section 3. README, `use.html` and the LLD are always in scope, and the operator asked for the body-thread sentence specifically.
**Evidence**: the instrumented GET run, `evict.py`'s first line, and the code paths cited.
**Scope and regression**: documents and doc comments. Items 2 and 3 can be fixed in code or in prose. Retiring the body in the GET arm makes item 2 true, and "when the cache is full" restores item 3. Item 1 needs "allocated and freed" in place of "read and freed".

## Smells

### S1, grey reply bytes are no longer bounded by the decode permits, and the resident formula still counts one grey per permit

**Where**: `schedule.rs:252-287` (`finish`, `GreyHome::restore_grey`), `:418-425` (`detach_grey` now accumulates). The plan's "Resident memory bound", which says that `5 * N_max` inside each permit is "4 bytes of `Stored` scratch plus 1 grey byte".
**Failure mechanism**: the amendment ends the permit before the write, so a worker starts its next decode while the previous grey is still being written, and each in-flight reply now holds its own grey. Grey bytes are bounded by the connection cap (32), not by `P`. Measured: 3 stalled readers gave `workspace` 12.6 MB against the 8.4 MB term, and 20 gave 83.9 MB with RSS up 83 MB. The plan excludes other clients, so this is not a bound violation for a stalled client. The page itself issues main, tile and prefetch previews concurrently, though, and the formula's grey term is no longer enforced by any mechanism. The three resident runs met, with `held_peaks.workspace` 17,400,000, so the measured workload is still inside it. A page change with more concurrent previews would leave it silently.
**Smallest fix**: a lead amendment, either a grey term per in-flight reply in the formula, or a cap on detached greys with a stated wait rule.

## Nitpicks

- **N1** `viewer_work_test.mjs:185-186`: "replacing either global lookup alone in assertPageHooks turns one of them green". The mutation makes the test red. The sentence means that a refusal stops refusing.
- **N2** `serve.rs` `BodyHome` doc: "the body was allocated by another worker". It may be the same worker.
- **N3** the steal sentences and the amendment say "oldest" frame. `frame_order` is touched on every hit, so it is the least recently used frame.
- Carried from pass 4, not re-examined this pass: N1 to N6, N8, N9, N10 (this pass's three resident runs started at loads 6.2, 5.8 and 5.5), N11, N12 and N13.

## Verified clean

- **Pixel correctness of the new body-form path**: 0 mismatches over 13,500 requests in four runs, including 503 body claims of a parked buffer across geometries A, B and C, failed decodes (T) after a claim, and the debug binary. The corpus poison test reported 0 differences.
- **Budget**: no census line had `used + free > budget` in any run, and the debug `debug_assert` never fired.
- **Concurrency**: no decode job acquires a body slot or waits on anything a connection holds. The `call_worker` jobs (allocation, decode) and the `post_job` restores share one FIFO per worker, and none blocks. A stalled body-form reader drops its `BodyCharge` and slot before the write (`slots.py`). Permits are RAII, and every early return in `preview_route` and `preview_held_on_worker` drops the permit before the reply is written. `GreyHome::restore_grey` after a worker exits drops the grey on the caller, and `DETACHED` returns to its baseline after the stalled sockets close. `Scheduler::drop` sends `Stop` behind queued jobs, so queued restores run first. That drop happens only in tests, because the server's `Arc<Viewer>` lives for the process. That last point was reasoned, not exercised.
- **Prose executed**: "Every preview decodes there, one that carries the file's bytes included" (census), "a decode permit ends with its decode, so a client that stops reading holds no permit" (stall replays), "parked ... when cached bytes plus free bytes still fit the budget, and is freed otherwise" (code paths), "A decode takes a parked buffer of its length when there is one" (`claim_buffer`), the `held.rs` module and `CachedFrame` docs, `restore_grey`'s dead-worker sentence, and the `scheduled_decodes` module doc. The false sentences are D1, D2, D3 and D4.
- **Memory**: three one-process resident runs and the four other release subjects, all met and all `profile: release`.
- No `as` cast, `#[allow]` or `unsafe` was added. The six required commands and gates were green.

## Limitations and handoff

- Not exercised: a non-loopback slow network, a stalled reader through Chromium, the scheduler's drop with queued jobs on a live server, and Enhanced multi-frame corpus files through the server's body-form path. The corpus identity test drives `Viewer::new`'s direct path. The body-form identity on `Viewer::server` rests on the black-box runs above.
- This is a clean bounded review, not acceptance and not sprint readiness. AC17 and AC18 measured met on this tree.
- Next owner: implementation, for D1 to D4. The lead, for S1's formula and for whether D4 items 2 and 3 are fixed in code or in prose. A fresh pass should replay `evict.py`, mutation 3 with two stalled readers, and the fallback timing.
