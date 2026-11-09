# F-215 review, pass 1

**Reviewed**: f265d09a..f2ad5de2 on `work/f-215-xai`, tip f2ad5de2, three commits (3abfeaf6, d1a37e7a, f2ad5de2). The worktree was clean apart from the untracked `.venv` and `tools/oracle/out`, which are not part of the story.
**Author / reviewer**: a separate Claude implementer agent wrote the three commits. This pass is an independent Claude reviewer that wrote none of the reviewed material. Both are Claude, so independence is per participant, not per model.
**Result**: 6 defects, 4 smells, 6 nitpicks

## Acceptance challenged

- **AC20, standard check.** Every transcribed clause was read against PS3.3, PS3.6, PS3.10, PS3.5 and PS3.16 edition 2026d, fetched from dicom.nema.org into the session scratchpad. The clause-by-clause result is under Verified clean. The writers match the standard on every attribute Type, code, relationship and value type checked. What the standard check found is in the Part 10 header (D4) and the UID values (S4), neither of which the plan transcribed.
- **AC18, independently derived.** The gate's browser test covers the Load marks input only. A scratch Chromium probe also opened a folder holding a synthetic 80 by 64 image, its saved SR and its Key Object Selection through the real `#folder` input. The page listed 1 series, placed both marks and the key image, and kept a rectangle from 0,0 to 80,64, which is exactly Columns and Rows. The same probe reproduced D1 in the page.
- **Lead amendment 3, discriminating case.** The amendment's own test deletes one of several marks. Deleting the only mark of a study, then saving and reopening, brings the deleted mark back (D2).
- **Lead amendment 6.** The browser test's move clamp was mutated out in a scratch copy and the browser test went red. The freeform grip clamp is reached by no test (S1).
- **Lead amendment 4.** Reversing the newest rule goes red. Dropping the rule's `Ocelli` filter survives (S2).
- **Text round trip.** AC7 uses ASCII text only. A comment holding a non-ASCII character or a backslash does not survive (D1).

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `npm run build -w @ocelli/core`, then `npx vitest run` over tid1500-groups, tid2010, sr, tid1500, index and mark-documents tests | f2ad5de2 | now | 0, 64 of 64 pass |
| `cargo test -p ocelli-folder-viewer --test preview` | f2ad5de2 | now | 0, 118 of 118 pass |
| `npx eslint examples/folder-viewer packages/core/src`, `npx tsc --build`, `python3 scripts/prose_check.py` | f2ad5de2 | now | 0, 0, 0 |
| `bin/ocelli.sh gate viewer-browser` | f2ad5de2, library built, OCELLI_PORT=0 | now, after the implementer's last edit | 0, ALL GREEN, marks.mjs: "Saved 2 marks and 1 key image in 2 files.", rectangle stopped at column 128 and came back at its FL points |
| Scratch harness over `buildDocuments`, `readDocument`, `placeDocuments` | all five kinds, a 5-frame Enhanced CT at frame index 2, a predecessor, an angle | now | 6 marks and 2 key images saved, the angle counted unsaved, every mark placed back at its FL point, frame 3 written as Referenced Frame Number |
| `dsrdump -Ec -Ee +Pc +U8` (DCMTK 3.7.0) on the harness SR and KO | harness files | now | 0 for both. See Second readers |
| `dcmdump -E +U8` on both | harness files | now | 0, no warnings. File meta holds 4 elements (D4) |
| pydicom 3.0.2 `dcmread` on both | harness files | now | reads both, decodes `note é` correctly, meta 4 elements |
| `dciodvfy` | none | not run | not installed, no Homebrew formula |
| Text round trip, `"a\\b"`, `"Läsion"`, `"肺"` | harness | now | `"a"`, `"LÃ¤sion"`, `"èº"` (D1) |
| Chromium probe, `#folder` with a synthetic image, SR and KO | release binary, OCELLI_PORT=0, probe file removed after | now | "1 series. Read 2 documents: 2 marks and 1 key image placed.", text came back `"LÃ¤sion"` |
| Delete-all supersession harness | harness | now | second save writes `["ko"]` only, the deleted rect is placed again (D2) |
| `POST /file` with the SR and KO to the release binary | harness files | now | 200 for both, `sopClassUid` and `studyUid` present |
| Mutation catalogue, 26 mutants over a scratch copy | f2ad5de2 copy | now | 25 red, M16 survived (S2). Table below |
| Browser test against a move clamp mutant | scratch copy, scratch `CARGO_TARGET_DIR` | now | 1, "the dragged rectangle left the image" |
| Rust mutant, Study Instance UID read from (0020,000E) | scratch copy, scratch `CARGO_TARGET_DIR` | now | 101, `inspect_reports_sop_class_and_study` red |

Mutations, all in scratch copies, each reverted. M1 to M3 are the plan's three, the rest are this reviewer's.

| Mutant | Result |
|---|---|
| M1 add 0.5 to the column | red, 8 tests |
| M2 detector length counted as patient mm | red, `detector_length_has_no_num` |
| M3 predecessor skip removed | red, `predecessor_is_skipped` |
| M4 ellipse axis order flipped | red, `ellipse_major_axis_first` |
| M5 rectangle not closed | red, 3 tests |
| M6 SR frame counted from 0 | red, 2 tests |
| M7 KO frame counted from 0 | red, `placement_by_sop_instance` |
| M8 relationship written with an underscore | red, 8 tests |
| M9 IMAGE frame dropped by the writer | red, 4 tests |
| M10 a point on the far edge refused | red, `counts every reason a group is not placed` |
| M11 the older KO wins | red |
| M12 KO reference outside the evidence accepted | red |
| M13 observer context dropped | red |
| M14 Graphic Data not `fround` | red, `graphic_data_is_fl` |
| M15 a repeated Tracking UID placed twice | red |
| M16 the KO newest rule applied to every writer, not only `Ocelli` | **survived**, 60 of 60 |
| M17 Study Time not written | red |
| M18 ELLIPSE takes 2 or more points | red, 2 tests |
| M19 frame 0 accepted | red |
| M20 placement frame off by one | red |
| M21 NUM not rounded to 12 digits | red |
| M22 duplicate SOP Instance not refused | red |
| M23 Manufacturer not written | red, 3 tests |
| M24 KO evidence lists every source instance | red |
| M25 text comment dropped on load | red |
| M26 `writeKeyObject` back to COMPOSITE | red |
| page move clamp removed | red in the browser test |
| inspect Study UID tag wrong | red in `tests/preview.rs` |

## Defects

### D1, a loaded text comment is corrupted when it holds a non-ASCII character or a backslash

**Where**: `packages/core/src/tid1500.ts:699` (`decode`, the TEXT branch), reached from `readTid1500Groups`
**What**: the SR is written correctly, `(0040,A160) UT [a\b 肺]` under `ISO_IR 192` in dcmdump. dcmjs 0.52.2 then reads a nested string as Latin-1 and splits a UT value on its backslash, and `decode` keeps the first element. `"Läsion"` loads as `"LÃ¤sion"`, `"肺"` as `"èº"`, `"a\b"` as `"a"`. The next Save writes the corrupted text, so it becomes permanent.
**Why it is wrong**: PS3.3 C.12.1.1.2. Specific Character Set `ISO_IR 192` applies to every string in the data set, sequence items included. PS3.5 6.2 makes UT a single value in which a backslash is ordinary text.
**Evidence**: the scratch text harness, and the Chromium folder probe showing `"LÃ¤sion"` on the page. pydicom and dsrdump read the same file as `note é`.
**Scope and regression**: every text mark, and any TEXT item another writer's SR holds. Present on HEAD. Fix in the reader: when the data set's Specific Character Set is `ISO_IR 192`, re-decode each string read from the content tree as UTF-8, and join an array TextValue with `\`. Smallest regression: a `tid1500-groups.test.ts` case that round-trips `"Läsion 肺 a\b"` through `writeTid1500Groups` and `readTid1500Groups`.

### D2, deleting every mark of a study between saves brings the marks back

**Where**: `examples/folder-viewer/src/mark-documents.mjs:310`, `examples/folder-viewer/src/page.html:5163`, and `packages/core/src/tid1500.ts:577`
**What**: a study whose marks were all deleted has no groups, so `buildDocuments` writes no SR for it. When nothing is left at all, `saveMarks` stops at "There are no marks or key images to save." The earlier SR is not superseded, so reopening the folder with every download in it places the deleted marks again. The same holds for a key image unflagged when it was the last one.
**Why it is wrong**: lead amendment 3, "so a mark deleted between saves does not come back". PS3.16 TID 1500 rows 6, 10 and 12 allow an empty Imaging Measurements container ("any of them may be present but empty"), so the standard can express it.
**Evidence**: the delete-all harness. Save 1 writes an SR with one rect and a KO. Save 2, with the rect deleted and the session predecessor passed, writes `["ko"]`. Loading every file places `rect` again.
**Scope and regression**: present on HEAD. Fix: when the session holds a predecessor for a study that now has no saved marks, write an SR with an empty Imaging Measurements container naming the predecessors, which needs `writeTid1500Groups` to accept no groups when predecessors are given, and do not stop early in `saveMarks` in that case. TID 2010 row 8 to 10 needs at least one reference, so an emptied key image list cannot be written. The page and README must say so. Smallest regression: save one mark, delete it, save, load both, the mark is absent.

### D3, the documents copy a Study Time with its fraction cut off

**Where**: `examples/folder-viewer/src/page.html:5122` and `examples/folder-viewer/src/mark-documents.mjs:245`
**What**: `info.studyTime` is F-212's display text from `dicom_time_text`, which drops the fraction. `tests/preview.rs:5214` pins `"143012.123456"` to `"14:30:12"`. The SR and KO then state Study Time `143012` for a study whose images say `143012.123456`.
**Why it is wrong**: PS3.3 C.7.2.1. Study Time is an attribute of the study, and the plan copies it from the referenced images. A truncated copy is a different value for the same Study Instance UID. Lead amendment 8 named F-212's field as the source, so this is the amendment's wording followed literally.
**Evidence**: `corners.rs:60` and its pinned test cases. The synthetic folder image carried `101500.250000`.
**Scope and regression**: every saved document whose study has fractional seconds, common on CT. Fix: add a stored-value inspect field for Study Time and copy it verbatim. Smallest regression: a `buildDocuments` case with `101500.25` that expects `101500.25` in the SR.

### D4, the Part 10 file meta omits two Type 1 attributes

**Where**: `packages/core/src/tid1500.ts:475` (`part10`), used by `writeTid1500Groups` and `writeKeyObjects`
**What**: the file meta holds only Group Length, Media Storage SOP Class UID, Media Storage SOP Instance UID and Transfer Syntax UID. File Meta Information Version (0002,0001) and Implementation Class UID (0002,0012) are absent.
**Why it is wrong**: PS3.10 2026d Table 7.1-1 makes both Type 1.
**Evidence**: `dcmdump -E` and pydicom list four meta elements for both files.
**Scope and regression**: inherited from F-196's helper, which `git show f265d09a:packages/core/src/tid2010.ts` shows. F-215 now writes these files for other systems to read, which is AC19's purpose. Fix: write `FileMetaInformationVersion` `00H 01H` and a fixed Implementation Class UID, plus an Implementation Version Name. PS3.5 B.2 advises against a UUID-derived UID for an Implementation Class UID. Smallest regression: a writer test that reads `dict.meta` and requires both.

### D5, README says every SR file is read as a document

**Where**: `README.md:402`
**What**: "An SR or Key Object Selection file is read as a document and is not listed as a series." `DOCUMENT_CLASSES` holds four classes: Enhanced, Comprehensive and Comprehensive 3D SR, and Key Object Selection. Any other SR, such as X-Ray Radiation Dose SR `1.2.840.10008.5.1.4.1.1.88.67` or Basic Text SR `...88.11`, still goes to `groupSeries` and is listed as a series that fails to preview.
**Why it is wrong**: a false sentence in README is a defect under `/microscope` section 3.
**Evidence**: `page.html` `DOCUMENT_CLASSES` and the `readFolder` filter. A dose SR is in most CT exports.
**Scope and regression**: fix the sentence to name the four classes, or take every SR and KO storage class out of the series list. The second fix matches what the sentence says and the plan's problem statement.

### D6, a comment claims a float32 step where the code allows a thousandth

**Where**: `examples/folder-viewer/src/mark-documents.mjs:154`
**What**: "Whether `value` is within a float32 step of `want`". The test is `Math.abs(value - want) <= 1e-3 * Math.max(1, Math.abs(want))`, about 16000 float32 steps at 1000.
**Why it is wrong**: a false claim in prose. Behaviourally the loose bound accepts another writer's off-centre ellipse as axis-aligned.
**Evidence**: reading the line against IEEE 754 binary32 spacing.
**Scope and regression**: fix the bound to a few float32 ulps of `want`, or state the bound the code uses.

## Smells

### S1, the freeform grip clamp is reached by no test

**Where**: `examples/folder-viewer/src/page.html:4774` (implementer adaptation 8)
**What**: the change is right per lead amendment 6, but no browser test and no Rust test touches a grip (`grep grip` over `tests/` is empty). Reverting it stays green.
**Fix**: extend `tests/browser/marks.mjs` to drag a freeform handle past an edge, save, load and find it.

### S2, the KO newest rule's `Ocelli` filter is unguarded

**Where**: `examples/folder-viewer/src/mark-documents.mjs:409` and `:450`
**What**: mutant M16, which applies the newest rule to every writer's KO, survived 60 of 60. A foreign KO hidden behind a newer Ocelli one would go unnoticed.
**Fix**: a `placeDocuments` case with two Ocelli KOs and one foreign KO for the same study, expecting the newer Ocelli one and the foreign one.

### S3, both Key Object writers write IMAGE for any referenced class

**Where**: `packages/core/src/tid2010.ts:136` and `:293` (implementer adaptation 4)
**What**: `KeyObjectRequest.referencedClass` and `KeyReference.sopClass` take any UID. A caller flagging an SR or an encapsulated PDF gets an IMAGE item, which TID 2010 row 10 says is COMPOSITE. The folder viewer and the React example pass image classes only, so nothing is wrong today.
**Fix**: say in `docs/lld/key-object.md`, `use.html` and the doc comment that the reference must be an image, or write COMPOSITE for a class that is not an image storage class.

### S4, document UIDs exceed the 128-bit range of a UUID-derived UID

**Where**: `packages/core/src/tid1500.ts:464` and `:593`, `packages/core/src/tid2010.ts:270`
**What**: dcmjs 0.52.2 `DicomMetaDictionary.uid()` writes `2.25.` and 39 random decimal digits from `Math.random`, the first from 1 to 9. About two thirds exceed 2^128. The harness SR's SOP Instance UID was `2.25.961433696081574785265225509313210034060`, above 2^128 = 340282366920938463463374607431768211456. Such a value is not a UUID, which is what the `2.25` arc holds (PS3.5 B.2, ISO/IEC 9834-8). Unconfirmed whether a validator flags it, since dciodvfy was not available.
**Fix**: the page's `newUid`, 128 bits from `crypto.getRandomValues`, already does it right. Use the same in the package.

## Nitpicks

- N1, adaptation 5. One fixed `2.25` Device Observer UID makes every installation the same device. PS3.5 B.2 notes a UUID-derived UID is usually not appropriate for a UID fixed at design time. Acceptable as chosen.
- N2. The length unit is written `(mm, UCUM, "mm")`. CID 7460 gives the meaning `millimeter`.
- N3. A length is a TID 1410 group around an open two-point POLYLINE "Image Region". TID 1500 row 9's description puts linear distance under TID 1501. This is the operator's Q5 answer, noted for AC19.
- N4. `README.md:271` is longer than the paragraph's wrap.
- N5. `use.html` and `more.html` do not say that `writeKeyObjects` returns nothing for a reference missing from `source.evidence`.
- N6. A loaded foreign SR whose groups were all unread is still named in Predecessor Documents Sequence on the next save. PS3.3 C.17.2 defines that sequence as documents whose content was included. This follows amendment 3's wording.

## Verified clean

**AC20, PS3.3, PS3.6, PS3.10, PS3.5 and PS3.16 edition 2026d, clause by clause.**
- Table C.18.6-1, Graphic Data. The plan's origin text matches, and the clause sits in the table, not C.18.6.1.1, as amendment 8 already says. The plan leaves out "The values must be within the range 0\0 to Columns\Rows", which amendment 6 relies on. The writer, the clamp and the `<=` load check agree with it.
- C.18.6.1.2. POINT, POLYLINE closure, CIRCLE and ELLIPSE (major axis endpoints, then minor) match in substance. Only the wording differs.
- C.18.4. Referenced Frame Number is 1C, counted from 1, written only for a multi-frame object. Verified in the dump: frame index 2 of 5 is `(0008,1160) IS [3]`.
- PS3.6. (0070,0022) FL 2-n, (0040,A30A) DS, (0008,1160) IS and (0040,A160) UT. The dump shows each VR.
- C.17.1 and C.17.6.1. Modality, Series Instance UID and Series Number are Type 1, and Referenced Performed Procedure Step Sequence is Type 2. All are written.
- C.17.2. Instance Number, Completion Flag, Verification Flag, Content Date and Content Time are Type 1. Performed Procedure Code Sequence is Type 2 and written empty. Current Requested Procedure Evidence is 1C, required because the tree references instances, and lists exactly those. Predecessor Documents is 1C and present when named. Verifying Observer is 1C and not required for `UNVERIFIED`. Identical Documents and Pertinent Other Evidence do not apply.
- C.17.6.2. Instance Number, Content Date and Content Time are Type 1. Current Requested Procedure Evidence is Type 1 with "No other instances", guarded by M24. Identical Documents is 1C and not required for one study. There is no Predecessor Documents Sequence, so amendment 4 is right.
- A.35.13 and A.35.4. Patient, General Study, General Equipment (Manufacturer Type 2) and SOP Common are present. Every value type and relationship written is in Tables A.35.13-2 and A.35.4-2.
- TID 1500. Row 1, CID 7021 holds `126000`. Row 3, TID 1001, is M and written. Rows 2 and 4 are U. Row 6, `126010`, is present. Row 7 is TID 1410.
- TID 1410. Rows 1, 2, 3 and 5 (`125007`, `112039` TEXT, `112040` UIDREF, `111030` SCOORD that is not MULTIPOINT) are written. Row 6 SELECTED FROM IMAGE is M, written with the optional concept `121112`. Row 11 is TID 1419 row 5: `410668003` is in CID 7470, inside 7469, inside 218, and `mm` is in CID 7460, inside 7181. Row 13 TEXT is `$QualType`, which TID 1500 row 7 leaves unbound, so `121106` is allowed.
- TID 1001, 1002 and 1004. Observer context is required because no Author Observer Sequence is written. The items are `121005` = `121007` (CID 270), `121012` UIDREF M and `121013` TEXT U, in order. Procedure and subject context are inherited from the modules. TID 1003 does not apply.
- TID 2010. Row 1 `113000` is in CID 7010. Row 8 IMAGE, with no purpose of reference. Every evidence instance is referenced in row 8.
- Codes against Annex D and the CIDs: `125007`, `112039`, `112040`, `111030`, `121106`, `121112`, `126000`, `126010`, `121005`, `121007`, `121012`, `121013`, `113000` and SCT `410668003`. Each meaning matches.

**Arithmetic.** Rect closure, the ellipse axes with min and max box corners (absolute widths, amendment 8), the `>=` tie to horizontal, contour closure, the FL `fround` on write and its write-back after save (AC6), the NUM from the `/measure` reply under `pixel-spacing` and `imager-spacing` only, with `detector` and px refused (amendment 1, `calibration.rs` names), `toPrecision(12)` within DS's 16 characters (dump `DS [85.4400374532]`), frame `n` to page frame `n - 1`, and the move clamp, which keeps lower bound at most 0 and upper bound at least 0 because every creation path already clamps or refuses outside points. `fround` is monotone and Columns and Rows are exact in binary32, so a clamped point never rounds outside. No Rust `as` cast was added. The one TS assertion, `graphic as GraphicType`, follows a set membership check.

**Load placement.** Placement by SOP Instance UID and frame, the six reasons (each red under mutation), a frameless multi-frame key reference marking every frame, predecessor skip and Tracking UID dedupe (M3, M15), and the generation checks in `readDocuments`.

**Cornerstone and dcmjs.** dcmjs `flattenPoints` pushes `point[0] || point.x`, so a 0 column becomes `undefined`. This is in 0.52.2 (`node_modules/dcmjs/build/dcmjs.es.js:16790`) and in the oracle's 0.52.0 (`tools/oracle/node_modules/dcmjs/build/dcmjs.es.js:16693`), which confirms the AC5 claim. The oracle install holds `@cornerstonejs/tools` 5.8.2 and no `@cornerstonejs/adapters`, so the no-hydration claim (Q5) has no installed source to check against, and the story makes no hydration claim.

**Implementer adaptations.** 1, names: `mark-documents.mjs` meets amendment 8's stem rule. 2, API: additive, `index.test.ts` PUBLISHED updated, `dicomUid`, `documentModules` and `part10` stay off the index. 3, the evidence refusal: right for C.17.6.2's "no other instances", guarded by M12. 4, IMAGE in `writeKeyObject`: right for images per amendment 5, see S3. 5, the fixed Device Observer UID: see N1. 6, per-sentence pins: match amendment 2, and every kept and changed sentence is pinned. 7, the F-213 AC12 change: only the literal `TID 1500` is removed before the `1500` search, which is narrow. 8, the grip clamp: right, see S1. 9, tooling: the folder viewer tsconfig, `.d.mts`, eslint globals for the browser test and the gate list, with tsc and lint green. 10, CHANGELOG: `/complete-feature` step 4 owns it, so leaving it is right and it is owed at completion.

**No patient data.** Every fixture and the browser test use `Synthetic^Phantom` and `SYNTH-` IDs. No binary file is in the diff. The scratch folder image is synthetic.

**User docs.** README step 5 and 8, `use.html` chapters 4 and 14 and `more.html` `#documents` match the tree, apart from D5 and N5. The retired sentences are gone and the F-211 kinds keep "not saved".

## Second readers

- **dsrdump 3.7.0**: exit 0 for both. "W: The VR checker does not support this Specific Character Set: ISO_IR 192" on both is a DCMTK limitation, not a file fault. "W: Check for template constraints not yet supported" on the KO is also a DCMTK limitation. The tree prints as intended, with every relationship, the NUM in mm and the frame `(CTe image,,3)`.
- **dcmdump -E**: exit 0, no warning. It shows D4.
- **pydicom 3.0.2**: reads both, text correct, which places D1 in the reader.
- **dciodvfy**: not installed and not run. Owed.

## Limitations and handoff

- AC18 in real Chrome by the operator is still owed. Headless Chromium now covers both entry points: Load marks, through the gate, and folder open, through this pass's probe.
- AC19 is open. No second viewer drew the regions. dsrdump checks structure only.
- dciodvfy should run on the remediated files.
- CHANGELOG is owed at `/complete-feature`.
- Not exercised: Save on a real multi-frame object in the page, a blocked second download, ultrasound and manual calibration (refused by `/measure`), and oversized documents in the page.
- A scratch probe was written untracked at `examples/folder-viewer/tests/browser/f215-p1-probe.mjs` for one run and removed. No tracked file was edited apart from this review.
- Next: implementation remediates D1 to D6 and S1 to S4, then a fresh independent pass 2 reviews the changed material.

F-215 pass 1: 6 defects, 4 smells, 6 nitpicks.
