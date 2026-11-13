# F-204 review, implementation pass 1

**Reviewed**: work/f-204-glm at d9aa76ea.
**Author / reviewer**: GLM through opencode. Review by Claude, a read-only subagent, composed by the Claude coordinator for S47.
**Result**: 5 defects, 5 smells, 2 nitpicks

Eleven mutants were killed. The AC3 and AC19 byte proofs were cut without authority, and a probe showed 1:1 is byte-exact at 1.25 and 1.5 with a whole-CSS-px static position or a device-sized backing. An odd stored extent at ratio 2 is resampled under a Zoom 1:1 label, below the cornerstone floor. The prose overclaims. AC5 and AC7a did not reject their named wrong implementations, the snap and congruence machinery had no observable effect, the crop searched a window, centre-keeping and Fit leaving the mode were untested and one width assertion was dead. The dicomweb gate failure was not reproduced and is not F-204's.
