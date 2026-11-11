# F-215 review, pass 7

**Reviewed**: 3572053e..58dc144b on `work/f-215-xai`, tip 58dc144b, one commit ("F-215, review pass 6 remediation"). The worktree was clean apart from the untracked `.venv` and `tools/oracle/out`, which are not part of the story.
**Author / reviewer**: a separate Claude implementer agent wrote 58dc144b. This pass is an independent Claude reviewer that wrote none of the reviewed material and none of passes 1 to 6. Both are Claude, so independence is per participant, not per model.
**Result**: 1 defect, 0 smells, 2 nitpicks

## Acceptance challenged

- **The session-wide set, every read order, at module level.** A scratch probe drives `placeDocuments` and `notePartialReports` the way `readDocuments` in `page.html` does, with the page's own `sessionReplaced`, `sessionPartialReports` and remembered predecessors, then saves with `buildDocuments` and reopens every file read plus the saved files in one fresh read. Documents: P, a rectangle on an image and a length on `ct`. Q, P's length alone, naming P. R, P's length and a new length, naming Q. X, a second partial report nothing names. Two folders, `ct` and the rectangle's image (P whole) or `ct` alone (P in part). Every ordered partition of {P, Q}, {P, Q, R}, {P, Q, R, X}, {P, R} and {Q, R}, each with every shown mark deleted before the save and with every shown mark kept. 388 cases. The expectations are derived from the pass 6 ruling, not from the code: a report is skipped when any read so far, this one included, named it. `kept` is the reports placed in part that nothing read in the session named. The reopen shows the kept reports' placeable marks, plus the shown marks when they were kept. All 388 agree on the tip. The same probe on 3572053e fails 370.
- **The chain.** In every order where R is read, Q is never placed once R has been read, and P is never placed once Q or R has been read, because a skipped Q's predecessors still join the set. Held in all 388 cases. My two chain mutants are red only in the probe (N1).
- **Status line against reopen, when the replacing document is not in the reopened folder.** One page load opens folder A, the whole study with Q, then folder B, the whole study with P and no Q. Folder B shows 0 marks and says P is replaced. The save says its file "replaces the earlier one". Folder B with the new file then shows P's two marks, the rectangle Q's author deleted among them. On 3572053e the same steps show P's two marks in folder B, save them, and reopen two. D1 below.
- **A report nothing named.** `sessionReplaced` gains UIDs only from `placed.named`, the Predecessor Documents Sequence of an SR this page read. A Key Object Selection's references never enter it, and a file `readDocument` cannot read names nothing. UIDs are globally unique, PS3.5 chapter 9, so a report in another study cannot share a named UID. The only way the set hides a report is a name read in this page load, which is D1's case when the namer is absent from the folder later saved from.
- **A foreign SR naming an Ocelli report.** `readTid1500Groups` reads Predecessor Documents Sequence with no Manufacturer test, so a foreign TID 1500 SR naming P is the same path as Q and skips P in any order. PS3.3 Table C.17-2, current edition read now, says the sequence holds "References to SOP Instances (e.g., prior or preliminary reports) whose content has been wholly or partially included in this document with or without modification." The standard does not call P obsolete, and a foreign SR that included part of P hides the rest of P on this page. That is the skip-on-load rule the operator chose for open question 3, now applied across reads as ruled, and not counted here. Handed to the lead under Limitations.
- **The new `observeReverse` page step.** Green on the tip through `bin/ocelli.sh gate viewer-browser`. The tip's `marks.mjs` run against 3572053e's page goes red with two problems: Load marks with P after Q said "1 item not in this folder was not placed. 1 mark was already shown." and the save said "1 report placed only in part is kept". A page mutant that drops `sessionReplaced` from `notePartialReports` goes red with the same two problems.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `npx tsc --build`, `npx eslint examples/folder-viewer packages/core/src`, `python3 scripts/prose_check.py`, en-dash and em-dash grep of the diff | 58dc144b worktree | now | 0, 0, 0, no match |
| `npx vitest run examples/folder-viewer` plus tid1500-groups, tid2010, sr | 58dc144b worktree, core built | now | 0, 87 of 87 |
| `bin/ocelli.sh gate viewer-browser`, `OCELLI_PORT=0`, scratch `CARGO_TARGET_DIR`, after `pgrep` showed only idle Playwright MCP servers | 58dc144b worktree, fresh release build | now | 0, ALL GREEN. reverse: loaded "Replaced by a later document: p.dcm.", save with no kept sentence, reopened 0 |
| tip `marks.mjs` against 3572053e's page | `git archive 3572053e` copy, its own release binary, embedded page checked to lack `sessionReplaced` | now | 1, the two reverse problems above |
| page mutant, `notePartialReports` called without `sessionReplaced` | `git archive 58dc144b` copy, rebuilt | now | 1, the same two problems |
| order probe, 388 cases | tip copy and base copy | now | tip 0 failures, base 370 |
| folder A with Q, then folder B with P, save, reopen B with the new file, through the page | tip copy and base copy, each binary rebuilt and checked | now | tip: B shows 0, reopen 2. Base: B shows 2, reopen 2 |
| truncation sample, a four-length report, every third cut from byte 140 | tip copy | now | 1843 cuts, 0 non-null, 0 throws |
| PS3.3 Table C.17-2, Predecessor Documents Sequence | dicom.nema.org current | now | as quoted above |
| mutation catalogue, 4 on the new tests and 3 of my own | tip copy, restored and checked equal | now | 6 red, 1 survived. Table below |

A note for the next reviewer of a page change: a `git archive` copy keeps commit-time mtimes, and cargo with a shared target directory then reports the other tree's viewer binary as fresh. My first base run used the tip's binary and passed. Every page run above was after touching the copy's `page.html` and `serve.rs` and checking the binary's strings.

| Mutant | Result |
|---|---|
| `placeDocuments` ignores `replacedBefore` | red, both new tests and the probe |
| `notePartialReports` does not join this read's names | red, all three D1 tests and the probe |
| removal by this read's names only, not the session set | survived everything, the probe included. Equivalent, N2 |
| a match drops the whole study, pass 6 N1 | red, "does not count a partial report a later document replaces" and the probe |
| own, chain: `named` leaves out the predecessors of a skipped report | red in the probe only (N1) |
| own, chain: within one read, a skipped report's predecessors do not skip | red in the probe only (N1) |
| own, this read's names join the set after the removal | red, the pass 5 test and the probe |

## Defects

### D1, a report skipped as replaced is not named by the next save, so its marks come back without the replacing document

**Where**: `examples/folder-viewer/src/mark-documents.mjs:472` to `:475` skips the report without returning it in `predecessors`, and `examples/folder-viewer/src/page.html:5279` and `:5300` pass the session set and remember only `placed.predecessors`.
**What**: once any read of the page load names P, every later read skips P. Nothing records P for the next save, so the save names only the documents that were placed. When the folder saved from holds P and not the document naming it, the page shows none of P's marks and the save says its file "replaces the earlier one", and then that folder with the new file shows P's marks, including a mark deleted in Q.
**Why it is wrong**: the page hid P because Q replaces it, PS3.3 C.17.2, and then wrote a document that does not replace it. The status line and the reopen disagree, which is the property pass 5 D1 and pass 6 D1 were about. README's "Deleting every mark of a study is saved too, as an SR with no marks that replaces the earlier ones" reads as covering what the page hid.
**Evidence**: the folder A then folder B page probe. Tip: folder B "Read 1 document: 0 marks and 0 key images placed. Replaced by a later document: report-p.dcm.", save "Saved 0 marks and 0 key images in 1 file. 1 study has no marks left, so its file holds no marks and replaces the earlier one.", reopen of folder B with the new file "Read 2 documents: 2 marks". On 3572053e the same steps place P's two marks in folder B and reopen the same two. Module probe, P partial: Q then P, or P then Q, gives `kept` 0 and P's length back on a reopen of P with the new file. The P-then-Q partial variant is from pass 5, the whole variant is new in this range.
**Scope and regression**: present on the tip. It needs two reads in one page load and a later folder without the replacing document, for example two folders of one study, or Q loaded with Load marks from outside the folder. The gate's reverse case reopens with Q present, so it does not see this. Fix: when `placeDocuments` skips an SR as replaced, return its reference with the predecessors, `empty: false`, so `rememberDocument` records it and the next save names it too. Its content is included through the document that names it, which Table C.17-2 permits. For the partial forward order the page also needs the reference of a report it removes from `sessionPartialReports`, for example by returning `reference` with each `partial` row. Regression: Q read, then P read whole, every mark deleted, save, then a read of P and the saved file alone expects 0 marks. It is red on the tip.

## Smells

None.

## Nitpicks

- N1. No committed test holds the chain. With R naming Q and Q naming P, reading R, then Q, then P must skip P, which needs a skipped Q's predecessors to join the set. Both my chain mutants pass the suite and fail only my probe. Add R to `replacedPair` and read R, Q and P in three calls, expecting `second.replaced` and `third.replaced` and no marks.
- N2. In `notePartialReports`, removal over the session set is equivalent to removal over this read's names: a report added in this read is never in the set, because `placeDocuments` skipped it. The mutant survives every test and the 388 cases. The comment "One pass removes every replaced report, added just now or kept earlier" names a case that cannot arise. Not wrong behaviour.

## Verified clean

| Pass 6 | State | How |
|---|---|---|
| D1 | **closed** for placement and `kept` in every order, **reopened in a new form** as D1 above | 388 order cases at module level, the gate's reverse step, the base page red |
| D2 | closed | the comment is the one sentence ruled. 1843 truncations return null, none throw |
| D3 | closed | README's sentence covers every reason `placeDocuments` sets `whole` false: unread groups, a group that does not map to a mark, a target refused for missing, duplicate or frameless, and outside. It no longer reads as a full list. "most often" is the lead's ruled wording, not a measured rate |
| N1 | closed | the committed test holds a second partial report, red under the study-drop mutant |
| concern outside the range, P read whole after Q placed again | closed for placement | the new whole test and the probe. The same change opens D1 |

The `.d.mts` declares both optional parameters as the module has them, and `named` is "a report of this read", as `namedHere` is. `observeReverse`'s header comment matches its steps and the file's 0-based slice numbering. `use.html` and `more.html` are unchanged and make no claim this range touches. The gate is green on the tip.

## Limitations and handoff

- Page probes ran in Playwright's Chromium on the synthetic CT. Real Chrome, AC18 and AC19 stay with the operator at sprint close, as ruled. AC20 was answered before implementation and is not re-run.
- For the lead, not counted: Table C.17-2 says a predecessor's content may be only partially included. The skip-on-load rule hides all of P when a foreign SR named it after including part of it, and the session set now does that across reads. Ocelli's own saves name only reports placed whole, so the rule is exact between Ocelli files.
- Outside this range, not counted: an SR this viewer cannot read, such as a TID 2000 report, that names P does not skip P.
- The whole-P reverse order is held at module level only. The gate's reverse case is the partial order.
- All probes and mutants ran in `git archive` copies with linked node_modules and their own `CARGO_TARGET_DIR`. Scratch was removed with Python. No tracked file was edited. This review is left uncommitted.
- Next: implementation remediates D1, then a fresh independent pass 8.

F-215 pass 7: 1 defect, 0 smells, 2 nitpicks.
