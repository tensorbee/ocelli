// The startup fill-rate workload of F-004. See `probe.rs`.
//
// Fragment throughput is what separates a software rasteriser from a real
// adapter by orders of magnitude, and it is also what a stack viewport
// actually spends its frame on, so it is the right thing to measure.
//
// This shader is NOT part of the render graph and never draws anything a user
// sees. Its output texture is written and never read back.

// One oversized triangle covering the whole viewport, with no vertex buffer.
//
//   index 0 -> (-1, -1)
//   index 1 -> (-1,  3)
//   index 2 -> ( 3, -1)
//
// which covers clip space entirely, so every texel of the target is shaded
// exactly once per pass and the pixel count is width * height * passes.
@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let i = i32(index);
    let x = f32(i / 2) * 4.0 - 1.0;
    let y = f32(i & 1) * 4.0 - 1.0;
    return vec4<f32>(x, y, 0.0, 1.0);
}

// A fixed count of ALU operations per fragment. The accumulator is seeded from
// the fragment position so the loop cannot be hoisted out of the fragment
// stage, and the result reaches the colour output so it cannot be dropped.
//
// FILL_RATE_ALU_STEPS in probe.rs records this count. The two have to agree
// only in the sense that changing one invalidates a recorded measurement, and
// `ci/tier-thresholds.json` records the workload it was measured with.
@fragment
fn fs_main(@builtin(position) pos: vec4<f32>) -> @location(0) vec4<f32> {
    var acc = pos.x * 0.0009765625 + pos.y * 0.00048828125;
    for (var step = 0; step < 64; step = step + 1) {
        acc = fma(acc, 1.0000305175781, 0.0001220703125);
    }
    return vec4<f32>(fract(acc), fract(acc * 2.0), 0.0, 1.0);
}
