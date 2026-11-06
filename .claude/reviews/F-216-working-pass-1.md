# F-216 review, pass 1

**Reviewed**: `git diff 88ddaa1..8b00223` on `work/f-216-xai` (HEAD `8b00223d5c5bcd76d77c54bfb93d738b1374822d`, tree `b7ba8866950a1a17ff405c8ef3fcd1f6cc9d6660`), worktree clean apart from the untracked `tools/oracle/out`. The same HEAD and tree were pinned at the start and the end of the pass. This file is the only file the pass wrote in the worktree.
**Author / reviewer**: implementation by an external coding agent (commit `ab5e576`), merge `8b00223` by the sprint lead. Reviewed by a Claude agent the lead invoked, which wrote none of the reviewed material. The lead amendment in the plan is the lead's text and is reviewed here as part of the diff.
**Result**: 7 defects, 4 smells, 4 nitpicks

## Acceptance challenged

- **Pixel identity of the new borrowed Pixel Data path and the stored-sample table (AC2, AC3, priority 1).** I derived the expectation independently: the pre-story `preview_frame` at `88ddaa1` is the reference. I built both trees in isolated scratch targets and decoded 179 files (95 corpus rows plus 84 synthetic edge files) under 168 requests each: frames 0 to 2, invert on and off, the file window, the file window with each of the three functions, seven numeric windows under each function, and the three presets. The edge files cover Explicit VR LE, Implicit VR LE and Explicit VR BE. They also cover Bits Allocated 8, 16 and 32, Bits Stored from 5 to 32, signed and unsigned, garbage bits above Bits Stored, multi-frame, odd pixel counts, rescale 2.5 and -1024, MONOCHROME1, undefined-length sequences with nested icon Pixel Data, a UN element of undefined length, trailing padding and an over-long pixel value. 156 of the 179 files take the borrowed path. Status, headers and the grey hash agree on 30,072 of 30,072 rows, apart from one request class (D1). The wrong behaviours this rejects are wrong byte order, wrong frame offset, a missing mask, a missing sign extension and a misaligned value.
- **`LutChain::stored_sample_table` (priority 1b).** A scratch test written from PS3.3 C.7.6.3 alone compares the table with `LutChain::apply` for every pattern. It covers Bits Allocated 8, 16 and 32, every Bits Stored from 1 to 16, both signs, three rescales, five windows, three functions and both MONOCHROME interpretations, 44,125,200 entries in all, and every entry is bit-identical. `map_containers` against an independent unpack, with random containers carrying garbage high bits, agrees in both byte orders. I hand-checked two entries. Stored -3 as a 12-bit signed pattern `0xFFD`, slope 2.5, intercept -1024 gives -1031.5 (C.11.1). Under LINEAR c -1000 w 101 that maps to `((-1031.5 + 1000.5) / 100 + 0.5) * 255 = 48.45` (C.11.2.1.2), and under LINEAR_EXACT to `(-31.5 / 101 + 0.5) * 255 = 47.970` (C.11.2.1.3.2). Both match the table. Container `0xFFFD` masks to the same entry.
- **display_index (priority 1c).** The end-to-end comparison above checks the baked table against the old per-pixel `display_index` loop, because the old loop is the reference. 30,072 rows agree.
- **AC15 and AC17 scroll (priority 3).** I challenged the instrument with the pre-story page as the wrong implementation AC15 names, and then on the debug build, which the approved threshold names (D3).
- **AC18 resident memory.** I challenged it with the plan's own subject definition rather than the runner's single pass (D4).

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo fmt --all --check` | worktree HEAD | now | exit 0 |
| `cargo clippy -p ocelli-folder-viewer -p ocelli-pixel -p ocelli-cache --all-targets -- -D warnings` | worktree HEAD | now | exit 0 |
| `cargo test -p ocelli-folder-viewer -p ocelli-pixel -p ocelli-cache` | worktree HEAD | now | exit 0. preview 56, lut_chain 18, all suites ok |
| `bin/ocelli.sh gate bench`, `prose`, `provenance`, `unsafe`, `ci` | worktree HEAD | now | each exit 0, ALL GREEN |
| `bin/ocelli.sh gate guards` | worktree HEAD | now | **exit 1**, `FAIL: the guard census` (D2) |
| `python3 scripts/guard_census.py` | worktree HEAD | now | **exit 1**, 13 unclaimed refusals in `tools/bench/src/viewer_work.mjs` (D2) |
| scratch `dump` example, old against new, 179 files by 168 requests | `git archive 88ddaa1` and `git archive HEAD`, scratch targets | now | 30,072 rows, 284 differ, all of them file-window-plus-function on a full-range file (D1) |
| scratch `which` example | HEAD plus a scratch-only probe of the borrowed branch | now | 156 borrowed, 23 fallback (encapsulated, deflated, YBR 4:2:2, odd 8-bit, over-long value, UN undefined length) |
| scratch `tests/scratch_table.rs` | HEAD `ocelli-pixel`, scratch copy | now | 2 passed, 44,125,200 entries checked |
| Mutations in the scratch copy, story tests only (`--lib --test lut_chain --test preview`) | HEAD scratch copy | now | all 7 red, see below |
| `bin/ocelli.sh bench --out <scratch> viewer.open viewer.first_image viewer.scroll_rate viewer.window_step viewer.resident_memory` | worktree HEAD, release built by the runner into the worktree target | now | exit 0. open 960 ms, first image CT 8.0 and DX 40.7 ms, scroll 109.05 fps with 0 blanks and timer ceiling 53.48/s, window CT 2.2 and DX 18.6 ms, peak RSS 494,960,640 against 528,194,192, heap 159,669,335 |
| same scroll subject, page from `88ddaa1` served by the HEAD server | scratch tree | now | 109.03 fps, 0 blank samples (D3) |
| scroll subject with the stage sampled in every animation callback, release | scratch trees, old and new page | now | old 0 of about 1,810, new 0 of about 1,810 |
| scroll and window subjects on the **debug** build, new page | scratch tree, runner patched to drop `--release` | now | scroll 0.91 fps. Official `blank_samples` 0, per-callback hidden 209/361 and 74/362, then 176/361 on a rerun, with the fault text `this process is already serving its connection limit` (D3). Window CT 26.2 ms, DX 217.6 ms |
| resident subject with the plan's full four-subject workload in one process, release | scratch tree, runner extended | now | **799,309,824** and **772,866,048** bytes against 528,194,192 (D4) |
| implementer logs the implementer's log in the session scratchpad | external agent's receipts | prior | figures as reported. The first log's numbers are void by the lead's note |

Environment: Apple M4 Max, macOS 27.0.1, Chromium headless shell 151.0.7922.34, Playwright 1.62.1. Load average was 11 to 17 throughout, because Spotlight indexed the 850 MiB workload and the antivirus scanned it. The machine was not idle, and the timing figures carry that caveat. `OCELLI_PORT=0` was used throughout. Ports 8742 and 8765 were never used. Every server was stopped by its runner. The process on 8742 belongs to the operator (started 03:53) and was not touched.

A process note for the lead. Building a `git archive` copy of the tree in the worktree's own `CARGO_TARGET_DIR` reused the worktree's release fingerprints. Cargo hashes workspace members by path relative to the workspace root, and the archive's file times are older than the build. So the first attempt linked the old tree's `ocelli-pixel` into the new tree's build. I deleted `target/release` in the worktree, which is ignored build output, and gave every scratch tree its own target. The worktree's release binary used for the bench above was rebuilt by the runner from HEAD, which I confirmed by string search of the binary. This is the same hazard that voided the implementer's first run.

Mutations, each applied to the scratch copy, run, then restored:

1. `stored_sample` sign bit from `bits_stored - 1` to `bits_stored`: `stored_sample_table_matches_apply_for_every_stored_integer` red.
2. `meaningful_bits` mask to `u32::MAX`: `stored_sample_table_maps_each_container_through_its_pattern` red.
3. `map_grey16_le` read big endian: `a_native_frame_is_read_from_the_file_slice` red.
4. 16-bit fast path widened to `>= 12` stored bits: the container-map test red.
5. `pixel_value_span` explicit header length 12 to 8, a 4-byte misalignment of the borrowed slice: `inspect_drops_the_pixel_payload` and `a_native_frame_is_read_from_the_file_slice` red.
6. Table invert `255 - byte` to `254 - byte`: three integration tests red, including `held_frames_keep_the_hand_computed_grey`.
7. `borrowed_pixels_fit` always false: `a_native_frame_is_read_from_the_file_slice` red.

## Defects

### D1, a requested VOI function is ignored on a full-range file

**Where**: `examples/folder-viewer/src/preview.rs:848`
**What**: `select_window` returns LINEAR_EXACT for `FileWindow::Full` and drops `request.window_function`. At `88ddaa1` the requested function replaced the file function in both branches. `POST /preview` with only `X-Ocelli-Window-Function: SIGMOID` or `LINEAR` on a file with no Window Center now returns different grey bytes and the label `LINEAR_EXACT`.
**Why it is wrong**: the plan states `preview_frame` is unchanged and the body form of `/preview` is "Unchanged, pinned". `validate_frame_request` still accepts a function with no window, so the request is valid and is silently answered with a different transform.
**Evidence**: the scratch dump differs on exactly 284 rows, all `filefn` requests on full-range files, for example `edge/ebe_4x4x2_ba32_bs16_s_g_MONOCHROME2.dcm f0 filefn SIGMOID`: old `SIGMOID h b1bd4c1fb17fca87`, new `LINEAR_EXACT h 1d43b07f449dfd8e`. AC2's test compares the id form with the body form of the same new code, so it cannot see this.
**Scope and regression**: the public `preview_frame`, `/preview` body form and id form. The page always sends a function together with a centre and width, so the page itself does not reach it. Smallest fix is `let function = request.window_function.unwrap_or(VoiFunction::LinearExact);` in that arm, plus a fixture that posts a function alone on `mono_frame(false)` and asserts the old label and grey.

### D2, the guards gate and the guard census fail on the story tree

**Where**: `tools/bench/src/viewer_work.mjs:27, 84, 96, 384, 498, 509, 556, 598, 599, 602, 628, 633, 781`
**What**: 13 new refusals have no entry in `scripts/guards/catalogue.py`. `bin/ocelli.sh gate guards` exits 1 and `python3 scripts/guard_census.py` exits 1.
**Why it is wrong**: CLAUDE.md says the unwatched bucket is a ratchet that may only decrease. The implementer's log reports `OK: 939 refusal(s) ... 0 watched by nothing`, which does not match the committed tree. The likely cause is a census run over an unstaged file, which is the hazard AGENTS.md "Stage before you gate" describes.
**Evidence**: `bin/ocelli.sh gate guards` printed `FAIL: the guard census` with the 13 lines and `0 passed, 1 failed`.
**Scope and regression**: blocks `gate --sprint`. Smallest fix is a catalogue entry per refusal naming what it is for and how to drive it red. The mutations in the implementer's log already show how for several of them.

### D3, scroll blanks the viewport on the debug build, and the instrument cannot see it

**Where**: `examples/folder-viewer/src/page.html:1850` (a 503 becomes a fault), `examples/folder-viewer/src/serve.rs:112` and `:163` (a slot is held by a thread whose client aborted), `tools/bench/src/viewer_work.mjs:357` (blank sampled only at a draw) and `:176` (release only)
**What**: on the debug build, an ArrowDown per animation frame aborts prefetch and main-view requests. Each aborted request leaves a server thread holding a connection slot while it waits for one of the two permits. The browser opens new connections, the 32-slot cap is reached, and `/preview` answers 503 `busy`. `requestGrey` turns that into `showFault`, and `syncStage` hides the stage. The runner records `hidden` in a microtask after a `putImageData`, so a stage hidden between draws is never sampled. It also only ever builds `--release`.
**Why it is wrong**: AC15 is "Scroll never blanks the viewport". Open question A's approved answer includes "zero blank samples on debug". The plan's Verification section says a missing run is a missing observation and does not become a passed row. The same runner also cannot reject AC15's named wrong implementation: the `88ddaa1` page scores 109.03 fps and 0 blanks on release, so 109 fps with 0 blanks says nothing about the fix.
**Evidence**: debug runs in scratch gave official `blank_samples: 0` against per-callback hidden counts of 209/361 and 74/362. A rerun gave 176/361 with the reason `fault=this process is already serving its connection limit|imageWidth=512|grid=1`.
**Scope and regression**: reproduced on debug. On release it is plausible for any series whose decode is slow enough to queue, for example large radiographs or a slower host. That case is unconfirmed. Smallest fix has three parts. The page retries a 503 for the current index rather than showing the fault, or the server stops counting a connection whose peer has closed before it takes a permit. The scroll runner samples `stage.hidden` in every animation callback. It also runs once on the debug binary for the blank-only half of the threshold.

### D4, peak RSS exceeds the stated bound under the subject the plan defines

**Where**: `tools/bench/src/viewer_work.mjs:804` (`measureResident`)
**What**: the plan defines `viewer.resident_memory` as process start to "end of the four subjects above in one process". The runner does one CT open, one scroll, nine window steps and one DX open. With the four subjects' own workloads in one process (9 opens, 9 plus 9 first images, 5 scrolls, window steps on CT and DX), peak RSS was 799,309,824 and 772,866,048 bytes against the bound 528,194,192.
**Why it is wrong**: AC18 is "Server peak RSS within the stated bound". It is met only by a lighter workload than the plan stated before measurement. The implementer reached the bound by repeated code changes against the lighter workload, and my rerun of that lighter workload measured 494,960,640, which is 47.6 MB above the implementer's 447,348,736.
**Evidence**: two runs of the extended resident subject in scratch, `thresholds_met: false` both times, `held` at the end 250,883,272 of 268,435,456.
**Scope and regression**: AC18 is open. The bound's inputs are correct: `S_max` is the real `dx-0000.dcm` length 17,400,538 and `N_max` is 3000 by 2900. Smallest fix is to make the runner execute the defined workload. If it then misses, the miss goes back to the operator rather than being tuned away.

### D5, the committed baseline is the void first run

**Where**: `ci/bench-baseline.json:404` onwards, the `darwin|27.0.0|arm64|Apple_M4_Max|16|51539607552` host class
**What**: the five viewer entries record `viewer.scroll_rate` 51.28 under the retired `setTimeout(16)` instrument, `viewer.window_step` 8.5 ms with DX 132.4 ms, and `viewer.resident_memory` 566,099,968. They carry `thresholds_met: false` for three subjects, load average 9.95, and `why` "from the uncommitted working tree".
**Why it is wrong**: the lead's note says the first run used a shared cargo target and its numbers are void. The resident entry has tolerance 0.25, so the regression gate accepts up to about 707 MB, which is above the design bound. The scroll entry measures a different instrument from the one the amendment installed, so a later comparison is not like for like.
**Evidence**: read from the file. `gate bench` is green with these entries.
**Scope and regression**: AC17's "Five runners and `ci/bench-baseline.json`". Smallest fix is to re-baseline from a valid run of the final tree on an idle host with `--accept --story F-216 --why`, after D3 and D4.

### D6, "each file is posted once" is false for a folder larger than the budget

**Where**: `README.md:185`, `examples/folder-viewer/src/more.html:393`
**What**: "Each file is posted once and held by the process" and "posts each file once and holds it by id". The plan's own Open question B expects a 1,693-file CT at 256 MiB to keep about 500 files and post the rest again. A restart posts every file again. A single-frame file's bytes are dropped once frame 0 is held, so the headers sheet posts it again.
**Why it is wrong**: microscope section 3, a false sentence in README or the two pages is a defect. AC13's own wording is "posted once while held".
**Evidence**: `/held` at the end of the resident pass was 80 files and 366 frames for a 1,693-file folder, so most files were not held and are posted again when shown.
**Scope and regression**: two sentences. Smallest fix is to say each file is posted once while the process holds it, and posted again after eviction or a restart, or to delete the sentence.

### D7, `ocelli-pixel` changed against the plan's stated non-goal, with no amendment and no LLD

**Where**: `crates/ocelli-pixel/src/lut.rs:629-764`, `crates/ocelli-pixel/src/stored_pixel.rs:215-248`, `crates/ocelli-pixel/src/lib.rs:30`, plan line 99
**What**: the plan's Non-goals list "any change to `ocelli-cache`, `ocelli-pixel`". The diff adds the public `LutChain::stored_sample_table`, `StoredSampleTable` with `map_containers`, `StoredBits::meaningful_bits` and `StoredBits::stored_sample`, and refactors `extract`. The only lead amendment is the scroll instrument. `docs/lld/pixel-pipeline.md` does not mention the new API.
**Why it is wrong**: `.claude/WORKFLOW.md` makes the approved plan the scope. A public API in the pixel crate is the area HLD section 18 guards, and adding it needs a design decision, not a perf log entry.
**Evidence**: `grep -rln StoredSampleTable docs/ .claude/plans` finds nothing. The arithmetic itself is correct by the exhaustive test above, so this finding is about scope and record, not pixels.
**Scope and regression**: smallest fix is a lead amendment recording the scope change and why the DX 70 ms budget needed it (the perf log's 100 ms `display_index` loop), plus a `pixel-pipeline.md` paragraph naming the table as `apply` per stored integer.

## Smells

### S1, an over-cap connection buffers a whole body before its 503

**Where**: `examples/folder-viewer/src/serve.rs:112` and `:163`
**What**: `serve_on` spawns a thread for every accepted connection before the cap check. The refused thread calls `read_request`, which reserves `declared` bytes, up to `BODY_CAP` of 1 GiB, outside the budget, the permits and the RSS bound.
**Failure mechanism**: the plan's AC11 lists "Unbounded threads" as the wrong implementation it rejects. Thread count and memory on the busy path are bounded only by the clients. The page alone reaches this path (D3).
**Smallest fix**: drain a refused request through a fixed buffer without keeping it, or answer 503 from the accept loop before spawning.

### S2, the page heap bound counts four DX-sized tile canvases the workload never sizes

**Where**: `tools/bench/src/viewer_work.mjs:847`
**What**: `pageHeapBound` adds `4 * 2900 * 3000` for the main view and for four tiles. The measured page never enters grid mode. With the main view only, the formula gives 118,686,080. The measured heap of 159,669,335 is inside 257,886,080 only because of the tile terms.
**Failure mechanism**: a bound that always passes is not a measurement. The plan's "4 * W * H per canvas" does not say whether W by H is the largest frame or the canvas's own size, so this needs the lead's ruling before AC18's heap half is claimed.
**Smallest fix**: use each canvas's actual size at measurement time, or record the ruling in the plan.

### S3, `subject_story: null` and a note that goes stale at completion

**Where**: `tools/bench/subjects.json` (five viewer rows), `docs/lld/benchmarks.md:667`
**What**: the rows name no blocking story "because a pending backlog row is refused", and the LLD says "F-216 is pending in the backlog".
**Failure mechanism**: the sentence becomes false the moment `/complete-feature F-216` runs, and nothing is scheduled to set the story. `bench --list` will then show five subjects with no owner. This is the stale-count shape CLAUDE.md keeps correcting.
**Smallest fix**: set `subject_story` to `F-216` at completion and delete the sentence, or state the rule without naming the backlog state.

### S4, a second DICOM data set walker outside `ocelli-dicom`

**Where**: `examples/folder-viewer/src/preview.rs:1708-1972`
**What**: `split_pixel_payload` and its helpers walk Part 10 element headers by hand to locate the top-level Pixel Data. The location is trusted, and only the header copy is re-parsed.
**Failure mechanism**: today a misalignment makes the header parse fail and the path falls back. Every edge file I built, including a UN undefined-length element, either matched or fell back. The safety depends on that coincidence, though, not on a check. Two concrete gaps: the walker treats every unknown transfer syntax as Explicit VR Little Endian, which is wrong for JPIP Referenced Deflate `1.2.840.10008.1.2.4.95`, whose data set is deflated, and the borrowed range is never compared with the element offset `ocelli-dicom` would report.
**Smallest fix**: return `None` for any syntax the walker does not name, including `.4.95`, and assert the walked value length equals the header's declared Pixel Data length before borrowing. Better still, move the offset into `ocelli-dicom`, which owns parsing.

## Nitpicks

- **N1** `.claude/plans/F-216-design.md:328`. The lead amendment paragraph sits between table rows, so the `viewer.window_step` and `viewer.resident_memory` rows render as a headerless fragment. Move the paragraph below the table.
- **N2** `examples/folder-viewer/src/held.rs:135`. A held frame is charged `samples.capacity() + size_of::<HeldFrame>()`. The plan says the capacity of the owned buffers, which also includes the window centre and width vectors and the facts strings. The gap is tens of bytes a frame.
- **N3** `examples/folder-viewer/src/held.rs:95`. `u64::from_str_radix` accepts a leading `+` and leading zeros, so `+a.+b` and `0a.b` alias id `a.b`. The plan says lowercase hex.
- **N4** `crates/ocelli-pixel/src/stored_pixel.rs:221, 235`. The new public `meaningful_bits` and `stored_sample` are not `const` or `#[must_use]`, unlike their neighbours.

## Verified clean

- Borrowed Pixel Data from the original file bytes (PS3.5 7.1 and A.1 to A.3): byte order per syntax, explicit against implicit VR, 8, 16 and 32-bit containers, Bits Stored and High Bit masking, sign extension from `BitsStored - 1`, multi-frame offsets, and fallback for odd lengths, padding, undefined lengths, encapsulated and deflated data. Byte-identical with `88ddaa1` across 156 borrowed files and 23 fallbacks.
- Refusal ordering. Every error code across 30,072 rows equals `88ddaa1`, including the Modality LUT Sequence and VOI LUT Sequence refusals, colour, palette, YBR, frame out of range and not Part 10.
- `stored_sample_table` equals `apply` exhaustively. Signed tables index negative values by their masked pattern. Masking below Bits Allocated holds. The table refuses widths above 16 bits. Two hand values match C.11.1, C.11.2.1.2 and C.11.2.1.3.2. Seven mutations went red.
- Protocol: upload once, id form `generation.serial` in lowercase hex, 410 `file-gone` for evicted, unknown and other-process ids, a file larger than the budget answered `X-Ocelli-Held: 0` with no id, an id with a body refused 400 `header`, keep-alive, pipelining that keeps surplus bytes, the 32 cap answering 503, 2 permits, one `Lru` with no second cache, single-frame bytes dropped after frame 0 and multi-frame bytes kept until every frame is held, accounting by capacity, `OCELLI_CACHE_MIB` parsing and its messages. The pinned strings in `tests/preview.rs` are unchanged, and only its import line moved.
- Page: `ensureFile` with a shared in-flight post, a single 410 re-post for previews and headers, abort controllers for thumbnails, prefetch and tiles, the main view coalesced to the latest index, image and labels committed together in `applyGrey`, reused `ImageData` buffers, and the `Uint32Array` table as channel copy with no window arithmetic. Restart recovery was observed (`restart_recovered: true`).
- Measurements: my release figures match the implementer's within noise, apart from RSS (D4). The bound's inputs come from the real files. The scroll runner follows the amendment: one key per animation frame, presented frames counted on an index change, the timer ceiling reported (53.48/s). No budget, tolerance or formula was changed against the plan.
- Docs: the README and `use.html` release commands, the `OCELLI_CACHE_MIB` sentence, and "about ten times slower on a window step". That last claim was measured now as 11.9 times on CT and 11.7 times on DX through the page, which is close enough. `cache.md` and `benchmarks.md` are accurate apart from S3. The `postDataBuffer` sentence was not re-verified.

## Limitations and handoff

- The host was loaded (Spotlight and antivirus), so the timing figures are indicative and are not a baseline. RSS was measured twice under the full workload and once under the runner's workload.
- AC13 is covered by the runner only for open, prefetch, thumbnails and window steps. Invert, preset, live W/L and tiles were not exercised with body counting.
- The release-build half of D3 is unconfirmed. Pixel Data inside an Enhanced object's functional groups was not exercised beyond the corpus `ct_multiframe_perframe.dcm`.
- Next owner: implementation, for D1 to D6 and S1 to S4. D7 and S2 also need the lead, for a plan amendment and a ruling. A fresh independent pass is required after remediation, and it should replay D1's request, D3's debug scroll with per-callback sampling, and D4's full workload.
