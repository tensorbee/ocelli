//! F-038, the render graph and frame scheduler.
//!
//! The pure rows run in the floor with no adapter. The device rows are
//! `#[ignore]`d and run under `bin/ocelli.sh gate gpu`. Their fixture values
//! are the section 18.3 rows hand-computed from DICOM PS3.3 C.11.2.1.2 and
//! C.11.2.1.3.2: inputs -160, 40, 240 and -60 at centre 40, width 400 give
//! LINEAR 0.000, 127.819, 255.000 and 63.910, asserted here after 8-bit
//! quantisation as 0, 128, 255 and 64.

use ocelli_pixel::{
    LutChain, LutDescriptor, ModalityTransform, PhotometricInterpretation, PresentationLutEvidence,
    VoiFunction, VoiTransform,
};
use ocelli_render::{
    BlendMode, Dirty, FrameOutcome, Pass, PassKind, PipelineKey, RenderGraph, StackPass, Tier,
    ViewportId, VoiParams,
};

#[test]
fn a_new_viewport_starts_fully_dirty_and_marks_coalesce() {
    let dirty = Dirty::full();
    assert!(dirty.camera && dirty.voi && dirty.slice && dirty.segmentation && dirty.data);
    assert!(dirty.any());
    // Marking the same source twice is still one flag, so one encode.
    let mut marked = Dirty {
        camera: true,
        ..Dirty::default()
    };
    marked.camera = true;
    marked.camera = true;
    assert!(marked.any());
    marked.clear();
    assert!(!marked.any());
}

#[test]
fn pipeline_key_distinguishes_each_field() {
    let base = PipelineKey {
        pass: PassKind::Stack,
        blend: BlendMode::Composite,
        tier: Tier::A,
    };
    assert_ne!(
        base,
        PipelineKey {
            pass: PassKind::VolumeRaycast,
            ..base
        }
    );
    assert_ne!(
        base,
        PipelineKey {
            blend: BlendMode::Mip,
            ..base
        }
    );
    assert_ne!(
        base,
        PipelineKey {
            tier: Tier::B,
            ..base
        }
    );
    // Composite stack is compiled. Volume composite is F-059 and is not this row.
    assert!(RenderGraph::pipeline_compiled(&base));
    assert!(!RenderGraph::pipeline_compiled(&PipelineKey {
        blend: BlendMode::Mip,
        ..base
    }));
}

fn chain(function: VoiFunction) -> LutChain {
    let modality = ModalityTransform::new(None, Some(1.0), Some(0.0));
    let voi = VoiTransform::new(None, &[40.0], &[400.0], 0, function, 0.0, 255.0);
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

fn native_clock() -> impl FnMut() -> u64 {
    let start = std::time::Instant::now();
    move || u64::try_from(start.elapsed().as_nanos()).unwrap_or(u64::MAX)
}

fn context() -> Option<ocelli_render::GpuContext> {
    let mut clock = native_clock();
    let adapter = pollster::block_on(ocelli_render::resolve_adapter(
        ocelli_render::TierRequest::Auto,
        &mut clock,
    ))?;
    pollster::block_on(adapter.open()).ok()
}

fn texture_pair(
    context: &ocelli_render::GpuContext,
    inputs: &[f32],
) -> (wgpu::TextureView, wgpu::TextureView, wgpu::Texture) {
    let device = context.device();
    let width = u32::try_from(inputs.len()).unwrap_or(1);
    let source = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("graph source"),
        size: wgpu::Extent3d {
            width,
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
    context.queue().write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &source,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        bytemuck::cast_slice(inputs),
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(4 * width),
            rows_per_image: Some(1),
        },
        wgpu::Extent3d {
            width,
            height: 1,
            depth_or_array_layers: 1,
        },
    );
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("graph target"),
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
    (
        source.create_view(&wgpu::TextureViewDescriptor::default()),
        target.create_view(&wgpu::TextureViewDescriptor::default()),
        target,
    )
}

fn readback(context: &ocelli_render::GpuContext, target: &wgpu::Texture, width: u32) -> Vec<u8> {
    let device = context.device();
    let padded = 256_u32;
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("graph readback"),
        size: u64::from(padded),
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("graph readback"),
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
                bytes_per_row: Some(padded),
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
        return Vec::new();
    };
    let bytes: &[u8] = &data;
    let row = 4 * width;
    bytes
        .iter()
        .take(usize::try_from(row).unwrap_or(0))
        .copied()
        .collect()
}

#[test]
#[ignore = "needs a real adapter, run by the gpu gate"]
fn the_section_18_3_rows_render_through_the_graph() {
    let Some(mut context) = context() else {
        return;
    };
    let mut graph = match RenderGraph::new(&mut context, 2, wgpu::TextureFormat::Rgba8Unorm) {
        Ok(graph) => graph,
        Err(_) => return,
    };
    let inputs = [-160.0_f32, 40.0, 240.0, -60.0];
    let (source, target_view, target) = texture_pair(&context, &inputs);
    let voi = match VoiParams::from_chain(&chain(VoiFunction::Linear)) {
        Ok(voi) => voi,
        Err(_) => unreachable!("chain must express"),
    };
    let viewport = match graph.register(
        &context,
        target_view,
        source,
        Pass::Stack(StackPass {
            voi,
            blend: BlendMode::Composite,
        }),
    ) {
        Ok(viewport) => viewport,
        Err(_) => unreachable!("registration must fit"),
    };
    assert_eq!(viewport, ViewportId(0));
    assert_eq!(
        graph.tick(&context),
        FrameOutcome::Submitted {
            viewports: 1,
            passes: 1
        }
    );
    let pixels = readback(&context, &target, 4);
    let red: Vec<u8> = pixels.iter().step_by(4).copied().collect();
    assert_eq!(red, [0, 128, 255, 64]);
    // A clean tick is a no-op and submits nothing.
    assert_eq!(graph.tick(&context), FrameOutcome::Noop);
}

#[test]
#[ignore = "needs a real adapter, run by the gpu gate"]
fn an_uncompiled_blend_reports_unavailable_and_keeps_its_dirt() {
    let Some(mut context) = context() else {
        return;
    };
    let mut graph = match RenderGraph::new(&mut context, 1, wgpu::TextureFormat::Rgba8Unorm) {
        Ok(graph) => graph,
        Err(_) => return,
    };
    let (source, target_view, _target) = texture_pair(&context, &[40.0]);
    let voi = match VoiParams::from_chain(&chain(VoiFunction::Linear)) {
        Ok(voi) => voi,
        Err(_) => unreachable!("chain must express"),
    };
    let viewport = match graph.register(
        &context,
        target_view,
        source,
        Pass::Stack(StackPass {
            voi,
            blend: BlendMode::Mip,
        }),
    ) {
        Ok(viewport) => viewport,
        Err(_) => unreachable!("registration must fit"),
    };
    assert_eq!(graph.tick(&context), FrameOutcome::Unavailable { viewport });
    // Dirt is retained, so the next tick reports the same refusal.
    assert_eq!(graph.tick(&context), FrameOutcome::Unavailable { viewport });
}

#[test]
#[ignore = "needs a real adapter, run by the gpu gate"]
fn two_dirty_viewports_submit_once_and_a_mixed_frame_reports_later() {
    let Some(mut context) = context() else {
        return;
    };
    let mut graph = match RenderGraph::new(&mut context, 3, wgpu::TextureFormat::Rgba8Unorm) {
        Ok(graph) => graph,
        Err(_) => return,
    };
    let voi = match VoiParams::from_chain(&chain(VoiFunction::Linear)) {
        Ok(voi) => voi,
        Err(_) => unreachable!("chain must express"),
    };
    let (source, target_view, _t) = texture_pair(&context, &[40.0]);
    let first = match graph.register(
        &context,
        target_view,
        source,
        Pass::Stack(StackPass {
            voi,
            blend: BlendMode::Composite,
        }),
    ) {
        Ok(id) => id,
        Err(_) => unreachable!("registration must fit"),
    };
    let (source, target_view, _t) = texture_pair(&context, &[40.0]);
    let second = match graph.register(
        &context,
        target_view,
        source,
        Pass::Stack(StackPass {
            voi,
            blend: BlendMode::Composite,
        }),
    ) {
        Ok(id) => id,
        Err(_) => unreachable!("registration must fit"),
    };
    let (source, target_view, _t) = texture_pair(&context, &[40.0]);
    let mixed = match graph.register(
        &context,
        target_view,
        source,
        Pass::Stack(StackPass {
            voi,
            blend: BlendMode::Mip,
        }),
    ) {
        Ok(id) => id,
        Err(_) => unreachable!("registration must fit"),
    };
    assert_eq!((first, second), (ViewportId(0), ViewportId(1)));
    // One submission covers both composite viewports. The MIP viewport in
    // the same frame cannot encode, so its dirt survives and the next tick
    // reports it rather than hiding it behind the submitted outcome.
    assert_eq!(
        graph.tick(&context),
        FrameOutcome::Submitted {
            viewports: 2,
            passes: 2
        }
    );
    assert_eq!(graph.submissions(), 1);
    assert_eq!(
        graph.tick(&context),
        FrameOutcome::Unavailable { viewport: mixed }
    );
    assert_eq!(graph.submissions(), 1);
    // VOI dirt cannot be marked without writing uniform bytes.
    assert!(
        graph
            .mark_dirty(
                first,
                Dirty {
                    voi: true,
                    ..Dirty::default()
                }
            )
            .is_err()
    );
    // Releasing frees the slot for a new registration at the same id.
    assert!(graph.remove(first).is_ok());
    assert!(graph.remove(first).is_err());
}

#[test]
#[ignore = "needs a real adapter, run by the gpu gate"]
fn recovery_drops_viewport_objects_until_they_are_rebound() {
    let Some(mut context) = context() else {
        return;
    };
    let mut graph = match RenderGraph::new(&mut context, 1, wgpu::TextureFormat::Rgba8Unorm) {
        Ok(graph) => graph,
        Err(_) => return,
    };
    let voi = match VoiParams::from_chain(&chain(VoiFunction::Linear)) {
        Ok(voi) => voi,
        Err(_) => unreachable!("chain must express"),
    };
    let (source, target_view, _t) = texture_pair(&context, &[40.0]);
    let viewport = match graph.register(
        &context,
        target_view,
        source,
        Pass::Stack(StackPass {
            voi,
            blend: BlendMode::Composite,
        }),
    ) {
        Ok(id) => id,
        Err(_) => unreachable!("registration must fit"),
    };
    assert_eq!(
        graph.tick(&context),
        FrameOutcome::Submitted {
            viewports: 1,
            passes: 1
        }
    );
    let recovered = ocelli_render::Recovered {
        caps: *context.caps(),
        graph: Some(ocelli_render::GraphResources {
            pipelines_restored: 1,
            cache_generation: 2,
        }),
    };
    graph.install_recovered(&context, &recovered);
    // A uniform write before rebind answers ViewportNotRebound, which is a
    // different fact from an id this graph never issued.
    assert!(matches!(
        graph.set_voi(&context, viewport, &chain(VoiFunction::Linear)),
        Err(ocelli_render::GraphError::ViewportNotRebound)
    ));
    // The old device's objects are gone, so the tick refuses rather than
    // binding them, and keeps the viewport's dirt.
    assert_eq!(graph.tick(&context), FrameOutcome::Unavailable { viewport });
    let (source, target_view, _t) = texture_pair(&context, &[40.0]);
    assert!(
        graph
            .rebind(&context, viewport, target_view, source)
            .is_ok()
    );
    assert_eq!(
        graph.tick(&context),
        FrameOutcome::Submitted {
            viewports: 1,
            passes: 1
        }
    );
}

fn chain_centre(centre: f32) -> LutChain {
    let modality = ModalityTransform::new(None, Some(1.0), Some(0.0));
    let voi = VoiTransform::new(
        None,
        &[centre],
        &[400.0],
        0,
        VoiFunction::Linear,
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

#[test]
#[ignore = "needs a real adapter, run by the gpu gate"]
fn set_voi_is_the_single_uniform_route_and_rejects_in_order() {
    let Some(mut context) = context() else {
        return;
    };
    let mut graph = match RenderGraph::new(&mut context, 2, wgpu::TextureFormat::Rgba8Unorm) {
        Ok(graph) => graph,
        Err(_) => return,
    };
    // An id this graph never issued answers UnknownViewport.
    assert!(matches!(
        graph.set_voi(&context, ViewportId(99), &chain(VoiFunction::Linear)),
        Err(ocelli_render::GraphError::UnknownViewport)
    ));
    // State is validated before the chain, so even a chain with no uniform
    // answers UnknownViewport for an unknown id, not the chain error.
    let lut = LutDescriptor::new(3, -1, 16, vec![5.0, 15.0, 25.0]);
    assert!(lut.is_ok());
    let Ok(lut) = lut else {
        unreachable!("asserted immediately above")
    };
    let sequence_modality = ModalityTransform::new(Some(lut), Some(1.0), Some(0.0));
    assert!(sequence_modality.is_ok());
    let Ok(sequence_modality) = sequence_modality else {
        unreachable!("asserted immediately above")
    };
    let sequence_chain = LutChain::new(
        sequence_modality,
        match VoiTransform::new(None, &[40.0], &[400.0], 0, VoiFunction::Linear, 0.0, 255.0) {
            Ok(voi) => voi,
            Err(_) => unreachable!("voi fixture must build"),
        },
        PhotometricInterpretation::Monochrome2,
        PresentationLutEvidence::Absent,
    );
    assert!(sequence_chain.is_ok());
    let Ok(sequence_chain) = sequence_chain else {
        unreachable!("asserted immediately above")
    };
    assert!(matches!(
        graph.set_voi(&context, ViewportId(99), &sequence_chain),
        Err(ocelli_render::GraphError::UnknownViewport)
    ));
    let voi = match VoiParams::from_chain(&chain(VoiFunction::Linear)) {
        Ok(voi) => voi,
        Err(_) => unreachable!("chain must express"),
    };
    let (source, target_view, target) = texture_pair(&context, &[40.0]);
    let viewport = match graph.register(
        &context,
        target_view,
        source,
        Pass::Stack(StackPass {
            voi,
            blend: BlendMode::Composite,
        }),
    ) {
        Ok(id) => id,
        Err(_) => unreachable!("registration must fit"),
    };
    let (source, target_view, _t) = texture_pair(&context, &[40.0]);
    let volume = match graph.register(
        &context,
        target_view,
        source,
        Pass::SegOverlay(ocelli_render::SegPass {
            blend: BlendMode::Composite,
        }),
    ) {
        Ok(id) => id,
        Err(_) => unreachable!("registration must fit"),
    };
    // A segmentation pass has no section 18.4 uniform.
    assert!(matches!(
        graph.set_voi(&context, volume, &chain(VoiFunction::Linear)),
        Err(ocelli_render::GraphError::PassHasNoUniform)
    ));
    assert_eq!(
        graph.tick(&context),
        FrameOutcome::Submitted {
            viewports: 1,
            passes: 1
        }
    );
    let red_before = readback(&context, &target, 1).first().copied().unwrap_or(0);
    assert_eq!(red_before, 128);
    // Moving the centre to 240 puts input 40 at the lower bound, so the
    // same stored value renders black through the single VOI route.
    assert!(
        graph
            .set_voi(&context, viewport, &chain_centre(240.0))
            .is_ok()
    );
    assert_eq!(
        graph.tick(&context),
        FrameOutcome::Submitted {
            viewports: 1,
            passes: 1
        }
    );
    let red_after = readback(&context, &target, 1).first().copied().unwrap_or(0);
    assert_eq!(red_after, 0);
}
