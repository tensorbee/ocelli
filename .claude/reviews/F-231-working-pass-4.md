# F-231 review, pass 4

**Reviewed**: `git diff fc82f385 ebcb156a`, the pass 3 remediation, plus one sweep of the whole story, `git diff 308a0f06 ebcb156a -- . ':!.claude/reviews'`. `308a0f06` is F-224's final commit and the second parent of the merge `5fa5a69b`, so that range is `03c57a2..HEAD` without what the F-224 merge brought. HEAD `ebcb156a035d92797f96040c9410938c0e6b6ec4`, tree `b6d906aade809a0fa1af802358ce3c966aa41b6e`, worktree `ocelli-wt-f-231` on `work/f-231-xai`. Before this report the only untracked files were the pass 1 to pass 3 reports.
**Author / reviewer**: the brief says a Claude agent wrote the remediation. The commit's git author is the operator's identity, so that cannot be confirmed from the tree. The reviewer is an independent Claude agent. It did not write the remediation, the plan, the rulings or passes 1 to 3.
**Result**: 1 defect, 0 smells, 3 nitpicks

## Acceptance challenged

- **No code changed in the remediation.** Every changed line in `git diff fc82f385 HEAD` is a `//` or `*` comment line, except the README hunk, which is prose. That covers five files: `README.md`, `session.rs:3448` to `:3451`, `use.html:409` to `:412`, `bulk.test.ts:937` to `:939` and `frame-cache.ts:15` to `:18` and `:120` to `:125`. The release module is 2,202,813 bytes, the same figure as pass 3.
- **The rewritten `showResident` sentences, run on the built dev module** (rebuilt from HEAD). Stack of three under series `0x1_0000_0005`, budget 1024, indices 0 and 1 committed with content, selection at 2:
  - P1. Wrong epoch at the selected index 2: `false`, no event.
  - P2. Wrong epoch at index 0, which is cached and matches: `false`, no event, and `frameResident` was still true. Epoch 0 at index 0 also gave `false`.
  - P3. Current epoch at index 0 (cached, matching): threw `CommitRefused`. Slice 2 stayed selected, loading stayed set, no event, and the entry was kept. At index 1 with a mismatched content: threw, and the entry was kept, so the refusal comes before the stale rule. At index 99: threw.
  - P4. Current epoch at the selected index 2, empty slot (a miss): `false`, display unchanged, no event.
  - P5. Scrolled to 1, then a stale content at the selected index 1: `false`, display unchanged, and the entry was removed. A stale extent (`columns: 2`) at the selected index 0: `false`, entry removed.
  - P6. A hit at the selected index 0: `true`, loading cleared.
  - P7. After a restack: the old epoch gave `false` at the selected index and at another one. The new epoch gave `true` at the selected index and threw at another one.
  Each clause of README `:313` to `:316`, the `use.html` sample comment and the `showResident` doc holds. Pass 3 N1 is closed.
- **The `frame-cache.ts` module doc.** Each function validates every argument before its export call: `residentRequest`, `u32` and `u64`, and `releaseStudy` maps the whole list first. The only `throw` after an export call in the file is `showResident`'s answer-2 branch. Pass 3 D1 is closed.
- **The `bulk.test.ts` comment against `frameCommitMeta`** (`bulk.ts:494` to `:513`). If the `:499` guard is removed, `{content}` alone has `selection` false and `cached` false, so the content is silently dropped. That is the case that depends on the guard. A lone position has `selection` true, and the missing position reaches `u32(undefined)` and is refused. Pass 3 D3 is closed.
- **The S1 test comment, by mutation** (scratch copy, its own target directory, `--no-fail-fast`):
  - M2, `Displaced` counted in `note_counters`: red at `:3502`.
  - M1, `Released` counted only: green, 51 passed.
  - M6, `release_series` calls `note_counters` and `Released` is counted: red at `:3528`, the release half.
  - M6b, the fold added and `Released` not counted: green.
  So "goes red only when a release both reaches note_counters and is counted there" holds, and pass 3 D2 is closed.
- **The whole-story sweep, one discriminating case per area.** AC11's session test was run with a probe that counted the event kinds held. The finding is D1 below.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo fmt --all --check` | HEAD, `CARGO_TARGET_DIR=<wt>/target` | now | 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | HEAD | now | 0 |
| `cargo test -p ocelli-cache -p ocelli-viewport -p ocelli-wasm` | HEAD | now | 0. `ocelli_wasm` lib 51 passed |
| `bin/ocelli.sh wasm`, `pin_and_size_check.py --with-size` | HEAD release | now | 0, 0. 2,202,813 bytes, baseline 2,151,196, ceiling 2,258,755 |
| `ci/check-bindgen-isolation.sh` | HEAD | now | 0, confined to `ocelli-wasm` |
| Generated `pkg/ocelli_wasm.d.ts` | HEAD release | now | new exports return `number` or `bigint \| undefined` (AC19) |
| `wasm-pack build crates/ocelli-wasm --dev --target web --out-dir target/boundary-wasm` | HEAD | now | 0 |
| `npx vitest run --config vitest.boundary.config.ts` | HEAD dev module | now | 0, 17 passed |
| `npx vitest run packages/core` | HEAD | now | 0, 458 passed in 49 files |
| `npm run typecheck`, `npm run lint` | HEAD | now | 0, 0 |
| `guard_census.py` | HEAD | now | 0, 0 watched by nothing |
| `prose_check.py`, `source_provenance_check.py`, `event_catalogue_check.py` | HEAD | now | 0, 0, 0 (50 core and 53 tool names) |
| Probes P1 to P7 | built dev module through the public `@ocelli/core` helpers, a probe under ignored `target/rv231p4` | now | as in "Acceptance challenged" |
| M1, M2, M6, M6b | scratch copy of `git archive HEAD`, own target dir, `--no-fail-fast` | now | green, red, red, green |
| AC11 session probe | scratch copy, a `panic!` printing the held kinds after the test's asserts | now | first kind 4 (pressure), 1 pressure, 255 displaced, 256 held |
| AC17 parent-commit figure | pass 1 to 3 receipts | prior | not rerun |

## Defects

### D1, the AC11 session test says 300 commits displace 300 events, and they displace 299

**Where**: `crates/ocelli-wasm/src/session.rs:3532` to `:3534`, the comment on `a_burst_over_the_ring_capacity_counts_the_dropped`.
**What**: the comment says "300 commits of one index displace 300 events into an empty ring of 256". The first commit goes into an empty 4-byte budget. That is an insert, not a displacement, and it moves the band from Low to Critical (`10 * 4 >= 9 * 4`), so it pushes one `cache.pressure`. Only commits 2 to 300 displace. The burst is 1 pressure event and 299 displaced events. The totals the test asserts (256 held, 44 dropped) are still right, because the sum is 300.
**Why it is wrong**: microscope section 3, a false sentence about what the code does. A reader who takes the comment as written expects the ring to hold 256 `cache.displaced` events. It holds one `cache.pressure` and then 255 `cache.displaced`. Someone who tightens the test to "every held event is kinds 2" from this comment gets a red test and a misleading lead. Plan AC11 says "300 events in one burst" and never says 300 displacements, so the comment adds a claim of its own.
**Evidence**: a probe in a scratch copy, placed after the test's own asserts, printed `first Some(4) pressure 1 displaced 255 held 256`. `KIND_PRESSURE` is 4 and `KIND_DISPLACED` is 2 (`image.rs:402`, `:409`).
**Scope and regression**: present on HEAD since `747ca66b`, and passes 1 to 3 did not catch it. Test comment only, and the behaviour is correct. Smallest fix: "The first commit inserts and pushes one pressure crossing, and the other 299 displace, so 300 events reach an empty ring of 256". Or drop the word "displace". Prefer rewording the existing sentence to adding a new one.

## Smells

None.

## Nitpicks

- N1. `examples/folder-viewer/src/use.html:415` to `:416`: "a mismatched show returns false and removes the entry" has no qualifier of its own. A mismatched show at another index under the current epoch throws and keeps the entry (P3). Under a wrong epoch it returns false and keeps the entry (P2). The comment four lines above now scopes it ("at the selected index a miss or a stale entry returns false"), so it reads correctly in context. Saying "a mismatched show at the selected slice" would remove the dependence on the earlier comment.
- N2. `packages/core/src/frame-cache.test.ts:6`: "The bigint passthrough of AC14 and AC15" names the wrong criteria. In the plan, AC14 is `releaseStudy`'s deduplication, run over `5n` and `6n`, and AC15 is refusals. The passthrough cases (`0x1_0000_0001n`, `0x1_0000_0005n`) carry no AC label, and the plan puts truncation at AC13, on the built module. The behaviour clause of the sentence is true.
- N3, carried from pass 3 N2. The boundary test title "answers false only for a miss" and the ruling's wording say "only for a miss", while the code and every document say false for a miss, a stale entry or a wrong epoch.
- Carried: pass 2 N1 (`evicted_bytes` is gathered and then dropped by F-034's 32-byte payload) is unchanged and outside F-231.

## Verified clean

- **Pass 3 D1, D2, D3 and N1 are closed.** Each was run as described above, not read.
- **Each AC maps to a named test or observation:**
  - AC1: `tests/image_cache.rs` `a_committed_frame_counts_its_samples_at_four_bytes`.
  - AC2: `four_1mib_commits_report_pressure_then_eviction_in_order`.
  - AC3: `a_default_budget_caches_nothing_and_leaves_the_ring_to_kind_6`.
  - AC4: `residency_is_not_a_use`, plus `residency_on_the_committed_tier_is_not_a_use` in `image_cache.rs`.
  - AC5: `show_resident_displays_and_stale_refuses` and `show_resident_refuses_an_index_that_is_not_the_selection`.
  - AC6: `a_stack_change_keeps_the_cache_and_the_epoch_gates_it`.
  - AC7: `release_series_reports_one_event_and_keeps_the_display`.
  - AC8: `a_shrunk_budget_evicts_then_reports_the_budget_change`, plus `a_budget_change_that_crosses_a_band_reports_the_pressure_last`.
  - AC9: `a_non_multiple_of_four_and_a_contentless_commit_are_not_cached`.
  - AC10: `a_frame_larger_than_the_whole_budget_is_refused_and_displayed`.
  - AC11: `tests/ring.rs` `three_hundred_events_into_an_empty_ring_push_256_and_count_44` and the session case, whose comment is D1.
  - AC12: boundary `hashes a content id in the core, drops the slot, and answers the pinned FNV value`, with the native `content_id_hashes_the_slot_and_drops_it`.
  - AC13: boundary `caches, shows, reports and releases committed frames with bigint identities`, plus the ruling test.
  - AC14: `frame-cache.test.ts` `releases a study once per series and sums the counts`.
  - AC15: `refuses a bad input before any export call`, `refuses an unknown request key ...` and `refuses a request that is not an object`.
  - AC16: `events.test.ts` `names_the_producer_that_now_ships_the_evicted_kind` and `event_catalogue_check.py` exit 0.
  - AC17: this pass's HEAD figure, inside the ceiling with no `--accept-size`. The parent figure is from prior passes.
  - AC18: the isolation script.
  - AC19: `mod bind` and the generated `.d.ts`.
  - AC20: README `:307` to `:320`, `use.html` `:365`, `:391` to `:418` and `:742`, and `more.html:414`.
- **The whole-story sweep. No other false sentence was found** in the `ocelli-cache` diff (`CommittedFrame`, `CommittedSource`, `HeldValue<V>`, `lookup`, `is_resident`, `set_budget`, `Lru::peek`, `Debug`), in `ring.rs` `push_with_seq` (sequence consumed on a full ring, the same as `push_presented`), in the `session.rs` methods and their doc comments, in the bind, in `bulk.ts` (`FrameCommitMeta`, `sessionBulkSink` routing, `contentId`), `options.ts`, `events.ts`, `index.ts`, or the `docs/lld/cache.md` and `more.html` hunks. Specific checks:
  - `Stored` derives no `PartialEq` (`value.rs:15`).
  - `content_id` drops the slot on every refusal except a foreign ticket.
  - `Budgeted::bytes` saturates.
  - `stack_key_at` uses `usize::try_from`.
  - No new `as` cast, `#[allow]` or `unsafe` anywhere in the story.
- **The README and use.html session-cache paragraphs, re-read in full.** Every API, behaviour and refusal sentence was checked against P1 to P7 or against pass 3's executed replays (R2 to R6, release and pressure ordering). The only other imprecision is N1.

## Limitations and handoff

- No GPU, no oracle and no browser worker. The boundary runs used the dev module under Node with canvas stubs.
- AC17's parent-commit figure was not rerun. Saturation at `u32::MAX` was not driven. `CHANGELOG.md` has no F-231 line. Sibling stories land theirs at integration, so this is noted and not raised.
- The scratch copy and the probe directory `target/rv231p4` were deleted. The worktree's dev module and release `pkg/` were rebuilt from HEAD and are ignored output. After cleanup, `git status --short` showed only the pass 1 to pass 3 reports, and `git write-tree` was `b6d906aade809a0fa1af802358ce3c966aa41b6e`, equal to `HEAD^{tree}`. Nothing tracked was edited.
- Next owner: implementation rewords D1, one test comment with no behaviour change. N1 and N2 are cheap to fix in the same edit. Pass 5 reviews that change.
