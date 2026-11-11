# F-215 review, pass 5

**Reviewed**: 2c75bcc0..e1add52a on `work/f-215-xai`, tip e1add52a, one commit ("F-215, review pass 4 remediation"). The worktree was clean apart from the untracked `.venv` and `tools/oracle/out`, which are not part of the story.
**Author / reviewer**: a separate Claude implementer agent wrote e1add52a. This pass is an independent Claude reviewer that wrote none of the reviewed material and none of passes 1 to 4. Both are Claude, so independence is per participant, not per model.
**Result**: 2 defects, 0 smells, 3 nitpicks

## Acceptance challenged

- **Pass 4 D1, ISO_IR 166.** The ISO-IR 166 registration was read page by page from itscj.ipsj.or.jp/ir/166.pdf. It is a "96-character graphic character set", "A set of 88 Thai characters", G1 by ESC 2/13 5/4. Position 2/0 is NO-BREAK SPACE. Positions 5/11 to 5/14 read "This position shall not be used", and the code chart cross-hatches 5/11 to 5/14 and 7/12 to 7/15. 96 positions less those 8 is 88, so A0 is one of the 88. PS3.3 Table C.12-2 gives ISO_IR 166 as ISO-IR 166, 88 characters, G1, TIS 620-2533. The tip reads `A0 41` as `" A"` and refuses each of DB, DC, DD, DE, FC, FD, FE and FF.
- **Pass 4 D2, the wider retry.** A deflated ISO_IR 203 file now reads with every group unread rather than as no document. Truncations and bit flips were run on five variants of one four-group package SR: ISO_IR 192, no Specific Character Set, deflated with no set, deflated ISO_IR 203 and deflated ISO_IR 192. About 410 cut points per variant from byte 140 to the end, and 1500 seeded single-bit flips past the preamble, each at the tip and at 2c75bcc0. **Every truncation of every variant returns null at both commits.** Bit flips are another matter (D2 below).
- **Pass 4 D3, through the page.** `observeKept` passed in `bin/ocelli.sh gate viewer-browser` on the tip. The tip's `marks.mjs` run against 2c75bcc0's page and module went red with "which does not say the partly placed report is kept". The binary was rebuilt from touched sources, and its status line lacked the new sentence, so the old page was served.
- **D3, adjacent case through the page.** P holds a rectangle on slice 1 and a length on slice 6. A second page opened the whole study with P, deleted the rectangle and saved Q, which names P. A third page opened slices 2 to 12 with P, then Load marks with Q, deleted the one shown length and pressed Save marks. It said "1 report placed only in part is kept, so the marks it holds come back." A fresh page on the whole study with P, Q and the new file showed 0 marks: "Replaced by a later document: report-q.dcm, report-p.dcm." The sentence is false (D1 below).
- **Several, one later whole, another folder.** At module level, two partial reports in one study give `kept` 2, which the page words "2 reports placed only in part are kept, so the marks they hold come back." A report placed whole later is in `sessionDocuments`, so it is not counted. A study not among the folder's images is not counted.
- **Saving behaviour.** `kept` is read only by the status line. The new `keeping` term in the early return of `saveMarks` changes only the message. With no entries and no predecessor for a study in the folder, `buildDocuments` writes no document, so "There are no marks or key images to save." becomes "Nothing was saved. 1 report placed only in part is kept, ...", and no file is written either way. `sessionPartialReports` grows by one UID per distinct report read and placed in part, the same bound as `sessionDocuments`, and is filtered by the open folder's studies at every save. It is never cleared on a folder change, which is right, because the report still exists on disk.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `npx vitest run` tid1500-groups, tid2010, secondary-capture, sr, tid1500, mark-documents | e1add52a worktree, core built | now | 0, 97 of 97 |
| `npx tsc --build`, `npx eslint examples/folder-viewer packages/core/src`, `python3 scripts/prose_check.py` | e1add52a | now | 0, 0, 0. No em-dash or en-dash in the diff |
| `bin/ocelli.sh gate viewer-browser`, `OCELLI_PORT=0`, scratch `CARGO_TARGET_DIR`, after `pgrep` showed only idle Playwright MCP servers | e1add52a | now | 0, ALL GREEN. kept: save "... 1 report placed only in part is kept, so the marks it holds come back.", reopened 2 |
| tip `marks.mjs` in a copy of 2c75bcc0, sources touched, binary rebuilt | `git archive 2c75bcc0` | now | 1. kept: save lacks the sentence, reopened 2 |
| superseded partial report, Playwright through the page | copy of e1add52a, release binary | now | as under Acceptance challenged, D1 |
| same, module level, `placeDocuments` twice then `buildDocuments` | copy of e1add52a | now | `kept` 1, reopen 0 marks, `replaced` [p, q] |
| truncation and flip probe, 5 variants | copies of e1add52a and 2c75bcc0 | now | table below |
| ISO-IR 166 registration | itscj.ipsj.or.jp/ir/166.pdf, pages 1 to 4 read as images | now | as above |
| PS3.3 Table C.12-2 | dicom.nema.org current | now | ISO_IR 166: ISO-IR 166, 88, G1, TIS 620-2533 |
| mutation catalogue, 7 on the new tests and 3 of my own | copy of e1add52a, each restored | now | 8 red, 2 survived. Table below |

| Variant | Truncations, tip and base | Flips returning fewer than 4 groups, tip | Of those, through the retry |
|---|---|---|---|
| ISO_IR 192 | all null | 6 | 0 |
| no set | all null | 13 | 0 |
| deflated, no set | all null | 8 | 0 |
| deflated ISO_IR 203 | all null | 12 | **12**. At 2c75bcc0 all 1500 flips were null or whole |
| deflated ISO_IR 192 | all null | 10 | 0 |

| Mutant | Result |
|---|---|
| D1, A0 refused again | red |
| D1, refusal stops at DD | red |
| D2, retry only when the walk found a set | red, the deflated ISO_IR 203 test |
| S1, O2: the default repertoire whenever the walk fails | red, the deflated ISO_IR 100 test |
| D3, `placeDocuments` records no partial report | red |
| D3, a report placed whole later still counted | red |
| D3, a study outside the folder counted | red |
| own, every partial report also when placed whole | red |
| own, one per study rather than one per report | **survived** (N1) |
| own, `kept` set to 0 on a failed build | survived, and correct to: the failed branch returns before `kept` is read |

## Defects

### D1, the status line says a partial report's marks come back when a document the page read has replaced it

**Where**: `examples/folder-viewer/src/page.html:5297` to `:5301`, `examples/folder-viewer/src/mark-documents.mjs:329` to `:334`, and `README.md:396` to `:398`
**What**: `sessionPartialReports` keeps a report once it is placed in part, and `buildDocuments` drops it only when it is itself a predecessor. When a later read places a report Q that names P as its predecessor, P stays counted. The save says "1 report placed only in part is kept, so the marks it holds come back.", and on reopen P is replaced by Q and its marks do not come back. README's "An SR placed only in part ... is never replaced, so its marks come back" is false in the same case.
**Why it is wrong**: the pass 4 D3 ruling asks the status line and README to say what happens. PS3.3 C.17.2 makes Q's Predecessor Documents Sequence replace P, which `placeDocuments` itself honours at `mark-documents.mjs:447`.
**Evidence**: the page probe under Acceptance challenged, reproduced at module level: `kept` 1, reopen 0 marks, `replaced` ["p.dcm", "q.dcm"].
**Scope and regression**: new in this range. It needs two reads in one session, the folder and then Load marks, or two folder opens. Fix: `placeDocuments` returns the UIDs it found named as predecessors, the `named` set, and the page removes those from every set in `sessionPartialReports`. README: "is not replaced by a save from this page". Regression: in `mark-documents.test.ts`, P placed in part, then Q naming P placed in a second `placeDocuments` call, expect `kept` 0. It is red on the tip.

### D2, the `parsed` comment says a malformed file still fails, and a damaged deflated file in a set dcmjs refuses now reads as a partial report

**Where**: `packages/core/src/tid1500.ts:1049` to `:1050`
**What**: "A file truncated or malformed elsewhere still fails, as the identity and template checks below find nothing to read." True for every truncation tried. False for damage: 12 of 1500 single-bit flips of a deflated ISO_IR 203 report return a document with fewer than its 4 groups, 3 of them 0 groups and 0 unread, which `placeDocuments` treats as whole and empty. At 2c75bcc0 those files returned null. Files the strict parse accepts show the same class at both commits, so the behaviour is dcmjs's and not new in kind.
**Why it is wrong**: a false claim in new prose, `/microscope` section 3. The reviewed task asked whether damaged files still return no document. They do not.
**Scope and regression**: rare, only damaged deflated files in a set dcmjs refuses. Fix, prose only: "A truncated file still returns nothing, because the partial read then lacks the template or the identity. A file damaged inside its content can read as fewer groups, with or without the retry." No behaviour change is asked.

## Smells

None.

## Nitpicks

- N1. Nothing tests `kept` above 1, so counting one per study rather than one per report survives. The code is right today, 2 for two reports. Add a second partial report to the D3 test.
- N2. `docs/lld/public-api.md:298` says under ISO_IR 166 "only DB to DE and FC to FF ... make a string unreadable", and the same paragraph says a C1 byte does too, which `tid1500.ts:826` applies to ISO_IR 166. The paragraph also states the 166 refusal twice. Drop "only" and the earlier copy.
- N3. README says a report is placed only in part "because the folder lacks an image one of its marks is on". A report with an unread group or a mark outside its image is kept too, and the status line counts it the same way. "for example because" would cover it.

## Verified clean

| Pass 4 | State | How |
|---|---|---|
| D1 | closed | registration and Table C.12-2 read now. A0 reads as U+00A0, all eight refused. Both mutants red |
| D2 | closed in behaviour | deflated ISO_IR 203 reads 0 groups and 1 unread. Every truncation null at tip and base. The comment is D2 above |
| D3 | closed for the case ruled | gate green on the tip, red on 2c75bcc0's page. The adjacent superseded case is D1 above |
| S1 | closed | the deflated ISO_IR 100 ASCII test is red under O2 |
| N1 | closed | the loop covers all eight bytes, the DD mutant is red |
| N2 | carried, ruled to stay at module level | |

The status sentence for one partial report, for several and for one later placed whole is right. A study outside the open folder is not counted. `use.html`'s new sentence on files the reader cannot walk matches the tests. `more.html` is unchanged and makes no claim this range touches. The `.d.mts` matches the module's new `kept`, `partial` and `partialReports`.

## Limitations and handoff

- The superseded case was driven through the page with Playwright's Chromium on the synthetic CT. Real Chrome and AC19 stay with the operator at sprint close, as ruled.
- The ISO_IR 166 cases ran in Node only this pass. Pass 4 showed Node and Chromium agree on this reader.
- All probes and mutants ran in `git archive` copies with linked node_modules and their own `CARGO_TARGET_DIR`. Scratch was removed with Python. No tracked file was edited. This review is left uncommitted.
- Next: implementation remediates D1 and D2, then a fresh independent pass 6.

F-215 pass 5: 2 defects, 0 smells, 3 nitpicks.
