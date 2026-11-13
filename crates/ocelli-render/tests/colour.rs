//! F-044 colour LUT fixtures.
//!
//! Expected values are PS3.3 C.7.6.3.1.5 and the two preset tables named by
//! the design plan. They were written before the upload existed. The palette
//! rows are also compared with `PaletteColorLut` through `ColorTransform`,
//! not re-derived from the texture builder.
//!
//! GPU rows are `#[ignore]`d. The byte tables, the index clamp and the
//! refusals need no device.

use ocelli_core::{Display, Stored};
use ocelli_pixel::{
    ColorTransform, DecodedLayout, DecodedPhotometric, LutChain, LutDescriptor, ModalityTransform,
    PaletteColorLut, PhotometricInterpretation, PixelDataEncoding, PixelError,
    PresentationLutEvidence, Rgb, SampleLayout, VoiFunction, VoiTransform,
};
use ocelli_render::{
    BlendMode, ColourLut, ColourLutError, FrameOutcome, IccProfilePresence, Pass, PresetId,
    RenderGraph, StackPass, StoredRoute, VoiParams,
};

/// Not widened. The same bound `tests/upload.rs` and `tests/voi_shader.rs` use.
const TOLERANCE: f32 = 0.001;

/// PS3.3 C.7.6.3.1.5. Not zero, so a missing offset is visible.
const FIRST_MAPPED: i32 = 10;

/// Entry 0, chosen so the three channels are not a permutation of each other.
const FIRST_ENTRY: [f32; 3] = [11.0, 22.0, 33.0];

/// Entry 1. Stored 11 must not come back as entry 0.
const SECOND_ENTRY: [f32; 3] = [44.0, 55.0, 66.0];

/// PS3.3 C.11.2.1.2 in `f32`, the carried LINEAR finding.
///
/// `c = 1024.5`, `w = 1.0003662109375`, so `c' = 1024` and
/// `w' = 0.0003662109375`. `fl(c' + w' / 2)` is `1024.000244140625`.
/// The body there is `297.5000305175781`, bits `0x4394c001`, outside the
/// declared range `[0, 255]`. The `f32` literals are the short spellings of
/// those three values.
const FINDING_INPUT: f32 = 1_024.000_2;
const FINDING_BITS: u32 = 0x4394_c001;

macro_rules! require_ok {
    ($value:expr) => {{
        let result = $value;
        assert!(result.is_ok(), "{result:?}");
        let Ok(value) = result else { return };
        value
    }};
}

fn texel(bytes: &[u8], index: usize) -> [u8; 4] {
    let start = index.saturating_mul(4);
    let Some(chunk) = bytes.get(start..start.saturating_add(4)) else {
        return [0, 0, 0, 0];
    };
    let Ok(array) = <[u8; 4]>::try_from(chunk) else {
        return [0, 0, 0, 0];
    };
    array
}

fn constant_display(value: f32) -> LutChain {
    let modality = ModalityTransform::new(None, Some(1.0), Some(0.0));
    let voi = VoiTransform::new(
        None,
        &[0.0],
        &[400.0],
        0,
        VoiFunction::LinearExact,
        value,
        value,
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

fn finding_chain() -> LutChain {
    let modality = ModalityTransform::new(None, Some(1.0), Some(0.0));
    let voi = VoiTransform::new(
        None,
        &[1024.5],
        &[1.000_366_2],
        0,
        VoiFunction::Linear,
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

fn palette_channels(
    count: u16,
    first: i32,
    samples: Vec<f32>,
) -> Result<PaletteColorLut, PixelError> {
    let red = LutDescriptor::new(count, first, 8, samples.clone())?;
    let green = LutDescriptor::new(count, first, 8, samples.clone())?;
    let blue = LutDescriptor::new(count, first, 8, samples)?;
    PaletteColorLut::new(red, green, blue)
}

fn palette_lut() -> Result<PaletteColorLut, PixelError> {
    let red = LutDescriptor::new(2, FIRST_MAPPED, 8, vec![FIRST_ENTRY[0], SECOND_ENTRY[0]])?;
    let green = LutDescriptor::new(2, FIRST_MAPPED, 8, vec![FIRST_ENTRY[1], SECOND_ENTRY[1]])?;
    let blue = LutDescriptor::new(2, FIRST_MAPPED, 8, vec![FIRST_ENTRY[2], SECOND_ENTRY[2]])?;
    PaletteColorLut::new(red, green, blue)
}

fn palette_transform(lut: PaletteColorLut) -> Result<ColorTransform, PixelError> {
    ColorTransform::resolve(
        SampleLayout::new(1, PhotometricInterpretation::PaletteColor, None)?,
        PixelDataEncoding::Native,
        DecodedPhotometric::Preserved,
        DecodedLayout::Preserved,
        Some(lut),
    )
}

fn assert_rgb_bits(actual: Rgb, expected: [f32; 3]) {
    assert_eq!(actual.r.to_bits(), expected[0].to_bits());
    assert_eq!(actual.g.to_bits(), expected[1].to_bits());
    assert_eq!(actual.b.to_bits(), expected[2].to_bits());
}

fn map_one(colour: &ColourLut, chain: &LutChain, stored: f32) -> Rgb {
    let mut destination = [Rgb::BLACK];
    let mapped = colour.map_preset_into(chain, &[Stored(stored)], &mut destination);
    assert!(mapped.is_ok(), "{mapped:?}");
    destination[0]
}

/// `grey` maps display `d` to RGB `(d, d, d)`. `stops` is the three texels
/// named by the plan, and every other index is black.
#[test]
fn grey_at_display_40_is_rgb_40_and_stops_are_the_three_texels() {
    let grey = require_ok!(ColourLut::preset(
        PresetId::Grey,
        PhotometricInterpretation::Monochrome2
    ));
    let grey_bytes = grey.texture_bytes();
    assert_eq!(texel(grey_bytes, 40), [40, 40, 40, 255]);
    assert_eq!(texel(grey_bytes, 0), [0, 0, 0, 255]);
    assert_eq!(texel(grey_bytes, 255), [255, 255, 255, 255]);
    for index in 0..256_u16 {
        let d = u8::try_from(index).unwrap_or(0);
        assert_eq!(
            texel(grey_bytes, usize::from(index)),
            [d, d, d, 255],
            "grey {index}"
        );
    }
    let at_40 = map_one(&grey, &constant_display(40.0), 0.0);
    assert_rgb_bits(at_40, [40.0, 40.0, 40.0]);

    let stops = require_ok!(ColourLut::preset(
        PresetId::Stops,
        PhotometricInterpretation::Monochrome2
    ));
    let stop_bytes = stops.texture_bytes();
    assert_eq!(texel(stop_bytes, 0), [0, 0, 0, 255]);
    assert_eq!(texel(stop_bytes, 128), [255, 0, 0, 255]);
    assert_eq!(texel(stop_bytes, 255), [255, 255, 255, 255]);
    for index in 0..256_usize {
        let expected = match index {
            0 => [0, 0, 0, 255],
            128 => [255, 0, 0, 255],
            255 => [255, 255, 255, 255],
            _ => [0, 0, 0, 255],
        };
        assert_eq!(texel(stop_bytes, index), expected, "stops {index}");
    }
    assert_rgb_bits(
        map_one(&stops, &constant_display(0.0), 7.0),
        [0.0, 0.0, 0.0],
    );
    assert_rgb_bits(
        map_one(&stops, &constant_display(128.0), 7.0),
        [255.0, 0.0, 0.0],
    );
    assert_rgb_bits(
        map_one(&stops, &constant_display(255.0), 7.0),
        [255.0, 255.0, 255.0],
    );
    assert_rgb_bits(
        map_one(&stops, &constant_display(127.0), 7.0),
        [0.0, 0.0, 0.0],
    );
    assert_rgb_bits(
        map_one(&stops, &constant_display(129.0), 7.0),
        [0.0, 0.0, 0.0],
    );
}

/// PS3.3 C.7.6.3.1.5. Stored values below the first mapped input clamp to
/// the first entry. Stored 10 is that first mapped input.
#[test]
fn palette_first_mapped_input_10_clamps_stored_9_and_maps_stored_10() {
    let lut = require_ok!(palette_lut());
    let transform = require_ok!(palette_transform(lut.clone()));
    let colour = require_ok!(ColourLut::palette(
        lut,
        PhotometricInterpretation::PaletteColor,
        IccProfilePresence::Absent
    ));
    let bytes = colour.texture_bytes();

    let mut from_chain = [Rgb::BLACK; 3];
    assert_eq!(
        transform.map_into(
            &[Stored(9.0), Stored(10.0), Stored(11.0)],
            &mut from_chain,
            3,
        ),
        Ok(())
    );
    assert_rgb_bits(from_chain[0], FIRST_ENTRY);
    assert_rgb_bits(from_chain[1], FIRST_ENTRY);
    assert_rgb_bits(from_chain[2], SECOND_ENTRY);

    // Texel 0 is the first mapped input. Stored 9 is not a separate texel.
    assert_eq!(bytes.len(), 8);
    assert_eq!(texel(bytes, 0), [11, 22, 33, 255], "stored 10 is texel 0");
    assert_eq!(texel(bytes, 1), [44, 55, 66, 255], "stored 11 is texel 1");
    assert_eq!(
        from_chain[0].r.to_bits(),
        f32::from(texel(bytes, 0)[0]).to_bits()
    );
    assert_eq!(
        from_chain[2].b.to_bits(),
        f32::from(texel(bytes, 1)[2]).to_bits()
    );
}

/// A mapped range that does not contain 0 stays that wide.
///
/// PS3.3 C.7.6.3.1.5. First mapped input 8192 and 256 entries end at 8447.
/// Extending the table down to stored 0 makes 8448 texels, which a device
/// whose limit is 8192 then refuses. Entry 0 is 7 and entry 255 is 9, so a
/// texel taken from the padded prefix cannot pass for the last entry.
#[test]
fn a_palette_from_8192_stays_256_texels_wide() {
    let mut samples = vec![7.0_f32];
    samples.resize(255, 0.0);
    samples.push(9.0);
    let lut = require_ok!(palette_channels(256, 8192, samples));
    let transform = require_ok!(palette_transform(lut.clone()));
    let colour = require_ok!(ColourLut::palette(
        lut,
        PhotometricInterpretation::PaletteColor,
        IccProfilePresence::Absent
    ));
    let bytes = colour.texture_bytes();
    assert_eq!(bytes.len(), 256 * 4);

    let mut from_chain = [Rgb::BLACK; 3];
    assert_eq!(
        transform.map_into(
            &[Stored(8191.0), Stored(8192.0), Stored(8447.0)],
            &mut from_chain,
            3,
        ),
        Ok(())
    );
    assert_eq!(texel(bytes, 0), [7, 7, 7, 255]);
    assert_eq!(texel(bytes, 255), [9, 9, 9, 255]);
    assert_eq!(
        from_chain[0].r.to_bits(),
        f32::from(texel(bytes, 0)[0]).to_bits()
    );
    assert_eq!(
        from_chain[2].r.to_bits(),
        f32::from(texel(bytes, 255)[0]).to_bits()
    );
}

/// The same bound on the negative side. First mapped input -4 and two
/// entries end at -3. Growing the table up to stored 0 would add three texels.
#[test]
fn a_palette_below_zero_does_not_grow_up_to_stored_zero() {
    let lut = require_ok!(palette_channels(2, -4, vec![1.0, 2.0]));
    let colour = require_ok!(ColourLut::palette(
        lut,
        PhotometricInterpretation::PaletteColor,
        IccProfilePresence::Absent
    ));
    let bytes = colour.texture_bytes();
    assert_eq!(bytes.len(), 8);
    assert_eq!(texel(bytes, 0), [1, 1, 1, 255]);
    assert_eq!(texel(bytes, 1), [2, 2, 2, 255]);
}

/// The table address clamps. `LutChain` does not.
///
/// Display 254 is the neighbour of the last entry, so an endpoint hit is not
/// "every large value becomes 254". Display -4 is below the table and the
/// chain still reports -4. The LINEAR finding stays outside `[0, 255]` and
/// addresses texel 255.
#[test]
fn an_out_of_range_display_index_uses_the_endpoint_and_the_chain_stays_unclamped() {
    let grey = require_ok!(ColourLut::preset(
        PresetId::Grey,
        PhotometricInterpretation::Monochrome2
    ));
    assert_rgb_bits(
        map_one(&grey, &constant_display(254.0), 0.0),
        [254.0, 254.0, 254.0],
    );
    assert_rgb_bits(
        map_one(&grey, &constant_display(255.0), 0.0),
        [255.0, 255.0, 255.0],
    );
    assert_rgb_bits(
        map_one(&grey, &constant_display(300.0), 0.0),
        [255.0, 255.0, 255.0],
    );
    let below = constant_display(-4.0);
    assert_eq!(below.apply(Stored(0.0)).0.to_bits(), (-4.0_f32).to_bits());
    assert_rgb_bits(map_one(&grey, &below, 0.0), [0.0, 0.0, 0.0]);

    let finding = finding_chain();
    let mut held = [Display(0.0)];
    assert_eq!(
        finding.map_into(&[Stored(FINDING_INPUT)], &mut held),
        Ok(())
    );
    assert_eq!(held[0].0.to_bits(), FINDING_BITS);
    assert!(held[0].0 > 255.0);
    assert_rgb_bits(
        map_one(&grey, &finding, FINDING_INPUT),
        [255.0, 255.0, 255.0],
    );
    assert_eq!(
        finding.apply(Stored(FINDING_INPUT)).0.to_bits(),
        FINDING_BITS,
        "indexing the table clamped the chain value"
    );
}

#[test]
fn icc_and_a_palette_on_monochrome_and_a_preset_on_palette_are_unavailable() {
    let lut = require_ok!(palette_lut());
    assert_eq!(
        ColourLut::icc(PhotometricInterpretation::Monochrome2),
        Err(ColourLutError::Unavailable)
    );
    assert_eq!(
        ColourLut::icc(PhotometricInterpretation::Rgb),
        Err(ColourLutError::Unavailable)
    );
    assert_eq!(
        ColourLut::palette(
            lut.clone(),
            PhotometricInterpretation::Monochrome2,
            IccProfilePresence::Absent,
        ),
        Err(ColourLutError::Unavailable)
    );
    assert_eq!(
        ColourLut::palette(
            lut,
            PhotometricInterpretation::Monochrome1,
            IccProfilePresence::Absent,
        ),
        Err(ColourLutError::Unavailable)
    );
    assert_eq!(
        ColourLut::preset(PresetId::Grey, PhotometricInterpretation::PaletteColor),
        Err(ColourLutError::Unavailable)
    );
    assert_eq!(
        ColourLut::preset(PresetId::Stops, PhotometricInterpretation::PaletteColor),
        Err(ColourLutError::Unavailable)
    );
}

/// D3. A GPU route given a profiled frame reports unavailable rather than
/// baking device RGB. The palette bake refuses an accepted profile before
/// any texel is built. A profile the subset refused is
/// [`IccProfilePresence::Absent`], and the same palette bakes, so the
/// refusal is the profile's and not the palette's.
#[test]
fn a_profiled_palette_is_not_baked_as_device_rgb() {
    let lut = require_ok!(palette_lut());
    assert_eq!(
        ColourLut::palette(
            lut.clone(),
            PhotometricInterpretation::PaletteColor,
            IccProfilePresence::Accepted
        ),
        Err(ColourLutError::Unavailable)
    );
    assert!(
        ColourLut::palette(
            lut,
            PhotometricInterpretation::PaletteColor,
            IccProfilePresence::Absent
        )
        .is_ok()
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

fn params(chain: &LutChain) -> VoiParams {
    match VoiParams::from_chain(chain) {
        Ok(voi) => voi,
        Err(_) => unreachable!("chain must express"),
    }
}

fn target(device: &wgpu::Device, width: u32) -> (wgpu::TextureView, wgpu::Texture) {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("colour lut readback"),
        size: wgpu::Extent3d {
            width,
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
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    (view, texture)
}

fn read_rgb(
    context: &ocelli_render::GpuContext,
    texture: &wgpu::Texture,
    width: u32,
) -> Vec<[f32; 3]> {
    let device = context.device();
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("colour lut readback"),
        size: 256,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("colour lut readback"),
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
    let mut colours = Vec::new();
    let count = usize::try_from(width).unwrap_or(0);
    for index in 0..count {
        let base = index.saturating_mul(16);
        let mut channels = [0.0_f32; 3];
        for (channel, slot) in channels.iter_mut().enumerate() {
            let start = base.saturating_add(channel.saturating_mul(4));
            let Some(end) = start.checked_add(4) else {
                continue;
            };
            let Some(chunk) = bytes.get(start..end) else {
                continue;
            };
            let Ok(array) = <[u8; 4]>::try_from(chunk) else {
                continue;
            };
            *slot = f32::from_le_bytes(array) * 255.0;
        }
        colours.push(channels);
    }
    colours
}

fn assert_channel_close(actual: f32, expected: f32) {
    let difference = (actual - expected).abs();
    assert!(
        difference < TOLERANCE,
        "was {actual}, wanted {expected}, difference {difference}"
    );
}

fn assert_colour_close(actual: [f32; 3], expected: [f32; 3]) {
    for index in 0..3_usize {
        let got = actual.get(index).copied().unwrap_or(f32::NAN);
        let want = expected.get(index).copied().unwrap_or(f32::NAN);
        assert_channel_close(got, want);
    }
}

fn draw(context: &ocelli_render::GpuContext, graph: &mut RenderGraph) {
    assert_eq!(
        graph.tick(context),
        FrameOutcome::Submitted {
            viewports: 1,
            passes: 1
        }
    );
}

/// Window changes write thirty-two bytes. They do not upload the colour table
/// and they do not upload the stored image again.
#[test]
#[ignore = "needs a real GPU adapter, which the CI floor does not have (D-04)"]
fn grey_and_stops_read_back_and_a_window_change_does_not_upload() {
    let Some((adapter, mut context)) = session() else {
        println!("this machine resolved tier C, so there is no device");
        return;
    };
    let tier = context.caps().tier;
    assert_eq!(tier, ocelli_render::Tier::A, "tier A colour readback only");
    let built = RenderGraph::new(&mut context, 1, wgpu::TextureFormat::Rgba32Float);
    assert!(built.is_ok());
    let Ok(mut graph) = built else { return };
    let initial = constant_display(40.0);
    let (view, texture) = target(context.device(), 1);
    let registered = graph.register_stored(
        &context,
        &adapter,
        view,
        1,
        1,
        Pass::Stack(StackPass {
            voi: params(&initial),
            blend: BlendMode::Composite,
        }),
        StoredRoute::Tabled {
            first: 0,
            count: 65_536,
        },
    );
    assert!(registered.is_ok());
    let Ok(viewport) = registered else { return };
    assert_eq!(graph.uploads(), 0);
    assert_eq!(graph.lut_uploads(), 0);
    assert!(
        graph
            .upload_stored(&context, &adapter, viewport, 1, 1, &[0.0])
            .is_ok()
    );
    assert_eq!(graph.uploads(), 1);
    let grey = require_ok!(ColourLut::preset(
        PresetId::Grey,
        PhotometricInterpretation::Monochrome2
    ));
    assert!(
        graph
            .select_colour_lut(&context, &adapter, viewport, &grey)
            .is_ok()
    );
    assert_eq!(graph.lut_uploads(), 1);
    assert_eq!(graph.uploads(), 1);
    let shifted = constant_display(0.0);
    assert!(graph.set_voi(&context, viewport, &shifted).is_ok());
    assert_eq!(graph.lut_uploads(), 1);
    assert_eq!(graph.uploads(), 1);
    assert!(graph.set_voi(&context, viewport, &initial).is_ok());
    assert_eq!(graph.lut_uploads(), 1);
    assert_eq!(graph.uploads(), 1);
    draw(&context, &mut graph);
    let row = read_rgb(&context, &texture, 1);
    let got = row.first().copied().unwrap_or([f32::NAN; 3]);
    assert_colour_close(got, [40.0, 40.0, 40.0]);
    assert_eq!(graph.lut_uploads(), 1);
    assert_eq!(graph.uploads(), 1);

    let stops = require_ok!(ColourLut::preset(
        PresetId::Stops,
        PhotometricInterpretation::Monochrome2
    ));
    assert!(
        graph
            .select_colour_lut(&context, &adapter, viewport, &stops)
            .is_ok()
    );
    assert_eq!(graph.lut_uploads(), 2);
    assert_eq!(graph.uploads(), 1);
    assert!(
        graph
            .set_voi(&context, viewport, &constant_display(128.0))
            .is_ok()
    );
    assert_eq!(graph.lut_uploads(), 2);
    assert_eq!(graph.uploads(), 1);
    draw(&context, &mut graph);
    let row = read_rgb(&context, &texture, 1);
    assert_colour_close(
        row.first().copied().unwrap_or([f32::NAN; 3]),
        [255.0, 0.0, 0.0],
    );
    assert!(
        graph
            .set_voi(&context, viewport, &constant_display(0.0))
            .is_ok()
    );
    assert_eq!(graph.lut_uploads(), 2);
    draw(&context, &mut graph);
    let row = read_rgb(&context, &texture, 1);
    assert_colour_close(
        row.first().copied().unwrap_or([f32::NAN; 3]),
        [0.0, 0.0, 0.0],
    );
    assert!(
        graph
            .set_voi(&context, viewport, &constant_display(255.0))
            .is_ok()
    );
    assert_eq!(graph.lut_uploads(), 2);
    assert_eq!(graph.uploads(), 1);
    draw(&context, &mut graph);
    let row = read_rgb(&context, &texture, 1);
    assert_colour_close(
        row.first().copied().unwrap_or([f32::NAN; 3]),
        [255.0, 255.0, 255.0],
    );

    // The finding, with no colour table. The graph must still emit it.
    assert!(graph.remove(viewport).is_ok());
    let finding = finding_chain();
    let (view, texture) = target(context.device(), 1);
    let registered = graph.register_stored(
        &context,
        &adapter,
        view,
        1,
        1,
        Pass::Stack(StackPass {
            voi: params(&finding),
            blend: BlendMode::Composite,
        }),
        StoredRoute::Tabled {
            first: 0,
            count: 65_536,
        },
    );
    assert!(registered.is_ok());
    let Ok(viewport) = registered else { return };
    let uploads_before = graph.uploads();
    let lut_before = graph.lut_uploads();
    assert!(
        graph
            .upload_stored(&context, &adapter, viewport, 1, 1, &[FINDING_INPUT])
            .is_ok()
    );
    draw(&context, &mut graph);
    let row = read_rgb(&context, &texture, 1);
    let red = row.first().map(|channels| channels[0]).unwrap_or(f32::NAN);
    assert!(
        red > 255.0,
        "greyscale path clamped the LINEAR finding to {red}"
    );
    assert_channel_close(red, 297.500_03);
    assert_eq!(graph.lut_uploads(), lut_before);
    assert_eq!(graph.uploads(), uploads_before + 1);
}

/// Stored 9 and stored 10 read back the first palette entry. The shader does
/// not run `lut_chain` for this binding, so a window change cannot recolour it.
#[test]
#[ignore = "needs a real GPU adapter, which the CI floor does not have (D-04)"]
fn palette_stored_samples_read_back_the_descriptor_entries() {
    let Some((adapter, mut context)) = session() else {
        println!("this machine resolved tier C, so there is no device");
        return;
    };
    assert_eq!(context.caps().tier, ocelli_render::Tier::A);
    let built = RenderGraph::new(&mut context, 1, wgpu::TextureFormat::Rgba32Float);
    assert!(built.is_ok());
    let Ok(mut graph) = built else { return };
    let chain = constant_display(40.0);
    let (view, texture) = target(context.device(), 3);
    let registered = graph.register_stored(
        &context,
        &adapter,
        view,
        3,
        1,
        Pass::Stack(StackPass {
            voi: params(&chain),
            blend: BlendMode::Composite,
        }),
        StoredRoute::Tabled {
            first: 0,
            count: 65_536,
        },
    );
    assert!(registered.is_ok());
    let Ok(viewport) = registered else { return };
    assert!(
        graph
            .upload_stored(&context, &adapter, viewport, 3, 1, &[9.0, 10.0, 11.0])
            .is_ok()
    );
    let uploads = graph.uploads();
    let lut = require_ok!(palette_lut());
    let colour = require_ok!(ColourLut::palette(
        lut,
        PhotometricInterpretation::PaletteColor,
        IccProfilePresence::Absent
    ));
    assert!(
        graph
            .select_colour_lut(&context, &adapter, viewport, &colour)
            .is_ok()
    );
    assert_eq!(graph.lut_uploads(), 1);
    assert_eq!(graph.uploads(), uploads);
    assert!(
        graph
            .set_voi(&context, viewport, &constant_display(200.0))
            .is_ok()
    );
    assert_eq!(graph.lut_uploads(), 1);
    assert_eq!(graph.uploads(), uploads);
    draw(&context, &mut graph);
    let row = read_rgb(&context, &texture, 3);
    assert_colour_close(row.first().copied().unwrap_or([f32::NAN; 3]), FIRST_ENTRY);
    assert_colour_close(row.get(1).copied().unwrap_or([f32::NAN; 3]), FIRST_ENTRY);
    assert_colour_close(row.get(2).copied().unwrap_or([f32::NAN; 3]), SECOND_ENTRY);
}

/// F-239 item 11 through the GPU bake. Tiers A and B read a palette texel the
/// CPU baked, so the bake must hold the scaled entry tier C draws.
///
/// ```text
/// descriptors 5\0\16 for all three channels
/// red   0 255 256 32768 65535 -> >> 8 -> 0 0 1 128 255
/// green 65535 32768 256 255 0 -> >> 8 -> 255 128 1 0 0, blue the same
/// texel i is (red i, green i, blue i, 255)
/// ```
///
/// Before F-239 the bake refused this table, because 256 and above is not a
/// display byte.
#[test]
fn the_gpu_bake_holds_sixteen_bit_entries_after_their_display_scale() {
    let red = require_ok!(LutDescriptor::new(
        5,
        0,
        16,
        vec![0.0, 255.0, 256.0, 32_768.0, 65_535.0]
    ));
    let falling = vec![65_535.0, 32_768.0, 256.0, 255.0, 0.0];
    let green = require_ok!(LutDescriptor::new(5, 0, 16, falling.clone()));
    let blue = require_ok!(LutDescriptor::new(5, 0, 16, falling));
    let colour = require_ok!(ColourLut::palette(
        require_ok!(PaletteColorLut::new(red, green, blue)),
        PhotometricInterpretation::PaletteColor,
        IccProfilePresence::Absent,
    ));
    assert_eq!(
        colour.texture_bytes(),
        [
            0, 255, 255, 255, 0, 128, 128, 255, 1, 1, 1, 255, 128, 0, 0, 255, 255, 0, 0, 255,
        ]
    );
}

/// The R1 fallback through the GPU bake. All three channels are declared
/// 16-bit and every entry fits in 8 bits, so the palette is read as 8-bit and
/// its texels are the entries themselves.
#[test]
fn the_gpu_bake_reads_a_narrow_sixteen_bit_table_as_eight_bit() {
    let entries = vec![10.0, 200.0];
    let colour = require_ok!(ColourLut::palette(
        require_ok!(PaletteColorLut::new(
            require_ok!(LutDescriptor::new(2, 0, 16, entries.clone())),
            require_ok!(LutDescriptor::new(2, 0, 16, entries.clone())),
            require_ok!(LutDescriptor::new(2, 0, 16, entries)),
        )),
        PhotometricInterpretation::PaletteColor,
        IccProfilePresence::Absent,
    ));
    assert_eq!(
        colour.texture_bytes(),
        [10, 10, 10, 255, 200, 200, 200, 255]
    );
}
