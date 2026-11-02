Result: 0 defects, 10 smells, 0 nitpicks

# F-072 review, pass 1

**Reviewed**: working tree on `sprint/s26` against HEAD `fc7ed4728c7eaf9e3a195a8fd2356b8ca4c8f8fa`. Untracked `packages/core/src/cursor.ts` and `packages/core/src/cursor.test.ts`. Modified `packages/core/src/index.ts`, `packages/core/src/index.test.ts`, and `.claude/plans/F-072-design.md`. The design diff is the reproducer order only.
**Author / reviewer**: the author is the implementer of this uncommitted tree. The reviewer did not write this code and did not resume an earlier reviewer.
**Result**: 0 defects, 10 smells, 0 nitpicks

## Acceptance challenged

The contract is `.claude/plans/F-072-design.md`, not an HLD cursor formula. The plan's reproducer and API section both put the subject first. `tool.modeChanged` for the subject comes before `tool.activated`. Demoted names follow, in the registry's returned order.

AC1 and the reproducer order are locked. Swapping `tool.activated` ahead of `tool.modeChanged` failed `activation follows the mode change` and `demotion follows the subject`.

AC2's "sorting the names" is not locked. The demotion tests use one name. Sorting the demoted list stayed green.

AC3, AC4 for `active-needs-binding`, AC5, AC6, AC7 against "first registered always", AC8 against a sorted name, AC11, AC12, and the `addEventListener` half of AC13 are implemented and the permanent tests reject the wrong behaviour they name. AC7 does not by itself reject "last registered wins". AC8 does, because `Z` then `A` with no primary expects `grab`.

These required outcomes stay green when broken. A subject move to `passive` or `off` can also emit `tool.activated`. A passive tool can own the cursor. An unknown group can return `grab`. `sync` can skip an unchanged string. The subject `tool.modeChanged` can carry an empty `groupId`. A demoted event can carry the subject's command. A listener throw can be caught and the previous modes written back. `import "./pointer.js"` is invisible to the source scan. The cursor type re-exports can be deleted.

A deleted probe, run before those mutations and then removed, called `createToolState` and `createCursorController` from `packages/core/src/index.ts`. It is not part of the tree. Its observations are recorded under verified clean. They show the restored product already does the behaviour the permanent suite fails to lock.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `npx vitest run packages/core/src/cursor.probe.test.ts packages/core/src/cursor.test.ts packages/core/src/index.test.ts` | Published `createToolState` and `createCursorController`. Probe deleted afterward. | Executed now. Not a receipt. | Exit 0. 30 passed. 10 probe, 16 cursor, 4 index. |
| Same vitest command without the probe. Subject `tool.activated` emitted before `tool.modeChanged`. | `cursor.ts` only. Restored after. | Executed now | Exit 1. 2 failed, 18 passed. `activation follows the mode change`. `demotion follows the subject`. Received demotion order started `tool.activated:Zoom:off:active`. |
| Same command. `result.value.demoted` copied and sorted. | `cursor.ts` only. Restored after. | Executed now | Exit 0. 20 passed. |
| Same command. `tool.activated` also emitted for `passive` and `off`. | `cursor.ts` only. Restored after. | Executed now | Exit 0. 20 passed. |
| Same command. Active filter widened to `mode !== "off"`. | `cursor.ts` only. Restored after. | Executed now | Exit 0. 20 passed. |
| Same command. Unknown group returned `grab`. | `cursor.ts` only. Restored after. | Executed now | Exit 0. 20 passed. |
| Same command. `sync` assigned only when the string changed. | `cursor.ts` only. Restored after. | Executed now | Exit 0. 20 passed. |
| Same command. Subject `tool.modeChanged` used `groupId: ""`. | `cursor.ts` only. Restored after. | Executed now | Exit 0. 20 passed. |
| Same command. Demoted `command` copied from the subject. | `cursor.ts` only. Restored after. | Executed now | Exit 0. 20 passed. |
| Same command. Listener wrapped in `try/catch`, previous modes written back, error rethrown. | `cursor.ts` only. Restored after. | Executed now | Exit 0. 20 passed. No test throws. |
| Same command. `import "./pointer.js"` added. Source contained `pointer.js` and not `pointer.ts`. | `cursor.ts` only. Restored after. | Executed now | Exit 0. 20 passed. |
| `npx tsc --build` and the vitest command. Cursor `export type` block removed. | `index.ts` only. Restored after. | Executed now | Both exit 0. |
| `npx tsc --build` | Probe present, importing `CursorStyle` and `ToolStateEvent` from `index.ts`. Probe then deleted. | Executed now | Exit 0. |
| `npx tsc --build` | Restored tree, probe absent. | Executed now | Exit 0. |
| `npx eslint` on `cursor.ts`, `cursor.test.ts`, `index.ts`, `index.test.ts` | Restored tree | Executed now | Exit 0. |
| `npx vitest run packages/core/src/cursor.test.ts packages/core/src/index.test.ts` | Restored tree | Executed now | Exit 0. 20 passed. |

Node v24.13.0. Vitest 4.1.11. `cursor.ts` sha256 before the mutations and after the last restore was `fb1c8ae76c1275c1cd1fccff90dfb92089b61a76108f93304de528e91f11fb1e`. `cmp` against `/tmp/f072-cursor.ts` was identical. `cursor.test.ts` stayed `a6d536632368f6cc8f08cf6b7a510dd5525f1754ac7181a20ebab702c02e0fe7`. `index.ts` stayed `6d1536120fbd61800861d87801f4004d7af2e4e77bdb8cd92da537d7408ceba8`. `index.test.ts` stayed `d9c67a80977f96cb3fdf985d25e53d793bf666c13f4efc598506d359ad05fddd`. No probe file remains.

## Defects

None.

## Smells

### S1, demotion order can be sorted

**Where**: `packages/core/src/cursor.ts:135`. Tests at `packages/core/src/cursor.test.ts:51` and `:67`.
**What**: Replacing `result.value.demoted` with a sorted copy left the suite green. Both demotion tests demote one name, so a sort does not change the expectation.
**Why it is wrong**: The plan's AC2 says the wrong implementation is activating the loser or sorting the names. Events must follow the returned order. The restored loop does that. The suite does not reject a sort.
**Evidence**: Sort mutation. `npx vitest run packages/core/src/cursor.test.ts packages/core/src/index.test.ts` exit 0. 20 passed.
**Scope and regression**: A later edit can emit `a` before `n` when the registry returned `n` then `a`. Smallest regression is one `setMode` that takes two bindings owned by tools registered in a non-alphabetical order. The product on this tree was not left sorted.

### S2, leaving active can also activate

**Where**: `packages/core/src/cursor.ts:124`.
**What**: Emitting `tool.activated` for every subject mode change, including `passive` and `off`, left the suite green. The listened transitions are `off` to `active`, a no-op, and a binding change that stays `active`.
**Why it is wrong**: The plan says `tool.activated` fires only for the subject and only when the new mode is `active`. A move from `active` to `passive` or `off` does not activate. The restored guard is `mode === "active"` inside a real mode change. Nothing in the suite reaches the other two modes through the wrapper.
**Evidence**: Condition widened to all three modes. Same vitest command exit 0. 20 passed.
**Scope and regression**: A shell that treats every `tool.activated` as "this tool is now the active one" would follow a tool that just turned off. Smallest regression is wrapper `setMode` from `active` to `passive` and from `active` to `off`, asserting one `tool.modeChanged` and no `tool.activated`.

### S3, a passive tool can own the cursor

**Where**: `packages/core/src/cursor.ts:175`. Idle coverage is `packages/core/src/cursor.test.ts:222`.
**What**: Treating every tool that is not `off` as a cursor candidate left the suite green. The idle test leaves the only tool `off`. No cursor assertion uses `passive`.
**Why it is wrong**: The plan says the cursor tool is the active tool that owns `primary`, else the first active tool. If none is active, the cursor is `default`. `passive` is not active. The restored filter keeps only `active`.
**Evidence**: Filter changed to `item.mode !== "off"`. Same vitest command exit 0. 20 passed.
**Scope and regression**: After `setMode` to `passive`, `sync("idle")` would keep `grab` for `Pan`. Smallest regression is one passive tool and an expected `default`.

### S4, an unknown group can return another cursor

**Where**: `packages/core/src/cursor.ts:172`.
**What**: Returning `grab` when `registry.group` is not ok left the suite green. Every controller in the suite uses group `g`, which exists.
**Why it is wrong**: An unknown group is `default`, for both phases. The restored branch returns `default`. Deleting it would throw on `group.value` and leave the previous `style.cursor` in place.
**Evidence**: Return changed to `grab`. Same vitest command exit 0. 20 passed.
**Scope and regression**: A destroyed group would keep the last tool's cursor or throw. Smallest regression is `groupId: "missing"` for `cursor` and `sync` on both phases.

### S5, sync can skip an unchanged string

**Where**: `packages/core/src/cursor.ts:189`.
**What**: Assigning `style.cursor` only when the next string differs left the suite green. The sync tests set the field to `stale` or `""` before they expect a write, so the value always changes.
**Why it is wrong**: The plan says `sync` writes `style.cursor` every time, including when the string is unchanged. The restored body assigns on every call. A setter that notifies on assignment would miss the second call.
**Evidence**: Equality guard added around the assignment. Same vitest command exit 0. 20 passed.
**Scope and regression**: Smallest regression is two `sync("idle")` calls on a target whose setter counts, with the cursor already equal to the chosen string.

### S6, groupId is not read back

**Where**: `packages/core/src/cursor.ts:118`. Assertions in `packages/core/src/cursor.test.ts:46` and `:60`.
**What**: The subject `tool.modeChanged` carried `groupId: ""` and the suite stayed green. `toMatchObject` does not name `groupId`. The demotion string omits it. Every fixture group is `g`, so a hardcoded `g` would pass as well.
**Why it is wrong**: Event fields are `name`, `groupId`, `tool`, `command`, `mode`, and `previous`. `groupId` is the group passed to `setMode`. The restored event uses that argument.
**Evidence**: One field set to `""`. Same vitest command exit 0. 20 passed. Activated and demoted events still had the real id. That was enough for green.
**Scope and regression**: A listener for a second group would update `g`, or update nothing. Smallest regression is a group id other than `g` on all three event shapes.

### S7, a demoted event can carry the winner's command

**Where**: `packages/core/src/cursor.ts:144`.
**What**: The demoted event's `command` was taken from the subject. The suite stayed green. The only command assertion is `command: "Pan"` on the Pan activation. The Zoom and Pan demotion strings do not include `command`.
**Why it is wrong**: `command` is the command stored for that tool, read before the call. A demoted `Pan` must not be reported as `Zoom`. The restored event uses `demoted.command`.
**Evidence**: `command: subject.command` in the demotion loop. Same vitest command exit 0. 20 passed.
**Scope and regression**: The shell would show the winner's command on the loser's `tool.modeChanged`. Smallest regression is the Pan then Zoom reproducer with `command` asserted on all three events.

### S8, a listener throw can roll the registry back

**Where**: `packages/core/src/cursor.ts:105`. No test throws.
**What**: Each notify was wrapped in `try/catch`. On a throw the wrapper wrote every tool in the before-image back through `registry.setMode` and rethrew. The suite stayed green.
**Why it is wrong**: The plan says a listener throw is the listener's. The wrapper does not catch it and does not roll the registry back. The restored function has no `try`. The success path does not show that.
**Evidence**: Catch-and-restore mutation. Same vitest command exit 0. 20 passed.
**Scope and regression**: A throwing listener would undo a demotion the registry had already committed. Smallest regression is a listener that throws on the first event, then `group` shows the new modes.

### S9, the pointer import guard looks for the wrong specifier

**Where**: `packages/core/src/cursor.test.ts:251`.
**What**: Adding `import "./pointer.js"` left `the cursor module does not listen` green. The assertion searches for `pointer.ts`. This package imports `./pointer.js`. The file did not contain `pointer.ts`.
**Why it is wrong**: The plan says this module does not import `pointer.ts` and does not call `addEventListener`. The `addEventListener` search would still see that call. It does not see the import that would actually be written.
**Evidence**: Import added. Same vitest command exit 0. 20 passed. The restored file imports `./generated.js` and `./tools.js` only.
**Scope and regression**: A later edit can pull pointer helpers in without a red AC13. Smallest regression is a source assertion for `pointer.js` as well as `addEventListener`.

### S10, the cursor type re-exports are not watched

**Where**: `packages/core/src/index.ts:203` and the claim at `packages/core/src/index.test.ts:33`.
**What**: Deleting the `export type` block for `CursorController`, `CursorControllerOptions`, `CursorPair`, `CursorPhase`, `CursorStyle`, `ToolState`, and `ToolStateEvent` left `npx tsc --build` and the vitest command green. Value exports stayed. The comment says `npx tsc` watches those erased names. Nothing in the review tree imports them from `index.ts`.
**Why it is wrong**: The plan publishes the wrapper and controller types from `index.ts`. The sentence that names the new types says tsc watches them. Removal was not a type error and not a test failure.
**Evidence**: Type export block removed, then restored. `npx tsc --build` exit 0. Vitest exit 0. 20 passed. `index.ts` sha256 matched the pre-edit hash.
**Scope and regression**: A consumer of the type names would fail later, outside this suite. Smallest regression is one file that imports those types from `index.ts` and is part of `tsc --build`. The value names `COMMAND_CURSORS`, `CURSOR_PHASES`, `createCursorController`, and `createToolState` are on the exact key list and were not removed in this check.

## Nitpicks

None.

## Verified clean

On the restored bytes, the product matches the plan. The permanent tests lock the cases below. The deleted probe repeated them through the published exports and added the cases the suite does not lock. Those probe results describe this tree. They are not a substitute for the missing tests.

`createToolState` reads the group, calls `registry.setMode`, and emits only when that call is ok. A direct `registry.setMode` emits nothing. A second wrapper is not subscribed. `registerTool` and `removeTool` forward and emit nothing. Refusals `active-needs-binding`, `unknown-tool`, `unknown-group`, and `bindings-only-when-active` return the registry outcome and emit nothing. The unknown-tool refusal leaves the tool `off`.

An identical `setMode` emits nothing, including modifiers listed as `shift` then `alt` and then `alt` then `shift`. A binding change that keeps the subject `active` emits no subject event and no `tool.activated`. It does emit one `tool.modeChanged` per demoted name.

`off` to `active` emits `tool.modeChanged` then `tool.activated`. The listener in the probe read the group during the event and saw the new mode, so the emit is after the commit. `passive` to `active` emits the same pair with `previous: "passive"`. `active` to `passive`, `off` to `passive`, and `active` to `off` emit one `tool.modeChanged` and no `tool.activated`.

The Pan then Zoom reproducer emits Zoom `off` to `active`, then `tool.activated`, then Pan `active` to `passive`. Fields on those events are the group, the tool, that tool's command, the new mode, and the mode from the before-image. Two demotions return `n` then `a` and the events follow that order, with commands `Pan` and `Zoom`. Neither loser is activated. The wrapper outcome equals a twin registry's outcome.

There is no second mode store. After a direct `setMode`, and after `removeTool` of the primary owner, the next `cursor` call follows the registry. Removing the primary `Pan` and leaving `Zoom` on secondary yields `zoom-in` and no event.

The cursor table matches the eleven `STACK_COMMANDS`. Pan is `grab` and `grabbing`. Zoom is `zoom-in`. Rotate and SetWindow are `crosshair`. Scroll is `ns-resize`. Fit, FlipHorizontal, FlipVertical, ApplyPreset, SetInvert, and SetColormap are `default`. An override replaces Pan and leaves Zoom at `zoom-in`. The earlier primary tool beats a later secondary tool. Dragging uses that command's dragging string.

`sync` assigns on every call. A setter already holding `default` for a missing group recorded two writes. A second `sync("idle")` on Pan recorded another write of `grab`.

A listener that throws on the first event leaves Zoom `active` and Pan `passive` with empty bindings. The cursor then syncs to `zoom-in`. The wrapper does not catch.

`cursor.ts` does not call `addEventListener`, does not import `pointer.ts` or `pointer.js`, and does not mention wasm, unsafe, or pixels. Tiers A, B, and C are not applicable. No new trait or generic was added. `registerTool` and `removeTool` forward. `setMode` does not.

`index.ts` publishes the four values. The exact key list in `index.test.ts` names them. While the probe was still on disk, `tsc --build` accepted imports of `CursorStyle` and `ToolStateEvent` from `index.ts`.

The design reproducer now says Zoom's `tool.modeChanged`, then `tool.activated`, then Pan's `tool.modeChanged`. That matches the code.

## Limitations and handoff

The product behaviour above was observed and then the mutations were restored. The smells are missing guards, not a wrong restored branch. Remediation should add tests. It should not change the emission order, the cursor table, or the demotion rule.

`docs/lld/public-api.md` still says F-072 will own cursors. The plan defers that section to story completion. It was not in this diff.

Modifiers do not split cursor ownership. A tool registered first on primary plus shift supplies the cursor. `bindingOwner` for unmodified primary can be a later tool. The plan does not pass modifiers into `cursor` or `sync`. This pass does not call that a defect.

The probe file was deleted. It must not be treated as coverage. No cargo, wasm-pack, or `bin/ocelli.sh` run. No commit and no stage. The next pass should review the tests that close S1 to S10 on a new tree.
