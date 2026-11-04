# F-198 review, pass 3

**Reviewed**: uncommitted F-198 on `sprint/s43`. HEAD was `90723c4`. The tracked delta is the working tree against that commit. Untracked story files are `crates/ocelli-wasm/src/tier_label.rs`, the package loader and instance-byte files, and the example worker, panel and tests.
**Author / reviewer**: implementation by the sprint host. Review by Claude Code, read only, with no shell.
**Result**: 0 defects, 0 smells, 2 nitpicks

## Acceptance challenged

AC1 expects false from a clean import. AC2 expects a rejecting initialiser to leave the flag false, two concurrent successes to share one initialiser, and a later success to skip it. AC3 requires the `resolvedTier` export and still bans pixel getters. AC4 posts the multipart envelope and the boundary. AC5 traces `bindCpuCanvas` and `publishCpuCanvas` and then `resolvedTier`. On that path no adapter is stored, so the label is `resolve(Auto)`.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| boundary vitest filter `reports the tier` | dev wasm build from pass 2 | prior receipt | 1 passed, exit 0 |
| Playwright Chromium `resolvedTier` | `crates/ocelli-wasm/target/boundary-wasm` | prior receipt | printed `A`, exit 0 |
| flag wording grep | working tree | read by the reviewer | the six sites say `resolves true` |

The Chromium launch used Playwright from `tools/oracle` with `--use-gl=angle`, `--use-angle=metal`, `--ignore-gpu-blocklist`, `--enable-webgl`, `--enable-webgl2` and `--enable-unsafe-webgpu`. The call was on the page realm of a small module page, not inside the example worker. The tier-b probe's recorded `B` is a GL-only export. `resolve(Auto)` with WebGPU enabled is expected to report `A`.

## Defects

None.

## Smells

None.

## Nitpicks

### N1, the production bundle carries a dead absolute path

`vite.config.ts` still defines the dev `/@fs` URL. A production bundle cannot serve it. The page stays on `core not built`, which is the declared failure state.

### N2, an empty tier line on a failed load

A failed load posts an empty tier string, and the panel renders the word Tier with nothing after it.

## Verified clean

Pass 2's false flag sentence is closed. The six sites say the flag stays false until `loadCore` resolves true. The thrown-load sentence is gone. The vacuous pixel assertion is gone. The Chromium observation printed `A`, so a constant `C` is not what that browser returned. The plan's tests table asks for package contracts. AC5's integrated evidence is this observation. Node has no GPU, so the boundary membership check alone does not reject a constant `C`.

`loadCore` sets the flag only after the initialiser returns. A thrown initialiser leaves it false. One worker does not start a second wasm instance after a successful load. Instance bytes stay in the only bulk slot and are not passed to `commit_frame`. The boundary is kept beside the ticket. An empty URL does not fetch. The example server has no synthetic Part 10 route.

## Limitations and handoff

The reviewer ran nothing. The live example page was not clicked. The worker realm was not the realm that printed `A`. The completion entry must record that Chromium receipt. No remediation is required for completion.
