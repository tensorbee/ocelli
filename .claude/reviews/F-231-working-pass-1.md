# F-231 review, pass 1

**Reviewed**: `git diff 03c57a2 747ca66` as F-231's own change, and `git show 5fa5a69` as the merge of F-224's reviewed library fixes. HEAD `5fa5a69b08f4aa661c236b667605e262f7c4c06f`, index tree `c4d8c25a6ae4e307200bb8c3b35ba3ff4186e518`, worktree `ocelli-wt-f-231` on `work/f-231-xai`, no unstaged or untracked paths before this report.
**Author / reviewer**: implementation by GLM-5.3 in opencode (747ca66). Merge resolution by a Claude agent (5fa5a69). Reviewer is an independent Claude agent that authored neither. It did not write the design plan either.
**Result**: 4 defects, 3 smells, 3 nitpicks

## Acceptance challenged

- **AC2, AC5, AC7, AC8, AC10, AC11 re-derived by hand.** AC2: budget 3,145,728. After two frames `2 * 2,097,152 = 4,194,304 >= 3,145,728` and `10 * 2,097,152 = 20,971,520 < 28,311,552`, Elevated. After three `31,457,280 >= 28,311,552`, Critical. The fourth evicts index 0 at 1,048,576 with no band event. AC5: 12 of 12 is Critical (`120 >= 108`), 8 of 12 is Elevated (`80 < 108`, `16 >= 12`), so the stale show's removal is one crossing. AC7: `41,943,040 >= 37,748,736` Critical, then `2,097,152 < 4,194,304` Low. AC8: two LRU evictions of 1,048,576, total 2,097,152, and Critical both sides (`10,485,760 >= 9,437,184`). AC10: 8 bytes against 4 refused. AC11: 256 held plus 44 dropped is 300. Every test asserts these values. The payload offsets in the tests match `TelemetryEvent::encode` and `CacheEvent` encoding.
- **AC12 re-derived independently** in Python from the FNV-1a 64 definition with 8-byte little-endian length prefixes: `0x49e8b650ff1cd3f1`. The same fold gives the published FNV-1a vector for `"a"`, `0xaf63dc4c8601ec8c`, so the fold itself is right.
- **AC1**: 512 * 512 * 4 = 1,048,576 and 4 * 4 * 3 * 4 = 192, asserted.
- **Selection against show.** A discriminating case the suite does not contain: show a cached index that is not the selected one. Reproduced on native and on the built module, D1.
- **Content without a stack selection** through `writeFrame`, D2.
- **Pressure payload counters** after an eviction, D3.
- **Budget 0 then non-zero then 0** through `set_frame_budget`: probe gave `[5]`, then `[4, 4]` for two inserts, then `[1, 1, 5, 4]` with `used` 0. Order is F-034's. The trailing band event is unguarded, S1.
- **`release_series` at the default budget** emits nothing (`PressureBand` at budget 0 and used 0 is Low), so AC3 holds for a release too.
- **`configure_stack` with other keys** at the same index: residency 0. Returning to the first stack: show 1. The cache follows the key, not the index.
- **Extent against sample count**: a 2 by 2 commit of 1 sample is cached and resident at 2 by 2. That matches what `commit_current_frame` already accepts for display, so it is recorded as N3 and not a new defect.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo fmt --all --check` | HEAD | now | 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | HEAD | now | 0 |
| `cargo test -p ocelli-cache -p ocelli-viewport -p ocelli-wasm` | HEAD | now | 0. `ocelli_wasm` lib 47 tests, 13 of them `session_cache` (listed with `--list`). `image_cache` 20, `ring` 2 |
| `cargo test -p ocelli-viewport --test typegen` | HEAD | now | 0, 2 passed |
| `bin/ocelli.sh wasm`, then `python3 scripts/pin_and_size_check.py --with-size` | HEAD | now | 0. 2,202,427 bytes, ceiling 2,258,755. Decode 16,455 |
| `bin/ocelli.sh wasm` on `git archive 03c57a2` in its own target dir | parent | now | 0. 2,179,308 bytes. Growth 23,119, 1.06%. GLM's receipt said 2,202,389 for the pre-merge tree, and the 38-byte difference is the merge |
| `ci/check-bindgen-isolation.sh` | HEAD | now | 0, confined to `ocelli-wasm` |
| generated `pkg/ocelli_wasm.d.ts`, the seven new exports | HEAD release build | now | returns are `number` and `bigint \| undefined` only |
| `wasm-pack build crates/ocelli-wasm --dev --target web --out-dir target/boundary-wasm` | HEAD | now | 0. The module on disk was built at 20:16, before the 20:33 merge, so it was rebuilt before the boundary run |
| `npx vitest run packages/core` | HEAD | now | 0, 49 files, 455 tests |
| `npx vitest run --config vitest.boundary.config.ts` | HEAD dev module | now | 0, 16 tests |
| `npm run typecheck`, `npm run lint` | HEAD | now | 0, 0 |
| `event_catalogue_check.py` | HEAD | now | 0, 50 core and 53 tool names |
| `guard_census.py`, `--check-runbook` | HEAD | now | 0, 1102 refusals in 78 files, all claimed |
| `prose_check.py`, `source_provenance_check.py` | HEAD | now | 0, 0 |
| Rust mutations M1 to M10, scratch `git archive HEAD` under `target/rv231`, own target dir | HEAD | now | nine red, M8 green (S1). Table below |
| TS mutations T1 to T8, same scratch copy | HEAD | now | six red, T6 and T7 green (S2) |
| Native probes in the scratch copy | HEAD | now | D1, budget walk, other-stack residency, extent mismatch |
| Built-module probe appended to the scratch boundary file | HEAD dev module | now | D1 on the built module |
| GLM's ten mutations | 747ca66 | prior receipt, not re-run except where overlapping mine | reported red |

Mutations, each applied to one site in the scratch copy, run, and restored.

| ID | Mutation | Result |
|----|----------|--------|
| M1 | stale show pushes no band event | red, `show_resident_displays_and_stale_refuses` |
| M2 | `is_resident` ignores rows | red, `residency_is_not_a_use` |
| M3 | `push_with_seq` does not advance `seq` | red, the AC11 ring test |
| M4 | `content_id` swaps the UID and transfer syntax fields | red, `content_id_hashes_the_slot_and_drops_it` |
| M5 | `set_frame_budget` halves each evicted byte count | red, AC8 |
| M6 | `lookup` keeps a stale entry | red, four `image_cache` tests across both tiers |
| M7 | the budget-zero guard in `cache_committed_frame` removed | red, AC3 |
| M8 | `emit_band` after `set_frame_budget` removed | **green**, S1 |
| M9 | `show_resident` leaves `last_frame` unwritten | red, AC5 |
| M10 | a commit caches under the last stack key | red, six tests |
| T1 | `releaseStudy` without the duplicate skip | red, AC14 |
| T2 | `releaseStudy` validates lazily | red, AC15 |
| T3 | `contentId` does not release on a throw | red |
| T4 | `refuseUnknownOptions` removed from the request | red, two tests |
| T5 | `frame-cache.ts` `u64` loses its upper bound | red, AC15 |
| T6 | `bulk.ts` `u64` loses its upper bound | **green**, S2 |
| T7 | `frameCommitMeta` passes `content` unvalidated | **green**, S2 |
| T8 | a stack commit with content routed to `commit_selected_frame` | red |

## Defects

### D1, `show_resident` displays a frame the viewport has not selected

**Where**: `crates/ocelli-wasm/src/session.rs:445`, `show_resident`. The tests that encode it: `show_resident_displays_and_stale_refuses` at line 3004, and `packages/core/src/boundary.wasm.test.ts:798`.
**What**: `show_resident` checks the epoch and the cache entry but not `self.viewport.scroll().index() == Some(index)`. `commit_selected_frame` refuses exactly this case with `SessionError::Selection`. So a shell at slice 0 can call `showResident({ index: 1, ... })`, and the session adopts slice 1's samples, clears the loading flag and draws them while `copy_state` reports slice 0.
**Why it is wrong**: the plan says `show_resident` adopts the frame "exactly as `commit_current_frame` does after its checks", and the only route into the displayed frame for a stack today is `commit_selected_frame`, documented as "Commit only the selected immutable frame in the currently installed stack". Showing a frame for another index is the quietly wrong pixel CLAUDE.md names: the image on screen is not the slice the state, the scroll bar and any measurement say it is, and the loading flag that would have warned is cleared.
**Evidence**: native probe in the scratch copy. Stack `[(5,0), (5,1)]`, budget 8, commit index 0 at 10.0, scroll +1, commit index 1 at 20.0, scroll -1. Then `show_resident(1, epoch, CONTENTS[1], 1, 1)` printed `shown=1 selected=Some(0) first=[0, 0, 160, 65]`, which is 20.0, index 1's sample. On the built dev module through `@ocelli/core`, the same sequence gave `{ shown: true, sliceIndex: 0 }`. The committed AC5 test does the same thing (two `scroll(-1)` take the selection to index 0 before `show_resident(1, ...)` returns 1), and AC13 shows index 1 while index 3 is selected, so both tests assert the wrong behaviour.
**Scope and regression**: present on HEAD. Affects every shell that calls `showResident`, and the folder viewer once it does. Smallest regression: the probe above asserting `show_resident` returns 0 and leaves the display and loading flag unchanged when `index` is not the selected index, plus AC5 and AC13 corrected to scroll to the index first. `frame_resident` is a query and needs no selection check.

### D2, a `content` id without a stack selection is silently ignored and never validated, while the docs say it fills the cache

**Where**: `packages/core/src/bulk.ts:493` and `:500`. `README.md:309`. `examples/folder-viewer/src/use.html:391`.
**What**: `frameCommitMeta` validates and keeps `content` only when `stackIndex` or `stackEpoch` is present. Otherwise the key is accepted, not validated and dropped, and the commit goes to `commit_frame`. README says "A commit that carries a `content` id fills the cache", and use.html says "A commit that carries a `content` id is also cached, if the session's frame budget admits it".
**Why it is wrong**: plan decision 4 says only `commit_cached_frame` fills the cache, and the plan's API section routes "a meta with `stackIndex`, `stackEpoch` and `content`". F-230's rule, which the plan adopts for this key, is that a bad value is refused with `InvalidValue`, not passed or dropped. GLM's open decision 3 chose silence, which leaves a caller who believes it is caching with no signal and accepts values that are not ids at all.
**Evidence**: scratch vitest probe. `writeFrame(wasm, sink, bytes, { viewport: 1, columns: 1, rows: 1, generation: 0, content: "not a u64" })` and the same with `content: 7n` both succeeded, and the stub recorded `["commit_frame", "commit_frame"]`.
**Scope and regression**: present on HEAD. Two acceptable fixes, which are the lead's choice: refuse `content` without a selection with `InvalidValue` (or `UnknownOption`), or validate it always and document that only a stack commit is cached. Either way the two user-document sentences must name the stack selection. Regression: a `bulk.test.ts` case for the no-selection route.

### D3, every pressure event reports `evicted` and `refused` as zero

**Where**: `crates/ocelli-wasm/src/session.rs:580` to `:590`, `pressure_reading`.
**What**: the reading hard-codes `evicted: 0`, `evicted_bytes: 0` and `refused: 0`. A crossing or a requested reading that follows evictions or refusals reports none.
**Why it is wrong**: `TierTelemetry` documents `evicted` as "Entries evicted to make room since the previous reading" and `refused` as "Entries the budget could not hold since the previous reading", and both are on the wire (`docs/lld/events.md` row 4, `CachePressurePayload` in `events.ts`). F-034 ships `TierCounters` with `record` and `take` for exactly this, so "the session keeps no per reading counters" (GLM's open decision 1) is a choice not to use the existing owner, and the value it produces is false. In AC2's sequence, a `request_frame_pressure()` after the fourth commit reports `evicted 0` with one eviction since the previous reading.
**Evidence**: source at the lines above. No test reads offsets 24 or 28 of a pressure payload after an eviction or a refusal.
**Scope and regression**: present on HEAD. The shell's telemetry consumer, and any pressure display. Smallest regression: AC2 extended with a `request_frame_pressure()` after the eviction asserting `evicted` 1 at payload offset 24, and AC10 with `refused` 1 at offset 28. The count can be folded from the `CacheEvents` the session already iterates, and reset at each pressure push.

### D4, two user-document statements are wrong

**Where**: `examples/folder-viewer/src/use.html:392` to `:397`, and `README.md:311` to `:312`.
**What**: the use.html sample calls `writeStack(session, keys)` but its import list does not include `writeStack` (it imports `frameResident`, which the sample never calls). Copied as written it throws a `ReferenceError`. README says `releaseSeries` and `releaseStudy` empty the cache "one event per series through the ring", but a series with nothing cached emits no event (F-226's `release_series` returns none, which `ImageCache::release_series` documents), and `releaseSeries` empties one series rather than the cache.
**Why it is wrong**: microscope section 3 puts every API sentence of README and use.html in scope, and a false one is a defect.
**Evidence**: `grep -n writeStack examples/folder-viewer/src/use.html` shows the call at 397 and no import. Release behaviour from `ImageCache::release_series` and probe PROBE2d, which printed `release=0 kinds=[]` for an empty series.
**Scope and regression**: present on HEAD. Fix the import list and say "one event per series that held frames".

## Smells

### S1, the band-crossing half of `set_frame_budget` has no test

**Where**: `crates/ocelli-wasm/src/session.rs:355`.
**What**: mutation M8 removed the `emit_band()` after the `BudgetChanged` push and every test stayed green. AC8's fixture stays Critical on both sides, and the boundary test's budget change goes Low to Low. The probe shows the branch is live: `set_frame_budget(0)` over two cached frames produces `[1, 1, 5, 4]`, and the trailing pressure event is the one nothing watches.
**Hazard**: a refactor that drops or reorders that call ships silently, and AC8's "then a band event only if it crossed" is half-proved. Regression: the `[1, 1, 5, 4]` case, or a shrink from Critical to Low.

### S2, `FrameCommitMeta.content` validation is unwatched

**Where**: `packages/core/src/bulk.ts:500` and the `u64` helper below it.
**What**: T6 (no upper bound) and T7 (no validation at all) both stayed green. A `content` of `2n ** 64n` or a number would reach `commit_cached_frame`, where wasm-bindgen's `u64` conversion wraps or throws a `TypeError`, not F-230's `InvalidValue`. The plan puts this key under F-230's refusal rules.
**Hazard**: the refusal exists and looks authoritative, and nothing executes it. Regression: a `bulk.test.ts` case per bad value with zero export calls asserted. Lands naturally with D2's fix.

### S3, the LLD names a type the merged tree no longer has

**Where**: `docs/lld/cache.md:24` and `:226`.
**What**: F-224's review fix, merged in 5fa5a69 without conflict, describes `ImageCache` as `Lru<FrameKey, HeldFrame>` with "the private `HeldFrame`". F-231 renamed it to `HeldValue<V>` and made `ImageCache` generic, so both lines are false on HEAD.
**Hazard**: the plan schedules `cache.md` for `/complete-feature`, and an author adding F-231's rows can leave these two in place. Recorded here so that pass rewrites them rather than appends.

## Nitpicks

- N1. `packages/core/src/boundary.wasm.test.ts`, the AC13 `it` closes with `});` at column 0 inside the `describe`. Lint passes.
- N2. `releaseStudy` given a non-array throws a `TypeError` from `.map`, not an `OcelliError`. Not in AC15's list.
- N3. A committed frame is cached with the extent the commit named even when `columns * rows` differs from the sample count. The display path already accepts that, so it is parity, but a cached frame inherits it.

## Verified clean

- **Merge resolution, 5fa5a69.** F-224's stale-bytes fix sits in the shared `ImageCache<V>::lookup`, so both tiers take the removal's own byte count. The decoded tier's `get` and the committed tier's `get` both route through it, and M6 turned four tests red across both tiers. Both sides' tests survive: every test name in `308a0f0`'s `image_cache.rs` is in HEAD, including F-224's `a_hit_refreshes_recency_so_the_second_is_evicted`, and F-231's three committed-tier tests are there too. The import-list hunk drops `Lru`, `CacheTier` and `TierCounters`, and the file still compiles and passes 20 tests. `content_id.rs` 3 tests pass.
- **Content id on every lookup.** `get` on both tiers and `is_resident` compare `ContentId`, columns and rows (decoded tier: description and plane). `show_resident` with a stale entry removes it, returns 0, leaves the display, emits no cache event and one band event on a crossing (M1, AC5).
- **Residency is not a use.** `is_resident` goes through the new `Lru::peek`, which takes `&self` and moves no tick. Proven by the next eviction in both `image_cache.rs` and `session_cache` (M2). `Lru::peek` is outside the plan's write set and is the smallest addition that makes the plan's `&self` query possible.
- **`configure_stack` keeps the cache across an epoch**, residency 1 under the new epoch and 0 under the old (AC6). The cache follows the `FrameKey`, so a different stack at the same index answers 0.
- **`set_frame_budget` evicts least recently used first**, then one `BudgetChanged` with count and bytes from the returned evictions, then the band (M5, AC8). Budget is `u32`, converted with `usize::try_from`.
- **A frame larger than the budget** is displayed and refused with one `cache.refused` naming the key and 8 bytes, `used` unchanged (AC10).
- **AC9.** A 3-byte commit and a contentless `commit_selected_frame` are displayed and not cached, with no event.
- **AC11.** 300 pushes into an empty ring: 256 held with sequence numbers 0 to 255, 44 in `dropped` at header offset 12, nothing overwritten. The session half reaches the same figures through Displaced events.
- **AC3.** Budget 0 inserts nothing and emits nothing (M7). Release at budget 0 emits nothing.
- **AC19 and D3 of the HLD.** `mod bind` adds seven exports returning `u32` or `Option<u64>`, and the generated `.d.ts` shows only `number` and `bigint | undefined`. No export returns samples. `drain_events` is unchanged.
- **D2 of the HLD.** `wasm-bindgen` only in `ocelli-wasm`, by the script and by the export list living in `session.rs`'s `mod bind`.
- **Casts.** The diff adds no `as` cast, no `#[allow]` and no `unsafe`. Every width change is `try_from` with a stated fallback, or `u32::from`.
- **Render-loop allocation.** `present` and the draw path are not touched. The new allocations (`stored_samples.clone()` for an insert, buffer growth in `show_resident`) happen in commit and show calls, which the plan places outside the frame loop.
- **TypeScript refusals (AC15).** Index, epoch, columns, rows and budget through F-230's `InvalidValue`, unknown request keys through `UnknownOption`, a non-object through `InvalidOptions`, all before any export call (T2, T4, T5). `ResidentFrameRequest` joins `OPTION_KEY_LISTS` with a completeness check, and `FrameCommitMeta`'s list gains `content`.
- **`releaseStudy` validates every entry before the first call**, skips duplicates and sums (T1, T2, AC14).
- **AC12 and AC13 through the built wasm**, after rebuilding the dev module from HEAD. `contentId` returns the hand-derived value and drops the slot. The `u64` series and contents carry bits above 32.
- **AC16.** The `IMAGE_CACHE_IMAGE_REMOVED` row names the session as producer, and the catalogue script passes.
- **AC17.** Parent 2,179,308, HEAD 2,202,427, growth 23,119 bytes (1.06%) against a 5% tolerance on the 2,151,196 baseline. No `--accept-size` run.
- **Docs executed.** README's other sentences (budget default 0, `setFrameBudget(wasm, bytes)`, no bytes crossing on a show, display survives a release) hold against the tree and the probes. use.html 5.3's new sentence holds. use.html section 12's `CommittedFrame` paragraph holds. more.html `#worker` holds: the render session's cache is built, decode in the worker is not, and decode instances hold no session cache. The use.html "six options" line and its `cache` row are F-224's and were not reviewed as F-231 material.
- **GLM's eight open decisions.** 1 rejected (D3). 2 accepted: `BudgetChanged` names the decoded tier and the ring's live `dropped`. 3 rejected (D2). 4 accepted: a foreign ticket leaves the slot, every other refusal drops it, and `content_id` refuses before hashing. 5 accepted. 6 accepted, consistent with decision 3 of the plan. 7 accepted, comparisons are on little-endian bytes. 8 accepted, an out-of-range index answers as a miss.

## Limitations and handoff

- No GPU, no oracle, no browser worker. The boundary suite runs the dev module under Node with canvas stubs, which is not a worker observation and not a release-size build.
- `CHANGELOG.md` is in the plan's write set and untouched. `WORKFLOW.md` assigns it to `/release-notes`, so it is not a finding here. `docs/lld/` is deferred to `/complete-feature` by the plan, with S3 carried for that pass.
- GLM's own ten mutations are a prior receipt. The overlapping ones (AC3, AC4, AC5, AC8 shapes) were re-run as M7, M2, M1 and M9, M5.
- Scratch copies `target/rv231`, `target/rv231-base` and their target directories were deleted. No tracked file was edited. The dev module under `crates/ocelli-wasm/target/boundary-wasm` and the release `pkg/` were rebuilt from HEAD and are ignored build output. `git status --short` was empty and `git write-tree` was `c4d8c25a6ae4e307200bb8c3b35ba3ff4186e518` after cleanup, before this report.
- Next owner: implementation remediates D1 to D4 and S1 to S3, then a fresh independent pass 2 reviews the changed material and replays D1's probe through the built module.
