# F-194 review, pass 3

**Reviewed**: working tree on `sprint/s42` over parent `c90bdcd` after the pass 2 remediation, as saved in `/tmp/ocelli-f194-pass3.diff`, plus the untracked `interpolation.rs` and `tests/interpolation.rs`. No tree hash. The reviewer had no shell.
**Author / reviewer**: implementation by the sprint host. Review by Claude Code, read only, with no shell.
**Result**: 0 defects, 1 smell, 1 nitpick

## Acceptance challenged

I checked each pass 2 item against the current source, then looked for a second copy of the coordinate claim that pass 2 D2 corrected. I also looked for any `-0.0` input that could split the CPU blend from the shader blend.

## Evidence

| Command / selected cases | Source and input identity | Executed now / prior receipt | Exit and observation |
|---|---|---|---|
| Read and grep only | working tree | executed now | No test was run in this pass |
| Grep `need_x` and mixed `&&` with `\|\|` | `graph.rs` | executed now | One mixed line, 764. It is parenthesised |
| Grep `display_scratch` assignments | `src/` | executed now | Only `reserve_source` assigns it, at lines 310 and 311 |

## Pass 2 items

1. **Footprint `if`.** `graph.rs:764` reads `if ((need_x && x0 + 1.0 >= f32(dims.x)) || (need_y && y0 + 1.0 >= f32(dims.y)))`. The source test at `tests/interpolation.rs:135` requires that exact parenthesised substring, so taking out either pair of parentheses turns the test red. No other line in `GRAPH_WGSL` mixes `&&` and `||`. The `tier_b_fragment_module` that the test reads is `VOI_WGSL`, `TABLE_INDEX_WGSL` and `GRAPH_WGSL` joined, so the test reads the module the browser compiles. Fixed.
2. **Nearest coordinate.** `nearest` still calls `texel_of` on the raw coordinate. The `blend_display` doc now says linear treats 0 as the centre of texel 0 and `texel_of` reads a corner-origin coordinate. It says the approved pair at `(0.5, 0.5)` is not one geometric point. It also says the raster passes an index to linear and `index + 0.5` to `texel_of`, which matches `cpu.rs:861`. The fixture still asserts nearest 0 and linear 138.75, which agrees with design line 41 and AC2. Fixed for `blend_display`. See S1 for the sibling doc.
3. **Unreserved linear draw.** `fill_display` returns `Extent` when `display_scratch.len() < stored.len()`, and it does so before any write to `out`. `gate` reads `out` but does not write it. `display_scratch` starts empty and only `reserve_source` assigns it. `linear_without_a_reserve_refuses_and_does_not_allocate` asserts `Extent`, then `out == [4, 4, 4, 4]`, then that `allocations` is unchanged. If a `Vec` replaced the reserve inside `fill_display`, the draw would succeed and the `Extent` assertion would go red even with the counter untouched. Fixed.

## Defects

None.

## Smells

### S1, `TierSample` still documents one coordinate for both modes

**Where**: `interpolation.rs:149` and `:151`, the `x` and `y` field docs on the public `TierSample`.

**Mechanism**: both fields say "coordinate after the centre shift", and the struct carries a `mode` that can be `Nearest`. `blend_display` now says nearest takes a corner-origin coordinate and linear takes a centre-origin one. If the centre shift means the `+ 0.5` in `cpu.rs:24`, the doc is wrong for linear. If it means the index space `centre_sample` returns, it is wrong for nearest. Either way a `blend_on_tier` caller that follows it gets one mode half a texel off. This is the class pass 2 D2 named, left in a second place. The phrase comes from design line 41, so the fix is documentation only: point the fields at `blend_display`'s coordinate rule. No approved number moves. There is no caller today, which is why this is a smell and not a defect.

## Nitpicks

- `blend_points` tests for a zero fraction by comparing bits with `+0.0`. `floor_fraction(-0.0, _)` gets past the `value < 0.0` check and returns a fraction of `-0.0`, so `blend_display(1, &[5.0], -0.0, 0.0, Linear)` asks for column 1 and returns `Outside`. The shader's `tx != 0.0` treats `-0.0` as zero and would draw 5. The raster cannot produce `-0.0` through `centre_sample`, so only a direct public caller can reach this.

## Verified clean

- Line 764 is the only line in `GRAPH_WGSL` that mixes `&&` and `||`.
- The shader's linear branch and the CPU path agree on:
  - subtracting half a texel
  - refusing a negative shifted coordinate
  - skipping the next texel at a zero fraction
  - setting `p11` to zero unless both fractions are non-zero
- The raster's nearest path still uses `texel_of(index + INDEX_TO_IMAGE_CENTRE)`.
- `fill_rgb_planes` checks the reserved length before it writes.
- `write_mono_linear` and `write_color_linear` run every fill before their first `write_pixel`, so a refusal leaves the caller's buffer untouched.
- The mode 2 test in `cpu.rs` asserts that linear is kept and that the next settled draw does not write.
- The `allocations()` change from 1 to 2 in `settled_idle_rejects_a_source_that_does_not_match_the_layout` matches the second buffer that `reserve_source` now allocates.
- The page, the changelog and the helper test all show 138.75 for linear and the nearest corner for nearest.
- No `unsafe`, no sampler, no new `wasm-bindgen`.

## Limitations and handoff

No test or shader compiler was run. The Tint finding still rests on the grammar and not on a Dawn run. The implementer fixes S1, and can optionally use an ordinary float comparison for the nitpick. Pass 4 reviews that tree. This pass is not sprint readiness by itself.
