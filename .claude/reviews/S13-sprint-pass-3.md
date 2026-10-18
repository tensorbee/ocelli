# S13 whole-sprint review, pass 3

**Reviewed**: working tree against `439c44287cfe13b2a0d67acb69aa159afbe48cc1`, HEAD `6e9260808803f948f2c64232ccaa0bf9f8fedb56`. Staged remediation is in `crates/ocelli-wasm/src/lib.rs`, `docs/hld/DEVIATIONS.md`, `docs/lld/README.md`, `docs/lld/build-targets.md`, `docs/sprints/AS_BUILT.md` and `docs/sprints/CURRENT_SPRINT.md`. `docs/sprints/AS_BUILT.md` also has an unstaged edit on the F-039 deviations line. Untracked `.claude/reviews/S13-sprint-pass-1.md` and `.claude/reviews/S13-sprint-pass-2.md` were read and not edited.
**Result**: 0 defects, 0 smells, 0 nitpicks

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

`python3 scripts/prose_check.py` exited 0 over 382 tracked files. This review was not in that set. The unstaged deviations sentence has no em-dash and no prose semicolon. `docs/hld/` is exempt and the D-14 cell was read anyway.

The unstaged F-039 deviations line at `docs/sprints/AS_BUILT.md:3397` is the pass 2 fix. At commit `b93eef0` the D-14 Why cell says the measured cost today is 1,999,061 bytes, because F-039 made `ocelli-wasm` depend on `ocelli-render` and on `wgpu`. HEAD still has that sentence. The working-tree cell at `docs/hld/DEVIATIONS.md:31` calls 1,999,061 the recorded budget, records the F-044 verification rebuild of the product module at 2,010,440 bytes, says that size is inside the 5 percent ceiling, and says the budget file was not moved. `ci/wasm-size-budget.json` still has `"bytes": 1999061` and `"tolerance": 0.05`. `scripts/pin_and_size_check.py` sets the ceiling with `int(baseline * (1 + TOLERANCE))`. `int(1999061 * 1.05)` is 2099014. 2010440 is inside that ceiling. The fraction over the baseline is about 0.57 percent. `git log 439c44287cfe13b2a0d67acb69aa159afbe48cc1..HEAD -- ci/wasm-size-budget.json` names only `b93eef0`. The working tree does not modify that file. `.claude/scratch/F-044-progress.md` records `bin/ocelli.sh gate wasm` exiting 0, product module 2,010,440, baseline 1,999,061, ceiling 2,099,014, and says the budget file was not edited. `crates/ocelli-wasm/pkg/ocelli_wasm_bg.wasm` is 2,010,440 bytes and contains the bytes `naga-30.0.1`. This pass did not rebuild wasm.

The same F-039 entry still says feature verification rebuilt the module at 1,999,100 bytes and did not move the budget. A prior `gate wasm` log printed `wasm size 1,999,100 bytes, baseline 1,999,061, ceiling 2,099,014`. That sentence was not charged. The F-042 entry says a product-module rebuild stayed inside the 5 percent tolerance and does not name a byte count. `.claude/scratch/F-042-progress.md` records a gate measurement of 1,999,272 bytes against the same baseline and ceiling. 1999272 is inside 2099014. That entry was not charged.

The nine LLD files this sprint edited still have `F-IDs that contributed` headers that match their `docs/lld/README.md` rows. A header-to-index comparison of every file under `docs/lld/` found one mismatch, `oracle.md`, and that file is not in the sprint diff. Cache is F-031 through F-036 with F-035 included. GPU ownership includes F-042. Tier resolution is F-004, F-037, F-039, F-042, F-X001, F-X016. TypeScript packaging ends in F-035. Pixel pipeline, render graph, build targets, benchmarks and errors match as well.

`docs/lld/build-targets.md` lines 88 to 92 say six functions. The table at lines 297 to 304 lists `ocelli_version`, `install_panic_hook`, `panic_record_ptr`, `panic_record_len`, `attachOffscreenCanvas` and `resizeOffscreenCanvas`. The `ocelli_version` comment in `crates/ocelli-wasm/src/lib.rs` says the product module also exports the panic record and the two canvas functions. It does not say that function is the entire surface. `attachOffscreenCanvas` is at `crates/ocelli-wasm/src/canvas.rs:66` and `resizeOffscreenCanvas` at line 89. `tierBSection183` at line 371 is behind `feature = "tier-b-probe"`. `panic_probe_trigger` stays behind `panic-probe`. `packages/core/src/index.ts` exports `exportDecodedFrame`, `acceptTransferredFrame` and `transferredFrameMessage`. `coreAvailable` still returns false.

`docs/sprints/AS_BUILT.md` around line 3467 keeps the F-040 wider-format rule in the past and says the code today follows the F-042 entry. Tier B may use `Rg32Float` or `Rgba32Float` when `R32Float` cannot be sampled. Tier A still returns only `R32Float`. `docs/sprints/CURRENT_SPRINT.md` says the graph acquires an OffscreenCanvas when one is bound, and that a viewport with no surface still draws into a caller-supplied texture view. It says the handoff copies into a fresh ArrayBuffer. It does not say the graph acquires no surface, and it does not say the handoff is a pointer and a length. The status column still says pending. The file says that column is stale and that the backlog is the authority. The column was not charged.

`docs/sprints/AS_BUILT.md` around line 672, the F-004 entry, still says the measured cost today is zero bytes because `ocelli-wasm` did not depend on `ocelli-render`. That sentence predates S13. It was not charged. `ci/bench-baseline.json` was not charged. No tier B colour LUT draw was required or run.

`docs/lld/gpu-ownership.md` lines 413 to 414 say `ocelli-wasm` names `OffscreenCanvas` and does not contain that substring. The crate source contains `OffscreenCanvas` and does not contain `create_surface`. The ownership table's forbidden name is `create_surface`. `ocelli-render` source does not contain `web_sys`. The sentence was read as that source claim and was not charged.

The sprint diff adds no `unsafe` and no numeric `as` cast. `wasm_bindgen` added by the diff is only in `crates/ocelli-wasm/src/canvas.rs`. `request_device` remains two calls, both in `crates/ocelli-render/src/probe.rs`. `tick` submits once, at `graph.rs:1780`. `shaders/voi.wgsl` is not in the sprint diff.

The stops mutation was applied and then restored. In `preset_bytes`, index 128 was changed from `[255, 0, 0]` to `[0, 255, 0]`. `bin/ocelli.sh test ocelli-render --test colour -- --test-threads=1 grey_at_display_40_is_rgb_40_and_stops_are_the_three_texels` exited 101. The assertion at `crates/ocelli-render/tests/colour.rs:179` showed left `[0, 255, 0, 255]` and right `[255, 0, 0, 255]`. `git checkout -- crates/ocelli-render/src/colour.rs` restored the file. `git diff -- crates/ocelli-render/src/colour.rs` was empty. The same test then exited 0. The constant at `colour.rs:232` is `[255, 0, 0]` again.
