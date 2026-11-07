# F-216 review, pass 7

**Reviewed**: `work/f-216-xai`, `git diff f2fe29e 6ed89e7`, HEAD `6ed89e79b7125b1e26be2fcdb2b92d4baaf2a576` ("F-216, a frame over the design size draws alone"). Five paths: README.md, docs/lld/cache.md, held.rs, serve.rs, use.html. The tree was clean apart from the untracked `.claude/reviews/` passes 1 to 6 and `tools/oracle/out`, which were not reviewed. `.claude/scratch/F-216-progress.md` (last section) and `target/f216-runs/p8-*` were read as records. This report is the only file this pass adds. Mutations ran in a copy of HEAD at `target/p7rev-scratch` with `CARGO_TARGET_DIR=target/p7rev-target`. Every mutated file was restored and checked identical to the worktree, and both directories were deleted at the end. Probes and their outputs are in this session's scratchpad (`p7/`). `cargo test` reran the corpus identity test, which rewrites `target/f216-runs/corpus-identity.txt`.

**Author / reviewer**: a Claude implementer agent wrote this round from the operator's answer after pass 6. The reviewer is a Claude agent invoked by the lead. It wrote none of the reviewed material and none of pass 6.

**Result**: 2 defects, 0 smells, 3 nitpicks

## Acceptance challenged

- **The operator answer, drawn**: a 4740 by 5928 frame answered 200 with 28,098,720 grey bytes, fresh 47.2 ms and cached 24.3 ms (implementer probe rerun). 8192 by 8192, exactly `MAX_PIXELS` (67,108,864), answered 200 held, held again and body form. 8192 by 8193 answered 400 `too-large`. So "no frame is refused for its size below the limit" holds at the default config.
- **Deadlock attempts, all through the release server**:
  - Two oversized frames at once, one of them stalled, plus a small preview and an oversized body-form preview waiting behind it. Closing the stall served all three (200 each, about 1.05 s after the first was sent). End census: `grey_replies` 0, `inflight` 0.
  - An oversized waiter whose client vanishes. A normal stall held 8.7 MB and the oversized preview waited. A small preview that fits beside the stall was held back, and was still unanswered after 1.0 s. Once the oversized client closed, it was served within 35.4 ms (held form) and 31.8 ms (body form). So the guard drop ends the hold-back on the disconnect path.
  - An oversized preview behind a 60 s stall answered 200 at 60.04 s. The stall was admitted first, so its write deadline always ends before the waiter's.
  - A request already holding room renders while an oversized frame waits. Mutation Mb is red.
  - No hang was seen anywhere. Workers never block on the gate. A waiter holds no permit, and idle workers trim their kept grey every 50 ms (`worker_loop`, `trim_returned`).
- **Starvation, the other way round**: two healthy clients previewing two different oversized frames in a loop, with each reply read fully. A third client's 512 by 512 preview answered **503 after 60.01 s**, while the two loops were served 1,335 times each with a median of 46.3 ms (D1).
- **The resident sentence**: one sequential client posted an 8192 by 8192 file and previewed it held and then in body form. RSS was then 612,630,528, above the design bound plus one grey (528,194,192 + 67,108,864 = 595,303,056). With the cache filled and four 4740 by 5928 body-form previews waiting, RSS was 626,769,920 against 556,292,912 (D2).

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo fmt --all --check` | worktree at 6ed89e7, `CARGO_TARGET_DIR=<wt>/target` | now | exit 0 |
| `cargo clippy -p ocelli-folder-viewer -p ocelli-pixel --all-targets -- -D warnings` | same | now | exit 0 |
| `cargo test -p ocelli-folder-viewer -p ocelli-pixel` | same | now | exit 0. viewer lib 56, main 0, body_form_budget 1, preview 59, scheduled_decodes 1, stalled_reader 1, pixel lib 14, color 15, image_plane 4, lut_chain 18, modality 5, palette 7, stored_pixel 2, voi 6 |
| `bin/ocelli.sh gate guards` | same | now | exit 0, ALL GREEN |
| `guard_census.py`, `--check-runbook` | same | now | exit 0 each, `0 watched by nothing` |
| `prose_check.py` | same | now | exit 0, 840 files |
| `source_provenance_check.py` | same | now | exit 0, 1677 files |
| `p8_large_probe.py` (implementer's, rerun) | `target/release` at HEAD, load 5.2 | now | 47.2 / 24.3 ms, small served 4.1 ms after release, two together peaked 28,098,720. Matches the record's 45.6 / 23.9 / 3.1 ms |
| `starve.py` | same | now | small preview 503 at 60.01 s. Loops 1,335 + 1,335 at 200 (D1) |
| `vanish.py` A, B, C | same | now | see above. Every end census had `grey_replies` 0 and `inflight` 0 |
| `rss.py`, `rss2.py`, `maxpx.py` | same | now | 457.1 MB with four oversized body waiters and an empty cache. 626.8 MB with a full cache. 612.6 MB after one 8192 by 8192 post, held preview and body preview (D2). `inflight` 449,581,432 with four waiters |
| gate unit check `p7_second_oversized_jumps_the_waiter` (scratch only) | scratch copy | now | `enter_wait(150)` then `reserve(150)` admitted the second grey (D2 part 2) |
| `viewer.resident_memory`, once, `--no-build` | `target/release`, load 6.6 | now | 451,411,968 against 528,194,192, thresholds met, `profile: release`, heap 98,123,815 |
| implementer records `p8-mut-M1..M5`, `p8-resident-r1..3`, `p8-viewer.*` | records | prior | consistent with my mutations and the bench run |

Mutations, each with a count-of-one assertion, then the whole `cargo test -p ocelli-folder-viewer --no-fail-fast`, then restored:

| Mutation | Result |
|----------|--------|
| Ma, delete the hold-back in `fits` | **red**: the unit hold-back test and `normal_previews_do_not_starve_a_waiting_large_frame` |
| Mb, `admit` passes `holds_room = false` | **red**: the unit hold-back test only (no server test covers it) |
| Mc, `GreyWaiter::drop` does not `notify_all` | green (N1) |
| Md, `GreyWaiter::drop` does not decrement `oversized_waiting` | **red**: two tests |
| Me, `enter_wait` never marks an oversized wait | **red**: two tests |
| Mf, `wait_for_grey_until` calls `enter_wait(0)` | **red**: the server starvation test |
| Mg, `return_reply` keeps a grey over the cap | **red**: two tests |
| Mh, `render_capped` does not free the over-cap buffer before the settle | **red**: `a_failed_render_over_the_cap_keeps_no_buffer` |
| Mi, an oversized grey always fits | **red**: five tests |
| Mj, `len > self.cap` becomes `>=` in `fits` | green. Only a grey of exactly the cap behaves differently, and it fits only alone either way |

Environment: Apple M4 Max, macOS 27.0.0 kernel, Python 3.14.5, load 5 to 10 with 11 users. Every viewer I started ran with `OCELLI_PORT=0` and was stopped by `Popen.kill` on its own PID, or by the bench runner. PID 98891 (the operator's debug viewer) was not signalled. Ports 8742 and 8765 were not used. No oracle or GPU job was run.

## Defects

### D1, a stream of oversized previews holds every other preview back until it answers 503 at 60 s, and no document says so

**Where**: `examples/folder-viewer/src/held.rs:519-526` (`GreyGate::fits`: `!holds_room && state.oversized_waiting > 0` refuses), `held.rs:654-665` (`enter_wait`), `serve.rs:917` (`wait_for_grey_until`). Docs: README.md:185, use.html:270, docs/lld/cache.md:204-207.
**What**: while any oversized grey waits, no request that holds no room is admitted, whatever room there is. With two or more clients previewing oversized frames back to back, one oversized grey is nearly always held and another nearly always waits. A smaller preview that would fit is never admitted, and it answers 503 at its 60 s deadline. Measured with two healthy clients: a 512 by 512 preview answered **503 after 60.01 s**, while the oversized loops were served 2,670 times at a median of 46.3 ms. The hold-back makes things worse than serial admission alone would, because without it a smaller grey could enter in the moments when no oversized grey is held. The page itself requests the current frame, the next two and the previous one at once (`page.html:792`, `schedulePrefetch`), so a single page scrolling a series of oversized frames creates this pressure for its own thumbnails and for any other client.
**Why it is wrong**: the hold-back exists so that "a stream of smaller previews cannot keep it waiting". The rule removes starvation in one direction and creates it in the other. The documents describe a 503 after the write timeout only for clients that stop reading. Here every client reads. The operator answer covers drawing the oversized frame and does not accept, or mention, a minute-long stall for other previews. Like pass 6 D1, this is a conflict inside the design, so the call belongs to the lead.
**Evidence**: `starve.py` (scratchpad `p7/starve.txt`).
**Scope and regression**: any viewer where oversized frames are previewed concurrently, by two clients or by one page's prefetch over an oversized series. Options for the lead: admit waiters in arrival order (a ticket per `enter_wait`, so each direction waits only for those ahead of it), hold back only requests that arrived after the oversized waiter, or accept the behaviour and document it. Smallest regression: two threads looping oversized previews with replies read, then a small preview that must answer 200 within a bound, or 503 if the lead accepts the degradation.

### D2, two new sentences in all three documents are false

**Where**: README.md:185, use.html:270, docs/lld/cache.md:203-209, and the `GreyGate` doc comment at `held.rs:385-389`.
**What**:
1. "The resident bound adds that one frame's grey above the design term." No documented bound was changed. `.claude/plans/F-216-design.md:279`, `tools/bench/src/viewer_work.mjs:102` and `ci/bench-baseline.json` (`rss_bound_bytes` 528194192) all carry the old formula with no such term. The sentence also implies that one grey is the only excess, and it is not. A frame over the design size also raises the `4 * S_max` body term, and while body-form previews wait for the cap they keep their samples (`inflight` 449,581,432 with four 4740 by 5928 waiters). Measured: **612,630,528** RSS after one sequential client posted an 8192 by 8192 file and previewed it held and then in body form, against 595,303,056 (the design bound plus that grey). **626,769,920** with a full cache and four oversized body-form waiters, against 556,292,912.
2. "Once such a frame waits, no preview that holds no room yet is admitted until it has been served" (held.rs: "no request that holds no room yet is admitted"). In `fits`, the `len > self.cap` branch returns `others == 0` before the hold-back check. So a second oversized preview that holds no room is admitted while the first is still waiting. Reproduced at the gate: `enter_wait(150)`, then `reserve(150)` returned `Some`. Through the server, two oversized waiters race at every release, and the clause "so a stream of smaller previews cannot keep it waiting" is the only part that holds.
**Why it is wrong**: microscope section 3. README and `use.html` are always in scope.
**Evidence**: `rss.py`, `rss2.py`, `maxpx.py`, the scratch gate check, and a grep of the three bound sites.
**Scope and regression**: prose only. For 1, either state the bound for the workload's own `S_max` and `N_max` (as the plan defines them, which covers both measurements) and say that only the grey cap stays at the design value, or delete the sentence. For 2, say "no smaller preview". The regression is a re-read, plus a unit test if the lead wants oversized waiters served in order (see D1).

## Smells

None.

## Nitpicks

- **N1** The `notify_all` in `GreyWaiter::drop` ("Requests held back behind this grey may be admitted now") is untested. Mutation Mc is green, because held-back waiters also wake every 50 ms in `wait_slice`. The worst cost of losing it is 50 ms, so it is a latency nicety rather than a guard.
- **N2** `GreyState.peak` and `note_peak` run in production, while only the accessors are `#[cfg(test)]`. The progress note says "`GreyGate::peak` and `reset_peak` are test only", which is true of the accessors and not of the field.
- **N3** docs/lld/cache.md:214 is not wrapped ("peaked at 28,098,720 grey bytes, one frame. A preview whose grey does not fit waits for room for up to the 60" is one long line in a wrapped paragraph).
- Carried and not re-examined: pass 6 S1 (the formula's grey term accounting) and its plausible concern about body-form waiters' samples outside any permit. D2 part 1 is that concern, made larger by frames over the design size and now contradicted by a sentence.

## Verified clean

- **One admission rule**: `reserve` and `admit` both go through `fits`. `reserve` passes `holds_room = false`. `admit` passes `claim.bytes > 0` and charges `max(kept, len)` against `used - claim - kept`. With an oversized grey admitted only at `others == 0`, no other reply claim or returned buffer can exist at its reserve, so its second round always admits. That makes the new `GREY_ROUNDS` sentence correct. Ma, Mb, Mi are red.
- **Waiting count**: `enter_wait(len)` counts `oversized_waiting` when `len > cap`. The guard is a local in `wait_for_grey_until`, so every return (claim, disconnect, deadline) and an unwind drop it. Disconnect measured at 35 ms and 32 ms. Md, Me, Mf are red.
- **Workers never keep an oversized grey**: `return_reply` frees when `claim.bytes > cap` (Mg red, `grey` 0 after every oversized write in every probe). `render_capped` frees a leftover buffer over the cap before `settle` (Mh red). A kept normal buffer only delays an oversized grey until the worker's next 50 ms trim.
- **Cap accounting**: grey bytes peaked at exactly one oversized frame in the two-frame test (gate `peak`, exact) and in the sampled probe. `settle` and `return_reply` never raise `used`, so `note_peak` at `reserve`, `admit` and `count_returned` sees every high point.
- **60 s then 503**: an oversized waiter behind a stall is served at the stall's deadline (60.04 s), not refused. The 503 under an oversized stream is D1.
- **Prose executed**: "draws alone", "admitted only when no other grey is held", "while it is held no other grey is admitted", "stay within the larger of the cap and that one frame", "frees that grey after the write and does not keep it", "a refused render of that size leaves no buffer behind", "A preview that already holds room still renders", "No frame is refused for its size below ... 67,108,864", the four measured numbers (re-measured as 47.2, 24.3 and 4.1 ms, and a 28,098,720 peak), and "The measured workload has no such frame". The false sentences are D2.
- **Memory**: one `viewer.resident_memory` run met at 451,411,968, `profile: release`.
- No `as` cast, `#[allow]` or `unsafe` was added. The new test helpers use `try_from` and `saturating` arithmetic. The six required commands were green.

## Limitations and handoff

- Not exercised: a non-loopback network, Chromium driving an oversized series, compressed oversized files, `OCELLI_CACHE_MIB` below an oversized file's size, and an oversized waiter actually reaching its own 60 s deadline. That last one is hard to construct, because the hold-back stops new greys from extending the stall.
- This is a bounded review of the paths above. It is not acceptance and not sprint readiness.
- Next owner: the lead for D1 (fair ordering, a narrower hold-back, or accept and document). Implementation for D2 (two prose fixes in four places). A fresh pass should replay `starve.py`, the gate check from D2 part 2, and `rss2.py` against whatever the bound sentence becomes.
