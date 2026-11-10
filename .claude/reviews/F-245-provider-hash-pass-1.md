# F-245 provider hash review, pass 1

**Reviewed**: Exact private two-hunk change from QIDO product 9d3badf9a9b1a5c7f1ce10a734402c2222051ae8d86355155b18328d00b0b311 to 1eb13136c682a95adbdc8ab4ab2d5e5fd208d76e324884157e4cf38d82a70671. No WADO implementation is included.

**Author / reviewer**: GLM authored the repair and own inventory report. Worker Codex coordinated application. Root Codex independently verified the entire two-hunk composition against the previously reviewed snapshot and replayed it in a private copy.

**Result**: 0 defects, 0 smells, 0 nitpicks in this narrow delta.

## Acceptance challenged

The hash helper must digest the actual owned ArrayBuffer without allocating another explicit payload copy. The resulting digest and the existing QIDO request lifecycle must remain correct. The caller still creates its one owned copy, then passes that exact ArrayBuffer to both hashing and the native request. No cast supplies the ownership claim.

## Evidence

Root independently reconstructed the full proposed bytes from the two declared replacements. `f245-root-provider-review/hash-delta-query29.json` and its log record Node 24.13.0 and Vitest 4.1.11, exit 0 with 29 tests: 28 author QIDO controls and the root live-slot consumer. This is a fresh replay on 1eb13136, not reuse of the earlier 9d3 receipt.

Prior worker receipts `provider-owned-buffer-query28.json` and `provider-owned-buffer-test-tsc.json` record QIDO 28 and the forced actual test-project build, both exit 0 before the missing-WADO-method test was added. The new WADO test's missing-API RED remains separate.

## Verified clean

Only the private helper signature/body and its single call site change. The helper now passes the supplied ArrayBuffer directly to WebCrypto. The digest-to-hex conversion and caller byte content are unchanged. There is no public API, cap, tolerance, pool or legacy client change. The raw own report's two defect labels describe correct repaired behavior, not failed invariants. Its corrected inventory explicitly retains raw plus owned, two accepted payload buffers per live operation.

## Limitations and handoff

This proves the narrow source change and QIDO regression behavior. Future WADO composition must preserve at most two provider-owned accepted payload buffers per live operation plus one staged buffer. No physical heap, RSS, crypto-internal or network-buffer bound is measured here. WADO, native HTTP and page integration remain outside this pass. GLM owns subsequent implementation and must return its changed material for review.
