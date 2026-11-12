//! F-066 representation registry.
//!
//! The device rows are ignored and run under the gpu gate. A new composite
//! overlay starts as labelmap. A stack, a slab and a volume refuse the setter.
//! Changing the choice drops the overlay and does not upload.

use ocelli_core::Pt;
use ocelli_render::{
    BlendMode, ContourDrawError, ContourView, Dirty, FrameOutcome, GraphError, GraphResources,
    LabelmapDrawError, Pass, RayAxis, Recovered, RenderGraph, SegPass, SlabPass, StackPass,
    StoredRoute, SurfaceDrawError, SurfaceView, ViewportId, VoiParams, VolumePass,
};
use ocelli_seg::{
    Contour, ContourGeometricType, LabelColourLut, LabelGrid, LabelStyle, PlaneFrame,
    Representation, Rgba8, SurfaceMesh, present_contour,
};

fn colour() -> Rgba8 {
    Rgba8 {
        r: 10,
        g: 20,
        b: 30,
        a: 128,
    }
}

fn voi() -> VoiParams {
    VoiParams {
        center: 0.0,
        width: 1.0,
        slope: 1.0,
        intercept: 0.0,
        ymin: 0.0,
        ymax: 255.0,
        fn_kind: 0,
        invert: 0,
    }
}

fn lut() -> LabelColourLut {
    LabelColourLut::from_segment_one(vec![colour()])
}

fn grid() -> LabelGrid {
    LabelGrid {
        width: 1,
        height: 1,
        indices: vec![1],
    }
}

fn bad_grid() -> LabelGrid {
    LabelGrid {
        width: 1,
        height: 1,
        indices: vec![2],
    }
}

fn style() -> LabelStyle {
    LabelStyle {
        fill: true,
        outline: false,
    }
}

fn frame() -> PlaneFrame {
    PlaneFrame {
        origin: Pt::new(0.0, 0.0, 0.0),
        spacing_x_mm: 1.0,
        spacing_y_mm: 1.0,
    }
}

fn line() -> Contour {
    Contour {
        segment: 1,
        kind: ContourGeometricType::OpenPlanar,
        points: vec![Pt::new(0.0, 0.0, 0.0), Pt::new(3.0, 0.0, 0.0)],
    }
}

fn contour_view() -> ContourView {
    ContourView {
        width: 4,
        height: 2,
    }
}

fn mesh() -> SurfaceMesh {
    SurfaceMesh {
        segment: 1,
        vertices: vec![
            Pt::new(0.0, 0.0, 0.0),
            Pt::new(4.0, 0.0, 0.0),
            Pt::new(0.0, 4.0, 0.0),
        ],
        triangles: vec![[9, 1, 2]],
    }
}

fn surface_view() -> SurfaceView {
    SurfaceView {
        width: 5,
        height: 5,
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

fn graph(context: &mut ocelli_render::GpuContext) -> Option<RenderGraph> {
    RenderGraph::new(context, 8, wgpu::TextureFormat::Rgba8Unorm).ok()
}

fn target(
    context: &ocelli_render::GpuContext,
    width: u32,
    height: u32,
) -> (wgpu::TextureView, wgpu::TextureView, wgpu::Texture) {
    let device = context.device();
    let source = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("registry source"),
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
    let image = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("registry target"),
        size: wgpu::Extent3d {
            width,
            height,
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
        image.create_view(&wgpu::TextureViewDescriptor::default()),
        image,
    )
}

fn readback(
    context: &ocelli_render::GpuContext,
    image: &wgpu::Texture,
    width: u32,
    height: u32,
) -> Vec<u8> {
    let stride = 256_u32;
    let bytes = u64::from(stride).saturating_mul(u64::from(height));
    let device = context.device();
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("registry readback"),
        size: bytes,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("registry readback"),
    });
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: image,
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

fn overlay(context: &ocelli_render::GpuContext, graph: &mut RenderGraph) -> Option<ViewportId> {
    let (source, target_view, _image) = target(context, 4, 2);
    graph
        .register(
            context,
            target_view,
            source,
            Pass::SegOverlay(SegPass {
                blend: BlendMode::Composite,
            }),
            StoredRoute::PreApplied,
        )
        .ok()
}

fn presented_contour() -> Vec<u8> {
    let mut out = vec![0_u8; 32];
    let painted = present_contour(&line(), &frame(), &lut(), 4, 2, &mut out);
    assert!(painted.is_ok());
    out
}

#[test]
#[ignore = "needs a real adapter, run by the gpu gate"]
fn a_new_overlay_starts_as_labelmap() {
    let Some(mut context) = context() else {
        return;
    };
    let Some(mut graph) = graph(&mut context) else {
        return;
    };
    let Some(id) = overlay(&context, &mut graph) else {
        return;
    };
    assert_eq!(graph.representation(id), Ok(Representation::Labelmap));
    assert_eq!(
        graph.representation(ViewportId(9)),
        Err(GraphError::UnknownViewport)
    );
    assert_eq!(
        graph.set_representation(ViewportId(9), Representation::Contour),
        Err(GraphError::UnknownViewport)
    );
}

#[test]
#[ignore = "needs a real adapter, run by the gpu gate"]
fn a_stack_has_no_representation() {
    let Some(mut context) = context() else {
        return;
    };
    let Some(mut graph) = graph(&mut context) else {
        return;
    };
    let (source, target_view, _image) = target(&context, 1, 1);
    let Ok(stack) = graph.register(
        &context,
        target_view,
        source,
        Pass::Stack(StackPass {
            voi: voi(),
            blend: BlendMode::Composite,
        }),
        StoredRoute::PreApplied,
    ) else {
        return;
    };
    assert_eq!(
        graph.set_representation(stack, Representation::Contour),
        Err(GraphError::NotOverlay)
    );
    assert_eq!(graph.representation(stack), Err(GraphError::NotOverlay));
    assert_eq!(
        graph.tick(&context),
        FrameOutcome::Submitted {
            viewports: 1,
            passes: 1
        }
    );
    let (source, target_view, _image) = target(&context, 1, 1);
    let Ok(volume) = graph.register(
        &context,
        target_view,
        source,
        Pass::VolumeRaycast(VolumePass {
            blend: BlendMode::Composite,
            voi: voi(),
            axis: RayAxis::Slice,
            from_far: false,
        }),
        StoredRoute::PreApplied,
    ) else {
        return;
    };
    assert_eq!(
        graph.set_representation(volume, Representation::Surface),
        Err(GraphError::NotOverlay)
    );
    let (source, target_view, _image) = target(&context, 1, 1);
    let Ok(slab) = graph.register(
        &context,
        target_view,
        source,
        Pass::Slab(SlabPass {
            blend: BlendMode::Composite,
            voi: voi(),
        }),
        StoredRoute::PreApplied,
    ) else {
        return;
    };
    assert_eq!(
        graph.set_representation(slab, Representation::Labelmap),
        Err(GraphError::NotOverlay)
    );
}

#[test]
#[ignore = "needs a real adapter, run by the gpu gate"]
fn switching_drops_the_overlay_without_upload() {
    let Some(mut context) = context() else {
        return;
    };
    let Some(mut graph) = graph(&mut context) else {
        return;
    };
    let Some(id) = overlay(&context, &mut graph) else {
        return;
    };
    let colours = lut();
    assert!(
        graph
            .show_labelmap(&context, id, &grid(), &colours, style())
            .is_ok()
    );
    assert_eq!(
        graph.tick(&context),
        FrameOutcome::Submitted {
            viewports: 1,
            passes: 1
        }
    );
    let before = graph.submissions();
    assert_eq!(
        graph.set_representation(id, Representation::Contour),
        Ok(())
    );
    assert_eq!(graph.representation(id), Ok(Representation::Contour));
    assert_eq!(
        graph.tick(&context),
        FrameOutcome::Unavailable { viewport: id }
    );
    assert_eq!(graph.submissions(), before);
    assert_eq!(
        graph.show_labelmap(&context, id, &bad_grid(), &colours, style()),
        Err(LabelmapDrawError::RepresentationMismatch)
    );
    assert_eq!(graph.submissions(), before);
}

#[test]
#[ignore = "needs a real adapter, run by the gpu gate"]
fn the_matching_show_uploads() {
    let Some(mut context) = context() else {
        return;
    };
    let Some(mut graph) = graph(&mut context) else {
        return;
    };
    let (source, target_view, image) = target(&context, 4, 2);
    let Ok(id) = graph.register(
        &context,
        target_view,
        source,
        Pass::SegOverlay(SegPass {
            blend: BlendMode::Composite,
        }),
        StoredRoute::PreApplied,
    ) else {
        return;
    };
    let colours = lut();
    assert!(
        graph
            .show_labelmap(&context, id, &grid(), &colours, style())
            .is_ok()
    );
    assert_eq!(
        graph.tick(&context),
        FrameOutcome::Submitted {
            viewports: 1,
            passes: 1
        }
    );
    assert_eq!(
        graph.set_representation(id, Representation::Contour),
        Ok(())
    );
    assert_eq!(
        graph.tick(&context),
        FrameOutcome::Unavailable { viewport: id }
    );
    assert!(
        graph
            .show_contour(&context, id, &line(), &frame(), &colours, contour_view())
            .is_ok()
    );
    assert_eq!(
        graph.tick(&context),
        FrameOutcome::Submitted {
            viewports: 1,
            passes: 1
        }
    );
    assert_eq!(readback(&context, &image, 4, 2), presented_contour());
}

#[test]
#[ignore = "needs a real adapter, run by the gpu gate"]
fn show_contour_respects_the_default() {
    let Some(mut context) = context() else {
        return;
    };
    let Some(mut graph) = graph(&mut context) else {
        return;
    };
    let Some(id) = overlay(&context, &mut graph) else {
        return;
    };
    assert_eq!(graph.representation(id), Ok(Representation::Labelmap));
    let before = graph.submissions();
    assert_eq!(
        graph.show_contour(&context, id, &line(), &frame(), &lut(), contour_view()),
        Err(ContourDrawError::RepresentationMismatch)
    );
    assert_eq!(graph.submissions(), before);
    assert_eq!(
        graph.tick(&context),
        FrameOutcome::Unavailable { viewport: id }
    );
    assert_eq!(graph.submissions(), before);
}

#[test]
#[ignore = "needs a real adapter, run by the gpu gate"]
fn a_surface_show_respects_the_registry() {
    let Some(mut context) = context() else {
        return;
    };
    let Some(mut graph) = graph(&mut context) else {
        return;
    };
    let Some(id) = overlay(&context, &mut graph) else {
        return;
    };
    assert_eq!(
        graph.set_representation(id, Representation::Contour),
        Ok(())
    );
    let before = graph.submissions();
    assert_eq!(
        graph.show_surface(&context, id, &mesh(), &frame(), &lut(), surface_view()),
        Err(SurfaceDrawError::RepresentationMismatch)
    );
    assert_eq!(
        graph.tick(&context),
        FrameOutcome::Unavailable { viewport: id }
    );
    assert_eq!(graph.submissions(), before);
}

#[test]
#[ignore = "needs a real adapter, run by the gpu gate"]
fn the_same_choice_keeps_the_binding() {
    let Some(mut context) = context() else {
        return;
    };
    let Some(mut graph) = graph(&mut context) else {
        return;
    };
    let Some(id) = overlay(&context, &mut graph) else {
        return;
    };
    assert!(
        graph
            .show_labelmap(&context, id, &grid(), &lut(), style())
            .is_ok()
    );
    assert_eq!(
        graph.tick(&context),
        FrameOutcome::Submitted {
            viewports: 1,
            passes: 1
        }
    );
    let before = graph.submissions();
    assert_eq!(
        graph.set_representation(id, Representation::Labelmap),
        Ok(())
    );
    assert_eq!(graph.dirty(id), Ok(Dirty::default()));
    assert_eq!(graph.tick(&context), FrameOutcome::Noop);
    assert_eq!(graph.submissions(), before);
}

#[test]
#[ignore = "needs a real adapter, run by the gpu gate"]
fn recovery_keeps_the_choice_and_drops_the_texture() {
    let Some(mut context) = context() else {
        return;
    };
    let Some(mut graph) = graph(&mut context) else {
        return;
    };
    let (source, target_view, _image) = target(&context, 4, 2);
    let Ok(id) = graph.register(
        &context,
        target_view,
        source,
        Pass::SegOverlay(SegPass {
            blend: BlendMode::Composite,
        }),
        StoredRoute::PreApplied,
    ) else {
        return;
    };
    assert_eq!(
        graph.set_representation(id, Representation::Contour),
        Ok(())
    );
    let colours = lut();
    assert!(
        graph
            .show_contour(&context, id, &line(), &frame(), &colours, contour_view())
            .is_ok()
    );
    assert_eq!(
        graph.tick(&context),
        FrameOutcome::Submitted {
            viewports: 1,
            passes: 1
        }
    );
    let recovered = Recovered {
        caps: *context.caps(),
        graph: Some(GraphResources {
            pipelines_restored: 1,
            cache_generation: 2,
        }),
    };
    graph.install_recovered(&context, &recovered);
    assert_eq!(graph.representation(id), Ok(Representation::Contour));
    assert_eq!(
        graph.show_contour(&context, id, &line(), &frame(), &colours, contour_view()),
        Err(ContourDrawError::NotRebound)
    );
    assert_eq!(
        graph.tick(&context),
        FrameOutcome::Unavailable { viewport: id }
    );
    let (source, target_view, image) = target(&context, 4, 2);
    assert!(graph.rebind(&context, id, target_view, source).is_ok());
    assert_eq!(
        graph.tick(&context),
        FrameOutcome::Unavailable { viewport: id }
    );
    assert!(
        graph
            .show_contour(&context, id, &line(), &frame(), &colours, contour_view())
            .is_ok()
    );
    assert_eq!(
        graph.tick(&context),
        FrameOutcome::Submitted {
            viewports: 1,
            passes: 1
        }
    );
    assert_eq!(readback(&context, &image, 4, 2), presented_contour());
}
