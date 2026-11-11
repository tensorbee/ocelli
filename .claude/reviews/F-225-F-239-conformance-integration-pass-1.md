# F-225 and F-239 conformance integration, pass 1

Reviewer: Codex. Date: 2026-11-08. Result: 0 defects, 0 smells, 0 nits.
This is a bounded independent review of the root-authored test adaptation,
not a new feature review or a performance acceptance. The reviewer did not
write this delta and launched no browser or viewer during this review.

## Exact scope

Base tree `3ecb1239` to input tree
`ffe0568cf089d189702678a308d4dff118489d3a`. The sole changed path is
`examples/folder-viewer/tests/browser/conformance.mjs`.
Its SHA-256 is
`f583ddd910ac5245e02e2a667009e3b88b5496156bd97546e2278383ab5e35a8`.
The inspected runtime candidate has byte-identical test source. Its restored
`examples/folder-viewer/src/page.html` is byte-identical to the frozen tree,
with SHA-256
`f35a6b4f358e147d61f1ab986838fb7518b7a819e67b6be0be6620cd191204fe`.
Both identities were checked directly after the restored run.

## Assessment

F-239 now draws this supported native RGB frame. Waiting for a colour fault
was an inherited test expectation that no longer described the product.
The adaptation fills the fixture with literal RGB `10,100,200`, waits for an
actual 32-pixel-wide cell-0 canvas paint with the fault hidden, then reads
pixel zero from that canvas and compares exact RGBA `10,100,200,255`.
The expected value does not call production conversion or repeat its formula.
The refused-file branch still requires a visible fault. The unconditional
badge check still requires four findings for each branch without opening the
sheet. Existing report timing, privacy, request and stale-folder controls
are unchanged.

## Evidence inspected

The direct source commands were `git diff 3ecb1239 ffe0568cf089d189702678a308d4dff118489d3a -- examples/folder-viewer/tests/browser/conformance.mjs`
and `git show ffe0568cf089d189702678a308d4dff118489d3a:examples/folder-viewer/tests/browser/conformance.mjs`.
Both exited 0. Independent source hash comparisons exited 0 and matched both
runtime files to the frozen input.

Root ran `node examples/folder-viewer/tests/browser/conformance.mjs` in
`/private/tmp/ocelli-s46-integration-candidate`. The reviewer read the direct
subprocess JSON receipts and complete relevant log tails under
`/private/tmp/ocelli-s46-codex/`:

- `integration-conformance-colour-adaptation.json` and `.log`: exit 0,
  14.375332958 seconds. Colour painted and badge 4. Refused file faulted and
  badge 4.
- `integration-conformance-colour-badge-mutant.json` and `.log`: exit 1,
  31.680384999 seconds. Hiding the colour badge left the literal paint valid,
  but the test reported `colour: the badge reads null`. The refused badge
  remained 4. This is the intended semantic failure.
- `integration-conformance-colour-restored.json` and `.log`: exit 0,
  28.140650167 seconds. Both badges again read 4 with the correct separate
  painted and refused behaviours.

The combined identity receipt `conformance-colour-mutant-results.json` was
also read and agrees with these direct exits and source hashes.

The root mutation controller initially expected the error text to say
`badge reads 0`, whereas the actual hidden badge is represented by `null`.
That post-run controller assertion was wrong. It does not invalidate the
recorded direct node exit or the semantic failure, and root restored the
page in its cleanup path. The fresh green run and independent exact page
hash establish restoration. This is a coordinator control-message mistake,
not a product or GLM defect.

## Limits and ownership

Only this root-authored test adaptation is accepted by this bounded review.
The synthetic RGB file deliberately retains the existing CT report reproducer
so the four original conformance findings remain the subject of the test.
This is not a claim that its RGB pixel module is a conformant CT IOD.
No production source, model-owned benchmark path, index, canonical sprint
ledger or story status was changed by this review. The report is a scratch
handoff in the private ICC benchmark copy and is excluded from GLM authorship.
