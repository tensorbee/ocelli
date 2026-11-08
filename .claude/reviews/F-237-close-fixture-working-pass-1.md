# F-237 inherited-handoff fixture, bounded working review pass 1

**Author**: Codex recovery controller. **Independent reviewer**: root Codex,
who did not author this patch or test. **Result**: 0 defects, 0 smells, 1 nonblocking style nit
on final tree `1357745c7923ffab0cbd94b7c7a4e388e09ee3b6` against
`de803f7358e2677e93cee453538769d855c96037`. This is a two-file harness review,
not whole-sprint review, story completion or real integration.

The complete delta changes only scripts/guards/catalogue.py and its standing
catalogue test. All other 2350 blobs are unchanged. Complete patch SHA-256 is
`6266848d838561969a274337adbe453cfc07f9b6aa9f2b90d3199a2ece4fcd22`.

## Acceptance challenged

The synthetic completed-state fixture removes inherited ready handoffs only
inside its disposable Sandbox. The first Git call validates that sandbox
before deletion. It stages and commits only previously tracked matching
paths before capturing the evidence tree, without force-staging an existing
ignored run-state file. Production sprint_workflow.py is unchanged.

The regression drives the actual lazy close-preflight control with a copied
handoff, then repeats setup with an already-done backlog and ignored state.
It plants a new committed handoff after setup, refreshes otherwise-current
receipt trees, and observes the real production handoff refusal. It checks
that source handoffs remain byte-identical. Thus fixture isolation does not
weaken the refusal for an unconsumed integration handoff.

The original helper fails the actual control with inherited F237 and synthetic
handoffs present. A first Codex draft also shadowed the sprint name. Explicit
setup followed by lazy preflight masked that new fault in the first unit run
against the shadowing draft. The actual probe caught missing run state. The strengthened lazy-first
unit rejects that draft too. Final 58 catalogue tests, both acceptance probes
and their mandatory healthy control pass after repair. Earlier failed checks
are retained, not replaced by the final successes.

Root read the full patch, actual Sandbox path guards, both semantic failures,
final logs, and verified the complete delta against the handoff and candidate
base bytes. Source composition used the complete patch with three-way checks.
The actual candidate F237 handoff SHA-256 remains
`106e88e9e44e7508fb474f793fee522dae43c2ba18595fd93183afd56873aa2a`.

## Roles and limits

The preceding GLM implementation attempt exited 130 after 389.214 seconds,
with two broad-read scope failures and no edits or tests. It remains unfinished.
This repair and its shadowing defect belong to Codex. Root also missed the
shadowing on an initial read before returning any approval. The permanent
regression implements the existing workflow rule to challenge an acceptance
claim through its actual caller. No new workflow exception is introduced.

Exact direct exits, clocks, complete patch and source-preservation receipts
are in `/private/tmp/ocelli-s46-codex/close-fixture-codex-recovery/handoff.json`.
No browser, benchmark, GPU, profile or overall verification success is supplied
by this bounded review. The assembled candidate still needs its final checks.

## Supplementary GLM findings and root disposition

A separate read-only model reviewed the frozen packet in two calls, direct
exit zero in 343.645 seconds. Its unchanged final report is at
`/private/tmp/ocelli-s46-codex/close-fixture-independent-glm/first-output/report.md`,
SHA-256 `aef5ed32f3fe8334dabb0bdbe0d91898c1ed94f640880b052de17e669c1a26e5`.
The model reported one evidence defect and three nits, not a code defect.
Its 306 words exceed the 300-word request. It executed no checks, copied source
hashes without verification, and usage counters are unavailable.

D1 correctly asks for clarity about the historically misnamed probes-green
receipt. That receipt exits one and is failed evidence. The complete sequence
is explicit here, preserving all original filenames and records:

1. Original helper fails with inherited handoffs at 21:25:31.
2. The shadowing draft's first 58-unit run passes at 21:25:54, but actual
   probes fail at 21:25:57. That pair is not healthy or final evidence.
3. The strengthened lazy-first unit rejects the shadowing draft at 21:27:11.
4. The restored final 58-unit run passes at 21:27:14 and final probes pass at
   21:27:17. Only this final pair supports the repaired-source claim.

N1 asks to change the ignored-state comment. It explains why a forced
stage_all is avoided and is accurate. N2 suggests the test depends on a real
F237 handoff. The test unconditionally commits its own synthetic inherited
handoff, so its isolation control works with an empty source handoff directory.
Real F237 preservation is a separate actual receipt. Neither requires a code
change. N3 notes class-spacing style in the test file. It is accepted as a
nonblocking nit and no runtime or coverage claim follows from it.

This disposition resolves the evidence ambiguity without relabelling a failed
run, changing the production guard or substituting review for final checks.
