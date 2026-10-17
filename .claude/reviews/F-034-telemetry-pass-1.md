# F-034 review, pass 1

**Reviewed**: working tree on `sprint/s12`. Unstaged diff is 139 insertions and 14 deletions. Untracked files are `crates/ocelli-cache/src/telemetry.rs` and `crates/ocelli-cache/tests/telemetry.rs`. After the mutations below were reverted, `lru.rs` hashed to `623ced0599778163b4cee4ea9e6a12c08edaa151` and `tier.rs` hashed to `a9d7a1c39d1ad7122979c69e3462da0caa7c8feb`.
**Result**: 1 defect, 0 smells, 4 nitpicks

## Defects

### D1, Zero-budget drain of a zero-byte entry is not guarded

**Where**: `crates/ocelli-cache/src/lru.rs:252` and `crates/ocelli-cache/tests/telemetry.rs:242`
**What**: `set_budget(0)` keeps a zero-byte entry that is not least recently used unless the second half of the while condition runs. The committed fixture inserts the zero-byte block first, so that entry is least recently used and leaves under `used > budget` while `used` is still 5. Deleting `(self.budget == 0 && !self.entries.is_empty())` leaves every committed telemetry test green.
**Why it is wrong**: The plan says a budget of zero evicts everything. `docs/lld/cache.md` says a budget of zero empties the tier, zero-byte entries included. A reading that makes the zero-byte entry most recently used then leaves it live at `used == 0`, which still satisfies `used <= budget` and the property test's byte sum. Microscope items 2 and 4. A test that stays green without the branch will be counted as coverage of a drain it does not perform.
**Evidence**: With that clause removed, `bin/ocelli.sh test ocelli-cache --test telemetry` kept all 11 committed tests passing, including `set_budget_grows_shrinks_empties_and_ignores_the_same_value` and `eviction_bytes_match_what_the_tier_stopped_holding`. A temporary scenario inserted `Block(0)`, then `Block(5)`, called `get` on the zero-byte key, and called `set_budget(0)`. It failed with `evicted.len()` of 1 against 2. Restoring the clause made that same scenario pass. The temporary test was removed and the clause in the tree is the original one.

## Smells

None.

## Nitpicks

### N1, The wide-budget comment names the wrong wrapping band

**Where**: `crates/ocelli-cache/tests/telemetry.rs:110`
**What**: The comment says a wrapping 32-bit multiply reports Low. Wrapping both products in `u32` makes `3_000_000_000 * 10` compare above `3_500_000_000 * 9`, so the same control flow returns Critical. The assertion still expects Elevated, so either wrong band fails the test. Only the comment is false.

### N2, The downward-crossing comment does not describe its loop

**Where**: `crates/ocelli-cache/tests/telemetry.rs:194`
**What**: The comment says removing through 150 leaves 149. The loop removes keys `1..=121`. `270 - 121` is 149. `270 - 150` is 120. The assertions check two crossings and a final Low band, and both stop points satisfy that.

### N3, `evicted_bytes` re-reads `Budgeted::bytes`

**Where**: `crates/ocelli-cache/src/telemetry.rs:66`
**What**: `Lru` releases the size stored at admission and then drops that field. `TierCounters::record` calls `value.bytes()` on the returned value. The doc says this equals the admitted size only when `bytes` does not change. `Block` never changes, so the property test cannot disagree with the drop in `used`.

### N4, The band fractions are literals

**Where**: `crates/ocelli-cache/src/tier.rs:81`
**What**: Decision E1 asked for named constants beside `Pressure::band`. The comparisons inline `2`, `10` and `9`. The boundary fixture still pins the integer fractions.

## Verified clean

- Arithmetic. No `as` cast in the new cache sources or in `tests/telemetry.rs`. Width changes go through `u64::try_from` and `u32::try_from`, with `unwrap_or` to the maximum. A `usize` fits in `u64` on every target this crate builds, so those fallbacks do not change a real budget. `Pressure::band` special-cases a zero budget, then compares `used * 10` with `budget * 9` before `used * 2` with `budget`, both with `>=`, both in `u64` via `saturating_mul`. Hand check for budget 300: `149 * 2 = 298` is Low, `150 * 2 = 300` is Elevated, `269 * 10 = 2690` is below `2700` so Elevated, and `270 * 10 = 2700` is Critical. `3_000_000_000` against `3_500_000_000` is at or above half and below nine tenths, so Elevated. Empty at budget 0 is Low. One byte at budget 0 is Critical.
- Mutation, nine tenths. `budget.saturating_mul(9)` was changed to `8`. `bin/ocelli.sh test ocelli-cache --test telemetry` exited 101. `band_boundaries_are_the_integer_fractions` failed at line 99 with left Critical and right Elevated. `a_zero_budget_is_low_when_empty_and_critical_when_it_holds_bytes` failed at line 114 with the same pair. `an_unchanged_band_emits_nothing_and_each_crossing_emits_one` stayed green, so the crossing count does not pin the fraction. The boundary fixture does. The multiplier was restored to 9. `tier.rs` returned to the hash above.
- Mutation, saturation. `evicted.saturating_add(1)` was changed to `wrapping_add`. `counters_saturate_instead_of_wrapping` failed at line 283 with left 0 and right 4294967295. The call was restored to `saturating_add`. `refused` and `evicted_bytes` use the same saturating form.
- Displacement. `counters_count_an_eviction_and_a_refusal_and_ignore_a_displacement` admits 6, evicts that 6 to admit another 6, refuses 11, then replaces the live key with 4. It expects `evicted == 1`, `refused == 1` and `evicted_bytes == 6`. Counting the replacement as an eviction fails the first equality. `record` never reads `displaced`.
- Eviction order and budget motion. `set_budget` assigns the new budget, then calls the same `evict_least_recently_used` scan `insert` uses, least tick first. The shrink fixture expects keys 1 then 2. A most-recently-used scan on that fixture would return 3 then 2. Growing to 80 and repeating 80 return an empty `Admission`. After the shrink to 10, `used` is 10 and `len` is 1. The sum check after a shrink to 50 leaves `used == 30`, which is the one surviving entry.
- Payload and kinds. `KIND_PRESSURE` is 4 and `KIND_BUDGET_CHANGED` is 5, beside F-032's 1, 2 and 3. There is no `ci/event-kinds.json`. `encode` writes little-endian into a zeroed 32-byte payload and sets `viewport` to `u32::MAX`. Pressure bytes match the plan: tier at 0, `used` at 4, `budget` at 12, `entries` at 20, `evicted` at 24, `refused` at 28. The fixture's `used` bytes are `00 00 04 00 00 00 00 00` for 262144 and the budget bytes are `00 00 08 00 00 00 00 00` for 524288. `evicted_bytes` is not written on that path. `BudgetChanged` writes tier, budget at 4, evicted bytes at 12, evicted count at 20 and `dropped` at 24, with bytes 28 to 31 left zero. `RingEvent` is asserted at 48 bytes with `kind` at 0, `viewport` at 4, `seq` at 8 and payload at 16. Tier codes are Encoded 0, Decoded 1, Gpu 2. The tests pin Decoded and Gpu.
- Allocation claims in `docs/lld/cache.md`. A counting allocator around `set_budget` measured grow 0, repeated budget 0, one `u8` eviction 1, one `String` eviction 2, and one evicting `u8` insert 1. That is the `Vec` push, plus a second allocation when the victim key owns memory. Two `u8` evictions measured 1, because the first `Vec` reservation held the second push. The sentence does not claim one allocator call per eviction. `budget`, `pressure` and `len` only read fields. `band` reads a copied `Pressure`. The reading test calls those ten times and the next eviction is still key 1.
- Other prose that was executed. Kinds 4 and 5, sentinel viewport, little-endian payload, and the absence of `evicted_bytes` on the pressure wire match the encoder. No `crates/ocelli-wasm/src/ring.rs` file exists. Search found no `wasm-bindgen` and no `unsafe` under `crates/ocelli-cache`. The payload is counts and a tier code, not samples. `docs/lld/cache.md` still answers tiers A, B and C as not applicable. No new trait and no `Box<dyn>`. `TierCounters::record` is the generic the plan wrote, because `Admission` is generic. `bin/ocelli.sh clippy ocelli-cache` exited 0. `bin/ocelli.sh test ocelli-cache` exited 0 before the temporary edits, 21 unit tests plus budget, image, invariant and telemetry suites. After the last revert, `bin/ocelli.sh test ocelli-cache --test telemetry` exited 0 with 11 passed.
