//! F-052. An orthographic plane is uploaded only when every sample has landed.
//!
//! PS3.3 C.7.6.2.1.1 places the voxel. The window is PS3.3 C.11.2.1.2,
//! LINEAR, centre 40, width 400, output 0 to 255. Expected codes are
//! `LutChain::map_into` of the stored samples. The bound is F-041's sweep
//! bound, two `f32` ULP at 255.
//!
//! The GPU rows are `#[ignore]`d. Tier C needs no device.

use glam::DMat3;
use ocelli_core::{Display, Pt, Stored, World};
use ocelli_pixel::{
    LutChain, ModalityTransform, PhotometricInterpretation, PresentationLutEvidence, VoiFunction,
    VoiTransform,
};
use ocelli_render::{
    BlendMode, FrameOutcome, OrthoError, Pass, RenderGraph, StackPass, Tier, VoiParams,
    present_plane,
};
use ocelli_volume::{
    Level, OrthoImage, OrthoPlane, PlaneSample, SlicePresence, Volume, VoxelKind, VoxelStore,
};

/// F-041's asserted sweep bound. Two `f32` ULP at 255 sit inside it.
const TWO_ULP_AT_255: f32 = 1e-4;

#[test]
fn ortho_viewport_is_unavailable_on_tier_c() {
    let image = row(&[PlaneSample::Present(1.0), PlaneSample::Present(2.0)]);
    let mut calls = 0_u32;
    let refused = present_plane(Tier::Cpu, &image, |_| {
        calls = calls.saturating_add(1);
        Ok(())
    });
    assert!(matches!(refused, Err(OrthoError::TierCUnavailable)));
    assert_eq!(calls, 0);
    let missing = row(&[PlaneSample::Absent]);
    let refused = present_plane(Tier::Cpu, &missing, |_| {
        calls = calls.saturating_add(1);
        Ok(())
    });
    assert!(matches!(refused, Err(OrthoError::TierCUnavailable)));
    assert_eq!(calls, 0);
}

#[test]
fn an_absent_sample_is_not_uploaded() {
    let image = row(&[
        PlaneSample::Present(1.0),
        PlaneSample::Absent,
        PlaneSample::Present(11.0),
    ]);
    let mut calls = 0_u32;
    let refused = present_plane(Tier::A, &image, |_| {
        calls = calls.saturating_add(1);
        Ok(())
    });
    assert!(matches!(refused, Err(OrthoError::PlaneIncomplete)));
    assert_eq!(calls, 0);
}

#[test]
fn a_complete_plane_hands_stored_values_to_the_upload() {
    let image = row(&[
        PlaneSample::Present(0.0),
        PlaneSample::Present(1.0),
        PlaneSample::Present(10.0),
        PlaneSample::Present(11.0),
    ]);
    let mut seen = Vec::new();
    let uploaded = present_plane(Tier::A, &image, |samples| {
        seen = samples.to_vec();
        Ok(())
    });
    assert!(uploaded.is_ok());
    assert_eq!(
        seen.iter().copied().map(f32::to_bits).collect::<Vec<_>>(),
        [0.0_f32, 1.0, 10.0, 11.0].map(f32::to_bits)
    );
}

fn row(samples: &[PlaneSample]) -> OrthoImage {
    OrthoImage {
        width: u32::try_from(samples.len()).unwrap_or(0),
        height: 1,
        column_mm: 1.0,
        row_mm: 1.0,
        samples: samples.to_vec(),
    }
}

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
    match built {
        Ok(chain) => chain,
        Err(_) => unreachable!("slope 1 and a LINEAR window are a chain"),
    }
}

fn params(chain: &LutChain) -> VoiParams {
    match VoiParams::from_chain(chain) {
        Ok(voi) => voi,
        Err(_) => unreachable!("this chain has a uniform"),
    }
}

fn cpu_display(chain: &LutChain, inputs: &[f32]) -> Vec<f32> {
    let stored: Vec<Stored> = inputs.iter().copied().map(Stored).collect();
    let mut destination = Vec::new();
    destination.resize(stored.len(), Display(0.0));
    let mapped = chain.map_into(&stored, &mut destination);
    assert!(mapped.is_ok());
    destination.iter().map(|display| display.0).collect()
}

fn target(device: &wgpu::Device, width: u32, height: u32) -> (wgpu::TextureView, wgpu::Texture) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("ortho target"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    (view, texture)
}

fn read_display(
    context: &ocelli_render::GpuContext,
    texture: &wgpu::Texture,
    width: u32,
    height: u32,
) -> Vec<f32> {
    let rows = usize::try_from(height).unwrap_or(0);
    let cols = usize::try_from(width).unwrap_or(0);
    let size = 256_u64.saturating_mul(u64::try_from(rows).unwrap_or(0));
    let device = context.device();
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("ortho readback"),
        size,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("ortho readback"),
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
    let bytes: &[u8] = &data;
    let mut display = Vec::new();
    for row in 0..rows {
        for col in 0..cols {
            let start = row
                .saturating_mul(256)
                .saturating_add(col.saturating_mul(16));
            let Some(end) = start.checked_add(4) else {
                return display;
            };
            let Some(chunk) = bytes.get(start..end) else {
                return display;
            };
            let Ok(array) = <[u8; 4]>::try_from(chunk) else {
                return display;
            };
            display.push(f32::from_le_bytes(array) * 255.0);
        }
    }
    display
}

fn assert_within_bound(actual: &[f32], expected: &[f32]) {
    assert_eq!(actual.len(), expected.len());
    for (index, want) in expected.iter().copied().enumerate() {
        let got = actual.get(index).copied().unwrap_or(f32::NAN);
        let difference = (got - want).abs();
        assert!(
            difference < TWO_ULP_AT_255,
            "index {index}: was {got}, wanted {want}, difference {difference}"
        );
    }
}

fn fixture() -> Volume {
    let mut volume = Volume {
        dims: [2, 2, 2],
        spacing: [0.5, 2.0, 3.0],
        origin: Pt::<World>::new(10.0, 20.0, 30.0),
        direction: DMat3::IDENTITY,
        voxel: VoxelKind::U8,
        data: VoxelStore::new([2, 2, 2], VoxelKind::U8),
        levels: vec![Level {
            dims: [2, 2, 2],
            spacing: [0.5, 2.0, 3.0],
            offset: 0,
        }],
        present: SlicePresence::new(2),
        order: Vec::new(),
    };
    assert!(volume.place(0, &[0, 1, 10, 11]).is_ok());
    volume
}

fn stored_of(image: &OrthoImage) -> Vec<f32> {
    image
        .samples
        .iter()
        .map(|sample| match sample {
            PlaneSample::Present(value) => *value,
            PlaneSample::Absent => f32::NAN,
        })
        .collect()
}

#[test]
#[ignore = "needs a real GPU adapter, which the CI floor does not have (D-04)"]
fn axial_upload_matches_the_lut_chain() {
    let Some((adapter, mut context)) = session() else {
        println!("this machine resolved tier C, so there is no device");
        return;
    };
    let built = RenderGraph::new(&mut context, 1, wgpu::TextureFormat::Rgba32Float);
    assert!(built.is_ok());
    let Ok(mut graph) = built else { return };
    let chain = chain();
    let (view, texture) = target(context.device(), 2, 2);
    let registered = graph.register_stored(
        &context,
        &adapter,
        view,
        2,
        2,
        Pass::Stack(StackPass {
            voi: params(&chain),
            blend: BlendMode::Composite,
        }),
    );
    assert!(registered.is_ok());
    let Ok(viewport) = registered else { return };
    let mut volume = fixture();
    assert!(volume.place(1, &[100, 101, 110, 111]).is_ok());
    let axial = volume.reslice(OrthoPlane::Axial { slice: 0.0 });
    let Ok(axial) = axial else { return };
    assert_eq!(graph.uploads(), 0);
    assert!(
        graph
            .show_ortho(&context, &adapter, viewport, &axial)
            .is_ok()
    );
    assert_eq!(graph.uploads(), 1);
    assert_eq!(
        graph.tick(&context),
        FrameOutcome::Submitted {
            viewports: 1,
            passes: 1
        }
    );
    let read = read_display(&context, &texture, 2, 2);
    assert_within_bound(&read, &cpu_display(&chain, &stored_of(&axial)));

    let sagittal = volume.reslice(OrthoPlane::Sagittal { column: 1.0 });
    let Ok(sagittal) = sagittal else { return };
    assert!(
        graph
            .show_ortho(&context, &adapter, viewport, &sagittal)
            .is_ok()
    );
    assert_eq!(graph.uploads(), 2);
    assert_eq!(
        graph.tick(&context),
        FrameOutcome::Submitted {
            viewports: 1,
            passes: 1
        }
    );
    let read = read_display(&context, &texture, 2, 2);
    assert_within_bound(&read, &cpu_display(&chain, &stored_of(&sagittal)));
}

#[test]
#[ignore = "needs a real GPU adapter, which the CI floor does not have (D-04)"]
fn an_absent_plane_does_not_replace_the_texture() {
    let Some((adapter, mut context)) = session() else {
        println!("this machine resolved tier C, so there is no device");
        return;
    };
    let built = RenderGraph::new(&mut context, 1, wgpu::TextureFormat::Rgba32Float);
    assert!(built.is_ok());
    let Ok(mut graph) = built else { return };
    let chain = chain();
    let (view, texture) = target(context.device(), 2, 2);
    let registered = graph.register_stored(
        &context,
        &adapter,
        view,
        2,
        2,
        Pass::Stack(StackPass {
            voi: params(&chain),
            blend: BlendMode::Composite,
        }),
    );
    assert!(registered.is_ok());
    let Ok(viewport) = registered else { return };
    let volume = fixture();
    let axial = volume.reslice(OrthoPlane::Axial { slice: 0.0 });
    let Ok(axial) = axial else { return };
    assert!(
        graph
            .show_ortho(&context, &adapter, viewport, &axial)
            .is_ok()
    );
    assert_eq!(graph.uploads(), 1);
    assert_eq!(
        graph.tick(&context),
        FrameOutcome::Submitted {
            viewports: 1,
            passes: 1
        }
    );
    let ramp = read_display(&context, &texture, 2, 2);
    assert_within_bound(&ramp, &cpu_display(&chain, &stored_of(&axial)));
    let missing = volume.reslice(OrthoPlane::Axial { slice: 1.0 });
    let Ok(missing) = missing else { return };
    assert!(matches!(
        graph.show_ortho(&context, &adapter, viewport, &missing),
        Err(OrthoError::PlaneIncomplete)
    ));
    assert_eq!(graph.uploads(), 1);
    assert_eq!(graph.tick(&context), FrameOutcome::Noop);
    let kept = read_display(&context, &texture, 2, 2);
    assert_within_bound(&kept, &ramp);
}
