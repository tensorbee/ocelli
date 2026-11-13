# F-245 WADO helper review, pass 1

**Reviewed**: worker helper SHA256 215e2747deb44e110ba260e0ce72b35f37086ae5e601efa121a45567ff4b0afa, lib4182ba1b, Cargo manifest309cfe22, lock934b01bc and plan afc67edb, bound in native-wado-root-review-manifest.json over worker HEAD d85406a5eed7d4340b3b4e6db961631569bff99a.
**Author / reviewer**: GLM helper and permanent tests, completed GLM own source review, independent Codex root review. Root authored only the private discriminating consumer harness and fixture transformation used for this review.
**Result**: 0 defects, 0 smells, 2 nonblocking clarity nits in the helper slice. Full feature acceptance remains open.

## Acceptance challenged

AC9: actual dataset UID triple, UI VR and VM1, complete original multipart digest and size, single-part admission, context, and fixed encoded/decoded budgets. File Meta does not substitute for a missing dataset SOP UID. Same-length bodies with the same identity and different pixels must have different independent digests. A one-byte cap excess must be budget-refused even when the resulting payload is otherwise invalid.

Root added a pydicom-verified single-NUL-padded Study UID, declared boundary mismatch and illegal boundary, then valid retries. Changing only Content-Location preserved the parsed identity while changing the full original body hash and byteLength. Root also reversed the permanent multipart case: an invalid first Part10 followed by a valid second part must hit the one-part budget before any Part10 parse. These challenge the actual borrowed parser and helper together, not only a UID predicate.

## Evidence

Root evidence directory: /private/tmp/ocelli-s46-codex/f245-root-native-review.

| Command or material | Executed now or prior receipt | Exit and observation |
| --- | --- | --- |
| bin/ocelli.sh test ocelli-folder-viewer --lib dicomweb::tests:: | Root now, wado-full.json and log | 0, exactly17 selected tests pass, helper hash unchanged |
| Private wado-consumer Cargo test root_consumer:: | Root now, wado-composition.json and log | 0, exactly2 selected tests pass, exact source prefix and worker-matching dependency versions |
| Independent fixture verification | Root now with pydicom3.0.2 and Python hashlib | Both synthetic Part10 payloads retain expected UID triple, UI/VM and pixel0, full multipart digests independently recorded |
| Initial private composition fixture | Root earlier, wado-composition-initial-fixture files | 1, padded-UID case passed, two-part fixture failed because it omitted existing required part headers, not a product defect |
| Eight semantic mutant logs and restored17 receipt | Author-run, inspected now by root | Each selected1 test and failed at its intended assertion with exit101, restored17 exit0 |

The private fixture correction added Content-Location and Content-Length and made the selected tuple match the valid second part. No product source changed. wado-composition-fixture-qualification.json preserves that reviewer setup error. The private consumer copies the entire exact helper then appends only review tests. Its lock is seeded from the worker, and comparison finds no extra non-consumer package resolutions.

Mutation sensitivity covers dataset SOP versus File Meta fallback, VR, VM, requested tuple, full body digest, second-part budget, and both one-byte cap boundaries. wado-mutant-log-check.json records actual log membership and assertion inspection. The decoded-plus-one mutant fails with InvalidValue instead of the required AllocationRefused, establishing that generic malformed-data refusal does not satisfy the budget contract.

## Own review disposition and standards

Raw GLM own review and its qualification are preserved. It executed no tools. SPACE is not valid UI padding, and the odd compressed-stream padding is distinct from the OB element in the decompressed dataset. Root separately checked [PS3.5 A.5](https://dicom.nema.org/Medical/Dicom/current/output/chtml/part05/sect_A.5.html) and [PS3.5 9.1](https://dicom.nema.org/medical/dicom/current/output/chtml/part05/chapter_9.html), current2026d official indexed text. Earlier author2025c retrieval and failed2026d URL remain historical, not rewritten as a successful fetch. wado-standard-check.json names the separate root check.

Boundary syntax and marker matching are enforced by the unchanged multipart consumer. json serialization escapes values. Fixed safe error mapping deliberately omits offending media details. pop followed by checking no remaining instances is correct under the earlier part budget. byteLength means complete encoded multipart size, as approved. No successful two-part result or unrelated UN padding support is required.

## Verified clean

The helper validates request identity, nonce and safe generation before consumption, then uses fixed32MiB encoded and128MiB decoded limits with max_parts1. The borrowed reader keeps raw ownership with its caller. UID extraction uses actual dataset elements with UI and VM1, preserving valid NUL padding semantics. All three actual UIDs must match the selection. SHA256 covers the same full original slice, not reconstructed JSON or Part10 bytes. The response contains only the intended six fields and no part-location echo. Prior QIDO tests still pass.

## Nits and remaining work

N1: broaden the QIDO-only module description to include WADO and remove the generated append-location comment when touching documentation next. N2: the encoded-cap test boolean named over is true for the passing exact-cap row. Rename it should_pass for clarity without changing expectations. Neither blocks this bounded approval.

Actual HTTP admission before body allocation, typed header parsing, request body charging and retirement, reply serialization cap, browser provider and page UI, all browser regressions, release performance and final feature verification remain owed. Broad viewer clippy currently exits101 for unwired consumers, preserved rather than suppressed. This review does not turn that failure green. Later HTTP wiring and source changes require their own review and normal final verification. The native lease is returned to the sole writer.

## Subsequent visibility-only delta

Root independently reconstructed current helper SHA-256 864b7ad2dd7c34b27f7104ff6f2e7e3371a934ba2d3875bd56981c666c825013 from reviewed 215e2747deb44e110ba260e0ce72b35f37086ae5e601efa121a45567ff4b0afa by exactly three `pub(crate)` visibility additions on the UID and nonce validators and maximum generation constant. All other bytes match. This source check ran after the original review, with no runtime rerun. New HTTP consumers require their own independent review. Evidence is `/private/tmp/ocelli-s46-codex/f245-root-native-review/wado-validator-visibility-root-check.json`.
