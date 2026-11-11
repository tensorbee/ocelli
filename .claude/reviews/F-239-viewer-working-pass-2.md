# F-239 viewer harness amendments, working pass 2

Reviewer: Codex, independent of root's post-handoff amendments. Reviewed
staged source `e98bd53` against the preserved pre-amendment harness SHA256
`ac6be46622e59b2157645be01dd26e472e280c2e2754ca9ac5411a0d8d1564a8`.
Scope is only root's later amendments and the existing semantic receipts, not
self-approval of this reviewer's four-cell and mixed-series test additions.

F-239 viewer pass 2: 1 defect, 0 smells, 0 nitpicks.

## D1. Visible statistics assertion accepts only the unit

The assertion on #marks uses `/R[\s\S]*G[\s\S]*B/`. It accepts the string
`RGB`, so a page that renders only `reply.stats.unit` instead of its channel
statistics passes this assertion while showing none of the promised values.
The backend channel JSON assertions do not establish that the user sees them.
A Node negative control confirms the current assertion accepts unit-only RGB.
The hand fixture requires visible R mean 105.0, SD 95.0, min 10, max 200,
G mean 60.0, SD 40.0, min 20, max 100 and B mean 40.0, SD 10.0, min 30, max 50.
Root was notified to strengthen the visible check to these three hand values.

## Clean reviewed amendments and actual receipts

The new 2 by 2 native region fixture has the two original RGB pixels repeated
on its second row. Independent pydicom inspection confirms its exact metadata
and 12-byte Pixel Data. Its rectangle from .05,.05 to 1.95,1.2 exceeds the
normal 1-pixel height minimum, contains exactly the two row-zero centres and
excludes both row-one centres. Statistics remain based on the original two
hand pixels. The expected window bytes repeat the same independently supplied
row twice. Row-aware canvas assertions retain exact dimensions and RGBA.

The ICC change uses a normal mouse move into the canvas bounds and therefore
keeps the ink overlay's intended pointer route. The link setup uses real cell
and series-row clicks to force a fresh initial preview, preserving consumed
response and new-paint requirements. Printing an error stack changes only the
failure report.

Root's full combined release browser receipt exits 0 in 2.579093 seconds. Its
link-kind guard-removal control exits 1 at the unchanged CT pixel assertion,
with both CT states proven to hold the wrong 128/100 colour window. Its mixed
windowView-return-view control exits 1 in the colour prefetch assertion, with
a colour Lung preset request proven to answer 400. Both controls reach the
intended semantics, rather than failing to launch. The first original ICC
hover and region interaction failures and their ownership remain recorded in
the handoff. No additional browser was launched for this review.

Evidence is in `/private/tmp/ocelli-codex-f239-page-review/` and root receipts
`/private/tmp/ocelli-s46-codex/colour-browser-{link,mixed}-mutant.{json,log}`
and `colour-viewer-browser-link-setup.json`. Whole-feature and profile checks
remain outside this bounded review.
