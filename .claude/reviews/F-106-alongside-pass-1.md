# F-106 review, pass 1

**Reviewed**: Uncommitted F-106 work on sprint/s38. HEAD is 9eec472. The product paths are untracked `packages/core/src/alongside.ts` (sha256 `fcd56226cadfa29f03206dbdf2a7b169466bb899d0161963baef3da89b1ed836`) and `packages/core/src/alongside.test.ts`, plus the export lines in `packages/core/src/index.ts` and `packages/core/src/index.test.ts`. This review file is not part of that diff.

**Reviewer**: Independent subagent `01a19fba-77c1-7063-b817-d9df9fb48fa6`. This reviewer did not write the code or the design.

**Result**: 0 defects, 0 smells, 0 nitpicks

## Acceptance challenged

AC1 to AC5 were run from `alongside.test.ts`, then again from a deleted probe that imported `attachSideBySide` and the package index. The probe used real `EventTarget` listeners rather than the fake `dispatchEvent` recorder.

AC1 rejects enabling the neighbour and rejects delivery of an Ocelli event to the neighbour. The Ocelli target received `viewport.mounted`. The neighbour did not. A neighbour listener still ran for `other.library`. `frame.presented` stayed on the Ocelli target. The probe held the caller object as `detail`, with `bubbles` false and `cancelable` false. A spy on the neighbour `dispatchEvent` stayed at 0 through attach, Ocelli dispatch, and `release`.

AC2 rejects enabling one element as both sides. The same object returns null and receives no `viewport.mounted`. That call was executed. The source edit that deletes the `Object.is` check was not the mutation run below.

AC3 rejects a second `viewport.unmounted` and rejects enabling the neighbour from `release`. The executed mutation called `enableElement` on the neighbour inside `release`. The release test went red because the neighbour then saw `viewport.mounted`. On the restored tree, a neighbour that had its own seam stayed mounted after `release`, and a later Ocelli dispatch on it still returned true. A second `release` returned false. After the first `release`, a new `attachSideBySide` on that Ocelli element mounted again, so the pair entry had been removed.

AC4 rejects a throw and rejects leaving the Ocelli element enabled. A non-object neighbour returns null and the Ocelli element is not mounted. The unit file uses `1`. The probe also used `null`, `undefined`, a string, `false`, a function, and a symbol. None threw. A `dispatchEvent` that throws on mounted returns null, does not store a pair, and leaves the neighbour listener list empty. A later attach of a fresh element succeeds. A plain object with no `dispatchEvent` is refused as the Ocelli element and accepted as the neighbour. That matches `enableElement` owning the target check, and this module not calling the neighbour.

AC5 rejects replacement of a live pair. A second attach of the same Ocelli element with another neighbour returns null. The first seam still dispatches `frame.presented`. The first neighbour stays. The other element receives nothing. An element already enabled through `enableElement` is also refused, the existing seam still dispatches, and the neighbour is not mounted. Two crossed pairs can live at once. Releasing one unmounts only its Ocelli element.

The plan's removal order was checked with a nested attach from `viewport.unmounted`. The new pair was returned and still dispatched after the outer `release` returned true. Only one unmounted event was recorded before the nested `release`. That is the delete-before-disable order. A throwing `dispatchEvent` on unmount still made the first `release` return true, and a later attach succeeded after the method was restored.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `npx vitest run packages/core/src/alongside.test.ts` | `alongside.ts` sha256 `fcd56226cadfa29f03206dbdf2a7b169466bb899d0161963baef3da89b1ed836`, five AC tests | Executed now, before the mutation | Exit 0. 5 passed. |
| Same vitest file after `release` also calls `enableElement(neighbour)` | Mutated sha256 `3b2a600315982fa1a9e69098f43ea7d8bfef10a943c6cbabdcb5613092c6169d` | Executed now | Exit 1. `releases only the Ocelli element` failed. Expected `["other.library"]` and received `["viewport.mounted", "other.library"]`. The other 4 tests passed. |
| `cmp` of `/tmp/f106-alongside.ts.blob` back onto `packages/core/src/alongside.ts`, then `shasum -a 256` | Restored source | Executed now | `cmp` matched. sha256 returned to `fcd56226cadfa29f03206dbdf2a7b169466bb899d0161963baef3da89b1ed836`. `seam.ts` stayed `585c06cdc220e17f3073984acaa0dec5877cb7be372c29d963d973004582c779`. |
| `npx vitest run packages/core/src/alongside.test.ts packages/core/src/index.test.ts packages/core/src/seam.test.ts packages/core/src/events.test.ts` | Restored tree. Node v24.13.0. Vitest 4.1.11. | Executed now | Exit 0. 4 files, 23 tests passed. |
| `npx tsc -p packages/core --noEmit --pretty false` | `packages/core` tsconfig, tests included | Executed now | Exit 0. No diagnostics. `--incremental false` was not passed. |
| `npx vitest run --config tmp-f106-vitest.config.ts` | Probe `/tmp/f106-alongside-probe.test.ts`, 6 cases, importing source | Executed now on the restored tree | Exit 0. 6 passed. The probe and the config were deleted afterward. |
| `npx eslint` on `alongside.ts`, `alongside.test.ts`, `index.ts`, and `index.test.ts` | Restored tree | Executed now | Exit 0. No diagnostics. |
| `git status --porcelain` and `git branch --show-current` | sprint/s38 at `9eec472` | Executed now, after restore and after deleting the probe | `seam.ts`, `events.ts`, and `migrate.ts` are not modified. |

No cargo, rustc, or wasm-pack command was started.

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

- `attachSideBySide` has the planned signature and returns null or a `SideBySide`. `release` returns a boolean. The value is on the package object under that name, and the probe saw the same function as the direct module export. `SideBySide` is a type-only export. `index.test.ts` passed the exact runtime key set.
- The module imports `enableElement` and the `ElementSeam` type from `./seam.js` only. It does not import another viewer. `release` calls `disable` on the Ocelli seam and does not pass the neighbour to `enableElement` or `disableElement`.
- The same object is refused before `enableElement`. A second live Ocelli element is refused before `enableElement`. A null seam is not stored. The neighbour argument is stored by reference.
- `release` deletes the `WeakMap` entry and then disables. A throw from unmounted delivery does not put the entry back. A nested attach during unmounted keeps the new pair. A second `release` does not send a second `viewport.unmounted`.
- Dispatch from the returned seam reaches only that element. An already enabled element is left on its existing seam when attach returns null. Crossed pairs do not unmount each other.
- There is no feature flag and no second render. `seam.ts` was not edited. No wasm-bindgen, no `unsafe`, and no pixel path was added. Tiers A, B, and C stay not applicable, as the plan states.
- The restored `alongside.ts` hash is the hash given for the start of this pass.

## Limitations and handoff

The unit file does not attach again after `release`, and its non-object sample is the number `1`. Those two observations come from the probe, which is not part of the suite. The suite does lock the mounted list, the null returns, the single unmounted event, and the neighbour event list.

`pair.ocelli.disable()` and `disableElement` clear the F-105 seam and leave the F-106 entry in place. A later `attachSideBySide` on that element returns null and does not enable it. `release` then returns false and drops the entry, after which attach works. The plan names `release` as the removal path and says a live pair refuses a second attach. This pass does not treat the other disable door as a defect. A caller who no longer holds the pair has no lookup that clears the entry. The element stays refused by `attachSideBySide` while it remains reachable.

No DOM parent chain was built. The delivered event's `bubbles` and `cancelable` flags were false. Node v24.13.0 supplied `EventTarget`.

`docs/lld/migration.md` has no F-106 paragraph. `docs/lld/public-api.md` does not name `attachSideBySide`. Workflow assigns those LLD edits to completion, so they are outside this implementation diff. The story is still uncommitted. The feature profile in the plan was not run.

dwv, Horos, and the medical viewer Grok were not opened.
