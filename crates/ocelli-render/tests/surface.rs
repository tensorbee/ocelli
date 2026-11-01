//! F-039, backing-store resize and one device for every viewport.
//!
//! Native tests have no canvas. The colour target is an offscreen texture the
//! graph already accepts. The clear is the graph's existing black clear, so a
//! frame that still drew the previous extent leaves the new texture's far
//! pixel untouched and the previous texture overwritten.
//!
//! The GPU rows are `#[ignore]`d. `bin/ocelli.sh test ocelli-render --test surface -- --ignored --test-threads=1`
//! runs them. Do not run a second GPU suite beside it. The F-065 upload row is
//! in this file so the F-039 resize row stays on that command.

use ocelli_core::Pt;
use ocelli_render::{
    BlendMode, Dirty, FrameOutcome, GraphError, GraphResources, Pass, Recovered, RenderGraph,
    SegPass, StackPass, SurfaceDrawError, SurfaceView, ViewportId, VoiParams,
};
use ocelli_seg::{LabelColourLut, PlaneFrame, Rgba8, SurfaceMesh, present_surface};

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

fn stack_pass() -> Pass {
    Pass::Stack(StackPass {
        voi: VoiParams {
            center: 0.0,
            width: 1.0,
            slope: 1.0,
            intercept: 0.0,
            ymin: 0.0,
            ymax: 255.0,
            fn_kind: 0,
            invert: 0,
        },
        blend: BlendMode::Composite,
    })
}

fn color_target(device: &wgpu::Device, width: u32) -> (wgpu::Texture, wgpu::TextureView) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("surface stand-in"),
        size: wgpu::Extent3d {
            width,
            height: 1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT
            | wgpu::TextureUsages::COPY_SRC
            | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    (texture, view)
}

fn source_view(device: &wgpu::Device) -> (wgpu::Texture, wgpu::TextureView) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("surface stand-in source"),
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
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    (texture, view)
}

fn paint(context: &ocelli_render::GpuContext, texture: &wgpu::Texture, width: u32, byte: u8) {
    let count = usize::try_from(width).unwrap_or(0).saturating_mul(4);
    let bytes = vec![byte; count];
    context.queue().write_texture(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &bytes,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(width.saturating_mul(4)),
            rows_per_image: Some(1),
        },
        wgpu::Extent3d {
            width,
            height: 1,
            depth_or_array_layers: 1,
        },
    );
}

fn readback(context: &ocelli_render::GpuContext, target: &wgpu::Texture, width: u32) -> Vec<u8> {
    let device = context.device();
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("surface readback"),
        size: 256,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("surface readback"),
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
    let row = 4 * usize::try_from(width).unwrap_or(0);
    data.iter().take(row).copied().collect()
}

#[test]
#[ignore = "needs a real GPU adapter, which the CI floor does not have (D-04)"]
fn resize_replaces_the_target_and_two_viewports_submit_once() {
    let Some((adapter, mut context)) = session() else {
        println!("this machine resolved tier C, so there is no device");
        return;
    };
    // One context. Both targets are created from its device, and the handle
    // compares equal to itself, which is the single-device assertion.
    let device = context.device().clone();
    assert_eq!(context.device(), &device);
    assert!(adapter.opened_from_this_instance(&context));

    let built = RenderGraph::new(&mut context, 4, wgpu::TextureFormat::Rgba8Unorm);
    assert!(built.is_ok());
    let Ok(mut graph) = built else { return };

    let (source_a, source_view_a) = source_view(context.device());
    let (old_target, old_view) = color_target(context.device(), 4);
    let registered = graph.register(&context, old_view, source_view_a, stack_pass());
    assert!(registered.is_ok());
    let Ok(first) = registered else { return };
    assert_eq!(first, ViewportId(0));

    let (_source_b, source_view_b) = source_view(context.device());
    let (_other_target, other_view) = color_target(context.device(), 4);
    let registered = graph.register(&context, other_view, source_view_b, stack_pass());
    assert!(registered.is_ok());
    let Ok(second) = registered else { return };
    assert_eq!(second, ViewportId(1));
    assert_eq!(context.device(), &device);

    assert_eq!(
        graph.tick(&context),
        FrameOutcome::Submitted {
            viewports: 2,
            passes: 2
        }
    );
    assert_eq!(graph.submissions(), 1);
    assert_eq!(graph.uploads(), 0);

    // The first frame drew into the 4-wide target. Overwrite it so a second
    // frame that still targeted it would destroy the sentinel.
    paint(&context, &old_target, 4, 0x11);
    let (new_target, new_view) = color_target(context.device(), 8);
    paint(&context, &new_target, 8, 0x22);
    let rebound_source = source_a.create_view(&wgpu::TextureViewDescriptor::default());
    assert!(
        graph
            .rebind(&context, first, new_view, rebound_source)
            .is_ok()
    );
    assert!(
        graph
            .mark_dirty(
                first,
                Dirty {
                    camera: true,
                    ..Dirty::default()
                }
            )
            .is_ok()
    );
    // The other viewport is clean, so this frame encodes the resized one only.
    // Still one submit, and the resize did not upload stored samples.
    assert_eq!(
        graph.tick(&context),
        FrameOutcome::Submitted {
            viewports: 1,
            passes: 1
        }
    );
    assert_eq!(graph.submissions(), 2);
    assert_eq!(graph.uploads(), 0);
    assert_eq!(context.device(), &device);

    let fresh = readback(&context, &new_target, 8);
    // 8 texels, not the previous 4. Pixel 7 is past a stale 4-wide target, so
    // a frame that drew the old extent leaves that byte at the sentinel.
    assert_eq!(fresh.len(), 32);
    assert_eq!(fresh.get(3).copied(), Some(255));
    assert_eq!(fresh.get(31).copied(), Some(255));
    let stale = readback(&context, &old_target, 4);
    assert_eq!(stale, vec![0x11; 16]);

    let (stored_target, stored_view) = color_target(context.device(), 1);
    let registered = graph.register_stored(&context, &adapter, stored_view, 1, 1, stack_pass());
    assert!(registered.is_ok());
    let Ok(stored) = registered else { return };
    assert_eq!(graph.uploads(), 0);
    assert!(
        graph
            .upload_stored(&context, &adapter, stored, 1, 1, &[0.0])
            .is_ok()
    );
    assert_eq!(graph.uploads(), 1);
    let refused_target = stored_target.create_view(&wgpu::TextureViewDescriptor::default());
    let refused_source = stored_target.create_view(&wgpu::TextureViewDescriptor::default());
    assert!(matches!(
        graph.rebind(&context, stored, refused_target, refused_source),
        Err(GraphError::StoredNeedsRebind)
    ));
    let rebound = stored_target.create_view(&wgpu::TextureViewDescriptor::default());
    assert!(
        graph
            .rebind_stored(&context, &adapter, stored, rebound)
            .is_ok()
    );
    assert_eq!(graph.uploads(), 1);
}

fn overlay_colour() -> Rgba8 {
    Rgba8 {
        r: 10,
        g: 20,
        b: 30,
        a: 128,
    }
}

fn overlay_frame() -> PlaneFrame {
    PlaneFrame {
        origin: Pt::new(0.0, 0.0, 0.0),
        spacing_x_mm: 1.0,
        spacing_y_mm: 1.0,
    }
}

fn overlay_mesh() -> SurfaceMesh {
    SurfaceMesh {
        segment: 1,
        vertices: vec![
            Pt::new(0.0, 0.0, 0.0),
            Pt::new(4.0, 0.0, 0.0),
            Pt::new(0.0, 4.0, 0.0),
        ],
        triangles: vec![[0, 1, 2]],
    }
}

fn overlay_view() -> SurfaceView {
    SurfaceView {
        width: 5,
        height: 5,
    }
}

fn overlay_presented() -> Vec<u8> {
    let mut out = vec![0_u8; 100];
    let painted = present_surface(
        &overlay_mesh(),
        &overlay_frame(),
        &LabelColourLut::from_segment_one(vec![overlay_colour()]),
        5,
        5,
        &mut out,
    );
    assert!(painted.is_ok());
    out
}

fn overlay_context() -> Option<ocelli_render::GpuContext> {
    let mut clock = native_clock();
    let adapter = pollster::block_on(ocelli_render::resolve_adapter(
        ocelli_render::TierRequest::Auto,
        &mut clock,
    ))?;
    pollster::block_on(adapter.open()).ok()
}

fn overlay_target(
    context: &ocelli_render::GpuContext,
    width: u32,
    height: u32,
) -> (wgpu::TextureView, wgpu::TextureView, wgpu::Texture) {
    let device = context.device();
    let source = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("surface source"),
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
        label: Some("surface target"),
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

fn overlay_readback(
    context: &ocelli_render::GpuContext,
    image: &wgpu::Texture,
    width: u32,
    height: u32,
) -> Vec<u8> {
    let stride = 256_u32;
    let bytes = u64::from(stride).saturating_mul(u64::from(height));
    let device = context.device();
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("surface readback"),
        size: bytes,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("surface readback"),
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
fn shown_surface_matches_the_presenter() {
    let Some(mut context) = overlay_context() else {
        return;
    };
    let mut graph = match RenderGraph::new(&mut context, 2, wgpu::TextureFormat::Rgba8Unorm) {
        Ok(graph) => graph,
        Err(_) => return,
    };
    let (source, target_view, _image) = overlay_target(&context, 5, 5);
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
    let lut = LabelColourLut::from_segment_one(vec![overlay_colour()]);
    assert_eq!(
        graph.show_surface(
            &context,
            edge,
            &overlay_mesh(),
            &overlay_frame(),
            &lut,
            overlay_view()
        ),
        Err(SurfaceDrawError::BlendUnavailable)
    );
    assert!(graph.remove(edge).is_ok());
    let (source, target_view, image) = overlay_target(&context, 5, 5);
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
    assert!(
        graph
            .show_surface(
                &context,
                id,
                &overlay_mesh(),
                &overlay_frame(),
                &lut,
                overlay_view()
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
    let expected = overlay_presented();
    assert_eq!(expected.get(0..4), Some(&[10_u8, 20, 30, 128][..]));
    assert_eq!(expected.get(16..20), Some(&[0_u8, 0, 0, 0][..]));
    assert_eq!(overlay_readback(&context, &image, 5, 5), expected);
    let before = graph.submissions();
    assert!(
        graph
            .show_surface(
                &context,
                id,
                &overlay_mesh(),
                &overlay_frame(),
                &lut,
                overlay_view()
            )
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
        graph.show_surface(
            &context,
            id,
            &overlay_mesh(),
            &overlay_frame(),
            &lut,
            overlay_view()
        ),
        Err(SurfaceDrawError::NotRebound)
    );
    assert_eq!(
        graph.tick(&context),
        FrameOutcome::Unavailable { viewport: id }
    );
    let (source, target_view, image) = overlay_target(&context, 5, 5);
    assert!(graph.rebind(&context, id, target_view, source).is_ok());
    assert_eq!(
        graph.tick(&context),
        FrameOutcome::Unavailable { viewport: id }
    );
    assert!(
        graph
            .show_surface(
                &context,
                id,
                &overlay_mesh(),
                &overlay_frame(),
                &lut,
                overlay_view()
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
    assert_eq!(overlay_readback(&context, &image, 5, 5), expected);
    assert_eq!(
        graph.show_surface(
            &context,
            ViewportId(9),
            &overlay_mesh(),
            &overlay_frame(),
            &lut,
            overlay_view()
        ),
        Err(SurfaceDrawError::UnknownViewport)
    );
}
