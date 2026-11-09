# Readout paintTiles ESLint declaration, bounded peer review

2026-11-09. Source-only result: 0 defects, 0 smells, 0 nitpicks. Root authored the reviewed delta. This reviewer authored neither the readout repair nor its ESLint declaration. Only this private peer record was written.

The actual diff adds `paintTiles: "writable"` to the existing folder browser-test globals block and adjusts its descriptive comment. The fresh readout adversary reads, wraps and restores that page-global function inside Playwright callbacks. A writable declaration accurately describes those assignments. The block remains restricted to examples/folder-viewer/tests/browser/**/*.mjs. No enforcement rule, wasm-memory allowance, production source or test assertion is weakened. The comment removes a stale function count and names the real held-painter control.

This is static review of the exact diff, not a lint execution or final-tree verification. Root owns the required lint/profile rerun after the previously failed profile. No compiler, browser, native or GPU job ran in this review.

Root direct check: the actual repaired readout file passed ESLint, exit 0 in 0.680876833 seconds. The permanent held-paint adversary and magnifier assertions are unchanged. Final feature verification remains required.
