# F-266 prepared worker handoff

**Branch**: `work/f-266-glm`
**Base**: `1d627bd93f0c0e3b44231be9a0f3e76607a8eeb9`
**Head**: `ace04b78c277ff4ac604c86d732a8e21352fda0a`
**Files touched**: .claude/plans/F-266-design.md, .claude/reviews/F-266-amendment-pass-1.md, .claude/reviews/F-266-amendment-pass-2.md, .claude/reviews/F-266-amendment-pass-3.md, .claude/reviews/F-266-design-pass-1.md, .claude/reviews/F-266-design-pass-2.md, .claude/reviews/F-266-implementation-pass-1.md, .claude/reviews/F-266-implementation-pass-2.md, .claude/reviews/F-266-implementation-pass-3.md, .claude/reviews/F-266-implementation-pass-4.md, CHANGELOG.md, README.md, docs/lld/README.md, docs/lld/build-targets.md, docs/lld/typescript-packaging.md, docs/runbooks/guard-verification.md, docs/sprints/BACKLOG.md, examples/viewer-react/csp-probe.mjs, examples/viewer-react/src/core-worker.ts, examples/viewer-react/vite.config.ts, scripts/guards/catalogue.py, scripts/tests/test_viewer_build_check.py, scripts/viewer_build_check.py
**Review**: `.claude/reviews/F-266-implementation-pass-4.md`
**Verify tree**: `cdbfaa2424389faceaf35c08fb7da6ddd77c6957`

## Verified source

The source commit above passed the actual feature floor, all 29 gates, and
full corpus verification over this exact staged tree. The corpus has 133
verified rows, with none missing or mismatched. The nine corpus integration
tests passed. The checked generator passed separately. The hook supplied the
feature-profile provenance trailers from the exact-tree receipt.

The real branch-built glue and WASM match the preserved runtime-tested input
bytes exactly. The WASM SHA256 is
`e9f275dc62700c3b325acbef2283d72c180535c7eec37af59b5936ac316327f2`.
The glue SHA256 is
`be1870549103534b68fdfc2a4ad183994b7a99630536059c68b5ee2a5b871002`.
The build-source, runtime, received-policy, body-digest and mutation evidence
is retained under `/private/tmp/ocelli-s46-codex/f266-final-feature-profile/`
and `/private/tmp/ocelli-s46-codex/f266-final-readiness/`.

The independent review closed every implementation defect and smell, with
one optional line-wrapping nit. The HTTP matrix distinguishes six real-pkg
PRESENT 200 observations, five ABSENT HTML 200 fallbacks and one literal-
percent ABSENT HTTP 500 no-package observation. The 500 is not healthy HTTP
or browser readiness evidence. Source-bound preflight-fetch, actual CSP
header-removal, outside-worker, plus-path and non-UTF-8 sensitivity controls
remain separately attributed.

## Transport and integration boundary

This tracked handoff is a local transport addition after the verified source
commit. It is not an implementation change, has no verification receipt and
carries no verification trailer. Its transport tree must never be presented
as the verified source tree above. It is not push-ready. No CI, push,
canonical integration, shared Done status or S46 closure is claimed.

Root integrates the complete branch by three-way merge from the base above,
consumes this handoff and performs the normal final canonical checks and
completion records. The current worker leaves shared delivery completion to
that integrator. CURRENT_SPRINT remains S46 under the explicit isolated S47
work-order exception.
