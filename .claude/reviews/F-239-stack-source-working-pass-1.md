# F-239 viewer stack source union, working pass 1

Reviewer: root Codex, independent of the coordinator who resolved this union.
Scope: source tree `933e84f7cadedd9a0afbdfa54ec4fa947790a188`.
Result: 0 defects, 0 smells, 0 nitpicks in the source reconciliation scope.
Colour viewer implementation and acceptance are separate work.

The inputs are the reviewed codec and library union plus the approved readout
table correction at `1500cd542d13c6dd570a986c1b1161ffba8d2585`, and the
reviewed F-215, F-225 and folder CSP stack at
`ec185b3fa8ab3c4f01a5a85c5ba9d8ea47da68c4`, from ancestor
`4458d5fec5b2150ba06b5eb03cf8a192100e155e`. The coordinator applied the
complete 385-path source delta and resolved 39 paths, with no commit.

The independent root probe pins immutable trees, rather than reading the live
index. It verifies all 2021 one-sided or identical blobs and modes. Eleven
viewer runtime, test and benchmark files match the reviewed CSP source exactly.
The F-239 and F-243 plans, codec registry tests and corpus generator and its
tests retain the reviewed destination bytes. DICOM exports retain the palette,
ICC presence, conformance and cine providers together. All 36 gate declarations
remain present, the budget count agrees, and both the viewer build check and
conformance browser invocation remain in the executable gate recipe.

Root inspected the conflict choices, the complete union diff, codec and library
policy retention, and the one-sided guard additions. The coordinator's full TSV
and vendor pin verification and successful folder all-targets check are retained
as their receipts. The new root immutable-tree probe exits 0. This review does
not substitute for the later colour server, browser or feature profile checks.

Evidence: `/private/tmp/ocelli-codex-f239-csp-union/` and
`/private/tmp/ocelli-s46-codex/csp-colour-union-independent-root.json`.
