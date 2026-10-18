//! F-039, backing-store resize and one device for every viewport.
//!
//! Native tests have no canvas. The colour target is an offscreen texture the
//! graph already accepts. The clear is the graph's existing black clear, so a
//! frame that still drew the previous extent leaves the new texture's far
//! pixel untouched and the previous texture overwritten.
//!
//! The GPU row is `#[ignore]`d. `bin/ocelli.sh test ocelli-render --test surface -- --ignored --test-threads=1`
//! runs it. Do not run a second GPU suite beside it.

use ocelli_render::{
    BlendMode, Dirty, FrameOutcome, GraphError, Pass, RenderGraph, StackPass, ViewportId, VoiParams,
};

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
