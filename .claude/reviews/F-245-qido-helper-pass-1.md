# F-245 QIDO helper review, pass 1

**Author / reviewer**: GLM product and permanent tests, GLM own source review, independent Codex root review.
**Reviewed**: worker helper SHA256 29264f18476dbdc64918f689bb3fd1ed9dda7207310fe1dfe91645e777c54274. native-qido-root-review-handoff.json binds the helper, module registration, Cargo manifest and lock snapshots over worker HEAD d85406a5eed7d4340b3b4e6db961631569bff99a.
**Result**: 0 defects, 0 smells, 0 nits in this helper scope.

## Acceptance challenged

AC9, typed query identity and optional returned parent validation. Required leaf UI values must have VM1 and valid UID syntax. Missing returned parents remain absent rather than becoming asserted returned values. Captured request scope is separately preserved even for an empty response. Wrong returned parents refuse the batch. Only identifier projections reach the reply.

Root challenged a multirow response whose first row is valid and whose last row has a wrong parent, followed by a valid retry. The whole invalid batch refuses. The retry preserves captured scope, nonce and the maximum safe generation, drops an unrelated wholly synthetic text value, and hashes the complete original body. Adding whitespace preserves rows and changes the body digest. No clinical file or value was used.

## Evidence

Root evidence is under /private/tmp/ocelli-s46-codex/f245-root-native-review.

| Command and selected cases | Execution | Exit and observation |
| --- | --- | --- |
| bin/ocelli.sh test ocelli-folder-viewer --lib dicomweb::tests:: | Root now, qido-full.json and log | 0, exactly 8 tests pass, helper hash unchanged |
| Private qido-consumer Cargo test with exact independent::late_bad_parent_refuses_whole_batch_then_valid_retry selector | Root now, qido-composition.json and log | 0, exactly 1 selected test passes, helper hash unchanged |
| Same private consumer before seeding the lock from the worker | Root earlier in this pass, qido-composition-initial-resolution files | 0, separate dependency resolution, superseded by the worker-lock-seeded replay |
| Four VR, parent, encoded-cap and body-hash mutants, then restored eight controls | Author receipts inspected now | Each mutant selects 1 actual test and exits101 at the intended semantic assertion, restored eight exit0 |

The consumer copies the exact helper bytes into an isolated crate. It does not change product source. Its final lock is seeded from the worker. qido-composition-lock-comparison.json confirms no non-consumer package resolution is absent from the worker lock. This remains a private helper caller, not an HTTP or browser consumer.

## Own review disposition

The raw own report is preserved, with its fenced-JSON transport refusal and later mechanical extraction kept separate from product checks. The model did not execute tests itself. Its hypothetical disagreement between raw and semantic text is not a present counterexample. Semantic normalization belongs to the existing metadata provider and does not require byte equality. A future shared cap constant is an integration choice when the HTTP reader exists. The approved fixed code-only error mapping intentionally avoids exposing parser context. These observations do not establish current defects or smells.

## Verified clean

All three query levels validate the requested context before parsing. Returned optional parents are checked independently from captured scope. Empty rows still bind context. The borrowed parser enforces encoded length and root row limits, and the helper hashes the same complete received bytes. No partial projected batch escapes after a late row refusal. The legacy owned parser is not modified by this helper. Root did not author the product implementation.

## Limitations and handoff

HTTP admission before allocation, header grammar and duplicate handling, retained body charging, closed status replies and the serialized reply cap remain unfinished integration work. WADO, configuration, CSP, provider lifecycle, page UI, browser regression, performance and full feature verification are outside this clean slice. This is neither feature completion nor sprint readiness. The sole F245 writer may resume the native lane and helper edits, with later changes requiring fresh review.
