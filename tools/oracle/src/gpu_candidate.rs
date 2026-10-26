//! Bounded native tier A candidate using the existing stack graph and Grey LUT.
//! Readback belongs to this native oracle, never to the public shell boundary.

use ocelli_core::Stored;
use ocelli_pixel::PhotometricInterpretation;
use ocelli_render::{
    BlendMode, ColourLut, FrameOutcome, GpuContext, Pass, PresetId, RenderGraph, ResolvedAdapter,
    StackPass, Tier, TierRequest, VoiParams, resolve_adapter,
};
use ocelli_viewport::presentation::Presentation;
use ocelli_viewport::view::View;

pub(crate) struct GpuStack {
    adapter: ResolvedAdapter,
    context: GpuContext,
    graph: RenderGraph,
    grey: ColourLut,
}

impl GpuStack {
    pub fn open() -> Result<Self, &'static str> {
        let start = std::time::Instant::now();
        let mut clock = || u64::try_from(start.elapsed().as_nanos()).unwrap_or(u64::MAX);
        let adapter = pollster::block_on(resolve_adapter(TierRequest::Auto, &mut clock))
            .ok_or("tier-a-unavailable")?;
        let mut context = pollster::block_on(adapter.open()).map_err(|_| "tier-a-unavailable")?;
        if context.caps().tier != Tier::A {
            return Err("tier-a-unavailable");
        }
        let graph = RenderGraph::new(&mut context, 1, wgpu::TextureFormat::Rgba8Unorm)
            .map_err(|_| "gpu-graph-refused")?;
        let grey = ColourLut::preset(PresetId::Grey, PhotometricInterpretation::Monochrome2)
            .map_err(|_| "gpu-grey-refused")?;
        Ok(Self {
            adapter,
            context,
            graph,
            grey,
        })
    }

    pub fn execution(&self) -> serde_json::Value {
        serde_json::json!({
            "tier": "A", "entry": "native RenderGraph stack with Grey LUT",
            "resolution": format!("{:?}", self.adapter.resolution()),
            "wgpu": "30.0.1", "browser": false,
        })
    }

    pub fn draw(
        &mut self,
        view: &View,
        presentation: &Presentation,
        stored: &[Stored],
        source: [u32; 2],
        target: [u32; 2],
    ) -> Result<Vec<u8>, &'static str> {
        let limit = self.context.device().limits().max_texture_dimension_2d;
        if source
            .into_iter()
            .chain(target)
            .any(|n| n == 0 || n > limit)
        {
            return Err("gpu-texture-extent-refused");
        }
        let extent = wgpu::Extent3d {
            width: target[0],
            height: target[1],
            depth_or_array_layers: 1,
        };
        let texture = self
            .context
            .device()
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("oracle settled tier A"),
                size: extent,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: wgpu::TextureFormat::Rgba8Unorm,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            });
        let voi = VoiParams::from_chain(presentation.chain()).map_err(|_| "gpu-voi-refused")?;
        let id = self
            .graph
            .register_stored(
                &self.context,
                &self.adapter,
                texture.create_view(&Default::default()),
                source[0],
                source[1],
                Pass::Stack(StackPass {
                    voi,
                    blend: BlendMode::Composite,
                }),
            )
            .map_err(|_| "gpu-upload-refused")?;
        let drawn = (|| {
            self.graph
                .set_view(
                    &self.context,
                    id,
                    view.camera_matrix(source[0], source[1]),
                    1.0,
                )
                .map_err(|_| "gpu-camera-refused")?;
            // Presentation.colormap is Grey in this bounded monochrome producer.
            // Select the same declared preset on both tiers. Plain VOI output
            // without the preset has a different quantization path.
            self.graph
                .select_colour_lut(&self.context, &self.adapter, id, &self.grey)
                .map_err(|_| "gpu-grey-refused")?;
            let samples: Vec<f32> = stored.iter().map(|sample| sample.0).collect();
            self.graph
                .upload_stored(
                    &self.context,
                    &self.adapter,
                    id,
                    source[0],
                    source[1],
                    &samples,
                )
                .map_err(|_| "gpu-upload-refused")?;
            if !matches!(
                self.graph.tick(&self.context),
                FrameOutcome::Submitted { .. }
            ) {
                return Err("gpu-frame-not-submitted");
            }
            readback(&self.context, &texture, target)
        })();
        self.graph
            .remove(id)
            .map_err(|_| "gpu-slot-remove-refused")?;
        drawn
    }
}

fn readback(
    context: &GpuContext,
    texture: &wgpu::Texture,
    target: [u32; 2],
) -> Result<Vec<u8>, &'static str> {
    let row = target[0].checked_mul(4).ok_or("gpu-readback-extent")?;
    let padded = row.checked_add(255).ok_or("gpu-readback-extent")? / 256 * 256;
    let buffer = context.device().create_buffer(&wgpu::BufferDescriptor {
        label: Some("oracle tier A readback"),
        size: u64::from(padded) * u64::from(target[1]),
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder = context.device().create_command_encoder(&Default::default());
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
                bytes_per_row: Some(padded),
                rows_per_image: Some(target[1]),
            },
        },
        wgpu::Extent3d {
            width: target[0],
            height: target[1],
            depth_or_array_layers: 1,
        },
    );
    context.queue().submit([encoder.finish()]);
    let slice = buffer.slice(..);
    let (send, recv) = std::sync::mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |result| {
        let _sent = send.send(result);
    });
    context
        .device()
        .poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(std::time::Duration::from_secs(30)),
        })
        .map_err(|_| "gpu-poll-refused")?;
    recv.recv_timeout(std::time::Duration::from_secs(30))
        .map_err(|_| "gpu-map-refused")?
        .map_err(|_| "gpu-map-refused")?;
    let mapped = slice.get_mapped_range().map_err(|_| "gpu-map-refused")?;
    let mut rgba = Vec::new();
    for bytes in mapped.chunks_exact(usize::try_from(padded).map_err(|_| "gpu-readback-extent")?) {
        rgba.extend_from_slice(
            bytes
                .get(..usize::try_from(row).map_err(|_| "gpu-readback-extent")?)
                .ok_or("gpu-readback-extent")?,
        );
    }
    Ok(rgba)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ocelli_pixel::{
        LutChain, ModalityTransform, PresentationLutEvidence, VoiFunction, VoiTransform,
    };
    use ocelli_viewport::presentation::Colormap;

    #[test]
    #[ignore = "exclusive native tier A adapter"]
    fn settled_native_stack_matches_hand_derived_linear_grey() -> Result<(), &'static str> {
        let mut gpu = GpuStack::open()?;
        let modality =
            ModalityTransform::new(None, Some(1.0), Some(0.0)).map_err(|_| "identity refused")?;
        let voi = VoiTransform::new(None, &[40.0], &[400.0], 0, VoiFunction::Linear, 0.0, 255.0)
            .map_err(|_| "LINEAR refused")?;
        let chain = LutChain::new(
            modality,
            voi,
            PhotometricInterpretation::Monochrome2,
            PresentationLutEvidence::Absent,
        )
        .map_err(|_| "monochrome chain refused")?;
        let presentation = Presentation::new(
            chain,
            PhotometricInterpretation::Monochrome2,
            PresentationLutEvidence::Absent,
            Colormap::Grey,
        );
        // PS3.3 C.11.2.1.2.1: -160 maps to 0, 40 maps to 127.81955.
        // The declared Grey table rounds the display address to byte 128.
        let samples: Vec<_> = (0..256)
            .map(|i| Stored(if i < 128 { -160.0 } else { 40.0 }))
            .collect();
        let actual = gpu.draw(&View::new(), &presentation, &samples, [16, 16], [16, 16])?;
        for (index, pixel) in actual.chunks_exact(4).enumerate() {
            let expected = if index < 128 { 0 } else { 128 };
            assert_eq!(pixel, &[expected, expected, expected, 255]);
        }
        assert_eq!(
            gpu.draw(&View::new(), &presentation, &samples, [16, 16], [0, 16]),
            Err("gpu-texture-extent-refused")
        );
        Ok(())
    }
}
