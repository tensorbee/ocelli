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
// The input is a STORED value, not a Modality value. Section 18.4 puts `slope`
// and `intercept` in the uniform, so stage 1 is the shader's too.
fn lut_chain(stored : f32) -> f32 {
    // Stage 1, section 18.1. Stored -> Modality.
    let m = stored * voi.slope + voi.intercept;

    // Stage 2, section 18.2. Modality -> Display.
    var d = voi_window(m);

    // Stage 3, PS3.3 C.11.6. A reflection about the midpoint of the output
    // range, NOT `ymax - d`, which is correct only when ymin is zero. The flag
    // was resolved once on the CPU by LutChain::inverts.
    if (voi.invert != 0u) {
        d = voi.ymin + voi.ymax - d;
    }
    return d;
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
// 18.4's and is reserved by this file. Bindings 1 upward belong to the
// consumer.
//
// Today there is exactly one consumer and it is a test:
// `tests/voi_shader.rs` appends a compute entry point over two storage buffers,
// because it needs to push arbitrary inputs in and read exact f32 values back,
// and it runs natively on a machine this project resolves as tier A. **That
// harness is tier A only and this shader is not.** F-038's render graph and
// F-040's texture upload are what append a fragment entry point sampling a
// stored value from a texture, which is the tier-B-legal production path, and
// they append it to this same text.
