// Two volumes on one plane. Volume B is in front.
// `lut_chain` and `table_index` are prepended. This file does not re-type them.
// The stored plane holds modality values, because the graph pre-applies stage
// 1 on the CPU when it packs, and the route flag is 0. A texel holds 0 when
// presence is 0. That texel is not windowed.
// There is no sampler. `textureLoad` does not filter a border of zeros.

struct FusionParams {
    opacity_pet : f32,
    opacity_ct : f32,
    use_table : u32,
    last : u32,
}

@group(0) @binding(3) var<uniform> fusion : FusionParams;
@group(1) @binding(0) var stored_tex : texture_2d<f32>;
@group(1) @binding(1) var presence_tex : texture_2d<f32>;
@group(1) @binding(2) var colour_lut : texture_2d<f32>;
@group(1) @binding(3) var front_tex : texture_2d<f32>;

fn windowed_rgb(texel : vec2<i32>) -> vec3<f32> {
    let stored = textureLoad(stored_tex, texel, 0).r;
    let display = lut_chain(stored);
    if (fusion.use_table == 1u) {
        let index = table_index(display, fusion.last);
        return textureLoad(colour_lut, vec2<i32>(index, 0), 0).rgb;
    }
    let range = voi.ymax - voi.ymin;
    let unit = select(0.0, (display - voi.ymin) / range, range > 0.0);
    return vec3<f32>(unit);
}

@vertex
fn vs_main(@builtin(vertex_index) index : u32) -> @builtin(position) vec4<f32> {
    let u = f32((index << 1u) & 2u);
    let v = f32(index & 2u);
    return vec4<f32>(u * 2.0 - 1.0, v * 2.0 - 1.0, 0.0, 1.0);
}

@fragment
fn fs_window(@builtin(position) at : vec4<f32>) -> @location(0) vec4<f32> {
    if (at.x < 0.0 || at.y < 0.0) {
        return vec4<f32>(0.0, 0.0, 0.0, 0.0);
    }
    let texel = vec2<i32>(i32(at.x), i32(at.y));
    let dims = textureDimensions(presence_tex);
    if (u32(texel.x) >= dims.x || u32(texel.y) >= dims.y) {
        return vec4<f32>(0.0, 0.0, 0.0, 0.0);
    }
    let present = textureLoad(presence_tex, texel, 0).r;
    if (!(present > 0.0)) {
        return vec4<f32>(0.0, 0.0, 0.0, 0.0);
    }
    return vec4<f32>(windowed_rgb(texel), 1.0);
}

@fragment
fn fs_fuse(@builtin(position) at : vec4<f32>) -> @location(0) vec4<f32> {
    if (at.x < 0.0 || at.y < 0.0) {
        discard;
    }
    let texel = vec2<i32>(i32(at.x), i32(at.y));
    let dims = textureDimensions(presence_tex);
    if (u32(texel.x) >= dims.x || u32(texel.y) >= dims.y) {
        discard;
    }
    let pet_sample = textureLoad(front_tex, texel, 0);
    let pet_on = pet_sample.a > 0.0;
    let ct_on = textureLoad(presence_tex, texel, 0).r > 0.0;
    if (!pet_on && !ct_on) {
        discard;
    }
    var ct = vec3<f32>(0.0);
    if (ct_on) {
        ct = windowed_rgb(texel);
    }
    let pet = pet_sample.rgb;
    let opacity_pet = fusion.opacity_pet;
    let opacity_ct = fusion.opacity_ct;
    if (!pet_on) {
        return vec4<f32>(ct * opacity_ct, 1.0);
    }
    if (!ct_on) {
        return vec4<f32>(pet * opacity_pet, 1.0);
    }
    let out = pet * opacity_pet + ct * opacity_ct * (1.0 - opacity_pet);
    return vec4<f32>(out, 1.0);
}
