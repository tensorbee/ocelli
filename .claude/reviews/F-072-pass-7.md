Result: 0 defects, 2 smells, 0 nitpicks

# F-072 review, pass 7

**Reviewed**: Working tree on `sprint/s26` against HEAD `fc7ed4728c7eaf9e3a195a8fd2356b8ca4c8f8fa`. Untracked `packages/core/src/cursor.ts` and `packages/core/src/cursor.test.ts`. Modified `packages/core/src/index.ts`, `packages/core/src/index.test.ts`, and the reproducer sentence in `.claude/plans/F-072-design.md`. `cursor.ts` sha256 `fb1c8ae76c1275c1cd1fccff90dfb92089b61a76108f93304de528e91f11fb1e` matches the bytes this pass started from and the bytes restored after mutation. `cmp` against the pre-mutation copy was identical. `cursor.test.ts` is `3e0d5df3c43e081cc041c97902e4c0f2b73becd94ee77346d0b7f3d5aa589550`. `index.ts` is `6d1536120fbd61800861d87801f4004d7af2e4e77bdb8cd92da537d7408ceba8`. `index.test.ts` is `d9c67a80977f96cb3fdf985d25e53d793bf666c13f4efc598506d359ad05fddd`. The plan file is `4c54d58aa35d5208a7e4e6a1b2f6a2fd6196ef29ad3367006f0067d8dc73fcf8`. `git diff` does not include `tools.ts`, `pointer.ts`, `events.ts`, or `commands.ts`.
**Author / reviewer**: The author is the implementer of this uncommitted tree. This reviewer did not write the code and did not resume reviewers 01a19ac6, 01a19ab6, 01a19aa7, 01a19a97, 01a19a88, or 01a19a71.
**Result**: 0 defects, 2 smells, 0 nitpicks

## Acceptance challenged

The contract is `.claude/plans/F-072-design.md`. Cursor strings and emission order are local decisions in that file. No HLD formula specifies them.

Pass 6 S1 is closed. Falling back to the first active tool when the primary cursor is `default` failed `a later primary default cursor is kept` at `packages/core/src/cursor.test.ts:453`. Idle was `grab`. Expected `default`. The same edit also failed `a later modified primary keeps a default cursor` at line 491. A probe of that edit printed `grab`, `grabbing`, `grab`, and `grabbing` for both scenarios. The restored probe printed `default` four times for both.

Pass 6 S2 is closed. Accepting a modified primary only in slot 0, or an unmodified primary in any slot, failed `a later modified primary still owns the cursor` at `packages/core/src/cursor.test.ts:472`. Idle was `grab`. Expected `zoom-in`. It also failed the default-cursor case at line 491. The zoom probe printed `grab`, `grabbing`, `grab`, and `grabbing`. The restored probe printed `zoom-in` four times.

Ignoring primary only when it is not slot 0, has a modifier, and its cursor is `default` failed only `a later modified primary keeps a default cursor`. The other 30 cursor tests passed. The probe kept `default` for Pan then Fit on an unmodified primary, and `zoom-in` for Pan then Zoom on auxiliary then primary plus shift. Only Pan then Fit on auxiliary then primary plus shift became `grab`, `grabbing`, `grab`, and `grabbing`.

Using only `bindings[0]` failed `primary can sit after another binding`, plus the two later modified-primary tests. Requiring an empty modifier list failed `primary with a modifier still owns the cursor`, plus those same two tests. Skipping a `default` cursor when nobody owns primary failed `the first active default cursor is kept` at line 402. Idle was `grab`. Expected `default`. Sorting the no-primary fallback by tool name failed `the first active tool owns the cursor` at line 372. Idle was `zoom-in`. Expected `grab`.

Taking the dragging cursor from the first active tool, while idle stays on the primary owner, failed six tests. `primary owns the cursor` received `grabbing` and expected `zoom-in`.

Two further wrong edits stayed green. They are the smells below. The restored source does not contain either edit. Zoom registered before Pan, with no primary owner, is `zoom-in` for both phases and both `sync` writes. A demotion whose returned names are `a` then `n` emits `a` before `n`.

On the restored bytes the plan reproducer matched the probe. The first call emits Pan `tool.modeChanged` from `off` to `active`, then `tool.activated` for Pan. The sentence is about the second call. That call emits Zoom `tool.modeChanged` from `off` to `active`, then `tool.activated` for Zoom, then Pan `tool.modeChanged` from `active` to `passive`. `sync` with phase `idle` writes `zoom-in`.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `npx vitest run` on `cursor.test.ts`, `index.test.ts`, and `events.test.ts` | Tree before mutations, and again after the last restore. Not a receipt. | Executed now | Exit 0 both times. 43 passed. 31 cursor, 4 index, 8 events. |
| `npx tsc --build` | Same trees | Executed now | Exit 0 both times. |
| `npx eslint` on `cursor.ts`, `cursor.test.ts`, `index.ts`, `index.test.ts` | Same trees | Executed now | Exit 0 both times. |
| Primary `default` falls back to the first active tool | `cursor.ts` only. Restored after. | Executed now | Full `cursor.test.ts` exit 1. 2 failed. Idle was `grab` at lines 453 and 491. Probe for both failing setups was `grab`, `grabbing`, `grab`, `grabbing`. |
| Modified primary counts only in slot 0, or when it has no modifiers | `cursor.ts` only. Restored after. | Executed now | Full suite exit 1. 2 failed. Lines 472 and 491 received `grab`. Zoom probe was `grab`, `grabbing`, `grab`, `grabbing`. |
| Primary is ignored only when it is not slot 0, has a modifier, and its cursor is `default` | `cursor.ts` only. Restored after. | Executed now | Full suite exit 1. 1 failed, line 491, received `grab`. The other two new scenarios stayed `default` and `zoom-in` in the probe. |
| `tool.activated` is emitted before `tool.modeChanged` | `cursor.ts` only. Restored after. | Executed now | Full suite exit 1. 4 failed. `activation follows the mode change` received `tool.activated` then `tool.modeChanged`. |
| A refusal emits `tool.modeChanged` | `cursor.ts` only. Restored after. | Executed now | Full suite exit 1. `a refusal is silent` received one event. Expected none. |
| The cursor table is `COMMAND_CURSORS` with no spread | `cursor.ts` only. Restored after. | Executed now | Full suite exit 1. `an override replaces one cursor` line 575. The other controller's idle was `move`. Expected `grab`. |
| The group is read after `setMode` | `cursor.ts` only. Restored after. | Executed now | Full suite exit 1. 7 failed. `activation follows the mode change` received no events. |
| A listener throw restores the before-image | `cursor.ts` only. Restored after. | Executed now | Full suite exit 1. After the throw, Pan was `off` with no bindings. Expected `active` on primary. |
| No-primary fallback is sorted by command | `cursor.ts` only. Restored after. | Executed now | Full suite exit 0, including `--no-cache`. 31 passed. Probe for Zoom then Pan was `grab`, `grabbing`, `grab`, `grabbing`. |
| Demoted names are sorted by command | `cursor.ts` only. Restored after. | Executed now | Full suite exit 0, including `--no-cache`. 31 passed. Probe return stayed `["a", "n"]`. Events put `n` before `a`. |
| No-primary fallback is sorted by tool name | `cursor.ts` only. Restored after. | Executed now | Full suite exit 1. `the first active tool owns the cursor` received `zoom-in`. Expected `grab`. |
| No-primary fallback skips a `default` cursor | `cursor.ts` only. Restored after. | Executed now | Full suite exit 1. `the first active default cursor is kept` received `grab`. Expected `default`. |
| Primary owner is `bindings[0]` only | `cursor.ts` only. Restored after. | Executed now | Full suite exit 1. `primary can sit after another binding` received `grab`. Expected `zoom-in`. |
| Primary requires `modifiers.length === 0` | `cursor.ts` only. Restored after. | Executed now | Full suite exit 1. `primary with a modifier still owns the cursor` received `grab`. Expected `zoom-in`. |
| Dragging comes from the first active tool | `cursor.ts` only. Restored after. | Executed now | Full suite exit 1. 6 failed. `primary owns the cursor` dragging was `grabbing`. Expected `zoom-in`. |
| Source contains the text `addEventListener` | `cursor.ts` only. Restored after. | Executed now | Full suite exit 1. `the cursor module does not listen` expected `false`. |
| Restored reproducer and order probes | `tsc --build` output of the restored source | Executed now | Second call and `sync` matched the plan sentence. Zoom before Pan was `zoom-in` four times. Demotion events were `a` then `n`. |

Node v24.13.0. Vitest 4.1.11. TypeScript 5.9.3. ESLint 9.39.5. The probe script lived under `/tmp` and was not part of the tree. Mutations were restored before the closing checks.

## Defects

None.

## Smells

### S1, a command sort can replace registration order

**Where**: `packages/core/src/cursor.ts:179`. The expect that stayed green is `packages/core/src/cursor.test.ts:372`, in `the first active tool owns the cursor`.
**What**: With no primary owner, sorting the active tools by command and taking the first left all 31 cursor tests green. Zoom registered on auxiliary, then Pan on secondary, returned `grab` and `grabbing`. Both `sync` calls wrote those strings.
**Why it is wrong**: The plan says that if nobody owns `primary`, the first active tool in registration order wins. AC8 names a sorted name as the wrong implementation. `the first active tool owns the cursor` registers Pan before Zoom. `the first active default cursor is kept` registers Fit before Pan. In both fixtures the command order is the registration order, so a command sort cannot be seen. A tool-name sort does go red, because that test registers `Z` before `A`.
**Evidence**: Full `npx vitest run packages/core/src/cursor.test.ts --no-cache` exit 0. 31 passed. The probe printed `idle grab`, `dragging grabbing`, `syncIdle grab`, and `syncDragging grabbing`. The restored probe printed `zoom-in` four times. Restored afterward. The hash matched. The edit was:

```ts
const fallback = active
  .slice()
  .sort((left, right) => left.command.localeCompare(right.command))[0];
const chosen = primary ?? fallback;
```

**Scope and regression**: A later tool whose command sorts first would take the cursor from the tool registered before it. Smallest regression is Zoom on auxiliary, then Pan on secondary, with idle and dragging both `zoom-in`, and both `sync` writes `zoom-in`. Do not change `active[0]`.

### S2, demotion events can follow command order

**Where**: `packages/core/src/cursor.ts:135`. The expect that stayed green is `packages/core/src/cursor.test.ts:106`, in `demotion keeps registration order`.
**What**: Sorting `result.value.demoted` by command left all 31 cursor tests green. The in-file fixture registers Pan before Zoom, so the sort does not move those names. A probe registered Zoom as `a` and then Pan as `n`. The registry still returned `["a", "n"]`. The events after the subject were `n` then `a`.
**Why it is wrong**: The plan says demoted names follow the returned order. AC2 rejects sorting the names. `demotion keeps registration order` registers `n` with Pan before `a` with Zoom. Pan sorts before Zoom, so that returned list is already command order. The tool names `n` then `a` reject a name sort. They do not reject a command sort.
**Evidence**: Full `cursor.test.ts --no-cache` exit 0. 31 passed. The probe returned `{ demoted: ["a", "n"] }` and then emitted `tool.modeChanged:n:Pan` before `tool.modeChanged:a:Zoom`. The restored probe emitted `a` before `n`. Restored afterward. The hash matched. The edit sorted the demoted names by `command` before the listener loop.
**Scope and regression**: A listener would see the alphabetically earlier command first while `setMode` still reports the other order. Smallest regression is `a` on Zoom and `n` on Pan, then one tool taking both of those bindings. After the subject events, the demoted events are `a` then `n`, matching `["a", "n"]`. Do not sort the loop.

## Nitpicks

None.

## Verified clean

Pass 6's two smells are locked by the tests added since that pass. The product file hash did not change. A later unmodified primary whose cursor is `default` now fails. A later modified primary now fails. The conjunction, a later modified primary whose cursor is `default`, fails on its own. The restored source still chooses one tool and then that command's phase string.

On the restored bytes the wrapper reads the group, calls `registry.setMode`, and emits only when that call is ok. Subject events come first. `tool.modeChanged` fires only when the stored mode changes. `tool.activated` fires only for the subject, only when the new mode is `active`, and only after `tool.modeChanged`. Reading the group after the write suppressed the subject events. Swapping the two subject names reversed the order. A refusal that still emits failed `a refusal is silent`. A listener throw that writes the before-image back left Pan `off`. There is no second mode store.

The cursor table has the same eleven commands as `STACK_COMMANDS`. Pan is `grab` and `grabbing`. Zoom is `zoom-in` for both phases. Rotate and SetWindow are `crosshair`. Scroll is `ns-resize`. Fit, FlipHorizontal, FlipVertical, ApplyPreset, SetInvert, and SetColormap are `default`. An override that aliases the exported table makes the other controller report `move`. Replacing one pair on the restored source leaves the other controller and `COMMAND_CURSORS.Pan` on `grab` and `grabbing`. One chosen tool supplies both phases. Sending dragging to the first active tool failed. No active tool, including a passive-only tool, is `default`. An unknown group is `default` for both phases, and `sync` writes that string. The caller passes the phase. Inserting the text `addEventListener` failed the source guard. There is no `pointer.ts` or `pointer.js` import.

`cursor.test.ts` imports `CursorController`, `CursorControllerOptions`, `CursorPair`, `CursorPhase`, `CursorStyle`, `ToolState`, and `ToolStateEvent` from `./index.js`. It imports `COMMAND_CURSORS` from `./index.js` as well. `packages/core/tsconfig.json` includes `src/**/*`, so those type imports are in `tsc --build`. The value names `COMMAND_CURSORS`, `CURSOR_PHASES`, `createCursorController`, and `createToolState` stay on the exact key list in `index.test.ts`. This pass did not delete an export.

`events.test.ts` passed. `events.ts` was not edited. The catalogue names are still `tool.activated` and `tool.modeChanged`, with shell delivery. This module emits those names and does not write a ring record.

The reproducer sentence matches the restored second call and the idle `sync`. It is not a false claim. The first call also emits, and the sentence does not say otherwise. No wasm, no pixels, and no `unsafe`. The only `as` cast walks override keys. A stored command is one of the eleven, so an excess key cannot be selected. Tiers A, B, and C are not applicable. The plan says so, and the file has no tier branch. No new trait or generic was added. `tools.ts` was not edited.

## Limitations and handoff

S1 and S2 are missing assertions. The restored product already follows the plan on both. Remediation should add tests only. It should not change the emission order, the cursor table, the group lookup, the spread, the chosen expression, the primary predicate, or the demotion loop.

F-068 lets primary and primary-plus-shift both stay active. The restored probe kept Pan on primary plus shift and Zoom on unmodified primary, with `demoted` empty, and the cursor was `grab` and `grabbing`. No cursor test builds two primary owners. The plan does not say which of them wins. This pass did not file that tie-break.

This pass did not repeat every edit that already went red in an earlier pass. The owner-matrix cases run now, and not refiled, are a primary-only slot, an unmodified primary in a later slot, a modified primary in slot 0, Fit on primary registered first, Fit first with no primary, and the first active tool by name when nobody owns primary. Identical `setMode`, `registerTool`, `removeTool`, and a direct `registry.setMode` were not mutated again. Their expects are still in the file.

Vitest stops at the first bad expect inside one test. The dragging and sync lines in the three lock tests were covered by the probe, not by a second vitest failure line.

`docs/lld/public-api.md` is unchanged. The plan defers that section until the story is completed. It was not in this diff.

`tools.test.ts` and `pointer.test.ts` were not run. Those sources are not in the diff. No cargo, wasm-pack, or `bin/ocelli.sh` run. No commit and no stage. This is a bounded review of the F-072 tree, not sprint acceptance.
