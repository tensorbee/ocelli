# F-216 review, pass 13

**Reviewed**: `git diff f799b90 5c7f1c9` (the lint remediation), and the merge f799b90 against its parents 952e9ad and de21a8f (sprint/s46). HEAD 5c7f1c9, tree `064143fa67b4fce7768be822b6cc3cb9b2610f30`, worktree `/Users/atulsharma/Documents/projects/tensorbee/ocelli-wt-f-216`, branch `work/f-216-xai`. No unstaged tracked change. The only untracked paths are the review files and `tools/oracle/out`, a symlink into the main checkout dated 5 Nov, which was there before this pass.
**Author / reviewer**: 5c7f1c9 was written by the lead and a Claude implementer. The merge f799b90 is the lead's. This pass is a separate Claude reviewer that wrote neither commit and has not edited the tree. The author identity comes from the brief, not from the trailer.
**Result**: 1 defect, 0 smells, 2 nitpicks

## Acceptance challenged

- **The globals block names exactly what the file uses.** I checked this by deleting the block in a scratch config, not by reading the code. With the block removed, `no-undef` fires on all 18 names and on nothing else. So every entry is needed and none is spare. Removing the `node:buffer` import fires `no-undef Buffer` 22 times in `viewer_work.mjs` and 2 times in `viewer_work_test.mjs`.
- **Writable only where the driver assigns.** Setting each of the three writable entries to `readonly` gives one `no-global-assign` each: `overrideWindow` at :674, `activePreset` at :675 and `cacheBytes` at :739. None of the 15 readonly names is assigned as a bare identifier anywhere in the file. The only assignments are the member writes `window.fetch` (:503), `window.requestGrey` (:524) and `window.drawGrey` (:530).
- **Each page name is a real top-level binding in one classic script.** `page.html` has one `<script>` tag with no attributes, from :733 to :3545. All ten names are declared at column 0 inside it: `viewIndex` :848, `cache` :851, `cacheBytes` :858, `overrideWindow` :879, `activePreset` :881, `postPreview` :1204, `currentImage` :1605, `showIndex` :1667, `requestGrey` :1862 and `drawGrey` :2054. Top-level `let` and `const` live in the realm's global lexical environment, so a function that Playwright evaluates in the main world can reach them by name. The driver's existing comment at :966 makes the same point from the other side: a `let` binding is not a property of `window`.
- **The block's glob reaches only `viewer_work.mjs`.** I linted the same probe line through stdin under five other names: `tools/bench/tests/viewer_work_test.mjs`, `src/registry.mjs`, `src/browser_session.mjs`, a non-existent `src/viewer_work_extra.mjs` and `run.mjs`. The probe uses `overrideWindow = 1`, `document`, `window`, `MutationObserver` and `drawGrey`, and all five names are still refused in each of those files.
- **Discriminating case for the split the bench block claims.** I appended a node-side `export function nodeSide() { return document.title + window.name; }` to `viewer_work.mjs`. Lint reports 0 messages. In a second case I deleted the `cacheBytes` parameter from the node-side `pageHeapBound`. The current config reports 0 messages, while the config without the block reports `139 'cacheBytes' is not defined`. This is D1.
- **The merge.** `git merge-tree --write-tree 952e9ad sprint/s46` gives tree `8b14f3c1`. Its only conflict is `docs/runbooks/guard-verification.md`. `git diff 8b14f3c1 f799b90 -- . ':!docs/runbooks/guard-verification.md'` is empty, so outside the runbook the merge is exactly git's automatic result. The plain byte comparison the brief suggested, of `952e9ad..f799b90` against `c07ef382..sprint/s46`, differs at line 4701. That is expected: both sides touched `README.md`, `use.html`, `ci/guard-probe-budget.json` and `scripts/guards/catalogue.py`, so those files' diffs carry F-216's own hunks as context. I read both sides of the three non-catalogue files and they change separate lines without contradicting each other. The census accepts the merged catalogue and budget. The runbook has no conflict markers, and `--check-runbook` exits 0, which is a byte comparison against `runbook_body(render_table())` (`guard_census.py:228-236`). sprint/s46 did not touch `eslint.config.js`, `package.json` or `tools/bench`, so the lint failure was already present at 952e9ad. Passes 1 to 12 did not run the lint gate.
- **The Buffer imports change nothing at runtime.** Under node v24.13.0, `import { Buffer } from "node:buffer"` is `=== globalThis.Buffer` (true). No local binding named `Buffer` is shadowed.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `npm run lint` | tree 064143fa | now | 0 |
| `bin/ocelli.sh gate lint` | tree 064143fa | now | 0, includes the 4 wasm-view rule tests, ALL GREEN |
| `node --test tools/bench/tests/viewer_work_test.mjs` | tree 064143fa | now | 0, 14 pass |
| `python3 scripts/guard_census.py` | tree 064143fa | now | 0, 1128 refusals, 0 watched by nothing |
| `python3 scripts/guard_census.py --check-runbook` | tree 064143fa | now | 0 (a first attempt exited 2 because my shell loop passed the flag as part of the file name, rerun correctly) |
| `python3 scripts/prose_check.py` | tree 064143fa | now | 0, clean over 864 files |
| `python3 scripts/source_provenance_check.py` | tree 064143fa | now | 0, clean over 1718 files |
| eslint 9.39.5, `-c` scratch copy identical to `eslint.config.js` | `target/f216-p13-mut/` | now | 0 on both F-216 files, so the scratch route resolves globs the same way |
| block deleted from the scratch config | same | now | 1, `no-undef` on exactly the 18 names |
| each writable entry flipped to readonly, one at a time | same | now | 1 each, `no-global-assign` at :674, :675 and :739 |
| stdin probe under five other bench file names | real config | now | every probe name refused in every file |
| node-side `document` and `window` appended to `viewer_work.mjs` | real config, stdin | now | 0 messages (D1) |
| `pageHeapBound` with its `cacheBytes` parameter removed | real config against the config without the block | now | 0 messages against `'cacheBytes' is not defined` |
| `Buffer` import line removed | real config, stdin | now | 22 and 2 `no-undef Buffer` |
| `git merge-tree --write-tree 952e9ad sprint/s46` against f799b90 | merge parents | now | only difference is the runbook |

The scratch directory `target/f216-p13-mut` was deleted afterwards. The tree is still `064143fa`, HEAD is still 5c7f1c9, and `git status` shows the same paths as at the start apart from this report.

## Defects

### D1, the bench block's stated split is now false for `viewer_work.mjs`

**Where**: `eslint.config.js:414-417` (unchanged text), made false by the new block at `:433-467`.
**What**: The bench block says the split against the page block runs both ways so that "`no-undef` then catches a driver file reaching for `document`". `viewer_work.mjs` is a driver file under that block's glob, and 5c7f1c9 grants it `window`, `document`, `MutationObserver`, `KeyboardEvent`, `CanvasRenderingContext2D` and ten page bindings for the whole file, including its node half. The new block's comment explains why the browser names are there. It does not say that the split above no longer holds for this file, so the older sentence still reads as a guarantee.
**Why it is wrong**: microscope section 3. A claim in prose that is false is a defect, and this one is a claim about what a guard catches.
**Evidence**: a node-side `return document.title + window.name;` added to `viewer_work.mjs` lints at 0 messages. Removing `pageHeapBound`'s `cacheBytes` parameter leaves the body reading a name that only the writable page global now satisfies. The current config gives 0 messages, and without the block the result is `139 'cacheBytes' is not defined`. That particular slip is still caught at runtime by `viewer_work_test.mjs:60-82`. The `measure*` functions that launch a browser do not run in the floor, so a node-side slip there would show up only on a bench run.
**Scope and regression**: present at HEAD 5c7f1c9 and limited to this one file, since the glob probe shows every other bench file still refuses the names. The other three driver blocks (`offscreen-canvas.mjs`, `tools/tier-b`, `tools/swiftshader`) accept the same mixed grant, but none of them claims a split. There are two fixes. The smallest is to say in the new block, or in the bench block's sentence, that the split does not hold for this file, so that `no-undef` will not catch its node half reaching for these names. The alternative keeps the split: move the Playwright callbacks into a module under a browser-only block, which works because Playwright serialises the function source. No regression test is needed for a comment fix. If the callbacks move, the probe above (node-side `document` refused) is the regression check.

## Smells

None.

## Nitpicks

### N1, "the browser globals they use are named here" leaves out `setTimeout`

The callbacks at :589, :650 and :660 use `setTimeout`. It reaches them from the bench block's node list, not from this block. In practice the effective config names it, so lint behaves correctly. Only the sentence is loose.

### N2, carried from pass 12 N2 and N3

Nothing tests the registry-fault exit code, and a named subject the run cannot measure is dropped silently. Neither is touched by this diff.

## Verified clean

- **Pass 12 N1** (main's refusal is held only on the `--list` route) is unchanged and still does not block.
- **The new comment sentences**, apart from D1's omission. "Takes the node globals from the bench block above" holds: the config without the block flags no node name. "`fetch` is used on both sides" holds: node at :302, :1030 and :1040, the browser at :1095. "Loads one classic script" holds. "Top-level `let`, `const` and function declarations are reachable by name" holds. "The three the driver assigns are writable and the rest are read only" holds, as shown by the mutations. "Not a wasm memory view and does not touch the allowance list" also holds: the block has no `rules` key, and `assertTheBanIsIntact` ran when the config loaded.
- **No rule is disabled.** The block contains only `files` and `languageOptions.globals`.
- **No other file's lint changes.** The glob is an exact path, and the five-name stdin probe confirms it.
- **The Buffer imports**: same object as the global, and no runtime change.
- **The merge**: equal to git's automatic merge outside the runbook, and the runbook equals the render.
- **The commit message** "F-216, the bench driver's browser and page globals named for lint" has no em-dash and no semicolon. `prose_check.py` is clean.
- **Docs.** No README, `use.html`, `more.html` or LLD sentence describes the eslint bench split. The diff changes no user-visible behaviour.

## Limitations and handoff

- No bench, oracle or GPU job ran. No browser was launched. Ports 8742 and 8765 were not used. PID 98891 was not touched.
- `page.html` reachability is established from the spec semantics of the global lexical environment and from the column-0 declarations, not by running a Playwright callback.
- This is a bounded review of f799b90 and 5c7f1c9. It is not AC17 acceptance and not sprint readiness.
- Next action: the implementation owner fixes D1, either with the comment or by moving the callbacks, and a fresh independent pass reviews that change.
