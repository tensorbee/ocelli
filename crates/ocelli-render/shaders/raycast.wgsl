// Volume composite ray. Appended to voi.wgsl and table_index.
// One step is one voxel. A residency flag of 0 skips the step before any
// brick is read. Each ready brick is its own texture_3d. The residency
// texture is the last binding in group 1. Opacity is a 256-entry uniform
// at group 0 binding 2. The address is table_index of the display value
// with last 255. Entries are packed as vec4 so the uniform stride is 16.
// Shade weights are a 16-byte uniform at group 0 binding 3. The default
// the host writes is ambient 1 and diffuse 0. The normal is the central
// difference of stored samples. A missing neighbour contributes 0. The
// light is the unit walk axis toward the near end, which is decreasing
// index. Shade is ambient when the gradient length is 0, otherwise
// ambient + diffuse * max(0, dot(n, light)). The shaded value is
// colour * shade. Transfer opacity is not multiplied by the shade.
// Clip planes are a 256-byte uniform at group 0 binding 4. The patient
// point uses the volume origin, spacing and direction. A sample is kept
// when dot(point - plane_origin, normal) >= 0, which includes the plane.
// A disabled slot has normal.w of 0. A discarded sample adds nothing.
// No storage buffer and no compute.
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
    spacing_x : f32,
    spacing_y : f32,
    spacing_z : f32,
    coords : array<vec4<u32>, __RAY_SLOTS__>,
}

@group(0) @binding(1) var<uniform> ray : RayParams;

struct OpacitySamples {
    values : array<vec4<f32>, 64>,
}

@group(0) @binding(2) var<uniform> opacity : OpacitySamples;

struct ShadeParams {
    ambient : f32,
    diffuse : f32,
    pad0 : f32,
    pad1 : f32,
}

@group(0) @binding(3) var<uniform> shade : ShadeParams;

struct CropSlot {
    origin : vec4<f32>,
    normal : vec4<f32>,
}

struct CropParams {
    origin : vec4<f32>,
    axis_x : vec4<f32>,
    axis_y : vec4<f32>,
    axis_z : vec4<f32>,
    planes : array<CropSlot, 6>,
}

@group(0) @binding(4) var<uniform> crop : CropParams;

fn opacity_of(index : i32) -> f32 {
    var lane = 0u;
    if (index > 0) {
        lane = u32(index);
    }
    if (lane > 255u) {
        lane = 255u;
    }
    let packed = opacity.values[lane / 4u];
    let component = lane % 4u;
    if (component == 0u) {
        return packed.x;
    }
    if (component == 1u) {
        return packed.y;
    }
    if (component == 2u) {
        return packed.z;
    }
    return packed.w;
}

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

// A neighbour outside the volume, or outside the centre sample's brick,
// contributes 0. The brick texture is read even when that step's residency
// flag is 0, because the flag skips the integral and does not erase stored
// samples.
fn neighbour_stored(voxel : vec3<i32>, slot : u32, home : vec3<u32>) -> f32 {
    if (voxel.x < 0 || voxel.y < 0 || voxel.z < 0) {
        return 0.0;
    }
    let x = u32(voxel.x);
    let y = u32(voxel.y);
    let z = u32(voxel.z);
    if (x >= ray.dim_x || y >= ray.dim_y || z >= ray.dim_z) {
        return 0.0;
    }
    if (ray.edge == 0u) {
        return 0.0;
    }
    let bx = x / ray.edge;
    let by = y / ray.edge;
    let bz = z / ray.edge;
    if (bx != home.x || by != home.y || bz != home.z) {
        return 0.0;
    }
    let local = vec3<i32>(i32(x % ray.edge), i32(y % ray.edge), i32(z % ray.edge));
    return f32(load_brick(slot, local));
}

fn stored_difference(voxel : vec3<i32>, axis : vec3<i32>, slot : u32, home : vec3<u32>, spacing : f32) -> f32 {
    if (!(spacing > 0.0)) {
        return 0.0;
    }
    let plus = neighbour_stored(voxel + axis, slot, home);
    let minus = neighbour_stored(voxel - axis, slot, home);
    return (plus - minus) / (2.0 * spacing);
}

fn light_toward_near() -> vec3<f32> {
    if (ray.axis == 0u) {
        return vec3<f32>(-1.0, 0.0, 0.0);
    }
    if (ray.axis == 1u) {
        return vec3<f32>(0.0, -1.0, 0.0);
    }
    return vec3<f32>(0.0, 0.0, -1.0);
}

fn shade_of(gradient : vec3<f32>) -> f32 {
    let ambient = shade.ambient;
    let diffuse = shade.diffuse;
    let gradient_len = length(gradient);
    if (gradient_len == 0.0) {
        return ambient;
    }
    let n = gradient / gradient_len;
    let light = light_toward_near();
    let facing = dot(n, light);
    return ambient + diffuse * max(0.0, facing);
}

fn patient_point(voxel : vec3<i32>) -> vec3<f32> {
    let index = vec3<f32>(voxel);
    return crop.origin.xyz
        + crop.axis_x.xyz * (ray.spacing_x * index.x)
        + crop.axis_y.xyz * (ray.spacing_y * index.y)
        + crop.axis_z.xyz * (ray.spacing_z * index.z);
}

fn sample_kept(point : vec3<f32>) -> bool {
    for (var i = 0u; i < 6u; i = i + 1u) {
        let normal = crop.planes[i].normal;
        if (normal.w == 0.0) {
            continue;
        }
        let delta = point - crop.planes[i].origin.xyz;
        if (!(dot(delta, normal.xyz) >= 0.0)) {
            return false;
        }
    }
    return true;
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
        if (!sample_kept(patient_point(voxel))) {
            continue;
        }
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
        let home = vec3<u32>(bx, by, bz);
        let gx = stored_difference(voxel, vec3<i32>(1, 0, 0), u32(slot), home, ray.spacing_x);
        let gy = stored_difference(voxel, vec3<i32>(0, 1, 0), u32(slot), home, ray.spacing_y);
        let gz = stored_difference(voxel, vec3<i32>(0, 0, 1), u32(slot), home, ray.spacing_z);
        let lit = colour * shade_of(vec3<f32>(gx, gy, gz));
        let alpha = opacity_of(table_index(display, 255u));
        accum_c = accum_c + (1.0 - accum_a) * lit * alpha;
        accum_a = accum_a + (1.0 - accum_a) * alpha;
    }
    return vec4<f32>(accum_c, accum_c, accum_c, accum_a);
}
