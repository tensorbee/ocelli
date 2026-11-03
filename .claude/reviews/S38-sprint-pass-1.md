# S38 review, pass 1

**Reviewed**: Range `2dcdaffd..HEAD` on branch `sprint/s38`. HEAD is `06eeef4d0506f647dc07a3b5d710f0e5b6d63863`. Tree is `a97c50a0214f769d67d00178a6bda664b4fcb1b1`. This review file is not part of that tree.

**Reviewer**: Independent subagent `01a19fce-7806-7123-b2aa-e02bdb10c0c0`. This reviewer did not write the S38 code, the design, or the story review.

**Result**: 0 defects, 0 smells, 0 nitpicks

Closing this sprint does not complete M12 and does not publish.

## Acceptance challenged

The approved plan is the contract. HLD section 12 requires both libraries to coexist. The feature flag is F-107 and is not this sprint. Shadow mode is F-109 and is not this sprint. Decision D1 keeps a new API. The plan's local decision is `attachSideBySide`, which enables only the Ocelli element and does not import another viewer.

AC1 rejects enabling the neighbour and rejects delivery of an Ocelli event to that neighbour. On a real `EventTarget`, the Ocelli target received `viewport.mounted` with `bubbles` false and `cancelable` false. The neighbour listener list stayed empty through attach, `frame.presented`, and `release`. A neighbour `other.library` listener ran before release and again after it. A spy on the neighbour `dispatchEvent` stayed at 0 until that neighbour event.

AC2 rejects one object used as both. `Object.is` returns null before `enableElement`. The target received no `viewport.mounted`. A later `enableElement` on that same object still mounted it, so the refusal did not leave a registration.

AC3 rejects a second `viewport.unmounted` and rejects enabling the neighbour from `release`. The committed test went red when `release` also called `enableElement` on the neighbour. On the restored tree the first `release` returned true, the neighbour listener still ran, the Ocelli dispatch returned false, and the second `release` returned false. The Ocelli target recorded one unmounted event.

AC4 rejects a non-object neighbour and a mounted dispatch that throws. `null`, `undefined`, a string, `false`, a function, and a symbol each returned null and did not mount the other argument. A `dispatchEvent` that throws on mounted returned null. Restoring that method and attaching again delivered one `viewport.mounted`, so no pair and no seam remained.

AC5 rejects replacement. A second attach with another neighbour returned null. The first seam still dispatched `frame.presented`. The first neighbour stayed. The other target received nothing.

Further combinations the committed file does not lock: an element already enabled through `enableElement` stays on that seam and the neighbour is not mounted. Two crossed pairs can live at once, and releasing one unmounts only its Ocelli element. A neighbour that already has its own seam stays mounted after `release`. A nested attach from `viewport.unmounted` keeps the new pair, and the old `release` does not disable it. A throwing unmounted `dispatchEvent` still makes the first `release` return true, and a later attach succeeds. The built `packages/core/dist/alongside.js` follows the same control flow on its own module instance.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `git status --short`, `git rev-parse HEAD`, `git rev-parse HEAD^{tree}` | `sprint/s38` before edits | Executed now | Status empty. HEAD `06eeef4d0506f647dc07a3b5d710f0e5b6d63863`. Tree `a97c50a0214f769d67d00178a6bda664b4fcb1b1` |
| `git diff --name-only 2dcdaffd..HEAD` | Open `5814ca3`, design `9eec472`, feature `06eeef4` | Executed now | 14 paths. `seam.ts`, `events.ts`, and `migrate.ts` are absent |
| `shasum -a 256` of `alongside.ts` and `seam.ts` | Clean tree before the mutation and after restore | Executed now | `fcd56226cadfa29f03206dbdf2a7b169466bb899d0161963baef3da89b1ed836` and `585c06cdc220e17f3073984acaa0dec5877cb7be372c29d963d973004582c779` |
| Header compare | `docs/lld/public-api.md` and `docs/lld/migration.md` against `docs/lld/README.md` | Executed now | Public-api strings are equal, 286 characters, and both end in F-106. Migration header and migration row cell are both `F-108, F-106` |
| `npx vitest run --config /tmp/s38-vitest.config.ts` | Probe `/tmp/s38-alongside-probe.test.ts`, 11 cases, source and `dist/alongside.js` | Executed now on the clean tree | Exit 0. 11 passed. Node v24.13.0. Vitest 4.1.11. The probe and the config were outside the repository and were deleted afterward |
| `npx vitest run packages/core/src/alongside.test.ts` after `release` also calls `enableElement(neighbour)` | Mutated sha256 `3b2a600315982fa1a9e69098f43ea7d8bfef10a943c6cbabdcb5613092c6169d` | Executed now | Exit 1. `releases only the Ocelli element` expected `["other.library"]` and received `["viewport.mounted", "other.library"]`. The other 4 tests passed |
| `cmp` of `/tmp/s38-alongside.ts.blob` back onto `packages/core/src/alongside.ts`, then `shasum -a 256` | Restored source | Executed now | `cmp` matched. sha256 returned to `fcd56226cadfa29f03206dbdf2a7b169466bb899d0161963baef3da89b1ed836`. `seam.ts` stayed `585c06cdc220e17f3073984acaa0dec5877cb7be372c29d963d973004582c779`. Status empty. Tree unchanged |
| `npx vitest run packages/core/src/alongside.test.ts packages/core/src/index.test.ts packages/core/src/seam.test.ts` | Restored tree | Executed now | Exit 0. 3 files, 15 tests passed. 5 alongside, 4 index, 6 seam |
| `npx tsc -p packages/core --noEmit --pretty false` | `packages/core` tsconfig | Executed now | Exit 0. No diagnostics. `--incremental false` was not passed |
| `npx eslint` on `alongside.ts`, `alongside.test.ts`, `index.ts`, and `index.test.ts` | Restored tree | Executed now | Exit 0. No diagnostics |
| `pgrep -lf 'cargo\|rustc\|wasm-pack'` | Process list after the suite | Executed now | No matching compiler |

No cargo, rustc, or wasm-pack command was started. The feature commit trailer records profile `feature`, corpus `pass`, and tree `a97c50a0214f`. That receipt was read and was not re-run. The story review `.claude/reviews/F-106-alongside-pass-1.md` is a prior receipt at 0 defects, 0 smells, and 0 nitpicks.

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

`git diff --name-only 2dcdaffd..HEAD` lists the design, the story review, `CHANGELOG.md`, the migration and public-api LLD files plus their index rows, the sprint ledger files, and four files under `packages/core/src`: `alongside.ts`, `alongside.test.ts`, `index.ts`, and `index.test.ts`.

`attachSideBySide` returns null or a `SideBySide`. `release` returns a boolean. The value is on the package object under that name. The source index export and the direct module export are the same function. `SideBySide` is a type-only export. `index.test.ts` passed the runtime key set. The gitignored `packages/core/dist/index.js` re-exports `attachSideBySide` from `./alongside.js`. That emit was read and was not rebuilt by this pass. `dist/alongside.js` imports `enableElement` from `./seam.js` only. The dist probe called that file's export, which is a separate module instance from the source.

The restored module imports `enableElement` and the `ElementSeam` type from `./seam.js` only. It does not import another viewer. `release` calls `disable` on the Ocelli seam and does not pass the neighbour to `enableElement` or `disableElement`. The same object is refused before `enableElement`. A second live Ocelli element is refused before `enableElement`. A null seam is not stored. The neighbour is stored by reference.

`release` deletes the `WeakMap` entry and then disables. A throw from unmounted delivery does not put the entry back. A nested attach during unmounted keeps the new pair. A second `release` does not send a second `viewport.unmounted`. Dispatch from the returned seam reaches only that element.

There is no feature flag and no second render. `seam.ts` was not edited. No wasm-bindgen, no `unsafe`, and no pixel path was added. Tiers A, B, and C stay not applicable, as the plan states.

Backlog F-106 is `done`. `CURRENT_SPRINT.md` marks F-106 `done`. F-X003 stays `in-progress`. F-X011 stays `pending`. F-107 and F-109 stay `pending` in S39. Unreleased names the neighbour and says nothing is published. `AS_BUILT.md` has an F-106 section. The new tracker cell starts with `not measured`. It does not record a duration or a ratio. `packages/core/package.json`, `packages/react/package.json`, and the workspace `Cargo.toml` are version `0.1.0`. D-22 remains in `docs/hld/DEVIATIONS.md` and still blocks publication. M12 is S37 to S39.

The on-disk `crates/ocelli-wasm/pkg/ocelli_wasm_bg.wasm` is 2,171,131 bytes. This pass did not rebuild it. The budget file still records 2,151,196 bytes with tolerance 0.05. The ceiling named in the sprint file is 2,258,755.

## Limitations and handoff

`pair.ocelli.disable()` and `disableElement` clear the F-105 seam and leave the F-106 entry in place. A later `attachSideBySide` on that element returns null and does not enable it. `enableElement` can still register a new seam while that entry remains. `release` then returns false, drops the entry, and does not disable the newer seam. The plan names `release` as the removal path and says a live pair refuses a second attach. This pass does not treat the other disable door as a defect. A caller who no longer holds the pair has no lookup that clears the entry.

The committed suite uses a fake `dispatchEvent`. The real `addEventListener` observations, the non-object samples other than `1`, the reattach after release, and the crossed pairs come from the probe. The probe is not part of the suite. The suite does lock the mounted list, the null returns, the single unmounted event, and the neighbour event list.

No DOM parent chain was built. No browser document was opened. The feature floor, the corpus gate, and the comparison report were not re-run. A clean result here is not a sprint-profile record.

Closing this sprint does not complete M12 and does not publish.

dwv, Horos, and the medical viewer Grok were not opened.
