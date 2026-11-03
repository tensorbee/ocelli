# F-109 review, pass 1

**Reviewed**: Uncommitted working tree on `sprint/s39`. HEAD is `bf8981a5a67a9a12cbd95fc5f2b27a35f7fbc6fb`. Nothing in this diff is staged. `packages/core/src/shadow.ts` and `packages/core/src/shadow.test.ts` are untracked. `packages/core/src/index.ts` and `packages/core/src/index.test.ts` are unstaged modifications. `shadow.ts` sha256 is `49eb84526e5fd59f3de4a7d29f4266046e35c9593e6b23ecf1c4c9fda558a001` at the start and after the comparison restore.

**Author / reviewer**: Author is the parent session. Reviewer is independent subagent `01a1a012-56d3-7c23-b882-4ccedd21c4e3`. This reviewer did not write the code or the design.

**Result**: 0 defects, 0 smells, 0 nitpicks

## Acceptance challenged

AC1 to AC4 were checked against the approved plan, not against the test titles. The caller-supplied buffers, the integer limit from 0 to 255, the name `shadowDivergence`, and the refusal to render or import the other library are local plan decisions. They are not HLD formulas. HLD section 12 says shadow mode renders both, diffs, alerts, and only then flips the flag. This function is the diff count. A count above 0 is the alert the caller reads. It does not render and it does not flip a flag.

The unit file locks a difference of 2 at limit 1, a difference of 1 at limit 1, equal bytes at limit 0, unequal lengths, limits 1.5 and 256 and -1, and unchanged bytes after a later difference. It does not lock an empty pair, a difference of 255 at limit 255, `NaN`, the reverse argument order, or a count above 1.

The probe derived these expectations from the plan:

- Two empty buffers at limit 0 count 0.
- `[0]` and `[255]` at limit 255 count 0 in both orders. At limit 254 the count is 1 in both orders. A subtraction without `Math.abs` would accept only one order.
- `NaN`, `Infinity`, and 255.5 return null on buffers that differ. Both buffers stay `[0]` and `[255]`.
- `[0, 0, 10]` and `[1, 0, 0]` at limit 0 count 2 in both orders. A boolean hidden as 0 or 1 would fail.
- A two-byte view of `[4, 9, 1]` against `[4, 8]` at limit 0 counts 1, and the underlying bytes stay `[4, 9, 1]`.
- The same `Uint8Array` as both arguments counts 0 and stays `[7, 8]`.
- `core.shadowDivergence` is the same function as the source export.
- `kindEnabled` stays false for stack, MPR, and volume after those calls.
- `shadow.ts` has no `addEventListener(` call and does not mention `tolerances.json`, `comparator`, or `@cornerstonejs`.

Those observations reject a compare that writes a buffer, treats an equal difference as a miss, accepts a non-integer limit, drops the sign, or collapses every alert to 1.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `shasum -a 256` on `shadow.ts` and `flag.ts`, `git rev-parse HEAD`, `git status --short` | Working tree before the comparison edit | Executed now | Exit 0. HEAD `bf8981a5a67a9a12cbd95fc5f2b27a35f7fbc6fb` on `sprint/s39`. `shadow.ts` is `49eb84526e5fd59f3de4a7d29f4266046e35c9593e6b23ecf1c4c9fda558a001`. `flag.ts` is `c4daca94e72ace24876443d7c3923dc816e80834b00b7627cb115d34b709753f`. Status listed the two untracked shadow files and the two unstaged index files. |
| `npx vitest run` on `shadow.test.ts`, `index.test.ts`, `events.test.ts`, and `flag.test.ts` | Product bytes before the mutation | Executed now | Exit 0. 26 tests passed. 4 in `shadow.test.ts`, 4 in `index.test.ts`, 8 in `events.test.ts`, and 10 in `flag.test.ts`. |
| `npx tsc -p packages/core --noEmit --pretty false` | `packages/core` tsconfig include `src/**/*` | Executed now | Exit 0. No diagnostics. `--incremental false` was not passed. |
| `npx eslint` on `shadow.ts`, `shadow.test.ts`, `index.ts`, and `index.test.ts` | Those four files | Executed now | Exit 0. No diagnostics. |
| Same vitest command after `>` became `>=` on `Math.abs(left - right) > limit` | Mutated `shadow.ts` sha256 `66f51c108d8702e502bdd5b55d969269f643f89c50770ef5aafa8ca10a2cfb81` | Executed now | Exit 1. 3 failed and 23 passed. AC1 expected 0 and received 1 for `[0]` against `[1]` at limit 1. AC2 expected 0 and received 2. AC4 expected 0 and received 2 for equal bytes at limit 0. AC3 stayed green. `flag.test.ts`, `events.test.ts`, and `index.test.ts` stayed green. |
| `cmp` of the saved original onto `shadow.ts`, then `shasum -a 256` | Pre-mutation `shadow.ts` | Executed now | `cmp` exit 0. sha256 returned to `49eb84526e5fd59f3de4a7d29f4266046e35c9593e6b23ecf1c4c9fda558a001`. `flag.ts` was still `c4daca94e72ace24876443d7c3923dc816e80834b00b7627cb115d34b709753f`. |
| `npx vitest run --config /tmp/f109-shadow-probe.config.mjs` | Probe imported `shadow.ts`, `index.ts`, and `flag.ts` | Executed now | Exit 0. 1 test passed. Empty, limit 255, `NaN`, both orders, count 2, the view, and the shared buffer matched the expectations above. A first config in `/tmp` could not resolve `vitest/config` and was replaced. Both configs and the probe were deleted. |
| `rg` for `addEventListener`, `tolerances.json`, `comparator`, and `@cornerstonejs` | `shadow.ts` and `shadow.test.ts` | Executed now | No match in `shadow.ts`. The only `Uint8Array.from` is the test helper that builds a fixture. |
| `git status --short` on `flag.ts`, `seam.ts`, `alongside.ts`, `events.ts`, and `migrate.ts` | Those paths before the mutation and after the restore | Executed now | No output. They are not modified. |

Environment: Node `v24.13.0`, TypeScript `5.9.3`, Vitest `4.1.11`. No `cargo`, `rustc`, or `wasm-pack` was started.

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

- AC1 counts `[0]` against `[2]` at limit 1 as 1, and `[0]` against `[1]` at limit 1 as 0. The bytes stay `[0]` and `[2]`. Changing `>` to `>=` made that second count 1 and the test went red.
- AC2 counts `[0, 5]` against `[0, 5]` at limit 0 as 0. The same mutation counted both equal bytes and went red.
- AC3 returns null for lengths 1 and 2 at limit 0, and for limits 1.5, 256, and -1. The length pair keeps `[4]` and `[4, 9]`. Limit 0 is a valid integer, so the null on that call is the length refusal.
- AC4 returns 0 for `[0, 10]` against `[0, 10]` at limit 0, then 1 for `[0, 10]` against `[0, 11]`. All three buffers stay unchanged. The mutation failed on the equal call.
- Length is checked at `shadow.ts` lines 14 to 16. The limit is checked at lines 17 to 19. Both happen before the loop. `Number.isInteger` refuses `NaN`, `Infinity`, and 255.5. Limit 255 is accepted. `[0]` against `[255]` at limit 255 counts 0, and the same pair at limit 254 counts 1.
- The absolute difference at line 27 is symmetric. Both argument orders agreed on the probe.
- A count of 2 is returned when two samples are above the limit. The function does not collapse that to a boolean.
- The function writes no index. `shadow.ts` does not call `slice`, `set`, `copyWithin`, or `subarray`. A view's underlying buffer stayed unchanged. Passing one buffer as both arguments stayed unchanged.
- `index.ts` line 243 adds the value export `shadowDivergence` from `./shadow.js`. `index.test.ts` lines 191 to 192 add that name to the hand-written runtime list. The sorted set assertion passed. The probe saw the same function on the package object.
- `events.test.ts` reads every non-test source under `packages/core/src` with `readdirSync`, including this untracked file, and expects the only `addEventListener(` call to be in `pointer.ts`. That file passed.
- `flag.test.ts` passed. `flag.ts` sha256 did not change. `shadow.ts` has no import. Calling the count left stack, MPR, and volume off.
- There is no `as` cast, no `unsafe`, and no `wasm-bindgen`. The plan marks tiers A, B, and C not applicable, and the function has no render path that could disagree by tier.
- The source comment does not claim a section 25.1 tolerance or a corpus verdict.

Each index read is checked for `undefined` before the subtraction, and a miss returns null. `tsconfig.base.json` enables `noUncheckedIndexedAccess`. Every unit and probe buffer was a real `Uint8Array`, and no in-range sample was `undefined`, so that return was not executed.

## Limitations and handoff

This is not the feature gate and not ledger completion. Missing F-109 paragraphs in the LLD are completion work, not a defect in this diff.

`bin/ocelli.sh gate --floor` and the corpus gate were not run. No GPU and no corpus row were used. The undefined index return was not executed, because an in-range `Uint8Array` sample is a number from 0 to 255. This pass did not stage or commit. The next owner can record the LLD paragraphs and the feature gate on this tree.

dwv, Horos, and the medical viewer Grok were not opened.
