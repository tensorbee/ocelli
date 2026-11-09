# F-239 code and library source union, working pass 1

Reviewer: Codex, independent of GLM source reconciliation. Reviewed source tree
`fddb8c9a3ee29f86c8e2b4e136da621b8ef86e8b`, with HEAD unchanged at
`055b8633f8e4d1ef98e07fe40721b8050ee92c35`.

Result: 0 defects, 0 smells, 0 nitpicks in the bounded source-union scope.
This is not feature acceptance or a sprint gate receipt.

## Inputs and authorship

The complete 291-path codec delta from common ancestor
`05d9aca259bd4dd651f32652230a487881cd253c` to staged codec source
`4d818b2dd7edd92ae59ec5c7d308a0a0d3803f2a` was applied with
`git apply --3way --index` to the reviewed library branch. The source includes
the clean codec pass 8 report. GLM resolved five documentation conflicts.
The nine overlapping Rust paths merged automatically. GLM then removed duplicate
imports in the reader tests after the first compile failed with E0252. This is
the sole delta from first-resolved tree `1bcfb09dac83442debeb121392579c54003f4173`.
Imported implementations remain their original authors' work.

Codex made no product correction. Two scratch harness corrections are recorded
separately: accepting git merge-file's documented conflict-count exit values,
and formatting a scratch PixelError through Debug rather than Display.

## Independent checks

`python3 /private/tmp/ocelli-f239-union-verify.py` exits 0 on the final tree.
It verifies 2000 one-sided or identical blobs and their modes against pinned
inputs, all columns of 27 decode digest rows, 109 manifest rows, 150 JPEG-LS
stream rows and 9259 verdict rows. All 15 patched vendor source/manifest pins
and both provenance hashes match. There are no unmerged entries or unstaged
tracked changes. The colour browser harness remains untracked agent work.

An independent three-way merge comparison confirms eight overlapping Rust
paths remain byte-identical to Git's clean merge. The ninth differs only by
the duplicate-import removal. All 32 plan AC rows and all one-sided deviation
rows preserve their input values. The five manually resolved documents retain
their input prose paragraphs, including the library tie-rounding, palette and
reader rules and the codec XYB rules. Historical wasm measurements remain
historical, not measurements of this combined tree.

Focused checks pass: pixel colour 25 and palette 12 tests, render colour bytes
8 and CPU 20 tests, reader frame-open 38 tests, codec HTJ2K 18, JPEG 2000 12,
JPEG-LS 33 and registry 19 tests. The pins check exits 0. A separate scratch
composition probe exercises native 4:2:2 refusal buffer preservation, exact
native decode and CPU display conversion, and exits 0.

`git diff --check` exits 0. `git diff --cached --check` exits 2 because the
inherited manifest rows end with the tab for their empty final field. The
source-union verifier confirms these bytes are unchanged. This failure is
retained as evidence, not relabelled as a successful gate.

## Remaining scope

The colour viewer is not implemented by this union. GPU, oracle, wasm,
whole-workspace and profile gates were not run. Root separately identified the
inherited plan protocol table's R 47.0 readout, which contradicts AC13's raw
R 46.7. Root authorized a table-only correction after this frozen review.
No pixel formula, tolerance or byte-rounding rule changes are required.

Evidence: `/private/tmp/ocelli-codex-f239-lib-union/`, including raw GLM log,
source snapshots, independent reports, actual check exits and sanitized session.
GLM model `dgx/GLM-5.3-Flash-EXL3`, 2026-11-08 12:40:37 to 12:58:33 UTC,
1076.146814 seconds, exit 0, no retries. Isolated usage is 78 steps,
208879 input, 20044 output, 32351 reasoning and 11826176 cache-read tokens.
Tool RPC success counts do not substitute for the command exits above.

## Follow-on CSP stack source preparation, 2026-11-08

Author: Codex coordinator. This section is a source reconciliation handoff,
not an independent approval of the coordinator's own resolution choices.

Root authorized the complete 385-path delta from original ancestor
`4458d5fec5b2150ba06b5eb03cf8a192100e155e` to reviewed CSP stack tree
`ec185b3fa8ab3c4f01a5a85c5ba9d8ea47da68c4`. Destination tree was
`1500cd542d13c6dd570a986c1b1161ffba8d2585`, the frozen GLM union plus
root's separately attributed R46.7 protocol-table correction. Complete stage
and all four untracked files or symlinks were backed up before mutation.
An initial backup attempt stopped on the .venv directory symlink, without
source mutation. The corrected backup preserves links rather than following
them. No additional GLM run was used because root requested bounded completion.

`git apply --3way --index` exited 1 and reported 39 conflicted paths, retained
in conflicts.json. Codex reconciled them and staged only the conflict paths.
The reviewed F215/F225/CSP viewer implementation takes precedence over the old
viewer branch. The final viewer Rust, page and tests are byte-identical to the
CSP source. The only folder-viewer differences from it are existing F243
JPEG-LS explanations in Use and More and F239's decoder-output-length Use
example. Diff inspection caught one moved-function duplication in preview.rs,
which was removed by preserving the reviewed source blob exactly. No colour
viewer behavior was implemented.

F239 rounding, palette and reader rulings and F243 JPEG-LS corrections retain
their reviewed destination values. F214/F215 rulings and the done F213 ledger
rows import source values. Ledger resolution selects exact input rows, with
no delivery-state invention. Decode digests merge by complete keyed input
rows. The guard catalogue retains both sides' additions. Its generated runbook
probe table was refreshed with --render-runbook, without changing prose or
the guard policy. The budget keeps the source viewer-build entry and the
destination gate count of 36. The source-only census confirms both.

Frozen combined source: `933e84f7cadedd9a0afbdfa54ec4fa947790a188`. HEAD
remains `055b8633f8e4d1ef98e07fe40721b8050ee92c35`. No commits were made.
`bin/ocelli.sh check ocelli-folder-viewer` exits 0 with two Cargo jobs. The
independent input-preservation script exits 0, checking 2021 one-sided blobs
and modes, all columns of the same four TSVs above, 15 vendor source pins and
two provenance hashes. All untracked paths and links are preserved, including
the independent colour browser harness.
`python3 scripts/guard_census.py --check-runbook` exits 0.
These checks establish bounded source coherence. Feature/browser/profile
acceptance and independent review of Codex's resolutions remain with root.

Evidence: `/private/tmp/ocelli-codex-f239-csp-union/`, including complete patch
SHA256 `8898cd9d906bb167d6bb9d8978316a9eca2a703e87311df673acc14a2d7b2fd8`,
preflight inputs, conflict list, per-block choices, complete destination
backup, actual command exits, structural verification and frozen identity.
