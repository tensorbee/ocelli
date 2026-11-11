# F-272 review, pass 6

**Reviewed**: work/f-272-glm at 8e4157fa.
**Author / reviewer**: GLM through opencode, with the coordinator's lead amendment a8dc0bbe and documentation commit 9c1b5346. Review by Claude, a read-only subagent, composed by the Claude coordinator for S47.
**Result**: 2 defects, 2 smells, 4 nitpicks

D1 the mirror copied all of corpus/data into the temporary directory on every run, unchecked and surviving Ctrl-C, where a symlink gives the same guarantee. D2 the plan was not amended for the mirror. S1 the child inherited the real working directory. S2 the one-helper rule was prose only.
