# F-215 review, pass 6

**Reviewed**: 1d2bcc8f..980062f9 on `work/f-215-xai`, tip 980062f9, one commit ("F-215, review pass 5 remediation"). The worktree was clean apart from the untracked `.venv` and `tools/oracle/out`, which are not part of the story.
**Author / reviewer**: a separate Claude implementer agent wrote 980062f9. This pass is an independent Claude reviewer that wrote none of the reviewed material and none of passes 1 to 5. Both are Claude, so independence is per participant, not per model.
**Result**: 3 defects, 0 smells, 1 nitpick

## Acceptance challenged

- **Pass 5 D1, forward order, through the page.** P holds a rectangle on slice 1 and a length on slice 6, saved by its own page. A second page opened the whole study with P, deleted the rectangle and saved Q, which names P. A third page opened slices 2 to 12 with P ("1 item not in this folder was not placed"), then Load marks with Q ("1 mark was already shown"), deleted the shown length and saved. The save said "Saved 0 marks and 0 key images in 1 file. 1 study has no marks left, so its file holds no marks and replaces the earlier one." with no kept sentence. A fresh page on the whole study with P, Q and the new file showed 0 marks, "Replaced by a later document: report-q.dcm, report-p.dcm." The status line and the reopen agree. Closed for this order.
- **Pass 5 D1, reverse order, through the page.** Same P and Q. A page opened slices 2 to 12 with Q, which placed Q's length whole, then Load marks with P, which placed nothing ("1 item not in this folder was not placed. 1 mark was already shown."). The shown length was deleted and Save marks pressed. The save said "... 1 report placed only in part is kept, so the marks it holds come back." The reopen of the whole study with P, Q and the new file showed 0 marks and "Replaced by a later document: report-q.dcm, report-p.dcm." The two disagree (D1 below).
- **Study drop.** At module level, P1 and P2 of one study placed in part in one read, then Q naming P1 in a second read, leaves `{ study: [P2] }`. The helper deletes a study only when its set is empty. Correct today, but no committed test holds it (N1).
- **Pass 5 D2, the comment.** Re-run on a small sample: a four-group ISO_IR 192 report and the same report deflated as ISO_IR 203, every seventh cut from byte 140 and 300 seeded single-bit flips past the preamble. Every truncation of both returns null. A scratch copy of `readGroups` instrumented with the exit taken shows why: for the ISO_IR 192 file 743 of 937 cuts return null because no row 6 Measurements container is found, and 194 because the walk, the parse or naturalising throws. For the deflated file all 175 throw. **None return null because the template or the identity is missing**, which is the reason the new comment gives (D2 below). The flip half holds: ISO_IR 192 had 79 fewer-group documents of 300 and deflated ISO_IR 203 had 129, among them 3 with 0 groups and 0 unread.
- **README's causes against the counting.** `placeDocuments` marks a report not whole for any of seven reasons, the keys of `UNPLACED_TEXT`: missing, duplicate, frameless, outside, graphic, geometry and unread. Each puts it in `partial`, so the status line counts it kept. README names three. A report whose one length sits on an image two files share gives `unplaced` `{ duplicate: 1 }` and `kept` 1 (D3 below).
- **public-api.md against the standards.** ISO-IR 166 registration read page by page now, from itscj.ipsj.or.jp/ir/166.pdf: a 96-character set of 88 Thai characters, 2/0 NO-BREAK SPACE, and positions 5/11 to 5/14 and 7/12 to 7/15 each "This position shall not be used". In GR those are A0, DB to DE and FC to FF, as the doc says. PS3.3 Table C.12-2, current edition at dicom.nema.org, gives ISO_IR 166 as ISO-IR 166, 88, G1, TIS 620-2533, with ISO-IR 6 as G0 and no C1 element. PS3.5 chapter 6 says "characters from the C1 set shall not be used". The doc's C1 sentence and its ISO_IR 166 sentence are both right, and the 166 rule is now stated once. `stringsFor` applies the C1 refusal to every `SINGLE_BYTE` set and the default repertoire refuses every byte above 7F, so "in a single-byte set" holds.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `npx tsc --build`, `npx eslint examples/folder-viewer packages/core/src`, `python3 scripts/prose_check.py` | 980062f9 worktree | now | 0, 0, 0. No em-dash or en-dash in the commit |
| `npx vitest run examples/folder-viewer` plus tid1500-groups, tid2010, secondary-capture, sr, tid1500 | 980062f9 worktree, core built | now | 0, 99 of 99 |
| `bin/ocelli.sh gate viewer-browser`, `OCELLI_PORT=0`, scratch `CARGO_TARGET_DIR`, after `pgrep` showed only idle Playwright MCP servers | 980062f9 worktree, fresh release build | now | 0, ALL GREEN. kept: save "... 1 report placed only in part is kept, so the marks it holds come back.", reopened 2 |
| forward and reverse superseded cases through the page, Playwright's Chromium | `git archive 980062f9` copy, the gate's release binary | now | as under Acceptance challenged |
| module probe, Q then P in two `placeDocuments` calls, `notePartialReports` after each | same copy | now | `kept` 1, session `{ study: [P] }`, reopen 0 marks, `replaced` [P, Q] |
| module probe, P1 and P2 in part, then Q naming P1 | same copy | now | session `{ study: [P2] }` |
| truncation and flip sample, instrumented exit | same copy, `tid1500.ts` instrumented then restored | now | as under Acceptance challenged |
| truncation and flip probe, 5 variants, 410 cuts and 1500 flips each | pass 5 | prior receipt | every truncation null, flips fewer-group at both commits |
| ISO-IR 166 registration, PS3.3 C.12, PS3.5 chapter 6 | itscj.ipsj.or.jp, dicom.nema.org current | now | as above |
| mutation catalogue, 4 on the new tests and 3 of my own | copy, each restored | now | 6 red, 1 survived. Table below |

| Mutant | Result |
|---|---|
| `notePartialReports` removes nothing | red, "does not count a partial report a later document replaces" |
| `placeDocuments` returns `named: []` | red, the same test |
| `notePartialReports` adds nothing | red, both pass 5 tests |
| `kept` counts one per study | red, "counts each partial report of a study" |
| own, a named report drops its whole study from the record | **survived** (N1) |
| own, an emptied study left in the map | red, the `toEqual({})` check |
| own, `named` taken from no document | red |

## Defects

### D1, a partial report read after the document that replaces it is still counted kept

**Where**: `examples/folder-viewer/src/mark-documents.mjs:543`, called at `examples/folder-viewer/src/page.html:5297`
**What**: `notePartialReports` removes a report only when a document of the same read names it. When Q, which names P, is read first and P is read in a later read and placed in part, P is added and nothing removes it. The save then says "1 report placed only in part is kept, so the marks it holds come back.", and on reopen Q replaces P and P's marks do not come back.
**Why it is wrong**: the pass 5 D1 ruling asks that a partial report a later document replaces is not counted kept, and PS3.3 C.17.2 makes Q replace P whatever order the page read them in. README's own "A later SR that names it as replaced does replace it" says the same thing the status line now contradicts.
**Evidence**: the reverse-order page probe and the module probe above. The forward order is right.
**Scope and regression**: present on the tip. It needs two reads in one session, the folder and then Load marks, or two folder opens, with the replacing document read first. Fix: keep every UID any read has named, for example a `sessionNamed` set the page passes to `notePartialReports`, and neither add nor keep a report in it. Regression: in `mark-documents.test.ts`, Q naming P placed in one call, then P in a second call, expect `kept` 0. It is red on the tip.

### D2, the `parsed` comment gives a reason for a truncated file returning nothing that no sampled truncation takes

**Where**: `packages/core/src/tid1500.ts:1049` to `:1050`
**What**: "A truncated file still returns nothing, because the partial read then lacks the template or the identity checked below." The first half is true. The reason is not: of 1112 sampled cuts across the two variants, none returned null at the template or identity check. They returned null because no row 6 container was found, 743, or because the walk, the parse or naturalising threw, 369.
**Why it is wrong**: a false claim in new prose, `/microscope` section 3. The reason clause came from pass 5's suggested wording and pass 5 did not execute it, which this pass corrects.
**Evidence**: a scratch copy of `readGroups` recording which exit it took, run over every seventh cut.
**Scope and regression**: prose only, no behaviour change. Fix: delete the reason, leaving "A truncated file still returns nothing." If a reason is wanted, "because the parse then fails or finds no row 6 container" is what was observed.

### D3, README names three causes of a report placed only in part, and the status line counts seven

**Where**: `README.md:396` to `:398`
**What**: "An SR placed only in part, because the folder lacks an image one of its marks is on, a group of it cannot be read, or a mark of it lies outside its image, is not replaced ...". `placeDocuments` also leaves a report in part when a mark is on an image whose SOP Instance UID two files share, on a multi-frame object with no frame number, drawn as a shape this viewer does not draw, or has no image region to draw. Each is counted kept and comes back the same way.
**Why it is wrong**: the pass 5 ruling on N3 asks README to name every cause, and its list of three was short. A README sentence that reads as the full list and is not is a defect under `/microscope` section 3.
**Evidence**: module probe, one length on an image two page images share: `unplaced` `{ duplicate: 1 }`, `kept` 1.
**Scope and regression**: prose only. Fix: "An SR placed only in part, because one of its marks is not placed or a group of it cannot be read, ...", which covers every reason the read's status line names, or keep the three as "for example because". No test is asked.

## Smells

None.

## Nitpicks

- N1. Nothing tests that a report named as replaced leaves another partial report of the same study in the record. Dropping the whole study on a match survives the suite. Add a second partial report of the study to the D1 test and expect it to stay.

## Verified clean

| Pass 5 | State | How |
|---|---|---|
| D1 | **open**, closed for the forward order only | forward order through the page agrees with the reopen. Reverse order is D1 above |
| D2 | **open** for the reason clause, closed for the rest | truncations null and flips fewer-group, both reproduced now. The reason is D2 above |
| N1 | closed | `kept` 2 for two reports, the one-per-study mutant is red |
| N2 | closed | the C1 and ISO_IR 166 sentences match the registration, Table C.12-2 and PS3.5 chapter 6, and the 166 rule appears once |
| N3 | **open** | the list is still short, D3 above |

The page calls the helper in place of its old loop, and the gate's kept case and the forward probe both run through it. The `.d.mts` declares `named` and `notePartialReports` as the module has them. `use.html` and `more.html` are unchanged and make no claim this range touches. The gate is green on the tip.

## Limitations and handoff

- Both page probes ran in Playwright's Chromium on the synthetic CT. Real Chrome, AC18 and AC19 stay with the operator at sprint close, as ruled. AC20 was answered before implementation and is not re-run.
- A plausible concern outside this range, not counted: `placeDocuments` skips a replaced report only within one read, so a page that reads Q and later reads P with all of P's images present places P's marks again and names P in the next save. That is placement behaviour from earlier passes.
- The truncation and flip sample is smaller than pass 5's, and the deflated sample is one set.
- All probes and mutants ran in a `git archive` copy with linked node_modules and their own `CARGO_TARGET_DIR`. Scratch was removed with Python. No tracked file was edited. This review is left uncommitted.
- Next: implementation remediates D1 to D3, then a fresh independent pass 7.

F-215 pass 6: 3 defects, 0 smells, 1 nitpick.
