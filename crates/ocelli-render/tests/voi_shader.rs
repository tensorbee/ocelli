//! HLD section 18.4's shader, on a real adapter.
//!
//! **Every test here is `#[ignore]`d** and `bin/ocelli.sh gate gpu` runs them,
//! in the `--sprint` and `--all` profiles and never in CI, because deviation
//! D-04 leaves CI without an adapter.
//!
//! # What class of evidence this is
//!
//! Two kinds, and they are not the same strength.
//!
//! The **section 18.3 rows and the boundary rows are hand-computed from PS3.3
//! C.11.2.1.2 and C.11.2.1.3.2** and asserted against the GPU directly. Nothing
//! in this repository produced those numbers. They are the strong half, and
//! they are what stops the comparison below from being circular.
//!
//! The **sweep is a comparison against `ocelli-pixel`**, which is this
//! repository's own validated CPU path. It is **weaker than an oracle verdict**,
//! because both sides are ours and a shared misreading of PS3.3 would agree
//! with itself. It is **stronger than a screenshot**, because it is a numeric
//! diff at `f32` with no quantisation to eight bits hiding a 0.32-of-255
//! divergence. The oracle's verdict on a rendered frame arrives when there is a
//! renderer to render one. F-038 schedules a frame and acquires no surface.
//!
//! # The harness is tier A only and the shader is not
//!
//! `shaders/voi.wgsl` carries no entry point and uses nothing HLD section 7
//! denies tier B. This file appends a compute entry point over two storage
//! buffers, which tier B has neither of, because it needs to push arbitrary
//! `f32` inputs in and read exact `f32` values back. **That is a property of
//! the harness, not of the shader**, and F-038's fragment pass is what will
//! exercise the same text on a tier B path.
//!
//! Reading back is allowed. Decision D3 is that pixels never cross **the wasm
//! boundary**, and a native test mapping a buffer on the host is not that
//! boundary.

use ocelli_core::{Display, Stored};
use ocelli_pixel::{
    LutChain, ModalityTransform, PhotometricInterpretation, PresentationLutEvidence, VoiFunction,
    VoiTransform,
};
use ocelli_render::{
    GpuContext, MODALITY_FOLD, ModalityRoute, StoredRoute, TierRequest, VOI_WGSL, VoiParams,
    fill_modality_table, resolve_adapter,
};
use wgpu::util::DeviceExt;

/// The sweep's agreement bound, over a `[0, 255]` output range.
///
/// **This tolerance is written here for the first time and HLD section 25.1
/// does not cover it.** Section 25.1's bounds are for the oracle's eight-bit
/// frames against cornerstone3D. This is `f32` against `f32` on one machine.
/// Section 25.1's own last bullet governs the act: "A tolerance change is a
/// pull request with a rationale, reviewed like code."
///
/// `1e-4` of 255 is 4e-7 of full scale, three and a half orders of magnitude
/// tighter than the 0.32-of-255 divergence this story exists to detect.
///
/// **Bit-exactness is not claimed**, which is decision D14: claim MEASURED
/// divergence, never bit-exact reproducibility. WGSL's `exp` is not required to
/// be correctly rounded and the CPU's comes from glam's libm backend, so
/// SIGMOID can legitimately differ in the last places on some hardware.
const SWEEP_TOLERANCE: f32 = 1e-4;

/// The section 18.3 rows are asserted at the same `0.001`
/// `crates/ocelli-pixel/tests/voi.rs` already uses for the same four numbers,
/// so the two suites state one thing about one set of values rather than two.
const FIXTURE_TOLERANCE: f32 = 0.001;

/// The compute harness appended to `VOI_WGSL`. See the module header.
///
/// The storage bindings sit at 3 and 4, past the route uniform and the
/// folded table that `voi.wgsl` reserves at bindings 1 and 2. `cs_main`
/// runs the whole chain. `cs_stage_one` runs stage 1 alone, which is what
/// the F-227 pair fixtures read bits from.
const HARNESS_WGSL: &str = r"
@group(0) @binding(3) var<storage, read> inputs : array<f32>;
@group(0) @binding(4) var<storage, read_write> outputs : array<f32>;

@compute @workgroup_size(64)
fn cs_main(@builtin(global_invocation_id) gid : vec3<u32>) {
    let i = gid.x;
    if (i >= arrayLength(&inputs)) { return; }
    outputs[i] = lut_chain(inputs[i]);
}

@compute @workgroup_size(64)
fn cs_stage_one(@builtin(global_invocation_id) gid : vec3<u32>) {
    let i = gid.x;
    if (i >= arrayLength(&inputs)) { return; }
    outputs[i] = modality_stage_one(inputs[i]);
}
";

fn native_clock() -> impl FnMut() -> u64 {
    let start = std::time::Instant::now();
    move || u64::try_from(start.elapsed().as_nanos()).unwrap_or(u64::MAX)
}

/// A context, or `None` on a machine that resolves tier C.
fn context() -> Option<GpuContext> {
    let mut clock = native_clock();
    let adapter = pollster::block_on(resolve_adapter(TierRequest::Auto, &mut clock))?;
    let opened = pollster::block_on(adapter.open());
    assert!(
        opened.is_ok(),
        "an adapter resolved and no device opened: {opened:?}"
    );
    opened.ok()
}

/// Build the chain the fixtures are written against.
///
/// Rescale is the identity here, deliberately: HLD section 18.3's table is in
/// Hounsfield units and its inputs ARE the modality values, so a non-identity
/// rescale would make the transcribed numbers apply to different inputs.
fn chain(function: VoiFunction, photometric: PhotometricInterpretation) -> LutChain {
    let modality = ModalityTransform::new(None, Some(1.0), Some(0.0));
    let voi = VoiTransform::new(None, &[40.0], &[400.0], 0, function, 0.0, 255.0);
    assert!(modality.is_ok() && voi.is_ok());
    let (Ok(modality), Ok(voi)) = (modality, voi) else {
        unreachable!("asserted immediately above")
    };
    let built = LutChain::new(modality, voi, photometric, PresentationLutEvidence::Absent);
    assert!(built.is_ok());
    match built {
        Ok(built) => built,
        Err(_) => unreachable!("asserted immediately above"),
    }
}

/// A route uniform and the folded table it names, bound beside the harness.
struct RouteFixture {
    route: ModalityRoute,
    table: Vec<f32>,
}

/// The flag-0 route: the texture already holds modality values, so stage 1
/// returns its input. The 1x1 zero table is the stand-in nothing reads.
fn identity_route() -> RouteFixture {
    RouteFixture {
        route: ModalityRoute {
            tabled: 0,
            first: 0,
            fold: MODALITY_FOLD,
            pad: 0,
        },
        table: vec![0.0],
    }
}

/// The flag-1 route over a whole container, the table filled by
/// `fill_modality_table` from the pair the fixture names.
fn tabled_route(first: i32, count: u32, slope: f32, intercept: f32) -> RouteFixture {
    let height = usize::try_from(count.div_ceil(MODALITY_FOLD)).unwrap_or(1);
    let width = usize::try_from(MODALITY_FOLD).unwrap_or(1);
    let mut table = vec![0.0_f32; width.saturating_mul(height)];
    let filled = fill_modality_table(
        StoredRoute::Tabled { first, count },
        slope,
        intercept,
        &mut table,
    );
    assert!(
        filled.is_ok(),
        "the fixture declares a legal container: {filled:?}"
    );
    RouteFixture {
        route: ModalityRoute {
            tabled: 1,
            first,
            fold: MODALITY_FOLD,
            pad: 0,
        },
        table,
    }
}

/// The f32 at one pinned bit pattern, the way the fixtures name inputs.
fn from_bits(bits: u32) -> f32 {
    f32::from_bits(bits)
}

/// Run `modality_stage_one` on the GPU for every input, under one route and
/// uniform. Stage 1 reads neither the window nor the rescale, so the uniform
/// only matters for what a wrong branch would have read.
fn stage_one_on_gpu(
    context: &GpuContext,
    params: VoiParams,
    fixture: &RouteFixture,
    inputs: &[f32],
) -> Vec<f32> {
    dispatch_routed(context, params, fixture, inputs, "cs_stage_one")
}

/// Run `lut_chain` on the GPU for every input, under one route and uniform.
fn on_gpu_routed(
    context: &GpuContext,
    params: VoiParams,
    fixture: &RouteFixture,
    inputs: &[f32],
) -> Vec<f32> {
    dispatch_routed(context, params, fixture, inputs, "cs_main")
}

/// Run `lut_chain` on the GPU for every input, under one `VoiParams`.
///
/// One `queue.submit`, which is HLD section 22's rule, and one pipeline
/// compiled before anything is dispatched, which is its "pipelines compile at
/// init, never mid-frame".
fn on_gpu(context: &GpuContext, params: VoiParams, inputs: &[f32]) -> Vec<f32> {
    on_gpu_routed(context, params, &identity_route(), inputs)
}

fn dispatch_routed(
    context: &GpuContext,
    params: VoiParams,
    fixture: &RouteFixture,
    inputs: &[f32],
    entry: &str,
) -> Vec<f32> {
    let device = context.device();
    let queue = context.queue();

    let source = format!("{VOI_WGSL}{HARNESS_WGSL}");
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("ocelli voi"),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    });
    // An explicit layout, because a derived one only carries the bindings
    // the dispatched entry point reads, and the stage-1 probe does not read
    // the section 18.4 uniform.
    let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("ocelli voi harness"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 3,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: true },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 4,
                visibility: wgpu::ShaderStages::COMPUTE,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Storage { read_only: false },
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            },
        ],
    });
    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("ocelli voi harness"),
        bind_group_layouts: &[Some(&layout)],
        immediate_size: 0,
    });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("ocelli voi"),
        layout: Some(&pipeline_layout),
        module: &module,
        entry_point: Some(entry),
        compilation_options: wgpu::PipelineCompilationOptions::default(),
        cache: None,
    });

    // Thirty-two bytes, written with bytemuck rather than a transmute.
    let uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("ocelli voi params"),
        contents: bytemuck::bytes_of(&params),
        usage: wgpu::BufferUsages::UNIFORM,
    });
    assert_eq!(uniform.size(), 32, "section 18.4's uniform is not 32 bytes");
    let route_uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("ocelli voi route"),
        contents: bytemuck::bytes_of(&fixture.route),
        usage: wgpu::BufferUsages::UNIFORM,
    });
    assert_eq!(
        route_uniform.size(),
        16,
        "the route uniform is not 16 bytes"
    );

    let fold = usize::try_from(fixture.route.fold).unwrap_or(1);
    let (width, height) = if fixture.route.tabled != 0 {
        (fold, fixture.table.len() / fold.max(1))
    } else {
        (1, 1)
    };
    let table_texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("ocelli voi table"),
        size: wgpu::Extent3d {
            width: u32::try_from(width).unwrap_or(1),
            height: u32::try_from(height).unwrap_or(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::R32Float,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let mut staged_table = fixture.table.clone();
    staged_table.resize(width.saturating_mul(height).max(1), 0.0);
    // A copy row is the row width rounded up to the 256-byte alignment, and
    // never below the row's own bytes: 256 texels of R32Float is 1024.
    let row_bytes = u32::try_from(width)
        .unwrap_or(0)
        .saturating_mul(4)
        .next_multiple_of(256)
        .max(256);
    let row_bytes_us = usize::try_from(row_bytes).unwrap_or(0);
    let mut copy = vec![0_u8; row_bytes_us.saturating_mul(height.max(1))];
    for (index, value) in staged_table.iter().copied().enumerate() {
        let at = index.saturating_mul(4);
        let Some(slot) = copy.get_mut(at..at.saturating_add(4)) else {
            break;
        };
        slot.copy_from_slice(&value.to_le_bytes());
    }
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &table_texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &copy,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(row_bytes),
            rows_per_image: Some(u32::try_from(height).unwrap_or(1)),
        },
        wgpu::Extent3d {
            width: u32::try_from(width).unwrap_or(1),
            height: u32::try_from(height).unwrap_or(1),
            depth_or_array_layers: 1,
        },
    );
    let table_view = table_texture.create_view(&wgpu::TextureViewDescriptor::default());

    let input = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("ocelli voi inputs"),
        contents: bytemuck::cast_slice(inputs),
        usage: wgpu::BufferUsages::STORAGE,
    });
    let span = u64::try_from(std::mem::size_of_val(inputs)).unwrap_or(0);
    let output = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("ocelli voi outputs"),
        size: span,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("ocelli voi readback"),
        size: span,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("ocelli voi"),
        layout: &layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: route_uniform.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::TextureView(&table_view),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: input.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: output.as_entire_binding(),
            },
        ],
    });

    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("ocelli voi"),
    });
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("ocelli voi"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bind, &[]);
        let groups = u32::try_from(inputs.len().div_ceil(64)).unwrap_or(1);
        pass.dispatch_workgroups(groups, 1, 1);
    }
    encoder.copy_buffer_to_buffer(&output, 0, &readback, 0, span);
    queue.submit(std::iter::once(encoder.finish()));

    let slice = readback.slice(..);
    slice.map_async(wgpu::MapMode::Read, |_| {});
    let polled = device.poll(wgpu::PollType::Wait {
        submission_index: None,
        timeout: None,
    });
    assert!(polled.is_ok(), "the device would not complete the dispatch");

    // `get_mapped_range` is fallible in the pinned wgpu, so the map is checked
    // rather than assumed. A silently empty readback would make every
    // comparison below vacuous.
    let view = slice.get_mapped_range();
    assert!(view.is_ok(), "the readback buffer did not map");
    let Ok(view) = view else { return Vec::new() };
    let values: Vec<f32> = bytemuck::cast_slice(&view).to_vec();
    drop(view);
    readback.unmap();
    assert_eq!(
        values.len(),
        inputs.len(),
        "the readback returned a different number of values than were dispatched"
    );
    values
}

fn params_of(chain: &LutChain) -> VoiParams {
    let built = VoiParams::from_chain(chain);
    assert!(built.is_ok(), "the fixture chain has no uniform: {built:?}");
    match built {
        Ok(built) => built,
        Err(_) => unreachable!("asserted immediately above"),
    }
}

/// One value out of a GPU readback, refusing rather than indexing.
///
/// `indexing_slicing` is denied at the workspace, and the reason applies here
/// rather than only in shipped code: a readback that came back short would
/// otherwise panic with an index message instead of saying the dispatch
/// returned the wrong number of values.
#[track_caller]
fn value(values: &[f32], index: usize) -> f32 {
    let got = values.get(index).copied();
    assert!(got.is_some(), "the GPU returned no value at index {index}");
    got.unwrap_or(f32::NAN)
}

#[track_caller]
fn assert_close(actual: f32, expected: f32, tolerance: f32, what: &str) {
    assert!(
        (actual - expected).abs() < tolerance,
        "{what}: was {actual}, wanted {expected}"
    );
}

/// **HLD section 18.3's four rows, on the GPU, hand-computed.**
///
/// Soft-tissue CT window, centre 40, width 400, output range 0 to 255.
///
/// | Input (HU) | LINEAR | LINEAR_EXACT |
/// |---|---|---|
/// | -160 | 0.000 | 0.000 |
/// | 40 | 127.819 | 127.500 |
/// | 240 | 255.000 | 255.000 |
/// | -60 | 63.910 | 63.750 |
///
/// **Row one's `LINEAR_EXACT` value is `0.000`, not section 18.3's `1.594`,
/// under declared deviation D-13.** Section 18.2 clamps at `x <= c - w/2`,
/// which is -160, and -160 is not greater than -160. The formula body also
/// evaluates to zero there. Section 18.2 is the formula and section 18.3 is a
/// worked value, and the formula is the specification.
///
/// These are the values `crates/ocelli-pixel/tests/voi.rs` already asserts on
/// the CPU, computed independently from PS3.3 rather than copied from it.
#[test]
#[ignore = "needs a real GPU adapter, which the CI floor does not have (D-04)"]
fn hld_section_18_3_rows_on_the_gpu_apply_d_13() {
    let Some(context) = context() else {
        println!("this machine resolved tier C, so there is no device");
        return;
    };
    let inputs = [-160.0_f32, 40.0, 240.0, -60.0];

    let linear = on_gpu(
        &context,
        params_of(&chain(
            VoiFunction::Linear,
            PhotometricInterpretation::Monochrome2,
        )),
        &inputs,
    );
    let exact = on_gpu(
        &context,
        params_of(&chain(
            VoiFunction::LinearExact,
            PhotometricInterpretation::Monochrome2,
        )),
        &inputs,
    );

    for (index, (wanted_linear, wanted_exact)) in [
        (0.0_f32, 0.0_f32),
        (127.819_55, 127.5),
        (255.0, 255.0),
        (63.909_775, 63.75),
    ]
    .into_iter()
    .enumerate()
    {
        assert_close(
            value(&linear, index),
            wanted_linear,
            FIXTURE_TOLERANCE,
            "LINEAR row",
        );
        assert_close(
            value(&exact, index),
            wanted_exact,
            FIXTURE_TOLERANCE,
            "LINEAR_EXACT row",
        );
    }

    // READ THIS ROW, section 18.3: at the window centre the two functions
    // differ by 0.32 of 255. If the shader used one formula for both, this is
    // zero and every row above still passes.
    let divergence = value(&linear, 1) - value(&exact, 1);
    assert_close(
        divergence,
        0.319_549,
        FIXTURE_TOLERANCE,
        "the LINEAR against LINEAR_EXACT divergence at the window centre",
    );
}

/// **The boundary VALUES, and the fact that the two functions clamp at
/// different inputs.**
///
/// **This test pins neither comparison operator**, and its name used to imply
/// it did. All four operator mutations leave it green. What it does pin is the
/// values either side of each bound and the LINEAR against LINEAR_EXACT split,
/// and it is red when LINEAR loses its `c - 0.5` and `w - 1`.
///
/// PS3.3 C.11.2.1.2 and C.11.2.1.3.2: the lower bound is `<=` and the upper
/// bound is `>`. LINEAR's upper bound is `c' + w'/2 = 239` and LINEAR_EXACT's
/// is `c + w/2 = 240`, which is the asymmetry section 18.3's row one was
/// reaching for and does not have, because both lower bounds coincide at -160.
/// At 239 itself LINEAR does not clamp: the comparison is `>`, so the body runs
/// and returns `ymax` by continuity. What differs at 239 is that LINEAR has
/// reached `ymax` and LINEAR_EXACT has not.
///
/// Every value is hand-computed as an exact rational and written as the
/// expression that produces it, the same way `crates/ocelli-pixel/tests/voi.rs`
/// writes them.
#[test]
#[ignore = "needs a real GPU adapter, which the CI floor does not have (D-04)"]
fn the_voi_boundary_values_and_the_two_upper_bounds_differ_on_the_gpu() {
    let Some(context) = context() else {
        println!("this machine resolved tier C, so there is no device");
        return;
    };
    let inputs = [-160.0_f32, -159.0, 238.0, 239.0, 240.0];

    let linear = on_gpu(
        &context,
        params_of(&chain(
            VoiFunction::Linear,
            PhotometricInterpretation::Monochrome2,
        )),
        &inputs,
    );
    assert_close(
        value(&linear, 0),
        0.0,
        FIXTURE_TOLERANCE,
        "LINEAR at -160 clamps to ymin",
    );
    assert_close(
        value(&linear, 1),
        255.0 / 399.0,
        FIXTURE_TOLERANCE,
        "LINEAR at -159",
    );
    assert_close(
        value(&linear, 2),
        255.0 * 398.0 / 399.0,
        FIXTURE_TOLERANCE,
        "LINEAR at 238",
    );
    assert_close(
        value(&linear, 3),
        255.0,
        FIXTURE_TOLERANCE,
        "LINEAR at 239 reaches ymax",
    );

    let exact = on_gpu(
        &context,
        params_of(&chain(
            VoiFunction::LinearExact,
            PhotometricInterpretation::Monochrome2,
        )),
        &inputs,
    );
    assert_close(
        value(&exact, 0),
        0.0,
        FIXTURE_TOLERANCE,
        "LINEAR_EXACT at -160",
    );
    assert_close(
        value(&exact, 1),
        255.0 / 400.0,
        FIXTURE_TOLERANCE,
        "LINEAR_EXACT at -159",
    );
    assert_close(
        value(&exact, 3),
        255.0 * 399.0 / 400.0,
        FIXTURE_TOLERANCE,
        "LINEAR_EXACT at 239 does NOT clamp",
    );
    assert_close(
        value(&exact, 4),
        255.0,
        FIXTURE_TOLERANCE,
        "LINEAR_EXACT at 240",
    );

    // The asymmetry itself: at 239 one has clamped and the other has not.
    assert!(
        value(&linear, 3) > value(&exact, 3),
        "the two upper bounds coincide, so one of the clamps is at the wrong input"
    );
}

/// **SIGMOID at -60, where PS3.3 C.11.2.1.3.1's exponent is exactly +1.**
///
/// `y = (ymax - ymin) / (1 + exp(-4 * (x - c) / w)) + ymin`, with x = -60,
/// c = 40, w = 400: `-4 * (-100) / 400 = 1`, so `y = 255 / (1 + e)`.
///
/// Computed from `core::f32::consts::E` rather than from either implementation,
/// which is what makes it independent of the CPU's glam-libm `exp` as well as
/// of the GPU's.
#[test]
#[ignore = "needs a real GPU adapter, which the CI floor does not have (D-04)"]
fn sigmoid_at_minus_sixty_is_255_over_one_plus_e_on_the_gpu() {
    let Some(context) = context() else {
        println!("this machine resolved tier C, so there is no device");
        return;
    };
    let got = on_gpu(
        &context,
        params_of(&chain(
            VoiFunction::Sigmoid,
            PhotometricInterpretation::Monochrome2,
        )),
        &[-60.0],
    );
    assert_close(
        value(&got, 0),
        255.0 / (1.0 + core::f32::consts::E),
        FIXTURE_TOLERANCE,
        "SIGMOID at -60",
    );
}

/// **Inversion is a reflection about the midpoint of the output range.**
///
/// PS3.3 C.11.6, `ymin + ymax - y`. Over a range of `[16, 235]` a value mapping
/// to 100 inverts to 151. `ymax - y` gives 135 and `1 - y` is nonsense, so this
/// row separates the reflection from both wrong forms, which a `[0, 255]` range
/// cannot: there `ymax - y` and `ymin + ymax - y` are the same expression.
#[test]
#[ignore = "needs a real GPU adapter, which the CI floor does not have (D-04)"]
fn inversion_reflects_about_the_midpoint_of_a_non_zero_range() {
    let Some(context) = context() else {
        println!("this machine resolved tier C, so there is no device");
        return;
    };
    // Centre 100, width 200, range [16, 235]. At x = 100 the LINEAR_EXACT
    // formula gives ((100 - 100)/200 + 0.5) * 219 + 16 = 125.5, and inverted
    // that is 16 + 235 - 125.5 = 125.5, which is the midpoint and cannot
    // separate anything. So the input is 150, where the value is
    // ((150-100)/200 + 0.5) * 219 + 16 = 180.25 and the reflection is 70.75.
    let modality = ModalityTransform::new(None, Some(1.0), Some(0.0));
    let voi = VoiTransform::new(
        None,
        &[100.0],
        &[200.0],
        0,
        VoiFunction::LinearExact,
        16.0,
        235.0,
    );
    assert!(modality.is_ok() && voi.is_ok());
    let (Ok(modality), Ok(voi)) = (modality, voi) else {
        return;
    };
    let inverted = LutChain::new(
        modality,
        voi,
        PhotometricInterpretation::Monochrome1,
        PresentationLutEvidence::Absent,
    );
    assert!(inverted.is_ok());
    let Ok(inverted) = inverted else { return };

    let got = on_gpu(&context, params_of(&inverted), &[150.0]);
    assert_close(value(&got, 0), 70.75, FIXTURE_TOLERANCE, "inverted value");
    assert!(
        (value(&got, 0) - (235.0 - 180.25)).abs() > 1.0,
        "the shader used ymax - y, which is correct only when ymin is zero"
    );
}

/// **Agreement with `LutChain::map_into` over a sweep, all three functions,
/// inverted and not.**
///
/// 4096 stored values from -1200 to 3000, which spans both clamps of the
/// soft-tissue window and the whole informative region between them. This is
/// the comparison against this repository's own CPU path, and the module header
/// says what class of evidence that is.
///
/// The measured maximum divergence is printed, because decision D14 is to claim
/// MEASURED divergence rather than bit-exactness, and a bound that passes with
/// nothing reported is a bound nobody can calibrate.
#[test]
#[ignore = "needs a real GPU adapter, which the CI floor does not have (D-04)"]
fn the_shader_agrees_with_ocelli_pixel_over_a_sweep() {
    let Some(context) = context() else {
        println!("this machine resolved tier C, so there is no device");
        return;
    };

    let inputs: Vec<f32> = (0..4096)
        .map(|index| {
            let t = f32::from(u16::try_from(index).unwrap_or(0)) / 4095.0;
            (t - 1200.0 / 4200.0) * 4200.0
        })
        .collect();

    let mut worst = 0.0_f32;
    for function in [
        VoiFunction::Linear,
        VoiFunction::LinearExact,
        VoiFunction::Sigmoid,
    ] {
        for photometric in [
            PhotometricInterpretation::Monochrome2,
            PhotometricInterpretation::Monochrome1,
        ] {
            let built = chain(function, photometric);
            let params = params_of(&built);
            let on_device = on_gpu(&context, params, &inputs);

            let stored: Vec<Stored> = inputs.iter().map(|value| Stored(*value)).collect();
            let mut on_cpu = vec![Display(0.0); stored.len()];
            let mapped = built.map_into(&stored, &mut on_cpu);
            assert!(mapped.is_ok());

            for (index, cpu) in on_cpu.iter().enumerate() {
                // **The output stays inside the declared range**, which is what
                // the clamps exist for and what nothing else here asserted.
                //
                // Scoped to THIS sweep's parameters rather than claimed as an
                // invariant. The F-041 review's second pass found legal windows
                // where the LINEAR body overshoots `ymax` at the breakpoint
                // through `f32` rounding, by 42.5 of 255 in the worst case it
                // built. That is a property of the specification's arithmetic
                // in single precision, the CPU reproduces it exactly, and it is
                // not this story's to change. Here the range is `[0, 255]` with
                // a width of 400, where it does not arise.
                let got = value(&on_device, index);
                // The bound is read from the uniform the shader was given
                // rather than written as a literal, so it follows the chain if
                // a future row uses a different range.
                assert!(
                    (params.ymin..=params.ymax).contains(&got),
                    "{function:?} {photometric:?} at stored {}: gpu {got} left the \
                     declared output range of {} to {}",
                    value(&inputs, index),
                    params.ymin,
                    params.ymax
                );
                let difference = (got - cpu.0).abs();
                worst = worst.max(difference);
                assert!(
                    difference < SWEEP_TOLERANCE,
                    "{function:?} {photometric:?} at stored {}: gpu {} cpu {}, \
                     difference {difference} exceeds {SWEEP_TOLERANCE}",
                    value(&inputs, index),
                    value(&on_device, index),
                    cpu.0
                );
            }
        }
    }
    println!("measured maximum divergence over the sweep: {worst}");
}

/// **A window-level change moves only the thirty-two-byte uniform.**
///
/// HLD section 26: "Prefer a uniform update to a texture update. Window/level
/// is thirty-two bytes, not a re-upload."
///
/// **What this asserts is the first half only.** Two windows over the same
/// input give different outputs, and the whole difference between the two runs
/// is a `VoiParams`, whose size is asserted here and whose 32 bytes are
/// asserted field by field in `tests/voi_params.rs`. It does **not** assert
/// that no texture is created, and an earlier version of this comment said it
/// did. `on_gpu` is a test harness that rebuilds its module, pipeline and
/// buffers on every call, so there is nothing here to measure a re-upload
/// against. The claim that a drag re-uploads nothing belongs to F-038's render
/// graph, which is the first code with a frame to not re-upload during.
///
/// **The narrow window is built through `VoiTransform::new`**, not with struct
/// update syntax on `VoiParams`. `VoiParams`'s fields are public, so
/// `VoiParams { width: 0.0, ..soft }` compiles and would hand the shader a
/// width PS3.3 C.11.2.1.2 forbids. Under LINEAR that produces a clean two-tone
/// threshold image rather than a NaN, which is the hazard `VoiParams` documents
/// and is worse than a loud failure. A test that reached the GPU that way would
/// be demonstrating the bypass while claiming to demonstrate a drag.
#[test]
#[ignore = "needs a real GPU adapter, which the CI floor does not have (D-04)"]
fn changing_the_window_changes_the_output_through_thirty_two_bytes() {
    let Some(context) = context() else {
        println!("this machine resolved tier C, so there is no device");
        return;
    };
    let inputs = [40.0_f32];

    let soft = params_of(&chain(
        VoiFunction::Linear,
        PhotometricInterpretation::Monochrome2,
    ));

    let modality = ModalityTransform::new(None, Some(1.0), Some(0.0));
    let narrow_voi = VoiTransform::new(None, &[40.0], &[80.0], 0, VoiFunction::Linear, 0.0, 255.0);
    assert!(modality.is_ok() && narrow_voi.is_ok());
    let (Ok(modality), Ok(narrow_voi)) = (modality, narrow_voi) else {
        return;
    };
    let narrow_chain = LutChain::new(
        modality,
        narrow_voi,
        PhotometricInterpretation::Monochrome2,
        PresentationLutEvidence::Absent,
    );
    assert!(narrow_chain.is_ok());
    let Ok(narrow_chain) = narrow_chain else {
        return;
    };
    let narrow = params_of(&narrow_chain);

    // The two uniforms differ in `width` and in nothing else, which is what a
    // drag writes.
    assert_eq!(
        VoiParams {
            width: soft.width,
            ..narrow
        },
        soft
    );

    let before = on_gpu(&context, soft, &inputs);
    let after = on_gpu(&context, narrow, &inputs);
    assert!(
        (value(&before, 0) - value(&after, 0)).abs() > FIXTURE_TOLERANCE,
        "narrowing the window through the uniform changed nothing"
    );
    assert_eq!(core::mem::size_of::<VoiParams>(), 32);
}

/// **A window of width one, where `<` instead of `<=` returns NaN.**
///
/// It is NOT the only width at which the lower operator is observable, and an
/// earlier version of this heading said "the only input". The F-041 review's
/// fourth pass measured three legal chains with `w` near but not equal to 1
/// where `fl(c' - w'/2) - c'` is not exactly `-w'/2`, so the body at the
/// computed breakpoint is not `ymin`. What `w = 1` is uniquely is the width
/// where the divergence is a NaN, because `w' = 0` only there.
///
/// **No rate is quoted for how common that is.** One was, 83 per cent, with no
/// sampling frame. `docs/lld/pixel-pipeline.md` records why a rate without its
/// frame is refused here. Measured over several frames, the same quantity
/// ranges from 6.0 to 99.9 per cent, so it describes the sampler rather than
/// the code. The counterexamples establish "not the only width" on their own
/// and need no population behind them.
///
/// PS3.3 C.11.2.1.2 requires `w >= 1`, so `w = 1` is legal and
/// `VoiTransform::new` admits it. There `w' = w - 1 = 0`, both breakpoints
/// collapse onto `c'`, and the body is `0 / 0`.
///
/// With the operators as the standard writes them, `x <= c'` returns `ymin` and
/// `x > c'` returns `ymax`, and the body is unreachable. Change `<=` to `<` and
/// `x == c'` falls through to the division and the shader returns **NaN** where
/// it must return `ymin`.
///
/// Centre 0, width 1, so `c' = -0.5`. The two inputs are the two sides of that
/// breakpoint and they are the rows
/// `crates/ocelli-pixel/tests/voi.rs::voi_bounds_use_lower_less_equal_and_upper_strict_greater`
/// already pins on the CPU.
///
/// **Every other GPU test here uses a width of 400, 200 or 80**, where the body
/// is continuous at the breakpoints and the mutation is invisible. The F-041
/// review's first pass measured that, and this test is what closes it.
///
/// **It pins the lower operator and no test pins the upper one**, which is why
/// the name says `lower`. No claim is made here about WHY, because three
/// attempts at that claim were each falsified. The shader records that the
/// argument was deleted rather than corrected a fourth time.
#[test]
#[ignore = "needs a real GPU adapter, which the CI floor does not have (D-04)"]
fn voi_linear_at_width_one_pins_the_lower_boundary_operator() {
    let Some(context) = context() else {
        println!("this machine resolved tier C, so there is no device");
        return;
    };
    let modality = ModalityTransform::new(None, Some(1.0), Some(0.0));
    let voi = VoiTransform::new(None, &[0.0], &[1.0], 0, VoiFunction::Linear, 0.0, 255.0);
    assert!(modality.is_ok() && voi.is_ok());
    let (Ok(modality), Ok(voi)) = (modality, voi) else {
        return;
    };
    let built = LutChain::new(
        modality,
        voi,
        PhotometricInterpretation::Monochrome2,
        PresentationLutEvidence::Absent,
    );
    assert!(built.is_ok());
    let Ok(built) = built else { return };

    let got = on_gpu(&context, params_of(&built), &[-0.5_f32, -0.499]);

    // The lower bound is `<=`, so the breakpoint itself clamps to ymin.
    assert!(
        value(&got, 0).is_finite(),
        "the shader divided by a zero w', so the lower comparison is not `<=`"
    );
    assert_close(
        value(&got, 0),
        0.0,
        FIXTURE_TOLERANCE,
        "LINEAR at c' with width one",
    );
    // The upper bound is `>`, so one ULP above the breakpoint clamps to ymax.
    assert_close(
        value(&got, 1),
        255.0,
        FIXTURE_TOLERANCE,
        "LINEAR just above c' with width one",
    );
}

/// **Stage 1 is executed through the folded table, and no other identity
/// chain here executes it.**
///
/// Every other chain in this file uses rescale slope 1 and intercept 0, so
/// deleting the table route and passing the stored value straight through
/// leaves them all green. The F-041 review's first pass measured that.
///
/// PS3.3 C.11.1 and HLD section 18.1: `modality = stored * slope + intercept`.
/// Slope 2 and intercept -1024 are both non-identity and distinguishable from
/// each other, so dropping either one alone is caught: with the slope dropped
/// the modality value is 1082 rather than 3188, and with the intercept dropped
/// it is 4212. The harness fills the folded table from this test's own chain,
/// and entry 2106 is `modality(2106) = 3188.0` exactly.
///
/// The window is centred on the expected modality value, so the expected
/// display value is the window centre under `LINEAR_EXACT`, which is
/// `(ymax - ymin) / 2 + ymin`, and here `ymin` is zero. Hand-computed: stored
/// 2106, modality `2106 * 2 - 1024 = 3188`, centre 3188 width 400, so
/// `((3188 - 3188) / 400 + 0.5) * 255 = 127.5`.
#[test]
#[ignore = "needs a real GPU adapter, which the CI floor does not have (D-04)"]
fn stage_one_applies_the_rescale_slope_and_intercept_on_the_gpu() {
    let Some(context) = context() else {
        println!("this machine resolved tier C, so there is no device");
        return;
    };
    let modality = ModalityTransform::new(None, Some(2.0), Some(-1024.0));
    let voi = VoiTransform::new(
        None,
        &[3188.0],
        &[400.0],
        0,
        VoiFunction::LinearExact,
        0.0,
        255.0,
    );
    assert!(modality.is_ok() && voi.is_ok());
    let (Ok(modality), Ok(voi)) = (modality, voi) else {
        return;
    };
    let built = LutChain::new(
        modality,
        voi,
        PhotometricInterpretation::Monochrome2,
        PresentationLutEvidence::Absent,
    );
    assert!(built.is_ok());
    let Ok(built) = built else { return };

    let fixture = tabled_route(0, 65_536, 2.0, -1024.0);
    let staged = stage_one_on_gpu(&context, params_of(&built), &fixture, &[2106.0]);
    assert_eq!(
        value(&staged, 0).to_bits(),
        3188.0_f32.to_bits(),
        "the table entry for 2106 is not modality's exact 3188.0"
    );
    let got = on_gpu_routed(&context, params_of(&built), &fixture, &[2106.0]);
    assert_close(
        value(&got, 0),
        127.5,
        FIXTURE_TOLERANCE,
        "stage 1 did not apply slope and intercept",
    );
}

/// **`VOI_WGSL` compiles into a real render pipeline, which is stronger than a
/// text grep.**
///
/// HLD section 7 makes tier B "fragment shaders only, no compute, no storage
/// buffers". `voi::tests` asserts that by searching the text for `@compute` and
/// `var<storage`, which proves the file does not SAY those words. This proves
/// the composed module is accepted as a vertex and fragment pair, which is the
/// shape tier B can actually run.
///
/// **It still does not prove tier B**, and that is worth stating rather than
/// letting a green pipeline imply it. This runs on whatever tier the machine
/// resolved, which for every machine this project has is tier A, and a downlevel
/// adapter can refuse a module a tier A adapter accepts. F-042 is the WebGL2
/// story and F-X002 is the one that gets a software adapter into a test.
#[test]
#[ignore = "needs a real GPU adapter, which the CI floor does not have (D-04)"]
fn the_shader_composes_into_a_render_pipeline_not_only_a_compute_one() {
    let Some(context) = context() else {
        println!("this machine resolved tier C, so there is no device");
        return;
    };
    let device = context.device();

    // A vertex and fragment pair over the same text the compute harness uses,
    // with the stored value carried in a varying rather than a storage buffer.
    let fragment_harness = r"
@vertex
fn vs_main(@builtin(vertex_index) index : u32) -> @builtin(position) vec4<f32> {
    // The standard oversized triangle: (-1,-1), (3,-1), (-1,3) in clip space,
    // which covers the viewport with one primitive and no vertex buffer.
    // Written correctly because F-038's render graph is likely to copy it.
    let u = f32((index << 1u) & 2u);
    let v = f32(index & 2u);
    return vec4<f32>(u * 2.0 - 1.0, v * 2.0 - 1.0, 0.0, 1.0);
}

@fragment
fn fs_main(@builtin(position) at : vec4<f32>) -> @location(0) vec4<f32> {
    let display = lut_chain(at.x);
    let unit = (display - voi.ymin) / max(voi.ymax - voi.ymin, 1.0);
    return vec4<f32>(unit, unit, unit, 1.0);
}
";
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("ocelli voi fragment"),
        source: wgpu::ShaderSource::Wgsl(format!("{VOI_WGSL}{fragment_harness}").into()),
    });
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("ocelli voi fragment"),
        layout: None,
        vertex: wgpu::VertexState {
            module: &module,
            entry_point: Some("vs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[],
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: &module,
            entry_point: Some("fs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: wgpu::TextureFormat::Rgba8Unorm,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    });
    // The pipeline exists, which is the assertion: `create_render_pipeline`
    // validates the composed module and a validation failure reaches wgpu's
    // default uncaptured-error handler, which panics. No error scope is pushed
    // here and none is needed for that. Asking for the derived bind group
    // layout confirms NOTHING further, measured in the F-041 review's second
    // pass, so it is not asked for. What watches the binding set is
    // `voi::tests::the_shader_is_composable_and_reserves_bindings_zero_one_and_two`,
    // which reads the text and runs in the floor.
    drop(pipeline);
}

/// **AC1, the F-227 stage-1 pair through the folded table, pinned as bits.**
///
/// Stored 2371.0 and 2372.0, slope `0x3571C0F8`, intercept `0x44E7C2FC`,
/// route `Tabled { first: 0, count: 65_536 }`. The stage-1 probe returns
/// exactly `0x44E7C30D` and `0x44E7C30E`, `ocelli_pixel::modality`'s own
/// two-rounding bits: PS3.3 C.11.1's output units are m*SV + b, applied as
/// multiply then add. Hand-computed with struct pack round-trips: 2371.0 *
/// `0x3571C0F8` is `0x3B0BF0E4`, and `0x3B0BF0E4 + 0x44E7C2FC` is
/// `0x44E7C30D`. 2372.0 * slope is `0x3B0C0000`, and that plus the
/// intercept is `0x44E7C30E`. A table computed with `f32::mul_add` holds
/// `0x44E7C30D` for both, and today's uniform route fuses the same way.
///
/// Under LINEAR_EXACT centre `0x44E7C30D` width `0x38FFFFD5` output 0 to
/// 255 the display values are exactly 0.0 and 255.0: with the pinned width
/// both bounds `c - w/2` and `c + w/2` round to `0x44E7C30D`, so the centre
/// value clamps to ymin and the next float up clamps to ymax. The fused
/// stage 1 draws 0.0 for both, which is the 99th-percentile-black defect.
#[test]
#[ignore = "needs a real GPU adapter, which the CI floor does not have (D-04)"]
fn the_stage_one_pair_returns_ocelli_pixels_bits_and_draws_the_window_ends() {
    let Some(context) = context() else {
        println!("this machine resolved tier C, so there is no device");
        return;
    };
    let fixture = tabled_route(0, 65_536, from_bits(0x3571_C0F8), from_bits(0x44E7_C2FC));
    let uniform = VoiParams {
        center: from_bits(0x44E7_C30D),
        width: from_bits(0x38FF_FFD5),
        slope: from_bits(0x3571_C0F8),
        intercept: from_bits(0x44E7_C2FC),
        ymin: 0.0,
        ymax: 255.0,
        fn_kind: 1,
        invert: 0,
    };
    let staged = stage_one_on_gpu(&context, uniform, &fixture, &[2371.0, 2372.0]);
    assert_eq!(
        value(&staged, 0).to_bits(),
        0x44E7_C30D,
        "stage 1 of 2371 is not modality's exact bits"
    );
    assert_eq!(
        value(&staged, 1).to_bits(),
        0x44E7_C30E,
        "stage 1 of 2372 fused the multiply and add into one rounding"
    );

    let params = VoiParams {
        center: from_bits(0x44E7_C30D),
        width: from_bits(0x38FF_FFD5),
        slope: from_bits(0x3571_C0F8),
        intercept: from_bits(0x44E7_C2FC),
        ymin: 0.0,
        ymax: 255.0,
        fn_kind: 1,
        invert: 0,
    };
    let drawn = on_gpu_routed(&context, params, &fixture, &[2371.0, 2372.0]);
    assert_eq!(
        value(&drawn, 0).to_bits(),
        0.0_f32.to_bits(),
        "the centre value did not clamp to ymin"
    );
    assert_eq!(
        value(&drawn, 1).to_bits(),
        255.0_f32.to_bits(),
        "the next float up did not clamp to ymax"
    );
}

/// **AC2, the cancellation pair through the folded table, pinned as bits.**
///
/// Stored 56893.0, slope `0x3D47EF8C`, intercept `0xC52D91AC`, the same
/// tabled route. The probe returns exactly `0xBC9A0000`: the product
/// `f32(56893.0 * slope)` is `0x452D915F`, which cancels against the
/// intercept's leading bits, so the two-rounding value is `0xBC9A0000`
/// (-0.018798828125) and the fused value is `0xBC990748`
/// (-0.018680229783058167), about 1.19e-4 apart with no unit margin able to
/// cover it. Under LINEAR_EXACT centre `0xBC9A0000` width `0x39D1B717`
/// output 0 to 255 the display is exactly 127.5, the window centre. The
/// fused bits under the same window draw 203.1064453125, which is the red
/// this row exists for.
#[test]
#[ignore = "needs a real GPU adapter, which the CI floor does not have (D-04)"]
fn the_cancellation_pair_returns_ocelli_pixels_bits_and_draws_the_centre() {
    let Some(context) = context() else {
        println!("this machine resolved tier C, so there is no device");
        return;
    };
    let fixture = tabled_route(0, 65_536, from_bits(0x3D47_EF8C), from_bits(0xC52D_91AC));
    let uniform = VoiParams {
        center: from_bits(0xBC9A_0000),
        width: from_bits(0x39D1_B717),
        slope: from_bits(0x3D47_EF8C),
        intercept: from_bits(0xC52D_91AC),
        ymin: 0.0,
        ymax: 255.0,
        fn_kind: 1,
        invert: 0,
    };
    let staged = stage_one_on_gpu(&context, uniform, &fixture, &[56_893.0]);
    assert_eq!(
        value(&staged, 0).to_bits(),
        0xBC9A_0000,
        "the cancellation case fused the product into the intercept"
    );

    let params = VoiParams {
        center: from_bits(0xBC9A_0000),
        width: from_bits(0x39D1_B717),
        slope: from_bits(0x3D47_EF8C),
        intercept: from_bits(0xC52D_91AC),
        ymin: 0.0,
        ymax: 255.0,
        fn_kind: 1,
        invert: 0,
    };
    let drawn = on_gpu_routed(&context, params, &fixture, &[56_893.0]);
    assert_eq!(
        value(&drawn, 0).to_bits(),
        127.5_f32.to_bits(),
        "the window centre did not draw 127.5"
    );
    // The red this row exists for: the fused bits under the same window.
    // Hand-computed on the pinned bits, independent of the implementation:
    // (0xBC990748 - 0xBC9A0000) / 0x39D1B717 + 0.5, times 255.
    let fused_display = ((f32::from_bits(0xBC99_0748) - f32::from_bits(0xBC9A_0000))
        / f32::from_bits(0x39D1_B717)
        + 0.5)
        * 255.0;
    assert!(
        (value(&drawn, 0) - fused_display).abs() > 1.0,
        "the drawn value moved to the fused value's grey {fused_display}"
    );
}

/// **AC4, every integer of a 16-bit container through the folded table.**
///
/// One dispatch of 65,536 values: the probe's bits equal
/// `ocelli_pixel::modality`'s for each, with AC1's rescale. The display
/// sweep under LINEAR_EXACT centre `f32(1854.12)` width `f32(0.1)` stays
/// within `SWEEP_TOLERANCE` of the CPU chain, compared value by value
/// against `LutChain::map_into`. This is what kills the fold index applied
/// twice or not at all, which zeroes half the range, and the one-row
/// 65,536-wide table, which fails `create_texture` on every named adapter.
#[test]
#[ignore = "needs a real GPU adapter, which the CI floor does not have (D-04)"]
fn every_container_integer_reads_its_own_table_entry() {
    let Some(context) = context() else {
        println!("this machine resolved tier C, so there is no device");
        return;
    };
    let slope = from_bits(0x3571_C0F8);
    let intercept = from_bits(0x44E7_C2FC);
    let fixture = tabled_route(0, 65_536, slope, intercept);
    let inputs: Vec<f32> = (0..65_536_u32)
        .map(|index| f32::from(u16::try_from(index).unwrap_or(0)))
        .collect();
    let uniform = VoiParams {
        center: 1854.12_f32,
        width: 0.1_f32,
        slope,
        intercept,
        ymin: 0.0,
        ymax: 255.0,
        fn_kind: 1,
        invert: 0,
    };
    let staged = stage_one_on_gpu(&context, uniform, &fixture, &inputs);
    for (index, input) in inputs.iter().copied().enumerate() {
        let wanted = ocelli_pixel::modality(Stored(input), slope, intercept).0;
        let got = value(&staged, index);
        assert_eq!(
            got.to_bits(),
            wanted.to_bits(),
            "container value {index} staged {got} bits {}, wanted {wanted} bits {}",
            got.to_bits(),
            wanted.to_bits()
        );
    }

    let modality = ModalityTransform::new(None, Some(slope), Some(intercept));
    let voi = VoiTransform::new(
        None,
        &[1854.12_f32],
        &[0.1_f32],
        0,
        VoiFunction::LinearExact,
        0.0,
        255.0,
    );
    assert!(modality.is_ok() && voi.is_ok());
    let (Ok(modality), Ok(voi)) = (modality, voi) else {
        return;
    };
    let built = LutChain::new(
        modality,
        voi,
        PhotometricInterpretation::Monochrome2,
        PresentationLutEvidence::Absent,
    );
    assert!(built.is_ok());
    let Ok(built) = built else { return };
    let params = params_of(&built);
    let drawn = on_gpu_routed(&context, params, &fixture, &inputs);
    let stored: Vec<Stored> = inputs.iter().map(|value| Stored(*value)).collect();
    let mut on_cpu = vec![Display(0.0); stored.len()];
    assert!(built.map_into(&stored, &mut on_cpu).is_ok());
    let mut worst = 0.0_f32;
    for (index, cpu) in on_cpu.iter().enumerate() {
        let got = value(&drawn, index);
        let difference = (got - cpu.0).abs();
        worst = worst.max(difference);
        assert!(
            difference < SWEEP_TOLERANCE,
            "container value {index}: gpu {got} cpu {}, difference {difference}",
            cpu.0
        );
    }
    println!("measured maximum display divergence over the container sweep: {worst}");
}

/// **AC12, the flag-0 route returns its input unchanged.**
///
/// The harness probe with route flag 0 and the AC2 rescale in the section
/// 18.4 uniform: input `0x44E7C30D` (1854.0953369140625) with slope
/// `0x3D47EF8C` and intercept `0xC52D91AC` in the uniform returns exactly
/// `0x44E7C30D`. A surviving stage 1 would return `0xC527E9A0`,
/// -2686.6015625, about -2686.6, because the texture already holds modality
/// values and a second application re-rescales them.
#[test]
#[ignore = "needs a real GPU adapter, which the CI floor does not have (D-04)"]
fn the_flag_zero_route_returns_its_input_bits_unchanged() {
    let Some(context) = context() else {
        println!("this machine resolved tier C, so there is no device");
        return;
    };
    // The AC2 rescale rides in the section 18.4 uniform, where a surviving
    // stage 1 would read it. The route fixture carries flag 0, so the probe
    // must return its input bits and nothing else.
    let params = VoiParams {
        center: 0.0,
        width: 1.0,
        slope: from_bits(0x3D47_EF8C),
        intercept: from_bits(0xC52D_91AC),
        ymin: 0.0,
        ymax: 255.0,
        fn_kind: 1,
        invert: 0,
    };
    let probed = stage_one_on_gpu(
        &context,
        params,
        &identity_route(),
        &[from_bits(0x44E7_C30D)],
    );
    assert_eq!(
        value(&probed, 0).to_bits(),
        0x44E7_C30D,
        "the flag-0 probe re-applied the rescale and returned 0xC527E9A0's class"
    );
}

/// **AC15, the non-multiple container, pinned.**
///
/// `Tabled { first: 0, count: 4095 }` registers a 256 x 16 table, 4096
/// texels with the last row's final texel padding. The stage-1 probe of
/// stored 3840 to 4094, all inside the declared range and so past
/// `SampleOutsideTable`, returns exactly `ocelli_pixel::modality`'s bits
/// with AC1's rescale pair. The truncated height of the earlier draft,
/// 4095 / 256 = 15, addresses row 15 for exactly these values, and WGSL
/// 17.7.4 returns zero or a neighbouring texel for it, a quietly wrong grey.
#[test]
#[ignore = "needs a real GPU adapter, which the CI floor does not have (D-04)"]
fn the_non_multiple_container_probes_its_last_row() {
    let Some(context) = context() else {
        println!("this machine resolved tier C, so there is no device");
        return;
    };
    let slope = from_bits(0x3571_C0F8);
    let intercept = from_bits(0x44E7_C2FC);
    let fixture = tabled_route(0, 4_095, slope, intercept);
    assert_eq!(
        fixture.table.len(),
        4_096,
        "the padded table holds fold * count.div_ceil(fold) texels"
    );
    let inputs: Vec<f32> = (3_840_u32..=4_094_u32)
        .map(|index| f32::from(u16::try_from(index).unwrap_or(0)))
        .collect();
    let uniform = VoiParams {
        center: from_bits(0x44E7_C30D),
        width: from_bits(0x38FF_FFD5),
        slope,
        intercept,
        ymin: 0.0,
        ymax: 255.0,
        fn_kind: 1,
        invert: 0,
    };
    let staged = stage_one_on_gpu(&context, uniform, &fixture, &inputs);
    for (index, input) in inputs.iter().copied().enumerate() {
        let wanted = ocelli_pixel::modality(Stored(input), slope, intercept).0;
        let got = value(&staged, index);
        assert_eq!(
            got.to_bits(),
            wanted.to_bits(),
            "value {} staged {got} bits, wanted {wanted} bits",
            input
        );
    }
}

/// **AC16, the below-one-byte container, pinned.**
///
/// `Tabled { first: 0, count: 16 }`, a 4-bit `bitsStored` container,
/// registers a 256 x 1 table whose first 16 entries are real and 240
/// padding, and the probe of stored 0 to 15 returns
/// `ocelli_pixel::modality`'s bits with AC1's rescale pair. The truncated
/// height of the earlier draft computes 0 and `create_texture` refuses the
/// zero extent, making every small container unavailable.
#[test]
#[ignore = "needs a real GPU adapter, which the CI floor does not have (D-04)"]
fn the_below_one_byte_container_probes_its_sixteen_entries() {
    let Some(context) = context() else {
        println!("this machine resolved tier C, so there is no device");
        return;
    };
    let slope = from_bits(0x3571_C0F8);
    let intercept = from_bits(0x44E7_C2FC);
    let fixture = tabled_route(0, 16, slope, intercept);
    assert_eq!(
        fixture.table.len(),
        256,
        "a 4-bit container still registers a full fold row with 240 padded texels"
    );
    let inputs: Vec<f32> = (0..16_u32)
        .map(|index| f32::from(u16::try_from(index).unwrap_or(0)))
        .collect();
    let uniform = VoiParams {
        center: from_bits(0x44E7_C30D),
        width: from_bits(0x38FF_FFD5),
        slope,
        intercept,
        ymin: 0.0,
        ymax: 255.0,
        fn_kind: 1,
        invert: 0,
    };
    let staged = stage_one_on_gpu(&context, uniform, &fixture, &inputs);
    for (index, input) in inputs.iter().copied().enumerate() {
        let wanted = ocelli_pixel::modality(Stored(input), slope, intercept).0;
        let got = value(&staged, index);
        assert_eq!(
            got.to_bits(),
            wanted.to_bits(),
            "value {index} staged {got} bits, wanted {wanted} bits"
        );
    }
}
