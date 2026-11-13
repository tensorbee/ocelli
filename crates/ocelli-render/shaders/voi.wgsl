// ocelli-render/shaders/voi.wgsl
//
// DICOM PS3.3 C.11 stages 1 to 3, evaluated per fragment from HLD section
// 18.4's uniform.
//
// THIS SHADER MAKES NO LUT DECISION, AND THAT IS THE WHOLE CONTRACT.
// HLD section 18: "Implement it once, in ocelli-pixel, and let the shader read
// the parameters - do not let a second copy of this logic appear anywhere."
// What must exist once is every DECISION, and this shader is given no input
// from which it could re-make one:
//
//   - It receives a resolved `invert` flag and NO photometric interpretation,
//     so it cannot recompute inversion. That is the double-inversion defect
//     F-029 spent a story preventing. `LutChain::inverts` resolved it on the
//     CPU, exactly once, from PS3.3 C.11.6 and C.7.6.3.1.2.
//   - It receives one selected `center` and `width` and NO window
//     multiplicity, so it cannot select a different pair.
//   - It receives `slope` and `intercept` and NO sequence, so it cannot apply
//     one. A chain driven by a Modality LUT Sequence or a VOI LUT Sequence has
//     no uniform to be expressed in and reports unavailable on the host side,
//     which is HLD section 31's rule generalised by deviation D-07.
//   - It receives a `width` already validated against each function's domain,
//     `w >= 1` for LINEAR and `w > 0` for the other two, so it is never handed
//     a width the standard forbids. That validation does NOT stop `w - 1` being
//     zero, because `w = 1` is legal, and what keeps the division unreachable
//     there is the pair of boundary comparisons below.
//
// The three window functions below ARE written out here, because section
// 18.4's uniform specifies `center`, `width` and `fn_kind` and a shader given
// those has to evaluate something. They are transcribed from section 18.2
// character for character and add no arithmetic `ocelli-pixel` does not own.
// `tests/voi_shader.rs` asserts they agree with `LutChain::map_into`.

// HLD section 18.4, field for field and in its order. Thirty-two bytes.
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

// The modality route, F-235. Stage 1 exists once, in `ocelli-pixel`, and this
// shader is given its RESULTS rather than a second copy of the formula:
//
//   - `tabled` 1 reads the folded modality table, whose entries are
//     `ocelli_pixel::modality`'s own outputs for the container's whole
//     range. `first` is the container's smallest stored value and `fold`
//     the texels per row, so the index of a stored value `v` is
//     `i32(v) - first`, split as `x = index % fold`, `y = index / fold`,
//     identically to the CPU builder. The route is resolved on the CPU and
//     this shader still makes no LUT decision.
//   - `tabled` 0 returns its input unchanged, because the texture already
//     holds modality values that a CPU pre-apply staged. The uniform's
//     `slope` and `intercept` are then not read at all, and a caller that
//     hands raw stored values with flag 0 draws them unrescaled, by
//     contract.
//
// The Rust validation keeps every index inside the table: an all-integral
// upload with a sample outside the declared container is refused before any
// write, and the pre-applied routes never index at all.
struct ModalityRoute {
    tabled : u32, // 1 reads the folded table, 0 returns its input
    first : i32,  // the container's smallest stored value
    fold : u32,   // texels per table row
    pad : u32,
};
@group(0) @binding(1) var<uniform> modality_route : ModalityRoute;
@group(0) @binding(2) var modality_table : texture_2d<f32>;

// PS3.3 C.11.2.1.2 -- LINEAR. Requires w >= 1.
// c' = c - 0.5 ; w' = w - 1
// x <= c' - w'/2 -> ymin
// x > c' + w'/2 -> ymax
// else y = ((x - c') / w' + 0.5) * (ymax - ymin) + ymin
//
// Early returns rather than `select`, because `select` evaluates both arms and
// `wp` is zero at the legal width of 1, so the division would be evaluated at
// exactly the width the first comparison exists to skip.
fn voi_linear(x : f32, c : f32, w : f32, ymin : f32, ymax : f32) -> f32 {
    let cp = c - 0.5;
    let wp = w - 1.0;
    if (x <= cp - wp / 2.0) { return ymin; }
    if (x > cp + wp / 2.0) { return ymax; }
    return ((x - cp) / wp + 0.5) * (ymax - ymin) + ymin;
}

// PS3.3 C.11.2.1.3.2 -- LINEAR_EXACT. Requires w > 0.
// x <= c - w/2 -> ymin
// x > c + w/2 -> ymax
// else y = ((x - c) / w + 0.5) * (ymax - ymin) + ymin
//
// NO `- 0.5` AND NO `- 1`. The difference from LINEAR is a half and a one, and
// it is the single most commonly mis-ported detail in DICOM viewers. At the
// centre of the soft-tissue window the two differ by 0.32 of 255.
fn voi_linear_exact(x : f32, c : f32, w : f32, ymin : f32, ymax : f32) -> f32 {
    if (x <= c - w / 2.0) { return ymin; }
    if (x > c + w / 2.0) { return ymax; }
    return ((x - c) / w + 0.5) * (ymax - ymin) + ymin;
}

// PS3.3 C.11.2.1.3.1 -- SIGMOID. Requires w > 0.
// y = (ymax - ymin) / (1 + exp(-4 * (x - c) / w)) + ymin
//
// No clamps. The sigmoid is asymptotic and never leaves the open range, which
// is why the other two have boundary comparisons and this does not.
fn voi_sigmoid(x : f32, c : f32, w : f32, ymin : f32, ymax : f32) -> f32 {
    return (ymax - ymin) / (1.0 + exp(-4.0 * (x - c) / w)) + ymin;
}

// The lower bound is `<=` and the upper bound is `>`, in both linear functions,
// transcribed from PS3.3 C.11.2.1.2 and C.11.2.1.3.2. They are what the
// standard says and what `ocelli-pixel` does, and that is the whole reason they
// are written this way.
//
// `voi_linear_at_width_one_pins_the_lower_boundary_operator` is the test that
// holds the lower one. At `w = 1`, which PS3.3 permits, `w' = 0` and the body
// is `0 / 0`, so `<` instead of `<=` lets `x == c'` reach the division and the
// shader returns NaN where it must return `ymin`. Measured on a real adapter.
//
// **THREE EARLIER VERSIONS OF THIS COMMENT ARGUED ABOUT WHICH OPERATOR IS
// OBSERVABLE AND ALL THREE WERE WRONG.** The argument is deleted rather than
// corrected a fourth time. Nothing depends on it: the operators are kept
// because the specification writes them, not because a mutation catches them,
// and the F-041 review's second pass found legal windows where the upper
// operator changes the output by 42.5 of 255, which is the opposite of what the
// third version claimed. A comment that has shipped wrong three times about a
// property nothing relies on is worth less than the space it takes.

fn voi_window(x : f32) -> f32 {
    if (voi.fn_kind == 1u) {
        return voi_linear_exact(x, voi.center, voi.width, voi.ymin, voi.ymax);
    }
    if (voi.fn_kind == 2u) {
        return voi_sigmoid(x, voi.center, voi.width, voi.ymin, voi.ymax);
    }
    return voi_linear(x, voi.center, voi.width, voi.ymin, voi.ymax);
}

// Stages 1 to 3, in PS3.3 C.11's order, applied once each.
//
// The input of `lut_chain` is a STORED value, not a Modality value. Section
// 18.1's stage 1 reads the folded table this file reserves at binding 2, or
// returns its input when the route flag says the bytes are already modality
// values. Sections 18.2 and C.11.6 are evaluated below, from the section
// 18.4 uniform.
fn modality_stage_one(stored : f32) -> f32 {
    if (modality_route.tabled != 0u) {
        let index = i32(stored) - modality_route.first;
        let x = index % i32(modality_route.fold);
        let y = index / i32(modality_route.fold);
        return textureLoad(modality_table, vec2<i32>(x, y), 0).r;
    }
    return stored;
}

fn voi_stages(modality_value : f32) -> f32 {
    // Stage 2, section 18.2. Modality -> Display.
    var d = voi_window(modality_value);

    // Stage 3, PS3.3 C.11.6. A reflection about the midpoint of the output
    // range, NOT `ymax - d`, which is correct only when ymin is zero. The flag
    // was resolved once on the CPU by LutChain::inverts.
    if (voi.invert != 0u) {
        d = voi.ymin + voi.ymax - d;
    }
    return d;
}

fn lut_chain(stored : f32) -> f32 {
    return voi_stages(modality_stage_one(stored));
}

// THERE IS NO ENTRY POINT IN THIS FILE, DELIBERATELY.
//
// HLD section 7 makes tier B "fragment shaders only, no compute, no storage
// buffers". Everything above is tier-B-legal: a uniform block and scalar
// arithmetic, nothing else. An entry point here would decide for every consumer
// how a stored value arrives and where a display value goes, and those are
// exactly the choices that differ between tiers and between stories.
//
// So this file is COMPOSED rather than compiled on its own. WGSL has no
// include directive, so a consumer concatenates `ocelli_render::VOI_WGSL` ahead
// of its own entry point and bindings. `@group(0) @binding(0)` is section
// 18.4's, and bindings 1 and 2 are the route uniform and the folded table
// reserved by this file. Bindings 3 upward belong to the consumer.
//
// Today there are several consumers and they are all in this crate: the
// render graph's stack body, the volume ray, the slab and the fusion, and
// `tests/voi_shader.rs`, which appends compute entry points over two storage
// buffers because it needs to push arbitrary inputs in and read exact f32
// values back. The harness binds its storage at bindings 3 and 4, past the
// reserved set. **That harness is tier A only and this shader is not.** The
// consumers above append fragment entry points sampling a stored value from a
// texture, which is the tier-B-legal production path, and they append it to
// this same text.
