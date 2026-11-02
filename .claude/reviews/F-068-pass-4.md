Result: 0 defects, 1 smell, 0 nitpicks

# F-068 review, pass 4

**Reviewed**: uncommitted F-068 tree on `sprint/s25` at HEAD `06768e19630e6336bc7824ff16eb81168ffb7db7`. Paths read: `packages/core/src/tools.ts`, `packages/core/src/tools.test.ts`, the F-068 exports in `packages/core/src/index.ts`, the `PUBLISHED` list in `packages/core/src/index.test.ts`, `docs/lld/public-api.md` section "Tool lifecycle", the `public-api.md` row in `docs/lld/README.md`, `.claude/plans/F-068-design.md`, and `.claude/reviews/F-068-pass-3.md`. Product source was mutated only for the checks below and then restored. `packages/core/src/tools.ts` sha256 started and ended at `ca6a90079cb00c974f602f978f4bcb8eb866024dd4678c4ceadfa0e217b6c2c9`. `cmp` against the pre-mutation copy was identical. The temporary probe file was removed. No mutation was left in the tree.
**Author / reviewer**: The implementation author is not established from this working tree. This reviewer did not write the registry and did not resume reviewer `01a19979-20d5-72c0-bf0f-d98683579256`, `01a19990-b8e0-71c1-920d-af7869c03e40`, or `01a199a6-9025-7462-b7b7-a6561987d957`.
**Result**: 0 defects, 1 smell, 0 nitpicks

## Acceptance challenged

AC1 to AC16 were checked against the plan's local rules. Modes, buttons, and demotion are not HLD formulas. A search of `docs/hld/` for a tool group, a passive mode, an auxiliary button, and `createToolRegistry` found none. The HLD quote in the plan matches `docs/hld/03-architecture-and-crates.md`. Section 10 of `docs/hld/07-concurrency-and-typescript.md` places tool interaction state machines in the shell and leaves hit-testing in `ocelli-geom`. The HLD was not used to invent a missing formula.

Pass 3 S1 is closed. Inside the `duplicate-binding` return, demoting other active tools whose bindings are already in `keys` made `a refused binding does not demote another tool` fail. Pan was active on primary. Zoom sent `[PRIMARY, PRIMARY]`. The refusal stayed `duplicate-binding` and Pan became passive with an empty binding list. The same test also goes red when the `bad-binding` return demotes owners of keys already collected, when `active-needs-binding` demotes every other active tool, and when `bindings-only-when-active` does that too.

`bindingOwner` keyed with `bindingKey(binding)` instead of `bindingKey(normalized)` made `modifier order does not make a second binding` return null for `["shift", "alt"]`.

One wrong read stayed green. It is the smell below. A deleted probe showed the restored registry keeps a non-numeric viewport order, so the green suite is the test data and not the implementation.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| duplicate-binding demotes owners of keys already collected, then refuses | `-t "a refused binding does not demote another tool"`, Pan active on primary, Zoom `[PRIMARY, PRIMARY]` | executed now, then restored | exit 1. refusal stayed `duplicate-binding`. Pan became passive with `bindings: []` |
| `bindingOwner` uses `bindingKey(binding)` | `-t "modifier order does not make a second binding"` | executed now, then restored | exit 1. first query `["shift", "alt"]` returned null. The repeat query was not reached |
| `group()` sorts viewport ids numerically | full `tools.test.ts` and `index.test.ts` | executed now, then restored | exit 0. 31 passed. Smell S1 |
| same sort, probe attaches 3, then 1, then 2, and detaches 1 | temporary `tools.probe.test.ts`, removed | executed now, under the sort | exit 1. received `[2, 3]`, expected `[3, 2]` |
| same probe on the restored registry | restored `tools.ts` `ca6a9007…c2c9` | executed now, then the file was removed | exit 0. 7 passed, including `[3, 2]` |
| bad-binding demotes owners of keys already collected, then refuses | Zoom `[PRIMARY, middle]` while Pan owns primary | executed now, then restored | exit 1. refusal stayed `bad-binding`. Pan became passive with `bindings: []` |
| `active-needs-binding` demotes every other active tool, then refuses | Zoom `active []` while Pan owns primary | executed now, then restored | exit 1. the later snapshot showed Pan passive. The empty-list refusal had already matched |
| `bindings-only-when-active` demotes every other active tool, then refuses | Zoom `passive [PRIMARY]` while Pan owns primary | executed now, then restored | exit 1. the snapshot after that refusal showed Pan passive |
| `npx vitest run packages/core/src/tools.test.ts packages/core/src/index.test.ts` | restored `tools.ts` `ca6a9007…c2c9`, Node v24.13.0, Vitest 4.1.11 | executed now, after every mutation and after the probe file was removed | exit 0. 27 tool tests and 4 index tests passed |
| `npx tsc -p packages/core --pretty false --noEmit` | `packages/core` project | executed now | exit 0. no diagnostics |
| `npx eslint` on `tools.ts`, `tools.test.ts`, `index.ts`, `index.test.ts` | those four files | executed now | exit 0 |
| `python3 scripts/prose_check.py` | `git ls-files`, working-tree bytes | executed now | exit 0. `OK: voice rules clean over 566 file(s)` |

## Defects

None. The restored registry matches the approved plan and the lifecycle section. Refusals on the traced paths return before a write. One active owner per normalised binding inside a group. Demotion sets the previous owner to passive and clears every binding that owner had. Names come back in registration order. A viewport id sits in at most one group. `viewportId(0)` attaches. The negative, fractional, and unsafe values in the suite are refused. Reads return deep copies. There is no module singleton, no listener, no command packing, and no hit or length math.

No false sentence was found in the plan or in the lifecycle section. The backlog titles for F-069, F-071, and F-072 still match the dispatch, drawing, and cursor sentences. The README `public-api.md` row names the tool registry and F-068.

## Smells

### S1. Detach order is asserted only for already sorted viewport ids

**Where**: `packages/core/src/tools.test.ts` lines 590-601. The plan states the rule at lines 84 and 110. The copy is `packages/core/src/tools.ts` line 274. The removal is line 416.
**What**: `detach keeps the remaining viewport order` attaches `viewportId(1)`, `viewportId(2)`, and `viewportId(3)`, detaches 2, and expects `[1, 3]`. Sorting the copied viewport list numerically still returns `[1, 3]`. `tools.test.ts` and `index.test.ts` stayed green.
**Why it matters**: The plan says the viewport list preserves insertion order and that detach keeps the order of the rest. The lifecycle section does not repeat that sentence. The shipped test is the suite's claim that the order survives. A later edit can sort on read and that test still passes. A caller walking `group().viewports` would then see numeric order. The deleted probe attached 3, then 1, then 2, detached 1, and the sorted read returned `[2, 3]`.
**Evidence**: The sort mutation left both shipped files at exit 0, 31 passed. The probe under that mutation exited 1. On the restored source the same probe exited 0 and the viewports were `[3, 2]`.
**Scope and regression**: Current behaviour is correct. The hole is the suite. Smallest regression is attach `viewportId(3)`, then `viewportId(1)`, then `viewportId(2)`, detach `viewportId(1)`, and expect `[3, 2]`.

## Nitpicks

None.

## Verified clean

Pass 3 S1 is closed by the four red refusal mutations above. The raw `bindingKey(binding)` hole is still red. `demoted.unshift` and a `group()` return of the internal arrays were not replayed in this pass.

The deleted probe, run on the restored source, also showed these results. A secondary binding with modifiers `meta`, `ctrl`, `shift`, and `alt` is stored as `["alt", "ctrl", "meta", "shift"]`, and the reversed modifier list finds that owner. A second `setMode` of `[auxiliary, primary]` after `[primary, auxiliary]` returns `demoted: []` and leaves the group view unchanged. Replacing that set with primary and primary-plus-shift demotes the other owner of primary-plus-shift. All eleven `STACK_COMMANDS` register. Setting Pan passive leaves Zoom active on shift. Removing the middle tool leaves the remaining tools in registration order and leaves the active tool active. A caller push onto the modifier array passed into `setMode` does not change the stored row.

`createToolRegistry` closes over its group array. The shipped independence test passed. The source test for `packCommands`, `writeCommands`, and `addEventListener` passed, and a read of `tools.ts` found none of those calls. Value exports match the `PUBLISHED` list. Type exports match the erased-type comment. `TOOL_REFUSALS` is the sixteen names in the plan, in that order. No patient data is in the new registry, its tests, or the lifecycle section. No numeric cast was added. The `as readonly string[]` assertions only widen the const lists for `includes`.

Group ids in the shipped order test are `c`, `a`, `b`. Tool rows in the demotion test are `Z`, `A`, `M`. Those fixtures are not alphabetical. This pass did not re-execute a sort of either list.

## Limitations and handoff

This pass did not replay `demoted.unshift`, a return of the internal group arrays, `groups().sort()`, or a `removeTool` that sets every survivor active. Pass 3 recorded those as red. They were not run again.

No shipped test names `secondary`, `ctrl`, or `meta`. The deleted probe did, on the restored source. Not filed. The suite does not claim those names. `viewportId(-0)` was not rechecked.

The string scan in `the registry does not dispatch` would not see a renamed import. Not filed. `prose_check.py` reads working-tree bytes for `git ls-files` paths. Untracked `tools.ts` and `tools.test.ts` are outside that 566. Their block comments have no em dash and no prose semicolon. That was a separate read.

F-070 paths were not reviewed and were not edited. Their sha256 values at the end matched the values taken at the start. The geometry row added to `docs/lld/README.md` in the same working tree was not judged. Cargo, wasm-pack, and gates were not run. No GPU and no browser were used.

The implementation behaviour checked here is correct. The one smell is a suite hole. It blocks a clean pass until a later change adds the viewport-order regression named above and a new independent pass reviews that change. This pass is not sprint acceptance.
