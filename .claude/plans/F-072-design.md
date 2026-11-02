# F-072, cursor management and tool state machine

**Status**: approved
**Epic ref**: E11.5
**Sprint**: S26
**Estimate**: 3w

## Normative source, transcribed

HLD `docs/hld/07-concurrency-and-typescript.md` section 10:

```text
Tool interaction state machines; the geometry and hit-testing behind them is Rust, in ocelli-geom
```

The same section puts DOM work in TypeScript. HLD `docs/hld/03-architecture-and-crates.md` section 3 names tool interaction state as shell work.

F-103 names `tool.activated` and `tool.modeChanged`. Delivery is `shell`. Producer is `shell tool state`. This story is that producer. The events are not ring records.

F-068 stores modes and bindings and does not emit those names. Its `setMode` return is `{ demoted }` and a no-op returns an empty demotion list. Demotion moves another active owner to `passive` and clears that owner's bindings. This story does not change that function.

The allocation note for F-072 is empty. The epic layer cell says Rust+TS. F-068's note placed lifecycle in TypeScript, and this story is the rest of that shell state. No Rust file is added. Cursor presence in HLD section 26 is a later collaboration feature, F-156, and is not this cursor.

## What the specification does not cover

The HLD does not name cursor strings or when `tool.activated` fires. The decisions below are local.

- `createToolState` wraps one registry. `setMode` on the wrapper calls the registry and then emits. A direct `registry.setMode` does not emit. Callers that want the events use the wrapper.
- `tool.modeChanged` fires when the subject's stored mode changes, and once per demoted tool. The previous mode is included. A refused call emits nothing. An identical `setMode` emits nothing.
- `tool.activated` fires only for the subject, and only when the new mode is `active`. A demoted tool does not activate. A move from `active` to `passive` or `off` does not activate.
- `registerTool` and `removeTool` emit nothing. A new tool starts `off`.
- The cursor table is local CSS. `Pan` is `grab` and `grabbing`. `Zoom` is `zoom-in`. `Rotate` and `SetWindow` are `crosshair`. `Scroll` is `ns-resize`. The other commands are `default`. An override replaces one command and leaves the rest.
- The cursor tool is the active tool that owns `primary`. If none owns `primary`, it is the first active tool in registration order. If none is active, the cursor is `default`.
- Phase `dragging` selects that command's dragging cursor. Phase `idle` selects the idle cursor. This module does not listen for pointer events. The caller passes the phase.
- `sync` writes `style.cursor` every time, including when the string is unchanged.

## Problem and reproducer

F-068 can demote a tool and nothing tells the shell, so the element can keep the previous cursor. The observation is a wrapper call. Set `Pan` active on primary, then set `Zoom` active on primary. The listener receives `tool.modeChanged` for `Pan` from `active` to `passive`, then `tool.modeChanged` for `Zoom` from `off` to `active`, then `tool.activated` for `Zoom`. `sync` with phase `idle` writes `zoom-in`.

## Scope, API and compatibility

In scope:

- `packages/core/src/cursor.ts`
- `packages/core/src/cursor.test.ts`
- Re-exports from `packages/core/src/index.ts` and the published-name list
- `docs/lld/public-api.md` gains the cursor section when the story is completed

Out of scope:

- DOM listeners. F-069 owns those. This module does not import `pointer.ts`.
- Editing `tools.ts` or the demotion rule.
- SVG, hit-testing, and F-156 presence cursors.
- Packing commands.

`createToolState(registry, listener): ToolState`

`ToolState.setMode(groupId, tool, mode, bindings)` returns the registry's `ToolOutcome`. On success it emits, subjects first and then demoted names in the returned order. The subject's `tool.modeChanged` comes before `tool.activated` when both fire.

`createCursorController(options): CursorController`

`options` is the registry, the group id, a `{ cursor: string }` style target, and an optional partial cursor table. `cursor(phase)` returns the string. `sync(phase)` writes it to `style.cursor`.

Event fields are `name`, `groupId`, `tool`, `command`, `mode` and `previous`. `previous` is null only when the tool was not in the group, which this wrapper does not emit because an unknown tool is a refusal.

## Consumer and provider map

| Consumer | Provider | Policy | Output |
|----------|----------|--------|--------|
| Shell listener | `createToolState` | emit only after a successful write | `tool.activated`, `tool.modeChanged` |
| Element style | `createCursorController` | primary, else registration order | one CSS cursor |
| F-068 registry | the wrapped object | no second mode store | the same `ToolOutcome` |

## State transitions and invariants

| Start | Input | Result | Preserved on refusal |
|-------|-------|--------|----------------------|
| tool `off` | `setMode` active with a binding | `modeChanged` then `activated` | registry unchanged, no event |
| other tool active on that binding | subject becomes active | subject events, then one `modeChanged` per demoted name | registry unchanged, no event |
| same mode and same bindings | `setMode` | no event | registry |
| one active `Pan` | `sync("dragging")` | `grabbing` | registry |
| `Pan` on secondary, later tool on primary | `sync("idle")` | the primary tool's cursor | registry |
| no active tool | `sync("idle")` | `default` | registry |

The wrapper does not keep a second copy of modes. It reads the group before the call and uses the registry result after the call.

## Ownership, commit and recovery

The registry commit stays inside F-068 `setMode`. Events are emitted only after that call returns `ok`. A throw from the listener is the listener's. The wrapper does not catch it and does not roll the registry back. An identical `sync` writes the same string again.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test | Wrong implementation rejected | Stage |
|----|-------------------------------|------|-------------------------------|-------|
| AC1 | Active from `off` emits `modeChanged` then `activated` | `activation follows the mode change` | emitting only one of them | implementation |
| AC2 | Demotion emits `modeChanged` and not `activated`, in returned order | `demotion is not activation` | activating the loser, or sorting the names | implementation |
| AC3 | Identical `setMode` emits nothing | `a no op emits nothing` | a repeated `activated` | implementation |
| AC4 | A refusal emits nothing | `a refusal is silent` | emitting before the write | implementation |
| AC5 | `registerTool` and `removeTool` emit nothing | `registration is not activation` | an event from the passthrough | implementation |
| AC6 | `Pan` idle is `grab`, dragging is `grabbing` | `pan uses grab and grabbing` | one cursor for both phases | implementation |
| AC7 | Primary wins over an earlier secondary tool | `primary owns the cursor` | first-registered always | implementation |
| AC8 | No primary owner uses registration order | `the first active tool owns the cursor` | a sorted name | implementation |
| AC9 | No active tool is `default` | `an idle group uses the default cursor` | keeping the previous command's cursor | implementation |
| AC10 | `sync` writes `style.cursor` | `sync writes the style` | returning the string and not writing | implementation |
| AC11 | An override replaces one command | `an override replaces one cursor` | replacing the whole table | implementation |
| AC12 | The wrapper returns the registry demotion list | `the wrapper does not change demotion` | a second demotion policy | implementation |
| AC13 | The source does not call `addEventListener` | `the cursor module does not listen` | a second pointer listener | implementation |

## Approach

`packages/core/src/cursor.ts` holds the default table, the wrapper and the controller. It imports the registry types from `tools.ts` and does not import `pointer.ts`.

Rejected: editing `setMode` to emit, a module-singleton cursor, and listening inside this file.

## Dependencies, write ownership and resources

Depends on F-068, which is done. Runs after F-069 is committed because both edit `index.ts` and the ledgers. This story writes `cursor.ts`, `cursor.test.ts`, `index.ts` and `index.test.ts`. No shader and no GPU.

## Verification and evidence reuse

`npm test`, `npm run typecheck` and `npm run lint` in `packages/core`. No wasm rebuild is required for this story alone. `/complete-feature` still runs the feature profile. A later edit to `cursor.ts` invalidates that receipt.

No pixel fixture and no geometry fixture.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none
- unsafe: none
- Tier A (WebGPU): n/a
- Tier B (WebGL2): n/a
- Tier C (CPU): n/a

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| unit | the acceptance rows above | `packages/core/src/cursor.test.ts` |
| fixture | none. No pixel arithmetic | |

## Parity surface covered

None of the tool-class rows. The shell tool-state producer for `tool.activated` and `tool.modeChanged` is this module. The catalogue file is not edited.

## Deviations

none

## LLD impact

`docs/lld/public-api.md` gains a cursor and tool-state section. No new LLD file.

## Open questions

None. The operator declined a design-question round on 2026-10-31. The cursor strings and the emission order are decided in this plan.
