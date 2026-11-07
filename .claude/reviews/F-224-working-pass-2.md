# F-224 review, pass 2

**Reviewed**: `git diff d33535b 9f4d85c` (the pass 1 remediation) in worktree `ocelli-wt-f-224`, branch `work/f-224-xai`, HEAD `9f4d85c`, clean tree apart from the untracked pass 1 report. Plus one sweep of the whole library half, `git diff c7aa9f3 9f4d85c -- crates packages docs README.md`, which is `fc20d84..HEAD` over those paths minus what merge 03c57a2 brought, since pass 1 showed that merge brought exactly F-226's c7aa9f3. Scope is AC1 to AC12, plus the one `use.html` line this round changed.
**Author / reviewer**: remediation by GLM-5.3 in opencode (per the lead's brief and `glm-F-224-fix1.log`. The commit's author field is Atul Sharma, so authorship rests on the brief, not the trailer). Reviewer Claude Opus 5.5 as an independent `/microscope` subagent. It did not write or remediate any of this code.
**Result**: 1 defect, 0 smells, 0 nitpicks

## Prior findings

| Pass 1 | Status | How it was checked |
|--------|--------|--------------------|
| D1, vacuous `TierCounters` block | Closed | Block and its unused imports are gone. The plan's "Lead ruling after review pass 1" makes AC7's counter half structural. Checked structurally: `ImageCache`'s public surface (`new`, `insert`, `get`, `contains`, `remove`, `release_series`, `pressure`) returns no `Admission`, and `insert` consumes the one it gets. M13 replayed, see below |
| D2, stale LLD row | Closed | `docs/lld/cache.md:24` now says `Lru<FrameKey, HeldFrame>` and names the private `HeldFrame`. Matches `image.rs` (`decoded: Lru<FrameKey, HeldFrame>`, and `struct HeldFrame` is not `pub`) |
| S1, no `ImageCache`-level recency test | Closed | `a_hit_refreshes_recency_so_the_second_is_evicted` added. M15 replayed and it goes red |
| N1, `content_id.rs` comment | Closed | The new sentence is what per-step bijectivity gives. AC2's two values recomputed independently (`0x49e8b650ff1cd3f1`, `0x49e8b750ff1cd5a4`) |
| N2, stale bytes from the removed entry | Closed, checked below | |
| Carried: `use.html` "five options" | Changed this round, now D1 | |
| Carried: CHANGELOG 0.x entry | Still owed, not counted | |

## Mutation replay

A scratch copy (`rsync` of the worktree without `.git`, `target`, `node_modules`) under `target/mut-f224-p2`, its own `CARGO_TARGET_DIR` `target/mut-target-p2`, `cargo test -p ocelli-cache --tests --no-fail-fast`. Baseline green (lib 36, image_cache 17, telemetry 13, content_id 3). Each mutation applied, run, reverted. Copy, target dir and script deleted. No worktree file was edited, so no touch restore was needed.

| Mutation | Result | Red tests |
|----------|--------|-----------|
| M13, `TierCounters::record` counts `displaced` as an eviction | red | `telemetry.rs`: `counters_count_an_eviction_and_a_refusal_and_ignore_a_displacement`, `eviction_bytes_match_what_the_tier_stopped_holding`. No `image_cache` test, which is what the structural ruling expects |
| M15, `Lru::get` does not move `used_at` | red | three `lru::tests` unit tests and **`a_hit_refreshes_recency_so_the_second_is_evicted`** |
| N2a, stale reports 0 bytes | red | `stale_removal_is_explicit`, `metadata_changes_are_stale`, `a_reprocessed_object_in_the_same_slot_is_not_served` |
| N2b, stale path reads with `get` and does not remove | red | the same three |
| N2c, stale reports twice the bytes | red | the same three |
| N2d, `Lru::remove` does not release its bytes | red | two `lru::tests`, `stale_removal_is_explicit`, `a_reprocessed_object_in_the_same_slot_is_not_served`, plus four other cache tests |

A method note for whoever replays next. Without `--no-fail-fast`, M15 shows only the `lru::tests` failures, because cargo stops after the failing lib binary and never runs `image_cache`. My first M15 run looked like S1 was still open for that reason.

## N2's stale path

`let bytes = self.decoded.remove(key).map_or(0, |h| h.frame.bytes());`. `Lru::remove` releases `entry.bytes`, the admitted size recorded from `HeldFrame::bytes` at insert, and the report reads `frame.bytes()` of the value that came out. These are equal because `DecodedFrame` has no mutating accessor and `HeldFrame` is private. The `0` fallback is unreachable, since `get` returned `Some` for the same key under the same `&mut self`. The comment "The reference is not used past this line" now sits after `let stale = ...`, the last use of `held`, so it is still true. Events: `get` returns no `CacheEvents`, and `stale_removal_is_explicit` still asserts `Stale { bytes: 32 }`, `pressure().used` 0, an empty insert after it at `used` 32, and the `Displaced { bytes: 32 }` control. N2a to N2d show each of those figures is watched.

## Defects

### D1, `use.html` now says six options above a table of five
**Severity**: defect (microscope section 3, false prose in an always-in-scope document).
**Where**: `examples/folder-viewer/src/use.html:393` and the table at 394 to 403, section 5.6 DICOMweb.
**What**: The sentence now reads "Its constructor takes six options." and the `<tbody>` directly below it has five rows: `studiesServiceUrl`, `wadoUriUrl`, `fetch`, `wasm`, `sink`. `cache` is nowhere on the page (`grep -n 'cache' use.html` finds only the ring event names and the `ocelli-cache` chapter). Before this round the page was stale against the code but agreed with itself. Now it contradicts itself in two adjacent elements, and a reader looking for the sixth option cannot find it or its default.
**Expected**: the count and the table agree with `DICOMWEB_CLIENT_OPTION_KEYS` in `packages/core/src/options.ts`, which has six keys, the last `cache`.
**Why it counts this round**: the round changed that line, and the change created the contradiction. GLM noticed the five rows and left them because its brief allowed one line. That makes it a scoping problem rather than carelessness, but the page is still false as shipped.
**Smallest true fix**: add one row to the table, not revert the count. Reverting to "five" puts back the stale claim about the code. A row consistent with `dicomweb.ts`'s doc comment and README lines 348 to 356 would be: `<tr><td><code>cache</code></td><td><code>DicomwebCacheMode</code>, optional. <code>"no-cache"</code> by default, which revalidates with the server. <code>"no-store"</code>, <code>"reload"</code> and <code>"default"</code> are accepted, and <code>"force-cache"</code> and <code>"only-if-cached"</code> are refused with <code>InvalidRequest</code></td></tr>`. AC19's other `use.html` and `more.html` claims (the identity rule, the three limits) still belong to the viewer round, but this row is the minimum that makes the page stop contradicting itself.

## Acceptance map, AC1 to AC12

| AC | Named test or ruling | Executed now |
|----|----------------------|--------------|
| AC1 | `image.rs` `content_id_tests::fixture_fnv1a_reference_vectors` | lib 36 ok |
| AC2 | `content_id.rs` `fixture_content_id_values` | ok, two values recomputed independently |
| AC3 | `content_id.rs` `fixture_field_boundaries` | ok |
| AC4 | `content_id.rs` `property_one_byte_change_changes_the_id` | ok |
| AC5 | `image_cache.rs` `a_reprocessed_object_in_the_same_slot_is_not_served` | ok, red under N2a to N2d |
| AC6 | `image_cache.rs` `metadata_changes_are_stale` | ok, red under N2a to N2c |
| AC7 events | `image_cache.rs` `stale_removal_is_explicit` | ok, red under N2a to N2d |
| AC7 counters | Lead ruling after review pass 1, structural. Confirmed by the public surface above, and by M13 being caught only by `TierCounters`' own tests | checked |
| AC8 | `crates/ocelli-viewport/tests/scroll.rs`, the `ScrollError::Stale` case at line 138 | scroll 6 ok |
| AC9 | `image.rs` `handoff_tests` and `released_tests`, `crates/ocelli-wasm` session tests | lib ok, `cargo test -p ocelli-wasm` 34 ok |
| AC10 | `dicomweb.test.ts` `sends the cache mode on every request` | vitest ok |
| AC11 | `dicomweb.test.ts` `refuses a cache mode that can serve a stale response` | vitest ok |
| AC12 | `instance-bytes.test.ts` `sends the requested cache mode and defaults to no-cache` | vitest ok |
| S1 guard | `image_cache.rs` `a_hit_refreshes_recency_so_the_second_is_evicted`, the state table's hit row | red under M15 |

## Whole-library sweep

`git diff c7aa9f3 9f4d85c -- crates packages docs README.md`, 16 files. Read in full for `crates/ocelli-cache/src`, `crates/ocelli-viewport/src`, the TS sources and every doc and README line. Nothing new beyond pass 1's verified-clean list. Rechecked this pass:

- No `as` cast. `u64::from(*byte)`, `u64::try_from(len).unwrap_or(u64::MAX)`, `wrapping_mul`.
- `get`'s two-read shape: on the stale path the first read's tick goes to an entry removed in the same call, and the hit path's second read makes the frame most recently used (M15).
- `docs/lld/dicomweb.md` "refused at construction ... before any fetch" also holds for `postWadoInstance`, because it builds a `DicomwebClient` with the field passed through (`instance-bytes.ts:40` to 45).
- `docs/lld/cache.md` "Frame identity, F-224", the three new type rows and the corrected `ImageCache` row agree with `image.rs`. The `DecodedFrame` row and line 239's structural `TierCounters` sentence are true.
- README `DicomwebClient` paragraph (lines 348 to 356) agrees with `dicomweb.ts`. No README, `use.html` or `more.html` sentence names `ImageCache`, `samples_for_upload`, `FrameLookup` or `ContentId`, so the Rust signature change leaves none of them stale.
- No `unsafe`, no `wasm-bindgen`, no new dependency, no boundary or tier change.

Every changed sentence was executed: the two `image_cache.rs` comments against the mutations above, the `content_id.rs` comment against the recomputed values, the `cache.md` row against `image.rs`, the `image.rs` comment against the code below it, and the `use.html` line against `options.ts` and the table (D1).

## Evidence

| Command | Source | Executed now / prior | Exit and observation |
|---------|--------|----------------------|----------------------|
| `cargo fmt --all --check` | HEAD 9f4d85c, `CARGO_TARGET_DIR=<wt>/target` | now | 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | HEAD | now | 0 |
| `cargo test -p ocelli-cache -p ocelli-viewport` | HEAD | now | 0, image_cache 17, content_id 3, lib 36, scroll 6, all ok |
| `cargo test -p ocelli-viewport --test typegen` | HEAD | now | 0 |
| `cargo check -p ocelli-cache --target wasm32-unknown-unknown` | HEAD | now | 0 |
| `cargo test -p ocelli-wasm` | HEAD | now | 0, 34 passed |
| `npx vitest run packages/core` | HEAD | now | 0, 440 passed |
| `npm run typecheck`, `npm run lint` | HEAD | now | 0, 0 |
| `guard_census.py`, `guard_census.py --check-runbook` | HEAD | now | 0, 0. "1102 refusal(s) in 78 file(s), all claimed", "0 watched by nothing" |
| `prose_check.py` | HEAD | now | 0, "clean over 864 file(s)" |
| `source_provenance_check.py` | HEAD | now | 0, "clean over 1707 files" |
| Mutations M13, M15, N2a to N2d | scratch copy of HEAD | now | all red, table above |
| GLM's red run and check tails | `glm-F-224-fix1.log` | prior receipt, not relied on | consistent with mine. Its "only that test failed (16 passed)" was scoped to `image_cache` |

Environment: Apple arm64, Darwin 27.0.0, the worktree's pinned toolchain. No GPU, oracle, browser or bench job. `git status --porcelain` before this report showed only the untracked pass 1 report, and HEAD was still `9f4d85c`.

## Limitations and handoff

- Not exercised: no browser fetch, so WHATWG cache-mode behaviour is taken from the plan's citation. AC13 to AC18 and the rest of AC19 are the viewer round.
- Still owed before completion, not counted here: the CHANGELOG 0.x entry for the `ImageCache::insert`, `get` and `samples_for_upload` signature change and the new DICOMweb default.
- Next: add the `cache` row to `use.html` (D1), then a fresh pass checks that row. The library code and tests for AC1 to AC12 are clean on this pass. With D1 open, this pass does not pass.
