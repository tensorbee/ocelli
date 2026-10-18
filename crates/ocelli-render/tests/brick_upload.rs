//! F-043. Bricks upload at their true extent, in the sample's own format.
//!
//! The ramp is section 19's layout and PS3.3 C.7.6.3 row-major pixel data.
//! Voxel `(x, y, z)` of a 3 by 3 by 3 volume is `z * 9 + y * 3 + x`. The 27
//! values are 0 through 26. The texture is `[3, 3, 3]`. `write_texture` uses
//! a tight row of 3 bytes. A readback pads to 256 and then unpacks.
//!
//! The same two bytes `0x00 0x80` are `R16Sint` or `R16Uint`. PS3.3 C.7.6.3
//! Pixel Representation 0 is unsigned and 1 is signed. The bits are not a
//! number until the format says which.
//!
//! A 512 by 512 by 600 sixteen-bit volume is 8 by 8 by 10 bricks of edge 64.
//! Brick `[0, 0, 9]` starts at z 576 and is 24 deep, because 600 - 576 = 24.
//! Its texture is `[64, 64, 24]`, not `[512, 512, 600]`. An 8 by 8 by 8
//! eight-bit volume plans `[8, 8, 8]`, not a 128 cube. A `[130, 1, 1]`
//! eight-bit volume's second brick is width 2, not 128.
//!
//! The GPU row is `#[ignore]`d. The plan and the length check need no device.

use ocelli_cache::{Brick, BrickGrid, BrickKey, BrickSample, BrickWidth, VolumeId};
use ocelli_pixel::{
    LutChain, ModalityTransform, PhotometricInterpretation, PresentationLutEvidence, VoiFunction,
    VoiTransform,
};
use ocelli_render::{
    BlendMode, BrickPlan, BrickUploadError, Pass, RenderGraph, StackPass, Tier, VoiParams,
    brick_bytes_match, brick_format, brick_plan,
};

fn binding(usages: wgpu::TextureUsages) -> wgpu::TextureFormatFeatures {
    wgpu::TextureFormatFeatures {
        allowed_usages: usages,
        flags: wgpu::TextureFormatFeatureFlags::empty(),
    }
}

fn can_bind() -> wgpu::TextureFormatFeatures {
    binding(wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST)
}

fn plan_of(tier: Tier, limit: u32, brick: &Brick) -> Result<BrickPlan, BrickUploadError> {
    brick_plan(tier, limit, brick_format(brick.sample), can_bind(), brick)
}

/// Section 19, x fastest. Values 0 through 26.
fn ramp_3x3x3() -> Vec<u8> {
    let mut samples = Vec::new();
    let mut z = 0u8;
    while z < 3 {
        let mut y = 0u8;
        while y < 3 {
            let mut x = 0u8;
            while x < 3 {
                samples.push(z * 9 + y * 3 + x);
                x += 1;
            }
            y += 1;
        }
        z += 1;
    }
    samples
}

fn brick_of(extent: [u32; 3], samples: Vec<u8>, sample: BrickSample) -> Brick {
    Brick {
        extent,
        samples,
        sample,
    }
}

#[test]
fn a_3x3x3_ramp_plans_its_own_extent_and_a_tight_row() {
    let samples = ramp_3x3x3();
    assert_eq!(samples.len(), 27);
    assert_eq!(samples.first().copied(), Some(0));
    assert_eq!(samples.last().copied(), Some(26));
    let brick = brick_of([3, 3, 3], samples, BrickSample::U8);
    let planned = plan_of(Tier::A, 2048, &brick);
    assert!(planned.is_ok());
    let Ok(plan) = planned else { return };
    assert_eq!(plan.extent(), [3, 3, 3]);
    assert_eq!(plan.format(), wgpu::TextureFormat::R8Uint);
    assert_eq!(plan.bytes_per_row(), 3);
    assert_ne!(plan.bytes_per_row(), 256);
    assert_eq!(plan.rows_per_image(), 3);
    assert!(brick_bytes_match(&brick).is_ok());

    let mut short = ramp_3x3x3();
    short.pop();
    let short = brick_of([3, 3, 3], short, BrickSample::U8);
    assert!(matches!(
        brick_bytes_match(&short),
        Err(BrickUploadError::WrongLength)
    ));
}

#[test]
fn signed_and_unsigned_sixteen_bit_select_different_formats() {
    let bits = vec![0x00, 0x80];
    let signed = brick_of([1, 1, 1], bits.clone(), BrickSample::I16);
    let unsigned = brick_of([1, 1, 1], bits, BrickSample::U16);
    assert_eq!(signed.samples, unsigned.samples);
    assert_eq!(brick_format(BrickSample::I16), wgpu::TextureFormat::R16Sint);
    assert_eq!(brick_format(BrickSample::U16), wgpu::TextureFormat::R16Uint);
    assert_ne!(
        brick_format(BrickSample::I16),
        brick_format(BrickSample::U16)
    );
    assert_eq!(
        brick_format(BrickSample::F32),
        wgpu::TextureFormat::R32Float
    );
    assert_ne!(brick_format(BrickSample::U8), wgpu::TextureFormat::R8Unorm);
    assert_ne!(
        brick_format(BrickSample::U16),
        wgpu::TextureFormat::R16Unorm
    );
    assert_ne!(
        brick_format(BrickSample::I16),
        wgpu::TextureFormat::R32Float
    );

    let signed_plan = plan_of(Tier::A, 2048, &signed);
    assert!(signed_plan.is_ok());
    let Ok(signed_plan) = signed_plan else { return };
    assert_eq!(signed_plan.format(), wgpu::TextureFormat::R16Sint);
    let unsigned_plan = plan_of(Tier::B, 256, &unsigned);
    assert!(unsigned_plan.is_ok());
    let Ok(unsigned_plan) = unsigned_plan else {
        return;
    };
    assert_eq!(unsigned_plan.format(), wgpu::TextureFormat::R16Uint);

    assert!(matches!(
        brick_plan(
            Tier::A,
            2048,
            wgpu::TextureFormat::R32Float,
            can_bind(),
            &signed
        ),
        Err(BrickUploadError::FormatUnavailable)
    ));
    let eight = brick_of([1, 1, 1], vec![0], BrickSample::U8);
    assert!(matches!(
        brick_plan(
            Tier::A,
            2048,
            wgpu::TextureFormat::R8Unorm,
            can_bind(),
            &eight
        ),
        Err(BrickUploadError::FormatUnavailable)
    ));
}

#[test]
fn a_volume_that_fits_still_plans_brick_extents() {
    // Pinned wgpu 30.0.1. Tier B's 3D floor is 256. A brick edge of 128 fits.
    assert_eq!(
        wgpu::Limits::downlevel_defaults().max_texture_dimension_3d,
        256
    );
    let edge = brick_of([128, 1, 1], vec![0u8; 128], BrickSample::U8);
    let fitted = plan_of(Tier::B, 256, &edge);
    assert!(fitted.is_ok());
    let Ok(fitted) = fitted else { return };
    assert_eq!(fitted.extent(), [128, 1, 1]);

    let too_long = brick_of([257, 1, 1], vec![0u8; 257], BrickSample::U8);
    assert!(matches!(
        plan_of(Tier::B, 256, &too_long),
        Err(BrickUploadError::ExtentUnavailable)
    ));

    let grid = BrickGrid::new([512, 512, 600], BrickWidth::SixteenBit);
    assert_eq!(grid.grid, [8, 8, 10]);
    assert_eq!(grid.extent_of([0, 0, 9]), Ok([64, 64, 24]));
    let voxels = 64usize * 64 * 24;
    let mut samples = vec![0u8; voxels * 2];
    if let Some(first) = samples.get_mut(0) {
        *first = 0x00;
    }
    if let Some(second) = samples.get_mut(1) {
        *second = 0x80;
    }
    let ct = brick_of([64, 64, 24], samples, BrickSample::I16);
    let planned = plan_of(Tier::A, 2048, &ct);
    assert!(planned.is_ok());
    let Ok(plan) = planned else { return };
    assert_eq!(plan.extent(), [64, 64, 24]);
    assert_ne!(plan.extent(), [512, 512, 600]);
    assert_eq!(plan.bytes_per_row(), 128);
    assert_eq!(plan.format(), wgpu::TextureFormat::R16Sint);
    assert!(brick_bytes_match(&ct).is_ok());

    let small = brick_of([8, 8, 8], vec![0u8; 512], BrickSample::U8);
    let small_plan = plan_of(Tier::A, 2048, &small);
    assert!(small_plan.is_ok());
    let Ok(small_plan) = small_plan else { return };
    assert_eq!(small_plan.extent(), [8, 8, 8]);
    assert_ne!(small_plan.extent(), [128, 128, 128]);

    let wide = BrickGrid::new([130, 1, 1], BrickWidth::EightBit);
    assert_eq!(wide.grid, [2, 1, 1]);
    assert_eq!(wide.extent_of([1, 0, 0]), Ok([2, 1, 1]));
    let tail = brick_of([2, 1, 1], vec![9, 10], BrickSample::U8);
    let tail_plan = plan_of(Tier::B, 256, &tail);
    assert!(tail_plan.is_ok());
    let Ok(tail_plan) = tail_plan else { return };
    let [width, height, depth] = tail_plan.extent();
    assert_eq!(width, 2);
    assert_eq!(height, 1);
    assert_eq!(depth, 1);
    assert_ne!(width, 128);
    assert_eq!(tail_plan.bytes_per_row(), 2);
}

#[test]
fn tier_c_a_missing_usage_and_an_empty_extent_create_nothing() {
    let signed = brick_of([1, 1, 1], vec![0x00, 0x80], BrickSample::I16);
    assert!(matches!(
        brick_plan(
            Tier::Cpu,
            2048,
            wgpu::TextureFormat::R32Float,
            can_bind(),
            &signed
        ),
        Err(BrickUploadError::Unavailable)
    ));
    let eight = brick_of([2, 2, 2], vec![0u8; 8], BrickSample::U8);
    let copy_only = binding(wgpu::TextureUsages::COPY_DST);
    assert!(matches!(
        brick_plan(Tier::B, 256, wgpu::TextureFormat::R8Uint, copy_only, &eight),
        Err(BrickUploadError::FormatUnavailable)
    ));
    let bind_only = binding(wgpu::TextureUsages::TEXTURE_BINDING);
    assert!(matches!(
        brick_plan(
            Tier::A,
            2048,
            wgpu::TextureFormat::R8Uint,
            bind_only,
            &eight
        ),
        Err(BrickUploadError::FormatUnavailable)
    ));
    let empty = brick_of([3, 0, 3], Vec::new(), BrickSample::U8);
    assert!(matches!(
        plan_of(Tier::A, 2048, &empty),
        Err(BrickUploadError::EmptyExtent)
    ));
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

fn chain_at(function: VoiFunction) -> LutChain {
    let modality = ModalityTransform::new(None, Some(1.0), Some(0.0));
    let voi = VoiTransform::new(None, &[40.0], &[400.0], 0, function, 0.0, 255.0);
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

fn key_at(id: u64, brick: [u32; 3]) -> BrickKey {
    BrickKey {
        volume: VolumeId(id),
        level: 0,
        brick,
    }
}

fn target(device: &wgpu::Device) -> wgpu::TextureView {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("brick upload voi target"),
        size: wgpu::Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba32Float,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    texture.create_view(&wgpu::TextureViewDescriptor::default())
}

fn voxel_offset(x: usize, y: usize, z: usize, height: usize) -> Option<usize> {
    let stride = 256usize;
    let row = y.checked_mul(stride)?;
    let plane = stride.checked_mul(height)?.checked_mul(z)?;
    plane.checked_add(row)?.checked_add(x)
}

fn unpack_u8(copied: &[u8], extent: [u32; 3]) -> Vec<u8> {
    let [width, height, depth] = extent;
    let width_us = usize::try_from(width).unwrap_or(0);
    let height_us = usize::try_from(height).unwrap_or(0);
    let depth_us = usize::try_from(depth).unwrap_or(0);
    let mut out = Vec::new();
    let mut z = 0usize;
    while z < depth_us {
        let mut y = 0usize;
        while y < height_us {
            let mut x = 0usize;
            while x < width_us {
                let Some(offset) = voxel_offset(x, y, z, height_us) else {
                    return Vec::new();
                };
                let Some(byte) = copied.get(offset).copied() else {
                    return Vec::new();
                };
                out.push(byte);
                x += 1;
            }
            y += 1;
        }
        z += 1;
    }
    out
}

fn read_u8(
    context: &ocelli_render::GpuContext,
    texture: &wgpu::Texture,
    extent: [u32; 3],
) -> Vec<u8> {
    let [width, height, depth] = extent;
    let Some(size) = 256u64
        .checked_mul(u64::from(height))
        .and_then(|rows| rows.checked_mul(u64::from(depth)))
    else {
        return Vec::new();
    };
    let device = context.device();
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("brick readback"),
        size,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("brick readback"),
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
            depth_or_array_layers: depth,
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
    let copied = {
        let Ok(data) = slice.get_mapped_range() else {
            return Vec::new();
        };
        data.to_vec()
    };
    buffer.unmap();
    unpack_u8(&copied, extent)
}

fn read_resident(
    graph: &RenderGraph,
    context: &ocelli_render::GpuContext,
    key: &BrickKey,
    extent: [u32; 3],
    format: wgpu::TextureFormat,
) -> Vec<u8> {
    assert_eq!(graph.brick_extent(key), Some(extent));
    assert!(graph.brick_texture(key).is_some());
    let Some(texture) = graph.brick_texture(key) else {
        return Vec::new();
    };
    assert_eq!(texture.dimension(), wgpu::TextureDimension::D3);
    assert_eq!(texture.format(), format);
    let [width, height, depth] = extent;
    assert_eq!(
        texture.size(),
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: depth,
        }
    );
    assert!(texture.usage().contains(wgpu::TextureUsages::COPY_SRC));
    read_u8(context, texture, extent)
}

#[test]
#[ignore = "needs a real GPU adapter, which the CI floor does not have (D-04)"]
fn a_brick_upload_matches_the_ramp_and_a_window_does_not_reupload() {
    let Some((adapter, mut context)) = session() else {
        println!("this machine resolved tier C, so there is no device");
        return;
    };
    let built = RenderGraph::new(&mut context, 1, wgpu::TextureFormat::Rgba32Float);
    assert!(built.is_ok());
    let Ok(mut graph) = built else { return };
    let linear = chain_at(VoiFunction::Linear);
    let view = target(context.device());
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

    let ramp_key = key_at(1, [0, 0, 0]);
    let mut short = ramp_3x3x3();
    short.pop();
    let refused = graph.upload_brick(
        &context,
        &adapter,
        ramp_key,
        brick_of([3, 3, 3], short, BrickSample::U8),
    );
    assert!(matches!(
        &refused,
        Err((brick, BrickUploadError::WrongLength)) if brick.samples.len() == 26
    ));
    assert_eq!(graph.brick_uploads(), 0);
    assert!(graph.brick_texture(&ramp_key).is_none());
    assert_eq!(graph.uploads(), 0);

    let ramp = ramp_3x3x3();
    let uploaded = graph.upload_brick(
        &context,
        &adapter,
        ramp_key,
        brick_of([3, 3, 3], ramp.clone(), BrickSample::U8),
    );
    assert!(uploaded.is_ok());
    assert_eq!(graph.brick_uploads(), 1);
    assert_eq!(
        read_resident(
            &graph,
            &context,
            &ramp_key,
            [3, 3, 3],
            wgpu::TextureFormat::R8Uint
        ),
        ramp
    );

    let edge_key = key_at(1, [1, 0, 0]);
    let edge = vec![9u8, 10];
    let edge_uploaded = graph.upload_brick(
        &context,
        &adapter,
        edge_key,
        brick_of([2, 1, 1], edge.clone(), BrickSample::U8),
    );
    assert!(edge_uploaded.is_ok());
    assert_eq!(graph.brick_uploads(), 2);
    assert_eq!(
        read_resident(
            &graph,
            &context,
            &edge_key,
            [2, 1, 1],
            wgpu::TextureFormat::R8Uint
        ),
        edge
    );
    assert_ne!(graph.brick_extent(&edge_key), Some([128, 1, 1]));

    let exact = chain_at(VoiFunction::LinearExact);
    assert!(graph.set_voi(&context, viewport, &exact).is_ok());
    assert_eq!(graph.brick_uploads(), 2);
    assert_eq!(graph.uploads(), 0);
    assert_eq!(graph.lut_uploads(), 0);
    assert_eq!(
        read_resident(
            &graph,
            &context,
            &ramp_key,
            [3, 3, 3],
            wgpu::TextureFormat::R8Uint
        ),
        ramp
    );

    let mut replaced = ramp.clone();
    if let Some(first) = replaced.get_mut(0) {
        *first = 4;
    }
    let again = graph.upload_brick(
        &context,
        &adapter,
        ramp_key,
        brick_of([3, 3, 3], replaced.clone(), BrickSample::U8),
    );
    assert!(again.is_ok());
    assert_eq!(graph.brick_uploads(), 3);
    assert_eq!(
        read_resident(
            &graph,
            &context,
            &ramp_key,
            [3, 3, 3],
            wgpu::TextureFormat::R8Uint
        ),
        replaced
    );

    let recovered = ocelli_render::Recovered {
        caps: *context.caps(),
        graph: Some(ocelli_render::GraphResources {
            pipelines_restored: 1,
            cache_generation: 2,
        }),
    };
    graph.install_recovered(&context, &recovered);
    assert_eq!(graph.brick_uploads(), 3);
    assert!(graph.brick_texture(&ramp_key).is_none());
    assert!(graph.brick_texture(&edge_key).is_none());
    let restored = graph.upload_brick(
        &context,
        &adapter,
        ramp_key,
        brick_of([3, 3, 3], ramp.clone(), BrickSample::U8),
    );
    assert!(restored.is_ok());
    assert_eq!(graph.brick_uploads(), 4);
    assert_eq!(
        read_resident(
            &graph,
            &context,
            &ramp_key,
            [3, 3, 3],
            wgpu::TextureFormat::R8Uint
        ),
        ramp
    );
}
