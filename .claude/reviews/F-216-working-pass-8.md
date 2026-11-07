# F-216 review, pass 8

**Reviewed**: `work/f-216-xai`, `git diff 6ed89e7 171804b`, HEAD `171804b73e0c9217f5134d4519b41b1c96dc64b9` ("F-216, a kept grey holds room under the cap"). Seven paths: `.claude/plans/F-216-design.md` (two lead rulings, 3efe045 and d22c29c), README.md, docs/lld/cache.md, held.rs, schedule.rs, serve.rs, use.html. Two implementer rounds: 1834b98 (arrival-order queue) and 171804b (kept-grey test and documents). The tree was clean apart from the untracked `.claude/reviews/` passes 1 to 7 and `tools/oracle/out`, which were not reviewed. `.claude/scratch/F-216-progress.md` (last two sections) and `target/f216-runs/p9-*` and `p10-*` were read as records. This report is the only file this pass adds. Mutations ran in `git archive HEAD` extracted to `target/p8rev-scratch` with `CARGO_TARGET_DIR=target/p8rev-target`. Each mutated file was restored and compared identical to the worktree after every mutation, the whole scratch `src` was compared identical at the end, and both directories were deleted. Probes and outputs are in this session's scratchpad (`p8/`). `cargo test` in the worktree reran the corpus identity test, which rewrites `target/f216-runs/corpus-identity.txt`.

**Author / reviewer**: Claude implementer agents wrote both rounds from the lead's two rulings. The reviewer is a Claude agent invoked by the lead. It wrote none of the reviewed material and none of passes 6 or 7.

**Result**: 0 defects, 0 smells, 5 nitpicks

## Acceptance challenged

- **Pass 7 D1 replayed through the release binary** (`starve.py`, unchanged from pass 7): two clients loop two different 4740 by 5928 previews and read every reply. A third client's 512 by 512 preview answered **200 in 0.03 s**, where pass 7 measured 503 at 60.01 s. The loops were served 44 and 43 times at a median of 47 ms. End census: `grey` 0, `grey_replies` 0, `inflight` 0. Closed.
- **Pass 7 D2's second oversized case, through the server** (`order.py`, three rounds): a non-reading client holds a 3000 by 2900 grey. Then, 0.25 s apart, oversized held preview O1, oversized held preview O2, a 512 by 512 preview S and an oversized body-form preview O3. Nothing answered while the stall held. After it closed, the completion order was O1, O2, S, O3 in all three rounds, every one 200. So a second oversized preview no longer passes the first, and a small one does not pass an oversized one that waited before it, nor is passed by one that came after. The gate-level form is `a_second_grey_over_the_cap_does_not_pass_the_queue`, and mutation A (no head rule) turns it red. Closed.
- **The head of the queue vanishes** (`vanhead.py`): with a normal stall held, an oversized held head, an oversized body-form head, and a normal head behind 12.7 MB of stalls each had a small preview queued behind them. The small preview was unanswered while the head waited, and was served **11.8 ms, 3.0 ms and 34.2 ms** after the head's client closed, with the stall still held. That bounds the gone-client path by `LIVE_POLL`.
- **Concurrent mixed sizes with vanishing clients** (`chaos.py`): 8 clients by 25 requests over five sizes (4740 by 5928, 3000 by 5801, 3000 by 2900, 2000 by 2000, 512 by 512), a quarter body form, 15 % closed before reading and 7 % closed after the first 4 KiB. Seeds 1, 2, 3: 158, 161 and 158 reads, every one 200, no thread hung, slowest read 226 ms. A 16 by 40 variant (seed 7): 498 reads, every one 200, slowest 479 ms. Sampled grey peak in every run was 28,098,720, exactly one oversized frame. End census in every run: `grey_replies` 0, `inflight` 0, and `grey` either 0 or a kept grey under the cap (8,962,144 and 4,000,000).
- **A stall plus an oversized head** (`stallhead.py`): one client that never reads holds 8.7 MB, half the cap. An oversized preview queues, then a small one that would fit beside the stall. The oversized preview answered 200 at 60.05 s, and the small one **200 at 59.54 s**. That is the ruling working as stated (the oversized frame draws alone and waiters are served in order). The documents' wording is N2.
- **Deadlock search by reading**: a refused `admit` takes its ticket and drops any claim it carried, so no queued request holds room. A waiter holds no decode permit. Workers never block on the gate and trim their kept grey within `WAIT` (50 ms) while the queue is not empty. A head that is admitted holds room while it waits for a permit, and permit holders only decode and render. A body-form waiter keeps a body slot, and the head never needs a second one. No cycle was found and none was seen in any probe.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo fmt --all --check` | worktree at 171804b, `CARGO_TARGET_DIR=<wt>/target` | now | exit 0 |
| `cargo clippy -p ocelli-folder-viewer -p ocelli-pixel --all-targets -- -D warnings` | same | now | exit 0 |
| `cargo test -p ocelli-folder-viewer -p ocelli-pixel` | same | now | exit 0. viewer lib 63, main 0, body_form_budget 1, preview 59, scheduled_decodes 1, stalled_reader 1, pixel lib 14, color 15, image_plane 4, lut_chain 18, modality 5, palette 7, stored_pixel 2, voi 6 |
| `bin/ocelli.sh gate guards` | same | now | exit 0, ALL GREEN |
| `bin/ocelli.sh gate bench` | same | now | exit 0, ALL GREEN. 146 tests, 144 pass, 0 fail, 2 skipped |
| `guard_census.py`, `--check-runbook` | same | now | exit 0 each, `0 watched by nothing` |
| `prose_check.py` | same | now | exit 0, 840 files |
| `source_provenance_check.py` | same | now | exit 0, 1677 files |
| `cargo build --release -p ocelli-folder-viewer` | same | now | exit 0, the binary every probe used |
| `starve.py`, `order.py`, `vanhead.py`, `chaos.py` seeds 1 to 3, `chaos16.py` seed 7, `stallhead.py` | `target/release` at HEAD, load 6 to 9 | now | see above |
| `p9_probe.py` (implementer's, rerun) | same | now | 47.8 / 24.2 ms, small served 1.4 ms after release, two together peaked 28,098,720, twenty small previews all 200 with the slowest 40.6 ms, 8192 by 8192 RSS 612,663,296. Matches the record's 49.8 / 25.5 / 3.8 / 42.0 ms and 612,630,528 |
| `a_kept_grey_holds_room_until_it_is_freed`, 6 parallel loops of 50 | HEAD's lib test binary | now | 300 of 300 green (N3) |
| `bin/ocelli.sh bench viewer.resident_memory --no-build`, once | `target/release`, load 8.3 | now | 430,899,200 against 528,194,192, `thresholds_met` true, `profile: release`, heap 98,123,775, `held_peaks.workspace` 17,400,000 |
| implementer records `p9-mut-Q1..Q8`, `p10-mut-K1..K3`, `p9-probe.txt`, `p9-resident-r*` | records | prior | consistent with my mutations and reruns |

Mutations, each with a count-of-one assertion on the old text, then `cargo test -p ocelli-folder-viewer --lib --no-fail-fast -- --skip corpus_file` (61 tests, the two corpus tests need `corpus/data`), then restored:

| Mutation | Result |
|----------|--------|
| A, `take_turn` admits any queued ticket, not only the head | **red**, 7: the three gate order tests, the admission wake test, both starvation tests, the twenty-stall test |
| B, `wait_turn` does not wake the others after an admission | **red**: `an_admission_at_the_head_wakes_the_next_without_the_poll` |
| C, `wait_for_grey_until` never checks the client | **red**: `a_wait_that_fails_releases_its_samples_and_its_wait` |
| D, `LIVE_POLL` 20 s | **red**: the same test |
| E, `GreyWaiter::drop` does not wake | **red**: `a_waiter_that_leaves_wakes_the_next_without_the_poll` |
| F, a request that holds room is held to the queue rule | **red**: the gate test and both starvation tests |
| G, the frame gone after the wait leaks the claim (`forget`) | **red**: `a_frame_gone_after_the_grey_wait_releases_its_room` |
| H, a refused render leaks its claim before the settle | **red**: three, `a_refused_render_after_the_grey_wait_releases_its_room` among them |
| I, a body-form wait failure keeps its inflight charge | **red**: `a_wait_that_fails_releases_its_samples_and_its_wait` |
| J, a written reply is kept while a request waits | **red**: `a_written_reply_is_kept_only_while_nobody_waits` |
| K, `used` leaves out the kept greys | **red**: three, the new `a_kept_grey_holds_room_until_it_is_freed` among them |
| L, `wait_turn` drops the lock and sleeps instead of waiting on the condvar | **red**: both 20 s poll tests |
| N, `return_reply` keeps a grey over the cap | **red**: two |
| O, `trim_returned` never frees | **red**: the new kept-grey test and `returned_buffers_make_room_for_a_waiting_grey` |

Environment: Apple M4 Max, macOS 27.0.0 kernel, Python 3.14.5, load 6 to 9 with 11 users. Every viewer I started ran with `OCELLI_PORT=0` and was stopped by `Popen.kill` on its own PID, or by the bench runner. At the end the only viewer process was PID 98891, the operator's debug viewer, which was not signalled. Ports 8742 and 8765 were not used. No oracle or GPU job was run.

## Pass 7 and pass 6 findings replayed

| Finding | Status | Replay and adjacent case |
|---------|--------|--------------------------|
| Pass 7 D1, oversized stream starves a small preview | **closed** | `starve.py` 200 in 0.03 s. Adjacent: `order.py` arrival order, `chaos.py` mixed sizes |
| Pass 7 D2.1, "adds that one frame's grey" | **closed** | gone from all four places. The new sentence makes no figure, and 612,663,296 against 528,194,192 for one 8192 by 8192 frame bears it out |
| Pass 7 D2.2, a second oversized preview passes the first | **closed** | gate test plus mutation A red, server order O1 then O2 three times |
| Pass 7 N1, waiter drop wake untested | **closed** | mutation E red under the 20 s poll test |
| Pass 7 N2, `peak` in production | **closed** | `GreyState.peak` and the body of `note_peak` are `#[cfg(test)]` |
| Pass 7 N3, cache.md wrapping | **reopened in a new line**, N1 below |
| Pass 6 S1, two terms for one set of bytes | **closed** by the lead's ruling and the code. `used = replies + returned`, so a kept grey refuses admission (mutation K red). `WORKSPACE_GREYS` is now 6 native and scratch greys per workspace, 12 for two, plus the cap's 2 for replies and kept greys, 14, matching the formula's 14 for 16-bit raw once the parse copy is left out. The 3000 by 2900 test frame is 16-bit (`grey_file` doc) and `Stored` is an `f32` |
| Pass 6 S2, three release paths without tests | **closed**. Each has a named test: frame gone after the wait (`a_frame_gone_after_the_grey_wait_releases_its_room`, mutation G red), the refused render after the wait, held and body form (`a_refused_render_after_the_grey_wait_releases_its_room`, H red), and the body-form wait failure (`a_wait_that_fails_releases_its_samples_and_its_wait`, I red). The body-form refused render now goes through the shared `render_capped`, so H covers both forms |

## Defects

None.

## Smells

None.

## Nitpicks

- **N1** docs/lld/cache.md:196 is 115 characters in a paragraph wrapped at 80 ("grey counts for as long as it is kept. A decode thread frees the grey it kept while any preview waits for room, and"). This is the same class as pass 7 N3.
- **N2** README.md:185, use.html:270 and cache.md describe the 60 s wait only "When such clients fill the cap". A single non-reading client that holds half the cap, plus one queued oversized preview, makes every later preview wait for that stall's write timeout. Measured: a 512 by 512 preview answered 200 at 59.54 s (`stallhead.py`), and one sent a little later would have got 503. No sentence is false, since "draws alone" and "served in the order they first waited" together imply it. But a reader of the stall sentence would not work it out, and an oversized series (mammography) is where the two meet. This is for the lead, if a sentence is wanted.
- **N3** `a_kept_grey_holds_room_until_it_is_freed` (schedule.rs:844) has a window it does not close. After `call_worker` returns, worker 0 runs `trim_returned`, and it runs it again on each 50 ms idle timeout. If either sees the waiter between the `admit` refusal and the `gate.held() != (0, cap)` check, the worker frees the grey and the check fails. It was not observed: 300 of 300 green in six parallel loops. Checking `held()` before the `admit` would not close it either, because the refusal itself creates the waiter.
- **N4** README and use.html: "Previews that wait for room are served in the order they first waited". The code orders admission. Two admitted previews can be written in the other order (a small one admitted second can finish first), and a waiter that leaves lets the next one past. The next sentence states the exact rule, so this is wording only.
- **N5** `.claude/plans/F-216-design.md:506`, in the ruling after review pass 6, still says "The documented resident bound adds that one frame's grey above the design term", with no note that the pass 7 ruling (:513) replaces it. The pass 6 amendments mark what they supersede, so this is a record gap and not a code or user-document one.

## Verified clean

- **Ticket and refusal share one lock**: `admit` calls `join_queue` inside the guard that decided `fits` was false, then drops the guard and the claim. `enter_wait` is test only, so no other door joins the queue.
- **Head-only admission**: `take_turn` requires `queue.front() == ticket` and `size_fits(used, len)`, and pops under the same lock (mutation A red). A request that is not waiting enters only with an empty queue (`fits`, Q1 in the records). A room holder bypasses only the queue check, and the size check still applies.
- **A room holder is never queued in practice**: its `admit` charges `max(kept, len)` against `used - claim - kept`, which is at most `used`, and so fits the cap. An oversized holder was admitted at `used == 0`, and nothing can enter beside it, so its second round always fits. The `Err` arm of a room holder's `admit` is unreachable by this argument. If it were reached, the request would requeue at the back and answer 503 at the end of its rounds, never deadlock.
- **Every exit leaves the queue and wakes**: admission pops and `wait_turn` notifies (B red). Deadline, gone client, the impossible `Hit` arm and an unwind all drop `GreyWaiter`, which removes the ticket and notifies (E red). A job that never runs drops its closure and the claim inside it (`a_refused_decode_slot_after_the_grey_wait_releases_its_room`). The release profile is `panic = "abort"` and the lints deny `unwrap`, `expect` and `panic`, so a render panic ends the process rather than leaving queue state behind. In tests an unwind drops the place.
- **No lost wake**: `wait_turn` checks and waits under one guard. Every place that lowers `used` or changes the head notifies after its own locked update: `release_replies`, `release_returned` (when nonzero), `admit` (when `after < before`), `settle` (when it freed bytes), the waiter drop and an admission. `return_reply`'s Keep arm moves bytes between counts and leaves `used` alone. L (sleep without the condvar) is red.
- **`LIVE_POLL` only for gone clients**: the slice is `min(deadline - now, LIVE_POLL)`, and admission comes from wakes (the 20 s poll tests pass on HEAD, and B, E and L make them fail). C and D are red on the gone-client test, and `vanhead.py` measured 3 to 34 ms.
- **Kept greys under the cap**: `used` counts `returned`. While a kept grey is in transit, the worker's old buffer and the new one both count until `restore_grey`'s job runs, so `used` never rises from keeping. A grey over the cap is freed after its write (N red), and an oversized buffer left by a render is freed before the settle. The kept grey test goes red under K and O.
- **Prose executed**: every new sentence in README.md:185, use.html:270 (the same paragraph as the README, compared mechanically) and docs/lld/cache.md:191-220. That covers the cap as the grey term, counting from admission to write, kept greys counting while kept, arrival order, admission at the head when the grey fits, at once only when the queue is empty, "already holds room still renders and is never queued", "every way out of the wait", the bound stated for frames within the design size and above it for larger frames, and the four measured figures (re-measured 47.8, 24.2, 1.4 and 40.6 ms, and a 28,098,720 peak). The `GREY_CAP`, `GreyGate`, `GreyState.queue`, `GreyWaiter`, `fits`, `reserve`, `take_turn`, `wait_turn`, `admit`, `enter_wait`, `CongestedWork.place`, `Capped::Full`, `wait_for_grey` and `LIVE_POLL` comments all match the code. N4 is the one loose word.
- **`WORKSPACE_GREYS` arithmetic**: 2 native plus 4 scratch greys per workspace, 12 in all, plus the cap's 2, which is 14. The test checks `census.workspace`, which is `workspace_bytes` (scratch times `size_of::<Stored>()`, plus native, plus grey) per worker plus detached reply greys, against `14 * grey_len`, and it is green.
- **Stale wording**: `git grep` finds no "held back", "holds no room yet", `oversized_waiting`, `wait_slice` or `GreyRefusal` outside the plan's dated rulings and the review files.
- **Memory**: one `viewer.resident_memory` run met at 430,899,200, `profile: release`.
- No `as` cast, `#[allow]` or `unsafe` was added. `next_ticket` uses `wrapping_add` on a `u64`. The new test helpers use `try_from` and saturating arithmetic.

## Limitations and handoff

- Not exercised: Chromium driving an oversized series, a non-loopback network, compressed oversized files, `OCELLI_CACHE_MIB` below an oversized file's size, and the unreachable room-holder refusal arm (argued, not constructed).
- The timing probes ran at load 6 to 9 alongside other work. They are indicative, not a baseline, and no baseline was written.
- This is a bounded review of the range above. It is not acceptance and not sprint readiness.
- Next owner: the lead for N2 (whether the stall sentence should name the oversized case). The rest is optional tidying for implementation.
