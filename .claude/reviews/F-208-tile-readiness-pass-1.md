# Readout first-tile readiness repair, bounded independent source review

Reviewed 2026-11-09. Result: 0 defects, 0 smells, 0 nitpicks in the exact test-only diff. Root authored the repair. This reviewer authored neither the readout implementation nor this remediation. No runtime or hardware command was executed by the reviewer, and no canonical source, index or status changed.

## Exact source

HEAD is 9a4456b6a027d2fb1755a2f8bcccb5d64c8853f2. The reviewed working readout.mjs SHA-256 is 6dec9ef7b57f7f50d5bf59c3fd4ed3a1d9a786d33d2d1a2b3c630ca6efc09153. The diff is preserved beside this report. It adds one exported readiness helper, one held-painter adversary in emptyTileCase, and replaces the two width-only waits. No production page or timeout changed in that diff.

## Readiness

A fresh canvas has positive default backing dimensions before image publication. The new helper checks both backing extents, actual fitted CSS extents, current bounding extents and an opaque backing pixel. `paintGrey` supplies alpha 255. The page calls it, records the tile aspect and sizes the tile within one synchronous continuation after the awaited request. A poll cannot interleave inside that continuation. Consequently a separate aspect check is not required to reject this first-paint race when opacity and current geometry are both required.

The helper is intentionally suitable for the two fresh-page first-grid cases. It does not establish current-generation publication on a reused opaque canvas. The existing suite creates a fresh page separately for emptyTileCase and tileGapCase, so no retained prior tile image can satisfy this first-paint readiness check. No assertion about absolute pixel correctness is substituted for the existing magnifier pixel and visibility assertions.

## Adversary and restoration

The held-painter control intercepts the actual page-global paintTiles at entry. It records that the real tile painter was invoked, awaits a test-owned promise, then calls the exact original with the original arguments. It supplies no fake pixel, preview result or product state. The finally block restores the original function, releases pending invocations and removes the test gate before ordinary image/pointer assertions continue. Multiple invocations already waiting on the same promise safely call the saved original after release. The wrapper does not dereference the deleted gate after its await.

The helper is actually invoked while paint is held. A width-only mutation will accept the default backing store and raise the named unpainted-image assertion. This checks the consumer readiness guard even on a host where an early pointer does not miss the picture. After release, the test waits for the repaired helper, reads current geometry, performs real pointer moves, requires the magnifier to show, and retains the scrub-to-empty checks. The gap case also uses the repaired wait before computing the actual picture and strip coordinates. Existing timeout values and magnifier assertions remain unchanged.

## Evidence boundary

Root's separate delayed-paint observation reported default 300 by 150, zero CSS extents and alpha 0 while the old predicate passed. Early real pointer moves left the magnifier hidden, and after release the painted 128 by 96 tile still did not restore it. This is root-owned runtime evidence. The participant read its direct exit receipt, 0 in 0.759377 seconds, and received the actual detailed observation from root. The participant has not independently rerun it.

Runtime acceptance still requires the repaired full readout run and the actual old-predicate helper mutant failing semantically, followed by restored GREEN. Those are root-owned and were active or pending at this review. This source review must not be called a reproduced runtime GREEN or CI pass.

## Earlier inspection timing

The earlier analysis.md described the original width-only waits read from the pre-repair source. Its source-proof.json captured working hashes after root had begun the repair. Preserve that initial inspection rather than treating its working-file hash as an immutable old-predicate snapshot. HEAD 9a4456b6 and the preserved repair diff identify the original versus repaired code precisely. This report pins the actual repair working bytes separately and makes no invented in-run hash claim.

## Root runtime disposition, 2026-11-09

The actual held-paint experiment exited 0 in 0.759377167 seconds. While held, tile 4 retained its default 300 by 150 backing extent, zero fitted CSS extent and alpha 0. The old width predicate accepted it. Real pointer moves at that zero-sized rectangle left the magnifier hidden. After release, the 128 by 96 opaque image was fitted and the magnifier remained hidden. This reproduces the readiness race without an inferred Linux rendering defect.

The repaired full readout suite exited 0 in 14.551074541 seconds. An isolated module with only the helper restored to the old width predicate exited 1 in 9.498346209 seconds at `an unpainted tile was accepted as a displayed image`. Its relative imports were replaced by absolute imports solely to execute the private module. The unchanged canonical repaired module then exited 0 in 13.940164042 seconds. All existing magnifier, scrub, fault, layout, drag and tile-gap assertions remain. Browser Chromium was 151.0.7922.34. Direct commands and own exit receipts are in `readout-held-paint-old-predicate`, `readout-painted-tile-repair-green`, `readout-painted-tile-old-size-mutant` and `readout-painted-tile-restored-green` under the private S46 evidence directory. This is local synthetic-browser evidence, not the still-owed qualification CI rerun or operator acceptance.
