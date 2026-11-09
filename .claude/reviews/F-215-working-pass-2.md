# F-215 review, pass 2

**Reviewed**: c66a6343..dd11ed0d on `work/f-215-xai`, tip dd11ed0d, one commit ("F-215, review pass 1 remediation"). The worktree was clean apart from the untracked `.venv` and `tools/oracle/out`, which are not part of the story.
**Author / reviewer**: a separate Claude implementer agent wrote dd11ed0d. This pass is an independent Claude reviewer that wrote none of the reviewed material and none of pass 1. Both are Claude, so independence is per participant, not per model.
**Result**: 6 defects, 2 smells, 4 nitpicks

## Acceptance challenged

- **D1 ruling, through the real caller.** The new test round-trips text in Node only. The same reader was run in headless Chromium through the page's own `@ocelli/core` import, and on files whose Specific Character Set is ISO_IR 100, ISO_IR 101, ISO_IR 144, GB18030 and absent. Chromium returns `"肺"` as `"è‚º"`, and an ISO_IR 100 file that read correctly before this commit now reads `"Läsion"` as `"L�sion"` (D1).
- **D2 ruling, delete all.** Through the page in Chromium: draw a rectangle, save, Remove, save, reload, Load marks with both files. The second save says "Saved 0 marks and 0 key images in 1 file. 1 study has no marks left, so its file holds no marks and replaces the earlier one." and the load places 0 marks. Closed for this path.
- **D2 ruling, unflag.** Through the page: flag the key image, save, unflag, save. The page says "There are no marks or key images to save." and the earlier Key Object Selection still flags the image (D3 below). A key image study from an earlier folder is counted as unflagged in a folder that does not hold it (D4).
- **D2, adjacent case.** A folder that holds part of a study and its SR, whose marks are on images not in the folder, saved with no marks shown, writes an empty report that supersedes those marks (D5).
- **S2.** Two Ocelli Key Object Selections and one newer foreign one for the same study, through `placeDocuments`. The newer Ocelli one and the foreign one are shown, and the older Ocelli one is replaced. M16 goes red.
- **S3.** The 71 classes were diffed against PS3.4 2026d Table B.5-1 and PS3.6 Annex A fetched from dicom.nema.org (D6).

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `npm run build -w @ocelli/core`, then `npx vitest run` over tid1500-groups, tid2010, sr, tid1500, index and mark-documents tests | dd11ed0d | now | 0, 76 of 76 |
| the same file meta test alone, 30 runs | dd11ed0d | now | 5 of 30 red, `RangeError: offset is out of bounds` (D2) |
| `cargo test -p ocelli-folder-viewer --test preview` | dd11ed0d, fresh scratch `CARGO_TARGET_DIR` | now | 0, 119 of 119 |
| `npx tsc --build`, `npx eslint examples/folder-viewer packages/core/src`, `python3 scripts/prose_check.py` | dd11ed0d | now | 0, 0, 0 |
| `bin/ocelli.sh gate viewer-browser`, `OCELLI_PORT=0`, scratch `CARGO_TARGET_DIR` | dd11ed0d, after `pgrep` showed no browser job | now | 0, ALL GREEN, "Saved 3 marks and 1 key image in 2 files.", "Read 2 documents: 3 marks and 1 key image placed." |
| Node harness, `writeTid1500Groups` then `readTid1500Groups` | `"Läsion"`, `"肺"`, `"a\b"`, `"é"`, `"Ã©"`, `"plain"` | now | all six back unchanged in Node |
| pydicom 3.0.2 rewrites of that SR's comments under ISO_IR 100, 101, 144, GB18030 and no Specific Character Set, read by pydicom and by `readTid1500Groups` | scratch files | now | pydicom reads every variant correctly. `readTid1500Groups`: none clean, ISO_IR 100 `"L�sion"` and `"Ã©"` to `"é"`, the other three garbled |
| dcmjs 0.52.2 raw naturalize of the ISO_IR 100 file | `node_modules/dcmjs/build/dcmjs.es.js:12879` | now | `SpecificCharacterSet` comes back `"ISO_IR 192"`, the comment `"Läsion"` byte for byte |
| Chromium probe, page `import("@ocelli/core")`, same files | release binary from the worktree, scratch target | now | UTF-8 file: `"肺"` to `"è‚º"`, `"Ã©"` to `"ÃƒÂ©"`. `new TextDecoder("latin1")` maps 0x82 to U+201A |
| Chromium probe, delete all and unflag through the page | same binary | now | as under Acceptance challenged |
| `buildDocuments` probe, a key study not in the folder | dd11ed0d | now | `unflagged` 1 |
| `placeDocuments` then `buildDocuments` probe, a partial folder | dd11ed0d | now | first load 0 marks, `missing` 1, then an empty SR, then the whole folder 0 marks |
| `dsrdump -Ec -Ee +Pc +U8` and `dcmdump -E +U8`, DCMTK 3.7.0 | SR with six texts, empty SR, KO written by the package, and the empty SR and KO the page downloaded | now | exit 0 for every file, no dcmdump warning. dsrdump warns only that its VR checker does not support ISO_IR 192 and that template checks are not supported |
| Mutation catalogue, 20 package mutants and 2 page and Rust mutants, scratch copy | dd11ed0d copy | now | 20 red, 2 survived (X4, X6). Table below |

Mutations in a scratch copy, each restored after its run. M1 to M13 break what each new test names. X1 to X7 are this reviewer's own.

| Mutant | Result |
|---|---|
| M1 the ISO_IR 192 re-decode off | red, the character set test |
| M2 UT keeps its first split value | red |
| M3 an empty group list refused | red, 2 tests |
| M4 a cleared study not added | red |
| M5 unflagged not counted | red |
| M6 Study Time fraction dropped | red |
| M7 File Meta Information Version not written | red, 2 tests |
| M8 Implementation Class UID not written | red, 2 tests |
| M9, pass 1 M16, the newest rule for every writer | red, the S2 test |
| M10 always IMAGE | red, the S3 test |
| M11 dcmjs `uid()` | red, the S4 test |
| M12 unit meaning not written | red |
| M13 every report a predecessor | red, the N6 test |
| Rust, `studyTimeValue` read from Study Date | red, `inspect_reports_the_stored_study_time` |
| page, the grip clamp removed | red in `tests/browser/marks.mjs`, "the freeform handle left the image: column 299.99" |
| X1 a unit meaning kept when it equals the code | red, 2 tests |
| X2 a cleared study writes no SR | red |
| X3 evidence never required | red, 3 tests |
| X4 Current Requested Procedure Evidence Sequence written for the empty report | **survived** (S1) |
| X5 a foreign Key Object Selection hidden | red |
| X6 the UTF-8 re-decode for every character set | **survived**, which matches today's behaviour (D1) |
| X7 the C1 guard narrowed to 0x80 to 0x9F | red |

Several mutant runs also showed the file meta test red with nothing it guards changed. That is D2 and not a mutant effect.

## Defects

### D1, the text re-decode corrupts every non-UTF-8 character set and leaves CJK text corrupted in the browser

**Where**: `packages/core/src/tid1500.ts:879`, which picks the decoder, and `:718`, `FROM_UTF8`
**What**: two failures.
1. dcmjs 0.52.2 rewrites Specific Character Set to `ISO_IR 192` whenever the element is present (`dcmjs.es.js:12879`, "change SpecificCharacterSet to UTF-8"). So the reader's test is true for every file that names a character set, and Latin-1 bytes are re-decoded as UTF-8. An ISO_IR 100 file whose comment is `Läsion` loaded correctly before dd11ed0d and now loads as `L�sion`. ISO_IR 101, ISO_IR 144 and GB18030 come back garbled. Only a file with no Specific Character Set is clean.
2. In Chromium, dcmjs reads a nested string with `new TextDecoder("latin1")`, which the WHATWG Encoding standard maps to windows-1252. Bytes 0x80 to 0x9F become code points above U+00FF, such as 0x82 to U+201A. The `code > 0xff` guard then keeps the string as read. Every UTF-8 sequence holding a continuation byte in that range stays corrupted. `"肺"` (E8 82 BA) loads in the page as `"è‚º"`. `"Läsion"`, pass 1's example, survives only because 0xA4 is outside that range. The vitest case passes because Node's `latin1` decoder keeps bytes as they are.

The next Save writes the corrupted text, so it becomes permanent. `docs/lld/public-api.md:298` says the reader "decodes every string of the content tree as the file's Specific Character Set says, so under ISO_IR 192 a text holding any character or a backslash comes back whole". Both halves are false.
**Why it is wrong**: PS3.5 6.1 and PS3.3 C.12.1.1.2. The lead ruling for D1 says "decodes text as the file's Specific Character Set says".
**Evidence**: the pydicom variants and both harnesses in the table. Mutant X6, which re-decodes under every character set, survives because it is today's behaviour.
**Scope and regression**: every text mark on a page with a character above U+07FF and many below it. Also every foreign SR in a single-byte set other than ASCII. Present on HEAD, and the ISO_IR 100 part is a regression from c66a6343. Fix: read the file's own (0008,0005) before dcmjs rewrites it, for example from the element's raw bytes after the meta group. Recover the bytes of each nested string by inverting windows-1252 as well as Latin-1. All 256 byte values have a distinct image, so the inversion is exact. Then decode with the label of that character set. A set with code extensions, or one TextDecoder does not know, counts the group as unread rather than showing wrong text. Regressions: a browser step in `tests/browser/marks.mjs` that saves and loads a text mark holding `肺`, and a vitest case that reads an ISO_IR 100 file written by another tool and needs `Läsion` back.

### D2, the new file meta test fails about one run in six

**Where**: `packages/core/src/tid1500-groups.test.ts:458` and `:459`
**What**: `new ArrayBuffer(bytesOf([everyType()]).byteLength)` sizes the buffer from one write, then `set(bytesOf([everyType()]))` copies a second write. Each write makes new `newUid()` values, and a 128-bit value has 39 or fewer decimal digits, so the two lengths can differ. A longer second write throws `RangeError: offset is out of bounds`.
**Why it is wrong**: a test that fails at random is red on CI for no defect and teaches people to rerun past real failures.
**Evidence**: 5 of 30 runs of that one test red on dd11ed0d. It also went red in four mutant runs that did not touch what it guards.
**Scope and regression**: CI and `gate --floor`. Fix: write the bytes once, then copy them. No other test does a double write (grep for `ArrayBuffer(` over both test trees).

### D3, unflagging the last key image is not explained when the study has no marks

**Where**: `examples/folder-viewer/src/page.html:5174` to `:5177`, and `README.md:396` to `:399`
**What**: flag a key image, save, unflag it, save. `entries` is empty and `sessionDocuments` has no SR for the study, so `saveMarks` stops at "There are no marks or key images to save." before `buildDocuments` counts the unflag. On the next load the earlier Key Object Selection flags the image again, and the page never said why. This is the most likely way to reach the case.
**Why it is wrong**: the lead ruling for D2 says "the page and README say that unflagging the last key image of a study is not saved, and why". README says "and the status line says so", which is false on this path.
**Evidence**: the Chromium probe, "C save 2: There are no marks or key images to save." with `keyImages.size` 0.
**Scope and regression**: present on HEAD. Fix: count a session key study held in the folder as a reason to call `buildDocuments`, or check for unflags before the early return. Regression: extend `tests/browser/marks.mjs` to unflag and save, then require the unflag sentence.

### D4, a key image study from an earlier folder is reported as unflagged

**Where**: `examples/folder-viewer/src/mark-documents.mjs:327` to `:330`, fed by `page.html:5189`
**What**: `sessionKeyStudies` lasts the page load across folder opens, and the page passes all of it. `buildDocuments` counts every listed study that has no key image in this save, including studies that are not in the folder. Save a key image in folder A, open folder B, add a mark, save. The page says "1 study has no key image left. That is not saved..." about a study the user did not touch.
**Why it is wrong**: a false status sentence. The SR path checks `state.studies[uid]` for this reason, and the key image path does not.
**Evidence**: `buildDocuments` with `keyStudies: ["1.2.3.4.5"]` and only study `9.9.9` in `studies` and `entries` returns `unflagged: 1`.
**Scope and regression**: fix by counting only studies present in `state.studies`. Regression: that case, expecting 0.

### D5, an empty report supersedes marks that were read but never placed

**Where**: `examples/folder-viewer/src/mark-documents.mjs:318` to `:322` and `:497` to `:502`, and `page.html:5174`
**What**: open a folder holding one series of a study and its SR, whose marks are on another series. The marks are not placed ("missing"), but the SR is named in `placed.predecessors`, so it joins `sessionDocuments`. Save marks with nothing drawn. Before dd11ed0d this wrote nothing. Now it writes an empty SR that names the earlier one, and the status line says "1 study has no marks left". Loading the whole study later places none of the marks. The user deleted nothing.
**Why it is wrong**: lead amendment 3 supersedes so that "a mark deleted between saves does not come back". These marks were not deleted. PS3.3 C.17.2 defines Predecessor Documents Sequence as documents whose content "has been wholly or partially included in this document", and none of it was.
**Evidence**: the partial folder probe: 0 marks placed with `missing` 1, then an empty SR, then 0 marks on the whole folder with the first SR listed as replaced.
**Scope and regression**: new in this range for the no-marks save. The same mechanism already drops unplaced marks when the save holds other marks. That variant predates this range and is noted for the lead, not counted here. Fix: carry each unplaced group into the next SR verbatim, or name a report as a predecessor only when every group it holds was placed. Regression: the probe above, expecting the mark back.

### D6, the image class list leaves out three image IODs and claims to be complete

**Where**: `packages/core/src/tid2010.ts:82` to `:88` and `:90`, and `docs/lld/key-object.md:6`
**What**: the comment and the LLD say the list holds every B.5-1 "Image Storage" class, plus the classes "whose IOD holds image pixels without the word", plus the retired image classes. All 60 "Image Storage" rows of B.5-1 are present, and so are the five additions and the six retired classes, each checked by UID. Three B.5-1 classes whose IOD includes the General Image and Image Pixel modules are missing: Ophthalmic Thickness Map Storage `1.2.840.10008.5.1.4.1.1.81.1` (PS3.3 A.67.3), Corneal Topography Map Storage `...1.82.1` (A.73.3) and Ophthalmic Optical Coherence Tomography B-scan Volume Analysis Storage `...1.77.1.5.8` (A.84.3). A key image of one of those is written as COMPOSITE. Height Map Segmentation Storage `...1.66.8` was not checked. The retired Hardcopy Grayscale and Color Image Storage classes `1.2.840.10008.5.1.1.29` and `.30` are in PS3.6 and not in the list.
**Why it is wrong**: PS3.3 C.18.3. COMPOSITE references an object "that is not a DICOM Image or Waveform". The prose claims completeness.
**Evidence**: a script diff of the list against B.5-1 parsed from the fetched page. The three module tables were fetched from `sect_A.67.3`, `sect_A.73.3` and `sect_A.84.3`.
**Scope and regression**: fix by adding the three classes, checking 66.8, and stating the rule the list follows. Regression: one of the three in the S3 test, expecting IMAGE.

## Smells

### S1, nothing checks that the empty report omits Current Requested Procedure Evidence Sequence

**Where**: `packages/core/src/tid1500.ts:646` to `:650`
**What**: mutant X4 writes the sequence even when the evidence list is empty, and every test stays green. C.17.2 says "One or more Items shall be included in this Sequence", so dropping the guard would write an invalid empty-study item.
**Fix**: in the empty-report test, require `CurrentRequestedProcedureEvidenceSequence` to be absent.

### S2, a waveform reference is written as COMPOSITE

**Where**: `packages/core/src/tid2010.ts:164`, `itemType`
**What**: `writeKeyObjects` and `writeKeyObject` take any class. A 9.x waveform class becomes COMPOSITE, which C.18.3 excludes, and TID 2010 row 9 is WAVEFORM. The folder viewer never flags a waveform, so nothing is wrong today. The ruling said "COMPOSITE otherwise" and did not mention row 9.
**Fix**: write WAVEFORM for the waveform storage classes, or refuse them and say so in the doc comment.

## Nitpicks

- N1. `README.md:272` is 89 characters. Pass 1's N4 asked for a rewrap, and the long line only moved.
- N2. Each Save with no marks after a cleared report writes another empty report naming every earlier one.
- N3. `docs/lld/key-object.md` still says "F-IDs that contributed: F-196" and "Last updated: 2026-11-04", although F-215 rewrote two paragraphs.
- N4. `writeSecondaryCapture` builds its own file meta at `packages/core/src/secondary-capture.ts:176` and still lacks (0002,0001) and (0002,0012). The D4 ruling named "F-196's writer", which may mean only the key object writer. Left to the lead.

## Verified clean

Pass 1 findings replayed through the real caller:

| Pass 1 | State | How |
|---|---|---|
| D1 | **open** | D1 above |
| D2 | delete all closed, unflag **open** | page probe B closed. D3, D4 and D5 above |
| D3 | closed | Rust `studyTimeValue` test 119 of 119, red when read from Study Date. M6 red. `markImage` reads `studyTimeValue`. TM regex keeps `101500.25` |
| D4 | closed | dcmdump shows `(0002,0001) OB 00\01` and `(0002,0012)` on the package SR, the empty SR, the KO and both page downloads. The value is the constant `2.25.181940322218151432116404530960524932737` in every file, across separate runs and the page, so it is fixed, not new per file. Also writes `(0002,0013) OCELLI_0_1` |
| D5 | closed | README names the four classes, which match `DOCUMENT_CLASSES` at `page.html:1020`, and says a dose SR still shows as a series |
| D6 | closed | the comment matches `1e-3 * max(1, abs(want))` |
| S1 | closed | the browser test drags a grip to column 300, and the grip clamp mutant went red in the browser |
| S2 | closed | see the S2 answer below. M16 red |
| S3 | PDF case closed, list **open** | D6 above |
| S4 | closed, but see D2 | `newUid` takes 16 bytes from `crypto.getRandomValues`, so the value is below 2^128 with no leading zero and at most 44 characters. The two fixed UIDs, the Device Observer and Implementation Class, are 44 characters and below 2^128. No `DicomMetaDictionary.uid()` call is left in `packages/core/src` or the folder viewer. M11 red |
| N2 | closed | `(mm, UCUM, "millimeter")` written and read back, M12 red |
| N5 | closed | `use.html` and `more.html` both say a reference must be in `source.evidence` |
| N6 | closed as ruled | M13 red. D5 is a separate failure of the same list |

**S2 answer.** The code shows the newest Ocelli Key Object Selection, replaces older Ocelli ones, and always shows a foreign one, whatever its Content Date and Time. That is what amendment 4 says: it ranks only "Key Object Selections this viewer wrote", because C.17.6.2 gives no supersession link. It is also what a user needs, because a foreign Key Object Selection is another author's selection, and Ocelli has no basis for hiding it. The ruling's wording "requires Ocelli's to win" is met in the only sense the standard supports: a newer foreign document does not displace Ocelli's. Under M16 it does, and the test goes red. Hiding the foreign one would be wrong behaviour. The implementer is right that both must be shown. The lead should restate the ruling as "a newer foreign Key Object Selection does not hide Ocelli's, and both are shown".

**D2 against the standard.** PS3.16 TID 1500 row 6 is MC "IF Row 10 and Row 12 are absent", and rows 7 to 9 are U, so an empty Imaging Measurements container is valid. dsrdump shows the empty report as the root, the three TID 1001 device items and an empty `(126010)` container. Under C.17.2, Current Requested Procedure Evidence Sequence is required "if Instances are referenced in the Content Tree", or when the creator knows the procedure's acquired objects. The empty report references none and omits it, which matches the non-empty writer's existing reading. Predecessor Documents Sequence is present with one item.

**S3 sample.** CT `...1.1.2`, Enhanced CT `...1.2.1`, MR `...1.4`, Ultrasound Multi-frame `...1.3.1`, VL Whole Slide Microscopy `...1.77.1.6` and Enhanced Continuous RT Image `...1.481.24` match B.5-1 by name and UID. All 60 "Image Storage" rows are present. Every addition was checked: Enhanced US Volume, Parametric Map, Segmentation, Label Map Segmentation and RT Dose are in B.5-1 under those names. Ultrasound Multi-frame `...1.3`, Nuclear Medicine `...1.5`, Ultrasound `...1.6`, X-Ray Angiographic Bi-Plane `...1.12.3`, VL Image Trial `...1.77.1` and VL Multi-frame Trial `...1.77.2` are PS3.6 Annex A retired image storage classes with those UIDs. No UID is listed twice.

**Second readers.** dsrdump and dcmdump -E accept every file this pass wrote, including the empty report and the page's own downloads.

**Page.** `bin/ocelli.sh gate viewer-browser` ALL GREEN in headless Chromium.

**User docs.** README's new sentences on delete all, document classes and the item limit were read against the tree. Only the unflag clause is false (D3). The `use.html` and `more.html` changes are correct. `docs/lld/public-api.md:298` is false on character sets (D1).

## Limitations and handoff

- AC18 in real Chrome and AC19 stay with the operator at sprint close, as ruled. dciodvfy is not installed.
- The D1 browser result comes from a page-context call to the reader, not from a text mark typed in the page. The decode path is the same.
- Height Map Segmentation's IOD was not fetched.
- Reviewer process notes. `rm -f` was used once on two scratch-copy files inside the scratchpad, not shutil as instructed. The copy and the worktree first shared one scratch `CARGO_TARGET_DIR`, and cargo reused the copy's mutant test binary for one worktree run, which showed a red that was not the tree's. The worktree Rust result above is from a fresh target dir. The gate ran before any copy build. Two probe files under `examples/folder-viewer` were written untracked for this pass and removed with Python. No tracked file was edited apart from this review.
- Next: implementation remediates D1 to D6 and S1 and S2. Then a fresh independent pass 3 reviews the changed material.

F-215 pass 2: 6 defects, 2 smells, 4 nitpicks.
