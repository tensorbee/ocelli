# F-202 design review, pass 1

**Reviewed**: design round 1 of .claude/plans/F-202-design.md.
**Author / reviewer**: GLM through opencode. Review by Claude, a read-only subagent, composed by the Claude coordinator for S47.
**Result**: 13 defects, 13 smells, 6 nitpicks

The first draft left the decode dependency guard, the decode budget, the boundary record, colour and windows, the worker policy and trap recovery undesigned. The operator then decided the dependency allowlist, a separate lazy decode budget and staging once.
