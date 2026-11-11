# F-215 review, pass 12

**Reviewed**: staged tree `ea01d561318659989434ec73a6c95215c47092cd` on `work/f-215-xai`, HEAD `b599c273446c39a30f421bbcf59773a46e685479`. This pass reviews root's remediation of pass 11, with no unstaged tracked changes. The earlier pass 11 report is historical context, not new material independently approved by its own author.
**Author / reviewer**: remediation by root Codex. Independent review by Codex participant `/root/f244_review`, which authored none of the remediation or permanent test changes. Browser probes and mutations are review-only scripts outside the worktree.
**Result**: 1 defect, 0 smells, 0 nitpicks

## Acceptance challenged

- Pass 11 D1, both folder replacement and a same-folder edit while the first document module import is held. The independent probe checks the old operation before making a new valid Save, so a later operation cannot hide an earlier state change.
- Pass 11 D2, the second public Load marks of the same untracked document, checking both the visible count and the status.
- Pass 11 D3, P then Q compared with loading both together, plus an unrelated authored mark.
- The new saved-report ownership rule, through a refused save, a successful retry and a later document naming only the older source report.
- The permanent pending-save browser test, challenged by removing the production generation guard in the served page without changing the test.

## Evidence

Browser observations below ran now against the existing release viewer in the worktree, built by root for this staged source. Chromium reported `151.0.7922.34`. The scripts start their own loopback viewer and stop it and their browser in `finally`. They use only generated synthetic CT and SR data. Temporary scripts, logs and downloads are under `/private/tmp/ocelli-f215-review-p12`.

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `git write-tree` | Actual index | now | 0, exact tree above |
| `async-save-verified.mjs` | Independent delayed import and folder-switch control | now | 0, stale operation writes zero documents, B remains changed, valid B Save writes B's one group |
| `async-save-verified.mjs same-folder` | Independent delayed import with second mark added | now | 0, one document contains both visible marks |
| Pass 11 `load-lifecycle.mjs` replay | Same U, P and Q constructions as the original failure | now | 0, duplicate status corrected, separate and combined successor reads both show one mark |
| `ownership.mjs` | P plus unrelated drawing, item-limit refusal, successful retry, Q naming P | now | 0, preservation and ownership controls below pass |
| `pending-permanent-mutant.mjs base` | Actual exported `observePendingSaves` | now | 0, original expected group counts |
| `pending-permanent-mutant.mjs mutant` | Same test, Save generation check removed in served HTML | now | 0, same expected group counts despite guard removal, D1 |
| Full `marks.mjs`, lint and staged content/prose checks | Root's evidence for this source | prior receipt, not rerun by reviewer | Root reports exit 0 |

The served-page mutation changes only the unique Save sequence consisting of the generation capture, module await and following generation check. It does not modify the source worktree, test helper, document module or binary. Two mutated page responses were served during the permanent test's two cases, and that count is recorded in its log.

## Defects

### D1, the permanent pending-save test does not observe the registered handler

**Where**: `examples/folder-viewer/tests/browser/marks.mjs`, `observePendingSaves`, the `window.saveMarks` wrapper and `window.pendingSaveProbe` waits around lines 859 to 878.

**What**: the page registered its original `saveMarks` function as the button listener before the test replaces `window.saveMarks`. Reassigning the global property does not replace the function reference held by `addEventListener`. The click still invokes the original listener, and the wrapper does not capture that operation's promise. Waiting for `window.pendingSaveProbe` therefore does not establish that the pending button operation finished.

The folder-switch case then issues a valid B Save without first asserting that the old operation left B untouched. A stale operation that saves B after the import resolves can consume B's change, and the explicit B click then writes nothing. The resulting one B download matches the test's final assertion and masks the absent generation guard.

**Why it is wrong**: `.claude/commands/microscope.md` section 2 requires tests to discriminate the wrong implementation. The lead's pass 11 ruling specifically requires discarding a pending save when folder generation changes. The new comment claims observation of the actual button handler's promise, which this wrapper does not supply.

**Evidence**: the permanent helper was run twice, first unmodified and then with only the production generation check removed in the served HTML. Both executions exit 0 and report the same result:

```text
switchFolder true: one download, one group, first column 60
switchFolder false: one download, two groups, first column 10.00000286102295
```

The mutation was applied to two page responses. Logs are `permanent-base.log` and `permanent-no-generation.log`. Root's product implementation itself passes the independent stronger control: waiting for the shared module promise and its continuation before a new Save shows zero old writes and B still changed. The defect is the permanent regression's observation, not an observed remaining product failure.

**Scope and regression**: wire the observation to the actual button listener, for example remove its original registered function and add a wrapper calling that function. After awaiting the stale operation, require no remembered document and B still changed before issuing a valid B Save. Keep the same-folder document-content control. Re-run the generation-guard mutation and require a semantic failure. Root acknowledged this ruling during the pass. No implementation edit was made by this reviewer.

## Smells

None found in this bounded pass.

## Nitpicks

None.

## Verified clean

- Pass 11 D1 product behavior: after folder replacement the pending A operation makes no document and does not clear B's changed state. A valid B Save writes B's geometry. In the same folder, a mark added during module loading is included with the first mark.
- Pass 11 D2: the second untracked U read leaves one visible mark and says zero placed and one already shown.
- Pass 11 D3: P then Q leaves one mark, matching P plus Q loaded together. Adding an unrelated drawing before Q leaves that drawing and Q's one mark.
- Refused save: an oversized synthetic mark list makes the writer refuse at its content-item bound. No document is written, the original P ownership remains on its two loaded notes, the changed study remains, and the remembered document count is unchanged.
- Successful retry: after restoring the small synthetic fixture, Save writes one SR R, assigns all three saved notes to R, and clears the changed study. Reading Q, which names only P, preserves all three R-owned notes. This tests the new ownership assignment against the later removal consumer.
- The source repair keeps all work after the module await synchronous, so the snapshot and its successful ownership update cannot interleave with another browser event in that section. This last observation is source inspection, separate from the executed cases.

## Limitations and handoff

- Root owns the test repair and its next independent pass. Product source was not remediated by this reviewer.
- The complete permanent marks suite was not rerun by the reviewer. Its partial-report and other cases are root's recorded execution, not new independent observations in this pass. The independent probes above target the repaired lifecycle and ownership paths.
- No full frontend, viewer-browser gate, GPU, oracle, cross-target, wasm or sprint profile was run. Real Chrome AC18 and external-viewer AC19 remain open.
- The old read-order enumeration was not rerun. Clean observations here are bounded to the source and combinations above.
- Browser slot released to `f225_review` after the probes and owned processes finished.
