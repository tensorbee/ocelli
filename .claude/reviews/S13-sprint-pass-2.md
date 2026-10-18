# S13 whole-sprint review, pass 2

**Reviewed**: working tree against `439c44287cfe13b2a0d67acb69aa159afbe48cc1`, HEAD `6e9260808803f948f2c64232ccaa0bf9f8fedb56`, including the uncommitted remediation. `git status --short` shows the six dirty files and untracked `.claude/reviews/S13-sprint-pass-1.md`.
**Result**: 1 defect, 0 smells, 0 nitpicks

## Defects

### D1, F-039 still says the Why cell records 1,999,061 as the measured size

**Where**: `docs/sprints/AS_BUILT.md:3397`
**What**: The deviations line says D-14's Why cell now records the measured size of 1,999,061 bytes.
**Why it is wrong**: The sentence is present tense about the cell as it stands. The remediation changed that cell. It now calls 1,999,061 the recorded budget and records a later measurement of 2,010,440. Calling the budget figure the measured size is the claim pass 1 refused, and this line still makes it. The F-039 entry is S13. It is not a prior-sprint row. HLD section 15.2's recorded budget is the file, and the cell now points at that file rather than at today's module.
**Evidence**: `docs/hld/DEVIATIONS.md:31` says the recorded budget is 1,999,061 bytes, from the edge that made `ocelli-wasm` depend on `ocelli-render` and on `wgpu`, and that the F-044 verification rebuild measured the product module at 2,010,440 bytes, inside the 5 percent ceiling, and that the budget file was not moved. `ci/wasm-size-budget.json` still has `"bytes": 1999061` and `"tolerance": 0.05`. `int(1999061 * 1.05)` is 2099014, and 2010440 is inside that ceiling. `.claude/scratch/F-044-progress.md` records `bin/ocelli.sh gate wasm` exiting 0, product module 2,010,440, baseline 1,999,061, ceiling 2,099,014, and says the budget file was not edited. This pass did not rebuild wasm. `git log 439c44287cfe13b2a0d67acb69aa159afbe48cc1..HEAD -- ci/wasm-size-budget.json` names only `b93eef0`. Commit `979d14e` does not touch that file. `crates/ocelli-wasm/Cargo.toml` depends on both `ocelli-render` and `wgpu`. `CHANGELOG.md` still records 1,999,061 as the budget figure for the render edge. That changelog sentence is not contradicted.

## Smells

None.

## Nitpicks

None.

## Verified clean

`python3 scripts/prose_check.py` exited 0 over 382 tracked files. This review was not in that set. The remediation sentences have no em-dash and no prose semicolon. `docs/hld/` is exempt and was read anyway.

The nine LLD files this sprint edited have `F-IDs that contributed` headers that match their `docs/lld/README.md` rows, in order. Cache is F-031 through F-036 with F-035 included. GPU ownership includes F-042. Tier resolution is F-004, F-037, F-039, F-042, F-X001, F-X016. TypeScript packaging ends in F-035. Pixel pipeline, render graph, build targets, benchmarks and errors match as well. Commit `3f035c3` edited cache and TypeScript packaging. `5e2b916` edited GPU ownership, render graph and tier resolution. `b93eef0` edited the F-039 files whose headers include F-039. `9426845` and `979d14e` edited render graph, and F-044 also edited pixel pipeline. Those headers include F-040 and F-044.

`docs/lld/build-targets.md` lines 88 to 92 say six functions. The table at lines 297 to 304 lists `ocelli_version`, `install_panic_hook`, `panic_record_ptr`, `panic_record_len`, `attachOffscreenCanvas` and `resizeOffscreenCanvas`. The `ocelli_version` comment in `crates/ocelli-wasm/src/lib.rs` no longer says that function is the entire exported surface. `attachOffscreenCanvas` is at `crates/ocelli-wasm/src/canvas.rs:66` and `resizeOffscreenCanvas` at line 89. `tierBSection183` at line 371 is behind `feature = "tier-b-probe"`. `panic_probe_trigger` stays behind `panic-probe`. `packages/core/src/index.ts` `coreAvailable` still returns false.

`Upload::preserving_format` returns `Rg32Float` or `Rgba32Float` on tier B only after `R32Float` lacks `TEXTURE_BINDING` or `COPY_DST`, and only when the chosen format has both. Tier A returns `R32Float` or `FormatUnavailable`. `STORED_FORMAT` remains `R32Float`. The F-040 tier row states the wider formats in the present tense and keeps "no wider format" in the past, for the story as it landed. `docs/lld/render-graph.md` says a format is usable when it has both usages. The F-040 row's "cannot be sampled" is the code's `sampleable` helper, which sets both flags. It does not say the substitute is absent today.

`docs/sprints/CURRENT_SPRINT.md` no longer says the graph acquires no surface, and no longer says the handoff is a pointer and a length. `bind_surface` stores the surface. `acquire_dirty_surfaces` calls `get_current_texture`. A viewport with no surface uses the caller-supplied view. `exportDecodedFrame` allocates a fresh `ArrayBuffer`, copies into it, and returns that buffer. The status column still says pending. The file says that column is stale and that the backlog is the authority. The backlog status for F-035, F-039, F-040, F-042 and F-044 is `done`. The column was not charged.

The sprint diff adds no `unsafe` and no numeric `as` cast. New conversions go through `try_from` or the bit walk in `saturating_u32`. `wasm_bindgen` added by the diff is only in `crates/ocelli-wasm/src/canvas.rs`. `request_device` remains two calls, both in `crates/ocelli-render/src/probe.rs`. `tick` submits once, at `graph.rs:1780`. `present` is after that submit and is not a second submit. `RenderGraph::new` still returns `TierCUnavailable` on tier C. `wgpu` in the workspace manifest is `=30.0.1`, and `ocelli-render` enables `webgl`. No new trait, generic parameter or `Box<dyn>` was added. `shaders/voi.wgsl` is not in the sprint diff. `voi_linear` still uses `c - 0.5` and `w - 1.0`, with `<=` on the low side and `>` on the high side. `voi_linear_exact` uses neither adjustment. Colour mode 2 returns before `lut_chain`. Mode 1 indexes the display value and does not also normalise. ICC, a palette on monochrome, and a preset on palette colour return `Unavailable`.

The stops mutation was applied and then restored. In `preset_bytes`, index 128 was changed from `[255, 0, 0]` to `[0, 255, 0]`. `bin/ocelli.sh test ocelli-render --test colour -- --test-threads=1 grey_at_display_40_is_rgb_40_and_stops_are_the_three_texels` exited 101. The assertion at `crates/ocelli-render/tests/colour.rs:179` showed left `[0, 255, 0, 255]` and right `[255, 0, 0, 255]`. `git checkout -- crates/ocelli-render/src/colour.rs` restored the file. `git diff -- crates/ocelli-render/src/colour.rs` was empty. The same test then exited 0.

The carried story notes were re-read and not raised. No tier B colour draw was required or run. `docs/lld/render-graph.md` and the F-044 as-built entry both say that readback was not run. `ci/bench-baseline.json` was not charged. The F-004 as-built sentence that the measured cost today is zero bytes predates S13 and was not charged.
