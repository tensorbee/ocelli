# F-226 review, pass 4

**Reviewed**: `git diff fc20d84 HEAD` (the pass 3 remediation) and one sweep of `git diff 73b68ad HEAD -- crates packages docs examples/folder-viewer/src/use.html`, on branch `work/f-226-xai`, worktree `ocelli-wt-f-226`, HEAD `c7aa9f36460342a02f9acf5a4f3bd8faacc23301`, tree `7fe3e1b30894fd11a82a34c97343a894acb9a0fa` at start and at end. Library half only, AC1 to AC9 and AC17. The untracked paths at start were the pass 1 to pass 3 reports.
**Author / reviewer**: the remediation commit `c7aa9f3` is by a Claude implementer, per the lead's brief. The trailer alone cannot establish that. The reviewer is a separate Claude subagent that wrote neither the remediation nor passes 1 to 3.
**Result**: 0 defects, 0 smells, 2 nitpicks

## Acceptance challenged

- **Pass 3 D1, every arm of `bounds_are_empty`**. Each mutation was applied alone in a `git archive HEAD` copy under `target/`, with the anchor count asserted 1, and the full `ocelli-cache` suite was run with `--no-fail-fast`:
  - A, the `Inc/Inc` arm deleted (newly asked for): exit 101. Red in `an_inverted_or_equal_excluded_range_releases_nothing_and_changes_nothing` and in both `invariants.rs` properties, with the panic at `btree/search.rs:121`.
  - B (pass 3 N-a), the `Inc/Exc` arm deleted: exit 101, red in the empty-shape test, panic at `search.rs:121`.
  - C (N-b), the `Exc/Inc` arm deleted: exit 101, same test, same panic.
  - D (N-c), `Exc/Exc` narrowed to `lo == hi`: exit 101, same test, same panic.
  - E, `Exc/Exc` narrowed to `lo > hi`: exit 101, panic at `search.rs:112`, so the equal excluded pair `(Excluded(4), Excluded(4))` carries its own row.
  - F, `Exc/Inc` narrowed to `lo > hi`: exit 0. This is the same shape as pass 3's N-d. `(Excluded(k), Included(k))` does not panic in std and names nothing, so the narrowing changes no behaviour. It is not a finding.
  Pass 3 D1 is closed.
- **The changed comments**. The empty-shape test's comment says each of the four arms is driven inverted, 5 against 3, plus the equal excluded pair. The test body holds exactly those five calls, `lo..=hi` being `Included..=Included`, and A to D above show each call is the one that goes red. The bound-shape table's comment now says the test above covers "every one of the four bounded pairs inverted and the equal excluded pair". That is true. "The adjacent excluded pair names nothing" is the `(Excluded(3), Excluded(4))` row, which expects `[]`.
- **Pass 3 N1**. `image.rs` `encode` doc and `cache.md:171` now say "every kind `encode` writes keeps `bytes` at offset 16". `encode` writes kinds 1, 2, 3 and 7. The keyed arms write bytes at `payload[16..24]` and `Released` at `payload[16..24]`, so the sentence is true and no longer reads over kinds 4 and 5. Closed. The `cache.md` line now runs past the paragraph's wrap width, which is cosmetic.
- **AC7 under an independently chosen wrong pairing**. AC7's named mutation pairs `cache.released` with `IMAGE_CACHE_IMAGE_REMOVED`. Repointing that row to `cache.released` and deleting the `UNMAPPED_EVENTS` row turns `names_are_unique` and `ring_kinds_match_generated` red, exit 1. I also tried a pairing the plan does not name: `VOLUME_CACHE_VOLUME_REMOVED` repointed to `cache.released` with `delivery: "ring"`, and the unmapped row deleted. `events.test.ts` passes 8 of 8 and `event_catalogue_check.py` prints OK. See N2.
- **AC1 recomputed** from the plan alone. 1 + 2 + ... + 256 = 511, and 4 + 8 + 16 + 32 = 60, so 451. After `get(1)` and `get(3)` the ticks are 1 at 10 and 3 at 11. In-range recency is 4, 5, 6, 3. The survivors are 2 (tick 2), 7, 8, 9 and 1 (tick 12 after the later `get`). 549 is free, so a 600-byte insert evicts 2 (551 free) and then 7 (615 free), and used ends at 985. The test asserts each of these.
- **AC3 under the amendment**. 6 * 32 = 192, 5 * 32 = 160, 192 - 160 = 32. The comment's three end-bound and start-bound mutations each leave one of `(5,0,0)` or `(5,u32::MAX,u32::MAX)` behind, which the `!contains` lines reject. 32 + 161 = 193 is one over 192.
- **AC4 and AC5 bytes**. `0xA0B0C0D0 = 2695938256` and `0x00080000 = 524288`, recomputed with Python. The Rust and TypeScript fixtures carry the same 24 bytes.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo fmt --all --check` | tree 7fe3e1b, `CARGO_TARGET_DIR=<wt>/target` | now | 0 |
| `cargo clippy -p ocelli-cache -p ocelli-viewport --all-targets -- -D warnings` | same | now | 0 |
| `cargo test -p ocelli-cache -p ocelli-viewport` | same | now | 0, 19 binaries, 178 passed, 0 failed |
| `cargo test -p ocelli-viewport --test typegen` | same | now | 0, 2 passed |
| `npx vitest run packages/core` | same | now | 0, 48 files, 437 tests |
| `npm run typecheck`, `npm run lint` | same | now | 0, 0 |
| `event_catalogue_check.py` | same | now | 0, 50 core and 53 tool names at 5.8.2 |
| `guard_census.py`, `guard_census.py --check-runbook` | same | now | 0 and 0, "0 watched by nothing" |
| `prose_check.py`, `source_provenance_check.py` | same | now | 0 (864 files), 0 (1706 files) |
| Mutations A to F on `bounds_are_empty` | `git archive HEAD` into `target/f226-review4-mut`, `cargo test -q -p ocelli-cache --no-fail-fast`, each alone, restored from a saved original and `cmp`-checked | now, copy deleted after | A to E exit 101, F exit 0 (harmless, see above) |
| Catalogue pairing mutations | same copy, `node_modules` symlinked, `events.test.ts` and the script | now | `IMAGE_CACHE_IMAGE_REMOVED`: vitest exit 1, 2 failed. `VOLUME_CACHE_VOLUME_REMOVED`: vitest 8 passed, script OK |
| Stale count sweep | `grep` for "six kinds", "kinds 1 to 6" and "above 6" over README, `docs/lld`, `examples/folder-viewer/src`, `crates`, `packages/core/src` | now | no match |

## Defects

None.

## Smells

None.

## Nitpicks

- **N1.** `crates/ocelli-cache/tests/bricks.rs`, the comment opening `release_volume_releases_every_level_and_leaves_other_volumes`, says "with the level 0 brick [0, 1, 1] and the top brick at u32::MAX in all three coordinates held so the range's ends are occupied". The range's low end is `[0, 0, 0]` at level 0, which the fixture also holds, so the conclusion is true. But `[0, 1, 1]` is not an end, and I found no wrong range that only it rejects. Every start or end mutation I traced is already rejected by `[0, 0, 0]`, the level 1 brick or the top brick. Either name `[0, 0, 0]` as the low end, or say what `[0, 1, 1]` is for.
- **N2.** No test or script pins which cornerstone row `cache.released` may pair with. AC7's own named pairing goes red, as above. Repointing `VOLUME_CACHE_VOLUME_REMOVED` (unavailable today, reason "volume removal is not frame-cache eviction") to `cache.released` and deleting the unmapped row passes `events.test.ts` and the catalogue script. AC7 says the map is "unchanged", and that is a property of this diff, which I confirmed: the diff does not touch `CORE_ROWS` or the tool rows. So this is not an unmet AC. It does mean a later semantic edit of the map is caught only by review. It is not counted as blocking, because no AC or plan line claims a test pins the map's pairings.
- Carried from pass 2, outside this diff, not counted: `RingEvent::kind`'s doc at `image.rs:296`, "One of this crate's `KIND_` constants", has been untrue since kind 6.

## Verified clean

- **AC1**: `lru.rs` `remove_range_returns_the_keys_ascending_and_releases_their_bytes`, recomputed above. Pass 3 replayed R6 and R9 against it.
- **AC2**: `invariants.rs`, both properties, with `RemoveRange` drawn unsorted and `SetBudget`. The expectation is computed from the test's own `BTreeMap<u8, usize>` filter and never from `range`. Mutation A is red in both properties.
- **AC3**: `image_cache.rs` `release_series_releases_one_series`, against the lead's amendment.
- **AC4**: `image.rs` `released_tests::a_released_event_encodes_the_series_release_payload`, byte for byte against the plan's payload.
- **AC5**: `events.test.ts` `decodes_fixture_payloads` (series, frames and bytes, plus the exact 24-byte boundary), `refuses_a_short_or_unknown_payload` (23 bytes is null), and `refuses_unknown_kind` (kind 8 is null).
- **AC6**: `boundary.rs` `ring_kinds_are_unique` (7 entries, `KIND_RELEASED == 7`, pairwise distinct) and `tests/typegen.rs`, whose byte-for-byte comparison passes with `released: 7,`.
- **AC7**: `cache.released` is in `OCELLI_EVENTS` and `UNMAPPED_EVENTS`. `CORNERSTONE_EVENT_MAP` is not in the diff. The script passes. The named pairing goes red. N2 records the unnamed one.
- **AC8**: `bricks.rs` `release_volume_releases_every_level_and_leaves_other_volumes`. 8 + 16 + 4 + 32 + 64 = 124, 188 - 124 = 64, 5 entries. The repeat returns `ReleaseSummary::default()`. Volume 3 stays resident.
- **AC9**: by review. `get` and `insert` are not in the `lru.rs` diff. `remove_range` allocates the key `Vec` and the return only after the guard, and the guard's `Vec::new()` does not allocate. AC1's test asserts `len` stays 5 after a later `get`. The module doc and `cache.md` "insert, set_budget and remove_range" sentence agrees with the non-test methods.
- **AC17**: by review. `index.ts` gains only `export type { CacheReleasedPayload }`. `index.test.ts` is not in the diff. `grep releaseSeries\|releaseStudy` over `packages/` prints nothing. The only other TypeScript additions are the `cache.released` catalogue entries, the decoder arm, and `generated.ts`'s generator line.
- **The sweep of the whole library diff**: `ReleaseSummary` doc ("`pressure().used` drops by exactly this") holds, because `Brick::bytes` and `DecodedFrame::bytes` are `samples.len()` behind `&` access only, so the re-read in `release_volume` and `release_series` equals the admitted size. `Released`'s "Saturates at `u32::MAX`" matches `u32::try_from(..).unwrap_or(u32::MAX)`. `KIND_EVICTED`'s "1 to 5 and 7, with ... kind 6 between" is true. `events.md`'s table row 7, "any kind above 7", and the unmapped paragraph match `events.ts`. `public-api.md`'s "seven ring kinds" and `viewport.md`'s "Cache kinds 1 to 5 and 7 are not produced here" are true. The `use.html:365` sentence matches `RING_EVENT_KIND` and `decodeNamedEvent` name for name. The `invariants.rs` generator and arm comments say only what the arms assert. The `released_tests` doc's byte derivations are correct.
- No `unsafe`, `as` cast or `wasm-bindgen` in the diff. No pixel, LUT or geometry arithmetic in the diff.

## Limitations and handoff

- Not exercised: the folder viewer half (AC10 to AC16, AC18), including README and `more.html`, which the library diff does not touch and which AC18 assigns to that half. No browser, GPU or oracle job ran.
- All mutations ran in a `git archive` copy under `target/`, deleted after. Nothing tracked was edited. At the end HEAD is `c7aa9f3`, `git write-tree` is `7fe3e1b` and matches HEAD's tree, `git diff` is empty, and the untracked paths are the four review reports.
- The library half has no blocking finding at this pass. N1 and N2 are optional. This is a bounded review of the library half. It is not completed acceptance and not sprint readiness, which still need the folder viewer half and `gate --sprint`.
