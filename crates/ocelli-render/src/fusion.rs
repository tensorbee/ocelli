//! Two volumes on one plane.
//!
//! Volume B is in front of volume A. The composite is
//! `pet * opacity_pet + ct * opacity_ct * (1 - opacity_pet)`.
//! An absent sample adds no colour. Tier C returns before any sample.
//! The shader calls `lut_chain` and `table_index`. It does not own a second copy.

use bytemuck::{Pod, Zeroable};
use ocelli_core::{Display, Stored};
use ocelli_pixel::{DIRECTION_COSINE_TOLERANCE, LutChain, Rgb};
use ocelli_volume::{FusionPlane, PlaneSample, Volume, VoxelKind, world_on_plane};
use wgpu::util::DeviceExt;

use crate::caps::Tier;
use crate::colour::{byte_from_channel, display_index};
use crate::gpu::GpuContext;
use crate::graph::{Dirty, TABLE_INDEX_WGSL, Upload, aligned_row_bytes};
use crate::probe::ResolvedAdapter;
use crate::voi::{MODALITY_FOLD, ModalityRoute, VOI_WGSL, VoiParams, VoiParamsError};

const FUSION_WGSL: &str = include_str!("../shaders/fusion.wgsl");
const TABLE_LAST: u32 = 255;

/// One fusion request.
///
/// `volume_a` is the back volume and `volume_b` is the front volume.
/// A field for a patient weight is not part of this request.
pub struct FusionRequest<'a> {
    /// Back volume.
    pub volume_a: &'a Volume,
    /// Frame of reference of `volume_a`.
    pub frame_a: &'a ocelli_volume::FrameOfReferenceUid,
    /// Front volume.
    pub volume_b: &'a Volume,
    /// Frame of reference of `volume_b`.
    pub frame_b: &'a ocelli_volume::FrameOfReferenceUid,
    /// Window chain for `volume_a`.
    pub chain_a: &'a LutChain,
    /// Window chain for `volume_b`.
    pub chain_b: &'a LutChain,
    /// Opacity of `volume_a`, from 0 to 1 inclusive.
    pub opacity_a: f32,
    /// Opacity of `volume_b`, from 0 to 1 inclusive.
    pub opacity_b: f32,
    /// Optional 256-entry table for `volume_a`, applied after its window.
    pub colormap_a: Option<&'a [Rgb]>,
    /// Optional 256-entry table for `volume_b`, applied after its window.
    pub colormap_b: Option<&'a [Rgb]>,
    /// Shared plane.
    pub plane: &'a FusionPlane,
}

/// Why a fusion frame was not published.
#[derive(Debug, thiserror::Error, PartialEq)]
pub enum FusionError {
    /// Tier C has no fusion image.
    #[error("fusion is unavailable on tier C, which holds no device")]
    TierCUnavailable,
    /// The frame-of-reference strings differ, or either one is empty.
    #[error("the two volumes do not share a frame of reference")]
    FrameMismatch,
    /// An opacity is outside 0 to 1, or is not finite.
    #[error("an opacity is outside 0 to 1")]
    Opacity,
    /// A chain has no section 18.4 uniform.
    #[error(transparent)]
    Chain(#[from] VoiParamsError),
    /// A colormap was supplied and its length is not 256.
    #[error("a fusion colormap must hold 256 entries")]
    ColormapLength,
    /// The plane or a volume spacing cannot be sampled.
    #[error("the fusion plane cannot be sampled")]
    Plane,
    /// The device cannot take the frame.
    #[error("fusion is unavailable")]
    Unavailable,
}

/// Front-over-back composite of two present colours.
///
/// `out = pet * opacity_pet + ct * opacity_ct * (1 - opacity_pet)`.
pub fn composite(pet: Rgb, ct: Rgb, opacity_pet: f32, opacity_ct: f32) -> Rgb {
    Rgb {
        r: pet.r * opacity_pet + ct.r * opacity_ct * (1.0 - opacity_pet),
        g: pet.g * opacity_pet + ct.g * opacity_ct * (1.0 - opacity_pet),
        b: pet.b * opacity_pet + ct.b * opacity_ct * (1.0 - opacity_pet),
    }
}

/// Composite when either sample may be absent.
///
/// Absence is not stored zero. The present volume is drawn on its own.
/// Both absent yields [`None`], which the shader discards.
pub fn composite_sample(
    pet: Option<Rgb>,
    ct: Option<Rgb>,
    opacity_pet: f32,
    opacity_ct: f32,
) -> Option<Rgb> {
    match (pet, ct) {
        (None, None) => None,
        (Some(pet), None) => Some(scale(pet, opacity_pet)),
        (None, Some(ct)) => Some(scale(ct, opacity_ct)),
        (Some(pet), Some(ct)) => Some(composite(pet, ct, opacity_pet, opacity_ct)),
    }
}

/// Map one stored sample through the chain, then the existing display index.
///
/// An omitted table repeats the display value on every channel. A supplied
/// table is read at the display index, not at the stored sample.
///
/// # Errors
///
/// [`FusionError::Chain`] when the chain has no uniform.
/// [`FusionError::ColormapLength`] when a table is not 256 entries.
/// [`FusionError::Unavailable`] when the chain cannot write the display slot.
pub fn colour_after_voi(
    chain: &LutChain,
    stored: f32,
    table: Option<&[Rgb]>,
) -> Result<Rgb, FusionError> {
    let _uniform = VoiParams::from_chain(chain)?;
    if let Some(entries) = table
        && entries.len() != 256
    {
        return Err(FusionError::ColormapLength);
    }
    let mut destination = [Display(0.0)];
    chain
        .map_into(&[Stored(stored)], &mut destination)
        .map_err(|_| FusionError::Unavailable)?;
    let display = match destination.first() {
        Some(display) => display.0,
        None => return Err(FusionError::Unavailable),
    };
    let Some(entries) = table else {
        return Ok(Rgb {
            r: display,
            g: display,
            b: display,
        });
    };
    let index = display_index(display, TABLE_LAST);
    let at = usize::try_from(index).map_err(|_| FusionError::Unavailable)?;
    match entries.get(at) {
        Some(rgb) => Ok(*rgb),
        None => Err(FusionError::Unavailable),
    }
}

/// `VOI_WGSL`, the shared `table_index`, then the fusion entry points.
pub fn fusion_module() -> String {
    format!("{VOI_WGSL}{TABLE_INDEX_WGSL}{FUSION_WGSL}")
}

/// Floor stand-in for a published frame and its submit count.
///
/// Refusals return before the encode closure. A settled request does not
/// call it and does not allocate.
pub struct FusionLedger {
    published: Vec<u8>,
    staging: Vec<u8>,
    allocations: u32,
    submits: u64,
    dirty: Dirty,
    bound: Option<Bound>,
}

struct Bound {
    stamp: FusionStamp,
    uid: String,
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) struct FusionStamp {
    opacity_a: u32,
    opacity_b: u32,
    voi_a: VoiParams,
    voi_b: VoiParams,
    table_a: u64,
    table_b: u64,
    plane: FusionPlane,
    samples_a: u64,
    samples_b: u64,
    geometry_a: u64,
    geometry_b: u64,
}

pub(crate) enum Decision {
    Refuse(FusionError),
    Idle,
    Encode(Box<FusionStamp>),
}

impl Default for FusionLedger {
    fn default() -> Self {
        Self::new()
    }
}

impl FusionLedger {
    /// An empty frame, fully dirty, with no binding.
    pub fn new() -> Self {
        Self {
            published: Vec::new(),
            staging: Vec::new(),
            allocations: 0,
            submits: 0,
            dirty: Dirty::full(),
            bound: None,
        }
    }

    /// Size the published bytes when the plane is bound.
    ///
    /// A later present reuses this buffer.
    pub fn bind_extent(&mut self, pixels: u32) {
        let count = usize::try_from(pixels).unwrap_or(0).saturating_mul(4);
        if count == 0 || self.published.len() >= count {
            return;
        }
        self.published.resize(count, 0);
        self.staging.resize(count, 0);
        self.allocations = self.allocations.saturating_add(1);
    }

    /// Write bytes into the published buffer without counting a submit.
    pub fn seed_published(&mut self, bytes: &[u8]) -> bool {
        let Some(slot) = self.published.get_mut(..bytes.len()) else {
            return false;
        };
        slot.copy_from_slice(bytes);
        true
    }

    /// Mark the existing data flag. This is not a new flag.
    pub fn mark_data(&mut self) {
        self.dirty.data = true;
    }

    /// The last accepted frame-of-reference string.
    pub fn bound_uid(&self) -> Option<&str> {
        self.bound.as_ref().map(|bound| bound.uid.as_str())
    }

    /// Bytes of the last accepted encode.
    pub fn published(&self) -> &[u8] {
        &self.published
    }

    /// How many encodes were accepted.
    pub fn submits(&self) -> u64 {
        self.submits
    }

    /// How many times the published buffer grew.
    pub fn allocations(&self) -> u32 {
        self.allocations
    }

    /// Admit `request`, or leave the published bytes where they are.
    ///
    /// The closure runs only when a new frame is committed.
    ///
    /// # Errors
    ///
    /// [`FusionError`] when the request is refused or the closure fails.
    pub fn present(
        &mut self,
        tier: Tier,
        request: &FusionRequest<'_>,
        encode: impl FnOnce(&mut [u8]) -> Result<(), FusionError>,
    ) -> Result<(), FusionError> {
        let decision = decide(
            tier,
            request,
            self.dirty,
            self.bound.as_ref().map(|bound| &bound.stamp),
            self.bound.as_ref().map(|bound| bound.uid.as_str()),
        );
        match decision {
            Decision::Refuse(err) => Err(err),
            Decision::Idle => Ok(()),
            Decision::Encode(stamp) => {
                if self.staging.is_empty() || self.published.len() != self.staging.len() {
                    return Err(FusionError::Plane);
                }
                encode(&mut self.staging)?;
                self.published.copy_from_slice(&self.staging);
                self.submits = self.submits.saturating_add(1);
                self.bound = Some(Bound {
                    stamp: *stamp,
                    uid: request.frame_a.0.clone(),
                });
                self.dirty.clear();
                Ok(())
            }
        }
    }
}

pub(crate) fn decide(
    tier: Tier,
    request: &FusionRequest<'_>,
    dirty: Dirty,
    bound_stamp: Option<&FusionStamp>,
    bound_uid: Option<&str>,
) -> Decision {
    if tier == Tier::Cpu {
        return Decision::Refuse(FusionError::TierCUnavailable);
    }
    if request.frame_a.0.is_empty() || request.frame_b.0.is_empty() {
        return Decision::Refuse(FusionError::FrameMismatch);
    }
    if request.frame_a.0 != request.frame_b.0 {
        return Decision::Refuse(FusionError::FrameMismatch);
    }
    if !opacity_ok(request.opacity_a) || !opacity_ok(request.opacity_b) {
        return Decision::Refuse(FusionError::Opacity);
    }
    let voi_a = match VoiParams::from_chain(request.chain_a) {
        Ok(voi) => voi,
        Err(err) => return Decision::Refuse(FusionError::Chain(err)),
    };
    let voi_b = match VoiParams::from_chain(request.chain_b) {
        Ok(voi) => voi,
        Err(err) => return Decision::Refuse(FusionError::Chain(err)),
    };
    if !table_len_ok(request.colormap_a) || !table_len_ok(request.colormap_b) {
        return Decision::Refuse(FusionError::ColormapLength);
    }
    if !plane_ok(request.plane) || !spacing_ok(request.volume_a) || !spacing_ok(request.volume_b) {
        return Decision::Refuse(FusionError::Plane);
    }
    let stamp = FusionStamp {
        opacity_a: request.opacity_a.to_bits(),
        opacity_b: request.opacity_b.to_bits(),
        voi_a,
        voi_b,
        table_a: table_hash(request.colormap_a),
        table_b: table_hash(request.colormap_b),
        plane: *request.plane,
        samples_a: sample_hash(request.volume_a),
        samples_b: sample_hash(request.volume_b),
        geometry_a: geometry_hash(request.volume_a),
        geometry_b: geometry_hash(request.volume_b),
    };
    if !dirty.any()
        && let (Some(held), Some(uid)) = (bound_stamp, bound_uid)
        && held == &stamp
        && uid == request.frame_a.0
    {
        return Decision::Idle;
    }
    Decision::Encode(Box::new(stamp))
}

fn scale(colour: Rgb, opacity: f32) -> Rgb {
    Rgb {
        r: colour.r * opacity,
        g: colour.g * opacity,
        b: colour.b * opacity,
    }
}

fn opacity_ok(value: f32) -> bool {
    (0.0..=1.0).contains(&value)
}

fn table_len_ok(table: Option<&[Rgb]>) -> bool {
    match table {
        None => true,
        Some(entries) => entries.len() == 256,
    }
}

fn plane_fits_limit(width: u32, height: u32, max_texture_dimension_2d: u32) -> bool {
    width <= max_texture_dimension_2d && height <= max_texture_dimension_2d
}

fn plane_ok(plane: &FusionPlane) -> bool {
    finite_point(plane.origin)
        && unit_dir(plane.row_dir)
        && unit_dir(plane.column_dir)
        && positive(plane.column_mm)
        && positive(plane.row_mm)
        && plane.width > 0
        && plane.height > 0
}

fn finite_point(point: ocelli_core::Pt<ocelli_core::World>) -> bool {
    point.x.is_finite() && point.y.is_finite() && point.z.is_finite()
}

fn unit_dir(dir: (f64, f64, f64)) -> bool {
    if !dir.0.is_finite() || !dir.1.is_finite() || !dir.2.is_finite() {
        return false;
    }
    let length_sq = dir.0 * dir.0 + dir.1 * dir.1 + dir.2 * dir.2;
    if !length_sq.is_finite() {
        return false;
    }
    // The same bound `ImageOrientationPatient::new` uses before it normalises.
    // A stored column can be one ulp off length 1. This check does not
    // renormalise, so a direction of length 2 stays refused.
    (length_sq.sqrt() - 1.0).abs() <= DIRECTION_COSINE_TOLERANCE
}

fn positive(value: f64) -> bool {
    value.is_finite() && value > 0.0
}

fn spacing_ok(volume: &Volume) -> bool {
    positive(volume.spacing[0]) && positive(volume.spacing[1]) && positive(volume.spacing[2])
}

fn table_hash(table: Option<&[Rgb]>) -> u64 {
    let Some(entries) = table else {
        return 0;
    };
    let mut hash = 0x8422_2325_cbf2_9ce4_u64;
    for rgb in entries {
        hash = mix(hash, u64::from(rgb.r.to_bits()));
        hash = mix(hash, u64::from(rgb.g.to_bits()));
        hash = mix(hash, u64::from(rgb.b.to_bits()));
    }
    hash
}

/// Bytes plus each slice's presence bit.
///
/// A zero slice that arrives does not change the bytes. It is still a
/// different sample, because absence is not stored zero.
fn sample_hash(volume: &Volume) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    let mut z = 0_u32;
    while z < volume.dims[2] {
        hash = mix(hash, u64::from(volume.present.test(z)));
        match volume.data.slice(z) {
            Some(bytes) => {
                for byte in bytes {
                    hash = mix(hash, u64::from(*byte));
                }
            }
            None => hash = mix(hash, 0xff),
        }
        z = z.saturating_add(1);
    }
    hash
}

/// Spacing, origin, direction, dimensions and voxel kind.
///
/// `sample_at` reads the dimensions and the kind. Neither is visible in
/// the byte hash.
fn geometry_hash(volume: &Volume) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    hash = mix(hash, u64::from(volume.dims[0]));
    hash = mix(hash, u64::from(volume.dims[1]));
    hash = mix(hash, u64::from(volume.dims[2]));
    hash = mix(hash, kind_code(volume.voxel));
    for value in volume.spacing {
        hash = mix(hash, value.to_bits());
    }
    hash = mix(hash, volume.origin.x.to_bits());
    hash = mix(hash, volume.origin.y.to_bits());
    hash = mix(hash, volume.origin.z.to_bits());
    let mut column = 0_usize;
    while column < 3 {
        let axis = volume.direction.col(column);
        hash = mix(hash, axis.x.to_bits());
        hash = mix(hash, axis.y.to_bits());
        hash = mix(hash, axis.z.to_bits());
        column = column.saturating_add(1);
    }
    hash
}

fn kind_code(kind: VoxelKind) -> u64 {
    match kind {
        VoxelKind::U8 => 0,
        VoxelKind::I16 => 1,
        VoxelKind::U16 => 2,
        VoxelKind::F32 => 3,
    }
}

fn mix(hash: u64, value: u64) -> u64 {
    hash.wrapping_mul(0x0000_0100_0000_01b3) ^ value
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct FusionParams {
    opacity_pet: f32,
    opacity_ct: f32,
    use_table: u32,
    last: u32,
}

pub(crate) struct FusionGpu {
    window_pipeline: wgpu::RenderPipeline,
    fuse_pipeline: wgpu::RenderPipeline,
    window_uniforms: wgpu::BindGroup,
    window_sources: wgpu::BindGroup,
    fuse_uniforms: wgpu::BindGroup,
    fuse_sources: wgpu::BindGroup,
    pet_window: wgpu::TextureView,
    output: wgpu::TextureView,
    pet_voi: wgpu::Buffer,
    ct_voi: wgpu::Buffer,
    route: wgpu::Buffer,
    table: wgpu::Texture,
    pet_params: wgpu::Buffer,
    ct_params: wgpu::Buffer,
    pet_stored: wgpu::Texture,
    pet_presence: wgpu::Texture,
    ct_stored: wgpu::Texture,
    ct_presence: wgpu::Texture,
    pet_lut: wgpu::Texture,
    ct_lut: wgpu::Texture,
    _front_dummy: wgpu::Texture,
    dirty: Dirty,
    stamp: FusionStamp,
    uid: String,
    width: u32,
    height: u32,
    /// Bytes of one sample texel, including the zero lanes past `.r`.
    texel_bytes: u32,
    sample_uploads: u64,
}

impl FusionGpu {
    pub(crate) fn dirty(&self) -> Dirty {
        self.dirty
    }

    pub(crate) fn stamp(&self) -> FusionStamp {
        self.stamp
    }

    pub(crate) fn uid(&self) -> Option<&str> {
        Some(self.uid.as_str())
    }

    pub(crate) fn sample_uploads(&self) -> u64 {
        self.sample_uploads
    }

    pub(crate) fn matches_extent(&self, plane: &FusionPlane) -> bool {
        self.width == plane.width && self.height == plane.height
    }

    pub(crate) fn needs_encode(&self) -> bool {
        self.dirty.any()
    }

    /// Record the window pass and the composite pass. The caller submits once.
    pub(crate) fn encode(&mut self, encoder: &mut wgpu::CommandEncoder) -> u32 {
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("ocelli fusion window"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.pet_window,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 0.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            pass.set_pipeline(&self.window_pipeline);
            pass.set_bind_group(0, &self.window_uniforms, &[]);
            pass.set_bind_group(1, &self.window_sources, &[]);
            pass.draw(0..3, 0..1);
        }
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("ocelli fusion composite"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.output,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 0.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                ..Default::default()
            });
            pass.set_pipeline(&self.fuse_pipeline);
            pass.set_bind_group(0, &self.fuse_uniforms, &[]);
            pass.set_bind_group(1, &self.fuse_sources, &[]);
            pass.draw(0..3, 0..1);
        }
        self.dirty.clear();
        2
    }

    fn create(
        ctx: &GpuContext,
        adapter: &ResolvedAdapter,
        tier: Tier,
        format: wgpu::TextureFormat,
        request: &FusionRequest<'_>,
        stamp: FusionStamp,
    ) -> Result<Self, FusionError> {
        let sample_format = fusion_sample_format(adapter, tier)?;
        let texel_bytes = sample_texel_bytes(sample_format)?;
        let tables = prepare_tables(request.colormap_b, request.colormap_a)?;
        let uniforms = prepared_uniforms(request)?;
        let width = request.plane.width;
        let height = request.plane.height;
        let planes = prepare_plane_samples(request, width, height, texel_bytes)?;
        let device = ctx.device();
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ocelli fusion"),
            source: wgpu::ShaderSource::Wgsl(fusion_module().into()),
        });
        // Both volumes pre-apply stage 1 at plane packing, so the route flag
        // is 0 and the table is a 1x1 stand-in that nothing reads.
        let route = uniform_buffer(
            device,
            "ocelli fusion route",
            bytemuck::bytes_of(&ModalityRoute {
                tabled: 0,
                first: 0,
                fold: MODALITY_FOLD,
                pad: 0,
            }),
        );
        let table = texture_2d(
            device,
            "ocelli fusion modality table",
            1,
            1,
            wgpu::TextureFormat::R32Float,
            wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        );
        // Group 0: the VOI uniform at 0, the route uniform and the stand-in
        // table at the bindings `voi.wgsl` reserves, and FusionParams at 3.
        // Both volumes are pre-applied at plane packing, so the flag is 0.
        let table_view = table.create_view(&wgpu::TextureViewDescriptor::default());
        let uniform_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ocelli fusion uniform"),
            entries: &[
                uniform_entry(0),
                uniform_entry(1),
                float_entry(2),
                uniform_entry(3),
            ],
        });
        let source_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ocelli fusion source"),
            entries: &[
                float_entry(0),
                float_entry(1),
                unorm_entry(2),
                float_entry(3),
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ocelli fusion"),
            bind_group_layouts: &[Some(&uniform_layout), Some(&source_layout)],
            immediate_size: 0,
        });
        let window_pipeline = pipeline(
            device,
            &pipeline_layout,
            &module,
            wgpu::TextureFormat::Rgba16Float,
            "fs_window",
            "ocelli fusion window",
        );
        let fuse_pipeline = pipeline(
            device,
            &pipeline_layout,
            &module,
            format,
            "fs_fuse",
            "ocelli fusion composite",
        );
        let sample_usage = wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST;
        let pet_stored = texture_2d(
            device,
            "ocelli fusion pet",
            width,
            height,
            sample_format,
            sample_usage,
        );
        let pet_presence = texture_2d(
            device,
            "ocelli fusion pet presence",
            width,
            height,
            sample_format,
            sample_usage,
        );
        let ct_stored = texture_2d(
            device,
            "ocelli fusion ct",
            width,
            height,
            sample_format,
            sample_usage,
        );
        let ct_presence = texture_2d(
            device,
            "ocelli fusion ct presence",
            width,
            height,
            sample_format,
            sample_usage,
        );
        let pet_lut = texture_2d(
            device,
            "ocelli fusion pet lut",
            256,
            1,
            wgpu::TextureFormat::Rgba8Unorm,
            sample_usage,
        );
        let ct_lut = texture_2d(
            device,
            "ocelli fusion ct lut",
            256,
            1,
            wgpu::TextureFormat::Rgba8Unorm,
            sample_usage,
        );
        let front_usage =
            wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT;
        let pet_target = texture_2d(
            device,
            "ocelli fusion front",
            width,
            height,
            wgpu::TextureFormat::Rgba16Float,
            front_usage,
        );
        let output_tex = texture_2d(
            device,
            "ocelli fusion output",
            width,
            height,
            format,
            wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_SRC,
        );
        let front_dummy = texture_2d(
            device,
            "ocelli fusion front dummy",
            1,
            1,
            wgpu::TextureFormat::Rgba16Float,
            wgpu::TextureUsages::TEXTURE_BINDING,
        );
        let pet_voi = uniform_buffer(
            device,
            "ocelli fusion pet voi",
            bytemuck::bytes_of(&uniforms.voi_b.with_identity_rescale()),
        );
        let ct_voi = uniform_buffer(
            device,
            "ocelli fusion ct voi",
            bytemuck::bytes_of(&uniforms.voi_a.with_identity_rescale()),
        );
        let pet_params = uniform_buffer(
            device,
            "ocelli fusion pet params",
            bytemuck::bytes_of(&uniforms.pet),
        );
        let ct_params = uniform_buffer(
            device,
            "ocelli fusion ct params",
            bytemuck::bytes_of(&uniforms.ct),
        );
        let pet_window = pet_target.create_view(&wgpu::TextureViewDescriptor::default());
        let output = output_tex.create_view(&wgpu::TextureViewDescriptor::default());
        let dummy_view = front_dummy.create_view(&wgpu::TextureViewDescriptor::default());
        let window_uniforms = bind_uniforms(device, &uniform_layout, &pet_voi, &route, &table_view, &pet_params);
        let fuse_uniforms = bind_uniforms(device, &uniform_layout, &ct_voi, &route, &table_view, &ct_params);
        let window_sources = bind_sources(
            device,
            &source_layout,
            &pet_stored,
            &pet_presence,
            &pet_lut,
            &dummy_view,
        );
        let fuse_sources = bind_sources(
            device,
            &source_layout,
            &ct_stored,
            &ct_presence,
            &ct_lut,
            &pet_window,
        );
        let view = Self {
            window_pipeline,
            fuse_pipeline,
            window_uniforms,
            window_sources,
            fuse_uniforms,
            fuse_sources,
            pet_window,
            output,
            pet_voi,
            ct_voi,
            route,
            table,
            pet_params,
            ct_params,
            pet_stored,
            pet_presence,
            ct_stored,
            ct_presence,
            pet_lut,
            ct_lut,
            _front_dummy: front_dummy,
            dirty: Dirty::full(),
            stamp,
            uid: request.frame_a.0.clone(),
            width,
            height,
            texel_bytes,
            sample_uploads: 1,
        };
        commit_planes(
            ctx.queue(),
            &view.pet_stored,
            &view.pet_presence,
            &view.ct_stored,
            &view.ct_presence,
            &planes,
        );
        write_lut(ctx.queue(), &view.pet_lut, &tables.pet);
        write_lut(ctx.queue(), &view.ct_lut, &tables.ct);
        Ok(view)
    }

    fn update(
        &mut self,
        ctx: &GpuContext,
        request: &FusionRequest<'_>,
        stamp: FusionStamp,
    ) -> Result<(), FusionError> {
        let samples_changed = !same_samples(&self.stamp, &stamp);
        let tables_changed =
            self.stamp.table_a != stamp.table_a || self.stamp.table_b != stamp.table_b;
        // Pack and fill before any queue write. A refusal leaves the bound
        // textures and this stamp where they were.
        let planes = if samples_changed {
            Some(prepare_plane_samples(
                request,
                self.width,
                self.height,
                self.texel_bytes,
            )?)
        } else {
            None
        };
        let tables = if tables_changed {
            Some(prepare_tables(request.colormap_b, request.colormap_a)?)
        } else {
            None
        };
        let uniforms = prepared_uniforms(request)?;
        if let Some(planes) = planes.as_ref() {
            commit_planes(
                ctx.queue(),
                &self.pet_stored,
                &self.pet_presence,
                &self.ct_stored,
                &self.ct_presence,
                planes,
            );
            self.sample_uploads = self.sample_uploads.saturating_add(1);
        }
        if let Some(tables) = tables.as_ref() {
            write_lut(ctx.queue(), &self.pet_lut, &tables.pet);
            write_lut(ctx.queue(), &self.ct_lut, &tables.ct);
        }
        commit_uniforms(ctx.queue(), self, &uniforms);
        self.stamp = stamp;
        self.uid = request.frame_a.0.clone();
        self.dirty.data = true;
        Ok(())
    }
}

pub(crate) fn stage(
    ctx: &GpuContext,
    adapter: &ResolvedAdapter,
    tier: Tier,
    slot: &mut Option<FusionGpu>,
    format: wgpu::TextureFormat,
    request: &FusionRequest<'_>,
    stamp: FusionStamp,
) -> Result<(), FusionError> {
    let limit = ctx.device().limits().max_texture_dimension_2d;
    if !plane_fits_limit(request.plane.width, request.plane.height, limit) {
        return Err(FusionError::Unavailable);
    }
    if slot
        .as_ref()
        .is_some_and(|view| view.matches_extent(request.plane))
    {
        return match slot.as_mut() {
            Some(view) => view.update(ctx, request, stamp),
            None => Err(FusionError::Unavailable),
        };
    }
    let created = FusionGpu::create(ctx, adapter, tier, format, request, stamp)?;
    *slot = Some(created);
    Ok(())
}

fn same_samples(held: &FusionStamp, next: &FusionStamp) -> bool {
    held.samples_a == next.samples_a
        && held.samples_b == next.samples_b
        && held.geometry_a == next.geometry_a
        && held.geometry_b == next.geometry_b
        && held.plane == next.plane
        // A rescale change re-packs the planes, because the staged bytes
        // were computed with the old pair.
        && held.voi_a.slope == next.voi_a.slope
        && held.voi_a.intercept == next.voi_a.intercept
        && held.voi_b.slope == next.voi_b.slope
        && held.voi_b.intercept == next.voi_b.intercept
}

fn params_of(opacity_pet: f32, opacity_ct: f32, table: Option<&[Rgb]>) -> FusionParams {
    FusionParams {
        opacity_pet,
        opacity_ct,
        use_table: u32::from(table.is_some()),
        last: TABLE_LAST,
    }
}

struct PackedTables {
    pet: [u8; 1024],
    ct: [u8; 1024],
}

fn prepare_tables(pet: Option<&[Rgb]>, ct: Option<&[Rgb]>) -> Result<PackedTables, FusionError> {
    let mut pet_bytes = [0_u8; 1024];
    let mut ct_bytes = [0_u8; 1024];
    pack_table(&mut pet_bytes, pet)?;
    pack_table(&mut ct_bytes, ct)?;
    Ok(PackedTables {
        pet: pet_bytes,
        ct: ct_bytes,
    })
}

struct PreparedUniforms {
    voi_a: VoiParams,
    voi_b: VoiParams,
    pet: FusionParams,
    ct: FusionParams,
}

fn prepared_uniforms(request: &FusionRequest<'_>) -> Result<PreparedUniforms, FusionError> {
    Ok(PreparedUniforms {
        voi_a: VoiParams::from_chain(request.chain_a)?,
        voi_b: VoiParams::from_chain(request.chain_b)?,
        pet: params_of(request.opacity_b, request.opacity_a, request.colormap_b),
        ct: params_of(request.opacity_b, request.opacity_a, request.colormap_a),
    })
}

struct PreparedPlanes {
    row_bytes: u32,
    width: u32,
    height: u32,
    pet_stored: Vec<u8>,
    pet_presence: Vec<u8>,
    ct_stored: Vec<u8>,
    ct_presence: Vec<u8>,
}

fn prepare_plane_samples(
    request: &FusionRequest<'_>,
    width: u32,
    height: u32,
    texel_bytes: u32,
) -> Result<PreparedPlanes, FusionError> {
    let count = pixel_count(width, height)?;
    let mut pet_samples = vec![0.0; count];
    let mut pet_mask = vec![0.0; count];
    let mut ct_samples = vec![0.0; count];
    let mut ct_mask = vec![0.0; count];
    fill_plane(
        request.volume_b,
        request.plane,
        &mut pet_samples,
        &mut pet_mask,
    )?;
    fill_plane(
        request.volume_a,
        request.plane,
        &mut ct_samples,
        &mut ct_mask,
    )?;
    // Stage 1 on the CPU, per volume: pet through chain_b's rescale and ct
    // through chain_a's, in place on the Vecs above. A presence-0 texel is
    // skipped and stays at 0.0, which keeps the shader header's invariant
    // that a stored plane holds 0 where presence is 0.
    let voi_b = VoiParams::from_chain(request.chain_b)?;
    let voi_a = VoiParams::from_chain(request.chain_a)?;
    for (sample, present) in pet_samples.iter_mut().zip(&pet_mask) {
        if *present != 0.0 {
            *sample = ocelli_pixel::modality(Stored(*sample), voi_b.slope, voi_b.intercept).0;
        }
    }
    for (sample, present) in ct_samples.iter_mut().zip(&ct_mask) {
        if *present != 0.0 {
            *sample = ocelli_pixel::modality(Stored(*sample), voi_a.slope, voi_a.intercept).0;
        }
    }
    let raw = width.checked_mul(texel_bytes).ok_or(FusionError::Plane)?;
    let row_bytes = aligned_row_bytes(raw).ok_or(FusionError::Plane)?;
    Ok(PreparedPlanes {
        row_bytes,
        width,
        height,
        pet_stored: pack_plane(width, height, row_bytes, texel_bytes, &pet_samples)?,
        pet_presence: pack_plane(width, height, row_bytes, texel_bytes, &pet_mask)?,
        ct_stored: pack_plane(width, height, row_bytes, texel_bytes, &ct_samples)?,
        ct_presence: pack_plane(width, height, row_bytes, texel_bytes, &ct_mask)?,
    })
}

fn pack_plane(
    width: u32,
    height: u32,
    row_bytes: u32,
    texel_bytes: u32,
    samples: &[f32],
) -> Result<Vec<u8>, FusionError> {
    let rows = usize::try_from(height).map_err(|_| FusionError::Plane)?;
    let stride = usize::try_from(row_bytes).map_err(|_| FusionError::Plane)?;
    let bytes = rows.checked_mul(stride).ok_or(FusionError::Plane)?;
    let mut dst = vec![0_u8; bytes];
    pack_f32(&mut dst, width, height, row_bytes, texel_bytes, samples)?;
    Ok(dst)
}

fn commit_planes(
    queue: &wgpu::Queue,
    pet_stored: &wgpu::Texture,
    pet_presence: &wgpu::Texture,
    ct_stored: &wgpu::Texture,
    ct_presence: &wgpu::Texture,
    planes: &PreparedPlanes,
) {
    write_plane(
        queue,
        pet_stored,
        &planes.pet_stored,
        planes.row_bytes,
        planes.width,
        planes.height,
    );
    write_plane(
        queue,
        pet_presence,
        &planes.pet_presence,
        planes.row_bytes,
        planes.width,
        planes.height,
    );
    write_plane(
        queue,
        ct_stored,
        &planes.ct_stored,
        planes.row_bytes,
        planes.width,
        planes.height,
    );
    write_plane(
        queue,
        ct_presence,
        &planes.ct_presence,
        planes.row_bytes,
        planes.width,
        planes.height,
    );
}

fn write_plane(
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    data: &[u8],
    row_bytes: u32,
    width: u32,
    height: u32,
) {
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        data,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(row_bytes),
            rows_per_image: Some(height),
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
}

fn commit_uniforms(queue: &wgpu::Queue, view: &FusionGpu, uniforms: &PreparedUniforms) {
    // The planes hold modality values, so the uniform's rescale is identity
    // while FusionStamp keeps the real pair for comparison.
    queue.write_buffer(
        &view.ct_voi,
        0,
        bytemuck::bytes_of(&uniforms.voi_a.with_identity_rescale()),
    );
    queue.write_buffer(
        &view.pet_voi,
        0,
        bytemuck::bytes_of(&uniforms.voi_b.with_identity_rescale()),
    );
    queue.write_buffer(&view.pet_params, 0, bytemuck::bytes_of(&uniforms.pet));
    queue.write_buffer(&view.ct_params, 0, bytemuck::bytes_of(&uniforms.ct));
}

fn fusion_sample_format(
    adapter: &ResolvedAdapter,
    tier: Tier,
) -> Result<wgpu::TextureFormat, FusionError> {
    let handle = adapter.wgpu_adapter();
    let stored = [
        handle.get_texture_format_features(wgpu::TextureFormat::R32Float),
        handle.get_texture_format_features(wgpu::TextureFormat::Rg32Float),
        handle.get_texture_format_features(wgpu::TextureFormat::Rgba32Float),
    ];
    let window = handle.get_texture_format_features(wgpu::TextureFormat::Rgba16Float);
    sample_format_for(tier, stored, window)
}

fn sample_format_for(
    tier: Tier,
    stored: [wgpu::TextureFormatFeatures; 3],
    window: wgpu::TextureFormatFeatures,
) -> Result<wgpu::TextureFormat, FusionError> {
    let needed = wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING;
    if !window.allowed_usages.contains(needed) {
        return Err(FusionError::Unavailable);
    }
    Upload::preserving_format(tier, stored).map_err(|_| FusionError::Unavailable)
}

fn sample_texel_bytes(format: wgpu::TextureFormat) -> Result<u32, FusionError> {
    let channels = crate::graph::stored_float_channels(format).ok_or(FusionError::Unavailable)?;
    channels.checked_mul(4).ok_or(FusionError::Unavailable)
}

fn fill_plane(
    volume: &Volume,
    plane: &FusionPlane,
    stored: &mut [f32],
    presence: &mut [f32],
) -> Result<(), FusionError> {
    let mut row = 0_u32;
    while row < plane.height {
        let mut column = 0_u32;
        while column < plane.width {
            let point = world_on_plane(plane, column, row);
            let index = pixel_index(column, row, plane.width)?;
            let (value, flag) = match volume.sample_world(point) {
                PlaneSample::Present(value) => (value, 1.0_f32),
                PlaneSample::Absent => (0.0, 0.0),
            };
            let Some(slot) = stored.get_mut(index) else {
                return Err(FusionError::Plane);
            };
            *slot = value;
            let Some(mask) = presence.get_mut(index) else {
                return Err(FusionError::Plane);
            };
            *mask = flag;
            column = column.saturating_add(1);
        }
        row = row.saturating_add(1);
    }
    Ok(())
}

fn pixel_count(width: u32, height: u32) -> Result<usize, FusionError> {
    let columns = usize::try_from(width).map_err(|_| FusionError::Plane)?;
    let rows = usize::try_from(height).map_err(|_| FusionError::Plane)?;
    columns.checked_mul(rows).ok_or(FusionError::Plane)
}

fn pixel_index(column: u32, row: u32, width: u32) -> Result<usize, FusionError> {
    let x = usize::try_from(column).map_err(|_| FusionError::Plane)?;
    let y = usize::try_from(row).map_err(|_| FusionError::Plane)?;
    let columns = usize::try_from(width).map_err(|_| FusionError::Plane)?;
    y.checked_mul(columns)
        .and_then(|base| base.checked_add(x))
        .ok_or(FusionError::Plane)
}

fn pack_f32(
    dst: &mut [u8],
    width: u32,
    height: u32,
    row_bytes: u32,
    texel_bytes: u32,
    samples: &[f32],
) -> Result<(), FusionError> {
    if texel_bytes < 4 || !texel_bytes.is_multiple_of(4) {
        return Err(FusionError::Unavailable);
    }
    for byte in dst.iter_mut() {
        *byte = 0;
    }
    let stride = usize::try_from(row_bytes).map_err(|_| FusionError::Plane)?;
    let columns = usize::try_from(width).map_err(|_| FusionError::Plane)?;
    let texel = usize::try_from(texel_bytes).map_err(|_| FusionError::Unavailable)?;
    let mut row = 0_u32;
    while row < height {
        let y = usize::try_from(row).map_err(|_| FusionError::Plane)?;
        let start = y.checked_mul(stride).ok_or(FusionError::Plane)?;
        let mut column = 0_usize;
        while column < columns {
            let sample_at = y
                .checked_mul(columns)
                .and_then(|base| base.checked_add(column))
                .ok_or(FusionError::Plane)?;
            let sample = samples.get(sample_at).copied().ok_or(FusionError::Plane)?;
            let byte_at = start
                .checked_add(column.checked_mul(texel).ok_or(FusionError::Plane)?)
                .ok_or(FusionError::Plane)?;
            let bytes = sample.to_le_bytes();
            let mut offset = 0_usize;
            while offset < 4 {
                let Some(slot) = dst.get_mut(byte_at.saturating_add(offset)) else {
                    return Err(FusionError::Plane);
                };
                let Some(byte) = bytes.get(offset) else {
                    return Err(FusionError::Plane);
                };
                *slot = *byte;
                offset = offset.saturating_add(1);
            }
            column = column.saturating_add(1);
        }
        row = row.saturating_add(1);
    }
    Ok(())
}

fn pack_table(dst: &mut [u8], table: Option<&[Rgb]>) -> Result<(), FusionError> {
    for byte in dst.iter_mut() {
        *byte = 0;
    }
    let Some(entries) = table else {
        return Ok(());
    };
    if entries.len() != 256 {
        return Err(FusionError::ColormapLength);
    }
    for (index, rgb) in entries.iter().enumerate() {
        let start = index.checked_mul(4).ok_or(FusionError::Unavailable)?;
        put_channel(dst, start, rgb.r)?;
        put_channel(dst, start.saturating_add(1), rgb.g)?;
        put_channel(dst, start.saturating_add(2), rgb.b)?;
        let Some(alpha) = dst.get_mut(start.saturating_add(3)) else {
            return Err(FusionError::Unavailable);
        };
        *alpha = 255;
    }
    Ok(())
}

fn put_channel(dst: &mut [u8], at: usize, value: f32) -> Result<(), FusionError> {
    let byte = byte_from_channel(value).ok_or(FusionError::Unavailable)?;
    let Some(slot) = dst.get_mut(at) else {
        return Err(FusionError::Unavailable);
    };
    *slot = byte;
    Ok(())
}

fn write_lut(queue: &wgpu::Queue, texture: &wgpu::Texture, bytes: &[u8]) {
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        bytes,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(1024),
            rows_per_image: Some(1),
        },
        wgpu::Extent3d {
            width: 256,
            height: 1,
            depth_or_array_layers: 1,
        },
    );
}

fn texture_2d(
    device: &wgpu::Device,
    label: &str,
    width: u32,
    height: u32,
    format: wgpu::TextureFormat,
    usage: wgpu::TextureUsages,
) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage,
        view_formats: &[],
    })
}

fn uniform_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

fn float_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: false },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    }
}

fn unorm_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: false },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    }
}

fn uniform_buffer(device: &wgpu::Device, label: &str, contents: &[u8]) -> wgpu::Buffer {
    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some(label),
        contents,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    })
}

fn bind_uniforms(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    voi: &wgpu::Buffer,
    route: &wgpu::Buffer,
    table_view: &wgpu::TextureView,
    params: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("ocelli fusion uniforms"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: voi.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: route.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::TextureView(table_view),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: params.as_entire_binding(),
            },
        ],
    })
}

fn bind_sources(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    stored: &wgpu::Texture,
    presence: &wgpu::Texture,
    lut: &wgpu::Texture,
    front: &wgpu::TextureView,
) -> wgpu::BindGroup {
    let stored_view = stored.create_view(&wgpu::TextureViewDescriptor::default());
    let presence_view = presence.create_view(&wgpu::TextureViewDescriptor::default());
    let lut_view = lut.create_view(&wgpu::TextureViewDescriptor::default());
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("ocelli fusion sources"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&stored_view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&presence_view),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::TextureView(&lut_view),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: wgpu::BindingResource::TextureView(front),
            },
        ],
    })
}

fn pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    module: &wgpu::ShaderModule,
    format: wgpu::TextureFormat,
    entry: &'static str,
    label: &'static str,
) -> wgpu::RenderPipeline {
    let stage = wgpu::PipelineCompilationOptions {
        constants: &[],
        zero_initialize_workgroup_memory: true,
    };
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module,
            entry_point: Some("vs_main"),
            compilation_options: stage.clone(),
            buffers: &[],
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module,
            entry_point: Some(entry),
            compilation_options: stage,
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}

#[cfg(test)]
mod format_decision {
    use super::{FusionError, prepare_tables, sample_format_for};
    use crate::caps::Tier;
    use ocelli_pixel::Rgb;

    fn sampleable() -> wgpu::TextureFormatFeatures {
        wgpu::TextureFormatFeatures {
            allowed_usages: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            flags: wgpu::TextureFormatFeatureFlags::empty(),
        }
    }

    fn unsampleable() -> wgpu::TextureFormatFeatures {
        wgpu::TextureFormatFeatures {
            allowed_usages: wgpu::TextureUsages::COPY_SRC,
            flags: wgpu::TextureFormatFeatureFlags::empty(),
        }
    }

    #[test]
    fn a_fusion_plane_past_the_device_limit_does_not_fit() {
        assert!(super::plane_fits_limit(4, 4, 4));
        assert!(!super::plane_fits_limit(5, 4, 4));
        assert!(!super::plane_fits_limit(4, 5, 4));
    }

    fn window_ok() -> wgpu::TextureFormatFeatures {
        wgpu::TextureFormatFeatures {
            allowed_usages: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING,
            flags: wgpu::TextureFormatFeatureFlags::empty(),
        }
    }

    #[test]
    fn an_unbindable_sample_format_is_unavailable() {
        let window = window_ok();
        let none = [unsampleable(), unsampleable(), unsampleable()];
        assert_eq!(
            sample_format_for(Tier::B, none, window),
            Err(FusionError::Unavailable)
        );
        let widened = [unsampleable(), sampleable(), sampleable()];
        assert_eq!(
            sample_format_for(Tier::B, widened, window),
            Ok(wgpu::TextureFormat::Rg32Float)
        );
        assert_eq!(
            sample_format_for(Tier::A, widened, window),
            Err(FusionError::Unavailable)
        );
        assert_ne!(
            sample_format_for(Tier::B, widened, window).ok(),
            Some(wgpu::TextureFormat::Rgba8Unorm)
        );
        let stored = [sampleable(), sampleable(), sampleable()];
        let window_missing = wgpu::TextureFormatFeatures {
            allowed_usages: wgpu::TextureUsages::COPY_DST,
            flags: wgpu::TextureFormatFeatureFlags::empty(),
        };
        assert_eq!(
            sample_format_for(Tier::A, stored, window_missing),
            Err(FusionError::Unavailable)
        );
    }

    #[test]
    fn a_fractional_ct_channel_is_refused_before_either_table_is_ok() {
        let pet = [Rgb {
            r: 255.0,
            g: 0.0,
            b: 0.0,
        }; 256];
        let mut ct = [Rgb {
            r: 0.0,
            g: 0.0,
            b: 0.0,
        }; 256];
        let Some(first) = ct.first_mut() else {
            unreachable!("the table has one entry");
        };
        first.r = 0.5;
        assert!(matches!(
            prepare_tables(Some(&pet), Some(&ct)),
            Err(FusionError::Unavailable)
        ));
        let ct_byte = [Rgb {
            r: 7.0,
            g: 0.0,
            b: 0.0,
        }; 256];
        let packed = match prepare_tables(Some(&pet), Some(&ct_byte)) {
            Ok(packed) => packed,
            Err(_) => unreachable!("both tables are exact bytes"),
        };
        assert_eq!(packed.pet.first().copied(), Some(255));
        assert_eq!(packed.ct.first().copied(), Some(7));
    }
}
