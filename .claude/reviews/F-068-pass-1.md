Result: 0 defects, 3 smells, 0 nitpicks

# F-068 review, pass 1

Reviewed the uncommitted F-068 tree on `sprint/s25` at HEAD `06768e19630e6336bc7824ff16eb81168ffb7db7`. F-070 paths were not reviewed. Product source was mutated only for the checks below and then restored. `packages/core/src/tools.ts` sha256 is `ca6a90079cb00c974f602f978f4bcb8eb866024dd4678c4ceadfa0e217b6c2c9`, the same hash taken before those mutations.

## Defects

None. The registry matches the approved plan and the tool-lifecycle section in `docs/lld/public-api.md`. Refusals write nothing. One active owner per normalised binding inside a group. Demotion sets the previous owner to passive and clears every binding that owner had. Names come back in registration order. A viewport sits in at most one group. `viewportId(0)` attaches. Every id that is not a safe integer greater than or equal to zero is refused. Reads return deep copies. There is no module singleton, no listener, no command packing, and no hit or length math. Modes are `active`, `passive` and `off`.

No false sentence was found in the plan or in the lifecycle section. The README `public-api.md` row names the tool registry and F-068.

## Smells

### S1. The copy test does not write through the returned array

`packages/core/src/tools.test.ts:305-313`

`reads return copies` spreads `view.value.tools` into a new array and pushes onto that copy. Replacing the `group` copy at `packages/core/src/tools.ts:273` with the internal `found.tools` array, and the viewport spread at line 274 with the internal array, left all 19 tests in `tools.test.ts` green. The wrong implementation named for AC13 is not rejected.

The implementation does copy. `group` maps `copyTool` and copies the viewport list at `packages/core/src/tools.ts:271-274`. `bindingOwner` copies at line 435. A deleted probe mutated a returned tool mode, a returned bindings array, a nested modifiers array, a returned viewport list, the array from `groups()`, the `registerTool` value, and the `bindingOwner` value. The next read was unchanged.

### S2. A binding left on a passive tool stays green

`packages/core/src/tools.test.ts:73-81` and `packages/core/src/tools.test.ts:125-129`

`bindingOwner` returns a row only when its mode is `active` (`packages/core/src/tools.ts:430-434`). Stored bindings on a passive row are invisible to that read.

Removing `row.bindings = []` at `packages/core/src/tools.ts:376` failed only `a second active tool takes the binding`, which expects length 0 at line 56. `taking one binding demotes the whole owner` stayed green. It checks that auxiliary has no active owner, not that Pan's stored list is empty.

Assigning `subject.bindings` only when the mode is `active`, and skipping the write otherwise, left all 19 tests green. `passive and off store no bindings` snapshots the refusal, then after a successful passive call it only checks `bindingOwner`. The stored row is not read. The only successful `off` call in the file, at line 335, is a no-op on a tool that is already `off`.

The implementation does clear. Demotion writes `passive` and an empty list at lines 375-376. The subject write at lines 378-379 stores the normalised list, which is empty unless the mode is `active`. A deleted probe read a voluntary passive row, a voluntary `off` row, and a multi-binding demotion, and each non-active list was empty.

### S3. One demoted name cannot show registration order

`packages/core/src/tools.test.ts:47-49`

Every shipped demotion expectation has one name. Replacing `demoted.push` with `demoted.unshift` at `packages/core/src/tools.ts:365` left all 19 tests green. A deleted probe registered `Z` then `A`, took both bindings, and got `["Z", "A"]`. The code walks `found.tools` and pushes, so the order is registration order. The shipped tests would not see a reversal.

## Nitpicks

None.

## Checks

- Read `.claude/plans/F-068-design.md`, `packages/core/src/tools.ts`, `packages/core/src/tools.test.ts`, the F-068 exports in `packages/core/src/index.ts`, the `PUBLISHED` list in `packages/core/src/index.test.ts`, `docs/lld/public-api.md` section "Tool lifecycle", and the `public-api.md` row in `docs/lld/README.md`.
- `STACK_COMMANDS` in `packages/core/src/generated.ts` is the eleven names, and `tools.ts` imports that array rather than a second list.
- `tools.ts` has no `packCommands`, `writeCommands`, `submitCommands`, `addEventListener`, `wasm-bindgen`, `unsafe`, `globalThis`, or an `ocelli-geom` import.
- `npx vitest run packages/core/src/tools.test.ts packages/core/src/index.test.ts` exited 0. 23 tests passed.
- `npx tsc -p packages/core --pretty false --noEmit` exited 0.
- `npx eslint` on the four TypeScript files exited 0.
- `python3 scripts/prose_check.py` reported clean over 566 files. The new lifecycle prose has no semicolon and no em dash. The plan's lifecycle sentences match the code.
- A temporary probe, deleted before this file was written, ran 8 tests. They covered two groups owning the same button, deep copies, demotion order `Z` then `A`, no self-demotion when a set is reordered or shrunk, a normalised duplicate binding refused with no write, passive and `off` clearing bindings, `2^53` refused, `Number.MAX_SAFE_INTEGER` accepted, detach keeping the remaining order, and whitespace tool names staying distinct.
- `viewportId(-0)` is accepted. The guard is `Number.isSafeInteger(id) && id >= 0`, and `-0 < 0` is false. A later `viewportId(0)` on another group is `viewport-taken`, so `-0` is not a second viewport. Not filed.
- The author's `continue` mutation was not repeated. Line 49 expects `["Pan"]`, and line 365 still pushes. The tree matches the pre-mutation hash.
- F-070 files were left untouched.
