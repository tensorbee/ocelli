# S35 review, pass 1

**Reviewed**: Range `c3f85e51a6c5e0a7e71ae9ed17b90faf2dcc6b19..HEAD` on `sprint/s35`. HEAD is `2ac7844150c0f074d62321e2d9bd5bb7615f1f2f` (F-099). Tree is `d7d75e3f7d42d36496feb0125e4924b9c12a0597`. `git status --short` was empty before this pass and empty again after the four restores and after both probes were deleted. This review file is not part of that tree.

**Author / reviewer**: The author of the implementation is the parent implementation agent. The reviewer is independent subagent `01a19ecf-1f73-79a1-ae32-d7db473b42a3` and did not write the open, the design, the feature diffs, or the story reviews.

**Result**: 1 defect, 0 smells, 0 nitpicks

## Acceptance challenged

The approved plans are the contract. A missing HLD formula was not treated as specified. Cornerstone behaviour was not treated as specified. Closing S35 does not complete M11 and does not publish. F-096 and F-097 stay in S36. F-X003 stays in progress.

F-094. In `packages/core/src/sr.ts` the SCOORD width was changed from 2 to 3. `npm test -- packages/core/src/sr.test.ts` exited 1. `keeps one 2D point` received null and expected the point `(1, 2)`. The other five tests passed. The saved bytes were copied back. `cmp` matched. sha256 returned to `0557fb71fc8795422fcfb2002c7eb93ffcc97d3a14e38493a7cac9283e2f1da8`.

F-095. In `packages/core/src/annotation-state.ts` the blank-id comparison was changed from `id.length === 0` to `id.length === 2`. `npm test -- packages/core/src/annotation-state.test.ts` exited 1. The empty id was stored. The expect at line 111 wanted null. The other four tests passed. Those four use the one-character ids `a`, `b`, and `c`. The saved bytes were copied back. `cmp` matched. sha256 returned to `6c7f9f15858248c1043f84fe95718ed22773b41ce81198183dd12ad0d7e8438c`.

F-098. In `crates/ocelli-seg/src/seg_roundtrip.rs` `EXACT_INT_LIMIT` was changed from `9_007_199_254_740_992.0` to `0.0`. `bin/ocelli.sh test ocelli-seg --test seg_roundtrip` exited 101. `a_bad_frame_refuses_the_whole_call` received `Ok` for samples `[1.5, 0.0]` and expected `Err(Sample)`. The other three tests passed. The saved bytes were copied back. `cmp` matched. sha256 returned to `8dc88e3aed9b232509105b9c01aeb8a721d7a8c760fa8fa82fe01cd330848679`.

F-099. In `crates/ocelli-seg/src/contour.rs` `spacing / 2.0` was changed to `spacing / 1.0`. `bin/ocelli.sh test ocelli-seg --test rtstruct_import` exited 101. `half_spacing_keeps_10_9_and_refuses_11_1` received `Ok` with caller z `11.1` and expected `Err(OutOfPlane)`. The other four tests passed. The saved bytes were copied back. `cmp` matched. sha256 returned to `997c27cb9a7ad6476ee60bd24acc744df42cf27191a500b7a7fe314889877ab3`.

One mutation was in place at a time. Each restore used the blob saved before that edit. After every restore the tree was still `d7d75e3f7d42d36496feb0125e4924b9c12a0597` and status was empty.

A shell probe, deleted after the run, put provenance children into a container and then into annotation state. It also replaced a middle id, moved one id to another frame, redid the middle state after two undos, and kept redo after a refused put. `npm test -- packages/core/src/s35-review-probe.test.ts` exited 0. 3 passed. A Rust probe, deleted after the run, checked the low side of the slab, a mixed contour, a negative spacing, an open contour, a point contour, decode of a non-integer, a zero column, and the unchanged presenters. `bin/ocelli.sh test ocelli-seg --test s35_review_probe` exited 0. 1 passed.

## Evidence

| Command | Identity | Executed now | Exit and observation |
|---|---|---|---|
| `git rev-parse HEAD`, `git status --short`, `git rev-parse HEAD^{tree}` | Branch `sprint/s35` before any edit | Yes | Exit 0. HEAD `2ac7844150c0f074d62321e2d9bd5bb7615f1f2f`. Status empty. Tree `d7d75e3f7d42d36496feb0125e4924b9c12a0597` |
| `git diff --name-only` for the review range | 27 paths | Yes | Exit 0. `annotation.ts` and `labelmap.rs` are absent. The contour diff adds `import_rtstruct_contour` after `rounded_i32` |
| Pre-edit `shasum -a 256` of the four modules | Compared with the four expected hashes | Yes | All four matched before the first edit |
| Contributing F-ID line against the public-api row | `docs/lld/public-api.md` header and the last cell of the `public-api.md` row in `docs/lld/README.md` | Yes | The two strings are equal, both length 251, and both end in F-099 |
| Sprint markdown through `scripts/prose_check.py` rules, plus the six commit messages | Fences, table rows, and inline code excluded | Yes | 0 problems. No em-dash. No prose semicolon |
| F-094 SCOORD width forced to 3, then the content-tree test | Only `sr.ts` differed from its blob | Yes | Exit 1. Vitest 4.1.11. Received null against the `(1, 2)` point. 5 passed |
| Blob copy, `cmp`, `shasum -a 256` | Restored `sr.ts` | Yes | `cmp` exit 0. Hash `0557fb71fc8795422fcfb2002c7eb93ffcc97d3a14e38493a7cac9283e2f1da8`. Tree unchanged. Status empty |
| F-095 blank-id length forced to 2, then the state test | Only `annotation-state.ts` differed from its blob | Yes | Exit 1. Vitest 4.1.11. Empty id stored. Line 111 expected null. 4 passed |
| Blob copy, `cmp`, `shasum -a 256` | Restored `annotation-state.ts` | Yes | `cmp` exit 0. Hash `6c7f9f15858248c1043f84fe95718ed22773b41ce81198183dd12ad0d7e8438c`. Tree unchanged. Status empty |
| F-098 integer limit forced to 0, then the SEG test | Only `seg_roundtrip.rs` differed from its blob | Yes | Exit 101. Received `Ok` for `[1.5, 0.0]` against `Err(Sample)`. 3 passed |
| Blob copy, `cmp`, `shasum -a 256` | Restored `seg_roundtrip.rs` | Yes | `cmp` exit 0. Hash `8dc88e3aed9b232509105b9c01aeb8a721d7a8c760fa8fa82fe01cd330848679`. Tree unchanged. Status empty |
| F-099 divisor forced to 1, then the RTSTRUCT test | Only `contour.rs` differed from its blob | Yes | Exit 101. Received `Ok` for z 11.1 against `Err(OutOfPlane)`. 4 passed |
| Blob copy, `cmp`, `shasum -a 256` | Restored `contour.rs` | Yes | `cmp` exit 0. Hash `997c27cb9a7ad6476ee60bd24acc744df42cf27191a500b7a7fe314889877ab3`. Tree unchanged. Status empty |
| Deleted shell probe of provenance, replace, move, and redo | Restored modules. Probe path removed in the same command | Yes | Exit 0. Vitest 4.1.11. 3 passed. Status empty. Tree unchanged |
| Deleted Rust probe of the slab, decode, and the two presenters | Restored modules. Probe path removed in the same command | Yes | Exit 0. 1 passed. Status empty. Tree unchanged |
| Four `shasum -a 256`, `git status --short`, `git rev-parse HEAD^{tree}` | After both probes | Yes | The four hashes match the values above. Status empty. Tree unchanged |

## Defects

### D1. The F-095 note describes the wrong red row

**Where**: `docs/sprints/AS_BUILT.md` line 4477.

**What**: The completion note says a blank-id length of 2 refused a one-character id. The mutation stores the empty id. The one-character ids `a`, `b`, and `c` still pass.

**Why it is wrong**: `.claude/plans/F-095-design.md` AC5 says a blank id returns null. The shipped blank id is the empty string at `packages/core/src/annotation-state.test.ts` line 111. The other rows use one-character ids. A comparison with length 2 does not refuse those ids. The note states the opposite of that acceptance row. The same tree's F-095 story review records the empty id being stored and the other four tests staying green.

**Evidence**: `npm test -- packages/core/src/annotation-state.test.ts` after the length change exited 1. Vitest 4.1.11 reported 1 failed and 4 passed. The failure was `expected { records: [ { id: '', ... } ], ... } to be null` at line 111. `cmp` then exited 0 and the hash returned to `6c7f9f15858248c1043f84fe95718ed22773b41ce81198183dd12ad0d7e8438c`.

**Scope and regression**: The function on the restored tree still refuses the empty id. The false sentence is the completion note. The smallest change is to say that a length of 2 stored the empty id and that the one-character ids stayed. This pass did not edit the note.

## Smells

None.

## Nitpicks

None.

## Verified clean

`contentTree` copies a valid tree and returns null for one bad node. The root relationship must be absent. A child relationship is one of the five names. The eight value types are checked before the payload is read. A `NUM` must be finite and its unit must be non-empty. The unit is not checked against a UCUM table. An `SCOORD` point is two finite numbers. An `SCOORD3D` point is three finite numbers and carries a non-empty frame of reference. `POINT` has one point. `POLYLINE` has at least two. `CIRCLE` has two. The caller object is not written. `provenanceItems` returns tool, engine, algorithm, and operator as `HAS_OBS_CONTEXT` text, in that order. A null algorithm omits that child. An empty tool, engine, operator, or non-null algorithm returns null. The probe stored four provenance children under one container. A write of the caller text left the stored text at `v1`. A provenance item passed as a root returned null. There is no Rust twin. `annotation.ts` is not in the diff. Its comment still says the display list is not this tree.

`createAnnotationState` starts empty. `putAnnotation` appends a new id and replaces the same id in place. A blank id, a blank UID, or a tree `contentTree` refuses returns null and does not replace the previous arrays. The probe replaced middle id `b` with number 4, kept order `a`, `b`, and restored number 2 on undo. The same id moved from `1.2.3` to `1.2.9` and left only `b` on the first frame. `annotationsForFrame` returns exact UID matches in insertion order. An interleaved `a`, `c`, `b` put listed `a`, `b` for `1.2.3` and `c` for `1.2.4`. The listed tree was not the caller tree. Undo and redo move one record list. Two undos from three puts left id `a`. One redo restored `a`, `b`. The next redo restored `a`, `b`, `c`. A further redo returned null. A refused put left that redo in place. A successful put clears redo. The module calls `contentTree` and does not reimplement the tree check. It does not measure.

`encode_seg` and `decode_seg` copy frames in caller order. Both use `copy_frames`. There is no sort. The same SOP can occur twice. Segment 0 stays. One bad frame returns an error before the copy loop. The caller slice is not written. A sample must be a finite integer of at least 0. `1.5` is `SegError::Sample` on encode and on decode. A zero column is `SegError::Side`. The module is private. The two functions and their two types are re-exported. There is no TypeScript copy. `packages/` has no `encode_seg`, `decode_seg`, or `import_rtstruct` name. The diff does not include `labelmap.rs`. A 1 by 2 grid built from decoded samples 0 and 1 still painted a transparent background and outline bytes `[10, 20, 30, 255]`. That is the existing presenter, not a SEG file.

`import_rtstruct_contour` calls `kind_len`. That helper is the only geometric-type length match. The contour diff does not change `present_contour`. Segment 0 is `MissingColour`. A short closed contour is `Length`. Spacing must be finite and greater than 0. Spacing `-1` was `OutOfPlane`. A point at z `9.0` with slice z `10` and spacing `2` was kept, and the stored z was `9.0`. A point at z `8.9` was `OutOfPlane`. One point at z `11.1` after two points at z `10` refused the whole contour and left the caller vector unchanged. An open pair and a single point kind were accepted. The stored z for an in-slab point at `10.9` stayed `10.9`. `present_contour` of that point on a plane at z `10` returned `OutOfPlane` and left the sentinel bytes unchanged. The same import at z `10` painted `[10, 20, 30, 128]`. The import does not project and does not snap z to the slice.

`index.ts` exports `contentTree`, `provenanceItems`, and the five annotation-state functions. `PUBLISHED` names those same runtime values. `VERSION` is the literal `0.1.0`. `packages/core/package.json` is `0.1.0`. The workspace `Cargo.toml` version line is `0.1.0`. Neither version file is in the sprint diff.

The four product files have no `unsafe` and no `wasm-bindgen`. They do not open a device. Tiers A, B, and C are not branched. The plans and the as-built rows mark all three n/a. There is no second annotation type beside the content tree. There is no Part 10 SEG writer. RTSTRUCT points stay patient millimetres.

The four feature commits carry `profile=feature` verify trailers. Their tree prefixes are `ee94d62840f4`, `6fc71ca43911`, `4dd08354c81e`, and `d7d75e3f7d42`, and those prefixes are the trees of those commits. The open and the design commits have no verify trailer. The trailers were read. The gates were not re-run. A trailer is not proof of behaviour.

`CHANGELOG.md` under Unreleased still says nothing is published. The four new bullets are newest first. They name the RTSTRUCT import, the SEG frame copy, the frame-of-reference undo, and the SR content tree. `CURRENT_SPRINT.md` says closing this sprint does not complete M11 and does not publish. Its table marks F-094, F-095, F-098, and F-099 done. The backlog marks those four done. F-096 and F-097 are pending on S36. F-X003 stays `in-progress`. The backlog command for S35 prints those four done rows. Tag `s34` is `bff12197` and peels to `c3f85e5`. That merge tree is `ae3fbbc21b6a`, which is also the tree of `1c1b85a`. The carried F-089, F-092, F-093, open, and design prefixes match. `ci/wasm-size-budget.json` records 2151196 bytes and tolerance 0.05. `int(baseline * (1 + tolerance))` in `scripts/pin_and_size_check.py` is 2258755. This pass did not rebuild wasm.

The sprint diff does not edit `annotation.ts`, `labelmap.rs`, `DEVIATIONS.md`, a tolerance, or the wasm size budget. No DICOM preamble and no patient name were in the diff.

## Limitations and handoff

The sprint gate was not run. No wasm-pack and no oracle run was started. The measured wasm size named in `CURRENT_SPRINT.md` was not rebuilt here. The budget file and the ceiling formula were read.

`graphicCount` uses the circle count for every name other than `POINT` and `POLYLINE`. The plan states counts for those three names. It does not say that another graphic returns null. This pass did not execute another graphic and does not count that acceptance. An empty `TEXT` payload is not an acceptance row. A non-finite slice z is refused by the finite check. The plan names bad spacing and a z outside the slab. It does not name a separate error for the slice value. This pass does not count that choice.

The shell probe and the Rust probe were deleted before the final status check. Both exited 0 on the restored bytes. The Rust compile printed one unused-mut warning in the deleted probe. The product files were not edited by that warning.

D1 is a false completion sentence. The restored `putAnnotation` still refuses the empty id. This pass does not clear the sprint. It did not stage, commit, push, or record a sprint review. The next change is the note. A later independent pass has to review that edit.

This file is untracked until the parent stages it.

dwv, Horos, and the medical viewer Grok were not opened.
