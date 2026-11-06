# F-216 review, pass 3

**Reviewed**: `work/f-216-xai` HEAD `36d79975f80d8a9dab4371294080cdf4cc9c6807` plus the uncommitted worktree, reviewed as `git diff 36d7997` (18 tracked files, `sha256` of the diff `890976bbfd01bc1c91a3b6303195854ff0da2ba3b7864dedaaba462cc033fe64`, the same at the start and at the end of the pass). Untracked and not reviewed: `.claude/reviews/F-216-working-pass-1.md`, `.claude/reviews/F-216-working-pass-2.md`, `tools/oracle/out`. The only file this pass leaves in the worktree is this report. One probe file was created under `tools/bench` by mistake and deleted in the same command, before anything ran. Mutations, variants and instrumented runners ran in an `rsync` copy under the session scratchpad, `scratchpad/p3r/tree`, with its own target dir `scratchpad/p3r/target`. Each mutated file was restored from the worktree, and `diff -r` was confirmed clean.
**Author / reviewer**: two remediation rounds by an external coding agent (the implementer's log in the session scratchpad, the implementer's log in the session scratchpad), on top of the lead's plan amendments and the operator's answer. Reviewed by a Claude agent the lead invoked. This agent wrote none of the reviewed material and did not write passes 1 or 2.
**Result**: 3 defects, 5 smells, 8 nitpicks

## Acceptance challenged

- **Pixel identity after the owned path was removed and the sample buffer started coming from the pool.** HEAD release against a fresh `36d7997` release build (`scratchpad/p3r/viewer-36d7997`). Both ran pass 2's `parity.mjs` over the same 528 files (433 synthetic edge files and 95 corpus rows), each file posted twice and sent 6 id-form requests in both orders. The result was 6,336 rows each and **zero differing rows** in status, window headers or grey hash. This rules out stale pooled bytes in a reused sample buffer, a short `resize`, and a changed refusal order.
- **S4 re-derived from PS3.3, independently, in `f32`.** I computed it with `Math.fround` at every operation, in `apply_window`'s operation order (`lut.rs:775-797`), then applied `display_index`'s floor of `y + 0.5` (`colour.rs:482-507`).
  - LINEAR (C.11.2.1.2) at center 100 and width 200 uses `c - 0.5 = 99.5` and `w - 1 = 199`. Stored 27 gives `((27 - 99.5) / 199 + 0.5) * 255 = 34.597988`, grey 35. Stored 34 gives 43.567841, grey 44.
  - LINEAR_EXACT (C.11.2.1.3.2) at stored 27 gives `((27 - 100) / 200 + 0.5) * 255 = 34.424999`, grey 34. Stored 34 gives 43.349998, grey 43.
  - The fractional parts of `y + 0.5` are .098, .925, .068 and .850. Each is at least 0.068 from a tie, which is far beyond `f32` error, so the fixture no longer depends on rounding.
  - Stored 0 sits on both lower edges (0 and 0) and gives grey 0. Stored 200 is above LINEAR's upper edge `99.5 + 99.5 = 199`, and LINEAR_EXACT gives exactly 255 there.
  - The fixture's `[0, 35, 44, 255]` and `[0, 34, 43, 255]` match. Mutating the full-range default to `VoiFunction::Linear` turned the test red with body `[0, 35, 44, 255]`.
  - **Two pass 2 errors are corrected here.** Pass 2's suggested stored-50 values were wrong: LINEAR is 64.070, grey 64, not 63. Pass 2's LINEAR upper edge of 198.5 was also wrong, it is 199. The implementer's comment is right on both.
- **D1 through the real caller, plus an adjacent eviction race.** Two concurrent id previews: 80 of 80 returned 200 on a 2 MB file and 80 of 80 on a 398-byte file. Then I raced 2 previews and 1 `/headers` of a fresh id against 3 posts that evict it, 40 trials each at `OCELLI_CACHE_MIB` 3, 5 and 9. Every preview and every `/headers` returned 200, every 200 body was byte-identical, `used <= budget` held, and `inflight` was 0 after every trial.
- **D2 through Chromium, which is the page's real client.** Over the cap, `fetch("/file")` with bodies of 32,768, 61,440, 524,822 and 17,400,538 bytes read a complete `503 busy` 10 of 10 times at every size, and a later fetch was served. Slow bodies were checked with a raw socket: headers then a 500 ms pause, a 16 KiB body with a 500 ms pause in the middle, and headers then a 150 ms pause. All ended with a complete 503 and a clean close, and the process kept serving.
- **D3, one process.** `measureResident` calls `startViewer` once, and nothing reassigns `server.pid`. `restartAndDraw` is gone from it, so the sampler follows a single PID. Three committed runs and two scratch runs were all under 528,194,192. I also ran a variant with the pool and the sample-class reservation removed, to decide whether those tweaks still pay for themselves (S5).

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo fmt --all --check` | worktree, `CARGO_TARGET_DIR=<wt>/target` | now | exit 0 |
| `cargo clippy -p ocelli-folder-viewer -p ocelli-pixel --all-targets -- -D warnings` | same | now | exit 0 |
| `cargo test -p ocelli-folder-viewer -p ocelli-pixel` | same | now | exit 0. viewer lib 23, preview 59, pixel lib 14, color 15, image_plane 4, lut_chain 18, modality 5, palette 7, stored_pixel 2, voi 6 |
| `cargo deny --locked check licenses` | same | now | exit 0, `licenses ok` |
| `bin/ocelli.sh gate licences` | same | now | exit 0, `OK: licence check (rust, npm, python, notices, holder)`, 31 tests |
| `bin/ocelli.sh gate bench` | same | now | exit 0, 139 tests, 137 pass, 0 fail, 2 skipped |
| `bin/ocelli.sh gate guards` | same | now | exit 0, ALL GREEN |
| `python3 scripts/guard_census.py`, then `--check-runbook` | same | now | exit 0 and exit 0, 0 watched by nothing |
| `python3 scripts/prose_check.py` | same | now | exit 0, 840 files |
| `python3 scripts/source_provenance_check.py` | same | now | exit 0, 1673 files |
| `viewer.resident_memory` x3, release, built by the runner | worktree | now, loads 8.17, 9.81, 11.62 | **492,634,112**, **466,042,880**, **492,273,664** against 528,194,192, all `thresholds_met: true`. Heap 98,123,959 to 98,124,127 retained (before gc 125.02 to 125.04 MB) against 118,686,080. `held_peaks.pool` 52,726,784 (3 DX + 1 CT spares) |
| `viewer.open` | worktree | now, load 8.79 | 950 ms, met |
| `viewer.first_image` | worktree | now, load 9.03 | CT 8.1 ms, DX median 29.6 ms, met |
| `viewer.scroll_rate` (5 release runs and the debug half inside it) | worktree | now, load 8.62 | 109.03 fps, 1,802 keys, `blank_samples 0`, `debug_blank_samples 0`, met |
| `viewer.window_step` | worktree | now, load 9.61 | CT 2.3 ms, DX 18.8 ms, `preview_bodies 0`, met |
| `bin/ocelli.sh bench --no-build viewer.window_step` | worktree | now, load 9.61 | `profile: release`, CT 2.2 ms, DX 17.8 ms, `pre-existing`. D4 closed in behaviour |
| `measureScroll({ profile: "debug" })` via `node -e` | worktree | now, load 10.36 | 0.908 fps, 360 keys, `blank_samples 0`, `thresholds_met: false` (under 55, as designed) |
| resident, scratch copy, heap readings logged at each gc | scratch, HEAD source | now, loads 8.12, 7.45 | 453,476,352 and 494,075,904. Readings `[124293603, 98124338, 98125478, 98125478, ...]` (D1) |
| resident, scratch, pool off and no sample-class reservation | scratch variant | now, loads 10.08, 11.27 | 492,240,896 and 500,236,288 (S5) |
| `parity.mjs` 528 files x 12 requests | HEAD release against `36d7997` release | now | 6,336 rows each, 0 differences |
| `race.mjs`, `hdr.mjs` (pass 2) | HEAD release | now, load 4.35 | 200:80 (2 MB), 200:80 (398 B). `hdr.mjs`: headers after a decoded single-frame preview gives 410, which the plan sanctions at line 237 (N8) |
| `evict.mjs` (new) | HEAD release | now | all 200 at 3, 5 and 9 MiB, 0 mismatched bodies, 0 bad census rows |
| `overcap.mjs` (pass 2), raw socket, 20 per size | HEAD release | now | 32 KiB and 60 KiB give a clean 503 20 of 20. 200 KB, 524,822 B and 2 MB bodies give the 503 status line and then ECONNRESET or EPIPE |
| same | scratch build with the drain removed | now | 32 KiB and 60 KiB give `ECONNRESET after HTTP/1.1 503` 20 of 20 (S1) |
| `obrowser.mjs` (new), Chromium `fetch` over the cap, 10 per size | HEAD release, and the drain-removed build | now | complete `503 busy` 10 of 10 at every size on **both** binaries |
| `slow.mjs` (new) | HEAD release | now | three slow-body shapes, a complete 503 and a clean close 10 of 10 each. `/held` then 200 |
| implementer logs | external agent's receipts | prior | resident 493.6, 490.4 and 460.8 MB. Not relied on. The runs above reproduce the range |

Mutations, each in the scratch copy, run and then restored:

1. Full-range default changed to `VoiFunction::Linear`: `a_function_sent_alone_selects_the_full_range_window` red, body `[0, 35, 44, 255]`.
2. `discard_busy_body` call removed: `a_busy_post_with_a_body_is_still_readable` **green** (S1).
3. Drain and `set_nodelay` and `flush` all removed (the pass 2 shape): the same test red 8 of 8, `Connection reset by peer`. Removing only `set_nodelay`, or only drain and flush, leaves it green 8 of 8.
4. `ensureViewerBinary` changed to pick the debug profile when `build === false` (the pass 2 D4 shape, in the real path): `viewer_work_test.mjs` and `record_test.mjs` **green** (S2).
5. `await page.evaluate(assertPageHooks)` removed from `openPage`: `openPage evaluates the real in-page hooks check` red. S1 from pass 2 is closed at the call site.
6. `assertPageHooks`'s no-argument lookup of `globalThis.requestGrey` replaced by a function: all tests **green** (S3).
7. The debug-binary refusal in `acceptRecord` removed: `a viewer subject measured on a debug binary is refused` red.

Environment: Apple M4 Max, macOS 27.0.1, Node 24.13.0, Playwright 1.62.1 from `tools/bench`. Load average ran from 3.2 to 11.6, with 10 users logged in and the operator's debug viewer, PID 98891, on 8742. **The timings are indicative and are not a baseline.** Every viewer ran with `OCELLI_PORT=0` and was stopped by its runner or by `child.kill` on its own PID. Ports 8742 and 8765 were never used, and PID 98891 was not touched. No viewer from this pass is still running.

## Pass 2 findings replayed

| Pass 2 | Status | Replay and adjacent case |
|--------|--------|--------------------------|
| D1 held id 410 under concurrency | **closed** | 80 of 80 at 200 on both file sizes. The serve test takes both permits, so it would be red on the pass 2 code. Adjacent eviction race: all 200, bodies identical |
| D2 over-cap body reset | **closed for the page's client** | Chromium reads the 503 at every size up to 17.4 MB, and slow bodies close cleanly. Adjacent: a raw socket gets a reset after the 503 above 64 KiB, which is the stated cap. The drain is not what the regression test detects (S1) |
| D3 resident memory, one process | **closed** | One PID. 466.0 to 494.1 MB over five runs against 528.2 MB. mimalloc is the global allocator in `examples/folder-viewer/src/main.rs` only, behind `cfg(not(target_arch = "wasm32"))`. It is pinned `=0.1.52` in `Cargo.toml:146`, default features, with no `unsafe` of ours, and the licence gates are green. Lock adds `mimalloc 0.1.52` and `libmimalloc-sys 0.1.49`. The leftover pool tweaks are S5 |
| D4 `--no-build` measured debug | **closed in behaviour** | `--no-build` ran release, `pre-existing`. The regression test does not cover the real path (S2) |
| D5 status sentences in notes | **closed** | All five notes now state only the direction, plus the resident formula sentence |
| S1 hooks refusal through a twin | **closed at the call site, narrowed** | Mutation 5 red. The no-argument branch that runs in the page is untested (S3) |
| S2 forced collection undocumented | **closed by amendment, implementation short** | The amendment says "until two readings agree". The code stops after two collections whether or not they agree (D1) |
| S3 pool and in-flight in the LLD | **closed** | `cache.md:169-186` names both. Two doc comments in `held.rs` contradict it (D2) |
| S4 tie fixture | **closed** | Re-derived above. Mutation 1 red |
| N1 plan paragraph inside the table | open | Still at the `viewer.scroll_rate` row |
| N2 frame cost | open | `held.rs:160` unchanged |
| N3 RLE stand-in for deflated | open | `preview.rs:2654` |
| N4 out-of-range frame answers 410 | open, pre-existing | not re-run |
| N5 thumbnail sampling | not re-checked | `page.html` is not in this diff |
| N6 gc comment and loop count | superseded by D1 | |

## Defects

### D1, the retained heap reading does not wait for two readings to agree

**Where**: `tools/bench/src/viewer_work.mjs:1010-1025`
**What**: the comment and the lead amendment both say the heap is read "after forced collection until two readings agree". The loop runs at most two collections and returns the second reading, whether or not it equals the first. In every instrumented run the first two readings differ: `124,293,603` after one gc and `98,124,338` after two. The third reading is `98,125,478`, which differs again by 1,140 bytes, and the readings stay equal after that. So the reported figure has never been read under the agreement rule that defines it.
**Why it is wrong**: the plan's "Lead amendments after review pass 2", first bullet, is the instrument definition for AC18's page half. The comment "Two collections are enough for this cache" is contradicted by these readings.
**Evidence**: `HEAPREADINGS` from the scratch runner, two runs, shown in the evidence table.
**Scope and regression**: on this workload the error is 1,140 bytes against a 20.6 MB margin, so no verdict flips. The instrument is still not the one the amendment defines, and on a page that settles more slowly it would report a reading still in motion. Smallest fix: loop until two consecutive readings are equal, with a stated cap. Refuse when they never agree, and record the number of collections in `detail`.

### D2, two `MemoryCensus` doc comments are false

**Where**: `examples/folder-viewer/src/held.rs:271` and `:273`
**What**: `pool` says "Spare buffers. Counted inside `pressure.budget`." That is false. `held.rs:200`, `memory_census`'s own comment at `:354` and `cache.md:170` all say the pool sits outside the budget, and `/held` at `OCELLI_CACHE_MIB=3` reported `pool 6,292,992` against `budget 3,145,728`. `inflight` says "Body reads and decode copies that are not in the LRU or the pool yet." But `serve.rs:228-230` counts a body only from the end of its read to the end of `route`. A body still being read is therefore not counted. A `/file` body is counted twice once `insert_file` has moved it into the LRU, until `route` returns. The `held_peaks.inflight` figure the runner now records (17,400,576) is read off this counter.
**Why it is wrong**: microscope section 3. A doc comment is a claim about the code.
**Evidence**: the `evict.mjs` census rows, and the code path above.
**Scope and regression**: the census is informational and has no threshold. The implementer's report used these fields as live peaks behind the formula terms. Smallest fix: correct the `pool` sentence, and either count the body from the start of its read to its insert or describe what the counter actually measures.

### D3, the plan's `/held` row no longer describes the reply

**Where**: `.claude/plans/F-216-design.md:110`
**What**: the API table gives the reply as `{"build":"release","budget":N,"used":N,"files":N,"frames":N}`. The tree now also returns `pool`, `inflight` and `workspace` (`serve.rs:289-305`), and the resident runner consumes them as `held_peaks`. No amendment records the change.
**Why it is wrong**: the plan is the design contract for this route, and its row is now false of the tree.
**Evidence**: `/held` replies shown in the evidence table.
**Scope and regression**: the runner and any later reader of the contract. Smallest fix: a lead amendment naming the three fields and what each measures, after D2 is settled.

## Smells

### S1, the over-cap regression test does not detect the drain it was written for

**Where**: `examples/folder-viewer/tests/preview.rs:2506` (`a_busy_post_with_a_body_is_still_readable`), `examples/folder-viewer/src/serve.rs:33-38`
**Failure mechanism**: with only the drain removed, the test stays green (mutation 2). With the drain kept and only `set_nodelay` removed, it also stays green. The test goes red only when all three changes are removed (mutation 3). So `discard_busy_body` can be deleted with the suite green. A raw client then gets `ECONNRESET after HTTP/1.1 503` 20 of 20 at 32 KiB, where HEAD closes cleanly. The constant's comment ends "the test body fits inside it", which suggests the test exercises the drain. It does not. Chromium reads the 503 with or without the drain at every size up to 17.4 MB, so the page does not depend on it today.
**Smallest fix**: after reading the 503, read on to the end of the stream and require `Ok(0)`, not a reset. That goes red under mutation 2. Or, if the drain is judged unnecessary for the page, delete it and its constant.

### S2, the D4 regression covers the argument builder, not the path that regressed

**Where**: `tools/bench/tests/viewer_work_test.mjs:78`, `tools/bench/src/viewer_work.mjs:240-266`, `tools/bench/src/record.mjs:322-324`
**Failure mechanism**: the test asserts `ensureReleaseBinaryArgs(false)`. Pass 2's D4 was in the composition, the code that picks a profile from `build`. Reintroducing that defect in `ensureViewerBinary` leaves every test green (mutation 4). Under that mutation the measured binary is debug, but `detail.profile` still says `release`, because every `measure*` sets it from a `profile` argument that `run.mjs:202-205` never passes. So the new `acceptRecord` refusal cannot fire through the real caller. It would record a debug figure as a release subject, which is the exact shape it exists to refuse. The process already reports its own build: `/held` returns `"build"` from `cfg!(debug_assertions)`.
**Smallest fix**: set or check `detail.profile` from `/held`'s `build`, which the runner already fetches, and test `ensureViewerBinary`'s chosen args for `build: false`. Either delete the unused `profile` parameter on the five runners, or have one caller pass it.

### S3, the branch of `assertPageHooks` that runs in the page is untested

**Where**: `tools/bench/src/viewer_work.mjs:43-49`
**Failure mechanism**: in the page, `page.evaluate(assertPageHooks)` calls the function with no arguments, so it reads `globalThis.requestGrey` and `globalThis.drawGrey`. The refusal test passes explicit arguments. Replacing the `globalThis` lookup with a function that always exists leaves every test green (mutation 6), and the instrument would then accept a page with no hooks. This is pass 2's S1, now narrowed to one branch.
**Smallest fix**: a unit case that calls `assertPageHooks()` with `globalThis.requestGrey` unset and expects the refusal.

### S4, three prose copies of the mimalloc version, and "the release binary"

**Where**: `README.md:185`, `examples/folder-viewer/src/use.html:270`, `docs/lld/cache.md:182`
**Failure mechanism**: the version is owned by `Cargo.toml:146`, an exact pin, and nothing ties these three sentences to it. The next bump leaves all three false with every gate green. That is the failure class `CLAUDE.md` records for numbers written in prose next to the file that holds them. README and `use.html` also say "The release binary uses mimalloc", but `main.rs:3-5` sets the allocator for every profile. A reader comparing the "ten times slower" debug build could reasonably infer that the debug build keeps the system allocator.
**Smallest fix**: drop the version from all three, say "pinned exactly in the workspace manifest", and say "the folder viewer binary". The plan's operator answer already does both.

### S5, the pool tweaks lost their measured justification with mimalloc and still carry libmalloc claims

**Where**: `examples/folder-viewer/src/held.rs:199-217` (pool, 64 MiB), `:510-540` (`take_buffer`, the twice-class match), `examples/folder-viewer/src/serve.rs:449-464` (sample buffer taken at the file's class), `preview.rs:567-571` and `:584-590`, `held.rs:803-807` (test comment), `docs/lld/cache.md:185-186`
**Failure mechanism**: the tweaks were introduced to fight libmalloc's resident empty magazines (implementer fix2 log). Under mimalloc, a scratch build with the pool switched off and no sample-class reservation measured 492.2 and 500.2 MB. HEAD measured 453.5 to 494.1 MB over five runs. That is the same spread, and both are under the bound. What remains is cost and claims:
- Up to 64 MiB of idle spares sit outside `B`. That uses most of the `4 * S_max` upload term, 69.6 MB, which the plan wrote for uploads in flight.
- A multi-frame preview reserves a file-sized buffer per frame and then shrinks it.
- The comments still state allocator behaviour as fact: "shrinking it would allocate a second class the process would keep", "Growing would keep both", "dropping it on this thread does not make the next thread's allocation reuse the pages". No measurement on the allocator now in use supports them.
- `cache.md`'s stated reason for keeping them ("They are the upload term and the shared class for files in one series") is accounting, not a measured need.

The next change will treat these as constraints.
**Smallest fix**: the lead decides. Either remove the pool and the sample-class reservation, which measured equal and is simpler. Or keep them with a mimalloc measurement that shows a benefit, and delete the allocator claims either way.

## Nitpicks

- **N1** (carried) the lead amendment paragraph still splits the subject table in the plan.
- **N2** (carried) `held.rs:160`: a frame is charged its sample capacity plus `size_of::<HeldFrame>()` only.
- **N3** (carried) `preview.rs:2654`: `a_deflated_syntax_is_not_walked` substitutes RLE `.1.2.5`, not deflated `.1.99`.
- **N4** (carried, pre-existing) an out-of-range frame on a held single-frame id answers 410, not 400.
- **N5** `held.rs` `leave_inflight` saturates instead of subtracting, which would hide an unpaired call. `serve.rs:456` counts `source_class`, not the capacity `take_buffer` actually returned, which can be up to twice that.
- **N6** `viewer_work.mjs:956`: `held_peaks` is read on every tenth RSS tick, at least 200 ms apart. It is a sampled maximum, not a peak, and its name says peak.
- **N7** `cache.md:169`: "spare posted buffers". The pool also takes evicted frame sample buffers (`held.rs` `unique_bytes`).
- **N8** (carried from pass 2's D6 note) `/headers` after a decoded single-frame preview answers 410, so the headers sheet re-posts the file. `.claude/plans/F-216-design.md:237` sanctions this. README's "posted again after eviction or a restart" does not mention it.

## Verified clean

- Pixel identity HEAD against `36d7997` on 6,336 rows, 0 differences. The fresh `resize` zero-fills a pooled buffer before decode, so no stale bytes can reach a frame. `finish_decode`'s new `output_len == expected` check is equivalent to the removed length check.
- The S4 fixture values were re-derived from C.11.2.1.2, C.11.2.1.3.2 and `display_index` in `f32`, they discriminate, and mutation 1 is red.
- D1: the posted file stays in the LRU through the permit wait, and the permit is taken before `take_buffer`. That satisfies the amendment "bytes waiting for a decode permit are inside B". No lock is held across another: `memory_census`, `store_frame`, `keep_buffers` and `take_buffer` take the pool, cache and permit locks one at a time.
- D2: complete 503 for the page's client at every size, a clean close on slow bodies, and recovery afterwards.
- D3: one PID per resident run and five runs under the bound. mimalloc is scoped to the binary, pinned exactly with default features, and adds no `unsafe` of ours. `deny` and the licence gate are green. `THIRD_PARTY_NOTICES` covers vendored code only, so it has no row to add.
- D4: `--no-build` measures release. The debug-binary refusal is red under its mutation.
- D5: the five notes are clean.
- `subjects.json`, `record.mjs` and the `bench.record` probe budget (6 to 7) agree. The census reports 0 watched by nothing.
- The cap arithmetic in `cache.md`: 64 MiB is 67,108,864, under `4 * 17,400,538 = 69,602,152`. The pool's observed peak of 52,726,784 is three DX buffers and one CT buffer.

## Limitations and handoff

- Not exercised: a big-endian host, Enhanced multi-frame files beyond the corpus and the 3-frame edge file, a 503 storm on the real page, and the drain's effect on non-browser HTTP clients other than a raw socket.
- The no-pool variant was measured twice, with an instrumented runner in the scratch copy, so S5 rests on two runs plus five HEAD runs at a run-to-run spread of about 40 MB.
- The baseline was not re-recorded. Pass 2's tolerance guidance still applies. The resident figures now support `--tolerance` at most `528,194,192 / V - 1`, which is about 0.07 at the 494 MB high reading.
- Next owner: implementation, for D1, D2, S1, S2, S3 and S4. The lead, for D3's amendment and the S5 decision. A fresh independent pass is required after remediation. It should replay mutations 2, 4 and 6 and the heap-reading log.
