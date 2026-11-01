// SegOverlay composite. The texel is the presenter byte.
// This shader does not evaluate the image VOI and does not test neighbours.

@group(0) @binding(0) var overlay : texture_2d<f32>;

@vertex
fn vs_main(@builtin(vertex_index) index : u32) -> @builtin(position) vec4<f32> {
    let u = f32((index << 1u) & 2u);
    let v = f32(index & 2u);
    return vec4<f32>(u * 2.0 - 1.0, v * 2.0 - 1.0, 0.0, 1.0);
}

@fragment
fn fs_main(@builtin(position) at : vec4<f32>) -> @location(0) vec4<f32> {
    let dims = textureDimensions(overlay);
    let x = i32(at.x);
    let y = i32(at.y);
    if (x < 0 || y < 0 || u32(x) >= dims.x || u32(y) >= dims.y) {
        discard;
    }
    return textureLoad(overlay, vec2<i32>(x, y), 0);
}
