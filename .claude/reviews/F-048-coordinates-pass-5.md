# F-048 review, pass 5

**Reviewed**: staged tree `111010c41245e2f887a36dd0f00f4f642bc08957`, branch `sprint/s16`, HEAD `22800f8`, diff `/tmp/review-F-048-p5.diff`. Unstaged paths: none observed.
**Author / reviewer**: xAI initial implementation, Codex completion. Reviewer is an independent participant. The reviewer did not author F-048 or its repairs.
**Result**: 2 defects, 1 smell, 3 nitpicks.

## Acceptance challenged

AC2 and S1. The axial plane is 8 rows by 9 columns. The unfitted anchor is the image centre `(4.5, 4.0)`. Index `(2, 3)` is image `(2.5, 3.5)`, which is offset `(-2.0, -0.5)` from the centre. Column spacing 0.5 scales x to `-1.0`, row spacing 2.0 scales y to `-1.0`. The translation is the anchor, so the canvas point is `(3.5, 3.0)`. A transposed `set_spacing(2.0, 0.5)` scales x by 2.0 and y by 0.5, giving `(0.5, 3.75)`. The fixture asserts `(3.5, 3.0)` independently of `image_to_canvas`, so a transposed view spacing is red. I derived these values from the View composition and PS3.3 C.7.6.2.1.1, not from the test's output. The world point `(11, 26, 30)` is separately derived from the PS3.3 equation.

AC4. The non-ignored shader test now asserts the exact `css_x` and `css_y` lines and exactly two `/ scale` occurrences. I read the shader source and confirmed exactly two occurrences, both on those lines. A shader with `at.x / scale / scale` fails the exact line and the count. A shader that leaves `at.y` undivided fails the `css_y` line. The test still does not assert that the multiply consumes `css_x` and `css_y`, which is N5.

## Evidence

| Command or selected cases | Source and input identity | Executed now or prior receipt | Exit and observation |
|---|---|---|---|
| Read `/tmp/review-F-048-p5.diff` | staged tree `111010c...`, full F-048 diff | executed now | 2239 lines, prior review plus product and doc changes |
| Read `.claude/reviews/F-048-coordinates-pass-4.md` | prior pass, 4 defects, 3 smells, 4 nitpicks | executed now | each prior finding mapped below |
| Read `.claude/plans/F-048-design.md` | approved F-048 plan and repairs | executed now | AC1 to AC7, PS3.3 equation, dpr repair |
| Read `docs/lld/viewport.md`, `docs/lld/render-graph.md` | staged tree | executed now | shader division, 80-byte uniform, refusal prose |
| Read `crates/ocelli-render/src/extent.rs`, `graph.rs`, `cpu.rs`, `colour.rs` | staged tree | executed now | dpr refusal, 80-byte uniform, CPU raster, table_index |
| Read `crates/ocelli-viewport/src/view.rs`, `image_plane.rs`, `presentation.rs`, `ring.rs` | staged tree | executed now | index maps, PS3.3 plane, presentation and ring claims |
| Read `crates/ocelli-render/tests/view.rs`, `crates/ocelli-viewport/tests/coordinates.rs` | staged tree | executed now | D3, D4, S1, N3 observations |
| Read `packages/core/src/viewport.ts`, `viewport.test.ts` | staged tree | executed now | 96-byte readback and flags |
| Grep `set_view(`, `INDEX_TO_IMAGE_CENTRE`, `f32_from_f64`, `/ scale`, `IMAGE_FROM_INDEX` | staged tree | executed now | all set_view callers updated, exactly two `/ scale`, no `IMAGE_FROM_INDEX` |
| A focused `bin/ocelli.sh test` or a GPU test | none run | not executed | no adapter observation, the assignment prohibits ignored GPU tests and the oracle |
| `gate --sprint` and the oracle | prohibited | not executed | not an acceptance claim |

## Defects

### D5, `NonPositiveRatio` is documented as a zero or negative ratio but is returned for a positive subnormal dpr
**Where**: `crates/ocelli-render/src/extent.rs:31-32`, with the stale variant doc at `crates/ocelli-render/src/extent.rs:12-14`
**What**: The new check returns `ExtentError::NonPositiveRatio` for any `dpr` whose `f32` narrowing is not positive. The variant doc says "The device pixel ratio was zero or negative" and the message says "device pixel ratio must be strictly positive". For `dpr = f64::MIN_POSITIVE / 2.0`, the input is finite and positive, and the error is returned.
**Why it is wrong**: The microscope's prose rule treats a false claim about an error condition as a defect. The design says the GPU camera uniform and `device_extent` must carry the same ratio, so the refusal is deliberate, but the variant's contract was not updated with it.
**Evidence**: The extent test at `extent.rs:140-143` asserts `device_extent(1.0, 1.0, subnormal)` is `Err(NonPositiveRatio)`. The subnormal is positive, so the assertion proves the doc and the message are false for that input.
**Scope and regression**: Every caller that matches on `ExtentError` or reads its message. The smallest regression is an assertion that the `Display` message for a positive subnormal dpr names the `f32` narrowing, or a distinct variant for the narrowing refusal.

### D6, `DevicePixelRatio` says the ratio is not finite and strictly positive but refuses a finite positive subnormal
**Where**: `crates/ocelli-render/src/graph.rs:242-244`, with the `set_view` `# Errors` entry at `crates/ocelli-render/src/graph.rs:1524-1525`
**What**: The variant doc says "A device pixel ratio that is not finite and strictly positive". The message says "the device pixel ratio is not finite and strictly positive". The `set_view` errors list says the error is for a ratio that is "not finite and strictly positive". `set_view` also returns this error for a finite, strictly positive dpr whose `f32` narrowing is not positive, at `graph.rs:1553-1558`.
**Why it is wrong**: The message is false for the new input. A caller passing `f64::MIN_POSITIVE / 2.0` gets an error that says the ratio is not finite and strictly positive, which is the opposite of the input. The `# Errors` list omits the narrowing refusal.
**Evidence**: Read of `graph.rs:1553-1558`. The condition is `!dpr.is_finite() || dpr <= 0.0 || f32_from_f64(dpr) <= 0.0`. The first two clauses match the doc. The third does not.
**Scope and regression**: Every `set_view` caller that handles `GraphError::DevicePixelRatio`. The smallest regression is an ignored GPU assertion that a rebound stack viewport with `f64::MIN_POSITIVE / 2.0` returns `DevicePixelRatio`, plus a message assertion that names the narrowing. The ignored test at `tests/view.rs:217-220` only checks the unknown-id precedence.

## Smells

### S4, The `set_view` narrowing refusal is not exercised by any test
**Where**: `crates/ocelli-render/src/graph.rs:1553-1558` and `crates/ocelli-render/tests/view.rs:217-220`
**What**: The extent test covers the narrowing refusal in `device_extent` and `css_from_device`. No test covers the copy in `set_view`. The ignored GPU test asserts only `ViewportId(99)` with dpr `0.0`, which returns `UnknownViewport` before the ratio is read.
**Why it is a smell**: The GPU path would carry a zero scale and the shader would skip the division, while the CPU path refuses the same dpr. A future edit that drops the `set_view` copy would leave the suite green and restore the tier disagreement that S2 closed.
**Evidence**: Read of `graph.rs:1553-1558`, `tests/view.rs:217-220`, and the extent test at `extent.rs:132-143`.
**Scope and regression**: The GPU camera path. The smallest regression is an assertion in the ignored GPU test that a valid rebound stack viewport with `f64::MIN_POSITIVE / 2.0` is `DevicePixelRatio`, or a pure predicate for the dpr policy that the extent and graph paths both call and that is tested without an adapter.

## Nitpicks

### N5, The shader test does not assert that the multiply consumes the divided coordinates
**Where**: `crates/ocelli-render/tests/view.rs:438-447`
**What**: The test asserts the exact `css_x` and `css_y` lines and exactly two `/ scale` occurrences. It does not assert the multiply operands. A shader that declares both divided coordinates and then writes `camera.css * vec4<f32>(at.x, at.y, 0.0, 1.0)` passes all assertions.
**Why it is worth fixing**: The test's comment says the exact lines reject a missing division. The multiply-operand mutation is a missing division in the computation and stays green. Add an assertion that the shader contains the exact multiply line `camera.css * vec4<f32>(css_x, css_y, 0.0, 1.0)`.

### N6, The render-graph LLD omits the new device_extent refusal
**Where**: `docs/lld/render-graph.md:170-171`
**What**: The paragraph says "Non-finite inputs and a ratio that is not strictly positive are refused". The code now also refuses a positive ratio whose `f32` narrowing is not positive.
**Why it is worth fixing**: The LLD is the normative description of `device_extent`. A reader would believe a positive ratio is always accepted. Add the narrowing condition.

### N7, The viewport LLD says set_view uploads sixteen floats
**Where**: `docs/lld/viewport.md:7`
**What**: The opening paragraph says "`RenderGraph::set_view` uploads the sixteen floats `View::camera_matrix` returns". The upload is the 80-byte `DeviceCamera`, twenty floats. The detailed section below is correct.
**Why it is worth fixing**: The opening summary understates the uniform and conflicts with the 80-byte section. Say twenty floats, or say the CSS matrix plus the ratio.

## Verified clean

- Prior D1 closed. `extent.rs` now has a separate doc block for `device_extent` and a summary for `css_from_device`. The inverse function's doc describes the division without rounding and names both errors.
- Prior D2 closed. `docs/lld/viewport.md` and `View::camera_matrix` now say the shader divides the device position by the ratio first, then multiplies `(css_x, css_y, 0, 1)`. The stale "multiplies `at.x`" sentence is gone.
- Prior D3 closed. `set_view` checks `UnknownViewport`, then `PassHasNoUniform`, then `ViewportNotRebound`, then `DevicePixelRatio`. The doc's errors list has the same order. The ignored test asserts `ViewportId(99)` with dpr `0.0` is `UnknownViewport`.
- Prior D4 closed for a double division and a one-axis division. The shader has exactly two `/ scale` occurrences, on the exact `css_x` and `css_y` lines. A second division fails the exact line and the count. A one-axis division fails the `css_y` line. N5 records the remaining multiply-operand gap.
- Prior S1 closed. `anisotropic_index_2_3_is_world_11_26_30` asserts the hand-derived canvas `(3.5, 3.0)` independently of `image_to_canvas`. My derivation gives `(0.5, 3.75)` for a transposed `set_spacing(2.0, 0.5)`, so the fixture rejects a transposed view spacing.
- Prior S2 closed on behaviour. `device_extent`, `css_from_device` and `set_view` all refuse a dpr whose `f32` narrowing is not positive. The extent test asserts the subnormal case for `device_extent` and `css_from_device`. S4 records the missing `set_view` assertion.
- Prior S3 closed. `INDEX_TO_IMAGE_CENTRE` is public in `ocelli-viewport`, `cpu.rs` imports it for both axes, and `IMAGE_FROM_INDEX` is absent from the tree.
- Prior N1 to N4 closed. The extent test covers `css_from_device` at dpr 1. The render-graph F-IDs line includes F-048. The singular-map test asserts all sixteen components are finite. `canvas_to_world` and `world_to_canvas` state the fitted-grid precondition.
- The `DeviceCamera` layout is 16 column-major `f32` values followed by four scale lanes, 80 bytes, and matches the WGSL `mat4x4<f32>` plus `vec4<f32>` layout. Registration, `set_view` and `rebind` all build it through `device_camera`.
- All `set_view` callers are updated. `tools/oracle/src/gpu_candidate.rs:101` passes `1.0`. The test callers pass explicit ratios.
- `table_index` rounds half up and matches `display_index`. The `f32` narrowing is ties-to-even and the subnormal case returns zero.
- The CPU raster maps `CoordinateError::Singular` to `RasterError::Singular` and does not write the view. The half texel enters only through `INDEX_TO_IMAGE_CENTRE`.

## Limitations and handoff

- No adapter was opened. The GPU tests `set_view_dirties_only_the_camera_and_keeps_the_css_matrix`, `device_fragments_are_divided_by_dpr_before_the_css_camera` and `fractional_dpr_selects_the_derived_texels` were not executed. No browser or WebGL2 DPR observation was performed.
- The ignored `device_fragments_are_divided_by_dpr_before_the_css_camera` test accepts either `Submitted { viewports: 1, passes: 1 }` or `Submitted { viewports: 2, passes: 2 }` for the second tick. The second viewport is the only dirty one, so the `2, 2` branch would accept a tick that re-encodes a clean viewport. This is a test-strength observation, not a product defect.
- The AC3 canvas assertion uses `image_to_canvas`, which is the approved composition. The world point is the independent PS3.3 value. The canvas side is not independent.
- AC4's GPU division remains an unexecuted observation. The source assertions are green, but no rendered frame was observed in this pass.
- The oracle, `gate --sprint` and all `#[ignore]` GPU tests were not run, as the assignment requires.
- Next owner: correct the two error contracts (D5, D6), add the `set_view` narrowing assertion (S4), add the multiply-line assertion (N5), and update the two LLD sentences (N6, N7). Then run a fresh independent pass on the changed tree.
