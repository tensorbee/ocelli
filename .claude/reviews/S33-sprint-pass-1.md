# S33 review, pass 1

**Reviewed**: Range `7ae7559a9c64fd9f4c6fdd3bde063e26b50ec149..HEAD` on `sprint/s33`. HEAD is `9e10257f1ed76b9adc6c28a7a4401d2bd1fc332d` (F-091). Tree is `a070017e95e706f31efbda536b24d12fb525b598`. `git status --short` was empty before this pass and empty again after the four restores, before this file existed. This review file is not part of that tree.

**Author / reviewer**: `git log` names Atul Sharma on every commit in the range. The author of the implementation is the parent implementation agent. I am independent subagent `01a19e23-5801-78a3-ae6a-c7ae04547739`. I did not write the open, the design, the feature diffs, or the four story reviews. I did not resume `01a19dfc-c6f1-77a1-a62a-d940a472ae5e`, `01a19dfc-c6f1-77a1-a62a-d95a7b4a4438`, `01a19dfc-c6f1-77a1-a62a-d96c6310920e`, or `01a19dfc-c6f1-77a1-a62a-d97f0cbb9a84`. This pass is not a self-review.

**Result**: 0 defects, 0 smells, 0 nitpicks

## Acceptance challenged

The approved plans are the contract. A missing HLD formula was not treated as specified. Cornerstone behaviour was not treated as specified.

F-087. In `packages/core/src/manipulation.ts` the width line was changed from plus to minus. `DRAG_SCALE` stayed 256. `npm test -- packages/core/src/manipulation.test.ts` exited 1. The upward-drag test expected width 401 and received width 399. Centre stayed 41. The non-positive width test also went red and received width 1.5. The saved blob was copied back. `cmp` matched. sha256 returned to `d3e557a0194209b80ae461f3a61b724bd65fff9d4b5c625940086217af56e8c5`. The same test then exited 0 with 7 passed.

F-088. In `packages/core/src/crosshair.ts` the horizontal end was changed from `width` to `width - 1`. `npm test -- packages/core/src/crosshair.test.ts` exited 1. The span test received `x1` 9 and expected `x1` 10. The edge case and the reference-line case received the same 9. The outside case stayed green. The saved blob was copied back. `cmp` matched. sha256 returned to `91bd816188d42ac7893015d5a73a811d95b19f20edf3304e7dc7f828d1287df9`. The same test then exited 0 with 4 passed.

F-090. In `packages/core/src/orientation.ts` the drag line was changed from plus to minus. `npm test -- packages/core/src/orientation.test.ts` exited 1. The 256-pixel drag expected 1 and received -1. The other four tests passed. The saved blob was copied back. `cmp` matched. sha256 returned to `6880f125595e0d3e72182af24ecfe6de397d1fb67e1ef10b1d0933a6e74b5049`. The same test then exited 0 with 5 passed.

F-091. In `packages/core/src/magnify.ts` the loop bound was changed from `step < quadrant` to `step < 0`. `npm test -- packages/core/src/magnify.test.ts` exited 1. Quadrant 1 received `A B C D` and expected `B C D A`. The other five tests passed. The saved blob was copied back. `cmp` matched. sha256 returned to `7bf17030c581109fc6e2bda6816d703477a22789b5626445bba1d5d1111e15bc`. The same test then exited 0 with 6 passed.

One mutation was in place at a time. Each restore used the blob saved before any edit. After the fourth restore the five-file command exited 0 with 26 passed. The tree hash was still `a070017e95e706f31efbda536b24d12fb525b598`. Status was empty.

## Evidence

| Command | Identity | Executed now | Exit and observation |
|---|---|---|---|
| `git rev-parse HEAD`, `git status --short`, `git rev-parse HEAD^{tree}` | Branch `sprint/s33` before any edit | Yes | Exit 0. HEAD `9e10257f1ed76b9adc6c28a7a4401d2bd1fc332d`. Status empty. Tree `a070017e95e706f31efbda536b24d12fb525b598` |
| `git diff --name-only` for the review range | 25 paths. Forbidden names checked by suffix | Yes | Exit 0. `pointer.ts`, `annotation.ts`, `tools.ts`, `commands.ts`, `events.ts` and `cursor.ts` are absent |
| Contributing F-ID line against the public-api row | `docs/lld/public-api.md` header and the last cell of the `public-api.md` row in `docs/lld/README.md` | Yes | The two strings are equal and both end in F-091 |
| Added markdown and commit messages in the range | Code fences excluded from the semicolon scan. TypeScript punctuation was not treated as prose | Yes | No em-dash. No prose semicolon |
| `npm test` on the five named shell files | Clean modules. Vitest 4.1.11 | Yes | Exit 0. 5 files passed. 26 tests passed |
| F-087 width sign flipped to minus, then the manipulation test | Only `manipulation.ts` differed from its blob | Yes | Exit 1. Received width 399 against 401. A second test received width 1.5 against null |
| Blob copy, `cmp`, `shasum -a 256`, manipulation test | Restored `manipulation.ts` | Yes | `cmp` exit 0. Hash `d3e557a0194209b80ae461f3a61b724bd65fff9d4b5c625940086217af56e8c5`. Test exit 0. 7 passed |
| F-088 horizontal end set to `width - 1`, then the crosshair test | Only `crosshair.ts` differed from its blob | Yes | Exit 1. Received `x1` 9 against 10 on the span, the edge and the reference lines. 1 passed |
| Blob copy, `cmp`, `shasum -a 256`, crosshair test | Restored `crosshair.ts` | Yes | `cmp` exit 0. Hash `91bd816188d42ac7893015d5a73a811d95b19f20edf3304e7dc7f828d1287df9`. Test exit 0. 4 passed |
| F-090 drag sign flipped to minus, then the orientation test | Only `orientation.ts` differed from its blob | Yes | Exit 1. Received -1 against 1. 4 passed |
| Blob copy, `cmp`, `shasum -a 256`, orientation test | Restored `orientation.ts` | Yes | `cmp` exit 0. Hash `6880f125595e0d3e72182af24ecfe6de397d1fb67e1ef10b1d0933a6e74b5049`. Test exit 0. 5 passed |
| F-091 loop bound set to `step < 0`, then the magnify test | Only `magnify.ts` differed from its blob | Yes | Exit 1. Quadrant 1 received `A B C D` against `B C D A`. 5 passed |
| Blob copy, `cmp`, `shasum -a 256`, magnify test | Restored `magnify.ts` | Yes | `cmp` exit 0. Hash `7bf17030c581109fc6e2bda6816d703477a22789b5626445bba1d5d1111e15bc`. Test exit 0. 6 passed |
| Node probe of the restored modules | Same four hashes. No file written in the repo | Yes | Exit 0. Printed `PROBE_OK` |
| Five-file `npm test`, four `shasum -a 256`, `git status --short`, `git rev-parse HEAD^{tree}` | After every restore | Yes | Test exit 0. 26 passed. Hashes match the four values above. Status empty. Tree unchanged |

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

`windowDrag` adds `(lastX - originX) / 256` to centre and `(originY - lastY) / 256` to width. `DRAG_SCALE` is 256. A result width that is not greater than 0 returns null. A non-finite input or result returns null. The function does not assign the caller basis. `windowRegion` returns `(min + max) / 2` and `max - min`. An empty list, a non-finite sample, or `min == max` returns null. A probe of samples 0, 0 and 10 returned centre 5 and width 10, which is the mid-span. `stackDrag` is `Math.trunc((originY - lastY) / 256)`. A step of 0 returns null. A negative step is returned. `stackDrag(0, 256)` is -1 in the suite, and a probe of 300 pixels returned -1. The module text has no `panX` and no `Math.exp`. `MANIPULATION_TOOLS` binds Pan, Zoom, WindowLevel, WindowLevelRegion and StackScroll to Pan, Zoom, SetWindow, SetWindow and Scroll. Those command names are already in `STACK_COMMANDS`. The registry test reads them back.

`crosshairLines` returns the horizontal segment from x 0 to the width at y, then the vertical segment from y 0 to the height at x, when the point is inside, the sides are greater than 0, and every input is finite. Inside includes the edges. A probe of the far corner `(10, 8)` on a 10 by 8 view returned both segments and the same cursor point. A point just past the width returned null. `referenceLines` is the same function object as `crosshairLines`. `referenceCursor` returns the point or null. The module has no import and does not project.

`rotationDrag` is `radians + (lastX - originX) / 256`. A non-finite input or result returns null. The F-069 line in `pointer.ts` is the same delta over private `GESTURE_SCALE` 256. That file is not in the sprint diff. `ROTATION_TOOLS` binds TrackballRotate, VolumeRotate and PlanarRotate to Rotate. OrientationController is `orientationQuadrant` and is not a registry row. The quadrant truncates `radians / (PI / 2)` toward zero and wraps with `((raw % 4) + 4) % 4`. Negative `PI / 2` is 3. A probe put a tiny negative angle in 0, `PI` in 2, `-PI` in 2, a full turn in 0, and `3 * PI / 2` in 3. The index is read from a const tuple. The return expression has no `as` cast. The module does not write a camera.

`magnifyCircle` requires a finite radius greater than 0. A null scale is stored as 1. An advanced scale must be finite and greater than 1. Scale 1, radius 0, a negative radius, a non-finite centre, and an infinite scale return null. The non-finite centre refusal is the accepted local guard, and the suite locks a NaN centre. `scaleLength` returns the product. A non-finite factor, a pixels-per-millimetre that is not greater than 0, or a non-finite product returns null. A probe of -10 mm at 2 pixels per millimetre returned -20, which is the product the review rule allows. The module does not read DICOM spacing. The only use of those words is the comment that states the limit. `orientationMarker` leaves quadrant 0 unchanged. Each later exact quadrant moves the first letter to the end. A probe gave quadrant 2 as `C D A B` and quadrant 3 as `D A B C`, and the input array was unchanged. Quadrant 4 and quadrant 8 return null and do not wrap. A length other than 4, or a non-string, returns null.

`index.ts` exports the manipulation, crosshair, orientation and magnify values. `PUBLISHED` names those same runtime values. The five-file run includes that list. There is no `wasm-bindgen` and no `unsafe` in the four modules. They do not open a device and they do not return pixels. Tiers A, B and C are not branched. The plans and the as-built rows mark all three n/a.

The workspace version stays 0.1.0. `package.json` is not in the sprint diff. The `VERSION` literal is not in the `index.ts` diff, and the index test expects `0.1.0`. `CHANGELOG.md` under Unreleased still says nothing is published. Its four new bullets name the magnify circle and marker, the three rotation tools, the crosshair segments, and the five manipulation names. `CURRENT_SPRINT.md` says closing S33 does not complete M10 and does not publish. M10 remains S33 to S34. Backlog rows F-089, F-092 and F-093 stay pending. F-X003 stays in-progress. The backlog diff only changes the four S33 rows to done.

The four feature commits carry verify trailers. The trailers were read. The 27 gates were not re-run. A trailer is not proof of behaviour.

## Limitations and handoff

The sprint gate was not run. No cargo, wasm-pack, or `bin/ocelli.sh` run was started. The approved samples 10, 30 and 20 make the mean and the mid-span the same number. The implementation still walks the minimum and the maximum, and the separate probe above separates them. Quadrants 2 and 3 are locked by that probe and by reading the one loop. They are not separate vitest cases. dwv, Horos, and the medical viewer Grok were not opened.

This file is untracked until the parent stages it. A clean pass is not sprint closure.
