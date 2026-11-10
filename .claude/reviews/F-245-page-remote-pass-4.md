# F-245 page remote chooser and browser suite review, pass 4

**Reviewed**: `examples/folder-viewer/tests/browser/dicomweb.mjs` 2941956992f3579f against pass 3 at 761c29349c73b4ea. Page unchanged at 2c9f4b7399f1c6fd.

**Author / reviewer**: GLM authored the two repairs. Claude coordinated and reviewed this pass directly, because the pass 3 subagent reviewer specified the exact fix and its discriminating mutant, and this pass executes that mutant.

**Result**: 0 defects, 0 smells, 0 nitpicks.

## Pass 3 disposition

D1, the pending hold never ran because `heldUrl` repeated `/dicom-web`. GLM corrected the URL to `${upstream.url}/studies/...`, counts handler calls, keeps the `route.continue()` promise and awaits it after release and before `page.unroute`, and asserts at the pending snapshot that the handler ran exactly once and the upstream has no GET for instance A yet. A second GLM repair replaced an early call to the later `got` helper, which threw in its temporal dead zone, with an inline check.

## Acceptance challenged

The reviewer's undo-on-completion mutant writes `#status` only while the instance fetch is pending and restores it on success and on refusal, so only a real hold can see it. It now fails with `the pending fetch changed the local state` and nothing else. Restored, the suite passes.

## Evidence

| Check | Result |
| --- | --- |
| Suite, two consecutive runs | 0 and 0, `ok` |
| Undo-on-completion pending mutant | 1, pending stage only |
| Restored | 0 |
| Earlier AC3 mutants, resetChecks and dropCache on success, notes cleared on refusal, keys cleared while pending | each 1 |

Receipts `claude-browser-ac3-mutants.json`, `claude-browser-flow-run-v14-1.log` and `-2.log` in the evidence root.

## Still open

An SR document loaded before the remote flow, AC6 tier independence, AC7 More matrix row and documentation, registration under viewer-browser in `bin/ocelli.sh` owned by the F-268 shared slice, launcher runs and release measurements. Rapid-click races are verified by reading only.
