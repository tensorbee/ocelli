# S36 review, pass 1

**Reviewed**: Range `ea3f140..HEAD` on `sprint/s36`. HEAD is `36366831755f94c96edeadfcbf4febce4f841529`. Tree is `aa8ce28a216db58faddf21eacf05437962b3f52b`. This review file is not part of that tree.

**Reviewer**: Independent subagent `01a19f35-c66b-72a0-a02c-f150bd0eb054`. This reviewer did not write the S36 code, the designs, or the story reviews.

**Result**: 0 defects, 0 smells, 0 nitpicks

## Acceptance challenged

The approved plans are the contract. A missing HLD formula was not treated as specified. Cornerstone behaviour was not treated as specified. Closing this sprint completes M11 and does not publish. The version stays 0.1.0. F-X003 stays in progress.

F-096. The pre-edit sha256 of `packages/core/src/measurement-persist.ts` was `2ab55d311dcb37ea3d64d988e8b6bca3e1f11c309489f511c1bbe4a235c40a25`, which is the expected hash. The only bulk comparison, `count >= MEASUREMENT_ITEM_LIMIT`, was changed to `count > MEASUREMENT_ITEM_LIMIT`. `npm test -- packages/core/src/measurement-persist.test.ts` exited 1. `persists 3999 nodes and refuses 4000` expected null and received a JSON document for a 4000-node tree. The other four tests passed. The saved blob was copied back. `cmp` matched. The sha256 returned to the same value. The tree was `aa8ce28a216db58faddf21eacf05437962b3f52b` and status was empty.

F-097. The pre-edit sha256 of `packages/core/src/tid1500.ts` was `77ecc227e51813e66f7aad48a997659781a3d61c99d83a43348cd1fe5645c4cc`, which is the expected hash. The only `CodeValue: num.unit` was changed to `CodeValue: "cm"`. `npm test -- packages/core/src/tid1500.test.ts` exited 1. `reads 12.5 mm back with the same concept` kept the number 12.5 and received unit `cm` where it expected `mm`. The two-measurement test failed the same way. The other six tests passed. The saved blob was copied back. `cmp` matched. The sha256 returned to the same value. The tree and status matched the restored F-096 state.

One mutation was in place at a time. Each restore used the blob saved before that edit.

On the restored tree, a probe under `/tmp` imported the shell the way the feature tests do. A persisted NUM of 12.5 mm loaded, and `writeTid1500` then `readTid1500` returned 12.5, `mm`, and the same concept. A nested NUM of -12.5 cm did the same. An SCOORD point persisted with its graphic and its point, and `writeTid1500` of that loaded tree returned null. `MEASUREMENT_ITEM_LIMIT` is not a key of the package index object. `persistMeasurements`, `loadMeasurements`, `writeTid1500`, and `readTid1500` are functions on that object. The written file meta transfer syntax is `1.2.840.10008.1.2.1`. The dataset SOP class is `1.2.840.10008.5.1.4.1.1.88.34`. The evidence series is the source series, and the document series UID is different. The referenced SOP class and instance are the source values. A value of 0.1 reads back as 0.1 mm. A list of 3999 NUM roots reads back with length 3999, first number 0 and last number 3998. A list of 4000 NUM roots returns null. An empty input writes a template document and reads back as an empty array. That empty call is not a defect. The plan refuses a root with no NUM. It does not refuse a call that has no root.

The public-api header in `docs/lld/public-api.md` and the last cell of the public-api row in `docs/lld/README.md` are the same string. Both have length 265 and both end in F-097.

## Evidence

| Command | Identity | Executed now | Exit and observation |
|---|---|---|---|
| `git rev-parse HEAD`, `git write-tree`, `git status --short` | Branch `sprint/s36` before any edit | Yes | HEAD `36366831755f94c96edeadfcbf4febce4f841529`. Tree `aa8ce28a216db58faddf21eacf05437962b3f52b`. Status empty |
| `shasum -a 256` of the two product modules | Compared with the two expected hashes | Yes | Both matched before the first edit and again after each restore |
| `git diff --name-only ea3f140..HEAD` | 21 paths | Yes | No path under `crates/`. `annotation.ts` is absent |
| Contributing F-ID line against the public-api row | Header after the label, and the last cell of the `public-api.md` row | Yes | Equal. Length 265. Both end in F-097 |
| Added sprint markdown, fences and table rows skipped, plus `prose_check.py --commit-msg` on four messages | The four commits and the markdown added in the range | Yes | 0 added-line problems. Each commit message exited 0 |
| F-096 comparison forced from `>=` to `>`, then the persistence test | Only `measurement-persist.ts` differed from its blob | Yes | Exit 1. Vitest 4.1.11. A 4000-node document was stored. 4 passed |
| Blob copy, `cmp`, `shasum -a 256`, `git write-tree` | Restored `measurement-persist.ts` | Yes | `cmp` exit 0. Hash `2ab55d311dcb37ea3d64d988e8b6bca3e1f11c309489f511c1bbe4a235c40a25`. Tree unchanged. Status empty |
| F-097 unit code forced to `cm`, then the TID 1500 test | Only `tid1500.ts` differed from its blob | Yes | Exit 1. Vitest 4.1.11. A millimetre measurement read back as cm. 6 passed |
| Blob copy, `cmp`, `shasum -a 256`, `git write-tree` | Restored `tid1500.ts` | Yes | `cmp` exit 0. Hash `77ecc227e51813e66f7aad48a997659781a3d61c99d83a43348cd1fe5645c4cc`. Tree unchanged. Status empty |
| `npm test -- packages/core/src/measurement-persist.test.ts packages/core/src/tid1500.test.ts packages/core/src/index.test.ts` | Restored tree | Yes | Exit 0. 17 passed. 5 persistence, 8 TID 1500, 4 index |
| `npx tsc -p packages/core --noEmit --pretty false` | Restored tree. `--incremental false` was not passed | Yes | Exit 0. No diagnostic |
| `/tmp/s36-probe.test.ts` through vitest | Restored modules. The probe is not in the repo | Yes | Exit 0. 7 passed, including the 3999-root list |
| `packages/core/package.json` and the lockfile entry `node_modules/dcmjs` | Package version and the dcmjs dependency | Yes | Version `0.1.0`. Dependency exactly `0.52.2`. Lockfile resolves `0.52.2` |
| Backlog rows, current-sprint table, and Unreleased changelog | F-096 and F-097 status | Yes | Both backlog rows are `done`. The sprint table marks both `done`. Unreleased names the JSON document and the TID 1500 document. F-X003 stays `in-progress` |
| `pgrep -lf 'cargo\|rustc\|wasm-pack'` | Host processes before and after the checks | Yes | No matching process |

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

`persistMeasurements` copies every root through `contentTree` before it writes. One bad node returns null and does not write the caller nodes. The count is every copied node. A count of 4000 or more returns null from persist and from load, because both use the same comparison. An empty list loads as a new empty array. The document format is `ocelli-measurement` and the version is 1. A wrong format, a wrong version, a non-object, and invalid JSON return null. There is no bulk binary layout. `index.ts` exports the two functions and does not export the limit. The constant stays a module export so F-097 can use the same threshold.

`writeTid1500` copies every root through `contentTree`. A null copy returns null. Each copied root must contain one NUM. Zero or two return null. The node count uses `MEASUREMENT_ITEM_LIMIT`, which is 4000. The concept is `ConceptNameCodeSequence`. The number is `NumericValue`. The unit code value uses scheme `UCUM`. An operator TEXT with scheme `OCELLI` and value `operator` sets the dcmjs person name, and the reader does not return that name. The shell is `TID1500MeasurementReport`. Each measurement is one `TID1501MeasurementGroup`. `DicomDict.write` produces the bytes. A dcmjs throw returns null.

The reader calls `DicomMessage.readFile`. In dcmjs 0.52.2 that is the file reader, and it returns a `DicomDict`. `DicomDict` itself has `write` and does not have `readFile`. The plan names `DicomDict.readFile`. The call that exists is the one that reads the Part 10 buffer. A template other than `1500` returns null. The reader keeps the content item whose code is `126010` and requires one NUM under each group. The returned roots are copies from `contentTree`, in group order, with a null relationship and no children. The number, the unit, and the three concept strings survive. Language, procedure, and the observer are not returned as measurements. SCOORD and SCOORD3D are not mapped. A tree that is only SCOORD is persisted by F-096 and refused by `writeTid1500`.

`packages/core/package.json` depends on `dcmjs` at exactly `0.52.2`. The package version is `0.1.0`, and the index `VERSION` test expects that literal. No Rust measurement twin was added. `annotation.ts` is not in the diff. No `unsafe` and no `wasm-bindgen` were added. Tiers A, B, and C are n/a in both plans, and the new functions do not open a device.

HLD section 34 states that the in-memory type is a DICOM SR content-item tree and that a bulk binary is the other tier once counts pass a few thousand. It does not give a byte layout or the count 4000. Those two choices are the F-096 plan. Decision D10 keeps dcmjs for SR generation. Section 10 of `docs/hld/07-concurrency-and-typescript.md` places the dcmjs bridge for TID 1500 in TypeScript. It does not map a NUM onto `MeasuredValueSequence`. The mapping, the SOP class, and the transfer syntax are the F-097 plan. The written meta transfer syntax and the comprehensive 3D SR class match that plan.

The F-096 and F-097 notes in `docs/sprints/AS_BUILT.md` match the files measured here. The recorded module hashes are the restored hashes. The persistence file has five tests and the TID 1500 file has eight. Pass 1 of F-096 reports 1 defect and 0 smells, and pass 2 reports 0 defects, 0 smells, and 0 nitpicks. The F-097 pass reports 0 defects, 0 smells, and 0 nitpicks. The reviewer ids in those notes match the review files. The F-096 commit tree starts with `be883549ab61`, which is the trailer prefix. The F-097 trailer prefix `aa8ce28a216d` is this HEAD tree. Tag `s35` is object `466ffac` and peels to `ea3f140`. The carried S35 tip, tree, and story prefixes match `1986cf2`, `b3315fb23248`, `32d7534`, `4c13d23`, `bec0072`, `2ac7844`, `1c133de`, and `79eb0ae`.

## Limitations and handoff

This pass did not run cargo, rustc, wasm-pack, the oracle, or `bin/ocelli.sh gate`. No such process was running. The feature-profile trailers were read and were not re-executed. The carried wasm byte count was not remeasured. `npm audit fix` was not run. A deep chain that still has fewer than 4000 nodes can throw inside `JSON.stringify`. The plan does not require persist to catch that throw, and the acceptance shape is a wide tree. An empty TID 1500 call writes a template and reads back as an empty array. That is not a refusal of a root with no NUM.

Closing S36 completes M11 and does not publish. This file is untracked until the parent stages it.

dwv, Horos, and the medical viewer Grok were not opened.
