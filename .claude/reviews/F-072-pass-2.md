Result: 0 defects, 2 smells, 0 nitpicks

# F-072 review, pass 2

**Reviewed**: Working tree on `sprint/s26` against HEAD `fc7ed4728c7eaf9e3a195a8fd2356b8ca4c8f8fa`. Untracked `packages/core/src/cursor.ts` and `packages/core/src/cursor.test.ts`. Modified `packages/core/src/index.ts`, `packages/core/src/index.test.ts`, and the reproducer sentence in `.claude/plans/F-072-design.md`. `cursor.ts` sha256 `fb1c8ae76c1275c1cd1fccff90dfb92089b61a76108f93304de528e91f11fb1e` matches the bytes pass 1 restored. The test file hash is `45bbb4e7da66ff09ae54b86ddb9d6155e698bdc607f4ef32a34470238d347eec`. `index.ts` and `index.test.ts` match the pass 1 hashes. Mutations were restored. `cmp` against the pre-mutation copies was identical. `git diff` does not include `tools.ts`, `pointer.ts`, `events.ts`, or `commands.ts`.
**Author / reviewer**: The author is the implementer of this uncommitted tree. This reviewer did not write the code and did not resume the pass 1 reviewer.
**Result**: 0 defects, 2 smells, 0 nitpicks

## Acceptance challenged

The contract is `.claude/plans/F-072-design.md`. Cursor strings and emission order are local decisions in that file. No HLD formula specifies them.

AC1. Reading the group after `setMode` made `activation follows the mode change` receive no events. The after-image already has the new mode, so the subject comparison does not fire. That rejects an emit taken from the committed group. This pass did not swap `tool.activated` ahead of `tool.modeChanged`. The assertion still requires `tool.modeChanged` then `tool.activated`.

AC2. Sorting the demoted names failed `demotion keeps registration order`. The events put `a` before `n`. The returned list stayed `n` then `a`, and the subject `winner` stayed out of that list. Copying the subject command onto the demoted event failed `demotion follows the subject`. The Pan event carried `Zoom`.

AC3. Subject events on an unchanged mode failed `a no op emits nothing`. The same edit failed `demotion is not activation`. A binding change that keeps the subject active must not emit a subject event. It still emits one `tool.modeChanged` for the demoted name.

AC4. A `tool.modeChanged` on `active-needs-binding` failed `a refusal is silent`. The assertion also requires the tool to stay `off` with no bindings. Other refusal names share that early return. This pass did not give each refusal its own fixture.

AC7. Choosing the first active tool instead of the primary owner failed `primary owns the cursor`. Received `grab`. Expected `zoom-in`.

AC8. Sorting active tools by name failed `the first active tool owns the cursor`. Received `zoom-in`. Expected `grab`.

AC10. Writing `style.cursor` only when the string changes failed `sync writes the style`. The counted setter stayed at 0. Expected 2.

AC11. Building the table from the override alone failed `an override replaces one cursor`. Zoom had no pair.

AC13. Adding `import "./pointer.js"` failed `the cursor module does not listen`.

These two wrong behaviours stayed green. They are the smells. `tool.activated` only when the previous mode is `off` left all 22 cursor tests passing. A cursor lookup hardcoded to group `g` left all 22 passing.

AC5, AC6, AC9, and AC12 were not mutated again this pass. The assertions are still in `cursor.test.ts`. Pass 1 had already driven those wrong implementations red.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `npx vitest run packages/core/src/cursor.test.ts packages/core/src/index.test.ts packages/core/src/events.test.ts` | Restored tree before mutations | Executed now. Not a receipt. | Exit 0. 34 passed. 22 cursor, 4 index, 8 events. |
| `npx tsc --build` | Same tree, before mutations and again after the last restore | Executed now | Exit 0 both times. |
| `npx eslint` on `cursor.ts`, `cursor.test.ts`, `index.ts`, `index.test.ts` | Restored tree before mutations | Executed now | Exit 0. |
| Plan prose scan of `.claude/plans/F-072-design.md` | Uncommitted reproducer sentence included | Executed now | 0 problems. |
| Sort `result.value.demoted` | `cursor.ts` only. Restored after. | Executed now | `cursor.test.ts -t "demotion keeps registration order"` exit 1. `a` before `n`. |
| `tool.activated` also for `passive` and `off` | `cursor.ts` only. Restored after. | Executed now | `-t "leaving active does not activate"` exit 1. Extra `tool.activated`. |
| Active filter widened to `mode !== "off"` | `cursor.ts` only. Restored after. | Executed now | `-t "a passive tool does not own the cursor"` exit 1. Received `grab`. |
| Unknown group returns `grab` | `cursor.ts` only. Restored after. | Executed now | `-t "an unknown group uses the default cursor"` exit 1. Received `grab`. |
| `sync` assigns only when the string changes | `cursor.ts` only. Restored after. | Executed now | `-t "sync writes the style"` exit 1. Writes 0. |
| Subject `tool.modeChanged` uses `groupId: ""` | `cursor.ts` only. Restored after. | Executed now | `-t "events use the group that was written"` exit 1. First id was empty. |
| Demoted `command` copied from the subject | `cursor.ts` only. Restored after. | Executed now | `-t "demotion follows the subject"` exit 1. Pan event command was `Zoom`. |
| Listener `try/catch` writes the before-image back and rethrows | `cursor.ts` only. Restored after. | Executed now | `-t "a listener throw leaves the registry written"` exit 1. Mode was `off`. |
| `import "./pointer.js"` | `cursor.ts` only. Restored after. | Executed now | `-t "the cursor module does not listen"` exit 1. |
| `export type` drops `CursorStyle` | `index.ts` only. Restored after. | Executed now | `npx tsc --build` exit 2. `cursor.test.ts:13` TS2305. |
| Subject block runs even when the mode is unchanged | `cursor.ts` only. Restored after. | Executed now | Exit 1. `a no op emits nothing` and `demotion is not activation`. |
| Refusal emits `tool.modeChanged` | `cursor.ts` only. Restored after. | Executed now | `-t "a refusal is silent"` exit 1. |
| `chosen = active[0]` | `cursor.ts` only. Restored after. | Executed now | `-t "primary owns the cursor"` exit 1. Received `grab`. |
| Group read after `setMode` | `cursor.ts` only. Restored after. | Executed now | `-t "activation follows the mode change"` exit 1. No events. |
| `tool.activated` only when previous mode is `off` | `cursor.ts` only. Restored after. | Executed now | Full `cursor.test.ts` exit 0. 22 passed. |
| Active tools sorted by name | `cursor.ts` only. Restored after. | Executed now | `-t "the first active tool owns the cursor"` exit 1. Received `zoom-in`. |
| Cursor lookup uses `"g"` instead of `options.groupId` | `cursor.ts` only. Restored after. | Executed now | Full `cursor.test.ts` exit 0. 22 passed. |
| Table is only the override | `cursor.ts` only. Restored after. | Executed now | `-t "an override replaces one cursor"` exit 1. Missing Zoom pair. |

Node v24.13.0. Vitest 4.1.11. TypeScript 5.9.3. ESLint 9.39.5. After the last restore, `cursor.ts` was `fb1c8ae76c1275c1cd1fccff90dfb92089b61a76108f93304de528e91f11fb1e` and the other three hashes matched the pre-mutation copies.

## Defects

None.

## Smells

### S1, reactivation from passive need not emit tool.activated

**Where**: `packages/core/src/cursor.ts:124`. The discarded call is `packages/core/src/cursor.test.ts:123`.
**What**: The activated guard was narrowed to `mode === "active" && subject.mode === "off"`. All 22 cursor tests stayed green. `leaving active does not activate` does set the tool active again from `passive`, then splices those events away before any expect.
**Why it is wrong**: The plan says `tool.activated` fires for the subject when the new mode is `active`. A demoted tool is `passive`. Setting it active again is a normal later call. The restored source emits `tool.modeChanged` and then `tool.activated` for that call. The suite does not.
**Evidence**: Full `npx vitest run packages/core/src/cursor.test.ts` exit 0. 22 passed. Restored afterward. The hash matched.
**Scope and regression**: A shell that treats `tool.activated` as the signal to follow would miss a tool coming back from `passive`. Smallest regression is to expect `tool.modeChanged` then `tool.activated` on the call at `cursor.test.ts:123`, with `previous` `passive`, before the splice. Do not change `cursor.ts` for this. The restored branch is already `mode === "active"`.

### S2, the cursor group id can be a constant

**Where**: `packages/core/src/cursor.ts:171`. The fixture is `packages/core/src/cursor.test.ts:341`.
**What**: `options.registry.group(options.groupId)` was changed to `options.registry.group("g")`. All 22 cursor tests stayed green. Every populated cursor fixture uses group `g`. The unknown-group fixture also creates empty group `g` and then asks for `missing`. A lookup of `g` finds no active tool and returns `default` for both phases and both `sync` writes.
**Why it is wrong**: The controller must read the group id it was given. An unknown group is `default` because that lookup fails. The pass 1 mutation, returning `grab` from the failure branch, is now red. The lookup argument is not. A controller for any other group can show `g`.
**Evidence**: Full `npx vitest run packages/core/src/cursor.test.ts` exit 0. 22 passed. Restored afterward. Separately, returning `grab` on a failed lookup failed `an unknown group uses the default cursor`.
**Scope and regression**: Two groups would keep the cursor of `g`. Smallest regression is an active Pan in group `g` and a controller whose `groupId` is `missing`, expecting `default` for both phases. The restored source already looks up `options.groupId`. Add the test. Do not change the lookup.

## Nitpicks

None.

## Verified clean

Pass 1 reported ten smells. Each of those mutations is red on this tree. The product file hash did not change. The new tests are what went red.

1. Demotion order. Tools `n` (Pan, secondary), `a` (Zoom, auxiliary), and `winner` (Rotate) taking both bindings. A sorted event list emitted `a` before `n`.
2. Leaving active. `active` to `passive` emitted an extra `tool.activated`. The same test also expects `active` to `off` to emit one `tool.modeChanged` and no `tool.activated`.
3. A passive Pan no longer supplies `grab`. Widening the filter to every mode other than `off` returned `grab`.
4. The unknown-group failure branch cannot return `grab`. Both phases and the `sync` writes are in that test. See S2 for the lookup argument, which this does not lock.
5. Two `sync("idle")` calls while the setter already holds `grab` must write twice. An equality guard left the count at 0.
6. Group `stack` is read back on the subject `tool.modeChanged`. A blank id failed. `demotion follows the subject` also puts group `g` on all three events.
7. The demoted Pan event cannot carry Zoom. The registration-order test also expects demoted commands Pan then Zoom.
8. A listener throw cannot roll the registry back. After the rethrow, Pan was `off` with no bindings. The restored source has no `try`.
9. The source guard rejects `pointer.js` as well as `pointer.ts` and `addEventListener`. The restored file imports `./generated.js` and `./tools.js` only.
10. `cursor.test.ts` imports `CursorController`, `CursorControllerOptions`, `CursorPair`, `CursorPhase`, `CursorStyle`, `ToolState`, and `ToolStateEvent` from `./index.js`. Removing `CursorStyle` from the `export type` block failed `tsc`. The value names `COMMAND_CURSORS`, `CURSOR_PHASES`, `createCursorController`, and `createToolState` stay on the exact key list in `index.test.ts`.

On the restored bytes the wrapper reads the group, calls `registry.setMode`, and emits only when that call is ok. A refusal returns the registry outcome and emits nothing. An identical `setMode` emits nothing. A binding change that keeps the subject active emits no subject event and no `tool.activated`. It emits one `tool.modeChanged` per demoted name, in the returned order. The subject pair is `tool.modeChanged` then `tool.activated` only when the stored mode changes to `active`. `previous` comes from the before-image. Demoted events use the demoted tool name and that tool's command, with mode `passive`.

`registerTool` and `removeTool` are forwarded. The registration test expects no events. A direct `registry.setMode` test expects no events. This pass did not mutate those two paths again.

The cursor table matches the eleven `STACK_COMMANDS`. Pan is `grab` and `grabbing`. Zoom is `zoom-in` for both phases. Rotate and SetWindow are `crosshair`. Scroll is `ns-resize`. Fit, FlipHorizontal, FlipVertical, ApplyPreset, SetInvert, and SetColormap are `default`. The controller spreads `COMMAND_CURSORS` and replaces one pair. Replacing the whole table left Zoom without a pair. Primary beats an earlier active tool. With no primary owner, registration order beats a sorted name. `sync` assigns `style.cursor` on every call. There is no `addEventListener` and no pointer import. No wasm, no pixels, and no `unsafe`. Tiers A, B, and C are not applicable. The plan says so, and the file has no tier branch. No new trait or generic was added.

The reproducer sentence now says Zoom `tool.modeChanged` from `off` to `active`, then `tool.activated` for Zoom, then Pan `tool.modeChanged` from `active` to `passive`, and `sync` idle writes `zoom-in`. `demotion follows the subject` locks the three events. `primary owns the cursor` locks the idle string for a primary Zoom. A separate Node import of the TypeScript sources failed on the `.js` specifier. That is an environment limit. It is not a product failure.

`events.test.ts` passed. `events.ts` was not edited. The catalogue names are still `tool.activated` and `tool.modeChanged`, with shell delivery. This module emits those names and does not write a ring record.

## Limitations and handoff

S1 and S2 are missing assertions. The restored product already follows the plan on both. Remediation should add tests only. It should not change the emission order, the cursor table, the group lookup, or the demotion rule.

`off` to `passive` and `passive` to `off` are not asserted. This pass did not mutate them. That note is not a reproduced failure.

A binding whose button is `primary` counts as owning primary even when modifiers are present. The plan does not pass modifiers into `cursor` or `sync`. This pass does not call that a defect.

`docs/lld/public-api.md` is unchanged. The plan defers that section until the story is completed. It was not in this diff.

AC5, AC6, AC9, AC12, and an activated-before-modeChanged swap were not mutated again. `tools.test.ts` and `pointer.test.ts` were not run. Those sources are not in the diff. No cargo, wasm-pack, or `bin/ocelli.sh` run. No commit and no stage.
