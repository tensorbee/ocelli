# F-194 review, pass 4

**Reviewed**: the working tree on `sprint/s42` over parent `c90bdcd`, as saved in `/tmp/ocelli-f194-pass4.diff`, plus the current source of the untracked `interpolation.rs`, `tests/interpolation.rs` and `examples/example-server/src/interpolate.rs`. No tree hash. The reviewer had no shell.
**Author / reviewer**: implementation by the sprint host. Review by Claude Code, read only, with no shell.
**Result**: 0 defects, 0 smells, 0 nitpicks

## Pass 3 items

1. **S1, the `TierSample` coordinate docs.** `interpolation.rs:149` now says "Column coordinate. Nearest and linear do not share an origin." and points at `blend_display`. Line 152 says `y` follows the same rule as `Self::x`. The words "centre shift" no longer appear anywhere under `crates/ocelli-render`, in the LLD page, in the README, in the changelog or in `interpolate.rs`. The `blend_display` doc at lines 91 to 95 still says the approved pair at `(0.5, 0.5)` is nearest texel 0 and linear 138.75, and that those are not one geometric point. No formula moved. Fixed.
2. **Nitpick, the `-0.0` fraction.** `fraction_is_zero` at line 247 is `value.abs().to_bits() == 0.0_f32.to_bits()`. Three places call it: `optional_neighbour` (line 257) for `p10` and `p01`, and both halves of the `p11` guard (line 226). Nothing left in `interpolation.rs` compares a fraction against `+0.0` bits directly. Fixed.

## Remediation checked for a new defect

- **`-0.0` on x.** I traced `blend_display(1, &[5.0], -0.0, 0.0, Linear)` by hand:
  - `floor_fraction(-0.0, 1)` passes `is_finite`. `-0.0 < 0.0` is false and `-0.0 >= 1.0` is false.
  - The search loop does not run, so the function returns `(0, -0.0)`.
  - `fraction_is_zero(-0.0)` is true, so column 1 is never requested.
  - The weights come out as `w00 = 1`, `w10 = -0`, `w01 = 0` and `w11 = -0`. The sum is `5 + (-0) + 0 + (-0)`, which is `+5.0`, so the bit comparison holds.
  - Without `abs`, `optional_neighbour` asks for column 1, which is not below width 1, and the call returns `Outside`. That is the `result.is_ok()` failure the host's mutation receipt reports.
- **`-0.0` on y.** `ty` goes through the same `floor_fraction` and then through the same helper, both in `optional_neighbour` for `p01` and in the `p11` guard. So `-0.0` on y also skips the missing row. No test covers the y axis on its own. A mutation of the shared helper is still caught by the x-axis test, so I am not raising this.
- **Shader agreement.** At `graph.rs:762` and `:763`, `need_x = tx != 0.0` and `need_y = ty != 0.0` are IEEE comparisons, so `-0.0` counts as zero there too. The footprint guard at line 764, the `need_x && need_y` gate on `p11` at line 793, and the weights at line 796 match `blend_points` term for term. So the CPU and the shader now give the same answer for a signed zero fraction. In the shader, `floor(-0.0) = -0.0` gives `tx = +0`, which is zero either way.
- **Approved values.** `interpolation_nearest_follows_texel_of` still asserts nearest 0 at `(0.5, 0.5)`, and `interpolation_linear_centre_is_one_quarter_of_each_texel` still asserts 138.75. This agrees with design line 41 and AC2. The helper route still calls `blend_display` with `(0.5, 0.5)` for both modes.
- **Raster origins.** `cpu.rs:907` `centre_sample` passes the raw index, with no `+ 0.5`, to the linear paths at lines 731 and 804. The nearest path at `cpu.rs:861` and `:864` still uses `texel_of(index + INDEX_TO_IMAGE_CENTRE, ...)`. That matches what the `blend_display` doc says the raster does.
- **Receipts against the source.** `tests/interpolation.rs` has 9 `#[test]` functions, and `interpolation_linear_negative_zero_is_the_texel_centre` at line 57 is one of them. That matches the reported 9 passed. The test reads `bits(value) == bits(5.0)` through `opened`, which asserts `is_ok` first. That is consistent with where the mutation receipt says it failed.

## Verified clean

- No doc in the crate, the LLD page, the README, the changelog or the helper claims one origin for both modes.
- The shader source test still requires the parenthesised footprint `if` and the four-term weight string.
- The test has no `sampler` and adds no `unsafe` or new `wasm-bindgen`.
- The helper's `number` formatter compares against `+0.0` bits. It only formats output, and the nearest value it receives is the stored `0.0` sample, so it does not affect the blend.

## Limitations and handoff

I did not run any test, clippy or shader compiler. The three host receipts are prior evidence, and the source still matches them. The Tint grammar finding from earlier passes still rests on the grammar and not on a Dawn run. This pass reports zero defects and zero smells, but it is not sprint readiness by itself.
