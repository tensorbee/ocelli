//! Tiers A and B against tier C's channel byte rule at exact ties. F-239.
//!
//! **Every test here is `#[ignore]`d** and `bin/ocelli.sh gate gpu` runs them,
//! because deviation D-04 leaves CI without an adapter. On a machine that
//! resolves tier C each test says so and returns.
//!
//! The operator decided on 2026-11-07 that tier C rounds a channel tie to the
//! even byte, as cornerstone3D 5.8.2's `Uint8ClampedArray` does, and asked for
//! the GPU path to be compared with it. A GPU frame reaches a byte two ways:
//!
//! - through a table the CPU baked, addressed in WGSL. The grey LINEAR ramp's
//!   byte is `round_half_even_byte` in `tier_b_fragment_module`, which is ours
//!   and is ASSERTED here against tier C at every tie in `0 ..= 255`.
//! - through the render target's own float to `unorm8` conversion, when
//!   `present_display` returns `unit` with no colour table. That conversion
//!   belongs to the hardware and its driver, so it is MEASURED here and
//!   printed, never asserted, which is decision D14.

use ocelli_core::Stored;
use ocelli_pixel::{
    ColorTransform, ColourWindow, DecodedLayout, DecodedPhotometric, PhotometricInterpretation,
    PixelDataEncoding, PlanarConfiguration, SampleLayout,
};
use ocelli_render::{
    GpuContext, RasterColor, TierRequest, colour_bytes_into, resolve_adapter,
    tier_b_fragment_module,
};
use wgpu::util::DeviceExt;

/// Every exact tie `k + 0.5` for `k` in `0 ..= 254`, then the float either
/// side of each, which is not a tie.
fn inputs() -> Vec<f32> {
    let ties: Vec<f32> = (0_u8..255).map(|k| f32::from(k) + 0.5).collect();
    let mut all = ties.clone();
    all.extend(ties.iter().map(|tie| f32::from_bits(tie.to_bits() - 1)));
    all.extend(ties.iter().map(|tie| f32::from_bits(tie.to_bits() + 1)));
    all
}

/// Tier C's byte for each value, through the public colour entry.
///
/// An unscaled native RGB transform is the identity on its samples, and the
/// initial colour window is skipped, so the byte is the channel rule's alone.
fn tier_c_bytes(values: &[f32]) -> Vec<u8> {
    let layout = SampleLayout::new(
        3,
        PhotometricInterpretation::Rgb,
        Some(PlanarConfiguration::Interleaved),
    );
    let Ok(layout) = layout else {
        unreachable!("three-sample RGB is a valid layout")
    };
    let transform = ColorTransform::resolve(
        layout,
        PixelDataEncoding::Native,
        DecodedPhotometric::Preserved,
        DecodedLayout::Preserved,
        None,
    );
    let Ok(transform) = transform else {
        unreachable!("native RGB resolves")
    };
    let Ok(color) = RasterColor::supported(&transform) else {
        unreachable!("an RGB transform has no palette to refuse")
    };
    let stored: Vec<Stored> = values
        .iter()
        .flat_map(|value| [Stored(*value); 3])
        .collect();
    let mut out = vec![0_u8; values.len() * 3];
    let columns = u32::try_from(values.len()).unwrap_or(0);
    let drawn = colour_bytes_into(
        color,
        &ColourWindow::initial(),
        &stored,
        columns,
        &mut Vec::new(),
        &mut out,
    );
    assert_eq!(drawn, Ok(()));
    out.iter().step_by(3).copied().collect()
}

fn native_clock() -> impl FnMut() -> u64 {
    let start = std::time::Instant::now();
    move || u64::try_from(start.elapsed().as_nanos()).unwrap_or(u64::MAX)
}

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

/// The WGSL text of one function from the tier B fragment module, so the
/// harness runs the shipped function rather than a copy of it.
fn wgsl_function(name: &str) -> String {
    let module = tier_b_fragment_module();
    let head = format!("fn {name}(");
    let Some(start) = module.find(&head) else {
        return String::new();
    };
    let Some(length) = module.get(start..).and_then(|rest| rest.find("\n}\n")) else {
        return String::new();
    };
    module
        .get(start..start + length + 3)
        .unwrap_or_default()
        .to_owned()
}

/// Run `round_half_even_byte` over `values` and read the results back.
fn wgsl_bytes(context: &GpuContext, values: &[f32]) -> Vec<f32> {
    let device = context.device();
    let function = wgsl_function("round_half_even_byte");
    assert!(
        function.contains("fn round_half_even_byte("),
        "the tier B module no longer carries round_half_even_byte"
    );
    let source = format!(
        "{function}
@group(0) @binding(0) var<storage, read> inputs : array<f32>;
@group(0) @binding(1) var<storage, read_write> outputs : array<f32>;

@compute @workgroup_size(64)
fn cs_main(@builtin(global_invocation_id) gid : vec3<u32>) {{
    let i = gid.x;
    if (i >= arrayLength(&inputs)) {{ return; }}
    outputs[i] = round_half_even_byte(inputs[i]);
}}
"
    );
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("ocelli byte ties"),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    });
    let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
        label: Some("ocelli byte ties"),
        layout: None,
        module: &module,
        entry_point: Some("cs_main"),
        compilation_options: wgpu::PipelineCompilationOptions::default(),
        cache: None,
    });
    let input = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("ocelli byte ties inputs"),
        contents: bytemuck::cast_slice(values),
        usage: wgpu::BufferUsages::STORAGE,
    });
    let span = u64::try_from(std::mem::size_of_val(values)).unwrap_or(0);
    let output = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("ocelli byte ties outputs"),
        size: span,
        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        mapped_at_creation: false,
    });
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("ocelli byte ties readback"),
        size: span,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("ocelli byte ties"),
        layout: &pipeline.get_bind_group_layout(0),
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: input.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: output.as_entire_binding(),
            },
        ],
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("ocelli byte ties"),
    });
    {
        let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
            label: Some("ocelli byte ties"),
            timestamp_writes: None,
        });
        pass.set_pipeline(&pipeline);
        pass.set_bind_group(0, &bind, &[]);
        let groups = u32::try_from(values.len().div_ceil(64)).unwrap_or(1);
        pass.dispatch_workgroups(groups, 1, 1);
    }
    encoder.copy_buffer_to_buffer(&output, 0, &readback, 0, span);
    context.queue().submit(std::iter::once(encoder.finish()));
    let mapped = map_read(context, &readback);
    bytemuck::cast_slice(&mapped).to_vec()
}

/// Map a readback buffer and copy its bytes out.
fn map_read(context: &GpuContext, buffer: &wgpu::Buffer) -> Vec<u8> {
    let slice = buffer.slice(..);
    slice.map_async(wgpu::MapMode::Read, |_| {});
    let polled = context.device().poll(wgpu::PollType::Wait {
        submission_index: None,
        timeout: None,
    });
    assert!(polled.is_ok(), "the device would not complete the work");
    let view = slice.get_mapped_range();
    assert!(view.is_ok(), "the readback buffer did not map");
    let Ok(view) = view else { return Vec::new() };
    let bytes = view.to_vec();
    drop(view);
    buffer.unmap();
    bytes
}

/// The render target's float to `unorm8` store of `(k + 0.5) / 255`, which is
/// what `present_display` returns as `unit` for display `k + 0.5` over the
/// `0 ..= 255` range, one pixel per `k` in `0 ..= 254`.
fn hardware_bytes(context: &GpuContext) -> Vec<u8> {
    let device = context.device();
    let width = 256_u32;
    let source = r"
@vertex
fn vs_main(@builtin(vertex_index) index : u32) -> @builtin(position) vec4<f32> {
    let u = f32((index << 1u) & 2u);
    let v = f32(index & 2u);
    return vec4<f32>(u * 2.0 - 1.0, v * 2.0 - 1.0, 0.0, 1.0);
}

@fragment
fn fs_main(@builtin(position) at : vec4<f32>) -> @location(0) vec4<f32> {
    let display = floor(at.x) + 0.5;
    let unit = (display - 0.0) / 255.0;
    return vec4<f32>(unit, unit, unit, 1.0);
}
";
    let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("ocelli unorm ties"),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    });
    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("ocelli unorm ties"),
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
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("ocelli unorm ties"),
        size: wgpu::Extent3d {
            width,
            height: 1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let row = width * 4;
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("ocelli unorm ties readback"),
        size: u64::from(row),
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("ocelli unorm ties"),
    });
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("ocelli unorm ties"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&pipeline);
        pass.draw(0..3, 0..1);
    }
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(row),
                rows_per_image: Some(1),
            },
        },
        wgpu::Extent3d {
            width,
            height: 1,
            depth_or_array_layers: 1,
        },
    );
    context.queue().submit(std::iter::once(encoder.finish()));
    let bytes = map_read(context, &buffer);
    bytes.iter().step_by(4).take(255).copied().collect()
}

/// `round_half_even_byte` and tier C's channel rule agree at all 255 exact
/// ties and at the float either side of each, 765 values.
#[test]
#[ignore = "needs a real GPU adapter, which the CI floor does not have (D-04)"]
fn the_wgsl_tie_rule_agrees_with_tier_c_on_a_real_adapter() {
    let Some(context) = context() else {
        println!("this machine resolved tier C, so there is no device");
        return;
    };
    let values = inputs();
    let cpu = tier_c_bytes(&values);
    let gpu = wgsl_bytes(&context, &values);
    assert_eq!(gpu.len(), values.len());
    let mut disagreements = Vec::new();
    for ((value, cpu), gpu) in values.iter().zip(&cpu).zip(&gpu) {
        if gpu.to_bits() != f32::from(*cpu).to_bits() {
            disagreements.push((*value, *cpu, *gpu));
        }
    }
    println!(
        "round_half_even_byte against tier C: {} values, {} disagree",
        values.len(),
        disagreements.len()
    );
    assert!(disagreements.is_empty(), "{disagreements:?}");
}

/// The hardware's own `unorm8` store of each tie, measured and printed.
///
/// `(k + 0.5) / 255` is not exact in `f32`, so the stored float sits a little
/// above or below the true tie and the driver's conversion decides. A count
/// of differing codes is the measured divergence D14 asks for.
#[test]
#[ignore = "needs a real GPU adapter, which the CI floor does not have (D-04)"]
fn the_render_target_store_at_ties_is_measured_against_tier_c() {
    let Some(context) = context() else {
        println!("this machine resolved tier C, so there is no device");
        return;
    };
    let ties: Vec<f32> = (0_u8..255).map(|k| f32::from(k) + 0.5).collect();
    let cpu = tier_c_bytes(&ties);
    let gpu = hardware_bytes(&context);
    assert_eq!(gpu.len(), ties.len(), "the readback is one byte per tie");
    let differ: Vec<(f32, u8, u8)> = ties
        .iter()
        .zip(&cpu)
        .zip(&gpu)
        .filter(|((_, cpu), gpu)| cpu != gpu)
        .map(|((tie, cpu), gpu)| (*tie, *cpu, *gpu))
        .collect();
    let up = gpu
        .iter()
        .zip(&ties)
        .filter(|(byte, tie)| f32::from(**byte) > **tie)
        .count();
    println!(
        "unorm8 store at 255 ties: {} differ from tier C, {} rounded up, {} rounded down",
        differ.len(),
        up,
        ties.len() - up
    );
    println!("differing (tie, tier C, gpu): {differ:?}");
}
