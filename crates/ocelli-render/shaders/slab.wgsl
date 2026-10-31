// Thick slab. Host samples are already in the 3D texture.
// `slab_blend` is a pipeline override, so each compiled pipeline keeps one arm.
// 0 composite is the centre sample. 1 MIP, 2 MinIP and 3 average reduce
// stored values. `lut_chain` runs once after that reduction.
// An absent pixel is mask 0. Discard runs before any value load.

override slab_blend : u32;

@group(1) @binding(0) var values : texture_3d<f32>;
@group(1) @binding(1) var mask : texture_2d<f32>;

fn reduced(at : vec2<i32>, depth : u32) -> f32 {
    if (slab_blend == 0u) {
        return textureLoad(values, vec3<i32>(at, i32(depth / 2u)), 0).r;
    }
    if (slab_blend == 1u) {
        var best = textureLoad(values, vec3<i32>(at, 0), 0).r;
        for (var z : u32 = 1u; z < depth; z = z + 1u) {
            let sample = textureLoad(values, vec3<i32>(at, i32(z)), 0).r;
            best = max(best, sample);
        }
        return best;
    }
    if (slab_blend == 2u) {
        var best = textureLoad(values, vec3<i32>(at, 0), 0).r;
        for (var z : u32 = 1u; z < depth; z = z + 1u) {
            let sample = textureLoad(values, vec3<i32>(at, i32(z)), 0).r;
            best = min(best, sample);
        }
        return best;
    }
    var sum = 0.0;
    for (var z : u32 = 0u; z < depth; z = z + 1u) {
        sum = sum + textureLoad(values, vec3<i32>(at, i32(z)), 0).r;
    }
    return sum / f32(depth);
}

@vertex
fn vs_main(@builtin(vertex_index) index : u32) -> @builtin(position) vec4<f32> {
    let u = f32((index << 1u) & 2u);
    let v = f32(index & 2u);
    return vec4<f32>(u * 2.0 - 1.0, v * 2.0 - 1.0, 0.0, 1.0);
}

@fragment
fn fs_main(@builtin(position) at : vec4<f32>) -> @location(0) vec4<f32> {
    let value_dims = textureDimensions(values);
    let mask_dims = textureDimensions(mask);
    if (at.x < 0.0 || at.y < 0.0) {
        discard;
    }
    let x = i32(at.x);
    let y = i32(at.y);
    if (u32(x) >= value_dims.x || u32(y) >= value_dims.y || u32(x) >= mask_dims.x || u32(y) >= mask_dims.y) {
        discard;
    }
    let covered = textureLoad(mask, vec2<i32>(x, y), 0).r;
    if (!(covered > 0.0)) {
        discard;
    }
    let display = lut_chain(reduced(vec2<i32>(x, y), value_dims.z));
    let range = voi.ymax - voi.ymin;
    let unit = select(0.0, (display - voi.ymin) / range, range > 0.0);
    return vec4<f32>(unit, unit, unit, 1.0);
}
