# F-227 review, pass 18

**Reviewed**: `git diff e411b7b3..5d4065b8` on branch `work/f-227-xai`, HEAD `5d4065b8`, tracked tree clean (untracked `.venv` and `tools/oracle/out` only). The viewer half's pass 17 remediation, one commit, 7 files, 294 insertions, 62 deletions. The lead rulings after pass 17 in `.claude/plans/F-227-design.md` are read as the contract, not reviewed.
**Author / reviewer**: the remediation is by a Claude implementer, per the commit and `.claude/scratch/F-227-progress.md`. This pass is by an independent Claude reviewer who authored none of the reviewed material, none of the earlier remediation and none of the rulings.
**Result**: 1 defect, 2 smells, 1 nitpick

## Acceptance challenged

- **Pass 17 D1, S1, S2 and N1 to N4, replayed through auto.mjs, the HTTP route and the page** on the HEAD release binary. Each is closed, see Verified clean. S2's replacement, the unstored decode, is challenged below and produces D1 and S1 of this pass.
- **Challenge 1, the `Unstored` owner.** Three questions, each answered by measurement.
  - Aliasing. Series B, 20 files of 512 square, was previewed under a 20 MiB cache, 12 of its ids forgotten so 11 buffers of the frame's exact length sat on the free list (`free` 5,767,168), then series A, values 10 to 59, was pooled by three parallel rounds. All three replies equal a fresh 256 MiB process's (`34.499977`, `48.999924`), a cached B frame's grey hashes the same before and after (`377b5112c04e38c9`), and `used`, `files`, `frames` and `free` are unchanged. The free list holds only owned `Vec`s moved out of a dropped `CachedFrame` or recycled by their last owner, and `prepare_samples` clears and resizes before decode, so a reused buffer cannot alias a served frame or leak its old values.
  - The in-flight charge. `GET /held` `inflight` read 0 after a refused decode (a corrupted RLE file, `skipped 1`), a first pass 410 (`{"gone":["0.1"]}`), a second pass 410 (below), and six clients that closed mid-request (peak 16,777,990 seen while they ran, 0 after). A panic unwind is not reachable in the release binary, `[profile.release]` sets `panic = "abort"`. In a test build `Unstored::drop` runs on unwind, and a panic inside the worker job after `on_samples` would skip the job's release, the same shape as `decode_for_value` and `pooled_body_preview`.
  - Peak memory against `OCELLI_CACHE_MIB`. Measured with a scratch probe that records, at each `Unstored::new`, the live unstored sample bytes, the round's `Stored` scratch and `CacheState::charged()`. It does not stay inside. See D1 and S1.
- **Challenge 2, termination with the second pass gone.** A sampled id forgotten through `POST /forget` 0 to 40 ms after the round began, 20 files of 2048 square, answered `410` naming it and then `200` once the page's rule re-posted it, at every delay. The mutant GONE2 (second pass `Gone` read as `Refused`) answered `200, skipped 1` on the same input, so the 410 really came from the second pass. The termination argument of pass 17 holds unchanged: a second pass gone id is still a parsed body id, so a continuing round still names an original sampled id still in `lines` and replaces it. At most 16 rounds continue, so at most 17 requests answer 410.
- **Challenge 3, the per generation counter.** MA, `series_window` called from `preview_route` for every `/preview` carrying `X-Ocelli-Series-Window`, is red (`three previews made 4 pools`). A scratch test that pools on its own viewer in a loop for 4 s beside AC14, 4 test threads, 3 runs: AC14 green each time at HEAD. With the counter keyed to one constant generation it goes red each time (`one request made 3 pools`).
- **Challenge 4.** F-216's threshold holds, but its runner still opens no no-window series. cache.md and README say nothing false. pixel-pipeline.md and two doc comments do, D1.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo test -p ocelli-folder-viewer` | worktree HEAD, worktree target | now | 0. lib 91, preview 116, value 24, measure 16, the rest green |
| `cargo clippy -p ocelli-folder-viewer --all-targets -- -D warnings`, `cargo fmt --all --check` | worktree HEAD | now | 0 and 0 |
| `scripts/prose_check.py`, `scripts/source_provenance_check.py`, `scripts/guard_census.py` | worktree HEAD | now | clean over 988 and 1872 files, census 0, 0 watched by nothing |
| `scripts/guard_probe.py --only viewer-browser.auto-per-slice` | worktree HEAD | now | red for its declared reason, control green |
| `node examples/folder-viewer/tests/browser/auto.mjs` | worktree HEAD, release | now | 0. restart `200, 410, 200`, 4 MiB `410, 200` at WW 114.74987, changed `2 marked, 200`, 1 MiB `3 replies`, `Auto window, this image` |
| `node tools/bench/run.mjs viewer.resident_memory --no-build` | worktree HEAD, release | now | 447,971,328 bytes, bound 528,194,192, `thresholds_met` true. Opens the CT and DX workloads only |
| `memprobe.py`, probe build, 20 files of 1024 square, no window | scratch, HEAD plus a counter in `Unstored` and `contribute` | now | one round at 44 MiB: peak 2,097,152 unstored plus 4,194,304 scratch outside the cache, 2,112,108 bytes past the budget with the cache near full. Four parallel rounds, 4 series, 172 MiB, 10 repeats: peak 20,971,520 bytes outside (2 frames, 4 scratches). `inflight` 0 after each |
| `memprobe.py` at 41 MiB, HEAD binary and EVICT binary | worktree release, scratch release | now | HEAD: `used` 41,957,996 and 20 files before and after. EVICT: 20 files then 19, `used` 39,860,096 |
| `paths.py`: refused decode, unknown id, six mid-request disconnects | worktree release | now | `inflight` 0 after each |
| `midround.py`: forget a sampled id during the round, page rule replayed | worktree release and GONE2 | now | HEAD `410, 200` at 0, 5, 10, 20 and 40 ms, `inflight` 0 after each reply. GONE2 `200, skipped 1` |
| `alias.py`: parked buffers of the same length reused by three parallel rounds | worktree release | now | every reply equal to a fresh process, cached frame grey unchanged |
| Mutants MA, GLOBALGEN with a parallel noise test | scratch | now | MA red. GLOBALGEN red 3 of 3, HEAD green 3 of 3 |
| Mutants GONE2, GONE2B, NOREL, NORECYCLE, EVICT, STORE, full Rust suite | scratch | now | all six green, see S2 |
| Mutants GONE2, NOREL, EVICT, STORE, NOPOSTED, auto.mjs | scratch, release | now | GONE2, NOREL and EVICT exit 0 (EVICT 4 MiB `410, 410, 200`). STORE exit 1, 4 MiB `410` five times. NOPOSTED exit 1, 2,118 replies |
| Implementer's NOPOSTED (2,129 replies), V1 and MA red runs, second decode timing | implementer | prior receipt | progress note, informational |

## Defects

### D1, "the window's memory stays inside `OCELLI_CACHE_MIB`" is false

**Where**: `docs/lld/pixel-pipeline.md:256-257`, "The route keeps no sampled bytes past their read, so the window's memory stays inside `OCELLI_CACHE_MIB`". `examples/folder-viewer/src/series_window.rs:12-14`, the same claim. `examples/folder-viewer/src/serve.rs:1950-1951`, "so the round neither evicts nor keeps anything outside the cache budget". `series_window.rs:42`, "Its samples go back to the free list".
**What**: a round holds, outside the cache budget, the one decoded frame it is reading and the `Stored` scratch `contribute` unpacks it into (`series_window.rs:196`, 4 bytes a pixel), until that frame is read and the round ends. `claim_parked_buffer` takes the buffer off the free list, so `CacheState::charged()` drops by its size while the frame lives, and a fresh buffer was never charged. The cache can then fill to its budget beside the round. The scratch is reported nowhere. The fourth sentence is false when the budget is full: `recycle_sample_buffer` parks only when cached plus free bytes fit, and at 41 MiB `free` stayed 0, so the samples were freed.
**Why it is wrong**: microscope section 3, a false sentence in an LLD and in two doc comments. The lead's ruling on S2 rests on the same premise, "it keeps the window's memory inside `OCELLI_CACHE_MIB`, so F-216's bound needs no new term", see S1.
**Evidence**: probe build, 20 files of 1024 square, `OCELLI_CACHE_MIB=44`. Peak at one `Unstored::new`: `charged` 41,957,996, unstored 2,097,152, scratch 4,194,304, 2,112,108 bytes past the 46,137,344 byte budget. Four parallel rounds: 20,971,520 bytes outside at peak.
**Scope and regression**: prose, present on HEAD. Fix by stating what holds, for example "A round holds one decoded frame, charged as in flight, and an unpacked copy of it outside the cache budget while it reads each image, as a body preview's decode does", and "returned to the free list when the budget fits them and freed otherwise". Or move the unpack and `add_frame` into the decode job so both live inside a permit, which makes the sentence true for F-216's bound and is S1's option 1.

## Smells

### S1, F-216's bound has no term for a round's frame and scratch

**Where**: `examples/folder-viewer/src/series_window.rs:196-214` (frame and scratch held on the connection thread) and `serve.rs:1576` `decode_for_window`, whose permit ends when it returns. Bound: `.claude/plans/F-216-design.md:279`.
**What**: F-216 counts each decode's samples and `Stored` scratch inside `P = 2` permits, `P (2 S_max + 5 N_max)`. The route decodes inside a permit and then reads the frame and unpacks it on the connection thread after the permit ends, so neither is inside the P term. One round in flight per series, and the page asks for each shown series at once through `pumpMain`, so four cells of four no-window series give four rounds. The scratch half predates this range. The frame half is new with V2.
**Failure mechanism**: measured 20,971,520 bytes outside the budget with four parallel rounds on 1 Mpx frames, 2 frames and 4 scratches. On F-216's design frame, 8.7 Mpx and 17,400,516 bytes, four rounds would hold about 4 x (17.4 + 34.8) MB, 209 MB, beyond `B` and the P term. That figure is arithmetic on the design size, not a measurement. `viewer.resident_memory` opens no no-window series, so it cannot see it.
**Disposition**: the lead rules, since the ruling stated the opposite. Options: (1) unpack and pool inside the decode job, moving the pool and scratch into the job and back, so the P term covers both and D1's sentence becomes true for the bound. (2) Keep the code, add a term of one frame plus its scratch per concurrent round to F-216's bound, name it in cache.md beside the kept pool, and add the `pet_no_window` workload to `viewer.resident_memory`.

### S2, the round's new properties have no assertion

**Where**: `series_window.rs:68-77` (`Unstored::drop`), `serve.rs:1605` and `:1636` (`claim_parked_buffer`), `serve.rs:1977` and `series_window.rs:206` (the second pass `Gone`).
**What**: each property the ruling and the new doc comments claim survives every suite when removed. NOREL (no in-flight release), NORECYCLE (no recycle), EVICT (`claim_sample_buffer` in both closures, which evicts), GONE2 and GONE2B (a second pass gone read as refused or skipped) keep `cargo test -p ocelli-folder-viewer` green. GONE2, NOREL and EVICT keep auto.mjs at exit 0, EVICT only changing the 4 MiB replies to `410, 410, 200`, which `evictedProblems` accepts.
**Failure mechanism**: EVICT is the ruling's forbidden behaviour, a round that evicts. Through HTTP at 41 MiB it takes a file out of the cache, `files` 20 to 19, and nothing goes red. NOREL makes `GET /held` report in-flight bytes that no longer exist, growing by one frame per sampled image for every round, and value.rs's `inflight` test reaches only `/value`.
**Smallest regression**: an integration test that fills the budget with posted files of one series, asks `/series-window`, and requires `used`, `files` and `frames` unchanged and `inflight` 0 after, red on EVICT and NOREL (the discriminating input is above). And a `series_window` unit test whose `present` accepts an id and whose `load` returns `Fetched::Gone`, requiring `SeriesRefusal::Gone` with that id, red on GONE2B. The integration suite runs the unscheduled path, so the scheduled closure at `serve.rs:1605` needs a scheduled viewer or auto.mjs to assert the same `/held` figures.

## Nitpicks

- **N1** `series_window.rs:71`. A sample buffer that does not fit the free list is freed on the connection thread, while a stored `CachedFrame` retires its buffer to the decode worker that allocated it and cache.md keeps request bodies on their allocating thread. Unconfirmed concern for mimalloc's resident set, not measured.
- **Carried**: pass 15 N1 and the earlier library nitpicks, unchanged.

## Verified clean

- **Pass 17 D1 closed.** The "cannot evict a sampled file not yet read" sentence is gone from pixel-pipeline.md and series_window.rs, and so is the hold it described.
- **Pass 17 S1 closed.** `observeStarved` runs the 40-file series under `OCELLI_CACHE_MIB=1`, 3 replies, `Auto window, this image`. NOPOSTED, the page's `posted.add(id)` removed, goes red with 2,118 replies and both problems printed.
- **Pass 17 S2 closed as to the hold.** `present` keeps nothing, `load` looks each id up again, and nothing the round decodes is stored. STORE, the route decoding through `decode_for_value` again, is red in auto.mjs, `410` five times and the window lost, which is pass 17's V1. At 41 MiB HEAD leaves `used` and `files` unchanged. What the replacement holds outside the budget is D1 and S1.
- **N1 closed.** The header names every case, the restart on the first port, and which cases stop by PID (`alone`, the first three) and which through `stopViewer`.
- **N2 closed.** Per generation, measured above with a parallel noise test.
- **N3 closed.** `heldId` returns `""` for a failed post: `ensureFile` deletes its `posting` entry in `finally` and sets no `filePosts` entry, and a rejected pending post is caught.
- **N4 closed.** No line of pixel-pipeline.md 240 to 270 exceeds 79 columns.
- The other new prose: pixel-pipeline.md's "read where the cache holds it, or decoded ... with a buffer that never evicts, and that frame is not stored", "decoded again" and "the rounds stop and each image keeps its own window". `decode_for_window`'s doc, the `series_window` doc's two pass description, the `Fetched` variants, auto.mjs's header and `observeStarved`'s doc, the catalogue and runbook rows (`starvedProblems` exists and sets exit 1, census and runbook agree). README's series window paragraph and more.html's are unchanged and still true. cache.md's folder viewer section says nothing of the route, so nothing false.
- No `as` cast, no `unsafe` and no `#[allow]` added. No pixel arithmetic changed. The pool, `contribute` and `AutoWindowPool` are unchanged. The oracle is untouched.

## Limitations and handoff

- The probe build adds counters to `Unstored::new`, its drop and `contribute`, and nothing else. The four-round figure comes from parallel HTTP clients, not from four cells of the page.
- The panic path inside a worker job was read, not driven, and the release profile aborts.
- Not exercised: a multiframe sampled more than once under eviction, and a network failure during a re-post. The second decode's cost was not timed. The implementer's figures are informational and AC17 stays with the sprint lead on an idle host. AC16 needs the operator's study.
- No GPU, wasm or oracle job, by instruction. Scratch work lived under `scratchpad/f227-p18` and is deleted with Python `shutil` after this file is written. Every viewer this pass started was stopped.
- Next owner: the lead rules on S1, because its option decides D1's wording. Then the implementer fixes D1 and S2, and a fresh independent pass reviews the result.
