//! F-059. A composite volume ray on tier A and tier B.
//!
//! The two stored values are hand-computed from PS3.3 C.11.2.1.3.2.
//! LINEAR_EXACT at centre 127.5 and width 255 maps stored 0 to display 0,
//! because `0 <= 127.5 - 127.5`. It maps stored 200 to display 200, because
//! 200 is inside `(0, 255]` and `((200 - 127.5) / 255 + 0.5) * 255 = 200`.
//! Front to back uses opacity 0.5. The expected colour is that formula in
//! `f32`, not a captured frame. Comparison of `C` is 2 ULP. `A` is exact.
//!
//! The GPU rows are `#[ignore]`d. The blend refusal, the tier C refusal and
//! the texture cap need no device.

use glam::DMat3;
use ocelli_cache::{BrickKey, VolumeId};
use ocelli_core::{Pt, World};
use ocelli_pixel::{
    LutChain, ModalityTransform, PhotometricInterpretation, PresentationLutEvidence, VoiFunction,
    VoiTransform,
};
use ocelli_render::{
    BlendMode, ClipPlane, FrameOutcome, GraphError, OpacityError, OpacityPreset, OpacityTable,
    Pass, PassKind, PipelineFamily, PipelineKey, RayAxis, RenderGraph, ShadeError, ShadeWeights,
    Tier, VOI_WGSL, ViewportId, VoiParams, VolumeBrick, VolumePass, VolumeRaySource,
    sampled_texture_limit, volume_ray_admission, volume_ray_module,
};
use ocelli_volume::{Level, SlicePresence, Volume, VoxelKind, VoxelStore};

const ORIGIN_MM: [f64; 3] = [0.0, 0.0, 0.0];
const IDENTITY_DIRECTION: [[f64; 3]; 3] = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];

/// `(200 / 255) * 0.5 * 0.5`. Both steps resident, near sample is 0.
fn both_resident() -> f32 {
    (200.0_f32 / 255.0) * 0.5 * 0.5
}

/// `(200 / 255) * 0.5`. The near sample is skipped, or it is the only contributor.
fn one_resident_colour() -> f32 {
    (200.0_f32 / 255.0) * 0.5
}

/// Brick 0 holds 100 and brick 1 holds 200. Both steps are resident.
fn two_brick_colour() -> f32 {
    let near = (100.0_f32 / 255.0) * 0.5;
    let far = (200.0_f32 / 255.0) * 0.5;
    near + (1.0 - 0.5) * far
}

fn within_two_ulp(got: f32, expected: f32) -> bool {
    if got.to_bits() == expected.to_bits() {
        return true;
    }
    if got.is_sign_negative() != expected.is_sign_negative() {
        return false;
    }
    got.to_bits().abs_diff(expected.to_bits()) <= 2
}

#[test]
fn a_volume_mip_is_not_the_composite_ray() {
    let composite = PipelineKey {
        pass: PassKind::VolumeRaycast,
        blend: BlendMode::Composite,
        tier: Tier::A,
    };
    let mip = PipelineKey {
        blend: BlendMode::Mip,
        ..composite
    };
    assert!(RenderGraph::pipeline_compiled(&composite));
    assert!(RenderGraph::pipeline_family_compiled(
        &composite,
        PipelineFamily::Fragment
    ));
    assert!(!RenderGraph::pipeline_family_compiled(
        &composite,
        PipelineFamily::Compute
    ));
    assert!(!RenderGraph::pipeline_compiled(&mip));
    assert!(!RenderGraph::pipeline_family_compiled(
        &mip,
        PipelineFamily::Fragment
    ));
    let viewport = ViewportId(1);
    assert_eq!(
        RenderGraph::request_outcome(&mip, PipelineFamily::Fragment, viewport),
        FrameOutcome::Unavailable { viewport }
    );
    assert!(!matches!(
        RenderGraph::request_outcome(&composite, PipelineFamily::Fragment, viewport),
        FrameOutcome::Unavailable { .. }
    ));
    let module = volume_ray_module(2);
    assert!(module.contains("var brick_0 : texture_3d<u32>"));
    assert!(module.contains("var brick_1 : texture_3d<u32>"));
    assert!(module.starts_with(VOI_WGSL));
    assert!(module.contains("table_index(display, 255u)"));
    assert!(module.contains("var<uniform> opacity"));
    assert!(module.contains("var<uniform> shade"));
    assert!(module.contains("@binding(3)"));
    assert!(module.contains("@binding(4)"));
    assert!(module.contains("dot(delta, normal.xyz) >= 0.0"));
    assert!(module.contains("crop.axis_x.xyz * (ray.spacing_x * index.x)"));
    assert!(module.contains("crop.axis_y.xyz * (ray.spacing_y * index.y)"));
    assert!(module.contains("crop.axis_z.xyz * (ray.spacing_z * index.z)"));
    assert!(module.contains("ambient + diffuse * max(0.0, facing)"));
    assert!(module.contains("let lit = colour * shade_of"));
    assert!(!module.contains("let alpha = 0.5"));
    assert!(module.contains("if (flag == 0u)"));
    assert!(!module.contains("var<storage"));
    assert!(!module.contains("@compute"));
    assert_eq!(RayAxis::Column.code(), 0);
    assert_eq!(RayAxis::Row.code(), 1);
    assert_eq!(RayAxis::Slice.code(), 2);
}

#[test]
fn volume_raycast_has_no_cpu_image() {
    // One ready brick. The success type is unit, so admission cannot return a
    // colour buffer. Tier C is refused before the sampled-texture check.
    let cpu_ray: Result<(), GraphError> = volume_ray_admission(Tier::Cpu, 1, 16);
    assert!(matches!(cpu_ray, Err(GraphError::TierCUnavailable)));
    assert!(volume_ray_admission(Tier::A, 1, 16).is_ok());
    // Two bricks fit in a limit of 16 and still leave one binding for residency.
    assert!(volume_ray_admission(Tier::A, 2, 16).is_ok());
    assert!(volume_ray_admission(Tier::B, 1, 2).is_ok());
    // A cap of 0 refuses before a shader runs. A limit of 1 has no room for
    // both a brick and the residency texture. A full limit of 16 cannot take
    // 16 bricks, because residency is the extra sampled binding.
    assert!(matches!(
        volume_ray_admission(Tier::A, 1, 0),
        Err(ocelli_render::GraphError::SampledTextureLimit)
    ));
    assert!(matches!(
        volume_ray_admission(Tier::B, 1, 1),
        Err(ocelli_render::GraphError::SampledTextureLimit)
    ));
    assert!(matches!(
        volume_ray_admission(Tier::A, 16, 16),
        Err(ocelli_render::GraphError::SampledTextureLimit)
    ));
}

/// PS3.3 C.11.2.1.3.2 LINEAR_EXACT, centre 127.5, width 255, output 0 to 255.
/// Stored 200 is inside `(0, 255]` and maps to display 200. One front-to-back
/// step from a clear pixel is `colour * alpha` with `colour = 200/255`.
fn one_step_colour(alpha: f32) -> f32 {
    (200.0_f32 / 255.0) * alpha
}

#[test]
fn flat_opacity_keeps_the_one_step_integral() {
    let table = OpacityTable::preset(OpacityPreset::Flat);
    let Some(alpha) = table.entries().get(200).copied() else {
        unreachable!("256 entries")
    };
    assert_eq!(alpha.to_bits(), 0.5_f32.to_bits());
    assert_eq!(
        one_step_colour(alpha).to_bits(),
        ((200.0_f32 / 255.0) * 0.5).to_bits()
    );
    assert!(
        table
            .entries()
            .iter()
            .all(|entry| entry.to_bits() == 0.5_f32.to_bits())
    );
}

#[test]
fn ramp_opacity_uses_display_200() {
    let table = OpacityTable::preset(OpacityPreset::Ramp);
    let Some(alpha) = table.entries().get(200).copied() else {
        unreachable!("256 entries")
    };
    // Display 200 addresses entry 200. The normalised colour 200/255 is not
    // an index, and the 1024-wide grey ramp is not this table.
    assert_eq!(alpha.to_bits(), (200.0_f32 / 255.0).to_bits());
    assert_eq!(
        one_step_colour(alpha).to_bits(),
        ((200.0_f32 / 255.0) * (200.0_f32 / 255.0)).to_bits()
    );
    assert_eq!(
        table.entries().first().copied().unwrap_or(1.0).to_bits(),
        0.0_f32.to_bits()
    );
    assert_eq!(
        table.entries().last().copied().unwrap_or(0.0).to_bits(),
        1.0_f32.to_bits()
    );
}

#[test]
fn a_bad_opacity_table_leaves_the_previous_one() {
    let previous = OpacityTable::preset(OpacityPreset::Flat);
    let short = [0.5_f32; 255];
    assert!(matches!(
        OpacityTable::from_entries(&short),
        Err(OpacityError::Length)
    ));
    let mut wide = [0.5_f32; 256];
    {
        let Some(first) = wide.first_mut() else {
            unreachable!("256 entries")
        };
        *first = 1.1;
    }
    assert!(matches!(
        OpacityTable::from_entries(&wide),
        Err(OpacityError::OutOfRange)
    ));
    {
        let Some(first) = wide.first_mut() else {
            unreachable!("256 entries")
        };
        *first = -0.1;
    }
    assert!(matches!(
        OpacityTable::from_entries(&wide),
        Err(OpacityError::OutOfRange)
    ));
    {
        let Some(first) = wide.first_mut() else {
            unreachable!("256 entries")
        };
        *first = f32::NAN;
    }
    assert!(matches!(
        OpacityTable::from_entries(&wide),
        Err(OpacityError::NotFinite)
    ));
    {
        let Some(first) = wide.first_mut() else {
            unreachable!("256 entries")
        };
        *first = f32::INFINITY;
    }
    assert!(matches!(
        OpacityTable::from_entries(&wide),
        Err(OpacityError::NotFinite)
    ));
    assert_eq!(
        previous
            .entries()
            .get(200)
            .copied()
            .unwrap_or(0.0)
            .to_bits(),
        0.5_f32.to_bits()
    );
}

#[test]
fn cpu_opacity_is_unavailable_before_a_sample() {
    assert!(matches!(
        volume_ray_admission(Tier::Cpu, 1, 16),
        Err(ocelli_render::GraphError::TierCUnavailable)
    ));
}

fn native_clock() -> impl FnMut() -> u64 {
    let start = std::time::Instant::now();
    move || u64::try_from(start.elapsed().as_nanos()).unwrap_or(u64::MAX)
}

fn session() -> Option<(ocelli_render::ResolvedAdapter, ocelli_render::GpuContext)> {
    let mut clock = native_clock();
    let adapter = pollster::block_on(ocelli_render::resolve_adapter(
        ocelli_render::TierRequest::Auto,
        &mut clock,
    ))?;
    let context = pollster::block_on(adapter.open()).ok()?;
    Some((adapter, context))
}

fn exact_chain(center: f32, width: f32) -> LutChain {
    let modality = ModalityTransform::new(None, Some(1.0), Some(0.0));
    let voi = VoiTransform::new(
        None,
        &[center],
        &[width],
        0,
        VoiFunction::LinearExact,
        0.0,
        255.0,
    );
    assert!(modality.is_ok() && voi.is_ok());
    let (Ok(modality), Ok(voi)) = (modality, voi) else {
        unreachable!("asserted immediately above")
    };
    let built = LutChain::new(
        modality,
        voi,
        PhotometricInterpretation::Monochrome2,
        PresentationLutEvidence::Absent,
    );
    assert!(built.is_ok());
    match built {
        Ok(chain) => chain,
        Err(_) => unreachable!("asserted immediately above"),
    }
}

fn params(chain: &LutChain) -> VoiParams {
    match VoiParams::from_chain(chain) {
        Ok(voi) => voi,
        Err(_) => unreachable!("the fixture chain expresses a uniform"),
    }
}

fn empty_volume(dims: [u32; 3]) -> Volume {
    Volume {
        dims,
        spacing: [1.0, 1.0, 1.0],
        origin: Pt::<World>::new(0.0, 0.0, 0.0),
        direction: DMat3::IDENTITY,
        voxel: VoxelKind::U8,
        data: VoxelStore::new(dims, VoxelKind::U8),
        levels: vec![Level {
            dims,
            spacing: [1.0, 1.0, 1.0],
            offset: 0,
        }],
        present: SlicePresence::new(dims[2]),
        order: Vec::new(),
    }
}

fn key_at(id: u64, brick: [u32; 3]) -> BrickKey {
    BrickKey {
        volume: VolumeId(id),
        level: 0,
        brick,
    }
}

fn target(device: &wgpu::Device) -> (wgpu::TextureView, wgpu::Texture) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("raycast target"),
        size: wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    (view, texture)
}

fn dummy_source(device: &wgpu::Device) -> wgpu::TextureView {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("raycast unused source"),
        size: wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::R32Float,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    texture.create_view(&wgpu::TextureViewDescriptor::default())
}

fn read_pixel(context: &ocelli_render::GpuContext, target: &wgpu::Texture) -> [f32; 4] {
    let device = context.device();
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("raycast readback"),
        size: 256,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("raycast readback"),
    });
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: target,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(256),
                rows_per_image: Some(1),
            },
        },
        wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
    );
    context.queue().submit(std::iter::once(encoder.finish()));
    let slice = buffer.slice(..);
    let (sender, receiver) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = sender.send(result);
    });
    let _ = device.poll(wgpu::PollType::Wait {
        submission_index: None,
        timeout: None,
    });
    let _ = receiver.recv();
    let Ok(data) = slice.get_mapped_range() else {
        return [f32::NAN, f32::NAN, f32::NAN, f32::NAN];
    };
    let mut pixel = [f32::NAN, f32::NAN, f32::NAN, f32::NAN];
    for (lane, cell) in pixel.iter_mut().enumerate() {
        let start = lane.saturating_mul(4);
        let Some(end) = start.checked_add(4) else {
            return pixel;
        };
        let Some(chunk) = data.get(start..end) else {
            return pixel;
        };
        let Ok(array) = <[u8; 4]>::try_from(chunk) else {
            return pixel;
        };
        *cell = f32::from_le_bytes(array);
    }
    pixel
}

fn assert_ray(pixel: [f32; 4], colour: f32, alpha: f32) {
    assert!(
        within_two_ulp(pixel[0], colour),
        "red {got} is not within 2 ULP of {colour}",
        got = pixel[0]
    );
    assert_eq!(pixel[1].to_bits(), pixel[0].to_bits(), "green is not red");
    assert_eq!(pixel[2].to_bits(), pixel[0].to_bits(), "blue is not red");
    assert_eq!(pixel[3].to_bits(), alpha.to_bits(), "alpha is not exact");
}

struct RayFixture {
    adapter: ocelli_render::ResolvedAdapter,
    context: ocelli_render::GpuContext,
    graph: RenderGraph,
    limit: u32,
}

fn open_ray() -> Option<RayFixture> {
    let (adapter, mut context) = session()?;
    let graph = match RenderGraph::new(&mut context, 1, wgpu::TextureFormat::Rgba32Float) {
        Ok(graph) => graph,
        Err(_) => return None,
    };
    let limit = sampled_texture_limit(&adapter);
    Some(RayFixture {
        adapter,
        context,
        graph,
        limit,
    })
}

fn place_pair(first: u8, second: u8) -> Volume {
    let mut volume = empty_volume([1, 1, 2]);
    assert!(volume.place(0, &[first]).is_ok());
    assert!(volume.place(1, &[second]).is_ok());
    volume
}

fn draw_ray(from_far: bool, samples: (u8, u8), flags: [u32; 2]) -> Option<[f32; 4]> {
    draw_ray_table(from_far, samples, flags, None)
}

fn draw_ray_table(
    from_far: bool,
    samples: (u8, u8),
    flags: [u32; 2],
    table: Option<&OpacityTable>,
) -> Option<[f32; 4]> {
    let mut fixture = open_ray()?;
    let volume = place_pair(samples.0, samples.1);
    let synced =
        fixture
            .graph
            .sync_volume(&fixture.context, &fixture.adapter, VolumeId(1), &volume);
    assert!(synced.is_ok());
    let chain = exact_chain(127.5, 255.0);
    let (view, texture) = target(fixture.context.device());
    let source = dummy_source(fixture.context.device());
    let viewport = match fixture.graph.register(
        &fixture.context,
        view,
        source,
        Pass::VolumeRaycast(VolumePass {
            blend: BlendMode::Composite,
            voi: params(&chain),
            axis: RayAxis::Slice,
            from_far,
        }),
    ) {
        Ok(id) => id,
        Err(_) => unreachable!("one viewport fits"),
    };
    let ready = volume.ready_bricks();
    assert_eq!(ready.len(), 1);
    let Some(coord) = ready.first().copied() else {
        unreachable!("one ready brick");
    };
    let key = key_at(1, coord);
    let bricks = [VolumeBrick { key: &key, coord }];
    assert!(
        fixture
            .graph
            .bind_volume_ray(
                &fixture.context,
                &fixture.adapter,
                viewport,
                VolumeRaySource {
                    bricks: &bricks,
                    edge: volume.brick_grid().edge,
                    dims: volume.dims,
                    flags: &flags,
                    sampled_limit: fixture.limit,
                    spacing_mm: [1.0, 1.0, 1.0],
                    origin_mm: ORIGIN_MM,
                    direction: IDENTITY_DIRECTION,
                },
            )
            .is_ok()
    );
    if let Some(table) = table {
        let before = fixture.graph.brick_uploads();
        assert!(
            fixture
                .graph
                .set_opacity(&fixture.context, viewport, table)
                .is_ok()
        );
        assert_eq!(fixture.graph.brick_uploads(), before);
    }
    assert_eq!(
        fixture.graph.tick(&fixture.context),
        FrameOutcome::Submitted {
            viewports: 1,
            passes: 1
        }
    );
    let pixel = read_pixel(&fixture.context, &texture);
    assert_eq!(fixture.graph.tick(&fixture.context), FrameOutcome::Noop);
    Some(pixel)
}

#[test]
#[ignore = "needs a real GPU adapter, which the CI floor does not have (D-04)"]
fn a_replaced_brick_is_not_sampled_until_the_ray_is_bound_again() {
    // PS3.3 C.11.2.1.3.2 LINEAR_EXACT, centre 127.5, width 255.
    // Stored 0 maps to display 0. Stored 255 maps to display 255.
    // Flat opacity is 0.5. Front to back accumulates
    // `(1 - A) * colour * alpha` and `(1 - A) * alpha`.
    // Near 0 and far 255: colour `0.5 * 1 * 0.5 = 0.25`, alpha `0.75`.
    // Both 255: colour `0.5 + 0.5 * 0.5 = 0.75`, alpha `0.75`.
    let Some(mut fixture) = open_ray() else {
        println!("this machine resolved tier C, so there is no device");
        return;
    };
    let mut volume = place_pair(0, 255);
    let synced =
        fixture
            .graph
            .sync_volume(&fixture.context, &fixture.adapter, VolumeId(1), &volume);
    assert!(synced.is_ok());
    let chain = exact_chain(127.5, 255.0);
    let (view, texture) = target(fixture.context.device());
    let source = dummy_source(fixture.context.device());
    let viewport = match fixture.graph.register(
        &fixture.context,
        view,
        source,
        Pass::VolumeRaycast(VolumePass {
            blend: BlendMode::Composite,
            voi: params(&chain),
            axis: RayAxis::Slice,
            from_far: false,
        }),
    ) {
        Ok(id) => id,
        Err(_) => unreachable!("one viewport fits"),
    };
    let ready = volume.ready_bricks();
    let Some(coord) = ready.first().copied() else {
        unreachable!("one ready brick");
    };
    let key = key_at(1, coord);
    let bricks = [VolumeBrick { key: &key, coord }];
    let source = VolumeRaySource {
        bricks: &bricks,
        edge: volume.brick_grid().edge,
        dims: volume.dims,
        flags: &[1, 1],
        sampled_limit: fixture.limit,
        spacing_mm: [1.0, 1.0, 1.0],
        origin_mm: ORIGIN_MM,
        direction: IDENTITY_DIRECTION,
    };
    assert!(
        fixture
            .graph
            .bind_volume_ray(&fixture.context, &fixture.adapter, viewport, source)
            .is_ok()
    );
    assert_eq!(
        fixture.graph.tick(&fixture.context),
        FrameOutcome::Submitted {
            viewports: 1,
            passes: 1
        }
    );
    assert_ray(read_pixel(&fixture.context, &texture), 0.25, 0.75);
    assert_eq!(fixture.graph.tick(&fixture.context), FrameOutcome::Noop);
    assert!(volume.place(0, &[255]).is_ok());
    let before = fixture.graph.brick_uploads();
    let replaced =
        fixture
            .graph
            .sync_volume(&fixture.context, &fixture.adapter, VolumeId(1), &volume);
    let Ok(report) = replaced else {
        unreachable!("a replaced present slice is still a ready brick");
    };
    assert_eq!(report.uploaded, 1);
    assert_eq!(fixture.graph.brick_uploads(), before + 1);
    // The old view must not be drawn. The tick stays unavailable until bind.
    assert_eq!(
        fixture.graph.tick(&fixture.context),
        FrameOutcome::Unavailable { viewport }
    );
    let dirt = fixture
        .graph
        .dirty(viewport)
        .unwrap_or(ocelli_render::Dirty::default());
    assert!(dirt.data);
    let source = VolumeRaySource {
        bricks: &bricks,
        edge: volume.brick_grid().edge,
        dims: volume.dims,
        flags: &[1, 1],
        sampled_limit: fixture.limit,
        spacing_mm: [1.0, 1.0, 1.0],
        origin_mm: ORIGIN_MM,
        direction: IDENTITY_DIRECTION,
    };
    assert!(
        fixture
            .graph
            .bind_volume_ray(&fixture.context, &fixture.adapter, viewport, source)
            .is_ok()
    );
    assert_eq!(
        fixture.graph.tick(&fixture.context),
        FrameOutcome::Submitted {
            viewports: 1,
            passes: 1
        }
    );
    assert_ray(read_pixel(&fixture.context, &texture), 0.75, 0.75);
}

#[test]
#[ignore = "needs a real GPU adapter, which the CI floor does not have (D-04)"]
fn two_resident_steps_match_the_integral() {
    let Some(pixel) = draw_ray(false, (0, 200), [1, 1]) else {
        println!("this machine resolved tier C, so there is no device");
        return;
    };
    assert_ray(pixel, both_resident(), 0.75);
}

#[test]
#[ignore = "needs a real GPU adapter, which the CI floor does not have (D-04)"]
fn an_absent_step_is_skipped() {
    // Both texels are 200. The flag, not the stored zero, is what skips step 0.
    let Some(pixel) = draw_ray(false, (200, 200), [0, 1]) else {
        println!("this machine resolved tier C, so there is no device");
        return;
    };
    assert_ray(pixel, one_resident_colour(), 0.5);
}

#[test]
#[ignore = "needs a real GPU adapter, which the CI floor does not have (D-04)"]
fn the_far_end_is_the_first_sample() {
    let Some(pixel) = draw_ray(true, (0, 200), [1, 1]) else {
        println!("this machine resolved tier C, so there is no device");
        return;
    };
    assert_ray(pixel, one_resident_colour(), 0.75);
}

#[test]
#[ignore = "needs a real GPU adapter, which the CI floor does not have (D-04)"]
fn two_ready_bricks_both_contribute() {
    // Dims z is 129, so the edge of 128 makes two bricks. Slice 0 is 100 in
    // the first brick. Slice 128 is 200 in the second. Every other slice is
    // 0 and its flag is 0, so only those two steps enter the integral.
    let Some(mut fixture) = open_ray() else {
        println!("this machine resolved tier C, so there is no device");
        return;
    };
    let mut volume = empty_volume([1, 1, 129]);
    for z in 0..129 {
        let byte = match z {
            0 => 100,
            128 => 200,
            _ => 0,
        };
        assert!(volume.place(z, &[byte]).is_ok());
    }
    let synced =
        fixture
            .graph
            .sync_volume(&fixture.context, &fixture.adapter, VolumeId(1), &volume);
    assert!(synced.is_ok());
    let ready = volume.ready_bricks();
    assert_eq!(ready.len(), 2);
    let Some(near) = ready.first().copied() else {
        unreachable!("two ready bricks");
    };
    let Some(far) = ready.get(1).copied() else {
        unreachable!("two ready bricks");
    };
    let near_key = key_at(1, near);
    let far_key = key_at(1, far);
    let bricks = [
        VolumeBrick {
            key: &near_key,
            coord: near,
        },
        VolumeBrick {
            key: &far_key,
            coord: far,
        },
    ];
    let mut flags = vec![0u32; 129];
    let Some(first) = flags.first_mut() else {
        unreachable!("129 flags");
    };
    *first = 1;
    let Some(last) = flags.get_mut(128) else {
        unreachable!("129 flags");
    };
    *last = 1;
    let chain = exact_chain(127.5, 255.0);
    let (view, texture) = target(fixture.context.device());
    let source = dummy_source(fixture.context.device());
    let viewport = match fixture.graph.register(
        &fixture.context,
        view,
        source,
        Pass::VolumeRaycast(VolumePass {
            blend: BlendMode::Composite,
            voi: params(&chain),
            axis: RayAxis::Slice,
            from_far: false,
        }),
    ) {
        Ok(id) => id,
        Err(_) => unreachable!("one viewport fits"),
    };
    assert!(
        fixture
            .graph
            .bind_volume_ray(
                &fixture.context,
                &fixture.adapter,
                viewport,
                VolumeRaySource {
                    bricks: &bricks,
                    edge: volume.brick_grid().edge,
                    dims: volume.dims,
                    flags: &flags,
                    sampled_limit: fixture.limit,
                    spacing_mm: [1.0, 1.0, 1.0],
                    origin_mm: ORIGIN_MM,
                    direction: IDENTITY_DIRECTION,
                },
            )
            .is_ok()
    );
    assert_eq!(
        fixture.graph.tick(&fixture.context),
        FrameOutcome::Submitted {
            viewports: 1,
            passes: 1
        }
    );
    assert_ray(
        read_pixel(&fixture.context, &texture),
        two_brick_colour(),
        0.75,
    );
}

#[test]
#[ignore = "needs a real GPU adapter, which the CI floor does not have (D-04)"]
fn ramp_opacity_matches_the_display_fraction() {
    let table = OpacityTable::preset(OpacityPreset::Ramp);
    let Some(pixel) = draw_ray_table(false, (0, 200), [0, 1], Some(&table)) else {
        println!("this machine resolved tier C, so there is no device");
        return;
    };
    let alpha = 200.0_f32 / 255.0;
    assert_ray(pixel, one_step_colour(alpha), alpha);
}

#[test]
#[ignore = "needs a real GPU adapter, which the CI floor does not have (D-04)"]
fn set_opacity_does_not_upload_bricks() {
    let Some(mut fixture) = open_ray() else {
        println!("this machine resolved tier C, so there is no device");
        return;
    };
    let volume = place_pair(0, 200);
    let synced =
        fixture
            .graph
            .sync_volume(&fixture.context, &fixture.adapter, VolumeId(1), &volume);
    assert!(synced.is_ok());
    let ready = volume.ready_bricks();
    let Some(coord) = ready.first().copied() else {
        unreachable!("one ready brick");
    };
    let key = key_at(1, coord);
    let bricks = [VolumeBrick { key: &key, coord }];
    let chain = exact_chain(127.5, 255.0);
    let (view, texture) = target(fixture.context.device());
    let source = dummy_source(fixture.context.device());
    let viewport = match fixture.graph.register(
        &fixture.context,
        view,
        source,
        Pass::VolumeRaycast(VolumePass {
            blend: BlendMode::Composite,
            voi: params(&chain),
            axis: RayAxis::Slice,
            from_far: false,
        }),
    ) {
        Ok(id) => id,
        Err(_) => unreachable!("one viewport fits"),
    };
    assert!(
        fixture
            .graph
            .bind_volume_ray(
                &fixture.context,
                &fixture.adapter,
                viewport,
                VolumeRaySource {
                    bricks: &bricks,
                    edge: volume.brick_grid().edge,
                    dims: volume.dims,
                    flags: &[0, 1],
                    sampled_limit: fixture.limit,
                    spacing_mm: [1.0, 1.0, 1.0],
                    origin_mm: ORIGIN_MM,
                    direction: IDENTITY_DIRECTION,
                },
            )
            .is_ok()
    );
    let before = fixture.graph.brick_uploads();
    let flat = OpacityTable::preset(OpacityPreset::Flat);
    assert!(
        fixture
            .graph
            .set_opacity(&fixture.context, viewport, &flat)
            .is_ok()
    );
    assert_eq!(fixture.graph.brick_uploads(), before);
    let ramp = OpacityTable::preset(OpacityPreset::Ramp);
    assert!(
        fixture
            .graph
            .set_opacity(&fixture.context, viewport, &ramp)
            .is_ok()
    );
    assert_eq!(fixture.graph.brick_uploads(), before);
    assert!(
        fixture
            .graph
            .set_opacity(&fixture.context, viewport, &ramp)
            .is_ok()
    );
    assert_eq!(fixture.graph.brick_uploads(), before);
    assert_eq!(
        fixture.graph.tick(&fixture.context),
        FrameOutcome::Submitted {
            viewports: 1,
            passes: 1
        }
    );
    let alpha = 200.0_f32 / 255.0;
    assert_ray(
        read_pixel(&fixture.context, &texture),
        one_step_colour(alpha),
        alpha,
    );
    assert_eq!(fixture.graph.brick_uploads(), before);
}

/// PS3.3 C.11.2.1.3.2 LINEAR_EXACT, centre 127.5, width 255, output 0 to 255.
/// Stored 200 displays as 200. Flat opacity is 0.5. One front-to-back step
/// from clear is `(200/255) * 0.5`. Unlit shade is 1, so that product stays.
/// Ambient 0 and diffuse 1 with the light along the gradient is also shade 1.
/// The same weights with the light against the gradient are shade 0, so the
/// colour is 0 and the alpha stays 0.5. A zero gradient uses the ambient
/// weight alone: ambient 0.5 gives `(200/255) * 0.5 * 0.5`.
#[test]
fn unlit_weights_keep_the_flat_product() {
    let weights = ShadeWeights::unlit();
    assert_eq!(weights.ambient().to_bits(), 1.0_f32.to_bits());
    assert_eq!(weights.diffuse().to_bits(), 0.0_f32.to_bits());
    assert_eq!(
        one_step_colour(0.5).to_bits(),
        ((200.0_f32 / 255.0) * 0.5).to_bits()
    );
}

#[test]
fn centre_gradient_of_100_200_300_is_one_hundred_per_millimetre() {
    // Index increases toward the far end. Spacing is 1 mm.
    // (sample at +1) minus (sample at -1), divided by 2.
    let against = (300.0_f32 - 100.0_f32) / 2.0;
    assert_eq!(against.to_bits(), 100.0_f32.to_bits());
    let along = (100.0_f32 - 300.0_f32) / 2.0;
    assert_eq!(along.to_bits(), (-100.0_f32).to_bits());
    let weights = match ShadeWeights::from_weights(0.0, 1.0) {
        Ok(weights) => weights,
        Err(_) => unreachable!("0 and 1 are in range"),
    };
    assert_eq!(weights.ambient().to_bits(), 0.0_f32.to_bits());
    assert_eq!(weights.diffuse().to_bits(), 1.0_f32.to_bits());
    // Dot 1 leaves the VOI colour. The pixel is still the one-step product.
    assert_eq!(
        (one_step_colour(0.5) * 1.0).to_bits(),
        one_step_colour(0.5).to_bits()
    );
    // Dot -1 contributes 0. The colour is 0. Alpha is the flat 0.5.
    assert_eq!((one_step_colour(0.5) * 0.0).to_bits(), 0.0_f32.to_bits());
    let ambient = match ShadeWeights::from_weights(0.5, 1.0) {
        Ok(weights) => weights.ambient(),
        Err(_) => unreachable!("0.5 and 1 are in range"),
    };
    assert_eq!(
        ((200.0_f32 / 255.0) * ambient * 0.5).to_bits(),
        ((200.0_f32 / 255.0) * 0.25).to_bits()
    );
}

#[test]
fn a_bad_shade_weight_leaves_the_previous_one() {
    let previous = ShadeWeights::unlit();
    assert!(matches!(
        ShadeWeights::from_weights(f32::NAN, 0.0),
        Err(ShadeError::NotFinite)
    ));
    assert!(matches!(
        ShadeWeights::from_weights(0.0, f32::INFINITY),
        Err(ShadeError::NotFinite)
    ));
    assert!(matches!(
        ShadeWeights::from_weights(1.1, 0.0),
        Err(ShadeError::OutOfRange)
    ));
    assert!(matches!(
        ShadeWeights::from_weights(-0.1, 1.0),
        Err(ShadeError::OutOfRange)
    ));
    assert_eq!(previous.ambient().to_bits(), 1.0_f32.to_bits());
    assert_eq!(previous.diffuse().to_bits(), 0.0_f32.to_bits());
}

#[test]
fn cpu_shade_is_unavailable_before_a_sample() {
    assert!(matches!(
        volume_ray_admission(Tier::Cpu, 1, 16),
        Err(GraphError::TierCUnavailable)
    ));
}

fn u16_line(samples: [u16; 3]) -> Volume {
    let dims = [1, 1, 3];
    let mut volume = Volume {
        dims,
        spacing: [1.0, 1.0, 1.0],
        origin: Pt::<World>::new(0.0, 0.0, 0.0),
        direction: DMat3::IDENTITY,
        voxel: VoxelKind::U16,
        data: VoxelStore::new(dims, VoxelKind::U16),
        levels: vec![Level {
            dims,
            spacing: [1.0, 1.0, 1.0],
            offset: 0,
        }],
        present: SlicePresence::new(dims[2]),
        order: Vec::new(),
    };
    for (index, sample) in samples.iter().copied().enumerate() {
        let Ok(z) = u32::try_from(index) else {
            unreachable!("three slices");
        };
        assert!(volume.place(z, &sample.to_le_bytes()).is_ok());
    }
    volume
}

fn draw_centre(samples: [u16; 3], weights: Option<&ShadeWeights>) -> Option<[f32; 4]> {
    let mut fixture = open_ray()?;
    let volume = u16_line(samples);
    let synced =
        fixture
            .graph
            .sync_volume(&fixture.context, &fixture.adapter, VolumeId(1), &volume);
    assert!(synced.is_ok());
    let chain = exact_chain(127.5, 255.0);
    let (view, texture) = target(fixture.context.device());
    let source = dummy_source(fixture.context.device());
    let viewport = match fixture.graph.register(
        &fixture.context,
        view,
        source,
        Pass::VolumeRaycast(VolumePass {
            blend: BlendMode::Composite,
            voi: params(&chain),
            axis: RayAxis::Slice,
            from_far: false,
        }),
    ) {
        Ok(id) => id,
        Err(_) => unreachable!("one viewport fits"),
    };
    let ready = volume.ready_bricks();
    assert_eq!(ready.len(), 1);
    let Some(coord) = ready.first().copied() else {
        unreachable!("one ready brick");
    };
    let key = key_at(1, coord);
    let bricks = [VolumeBrick { key: &key, coord }];
    assert!(
        fixture
            .graph
            .bind_volume_ray(
                &fixture.context,
                &fixture.adapter,
                viewport,
                VolumeRaySource {
                    bricks: &bricks,
                    edge: volume.brick_grid().edge,
                    dims: volume.dims,
                    flags: &[0, 1, 0],
                    sampled_limit: fixture.limit,
                    spacing_mm: [1.0, 1.0, 1.0],
                    origin_mm: ORIGIN_MM,
                    direction: IDENTITY_DIRECTION,
                },
            )
            .is_ok()
    );
    if let Some(weights) = weights {
        let before = fixture.graph.brick_uploads();
        assert!(
            fixture
                .graph
                .set_shade(&fixture.context, viewport, weights)
                .is_ok()
        );
        assert_eq!(fixture.graph.brick_uploads(), before);
    }
    assert_eq!(
        fixture.graph.tick(&fixture.context),
        FrameOutcome::Submitted {
            viewports: 1,
            passes: 1
        }
    );
    Some(read_pixel(&fixture.context, &texture))
}

fn diffuse_only() -> ShadeWeights {
    match ShadeWeights::from_weights(0.0, 1.0) {
        Ok(weights) => weights,
        Err(_) => unreachable!("0 and 1 are in range"),
    }
}

#[test]
#[ignore = "needs a real GPU adapter, which the CI floor does not have (D-04)"]
fn unlit_shade_keeps_the_flat_integral() {
    // 100, 200, 300 points the gradient at the far end, against the light.
    // A default diffuse weight would zero this pixel.
    let Some(pixel) = draw_centre([100, 200, 300], None) else {
        println!("this machine resolved tier C, so there is no device");
        return;
    };
    assert_ray(pixel, one_step_colour(0.5), 0.5);
}

#[test]
#[ignore = "needs a real GPU adapter, which the CI floor does not have (D-04)"]
fn light_along_the_gradient_keeps_the_voi_product() {
    // 300, 200, 100 points the gradient at the near end, along the light.
    // Shade is 1, so the pixel matches the unlit one-step product.
    let weights = diffuse_only();
    let Some(pixel) = draw_centre([300, 200, 100], Some(&weights)) else {
        println!("this machine resolved tier C, so there is no device");
        return;
    };
    assert_ray(pixel, one_step_colour(0.5), 0.5);
}

#[test]
#[ignore = "needs a real GPU adapter, which the CI floor does not have (D-04)"]
fn light_against_the_gradient_zeroes_the_colour() {
    let Some(mut fixture) = open_ray() else {
        println!("this machine resolved tier C, so there is no device");
        return;
    };
    let volume = u16_line([100, 200, 300]);
    let synced =
        fixture
            .graph
            .sync_volume(&fixture.context, &fixture.adapter, VolumeId(1), &volume);
    assert!(synced.is_ok());
    let chain = exact_chain(127.5, 255.0);
    let (view, texture) = target(fixture.context.device());
    let source = dummy_source(fixture.context.device());
    let viewport = match fixture.graph.register(
        &fixture.context,
        view,
        source,
        Pass::VolumeRaycast(VolumePass {
            blend: BlendMode::Composite,
            voi: params(&chain),
            axis: RayAxis::Slice,
            from_far: false,
        }),
    ) {
        Ok(id) => id,
        Err(_) => unreachable!("one viewport fits"),
    };
    let weights = diffuse_only();
    assert!(matches!(
        fixture
            .graph
            .set_shade(&fixture.context, viewport, &weights),
        Err(GraphError::RayUnavailable)
    ));
    let ready = volume.ready_bricks();
    let Some(coord) = ready.first().copied() else {
        unreachable!("one ready brick");
    };
    let key = key_at(1, coord);
    let bricks = [VolumeBrick { key: &key, coord }];
    let before = fixture.graph.brick_uploads();
    assert!(
        fixture
            .graph
            .bind_volume_ray(
                &fixture.context,
                &fixture.adapter,
                viewport,
                VolumeRaySource {
                    bricks: &bricks,
                    edge: volume.brick_grid().edge,
                    dims: volume.dims,
                    flags: &[0, 1, 0],
                    sampled_limit: fixture.limit,
                    spacing_mm: [1.0, 1.0, 1.0],
                    origin_mm: ORIGIN_MM,
                    direction: IDENTITY_DIRECTION,
                },
            )
            .is_ok()
    );
    assert_eq!(fixture.graph.brick_uploads(), before);
    assert!(
        fixture
            .graph
            .set_shade(&fixture.context, viewport, &weights)
            .is_ok()
    );
    assert_eq!(fixture.graph.brick_uploads(), before);
    assert_eq!(
        fixture.graph.tick(&fixture.context),
        FrameOutcome::Submitted {
            viewports: 1,
            passes: 1
        }
    );
    assert_ray(read_pixel(&fixture.context, &texture), 0.0, 0.5);
    assert!(
        fixture
            .graph
            .set_shade(&fixture.context, viewport, &weights)
            .is_ok()
    );
    assert_eq!(fixture.graph.tick(&fixture.context), FrameOutcome::Noop);
    assert_eq!(fixture.graph.brick_uploads(), before);
}

#[test]
#[ignore = "needs a real GPU adapter, which the CI floor does not have (D-04)"]
fn a_zero_gradient_uses_the_ambient_weight() {
    let weights = match ShadeWeights::from_weights(0.5, 1.0) {
        Ok(weights) => weights,
        Err(_) => unreachable!("0.5 and 1 are in range"),
    };
    let Some(pixel) = draw_centre([200, 200, 200], Some(&weights)) else {
        println!("this machine resolved tier C, so there is no device");
        return;
    };
    assert_ray(pixel, (200.0_f32 / 255.0) * 0.25, 0.5);
}

fn spacing_of(volume: &Volume) -> [f32; 3] {
    [
        ocelli_viewport::view::f32_from_f64(volume.spacing[0]),
        ocelli_viewport::view::f32_from_f64(volume.spacing[1]),
        ocelli_viewport::view::f32_from_f64(volume.spacing[2]),
    ]
}

fn direction_of(volume: &Volume) -> [[f64; 3]; 3] {
    let column = |axis: glam::DVec3| [axis.x, axis.y, axis.z];
    [
        column(volume.direction.col(0)),
        column(volume.direction.col(1)),
        column(volume.direction.col(2)),
    ]
}

fn clip_samples(samples: &[u8], spacing_z: f64) -> Volume {
    let count = u32::try_from(samples.len()).unwrap_or(0);
    let mut volume = empty_volume([1, 1, count]);
    volume.spacing = [1.0, 1.0, spacing_z];
    if let Some(level) = volume.levels.first_mut() {
        level.spacing = volume.spacing;
    }
    for (index, sample) in samples.iter().copied().enumerate() {
        let Ok(z) = u32::try_from(index) else {
            unreachable!("slice index");
        };
        assert!(volume.place(z, &[sample]).is_ok());
    }
    volume
}

fn draw_clip(samples: &[u8], spacing_z: f64, planes: Option<&[ClipPlane]>) -> Option<[f32; 4]> {
    let mut fixture = open_ray()?;
    let volume = clip_samples(samples, spacing_z);
    let synced =
        fixture
            .graph
            .sync_volume(&fixture.context, &fixture.adapter, VolumeId(1), &volume);
    assert!(synced.is_ok());
    let chain = exact_chain(127.5, 255.0);
    let (view, texture) = target(fixture.context.device());
    let source = dummy_source(fixture.context.device());
    let viewport = match fixture.graph.register(
        &fixture.context,
        view,
        source,
        Pass::VolumeRaycast(VolumePass {
            blend: BlendMode::Composite,
            voi: params(&chain),
            axis: RayAxis::Slice,
            from_far: false,
        }),
    ) {
        Ok(id) => id,
        Err(_) => unreachable!("one viewport fits"),
    };
    let ready = volume.ready_bricks();
    assert_eq!(ready.len(), 1);
    let Some(coord) = ready.first().copied() else {
        unreachable!("one ready brick");
    };
    let key = key_at(1, coord);
    let bricks = [VolumeBrick { key: &key, coord }];
    let flags = vec![1_u32; samples.len()];
    assert!(
        fixture
            .graph
            .bind_volume_ray(
                &fixture.context,
                &fixture.adapter,
                viewport,
                VolumeRaySource {
                    bricks: &bricks,
                    edge: volume.brick_grid().edge,
                    dims: volume.dims,
                    flags: &flags,
                    sampled_limit: fixture.limit,
                    spacing_mm: spacing_of(&volume),
                    origin_mm: [volume.origin.x, volume.origin.y, volume.origin.z],
                    direction: direction_of(&volume),
                },
            )
            .is_ok()
    );
    if let Some(planes) = planes {
        let before = fixture.graph.brick_uploads();
        assert!(
            fixture
                .graph
                .set_clip_planes(&fixture.context, viewport, planes)
                .is_ok()
        );
        assert_eq!(fixture.graph.brick_uploads(), before);
        assert_eq!(volume.ready_bricks().len(), 1);
    }
    assert_eq!(
        fixture.graph.tick(&fixture.context),
        FrameOutcome::Submitted {
            viewports: 1,
            passes: 1
        }
    );
    Some(read_pixel(&fixture.context, &texture))
}

#[test]
fn no_planes_keep_the_two_step_colour() {
    // Stored 200, LINEAR_EXACT centre 127.5 width 255, displays as 200.
    // Flat alpha is 0.5. The first step adds `0.5 * colour`. The second adds
    // half of that again. `0.75 * 200/255` is `150/255`.
    let colour = 200.0_f32 / 255.0;
    let two = colour * 0.5 + (1.0 - 0.5) * (colour * 0.5);
    assert_eq!(two.to_bits(), (150.0_f32 / 255.0).to_bits());
    assert_eq!(
        one_step_colour(0.5).to_bits(),
        (100.0_f32 / 255.0).to_bits()
    );
}

#[test]
fn cpu_clip_is_unavailable_before_a_sample() {
    assert!(matches!(
        volume_ray_admission(Tier::Cpu, 1, 16),
        Err(GraphError::TierCUnavailable)
    ));
}

#[test]
#[ignore = "needs a real GPU adapter, which the CI floor does not have (D-04)"]
fn no_planes_keep_both_resident_samples() {
    let Some(pixel) = draw_clip(&[200, 200], 10.0, None) else {
        println!("this machine resolved tier C, so there is no device");
        return;
    };
    assert_ray(pixel, (200.0_f32 / 255.0) * 0.75, 0.75);
}

#[test]
#[ignore = "needs a real GPU adapter, which the CI floor does not have (D-04)"]
fn a_plane_keeps_only_the_sample_past_five_millimetres() {
    let plane = ClipPlane {
        origin_mm: [0.0, 0.0, 5.0],
        normal: [0.0, 0.0, 1.0],
    };
    let Some(pixel) = draw_clip(&[200, 200], 10.0, Some(&[plane])) else {
        println!("this machine resolved tier C, so there is no device");
        return;
    };
    assert_ray(pixel, one_step_colour(0.5), 0.5);
}

#[test]
#[ignore = "needs a real GPU adapter, which the CI floor does not have (D-04)"]
fn a_sample_on_the_plane_stays_in_the_integral() {
    let plane = ClipPlane {
        origin_mm: [0.0, 0.0, 0.0],
        normal: [0.0, 0.0, 1.0],
    };
    let Some(pixel) = draw_clip(&[200], 1.0, Some(&[plane])) else {
        println!("this machine resolved tier C, so there is no device");
        return;
    };
    assert_ray(pixel, one_step_colour(0.5), 0.5);
}

#[test]
#[ignore = "needs a real GPU adapter, which the CI floor does not have (D-04)"]
fn a_refused_plane_list_keeps_the_clipped_colour() {
    let Some(mut fixture) = open_ray() else {
        println!("this machine resolved tier C, so there is no device");
        return;
    };
    let volume = clip_samples(&[200, 200], 10.0);
    let synced =
        fixture
            .graph
            .sync_volume(&fixture.context, &fixture.adapter, VolumeId(1), &volume);
    assert!(synced.is_ok());
    let chain = exact_chain(127.5, 255.0);
    let (view, texture) = target(fixture.context.device());
    let source = dummy_source(fixture.context.device());
    let viewport = match fixture.graph.register(
        &fixture.context,
        view,
        source,
        Pass::VolumeRaycast(VolumePass {
            blend: BlendMode::Composite,
            voi: params(&chain),
            axis: RayAxis::Slice,
            from_far: false,
        }),
    ) {
        Ok(id) => id,
        Err(_) => unreachable!("one viewport fits"),
    };
    let plane = ClipPlane {
        origin_mm: [0.0, 0.0, 5.0],
        normal: [0.0, 0.0, 1.0],
    };
    assert!(matches!(
        fixture
            .graph
            .set_clip_planes(&fixture.context, viewport, &[plane]),
        Err(GraphError::RayUnavailable)
    ));
    let ready = volume.ready_bricks();
    assert_eq!(ready.len(), 1);
    let Some(coord) = ready.first().copied() else {
        unreachable!("one ready brick");
    };
    let key = key_at(1, coord);
    let bricks = [VolumeBrick { key: &key, coord }];
    let before = fixture.graph.brick_uploads();
    assert!(
        fixture
            .graph
            .bind_volume_ray(
                &fixture.context,
                &fixture.adapter,
                viewport,
                VolumeRaySource {
                    bricks: &bricks,
                    edge: volume.brick_grid().edge,
                    dims: volume.dims,
                    flags: &[1, 1],
                    sampled_limit: fixture.limit,
                    spacing_mm: spacing_of(&volume),
                    origin_mm: [volume.origin.x, volume.origin.y, volume.origin.z],
                    direction: direction_of(&volume),
                },
            )
            .is_ok()
    );
    assert_eq!(fixture.graph.brick_uploads(), before);
    assert!(
        fixture
            .graph
            .set_clip_planes(&fixture.context, viewport, &[plane])
            .is_ok()
    );
    assert_eq!(fixture.graph.brick_uploads(), before);
    assert_eq!(volume.ready_bricks().len(), 1);
    assert_eq!(
        fixture.graph.tick(&fixture.context),
        FrameOutcome::Submitted {
            viewports: 1,
            passes: 1
        }
    );
    assert_ray(
        read_pixel(&fixture.context, &texture),
        one_step_colour(0.5),
        0.5,
    );
    let mut seven = Vec::new();
    for _ in 0..7 {
        seven.push(plane);
    }
    assert!(matches!(
        fixture
            .graph
            .set_clip_planes(&fixture.context, viewport, &seven),
        Err(GraphError::ClipCount)
    ));
    assert!(matches!(
        fixture.graph.set_clip_planes(
            &fixture.context,
            viewport,
            &[ClipPlane {
                origin_mm: [f64::NAN, 0.0, 0.0],
                normal: [0.0, 0.0, 1.0],
            }],
        ),
        Err(GraphError::ClipNotFinite)
    ));
    assert!(matches!(
        fixture.graph.set_clip_planes(
            &fixture.context,
            viewport,
            &[ClipPlane {
                origin_mm: [0.0, 0.0, 0.0],
                normal: [0.0, 0.0, 0.0],
            }],
        ),
        Err(GraphError::ClipZeroNormal)
    ));
    assert_eq!(fixture.graph.brick_uploads(), before);
    assert!(
        fixture
            .graph
            .set_clip_planes(&fixture.context, viewport, &[plane])
            .is_ok()
    );
    assert_eq!(fixture.graph.tick(&fixture.context), FrameOutcome::Noop);
    assert_eq!(fixture.graph.brick_uploads(), before);
}
