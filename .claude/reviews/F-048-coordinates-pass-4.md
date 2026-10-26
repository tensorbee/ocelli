# F-048 review, pass 4

**Reviewed**: staged tree `084d4d78d255a226fbf677f1a75475151508f6dd`, branch `sprint/s16`, HEAD `22800f8`, diff `/tmp/review-F-048.diff`. Unstaged paths: none observed.
**Author / reviewer**: xAI initial implementation, Codex completion. Reviewer is an independent participant. The reviewer did not author F-048 or its repairs.
**Result**: 4 defects, 3 smells, 4 nitpicks.

## Acceptance challenged

AC1. The centre fixture compares `index_to_canvas(0, 0, 0)` with `image_to_canvas(0.5, 0.5)`. A mutation of `INDEX_TO_IMAGE_CENTRE` to 0.0 changes the y canvas coordinate by 1.0 CSS pixel on the axial plane, above the quarter-pixel tolerance, so the fixture rejects a corner index treated as the centre. The x difference is exactly 0.25 and passes, but the y assertion is discriminating.

AC2. The independent `patient` fixture derives `(11, 26, 30)` from PS3.3 with column spacing 0.5 and row spacing 2.0. A transposed plane gives `(14, 21.5, 30)`, which the fixture rejects. The view spacing order is not guarded, because every canvas expectation uses the same `View` spacing. S1 records that gap.

AC3. The oblique plane, pan, quarter turn, horizontal flip, zoom 2 and F-049 shift produce the independent world point `(10 + s, 20 + s, 36)`. The canvas point is the approved `image_to_canvas(2.5, 3.5)` composition. The world round trip returns to that canvas point.

AC4. The GPU consumer divides the device fragment by the ratio before the CSS multiply. The non-ignored shader test only checks substring order and passes on a double division. The fractional dpr 1.25 fixture is the only test that rejects a double division, and it is `#[ignore]`. D4 records that gap. The CPU path's `css_from_device` division is exercised by the tier C tests, not by this pass.

AC5. The `Pt<Canvas>`, `Pt<World>` and `Pt<Index>` markers and the `Transform<A, B>` pair make a cross-space `apply` a compile error. The existing trybuild fixtures cover the mechanism. The coordinate fixtures round-trip within `1e-6` and the quarter-pixel bound.

AC6. Zoom 0 and zero spacing on both axes return `CoordinateError::Singular`. The view is not written by the mapping call. The raster fallback claim is checked only at element 15, which is tautological. N3 records that gap.

AC7. The mapping calls live in `ocelli-viewport`, which does not depend on `ocelli-render`. `set_view` writes only the camera buffer and sets `Dirty.camera`. No texture or brick write and no upload counter increment appears on the path.

## Evidence

| Command or selected cases | Source and input identity | Executed now or prior receipt | Exit and observation |
|---|---|---|---|
| `bin/ocelli.sh test ocelli-viewport --test coordinates` | staged tree `084d4d78d255a226fbf677f1a75475151508f6dd`, six coordinate fixtures | executed now | exit 0, 6 passed, 0 failed, 0 ignored |
| Read `view.rs`, `space.rs`, `image_plane.rs`, `coordinates.rs` | staged tree, F-048 mapping and PS3.3 plane | executed now | composition, half-voxel and typed transform checked |
| Read `graph.rs`, `extent.rs`, `tests/view.rs` | staged tree, DeviceCamera and shader | executed now | 80-byte layout and shader division checked |
| Read `cpu.rs`, `commands.rs`, `presentation.rs`, `ring.rs` | staged tree, tier C and readback | executed now | CPU division and state offsets checked |
| GPU tests `device_fragments_are_divided_by_dpr_before_the_css_camera` and `fractional_dpr_selects_the_derived_texels` | `#[ignore]`, needs a real adapter | not executed | no adapter observation, no browser or WebGL2 observation |
| Compile-fail fixtures under `crates/ocelli-core/tests/ui` | staged tree | not executed | type-safety conclusion rests on signatures and existing fixtures |
| `gate --sprint` and oracle | prohibited by the assignment | not executed | not an acceptance claim |

## Defects

### D1, `css_from_device` inherits `device_extent` documentation
**Where**: `crates/ocelli-render/src/extent.rs:16-35`
**What**: The doc comment that describes `device_extent` is attached to `css_from_device`, because the new function was inserted between the doc block and `device_extent`. The first paragraph of `css_from_device`'s doc says "Device-pixel extent of a CSS box at `dpr`" and describes `round(css * dpr)`, which is false for the inverse function. `device_extent` now has no doc comment.
**Why it is wrong**: A doc comment is a factual claim about the item it documents. The public docs now describe the inverse function as the forward extent and leave the forward function undocumented. The microscope prose rule and HLD section 27.2's documentation expectations apply.
**Evidence**: Read of `extent.rs:1-75` after the staged diff. The block at lines 16-25 ends with `NonPositiveRatio`, lines 26-35 add the `css_from_device` text without a blank line, `pub fn css_from_device` is at line 36, and `pub fn device_extent` follows with no doc block.
**Scope and regression**: Every reader of the `ocelli-render` public API. The smallest regression is `#![deny(missing_docs)]` on the crate, which fails on the undocumented `device_extent`, or a doc test that asserts the `device_extent` item carries the "Device-pixel extent" summary.

### D2, The LLD and `View::camera_matrix` still say the shader multiplies `at.x` directly
**Where**: `docs/lld/viewport.md:50-60` (the added "Canvas, index and world" section), with the stale claim at `docs/lld/viewport.md:43-44` and `crates/ocelli-viewport/src/view.rs:241-243`
**What**: The added section says "The fragment shader divides a device-pixel position by the ratio before the CSS multiply." The earlier "The mapping" paragraph still says "The fragment shader multiplies `(at.x, at.y, 0, 1)` by it and truncates with `i32` toward zero." `View::camera_matrix`'s doc says "The fragment shader multiplies `(at.x, at.y, 0, 1)` by it." The shader computes `css_x = at.x / scale` and multiplies `(css_x, css_y, 0, 1)`.
**Why it is wrong**: The LLD and the public doc contradict the executed shader and each other. A consumer following the old sentence would omit the division, which AC4 names as a wrong implementation. The microscope prose rule and the design's AC4 apply.
**Evidence**: Read of `docs/lld/viewport.md:38-70`, `view.rs:236-250`, and `graph.rs:510-525`. The shader source contains the `select` call `let css_x = select(at.x, at.x / scale, scale > 0.0)` before `camera.css *`.
**Scope and regression**: Every reader of the viewport LLD and every implementer of the GPU camera consumer. The smallest regression is a docs assertion in the existing `the_camera_uniform_carries_dpr_and_the_shader_divides_first` test that the mapping paragraph and `camera_matrix` doc name the division before the multiply.

### D3, `set_view` validation order contradicts its doc
**Where**: `crates/ocelli-render/src/graph.rs:1507-1521` (doc) and `crates/ocelli-render/src/graph.rs:1531-1533` (code)
**What**: The doc says "Validation follows [`RenderGraph::set_voi`]: an unknown id, then a pass with no stack uniform, then a viewport whose GPU objects have not been recreated." The `# Errors` list puts `DevicePixelRatio` last. The code checks `!dpr.is_finite() || dpr <= 0.0` first and returns `GraphError::DevicePixelRatio` before the id lookup. With `ViewportId(99)` and `dpr = 0.0`, the actual error is `DevicePixelRatio`, not `UnknownViewport`.
**Why it is wrong**: The doc is a factual claim about the validation order. `set_voi`'s own doc says "the answer names the first thing that is wrong in the order a caller can act on it." A caller that relies on the documented order gets a different error.
**Evidence**: Read of `graph.rs:1440-1600`. `set_voi` checks `UnknownViewport`, then `PassHasNoUniform`, then `ViewportNotRebound`. `set_view` checks `DevicePixelRatio`, then `UnknownViewport`, then `PassHasNoUniform`, then `ViewportNotRebound`.
**Scope and regression**: Every `set_view` caller that passes an invalid dpr with an unknown or non-stack id. The smallest regression is an assertion in `set_view_dirties_only_the_camera_and_keeps_the_css_matrix` that `set_view(&context, ViewportId(99), camera, 0.0)` is `UnknownViewport`. That test is `#[ignore]`, so the check would not run in the CI floor, which is a separate evidence gap.

### D4, The non-ignored shader test accepts a double DPR division
**Where**: `crates/ocelli-render/tests/view.rs:411-437`
**What**: `the_camera_uniform_carries_dpr_and_the_shader_divides_first` finds `"at.x / scale"` and `"camera.css *"` and asserts the first occurs before the second. A shader that computes `at.x / scale / scale` still contains the first substring before the multiply, so the test passes. A shader that divides only `at.x` and leaves `at.y` undivided also passes.
**Why it is wrong**: AC4 requires rejecting both a missing division and a second division. The only test that rejects a second division is `fractional_dpr_selects_the_derived_texels`, which is `#[ignore]`. The ignored `device_fragments_are_divided_by_dpr_before_the_css_camera` also does not reject a second division at its dpr 2 case, because both device pixels then select texel 0. The CI floor therefore has no test that rejects a double division, and the non-ignored test is counted as coverage.
**Evidence**: Read of `tests/view.rs:411-437` and `tests/view.rs:437-530`. The fractional fixture expects source indices 0, 1, 2, 2, 3 at dpr 1.25. A double division gives 0, 0, 1, 2, 2. The non-ignored test does not evaluate either array.
**Scope and regression**: The GPU camera consumer and the AC4 evidence. The smallest regression is to assert the shader contains exactly one `"/ scale"` per axis, or to assert the exact `let css_x` and `let css_y` lines, so a second division is red without an adapter.

## Smells

### S1, The coordinate fixtures cannot catch a transposed `View::set_spacing`
**Where**: `crates/ocelli-viewport/tests/coordinates.rs:66-110`
**What**: `view_matching_plane` calls `set_spacing(0.5, 2.0)`. Every canvas assertion compares `index_to_canvas(index)` to `image_to_canvas(index + 0.5)`, and both calls use the same `View` spacing. The world assertion is independent of the view spacing because `canvas_to_world` inverts the same view before applying `ImagePlane::index_to_world`. A transposed `set_spacing(2.0, 0.5)` changes both sides of the canvas assertion identically and leaves the world point unchanged, so all six tests pass.
**Why it is wrong**: The test file's header names the two orders as a hazard. The plane's `PixelSpacing` order is guarded by the independent `patient` fixture. The view's order is not. A future caller that passes the DICOM row and column in the wrong order gets a wrong display and green tests.
**Evidence**: Read of `coordinates.rs:1-207` and the executed `bin/ocelli.sh test ocelli-viewport --test coordinates` run. The canvas expectations are `image_to_canvas(0.5, 0.5)`, `image_to_canvas(2.5, 3.5)` and `image_to_canvas(2.5, 3.5)`.
**Scope and regression**: Every consumer that sets the view spacing from DICOM `PixelSpacing`. The smallest regression is a hand-derived canvas point for index `(2, 3)` on the axial plane, for example `(3.5, 3.0)` for the default 9 by 8 grid, asserted independently of `image_to_canvas`.

### S2, A positive subnormal dpr narrows to a zero uniform scale and the shader silently skips the division
**Where**: `crates/ocelli-render/src/graph.rs:265-290` and `crates/ocelli-render/src/graph.rs:518-524`
**What**: `set_view` accepts any finite `dpr > 0.0`. `device_camera` narrows it with `ocelli_viewport::view::f32_from_f64`, whose documented behaviour maps a subnormal f64 below the f32 subnormal range to signed zero. The shader computes `select(at.x, at.x / scale, scale > 0.0)`, so a zero scale selects `at.x` unchanged. For `dpr = f64::MIN_POSITIVE / 2.0`, the uniform scale is `0.0` and the shader uses device coordinates, which is a dpr of 1.0. The CPU path's `css_from_device` divides by the f64 dpr and can return a huge CSS coordinate or refuse the quotient, so the two paths disagree.
**Why it is wrong**: The design says the uniform carries the same dpr as `device_extent` and the shader divides by it. The `select` fallback is unreachable for dpr 0 because `set_view` refuses it, but it is reachable for an accepted positive subnormal dpr. The CPU raster and the GPU graph then produce different mappings for the same input.
**Evidence**: Read of `graph.rs:262-290`, `graph.rs:510-525`, `extent.rs:36-50`, and `view.rs:598-660`. `f32_from_f64`'s doc says a subnormal f64 becomes a signed zero. `set_view` checks only `is_finite` and `> 0.0`.
**Scope and regression**: Any caller that passes a subnormal positive dpr. The smallest regression is `assert_eq!(device_camera_words(identity, f64::MIN_POSITIVE / 2.0)[16], 0.0)` paired with an assertion that `set_view` refuses that dpr, or a shader path that never selects the device coordinate when `scale == 0.0`.

### S3, The half-voxel constant is duplicated in `cpu.rs`
**Where**: `crates/ocelli-viewport/src/view.rs:452-460`
**What**: F-048 adds `INDEX_TO_IMAGE_CENTRE = 0.5` in `view.rs`. `crates/ocelli-render/src/cpu.rs:19-25` already has `IMAGE_FROM_INDEX = 0.5`. The CPU raster maps canvas to index with `View::canvas_to_index` and then adds its own `0.5` to recover the image coordinate for texel selection. The two constants must agree, but neither crate imports the other's constant.
**Why it is wrong**: The design says "Use one f64 mapping owner in View" and the task asks for no second mapping formula. The half-voxel formula now exists twice. The CPU test `dpr_two_keeps_the_left_half_of_texel_zero` guards the combination, but a change to one constant requires a matching change to the other in a different crate and the compiler cannot enforce it.
**Evidence**: Read of `view.rs:452-460`, `cpu.rs:19-25`, `cpu.rs:572-590`, and `cpu.rs:623-625`. The CPU test at `tests/cpu.rs:372-395` exercises the pair.
**Scope and regression**: The tier C raster and the shared View inverse. The smallest regression is to expose one constant from `ocelli-viewport` and use it in `cpu.rs`, or a cross-crate test that mutates the constant and checks both the View map and the CPU texel selection.

## Nitpicks

### N1, `css_from_device` is not exercised at dpr 1
**Where**: `crates/ocelli-render/src/extent.rs:121-135`
**What**: The acceptance names dpr 1, 1.25 and 2. The helper test covers `css_from_device(200.0, 2.0)` and `css_from_device(125.0, 1.25)` but not `css_from_device(x, 1.0)`.
**Why it is worth fixing**: The identity case is the one that would catch a special-case branch at dpr 1. The viewport fixture covers dpr 1 but performs the division itself.
**Evidence**: Read of `extent.rs:121-135`.
**Scope and regression**: The `css_from_device` contract. Add `assert_eq!(css_from_device(100.0, 1.0), Ok(100.0))`.

### N2, The render-graph LLD credits F-X003 but not F-048
**Where**: `docs/lld/render-graph.md:3`
**What**: The staged diff changes the F-IDs line to `F-038, F-039, F-040, F-042, F-043, F-044, F-045, F-X003`. The 80-byte `DeviceCamera` and the shader division are F-048's work. The line omits F-048.
**Why it is worth fixing**: The F-IDs line is the provenance record for the LLD. A reader cannot trace the camera-uniform change to its story.
**Evidence**: Read of `docs/lld/render-graph.md:1-5` and `docs/lld/viewport.md:3`. The viewport LLD includes F-048.
**Scope and regression**: The LLD provenance. Add F-048 to the list, or a workflow check that the F-IDs line includes every F-ID whose plan names the LLD.

### N3, The singular-map test's raster fallback assertion is tautological
**Where**: `crates/ocelli-viewport/tests/coordinates.rs:180-195`
**What**: After `set_zoom(Zoom::new(0.0))`, the test asserts `view.camera_matrix(4, 4)[15].is_finite()`. Element 15 is the homogeneous row and is `1.0` in both the centre fallback and the composed matrix. The assertion passes even if the fallback writes NaN in any of the fifteen meaningful components.
**Why it is worth fixing**: The design says `camera_matrix` may still return a finite raster fallback. The test does not check that claim.
**Evidence**: Read of `coordinates.rs:180-195` and `view.rs:242-283`. Both `centre_matrix` and the composed path write `1.0` at index 15.
**Scope and regression**: The AC6 raster-fallback claim. Assert all sixteen components are finite, or compare the fallback to `centre_matrix(2.0, 2.0)`.

### N4, `canvas_to_world` does not state the fitted-grid precondition
**Where**: `crates/ocelli-viewport/src/view.rs:341-380`
**What**: `canvas_to_world` and `world_to_canvas` use `plane.dimensions()` as the grid. `View` may have been fitted with a different grid. `canvas_to_index`'s doc says "Columns and rows are the grid `fit` used", but the world functions do not repeat that precondition. A caller that fits 512 by 512 and passes an 8 by 9 plane gets a silently wrong canvas point.
**Why it is worth fixing**: The design says `View` does not store the grid, so the caller must pass the right one. The world entry points do not say so.
**Evidence**: Read of `view.rs:341-430` and the design's "Problem and scope" and "Proposed API". `canvas_to_world` uses `plane.dimensions()` at line 374.
**Scope and regression**: Every measurement caller that mixes a fitted view with a different plane. Add the precondition to both docs, or a test that fits a view with a different grid and observes the wrong mapping.

## Verified clean

- AC1: `index_to_canvas(0, 0, 0)` is `image_to_canvas(0.5, 0.5)` on the axial plane. A mutation of `INDEX_TO_IMAGE_CENTRE` to 0.0 changes the y canvas coordinate by 1.0 CSS pixel, above the quarter-pixel tolerance, so the fixture is discriminating.
- AC2: The plane's `PixelSpacing::new(2.0, 0.5)` and `ImagePlane::index_to_world` produce world `(11, 26, 30)` for index `(2, 3, 0)`. A transposed plane gives `(14, 21.5, 30)`, which the independent `patient` fixture rejects.
- AC3: The oblique plane, pan, quarter turn, horizontal flip, zoom and F-049 shift round-trip the independent world point `(10 + s, 20 + s, 36)`. The canvas assertion uses the approved `image_to_canvas` composition.
- AC5: The `Pt<Canvas>`, `Pt<World>` and `Pt<Index>` markers and the `Transform<A, B>` parameter pair make a cross-space `apply` a compile error. The existing trybuild fixtures `apply_refuses_a_point_from_another_space.rs` and `then_refuses_a_transform_that_does_not_join.rs` cover the mechanism. The coordinate tests round-trip within `1e-6` and the quarter-pixel bound.
- AC6: Zoom 0 and zero spacing on both axes return `CoordinateError::Singular`. The view is not written by the mapping call. The rank-one and centre fallbacks remain inside `camera_matrix`.
- AC7: The mapping calls live in `ocelli-viewport`, which does not depend on `ocelli-render`. `set_view` writes only the camera buffer and sets `Dirty.camera`. The code path has no texture or brick write and no upload counter increment.
- The `DeviceCamera` layout is 16 column-major floats followed by four scale lanes, 80 bytes, and matches the WGSL `mat4x4<f32>` plus `vec4<f32>` layout. Registration, `set_view` and `rebind` all write the same 80 bytes.
- The `write_state` offsets and the TypeScript `readViewportState` offsets agree at 0, 64, 68, 72, 76 and 80, and both are 96 bytes.
- `PixelSpacing::new` takes row then column, `View::set_spacing` takes column then row, and `apply_placement` passes `spacing.column_mm()` then `spacing.row_mm()`. The product's order is correct.
- The `table_index` change rounds half up, matching `display_index` in `colour.rs`.

## Limitations and handoff

- The GPU tests `device_fragments_are_divided_by_dpr_before_the_css_camera` and `fractional_dpr_selects_the_derived_texels` are `#[ignore]`. They were not executed. No adapter, WebGL2 or browser DPR observation was performed in this pass.
- The compile-fail fixtures were not executed in this pass. The type-safety conclusion rests on the `Transform` signatures and the existing fixtures.
- `gate --sprint` and the oracle were not run, as the assignment prohibits them.
- The focused run `bin/ocelli.sh test ocelli-viewport --test coordinates` passed 6 of 6. It does not exercise the GPU camera or the tier C raster.
- AC4's GPU division remains unverified. D4 and S2 are the open risks. The next owner should remediate D1 to D4 and S1 to S3, then run a fresh independent pass on the changed tree, including an adapter-backed DPR 1.25 and DPR 2 observation if one is available.
