# F-202 design review, pass 3

**Reviewed**: design round 3 of .claude/plans/F-202-design.md.
**Author / reviewer**: GLM through opencode. Review by Claude, a read-only subagent, composed by the Claude coordinator for S47.
**Result**: 10 defects, 9 smells, 8 nitpicks

AC16 missed that stored zero is left out of the automatic window, the resolver extraction left second copies, a resident hit had no facts record, the overlay gate was not real and the remote canvas sat inside an element the local layout hides.
