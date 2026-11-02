Result: 1 defect, 0 smells, 0 nitpicks

# S26 sprint review, pass 1

**Reviewed**: `sprint/s26` at `6cb75d26b0507a0436e76616bbc0d4c37fddaa7d` (`F-072, cursor and tool state`). Parent `fc7ed4728c7eaf9e3a195a8fd2356b8ca4c8f8fa` (`F-069, pointer, touch and wheel dispatch`). Design approval `1d1e066868ca2b3591745759cd7fff9da3e9b08a`. Sprint open `6792bcebcfb04f4e5331399968dde225667686c5`. Base `main` is `f1cd800c6c5e7a9f74db66ce40d4ba42521e00f5`. Tag `s25` peels to that merge. The product tree matched HEAD before this file. Two behaviour mutations were restored. This pass was not recorded with `scripts/sprint_workflow.py`.
**Author / reviewer**: Both story commits carry `Ocelli-Generated-By: grok`. A generated trailer does not establish the author. This reviewer did not write F-069 or F-072 and did not resume those review passes.
**Result**: 1 defect, 0 smells, 0 nitpicks

## Acceptance challenged

The contract for the gesture payloads is `.claude/plans/F-069-design.md`. The HLD batches one command buffer per frame and does not give the payload formula. The contract for emission order and cursor choice is `.claude/plans/F-072-design.md`. The HLD does not name cursor strings or when `tool.activated` fires.

`docs/lld/public-api.md` says Pan, Zoom and Rotate pack the basis plus the delta from the gesture origin. That sum was substituted for the zoom arm only. The first frame became factor 257. The committed test expects `Math.exp(-1)`. The restored arm is `basis.zoom * exp(-(lastY - originY) / 256)`. The rotate arm, which this pass did not replace, divides the x delta by 256. The same test expects radians 1 after a 256 pixel move. That test passed in the suite run before the mutation, and the restored hash is the hash of those bytes.

The F-072 bytes match the pass 8 hash, so locked cursor cases were not reopened as product findings. Swapping `tool.activated` ahead of `tool.modeChanged` made five cursor tests fail. `the first active tool does not follow command order` was not one of them. This pass did not replace that selection with a command sort. The bytes were restored.

No committed double-click sample moves the second down away from the first up. This pass did not delete that distance check.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `git rev-parse HEAD`, `main`, `s25^{commit}` | `sprint/s26` | executed now | HEAD `6cb75d26b0507a0436e76616bbc0d4c37fddaa7d`. `main` and `s25` are `f1cd800c6c5e7a9f74db66ce40d4ba42521e00f5`. |
| `git diff --name-only f1cd800...HEAD` and `--stat` | four S26 commits | executed now | 27 files, 4142 insertions, 62 deletions. No `crates/ocelli-render` path, no `cpu.rs`, no oracle producer, no `tools.ts`, no `events.ts`, no `commands.ts`, no version file, no `ci/wasm-size-budget.json`. |
| `sha256` of `cursor.ts` and `pointer.ts` | restored bytes | executed now | `fb1c8ae76c1275c1cd1fccff90dfb92089b61a76108f93304de528e91f11fb1e` and `0b359fd185998ef14c8c6aa963ad85ccdaf516b10852aab502226abff60c0a63`. Both match the story notes. The cursor hash matches F-072 pass 8. |
| `npx vitest run` on `cursor.test.ts`, `pointer.test.ts`, `events.test.ts`, `index.test.ts` | restored HEAD before mutations | executed now | Exit 0. 77 passed. 34 cursor, 31 pointer, 8 events, 4 index. |
| Zoom arm replaced by `basis.zoom + (lastY - originY)` | `pointer.ts` only. Restored after. | executed now | Exit 1. 2 failed, 29 passed. First factor was 257. Expected `0.36787944117144233`. The non-finite zoom test also packed a finite factor. |
| `tool.activated` emitted before `tool.modeChanged` | `cursor.ts` only. Restored after. | executed now | Exit 1. 5 failed, 29 passed. `activation follows the mode change` received `tool.activated` then `tool.modeChanged`. |
| Restored hashes and `git status` | both product files | executed now | Hashes match the values above. Product status was empty before this review file. |
| Ledger keys `acd39944c399578dd1d0fc6f054a7bfc0ec5374c` and `fdc017fa0e1dc5248512d039ad852e895f8ecbb6` | `.claude/verify-ledger.json` | executed now | Both are `profile=feature`, `corpus=pass`, agent `grok`. They are the trees of `fc7ed47` and `6cb75d2`. The gates were not re-run. |

Node v24.13.0. Vitest 4.1.11. No browser and no GPU.

## Defects

### D1, The pointer LLD adds Zoom and does not scale Rotate

**Where**: `docs/lld/public-api.md:153-155`
**What**: The section says Pan, Zoom and Rotate are captured from a caller basis, then says the packed value is the basis plus the delta from the gesture origin. Zoom is not that sum. The committed arm is `basis.zoom * exp(-(lastY - originY) / 256)`. Rotate is `basis.radians + (lastX - originX) / 256`, not the raw pixel delta. Pan is the only one of the three that is the basis plus the raw pixel delta.
**Why it is wrong**: The HLD does not specify these payloads. `.claude/plans/F-069-design.md` is the approved formula. It states the exponential zoom and the divisor 256 for rotation. A public API sentence that states a different formula is a false claim about a camera write that replaces the stored value.
**Evidence**: On HEAD, `zoom and rotate use the gesture origin` passes and expects `Math.exp(-1)` then `Math.exp(-2)` from basis zoom 1 and origin y 0. Replacing only the zoom arm with basis plus the y delta made that test fail. The first command was `{ kind: "Zoom", factor: 257 }`. Expected factor `0.36787944117144233`. `git checkout -- packages/core/src/pointer.ts` restored `0b359fd185998ef14c8c6aa963ad85ccdaf516b10852aab502226abff60c0a63`. The same sentence does not appear in `CHANGELOG.md`, `docs/sprints/AS_BUILT.md` or `docs/sprints/CURRENT_SPRINT.md`.
**Scope and regression**: Present on this HEAD. `packages/core/src/pointer.ts` already follows the plan, so the dispatcher is not the wrong implementation. On the committed zoom fixture the LLD sum is 257 and the plan formula is `exp(-1)`. The smallest change is to replace that one LLD sentence with the three plan formulas. This pass did not submit the packed zoom through `set_zoom`, and it did not substitute the raw pixel delta into the rotate arm.

## Smells

None.

## Nitpicks

None.

## Verified clean

The sprint range is the four commits above. F-069 and F-072 are `done` in `docs/sprints/BACKLOG.md`. F-071 is `pending` and is S27. No annotation or SVG path is in the diff. Closing S26 leaves F-071, so it does not complete M7. `docs/sprints/SPRINT_PLAN.md` says M7 is S25 to S27, five stories and 16 engineer-weeks. The backlog estimates are 3, 3, 4, 3 and 3. F-068 and F-070 stay `done`. F-069 depends on F-068. F-072 depends on F-068. They do not depend on each other. F-X003 remains `in-progress`. Root, core and react versions are `0.1.0`, and those files are not in the diff. `ci/wasm-size-budget.json` is not in the diff. It records 2151196 bytes and tolerance 0.05. `int(2151196 * 1.05)` is 2258755. The D-22 row still says `/release` cannot publish while the OpenJPH notice is missing. That gate was not re-run. `max_review_passes` is written in `scripts/sprint_workflow.py` and this pass found no reader that stops a review loop.

The F-069 tracker cell says three passes at 5/1, 2/0 and 0/0. The three review files say 5 defects and 1 smell, then 2 defects and 0 smells, then 0 and 0. The F-072 cell says eight passes at 0/10, 0/2, 0/4, 0/2, 0/2, 0/2, 0/2 and 0/0. Those files match, and pass 8 is 0 defects, 0 smells and 0 nitpicks. Both tracker cells start with `not measured`. The feature trees in the commit trailers match the ledger keys above.

`createPointerDispatch` keeps one open gesture. A second pointer id sets `sawExtra` and does not replace that gesture. `takeFrame` calls `packCommands` only when the command list is non-empty, and it calls it once. The source has no `submitCommands`, `writeCommands` or `packPointerFrame`. The census test passed, and the only non-test `addEventListener` file under `packages/core/src` and `packages/react/src` is `pointer.ts`. `events.ts` is unchanged and remains the catalogue. Touch uses the touch names. Mouse and pen fall through to the pointer names. A wheel is summed in `wheelStepOf` and clamped once in `addI32`. The restored suite includes the order-independent clamp cases. A second down after `pointerup` starts a new gesture because `gesture` is null. The click comparison is `< 3`. Engagement is `>= 3`.

`cursor.ts` does not import `pointer.ts` and does not call `addEventListener`. `createToolState` reads the group, calls `registry.setMode`, and emits only when that call is ok and the subject was present. The subject `tool.modeChanged` is first. `tool.activated` follows it and only for a new active mode. Demoted names are walked in `result.value.demoted`. `createCursorController` copies `COMMAND_CURSORS` and replaces pairs on the copy. One chosen tool supplies both phases. Primary is any binding whose button is `primary`. Otherwise the first active tool in the returned list is used. The pass 8 hash did not change, and the swapped-event mutation went red on five tests before the restore.

The cursor paragraphs in `docs/lld/public-api.md` match that restored source. The pointer paragraphs match it except for D1.

## Limitations and handoff

D1 blocks this pass. The product hashes above are the restored HEAD hashes. This review file is not part of that tree. Do not attach this pass to a later edit of the LLD sentence. A new independent pass has to see the corrected sentence.

No `bin/ocelli.sh` run, no wasm build, and no browser. The feature ledger rows were read. Their gates were not repeated. Packed zoom was not applied to `View::set_zoom`. Packed scroll was not applied to `scroll_by`.

The committed double-click tests use the same point for both clicks. Deleting the 3 CSS pixel check in `doubleCandidate` was not tried in this pass. F-069 pass 3 reported a deleted probe where a second down exactly 3 pixels away was not a double click. That probe is not in the tree. This is not filed again. The code on HEAD still uses `< 3`.

F-072 pass 8 left two active primary owners unspecified. The bytes match that pass, so this pass did not file the tie. A listener that calls `setMode` again from inside the callback was not exercised. The plan does not define that re-entry.

Pen was not given its own sample in this pass. The branch treats `pointerType === "pen"` as pen and every other non-touch type as mouse, and both use the pointer event names.
