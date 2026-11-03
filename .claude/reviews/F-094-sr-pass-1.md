# F-094 review, pass 1

**Reviewed**: untracked product files on `sprint/s35`. HEAD is `79eb0aeda6c177b6318119af6eabd750119728c9` (`79eb0ae`). Nothing was staged. `git diff --name-only` was empty. The reviewed sources are `packages/core/src/sr.ts` (sha256 `86bd597b2041dc2621102644dbd7b8eb5a5936e8e3b20830faad81274b8d6c19`) and `packages/core/src/sr.test.ts` (sha256 `0c773adbbc15f0710a8ffbc52a1116e86baf3f8404de9b179f5d1adad0df535d`). `packages/core/src/annotation.ts` was not in that name list.
**Author / reviewer**: The author is the parent implementation agent. The reviewer is independent subagent `01a19e85-9c0a-7c41-b1a0-cfde5c7ec570` and did not write the plan or the diff.
**Result**: 1 defect, 1 smell, 0 nitpicks

## Acceptance challenged

The approved plan is the contract. The HLD does not define the node fields. That absence is not a defect. No other viewer was used as the specification.

AC1. A container root with one `CONTAINS` `NUM` child of 12.5 and unit `mm` must copy both nodes. The caller child must be a different object. The shipped expect is deep equality with that child, plus two identity checks. The probe received 12.5 and `mm`. The root, the child, and both concept objects were different objects. The caller JSON was unchanged. A frozen input still copied.

AC2. A `POINT` `SCOORD` of `(1, 2)` must stay those two numbers. The shipped expect is deep equality with that root. Changing the SCOORD width constant from 2 to 3 made the call return null, and the expect received null. A probe write of the caller point from 1 to 9 left the copy at 1. The copy, the points array, and the point row were different objects.

AC3. A `CIRCLE` with one point returns null. `NaN` returns null. An empty concept value returns null. The shipped expects are those three nulls. Accepting a one-point circle would fail the first. Copying `NaN` would fail the second. Copying an empty value would fail the third. The same restored module also returned null for `Infinity`, for `-Infinity`, for an empty scheme, and for an empty meaning. A circle of two points was copied. A circle of three points returned null.

AC4. Provenance with tool `Length`, engine `0.1.0`, algorithm null, and operator `reader` returns three text children. The algorithm child is absent. An empty tool returns null. The shipped expects lock the length, the three texts, and the empty tool. Inserting an algorithm child would fail the length and the text list. Returning a list for an empty tool would fail the null expect. A separate probe with algorithm `v1` returned `Angle`, `0.2.0`, `v1`, and `other` in that order. Empty engine, empty operator, and an empty algorithm string each returned null.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `npm test -- packages/core/src/sr.test.ts` from the repo root | Restored SCOORD width 2. sha256 `86bd597b2041dc2621102644dbd7b8eb5a5936e8e3b20830faad81274b8d6c19` | executed now, before the mutation | Exit 0. 4 passed. |
| Same command after `points(item.points, 2, item.graphic)` became width 3 | One constant in `packages/core/src/sr.ts`. Mutated sha256 `ff221b9164bb283c1f3ab18dc98ebedc16170495b533c82fa14a06786f894685`. No other test file was run. | executed now | Exit 1. 1 failed and 3 passed. AC2 received null and expected the `(1, 2)` point at `sr.test.ts:43`. |
| `cmp` of the saved original against `packages/core/src/sr.ts`, then the same test | Bytes copied back from the pre-mutation file. The SCOORD call uses width 2 again. sha256 again `86bd597b2041dc2621102644dbd7b8eb5a5936e8e3b20830faad81274b8d6c19` | executed now | `cmp` exit 0. Test exit 0. 4 passed. The test file sha256 stayed `0c773adbbc15f0710a8ffbc52a1116e86baf3f8404de9b179f5d1adad0df535d`. |
| Node probe, before the mutation | Same bytes as the restored file, sha256 `86bd597b2041dc2621102644dbd7b8eb5a5936e8e3b20830faad81274b8d6c19`. Root relationship, missing child relationship, bad grandchild, empty unit, widths, graphic counts, non-finite points, provenance strings, frozen caller, foreign value type, shared concept. | executed now, no repo file added | Exit 0. The checklist rows matched the plan. The foreign value type and the shared concept did not. The probe was not repeated after the restore. |

Vitest 4.1.11. Node v24.13.0. No cargo, no wasm-pack, and no `bin/ocelli.sh`.

The restored comparison is:

```ts
const copied = points(item.points, 2, item.graphic);
```

## Defects

### D1. A value type outside the eight is not refused

**Where**: `packages/core/src/sr.ts:193`
**What**: The last arm copies a node as `SCOORD3D` without testing the value type. A container with one `CONTAINS` child of value type `DATE`, graphic `POINT`, one 3-wide point, and frame of reference `1.2.3` is returned. The child comes back as `SCOORD3D`. A `DATE` root with a concept and no points throws `TypeError: Cannot read properties of undefined (reading 'length')` at the point-list length check.
**Why it is wrong**: The plan says the value type is one of `CONTAINER`, `TEXT`, `NUM`, `CODE`, `UIDREF`, `IMAGE`, `SCOORD`, and `SCOORD3D`. One bad node returns nothing. A refusal returns null and writes nothing. `DATE` is not in that list, so the whole tree must be null. The comment on `contentTree` says one bad node returns nothing.
**Evidence**: The node probe on the pre-mutation module. The restore later matched that sha256, and the probe was not run again. The nested `DATE` child was copied as `SCOORD3D`. The bare `DATE` root threw. The shipped suite never builds a foreign value type, so those four tests stayed green.
**Scope and regression**: `contentTree` is the affected function. `index.ts` does not export it, so there is no package caller yet. The smallest regression is a valid container whose one child has a value type outside the eight and otherwise satisfies the 3D point checks. The result must be null, including when the foreign node has no point list. Uncertainty: a typed caller needs a cast to build that node. A relationship string outside the five already returns null, so this arm is the one that does not apply the same refusal.

## Smells

### S1. Provenance concept objects are shared module state

**Where**: `packages/core/src/sr.ts:225`
**What**: `provenanceText` stores `PROVENANCE[role]` on the returned item. Two calls return the same tool concept object. Writing `scheme` on the first result changes the scheme of the next call. The same assignment is used for engine, algorithm, and operator.
**Why it is a smell**: The three texts, the omitted null algorithm, and the empty-string refusals match the plan. The plan does not say each provenance concept object must be new. The failure appears when a caller writes a `Concept` field. The next call keeps that write. The plan says the functions do not keep state and the function owns the copy. This is not a finding about the `OCELLI` words. Those labels are the plan's gap, and they do not contradict the three-string rule.
**Evidence**: On the pre-mutation module, the two tool concept objects compared equal. After `scheme` was set to `MUTATED`, the next call returned `MUTATED`. Setting it back to `OCELLI` made the following call return `OCELLI`. The source sha256 did not change. The write lived only in that process. The restore later matched the probed bytes, and the probe was not run again.
**Scope and regression**: `provenanceItems` for all four roles. The smallest regression is two calls whose tool concepts are different objects, and a scheme write on the first that leaves the second unchanged.

## Nitpicks

None.

## Verified clean

`contentTree` returns null when the root relationship is not null. A child with a null relationship returns null. Each of `CONTAINS`, `HAS_OBS_CONTEXT`, `HAS_PROPERTIES`, `INFERRED_FROM`, and `SELECTED_FROM` is kept on a child. A bad relationship string returns null. One empty unit on a grandchild returns null for the whole tree. The caller object is not written. A frozen root and child still produce a copy, and the caller JSON after a normal copy is the same string as before the call.

`NUM` returns null for `NaN`, `Infinity`, and `-Infinity`, and for an empty unit. Zero is copied. The unit is not checked against a UCUM table.

An `SCOORD` point must be exactly two finite numbers. A 3-wide point returns null. An `SCOORD3D` point must be exactly three finite numbers, and an empty frame of reference returns null. A valid 3-wide `POINT` is copied with its frame. `POINT` requires one point. `POLYLINE` returns null for one point and copies two. `CIRCLE` copies two and returns null for one or three. A non-finite coordinate returns null.

`provenanceItems` omits a null algorithm and returns tool, engine, and operator as `HAS_OBS_CONTEXT` text, in that order. An empty tool, engine, operator, or non-null algorithm returns null. A present algorithm is the third of four texts.

The concept on every checked node is three strings. Empty scheme, value, and meaning each return null. The local provenance labels are scheme `OCELLI` with values `tool`, `engine`, `algorithm`, and `operator`. The plan's local rule is three non-empty strings. It does not assign SNOMED CT or UCUM codes to these children. The transcribed HLD sentence names those systems and then leaves the codes unset. The labels do not contradict the local rule.

`git diff -- packages/core/src/annotation.ts` was empty. A search for `contentTree`, `provenanceItems`, `HAS_OBS_CONTEXT`, and `SCOORD3D` found those names only in the two reviewed files. There is no Rust twin.

The module imports nothing. It does not read pixels. No `unsafe`. No `wasm-bindgen`. Tiers A, B, and C are not branched. The plan marks each tier n/a.

## Limitations and handoff

The mutation is not left in place. `cmp` against the saved original was exit 0, and the sha256 is `86bd597b2041dc2621102644dbd7b8eb5a5936e8e3b20830faad81274b8d6c19`.

The plan's API section says `index.ts` exports `contentTree` and `provenanceItems`. That file does not name either function. The plan's write list names `sr.ts`, its test, the plan, and the LLD paragraph. It does not name `index.ts`. `index.ts` was not a reviewed file and it is unchanged. This pass does not count the missing export. `docs/lld/public-api.md` still says F-094 owns the type and has no row for these functions. That paragraph was not a reviewed file and is not counted.

A graphic named `ELLIPSE` with two 2-wide points is copied, and the graphic string is kept. `graphicCount` uses the circle count for every name other than `POINT` and `POLYLINE`. The plan states counts for those three names. It does not say that another graphic returns null. This pass does not count that acceptance.

An empty `UIDREF` uid returns null. An empty `IMAGE` SOP class returns null. An empty `TEXT` payload is copied. The plan requires a non-empty string for concept fields, the `NUM` unit, the `SCOORD3D` frame of reference, and the provenance arguments. The empty-string refusal sits in the `provenanceItems` paragraph. The extra uid and image refusals, and the empty text copy, are not counted.

The shipped tests do not themselves cover a non-null root relationship, a missing child relationship, a bad grandchild, an empty unit, point width, `POLYLINE`, a two-point circle, `SCOORD3D`, empty engine, empty operator, or an empty algorithm string. The probe executed those cases on the pre-mutation bytes. The restore matched that hash. A cyclic child list was not exercised. No feature profile was run. No browser and no pixel oracle were used. This story has no rendered parity view.

No patient data was read.
dwv, Horos, and the medical viewer Grok were not opened.
