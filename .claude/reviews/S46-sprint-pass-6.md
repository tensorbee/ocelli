# S46 review, pass 6

**Reviewed**: the staged pass 5 nitpick round on sprint/s46 at the committed remediation, tree 185def6e at HEAD, staged tree 81300faa.
**Author / reviewer**: nitpick edits by GLM with a coordinator rewrap, dispositions by the Claude coordinator. Review by Claude, a read-only subagent that ran nothing, composed by the Claude coordinator.
**Result**: 0 defects, 0 smells, 3 nitpicks

## Acceptance challenged

- **Claim.** The pass 5 nitpick round changes only what it names and reopens nothing passes 1 to 5 closed.
- **Checked.** The new positive `manual` check against how the page assembles the corner line, the switch control without the `s` press, the F-280 notes and CURRENT_SPRINT dispositions against the tree, and the round 4 guards at HEAD.
- **Result.** No defect and no smell.

## Nitpicks

N1 `.claude/reviews/S46-sprint-pass-5.md` N6 said the unguarded tile paint in refreshFrameWindows predates S46. It came from F-213's review pass 2, b8ec548f, in this sprint. Corrected in that file.

N2 `docs/sprints/CURRENT_SPRINT.md` the dispositions bullet did not name pass 4's nitpicks. Pass 4 N1 to N4 are corrected in the tree, and N5 is now dispositioned as kept, matching the existing region check that reports a missing library build by name.

N3 `examples/folder-viewer/tests/browser/auto.mjs` the tile reader still recorded a width nothing reads. Removed.

## Verified clean

The corner's first line ends `  manual` after a committed W/L drag and would not on either failure, the series list is visible and clickable without the `s` press, the removed width clause could never fail, the jpegxl change is a comment rewrap, the F-280 notes agree everywhere, the prose rules hold, and the round 4 guards hold at HEAD.

## Evidence

The sprint run on tree 81300faa was stopped before a verdict, because these corrections change the tree. A fresh sprint profile on the corrected tree is required before close.
