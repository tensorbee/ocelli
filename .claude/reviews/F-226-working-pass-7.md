# F-226 review, pass 7

**Reviewed**: range `5a2e7f08..c126017e` on `work/f-226-xai`, one prose-only commit, "F-226, pass 6 prose fixes". HEAD `c126017e`, `git write-tree` `ac0226f4`, matching HEAD's tree. No unstaged tracked paths. Untracked at start and end: `.venv` and `tools/oracle/out`, neither touched.
**Author / reviewer**: the commit is the lead's. This pass is an independent Claude reviewer that wrote none of the reviewed material. The reviewer only added this report and a scratch copy under ignored `target/f226-review7`, deleted with Python `shutil` before this report was written.
**Result**: 2 defects, 0 smells, 0 nitpicks

## Acceptance challenged

- **Pass 6 N1, the 503 end of a grey wait, now executed.** In a `git archive HEAD` copy with its own `CARGO_TARGET_DIR`, the only code change was the deadline in `wait_for_grey`, from `WRITE_TIMEOUT` to 2 seconds, so the stalls' 60 second socket timeout could not release room first. A scratch test followed `a_forgotten_decode_that_waits_for_grey_answers_gone` but kept the stalls. During the wait the sets held 1 doomed id and 1 in-flight id. The preview answered `HTTP/1.1 503 Service Unavailable` after 2.02 s. Afterwards both sets were empty and the cache held 0 files and 1 frame, the stalls'. The code path agrees: `serve.rs` line 1084 returns `grey_busy_reply()` when `wait_for_grey` gives `None`, whether or not the id is doomed. The LLD's "or 503 if the wait itself runs out", the README's "or 503 if that wait runs out" and the plan row's "or 410 Gone after a wait for grey room" hold. The plan row does not name 503, but it does not claim 410 is the only answer.
- **Pass 6 D1, the bound.** "Both are bounded by the requests in flight, which F-216 caps at 32 connections" holds. `CONNECTION_CAP` is 32 in `held.rs` line 33, and the process server is `Viewer::server` (`serve.rs` line 155), which uses it. `handle_connection_with` refuses a connection past the cap with 503. `connection_session` serves one request at a time. `take_off`, the only place a flight is made, is called only from `preview_source`, and each route holds at most one flight at a time, because the worker route's round 1 flight drops at `continue` before round 2 looks up again. `forget_ids` adds an id to `doomed` only when it is in `in_flight`, and `land` takes it out of both. One sentence used to argue the bound is not true as written, see D1.
- **Pass 6 D2, the AC10 comment.** The comment now says "a forget keeping frame 1 fails here". In the scratch copy, a forget range ending at `Frame(id, 0)` keeps frame 1, and `forget_drops_files_and_frames` went red with `{"files":2,"frames":1,"bytes":1594,"unknown":2}`, not the expected `frames 2`, `bytes 2162`. The test holds frames 0 and 1 of a three-frame object, as the comment says.
- **Pass 6 N2.** The plan row no longer says the decode returns its reply unconditionally.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo test -p ocelli-folder-viewer --lib review7` | `git archive HEAD` into `target/f226-review7/src`, own target dir, grey wait deadline 2 s, a `#[cfg(test)]` reader of the two sets, one scratch test | now | 0. During the wait doomed 1, in flight 1. 503 at 2.02 s. After, both 0, census 0 files and 1 frame |
| `cargo test -p ocelli-folder-viewer --test preview forget_drops_files_and_frames` | same copy, plus the forget range ending at `Frame(id, 0)` | now | 101, red with frames 1 and bytes 1594 |
| Code reading | `serve.rs` lines 155, 263 to 294, 310 to 340, 942 to 1001, 1004 to 1124, 1231 to 1290, 1449 to 1470. `held.rs` lines 33, 170 to 230, 975 to 993, 1271 to 1295, 1517 to 1530, 1631 to 1707 | now | as cited in the findings |

## Defects

### D1, the LLD says a flight lasts the whole request, and it ends before the reply is written and is never taken by a cache hit

**Where**: `docs/lld/cache.md` line 265, "A flight lasts the whole request, including a wait for grey room", the sentence this range added.
**What**: a `Flight` is made only by `preview_source` when it hands out posted bytes (`held.rs` line 1283). A request whose frame is a cache hit gets `PreviewSource::Hit` and holds no flight at all (`held.rs` lines 183 to 188). On the decode workers, `_flight` is a local of the round (`serve.rs` line 1034). It does survive the grey wait, which my scratch test saw as 1 in-flight id during the wait. But `continue` drops it, so round 2 renders without it, and `return reply_with_home(...)` drops it before `connection_session` writes the reply. The direct route and `decode_for_value` also drop it when they return, before the write.
**Why it is wrong**: a false claim in prose, microscope section 3. The conclusion drawn from it, the bound by requests in flight, is true, because no request holds more than one flight at a time. The premise is broader than the code and invites a reader to think a forget during a reply's write is deferred.
**Evidence**: code reading above. The scratch test's 1 in-flight id during the wait confirms the part that is true.
**Scope and regression**: introduced by this range. Smallest fix: "A flight lasts from the lookup that hands out the posted bytes until after the store, and on the decode workers through the grey wait that follows it", keeping the bound sentence as it is.

### D2, the `Flight` doc comment still says both sets are bounded by the decodes in flight

**Where**: `examples/folder-viewer/src/held.rs` lines 216 to 219, "The last one out for an id that was forgotten meanwhile takes the id out of the doomed set, so both stay bounded by the decodes in flight."
**What**: this is pass 6 D1's false claim, which the range corrected in `docs/lld/cache.md` and left in the doc comment of the type it describes. A request waiting for grey room holds a flight with no decode running, as pass 6 measured with four waiters and this pass saw with one.
**Why it is wrong**: a false claim in prose, microscope section 3, and the same finding a third time once it is counted with pass 6 D1. The code comment sits next to the code a maintainer will read first.
**Evidence**: the scratch test above, doomed 1 and in flight 1 while the request waited with both decode permits free.
**Scope and regression**: present on HEAD and before this range, not touched by it. Smallest fix: "so both stay bounded by the requests in flight", the LLD's wording.

## Smells

None.

## Nitpicks

None.

## Verified clean

- README line 189's new clause, "or 503 if that wait runs out", executed above.
- The LLD's "or 503 if the wait itself runs out" and "bounded by the requests in flight, which F-216 caps at 32 connections", with the cap found at `held.rs` line 33 and wired through `Viewer::server`.
- The AC10 comment, red under a forget that keeps frame 1.
- The plan's state table row, which matches the two answers executed in pass 6 and this pass.
- No code changed in the range. No new prose semicolon or dash in the changed lines.

## Limitations and handoff

- Not run: the full test suite, `prose_check.py`, any gate, the browser job. The range is prose only and the scratch copy compiled the unchanged code.
- The 503 was observed with a 2 second wait, not the 60 second one. The deadline is the only thing the scratch change touched.
- This pass is blocking on D1 and D2, both prose. After remediation, a fresh independent pass on the changed sentences.
