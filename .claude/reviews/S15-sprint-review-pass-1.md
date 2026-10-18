# S15 review, pass 1

**Reviewed**: 8f890663be91c7e333bd710205da41157a257250..a1821bf62466cb74c8090e6a320022cc8aaa58ad
**Result**: 0 defects, 0 smells, 0 nitpicks

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

HEAD is `a1821bf62466cb74c8090e6a320022cc8aaa58ad`. The merge-base with main is `8f890663be91c7e333bd710205da41157a257250`. The range is the sprint open, the design approval, and F-100, F-045, F-046, F-047, F-049. `bin/ocelli.sh gate --sprint` was not run.

**Ledgers.** `docs/sprints/BACKLOG.md` marks F-045, F-046, F-047, F-049 and F-100 `done`. `docs/sprints/CURRENT_SPRINT.md` has the same five status cells as `done`. `docs/sprints/SPRINT_TRACKER.md` has one row for each. `docs/sprints/AS_BUILT.md` has one entry for each. `CHANGELOG.md` under `## Unreleased` has five bullets, one for each story. Dependencies F-039, F-041, F-032 and F-020 are `done`. No story in the sprint depends on another story in the sprint.

Review counts match the files. F-100 is 2 defects and 3 smells, then 0 and 0, then 0 and 0. F-045 is 0 defects, 0 smells and 1 nitpick. F-046 is 1 defect then 0, with 0 smells and 1 nitpick on each pass. F-047 is 0, 0 and 0. F-049 is 0 and 0, then 0 and 0, with 1 nitpick on the first pass. The accepted F-045 and F-046 nitpicks were not reopened. The recorded plan notes on glam, `Modality` and `usize::from(u32)` were not treated as missing HLD deviations.

**Oracle trailers.** Each feature trailer lists `profile=feature` and does not name `oracle`. F-046 and F-047 say oracle was not run. F-045 does not say the gate was run and does not say it was skipped. That is not a trailer claiming the opposite. F-049 says `gate oracle` was green over 102 views, with 29 mutations and 0 not detected, and that the feature trailer does not list the gate. The trailer at `a1821bf` does not list it. `tools/oracle/compare-out/compare.json` is `operation` identity, `views` 102, `green` true, 71 pass, 31 unmeasured and 0 fail. `./target/release/ocelli-compare mutations` against `tools/oracle/out` printed `compare: 29 mutations, 0 not detected` and exited 0. The full `gate oracle` command was not run again.

**Ownership.** `git log --name-only` shows `view.rs` and `graph.rs` only in `1dfaa71`. F-047 and F-049 do not edit them. `apply_placement` calls `set_spacing` and `set_index_shift`.

**Spacing.** `PixelSpacing::new` takes row millimetres then column millimetres. `View::set_spacing` takes column millimetres then row millimetres. `apply_placement` passes `column_mm()` then `row_mm()`. Swapping that call to row then column made `apply_placement_stores_spacing_and_the_measured_shift` exit 101 at `tests/placement.rs:123`. Left bits `4613937818241073152` are `3.0`. Right bits `4609434218613702656` are `1.5`. The call was restored. The same test file then exited 0. All 8 tests passed.

**One formula, one cache.** `frame_shift` reads `ImagePlane::index_to_world` and its inverse. `Presentation` rebuilds through `VoiTransform::new` and `LutChain::new`. `colour_lut_for` maps `Colormap` onto `PresetId` and calls `ColourLut::preset`. `StackSequence` holds keys and reads `ImageCache`. `samples_for_upload` calls `get` only for the selected key. A miss is `NotResident`. `scroll.rs` does not set `Dirty.slice`. `set_view` sets `dirty.camera` and leaves `slice` false. The ignored GPU test asserts that pair.

**Tiers.** `docs/lld/viewport.md` answers A, B and C for the camera, the chain, scroll and the shift. Tier C drawing stays unavailable. `RenderGraph::new` still returns `TierCUnavailable`. The shift and the chain have no tier branch.

**Boundary.** `cargo tree -p ocelli-viewport -e normal` lists glam, `ocelli-cache`, `ocelli-pixel` and `ocelli-core`. It does not list wgpu or `ocelli-render`. `ocelli-render` depends on `ocelli-viewport`. The viewport product diff adds no `unsafe` and no `wasm-bindgen`. The new test asserts that `viewport.ts` does not contain `wasm-bindgen`.
