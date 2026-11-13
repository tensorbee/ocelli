# F-204 review, implementation pass 3

**Reviewed**: work/f-204-glm at 24245f0a.
**Author / reviewer**: GLM through opencode. Review by Claude, a read-only subagent, composed by the Claude coordinator for S47.
**Result**: 3 defects, 2 smells, 2 nitpicks

The sized-up box reaches both cell edges at 1.25 and 1.57, paint, readout, magnifier and ink agree at every probed point, the flag-guarded restore and the reason rule hold, and the floor-box, restore, device offset, CSS transform and CSS-size backing mutants are red. The box floors a fractional cell side before taking the step multiple, so the extra step vanishes and a 3x3 middle cell misses its edge by up to 0.69 device px. The README and More still say the reason clears when the ratio changes, the gate comment names two refusal reasons, entering the mode does not refuse when no step exists, and the reach rows pin only the horizontal axis.
