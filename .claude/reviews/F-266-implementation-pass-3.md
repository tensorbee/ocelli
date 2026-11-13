# F-266 implementation microscope, pass 3

Reviewer: Codex, independent of GLM. GLM completed checker, catalogue and HTTP own microscopes before this pass. Reviewed frozen staged tree `5f8ef145ca9b42331f0b1c0929133f8a452c0c69`, checker SHA256 `b692f4082d007adf1cc008a73d81439584bb779d3305e604a2d56a7974817ef8`, test SHA256 `e662bb3758c18c57023f91d54db5d0dbecd1635e5ca5567d403976a0cbb5eaa4`, and HTTP probe `692adca7c7ab02e0c31afa81d278b8f1e7fea2e27cfc680065a55803e9dada8c` against base `1d627bd93f0c0e3b44231be9a0f3e76607a8eeb9`.

Result: 2 defects, 0 smells, 1 nitpick. No completion or prepared handoff is accepted. Remaining matrix and feature verification are separate outstanding acceptance work.

## Defects

D1. The shared Python URL oracle is not equivalent to the actual Node file URL pathname. `wasm_url` and `_known_roots` use `quote(..., safe="/")`, which encodes a plus sign as `%2B`. The actual config's Node URL pathname keeps `+`. Root built two actual pkg-ABSENT copies from the same current Vite provider. Both builds exited 0. The plain-root checker passed, while the plus-root checker falsely refused the valid emitted runtime import. Replacing only the expected URL argument with the actual Node pathname made that same output pass. A checkout or temporary root containing plus is a valid host path, not an unsupported future bundle layout.

The same cause misses the declared encoded-root refusal. With a real temporary base containing plus and a space, actual Node pathname is `plus+%20space`. The raw-root pattern contains a space and the Python-quoted pattern contains `%2B`, so neither matches those bytes in a non-worker output without `/@fs`. The actual PRESENT checker returns an empty problem list with all healthy import and asset conditions intact. This second observation is a direct checker counterexample, not an emitted-runtime proof.

Use the actual supported pathname encoding provider consistently, without inventing another general URL parser or applying a plus-only spelling patch. Add permanent healthy ABSENT plus-path and bad PRESENT plus-and-space encoded-root controls, derived independently from the actual URL contract. Preserve current path cases, the explicit base, raw-file scan and intentional ABSENT external import. Evidence is `/private/tmp/ocelli-s46-codex/f266-root-url-encoding-review/actual-build-proof.json` and `encoded-root-counterexample.json`, including both actual build sources, own exits and output hashes.

D2. `test_a_binary_known_root_output_is_refused_not_decoded` does not exercise its declared positive binary case. It creates an unused payload, then plants only the five UTF-8-compatible bytes `00 61 73 6d 01` as the healthy WASM file. This is not a valid non-UTF-8 module. Root copied the actual production checker and permanent test into a disposable source, inserted a real production-code mutation refusing every non-UTF-8 output, and executed that exact named test. Both original and wrong implementation exited 0. The test therefore cannot establish the promised acceptance of valid non-UTF-8 WASM.

A separate valid custom-section module containing `ff fe fd` passes actual Node `WebAssembly.validate`, is accepted by the original checker and is wrongly refused by that same mutation. Have GLM plant a valid non-UTF-8 payload in the permanent healthy fixture, retain its known-root negative leg, and demonstrate the actual production mutant failing that test followed by unmutated GREEN. Evidence, source identities and direct test logs are in `binary-test-sensitivity.json` beside the URL proofs. This follows microscope section 2, a test that passes both ways is a defect. It is not a claim that the current production checker rejects real WASM.

## Nitpick

N1. The private static harness still throws for malformed percent-encoded request targets. The current generated viewer paths are well formed, and no production server behavior is being claimed here, so no server framework expansion is needed. Narrow the two script comments and corresponding LLD sentence that say every unknown path receives 404 to the ordinary well-formed, decodable paths actually handled. Keep malformed-target behavior explicitly outside this probe's accepted scope.

## Verified repairs and bounded clean areas

Pass 2 D1 now requires the exact literal emitted glue target. Both missing-import and longer-wrong-target controls failed on the old checker and passed after GLM's repair. The current matcher is bounded to the actual emitted literal import shape, with no arbitrary JavaScript analysis claim. The pinned provider emits worker and glue together. Speculative different future output layouts are not supported current defects and do not justify accepting a basename anywhere.

D2 now performs awaited real HTTP requests for all five original positive URLs and compares the received policy with the expected policy. Original browser traffic and served-body hashes are snapshotted before the extra requests. Root inspected this source plus the actual intact, header-removed and restored logs. The old false green is retained. The new deliberate header removal exits 1 at the received-header assertion, while intact and restored states exit 0 with all 14 cases passing. The directory request now has a real 404 branch and permanent observation.

S1 is repaired by removing forwarding and dead helpers and the unused private URL parameter. The intentionally uniform public checker signatures remain. The raw recursive map and exact explicit base are present, with no binary decoding. A real production worker-only scan mutation fails the permanent bad-main test and exact restoration passes. The separate reduced-map helper test is not misreported as this executed mutation.

Root inspected config, worker initialization, actual checker callers, raw-map test fixtures, catalogue controls and HTTP lifecycle. The guard budget file is byte-identical to base at this snapshot. Normalizing generated runbook ordinals proves no old row payload or surrounding prose was removed or altered, and only the two declared viewer-build probe rows were added. Four registered probes and both healthy controls pass under the author receipts. The module initializes before readiness, dev and build specifiers remain distinct, and the ABSENT external import is preserved.

## Scope and next step

The ten real-package build cases cover the planned five path shapes and both package states. They did not include the independently discovered plus case. Dev and allowance sensitivity, final AC7 reconciliation, final LLD contribution metadata and the full feature profile remain owed at their final source identities. Readiness cannot follow from a clean bounded repair or an earlier matrix alone. GLM owns the two repairs and its next own review. Root will review the resulting final union and acceptance evidence. No pixel parity, manual study acceptance, S46 integration, push or sprint closure is claimed.
