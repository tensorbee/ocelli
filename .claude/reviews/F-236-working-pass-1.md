# F-236 review, pass 1

**Reviewed**: `282b9e82..549427ef` on `work/f-236-xai` in `/Users/atulsharma/Documents/projects/tensorbee/ocelli-wt-f-236`, one commit, three files (`packages/core/src/bulk.ts`, `bulk.test.ts`, `boundary.wasm.test.ts`). The working tree was clean apart from the untracked `.venv` and `tools/oracle/out`, which are outside the change.
**Author / reviewer**: implemented by GLM in opencode with three self-review cycles (`.claude/scratch/F-236-self-review-1.md` to `-3.md`). Reviewed by Claude Opus 5.5 as an independent `/microscope --working` pass. The reviewer did not author the change or its self-reviews.
**Result**: 3 defects, 1 smell, 0 nitpicks

## Acceptance challenged

- **AC1, AC5, AC7, routing through the real module.** Independently probed on the dev build. A transferred frame with both stack keys at the current epoch and index commits with `bulk_ticket()` 0. A frame with both keys and `content` commits and `frameResident` for that index, epoch, content and 1 by 1 extent answers true, and false for a different content id. The wrong implementations rejected are the base source (red on AC5 and AC7) and each single dropped or misnamed key (red, table below).
- **AC2, stale selection.** On the built module a stale epoch and a different index each throw `CommitRefused` with `bulk_ticket()` 0 on the selected route, and also on the cached route, where nothing becomes resident. A reinstall of identical keys moves the epoch from 1 to 2 and the old epoch is refused, which is the plan's named departure from cornerstone3D. A frame with no keys into a stacked session is `CommitRefused` with ticket 0, unchanged from base.
- **AC3 and amendment 6, refusal order on the built module.** Unknown key plus wrong length plus lone key gives `UnknownOption`. Wrong length plus lone key gives `LengthMismatch`. A lone `stackIndex` gives `InvalidValue` "stackEpoch is not a u32", a lone `stackEpoch` gives "stackIndex is not a u32", `content` alone or with one key gives "content needs both stackIndex and stackEpoch". A lone key with a length over the 64 MiB bulk limit gives `InvalidValue`, not `AllocationRefused`, so metadata precedes `alloc`. The same oversize length with both keys gives `AllocationRefused`. `bulk_ticket()` is 0 after every one of these, and a current transfer then commits.
- **Amendment 9, the thrown `ExportUnavailable` released through the TS path.** Not covered by any test. See D1.
- **AC9, one reservation and one copy.** The copy half is not observed. See D3.
- **AC6, old-source RED.** Re-run independently with `git show 282b9e82:packages/core/src/bulk.ts` in the scratch copy: four new unit cases and both new boundary cases red, AC3 green as expected because the lone-key refusal predates the story.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `wasm-pack build crates/ocelli-wasm --dev --target web --out-dir target/boundary-wasm` | tip, no Rust in the range | now | 0 |
| `npm run lint` | tip | now | 0 |
| `npm run typecheck` | tip | now | 0 |
| `npm test` | tip | now | 0, 61 files, 512 tests |
| `npx vitest run --config vitest.boundary.config.ts` | tip, dev build above | now | 0, 19 tests |
| `bin/ocelli.sh gate packages` | tip | now | 0, ALL GREEN |
| `bin/ocelli.sh gate wasm` | tip | now | 0, pkg 2,202,907 bytes, pkg-decode 16,455 bytes, both within budget, matching the self-review's base figures |
| Old-source RED | base `bulk.ts` with tip tests, scratch copy | now | unit 4 red (AC1, AC2, AC9, AC7), boundary 2 red (AC5, AC7) |
| Mutation catalogue, 16 mutants | scratch copy of `packages/core` at `.../scratchpad/f236-p1`, restored after each and diffed against the worktree | now | table under "Verified clean", plus D1 and D3 |
| Built-module probes P1 to P3 | scratch copy of `boundary.wasm.test.ts` with an appended describe, dev build above, restored after | now | all observations quoted under "Acceptance challenged" |
| Probe for D1 | scratch `probe.test.ts`, tip and tip with line 460 deleted | now | tip green, mutant red with `released` `[]` and ticket 21 outstanding |
| cornerstone3D 5.8.2 source | `/Users/atulsharma/Documents/projects/tensorbee/ocelli/tools/oracle/node_modules/@cornerstonejs/core`, `package.json` version 5.8.2 | now | citations hold, see "Verified clean" |

Two core unit cases (`events.test.ts` F-103 catalogue and `matrix.test.ts` browser matrix) and the `playbook.test.ts` file fail in the scratch copy on the unmutated tip as well, because they read repository files relative to the root. They are environment results of the scratch layout, green in the worktree, and were subtracted from every mutant's count.

## Defects

### D1, the transferred route's release after a thrown `ExportUnavailable` is guarded by nothing

**Where**: `packages/core/src/bulk.ts:460`, the `session.releaseAllocated?.(ticket)` in `acceptTransferredFrame`'s catch, and the missing case in `packages/core/src/bulk.test.ts`.
**What**: Amendment 9 says "the state table's export-missing row asserts the mock's `release_bulk` call for exactly that case", because a thrown `ExportUnavailable` leaves `commitAllocated` before `ticket = 0` and is the only refusal the TS path releases. No test drives `acceptTransferredFrame` with stack keys into a sink lacking `commit_selected_frame` or `commit_cached_frame`. The only `ExportUnavailable` case (`errors-class.test.ts` "maps a missing selected-frame export") goes through `writeFrame` and does not assert the release.
**Why it is wrong**: the plan's state table and amendment 9 make this an acceptance observation, and HLD 17.2's slot ownership means a leaked slot makes every later `writeFrame`, `writeCommands` and `writeStack` refuse until released.
**Evidence**: deleting line 460 leaves the core unit tests and the boundary file green (only the two scratch-environment failures). A probe with a `SessionBulkExports` lacking `commit_selected_frame` passes on tip and fails on the mutant with `released` `[]` and the session ticket still 21.
**Scope and regression**: present on HEAD. Affects any caller whose module predates the selected-frame or cached-frame export. Smallest regression: one `bulk.test.ts` case per stack route through `sessionBulkSink` over a mock missing that export, asserting code `ExportUnavailable`, the message, `release_bulk` called once with the captured ticket and `bulkTicket()` 0.

### D2, the AC1 comment states a failure mode the base source does not produce

**Where**: `packages/core/src/bulk.test.ts:818` to `:820`.
**What**: "Without the forwarding the routed export would be `commit_frame` and the first expectation fails with a saw-0 count." On the base source the mock's stacked `commit_frame` answers 0, so `acceptTransferredFrame` throws `OcelliError: commit refused` before `one(mock.selected, ...)` runs. The saw-0 message never appears for this case.
**Why it is wrong**: a factual sentence that is false, microscope section 3. Self-review cycle 1 records this sentence as "observed in the old-source RED run and in mutation M1", which it was not.
**Evidence**: old-source RED run, AC1 fails with `OcelliError: commit refused`. AC2 is the case that fails with "expected one selected-frame call, saw 0".
**Scope and regression**: test prose only, present on HEAD. Smallest fix: say the call throws `CommitRefused` because the plain route refuses with a stack installed, or delete the sentence.

### D3, AC9's "one copy" is claimed and not observed, and a second copy stays green

**Where**: `packages/core/src/bulk.test.ts:936` to `:943`, the case "keeps one reservation and one copy per transferred call on every route".
**What**: the plan's "Size and speed" row says AC9 "counts both" the `alloc` and the `set`, and names "A second copy" as a rejected wrong implementation. The test counts reservations only and its comment concedes a second identical copy is not countable. Its title still says "one copy".
**Why it is wrong**: an acceptance row whose named wrong implementation passes, microscope section 2. The title is a claim the test cannot fail on.
**Evidence**: mutant `second_copy`, the `set` line in `acceptTransferredFrame` duplicated, leaves the whole core unit suite and the boundary file green.
**Scope and regression**: present on HEAD, test only. A copy is observable without a spy, because each view is built from `wasm.memory.buffer` and `acceptTransferredFrame` takes `wasm` separately from the sink. A `WasmMemory` whose `memory` getter counts reads (one per view) would go red on the mutant. Alternatively, rename the case and amend AC9 to what is observed, which is the lead's call.

## Smells

### S1, the AC5 comment claims the committed frame is kept, and the test observes the scroll index

**Where**: `packages/core/src/boundary.wasm.test.ts:290` to `:292`, with the assertions at `:310` and `:337`.
**What**: the comment says "the committed frame at slice 0 is kept", and the plan's state table makes "the previous committed frame stays" the preserved state for a stale refusal. The test asserts `readViewportState(...).sliceIndex` is 0, which is the selection, not the committed frame, and no refusal path in the change could alter the selection.
**Failure mechanism**: a session change that clears or replaces the displayed frame inside a refused `commit_selected_frame` would leave this test green while its comment reads as coverage. The Rust test `configured_stack_rejects_neighbours_and_old_epochs_without_publishing` (`crates/ocelli-wasm/src/session.rs:2488`) watches the session side, so the behaviour is guarded natively, but this built-module case is the one the plan names for AC5. Microscope "for visible state, inspect the sink after refusal".
**Smallest regression**: bind the stub CPU canvas the file already installs, publish after the first commit and after the refusals, and compare the pixels, or reduce the comment to what is asserted.

## Nitpicks

None.

## Verified clean

- **The change itself.** `committed = { handoff: meta, ...fields }` at `bulk.ts:443` forwards exactly what `frameCommitMeta` returned. `commitAllocated` (`bulk.ts:285` to `:322`) re-validates and routes to `commit_cached_frame`, `commit_selected_frame` or `commit_frame` unchanged. A target without keys produces the same five fields as before, which `bulk.test.ts` "finishes the copy before commit_frame" still pins. No Rust file, no `options.ts` change, no new view over wasm memory outside `bulk.ts`, no wasm-bindgen, no tier path. Tiers A, B and C are n/a as the plan says.
- **Mutations that went red**, each in the scratch copy: drop `stackIndex`, drop `stackEpoch` (4 unit, 2 boundary red each), drop `content` (2 unit, 1 boundary), drop `viewport`, `columns`, `rows` or `generation` (5 unit, 3 boundary each), drop `handoff` (1 unit), forward `stackIndex` as `index` or `stackEpoch` as `epoch` (4 unit, 2 boundary), forward `content` as `contentId` (2 unit, 1 boundary), swap index and epoch (4 unit, 2 boundary), skip `ticket = 0` in `commitAllocated` (2 unit), remove the `CommitRefused` throw on code 0 (2 unit, 1 boundary). Survivors are D1 and D3.
- **The real exports.** The mock's argument order for `commit_selected_frame` and `commit_cached_frame` agrees with the built glue, proved by AC5 and AC7 committing on the dev build and by the frame becoming resident only for the forwarded content id.
- **Ticket after refusal.** `session.rs:217` to `:220` and `:238` to `:241` release on `Selection`, `commit_cached_frame` delegates to `commit_selected_frame` before caching (`:375` to `:389`), and every built-module refusal probed left `bulk_ticket()` 0.
- **cornerstone3D floor rows**, against the pinned 5.8.2 dist: `StackViewport.js:1417` to `:1419` is the CPU `successCallback` returning when `currentImageIdIndex !== imageIdIndex`, `:1509` to `:1513` is the GPU `successCallback` with `this.imageIds.indexOf(imageId)` at `:1510`, `loaders/imageLoader.js:84` is `cache.putImageLoadObject(imageId, imageLoadObject)` and `cache/cache.js:395` is `putImageLoadObject`. The late-frame row holds on the built module (stale index refused, ticket 0). The kept-frame half of that row is S1.
- **Sentences added by the diff, executed.** `bulk.ts:419` to `:422` (forwarded fields select both routes as `writeFrame`'s meta does, read at `bulk.ts:205` to `:236` and driven by AC1, AC7 and the probes). `transferredMock` doc (selection refusal releases inside the export, true per `session.rs`). AC2 comment (the refusal is the selected export's own 0, confirmed by base RED "saw 0"). AC3 comment (codes, messages and order, confirmed by P2). AC7 unit comment (dropping `content` commits through `commit_selected_frame`, confirmed by mutant). Boundary AC7 comment (budget and four-byte multiple, read at `session.rs:394` to `:396`). `codeOf` docs. False or unobserved: D2, D3's title, S1.
- **Prose rules.** No em-dash, en-dash or semicolon in any added comment line, scanned over the diff.
- **User documents.** `README.md`, `use.html` and `more.html` contain no `acceptTransferredFrame`, by `grep -c`, so nothing there went stale.

## Self-review catch figure

All 4 blocking findings should have been caught by GLM's three self-review notes. D1: cycle 2 claims the AC inventory was "re-checked row by row against the plan, including the amendments", and amendment 9's row has no test, and "skip the ticket release" is a first-order mutation the catalogue omitted. D2: cycle 1 lists this exact sentence as executed against the RED run. D3: cycle 1 saw the limitation and kept it rather than raising the unmet AC9 row. S1: cycle 1 lists the boundary AC5 comment as "executed by the test's assertions". Figure, 4 of 4.

## Limitations and handoff

- Not exercised: a browser worker, a release build in a page, a GPU adapter. The built-module evidence is the dev build under Node, as the test header says.
- `docs/lld/typescript-packaging.md:117` to `:119` still says the order ends in `commit_frame`, which is untrue for a stack target on HEAD. The plan assigns that edit and `CHANGELOG.md` to completion, so it is carried as a completion obligation, not a finding of this working pass.
- Next owner: the implementer remediates D1, D2, D3 and S1, then a fresh independent pass reviews the changed material. Scratch material lives under `/private/tmp/claude-501/-Users-atulsharma-Documents-projects-tensorbee-ocelli/7387c89c-42ee-4e98-93b8-dbe52072290d/scratchpad/` and is deleted with Python `shutil` at the end of this pass.
