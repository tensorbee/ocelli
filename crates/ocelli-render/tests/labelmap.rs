//! F-063 composite labelmap.
//!
//! The floor row checks which blend compiles. The device row is ignored and
//! runs under the gpu gate. Its expected bytes are the same hand fixture as
//! `ocelli-seg`: a 3 by 3 segment 1, fill `(10, 20, 30, 128)` at the centre
//! and outline `(10, 20, 30, 255)` on the boundary. DICOM PS3.3 C.8.20.2
//! makes 0 background. The outline alpha is the F-063 plan.

use ocelli_pixel::{
    LutChain, ModalityTransform, PhotometricInterpretation, PresentationLutEvidence, VoiFunction,
    VoiTransform,
};
use ocelli_render::{
    BlendMode, Dirty, FrameOutcome, GraphError, LabelmapDrawError, Pass, PassKind, PipelineKey,
    RenderGraph, SegPass, StackPass, Tier, ViewportId, VoiParams,
};
use ocelli_seg::{LabelColourLut, LabelGrid, LabelStyle, LabelmapError, Rgba8, present_labelmap};

#[test]
fn seg_overlay_compiles_composite_only() {
    let composite = PipelineKey {
        pass: PassKind::SegOverlay,
        blend: BlendMode::Composite,
        tier: Tier::A,
    };
    let edge = PipelineKey {
        pass: PassKind::SegOverlay,
        blend: BlendMode::LabelmapEdgeProjection,
        tier: Tier::A,
    };
    let slab_edge = PipelineKey {
        pass: PassKind::Slab,
        blend: BlendMode::LabelmapEdgeProjection,
        tier: Tier::B,
    };
    assert!(RenderGraph::pipeline_compiled(&composite));
    assert!(!RenderGraph::pipeline_compiled(&edge));
    assert!(!RenderGraph::pipeline_compiled(&slab_edge));
    assert!(!RenderGraph::pipeline_compiled(&PipelineKey {
        blend: BlendMode::Mip,
        ..composite
    }));
}

fn segment_one() -> Rgba8 {
    Rgba8 {
        r: 10,
        g: 20,
        b: 30,
        a: 128,
    }
}

fn grid_all_one() -> LabelGrid {
    LabelGrid {
        width: 3,
        height: 3,
        indices: vec![1, 1, 1, 1, 1, 1, 1, 1, 1],
    }
}

fn style_on() -> LabelStyle {
    LabelStyle {
        fill: true,
        outline: true,
    }
}

fn presented(lut: &LabelColourLut) -> Vec<u8> {
    let mut out = vec![0_u8; 36];
    let painted = present_labelmap(&grid_all_one(), lut, style_on(), &mut out);
    assert!(painted.is_ok());
    out
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

fn chain() -> Option<LutChain> {
    let modality = ModalityTransform::new(None, Some(1.0), Some(0.0)).ok()?;
    let voi =
        VoiTransform::new(None, &[40.0], &[400.0], 0, VoiFunction::Linear, 0.0, 255.0).ok()?;
    LutChain::new(
        modality,
        voi,
        PhotometricInterpretation::Monochrome2,
        PresentationLutEvidence::Absent,
    )
    .ok()
}

fn overlay_target(
    context: &ocelli_render::GpuContext,
) -> (wgpu::TextureView, wgpu::TextureView, wgpu::Texture) {
    let device = context.device();
    let source = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("labelmap source"),
        size: wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::R32Float,
        usage: wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("labelmap target"),
        size: wgpu::Extent3d {
            width: 3,
            height: 3,
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

fn readback_plane(
    context: &ocelli_render::GpuContext,
    target: &wgpu::Texture,
    width: u32,
    height: u32,
) -> Vec<u8> {
    let stride = 256_u32;
    let bytes = u64::from(stride).saturating_mul(u64::from(height));
    let device = context.device();
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("labelmap readback"),
        size: bytes,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("labelmap readback"),
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
                bytes_per_row: Some(stride),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d {
            width,
            height,
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
    let row = usize::try_from(width.saturating_mul(4)).unwrap_or(0);
    let stride_us = usize::try_from(stride).unwrap_or(0);
    let mut out = Vec::new();
    let rows = usize::try_from(height).unwrap_or(0);
    for y in 0..rows {
        let start = y.saturating_mul(stride_us);
        let end = start.saturating_add(row);
        if let Some(piece) = data.get(start..end) {
            out.extend_from_slice(piece);
        }
    }
    out
}

#[test]
#[ignore = "needs a real adapter, run by the gpu gate"]
fn shown_labelmap_matches_the_presenter() {
    let Some(mut context) = context() else {
        return;
    };
    let mut graph = match RenderGraph::new(&mut context, 2, wgpu::TextureFormat::Rgba8Unorm) {
        Ok(graph) => graph,
        Err(_) => return,
    };
    let (source, target_view, target) = overlay_target(&context);
    let id = match graph.register(
        &context,
        target_view,
        source,
        Pass::SegOverlay(SegPass {
            blend: BlendMode::Composite,
        }),
    ) {
        Ok(id) => id,
        Err(_) => return,
    };
    assert_eq!(
        graph.tick(&context),
        FrameOutcome::Unavailable {
            viewport: ViewportId(0)
        }
    );
    assert_eq!(graph.submissions(), 0);
    let lut = LabelColourLut::from_segment_one(vec![segment_one()]);
    let shown = graph.show_labelmap(&context, id, &grid_all_one(), &lut, style_on());
    assert!(shown.is_ok());
    assert_eq!(
        graph.tick(&context),
        FrameOutcome::Submitted {
            viewports: 1,
            passes: 1
        }
    );
    let expected = presented(&lut);
    // Centre offset 16 is fill alpha 128. An edge byte is outline alpha 255.
    assert_eq!(expected.get(16..20), Some(&[10_u8, 20, 30, 128][..]));
    assert_eq!(expected.get(0..4), Some(&[10_u8, 20, 30, 255][..]));
    assert_eq!(readback_plane(&context, &target, 3, 3), expected);
    let Some(voi) = chain() else {
        return;
    };
    assert!(matches!(
        graph.set_voi(&context, id, &voi),
        Err(GraphError::PassHasNoUniform)
    ));
    let before = graph.submissions();
    let again = graph.show_labelmap(&context, id, &grid_all_one(), &lut, style_on());
    assert!(again.is_ok());
    assert_eq!(graph.dirty(id), Ok(Dirty::default()));
    assert_eq!(graph.tick(&context), FrameOutcome::Noop);
    assert_eq!(graph.submissions(), before);
    let next = LabelColourLut::from_segment_one(vec![Rgba8 {
        r: 1,
        g: 2,
        b: 3,
        a: 64,
    }]);
    assert!(
        graph
            .show_labelmap(&context, id, &grid_all_one(), &next, style_on())
            .is_ok()
    );
    assert_eq!(
        graph.tick(&context),
        FrameOutcome::Submitted {
            viewports: 1,
            passes: 1
        }
    );
    assert_eq!(readback_plane(&context, &target, 3, 3), presented(&next));
}

#[test]
#[ignore = "needs a real adapter, run by the gpu gate"]
fn labelmap_edge_and_a_foreign_pass_upload_nothing() {
    let Some(mut context) = context() else {
        return;
    };
    let mut graph = match RenderGraph::new(&mut context, 3, wgpu::TextureFormat::Rgba8Unorm) {
        Ok(graph) => graph,
        Err(_) => return,
    };
    let (source, target_view, _target) = overlay_target(&context);
    let edge = match graph.register(
        &context,
        target_view,
        source,
        Pass::SegOverlay(SegPass {
            blend: BlendMode::LabelmapEdgeProjection,
        }),
    ) {
        Ok(id) => id,
        Err(_) => return,
    };
    let lut = LabelColourLut::from_segment_one(vec![segment_one()]);
    assert_eq!(
        graph.show_labelmap(&context, edge, &grid_all_one(), &lut, style_on()),
        Err(LabelmapDrawError::BlendUnavailable)
    );
    let (source, target_view, _target) = overlay_target(&context);
    let missing = LabelGrid {
        width: 1,
        height: 1,
        indices: vec![2],
    };
    let composite = match graph.register(
        &context,
        target_view,
        source,
        Pass::SegOverlay(SegPass {
            blend: BlendMode::Composite,
        }),
    ) {
        Ok(id) => id,
        Err(_) => return,
    };
    assert_eq!(
        graph.show_labelmap(&context, composite, &missing, &lut, style_on()),
        Err(LabelmapDrawError::Present(LabelmapError::MissingColour))
    );
    let (source, target_view, _target) = overlay_target(&context);
    let voi = VoiParams {
        center: 40.0,
        width: 400.0,
        slope: 1.0,
        intercept: 0.0,
        ymin: 0.0,
        ymax: 255.0,
        fn_kind: 0,
        invert: 0,
    };
    let stack = match graph.register(
        &context,
        target_view,
        source,
        Pass::Stack(StackPass {
            voi,
            blend: BlendMode::Composite,
        }),
    ) {
        Ok(id) => id,
        Err(_) => return,
    };
    assert_eq!(
        graph.show_labelmap(&context, stack, &grid_all_one(), &lut, style_on()),
        Err(LabelmapDrawError::NotOverlay)
    );
    assert_eq!(
        graph.show_labelmap(
            &context,
            composite,
            &grid_all_one(),
            &lut,
            LabelStyle {
                fill: false,
                outline: false,
            },
        ),
        Err(LabelmapDrawError::Present(LabelmapError::NothingToDraw))
    );
    assert_eq!(
        graph.show_labelmap(&context, ViewportId(9), &grid_all_one(), &lut, style_on()),
        Err(LabelmapDrawError::UnknownViewport)
    );
    assert_eq!(graph.submissions(), 0);
    // The stack still encodes. The edge blend and the missing colour did not.
    assert_eq!(
        graph.tick(&context),
        FrameOutcome::Submitted {
            viewports: 1,
            passes: 1
        }
    );
}
