# F-068, tool lifecycle, tool groups, bindings and modes

**Status**: approved
**Epic ref**: E11.1
**Sprint**: S25
**Estimate**: 3w

## Normative source, transcribed

HLD `docs/hld/03-architecture-and-crates.md` section 3:

```text
The shell is TypeScript on the main thread: DOM and pointer events, the SVG annotation layer, tool interaction state, framework bindings, and DICOMweb fetch and authentication. The core is Rust running in workers. Between them sits a deliberately narrow boundary carrying commands down, bulk bytes down, and events up.
```

HLD `docs/hld/07-concurrency-and-typescript.md` places tool interaction state machines in the shell. The geometry and hit-testing behind them is Rust, in `ocelli-geom`. The SVG annotation drawing layer stays in the DOM. Those two sentences are the split this story implements. The HLD does not name a tool group, a mode, or a binding.

HLD `docs/hld/B-parity-surface.md` counts about 63 tool classes, of which 26 are annotation, 12 are segmentation, and 25 are manipulation and utility. It says the core events are re-shaped rather than copied. Decision D1 is a new API, not a drop-in shim.

F-100, recorded in `docs/lld/public-api.md`, already publishes `ToolBinding` as one caller-chosen name mapped to one of the eleven stack commands. The pointer gesture stays in the shell. Hit-testing stays in `ocelli-geom`. Annotation tools are not in that command set.

The allocation note for this story says the work is TypeScript interaction state, not pixels.

## What the specification does not cover

The HLD does not name modes, groups, pointer buttons, or what happens when two tools want the same gesture. The decisions below are local. They are not a reading of a formula the HLD forgot to write.

- A registry is created by `createToolRegistry`. There is no module singleton. F-100 already refused a singleton for the React binding, and this state has the same shape.
- Modes are `active`, `passive` and `off`. `active` owns one or more pointer bindings and is the only tool a later dispatcher may run for that binding. `passive` and `off` own no bindings. This story does not draw, so `passive` and `off` differ only as stored mode. F-072 owns cursors. F-071 owns SVG.
- Pointer buttons are `primary`, `auxiliary` and `secondary`. Modifiers are `alt`, `ctrl`, `meta` and `shift`. A binding is one button plus a set of modifiers. Order and repeated modifiers do not make a second binding.
- Setting a tool `active` on a binding moves every other `active` owner of that binding, in the same group, to `passive` and clears that owner's bindings. The moved names are returned. The change is not silent.
- A viewport id belongs to at most one group in the registry. A second attach is refused.
- A refused call changes nothing.
- This story does not listen for events, pack commands, or call wasm. F-069 dispatches. Registering a tool does not run its command.
- Cornerstone tool class names are not accepted as a closed list and are not required. The tool name is caller-chosen, as F-100 already says. The command must still be one of the eleven.

## Problem and reproducer

No registry exists. `ToolBinding` is a pair of fields with no owner, no mode and no pointer binding. Two callers can each believe they own primary-button pan. Nothing reports the collision.

The observation this story adds is a registry call, not a rendered frame. `createToolRegistry`, register `Pan` with command `Pan`, set it active on primary with no modifiers, then set `Zoom` active on the same binding. The group then has one active owner, `Zoom`, and the return names `Pan` as demoted.

## Scope, API and compatibility

In scope:

- `packages/core/src/tools.ts`, the registry.
- Tests in `packages/core/src/tools.test.ts`.
- Re-exports from `packages/core/src/index.ts` and the published-name list in `index.test.ts`.
- `docs/lld/public-api.md` gains the lifecycle section when the story is completed.

Out of scope:

- Pointer, touch and wheel listeners. That is F-069.
- Cursor style. That is F-072.
- SVG drawing. That is F-071.
- Annotation, segmentation and measurement tool classes. Those start at F-073.
- Any change to command ids, the 96-byte readback, or `ToolBinding`'s two fields.
- Hit-testing and measurement. That is F-070.

`createToolRegistry(): ToolRegistry`.

`ToolRegistry` operations, each returning `ToolOutcome<T>`:

| Operation | Success value | Refusal leaves |
|-----------|---------------|----------------|
| `createGroup(id)` | the id | the registry |
| `destroyGroup(id)` | the id | the registry |
| `registerTool(groupId, binding)` | the stored tool, mode `off`, no bindings | the group |
| `removeTool(groupId, tool)` | the removed name | the group |
| `setMode(groupId, tool, mode, bindings)` | `{ demoted }` | the group |
| `attachViewport(groupId, id)` | the id | both groups |
| `detachViewport(groupId, id)` | the id | the group |
| `group(id)` | a copy of the group | nothing, it does not write |
| `bindingOwner(groupId, binding)` | the active tool, or null when nobody owns it | nothing, it does not write |
| `groups()` | ids in creation order | nothing |

`ToolOutcome<T>` is `{ ok: true, value: T }` or `{ ok: false, refusal }`. A refusal is one of `empty-group`, `duplicate-group`, `unknown-group`, `empty-tool`, `duplicate-tool`, `unknown-tool`, `unknown-command`, `bad-mode`, `bad-binding`, `active-needs-binding`, `bindings-only-when-active`, `duplicate-binding`, `bad-viewport`, `duplicate-viewport`, `missing-viewport`, `viewport-taken`.

A group id or a tool name is empty when it has no non-whitespace character. The stored string is the caller's string, not a trimmed copy. `" pan"` and `"pan"` are different names.

`active` requires at least one binding. `passive` and `off` require an empty binding list. Duplicate bindings in one `setMode` are refused before any demotion. A repeated modifier on one binding is one modifier.

`groups()` and the tool list and the viewport list preserve insertion order. Reads return copies. Mutating a returned array does not change the registry.

The same `setMode` value, compared by button and modifier set, returns `demoted: []` and does not rewrite the other tools.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| A later dispatcher, F-069 | `bindingOwner` | One group, one pointer binding | The active tool, or null |
| `registerTool` | F-100 `ToolBinding` | Caller-chosen name, one of eleven commands | Mode `off` until `setMode` |
| Viewport attach | `viewportId` values | At most one group | `viewport-taken` or the id |
| This story's tests | `createToolRegistry` | Fresh registry per test | The outcomes above |

No render path reads this state. No wasm export is added.

## State transitions and invariants

| Entry point / starting state | Input or event | Resulting state and output | Refusal and preserved state |
|------------------------------|----------------|----------------------------|-----------------------------|
| No group | `createGroup("g")` | Group with no tools and no viewports | `empty-group` or `duplicate-group` adds nothing |
| Group, no tool | `registerTool` of `{ tool: "Pan", command: "Pan" }` | One tool, mode `off` | `duplicate-tool` keeps the first command |
| Tool `off` | `setMode` active, primary, no modifiers | That tool active with that binding | Empty binding list is `active-needs-binding` |
| Tool A active on primary | `setMode` B active on primary | B active, A passive, bindings cleared, `demoted` is `["A"]` | A bad mode on B leaves A active |
| Tool active | `setMode` passive with `[]` | That tool passive, bindings empty, `demoted` empty | A non-empty list is `bindings-only-when-active` and the tool stays active |
| Viewport free | `attachViewport` | Id appended | Non-integer, negative, or unsafe number is `bad-viewport` |
| Viewport on group A | `attachViewport` on group B | A unchanged | `viewport-taken` |
| Viewport on the group | `detachViewport` | Id removed, order of the rest kept | `missing-viewport` leaves the list |
| Unknown group | any group operation | no write | `unknown-group` |

Invariant after every successful write: inside one group, each normalised binding has at most one `active` owner, and a `passive` or `off` tool has an empty binding list. A viewport id appears in at most one group.

Destroying a group drops its tools and releases its viewports. Removing the active tool does not promote another tool.

## Ownership, commit and recovery

| Resource or visible state | Acquired or staged by | Lifetime identity | Observable commit point | Failure and duplicate-operation behavior |
|---------------------------|-----------------------|-------------------|-------------------------|------------------------------------------|
| The registry object | `createToolRegistry` | The object the caller holds | The call returns | A second call makes a second registry. Groups are not shared. |
| One group | `createGroup` | The caller-chosen id string | `ok: true` | A failed create does not insert a partial group. |
| One tool row | `registerTool` | The tool name inside that group | `ok: true` | A failed register does not change the existing row. |
| Mode and bindings | `setMode` | The tool name | `ok: true` after demotions and the new mode are both stored | A refusal returns before either write. |
| Viewport membership | `attachViewport` | The branded number | `ok: true` | `viewport-taken` does not detach the current owner. |

There is no device, no ticket and no frame. An identical `createGroup` is `duplicate-group`, not a no-op success. An identical `setMode` is a no-op success with an empty demotion list.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | Second tool set active on the same binding demotes the first to passive and returns its name | `a second active tool takes the binding` | Both stay active, or the first is removed | implementation |
| AC2 | Primary and primary-plus-shift are different bindings and can both be active | `modifiers split a button into two bindings` | Shift is ignored and the second call demotes the first | implementation |
| AC3 | `setMode` active with no bindings is refused and the previous owner stays | `an active tool needs a binding` | The tool becomes active and owns nothing | implementation |
| AC4 | `setMode` passive with a binding is refused and the tool stays active | `passive and off store no bindings` | The binding is kept on a passive tool | implementation |
| AC5 | Duplicate register is refused and the first command remains | `a second registration does not replace the command` | The command becomes the second call's command | implementation |
| AC6 | A name `Pan` may issue command `Zoom` | `the tool name is not the command` | The registry requires the name to equal the command | implementation |
| AC7 | Unknown command, blank name, blank group, bad mode and bad button are refused | `refusals leave the registry unchanged` | A blank name is stored, or a bad call still demotes | implementation |
| AC8 | The same `setMode` returns an empty demotion list | `the same mode is a no-op` | The tool is demoted by its own call | implementation |
| AC9 | Repeated modifier `shift` is one binding, not `duplicate-binding` | `repeated modifiers collapse` | The call is refused | implementation |
| AC10 | A viewport on group A cannot attach to group B | `a viewport belongs to one group` | Both groups list it | implementation |
| AC11 | Detach of a missing id is `missing-viewport` | `detach of an absent viewport is refused` | The call returns success | implementation |
| AC12 | `viewportId(0)` attaches. `-1`, `1.5`, `NaN` do not | `viewport ids are non-negative integers` | `0` is refused, or `1.5` is stored | implementation |
| AC13 | A read copy cannot push a tool into the registry | `reads return copies` | The internal array is returned | implementation |
| AC14 | Two registries do not share groups | `registries are independent` | A module-level map remembers the first registry's group | implementation |
| AC15 | The module source does not pack a command or add a listener | `the registry does not dispatch` | `registerTool` imports `packCommands` or calls `addEventListener` | implementation |
| AC16 | After demotion the binding owner is the new tool | `bindingOwner follows the mode` | `bindingOwner` still returns the demoted tool | implementation |

## Approach

One module, `packages/core/src/tools.ts`. The registry is a closure over a group array. Validation finishes before any splice or field write. Binding identity is the button plus the modifiers sorted in `POINTER_MODIFIERS` order, with duplicates removed.

Rejected alternatives:

- Copying cornerstone's `Active`, `Passive`, `Enabled` and `Disabled` enum. Decision D1 forbids a shim, and `Enabled` would pretend this story draws.
- Silently replacing the active tool without naming it. The demotion list is the observation.
- Storing the registry on `globalThis`. That is the singleton F-100 avoided.
- Running the stack command inside `setMode`. F-069 owns dispatch. This story would then depend on a gesture it does not have.

## Dependencies, write ownership and resources

Depends on F-100, which is done. No source dependency on F-070. Files this story writes: `packages/core/src/tools.ts`, `packages/core/src/tools.test.ts`, `packages/core/src/index.ts`, `packages/core/src/index.test.ts`, and at completion the sprint ledgers and `docs/lld/public-api.md`. It does not edit `ocelli-geom`, shaders, or the command table.

Serial, in the canonical worktree. No GPU.

## Verification and evidence reuse

Focused check: `npx vitest run packages/core/src/tools.test.ts` and `npx vitest run packages/core/src/index.test.ts`. Completion uses the feature profile with corpus, because that is the story gate, not because this story reads a DICOM file. The corpus result is inherited. No comparison report is regenerated. CPU candidate bytes do not change.

A metadata edit after the feature record invalidates that record. The sprint record is a later commit.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none
- unsafe: none
- Tier A (WebGPU): n/a. The registry does not draw and does not read a device.
- Tier B (WebGL2): n/a. Same state on every tier.
- Tier C (CPU): n/a. Same state on every tier. It does not report a different command on CPU.

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| unit | AC1 to AC16 | `packages/core/src/tools.test.ts` |
| unit | The new value exports are on the published list | `packages/core/src/index.test.ts` |

No pixel fixture. This story does no pixel arithmetic and no geometry arithmetic.

## Parity surface covered

None. The parity table names tool classes this story does not implement. The lifecycle is the substrate those later stories will use. No cornerstone event is emitted.

## Deviations

none

## LLD impact

`docs/lld/public-api.md` and its row in `docs/lld/README.md`.

## Open questions

None. The operator declined a design-question round on 2026-10-31. The local decisions above are the ones that would have been asked. The plan is approved.
