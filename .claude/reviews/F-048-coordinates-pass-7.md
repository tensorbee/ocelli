# F-048 review, pass 7

**Reviewed**: staged tree `2cd9844d99ec3fedb2c044bfc46512773e6764b3`, branch `sprint/s16`, HEAD `22800f8`, diff `/tmp/review-F-048-p7.diff`. Unstaged paths: none observed. The staged tree also contains unrelated staged skill and workflow files outside the supplied F-048 diff. They were unchanged between pass 6 and pass 7.
**Author / reviewer**: xAI initial implementation, Codex completion. Reviewer is an independent participant. The reviewer did not author F-048 or its repairs.
**Result**: 0 defects, 0 smells, 0 nitpicks.

## Acceptance challenged

AC4. The remediation asserts that a valid registered stack viewport with `f64::MIN_POSITIVE / 2.0` returns `DevicePixelRatio`. I derived the input independently. `f64::MIN_POSITIVE` is the smallest positive normal f64, `2^-1022`. Halving it gives `2^-1023`, a positive subnormal. The smallest positive f32 subnormal is `2^-149`. Since `2^-1023` is below that, `f32_from_f64` maps it to `+0.0`. `usable_ratio` requires `f32_from_f64(dpr) > 0.0`, so it returns false. `set_view` checks `usable_ratio` after the id, pass and rebound checks and before any buffer write. A freshly registered stack viewport has a GPU slot from `register`, so the call reaches the ratio branch and returns `DevicePixelRatio` without writing. If the ratio check were removed, the call would return `Ok(())` and upload a zero scale, so the assertion rejects that regression. The assertion is after `graph.register` and before the later `set_view(viewport, camera, 1.0)`, so no accepted ratio is used before the refusal.

AC2. The axial plane is 8 rows by 9 columns. The unfitted anchor is the image centre `(4.5, 4.0)`. Index `(2, 3)` is image `(2.5, 3.5)`, offset `(-2.0, -0.5)` from the centre. Column spacing 0.5 scales x to `-1.0`. Row spacing 2.0 scales y to `-1.0`. The translation is the anchor, so the canvas point is `(3.5, 3.0)`. A transposed `set_spacing(2.0, 0.5)` gives `(0.5, 3.75)`. The fixture asserts `(3.5, 3.0)` independently of `image_to_canvas`, so it rejects the transpose. The remediation does not touch this fixture.

## Evidence

| Command or selected cases | Source and input identity | Executed now or prior receipt | Exit and observation |
|---|---|---|---|
| Read `/tmp/review-F-048-p7.diff` | staged tree `2cd9844...`, full F-048 diff, 2486 lines | executed now | prior reviews plus product and doc changes |
| `git rev-parse HEAD` and `git write-tree` | staged tree `2cd9844...`, HEAD `22800f8` | executed now | tree matches the assignment |
| `git diff 26fb29182f26297f165b09606c19f5b3056e7a6d 2cd9844d99ec3fedb2c044bfc46512773e6764b3 --stat` | pass 6 tree to pass 7 tree | executed now | 4 files, 88 insertions, 2 deletions. Product delta is 3 files, 10 insertions, 2 deletions |
| Read the pass 7 delta for `cpu.rs`, `graph.rs` and `tests/view.rs` | pass 6 tree to pass 7 tree | executed now | import reorder, three-line error attribute, narrowing assertion |
| Read `extent.rs`, `graph.rs` and `view.rs` | staged tree | executed now | `usable_ratio`, `f32_from_f64` and `set_view` validation order |
| Read `tests/view.rs:195-245` | staged tree | executed now | assertion after registration and before the accepted ratio |
| `cargo fmt --all --check` | staged tree, whole workspace | executed now | exit 0 |
| `bin/ocelli.sh test ocelli-render --test view the_camera_uniform_carries_dpr_and_the_shader_divides_first` | staged tree, one non-ignored source test | executed now | exit 0, 1 passed, 0 failed, 0 ignored, 5 filtered out |
| GPU tests `set_view_dirties_only_the_camera_and_keeps_the_css_matrix`, `device_fragments_are_divided_by_dpr_before_the_css_camera` and `fractional_dpr_selects_the_derived_texels` | `#[ignore]`, need a real adapter | not executed | no adapter observation |
| Oracle and `gate --sprint` | prohibited by the assignment | not executed | not an acceptance claim |

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

- Prior D1 closed. The `DevicePixelRatio` attribute is the rustfmt three-line form at `crates/ocelli-render/src/graph.rs:244-246`. The message is unchanged. `cargo fmt --all --check` exits 0 on the staged tree, so the `fmt` gate claim holds.
- Prior N1 closed. `crates/ocelli-render/tests/view.rs:228-231` asserts that `set_view(viewport, camera, f64::MIN_POSITIVE / 2.0)` is `DevicePixelRatio`. The assertion is after `graph.register` for the stack viewport at `tests/view.rs:222-225` and before the volume registration and the later `set_view(viewport, camera, 1.0)` at `tests/view.rs:268`. The viewport has a GPU slot from `register`, so the ratio branch is reached.
- The import order in `crates/ocelli-render/src/cpu.rs:14` is `{CoordinateError, INDEX_TO_IMAGE_CENTRE, View}`. `cargo fmt --all --check` accepts it. All three names are used.
- `set_view` still validates in the documented order: unknown id, pass without a stack uniform, viewport not rebound, then ratio. The `# Errors` list has the same order.
- `usable_ratio` is the shared predicate for `device_extent`, `css_from_device` and `set_view`. The subnormal input is refused by all three.
- The focused non-ignored shader test still passes. The test binary compiles the new ignored assertion.
- The pass 6 product and doc conclusions are unchanged by this delta. The 80-byte uniform, the shader division, the 96-byte readback and the coordinate fixtures are not touched.

## Limitations and handoff

- The ignored GPU test `set_view_dirties_only_the_camera_and_keeps_the_css_matrix` was not executed. The new narrowing assertion is source-level evidence only. No adapter, WebGL2 or browser DPR observation was performed.
- The narrowing refusal is reached only by an ignored test. The CI floor still does not execute the `set_view` `DevicePixelRatio` branch. This is the known GPU-gate limitation, not a new finding.
- The staged tree `2cd9844` contains unrelated staged skill and workflow files outside the supplied F-048 diff. They were unchanged between pass 6 and pass 7 and are not reviewed here.
- The oracle and `gate --sprint` were not run, as the assignment prohibits them.
- No new defects, smells or nitpicks were introduced by the remediation. This pass is clean for the supplied F-048 diff and the changed material. The next owner should run the ignored GPU test on an adapter-backed environment before closing the F-048 GPU evidence.
