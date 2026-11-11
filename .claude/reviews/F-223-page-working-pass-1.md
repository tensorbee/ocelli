# F-223 comparator page, working review pass 1

Reviewer: Codex, independent of the local GLM page author. The review covers
only the four original GLM page, helper and helper-test paths preserved under
`/private/tmp/ocelli-s46-codex/f223-comparison-page-glm/initial-output`.
Their exact hashes are in that directory's `identity.json`. The original
`bench.mjs` hash is
`0bd7a751deedcbda41088bbfe4f7fc166931c6e7af9e2257541e0ee3832e937d`.
The reviewer authored the separate F223 controller and its tests, so this is
not an independent approval of that material or its composition.

Result: 3 defects, 0 smells, 0 nits. The page is not review-clean.

## D1, pending work is reported as a blank presented frame

The ArrowDown handler changes the requested `index` and sets `blank = true`
before its asynchronous `setImageIdIndex` completes. Snapshot then describes
the request as a presented image, while the previous image is still visible.
A collector using those fields counts pending work as an actually blank stage
and can assign a new index to an old canvas. This violates the declared
presented-frame and blank-sample endpoints, without involving any duration
threshold.

Independent real-browser control: two generated 512 by 512 CT files have
opposing ramps. After the first image is committed, a synchronous ArrowDown
and snapshot occur before any new commit. The real canvas remains byte for
byte unchanged and visible. Snapshot nevertheless changes index 0 to 1 and
reports blank true. The assertion requiring retained visible pixels to remain
nonblank fails. Direct exit 1 is retained in
`f223-glm-page-pending-visible-red.json` and its log under the common temporary
receipt directory. The control closes its browser and server in `finally`.
No private operator input was used.

Lead direction is to track the committed index on IMAGE_RENDERED and sample
actual stage visibility, preserving retained visible pixels while a request is
pending. The controller's Ocelli blank observation also needs live visible
stage and fault sampling. That controller change is reviewer-authored and
requires a fresh independent reviewer.

## D2, a valid explicit LINEAR selector prevents the second frame committing

The permanent actual-browser extension waits for a second opposing-ramp CT
frame. It timed out after 120 seconds. A temporary served-page diagnostic,
without changing installed source, identified `Invalid VOI LUT function` in
actor update. The pinned loader derives `L` from the provider's `LINEAR`
string by taking `[0]`. This existing library defect is documented in the
oracle app's fileWindow comment. The GLM brief required reading that file but
did not explicitly call out the defect in its own prompt text. The comparator had omitted an adaptation and
its explicit first render concealed the missing later render.

Receipt `f223-permanent-page-browser.json` has direct exit 1. Separate
synthetic-only diagnostic receipts preserve the requested and actual image
identities and the swallowed actor-update exception. These are a real missing
commit, rather than a performance budget failure or a changed tolerance.
The lead approved a comparator-only provider adaptation for recognized valid
VOI strings, with package bytes and worker/cache defaults retained and the
adaptation disclosed in the record. Remediation is reviewer-authored and
requires a fresh independent review.

## D3, the unmatched per-file cap changes the selected first series

Root's fresh consumer challenge found that groupBenchSeries explicitly omitted
the folder viewer's FRAME_LIST_CAP while still receiving a usable frame count.
At frozen tree `053088bf7d76b03463fd945bbceb637d5f26831f`, two drawable files
in series 1 and 2 declare 65,537 and 65,538 frames. The actual production
folder groupSeries, numeric and drawable functions were extracted unchanged
and executed in a private VM. Both lists contain 65,536 images with omitted
counts 1 and 2, so the earlier series 1 wins the tie. The comparator lists
65,537 and 65,538 and selects series 2. The selected first files differ.

Independent receipts `f223-frame-cap-independent-red` and
`f223-frame-cap-selection-red` both exit 1. The permanent consumer fixture also
fails on the old helper at its literal expected counts. This is a different
measurement input, beyond a generic allocation warning. The bounded probe
uses about 131,000 entries and no pixel payload, huge count or OOM attempt.

The lead approved matching the existing per-file cap and omitted bookkeeping,
with no aggregate series cap. Provider number/default interpretation remains
unchanged. No new integer-string grammar or raw-frame parser is introduced.
This is the third original GLM consumer-policy defect, discovered through
root's fresh challenge. The Codex repair requires independent review.

## Checks and limits

Read all four original files and the installed MIT loader file-manager and
render-event paths. The original pure helper suite passed independently.
The GLM author also recorded its helper mutation and restore receipts.
The third-entry build passed, and the prior stack and volume bundle hashes
remained byte-identical. A real Metal Chromium 151 smoke opened the same
synthetic CT file in both viewers, captured their actual 512 by 512 RGBA
canvases and ran the unchanged release pair comparator. It passed with no
low-information qualification. Repeated folder and window actions also
completed with no page errors. Receipt: `f223-both-viewer-smoke.json`, exit 0.

These are bounded functional controls. No equipped timing acceptance, complete
pseudo-input matrix, full oracle corpus run or publication was performed in
this pass. The positive helper and pixel controls do not demonstrate every
page refusal. The original GLM output remains preserved for measurement of
what survived its self-review.
