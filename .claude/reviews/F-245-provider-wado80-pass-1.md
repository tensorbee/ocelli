# F-245 WADO provider review, pass 1

**Reviewed**: Worker worktree `/private/tmp/ocelli-s47-f245-glm`, branch `work/f-245-glm`, HEAD d85406a5eed7d4340b3b4e6db961631569bff99a, dirty tree preserved. Exact files, verified with `shasum -a 256` before and after the pass:

| Path | SHA-256 | State |
|------|---------|-------|
| packages/core/src/dicomweb-viewer.ts | 7d5e1bbe3ae5fced06d8a2701e17da7d47b9d20e25a98eb1984963886be066fa | new, untracked |
| packages/core/src/dicomweb-viewer.test.ts | a8d7cfc1a3328f2671f6b3465a4b9e8bdb4a3032181b437b051b7c512ce0a4de | new, untracked |
| packages/core/src/index.ts | d84fc60627d8f5ff0c04f9a26a8e4b124b7a833cc1aa48ccd768894a948cdd4b | diff against d85406a5 |
| packages/core/src/options.ts | 5c4d313550fb9941b1119fb18d5bf4621e8960ebdd4765b6b19ec540156b5597 | diff against d85406a5 |

Read alongside, not under review: `packages/core/src/dicomweb.ts` (ab104227) for the bounded delivery branch the provider composes on.

**Author / reviewer**: GLM authored the semantic source and tests. Codex coordinated application and receipts. Reviewer is Claude (root independent reviewer, subagent), which authored none of the reviewed material. This pass does not review the plan itself.

**Result**: 0 defects, 1 smell, 4 nitpicks.

## Acceptance challenged

AC4 and the provider half of AC9, against plan sections "Viewer provider ownership and staged state" and "Concurrency and lifecycle contract".

Root wrote seven independent public-caller cases in a scratch copy (`/private/tmp/claude-501/f245-w80-scratch/packages/core/src/root-challenge.test.ts`), none derived from the implementation's tests.

1. Mixed QIDO and WADO ownership of one pool. w1 (WADO) and q1 (QIDO) are held at the native gate, then w2 (WADO) queues and is replaced by q2 (QIDO). Expected from the plan: w2 settles Aborted without reaching upstream, w1's later valid native reply settles Aborted and stages nothing, q2 is admitted only when w1's finally frees a slot, q1 settles Aborted, and a fresh WADO then stages and is taken. Observed as expected. This rejects a per-kind pool, a queue that fetches before admission and a stale stage.
2. Cancel during the synchronous take. A receipt whose `sourceNonce` getter calls `cancelPending()` must return null and leave nothing takeable. Observed null. Removing the post-check re-validation at dicomweb-viewer.ts:1301 turns this case red, so the re-check is load-bearing.
3. Stale WADO native success after `cancelPending` settles Aborted and never stages.
4. `dispose` with two live WADO operations and one queued settles all three Aborted, with no third native call.
5. Public WADO 422 `{"code":"InvalidValue"}` maps to InvalidValue, and an oversized 422 body after cancel maps to Aborted rather than AllocationRefused.
6. WADO upstream that streams 32 MiB + 1 behind `Content-Length: 4` is refused AllocationRefused with zero native calls.
7. A pre-aborted QIDO signal refusal must not alter staged state. This case fails, see S1.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `node node_modules/vitest/vitest.mjs run packages/core/src/dicomweb.test.ts packages/core/src/options.test.ts packages/core/src/dicomweb-viewer.test.ts`, Node 24.13.0, Vitest 4.1.11, worker root | Exact hashes above, synthetic mocks only | Executed now | 0, 3 files, 80 tests passed |
| Root challenge file, scratch copy, unmodified product 7d5e1bbe | Seven cases listed above | Executed now | 1, 6 passed, 1 failed (case 7, S1) |
| Mutant M3, remove re-validation at :1301, `-t reentrant` | Product 475b41e3 in scratch | Executed now | 1, the reentrant take case fails, others skipped |
| Mutant M5, disable identity, byteLength and boundary comparison in `decodeInstanceMetadata` (:410, :439, :452), author provider suite | Product bbbe59f5 in scratch | Executed now | 0, all 35 provider tests pass. Open coverage, see handoff |
| Mutant M6, `takeStagedInstance` ignores the caller receipt and compares stored metadata with itself, author provider suite | Product 13a79317 in scratch | Executed now | 0, all 35 provider tests pass. Open coverage, see handoff |
| Scratch product restored, worker hashes rechecked | 7d5e1bbe in scratch and worker | Executed now | Hashes match. Worker `git status --short` line count unchanged at 25 before this file |
| Forced test-project `tsc --build`, five-file ESLint | Handoff receipts | Prior receipt only, not rerun | 0 for both per handoff |
| Author mutants for 422 text, header-only 422, no-transfer, raised upstream cap | Handoff receipts | Prior receipt only | 1 each per handoff |

## Defects

None reproduced.

## Smells

### S1, a pre-aborted QIDO signal refusal discards the staged instance and aborts live work

**Where**: packages/core/src/dicomweb-viewer.ts:727 calls `#beginIntent()` before the pre-aborted check at :741-745.
**What**: `searchStudies({ signal: AbortSignal.abort() })` rejects "operation was aborted before admission", but by then `#beginIntent` has already aborted every live entry, nulled `#staged`, rejected any queued job and advanced the generation. The request is refused as never admitted while its side effects are those of an admitted intent.
**Why it is wrong**: the plan advances the generation "once per user intent BEFORE capture" and releases the staged body on "a new intent, a source change or a cancel". A request refused before admission is none of those. Every other refusal path in the module (invalid options at :723-726, invalid UIDs at :564, :579, :1027) refuses before `#beginIntent` and has no side effect, so the refusal policy is inconsistent across entry points. Microscope's rule is that a refusal followed by a valid operation must not expose hidden state changes.
**Evidence**: root case 7. Stage an instance, call `searchStudies` with an already-aborted signal, observe `Aborted`, then `readStagedInfo()` returns null instead of the staged receipt.
**Scope and regression**: affects any consumer passing a caller-owned signal, for example a page that reuses a tab-scoped controller. The staged body is lost and a live WADO fetch for the current selection is aborted, with only a pre-admission Aborted visible. Smallest fix is to check `externalSignal.aborted` before `#beginIntent`. Smallest regression is root case 7, plus the same with a live held WADO that must still stage.

## Nitpicks

### N1, two guards no input can reach
dicomweb-viewer.ts:1197 compares `owned.byteLength` with `raw.byteLength` after `owned` was allocated from `raw.byteLength` and never escapes. dicomweb-viewer.ts:1311 checks the length of a `structuredClone` transfer, which preserves length. Neither can go red. They read as protection that does not exist.

### N2, oversized native replies map to AllocationRefused
dicomweb-viewer.ts:169 and :902-908 surface a native 200 reply or a native 422 body above 16 MiB as AllocationRefused, the same code the plan gives native 413. The plan's client-only list maps a malformed native reply to CommitRefused and names no code for an oversized one. The author test at dicomweb-viewer.test.ts:1020 pins AllocationRefused. Worth a one-line plan decision so the page does not report "request too large" for a misbehaving native handler.

### N3, unread native error bodies are never cancelled
dicomweb-viewer.ts:873-892 and :1147-1166 throw on 400, 413, 415, 503 and other statuses without cancelling `response.body`, and :869 and :1143 throw on a stale response the same way. Harmless under the mocks, but it leaves a same-origin connection body undrained in a real browser.

### N4, dead and unguarded counters
`#drainQueued` calls `#removeExternalListener(null, null)` for WADO at :783, a no-op. `cancelPending` at :600 increments the generation without the `QIDO_MAX_GENERATION` guard `#beginIntent` applies at :496. Unreachable in practice.

## Verified clean

- One `#live` set and one `#queued` slot are shared by QIDO and WADO (:483-484, :997-1020). Entries leave `#live` only in each operation's own `finally` (:834, :1220), so canceled-but-unsettled operations keep occupying slots. Root case 1 exercised this across kinds.
- A replaced or stale queued job settles Aborted without fetching (:503-514, :774-787). `#drainQueued` rechecks the current generation before admitting.
- Every await in `#executeInstance` is followed by `#checkFresh`, and the catch maps any stale failure to Aborted, so a canceled operation's error does not surface as a UI error and a stale success cannot stage (root cases 3 and 4).
- `takeStagedInstance` validates the receipt synchronously, re-validates the snapshot after any reentrant getter (:1301, proved by M3), nulls the stage before transfer, uses `structuredClone` with an exact transfer list and no fallback copy, and returns a plain `{ body, metadata }` with no release wrapper. The author transfer test fails under its own no-transfer mutant per the handoff.
- Provider-created raw copies. Per WADO operation the client's assembled buffer (at most C) and the provider's `owned` copy (at most C) coexist until the operation returns, so one operation holds at most 2C. Only the current generation can stage, and any new intent clears the stage, so two live operations plus the stage peak at 4C, inside the plan's 5C logical ceiling. Network, fetch-body and crypto copies are outside that bound as the plan states.
- Caps are checked before the assembled allocation. `readBoundedBody` (:166-172) and the client's `#readBounded` (dicomweb.ts:519-524) count streamed bytes, never trust Content-Length, and allocate the merged buffer only after EOF at or below the cap. Root case 6 confirmed an underreported header does not admit 32 MiB + 1, and the author exact-cap pair covers 32 MiB accepted.
- The 422 body is bounded at 16 MiB, parsed to an exact `{"code"}` shape, and only InvalidValue maps to InvalidValue. Public WADO 422 mapping and the stale-oversized Aborted path behave as planned (root case 5).
- Upstream errors from `DicomwebClient` are preserved, since `DicomwebError` extends `OcelliError` and is rethrown unchanged when the operation is fresh.
- Every promise reaches settlement in all root lifecycle cases, including dispose with queued work, and no case timed out.
- index.ts adds exports only. options.ts adds `DICOMWEB_PROVIDER_OPTION_KEYS` with a compile-time completeness check, a type-only import, and registers it in `OPTION_KEY_LISTS`.

## Limitations and handoff

Known-open items from the handoff, reported as open coverage, not new defects:
- Receipt and native DTO single-fault controls. M5 and M6 show that no author test fails when the native DTO identity, byteLength or boundary comparison is removed, or when `takeStagedInstance` ignores the caller receipt entirely. The source is correct on inspection, but the plan's S1 amendment controls are not yet tests.
- Public WADO 422 counterparts and stale or malformed native controls. Root case 5 is an independent probe, not a retained author test.
- The mixed QIDO and WADO pool consumer. Root case 1 is an independent probe that may be adopted with its authorship stated.
- Transfer-failure-then-refetch recovery is not exercised.
- Native HTTP, page, CSP, browser, release performance and memory measurements are not exercised here. No GPU, browser or cargo command was run.

This is a bounded clean-of-defects review of the four files above. It is not complete F-245 acceptance. Next owner is the GLM implementation coordinator for S1 and the open coverage, followed by a fresh independent pass over the changed material.
