# F-204 review, implementation pass 2

**Reviewed**: work/f-204-glm at d079790f.
**Author / reviewer**: a Claude implementer agent while the GLM server was down. Review by Claude, a read-only subagent, composed by the Claude coordinator for S47.
**Result**: 2 defects, 1 smell, 3 nitpicks

Sixteen mutants were run, fifteen red and one equivalent. Every byte row passes with crop offsets from observation, and the outside-the-mode magnifier regression is pinned. The mode's box is rounded down and shifted, so a 2048-wide frame at ratio 1.57 leaves a 36 CSS px black band inside the cell under Zoom 1:1, with ink and readout over it, below the cornerstone floor. Leaving the mode by a zoom tick can keep the mode's panned buffer in a stored-size backing when the box times the ratio equals the stored size. The corner can show a stale refusal reason, and the INVERSE citation should be C.11.6.1.2.
