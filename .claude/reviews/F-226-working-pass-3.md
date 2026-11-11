# F-226 review, pass 3

**Reviewed**: `git diff 284f878 HEAD` on branch `work/f-226-xai`, worktree `ocelli-wt-f-226`, HEAD `fc20d84cc4f8`, tree `c673549f5ba8d30b20ee902238deceaf73414b23` at start and at end. Library half only: AC1 to AC9 and AC17. The untracked paths at start were the pass 1 and pass 2 reports. Six files changed: `lru.rs`, `image.rs`, `tests/image_cache.rs`, `tests/invariants.rs`, `docs/lld/cache.md`, `docs/lld/events.md`.
**Author / reviewer**: the remediation is GLM-5.3 in opencode, per `scratchpad/glm-F-226-fix2.log`. That log says it staged six files and committed nothing, so the commit `fc20d84` was made from its staged work by another participant, and the trailer alone cannot say who. The reviewer is a separate Claude subagent that wrote neither the remediation nor passes 1 and 2.
**Result**: 1 defect, 0 smells, 1 nitpick

## Acceptance challenged

- **Pass 2 D1, the non-empty direction of `bounds_are_empty`**. B1 (`Inc/Inc` to `lo >= hi`), B2 (`_ => true`), B7, B8 and B9 (`Exc/Inc`, `Inc/Exc`, `Exc/Exc` arm to `true`) were each applied alone in a scratch copy and each is red now, exit 101. B1 fails the bound-shape table, the new drift test and both properties. B2, B7, B8 and B9 each fail the bound-shape table. B10, a sorted `lo.min(hi)..=lo.max(hi)` in place of the drawn range, is red in property 1 and, applied separately, in property 2. Closed.
- **Independence of the property expectation.** Both arms compute `expected` as `live.keys().copied().filter(|key| *key >= lo && *key <= hi)`, where `live` is the test's own `BTreeMap<u8, usize>` model, and never call `range` or `remove_range` for it. The filter is empty on an inverted draw by construction. The bound-shape table's expected keys are literals written from the bounds over keys 1 to 6, and I recomputed all twelve rows by hand: they are right.
- **Pass 2 S1, the size re-read in `remove_range`** (R13, `self.release(entry.value.bytes())`). Red in the new `Shifting` test. In the debug test build it panics at `release`'s `debug_assert`, and under `--release` the `used == 100` assertion fails with left 0, so the comment "Re-reading would saturate `used` to zero" is true of the shipped profile and the test is red either way. Closed.
- **The empty direction, which no pass has replayed per arm.** Removing the `Inc/Exc` arm, removing the `Exc/Inc` arm, or narrowing `Exc/Exc` to `lo == hi` each leaves the whole suite at exit 0, and each makes an inverted range of that shape panic in `BTreeMap::range`. See D1.
- **Pass 1 and pass 2 closed items**: R1, R2, R3, R5, R6, R7 and R10 each red now (R6 in seven tests including both properties). `RELEASED_BYTES` at 25 and at 23 each red in `events.test.ts`, 8 of 8 after restoring. Still closed.
- **AC3 under the lead's amendment after pass 2**: the fixture is the table's three frames `(5,1,0)` to `(5,3,0)` plus `(5,0,0)` and `(5,u32::MAX,u32::MAX)`, which are exactly the two ends of `release_series`'s range at `image.rs:450`. 6 * 32 = 192, 5 * 32 = 160, 192 - 160 = 32. The comment now says these are the fixture's numbers and not the table's, which is what the amendment records.
- **The encode payload layout**: `image.rs:323` to `:328` writes series at 0..8, frames at 8..12, leaves 12..16 zero and writes bytes at 16..24. `released_tests` asserts 12..16 is four zeros and 24..32 is zero. `packages/core/src/events.ts:418` to `:420` reads series with `getBigUint64(0)`, frames with `getUint32(8)`, bytes with `getBigUint64(16)`, and its doc at `:409` says the same. `cache.md:164` to `:173` and the `encode` doc agree with all three.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo fmt --all --check` | tree c673549 | now | 0 |
| `cargo clippy -p ocelli-cache -p ocelli-viewport --all-targets -- -D warnings` | same | now | 0 |
| `cargo test -p ocelli-cache -p ocelli-viewport` | same | now | 0, 19 binaries, 178 passed, 0 failed |
| `cargo test -p ocelli-viewport --test typegen` | same | now | 0, 2 passed |
| `npx vitest run packages/core` | same | now | 0, 48 files, 437 tests |
| `npm run typecheck`, `npm run lint` | same | now | 0, 0 |
| `event_catalogue_check.py` | same | now | 0, 50 core and 53 tool names |
| `guard_census.py`, `guard_census.py --check-runbook` | same | now | 0 and 0, 0 watched by nothing |
| `prose_check.py`, `source_provenance_check.py` | same | now | 0 (864 files), 0 (1706 files) |
| B1, B2, B3, B7, B8, B9, B10a, B10b, R13 | `git archive HEAD` into `target/f226-review3-mut`, `CARGO_TARGET_DIR=<wt>/target`, `cargo test -q -p ocelli-cache --no-fail-fast`, each alone, anchor count asserted 1, reverted | now, copy deleted after | all exit 101, failing tests as above |
| R13 under `--release` | same copy | now | 101, `left: 0, right: 100` |
| R1, R2, R3, R5, R6, R7, R10 | same copy | now | all exit 101 |
| `RELEASED_BYTES` 25, 23 | same copy, `node_modules` symlinked, `events.test.ts` | now | 1 failed of 8 each, 8 passed restored |
| Arm removals N-a, N-b, N-c, N-d | same copy, full `ocelli-cache` suite, then a throwaway `tests/zz_probe.rs` | now | suite exit 0 on all four. Probe red on N-a, N-b, N-c with the panic at `alloc/src/collections/btree/search.rs:121`, green on N-d |
| GLM's B1 to B9 and R13 table and gate tails | `glm-F-226-fix2.log` | prior receipt | consistent with the replays, not relied on |

## Defects

### D1, three of `bounds_are_empty`'s four arms can be removed with the suite green, and the new table's comment says they are covered

**Where**: `crates/ocelli-cache/src/lru.rs:139` to `:141` (the `Inc/Exc`, `Exc/Inc` and `Exc/Exc` arms). The test `an_inverted_or_equal_excluded_range_releases_nothing_and_changes_nothing` at `:738`, and the comment opening `a_release_over_every_bound_shape_returns_exactly_the_keys_named` at `:765`.
**What**: the only empty-shape test exercises an inverted `Included..=Included` (`5..=3`) and an equal `Excluded/Excluded` (`Exc(4), Exc(4)`). The properties draw only `lo..=hi`. So nothing drives an inverted half-open or excluded range. Measured in a scratch copy, each alone, full `ocelli-cache` suite at exit 0:
- N-a, the `(Included, Excluded)` arm deleted, so it falls to `_ => false`.
- N-b, the `(Excluded, Included)` arm deleted.
- N-c, the `(Excluded, Excluded)` arm narrowed to `lo == hi`.
Under each, `remove_range((Bound::Included(5), Bound::Excluded(3)))`, `((Excluded(5), Included(3)))` or `((Excluded(5), Excluded(3)))` respectively panics in `BTreeMap::range` ("range start is greater than range end", `search.rs:121`). The new table's comment says "the equal excluded and inverted shapes are the empty cases the test above covers", which is true for one inverted shape and false for the other three.
**Why it is wrong**: microscope sections 2 and 4. The guard's own doc and `remove_range`'s doc say an inverted range "releases nothing and returns an empty `Vec`" and HLD section 23 leaves no exported path a panic can reach. Three of the four branches that make that true are present and unwatched, which is the same shape as pass 2's D1 in the other direction. The comment is a claim in prose that is false.
**Evidence**: the arm mutations above against the suite (exit 0) and against a five-test probe (red on N-a, N-b, N-c). N-d, `Inc/Exc` to `lo > hi`, is green under both because `Included(k)..Excluded(k)` does not panic in std and returns nothing, so it is behaviourally harmless and not part of this finding.
**Scope and regression**: present since `a113768`, which pass 2 reviewed and did not replay per arm. This round's comment newly asserts the coverage. No caller reaches it today, since `release_series` and `release_volume` pass ordered `..=` bounds. The plan names `remove_range` as the release primitive for later stories that may compute any `RangeBounds`. Smallest fix: add `(Included(5), Excluded(3))`, `(Excluded(5), Included(3))` and `(Excluded(5), Excluded(3))` to the empty-shape test, each asserting `[]` with `used`, `len` and both keys unchanged, and correct the table's comment to name what the test above covers. Then replay N-a, N-b and N-c and confirm each is red.

## Smells

None.

## Nitpicks

- **N1.** `cache.md:171` and the `encode` doc both end "so every kind keeps `bytes` at offset 16". In context the kinds are the four `encode` writes, 1, 2, 3 and 7. The same crate's kinds 4 and 5 (`TelemetryEvent::encode`) have no `bytes` at 16, and `cache.md:177` names kinds 1 to 5 two lines later. "every kind `encode` writes" would remove the misreading.
- Carried from pass 2, outside this diff: pass 2's N3 (`RingEvent::kind`'s "One of this crate's `KIND_` constants", untrue since kind 6) is still open. Not counted.

## Verified clean

- `lru.rs` module doc and `cache.md:287` to `:291`: `insert`, `set_budget` and `remove_range` are the only `Lru` methods that build a `Vec` (grepped every method in the non-test part of `lru.rs`). The early `Vec::new()` on the guard does not allocate, which the sentence does not contradict.
- `cache.md:53` and the `Shifting` doc: four `Shifting` tests, one each for `remove`, an eviction, a replacement and `remove_range`. Counted.
- The new `Shifting` test's comment, including the saturation sentence, checked under both profiles.
- The bound-shape table covers all nine `Bound` combinations, twelve rows, each on a fresh cache, asserting keys, `used`, `len` and survivors.
- The proptest generator and arm comments say only what the arms assert. The property 1 doc's "a range judged empty that holds keys and a double release both go red here" is true (B1 and R6 red in it).
- `image_cache.rs` fixture comment against the fixture, the range at `image.rs:450` and the lead's amendment.
- `image.rs` `CacheEvents` doc (pass 2 N2) is true. `events.md:46` to `:50` reorder (pass 2 N1): "the same reason" now refers to "no 5.8.2 name", and "one of them" still reads correctly.
- `cache.md:164` to `:173` against `CacheEvent::encode`, `released_tests` and `events.ts` `readReleased`, offset by offset.
- No `unsafe`, `as` cast or `wasm-bindgen` in the diff. No TypeScript change in this round, so AC17 is as pass 2 left it.

## Limitations and handoff

- Not exercised: the folder viewer half (AC10 to AC16, AC18), any browser, GPU or oracle job.
- All mutations and the probe ran in a `git archive` copy under `target/`, deleted after. A `--release` test build was added to the worktree's shared `target/`. Nothing tracked was edited. At the end HEAD is `fc20d84` and the tree `c673549`, unchanged, and the untracked paths are the three review reports.
- Next owner: the implementer fixes D1 and optionally N1. A fresh pass 4 replays N-a, N-b and N-c and re-reads the table comment.
- This is a bounded review of the library half. It is not completed acceptance and not sprint readiness.
