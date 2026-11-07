# F-226 review, pass 6

**Reviewed**: range `32461c96..5a2e7f08` on `work/f-226-xai`, the folder viewer half, one commit, "F-226, review pass 5 remediation". HEAD `5a2e7f08`, `git write-tree` `e01b7b17`, matching HEAD's tree. No unstaged tracked paths. Untracked at start and end: `.venv` and `tools/oracle/out`, neither touched.
**Author / reviewer**: the remediation is a Claude implementer's. This pass is an independent Claude reviewer that wrote none of the reviewed material and none of pass 5. The reviewer only added this report and scratch files under ignored `target/f226-review6-mut`, deleted after the report with Python `shutil`.
**Result**: 2 defects, 0 smells, 2 nitpicks

## Acceptance challenged

- **Pass 5 D1, replayed through the real caller.** The new test `a_forgotten_decode_that_waits_for_grey_answers_gone` is pass 5's own trigger: grey cap full, both decode permits held, preview sent, forget sent, permits released, stalls dropped. It answers 410 and leaves one frame, the stalls'. It goes red when `store_frame_with` ignores `doomed` (M1), when the pooled route drops its flight at once (M2) and when the pooled route drops its flight just before the congested store (M14, a new mutation one step from the fix). The corrected README and `docs/lld/cache.md` sentences were executed path by path: a forget during a `/value` decode answers 200, during a `/measure` decode 200, during a direct decode 200, during a pooled decode 200, and during a decode that then waits for grey room 410. Each leaves 0 files, 0 frames and 0 bytes of the forgotten id. Those sentences hold. A different sentence of the same rewritten paragraph does not, see D1.
- **Pass 5 D2.** AC10 now holds frames 0 and 1 of a three-frame object. M7, the range `Frame(id, 0)..=Frame(id, 0)`, now turns `forget_drops_files_and_frames` red. The adjacent case, a forget whose range ends at frame 1, passes AC10 and is caught only by my scratch test that holds frames 0 and 2. That falsifies the new test comment, see D2.
- **Pass 5 S1.** M3 (`decode_for_value` drops its flight at once) turns `a_forget_during_a_value_decode_is_not_undone_by_its_store` red, and my scratch `/measure` variant red as well. M4 (the direct `preview_held` drops its flight at once) turns `a_forget_during_a_direct_decode_is_not_undone_by_its_store` red. M2 turns the pooled AC12 test red as before. The direct test's viewer comes from `Viewer::with_connection_cap`, which has no scheduler, so it does run the direct route.
- **Adjacent cases derived here.** A preview and a `/value` of one id both waiting for a permit with the posted bytes, one forget: both 200, nothing held, and red under M1 and M2. A forget during the decode and a second forget during the grey wait: the second answers `{"files":0,"frames":0,"bytes":0,"unknown":1}`, the preview 410, one frame left. Under M1, M2 or M14 the second forget instead finds the stored frame, `{"files":0,"frames":1,"bytes":524848,"unknown":0}`, and the test goes red. No forget during the decode and two during the wait: `{"files":0,"frames":1,...}` then `unknown` 1, and the preview 410, which is F-216's path.
- **Pass 5 nitpicks.** N1: M11 (the sweep keeps only ids whose posted bytes are held) turns the new `a_sweep_keeps_an_id_whose_frame_is_held` red, and so does M10 (no sweep). I checked that every churn post in that test is accepted, so its "the sweep ran" check is not vacuous. N2: `hide.other` is now measured from `GET /held` after the second page drew, and `forgetProblems` refuses `other` 0 or absent as well as a mismatch, executed with five inputs. N3: the console line now prints the count and the largest size, observed as "1 forget requests, the largest 37 bytes". N4: the README now says the cache budget, which `OCELLI_CACHE_MIB` sets. N5 needed no change.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo test -p ocelli-folder-viewer -p ocelli-cache` | worktree, HEAD `5a2e7f08`, default target | now | 0. folder viewer lib 81, measure 14, preview 79, value 24, and the rest ok. cache 35, bricks 24, budget 4, image_cache 13, invariants 2, telemetry 13 |
| `bin/ocelli.sh gate viewer-browser` | worktree, release build, synthetic folders, port 0 | now, the only browser job | exit 0, ALL GREEN. forget: `left` B 3 files, 0 files and 3 frames held, A 410 x4. `late` 410 and 0 posts. `hide` 3400 ids, 1 forget request of 37 bytes, 3 held, the second page's measured 3. `leave` 0 held |
| `prose_check.py`, `source_provenance_check.py` | worktree | now | both exit 0 |
| `guard_census.py`, `--check-runbook` | worktree | now | "0 watched by nothing", runbook exit 0 |
| `guard_probe.py --only viewer-browser.forget-left-folder` and `--only viewer-browser.forget-late-post` | worktree | now | both OK, red for the declared reason, control green |
| `forgetProblems` with five `hide` inputs, `node --input-type=module` | worktree `forget.mjs` | now | `held 3 other 3` no problem. `other 0` and `other` absent refused as "held nothing". `held 0 other 3` and `held 4 other 3` refused as a mismatch |
| Rust mutations, `cargo test -p ocelli-folder-viewer --no-fail-fast` | `git archive HEAD` into `target/f226-review6-mut/src`, own `CARGO_TARGET_DIR` `target/f226-review6-mut/target`, scratch tests added. Each mutation applied alone, files restored from a saved base and compared byte for byte after | now | P0 control exit 0. See the mutation table |
| Unmutated scratch lib suite, six runs | same copy | now | 85 passed, 6 of 6 |
| `review6_waiters_hold_flights` | scratch: four 512 by 512 files posted, grey cap filled by two stalls, four previews sent, four waiting, then a `#[cfg(test)]` read of `in_flight.len()` | now | 4 ids in `in_flight` with `inflight` bytes 0 and two decode threads, 0 after the waits ended and all four answered 200 |

Mutation table, against the scratch copy with the scratch tests:

| ID | Mutation | Red tests |
|----|----------|-----------|
| M1 | `store_frame_with` ignores `doomed` | held AC12, pooled AC12, the new value, direct and grey-wait tests, scratch measure, two flights, forget twice |
| M2 | pooled route drops its flight at once | pooled AC12, new grey-wait test, scratch two flights, forget twice |
| M3 | `decode_for_value` drops its flight at once | new value test, scratch measure |
| M4 | direct `preview_held` drops its flight at once | new direct test. Also `a_pooled_server_posts_and_previews_on_two_workers` once, which does not run the direct route, see Limitations |
| M7 | frame range ends at `Frame(id, 0)` | `forget_drops_files_and_frames`, scratch frames 0 and 2 |
| M7b | frame range ends at `Frame(id, 1)` | scratch frames 0 and 2 only. AC10 stays green |
| M10 | `sweep_pages` never sweeps | new sweep test, `page_records_of_evicted_files_are_swept` |
| M11 | sweep keeps only ids whose posted bytes are held | new sweep test |
| M14 | pooled route drops its flight before the congested store | new grey-wait test, scratch forget twice |

## Defects

### D1, `docs/lld/cache.md` says the in-flight and doomed sets are bounded by the decodes in flight, and a request waiting for grey room keeps its entry with no decode running

**Where**: `docs/lld/cache.md` line 265, "and the last one out for an id takes it out of both sets, so both are bounded by the decodes in flight", in the paragraph this range rewrote. The cause is `examples/folder-viewer/src/serve.rs` line 1034, where the flight is kept to the end of the round, and the round's grey wait at line 1084.
**What**: on the decode workers, a round whose render finds no grey room stores its frame, drops its permit and waits for room for up to the 60 second write timeout while `_flight` is still alive. Its id stays in `in_flight` for the whole wait, and a forget during the wait adds it to `doomed`. So the sets hold one entry per waiting request, not one per decode.
**Why it is wrong**: a false claim in prose, microscope section 3. The plan's decision 6 states the true bound, "bounded by the requests in flight, which F-216 caps at 32 connections". The LLD narrowed it to decodes. The paragraph's own new sentences describe the very wait during which the narrower claim fails. The behaviour does no harm, since the sets are still bounded by the connection cap and the waiting id's frame is already stored.
**Evidence**: `review6_waiters_hold_flights` in the scratch copy. With two decode threads and no decode running, four waiting previews hold four `in_flight` ids, and `inflight` bytes is 0. All four answer 200 once the stalls drop, and the count returns to 0.
**Scope and regression**: present on HEAD. The sentence was already in the text pass 5 reviewed, and this range rewrapped it into the rewritten paragraph. Smallest fix: restore the plan's wording, bounded by the requests holding posted bytes, which the connection cap bounds. Dropping the flight after the congested store and before the wait would also make the sentence true, since a forget during the wait then removes the stored frame and the next round answers 410. If the code changes, M14's case still has to go red and the scratch measurement has to show at most two ids.

### D2, the AC10 comment says a forget keeping any frame past 0 fails the test, and one keeping frames 2 and up passes

**Where**: `examples/folder-viewer/tests/preview.rs` lines 3780 to 3781, "Frame 1 is held so that a forget keeping any frame past 0 fails here."
**What**: only frames 0 and 1 are held, so a forget whose range ends at `Frame(id, 1)` (M7b) keeps every frame from 2 up and the whole suite stays green. My scratch test that holds frames 0 and 2 of the same three-frame object goes red under it.
**Why it is wrong**: a false claim in prose about what the test proves, microscope sections 2 and 3. The comment will be read as coverage of the whole range that decision 5 names, `Frame(id, 0)..=Frame(id, u32::MAX)`. The code on HEAD is correct and the test does reject AC10's named wrong implementation and pass 5's M7.
**Evidence**: M7b in the mutation table.
**Scope and regression**: a test comment on HEAD only. Smallest fix: say "a forget keeping frame 1 fails here", or delete the sentence. No finite fixture covers every range end, so the comment should name the frame it holds.

## Smells

None.

## Nitpicks

- **N1.** README line 189 says the request is answered "with 410 Gone when its render had to wait for grey room", and the LLD that the round after the wait "answers 410 Gone". A wait that gets no room within the write timeout, or whose client leaves, answers 503 busy from `wait_for_grey` instead, read from `serve.rs` line 1084 and not executed here because it takes 60 seconds. The README's earlier paragraph states the 503 rule for every waiting preview, so a reader is not misled for long.
- **N2.** The plan's state table row "A decode for A's id in flight" still says "the decode returns its reply", which pass 5 found holds only without a grey wait. The LLD and README now say the exception. The plan is the lead's, so this is a handoff, not an implementer change.

## Verified clean

- **Every pass 5 finding is closed.** D1's README and LLD sentences about the reply hold on all five routes executed above. D2's M7 is red. S1's M3 and M4 are red, and the `/measure` path, which shares `decode_for_value`, is red under M3 through my scratch test. N1 to N4 are closed as listed under Acceptance challenged.
- **The new tests discriminate.** Each goes red under the mutation it was written for and the adjacent one: the value test under M1 and M3, the direct test under M1 and M4, the grey-wait test under M1, M2 and M14, the sweep test under M10 and M11, AC10 under M7. None goes red under the P0 control, in six repeated runs of the lib suite.
- **The new tests' prose.** The grey-wait test's "Afterwards only the stalls' 3000 by 2900 frame is held" matches its census check of 0 files and 1 frame. The value test's "The value is still answered" is checked as a 200. The sweep test's budget, one frame and one 64-byte post, keeps the frame through every churn post, which it checks as a `Hit` after each, and every churn post is accepted. AC10's "Frame 2 is not decoded, so the posted bytes stay beside them" matches the census of 2 files and 2 frames before the forget.
- **`forget.mjs`.** The `hide` doc comment's "measured before the first page opened its folder" matches `observeHide`, which reads the census after the second page drew and before the first page is created. The catalogue's `refuses` text, "does not count exactly the second page's files and frames", is now what the check measures. The gate passes with the new `other` of 3.
- **README line 189 apart from N1**, "the cache budget, which `OCELLI_CACHE_MIB` sets" included. `use.html` and `more.html` are untouched by the range, and their forget sentences (`use.html` section 12, `more.html` `#worker`) still match the route.
- No `unsafe`, no `as` cast, no `wasm-bindgen` and no pixel arithmetic in the diff. No new prose semicolon or dash, `prose_check.py` exit 0.

## Limitations and handoff

- Not exercised: AC16, the bench rerun on the F-216 host. The 503 end of a grey wait (N1). bfcache restore. No GPU or oracle job ran.
- External, not F-226 material: `a_pooled_server_posts_and_previews_on_two_workers` failed once in eighteen runs of the lib suite, during M4's run, with "after preview inflight 0 frames 1 workspace 0". M4 changes only the direct route, which that test's scheduled viewer does not run, and the six unmutated reruns passed. The assertion comes from F-216's `ae7c9c9e`. It reads `workspace` straight after the reply, which plausibly races the worker returning its workspace. Unconfirmed, recorded for F-216's owner.
- Scratch: `target/f226-review6-mut` held the archive copy, the scratch tests, `mut.py`, the saved bases and the logs. It is deleted with Python `shutil` after this report. The implementer's earlier scratch under `target/` is no longer present.
- This pass is blocking on D1 and D2, both prose, with the code on HEAD behaving correctly. After remediation, a fresh independent pass on the changed material. This is a bounded review of the folder viewer half, not completed acceptance or sprint readiness.
