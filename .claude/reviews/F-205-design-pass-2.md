# F-205 design review, pass 2

**Reviewed**: design round 2.
**Author / reviewer**: GLM through opencode. Review by Claude, a read-only subagent, composed by the Claude coordinator for S47.
**Result**: 9 defects, 10 smells, 4 nitpicks

The default preset returned after Reset, corrupt documents had contradictory rules, two tabs lost updates, sentences could sit in a hidden column, the no-patient-data test missed a stored folder name and the cornerstone3D persistence claim was false.
