# F-268 final union microscope, pass 2

Reviewer: Codex, following the actual full feature-floor failure.

Result: 1 defect, 0 smells, 0 nitpicks. Not ready for prepared status. The earlier independent slice and union reviews missed this catalogue composition case. Their original records remain intact.

## D1, the new docs arm invalidates existing fixture-selection assumptions

The actual final staged floor exited 1 in 790.240854625 seconds: 29 gates passed, guards failed, none skipped. Source tree before and after is `3c4a55fe106a3462abdbfcda3f436a6729400e3e` and tracked working content matches the index. Corpus was not run. No successful feature receipt or completion is inferred from the 29 passing gates.

Root independently reran only `ci-floor.gate-with-an-unextractable-arm-command` and `ci-floor.background-operator-in-an-arm`. Direct exit 1 reproduced the same HARNESS and builder error in approximately 4 seconds. These are fixture failures, not evidence that the actual CI docs step is absent.

The selector `_a_gate_with_an_unextractable_arm_command` now chooses docs before the older eligible arms. Its `env ... cargo doc` arm is seen as containing visible and unextractable command text even though the old mutations cannot separate it into the assumed independent statements. Expanding its named CI step after removing the visible command reaches a missing-floor-gate refusal rather than the declared unextractable-command refusal. The background-operator fixture also assumes a preceding chained && statement, which the single docs command does not have. The selector does not prove either caller's concrete mutation preconditions.

GLM must repair the fixture selection/construction against actual required shapes, retaining the meaning of both existing probes. Do not alter the production docs command merely to suit a fixture, remove a probe, weaken expected diagnostics, exclude the docs gate, change CI policy, or adjust a budget without an actual measured source change. Include a permanent control that the newly introduced docs-arm ordering cannot select a non-constructible state. A probe must mutate the intended state and reject it for its declared reason. Rerun focused catalogue tests, both old probes, actual docs controls, census and generated records as affected, followed by independent re-review and the actual final staged feature floor plus corpus. Existing failed receipts stay preserved. New semantic changes invalidate the earlier successful affected-source checks as appropriate.

Root has not authored the semantic repair. New GLM source transmissions are temporarily held after automatic approval review rejected the separately requested F-266 source self-review for insufficiently specific endpoint authorization. The specific server approval request is pending. Local read-only diagnosis and evidence preservation remain authorized. No gate is waived and no sprint, merge or publication state changes here.
