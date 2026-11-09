# F-244 and monochrome measurement LLD, independent review

Reviewer: Codex `/root/f225_review`. Author: root Codex. Result: 0 defects,
0 smells, 0 notes in this bounded source and record review. Exact draft and
receipt hashes are in `measurement-lld-review-identity.json`.

## F-244 isolated release size

The base commit and tree, reviewed tip tree, two direct gate exits and four
target-qualified dependency-tree exits match the actual pair measurement.
Both private target directories and two-job build settings are recorded.
Neither source tree changed during its run.

Raw module byte counts, deterministic gzip counts and all module digests in
the draft match the measurement. Render is 2,202,907 bytes and decode 16,455
on both sources, with equal digests. All recorded graphs explicitly target
wasm32-unknown-unknown and report no forbidden DICOM crate. Both budget hashes
are equal and no size acceptance command was recorded. The claim isolates
F-244's source pair instead of attributing the full later viewer stack to it.
It is not claimed as whole-feature or sprint acceptance.

## Same-host monochrome observations

The reread draft contains the completed source-identity and raw-hash additions.
The original measured tree is
`68702951938d8d7194e8862739d0a51ffcf60244`, HEAD
`5451cb7d4ca6c9e1304b96e1e95c493acbbbd060`. Git object inspection confirms
that the only delta to that old product is `tools/bench/src/viewer_work.mjs`.
That helper is byte-identical to the helper in measured assembled tree
`9b586d477dc5172969253f6a9d8bc44437214a63`. Its SHA-256 is
`db365bb51fb29a99d8fbd02849667c4b060bf7c0f7a7cadb66be82a8fa4e9360`.
The draft correctly separates these observations from later byte-store edits.

Both actual run SHA-256 values match the text. Node, Playwright, Chromium,
full host identity and all five subject values agree with the two raw runs.
All declared absolute threshold flags are true. Blank-scroll counts,
restart-recovery flags, body-post counts, DX medians and informational PET
first-image figure agree with the records. No PET threshold was invented.

The existing comparator verdicts are preserved accurately. Assembled first
image and window updates are moved worse outside their unchanged 25 percent
bands, while the original open figure moves in the improving direction past
its 6 percent band. The draft does not convert absolute-budget success into
a relative-regression pass. Earlier Node 26 observations remain incomparable
against the historical instrument, rather than being used as valid relative
acceptance. Load and external-activity wording does not claim a fully idle host.

The draft's source and record provenance is complete for these explanatory
observations. This review does not claim a fresh product run, a later exact-tree
verification, completed operator acceptance, or resolution of the relative
regressions. No source or draft was edited by the reviewer, and no native,
wasm, profiling, timing, browser or GPU job was started during root's hold.
