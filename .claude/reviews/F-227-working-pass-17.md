# F-227 review, pass 17

**Reviewed**: `git diff 0767db58..85bd4672` on branch `work/f-227-xai`, HEAD `85bd4672`, tracked tree clean (untracked `.venv` and `tools/oracle/out` only). The viewer half's pass 16 remediation: 13 files, 417 insertions, 110 deletions. The lead rulings after pass 16 sit in `0767db58`, the base, and are read as the contract, not reviewed.
**Author / reviewer**: the remediation is by a Claude implementer, per the commit message and `.claude/scratch/F-227-progress.md`. This pass is by an independent Claude reviewer who authored none of the reviewed material, none of the earlier remediation and none of the rulings.
**Result**: 1 defect, 2 smells, 4 nitpicks

## Acceptance challenged

- **Pass 16 D1 to D4 and N1 to N3, replayed through the real caller.** auto.mjs, the HTTP route and the page, on the HEAD release binary. Each is closed, see Verified clean.
- **The lead's challenge, the hold beyond the ruling.** Three route variants were built in a scratch copy and driven through auto.mjs's own `observeEvicted` and an HTTP driver that follows `askSeriesWindow`'s rule.
  - V0 is HEAD, with a probe that records the bytes `locate` returned and, before and after each `load`, how many of the not yet loaded ones have left the LRU.
  - V1 drops the hold: `locate` only checks presence, `load` looks the id up again, decodes through `decode_for_value` and stores.
  - V2 drops the hold and the store: `load` looks the id up again and decodes on a decode permit with a buffer claim that makes no room and no `store_frame`, as a body preview does.
  - The plausible wrong implementations rejected: V1 is what the ruling literally asked for, and it loses the series window. V2 shows the hold is not the only way to converge.
- **Termination of the 410 rule, derived independently.** Every id a 410 names was in that request's body, because the route names only parsed ids and `FileId::parse` then `header_value` round trips (lowercase hex, no leading zero). A body id is an original id from `heldId`, an id a repost returned, which `posted` always records when it is not empty, or `-`. So a round that continues names an original id still in `lines`, and that round replaces it. Original ids never return to `lines`, so the count of original sampled ids left falls by one or more each continuing round, and there are at most 16 sampled ids. At most 17 requests answer 410. Observed: V1, where a sampled file is evicted every round, stopped after 5. HEAD under a 1 MiB cache, where the 16 sampled files cannot all be held, stopped after 3.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo test -p ocelli-folder-viewer` | worktree HEAD, worktree target | now | 0. lib 91, preview 116, value 24, measure 16, the rest green |
| `cargo clippy -p ocelli-folder-viewer --all-targets -- -D warnings`, `cargo fmt --all --check` | worktree HEAD | now | 0 and 0 |
| `scripts/prose_check.py`, `scripts/source_provenance_check.py`, `scripts/guard_census.py` | worktree HEAD | now | clean over 987 and 1871 files, census prints only declared LIMIT rows |
| `scripts/guard_probe.py --only viewer-browser.auto-per-slice` | worktree HEAD | now | red for its declared reason, control green |
| `node examples/folder-viewer/tests/browser/auto.mjs` | worktree HEAD, release | now | 0. restart `200, 410, 200`, 4 MiB cache `410, 200`, changed `2 marked, 200`, every observed slice `Auto window` |
| `p17.mjs` 4 MiB eviction, 3 runs each, V0, V1, V2 | scratch, own target, release | now | V0 `410, 200` 3 of 3, 0 bytes outside the LRU. V1 `410` five times, then slices at WW 44.5, 66.7, 89.0, 111.2 with `Auto window, this image`, 3 of 3. Its gone ids at load were all located as `Hit`, frame only ids whose posted bytes `take_encoded_if_ready` had dropped. V2 `410, 200` 3 of 3, WW 114.74987 as V0 |
| full auto.mjs against V2 | scratch, release | now | 0, every printed value identical to HEAD |
| `memprobe.py` V0 over `corpus/data/real/ct_cmb_mml`, 27 files, cache 256, 8, 4, 3, 2 MiB | scratch V0, corpus, HTTP | now | 16 located, 3,767,928 bytes held, 0 bytes outside the LRU at every budget. At 3 and 2 MiB `410, 410, 410` then stop. CT stores windows, so the round ends 400 after pooling |
| `memprobe.py` over 40 synthetic RLE PT files of 384 to 696 square, 6 to 10 KB each | scratch, written by pydicom into the scratch folder | now | V0 at 3 MiB: 200, 122,900 bytes outside the LRU at peak. At 2 MiB: 200, 145,544 bytes. V1 at 3 and 2 MiB: 410 rounds, the window lost. V2 at 3 and 2 MiB: 200 at once, the same centre 119.49994 and width 38.999634 as V0 |
| `node tools/bench/run.mjs viewer.resident_memory --no-build --out <scratch>` | worktree HEAD, release | now | 445,333,504 bytes, `thresholds_met` true. The runner opens only the CT and DX workloads, which store windows |
| Mutant MA, `series_window` called on every `/preview` carrying a series header | scratch | now | `the_series_window_is_not_recomputed_per_preview` FAILED, `three previews made 4 pools` |
| Mutants N2 (function check removed), FIRST (route stops at the first gone id), DASH (a `-` line not counted skipped) | scratch | now | each 101, red in `the_series_window_refuses_conflicts` or `a_mixed_series_skips_windowed_images` |
| Mutant P1, the `posted.add(id)` line removed from the page | scratch, release | now | auto.mjs 0, `the_page_draws_a_series_at_its_automatic_window` ok. Under a 1 MiB cache with auto.mjs's 40 file series, 2,263 requests in 15 s, all 410, against 3 for HEAD |
| Red runs on `0767db58` for the new restart, eviction and changed cases | implementer | prior receipt | progress note: restart `200, 410, 410`, 4 MiB `410, 410`, changed with no request |
| pass 16 `mono1.py`, MONOCHROME1 background frame | pass 16 reviewer | prior receipt | `[255, 255, 255, 255]` against `[0, 0, 0, 0]` on MONOCHROME2 |

## Defects

### D1, "cannot evict a sampled file not yet read" is false

**Where**: `docs/lld/pixel-pipeline.md:254-255`, "The bytes a lookup found stay alive until the pool is made, so a frame stored during it cannot evict a sampled file not yet read", and the same claim in `examples/folder-viewer/src/series_window.rs:68-71`.
**What**: the hold does not stop eviction. A sampled file located but not yet read still leaves the LRU while the round decodes, through the buffer claim's `make_room_dropping` and the store's `reserve_for_insert`, neither of which passes over a posted file. What the hold guarantees is that the route still reads those bytes, because its own `Arc` keeps them alive. They are then outside `OCELLI_CACHE_MIB`.
**Why it is wrong**: microscope section 3, a false sentence in an LLD and in a doc comment. It also hides the cost S2 describes, since a reader is told nothing leaves the cache.
**Evidence**: V0's probe, 40 RLE files under a 2 MiB cache: 145,544 bytes of located, not yet loaded posted files absent from the LRU at peak, and the round still answered 200.
**Scope and regression**: prose only, present on HEAD. Fix by stating what holds, for example "the route keeps the bytes each lookup found until the pool is made, so a file evicted during the round is still read. Those bytes are then outside the cache budget", or by removing the hold (S2, option 2) and the sentence with it.

## Smells

### S1, nothing executes the 410 stopping rule

**Where**: `examples/folder-viewer/src/page.html:2388` and `:2394`, the stop test and `posted.add`.
**What**: the ruling requires that the loop terminates. No test reaches the branch that returns `null` on a 410, because every 410 in auto.mjs is followed by a 200. The page source test checks the stop line's text but not that `posted` is filled.
**Failure mechanism**: mutant P1 removes `posted.add`. Every suite stays green, and when the sampled files exceed the budget the page sends `/series-window` without end, 2,263 requests in 15 s under a 1 MiB cache.
**Smallest regression**: auto.mjs's 40 file series under `OCELLI_CACHE_MIB=1`, requiring at most 17 `/series-window` replies and `Auto window, this image` on the first slice. It goes red on P1.

### S2, the hold puts sampled bytes outside the cache budget with no accounting

**Where**: `examples/folder-viewer/src/series_window.rs:103` (`found` keeps every `locate` result until its `load`), driven by `serve.rs:1882-1900`.
**What**: one `/series-window` request keeps up to 16 posted files or decoded frames alive until each is read. An entry evicted meanwhile stays in memory outside `OCELLI_CACHE_MIB`. Nothing reports it, since `GET /held` `used` drops it. F-216's bound `B + 4 S_max + P (2 S_max + 5 N_max) + 32 MiB` has no term for it, and `viewer.resident_memory` never runs the route.
**Failure mechanism**: for a no-window series of files near F-216's design size, 17,400,516 bytes, 16 held files would be 278 MB beyond `B` per request. That figure is arithmetic on the design size, not a measurement. The largest measured excess is 145,544 bytes on the synthetic RLE series.
**Disposition**: correct today and beyond the ruling. The lead rules, with the options in the answers below.

## Nitpicks

- **N1** `auto.mjs:28-33`. The header says the script starts the viewer with `OCELLI_PORT=0` and stops it by its PID, while `observeRestart` starts the second process on the first one's port and the new cases stop through `stopViewer`. The list of wrong pages rejected does not name the restart, eviction or changed cases.
- **N2** `series_window.rs:60`, `POOLS` is one static for the whole test binary. The AC14 test reads a delta, which a future unit test that pools in parallel would disturb. One test pools today, so it is not flaky now.
- **N3** `page.html:2341-2343`, `heldId`'s comment names a changed file and one the process would not hold. A file whose last re-post failed has no `filePosts` entry either, so it is sent as `-` and skipped rather than posted.
- **N4** `docs/lld/pixel-pipeline.md:255` runs to about 140 columns, unwrapped beside its 79 column paragraph.
- **Carried**: pass 15 N1 and the earlier library nitpicks, unchanged.

## Answers to the lead's questions

**(a) Are the route's stores needed, or would not storing converge without the hold?** Not storing converges. V2 has no hold and no store, and it answered `410, 200` in 3 of 3 runs of auto.mjs's 4 MiB case, passed all of auto.mjs, and answered 200 at once on the RLE series where V1 lost the window. V1 is the ruling as written, two passes with stores and no hold, and it lost the window in every run. Its gone ids in the auto.mjs case were frame only ids evicted by the round's own decodes. The cost of V2 is a second decode later for each sampled image that is then shown, slice 1 included, and no stored frame from the round. V2 is a scratch proof on the scheduled path only, not a proposed patch.

**(b) Peak extra memory on the largest series in the corpus.** The largest by bytes is `real/ct_cmb_mml`, 27 files, 6,358,380 bytes, the largest file 235,496. Driven over HTTP, the hold kept 16 files, 3,767,928 bytes, alive during the round, and none of them left the LRU while held at 256, 8, 4, 3 or 2 MiB. So the measured extra memory beyond the budget is 0 bytes. That series stores windows, so the page never asks for it. The only corpus series the page would ask for is `synthetic/ct_multiframe_perframe.dcm`, one file of 3,466 bytes. The mechanism is real outside the corpus: 122,900 and 145,544 bytes on the synthetic RLE series at 3 and 2 MiB. F-216's threshold holds at HEAD, 445,333,504 bytes against the recorded 445,956,096 with tolerance 0.12, 499,470,827, and the 528,194,192 byte bound. But that runner opens no no-window series, so it does not measure the hold.

**(c) Does a doc now state something false about what the budget covers?** Yes, D1, in `pixel-pipeline.md` and the `series_window.rs` doc comment. `docs/lld/cache.md` says nothing false. Its folder viewer section names one pool outside the budget, the kept inspect replies, at line 196, and says nothing of the hold, and README line 193 says the same. Both are silent rather than false. F-216's formula in `.claude/plans/F-216-design.md` claims a bound for the page's own request pattern, which now includes this route, and has no term for it.

**Options for the lead.**
1. Keep the hold and account for it. Add a term of up to 16 times the larger of `S_max` and one frame's samples per `/series-window` in flight to F-216's bound, name it in cache.md and README beside the kept pool, consider reporting it in `GET /held`, and add a no-window workload to `viewer.resident_memory`.
2. Drop the hold and the store, V2. Keep the two passes and the full gone list. Accept a second decode of each sampled image when shown. D1's sentence goes away.
3. Drop the hold and keep the stores, V1. Rejected by measurement, it loses the window.

**Plan observation, outside the range.** The ruling's "a series window is lost only when a file cannot be posted at all" is not what HEAD does when the 16 sampled files exceed the cache. Under 1 MiB with auto.mjs's 40 file series, and under 3 or 2 MiB with `ct_cmb_mml` over HTTP, every file posted and the window was still lost after three rounds. That is the stopping rule doing its job. The sentence is the lead's and no user document repeats it.

## Verified clean

- **Pass 16 D1 closed.** Restart: `200, 410, 200`, four slices of series B at one `Auto window`. 4 MiB: `410, 200`, four slices at WW 114.74987 `Auto window`. The route names every gone id in sample order (`{"gone":[...]}`, `X-Ocelli-Error: file-gone`), FIRST goes red. The page re-posts exactly the named files, once per file for a multiframe, through `repost`, which deletes the stale `filePosts` entry only when it is the gone id.
- **Pass 16 D2 closed.** Two files of series B marked by F-224's probe, index 4 outside and index 5 inside `series_sample(20)`, which I computed as 0 to 3, 5 to 8, 10 to 13, 15 to 17 and 19. Reply `sampled 16`, `skipped 1`, one window on the slices. No file is posted to ask, the body carries held ids and `-`. DASH goes red. The HTTP case's hand values hold: stored 1 to 100, n = 100, ranks 1 and 99, centre 50, width 98, reply `49.99994` and `97.99988`, within the 16 unit margin.
- **Pass 16 D3 closed.** `POOLS` is incremented inside `series_window` after the pool is built, and MA now goes red.
- **Pass 16 D4 closed.** more.html and pixel-pipeline.md say output minimum, black on MONOCHROME2 and white on MONOCHROME1, citing PS3.3 C.7.6.3.1.2, which matches pass 16's measured `[0, 0, 0, 0]` and `[255, 255, 255, 255]`.
- **N1 to N3 closed.** `seriesHeader` returns `""` for an image that is not `autoEligible`. A series window with `X-Ocelli-Window-Function` is refused, mutant N2 red, and the doc comment of `validate_frame_request` says so. use.html's example checks `opened.photometric()`, which is `OpenedFrame::photometric` returning `ocelli_pixel::PhotometricInterpretation` with variants `Monochrome1` and `Monochrome2`.
- **Termination**, by the argument above and by the observed stops.
- The other new prose: pixel-pipeline.md's line and `-` sentences, the gone list sentence and the "as long as each round names an id it has not posted" sentence. serve.rs's route comment, `SeriesRefusal::Gone`'s order, `parse_series_lines`' doc, auto.mjs's 128 KiB and "more than a 4 MiB cache holds" (40 times 131,072 bytes, and round 1 found 12 of 16), benchmarks.md's new sentence, and the catalogue and runbook rows, whose three new refusal functions exist and set exit 1.
- No `as` cast, no `unsafe` and no `#[allow]` added. No pixel arithmetic changed. The pool, `contribute` and the window are unchanged apart from the two pass split.
- The oracle is untouched. No file under `tools/oracle` is in the range.

## Limitations and handoff

- Timing was not compared between V0 and V2, so V2's extra decodes are named, not measured. AC17 stays with the sprint lead on an idle host.
- The RSS peaks `memprobe.py` printed, 14 to 36 MB, come from one sample every 5 ms on a busy host. They are informational and were not used for any answer.
- V2 changes only the scheduled decode path, so it was run only as the release process server, not through the unscheduled `serve_on` the integration tests use.
- Not exercised: several `/series-window` requests in flight at once from a four cell layout, a multiframe sampled more than once under eviction, and a network failure during a re-post.
- AC16 needs the operator's study and was not run.
- No GPU, wasm or oracle job, by instruction. Scratch work lives under `scratchpad/f227-p17` and is deleted with Python `shutil` after this file is written. Every viewer this pass started was stopped.
- Next owner: the lead rules on S2 first, because its option decides how D1 is fixed. Then the implementer fixes D1 and S1, and a fresh independent pass reviews the result.
