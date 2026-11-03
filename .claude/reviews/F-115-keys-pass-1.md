# F-115 review, pass 1

**Reviewed**: Uncommitted worktree on `sprint/s41`. HEAD is `75898744503b42844d3d9e832532d3e62b90d44e`. `packages/core/src/keys.ts` and `packages/core/src/keys.test.ts` are untracked. The other F-115 edits are unstaged. This file is untracked. Nothing was staged or committed.

**Author / reviewer**: uncommitted implementation / 01a1a17c-2a37-7731-b6df-093a62c18120 / independent pass 1. This reviewer did not write the change.

**Result**: 0 defects, 0 smells, 1 nitpick

## Acceptance challenged

AC1 to AC7 were checked against `.claude/plans/F-115-design.md` and HLD section 26 in `docs/hld/23-performance-rules.md`. Section 26 says to batch pointer events into one command buffer per animation frame and never to cross the boundary per event. The HLD does not name a key map. The eight action names are the approved local plan. They are not an HLD formula. No file in this diff listens, reads the DOM, or moves a camera.

The unit file locks these cases.

- ArrowLeft, ArrowRight, ArrowUp and ArrowDown without shift return pan-left, pan-right, pan-up and pan-down.
- Shift with ArrowUp returns scroll-previous. Shift with ArrowDown returns scroll-next.
- Shift with ArrowLeft stays pan-left. Shift with ArrowRight stays pan-right.
- `+` and `=` return zoom-in. `-` returns zoom-out. Shift on `+` and shift on `-` stay on those zoom names.
- arrowleft, Enter and an empty string return null.
- The text of `keys.ts` has no `addEventListener` call.
- `index.ts` exports `keyAction`, and the published name list includes it.

An independent probe of the restored function was run and was not added to the tree. It also checked cases the unit file does not lock.

- `=` with shift stays zoom-in.
- arrowleft with shift returns null. ARROWLEFT, Arrowleft, a leading space, a trailing space, `_` with and without shift, Shift, NumpadAdd, Add, Equal and Minus return null.
- The index binding is the same function as the module export. `typeof` is function.

The ArrowLeft arm was changed from pan-left to pan-right. That is one constant. `npx vitest run packages/core/src/keys.test.ts` then failed two assertions. `pans on the arrow keys` expected pan-left and received pan-right at `keys.test.ts:16`. `keeps a shifted horizontal arrow as a pan` expected pan-left and received pan-right at `keys.test.ts:28`. Exit 1. Four tests still passed. The file was restored to the pre-mutation bytes. sha256 `205d9b72df263596ac64a9f920d56548ceec51dffd69ff9c58388a5e04f332eb` matched before the edit and after the restore. A byte compare against the saved original was empty. The restored suite exited 0.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `git status --short` and `git rev-parse HEAD` | Worktree before review | Executed now | HEAD `75898744503b42844d3d9e832532d3e62b90d44e` on `sprint/s41`. Five modified tracked files and two untracked module files. |
| Python compare of the public-api F-ID header with the last cell of the `public-api.md` row | Worktree text | Executed now | The strings are equal. Both have 47 ids. Both end with F-115. |
| Read of `keys.ts` for `addEventListener`, an import, `pointer`, `wasm-bindgen` and `unsafe` | All 38 lines | Executed now | None of those strings are present. There is no import. |
| Node probe file, then deleted | Restored `keyAction` and the index re-export. 13 inputs plus function identity. | Executed now | Exit 0. 1 test passed. The probe file is absent. |
| `npx vitest run packages/core/src/keys.test.ts` after ArrowLeft returned pan-right | Mutated worktree, not staged | Executed now | Exit 1. 2 failed and 4 passed. Failures at `keys.test.ts:16` and `keys.test.ts:28`. |
| Same file restored, then `npx vitest run` on `keys.test.ts` and `index.test.ts` | Restored worktree | Executed now | Exit 0. 10 passed. 6 in `keys.test.ts` and 4 in `index.test.ts`. |
| `npx tsc -p packages/core --noEmit --pretty false` | Worktree sources after the restore | Executed now | Exit 0. |
| `npx eslint` on `keys.ts`, `keys.test.ts`, `index.ts` and `index.test.ts` | Those four files, after the restore | Executed now | Exit 0. ESLint 9.39.5. |
| sha256 of `keys.ts` and `cmp` against the saved original | Restored bytes, length 813 | Executed now | sha256 `205d9b72df263596ac64a9f920d56548ceec51dffd69ff9c58388a5e04f332eb`. Compare empty. |

Environment: Node v24.13.0. Vitest 4.1.11. No cargo, rustc or wasm-pack was started. The floor gate and the corpus gate were not run.

## Defects

None.

## Smells

None.

## Nitpicks

### N1, the type-export list omits KeyAction

**Where**: `packages/core/src/index.test.ts:36` and `packages/core/src/index.ts:264`

**What**: The parenthetical names the type-only exports and stops at `MatrixTier`. This diff also exports `KeyAction`. `ColdPhases` and `StartupRole` were already unnamed. The runtime list does name `keyAction`.

**Why it is here**: The list is hand maintained. This diff publishes one type and does not name it. The runtime test cannot see a type export, so the omission does not fail a test. It does not change behaviour.

## Verified clean

- AC1. Unshifted arrows return the four pan names. Changing ArrowLeft to pan-right went red on the unshifted assertion.
- AC2. Shifted ArrowUp is scroll-previous. Shifted ArrowDown is scroll-next. A mutation that left shifted ArrowUp as pan-up would miss the assertion at `keys.test.ts:23`.
- AC3. Shift does not change ArrowLeft or ArrowRight. The same ArrowLeft mutation also went red on the shifted assertion.
- AC4. `+` and `=` return zoom-in. `-` returns zoom-out. The unit file locks shift for `+` and for `-`. The probe locked shift for `=`, which is the same branch as `+`.
- AC5. The match is case-sensitive. arrowleft is null. The probe also refused a shifted arrowleft, ARROWLEFT, Arrowleft, and both space paddings.
- AC6. `keys.ts` does not call `addEventListener`. It does not import `pointer.ts`. It contains no `wasm-bindgen` and no `unsafe`. It does not read the DOM and it does not move a camera.
- AC7. `keyAction` is exported from `index.ts`. The published list names it. `Object.keys` of the module matched that list. The probe saw the same function.
- The returned names are pan-left, pan-right, pan-up, pan-down, zoom-in, zoom-out, scroll-previous and scroll-next. Any other key returns null. The function is stateless.
- HLD section 26 is untouched. The function does not register a listener and does not cross the boundary per event. Pointer dispatch is not called. Pan, zoom and stack arithmetic are not changed.
- The public-api F-ID header equals the last cell of the public-api row in `docs/lld/README.md`. Both end with F-115.
- The keyboard section names the shift exception for the vertical arrows, the unchanged horizontal pan, zoom in on plus and equals, zoom out on minus, null for any other key, and a case-sensitive match. It says the function does not listen and does not move a camera. Tiers A, B and C are not applicable. The section calls the map the approved local plan.
- The changelog bullet names the arrow keys, plus, equals and minus. It says shift changes only the vertical arrows into slice scroll. It says any other key returns nothing and the function does not listen. That matches the function. The function returns null for the unbound keys.
- `events.ts` is not in the diff. `key.down` and `key.up` stay shell events.
- No new deviation. The design records none.
- `tsc --noEmit` and eslint on the four TypeScript files exited 0. The restored vitest run exited 0.

## Limitations and handoff

The floor gate and the corpus gate were not run. The story was not committed. `git diff` cannot show a restore of `keys.ts`, because that file is still untracked. The sha256 match and the empty byte compare are the restore evidence.

Behaviour matches the approved map. The type-export parenthetical is the only open note.
