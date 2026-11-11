# F-231 review, pass 3

**Reviewed**: `git diff 8daf54c9 fc82f385`, the pass 2 remediation, read against the lead rulings in `.claude/plans/F-231-design.md`, the newest being "`showResident` throws the coded `CommitRefused` error for a `Selection` refusal ... and returns false only for a miss". HEAD `fc82f3850bd9fbe9798d02f5038d135b005505b6`, tree `7f19c7bced460e447067e0c0b57716fc8e2f5e3e`, worktree `ocelli-wt-f-231` on `work/f-231-xai`. Untracked before this report were only the pass 1 and pass 2 reports.
**Author / reviewer**: GLM-5.3 in opencode wrote the remediation. Its log says no commit, so `fc82f385` was committed by the lead. The reviewer is an independent Claude agent that did not write the remediation, the plan, the rulings or passes 1 and 2.
**Result**: 3 defects, 0 smells, 2 nitpicks

## Acceptance challenged

- **The ruling after pass 2, refusal against miss, on the built dev module** (rebuilt from HEAD before the runs). Stack of three under series `0x1_0000_0005`, budget 1024, index 0 committed, scroll to 1, index 1 committed:
  - `showResident(index 0)` with the matching content threw `OcelliError` `CommitRefused`. `sliceIndex` was 1 and flags 0 before and after, with no ring event, and `frameResident(index 0)` was still true. So the refusal drew nothing and removed nothing.
  - A mismatch at the selected index 1 returned false with no throw, and `frameResident(index 1)` was then false, which is the stale removal.
  - An empty slot at the selected index 2 returned false.
  - Index 99 (out of range) threw `CommitRefused`.
  - An unselected index under a wrong epoch returned false. That is pass 2's N3 ordering, see N1 below.
  - The follow-up check: back at slice 0, `showResident(index 0)` returned true and cleared the loading flag. A `writeFrame` after the refusal committed and gave one `cache.displaced`, so the refusal left no hidden state.
- **Release of one series with two cached.** Keys `A/0`, `A/1`, `7/0`, all three committed. `releaseSeries(A)` returned 2 and gave one event, `cache.released` with frames 2 and bytes 8. Series 7 stayed resident, `A/0` did not, and the display flags stayed 0.
- **Empty release.** A repeat `releaseSeries(A)` returned 0 with no events. `releaseStudy([A, 99n, A])` returned 0 with no events, and `releaseStudy([])` returned 0. The next requested reading showed entries 1 and used 4.
- **A release that crosses a band.** Budget 8, two 4-byte frames (Critical). `releaseSeries(A)` gave `["cache.released","cache.pressure"]` in that order, and the pressure had used 0, evicted 0 and refused 0. After an eviction walk on three frames, the release's crossing reading carried `evicted 1`. That is the earlier eviction, which is correct for "since the previous reading", and the next reading was 0.
- **Every route that evicts or refuses feeds the counters.** `self.frames` is mutated in exactly four places in `session.rs`. `set_budget` (`:340`) and `insert` (`:402`) are folded. `release_series` (`:524`) is not folded, but `ImageCache::release_series` (`image.rs:627`) can return only `Released`, which is never counted. A stale removal in `lookup` (`image.rs:686`) emits no event. `configure_stack` and every other route never touch `frames`, and `ImageCache::remove` is not called by the session. So nothing evicts or refuses unfolded and no reading under-reports. M7 and M8 confirm both folds are watched. R6 on the built module: an 8-byte commit at budget 4 gave `cache.refused` and the next reading had `refused 1`.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `cargo fmt --all --check` | HEAD, `CARGO_TARGET_DIR=<wt>/target` | now | 0 |
| `cargo clippy --workspace --all-targets -- -D warnings` | HEAD | now | 0 |
| `cargo test -p ocelli-cache -p ocelli-viewport -p ocelli-wasm` | HEAD | now | 0, `ocelli_wasm` lib 51 passed |
| `bin/ocelli.sh wasm`, `pin_and_size_check.py --with-size` | HEAD release | now | 0, 0. 2,202,813 bytes, ceiling 2,258,755 |
| `ci/check-bindgen-isolation.sh` | HEAD | now | 0, confined to `ocelli-wasm` |
| `wasm-pack build crates/ocelli-wasm --dev --target web --out-dir target/boundary-wasm` | HEAD | now | 0 |
| `npx vitest run --config vitest.boundary.config.ts` | HEAD dev module | now | 0, 17 passed |
| `npx vitest run packages/core` | HEAD | now | 0, 458 passed in 49 files |
| `npm run typecheck`, `npm run lint` | HEAD | now | 0, 0 |
| `event_catalogue_check.py` | HEAD | now | 0, 50 core and 53 tool names |
| `guard_census.py`, `--check-runbook` | HEAD | now | 0, 0, watched by nothing 0 |
| `prose_check.py`, `source_provenance_check.py` | HEAD | now | 0, 0 |
| Replays R1 to R6 | built dev module through the public `@ocelli/core` helpers, a probe under ignored `target/rv231p3` | now | as in "Acceptance challenged" |
| Which refusal fires for content with a lone position | stub sink, public `writeFrame` | now | all three cases: `InvalidValue` "content needs both stackIndex and stackEpoch" |
| M1, `Released` counted in `note_counters` only | scratch copy, own target dir, `--no-fail-fast` | now | **green, 51 passed**. See D2 |
| M2, `Displaced` counted | scratch | now | red, `displaced_and_released_frames_are_not_counted_as_evictions` |
| M3, bind maps `Selection` to 0 | scratch, native and rebuilt dev module | now | native green (the bind is not natively tested). Boundary red, 2 failed, both ruling tests |
| M4, TS throw guard `answer === 3` | scratch | now | red, the stub test `turns an unselected-index refusal into the coded CommitRefused` |
| M5, bind answers 2 for every `Ok(0)` | scratch, rebuilt dev module | now | red, 2 failed |
| M6, `release_series` folded and `Released` counted | scratch | now | red, the S1 test |
| M7, `insert` fold removed | scratch | now | red, 3 tests |
| M8, `set_budget` fold removed | scratch | now | red, `a_budget_change_that_crosses_a_band_reports_the_pressure_last` |
| M9, the content guard at `bulk.ts:499` disabled | scratch | now | red at `bulk.test.ts:963`, the no-position case |

## Defects

### D1, the `frame-cache.ts` module doc still says every function refuses before any export call

**Where**: `packages/core/src/frame-cache.ts:15`.
**What**: the module comment says "Every function here refuses before any export call." Since this diff, `showResident` calls `show_resident` first and then throws `CommitRefused` on the answer 2 (`:136` to `:144`). So one refusal in this file comes after the export call.
**Why it is wrong**: microscope section 3. A changed provider must not leave its own module documentation stale, and the brief asked for every sentence in this file to be checked. The sentence was true when it was written and this diff made it false. It sits in the `@ocelli/core` module doc that editor hovers and generated API docs show.
**Evidence**: R1 on the built module. The unselected show threw `CommitRefused`, and it can only do that from the value the export returned. Mutation M4 confirms the throw depends on the export's answer.
**Scope and regression**: present on HEAD. Affects readers of the module doc. Smallest fix: limit the sentence to the validation refusals ("Every argument is validated before any export call"), or name `showResident`'s `CommitRefused` as the one refusal that comes from the session's answer. Prefer rewording the existing sentence to adding a new one.

### D2, the S1 test claims a mutation goes red that stays green

**Where**: `crates/ocelli-wasm/src/session.rs:3448` to `:3449`, the comment on `displaced_and_released_frames_are_not_counted_as_evictions`, and the release half at `:3518` to `:3525`.
**What**: the comment says "Counting a Displaced or a Released event as an eviction in note_counters goes red here." Counting `Released` in `note_counters` stays green (M1, 51 passed), because `release_series` (`:523`) never calls `note_counters`. The release assertions catch only the two-part change in M6, which both folds the release and counts it. GLM's own log says the pure mutation cannot be observed, but the sentence that went into the tree says the opposite.
**Why it is wrong**: microscope section 2 ("A test that passes both ways ... will be counted as coverage forever") and section 3. A later reader will trust this comment and believe a catch-all arm is watched on the release path. It is not watched there, and today it does not need to be. Pass 2's S1 hazard sentence ("inflates `evicted` on ... every release") was itself wrong for the same reason: the release route is not folded.
**Evidence**: M1 green, M6 red, both in a scratch copy with its own target directory and `--no-fail-fast`.
**Scope and regression**: present on HEAD, a test comment only. The behaviour is correct (R4, R5). Smallest fix: say what is actually watched. The displaced half watches the arm (M2). The release half watches a release that reaches the fold, because `release_series` does not fold today.

### D3, the new `bulk.test.ts` comment has the refusal order backwards

**Where**: `packages/core/src/bulk.test.ts:937` to `:939`, added by this diff for pass 2's N2.
**What**: it says "Only the no-position case reaches that guard: a lone stackIndex or stackEpoch is refused by the u32 check first." In `frameCommitMeta` (`bulk.ts:499`) the content guard runs before any `u32` check (`:507` to `:511`). So all three cases reach the guard, and the guard refuses all three.
**Why it is wrong**: microscope section 3. Pass 2's N2 was about equivalence: with the guard gone, a lone position would still be refused by `u32(undefined)`. That is not an execution order, and the comment turned it into a false claim about the order.
**Evidence**: the stub probe through `writeFrame`. `{stackIndex: 1, content}`, `{stackEpoch: 4, content}` and `{content}` each threw `InvalidValue` with the guard's message "content needs both stackIndex and stackEpoch", not "... is not a u32". M9 (guard disabled) went red at `:963`, the no-position case, so the lone-position cases are the ones that do not depend on the guard.
**Scope and regression**: present on HEAD, a test comment only. Smallest fix: "Only the no-position case depends on that guard. Without it, a lone stackIndex or stackEpoch would still be refused by the u32 check."

## Smells

None.

## Nitpicks

- N1. Pass 2's N3 is still open and now shows in the new prose. `README.md:313` to `:314` and the `use.html` 5.5 sample comment say another index "throws the coded `CommitRefused`". Under a wrong epoch, an unselected index returns false (R1), because `show_resident` checks the epoch before the selection. `writeFrame` refuses a wrong epoch with `CommitRefused` (`commit_selected_frame`'s `Selection`). So showing and committing still differ on a stale epoch. The `frame-cache.ts` doc's "`false` is ... a wrong epoch" covers it, and README does not. A lead call, and not part of the ruling as written.
- N2. The ruling says false "only for a miss". The code and every doc say false for a miss, a stale entry or a wrong epoch. That reads as the ruling's intent, the split between a refusal and not shown, so it is noted rather than raised.
- Carried: pass 2 N1 (`evicted_bytes` is gathered and dropped by F-034's 32-byte payload) is unchanged and outside F-231.

## Verified clean

- **Pass 2 D1 is closed.** README `:315` to `:318` and the `frame-cache.ts` module doc now say the calls release the named series, one `cache.released` per series that held frames, no event for an empty series, and `cache.pressure` after `cache.released` on a crossing. R2, R3 and R4 hold each clause on the built module. "The displayed frame ... stays on screen through a release" holds (flags 0 after the release in R2).
- **Pass 2 D2 is closed.** The `showResident` and `FrameCommitMeta` comments state the selection rule and the position rule. Every clause of the `showResident` comment was executed in R1. The `FrameCommitMeta` clause was executed by the stub probe.
- **The pass 2 ruling (N4) is implemented.** The bind answers 2 only for `Err(SessionError::Selection)`, which is the only error `Session::show_resident` returns. `showResident` throws on 2 and returns `answer === 1`. M3, M4 and M5 show that both the throw and the miss halves are watched at the boundary and at stub level.
- **Pass 2 S1, the displaced half, is closed** (M2 red). The release half is D2.
- **Counter completeness**, the lead's question. Answered yes in "Acceptance challenged", M7 and M8.
- **Pass 1 closed items are still closed.** D1 (the selection refusal leaves the state, R1). D2 (content without positions is refused before allocation, by the probe and M9). D3 (real counts that reset, R5 and R6). D4 (release prose, now correct).
- **Sentences checked in full, not only the changed ones**: README `:307` to `:318`. `use.html` `:365`, `:391` and the 5.5 sample (`:392` to `:416`). Every doc comment in `frame-cache.ts`, apart from D1. The `bulk.ts` comments on `BulkSink`, `FrameCommitMeta`, `writeStack`, `writeFrame` and `sessionBulkSink`. The Rust doc comments on `Session::show_resident` and the bind. `more.html:414` names "the render session's own frame cache" as built, and that holds.
- **Boundary.** No new `as` cast, `#[allow]`, `unsafe` or export. The new `use super::SessionError` is inside the bind module. Bindgen isolation passes. +12 bytes on pass 2's figure, inside the ceiling. No `--accept-size`.
- **GLM's report against the tree.** The 51 lib tests, the 2,202,813 bytes, the 17 boundary and 458 core tests and every gate tail reproduced. The red runs it claims are consistent with M3 and M4. Its M5 note is accurate in the log but did not reach the test comment (D2).

## Limitations and handoff

- No GPU, no oracle and no browser worker. The boundary runs used the dev module under Node with canvas stubs.
- Saturation at `u32::MAX` was not driven.
- The scratch copy (under the session scratchpad, with its own target directory) and the probe directory `target/rv231p3` were deleted. The worktree's dev module and release `pkg/` were rebuilt from HEAD and are ignored output. After cleanup, `git status --short` showed only the pass 1 and pass 2 reports, and `git write-tree` was `7f19c7bced460e447067e0c0b57716fc8e2f5e3e`, equal to `HEAD^{tree}`. Nothing tracked was edited.
- Next owner: implementation fixes D1 to D3, which are three comment sentences with no behaviour change. N1 is a lead decision. Pass 4 reviews the changed material.
