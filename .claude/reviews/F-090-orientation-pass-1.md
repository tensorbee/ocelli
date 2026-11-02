# F-090 review, pass 1

**Reviewed**: uncommitted working tree on `sprint/s33`, HEAD `43a894b`. Read `.claude/plans/F-090-design.md`, `packages/core/src/orientation.ts`, `packages/core/src/orientation.test.ts`, the F-090 paragraph and contributed-ID line in `docs/lld/public-api.md`, the public-api row in `docs/lld/README.md`, the orientation export in `packages/core/src/index.ts`, the matching `PUBLISHED` names in `packages/core/src/index.test.ts`, `ToolBinding` in `packages/core/src/viewport.ts`, `registerTool` and `group` in `packages/core/src/tools.ts`, the F-069 rotate line and `GESTURE_SCALE` in `packages/core/src/pointer.ts`, and `STACK_COMMANDS` in `packages/core/src/generated.ts`.
**Author / reviewer**: Author is the parent implementation agent. Reviewer is independent subagent `01a19dfc-c6f1-77a1-a62a-d96c6310920e` and did not write the diff.
**Result**: 0 defects, 0 smells, 0 nitpicks

## Acceptance challenged

AC1. `rotationDrag(0, 0, 256)` is 1. A non-zero origin, `rotationDrag(0, 10, 266)`, is also 1. Basis 0.25 with no pixel delta stays 0.25. A drag of 256 left, `rotationDrag(0, 256, 0)`, is -1. The sign is the F-069 plus.

AC2. A NaN basis, a NaN origin, a NaN last point, an Infinity basis, an Infinity last point, and the `Number.MAX_VALUE` pair all return null. The permanent test refuses the NaN basis, the NaN last point, and the overflow pair.

AC3. `Math.PI / 2` is quadrant 1. `Math.PI / 2 - 1e-12` is quadrant 0, so the cut is truncation toward zero rather than a round up.

AC4. `-Math.PI / 2` divided by `Math.PI / 2` is exactly -1, and the quadrant is 3. A tiny negative angle, `-1e-10`, stays quadrant 0. `-Math.PI` is 2. `-3 * Math.PI / 2` is 1. A full turn in either direction is 0. Infinity and NaN are null.

AC5. The three bindings are TrackballRotate, VolumeRotate and PlanarRotate, each with command Rotate. The registry readback is that same list. OrientationController is `orientationQuadrant` and is not an entry in `ROTATION_TOOLS`.

## Evidence

| Command | What | Exit and observation |
|---|---|---|
| `npm test -- packages/core/src/orientation.test.ts` | Plus form, before the mutation. Vitest 4.1.11. | Exit 0. 5 passed. |
| Same command | Drag sign changed from plus to minus. | Exit 1. AC1 failed. Received -1, expected 1. The other four passed. |
| `shasum -a 256 packages/core/src/orientation.ts` | After the plus form was restored. | `6880f125595e0d3e72182af24ecfe6de397d1fb67e1ef10b1d0933a6e74b5049`, the same hash as before the edit. |
| `node --experimental-strip-types` import of `orientation.ts` | The cases in Acceptance challenged. | 0 mismatches. |
| `git diff --name-only` | Working tree against HEAD. | `pointer.ts` is absent. `git status` does not list it either. |

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

`rotationDrag` is `radians + (lastX - originX) / 256`. `DRAG_SCALE` is 256, the same value as the private `GESTURE_SCALE`. The F-069 line is `source.basis.radians + (source.lastX - source.originX) / GESTURE_SCALE`. `pointer.ts` was not edited.

A non-finite input returns null, and a non-finite result returns null. The overflow fixture uses two finite inputs whose difference is not finite, so that fixture is what rejects a missing result check.

`orientationQuadrant` divides by `Math.PI / 2`, truncates toward zero, then applies `((raw % 4) + 4) % 4`. The quarter value is the same `Math.PI / 2` the caller uses. Finite angles exercised here, including both full turns and a value just under one quarter, land in 0, 1, 2 or 3.

`ROTATION_TOOLS` binds the three names to the existing Rotate command. Rotate is one of `STACK_COMMANDS`. `registerTool` stores that command, and the test reads it back from the group. There is no second rotate command.

The module imports only the `ToolBinding` type. It does not write a camera and does not build a matrix. A search of the file finds those words only in the comment that states the limit.

`index.ts` publishes `ROTATION_TOOLS`, `orientationQuadrant` and `rotationDrag`. Those are the three F-090 strings in `PUBLISHED`. The LLD paragraph states the F-069 delta, the three Rotate bindings, the toward-zero wrap, quadrant 3 for a negative quarter turn, that OrientationController is not a registry tool, and that the module does not write a camera. The README public-api row names rotation names, an orientation quadrant, and F-090.

The mutated line was restored to `radians + (lastX - originX) / DRAG_SCALE`. The sha256 matches the hash taken before the edit.

## Limitations and handoff

`index.test.ts` was read and not executed. The orientation test imports `./orientation.js` directly. The extra cases ran from a node import of that module after the restore, not from a second vitest file. No test file other than `orientation.test.ts` ran while the minus sign was in place. No cargo, wasm-pack, or `bin/ocelli.sh` run. The mutation is not left in the tree.
