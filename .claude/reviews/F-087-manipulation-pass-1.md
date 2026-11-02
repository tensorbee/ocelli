# F-087 review, pass 1

**Reviewed**: uncommitted working tree on `sprint/s33`. HEAD is `43a894b` (`S33, approve the manipulation designs`). Read `.claude/plans/F-087-design.md`, `packages/core/src/manipulation.ts`, `packages/core/src/manipulation.test.ts`, the F-087 paragraph in `docs/lld/public-api.md`, the manipulation exports in `packages/core/src/index.ts`, the matching `PUBLISHED` names in `packages/core/src/index.test.ts`, the public-api row in `docs/lld/README.md`, `STACK_COMMANDS` in `packages/core/src/generated.ts`, `ToolBinding` in `packages/core/src/viewport.ts`, `registerTool` in `packages/core/src/tools.ts`, and the pan and zoom expressions in `packages/core/src/pointer.ts`.
**Author / reviewer**: Author is the parent implementation agent. Reviewer is independent subagent `01a19dfc-c6f1-77a1-a62a-d940a472ae5e` and did not write the diff.
**Result**: 0 defects, 0 smells, 0 nitpicks

## Acceptance challenged

AC1. Basis centre 40 and width 400. Origin is 0, 0. Last point is 256, -256. Centre is `40 + (256 - 0) / 256`, which is 41. Width is `400 + (0 - -256) / 256`, which is 401. `windowDrag` returns a new object. It does not assign `basis.centre` or `basis.width`. The test holds the same basis and expects 40 and 400 after the call.

AC2. Width 0.5 with last point 256, 256. Width is `0.5 + (0 - 256) / 256`, which is -0.5. That is not greater than 0, so the result is null and the basis stays 40 and 0.5. Width 1 with a 256-pixel downward drag becomes 0 and is also null. The second basis stays 40 and 1.

AC3. Samples 10, 30 and 20. The walk keeps minimum 10 and maximum 30. Centre is `(10 + 30) / 2`, which is 20. Width is `30 - 10`, which is 20. The function does not divide the sum by the count.

AC4. An empty list has no first sample and returns null. A list containing `NaN` returns null from `Number.isFinite`. The pair 7, 7 has `max === min` and returns null through `!(max > min)`. A NaN centre returns null before a result is built. Origin X `-Number.MAX_VALUE` and last X `Number.MAX_VALUE` make a non-finite centre and return null.

AC5. `stackDrag(0, -256)` is `Math.trunc((0 - -256) / 256)`, which is 1. `stackDrag(0, -255)` is `Math.trunc(255 / 256)`, which is 0, so it returns null. A rounded step does not pass.

AC6. `stackDrag(4, 4)` is 0 and returns null. A NaN endpoint returns null. `stackDrag(0, 256)` is -1. The plan refuses only a step of 0, so a negative step is returned. `stackDrag(Number.MAX_VALUE, -Number.MAX_VALUE)` is not finite and returns null.

AC7. `MANIPULATION_TOOLS` is Pan to Pan, Zoom to Zoom, WindowLevel to SetWindow, WindowLevelRegion to SetWindow, and StackScroll to Scroll. A new registry accepts each binding. The group read-back is those five pairs in that order. `registerTool` refuses a command that is not in `STACK_COMMANDS`. Pan, Zoom, SetWindow and Scroll are already in that list. There is no new command kind. The same test reads `manipulation.ts` and requires that it does not contain `panX` or `Math.exp`.

The width line was changed from addition to subtraction and `npm test -- packages/core/src/manipulation.test.ts` was run again. AC1 went red. The received value was centre 41 and width 399. The expected width was 401. The original plus line was restored before this file was written.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `git status --short` and `git diff --name-only` | Working tree on `sprint/s33` at HEAD `43a894b` | Executed now | Exit 0. Modified shared files are `docs/lld/README.md`, `docs/lld/public-api.md`, `packages/core/src/index.ts` and `packages/core/src/index.test.ts`. Untracked product files include `manipulation.ts` and `manipulation.test.ts`, plus the other S33 modules. `pointer.ts`, `tools.ts`, `commands.ts` and `annotation.ts` are absent from both outputs |
| `npm test -- packages/core/src/manipulation.test.ts` | Restored-before-mutation source. sha256 `d3e557a0194209b80ae461f3a61b724bd65fff9d4b5c625940086217af56e8c5` | Executed now | Exit 0. Vitest 4.1.11. 1 file, 7 tests passed |
| Same npm command after one edit | Width line uses subtraction. No other test file was run | Executed now | Exit 1. 2 failed and 5 passed. AC1 at `manipulation.test.ts:24` expected width 401 and received 399. The basis assertion on the next line did not run. AC2 expected null and received centre 41 and width 1.5 |
| `cp` of the pre-edit copy, then `cmp` and `shasum -a 256` | `/tmp/f087-manipulation.ts.orig` against `packages/core/src/manipulation.ts` | Executed now | `cmp` exit 0. Both hashes are `d3e557a0194209b80ae461f3a61b724bd65fff9d4b5c625940086217af56e8c5`. Line 50 is the plus form. A search for the minus form found nothing |
| Repeated `git status --short` and `git diff --name-only` for the four untouched paths | After the restore | Executed now | Both empty |

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

`windowDrag` checks `basis.centre`, `basis.width`, `originX`, `originY`, `lastX` and `lastY` with `Number.isFinite`. Centre is `basis.centre + (lastX - originX) / 256`. Width is `basis.width + (originY - lastY) / 256`. A non-finite result or a width that is not greater than 0 returns null. The basis fields are not written. The parameter marks those fields readonly. A caller object of the plan's shape is still accepted. The return shape is `{ centre, width } | null`.

`windowRegion` walks the caller array only. Centre is `(min + max) / 2`. Width is `max - min`. An empty list, a non-finite sample, a span of 0, or a non-finite result returns null. It does not read pixels and it does not apply a VOI LUT.

`stackDrag` returns `Math.trunc((originY - lastY) / 256)`. `Math.trunc` is toward zero. A step of 0 or a non-finite input or step returns null. A negative step is kept.

`DRAG_SCALE` is the literal 256. It is not imported from `pointer.ts`. The pan expression in `pointer.ts` is `panX + (lastX - originX)` and `panY + (lastY - originY)` with no scale. The zoom expression is `zoom * Math.exp(-(lastY - originY) / 256)`. Neither expression is in `manipulation.ts`. The source check for `panX` and `Math.exp` ran in the passing suite. There is no canvas point type and no patient point type in this module. The inputs are plain numbers.

The five names use existing command kinds. `packages/core/src/index.ts` exports `MANIPULATION_TOOLS`, `stackDrag`, `windowDrag` and `windowRegion`. `PUBLISHED` lists those four names under the F-087 comment and no other name from this module. The F-087 paragraph in `docs/lld/public-api.md` states the same centre, width, mid-span, truncation, refusal, registry and non-LUT rules. That paragraph has no em-dash and no prose semicolon. The public-api row in `docs/lld/README.md` names window and stack drags and includes F-087. The F-ID list on that row matches the header list in `public-api.md`. `pointer.ts`, `tools.ts`, `commands.ts` and `annotation.ts` were not modified.

## Limitations and handoff

This pass ran only `packages/core/src/manipulation.test.ts`. It did not run `index.test.ts`, lint, typecheck, cargo, wasm-pack, `bin/ocelli.sh`, a gate, the oracle or the corpus. The mean of 10, 30 and 20 is also 20, so that one AC3 assertion would stay green if the centre were the mean and the width stayed `max - min`. The implementation uses the minimum and the maximum, and those samples are the samples in the approved plan. A 255-pixel downward stack drag is not in the fixture. `Math.trunc` still moves that value toward zero. Shared diffs also contain F-088, F-090 and F-091. Those stories were not reviewed. No patient data was read. dwv, Horos, the medical viewer named Grok, and cornerstone source were not opened.

The width mutation was restored. The product file matches the pre-edit bytes. This review file is the only new file left by the pass. It was not committed. No blocking finding. A clean bounded review is not the feature profile and is not sprint readiness.
