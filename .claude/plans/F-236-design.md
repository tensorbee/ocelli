# F-236, acceptTransferredFrame honours stackIndex and stackEpoch

**Status**: draft
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

- **Q1, forward the keys or call the export directly.** Affects AC1 and AC5. Proposed: forward, decision 1. Owner: the lead in the S47 design round.
- **Q2, F-231's merged commit path.** Affects every row. The plan is re-read against it before approval. Owner: the lead.
