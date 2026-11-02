Result: 0 defects, 3 smells, 0 nitpicks

# F-068 review, pass 2

**Reviewed**: uncommitted F-068 tree on `sprint/s25` at HEAD `06768e19630e6336bc7824ff16eb81168ffb7db7`. Paths read: `packages/core/src/tools.ts`, `packages/core/src/tools.test.ts`, the F-068 exports in `packages/core/src/index.ts`, the `PUBLISHED` list in `packages/core/src/index.test.ts`, `docs/lld/public-api.md` section "Tool lifecycle", the `public-api.md` row in `docs/lld/README.md`, and `.claude/plans/F-068-design.md`. Product source was mutated only for the checks below and then restored. `packages/core/src/tools.ts` sha256 started and ended at `ca6a90079cb00c974f602f978f4bcb8eb866024dd4678c4ceadfa0e217b6c2c9`.
**Author / reviewer**: The implementation author is not established from this working tree. This reviewer did not write the registry and did not resume the pass 1 reviewer.
**Result**: 0 defects, 3 smells, 0 nitpicks

## Acceptance challenged

AC1 to AC16 were checked against the plan's local rules. Modes, buttons, and demotion are not HLD formulas. The HLD was not used to invent a missing formula.

The pass 1 holes are closed. Returning `found.tools` and `found.viewports` from `group()` made `reads return copies` fail. Deleting `row.bindings = []` made `taking one binding demotes the whole owner` fail on the stored row. Writing `subject.bindings` only when `mode === "active"` made `passive and off store no bindings` fail on the stored passive row. `demoted.unshift` made `demotion names follow registration order` return `["A", "Z"]`.

New tests that this pass was not told were load-bearing also went red under a direct break. `Number.isInteger` instead of `Number.isSafeInteger` accepted `2 ** 53`. Dropping the duplicate-key refusal made the normalised-duplicate test return success. `splice(0, 1)` on detach left `[2, 3]`. A cross-group demotion scan made `two groups can own the same button` return `["Pan"]`. Removing the subject skip made shrinking return `demoted: ["Pan"]`.

Three wrong implementations stayed green. They are the smells below. A deleted probe on the restored source showed the current code does the right thing for each of them.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `npx vitest run packages/core/src/tools.test.ts packages/core/src/index.test.ts` | restored `tools.ts` `ca6a9007…c2c9`, Node v24.13.0, Vitest 4.1.11 | executed now, after every mutation was restored | exit 0. 25 tool tests and 4 index tests passed |
| `npx tsc -p packages/core --pretty false --noEmit` | `packages/core` project | executed now | exit 0. no diagnostics |
| `npx eslint` on `tools.ts`, `tools.test.ts`, `index.ts`, `index.test.ts` | those four files | executed now | exit 0 |
| `python3 scripts/prose_check.py` | `git ls-files` scope | executed now | exit 0. `OK: voice rules clean over 566 file(s)`. No `NameError` |
| `group()` returns `found.tools` and `found.viewports` | `-t "reads return copies"` | executed now, then restored | exit 1. re-read showed mode `off`, modifier `ctrl`, an auxiliary binding, tool `Zoom`, and viewport `9` |
| delete `row.bindings = []` | `-t "taking one binding demotes the whole owner"` | executed now, then restored | exit 1. Pan stayed passive and still held primary and auxiliary |
| assign bindings only when `mode === "active"` | `-t "passive and off store no bindings"` | executed now, then restored | exit 1. voluntary passive row still held primary |
| `demoted.unshift` | `-t "demotion names follow registration order"` | executed now, then restored | exit 1. received `["A", "Z"]` |
| `Number.isInteger` for viewport ids | `-t "viewport limits are the safe integer range"` | executed now, then restored | exit 1. `2 ** 53` was accepted |
| `bindingOwner` keys the raw binding | full `tools.test.ts` | executed now, then restored | exit 0. 25 passed. Smell S1 |
| demotion walks new binding keys | full `tools.test.ts` | executed now, then restored | exit 0. 25 passed. Smell S2 |
| duplicate-binding demotes other active tools, then refuses | full `tools.test.ts` | executed now, then restored | exit 0. 25 passed. Smell S3 |
| duplicate keys are skipped instead of refused | `-t "bindings that normalise to one key are refused"` | executed now, then restored | exit 1 |
| detach splices index 0 | `-t "detach keeps the remaining viewport order"` | executed now, then restored | exit 1. received `[2, 3]` |
| `setMode` scans every group | `-t "two groups can own the same button"` | executed now, then restored | exit 1. group `b` demoted `Pan` |
| subject is eligible for demotion | `-t "shrinking a binding set does not demote the subject"` | executed now, then restored | exit 1. `demoted: ["Pan"]` |
| `removeTool` sets the survivor mode to `active` and copies no binding | `-t "removing the active tool does not promote another"` | executed now, then restored | exit 0. Not filed. See limitations |
| `groups()` sorts ids | full `tools.test.ts` | executed now, then restored | exit 0. Not filed. See limitations |
| deleted probe, 9 tests | temporary `tools.probe.test.ts`, removed before the restored suite | executed now | exit 0. File is gone |

## Defects

None. The restored registry matches the approved plan and the lifecycle section. Refusals on the traced paths return before a write. One active owner per normalised binding inside a group. Demotion sets the previous owner to passive and clears every binding that owner had. A viewport id sits in at most one group. `viewportId(0)` attaches. Values that are not safe integers greater than or equal to zero are refused. Reads return deep copies. There is no module singleton, no listener, no command packing, and no hit or length math.

No false sentence was found in the plan or in the lifecycle section. The backlog rows for F-069, F-071, and F-072 match the dispatch, drawing, and cursor sentences. The README `public-api.md` row names the tool registry and F-068.

## Smells

### S1. Read-side modifier identity is not locked

**Where**: `packages/core/src/tools.ts` lines 425-429. `packages/core/src/tools.test.ts` lines 428-444 and lines 240-259.
**What**: `bindingOwner` normalises the query, then compares keys. Replacing that with `bindingKey` on the raw object left all 25 tests green. `modifier order does not make a second binding` stores `["shift", "alt"]` and queries `["alt", "shift"]`. `alt` already precedes `shift` in `POINTER_MODIFIERS`, so the query is the stored order. A lookup that does not sort still matches. `repeated modifiers collapse` queries the collapsed `PRIMARY_SHIFT` value, not `["shift", "shift"]`.
**Why it matters**: The plan says order and a repeated modifier do not make a second binding, and binding identity is the sorted modifier set. F-069 will call `bindingOwner` with event order. Dropping read-side normalisation then misses the owner, and this suite stays green.
**Evidence**: full `tools.test.ts` exit 0 under that mutation. A deleted probe on the restored source queried `["shift", "alt"]` and `["shift", "shift", "alt"]` and got Pan. The code is right. The suite does not lock it.
**Scope and regression**: Current HEAD behaviour is correct. Smallest regression is those two `bindingOwner` queries after a store of `["shift", "alt"]`, expecting Pan.

### S2. The registration-order fixture matches the binding list

**Where**: `packages/core/src/tools.test.ts` lines 457-467. `packages/core/src/tools.ts` lines 357-366.
**What**: The order test registers Z, then A, then M. Z owns primary. A owns auxiliary. M takes `[PRIMARY, AUXILIARY]` and expects `["Z", "A"]`. Replacing the walk of `found.tools` with a walk of the new binding keys, pushing each owner once, left all 25 tests green. On this fixture the binding list, the activation order, and reverse alphabetical order are the same sequence as registration.
**Why it matters**: The lifecycle section says the moved names come back in registration order. The plan only says the names are returned. The code walks registration order, and `unshift` now goes red, so pass 1's reversal hole is closed. A walk of the call's bindings is a different rule and stays green. A crossed call is what separates them.
**Evidence**: key-walk mutation, full `tools.test.ts`, exit 0. A deleted probe called `setMode` for M with auxiliary then primary. The restored code returned `["Z", "A"]`.
**Scope and regression**: Current behaviour matches the lifecycle sentence. Smallest regression is that crossed call, still expecting `["Z", "A"]`.

### S3. A duplicate refusal can demote another owner without a red test

**Where**: `packages/core/src/tools.ts` lines 346-348. `packages/core/src/tools.test.ts` lines 189-223 and lines 500-512.
**What**: On `duplicate-binding`, a mutated registry set every other active tool to passive, cleared its bindings, and then returned the refusal. All 25 tests stayed green. Both refusal tests register only Pan, so there is nobody else to demote.
**Why it matters**: The plan says duplicate bindings are refused before any demotion, and a refused call writes nothing. AC7 names "a bad call still demotes" as the rejected implementation. The previous owner would become passive while the caller sees a refusal.
**Evidence**: full `tools.test.ts` exit 0 under that mutation. A deleted probe activated Pan on primary, then sent Zoom two bindings that normalise to one key. The restored code returned `duplicate-binding` and left Pan active with that binding. A bad button in the same probe also left Pan active.
**Scope and regression**: The current `duplicate-binding` return is before the demotion loop. The hole is the suite. Smallest regression is the Pan-plus-Zoom case in that probe.

## Nitpicks

None.

## Verified clean

Pass 1 S1, S2, and S3 are closed by the four red mutations in the evidence table. The copy failure showed the nested modifier, the tool object, the tools array, and the viewport array. The passive failure was the stored row, not `bindingOwner`. The unshift failure was the two-name list.

`createToolRegistry` closes over its group array. A second registry does not see the first group's id. `registerTool` stores mode `off` and an empty binding list, does not replace a duplicate command, and allows the name `Pan` with command `Zoom`. `" pan"` and `"pan"` both stick as given. Whitespace-only ids and names are refused. Unknown commands, bad modes, and bad buttons are refused. The same mode and the same binding set return an empty demotion list. Shrinking drops the released button and does not demote the subject or the other owner. Primary and primary-plus-shift can both be active. Taking one button of a two-button owner clears the other button too. Removing the active tool leaves `bindingOwner` null. `viewportId(0)` attaches. `-1`, `1.5`, `NaN`, `Infinity`, and `2 ** 53` are refused. `Number.MAX_SAFE_INTEGER` attaches. Detach of a missing id is `missing-viewport`, and a middle detach keeps the neighbours in order. Destroying the owning group lets the other group attach.

`tools.ts` imports `STACK_COMMANDS` from `generated.ts`. The eleven names are not copied into this file. Grep of `tools.ts` found no `packCommands`, `writeCommands`, `submitCommands`, `addEventListener`, `wasm-bindgen`, `unsafe`, `globalThis`, or an `ocelli-geom` import. The source test checks the first, the third, and `addEventListener`. Value exports match the `PUBLISHED` list. Type exports match the erased-type comment. No patient data is in the new registry, its tests, or the lifecycle section.

A deleted nine-test probe, removed before the restored suite, also saw all eleven commands register, `bindingOwner` refuse a bad button, caller modifier arrays stay unaliased, a released button be taken by the other tool, a voluntary passive call leave the other tool active, one two-button owner appear once in `demoted`, `groups()` stay `["a", "c"]` after `destroyGroup("b")`, and that destroyed id become `unknown-group`. `viewportId(-0)` attaches and then collides with `viewportId(0)` as `duplicate-viewport` on the same group and `viewport-taken` on the other. `-0 < 0` is false, so this is not a second id. Not filed.

## Limitations and handoff

`groups().sort()` left all 25 tests green. The probe saw creation order, including order after a middle destroy. No shipped test reads two ids from `groups()`, so the plan's creation-order line is not locked. Not filed, because no test claims that order.

Setting the surviving tool's mode to `active` after `removeTool`, without copying a binding, left `removing the active tool does not promote another` green. `bindingOwner` stays null when the survivor has no bindings. Copying the removed binding onto that survivor would make `bindingOwner` return it, and that assertion would go red. The probe saw Zoom remain `off` with an empty list. Not filed, because the dispatcher-visible handoff is what that test rejects.

This pass did not run an "ignore shift" mutation or a "both stay active" mutation. Those shipped assertions expect no demotion, or Pan passive and Zoom active. They were not re-executed as edits.

F-070 paths were not reviewed and were not edited. `crates/ocelli-geom`, `Cargo.lock`, `docs/lld/geometry.md`, and `.claude/plans/F-070-design.md` were left untouched. Cargo, wasm-pack, and gates were not run. No GPU and no browser were used. The string scan in `the registry does not dispatch` would not see a renamed import.

The implementation behaviour checked here is correct. The three smells are suite holes. They block a clean pass until a later change adds the regressions named above and a new independent pass reviews that change. This pass is not sprint acceptance.
