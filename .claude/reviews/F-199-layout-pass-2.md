# F-199 layout, pass 2

**Result**: 0 defects, 0 smells, 0 nitpicks

The page calls `overlayGrid`, `stackDrag`, `referenceLines` and `referenceCursor`. The drag label is Stack drag. The story title says stack drag in the plan, the backlog, the sprint plan and the allocation. Horizontal and vertical reference lines are labelled separately. Full screen turns on only when the browser flag is true and the section is the active element. Prior studies, linked scroll, synced window, MPR, slab, volume rendering and fusion read unavailable. Those controls are F-203.

Pass 1 was 0 defects, 1 smell and 3 nitpicks. The smell was a Series drag label on a stack step. A constant grid count of 2 failed the one-cell test and was restored. A full screen handler that returned immediately failed the click test and was restored.

Reviewer Claude Code, read only, with no shell. This pass did not run the tests.
