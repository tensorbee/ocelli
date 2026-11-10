# F-245 page remote slice review, pass 3

**Reviewed**: unstaged worker files in /private/tmp/ocelli-s47-f245-glm, the same hashes before and after the pass.
- `examples/folder-viewer/src/page.html` sha256 prefix 2c9f4b7399f1c6fd, diffed against `claude-page-before-nits.html`.
- `examples/folder-viewer/tests/browser/dicomweb.mjs` sha256 prefix 761c29349c73b4ea, diffed against `claude-browser-suite-before-ac3.mjs`.

**Author / reviewer**: GLM authored the AC3 extension, and Codex coordinated the repairs. Reviewer Claude (root independent reviewer, subagent), who wrote none of the reviewed material. Copies were instrumented under /private/tmp/claude-501/f245r only.

**Result**: 1 defect, 0 smells, 0 nitpicks.

## Acceptance challenged

- AC3 pending stage: does the route hold the instance A request while the snapshot is taken?
- Whether the baseline wait can hang or mask a reset.
- Whether the lexical-global reads are real and the vacuity guard is effective.
- Pass 2 nits N1 and N2.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|--------------------------|---------------------------|------------------------------|----------------------|
| `node examples/folder-viewer/tests/browser/dicomweb.mjs` | worker tree | executed now | exit 0, `ok` |
| `suite3.mjs`, a copy with a counter in the route handler | as above | executed now | `ok`. Handler calls at the pending snapshot: 0. `heldUrl` is `.../dicom-web/dicom-web/studies/...`. The upstream had already served A once |
| `suite3fix.mjs`, the same copy with `${upstream.url}/studies/` | as above | executed now | handler calls 1, upstream A GETs 0 at the snapshot. With the existing `releaseFetch(); await page.unroute(heldUrl)` it crashes with `route.continue: Route is already handled!`. With the unroute removed it passes |
| `MUT=pendingonly`, which writes `#status` while fetching and restores it on success and on refusal, on both copies | page via `page.route`, CSP removed, Chromium local network checks disabled | executed now | killed on both copies. In the unheld copy the kill depends on the native post-processing still being in flight when `readLocal` runs |
| p2 static probe | current page | executed now | at 1 and 60 rows, `#remote` scrollWidth 358 equals clientWidth 358, and `#wrap` stays 848x609 |
| `claude-browser-ac3-mutants.json`, runs v12-1 and v12-2 | coordinator | prior receipt | four mutants killed, restored `ok` twice |

## Defects

### D1, the pending hold never matches, so the pending stage is a timing race

**Where**: `examples/folder-viewer/tests/browser/dicomweb.mjs:367`, with `:139`.
**What**: `upstream.url` already ends in `/dicom-web`, so `heldUrl` doubles the segment and `page.route` never fires. Instance A is fetched unheld. The pending snapshot is still taken while `#remote-state` reads `Fetching the instance.`, but only because hashing and the native POST had not finished in time. On a faster run, the fetch can complete between the `waitForFunction` poll and `readLocal`. A pending-only mutation that is undone on completion then escapes, or the wait times out. Fixing the URL alone exposes a second fault: `page.unroute` straight after `releaseFetch()` races the handler's `route.continue()` and throws `Route is already handled!`.
**Why it is wrong**: AC3 requires preservation through the pending stage, and a hold that never runs is a guard nothing executes. The coordinator's `keyImages.clear while pending` mutant is permanent, not pending-only, so the `afterFetch` comparison can catch it whether or not the hold works.
**Evidence**: the handler counter above, the fixed-URL crash, and the fixed-URL run without unroute.
**Scope and regression**: test only. Use `${upstream.url}/studies/...`. Then either await the handler's continuation before `unroute`, or unroute after the final state. Also assert that the handler ran and that the upstream had no A GET at the snapshot.

## Verified clean

- **The baseline wait cannot mask a remote reset.** Every remote code path except `probeRemoteSource` runs after the baseline, and the probe runs before any local study, mark or key image exists. A wait that never settles fails at the 30 s timeout rather than passing.
- `readLocal` reads `notes`, `keyImages`, `sessionDocuments`, `cache`, `cacheGeneration` and `checks` lexically with no fallbacks. A missing name throws and fails the run. The `marks >= 1` and `keys >= 1` guard prevents a vacuous baseline.
- **N1 closed**: the long UIDs wrap inside the panel.
- **N2 closed**: `#remote` is now z-index 6, below `#modal` at 7. `#note`, also at 6, comes later in the DOM and paints above it, which is harmless.

## Limitations and handoff

- Still uncovered:
  - an SR document load before the remote flow (`sessionDocuments` is compared but stays empty or unchanged unless loaded)
  - AC6 tier independence
  - the AC7 More matrix row and docs
  - `viewer-browser` registration in `bin/ocelli.sh`
  - rapid-click races, which were checked by reading only
- Mutants ran without CSP. This is not acceptance of F-245.
