# F-107 review, pass 2

**Reviewed**: Uncommitted F-107 on `sprint/s39`. HEAD is `c446a84bd74a59f4416fb92f1f1eec799bba1844`. The product paths are untracked `packages/core/src/flag.ts` (sha256 `c4daca94e72ace24876443d7c3923dc816e80834b00b7627cb115d34b709753f`) and `packages/core/src/flag.test.ts`, plus the export lines in `packages/core/src/index.ts` and `packages/core/src/index.test.ts`. This review file is not part of that diff. `seam.ts` is still `585c06cdc220e17f3073984acaa0dec5877cb7be372c29d963d973004582c779`. `alongside.ts` is still `fcd56226cadfa29f03206dbdf2a7b169466bb899d0161963baef3da89b1ed836`.

**Author / reviewer**: Author is the parent session. Reviewer is independent subagent `01a19ffc-32cd-7bc2-b294-59c58b7142d6`. This reviewer did not write the code, the design, or pass 1.

**Result**: 0 defects, 0 smells, 0 nitpicks

## Acceptance challenged

AC1 to AC5 remain in `flag.test.ts`. The restored file passes all 10 tests in that file. Forcing `kept` to `true` makes the two new rollback tests return a seam, so those assertions see the mount path that pass 1 missed.

The corrected probe imported `flag.ts`, `index.ts`, and `seam.ts` from source and used real `EventTarget` listeners. `enableFlagged`, `kindEnabled`, and `setKindEnabled` on the package index are the same functions as the module exports. The probe was deleted after the run.

The old one-element failure is now rejected. A mounted listener called `setKindEnabled("stack", false)` and that call returned true. After `enableFlagged` returned, the kind was off, the seam was null, and `frame.presented` did not dispatch. A second off call returned true and the element still did not dispatch. Removing that listener and turning stack on again produced a seam that did dispatch.

The old two-element failure is now rejected. The second mount turned stack off. The first seam did not dispatch. The second call returned null. Both elements received `viewport.unmounted`. An enable of a third element from the first element's unmount returned null, and that third element did not dispatch.

Adjacent cases that also held: stack off from a stack mount is refused while MPR is on, and that in-flight stack seam stays. Volume off from a volume mount drops only the volume element. Stack and MPR keep dispatching. An element already enabled through `enableElement` is not stored, and turning stack off does not disable it. `null` and a plain object return null. If the mounted listener turns stack off and then back on before returning, the in-flight element is still dropped, and a different element enabled from the rollback unmount stays. A nested `setKindEnabled("stack", true)` during that unmount returns false.

A throwing listener on a real `EventTarget` is not the null-seam path. `enableFlagged` does not throw. The element stays mounted and dispatches. A second enable returns null because the first seam is stored. Node reported `mounted-boom` as an uncaught exception. The unit test that expects a null seam replaces `dispatchEvent` so the method itself throws. That path still does not store the element, and a later enable works. The two inputs are not the same, and the real listener result matches `enableElement` returning a seam.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `npx vitest run` on `flag.test.ts`, `index.test.ts`, `events.test.ts`, `seam.test.ts`, and `alongside.test.ts` | `flag.ts` sha256 `c4daca94e72ace24876443d7c3923dc816e80834b00b7627cb115d34b709753f`. Node v24.13.0. Vitest 4.1.11. | Executed now, before the mutation | Exit 0. 5 files, 33 tests passed. |
| `npx tsc -p packages/core --noEmit --pretty false` | `packages/core` tsconfig, tests included. TypeScript 5.9.3. | Executed now | Exit 0. No diagnostics. `--incremental false` was not passed. |
| `npx eslint` on `flag.ts`, `flag.test.ts`, `index.ts`, and `index.test.ts` | ESLint 9.39.5. Same tree. | Executed now | Exit 0. No diagnostics. |
| `npx vitest run packages/core/src/flag.test.ts` after `const kept = true` | Mutated sha256 `0329ae7adbf346d46fe32cde6bd2115473c5b4c3b11c2ffe60df2ab5e8d8f2c8` | Executed now | Exit 1. 2 failed and 8 passed. Both new rollback tests received a seam where they expect null. |
| `cmp /tmp/f107-pass2-flag.ts.blob packages/core/src/flag.ts` and `shasum -a 256` | Restored source | Executed now | `cmp` exit 0. sha256 returned to `c4daca94e72ace24876443d7c3923dc816e80834b00b7627cb115d34b709753f`. |
| `npx vitest run packages/core/src/flag.test.ts` | Restored hash above | Executed now, after the restore | Exit 0. 10 tests passed. |
| First `npx vitest run --config /tmp/f107-pass2-vitest.config.ts` | Probe left the off listener attached for the re-enable | Executed now, discarded as a probe mistake | Exit 1. 7 passed and 1 failed. The logged rollback was already null and not dispatching. The failure was the later enable hitting that same listener. |
| Corrected probe, same command, 9 cases | Real `EventTarget`, source imports, restored `flag.ts` | Executed now | Exit 0. 9 tests passed. Probe and config were deleted afterward. |
| `rg addEventListener\(` on `flag.ts` | Restored `flag.ts` | Executed now | Exit 1. No matches. `events.test.ts` passed in the first vitest run, and that test still expects the listener files to be only `pointer.ts`. |

No cargo, rustc, or wasm-pack command was started. Exit codes were taken from `$?` on the command itself.

## Closed

### D1, an in-flight enable stays mounted after its kind turns off

**Severity**: defect, closed on this tree

**Where it was**: `packages/core/src/flag.ts` stored the seam only after `enableElement` returned. Pass 1 saw a mounted listener turn the kind off before that store. The off path did not see the element. The function then stored the seam. A later off call was a no-op, and `frame.presented` still dispatched.

**What the tree does now**: `enableFlagged` reserves the element and adds it to that kind's set before `enableElement` (`flag.ts` lines 127 to 129). The off path deletes the reservation (line 94) and does not disable the in-flight seam, because it is not stored yet. After `enableElement` returns, `kept` is true only when the reservation is still this kind and the flag is still on (line 130). Otherwise the seam is disabled while `changing` is true, and the function returns null (lines 131 to 143).

**Why the old result was wrong**: The approved plan says an element is enabled only while its kind flag is on, and that turning a kind off disables the elements this module enabled. HLD section 12 keeps both libraries in the application and uses the feature flag as the cutover, with stack first, then MPR, then 3D. The plan names that third kind `volume`. A kind flag that reads off while the returned seam still dispatches leaves that viewport on the new path.

**Evidence**: On the restored file, the one-element probe logged:

```text
PROBE_SINGLE {"nested":true,"nestedOn":false,"enabledAfter":false,"enabled":true,"seamNull":true,"still":false,"liveAfterOff":true,"via":false,"repeated":true,"after":false,"againNull":false,"againLive":true,"types":["mounted","unmounted"]}
```

`enabledAfter` is the flag immediately after `enableFlagged` returned. `via` and `after` are `dispatchOnElement` of `frame.presented` before the re-enable. `againLive` is the dispatch after the off listener was removed and stack was turned on again. `nestedOn` is `setKindEnabled("stack", true)` from the rollback unmount, and it was refused.

The two-element probe logged:

```text
PROBE_PAIR {"enabled":false,"firstLive":false,"secondLive":false,"firstNull":false,"secondNull":true,"thirdNull":true,"thirdLive":false,"types":["first-mounted","second-mounted","first-unmounted","second-unmounted"]}
```

Pass 1's pair log had `secondLive` true, `secondNull` false, and no second unmount. That ending is gone.

Forcing `kept` to `true` makes those two unit tests fail. The one-element test expected null and received a seam at `flag.test.ts:153`. The two-element test expected null and received a seam at `flag.test.ts:178`. The other 8 tests passed. Restoring the file put the hash back and the 10 tests passed.

**Scope**: The old behavior was only in the uncommitted file pass 1 reviewed. This tree rejects that ending for stack and for volume. No uncertainty about the observed rollback.

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

- The restored `flag.ts` hash is the hash given at the start of this pass. `cmp` against the pre-mutation blob matched. `seam.ts` and `alongside.ts` still have the required hashes. `events.ts` stayed `79b06026c188dc0f4196254fde1e11539806472b92d8178a29298a7c5b5f5978`. `migrate.ts` stayed `286fe2f00b8eb2343947f9654d64554efcb262aab5e803246bb406c48c8e254e`. Those four files were not edited.
- Flags still start off. MPR is refused while stack is off. Volume is refused while MPR is off. Stack off is refused while MPR is on, including when the call comes from `viewport.mounted` on a real element. The in-flight stack seam then stays and dispatches. The order-check deletion from pass 1 was not repeated. The check is still present at `flag.ts` lines 72 to 84, and AC1 and AC2 passed.
- A repeated `setKindEnabled("stack", true)` does not disable. An unknown kind returns false from `setKindEnabled` and null from `enableFlagged`. AC5 passed.
- A normal turn-off still unmounts a stored element and a later dispatch returns false. One kind's elements stay when another kind turns off. The volume rollback probe left stack and MPR dispatching.
- A throw from the unit fake's `dispatchEvent` does not store the element. The flag stays on. Replacing `dispatchEvent` with a normal method lets a later `enableFlagged` return a new seam. AC3's throwing test passed, including under the `kept = true` mutation, because that path returns a null seam before the `kept` branch can store it.
- While the off path or the rollback `disable` is in progress, a nested `setKindEnabled` that would change a flag returns false. The unit file locks the off path. The probe locked `setKindEnabled("stack", true)` during the rollback unmount.
- After `setKindEnabled("stack", false)` returns to the mounted listener, and before that listener returns, `dispatchOnElement` of `frame.presented` is still true (`liveAfterOff`). The seam is disabled after `enableElement` returns. After `enableFlagged` returns, the kind is off and the same dispatch is false. That gap is the rollback timing in lines 129 to 143. It is not the closed defect, which was a seam that still dispatched after the function returned.
- If the listener turns the kind off and then on again before returning, reservation loss still drops the in-flight element. The probe logged `seamNull` true and `cancelledLive` false while the flag ended on, and the element enabled from the rollback unmount dispatched.
- The module imports `enableElement` and the `ElementSeam` type from `./seam.js` only. It does not import another viewer. It does not compare frames. It does not call `addEventListener`. There is no per-element flag that skips the kind order.
- The index change exports the three functions and the `ViewportKind` type. `index.test.ts` lists those three value names and names `ViewportKind` as type-only. The probe saw one function identity for each value.
- No wasm-bindgen, no `unsafe`, and no pixel path was added. Tiers A, B, and C stay not applicable, as the plan states.

## Limitations and handoff

This is an implementation review of the uncommitted diff. It is not the feature gate and it is not ledger completion. The plan's feature profile, `bin/ocelli.sh gate --floor` plus the corpus gate, was not run. No comparison report was produced. Frame comparison belongs to F-109.

`docs/lld/migration.md` and `docs/lld/public-api.md` have no F-107 paragraph. Missing F-107 paragraphs in the LLD are completion work, not a defect in this diff.

`packages/core/dist` is gitignored and was not rebuilt. The published check used the source index, which is what `index.test.ts` loads. A real `EventTarget` listener that throws does not unroll the mount. The unit file covers the separate case where `dispatchEvent` itself throws.

The first probe run is not a product result. The corrected probe, the vitest files, and the `kept` mutation are the evidence above. After the probe was deleted, `git status --porcelain` showed modifications to `packages/core/src/index.ts` and `packages/core/src/index.test.ts`, and untracked `packages/core/src/flag.ts`, `packages/core/src/flag.test.ts`, and `.claude/reviews/F-107-flag-pass-1.md`. It did not show `seam.ts`, `alongside.ts`, `events.ts`, or `migrate.ts`.

dwv, Horos, and the medical viewer Grok were not opened.
