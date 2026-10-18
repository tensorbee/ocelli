//! Section 18.3 drawn through the production graph.
//!
//! The four stored values are PS3.3 C.11.2.1.2 and C.11.2.1.3.2 at centre
//! 40, width 400, slope 1, intercept 0, output 0 to 255. The CPU row is
//! `LutChain::map_into`. This module does not re-type the window functions.
//!
//! The readback is the test. It is not a product path. Pixels stay in this
//! process and are not a session export. The colour target is `Rgba32Float`
//! so the 0 to 255 display scale is an `f32` multiply, not a normalised
//! 8-bit value. When that format cannot be a render attachment and a copy
//! source, the stack pass reports unavailable and draws nothing.

use ocelli_core::{Display, Stored};
use ocelli_pixel::{
    LutChain, ModalityTransform, PhotometricInterpretation, PresentationLutEvidence, VoiFunction,
    VoiTransform,
};

use crate::caps::Tier;
use crate::gpu::GpuContext;
use crate::graph::{
    BlendMode, FrameOutcome, GraphError, Pass, RenderGraph, StackPass, UploadError,
};
use crate::probe::ResolvedAdapter;
use crate::voi::VoiParams;

/// Stored inputs for the section 18.3 rows, in order.
const INPUT_LOW: f32 = -160.0;
const INPUT_CENTRE: f32 = 40.0;
const INPUT_HIGH: f32 = 240.0;
const INPUT_EDGE: f32 = -60.0;
const INPUTS: [f32; 4] = [INPUT_LOW, INPUT_CENTRE, INPUT_HIGH, INPUT_EDGE];

const COLOUR_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba32Float;

/// Why the section 18.3 draw did not return pixels.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Section183Error {
    /// No preserving stored format, or the colour target cannot be read back
    /// as `f32`. Nothing was drawn into another format.
    Unavailable,
    /// Tier C. The graph runs no shader.
    TierC,
    /// The tick did not submit the one viewport.
    NotSubmitted,
    /// The copy back from the colour target did not yield four values.
    ///
    /// The text is the step that stopped, so a WebGL map failure is not
    /// reported as a short buffer.
    Readback(String),
}

/// GPU displays beside the CPU chain, both on the 0 to 255 scale.
#[derive(Debug, Clone, Copy)]
pub struct Section183Pixels {
    /// The stored-sample format the graph selected.
    pub format: wgpu::TextureFormat,
    /// LINEAR, four texels.
    pub linear: [f32; 4],
    /// LINEAR_EXACT, the same four texels.
    pub linear_exact: [f32; 4],
    /// `LutChain::map_into` for LINEAR.
    pub cpu_linear: [f32; 4],
    /// `LutChain::map_into` for LINEAR_EXACT.
    pub cpu_linear_exact: [f32; 4],
}

/// A short name for the three preserving formats.
#[must_use]
pub fn stored_format_name(format: wgpu::TextureFormat) -> &'static str {
    match format {
        wgpu::TextureFormat::R32Float => "R32Float",
        wgpu::TextureFormat::Rg32Float => "Rg32Float",
        wgpu::TextureFormat::Rgba32Float => "Rgba32Float",
        _ => "other",
    }
}

/// Upload the four section 18.3 values, draw LINEAR and LINEAR_EXACT, and
/// read the colour target back.
///
/// # Errors
///
/// [`Section183Error::Unavailable`] when `Rgba32Float` cannot be rendered
/// and copied, or when no preserving stored format exists.
/// [`Section183Error::TierC`] on CPU. The other variants mean the draw or
/// the readback did not complete. A refusal draws no substitute image.
pub fn read_section_18_3(
    ctx: &mut GpuContext,
    adapter: &ResolvedAdapter,
) -> Result<Section183Pixels, Section183Error> {
    if ctx.caps().tier == Tier::Cpu {
        return Err(Section183Error::TierC);
    }
    let features = adapter
        .wgpu_adapter()
        .get_texture_format_features(COLOUR_FORMAT);
    let needed = wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC;
    if !features.allowed_usages.contains(needed) {
        return Err(Section183Error::Unavailable);
    }
    let linear = window_chain(VoiFunction::Linear)?;
    let exact = window_chain(VoiFunction::LinearExact)?;
    let cpu_linear = cpu_row(&linear)?;
    let cpu_linear_exact = cpu_row(&exact)?;
    let mut graph = match RenderGraph::new(ctx, 1, COLOUR_FORMAT) {
        Ok(graph) => graph,
        Err(GraphError::TierCUnavailable) => return Err(Section183Error::TierC),
        Err(error) => return Err(Section183Error::Readback(error.to_string())),
    };
    let (view, texture) = colour_target(ctx.device());
    let voi = VoiParams::from_chain(&linear)
        .map_err(|error| Section183Error::Readback(format!("{error:?}")))?;
    let viewport = match graph.register_stored(
        ctx,
        adapter,
        view,
        4,
        1,
        Pass::Stack(StackPass {
            voi,
            blend: BlendMode::Composite,
        }),
    ) {
        Ok(viewport) => viewport,
        Err(UploadError::FormatUnavailable) => return Err(Section183Error::Unavailable),
        Err(error) => return Err(Section183Error::Readback(error.to_string())),
    };
    let format = remembered_or_stored(adapter, ctx.caps().tier)?;
    match graph.upload_stored(ctx, adapter, viewport, 4, 1, &INPUTS) {
        Ok(()) => {}
        Err(UploadError::FormatUnavailable) => return Err(Section183Error::Unavailable),
        Err(error) => return Err(Section183Error::Readback(error.to_string())),
    }
    require_frame(graph.tick(ctx))?;
    let linear_pixels = read_displays(ctx, &texture)?;
    graph
        .set_voi(ctx, viewport, &exact)
        .map_err(|error| Section183Error::Readback(format!("{error:?}")))?;
    require_frame(graph.tick(ctx))?;
    let exact_pixels = read_displays(ctx, &texture)?;
    Ok(Section183Pixels {
        format,
        linear: linear_pixels,
        linear_exact: exact_pixels,
        cpu_linear,
        cpu_linear_exact,
    })
}

fn remembered_or_stored(
    adapter: &ResolvedAdapter,
    tier: Tier,
) -> Result<wgpu::TextureFormat, Section183Error> {
    let features = [
        adapter
            .wgpu_adapter()
            .get_texture_format_features(wgpu::TextureFormat::R32Float),
        adapter
            .wgpu_adapter()
            .get_texture_format_features(wgpu::TextureFormat::Rg32Float),
        adapter
            .wgpu_adapter()
            .get_texture_format_features(wgpu::TextureFormat::Rgba32Float),
    ];
    crate::graph::Upload::preserving_format(tier, features)
        .map_err(|_| Section183Error::Unavailable)
}

fn require_frame(outcome: FrameOutcome) -> Result<(), Section183Error> {
    match outcome {
        FrameOutcome::Submitted {
            viewports: 1,
            passes: 1,
        } => Ok(()),
        FrameOutcome::Unavailable { .. } => Err(Section183Error::Unavailable),
        FrameOutcome::Submitted { .. } | FrameOutcome::Noop | FrameOutcome::DeviceLost => {
            Err(Section183Error::NotSubmitted)
        }
    }
}

fn window_chain(function: VoiFunction) -> Result<LutChain, Section183Error> {
    let modality = ModalityTransform::new(None, Some(1.0), Some(0.0))
        .map_err(|error| Section183Error::Readback(format!("{error:?}")))?;
    let voi = VoiTransform::new(None, &[40.0], &[400.0], 0, function, 0.0, 255.0)
        .map_err(|error| Section183Error::Readback(format!("{error:?}")))?;
    LutChain::new(
        modality,
        voi,
        PhotometricInterpretation::Monochrome2,
        PresentationLutEvidence::Absent,
    )
    .map_err(|error| Section183Error::Readback(format!("{error:?}")))
}

fn cpu_row(chain: &LutChain) -> Result<[f32; 4], Section183Error> {
    let stored = [
        Stored(INPUT_LOW),
        Stored(INPUT_CENTRE),
        Stored(INPUT_HIGH),
        Stored(INPUT_EDGE),
    ];
    let mut destination = [Display(0.0), Display(0.0), Display(0.0), Display(0.0)];
    chain
        .map_into(&stored, &mut destination)
        .map_err(|error| Section183Error::Readback(format!("{error:?}")))?;
    let mut row = [0.0, 0.0, 0.0, 0.0];
    for (slot, display) in row.iter_mut().zip(destination) {
        *slot = display.0;
    }
    Ok(row)
}

fn colour_target(device: &wgpu::Device) -> (wgpu::TextureView, wgpu::Texture) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("ocelli section 18.3 target"),
        size: wgpu::Extent3d {
            width: 4,
            height: 1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: COLOUR_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    (view, texture)
}

fn read_displays(ctx: &GpuContext, texture: &wgpu::Texture) -> Result<[f32; 4], Section183Error> {
    let units = read_units(ctx, texture)?;
    if units.len() != 4 {
        return Err(Section183Error::Readback("short display row".to_owned()));
    }
    let mut displays = [0.0, 0.0, 0.0, 0.0];
    for (slot, unit) in displays.iter_mut().zip(units) {
        *slot = unit * 255.0;
    }
    Ok(displays)
}

fn await_map(
    device: &wgpu::Device,
    receiver: &std::sync::mpsc::Receiver<Result<(), wgpu::BufferAsyncError>>,
) -> Result<(), Section183Error> {
    // WebGL cannot `clientWaitSync` for a non-zero timeout. `Poll` reads the
    // fence without that wait. The probe instance uses `AutoFinish`, so the
    // first poll sees the submission as done. A tight loop cannot yield to
    // the browser, so this stops instead of spinning.
    #[cfg(target_arch = "wasm32")]
    {
        const ATTEMPTS: u32 = 4;
        for _ in 0..ATTEMPTS {
            if let Err(error) = device.poll(wgpu::PollType::Poll) {
                return Err(Section183Error::Readback(error.to_string()));
            }
            match receiver.try_recv() {
                Ok(Ok(())) => return Ok(()),
                Ok(Err(error)) => {
                    return Err(Section183Error::Readback(format!("{error:?}")));
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {}
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    return Err(Section183Error::Readback(
                        "map callback did not run".to_owned(),
                    ));
                }
            }
        }
        return Err(Section183Error::Readback(
            "map callback did not run".to_owned(),
        ));
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        if let Err(error) = device.poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: None,
        }) {
            return Err(Section183Error::Readback(error.to_string()));
        }
        let received = receiver
            .recv()
            .map_err(|error| Section183Error::Readback(format!("{error:?}")))?;
        received.map_err(|error| Section183Error::Readback(format!("{error:?}")))?;
        Ok(())
    }
}

fn read_units(ctx: &GpuContext, texture: &wgpu::Texture) -> Result<[f32; 4], Section183Error> {
    let device = ctx.device();
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("ocelli section 18.3 readback"),
        size: 256,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("ocelli section 18.3 readback"),
    });
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture,
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
            width: 4,
            height: 1,
            depth_or_array_layers: 1,
        },
    );
    ctx.queue().submit(std::iter::once(encoder.finish()));
    let slice = buffer.slice(..);
    let (sender, receiver) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = sender.send(result);
    });
    await_map(device, &receiver)?;
    let data = slice
        .get_mapped_range()
        .map_err(|error| Section183Error::Readback(format!("{error:?}")))?;
    let bytes: &[u8] = &data;
    let mut units = [0.0, 0.0, 0.0, 0.0];
    for (index, slot) in units.iter_mut().enumerate() {
        let start = index.saturating_mul(16);
        let Some(end) = start.checked_add(4) else {
            return Err(Section183Error::Readback("readback offset".to_owned()));
        };
        let Some(chunk) = bytes.get(start..end) else {
            return Err(Section183Error::Readback("short map".to_owned()));
        };
        let Ok(array) = <[u8; 4]>::try_from(chunk) else {
            return Err(Section183Error::Readback("short texel".to_owned()));
        };
        *slot = f32::from_le_bytes(array);
    }
    Ok(units)
}
