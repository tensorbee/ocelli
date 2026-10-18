# F-044 review, pass 2

**Reviewed**: working tree against 5e2b916
**Result**: 0 defects, 0 smells, 0 nitpicks

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Verified clean

The three documentation fixes were re-read in the files. They were not taken from the earlier review.

`docs/lld/render-graph.md` lines 95 to 105 say the unbound LINEAR finding read back above 255, within `0.001` of `297.5000305175781`, on an `Rgba32Float` target. They call that the existing shader tolerance and say it is not an 8-bit comparison with tier B. They say a tier B colour LUT readback was not run, so the plan's tier A against tier B failure condition was not measured. They say the ignored native GL test was run and found no adapter, and that this miss is not a tier B colour pass. Lines 172 to 174 say native `Backends::GL` found no adapter and that the miss is not the tier B run. Those two passages do not contradict each other. The section 18.3 paragraph under them is the greyscale path only. `bin/ocelli.sh test ocelli-render --test tier_b -- --ignored --test-threads=1 --nocapture native_gl_section_18_3_or_a_recorded_miss` exited 0 and printed `native Backends::GL found no adapter. This is not a tier B run.`

`docs/lld/pixel-pipeline.md` line 7 and `docs/lld/README.md` line 24 both list F-018, F-029, F-030, F-041, F-044. `docs/lld/render-graph.md` line 3 and `docs/lld/README.md` line 31 both list F-038, F-039, F-040, F-042, F-044. No sentence in the edited LLD files claims the tier A against tier B colour comparison was performed. `python3 scripts/prose_check.py` exited 0 over 380 tracked files. The checker reads `git ls-files`, so this untracked review is not in that count. This file has no em-dash and no prose semicolon. `git diff HEAD -- ci/wasm-size-budget.json` and `git diff HEAD -- crates/ocelli-render/shaders/voi.wgsl` are empty. Wasm was not rebuilt.

`ColourLut` is `Preset(Grey | Stops)` or `Palette`. `PresetId` has no clinical name. `preset_bytes` writes `(d, d, d, 255)` for every index in `0..256`. For `stops` it writes `(0, 0, 0)` at 0, `(255, 0, 0)` at 128, `(255, 255, 255)` at 255, and black elsewhere, with alpha 255. `ColourLut::palette` bakes one `ColorTransform::map_into` per stored value in the span. The shader mode 2 path indexes `stored - origin` and returns before `lut_chain`. Mode 1 calls `lut_chain` once and returns the texel. It does not also apply the greyscale normalisation. Mode 0 is that normalisation and does not clamp `display`. `let cp = c - 0.5` remains only in `shaders/voi.wgsl` line 57. `LINEAR_EXACT` in that file uses neither `c - 0.5` nor `w - 1`. This diff does not touch the file.

ICC, a palette on either monochrome photometric interpretation, and a preset on palette colour return `ColourLutError::Unavailable`. A channel outside `0..=255` is refused by `byte_from_channel` rather than scaled. That includes a 16-bit entry above 255. Tier C preset indexing is `map_preset_into` after `LutChain::map_into`. Tier C palette is `ColorTransform::map_into`. There is no second palette formula and no device on that path.

`select_colour_lut` creates an `Rgba8Unorm` texture of height 1 and fills it with `write_texture`. That is the only increment of `lut_uploads`. `set_voi` writes `bytemuck::bytes_of` of `VoiParams` and does not touch the colour texture or either counter. The registration placeholder is `ColourParams::NONE` (mode 0) and is not counted. `tick` has one `queue.submit`, at `graph.rs` line 1780. The test readback submits again in `read_rgb`, outside `tick`. `STORED_FORMAT` stays `R32Float`. `Rgba8Unorm` is not a stored-sample candidate. A device-loss rebind writes the remembered table through `colour_texture_for` and does not increment `lut_uploads`.

No new `unsafe`, no `as` cast, no `wasm_bindgen`, no new trait, and no `Box<dyn>` in the added Rust. `ocelli-wasm` is not in the diff, so there is no new wasm export and no pixel path into JavaScript. The LUT texture is created at selection, registration, or rebind, not inside `encode_frame`. wgpu 30.0.1 `wgpu-core` `device/resource.rs` accepts `Rgba8Unorm` on `TextureSampleType::Float { filterable: false }`. The plan's Tests table does not list a tier B colour draw. That omission is not a defect.

`bin/ocelli.sh test ocelli-render --test colour -- --test-threads=1` exited 0. Four CPU tests passed and the two GPU tests stayed ignored. `bin/ocelli.sh test ocelli-render --lib colour -- --test-threads=1` exited 0. Both shader-source tests passed. `bin/ocelli.sh test ocelli-render --test colour -- --ignored --test-threads=1` exited 0. Both ignored tests passed in 0.51s. A second run with `--nocapture` exited 0 in 0.67s and did not print the tier C skip. Both tests assert `Tier::A` before drawing, at `tests/colour.rs` lines 473 and 620, so the default adapter on this host is tier A. The finding assert is `red > 255.0` and `assert_channel_close(red, 297.500_03)` at tolerance `0.001`. `297.500_03` and `297.5000305175781` are the same `f32`, bits `0x4394c001`.

Mutation of the `stops` texel at index 128 in `preset_bytes`, from `[255, 0, 0]` to `[0, 255, 0]`, made `grey_at_display_40_is_rgb_40_and_stops_are_the_three_texels` fail. The assertion at `crates/ocelli-render/tests/colour.rs:179` reported left `[0, 255, 0, 255]` and right `[255, 0, 0, 255]`. The command exited 101. The constant was restored to `[255, 0, 0]`. `git status --short` shows the same paths as before the mutation, and `git diff HEAD` on `voi.wgsl` and the size budget is still empty.
