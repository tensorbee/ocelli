# F223 independent source review, tools/bench/src/comparison_run.mjs

Reviewer: GLM-5.3 Flash (DGX), OpenCode session, read-only source review. Date 2026-11-08.

## Source identity

- Workspace: /private/tmp/ocelli-s46-f223-refusal-cpu-glm-review (immutable snapshot, only reads performed)
- Tree under review: 4605d625371ff02b0617a8e35e8f2fdb420972a9 (source.json, authoritative)
- HEAD anchor: 22e9aab5d3f8ad1aa452ac9de17bed8688129e43, verified equal to the workspace HEAD
- Base: e4a3b620ea853f22a1bf1824008f134c7cdf9ae3
- Author: Codex glm_codec_merge
- Reviewed: full comparison_run.mjs at the review tree plus the base diff of that one file. The diff is 9 hunks, 49 insertions, 8 deletions
- Supporting reads: comparison.mjs (renderReport 1740, sharedBody 1686, versionsSection 1574, measurementSection 1507, scanRecordForDenied 1012, scanForDenied 955, cpuFigure 712), viewer_work.mjs (SERVER_PERMITS 110), run.mjs (caller 356 to 366), comparison_colour.mjs (consumer validation 206 to 207, tools call 264)

## Verdict

0 confirmed defects. 2 low smells and 2 low notes. None blocking. Zero findings was a legitimate outcome and the bounded contracts are met on every traced path.

## Contract conformance, traced

- Caller flow and exit codes. run.mjs 356 to 366 prints error.safeMessage and returns error.exitCode or 2. The three refusal kinds map through RUN_REFUSALS (comparison_run.mjs 27 to 31, thrown at 819 to 823) to exit 2, 2 and 1, and the three messages and exit codes match the base throws character for character (base lines 445, 686, 735). No other throw site of these messages exists.
- Environment refusal record. pixelPrecheck returns the refusal with side metadata (466, 467). executeComparison writes the ignored pair only (721 to 722) with state refused_environment, four per-side Chromium version and renderer strings (716 to 717), empty metrics, no pair_verdict, no trials, no machine, no defaults. sharedBody omits the pixel section when pair_verdict is absent (comparison.mjs 1703 to 1705) and renders the refusals list (1753 to 1754), so the record carries no pixel-pass or timing claim.
- Side metadata privacy. The record passes scanRecordForDenied, the JSON scan and the text scan inside safeOutputs (118 to 124) before any write. versionsSection escapes pipes (comparison.mjs 1588). A renderer string carrying a denied value throws before writePair and the finally removes all five output names (792 to 795).
- Cleanup order and partial output. The refusal record is written before the finally. The finally removes it when interrupted, when context.cleanupFailed is set (including a failed actor close inside pixelPrecheck, comparison_run.mjs 327) or when any cleanup promise rejects, and sets outcome.cleanupFailed (791 to 795). runComparison then throws the cleanup error (818) before the refusal check (819), so a cleanup failure wins and the record does not survive. writePair rolls back a partial replacement (138 to 150). The lock refusal at 675 precedes the removal of previous outputs at 690, so prelaunch refusals keep their no-new-output behavior and a previous record survives a lock refusal.
- No tracked publication of a refusal. Neither refusal path sets pendingPublication, and runComparison throws before its publishComparison call (819 before 824). A tracked pair is untouched even with --publish.
- Invalid publication. The raw invalid record is deny-scanned and written first (767 to 768), then the publish check returns the invalid_publication refusal (771). After clean cleanup the privacy-scanned ignored invalid record is retained. A real cleanup or privacy failure removes it and exits 1 through the cleanup error.
- Bounded CPU baseline. cpuBaseline (186 to 196) makes at most three total attempts, each a fresh processInventory plus cpuSnapshot, traced as attempts 1, 2 then throw at 3. The retry condition requires both the comparison marker (39) and the exact message "a process exited during CPU collection" (179). Every other error refuses immediately, including the inventory no-root error (164), exec failures and browser-closed errors. Persistent incoherence throws before startProcessSampler and before actor.navigate in both consumers, memorySession (554 to 555) and measureColourMemorySession (393 to 394), so no navigation, sampling or measured action is repeated. End snapshots keep the direct non-retried cpuSnapshot (399, 568). collectTrialSamples, trial counts, summarise, cpuFigure and all thresholds are untouched, and both the default and colour consumers use the same helper.
- decodePermits. SERVER_PERMITS is 2 (viewer_work.mjs 110, documented as the folder viewer's decode permits), so the metadata value is unchanged and now single-sourced.

## Findings

### S1, comment overstates the retry count. Smell, low

comparison_run.mjs 183 to 185. "A missing PID retries both inventory and counters, at most three times." The loop performs at most three total attempts, which is at most two retries. The code satisfies the approved bound of at most three fresh complete reads. Uncertainty: the sentence is ambiguous rather than wrong, and the code is authoritative and correct. Suggest "at most three attempts" or "at most two retries".

### S2, refusal-kind whitelist restates RUN_REFUSALS keys. Smell, low

comparison_run.mjs 712. The literal ["environment_mismatch", "software_after_headed"] duplicates the RUN_REFUSALS keys minus invalid_publication instead of deriving from them. A future kind added to RUN_REFUSALS alone would be refused here as unrecognized. Behavior is loud and safe (comparisonFailure 2, no record written, outputs removed). No drift today, this is a maintenance risk only.

### S3, injected checkPixels without metadata fails as exit 1, not exit 2. Note, low, injected services only

comparison_run.mjs 709 and 716. `const info = checked.metadata` is unguarded. An injected services.checkPixels returning the base-era shape { needsHeaded: true } without metadata on both attempts, or an environmentRefusal without metadata, reaches info[0].version and throws a TypeError, which the catch at 777 to 783 wraps as the generic "comparison could not complete" with exit 1 rather than the refusal exit 2. The real pixelPrecheck always attaches metadata (466, 467), and the throw precedes safeOutputs and writePair, so no partial record survives. Uncertainty: reachable only through separately authored controls, no real-flow impact today.

### S4, refusal record carries headed but not software. Note, very low

comparison_run.mjs 713 to 720. An environment mismatch under an explicit --software run records the renderer strings and headed but not that --software was in effect. The approved contract requires only the version and renderer strings per side, so this is completeness rather than conformance. No leak and no false claim. Uncertainty: a judgement call on diagnostic completeness.

## Method note

Full-file read at the review tree, base diff of the one reviewed file, and targeted reads of the four named neighbours only. No tests, builds, browser, GPU, oracle or network. Existing tests predate these changes and controls are authored separately, so absent new tests were not treated as a defect or as acceptance proof. No reviews, progress notes, scratch, logs or later rulings were read.
