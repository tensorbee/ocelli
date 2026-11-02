Result: 0 defects, 0 smells, 0 nitpicks

# F-068 review, pass 5

**Reviewed**: uncommitted F-068 tree on `sprint/s25` at HEAD `06768e19630e6336bc7824ff16eb81168ffb7db7`. Paths read: `packages/core/src/tools.ts`, `packages/core/src/tools.test.ts`, the F-068 exports in `packages/core/src/index.ts`, the `PUBLISHED` list in `packages/core/src/index.test.ts`, `docs/lld/public-api.md` section "Tool lifecycle" and the F-068 header line, the `public-api.md` row in `docs/lld/README.md`, `.claude/plans/F-068-design.md`, and `.claude/reviews/F-068-pass-4.md`. Product source was mutated only for the checks below and then restored. `packages/core/src/tools.ts` sha256 started and ended at `ca6a90079cb00c974f602f978f4bcb8eb866024dd4678c4ceadfa0e217b6c2c9`. `cmp` against the pre-mutation copy was identical. The temporary probe file was removed. No mutation was left in the tree.
**Author / reviewer**: The implementation author is not established from this working tree. This reviewer did not write the registry and did not resume reviewer `01a19979-20d5-72c0-bf0f-d98683579256`, `01a19990-b8e0-71c1-920d-af7869c03e40`, or `01a199a6-9025-7462-b7b7-a6561987d957`.
**Result**: 0 defects, 0 smells, 0 nitpicks

## Acceptance challenged

AC1 to AC16 were checked against the plan's local rules. Modes, buttons, and demotion are not HLD formulas. `docs/hld/03-architecture-and-crates.md` contains the plan's shell sentence, including tool interaction state. `docs/hld/07-concurrency-and-typescript.md` section 10 places tool interaction state machines in the shell and leaves hit-testing in `ocelli-geom`. A search of `docs/hld/` found no tool group, passive mode, auxiliary button, or `createToolRegistry`. The HLD was not used to invent a missing formula.

Pass 4 S1 is closed. Sorting the copied viewport list with `(left, right) => left - right` made `detach keeps the remaining viewport order` fail. The received list was `[2, 3]` and the expected list was `[3, 2]`.

The raw `bindingKey(binding)` hole is still red. `bindingOwner` keyed that way made `modifier order does not make a second binding` return null for `["shift", "alt"]`.

Sorting the copied tool list, sorting `groups()`, and `demoted.unshift` are also red. The discriminating fixtures are the ones named for this pass. Group ids are `c`, `a`, `b`. Tool rows are `Z`, `A`, `M`. Viewports are attached as `3`, then `1`, then `2`. The demotion call passes auxiliary before primary and still expects `Z` then `A`.

An independent case activated `A` before `Z` and then took primary before auxiliary. The restored registry still returned `["Z", "A"]`. That rejects activation order and binding order as the demotion rule. The same probe rejected a doubled demotion name, a passive call that cleared the other owner, a reversed binding list that rewrote the other tool, and a detach that kept the id taken.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| sort copied viewports with `(left, right) => left - right` | `-t "detach keeps the remaining viewport order"`, attach 3 then 1 then 2, detach 1 | executed now, then restored | exit 1. received `[2, 3]`, expected `[3, 2]` |
| `bindingOwner` uses `bindingKey(binding)` | `-t "modifier order does not make a second binding"` | executed now, then restored | exit 1. query `["shift", "alt"]` returned null |
| `group()` sorts tools with `localeCompare` | `-t "demotion names follow registration order"` | executed now, then restored | exit 1. tool rows arrived `A`, `M`, `Z` against `Z`, `A`, `M` |
| `groups()` sorts ids | `-t "group ids stay in creation order"` | executed now, then restored | exit 1. received `["a", "b", "c"]` against `["c", "a", "b"]` |
| `demoted.unshift` | `-t "demotion names follow registration order"` | executed now, then restored | exit 1. received `["A", "Z"]` against `["Z", "A"]` |
| deleted probe, 13 tests | temporary `tools.probe.test.ts`, removed before the restored suite | executed now, on restored `tools.ts` `ca6a9007...c2c9` | exit 0. 13 passed. File is gone |
| `npx vitest run packages/core/src/tools.test.ts packages/core/src/index.test.ts` | restored `tools.ts` `ca6a9007...c2c9`, Node v24.13.0, Vitest 4.1.11 | executed now, after every mutation and after the probe file was removed | exit 0. 27 tool tests and 4 index tests passed |
| `npx tsc -p packages/core --pretty false --noEmit` | `packages/core` project | executed now | exit 0. no diagnostics |
| `npx eslint` on `tools.ts`, `tools.test.ts`, `index.ts`, `index.test.ts` | those four files | executed now | exit 0 |
| `python3 scripts/prose_check.py` | `git ls-files`, working-tree bytes | executed now | exit 0. `OK: voice rules clean over 566 file(s)` |

## Defects

None. The restored registry matches the approved plan and the lifecycle section.

## Smells

None.

## Nitpicks

None.

## Verified clean

Refusals on the traced paths return before a write. One active owner per normalised binding inside a group. Demotion sets the previous owner to passive and clears every binding that owner had. Names come back in registration order. A viewport id sits in at most one group. `viewportId(0)` attaches. The negative, fractional, and unsafe values in the suite are refused. Reads return deep copies. There is no module singleton, no listener, no command packing, and no hit or length math.

No false sentence was found in the plan or in the lifecycle section. The header of `docs/lld/public-api.md` now names F-068. The backlog titles for F-069, F-071, and F-072 still match the dispatch, drawing, and cursor sentences. The README `public-api.md` row names the tool registry and F-068. `viewport.ts` is unchanged, so `ToolBinding` still has the caller-chosen name and the command.

The deleted probe, run on the restored source, also showed these results. Activating `A` before `Z` still demotes `Z` then `A`. One owner of both taken bindings is named once and loses both. Setting Pan passive leaves Zoom active on secondary, and setting Zoom off leaves Pan passive. All eleven `STACK_COMMANDS` register under a different name. A caller push of `ctrl` onto the modifier array passed into `setMode` does not change the stored row. Detach of viewport 3 lets the other group attach it. A secondary binding with `meta`, `ctrl`, `shift`, and `alt` is stored as `["alt", "ctrl", "meta", "shift"]`, and a reversed list with a repeated `alt` finds that owner. Unknown destroy, remove, setMode, attach, detach, bindingOwner, and group return `unknown-group` and leave `groups()` empty. A group copy taken before demotion still shows Pan active. A repeated `shift` that normalises onto the same key is `duplicate-binding` and leaves Pan active. `[auxiliary, primary]` after `[primary, auxiliary]` returns `demoted: []` and leaves the secondary owner unchanged. Removing middle tool `A` leaves `Z` then `M`, and `M` stays active.

`createToolRegistry` closes over its group array. The shipped independence test passed. The source test for `packCommands`, `writeCommands`, and `addEventListener` passed, and a read of `tools.ts` found none of those calls. The imports are `STACK_COMMANDS` and types from `generated.ts`, plus `ToolBinding` and `ViewportId` from `viewport.ts`. Value exports match the `PUBLISHED` list. Type exports match the erased-type comment. `TOOL_REFUSALS` is the sixteen names in the plan, in that order. No patient data is in the new registry, its tests, or the lifecycle section. No numeric cast was added. The `as readonly string[]` assertions only widen the const lists for `includes`.

Modifier order and the order of bindings inside one `setMode` were not treated as a smell. The plan defines both as set identity. The probe's reversed binding list was a no-op on the restored source.

## Limitations and handoff

No shipped test names `secondary`, `ctrl`, or `meta` on its own. The deleted probe did, on the restored source. Not filed. Those names sit on the same `includes` checks as `primary`, `alt`, and `shift`, and the published lists contain them.

`viewportId(-0)` was not rechecked. Not filed.

The string scan in `the registry does not dispatch` would not see a renamed import. The import list was read and does not call the dispatcher. Not filed.

`prose_check.py` reads working-tree bytes for `git ls-files` paths. Untracked `tools.ts` and `tools.test.ts` are outside that 566. Their comments have no em dash and no prose semicolon. That was a separate read.

F-070 paths were not reviewed and were not edited. The sha256 of `Cargo.lock`, `crates/ocelli-geom/Cargo.toml`, `crates/ocelli-geom/src/lib.rs`, `crates/ocelli-geom/src/measure.rs`, `crates/ocelli-geom/tests/measure.rs`, `docs/lld/geometry.md`, `.claude/plans/F-070-design.md`, and `.claude/reviews/F-070-pass-1.md` matched at the end and at the start. The geometry row added to `docs/lld/README.md` in the same working tree was not judged. Cargo, wasm-pack, and gates were not run. No GPU and no browser were used.

The order rules checked here go red when replaced. This pass did not find a wrong result on the restored registry. It is a bounded review of the F-068 tree, not sprint acceptance.
