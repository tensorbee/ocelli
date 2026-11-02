Result: 0 defects, 4 smells, 0 nitpicks

# F-072 review, pass 3

**Reviewed**: Working tree on `sprint/s26` against HEAD `fc7ed4728c7eaf9e3a195a8fd2356b8ca4c8f8fa`. Untracked `packages/core/src/cursor.ts` and `packages/core/src/cursor.test.ts`. Modified `packages/core/src/index.ts`, `packages/core/src/index.test.ts`, and the reproducer sentence in `.claude/plans/F-072-design.md`. `cursor.ts` sha256 `fb1c8ae76c1275c1cd1fccff90dfb92089b61a76108f93304de528e91f11fb1e` matches the bytes pass 2 restored. `cursor.test.ts` is `949916b37ede40342cabcdc0991d6c08cfc94b2a21d9f93470b4b652004b700b`. `index.ts` is `6d1536120fbd61800861d87801f4004d7af2e4e77bdb8cd92da537d7408ceba8`. `index.test.ts` is `d9c67a80977f96cb3fdf985d25e53d793bf666c13f4efc598506d359ad05fddd`. Mutations were restored. `cmp` against the pre-mutation copy was identical. `git diff` does not include `tools.ts`, `pointer.ts`, `events.ts`, or `commands.ts`.
**Author / reviewer**: The author is the implementer of this uncommitted tree. This reviewer did not write the code and did not resume reviewer 01a19a88 or 01a19a71.
**Result**: 0 defects, 4 smells, 0 nitpicks

## Acceptance challenged

The contract is `.claude/plans/F-072-design.md`. Cursor strings and emission order are local decisions in that file. No HLD formula specifies them.

Pass 2 S1 is closed. `tool.activated` only when the previous mode is `off` failed `leaving active does not activate` at the passive-to-active expect, before the splice. The names were `tool.modeChanged` alone. The test requires `tool.modeChanged` then `tool.activated`, with previous `passive`. The later active-to-off expect is one `tool.modeChanged` and no `tool.activated`.

Pass 2 S2 is closed. `options.registry.group("g")` failed both new lookups. `an unknown group uses the default cursor` has an active Pan in group `g` and a controller for `missing`. Idle was `grab`, expected `default`. `the controller reads its group` has active Zoom in group `other`. Idle was `grab`, expected `zoom-in`.

Sorting demoted names failed `demotion keeps registration order`. The events put `a` before `n`. Choosing `active[0]` and ignoring primary failed `primary owns the cursor`. Received `grab`. Expected `zoom-in`.

Four wrong edits stayed green. They are the smells. An override can keep Pan's built-in dragging cursor. The cursor table can be the shared `COMMAND_CURSORS` object. A stored mode change that does not touch `active` can emit nothing. A subject `tool.modeChanged` can carry command `Zoom` when the new mode is not `active`.

On the restored bytes a separate probe ran the plan reproducer. Pan active on primary, then Zoom active on primary, emitted Zoom `tool.modeChanged` from `off` to `active`, then `tool.activated` for Zoom, then Pan `tool.modeChanged` from `active` to `passive`. `sync` idle wrote `zoom-in`.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `npx vitest run packages/core/src/cursor.test.ts packages/core/src/index.test.ts packages/core/src/events.test.ts` | Tree before mutations. Not a receipt. | Executed now | Exit 0. 35 passed. 23 cursor, 4 index, 8 events. |
| `npx tsc --build` | Same tree, before mutations | Executed now | Exit 0. |
| `npx eslint` on `cursor.ts`, `cursor.test.ts`, `index.ts`, `index.test.ts` | Same tree, before mutations | Executed now | Exit 0. |
| `tool.activated` only when previous mode is `off` | `cursor.ts` only. Restored after. | Executed now | `-t "leaving active does not activate"` exit 1. Line 124. Names were `tool.modeChanged` only. |
| `options.registry.group("g")` | `cursor.ts` only. Restored after. | Executed now | Exit 1. Unknown group idle was `grab`, expected `default`. Group `other` idle was `grab`, expected `zoom-in`. |
| Demoted names sorted | `cursor.ts` only. Restored after. | Executed now | `-t "demotion keeps registration order"` exit 1. `a` before `n`. |
| `chosen = active[0]` | `cursor.ts` only. Restored after. | Executed now | `-t "primary owns the cursor"` exit 1. Received `grab`. Expected `zoom-in`. |
| Override stores idle only | `cursor.ts` only. Restored after. | Executed now | Full `cursor.test.ts` exit 0. 23 passed. `tsc --build` exit 0. Probe dragging was `grabbing`. |
| Table is `COMMAND_CURSORS` with no spread and no cast | `cursor.ts` only. Restored after. | Executed now | Full `cursor.test.ts` exit 0. 23 passed. `tsc --build` exit 0. Probe second controller and the export were `move`. |
| Emit only when one side is `active` | `cursor.ts` only. Restored after. | Executed now | Full `cursor.test.ts` exit 0. 23 passed. Probe off to passive and passive to off emitted nothing. |
| Subject command is `Zoom` unless the new mode is `active` | `cursor.ts` only. Restored after. | Executed now | Full `cursor.test.ts` exit 0. 23 passed. Probe command was `Zoom`. |
| Same vitest command, `npx tsc --build`, and eslint on the four files | After the last restore. `cursor.ts` hash matched. | Executed now | All three exit 0. 35 passed. |

Node v24.13.0. Vitest 4.1.11. TypeScript 5.9.3. ESLint 9.39.5. The `vite-node` probes lived under `/tmp` and were deleted. They did not change the tree.

## Defects

None.

## Smells

### S1, an override can keep the built-in dragging cursor

**Where**: `packages/core/src/cursor.ts:165`. The idle expect is `packages/core/src/cursor.test.ts:412`.
**What**: The replacement stored `pair.idle` and the built-in dragging string. All 23 cursor tests passed, and `tsc --build` passed. Pan overridden to idle `move` and dragging `move` returned idle `move` and dragging `grabbing`.
**Why it is wrong**: The plan says an override replaces one command and leaves the rest. Dragging selects that command's dragging cursor. The restored assignment stores the whole pair. The override test reads idle only, then turns Pan off and checks Zoom.
**Evidence**: Full `npx vitest run packages/core/src/cursor.test.ts` exit 0. `npx tsc --build` exit 0. The probe printed `idle move` and `dragging grabbing`. The export stayed `grab` and `grabbing`. Restored afterward. The hash matched.
**Scope and regression**: A drag after that override still shows `grabbing`. Smallest regression is `cursor("dragging")` equals `move` in `an override replaces one cursor`, before Pan is turned off. Do not change `cursor.ts`.

### S2, an override can write through to the shared default table

**Where**: `packages/core/src/cursor.ts:160`. The write is the assignment at `packages/core/src/cursor.ts:165`.
**What**: The spread was removed, so the table was the exported `COMMAND_CURSORS` object. `const table: Record<StackCommand, CursorPair> = COMMAND_CURSORS` typechecks. All 23 cursor tests passed. A second controller with no override returned `move` for both phases. The export's Pan pair was `move` and `move`.
**Why it is wrong**: An override replaces one command for that controller and leaves the defaults other controllers read. The restored line copies the record before the replacement. No test builds a second controller after an override. `each command has its cursor` runs before the override test, so it still sees the original Pan pair.
**Evidence**: `npx tsc --build` exit 0. Full `cursor.test.ts` exit 0. The probe printed `first move move`, `second move move`, and `export move move`. Restored afterward. The hash matched.
**Scope and regression**: One viewport's Pan override changes every later controller and the published constant. Smallest regression is a second controller with no `cursors` option, plus `COMMAND_CURSORS.Pan`, both still `grab` and `grabbing` after the first controller's override. Do not remove the spread.

### S3, a mode change that does not touch active can be silent

**Where**: `packages/core/src/cursor.ts:115`. No test calls these two transitions.
**What**: The subject guard also required `subject.mode === "active" || mode === "active"`. All 23 cursor tests passed. `setMode` from `off` to `passive`, and from `passive` to `off`, returned ok and emitted nothing.
**Why it is wrong**: The plan says `tool.modeChanged` fires when the subject's stored mode changes. Both calls change the stored mode. Neither new mode is `active`, so neither emits `tool.activated`. The restored inequality emits one `tool.modeChanged` for each, with command `Pan`. Every listened transition in the suite has `active` on one side, so the narrower guard still passes.
**Evidence**: Full `cursor.test.ts` exit 0. The mutated probe printed `off-to-passive true (none)` and `passive-to-off true (none)`. The same probe on the restored file printed `tool.modeChanged:Pan:Pan:off:passive` and `tool.modeChanged:Pan:Pan:passive:off`. Restored afterward. The hash matched.
**Scope and regression**: A listener that tracks mode from events keeps the old mode while the registry has changed. Smallest regression is those two wrapper calls, each one `tool.modeChanged` and no `tool.activated`, with previous taken from the before-image. Do not change the inequality.

### S4, the subject command can be wrong when the new mode is not active

**Where**: `packages/core/src/cursor.ts:120`. The leave expects are `packages/core/src/cursor.test.ts:121` and `packages/core/src/cursor.test.ts:143`.
**What**: The subject `tool.modeChanged` used `subject.command` only when the new mode was `active`. Otherwise the command was `Zoom`. All 23 cursor tests passed. A Pan tool moving from `off` to `passive`, and from `passive` to `off`, emitted command `Zoom`.
**Why it is wrong**: The event fields are name, groupId, tool, command, mode, and previous. Command is that tool's command. `leaving active does not activate` reads mode and previous on the way to `passive` and on the way to `off`. It does not read command. Activation expects do read command, and those calls use mode `active`, so they stayed green.
**Evidence**: Full `cursor.test.ts` exit 0. The probe printed `tool.modeChanged:Pan:Zoom:off:passive` and `tool.modeChanged:Pan:Zoom:passive:off`. Restored afterward. The hash matched.
**Scope and regression**: The active-to-passive and active-to-off calls already in that test use the same expression and omit the same field. Smallest regression is command `Pan` on both of those expects. Do not change the command expression. A full field check on the S3 calls would also reject this ternary for those two transitions.

## Nitpicks

None.

## Verified clean

Pass 2's two smells are locked by the tests added since that pass. The product file hash did not change. The restored source still emits `tool.activated` for a subject whose new mode is `active`, including from `passive`, and it still reads `options.groupId`.

On the restored bytes the wrapper reads the group, calls `registry.setMode`, and emits only when that call is ok. A refusal returns the registry outcome and emits nothing. An identical `setMode` emits nothing. A binding change that keeps the subject active emits no subject event and no `tool.activated`. It emits one `tool.modeChanged` per demoted name, in the returned order. The subject pair is `tool.modeChanged` then `tool.activated` only when the stored mode changes to `active`. `previous` comes from the before-image. Demoted events use the demoted tool's command and mode `passive`. Sorting that list failed. Ignoring primary failed.

`registerTool` and `removeTool` are forwarded. The registration test expects no events. A direct `registry.setMode` test expects no events. This pass did not mutate those two paths again.

The cursor table matches the eleven `STACK_COMMANDS`. Pan is `grab` and `grabbing`. Zoom is `zoom-in` for both phases. Rotate and SetWindow are `crosshair`. Scroll is `ns-resize`. Fit, FlipHorizontal, FlipVertical, ApplyPreset, SetInvert, and SetColormap are `default`. Primary beats an earlier active tool. With no primary owner, registration order beats a sorted name. A passive tool does not own the cursor. An unknown group is `default` for both phases, and `sync` writes that string. `sync` assigns `style.cursor` on every call. There is no `addEventListener` and no `pointer.ts` or `pointer.js` import. The `as StackCommand[]` on `Object.keys` only walks the override keys. A stored command is one of the eleven, so an excess key cannot be selected.

No wasm, no pixels, and no `unsafe`. Tiers A, B, and C are not applicable. The plan says so, and the file has no tier branch. No new trait or generic was added.

`cursor.test.ts` imports `CursorController`, `CursorControllerOptions`, `CursorPair`, `CursorPhase`, `CursorStyle`, `ToolState`, and `ToolStateEvent` from `./index.js`. The value names `COMMAND_CURSORS`, `CURSOR_PHASES`, `createCursorController`, and `createToolState` stay on the exact key list in `index.test.ts`. This pass did not delete an export. `tsc --build` passed with those imports present.

`events.test.ts` passed. `events.ts` was not edited. The catalogue names are still `tool.activated` and `tool.modeChanged`, with shell delivery. This module emits those names and does not write a ring record.

The reproducer sentence matches the restored probe. It is not a false claim.

## Limitations and handoff

S1 to S4 are missing assertions. The restored product already follows the plan on all four. Remediation should add tests only. It should not change the emission order, the cursor table, the group lookup, the spread, or the demotion rule.

This pass did not repeat the pass 2 mutations that already went red and whose expects are still in the file. Those are demoted command, blank group id, listener rollback, sync write count, passive filter, unknown-group `grab`, pointer import, refusal emit, read after `setMode`, and `tool.activated` before `tool.modeChanged`. AC5, AC6, AC9, and AC12 were not mutated again.

The active-to-off expect does not name groupId or tool. That split was not mutated. It is not a separate finding. Command on that call is S4.

A binding whose button is `primary` still counts as owning primary when modifiers are present. Pass 1 and pass 2 did not call that a defect. The plan does not pass modifiers into `cursor` or `sync`. This pass does not reopen it.

`docs/lld/public-api.md` is unchanged. The plan defers that section until the story is completed. It was not in this diff.

`tools.test.ts` and `pointer.test.ts` were not run. Those sources are not in the diff. No cargo, wasm-pack, or `bin/ocelli.sh` run. No commit and no stage. This is a bounded review of the F-072 tree, not sprint acceptance.
