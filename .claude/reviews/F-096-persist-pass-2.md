# F-096 review, pass 2

**Reviewed**: uncommitted F-096 diff on `sprint/s36`. HEAD is `b1f4013b229f5f6b8e1e1e1fff2587fda43eead6`. Nothing is staged. The product module is `packages/core/src/measurement-persist.ts` (sha256 `2ab55d311dcb37ea3d64d988e8b6bca3e1f11c309489f511c1bbe4a235c40a25` before the mutation and after the restore). The test is `packages/core/src/measurement-persist.test.ts` (sha256 `72e5d51712804e3d9e1969981050534586a6e7d62ea7b8a0f774b826d682d94b`). `packages/core/src/index.ts` exports the two functions. `packages/core/src/index.test.ts` lists those two names and does not list `MEASUREMENT_ITEM_LIMIT`.
**Author / reviewer**: Independent reviewer `01a19f07-516a-7612-a840-4f009f8c1328`. This pass did not write the plan, the diff, or pass 1.
**Result**: 0 defects, 0 smells, 0 nitpicks

This pass found 0 defects and 0 smells. Pass 1's D1 is closed. The limit mutation is recorded below.

## Acceptance challenged

The approved plan is the contract. The HLD does not give the document layout or the count inside "a few thousand". Those two gaps are the plan's local decision. They are not defects. No other viewer was used as the specification.

AC1. Two valid roots load in caller order. A nested text node is not the caller node. The shipped expect checks length 2, the texts `0` and `later`, and the identity of the loaded child and the loaded root. That test passed during the mutated run. The probe saw the same order. The loaded root, the child, the child list, and the concept were different objects. A write of the caller text left the reloaded text at `0`. A write of the loaded text left the next load at `0`. An extra field on a caller node was absent from the document. A frozen root and a frozen caller array still copied. A second call with equal items returned the same document text, and the two loads were different nodes.

AC2. One bad node returns null. The shipped expect uses a root whose relationship is `CONTAINS`, and the earlier root's text stays `kept`. That expect passed at runtime. Pass 1's TS2339 was at the read of `previous.text`. The test now throws unless `previous.valueType` is `"TEXT"`, and only then reads `previous.text`. The noEmit compile of `packages/core` exited 0 with no diagnostic. D1 is closed. The probe also refused a bad child, a bad grandchild, and a `NUM` of `NaN`, on persist. A crafted document with a bad child, and a text node with no `children` array, both loaded as null. A valid call after a refusal still returned the kept text.

AC3. 3999 nodes persist. 4000 nodes return null. The shipped expect uses one container and its direct children. 3998 children are 3999 nodes and persist. 3999 children are 4000 nodes and return null. The probe counted 3999 nodes on that loaded tree. 3999 separate roots loaded with length 3999, first text `0`, and last text `3998`. 4000 roots returned null. A split of 1999 and 2000 nodes persisted. A split of 2000 and 2000 returned null. A grandchild tree of 3999 nodes persisted, and one of 4000 returned null. A crafted document of 4000 nodes loaded as null. A crafted document of 3999 nodes loaded with count 3999. Changing the limit from 4000 to 4001 made the 4000-node call return a document. The expect received that document.

AC4. A wrong format, a wrong version, and a non-object return null. The shipped expects cover another format, version 2, an array, null, and a truncated object. The probe also refused a different format case, a version string, a missing `items` field, an `items` object, `items` null, `true`, `false`, a bare number, a bare string, and a trailing value. An extra field on an otherwise empty document loaded as an empty array. The plan does not reject an extra header field.

AC5. An empty list loads as a new empty array. The shipped expect uses `toEqual([])` and `not.toBe` on two loads. The document was `{"format":"ocelli-measurement","version":1,"items":[]}`. The two arrays were different objects.

A container whose children came from `provenanceItems` loaded the texts `Length`, `0.1.0`, `v1`, and `reader` in that order. A text tree stored with `putAnnotation` persisted from the record returned by `annotationsForFrame`. The empty state was not written. `NUM` 12.5 with unit `mm` round-tripped. `CODE`, `UIDREF`, `IMAGE`, `SCOORD`, and `SCOORD3D` round-tripped. The loaded `SCOORD` point row was not the caller row.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `npx tsc -p packages/core --noEmit --pretty false` | Pre-mutation sources. Module sha256 `2ab55d311dcb37ea3d64d988e8b6bca3e1f11c309489f511c1bbe4a235c40a25` | executed now, before the mutation | Exit 0. No diagnostic. |
| Node probe of the acceptance rows | Type-stripped emit of that same module. Wide count, copy identity, bad child, value types, provenance, annotation state | executed now, before the mutation | Exit 0. 0 failed checks. |
| Node cold chain of 2500 | Same module bytes. First call in the process | executed now, before the mutation | Exit 0. `contentTree` threw RangeError. persist returned null. |
| Node chain after repeated copies | File emit of the same module, warmed by successful copies | executed now, before the mutation | Exit 0. A sampled chain of 3000 persisted. Chains of 3500 and 3999 threw from persist. A chain of 4000 returned null. |
| `npm test -- packages/core/src/measurement-persist.test.ts` | One edit. `MEASUREMENT_ITEM_LIMIT` was 4001. No other test file was run. | executed now | Exit 1. 1 failed and 4 passed. The 4000-node expect received a document. |
| `cmp` of the saved original against `packages/core/src/measurement-persist.ts` | Bytes copied back from the pre-mutation file. The limit is 4000 again. sha256 again `2ab55d311dcb37ea3d64d988e8b6bca3e1f11c309489f511c1bbe4a235c40a25` | executed now | `cmp` exit 0. The test sha256 is unchanged. |

Vitest 4.1.11. Node v24.13.0. No cargo, no wasm-pack, and no `bin/ocelli.sh`. `pgrep` found no `cargo`, `rustc`, or `wasm-pack` before `tsc` and before the test. This was the only npm test. `tsc` was not given `--incremental false`.

The failing expect is:

```ts
expect(persistMeasurements([container(3999)])).toBeNull();
```

Vitest reported this assertion at `packages/core/src/measurement-persist.test.ts:86`:

```text
AssertionError: expected '{"format":"ocelli-measurement","versi…' to be null
```

The received value was a string. Its start was `{"format":"ocelli-measurement","version":1,"items":[{"relationship":null,"valueType":"CONTAINER"`. The expects above that line in the same test had already passed.

The restored comparison is:

```ts
export const MEASUREMENT_ITEM_LIMIT = 4000;
```

The closed narrow is:

```ts
if (previous.valueType !== "TEXT") {
  throw new Error("expected a text root");
}
expect(previous.text).toBe("kept");
```

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

Pass 1's D1 is closed. `previous` is a `ContentItem`. `text` exists on the `TEXT` member and not on `CONTAINER`. The new branch throws for every other member, so the read is on the `TEXT` member. The package compile includes this test and exited 0. The bad-node test also passed under Vitest.

`persistMeasurements` runs `contentTree` on every root and then `JSON.stringify`. `loadMeasurements` checks format `ocelli-measurement` and version 1, then runs `contentTree` again. One null copy returns null for the whole call. The caller array is not the stored `items` array. The `as ContentItem` cast in `copyRoot` is the bridge into `contentTree`. A throw from a malformed node is caught there and becomes null. Missing `children`, a bad child, and a non-object item were all null.

The count is every copied node, roots and nested children. The comparison is `count >= 4000`, so 3999 persists and 4000 returns null. That holds for a wide tree, for 4000 roots, for two roots whose counts add to 4000, and for a grandchild tree. There is no second binary layout.

`index.ts` exports `loadMeasurements` and `persistMeasurements` and does not export `MEASUREMENT_ITEM_LIMIT`. The module does export that constant. The published-name list in `index.test.ts` names only the two functions. There is no Rust twin under `crates/` or `packages/`. `annotation.ts` is not in the diff. The module does not import dcmjs and does not compute a measurement.

The public-api paragraph, the README row, and the changelog line name the JSON document, the null refusal, the count of 4000, and the absence of a bulk layout. No `unsafe`. No `wasm-bindgen`. Tiers A, B, and C are not branched. The plan marks each tier n/a.

## Limitations and handoff

The mutation is not left in place. `cmp` against the saved original was exit 0, and the sha256 is `2ab55d311dcb37ea3d64d988e8b6bca3e1f11c309489f511c1bbe4a235c40a25`. HEAD is still `b1f4013b229f5f6b8e1e1e1fff2587fda43eead6`.

A single chain a few thousand nodes deep is not the acceptance shape. The plan says to copy through `contentTree` and then call `JSON.stringify`. On a cold call, `contentTree` raised RangeError for a chain of 2500, and persist returned null because that error is caught around the copy. After repeated successful copies, persist returned a document for sampled chains of 100, 500, 1000, 1500, 2000, 2500, and 3000. A chain of 3500 and a chain of 3999 then reached `JSON.stringify`, which raised RangeError that persist does not catch. A chain of 4000 returned null from the count before stringify. A direct stringify of that 4000 chain still threw. The approved wide tree of 3999 nodes persisted. A chain of 100 also persisted. The throw is the engine stack on a shape the plan does not use as acceptance. This pass does not count the chain.

The same source did not throw on every emit. In a second type-stripped copy that had not been warmed as far, `contentTree` still threw for a chain of 3999, and the catch turned that into null. The product rule is the count of 4000. The stack budget moved with optimization. It is not a second threshold.

The shipped tests do not themselves lock a bad child on load, a grandchild count, a root count of 3999, a split across two roots, or a `NUM` round trip. The probe executed those cases on the pre-mutation emit. The restore matched that source hash. `index.test.ts` was not run. The export lines were read. No feature profile was run. No browser and no pixel oracle were used. This story has no rendered parity view.

`docs/sprints/BACKLOG.md` and `docs/sprints/CURRENT_SPRINT.md` already say F-096 is done. That status word is not a finding.

No patient data was read.
dwv, Horos, and the medical viewer Grok were not opened.
