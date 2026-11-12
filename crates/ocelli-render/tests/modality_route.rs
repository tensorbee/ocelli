//! F-235, the modality route through the graph.
//!
//! Every device row here is `#[ignore]`d and `bin/ocelli.sh gate gpu` runs
//! it. The fixture values are hand-computed from PS3.3 C.11.1 and
//! C.11.2.1.3.2 with the F-227 review's pinned bits, never read off the
//! implementation: the working sits in each row's comment.

use ocelli_pixel::{
    LutChain, LutDescriptor, ModalityTransform, PaletteColorLut, PhotometricInterpretation,
    PresentationLutEvidence, VoiFunction, VoiTransform,
};
use ocelli_render::{
    BlendMode, ColourLut, ColourLutError, FrameOutcome, GraphError, IccProfilePresence, Pass,
    PresetId, RenderGraph, StackPass, StoredRoute, UploadError, ViewportId, VoiParams,
    volume_ray_admission,
};
use ocelli_volume::Volume;

/// The pinned pair A: the f32 nearest the decimal 9.006030268210452e-07 and
/// the f32 nearest 1854.0933.
const SLOPE_A: u32 = 0x3571_C0F8;
const INTERCEPT_A: u32 = 0x44E7_C2FC;
/// The pinned pair B, the F-227 cancellation rescale.
const SLOPE_B: u32 = 0x3D47_EF8C;
const INTERCEPT_B: u32 = 0xC52D_91AC;

fn from_bits(bits: u32) -> f32 {
    f32::from_bits(bits)
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

/// A chain over one rescale pair and one LINEAR_EXACT window.
fn pair_chain(slope: u32, intercept: u32, center: f32, width: f32) -> LutChain {
    let modality = ModalityTransform::new(None, Some(from_bits(slope)), Some(from_bits(intercept)));
    let voi = VoiTransform::new(
        None,
        &[center],
        &[width],
        0,
        VoiFunction::LinearExact,
        0.0,
        255.0,
    );
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

fn uniform_of(chain: &LutChain) -> VoiParams {
    match VoiParams::from_chain(chain) {
        Ok(voi) => voi,
        Err(_) => unreachable!("the fixture chain expresses a uniform"),
    }
}

/// A 2x1 `Rgba8Unorm` target plus its view.
fn target(device: &wgpu::Device, width: u32) -> (wgpu::TextureView, wgpu::Texture) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("modality route target"),
        size: wgpu::Extent3d {
            width,
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
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    (view, texture)
}

/// Bytes of the target's first `width` texels, `tests/graph.rs`'s pattern.
fn readback(context: &ocelli_render::GpuContext, target: &wgpu::Texture, width: u32) -> Vec<u8> {
    let device = context.device();
    let padded = 256_u32;
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("modality route readback"),
        size: u64::from(padded),
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("modality route readback"),
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
    let bytes: &[u8] = &data;
    let row = 4usize.saturating_mul(usize::try_from(width).unwrap_or(0));
    bytes.iter().take(row).copied().collect()
}

/// A caller-owned 2x1 `R32Float` source view holding `values`.
fn caller_source(context: &ocelli_render::GpuContext, values: &[f32]) -> wgpu::TextureView {
    let device = context.device();
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("modality route caller source"),
        size: wgpu::Extent3d {
            width: u32::try_from(values.len()).unwrap_or(1),
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
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        bytemuck::cast_slice(values),
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(256),
            rows_per_image: Some(1),
        },
        wgpu::Extent3d {
            width: u32::try_from(values.len()).unwrap_or(1),
            height: 1,
            depth_or_array_layers: 1,
        },
    );
    texture.create_view(&wgpu::TextureViewDescriptor::default())
}

fn require_frame(outcome: FrameOutcome) {
    assert!(
        matches!(outcome, FrameOutcome::Submitted { .. }),
        "the frame was not submitted: {outcome:?}"
    );
}

fn palette_lut() -> PaletteColorLut {
    // Two entries mapped from stored 8: the first texel red, the second blue.
    let red = LutDescriptor::new(2, 8, 8, vec![255.0, 0.0]);
    assert!(red.is_ok(), "a two-entry red channel: {red:?}");
    let green = LutDescriptor::new(2, 8, 8, vec![0.0, 0.0]);
    assert!(green.is_ok(), "a two-entry green channel: {green:?}");
    let blue = LutDescriptor::new(2, 8, 8, vec![0.0, 255.0]);
    assert!(blue.is_ok(), "a two-entry blue channel: {blue:?}");
    let (Ok(red), Ok(green), Ok(blue)) = (red, green, blue) else {
        unreachable!("asserted immediately above")
    };
    let triple = PaletteColorLut::new(red, green, blue);
    assert!(triple.is_ok(), "a palette descriptor triple: {triple:?}");
    match triple {
        Ok(built) => built,
        Err(_) => unreachable!("asserted immediately above"),
    }
}

fn palette_colour() -> ColourLut {
    let baked = ColourLut::palette(
        palette_lut(),
        PhotometricInterpretation::PaletteColor,
        IccProfilePresence::Absent,
    );
    assert!(baked.is_ok(), "the palette bakes: {baked:?}");
    match baked {
        Ok(built) => built,
        Err(_) => unreachable!("asserted immediately above"),
    }
}

fn preset_colour() -> ColourLut {
    let baked = ColourLut::preset(PresetId::Grey, PhotometricInterpretation::Monochrome2);
    assert!(baked.is_ok(), "the grey preset bakes: {baked:?}");
    match baked {
        Ok(built) => built,
        Err(_) => unreachable!("asserted immediately above"),
    }
}

/// **AC8, the pre-applied route through the graph, pinned.**
///
/// A `PreApplied` viewport with pass rescale pair A uploads the fractional
/// samples 2371.5 and 2372.5 as a 2x1 frame. The graph pre-applies stage 1
/// on the CPU with the recorded pair, staging `0x44E7C30D` and
/// `0x44E7C30E`, and the uniform's rescale is identity, so the shader's
/// flag-0 branch must not multiply again. Under LINEAR_EXACT centre
/// `0x44E7C333` width `0x3C23D70A` output 0 to 255 the hand displays are
/// 9.2138671875 and 12.32666015625, whose encodes are 9 and 12. Hand
/// working: staged minus centre is -0.004638671875 and 0.0045166015625,
/// over the pinned width plus one half, times 255.
///
/// The rejected wrong implementations: the uniform keeping the real rescale
/// while the flag-0 branch multiplies, which applies stage 1 twice and
/// draws 0 and 0, and no pre-apply at all, which draws 255.
#[test]
#[ignore = "needs a real GPU adapter, which the CI floor does not have (D-04)"]
fn the_pre_applied_route_draws_the_hand_displays() {
    let Some((adapter, mut context)) = session() else {
        println!("this machine resolved tier C, so there is no device");
        return;
    };
    let mut graph = match RenderGraph::new(&mut context, 1, wgpu::TextureFormat::Rgba8Unorm) {
        Ok(graph) => graph,
        Err(_) => return,
    };
    let chain = pair_chain(
        SLOPE_A,
        INTERCEPT_A,
        from_bits(0x44E7_C333),
        from_bits(0x3C23_D70A),
    );
    let (view, texture) = target(context.device(), 2);
    let viewport = match graph.register_stored(
        &context,
        &adapter,
        view,
        2,
        1,
        Pass::Stack(StackPass {
            voi: uniform_of(&chain),
            blend: BlendMode::Composite,
        }),
        StoredRoute::PreApplied,
    ) {
        Ok(id) => id,
        Err(error) => unreachable!("the pre-applied registration fits: {error:?}"),
    };
    assert!(
        graph
            .upload_stored(&context, &adapter, viewport, 2, 1, &[2371.5, 2372.5])
            .is_ok()
    );
    require_frame(graph.tick(&context));
    assert_eq!(
        readback(&context, &texture, 2),
        vec![9, 9, 9, 255, 12, 12, 12, 255]
    );
}

/// **AC6, the counters.**
///
/// A window drag on a tabled viewport writes the 32-byte `VoiParams` and
/// nothing else: `modality_uploads` unchanged, `uploads` unchanged. A
/// rescale change rebuilds the table exactly once and increments
/// `modality_uploads` by one.
#[test]
#[ignore = "needs a real GPU adapter, which the CI floor does not have (D-04)"]
fn a_window_drag_rebuilds_nothing_and_a_rescale_change_rebuilds_once() {
    let Some((adapter, mut context)) = session() else {
        println!("this machine resolved tier C, so there is no device");
        return;
    };
    let mut graph = match RenderGraph::new(&mut context, 1, wgpu::TextureFormat::Rgba8Unorm) {
        Ok(graph) => graph,
        Err(_) => return,
    };
    let first = pair_chain(SLOPE_A, INTERCEPT_A, from_bits(0x44E7_C30D), 2.0);
    let (view, _texture) = target(context.device(), 2);
    let viewport = match graph.register_stored(
        &context,
        &adapter,
        view,
        2,
        1,
        Pass::Stack(StackPass {
            voi: uniform_of(&first),
            blend: BlendMode::Composite,
        }),
        StoredRoute::Tabled {
            first: 0,
            count: 65_536,
        },
    ) {
        Ok(id) => id,
        Err(error) => unreachable!("the tabled registration fits: {error:?}"),
    };
    assert!(
        graph
            .upload_stored(&context, &adapter, viewport, 2, 1, &[2371.0, 2372.0])
            .is_ok()
    );
    assert_eq!(graph.uploads(), 1);
    assert_eq!(graph.modality_uploads(), 0);
    // A window-only change: the same rescale, a different width.
    let dragged = pair_chain(SLOPE_A, INTERCEPT_A, from_bits(0x44E7_C30D), 400.0);
    assert!(graph.set_voi(&context, viewport, &dragged).is_ok());
    assert_eq!(graph.modality_uploads(), 0, "a drag rebuilds no table");
    assert_eq!(graph.uploads(), 1);
    // A rescale change rebuilds the table once, whatever the bytes hold.
    let second = pair_chain(SLOPE_B, INTERCEPT_B, from_bits(0xC526_55EC), 2.0);
    assert!(graph.set_voi(&context, viewport, &second).is_ok());
    assert_eq!(graph.modality_uploads(), 1);
    assert_eq!(graph.uploads(), 1);
    // Repeating the same chain writes nothing new.
    assert!(graph.set_voi(&context, viewport, &second).is_ok());
    assert_eq!(graph.modality_uploads(), 1);
}

/// **AC9, the device refusals.**
///
/// An all-integral upload with one sample outside the declared container
/// refuses `SampleOutsideTable` and writes nothing, so `uploads` stays put
/// and a following valid upload still draws. A palette selection is refused
/// on a declared-`PreApplied` viewport before any upload, on staged
/// pre-applied content after one, and a non-palette selection is refused on
/// raw fractional content.
#[test]
#[ignore = "needs a real GPU adapter, which the CI floor does not have (D-04)"]
fn the_route_refusals_fire_and_preserve_the_previous_state() {
    let Some((adapter, mut context)) = session() else {
        println!("this machine resolved tier C, so there is no device");
        return;
    };
    let mut graph = match RenderGraph::new(&mut context, 4, wgpu::TextureFormat::Rgba8Unorm) {
        Ok(graph) => graph,
        Err(_) => return,
    };
    let chain = pair_chain(SLOPE_A, INTERCEPT_A, 1854.0, 100.0);
    // One: the out-of-range integral upload writes nothing.
    let (view, texture) = target(context.device(), 1);
    let outside = match graph.register_stored(
        &context,
        &adapter,
        view,
        1,
        1,
        Pass::Stack(StackPass {
            voi: uniform_of(&chain),
            blend: BlendMode::Composite,
        }),
        StoredRoute::Tabled {
            first: 0,
            count: 256,
        },
    ) {
        Ok(id) => id,
        Err(error) => unreachable!("the 8-bit registration fits: {error:?}"),
    };
    assert!(matches!(
        graph.upload_stored(&context, &adapter, outside, 1, 1, &[300.0]),
        Err(UploadError::SampleOutsideTable)
    ));
    assert_eq!(graph.uploads(), 0, "the refused upload wrote nothing");
    // The positive control: the same viewport accepts an in-range upload.
    // Hand-computed: modality(40) under pair A is the intercept itself,
    // `0x44E7C2FC` = 1854.09326171875, because 40 * `0x3571C0F8` is about
    // 3.6e-5, under half a ULP at 1854. Under LINEAR_EXACT centre 1854.0
    // width 100.0 output 0 to 255 the display is
    // ((1854.09326171875 - 1854.0) / 100 + 0.5) * 255 = 127.73782348632812,
    // whose encode is 128.
    assert!(
        graph
            .upload_stored(&context, &adapter, outside, 1, 1, &[40.0])
            .is_ok()
    );
    assert_eq!(graph.uploads(), 1);
    require_frame(graph.tick(&context));
    assert_eq!(readback(&context, &texture, 1), vec![128, 128, 128, 255]);

    // Two: a palette on a declared-PreApplied viewport, before any upload.
    let (view, _texture) = target(context.device(), 1);
    let pre = match graph.register_stored(
        &context,
        &adapter,
        view,
        1,
        1,
        Pass::Stack(StackPass {
            voi: uniform_of(&chain),
            blend: BlendMode::Composite,
        }),
        StoredRoute::PreApplied,
    ) {
        Ok(id) => id,
        Err(error) => unreachable!("the pre-applied registration fits: {error:?}"),
    };
    assert!(matches!(
        graph.select_colour_lut(&context, &adapter, pre, &palette_colour()),
        Err(GraphError::ColourUnavailable)
    ));

    // Three: a palette on staged pre-applied content, after one upload.
    let (view, _texture) = target(context.device(), 1);
    let staged = match graph.register_stored(
        &context,
        &adapter,
        view,
        1,
        1,
        Pass::Stack(StackPass {
            voi: uniform_of(&chain),
            blend: BlendMode::Composite,
        }),
        StoredRoute::Tabled {
            first: 0,
            count: 65_536,
        },
    ) {
        Ok(id) => id,
        Err(error) => unreachable!("the tabled registration fits: {error:?}"),
    };
    assert!(
        graph
            .upload_stored(&context, &adapter, staged, 1, 1, &[2371.5])
            .is_ok()
    );
    assert!(matches!(
        graph.select_colour_lut(&context, &adapter, staged, &palette_colour()),
        Err(GraphError::ColourUnavailable)
    ));

    // Four: a non-palette selection on raw fractional content, which only a
    // palette colour mode keeps.
    let (view, _texture) = target(context.device(), 1);
    let fractional = match graph.register_stored(
        &context,
        &adapter,
        view,
        1,
        1,
        Pass::Stack(StackPass {
            voi: uniform_of(&chain),
            blend: BlendMode::Composite,
        }),
        StoredRoute::Tabled {
            first: 0,
            count: 65_536,
        },
    ) {
        Ok(id) => id,
        Err(error) => unreachable!("the tabled registration fits: {error:?}"),
    };
    assert!(
        graph
            .select_colour_lut(&context, &adapter, fractional, &palette_colour())
            .is_ok()
    );
    assert!(
        graph
            .upload_stored(&context, &adapter, fractional, 1, 1, &[2371.5])
            .is_ok()
    );
    assert!(matches!(
        graph.select_colour_lut(&context, &adapter, fractional, &preset_colour()),
        Err(GraphError::ColourUnavailable)
    ));
}

/// A 1x1x2 unsigned volume with the two texels the ray rows need.
fn ray_volume(low: u8, high: u8) -> Volume {
    use glam::DMat3;
    use ocelli_core::{Pt, World};
    use ocelli_volume::{Level, SlicePresence, VoxelKind, VoxelStore};
    let dims = [1, 1, 2];
    let mut volume = Volume {
        dims,
        spacing: [1.0, 1.0, 1.0],
        origin: Pt::<World>::new(0.0, 0.0, 0.0),
        direction: DMat3::IDENTITY,
        voxel: VoxelKind::U8,
        data: VoxelStore::new(dims, VoxelKind::U8),
        levels: vec![Level {
            dims,
            spacing: [1.0, 1.0, 1.0],
            offset: 0,
        }],
        present: SlicePresence::new(dims[2]),
        order: Vec::new(),
    };
    assert!(volume.place(0, &[low]).is_ok());
    assert!(volume.place(1, &[high]).is_ok());
    volume
}

/// **AC11, the ray's bind path.**
///
/// A declared container that agrees with the bricks' own format binds, and
/// one that disagrees refuses `RayUnavailable`. The arithmetic itself is the
/// floor unit beside `brick_slot_count`, which compiles 14 slots at a
/// sampled limit of 16 and 94 at the M4 Max's 96.
#[test]
#[ignore = "needs a real GPU adapter, which the CI floor does not have (D-04)"]
fn the_ray_binds_the_declared_container_and_refuses_a_disagreement() {
    let Some((adapter, mut context)) = session() else {
        println!("this machine resolved tier C, so there is no device");
        return;
    };
    let limit = ocelli_render::sampled_texture_limit(&adapter);
    let mut graph = match RenderGraph::new(&mut context, 3, wgpu::TextureFormat::Rgba8Unorm) {
        Ok(graph) => graph,
        Err(_) => return,
    };
    let volume = ray_volume(0, 255);
    assert!(
        graph
            .sync_volume(&context, &adapter, ocelli_cache::VolumeId(1), &volume)
            .is_ok()
    );
    let chain = pair_chain(0x3F80_0000, 0x0000_0000, 127.5, 255.0);
    let ready = volume.ready_bricks();
    let Some(coord) = ready.first().copied() else {
        unreachable!("one ready brick");
    };
    let key = ocelli_cache::BrickKey {
        volume: ocelli_cache::VolumeId(1),
        level: 0,
        brick: coord,
    };
    let bricks = [ocelli_render::VolumeBrick { key: &key, coord }];
    let ray_source = || ocelli_render::VolumeRaySource {
        bricks: &bricks,
        edge: volume.brick_grid().edge,
        dims: volume.dims,
        flags: &[1, 1],
        sampled_limit: limit,
        spacing_mm: [1.0, 1.0, 1.0],
        origin_mm: [0.0, 0.0, 0.0],
        direction: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
    };
    // The agreeing declaration binds.
    let (view, _texture) = target(context.device(), 1);
    let agreeing = match graph.register(
        &context,
        view,
        dummy_stack_source(context.device()),
        Pass::VolumeRaycast(ocelli_render::VolumePass {
            blend: BlendMode::Composite,
            voi: uniform_of(&chain),
            axis: ocelli_render::RayAxis::Slice,
            from_far: false,
        }),
        StoredRoute::Tabled {
            first: 0,
            count: 256,
        },
    ) {
        Ok(id) => id,
        Err(error) => unreachable!("the ray registration fits: {error:?}"),
    };
    assert!(
        graph
            .bind_volume_ray(&context, &adapter, agreeing, ray_source())
            .is_ok()
    );
    // The disagreeing declarations refuse.
    let (view, _texture) = target(context.device(), 1);
    let wrong_container = match graph.register(
        &context,
        view,
        dummy_stack_source(context.device()),
        Pass::VolumeRaycast(ocelli_render::VolumePass {
            blend: BlendMode::Composite,
            voi: uniform_of(&chain),
            axis: ocelli_render::RayAxis::Slice,
            from_far: false,
        }),
        StoredRoute::Tabled {
            first: 0,
            count: 65_536,
        },
    ) {
        Ok(id) => id,
        Err(error) => unreachable!("the ray registration fits: {error:?}"),
    };
    assert!(matches!(
        graph.bind_volume_ray(&context, &adapter, wrong_container, ray_source()),
        Err(GraphError::RayUnavailable)
    ));
    let (view, _texture) = target(context.device(), 1);
    let pre_applied = match graph.register(
        &context,
        view,
        dummy_stack_source(context.device()),
        Pass::VolumeRaycast(ocelli_render::VolumePass {
            blend: BlendMode::Composite,
            voi: uniform_of(&chain),
            axis: ocelli_render::RayAxis::Slice,
            from_far: false,
        }),
        StoredRoute::PreApplied,
    ) {
        Ok(id) => id,
        Err(error) => unreachable!("the ray registration fits: {error:?}"),
    };
    assert!(matches!(
        graph.bind_volume_ray(&context, &adapter, pre_applied, ray_source()),
        Err(GraphError::RayUnavailable)
    ));
}

fn dummy_stack_source(device: &wgpu::Device) -> wgpu::TextureView {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("modality route dummy source"),
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
    texture.create_view(&wgpu::TextureViewDescriptor::default())
}

/// The floor guard the bind test relies on, restated so a regression in the
/// pure half is visible without a device.
#[test]
fn the_ray_admission_room_is_the_sampled_limit_minus_two() {
    assert!(volume_ray_admission(ocelli_render::Tier::A, 14, 16).is_ok());
    assert!(matches!(
        volume_ray_admission(ocelli_render::Tier::A, 15, 16),
        Err(GraphError::SampledTextureLimit)
    ));
}

/// **AC13, the ortho flip, pinned.**
///
/// A `Tabled { first: 0, count: 65_536 }` viewport with pass rescale slope
/// 2.0 intercept -1024.0 uploads the fractional samples 2371.25 and 2371.5
/// as a 2x1 frame. The graph pre-applies, staging 3718.5 and 3719.0, and
/// under LINEAR_EXACT centre 3719.0 width 2.0 output 0 to 255 the readback
/// bytes are 64 and 128, the encodes of 63.75 and 127.5, equal to the CPU
/// chain's. An `i32`-truncating pre-apply reads entry 2371 for both, 3718.0,
/// and draws byte 0 for both. A following integral upload of 2371.0 and
/// 2372.0 flips back and the table serves it: bytes 0 and 255, the encodes
/// of the table entries 3718.0 and 3720.0 under the same window.
#[test]
#[ignore = "needs a real GPU adapter, which the CI floor does not have (D-04)"]
fn the_ortho_flip_pre_applies_fractional_and_flips_back_on_integral() {
    let Some((adapter, mut context)) = session() else {
        println!("this machine resolved tier C, so there is no device");
        return;
    };
    let mut graph = match RenderGraph::new(&mut context, 1, wgpu::TextureFormat::Rgba8Unorm) {
        Ok(graph) => graph,
        Err(_) => return,
    };
    let chain = {
        let modality = ModalityTransform::new(None, Some(2.0), Some(-1024.0));
        let voi = VoiTransform::new(
            None,
            &[3719.0],
            &[2.0],
            0,
            VoiFunction::LinearExact,
            0.0,
            255.0,
        );
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
        assert!(built.is_ok(), "the slope-2 chain builds: {built:?}");
        match built {
            Ok(chain) => chain,
            Err(_) => unreachable!("asserted immediately above"),
        }
    };
    let (view, texture) = target(context.device(), 2);
    let viewport = match graph.register_stored(
        &context,
        &adapter,
        view,
        2,
        1,
        Pass::Stack(StackPass {
            voi: uniform_of(&chain),
            blend: BlendMode::Composite,
        }),
        StoredRoute::Tabled {
            first: 0,
            count: 65_536,
        },
    ) {
        Ok(id) => id,
        Err(error) => unreachable!("the tabled registration fits: {error:?}"),
    };
    // Fractional: pre-applied, staged 3718.5 and 3719.0, bytes 64 and 128.
    assert!(
        graph
            .upload_stored(&context, &adapter, viewport, 2, 1, &[2371.25, 2371.5])
            .is_ok()
    );
    require_frame(graph.tick(&context));
    assert_eq!(
        readback(&context, &texture, 2),
        vec![64, 64, 64, 255, 128, 128, 128, 255],
        "a truncating pre-apply would draw byte 0 for both"
    );
    // Integral: the flip-back, the table serves 3718.0 and 3720.0.
    assert!(
        graph
            .upload_stored(&context, &adapter, viewport, 2, 1, &[2371.0, 2372.0])
            .is_ok()
    );
    require_frame(graph.tick(&context));
    assert_eq!(
        readback(&context, &texture, 2),
        vec![0, 0, 0, 255, 255, 255, 255, 255],
        "the flip-back did not return the table to the serving route"
    );
}

/// **AC14, the rescale change on pre-applied content, pinned.**
///
/// A `PreApplied` viewport with pass rescale pair A and initial chain window
/// LINEAR_EXACT centre `0x44E7C30D` width 2.0 output 0 to 255 uploads
/// 2371.5 and 2372.5, staged `0x44E7C30D` and `0x44E7C30E`. `set_voi` with
/// pair B's chain, window LINEAR_EXACT centre `0xC5265588` width 2.0 output
/// 0 to 255, records the new chain and marks the content stale: the next
/// frame encodes a clear-only pass and reads back `0, 0, 0, 255`. A
/// re-upload of the same samples re-pre-applies with the new pair, staged
/// `0xC5265588` and `0xC52654C0`, and reads back 128 and 134, the encodes
/// of 127.5 and 133.7255859375.
#[test]
#[ignore = "needs a real GPU adapter, which the CI floor does not have (D-04)"]
fn a_rescale_change_marks_stale_and_the_next_upload_recovers() {
    let Some((adapter, mut context)) = session() else {
        println!("this machine resolved tier C, so there is no device");
        return;
    };
    let mut graph = match RenderGraph::new(&mut context, 1, wgpu::TextureFormat::Rgba8Unorm) {
        Ok(graph) => graph,
        Err(_) => return,
    };
    let pair_a = pair_chain(SLOPE_A, INTERCEPT_A, from_bits(0x44E7_C30D), 2.0);
    let (view, texture) = target(context.device(), 2);
    let viewport = match graph.register_stored(
        &context,
        &adapter,
        view,
        2,
        1,
        Pass::Stack(StackPass {
            voi: uniform_of(&pair_a),
            blend: BlendMode::Composite,
        }),
        StoredRoute::PreApplied,
    ) {
        Ok(id) => id,
        Err(error) => unreachable!("the pre-applied registration fits: {error:?}"),
    };
    assert!(
        graph
            .upload_stored(&context, &adapter, viewport, 2, 1, &[2371.5, 2372.5])
            .is_ok()
    );
    // Pair B's chain: rescale `0x3D47EF8C` / `0xC52D91AC`, window centre
    // `0xC5265588` width 2.0.
    let pair_b = pair_chain(SLOPE_B, INTERCEPT_B, from_bits(0xC526_5588), 2.0);
    assert!(graph.set_voi(&context, viewport, &pair_b).is_ok());
    require_frame(graph.tick(&context));
    assert_eq!(
        readback(&context, &texture, 2),
        vec![0, 0, 0, 255, 0, 0, 0, 255],
        "the stale frame drew instead of clearing"
    );
    assert!(
        graph
            .upload_stored(&context, &adapter, viewport, 2, 1, &[2371.5, 2372.5])
            .is_ok()
    );
    require_frame(graph.tick(&context));
    assert_eq!(
        readback(&context, &texture, 2),
        vec![128, 128, 128, 255, 134, 134, 134, 255],
        "the re-upload did not re-apply the recorded pair"
    );
}

/// **AC17, the flip, the rescale change and the flip-back, pinned.**
///
/// A `Tabled { first: 0, count: 65_536 }` viewport with pass rescale slope
/// 2.0 intercept -1024.0 uploads the fractional samples 2371.25 and 2371.5:
/// pre-applied, staged 3718.5 and 3719.0, route uniform flag 0. `set_voi`
/// with pair B's chain rebuilds the table with the new pair
/// (`modality_uploads` +1) and marks the stale content: the next frame is
/// the clear colour. An integral upload of 2371.0 and 2372.0 flips back:
/// raw write, route uniform flag 1, VOI uniform with the real pair, stale
/// cleared. Under LINEAR_EXACT centre `0xC52655EC` width 2.0 output 0 to
/// 255 the readback bytes are 128 and 134, the encodes of 127.5 and
/// 133.7255859375. The rebuilt entry is pinned floor-side by the builder
/// unit in `src/voi.rs`: index 2371 holds `0xC52655EC`, not pair A's
/// 3718.0. Skipping the rebuild because the effective route is pre-applied
/// leaves pair A's values in the table and draws 255 and 255 here.
#[test]
#[ignore = "needs a real GPU adapter, which the CI floor does not have (D-04)"]
fn the_flip_rebuilds_the_table_and_the_flip_back_serves_it() {
    let Some((adapter, mut context)) = session() else {
        println!("this machine resolved tier C, so there is no device");
        return;
    };
    let mut graph = match RenderGraph::new(&mut context, 1, wgpu::TextureFormat::Rgba8Unorm) {
        Ok(graph) => graph,
        Err(_) => return,
    };
    let slope_two = {
        let modality = ModalityTransform::new(None, Some(2.0), Some(-1024.0));
        let voi = VoiTransform::new(
            None,
            &[3719.0],
            &[2.0],
            0,
            VoiFunction::LinearExact,
            0.0,
            255.0,
        );
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
        assert!(built.is_ok(), "the slope-2 chain builds: {built:?}");
        match built {
            Ok(chain) => chain,
            Err(_) => unreachable!("asserted immediately above"),
        }
    };
    let (view, texture) = target(context.device(), 2);
    let viewport = match graph.register_stored(
        &context,
        &adapter,
        view,
        2,
        1,
        Pass::Stack(StackPass {
            voi: uniform_of(&slope_two),
            blend: BlendMode::Composite,
        }),
        StoredRoute::Tabled {
            first: 0,
            count: 65_536,
        },
    ) {
        Ok(id) => id,
        Err(error) => unreachable!("the tabled registration fits: {error:?}"),
    };
    assert!(
        graph
            .upload_stored(&context, &adapter, viewport, 2, 1, &[2371.25, 2371.5])
            .is_ok()
    );
    let pair_b = pair_chain(SLOPE_B, INTERCEPT_B, from_bits(0xC526_55EC), 2.0);
    assert!(graph.set_voi(&context, viewport, &pair_b).is_ok());
    assert_eq!(
        graph.modality_uploads(),
        1,
        "the rescale change rebuilt the table"
    );
    require_frame(graph.tick(&context));
    assert_eq!(
        readback(&context, &texture, 2),
        vec![0, 0, 0, 255, 0, 0, 0, 255],
        "the stale content drew instead of clearing"
    );
    assert!(
        graph
            .upload_stored(&context, &adapter, viewport, 2, 1, &[2371.0, 2372.0])
            .is_ok()
    );
    require_frame(graph.tick(&context));
    assert_eq!(
        readback(&context, &texture, 2),
        vec![128, 128, 128, 255, 134, 134, 134, 255],
        "the flip-back drew the old pair's table or skipped stage 1"
    );
}

/// **AC19, the caller-view rescale change recovers, pinned.**
///
/// A caller-view `PreApplied` viewport registered as `canvas.rs` is, 2x1
/// with pair A staged in the caller's own view: the display bytes are 9 and
/// 12 as AC8. `set_voi` with pair B's chain succeeds, records the chain and
/// marks the content stale: the next frame's readback is the clear colour.
/// `upload_stored` returns `NotStored`, so the recovery is the caller's:
/// stage pair B's samples 2371.5 and 2372.5 pre-applied with the recorded
/// pair, `0xC5265588` and `0xC52654C0`, into a fresh view and `rebind`.
/// The next frame draws them under LINEAR_EXACT centre `0xC5265588` width
/// 2.0 through the identity-rescale uniform, bytes 128 and 134.
#[test]
#[ignore = "needs a real GPU adapter, which the CI floor does not have (D-04)"]
fn the_caller_view_recovers_through_rebind() {
    let Some((adapter, mut context)) = session() else {
        println!("this machine resolved tier C, so there is no device");
        return;
    };
    let mut graph = match RenderGraph::new(&mut context, 1, wgpu::TextureFormat::Rgba8Unorm) {
        Ok(graph) => graph,
        Err(_) => return,
    };
    let pair_a = pair_chain(
        SLOPE_A,
        INTERCEPT_A,
        from_bits(0x44E7_C333),
        from_bits(0x3C23_D70A),
    );
    let (view, texture) = target(context.device(), 2);
    let viewport = match graph.register(
        &context,
        view,
        caller_source(&context, &[from_bits(0x44E7_C30D), from_bits(0x44E7_C30E)]),
        Pass::Stack(StackPass {
            voi: uniform_of(&pair_a),
            blend: BlendMode::Composite,
        }),
        StoredRoute::PreApplied,
    ) {
        Ok(id) => id,
        Err(error) => unreachable!("the caller-view registration fits: {error:?}"),
    };
    require_frame(graph.tick(&context));
    assert_eq!(
        readback(&context, &texture, 2),
        vec![9, 9, 9, 255, 12, 12, 12, 255]
    );

    let pair_b = pair_chain(SLOPE_B, INTERCEPT_B, from_bits(0xC526_5588), 2.0);
    assert!(graph.set_voi(&context, viewport, &pair_b).is_ok());
    require_frame(graph.tick(&context));
    assert_eq!(
        readback(&context, &texture, 2),
        vec![0, 0, 0, 255, 0, 0, 0, 255],
        "the caller view drew a stale frame instead of clearing"
    );
    // No owned texture, so no upload path can clear the flag.
    assert!(matches!(
        graph.upload_stored(&context, &adapter, viewport, 2, 1, &[2371.5, 2372.5]),
        Err(UploadError::NotStored)
    ));
    // The caller stages pair B's samples itself, through
    // `ocelli_pixel::modality`, and rebinds with a fresh view over the same
    // target texture.
    let rebound_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let fresh = caller_source(
        &context,
        &[
            ocelli_pixel::modality(
                ocelli_core::Stored(2371.5),
                from_bits(SLOPE_B),
                from_bits(INTERCEPT_B),
            )
            .0,
            ocelli_pixel::modality(
                ocelli_core::Stored(2372.5),
                from_bits(SLOPE_B),
                from_bits(INTERCEPT_B),
            )
            .0,
        ],
    );
    assert!(
        graph
            .rebind(&context, viewport, rebound_view, fresh)
            .is_ok()
    );
    require_frame(graph.tick(&context));
    assert_eq!(
        readback(&context, &texture, 2),
        vec![128, 128, 128, 255, 134, 134, 134, 255],
        "the rebound caller view did not draw the fresh bytes"
    );
}

/// The palette refusal keeps its error shape in the floor, so a rename
/// cannot silently widen the guard.
#[test]
fn the_palette_refusal_error_is_colour_unavailable() {
    assert!(matches!(
        ColourLut::preset(PresetId::Grey, PhotometricInterpretation::PaletteColor),
        Err(ColourLutError::Unavailable)
    ));
}

/// The viewport id type stays a newtype over u32, which the readbacks index.
#[test]
fn viewport_ids_stay_newtype() {
    let id = ViewportId(0);
    assert_eq!(id, ViewportId(0));
}
