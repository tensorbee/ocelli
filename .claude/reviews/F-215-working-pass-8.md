# F-215 review, pass 8

**Reviewed**: b3794728..ebf5652c on `work/f-215-xai`, tip ebf5652c, one commit ("F-215, review pass 7 remediation"). The worktree was clean apart from the untracked `.venv` and `tools/oracle/out`, which are not part of the story.
**Author / reviewer**: a separate Claude implementer agent wrote ebf5652c. This pass is an independent Claude reviewer that wrote none of the reviewed material and none of passes 1 to 7. Both are Claude, so independence is per participant, not per model.
**Result**: 0 defects, 0 smells, 3 nitpicks

## Acceptance challenged

- **The predecessor naming model, rebuilt from the rulings.** A scratch driver calls `placeDocuments`, `notePartialReports` and `buildDocuments` the way `readFolder`, `readDocuments` and `saveMarks` in `page.html` do: opening a folder clears the shown marks and keeps the session state, Load marks keeps both, and every predecessor and every removed partial row is remembered. Documents: P, a rectangle on an image and a length on `ct`. Q, P's length, naming P. R, P's length and a new length, naming Q. X, a second report nothing names. E, an SR of no marks naming P. Two folders, the whole study and `ct` alone. The expectation is written from the rulings and not from the code: a report is skipped when any read so far named it, the names of a skipped report included. `kept` is the reports placed in part, never placed whole and named by nothing read. The reopen must show exactly the marks shown at the save plus the placeable marks of the kept reports, and a mark the user deleted may come back only when it belongs to a kept report.
  - Plain: every ordered partition of {P, Q}, {P, Q, R}, {P, Q, R, X}, {P, R}, {Q, R} and {P, E}, each read opening the whole study, opening `ct` alone, or with Load marks (the first read opens a folder), with every shown mark deleted or kept between reads, and every subset of the shown marks deleted before the save. **41784 cases.**
  - Extended: the same, plus a final step that opens either folder again holding no document or one document already read, which is the two-folder shape of pass 7 D1, with no mark or every mark deleted before the save. **401260 cases.**
  - Each case is saved and reopened three ways in the folder saved from: with every file, with every file read since that folder was opened, and with that folder's own files alone, which leaves out a report brought in with Load marks. A case where the save writes nothing is checked the same way.
  - **Tip: 0 failures of 41784 and 0 of 401260.** No case writes nothing, so every case was fully checked. The same model on b3794728 fails 11888 plain and 78573 extended, the first being pass 7 D1's folder A then folder B shape.
  - Every saved SR in every case names only UIDs of the documents above, each with that document's series and class.
- **`observeTwoFolders` through the page.** Green on the tip through `bin/ocelli.sh gate viewer-browser`, `OCELLI_PORT=0`, scratch `CARGO_TARGET_DIR`, after `pgrep -fl 'chromium|playwright|oracle'` showed only idle Playwright MCP servers. Folder B said "Replaced by a later document: report-p.dcm.", the save said "1 study has no marks left, so its file holds no marks and replaces the earlier one.", and the reopen said "Read 2 documents: 0 marks and 0 key images placed. Replaced by a later document: report-p.dcm." The tip's `marks.mjs` against b3794728's page goes red with one problem: "folder B with the new file showed 2 marks, which the page had hidden". That base copy was a `git archive`, with `page.html`, `serve.rs` and `mark-documents.mjs` touched, its own target directory, and the binary checked to hold the base page's `notePartialReports(sessionPartialReports, placed, sessionReplaced);` and no `skipped.push(document)`.
- **The saved SR, PS3.3 C.17.2.** A copy of `marks.mjs` that also copies P, Q and folder B's download out of its temporary folder, run against the tip page. `dcmdump` and `dsrdump` both exit 0 on the downloaded file. `dsrdump` reports "Predecessor Docs : 2". The Predecessor Documents Sequence holds one item, Study Instance UID 2.25.43000, with two Referenced Series Sequence items. One is Q's Series Instance UID 2.25.232790705320406993276297711983539752307 with Comprehensive 3D SR Storage and Q's SOP Instance UID 2.25.138596476615798525688538733220704639042. The other is P's series 2.25.100067509206395611250714547087342421602, the same class and P's instance 2.25.151330421371926422820715485872294986585. Each value equals the top-level (0008,0016), (0008,0018), (0020,000d) and (0020,000e) of `report-q.dcm` and `report-p.dcm`. That is the Hierarchical SOP Instance Reference Macro's shape, Table C.17-3, with real instances only. A module-built forward-order file, P in part then Q, names the same two.
- **Pass 7's foreign SR concern.** Not changed in this range and ruled as not changed. Not counted.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `npx tsc --build`, `npx eslint examples/folder-viewer packages/core/src`, `python3 scripts/prose_check.py`, en-dash and em-dash grep of the diff | ebf5652c worktree | now | 0, 0, 0, no match |
| `npx vitest run examples/folder-viewer packages/core/src/tid1500-groups packages/core/src/tid2010 packages/core/src/sr` | ebf5652c worktree | now | 0, 90 of 90 |
| `bin/ocelli.sh gate viewer-browser` | ebf5652c worktree, fresh release build in a scratch target | now | 0, ALL GREEN, folders as quoted above |
| tip `marks.mjs` against b3794728's page | `git archive b3794728` copy, own target, binary strings checked | now | 1, the one folders problem |
| page mutants, below | `git archive ebf5652c` copy, page touched and rebuilt each time, binary checked to hold the mutated text | now | table below |
| model, plain and extended | tip, base and mutant copies of `mark-documents.mjs` | now | tip 0 of 41784 and 0 of 401260 |
| `dcmdump`, `dsrdump` on the page's folder B download | Homebrew DCMTK on the host | now | 0, 0, as above |

| Mutant | Result |
|---|---|
| M1, `placeDocuments` leaves a skipped report out of `predecessors` | red, "names a report it skipped as replaced in the next save", the model (11516 plain). Not run through the page, where PC is the matching mutant |
| M2, `notePartialReports` returns no removed report | red, "names a partial report a later read replaced in the next save", the model (560 plain, 361 extended) |
| M3, a `partial` row carries no `reference` | red, that test and "counts a report placed only in part as kept when a delete all is saved", the model |
| M4, chain: a skipped report's names leave `named` | red, "skips every report of a chain whatever order it is read in", the model |
| M5, chain within one read: a skipped report's names do not skip | red, the chain test, the model (666 plain) |
| own, PA, the page ignores `notePartialReports`' removed reports | **survived the gate** and is outside vitest. Red in the model as M2. N1 |
| own, PB, the page passes a new set instead of `sessionReplaced` | red, the gate: folder B placed 2 marks and the reopen showed 2 |
| own, PC, the page remembers no predecessor of a read that replaced one | red, the gate: the reopen showed 2 |
| own, O1, a skipped report is returned with `empty: true` | survived vitest and the gate. Red in the model only, 8 plain cases. N2 |
| own, O3, a report skipped by a name of this same read is not returned | survived everything, the model included. Equivalent: a later read of P in any folder is skipped by the session set and returns it then |

Each module mutant ran in the scratch copy, restored and checked equal. Each page mutant ran in its own rebuilt binary and the page was restored and checked equal.

## Defects

None.

## Smells

None.

## Nitpicks

- **N1, the page's half of the forward order has no page step.** `page.html:5303` to `:5305` remembers every partial report `notePartialReports` removes. Replacing the loop with a bare call survives the gate, because `observeTwoFolders` reaches only the skipped path, and the vitest test at `mark-documents.test.ts:951` repeats the page's loop rather than running it. The model shows what that mutant does: `ct` alone with P, Load marks with Q, the length deleted, save, then `ct` alone with P and the new file shows the deleted length back and the status line said nothing is kept. Same class as pass 7 N1. A page step of those five actions, expecting 0 marks and no "kept" sentence, would be red under PA.
- **N2, `empty: false` on a skipped report is not held by any test.** `mark-documents.mjs:541`. It matters when the replacing report is itself an SR of no marks: folder A with E, then folder B with P alone. With `empty: true` the save writes nothing, says "Nothing was saved.", and folder B with P then shows P's two marks although the page hid them. The tip writes an SR of no marks naming E and P, and the reopen shows none. Only the model sees it. A vitest case reading E, then P whole, saving with no entries and expecting one document that names P would be red under O1.
- **N3, a Save with nothing changed now writes a file in one case.** A folder holding O and the SR of no marks E that replaces it shows no mark. On b3794728 three Saves write nothing. On the tip each of three Saves writes another SR of no marks, because O is now remembered as not empty, so `replaceable` at `mark-documents.mjs:352` holds. The status line is true and every reopen is right. It is the cost of the ruled `empty: false`, and it differs from the pass 2 ruling "A Save with nothing to save, and only empty reports read, writes nothing" only because O, read and not shown, is not empty. For the lead. Module level only, not run through the page.

## Verified clean

| Pass 7 | State | How |
|---|---|---|
| D1 | **closed** | Through the page: the gate's folders step green on the tip and red on b3794728's page, with the ruled two-folder path. The ruled regression, Q then P whole, every mark deleted, save, P and the saved file alone show 0, is `mark-documents.test.ts:921` and red under M1. The forward order, P in part then Q, closed at module level, the test at `:951` red under M2 and M3, and in the model's folder-only reopen. The adjacent case, folder A with Q then folder B with no document, names Q in the save on both the tip and b3794728, so it is not new in this range |
| N1 | **closed** | `mark-documents.test.ts:968` reads R, Q and P in three calls and in one, red under both chain mutants |
| N2 | **closed** | the comment drops the case that cannot arise and says why: `placeDocuments` skips every report in `replacedBefore` and this read's names, which are the names `notePartialReports` adds first. The removal over the session set stays equivalent to removal over this read's names |

The `.d.mts` declares `partial` with `reference` and `notePartialReports` returning `{ study, reference }[]`, as the module has them. `saveMarks` sends `partialReports` as the keys of the new inner map, which is the shape `buildDocuments` reads. The README sentence "and those it did not show because a later SR replaces them" matches the observed save and reopen, and is wrapped within 79 columns. `use.html` and `more.html` are unchanged and make no claim this range touches. `docs/lld` names none of the changed functions.

## Limitations and handoff

- Page runs were in Playwright's Chromium on the synthetic CT. Real Chrome, AC18 and AC19 stay with the operator at sprint close, as ruled. AC20 was answered before implementation and is not re-run.
- For the lead, outside the range and not counted: the session-wide predecessor list of amendment 3 lets a save from folder B name a report only folder A held. Folder A with Q, then folder B with no document, then Save writes an SR of no marks naming Q, so Q's length goes although nobody deleted it, and the status line says the study "has no marks left". Same on b3794728.
- The model covers one study, two folders and the five documents above. Key Object Selections, a second study and multiframe targets were not in it.
- N3 was observed at module level only.
- All probes and mutants ran in scratch copies prefixed `f215-p8-` with linked `node_modules` and their own `CARGO_TARGET_DIR`. Scratch was removed with Python. No tracked file was edited. This review is left uncommitted.
- Next: the lead decides on the three nitpicks. On this pass's terms F-215 has no blocking finding left in this range.

F-215 pass 8: 0 defects, 0 smells, 3 nitpicks.
