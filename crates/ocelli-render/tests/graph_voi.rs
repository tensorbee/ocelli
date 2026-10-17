//! F-038, the VOI functions through the production graph at half-float
//! precision.
//!
//! An 8-bit target cannot separate LINEAR from LINEAR_EXACT: both centre
//! values, 127.819 and 127.500 from DICOM PS3.3 C.11.2.1.2 and C.11.2.1.3.2,
//! quantise to 128. These rows render into `Rgba16Float` and read the red
//! channel back as a float, so the 0.32-of-255 divergence the whole oracle
//! exists to catch is visible through the graph composition, its uniform
//! plumbing and its function selection. All rows are `#[ignore]`d for the
//! gpu gate.

use ocelli_pixel::{
    LutChain, ModalityTransform, PhotometricInterpretation, PresentationLutEvidence, VoiFunction,
    VoiTransform,
};
use ocelli_render::{BlendMode, FrameOutcome, Pass, RenderGraph, StackPass, VoiParams};

fn chain(function: VoiFunction, photometric: PhotometricInterpretation) -> LutChain {
    let modality = ModalityTransform::new(None, Some(1.0), Some(0.0));
    let voi = VoiTransform::new(None, &[40.0], &[400.0], 0, function, 0.0, 255.0);
    assert!(modality.is_ok() && voi.is_ok());
    let (Ok(modality), Ok(voi)) = (modality, voi) else {
        unreachable!("asserted immediately above")
    };
    let built = LutChain::new(modality, voi, photometric, PresentationLutEvidence::Absent);
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

/// Decode one IEEE 754 half-float. Exact for the normal values used here.
fn half_to_f32(bits: u16) -> f32 {
    let sign = f32::from(bits >> 15) * -2.0 + 1.0;
    let exponent = i32::from((bits >> 10) & 0x1f);
    let fraction = f32::from(bits & 0x3ff) / 1024.0;
    if exponent == 0 {
        return sign * fraction * f32::powf(2.0, -14.0);
    }
    sign * (1.0 + fraction) * 2.0_f32.powi(exponent - 15)
}

fn render_one(
    context: &ocelli_render::GpuContext,
    graph: &mut RenderGraph,
    input: f32,
    function: VoiFunction,
) -> f32 {
    let device = context.device();
    let source = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("graph voi source"),
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
    context.queue().write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &source,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        bytemuck::cast_slice(&[input]),
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
    let target = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("graph voi target"),
        size: wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba16Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let voi = match VoiParams::from_chain(&chain(function, PhotometricInterpretation::Monochrome2))
    {
        Ok(voi) => voi,
        Err(_) => unreachable!("chain must express"),
    };
    let registered = graph.register(
        context,
        target.create_view(&wgpu::TextureViewDescriptor::default()),
        source.create_view(&wgpu::TextureViewDescriptor::default()),
        Pass::Stack(StackPass {
            voi,
            blend: BlendMode::Composite,
        }),
    );
    let viewport = match registered {
        Ok(viewport) => viewport,
        Err(_) => unreachable!("registration must fit"),
    };
    assert_eq!(
        graph.tick(context),
        FrameOutcome::Submitted {
            viewports: 1,
            passes: 1
        }
    );
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("graph voi readback"),
        size: 256,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("graph voi readback"),
    });
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: &target,
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
        return f32::NAN;
    };
    let bytes: &[u8] = &data;
    let bits = u16::from_le_bytes([
        bytes.first().copied().unwrap_or(0),
        bytes.get(1).copied().unwrap_or(0),
    ]);
    let value = half_to_f32(bits);
    assert!(graph.remove(viewport).is_ok());
    value
}

#[test]
#[ignore = "needs a real adapter, run by the gpu gate"]
fn linear_and_linear_exact_separate_at_the_window_centre() {
    let Some(mut context) = context() else {
        return;
    };
    let mut graph = match RenderGraph::new(&mut context, 1, wgpu::TextureFormat::Rgba16Float) {
        Ok(graph) => graph,
        Err(_) => return,
    };
    // PS3.3 C.11.2.1.2 gives 127.819 and C.11.2.1.3.2 gives 127.500 at the
    // centre, over a 0 to 255 range. Normalised, 0.50125 against 0.5.
    let linear = render_one(&context, &mut graph, 40.0, VoiFunction::Linear);
    let exact = render_one(&context, &mut graph, 40.0, VoiFunction::LinearExact);
    assert!((exact - 0.5).abs() < 0.001, "LINEAR_EXACT centre {exact}");
    assert!(
        linear - exact > 0.000_8,
        "LINEAR {linear} must sit above LINEAR_EXACT {exact} at the centre"
    );
}

#[test]
#[ignore = "needs a real adapter, run by the gpu gate"]
fn sigmoid_through_the_graph() {
    let Some(mut context) = context() else {
        return;
    };
    let mut graph = match RenderGraph::new(&mut context, 1, wgpu::TextureFormat::Rgba16Float) {
        Ok(graph) => graph,
        Err(_) => return,
    };
    // PS3.3 C.11.2.1.3.1 at input -60, centre 40, width 400: the exponent is
    // exactly plus one, so the normalised value is 1 / (1 + e).
    let sigmoid = render_one(&context, &mut graph, -60.0, VoiFunction::Sigmoid);
    assert!(
        (sigmoid - 0.268_941_42).abs() < 0.002,
        "SIGMOID at -60 {sigmoid}"
    );
}
