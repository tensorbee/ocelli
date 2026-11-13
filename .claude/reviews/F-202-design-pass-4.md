# F-202 design review, pass 4

**Reviewed**: design round 4 of .claude/plans/F-202-design.md.
**Author / reviewer**: GLM through opencode. Review by Claude, a read-only subagent, composed by the Claude coordinator for S47.
**Result**: 10 defects, 6 smells, 7 nitpicks

The lead's round 3 AC16 ruling was itself wrong, because the margin moves by f32 steps. AC24 and AC12 could not fail or failed a correct build, the remote field was cleared by ordinary local paints, input routing was incomplete and the decode size check would turn every later change red.
