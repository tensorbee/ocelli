Result: 0 defects, 2 smells, 0 nitpicks

# F-072 review, pass 5

**Reviewed**: Working tree on `sprint/s26` against HEAD `fc7ed4728c7eaf9e3a195a8fd2356b8ca4c8f8fa`. Untracked `packages/core/src/cursor.ts` and `packages/core/src/cursor.test.ts`. Modified `packages/core/src/index.ts`, `packages/core/src/index.test.ts`, and the reproducer sentence in `.claude/plans/F-072-design.md`. `cursor.ts` sha256 `fb1c8ae76c1275c1cd1fccff90dfb92089b61a76108f93304de528e91f11fb1e` matches the bytes this pass started from and the bytes restored after mutation. `cmp` against the pre-mutation copy was identical. `cursor.test.ts` is `e98b31a55ca5c79cd8a0edd0c2f57f6538598c8d919eb60f9df0b9ef5716e432`. `index.ts` is `6d1536120fbd61800861d87801f4004d7af2e4e77bdb8cd92da537d7408ceba8`. `index.test.ts` is `d9c67a80977f96cb3fdf985d25e53d793bf666c13f4efc598506d359ad05fddd`. The plan file is `4c54d58aa35d5208a7e4e6a1b2f6a2fd6196ef29ad3367006f0067d8dc73fcf8`. `git diff` does not include `tools.ts`, `pointer.ts`, `events.ts`, or `commands.ts`.
**Author / reviewer**: The author is the implementer of this uncommitted tree. This reviewer did not write the code and did not resume reviewers 01a19aa7, 01a19a97, 01a19a88, or 01a19a71.
**Result**: 0 defects, 2 smells, 0 nitpicks

## Acceptance challenged

The contract is `.claude/plans/F-072-design.md`. Cursor strings and emission order are local decisions in that file. No HLD formula specifies them.

Pass 4 S1 is closed. For `dragging`, choosing `active[0]` instead of the primary owner failed `primary owns the cursor` at `cursor.test.ts:357`. Received `grabbing`. Expected `zoom-in`. Idle stayed `zoom-in`. A probe on that edit showed `sync("dragging")` wrote `grabbing`, not `zoom-in`. Vitest stopped before the sync expect at `cursor.test.ts:361`.

Pass 4 S1's other arm is closed. Choosing the last active tool for `dragging` when nobody owns primary failed `the first active tool owns the cursor` at `cursor.test.ts:373`. Received `zoom-in`. Expected `grabbing`. Idle stayed `grab`. The probe showed `sync("dragging")` wrote `zoom-in`, not `grabbing`. Vitest stopped before `cursor.test.ts:375`.

Pass 4 S2 is closed. Falling through from a `default` cursor to another active tool failed `a default cursor is not skipped` at `cursor.test.ts:386`. Fit on primary and Pan on secondary returned `grab` at idle. Expected `default`. The probe showed dragging `grabbing`, and both `sync` writes were `grab` and `grabbing`. Vitest stopped before those expects.

Two further wrong edits stayed green. They are the smells below. The restored source does not contain either edit. Fit registered first on secondary, with Pan later on auxiliary, is `default` for both phases and both `sync` writes. Pan on secondary and Zoom with auxiliary then primary is `zoom-in` for both phases and both `sync` writes.

Reading the group after `setMode` failed `activation follows the mode change`. The call returned ok and emitted nothing. Expected `tool.modeChanged` then `tool.activated`. Emitting `tool.activated` before `tool.modeChanged` failed that same test. Received `tool.activated` then `tool.modeChanged`.

On the restored bytes the plan reproducer matched the probe. Pan active on primary, then Zoom active on primary, emitted Zoom `tool.modeChanged` from `off` to `active`, then `tool.activated` for Zoom, then Pan `tool.modeChanged` from `active` to `passive`. `sync` idle wrote `zoom-in`.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `npx vitest run packages/core/src/cursor.test.ts packages/core/src/index.test.ts packages/core/src/events.test.ts` | Tree before mutations. Not a receipt. | Executed now | Exit 0. 37 passed. 25 cursor, 4 index, 8 events. |
| `npx tsc --build` | Same tree, before mutations | Executed now | Exit 0. |
| `npx eslint` on `cursor.ts`, `cursor.test.ts`, `index.ts`, `index.test.ts` | Same tree, before mutations | Executed now | Exit 0. |
| Dragging uses `active[0]` and idle keeps primary | `cursor.ts` only. Restored after. | Executed now | `-t "primary owns the cursor"` exit 1. Dragging was `grabbing`. Expected `zoom-in`. Probe sync dragging was `grabbing`. |
| Dragging with no primary uses the last active tool | `cursor.ts` only. Restored after. | Executed now | `-t "the first active tool owns the cursor"` exit 1. Dragging was `zoom-in`. Expected `grabbing`. Probe sync dragging was `zoom-in`. |
| A `default` cursor falls through to another active tool | `cursor.ts` only. Restored after. | Executed now | `-t "a default cursor is not skipped"` exit 1. Idle was `grab`. Expected `default`. Probe dragging and both sync writes followed Pan. |
| Group is read after `setMode` | `cursor.ts` only. Restored after. | Executed now | `-t "activation follows the mode change"` exit 1. Received no events. Expected `tool.modeChanged` then `tool.activated`. |
| `tool.activated` is emitted before `tool.modeChanged` | `cursor.ts` only. Restored after. | Executed now | Same test exit 1. Received `tool.activated` then `tool.modeChanged`. |
| Primary owner is `bindings[0]` only | `cursor.ts` only. Restored after. | Executed now | Full `cursor.test.ts` exit 0. 25 passed. Probe for auxiliary then primary was `grab` and `grabbing`. |
| No-primary fallback skips a `default` cursor | `cursor.ts` only. Restored after. | Executed now | Full `cursor.test.ts` exit 0. 25 passed. Probe for Fit then Pan with no primary was `grab` and `grabbing`. |
| Same vitest command, `npx tsc --build`, and eslint on the four files | After the last restore. `cursor.ts` hash matched. `cmp` was identical. | Executed now | All three exit 0. 37 passed. |

Node v24.13.0. Vitest 4.1.11. TypeScript 5.9.3. ESLint 9.39.5. The Vite probes lived under `/tmp` and were deleted. They did not change the tree.

## Defects

None.

## Smells

### S1, a default cursor can be skipped when nobody owns primary

**Where**: `packages/core/src/cursor.ts:179`. The idle expects that stayed green are `packages/core/src/cursor.test.ts:372` and `packages/core/src/cursor.test.ts:386`.
**What**: With no primary owner, the fallback skipped an active tool whose cursor was `default` and used the next active tool. All 25 cursor tests passed. Fit registered first on secondary, and Pan later on auxiliary, returned `grab` and `grabbing`. Both `sync` calls wrote those strings.
**Why it is wrong**: The plan says that if nobody owns primary, the first active tool in registration order wins for both phases. Fit's pair is `default` on both phases. `the first active tool owns the cursor` uses Pan then Zoom, so the first cursor is not `default`. `a default cursor is not skipped` puts Fit on primary, so this fallback never runs.
**Evidence**: Full `npx vitest run packages/core/src/cursor.test.ts` exit 0. 25 passed. The probe printed `idle grab`, `dragging grabbing`, `syncIdle grab`, and `syncDragging grabbing`. The same probe on the restored file printed `default` four times. Restored afterward. The hash matched. The edit was:

```ts
const fallback =
  active.find((item) => {
    const pair = table[item.command];
    const value = phase === "dragging" ? pair.dragging : pair.idle;
    return value !== "default";
  }) ?? active[0];
const chosen = primary ?? fallback;
```

**Scope and regression**: A window, flip, or fit tool that is the only early active tool would show a later tool's cursor while dragging. Smallest regression is Fit on secondary and Pan on auxiliary, with idle and dragging both `default`, and both `sync` writes `default`. Do not change the chosen expression.

### S2, primary ownership can ignore every binding after the first

**Where**: `packages/core/src/cursor.ts:176`. The owner expect that stayed green is `packages/core/src/cursor.test.ts:356`.
**What**: Primary ownership used `bindings[0]` instead of any binding whose button is `primary`. All 25 cursor tests passed. Pan on secondary, then Zoom active on auxiliary and primary, returned `grab` and `grabbing`. Both `sync` calls wrote those strings.
**Why it is wrong**: The plan says the active tool that owns primary wins, including over an earlier tool. F-068 stores the whole binding list on that tool at `packages/core/src/tools.ts:379`. Auxiliary then primary is one active owner of primary. Every current cursor fixture puts `primary` in the only slot, or does not use it, so `bindings[0]` still passes.
**Evidence**: Full `cursor.test.ts` exit 0. 25 passed. The probe printed `idle grab`, `dragging grabbing`, `syncIdle grab`, and `syncDragging grabbing`. The restored probe printed `zoom-in` four times. Restored afterward. The hash matched. The edit was `item.bindings[0]?.button === "primary"` in place of the `some` predicate.
**Scope and regression**: A tool bound to auxiliary and primary would leave the cursor on an earlier tool. Smallest regression is Pan on secondary and Zoom on auxiliary then primary, with idle and dragging both `zoom-in`, and both `sync` writes `zoom-in`. Do not change the predicate.

## Nitpicks

None.

## Verified clean

Pass 4's two smells are locked by the assertions added since that pass. The product file hash did not change. The restored source still chooses one tool and then that command's phase string. Dragging cannot follow `active[0]` while idle follows primary. Dragging cannot follow the last active tool when nobody owns primary. A chosen `default` cursor is not replaced by another active tool. Fit on primary and Pan on secondary stays `default` for idle, dragging, and both `sync` writes.

On the restored bytes the wrapper reads the group, calls `registry.setMode`, and emits only when that call is ok. Subject events come first. `tool.modeChanged` fires only when the stored mode changes. `tool.activated` fires only for the subject, only when the new mode is `active`, and only after `tool.modeChanged`. That pair is emitted from `off` and from `passive`. Demoted names then follow the returned order, each as `tool.modeChanged` to `passive`, with `command` and `previous` from the before-image. A move from `active` to `passive` or `off` does not activate. An identical `setMode` emits nothing. A binding change that keeps the subject active emits no subject event. A refusal emits nothing. `registerTool` and `removeTool` emit nothing. A direct `registry.setMode` emits nothing. Reading the group after the write suppressed the subject events. Swapping the two subject names reversed the order. There is no second mode store.

The cursor table matches the eleven `STACK_COMMANDS`. Pan is `grab` and `grabbing`. Zoom is `zoom-in` for both phases. Rotate and SetWindow are `crosshair`. Scroll is `ns-resize`. Fit, FlipHorizontal, FlipVertical, ApplyPreset, SetInvert, and SetColormap are `default`. An override replaces one whole pair for that controller and leaves `COMMAND_CURSORS.Pan` at `grab` and `grabbing`. A second controller without the override still reads `grab` and `grabbing`. No active tool, including a passive-only tool, is `default`. An unknown group is `default` for both phases, and `sync` writes that string. `sync` assigns `style.cursor` on every call. The caller passes the phase. There is no `addEventListener` and no `pointer.ts` or `pointer.js` import.

`cursor.test.ts` imports `CursorController`, `CursorControllerOptions`, `CursorPair`, `CursorPhase`, `CursorStyle`, `ToolState`, and `ToolStateEvent` from `./index.js`. It imports `COMMAND_CURSORS` from `./index.js` as well. `packages/core/tsconfig.json` includes `src/**/*`, so those type imports are in `tsc --build`. The value names `COMMAND_CURSORS`, `CURSOR_PHASES`, `createCursorController`, and `createToolState` stay on the exact key list in `index.test.ts`. This pass did not delete an export.

`events.test.ts` passed. `events.ts` was not edited. The catalogue names are still `tool.activated` and `tool.modeChanged`, with shell delivery. This module emits those names and does not write a ring record.

The reproducer sentence matches the restored probe. It is not a false claim. No wasm, no pixels, and no `unsafe`. Tiers A, B, and C are not applicable. The plan says so, and the file has no tier branch. No new trait or generic was added. `tools.ts` was not edited.

## Limitations and handoff

S1 and S2 are missing assertions. The restored product already follows the plan on both. Remediation should add tests only. It should not change the emission order, the cursor table, the group lookup, the spread, the chosen expression, the primary predicate, or the demotion rule.

This pass did not repeat the pass 4 mutations that already went red. Those are an override that stores idle only, a table assigned without a spread, emission only when one side is `active`, a subject command forced to `Zoom`, `chosen = active[0]` for both phases, and `sync` writing only when the string changes. It also did not repeat demoted command, blank group id, listener rollback, the passive filter, unknown-group `grab`, the pointer import, or a refusal emit. The read-after and activated-before edits above are the two extra discriminating cases, and both went red.

Vitest stops at the first bad expect inside one test. The `sync("dragging")` line in `primary owns the cursor`, the sync line in `the first active tool owns the cursor`, and the dragging and sync lines in `a default cursor is not skipped` were covered by the probe, not by a second vitest failure line.

A binding whose button is `primary` still counts as owning primary when modifiers are present. Earlier passes did not call that a defect. This pass does not reopen it. The new S2 is a different hole. It is a later slot in the binding list, not a modifier on the first slot.

The published `createCursorController` and `createToolState` values were not called through a `typeof` check. The name list locks them, and `COMMAND_CURSORS.Pan` is read from `index.ts`. Replacing the function exports with non-functions was not tried. This pass did not edit `index.ts`.

`docs/lld/public-api.md` is unchanged. The plan defers that section until the story is completed. It was not in this diff.

`tools.test.ts` and `pointer.test.ts` were not run. Those sources are not in the diff. No cargo, wasm-pack, or `bin/ocelli.sh` run. No commit and no stage. This is a bounded review of the F-072 tree, not sprint acceptance.
