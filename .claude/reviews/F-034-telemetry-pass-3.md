# F-034 review, pass 3

**Reviewed**: working tree on sprint/s12 at 4629015, after the pass 2 prose remediation
**Result**: 1 defect, 0 smells, 2 nitpicks

## Defects

### D1, The replacement sentences name a count the variant fold does not change

**Where**: `docs/lld/cache.md:158` and `docs/sprints/AS_BUILT.md:3332`
**What**: The cache LLD says a replacement counted as an eviction would inflate the eviction count, in the paragraph about not collapsing `CacheEvent` variants. The F-032 note says the displaced variant must never be folded into evicted, because a replacement counted as one would inflate that number. Neither sentence names a count. Folding `CacheEvent::Displaced` into `CacheEvent::Evicted` leaves `CacheEvents::len` at 1 and leaves `TierCounters.evicted` at 1. The only eviction count is `TierCounters.evicted`. It rises only when `record` counts `Admission.displaced`, and `record` does not read a `CacheEvent`.
**Why it is wrong**: A claim in prose that is false is a defect, and a definite count with no antecedent is the same failure. Pass 2 removed the false subject, that F-034 counts evictions from these events. The replacement keeps the consequence and attaches it to the variant fold. That fold is not what moves the count. The note's preceding sentence names review tallies and kind pins, not an eviction count, so "that number" has nothing to point at. D-24 and the F-034 section already say the telemetry folds `Admission` and ignores a displacement. These two sentences add a cause the code does not have.
**Evidence**: Changed only the displaced push in `ImageCache::insert` from `CacheEvent::Displaced` to `CacheEvent::Evicted`. `bin/ocelli.sh test ocelli-cache --test image_cache displaced_and_refused_are_not_evicted` exited 101 at `crates/ocelli-cache/tests/image_cache.rs:171`. The length assert at line 170 had passed, so the list was still one event. Left was `Evicted` and right was `Displaced`. `bin/ocelli.sh test ocelli-cache --test telemetry counters_count_an_eviction_and_a_refusal_and_ignore_a_displacement` exited 0. `image.rs` was restored to `5ff853db210519916041cafcb2302ee2cf771ab3`. Counting `admission.displaced` inside `TierCounters::record`, with the variant code left alone, made that same telemetry test exit 101 at `crates/ocelli-cache/tests/telemetry.rs:161` with left 2 and right 1. That inflation does not depend on `CacheEvent`. `telemetry.rs` was restored to `d011b56f259d4b01a59a002717f41189d5bfb99c`.

## Smells

None.

## Nitpicks

### N1, `evicted_bytes` re-reads `Budgeted::bytes`

**Where**: `crates/ocelli-cache/src/telemetry.rs:66`
**What**: `record` adds `value.bytes()` from the `V` in `admission.evicted`. The admitted size lived on the dropped `Entry` and is not on `Admission`. The field comment says a value whose `bytes` does not change reports the admitted size. The property test uses `Block`, which never changes, so the reported sum cannot disagree with the drop in `used`.
**Judgment**: Still not a defect and not a smell. The plan's `record` receives only `Admission`. No wrong result follows while `bytes` is stable.

### N2, Encoded's wire code is not pinned

**Where**: `crates/ocelli-cache/src/telemetry.rs:184` and `crates/ocelli-cache/tests/telemetry.rs:77` and `:318`
**What**: `tier_code` maps Encoded to 0, Decoded to 1 and Gpu to 2. The pressure fixture asserts payload byte 2. The budget fixture asserts payload byte 1. Neither encodes Encoded, so changing that arm alone leaves the suite green. The code is 0 today. This is a missing pin, not a wrong code.

## Verified clean

- The ten paths hashed to the pass 2 tree before any edit. After every mutation was put back by an edit, they hashed to the same values. `pub fn set_budget` is still at `crates/ocelli-cache/src/lru.rs:246`, and the while condition is still the original two-disjunct line.
- Deleted only `(self.budget == 0 && !self.entries.is_empty())` from `Lru::set_budget` and left `while self.used > self.budget`. `bin/ocelli.sh test ocelli-cache --test telemetry` exited 101. `a_recently_read_zero_byte_entry_still_leaves_at_budget_zero` failed at `crates/ocelli-cache/tests/telemetry.rs:226` with left 1 and right 2. The other 11 tests stayed green, including `set_budget_grows_shrinks_empties_and_ignores_the_same_value`. The original line was restored. `lru.rs` hashes to `623ced0599778163b4cee4ea9e6a12c08edaa151`.
- `TierCounters::record` at lines 63 to 71 folds `admission.evicted` and `admission.refused` and does not read `displaced` or `CacheEvent`. That matches `docs/lld/cache.md:194` to 196. An eviction counts, a refusal counts, and a displacement counts as neither. The adds are `saturating_add`. `evicted_bytes` sums `Budgeted::bytes` of the evicted values, which is the call at line 66.
- `Lru::set_budget` matches `docs/lld/cache.md:198` to 200. An equal budget returns an empty `Admission` before the loop. The loop evicts through `evict_least_recently_used`. A zero budget continues while an entry remains, which is the disjunct the test above kills. The returned value pushes only onto `evicted`.
- No sentence in the working diff, `docs/lld/cache.md`, or `docs/sprints/AS_BUILT.md` still says F-034 counts `CacheEvent`s or counts evictions from these events. `docs/sprints/AS_BUILT.md:3264` and D-24 say the telemetry counts evictions and a replacement would inflate that count. That is the `record` mutation above, not a claim that the counter reads events.
- `Pressure::band` compares `used * 2` with `budget` and `used * 10` with `budget * 9` in `u64`, and a zero budget is decided before those products. `crossing` returns one `Pressure` event or nothing. `encode` writes kinds 4 and 5, `viewport` as `u32::MAX`, and little-endian fields. The pressure arm does not write `evicted_bytes`. `budget`, `pressure`, and `len` do not call `tick`. `band` is a method on `Pressure` and does not see the cache. `crates/ocelli-wasm/src/` holds `lib.rs` and `panic.rs` only.
