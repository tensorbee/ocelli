# F-226 review, pass 2

**Reviewed**: `git diff a977b45 a113768` on branch `work/f-226-xai`, worktree `ocelli-wt-f-226`, HEAD `a11376821e53`, tree `9945b77c7ec73ed0c99157bd60353b1637eb37d6` at start and at end. Library half only: AC1 to AC9 and AC17. The only untracked path at start was `.claude/reviews/F-226-working-pass-1.md`. Where a finding sits in `1ce87a9` material that pass 1 also saw, it says so.
**Author / reviewer**: a113768 by GLM-5.3 in opencode, per its report at `scratchpad/glm-F-226-fix1b.log`. The reviewer is a separate Claude subagent that wrote neither a113768 nor pass 1.
**Result**: 4 defects, 1 smell, 3 nitpicks

## Acceptance challenged

- **S1 as ruled** (an empty or inverted range releases nothing, returns an empty `Vec`, never panics). I read std 1.97.1's `BTreeMap::range` docs (`map.rs:1410`) and its implementation (`search.rs:107` to `:123`). It panics on `Excluded(s), Excluded(e)` with `s == e`, and on any two finite bounds with `s > e`. Nothing else panics. `bounds_are_empty` returns true on `Inc/Inc lo > hi`, `Inc/Exc lo >= hi`, `Exc/Inc lo >= hi` and `Exc/Exc lo >= hi`, and false whenever a side is `Unbounded`. So every panicking input is caught, and the extra cases it catches (`Inc(k)..Exc(k)`, `Exc(k)..=Inc(k)`) really are empty. Adjacent excluded keys such as `(Exc(3), Exc(4))` are not judged empty and reach `range`, which returns nothing and does not panic. Under a coherent `Ord` no non-empty range is judged empty. I checked this independently with a throwaway exhaustive probe: keys 1 to 6, every start and end in {Included, Excluded, Unbounded} over values 0 to 7, which makes 576 cases covering equal, adjacent, inverted, out-of-range and unbounded. Each result was compared against the set definition, along with `used` and `len`. All 576 passed with no panic. The probe went red on each of B1, B2, B3, B7, B8 and B9 below, so it discriminates. The tree's own tests do not, which is D1.
- **Pass 1 D3, the five wrong ranges**, replayed one at a time in a scratch copy. R1 `(series, 3, 0)`, R2 `(series, u32::MAX, 0)` and R3 start `(series, 1, 0)` each make `release_series_releases_one_series` fail with exit 101. R4 brick end `level: 1` and R5 `brick: [u32::MAX, 0, 0]` each make `release_volume_releases_every_level_and_leaves_other_volumes` fail with exit 101. Closed.
- **Pass 1 closed items** replayed: R10, which ends the range at `(series + 1, 0, 0)` inclusive, goes red. R7, an empty release that emits, goes red. R6, a double release in `remove_range`, goes red in five tests, including both properties. Closed items stay closed.
- **N1**: `RELEASED_BYTES` set to 25 makes `decodes_fixture_payloads` fail, and so does 32. Set to 23 it makes `refuses_a_short_or_unknown_payload` fail. Closed.
- **The generator**: `(0u8..8, 0u8..8)` without sorting gives `lo > hi` in 28 of 64 draws. Removing the guard (B5) makes both properties fail, so inverted ranges do reach the cache, and that half of the comment is true. The other half is not checked, see D1.
- **The clippy claim** in the new unit test's comment: I wrote `remove_range(5_u8..=3)` as a literal in scratch. Clippy 1.97.1 then fails with `#[deny(clippy::reversed_empty_ranges)] on by default`. True.
- **AC3 and AC8 arithmetic**: 6 * 32 = 192, 5 * 32 = 160, 32 + 161 = 193. 8 + 16 + 4 + 32 + 64 = 124, 188 - 124 = 64. All correct. AC3's own numbers are no longer what the test asserts, see D4.
- **AC17**: no TypeScript runtime change in this round. The only TypeScript change is one assertion in `events.test.ts`.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo fmt --all --check` | tree 9945b77 | now | 0 |
| `cargo clippy -p ocelli-cache -p ocelli-viewport --all-targets -- -D warnings` | same | now | 0 |
| `cargo test -p ocelli-cache -p ocelli-viewport` | same | now | 0, every binary ok |
| `cargo test -p ocelli-viewport --test typegen` | same | now | 0, 2 passed |
| `npx vitest run packages/core` | same | now | 0, 48 files, 437 tests |
| `npm run typecheck` | same | now | 0 |
| `event_catalogue_check.py` | same | now | 0, 50 core and 53 tool |
| `guard_census.py`, `guard_census.py --check-runbook` | same | now | 0, 0 watched by nothing. Both 0 |
| `prose_check.py`, `source_provenance_check.py` | same | now | 0 (864 files), 0 (1706 files) |
| Mutations R1 to R7, R10, R13, B1 to B5, B7 to B10 | `git archive HEAD` in `target/f226-review2-mut`, `CARGO_TARGET_DIR=<wt>/target`, `cargo test -q -p ocelli-cache -p ocelli-viewport --no-fail-fast`, each applied alone and reverted | now, copy deleted after | see findings |
| `RELEASED_BYTES` 25, 32, 23 | same copy, `node_modules` symlinked, `npx vitest run packages/core/src/events.test.ts` | now | red on all three. Restored, 8 passed |
| Exhaustive `Bound` probe, 576 cases | throwaway `tests/zz_review_probe.rs` in the copy | now | pass on HEAD, red on B1 to B3 and B7 to B9 |
| Implementer's R1 to R5 table and gate tails | `glm-F-226-fix1b.log` | prior receipt | consistent with the replays above, not relied on |

## Defects

### D1, `bounds_are_empty` can judge a non-empty range empty and no test notices

**Where**: `crates/ocelli-cache/src/lru.rs:134` to `:142`. Tests `lru.rs` `an_inverted_or_equal_excluded_range_releases_nothing_and_changes_nothing`, `crates/ocelli-cache/tests/invariants.rs:37` to `:39`, `:50` to `:53`, `:92` to `:97`, `:168` to `:174`.
**What**: the new guard's other direction, a range that does hold keys but is judged empty, passes the whole suite with exit 0:
- B1, `Inc/Inc` changed to `lo >= hi`. Every single-key range `k..=k` then releases nothing.
- B2, `_ => true`. Every range with an `Unbounded` side (`..`, `k..`, `..=k`) then releases nothing.
- B7, B8 and B9, the `Exc/Inc`, `Inc/Exc` or `Exc/Exc` arm changed to `true`. Every half-open or excluded range then releases nothing.
The cause is in the tests. The unit test covers only the two empty shapes. AC1 and `release_series` use only `Included..=Included` with `lo < hi`. Both properties are driven by what `remove_range` reports and never check which keys it should have reported, so a release that silently returns nothing satisfies them. B10 shows this from the other side: property 1 calling `remove_range(lo.min(hi)..=lo.max(hi))`, which releases keys on an inverted draw, still passes. So the generator's sentence "must come back as an empty release" names a requirement the property does not assert.
**Why it is wrong**: microscope section 2, a test that passes both ways is a defect. The lead's ruling says an empty range releases nothing, and a non-empty range must release its keys. A wrong `true` here is a silent no-release: the caller sees an empty `Vec`, `used` keeps the bytes, and nothing refuses. That is the quiet failure CLAUDE.md warns about. The plan names `remove_range` as the one release primitive for F-122 and the viewer's forget (plan step 2, `:166`, `:167`), and those may compute any `RangeBounds`.
**Evidence**: each mutation applied alone in the scratch copy, exit 0, reverted. The exhaustive probe goes red on every one of them. Adding to property 1's `RemoveRange` arm `prop_assert_eq!(returned keys, live keys k with lo <= k && k <= hi)` makes B1 red as well (checked in scratch).
**Scope and regression**: present on HEAD. No caller reaches the bad shapes today, since `release_series` and `release_volume` pass ordered `..=` bounds with `lo < hi`. Smallest regression: (a) in both properties, assert that the returned keys equal the model's live keys inside `lo..=hi`, filtered without `BTreeMap::range` so an inverted draw expects none. That pins the generator's sentence and B1. (b) A unit test, or a small table, that holds one key and covers `k..=k`, `..`, `k..`, `..=k`, `(Excluded(k-1), Included(k))`, `(Included(k), Excluded(k+1))` and `(Excluded(k-1), Excluded(k+1))`, each releasing it. That pins B2 and B7 to B9. Then re-run B1, B2 and B7 to B10 and confirm each is red.

### D2, "`insert` and `set_budget` are the methods that allocate" is false since `remove_range`

**Where**: `crates/ocelli-cache/src/lru.rs:13` to `:16` (module doc). `docs/lld/cache.md:283` to `:285`, which ends "The other methods do not build a `Vec`."
**What**: `remove_range` builds two `Vec`s, the key collection and the return, and its own doc at `lru.rs:237` to `:239` says "This allocates the return and the collection". The module doc in the same file still names only `insert` and `set_budget`.
**Why it is wrong**: microscope section 3, a false claim in prose. AC9 is about where release allocates, and these are the two sentences a reader checks for that.
**Evidence**: `sed -n 13,16p lru.rs` against `sed -n 237,262p lru.rs`. `grep -n "set_budget\` are" docs/lld/cache.md`.
**Scope and regression**: added by 1ce87a9, which pass 1 also saw. `cache.md` was edited in this round. Smallest fix: name `remove_range` in both places as a release call that allocates, or delete the "the other methods" sentence. The plan's new "Release" section in `cache.md` would not by itself correct `:283`.

### D3, `cache.md`'s encode paragraph still describes only the keyed kinds

**Where**: `docs/lld/cache.md:164` to `:168`
**What**: "`CacheEvent::encode` writes the event into section 17.3's `RingEvent`: kind 1, 2 and 3 for evicted, displaced and refused, ... and a payload of series, instance, frame and bytes". `encode` now also writes kind 7 for `Released`, with series, frames, a zero `u32` and bytes (`image.rs:313` to `:331`). The sentence two lines above, edited in this round, says there are four variants.
**Why it is wrong**: an enumeration of what `encode` writes that a reader takes as complete, which is the same class as pass 1's D2 in `events.md`. "a payload of series, instance, frame and bytes" is false for kind 7.
**Evidence**: `sed -n 157,180p docs/lld/cache.md` read against `CacheEvent::encode`.
**Scope and regression**: LLD readers. Fix: add kind 7 and its payload to the sentence, or point to `events.md`'s ring kinds table.

### D4, the `release_series` fixture comment says "the state table's first three rows" with numbers that are not the table's

**Where**: `crates/ocelli-cache/tests/image_cache.rs:389` to `:393`. Plan AC3 at `.claude/plans/F-226-design.md:208` and the state table row at `:179`.
**What**: the comment opens with "The state table's first three rows, hand-computed" and then describes five frames, 192 bytes and a release of 160. The table's first row is three frames, `Released { frames 3, bytes 96 }`, `used` 32, and AC3 says "`used` 128 to 32". After the pass 1 D3 fix, the test asserts a superset of the table and no longer asserts the table's own values.
**Why it is wrong**: a false claim in a test comment, and AC3's stated observation is no longer the one the named test records. When AC3 is checked at `/complete-feature`, the test will show 192, 5 and 160 against a plan that says 128, 3 and 96.
**Evidence**: `sed -n 387,420p image_cache.rs` against plan lines 179 and 208.
**Scope and regression**: acceptance bookkeeping only. The arithmetic is right. Fix: reword the comment ("the state table's first three rows, with the range's two extreme keys added to series 5"), and have the lead amend AC3's numbers or record the superset.

## Smells

### S1, `remove_range` gives bytes back with no shifting-value test, and `cache.md` says each give-back site has one

**Where**: `crates/ocelli-cache/src/lru.rs:252` (`self.release(entry.bytes)`) and `docs/lld/cache.md:53` to `:57`
**What**: `cache.md` says three unit tests use a value whose size changes after admission, "one for each place bytes are given back: `remove`, an eviction, and the replacement of a live key". `remove_range` is a fourth such place. R13, which changes it to `self.release(entry.value.bytes())`, passes the suite with exit 0, because every value `remove_range` sees in a test is immutable.
**Why it is a hazard**: the code is correct today. But the `cache.md` paragraph says why those tests exist ("no number of random cases can tell a cache that kept the admitted number from one that re-reads it"), and that is the mechanism R13 gets through. A later edit to `remove_range` that re-reads would desynchronise `used` with nothing going red, and the LLD enumeration would mislead the reviewer of that edit.
**Evidence**: R13 in the scratch copy, exit 0, reverted.
**Scope and regression**: in 1ce87a9 material, not this round. Fix: a fourth `Shifting` unit test through `remove_range`, and "four" with `remove_range` named in `cache.md:53`.

## Nitpicks

- **N1.** `docs/lld/events.md:47` to `:49`: the new sentence about `cache.released` now sits between "have no 5.8.2 name" and "`viewport.resized` ... is unmapped for the same reason", so "the same reason" reads as the per-series reason. Moving the `cache.released` sentence after the `viewport.resized` one restores the reference.
- **N2.** `image.rs` `CacheEvents` doc: "the insert and release paths, which are decode-path calls". A release is not on the decode path, and `lru.rs` calls it "a release call and not a render-loop call". "which are not render-loop calls" is enough.
- **N3.** std notes (`search.rs:104`, issue 81138) that a `RangeBounds` can report different bounds on a second call. `remove_range` reads them once for the guard, and `BTreeMap::range` reads them again, so an adversarial `RangeBounds` impl could still reach the panic. No real caller does that, and the guard also assumes `K`'s `Ord` agrees with its `PartialEq`, which every derived key here does. A pre-existing related item, outside this diff: `RingEvent::kind`'s doc "One of this crate's `KIND_` constants" has not been true since F-101's `KIND_PRESENTED`, and the new `KIND_EVICTED` doc now states that.

## Verified clean

- `bounds_are_empty` against std's documented and implemented panic conditions, every `Bound` pair, equal, adjacent and inverted keys, through the 576-case probe. No panicking input remains, and on HEAD no non-empty range is judged empty.
- The generator really produces inverted ranges, and they reach the cache (B5 red).
- The new unit test goes red on B3 (`Exc/Exc` to `lo > hi`) and on B4 or B5 (guard removed). It asserts `used`, `len` and both keys unchanged.
- The pass 1 D3 fixtures: R1 to R5 are all red, and the comment arithmetic is correct.
- Pass 1 D1, the `image.rs` docs: every reworded sentence is true against `release_series` and `encode`, apart from N2. The `CacheEvent` doc, the module doc, the `RingEvent::payload` doc, the `CacheEvents`, `is_empty` and `len` docs, and `KIND_EVICTED` (kinds 1 to 5 and 7 in `image.rs`, 6 in `boundary.rs:216`).
- Pass 1 D2: `public-api.md:18` "seven ring kinds" against `RING_EVENT_KINDS` length 7. `events.md`'s unmapped list against the `events.ts:292` row and its reason. `viewport.md:91` against `ocelli-viewport`, which reads `KIND_RELEASED` only in the name table and tests. `cache.md:25` and `:161`.
- Pass 1 N1 and N2: closed.
- A stale-count search over README, `use.html`, `more.html` and `docs/lld` for variant, kind and ring-kind counts. `use.html:365` is true. The only remaining stale count is `CHANGELOG.md:159` "six ring kinds", which pass 1 already scheduled for step 5 and which is not counted here.
- AC17 is unchanged, and no `releaseSeries` or `releaseStudy` is under `packages/`.
- There is no `unsafe`, `as` cast or `wasm-bindgen` in the diff.

## Limitations and handoff

- Not exercised: the folder viewer half (AC10 to AC16, AC18), any browser, GPU or oracle job, and `frames` saturation.
- The exhaustive probe and every mutation ran in a deleted `git archive` copy. Nothing in the worktree was edited except this report. At the end the tree is `9945b77` and HEAD is `a113768`, unchanged, and the untracked paths are this file and pass 1's.
- Next owner: the implementer fixes D1 to D4 and S1. The lead decides AC3's numbers. A fresh pass 3 then re-runs B1, B2, B7 to B10 and R13, checks that each is red, and re-reads `cache.md:53`, `:164` and `:283` and `lru.rs:13`.
- This is a bounded review of the library half. It is not completed acceptance and not sprint readiness.
