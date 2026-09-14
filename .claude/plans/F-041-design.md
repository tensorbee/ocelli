# F-041, WGSL LUT-chain shader

**Status**: approved
**Epic ref**: E6.5
**Sprint**: S11
**Estimate**: 4w

## Normative source, transcribed

### `docs/hld/15-lut-chain.md`, section 18, opening

|  |
|----|
| This is the highest-risk arithmetic in the project. It is specified in DICOM PS3.3 C.11 and the stages apply strictly in order. Implement it once, in ocelli-pixel, and let the shader read the parameters — do not let a second copy of this logic appear anywhere. |

The stage table, transcribed:

| **Stage** | **From → To** | **Source** |
|----|----|----|
| 1\. Modality LUT | Stored → Modality | Rescale slope/intercept, or a Modality LUT Sequence which takes precedence |
| 2\. VOI LUT | Modality → Display | Window centre/width with a function, or a VOI LUT Sequence |
| 3\. Presentation LUT | Display → Display | Identity or INVERSE; presentation state may override |
| 4\. Palette / ICC | Display → RGB | Palette colour LUT, or the display colour pipeline |

### Section 18.1, the modality stage

```rust
pub fn modality(sv: Stored, slope: f32, intercept: f32) -> Modality {
    Modality(sv.0 * slope + intercept)
}
```

### Section 18.2, the three VOI functions, character for character

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

**The comparisons are asymmetric.** Lower bound `<=`, upper bound `>`. Both
bounds, both functions, four operators, and writing any of them the other way
moves exactly one boundary value.

### Section 18.3, the fixture table

Soft-tissue CT window, centre 40, width 400, output range 0 to 255.

| **Input (HU)** | **LINEAR** | **LINEAR_EXACT** | **Why this row** |
|----|----|----|----|
| −160 | 0.000 | 1.594 | LINEAR boundary is c'−w'/2 = −160 exactly; the comparison is \<=, so this clamps |
| 40 | 127.819 | 127.500 | The window centre. A 0.32 divergence no reviewer would see by eye |
| 240 | 255.000 | 255.000 | LINEAR upper bound is c'+w'/2 = 239, so 240 clamps |
| −60 | 63.910 | 63.750 | Mid-lower quarter; catches sign and slope errors |

|  |
|----|
| **READ THIS ROW** At the window centre the two functions differ by 0.32 of 255. That is invisible to a human comparing screenshots and immediately visible to a pixel diff. It is the entire argument for building the oracle before writing the code it validates. |

**Row one's `LINEAR_EXACT` value is `0.000`, not `1.594`, under declared
deviation D-13**, which F-011, F-018 and F-029 already apply and which
`crates/ocelli-pixel/tests/voi.rs` already asserts. Section 18.2 clamps at
`x <= c - w/2 = -160`, and `-160 <= -160` holds.

### Section 18.4, the shader side, in full

```wgsl
// ocelli-render/shaders/voi.wgsl
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

|  |
|----|
| *A window-level drag updates thirty-two bytes per frame. No texture is re-uploaded, and that is the concrete performance claim behind Figure 2.* |

**Eight scalars, four bytes each, thirty-two bytes. The file path is
`ocelli-render/shaders/voi.wgsl` and the binding is `@group(0) @binding(0)`.**
All three are specified, not chosen.

### `docs/hld/05-rendering.md`, section 7, the tier bullet this story lives under

|  |
|----|
| **Two capability tiers, one codebase.** Tier A is WebGPU: compute shaders, storage buffers, 3D textures to 2048. Tier B is WebGL2 through wgpu's downlevel profile: fragment shaders only, no compute, no storage buffers, a conservative 3D-texture floor of 256. Every feature declares the tier it needs; the tier resolves once at startup. |

### `docs/hld/19-render-graph.md`, section 22

|  |
|----|
| **Pipelines compile at init**, keyed by (pass kind, blend mode, tier). Never compile a shader mid-frame. |

### `docs/hld/23-performance-rules.md`, section 26, the one this story's uniform exists for

Transcribed from the file:

|  |
|----|
| Prefer a uniform update to a texture update. Window/level is thirty-two bytes, not a re-upload. |

## What the specification does not cover

1. **What the shader does when the chain is driven by a LUT Sequence.**
   Section 18.4's uniform carries `slope`, `intercept`, `center` and `width`. It
   has no room for a Modality LUT Sequence or a VOI LUT Sequence, both of which
   section 18's stage table says take precedence when present. **The uniform
   cannot express them**, so a chain built on either has no shader path. This
   plan refuses rather than substituting the window values, which is HLD section
   31's rule generalised by D-07.
2. **Which end of the chain the shader's input is.** `slope` and `intercept` are
   in the uniform, so the shader's input is a **Stored** value and the shader
   runs stage 1 as well as stages 2 and 3. It is not handed a Modality value.
   Section 18.4 does not say this in words and its field list says it exactly.
3. **What `invert` does arithmetically.** Section 18.4 gives the flag and no
   formula. `PresentationTransform::apply` already owns it:
   `Display(ymin + ymax - display.0)`, PS3.3 C.11.6, a reflection about the
   midpoint of the output range. The shader transcribes that expression, it does
   not invent `1.0 - y` or `ymax - y`, both of which are correct only when
   `ymin` is zero.
4. **Where inversion is resolved.** It is resolved on the CPU, exactly once, by
   `LutChain::inverts`, which F-029 wrote for this purpose and whose doc comment
   already says "The single resolved inversion flag HLD section 18.4's uniform
   carries." **A shader that recomputes inversion from Photometric
   Interpretation is the double-inversion defect F-029 spent a story
   preventing.** This plan's shader has no photometric interpretation to
   recompute from, because the uniform does not carry one.
5. **The accessors that build the uniform.** F-029's plan says "F-029 exposes the
   six scalars and the two `u32` discriminants as plain accessors so a later
   shader story reads them". **It does not.** `VoiSelection::Window`'s fields and
   `ModalitySelection::Rescale`'s fields are private and only `output_range` and
   `inverts` are reachable. This story adds the accessors, which is the "later
   shader story" arriving.
6. **The tolerance for a shader-versus-CPU comparison.** Section 25.1's
   tolerances are written for the oracle's frames against cornerstone3D. This
   comparison is against this repository's own CPU path, in `f32`, with no
   quantisation to eight bits in between. Decided in Approach section 5.
7. **Whether the shader is a fragment shader or a compute shader.** Section 7
   makes tier B fragment-only. This plan writes a fragment shader, so one shader
   serves both GPU tiers and there is no tier B variant to keep in step. F-042
   is the WebGL2 fallback story and has less to do because of it.

## Approach

### 0. The thing this story is most likely to get wrong, stated first

`docs/sprints/CURRENT_SPRINT.md` says:

> F-041 writes a shader whose whole job is to apply `LINEAR`, `LINEAR_EXACT` and
> `SIGMOID`, and the obvious way to write it is to type the three formulas into
> WGSL. That is the forbidden second copy.

**Read literally that forbids the story, because section 18.4's uniform requires
the shader to evaluate the functions.** A shader given `center`, `width` and
`fn_kind` and told not to compute a window function has nothing to do with them,
and "a window-level drag updates thirty-two bytes per frame" is only true if the
thirty-two bytes are the parameters of a function the shader evaluates. The
alternative shape, a sampled LUT texture the shader indexes, makes a
window-level drag a texture re-upload, which is the thing section 26 says to
prefer a uniform over.

So the two readings have to be separated, and this plan takes the second:

- **Forbidden**: a second place where a LUT *decision* is made. Which
  comparison is `<=`. Whether a sequence beats a window. Which of several window
  pairs is selected. Whether a `MONOCHROME1` frame inverts. Whether a width of 1
  is legal. Every one of those is resolved on the CPU before the uniform is
  written, and the shader has no input that would let it re-decide any of them.
- **Required**: the three evaluation formulas, transcribed from section 18.2
  into WGSL, adding no arithmetic `ocelli-pixel` does not already own, with a
  test on a real device that the shader and `LutChain` agree.

`CURRENT_SPRINT.md`'s own "What done means" row for this story is the same
reading, and it is the one to hold: "transcribed rather than reinvented, and
**adds no arithmetic that `ocelli-pixel` does not already own**". The two
sentences in that file pull in different directions and the S11 consolidated
design round settled it this way. The rejected alternative and why is in
Decisions from the S11 design round.

### 1. `ocelli-pixel` gains the accessors and nothing else

No arithmetic is added. Three accessors over state the types already hold:

```rust
impl ModalityTransform {
    /// Section 18.4's `slope` and `intercept`, or `None` when a Modality LUT
    /// Sequence is selected, which the uniform cannot express.
    pub fn rescale(&self) -> Option<(f32, f32)>;
}

impl VoiTransform {
    /// Section 18.4's `center`, `width` and `fn_kind`, or `None` when a VOI LUT
    /// Sequence is selected.
    pub fn window(&self) -> Option<(f32, f32, VoiFunction)>;
}

impl LutChain {
    /// Stage 1's transform, so a caller can read its parameters.
    pub fn modality(&self) -> &ModalityTransform;
    /// Stage 2's transform, so a caller can read its parameters and range.
    pub fn voi(&self) -> &VoiTransform;
}
```

`LutChain::inverts` and `VoiTransform::output_range` already exist and are
unchanged. `VoiFunction` already has exactly the three variants section 18.4
numbers 0, 1 and 2.

### 2. `ocelli-render` owns the layout, because a uniform layout is a rendering concern

```rust
// crates/ocelli-render/src/voi.rs

/// HLD section 18.4's uniform, field for field and in its order.
///
/// Thirty-two bytes. `#[repr(C)]` and `bytemuck::Pod` so the bytes written are
/// the bytes declared, and section 20's third bullet says to use bytemuck here
/// rather than a hand-written transmute.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub struct VoiParams {
    pub center: f32,
    pub width: f32,
    pub slope: f32,
    pub intercept: f32,
    pub ymin: f32,
    pub ymax: f32,
    pub fn_kind: u32,
    pub invert: u32,
}

impl VoiParams {
    /// Read the parameters off a resolved chain. Computes nothing.
    ///
    /// # Errors
    ///
    /// [`VoiParamsError::SequenceNotExpressible`] when either stage is driven
    /// by a LUT Sequence. The uniform has no field for one, and substituting
    /// the window values would answer a different question.
    pub fn from_chain(chain: &LutChain) -> Result<Self, VoiParamsError>;
}
```

`fn_kind` is section 18.4's own numbering, `0 LINEAR, 1 LINEAR_EXACT, 2 SIGMOID`,
and the mapping from `VoiFunction` is a total match with no wildcard so a fourth
variant would stop compiling. `invert` is `u32::from(chain.inverts())` and
nothing else.

**`ocelli-render` gains a dependency on `ocelli-pixel`.** Section 4's crate
table forbids no direction, and section 18's instruction is that the shader
"read the parameters", which requires the renderer to be able to see them.

### 3. The shader, `crates/ocelli-render/shaders/voi.wgsl`

Section 18.4 gives the path as `ocelli-render/shaders/voi.wgsl`, so the file
goes in a new `shaders/` directory rather than beside `fill_rate.wgsl` in
`src/`. `fill_rate.wgsl` is not moved: it is a benchmark workload rather than a
render shader, and moving it would touch F-004's recorded workload, which
`ci/tier-thresholds.json` pins.

The uniform block is section 18.4's, character for character. The body is
section 18.2's three formulas, with the constant `4.0` of SIGMOID and the `0.5`
and `1.0` of LINEAR written as they appear there:

```wgsl
// PS3.3 C.11.2.1.2 -- LINEAR. Requires w >= 1, validated on the CPU.
fn voi_linear(x: f32, c: f32, w: f32, ymin: f32, ymax: f32) -> f32 {
    let cp = c - 0.5;
    let wp = w - 1.0;
    if (x <= cp - wp / 2.0) { return ymin; }
    if (x > cp + wp / 2.0) { return ymax; }
    return ((x - cp) / wp + 0.5) * (ymax - ymin) + ymin;
}
```

and the same shape for `LINEAR_EXACT` and `SIGMOID`. `select` is not used for
the two clamps, because `select` evaluates both arms and `wp` is zero when
`w == 1`, so the division would be evaluated at a width the early return exists
to skip.

Stage 1 and stage 3 are one line each:

```wgsl
fn lut_chain(stored: f32) -> f32 {
    // Stage 1, section 18.1. Stored -> Modality.
    let m = stored * voi.slope + voi.intercept;
    // Stage 2, section 18.2. Modality -> Display.
    var d = voi_window(m);
    // Stage 3, PS3.3 C.11.6. A reflection about the midpoint of the range,
    // NOT `ymax - d`, which is correct only when ymin is zero. The flag was
    // resolved once on the CPU by LutChain::inverts.
    if (voi.invert != 0u) { d = voi.ymin + voi.ymax - d; }
    return d;
}
```

Entry points are a `vs_main` drawing one oversized triangle and an `fs_main`
that reads a stored value and writes `lut_chain(stored)`. The stored value's
source is a texture in F-040's upload path, which does not exist, so this
story's shader takes it from a second small uniform of test inputs. That is
stated plainly rather than dressed as a design: **it is a test harness, and
F-040 replaces it with a texture sample.**

### 4. The evidence, and what class of evidence it is

`CURRENT_SPRINT.md` asks this story to say which it is, so:

> The shader's output is compared against `LutChain::map_into` over the section
> 18.3 fixture inputs and a wider sweep. That is a comparison against **this
> repository's own validated CPU path**. It is weaker than an oracle verdict,
> because both sides are ours and a shared misreading of PS3.3 would agree with
> itself. It is stronger than a screenshot, because it is a numeric diff at
> `f32` with no quantisation hiding a 0.32-of-255 divergence. The oracle's
> verdict on a rendered frame arrives when there is a renderer to render one,
> which is F-038 and F-040, not this story.

The CPU path's own claim to correctness is unchanged and comes from elsewhere:
the section 18.3 fixtures citing PS3.3 C.11.2.1.2 and C.11.2.1.3.2, already in
`crates/ocelli-pixel/tests/voi.rs`.

**The shader is also asserted directly against the four section 18.3 rows**,
not only against the CPU. If both sides drifted together, the fixture rows still
go red, and that is what keeps the comparison from being circular.

Readback is needed, and it is worth being explicit that this is allowed:
decision D3 is that pixels never cross **the wasm boundary**. A native test
mapping a buffer on the host is not that boundary. `probe.rs` avoids readback
for a different reason, which is that it would measure transfer rather than
shading.

### 5. The tolerance, written down once with its rationale

Section 25.1's bounds are written for the oracle's eight-bit frames against
cornerstone3D and none of them fits a `f32`-against-`f32` comparison on one
machine. This story therefore writes a tolerance for the first time, which
section 25.1's own last bullet governs: "A tolerance change is a pull request
with a rationale, reviewed like code."

- **`1e-4` absolute over the `[0, 255]` output range** for the sweep. That is
  4e-7 of full scale, and the divergence this project exists to catch is 0.32 of
  255, so the bound is three and a half orders of magnitude tighter than the
  thing it has to detect.
- **`0.001` for the four section 18.3 rows**, which is the constant
  `crates/ocelli-pixel/tests/voi.rs` already uses for the same four numbers. The
  two suites then state the same thing about the same values, and a reviewer
  comparing them sees one tolerance rather than two.

**Bit-exactness is not claimed and not required.** WGSL's `exp` is not required
to be correctly rounded and the CPU's comes from glam's libm backend, so SIGMOID
would differ on some hardware for a reason that is not a defect. Decision D14 is
explicit: claim MEASURED divergence, never bit-exact reproducibility. The
measured divergence across the sweep is recorded in
`docs/lld/pixel-pipeline.md` when the story lands, as a number rather than a
claim.

**No row is added to HLD section 25.1.** That would be an HLD edit needing its
own deviation, and this tolerance governs an internal comparison rather than the
oracle's verdict, which is what section 25.1 is about.

### 6. The gate that runs any of this

F-037's Approach section 7 adds a `gpu|YES` gate running
`cargo test -p ocelli-render --test device --test voi_shader -- --ignored`, and
**`voi_shader.rs` is this story's half of it**. Without that gate every test in
the table below is `#[ignore]`d and run by no profile, and this story's stated
evidence would be evidence nothing checks. The gate is F-037's to add because it
lands first in the wave order, and this story's implementation must confirm the
gate names its test file rather than assuming it.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no. The readback is host-side in a native test and
  never reaches `ocelli-wasm`. D3 is untouched
- Render-loop allocation: none. `VoiParams` is 32 bytes of plain data, written
  with `queue.write_buffer` into a buffer created at init. The pipeline is
  compiled at init, which is section 22's rule. **This is the story that makes
  section 26's "window/level is thirty-two bytes, not a re-upload" true**
- unsafe: none. `bytemuck::Pod` is the derive, not a transmute, which is section
  20's third bullet
- Tier A (WebGPU): **full.** A fragment shader and one uniform buffer, both
  core WebGPU
- Tier B (WebGL2): **full, and this is the reason it is a fragment shader.**
  Section 7 gives tier B fragment shaders only, no compute, no storage buffers.
  This shader uses a fragment entry point and a uniform buffer, so it needs
  nothing tier B lacks and there is **no tier B variant**. The GPU test asserts
  the shader compiles and agrees on whichever tier the machine resolves, and
  `CURRENT_SPRINT.md` is right that nothing has ever run on tier B, so this is
  the first story whose shader could be tried there. It does not prove tier B on
  a machine that resolves tier A, and the plan says so rather than claiming it
- Tier C (CPU): **n/a as a shader, and the arithmetic is already full.**
  Deviation D-07 makes `ocelli-pixel` tier C's authoritative path, and
  `LutChain::map_into` is it. This story reimplements nothing for tier C: a tier
  C session runs the same arithmetic that the shader is compared against. That
  is the correct tier C answer for a shader story and it is an answer, not an
  omission

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| `fixture` | `VoiParams` is exactly 32 bytes and its fields are at offsets 0, 4, 8, 12, 16, 20, 24, 28, which is section 18.4's declaration and section 26's "thirty-two bytes" | `crates/ocelli-render/tests/voi_params.rs` |
| `fixture` | `fn_kind` is 0 for `Linear`, 1 for `LinearExact`, 2 for `Sigmoid`, section 18.4's own comment | same |
| `fixture` | `from_chain` on a soft-tissue chain, centre 40 width 400 rescale slope 1 intercept -1024 `MONOCHROME2`, yields `center: 40.0, width: 400.0, slope: 1.0, intercept: -1024.0, ymin: 0.0, ymax: 255.0, fn_kind: 0, invert: 0`, every field hand-written | same |
| `fixture` | The same chain with `MONOCHROME1` yields `invert: 1` and **changes nothing else**, so inversion enters the uniform by the one resolved flag and by no other field | same |
| `unit` | A Modality LUT Sequence chain refuses with `SequenceNotExpressible`, and a VOI LUT Sequence chain refuses too. Neither silently reports the window values | `crates/ocelli-render/src/voi.rs` |
| `unit` | The WGSL source declares `struct VoiParams` with the eight field names in section 18.4's order, and `@group(0) @binding(0)`. A text assertion, which is weak, and it is here because it is the only check that survives on a machine with no adapter | same |
| `browser` (device) | **The four section 18.3 rows, on the GPU.** Centre 40, width 400, range 0 to 255, slope 1 intercept 0, no inversion. LINEAR gives 0.000, 127.819, 255.000, 63.910 and LINEAR_EXACT gives 0.000, 127.500, 255.000, 63.750, D-13 applied. Hand-computed values, not CPU-derived | `crates/ocelli-render/tests/voi_shader.rs` |
| `browser` (device) | **The boundaries, where the asymmetric comparisons live.** LINEAR at -160 and -159, at 238 and 239. LINEAR_EXACT at -160, -159, 239 and 240. The same eight points `crates/ocelli-pixel/tests/voi.rs` already pins on the CPU | same |
| `browser` (device) | **SIGMOID at -60 is `255 / (1 + e)`**, PS3.3 C.11.2.1.3.1 with the exponent exactly +1, hand-computed and independent of the CPU's glam-libm `exp` | same |
| `browser` (device) | **Inversion is a reflection about the midpoint.** Range `[16, 235]`, a value mapping to 100, inverted is 151. `ymax - y` gives 135 and `1 - y` is nonsense, so both wrong forms are separated | same |
| `browser` (device) | **Agreement with `LutChain::map_into` over a sweep**, 4096 stored values across the window and beyond both clamps, all three functions, inverted and not, within `1e-4` absolute per Approach section 5 | same |
| `browser` (device) | **The divergence the whole project is about is visible.** The same sweep under LINEAR and under LINEAR_EXACT differs by 0.32 at the window centre on the GPU, so a shader that quietly used one formula for both is caught | same |
| `browser` (device) | Updating the window is a `write_buffer` of 32 bytes and no texture is created or written between two draws, asserted by the absence of a texture write in the recorded sequence | same |

**Mutation check, HLD 27.3.** Each applied to the WGSL, the named test observed
red, the mutation reverted:

1. `x < cp - wp/2.0` instead of `<=`. The LINEAR boundary row at -160 goes red.
2. `x >= cp + wp/2.0` instead of `>`. The LINEAR row at 239 goes red.
3. Drop the `- 0.5` and `- 1.0` from LINEAR so it becomes LINEAR_EXACT. The
   window-centre row goes red at 127.819 against 127.500, which is the 0.32 the
   HLD's READ THIS ROW is about.
4. `d = voi.ymax - d` for inversion. The non-zero-`ymin` row goes red.
5. `exp(-2.0 * ...)` in SIGMOID. The `255 / (1 + e)` row goes red.
6. Ignore `voi.invert` entirely. The `MONOCHROME1` sweep goes red.

Mutations 1, 2 and 3 are the ones a sweep alone would miss or nearly miss, which
is why the boundary rows exist separately from the sweep.

**On the `browser` (device) label**, see F-037's test table note. The taxonomy
in `.claude/WORKFLOW.md` has no category for a native test needing a real
adapter, `browser` is the nearest, and the label is qualified rather than
invented.

## Parity surface covered

`docs/hld/B-parity-surface.md`'s surface table row **VOI LUT functions, count 3,
"LINEAR, LINEAR_EXACT, SAMPLED_SIGMOID"**. All three reach the GPU with this
story. The appendix says `SAMPLED_SIGMOID` and PS3.3 C.11.2.1.3.1 says
`SIGMOID`, which F-029's plan already recorded as an observation rather than a
deviation, and this story inherits that reading unchanged. The appendix has no
`Covered by` column in this repository to update.

## Deviations

**D-13, already declared**, applied to the section 18.3 row-one value for the
third time after F-011 and F-029. No new row.

No new deviation is expected. The refusal for a LUT Sequence is not a departure:
section 18.4's uniform has no field for one, so refusing is what the
specification's own field list implies, and HLD section 31's unavailable rule is
what it is refused under.

The sampled-LUT-texture alternative was rejected in the S11 design round, and it
is worth recording that it would have needed a deviation: it leaves six of
section 18.4's eight fields dead and makes a window-level drag a texture upload,
which contradicts section 26's "prefer a uniform update to a texture update".
The reading that was taken needs none, because it implements section 18.4 as
written.

The `1e-4` tolerance of Approach section 5 adds **no** row to HLD section 25.1
and is therefore not a deviation either. It governs an internal comparison
rather than the oracle's verdict.

## LLD impact

- `docs/lld/pixel-pipeline.md`, which gains the accessors and the statement that
  the uniform is read from the chain rather than rebuilt beside it
- `docs/lld/gpu-ownership.md`, which gains `ocelli-render`'s new dependency on
  `ocelli-pixel` and why the direction is right
- `docs/lld/feature-availability.md`, which gains the LUT Sequence row: a chain
  driven by a sequence reports unavailable on the GPU and is full on tier C

## Open questions

None. All were answered in the S11 consolidated design round, and the tolerance
was decided in this plan with its rationale in Approach section 5.

## Decisions from the S11 design round

**1. The shader evaluates the three formulas, and the CPU owns every
decision.** Approach section 0 is the reasoning and this is the reading that was
approved. The alternative, a sampled LUT texture the shader indexes, was
rejected: it puts the arithmetic in exactly one place with no transcription at
all, and it does so by making a window-level drag a texture re-upload, which
contradicts HLD section 26's "prefer a uniform update to a texture update.
Window/level is thirty-two bytes, not a re-upload", and by leaving six of section
18.4's eight uniform fields with nothing to do.

**The consequence to hold in review, stated because it is the opposite of what
`CURRENT_SPRINT.md`'s warning paragraph reads like on its own:** there WILL be
three window formulas written in WGSL in this repository, and a reviewer looking
for "no second copy" will find something that looks like one. What makes it not
one is that the shader has no input from which it could re-decide anything. It
receives a resolved `invert` flag and no Photometric Interpretation, a selected
`center` and `width` and no window multiplicity, a `slope` and `intercept` and no
Modality LUT Sequence, and a validated `width` it never checks. Every decision
`ocelli-pixel` owns is absent from the uniform by construction, and that absence
is the mechanism. The review item is therefore **"can the shader re-decide
anything?"** and not "does the shader contain a formula?"

The third option, a checked-in canonical text of the formulas that both the WGSL
and a Rust comment must match by string comparison, was declined. It catches a
future one-sided edit and costs a third place the formulas are written, which is
the thing section 18 warns about, and the numeric agreement test catches the same
edit with no extra copy.

**2. The GPU tests are run by a new `gpu|YES` gate**, added by F-037 and
described in Approach section 6. Without it this story's evidence is checked by
no profile.

**3. `ocelli-render` is not wired into `ocelli-wasm` this sprint**, so no shader
byte reaches the wasm size budget and gate A4's measurement waits for F-039.
F-037's Approach section 8 is the change and the correction to the stale
forecast.

**4. The tolerance is `1e-4` for the sweep and `0.001` for the section 18.3
rows.** Approach section 5, decided in this plan rather than asked, because
section 25.1's own rule is that a tolerance comes with a written rationale and
this one is being written for the first time rather than changed.
