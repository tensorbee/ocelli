//! F-064 contour upload.
//!
//! The device row is ignored and runs under the gpu gate. Expected bytes are
//! the open line from the seg fixture: spacing 1, origin 0, LUT alpha 128.

use ocelli_core::Pt;
use ocelli_render::{
    BlendMode,
    ContourDrawError,
    ContourView,
    Dirty,
    FrameOutcome,
    GraphResources,
    Pass,
    Recovered,
    RenderGraph,
    SegPass,
    StoredRoute,
    ViewportId,
};
use ocelli_seg::{
    Contour, ContourGeometricType, LabelColourLut, PlaneFrame, Representation, Rgba8,
    present_contour,
};

fn colour() -> Rgba8 {
    Rgba8 {
        r: 10,
        g: 20,
        b: 30,
        a: 128,
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

fn view() -> ContourView {
    ContourView {
        width: 4,
        height: 2,
    }
}

fn presented() -> Vec<u8> {
    let mut out = vec![0_u8; 32];
    let painted = present_contour(
        &line(),
        &frame(),
        &LabelColourLut::from_segment_one(vec![colour()]),
        4,
        2,
        &mut out,
    );
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

fn target(
    context: &ocelli_render::GpuContext,
    width: u32,
    height: u32,
) -> (wgpu::TextureView, wgpu::TextureView, wgpu::Texture) {
    let device = context.device();
    let source = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("contour source"),
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
        label: Some("contour target"),
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
        label: Some("contour readback"),
        size: bytes,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("contour readback"),
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

#[test]
#[ignore = "needs a real adapter, run by the gpu gate"]
fn shown_contour_matches_the_presenter() {
    let Some(mut context) = context() else {
        return;
    };
    let mut graph = match RenderGraph::new(&mut context, 2, wgpu::TextureFormat::Rgba8Unorm) {
        Ok(graph) => graph,
        Err(_) => return,
    };
    let (source, target_view, _image) = target(&context, 4, 2);
    let edge = match graph.register(
        &context,
        target_view,
        source,
        Pass::SegOverlay(SegPass {
            blend: BlendMode::LabelmapEdgeProjection,
        }),
    StoredRoute::PreApplied,
) {
        Ok(id) => id,
        Err(_) => return,
    };
    let lut = LabelColourLut::from_segment_one(vec![colour()]);
    assert_eq!(
        graph.show_contour(&context, edge, &line(), &frame(), &lut, view()),
        Err(ContourDrawError::BlendUnavailable)
    );
    assert!(graph.remove(edge).is_ok());
    let (source, target_view, image) = target(&context, 4, 2);
    let id = match graph.register(
        &context,
        target_view,
        source,
        Pass::SegOverlay(SegPass {
            blend: BlendMode::Composite,
        }),
    StoredRoute::PreApplied,
) {
        Ok(id) => id,
        Err(_) => return,
    };
    assert_eq!(
        graph.set_representation(id, Representation::Contour),
        Ok(())
    );
    assert!(
        graph
            .show_contour(&context, id, &line(), &frame(), &lut, view())
            .is_ok()
    );
    assert_eq!(
        graph.tick(&context),
        FrameOutcome::Submitted {
            viewports: 1,
            passes: 1
        }
    );
    let expected = presented();
    assert_eq!(expected.get(0..4), Some(&[10_u8, 20, 30, 128][..]));
    assert_eq!(expected.get(16..20), Some(&[0_u8, 0, 0, 0][..]));
    assert_eq!(readback(&context, &image, 4, 2), expected);
    let before = graph.submissions();
    assert!(
        graph
            .show_contour(&context, id, &line(), &frame(), &lut, view())
            .is_ok()
    );
    assert_eq!(graph.dirty(id), Ok(Dirty::default()));
    assert_eq!(graph.tick(&context), FrameOutcome::Noop);
    assert_eq!(graph.submissions(), before);
    let recovered = Recovered {
        caps: *context.caps(),
        graph: Some(GraphResources {
            pipelines_restored: 1,
            cache_generation: 2,
        }),
    };
    graph.install_recovered(&context, &recovered);
    assert_eq!(
        graph.show_contour(&context, id, &line(), &frame(), &lut, view()),
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
            .show_contour(&context, id, &line(), &frame(), &lut, view())
            .is_ok()
    );
    assert_eq!(
        graph.tick(&context),
        FrameOutcome::Submitted {
            viewports: 1,
            passes: 1
        }
    );
    assert_eq!(readback(&context, &image, 4, 2), expected);
    assert_eq!(
        graph.show_contour(&context, ViewportId(9), &line(), &frame(), &lut, view()),
        Err(ContourDrawError::UnknownViewport)
    );
}
