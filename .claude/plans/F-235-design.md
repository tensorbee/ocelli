# F-235, VOI shader stage 1 matches LutChain on every tier

**Status**: approved
**Epic ref**: E6.9
**Sprint**: S47
**Estimate**: 2w

Story row, `docs/sprints/BACKLOG.md:380`: "VOI shader stage 1 matches LutChain
on every tier: the shader's rescale gives the bits of ocelli_pixel::modality on
a backend that fuses multiply and add, proven by a tier A fixture from the
F-227 reviews that fails today". Layer Rust, depends on F-227, milestone M20.

`allocation.json` notes (`docs/sprints/allocation.json:3057`): "Found by F-227's
fourth and fifth reviews on Apple M4 Max under Metal. Answered by the operator
on 2026-11-06: a story in S47 with a tier A fixture that fails today."

**This is design round 4.** Round 1 was reviewed at 9 defects, 8 smells and 4
nitpicks, and re-planned the route: the table folded to two dimensions, the
non-tabled shader branch applying nothing, every interpolated route given a
correct stage 1, the volume ray's sampled-texture budget accounted, the route
resolved at registration, the acceptance rows rebuilt on hex-pinned fixtures
with a real tier B run, and the full write set named. Round 2 was reviewed at
1 defect, 7 smells and 4 nitpicks and found all round-1 findings resolved.
Round 3 was reviewed at 3 defects, 2 smells and 7 nitpicks, and found all
round-2 findings resolved and every hex fixture correct by hand. This round
applies the round-3 findings, all three of which are quietly wrong pixels and
each pinned by a fixture that fails on its named wrong implementation: the
stale slab that cannot recover (D1, the stamp gains the staged rescale, the
stale pass clears, commit_slab clears the flag, AC18), the truncated fold
height for a container whose count is not a multiple of 256 (D2, div_ceil
with a padded last row, AC15 and AC16), and the undefined effective route
that lets a flip-back draw old-table values (D3, the route defined once, the
table rebuilt on every rescale change of a declared-Tabled viewport, AC17).
The two smells are named conversion routes (S1) and the caller-view stale
recovery (S2, AC19), and the seven nitpicks are applied line by line. Every
changed fact was re-executed against the main checkout at sprint/s47
`7e8edec4` on 2026-11-11, which differs from the round-3 base `e6b913e1` by
one doc-comment line in `graph.rs` (git diff e6b913e1 sprint/s47 -- 
crates/ocelli-render/src/graph.rs, one hunk at 1853), so every line number
below holds on both. Status stays draft.

## Normative source, transcribed

### HLD 18, `docs/hld/15-lut-chain.md` lines 20 to 22

```text
This is the highest-risk arithmetic in the project. It is specified in DICOM
PS3.3 C.11 and the stages apply strictly in order. Implement it once, in
ocelli-pixel, and let the shader read the parameters - do not let a second
copy of this logic appear anywhere.
```

The em-dash of the original is transcribed here as a hyphen because this plan
is prose-checked. Nothing about the rule hinges on the dash. The source file
is exempt from the check, the plan is not.

### HLD 18.1, the modality stage, `docs/hld/15-lut-chain.md` lines 31 to 35

```rust
pub fn modality(sv: Stored, slope: f32, intercept: f32) -> Modality {
    Modality(sv.0 * slope + intercept)
}
```

Followed by, lines 37 to 39: "If a Modality LUT Sequence is present it wins
over slope and intercept. PET SUV is a separate path and needs the
radiopharmaceutical sequence - do not fold it in here."

The tree's own copy is `crates/ocelli-pixel/src/lut.rs:210-212`, and
`ModalityTransform::apply` delegates to it for the rescale selection
(`lut.rs:251-256`). `grep -rn mul_add crates/ocelli-pixel/src/` returns
nothing, so the CPU reference is two roundings, multiply then add.

### HLD 18.2, the three VOI functions, `docs/hld/15-lut-chain.md` lines 43 to 55

```text
// PS3.3 C.11.2.1.2 -- LINEAR. Requires w >= 1.
// c' = c - 0.5 ; w' = w - 1
// x <= c' - w'/2 -> ymin
// x > c' + w'/2 -> ymax
// else y = ((x - c') / w' + 0.5) * (ymax - ymin) + ymin
// PS3.3 C.11.2.1.3.2 -- LINEAR_EXACT. Requires w > 0.
// x <= c - w/2 -> ymin
// x > c + w/2 -> ymax
// else y = ((x - c) / w + 0.5) * (ymax - ymin) + ymin
// PS3.3 C.11.2.1.3.1 -- SIGMOID. Requires w > 0.
// y = (ymax - ymin) / (1 + exp(-4 * (x - c) / w)) + ymin
```

The lower comparison is `<=` and the upper is `>`, in both linear functions.
`LINEAR` uses `c - 0.5` and `w - 1`. `LINEAR_EXACT` uses neither.

### HLD 18.4, the shader side, `docs/hld/15-lut-chain.md` lines 74 to 89

```wgsl
struct VoiParams {
    center : f32,
    width : f32,
    slope : f32,
    intercept : f32,
    ymin : f32,
    ymax : f32,
    fn_kind : u32, // 0 LINEAR, 1 LINEAR_EXACT, 2 SIGMOID
    invert : u32,
};
@group(0) @binding(0) var<uniform> voi : VoiParams;
```

Section 18.4 fixes binding 0 for this uniform and is silent about any other
binding. `voi.wgsl:146-148` records the tree's own composition contract:
"`@group(0) @binding(0)` is section 18.4's and is reserved by this file.
Bindings 1 upward belong to the consumer."

### HLD 23, the uniform rule, `docs/hld/23-performance-rules.md` line 24

```text
Prefer a uniform update to a texture update. Window/level is thirty-two
bytes, not a re-upload.
```

### HLD 22, the pipeline key, `docs/hld/19-render-graph.md` line 34

```text
- **Pipelines compile at init**, keyed by (pass kind, blend mode, tier).
  Never compile a shader mid-frame.
```

### HLD 7, the tiers, `docs/hld/05-rendering.md` lines 20 to 23

```text
Tier A is WebGPU: compute shaders, storage buffers, 3D textures to 2048.
Tier B is WebGL2 through wgpu's downlevel profile: fragment shaders only, no
compute, no storage buffers, a conservative 3D-texture floor of 256. Every
feature declares the tier it needs; the tier resolves once at startup.
```

### HLD 31, the tier rule, `docs/hld/26-differentiating-capabilities.md` line 74

```text
A kernel with no fallback marks its feature unavailable; it never silently
produces a different answer.
```

`CLAUDE.md`'s hard rule, exact: "A feature that cannot run on the resolved
tier **reports unavailable**. It never quietly produces a different result,
which is §31's rule generalised."

### DICOM PS3.3 C.11.1, the rescale definition

Table C.11-1b, Rescale Intercept: "The value b in relationship between stored
values (SV) and the output units specified in Rescale Type (0028,1054).
Output units = m*SV + b." Fetched 2026-11-11 from
`dicom.nema.org/medical/dicom/current/output/chtml/part03/sect_C.11.html`.

C.11.1.1, the output range: "In the case where Rescale Slope and Rescale
Intercept are used, the output ranges from (minimum pixel value*Rescale
Slope+Rescale Intercept) to (maximum pixel value*Rescale Slope+Rescale
Intercept), where the minimum and maximum pixel values are determined by Bits
Stored and Pixel Representation."

Nothing in C.11.1 constrains the sign of m. The plan may not assume a positive
slope. PS3.3 C.7.6.3 governs which bits are the stored sample.

### The pinned wgpu, limits that bound the table

wgpu is pinned exactly, `Cargo.toml:91`: `wgpu = { version = "=30.0.1",
default-features = false }`, and `cargo tree -p ocelli-render --depth 1`
shows `wgpu v30.0.1`. Every limit below is read from the pinned registry
source, `~/.cargo/registry/src/*/wgpu-types-30.0.1/src/limits.rs` and
`wgpu-hal-30.0.1/src/`, not from memory.

- `Limits::defaults()`: `max_texture_dimension_2d: 8192` (limits.rs:424),
  `max_sampled_textures_per_shader_stage: 16` (limits.rs:432),
  `max_bindings_per_bind_group: 1000` (limits.rs:429).
- `Limits::downlevel_defaults()`: `max_texture_dimension_2d: 2048`
  (limits.rs:560), the rest inherited from `defaults()`.
- The Metal backend reports the adapter's own values
  (`wgpu-hal-30.0.1/src/metal/adapter.rs:907-925`): `max_texture_size` is
  16384 for `macOS_GPUFamily1_v1` and later, 8192 below that, 32768 on
  `MTLGPUFamily::Apple10`. The per-stage sampled-texture count is
  `max_textures_per_stage.0`, `(96, 32)` on macOS (`metal/adapter.rs:847-856`).
- The GL backend queries
  `min(MAX_VERTEX_UNIFORM_BLOCKS, MAX_FRAGMENT_UNIFORM_BLOCKS)` for
  `max_uniform_buffers_per_shader_stage`
  (`wgpu-hal-30.0.1/src/gles/adapter.rs:749-752`). What a given WebGL2 host
  reports for that query is host-specific, but the tier B contract does not
  rest on it: `Limits::downlevel_webgl2_defaults()` pins
  `max_uniform_buffers_per_shader_stage: 11`
  (`wgpu-types-30.0.1/src/limits.rs:646`), so 11 is the guaranteed floor the
  HLD's tier B names, and the ray's group 0 holds 6 uniform buffers after
  this story. Verified, not guessed.
- The session device opens with `required_limits: self.adapter.limits()`
  (`crates/ocelli-render/src/probe.rs:432`), so the adapter's own limits rule,
  never `Limits::default()`.
- `COPY_BYTES_PER_ROW_ALIGNMENT: u32 = 256`
  (`wgpu-types-30.0.1/src/lib.rs:116`). A 256-texel `R32Float` row is 1024
  bytes, already aligned.
- A zero-sized texture is refused, not clipped. `check_texture_dimension_size`
  returns `TextureDimensionError::Zero` for any dimension equal to 0
  (`wgpu-core-30.0.1/src/conv.rs:251-253`, called from texture creation at
  `wgpu-core-30.0.1/src/device/resource.rs:1389`), so a folded table whose
  height computes to 0 fails `create_texture`. Measured in the pinned
  registry source, not from memory.
- An out-of-bounds `textureLoad` is defined, and it is not a refusal. WGSL
  17.7.4: "If the logical texel address is invalid, the built-in function
  returns one of: The data for some texel within bounds of the texture. For
  non-depth textures, either: The vector (0,0,0,0) or (0,0,0,1) of the
  appropriate type" (W3C WGSL 2026-11-06, section 17.7.4). An index past the
  table's last row therefore draws either a zero or a neighbouring entry's
  value, quietly, never an error.

**The binding conclusion.** A modality table for a 16-bit container holds
65,536 entries. One row of 65,536 texels exceeds every `max_texture_dimension_2d`
above (16384 is the largest any named backend reports). Any 16-bit table must
therefore be two-dimensional and folded, and the fold must fit 2048 to be
legal on the smallest adapter the HLD's tier B names.

## Facts measured for this plan

Measured 2026-11-11 on branch `work/f-235-glm` at `e6b913e1` ("S47, open the
sprint"), `git log --oneline -1` and `git status --short` run first. Every
fact below was executed in this session. Where a fact came from round 1
unchanged, the round-1 command is restated. Facts 32 to 41 carry their own
measurement note below, taken on the main checkout at sprint/s47 `7e8edec4`,
and the load-bearing facts 1 to 31 were re-executed against that checkout in
round 4's session, which the round-4 note at the top of this plan records.

1. Stage 1 in the shader is `crates/ocelli-render/shaders/voi.wgsl:122`:
   `let m = stored * voi.slope + voi.intercept;` inside `lut_chain`
   (`voi.wgsl:120`). The uniform is declared at `voi.wgsl:45`, binding 0.
   Command: `sed -n '116,134p' crates/ocelli-render/shaders/voi.wgsl`.
2. The CPU reference is `crates/ocelli-pixel/src/lut.rs:210-212`.
   `grep -rn mul_add crates/ocelli-pixel/src/` returns nothing. Command:
   `sed -n '209,212p' crates/ocelli-pixel/src/lut.rs`.
3. wgpu is pinned exactly: `Cargo.toml:91`. Command:
   `grep -n '^wgpu' Cargo.toml`.
4. naga 30.0.1 parses `fma` only as an explicit builtin and never contracts,
   and wgpu-hal 30.0.1's Metal backend sets `setPreserveInvariance(true)` at
   `wgpu-hal-30.0.1/src/metal/device.rs:277` and never calls
   `setFastMathEnabled`. Carried from round 1's facts 5 and 6, commands
   there: `grep -rn '"fma"' ~/.cargo/registry/src/*/naga-30.0.1/src/` and
   `grep -rn "setPreserveInvariance\|setFastMath" ~/.cargo/registry/src/*/wgpu-hal-30.0.1/src/`.
5. `crates/ocelli-render/tests/voi_shader.rs`: the sweep is
   `the_shader_agrees_with_ocelli_pixel_over_a_sweep` (line 531) with
   `SWEEP_TOLERANCE: f32 = 1e-4` (line 60). `chain()` at lines 102 to 115
   builds identity rescale. `stage_one_applies_the_rescale_slope_and_
   intercept_on_the_gpu` (line 778) uses slope 2 and intercept -1024 on the
   single value 2106. The compute harness `HARNESS_WGSL` (lines 68 to 78)
   binds storage buffers at group 0 bindings 1 and 2 and calls `lut_chain`.
   The fragment harness is composed at line 860. Every test in the file is
   `#[ignore]`d and runs through `bin/ocelli.sh gate gpu`. Command:
   `sed -n '45,78p;100,115p;775,813p' crates/ocelli-render/tests/voi_shader.rs`.
6. Bricks upload in their stored integer format:
   `crates/ocelli-render/src/brick_upload.rs:78-88`, `brick_format` maps
   `U8` to `R8Uint`, `I16` to `R16Sint`, `U16` to `R16Uint`, `F32` to
   `R32Float`. Command: `sed -n '78,88p' crates/ocelli-render/src/brick_upload.rs`.
7. `crates/ocelli-pixel/src/auto_window.rs:520`:
   `const PERCENTILE_MARGIN: usize = 16;`. The tier-rationale sentences are
   at `:309-310` ("A tier whose stage 1 rounds differently by fewer units
   than the move therefore still draws the two percentiles black and
   white.") and `:470-476` ("so a tier whose stage 1 rounds differently by
   fewer units still draws grey 127 or 128"). Command:
   `sed -n '305,315p;465,480p;515,525p' crates/ocelli-pixel/src/auto_window.rs`.
8. The stack path's stored texture is float:
   `crates/ocelli-render/src/graph.rs:385`
   `pub const STORED_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::R32Float;`
   and the group 1 source at `graph.rs:645`
   `@group(1) @binding(0) var stored_tex : texture_2d<f32>;` inside
   `GRAPH_WGSL`. `upload_stored` (graph.rs:2608) takes `samples: &[f32]`.
   Command: `sed -n '378,385p;645,646p' crates/ocelli-render/src/graph.rs`.
9. **Every `lut_chain` call site, shaders included** (round 1's fact 11
   grep could not print the shader hits). Command:
   `grep -rn "lut_chain" crates/ocelli-render/`. The call sites are
   `graph.rs:786, 791, 794, 797, 810` (the `GRAPH_WGSL` stack body),
   `shaders/raycast.wgsl:259`, `shaders/slab.wgsl:62`,
   `shaders/fusion.wgsl:21`, and the definition `shaders/voi.wgsl:120`.
   The one-definition assertions are `tests/slab.rs:98` (exactly one
   `lut_chain(` in the slab entry after the prefix),
   `src/colour.rs:599` and `tests/fusion.rs:698` (exactly one
   `fn lut_chain(` each). The harness calls are `tests/voi_shader.rs:76`
   and `:853`.
10. The volume ray reads bricks as `texture_3d<u32>`
    (`graph.rs:1634-1648`, the `uint_texture` entries) and skips every
    non-unsigned kind: `shaders/raycast.wgsl:238-241`, "kind 0 is an unsigned
    integer container. Any other kind adds nothing." `bind_volume_ray`
    refuses brick formats other than `R8Uint` and `R16Uint`
    (`graph.rs:2886-2887`) and writes `kind: 0` unconditionally
    (`graph.rs:2987`). An unused brick slot is bound to a 1x1x1 `R8Uint`
    stand-in created at `graph.rs:1575-1588`.
11. Brick slot arithmetic: `brick_slot_count` is
    `max_sampled_textures_per_shader_stage - 1`, minned with the bind-group
    limit (`graph.rs:922-926`), and `volume_ray_admission` refuses a ready
    count above `sampled_limit - 1` (`graph.rs:947-960`). Command:
    `sed -n '917,960p' crates/ocelli-render/src/graph.rs`.
12. Group 0 of every composing consumer today: the stack reserves bindings
    1 and 2 for `ColourParams` and the camera (`graph.rs:639-644`), the ray
    reserves 1 to 4 for its params, opacity, shade and clip
    (`shaders/raycast.wgsl:40-70`), fusion reserves 1 for `FusionParams`
    (`shaders/fusion.wgsl:13`), the slab reserves none. Command:
    `grep -n "@group(0)" crates/ocelli-render/shaders/*.wgsl crates/ocelli-render/src/graph.rs`.
13. `set_voi` is `graph.rs:2317` and writes the 32-byte uniform through
    `VoiParams::from_chain` (`crates/ocelli-render/src/voi.rs:155`). A grep
    for `set_voi` outside `crates/ocelli-render` finds no production caller.
    Command: `grep -rn "set_voi" crates/ tools/ --include="*.rs"`.
14. `crates/ocelli-viewport/src/presentation.rs:174`: "This crate does not
    call `set_voi`." ocelli-viewport needs no change in this story.
15. The composition contract test is
    `crates/ocelli-render/src/voi.rs:213-229`,
    `the_shader_is_composable_and_reserves_only_binding_zero`, which asserts
    `!VOI_WGSL.contains("@binding(1)")`. It is rewritten by this story.
16. Composition sites of `VOI_WGSL`: `graph.rs:846` (stack and tier B
    fragment), `graph.rs:876` (ray), `src/slab.rs:95`, `src/fusion.rs:154`,
    `tests/voi_shader.rs:126` and `:860`. The prefix assertions are
    `tests/raycast.rs:95`, `tests/slab.rs:94` and `tests/fusion.rs:708`.
    Command: `grep -rn "VOI_WGSL" crates/ocelli-render/ | grep -v "^Binary"`.
17. The slab is staged by the graph itself. `show_slab` passes the closure
    `|image| self.commit_slab(ctx, adapter, id, image, blend)`
    (`graph.rs:3478`) to `slab::present_slab` (`src/slab.rs:109-125`), and
    `commit_slab` calls `stage_slab` (`graph.rs:3518-3534`) which creates the
    value texture and writes `image.samples` into it
    (`graph.rs:4255-4345`). Round 1 claimed the closure was caller-owned.
    That was false. Command: `sed -n '3455,3540p' crates/ocelli-render/src/graph.rs`.
18. The truly caller-owned source is the wasm canvas stack:
    `crates/ocelli-wasm/src/canvas.rs:277` registers with placeholder views
    (`canvas.rs:297-333`, a 1x1 `R32Float` source) and an identity
    `stack_pass()` (`canvas.rs:335-349`). The graph never sees samples for
    this viewport unless a later upload supplies them. Command:
    `sed -n '265,300p;335,349p' crates/ocelli-wasm/src/canvas.rs`.
19. `Stored` is an f32 newtype: `crates/ocelli-core/src/value.rs:16`
    `pub struct Stored(pub f32);`. `StoredBits` is
    `crates/ocelli-pixel/src/stored_pixel.rs:163`. `LutChain::stored_sample_table`
    (`crates/ocelli-pixel/src/lut.rs:665-680`) refuses to table above 16
    bits, "more than 65,536 entries". Command:
    `sed -n '660,684p' crates/ocelli-pixel/src/lut.rs`.
20. Interpolated routes feed fractional stored values into the float
    textures. `show_ortho` uploads `OrthoImage` samples through
    `upload_stored` (`graph.rs:3418-3436`), and the ortho sampler is
    trilinear: `crates/ocelli-volume/src/ortho.rs:234-285`, `sample_at`
    accumulates `weight * f64::from(value)` and narrows once. Slab samples
    come from the same walker. Fusion planes sample
    `volume.sample_world(point)` per texel (`src/fusion.rs:1120-1149`, the
    call at `:1132`). Command:
    `sed -n '234,285p' crates/ocelli-volume/src/ortho.rs` and
    `sed -n '1120,1149p' crates/ocelli-render/src/fusion.rs`.
21. Fusion windows both volumes through `lut_chain` with per-volume
    uniforms. `fusion.wgsl:19-29` (`windowed_rgb`) is called by `fs_window`
    and by `fs_fuse`. The window pass binds `pet_voi` and the fuse pass
    binds `ct_voi` (`src/fusion.rs:742-766`), where `pet_voi` is
    `VoiParams::from_chain(request.chain_b)` and `ct_voi` is
    `from_chain(request.chain_a)` (`src/fusion.rs:934-948`). `FusionStamp`
    holds `voi_a: VoiParams, voi_b: VoiParams` (`src/fusion.rs:176-188`),
    and `update` re-packs the planes when `same_samples` differs
    (`src/fusion.rs:824-871`, `same_samples` at `:901-907`). Command:
    `sed -n '176,188p;901,948p' crates/ocelli-render/src/fusion.rs`.
22. The oracle GPU candidate registers with `VoiParams::from_chain` and never
    calls `set_voi`: `tools/oracle/src/gpu_candidate.rs:84-98` builds the
    pass, `:114-124` uploads, and there is no `set_voi` call in the file.
    Command: `grep -n "set_voi\|from_chain\|register_stored" tools/oracle/src/gpu_candidate.rs`.
23. **The write set of the registration signature change is 62 call sites in
    16 files.** Command:
    `grep -rn "\.register(\|\.register_stored(" --include="*.rs" crates/ocelli-render/tests crates/ocelli-render/src/tier_b.rs tools/oracle/src/gpu_candidate.rs crates/ocelli-wasm/src/canvas.rs | awk -F: '{print $1}' | sort | uniq -c`.
    The counts: `tests/view.rs` 8, `tests/raycast.rs` 8, `tests/graph.rs` 8,
    `tests/registry.rs` 6, `tests/surface.rs` 5, `tests/slab.rs` 5,
    `tests/labelmap.rs` 5, `tests/ortho.rs` 4, `tests/upload.rs` 3,
    `tests/colour.rs` 3, `tests/contour.rs` 2, `tests/graph_voi.rs` 1,
    `tests/brick_upload.rs` 1, `src/tier_b.rs` 1,
    `tools/oracle/src/gpu_candidate.rs` 1, `crates/ocelli-wasm/src/canvas.rs`
    1. Sum 62. Round 2 counted 74 including 12 hits in
    `crates/ocelli-viewport/tests/sync.rs`, which are `scene.register` on
    `SyncScene`, a different API (`crates/ocelli-viewport/src/sync.rs:51`,
    `pub fn register(&mut self, id: u16, ...)`). A repo-wide sweep of
    `crates tools examples packages` adds only other registries:
    `ocelli-codec`'s decoder registry (`tests/jpeg2000.rs:212`),
    `ocelli-dicom`'s provider registry (`tests/metadata.rs`, 8 hits) and
    `graph.rs`'s own internal `registry.register` (`graph.rs:1835, 1891,
    4972`), none of them the public `RenderGraph::register`.
24. Tier B is driven today. F-042 (`docs/sprints/BACKLOG.md:169`) and F-X002
    (`docs/sprints/BACKLOG.md:172`) are `done`. `crates/ocelli-render/src/tier_b.rs:88`
    is `pub fn read_section_18_3`, a full graph stack path
    (`register_stored` at `:114`, `upload_stored` at `:130`, tick, readback),
    `crates/ocelli-render/tests/tier_b.rs:92-145` drives it on a native GL
    adapter through `resolve_native_gl`, and `tools/tier-b/run.mjs:1-8`
    drives the wasm export `tierBSection183` (`canvas.rs:381-383`, feature
    `tier-b-probe`) in headless Chromium WebGL2 with exit code 0 for a
    passing tier B run. Command:
    `sed -n '88,135p' crates/ocelli-render/src/tier_b.rs` and
    `sed -n '1,8p' tools/tier-b/run.mjs`.
25. `selected_stored_format` falls back to `Rg32Float` then `Rgba32Float` on
    tier B (`graph.rs:564-576`, `Upload::preserving_format` at
    `graph.rs:466-487`). Command: `sed -n '563,576p' crates/ocelli-render/src/graph.rs`.
26. `upload_stored` documents the no-allocation property for the single
    channel case: "The `R32Float` copy uses `bytemuck::cast_slice` on the
    caller slice and allocates no buffer" (`graph.rs:2590-2593`). The padded
    wide-format path already allocates (`write_stored_samples`,
    `graph.rs:4654-4670`). Command: `sed -n '2584,2596p' crates/ocelli-render/src/graph.rs`.
27. The colour mode lives in `ColourParams.mode`
    (`src/colour.rs:62-74`, mode 2 indexes a palette by the stored sample),
    and the mode-2 branches bypass `lut_chain` in both stack bodies
    (`graph.rs:770-784` linear and `:804-806` nearest, with `palette_at` at
    `:724-728` indexing `stored_tex` by the raw sample).
    `ColourLut::preset` refuses palette photometric
    (`src/colour.rs:125-127`). Command:
    `sed -n '62,82p;125,127p' crates/ocelli-render/src/colour.rs`.
28. S47's concurrent stories: F-202, F-204, F-205, F-241, F-245, F-266,
    F-267, F-268, F-269, F-272, F-273 and F-278 are in this sprint
    (`docs/sprints/CURRENT_SPRINT.md:9-23`, re-read this round). None of
    their plans names `crates/ocelli-render` as a write
    (`grep -c "crates/ocelli-render" .claude/plans/F-202-design.md` and the
    same for F-204, F-205, F-245, F-266, F-267, F-272, F-273 and F-278
    prints 0, and F-241's one hit is an AC row naming `crates/ocelli-render`
    tests as evidence, `F-241-design.md:257`, not a write). F-202's plan
    names no `crates/` path at all: `grep -c "crates/" .claude/plans/F-202-design.md`
    prints 0, and its providers are `packages/core/src/instance-bytes.ts,
    pool.ts and the ocelli-wasm session` (`F-202-design.md:126,173`).
    F-241's plan writes `crates/ocelli-wasm/src/` and `Cargo.toml`
    (`F-241-design.md:278`), which contains this story's one-line
    `canvas.rs` change. F-273's plan writes `tools/oracle/page/` and
    `tools/oracle/build-page.mjs` (`F-273-design.md:252-254`), not
    `tools/oracle/src/gpu_candidate.rs`. F-204's plan names `ocelli-render
    camera and typed Canvas, World and Index transforms`
    (`F-204-design.md:90,103`), not the graph's VOI route.
29. The graph test readback pattern exists:
    `crates/ocelli-render/tests/graph.rs:166` `fn readback(context,
    target, width) -> Vec<u8>` copies the target to a mapped buffer.
    Command: `sed -n '166,200p' crates/ocelli-render/tests/graph.rs`.
30. `use.html:764` is the user-facing sentence about the GPU route, and
    `more.html:387` names the slab reduction as a shader in `ocelli-render`.
    Command: `grep -n "from_chain" examples/folder-viewer/src/use.html`.
31. The F-227 measurements this story builds on:
    `.claude/reviews/F-227-working-pass-4.md:75` (the 2371 and 2372 pair,
    CPU draws 0 and 255, GPU draws 0 and 0) and
    `.claude/reviews/F-227-working-pass-5.md:11-12` (the pair still flips at
    HEAD, and the cancellation family, 15,879 of 800,000 draws, stored 56893
    at slope 4.8812434e-2 and intercept -2777.1045).

Facts 32 to 41 were measured for round 4, on the main checkout at sprint/s47
`7e8edec4`. Where a fact restates an earlier round's measurement, the command
is restated with it.

32. The frame's load op is keyed on the pass kind, and the slab loads.
    `crates/ocelli-render/src/graph.rs:4040-4043`:
    `load: if key.pass == PassKind::Slab { wgpu::LoadOp::Load } else {
    wgpu::LoadOp::Clear(wgpu::Color::BLACK) }`. The plan's round-3 claim that
    a stale viewport's pass "clears" is true only for non-slab passes.
    Command: `sed -n '4036,4046p' crates/ocelli-render/src/graph.rs`.
33. `commit_slab` returns early when the held stamp equals the incoming one:
    `crates/ocelli-render/src/graph.rs:3503-3505`, the `return Ok(())` arm of
    `if held == incoming`, inside `commit_slab` (`:3482`), which is the closure
    `show_slab` hands to `present_slab` (`:3478`). A repeated `show_slab(img)`
    after a rescale change is a no-op. Command:
    `sed -n '3476,3510p' crates/ocelli-render/src/graph.rs`.
34. `SlabStamp` (`crates/ocelli-render/src/graph.rs:1057-1067`) holds width,
    height, sample_count, step_bits, index_bits, samples and blend, and no
    rescale pair, so no rescale change can make `held == incoming` false.
    `slab_stamp` is built at `:4223-4252` from image and blend only. The
    blend-only shortcut is `same_pixels` (`:1068-1075`) guarded at
    `:3506-3516`: it fires on equal pixels regardless of any rescale
    difference, so adding the pair to the stamp alone is not enough, the
    blend-only arm needs the pair guard too. Command:
    `sed -n '1055,1080p;3498,3520p;4223,4252p' crates/ocelli-render/src/graph.rs`.
35. The fusion precedent the slab follows: `FusionStamp` holds
    `voi_a: VoiParams, voi_b: VoiParams`
    (`crates/ocelli-render/src/fusion.rs:176-188`) and `same_samples`
    compares the sample and geometry hashes only (`:901-907`), with the
    rescale change handled by re-packing. Command:
    `sed -n '176,190p;899,910p' crates/ocelli-render/src/fusion.rs`.
36. A caller-view viewport can never clear stale through an upload.
    `Registry::register` creates `SlotState` with `stored: None`
    (`crates/ocelli-render/src/graph.rs:1184`), and `remembered` returns
    `UploadError::NotStored` when `state.stored` is `None`
    (`crates/ocelli-render/src/graph.rs:2653-2660`), which `upload_stored`
    hits before any write. The only caller-view registration in production is
    the wasm canvas placeholder (`crates/ocelli-wasm/src/canvas.rs:277`).
    Command: `sed -n '1180,1188p;2653,2661p' crates/ocelli-render/src/graph.rs`.
37. The truncating fold height quietly misreads a non-multiple container.
    `Tabled { first: 0, count: 4095 }` computes height `4095 / 256` = 15, so
    the table holds 3840 entries and indices 3840 to 4094, all inside the
    declared range and so past `SampleOutsideTable`, address row 15, which
    does not exist: WGSL 17.7.4 returns zero or a neighbouring entry (the
    pinned-limits section above). A count below 256 computes height 0 and
    `create_texture` refuses it (the pinned-limits section above). Command:
    `python3 -c "print(4095 // 256, 4095 % 256)"` prints `15 255`.
38. `std` has no `From<i32>` for `f32`. `rustc` on
    `let _f: f32 = f32::from(5_i32);` prints `error[E0277]: the trait bound
    'f32: From<i32>' is not satisfied`, with the help line listing `From<bool>`,
    `From<i8>`, `From<u8>`, `From<i16>`, `From<u16>` among the implemented
    ones. The round-3 builder text `converting with f32::from` does not
    compile against an `i32` walk. Command: `rustc --edition 2021` on a
    one-line file, this session.
39. `div_ceil` is available: the workspace pins `rust-version = "1.97.1"`
    (`Cargo.toml:16`) and the pinned toolchain compiles
    `4095_u32.div_ceil(256)` (rustc check this session). Command:
    `grep -n rust-version Cargo.toml`.
40. The pass each of the 62 registration call sites declares, measured with
    `grep -n -A14 "\.register(\|\.register_stored(" <file>` per file. The
    split is 48 `register` (caller view) and 14 `register_stored` (owned).
    Every chain in the files that contain registration call sites is
    identity rescale: `grep -rn "ModalityTransform::new" crates/ocelli-render/
    tools/oracle/src/` returns a non-identity pair only in
    `tests/voi_shader.rs:783` (the slope 2 test that moves to the tabled
    route, Approach step 9), `tests/cpu.rs:32` and `tests/voi_params.rs`
    (no registration in either) and `tools/oracle/src/produce.rs:1312` (the
    CPU producer, not a registration), so raw bytes are already modality
    values under `PreApplied` at every site below. The gpu candidate's chain
    is `VoiParams::from_chain(presentation.chain())` with the sidecar's real
    rescale, and the table is filled from that pass's own pair (Approach
    step 3). The full per-file table is in Dependencies. The
    material rows: `tests/colour.rs:588,688,735` are the palette tests and
    declare `Tabled`, integral uploads (`&[0.0]` at `:605`, `&[9.0, 10.0,
    11.0]` at `:750`), so the new palette refusal on a declared-`PreApplied`
    viewport does not touch them. `tests/upload.rs:24` and
    `src/tier_b.rs:28-31` have negative inputs, so those five sites declare
    the signed container. `tests/ortho.rs` reslices on-grid planes of a `U8`
    volume (`:307,326,371,388,426,462`), so its four sites declare the
    unsigned 8-bit container. Command: the greps named here.
41. `wgpu::Color::BLACK` is `{ r: 0.0, g: 0.0, b: 0.0, a: 1.0 }`
    (`wgpu-types-30.0.1/src/lib.rs:354-359`). A cleared `Rgba8Unorm` target,
    which `tests/graph.rs:155` creates, reads back bytes `0, 0, 0, 255`. A
    cleared `Rgba32Float` target, which `tests/slab.rs:389` creates, stores
    the clear's channels as floats, so the readback is `0.0, 0.0, 0.0, 1.0`.
    Both stated per format in the AC rows. Command:
    `grep -n -A5 "pub const BLACK" ~/.cargo/registry/src/*/wgpu-types-30.0.1/src/lib.rs`.

## What the specification does not cover

1. How the shader obtains stage 1's bits on a backend that contracts. HLD 18
   gives the formula and HLD 18.4 gives the uniform, and both are silent on
   contraction. This is Q1, decided O1-prime by the operator on 2026-11-11
   and developed throughout.
2. The route's parameter channel. HLD 18.4 specifies one uniform at
   `@group(0) @binding(0)` and says nothing about any other binding. The
   plan reserves group 0 bindings 1 and 2 for a route uniform and the folded
   modality table, and renumbers the consumers' group 0 bindings. Every
   affected consumer is in this tree (fact 12).
3. The table's shape and domain. No HLD section sizes a GPU modality table.
   The plan sizes it to the container's whole range so an index cannot fall
   outside it for any sample the format can hold, on the precedent of
   `stored_sample_table`'s 16-bit limit (fact 19), and folds it to 256 texel
   columns so it fits the smallest `max_texture_dimension_2d` any tier B
   adapter reports (the pinned-limits section above). A table sized to a
   scanned frame minimum and maximum would return zero for an out-of-scan
   texel and look correct. No section sizes the fold either, and a container
   whose count is not a multiple of 256 is a legal DICOM container, so the
   plan pads the last row rather than refusing the width (fact 37, AC15).
4. Which route serves an interpolated input, and in which order the two
   steps run. The standard's stage 1 takes a stored sample. A trilinear
   reslice, a slab reduction and a fusion plane sample are fractional, so no
   integer table can index them and the shader cannot be given the rescale
   without contracting. The plan applies stage 1 on the CPU for every
   interpolated route (the operator's O1-prime answer), which the HLD does
   not name because it does not name the GPU routes at all. The order is
   interpolate first, then rescale: the CPU reference
   `present_cpu_plane` receives raw stored samples from the reslice walker
   and applies the chain inside the raster
   (`crates/ocelli-render/src/ortho.rs:108-135`), so the GPU routes match it
   by pre-applying stage 1 to the already-interpolated sample. Round 2's
   review records that cornerstone3D pre-scales and then interpolates, the
   opposite order. **This plan could not verify that from the tree**: the
   cornerstone source is not vendored here and no HLD section states an
   order. It is recorded as a divergence the oracle measures, under D14's
   MEASURED rule, not as a fact this plan checked.
5. The reduction space of a pre-applied slab. Today `slab.wgsl:1-5` reduces
   stored values and chains once. With stage 1 pre-applied, every blend
   reduces modality values. For a positive slope MIP, MinIP and composite
   select the same texel and produce the same bits (the affine map is
   monotone, so the maximum of the images is the image of the maximum). For
   a negative slope MIP and MinIP swap. C.11.1 constrains no sign. The
   average's rounding changes in every case and AC10 pins the new value.
   There is no tier C slab to contradict it (`src/slab.rs:118` refuses
   tier C, and `src/cpu.rs` has no slab symbol). `slab.wgsl`'s header lines
   3 to 4, which say the blends "reduce stored values", are rewritten to
   say what the texture holds.
6. Who pre-applies stage 1 for a caller-owned source view, and what happens
   when the rescale changes after an upload. The graph cannot inspect a
   caller view's bytes, so the route is a declaration the caller owns. A
   rescale change on staged pre-applied content cannot redraw in place: the
   raw samples are gone, the texture now holds modality values computed with
   the old pair. The plan marks the content stale and draws nothing until
   the next upload, which pre-applies with the now-recorded pair. Refusing
   the change instead would pin the old rescale forever, because `set_voi`
   is today the only writer of the pass's recorded chain
   (`graph.rs:2357-2361`) and the next pre-apply reads that recorded pair,
   so a refused viewport would re-apply the old rescale on every future
   upload. For a caller view there is no upload path to clear the flag
   (fact 36), so the caller clears it by re-pre-applying its own bytes and
   calling `rebind` with the new view, which rebuilds the slot and resets
   the flag. The caller-source contract is stated in Scope.
7. The palette interaction. A palette viewport indexes the stored texture by
   the raw sample (`graph.rs:724-728`) and bypasses `lut_chain` entirely, so
   pre-applied content and palette selection are mutually exclusive on one
   viewport. No HLD section covers the combination.
8. Whether stage 2's linear tail is in scope. Decided out of scope as a
   bit-equality fix, Q2's record under Open questions.
9. What happens to the margin's tier rationale once no tier rounds stage 1
   differently. Decided: the value stays, the two sentences are rewritten,
   Q3's record and AC7.
10. What "effective route" means once a declared-`Tabled` viewport has
    flipped to pre-applied content and back. The declaration is fixed at
    registration, the bytes change per upload, and a rescale change has to
    decide between rebuilding the table and marking stale from something.
    The plan defines the effective route once, in State transitions, and
    makes the table rebuild independent of it (D3, AC17).

## Problem and reproducer

On Apple M4 Max under Metal, the shader's stage 1 compiles to a fused
multiply-add. F-227's pass 4 (`.claude/reviews/F-227-working-pass-4.md:75`)
measured through the F-041 harness shape that every GPU stage-1 value equals
`f32::mul_add` bit for bit. Pass 5
(`.claude/reviews/F-227-working-pass-5.md:11-12`) confirmed the pair still
flips at HEAD and added the cancellation family.

**AC1's pair, pinned as bits.** Stored 2371.0 and 2372.0 (exact integers),
slope `0x3571C0F8` (the f32 nearest to the decimal 9.00603e-7,
9.006030268210452e-07), intercept `0x44E7C2FC` (the f32 nearest to 1854.0933,
1854.09326171875). The CPU applies two roundings:
`f32(f32(2371.0 * slope) + intercept)` is `0x44E7C30D` (1854.0953369140625)
and for 2372.0 is `0x44E7C30E` (1854.095458984375), adjacent floats. The
fused value is `0x44E7C30D` for both, computed with Python's
`math.fma(stored, slope, intercept)` narrowed to f32. The window is
LINEAR_EXACT, centre `0x44E7C30D`, output 0 to 255, and the width is the f32
nearest to the decimal 1.2207e-4, `0x38FFFFD5` (0.00012206999963382259).
**The width is not `2^-13`.** With the width exactly `0x39000000` the two
bounds `c - w/2` and `c + w/2` round to `0x44E7C30C` and `0x44E7C30E`, the
centre value sits strictly inside, and the CPU draws 127.5 and 382.5. With
the pinned width `0x38FFFFD5` both bounds round to `0x44E7C30D`, so the CPU
draws exactly 0.0 and 255.0, and the fused stage 1 draws 0.0 and 0.0, the
99th percentile black. Every one of these numbers was computed this session
with `struct.pack`/`unpack` round-trips and `math.fma`, and the working is
repeated in the AC rows.

**AC2's cancellation case, pinned as bits.** Stored 56893.0, slope
`0x3D47EF8C` (0.04881243407726288), intercept `0xC52D91AC`
(-2777.1044921875). The product `f32(56893.0 * slope)` is `0x452D915F`
(2777.085693359375), which cancels against the intercept's leading bits, so
the CPU value is `0xBC9A0000` (-0.018798828125) and the fused value is
`0xBC990748` (-0.018680229783058167), about 1.19e-4 apart with no unit
margin able to cover it (F-227 pass 5, 15,879 of 800,000 adversarial draws).

No test in the tree reaches a non-identity rescale through the shader today:
the sweep's chain is identity (fact 5) and the one non-identity test uses
slope 2 with intercept -1024, whose product is exact, so
`bin/ocelli.sh gate gpu` is green on the defect. The story exists because
tier A quietly produces a different grey from tier C, which is the tier
rule's forbidden outcome.

## Scope, API and compatibility

In scope: `voi.wgsl`'s stage 1 and its route, the route uniform and the
folded table binding, the table builder, the graph's per-viewport route
state, its effective route and its content-stale flag, registration, upload
and VOI paths, the slab and fusion staging, the slab stamp's staged rescale
pair, the frame loop's load op for a stale viewport, the
ray's bind-group arithmetic, the consumer binding renumbering, the F-041
fixture file, a new graph test file, and the two margin doc sentences in
`ocelli-pixel`.

Non-goals: stage 2's tail as a bit-equality fix (Q2). SIGMOID's `exp`, whose
precision the sweep tolerance already declares. Rendering F32 bricks in the
volume ray, which adds nothing today and still adds nothing. PET SUV, which
HLD 18.1 excludes. The Modality LUT Sequence and VOI LUT Sequence routes,
which stay unavailable through `VoiParamsError`. Tier C, which is the
reference and does not change.

New public surface, all in `ocelli-render`:

```rust
/// How a viewport's stage 1 reaches the shader.
///
/// `Tabled` is only legal for a whole integer container, because the
/// table's index is the container's own value. A legal range covers every
/// value the container can hold: `count` at least 1 and at most 65,536, and every covered
/// value inside the union of the signed and unsigned 16-bit domains,
/// -32,768 to 65,535. That expresses every integer DICOM container, 8, 12
/// or 16 bits, signed or unsigned, and keeps the builder's conversion
/// through `u16` or `i16` exact, since every accepted value fits one of
/// them. The two ranges the volume ray derives from `brick_format`, `(0, 256)`
/// for `R8Uint` and `(0, 65536)` for `R16Uint`, are the common cases, and
/// a 12-bit stack declares `(0, 4096)`. Any other range is refused at
/// registration, including `count` 0, which the shared check's decision
/// unit pins because `div_ceil(0)` would otherwise reach `create_texture`
/// as a zero extent.
pub enum StoredRoute {
    /// Integer stored data. `first` is the container's smallest value and
    /// `count` its width. The graph builds and owns the folded table. The
    /// table is `fold` texels wide and `count.div_ceil(fold)` high, so a
    /// count that is not a multiple of the fold is accepted and its last
    /// row is padded to the fold with entry values the validated index
    /// range never reaches (fact 37).
    Tabled { first: i32, count: u32 },
    /// Everything else, including F32, any width above 16 bits, and every
    /// interpolated route. Stage 1 is applied on the CPU before the bytes
    /// are written, and the uniform's rescale is written as identity.
    PreApplied,
}

/// The 16-byte route uniform, `voi.wgsl`'s `ModalityRoute`.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct ModalityRoute {
    /// 1 reads the folded table, 0 returns its input unchanged.
    pub tabled: u32,
    /// The container's smallest stored value.
    pub first: i32,
    /// Texels per table row. The fold width, 256.
    pub fold: u32,
    pub pad: u32,
}
```

Plus, on `RenderGraph`: `pub fn modality_uploads(&self) -> u64`, one per
accepted table content write driven by `set_voi`. Plus, on `VoiParams`:
`pub fn with_identity_rescale(self) -> Self`, which returns the same 32
bytes with `slope` 1.0 and `intercept` 0.0. Plus the table builder
`pub fn fill_modality_table(route: StoredRoute, slope: f32, intercept: f32,
out: &mut [f32]) -> Result<(), RouteError>`, which writes
`ocelli_pixel::modality(Stored(container_value), slope, intercept).0` at each
index, walking the container value as `i32` from `first`. The conversion to
`f32` is `f32::from(u16::try_from(value))` for a value of zero or above and
`f32::from(i16::try_from(value))` below, exact for every value in the
accepted union, because `std` has no `From<i32>` for `f32` (fact 38) and
every accepted value fits a `u16` or an `i16`. The buffer's tail beyond
`count`, the padded last row's unused texels, is filled with `0.0` and is
reachable by no validated index. There is no `as` cast and no re-derived
formula in the builder.

New errors: `GraphError::RouteInvalid` for `register`, whose result type is
`Result<ViewportId, GraphError>`, and `UploadError::RouteInvalid` for
`register_stored`, whose result type is `Result<ViewportId, UploadError>`
(the two signatures at `graph.rs:1827` and `:1873`). Both name the
same rejection, produced by one shared pure check that each registration
maps into its own error type: a `Tabled` range outside the accepted union,
a `Tabled` declaration on a `Slab` pass, whose samples the graph stages
itself pre-applied, and a `Tabled` declaration on a `SegOverlay` pass,
which composes no VOI text. `PreApplied` is legal on both. Round 3 put the variant only on `GraphError`, which does not
typecheck at `register_stored`. Plus `UploadError::SampleOutsideTable` (an
all-integral upload with a sample outside the declared container range).
Each arrives with a floor-runnable decision test and a device-path probe. A
rescale change on staged pre-applied content is not an error: it marks the
content stale, see the state transitions and AC14.

`register` and `register_stored` gain a final `route: StoredRoute`
parameter, because the route is declared where the slot is created and the
graph cannot infer a container from `&[f32]` samples. 62 call sites change
(fact 23). `upload_stored`, `set_voi`, `rebind`, `rebind_stored` and
`bind_volume_ray` keep their signatures. `show_slab` keeps its signature:
the slab's route is the graph's own decision, always pre-applied, because
its samples are interpolated (fact 20).

**The caller-source contract.** A viewport registered with a caller texture
view (today only the wasm canvas placeholder, fact 18) declares its own
route and owns the truth of it. `Tabled` promises the view holds raw
integral stored values inside the declared container. `PreApplied` promises
the view holds modality values already computed by `ocelli_pixel::modality`
with the pass's rescale, which is what makes the graph's uniform-side
identity write honest. The graph cannot inspect a caller view's bytes and
does not pretend to. The wasm canvas placeholder declares `PreApplied`
against its identity pass, which is exact for its 1x1 zero texture. A
rescale change on a caller-view `PreApplied` viewport marks the content
stale exactly as for an owned viewport, and because no `upload_stored` can
reach it (fact 36), the caller clears the stale state by re-pre-applying
its own bytes with the recorded chain and calling `rebind` with the fresh
view: `rebind` rebuilds the slot from the registry's recorded state and
resets the content flags, so the next frame draws the caller's new bytes.
The stale state is never silently permanent on either route.

Compatibility: `VoiParams` stays HLD 18.4's eight fields and 32 bytes, and a
window drag still writes only that uniform. The WGSL composition contract in
`voi.wgsl`'s header changes from "bindings 1 upward belong to the consumer"
to "bindings 1 and 2 are reserved by this file, bindings 3 upward belong to
the consumer", asserted by the rewritten `voi.rs` test. The pipeline cache
key gains nothing, because the route is a runtime uniform and not a module
shape, which is what HLD 22's "keyed by (pass kind, blend mode, tier)"
fixes.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| stack fragment, `graph.rs:786-810` | `lut_chain` in `voi.wgsl` | route uniform per viewport, group 0 bindings 1 and 2 reserved | display value per texel |
| volume ray, `raycast.wgsl:259` | `lut_chain` | route derived from the bricks' own format and validated against the declaration | display value per sample |
| slab, `slab.wgsl:62` | `lut_chain` over the staged values | the graph stages pre-applied, route uniform at flag 0 | display value per texel |
| fusion, `fusion.wgsl:21` | `lut_chain` | both volumes pre-applied at plane packing, both uniforms identity rescale | display value per texel |
| tier C raster, `cpu.rs` | `LutChain::map_into` | unchanged, the reference | display value per pixel |
| F-227 auto window | `PERCENTILE_MARGIN` | unchanged value, updated doc | the accepted window |
| `VoiParams::from_chain` | `voi.rs:155` | unchanged | the 32-byte uniform |
| oracle GPU candidate, `gpu_candidate.rs:87` | `register_stored` | declares the container's `Tabled` shape for integral decoded stacks, derived from the sidecar's `bitsStored` and `pixelRepresentation` (`produce.rs:744-753`), and `PreApplied` for anything else, so the D7 compare gate diffs the folded-table route; keeps `from_chain` in the pass | the candidate's frame |
| wasm canvas, `canvas.rs:277` | `register` | declares `PreApplied` for the placeholder | unchanged pixels |

The graph owns units and selection: the route is resolved once per viewport
at registration from the caller's declaration, the table's entries are
`ocelli_pixel::modality`'s own outputs, and the shader is given no input
from which it could re-make a stage 1 decision. The colour routes are
unchanged consumers of the same stored texture, which keeps holding raw
stored samples whenever the content is raw.

## State transitions and invariants

**The effective route.** The declaration is the `StoredRoute` fixed at
registration. The effective route is how the bytes now in the stored texture
are to be read: `Tabled` while they are raw integral samples that the folded
table serves, `PreApplied` once an upload has written modality values. It
starts equal to the declaration, flips to `PreApplied` on a pre-applied
upload to a declared-`Tabled` viewport, and flips back on an integral
upload. The shader's route uniform carries the effective route's flag, and
the VOI uniform's rescale is the pass's real pair while the effective route
is `Tabled` and identity while it is `PreApplied`. The distinction decides
every row below: the declaration decides who rebuilds the table, the
effective route decides what the shader reads. Round 3 left this term
undefined and its rows contradicted each other, which is D3.

| Entry point / starting state | Input or event | Resulting state and output | Refusal and preserved state |
|------------------------------|----------------|----------------------------|-----------------------------|
| `register` or `register_stored`, route `Tabled` on a container shape | container range | route uniform `tabled 1` written, folded table created and filled from the pass's own rescale, `GpuSlot` holds both, effective route `Tabled` | `GraphError::RouteInvalid` from `register` or `UploadError::RouteInvalid` from `register_stored` for a range outside the accepted union or a Slab or SegOverlay pass, nothing registered |
| `register` or `register_stored`, route `PreApplied` | any | route uniform `tabled 0`, 1x1 stand-in bound, VOI uniform written with identity rescale, effective route `PreApplied` | none for a legal declaration |
| `upload_stored`, declared `Tabled`, all samples integral and inside the range, effective route `Tabled` | raw samples | raw write, content raw integral, stale flag cleared, no uniform rewritten | `SampleOutsideTable` when all are integral and one is outside, nothing written |
| `upload_stored`, declared `Tabled`, all samples integral and inside the range, effective route `PreApplied` (the flip-back) | raw samples | raw write, route uniform rewritten to `tabled 1`, VOI uniform rewritten with the pass's real rescale, stale flag cleared, effective route `Tabled`. The table already holds the current pair's values, because rule D3 rebuilds it on every rescale change | `SampleOutsideTable` when all are integral and one is outside, nothing written. Round 3's row said nothing else rewritten, which contradicted AC13's own flip-back |
| `upload_stored`, declared `Tabled`, any sample fractional, colour mode not 2 | fractional samples | stage 1 applied on the CPU with the pass's rescale, modality values written, route uniform rewritten to `tabled 0`, VOI uniform rewritten to identity, content pre-applied, staged rescale recorded, stale flag cleared, effective route `PreApplied` | none, this is the ortho route |
| `upload_stored`, declared `Tabled`, fractional samples, colour mode 2 | fractional samples | raw write, content raw fractional, stale flag cleared, no flip, no refusal | the palette path indexes raw and bypasses stage 1, today's behaviour |
| `upload_stored`, declared `PreApplied` | any samples | stage 1 applied on the CPU with the pass's recorded rescale, modality values written, staged rescale recorded, stale flag cleared | none |
| `set_voi`, declared `Tabled`, rescale pair changed, effective route `Tabled` | new chain | table content rewritten in place once (rule D3), `modality_uploads` incremented, 32-byte uniform written with the real rescale | a failed table write leaves the old table and old frame |
| `set_voi`, declared `Tabled`, rescale pair changed, effective route `PreApplied` | new chain | the same table rebuild with the new pair and `modality_uploads` incremented, because the declaration is `Tabled` whatever the bytes are now, and the content marked stale, because its staged bytes were computed with the old pair. The written VOI uniform is identity while the content is pre-applied | a failed table write leaves the old table. This row is D3's fix: round 3 skipped the rebuild whenever the content was flipped, so the flip-back drew the old pair's table |
| `set_voi`, declared `Tabled`, window only | new chain | 32-byte uniform write only, no table write, counters unchanged, the rescale the effective route dictates, identity while the content is pre-applied | unchanged behaviour on drag |
| `set_voi`, declared `PreApplied`, rescale pair changed | new chain | the new chain recorded on the pass, the identity-rescale uniform written, content marked stale. The next frame draws nothing for the viewport until an upload re-pre-applies with the new pair, or until a caller view is rebound (AC19) | none. A refusal here would pin the old rescale forever, because the recorded chain is the only thing the next pre-apply reads |
| `set_voi`, declared `PreApplied`, window only, or same rescale | new chain | 32-byte uniform written with identity rescale, window change drawn | none |
| `set_voi`, content stale, window only | new chain | 32-byte uniform write only, content still stale | the viewport keeps drawing nothing until an upload or a rebind |
| `show_slab`, staged rescale pair differs from the pass's pair | same image, new pair | `SlabStamp` carries the staged pair (D1), so `held == incoming` is false and the slab re-stages with the current pair. A blend-only difference still takes the blend shortcut, which now requires the pair to match. A slab's stale state is derived, not stored: it is stale exactly when its staged pair differs from the pass's recorded pair, so a `set_voi` that returns to the staged pair clears it with no re-stage, and both early-return arms mark data dirty so that frame draws | round 3's early return made this a no-op and the slab kept the old pair's values forever |
| a stale viewport's pass, any pass kind | the next frame | the pass begins with a `LoadOp::Clear(wgpu::Color::BLACK)` load op, overriding the slab's `LoadOp::Load` at `graph.rs:4040-4043`, and encodes no draw. The readback is the clear colour: bytes `0, 0, 0, 255` on `Rgba8Unorm`, four floats `0.0, 0.0, 0.0, 1.0` on `Rgba32Float` | the flag is not wired into the not-ready `continue` at `graph.rs:4015-4017`, which would skip the pass before the clear and preserve the old frame |
| `select_colour_lut`, palette LUT, declared `PreApplied`, nothing uploaded yet | palette selection | `ColourUnavailable`, the previous colour binding preserved | the palette would index the modality values every upload stages |
| `select_colour_lut`, palette LUT, staged content pre-applied | palette selection | `ColourUnavailable`, the previous colour binding preserved | the palette would index modality values as raw |
| `select_colour_lut`, non-palette LUT, content raw fractional | non-palette selection | `ColourUnavailable`, the previous colour binding preserved | raw fractional samples have no correct stage 1 path in the shader |
| `rebind`, caller view, content stale | a fresh source view | the slot rebuilt from the recorded registry state, the stale flag reset, the route uniform and VOI uniform rewritten from the declaration and the recorded chain, and the slot marked fully dirty, because the tick skips a slot with no dirty bits (`graph.rs:3999-4001`) and the stale frame already cleared them | the caller's fresh view is by contract content staged with the recorded chain (S2). The graph validates the bytes no more than at registration |
| device loss and `rebind_stored` | rebuild | route state restored from the registry, the effective route reset to the declaration, content cleared, the stale flag reset, the table rebuilt from the pass's rescale | the caller uploads samples again, per the existing recovery contract |

The invariant this story establishes: for every input the shader can load,
the modality value entering stage 2 has exactly the f32 bits
`ocelli_pixel::modality` produces, on tiers A and B. Stage 2's own residual
divergence from the CPU stays inside the sweep's `1e-4` tolerance, measured,
and cannot reach half a grey where stage 1 agrees. A window drag still
changes only the 32-byte uniform.

## Ownership, commit and recovery

| Resource or visible state | Acquired or staged by | Lifetime identity | Observable commit point | Failure and duplicate-operation behavior |
|---------------------------|-----------------------|-------------------|-------------------------|------------------------------------------|
| folded modality table, 256 x `count.div_ceil(256)` `R32Float` or the tier B preserving format | the graph's slot, per viewport, created at registration. A tier B wide-format table is written through `write_stored_samples` (`graph.rs:4654-4670`), which pads the `.r` channel | viewport id plus container range, not the texture address | `queue.write_texture` accepted and `modality_uploads` incremented for a `set_voi` rebuild | a failed write leaves the previous table bound, a repeated equal `set_voi` writes nothing new |
| route uniform, 16 bytes | the graph's slot | viewport id | `queue.write_buffer` accepted | rewritten by a flip, a flip-back, or a route-bearing write |
| pre-applied samples | the graph's upload path, content pre-applied | viewport id plus staged rescale pair | `write_slot` accepted and `uploads` incremented | a rescale change marks the content stale and the viewport draws the clear colour until the next upload re-pre-applies with the new pair (AC14), or until a caller view is rebound (AC19) |
| caller-owned source view | the caller at `register` | the caller's view object | the caller's `rebind` with a fresh view (S2) | the graph validates nothing about the bytes, the route is a declaration, and `rebind` resets the content flags by contract |
| F-227's margin constant | `ocelli-pixel` | the constant's value, pinned by F-227's tests | the constant's value | doc sentences change, the value does not |

The table is sized once per registration and rewritten in place per rescale
change, both outside the frame loop, so there is no allocation in the render
loop. The table's extent never changes after registration because the
container range is fixed, so the rewrite is a `write_texture` into the same
texture and no bind group is rebuilt. The height is `count.div_ceil(256)`
(fact 39), fixed at registration, so the padded last row is allocated once
and its unused texels are written once. Recovery after device loss rebuilds
the table from the registry's route and pass state, the same path the
slot's other buffers take, and resets content to the declaration because
the samples are gone until the caller uploads again. `rebind` and
`rebind_stored` reset the content-stale flag for the same reason: the fresh
view or re-created texture is by contract content staged with the recorded
chain.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|---------------------|-------------------------------|------------------------|
| AC1 | Stored 2371.0 and 2372.0, slope `0x3571C0F8`, intercept `0x44E7C2FC`, route `Tabled { first: 0, count: 65536 }`. The stage-1 probe returns exactly `0x44E7C30D` and `0x44E7C30E`, `ocelli_pixel::modality`'s bits. Under LINEAR_EXACT centre `0x44E7C30D` width `0x38FFFFD5` output 0 to 255 the display values are exactly 0.0 and 255.0 | new fixture in `tests/voi_shader.rs`, tier A, `gate gpu` | a table computed with `f32::mul_add` (2372's entry becomes `0x44E7C30D` and draws 0.0), or today's uniform route (both draw 0.0) | implementation on the M4 Max host |
| AC2 | Stored 56893.0, slope `0x3D47EF8C`, intercept `0xC52D91AC`, same tabled route. The probe returns exactly `0xBC9A0000`. Under LINEAR_EXACT centre `0xBC9A0000` width `0x39D1B717` output 0 to 255 the display value is exactly 127.5, the window centre. The fused bits `0xBC990748` under the same window draw 203.1064453125, which is the red this row exists for | the same fixture file, stage-1 probe plus display | a pre-apply that itself uses `mul_add`, which moves the error rather than removing it | implementation on the M4 Max host |
| AC3 | The same inputs and window through today's text, the real rescale in the section 18.4 uniform, draw the fused greys 0.0 and 0.0. This is the semantic RED, recorded once on the base commit `e6b913e1` in the same session as AC1 and AC2 | the round-1 harness shape `on_gpu(context, params, &[2371.0, 2372.0])` run on the base tree before the route lands | a compile failure standing in for the red, which proves only that the API is missing | implementation, recorded with the run |
| AC4 | Every integer of a 16-bit container, 0 to 65535, through the folded table: the probe's bits equal `ocelli_pixel::modality`'s for each, with AC1's rescale slope `0x3571C0F8` and intercept `0x44E7C2FC`, and the display sweep under LINEAR_EXACT centre `f32(1854.12)` width `f32(0.1)` stays within `SWEEP_TOLERANCE` of the CPU chain | extended sweep in `tests/voi_shader.rs`, one dispatch, 65536 values | the fold index applied twice or not at all, which zeroes half the range, or a one-row 65536-wide table, which fails `create_texture` on every named adapter | implementation on the M4 Max host |
| AC5 | AC1 and AC2 hold through the WebGL2 path: the stack route on a native GL adapter where one opens (`tests/tier_b.rs:92` shape, `resolve_native_gl`) and in headless Chromium through `tools/tier-b/run.mjs` extended with the pair, exit 0 | the tier B vehicles that exist today, driven with the new pair | a recorded unavailable accepted while the tier row claims full, or a tier A run labelled tier B | implementation, browser plus native GL, recorded |
| AC6 | A window drag on a tabled viewport writes the 32-byte `VoiParams` and nothing else: `modality_uploads` unchanged, `uploads` unchanged. A rescale change rebuilds the table exactly once and increments `modality_uploads` by one | new graph test with the counters, `gate gpu` | rebuilding or re-uploading the table per drag | implementation |
| AC7 | `PERCENTILE_MARGIN` stays 16. The two tier-rationale sentences in `auto_window.rs` are rewritten to the margin's remaining role, and `cargo test -p ocelli-pixel` stays green | the edit plus the existing suite | deleting the margin, which turns F-227's own 15 and 17 mutations green, or leaving the stale tier sentence | implementation |
| AC8 | The pre-applied route through the graph with a non-identity rescale: `register_stored` with `PreApplied`, pass rescale `0x3571C0F8` and `0x44E7C2FC`, upload `&[2371.5, 2372.5]` as a 2x1 frame. Readback bytes 9 and 12, the encodes of the hand-computed displays 9.2138671875 and 12.32666015625 under LINEAR_EXACT centre `0x44E7C333` width `0x3C23D70A` | new graph test following `tests/graph.rs:166`'s readback | the uniform keeping the real rescale while the flag-0 branch multiplies, which applies stage 1 twice and draws 0 and 0, or no pre-apply at all, which draws 255 | implementation on the M4 Max host |
| AC9 | An all-integral upload with one sample outside the declared container range refuses `SampleOutsideTable` and writes nothing. A palette `select_colour_lut` on a declared-`PreApplied` viewport refuses `ColourUnavailable` before any upload, on staged pre-applied content refuses it after one, and a non-palette `select_colour_lut` on raw fractional content refuses it | new refusal tests: the decision units floor-runnable, the device paths under `gate gpu` | a silent zero read from an out-of-range texel index, or a palette indexing modality values as raw | implementation |
| AC10 | The slab average blend on a pre-applied viewport: a 1x1x2 `SlabImage` with samples 2371.5 and 2372.5, staged as `0x44E7C30D` and `0x44E7C30E`, the shader mean `0x44E7C30E`, display 12.32666015625, readback red channel the f32 `0.04833984375`, which is 12.32666015625 over 255, the normalization `tests/slab.rs:494` asserts, under AC8's window | a new GPU slab fixture in `tests/slab.rs`'s `show_slab` shape, `gate gpu` | staging raw samples (the mean of raw is 2372.0 and draws 255), or leaving the uniform's real rescale live with a multiplying flag-0 branch (draws 0) | implementation on the M4 Max host |
| AC11 | The volume ray composes the table in its group 0 and compiles brick slots at `sampled - 2`: `brick_slot_count` 14 where the sampled limit is 16 and 94 on the M4 Max, whose sampled count is 96 and whose bind-group limit is `u32::MAX`, and `volume_ray_admission`'s room matches. A declared route that disagrees with the bricks' format refuses `RayUnavailable` at bind | unit tests on the arithmetic plus a device bind test, `gate gpu` | composing the table without reducing the slots, which puts the ray fragment stage at `limit + 1` sampled textures and fails pipeline creation on every adapter, or silently substituting the derived route for the declared one | implementation |
| AC12 | The harness probe with route flag 0 and the AC2 rescale in the section 18.4 uniform returns its input bits unchanged: input `0x44E7C30D` (1854.0953369140625) with slope `0x3D47EF8C` and intercept `0xC52D91AC` in the uniform returns exactly `0x44E7C30D`. A surviving stage 1 would return `0xC527E9A0`, -2686.6015625, about -2686.6 | the probe entry point with `tabled 0`, `tests/voi_shader.rs` | keeping `stored * voi.slope + voi.intercept` in the non-tabled branch, which returns fused bits while the texture already holds modality values | implementation on the M4 Max host |
| AC13 | The ortho flip, pinned. A `Tabled { first: 0, count: 65536 }` viewport with pass rescale slope 2.0 intercept -1024.0 uploads the fractional samples 2371.25 and 2371.5 as a 2x1 frame. The graph pre-applies, staged 3718.5 and 3719.0, and under LINEAR_EXACT centre 3719.0 width 2.0 output 0 to 255 the readback bytes are 64 and 128, the encodes of 63.75 and 127.5, equal to the CPU chain's. An `i32`-truncating pre-apply reads entry 2371 for both, 3718.0, and draws byte 0 for both. A following integral upload of 2371.0 and 2372.0 flips back and the table serves it: bytes 0 and 255 | a graph test through `upload_stored` twice with a readback after each, `gate gpu` | refusing fractional samples, which makes every off-grid MPR plane unavailable, or truncating them through `i32`, which silently reads the neighbouring entry and draws 0 where 64 and 128 belong | implementation on the M4 Max host |
| AC14 | The rescale change on pre-applied content, pinned. A `PreApplied` viewport with pass rescale slope `0x3571C0F8` intercept `0x44E7C2FC` and initial chain window LINEAR_EXACT centre `0x44E7C30D` width `2.0` output 0 to 255 (pinned so the fixture is deterministic, though no readback is taken before `set_voi`) uploads 2371.5 and 2372.5, staged as `0x44E7C30D` and `0x44E7C30E`. `set_voi` with the chain whose rescale is slope `0x3D47EF8C` intercept `0xC52D91AC` and window LINEAR_EXACT centre `0xC5265588` width 2.0 output 0 to 255 records the new chain and marks the content stale: the next frame encodes a clear-only pass for that viewport and its readback is the clear colour, bytes `0, 0, 0, 255` on the `Rgba8Unorm` target `tests/graph.rs:155` creates, because `wgpu::Color::BLACK` is `{ 0.0, 0.0, 0.0, 1.0 }` (fact 41). A re-upload of the same samples re-pre-applies with the new pair, staged `0xC5265588` and `0xC52654C0`, and the readback bytes are 128 and 134, the encodes of 127.5 and 133.7255859375 | a graph test through `set_voi`, a frame readback, and a second `upload_stored` with a readback, `gate gpu` | the round-2 refusal design, which leaves the recorded rescale old, so the re-upload re-applies it and draws the old modality values `0x44E7C30D` and `0x44E7C30E` under the old window, bytes 0 and 255, on every upload forever | implementation on the M4 Max host |
| AC15 | The non-multiple container, pinned. `Tabled { first: 0, count: 4095 }` registers a 256 x 16 table, 4096 texels with the last row's final texel padding. The stage-1 probe of stored 3840 to 4094, all inside the declared range and so past `SampleOutsideTable`, returns exactly `ocelli_pixel::modality`'s bits for each with AC1's rescale pair. The truncated height of round 3, 4095 / 256 = 15, addresses row 15 for exactly these values, and WGSL 17.7.4 returns zero or a neighbouring texel for it, a quietly wrong grey | a stage-1 probe fixture over the last 255 container values in `tests/voi_shader.rs`, tier A, `gate gpu` | the truncating height `count / fold`, which leaves indices 3840 to 4094 addressable only out of bounds while validation passes them | implementation on the M4 Max host |
| AC16 | The below-one-byte container, pinned. `Tabled { first: 0, count: 16 }` (a 4-bit `bitsStored` container) registers a 256 x 1 table whose first 16 entries are real and 240 padding, and the probe of stored 0 to 15 returns `ocelli_pixel::modality`'s bits with AC1's rescale pair. The truncated height of round 3 computes 0 and `create_texture` refuses the zero extent, making every small container unavailable | the same probe fixture, `gate gpu` | the truncating height, which fails `create_texture` for any count below the fold, the opposite refusal from AC15's quiet grey and equally wrong | implementation on the M4 Max host |
| AC17 | The flip, the rescale change, and the flip-back, pinned (D3). A `Tabled { first: 0, count: 65536 }` viewport with pass rescale slope 2.0 intercept -1024.0 uploads the fractional samples 2371.25 and 2371.5: pre-applied, staged 3718.5 and 3719.0, route uniform `tabled 0`. `set_voi` with the AC14 pair `0x3D47EF8C` and `0xC52D91AC` rebuilds the table with the new pair (`modality_uploads` +1) and marks the stale content: the next frame is the clear colour. The rebuilt entry is pinned floor-side: the builder unit fills the pair-B table and index 2371 holds `0xC52655EC`, not pair A's 3718.0. An integral upload of 2371.0 and 2372.0 flips back: raw write, route uniform `tabled 1`, VOI uniform with the real pair, stale cleared. Under LINEAR_EXACT centre `0xC52655EC` width 2.0 output 0 to 255 the readback bytes are 128 and 134, the encodes of 127.5 and 133.7255859375 | the builder unit in `src/voi.rs` under `#[cfg(test)]`, plus a graph test through `upload_stored`, `set_voi` with the counters, and a second `upload_stored` with a readback after each frame, `gate gpu` | skipping the rebuild because the effective route is `PreApplied`, which leaves pair A's 3718.0 and 3720.0 in the table: the flip-back then draws 255 and 255 under pair B's window, or with the route uniform left at 0 draws 255 and 255 through the skipped stage 1. Both named wrong implementations are red against 128 and 134 | implementation on the M4 Max host, the builder unit in the floor |
| AC18 | The stale slab recovers, pinned (D1). A slab viewport staged by `show_slab` with pass rescale `0x3571C0F8` and `0x44E7C2FC`, a 1x1x2 `SlabImage` of 2371.5 and 2372.5, average blend: staged `0x44E7C30D` and `0x44E7C30E`, readback red channel the f32 `0.04833984375` under AC8's window, as AC10. `set_voi` with the AC14 pair marks the slab stale: the next frame's slab pass begins with a clear and no draw, and the `Rgba32Float` target `tests/slab.rs:389` creates reads `0.0, 0.0, 0.0, 1.0`. A second `show_slab` of the same image re-stages, because the stamp now carries the staged pair (fact 34), and clears the flag: the readback under LINEAR_EXACT centre `0xC5265588` width 2.0 is the mean of the pair-B staged values `0xC5265588` and `0xC52654C0`, `(0 + a1 + a2) / 2` = `0xC5265524` (-2661.3212890625), displayed 130.61279296875, the red channel f32 `0.51220703125` (`0x3F032000`). A second fixture runs pair A, then `set_voi` B, then `set_voi` A, then `show_slab` of the same image, and reads the pair-A mean `0x44E7C30E`, red `0.04833984375`, so a stale flag that survives a return to the staged pair fails | a new GPU slab fixture in `tests/slab.rs`'s `show_slab` shape through `set_voi` and a second `show_slab`, `gate gpu` | the round-3 compound failure: the slab pass loads (`graph.rs:4040-4043`) so the stale frame survives, and `commit_slab`'s equal-stamp early return (`graph.rs:3503-3505`) makes the re-show a no-op, so pair A's modality values draw under pair B's window on every frame forever | implementation on the M4 Max host |
| AC19 | The caller-view rescale change recovers, pinned (S2). A caller-view `PreApplied` viewport registered as `canvas.rs:277` is, 2x1 with pair A staged in the caller's own view: bytes 9 and 12 as AC8. `set_voi` with the AC14 pair succeeds, records the chain, and marks the content stale: the next frame's readback is the clear colour, bytes `0, 0, 0, 255`. `upload_stored` returns `NotStored` (fact 36), so the recovery is the caller's: stage the AC14 samples 2371.5 and 2372.5 pre-applied with the recorded pair `0xC5265588` and `0xC52654C0` into a fresh view and `rebind`. The next frame draws them under LINEAR_EXACT centre `0xC5265588` width 2.0 through the identity-rescale uniform, bytes 128 and 134, the encodes of 127.5 and 133.7255859375 | a graph test following `tests/graph.rs:166`'s readback through `set_voi`, a frame, `rebind`, and a frame, `gate gpu` | leaving the stale flag permanent on a caller view, which draws the clear colour forever, or refusing the `set_voi`, which keeps pair A's modality values under pair B's window | implementation on the M4 Max host |

## Approach

The smallest implementation in terms of the transcribed source keeps stage 1
existing exactly once in `ocelli-pixel`, exactly as HLD 18 asks, and gives
the shader the results rather than a second copy of the arithmetic.

1. `voi.wgsl` splits stage 1 into its own function and stages 2 and 3 into
   another, with `lut_chain` composing them. The route uniform and the
   folded table are reserved at group 0 bindings 1 and 2:

   ```wgsl
   struct ModalityRoute {
       tabled : u32, // 1 reads the folded table, 0 returns its input
       first : i32,  // the container's smallest stored value
       fold : u32,   // texels per table row
       pad : u32,
   };
   @group(0) @binding(1) var<uniform> modality_route : ModalityRoute;
   @group(0) @binding(2) var modality_table : texture_2d<f32>;

   fn modality_stage_one(stored : f32) -> f32 {
       if (modality_route.tabled != 0u) {
           let index = i32(stored) - modality_route.first;
           let x = index % i32(modality_route.fold);
           let y = index / i32(modality_route.fold);
           return textureLoad(modality_table, vec2<i32>(x, y), 0).r;
       }
       return stored;
   }

   fn voi_stages(modality_value : f32) -> f32 { /* today's stages 2 and 3 */ }
   fn lut_chain(stored : f32) -> f32 { return voi_stages(modality_stage_one(stored)); }
   ```

   The fold is the fix the pinned limits force: a 16-bit container is
   65,536 entries, a single row that wide exceeds every named
   `max_texture_dimension_2d`, so the table is `fold` texels wide and
   `count.div_ceil(fold)` high (fact 39), `256 x 256` for 16 bits,
   `256 x 1` for 8 bits and `256 x 1` for a 4-bit container, legal down to
   the 2048 downlevel floor. A count that is not a multiple of the fold
   pads its last row: `Tabled { first: 0, count: 4095 }` is 256 x 16 with
   one unused texel, which no validated index reaches (fact 37, AC15).
   Truncating instead, `count / fold`, leaves the last 255 values of that
   container indexing row 15 of a 15-row table, which WGSL 17.7.4 resolves
   to zero or a neighbouring texel, quietly. The index split is `x = index %
   fold`, `y = index / fold`, written identically in the shader and in the
   builder. The Rust validation is what keeps the index inside the table:
   `SampleOutsideTable` refuses an all-integral upload with a sample outside
   the container, and the pre-applied routes never index at all. The
   header's contract bullets gain the route: the table's entries are
   `ocelli-pixel`'s own outputs, the route is resolved on the CPU, and the
   shader still makes no LUT decision. `voi.rs:213`'s test is rewritten to
   assert the two reserved bindings and still no entry point and no
   storage.

2. `voi.rs` gains the `ModalityRoute` mirror, 16 bytes, `Pod`, and
   `VoiParams::with_identity_rescale`, and the table builder
   `fill_modality_table`, which fills a caller buffer with
   `ocelli_pixel::modality(Stored(container_value), slope, intercept).0`
   for each index, walking the container value as `i32` from `first` with
   checked range arithmetic. The conversion to `f32` goes through the
   16-bit types, `f32::from(u16::try_from(value))` at zero and above and
   `f32::from(i16::try_from(value))` below, exact for every value in the
   accepted union: `std` has no `From<i32>` for `f32` (fact 38) and a
   direct `as` is a denied truncation cast. The builder writes `0.0` to the
   padded tail beyond `count`, the unused texels of the last row when
   `count` is not a multiple of the fold. The builder
   calls `ocelli_pixel::modality`, it does not re-derive the formula.

3. `graph.rs` gains per-viewport route state: the declared `StoredRoute`,
   the effective route (State transitions), the content kind (raw integral,
   raw fractional, pre-applied), the staged
   rescale pair and a `content_stale` flag in `SlotState`, and the route
   uniform buffer, table texture and view on `GpuSlot`, created by
   `create_gpu_slot` from the state, the pattern `ray_dummy` already sets at
   `graph.rs:1575`. `rebind` and `rebind_stored` reset the effective route
   and the stale flag in the registry state at the same call, because the
   fresh view
   or re-created texture is by contract content staged with the recorded
   chain. At registration the table is filled from the pass's own
   rescale, `voi_of(&pass)`, which is what makes the oracle candidate's
   register-then-upload-never-set_voi sequence correct (fact 22). On the
   pre-applied route the VOI uniform is written through
   `with_identity_rescale`, and `voi_params` returns the same adjusted
   bytes, so its doc sentence "The VOI uniform a stack viewport will draw
   with" stays true. The group 0 layouts gain the two entries and the
   consumers renumber: stack colour 1 to 3 and camera 2 to 4 (`GRAPH_WGSL`),
   ray 1 to 4 becoming 3 to 6 (`raycast.wgsl`), fusion 1 to 3
   (`fusion.wgsl`). The slab's group 0 layout takes the new entries and
   `labelmap.wgsl:2` composes no VOI text, so it is untouched. The volume
   ray derives its route from the bricks' own format, `Tabled { first: 0,
   count: 256 }` for `R8Uint` and `{ first: 0, count: 65536 }` for
   `R16Uint`, and `bind_volume_ray` refuses a declaration that disagrees,
   which is the one precedence: the declaration is the source of truth for
   the table, the brick format must agree with it, and neither silently
   overrides the other. The pipeline cache key is untouched, because the
   route is a uniform and not a module shape, which is what HLD 22's "keyed
   by (pass kind, blend mode, tier)" fixes. The oracle GPU candidate
   declares the container's route: for an integral decoded stack with the
   standard bit layout (`highBit` equal to `bitsStored - 1`) the `Tabled`
   shape derived from the sidecar's `bitsStored` and
   `pixelRepresentation` (`produce.rs:744-753`, unsigned first 0 count
   `1 << bits`, signed first `-(1 << (bits - 1))` count `1 << bits`), and
   `PreApplied` for anything it cannot express, so the D7 compare gate
   diffs the folded-table route and not only the pre-applied one. The shift
   is validated before it runs: `bitsStored` 0 has no `highBit` and
   `bitsStored` above 16 exceeds the accepted union, so both fall back to
   `PreApplied` rather than shift (fact 37's AC16 and AC15 cover the
   legal small and non-multiple counts). The corpus exercises this at 8,
   12 and 16 bits (`REFERENCE_MONO12` and the signed 12-bit-in-16 rows are
   12-bit containers), which is why the accepted range union is general
   rather than the three brick shapes.

4. The ray's sampled-texture budget. The ray's fragment stage would sample
   `brick_slots` bricks, the residency texture and the modality table, so
   `brick_slot_count` becomes `sampled - 2` minned with the bind-group
   room, and `volume_ray_admission`'s room becomes `sampled_limit - 2`,
   both documented: the ray composes the modality table, so the room for
   bricks is the per-stage sampled limit minus the residency texture and
   minus the table. On a 16-limit adapter the compiled slots go 15 to 14,
   on the M4 Max they go 95 to 94, derived from the Metal backend's
   per-stage sampled count of 96 (`metal/adapter.rs:847-856`) and its
   `max_bindings_per_bind_group` of `u32::MAX`, reported as "No limit"
   (`metal/adapter.rs:1411`), so the bind-group room never binds first.
   The table itself is bound in the
   ray's group 0, which holds six uniform buffers and one texture, one
   uniform buffer more than today. The tier B floor pins
   `max_uniform_buffers_per_shader_stage` at 11
   (`wgpu-types-30.0.1/src/limits.rs:646`), so six fits the guaranteed
   downlevel floor with five to spare. The GL backend reports the host's
   own query result (`wgpu-hal-30.0.1/src/gles/adapter.rs:749-752`), which
   this plan does not need to guess: the floor is the contract HLD 7 names
   and it is verified against the pinned source.

5. `upload_stored` decides the content. Declared `Tabled` with every sample
   integral and inside the range writes raw. When the effective route is
   already `Tabled` that is the whole operation. When the effective route is
   `PreApplied` (the flip-back, D3) the raw write is followed by rewriting
   the route uniform to `tabled 1` and the VOI uniform with the pass's real
   rescale, because the table was rebuilt with the current pair by rule D3
   and is now the serving route again. Declared `Tabled` with any
   fractional sample applies stage 1 with `ocelli_pixel::modality` using
   the pass's recorded rescale, writes the modality values, rewrites the
   route uniform to `tabled 0` and the VOI uniform to identity, and records
   the staged rescale, which is what keeps every off-grid MPR plane available
   and correct. An all-integral upload with a sample outside the range
   refuses `SampleOutsideTable` and writes nothing. Declared `PreApplied`
   always pre-applies. When the viewport's colour mode is 2 the raw write
   is kept and no flip happens, because the palette indexes the raw sample
   (fact 27) and its branches never reach stage 1. The pre-apply needs a
   scratch `Vec<f32>` of `samples.len()`, 4 bytes per sample, one
   allocation per pre-applied upload freed at the end of the call, the
   same class as the padded wide-format path that already allocates at
   `graph.rs:4654-4670`, and the doc sentence "allocates no buffer"
   (`graph.rs:2590-2593`) is rewritten to name the two cases. A 512x512
   ortho plane stages 262,144 samples through a 1,048,576-byte scratch.
   `select_colour_lut` refuses a palette LUT on staged pre-applied content
   and a non-palette LUT on raw fractional content, both through
   `ColourUnavailable` with the previous binding preserved. It also refuses
   a palette LUT on a declared-`PreApplied` viewport before any upload,
   because every upload that viewport accepts will stage modality values
   and the guard must not wait for one.

6. `set_voi` on a declared-`Tabled` viewport rebuilds the table in place
   when the rescale pair changed, one `write_texture`, `modality_uploads`
   plus one, whatever the effective route is (rule D3). When the content is
   currently pre-applied, the rebuild is accompanied by marking it stale,
   because its staged bytes were computed with the old pair, and the
   written VOI uniform is identity. On a declared-`PreApplied` viewport it
   accepts a rescale change: the new chain is recorded on the pass, the
   identity-rescale uniform is written, and the content is marked stale.
   The raw samples are gone, the texture holds modality values computed
   with the old pair, and the recorded chain is the only thing the next
   pre-apply reads, so refusing the change would pin the old rescale
   forever: `set_voi` is today the only writer of the pass's chain
   (`graph.rs:2357-2361`), and a refused viewport would re-apply the old
   rescale on every future upload. The stale flag changes what the frame
   encodes: a stale viewport's render pass still begins, the load op is
   `LoadOp::Clear(wgpu::Color::BLACK)` for every pass kind, overriding the
   slab's `LoadOp::Load` at `graph.rs:4040-4043` (fact 32), and no draw
   call is encoded, so the viewport reads back as the clear colour. The
   round-3 text claimed the existing clear at `graph.rs:4043` already did
   this, which is false for the slab, whose pass loads. Wiring the flag
   into the `continue` at `graph.rs:4015-4017` would instead
   skip the pass before the clear and preserve the old frame, which is the
   quietly-stale display this design forbids. A window-only change on
   stale content writes the uniform and stays stale. The frame loop reads
   the flag from the registry state it already re-fetches for the dirty
   clear at `graph.rs:4075-4077`.

7. `stage_slab` pre-applies and the slab stamp carries the staged pair.
   The slab's samples are interpolated (fact 20),
   so `stage_slab` writes `ocelli_pixel::modality` values into the value
   texture with the pass's rescale and the slab's route uniform is written
   at 0. `slab_stamp` gains the staged rescale pair, mirroring
   `FusionStamp`'s `voi_a` and `voi_b` (fact 35): `commit_slab` computes
   the incoming stamp from the pass's current pair, so a rescale change
   makes `held == incoming` false (`graph.rs:3503-3505`) and the same
   image re-stages instead of returning early, which is what lets a
   `show_slab` after `set_voi` recover (AC18). The blend-only shortcut at
   `graph.rs:3506-3516` gains the pair guard: it fires only when the
   pixels and the pair both match, because with the pair alone in the
   stamp the equal-pixels check would still skip the re-stage. A slab's
   stale state is derived as staged pair differs from recorded pair,
   never stored, so it cannot outlive a return to the staged pair, and
   both early-return arms mark data dirty. The pre-apply skips absent
   samples and leaves absent columns at
   the zero fill, which keeps `graph.rs:4305-4307`'s comment ("An absent
   column stays the zero fill") true. `slab.wgsl`'s `reduced` body is
   unchanged: composite, MIP and MinIP reduce whatever the texture holds,
   and on pre-applied content
   that is modality values, which for a positive slope selects the same
   texel with the same bits and for a negative slope swaps MIP and MinIP
   (the specification-does-not-cover list, item 5). `slab.wgsl`'s header
   lines 3 to 4 are rewritten to say the blends reduce what the texture
   holds, modality values on a pre-applied slab. The average of
   modality values is the new AC10 behaviour. The module keeps exactly one
   `lut_chain(` occurrence after the prefix, which `tests/slab.rs:98`
   asserts.

8. Fusion pre-applies both volumes at plane packing. `prepare_plane_samples`
   maps `pet_samples` through `chain_b`'s rescale and `ct_samples` through
   `chain_a`'s, in place on the `Vec`s it already allocates, no new
   buffer. The pre-apply skips presence-0 texels and leaves them at 0.0,
   which keeps `fusion.wgsl:3`'s header line ("The stored plane holds 0
   when presence is 0") true: `windowed_rgb` is a shared function and the
   presence guards live in its callers, so a texel that reads as the
   intercept would falsify the file's own stated invariant. `fill_plane`
   writes exactly 0.0 with flag 0.0 for absent texels
   (`fusion.rs:1132-1135`), so the mask is the skip list. `same_samples`
   gains the two rescale pairs so a rescale change re-packs the planes,
   and the written `voi_a` and `voi_b` uniform bytes
   go through `with_identity_rescale` while `FusionStamp` keeps the real
   `VoiParams` for comparison. The fusion's group 0 gains the route
   uniform at 1, the stand-in table at 2, and `FusionParams` at 3.
   PET is the small non-integral slope case the finding names, and it is
   covered by the same pre-apply as the CT volume.

9. `tests/voi_shader.rs` moves its storage bindings to 3 and 4, binds the
   route uniform and the table, and gains the stage-1 probe entry point,
   the AC1, AC2, AC4, AC12, AC15 and AC16 fixtures and the container sweep.
   AC15 and AC16's height arithmetic additionally lands as a floor-runnable
   unit beside the device probe (Tests). One
   existing test cannot stay where it is:
   `stage_one_applies_the_rescale_slope_and_intercept_on_the_gpu`
   (`tests/voi_shader.rs:778`) drives slope 2 and intercept -1024 through
   the uniform on stored 2106 and expects 127.5. Under flag 0 the uniform's
   rescale is unread, the shader returns 2106 unchanged, and the window
   draws 0, so the test would go red. It moves to the tabled route: the
   harness fills the folded table from the test's own chain, entry 2106 is
   `modality(2106) = 3188.0` exactly, and the existing assertion, 127.5
   under LINEAR_EXACT centre 3188 width 400, is unchanged. The remaining
   existing tests use the identity rescale and run under flag 0 unchanged,
   because flag 0 returns its input and their inputs are already modality
   values. AC17 and AC19 land in the new `tests/modality_route.rs`, AC18 in
   `tests/slab.rs`, following the shapes their rows name.

Rejected alternative, recorded because its costs were checked against the
tree: fill the stage-1 body per route at composition, the way
`__RAY_SLOTS__` is substituted. That makes the module text route-dependent,
which forces a fourth field into the HLD 22 pipeline key and doubles the
compiled pipeline set, and a route change mid-session needs a shader
compile. The runtime route uniform needs none of that and matches the
existing stand-in-texture pattern.

## Dependencies, write ownership and resources

Declared: F-227, closed on `sprint/s46`, which owns
`crates/ocelli-pixel/src/auto_window.rs` and the margin constant. This story
edits two doc sentences in that file and no constant.

Files this story writes: `shaders/voi.wgsl`, `src/voi.rs`,
`src/graph.rs`, `src/slab.rs`, `shaders/slab.wgsl`, `src/fusion.rs`,
`shaders/fusion.wgsl` through its module string, `src/colour.rs` only where
it asserts composed text, `src/tier_b.rs` (the pair probe beside
`read_section_18_3`), `tests/voi_shader.rs`, `tests/tier_b.rs`, a new
`tests/modality_route.rs`, the view, raycast, graph, registry, surface,
slab, labelmap, ortho, upload, colour, contour, graph_voi and brick_upload
test files for the 62 signature call sites (fact 23), and
`crates/ocelli-pixel/src/auto_window.rs` doc sentences. Outside
`ocelli-render`: `tools/oracle/src/gpu_candidate.rs` gains the route
parameter and the container derivation, `tools/oracle/src/produce.rs`'s
`GpuStack::draw` call (about line 592) gains the route argument, derived
where the sidecar is read (`produce.rs:744-753`), with `GpuStack::open` at
line 290 unchanged, and `crates/ocelli-wasm/src/canvas.rs` one line.

**The route each of the 62 call sites declares (fact 40).** Every chain
behind a site in this table is identity rescale, so raw bytes are already
modality values and `PreApplied` changes no expectation, except where a
fixture wants the table. The implementer copies this table rather than
re-deriving it.

| File | Sites | Pass and content | Declared route |
|------|-------|------------------|----------------|
| `tests/graph.rs` | 238, 280, 312, 325, 338, 399, 527 | stack, caller view, identity rescale chain (`:72-80`) | `PreApplied` |
| `tests/graph.rs` | 540 | `SegOverlay`, no VOI text | `PreApplied` |
| `tests/view.rs` | 222, 351, 394, 474, 493, 517, 537 | stack, caller view, identity | `PreApplied` |
| `tests/view.rs` | 233 | volume ray that never binds bricks | `PreApplied` |
| `tests/raycast.rs` | 496, 581, 760, 842, 1033, 1142, 1274, 1413 | volume ray binding the `u8` brick pair of `place_pair` (`:469-474`) | `Tabled { first: 0, count: 256 }` |
| `tests/labelmap.rs` | 261, 339, 373, 465 | `SegOverlay` | `PreApplied` |
| `tests/labelmap.rs` | 399 | stack, caller view, identity | `PreApplied` |
| `tests/slab.rs` | 397, 461 | stack, caller view, identity | `PreApplied` |
| `tests/slab.rs` | 411, 478, 544 | `Slab`, graph-staged | `PreApplied`, a `Tabled` declaration is refused |
| `tests/registry.rs` | 238, 401, 545 | `SegOverlay` | `PreApplied` |
| `tests/registry.rs` | 289 | stack, caller view, identity | `PreApplied` |
| `tests/registry.rs` | 313 | volume ray that never binds | `PreApplied` |
| `tests/registry.rs` | 331 | `Slab`, graph-staged | `PreApplied`, a `Tabled` declaration is refused |
| `tests/contour.rs` | 195, 213 | `SegOverlay` | `PreApplied` |
| `tests/surface.rs` | 183, 190 | stack, caller view, identity | `PreApplied` |
| `tests/surface.rs` | 454, 479 | `SegOverlay` | `PreApplied` |
| `tests/surface.rs` | 251 | `register_stored`, uploads `&[0.0]` | `Tabled { first: 0, count: 65536 }` |
| `tests/graph_voi.rs` | 117 | stack, caller view, identity | `PreApplied` |
| `tests/colour.rs` | 588, 688, 735 | `register_stored` stacks, integral uploads (`&[0.0]` at `:605`, `&[9.0, 10.0, 11.0]` at `:750`), palette selects | `Tabled { first: 0, count: 65536 }`, which keeps the palette refusal row from turning them red |
| `tests/upload.rs` | 267, 364, 423 | `register_stored`, `INPUTS = [-160.0, 40.0, 240.0, -60.0]` (`:24`), integrals with negatives | `Tabled { first: -32768, count: 65536 }` |
| `tests/ortho.rs` | 292, 357, 412, 448 | `register_stored`, on-grid reslices of the `U8` fixture (`:307,326,371,388,426,462`), integral or absent samples | `Tabled { first: 0, count: 256 }` |
| `tests/brick_upload.rs` | 463 | `register_stored` dummy stack for `upload_brick` refusals, no stored upload | `PreApplied` |
| `src/tier_b.rs` | 114 | `register_stored`, `INPUTS = [-160.0, 40.0, 240.0, -60.0]` (`:28-31`) | `Tabled { first: -32768, count: 65536 }` |
| `crates/ocelli-wasm/src/canvas.rs` | 277 | stack placeholder, 1x1 zero texture, identity pass | `PreApplied` |
| `tools/oracle/src/gpu_candidate.rs` | the one `register_stored` | sidecar-derived container | `Tabled` for a standard integral layout, `PreApplied` otherwise (Approach step 3) |

The split is 48 `register` and 14 `register_stored`. The `Tabled` rows make
the signed-container union load-bearing: `tests/upload.rs` and
`src/tier_b.rs` feed -160.0 and -60.0, which no unsigned container covers.

**Measured overlaps with the sprint's other writers (fact 28).** F-241 is
approved and writes `crates/ocelli-wasm/src/` and `Cargo.toml`
(`F-241-design.md:278`), which contains this story's `canvas.rs:277` line.
That is the one real same-file overlap, and this story integrates its one
line before or after F-241's bridge work with a rebase. F-202's plan names
no `crates/` path at all (zero matches for `crates/` in
`F-202-design.md`), so the round-1 claim that no file recorded its write
set is corrected: the plan exists and does not name `graph.rs`. F-273's
plan writes `tools/oracle/page/`, not `src/gpu_candidate.rs`. F-204 names
the camera and transforms, not the VOI route. `graph.rs` is this story's
largest file and no measured S47 plan names it, which is a measured
statement about the plans, not a guarantee about F-202's implementation
diff, and the lead still sequences `graph.rs` against F-202 and F-204 as a
risk.

Exclusive resource: the operator's M4 Max for `bin/ocelli.sh gate gpu`, one
GPU story at a time, as the sprint's current note requires. No
wasm-bindgen, no boundary change, no new unsafe.

## Verification and evidence reuse

```bash
bin/ocelli.sh check ocelli-render
bin/ocelli.sh test  ocelli-render
bin/ocelli.sh clippy ocelli-render
bin/ocelli.sh gate gpu
bin/ocelli.sh gate wasm
cargo test -p ocelli-pixel
bin/ocelli.sh gate prose
```

AC1, AC2, AC4, AC8, AC10, AC12, AC13, AC14, AC15, AC16, AC17, AC18 and
AC19 need a real adapter that contracts,
the operator's M4 Max, the host the F-227 reviews used. AC3's red is
recorded on the base commit `e6b913e1` on that host in the same session as
AC1 and AC2, so the red and the green share an adapter identity. AC5's
vehicles are the native GL driver and the headless Chromium run, both
existing today (fact 24). AC6, AC9 and AC11's device paths run under
`gate gpu`, their decision units in the floor. AC7 is floor-runnable.
AC15 and AC16's height arithmetic is a floor-runnable unit next to the
device probe that pins the read.

Evidence invalidation: any change to `voi.wgsl`, `VoiParams`, the route
uniform's layout, the fold, or the pinned wgpu invalidates AC1 to AC5, AC8,
AC12 to AC16. A change to `auto_window.rs`'s margin invalidates AC7's
receipt. A change to the frame loop's load op or the slab stamp invalidates
AC17, AC18 and AC14. The F-041 sweep's `1e-4` tolerance is unchanged by
this story, and widening it is a design-plan decision, not a fix.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none. The table is sized at registration and
  rewritten in place, both outside the frame loop. The pre-apply scratch is
  one upload-time `Vec`, stated in Approach step 5, the same class as the
  existing padded-format path
- unsafe: none
- Tier A (WebGPU): full. Stage 1 is bit-equal to `ocelli_pixel::modality`
  through the folded table route and through the pre-applied routes. The
  folded table fits every reported Metal limit, 256 against 16384
- Tier B (WebGL2): full for the same routes. The route uniform and a
  non-filterable float texture are fragment-legal under HLD 7, the fold
  fits the 2048 downlevel floor, `textureLoad` filters nothing, and the
  table format follows `selected_stored_format`'s `R32Float` then
  `Rg32Float` then `Rgba32Float` precedent on this tier, written through
  `write_stored_samples` with channel padding when the format is wide.
  AC5 drives it through the two vehicles that exist today. The tier B
  uniform-block floor is verified: `downlevel_webgl2_defaults()` pins 11
  (`wgpu-types-30.0.1/src/limits.rs:646`) and the ray's group 0 holds 6
- Tier C (CPU): full and unchanged. `LutChain` is the reference the other
  two tiers are held to

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| fixture | AC1, AC2, AC4, AC12, AC15, AC16, hand-computed against PS3.3 C.11.1 and C.11.2.1.3.2 with the F-227 measured values, inputs pinned as hex bits | `crates/ocelli-render/tests/voi_shader.rs` |
| gpu | AC1, AC2, AC4, AC12, AC15, AC16, all `#[ignore]`d, through the composed module | `crates/ocelli-render/tests/voi_shader.rs`, `bin/ocelli.sh gate gpu` |
| gpu | AC6's counters, AC8's pre-applied upload, AC9's device paths, AC11's bind test, AC13's flip, AC14's stale flow, AC17's flip and flip-back, AC19's caller-view recovery | `crates/ocelli-render/tests/modality_route.rs`, `tests/upload.rs` for the existing counter shape, `tests/raycast.rs`, `gate gpu` |
| gpu | AC5's tier B vehicles, AC10's slab average and AC18's stale slab recovery | `tests/tier_b.rs`, `tools/tier-b/run.mjs`, `tests/slab.rs` |
| unit | AC9's four refusal decisions and AC11's slot arithmetic as pure comparisons, and AC17's builder entry | `crates/ocelli-render/src/graph.rs` and `src/voi.rs` under `#[cfg(test)]` |
| unit | AC15 and AC16's height arithmetic as pure comparisons | `crates/ocelli-render/src/graph.rs` under `#[cfg(test)]` |
| unit | AC7, the margin suite stays green and the sentences match the code | `cargo test -p ocelli-pixel` |
| gpu | The pass 4 and pass 5 surface grouping rule: a surface-bound viewport groups with no other viewport, in the same frame as a co-target's rescale change, for a stack and a slab | `crates/ocelli-render/src/graph.rs` `surface_grouping_tests` under `#[cfg(test)]`, `#[ignore]`d, `gate gpu` |

AC3 is not a fixture in the new file. It is the round-1 harness shape run
once on the base tree, recorded with the session that produces AC1 and AC2.

The fixtures derive from PS3.3 and the F-227 review measurements, never from
the implementation's output. Each new refusal test drives its named probe
red and its positive control green, per the workflow's guard rule. The
mutations the fixtures must survive: change one table entry, change the
fold split, truncate the table height to `count / fold` (AC15 and AC16 go
red), skip the table rebuild on a rescale change whose effective route is
pre-applied (AC17's flip-back draws 255 and 255 instead of 128 and 134),
change the flag-0
branch back to a multiply, wire the stale
flag into the frame loop's not-ready `continue` instead of a clear-only
pass (AC14, AC17 and AC18's zero readbacks become the preserved old
frame), leave the slab stamp without the pair or without the blend-only
guard (AC18's re-show stays a no-op), and
each named AC goes red for the reason it states.

## Parity surface covered

`docs/hld/B-parity-surface.md:27`, "VOI LUT functions, 3, LINEAR,
LINEAR_EXACT, SAMPLED_SIGMOID". This story establishes that the GPU routes
evaluate stage 1 with the same bits as the CPU chain on every tier, which is
the evidence the parity claim for those three functions rests on. It
establishes no new surface row and removes none.

## Deviations

None proposed. The route uniform and table binding extend the tree's own
composition contract, which `voi.wgsl`'s header documents, and touch no HLD
sentence: HLD 18.4 fixes binding 0 for the uniform and is silent about
others, HLD 22's key is unchanged, and the thirty-two-byte drag claim of
HLD 23 holds. If the lead prefers to record the contract change as a
deviation row anyway, it is written at approval, not by this draft.

## LLD impact

`docs/lld/pixel-pipeline.md`, the stage 1 section: the folded table and
pre-applied routes replace "two ULP at 255" with the measured bit-equality
claim and the stage-2 residual bound, and gain the padded-fold rule for a
container whose count is not a multiple of the fold. `docs/lld/render-graph.md`: the
reserved route uniform and table binding, the renumbered consumer bindings,
the fold, the `RouteInvalid` and `SampleOutsideTable` refusals and which
error type each registration returns, the effective route and its
flip-back, the content-stale draw rule and its clear-only pass for every
pass kind including the slab's `LoadOp::Load` override, the slab stamp's
staged rescale pair, the ray's
sampled-texture arithmetic, and the slab's reduction-space wording.

## User-facing documentation impact

`examples/folder-viewer/src/use.html`: the sentence at line 764 about the
GPU route gains the route wording, that integer frames take a folded
modality table computed by `ocelli-pixel` and every other frame pre-applies
stage 1, that both draw the same values as the CPU path, and that a rescale
change on already-staged float content draws the clear colour until the
next upload re-pre-applies it, or until a caller-owned view is rebound.
`examples/folder-viewer/src/more.html` line 387 says the slab reduction is
a shader in `ocelli-render`, which stays true, and gains one clause that the
reduction runs on modality values. `README.md` names no shader detail and
changes for nothing. The implementer searches all three documents for
`voi`, `rescale` and `stage 1` before review.

## Open questions

None remain. All three questions this draft carried are decided, and the
decisions are recorded here and carried into the body.

- **Q1, how stage 1 gets `modality`'s bits. DECIDED.** The operator answered
  O1-prime on 2026-11-11, in the round-3 instruction: a folded 256-wide
  two-dimensional modality table for whole integer containers, computed by
  `ocelli_pixel::modality` and read by the shader, plus stage 1 applied on
  the CPU for every interpolated route, with stage 1 existing only in
  `ocelli-pixel`. The plan above is written to that answer and affects
  every acceptance row. The accepted costs, computed against the tree:
  262,144 bytes per 16-bit tabled viewport and 1,024 per 8-bit, one
  `write_texture` of that size per rescale change and none per drag,
  65,536 `modality` calls per build, one allocation-free O(n) scan per
  tabled upload, one 4n-byte upload-time scratch plus n `modality` calls
  per pre-applied upload (a 512x512 ortho plane is 1,048,576 bytes of
  scratch, a 2048x2048 stack frame would be 16,777,216), the ray's brick
  slots 15 to 14 on a 16-limit adapter and 95 to 94 on the M4 Max, whose
  sampled count is 96 and whose bind-group limit is `u32::MAX`, stack
  group 0 uniform buffers 3 to 4 and the ray's 5 to 6, and 62 registration
  call sites across 16 files (fact 23).
  The CPU-everywhere alternative is removed as an option and kept as the
  rationale, because its costs were computed against the tree and are the
  reason the table exists: stage 1 applied on the CPU for every route into
  f32 textures makes every upload pay n `modality` calls plus a 4n-byte
  scratch, so a 2048x2048 U16 stack frame pays 4,194,304 calls and
  16,777,216 bytes per upload, per scroll step, where the table route pays
  a scan only. It saves the table memory, the sampled-texture slot, the
  group 0 renumbering and the route uniform, and the shader's stage 1
  becomes `return stored`. The decisive cost is the volume ray: bricks are
  shared integer textures keyed by `BrickKey` and the rescale is per
  viewport, so pre-applying into bricks is wrong across viewports, and
  per-viewport float brick copies cost `edge` cubed times 4 bytes.
  `BrickGrid::new` sets `edge` to 64 for `SixteenBit` and `Float32`
  (`crates/ocelli-cache/src/brick.rs:121-125`), so one float brick copy is
  64 cubed times 4 bytes, 1,048,576 bytes per brick per viewport. Round 2
  wrote 8,388,608 from a 128 edge, which is the `EightBit` edge and a
  one-byte voxel, and was wrong on both counts. The alternative must
  either keep the table for the ray, which reintroduces every binding cost
  it removed, or leave the ray contracted, which leaves the defect on a
  production route, and it must exempt palette frames, whose raw samples
  the palette indexes. The two earlier alternatives stay rejected on fact
  4: no source-level contraction control exists in the pinned naga 30.0.1
  or wgpu-hal 30.0.1, so neither explicit-`fma` emulation nor a backend
  setting can be verified not to contract.
- **Q2, stage 2's linear tail. DECIDED** in round 3, on the round-2
  proposal, by the operator's direction that the plan close with no open
  question: out of scope as a bit-equality fix. The tail's input is the
  modality value, which is continuous for float data and fractional for
  the slab average, so no table can serve it, and pre-applying stage 2
  would re-upload a display table on every drag and break HLD 23's
  thirty-two-byte rule. The measured impact of stage 2's contraction is
  bounded: F-227 pass 4 measured that where stage-1 values agree, stage
  2's error is relative to a quotient near 0.5 and cannot reach half a
  grey, and the F-041 sweep's `1e-4` tolerance holds today with an exact
  stage 1, which is the regime this story creates. The plan's invariant is
  therefore stage-1 bit equality plus the measured stage-2 bound, not
  whole-chain bit equality. Recorded in Scope's non-goals and the state
  invariant. Affects AC4 and AC8.
- **Q3, F-227's margin. DECIDED** in round 3, on the round-2 proposal,
  by the same direction: keep
  `crates/ocelli-pixel/src/auto_window.rs:520` at 16 and rewrite the two
  sentences whose only rationale was the tier difference, `:309-310` and
  `:470-476`, to state the margin's remaining role, the edge rounding
  F-227 pass 5 D2 measured, and that stage 1 no longer rounds differently
  on any tier once this story lands. The value stays because F-227's own
  mutation record has 15 and 17 red, so changing it would break
  closed-story evidence for no gain. Recorded as AC7.

## Lead design approval, 2026-11-11

Approved after four GLM design rounds and the coordinator amendment after round 4, on a final independent check with no defect and no smell. The operator decided Q1 as O1-prime. The final check's six nitpicks are implementation notes, not plan changes, and the implementer resolves each as stated:

1. AC18's second `set_voi A` carries AC8's chain, window centre `0x44E7C333` width `0x3C23D70A`, which is what makes the pair-A mean draw 12.32666015625. Under pair B's window it would draw 255.
2. `rebind` marks the slot fully dirty, as row 832 says, wherever Approach step 3 describes it.
3. A slab's stale state is derived (staged pair differs from recorded pair) everywhere the plan mentions a stale flag, including the `set_voi` PreApplied row, Approach step 6 and the Ownership pre-applied row. Only stack and caller-view content carries a stored flag.
4. The container derivation lives in `produce.rs` where the sidecar is read, and `gpu_candidate.rs` only gains the route parameter. Its own test calls to `draw` (about lines 237 and 243) pass `PreApplied`.
5. The count-0 refusal and the RouteInvalid decision unit get their own Tests row, and `RouteError` in `fill_modality_table`'s signature is defined as the shared check's error.
6. The header's "design round 4" note and its evidence-invalidation sentence are read as superseded by this section, and AC19's evidence depends on the stale clear and on rebind's dirty marking.

## Lead amendment after implementation review pass 1, 2026-11-11

**A stale Load pass never clears.** A slab composites over another viewport in the same target, which is why its pass loads. When a slab is stale, its pass keeps `LoadOp::Load` and issues no draw, and the graph marks every viewport that renders into the same target dirty, so the base layer redraws beneath it on that frame. The clear rule of the state table applies only to passes that own their target. An acceptance fixture registers a stack and a slab on two views of one texture, makes the slab stale with `set_voi`, and reads the stack's pixels intact where the slab is absent.

**AC5's tier B browser vehicle.** The headless Chromium WebGL2 run uses a second `wasm-bindgen` export in `crates/ocelli-wasm/src/canvas.rs` behind a cargo feature named `tier-b-probe`, off by default, so the shipped wasm and its size budget are unchanged. `tools/tier-b/run.mjs` builds with that feature and asserts 0.0, 255.0 and 127.5 with exit 0. Both files join the write set. The native GL vehicle records its miss on a host with no GL adapter, which is not the run, and the browser vehicle is the AC5 evidence.

## Lead amendment after implementation review pass 2, 2026-11-12

This replaces the stale-load paragraph of the pass 1 amendment. **One rule for every shared target, whatever the pass kinds.** When any viewport rendering into a target is stale on a frame, every viewport rendering into that target is marked dirty for that frame. The first pass encoded into the target on that frame clears it, every later pass into the same target loads, and a stale pass issues no draw whatever its kind or position. A target with a single viewport keeps the state table's clear rule. The result is that a stale stack shows the clear colour with any fresh slab drawn over it, a stale slab shows the base beneath it, and a target holding only slabs never keeps a stale composite. Fixtures: a stale stack with a fresh slab on one target reads the clear colour where the slab is absent and the slab where it is present, and two slabs on one target with no stack, one stale, read only the fresh slab's pixels.

## Lead amendment after implementation review passes 4 and 5, 2026-11-12

A viewport bound to a surface draws into its acquired surface texture, so the
graph excludes it from the shared-target grouping in both directions. It is
not marked by a stale co-target, it marks no co-target, and it is left out of
every first-into-target computation of the viewports sharing its registered
texture. The graph enforces this rather than relying on the caller giving a
surface viewport a texture no other viewport uses.

The fixture places the slot state that `bind_surface` writes through the
graph's own registry, because a native surface needs a window handle the test
harness cannot supply without `unsafe`. It exercises the grouping decision and
not the surface attachment choice, and its module doc says so. The flag
`surface_bound` and the attached surface must agree at every tick, which the
graph asserts in debug builds.

Any pass that draws into a freshly acquired surface texture clears it, because
an acquired texture's contents are undefined. No pass loads an acquired
surface texture.

