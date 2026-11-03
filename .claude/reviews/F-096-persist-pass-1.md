# F-096 review, pass 1

**Reviewed**: uncommitted F-096 diff on `sprint/s36`. HEAD is `b1f4013b229f5f6b8e1e1e1fff2587fda43eead6`. Nothing is staged. The product module is `packages/core/src/measurement-persist.ts` (sha256 `2ab55d311dcb37ea3d64d988e8b6bca3e1f11c309489f511c1bbe4a235c40a25` before the mutation and after the restore). The test is `packages/core/src/measurement-persist.test.ts` (sha256 `67e356d4479bcd49c3ebaa62b3e82d361e17f91bb19309da8613a8e3570fadd7`). `packages/core/src/index.ts` exports the two functions. `packages/core/src/index.test.ts` lists those two names and does not list `MEASUREMENT_ITEM_LIMIT`.
**Author / reviewer**: Independent reviewer `01a19ef6-21ba-74a1-a974-984187e3349b`. This pass did not write the plan or the diff.
**Result**: 1 defect, 0 smells, 0 nitpicks

This pass found 1 defect and 0 smells. The limit mutation is recorded below.

## Acceptance challenged

The approved plan is the contract. The HLD does not give the document layout or the count inside "a few thousand". Those two gaps are the plan's local decision. They are not defects. No other viewer was used as the specification.

AC1. Two valid roots load in caller order. A nested text node is not the caller node. The shipped expect checks length 2, the texts `0` and `later`, and the identity of the loaded child and the loaded root. The probe saw that order. The loaded root, the child, the child list, and the concept were different objects. A write of the caller text left the reloaded text at `0`. A write of the loaded text left the next load at `0`.

AC2. One bad node returns null. The shipped expect uses a root whose relationship is `CONTAINS`, and the earlier root's text stays `kept`. That runtime expect passed. The probe also refused a bad child and a bad grandchild, on persist and on load, and the caller text stayed `ok`. The expect that reads the caller text does not typecheck. That is D1.

AC3. 3999 nodes persist. 4000 nodes return null. The shipped expect uses one container and its direct children. 3998 children are 3999 nodes and persist. 3999 children are 4000 nodes and return null. The probe counted 3999 nodes on that loaded tree. 3999 separate roots persisted in order. 4000 roots returned null. A split of 1999 and 2000 nodes persisted. A split of 2000 and 2000 returned null. A crafted document of 4000 nodes loaded as null. Changing the limit from 4000 to 4001 made the 4000-node call return a document. The expect received that document.

AC4. A wrong format, a wrong version, and a non-object return null. The shipped expects cover another format, version 2, an array, null, and a truncated object. The probe also refused a version string, a bare number, a bare string, a missing `items` field, an `items` object, and a different format case.

AC5. An empty list loads as a new empty array. The shipped expect uses `toEqual([])` and `not.toBe` on two loads. The document was `{"format":"ocelli-measurement","version":1,"items":[]}`. The two arrays were different objects.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| noEmit `tsc` of `packages/core/src` under `tsconfig.base.json` | Pre-mutation sources. Module sha256 `2ab55d311dcb37ea3d64d988e8b6bca3e1f11c309489f511c1bbe4a235c40a25` | executed now, before the mutation | Exit 2. The only diagnostic is TS2339 at `measurement-persist.test.ts:75`. |
| Node probe of the pre-mutation emit | Same module bytes. Plan rows, copy identity, bad child, boundaries, `NUM`, `SCOORD`, frozen caller | executed now, before the mutation | Plan rows matched. One added chain check did not. See limitations. |
| `npm test -- packages/core/src/measurement-persist.test.ts` | One edit. `MEASUREMENT_ITEM_LIMIT` was 4001. No other test file was run. | executed now | Exit 1. 1 failed and 4 passed. The 4000-node expect received a document. |
| `cmp` of the saved original against `packages/core/src/measurement-persist.ts` | Bytes copied back from the pre-mutation file. The limit is 4000 again. sha256 again `2ab55d311dcb37ea3d64d988e8b6bca3e1f11c309489f511c1bbe4a235c40a25` | executed now | `cmp` exit 0. The test file was not edited. |

Vitest 4.1.11. Node v24.13.0. No cargo, no wasm-pack, and no `bin/ocelli.sh`. This was the only npm test.

The failing expect is:

```ts
expect(persistMeasurements([container(3999)])).toBeNull();
```

Vitest reported `AssertionError: expected '{"format":"ocelli-measurement","versi…' to be null` at `packages/core/src/measurement-persist.test.ts:83`. The received value was a string whose start was `{"format":"ocelli-measurement","version":1,"items":[{"relationship":null,"valueType":"CONTAINER"`. The 3999-node expect in that same test had already passed.

The restored comparison is:

```ts
export const MEASUREMENT_ITEM_LIMIT = 4000;
```

The type diagnostic is:

```text
packages/core/src/measurement-persist.test.ts(75,21): error TS2339: Property 'text' does not exist on type 'ContentItem'.
  Property 'text' does not exist on type 'ItemBase & { valueType: "CONTAINER"; }'.
```

## Defects

### D1. The bad-node test does not typecheck

**Where**: `packages/core/src/measurement-persist.test.ts:75`
**What**: `textItem` is declared to return `ContentItem`. `previous` is that return value. The expect reads `previous.text`. `text` exists on the `TEXT` member and not on `CONTAINER`.
**Why it is wrong**: `packages/core/tsconfig.json` includes `src/**/*`, and the root project references that package. `npm run typecheck` is `tsc --build`. This file is the AC2 evidence. The compiler rejects it. Vitest strips types, so the same expect still ran and passed while the limit was mutated. A green runtime file that fails the package compile is not an accepted test.
**Evidence**: The noEmit compile of `packages/core/src` under `tsconfig.base.json` exited 2 before the mutation. The only diagnostic is the fenced TS2339 above. The product module was in that compile and produced no diagnostic. `tsc --build` was not run.
**Scope and regression**: `persistMeasurements` and `loadMeasurements` are not the failure. The smallest fix is to narrow `previous` to `TEXT` before reading `text`. The runtime check that the caller text stays `kept` is the right AC2 assertion.

## Smells

None.

## Nitpicks

None.

## Verified clean

`persistMeasurements` runs `contentTree` on every root and then `JSON.stringify`. `loadMeasurements` checks `format` `ocelli-measurement` and version 1, then runs `contentTree` again. One null copy returns null for the whole call. The caller array is not the stored `items` array. An extra field on a caller node was absent from the document. A frozen root still copied. A second call with the same items returned the same document text, and the two loads were different nodes.

The count is every copied node, roots and nested children. 3999 wide nodes persist. 4000 wide nodes return null from persist. A crafted 4000-node document returns null from load. The same threshold holds for 4000 roots and for two roots whose counts add to 4000. There is no second binary layout.

`NUM` 12.5 with unit `mm` round-tripped. A two-point `POLYLINE` round-tripped, and the loaded point row was not the caller row. `NaN` returned null. Empty `items` loaded as a new array.

`index.ts` exports `loadMeasurements` and `persistMeasurements` and does not export `MEASUREMENT_ITEM_LIMIT`. The module does export that constant. The published-name list in `index.test.ts` names only the two functions. There is no Rust twin. `annotation.ts` is not in the diff. The module does not import dcmjs and does not compute a measurement.

The public-api paragraph, the README row, and the changelog line name the JSON document, the null refusal, the count of 4000, and the absence of a bulk layout. No `unsafe`. No `wasm-bindgen`. Tiers A, B, and C are not branched. The plan marks each tier n/a.

## Limitations and handoff

The mutation is not left in place. `cmp` against the saved original was exit 0, and the sha256 is `2ab55d311dcb37ea3d64d988e8b6bca3e1f11c309489f511c1bbe4a235c40a25`. HEAD is still `b1f4013b229f5f6b8e1e1e1fff2587fda43eead6`.

A single chain a few thousand nodes deep is not the acceptance shape. The plan says to copy through `contentTree` and then call `JSON.stringify`. On a cold call, `contentTree` raised `RangeError` for a chain of 2500, and persist returned null because that error is caught around the copy. After shallower copies, a chain of 3999 reached `JSON.stringify`, which raised `RangeError` that persist does not catch. A chain of 4000 returned null from the count before stringify. The approved wide tree of 3999 nodes persisted. This pass does not count the chain.

The shipped tests do not themselves lock a bad child on load, a root count of 3999, a split across two roots, or a `NUM` round trip. The probe executed those cases on the pre-mutation emit. The restore matched that source hash. No feature profile was run. No browser and no pixel oracle were used. This story has no rendered parity view.

`docs/sprints/BACKLOG.md` and `docs/sprints/CURRENT_SPRINT.md` already say F-096 is done. That status word is not a second finding. D1 is the reason this pass does not call the story clear.

No patient data was read.
dwv, Horos, and the medical viewer Grok were not opened.
