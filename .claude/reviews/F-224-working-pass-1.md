# F-224 review, pass 1

**Reviewed**: `git diff fc20d84 3792463` (F-224's library and package half, commit 3792463) in worktree `ocelli-wt-f-224`, branch `work/f-224-xai`, HEAD `03c57a2`. Checks ran on HEAD `03c57a2`, clean tree, no unstaged paths. Scope is AC1 to AC12. `examples/folder-viewer` (AC13 to AC18) and AC19's `use.html` and `more.html` halves are out of scope this round.
**Author / reviewer**: author GLM-5.3 in opencode (per the lead's brief and the run log, not established from a trailer). Reviewer Claude Opus 5.5 as an independent `/microscope` subagent. It did not write or remediate any of this code.
**Result**: 2 defects, 1 smell, 2 nitpicks

## Acceptance challenged

- **Merge 03c57a2.** `git diff 3792463 03c57a2` and `git diff fc20d84 c7aa9f3` differ only in index hashes and hunk offsets. The reverse, `git diff c7aa9f3 03c57a2` against `git diff fc20d84 3792463`, has identical `+` and `-` lines. So the merge brought exactly F-226's c7aa9f3 (`image.rs` 3 lines, `lru.rs`, `docs/lld/cache.md` 4 lines) and nothing else.
- **AC1 to AC3.** I wrote an independent Python FNV-1a 64 from the definition (basis `0xcbf29ce484222325`, prime `0x100000001b3`, `h = ((h ^ b) * p) mod 2^64`) with the plan's `<Q` length prefixes. It reproduces all four draft vectors and all six composite values, and every intermediate fold state quoted in the `content_id.rs` comments, including the 24-zero-byte states `0xa8c7f832281a39c5`, `0x88201fb960ff6465`, `0x81d23fd7003c2305`. FNV-1 order gives `0xd8dcca186bafadcb` for `"abc"`, so AC1 rejects it.
- **AC4.** Independently, all 2040 single-byte variants of `00..07` under the same UID and transfer syntax give an id different from the base. A digest of a 4-byte prefix goes red (M4).
- **AC5 to AC7, decoded tier validity.** `pressure().used` worked by hand: 4 x 4 x 2 = 32 per frame at budget 32 walks 32, 32 (hit), 0 (stale), 32, 0. Mutations: never stale, stale without removal, content-only, plane ignored, description ignored, stale bytes 0 all go red. Extra scratch cases I added: Pixel Spacing 1.0 to 1.25, and Bits Stored 16 to 12 with High Bit 11, are both `Stale`. A NaN position is refused by `ImagePositionPatient::new`, so no frame can be stuck stale forever by NaN inequality. A stale removal frees room, so the next insert evicts nothing. A hit refreshes recency at the `ImageCache` level.
- **Stale emits no event, no Displaced after Stale.** `get` returns no `CacheEvents` at all, and the insert after a stale lookup is empty while the live-slot control displaces 32 bytes. The `TierCounters` half is D1.
- **AC8.** `Stale` mapped to `NotResident` goes red, and so does `Stale` mapped to `Ok(&[])`.
- **AC9.** `crates/ocelli-wasm`, `FrameKey`, `HandoffMeta`, `CacheEvent::encode` and F-226's `Released`/`KIND_RELEASED` code are not in the diff's hunks. `cargo test -p ocelli-wasm` 34 passed, and the `handoff_tests` and `released_tests` lib tests pass.
- **F-226 release with `ContentId` in the tier** (scratch test). Series 1 holds two frames, one goes stale, and `release_series(1)` gives `Released { frames: 1, bytes: 32 }` with series 2 still hitting. A series holding two different content ids releases both (`frames: 2, bytes: 64`). A series whose only frame went stale releases nothing and emits nothing.
- **AC10 to AC12.** Ten TypeScript mutations, all red: `cache` dropped from `#get`, default `"default"`, `force-cache` accepted, `only-if-cached` accepted, validation removed, pass-through removed, key removed from either key list, frames alone sent `"default"`, `reload` rewritten to `no-store`. A scratch probe refuses `5`, `null`, `""`, `"NO-CACHE"` and `" no-cache"` with `InvalidRequest`, accepts an explicit `cache: undefined`, and refuses `postWadoInstance` with `force-cache` and `only-if-cached` with `DicomwebError InvalidRequest` and zero fetch calls. `#get` is the only `#fetch` call site in `packages/core/src`.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo fmt --all --check` | HEAD 03c57a2 | now | 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | HEAD, `CARGO_TARGET_DIR=<wt>/target` | now | 0 |
| `cargo test -p ocelli-cache -p ocelli-viewport` | HEAD | now | 0. content_id 3, image_cache 16, lib 36, scroll 6, all ok |
| `cargo test -p ocelli-viewport --test typegen` | HEAD | now | 0, 2 passed |
| `cargo check -p ocelli-cache --target wasm32-unknown-unknown` | HEAD | now | 0 |
| `cargo test -p ocelli-wasm` | HEAD | now | 0, 34 passed |
| `npx vitest run packages/core` | HEAD | now | 0, 440 passed, 48 files |
| `npm run typecheck`, `npm run lint` | HEAD | now | 0, 0 |
| `event_catalogue_check.py`, `guard_census.py`, `guard_census.py --check-runbook`, `prose_check.py`, `source_provenance_check.py` | HEAD | now | all 0. Census "0 watched by nothing" |
| `fnv_check.py` (scratchpad, independent) | FNV definition and plan prefix rule | now | every AC1 to AC4 value agrees |
| Rust mutations M1 to M15 | rsync copy of HEAD under `target/mut-f224`, separate target dir | now | M1 to M12 and M14 red. **M13 green** (D1). **M15 green on `image_cache`** (S1) |
| TS mutations T1 to T10 and edge probe | same copy, worktree's vitest | now | all red, edge probe as above |
| GLM's RED/GREEN and mutation list | `glm-F-224-lib.log` | prior receipt, not relied on | consistent with mine except the AC7 claim in D1 |

Environment: Apple arm64, rustc 1.97.1, Node 24.13.0. No GPU, oracle or browser job. Scratch copies deleted. `git status --porcelain` was empty before this report was written, and HEAD is still `03c57a2`.

## Defects

### D1, AC7's `TierCounters` block cannot fail, and claims a displacement case it never runs
**Where**: `crates/ocelli-cache/tests/image_cache.rs`, `stale_removal_is_explicit`, the final `Lru` block (the comment from "The same removal through the `Lru`" to the closing `assert_eq!(counters.take(), TierCounters::default())`).
**What**: The last assertion checks `counters.take()` after `lru.remove(...)`, but nothing calls `counters.record` between the previous `take()` and that check. The assertion therefore holds for any implementation of `remove`, `get` or the stale path. The comment says "An eviction counts and a displacement does not", but the block does no displacing insert. GLM's log repeats this as "displacement and `remove` record nothing", so the block is presented as AC7's `TierCounters` coverage.
**Why it is wrong**: microscope section 2, "A test that passes both ways is a defect, not a nitpick, because it will be counted as coverage forever". Plan AC7 names the wrong implementation as "Reporting a stale removal as an eviction, which inflates F-034's counters", and this block cannot detect that.
**Evidence**: mutation M13 makes `TierCounters::record` count `admission.displaced` as an eviction, and `stale_removal_is_explicit` stays green (exit 0). No mutation of `ImageCache::get` can turn the last assertion red, because the stale path never produces an `Admission`.
**Scope and regression**: present on HEAD. Test-only, no production consumer. GLM's decision 4 is right that the rule holds structurally: `ImageCache::insert` consumes the `Admission`, so no caller of this cache can feed `TierCounters`. The smallest fix is to delete the vacuous `Lru` block and keep the structural sentence, which `docs/lld/cache.md` already states. Do not add another `Lru`-level exercise, because it would test `TierCounters`, not F-224. If the lead wants AC7's counter clause observable, AC7 needs amending to the structural form.

### D2, `docs/lld/cache.md` still calls `ImageCache` an `Lru<FrameKey, DecodedFrame>`
**Where**: `docs/lld/cache.md` line 24, the type table row `| ImageCache | F-032. An Lru<FrameKey, DecodedFrame> fixed to the decoded tier, returning events |`.
**What**: After this diff `ImageCache` holds `Lru<FrameKey, HeldFrame>` (`crates/ocelli-cache/src/image.rs:497`). The diff added three rows directly below this one, and its new "Frame identity, F-224" section in the same file says "The `Lru`'s value is a private `HeldFrame`". The file now contradicts itself.
**Why it is wrong**: microscope section 3, a false claim in prose. The LLD is the as-built description.
**Evidence**: `grep -n 'Lru<FrameKey' docs/lld/cache.md crates/ocelli-cache/src/image.rs` returns line 24 (`DecodedFrame`) and image.rs:497 (`HeldFrame`).
**Scope and regression**: documentation only, present on HEAD. The fix is to say on that row that the value is the private `HeldFrame` (the content id beside the `DecodedFrame`). The `DecodedFrame` row, "the decoded tier's value type", is still defensible because that is what the tier stores and returns, so leave it.

## Smells

### S1, no `ImageCache`-level test that a hit refreshes recency
**Where**: `crates/ocelli-cache/src/image.rs` `ImageCache::get` hit path, and `crates/ocelli-cache/tests/image_cache.rs`.
**What**: The plan's state table says "`Hit`, the frame becomes most recently used", and the `get` doc comment says so too. The rewritten `get` gets that only because both of its reads go through `Lru::get`. No `image_cache.rs` test watches it. Mutation M15, making `Lru::get` not move the tick, leaves `image_cache` green. Only the `lru.rs` unit tests and my scratch test went red.
**Failure mechanism**: GLM's decision 1 explains the two-read shape by the missing `Lru::peek`. The obvious follow-up is to add a peek and use it for one of the two reads. Put on the second read, it silently stops a hit from refreshing recency, and the decoded tier then evicts the frame on screen first. Nothing at this entry point would go red.
**Smallest regression**: one case in `image_cache.rs`, budget 64, two 32-byte frames. Hit the first, insert a third, and assert `Evicted` names the second. That is the scratch test `review_hit_refreshes_recency_and_stale_frees_room`, which I ran and then deleted.

## Nitpicks

- N1. `content_id.rs`, the comment at the second AC2 assertion says "the state cannot return to a previous value because the prime is odd". Read literally, that is not what bijectivity gives, since a fold trajectory can revisit a value. The true statement, which the doc comment on `ContentId` gets right, is that two states that diverge at one position stay distinct under the same later bytes. Reword it to match.
- N2. `ImageCache::get` reads `held.frame.bytes()` for `Stale` and then discards the value `self.decoded.remove(key)` returns. `self.decoded.remove(key).map_or(0, |h| h.frame.bytes())` would report the bytes of the entry actually removed, with no separate read. It is equal today because `DecodedFrame` is immutable.

## Open decisions judged

1. **`get`'s two-read shape.** Accept. On the stale path the decision read's tick is consumed by an entry removed in the same call, which is unobservable, and on the hit path the frame becomes MRU as required. The cost is two `BTreeMap` lookups, with no allocation. S1 is the guard this shape needs.
2. **`u64::try_from(len).unwrap_or(u64::MAX)`.** Accept. It matches `check_buffer` and `CacheEvent::encode`, and it cannot fire on any target this crate builds.
3. **`FrameLookup` derives `Clone, Copy, Debug, PartialEq`.** Accept. `Eq` is impossible through `f64`, and `PartialEq` on `Hit` compares frames by value, which the tests use.
4. **AC7's `TierCounters` observation.** The structural conclusion is correct, but the test written for it is D1.

## Verified clean

- FNV-1a core, prefix composition, field order, no `as` cast (`u64::from(*byte)`, `try_from` for lengths), `wrapping_mul`, constants equal to the draft's.
- `HeldFrame::bytes` is `frame.bytes()`, so admitted sizes, `Evicted`, `Displaced`, `Refused` and `Released` byte counts are unchanged. The existing eviction, displacement, refusal and release tests and the event-accounting proptest pass under the new `insert`.
- `remove` still returns the `DecodedFrame` and emits nothing. `contains` is still not a use.
- `ScrollError::Stale` and `samples_for_upload` doc and behaviour.
- `DicomwebClient`: the validation order (unknown key refused before the cache check, both before `safeUrl` and any fetch), the `"no-cache"` default on all six request kinds, all four accepted values passed through unchanged, `cache` in both F-230 key lists, `export type DicomwebCacheMode`.
- Every changed doc sentence: README `DicomwebClient` paragraph, `docs/lld/dicomweb.md` "The fetch cache mode" and the narrowed "response caching of its own", `docs/lld/cache.md` "Frame identity, F-224" and its three new rows (D2 is an untouched neighbouring row), `docs/lld/viewport.md` Scroll paragraph. I checked each against the code, or against a mutation or probe above.
- No `unsafe`, no `wasm-bindgen`, no new dependency, no boundary or tier change. `no_std` wasm32 check passes.

## Limitations and handoff

- **Carried, not counted this round**: `examples/folder-viewer/src/use.html` line 393 says "Its constructor takes five options", and this diff makes it six. AC19's `use.html` and `more.html` rows and the README folder-viewer item belong to the viewer round. GLM flagged the line too. That round must close it.
- **CHANGELOG**: the plan requires a 0.x entry for the `ImageCache::insert`/`get`/`samples_for_upload` signature change and the new DICOMweb default. None was written, because GLM was told not to edit `CHANGELOG.md`. It is still owed before completion.
- **Not exercised**: no browser fetch, so the WHATWG cache-mode semantics are taken from the plan's citation and only the `RequestInit.cache` value was observed. No GPU, oracle or bench. AC13 to AC18 untouched.
- **Next**: GLM, or the lead, remediates D1, D2 and S1. Then a fresh pass 2 reviews the changed material. This is a bounded review of AC1 to AC12 only, and it does not mean the acceptance or the sprint is ready.
