//! F-054. Thick slab pipelines.
//!
//! Composite, MIP, MinIP and average are four slab keys. Volume MIP and
//! stack MIP stay uncompiled. Labelmap edge is not drawn as composite.
//! Tier C returns before the image is read. The GPU rows are `#[ignore]`d.

use ocelli_pixel::{
    LutChain, ModalityTransform, PhotometricInterpretation, PresentationLutEvidence, VoiFunction,
    VoiTransform,
};
use ocelli_render::{
    BlendMode, FrameOutcome, Pass, PassKind, PipelineFamily, PipelineKey, RenderGraph, SlabError,
    SlabPass, StackPass, Tier, VOI_WGSL, ViewportId, VoiParams, fragment_keys_for, present_slab,
    slab_blend_code, slab_compile_list, slab_module,
};
use ocelli_volume::{PlaneSample, SlabImage};

fn bad_extent() -> SlabImage {
    SlabImage {
        width: 0,
        height: 0,
        step_mm: 1.0,
        index_step: [0.0, 0.0, 1.0],
        sample_count: 0,
        samples: Vec::new(),
    }
}

fn one_sample(sample: PlaneSample) -> SlabImage {
    SlabImage {
        width: 1,
        height: 1,
        step_mm: 2.5,
        index_step: [0.0, 0.0, 1.0],
        sample_count: 1,
        samples: vec![sample],
    }
}

#[test]
fn slab_mip_is_not_the_composite_pipeline() {
    for tier in [Tier::A, Tier::B] {
        let list = slab_compile_list(tier);
        let mut seen = Vec::new();
        for (key, code) in list {
            assert_eq!(key.pass, PassKind::Slab);
            assert_eq!(key.tier, tier);
            assert_eq!(slab_blend_code(key.blend), Some(code));
            assert!(RenderGraph::pipeline_compiled(&key));
            assert!(RenderGraph::pipeline_family_compiled(
                &key,
                PipelineFamily::Fragment
            ));
            assert!(!RenderGraph::pipeline_family_compiled(
                &key,
                PipelineFamily::Compute
            ));
            assert!(!seen.contains(&key));
            seen.push(key);
        }
        let codes: Vec<u32> = list.iter().map(|(_, code)| *code).collect();
        assert_eq!(codes, vec![0, 1, 2, 3]);
        let Some(composite_code) = codes.first().copied() else {
            unreachable!("four slab codes");
        };
        let Some(mip_code) = codes.get(1).copied() else {
            unreachable!("four slab codes");
        };
        assert_ne!(composite_code, mip_code);
        let keys = fragment_keys_for(tier);
        assert_eq!(keys.len(), 6);
        assert_eq!(keys[0].pass, PassKind::Stack);
        assert_eq!(keys[1].pass, PassKind::VolumeRaycast);
        assert_eq!(keys[2].pass, PassKind::Slab);
    }
    let volume_mip = PipelineKey {
        pass: PassKind::VolumeRaycast,
        blend: BlendMode::Mip,
        tier: Tier::A,
    };
    let stack_mip = PipelineKey {
        pass: PassKind::Stack,
        blend: BlendMode::Mip,
        tier: Tier::A,
    };
    assert!(!RenderGraph::pipeline_compiled(&volume_mip));
    assert!(!RenderGraph::pipeline_compiled(&stack_mip));
    assert!(RenderGraph::pipeline_compiled(&PipelineKey {
        pass: PassKind::VolumeRaycast,
        blend: BlendMode::Composite,
        tier: Tier::A,
    }));
    let module = slab_module();
    let Some(entry) = module.strip_prefix(VOI_WGSL) else {
        unreachable!("the slab module starts with the VOI text");
    };
    assert!(entry.contains("override slab_blend"));
    assert_eq!(entry.matches("lut_chain(").count(), 1);
    assert!(!entry.contains("var<storage"));
    assert!(!entry.contains("@compute"));
    assert_eq!(slab_blend_code(BlendMode::Composite), Some(0));
    assert_eq!(slab_blend_code(BlendMode::Mip), Some(1));
    assert_eq!(slab_blend_code(BlendMode::MinIp), Some(2));
    assert_eq!(slab_blend_code(BlendMode::Average), Some(3));
    assert_eq!(slab_blend_code(BlendMode::LabelmapEdgeProjection), None);
}

#[test]
fn labelmap_edge_is_not_drawn_as_composite() {
    let key = PipelineKey {
        pass: PassKind::Slab,
        blend: BlendMode::LabelmapEdgeProjection,
        tier: Tier::A,
    };
    assert!(!RenderGraph::pipeline_compiled(&key));
    let viewport = ViewportId(4);
    assert_eq!(
        RenderGraph::request_outcome(&key, PipelineFamily::Fragment, viewport),
        FrameOutcome::Unavailable { viewport }
    );
    let mut called = false;
    let image = bad_extent();
    let refused = present_slab(
        Tier::A,
        true,
        BlendMode::LabelmapEdgeProjection,
        &image,
        2048,
        |_| {
            called = true;
            Ok(())
        },
    );
    assert!(matches!(refused, Err(SlabError::BlendUnavailable)));
    assert!(!called);
    let mut called = false;
    let not_a_slab = present_slab(Tier::B, false, BlendMode::Composite, &image, 2048, |_| {
        called = true;
        Ok(())
    });
    assert!(matches!(not_a_slab, Err(SlabError::BlendUnavailable)));
    assert!(!called);
}

#[test]
fn slab_is_unavailable_on_tier_c() {
    let image = bad_extent();
    let mut called = false;
    let refused = present_slab(Tier::Cpu, true, BlendMode::Composite, &image, 2048, |_| {
        called = true;
        Ok(())
    });
    assert!(matches!(refused, Err(SlabError::TierCUnavailable)));
    assert!(!called);
    let mut called = false;
    let labelmap_on_cpu = present_slab(
        Tier::Cpu,
        false,
        BlendMode::LabelmapEdgeProjection,
        &image,
        16,
        |_| {
            called = true;
            Ok(())
        },
    );
    assert!(matches!(labelmap_on_cpu, Err(SlabError::TierCUnavailable)));
    assert!(!called);
}

#[test]
fn a_slab_extent_above_the_limit_is_not_truncated() {
    let image = one_sample(PlaneSample::Present(200.0));
    let mut called = false;
    let deep = SlabImage {
        sample_count: 8,
        samples: vec![PlaneSample::Present(1.0); 8],
        ..image.clone()
    };
    let refused = present_slab(Tier::A, true, BlendMode::Mip, &deep, 7, |_| {
        called = true;
        Ok(())
    });
    assert!(matches!(refused, Err(SlabError::ExtentUnavailable)));
    assert!(!called);
    let empty = present_slab(
        Tier::B,
        true,
        BlendMode::Average,
        &bad_extent(),
        256,
        |_| {
            unreachable!("a zero extent is refused before the upload");
        },
    );
    assert!(matches!(empty, Err(SlabError::ExtentUnavailable)));
    let wide = SlabImage {
        width: 257,
        ..image
    };
    let wide = present_slab(Tier::B, true, BlendMode::Composite, &wide, 256, |_| {
        unreachable!("a width above the limit is refused before the upload");
    });
    assert!(matches!(wide, Err(SlabError::ExtentUnavailable)));
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

fn exact_chain() -> LutChain {
    let modality = ModalityTransform::new(None, Some(1.0), Some(0.0));
    let voi = VoiTransform::new(
        None,
        &[127.5],
        &[255.0],
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

fn colour_target(device: &wgpu::Device) -> (wgpu::TextureView, wgpu::TextureView, wgpu::Texture) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("slab shared target"),
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
    let stack = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let slab = texture.create_view(&wgpu::TextureViewDescriptor::default());
    (stack, slab, texture)
}

fn float_source(device: &wgpu::Device, queue: &wgpu::Queue, value: f32) -> wgpu::TextureView {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("slab stack source"),
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
    let bytes = [value];
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        bytemuck::cast_slice(&bytes),
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(4),
            rows_per_image: Some(1),
        },
        wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
    );
    texture.create_view(&wgpu::TextureViewDescriptor::default())
}

fn dummy_source(device: &wgpu::Device, queue: &wgpu::Queue) -> wgpu::TextureView {
    float_source(device, queue, 0.0)
}

fn read_pixel(context: &ocelli_render::GpuContext, target: &wgpu::Texture) -> [f32; 4] {
    let device = context.device();
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("slab readback"),
        size: 256,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("slab readback"),
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

#[test]
#[ignore = "needs a real GPU adapter, which the CI floor does not have (D-04)"]
fn one_submit_for_a_slab_and_a_stack() {
    let Some((adapter, mut context)) = session() else {
        println!("this machine resolved tier C, so there is no device");
        return;
    };
    let mut graph = match RenderGraph::new(&mut context, 2, wgpu::TextureFormat::Rgba32Float) {
        Ok(graph) => graph,
        Err(_) => return,
    };
    let chain = exact_chain();
    let voi = params(&chain);
    let (stack_target, _stack_texture) = single_target(context.device());
    let stack_source = float_source(context.device(), context.queue(), 40.0);
    let stack = match graph.register(
        &context,
        stack_target,
        stack_source,
        Pass::Stack(StackPass {
            voi,
            blend: BlendMode::Composite,
        }),
    ) {
        Ok(id) => id,
        Err(_) => unreachable!("the stack viewport fits"),
    };
    let (slab_target, _slab_texture) = single_target(context.device());
    let slab_source = dummy_source(context.device(), context.queue());
    let slab = match graph.register(
        &context,
        slab_target,
        slab_source,
        Pass::Slab(SlabPass {
            blend: BlendMode::Mip,
            voi,
        }),
    ) {
        Ok(id) => id,
        Err(_) => unreachable!("the slab viewport fits"),
    };
    assert!(
        graph
            .show_slab(
                &context,
                &adapter,
                slab,
                &one_sample(PlaneSample::Present(200.0)),
                BlendMode::Mip,
            )
            .is_ok()
    );
    let before = graph.submissions();
    assert_eq!(
        graph.tick(&context),
        FrameOutcome::Submitted {
            viewports: 2,
            passes: 2
        }
    );
    assert_eq!(graph.submissions(), before + 1);
    assert_ne!(stack, slab);
}

#[test]
#[ignore = "needs a real GPU adapter, which the CI floor does not have (D-04)"]
fn an_absent_slab_pixel_is_not_painted_zero() {
    let Some((adapter, mut context)) = session() else {
        println!("this machine resolved tier C, so there is no device");
        return;
    };
    let mut graph = match RenderGraph::new(&mut context, 2, wgpu::TextureFormat::Rgba32Float) {
        Ok(graph) => graph,
        Err(_) => return,
    };
    let chain = exact_chain();
    let voi = params(&chain);
    let (stack_view, slab_view, texture) = colour_target(context.device());
    let stack_source = float_source(context.device(), context.queue(), 200.0);
    match graph.register(
        &context,
        stack_view,
        stack_source,
        Pass::Stack(StackPass {
            voi,
            blend: BlendMode::Composite,
        }),
    ) {
        Ok(_) => {}
        Err(_) => unreachable!("the stack viewport fits"),
    };
    let slab_source = dummy_source(context.device(), context.queue());
    let slab = match graph.register(
        &context,
        slab_view,
        slab_source,
        Pass::Slab(SlabPass {
            blend: BlendMode::Composite,
            voi,
        }),
    ) {
        Ok(id) => id,
        Err(_) => unreachable!("the slab viewport fits"),
    };
    assert_eq!(
        graph.tick(&context),
        FrameOutcome::Submitted {
            viewports: 1,
            passes: 1
        }
    );
    let first = read_pixel(&context, &texture);
    let expected = 200.0_f32 / 255.0;
    assert_ne!(first[0].to_bits(), 0);
    assert!(
        within_two_ulp(first[0], expected),
        "red {got} is not within 2 ULP of {expected}",
        got = first[0]
    );
    assert_eq!(first[1].to_bits(), first[0].to_bits());
    assert_eq!(first[2].to_bits(), first[0].to_bits());
    assert_eq!(first[3].to_bits(), 1.0_f32.to_bits());
    assert!(
        graph
            .show_slab(
                &context,
                &adapter,
                slab,
                &one_sample(PlaneSample::Absent),
                BlendMode::Composite,
            )
            .is_ok()
    );
    assert_eq!(
        graph.tick(&context),
        FrameOutcome::Submitted {
            viewports: 1,
            passes: 1
        }
    );
    let second = read_pixel(&context, &texture);
    assert_eq!(second[0].to_bits(), first[0].to_bits());
    assert_eq!(second[1].to_bits(), first[1].to_bits());
    assert_eq!(second[2].to_bits(), first[2].to_bits());
    assert_eq!(second[3].to_bits(), first[3].to_bits());
}

#[test]
#[ignore = "needs a real GPU adapter, which the CI floor does not have (D-04)"]
fn slab_blend_readback_follows_one_voi_after_the_stored_reduction() {
    let Some((adapter, mut context)) = session() else {
        println!("this machine resolved tier C, so there is no device");
        return;
    };
    let mut graph = match RenderGraph::new(&mut context, 1, wgpu::TextureFormat::Rgba32Float) {
        Ok(graph) => graph,
        Err(_) => return,
    };
    let chain = exact_chain();
    let voi = params(&chain);
    let (view, texture) = single_target(context.device());
    let source = dummy_source(context.device(), context.queue());
    let slab = match graph.register(
        &context,
        view,
        source,
        Pass::Slab(SlabPass {
            blend: BlendMode::Composite,
            voi,
        }),
    ) {
        Ok(id) => id,
        Err(_) => unreachable!("the slab viewport fits"),
    };
    // Centre 30, not the maximum. Stored 300 is above the window, so one
    // VOI after the stored mean is 110 and the mean of the three windows
    // is 95. PS3.3 C.11.2.1.3.2. Composite and MIP still disagree.
    let image = SlabImage {
        width: 1,
        height: 1,
        step_mm: 2.5,
        index_step: [0.0, 0.0, 1.0],
        sample_count: 3,
        samples: vec![
            PlaneSample::Present(0.0),
            PlaneSample::Present(30.0),
            PlaneSample::Present(300.0),
        ],
    };
    let mut draw = |blend, expected: f32| {
        assert!(
            graph
                .show_slab(&context, &adapter, slab, &image, blend)
                .is_ok()
        );
        assert_eq!(
            graph.tick(&context),
            FrameOutcome::Submitted {
                viewports: 1,
                passes: 1
            }
        );
        let pixel = read_pixel(&context, &texture);
        let gap = (pixel[0] - expected).abs();
        // Composite, MIP, MinIP and the reduce-then-VOI average are separated
        // by more than 0.05. The other average, 95/255, is also that far
        // from 110/255. The f32 window then the divide by 255 moves a colour
        // by far less than 0.0001.
        assert!(
            gap < 1.0e-4,
            "{blend:?} red {got} is not within 0.0001 of {expected}",
            got = pixel[0]
        );
        assert_eq!(pixel[3].to_bits(), 1.0_f32.to_bits());
    };
    draw(BlendMode::Composite, 30.0 / 255.0);
    draw(BlendMode::Mip, 1.0);
    draw(BlendMode::MinIp, 0.0);
    draw(BlendMode::Average, 110.0 / 255.0);
    let averaged = read_pixel(&context, &texture);
    let voi_then_reduce = 95.0_f32 / 255.0;
    assert!(
        (averaged[0] - voi_then_reduce).abs() > 1.0e-2,
        "average red {got} matches a window of each sample",
        got = averaged[0]
    );
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

fn single_target(device: &wgpu::Device) -> (wgpu::TextureView, wgpu::Texture) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("slab private target"),
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
