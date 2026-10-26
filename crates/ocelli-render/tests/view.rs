//! F-045 camera uniform.
//!
//! `set_view` writes the CSS matrix and the device pixel ratio, 80 bytes, and
//! sets `Dirty.camera`. `camera()` stays the CSS matrix. It does not write the
//! VOI uniform and it does not increment an upload counter. Outside the image
//! the fragment is black, not a windowed zero. The section 18.3 row stays in
//! `graph.rs` and is not edited here.

use ocelli_pixel::{
    LutChain, ModalityTransform, PhotometricInterpretation, PresentationLutEvidence, VoiFunction,
    VoiTransform,
};
use ocelli_render::{
    BlendMode, Dirty, FrameOutcome, GraphError, Pass, RenderGraph, StackPass, ViewportId,
    VoiParams, VolumePass,
};

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

fn chain() -> LutChain {
    let modality = ModalityTransform::new(None, Some(1.0), Some(0.0));
    let voi = VoiTransform::new(None, &[40.0], &[400.0], 0, VoiFunction::Linear, 0.0, 255.0);
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

fn voi() -> VoiParams {
    match VoiParams::from_chain(&chain()) {
        Ok(voi) => voi,
        Err(_) => unreachable!("chain must express"),
    }
}

fn sized_views(
    context: &ocelli_render::GpuContext,
    inputs: &[f32],
    target_width: u32,
) -> (wgpu::TextureView, wgpu::TextureView, wgpu::Texture) {
    let device = context.device();
    let source_width = u32::try_from(inputs.len()).unwrap_or(1);
    let source = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("view source"),
        size: wgpu::Extent3d {
            width: source_width,
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
            bytes_per_row: Some(4 * source_width),
            rows_per_image: Some(1),
        },
        wgpu::Extent3d {
            width: source_width,
            height: 1,
            depth_or_array_layers: 1,
        },
    );
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("view target"),
        size: wgpu::Extent3d {
            width: target_width,
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
        label: Some("view readback"),
        size: u64::from(padded),
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("view readback"),
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
    let row = 4 * width;
    data.iter()
        .take(usize::try_from(row).unwrap_or(0))
        .copied()
        .collect()
}

fn stack() -> Pass {
    Pass::Stack(StackPass {
        voi: voi(),
        blend: BlendMode::Composite,
    })
}

/// Image = canvas + (10, 0). A fragment at 0.5 lands outside a width-1 image.
fn outside_camera() -> [f32; 16] {
    [
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 10.0, 0.0, 0.0, 1.0,
    ]
}

fn assert_matrix(got: [f32; 16], expect: [f32; 16]) {
    for (index, (left, right)) in got.iter().zip(expect.iter()).enumerate() {
        assert_eq!(left.to_bits(), right.to_bits(), "camera component {index}");
    }
}

fn graph() -> Option<(ocelli_render::GpuContext, RenderGraph)> {
    let mut context = context()?;
    println!("auto opened tier {:?}", context.caps().tier);
    match RenderGraph::new(&mut context, 4, wgpu::TextureFormat::Rgba8Unorm) {
        Ok(graph) => Some((context, graph)),
        Err(_) => None,
    }
}

#[test]
#[ignore = "needs a real adapter, run by the gpu gate"]
fn set_view_dirties_only_the_camera_and_keeps_the_css_matrix() {
    let Some((context, mut graph)) = graph() else {
        return;
    };
    let camera = outside_camera();
    assert!(matches!(
        graph.set_view(&context, ViewportId(99), camera, 1.0),
        Err(GraphError::UnknownViewport)
    ));
    // The documented order is the id first, then the pass, then the rebound
    // state, then the ratio. An unknown id with a bad ratio must still name
    // the id.
    assert!(matches!(
        graph.set_view(&context, ViewportId(99), camera, 0.0),
        Err(GraphError::UnknownViewport)
    ));
    let (source, target_view, target) = sized_views(&context, &[40.0], 1);
    let viewport = match graph.register(&context, target_view, source, stack()) {
        Ok(id) => id,
        Err(_) => unreachable!("registration must fit"),
    };
    // A valid stack viewport with a ratio that narrows to zero must refuse the
    // ratio rather than upload a zero scale the shader would skip.
    assert!(matches!(
        graph.set_view(&context, viewport, camera, f64::MIN_POSITIVE / 2.0),
        Err(GraphError::DevicePixelRatio)
    ));
    let (source, target_view, _target) = sized_views(&context, &[40.0], 1);
    let volume = match graph.register(
        &context,
        target_view,
        source,
        Pass::VolumeRaycast(VolumePass {
            blend: BlendMode::Composite,
        }),
    ) {
        Ok(id) => id,
        Err(_) => unreachable!("registration must fit"),
    };
    assert!(matches!(
        graph.set_view(&context, volume, camera, 1.0),
        Err(GraphError::PassHasNoUniform)
    ));
    assert_eq!(
        graph.tick(&context),
        FrameOutcome::Submitted {
            viewports: 1,
            passes: 1
        }
    );
    let red_before = readback(&context, &target, 1);
    assert_eq!(red_before.first().copied().unwrap_or(0), 128);
    let uploads = graph.uploads();
    let lut_uploads = graph.lut_uploads();
    let brick_uploads = graph.brick_uploads();
    let voi_before = match graph.voi_params(viewport) {
        Ok(voi) => voi,
        Err(_) => unreachable!("stack viewport has a uniform"),
    };
    assert!(matches!(
        graph.dirty(viewport),
        Ok(dirty) if !dirty.any()
    ));
    assert!(graph.set_view(&context, viewport, camera, 1.0).is_ok());
    assert!(matches!(
        graph.dirty(viewport),
        Ok(Dirty {
            camera: true,
            voi: false,
            slice: false,
            segmentation: false,
            data: false,
        })
    ));
    let stored = match graph.camera(viewport) {
        Ok(stored) => stored,
        Err(_) => unreachable!("viewport is registered"),
    };
    assert_matrix(stored, camera);
    let voi_after = match graph.voi_params(viewport) {
        Ok(voi) => voi,
        Err(_) => unreachable!("stack viewport has a uniform"),
    };
    assert_eq!(
        bytemuck::bytes_of(&voi_before),
        bytemuck::bytes_of(&voi_after)
    );
    assert_eq!(graph.uploads(), uploads);
    assert_eq!(graph.lut_uploads(), lut_uploads);
    assert_eq!(graph.brick_uploads(), brick_uploads);
    assert_eq!(
        graph.tick(&context),
        FrameOutcome::Submitted {
            viewports: 1,
            passes: 1
        }
    );
    // Stored 0 windowed through centre 40 width 400 is not black. Outside
    // the image must be (0, 0, 0, 255), which is this readback.
    let outside = readback(&context, &target, 1);
    assert_eq!(outside.as_slice(), [0, 0, 0, 255]);
    let recovered = ocelli_render::Recovered {
        caps: *context.caps(),
        graph: Some(ocelli_render::GraphResources {
            pipelines_restored: 1,
            cache_generation: 2,
        }),
    };
    graph.install_recovered(&context, &recovered);
    assert!(matches!(
        graph.set_view(&context, viewport, camera, 1.0),
        Err(GraphError::ViewportNotRebound)
    ));
    let (source, target_view, target) = sized_views(&context, &[40.0], 1);
    assert!(
        graph
            .rebind(&context, viewport, target_view, source)
            .is_ok()
    );
    let restored = match graph.camera(viewport) {
        Ok(stored) => stored,
        Err(_) => unreachable!("viewport is registered"),
    };
    assert_matrix(restored, camera);
    assert_eq!(
        graph.tick(&context),
        FrameOutcome::Submitted {
            viewports: 1,
            passes: 1
        }
    );
    // Rebind restored the camera bytes into the new uniform. Identity would
    // have sampled texel 0 and read back red 128.
    assert_eq!(readback(&context, &target, 1).as_slice(), [0, 0, 0, 255]);
}

#[test]
#[ignore = "needs a real adapter, run by the gpu gate"]
fn a_fragment_past_the_image_is_black_under_the_identity() {
    let Some((context, mut graph)) = graph() else {
        return;
    };
    let (source, target_view, target) = sized_views(&context, &[40.0], 2);
    let _viewport = match graph.register(&context, target_view, source, stack()) {
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
    let pixels = readback(&context, &target, 2);
    assert_eq!(pixels.as_slice(), [128, 128, 128, 255, 0, 0, 0, 255]);
}

#[test]
#[ignore = "native GL is not this host's tier B. A miss is recorded and is not the run."]
fn tier_b_margin_or_a_recorded_miss() {
    let mut clock = native_clock();
    let Some(adapter) = pollster::block_on(ocelli_render::resolve_native_gl(&mut clock)) else {
        println!("native Backends::GL found no adapter. This is not a tier B run.");
        return;
    };
    let mut context = match pollster::block_on(adapter.open()) {
        Ok(context) => context,
        Err(error) => {
            println!("native GL adapter did not open a device: {error}. This is not a tier B run.");
            return;
        }
    };
    if context.caps().tier != ocelli_render::Tier::B {
        println!(
            "native GL opened tier {:?}. This is not a tier B run.",
            context.caps().tier
        );
        return;
    }
    let graph = RenderGraph::new(&mut context, 4, wgpu::TextureFormat::Rgba8Unorm);
    assert!(graph.is_ok(), "tier B graph did not compile");
    let Ok(mut graph) = graph else {
        unreachable!("asserted immediately above")
    };
    let (source, target_view, target) = sized_views(&context, &[40.0], 2);
    let viewport = match graph.register(&context, target_view, source, stack()) {
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
    assert_eq!(
        readback(&context, &target, 2).as_slice(),
        [128, 128, 128, 255, 0, 0, 0, 255]
    );
    assert!(
        graph
            .set_view(&context, viewport, outside_camera(), 1.0)
            .is_ok()
    );
    assert_eq!(
        graph.tick(&context),
        FrameOutcome::Submitted {
            viewports: 1,
            passes: 1
        }
    );
    // Pixel 0 was red 128 under the identity. A translate of 10 puts it
    // outside the width-1 image, so both fragments are black.
    assert_eq!(
        readback(&context, &target, 2).as_slice(),
        [0, 0, 0, 255, 0, 0, 0, 255]
    );
}

/// The uploaded uniform carries the CSS matrix and the device pixel ratio.
/// The fragment shader divides by that ratio before the multiply.
#[test]
fn the_camera_uniform_carries_dpr_and_the_shader_divides_first() {
    let identity = [
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0_f32,
    ];
    let one = ocelli_render::device_camera_words(identity, 1.0);
    let exact = ocelli_render::device_camera_words(identity, 1.25);
    let two = ocelli_render::device_camera_words(identity, 2.0);
    assert_eq!(one[16].to_bits(), 1.0_f32.to_bits());
    assert_eq!(
        exact[16].to_bits(),
        ocelli_viewport::view::f32_from_f64(1.25).to_bits()
    );
    assert_eq!(two[16].to_bits(), 2.0_f32.to_bits());
    assert_eq!(two[0].to_bits(), identity[0].to_bits());
    let shader = ocelli_render::tier_b_fragment_module();
    // The exact lines reject a missing division, a second division, and a
    // division on one axis only. The substring-order check alone passed all
    // three, because `at.x / scale / scale` still contains `at.x / scale`
    // before the multiply.
    assert!(shader.contains("let css_x = select(at.x, at.x / scale, scale > 0.0);"));
    assert!(shader.contains("let css_y = select(at.y, at.y / scale, scale > 0.0);"));
    assert_eq!(shader.matches("/ scale").count(), 2);
    assert!(shader.contains("camera.css * vec4<f32>(css_x, css_y, 0.0, 1.0)"));
    let divide = shader.find("at.x / scale");
    let multiply = shader.find("camera.css *");
    assert!(divide.is_some() && multiply.is_some());
    let Some(divide) = divide else { return };
    let Some(multiply) = multiply else { return };
    assert!(divide < multiply);
}

#[test]
#[ignore = "needs a real adapter, run by the gpu gate"]
fn device_fragments_are_divided_by_dpr_before_the_css_camera() {
    let Some((context, mut graph)) = graph() else {
        println!("no adapter opened. This is not a device-to-CSS pixel observation.");
        return;
    };
    let css = [
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0_f32,
    ];
    let (source, target_view, target) = sized_views(&context, &[40.0, 4_000.0], 2);
    let viewport = match graph.register(&context, target_view, source, stack()) {
        Ok(id) => id,
        Err(_) => unreachable!("registration must fit"),
    };
    assert!(graph.set_view(&context, viewport, css, 2.0).is_ok());
    assert_eq!(
        graph.tick(&context),
        FrameOutcome::Submitted {
            viewports: 1,
            passes: 1
        }
    );
    // Centres 0.5 and 1.5 become CSS 0.25 and 0.75. Both are texel 0.
    assert_eq!(
        readback(&context, &target, 2).as_slice(),
        [128, 128, 128, 255, 128, 128, 128, 255]
    );

    let (source, target_view, target) = sized_views(&context, &[40.0, 4_000.0], 2);
    let viewport = match graph.register(&context, target_view, source, stack()) {
        Ok(id) => id,
        Err(_) => unreachable!("registration must fit"),
    };
    assert!(graph.set_view(&context, viewport, css, 1.0).is_ok());
    let outcome = graph.tick(&context);
    assert!(
        outcome
            == FrameOutcome::Submitted {
                viewports: 1,
                passes: 1
            }
            || outcome
                == FrameOutcome::Submitted {
                    viewports: 2,
                    passes: 2
                }
    );
    let pixels = readback(&context, &target, 2);
    assert_eq!(pixels.as_slice(), [128, 128, 128, 255, 255, 255, 255, 255]);

    let mut translated = css;
    translated[12] = -0.4;
    let (source, target_view, target) = sized_views(&context, &[40.0], 1);
    let viewport = match graph.register(&context, target_view, source, stack()) {
        Ok(id) => id,
        Err(_) => unreachable!("registration must fit"),
    };
    assert!(graph.set_view(&context, viewport, translated, 2.0).is_ok());
    let _ = graph.tick(&context);
    // CSS 0.25 plus a translate of -0.4 is outside the image.
    assert_eq!(readback(&context, &target, 1).as_slice(), [0, 0, 0, 255]);
}

/// Distinct texels make missing and double DPR division visibly disagree.
#[test]
#[ignore = "needs a real adapter, run by the gpu gate"]
fn fractional_dpr_selects_the_derived_texels() -> Result<(), &'static str> {
    let (context, mut graph) = graph().ok_or("this proof requires an actual adapter")?;
    let css = [
        1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
    ];
    let (source, target_view, target) = sized_views(&context, &[-160.0, 40.0, 4_000.0, -160.0], 5);
    let viewport = graph
        .register(&context, target_view, source, stack())
        .map_err(|_| "one slot refused")?;
    assert!(graph.set_view(&context, viewport, css, 1.25).is_ok());
    assert!(matches!(
        graph.tick(&context),
        FrameOutcome::Submitted { .. }
    ));
    // Device centres / 1.25 = .4, 1.2, 2.0, 2.8, 3.6.
    // Source texels = 0, 1, 2, 2, 3. Missing division selects 0,1,2,3,out.
    // A second division selects 0,0,1,2,2. PS3.3 C.11.2.1.2.1 maps
    // -160 to 0 and 40 to 127.81955, quantized by this plain pass to 128.
    assert_eq!(
        readback(&context, &target, 5).as_slice(),
        [
            0, 0, 0, 255, 128, 128, 128, 255, 255, 255, 255, 255, 255, 255, 255, 255, 0, 0, 0, 255,
        ]
    );
    Ok(())
}
