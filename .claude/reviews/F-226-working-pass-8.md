# F-226 review, pass 8

**Reviewed**: range `c126017e..599f9aeb` on `work/f-226-xai`, one prose-only commit, "F-226, pass 7 prose fixes". HEAD `599f9aeb`, `git write-tree` `cf4eef1c`, matching HEAD's tree. No unstaged tracked paths. Untracked at start and end: `.venv` and `tools/oracle/out`, neither touched. Then a sweep of every sentence in `git diff d9c2d477..HEAD -- '*.md' '*.rs' '*.html' '*.mjs'` that states a bound, lifetime or count for `in_flight`, `doomed`, a `Flight` or the page record.
**Author / reviewer**: the commit is the lead's. This pass is an independent Claude reviewer that wrote none of the reviewed material. The reviewer added this report and a scratch copy under ignored `target/f226-review8`. Deleting the scratch copy was refused by the permission system, so it is still there, 1.0 GB, for the operator to remove.
**Result**: 3 defects, 0 smells, 0 nitpicks

## Acceptance challenged

- **Pass 7 D1, the LLD flight lifetime, now executed.** `docs/lld/cache.md` line 265 says a flight lasts from the lookup that hands out the posted bytes until after the store, and on the decode workers through the grey wait that follows it. `take_off` is reached only from `preview_source` in its `Shared` arm (`held.rs` line 1283), so a cache `Hit` holds none. On the decode workers `_flight` is a local of the round (`serve.rs` line 1034), alive through `store_frame_with` and `wait_for_grey` (lines 1072 to 1084) and dropped by `continue` or by `return reply_with_home(...)`, both before `connection_session` writes the reply. The direct path keeps it to the end of the `Shared` arm (line 1267), and `decode_for_value` to its return (line 1465). A scratch test on a `git archive HEAD` copy, described below, saw no flight while the two stalls' replies were being written and one flight while a request waited for grey room after its store. The sentence holds.
- **Pass 7 D2, the `Flight` doc comment.** `held.rs` line 219 now says "bounded by the requests in flight". `forget_ids` dooms an id only when `in_flight` has it (line 1637), and `land` is the only remover from either set and takes the id out of both at zero (lines 1705 and 1706). Every flight lives on a connection thread, each connection serves one request at a time and holds at most one flight, and `Viewer::server` takes `CONNECTION_CAP`, 32. The comment holds.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo test -p ocelli-folder-viewer --lib review8 -- --nocapture` | `git archive HEAD` into `target/f226-review8/src`, own target dir, a `#[cfg(test)]` reader of the two set sizes, one scratch test following `a_forgotten_decode_that_waits_for_grey_answers_gone` with free permits and the forget sent while the request waited for grey room | now | 0. Stalls writing: doomed 0, in flight 0. Waiting before the forget: doomed 0, in flight 1, cache files 0, frames 2, decode inflight bytes 0. Forget reply `{"files":0,"frames":1,...}`. After it: doomed 1, in flight 1. Answer `410 Gone`. Settled: 0 and 0 |
| `cargo test -p ocelli-folder-viewer --lib -- a_forget_during_a_decode_is_not_undone_by_its_store page_records a_sweep_keeps` | same copy | now | 0, 3 passed |
| Code reading | `serve.rs` lines 256 to 297, 1004 to 1124, 1231 to 1290, 1330 to 1395, 1449 to 1520. `held.rs` lines 168 to 230, 360 to 375, 970 to 990, 1235 to 1340, 1405 to 1425, 1625 to 1710, 2262 to 2362, 2450 to 2525 | now | as cited in the findings |

## Defects

### D1, the `in_flight` field doc says it counts decodes holding the posted bytes

**Where**: `examples/folder-viewer/src/held.rs` lines 364 to 366, "Decodes holding each id's posted bytes, counted where `preview_source` hands them out and released when the decode's [`Flight`] drops after its store."
**What**: on the decode workers the job drops the posted bytes after decoding (`drop(bytes)` in `pooled_preview`, `serve.rs` line 1181), and the connection thread then stores the frame and waits for grey room still holding the round's `_flight`. In the scratch test, while the request waited, `in_flight` held 1 id, the cache held 0 files because the one-frame object's posted bytes had already left on its store, and the decode inflight byte count was 0. The count was held by a request with no decode running and no posted bytes held anywhere.
**Why it is wrong**: a false claim in prose, microscope section 3. This is the "decodes" against "requests" claim that pass 6 D1 and pass 7 D2 corrected in the LLD and the `Flight` comment, left in the doc of the field itself.
**Evidence**: the scratch test above, in flight 1 with files 0 and inflight 0 during the wait.
**Scope and regression**: added by this story, outside this pass's range. Smallest fix: "Flights per id, made where `preview_source` hands out posted bytes and released when each [`Flight`] drops." The lifetime is then said once, in the LLD.

### D2, the `doomed` field doc says an id is doomed only while a decode holds its bytes

**Where**: `examples/folder-viewer/src/held.rs` line 368, "Ids forgotten while a decode held their bytes."
**What**: `forget_ids` dooms any id `in_flight` has (line 1637), which includes a request waiting for grey room after its decode finished and its store ran. In the scratch test the forget was sent during that wait, found 0 files and 1 frame, and left `doomed` at 1 with no decode running and the posted bytes already gone from the cache. The request then answered 410 Gone.
**Why it is wrong**: a false claim in prose, microscope section 3, the same class as D1.
**Evidence**: the scratch test above, doomed 1 after the forget during the wait.
**Scope and regression**: added by this story, outside this pass's range. Smallest fix: "Ids forgotten while a flight was out for them."

### D3, the `Shared` variant says the cache keeps its `Arc` until the frame is stored

**Where**: `examples/folder-viewer/src/held.rs` line 176, "Posted bytes. The cache keeps its own `Arc` until the frame is stored.", a line this story edited.
**What**: `forget_ids` removes `HeldKey::File(id)` from the LRU whatever a decode holds (lines 1641 to 1647). `a_forget_during_a_decode_is_not_undone_by_its_store` forgets the id while its decode holds the `SharedBytes`, expects the forget to report `files: 1`, and passed in the scratch copy. The cache's `Arc` left before the frame was stored, and the store then inserted nothing.
**Why it is wrong**: a false lifetime claim in prose, microscope section 3, made false by the forget this story adds. The type doc two lines above it, "The posted bytes stay in the cache for the whole decode" (lines 169 and 170), is pre-existing and false for the same reason.
**Evidence**: the AC12 unit test above, green with `files: 1` removed mid-decode.
**Scope and regression**: line 176 is in this story's diff, lines 169 and 170 are not. Smallest fix: delete the second sentence of line 176, and in the type doc say the bytes stay in the cache for the decode unless a forget removes them.

## Smells

None.

## Nitpicks

None.

## Verified clean

- `docs/lld/cache.md` lines 254 to 268, executed above. The flight lifetime, the bound by requests in flight, the 32 connection cap, and that a forgotten waiting request answers 410 Gone. The scratch test also saw the forget land during the wait rather than during the decode, and the 410 still came.
- `docs/lld/cache.md` line 285, "`sweep_pages` drops the ids that hold nothing once the record passes 4,096 and twice the entries held". `sweep_pages` returns while `pages.len() <= max(PAGE_SWEEP_FLOOR, 2 * held)` (line 1672), and keeps an id with a tracked frame or a held file. Eviction never touches `pages`, whose only remover besides the sweep is `forget_ids` (line 1636).
- `held.rs` `PAGE_SWEEP_FLOOR` doc, the `pages` field doc, the `sweep_pages` doc and the two sweep tests' comments. The "at most the floor plus the one just posted" bound is 4,097 under a one-entry cache, and both tests passed.
- `held.rs` `take_off`, `land`, `Flight`, `SharedBytes` and `PreviewSource::take_flight` docs, and `serve.rs` comments at lines 1032, 994 to 997 and 1463 to 1464.
- The AC12 test comment at `held.rs` lines 2264 to 2272. The test drops its flight after `store_frame`.
- `README.md`, `use.html`, `more.html`, the plan's state table row and the lead ruling. None states a bound or lifetime for these sets beyond what is checked above.
- No code changed in the range. No new prose semicolon or dash in the changed lines.

## Limitations and handoff

- Not run: the full test suite, `prose_check.py`, any gate, the browser job. The range is prose only.
- The sweep covered the four sets named in the brief. D3 is the posted bytes' lifetime, found next to them and confirmed by the AC12 test.
- `target/f226-review8` is left in place because its deletion was refused. It is ignored and holds nothing tracked.
- This pass is blocking on D1, D2 and D3, all prose. After remediation, a fresh independent pass on the changed sentences.
