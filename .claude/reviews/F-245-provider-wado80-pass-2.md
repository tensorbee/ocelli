# F-245 WADO provider review, pass 2

**Reviewed**: Worker `/private/tmp/ocelli-s47-f245-glm`, HEAD d85406a5eed7d4340b3b4e6db961631569bff99a, dirty tree preserved. Hashes verified before and after:

| Path | SHA-256 |
|------|---------|
| packages/core/src/dicomweb-viewer.ts | 2817635599c9fb05fbf3d7a9186a87abfaee404748468c7ac9c5f4efb82bbdcf |
| packages/core/src/dicomweb-viewer.test.ts | 5a0ef45268085fa3315f63cead750cb9394a787d45416ca59eab26b2e57cd493 |
| packages/core/src/index.ts | d84fc60627d8f5ff0c04f9a26a8e4b124b7a833cc1aa48ccd768894a948cdd4b (unchanged) |
| packages/core/src/options.ts | 5c4d313550fb9941b1119fb18d5bf4621e8960ebdd4765b6b19ec540156b5597 (unchanged) |

Source delta reviewed against `/private/tmp/ocelli-s46-codex/f245-implementation/claude-provider-before-preabort.ts` (pass 1 base 7d5e1bbe).

**Author / reviewer**: GLM authored the repair and the new controls. Codex applied them and added one type import. Reviewer is Claude (root independent reviewer, subagent), who authored none of the reviewed material.

**Result**: 0 defects, 2 smells, 5 nitpicks.

## Acceptance challenged

The S1 repair, AC9's "wrong UID with the same length" and "a length-only check passes nothing", and the plan's S1 amendment rule that "a malformed supplied expected snapshot is refused rather than partially matched". Root re-ran its seven pass 1 public-caller cases against the new source, re-ran M5 and M6 against the new tests, and added three new mutants.

## Evidence

| Command / selected cases | Source identity | Executed now / prior | Exit and observation |
|--------------------------|-----------------|----------------------|----------------------|
| Three-file vitest command, worker root, Node 24.13.0, Vitest 4.1.11 | 28176355, 5a0ef452 | Executed now | 0, 113 passed |
| Root challenge file, scratch copy | 28176355 | Executed now | 0, 7 passed, including pre-aborted case 7 that was red in pass 1 |
| M5, remove identity, byteLength and boundary comparison in `decodeInstanceMetadata`, provider suite | a941167e | Executed now | 1, 3 failed, 65 passed |
| M6, `takeStagedInstance` ignores the caller receipt, provider suite | e4bce526 | Executed now | 1, 9 failed, 59 passed |
| PA_LIVE, pre-aborted branch also aborts every live entry before throwing | 2d2c0e93 | Executed now | 0, 68 passed |
| R1_KEYS, drop the outer `exactKeys` check on the caller receipt in `takeStagedInstance` | 3ec679aa | Executed now | 0, 68 passed |
| LEN_UID, native projection compares `instanceUid` by length only | 25d0d288 | Executed now | 0, 68 passed |
| Scratch restored, worker rehashed | 28176355 | Executed now | match |
| Forced test tsc, ESLint, GLM control mutants and pre-abort RED/GREEN | Coordinator receipts | Prior receipt only | 0, 0, and 1 per mutant as recorded |

## Defects

None.

## Smells

### S2, no same-length wrong-UID control at the provider layer
**Where**: dicomweb-viewer.test.ts:1388-1399 (projection) and :1529-1531 (receipt) use `2.25.999`, `2.25.888` and `2.25.777` against `2.25.1`, `2.25.2` and `2.25.3`. Each wrong value differs in length.
**What**: mutant LEN_UID replaces the `instanceUid` equality in `decodeInstanceMetadata` (dicomweb-viewer.ts:413) with a length comparison and all 68 provider tests stay green.
**Why it is wrong**: AC9 names "Wrong UID with the same length" and its failure column says "A length-only or digest-weak check passes nothing". The core unit tests are a named AC9 destination. The source is correct, but no provider test would catch the regression AC9 names.
**Scope and regression**: one same-length wrong value per UID field in both tables, for example `2.25.4` for the instance. It must go red under LEN_UID.

### S3, a malformed receipt shape is never presented to `takeStagedInstance`
**Where**: receipt cases at dicomweb-viewer.test.ts:1527-1541 only change values. No case adds or removes a key.
**What**: mutant R1_KEYS removes the outer `exactKeys` check (dicomweb-viewer.ts:1236-1247 region) and all 68 provider tests stay green.
**Why it is wrong**: the plan's S1 amendment requires a malformed expected snapshot to be "refused rather than partially matched", and says the tests cover it. Extra or missing keys are the malformed shapes. The projection table has extra-key cases for the native DTO, but the receipt table has none.
**Scope and regression**: add receipt cases with an outer extra key, an identity extra key and a representation extra key. Each must return null while the stage stays readable, and must go red under R1_KEYS.

## Nitpicks

- **N1 to N3**: carried from pass 1 as dispositions, unchanged. Unreachable length guards at :1197 and :1311. Oversized native reply or 422 body surfaces AllocationRefused. Unread native error bodies are not cancelled.
- **N4**: carried, not changed by the coordinator. `cancelPending` has no overflow guard, and a no-op `#removeExternalListener(null, null)` remains in `#drainQueued`.
- **N5**: the pre-aborted control at dicomweb-viewer.test.ts:1913 checks staged state and, through a successful take, the generation. It does not hold a live operation, so PA_LIVE (abort live entries in the refusal branch) survives. The realistic regression, calling `#beginIntent` first again, is caught because it clears the stage. Adding a held live WADO that must still stage would close this.

## Verified clean

- The repair moves the pre-aborted check ahead of `#beginIntent` (dicomweb-viewer.ts:728-731) and removes the later branch. The listener is attached only after admission. No path remains where an aborted signal is registered without being checked first, since `addEventListener` and the check are synchronous.
- No other refusal path has a side effect before admission. `#validateQidoOptions`, the UID checks in `searchSeries`, `searchInstances` and `fetchSelectedInstance`, and the disposed and overflow throws in `#beginIntent` (:493-498) all run before any abort, stage clear, queue rejection or generation change.
- The new controls inspected contain no tautology. The projection and receipt tables each pair a refusal with a valid recovery on the same provider. The transfer-failure test injects a `DataCloneError` through a spy, asserts CommitRefused, a cleared stage, take returning null and a fresh generation on refetch. The WADO 422 and stale-oversized cases are real native 422 responses, not counterfeited client refusals, and the stale case proves a newer stage survives. Native fixtures compute the digest from the posted body and the tests compare it with an independent hash of the upstream bytes.
- M5 and M6 now go red, so the pass 1 open coverage for the value checks is closed.

## Limitations and handoff

Native HTTP, page, CSP, browser, performance and memory remain unexercised. No browser, cargo or GPU was run. This is a bounded review of the four files, not F-245 acceptance. Next owner is GLM for S2 and S3, then a fresh independent pass on the changed tests.
