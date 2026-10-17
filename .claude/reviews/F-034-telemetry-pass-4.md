# F-034 review, pass 4

**Reviewed**: working tree on sprint/s12 at 4629015, after deleting the pass 3 sentences
**Result**: 0 defects, 0 smells, 2 nitpicks

## Defects

None.

## Smells

None.

## Nitpicks

### N1, `evicted_bytes` re-reads `Budgeted::bytes`

**Where**: `crates/ocelli-cache/src/telemetry.rs:66`
**What**: `record` adds `value.bytes()` from the `V` in `admission.evicted`. `Lru` stored the admitted size on the dropped `Entry`, and `Admission` does not carry that size. The field comment says a value whose `bytes` does not change reports the admitted size. The property test uses `Block`, which never changes, so the reported sum cannot disagree with the drop in `used`.
**Judgment**: Not a defect and not a smell. `record` receives only `Admission`. No wrong result follows while `bytes` is stable.

### N2, Encoded's wire code is not pinned

**Where**: `crates/ocelli-cache/src/telemetry.rs:184` and `crates/ocelli-cache/tests/telemetry.rs:77` and `:318`
**What**: `tier_code` maps Encoded to 0, Decoded to 1 and Gpu to 2. The pressure fixture asserts payload byte 2. The budget fixture asserts payload byte 1. Neither encodes Encoded, so changing that arm alone leaves the suite green. The code is 0 today. This is a missing pin, not a wrong code.

## Verified clean

- The ten paths hashed to the required values before any edit. After the drain disjunct was restored by an edit, they hashed to the same values. `pub fn set_budget` is at `crates/ocelli-cache/src/lru.rs:246`, and the while condition is the original two-disjunct line.
- Deleted the zero-budget disjunct from `Lru::set_budget` and left `while self.used > self.budget`. `bin/ocelli.sh test ocelli-cache --test telemetry` exited 101. `a_recently_read_zero_byte_entry_still_leaves_at_budget_zero` failed at `crates/ocelli-cache/tests/telemetry.rs:226` with left 1 and right 2. The other 11 tests stayed green, including `set_budget_grows_shrinks_empties_and_ignores_the_same_value`. That test stays green because its zero-byte entry is least recently used, so `used > budget` evicts it before `used` reaches zero. The dedicated test reads that entry first, which is why only the disjunct keeps it from surviving. The original line was restored. `lru.rs` hashes to `623ced0599778163b4cee4ea9e6a12c08edaa151`.
- Added lines in the working diff do not contain `inflate the eviction count`, `inflate that number`, or `from these events`. Those phrases are also absent from `docs/lld/cache.md`, `docs/sprints/AS_BUILT.md`, `docs/lld/README.md`, `docs/sprints/BACKLOG.md`, and the `ocelli-cache` sources. `git diff` still shows `inflate that number` and `from these events` on removed lines only. No remaining sentence says that folding a `CacheEvent` changes `TierCounters`.
- `docs/lld/cache.md` lines 155 to 158 say `ImageCache::insert` emits a displacement first, then evictions in least recently used order, and that the three variants are never collapsed. `ImageCache::insert` at `crates/ocelli-cache/src/image.rs:239` pushes `CacheEvent::Displaced`, then one `CacheEvent::Evicted` per `admission.evicted` entry, then `CacheEvent::Refused`. `Lru::insert` at `crates/ocelli-cache/src/lru.rs:155` releases the previous value before the eviction loop, and that loop pushes victims from `evict_least_recently_used`. The three variants stay distinct. The remaining sentence is true.
- `TierCounters::record` at lines 63 to 71 folds `admission.evicted` and `admission.refused` and does not read `displaced` or `CacheEvent`. That matches `docs/lld/cache.md` lines 193 to 195. An eviction counts, a refusal counts, and a displacement counts as neither. The adds are `saturating_add`. `evicted_bytes` sums `Budgeted::bytes` of the evicted values, which is the call at line 66.
- `Lru::set_budget` matches `docs/lld/cache.md` lines 197 to 200. An equal budget returns an empty `Admission` before the loop. The loop evicts through `evict_least_recently_used`. A zero budget continues while an entry remains, which is the disjunct the test above kills. The returned value pushes only onto `evicted`.
- `docs/sprints/AS_BUILT.md:3264` says F-034's telemetry counts evictions, which a replacement would inflate. That sentence is about `Admission`, not about `CacheEvent`. `record` ignores `displaced`. In `counters_count_an_eviction_and_a_refusal_and_ignore_a_displacement` the last insert replaces key 2, and the assert stays at `evicted == 1`. Counting that replacement would make the count 2. The sentence was not re-opened.
- `Pressure::band` compares `used * 2` with `budget` and `used * 10` with `budget * 9` in `u64`, and a zero budget is decided before those products. `crossing` returns one `Pressure` event or nothing. `encode` writes kinds 4 and 5, `viewport` as `u32::MAX`, and little-endian fields. The pressure arm does not write `evicted_bytes`. `budget`, `pressure`, and `len` do not call `tick`. `band` is a method on `Pressure` and does not see the cache. `crates/ocelli-wasm/src/` holds `lib.rs` and `panic.rs` only. `ocelli-cache` has no `unsafe` and no `wasm_bindgen`.
