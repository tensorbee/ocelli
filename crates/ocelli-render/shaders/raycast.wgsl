// Volume composite ray. Appended to voi.wgsl.
// One step is one voxel. A residency flag of 0 skips the step before any
// brick is read. Each ready brick is its own texture_3d. The residency
// texture is the last binding in group 1. Opacity is the constant 0.5 until
// F-060. No storage buffer and no compute.
//
// volume_ray_module fills the slot count, the brick bindings and the load
// cases. The slot count is the device's sampled-texture room after the
// residency binding.

struct RayParams {
    axis : u32,
    from_far : u32,
    dim_x : u32,
    dim_y : u32,
    dim_z : u32,
    steps : u32,
    kind : u32,
    edge : u32,
    brick_count : u32,
    pad0 : u32,
    pad1 : u32,
    pad2 : u32,
    coords : array<vec4<u32>, __RAY_SLOTS__>,
}

@group(0) @binding(1) var<uniform> ray : RayParams;

__BRICK_BINDINGS__

@vertex
fn vs_main(@builtin(vertex_index) index : u32) -> @builtin(position) vec4<f32> {
    let u = f32((index << 1u) & 2u);
    let v = f32(index & 2u);
    return vec4<f32>(u * 2.0 - 1.0, v * 2.0 - 1.0, 0.0, 1.0);
}

fn inside_image(px : u32, py : u32) -> bool {
    if (ray.axis == 0u) {
        return px < ray.dim_y && py < ray.dim_z;
    }
    if (ray.axis == 1u) {
        return px < ray.dim_x && py < ray.dim_z;
    }
    return px < ray.dim_x && py < ray.dim_y;
}

fn voxel_of(px : u32, py : u32, step : u32) -> vec3<i32> {
    // 0 walks x. 1 walks y. 2 walks z. The pixel axes are the other two.
    if (ray.axis == 0u) {
        return vec3<i32>(i32(step), i32(px), i32(py));
    }
    if (ray.axis == 1u) {
        return vec3<i32>(i32(px), i32(step), i32(py));
    }
    return vec3<i32>(i32(px), i32(py), i32(step));
}

fn slot_for(coord : vec3<u32>) -> i32 {
    for (var i = 0u; i < ray.brick_count; i = i + 1u) {
        let found = ray.coords[i];
        if (found.x == coord.x && found.y == coord.y && found.z == coord.z) {
            return i32(i);
        }
    }
    return -1;
}

fn load_brick(slot : u32, coord : vec3<i32>) -> u32 {
    switch slot {
        __BRICK_CASES__
        default: { return 0u; }
    }
}

@fragment
fn fs_main(@builtin(position) at : vec4<f32>) -> @location(0) vec4<f32> {
    let px = u32(at.x);
    let py = u32(at.y);
    if (!inside_image(px, py)) {
        return vec4<f32>(0.0, 0.0, 0.0, 0.0);
    }
    var accum_c = 0.0;
    var accum_a = 0.0;
    let n = ray.steps;
    for (var i = 0u; i < n; i = i + 1u) {
        let step = select(i, n - 1u - i, ray.from_far != 0u);
        let flag = textureLoad(resident_tex, vec2<i32>(i32(step), 0), 0).r;
        if (flag == 0u) {
            continue;
        }
        // kind 0 is an unsigned integer container. Any other kind adds nothing.
        if (ray.kind != 0u || ray.edge == 0u) {
            continue;
        }
        let voxel = voxel_of(px, py, step);
        let bx = u32(voxel.x) / ray.edge;
        let by = u32(voxel.y) / ray.edge;
        let bz = u32(voxel.z) / ray.edge;
        let local = vec3<i32>(
            i32(u32(voxel.x) % ray.edge),
            i32(u32(voxel.y) % ray.edge),
            i32(u32(voxel.z) % ray.edge),
        );
        let slot = slot_for(vec3<u32>(bx, by, bz));
        if (slot < 0) {
            continue;
        }
        let stored = f32(load_brick(u32(slot), local));
        let display = lut_chain(stored);
        let span = voi.ymax - voi.ymin;
        let colour = select(0.0, (display - voi.ymin) / span, span > 0.0);
        let alpha = 0.5;
        accum_c = accum_c + (1.0 - accum_a) * colour * alpha;
        accum_a = accum_a + (1.0 - accum_a) * alpha;
    }
    return vec4<f32>(accum_c, accum_c, accum_c, accum_a);
}
