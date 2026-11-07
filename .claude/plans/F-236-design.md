# F-236, acceptTransferredFrame honours stackIndex and stackEpoch

**Status**: approved
**Epic ref**: E16.7
**Sprint**: S47
**Estimate**: 1w

Story row, `docs/sprints/BACKLOG.md`: "acceptTransferredFrame honours stackIndex and stackEpoch: a transferred frame carrying both keys commits through the selected-frame export and is refused for a stale index or epoch, with tests at the unit and built wasm levels". Layer TS, depends on F-231, milestone M20.

`allocation.json` notes: "Found by F-230's first review. Answered by the operator on 2026-11-06: a story in S47 honours both keys with tests."

## Normative source, transcribed

### HLD 17.2, as `packages/core/src/bulk.ts` and `docs/lld/typescript-packaging.md` state it

`docs/lld/typescript-packaging.md` lines 117 to 121: "`acceptTransferredFrame` is the render-worker side of the same hop. It uses `writeFrame`'s order, which is `alloc`, then a view, then `set`, then `commit_frame`." The view over wasm memory is built after `alloc`, used once and dropped, which is HLD 17.2's rule.

### The operator's answer, `docs/sprints/CURRENT_SPRINT.md`

"Found by F-230's first review and older than F-230. Now that both keys are on F-230's accepted list, a caller who sets them gets neither a refusal nor the effect. ... **Answered by the operator on 2026-11-06:** a story in S47 honours both keys with tests."

### The finding, `.claude/reviews/F-230-working-pass-1.md` line 102

"`acceptTransferredFrame` builds `committed` without `stackIndex` or `stackEpoch` (base `bulk.ts:389-395`). F-230's key list now declares those two keys accepted there, and a correctly spelt selection is still committed down the generation path. The probe printed the committed keys as `handoff, viewport, columns, rows, generation`."

## Facts measured for this plan

Read on 2026-11-06 on `sprint/s46` at `1ba258e`.

1. `FrameCommitMeta` (`bulk.ts` 89 to 96) declares `viewport`, `columns`, `rows`, `generation` and optional `stackIndex` and `stackEpoch`. `FRAME_COMMIT_META_KEYS` in `options.ts` accepts all six.
2. `acceptTransferredFrame` (`bulk.ts:389`) validates through `frameCommitMeta(target)` and then builds `committed` from `handoff`, `viewport`, `columns`, `rows` and `generation` only (lines 408 to 414).
3. `frameCommitMeta` (`bulk.ts:449`) validates both stack keys as u32 when either is present, so a lone key is already refused.
4. `sessionBulkSink.commitAllocated` (`bulk.ts` 270 to 289) calls `wasm.commit_selected_frame(presented, len, viewport, columns, rows, stackIndex, stackEpoch)` when both keys are present, throws `OcelliError("ExportUnavailable", "selected-frame export is unavailable")` when that export is missing, and otherwise calls `commit_frame`.
5. In `crates/ocelli-wasm/src/session.rs`, `commit_frame` (185) refuses with `SessionError::Selection` whenever a stack is installed, and `commit_selected_frame` (205) refuses unless `epoch != 0`, `epoch == self.stack_epoch` and the viewport's scroll index equals `index`. So today a transferred frame into a session with an installed stack is always refused, with or without the keys.
6. Tests of `acceptTransferredFrame` in `bulk.test.ts`, `errors-class.test.ts`, `boundary.wasm.test.ts`, `options.test.ts` and `index.test.ts` pass no stack key. The stack-key tests use `writeFrame`, for example `boundary.wasm.test.ts:197` "installs ordered keys and refuses stale selection through the public bulk adapter".

## What the specification does not cover

1. Whether the fix forwards the keys into `committed` so the existing sink path selects the export, or calls the selected-frame export directly. This draft proposes forwarding, so there is one branch in `commitAllocated`.
2. F-231 changes the session's commit path for its frame cache. This draft is written against `sprint/s46` and is re-read against F-231's merged tree at approval.

## Problem and reproducer

A render worker calls `acceptTransferredFrame(wasm, session, buffer, meta, { viewport, columns, rows, generation, stackIndex, stackEpoch })` on a session with an installed stack. Today the call throws `CommitRefused`, because the keys are dropped and `commit_frame` refuses with a stack installed (facts 2 and 5). Expected: the frame commits when index and epoch are current, and `CommitRefused` when either is stale.

## Scope, API and compatibility

In scope: `acceptTransferredFrame` in `packages/core/src/bulk.ts` and its tests. Non-goals: `writeFrame`, which already honours the keys. The wasm session, unless F-231's tree requires a change.

No signature change. A caller who never sets the keys sees today's behaviour.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| `acceptTransferredFrame` with both keys | `commitAllocated` then `commit_selected_frame` | render worker, transferred buffer | commit, or `CommitRefused` for a stale selection |
| `acceptTransferredFrame` without the keys | `commit_frame` | no stack installed | unchanged |
| `writeFrame` | unchanged | | unchanged |

## State transitions and invariants

| Entry point / starting state | Input or event | Resulting state and output | Refusal and preserved state |
|------------------------------|----------------|----------------------------|-----------------------------|
| stack installed, epoch E, index I | keys `I`, `E` | frame committed, ticket consumed | none |
| same | keys `I`, `E - 1`, or a different index | `CommitRefused`, allocation released | the previous committed frame stays |
| no stack | no keys | `commit_frame` as today | as today |
| any | one key only | `InvalidOptions` before allocation (fact 3) | nothing allocated |
| export missing | both keys | `ExportUnavailable`, allocation released | nothing committed |

## Ownership, commit and recovery

The allocation is staged by `session.alloc`, the commit point is the export's success, and every refusal releases the allocation through the existing release-and-rethrow path. A repeated call with the same buffer after a stale refusal allocates again.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | Both keys current. The mock sink receives `commit_selected_frame` with the six numbers, after the copy | `bulk.test.ts`, "forwards stackIndex and stackEpoch to the selected-frame export" | Today's code, which calls `commit_frame` | Implementation |
| AC2 | A stale epoch and a stale index each throw `CommitRefused` and release the allocation | `bulk.test.ts` | Swallowing the refusal | Implementation |
| AC3 | A lone key throws `InvalidOptions` before `alloc` | `bulk.test.ts` | Ignoring a lone key | Implementation |
| AC4 | No keys, unchanged. `commit_frame` with the generation | existing tests | A regression on the generation path | Implementation |
| AC5 | Built wasm. Against the real module, a transferred frame with the epoch `writeStack` returned and the current index commits, and a stale epoch is refused | `boundary.wasm.test.ts` | A mock-only proof | Implementation |
| AC6 | AC1 and AC5 fail on today's code | old-source RED recorded | Tests that pass before the fix | Implementation, recorded |

## Approach

Copy `stackIndex` and `stackEpoch` into `committed` when `frameCommitMeta` returned them, and let `commitAllocated` choose the export as it already does.

## Dependencies, write ownership and resources

Declared: F-231, which writes `packages/core` and `crates/ocelli-wasm/src/session.rs`. Writes `packages/core/src/bulk.ts`, `bulk.test.ts` and `boundary.wasm.test.ts`. The built-module test needs `bin/ocelli.sh wasm`.

## Verification and evidence reuse

```bash
npm run test
bin/ocelli.sh wasm && npx vitest run --config vitest.boundary.config.ts
```

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no. The transferred buffer is copied into linear memory by the existing path, which is HLD 17.2's raw-bytes channel
- Render-loop allocation: none added
- unsafe: none
- Tier A (WebGPU): n/a, the commit is tier independent
- Tier B (WebGL2): n/a, the commit is tier independent
- Tier C (CPU): n/a, the commit is tier independent

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| unit | AC1 to AC4 | `packages/core/src/bulk.test.ts` |
| integration | AC5 against the built module | `packages/core/src/boundary.wasm.test.ts` |

## Parity surface covered

None.

## Deviations

None.

## LLD impact

`docs/lld/typescript-packaging.md` lines 117 to 121 and `docs/lld/public-api.md` line 82, the selected-frame route of `acceptTransferredFrame`.

## User-facing documentation impact

None expected. `README.md`, `use.html` and `more.html` do not mention `acceptTransferredFrame`, measured by grep. Rechecked at approval against F-231's tree.

## Open questions

- **Q1, forward the keys or call the export directly.** Affects AC1 and AC5. Proposed: forward, decision 1. Owner: the lead in the S47 design round. **Answered by the lead on 2026-11-07, forward, under "Lead decisions, pulled forward into S46" below.**
- **Q2, F-231's merged commit path.** Affects every row. The plan is re-read against it before approval. Owner: the lead. **Answered on 2026-11-07 by the re-read in "Lead amendments before implementation, against the sprint head after F-231" below.**

None remain open.

## Lead amendments before implementation, against the sprint head after F-231

Re-read by the lead on 2026-11-07 on `sprint/s46` at `eb0f81b3`, after F-231 integrated. Each amendment supersedes the sentence of this plan it names, and where an amendment and an earlier section disagree the amendment wins.

1. `FrameCommitMeta` is now `packages/core/src/bulk.ts:95` to `:103` and carries a seventh optional field, `content?: bigint`, which F-231 added and whose doc comment at `bulk.ts:89` to `:94` routes it to `commit_cached_frame`, so fact 1's "89 to 96" and its six fields are superseded.
2. `FRAME_COMMIT_META_KEYS` at `packages/core/src/options.ts:126` to `:134` accepts seven keys, the six of fact 1 plus `content`, so a transferred frame with `content` is today accepted and silently dropped, the same defect as the two stack keys.
3. `acceptTransferredFrame` is at `bulk.ts:421`, calls `frameCommitMeta(target)` at `bulk.ts:439` and builds `committed` at `bulk.ts:440` to `:446` from `handoff`, `viewport`, `columns`, `rows` and `generation` only, which replaces fact 2's `bulk.ts:389` and lines 408 to 414.
4. `frameCommitMeta` is at `bulk.ts:481`, not `:449`, and a lone stack key is refused by `u32` at `bulk.ts:511` and `:516` to `:526` with code `InvalidValue` and the message `stackIndex is not a u32` or `stackEpoch is not a u32`, so the `InvalidOptions` of fact 3, the state table's one-key row and AC3 become `InvalidValue`.
5. `content` without both stack keys is refused with `InvalidValue` and the message `content needs both stackIndex and stackEpoch` at `bulk.ts:497` to `:504`, before allocation, and AC3 asserts that refusal beside the lone-key one.
6. The refusal order in `acceptTransferredFrame` is `refuseUnknownOptions` at `bulk.ts:432`, then the length check at `bulk.ts:433` to `:438`, then `frameCommitMeta` at `bulk.ts:439`, then `alloc` at `bulk.ts:448`, so a lone key with a mismatched length throws `LengthMismatch` and AC3 uses a buffer whose length matches `meta.bytes`.
7. `sessionBulkSink.commitAllocated` is at `bulk.ts:285` to `:322`, not 270 to 289, and has three routes after F-231: `commit_cached_frame` when both stack keys and `content` are present (`bulk.ts:292` to `:305`, `ExportUnavailable` "cached-frame export is unavailable" at `:294`), `commit_selected_frame` with both keys and no `content` (`bulk.ts:307` to `:310`, "selected-frame export is unavailable" at `:308`), and `commit_frame` otherwise (`bulk.ts:312` to `:319`).
8. `commitAllocated` clears its ticket at `bulk.ts:320` after any returned code, so `releaseAllocated` at `bulk.ts:323` to `:329` is a no-op after a returned refusal and the slot is released by the Rust session itself (`crates/ocelli-wasm/src/session.rs:218` for `commit_frame` and `:240` for `commit_selected_frame`), which replaces "every refusal releases the allocation through the existing release-and-rethrow path" in "Ownership, commit and recovery" and makes AC2 and AC5 assert `bulk_ticket()` is 0 after each refusal.
9. Only a thrown `ExportUnavailable`, which leaves before `bulk.ts:320`, is released through the TS path at `bulk.ts:323` to `:329`, and the state table's export-missing row asserts the mock's `release_bulk` call for exactly that case.
10. In `session.rs`, `commit_frame` is at `:207` and refuses with `SessionError::Selection` when the viewport's scroll is not empty (`session.rs:217` to `:220`), and `commit_selected_frame` is at `:227` with its epoch and index check at `:238`, which replaces fact 5's `185` and `205`.
11. The `commit_selected_frame` and `commit_cached_frame` exports build `FrameMeta` with `generation: 0` (`session.rs:1439` to `:1462` and `:1474` to `:1498`), so a transferred frame's `generation` is validated and then has no effect on either stack route, as it already has none for `writeFrame`, and no test asserts a forwarded generation on those routes.
12. `commit_cached_frame` at `session.rs:375` commits through `commit_selected_frame` and then caches only when the committed length is a multiple of four and the budget is nonzero (`session.rs:394` to `:396`), so AC7 below sets a budget through `setFrameBudget` and uses a four-byte-multiple frame.
13. The stack test in `packages/core/src/boundary.wasm.test.ts` is at `:243`, not `:197`, the existing transferred-frame call is at `:501`, and F-231's cache test is at `:731`, which AC5 and AC7 extend beside.
14. The built-module test reads the dev build at `crates/ocelli-wasm/target/boundary-wasm` (`boundary.wasm.test.ts:7` to `:9` and `:59`), so the verification block's `bin/ocelli.sh wasm` is replaced for AC5 and AC7 by `wasm-pack build crates/ocelli-wasm --dev --target web --out-dir target/boundary-wasm`, and `bin/ocelli.sh gate wasm` is kept only for AC8.
15. The existing case "finishes the copy before commit_frame" at `packages/core/src/bulk.test.ts:580` to `:600` asserts `commit.meta` equals `{ handoff: meta, ...target }` at `:599`, which stays true under forwarding because its target carries no stack key, so AC4 names it.
16. `docs/lld/typescript-packaging.md:116` to `:122` says the order ends in `commit_frame` at `:118`, which becomes untrue for a stack target, so the LLD impact is that sentence plus `docs/lld/public-api.md:82`, which is still accurate.
17. `README.md`, `examples/folder-viewer/src/use.html` and `examples/folder-viewer/src/more.html` still do not mention `acceptTransferredFrame`, re-measured by grep at `eb0f81b3`, so user-facing documentation impact stays none.
18. The finding at `.claude/reviews/F-230-working-pass-1.md:102` and the operator's answer at `docs/sprints/CURRENT_SPRINT.md:131` to `:137` are quoted correctly.
19. `acceptTransferredFrame` takes a `BulkSink` and a `WasmMemory` (`bulk.ts:421` to `:427`), not `SessionBulkExports`, so it cannot call a selected-frame export directly without a signature change, which confirms decision 1.

### Acceptance rows added by the amendments

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC7 | Both keys and `content` current. Through `sessionBulkSink` over a mock, `commit_cached_frame` receives the seven numbers and the bigint. On the built module with a budget set, the transferred frame commits and `frame_resident` for that index, epoch, content and extent answers 1 | `bulk.test.ts` and `boundary.wasm.test.ts`, "a transferred frame with content is committed and cached" | Forwarding the two stack keys and dropping `content`, which commits and leaves the cache empty | Implementation |
| AC8 | Size, under "Size and speed" below | `bin/ocelli.sh gate wasm` on base and tip | A budget edited to make room | Implementation |
| AC9 | Speed, under "Size and speed" below | `bulk.test.ts`, one `alloc` and one copy per call | A second copy or a second allocation | Implementation |

AC6's old-source RED covers AC7 as well as AC1 and AC5.

## Size and speed

By the operator's instruction of 2026-11-07, relayed by the lead, a story must not grow the wasm build beyond budget or make the system slower.

| Row | Rule | Observation | Wrong implementation rejected |
|-----|------|-------------|-------------------------------|
| Wasm size | No Rust file changes, so neither module can change. `bin/ocelli.sh gate wasm` passes on the story's base and its tip against `ci/wasm-size-budget.json` and `ci/wasm-decode-size.json`, neither budget edited and `--accept-size` never run. The byte sizes of `crates/ocelli-wasm/pkg/ocelli_wasm_bg.wasm` and `pkg-decode/ocelli_wasm_bg.wasm` are recorded on both, and any non-zero delta stops the story, because the write set has no Rust in it | Recorded in the story's handoff | A Rust change smuggled into a TS story, or a re-baselined budget |
| Package size | The byte size of `packages/core/dist/bulk.js` is recorded on base and tip. The change replaces a five-field literal with a spread, and any growth is explained line by line in the handoff | Recorded in the story's handoff | Duplicated routing logic inside `acceptTransferredFrame` |
| Speed | The transferred path keeps one `alloc` of `len` bytes and one `set` of `len` bytes per call, as today. AC9 counts both with a mock sink on the selected, cached and generation routes. No `tools/bench` subject exercises this entry point (the ids in `tools/bench/subjects.json`), so no timing is claimed and no threshold is invented | `bulk.test.ts`, AC9 | A second copy, a second allocation, or a validation pass repeated per byte |

## cornerstone3D 5.8.2 is the floor

By the operator's clarification of 2026-11-07, relayed by the lead: every behaviour cornerstone3D 5.8.2 has here, Ocelli has too, or this plan names the gap and why, and a departure is better on a stated and measured axis. Read from `tools/oracle/node_modules/@cornerstonejs/core` at 5.8.2.

| Behaviour | cornerstone3D 5.8.2 | Ocelli after F-236 | Axis and measurement |
|-----------|---------------------|--------------------|----------------------|
| A loaded frame for the current index | Drawn | Committed through `commit_selected_frame` | Same. AC1, AC5 |
| A late frame for an index no longer current | Dropped silently, `StackViewport.js:1417` to `:1419` on CPU and `:1509` to `:1513` on GPU | Refused with `CommitRefused`, the slot released and the previous frame kept | Better, the refusal is visible to the caller rather than a silent return. AC2, AC5 |
| A late frame from a replaced stack | The index is recomputed by `this.imageIds.indexOf(imageId)` at `StackViewport.js:1510` against the current stack | Refused by epoch even when the index coincides | Same outcome for a different stack. **Departure, named, not better**: a frame in flight across a reinstall of identical keys is refused where cornerstone draws it, at the cost of one recommit, because a stack position is not an image identity in this API. AC2 |
| A loaded frame kept for reuse | Cached by image id, `loaders/imageLoader.js:84` into `cache/cache.js:395` | Cached by content id when the caller passes `content` | Same. AC7 |
| Speed | Not timed on a comparable entry point, and the oracle has no transferred-frame subject | One allocation and one copy, counted | Not compared. No cornerstone number exists for this path, so none is recorded |

## Lead decisions, pulled forward into S46

The operator decided on 2026-11-07 to pull bounded S47 stories forward into S46 while implementer slots are idle. F-236 was held because it depends on F-231, and F-231 integrated at `eb0f81b3`, so it is now pulled forward. Its backlog row keeps sprint S47 until it integrates, as F-237's and F-238's do.

- **Q1, forward, decision 1.** `committed` becomes `{ handoff: meta, ...fields }`, where `fields` is what `frameCommitMeta` returned, and `commitAllocated` chooses the export as it already does. The code supports it, by amendments 7 and 19.
- **`content` is forwarded too**, by amendment 2. Forwarding only the two stack keys would leave an accepted key silently dropped, which is the defect this story exists to remove and the rule F-230 wrote at `bulk.ts:497` to `:499`. It adds AC7 and stays within the 1w estimate, because the route already exists in `commitAllocated`.
- **Write set.** `packages/core/src/bulk.ts`, `packages/core/src/bulk.test.ts`, `packages/core/src/boundary.wasm.test.ts`, and at completion `docs/lld/typescript-packaging.md` and `CHANGELOG.md`. No Rust file, no `options.ts` change. `bulk.ts` is shared with any concurrent `packages/core` story, so the story starts from the sprint head and integrates through a three-way merge.
- **Design readiness.** The normative source is transcribed, the API is unchanged, every acceptance row from AC1 to AC9 has an observation and a stage, the write set is feasible on the sprint head, and no question is open. Status is approved.

### Lead ruling after review pass 2

- **D1, AC5's pixel comparison.** After `bindCpuCanvas`, the case submits a `Fit` to the canvas size and a narrow `SetWindow` so that the committed value changes the blit. A control commit of a different value is asserted to change the pixels, so the comparison is shown able to fail. The refusal case must then go red at the pixel comparison under a mutant whose refusal branch commits the refused frame anyway.
- **D2, AC9's one copy.** AC9 keeps its wording. The test counts copies, not views: it spies on `Uint8Array.prototype.set` for calls whose target buffer is the wasm memory's buffer, and on `Uint8Array.prototype.slice`, restores both, and asserts one copy in total. It must go red on a second `set` through the same view, two `set` calls through one saved buffer, a defensive `slice`, and a duplicated `set` line. The `countingMemory` doc and the AC9 comment then claim only what the counter counts.
- **Still owed at completion.** AC8's and AC9's base and tip sizes and AC6's RED go in the handoff. The `typescript-packaging.md` order and the CHANGELOG entry are the lead's completion edits.
