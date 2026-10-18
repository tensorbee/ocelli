//! F-040, stored samples uploaded with `write_texture`.
//!
//! The four rows are DICOM PS3.3 C.11.2.1.2 and C.11.2.1.3.2 at centre 40,
//! width 400, slope 1, intercept 0, output 0 to 255. Deviation D-13 keeps
//! `LINEAR_EXACT` at -160 on `ymin`. The tolerance is `0.001`, the same bound
//! `tests/voi_shader.rs` already uses, and it is not widened.
//!
//! The GPU rows are `#[ignore]`d. The length and format refusals need no device.

use ocelli_core::{Display, Stored};
use ocelli_pixel::{
    LutChain, ModalityTransform, PhotometricInterpretation, PresentationLutEvidence, VoiFunction,
    VoiTransform,
};
use ocelli_render::{
    BlendMode, FrameOutcome, GraphError, Pass, RenderGraph, StackPass, Upload, UploadError,
    VoiParams,
};

/// Not widened. Matches `tests/voi_shader.rs`.
const FIXTURE_TOLERANCE: f32 = 0.001;

/// PS3.3 C.11.2.1.2 and C.11.2.1.3.2 inputs, in stored units.
const INPUTS: [f32; 4] = [-160.0, 40.0, 240.0, -60.0];

/// LINEAR. The upper bound clamps 240. The centre is 127.819.
const LINEAR: [f32; 4] = [0.0, 127.819, 255.0, 63.910];

/// LINEAR_EXACT. D-13 keeps -160 on `ymin`, which is 0.000.
const LINEAR_EXACT: [f32; 4] = [0.0, 127.5, 255.0, 63.75];

fn binding_features(usages: wgpu::TextureUsages) -> wgpu::TextureFormatFeatures {
    wgpu::TextureFormatFeatures {
        allowed_usages: usages,
        flags: wgpu::TextureFormatFeatureFlags::empty(),
    }
}

#[test]
fn a_wrong_length_is_refused_and_a_missing_binding_is_unavailable() {
    let present =
        binding_features(wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST);
    let built = Upload::stored(present, 2, 2);
    assert!(built.is_ok());
    let Ok(upload) = built else { return };
    // 2 by 2 is four samples. Three is a refusal and uploads nothing.
    assert!(matches!(
        upload.accepts(&[0.0, 1.0, 2.0]),
        Err(UploadError::WrongLength)
    ));
    assert!(upload.accepts(&[0.0, 1.0, 2.0, 3.0]).is_ok());

    let no_binding = binding_features(wgpu::TextureUsages::COPY_DST);
    assert!(matches!(
        Upload::stored(no_binding, 2, 2),
        Err(UploadError::FormatUnavailable)
    ));
    // COPY_DST is the other usage `write_texture` needs. Missing it is the
    // same refusal, and no other format is produced.
    let binding_only = binding_features(wgpu::TextureUsages::TEXTURE_BINDING);
    assert!(matches!(
        Upload::stored(binding_only, 2, 2),
        Err(UploadError::FormatUnavailable)
    ));
    assert_eq!(ocelli_render::STORED_FORMAT, wgpu::TextureFormat::R32Float);
    assert_ne!(
        ocelli_render::STORED_FORMAT,
        wgpu::TextureFormat::Rgba8Unorm
    );
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

fn chain_at(function: VoiFunction, centre: f32) -> LutChain {
    let modality = ModalityTransform::new(None, Some(1.0), Some(0.0));
    let voi = VoiTransform::new(None, &[centre], &[400.0], 0, function, 0.0, 255.0);
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

fn params(chain: &LutChain) -> VoiParams {
    match VoiParams::from_chain(chain) {
        Ok(voi) => voi,
        Err(_) => unreachable!("chain must express"),
    }
}

fn cpu_row(chain: &LutChain, inputs: &[f32]) -> Vec<f32> {
    let stored: Vec<Stored> = inputs.iter().copied().map(Stored).collect();
    let mut destination = Vec::new();
    destination.resize(stored.len(), Display(0.0));
    let mapped = chain.map_into(&stored, &mut destination);
    assert!(mapped.is_ok());
    destination.iter().map(|display| display.0).collect()
}

fn assert_close(actual: f32, expected: f32) {
    let difference = (actual - expected).abs();
    assert!(
        difference < FIXTURE_TOLERANCE,
        "was {actual}, wanted {expected}, difference {difference}"
    );
}

fn assert_row(actual: &[f32], expected: &[f32]) {
    assert_eq!(actual.len(), expected.len());
    for (index, want) in expected.iter().copied().enumerate() {
        let got = actual.get(index).copied().unwrap_or(f32::NAN);
        let difference = (got - want).abs();
        assert!(
            difference < FIXTURE_TOLERANCE,
            "index {index}: was {got}, wanted {want}"
        );
    }
}

/// The fragment shader normalises a 0 to 255 display value into the target.
fn display_of(unit: f32) -> f32 {
    unit * 255.0
}

fn target(device: &wgpu::Device, width: u32, height: u32) -> (wgpu::TextureView, wgpu::Texture) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("stored upload target"),
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

fn read_units(context: &ocelli_render::GpuContext, target: &wgpu::Texture, width: u32) -> Vec<f32> {
    let device = context.device();
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("stored upload readback"),
        size: 256,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("stored upload readback"),
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
    let bytes: &[u8] = &data;
    let mut units = Vec::new();
    let count = usize::try_from(width).unwrap_or(0);
    for index in 0..count {
        let start = index.saturating_mul(16);
        let Some(end) = start.checked_add(4) else {
            break;
        };
        let Some(chunk) = bytes.get(start..end) else {
            break;
        };
        let Ok(array) = <[u8; 4]>::try_from(chunk) else {
            break;
        };
        units.push(f32::from_le_bytes(array));
    }
    units
}

fn displays(units: &[f32]) -> Vec<f32> {
    units.iter().copied().map(display_of).collect()
}

#[test]
#[ignore = "needs a real GPU adapter, which the CI floor does not have (D-04)"]
fn section_18_3_stored_samples_match_the_chain_and_voi_does_not_upload() {
    let Some((adapter, mut context)) = session() else {
        println!("this machine resolved tier C, so there is no device");
        return;
    };
    let built = RenderGraph::new(&mut context, 1, wgpu::TextureFormat::Rgba32Float);
    assert!(built.is_ok());
    let Ok(mut graph) = built else { return };
    let linear = chain_at(VoiFunction::Linear, 40.0);
    let (view, texture) = target(context.device(), 4, 1);
    let registered = graph.register_stored(
        &context,
        &adapter,
        view,
        4,
        1,
        Pass::Stack(StackPass {
            voi: params(&linear),
            blend: BlendMode::Composite,
        }),
    );
    assert!(registered.is_ok());
    let Ok(viewport) = registered else { return };
    assert_eq!(graph.uploads(), 0);
    assert!(
        graph
            .upload_stored(&context, &adapter, viewport, 4, 1, &INPUTS)
            .is_ok()
    );
    assert_eq!(graph.uploads(), 1);
    assert!(matches!(
        graph.upload_stored(&context, &adapter, viewport, 4, 1, &[0.0, 1.0, 2.0]),
        Err(UploadError::WrongLength)
    ));
    assert_eq!(graph.uploads(), 1);
    assert_eq!(
        graph.tick(&context),
        FrameOutcome::Submitted {
            viewports: 1,
            passes: 1
        }
    );
    let row = displays(&read_units(&context, &texture, 4));
    assert_row(&row, &LINEAR);
    assert_row(&row, &cpu_row(&linear, &INPUTS));

    let exact = chain_at(VoiFunction::LinearExact, 40.0);
    assert!(graph.set_voi(&context, viewport, &exact).is_ok());
    assert_eq!(graph.uploads(), 1);
    assert_eq!(
        graph.tick(&context),
        FrameOutcome::Submitted {
            viewports: 1,
            passes: 1
        }
    );
    let row = displays(&read_units(&context, &texture, 4));
    assert_row(&row, &LINEAR_EXACT);
    assert_row(&row, &cpu_row(&exact, &INPUTS));
    assert_close(row.first().copied().unwrap_or(f32::NAN), 0.0);

    // Centre 240, LINEAR. Input 40 lands on the lower bound, so it is ymin.
    // c' = 239.5, w' = 399, lower = 40, and 40 <= 40.
    let shifted = chain_at(VoiFunction::Linear, 240.0);
    assert!(graph.set_voi(&context, viewport, &shifted).is_ok());
    assert_eq!(graph.uploads(), 1);
    assert_eq!(
        graph.tick(&context),
        FrameOutcome::Submitted {
            viewports: 1,
            passes: 1
        }
    );
    let row = displays(&read_units(&context, &texture, 4));
    assert_close(row.get(1).copied().unwrap_or(f32::NAN), 0.0);
    assert_row(&row, &cpu_row(&shifted, &INPUTS));

    let replaced = [40.0, 40.0, 40.0, 40.0];
    assert!(
        graph
            .upload_stored(&context, &adapter, viewport, 4, 1, &replaced)
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
    let row = displays(&read_units(&context, &texture, 4));
    assert_row(&row, &cpu_row(&shifted, &replaced));
}

#[test]
#[ignore = "needs a real GPU adapter, which the CI floor does not have (D-04)"]
fn a_different_extent_replaces_the_stored_texture() {
    let Some((adapter, mut context)) = session() else {
        println!("this machine resolved tier C, so there is no device");
        return;
    };
    let built = RenderGraph::new(&mut context, 1, wgpu::TextureFormat::Rgba32Float);
    assert!(built.is_ok());
    let Ok(mut graph) = built else { return };
    let linear = chain_at(VoiFunction::Linear, 40.0);
    let (view, texture) = target(context.device(), 1, 1);
    let registered = graph.register_stored(
        &context,
        &adapter,
        view,
        1,
        1,
        Pass::Stack(StackPass {
            voi: params(&linear),
            blend: BlendMode::Composite,
        }),
    );
    assert!(registered.is_ok());
    let Ok(viewport) = registered else { return };
    assert!(
        graph
            .upload_stored(&context, &adapter, viewport, 1, 1, &[40.0])
            .is_ok()
    );
    assert_eq!(
        graph.tick(&context),
        FrameOutcome::Submitted {
            viewports: 1,
            passes: 1
        }
    );
    let first = displays(&read_units(&context, &texture, 1));
    assert_close(first.first().copied().unwrap_or(f32::NAN), 127.819);
    // A 2 by 2 extent does not fit the texture created at registration.
    // Replacing it is what lets stored 240 land on texel (0, 0). Writing the
    // larger size into the old texture would leave 127.819 in place.
    assert!(
        graph
            .upload_stored(&context, &adapter, viewport, 2, 2, &[240.0, 0.0, 0.0, 0.0])
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
    let second = displays(&read_units(&context, &texture, 1));
    assert_close(second.first().copied().unwrap_or(f32::NAN), 255.0);
}

#[test]
#[ignore = "needs a real GPU adapter, which the CI floor does not have (D-04)"]
fn recovery_recreates_the_stored_texture_without_counting_an_upload() {
    let Some((adapter, mut context)) = session() else {
        println!("this machine resolved tier C, so there is no device");
        return;
    };
    let built = RenderGraph::new(&mut context, 1, wgpu::TextureFormat::Rgba32Float);
    assert!(built.is_ok());
    let Ok(mut graph) = built else { return };
    let linear = chain_at(VoiFunction::Linear, 40.0);
    let (view, texture) = target(context.device(), 1, 1);
    let registered = graph.register_stored(
        &context,
        &adapter,
        view,
        1,
        1,
        Pass::Stack(StackPass {
            voi: params(&linear),
            blend: BlendMode::Composite,
        }),
    );
    assert!(registered.is_ok());
    let Ok(viewport) = registered else { return };
    assert!(
        graph
            .upload_stored(&context, &adapter, viewport, 1, 1, &[40.0])
            .is_ok()
    );
    assert_eq!(graph.uploads(), 1);
    let recovered = ocelli_render::Recovered {
        caps: *context.caps(),
        graph: Some(ocelli_render::GraphResources {
            pipelines_restored: 1,
            cache_generation: 2,
        }),
    };
    graph.install_recovered(&context, &recovered);
    assert!(matches!(
        graph.upload_stored(&context, &adapter, viewport, 1, 1, &[40.0]),
        Err(UploadError::NotRebound)
    ));
    assert_eq!(graph.uploads(), 1);
    let dummy_target = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let dummy_source = texture.create_view(&wgpu::TextureViewDescriptor::default());
    assert!(matches!(
        graph.rebind(&context, viewport, dummy_target, dummy_source),
        Err(GraphError::StoredNeedsRebind)
    ));
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    assert!(
        graph
            .rebind_stored(&context, &adapter, viewport, view)
            .is_ok()
    );
    assert_eq!(graph.uploads(), 1);
    assert!(
        graph
            .upload_stored(&context, &adapter, viewport, 1, 1, &[40.0])
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
    let row = displays(&read_units(&context, &texture, 1));
    assert_close(row.first().copied().unwrap_or(f32::NAN), 127.819);
}
