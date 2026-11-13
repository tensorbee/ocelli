# F-204 design review, pass 2

**Reviewed**: design round 2 of .claude/plans/F-204-design.md.
**Author / reviewer**: GLM through opencode. Review by Claude, a read-only subagent, composed by the Claude coordinator for S47.
**Result**: 5 defects, 8 smells, 5 nitpicks

Origin snapping was designed and its arithmetic held. AC6 contradicted the clamp floor, the 1.44 ratio probe could not fail after f32 storage, no pixel readback ran at 1.25 or 1.5, the F-202 seam was stale and open question 4 rested on a false cornerstone premise.
