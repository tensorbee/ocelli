Result: 0 defects, 1 smell, 0 nitpicks

# F-068 review, pass 3

**Reviewed**: uncommitted F-068 tree on `sprint/s25` at HEAD `06768e19630e6336bc7824ff16eb81168ffb7db7`. Paths read: `packages/core/src/tools.ts`, `packages/core/src/tools.test.ts`, the F-068 exports in `packages/core/src/index.ts`, the `PUBLISHED` list in `packages/core/src/index.test.ts`, `docs/lld/public-api.md` section "Tool lifecycle", the `public-api.md` row in `docs/lld/README.md`, `.claude/plans/F-068-design.md`, `.claude/reviews/F-068-pass-1.md`, and `.claude/reviews/F-068-pass-2.md`. Product source was mutated only for the checks below and then restored. `packages/core/src/tools.ts` sha256 started and ended at `ca6a90079cb00c974f602f978f4bcb8eb866024dd4678c4ceadfa0e217b6c2c9`. `cmp` against the pre-mutation copy was identical. No mutation was left in the tree.
**Author / reviewer**: The implementation author is not established from this working tree. This reviewer did not write the registry and did not resume reviewer `01a19979-20d5-72c0-bf0f-d98683579256` or `01a19990-b8e0-71c1-920d-af7869c03e40`.
**Result**: 0 defects, 1 smell, 0 nitpicks

## Acceptance challenged

AC1 to AC16 were checked against the plan's local rules. Modes, buttons, and demotion are not HLD formulas. A search of `docs/hld/` for a tool group, a passive mode, an auxiliary button, and `createToolRegistry` found none. The HLD was not used to invent a missing formula.

The pass 2 holes are closed. `bindingOwner` keyed with `bindingKey(binding)` made `modifier order does not make a second binding` return null for `["shift", "alt"]`. Walking the new binding list made `demotion names follow registration order` return `["A", "Z"]` for `[AUXILIARY, PRIMARY]`. Demoting every other active tool and then returning `duplicate-binding` made `a refused binding does not demote another tool` see Pan passive with an empty binding list. `demoted.unshift` returned `["A", "Z"]`. Returning the internal tool and viewport arrays from `group()` made `reads return copies` keep `ctrl`, an auxiliary binding, mode `off`, tool `Zoom`, and viewport `9`.

`groups().sort()` made `group ids stay in creation order` receive `["a", "b", "c"]`. Setting every surviving tool to `active` inside `removeTool` made `removing the active tool does not promote another` receive Zoom mode `active`. Those two were green in pass 2. They are red now.

One wrong implementation stayed green. It is the smell below. A deleted 13-test probe on the restored source showed the current code does the right thing for that case and for the other refusal paths it called.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `bindingOwner` uses `bindingKey(binding)` | `-t "modifier order does not make a second binding"` | executed now, then restored | exit 1. first query `["shift", "alt"]` returned null. The repeat query was not reached |
| demotion walks `normalized` | `-t "demotion names follow registration order"` | executed now, then restored | exit 1. `[AUXILIARY, PRIMARY]` returned `["A", "Z"]` |
| duplicate-binding demotes every other active tool, then refuses | `-t "a refused binding does not demote another tool"` | executed now, then restored | exit 1. refusal stood and Pan became passive with `bindings: []` |
| `demoted.unshift` | `-t "demotion names follow registration order"` | executed now, then restored | exit 1. received `["A", "Z"]` |
| `group()` returns `found.tools` and `found.viewports` | `-t "reads return copies"` | executed now, then restored | exit 1. re-read kept `ctrl`, auxiliary, mode `off`, tool `Zoom`, and viewport `9` |
| `groups().sort()` | `-t "group ids stay in creation order"` | executed now, then restored | exit 1. received `["a", "b", "c"]` |
| `removeTool` sets each survivor to `active` | `-t "removing the active tool does not promote another"` | executed now, then restored | exit 1. Zoom mode was `active` against expected `off` |
| duplicate-binding demotes owners of keys collected so far, then refuses | full `tools.test.ts` and `index.test.ts` | executed now, then restored | exit 0. 31 passed. Smell S1 |
| bad-binding demotes owners of keys collected so far, then refuses | full `tools.test.ts` | executed now, then restored | exit 0. 27 passed. Smell S1 |
| deleted probe, 13 tests | temporary `tools.probe.test.ts`, removed before the restored suite | executed now, on the restored source | exit 0. File is gone |
| same probe, overlap call only, under the overlap mutation | Zoom `[PRIMARY, PRIMARY]` while Pan owns primary | executed now, then restored | exit 1. Pan was passive with no bindings and the refusal was `duplicate-binding` |
| same probe, later bad binding, under the bad-binding mutation | Zoom `[PRIMARY, middle]` while Pan owns primary | executed now, then restored | exit 1. Pan was passive with no bindings and the refusal was `bad-binding` |
| `npx vitest run packages/core/src/tools.test.ts packages/core/src/index.test.ts` | restored `tools.ts` `ca6a9007…c2c9`, Node v24.13.0, Vitest 4.1.11 | executed now, after every mutation and after the probe file was removed | exit 0. 27 tool tests and 4 index tests passed |
| `npx tsc -p packages/core --pretty false --noEmit` | `packages/core` project | executed now | exit 0. no diagnostics |
| `npx eslint` on `tools.ts`, `tools.test.ts`, `index.ts`, `index.test.ts` | those four files | executed now | exit 0 |
| `python3 scripts/prose_check.py` | `git ls-files` scope | executed now | exit 0. `OK: voice rules clean over 566 file(s)` |

## Defects

None. The restored registry matches the approved plan and the lifecycle section. Refusals on the traced paths return before a write. One active owner per normalised binding inside a group. Demotion sets the previous owner to passive and clears every binding that owner had. Names come back in registration order. A viewport id sits in at most one group. `viewportId(0)` attaches. Values that are not safe integers greater than or equal to zero are refused. Reads return deep copies. There is no module singleton, no listener, no command packing, and no hit or length math.

No false sentence was found in the plan or in the lifecycle section. The backlog titles for F-069, F-071, and F-072 still match the dispatch, drawing, and cursor sentences. The README `public-api.md` row names the tool registry and F-068.

## Smells

### S1. A refused list that already names another tool's binding stays green

**Where**: `packages/core/src/tools.ts` lines 341-348. `packages/core/src/tools.test.ts` lines 508-529.
**What**: `a refused binding does not demote another tool` activates Pan on primary with no modifiers. Zoom's duplicate list is primary plus alt and shift, twice. Zoom's bad button is the only binding in that call. Neither refused list contains Pan's primary binding. Demoting every other active tool on `duplicate-binding` is now red. Demoting only the active owners of keys already collected in that call is not.
**Why it matters**: The plan says a refused call changes nothing, duplicate bindings are refused before any demotion, and a refusal returns before either write. The lifecycle section says a refused call writes nothing. AC7 names "a bad call still demotes" as the rejected implementation. The natural edit sits on the two returns inside the normalise loop. F-069 would then observe Pan as passive after a call the registry refused.
**Evidence**: The overlap mutation left `tools.test.ts` and `index.test.ts` at exit 0, 31 passed. A deleted probe sent Zoom `[PRIMARY, PRIMARY]` under that mutation and exited 1. Pan was passive and the refusal was still `duplicate-binding`. The bad-binding mutation left `tools.test.ts` at exit 0, 27 passed. The same probe sent `[PRIMARY, { button: "middle" }]` and exited 1, with Pan passive. On the restored source the probe's 13 tests exited 0, including both calls.
**Scope and regression**: Current behaviour is correct. The hole is the suite. Smallest regression is those two Zoom calls in the existing two-tool test, with the snapshot still required to match the pre-call group.

## Nitpicks

None.

## Verified clean

Pass 2 S1, S2, and S3 are closed by the red mutations in the evidence table. Pass 1's copy hole and `unshift` hole are still red. Pass 2 left `groups().sort()` and a mode-only `removeTool` promotion unfiled because no shipped test claimed them. Both are red now.

`createToolRegistry` from `packages/core/src/index.ts` closes over its group array. A second registry does not see the first registry's tools. All eleven `STACK_COMMANDS` register. The name `alias` may issue `Zoom`. `" pan"` and `"pan"` both stick as given. The stored modifier order for `["shift", "alt"]` is `["alt", "shift"]`. A caller push onto the array passed into `setMode` does not change that stored row. A second `setMode` with the modifiers reversed is a no-op. A second tool that sends the reversed modifiers demotes Pan and does not demote an auxiliary owner. `["shift", "shift"]` is the same binding as `["shift"]` for demotion. `[AUXILIARY, PRIMARY]` after `[PRIMARY, AUXILIARY]` is a no-op and leaves a secondary binding on the other tool. Four modifiers store as `["alt", "ctrl", "meta", "shift"]`, and the binding list stays in caller order.

`active-needs-binding`, `bindings-only-when-active`, a bad mode, an unknown tool, and an unknown group leave a second active owner in place. A duplicate list that repeats one of the subject's own bindings does not clear that subject. After a middle tool is removed, taking both of the remaining owner's buttons returns that owner once. Removing a non-active tool leaves the active tool active. A voluntary passive call clears the stored bindings. Destroying the owning group lets the other group attach. Unknown destroy, detach, remove, and `bindingOwner` refuse. A bad modifier on `bindingOwner` refuses and does not write. `viewportId(-0)` attaches and then collides with `viewportId(0)` as `duplicate-viewport` on the same group and `viewport-taken` on the other. A string id is `bad-viewport`.

`tools.ts` imports `STACK_COMMANDS` from `generated.ts`. The eleven names are not copied into this file. Grep of `tools.ts` found no `packCommands`, `writeCommands`, `submitCommands`, `addEventListener`, `wasm-bindgen`, `unsafe`, `globalThis`, or an `ocelli-geom` import. The source test checks the first, the third, and `addEventListener`. Value exports match the `PUBLISHED` list. Type exports match the erased-type comment. `TOOL_REFUSALS` is the sixteen names in the plan, in that order. No patient data is in the new registry, its tests, or the lifecycle section. No numeric cast was added. The `as readonly string[]` assertions only widen the const lists for `includes`.

## Limitations and handoff

This pass did not run an "ignore shift" mutation or a "both stay active" mutation. Those shipped assertions expect no demotion, or Pan passive and Zoom active. They were not re-executed as edits. The deletion of `row.bindings = []` was not repeated. The stored-row assertion from pass 2 is still present.

`active-needs-binding` and `bindings-only-when-active` are snapshotted in the shipped suite with only one tool. A deleted probe called both with a second active tool, and the restored code left that owner alone. Not filed. Those returns happen before a binding is collected, and no shipped test names a second tool for them.

`viewportId(-0)` is accepted because `-0 >= 0` is true and `Number.isSafeInteger(-0)` is true. It is not a second id. Not filed.

The string scan in `the registry does not dispatch` would not see a renamed import. F-070 paths were not reviewed and were not edited. Their sha256 values at the end matched the values taken at the start. The geometry row added to `docs/lld/README.md` in the same working tree was not judged. Cargo, wasm-pack, and gates were not run. No GPU and no browser were used.

The implementation behaviour checked here is correct. The one smell is a suite hole. It blocks a clean pass until a later change adds the two regressions named above and a new independent pass reviews that change. This pass is not sprint acceptance.
