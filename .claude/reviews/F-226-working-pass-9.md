# F-226 review, pass 9

**Reviewed**: range `599f9aeb..f2aea2c1` on `work/f-226-xai`, one comment-only commit, "F-226, pass 8 doc comment fixes", closing pass 8 D1 to D3 in `examples/folder-viewer/src/held.rs`. HEAD `f2aea2c1`, `git write-tree` `f116ce31`, matching HEAD's tree. No unstaged tracked paths. Untracked at start and end: `.venv` and `tools/oracle/out`, neither touched. Then a sweep of every doc comment and LLD sentence in `git diff d9c2d477..HEAD` about the posted bytes' lifetime, `Flight`, `in_flight`, `doomed` or the page record.
**Author / reviewer**: the commit is the lead's. This pass is an independent Claude reviewer that wrote none of the reviewed material. The reviewer added only this report. The scratch copy was outside the worktree, in the session scratchpad under `f226-review9`, with its own target dir, and was deleted with Python `shutil` at the end.
**Result**: 1 defect, 0 smells, 0 nitpicks

## Acceptance challenged

- **Pass 8 D1, the `in_flight` field doc**, now "Flights per id, made where `preview_source` hands out posted bytes and released when each [`Flight`] drops." `take_off` has one caller, `preview_source` in its `Shared` arm (`held.rs` line 1284), and `Flight::drop` calls `land`, which decrements and removes the id at zero (lines 1700 to 1710). A scratch test forgot an id after its frame was stored while the flight was still held, the state a request is in during the grey wait. `in_flight` held 1 id until the flight dropped, then 0. The sentence holds.
- **Pass 8 D2, the `doomed` field doc**, now "Ids forgotten while a flight was out for them." `forget_ids` dooms an id exactly when `in_flight` has it (line 1637), `store_frame_with` returns early for a doomed id (line 1322), and `land` removes it at zero. In the same scratch test the forget after the store reported `files: 0, frames: 1`, left `doomed` at 1 with the flight out, the next `preview_source` returned `Gone`, and both sets were empty once the flight dropped. The sentence holds.
- **Pass 8 D3, forget removing `HeldKey::File` during a decode.** `forget_ids` removes the file whatever a decode holds (lines 1642 to 1648). `a_forget_during_a_decode_is_not_undone_by_its_store` passed in the scratch copy. The "or the id is forgotten" half of both new sentences holds. The sentences are still false, as D1 below shows, because forget is not the only thing that takes the cache's `Arc` out during a decode.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo test -p ocelli-folder-viewer --lib review9 -- --nocapture` | `git archive HEAD` into the scratchpad `f226-review9/src`, own target dir, a `#[cfg(test)] mod review9` appended to `held.rs` with two tests | now | 0, 2 passed. Eviction test: `shared true in_flight 1 a_held false b_held true second_preview_gone true doomed 0`. Forget after store test: `file_after_store false`, forget `files: 0, frames: 1, bytes: 25136`, `in_flight 1 doomed 1 next_gone true`, settled `0 0` |
| `cargo test -p ocelli-folder-viewer --lib -- a_forget a_forgotten` | same copy | now | 0, 6 passed, the AC12 unit test, the three route tests and `a_forgotten_decode_that_waits_for_grey_answers_gone` among them |
| Code reading | `held.rs` lines 160 to 230, 355 to 385, 1230 to 1340, 1625 to 1715, 1720 to 1980, 2050 to 2060, 2264 to 2330. `serve.rs` lines 1025 to 1090, 1340 to 1380. `docs/lld/cache.md` lines 250 to 268 | now | as cited below |

## Defects

### D1, the posted bytes are said to stay in the cache during a decode unless the id is forgotten

**Where**: `examples/folder-viewer/src/held.rs` lines 169 to 171, the `PreviewSource` type doc, "The posted bytes stay in the cache during a decode unless the id is forgotten, so a second preview or `/headers` still finds them.", and lines 176 and 177, the `Shared` variant, "The cache keeps its own `Arc` until the frame is stored or the id is forgotten." Both were rewritten in the reviewed commit.
**What**: eviction also takes the posted bytes out mid-decode. `evict_oldest_unread` passes over only a `Held::Frame` whose `Arc` a reply shares (line 1890), and `evict_oldest` passes over nothing, so a `Held::File` whose `Arc` a decode holds is evictable. Both run under budget pressure from `insert_posted` and `store_frame_with` through `reserve_for_insert` (lines 1248 and 1329), and from the decode's own `claim_buffer` through `make_room_dropping` (line 1744). In the scratch test a 64-byte budget held posted file A, `preview_source(A, 0)` returned `Shared` and its flight was taken, then posting file B evicted A. With no forget and no store, `file_bytes(A)` was `None`, a second `preview_source(A, 0)` returned `Gone`, and `doomed` was empty.
**Why it is wrong**: a false lifetime claim in prose, microscope section 3. The new text names forget as the only exception, which makes a claim the old text did not, and the "so a second preview or `/headers` still finds them" clause is false in the same run.
**Evidence**: the scratch eviction test above.
**Scope and regression**: both sentences are in this story's diff since the reviewed commit. The eviction behaviour predates the story and is not itself a defect here. Smallest fix, following microscope section 3's preference for deletion: drop the lifetime clause from both. For example "A preview's view of one id." for the type doc and "Posted bytes, shared with the cache." for the variant.

## Smells

None.

## Nitpicks

None.

## Verified clean

- `held.rs` `Flight` doc, `SharedBytes` and its `take_flight` doc, `PreviewSource::take_flight` doc, the `take_off` and `land` docs, and the `pages` field doc. Each matches the code read above, and nothing in them claims the posted bytes stay in the cache.
- `held.rs` `forget_ids` doc and `store_frame_with`'s doomed comment at line 1323. Executed by the forget after store scratch test and the AC12 test.
- `serve.rs` comment at line 1032, "Kept to the end of this round, after the frame is stored". `_flight` is a local of the round, and the Congested arm stores before `wait_for_grey` and then `continue`s (lines 1068 to 1090).
- `docs/lld/cache.md` lines 255 to 268. A forget during the decode makes the Congested store insert nothing, and a forget during the wait removes the stored frame, so the next round's `preview_source` is `Gone` either way, as the scratch test's `next_gone true` shows. Pass 8's reading of the flight lifetime and the bound by requests in flight still holds, since the reviewed commit changed no code.
- The plan's state table row in the story's diff. No new prose semicolon or dash in the changed lines.

## Limitations and handoff

- Not run: the full test suite, `prose_check.py`, any gate, the browser job. The range is comment only.
- `.claude/plans/F-226-design.md` line 329 says the file stays in the cache until `store_frame_with` removes it, which eviction and forget both contradict. It is outside the story's diff and is not counted.
- This pass is blocking on D1, prose only. After remediation, a fresh independent pass on the changed sentences.
