# F-210 review, pass 3

**Reviewed**: `git diff bb0419f HEAD` on `work/f-210-xai`, HEAD `222edbe44bca74cac235d088e6887a33146db67c`, tree `520bfaa19681d6c1a1728aa106f109874778a81e`. That covers `e78ce77` (the lead's claim ruling and AC12 note in the plan) and `222edbe` (the pass 2 fixes). Nothing was staged or unstaged before or after the pass. The only untracked paths are `.claude/reviews/F-210-working-pass-1.md`, `.claude/reviews/F-210-working-pass-2.md` and this report.
**Author / reviewer**: The remediation is by a Claude implementer agent and the plan sections by the sprint lead. This review is by a separate Claude reviewer agent that wrote none of F-210, its plan, its remediations or the earlier reviews. All of them are the same model family, which limits independence.
**Result**: 1 defect, 2 smells, 3 nitpicks

## Acceptance challenged

- **Pass 2 closure.** D1 replayed by swapping `held.rs` back to `bb0419f` under HEAD's tests: `the_steal_route_makes_room_for_a_long_unit` goes red, and it is the only test that does (M0). S1, S2 and N3 each go red under the mutation that removes what they guard (M4, M11, M12, M13). S3 is closed by `e78ce77`, whose two sentences I checked against `UnitText::from_file` (65 characters refused `attribute` for Rescale Type and Units alike) and against every `claim_buffer` route.
- **The F-216 cache change, `make_room_dropping` through `evict_oldest_unread`.** Challenged against F-216's eviction rules (`.claude/plans/F-216-design.md` lines 229, 467, 477, 483, 512) with independent probes and ten mutations. The answers:
  - **Loop.** It cannot loop forever. `evict_oldest_unread` is bounded by `lru.len()` at entry, a reader it re-inserts takes a new tick and so is not the oldest again within the scan, and each turn of `make_room_dropping`'s outer loop either drops a free buffer, evicts at least one entry net, or returns.
  - **Starvation.** A cache of frames that are all being read makes the claim give up with nothing evicted (probe P1: 2 readers kept, `used` 880 of 890, claim fresh). The store's `reserve_for_insert` still uses `evict_oldest`, so the store then evicts the oldest reader and `used + free` stays within the budget (P1 after the store: 880 of 890). No claim waits, so nothing starves.
  - **Budget.** `make_room_dropping` never adds bytes, and `reserve_for_insert` is unchanged, so `used + free <= budget` holds at every store. `the_held_bytes_never_exceed_the_budget` and `cached_frames_and_free_buffers_share_one_budget` pass.
  - **Short claim.** `reserve_for_insert` still catches one. Each claim-term deletion (M4 the `ValueFacts` term, M8 the unit bound) turns a test red because the store then evicts the frame being read.
  - **Resident bound.** The bound is unchanged. Before this change the claim evicted a frame a reply held, which dropped it from `used` while its memory stayed live. Now an unread frame is evicted instead, which frees real memory. The F-216 accounting invariant, line 229, still holds.
  - **Free-list rules.** Line 467 (cached plus free never over the budget) and line 477 (the steal takes the oldest frame of that length only when no reply holds it) are unchanged in code.
  - **Body-form previews.** Line 483 still holds, because `claim_parked_buffer` does not reach `make_room_dropping`.
  - **The grey gate's arrival-order queue.** Line 512's queue, the `GreyGate` FIFO, shares no code with `CacheState`, and its five tests pass.
  - **Which frames are evicted.** This does change. A reader at the LRU end now moves to the most recent end. A failed steal attempt in F-216 already did the same, through `take_oldest_frame_buffer`'s re-insert. The steal route now also makes room for the whole claim, so on a full cache whose slack is under the claim's overstatement it evicts one extra entry (N1).
- **The lead ruling's last clause**, "a frame a reply is reading is never evicted by a longer unit". It holds at HEAD for one reader (the new test) and for two readers at the LRU end (P2 and P3, both readers kept). It holds only because the scan looks past the first entry, and nothing tests that (S1).
- **The AC12 note**, "a later change that gives the value its own stage 1 goes red". It was executed: giving `value_facts` its own slope-1 stage 1 turns `value_and_preview_agree_on_stage_one` red, with `fractional: value Some((1.0, -1024.5)) and preview Some((0.3333, -1024.5)) differ`.
- **The steal test's claim that the steal happened.** It does not hold (S2).

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo fmt --all --check` | HEAD tree above | now | 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | HEAD, `CARGO_TARGET_DIR=<wt>/target` | now | 0 |
| `cargo test -p ocelli-folder-viewer -p ocelli-pixel -p ocelli-dicom` | HEAD | now | 0, every binary `ok`, the folder viewer lib 71 passed |
| `npm run lint`, `npm run typecheck` | HEAD | now | 0, 0 |
| `bin/ocelli.sh gate guards`, `bin/ocelli.sh gate bench` | HEAD | now | 0 ALL GREEN, 0 ALL GREEN |
| `python3 scripts/guard_census.py`, `--check-runbook` | HEAD | now | 0, 0. "0 watched by nothing" |
| `python3 scripts/prose_check.py`, `scripts/source_provenance_check.py` | HEAD | now | 0 "voice rules clean over 878 file(s)", 0 "clean over 1734 files" |
| Corpus: `borrowed_bytes_match_the_parsed_pixels_for_every_accepted_corpus_file`, `recycled_grey_matches_a_fresh_decode_for_every_accepted_corpus_file`, AC12 `value_and_preview_agree_on_stage_one` | scratch copy of HEAD under `target/`, byte-identical `src/` (cmp), `corpus/data` symlinked to the main checkout | now | 3 passed in 2.78 s |
| M0 `held.rs` at `bb0419f` (pass 2 D1 replay) | scratch copy, own target dir | now | red: `the_steal_route_makes_room_for_a_long_unit` only |
| M1 steal route returns without room | same | now | red: the steal test |
| M2 `make_room_dropping` through `evict_oldest` | same | now | red: the steal test |
| M3 `take_oldest_frame_buffer` never steals | same | now | red: only F-216's `cached_frames_and_free_buffers_share_one_budget`. **The steal test stays green** (S2) |
| M4 claim without the `ValueFacts` term | same | now | red: `the_claim_charges_the_value_facts_struct` |
| M5 no `touch_frame` on a spared reader | same | now | red: `cached_frames_and_free_buffers_share_one_budget` |
| M6 the scan looks at one entry (`for _ in 0..1`) | same | now | **green, whole suite** (S1) |
| M7 `reserve_for_insert` spares readers too | same | now | green. Benign: `Lru::insert` still enforces the budget, and the reader tests observe the property and not the mechanism |
| M8 claim without the unit bound | same | now | red: three unit tests |
| M9 reader predicate `> 2`, M10 a reader is never re-inserted | same | now | red: the steal test, both |
| M11 padding refusal `unwrap_or(None)` | same | now | red: `value_refuses_a_malformed_padding_range` |
| M12 non-finite check removed, M13 non-finite as `window` | same | now | red: `value_refuses_a_value_outside_f32`, both |
| AC12 against a value with its own slope-1 stage 1 | same | now | red, `fractional` case |
| P1 all readers, claim then store, minimal frames, budget 2 x 440 + 10 | scratch probe in `held.rs` tests | now, HEAD and `bb0419f` | HEAD: claim gives up, 2 kept, store keeps `used` 880 of 890. `bb0419f`: claim evicts both readers |
| P2 readers R1 and R2, then unread X, then B with a 64-letter unit, synthetic slices, budget 3F + file B + 63 | scratch probe in `serve.rs` tests | now, HEAD, `bb0419f`, and M6 | HEAD: R1 and R2 200 `held`, X 410, `buffer_claims` (0, 3) to (0, 4), so B's claim was **fresh and X was evicted, not stolen**. `bb0419f`: R2 410. M6: green, because R1 was already moved to the newest end by the failed steal attempt |
| P3 readers R1 and R2 oldest, unread U, claim of another length (no steal), budget 1791 | scratch probe in `held.rs` tests | now, HEAD and M6 | HEAD: both readers kept, U evicted. M6: **R2 evicted while read**, U kept |
| P4 full cache of 3 same-length frames, 5 steal-route decodes, budget 3 x 440 + 10 | scratch probe in `held.rs` tests | now, HEAD and `bb0419f` | frames after each store: HEAD `[2, 2, 2, 2, 2]`, `bb0419f` `[3, 3, 3, 3, 3]`, all 5 claims reused in both (N1) |
| `bin/ocelli.sh bench viewer.resident_memory viewer.open` | HEAD | not run | The 1-minute load stayed between 6.3 and 25.3 for the whole pass, from other processes (sampled every 15 s), so the condition for running it never held |

Scratch copies lived in `target/f210-p3/` with their own `CARGO_TARGET_DIR`. They were deleted with `shutil.rmtree` after the corpus symlink was unlinked, and the main checkout's `corpus/data` was confirmed intact. One early mutation run shared the worktree's target dir with a second scratch copy. Its result was discarded and the catalogue was rerun in an isolated target. Afterwards I touched the worktree's `examples/folder-viewer/src/*.rs` and `tests/*.rs` so that no cross-copy artefact would be taken as fresh.

## Defects

### D1, the claim's doc comment names a steal the code does not make

**Where**: `examples/folder-viewer/src/held.rs:1498` to `1504`, the doc comment on `CacheState::claim_buffer`: "Every route makes room for the whole `frame_sample_cost` of the buffer it returns: a parked buffer, a buffer stolen from the oldest unread frame of the same length, and a fresh one."
**What**: `take_oldest_frame_buffer` takes the *first* frame of that length in `frame_order`. If a reply is reading that frame, it returns `None` and looks no further. It does not steal "the oldest unread frame of the same length". When the oldest same-length frame is being read, a newer unread frame of the same length is not stolen. The claim goes fresh, and `make_room_dropping` then evicts that unread frame and discards its buffer.
**Why it is wrong**: The comment states a behaviour the code does not have, and it was added in this round. F-216's rule is the code's behaviour: "The steal takes the oldest cached frame of that length only when no reply holds it, and otherwise allocates" (`.claude/plans/F-216-design.md:477`). `README.md` line 185 and `docs/lld/cache.md` line 182 state the same rule correctly. A maintainer who trusts the new comment will think the steal already skips readers, and that is the very assumption pass 2's D1 turned on.
**Evidence**: P2 at HEAD. R1 and R2 are being read and X is unread, all of one sample length. B's claim reports `buffer_claims` going from (0, 3) to (0, 4), which is fresh and not reused, and X then answers 410. X was evicted rather than stolen.
**Scope and regression**: Prose only, on current HEAD. Behaviour matches F-216's rule. The smallest fix is "a buffer stolen from the oldest frame of the same length when no reply reads it". Do not add sentences explaining it.

## Smells

### S1, the scan past the first entry in `evict_oldest_unread` is unguarded

**Where**: `examples/folder-viewer/src/held.rs:1663` to `1689`, `for _ in 0..self.lru.len()`, which the doc comment describes as "Each entry is looked at once at most".
**What**: M6 cuts the scan to one entry, and every test in the suite passes. With M6, a claim that finds one reader at the LRU end moves it to the newest end, evicts nothing and gives up. The store's `reserve_for_insert` then evicts the next oldest entry, and if that is a second reader, the frame being read is evicted. That is the outcome the ruling in `e78ce77` says never happens.
**Failure mechanism**: P3 is two readers at the LRU end, an unread frame U behind them, and a claim whose length matches no cached frame. At HEAD both readers stay and U is evicted. Under M6, R2 is evicted while it is read and U stays, and the suite is green. The new steal test cannot catch this, because it has one reader. P2 cannot either, because the failed steal attempt already moves the first reader to the newest end. A refactor that turns the scan into a single `if` therefore ships green, and the page then sees a 410 and re-posts.
**Regression**: A `held.rs` unit test shaped like P3. Store two frames whose `Arc`s the test holds, store one unread frame, claim a buffer of a length no cached frame has, at a budget where evicting the unread frame is enough, then store. Require both readers held and `used + free` within the budget. It must go red under `for _ in 0..1`.

### S2, the steal test's "the steal happened" check does not discriminate

**Where**: `examples/folder-viewer/src/serve.rs`, `the_steal_route_makes_room_for_a_long_unit`, the block commented "The steal happened: X's frame left and its bytes had already gone." It requires X to answer 410, and its message says "so it was not stolen".
**What**: X answers 410 whether its buffer was stolen or the frame was simply evicted. M3 disables stealing altogether, and this test stays green. Only F-216's `cached_frames_and_free_buffers_share_one_budget` goes red. P2 shows the same 410 from a plain eviction on a fresh claim.
**Failure mechanism**: The test's purpose is to regress the steal route, pass 2's D1. If a fixture change gives B a different sample length, or a frame-order change leaves X off the front, the steal route stops being exercised. The test still passes, and its comment and error message would then make a false claim about coverage. The steal-route mutation M1 is red at HEAD only because the layout happens to steal today.
**Regression**: Read `viewer.buffer_claims()` before and after B's preview and require the reused count to rise by one (fresh unchanged), so that the test fails under M3. The 410 check can stay as a second observation.

## Nitpicks

- **N1. The steal route's full-cost room costs one cache entry on an exactly full cache.** `bb0419f`'s steal route made no room, and a stolen frame of one length freed what the next one charged. HEAD makes room for `frame_sample_cost`, which overstates a rule-unit grey frame by `UNIT_MAX_BYTES` (256) and a facts-less frame by 376. When the cache's slack after the steal is under that overstatement, one more whole entry is evicted, and the cache then runs one frame short. P4 shows this at HEAD as `[2, 2, 2, 2, 2]` frames against `bb0419f`'s `[3, 3, 3, 3, 3]`, at a slack of 10 bytes. It follows from the lead ruling ("every route ... makes room for that whole cost") and keeps every F-216 rule. It occurs only when the slack is under about 376 bytes, which is rare for real frame sizes. The implementer's earlier note, "A cache of tiny frames makes room a little early", understates it. The extra cost is one entry, of any size, at most once per such step. Worth one line to the lead, who accepted that note, and no change is required.
- **N2.** `forget_evicted` now holds the count decrement, but `take_oldest_frame_buffer` (line 1744) and `absorb` (line 1771) still open-code the same `match` on `HeldKey`. That makes three copies of the bookkeeping that this round set out to make one.
- **N3.** The plan's Alternatives paragraph (`.claude/plans/F-210-design.md:200`) still says the value uses `open_frame` and that "AC12 checks that the two agree". That line predates the rulings and is superseded by them, since the plan's rulings hold where they differ. The new AC12 note at line 300 states the current position. Not introduced by this diff.
- Carried and not counted again: pass 2 N2 (`read_padding` is a forwarding wrapper, open decision 2), pass 1 N2 to N4.

## Verified clean

- **`claim_buffer`'s three routes.** The parked route, the steal route (through `reuse_claimed`) and the fresh route each make room for, or fit, `frame_sample_cost` of the buffer returned. `reuse_claimed` sizes from `buffer.capacity()`, which is the buffer's actual charge, and the buffer is off the free list, so it is not in `charged()`.
- **`evict_oldest_unread`.**
  - The `reading` test is `Arc::strong_count > 1` on frames only. Holders other than the LRU are transient render paths (`serve.rs` 1124, 1233, 1313, 1403, 1429). A file being decoded can still be evicted, as before (open decision 1).
  - Re-insert restores the original budget, so it cannot evict, and the `evicted` and `refused` arms are defensive.
  - `touch_frame` keeps `frame_order` in step, and M5 shows that it is needed.
  - `forget_evicted` does what both old copies did.
- **`evict_oldest` and `evict_oldest_keyed`.** The split leaves `evict_oldest` behaving as before, and `reserve_for_insert` is unchanged.
- **The new tests' arithmetic.**
  - `the_claim_charges_the_value_facts_struct`: 64 copies of U+10000 are 256 UTF-8 bytes (RFC 3629, 4 bytes each), and the budget `2F + 256 + file B - 1` leaves the claim short by exactly the 120-byte struct when that term is removed. M4 is red.
  - `value_refuses_a_value_outside_f32`: 65535 x 1e38 = 6.5535e42, above the largest finite f32 3.4028235e38. `1e38` is a valid DS under PS3.5 Table 6.2-1.
  - `value_refuses_a_malformed_padding_range`: a limit without a value, and value 10 above limit 0 in MONOCHROME2. Both match PS3.3 C.7.5.1.1.2 as cited and are refused by `read_padding`. M11 is red.
- **The test helpers** `short_element`, `splice_before` and `slice_one_with` keep tag order: (0008,0005) before (0008,0016), (0020,4000) before (0028,0002), (0028,1054) before (0028,1056). They pad odd values to even length. ISO_IR 192 is added only for a non-ASCII unit.
- **README step 7**, executed against `modality_unit`:
  - "HU on a CT whose Rescale Type is HU": the file's text `HU`.
  - "or is absent on a CT that is not multi-energy": `ct-default` for Multi-energy absent or `NO`.
  - "`unspecified` for Rescale Type `US`": the `US` arm.
  - "the Units attribute on a PET": the PT arm when Units is present.
- **The page sentence.** The `toggle-cross` sentence says the same in other words, avoiding the word Rescale that the page contract refuses. It is pinned by `the_page_offers_a_folder_picker`, which passes.
- **The other documents.** `use.html`, `more.html` and the `README.md` cache paragraph (line 185) have no sentence this diff makes false. "The cache evicts to make room for a frame or for a free buffer" and the steal sentence still describe the code.
- **The plan sections from `e78ce77`.** The 64-character bound and the `attribute` refusal match `UnitText::from_file` for both Rescale Type and Units. The PS3.5 Table 6.2-1 citation for LO is right.

## Limitations and handoff

- **Bench.** `viewer.resident_memory` and `viewer.open` were not run, because the 1-minute load never fell below 6. There is no new measurement against the baseline (445,956,096 bytes at 0.12 tolerance, 1018 ms at 0.06). The change can only lower resident memory, because a claim now frees an unread frame where it used to drop a read one from the accounting only, but that is reasoning and not a measurement.
- **Not exercised.**
  - A browser (AC16 is for sprint close) and pydicom (AC17).
  - Concurrent readers dropping their `Arc` during a scan. The lock is held, and a count can only fall, so the worst case is a reader kept that could have gone.
  - Multi-frame posted files evicted by the steal route's new room-making. The file is touched on each miss, so it is rarely the oldest.
- **Probes P1 to P4** were scratch tests and were deleted with the copies. Their shapes are written above so that the regressions for S1 and S2 can be rebuilt.
- **Next owner.** The implementer fixes D1, S1 and S2. N1 goes to the lead as information. A fresh independent pass 4 then reviews the change. No finding here has survived three passes. D1, S1 and S2 are new with `222edbe`.
