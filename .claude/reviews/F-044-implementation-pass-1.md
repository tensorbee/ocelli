# F-044 review, pass 1

**Reviewed**: working tree against 5e2b916
**Result**: 3 defects, 0 smells, 0 nitpicks

## Defects

### D1, native GL is said not to have been attempted

**Where**: docs/lld/render-graph.md:102
**What**: The colour measurement paragraph says native `Backends::GL` was not attempted.
**Why it is wrong**: That sentence is false in a current-state document. This story ran the ignored native GL test, and the test recorded a miss. The same file later says native GL found no adapter. The miss is still not a tier B colour pass, and saying the attempt did not happen is a different claim.
**Evidence**: `.claude/scratch/F-044-progress.md` records `bin/ocelli.sh test ocelli-render --test tier_b -- --ignored --test-threads=1 --nocapture native_gl_section_18_3_or_a_recorded_miss` exiting 0 and printing `native Backends::GL found no adapter. This is not a tier B run.` `docs/lld/render-graph.md:171` records the same miss. The review assignment states that this test was run.

### D2, the tolerance sentence describes a comparison that was not made

**Where**: docs/lld/render-graph.md:101
**What**: "Tolerance stayed 0.001 after the 8-bit quantisation both sides share" uses the plan's two-sided failure wording as a result of this measurement.
**Why it is wrong**: The plan's Boundary and tier section says a tier B result that differs from tier A by more than 0.001, after the 8-bit quantisation both sides share, is a failure. "Both sides" there means tier A and tier B. Tier B colour was not run, so that pair does not exist. The sentence before this one is the carried LINEAR finding. That readback is an `Rgba32Float` value the test requires to stay above 255. It is not an 8-bit texel, and the 0.001 check on it is not an 8-bit quantisation shared with a second tier.
**Evidence**: `crates/ocelli-render/tests/colour.rs` keeps `TOLERANCE` at 0.001. The unbound finding asserts `red > 255.0` and `assert_channel_close(red, 297.500_03)`. `bin/ocelli.sh test ocelli-render --test colour -- --ignored --test-threads=1` exited 0, both ignored tests passed, in 0.94s. The next LLD sentence says a tier B colour LUT readback was not run.

### D3, the LLD index omits F-044

**Where**: docs/lld/README.md:24 and docs/lld/README.md:31
**What**: The index F-IDs for `pixel-pipeline.md` stop at F-041. The index F-IDs for `render-graph.md` stop at F-040. Both file headers now list F-044. The render-graph header also lists F-042, which that index row still omits.
**Why it is wrong**: `docs/lld/README.md` lines 10 to 18 say the F-IDs column is the same current-state list as each file header, and that an edit to an LLD file updates its index row in the same hand. This diff edited both headers. The index is now a false list of who contributed. The story owns that list because it changed the headers the index claims to mirror.
**Evidence**: `docs/lld/pixel-pipeline.md:7` lists F-044. `docs/lld/render-graph.md:3` lists F-042 and F-044. `git diff HEAD -- docs/lld/README.md` is empty.

## Smells

None.

## Nitpicks

None.

## Verified clean

The missing WebGL2 colour draw is not a defect. The plan's Tests table lists CPU fixtures, one GPU readback, the upload counter, the index clamp, and the three `Unavailable` cases. It does not list a tier B draw. Boundary and tier says tier B is full because `Rgba8Unorm` is a WebGL2 sampled format, the same bytes are uploaded, and a tier B result that differs from tier A by more than 0.001 after the shared 8-bit quantisation is a failure. That is a failure condition for a result. It is not an instruction to produce the result in this story. The implementation uses one `Rgba8Unorm` upload on every GPU tier. wgpu 30.0.1 accepts that format on `TextureSampleType::Float { filterable: false }`, the same binding as the stored float texture (`wgpu-core` `device/resource.rs`, the arm commented "if we expect non-filterable, accept anything float"). `docs/lld/pixel-pipeline.md` and `docs/lld/render-graph.md` both say a tier B colour comparison was not run. That is the honest claim. The native GL miss is not counted as a tier B pass.

`bin/ocelli.sh test ocelli-render --test colour -- --test-threads=1` exited 0. Four CPU tests passed and the two GPU tests stayed ignored. The ignored run above then passed both GPU tests on this host, which asserts tier A before drawing. `bin/ocelli.sh test ocelli-render --lib colour -- --test-threads=1` exited 0. Both shader-source tests passed.

`ColourLut` is `Preset(Grey | Stops)` or `Palette`. There is no clinical name. `grey` writes `(d, d, d, 255)` for every index in `0..256`. `stops` writes `(0, 0, 0)` at 0, `(255, 0, 0)` at 128, `(255, 255, 255)` at 255, and black elsewhere. The CPU test checks every index, not only the three named ones. The known stops mutation was not repeated. `tests/colour.rs:179` compares texel 128 to `[255, 0, 0, 255]`, so the recorded substitution to `[0, 255, 0]` cannot stay green. The progress note records that red run and the revert.

Palette bytes are one `ColorTransform::map_into` per baked stored value. The shader indexes `stored - origin` and does not walk a descriptor. Mode 2 returns before `lut_chain`. The fixture first mapped input is 10. Stored 9 and stored 10 are the first entry `(11, 22, 33)`. Stored 11 is `(44, 55, 66)`. The CPU test compares those to `ColorTransform` and to the descriptor values it inserted, citing PS3.3 C.7.6.3.1.5. They are not a pasted GPU readback. The GPU palette draw uses the same constants, changes the window to a constant display of 200, and still expects the stored-key colours. A display-key lookup would have collapsed all three pixels onto the high endpoint. `address_span` bakes through `min(first, 0)` and `max(last, 0)`, and `LutDescriptor::lookup` clamps below the first mapped input and above the last entry, so the endpoint clamp matches the existing lookup for this span. A 16-bit entry outside `0..=255` is `Unavailable` rather than a new scale. That refusal is the comment on `ColourLut::palette`. It does not invent a second palette formula.

Preset mode calls `lut_chain` once and returns the texel. It does not also apply the greyscale normalisation. The GPU grey case uses a zero-width output range at 40, which the unbound path would write as black. The readback was RGB 40. `set_voi` writes only the thirty-two-byte VOI uniform. The GPU test left `lut_uploads` and `uploads` unchanged across window changes, and `select_colour_lut` incremented only `lut_uploads`. The registration placeholder is mode 0 and is not counted. `tick` still has one `queue.submit`, at `graph.rs:1780`. The test readback submits again, outside `tick`.

The unbound path is the old normalisation, including the unclamped LINEAR finding. `let cp = c - 0.5` is still the only LINEAR centre adjustment, in `shaders/voi.wgsl:57`, which this diff does not touch. The composed module has one `lut_chain`, one `voi_linear`, one `voi_linear_exact`, and one `voi_sigmoid`. The finding read back above 255 and within 0.001 of `297.50003`. Tolerance in the test is `0.001`. The CPU test checks that indexing does not change the chain bits `0x4394c001`.

Tier C preset indexing is `map_preset_into` after `LutChain::map_into`. Tier C palette is `ColorTransform::map_into`. ICC, palette on monochrome, and preset on palette colour all return `Unavailable`. No `unsafe`, no `wasm_bindgen`, no new trait, and no `as` cast in the new Rust. `git diff HEAD -- ci/wasm-size-budget.json` is empty. The recorded budget is 1,999,061 bytes. Wasm was not rebuilt for this pass. The pixel-pipeline sentence that `gate wasm` compiles `ocelli-pixel` and does not run its tests matches `ocelli-wasm`'s dependency on `ocelli-render`, that crate's dependency on `ocelli-pixel`, and `bin/ocelli.sh` running `wasm-pack` plus the size check rather than pixel tests.
