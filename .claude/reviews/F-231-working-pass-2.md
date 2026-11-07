# F-231 review, pass 2

**Reviewed**: `git diff 20f2c92 e18f192`, the pass 1 remediation, read against the lead rulings in `.claude/plans/F-231-design.md` "Lead rulings after review pass 1". HEAD `e18f19226c496de43d139e1c3cd07702a8330348`, tree `930713451c21d3d0860a5592cdb494f24f672a97`, worktree `ocelli-wt-f-231` on `work/f-231-xai`. The only untracked path before this report was `.claude/reviews/F-231-working-pass-1.md`.
**Author / reviewer**: remediation by GLM-5.3 in opencode (e18f192). Reviewer is an independent Claude agent. It did not write the remediation, the plan, the rulings or pass 1.
**Result**: 2 defects, 1 smell, 4 nitpicks

## Acceptance challenged

- **Ruling 1, `show_resident` shows only the selected slice.** Pass 1's D1 replayed natively. Stack `[(5,0),(5,1)]`, budget 8, commit 0 at 10.0, scroll +1, commit 1 at 20.0, scroll -1, then `show_resident(1, ...)` gave `Err(Selection)`, the selection `Some(0)`, no samples, loading flag still set and no ring event. Adjacent cases: a repeat gives `Err(Selection)` again. A mismatched content at the unselected index gives `Err(Selection)` and the entry stays resident, so the refusal comes before F-224's stale removal. A show of the selected index 0 gives `Ok(1)` with 10.0 and clears loading. `commit_selected_frame(1)` at selection 0 gives the same `Err(Selection)`. On the built dev module the same sequence gave `shown=false`, `sliceIndex` 0 before and after, flags 2 before and after, `frameResident` true and 0 events. Then `showResident(index 0)` gave true and flags 0.
- **Ruling 2, a content id needs its stack position.** On the built module, `writeFrame` with `content` and no position, with `stackIndex` only and with `stackEpoch` only each threw `InvalidValue`. `content` of `2n ** 64n`, `-1n`, `1.5`, `"7"` and `null` with both positions each threw `InvalidValue`. `bulk_ticket()` read 0 before and after, and the displayed frame's flags did not change. `writeFrame` validates `meta` before `alloc` (`bulk.ts:209` to `:216`), so nothing is written.
- **Ruling 3, real counts.** Native walk, 4-byte frames against an 8-byte budget, then the payload printed at every offset:
  - Three commits give pressure, pressure and then `Evicted` (index 0, 4 bytes).
  - Requested reading 1: used 8, budget 8, entries 2, `evicted` 1 at offset 24, `refused` 0 at offset 28.
  - Reading 2: 0 and 0. The reading resets the counts.
  - A displace (same index, same content) then a reading: 0. A stale removal (selected, mismatched) pushes a crossing reading with 0, and the next reading is 0 too. A release pushes `Released` then a crossing reading with 0.
  - `set_frame_budget(4)` evicts one, then an 8-byte commit is refused. The next reading gives `evicted` 1 and `refused` 1.
  - The built module agrees. After the eviction, reading 1 decoded as `{"entries":2,"evicted":1,"refused":0}` and reading 2 as `evicted 0`.
- **Counter definition.** `TierCounters::record` (`crates/ocelli-cache/src/telemetry.rs:59` to `:72`) counts `admission.evicted` for "one insert or one `set_budget`", counts a refusal, ignores `displaced` and saturates. `note_counters` counts `Evicted` and `Refused` and skips `Displaced` and `Released`, with `saturating_add` on all three fields. A stale removal emits no `CacheEvent` (`FrameLookup::Stale`), so it cannot be counted. Budget-shrink evictions are counted, which matches `record`'s "or one `set_budget`". `TierTelemetry`'s "since the previous reading" holds, because `pressure_reading` is the only `take()` and only pushed readings call it. `emit_band` returns before `pressure_reading` when the band has not moved (mutation M10 below).
- **S1, `set_frame_budget(0)` over two Critical 1 MiB frames.** The full list natively was `Evicted` (instance 0, 1,048,576), `Evicted` (instance 1, 1,048,576), `BudgetChanged` (tier 1, budget 0, evicted bytes 2,097,152, evicted 2, dropped 0), then `Pressure` (used 0, budget 0, entries 0, evicted 2, refused 0). A reading after it gave all zeros. The built module gave the same order at 4-byte frames.
- **Recency through a show.** On the built module, `showResident(0)` before the third commit made index 1 the least recently used, and the third commit evicted instance 1, not instance 0. That is plan decision 7.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo fmt --all --check` | HEAD, `CARGO_TARGET_DIR=<wt>/target` | now | 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | HEAD | now | 0 |
| `cargo test -p ocelli-cache -p ocelli-viewport -p ocelli-wasm` | HEAD | now | 0. `ocelli_wasm` lib 50, 19 of them `session_cache` |
| `cargo test -p ocelli-viewport --test typegen` | HEAD | now | 0, 2 passed |
| `bin/ocelli.sh wasm`, `pin_and_size_check.py --with-size` | HEAD release | now | 0, 0. 2,202,801 bytes, ceiling 2,258,755. Decode 16,455. +374 bytes on pass 1's 2,202,427 |
| `ci/check-bindgen-isolation.sh` | HEAD | now | 0 |
| `wasm-pack build crates/ocelli-wasm --dev --target web --out-dir target/boundary-wasm` | HEAD | now, before the boundary run | 0, module written 21:22 |
| `npx vitest run packages/core` | HEAD | now | 0, 49 files, 457 tests |
| `npx vitest run --config vitest.boundary.config.ts` | HEAD dev module | now | 0, 16 tests |
| `npm run typecheck`, `npm run lint` | HEAD | now | 0, 0 |
| `event_catalogue_check.py` | HEAD | now | 0, 50 core and 53 tool names |
| `guard_census.py`, `--check-runbook` | HEAD | now | 0, 0 |
| `prose_check.py`, `source_provenance_check.py` | HEAD | now | 0 (864 files), 0 (1709 files) |
| Native probes `zz_probe_d1`, `zz_probe_d3`, `zz_probe_s1` | `git archive HEAD` under `target/rv231p2`, own target dir | now | the observations above |
| Built-module probes, scratch `describe` blocks | HEAD dev module | now | the observations above, plus the doc runs below |
| Rust mutations M1 to M14, `--no-fail-fast` | scratch copy | now | table below |
| TS mutations T1 to T6 | scratch copy | now | table below |
| GLM's seven red runs | e18f192 | prior receipt (its log) | overlap re-run as M1, M14, M2/M3/M8, M9, T3, T6, T1 |

| ID | Mutation | Result |
|----|----------|--------|
| M1 | selection check removed from `show_resident` | red, both `show_resident` tests |
| M2 | `Evicted` not counted | red, the D3 test and the S1 test |
| M3 | `Refused` not counted | red, AC10 |
| M4 | `Displaced` counted as an eviction | **green**, S1 below |
| M5 | `Released` counted as an eviction | **green**, S1 below |
| M6 | `pressure_reading` reads without `take()` | red, the D3 test |
| M7 | `note_counters` removed from `set_frame_budget` | red, the S1 test |
| M8 | `note_counters` removed from `cache_committed_frame` | red, AC10 and the D3 test |
| M9 | `emit_band` after `BudgetChanged` removed | red, the S1 test. Pass 1's S1 is closed |
| M10 | `emit_band` takes the reading before the crossing check | red, AC10 and the D3 test |
| M11 | selection checked after the lookup, so a stale unselected entry is removed | red, AC5 |
| M12 | `evicted_bytes` not summed | green, and not observable, N1 |
| M13 | the Selection refusal clears the loading flag | red, the D1 reproduction |
| M14 | the bind maps `Err` to 1, built into a scratch dev module | red, boundary AC13 |
| T1 | the content-without-positions refusal removed | red |
| T2 | the refusal keyed on either position, not both | green, equivalent, N2 |
| T3 | `u64` upper bound removed | red |
| T4 | `u64` type check removed | red |
| T5 | `u64` lower bound removed | red |
| T6 | `content` passed unvalidated | red. Pass 1's S2 is closed |

## Defects

### D1, the release sentences pass 1 flagged are still false in README and in `frame-cache.ts`

**Where**: `README.md:313`, `packages/core/src/frame-cache.ts:10` to `:11`.
**What**: both still say `releaseSeries` and `releaseStudy` "empty it", meaning the cache. `frame-cache.ts` also still says "one event per series through the ring", word for word what pass 1's D4 named. This diff re-wrapped that comment and kept the sentence. README now says "one event per series that held frames". But a release that crosses a band pushes a `cache.pressure` after the `cache.released`.
**Why it is wrong**: microscope section 3, and pass 1 D4's second half: "`releaseSeries` empties one series rather than the cache" and "a series with nothing cached emits no event". GLM's report says D4 is done.
**Evidence**: built module. Series `0x1_0000_0005` and series 7 were both cached. `releaseSeries(0x1_0000_0005n)` returned 1 with `["cache.released"]`, and `frameResident` for series 7 was still true, so the cache was not emptied. `releaseStudy([0x1_0000_0005n, 99n])` returned 0 with no events, which the module comment says cannot happen. At a 1 MiB budget, `releaseSeries(7n)` gave `["cache.released","cache.pressure"]`.
**Scope and regression**: present on HEAD. Affects readers of the README and of the `@ocelli/core` module doc, which the generated API docs and editor hovers show. Smallest fix: say the calls release the named series and push one `cache.released` per series that held frames, then a pressure event if the band crossed. Delete the old clause rather than add a sentence beside it.

### D2, two TypeScript doc comments still describe the contracts the rulings changed

**Where**: `packages/core/src/frame-cache.ts:115` to `:119` (`showResident`), and `packages/core/src/bulk.ts:89` to `:91` (`FrameCommitMeta`).
**What**: `showResident`'s comment says it shows "the cached frame at `request.index` when it answers for `request.content` at `request.columns` by `request.rows`". After ruling 1, an entry that answers is not shown unless `index` is the selected slice. The call returns false, which a caller cannot tell from a miss. `FrameCommitMeta`'s comment says `content` "routes the commit to `commit_cached_frame`". After ruling 2, `content` without both positions is refused with `InvalidValue`. README and `use.html` do not mention the selection rule either. The `use.html` sample at index 0 works only while slice 0 is selected.
**Why it is wrong**: microscope section 3. Every factual sentence in a doc comment is checkable, and a changed provider must not leave its consumer documentation stale. The Rust doc on `Session::show_resident` and the bind's comment were both updated. The TypeScript surface that callers read was not.
**Evidence**: built module. With slice 0 selected and index 1 cached under the same content and extent, `frameResident` was true and `showResident` was false, so the documented condition held and nothing was shown. `writeFrame` with `content: 7n` and no positions threw `InvalidValue` and called no export.
**Scope and regression**: present on HEAD. Affects every shell author who reads the `@ocelli/core` docs, and the folder viewer once it calls `showResident`. Fix: one clause in each comment ("the selected slice's", "with both `stackIndex` and `stackEpoch`"), and one clause in README and `use.html` 5.5 saying a show answers only for the selected slice.

## Smells

### S1, nothing watches the rule that displaced and released frames are not counted

**Where**: `crates/ocelli-wasm/src/session.rs:617`, the `Displaced | Released => {}` arm of `note_counters`.
**What**: M4 counted every `Displaced` as an eviction, and M5 counted every `Released` as one. All 19 `session_cache` tests passed under both. The comment above the arm says neither is counted, and the rule matches `TierCounters::record`, which ignores `displaced`. The lead asked for exactly this check. The probe shows the code is right today, at offset 24 = 0 after a displace and after a release.
**Hazard**: a refactor to a catch-all arm (`_ =>` beside `Refused`) inflates `evicted` on every repeat commit and every release, and stays green. A pressure display would then report evictions that never happened, which is D3 in the opposite direction.
**Regression**: in the D3 test or a sibling, a repeat commit then `request_frame_pressure()` asserting offset 24 = 0, and a `release_series` then a reading asserting 0. The probe sequence `zz_probe_d3` above is that test.

## Nitpicks

- N1. `counters.evicted_bytes` is gathered and then dropped, because the 32-byte pressure payload (`telemetry.rs:148` to `:155`) carries no `evicted_bytes`. M12 is green for that reason. F-034's layout, not F-231's.
- N2. In `bulk.test.ts:932`, the stackIndex-only and stackEpoch-only cases are refused by `u32(undefined)` before the new guard matters (T2 is equivalent). Only the no-position case watches the guard, and the test comment does not say so.
- N3. `show_resident` checks the epoch before the selection. So an unselected index under a stale epoch answers `Ok(0)`, and an index out of range answers `Err(Selection)`, not the miss that pass 1's accepted decision 8 describes for `frame_resident`. The bind returns 0 for all three.
- N4. Natively the ruling is met: the refusal is `commit_selected_frame`'s `Selection`. In TypeScript the two differ. `writeFrame` throws `CommitRefused` on that refusal, and `showResident` returns false. A lead call on whether `showResident` should throw too. Not required by the ruling as written.

## Verified clean

- **Pass 1 D1** is closed in code, natively and on the built module, including the adjacent cases (repeat, stale at the unselected index, wrong epoch, selected show) and `commit_selected_frame` parity (M1, M11, M13, M14).
- **Pass 1 D2** is closed in code. The refusal comes before the allocation, and every bad `content` value is refused (T1, T3 to T6).
- **Pass 1 D3** is closed. The counts are real, reset at each pushed reading, saturate, include budget-shrink evictions as `TierCounters::record` does, and leave out displaced, released and stale removals (M2, M3, M6 to M8, M10). S1 above is the missing watch on the exclusion.
- **Pass 1 D4**: the `use.html` import list is fixed. All six named imports are exported from `index.ts`, and the sample ran on the built module and returned `showResident` true. The release half is D1 above.
- **Pass 1 S1** is closed (M9). **S2** is closed (T3 to T6). **S3** is closed: `cache.md:24` and `:226` name `HeldValue<V>`, which matches `image.rs:545` and `:563`, and no `HeldFrame` remains in the tree.
- **Pass 1 N1** is fixed. N2 and N3 stand as nitpicks.
- **Doc sentences executed.** README 309 to 312 (content with both positions is cached, and without both is refused with `InvalidValue` and commits nothing), `use.html` 391 (the same two sentences) and the `use.html` 5.5 sample all hold on the built module. README 313 to 316 is D1.
- **Boundary.** No new `as` cast, `#[allow]` or `unsafe`. `u64::try_from(bytes).unwrap_or(u64::MAX)` matches `record`. No new export. `show_resident` still returns `u32`. No pixels cross. The bindgen script passes.
- **Size.** +374 bytes on pass 1's figure, inside the ceiling. No `--accept-size` run.
- **AC5 and AC13 as corrected** assert the ruled behaviour. The test helpers `scroll`, `commit_at` and `cached_session` are unchanged.

## Limitations and handoff

- No GPU, no oracle and no browser worker. The boundary runs use the dev module under Node with canvas stubs.
- Saturation at `u32::MAX` was checked by reading the code against `record`. It was not driven.
- The scratch copy `target/rv231p2` and its target directory were deleted. M14's mutated module was built only inside the scratch copy. The worktree's dev module and release `pkg/` were rebuilt from HEAD and are ignored output. After cleanup, `git status --short` showed only pass 1's untracked report, and `git write-tree` was `930713451c21d3d0860a5592cdb494f24f672a97`, unchanged. Nothing tracked was edited.
- Next owner: implementation fixes D1, D2 and S1. N4 is a lead decision. Pass 3 then reviews the changed material.
