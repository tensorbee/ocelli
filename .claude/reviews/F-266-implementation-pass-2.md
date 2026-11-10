# F-266 implementation microscope, pass 2

**Reviewed**: frozen partial union after the approved amendment. Checker SHA256 `678a00a4b4aa8083d2277945dc5a7166c1503cc36b8604f005c833d21b5fb653`, CSP probe `c426d3c4bb71c8d65951071b07ce323487a0971b27bc26c03068f0212a34af99`. Both hashes were reconfirmed on resume, 2026-11-10. Runtime controls below were executed before the operator pause on 2026-11-09, not rerun today.

**Author / reviewer**: GLM authored the semantic code and returned three grouped own microscopes. Codex independently inspected the changed providers and consumers and executed the discriminating controls. Unsupported author-review findings are not adopted.

**Result**: 2 defects, 1 smell, 2 nitpicks. No completion or prepared handoff is accepted.

## Defects

D1. present_problems claims the worker must reference the emitted glue but never checks that reference. Passing valid worker, glue and WASM names with worker bytes containing only const booted = true yields an empty problem list. This actual helper invocation is recorded in /private/tmp/ocelli-s46-codex/f266-root-missing-worker-import-proof.json. Enforce the declared emitted-worker reference contract through the actual checker caller and add a meaningful missing-reference control with the other asset conditions valid. Asset names alone do not establish worker loading. No new general JavaScript parser framework is requested.

D2. The genuine static positive checks a locally recorded policy constant rather than an independent HTTP observation. Root copied the unmodified emitted dist, real release inputs and the permitted synthetic fixture into disposable sources. The original script passed all 14 cases. Renaming only the two actual Content-Security-Policy emission fields to an irrelevant control header, leaving the logged policy constant unchanged, still passed all 14 and claimed strict policy on every response. This is a failed sensitivity control. The exact original script was restored and again passed. The worker source was never edited. Inputs and direct exits are in /private/tmp/ocelli-s46-codex/f266-root-csp-header-control-input.json and f266-root-csp-header-control-native-results.json. Logs preserve the false positive. The initial sandbox browser-start denial is separate environment evidence.

Observe the actual emitted or received HTTP policy independently of the expected constant, preserving unmodified dist bodies and the strict policy. Demonstrate that removing the actual header now fails for its named reason, then restore GREEN. Retain genuine browser readiness and existing negative cases. The current server does emit the correct header, but its test cannot detect its loss.

## Smell

S1. The checker retains redundant forwarding and dead production helpers. wasm_url only forwards to browser_wasm_url. rollup_wasm_url has no production caller, and _partial_spellings is retained through a new test rather than through the executed checker. These separate implementations can remain green while the actual config or caller changes, and increase the places a reader must inspect. Consolidate the live URL calculation and remove unused production helpers and tests that merely keep them alive. Preserve actual encoding and mode coverage through meaningful consumer controls. Review the similarly trivial _raw_workers forwarding layer during that cleanup. This follows the repository's explicit no-forwarding-wrapper rule and test-sensitivity requirement.

## Nitpicks

N1. One test says its fixture has no ocelli prefix while the fixture is named ocelli_wasm-x.js. Correct the description.

N2. The new static server reads an existing directory as a file, which would throw for a request such as /assets/. Its ordinary positive paths work. Keep any claimed unknown-path behavior accurate without broadening this into an unrelated server framework.

## Verified clean

Pass 1's all-output omission is repaired: emitted returns recursive raw bytes and the PRESENT scan checks the separate raw root, once-encoded root and /@fs patterns, including binary outputs. Intentional ABSENT external import behavior remains. The declared budget change is only viewer-build 11 to 10 at this snapshot, with other budget values preserved. That is an observed count, not a target for later repairs.

Root actually replayed the registered harness with all three viewer-build probe IDs. Direct exit 0, three refusals caught for their declared reasons and two shared healthy controls passed. Evidence is /private/tmp/ocelli-s46-codex/f266-root-registered-probes.json. This contradicts the own review's high finding that the present healthy fixture embeds an external import. The actual fixture imports its bundled chunk. Another own diagnostic claim about documentEntry assertion ordering is contradicted by source and is not adopted.

The current worker still requires import and default initialization before readiness. The Vite mode and presence split and existing absence behavior remain intact. Source and header sensitivity controls are distinct from corpus or pixel parity.

## Limits and handoff

The full mode, presence and path matrix, production outside-worker countermutant and final feature verification remain open. A reduced input map is a simulation, not an executed production-code mutation. GLM owns the repairs and its next own microscope. After that, root reviews the full union again. No source change from this review, no S46 integration, push or sprint closure.
