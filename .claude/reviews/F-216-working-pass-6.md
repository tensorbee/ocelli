# F-216 review, pass 6

**Reviewed**: `work/f-216-xai`, `git diff 7bcf8e4 HEAD`, with HEAD at `18a3565de1228e4131ba89723d1997243572e320` ("F-216, checkpoint after the pass 5 remediation"). The diff covers 9 paths: the plan, README.md, docs/lld/cache.md, held.rs, schedule.rs, serve.rs, use.html, the new tests/body_form_budget.rs and tests/stalled_reader.rs. The working tree was clean apart from the untracked `.claude/reviews/` passes 1 to 5 and `tools/oracle/out`, which were not reviewed. `target/f216-runs/p6-*` and `.claude/scratch/F-216-progress.md` were read as records. This report is the only file this pass adds. One side effect: `cargo test` reran the corpus identity test, which rewrote `target/f216-runs/corpus-identity.txt` with the same five lines.

Mutations ran in an `rsync` copy of HEAD at `scratchpad/p6/tree`, and an allocator-instrumented build ran in a second copy at `scratchpad/p6/tree2`. Before-trees came from `git archive ae7c9c9` (the body form decoded on the connection thread, used for pixel expectations) and `git archive 7bcf8e4` (the pass 5 tree, used for the fallback comparison). All of them were built with `CARGO_TARGET_DIR=<wt>/target/p6rev/*`. Every mutated file was restored and confirmed identical to the worktree. `<wt>/target/p6rev` and `scratchpad/p6` were deleted at the end.

**Author / reviewer**: a Claude implementer wrote this round, building on a stopped GLM attempt (`.claude/scratch/F-216-progress.md`, "Pass 5 remediation, fix 8"). The sprint lead wrote the "Lead amendments after review pass 5" and the "Lead rulings on the pass 5 remediation". The reviewer is a Claude agent invoked by the lead. It wrote none of the reviewed material. It also wrote pass 5, so it is independent of this round but not of the findings it replays.

**Result**: 2 defects, 2 smells, 3 nitpicks

## Acceptance challenged

- **Pass 5 D1, the body form evicting at a full budget**: through the release server, six held ids at 16 MiB and 150 at 4 MiB all stayed 200 after body-form previews of an over-budget file and of a 9-frame file. The adjacent case, the free list at 256 MiB: three body-form previews of a 17.4 MB frame left `free` at 17,400,000 each time, so the body form pops and re-parks one buffer and never grows the list.
- **Pass 5 D2, the fallback latency, through the server**: one worker held by a 49 ms body-form decode, then held previews of four small frames homed on both workers, 32 previews per run. The worst on HEAD was 8.6, 8.5 and 8.5 ms. On 7bcf8e4 it was 23.8 and 25.3 ms, which is the end of the decode, so the old whole-`WAIT` sleep is visible. The adjacent case, the declined-wakeup test, ran 10 times in a row: green each time.
- **Pass 5 D3, the stalled-reader test**: my mutation Mb (the `GreyHome` keeps the `DecodePermit` until after the write, pass 5's mutation 3) turns `stalled_reader` **red**, and `twenty_stalled_readers_never_exceed_the_grey_cap` too. Closed. The test now passes because of a shape chosen to fit the cap, though (D1).
- **Pass 5 D4, body threads**: an allocator-instrumented release build logged the thread of every allocation and free of a pooled body. GET, 404, `/inspect`, `/headers`, id with a body, a bad body-form, a truncated body, `/file` garbage and a congested body-form preview were each **freed on the worker that allocated them**. Closed.
- **Pass 5 S1, grey bytes past the permits**: 20 stalled readers of one 4 MiB grey gave `workspace` 8,388,608 (2.0 greys). Pass 5 measured 20.0. Closed in bytes. Its cost is D1.
- **Deadlock attempt**: X is a 3000 by 2900 frame (8.7 MB grey). 20 stalled id readers of X, then 8 clients for 8 s posting A/B/C/T files, previewing by id (at the file window, an override and a refused zero width) and sending body-form previews. While it ran, the census showed `grey` 17,400,000, `decodes_now` 0 and `bodies_now` 3. All 8 clients finished **0.01 s** after the stalls closed. None hung. At the end `grey` 0, `inflight` 0, `bodies_now` 0 and `decodes_now` 0, and a post and a preview answered 200. **0 mismatches** against ae7c9c9.
- **Leaked reservations, through the server, with the census log**. In each case `grey` and `inflight` returned to 0. A client that disconnects mid-wait: four body-form waiters closed, after which `inflight` was 0, `bodies_now` 0 and the next post took 0.0 s. A 503 after the timeout, held and body forms: 60.0 s each. A refused render under a full cap: hit, freshly decoded and body form, each 400. Not reachable on a live server: a worker gone (mutation Md shows the suite covers the refused-permit path).
- **Re-lookup after eviction while waiting**: a 3-frame file whose frames 0 and 1 have distinct content. Both previews met a full cap and waited, and filler posts then evicted entries. With frame 0 evicted and the file kept (`files 3, frames 1`), the frame 0 preview **re-decoded and returned frame 0's grey** (`a7005e7f...`, the ae7c9c9 body-form value), never frame 1's (`e51ca51f...`). With the file also gone it answered 410. Sixteen configurations, 0 wrong greys.
- **Mixed geometry under concurrency**: `fl5.py` at 1 MiB with seeds 31 and 32 and at 64 MiB with seed 33, 3,000 requests each, 8 clients, 40 % body form, re-posting on 410. **0 mismatches** in status, the seven headers or the grey hash against ae7c9c9. 0 census lines over budget, 2 decode threads, peak decodes 2.
- **Corpus identity with the poison fill**: `files 86`, `frames 88`, `differences 0`, `reused 90`, `poison 0xa5`.
- **Ruling 3's premise, executed**: X was posted and two stalled readers filled the cap. Four body-form previews of X then waited with their body slots, and a small `/file` post **waited 57.77 s**. Each of the four answered 503 at 60.0 s.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo fmt --all --check` | worktree at 18a3565, `CARGO_TARGET_DIR=<wt>/target` | now | exit 0 |
| `cargo clippy -p ocelli-folder-viewer -p ocelli-pixel --all-targets -- -D warnings` | same | now | exit 0 |
| `cargo test -p ocelli-folder-viewer -p ocelli-pixel` | same | now | exit 0. viewer lib 31, main 0, body_form_budget 1, preview 59, scheduled_decodes 1, stalled_reader 1, pixel lib 14, color 15, image_plane 4, lut_chain 18, modality 5, palette 7, stored_pixel 2, voi 6 |
| `cargo test --lib held::schedule` x10 | same | now | 10 of 10 green, 5 tests each |
| `bin/ocelli.sh gate licences`, `gate bench`, `gate guards` | same | now | exit 0 each, ALL GREEN. bench 146 tests, 144 pass, 0 fail, 2 skipped |
| `guard_census.py`, then `--check-runbook` | same | now | exit 0 and exit 0, `0 watched by nothing` |
| `prose_check.py` | same | now | exit 0, 840 files |
| `source_provenance_check.py` | same | now | exit 0, 1677 files (one more than pass 5, the new test file) |
| `slown.py N id 10`, N = 1, 2 (pass 5 probe) | `target/release`, rebuilt as a no-op | now | N=1: six 200 in at most 2 ms. **N=2: none of six answered within 10 s** (D1). Pass 5 measured 4 ms on its tree |
| `cap.py disconnect` | same | now | 2 stalls gave `grey` 17,400,000. 4 body waiters gave `inflight` 139,202,648. Closed, then 0. A small id preview answered **503 after 60.0 s** |
| `cap.py slots` | same | now | `/file` post **57.77 s**. Four body-form previews 503 at 60.01 s. End `grey` 0, `inflight` 0 |
| `refuse.py` | same | now | refused hit, shared and body under the cap: 400 each. `grey` 17.4 MB while waiting, 0 at the end |
| `evwait.py`, budgets 10 to 20 MiB, 0 to 4 fillers, 2-frame and 3-frame files | same, expectations from ae7c9c9 | now | every answer was the requested frame's grey or 410. The re-decode path ran at `14 2` |
| `storm.py 21` | same | now | 0 hung, 0 bad, end census all zero |
| `fl5.py 1 31`, `1 32`, `64 33` | same, expectations from ae7c9c9 | now | 0 bad over 9,000. 0 over budget |
| `evict.py 16 3000 2900`, `evict2.py 4 ... 150` | same | now | `[200 x 6]`, `{200: 150}`, `frames` unchanged |
| `many.py 20` | same | now | `workspace` 8,388,608 (2.0 greys), RSS 35.1 MB |
| `bodythr.py`, allocator probe | scratch release of HEAD, mimalloc wrapped | now | 9 body sizes, each freed on its allocating thread |
| `fallback.py` | HEAD vs 7bcf8e4 release | now | worst 8.6, 8.5, 8.5 ms vs 23.8, 25.3 ms |
| `viewer.resident_memory` x3, `--no-build`, one process each | `target/release`, loads 5.3, 12.6, 15.1 at start | now | **461,029,376**, **447,610,880**, **440,549,376** against 528,194,192. Thresholds met. Heap 98,123,819, 98,123,735, 98,123,735 against 118,686,080. `held_peaks.workspace` 17,400,000, `held_peaks.free` 0 |
| `viewer.open` / `first_image` / `scroll_rate` / `window_step` | same, load 25.4 at start | now | 1,046 ms (bound 1,100). CT 9.9 ms, DX median 32 ms. 109.16 fps at 1,804 keys, 0 blanks, 0 debug blanks. CT 2.6 ms, DX median 18 ms, `preview_bodies` 0. All met, all `profile: release` |
| implementer records `p6-red-M1..M7`, `p6-resident-r*`, `p6-viewer.*` | records | prior | consistent with what I ran. M4 matches my Mb |

Mutations, each applied in the scratch copy with a count-of-one assertion, run against all of `cargo test -p ocelli-folder-viewer`, then restored:

| Mutation | Result |
|----------|--------|
| Mb, the permit is held through the write | **red**: `stalled_reader` and the twenty-stall test |
| Mc, no release when the frame is gone after the wait (`serve.rs:925`) | **green**, the whole suite (S2) |
| Md, no release when the permit is refused after the wait | **red**: "after every reply the gate still held 8700000" |
| Me, no release after the write | **red**: three tests |
| Mf, a cap of three greys | **red**: "held 26100000 grey bytes over 17400000" |
| Mg, the body form keeps the reservation on a refused render (`serve.rs:655`) | **green**, the whole suite (S2) |
| Mh, the body form keeps its inflight charge when the wait fails (`serve.rs:546`) | **green**, the whole suite (S2) |

Environment: Apple M4 Max, macOS 27.0.0 kernel, Python 3.14.5. Load ran from 3.9 to 25.4, with 11 users and the operator's debug viewer, PID 98891, on 8742. The timings are indicative, not a baseline, and no baseline was written. PIDs 56451 and 58972 were sampled with `sample`, which does not signal. Both are blocked in `held::schedule::tests::a_returned_slot_wakes_the_waiter_that_wants_it` at `schedule.rs:636`, a GLM-era test that HEAD does not contain. They sat at 0 to 0.3 % CPU and did not affect the measurements. Every viewer I started ran with `OCELLI_PORT=0` and was stopped by `Popen.kill` on its own PID or by the bench runner. Ports 8742 and 8765 were never used. PIDs 98891, 56451 and 58972 were not signalled.

## Pass 5 findings replayed

| Pass 5 | Status | Replay and adjacent case |
|--------|--------|--------------------------|
| D1 body form evicts | **closed** | `evict.py` and `evict2.py` keep every id. `free` stays one buffer across repeated body-form previews. Implementer M1 red |
| D2 fallback 55 ms | **closed** | 8.5 ms worst through the server against 25 ms on 7bcf8e4. The test has an upper bound (implementer M2 red). The declined-wakeup test is green 10 of 10 (implementer M3 red under `notify_one`) |
| D3 test passes both ways | **closed** | Mb is red. The test's shape hides D1, see below |
| D4 body-thread and steal sentences | **closed** | allocator probe over 9 routes. "Least recently used" and "does not fit beside the cached and free bytes" match `claim_buffer` (`charged() + cost > limit`, `frame_order` touched on hit) |
| S1 grey bytes unbounded | **closed in bytes** | 2.0 greys under 20 stalls. The bound accounting is S1 below, and the liveness cost is D1 |

## Defects

### D1, two stalled readers stop every preview for 60 s, the pass 4 D1 symptom at the same trigger, and the regression test is shaped so it cannot see it

**Where**: `examples/folder-viewer/src/held.rs:398-410` (`GreyGate::reserve`, `cap = PERMITS * max(max, len)`), `serve.rs:832` (`wait_for_grey`), `serve.rs:976` and `:545` (the two waits). `tests/stalled_reader.rs:4-9` and `:18-25` (`SIX_ROWS` 3000, `SIX_COLUMNS` 5800, "Beside the two stalled greys, one of these still fits the grey cap's second slot").
**What**: the cap is exactly two of the largest grey seen. Two clients that stop reading a largest-size grey fill it. After that, every preview of every size waits the full write timeout and answers 503: held and body form, 512 by 512 greys included. Measured: with two stalled 4 MiB greys, six 262 KB previews got no answer in 10 s, and one answered 503 after 60.0 s. The pass 4 tree failed the same way at the same trigger (pass 5: "answered none of six previews within 8 s once two readers stalled, one per worker"). The only difference is that the cap now holds it instead of the permits. `stalled_reader` passes because its six previews use a frame twice the stalled size, which raises `max` and opens room for one. Six previews of the stalled frame, or of any smaller frame, would fail it. Its module doc still says the old tree's six "waited for the stalled writes", which the new tree also does for those cases.
**Why it is wrong**: the pass 4 lead amendment exists so that a client that stops reading does not hold up other previews ("a client that stops reading holds no decode permit. Either decode thread takes the next job"). Pass 4 D1 was the user-visible stall, not the permit object. The pass 5 amendment's wait rule is followed to the letter. Ruling 1, though, sizes the cap with no headroom, and ruling 3 accepts degradation only for a `/file` post. Neither accepts that two broken clients stop the viewer for a minute. This is a conflict inside the design, so the call belongs to the lead.
**Evidence**: `slown.py 2 id 10` (none in 10 s), `cap.py disconnect` (503 at 60.0 s), the `stalled_reader.rs` constants.
**Scope and regression**: any viewer with two clients that stop reading (a hung tab or a broken script) at the largest frame size. The page itself reads every reply, so a single healthy page is not affected, and the resident runs and four subjects met. Options for the lead: give the cap headroom above `P * N_max` and carry it in the formula, stop counting a reply's grey against the cap once its write is stalled past a short bound, or accept the degradation and document it. Smallest regression: the existing test with six previews of the **stalled** frame, or of one smaller frame, that must answer within `DEADLINE`, or must answer 503 if the lead accepts the degradation.

### D2, ruling 3's condition is unmet because the user documents do not say a `/file` post can wait 60 s, and one doc comment is false in the deferred case

**Where**: `README.md:185`, `examples/folder-viewer/src/use.html:270`, `docs/lld/cache.md:191-195`. The ruling is at `.claude/plans/F-216-design.md:492`. Code: `serve.rs:613`.
**What**:
1. Ruling 3 accepts "a `/file` post can wait up to the 60 s write timeout ... and the user documents say so". None of the three documents mentions a post waiting, a body slot held by a waiting preview, or the four-waiter limit. They say only "A preview past the cap waits up to the 60 second write timeout and then answers 503 busy". Measured: a `/file` post waited 57.77 s.
2. `pooled_body_preview`'s doc says "The sample bytes are charged as in flight until the job ends". On the `Congested` return the job ends with the samples still charged. They stay charged until `settle` in `preview_route`, after the wait. Measured: `inflight` 139,202,648 with four waiters, all four jobs finished.
**Why it is wrong**: microscope section 3. README and `use.html` are always in scope, and the lead's ruling makes the documentation a condition of the acceptance.
**Evidence**: `cap.py slots` and `cap.py disconnect`. A grep of the three documents for "post", "wait" and "body slot" finds only the preview sentence.
**Scope and regression**: documents and one doc comment. Add one sentence to each document, for example "While four such previews wait, a file post waits for a body slot, up to the same 60 seconds". In the doc comment, change "until the job ends" to "until they are recycled, after any wait for the cap". These are prose fixes, so the regression is a re-read.

## Smells

### S1, rulings 1 and 2 assign the bound's one grey term to two sets of bytes that coexist

**Where**: `.claude/plans/F-216-design.md:282` (inside each permit, `5 * N_max` is "4 bytes of `Stored` scratch plus 1 grey byte"), `:484` (the cap is "equal to the formula's existing grey term"), `:490`, `:491`. README.md:185, use.html:270 and cache.md:191 say the cap "is the grey term of the bound". `serve.rs:2890-2894` (`WORKSPACE_GREYS`).
**Failure mechanism**: ruling 1 makes the cap the formula's `P * N_max`. Ruling 2 puts each worker's returned grey buffer under the per-permit term, which is that same `P * 1 * N_max`. The two coexist: `restore_grey` parks one grey per worker, and the cap separately admits two unwritten greys. The implementer's own test budget counts them separately: per workspace, 2 greys of native bytes, 4 of scratch and 1 returned grey, "and the cap is two more". That is 16 greys, against the formula's 14 for 16-bit raw (`P * (2 * S_max + 5 * N_max)` with `S_max` = 2 `N_max`, of which the parse copy is outside the workspace). The extra `P * N_max`, 17.4 MB at the measured 8.7 Mpx, is absorbed only by the 32 MiB constant, which the plan reserves for "the binary, thread stacks and allocator slack". The three resident runs met with about 67 MB to spare. A larger `N_max` makes the gap grow in step with the frame, while the constant does not grow. The README sentence then states an accounting the code does not follow.
**Smallest fix**: a lead decision. Either add `P * N_max` to the formula as the cap's own term, or rule that the cap absorbs the returned buffers and count them against it. Then make the three document sentences agree.

### S2, three release paths have no test, and a leak there is permanent and invisible in `/held`

**Where**: `serve.rs:925-928` (frame gone after the wait), `serve.rs:654-656` (body-form refused render), `serve.rs:545-547` (body-form wait failure releases the inflight charge).
**Failure mechanism**: mutations Mc, Mg and Mh each removed one release, and the whole `ocelli-folder-viewer` suite stayed green. A leaked reservation never comes back. Two leaks of a largest grey fill the cap for the life of the process, after which every preview answers 503 after 60 s, which is D1 with no stalled client at all. `GET /held` does not report `grey`. Only the opt-in `OCELLI_CENSUS_LOG` does, so the field cannot see it. The implementer's notes claim the inflight release "on every exit, including a refused render". The code does it (my `refuse.py` and `cap.py disconnect` runs), but no test holds it.
**Smallest fix**: extend `twenty_stalled_readers_never_exceed_the_grey_cap`, or add a sibling, with three waiters that end on those paths: an id whose file and frame are evicted during the wait, a body-form preview with a zero window width, and a body-form waiter that disconnects. Then the existing "gate still held" and inflight checks cover them. Mc, Mg and Mh must each go red.

## Nitpicks

- **N1** While a body-form preview waits for the cap, `inflight` still counts its body (`BodyCharge.bytes` until `route` returns), although `drop(body)` already sent the bytes back to be freed. Four waiters showed 139.2 MB, of which 69.6 MB was freed bodies. This over-counts, so it is conservative, but `MemoryCensus.inflight` says "Body bytes being read".
- **N2** `GREY_ROUNDS` says the third round "is not reached". The `release_reserved` and `grey_busy_reply` after the loop are therefore unreachable by the doc's own argument. A `debug_assert!` or a two-round loop would say so.
- **N3** `a_waiting_decode_ends_at_shutdown` and `a_returned_slot_reaches_a_waiter_that_accepts_it` would hang rather than fail if `wait_slot` stopped honouring `shut` or the decliner's deadline. That is reasoned, not run. PIDs 56451 and 58972 are a GLM-era test of the same shape that did hang for over two hours.
- Carried from pass 5 and not re-examined: N1 to N3 of pass 5 and its carried list.

## Verified clean

- **Concurrency**: no deadlock in the storm (20 stalls, 8 clients, posts, id, body and refused previews, then close). A waiter holds no decode permit (`decodes_now` 0 under 18 waiters). A held frame is in the cache before its preview waits (`evwait` showed `frames 2` while waiting, and implementer M6 is red). `wait_for_grey` checks the peer every 50 ms, and a disconnect frees the body slot, the samples and the inflight charge within 0.5 s.
- **Reservation accounting**: the reservation and the release both use `width * height`. `Preview.width` and `height` are `held.width` and `held.height` (`preview.rs:887-888`), so `connection_session`'s measure matches `grey_bytes`. Every Gone, refused-permit, dead-worker, refused-render and decode-error path either releases or carries the reservation into a 200 reply with a `GreyHome`. Each path was read. The ones reachable from a socket were run, and `grey` ended at 0.
- **Pixel correctness**: re-decode after eviction during the wait returns the requested frame. 9,000 mixed-geometry requests and the storm matched ae7c9c9. The corpus poison test reported 0 differences. File ids come from a monotonic `AtomicU64`, so a re-lookup cannot find another file under the same id.
- **Prose executed**: "twice the largest grey so far" (17.4 MB for 8.7 MB greys, and the third did not fit), "waits up to the 60 second write timeout and then answers 503 busy" (60.0 s, 503), "A waiting preview holds no decode permit", "A held frame it decoded is in the cache before it waits", "never evicts and never takes a cached frame's samples", the steal sentence, "allocated and freed on the `P` workers" with the connection thread reading, `wait_slot`'s doc, `render_decoded`'s "on whichever worker is free", and "the body slot is kept, so at most four body-form previews wait". The false or missing sentences are D2. The accounting sentence is S1.
- **Memory**: three one-process resident runs and the four other subjects, all met and all `profile: release`.
- No `as` cast, `#[allow]` or `unsafe` was added in the tracked diff. The six required commands and gates were green.

## Rulings judged

1. **Cap = `PERMITS` times the largest grey so far**: sound for the byte bound, because it is never above `P * N_max`. It is unsound for liveness: with no headroom, two stalled clients stop all previews (D1).
2. **Returned grey buffer under the per-permit term**: unsound as accounting, because that term is the one ruling 1 already gave to the cap (S1). The bound still met here through the 32 MiB constant.
3. **A waiting body-form preview keeps its body slot**: the behaviour is as described (57.77 s post). Its stated condition, "the user documents say so", is not met (D2). Not covered by the formula, and a plausible concern rather than a finding: four waiters hold decoded samples outside any permit. For a compressed body the samples exceed the body that the `4 * S_max` term pays for. The measured workload is uncompressed.

## Limitations and handoff

- Not exercised: a worker that dies on a live server (the server's `Arc<Viewer>` lives for the process), `Scheduler::drop` with queued jobs on a live server, a non-loopback slow network, a stalled reader through Chromium, and compressed files through a body-form wait.
- This is a clean bounded review of the paths named above, not acceptance and not sprint readiness. AC17 and AC18 measured met on this tree.
- Next owner: the lead, for D1 (cap headroom, or accept and document) and S1 (formula term). Implementation, for D2 and S2. A fresh pass should replay `slown.py 2 id`, `cap.py slots`, and mutations Mc, Mg and Mh.
- `scratchpad/p5` (7.1 GB, pass 5's copies) is still in this session's scratchpad. It was not created by this pass and was left in place.
