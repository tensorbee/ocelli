# Parallel handoffs

`F-XXX-ready.md`, written by `/complete-feature --prepare` on a worker branch
and consumed by `/integrate-feature` in the canonical worktree.

**A handoff left behind is not a leftover, it is evidence the integration step
did not run.** `/sync-status` and `close-preflight` both report one as a
failure rather than tidying it away.

This directory is normally empty. Only `README.md` is permanent.
