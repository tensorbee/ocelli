# F-215 review, pass 10

**Reviewed**: 561cb730..57995f45 on `work/f-215-xai`, one commit, 57995f45 ("F-215, review pass 9 remediation"). The worktree was clean apart from the untracked `.venv` and `tools/oracle/out`, which are not part of the story.
**Author / reviewer**: a separate Claude implementer agent wrote 57995f45. This pass is an independent Claude reviewer that wrote none of the reviewed material and none of passes 1 to 9. Both are Claude, so independence is per participant, not per model.
**Result**: 3 defects, 0 smells, 4 nitpicks

## Acceptance challenged

### Pass 9's findings replayed through the real caller

| Finding | State | How |
|---|---|---|
| D1, a deleted mark comes back and the status line says nothing | **open** | Pass 9's trigger is closed: the gate's `held` step says the held sentence, and it is red under PM1 and V1. Two adjacent shapes are not. With no mark drawn the save says only "Nothing was saved.", new D1. A report that a read already named as replaced is counted as held, and so is a report whose deleted mark was read again and saved, new D2 and D3 |
| D2, an SR without Tracking Unique Identifiers can never be replaced once read twice | **closed** | Through the page, probe 3: U and U2, two untracked reports each with one group at index 0, read in one folder got two different identifiers. U read in two folders got the same identifier both times. The gate's `untracked` step is red under PM2, and vitest under V2. The model's U, {U, U2} and {U, Q} pools have 0 failures |
| D3, the README's no-change sentence | **closed for README**, wording nitpick N4 | README says the rule as the code measures it, below |
| N1, the no-change rule has no page step | **closed** | the `twice` step is red under PM3, pass 9's PD, and under PM4, pass 9's PG |
| N3, no vitest case reaches the deletion half of `settled` | **closed** | `settles a report by a deletion alone` is red under V3, pass 9's O1 |

### D3, README against the status line and the code

`changedStudies` gains a study on an add, a move, a delete, a text edit, a key image toggle and a report hidden by an earlier read's name, and loses it only when a save writes a document for that study. No read clears it. README:401 to :404, "since the page loaded or since its last save", is that measure, with "its last save" read as the last save that wrote the study. The status sentence at `page.html:5240`, "since its documents were read or last saved", names a different baseline. It is true whenever it is shown, because no change since the page loaded implies no change since any later read, so a user is not told anything false. The two do not agree in wording, and the comments at `mark-documents.mjs:331`, `mark-documents.d.mts:60` and `:115`, `page.html:1161` and the plan's pass 8 ruling at `F-215-design.md:427` keep the read baseline, though the D3 ruling says that ruling's wording is corrected. N4.

### The predecessor model, rebuilt from the rulings

A scratch driver calls `placeDocuments`, `stableTracking`, `rememberReport`, `notePartialReports` and `buildDocuments` with the wiring of `readFolder`, `readDocuments`, `deleteSelected`, the add and move handlers and `saveMarks` in `page.html`, including the early "There are no marks" return and the order of the status remarks. Opening a folder clears the shown marks and keeps every session set. Load marks keeps both. The base variant replays 561cb730's page, a new identifier for an untracked mark on every read and no held sentence. Marks are identified by geometry, so a mark keeps its identity across saves whatever its Tracking Unique Identifier.

Documents, one study, two images, `ct` and `other`. P, a rectangle on each. Q, P's `ct` rectangle, naming P. R, Q's rectangle and a new one, naming Q. E, an SR of no marks naming P. X, an unrelated rectangle on `ct`. U and U2, one rectangle each on `ct` with no Tracking Unique Identifier, both group 0. T, pass 9's probe A report, a rectangle on each image. Pools {P, Q}, {P, Q, R}, {P, E}, {Q, R}, {P, X}, {U, Q}, {U}, {U, U2}, {T} and {T, X}.

Cases: two phases, each a read (the whole study or `ct` alone with any subset of the pool, or in the second phase Load marks of a non-empty subset), an edit (none, delete all, delete the first or the second shown mark, add, move inside the image) and a save or not, then Save pressed twice. And three phases over {T}, {P} and {U}, the third opening the whole study or `ct` alone with no document, the whole study with the pool, or Load marks of the pool, with no edit, delete all or add. Each case that saves is reopened in the last folder three ways, with every file read in the session, with the files read since that folder opened, and with that folder's own files, each with every file the session saved.

Invariants, from the rulings and the caller's brief:

- **I1b, never lost.** A mark of a pool document in the reopened view, whose report no document read in the session names as replaced, and that the user never deleted, is shown.
- **I1.** A mark shown at the last save is shown, split by whether that save wrote.
- **I2.** A deleted mark not shown at the last save comes back only if a status line after the deletion said the kept or the held sentence.
- **I3.** A Save with no add, move or delete since the last writing save, and no report hidden by an earlier read's name, writes nothing and says "Nothing was saved." or "There are no marks".
- **I4.** A save that says "replaces the earlier one" and no kept or held sentence is followed by no deleted mark on a reopen with every file.
- **I5.** A save that says the held sentence is followed by a deleted mark on a reopen of the whole study with every file. This one is added in this pass, because the held sentence is new.

**216 576 cases**, each run on both variants.

| Invariant | Tip 57995f45 | Base 561cb730 |
|---|---|---|
| I1b, never lost, every file | **80**, D3 | 32 |
| I1, last save wrote | 0 | 0 |
| I1, last save wrote nothing, own files | 1128, pass 9's N2 shape | 1128 |
| I1, last save wrote nothing, every file and since open | 0 | 0 |
| I2, a save wrote | **0** | 5748, 3344, 3320 |
| I2, nothing written | **56, 32, 32**, D1 | 924, 900, 900 |
| I3 | 0 | 0 |
| I4 | 0 | not applicable, no held sentence |
| I5 | **3368**, D2 and D3 | not applicable |

The 1128 I1 cases are Load marks of a report from outside the folder, then Save with no change, so nothing is written and the reopen with the folder's own files lacks that report. It holds the marks still, so nothing is lost. The status line says nothing was saved. Pass 9's N2, which the lead recorded and did not change. I1b on the base is the same D3 mechanism through tracked marks only. On the tip stable identifiers add untracked marks to it. The base's I2 counts are the class pass 9's D1 and D2 found. The tip removes every written-save case, and 56 nothing-written cases remain.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `npx vitest run examples/folder-viewer packages/core/src/tid1500-groups packages/core/src/tid2010 packages/core/src/sr` | 57995f45 worktree | now | 0, 97 of 97 |
| `npx tsc --build`, `npx eslint examples/folder-viewer packages/core/src`, `python3 scripts/prose_check.py`, dash grep of the diff, README column check | 57995f45 worktree | now | 0, 0, 0, no match, no added README line over 79 columns |
| `bin/ocelli.sh gate viewer-browser` | 57995f45 worktree, scratch `CARGO_TARGET_DIR`, after `pgrep -fl 'chromium\|playwright\|oracle'` showed only idle Playwright MCP servers | now | 0, ALL GREEN. `pass9` held save "Saved 1 mark and 0 key images in 1 file. 1 earlier report still holds a mark you deleted, ...", untracked reopened 0, twice files 1 |
| page probes 1 to 4 | `git archive` of 57995f45 in a scratch tree, linked `node_modules`, copied `packages/core/dist`, own target directory, the binary holding 2 `stableTracking` and 1 held sentence. Run after an until-loop waited out another worktree's viewer-browser job | now | as quoted in D1 to D3 and the D2 replay |
| module mutants, table below | the scratch tree, `mark-documents.mjs` | now | each run and restored, checked equal to the worktree file |
| page mutants, table below | the scratch tree, `page.html` mutated, `page.html`, `serve.rs` and `mark-documents.mjs` touched, rebuilt, the binary's strings checked to hold the mutant before `marks.mjs` ran | now | each run, restored, rebuilt, 0 mutant markers left |
| the model | scratch copies of the tip and 561cb730 `mark-documents.mjs` | now | as tabled above |

| Mutant | Result |
|---|---|
| V1, `held` never counted | red, `says an earlier report still holds a mark the user deleted` |
| V2, `stableTracking` always calls `fresh` | red, `gives an untracked mark the same identifier on every read of its report` |
| V3, `settled` ignores `deleted`, pass 9's O1 | red, `settles a report by a deletion alone` and the D2 test |
| own, V4, `held` also counts settled reports | **survived vitest**. The model's I5 rises from 3368 to 115 696. N3 |
| own, V5, the stable key ignores the report | **survived vitest and has no page step**. N2 |
| own, V6, the stable key ignores the group | **survived vitest**. N2 |
| PM1, the page drops the held remark | red, `held` |
| PM2, the page passes a new Map to `stableTracking` on each read | red, `untracked`: "FU with the new file showed 1 marks: save \"Nothing was saved.\"" |
| PM3, pass 9's PD, a save keeps `changedStudies` | red, `twice`: 2 files |
| PM4, pass 9's PG, the page sends no `changed` | red, `twice`: 2 files |
| own, PM5, a cleared save says "replaces the earlier one" whatever `held` is | **survived the gate**. N3 |

## Defects

### D1, a held deletion is not said when the save writes nothing

**Where**: `examples/folder-viewer/src/page.html:5242` returns before `:5263`
**What**: `buildDocuments` counts the held report, and the page returns "Nothing was saved." before it pushes the held remark. Probe 1 through the page: folder A with T, rectangles on slice 0 and slice 6. Delete the slice 0 rectangle. Open folder B, the same images and no document. Save, drawing nothing: "Nothing was saved." with 0 files. Folder A with T then reads "Read 1 document: 2 marks and 0 key images placed.", so the deleted rectangle is back. This is pass 9's probe A without the drawn mark.
**Why it is wrong**: the pass 9 D1 ruling says the status line names every such report, and the brief's invariant says a deleted mark returns only if the kept or the held sentence said so. README:398 to :401 says "the status line says so and asks you to save from the folder holding it".
**Evidence**: probe 1, quoted. Model I2 nothing written, 56, 32 and 32 on the tip, every one from this path.
**Scope and regression**: on HEAD, new in this range. Fix: push the held remark before the `!built.documents.length` return, as `unchanged` and `unflagged` are. Regression: the `held` page step without the drawn rectangle, expecting "Nothing was saved." and the held sentence. Red on today's code.

### D2, a report a read named as replaced is counted as held

**Where**: `examples/folder-viewer/src/mark-documents.mjs:378`, fed by `page.html:5337` and `notePartialReports`
**What**: a report remembered before a later read names it keeps `replaced: false`, so `held` counts it. Probe 4 through the page: Q is saved from a page that read T and deleted T's slice 6 rectangle, so Q names T and holds the slice 0 rectangle. A fresh page opens folder A with T, then folder FQ with Q alone, deletes the slice 0 rectangle and saves: "Saved 0 marks and 0 key images in 1 file. 1 study has no marks left, so its file holds no marks. 1 earlier report still holds a mark you deleted, so that deletion is not saved. Open the folder holding it and save there." T, Q and the saved file then read "0 marks", "Replaced by a later document: report-t.dcm, report-q.dcm". The deletion was saved. Following the advice opens folder A, where T is hidden, because the page's replaced set holds it, so there is nothing to save there.
**Why it is wrong**: the sentence is false, and it drops "replaces the earlier one", which was true. The ruling counts reports "not replaced". README:398 to :401 says such an SR "is not replaced, so that deletion is not saved". The brief's invariant: "replaces the earlier one" is said when true.
**Evidence**: probe 4, quoted. Model I5 in the {P, Q}, {P, Q, R} and {Q, R} pools, for example open the whole study with P, open it with Q, delete Q's rectangle, save.
**Scope and regression**: on HEAD, new in this range. Fix: a report whose SOP Instance UID is in the page's replaced set is replaced, for example `buildDocuments` takes `replaced: Array.from(sessionReplaced)` and `settled` and `held` honour it, or the page sets `replaced` on remembered entries when a read names them. Regression: probe 4 as a page step and as a vitest case, expecting no held sentence and "replaces the earlier one". Red on today's code.

### D3, a deletion stays recorded after the mark is read and saved again

**Where**: `examples/folder-viewer/src/page.html:4896` adds to `sessionDeleted`, and nothing removes from it. `mark-documents.mjs:327` and `:378` read it.
**What**: a mark deleted, then shown again by a later read and saved, still counts as deleted. Probe 2 through the page: folder A with T, delete both rectangles, open folder A again, both are shown, Save: "Saved 2 marks and 0 key images in 1 file." Open folder B, draw a rectangle, Save: "Saved 1 mark and 0 key images in 1 file." The first saved SR holds two marks that are in `sessionDeleted`, so it is settled and named, and T, both saves and folder B's images then show 1 mark, "Replaced by a later document: saved-0.dcm, report-t.dcm". Both of T's rectangles are gone and no sentence said so. A second shape makes the held sentence false: the whole study with T, delete the `ct` rectangle, open `ct` alone with T, which shows it again, Save. The save keeps the rectangle and says "1 earlier report still holds a mark you deleted, so that deletion is not saved."
**Why it is wrong**: the user put those marks back and saved them. A saved mark is lost with no status sentence, against "a mark nobody deleted is never lost" and the D1 ruling's rule that a lost deletion is said. README:396 to :397 says "a mark nobody deleted is never lost".
**Evidence**: probe 2, expected 3 marks, observed 1. Model I1b 80 on the tip, 32 on the base. Part of the model's I5 3368, the {T} and {T, X} pools.
**Scope and regression**: the lost-mark half is on 561cb730 too, through tracked marks, since pass 8's remediation, and pass 9's model did not reach it because its I1b excluded every deleted mark. This range adds untracked marks to it, because a stable identifier now matches the deletion, and adds the false held sentence. Fix: remove a tracking from `sessionDeleted` when a read places or repeats a mark carrying it, which `placeDocuments` already knows, and count `held` only for a deleted tracking not in the save. Regression: probe 2 as a page step expecting 3 marks, and a vitest case of a deleted then re-read mark expecting the report not named. Red on today's code.

## Smells

None.

## Nitpicks

- **N1, Load marks of an untracked report already shown places it again, now with the same identifier.** Probe 3: U shown, Load marks with U, "Read 1 document: 1 mark and 0 key images placed.", and two notes carry the one identifier. The duplicate is on 561cb730 too, with two identifiers. `placeDocuments` dedupes by identifier before `stableTracking` gives one, so an untracked mark is never counted as already shown. With stable identifiers the dedupe could run after `stableTracking`.
- **N2, no test holds two untracked reports apart.** V5 and V6, a stable key without the report or without the group, pass vitest, and the gate's `untracked` step reads one report of one group. The code is right, probe 3. A vitest case reading U and U2 together would be red under V5.
- **N3, "replaces the earlier one only when true" has no guard.** PM5 survives the gate and V4 survives vitest, because no case reaches a cleared save with a held report or a held count from a settled report. D2's regression would cover both.
- **N4, the no-change wording.** README now says "since the page loaded or since its last save". The status sentence at `page.html:5240`, the `marks.mjs:106` constant, the comments at `mark-documents.mjs:331`, `mark-documents.d.mts:60` and `:115` and `page.html:1161`, and the plan's pass 8 ruling at `F-215-design.md:427` keep "since its documents were read". The status sentence is true whenever it is shown. The D3 ruling says the pass 8 ruling's wording is corrected, and line 427 is unchanged.

## Verified clean

- `stableTracking` keys by the report's SOP Instance UID and the group's index, sets the identifier on the placed mark before the page builds the note, and returns, by report, the identifiers it gave, which the page appends to that report's remembered marks. A report's SOP Instance UID is required by `readTid1500Groups`, so the key is never empty. Probe 3 through the page and the model's U pools agree.
- `held` is computed only for a study that is saved and changed, and an unchanged study is skipped before it, so a second Save does not repeat it. `failed` carries `held`. The `.d.mts` declares `held`, `Placed.marks[].group` and `stableTracking` as the module has them. tsc and eslint are clean.
- The new vitest cases build their reports through `buildDocuments` and `readDocument`, not hand-made objects, and each is red under its mutant, V1 to V3. The new page steps build their folders from `/synthetic`, wait for each read, and are red under PM1 to PM4.
- I3 and I4 hold in every one of the 216 576 cases: a no-change Save never writes and always says so, and "replaces the earlier one" is never followed by a returned deletion that no kept or held sentence covered.
- README:394 to :413 is within 79 columns and passes the prose check. Apart from D1 to D3, its sentences match the model. `use.html` and `more.html` are not touched and make no claim this range changes.

## Limitations and handoff

- Page runs were in Playwright's Chromium on the synthetic CT. Real Chrome, AC18 and AC19, stay with the operator at sprint close, as ruled. AC20 was answered before implementation and is not re-run.
- The model covers one study, two images and the eight documents above. Key Object Selections, text marks, a second study and multiframe targets are not in it. A move is a change of geometry inside the image and is not checked against `froundPoints`.
- The probes, mutants and model ran in scratch copies prefixed `f215-p10-` with linked `node_modules` and their own `CARGO_TARGET_DIR`s, removed afterwards with Python `shutil`. No tracked file was edited apart from this review, which is left uncommitted.
- Owed acceptance rows: AC18 and AC19 at sprint close. Nothing else in this range.
- Next: the implementer fixes D1 to D3. The lead decides N1 to N4. A fresh pass reviews the remediation. D1's class, a deletion lost with nothing said, has now survived passes 9 and 10 in changing shapes, which is progress but worth the operator's attention.

F-215 pass 10: 3 defects, 0 smells, 4 nitpicks.
