# S14 review, pass 2

**Reviewed**: staged remediation on sprint/s14 against 30f7e16
**Result**: 0 defects, 0 smells, 0 nitpicks

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

`git rev-parse HEAD` is `30f7e1654f9c88445bb9ffac3dd28a91849a8afc`. `git status --short`, `git diff --cached`, and `git diff HEAD` are the same three paths. `.claude/reviews/S14-sprint-review-pass-1.md` is added. `docs/lld/volumes.md` and `docs/sprints/SPRINT_TRACKER.md` are modified. No product file is in either diff.

D1. The replacement paragraph in `docs/lld/volumes.md` says the brick upload in `ocelli-render` refuses an axis longer than the device's reported 3D limit, that the pinned tier B floor is 256, that a brick edge of 128 is guaranteed to fit that floor, and that this crate does not clamp the reported limit and does not apply the check. `brick_plan` in `crates/ocelli-render/src/brick_upload.rs` returns `ExtentUnavailable` when an axis is greater than the `max_texture_dimension_3d` it was given. `BrickResidency::upload` passes `ctx.caps().max_tex_3d` into that argument. `RenderGraph::upload_brick` in `crates/ocelli-render/src/graph.rs` delegates to that upload and does not pass 256. `classify` in `crates/ocelli-render/src/caps.rs` copies `facts.max_tex_3d` for every tier other than CPU. `docs/lld/render-graph.md` calls 256 the pinned tier B floor and says a brick edge of 128 fits. `a_volume_that_fits_still_plans_brick_extents` in `crates/ocelli-render/tests/brick_upload.rs` asserts the downlevel 3D limit is 256, accepts extent `[128, 1, 1]` from `plan_of(Tier::B, 256, ...)`, and refuses axis 257.

D2. The F-X002 row in `docs/sprints/SPRINT_TRACKER.md` says three independent review passes converging 1/0, 0/0, 0/0. `.claude/reviews/F-X002-layers-pass-1.md` records 1 defect and 0 smells. `.claude/reviews/F-X002-layers-pass-2.md` records 0 defects and 0 smells. `.claude/reviews/F-X002-layers-pass-3.md` records 0 defects and 0 smells. The F-X002 notes in `docs/sprints/AS_BUILT.md` say three review passes converged 1/0, 0/0, 0/0.

The F-043 tier sentence in `docs/sprints/AS_BUILT.md` has the caller-passes-256 reading. It says the pure plan accepts a brick inside the pinned downlevel 3D limit of 256 and refuses an axis of 257. That is `brick_plan` when the caller passes 256, which is what `plan_of(Tier::B, 256, ...)` does. The same paragraph says this machine's device test resolved tier A, so tier B was not a device upload. The sentence does not say the device limit is clamped to 256.

The remediation adds no new false sentence. `bin/ocelli.sh gate` was not run here. The task recorded the sprint gate on this staged tree as exit 0, 31 gates, all green. The same two oracle rows still print FAIL and remain the unsupported count of 2.

Commands: `git rev-parse HEAD`, `git status --short`, `git diff --cached`, `git diff HEAD`.
