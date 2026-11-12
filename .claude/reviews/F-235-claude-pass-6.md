# F-235 review, pass 6

**Reviewed**: work/f-235-glm at b6392342.
**Author / reviewer**: GLM through opencode. Review by Claude, a read-only subagent, composed by the Claude coordinator for S47.
**Result**: 2 defects, 1 smell, 0 nitpicks

All 32 unbound load-op rows matched the old choice. The acquired-surface clear was unpinned at its call site, two staleness sentences were false and the debug assert checked one direction.
