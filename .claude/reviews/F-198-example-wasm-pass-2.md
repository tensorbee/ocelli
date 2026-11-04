# F-198 review, pass 2

**Reviewed**: uncommitted F-198 on `sprint/s43` after the pass 1 remediation. HEAD was `90723c4`.
**Author / reviewer**: implementation by the sprint host. Review by Claude Code, read only, with no shell.
**Result**: 1 defect, 1 smell, 3 nitpicks

## Acceptance challenged

The StrictMode test creates a new canvas on each effect setup. A reused canvas made that test fail and left the header on `core not built`. The multipart fixture posts the envelope and the boundary `ocelli`. The host test `an_open_adapter_wins_over_a_cached_label` went red when the adapter branch was removed.

A production bundle that cannot serve `/@fs` stays on `core not built`. That is the plan's declared failure state, not a defect.

## Defects

### D1, the flag sentence says the flag flips when loadCore returns without throwing

**Where**: `crates/ocelli-wasm/src/lib.rs`, `packages/core/src/index.ts`, `packages/core/src/load.test.ts`, `docs/lld/build-targets.md`, `docs/lld/typescript-packaging.md`

**What**: `loadCore` catches a thrown initialiser and resolves false. It returns without throwing on a failed load as well as a successful one. The sentence read as if any finished call sets the flag.

**Why it is wrong**: a false claim in prose is a defect. The plan says the flag is true only after initialisation succeeds.

**Evidence**: `load.ts` and `load.test.ts`.

**Scope and regression**: prose only. Say the flag stays false until `loadCore` resolves true.

## Smells

### S1, the tier path the example takes is still undiscriminated

Replacing `tier_name(resolution.caps.tier)` with the constant `C` would leave the Node boundary test green. The example worker never opens an adapter, so the adapter-wins test does not cover that line.

## Nitpicks

### N1, a vacuous pixel assertion on a label that is already A, B or C

### N2, the production bundle carries a dead absolute path

### N3, an empty tier line on a failed load

## Verified clean

Pass 1's StrictMode defect is closed. The exclusive bulk slot is documented and tested. The boundary travels with the envelope. A failed publish still asks for the tier.

## Limitations and handoff

The reviewer ran nothing. No browser was opened. The next pass reviews the sentence fix and any tier observation.
