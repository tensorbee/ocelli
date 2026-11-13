# F-204 design review, pass 1

**Reviewed**: the GLM draft of .claude/plans/F-204-design.md on work/f-204-glm at ac962c64.
**Author / reviewer**: GLM through opencode. Review by Claude, a read-only subagent, composed by the Claude coordinator for S47.
**Result**: 6 defects, 6 smells, 6 nitpicks

The draft scaled the display size but never put the image origin on a device pixel, had no origin-sensitive or fractional-ratio test, claimed cornerstone3D was unreadable, missed fitView from the fitMode census and called anisotropic nearest sampling a calibrated stretch.
