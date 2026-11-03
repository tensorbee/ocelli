# F-094 review, pass 2

**Reviewed**: untracked product files on `sprint/s35`. HEAD is `79eb0aeda6c177b6318119af6eabd750119728c9` (`79eb0ae`). Nothing was staged. `git diff --name-only` was empty. The reviewed sources are `packages/core/src/sr.ts` (sha256 `0557fb71fc8795422fcfb2002c7eb93ffcc97d3a14e38493a7cac9283e2f1da8`) and `packages/core/src/sr.test.ts` (sha256 `b82b3ba1cddb2394bd3d709383c7e9360f9390b9f360d9f6cd98f270d6c5898c`). `packages/core/src/annotation.ts` was not in that name list.
**Author / reviewer**: The author is the parent implementation agent. The reviewer is independent subagent `01a19e95-a66a-76b2-9be6-2c785bc3d7ad` and did not write the plan or the diff.
**Result**: 0 defects, 0 smells, 0 nitpicks

## Acceptance challenged

The approved plan is the contract. The HLD does not define the node fields. That absence is not a defect. No other viewer was used as the specification.

Pass 1 D1 and S1 were replayed on these bytes. Both remediations hold. No new defect or smell was found.

AC1. A container root with one `CONTAINS` `NUM` child of 12.5 and unit `mm` must copy both nodes. The caller child must be a different object. The shipped expect is deep equality with that child, plus identity checks on the root and the child. The probe received 12.5 and `mm`. The root, the child, the children array, and both concept objects were different objects. One shared caller concept on the root and the child became two copy concepts. A write to the copy number, the copy scheme, and the copy child list left the caller at 12.5, `SCT`, and one child. The caller JSON was unchanged. A frozen root and child still copied.

AC2. A `POINT` `SCOORD` of `(1, 2)` must stay those two numbers. The shipped expect is deep equality with that root. Changing the SCOORD width constant from 2 to 3 made the call return null, and the expect received null. A probe write of the caller point from 1 to 9 left the copy at 1. A write of the copy from 2 to 8 left the caller at 2. The copy, the points array, and the point row were different objects.

AC3. A `CIRCLE` with one point returns null. `NaN` returns null. An empty concept value returns null. The shipped expects are those three nulls. Accepting a one-point circle would fail the first. Copying `NaN` would fail the second. Copying an empty value would fail the third. The same module also returned null for `Infinity`, for `-Infinity`, for a string number, for an empty unit, for an empty scheme, and for an empty meaning. Zero and `-1.5` were copied. A unit outside any UCUM table was copied. A circle of two points was copied. A circle of three points returned null.

AC4. Provenance with tool `Length`, engine `0.1.0`, algorithm null, and operator `reader` returns three text children. The algorithm child is absent. An empty tool returns null. The shipped expects lock the length, the three texts, and the empty tool. Inserting an algorithm child would fail the length and the text list. Returning a list for an empty tool would fail the null expect. The probe found `HAS_OBS_CONTEXT` on each child and concepts `tool`, `engine`, and `operator` in that order. A separate probe with algorithm `v1` returned `Angle`, `0.2.0`, `v1`, and `other` in that order, with concept values `tool`, `engine`, `algorithm`, and `operator`. Empty engine, empty operator, and an empty algorithm string each returned null.

The shipped tests do not put provenance items through `contentTree`. A container whose children were the three provenance items copied those texts and relationships. The copy concepts were not the provenance concepts. Writing the copy scheme to `MUTATED` left the next `provenanceItems` call at `OCELLI`. One provenance item passed as a root returned null.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `npm test -- packages/core/src/sr.test.ts` from the repo root | Restored SCOORD width 2. sha256 `0557fb71fc8795422fcfb2002c7eb93ffcc97d3a14e38493a7cac9283e2f1da8` | executed now, before the mutation | Exit 0. 6 passed. |
| Same command after `points(item.points, 2, item.graphic)` became width 3 | One constant in `packages/core/src/sr.ts`. Mutated sha256 `c93ff3136b7a2077f3717ca2609397cf426137c0f02e5b155adaaf0a07c69438`. No other test file was run. | executed now | Exit 1. 1 failed and 5 passed. AC2 received null and expected the `(1, 2)` point at `sr.test.ts:43`. |
| `cmp` of the saved original against `packages/core/src/sr.ts`, then the same test | Bytes copied back from the pre-mutation file. The SCOORD call uses width 2 again. sha256 again `0557fb71fc8795422fcfb2002c7eb93ffcc97d3a14e38493a7cac9283e2f1da8` | executed now | `cmp` exit 0. Test exit 0. 6 passed. The test file sha256 stayed `b82b3ba1cddb2394bd3d709383c7e9360f9390b9f360d9f6cd98f270d6c5898c`. |
| Node probe, before the mutation | Same bytes as the restored file, sha256 `0557fb71fc8795422fcfb2002c7eb93ffcc97d3a14e38493a7cac9283e2f1da8`. Acceptance rows, D1 foreign types, S1 concept identity, provenance wrapped in a container, relationships, widths, and the eight value types. | executed now, no repo file added | Exit 0. `FAILURES 0`. The checklist rows matched the plan. D1 returned null and did not throw. S1 did not share a concept object. The probe was not repeated after the restore. |

Vitest 4.1.11. Node v24.13.0. No cargo, no wasm-pack, and no `bin/ocelli.sh`.

The restored comparison is:

```ts
const copied = points(item.points, 2, item.graphic);
```

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

Pass 1 D1 is closed. `copyItem` checks the value type against the eight names before it reads a payload. A container with one `CONTAINS` child of value type `DATE`, graphic `POINT`, one 3-wide point, and frame of reference `1.2.3` returned null. The caller JSON was unchanged. A bare `DATE` root returned null. A `DATE` root with a concept and no points or children returned null and did not throw. A `DATE` root with a null concept returned null and did not throw. `DATETIME`, `PNAME`, `COMPOSITE`, `WAVEFORM`, `TCOORD`, `SCOORD2D`, the lowercase string `container`, and an empty value type each returned null. A `DATE` grandchild returned null for the whole tree. The later `SCOORD3D` arm is not the fallback for those names.

Pass 1 S1 is closed. Each provenance call builds a new concept from scheme, value, and meaning. Two calls did not share the tool concept. Writing scheme, value, and meaning on the first left the next call at scheme `OCELLI`, value `tool`, and meaning `Tool`. The same split held for engine, operator, and a present algorithm. Four concepts in one call were four objects. A push onto the first children array did not appear on the next call.

`contentTree` returns null when the root relationship is not null. A missing root relationship returns null. A child with a null relationship returns null. Each of `CONTAINS`, `HAS_OBS_CONTEXT`, `HAS_PROPERTIES`, `INFERRED_FROM`, and `SELECTED_FROM` is kept on a child. A bad relationship string returns null. A lowercase `contains` returns null. One empty unit on a grandchild returns null for the whole tree. A bad sibling returns null and does not change the good sibling. The caller object is not written. A frozen root and child still produce a copy.

`NUM` returns null for `NaN`, `Infinity`, `-Infinity`, a string number, and an empty unit. Zero and a negative number are copied. The unit is not checked against a UCUM table.

An `SCOORD` point must be exactly two finite numbers. A 3-wide point returns null. An `SCOORD3D` point must be exactly three finite numbers, and an empty frame of reference returns null. A valid 3-wide `POINT` is copied with its frame, and the points array, the point row, and the concept are different objects. A caller write of that point left the copy unchanged. `POINT` requires one point. `POLYLINE` returns null for one point and copies two. `CIRCLE` copies two and returns null for one or three. The same counts hold for `SCOORD3D`. A non-finite coordinate returns null. Child order is preserved.

`provenanceItems` omits a null algorithm and returns tool, engine, and operator as `HAS_OBS_CONTEXT` text, in that order. An empty tool, engine, operator, or non-null algorithm returns null. A present algorithm is the third of four texts. After a refusal, a later valid call still returns a fresh tree.

The concept on every checked node is three strings. Empty scheme, value, and meaning each return null. A `CODE` payload is copied into its own object. Writing the copy concept does not change that payload or the caller. The local provenance labels are scheme `OCELLI` with values `tool`, `engine`, `algorithm`, and `operator`. The plan's local rule is three non-empty strings. It does not assign SNOMED CT or UCUM codes to these children. The transcribed HLD sentence names those systems and then leaves the codes unset. The labels do not contradict the local rule.

All eight value types copied as a root and as a `CONTAINS` child. `sr.ts` has no `as` cast. The test casts a foreign child and a bare `DATE` root so those refusals are reachable. Those casts are not numeric conversions.

`git diff -- packages/core/src/annotation.ts` was empty. A search for `contentTree`, `provenanceItems`, and `SCOORD3D` in Rust and TypeScript found those names only in the two reviewed files. There is no Rust twin. The drawing-layer comment still says the display list is not this tree.

The module imports nothing. It does not read pixels. No `unsafe`. No `wasm-bindgen`. Tiers A, B, and C are not branched. The plan marks each tier n/a.

## Limitations and handoff

The mutation is not left in place. `cmp` against the saved original was exit 0, and the sha256 is `0557fb71fc8795422fcfb2002c7eb93ffcc97d3a14e38493a7cac9283e2f1da8`.

The plan's API section says `index.ts` exports `contentTree` and `provenanceItems`. That file does not name either function. The plan's write list names `sr.ts`, its test, the plan, and the LLD paragraph. It does not name `index.ts`. `index.ts` was not a reviewed file and it is unchanged. This pass does not count the missing export. `docs/lld/` does not name either function. `docs/lld/public-api.md` still says F-094 owns the type and has no row for these functions. That paragraph was not a reviewed file and is not counted.

A graphic named `ELLIPSE` with two 2-wide points is copied, and the graphic string is kept. `graphicCount` uses the circle count for every name other than `POINT` and `POLYLINE`. The plan states counts for those three names. It does not say that another graphic returns null. This pass does not count that acceptance.

An empty `UIDREF` uid returns null. An empty `IMAGE` sop class returns null. An empty `IMAGE` sop instance returns null. An empty `TEXT` payload is copied. The plan requires a non-empty string for concept fields, the `NUM` unit, the `SCOORD3D` frame of reference, and the provenance arguments. The empty-string refusal sits in the `provenanceItems` paragraph. The extra uid and image refusals, and the empty text copy, are not counted.

A cast `SCOORD` with no points array throws `TypeError: Cannot read properties of undefined (reading 'length')`. A cast `CONTAINER` with no children array throws `TypeError: items is not iterable`. A cast `SCOORD3D` with points and no frame of reference throws the same length `TypeError`. A two-node cycle whose child relationships are both `CONTAINS` throws `RangeError: Maximum call stack size exceeded`. Those inputs sit outside the typed item and outside the acceptance rows. This pass does not count them.

The shipped tests do not themselves cover a non-null root relationship, a missing child relationship, a bad grandchild, an empty unit, `POLYLINE`, a two-point circle, `SCOORD3D`, empty engine, empty operator, or an empty algorithm string. The two tests added after pass 1 cover the foreign value type and the provenance concept identity. The probe executed the remaining cases on the pre-mutation bytes. The restore matched that hash. No feature profile was run. No browser and no pixel oracle were used. This story has no rendered parity view.

No patient data was read.
dwv, Horos, and the medical viewer Grok were not opened.
