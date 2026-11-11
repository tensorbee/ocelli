# F-215 review, pass 9

**Reviewed**: 2ef0039f..f8f7fb08 on `work/f-215-xai`. That is c5f2542a ("F-215, review pass 8 rulings"), the implementer's commit, and f8f7fb08, the lead's confirmation of two readings. The worktree was clean apart from the untracked `.venv` and `tools/oracle/out`, which are not part of the story.
**Author / reviewer**: a separate Claude implementer agent wrote c5f2542a. This pass is an independent Claude reviewer that wrote none of the reviewed material and none of passes 1 to 8. Both are Claude, so independence is per participant, not per model.
**Result**: 3 defects, 0 smells, 4 nitpicks

## Acceptance challenged

### Pass 8's rulings replayed through the real caller

| Ruling | State | How |
|---|---|---|
| A mark nobody deleted is never superseded | **closed** | The gate's `nodoc` step is green on the tip and red on 2ef0039f's page with one problem, "folder B with Q after a save from a folder with no document showed 0 marks, not Q's one". The base save said "Saved 0 marks and 0 key images in 1 file", the tip save "Nothing was saved. 1 study has no mark added, moved or deleted since its documents were read or last saved". vitest `never supersedes a mark nobody deleted` is red under M1 and O3. The model's I1b, below, is 0 on the tip and 69892 on the base |
| N3, a Save with no change writes nothing | **closed at module level, unguarded through the page** | vitest `writes nothing for a study with no change` is red under M2 and O4. Through the page the probe drew a mark, read T with Load marks and pressed Save twice: "Saved 3 marks and 0 key images in 1 file.", then "Nothing was saved. 1 study has no mark added, moved or deleted since its documents were read or last saved, so nothing is written for it." No gate step holds it, N1 |
| N1, the forward order page step | **closed** | The `forward` step is red under pass 8's PA through a rebuilt binary: "slices 1 to 11 with P and the new file showed 1 marks". It is green on 2ef0039f's page, which is right, because that page already remembered the removed partial reports. The step guards PA and is not a regression test for the base, so the expectation that it goes red on 2ef0039f does not hold and should not |
| N2, E then P whole | **closed** | vitest `names a report an empty SR replaced when the save writes nothing else` is red under M3, hidden never recorded, and M4, which is pass 8's O1 |
| Lead reading 1, a hidden report is named whatever the page shows, a saved SR is remembered with its marks | **closed** | vitest `names a report remembered as shown and later hidden as replaced` is red under M5, M6, M7 and O2. Through the page the `folders` step names P from folder B |
| Lead reading 2, hidden by an earlier read's name is a change | **closed as implemented, reading open** | O4, which also counts names of the same read, is red in vitest. The code takes "the same folder" as "the same read", N4 |

### The predecessor model, rebuilt from the rulings

A scratch driver calls `placeDocuments`, `rememberReport`, `notePartialReports` and `buildDocuments` with the wiring `readFolder`, `readDocuments`, `deleteSelected`, `addNote`, the move handlers and `saveMarks` have in `page.html`. Opening a folder clears the shown marks and keeps every session set. Load marks keeps both. A mark another writer saved without a Tracking Unique Identifier gets a new one on each read, as `page.html:5311` does.

Documents, one study: P, a rectangle on a second image and a length on `ct`. Q, P's length, naming P. R, P's length and a new length, naming Q. E, an SR of no marks naming P. X, an unrelated rectangle on `ct`. U, a length on `ct` with no Tracking Unique Identifier. Folders: the whole study and `ct` alone, each with any subset of the pool, plus Load marks of any non-empty subset. Pools: {P, Q}, {P, Q, R}, {P, E}, {Q, R}, {P, X}, {U, Q} and {U}.

Cases: two phases, each a read, an edit (none, delete all, delete one of the first three marks, add a mark, move a mark) and a save or not, then Save pressed twice. And three phases, the third opening the whole study or `ct` alone with no document, opening the whole study with one document, or Load marks with one document, with no edit, delete all or add, and Save pressed twice. Each case that saves is reopened three ways in the folder saved from: with every file read in the session, with the files read since that folder was opened, and with that folder's own files, each with every file the session saved.

Invariants, written from the rulings and the caller's brief, not from the code:

- **I1b**, a mark a fresh open of the view shows, that the user did not delete and that some report of the view holds which no read document named as replaced, is shown again with the saves added.
- **I1**, a mark shown at the last save comes back. Split by whether the last save wrote.
- **I2**, a mark the user deleted and that was not shown at the last save comes back only when a save said a report placed only in part is kept. Split by whether the deletion was in the folder saved from or an earlier one, and whether anything was written.
- **I3**, a Save of a study that had no add, move, delete or flag since its last writing save, and no report hidden by a name an earlier read gave, writes nothing and says so.

**280420 cases.**

| Invariant | Tip c5f2542a | Base 2ef0039f |
|---|---|---|
| I1b, never lost | **0** | 69892 |
| I1, last save wrote | 0 | 0 |
| I1, last save wrote nothing | 5838, N2 | 0 |
| I2, a save wrote, deleted in the folder saved from | **22513**, D1 and D2 | 0 |
| I2, a save wrote, deleted in an earlier folder | **28066**, D1 | 0 |
| I2, nothing written, same folder | 1980, D2 | 0 |
| I2, nothing written, earlier folder | 832, said "Nothing was saved." | 0 |
| I3, no change but a file written | **0** | 341946 saves |

Under the other reading of lead reading 2, a report hidden by a name any document read since the folder opened gave is no change, I3 has 3574 saves on the tip, N4.

The range closes both things pass 8 asked for. It also opens a class the base did not have: a deleted mark that comes back with no status sentence saying so. D1 and D2 are that class through the real page.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `npm run build`, then `npx vitest run examples/folder-viewer packages/core/src/tid1500-groups packages/core/src/tid2010 packages/core/src/sr` | f8f7fb08 worktree | now | 0, 94 of 94 |
| `npx tsc --build`, `npx eslint examples/folder-viewer packages/core/src`, `python3 scripts/prose_check.py`, en-dash and em-dash grep of the diff, README column check | f8f7fb08 worktree | now | 0, 0, 0, no match, the new README lines are 70 to 79 columns |
| `bin/ocelli.sh gate viewer-browser` | f8f7fb08 worktree, scratch `CARGO_TARGET_DIR`, after `pgrep -fl 'chromium\|playwright\|oracle'` showed only idle Playwright MCP servers | now | 0, ALL GREEN. `nodoc` reopened 1, `forward` reopened 0, `folders` save "Saved 0 marks and 0 key images in 1 file." |
| tip `marks.mjs` against 2ef0039f's page | `git archive` of f8f7fb08 with 2ef0039f's `page.html`, `mark-documents.mjs` and `serve.rs`, all three touched, its own target directory. The binary holds 0 `rememberReport` and 3 `rememberDocument(study` | now | 1, one problem, the `nodoc` one quoted above. `forward` green |
| A first base attempt shared the tip's target directory | same copy | now, discarded | cargo's dep-info pointed at the worktree's sources, so it reused the tip binary, 4 `rememberReport`. Rerun with its own target directory as above |
| module mutants, table below | `git archive` of f8f7fb08, linked `node_modules` | now | each run and restored, checked equal |
| page mutants, table below | the same copy, own target directory, the mutated file and `page.html`, `mark-documents.mjs`, `serve.rs` touched, the binary checked to hold the mutant's marker | now | each run and restored, checked equal |
| page probe, scenarios A to D below | the same copy at the tip, rebuilt, 0 mutant markers and 4 `rememberReport` in the binary | now | as quoted in D1 to D3 |
| the model | scratch copies of the tip and base `mark-documents.mjs` | now | as tabled above |

| Mutant | Result |
|---|---|
| M1, `settled` true for every report | red, `never supersedes a mark nobody deleted` |
| M2, no study is unchanged | red, `writes nothing for a study with no change since its documents were read` |
| M3, `hidden` never recorded | red, `names a report an empty SR replaced when the save writes nothing else` |
| M4, a skipped report returned with `empty: true`, pass 8's O1 | red, the same test |
| M5, `rememberReport` merges `replaced` with AND | red, `names a report remembered as shown and later hidden as replaced` |
| M6, `rememberReport` keeps the first entry unmerged | red, the same test |
| M7, a saved SR records no marks | red, the same test |
| own, O1, `settled` ignores `deleted` | **survived vitest**. Red through the page as PF, in the delete all, kept, reverse and forward steps. N3 |
| own, O2, `settled` ignores `replaced` | red, the reading 1 test |
| own, O3, `placeDocuments` records no marks for a whole report | red, `never supersedes a mark nobody deleted` |
| own, O4, `hidden` also counts this read's names | red, the N3 test |
| PA, the page drops `notePartialReports`' removed reports | red, `forward` |
| own, PD, a save does not clear `changedStudies` | **survived the gate**. N1 |
| own, PE, `deleteSelected` records no change | red, the reverse step: "no Q was saved: Nothing was saved. 1 study has no mark added, moved or deleted since its documents were read or last saved" |
| PF, O1 through the page | red, six problems |
| own, PG, the page sends no `changed` | **survived the gate**. N1 |

## Defects

### D1, a deleted mark comes back and the status line says nothing

**Where**: `examples/folder-viewer/src/mark-documents.mjs:327` and `:378`, `examples/folder-viewer/src/page.html:5257`
**What**: a save names a remembered report only when each of its marks is in the save or was deleted. When the page cannot show one of its marks, that report is not named, so every mark the user deleted from it comes back, and no status sentence says so. Probe A through the page: open folder A holding T, rectangles 2.25.701 and 2.25.702 on two images. Delete 701. Open folder B, the same images and no document. Draw a rectangle. Save says "Saved 1 mark and 0 key images in 1 file." Folder A with T and the saved file then reads "Read 2 documents: 3 marks and 0 key images placed.", and 701 is among them. A second shape in the model: save once with a mark the user drew, so the saved SR holds P's marks and the new one. Then open P again and delete every mark. The save says "1 study has no marks left, so its file holds no marks and replaces the earlier one." It does not replace the first saved SR, which still holds the drawn mark, so the reopen shows P's deleted marks again.
**Why it is wrong**: the brief's invariant says a deleted mark returns only when the status line said a report is kept. The only "kept" sentence covers reports placed in part. README says "Such an SR is not replaced by a save from this page, so its marks come back even after they were deleted, and the status line says so" for those reports alone. The "replaces the earlier one" sentence is false in the second shape.
**Evidence**: probe A, quoted. Model I2: 22513 cases where a save wrote and the mark was deleted in the folder saved from, 28066 where it was deleted in an earlier folder. 0 on 2ef0039f, which named every report and so lost undeleted marks instead, 69892 I1b cases.
**Scope and regression**: on HEAD only. New in this range, the price of the ruled rule. Smallest fix: `buildDocuments` counts, per study in the save, remembered reports that are not `replaced`, not `settled`, and hold a mark in `deleted`, and returns the count. The page says, for example, "1 earlier report holds marks this page does not show, so it is not replaced, and the marks deleted from it come back." The page says "replaces the earlier one" only when no such report remains, and README says the same. Regression: probe A's shape as a vitest case and as a page step, expecting the sentence, red on today's code. Carrying the hidden groups forward into the new SR would remove the cost rather than name it, which is a ruling for the lead.

### D2, an SR without Tracking Unique Identifiers can never be replaced once read twice

**Where**: `examples/folder-viewer/src/page.html:5311` and `:5331`
**What**: a mark read without a Tracking Unique Identifier gets `newUid()` on every read, and `rememberReport` unions each read's identifiers into the report's `marks`. After the same folder is opened twice, the report holds an identifier no shown mark carries, so `settled` is never true for it in that page load. Probe B through the page: open folder FU, the images and U, one rectangle with no Tracking Unique Identifier. Open FU again. Delete the rectangle, draw another, save: "Saved 1 mark and 0 key images in 1 file." FU with the saved file then reads "Read 2 documents: 2 marks and 0 key images placed." The deleted rectangle is back. Probe C, the same without drawing: the save says "Nothing was saved." and the reopen shows the rectangle.
**Why it is wrong**: this is the folder saved from, and every mark of U the page showed was deleted, so the ruled rule would name U. README says "Deleting every mark of a study is saved too, as an SR with no marks", false in probe C. The brief's invariant fails with no status sentence. Lead amendment 7 and TID 1501 accept another writer's group without the identifier, and the page comment at `page.html:5308` says it handles one.
**Evidence**: probes B and C, quoted. Model: 1980 nothing-written and part of the 22513 written same-folder I2 cases come from U.
**Scope and regression**: on HEAD only. The untracked append at `:5331` is new in this range. Fix: give an untracked mark a stable identifier per report and group for the page load, for example a session `Map` keyed by the report's SOP Instance UID and the group's index, with `placeDocuments` returning that index on each placed mark. Regression: a vitest case through `rememberReport` and `buildDocuments` reading U twice, deleting its mark and expecting the save to name U, and a page step of probe C expecting 0 marks on reopen. Both red on today's code.

### D3, the README's no-change sentence is false

**Where**: `README.md:398` to `:400`
**What**: "A Save of a study with no mark added, moved or deleted since its files were read or last saved writes nothing and says so." The page measures change since the page loaded or the study was last saved, and counts a report hidden by an earlier read's name. Probe D: open folder B, draw a rectangle, read T with Load marks, Save. Nothing was added, moved or deleted since T was read, and the save wrote "Saved 3 marks and 0 key images in 1 file." The gate's own `folders` step: folder B is opened and nothing is edited, and the save writes "Saved 0 marks and 0 key images in 1 file." A key image flag and a text edit also count as changes and the sentence names neither.
**Why it is wrong**: README is always in scope and a user reads this sentence as when a Save does nothing. The behaviour is the safer one: measured from the read, the mark drawn before Load marks would never be saved. The ruling's own wording, "since its documents were read and since its last save", has the same gap, which is the lead's to restate.
**Evidence**: probe D and the gate's `folders` line, quoted.
**Scope and regression**: prose only. Fix, for example: "A Save writes nothing for a study, and says so, when none of its marks or key images was added, moved, edited, deleted or flagged on this page since the page opened or the study was last saved, unless the page hid an SR that an earlier document names as replaced." The status sentence at `page.html:5237` is true whenever it is shown and may stay.

## Smells

None.

## Nitpicks

- **N1, the no-change rule has no page step.** PG, the page sends no `changed`, and PD, a save never clears `changedStudies`, both survive the gate, so Save pressed twice through `page.html` writes two files under PD and nothing fails. The N3 vitest case passes the same `changed` list to both `buildDocuments` calls and does not reach the page's clearing at `page.html:5248`. A page step that draws, saves, saves again and expects one download and the "no mark added" sentence would be red under both.
- **N2, Load marks of a replacing SR leaves the replaced report's dropped marks on the page, and a Save now keeps nothing of them.** Open the whole study with P, then Load marks with Q, which names P and holds P's length only. P's rectangle stays shown, which is not new. The page counts no change, so Save writes nothing, and the reopen with P and Q no longer shows the rectangle. Q's author dropped it, so no mark nobody deleted is lost, but the page showed a mark the folder will not. The model's 5838 I1 cases with nothing written are this. For the lead: removing a replaced report's marks when a later read names it would close it.
- **N3, the pass 7 test's new `deleted` line is not what makes it pass.** `mark-documents.test.ts:942` adds `deleted: ["2.25.972"]` under "Every mark shown was deleted by the user, which is Q's length." Its records come from `remember(record, one.study, one.reference)` with no `marks`, so `settled` holds through `!Array.isArray(one.marks)` and O1, deletions ignored, passes it. No vitest case reaches the deletion half of `settled`. PF shows the page holds it. Giving that test's records their `marks`, or one case where a deletion alone settles a report, would make O1 red in vitest.
- **N4, lead reading 2 has two readings.** "Hidden by a document in the same folder does not count as a change." The code reads "the same folder" as the same read. A folder holding Q, then Load marks with P, hides P by a name from the folder's own read, so the code counts a change and the save writes an SR identical in marks to Q. Read as "any document read since this folder was opened", that is no change, and the model has 3574 such saves. Harmless to marks. For the lead to pick one.

## Verified clean

- `rememberReport` merges as its comment says: empty only when both say so, the union of marks, replaced when either says so. M5 and M6 are red.
- `placeDocuments` records a report's identifiers for repeated marks too, so a report read after its marks are already shown is still settled by them. O3 is red.
- `buildDocuments` still considers a study whose remembered reports may not be named, so an unchanged study is counted and said. `kept` and `unflagged` skip an unchanged study, so a no-change save does not repeat them.
- `markStudyChanged` is called on add, grip move, whole move, text edit, arrow note edit, key toggle and delete, and a text cleared to nothing goes through `deleteSelected`. It resolves the study through `allImages()`, so a key the open folder lacks changes nothing.
- `saveMarks` clears the change only for a study a document was written for, so a failed or empty save leaves it set. A writer failure returns before any clearing.
- The `.d.mts` matches the module for `marks`, `unchanged`, `hidden`, `predecessors[].marks` and `replaced`, `deleted`, `changed` and `rememberReport`. tsc and eslint are clean.
- README's new lines are within 79 columns and pass the prose check. "A save replaces an earlier SR only when every mark it holds is in the save or was deleted on this page, so a mark nobody deleted is never lost, and it also replaces an SR the page did not show because a later SR replaces it" matches the model, where I1b is 0, and the `folders` step. `use.html` and `more.html` are unchanged and make no claim this range touches.
- The `nodoc` and `forward` steps build their folders from `/synthetic`, wait for the read before acting, and compare counts the brief derives. `forward`'s "placed only in part" check would catch a kept sentence.

## Limitations and handoff

- Page runs were in Playwright's Chromium on the synthetic CT. Real Chrome, AC18 and AC19 stay with the operator at sprint close, as ruled. AC20 was answered before implementation and is not re-run.
- The model covers one study, two image folders and the six documents above. Key Object Selections, a second study and multiframe targets were not in it. Moves are counted as changes and not checked geometrically.
- The probes and mutants ran in scratch copies prefixed `f215-p9-` with linked `node_modules` and their own `CARGO_TARGET_DIR`s. No tracked file was edited apart from this review, which is left uncommitted.
- Owed acceptance rows: AC18 and AC19 at sprint close. Nothing else in this range.
- Next: the implementer fixes D1 to D3, or the lead rules on D1's cost. The lead decides N1 to N4. A fresh pass reviews the remediation.

F-215 pass 9: 3 defects, 0 smells, 4 nitpicks.
