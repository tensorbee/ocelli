# F-215 review, pass 3

**Reviewed**: 4448f37a..2b286ac8 on `work/f-215-xai`, tip 2b286ac8, one commit ("F-215, review pass 2 remediation"). The worktree was clean apart from the untracked `.venv` and `tools/oracle/out`, which are not part of the story.
**Author / reviewer**: a separate Claude implementer agent wrote 2b286ac8. This pass is an independent Claude reviewer that wrote none of the reviewed material and none of passes 1 and 2. Both are Claude, so independence is per participant, not per model.
**Result**: 3 defects, 1 smell, 4 nitpicks

## Acceptance challenged

- **D1 ruling, the character sets.** 26 files were written with pydicom 3.0.2 from an SR the package wrote: every single-byte set of PS3.3 Table C.12-2 except ISO_IR 13, the three sets of Table C.12-5, no Specific Character Set, `ISO_IR 6`, `ISO 2022 IR 100`, implicit VR little endian, defined-length sequences and items, a (0008,0000) group length, Explicit VR Big Endian and Deflated Explicit VR Little Endian. Each was read by `readTid1500Groups` in Node and in headless Chromium through the page's `/lib/ocelli-core/index.js`. Both hosts agree on every file. Every set the reader claims returns its text unchanged except ISO_IR 203, which returns no document at all (D1). Three files that 4448f37a read correctly now have every group unread (D2).
- **D1, through the page.** A text mark written by the page was recoded by pydicom into ISO_IR 100 (`Läsion é`), ISO_IR 144 (`Да Москва`), ISO_IR 192 (`肺 Läsion a\b`) and no Specific Character Set (`plain text`). Each was loaded with Load marks, saved, the page reloaded, and the new download loaded again. All four texts came back unchanged both times.
- **D5 ruling, partial folder, through the page.** A rectangle was saved from a folder holding only series 2 of a synthetic two-series CT study. A fresh page opened series 1 only and loaded that SR: "0 marks ... 1 item not in this folder was not placed." Save marks said "There are no marks or key images to save." and downloaded nothing. The whole study then loaded the mark back, and a fresh page on the whole study loaded it back too. Closed.
- **D5, adjacent case.** In the same page, after the whole study placed that report whole, every mark was deleted and Save marks pressed. It said only "Nothing was saved.", downloaded nothing, and a fresh page loaded the deleted mark back (D3).
- **The ruling's reason, challenged.** The new test says an empty report beside a partial one "would hide the partial one's marks behind it". A probe built that empty report without the partial rule, naming only the report placed whole, then placed all three documents on the whole study. The partial report's mark came back and the deleted one did not. The rule is not needed for D5, and it is what breaks delete all (D3).

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `npm run build -w @ocelli/core`, then `npx vitest run` over tid1500-groups, tid2010, secondary-capture, sr, tid1500 and mark-documents tests | 2b286ac8 | now | 0, 86 of 86 |
| the file meta test alone, 30 runs | 2b286ac8 | now | 30 of 30 green |
| `npx tsc --build`, `npx eslint examples/folder-viewer packages/core/src`, `python3 scripts/prose_check.py` | 2b286ac8 | now | 0, 0, 0 |
| `bin/ocelli.sh gate viewer-browser`, `OCELLI_PORT=0`, scratch `CARGO_TARGET_DIR` | 2b286ac8, after `pgrep` showed no browser job | now | 0, ALL GREEN. "Saved 4 marks and 1 key image in 2 files.", the text mark back as `肺 Läsion a\b`, the unflag sentence shown |
| the same gate with `packages/core/dist` moved away, in a scratch copy with its own binary and target dir | copy of 2b286ac8 | now | 0, ALL GREEN, "Saving needs the library build..." and the marks kept |
| Node and Chromium reads of the 26 pydicom files | scratch files | now | 21 clean in both hosts. ISO_IR 203 null. Big endian, deflated, `ISO_IR 6` and `ISO 2022 IR 100` every group unread |
| the same 26 files through the 4448f37a reader, built in a scratch copy | `git archive 4448f37a packages/core/src` | now | big endian, deflated and `ISO_IR 6` read `plain` with 0 unread. Every non-ASCII single-byte and GB file garbled |
| `dcmjs.data.DicomMessage.readFile` on the ISO_IR 203 file | dcmjs 0.52.2 | now | throws "Unsupported character set: iso-ir-203" |
| windows-1252 inversion, all 256 bytes | Node `TextDecoder("latin1")`, Chromium `TextDecoder("latin1")` (encoding `windows-1252`), and the WHATWG table | now | 0 wrong, 256 distinct images in each |
| TextDecoder labels in Chromium | the 14 labels the reader uses | now | all exist. `iso-8859-9` resolves to `windows-1254`, which equals ISO 8859-9 from A0 to FF, so the C1 refusal makes it exact |
| PS3.3 C.12.1.1.2 Tables C.12-2 and C.12-5, PS3.5 6.1 | dicom.nema.org, current, 2026d | now | the twelve single-byte defined terms and three multi-byte ones match the code. ISO_IR 13's G0 is ISO-IR 14. PS3.5 6.1.2 says "characters from the C1 set shall not be used" |
| PS3.4 Table B.5-1 parsed, 164 rows, and every listed IOD's module table in PS3.3 | dicom.nema.org | now | see Verified clean |
| page probes A to D, playwright 1.62.1 Chromium | release binary, scratch target | now | as under Acceptance challenged |
| `dsrdump -Ec -Ee +U8`, `dcmdump -E +U8`, DCMTK 3.7.0 | the package SR, a KO of Ophthalmic Thickness Map, Height Map Segmentation, 12-lead ECG and a GSPS, and five SRs the page downloaded | now | exit 0 for every file, no dcmdump warning. dsrdump shows `contains WAVEFORM:=(TwelveLeadECGWaveformStorage,)` and warns only about its ISO_IR 192 VR checker and template checks |
| Mutation catalogue, 18 package mutants and 2 page mutants, scratch copy | copy of 2b286ac8 | now | 15 red, 5 survived. Table below |

| Mutant | Result |
|---|---|
| M1 the inverse of byte 0x82 broken | red, the browser-decoder test |
| M2 the raw (0008,0005) ignored, ISO_IR 192 assumed | red, 3 tests |
| M3 ISO_IR 144 removed | red |
| M4 the default repertoire accepts bytes above 7F | red |
| M5 UT rejoined without the backslash | red, 2 tests |
| M6 no `ignoreErrors` retry for an unsupported set | red |
| M7 D4, the study-in-folder check removed | red |
| M8 D5, a missing image does not make a report partial | red |
| M9 D5, `partialStudies` ignored | red |
| M10 a second empty report written after empty ones | red |
| M11 S1, the evidence guard forced true | red |
| M12 S2, WAVEFORM written COMPOSITE | red |
| M13 D6, Ophthalmic Thickness Map removed | red |
| M14 the secondary capture's own file meta restored | red |
| P1 page, D3, the unflag check removed from the early return | red in `tests/browser/marks.mjs` |
| X1 the C1 refusal removed | **survived** (S1) |
| X2 a multi-valued set decoded by its first value | **survived** (S1) |
| X3 implicit VR walked as explicit | **survived** (S1) |
| X4 ISO_IR 203 removed | **survived**, the entry is unreachable (D1) |
| P2 page, `placed.partial` never recorded | **survived** in the browser gate. The rule is covered only at module level (D3) |

## Defects

### D1, every ISO_IR 203 file is refused whole, although the reader says it decodes it

**Where**: `packages/core/src/tid1500.ts:1037`, with `parsed` at `:1023` and the entry at `:763`. The claim is at `docs/lld/public-api.md:298`, "ISO_IR 100 to 203".
**What**: dcmjs 0.52.2's `encodingMapping` has no `iso-ir-203`, so `readFile` throws "Unsupported character set". `parsed` retries with `ignoreErrors` only when `strings` is null, and for ISO_IR 203 it is not. The throw reaches `readTid1500Groups`, which returns null, so Load marks treats the file as no report at all. The `SINGLE_BYTE` entry never runs. Mutant X4 removes it and every test stays green.
**Why it is wrong**: PS3.3 Table C.12-2 defines ISO_IR 203, Latin alphabet No. 9. The LLD and the code comment both say it is decoded.
**Evidence**: the pydicom ISO_IR 203 file holding `€ Œ Ÿ` returns null in Node and in Chromium. dcmjs alone throws on it.
**Scope and regression**: any foreign SR in ISO_IR 203. Not worse than 4448f37a, which also returned null, but a false claim in this range. Fix: retry with `ignoreErrors` whenever the reader decodes the set itself, since the reader never uses dcmjs's decode of a content-tree string, or whenever dcmjs refuses for any set read from the bytes. Regression: a vitest case recoding to ISO_IR 203 that needs `€` back.

### D2, an ASCII report that loaded correctly now has every group unread when its transfer syntax is big endian or deflated

**Where**: `packages/core/src/tid1500.ts:849`, `characterSetOf`, and `stringsFor` treating its null as unsupported
**What**: the raw walk handles only Implicit and Explicit VR Little Endian. For any other transfer syntax it returns null, and every group is counted unread, even when the file has no Specific Character Set and only ASCII text. A Deflated Explicit VR Little Endian SR from another tool, or an Explicit VR Big Endian one, loaded at 4448f37a and now places no marks.
**Why it is wrong**: PS3.5 Annex A.5 and A.3 are standard transfer syntaxes, dcmjs reads both, and the default repertoire needs no decoding. The task's "a file that previously loaded correctly is not now worse" does not hold.
**Evidence**: `f215-p3-none-deflated.dcm` and `f215-p3-none-bigendian.dcm` read `{"text":"plain","unread":0}` through the 4448f37a reader and `{"text":null,"unread":1}` now, in Node and in Chromium.
**Scope and regression**: present on HEAD, a regression from 4448f37a. Fix: when the walk cannot run and dcmjs's dataset has no `SpecificCharacterSet`, the set is the default repertoire exactly, because dcmjs adds that element only when the file holds it. Walking big endian is also only a change of byte order. Regression: a vitest case on a deflated or big endian ASCII report expecting 0 unread.

### D3, deleting every mark of a study that once held a partly placed report is silently not saved

**Where**: `examples/folder-viewer/src/mark-documents.mjs:343`, `page.html:1153` and `:5294`, and the reason given at `examples/folder-viewer/src/mark-documents.test.ts:624`. The user-facing claim is `README.md:395` to `:396`.
**What**: once any read report of a study is not placed whole, the study joins `sessionPartialStudies` for the rest of the page load, and `buildDocuments` never writes an empty report for it. Two triggers:
1. Open one series, Load marks, then open the whole study and Load marks again, which places the report whole. Delete every mark and Save marks. The page says only "Nothing was saved." with no reason, and the next load brings the deleted marks back.
2. In a folder holding a report placed whole and a report placed in part, delete every mark and save. The same.

The test's reason for the rule, that an empty report "would hide the partial one's marks behind it", is false. The empty report names only reports placed whole, so the partial one is never superseded. The probe shows the partial report's mark coming back and the deleted one staying deleted without the rule.
**Why it is wrong**: lead amendment 3 and the pass 2 D2 ruling say a mark deleted between saves does not come back, and README says "Deleting every mark of a study is saved too". The bare "Nothing was saved." also breaks the D3 rule that the status line says why something is not saved. This comes from the second sentence of the pass 2 D5 ruling, "An empty report is written only for a study whose read reports were all fully placed". The first sentence, naming a predecessor only when every group was placed, already satisfies D5 on its own, as the D5 page probe shows.
**Evidence**: page probe D, `clearSave` "Nothing was saved." with no files, then `freshLoad` "Read 1 document: 1 mark ... placed." The module probe, with `wrote: 1` and `marks: ["2.25.930"]`.
**Scope and regression**: new in this range. Fix: drop the `partialStudies` gate and keep the predecessor rule. If the lead keeps the rule, the status line must say why nothing was saved, README must say so, and a study must leave the set once its reports are all placed whole. Regression: the mixed case in `mark-documents.test.ts` expecting an empty report that names only the report placed whole, then the partial report's mark back on the whole study.

## Smells

### S1, three new refusals and the implicit VR walk have no test

**Where**: `packages/core/src/tid1500.ts:804`, `:788` and `:848`
**What**: mutants X1, X2 and X3 survive. With X1 an ISO_IR 100 byte 0x85, which PS3.5 6.1.2 forbids, would be shown as U+0085. With X2, `\ISO 2022 IR 87`, a Japanese file using code extensions, is decoded as the default repertoire and its escape sequences shown as text. With X3 every implicit VR file has its set misread. Each guard is right today and nothing would notice its removal.
**Fix**: one vitest case each, an ISO_IR 100 comment holding 0x85 counted unread, a two-valued set counted unread, and an implicit VR ISO_IR 100 file read whole.

## Nitpicks

- N1. `README.md:275` is 84 columns. The ruling asked for 79.
- N2. A file whose Specific Character Set is `ISO_IR 6` read correctly at 4448f37a and is now unread. Table C.12-2 gives the default repertoire the Defined Term "none", so this matches the ruling, but other tools write the value and dcmjs maps it. Consider treating it as the default repertoire.
- N3. The comments at `tid1500.ts:748` and `:803` say every single-byte set has a G1 of 96 characters. Table C.12-2 gives ISO_IR 166 88. `windows-874` maps the bytes TIS 620-2533 leaves undefined, A0, DB to DE and FC to FF, to U+00A0 and to private use code points rather than refusing them.
- N4. `String.fromCharCode(...bytes)` at `tid1500.ts:807` throws "Maximum call stack size exceeded" at 200000 bytes in Node. An ISO_IR 100 text value that long would make the whole report unreadable. Build the string without spreading.

## Verified clean

Pass 2 findings replayed through the real caller:

| Pass 2 | State | How |
|---|---|---|
| D1 | closed, apart from D1, D2 and S1 above | Node and Chromium agree on all 26 files. The four sets written through the page round-trip unchanged. The browser gate's `肺 Läsion a\b` round-trip passes. M1, M2, M3 and M5 red |
| D2 | closed | 30 of 30 green. The test writes once |
| D3 | closed | the gate's unflag step shows the sentence. P1 red in the browser |
| D4 | closed | M7 red. A study not in `state.studies` is skipped |
| D5 | closed for the reported trigger, D3 above is new | page probe D. M8 and M9 red |
| D6 | closed | see below |
| S1 | closed | the empty report omits (0040,A375). M11 red |
| S2 | closed | see below. M12 red. dsrdump accepts the WAVEFORM item |
| N1 | **open**, N1 above | |
| N2 | closed | M10 red |
| N3 | closed | `key-object.md` names F-196 and F-215, dated 2026-11-08 |
| N4 | closed | `writeSecondaryCapture` goes through `part10`. M14 red |

**The raw (0008,0005) walk.** Explicit and implicit little endian, defined-length and undefined-length sequences and items, and a (0008,0000) group length all return the right set. `LONG_VRS` matches PS3.5 Table 7.1-1. An undefined length before (0008,0005) returns null, which is safe, and can occur only in group 0004.

**The label mapping.** Every `SINGLE_BYTE` and `MULTI_BYTE` key is a Defined Term of Tables C.12-2 and C.12-5, with the label for the matching ISO/IEC 8859 part, TIS 620 or the multi-byte set. Leaving out ISO_IR 13 is right, because its G0 is ISO-IR 14. Decoding ISO_IR 100 byte for byte is right, because WHATWG's `iso-8859-1` is windows-1252. GB18030 and GBK decode, including `乗`, whose second byte is 0x5C, the backslash dcmjs splits on. Code extensions, single-valued `ISO 2022 IR 87` and unknown values are counted unread, and the file's predecessors are still read.

**D6.** The 77 image entries hold no duplicate. All 69 B.5-1 entries are in Table B.5-1 under the commented name, differing only in "-" against ",". Each cited IOD module table was fetched from dicom.nema.org and holds the Image Pixel Module C.7.6.3, and Height Map Segmentation A.91.3 holds only the Floating Point Image Pixel Module C.7.6.24, which the stated rule names. That includes the three added classes, CT A.3.3, Enhanced CT A.38.1.3, MR A.4.3, Ultrasound A.6.4, PET A.21.3, RT Dose A.18.3, Parametric Map A.75.3, Segmentation A.51.3 and VL Whole Slide A.32.8.3. Label Map Segmentation's IOD is Segmentation, A.51.3, per B.5-1. The eight retired entries are not in B.5-1, as stated. Every other B.5-1 IOD module table was searched for the Image Pixel and Waveform modules. The only hits came from the shared `sect_A.86.html` page, and table by table the 14 second-generation RT IODs there hold neither. The rule reaches no class the list leaves out.

**S2.** The 16 waveform classes are exactly B.5-1's waveform storage rows. Each cited IOD module table, A.34.2.3 to A.34.18.3, holds the Waveform Module C.10.9. Waveform Presentation State and Waveform Acquisition Presentation State are rightly not listed.

**Second readers.** dsrdump and dcmdump -E accept the package SR, the four-class KO and five SRs the page downloaded.

**The page.** `bin/ocelli.sh gate viewer-browser` is ALL GREEN with the library build and, in a scratch copy, without it.

**User docs.** `use.html:882` is correct apart from ISO_IR 203 (D1). `public-api.md:298` is false on ISO_IR 203 (D1). `README.md:395` is false under D3. `more.html` is unchanged in this range and still true of the code. `key-object.md`'s rule sentence matches the code.

## Limitations and handoff

- AC18 in real Chrome and AC19 stay with the operator at sprint close, as ruled. dciodvfy is not installed.
- ISO_IR 13, the ISO 2022 multi-valued Japanese and Korean sets, and real vendor files were not exercised. The character set files are pydicom recodes of a package-written SR.
- The partial folder probe used a synthetic two-series CT written by pydicom in the scratchpad, opened with Playwright's directory upload.
- Reviewer process notes. Mutants ran only in a scratch copy, each restored after its run, and the copy's sources were diffed against the worktree afterwards with no difference. The copy used its own `CARGO_TARGET_DIR`. Scratch files were removed with Python. No tracked file was edited apart from this review, which is left uncommitted.
- Next: implementation remediates D1 to D3 and S1, and the lead rules on D3's conflict between the two D5 sentences and the D2 ruling. Then a fresh independent pass 4 reviews the changed material.

F-215 pass 3: 3 defects, 1 smell, 4 nitpicks.
