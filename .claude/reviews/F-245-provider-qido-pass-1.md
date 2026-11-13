# F-245 QIDO provider review, pass 1

**Reviewed**: Isolated worker head d85406a5eed7d4340b3b4e6db961631569bff99a plus the frozen six-file snapshot in `provider-query28-root-review-manifest.json`. Provider product SHA-256 9d3badf9a9b1a5c7f1ce10a734402c2222051ae8d86355155b18328d00b0b311. Provider tests ca60bcbe3e33391317c47eb53319ad45b431d8942c80400911eeb79399f0526d. Options 5c4d313550fb9941b1119fb18d5bf4621e8960ebdd4765b6b19ec540156b5597. Index ccf4f148b5ea991e739d6d5877eb87c2004ba2c18ec973d7d314782e15bc6c54. Core product ab104227213966d7c6e047954c2b30fbb4aae52b8afd97475120ccd14b5b6189, core test ea0df3f1ed8d7d7103a18753280cb20787ed674f45e1db72a159451aee668ad2.

**Author / reviewer**: GLM authored semantic source, tests and own review fragments. Worker Codex coordinated exact application and qualified its receipts. Root Codex independently inspected the complete QIDO provider and changed exports/options, copied the frozen source, replayed tests and authored the additional public-consumer pool challenge. This review does not independently approve root-authored design preparation.

**Result**: 0 defects, 0 smells, 0 nitpicks in the bounded QIDO-only snapshot.

## Acceptance challenged

AC4 and the QIDO portion of AC9. Two canceled but nonsettled requests must continue occupying live slots. Replacement of the queued request must settle it without fetching. Once an old finally promotes a newer request, another old finally must not forget that new live entry. Caller options and nested filter tuples must remain captured across queue delay. Required hierarchy include fields must be added without mutating caller arrays or changing legacy client defaults.

Root extended the author sequence: q1 and q2 are held at the native fetch, q3 is replaced by queued q4, q1 frees a slot for q4, then q2 completes. With q4 still deliberately nonsettled, q5 starts in the remaining slot and q6 must queue. The final valid q6 result has generation 6. This challenges the live-entry ownership that merely observing q4 success after q2 did not establish.

## Evidence

Root evidence: `/private/tmp/ocelli-s46-codex/f245-root-provider-review/`. Author receipts named below are under the existing `f245-implementation` evidence directory.

| Command / selected cases | Source and input identity | Execution | Exit and observation |
| --- | --- | --- | --- |
| Node 24.13.0, Vitest 4.1.11 with private review config | Frozen provider and dependencies, wholly synthetic transport mocks | Root replay, query28-plus-root-slot.json/log | 0, 29 tests: 28 author controls plus one independent consumer |
| Replace only `this.#live.delete(entry)` with `this.#live.clear()` in disposable copied product | Mutant eac75f9dae2cc61d2d9a8b04c3740f28a694d7376e25dbf07259ab0337c1d7f1 | Root execution, whole-set-clear-mutant.json/log | 1, exactly one selected test fails at upstream admission count 5 rather than 4. No compiler or timeout failure |
| Restore original product and select root consumer | Restored 9d3badf9 | Root execution, root-slot-restored.json/log | 0, exactly one selected test, 28 skipped. Worker source never mutated by root |
| Combined provider/core/options suite | Same frozen subset and unchanged core product | Prior worker receipt provider-query28-plus-core45-restored.json/log inspected | 0, 73 tests: 28 provider plus 45 core/options |
| Forced actual test-project TypeScript build, targeted five-file ESLint | Worker snapshot reported in handoff | Prior worker receipts provider-current-query-tsc.json and provider-current-query-lint.json inspected | 0 for both. This is not the library-only TSC exclusion path |
| Remove mandatory study UID, remove dedup, disable freshness | Disposable author mutants, actual assertion logs inspected | Prior worker receipts | 1 each, selecting respectively 1, 3 and 2 failing assertions. Restoration then passes 73 |
| Replace strict empty-scope alias with permissive shape | Prior forced test-project check, not runtime Vitest | Prior worker receipt provider-static-empty-scope-mutant.json/log inspected | 2 at the intended type assertion. Restored forced build exits 0 |

## Defects and smells

None reproduced in this bounded snapshot. Raw own-review labels are retained rather than adopted as counts. The UID review calls several correct branches defects while explicitly describing them as correct. Its snapshot-coverage concern is addressed by the queued tuple/array control. The queue own review falsely says q2 never fetched, although the test waits for both q1 and q2 at the native gate. That trace is not evidence. Root inspected the raw reports and their separate qualifications.

## Verified clean

The required include-field helper runs after option validation, creates its own deterministic deduplicated array and preserves requested extras. The generic legacy client source is unchanged. The provider constructs a fresh bounded client per operation, captures scope/options/nonce/generation and checks freshness across its awaited boundaries. Successful native DTOs require exact keys, matching scope and context, actual raw-body digest and valid UID-only rows. Returned objects, nested scope and rows are frozen. Missing optional returned parents remain accepted, while present wrong parents refuse.

Inspection and actual controls cover the two-live/one-queued lifecycle, settled replacement, stale success/error rejection, immutable nested filter capture, response row validation, malformed or mismatched native envelopes, actual-byte upstream cap, safe HTTP status/code mapping, constructor option registration and public index exports. Root's additional mutation proves its live-entry challenge discriminates the plausible wrong whole-set cleanup.

The core test delta from reviewed 8e8270d3 to ea0df3f1 is exactly a stable capture of the already-checked ready resolver before entering the async callback. No other test behavior changes and no core product changes occur in that delta.

## Limitations and handoff

QIDO-only source and mocked browser transport are reviewed. Native HTTP dispatch, actual serving of the new module, configuration, CSP, source chooser, local image/marks/conformance preservation, browser network requests and release timing remain unproved here. WADO methods, staging, transfer-once ownership and shared QIDO/WADO pool composition are absent and require new review. This is not complete F-245, source preparation, corpus, tier, physical heap or RSS acceptance.

The planned WADO copy inventory remains required before carrying the QIDO hash-copy pattern into two 32 MiB operations. Provider-created byte copies count toward the approved accepted-raw ceiling. Crypto-internal and network buffers remain outside that scoped bound. No memory measurement or full 5C certification is supplied by this pass.

Next owner is the GLM implementation coordinator: continue WADO and native/page composition, preserve these controls, obtain own review and return the changed source. Root's consumer may be retained as an independently authored review regression with its authorship stated. Do not claim the new WADO source is covered by this QIDO snapshot review.
