# S46 review, pass 3

**Reviewed**: the staged pass 2 remediation on sprint/s46 a48a9172, staged tree 3285ba2f.
**Author / reviewer**: remediation by GLM with coordinator adaptations. Review by Claude, a read-only subagent that ran nothing, composed by the Claude coordinator.
**Result**: 1 defect, 3 smells, 4 nitpicks

## Acceptance challenged

- **Claim.** After pass 2 every window setter respects the last choice, and tile mode never stays blank.
- **Combination not in the controls.** A region window still waiting on its extent when a preset, drag or Reset lands, and a series-window request that fails.
- **Result.** The first breaks the claim. The second leaves tiles blank, below.

## Pattern

This is the third pass running to find the same class of defect in a new path: a late window reply overwriting a newer choice. Pass 1 found it in the pump, pass 2 in Reset, drag and link, and this pass in the region tool. Each repair was real and controlled. The root cause is that the viewer has several window setters and no single owner of the latest choice. A single window-change entry point is filed as a follow-up rather than refactored inside close-out.

## Defects

**D1** `examples/folder-viewer/src/page.html` applyRegion. It awaits the library and the region extent before calling chooseWindow, and its `still()` ignores `windowChoice`, so a preset, drag or Reset made while the extent is pending is overwritten when the region commits.

## Smells

**S1** `page.html` ensureSeriesWindow swallows a rejected fetch without storing an entry or a refusal, so the series window stays wanted and pumpMain's guarded tile paint never runs. Tile mode stays blank after a failed series-window request.

**S2** `tests/browser/window.mjs` the three window controls still settle on fixed waits, and the Reset control's `done` matches any `/preview` reply rather than the held one. A catch-all `/preview` route left in the out-of-order control is shadowed and dead.

**S3** Pass 1 N7 and pass 2 N3 and N4 are not filed or dispositioned, and CURRENT_SPRINT's sentence saying no follow-up lives only in a review file is therefore false.

## Nitpicks

N1 the dead catch-all route, carried with S2. N2 the JPEG XL refusal test's name says colour space where the rule is the colour encoding. N3 tile mode can paint twice in one pump iteration, harmless. N4 auto.mjs's moved JSDoc leaves `observeSeries` without one.

## Verified clean

The pass 2 counter bumps in drag commit, resetView and copyWindow are present and ordered before their commits. The JPEG XL decoder-path refusal case fails with the rule's call removed. Follow-ups F-274 to F-279 agree across BACKLOG, allocation, SPRINT_PLAN and the More roadmap.

## Evidence

The sprint run on tree 3285ba2f was stopped and recorded as aborted, because this pass required remediation and the remediation edited the tree it read. A new sprint profile and comparison run on the remediated tree is required before pass 4.
