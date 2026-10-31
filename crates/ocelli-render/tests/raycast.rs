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
    BlendMode, FrameOutcome, Pass, PassKind, PipelineFamily, PipelineKey, RayAxis, RenderGraph,
    Tier, VOI_WGSL, ViewportId, VoiParams, VolumeBrick, VolumePass, VolumeRaySource,
    sampled_texture_limit, volume_ray_admission, volume_ray_module,
};
use ocelli_volume::{Level, SlicePresence, Volume, VoxelKind, VoxelStore};

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
    assert!(module.contains("let alpha = 0.5"));
    assert!(module.contains("if (flag == 0u)"));
    assert!(!module.contains("var<storage"));
    assert!(!module.contains("@compute"));
    assert_eq!(RayAxis::Column.code(), 0);
    assert_eq!(RayAxis::Row.code(), 1);
    assert_eq!(RayAxis::Slice.code(), 2);
}

#[test]
fn volume_raycast_has_no_cpu_image() {
    assert!(matches!(
        volume_ray_admission(Tier::Cpu, 1, 16),
        Err(ocelli_render::GraphError::TierCUnavailable)
    ));
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
    let pixel = read_pixel(&fixture.context, &texture);
    assert_eq!(fixture.graph.tick(&fixture.context), FrameOutcome::Noop);
    Some(pixel)
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
