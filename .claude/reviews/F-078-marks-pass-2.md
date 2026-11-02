# F-078 review, pass 2

**Reviewed**: HEAD `f3012c6cf36193be860025435825edc6d5f321a0` on `sprint/s29`. That commit was not amended. This pass covers the unstaged inventory edit in `packages/core/src/index.test.ts` only. Pass 1's clean result does not cover this edit. After the tests, `shasum -a 1` of the working tree file is `a75067ec2d39539c8d99f6b5b800e144ebdeb805`. The path is not staged. `git diff HEAD` for it is 7 insertions. The three product files were not edited again. Their `shasum -a 1` values still match the pass 1 pins. `packages/core/src/index.ts` is `bd6271eb2a35433a1d546c1f044d0b597c86ee83`. `packages/core/src/marks.ts` is `30dcc463ef1743fc65aacdf2d52ba092479f3be6`. `packages/core/src/marks.test.ts` is `87a2eb660c5048f0cc855b2439992ac28623ae56`. Those three paths are staged. `git diff`, `git diff --cached`, and `git diff HEAD` for `packages/core/src/annotation.ts` are empty.
**Author / reviewer**: The F-078 change has no commit, so this pass does not name an author from a commit or a trailer. The reviewer did not write the change. Pass 1 is a different reviewer.
**Result**: 0 defects, 0 smells, 0 nitpicks

## Acceptance challenged

The inventory claim is the hand-written `PUBLISHED` array in `packages/core/src/index.test.ts`. It is not derived from `index.ts`. The exact-set test requires `Object.keys` of the barrel to equal that array. The assignment says the feature profile failed only the `packages` gate, and that this list was the missing names. This pass did not re-run that gate. `bin/ocelli.sh` runs the gate as `npm run test` and then `python3 scripts/package_check.py`. `npm test` is `vitest run`.

The six added list names are exactly `MARK_REFUSALS`, `arrowMark`, `etdrsMarks`, `etdrsRegion`, `keyImageMark`, and `labelMark`. The unstaged hunk adds those six strings and the comment `// ./marks.js, F-078`. It adds no other string.

`git diff HEAD` for `packages/core/src/index.ts` adds the same six names in one value export from `./marks.js`. The same hunk adds `export type { EtdrsRegion, MarkRefusal, MarkResult, MarksResult }` and one header sentence. No other runtime name was added. The four type names are not in `PUBLISHED`. The exact-set test passed without them, so they are not runtime keys.

A parser collected the value exports in `index.ts`, skipping `export type`, and compared them with the quoted strings in `PUBLISHED`. Each side has 115 names, with no duplicates. Both differences are empty. The six names occur in the same order on both sides.

`marks.test.ts` imports those six from `./index.js`, not from `marks.ts`. Its 9 tests passed on this tree. A barrel that dropped one of the six, or that bound a different value, would fail that file or the exact-set test. The list file was not mutated. The assignment said not to change it. The red case follows from the executed equality. `PUBLISHED` contains the six, and the passing test requires `Object.keys(core)` to equal `PUBLISHED`. An array without those six would not equal that key set.

A seventh runtime name in the list, or one of the four type names, would also make the sets unequal. The passing test rejects both. `annotation.ts` is not a second publisher of these names. Its diff against HEAD is empty. This pass does not reopen the constructor arithmetic. Pass 1 covered that, and the three product hashes still match.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `shasum -a 1` of `index.ts`, `marks.ts`, `marks.test.ts`, and `index.test.ts` | Working tree after the vitest run. HEAD `f3012c6cf36193be860025435825edc6d5f321a0` | Executed now | The first three digests match the pass 1 pins. `index.test.ts` is `a75067ec2d39539c8d99f6b5b800e144ebdeb805` |
| `git diff`, `git diff --cached`, and `git diff HEAD` for `annotation.ts` | Same HEAD | Executed now | All three diffs are empty |
| `git diff HEAD` for `index.ts` and unstaged `git diff` for `index.test.ts` | Same HEAD. `index.ts` is staged. `index.test.ts` is not | Executed now | `index.ts` is 11 insertions. The runtime names added are the six. `index.test.ts` is 7 insertions. The strings added are the six |
| Parser of value exports against `PUBLISHED` | Working `index.ts` and `index.test.ts` | Executed now | 115 and 115. No duplicates. Empty difference both ways. The six names match in order |
| `npx vitest run packages/core/src/index.test.ts packages/core/src/marks.test.ts` | Working tree. Vitest 4.1.11. Node `v24.13.0` | Executed now | Exit 0. `index.test.ts` has 4 passed. `marks.test.ts` has 9 passed. 13 tests, 0 failed |
| Feature-profile `packages` failure | Assignment statement | Prior receipt. Not re-run | Not executed in this pass |

Environment: macOS, zsh, Node `v24.13.0`. The vitest exit status is the process status, not a pipe. No patient data was used. dwv, Horos, and the medical viewer Grok were not opened.

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

The unstaged inventory hunk is the six runtime names under `// ./marks.js, F-078`, placed after the F-071 annotation names and before the F-050 snapshot names. That is the same order as the value export in `index.ts`. The names are `MARK_REFUSALS`, `arrowMark`, `etdrsMarks`, `etdrsRegion`, `keyImageMark`, and `labelMark`.

`marks.ts` exports those six values and the four types `MarkRefusal`, `MarkResult`, `MarksResult`, and `EtdrsRegion`. `index.ts` re-exports all ten. Only the six values belong in `PUBLISHED`. The exact-set test is sorted on both sides, and the source order still matches. There is no duplicate in either set.

`marks.test.ts` reaches the six through `./index.js`. The passing marks file shows the barrel bindings are the constructors and `MARK_REFUSALS`, not an unused alias. The passing inventory file shows the published key set is exactly the hand-written list, including those six and no further F-078 name.

`annotation.ts` has no staged or unstaged change. The constructor files were not edited in this pass. Their SHA-1 values match the pins recorded before the vitest run and again after it.

## Limitations and handoff

This pass is the inventory edit. It is not feature completion and it is not a sprint review. Pass 1 remains the constructor review. That clean result does not cover this list.

The inventory edit is unstaged. The staged index contains `index.ts`, `marks.ts`, and `marks.test.ts`, and it does not contain the six new list lines. A commit of that index alone would publish the exports without the inventory. That is a staging fact. It is not a wrong list in the working tree.

Staged paths outside this pass were not reviewed and are not accepted here. They are `.claude/reviews/F-078-marks-pass-1.md`, `CHANGELOG.md`, `docs/lld/README.md`, `docs/lld/public-api.md`, `docs/sprints/AS_BUILT.md`, `docs/sprints/BACKLOG.md`, `docs/sprints/CURRENT_SPRINT.md`, and `docs/sprints/SPRINT_TRACKER.md`.

The type-only parenthetical in `index.test.ts` was not part of this edit. A parse found 53 backticked names there and 90 `export type` names in `index.ts`. All 53 are real type exports. The four F-078 types are among the names the parenthetical does not list. The sentence does not say the parenthetical is the full set. This pass does not count that standing gap.

`npm run test`, `python3 scripts/package_check.py`, and `tsc` were not run. The declaration emit and the published tarball were not inspected. No GPU and no oracle comparison. The full feature profile was not repeated.

The list file was not mutated, so this pass did not watch a red run. The three product SHA-1 values match the pins. Do not attach this result to a later edit of `index.test.ts`. This review file is untracked.
