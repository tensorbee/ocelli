# F-107 review, pass 1

**Reviewed**: Uncommitted F-107 on `sprint/s39`. HEAD is `c446a84bd74a59f4416fb92f1f1eec799bba1844`. The product paths are untracked `packages/core/src/flag.ts` (sha256 `0b749c4874c7d84313c7389340aa186637293dcc66436617fdab3618e921520a`) and `packages/core/src/flag.test.ts`, plus the export lines in `packages/core/src/index.ts` and `packages/core/src/index.test.ts`. This review file is not part of that diff.

**Author / reviewer**: Author is the parent session. Reviewer is independent subagent `01a19feb-427f-7b12-a04d-6a862e2b0d8b`. This reviewer did not write the code or the design.

**Result**: 1 defect, 0 smells, 0 nitpicks

## Acceptance challenged

AC1 to AC5 were run from `flag.test.ts`. All 7 tests passed on the restored file. Dropping the previous-kind check made AC1 and AC2 fail, so those assertions do see the order gate.

The probe imported `flag.ts` and `index.ts` from source and used real `EventTarget` listeners. It is not part of the suite. It was deleted after the run.

The cross-kind case is one the unit file does not lock with a real listener. Stack and MPR elements were enabled. Volume was refused while MPR was off. Volume was then allowed. Disabling stack was refused and the stack seam still dispatched. Disabling volume left the stack seam and the MPR seam dispatching. The volume seam did not.

Other passing probe cases reject nearby wrong endings. A null seam from a `dispatchEvent` that throws is not stored, and a later enable of that same object works. A plain object and `null` return null. An element already enabled through `enableElement` is not stored, and turning stack off does not disable that outside seam. The same element cannot be enabled as MPR after stack. Turning MPR off leaves that stack seam. During volume unmount, a nested `setKindEnabled("mpr", false)` returns false and MPR stays on. A nested `setKindEnabled("volume", true)` returns false. Two stack elements are both disabled when stack turns off.

The mounted reentrancy cases do not pass. They are D1. `flag.test.ts` never turns the kind off from `viewport.mounted`, so AC3 and AC4 stay green on that path.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `npx vitest run` on `flag.test.ts`, `index.test.ts`, `events.test.ts`, `seam.test.ts`, and `alongside.test.ts` | `flag.ts` sha256 `0b749c4874c7d84313c7389340aa186637293dcc66436617fdab3618e921520a`. Node v24.13.0. Vitest 4.1.11. | Executed now, before the mutation | Exit 0. 5 files, 30 tests passed. |
| `npx tsc -p packages/core --noEmit --pretty false` | `packages/core` tsconfig, tests included | Executed now | Exit 0. No diagnostics. `--incremental false` was not passed. |
| `npx eslint` on `flag.ts`, `flag.test.ts`, `index.ts`, and `index.test.ts` | Same tree | Executed now | Exit 0. No diagnostics. |
| `npx vitest run packages/core/src/flag.test.ts` after deleting the previous-kind check | Mutated sha256 `b6a2ea9b38fbebfa1ef0ea5decafd699eed6dc84f5f061875d77c3f5df4b6c1e` | Executed now | Exit 1. AC1 and AC2 failed. MPR enable returned true while stack was off. Volume enable returned true while MPR was off. The other 5 tests passed. |
| `cmp /tmp/f107-flag.ts.blob packages/core/src/flag.ts` and `shasum -a 256` | Restored source | Executed now | `cmp` exit 0. sha256 returned to `0b749c4874c7d84313c7389340aa186637293dcc66436617fdab3618e921520a`. |
| `npx vitest run packages/core/src/flag.test.ts` | Restored hash above | Executed now, after the restore | Exit 0. 7 tests passed. |
| `npx vitest run --config /tmp/f107-vitest.config.ts` | Probe `/tmp/f107-flag-probe.test.ts`, 7 cases, source imports, real `EventTarget` | Executed now on the restored tree | Exit 1. 5 passed and 2 failed. The failures are D1. The probe and the config were deleted afterward. |
| `rg addEventListener\\(` on `flag.ts` | Restored `flag.ts` | Executed now | Exit 1. No matches. `events.test.ts:233` still expects `["pointer.ts"]`, and that file passed in the first vitest run. |

No cargo, rustc, or wasm-pack command was started. Exit codes were taken from `$?` on the command itself.

## Defects

### D1, an in-flight enable stays mounted after its kind turns off

**Severity**: defect

**Where**: `packages/core/src/flag.ts:122`

**What**: `enableFlagged` calls `enableElement` and only then stores the element. `viewport.mounted` runs inside that call. The kind set does not contain the element yet. A mounted listener can turn that kind off. The off path disables only the elements already in the set, sets the flag false, and returns true. `enableFlagged` then stores the seam and returns it. The kind is off and the seam still dispatches `frame.presented`. A later `setKindEnabled(kind, false)` hits the same-value return at line 65 and does not call `disable`.

```ts
  const seam = enableElement(element);
  if (seam === null) {
    return null;
  }
  stored.set(element, { kind, seam });
  heldFor(kind).add(element);
  return seam;
```

On one real `EventTarget`, the mounted listener called `setKindEnabled("stack", false)`. That call returned true. `kindEnabled("stack")` was false. The returned seam still dispatched. The listener list was only `mounted`. Calling off again returned true and the seam still dispatched.

On two stack elements, the second mount turned stack off. The first element was unmounted. The second seam was not null and still dispatched. There was no second `viewport.unmounted`.

**Why it is wrong**: The approved plan says an element is enabled only while its kind flag is on, and that turning a kind off disables the elements this module enabled. The problem statement says a flag that turns off without disabling the elements it enabled does not follow the strangler order. HLD section 12 keeps both libraries in the application and uses the feature flag as the cutover, with stack first, then MPR, then 3D. A kind flag that reads off while the element keeps dispatching leaves that viewport on the new path.

The same-value rule is the specified answer for a flag that is already off. Here it preserves an element that was stored after the off transition had already finished.

**Evidence**: The probe printed these lines before the two failing assertions. Vitest exit was 1.

```text
PROBE_SINGLE {"nested":true,"enabled":false,"still":true,"repeated":true,"after":true,"types":["mounted"]}
PROBE_PAIR {"enabled":false,"firstLive":false,"secondLive":true,"firstNull":false,"secondNull":false,"types":["first-mounted","second-mounted","first-unmounted"]}
```

`still` and `after` are `seam.dispatch` of `frame.presented`. `firstLive` and `secondLive` are the same check. The unit file does not cover this listener, so its green run does not contradict the probe.

**Scope and regression**: This is only in the uncommitted `flag.ts`. HEAD `c446a84` does not contain the file. The probe saw `core.enableFlagged` as the same function as the module export, so the package index has the same behavior. No later kind has to be on. The one-element trigger is enough. The two-element trigger shows that an earlier element of the same kind is disabled while the in-flight one is kept.

Smallest regression: one real `EventTarget` whose `viewport.mounted` listener calls `setKindEnabled("stack", false)`. After `enableFlagged` returns, the kind is off and `frame.presented` does not dispatch. A second off call still leaves it not dispatching. Adjacent case: two stack elements, the second mount turns stack off, and both seams do not dispatch.

Reproduced on this tree. No uncertainty about the observation.

## Smells

None.

## Nitpicks

None.

## Verified clean

- The restored `flag.ts` hash is the hash given at the start of this pass. `cmp` against the pre-mutation blob matched. `seam.ts` is still `585c06cdc220e17f3073984acaa0dec5877cb7be372c29d963d973004582c779`. `alongside.ts` is still `fcd56226cadfa29f03206dbdf2a7b169466bb899d0161963baef3da89b1ed836`. `events.ts` and `migrate.ts` were not edited.
- `setKindEnabled`, `kindEnabled`, and `enableFlagged` have the planned signatures. `ViewportKind` is a type-only export. The three value names are on the package object. The probe saw the same three functions as the direct module exports. `index.test.ts` passed the runtime key set.
- Flags start off. The order gate refuses MPR while stack is off and volume while MPR is off. It refuses stack off while MPR is on. The mutation that deletes the previous-kind check goes red on AC1 and AC2.
- Setting a flag to its current value returns true. AC5 covers a repeated true. The probe covered a repeated false after a real disable, and the element was not mounted again.
- An unknown kind returns false from `setKindEnabled` and null from `enableFlagged`. AC5 locks that. A refused change does not clear a live stack seam.
- `enableFlagged` does not call through to a stored element. A second enable of the same element returns null and the first seam stays, including a second kind. If `enableElement` returns null, nothing is stored. The probe used a throwing `dispatchEvent`, a plain object, `null`, and an element already enabled outside this module. Turning the kind off did not disable that outside seam.
- A normal turn-off deletes the stored elements, sets the flag false, then calls `disable`. One kind's elements stay when another kind turns off. The cross-kind probe left stack and MPR dispatching after volume turned off, and the volume seam stopped. A throw from the unit fake's `dispatchEvent` does not put the element back. AC4 locks that. On a real `EventTarget`, a throwing unmounted listener is reported by Node as an uncaught exception because `dispatchEvent` does not throw. The probe case still disabled both stack seams and refused the nested MPR and volume changes. That platform report is not treated as a second defect.
- While a turn-off is in progress, a nested `setKindEnabled` that would change a flag returns false. AC4 locks turning the same kind back on. The probe locked turning MPR off and volume back on during volume's unmount. The flag being turned off stayed off.
- The module imports `enableElement` and the `ElementSeam` type from `./seam.js` only. It does not import another viewer. It does not compare frames. It does not call `addEventListener`. There is no per-element flag that skips the kind order.
- No wasm-bindgen, no `unsafe`, and no pixel path was added. Tiers A, B, and C stay not applicable, as the plan states. `seam.ts`, `alongside.ts`, `events.ts`, and `migrate.ts` were not edited.

## Limitations and handoff

This is an implementation review of the uncommitted diff. It is not feature-gate completion and it is not ledger completion. The plan's feature profile, `bin/ocelli.sh gate --floor` plus the corpus gate, was not run. No comparison report was produced. Frame comparison belongs to F-109.

`docs/lld/migration.md` and `docs/lld/public-api.md` have no F-107 paragraph. Workflow assigns those LLD edits to completion, so their absence is not a defect in this diff.

The unit file's off-during-unmount case uses a fake whose `dispatchEvent` throws. A real `EventTarget` does not throw from `dispatchEvent` when a listener throws. The probe recorded that difference and the flag state still matched the plan for that unmount. The open defect is the mount path, which neither the fake nor the real listener in `flag.test.ts` exercises.

`packages/core/dist` is gitignored and was not rebuilt. The published check used the source index, which is what `index.test.ts` loads.

D1 needs a new test and a change in `flag.ts`. A later independent pass should replay the one-element mount rollback and the two-element case, then rerun the flag tests. The order-check mutation should stay red if the previous-kind check is removed again.

After the probe was deleted, `git status --porcelain` showed modifications to `packages/core/src/index.ts` and `packages/core/src/index.test.ts`, and untracked `packages/core/src/flag.ts`, `packages/core/src/flag.test.ts`, and this review file. It did not show `seam.ts`, `alongside.ts`, `events.ts`, or `migrate.ts`.

dwv, Horos, and the medical viewer Grok were not opened.
