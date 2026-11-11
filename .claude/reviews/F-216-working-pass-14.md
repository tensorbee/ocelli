# F-216 review, pass 14

**Reviewed**: `git diff 5c7f1c9 HEAD`, which is the single commit 9d08494 "F-216, the lint block says the split does not hold for the driver". HEAD 9d08494, tree `342a5962a13af77a2b45ed663adcf6f0461bfd52`, worktree `/Users/atulsharma/Documents/projects/tensorbee/ocelli-wt-f-216`, branch `work/f-216-xai`.
**Author / reviewer**: the brief says the sprint lead wrote 9d08494. This pass is a separate Claude reviewer that did not write it and has not edited the tree. The author identity comes from the brief, not from the trailer.
**Result**: 0 defects, 0 smells, 1 nitpick

## Scope of the change

`git diff 5c7f1c9 HEAD --name-only` lists only `eslint.config.js`. The diff is 5 insertions and 2 deletions, all inside the comment of the `tools/bench/src/viewer_work.mjs` block (`eslint.config.js:434-441`). No `files`, `globals` or other key changed, and no other block changed.

## Pass 13 findings

- **D1 closed.** The comment now says "These names cover the whole file, so the bench block's split does not catch this file's node side reaching for `document` or a page name." This is the smaller fix pass 13 offered. The bench block's sentence at `:414-417` is unchanged, and the exception for this file is stated in the very next block.
- **N1 closed.** "the browser globals they use, other than the bench block's timers, are named here" now accounts for `setTimeout`.

## Every sentence in the new comment, executed

| Sentence | Check | Observation |
|---|---|---|
| The callbacks' browser globals, other than the bench block's timers, are named here | Scratch config with the bench block's `globals` emptied, linted on the real file | `no-undef` fires only on `process` :347, `setTimeout` :352, :386, :607, :652, :663, :1052 and `clearTimeout` :365, :371, :377, :391. The only callback lines are :607 (`scrollOnce`) and :652 and :663 (`plainTimerCeiling`), all `setTimeout`. :347 to :391 are `startViewer` and `stopViewer`, and :1052 is the node RSS sampler loop. No callback uses `process`, `console`, `URL` or `clearTimeout` (`URL` appears at :356 and :378 only inside error strings). The callbacks' other free names (`globalThis`, `Promise`, `Math`, `Uint8Array`, `Number`, `String`, `Boolean`, `Error`) are ECMAScript builtins and come from neither block |
| The names in the block are needed | Scratch config with the block deleted | `no-undef` on exactly 18 distinct names, the 18 the block lists |
| `fetch` is used on both sides | grep | node :302, :1030, :1040, browser :1095 |
| These names cover the whole file, so the split does not catch the node side reaching for `document` or a page name | A scratch copy of the driver under `target/f216-p14/` with `export function nodeSide() { return document.title + window.name + String(drawGrey) + String(overrideWindow); }` appended, linted through stdin as `tools/bench/src/viewer_work.mjs` | Real config: 0 messages, exit 0. Control config without the block: `no-undef` on `document`, `window`, `drawGrey` and `overrideWindow` at :1136. So the probe discriminates, and the sentence is true |
| The file runs under node and takes the node globals from the bench block | The emptied bench config above | Every node global the file uses (`process`, `setTimeout`, `clearTimeout`) is satisfied only by the bench block |
| Not a wasm memory view, does not touch the allowance list | Block keys | Unchanged from pass 13: no `rules` key, and `assertTheBanIsIntact` ran when the config loaded |

An identical copy of `eslint.config.js` passed with `-c` gave 0 on the real file, so the scratch route resolves globs the same way the real config does.

## Evidence

| Command | Input | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| `npm run lint` | tree 342a5962 | now | 0 |
| `bin/ocelli.sh gate lint` | tree 342a5962 | now | 0, ALL GREEN, 1 gate |
| `python3 scripts/prose_check.py` | tree 342a5962 | now | 0, clean over 864 files |
| `python3 scripts/source_provenance_check.py` | tree 342a5962 | now | 0, clean over 1718 files |
| scratch config probes above | `target/f216-p14/` | now | as tabled |

`target/f216-p14` was deleted afterwards. `target/f216-runs` was already there (6 Nov 14:51, ignored) and was not touched. After the pass the tree is still `342a5962`, HEAD is still 9d08494, and `git status` shows the same untracked paths as at the start, plus this report.

## Defects

None.

## Smells

None.

## Nitpicks

### N1, "timers" is plural and the callbacks use one of them

The callbacks use `setTimeout` and never `clearTimeout`. The phrase excludes a category rather than claiming both are used, so it is not false.

### Carried

Pass 12 N2 and N3 (nothing tests the registry-fault exit code, and a named subject the run cannot measure is dropped silently) are still open. This diff does not touch either.

## Limitations

- No bench, oracle, GPU or browser job ran.
- This is a bounded review of 9d08494. It is not AC17 acceptance and not sprint readiness.
- The commit message has no em-dash and no semicolon.
