# F-226 review, pass 1

**Reviewed**: `git diff 73b68ad a977b45` on branch `work/f-226-xai`, worktree `ocelli-wt-f-226`, tree `dd5f44513ac1e9ba07fccd46d9c44c5280438beb` at start and at end. Library half only: Approach steps 1 to 3, AC1 to AC9 and AC17. No unstaged paths at start.
**Author / reviewer**: 1ce87a9 by GLM-5.3 in opencode. a977b45, two doc lines, by the sprint lead (Claude). Reviewer is a separate Claude subagent that wrote neither commit. It reviews the lead's two lines as another participant's material.
**Result**: 3 defects, 1 smell, 2 nitpicks

## Acceptance challenged

- **AC1**, re-derived from the plan. Keys 1 to 9 at 2^(k-1) bytes sum to 511. `3..=6` holds 4 + 8 + 16 + 32 = 60, so 451. After `get(1)` and `get(3)` the LRU order is 2, 4, 5, 6, 7, 8, 9, 1, 3, and in-range recency is 4, 5, 6, 3, so an evicting release returns a different order than ascending. After the release the survivors are 2, 7, 8, 9, 1. A 600-byte insert with 549 free evicts 2 (free 551) then 7 (free 615), used 985. The test asserts all of these. A reversed return (R9) goes red.
- **AC2**. Double release in `remove_range` (R6) goes red in debug through `Lru::release`'s `debug_assert` and in `--release` through the model assertion `pressure.used == sum(live)`, minimal input `budget = 0`. `set_budget` ignoring a non-zero budget (R12) goes red on `pressure.budget == current_budget`. So the property drives both new operations.
- **AC3**. The bleed mutation, end at `(series + 1, 0, 0)` inclusive (R10), goes red at `image_cache.rs:409`. Empty release emitting an event (R7) goes red. Release of an absent series and a repeat emit nothing and leave `used` 32. With one survivor the "recency unchanged" clause is only observable at the `Lru` level, which AC1 covers.
- **AC4**. `524_288 = 0x0008_0000`, little-endian `00 00 08 00 00 00 00 00` at 16. Series `08 07 .. 01` at 0, frames `D0 C0 B0 A0` at 8, zero `u32` at 12, which keeps `bytes` at 16 like the keyed arms (`payload[16..24]` in both). `viewport` `u32::MAX`, eight zero bytes at 24. Matches the test byte for byte.
- **AC5**. `0xA0B0C0D0 = 2695938256`. Bytes read at 12 (T2) red. `kind >= released` (T3) red on kind 8. `RELEASED_BYTES = 23` (T4) red on the 23-byte refusal. Kind 0 null is asserted.
- **AC6**. `cargo test -p ocelli-viewport --test typegen` compares the committed `generated.ts` byte for byte and passes, so the one added line `released: 7,` is the generator's output and not a hand edit that differs from it.
- **AC7**. Deleting the `UNMAPPED_EVENTS` row turns `names_are_unique` and `ring_kinds_match_generated` red (the catalogue script stays OK, it reads cornerstone rows). Deleting `cache.released` from `OCELLI_EVENTS` passes vitest but fails `npm run typecheck` with TS2820 on the unmapped row at `:292`, so the floor catches it. `CORNERSTONE_EVENT_MAP` is untouched. The reason text is true against the map: `IMAGE_CACHE_IMAGE_REMOVED` is the per-image pairing at `events.ts:176`, and `VOLUME_CACHE_VOLUME_REMOVED` is a separate unavailable row.
- **AC8**. 8 + 16 + 32 = 56, 120 - 56 = 64. A range over level 0 only (R4b) goes red.
- **AC9**. `get` and `insert` are untouched. `remove_range` allocates the key vector and the return, on the release call only. The lru unit test asserts `len` 5 after a later `get`.
- **AC17**. `grep releaseSeries\|releaseStudy packages/` prints nothing. `index.ts` gains only `export type CacheReleasedPayload`. `index.test.ts` is unchanged and passes.
- Unchanged consumers: no exhaustive `match` on `CacheEvent` outside `ocelli-cache`. `RING_EVENT_KINDS` has one other reader, `typegen.rs`. `ocelli-wasm` reads only `KIND_PRESENTED`.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo fmt --all --check` | tree dd5f445 | now | 0 |
| `cargo clippy -p ocelli-cache -p ocelli-viewport --all-targets -- -D warnings` | same | now | 0 |
| `cargo test -p ocelli-cache -p ocelli-viewport` | same | now | 0, 175 passed in 17 binaries |
| `cargo test -p ocelli-viewport --test typegen` | same | now | 0, 2 passed |
| `cargo check -p ocelli-cache --target wasm32-unknown-unknown` | same | now | 0 |
| `npx vitest run packages/core` | same | now | 0, 48 files, 437 tests |
| `npm run typecheck`, `npm run lint` | same | now | 0, 0 |
| `event_catalogue_check.py` | same | now | 0, 50 core and 53 tool |
| `guard_census.py`, `--check-runbook` | same | now | 0, 0 watched by nothing |
| `prose_check.py`, `source_provenance_check.py` | same | now | 0, 0 |
| Mutations R1 to R12, T1 to T4, catalogue row deletions | `git archive HEAD` copy in `target/f226-review-mut`, shared `CARGO_TARGET_DIR` | now, copy deleted after | see findings and Acceptance challenged |
| Inverted range probe `remove_range(5..=3)` | same copy, throwaway test appended | now | panic `range start is greater than range end in BTreeMap` |
| Implementer's four mutations and check list | `glm-F-226-lib.log` | prior receipt | not relied on, the overlapping ones were re-run above |

## Defects

### D1, the `CacheEvent` and `CacheEvents` doc comments still say insert only

**Where**: `crates/ocelli-cache/src/image.rs:1`, `:6`, `:226` to `:230`, `:300`, `:352`, `:368`, `:373`
**What**: The diff makes `CacheEvent` four variants and makes `release_series` return `CacheEvents`, but the docs around them were not touched. The module doc says "the events an insert emits" and "one `CacheEvent` per outcome" of an `Admission`. `CacheEvent` says "One thing an insert did to the decoded tier" and "Three variants because `Admission` has three fields". `CacheEvents` says "The events one `ImageCache::insert` produced", `is_empty` says "Whether the insert displaced, evicted and refused nothing", `len` says "How many events the insert produced". `RingEvent::payload` says "For a cache event, the key and byte count", which `Released` (series, frames, bytes, no key) contradicts.
**Why it is wrong**: microscope section 3, a false claim in prose is a defect. Each sentence is checkable against `release_series` in the same file, which returns a `CacheEvents` holding a `Released` that no insert produced.
**Evidence**: `sed -n 224,236p` and `sed -n 350,375p` of `image.rs` on dd5f445, read against `release_series` at `:443`.
**Scope and regression**: rustdoc readers and F-231, which will build on `release_series` and read these docs. Smallest fix: reword the six sites so they cover an insert or a release, and say `Released` is the fourth variant and comes from no `Admission`. `docs/lld/cache.md:25` and `:161` carry the same claims but are scheduled for `/complete-feature`, see handoff.

### D2, stale ring-kind prose in `docs/lld`

**Where**: `docs/lld/public-api.md:18`, `docs/lld/events.md:45` to `:48`
**What**: `public-api.md` says `boundary.rs` holds "the six ring kinds". It holds seven since this diff (`RING_EVENT_KINDS: [(&str, u32); 7]`). `events.md`, which a977b45 edited two lines above, still enumerates the unmapped ring events as "`cache.displaced`, `cache.refused` and `cache.budgetChanged` have no 5.8.2 name. They are `UNMAPPED_EVENTS`", and `cache.released` is now a fourth unmapped ring row.
**Why it is wrong**: the first is false. The second is an enumeration a reader takes as complete, in the file that the LLD impact section says must record `cache.released`'s `UNMAPPED_EVENTS` reason. `public-api.md` is not in the plan's LLD impact list at all, so `/complete-feature` would not pick it up.
**Evidence**: `grep -rniE "six (ring )?kinds" docs/lld` prints `public-api.md:18`. `grep -n "cache.released" packages/core/src/events.ts` shows the `UNMAPPED_EVENTS` row at `:292`.
**Scope and regression**: LLD readers. Fix: "the seven ring kinds" in `public-api.md:18`, and add `cache.released` with its reason to the `events.md` sentence. README, `use.html` and `more.html` hold no other stale count. The `use.html:365` sentence is now true against `RING_EVENT_KIND` and `decodeNamedEvent`.

### D3, the series and volume range ends are not pinned by any test

**Where**: `crates/ocelli-cache/src/image.rs:444` to `:446`, `crates/ocelli-cache/src/brick.rs:448` to `:458`, tests `release_series_releases_one_series` and `release_volume_releases_every_level_and_leaves_other_volumes`
**What**: Five wrong ranges pass the whole `ocelli-cache` and `ocelli-viewport` suite.
- R1, end `FrameKey::new(series, 3, 0)`: exit 0.
- R2, end `FrameKey::new(series, u32::MAX, 0)`: exit 0. A multi-frame instance's frames 1 and up would survive a release.
- R3, start `FrameKey::new(series, 1, 0)`: exit 0. Instance 0 would survive.
- R4, brick end `level: 1`: exit 0.
- R5, brick end `brick: [u32::MAX, 0, 0]`: exit 0. Bricks with y or z above 0 on the last level would survive.
**Why it is wrong**: the plan fixes the range at `(series, 0, 0)..=(series, u32::MAX, u32::MAX)` and the brick range across all levels and all coordinates, and microscope section 2 says a test that passes both ways is a defect. The fixtures hold only instances 1 to 3 at frame 0, and bricks at levels 0 and 1 with y and z 0, so neither extreme is ever occupied. The surviving frames would still count in `used` while the caller believes the series released.
**Evidence**: each mutation applied singly in the scratch copy, `cargo test -q -p ocelli-cache -p ocelli-viewport`, exit 0, reverted. The bleed mutation R10 and the level 0 mutation R4b do go red, so the tests catch the neighbour but not the extremes.
**Scope and regression**: F-231's session cache, which keys multi-frame instances, and F-122's brick policy. Smallest regression: hold `(5, 0, 0)` and `(5, u32::MAX, u32::MAX)` in the series fixture alongside the existing frames, and a brick at `level: u32::MAX, brick: [u32::MAX; 3]` plus one at `[0, 1, 1]` in the volume fixture, then re-run R1 to R5 red.

## Smells

### S1, `Lru::remove_range` panics on an inverted or empty excluded range

**Where**: `crates/ocelli-cache/src/lru.rs:222` to `:236`, and the generator comment at `crates/ocelli-cache/tests/invariants.rs:48` to `:51`
**What**: `remove_range(5..=3)` panics with "range start is greater than range end in BTreeMap", and `(Excluded(k), Excluded(k))` panics the same way. The doc comment has no `# Panics` and the function does not check its bounds. The property generator sorts `lo` and `hi` and says it does so "so the range is never empty", when the real reason is that an inverted range would panic the test.
**Why it is a hazard**: HLD section 23 says a panic in WebAssembly poisons the instance and no exported path may reach one, which is why the workspace denies `panic`, `unwrap_used` and `expect_used`. A `BTreeMap` panic gets past those lints. `release_series` and `release_volume` always pass ordered bounds, so nothing reaches it today. But the plan names `remove_range` as the one release primitive for F-122's per-frame residency policy and the folder viewer's forget, and a range a caller computes can come out inverted.
**Evidence**: a throwaway test in the scratch copy, panic observed, copy deleted.
**Scope and regression**: future callers of the public primitive. Fix: return an empty `Vec` when the bounds are inverted or empty excluded, or document `# Panics` and give the generator comment its real reason. A unit test on `5..=3` pins whichever is chosen.

## Nitpicks

- **N1.** `RELEASED_BYTES` set to 25 or 32 (T1b, T1) passes vitest, because no test decodes a payload of exactly 24 bytes. The other readers in `events.ts` follow the same short-only pattern and the ring always copies 32 bytes, so nothing observable changes.
- **N2.** `KIND_EVICTED`'s doc says "One range for this crate's kinds". The cache's kinds are now 1 to 5 and 7, around viewport's `KIND_PRESENTED` 6, and uniqueness is held only by `boundary.rs`'s test. The sentence was written for a contiguous range.

## Verified clean

- The `Released` encode arm against the keyed arms: same `u64::try_from(..).unwrap_or(u64::MAX)` conversion, `bytes` at 16 in both, `viewport` `u32::MAX`, reserved tail zero. `frames` saturates through `u32::try_from`.
- `release_series`'s `bytes` is the sum of `DecodedFrame::bytes()`, which is `samples.len()`. Nothing exposes `get_mut`, so that equals the admitted bytes `used` drops by.
- `remove_range` leaves every tick alone and calls the existing private `release`, so it is `remove`'s rule over a range.
- `RING_EVENT_KINDS` length 7, distinct names and kinds, and `KIND_RELEASED` 7 is pinned in three tests.
- `TestCaseError` in `image_cache.rs` resolves through `proptest::prelude`. The new `Released` arm there is a real refusal, since no insert produces one.
- README: no sentence covers the event list or cache release, so leaving it unchanged is right for this half. `use.html:365` is true. `more.html` holds no event or cache claim.
- No `unsafe`, no `wasm-bindgen`, no `as` cast in the diff. No render-loop path touched.

GLM's decisions the plan left open, judged:

1. Folding `parts()` into `encode` with an early return for `Released`: correct. Bytes for existing kinds are unchanged and their tests pass.
2. `SetBudget` and `RemoveRange` in both properties with a `current_budget` model: correct and needed, since AC2 names `set_budget`. R12 shows it is exercised.
3. A failing `Released` arm in the insert accounting property: correct.
4. Series 6 at `(6, 0, 0)` so the named bleed is caught: correct, and R10 confirms. The same reasoning was not applied to the range's own extremes, which is D3.
5. `ReleaseSummary` in `brick.rs` with a note that it has no ring kind: correct, matches the plan.
6. `events.ts` placement: harmless. Nothing reads `OCELLI_EVENTS` by position.

## Limitations and handoff

- Not exercised: the folder viewer half (`held.rs`, `serve.rs`, `page.html`, AC10 to AC16), any browser, GPU or oracle job, and `frames` saturation at `u32::MAX`, which would take four billion inserts.
- Scheduled for later rounds and not counted here: `docs/lld/cache.md` (`:25` and `:161` say three variants), the `CHANGELOG.md` `## Unreleased` entry, and AC18's README, `use.html` section 12 and `more.html` sentences. `CHANGELOG.md:159` under `## Unreleased` says "A named event catalogue for the six ring kinds", and those notes become a release's notes, so the step 5 entry should reconcile that line.
- Next owner: the implementer fixes D1 to D3 and S1, then a fresh independent pass 2 reviews the changed material and replays R1 to R5 and the inverted range probe.
- This is a bounded review of the library half. It is not completed acceptance and not sprint readiness.
