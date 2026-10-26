# F-048 review, pass 6

**Reviewed**: staged tree `26fb29182f26297f165b09606c19f5b3056e7a6d`, branch `sprint/s16`, HEAD `22800f8`, diff `/tmp/review-F-048-p6.diff`. Unstaged paths: none observed (`git diff --stat` empty, the working tree matches the index).
**Author / reviewer**: xAI initial implementation, Codex completion. Reviewer is an independent participant. The reviewer did not author F-048 or its repairs.
**Result**: 1 defect, 0 smells, 1 nitpick.

## Acceptance challenged

AC4. The fractional dpr fixture uses dpr 1.25, a target width of 5, a source width of 4 and the identity CSS camera. Device fragment centres are x = 0.5, 1.5, 2.5, 3.5, 4.5. Dividing by 1.25 gives CSS 0.4, 1.2, 2.0, 2.8, 3.6. The identity camera maps CSS to image coordinates, and the shader truncates toward zero, so the selected source texels are 0, 1, 2, 2, 3. With stored values -160, 40, 4000, -160, PS3.3 C.11.2.1.2.1 gives 0, 128, 255, 255, 0. A missing division selects 0.5, 1.5, 2.5, 3.5, 4.5 and therefore texels 0, 1, 2, 3, outside, which reads 0, 128, 255, 0, black. A double division selects 0.32, 0.96, 1.6, 2.24, 2.88 and therefore texels 0, 0, 1, 2, 2, which reads 0, 0, 128, 255, 255. The fixture rejects both. I derived these values from the device centres and the shader truncation, not from the fixture's expected array. The fixture is `#[ignore]`, so the pixel-level claim was not executed in this pass. The non-ignored source test now asserts the exact `css_x` and `css_y` lines, exactly two `/ scale` occurrences, and the multiply line consumes `css_x` and `css_y`. It rejects a missing division, a double division and a one-axis division in the source text. It does not execute the shader.

AC2. The axial plane is 8 rows by 9 columns. The unfitted anchor is the image centre (4.5, 4.0). Index (2, 3) is image (2.5, 3.5), offset (-2.0, -0.5) from the centre. Column spacing 0.5 scales x to -1.0. Row spacing 2.0 scales y to -1.0. The translation is the anchor, so the canvas point is (3.5, 3.0). A transposed `set_spacing(2.0, 0.5)` gives (0.5, 3.75). The fixture asserts (3.5, 3.0) independently of `image_to_canvas`, so it rejects the transpose. The world point (11, 26, 30) is separately derived from the PS3.3 C.7.6.2.1.1 equation.

## Evidence

| Command or selected cases | Source and input identity | Executed now or prior receipt | Exit and observation |
|---|---|---|---|
| Read `/tmp/review-F-048-p6.diff` | staged tree `26fb291...`, full F-048 diff, 2387 lines | executed now | prior reviews plus product and doc changes |
| `git diff 111010c41245e2f887a36dd0f00f4f642bc08957 26fb29182f26297f165b09606c19f5b3056e7a6d --stat` | pass 5 tree to pass 6 tree | executed now | 6 files, 137 insertions, 17 deletions. Product and doc delta is 5 files, 44 insertions, 17 deletions |
| Read the pass 6 delta for `extent.rs`, `graph.rs`, `tests/view.rs`, `render-graph.md`, `viewport.md` | pass 5 tree to pass 6 tree | executed now | `usable_ratio` extraction, error contract updates, one shader assertion, two doc sentences |
| Read `.claude/reviews/F-048-coordinates-pass-5.md` | prior pass, 2 defects, 1 smell, 3 nitpicks | executed now | each prior finding mapped below |
| Read `.claude/plans/F-048-design.md`, `docs/lld/viewport.md`, `docs/lld/render-graph.md` | staged tree | executed now | AC1 to AC7, the 80-byte uniform and the narrowing refusal |
| `bin/ocelli.sh test ocelli-render --test view the_camera_uniform_carries_dpr_and_the_shader_divides_first` | staged tree, one non-ignored source test | executed now | exit 0. 1 passed, 0 failed, 0 ignored, 5 filtered out |
| `rustfmt --check --edition 2024 crates/ocelli-render/src/extent.rs crates/ocelli-render/src/graph.rs crates/ocelli-render/tests/view.rs` | staged tree, changed Rust files | executed now | exit 1. One diff at `graph.rs:241`, rewriting the `#[error]` at line 244 |
| Read `extent.rs`, `graph.rs`, `view.rs`, `commands.rs`, `ring.rs`, `cpu.rs`, `viewport.ts`, `viewport.test.ts` | staged tree | executed now | predicate use, validation order, 80-byte uniform, 96-byte readback |
| GPU tests `set_view_dirties_only_the_camera_and_keeps_the_css_matrix`, `device_fragments_are_divided_by_dpr_before_the_css_camera` and `fractional_dpr_selects_the_derived_texels` | `#[ignore]`, need a real adapter | not executed | no adapter, WebGL2 or browser DPR observation |
| Oracle and `gate --sprint` | prohibited by the assignment | not executed | not an acceptance claim |

## Defects

### D1, The new `DevicePixelRatio` message exceeds rustfmt's 100-column limit and fails the `fmt` gate
**Where**: `crates/ocelli-render/src/graph.rs:244`
**What**: The pass 6 message change produced `#[error("the device pixel ratio must be finite, strictly positive and narrow to a positive f32")]`, which is 101 characters. `rustfmt.toml` sets `max_width = 100`. `rustfmt --check --edition 2024 crates/ocelli-render/src/graph.rs` exits 1 and reports a diff at `graph.rs:241`, rewriting the attribute to a three-line form. The `fmt` gate runs `cargo fmt --all --check`, so the staged tree fails that gate.
**Why it is wrong**: `bin/ocelli.sh gate --floor` includes `fmt`, so the overlong attribute blocks the CI floor and the sprint gate. The pass 5 message was shorter and formatted. The pass 6 wording introduced the overlong line.
**Evidence**: `rustfmt --check --edition 2024 crates/ocelli-render/src/extent.rs crates/ocelli-render/src/graph.rs crates/ocelli-render/tests/view.rs` exits 1 with `Diff in .../crates/ocelli-render/src/graph.rs:241:` and the replacement `#[error(` plus the message on its own line plus `)]`. `awk 'NR==244 {print length($0)}'` reports 101. `rustfmt.toml` reports `max_width = 100`.
**Scope and regression**: Every `fmt` gate run on this tree. The smallest meaningful regression is `bin/ocelli.sh fmt`, which must exit 0. The fix is the rustfmt-suggested multi-line attribute.
```suggestion
    #[error(
        "the device pixel ratio must be finite, strictly positive and narrow to a positive f32"
    )]
```

## Smells

None. The pass 6 delta is behaviour-preserving. `usable_ratio` is the same finite, positive and narrowing-to-positive-f32 policy that `device_extent`, `css_from_device` and `set_view` used inline in pass 5. The doc and message changes are accurate for the inputs that reach them. The shader assertion closes N5.

## Nitpicks

### N1, The `set_view` narrowing refusal is still not reached by a non-ignored test
**Where**: `crates/ocelli-render/src/graph.rs:1554`
**What**: The new unit test at `extent.rs:129` tests `usable_ratio`, not the `set_view` call. The ignored `set_view_dirties_only_the_camera_and_keeps_the_css_matrix` calls `set_view(ViewportId(99), camera, 0.0)` and gets `UnknownViewport` before the ratio check, then calls `set_view` with a valid viewport only at dpr 1.0. No test reaches the `DevicePixelRatio` branch. A future edit that removes the `if !crate::extent::usable_ratio(dpr)` block would leave the CI floor green and restore the GPU zero-scale fallback that S2 closed.
**Why it is worth fixing**: The predicate is tested and the call is visible in the source, so this is a test-strength gap rather than a product defect. The pass 5 S4 failure mechanism is narrowed but not eliminated.
**Evidence**: Read of `graph.rs:1554-1556` and `tests/view.rs:204-311`. The only dpr 0.0 call uses `ViewportId(99)`.
**Scope and regression**: The GPU camera path. The smallest regression is an assertion in the ignored GPU test that a valid rebound stack viewport with `f64::MIN_POSITIVE / 2.0` returns `DevicePixelRatio`, or a pure validation function that the test can call without an adapter.

## Verified clean

- Prior D5 closed. `extent.rs:12-16` and `extent.rs:15` now name the positive-subnormal narrowing refusal in both the doc and the message. The doc covers zero, negative and a positive ratio that narrows to a non-positive f32. The message says "device pixel ratio must be positive and narrow to a positive f32".
- Prior D6 closed. `graph.rs:242-244` names the narrowing refusal in the doc and the message. `graph.rs:1525-1526` adds it to the `set_view` errors list. The code order at `graph.rs:1530-1556` is UnknownViewport, PassHasNoUniform, ViewportNotRebound, DevicePixelRatio, which matches the doc.
- Prior S4 closed as specified. `usable_ratio` at `extent.rs:25` is used by `device_extent` at `extent.rs:43`, `css_from_device` at `extent.rs:64` and `set_view` at `graph.rs:1554`. The unit test at `extent.rs:129-138` covers 1.0, 1.25, 2.0, 0.0, -1.0, NaN, infinity and `f64::MIN_POSITIVE / 2.0`. N1 records the remaining wiring test gap.
- Prior N5 closed. `tests/view.rs:445` asserts the multiply line contains `camera.css * vec4<f32>(css_x, css_y, 0.0, 1.0)`. The focused non-ignored test passed.
- Prior N6 closed. `docs/lld/render-graph.md:170-173` now names the positive ratio whose f32 narrowing is not positive, and says that condition keeps the CPU extent and the GPU camera uniform on one policy.
- Prior N7 closed. `docs/lld/viewport.md:7-8` now says `set_view` uploads the CSS matrix plus the device pixel ratio, twenty floats in one 80-byte uniform.
- The `DeviceCamera` layout is 16 column-major `f32` values followed by four scale lanes, 80 bytes, and matches the WGSL `mat4x4<f32>` plus `vec4<f32>` layout. Registration, `set_view` and `rebind` all build it through `device_camera`.
- The 96-byte state readback offsets agree between `write_state` in `commands.rs:318-345` and `readViewportState` in `viewport.ts`: camera at 0, centre at 64, width at 68, slice at 72, count at 76, flags at 80, padding at 84 to 96. Flag bit 3 is 8.
- The shader `table_index` rounds half up and matches `display_index` in `colour.rs`. The composed tier B module contains exactly two `/ scale` occurrences, both on the `css_x` and `css_y` lines. `VOI_WGSL` contains none.
- All `set_view` callers pass the new dpr argument, including `tools/oracle/src/gpu_candidate.rs:101` with 1.0. No stale three-argument call remains.
- `usable_ratio` is the same acceptance policy as the pass 5 inline checks. No behaviour change in `device_extent`, `css_from_device` or `set_view`.
- The TS test at `viewport.test.ts` reads a copied 96-byte block and refuses a 95-byte copy.

## Limitations and handoff

- No adapter was opened. The GPU tests `set_view_dirties_only_the_camera_and_keeps_the_css_matrix`, `device_fragments_are_divided_by_dpr_before_the_css_camera` and `fractional_dpr_selects_the_derived_texels` were not executed. No browser or WebGL2 DPR observation was performed. AC4's pixel-level claim remains an unexecuted observation.
- The oracle and `gate --sprint` were not run, as the assignment prohibits them. The `fmt` gate was checked with a focused `rustfmt --check` on the changed Rust files, not a full gate run.
- `docs/lld/README.md:31` still lists the render-graph LLD without F-048, while `docs/lld/render-graph.md:3` includes it. This is a provenance inconsistency for the next owner.
- `docs/sprints/AS_BUILT.md:3522` is the F-045 entry and still says `set_view` writes sixteen floats. It is a historical record. The F-048 AS_BUILT entry is absent from this tree and is the next owner's completion work.
- The CPU path uses f64 for the camera map and the dpr division, while the GPU uses f32. `usable_ratio` aligns acceptance, not the last bits of the mapping. This is a pre-existing precision difference, not introduced by the pass 6 remediation.
- The ignored `device_fragments_are_divided_by_dpr_before_the_css_camera` test accepts either `Submitted { viewports: 1, passes: 1 }` or `Submitted { viewports: 2, passes: 2 }` for the second tick. The `2, 2` branch would accept a tick that re-encodes a clean viewport. This is a test-strength observation, not a product defect.
- Next owner: fix D1 with the rustfmt-suggested attribute, consider N1, then run a fresh independent pass on the new tree. This pass is not clean because D1 fails the `fmt` gate.
