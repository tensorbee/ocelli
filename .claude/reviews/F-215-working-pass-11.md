# F-215 review, pass 11

**Reviewed**: staged tree `8575b804da03c35fa7c7fff6db429d61ca17b6b2` on `work/f-215-xai`, HEAD `b599c273446c39a30f421bbcf59773a46e685479`. Eight staged paths, no unstaged tracked changes. The report itself is a later review artefact, outside that identity. The untracked `.venv` and `tools/oracle/out` symlinks were excluded.
**Author / reviewer**: the reset rework includes interrupted Claude changes preserved by the lead, then implementation by root Codex. Independent review by Codex participant `/root/f244_review`, which authored none of the implementation, tests, plan clarification or earlier F-215 passes. Independence is per participant, not per model.
**Result**: 3 defects, 0 smells, 0 nitpicks

## Acceptance challenged

- The operator's per-folder reset, challenged with a Save waiting for the first lazy module import while another folder is opened. The pending operation must not repopulate the new folder's document state or consume its changes.
- The retained no-change rule, challenged with a second mark added in the same folder while that Save waits. A later mark must be saved or remain changed for the next Save.
- The retained stable identifier rule, challenged through two actual Load marks operations on an untracked SR, including the visible count after deduplication.
- The retained within-folder predecessor policy, challenged with the same P and Q read separately and together. Q names P as a predecessor and omits one of P's marks.
- The optional Tracking Unique Identifier rule, tested by disabling identifier reuse in a scratch copy of the changed module and rerunning the focused suite.

## Evidence

Browser probes used the existing release viewer binary in the reviewed worktree, which the lead had just built for the staged source. Chromium reported `151.0.7922.34`. Probes start a viewer on port zero, launch headless Chromium, and close the browser and stop that viewer in `finally`. Only generated synthetic CT data and SR documents were used. All temporary scripts, logs and downloads remain under `/private/tmp/ocelli-f215-review-p11` and must not be staged.

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `git write-tree` | Reviewed worktree index | now | 0, exact tree above |
| `async-save.mjs` | Hold `/mark-documents.mjs`, save A, open B, add B mark, release | now | 1, stale A download and B changes cleared, D1 |
| `async-save.mjs same-folder` | Hold import, save one mark, add another, release | now | 1, one saved group beside two visible marks, changes cleared, D1 |
| `load-lifecycle.mjs` | Untracked U loaded twice, and P then Q against P plus Q together | now | 1, visible count and read-order mismatches, D2 and D3 |
| Focused Vitest suite in scratch copy | Staged module, declarations and tests copied without edits | now | 0, 40 tests pass |
| Same Vitest suite, stable identifier lookup replaced by `fresh()` | Scratch module only | now | 1, stable identifier test fails, 39 others pass |
| Same Vitest suite after restoration | Scratch source restored | now | 0, 40 tests pass |
| Full `marks.mjs`, lint, typecheck and 119 preview tests | Lead's implementation evidence for this source | prior receipt, not rerun by reviewer | Lead reports all exit 0. Logs are under `/private/tmp/ocelli-s46-codex` |
| `git diff --exit-code` | Actual worktree after probes | now | 0, no unstaged source changes |

The focused suite runs from `/private/tmp/ocelli-f215-review-p11/unit` with the worktree's installed dependencies linked in. It does not mutate the review tree. No timing comparison is inferred from these busy-host checks.

## Defects

### D1, a pending Save consumes state from a later folder or edit

**Where**: `examples/folder-viewer/src/page.html:5187` to `:5229`, then the document recording and `changedStudies.delete` at `:5252` onwards.

**What**: `saveMarks` gathers its entries and image lists before awaiting `loadMarksModule`. After the await it reads the live document and changed-study state without verifying the folder generation. The saved snapshot and the state declared saved can therefore belong to different moments or folders.

**Why it is wrong**: the operator's reset decision says nothing carries between folder opens. The retained no-change rule must not claim a mark was saved when the document does not contain it. This is a page lifecycle violation, not a DICOM arithmetic issue.

**Evidence**: on the initial synthetic folder, draw a rectangle at approximately (10,10) to (20,20), press Save, and hold the first `/mark-documents.mjs` request. Open the synthetic folder again, which changes generation from 2 to 4 and resets state. Draw another rectangle at approximately (60,60) to (70,70), then release the request. The download contains only the first rectangle's coordinates. The new folder still displays the second rectangle, but now has one remembered document and no changed study. Pressing Save again writes nothing and says no mark or key image changed.

The same-folder control keeps generation unchanged and adds the second rectangle while the import waits. Both marks remain visible, the download holds one group, the changed set is cleared, and the next Save also writes nothing. There were no page errors in the cross-folder probe.

**Scope and regression**: the missing async guard predates this remediation, but it defeats the new reset guarantee and the retained save rule through the changed page consumer. Load the module before taking the synchronous save snapshot and verify generation after the await. Add a public browser test with a delayed first import, covering folder replacement, a same-folder edit and the valid resumed operation. A generation check alone does not fix the same-folder form.

### D2, an untracked duplicate is counted as newly placed

**Where**: `examples/folder-viewer/src/page.html:5314` to `:5317` and `:5352` to `:5356`.

**What**: the new page-side deduplication after stable identifier assignment skips an already shown untracked group. The status still counts the original `placed.marks` array and does not increment the repeated count for that skipped group.

**Why it is wrong**: the status claims a new placement when none occurred. The new plan clarification promises repeated untracked reads keep one visible mark. The visible status must describe that outcome rather than report another mark placed.

**Evidence**: U contains one rectangle with no Tracking Unique Identifier. Open the synthetic folder, Load U, then Load U again. The page has one visible mark after both reads. Both status lines nevertheless say `Read 1 document: 1 mark and 0 key images placed.` The second line has no already-shown count. The real public Load marks input was used for both operations.

**Scope and regression**: introduced by this remediation's page-side deduplication. Count only marks actually inserted and include post-assignment duplicates in the already-shown count. Extend the existing untracked browser case to assert its second status and not merely the resulting mark count.

### D3, P then Q keeps a mark that P plus Q hides

**Where**: `examples/folder-viewer/src/page.html:5311` to `:5345`, composed with `placeDocuments` and the retained predecessor policy in the plan's pass 6 and pass 7 rulings.

**What**: reading a later SR records its predecessor UIDs for future reads, but does not remove marks already loaded from those predecessor reports. Deduplication then prevents the later report's retained mark from being inserted, while the omitted mark remains visible.

**Why it is wrong**: skipping superseded reports on load is an explicit project choice retained within one folder. DICOM's predecessor reference supplies the lineage, while the skip policy comes from the approved plan. The reset decision withdraws cross-folder state, not this within-folder rule. Reading the same pair together and separately must not give different visible marks under that rule.

**Evidence**: P has two tracked rectangles, A and B. Q names P as its predecessor and contains only B with its original tracking identifier. On one page in one synthetic folder, Load P gives two marks. Load Q then gives zero new marks and one already-shown mark, but both A and B remain visible. On a fresh page in the same folder, Load P and Q together gives one mark, B, and names P as replaced. The documents and image references are identical between the two controls.

**Scope and regression**: this behavior predates the reset diff. It is newly reproduced through the unchanged consumer paths that this remediation claims to preserve, and therefore is not evidence that root's reset introduced it. The lead accepted it into F-215 after the reproduction. Track ownership for loaded page notes so that loading a report can remove notes sourced from its explicitly superseded reports before placing the later content. Preserve unrelated marks authored in the page. Add the separate-read and combined-read controls through the public input, with an unrelated authored mark as a preservation control.

## Smells

None found in this bounded pass.

## Nitpicks

None.

## Verified clean

- `readFolder` clears all six new folder-scoped records synchronously, beside the existing note and key-image clear. The reset itself is direct and has one caller site.
- The removed deletion and settlement records have no remaining references in the page and module changes inspected. The retained stable identifier map is cleared with the folder.
- The new stable identifier unit test distinguishes reuse from a fresh identifier on every read. The recorded mutation fails semantically, and restoration passes.
- The focused module suite passes all 40 tests at the staged source.
- `readDocuments` checks generation after awaiting module load and file bytes, unlike the Save path in D1. This was source inspection, not a separate delayed-read browser measurement.
- The status and marker observations in this report come from synthetic browser operations. No patient data or actual patient document was opened.

## Limitations and handoff

- No implementation remediation was written. Root owns D1 to D3 and will send the repaired tree for another independent pass.
- The full read-order enumeration from earlier Claude reviews was not rerun, and its results do not establish this staged tree's page behavior. The new public-caller controls expose cases the retained module suite does not cover.
- No full viewer-browser gate, complete frontend suite, GPU, oracle, cross-target, wasm or sprint profile was executed by this reviewer. The lead's prior checks are listed separately.
- Real Chrome AC18 and external-viewer AC19 remain open. This is a bounded repair review, not completed story acceptance.
- The browser slot was released to `f225_review` after all probes finished and owned processes were stopped.
