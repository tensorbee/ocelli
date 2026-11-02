Result: 0 defects, 0 smells, 0 nitpicks

# F-072 review, pass 8

**Reviewed**: Working tree on `sprint/s26` against HEAD `fc7ed4728c7eaf9e3a195a8fd2356b8ca4c8f8fa`. Untracked `packages/core/src/cursor.ts` sha256 `fb1c8ae76c1275c1cd1fccff90dfb92089b61a76108f93304de528e91f11fb1e` and `packages/core/src/cursor.test.ts` sha256 `6decebf312d3f9c4ffea429ed5cc4c2d8ab938712d7df85484e83cde2477feaa`. Modified `packages/core/src/index.ts` sha256 `6d1536120fbd61800861d87801f4004d7af2e4e77bdb8cd92da537d7408ceba8`, `packages/core/src/index.test.ts` sha256 `d9c67a80977f96cb3fdf985d25e53d793bf666c13f4efc598506d359ad05fddd`, and the reproducer sentence in `.claude/plans/F-072-design.md` sha256 `4c54d58aa35d5208a7e4e6a1b2f6a2fd6196ef29ad3367006f0067d8dc73fcf8`. After every mutation, `cmp` against the pre-mutation copy was identical and the `cursor.ts` hash was the same value. `git diff` does not include `tools.ts`, `pointer.ts`, `events.ts`, or `commands.ts`.
**Author / reviewer**: The author is the implementer of this uncommitted tree. This reviewer did not write the code and did not resume reviewers 01a19ad4, 01a19ac6, 01a19ab6, 01a19aa7, 01a19a97, 01a19a88, or 01a19a71.
**Result**: 0 defects, 0 smells, 0 nitpicks

## Acceptance challenged

The contract is `.claude/plans/F-072-design.md`. Cursor strings and emission order are local decisions in that file. No HLD formula specifies them.

Pass 7 S1 is closed. Sorting the no-primary fallback by command failed `the first active tool does not follow command order` at `packages/core/src/cursor.test.ts:408`. Idle was `grab`. Expected `zoom-in`. The same edit also failed `a default cursor does not follow command order` at line 424. Idle was `grab`. Expected `default`. A probe of that edit, for Zoom on auxiliary then Pan on secondary, printed `grab`, `grabbing`, `grab`, and `grabbing` for idle, dragging, and both `sync` writes. SetInvert on auxiliary then Pan on secondary printed the same four strings. The restored probe printed `zoom-in` four times for the Zoom case and `default` four times for the SetInvert case. `the first active tool owns the cursor` stayed green under this sort, because that fixture registers Pan before Zoom and command order matches registration order there. `the first active default cursor is kept` also stayed green, because `Fit` sorts before `Pan`.

Pass 7 S2 is closed. Sorting `result.value.demoted` by command failed `demotion does not follow command order` at line 128. The return assertion did not fail. A probe of that edit kept `{ demoted: ["a", "n"] }` and then emitted `tool.modeChanged:n:Pan` before `tool.modeChanged:a:Zoom`. The restored probe emitted `a` before `n`. `demotion keeps registration order` stayed green, because that fixture registers Pan as `n` before Zoom as `a` and that list is already command order.

Swapping `tool.activated` ahead of `tool.modeChanged` failed five tests. `activation follows the mode change` received `tool.activated` then `tool.modeChanged`. A refusal that still emits failed `a refusal is silent` with one `tool.modeChanged` for Pan from `off` to `active`. Reading the group after `setMode` failed eight tests. Subject events were absent, and the Pan demotion previous mode was `passive` rather than `active`. Writing the override into the shared pair object failed `an override replaces one cursor`. The other controller's idle was `move`. Expected `grab`. A listener throw that writes the before-image back left Pan `off` with no bindings. Expected `active` on primary.

On the restored bytes the plan reproducer matched the probe. The second call emits Zoom `tool.modeChanged` from `off` to `active`, then `tool.activated` for Zoom, then Pan `tool.modeChanged` from `active` to `passive`. `sync` with phase `idle` writes `zoom-in`. The first call also emits, and the sentence does not say otherwise.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `npx vitest run` on `cursor.test.ts`, `index.test.ts`, and `events.test.ts` | Tree before mutations | Executed now | Exit 0. 46 passed. 34 cursor, 4 index, 8 events. |
| Same vitest command with `--no-cache` | Restored bytes after the last mutation | Executed now | Exit 0. 46 passed. Same counts. |
| `npx tsc --build` | Bytes whose `cursor.ts` hash is the restored hash | Executed now, before mutations | Exit 0. Not re-run after restore. The hash matched. |
| `npx eslint` on `cursor.ts`, `cursor.test.ts`, `index.ts`, `index.test.ts` | Same bytes | Executed now, before mutations | Exit 0. Not re-run after restore. The hash matched. |
| No-primary fallback is sorted by command | `cursor.ts` only. Restored after. | Executed now | Full `cursor.test.ts --no-cache` exit 1. 2 failed, 32 passed. Idle was `grab` at lines 408 and 424. Probe for both failing setups was `grab`, `grabbing`, `grab`, and `grabbing`. |
| Demoted names are sorted by command | `cursor.ts` only. Restored after. | Executed now | Full suite exit 1. 1 failed, 33 passed. Probe return stayed `["a", "n"]`. Events put `n` before `a`. |
| `tool.activated` is emitted before `tool.modeChanged` | `cursor.ts` only. Restored after. | Executed now | Full suite exit 1. 5 failed. `activation follows the mode change` received `tool.activated` then `tool.modeChanged`. |
| A refusal emits `tool.modeChanged` | `cursor.ts` only. Restored after. | Executed now | Full suite exit 1. `a refusal is silent` received one event. Expected none. |
| The group is read after `setMode` | `cursor.ts` only. Restored after. | Executed now | Full suite exit 1. 8 failed. `activation follows the mode change` received no events. Pan demotion was `passive:passive`. |
| The override writes into the shared pair | `cursor.ts` only. Restored after. | Executed now | Full suite exit 1. `an override replaces one cursor` line 629. The other controller's idle was `move`. Expected `grab`. |
| A listener throw restores the before-image | `cursor.ts` only. Restored after. | Executed now | Full suite exit 1. After the throw, Pan was `off` with no bindings. Expected `active` on primary. |
| Restored reproducer and order probe | `vite-node` import of the restored `cursor.ts` | Executed now | Second call and idle `sync` matched the plan sentence. Zoom before Pan was `zoom-in` four times. SetInvert before Pan was `default` four times. Demotion events were `a` then `n`. Unknown tool, empty active bindings, and an unknown group emitted nothing. A direct `registry.setMode` emitted nothing. |

Node v24.13.0. Vitest 4.1.11. TypeScript 5.9.3. ESLint 9.39.5. The probe script lived under `/tmp` and was not part of the tree. Mutations were restored before the closing checks.

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

Pass 7's two smells are locked by tests that go red. The product file hash did not change. A later tool whose command sorts first no longer leaves the full cursor suite green. A demotion whose command order differs from the returned order no longer leaves it green. The restored source still chooses one tool and then that command's phase string.

On the restored bytes the wrapper reads the group, calls `registry.setMode`, and emits only when that call is ok. Subject events come first. `tool.modeChanged` fires only when the stored mode changes. `tool.activated` fires only for the subject, only when the new mode is `active`, and only after `tool.modeChanged`. Reading the group after the write suppressed the subject events and reported the demoted previous mode as `passive`. Swapping the two subject names reversed the order. A refusal that still emits failed `a refusal is silent`. A listener throw that writes the before-image back left Pan `off`. There is no second mode store. `command` and `previous` come from the group copy taken before the call.

The cursor table has the same eleven commands as `STACK_COMMANDS`. Pan is `grab` and `grabbing`. Zoom is `zoom-in` for both phases. Rotate and SetWindow are `crosshair`. Scroll is `ns-resize`. Fit, FlipHorizontal, FlipVertical, ApplyPreset, SetInvert, and SetColormap are `default`. An override that writes into the shared pair makes the other controller report `move`. Replacing one pair on the restored source leaves the other controller and `COMMAND_CURSORS.Pan` on `grab` and `grabbing`. One chosen tool supplies both phases. Primary wins from any binding slot, with or without modifiers, including when that cursor is `default`. No active tool, including a passive-only tool, is `default`. An unknown group is `default` for both phases, and `sync` writes that string. `sync` assigns on every call. The caller passes the phase. The source guard rejects `addEventListener`, `pointer.ts`, and `pointer.js`. There is no such import.

`cursor.test.ts` has 34 tests. It imports `CursorController`, `CursorControllerOptions`, `CursorPair`, `CursorPhase`, `CursorStyle`, `ToolState`, and `ToolStateEvent` from `./index.js`. It imports `COMMAND_CURSORS` from `./index.js` as well. `packages/core/tsconfig.json` includes `src/**/*`, so those type imports are in `tsc --build`. The value names `COMMAND_CURSORS`, `CURSOR_PHASES`, `createCursorController`, and `createToolState` stay on the exact key list in `index.test.ts`. This pass did not delete an export.

`events.test.ts` passed. `events.ts` was not edited. The catalogue names are still `tool.activated` and `tool.modeChanged`, with shell delivery. This module emits those names and does not write a ring record.

The reproducer sentence matches the restored second call and the idle `sync`. It is not a false claim. No wasm, no pixels, and no `unsafe`. The only `as` cast walks override keys. A stored command is one of the eleven, so an excess key cannot be selected. Tiers A, B, and C are not applicable. The plan says so, and the file has no tier branch. No new trait or generic was added. `tools.ts` was not edited.

## Limitations and handoff

No blocking finding. The restored `cursor.ts` hash is `fb1c8ae76c1275c1cd1fccff90dfb92089b61a76108f93304de528e91f11fb1e`.

F-068 lets primary and primary-plus-shift both stay active. The restored probe kept Pan on primary plus shift and Zoom on unmodified primary, with both demotion lists empty, and the cursor was `grab` and `grabbing`. No cursor test builds two primary owners. The plan does not say which of them wins. This pass did not file that tie-break.

F-068 builds `demoted` by walking tools in registration order. A wrapper that sorted those names back into registration order would match every current return value. The new test separates that order from command order. It does not separate it from registration order, because the provider does not return those as different orders. This pass did not file that.

This pass did not repeat the name-sort edits. `demotion keeps registration order` still registers `n` before `a`. `the first active tool owns the cursor` still registers tool `Z` before tool `A`. Those expects were not re-mutated.

Vitest stops at the first bad expect inside one test. The dragging and sync lines in the two command-order cursor tests were covered by the probe, not by a second vitest failure line. A listener that calls `setMode` again from inside the callback was not exercised. The plan does not define re-entrancy.

`docs/lld/public-api.md` is unchanged. The plan defers that section until the story is completed. It was not in this diff.

`tools.test.ts` and `pointer.test.ts` were not run. Those sources are not in the diff. `tsc --build` and eslint were not repeated after the identical restore. No cargo, wasm-pack, or `bin/ocelli.sh` run. No commit and no stage. This is a bounded review of the F-072 tree, not sprint acceptance.
