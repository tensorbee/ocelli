# F-095 review, pass 1

**Reviewed**: untracked product files on `sprint/s35`. HEAD is `32d7534cb906e855129b7efe985111f4c839406a`. Nothing from this story was staged. `git diff --name-only` for the two reviewed paths was empty. The reviewed sources are `packages/core/src/annotation-state.ts` (sha256 `6c7f9f15858248c1043f84fe95718ed22773b41ce81198183dd12ad0d7e8438c`) and `packages/core/src/annotation-state.test.ts` (sha256 `1cdf50fe3c324b7da7ee16fb5f45153c537c972ddc5a66b541c461f383a8934f`). Other worktree paths were dirty. This pass did not review them.
**Author / reviewer**: The author is the parent implementation agent. The reviewer is independent subagent `01a19ea9-1ce9-7a72-86aa-8dcaed5b0bce` and did not write the plan or the diff.
**Result**: 0 defects, 0 smells, 0 nitpicks

This pass found 0 defects and 0 smells. The mutation is recorded below.

## Acceptance challenged

The approved plan is the contract. The HLD does not define the undo stack. That absence is not a defect. No measurement was invented. No other viewer was used as the specification.

AC1. Two puts on UID `1.2.3` and one put on UID `1.2.4` must list two records and one record, in insertion order. The shipped expect is ids `a`, `b` and id `c`, and the listed record is not the stored record. The probe listed an interleaved `a`, `c` pair with numbers 1 and 3, and `b` alone on `1.2.4`. UID `9.9.9` and prefix `1.2` each returned an empty list. A write of the listed child number left the stored number at 1. The listed record, its tree, and its child were different objects.

AC2. Replacing id `a` must keep one record and the new number 4. The shipped expect is length 1 and stored number 4. The probe replaced middle id `b` with 4, kept order `a`, `b`, `c`, and left the other numbers at 1 and 3.

AC3. Undo after two puts must restore the one-record state. A second undo must restore empty. A third undo must return null. The shipped expect is ids `a`, then an empty list, then null. The probe saw number 1 on that first undo. The third null left the empty state in place. Three puts, three undos, and three redos restored numbers 1, 2, and 3. A further redo returned null.

AC4. Redo after one undo must restore the latest put. A new put after undo must make redo return null. The shipped expect is ids `a`, `b` and a null redo. The probe restored numbers 1 and 2. A refused put while redo was present left that redo in place. A successful put cleared it.

AC5. A blank id must return null. A tree that `contentTree` refuses must return null. The shipped expects are those nulls, a blank UID, an unchanged records array, and a stored tree that is not the caller tree. Changing the blank-id length from 0 to 2 made the empty id return a state. The expect received that state at `annotation-state.test.ts:111`. The other four tests stayed green. The probe saw `contentTree` return null for a root whose relationship is `CONTAINS`, and `putAnnotation` return null for that tree, for `NaN`, and for an empty UID. The records, undo, and redo array references stayed the same. A frozen tree copied. A write of the caller child left the stored number unchanged.

The module calls `contentTree` from `sr.ts`. It does not reimplement the relationship, value type, or number checks.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `npm test -- packages/core/src/annotation-state.test.ts` from the repo root | Restored blank-id length 0. sha256 `6c7f9f15858248c1043f84fe95718ed22773b41ce81198183dd12ad0d7e8438c` | executed now, before the mutation | Exit 0. 5 passed. |
| Same command after `id.length === 0` became `id.length === 2` | One constant in `packages/core/src/annotation-state.ts`. Mutated sha256 `f6594a8c367d5fe23d2f711b55bec4ab79337dc014b89c04e31c4c3b34f8974a`. No other test file was run. | executed now | Exit 1. 1 failed and 4 passed. AC5 received a state for an empty id and expected null at `annotation-state.test.ts:111`. |
| `cmp` of the saved original against `packages/core/src/annotation-state.ts`, then the same test | Bytes copied back from the pre-mutation file. The blank-id comparison uses length 0 again. sha256 again `6c7f9f15858248c1043f84fe95718ed22773b41ce81198183dd12ad0d7e8438c` | executed now | `cmp` exit 0. Test exit 0. 5 passed. The test file sha256 stayed `1cdf50fe3c324b7da7ee16fb5f45153c537c972ddc5a66b541c461f383a8934f`. |
| `npx tsc --pretty false -p /tmp/f095-review/tsconfig.json --noEmit` | Temp config extended `tsconfig.base.json` and included the two reviewed files plus `sr.ts`. | executed now, before the mutation | Exit 0. No diagnostics. |
| Node probe, before the mutation | tsc emit of the reviewed source at sha256 `6c7f9f15858248c1043f84fe95718ed22773b41ce81198183dd12ad0d7e8438c`. Acceptance rows, copy isolation, replace, undo and redo numbers, a refused put keeping redo, and a same id moving frame. | executed now, no repo file added | Exit 0. `FAILURES 0`. The checklist rows matched the plan. The probe was not repeated after the restore. |

Vitest 4.1.11. Node v24.13.0. No cargo, no wasm-pack, and no `bin/ocelli.sh`.

The restored comparison is:

```
  if (id.length === 0 || frameOfReference.length === 0) {
```

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

`createAnnotationState` returns empty records and two empty stacks. Two calls do not share those arrays. The first undo and the first redo return null.

`putAnnotation` appends a new id and replaces the same id in place. A replace of the middle id keeps `a`, `b`, `c`. Undo of that replace restores number 2. Redo restores number 4. The same id with a new UID replaces the one record and moves it to the new frame. That is the plan's same-id rule.

A successful put pushes one copy of the previous record list and sets redo to empty. Undo and redo each move one step. They copy the record lists they keep. The current tree and the latest undo tree are different objects. A write of the current number left the undo snapshot unchanged. Three history steps round-trip the numbers 1, 2, and 3.

An empty id, an empty UID, a root relationship `CONTAINS`, and a `NaN` number each return null. None of those calls replaces the records, undo, or redo arrays. A refused put does not drop redo. The new tree, the listed tree, and the history tree are not the caller tree. A write of the caller number left the stored number unchanged. A write of the listed number left the stored number unchanged. A frozen root, child list, child, and concept still copy.

The refusal goes through `contentTree`. The module has no local value-type list, relationship list, or finite-number check. Undo and redo do not compute a measurement.

`git diff --name-only` for `packages/core/src/annotation.ts`, `packages/core/src/sr.ts`, and `packages/core/src/index.ts` was empty. A search of `index.ts` for `annotation-state`, `createAnnotationState`, `putAnnotation`, and `undoAnnotation` found nothing.

The module does not read pixels. No `unsafe`. No `wasm-bindgen`. Tiers A, B, and C are not branched. The plan marks each tier n/a.

## Limitations and handoff

The mutation is not left in place. `cmp` against the saved original was exit 0, and the sha256 is `6c7f9f15858248c1043f84fe95718ed22773b41ce81198183dd12ad0d7e8438c`.

The plan's API section names the five functions. `index.ts` does not export them. The plan's write list also names the LLD paragraph. `docs/lld/` does not name these functions. The export and that paragraph are completion work after a clean pass. This pass does not count either gap.

An id or UID of one space, and an id of one tab, were accepted. The acceptance row and the fixture use the empty string. The plan does not say trim. This pass does not count whitespace.

`putAnnotation` with a null tree threw `TypeError: Cannot read properties of null (reading 'valueType')`. An undefined tree threw the same type of error for `undefined`. AC5 is a tree `contentTree` refuses, and the parameter type is `ContentItem`. This pass does not count those throws.

The shipped tests do not themselves lock undo numbers, an interleaved frame, history isolation, a refused put keeping redo, or a same id moving frame. The probe executed those cases on the pre-mutation emit. The restore matched that source hash. No feature profile was run. No browser and no pixel oracle were used. This story has no rendered parity view.

No patient data was read.
dwv, Horos, and the medical viewer Grok were not opened.
