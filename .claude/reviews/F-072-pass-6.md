Result: 0 defects, 2 smells, 0 nitpicks

# F-072 review, pass 6

**Reviewed**: Working tree on `sprint/s26` against HEAD `fc7ed4728c7eaf9e3a195a8fd2356b8ca4c8f8fa`. Untracked `packages/core/src/cursor.ts` and `packages/core/src/cursor.test.ts`. Modified `packages/core/src/index.ts`, `packages/core/src/index.test.ts`, and the reproducer sentence in `.claude/plans/F-072-design.md`. `cursor.ts` sha256 `fb1c8ae76c1275c1cd1fccff90dfb92089b61a76108f93304de528e91f11fb1e` matches the bytes this pass started from and the bytes restored after mutation. `cmp` against the pre-mutation copy was identical. `cursor.test.ts` is `d54d65410fb39fd05110498208d6bee1c6d8e90e0485172bafd40ce05a220c77`. `index.ts` is `6d1536120fbd61800861d87801f4004d7af2e4e77bdb8cd92da537d7408ceba8`. `index.test.ts` is `d9c67a80977f96cb3fdf985d25e53d793bf666c13f4efc598506d359ad05fddd`. The plan file is `4c54d58aa35d5208a7e4e6a1b2f6a2fd6196ef29ad3367006f0067d8dc73fcf8`. `git diff` does not include `tools.ts`, `pointer.ts`, `events.ts`, or `commands.ts`.
**Author / reviewer**: The author is the implementer of this uncommitted tree. This reviewer did not write the code and did not resume reviewers 01a19ab6, 01a19aa7, 01a19a97, 01a19a88, or 01a19a71.
**Result**: 0 defects, 2 smells, 0 nitpicks

## Acceptance challenged

The contract is `.claude/plans/F-072-design.md`. Cursor strings and emission order are local decisions in that file. No HLD formula specifies them.

Pass 5 S1 is closed. With no primary owner, skipping an active tool whose cursor is `default` failed `the first active default cursor is kept` at `packages/core/src/cursor.test.ts:402`. Idle was `grab`. Expected `default`. Vitest stopped before the dragging and sync expects. A probe of that edit printed `grab`, `grabbing`, `grab`, and `grabbing` for idle, dragging, and the two sync writes.

Pass 5 S2 is closed. Using `bindings[0]` as the only primary check failed `primary can sit after another binding` at `packages/core/src/cursor.test.ts:421`. Idle was `grab`. Expected `zoom-in`. The probe printed `grab`, `grabbing`, `grab`, and `grabbing`.

Requiring `modifiers.length === 0` on a primary binding failed `primary with a modifier still owns the cursor` at `packages/core/src/cursor.test.ts:437`. Idle was `grab`. Expected `zoom-in`. The probe printed `grab`, `grabbing`, `grab`, and `grabbing`.

Reading the group after `setMode` failed `activation follows the mode change`. The call returned ok and emitted nothing. Expected `tool.modeChanged` then `tool.activated`. Emitting `tool.activated` before `tool.modeChanged` failed that same test. Received `tool.activated` then `tool.modeChanged`.

Forcing the subject command to `Zoom` failed `a mode change that is not active still emits` at `packages/core/src/cursor.test.ts:164`. Keeping the built-in dragging cursor on an override failed `an override replaces one cursor` at `packages/core/src/cursor.test.ts:515`. Received `grabbing`. Expected `move`. Hardcoding the cursor group to `g` failed `the controller reads its group` at `packages/core/src/cursor.test.ts:489`. Received `grab`. Expected `zoom-in`.

Two further wrong edits stayed green. They are the smells below. The restored source does not contain either edit. Pan on secondary and Fit on primary is `default` for both phases and both `sync` writes. Auxiliary then primary with shift is `zoom-in` for both phases and both `sync` writes.

On the restored bytes the plan reproducer matched the probe. Pan active on primary, then Zoom active on primary, emitted Zoom `tool.modeChanged` from `off` to `active`, then `tool.activated` for Zoom, then Pan `tool.modeChanged` from `active` to `passive`. `sync` idle wrote `zoom-in`.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `npx vitest run` on `cursor.test.ts`, `index.test.ts`, and `events.test.ts` | Tree before mutations. Not a receipt. | Executed now | Exit 0. 40 passed. 28 cursor, 4 index, 8 events. |
| `npx tsc --build` | Same tree, before mutations | Executed now | Exit 0. |
| `npx eslint` on `cursor.ts`, `cursor.test.ts`, `index.ts`, `index.test.ts` | Same tree, before mutations | Executed now | Exit 0. |
| No-primary fallback skips a `default` cursor | `cursor.ts` only. Restored after. | Executed now | `-t "the first active default cursor is kept"` exit 1. Idle was `grab`. Expected `default`. Probe idle, dragging, and both sync writes were `grab`, `grabbing`, `grab`, `grabbing`. |
| Primary owner is `bindings[0]` only | `cursor.ts` only. Restored after. | Executed now | `-t "primary can sit after another binding"` exit 1. Idle was `grab`. Expected `zoom-in`. Probe was `grab`, `grabbing`, `grab`, `grabbing`. |
| Primary requires `modifiers.length === 0` | `cursor.ts` only. Restored after. | Executed now | `-t "primary with a modifier still owns the cursor"` exit 1. Idle was `grab`. Expected `zoom-in`. Probe was `grab`, `grabbing`, `grab`, `grabbing`. |
| Group is read after `setMode` | `cursor.ts` only. Restored after. | Executed now | `-t "activation follows the mode change"` exit 1. Received no events. Expected `tool.modeChanged` then `tool.activated`. |
| `tool.activated` is emitted before `tool.modeChanged` | `cursor.ts` only. Restored after. | Executed now | Same test exit 1. Received `tool.activated` then `tool.modeChanged`. |
| Subject command forced to `Zoom` | `cursor.ts` only. Restored after. | Executed now | `-t "a mode change that is not active still emits"` exit 1 at `cursor.test.ts:164`. |
| Override stores idle and keeps built-in dragging | `cursor.ts` only. Restored after. | Executed now | `-t "an override replaces one cursor"` exit 1. Dragging was `grabbing`. Expected `move`. |
| Cursor group is the constant `g` | `cursor.ts` only. Restored after. | Executed now | `-t "the controller reads its group"` exit 1. Idle was `grab`. Expected `zoom-in`. |
| A `default` primary falls back to the first active tool | `cursor.ts` only. Restored after. | Executed now | Full `cursor.test.ts` exit 0. 28 passed. Probe for Pan then Fit on primary was `grab`, `grabbing`, `grab`, `grabbing`. |
| Primary counts only as the first binding or as an unmodified binding | `cursor.ts` only. Restored after. | Executed now | Full `cursor.test.ts` exit 0. 28 passed. Probe for auxiliary then primary with shift was `grab`, `grabbing`, `grab`, `grabbing`. |
| Same vitest command, `npx tsc --build`, and eslint on the four files | After the last restore. `cursor.ts` hash matched. `cmp` was identical. | Executed now | All three exit 0. 40 passed. |

Node v24.13.0. Vitest 4.1.11. TypeScript 5.9.3. ESLint 9.39.5. The Vite probes lived under `/tmp` and were deleted. They did not change the tree.

## Defects

None.

## Smells

### S1, a later primary tool can lose when its cursor is default

**Where**: `packages/core/src/cursor.ts:179`. The expect that stayed green is `packages/core/src/cursor.test.ts:386`. Fit is registered at `packages/core/src/cursor.test.ts:380`, before Pan.
**What**: When the primary tool's cursor was `default`, selection used the first active tool instead of that primary owner. All 28 cursor tests passed. Pan on secondary, registered first, and Fit on primary, registered second, returned `grab` and `grabbing`. Both `sync` calls wrote those strings.
**Why it is wrong**: The plan says the active tool that owns `primary` wins. Fit owns primary, and its pair is `default` on both phases. `a default cursor is not skipped` puts Fit first, so the first active tool and the primary owner are the same object. `primary owns the cursor` puts the primary tool second, but that command is Zoom, so the cursor is not `default`.
**Evidence**: Full `npx vitest run packages/core/src/cursor.test.ts` exit 0. 28 passed. The probe printed `idle grab`, `dragging grabbing`, `syncIdle grab`, and `syncDragging grabbing`. The same probe on the restored file printed `default` four times. Restored afterward. The hash matched. The edit was:

```ts
const primaryPair = primary === undefined ? undefined : table[primary.command];
const primaryValue =
  primaryPair === undefined
    ? undefined
    : phase === "dragging"
      ? primaryPair.dragging
      : primaryPair.idle;
const chosen =
  primary === undefined || primaryValue === "default" ? active[0] : primary;
```

**Scope and regression**: A fit, flip, invert, preset, or colormap tool on primary would show an earlier tool's cursor. Smallest regression is Pan on secondary and Fit on primary, with idle and dragging both `default`, and both `sync` writes `default`. Do not change the chosen expression.

### S2, a later primary binding can be ignored when it has a modifier

**Where**: `packages/core/src/cursor.ts:177`. The expects that stayed green are `packages/core/src/cursor.test.ts:421` and `packages/core/src/cursor.test.ts:437`.
**What**: Primary ownership accepted a modified primary binding only when it was the first binding, and accepted a later primary binding only when it had no modifiers. All 28 cursor tests passed. Pan on secondary, then Zoom active on auxiliary and then primary plus shift, returned `grab` and `grabbing`. Both `sync` calls wrote those strings.
**Why it is wrong**: The plan says the active tool that owns `primary` wins. F-068 stores the whole binding list at `packages/core/src/tools.ts:379`. Auxiliary then primary plus shift is one active owner of primary. `primary can sit after another binding` uses an unmodified primary in the second slot. `primary with a modifier still owns the cursor` puts shift on a primary binding that is the only slot. Either fixture alone still matches this predicate.
**Evidence**: Full `cursor.test.ts` exit 0. 28 passed. The probe printed `idle grab`, `dragging grabbing`, `syncIdle grab`, and `syncDragging grabbing`. The restored probe printed `zoom-in` four times. Restored afterward. The hash matched. The edit was:

```ts
item.bindings.some(
  (binding, index) =>
    binding.button === "primary" &&
    (index === 0 || binding.modifiers.length === 0),
)
```

**Scope and regression**: A tool whose primary binding is not first and carries a modifier would leave the cursor on an earlier tool. Smallest regression is Pan on secondary and Zoom on auxiliary then primary plus shift, with idle and dragging both `zoom-in`, and both `sync` writes `zoom-in`. Do not change the predicate.

## Nitpicks

None.

## Verified clean

Pass 5's two smells are locked by the tests added since that pass. The product file hash did not change. Requiring an empty modifier list now fails. Using only `bindings[0]` now fails. Skipping a `default` cursor when nobody owns primary now fails. The restored source still chooses one tool and then that command's phase string.

On the restored bytes the wrapper reads the group, calls `registry.setMode`, and emits only when that call is ok. Subject events come first. `tool.modeChanged` fires only when the stored mode changes. `tool.activated` fires only for the subject, only when the new mode is `active`, and only after `tool.modeChanged`. Reading the group after the write suppressed the subject events. Swapping the two subject names reversed the order. Forcing the subject command to `Zoom` failed the off-to-passive expect. There is no second mode store.

The cursor table matches the eleven `STACK_COMMANDS`. Pan is `grab` and `grabbing`. Zoom is `zoom-in` for both phases. Rotate and SetWindow are `crosshair`. Scroll is `ns-resize`. Fit, FlipHorizontal, FlipVertical, ApplyPreset, SetInvert, and SetColormap are `default`. An override that keeps the built-in dragging cursor fails. Replacing one pair leaves the other controller and `COMMAND_CURSORS.Pan` on `grab` and `grabbing` in the override test. A hardcoded cursor group fails. No active tool, including a passive-only tool, is `default`. An unknown group is `default` for both phases, and `sync` writes that string. The caller passes the phase. There is no `addEventListener` and no `pointer.ts` or `pointer.js` import.

`cursor.test.ts` imports `CursorController`, `CursorControllerOptions`, `CursorPair`, `CursorPhase`, `CursorStyle`, `ToolState`, and `ToolStateEvent` from `./index.js`. It imports `COMMAND_CURSORS` from `./index.js` as well. `packages/core/tsconfig.json` includes `src/**/*`, so those type imports are in `tsc --build`. The value names `COMMAND_CURSORS`, `CURSOR_PHASES`, `createCursorController`, and `createToolState` stay on the exact key list in `index.test.ts`. This pass did not delete an export.

`events.test.ts` passed. `events.ts` was not edited. The catalogue names are still `tool.activated` and `tool.modeChanged`, with shell delivery. This module emits those names and does not write a ring record.

The reproducer sentence matches the restored probe. It is not a false claim. No wasm, no pixels, and no `unsafe`. Tiers A, B, and C are not applicable. The plan says so, and the file has no tier branch. No new trait or generic was added. `tools.ts` was not edited.

## Limitations and handoff

S1 and S2 are missing assertions. The restored product already follows the plan on both. Remediation should add tests only. It should not change the emission order, the cursor table, the group lookup, the spread, the chosen expression, the primary predicate, or the demotion rule.

This pass did not repeat the earlier mutations that already went red and whose expects are still in the file. Those include demoted command, listener rollback, sync write count, the passive filter, unknown-group `grab`, the pointer import, a refusal emit, and a missing object spread. The read-after, activated-before, non-active command, override dragging, and hardcoded group edits above are the extra discriminating cases, and all five went red.

Vitest stops at the first bad expect inside one test. The dragging and sync lines in the three lock tests were covered by the probe, not by a second vitest failure line.

`docs/lld/public-api.md` is unchanged. The plan defers that section until the story is completed. It was not in this diff.

`tools.test.ts` and `pointer.test.ts` were not run. Those sources are not in the diff. No cargo, wasm-pack, or `bin/ocelli.sh` run. No commit and no stage. This is a bounded review of the F-072 tree, not sprint acceptance.
