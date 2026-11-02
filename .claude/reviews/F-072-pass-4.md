Result: 0 defects, 2 smells, 0 nitpicks

# F-072 review, pass 4

**Reviewed**: Working tree on `sprint/s26` against HEAD `fc7ed4728c7eaf9e3a195a8fd2356b8ca4c8f8fa`. Untracked `packages/core/src/cursor.ts` and `packages/core/src/cursor.test.ts`. Modified `packages/core/src/index.ts`, `packages/core/src/index.test.ts`, and the reproducer sentence in `.claude/plans/F-072-design.md`. `cursor.ts` sha256 `fb1c8ae76c1275c1cd1fccff90dfb92089b61a76108f93304de528e91f11fb1e` matches the bytes pass 3 restored. `cursor.test.ts` is `f5e9191871dea04b55a18a644fa4f4b468c2d341383002ad9f2519d11d750330`. `index.ts` is `6d1536120fbd61800861d87801f4004d7af2e4e77bdb8cd92da537d7408ceba8`. `index.test.ts` is `d9c67a80977f96cb3fdf985d25e53d793bf666c13f4efc598506d359ad05fddd`. Mutations were restored. `cmp` against the pre-mutation copy was identical. `git diff` does not include `tools.ts`, `pointer.ts`, `events.ts`, or `commands.ts`.
**Author / reviewer**: The author is the implementer of this uncommitted tree. This reviewer did not write the code and did not resume reviewers 01a19a97, 01a19a88, or 01a19a71.
**Result**: 0 defects, 2 smells, 0 nitpicks

## Acceptance challenged

The contract is `.claude/plans/F-072-design.md`. Cursor strings and emission order are local decisions in that file. No HLD formula specifies them.

Pass 3 S1 is closed. Storing `pair.idle` and the built-in dragging string failed `an override replaces one cursor` at the dragging expect. Received `grabbing`. Expected `move`.

Pass 3 S2 is closed. Assigning `COMMAND_CURSORS` with no spread failed that same test at the second controller. Idle was `move`. Expected `grab`. `npx tsc --build` still exited 0. A probe on that edit also showed dragging `move` for the second controller, and `COMMAND_CURSORS.Pan` imported from `index.ts` was `{ idle: "move", dragging: "move" }`. Vitest stopped before the export expect at `cursor.test.ts:449`.

Pass 3 S3 is closed. Emitting only when one side is `active` failed `a mode change that is not active still emits`. Off to passive returned ok and emitted nothing. Expected one `tool.modeChanged`. A probe on the same edit showed passive to off also returned ok and emitted nothing. Vitest stopped before that expect.

Pass 3 S4 is closed. A non-active subject command of `Zoom` failed `leaving active does not activate` and the new test. Active to passive carried `Zoom`. Off to passive carried `Zoom`. A probe on the same edit showed active to off and passive to off also carried `Zoom`. Those two expects were not the vitest failure lines.

Choosing `active[0]` and ignoring primary failed `primary owns the cursor`. Received `grab`. Expected `zoom-in`. Writing `style.cursor` only when the string changes failed `sync writes the style`. The counted setter stayed at 0. Expected 2.

Two wrong edits stayed green. They are the smells. Dragging can follow a different active tool than idle. A primary `Fit` can be skipped so an earlier `Pan` supplies `grab`.

On the restored bytes a separate probe ran the plan reproducer. Pan active on primary, then Zoom active on primary, emitted Zoom `tool.modeChanged` from `off` to `active`, then `tool.activated` for Zoom, then Pan `tool.modeChanged` from `active` to `passive`. `sync` idle wrote `zoom-in`. The same probe with Pan on secondary and Zoom on primary read `zoom-in` for both phases. Pan on secondary and Zoom on auxiliary read `grab` and `grabbing`. Pan on secondary and Fit on primary read `default` for both phases, and both `sync` calls wrote `default`.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `npx vitest run packages/core/src/cursor.test.ts packages/core/src/index.test.ts packages/core/src/events.test.ts` | Tree before mutations. Not a receipt. | Executed now | Exit 0. 36 passed. 24 cursor, 4 index, 8 events. |
| `npx tsc --build` | Same tree, before mutations | Executed now | Exit 0. |
| `npx eslint` on `cursor.ts`, `cursor.test.ts`, `index.ts`, `index.test.ts` | Same tree, before mutations | Executed now | Exit 0. |
| Override stores idle only | `cursor.ts` only. Restored after. | Executed now | `-t "an override replaces one cursor"` exit 1. Dragging was `grabbing`. Expected `move`. |
| Table is `COMMAND_CURSORS` with no spread | `cursor.ts` only. Restored after. | Executed now | Same test exit 1. Second controller idle was `move`. Expected `grab`. `tsc --build` exit 0. Probe export was `move` and `move`. |
| Emit only when one side is `active` | `cursor.ts` only. Restored after. | Executed now | `-t "a mode change that is not active still emits"` exit 1. Received no events. Probe passive to off also had no events. |
| Subject command is `Zoom` unless the new mode is `active` | `cursor.ts` only. Restored after. | Executed now | Exit 1. Both named tests failed. Active to passive and off to passive carried `Zoom`. Probe active to off and passive to off carried `Zoom`. |
| `chosen = active[0]` | `cursor.ts` only. Restored after. | Executed now | `-t "primary owns the cursor"` exit 1. Received `grab`. Expected `zoom-in`. |
| `sync` assigns only when the string changes | `cursor.ts` only. Restored after. | Executed now | `-t "sync writes the style"` exit 1. Writes 0. Expected 2. |
| Dragging uses `active[0]` and idle keeps primary | `cursor.ts` only. Restored after. | Executed now | Full `cursor.test.ts` exit 0. 24 passed. Probe dragging was `grabbing`. |
| Dragging with no primary uses the last active tool | `cursor.ts` only. Restored after. | Executed now | Full `cursor.test.ts` exit 0. 24 passed. Probe dragging was `zoom-in`. |
| A `default` cursor falls through to another active tool | `cursor.ts` only. Restored after. | Executed now | Full `cursor.test.ts` exit 0. 24 passed. Probe idle and sync were `grab`. |
| Same vitest command, `npx tsc --build`, and eslint on the four files | After the last restore. `cursor.ts` hash matched. | Executed now | All three exit 0. 36 passed. |

Node v24.13.0. Vitest 4.1.11. TypeScript 5.9.3. ESLint 9.39.5. The `vite-node` probes lived under `/tmp` and were deleted. They did not change the tree.

## Defects

None.

## Smells

### S1, dragging can follow a different tool than idle

**Where**: `packages/core/src/cursor.ts:179`. The idle expects are `packages/core/src/cursor.test.ts:356` and `packages/core/src/cursor.test.ts:368`.
**What**: The chosen tool was `active[0]` for `dragging` and the primary owner otherwise. All 24 cursor tests passed. Pan on secondary and Zoom on primary stayed `zoom-in` at idle and returned `grabbing` while dragging. A second edit used the last active tool for dragging when nobody owned primary. All 24 cursor tests passed. Pan on secondary and Zoom on auxiliary stayed `grab` at idle and returned `zoom-in` while dragging.
**Why it is wrong**: The plan says primary wins, and otherwise the first active tool in registration order wins. Dragging selects that command's dragging cursor. Both owner tests read idle only. Every dragging expect has one active tool, so a phase can change the owner without going red.
**Evidence**: Full `npx vitest run packages/core/src/cursor.test.ts` exit 0 for each edit. 24 passed. The probes printed `idle zoom-in` / `dragging grabbing`, then `idle grab` / `dragging zoom-in`. The same probes on the restored file printed `dragging zoom-in` and `dragging grabbing`. Restored afterward. The hash matched.
**Scope and regression**: A drag shows the other tool while idle shows the owner. Smallest regression is `cursor("dragging")` equals `zoom-in` in `primary owns the cursor`, and `cursor("dragging")` equals `grabbing` in `the first active tool owns the cursor`. Do not change the chosen expression.

### S2, a default cursor can be skipped for another active tool

**Where**: `packages/core/src/cursor.ts:183`. The single-tool rows are `packages/core/src/cursor.test.ts:307`.
**What**: When the chosen command's cursor was `default` and another tool was active, the controller returned the other tool. All 24 cursor tests passed. Fit on primary and Pan on secondary returned `grab` and `grabbing`. Both `sync` calls wrote those strings.
**Why it is wrong**: Primary still wins when the command is Fit, FlipHorizontal, FlipVertical, ApplyPreset, SetInvert, or SetColormap. Those pairs are `default`. The command loop checks each of them alone, and an idle group is also `default`, so dropping that owner stays green. The restored source returns `default` for both phases and both sync calls.
**Evidence**: Full `cursor.test.ts` exit 0. 24 passed. The probe printed `idle grab`, `dragging grabbing`, `sync-idle grab`, and `sync-dragging grabbing`. The restored probe printed `default` four times. Restored afterward. The hash matched.
**Scope and regression**: A window or flip tool would show Pan's cursor while Pan stays active on another button. Smallest regression is Pan on secondary and Fit on primary, with idle and dragging both `default`. Do not change the return.

## Nitpicks

None.

## Verified clean

Pass 3's four smells are locked by the tests added since that pass. The product file hash did not change. The restored source still stores the whole override pair, still copies `COMMAND_CURSORS` before replacement, still emits `tool.modeChanged` when the stored mode changes and neither side is `active`, and still puts that tool's command on the event.

On the restored bytes the wrapper reads the group, calls `registry.setMode`, and emits only when that call is ok. A refusal returns the registry outcome and emits nothing. The refusal probe covered `unknown-tool`, `unknown-group`, and `bindings-only-when-active`, with no events. An identical `setMode` emits nothing. A binding change that keeps the subject active emits no subject event and no `tool.activated`. It emits one `tool.modeChanged` per demoted name, in the returned order. The subject pair is `tool.modeChanged` then `tool.activated` only when the stored mode changes to `active`, including from `passive`. `previous` and `command` come from the before-image. Demoted events use the demoted tool's command and mode `passive`. Ignoring primary failed. Skipping an unchanged `sync` write failed.

`registerTool` and `removeTool` are forwarded. The registration test expects no events. A direct `registry.setMode` test expects no events. The refusal probe's direct `setMode` also emitted nothing. This pass did not mutate those two paths again.

The cursor table matches the eleven `STACK_COMMANDS`. Pan is `grab` and `grabbing`. Zoom is `zoom-in` for both phases. Rotate and SetWindow are `crosshair`. Scroll is `ns-resize`. Fit, FlipHorizontal, FlipVertical, ApplyPreset, SetInvert, and SetColormap are `default`. Primary beats an earlier active tool at idle. With no primary owner, registration order beats a sorted name at idle. A passive tool does not own the cursor. An unknown group is `default` for both phases, and `sync` writes that string. `sync` assigns `style.cursor` on every call. There is no `addEventListener` and no `pointer.ts` or `pointer.js` import. The `as StackCommand[]` on `Object.keys` only walks the override keys. A stored command is one of the eleven, so an excess key cannot be selected.

No wasm, no pixels, and no `unsafe`. Tiers A, B, and C are not applicable. The plan says so, and the file has no tier branch. No new trait or generic was added. `tools.ts` was not edited, so there is no second mode store.

`cursor.test.ts` imports `CursorController`, `CursorControllerOptions`, `CursorPair`, `CursorPhase`, `CursorStyle`, `ToolState`, and `ToolStateEvent` from `./index.js`. It imports `COMMAND_CURSORS` from `./index.js` as well. The value names `COMMAND_CURSORS`, `CURSOR_PHASES`, `createCursorController`, and `createToolState` stay on the exact key list in `index.test.ts`. This pass did not delete an export. `tsc --build` passed with those imports present.

`events.test.ts` passed. `events.ts` was not edited. The catalogue names are still `tool.activated` and `tool.modeChanged`, with shell delivery. This module emits those names and does not write a ring record.

The reproducer sentence matches the restored probe. It is not a false claim.

## Limitations and handoff

S1 and S2 are missing assertions. The restored product already follows the plan on both. Remediation should add tests only. It should not change the emission order, the cursor table, the group lookup, the spread, the chosen expression, or the demotion rule.

This pass did not repeat the earlier mutations that already went red and whose expects are still in the file. Those include demoted command, blank group id, listener rollback, passive filter, unknown-group `grab`, pointer import, refusal emit, read after `setMode`, and `tool.activated` before `tool.modeChanged`. AC5, AC6, AC9, AC12, and AC13 were not mutated again.

Vitest stops at the first bad expect inside one test. The passive-to-off expect and the active-to-off expect were covered by the probe, not by a second vitest failure line. The export expect in the override test was the same kind of later line. The second-controller failure is what locked the shared table.

A binding whose button is `primary` still counts as owning primary when modifiers are present. Earlier passes did not call that a defect. The plan does not pass modifiers into `cursor` or `sync`. This pass does not reopen it.

The published `createCursorController` and `createToolState` values were not called through `index.ts`. The name list locks them, and `COMMAND_CURSORS.Pan` is read from that module. Replacing the function exports with non-functions was not tried.

`docs/lld/public-api.md` is unchanged. The plan defers that section until the story is completed. It was not in this diff.

`tools.test.ts` and `pointer.test.ts` were not run. Those sources are not in the diff. No cargo, wasm-pack, or `bin/ocelli.sh` run. No commit and no stage. This is a bounded review of the F-072 tree, not sprint acceptance.
