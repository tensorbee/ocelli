# S46 review, pass 4

**Reviewed**: the staged pass 3 remediation on sprint/s46 a48a9172, staged tree 2a6d385a.
**Author / reviewer**: remediation by GLM with coordinator adaptations. Review by Claude, a read-only subagent that ran nothing, composed by the Claude coordinator.
**Result**: 2 defects, 5 smells, 5 nitpicks

## Acceptance challenged

- **Claim.** Pass 3's repairs leave every window setter guarded, tile mode never blank, and an image never drawn at two windows.
- **Combination not in the controls.** A second series loaded in tile mode while the first waits on its series window, a preset landing during a live W/L drag, and a series-window request that fails and is asked again.
- **Result.** The first breaks F-227's rule, a regression pass 3's repair introduced. The second and third are smells, below.

## Defects

**D1** `examples/folder-viewer/src/page.html` pumpMain. Pass 3 made the tile paint after the series-window wait unconditional. If series B is loaded in tile mode while series A's request is pending, the paint runs for B before B has its own series window, so B's tiles are drawn at each image's own window and then again at B's series window.

**D2** `docs/sprints/SPRINT_PLAN.md`. The follow-up rows were inserted between each table's header and its delimiter row in the S47, S48 and S50 tables, so none of the three renders as a table. `gen_sprint_plan.py --check` parses rows and cannot see it.

## Smells

**S1** `page.html` ensureSeriesWindow swallowed a rejected request and stored nothing, so the series stayed wanted, every scroll asked again, and a later success redrew shown images at a second window.

**S2** `page.html` chooseWindow's guard ignored a live W/L drag. A preset trial landing mid-drag replaced the drag, and a release with no further move lost it.

**S3** `tests/browser/window.mjs`. The out-of-order control released the held Lung request without waiting for Bone to commit, so the original page could pass by timing. The held-region control's settle read only the inflight map, which the region fetch is not in.

**S4** `tests/browser/auto.mjs`. The failed-series-window check accepted the placeholder Image label written before any grey arrives, and compared the tile div's width with the frame width.

**S5** `docs/sprints/CURRENT_SPRINT.md` said AC18's sprint profile passed on the closing tree. The pass on record is the tree before the review.

## Nitpicks

N1 SPRINT_PLAN's schedule note named the follow-ups F-274 to F-279 without F-280. N2 the Lung and Bone comment sat above the new `settled` helper. N3 the renamed JPEG XL test's inner comment described the old rule. N4 applyRegion bumped the choice before its own monochrome refusal. N5 the held-region control passes when no library build is served, matching the existing region check.

## Verified clean

The early choice bump does not cancel the region's own commit, in any order against a preset, drag, Reset, link or a second region. Success, refusal and same-series scroll during the wait paint at one stable window. `settled` cannot hang silently. F-280 agrees across BACKLOG, allocation, SPRINT_PLAN and the More roadmap, and the milestone sums add up. No added md or html line carries an em-dash or prose semicolon. The JPEG XL RgbLinear case matches the ISO 18181-1 colour encoding field order and values.

## Evidence

The sprint profile on tree 2a6d385a ran to completion during this pass and its result is recorded separately. It is stale for closure because this pass requires remediation.
