# F-239 viewer consumer, working pass 1

Reviewer: Codex, independent of root's page and the original browser harness.
Page source pinned at `0b182578cf4f63966cb6a236f4c9cd21bd29f4d3`, compared
with `ec185b3fa8ab3c4f01a5a85c5ba9d8ea47da68c4`. Original untracked colour
harness SHA256 `3c5652225d75b088346aa6c86226a47122bf2e68f5c8cb0eca44a61873a57184`.

F-239 viewer pass 1: 2 defects, 0 smells, 0 nitpicks.

## D1. Four images and prefetch send windows across image kinds

A series containing a CT followed by an RGB image, shown with the CT's Lung
preset, sends `X-Ocelli-Window-Preset: lung` to both tiles. The colour route
correctly refuses the preset, so the colour tile becomes a 1 by 1 blank
canvas. A numeric CT window is likewise applied as a colour channel window.
The new copyWindow kind guard protects linked viewports but not these paths.

The pinned source's actual paintTiles and requestGrey functions were executed
in Node with a minimal DOM and the approved colour-preset refusal response.
They produced the two headers above and blanked the RGB tile. Executing the
actual schedulePrefetch separately sends the same preset to RGB and constructs
its cache key with `lung`. Source functions and results are retained under
`/private/tmp/ocelli-codex-f239-page-review/`.

Root accepted the finding and owns a shared window-selection repair for
requestGrey, cacheKey and windowWord. The original viewport's supersession
tokens must remain effective even when the opposite-kind request uses default
window state. No production changes were made by this reviewer.

## D2. The browser harness does not exercise AC28

AC28 requires enabled window links between two RGB and two monochrome
viewports, in both directions, with the opposite kind unchanged. The original
harness creates only two viewports and never enables links.window. Its CT
window beside an unchanged RGB therefore passes even if linking is absent or
the kind guard is removed.

Executing the actual copyWindow with links enabled confirms the baseline
blocks CT to RGB. Removing only the kind guard makes the CT's -600/1500
window reach RGB. The permanent harness has no enabled-link path capable of
observing that mutation. Root accepted this defect and assigned this reviewer
only the harness remediation, which requires a fresh independent review.

## Controls and scope

The exact colour fill function produces `[10,20,30,255,200,100,50,255]` from
two RGB pixels. The exact response consumer accepts a valid RGB reply and
rejects missing, wrong or unsupported channels, fractional or infinite
dimensions and a short payload. The focused Node probe exits 0 after a scratch
harness correction added its missing headerNumber dependency. This correction
was not a product fix.

The review checked colour multiframe expansion, cache keys, frame selection,
window list suppression, kind reset, ICC notices, readout text passthrough,
prepared buffers and tiles. No browser, GPU, oracle, wasm or profile gate ran.
The server implementation is still being prepared independently. This report
makes no whole-feature claim and does not approve the reviewer's earlier CSP
source reconciliation.
