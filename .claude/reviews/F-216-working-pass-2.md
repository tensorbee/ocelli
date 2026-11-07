# F-216 review, pass 2

**Reviewed**: `work/f-216-xai` HEAD `36d79975f80d8a9dab4371294080cdf4cc9c6807`, tree `be54b2a7077f9b3c230abee23a9a6136fb24641f`, pinned at the start and the end of the pass. The story as `git diff c07ef38 36d7997` (merge base equals the `sprint/s46` tip `c07ef38`), restricted to F-216's files. The remediation as `git diff 8b00223 a7d0fb7` (`a7d0fb7` is `36d7997^1`). Untracked in the worktree and not reviewed: `.claude/reviews/F-216-working-pass-1.md`, `tools/oracle/out`. This file is the only file the pass wrote in the worktree. Mutations and instrumented runners ran in a `git archive` copy under the session scratchpad with their own target dir.
**Author / reviewer**: story by an external coding agent, merged by the sprint lead. Remediation commit `a7d0fb7` "by an external coding agent". The lead amendments in the plan are the lead's text. Reviewed by a Claude agent the lead invoked, which wrote none of the reviewed material and did not review pass 1's material as an author.
**Result**: 5 defects, 4 smells, 6 nitpicks

## Acceptance challenged

- **Pixel identity of the new owned-buffer decode (`decode_owned`, priority 1).** The reference is the pass 1 tree `8b00223`, whose id path pass 1 had already matched against `88ddaa1`. Both release binaries served 528 files: 433 synthetic edge files (Explicit VR LE, Implicit VR LE and Explicit VR BE, Bits Allocated 8, 16 and 32, Bits Stored 5 to 32, signed and unsigned, garbage bits above Bits Stored, 5 by 7 and 16 by 16, rescale 1/0 and 2.5/-1024, MONOCHROME1 and 2, and a 3-frame file) and the 95 corpus rows. Each file was posted twice and sent 6 id-form requests in both orders, so that `decode_owned` sees each request class first. 6,336 rows each. Status, window headers and the grey hash agree on every row except 290, and all 290 are `X-Ocelli-Window-Function: SIGMOID` alone on a full-range file. That is the D1 fix. The new tree labels them `SIGMOID` where the old one said `LINEAR_EXACT`. This rejects wrong byte order, a missing mask, a missing sign extension and a misaligned or truncated recycled buffer in the owned path.
- **Pass 1 D1, re-derived from PS3.3.** Stored 0, 20 and 200, slope 1 and intercept 0, full range c 100 and w 200. LINEAR (C.11.2.1.2) at 20: `((20 - 99.5) / 199 + 0.5) * 255 = 25.62814` in exact arithmetic, and 25.628143 in f32, so grey 26. LINEAR_EXACT (C.11.2.1.3.2) at 20: `((20 - 100) / 200 + 0.5) * 255` is exactly **51/2 = 25.5**, a tie. f32 gives 25.499998, and `display_index` keeps a float below the half on the lower byte, so grey 25 (S4). Stored 0 is at or below both lower edges (0.5 and 0), so grey 0. Stored 200 is above 198.5 for LINEAR and gives 255 for LINEAR_EXACT, so grey 255. SIGMOID (C.11.2.1.3.1) gives `255 / (1 + e^2) = 30.4`, `255 / (1 + e^1.6) = 42.8` and `255 / (1 + e^-2) = 224.6`. That is 30, 43 and 225, which the live server returned for both the body form and the id form.
- **Identity across requests (cached or settled output).** The scenario is a held id, then two concurrent previews of that id. The expectation is that both succeed, because the plan's 410 is for an evicted, unknown or other-process id. The observed result is D1.
- **AC15 instrument.** I injected a stage blank between draws into the page, and separately served the `88ddaa1` page through the HEAD server. Both scroll runs are now refused.
- **AC18.** I ran the committed runner six times, ran it with the restart removed, and broke the peak out by phase (D3).

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo fmt --all --check` | worktree HEAD, `CARGO_TARGET_DIR=<wt>/target` | now | exit 0 |
| `cargo clippy -p ocelli-folder-viewer -p ocelli-pixel --all-targets -- -D warnings` | same | now | exit 0 |
| `cargo test -p ocelli-folder-viewer -p ocelli-pixel` | same | now | exit 0. viewer lib 19, preview 58, lut_chain 18, all suites ok |
| `bin/ocelli.sh gate bench` | same | now | exit 0, ALL GREEN |
| `bin/ocelli.sh gate guards` | same | now | exit 0, ALL GREEN |
| `python3 scripts/guard_census.py` | same | now | exit 0, `1070 refusal(s) in 79 file(s), all claimed`, `0 watched by nothing` |
| `python3 scripts/guard_census.py --check-runbook` | same | now | exit 0 |
| `python3 scripts/prose_check.py` | same | now | exit 0, 840 files |
| `python3 scripts/source_provenance_check.py` | same | now | exit 0, 1673 files |
| `bin/ocelli.sh bench --out <scratch> viewer.open` | worktree HEAD, release built by the runner | now, load 3.97 | exit 0, 937 ms, `thresholds_met: true` |
| same, `viewer.first_image` | same | now, load 4.52 | CT 8.5 ms, DX 25.4 ms, `restart_recovered: true`, met |
| same, `viewer.scroll_rate` (5 release runs plus the 1 debug run inside the subject) | same | now, load 4.90 | 109.005 fps, 1,803 keys, `blank_samples 0`, `debug_blank_samples 0`, timer ceiling 51.39/s, met |
| same, `viewer.window_step` | same | now, load 6.34 | CT 2.5 ms, DX 19.0 ms, `preview_bodies 0`, met |
| same, `viewer.resident_memory`, three runs | same | now, loads 6.71, 6.38, 7.02 | **559,087,616**, **564,953,088**, 479,805,440 against 528,194,192. `thresholds_met` false, false, true. Heap 98.1 MB against 118,686,080, canvases `[[2900,3000]]` |
| resident subject with phase marks, committed workload | scratch copy, runner patched only to record phases | now, loads 7.12, 6.93, 3.93 | 475,447,296, **581,517,312**, **541,884,416**. Peak always in process 1, during the CT open before the restart |
| resident subject, the restart removed so one process runs the whole workload | scratch copy | now, loads 6.96, 7.22 | **658,685,952**, **667,598,848**. Phases 448 MB after opens and first images, 657 to 668 MB after scrolls |
| heap read before and after each `gc()` | scratch copy | now | 125,011,804 with no gc, 124,291,135 after one, 98,121,915 after two to six and 2 s later (S2) |
| `node race.mjs`: post a 2 MB file, two concurrent id previews, 40 trials | HEAD release, `8b00223` release in its own scratch target | now | HEAD `200: 40, 410 file-gone: 40`. A 398-byte file gives 43 and 37. `8b00223` gives `200: 80` (D1) |
| `node overcap.mjs`: 32 idle keep-alive connections, then 20 `POST /file` with a 2 MB body | HEAD and `8b00223` release | now | HEAD gives `EPIPE 4, EPIPE after 503 11, ECONNRESET after 503 2, ECONNRESET 3`. `8b00223` gives `503 Service Unavailable 20` (D2) |
| `bin/ocelli.sh bench --no-build --out <scratch> viewer.window_step` | worktree HEAD | now, load 4.85 | exit 0, `viewer.window_step measured 25.1 ms`, DX 210.8 ms, `artefact_provenance: pre-existing`. The debug binary was measured (D4) |
| `node tools/bench/run.mjs --list` and BACKLOG row status at `8b00223`, `a7d0fb7`, `36d7997` | tracked files | now | F-216 is `pending`, `pending`, `in-progress`. `--list` prints `F-216 (in-progress)` (D5) |
| `node abort.mjs`: 40 held ids, a burst of 40 previews each aborted at 30 ms, then 6 previews, 3 rounds | HEAD debug | now | every follow-up 200, round times 396, 263 and 265 ms. `/held` 34 files plus 6 frames for 40 ids, so every aborted owned buffer was restored |
| `node hdr.mjs`: post, headers, preview, headers, preview | HEAD release | now | 200, 200, **410 file-gone**, 200 `held` |
| `node page410.mjs`: real page on the 1,693-file CT, 200 ArrowDowns, and the DX folder 3 times | HEAD and `8b00223` release, Playwright from `tools/bench` | now | CT `/preview 410` 197 and 190 on HEAD against 184 on `8b00223`, confounded by eviction. DX: 1 `/file` and 1 `/preview` on both, no 410, no fault |
| implementer logs | external agent's receipts | prior | peak RSS 466,681,856 as reported, not reproduced as a pass (D3) |

Mutations, each in the scratch copy, run, then restored by copying the worktree file back:

1. D1 fix reverted (`let function = VoiFunction::LinearExact;`): `a_function_sent_alone_selects_the_full_range_window` red, body `[0, 25, 255]`.
2. S1 fix reverted (the over-cap branch reads the whole request): `a_busy_post_is_refused_before_its_body` red, `Resource temporarily unavailable`.
3. `range.end > bytes.len()` removed: `a_pixel_length_past_the_file_is_not_borrowed` red.
4. Allowlist widened back to `_ => (true, true)`: `a_deflated_syntax_is_not_walked` red. The corpus test stayed green, because every encapsulated corpus row has an undefined Pixel Data length and the walker refuses it anyway.
5. Walked range shifted by 2 bytes: four tests red, including `borrowed_bytes_match_the_parsed_pixels_for_every_accepted_corpus_file`.
6. Stage hidden in a `setTimeout(0)` after every `drawGrey` and shown on the next animation frame: the scroll subject is refused, `the stage was blank during the scroll`.
7. The `88ddaa1` page served by the HEAD server: the scroll subject is refused, `the stage was blank during the scroll`.
8. The in-page refusal `the page did not expose requestGrey and drawGrey` deleted: `tools/bench/tests/viewer_work_test.mjs` still 6 pass, 0 fail (S1).

Environment: Apple M4 Max, macOS 27.0.1, Chromium headless shell through Playwright 1.62.1 from `tools/bench`. Load average ranged from 3.9 to 11.7, with 10 users logged in and an operator process on 8742. **The timings are indicative and are not a baseline.** The RSS figures vary run to run by about 100 MB on identical input, which is itself part of D3. Every server ran with `OCELLI_PORT=0` and was stopped by its runner or by `child.kill` on its own PID. Ports 8742 and 8765 were never used, and process 98891 on 8742 was not touched. No cargo command used a target dir shared with another tree. The worktree used `<wt>/target`, the scratch copy `scratch/p2/target`, and `8b00223` `scratch/p2/oldtarget`.

## Pass 1 findings replayed

| Pass 1 | Status | Replay and adjacent case |
|--------|--------|--------------------------|
| D1 function ignored on full range | **closed** | Live server, id and body form: SIGMOID 30/43/225, LINEAR 0/26/255, none 0/25/255. Mutation 1 red. Adjacent: 290 parity rows move only in this class. The fixture's 25 is a tie artefact (S4) |
| D2 census and guards | **closed** | census, `--check-runbook` and `gate guards` exit 0. Adjacent: the in-page hooks refusal is watched only through a twin (S1) |
| D3 debug blanks | **closed for blanks** | Per-frame sampling, the 503 retry, the slot release on a closed peer and the debug run are present and observed. Mutations 6 and 7 red. Adjacent: an over-cap request with a body now loses its 503 (D2), and `--no-build` now means debug (D4) |
| D4 resident memory | **open** | The workload now runs every subject, but a restart splits it across two processes. Four of six committed runs miss the bound, and the one-process runs miss by about 130 MB (D3) |
| D5 baseline | deliberately not re-recorded | See Limitations for the tolerance the lead should use |
| D6 docs wording | **reopened by D1** | The sentence is now false for a held file under concurrency. Pass 1 named the headers-after-decode re-post, and the new sentence still does not cover it (`hdr.mjs`: 410) |
| D7 LLD and `#[must_use]` | **closed** | The plan amendment exists. Every sentence of the `pixel-pipeline.md` paragraph matches `lut.rs:629-649` and `stored_pixel.rs:222-245` (the table refuses Bits Stored above 16, each entry is `apply`, `extract` uses `stored_sample`). `#[must_use]` present |
| S1 503 before the body | **closed as stated, regressed adjacent** | Mutation 2 red. Real clients that send the body lose the reply (D2) |
| S2 heap bound | **closed for canvases, new question** | The bound uses the measured canvas, 118,686,080. Forced gc is not in the plan (S2) |
| S3 `subject_story` | **open** | Rows name F-216. The new note text names a backlog state and is false on HEAD (D5) |
| S4 walker | **closed** | Allowlist, past-end check and corpus cross-check present. Mutations 3, 4 and 5 red |
| N1 plan paragraph between rows | open | `.claude/plans/F-216-design.md:326` still splits the table |
| N2 frame cost | open | `held.rs:162` unchanged |
| N3 id aliasing | closed | `hex_id_part` and its test |
| N4 `const` or `#[must_use]` | closed | `#[must_use]` added |

## Defects

### D1, a held id answers 410 file-gone to a concurrent request

**Where**: `examples/folder-viewer/src/held.rs:318-356` (`preview_source` removes the file key to return `Owned`), `examples/folder-viewer/src/serve.rs:365-376` (the permit wait comes after that removal)
**What**: the first preview of a held id takes the posted bytes out of the cache before it waits for a permit and decodes. Until `store_frame` runs, the id names neither a file nor a frame. A second preview or `/headers` for the same id in that window gets `410 file-gone`. The page then re-posts the file under a new id, so one file is uploaded twice and its frame can be held twice. A second race on the retry shows the gone fault, because `postPreview` re-posts only once.
**Why it is wrong**: the plan's 410 is for an evicted, unknown or other-process id, and AC13 is "Each file is posted once while held". `README.md:185` and `more.html:393` now say "Each file is posted once while the process holds it". That sentence is false for a file the process holds. Pass 1 verified the 410 policy on `8b00223`, and this regression comes from the remediation.
**Evidence**: `race.mjs`, 40 trials of two concurrent id previews: HEAD gives 40 at 200 and **40 at 410**, `8b00223` gives 80 at 200. With a third concurrent `/headers`, HEAD gives 40 at 200 and 80 at 410.
**Scope and regression**: every id-form client. On the real CT page the 410 count rose from 184 to 190 and 197, but eviction confounds that comparison. The DX page did not hit it. The owned buffer also sits outside the budget while it waits for a permit (S3). Smallest fix: take the permit before `preview_source`, or leave a marker under the key so a concurrent lookup gets `Shared` and not `Gone`. Smallest regression: two threads preview one freshly posted id behind a barrier, and both must be 200.

### D2, an over-cap request with a body gets a reset in place of its 503

**Where**: `examples/folder-viewer/src/serve.rs:158-171`
**What**: the over-cap branch now reads headers only, writes the 503 and closes with the client's body unread. The unread bytes make the close a reset. Of 20 over-cap `POST /file` carrying a 2 MB body, 7 received no bytes at all (`EPIPE` 4, `ECONNRESET` 3), and 13 received the start of the 503 followed by an error. `8b00223` answered all 20 with a clean 503.
**Why it is wrong**: AC11's contract is `503 busy`, and the page's new retry (`page.html:1891`) acts only on a 503 it can read. A reset reaches `requestGrey`'s network-error branch, which ends in `UNREADABLE` or `GONE` and a fault. That is the visible failure pass 1 D3 asked to remove. `a_busy_post_is_refused_before_its_body` sends headers and never the body, so it cannot see this.
**Evidence**: `overcap.mjs` against both release binaries, shown in the evidence table.
**Scope and regression**: any client with a body, which includes the page's `/file` posts once 32 slots are taken. Id previews carry an empty body and are unaffected. Smallest fix is the one pass 1 named. Drain up to the declared length through a fixed 8 KiB buffer without keeping it, with a time limit, or shut down the write side and drain before close. Smallest regression: send the full body over the cap and require the complete 503 bytes.

### D3, peak RSS still exceeds the bound, and the runner does not measure one process

**Where**: `tools/bench/src/viewer_work.mjs:934-937` (`restartAndDraw` inside `measureResident`), `:905-910` (the sampler follows `server.pid`)
**What**: `measureResident` restarts the server midway, and the sampler switches to the new PID. The opens and first images run in process 1, and the scrolls and window steps run in process 2. So the reported peak is the larger of two halves, not "end of the four subjects above in one process". Even so, it misses: 559.1, 565.0, 581.5 and 541.9 MB against 528.2 MB, with 479.8 and 475.4 the only passes. With the restart removed, one process reaches 658.7 and 667.6 MB.
**Why it is wrong**: AC18 and the subject table at plan line 330. The implementer's 466,681,856 is one lucky draw from a distribution whose median is above the bound. Process 1 alone reaches 448 MB after the opens and first images, and 542 to 582 MB after one more CT open. So RSS rises with repeated workload. The formula assumes a high-water mark set by in-flight transients.
**Evidence**: the phase-marked runs in the evidence table.
**Scope and regression**: AC18 open on HEAD. **On `take_buffer`**: rounding to a 16 KiB multiple is sound for accounting, because frames and files are charged by capacity. It is also bounded, at under 16 KiB per body plus a recycle pool of at most 4 buffers and 64 MiB. It is not a reason that survives. 16 KiB is this host's page size (`getconf PAGESIZE` 16384), not a constant the code or the plan derives from the allocator. The pool is resident memory outside `B` that the formula does not name (S3). The measurements show the bound fails anyway. The miss goes back to the operator rather than being tuned again. Smallest regression: the runner measures one process from start to end, with any restart moved after the last sample or measured separately.

### D4, `--no-build` now measures a debug build under the release subjects

**Where**: `tools/bench/src/viewer_work.mjs:225-235` (`ensureReleaseBinary`), `:791-808` (`measureScroll`)
**What**: the remediation reused the runner's `build` option as the profile switch. `run.mjs --no-build` passes `build: false`, which before this commit meant "release, `--offline`". Now it runs `cargo build -p ocelli-folder-viewer`, which is a debug build and still a build despite the flag. It records the debug figure as `viewer.window_step` with no profile marker. `measureScroll` with `build: false` does one run and sets `fpsMet = true`, so a debug scroll is reported as meeting the 55 fps budget.
**Why it is wrong**: the plan's instrument identity is "the workspace release profile, the binary built by the runner". `--no-build` is documented as not rebuilding. A release subject that silently measures a different profile is the different-result-without-saying-so class.
**Evidence**: `bin/ocelli.sh bench --no-build viewer.window_step` reported `measured 25.1 ms`, with DX 210.8 ms against the release 2.5 and 19.0 ms, and `artefact_provenance: pre-existing`.
**Scope and regression**: any `--no-build` viewer run. `--accept` refuses `--no-build`, so the baseline cannot take one. Smallest fix: a separate `profile` argument for the debug scroll, with `build: false` keeping release and `--offline`. Add a floor test that `ensureReleaseBinary(false)` does not drop `--release`.

### D5, the five registry notes name a backlog state that is false on HEAD

**Where**: `tools/bench/subjects.json`, the `note` of each `viewer.*` row
**What**: "The backlog row is still pending, so the bench gate refuses the runner until that row is measurable." On HEAD the F-216 row is `in-progress`, `--list` prints `F-216 (in-progress)`, `gate bench` is green and every runner ran. At `a7d0fb7` the row was `pending`. If that note was true there, then `gate bench` was red on the remediation commit, and the merge of `sprint/s46` changed the state rather than the remediation.
**Why it is wrong**: microscope section 3, a false sentence in tracked prose. This is pass 1 S3's own shape, a sentence naming a backlog state that goes stale. Pass 1 asked to "state the rule without naming the backlog state".
**Evidence**: `git show <c>:docs/sprints/BACKLOG.md` at the three commits, plus `--list`.
**Scope and regression**: five notes. Smallest fix: delete the backlog sentence from each note.

## Smells

### S1, the in-page hooks refusal is covered only through a test-only twin

**Where**: `tools/bench/src/viewer_work.mjs:441-443` (the real refusal, inside `openPage`'s init script), `:43-47` (`assertPageHooks`, which no runner calls), `scripts/guards/catalogue.py` entry `bench.viewer-work`
**Failure mechanism**: the catalogue claim `the page did not expose requestGrey and drawGrey` matches both strings. The covering test drives the twin red. With the real refusal deleted, `viewer_work_test.mjs` still passes 6 of 6 (mutation 8), and the claim text still matches through the twin. The census was not run in the scratch copy because it needs git, so that half is by construction. The guard that protects the instrument is therefore watched by nothing.
**Smallest fix**: call `assertPageHooks` from a page-side check the runner reaches, or delete the twin and list the claim as browser-only with its limit.

### S2, the heap reading now forces collection, which the plan does not say

**Where**: `tools/bench/src/viewer_work.mjs:395` (`--js-flags=--expose-gc`), `:952-959`
**Failure mechanism**: before any collection the heap is 125.0 MB, after one it is 124.3 MB, and after two or more it is 98.1 MB. The bound is 118.7 MB. Collection measures retained heap honestly. The remaining 26.9 MB is about three discarded 8.7 Mpx greys, and the formula's terms (grey cache, canvases, allowance) are retained objects. So this does **not** hide a retention overrun. It does change the instrument after a miss, and the plan's wording ("measured with `performance.memory.usedJSHeapSize`") does not say "after collection". The page does briefly hold 125 MB. Without a recorded ruling, the next reader cannot tell which reading AC18 means.
**Smallest fix**: a lead amendment that names the measurement as retained heap after forced collection, with the flag in the instrument identity. Two collections suffice, so the comment and the loop count should agree.

### S3, resident memory outside both the budget and the formula

**Where**: `examples/folder-viewer/src/held.rs:199-217` (`RecyclePool`, `RECYCLE_BYTES` 64 MiB), `:318-356` (an owned buffer removed while waiting), `docs/lld/cache.md:157-167`
**Failure mechanism**: the pool keeps up to 64 MiB of empty buffers, and each waiting `Owned` request keeps a file's bytes out of the `Lru`, which then admits more posts. Neither term is in `RSS_peak <= B + 4 S_max + P(2 S_max + 5 N_max) + 32 MiB`, and `cache.md` describes neither. That is how a bound comes to hold "by luck of one run". It also leaves the next change with no stated budget to protect.
**Smallest fix**: name both in the formula and the LLD with their bounds, or remove them if D3's remedy makes them unnecessary.

### S4, the D1 fixture's LINEAR_EXACT expectation sits on an exact tie

**Where**: `examples/folder-viewer/tests/preview.rs`, `a_function_sent_alone_selects_the_full_range_window`
**Failure mechanism**: PS3.3 C.11.2.1.3.2 gives exactly 25.5 at stored 20. Grey 25 exists only because f32 makes `-80 / 200` round to `-0.4000000059604645`. A conforming reordering such as `(x - c) * 255 / w + 127.5` is exactly 25.5 in f32 and gives 26 under the project's ties-up rule. The test would then go red on correct code, and LINEAR and LINEAR_EXACT would agree, so the fixture would lose its discrimination. The comment presents the f32 figure as the PS3.3 value.
**Smallest fix**: use a stored value off the tie. At stored 50, LINEAR gives `(-49.5 / 199 + 0.5) * 255 = 63.07`, which is 63, and LINEAR_EXACT gives `(-50 / 200 + 0.5) * 255 = 63.75`, which is 64.

## Nitpicks

- **N1** (carried) `.claude/plans/F-216-design.md:326`: the lead amendment paragraph still splits the subject table.
- **N2** (carried) `held.rs:162`: a frame is charged its samples' capacity plus `size_of::<HeldFrame>()`, not its other owned buffers.
- **N3** `preview.rs` `a_deflated_syntax_is_not_walked` substitutes `1.2.840.10008.1.2.5`, which is RLE, not deflated. Deflated `.1.99` is exercised only by the corpus row when `corpus/data` exists.
- **N4** (pre-existing, both trees) an out-of-range `X-Ocelli-Frame` on a held single-frame id answers 410 `file-gone` rather than 400. That is 1,034 parity rows, identical on `8b00223`. The page does not send it.
- **N5** `page.html:1522-1554`: the thumbnail changed from smoothed `drawImage` to nearest-neighbour sampling with no plan or review mention. The letterbox is now transparent over the CSS black, which looks the same.
- **N6** `viewer_work.mjs:953`: the comment explains two collections and the loop runs four.

## Verified clean

- `decode_owned` and the recycled body buffer give byte-identical greys and identical headers to `8b00223` across 528 files by 12 requests, apart from the D1 fix class. Refusal codes are identical on all 6,336 rows.
- D1 closed on the body and id forms for SIGMOID, LINEAR and the default. The SIGMOID values were hand-checked against C.11.2.1.3.1.
- Every aborted owned buffer is restored. After an abort burst, 34 files and 6 frames for 40 ids, and follow-up previews 200 within 120 ms of the burst on debug. `loan_workspace` refuses within 200 ms when the peer is gone (unit test, and observed).
- The page's 503 loop retries only `busy`, stops on supersession or abort, drains the body, and keeps the coalesced main view's `AbortError` handling. Thumbnails, prefetch and tiles swallow a superseded retry as before, so this is not a regression.
- The scroll instrument now samples hidden and fault on every animation frame, refuses any blank or fault, runs the debug half, and rejects both an injected between-draw blank and the pre-story page.
- The walker allowlist names exactly `.2`, `.2.1` and `.2.2`. The past-end check and the corpus cross-check go red under mutation.
- `FileId::parse` refuses a sign, a leading zero and uppercase. `pixel-pipeline.md`, `benchmarks.md`, the runbook rows 537 to 540 and the plan amendments match the tree. The page heap bound of 118,686,080 is `64 MiB + 4 * 2900 * 3000 + 16 MiB`. The RSS bound of 528,194,192 is the plan's formula with `S_max` 17,400,538 and `N_max` 8,700,000.

## Limitations and handoff

- **D5 baseline, current state.** `ci/bench-baseline.json` still holds the void first run. It records open 1013, first image 16.4, scroll 51.28 under the retired instrument, window 8.5 and resident 566,099,968, each with tolerance 0.25 and three marked `thresholds_met: false`. Not counted as a defect, per the lead's instruction.
- **The tolerance the lead should use.** `record.mjs:326` carries the previous tolerance when `--tolerance` is omitted, and `--accept` does not read `thresholds_met`. A plain `bench --accept --story F-216` therefore keeps 0.25 everywhere and records whatever was measured, even above the bound. With a recorded value `V`, 0.25 admits `1.25 V` for an increase-is-worse subject. Resident admits a figure above 528,194,192 whenever `V > 422,555,353`, which every run this pass did exceeds. Recommendation:
  - Record nothing for `viewer.resident_memory` until D3 is resolved and a one-process run is at or under the bound.
  - Then accept each subject in its own call, with `--tolerance` at most `budget / V - 1`, rounded down: resident `528,194,192 / V - 1`, open `1100 / V - 1` (today 0.17 at 937 ms), and first image and window step at most 0.25. Scroll may keep 0.25, since `0.75 * 109 = 81.8` is above 55.
  - Use `--tolerance-why` to cite this derivation.
  - The CT figure is the recorded value for first image and window step, so their DX budgets (250 and 70 ms) have no tolerance protection at all.
- Not exercised: a big-endian host (the owned path refuses it by `native_order_matches`), Enhanced multi-frame functional groups beyond the corpus, live W/L and tile retries under 503, and the page under a real 503 storm on release.
- Next owner: implementation, for D1, D2, D4 and D5 and smells S1, S3 and S4. The lead, for an amendment on S2 and the D3 measurement definition (one process, restart placement). The operator, if D3's one-process miss stands. A fresh independent pass is required after remediation. It should replay `race.mjs`, `overcap.mjs`, a `--no-build` run and the one-process resident run.
