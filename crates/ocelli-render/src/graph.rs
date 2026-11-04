//! HLD section 22's render graph and frame scheduler, F-038.
//!
//! The graph registers viewports, tracks dirt per viewport, compiles every
//! pipeline it will use at creation, encodes each dirty viewport once per
//! frame into one command encoder, and submits that encoder exactly once.
//!
//! **F-039 acquires a surface per viewport that has one.** `tick` still
//! submits once. A viewport with a surface draws into
//! `Surface::get_current_texture`, and `present` runs after that submit, not
//! per viewport. A viewport with no surface keeps the caller-supplied view
//! from F-038. F-040 uploads stored samples with `write_texture` into an
//! `R32Float` texture created at registration. F-042 keeps that format when
//! the adapter can sample it, and on tier B may store `Rg32Float` or
//! `Rgba32Float` so `textureLoad` still returns the stored `f32`. F-044
//! uploads a colour LUT into a separate `Rgba8Unorm` texture of height 1.
//! F-043 uploads each ready brick as its own 3D texture at the brick's true
//! extent. `set_voi` does not upload that texture either. `brick_uploads`
//! counts the brick upload. F-057 calls that upload for each ready brick
//! whose bytes changed. An incomplete brick has no texture. F-060 replaces
//! the ray's constant opacity with a 256-entry uniform. `set_opacity` writes
//! that uniform and does not upload a brick. F-061 multiplies the ray colour
//! by a fragment shade. The default weights are unlit, so the F-060 colour
//! stays until `set_shade`. That call writes a 16-byte uniform and does not
//! upload a brick. F-062 clips that ray to at most six patient planes. The
//! default list is empty, so the unlit integral stays until
//! `set_clip_planes`. That call writes a 256-byte uniform and does not
//! upload a brick. F-063 uploads a composite labelmap from `ocelli-seg`
//! presenter bytes. The copy shader does not run the image VOI and does not
//! recompute the outline. F-067 compiles labelmap edge projection for the
//! segmentation overlay on that same copy shader and uploads outline
//! without fill. Slab, volume and stack leave that blend uncompiled. F-045 writes
//! the sixteen camera floats at group 0 binding 2. `set_view` does not
//! write the VOI uniform. The oracle still has no Ocelli frame to judge.
//! Decision D7 stays open.

use std::collections::BTreeMap;

use ocelli_cache::{Brick, BrickKey, VolumeId};
use ocelli_pixel::LutChain;
use ocelli_volume::{OrthoImage, PlaneSample, SlabImage, Volume};
use wgpu::util::DeviceExt;

use crate::brick_upload::{BrickResidency, BrickUploadError, brick_format};
use crate::caps::Tier;
use crate::clip::{CLIP_UNIFORM_BYTES, ClipList, PatientFrame};
use crate::colour::{ColourLut, ColourParams};
use crate::fusion::{self, FusionError, FusionGpu, FusionRequest};
use crate::gpu::{GpuContext, Recovered, SurfaceError, first_render_attachment};
use crate::interpolation::Interpolation;
pub use crate::labelmap::LabelmapDrawError;
use crate::opacity::{OpacityPreset, OpacityTable};
use crate::ortho::{self, OrthoError};
use crate::probe::ResolvedAdapter;
use crate::progressive::{self, ProgressError, ProgressReport};
use crate::shade::{ShadeWeights, spacing_mm_ok};
use crate::slab::{self, SlabError};
use crate::voi::{VOI_WGSL, VoiParams, VoiParamsError};

/// HLD section 22 listed stack, volume raycast and segmentation overlay.
/// F-054 adds the thick slab.
pub enum Pass {
    /// A stack viewport, windowed through section 18.4's uniform.
    Stack(StackPass),
    /// A volume raycast viewport.
    VolumeRaycast(VolumePass),
    /// A thick slab. The blend selects one of the four compiled pipelines.
    Slab(SlabPass),
    /// A segmentation overlay, scheduled after its stack pass.
    SegOverlay(SegPass),
}

impl Pass {
    /// Which pass kind this is. Section 22 named three. The slab is the fourth.
    pub fn kind(&self) -> PassKind {
        match self {
            Self::Stack(_) => PassKind::Stack,
            Self::VolumeRaycast(_) => PassKind::VolumeRaycast,
            Self::Slab(_) => PassKind::Slab,
            Self::SegOverlay(_) => PassKind::SegOverlay,
        }
    }

    /// The blend mode this pass schedules under.
    pub fn blend(&self) -> BlendMode {
        match self {
            Self::Stack(pass) => pass.blend,
            Self::VolumeRaycast(pass) => pass.blend,
            Self::Slab(pass) => pass.blend,
            Self::SegOverlay(pass) => pass.blend,
        }
    }
}

/// The stack pass payload. Scheduling data only, no pixel data and no LUT
/// decision: `voi` is the resolved uniform F-041 defined.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StackPass {
    /// Section 18.4's uniform for this viewport.
    pub voi: VoiParams,
    /// The blend mode the pipeline is keyed by.
    pub blend: BlendMode,
}

/// Which volume axis a ray walks. The pixel axes are the other two.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RayAxis {
    /// Walk x. The image is y then z.
    Column,
    /// Walk y. The image is x then z.
    Row,
    /// Walk z. The image is x then y.
    Slice,
}

impl RayAxis {
    /// Uniform code. 0 is x, 1 is y, 2 is z. The shader matches these numbers.
    pub const fn code(self) -> u32 {
        match self {
            Self::Column => 0,
            Self::Row => 1,
            Self::Slice => 2,
        }
    }

    fn length(self, dims: [u32; 3]) -> u32 {
        match self {
            Self::Column => dims[0],
            Self::Row => dims[1],
            Self::Slice => dims[2],
        }
    }
}

/// The volume raycast payload.
///
/// `voi` is the same section 18.4 uniform the stack pass carries. `axis` and
/// `from_far` choose the ray. The shader body exists only for composite.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VolumePass {
    /// The blend mode the pipeline is keyed by.
    pub blend: BlendMode,
    /// Section 18.4's uniform for this viewport.
    pub voi: VoiParams,
    /// The volume axis the ray walks.
    pub axis: RayAxis,
    /// When set, the far voxel is the first sample.
    pub from_far: bool,
}

/// The thick-slab payload.
///
/// `blend` selects the compiled slab pipeline. `voi` is the same section
/// 18.4 uniform the stack pass carries. The shader reduces stored samples
/// and calls that uniform once.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SlabPass {
    /// The blend mode the pipeline is keyed by.
    pub blend: BlendMode,
    /// Section 18.4's uniform for this viewport.
    pub voi: VoiParams,
}

/// The segmentation overlay payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SegPass {
    /// The blend mode the pipeline is keyed by.
    pub blend: BlendMode,
}

/// The five blend modes the parity surface counts.
///
/// The slab pass compiles composite, MIP, MinIP and average. The volume ray
/// and the stack pass compile composite only. Labelmap edge projection
/// compiles for the segmentation overlay only, on the same copy shader as
/// composite. Slab, volume and stack have no edge body. A lookup for an
/// uncompiled key reports unavailable rather than substituting composite,
/// which is deviation D-07's honesty rule applied to a blend mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BlendMode {
    /// The ordinary composite.
    Composite,
    /// Maximum intensity projection. Compiled for the slab pass only.
    /// The volume ray has no MIP body.
    Mip,
    /// Minimum intensity projection. Compiled for the slab pass only.
    MinIp,
    /// Average intensity projection. Compiled for the slab pass only.
    Average,
    /// Labelmap edge projection.
    ///
    /// The overlay compiles this key on the existing labelmap copy shader.
    /// Slab, volume and stack do not.
    LabelmapEdgeProjection,
}

/// Section 22's pass kinds, as a cache-key field.
///
/// `Slab` is the thick-slab key. It is not a volume ray and it is not a
/// stack. The HLD listed three kinds and did not say those were the last.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PassKind {
    /// The stack pass.
    Stack,
    /// The volume raycast pass.
    VolumeRaycast,
    /// The thick-slab pass.
    Slab,
    /// The segmentation overlay pass.
    SegOverlay,
}

/// A viewport identity. An index would transpose silently at a call site and
/// dirty the wrong viewport, which is a wrong frame with no error, and this
/// newtype removes that case in the same way `probe`'s `Edge` and `Passes` do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ViewportId(pub u32);

/// Section 22's pipeline cache key, transcribed into a type: pass kind, blend
/// mode and tier. The tier field is the variant axis. F-042 does not add a
/// field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PipelineKey {
    /// The pass kind.
    pub pass: PassKind,
    /// The blend mode.
    pub blend: BlendMode,
    /// The resolved tier the pipeline was compiled for.
    pub tier: Tier,
}

/// Which shader shape a key is being asked for.
///
/// The fragment shape is the stack composite body. Compute and storage are
/// tier A shapes. On tier B they are not compiled, and a lookup does not
/// return the fragment pipeline in their place.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PipelineFamily {
    /// Vertex and fragment. Legal on tier A and tier B.
    Fragment,
    /// A compute entry point. Not compiled on tier B.
    Compute,
    /// A storage-buffer pipeline. Not compiled on tier B.
    Storage,
}

/// One viewport's dirt, five flags named from section 22 word for word.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Dirty {
    /// The camera moved.
    pub camera: bool,
    /// The window or level changed.
    pub voi: bool,
    /// The slice changed.
    pub slice: bool,
    /// The segmentation changed.
    pub segmentation: bool,
    /// The data changed.
    pub data: bool,
}

impl Dirty {
    /// Fully dirty, which is how a viewport starts at registration.
    pub fn full() -> Self {
        Self {
            camera: true,
            voi: true,
            slice: true,
            segmentation: true,
            data: true,
        }
    }

    /// Whether any source has marked this viewport.
    pub fn any(self) -> bool {
        self.camera || self.voi || self.slice || self.segmentation || self.data
    }

    /// Clear every flag. Called only after a successful encode.
    pub fn clear(&mut self) {
        *self = Self::default();
    }
}

/// What one tick did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameOutcome {
    /// One encoder was submitted, covering this many viewports and passes.
    Submitted {
        /// Dirty viewports encoded into the submission.
        viewports: u32,
        /// Passes encoded into the submission.
        passes: u32,
    },
    /// Nothing was submitted. Either nothing was dirty, or every dirty
    /// surface viewport failed to yield a texture and its dirt stays.
    Noop,
    /// The device is lost. No dirt was consumed.
    DeviceLost,
    /// A viewport's pass has no compiled pipeline. Its dirt is retained.
    Unavailable {
        /// The first viewport that could not be encoded.
        viewport: ViewportId,
    },
}

/// Why the graph refused a call.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum GraphError {
    /// Registration beyond the capacity fixed at creation.
    #[error("viewport registration beyond the capacity fixed at graph creation")]
    CapacityExhausted,
    /// A viewport id this graph never issued.
    #[error("viewport id was never registered with this graph")]
    UnknownViewport,
    /// `set_representation` was called on a stack, a slab or a volume.
    #[error("this viewport is not a segmentation overlay")]
    NotOverlay,
    /// A VOI chain with no section 18.4 uniform.
    #[error("{0}")]
    Voi(#[from] VoiParamsError),
    /// Graph creation on tier C, which holds no device and reports unavailable.
    #[error("the render graph is unavailable on tier C, which holds no device")]
    TierCUnavailable,
    /// The requested stack sample mode is not offered on this tier.
    #[error("this interpolation mode is not available on this tier")]
    InterpolationUnavailable,
    /// A VOI dirty mark offered through `mark_dirty`.
    #[error(
        "VOI dirt is set only by set_voi, so a window change cannot schedule a frame without writing its uniform bytes"
    )]
    VoiViaMarkDirty,
    /// A uniform write offered for a pass with no uniform.
    #[error("this pass has no section 18.4 uniform to write")]
    PassHasNoUniform,
    /// A viewport registered but not yet rebound after a device rebuild.
    #[error("this viewport is registered but has no GPU objects on this device, rebind it first")]
    ViewportNotRebound,
    /// `rebind` was called on a viewport that owns a stored-sample texture.
    #[error("this viewport owns a stored-sample texture, rebind it with rebind_stored")]
    StoredNeedsRebind,
    /// `Rgba8Unorm` cannot be sampled and copied, the table has no texels,
    /// or the table is wider than the device `max_texture_dimension_2d`.
    #[error("the colour LUT texture cannot be sampled and copied")]
    ColourUnavailable,
    /// A device pixel ratio that is not finite and strictly positive, or that
    /// narrows to a non-positive `f32`.
    #[error(
        "the device pixel ratio must be finite, strictly positive and narrow to a positive f32"
    )]
    DevicePixelRatio,
    /// The ready brick count does not fit the sampled-texture limit.
    ///
    /// One sampled binding is the residency texture. Room for bricks is
    /// that limit minus one. A limit of zero, a ready count of zero, or a
    /// ready count above that room is refused. A count above the compiled
    /// brick slots is the same error and creates no texture. The ray does
    /// not draw a subset of the bricks the caller passed.
    #[error("the ready brick count does not fit the sampled-texture limit")]
    SampledTextureLimit,
    /// The ray has no brick, the flags do not match the axis, or the brick
    /// format is not an unsigned integer container.
    #[error("the volume ray has no unsigned brick and matching residency flags")]
    RayUnavailable,
    /// More than six clip planes.
    #[error("a volume ray accepts at most six clip planes")]
    ClipCount,
    /// A clip component is NaN or infinite, or it does not narrow to a finite f32.
    #[error("a clip plane component is not finite")]
    ClipNotFinite,
    /// The normal length is zero in f64, or its f32 image has length zero.
    #[error("a clip plane normal has length zero")]
    ClipZeroNormal,
}

/// Preferred stored-sample format. The constant stays `R32Float`.
///
/// `selected_stored_format` accepts this format on every tier when it has
/// `TEXTURE_BINDING` and `COPY_DST`. Tier A reports
/// [`UploadError::FormatUnavailable`] when it does not. Tier B may then use
/// `Rg32Float` and, only if that also lacks those usages, `Rgba32Float`.
/// `Rgba8Unorm` is not a candidate.
pub const STORED_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::R32Float;

/// Colour table format. Height 1. Not a stored-sample candidate.
const COLOUR_LUT_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

/// Column-major identity. Registration writes this, so a viewport that never
/// calls `set_view` samples the fragment coordinate unchanged.
const IDENTITY_CAMERA: [f32; 16] = [
    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
];

/// CSS camera plus the device-to-CSS scale the fragment shader divides by.
///
/// `scale.x` is the device pixel ratio. `scale.y` is the interpolation mode,
/// `0` nearest and `1` linear. The remaining lanes are padding so the
/// uniform stays a multiple of 16 bytes.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct DeviceCamera {
    css: [f32; 16],
    scale: [f32; 4],
}

fn device_camera(css: [f32; 16], dpr: f64, mode: Interpolation) -> DeviceCamera {
    DeviceCamera {
        css,
        scale: [
            ocelli_viewport::view::f32_from_f64(dpr),
            mode.lane(),
            0.0,
            0.0,
        ],
    }
}

/// The 20 floats uploaded for one camera. Index 16 is the device pixel ratio.
#[must_use]
pub fn device_camera_words(css: [f32; 16], dpr: f64) -> [f32; 20] {
    let uploaded = device_camera(css, dpr, Interpolation::Nearest);
    let mut words = [0.0_f32; 20];
    words[..16].copy_from_slice(&uploaded.css);
    words[16..].copy_from_slice(&uploaded.scale);
    words
}

/// Extent of the `R32Float` texture created for one viewport's stored samples.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Upload {
    width: u32,
    height: u32,
}

impl Upload {
    /// Decide an `R32Float` upload from adapter format features.
    ///
    /// `features` is what `get_texture_format_features` returned for
    /// [`STORED_FORMAT`]. `TEXTURE_BINDING` or `COPY_DST` absent is
    /// [`UploadError::FormatUnavailable`]. No other format is selected.
    /// `max_texture_dimension_2d` is the device limit. An axis longer than
    /// that limit is [`UploadError::ExtentUnavailable`]. An axis equal to it
    /// is accepted.
    pub fn stored(
        features: wgpu::TextureFormatFeatures,
        width: u32,
        height: u32,
        max_texture_dimension_2d: u32,
    ) -> Result<Self, UploadError> {
        let upload = Self::from_extent(width, height, max_texture_dimension_2d)?;
        if !binding_available(features) {
            return Err(UploadError::FormatUnavailable);
        }
        Ok(upload)
    }

    /// Choose a stored-sample format that `textureLoad` returns as `f32`.
    ///
    /// `features` is `[R32Float, Rg32Float, Rgba32Float]`, in that order.
    /// `R32Float` wins on every tier when it can be sampled and written.
    /// Tier B may use the next float format when `R32Float` cannot. Tier A
    /// does not substitute. `Rgba8Unorm` is not a candidate, because
    /// normalising the samples would be a different number.
    pub fn preserving_format(
        tier: Tier,
        features: [wgpu::TextureFormatFeatures; 3],
    ) -> Result<wgpu::TextureFormat, UploadError> {
        let formats = [
            wgpu::TextureFormat::R32Float,
            wgpu::TextureFormat::Rg32Float,
            wgpu::TextureFormat::Rgba32Float,
        ];
        for (index, format) in formats.into_iter().enumerate() {
            let Some(feature) = features.get(index) else {
                continue;
            };
            if !binding_available(*feature) {
                continue;
            }
            if format == wgpu::TextureFormat::R32Float || tier == Tier::B {
                return Ok(format);
            }
        }
        Err(UploadError::FormatUnavailable)
    }

    fn from_extent(
        width: u32,
        height: u32,
        max_texture_dimension_2d: u32,
    ) -> Result<Self, UploadError> {
        if width == 0 || height == 0 {
            return Err(UploadError::EmptyExtent);
        }
        if width > max_texture_dimension_2d || height > max_texture_dimension_2d {
            return Err(UploadError::ExtentUnavailable);
        }
        if width.checked_mul(4).is_none() {
            return Err(UploadError::ExtentOverflow);
        }
        let width_us = usize::try_from(width).map_err(|_| UploadError::ExtentOverflow)?;
        let height_us = usize::try_from(height).map_err(|_| UploadError::ExtentOverflow)?;
        if width_us.checked_mul(height_us).is_none() {
            return Err(UploadError::ExtentOverflow);
        }
        Ok(Self { width, height })
    }

    /// Width in texels.
    pub const fn width(self) -> u32 {
        self.width
    }

    /// Height in texels.
    pub const fn height(self) -> u32 {
        self.height
    }

    /// Whether `samples` is exactly one stored value per texel.
    ///
    /// A refusal means nothing is uploaded. The check does not write.
    pub fn accepts(self, samples: &[f32]) -> Result<(), UploadError> {
        let width = usize::try_from(self.width).map_err(|_| UploadError::ExtentOverflow)?;
        let height = usize::try_from(self.height).map_err(|_| UploadError::ExtentOverflow)?;
        let expected = width
            .checked_mul(height)
            .ok_or(UploadError::ExtentOverflow)?;
        if samples.len() == expected {
            Ok(())
        } else {
            Err(UploadError::WrongLength)
        }
    }
}

fn binding_available(features: wgpu::TextureFormatFeatures) -> bool {
    let needed = wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST;
    features.allowed_usages.contains(needed)
}

/// Channels a preserving format stores, with the sample in `.r`.
///
/// `R16Float` is not here. It is not the stored `f32` after `textureLoad`.
pub(crate) fn stored_float_channels(format: wgpu::TextureFormat) -> Option<u32> {
    match format {
        wgpu::TextureFormat::R32Float => Some(1),
        wgpu::TextureFormat::Rg32Float => Some(2),
        wgpu::TextureFormat::Rgba32Float => Some(4),
        _ => None,
    }
}

fn stored_bytes_per_row(width: u32, format: wgpu::TextureFormat) -> Result<u32, UploadError> {
    let channels = stored_float_channels(format).ok_or(UploadError::FormatUnavailable)?;
    let bytes_per_texel = channels.checked_mul(4).ok_or(UploadError::ExtentOverflow)?;
    width
        .checked_mul(bytes_per_texel)
        .ok_or(UploadError::ExtentOverflow)
}

/// The first preserving format this adapter can sample, in F-040's order.
fn selected_stored_format(
    adapter: &wgpu::Adapter,
    tier: Tier,
) -> Result<wgpu::TextureFormat, UploadError> {
    let features = [
        adapter.get_texture_format_features(wgpu::TextureFormat::R32Float),
        adapter.get_texture_format_features(wgpu::TextureFormat::Rg32Float),
        adapter.get_texture_format_features(wgpu::TextureFormat::Rgba32Float),
    ];
    Upload::preserving_format(tier, features)
}

/// Why a stored-sample upload did not write.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum UploadError {
    /// No sampled float format keeps the stored value as `f32`.
    #[error(
        "no float format can be sampled and copied while textureLoad still returns the stored value"
    )]
    FormatUnavailable,
    /// `samples.len()` is not `width * height`. Nothing was written.
    #[error("stored sample length is not width times height, so nothing was uploaded")]
    WrongLength,
    /// A viewport id this graph never issued.
    #[error("viewport id was never registered with this graph")]
    UnknownViewport,
    /// The viewport was registered with a caller-supplied source view.
    #[error("this viewport has no stored-sample texture")]
    NotStored,
    /// GPU objects were dropped and have not been recreated.
    #[error("this viewport is registered but has no GPU objects on this device, rebind it first")]
    NotRebound,
    /// Width or height is zero, so there is no texture to create.
    #[error("a stored texture extent is empty")]
    EmptyExtent,
    /// An axis is longer than the device `max_texture_dimension_2d`.
    ///
    /// The texture is not created.
    #[error("a stored texture axis is longer than the device texture limit")]
    ExtentUnavailable,
    /// Width times height does not fit in the copy dimensions.
    #[error("a stored texture extent does not fit the copy")]
    ExtentOverflow,
    /// Registration beyond the capacity fixed at creation.
    #[error("viewport registration beyond the capacity fixed at graph creation")]
    CapacityExhausted,
}

/// The fragment entry appended to `VOI_WGSL` at module build.
///
/// Group 0 binding 0 is section 18.4's and stays reserved by `voi.wgsl`.
/// Group 0 binding 1 is the colour-table address. Group 0 binding 2 is the
/// column-major camera matrix. Group 1 binding 0 is the caller's
/// stored-value texture. Group 1 binding 1 is the `Rgba8Unorm` colour table.
/// Both textures are read with `textureLoad`. No sampler state decides
/// anything and no compute or storage buffer appears.
///
/// `colour.mode` 2 indexes the table by the stored sample and does not call
/// `lut_chain`. `colour.mode` 1 calls `lut_chain` and indexes by that display
/// value. Grey LINEAR and LINEAR_EXACT with output range 0 to 255 and
/// `colour.pad` 1 read the same 1024 ramp as `reference_ramp_index`. SIGMOID
/// stays on `table_index`. Mode 0 is the greyscale normalisation, and it
/// still emits the unclamped display value.
///
/// Linear reads the four stored texels, runs `lut_chain` on each, and blends
/// those display values with the same weights as `blend_display`. Palette
/// linear blends the four table colours. It does not blend stored indices.
const GRAPH_WGSL: &str = r"
struct ColourParams {
    mode : u32,
    origin : i32,
    last : u32,
    pad : u32,
};

@group(0) @binding(1) var<uniform> colour : ColourParams;
struct DeviceCamera {
    css: mat4x4<f32>,
    scale: vec4<f32>,
}
@group(0) @binding(2) var<uniform> camera : DeviceCamera;
@group(1) @binding(0) var stored_tex : texture_2d<f32>;
@group(1) @binding(1) var colour_lut : texture_2d<f32>;

fn round_half_even_byte(value : f32) -> f32 {
    if ((value - value) != 0.0 || !(value > 0.0)) {
        return 0.0;
    }
    if (value >= 255.0) {
        return 255.0;
    }
    let base = floor(value);
    let fraction = value - base;
    if (fraction > 0.5) {
        return base + 1.0;
    }
    if (fraction < 0.5) {
        return base;
    }
    let half = base * 0.5;
    if (floor(half) == half) {
        return base;
    }
    return base + 1.0;
}

// Same byte as `reference_ramp_index` in colour.rs. `inverted` is voi.invert.
fn reference_ramp_byte(display : f32, inverted : bool) -> i32 {
    if ((display - display) != 0.0) {
        return 0;
    }
    var rising = display;
    if (inverted) {
        rising = 255.0 - display;
    }
    if ((rising - rising) != 0.0 || !(rising > 0.0)) {
        if (inverted) {
            return 255;
        }
        return 0;
    }
    if (rising >= 255.0) {
        if (inverted) {
            return 0;
        }
        return 255;
    }
    let width = 1024.0;
    let edge = 0.5 / width;
    var coord = rising / 255.0;
    if (coord < edge) {
        coord = edge;
    } else if (coord > 1.0 - edge) {
        coord = 1.0 - edge;
    }
    let bin = floor(coord * width) / 1023.0;
    var sample = bin;
    if (inverted) {
        sample = 1.0 - bin;
    }
    return i32(round_half_even_byte(sample * 255.0));
}

fn present_display(display : f32) -> vec4<f32> {
    if (colour.mode == 1u) {
        let linear = voi.fn_kind == 0u || voi.fn_kind == 1u;
        let unit_range = voi.ymin == 0.0 && voi.ymax == 255.0;
        let ramp = colour.pad == 1u && linear && unit_range;
        let index = select(
            table_index(display, colour.last),
            reference_ramp_byte(display, voi.invert != 0u),
            ramp
        );
        return textureLoad(colour_lut, vec2<i32>(index, 0), 0);
    }
    let range = voi.ymax - voi.ymin;
    let unit = select(0.0, (display - voi.ymin) / range, range > 0.0);
    return vec4<f32>(unit, unit, unit, 1.0);
}

fn palette_at(at : vec2<i32>) -> vec4<f32> {
    let stored = textureLoad(stored_tex, at, 0).r;
    let index = table_index(stored - f32(colour.origin), colour.last);
    return textureLoad(colour_lut, vec2<i32>(index, 0), 0);
}

@vertex
fn vs_main(@builtin(vertex_index) index : u32) -> @builtin(position) vec4<f32> {
    let u = f32((index << 1u) & 2u);
    let v = f32(index & 2u);
    return vec4<f32>(u * 2.0 - 1.0, v * 2.0 - 1.0, 0.0, 1.0);
}

@fragment
fn fs_main(@builtin(position) at : vec4<f32>) -> @location(0) vec4<f32> {
    let scale = camera.scale.x;
    let css_x = select(at.x, at.x / scale, scale > 0.0);
    let css_y = select(at.y, at.y / scale, scale > 0.0);
    let mapped = camera.css * vec4<f32>(css_x, css_y, 0.0, 1.0);
    let dims = textureDimensions(stored_tex);
    // A negative coordinate has to be rejected before `i32` truncation.
    // Truncation toward zero would otherwise select texel 0.
    if (mapped.x < 0.0 || mapped.y < 0.0 || mapped.x >= f32(dims.x) || mapped.y >= f32(dims.y)) {
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }
    let lane = camera.scale.y;
    if (lane != 0.0 && lane != 1.0) {
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }
    let mode = u32(camera.scale.y);
    if (mode == 1u) {
        // Image space stores the corner. The blend origin is the texel centre.
        let sx = mapped.x - 0.5;
        let sy = mapped.y - 0.5;
        if (sx < 0.0 || sy < 0.0) {
            return vec4<f32>(0.0, 0.0, 0.0, 1.0);
        }
        let x0 = floor(sx);
        let y0 = floor(sy);
        let tx = sx - x0;
        let ty = sy - y0;
        let need_x = tx != 0.0;
        let need_y = ty != 0.0;
        if ((need_x && x0 + 1.0 >= f32(dims.x)) || (need_y && y0 + 1.0 >= f32(dims.y))) {
            return vec4<f32>(0.0, 0.0, 0.0, 1.0);
        }
        if (colour.mode == 2u) {
            let c00 = palette_at(vec2<i32>(i32(x0), i32(y0)));
            var c10 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
            var c01 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
            var c11 = vec4<f32>(0.0, 0.0, 0.0, 0.0);
            if (need_x) {
                c10 = palette_at(vec2<i32>(i32(x0 + 1.0), i32(y0)));
            }
            if (need_y) {
                c01 = palette_at(vec2<i32>(i32(x0), i32(y0 + 1.0)));
            }
            if (need_x && need_y) {
                c11 = palette_at(vec2<i32>(i32(x0 + 1.0), i32(y0 + 1.0)));
            }
            return (1.0 - tx) * (1.0 - ty) * c00 + tx * (1.0 - ty) * c10 + (1.0 - tx) * ty * c01 + tx * ty * c11;
        }
        let p00 = lut_chain(textureLoad(stored_tex, vec2<i32>(i32(x0), i32(y0)), 0).r);
        var p10 = 0.0;
        var p01 = 0.0;
        var p11 = 0.0;
        if (need_x) {
            p10 = lut_chain(textureLoad(stored_tex, vec2<i32>(i32(x0 + 1.0), i32(y0)), 0).r);
        }
        if (need_y) {
            p01 = lut_chain(textureLoad(stored_tex, vec2<i32>(i32(x0), i32(y0 + 1.0)), 0).r);
        }
        if (need_x && need_y) {
            p11 = lut_chain(textureLoad(stored_tex, vec2<i32>(i32(x0 + 1.0), i32(y0 + 1.0)), 0).r);
        }
        let display = (1.0 - tx) * (1.0 - ty) * p00 + tx * (1.0 - ty) * p10 + (1.0 - tx) * ty * p01 + tx * ty * p11;
        return present_display(display);
    }
    let texel = vec2<i32>(i32(mapped.x), i32(mapped.y));
    let stored = textureLoad(stored_tex, texel, 0).r;
    if (colour.mode == 2u) {
        let index = table_index(stored - f32(colour.origin), colour.last);
        return textureLoad(colour_lut, vec2<i32>(index, 0), 0);
    }
    // `present_display` normalises into the colour target, or indexes a
    // preset. A LINEAR body outside `[ymin, ymax]` stays outside it.
    return present_display(lut_chain(stored));
}
";

/// Same address as `display_index` in `colour.rs`. The value is not modified.
///
/// A non-finite value uses texel 0. Values at or below 0 use texel 0.
/// Values at or above `last` use texel `last`. Anything between uses the
/// nearest integer, ties up. `i32` truncates toward zero, so the half is added
/// before the conversion.
pub(crate) const TABLE_INDEX_WGSL: &str = r"
fn table_index(value : f32, last : u32) -> i32 {
    if ((value - value) != 0.0) {
        return 0;
    }
    if (!(value > 0.0)) {
        return 0;
    }
    let last_f = f32(last);
    if (value >= last_f) {
        return i32(last);
    }
    let rounded = value + 0.5;
    if (rounded >= last_f) {
        return i32(last);
    }
    return i32(rounded);
}
";

/// The fragment module tier B compiles, and the module tier A compiles too.
///
/// `VOI_WGSL` is included unchanged. The three window functions are not
/// re-typed here. The entry point samples a float texture with `textureLoad`
/// and has no compute entry and no storage buffer.
pub fn tier_b_fragment_module() -> String {
    format!("{VOI_WGSL}{TABLE_INDEX_WGSL}{GRAPH_WGSL}")
}

/// The volume ray module. `VOI_WGSL` and `table_index` are included once, then the ray entry.
const RAYCAST_WGSL: &str = include_str!("../shaders/raycast.wgsl");

/// Fragment module for the composite volume ray on tier A and tier B.
///
/// `slots` is the number of `texture_3d<u32>` bindings. The residency
/// texture is one more sampled binding, so a ready set of `slots + 1`
/// bricks does not fit. The window functions stay the F-041 text. The ray
/// entry has no compute and no storage buffer.
pub fn volume_ray_module(slots: u32) -> String {
    let array_slots = slots.max(1);
    let mut bindings = String::new();
    for index in 0..slots {
        bindings.push_str(&format!(
            "@group(1) @binding({index}) var brick_{index} : texture_3d<u32>;\n"
        ));
    }
    let residency = slots;
    bindings.push_str(&format!(
        "@group(1) @binding({residency}) var resident_tex : texture_2d<u32>;\n"
    ));
    let mut cases = String::new();
    for index in 0..slots {
        cases.push_str(&format!(
            "case {index}u: {{ return textureLoad(brick_{index}, coord, 0).r; }}\n        "
        ));
    }
    format!("{VOI_WGSL}{TABLE_INDEX_WGSL}{RAYCAST_WGSL}")
        .replace("__RAY_SLOTS__", &array_slots.to_string())
        .replace("__BRICK_BINDINGS__", &bindings)
        .replace("__BRICK_CASES__", &cases)
}

const _: () = assert!(std::mem::size_of::<RayHeader>() == 48);

/// Bytes of the ray uniform, header then one `vec4<u32>` per compiled slot.
fn ray_uniform_bytes(header: &RayHeader, coords: &[[u32; 3]], slots: u32) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(bytemuck::bytes_of(header));
    let array_len = usize::try_from(slots.max(1)).unwrap_or(1);
    for index in 0..array_len {
        let coord = coords.get(index).copied().unwrap_or([0, 0, 0]);
        bytes.extend_from_slice(&coord[0].to_le_bytes());
        bytes.extend_from_slice(&coord[1].to_le_bytes());
        bytes.extend_from_slice(&coord[2].to_le_bytes());
        bytes.extend_from_slice(&0u32.to_le_bytes());
    }
    bytes
}

/// True extent of one brick clipped to `dims`. `None` when the brick starts
/// outside the volume.
fn clipped_axis(dim: u32, brick: u32, edge: u32) -> Option<u32> {
    let origin = brick.checked_mul(edge)?;
    if origin >= dim {
        return None;
    }
    Some((dim - origin).min(edge))
}

fn clipped_brick(dims: [u32; 3], coord: [u32; 3], edge: u32) -> Option<[u32; 3]> {
    Some([
        clipped_axis(dims[0], coord[0], edge)?,
        clipped_axis(dims[1], coord[1], edge)?,
        clipped_axis(dims[2], coord[2], edge)?,
    ])
}

/// Brick textures that fit beside the residency texture.
///
/// The sampled-texture limit and the bind-group limit both have to hold
/// the bricks and that one residency texture. A smaller ready set is not
/// substituted when the count does not fit.
fn brick_slot_count(limits: &wgpu::Limits) -> u32 {
    let sampled = limits.max_sampled_textures_per_shader_stage;
    let group = limits.max_bindings_per_bind_group;
    sampled.saturating_sub(1).min(group.saturating_sub(1))
}

fn clip_error(error: crate::clip::ClipError) -> GraphError {
    match error {
        crate::clip::ClipError::Count => GraphError::ClipCount,
        crate::clip::ClipError::NotFinite => GraphError::ClipNotFinite,
        crate::clip::ClipError::ZeroNormal => GraphError::ClipZeroNormal,
    }
}

/// Whether a volume ray may be bound on `tier`.
///
/// Tier C returns [`GraphError::TierCUnavailable`] and produces no image.
/// A sampled-texture limit of zero, a ready count of zero, or a ready count
/// that leaves no binding for the residency texture, returns
/// [`GraphError::SampledTextureLimit`]. Nothing is drawn in either case.
/// A ready count that fits is not reduced to a smaller set.
///
/// # Errors
///
/// [`GraphError::TierCUnavailable`] or [`GraphError::SampledTextureLimit`].
pub fn volume_ray_admission(
    tier: Tier,
    ready_bricks: u32,
    sampled_limit: u32,
) -> Result<(), GraphError> {
    if tier == Tier::Cpu {
        return Err(GraphError::TierCUnavailable);
    }
    let room = sampled_limit.saturating_sub(1);
    if sampled_limit == 0 || ready_bricks == 0 || ready_bricks > room {
        return Err(GraphError::SampledTextureLimit);
    }
    Ok(())
}

/// Sampled textures the adapter allows in one shader stage.
pub fn sampled_texture_limit(adapter: &ResolvedAdapter) -> u32 {
    adapter
        .wgpu_adapter()
        .limits()
        .max_sampled_textures_per_shader_stage
}

/// Colour table bytes remembered across a device rebuild.
///
/// Restoring the texture does not increment `lut_uploads`. That counter
/// counts a selection.
#[derive(Clone)]
struct HeldColour {
    params: ColourParams,
    bytes: Vec<u8>,
}

/// One registered viewport's decision state.
struct SlotState {
    dirty: Dirty,
    pass: Pass,
    /// Which representation a segmentation overlay shows. Other passes keep
    /// the labelmap discriminant and the setter refuses them.
    representation: ocelli_seg::Representation,
    /// Extent of the owned stored texture, remembered across a device rebuild.
    stored: Option<Upload>,
    /// The preserving format chosen for that texture. Absent when the
    /// viewport has no owned stored texture.
    stored_format: Option<wgpu::TextureFormat>,
    /// The colour table selected for this viewport. Absent until a selection.
    colour: Option<HeldColour>,
    /// Column-major CSS camera. Identity until `set_view`.
    camera: [f32; 16],
    /// Opacity table for a volume ray. Flat until `set_opacity`.
    opacity: OpacityTable,
    /// Shade weights for a volume ray. Unlit until `set_shade`.
    shade: ShadeWeights,
    /// Clip planes for a volume ray. Empty until `set_clip_planes`.
    clip: ClipList,
    /// Patient frame copied by a successful `bind_volume_ray`.
    frame: PatientFrame,
    /// Device pixels per CSS pixel uploaded beside the CSS camera.
    dpr: f64,
    /// Stack sample mode uploaded in `camera.scale.y`. Nearest until set.
    interpolation: Interpolation,
}

/// The stored-sample texture one viewport owns.
///
/// The format is `R32Float` when the adapter can sample it. On tier B a
/// wider float format is used only when that is the format that still
/// returns the stored `f32` from `textureLoad`. `Rgba8Unorm` is never stored
/// here.
struct StoredTexture {
    texture: wgpu::Texture,
    width: u32,
    height: u32,
    format: wgpu::TextureFormat,
}

/// One registered viewport's GPU resources, created at registration.
struct GpuSlot {
    target: wgpu::TextureView,
    /// The stored-sample view bound at group 1. Rebuilt when either texture
    /// object changes. Caller-owned views have no texture beside this one.
    source: wgpu::TextureView,
    uniform: wgpu::Buffer,
    colour_uniform: wgpu::Buffer,
    camera: wgpu::Buffer,
    colour_texture: wgpu::Texture,
    bind_uniform: wgpu::BindGroup,
    bind_source: wgpu::BindGroup,
    stored: Option<StoredTexture>,
    /// Brick view, residency and ray uniforms. Absent until [`RenderGraph::bind_volume_ray`].
    ray: Option<VolumeRayBinding>,
    /// Slab value texture, mask and bind groups. Absent until [`RenderGraph::show_slab`].
    slab: Option<SlabBinding>,
    /// Presented labelmap. Absent until [`RenderGraph::show_labelmap`].
    labelmap: Option<crate::labelmap::LabelmapBinding>,
}

/// Slab bind groups for one viewport. Created at `show_slab`, not inside a tick.
///
/// The groups own the texture views, and those views own the textures.
struct SlabBinding {
    bind_uniform: wgpu::BindGroup,
    bind_source: wgpu::BindGroup,
    stamp: SlabStamp,
}

/// Identity of the last accepted slab. Equal args do not allocate again.
#[derive(Clone, PartialEq, Eq)]
struct SlabStamp {
    width: u32,
    height: u32,
    sample_count: u32,
    step_bits: u64,
    index_bits: [u64; 3],
    samples: Vec<SlabSampleStamp>,
    blend: BlendMode,
}

impl SlabStamp {
    fn same_pixels(&self, other: &Self) -> bool {
        self.width == other.width
            && self.height == other.height
            && self.sample_count == other.sample_count
            && self.step_bits == other.step_bits
            && self.index_bits == other.index_bits
            && self.samples == other.samples
    }
}

#[derive(Clone, PartialEq, Eq)]
enum SlabSampleStamp {
    Present(u32),
    Absent,
}

/// One ready brick already uploaded for a volume ray.
pub struct VolumeBrick<'a> {
    /// The key `upload_brick` stored.
    pub key: &'a BrickKey,
    /// Brick coordinate, x then y then z.
    pub coord: [u32; 3],
}

/// What [`RenderGraph::bind_volume_ray`] uploads for one viewport.
///
/// `bricks` is the set this call binds. The function does not compare that
/// slice with `Volume::ready_bricks`. A shorter slice is bound as given
/// when it fits the sampled-texture room and the compiled slot count.
pub struct VolumeRaySource<'a> {
    /// The bricks to bind, in any order. The length is the ready count.
    pub bricks: &'a [VolumeBrick<'a>],
    /// Brick edge in voxels. Eight-bit volumes use 128.
    pub edge: u32,
    /// Voxel dimensions, x then y then z.
    pub dims: [u32; 3],
    /// One flag per step of the view axis. `1` is resident.
    pub flags: &'a [u32],
    /// Sampled textures the shader stage may bind, residency included.
    pub sampled_limit: u32,
    /// Millimetres per voxel, x then y then z. Each axis must be finite and
    /// greater than zero. The central difference divides by this spacing.
    pub spacing_mm: [f32; 3],
    /// Patient millimetres of the centre of voxel `(0, 0, 0)`. Each component
    /// must be finite and narrow to a finite `f32`.
    pub origin_mm: [f64; 3],
    /// Direction columns, the same order as `Volume::direction.col`. Column 0
    /// scales with `spacing_mm[0]` and the x index.
    pub direction: [[f64; 3]; 3],
}

/// Ray resources for one volume viewport. Created at bind, not inside a tick.
struct VolumeRayBinding {
    capacity: u32,
    residency: wgpu::Texture,
    params: wgpu::Buffer,
    /// 1024 bytes. Created at bind. `set_opacity` writes it and does not
    /// replace it.
    opacity: wgpu::Buffer,
    /// 16 bytes. Created at bind. `set_shade` writes it and does not
    /// replace it.
    shade: wgpu::Buffer,
    /// 256 bytes. Created at bind. `set_clip_planes` writes it and does not
    /// replace it.
    clip: wgpu::Buffer,
    bind_uniform: wgpu::BindGroup,
    bind_source: wgpu::BindGroup,
    /// Bricks whose views this binding samples. A later upload of one of
    /// these keys replaces the texture and drops the binding.
    keys: Vec<BrickKey>,
}

/// Fixed prefix of the ray uniform. Field order matches `raycast.wgsl`.
///
/// The brick-coordinate array follows these 48 bytes, one `vec4<u32>` per
/// compiled slot. `edge` is the brick edge. `brick_count` is the ready set.
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct RayHeader {
    axis: u32,
    from_far: u32,
    dim_x: u32,
    dim_y: u32,
    dim_z: u32,
    steps: u32,
    kind: u32,
    edge: u32,
    brick_count: u32,
    /// Millimetres between columns. The last three words stay inside the
    /// 48-byte prefix. They are not shade weights.
    spacing_x: f32,
    /// Millimetres between rows.
    spacing_y: f32,
    /// Millimetres between slices.
    spacing_z: f32,
}

/// The pure registration and dirty registry, sized at creation.
struct Registry {
    states: Vec<Option<SlotState>>,
}

impl Registry {
    fn new(capacity: usize) -> Self {
        let mut states = Vec::new();
        states.resize_with(capacity, || None);
        Self { states }
    }

    fn register(&mut self, pass: Pass) -> Result<ViewportId, GraphError> {
        for (index, slot) in self.states.iter_mut().enumerate() {
            if slot.is_none() {
                *slot = Some(SlotState {
                    dirty: Dirty::full(),
                    pass,
                    representation: ocelli_seg::Representation::Labelmap,
                    stored: None,
                    stored_format: None,
                    colour: None,
                    camera: IDENTITY_CAMERA,
                    dpr: 1.0,
                    interpolation: Interpolation::Nearest,
                    opacity: OpacityTable::preset(OpacityPreset::Flat),
                    shade: ShadeWeights::unlit(),
                    clip: ClipList::empty(),
                    frame: PatientFrame::identity(),
                });
                return Ok(ViewportId(
                    u32::try_from(index).map_err(|_| GraphError::CapacityExhausted)?,
                ));
            }
        }
        Err(GraphError::CapacityExhausted)
    }

    fn state_mut(&mut self, id: ViewportId) -> Option<&mut SlotState> {
        self.states
            .get_mut(usize::try_from(id.0).unwrap_or(usize::MAX))
            .and_then(Option::as_mut)
    }

    fn remove(&mut self, id: ViewportId) -> bool {
        match self
            .states
            .get_mut(usize::try_from(id.0).unwrap_or(usize::MAX))
        {
            Some(slot) => slot.take().is_some(),
            None => false,
        }
    }
}

/// Layouts and pipelines `compile` builds together.
struct CompiledGraph {
    uniform_layout: wgpu::BindGroupLayout,
    source_layout: wgpu::BindGroupLayout,
    volume_uniform_layout: wgpu::BindGroupLayout,
    volume_source_layout: wgpu::BindGroupLayout,
    slab_uniform_layout: wgpu::BindGroupLayout,
    slab_source_layout: wgpu::BindGroupLayout,
    labelmap_layout: wgpu::BindGroupLayout,
    /// `texture_3d` bindings compiled into the volume shader.
    brick_slots: u32,
    /// 1×1×1 stand-in for a brick slot the ready set does not use.
    ray_dummy: wgpu::Texture,
    keys: Vec<CompiledKey>,
    pipelines: Vec<wgpu::RenderPipeline>,
}

fn fragment_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    module: &wgpu::ShaderModule,
    format: wgpu::TextureFormat,
    label: &str,
    constants: &[(&str, f64)],
) -> wgpu::RenderPipeline {
    let stage = wgpu::PipelineCompilationOptions {
        constants,
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
            entry_point: Some("fs_main"),
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

/// One compiled pipeline: section 22's key plus the shader family.
///
/// The family is not a new key field. It is how a compute request is kept
/// from matching the fragment body that shares the pass, blend and tier.
struct CompiledKey {
    key: PipelineKey,
    family: PipelineFamily,
}

/// Find a compiled pipeline by its full key, tier included, and its family.
///
/// A free function over the key list so the lookup rule is testable with no
/// adapter. Dropping `tier` from the comparison returns another tier's
/// pipeline, and dropping `family` returns the fragment pipeline for a
/// compute request. The unit tests beside this function go red on either.
fn pipeline_index(
    keys: &[CompiledKey],
    key: &PipelineKey,
    family: PipelineFamily,
) -> Option<usize> {
    keys.iter()
        .position(|held| held.key == *key && held.family == family)
}

/// A surface bound to one viewport. The colour attachment is acquired from
/// it inside `tick` and is not stored on [`GpuSlot`].
struct AttachedSurface {
    surface: wgpu::Surface<'static>,
    width: u32,
    height: u32,
}

/// The render graph: registered viewports, their dirt, and the pipelines
/// compiled at creation.
pub struct RenderGraph {
    registry: Registry,
    gpu: Vec<Option<GpuSlot>>,
    /// Parallel to `gpu`. `None` keeps F-038's caller-supplied colour view.
    surfaces: Vec<Option<AttachedSurface>>,
    /// Surface textures acquired for this frame, presented after the submit.
    /// Capacity is fixed at creation so a tick does not grow it.
    frame_textures: Vec<Option<wgpu::SurfaceTexture>>,
    keys: Vec<CompiledKey>,
    pipelines: Vec<wgpu::RenderPipeline>,
    uniform_layout: wgpu::BindGroupLayout,
    source_layout: wgpu::BindGroupLayout,
    volume_uniform_layout: wgpu::BindGroupLayout,
    volume_source_layout: wgpu::BindGroupLayout,
    slab_uniform_layout: wgpu::BindGroupLayout,
    slab_source_layout: wgpu::BindGroupLayout,
    labelmap_layout: wgpu::BindGroupLayout,
    /// Brick bindings the volume shader was compiled with.
    brick_slots: u32,
    /// Kept alive so unused brick slots in a bind group stay valid.
    ray_dummy: wgpu::Texture,
    format: wgpu::TextureFormat,
    tier: Tier,
    submissions: u64,
    /// Successful `upload_stored` calls. `set_voi` does not touch this.
    uploads: u64,
    /// Successful `select_colour_lut` calls. `set_voi` does not touch this.
    lut_uploads: u64,
    /// One 3D texture per uploaded brick. `set_voi` does not touch this.
    bricks: BrickResidency,
    /// Bytes last accepted for each brick. Cleared with the textures.
    /// A missing key is not a texture of zeros.
    brick_stamps: BTreeMap<BrickKey, Vec<u8>>,
    /// Fusion resources, compiled when the fusion view is bound.
    ///
    /// This is not a [`Pass`]. A viewport with no fusion binding keeps its
    /// stack pipeline. The tick encodes this view in the same encoder.
    fusion: Option<FusionGpu>,
}

/// Blend compiled for the stack and the volume ray.
/// The slab pass also compiles MIP, MinIP and average.
const COMPILED_BLEND: BlendMode = BlendMode::Composite;

impl RenderGraph {
    /// Whether this story compiles a fragment pipeline for a key.
    ///
    /// Composite stack, composite volume and the segmentation overlay each
    /// have a body. The overlay compiles composite and labelmap edge
    /// projection on the same copy shader. The slab pass compiles composite,
    /// MIP, MinIP and average. Volume MIP, stack MIP, and labelmap edge on
    /// slab, volume or stack stay uncompiled. A compute or storage request
    /// is [`RenderGraph::pipeline_family_compiled`], which is false on tier B
    /// and is not this predicate.
    pub fn pipeline_compiled(key: &PipelineKey) -> bool {
        match key.pass {
            PassKind::Stack | PassKind::VolumeRaycast => key.blend == COMPILED_BLEND,
            PassKind::SegOverlay => {
                key.blend == COMPILED_BLEND || key.blend == BlendMode::LabelmapEdgeProjection
            }
            PassKind::Slab => slab::slab_blend_code(key.blend).is_some(),
        }
    }

    /// Whether `family` has a compiled body for `key`.
    ///
    /// Fragment is a compiled stack body, a compiled volume ray, a compiled
    /// overlay copy (composite or labelmap edge), or one of the four slab
    /// bodies. Compute and storage have no body in this story,
    /// on either GPU tier.
    /// On tier B that refusal is the section 7 rule: the fragment pipeline
    /// is not returned in their place.
    pub fn pipeline_family_compiled(key: &PipelineKey, family: PipelineFamily) -> bool {
        match family {
            PipelineFamily::Fragment => Self::pipeline_compiled(key),
            PipelineFamily::Compute | PipelineFamily::Storage => false,
        }
    }

    /// The frame outcome of asking for `family` on `key` when no viewport
    /// has been encoded yet.
    ///
    /// A compute or storage family is [`FrameOutcome::Unavailable`]. It is
    /// not a submitted frame that bound the fragment pipeline. `tick` uses
    /// the same family check before it binds anything.
    pub fn request_outcome(
        key: &PipelineKey,
        family: PipelineFamily,
        viewport: ViewportId,
    ) -> FrameOutcome {
        if Self::pipeline_family_compiled(key, family) {
            FrameOutcome::Noop
        } else {
            FrameOutcome::Unavailable { viewport }
        }
    }

    /// How many frame submissions this graph has issued.
    ///
    /// One per submitted tick, across all viewports. This is the observable
    /// half of section 22's one-submit rule: a tick that submitted per
    /// viewport would count its viewports instead of one.
    pub fn submissions(&self) -> u64 {
        self.submissions
    }

    /// How many stored-sample uploads have been written.
    ///
    /// One per successful [`RenderGraph::upload_stored`]. [`RenderGraph::set_voi`]
    /// does not increment it, which is the observable half of "a window change
    /// is thirty-two bytes, not a texture upload".
    pub fn uploads(&self) -> u64 {
        self.uploads
    }

    /// How many colour LUT uploads have been written.
    ///
    /// One per successful [`RenderGraph::select_colour_lut`].
    /// [`RenderGraph::set_voi`] does not increment it, and neither does a
    /// restore after device loss.
    pub fn lut_uploads(&self) -> u64 {
        self.lut_uploads
    }

    /// How many brick uploads have been written.
    ///
    /// One per successful [`RenderGraph::upload_brick`]. A refusal, a window
    /// change and a device-loss restore do not increment it.
    pub fn brick_uploads(&self) -> u64 {
        self.bricks.uploads()
    }

    /// Dirt flags for a registered viewport.
    ///
    /// # Errors
    ///
    /// [`GraphError::UnknownViewport`] for an id this graph never issued.
    pub fn dirty(&self, id: ViewportId) -> Result<Dirty, GraphError> {
        self.slot_state(id).map(|state| state.dirty)
    }

    /// The sixteen camera floats last stored for this viewport.
    ///
    /// Registration stores the identity. [`RenderGraph::set_view`] replaces
    /// them. The shader reads the same bytes from group 0 binding 2.
    ///
    /// # Errors
    ///
    /// [`GraphError::UnknownViewport`] for an id this graph never issued.
    pub fn camera(&self, id: ViewportId) -> Result<[f32; 16], GraphError> {
        self.slot_state(id).map(|state| state.camera)
    }

    /// The VOI uniform a stack viewport will draw with.
    ///
    /// # Errors
    ///
    /// [`GraphError::UnknownViewport`] for an id this graph never issued.
    /// [`GraphError::PassHasNoUniform`] for a pass with no VOI uniform.
    pub fn voi_params(&self, id: ViewportId) -> Result<VoiParams, GraphError> {
        let state = self.slot_state(id)?;
        if !matches!(
            state.pass,
            Pass::Stack(_) | Pass::VolumeRaycast(_) | Pass::Slab(_)
        ) {
            return Err(GraphError::PassHasNoUniform);
        }
        Ok(voi_of(&state.pass))
    }

    fn slot_state(&self, id: ViewportId) -> Result<&SlotState, GraphError> {
        self.registry
            .states
            .get(usize::try_from(id.0).unwrap_or(usize::MAX))
            .and_then(Option::as_ref)
            .ok_or(GraphError::UnknownViewport)
    }

    /// The 3D texture held for `key`, when an upload has been accepted and
    /// not dropped by a restore.
    pub fn brick_texture(&self, key: &BrickKey) -> Option<&wgpu::Texture> {
        self.bricks.texture(key)
    }

    /// The texture size of `key`, which is the brick extent that was uploaded.
    pub fn brick_extent(&self, key: &BrickKey) -> Option<[u32; 3]> {
        self.bricks.extent_of(key)
    }

    /// The colour format the pipelines were compiled for.
    ///
    /// A surface whose first renderable format differs from this is refused
    /// rather than drawn with the wrong attachment format.
    pub fn color_format(&self) -> wgpu::TextureFormat {
        self.format
    }

    /// Create a graph over a live context, compiling its pipelines before it
    /// returns, and register them for device-loss recovery.
    ///
    /// # Errors
    ///
    /// [`GraphError::TierCUnavailable`] when the context reports tier C,
    /// which holds no device under deviation D-07 and so has no graph.
    pub fn new(
        ctx: &mut GpuContext,
        capacity: usize,
        format: wgpu::TextureFormat,
    ) -> Result<Self, GraphError> {
        let tier = ctx.caps().tier;
        if tier == Tier::Cpu {
            return Err(GraphError::TierCUnavailable);
        }
        let compiled = Self::compile(ctx, tier, format);
        let restored = u32::try_from(compiled.pipelines.len()).unwrap_or(0);
        ctx.register_graph_resources(restored);
        let mut gpu = Vec::new();
        gpu.resize_with(capacity, || None);
        let mut surfaces = Vec::new();
        surfaces.resize_with(capacity, || None);
        let mut frame_textures = Vec::new();
        frame_textures.resize_with(capacity, || None);
        Ok(Self {
            registry: Registry::new(capacity),
            gpu,
            surfaces,
            frame_textures,
            keys: compiled.keys,
            pipelines: compiled.pipelines,
            uniform_layout: compiled.uniform_layout,
            source_layout: compiled.source_layout,
            volume_uniform_layout: compiled.volume_uniform_layout,
            volume_source_layout: compiled.volume_source_layout,
            slab_uniform_layout: compiled.slab_uniform_layout,
            slab_source_layout: compiled.slab_source_layout,
            labelmap_layout: compiled.labelmap_layout,
            brick_slots: compiled.brick_slots,
            ray_dummy: compiled.ray_dummy,
            format,
            tier,
            submissions: 0,
            uploads: 0,
            lut_uploads: 0,
            bricks: BrickResidency::new(),
            brick_stamps: BTreeMap::new(),
            fusion: None,
        })
    }

    fn compile(ctx: &GpuContext, tier: Tier, format: wgpu::TextureFormat) -> CompiledGraph {
        let device = ctx.device();
        // Tier B's compiled module is this fragment text. Tier A compiles the
        // same text. A compute module is not built for either tier.
        let source = tier_b_fragment_module();
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ocelli render graph"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        let brick_slots = brick_slot_count(&device.limits());
        let ray_source = volume_ray_module(brick_slots);
        let ray_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ocelli volume ray"),
            source: wgpu::ShaderSource::Wgsl(ray_source.into()),
        });
        let ray_dummy = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("ocelli ray empty brick"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D3,
            format: wgpu::TextureFormat::R8Uint,
            usage: wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let uniform_binding = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        };
        let uniform_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ocelli graph uniform"),
            entries: &[uniform_binding(0), uniform_binding(1), uniform_binding(2)],
        });
        // The ray keeps the VOI uniform at binding 0, ray params at binding 1,
        // the 256-entry opacity table at binding 2, the shade weights at
        // binding 3 and the clip planes at binding 4. It does not take the
        // colour table or the camera. The stack layout stays three bindings.
        let volume_uniform_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("ocelli volume ray uniform"),
                entries: &[
                    uniform_binding(0),
                    uniform_binding(1),
                    uniform_binding(2),
                    uniform_binding(3),
                    uniform_binding(4),
                ],
            });
        // `textureLoad` has no sampler. A non-filterable float binding accepts
        // the stored float formats and `Rgba8Unorm`.
        let texture_binding = |binding| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: false },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        };
        let source_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ocelli graph source"),
            entries: &[texture_binding(0), texture_binding(1)],
        });
        let uint_texture = |binding, view_dimension| wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Uint,
                view_dimension,
                multisampled: false,
            },
            count: None,
        };
        let mut volume_entries = Vec::new();
        for binding in 0..brick_slots {
            volume_entries.push(uint_texture(binding, wgpu::TextureViewDimension::D3));
        }
        volume_entries.push(uint_texture(brick_slots, wgpu::TextureViewDimension::D2));
        let volume_source_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("ocelli volume ray source"),
                entries: &volume_entries,
            });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ocelli render graph"),
            bind_group_layouts: &[Some(&uniform_layout), Some(&source_layout)],
            immediate_size: 0,
        });
        let volume_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ocelli volume ray"),
            bind_group_layouts: &[Some(&volume_uniform_layout), Some(&volume_source_layout)],
            immediate_size: 0,
        });
        let stack_key = CompiledKey {
            key: PipelineKey {
                pass: PassKind::Stack,
                blend: COMPILED_BLEND,
                tier,
            },
            family: PipelineFamily::Fragment,
        };
        let volume_key = CompiledKey {
            key: PipelineKey {
                pass: PassKind::VolumeRaycast,
                blend: COMPILED_BLEND,
                tier,
            },
            family: PipelineFamily::Fragment,
        };
        let pipeline = fragment_pipeline(
            device,
            &layout,
            &module,
            format,
            "ocelli render graph composite",
            &[],
        );
        let ray_pipeline = fragment_pipeline(
            device,
            &volume_layout,
            &ray_module,
            format,
            "ocelli volume ray composite",
            &[],
        );
        let slab_source = slab::slab_module();
        let slab_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ocelli slab"),
            source: wgpu::ShaderSource::Wgsl(slab_source.into()),
        });
        let slab_uniform_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("ocelli slab uniform"),
                entries: &[uniform_binding(0)],
            });
        let slab_value = wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: false },
                view_dimension: wgpu::TextureViewDimension::D3,
                multisampled: false,
            },
            count: None,
        };
        let slab_source_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("ocelli slab source"),
                entries: &[slab_value, texture_binding(1)],
            });
        let slab_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ocelli slab"),
            bind_group_layouts: &[Some(&slab_uniform_layout), Some(&slab_source_layout)],
            immediate_size: 0,
        });
        let mut keys = vec![stack_key, volume_key];
        let mut pipelines = vec![pipeline, ray_pipeline];
        for (key, code) in slab::slab_compile_list(tier) {
            let label = match key.blend {
                BlendMode::Composite => "ocelli slab composite",
                BlendMode::Mip => "ocelli slab mip",
                BlendMode::MinIp => "ocelli slab minip",
                BlendMode::Average => "ocelli slab average",
                BlendMode::LabelmapEdgeProjection => "ocelli slab labelmap",
            };
            let constants = [("slab_blend", f64::from(code))];
            keys.push(CompiledKey {
                key,
                family: PipelineFamily::Fragment,
            });
            pipelines.push(fragment_pipeline(
                device,
                &slab_layout,
                &slab_module,
                format,
                label,
                &constants,
            ));
        }
        let labelmap_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ocelli labelmap"),
            entries: &[texture_binding(0)],
        });
        let labelmap_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("ocelli labelmap"),
                bind_group_layouts: &[Some(&labelmap_layout)],
                immediate_size: 0,
            });
        let labelmap_module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ocelli labelmap"),
            source: wgpu::ShaderSource::Wgsl(crate::labelmap::LABELMAP_WGSL.into()),
        });
        keys.push(CompiledKey {
            key: PipelineKey {
                pass: PassKind::SegOverlay,
                blend: COMPILED_BLEND,
                tier,
            },
            family: PipelineFamily::Fragment,
        });
        pipelines.push(fragment_pipeline(
            device,
            &labelmap_pipeline_layout,
            &labelmap_module,
            format,
            "ocelli labelmap composite",
            &[],
        ));
        // The edge key is a second pipeline on the same copy shader.
        // The presented bytes are already outline-only. Slab, volume and
        // stack do not receive this key, so they cannot alias composite.
        keys.push(CompiledKey {
            key: PipelineKey {
                pass: PassKind::SegOverlay,
                blend: BlendMode::LabelmapEdgeProjection,
                tier,
            },
            family: PipelineFamily::Fragment,
        });
        pipelines.push(fragment_pipeline(
            device,
            &labelmap_pipeline_layout,
            &labelmap_module,
            format,
            "ocelli labelmap edge",
            &[],
        ));
        CompiledGraph {
            uniform_layout,
            source_layout,
            volume_uniform_layout,
            volume_source_layout,
            slab_uniform_layout,
            slab_source_layout,
            labelmap_layout,
            brick_slots,
            ray_dummy,
            keys,
            pipelines,
        }
    }

    fn pipeline(&self, key: &PipelineKey, family: PipelineFamily) -> Option<&wgpu::RenderPipeline> {
        pipeline_index(&self.keys, key, family).and_then(|index| self.pipelines.get(index))
    }

    /// Register a viewport rendering `pass` into `target` from `source`.
    ///
    /// Its uniform buffer and bind groups are created here, at registration,
    /// and never inside a frame. It starts fully dirty.
    ///
    /// # Errors
    ///
    /// [`GraphError::CapacityExhausted`] beyond the capacity fixed at
    /// creation, changing no existing slot.
    pub fn register(
        &mut self,
        ctx: &GpuContext,
        target: wgpu::TextureView,
        source: wgpu::TextureView,
        pass: Pass,
    ) -> Result<ViewportId, GraphError> {
        let voi = voi_of(&pass);
        let id = self.registry.register(pass)?;
        let slot = self.create_gpu_slot(
            ctx,
            target,
            source,
            (voi, IDENTITY_CAMERA, 1.0, Interpolation::Nearest),
            None,
            None,
        );
        if let Some(held) = self
            .gpu
            .get_mut(usize::try_from(id.0).unwrap_or(usize::MAX))
        {
            *held = Some(slot);
        }
        Ok(id)
    }

    /// Register `pass` into `target` and create its stored-sample texture.
    ///
    /// The texture is created here, at registration.
    /// [`selected_stored_format`] asks the adapter for `R32Float`, then
    /// `Rg32Float`, then `Rgba32Float`. `R32Float` is used on every tier when
    /// it can be sampled and written. Tier B may use the next float format
    /// when `R32Float` cannot, because `textureLoad` still returns the stored
    /// value as `f32`. Tier A does not substitute. `Rgba8Unorm` is never
    /// chosen. The slot starts fully dirty. Nothing is copied yet, so
    /// [`RenderGraph::uploads`] stays put.
    ///
    /// # Errors
    ///
    /// [`UploadError::FormatUnavailable`] when the format cannot be sampled
    /// and written. [`UploadError::EmptyExtent`] when an axis is zero.
    /// [`UploadError::ExtentUnavailable`] when an axis is longer than the
    /// device `max_texture_dimension_2d`. [`UploadError::ExtentOverflow`]
    /// when the extent does not fit the copy. Nothing is registered in those
    /// cases. [`UploadError::CapacityExhausted`] beyond the capacity fixed at
    /// creation, changing no existing slot.
    pub fn register_stored(
        &mut self,
        ctx: &GpuContext,
        adapter: &ResolvedAdapter,
        target: wgpu::TextureView,
        width: u32,
        height: u32,
        pass: Pass,
    ) -> Result<ViewportId, UploadError> {
        let format = selected_stored_format(adapter.wgpu_adapter(), self.tier)?;
        let upload = Upload::from_extent(
            width,
            height,
            ctx.device().limits().max_texture_dimension_2d,
        )?;
        let voi = voi_of(&pass);
        let id = self
            .registry
            .register(pass)
            .map_err(|_| UploadError::CapacityExhausted)?;
        if let Some(state) = self.registry.state_mut(id) {
            state.stored = Some(upload);
            state.stored_format = Some(format);
        }
        let texture = create_stored_texture(ctx.device(), upload, format);
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let stored = StoredTexture {
            texture,
            width: upload.width(),
            height: upload.height(),
            format,
        };
        let slot = self.create_gpu_slot(
            ctx,
            target,
            view,
            (voi, IDENTITY_CAMERA, 1.0, Interpolation::Nearest),
            Some(stored),
            None,
        );
        if let Some(held) = self
            .gpu
            .get_mut(usize::try_from(id.0).unwrap_or(usize::MAX))
        {
            *held = Some(slot);
        }
        Ok(id)
    }

    fn create_gpu_slot(
        &self,
        ctx: &GpuContext,
        target: wgpu::TextureView,
        source: wgpu::TextureView,
        uniforms: (VoiParams, [f32; 16], f64, Interpolation),
        stored: Option<StoredTexture>,
        colour: Option<HeldColour>,
    ) -> GpuSlot {
        let (voi, camera, dpr, mode) = uniforms;
        let uploaded = device_camera(camera, dpr, mode);
        let device = ctx.device();
        // A table wider than this device is not a texture. Selection already
        // refused it. Restore keeps the unbound path rather than creating one.
        let colour = colour.filter(|held| {
            colour_width(&held.bytes, device.limits().max_texture_dimension_2d).is_some()
        });
        let uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("ocelli graph voi"),
            contents: bytemuck::bytes_of(&voi),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let params = colour
            .as_ref()
            .map(|held| held.params)
            .unwrap_or(ColourParams::NONE);
        let colour_uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("ocelli graph colour"),
            contents: bytemuck::bytes_of(&params),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let camera_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("ocelli graph camera"),
            contents: bytemuck::bytes_of(&uploaded),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let colour_texture = colour_texture_for(device, ctx.queue(), colour.as_ref());
        let colour_view = colour_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_uniform = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ocelli graph voi"),
            layout: &self.uniform_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: colour_uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: camera_buffer.as_entire_binding(),
                },
            ],
        });
        let bind_source = self.bind_source_group(device, &source, &colour_view);
        GpuSlot {
            target,
            source,
            uniform,
            colour_uniform,
            camera: camera_buffer,
            colour_texture,
            bind_uniform,
            bind_source,
            stored,
            ray: None,
            slab: None,
            labelmap: None,
        }
    }

    fn bind_source_group(
        &self,
        device: &wgpu::Device,
        source: &wgpu::TextureView,
        colour: &wgpu::TextureView,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ocelli graph source"),
            layout: &self.source_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(source),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(colour),
                },
            ],
        })
    }

    /// Release a viewport, clearing its slot for re-registration.
    ///
    /// # Errors
    ///
    /// [`GraphError::UnknownViewport`] for an id this graph never issued, or
    /// one already released.
    pub fn remove(&mut self, id: ViewportId) -> Result<(), GraphError> {
        if !self.registry.remove(id) {
            return Err(GraphError::UnknownViewport);
        }
        let index = usize::try_from(id.0).unwrap_or(usize::MAX);
        // The frame texture borrows the surface, so it goes first.
        if let Some(held) = self.frame_textures.get_mut(index) {
            *held = None;
        }
        if let Some(held) = self.surfaces.get_mut(index) {
            *held = None;
        }
        if let Some(slot) = self.gpu.get_mut(index) {
            *slot = None;
        }
        Ok(())
    }

    /// Bind `surface` to a registered viewport.
    ///
    /// `tick` acquires that surface's current texture as the colour attachment
    /// and presents it after the frame's single submit. The view is not stored
    /// on the slot and is not installed through [`RenderGraph::rebind`]: a
    /// surface texture is only valid until it is presented, and `rebind` on a
    /// viewport that owns stored samples returns
    /// [`GraphError::StoredNeedsRebind`] before it would keep a view. A
    /// viewport with no surface keeps the caller-supplied view from F-038.
    /// This does not increment [`RenderGraph::uploads`].
    ///
    /// The caller configures `surface` before this, at `extent`. `tick` calls
    /// `get_current_texture`, which panics on a surface that was never
    /// configured.
    ///
    /// # Errors
    ///
    /// [`GraphError::UnknownViewport`] for an id this graph never issued.
    pub fn bind_surface(
        &mut self,
        id: ViewportId,
        surface: wgpu::Surface<'static>,
        extent: (u32, u32),
    ) -> Result<(), GraphError> {
        let index = usize::try_from(id.0).unwrap_or(usize::MAX);
        if self
            .registry
            .states
            .get(index)
            .and_then(Option::as_ref)
            .is_none()
        {
            return Err(GraphError::UnknownViewport);
        }
        if let Some(held) = self.frame_textures.get_mut(index) {
            *held = None;
        }
        let Some(slot) = self.surfaces.get_mut(index) else {
            return Err(GraphError::UnknownViewport);
        };
        *slot = Some(AttachedSurface {
            surface,
            width: extent.0,
            height: extent.1,
        });
        if let Some(state) = self.registry.state_mut(id) {
            state.dirty.camera = true;
        }
        Ok(())
    }

    /// Reconfigure a bound surface to `extent` and drop its previous texture.
    ///
    /// The held frame texture is dropped before `configure`, because wgpu
    /// panics when a surface texture from this surface is still alive.
    /// [`RenderGraph::rebind`] and [`RenderGraph::rebind_stored`] are not
    /// called, so a stored-sample texture stays where it is and
    /// [`RenderGraph::uploads`] does not change. The next tick draws the new
    /// extent because the camera flag is set here.
    ///
    /// # Errors
    ///
    /// [`SurfaceError::NotAttached`] when this viewport has no surface.
    /// [`SurfaceError::FormatDiffers`] when the format the surface would use
    /// is not the format the graph was compiled for, in which case the
    /// previous configuration is left in place. The other [`SurfaceError`]
    /// variants are those [`GpuContext::configure_viewport`] returns.
    pub fn reconfigure_surface(
        &mut self,
        ctx: &GpuContext,
        adapter: &ResolvedAdapter,
        id: ViewportId,
        extent: (u32, u32),
    ) -> Result<(), SurfaceError> {
        let index = usize::try_from(id.0).unwrap_or(usize::MAX);
        if let Some(held) = self.frame_textures.get_mut(index) {
            *held = None;
        }
        let attached = self
            .surfaces
            .get(index)
            .and_then(Option::as_ref)
            .ok_or(SurfaceError::NotAttached)?;
        let capabilities = attached.surface.get_capabilities(adapter.wgpu_adapter());
        let format = first_render_attachment(&capabilities.formats, |format| {
            adapter
                .wgpu_adapter()
                .get_texture_format_features(format)
                .allowed_usages
        })
        .ok_or(SurfaceError::NoRenderAttachment)?;
        if format != self.format {
            return Err(SurfaceError::FormatDiffers);
        }
        ctx.configure_viewport(&attached.surface, adapter, extent)?;
        if let Some(Some(attached)) = self.surfaces.get_mut(index) {
            attached.width = extent.0;
            attached.height = extent.1;
        }
        if let Some(state) = self.registry.state_mut(id) {
            state.dirty.camera = true;
        }
        Ok(())
    }

    /// Rebind a viewport's GPU objects after a device rebuild.
    ///
    /// `install_recovered` drops every per-viewport object the dead device
    /// owned. The caller re-supplies the target and source views on the new
    /// device, and this recreates the uniform buffer and bind groups from
    /// the pass the registry still holds.
    ///
    /// # Errors
    ///
    /// [`GraphError::UnknownViewport`] for an unregistered id.
    /// [`GraphError::StoredNeedsRebind`] when the viewport owns a stored
    /// texture, which [`RenderGraph::rebind_stored`] recreates.
    pub fn rebind(
        &mut self,
        ctx: &GpuContext,
        id: ViewportId,
        target: wgpu::TextureView,
        source: wgpu::TextureView,
    ) -> Result<(), GraphError> {
        let (voi, camera, dpr, interpolation, colour) = {
            let state = self
                .registry
                .states
                .get(usize::try_from(id.0).unwrap_or(usize::MAX))
                .and_then(Option::as_ref)
                .ok_or(GraphError::UnknownViewport)?;
            if state.stored.is_some() {
                return Err(GraphError::StoredNeedsRebind);
            }
            (
                voi_of(&state.pass),
                state.camera,
                state.dpr,
                state.interpolation,
                state.colour.clone(),
            )
        };
        let slot = self.create_gpu_slot(
            ctx,
            target,
            source,
            (voi, camera, dpr, interpolation),
            None,
            colour,
        );
        if let Some(held) = self
            .gpu
            .get_mut(usize::try_from(id.0).unwrap_or(usize::MAX))
        {
            *held = Some(slot);
        }
        Ok(())
    }

    /// Recreate the stored texture after a device rebuild.
    ///
    /// `install_recovered` dropped the texture with the rest of the slot. This
    /// creates it again at the extent registration recorded and does not
    /// increment [`RenderGraph::uploads`] or [`RenderGraph::lut_uploads`].
    /// A colour table the registry still holds is written again here. The
    /// caller then uploads samples.
    ///
    /// # Errors
    ///
    /// [`UploadError::UnknownViewport`] for an id this graph never issued.
    /// [`UploadError::NotStored`] when the viewport has no owned texture.
    /// [`UploadError::FormatUnavailable`] when the format chosen at
    /// registration can no longer be sampled or written, in which case the
    /// slot stays empty. The check is that remembered format, not a fresh
    /// search. [`UploadError::ExtentUnavailable`] when a recorded axis is
    /// longer than this device's `max_texture_dimension_2d`. The texture is
    /// not created and the slot stays empty.
    pub fn rebind_stored(
        &mut self,
        ctx: &GpuContext,
        adapter: &ResolvedAdapter,
        id: ViewportId,
        target: wgpu::TextureView,
    ) -> Result<(), UploadError> {
        let (upload, voi, camera, dpr, interpolation, format, colour) = {
            let state = self
                .registry
                .states
                .get(usize::try_from(id.0).unwrap_or(usize::MAX))
                .and_then(Option::as_ref)
                .ok_or(UploadError::UnknownViewport)?;
            let upload = state.stored.ok_or(UploadError::NotStored)?;
            let format = state.stored_format.ok_or(UploadError::FormatUnavailable)?;
            (
                upload,
                voi_of(&state.pass),
                state.camera,
                state.dpr,
                state.interpolation,
                format,
                state.colour.clone(),
            )
        };
        let features = adapter.wgpu_adapter().get_texture_format_features(format);
        if !binding_available(features) {
            return Err(UploadError::FormatUnavailable);
        }
        let upload = Upload::from_extent(
            upload.width(),
            upload.height(),
            ctx.device().limits().max_texture_dimension_2d,
        )?;
        let texture = create_stored_texture(ctx.device(), upload, format);
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let stored = StoredTexture {
            texture,
            width: upload.width(),
            height: upload.height(),
            format,
        };
        let slot = self.create_gpu_slot(
            ctx,
            target,
            view,
            (voi, camera, dpr, interpolation),
            Some(stored),
            colour,
        );
        if let Some(held) = self
            .gpu
            .get_mut(usize::try_from(id.0).unwrap_or(usize::MAX))
        {
            *held = Some(slot);
        }
        Ok(())
    }

    /// Mark sources dirty on a registered viewport.
    ///
    /// # Errors
    ///
    /// [`GraphError::UnknownViewport`] for an id this graph never issued.
    pub fn mark_dirty(&mut self, id: ViewportId, dirty: Dirty) -> Result<(), GraphError> {
        if dirty.voi {
            return Err(GraphError::VoiViaMarkDirty);
        }
        let state = self
            .registry
            .state_mut(id)
            .ok_or(GraphError::UnknownViewport)?;
        state.dirty.camera |= dirty.camera;
        state.dirty.slice |= dirty.slice;
        state.dirty.segmentation |= dirty.segmentation;
        state.dirty.data |= dirty.data;
        Ok(())
    }

    /// Write a resolved chain's thirty-two uniform bytes and mark VOI dirty.
    ///
    /// This is the only route that sets the VOI flag, so a window change
    /// cannot update bytes without scheduling the frame that shows them.
    /// It does not increment [`RenderGraph::uploads`],
    /// [`RenderGraph::lut_uploads`] or [`RenderGraph::brick_uploads`], and
    /// it does not write the stored texture, the colour table or a brick.
    ///
    /// Validation runs state first and the chain last, so the answer names
    /// the first thing that is wrong in the order a caller can act on it.
    ///
    /// # Errors
    ///
    /// [`GraphError::UnknownViewport`] for an id this graph never issued.
    /// [`GraphError::PassHasNoUniform`] for a registered pass with no
    /// uniform. [`GraphError::ViewportNotRebound`] for a stack viewport
    /// whose GPU objects a rebuild has not yet recreated. [`GraphError::Voi`]
    /// when the chain is driven by a LUT Sequence and has no section 18.4
    /// uniform, in which case no substituted values are written.
    pub fn set_voi(
        &mut self,
        ctx: &GpuContext,
        id: ViewportId,
        chain: &LutChain,
    ) -> Result<(), GraphError> {
        let has_uniform = {
            let state = self
                .registry
                .states
                .get(usize::try_from(id.0).unwrap_or(usize::MAX))
                .and_then(Option::as_ref)
                .ok_or(GraphError::UnknownViewport)?;
            matches!(
                state.pass,
                Pass::Stack(_) | Pass::VolumeRaycast(_) | Pass::Slab(_)
            )
        };
        if !has_uniform {
            return Err(GraphError::PassHasNoUniform);
        }
        // Rebound before the chain is read, so an unrebound viewport does
        // not report a chain it was never going to write.
        if self
            .gpu
            .get(usize::try_from(id.0).unwrap_or(usize::MAX))
            .and_then(Option::as_ref)
            .is_none()
        {
            return Err(GraphError::ViewportNotRebound);
        }
        let params = VoiParams::from_chain(chain)?;
        let slot = self
            .gpu
            .get(usize::try_from(id.0).unwrap_or(usize::MAX))
            .and_then(Option::as_ref)
            .ok_or(GraphError::ViewportNotRebound)?;
        ctx.queue()
            .write_buffer(&slot.uniform, 0, bytemuck::bytes_of(&params));
        if let Some(state) = self.registry.state_mut(id) {
            match &mut state.pass {
                Pass::Stack(stack) => stack.voi = params,
                Pass::VolumeRaycast(volume) => volume.voi = params,
                Pass::Slab(slab) => slab.voi = params,
                Pass::SegOverlay(_) => {}
            }
            state.dirty.voi = true;
        }
        Ok(())
    }

    /// Write the CSS camera and the device pixel ratio, and mark the camera dirty.
    ///
    /// The CSS bytes stay HLD section 17.4's `camera` field, column-major.
    /// The fragment shader divides the device fragment by `dpr` once before
    /// multiplying by that matrix. This does not write the VOI uniform, the
    /// colour uniform, the stored texture or a brick, and it does not
    /// increment [`RenderGraph::uploads`], [`RenderGraph::lut_uploads`] or
    /// [`RenderGraph::brick_uploads`].
    ///
    /// Validation follows [`RenderGraph::set_voi`]: an unknown id, then a
    /// pass with no stack uniform, then a viewport whose GPU objects have
    /// not been recreated.
    ///
    /// # Errors
    ///
    /// [`GraphError::UnknownViewport`] for an id this graph never issued.
    /// [`GraphError::PassHasNoUniform`] for a registered pass that is not a
    /// stack. [`GraphError::ViewportNotRebound`] when a rebuild has not yet
    /// recreated the slot. [`GraphError::DevicePixelRatio`] when `dpr` is not
    /// finite, not strictly positive, or does not narrow to a positive `f32`.
    pub fn set_view(
        &mut self,
        ctx: &GpuContext,
        id: ViewportId,
        camera: [f32; 16],
        dpr: f64,
    ) -> Result<(), GraphError> {
        let is_stack = {
            let state = self
                .registry
                .states
                .get(usize::try_from(id.0).unwrap_or(usize::MAX))
                .and_then(Option::as_ref)
                .ok_or(GraphError::UnknownViewport)?;
            matches!(state.pass, Pass::Stack(_))
        };
        if !is_stack {
            return Err(GraphError::PassHasNoUniform);
        }
        if self
            .gpu
            .get(usize::try_from(id.0).unwrap_or(usize::MAX))
            .and_then(Option::as_ref)
            .is_none()
        {
            return Err(GraphError::ViewportNotRebound);
        }
        if !crate::extent::usable_ratio(dpr) {
            return Err(GraphError::DevicePixelRatio);
        }
        let interpolation = self
            .registry
            .states
            .get(usize::try_from(id.0).unwrap_or(usize::MAX))
            .and_then(Option::as_ref)
            .map(|state| state.interpolation)
            .ok_or(GraphError::UnknownViewport)?;
        let slot = self
            .gpu
            .get(usize::try_from(id.0).unwrap_or(usize::MAX))
            .and_then(Option::as_ref)
            .ok_or(GraphError::ViewportNotRebound)?;
        let uploaded = device_camera(camera, dpr, interpolation);
        ctx.queue()
            .write_buffer(&slot.camera, 0, bytemuck::bytes_of(&uploaded));
        if let Some(state) = self.registry.state_mut(id) {
            state.camera = camera;
            state.dpr = dpr;
            state.dirty.camera = true;
        }
        Ok(())
    }

    /// Write the stack sample mode into the spare camera lane.
    ///
    /// `0` stays nearest. `1` selects linear. The call rewrites the camera
    /// uniform and marks the camera dirty. It does not allocate a texture.
    ///
    /// # Errors
    ///
    /// [`GraphError::UnknownViewport`] for an id this graph never issued.
    /// [`GraphError::PassHasNoUniform`] for a pass that is not a stack.
    /// [`GraphError::ViewportNotRebound`] when the slot has no GPU objects.
    pub fn set_stack_interpolation(
        &mut self,
        ctx: &GpuContext,
        id: ViewportId,
        mode: Interpolation,
    ) -> Result<(), GraphError> {
        let (camera, dpr, is_stack) = {
            let state = self
                .registry
                .states
                .get(usize::try_from(id.0).unwrap_or(usize::MAX))
                .and_then(Option::as_ref)
                .ok_or(GraphError::UnknownViewport)?;
            (
                state.camera,
                state.dpr,
                matches!(state.pass, Pass::Stack(_)),
            )
        };
        if !is_stack {
            return Err(GraphError::PassHasNoUniform);
        }
        if !crate::interpolation::available(mode, self.tier) {
            return Err(GraphError::InterpolationUnavailable);
        }
        let slot = self
            .gpu
            .get(usize::try_from(id.0).unwrap_or(usize::MAX))
            .and_then(Option::as_ref)
            .ok_or(GraphError::ViewportNotRebound)?;
        let uploaded = device_camera(camera, dpr, mode);
        ctx.queue()
            .write_buffer(&slot.camera, 0, bytemuck::bytes_of(&uploaded));
        if let Some(state) = self.registry.state_mut(id) {
            state.interpolation = mode;
            state.dirty.camera = true;
        }
        Ok(())
    }

    /// Upload a colour table into an `Rgba8Unorm` texture of height 1.
    ///
    /// The texture is created here, at selection, and filled once with
    /// `write_texture`. [`RenderGraph::set_voi`] does not write it again and
    /// does not increment [`RenderGraph::lut_uploads`]. This call increments
    /// that counter by one. [`RenderGraph::uploads`] stays put. `Dirty.data`
    /// is set so a clean viewport redraws. The VOI flag is not set here.
    ///
    /// A preset is indexed by the display value `lut_chain` already produced.
    /// A palette is indexed by the stored sample. The bytes are the table
    /// [`ColourLut`] already baked, so the shader does not walk the descriptor.
    ///
    /// # Errors
    ///
    /// [`GraphError::UnknownViewport`] for an id this graph never issued.
    /// [`GraphError::ViewportNotRebound`] when a rebuild has not yet
    /// recreated the slot. [`GraphError::ColourUnavailable`] when
    /// `Rgba8Unorm` cannot be sampled and copied, the table has no texels,
    /// or the table is wider than the device `max_texture_dimension_2d`.
    /// Nothing is counted in that case. The texture is not created.
    pub fn select_colour_lut(
        &mut self,
        ctx: &GpuContext,
        adapter: &ResolvedAdapter,
        id: ViewportId,
        lut: &ColourLut,
    ) -> Result<(), GraphError> {
        let index = usize::try_from(id.0).unwrap_or(usize::MAX);
        let known = self
            .registry
            .states
            .get(index)
            .and_then(Option::as_ref)
            .is_some();
        if !known {
            return Err(GraphError::UnknownViewport);
        }
        let rebound = self.gpu.get(index).and_then(Option::as_ref).is_some();
        if !rebound {
            return Err(GraphError::ViewportNotRebound);
        }
        let features = adapter
            .wgpu_adapter()
            .get_texture_format_features(COLOUR_LUT_FORMAT);
        if !binding_available(features) {
            return Err(GraphError::ColourUnavailable);
        }
        let bytes = lut.texture_bytes();
        let Some(width) = colour_width(bytes, ctx.device().limits().max_texture_dimension_2d)
        else {
            return Err(GraphError::ColourUnavailable);
        };
        let params = ColourParams::from_lut(lut);
        let texture = create_colour_texture(ctx.device(), width);
        write_colour_lut(ctx.queue(), &texture, bytes, width);
        let colour_view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_source = {
            let slot = self
                .gpu
                .get(index)
                .and_then(Option::as_ref)
                .ok_or(GraphError::ViewportNotRebound)?;
            self.bind_source_group(ctx.device(), &slot.source, &colour_view)
        };
        let slot = self
            .gpu
            .get_mut(index)
            .and_then(Option::as_mut)
            .ok_or(GraphError::ViewportNotRebound)?;
        ctx.queue()
            .write_buffer(&slot.colour_uniform, 0, bytemuck::bytes_of(&params));
        slot.bind_source = bind_source;
        slot.colour_texture = texture;
        self.lut_uploads += 1;
        if let Some(state) = self.registry.state_mut(id) {
            state.colour = Some(HeldColour {
                params,
                bytes: bytes.to_vec(),
            });
            state.dirty.data = true;
        }
        Ok(())
    }

    /// Copy stored samples into the viewport's texture with `queue.write_texture`.
    ///
    /// `samples` are stored values, one per texel, not windowed values. The
    /// length must be `width * height`. A wrong length returns before the copy
    /// and before [`RenderGraph::uploads`] changes. A matching extent reuses
    /// the texture. A different extent replaces it at this call, after asking
    /// the adapter for the format registration remembered. The `R32Float` copy
    /// uses `bytemuck::cast_slice` on the caller slice and allocates no
    /// buffer. A wider float format pads the `.r` sample at this call, which
    /// is an upload and not a frame.
    /// `uploads` increments by one and `Dirty.data` is set. Samples are not
    /// clamped.
    ///
    /// # Errors
    ///
    /// [`UploadError::UnknownViewport`] for an id this graph never issued.
    /// [`UploadError::NotStored`] when the viewport has no owned texture.
    /// [`UploadError::WrongLength`] when `samples` does not cover the extent,
    /// in which case nothing is written. [`UploadError::NotRebound`] when a
    /// rebuild has not yet recreated the slot. [`UploadError::FormatUnavailable`]
    /// when a new extent cannot be sampled, in which case the old texture
    /// stays. [`UploadError::ExtentUnavailable`] when an axis is longer than
    /// the device `max_texture_dimension_2d`. The old texture stays and
    /// `uploads` does not change.
    pub fn upload_stored(
        &mut self,
        ctx: &GpuContext,
        adapter: &ResolvedAdapter,
        id: ViewportId,
        width: u32,
        height: u32,
        samples: &[f32],
    ) -> Result<(), UploadError> {
        let current = self.remembered(id)?;
        let format = self.remembered_format(id)?;
        let upload = Upload::from_extent(
            width,
            height,
            ctx.device().limits().max_texture_dimension_2d,
        )?;
        upload.accepts(samples)?;
        stored_bytes_per_row(upload.width(), format)?;
        let slot_present = self
            .gpu
            .get(usize::try_from(id.0).unwrap_or(usize::MAX))
            .and_then(Option::as_ref)
            .is_some();
        if !slot_present {
            return Err(UploadError::NotRebound);
        }
        if current.width() != upload.width() || current.height() != upload.height() {
            let features = adapter.wgpu_adapter().get_texture_format_features(format);
            if !binding_available(features) {
                return Err(UploadError::FormatUnavailable);
            }
            self.replace_stored(ctx, id, upload, format)?;
        }
        self.write_slot(ctx, id, upload, samples)?;
        self.uploads += 1;
        if let Some(state) = self.registry.state_mut(id) {
            state.stored = Some(upload);
            state.dirty.data = true;
        }
        Ok(())
    }

    fn remembered(&self, id: ViewportId) -> Result<Upload, UploadError> {
        match self
            .registry
            .states
            .get(usize::try_from(id.0).unwrap_or(usize::MAX))
            .and_then(Option::as_ref)
        {
            None => Err(UploadError::UnknownViewport),
            Some(state) => state.stored.ok_or(UploadError::NotStored),
        }
    }

    fn remembered_format(&self, id: ViewportId) -> Result<wgpu::TextureFormat, UploadError> {
        match self
            .registry
            .states
            .get(usize::try_from(id.0).unwrap_or(usize::MAX))
            .and_then(Option::as_ref)
        {
            None => Err(UploadError::UnknownViewport),
            Some(state) if state.stored.is_none() => Err(UploadError::NotStored),
            Some(state) => state.stored_format.ok_or(UploadError::FormatUnavailable),
        }
    }

    fn replace_stored(
        &mut self,
        ctx: &GpuContext,
        id: ViewportId,
        upload: Upload,
        format: wgpu::TextureFormat,
    ) -> Result<(), UploadError> {
        let texture = create_stored_texture(ctx.device(), upload, format);
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let colour_view = {
            let slot = self
                .gpu
                .get(usize::try_from(id.0).unwrap_or(usize::MAX))
                .and_then(Option::as_ref)
                .ok_or(UploadError::NotRebound)?;
            slot.colour_texture
                .create_view(&wgpu::TextureViewDescriptor::default())
        };
        let bind_source = self.bind_source_group(ctx.device(), &view, &colour_view);
        let slot = self
            .gpu
            .get_mut(usize::try_from(id.0).unwrap_or(usize::MAX))
            .and_then(Option::as_mut)
            .ok_or(UploadError::NotRebound)?;
        slot.source = view;
        slot.bind_source = bind_source;
        slot.stored = Some(StoredTexture {
            texture,
            width: upload.width(),
            height: upload.height(),
            format,
        });
        Ok(())
    }

    fn write_slot(
        &self,
        ctx: &GpuContext,
        id: ViewportId,
        upload: Upload,
        samples: &[f32],
    ) -> Result<(), UploadError> {
        let slot = self
            .gpu
            .get(usize::try_from(id.0).unwrap_or(usize::MAX))
            .and_then(Option::as_ref)
            .ok_or(UploadError::NotRebound)?;
        let stored = slot.stored.as_ref().ok_or(UploadError::NotStored)?;
        if stored.width != upload.width() || stored.height != upload.height() {
            return Err(UploadError::WrongLength);
        }
        write_stored_samples(ctx.queue(), &stored.texture, upload, stored.format, samples)
    }

    /// Copy one brick into a 3D texture at its true extent.
    ///
    /// The bytes are the volume's container bytes. They are not windowed.
    /// The format is [`brick_format`] for `brick.sample`, and features are
    /// read for that format only. A second upload of `key` replaces the
    /// texture and increments [`RenderGraph::brick_uploads`] again. On
    /// refusal the brick is returned and the counter stays put, so a tier C
    /// caller can keep the host buffer.
    ///
    /// # Errors
    ///
    /// [`BrickUploadError`], with the brick returned alongside it. Nothing
    /// is created in that case.
    pub fn upload_brick(
        &mut self,
        ctx: &GpuContext,
        adapter: &ResolvedAdapter,
        key: BrickKey,
        brick: Brick,
    ) -> Result<(), (Brick, BrickUploadError)> {
        let format = brick_format(brick.sample);
        let features = adapter.wgpu_adapter().get_texture_format_features(format);
        self.bricks.upload(ctx, format, features, key, brick)?;
        // The new texture is a different object. A ray that still holds the
        // old view would sample the bytes this upload replaced.
        self.drop_ray_bound_to(key);
        Ok(())
    }

    /// Drop every ray that samples `key`, and mark that viewport data-dirty.
    ///
    /// `bind_volume_ray` is what builds views of the texture now stored for
    /// the key. Until that call, the viewport cannot encode, so `tick`
    /// reports it unavailable and keeps the dirt.
    fn drop_ray_bound_to(&mut self, key: BrickKey) {
        for index in 0..self.gpu.len() {
            let holds = self
                .gpu
                .get(index)
                .and_then(Option::as_ref)
                .and_then(|slot| slot.ray.as_ref())
                .is_some_and(|ray| ray.keys.contains(&key));
            if !holds {
                continue;
            }
            if let Some(slot) = self.gpu.get_mut(index).and_then(Option::as_mut) {
                slot.ray = None;
            }
            let id = ViewportId(u32::try_from(index).unwrap_or(u32::MAX));
            if let Some(state) = self.registry.state_mut(id) {
                state.dirty.data = true;
            }
        }
    }

    /// Bind the bricks in `source` and a residency row for a volume viewport.
    ///
    /// The residency texture is `R32Uint`, one texel per step of the view
    /// axis, written with `write_texture`. Its width is fixed at the first
    /// bind from `max(dims)`. A later bind that needs a longer axis returns
    /// [`GraphError::CapacityExhausted`] and keeps the previous binding.
    /// [`volume_ray_admission`] runs before any texture is created.
    /// A residency row longer than the device `max_texture_dimension_2d` is
    /// [`GraphError::RayUnavailable`] before that texture is created. The
    /// previous binding stays.
    ///
    /// # Errors
    ///
    /// [`GraphError::TierCUnavailable`], [`GraphError::SampledTextureLimit`],
    /// [`GraphError::UnknownViewport`], [`GraphError::ViewportNotRebound`],
    /// [`GraphError::RayUnavailable`], [`GraphError::CapacityExhausted`].
    pub fn bind_volume_ray(
        &mut self,
        ctx: &GpuContext,
        adapter: &ResolvedAdapter,
        id: ViewportId,
        source: VolumeRaySource<'_>,
    ) -> Result<(), GraphError> {
        let VolumeRaySource {
            bricks,
            edge,
            dims,
            flags,
            sampled_limit,
            spacing_mm,
            origin_mm,
            direction,
        } = source;
        let ready_bricks = u32::try_from(bricks.len()).map_err(|_| GraphError::RayUnavailable)?;
        volume_ray_admission(self.tier, ready_bricks, sampled_limit)?;
        if !spacing_mm_ok(spacing_mm) {
            return Err(GraphError::RayUnavailable);
        }
        let Some(frame) = PatientFrame::try_new(origin_mm, direction) else {
            return Err(GraphError::RayUnavailable);
        };
        if ready_bricks > self.brick_slots {
            return Err(GraphError::SampledTextureLimit);
        }
        let (axis, from_far, opacity, shade, clip) = {
            let state = self.slot_state(id)?;
            match &state.pass {
                Pass::VolumeRaycast(volume) => (
                    volume.axis,
                    volume.from_far,
                    state.opacity,
                    state.shade,
                    state.clip,
                ),
                _ => return Err(GraphError::RayUnavailable),
            }
        };
        let index = usize::try_from(id.0).unwrap_or(usize::MAX);
        if self.gpu.get(index).and_then(Option::as_ref).is_none() {
            return Err(GraphError::ViewportNotRebound);
        }
        let steps = u32::try_from(flags.len()).map_err(|_| GraphError::RayUnavailable)?;
        if steps == 0 || steps != axis.length(dims) {
            return Err(GraphError::RayUnavailable);
        }
        let requested = dims[0].max(dims[1]).max(dims[2]);
        if steps > requested {
            return Err(GraphError::RayUnavailable);
        }
        let capacity = {
            let slot = self
                .gpu
                .get(index)
                .and_then(Option::as_ref)
                .ok_or(GraphError::ViewportNotRebound)?;
            match slot.ray.as_ref() {
                Some(existing) => {
                    if requested > existing.capacity || steps > existing.capacity {
                        return Err(GraphError::CapacityExhausted);
                    }
                    existing.capacity
                }
                None => requested,
            }
        };
        if edge == 0 {
            return Err(GraphError::RayUnavailable);
        }
        let mut brick_format = None;
        for (index, ready) in bricks.iter().enumerate() {
            for earlier in bricks.iter().take(index) {
                if earlier.coord == ready.coord {
                    return Err(GraphError::RayUnavailable);
                }
            }
            let Some(texture) = self.bricks.texture(ready.key) else {
                return Err(GraphError::RayUnavailable);
            };
            let format = texture.format();
            if format != wgpu::TextureFormat::R8Uint && format != wgpu::TextureFormat::R16Uint {
                return Err(GraphError::RayUnavailable);
            }
            if let Some(held) = brick_format
                && held != format
            {
                return Err(GraphError::RayUnavailable);
            }
            brick_format = Some(format);
            let Some(extent) = self.bricks.extent_of(ready.key) else {
                return Err(GraphError::RayUnavailable);
            };
            let Some(need) = clipped_brick(dims, ready.coord, edge) else {
                return Err(GraphError::RayUnavailable);
            };
            if extent[0] < need[0] || extent[1] < need[1] || extent[2] < need[2] {
                return Err(GraphError::RayUnavailable);
            }
        }
        let needed = wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST;
        let residency_features = adapter
            .wgpu_adapter()
            .get_texture_format_features(wgpu::TextureFormat::R32Uint);
        if !residency_features.allowed_usages.contains(needed) {
            return Err(GraphError::RayUnavailable);
        }
        if !residency_row_fits(capacity, ctx.device().limits().max_texture_dimension_2d) {
            return Err(GraphError::RayUnavailable);
        }
        let texel_count = usize::try_from(capacity).map_err(|_| GraphError::RayUnavailable)?;
        let mut texels = vec![0u32; texel_count];
        for (offset, flag) in flags.iter().copied().enumerate() {
            let Some(cell) = texels.get_mut(offset) else {
                return Err(GraphError::RayUnavailable);
            };
            *cell = flag;
        }
        let row = capacity.checked_mul(4).ok_or(GraphError::RayUnavailable)?;
        let device = ctx.device();
        let residency = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("ocelli ray residency"),
            size: wgpu::Extent3d {
                width: capacity,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R32Uint,
            usage: needed,
            view_formats: &[],
        });
        ctx.queue().write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &residency,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            bytemuck::cast_slice(&texels),
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(row),
                rows_per_image: Some(1),
            },
            wgpu::Extent3d {
                width: capacity,
                height: 1,
                depth_or_array_layers: 1,
            },
        );
        let residency_view = residency.create_view(&wgpu::TextureViewDescriptor {
            label: Some("ocelli ray residency"),
            format: Some(wgpu::TextureFormat::R32Uint),
            dimension: Some(wgpu::TextureViewDimension::D2),
            ..Default::default()
        });
        let mut brick_views = Vec::new();
        for ready in bricks {
            let Some(texture) = self.bricks.texture(ready.key) else {
                return Err(GraphError::RayUnavailable);
            };
            brick_views.push(texture.create_view(&wgpu::TextureViewDescriptor {
                label: Some("ocelli ray brick"),
                dimension: Some(wgpu::TextureViewDimension::D3),
                ..Default::default()
            }));
        }
        let dummy_view = self.ray_dummy.create_view(&wgpu::TextureViewDescriptor {
            label: Some("ocelli ray empty brick"),
            dimension: Some(wgpu::TextureViewDimension::D3),
            ..Default::default()
        });
        let header = RayHeader {
            axis: axis.code(),
            from_far: u32::from(from_far),
            dim_x: dims[0],
            dim_y: dims[1],
            dim_z: dims[2],
            steps,
            kind: 0,
            edge,
            brick_count: ready_bricks,
            spacing_x: spacing_mm[0],
            spacing_y: spacing_mm[1],
            spacing_z: spacing_mm[2],
        };
        let coords: Vec<[u32; 3]> = bricks.iter().map(|ready| ready.coord).collect();
        let uniform = ray_uniform_bytes(&header, &coords, self.brick_slots);
        let params_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("ocelli ray params"),
            contents: &uniform,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let opacity_bytes = opacity.uniform_bytes();
        let opacity_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("ocelli ray opacity"),
            contents: &opacity_bytes,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let shade_bytes = shade.uniform_bytes();
        let shade_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("ocelli ray shade"),
            contents: &shade_bytes,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let clip_bytes = clip.uniform_bytes(&frame);
        let clip_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("ocelli ray clip"),
            contents: &clip_bytes,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let bind_uniform = {
            let slot = self
                .gpu
                .get(index)
                .and_then(Option::as_ref)
                .ok_or(GraphError::ViewportNotRebound)?;
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("ocelli volume ray uniform"),
                layout: &self.volume_uniform_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: slot.uniform.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: params_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: opacity_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: shade_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: clip_buffer.as_entire_binding(),
                    },
                ],
            })
        };
        let mut source_entries = Vec::new();
        for (binding, view) in brick_views.iter().enumerate() {
            let Ok(index) = u32::try_from(binding) else {
                return Err(GraphError::RayUnavailable);
            };
            source_entries.push(wgpu::BindGroupEntry {
                binding: index,
                resource: wgpu::BindingResource::TextureView(view),
            });
        }
        let slot_count = usize::try_from(self.brick_slots).unwrap_or(usize::MAX);
        for binding in brick_views.len()..slot_count {
            let Ok(index) = u32::try_from(binding) else {
                return Err(GraphError::RayUnavailable);
            };
            source_entries.push(wgpu::BindGroupEntry {
                binding: index,
                resource: wgpu::BindingResource::TextureView(&dummy_view),
            });
        }
        source_entries.push(wgpu::BindGroupEntry {
            binding: self.brick_slots,
            resource: wgpu::BindingResource::TextureView(&residency_view),
        });
        let bind_source = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ocelli volume ray source"),
            layout: &self.volume_source_layout,
            entries: &source_entries,
        });
        let slot = self
            .gpu
            .get_mut(index)
            .and_then(Option::as_mut)
            .ok_or(GraphError::ViewportNotRebound)?;
        let keys = bricks.iter().map(|ready| *ready.key).collect();
        let binding = VolumeRayBinding {
            capacity,
            residency,
            params: params_buffer,
            opacity: opacity_buffer,
            shade: shade_buffer,
            clip: clip_buffer,
            bind_uniform,
            bind_source,
            keys,
        };
        // The texture and the uniforms stay on the slot. Dropping any of
        // them would free a resource the bind group samples.
        let uniform_bytes = 48u64 + u64::from(self.brick_slots.max(1)) * 16;
        if binding.residency.size().width != binding.capacity
            || binding.params.size() != uniform_bytes
            || binding.opacity.size() != 1024
            || binding.shade.size() != 16
            || binding.clip.size() != u64::try_from(CLIP_UNIFORM_BYTES).unwrap_or(u64::MAX)
        {
            return Err(GraphError::RayUnavailable);
        }
        slot.ray = Some(binding);
        if let Some(state) = self.registry.state_mut(id) {
            state.frame = frame;
            state.dirty.data = true;
        }
        Ok(())
    }

    /// Write the opacity uniform for a bound volume ray.
    ///
    /// The buffer was created by [`RenderGraph::bind_volume_ray`]. This call
    /// replaces its bytes and does not create a second buffer. It does not
    /// increment [`RenderGraph::brick_uploads`], [`RenderGraph::uploads`] or
    /// [`RenderGraph::lut_uploads`], and it does not write a brick texture.
    /// The same bytes are a no-op, so a repeated call does not mark the
    /// viewport dirty again.
    ///
    /// Tier C returns [`GraphError::TierCUnavailable`] before a sample is
    /// read and before the uniform is written. A table this method refuses
    /// is refused by [`crate::opacity::OpacityTable::from_entries`] before
    /// the call, so the previous uniform stays.
    ///
    /// # Errors
    ///
    /// [`GraphError::TierCUnavailable`] on tier C.
    /// [`GraphError::UnknownViewport`] for an id this graph never issued.
    /// [`GraphError::PassHasNoUniform`] for a pass that is not a volume ray.
    /// [`GraphError::ViewportNotRebound`] when a rebuild has not yet
    /// recreated the slot. [`GraphError::RayUnavailable`] when the ray has
    /// not been bound, in which case nothing is written.
    pub fn set_opacity(
        &mut self,
        ctx: &GpuContext,
        id: ViewportId,
        table: &OpacityTable,
    ) -> Result<(), GraphError> {
        if self.tier == Tier::Cpu {
            return Err(GraphError::TierCUnavailable);
        }
        let index = usize::try_from(id.0).unwrap_or(usize::MAX);
        let is_volume = {
            let state = self
                .registry
                .states
                .get(index)
                .and_then(Option::as_ref)
                .ok_or(GraphError::UnknownViewport)?;
            matches!(state.pass, Pass::VolumeRaycast(_))
        };
        if !is_volume {
            return Err(GraphError::PassHasNoUniform);
        }
        if self.gpu.get(index).and_then(Option::as_ref).is_none() {
            return Err(GraphError::ViewportNotRebound);
        }
        if self
            .gpu
            .get(index)
            .and_then(Option::as_ref)
            .and_then(|slot| slot.ray.as_ref())
            .is_none()
        {
            return Err(GraphError::RayUnavailable);
        }
        if self.slot_state(id)?.opacity.same(table) {
            return Ok(());
        }
        let bytes = table.uniform_bytes();
        {
            let ray = self
                .gpu
                .get(index)
                .and_then(Option::as_ref)
                .and_then(|slot| slot.ray.as_ref())
                .ok_or(GraphError::RayUnavailable)?;
            ctx.queue().write_buffer(&ray.opacity, 0, &bytes);
        }
        if let Some(state) = self.registry.state_mut(id) {
            state.opacity = *table;
            state.dirty.data = true;
        }
        Ok(())
    }

    /// Write the shade uniform for a bound volume ray.
    ///
    /// The buffer was created by [`RenderGraph::bind_volume_ray`]. This call
    /// replaces its bytes and does not create a second buffer. It does not
    /// increment [`RenderGraph::brick_uploads`], [`RenderGraph::uploads`] or
    /// [`RenderGraph::lut_uploads`], and it does not write a brick texture.
    /// The same bytes are a no-op, so a repeated call does not mark the
    /// viewport dirty again.
    ///
    /// Tier C returns [`GraphError::TierCUnavailable`] before a sample is
    /// read and before the uniform is written. A pair this method would
    /// refuse is refused by [`crate::shade::ShadeWeights::from_weights`]
    /// before the call, so the previous uniform stays. A call before the
    /// ray is bound returns [`GraphError::RayUnavailable`] and does not
    /// store the weights, so the later bind stays unlit.
    ///
    /// # Errors
    ///
    /// [`GraphError::TierCUnavailable`] on tier C.
    /// [`GraphError::UnknownViewport`] for an id this graph never issued.
    /// [`GraphError::PassHasNoUniform`] for a pass that is not a volume ray.
    /// [`GraphError::ViewportNotRebound`] when a rebuild has not yet
    /// recreated the slot. [`GraphError::RayUnavailable`] when the ray has
    /// not been bound, in which case nothing is written.
    pub fn set_shade(
        &mut self,
        ctx: &GpuContext,
        id: ViewportId,
        weights: &ShadeWeights,
    ) -> Result<(), GraphError> {
        if self.tier == Tier::Cpu {
            return Err(GraphError::TierCUnavailable);
        }
        let index = usize::try_from(id.0).unwrap_or(usize::MAX);
        let is_volume = {
            let state = self
                .registry
                .states
                .get(index)
                .and_then(Option::as_ref)
                .ok_or(GraphError::UnknownViewport)?;
            matches!(state.pass, Pass::VolumeRaycast(_))
        };
        if !is_volume {
            return Err(GraphError::PassHasNoUniform);
        }
        if self.gpu.get(index).and_then(Option::as_ref).is_none() {
            return Err(GraphError::ViewportNotRebound);
        }
        if self
            .gpu
            .get(index)
            .and_then(Option::as_ref)
            .and_then(|slot| slot.ray.as_ref())
            .is_none()
        {
            return Err(GraphError::RayUnavailable);
        }
        if self.slot_state(id)?.shade.same(weights) {
            return Ok(());
        }
        let bytes = weights.uniform_bytes();
        {
            let ray = self
                .gpu
                .get(index)
                .and_then(Option::as_ref)
                .and_then(|slot| slot.ray.as_ref())
                .ok_or(GraphError::RayUnavailable)?;
            ctx.queue().write_buffer(&ray.shade, 0, &bytes);
        }
        if let Some(state) = self.registry.state_mut(id) {
            state.shade = *weights;
            state.dirty.data = true;
        }
        Ok(())
    }

    /// Write the clip uniform for a bound volume ray.
    ///
    /// The buffer was created by [`RenderGraph::bind_volume_ray`]. This call
    /// replaces its bytes and does not create a second buffer. It does not
    /// increment [`RenderGraph::brick_uploads`], [`RenderGraph::uploads`] or
    /// [`RenderGraph::lut_uploads`], and it does not write a brick texture.
    /// The same bytes are a no-op, so a repeated call does not mark the
    /// viewport dirty again. A call before the ray is bound returns
    /// [`GraphError::RayUnavailable`] and does not store the planes, so the
    /// later bind stays empty.
    ///
    /// Tier C returns [`GraphError::TierCUnavailable`] before a sample is
    /// read and before the uniform is written. More than six planes, a
    /// non-finite component, or a zero-length normal leaves the previous list
    /// unchanged and does not write the uniform.
    ///
    /// # Errors
    ///
    /// [`GraphError::TierCUnavailable`] on tier C.
    /// [`GraphError::UnknownViewport`] for an id this graph never issued.
    /// [`GraphError::PassHasNoUniform`] for a pass that is not a volume ray.
    /// [`GraphError::ViewportNotRebound`] when a rebuild has not yet
    /// recreated the slot. [`GraphError::RayUnavailable`] when the ray has
    /// not been bound. [`GraphError::ClipCount`], [`GraphError::ClipNotFinite`]
    /// and [`GraphError::ClipZeroNormal`] when the list itself is refused.
    pub fn set_clip_planes(
        &mut self,
        ctx: &GpuContext,
        id: ViewportId,
        planes: &[crate::clip::ClipPlane],
    ) -> Result<(), GraphError> {
        if self.tier == Tier::Cpu {
            return Err(GraphError::TierCUnavailable);
        }
        let index = usize::try_from(id.0).unwrap_or(usize::MAX);
        let is_volume = {
            let state = self
                .registry
                .states
                .get(index)
                .and_then(Option::as_ref)
                .ok_or(GraphError::UnknownViewport)?;
            matches!(state.pass, Pass::VolumeRaycast(_))
        };
        if !is_volume {
            return Err(GraphError::PassHasNoUniform);
        }
        if self.gpu.get(index).and_then(Option::as_ref).is_none() {
            return Err(GraphError::ViewportNotRebound);
        }
        if self
            .gpu
            .get(index)
            .and_then(Option::as_ref)
            .and_then(|slot| slot.ray.as_ref())
            .is_none()
        {
            return Err(GraphError::RayUnavailable);
        }
        let accepted = ClipList::from_planes(planes).map_err(clip_error)?;
        if self.slot_state(id)?.clip.same(&accepted) {
            return Ok(());
        }
        let bytes = {
            let frame = self.slot_state(id)?.frame;
            accepted.uniform_bytes(&frame)
        };
        {
            let ray = self
                .gpu
                .get(index)
                .and_then(Option::as_ref)
                .and_then(|slot| slot.ray.as_ref())
                .ok_or(GraphError::RayUnavailable)?;
            ctx.queue().write_buffer(&ray.clip, 0, &bytes);
        }
        if let Some(state) = self.registry.state_mut(id) {
            state.clip = accepted;
            state.dirty.data = true;
        }
        Ok(())
    }

    /// Upload each ready level-0 brick whose bytes changed.
    ///
    /// `adapter` is the one [`RenderGraph::upload_brick`] already requires.
    /// This graph does not store a second one. Tier C is refused by
    /// [`progressive::sync_ready_bricks`] before a cut and before a stamp
    /// write. A brick `ready_bricks` omits is not uploaded and is not filled
    /// with zeros. Recovery drops brick textures and their stamps, and the
    /// next sync uploads ready bricks again. An upload that replaces a
    /// texture a volume ray is bound to drops that ray. The viewport stays
    /// data-dirty, and the next tick is unavailable until
    /// [`RenderGraph::bind_volume_ray`] builds views of the new texture.
    ///
    /// # Errors
    ///
    /// [`ProgressError`]. Bricks uploaded before a later refusal stay
    /// resident, and their stamps stay. The refused brick is not stamped.
    pub fn sync_volume(
        &mut self,
        ctx: &GpuContext,
        adapter: &ResolvedAdapter,
        volume_id: VolumeId,
        volume: &Volume,
    ) -> Result<ProgressReport, ProgressError> {
        let mut stamps = std::mem::take(&mut self.brick_stamps);
        let outcome = progressive::sync_ready_bricks(
            self.tier,
            &mut stamps,
            volume_id,
            volume,
            |key, brick| match self.upload_brick(ctx, adapter, key, brick) {
                Ok(()) => Ok(()),
                Err((brick, source)) => Err(ProgressError::Upload {
                    brick: Some(brick),
                    source,
                }),
            },
        );
        self.brick_stamps = stamps;
        outcome
    }

    /// Upload one complete orthographic plane through the stack texture.
    ///
    /// `image` is the host reslice. This method does not sample the volume.
    /// Tier C is refused by [`ortho::present_plane`] before a sample is read.
    /// An absent sample is [`OrthoError::PlaneIncomplete`] and `upload_stored`
    /// is not called, so `uploads` and the previous texture stay.
    ///
    /// # Errors
    ///
    /// [`OrthoError`].
    pub fn show_ortho(
        &mut self,
        ctx: &GpuContext,
        adapter: &ResolvedAdapter,
        id: ViewportId,
        image: &OrthoImage,
    ) -> Result<(), OrthoError> {
        let tier = self.tier;
        ortho::present_plane(tier, image, |samples| {
            self.upload_stored(ctx, adapter, id, image.width, image.height, samples)
        })
    }

    /// Upload one host slab and select its compiled blend.
    ///
    /// Tier C is refused by [`slab::present_slab`] before a sample is read.
    /// An uncompiled blend, including labelmap edge, returns
    /// [`SlabError::BlendUnavailable`] and does not bind composite. A
    /// dimension above [`crate::caps::Caps::max_tex_3d`] returns
    /// [`SlabError::ExtentUnavailable`]. The tick encodes this viewport only
    /// after a successful upload. `uploads` does not change.
    ///
    /// Equal samples, step, index step, count, size and blend do not allocate
    /// a new texture and do not mark dirt. A blend change keeps the textures
    /// and marks data dirty. A failed upload drops the staged textures and
    /// keeps the last binding.
    ///
    /// # Errors
    ///
    /// [`SlabError`].
    pub fn show_slab(
        &mut self,
        ctx: &GpuContext,
        adapter: &ResolvedAdapter,
        id: ViewportId,
        image: &SlabImage,
        blend: BlendMode,
    ) -> Result<(), SlabError> {
        let max_tex_3d = ctx.caps().max_tex_3d;
        let viewport_is_slab = match self.slot_state(id) {
            Ok(state) => matches!(state.pass, Pass::Slab(_)),
            Err(_) => {
                return slab::present_slab(self.tier, true, blend, image, max_tex_3d, |_| {
                    Err(UploadError::UnknownViewport)
                });
            }
        };
        slab::present_slab(
            self.tier,
            viewport_is_slab,
            blend,
            image,
            max_tex_3d,
            |image| self.commit_slab(ctx, adapter, id, image, blend),
        )
    }

    fn commit_slab(
        &mut self,
        ctx: &GpuContext,
        adapter: &ResolvedAdapter,
        id: ViewportId,
        image: &SlabImage,
        blend: BlendMode,
    ) -> Result<(), UploadError> {
        let index = usize::try_from(id.0).unwrap_or(usize::MAX);
        if self.gpu.get(index).and_then(Option::as_ref).is_none() {
            return Err(UploadError::NotRebound);
        }
        let incoming = slab_stamp(image, blend)?;
        let held = self
            .gpu
            .get(index)
            .and_then(Option::as_ref)
            .and_then(|slot| slot.slab.as_ref())
            .map(|binding| binding.stamp.clone());
        if let Some(held) = held {
            if held == incoming {
                return Ok(());
            }
            if held.same_pixels(&incoming) {
                if let Some(binding) = self
                    .gpu
                    .get_mut(index)
                    .and_then(Option::as_mut)
                    .and_then(|slot| slot.slab.as_mut())
                {
                    binding.stamp.blend = blend;
                }
                self.mark_slab_blend(id, blend);
                return Ok(());
            }
        }
        let format = selected_stored_format(adapter.wgpu_adapter(), self.tier)?;
        let staged = {
            let slot = self
                .gpu
                .get(index)
                .and_then(Option::as_ref)
                .ok_or(UploadError::NotRebound)?;
            stage_slab(
                ctx,
                &self.slab_uniform_layout,
                &self.slab_source_layout,
                &slot.uniform,
                image,
                format,
                incoming,
            )?
        };
        if let Some(slot) = self.gpu.get_mut(index).and_then(Option::as_mut) {
            slot.slab = Some(staged);
        }
        self.mark_slab_blend(id, blend);
        Ok(())
    }

    fn mark_slab_blend(&mut self, id: ViewportId, blend: BlendMode) {
        if let Some(state) = self.registry.state_mut(id) {
            if let Pass::Slab(pass) = &mut state.pass {
                pass.blend = blend;
            }
            state.dirty.data = true;
        }
    }

    /// The representation a composite overlay is showing.
    ///
    /// A new overlay is [`ocelli_seg::Representation::Labelmap`].
    ///
    /// # Errors
    ///
    /// [`GraphError::UnknownViewport`] when the id was never registered.
    /// [`GraphError::NotOverlay`] for a stack, a slab or a volume.
    pub fn representation(&self, id: ViewportId) -> Result<ocelli_seg::Representation, GraphError> {
        let state = self.slot_state(id)?;
        match &state.pass {
            Pass::SegOverlay(_) => Ok(state.representation),
            _ => Err(GraphError::NotOverlay),
        }
    }

    /// Choose which representation the next matching show uploads.
    ///
    /// The same value keeps the overlay texture and does not mark dirt.
    /// A different value drops that texture and marks segmentation dirt.
    /// This call does not upload.
    ///
    /// # Errors
    ///
    /// [`GraphError::UnknownViewport`] when the id was never registered.
    /// [`GraphError::NotOverlay`] for a stack, a slab or a volume.
    /// Those viewports are left unchanged.
    pub fn set_representation(
        &mut self,
        id: ViewportId,
        representation: ocelli_seg::Representation,
    ) -> Result<(), GraphError> {
        let index = usize::try_from(id.0).unwrap_or(usize::MAX);
        let changed = {
            let state = self
                .registry
                .state_mut(id)
                .ok_or(GraphError::UnknownViewport)?;
            if !matches!(&state.pass, Pass::SegOverlay(_)) {
                return Err(GraphError::NotOverlay);
            }
            if state.representation == representation {
                false
            } else {
                state.representation = representation;
                state.dirty.segmentation = true;
                true
            }
        };
        if changed && let Some(slot) = self.gpu.get_mut(index).and_then(Option::as_mut) {
            slot.labelmap = None;
        }
        Ok(())
    }

    /// Upload one presented labelmap onto a composite segmentation viewport.
    ///
    /// Tier C is refused before the grid is read. A viewport that is not a
    /// `SegOverlay` is refused before the presenter runs. Composite uses the
    /// caller style. Labelmap edge projection forces outline without fill,
    /// so a fill request does not alias composite. A stack viewport does not
    /// gain an overlay. A presenter error uploads nothing.
    ///
    /// Equal bytes do not allocate a texture and do not mark dirt. A new
    /// image replaces the binding only after the texture exists, then sets
    /// segmentation and data dirt. The tick copies those bytes. It does not
    /// recompute the outline. `uploads` does not change.
    ///
    /// # Errors
    ///
    /// [`LabelmapDrawError`].
    pub fn show_labelmap(
        &mut self,
        ctx: &GpuContext,
        id: ViewportId,
        grid: &ocelli_seg::LabelGrid,
        lut: &ocelli_seg::LabelColourLut,
        style: ocelli_seg::LabelStyle,
    ) -> Result<(), LabelmapDrawError> {
        if self.tier == Tier::Cpu {
            return Err(LabelmapDrawError::TierCUnavailable);
        }
        let (blend, representation) = match self.slot_state(id) {
            Ok(state) => match &state.pass {
                Pass::SegOverlay(pass) => (pass.blend, state.representation),
                _ => return Err(LabelmapDrawError::NotOverlay),
            },
            Err(_) => return Err(LabelmapDrawError::UnknownViewport),
        };
        let edge = blend == BlendMode::LabelmapEdgeProjection;
        if blend != BlendMode::Composite && !edge {
            return Err(LabelmapDrawError::BlendUnavailable);
        }
        if representation != ocelli_seg::Representation::Labelmap {
            return Err(LabelmapDrawError::RepresentationMismatch);
        }
        let drawn = if edge {
            ocelli_seg::LabelStyle {
                fill: false,
                outline: true,
            }
        } else {
            style
        };
        let index = usize::try_from(id.0).unwrap_or(usize::MAX);
        if self.gpu.get(index).and_then(Option::as_ref).is_none() {
            return Err(LabelmapDrawError::NotRebound);
        }
        if !drawn.fill && !drawn.outline {
            let mut unused = [];
            return ocelli_seg::present_labelmap(grid, lut, drawn, &mut unused)
                .map_err(LabelmapDrawError::Present);
        }
        let width_us = usize::try_from(grid.width).map_err(|_| LabelmapDrawError::Extent)?;
        let height_us = usize::try_from(grid.height).map_err(|_| LabelmapDrawError::Extent)?;
        let pixels = width_us
            .checked_mul(height_us)
            .ok_or(LabelmapDrawError::Extent)?;
        let byte_len = pixels.checked_mul(4).ok_or(LabelmapDrawError::Extent)?;
        let mut bytes = vec![0_u8; byte_len];
        ocelli_seg::present_labelmap(grid, lut, drawn, &mut bytes)
            .map_err(LabelmapDrawError::Present)?;
        self.store_overlay(ctx, id, index, grid.width, grid.height, bytes)
    }

    /// Upload one presented contour onto a composite segmentation viewport.
    ///
    /// Tier C is refused before the points are read. The copy shader is the
    /// labelmap shader. It does not stroke and it does not fill.
    ///
    /// # Errors
    ///
    /// [`crate::contour::ContourDrawError`].
    pub fn show_contour(
        &mut self,
        ctx: &GpuContext,
        id: ViewportId,
        contour: &ocelli_seg::Contour,
        frame: &ocelli_seg::PlaneFrame,
        lut: &ocelli_seg::LabelColourLut,
        view: crate::contour::ContourView,
    ) -> Result<(), crate::contour::ContourDrawError> {
        if self.tier == Tier::Cpu {
            return Err(crate::contour::ContourDrawError::TierCUnavailable);
        }
        let (blend, representation) = match self.slot_state(id) {
            Ok(state) => match &state.pass {
                Pass::SegOverlay(pass) => (pass.blend, state.representation),
                _ => return Err(crate::contour::ContourDrawError::NotOverlay),
            },
            Err(_) => return Err(crate::contour::ContourDrawError::UnknownViewport),
        };
        if blend != BlendMode::Composite {
            return Err(crate::contour::ContourDrawError::BlendUnavailable);
        }
        if representation != ocelli_seg::Representation::Contour {
            return Err(crate::contour::ContourDrawError::RepresentationMismatch);
        }
        let index = usize::try_from(id.0).unwrap_or(usize::MAX);
        if self.gpu.get(index).and_then(Option::as_ref).is_none() {
            return Err(crate::contour::ContourDrawError::NotRebound);
        }
        let width_us =
            usize::try_from(view.width).map_err(|_| crate::contour::ContourDrawError::Extent)?;
        let height_us =
            usize::try_from(view.height).map_err(|_| crate::contour::ContourDrawError::Extent)?;
        let pixels = width_us
            .checked_mul(height_us)
            .ok_or(crate::contour::ContourDrawError::Extent)?;
        let byte_len = pixels
            .checked_mul(4)
            .ok_or(crate::contour::ContourDrawError::Extent)?;
        let mut bytes = vec![0_u8; byte_len];
        ocelli_seg::present_contour(contour, frame, lut, view.width, view.height, &mut bytes)
            .map_err(crate::contour::ContourDrawError::Present)?;
        self.store_overlay(ctx, id, index, view.width, view.height, bytes)
            .map_err(crate::contour::ContourDrawError::from_labelmap)
    }

    /// Upload one presented surface onto a composite segmentation viewport.
    ///
    /// Tier C is refused before the mesh is read. The copy shader is the
    /// labelmap shader. It does not test triangle coverage.
    ///
    /// # Errors
    ///
    /// [`crate::surface::SurfaceDrawError`].
    pub fn show_surface(
        &mut self,
        ctx: &GpuContext,
        id: ViewportId,
        mesh: &ocelli_seg::SurfaceMesh,
        frame: &ocelli_seg::PlaneFrame,
        lut: &ocelli_seg::LabelColourLut,
        view: crate::surface::SurfaceView,
    ) -> Result<(), crate::surface::SurfaceDrawError> {
        if self.tier == Tier::Cpu {
            return Err(crate::surface::SurfaceDrawError::TierCUnavailable);
        }
        let (blend, representation) = match self.slot_state(id) {
            Ok(state) => match &state.pass {
                Pass::SegOverlay(pass) => (pass.blend, state.representation),
                _ => return Err(crate::surface::SurfaceDrawError::NotOverlay),
            },
            Err(_) => return Err(crate::surface::SurfaceDrawError::UnknownViewport),
        };
        if blend != BlendMode::Composite {
            return Err(crate::surface::SurfaceDrawError::BlendUnavailable);
        }
        if representation != ocelli_seg::Representation::Surface {
            return Err(crate::surface::SurfaceDrawError::RepresentationMismatch);
        }
        let index = usize::try_from(id.0).unwrap_or(usize::MAX);
        if self.gpu.get(index).and_then(Option::as_ref).is_none() {
            return Err(crate::surface::SurfaceDrawError::NotRebound);
        }
        let width_us =
            usize::try_from(view.width).map_err(|_| crate::surface::SurfaceDrawError::Extent)?;
        let height_us =
            usize::try_from(view.height).map_err(|_| crate::surface::SurfaceDrawError::Extent)?;
        let pixels = width_us
            .checked_mul(height_us)
            .ok_or(crate::surface::SurfaceDrawError::Extent)?;
        let byte_len = pixels
            .checked_mul(4)
            .ok_or(crate::surface::SurfaceDrawError::Extent)?;
        let mut bytes = vec![0_u8; byte_len];
        ocelli_seg::present_surface(mesh, frame, lut, view.width, view.height, &mut bytes)
            .map_err(crate::surface::SurfaceDrawError::Present)?;
        self.store_overlay(ctx, id, index, view.width, view.height, bytes)
            .map_err(crate::surface::SurfaceDrawError::from_labelmap)
    }

    fn store_overlay(
        &mut self,
        ctx: &GpuContext,
        id: ViewportId,
        index: usize,
        width: u32,
        height: u32,
        bytes: Vec<u8>,
    ) -> Result<(), LabelmapDrawError> {
        let same = self
            .gpu
            .get(index)
            .and_then(Option::as_ref)
            .and_then(|slot| slot.labelmap.as_ref())
            .is_some_and(|held| held.same_bytes(width, height, &bytes));
        if same {
            return Ok(());
        }
        let staged = crate::labelmap::stage(&self.labelmap_layout, ctx, width, height, bytes)?;
        let Some(slot) = self.gpu.get_mut(index).and_then(Option::as_mut) else {
            return Err(LabelmapDrawError::NotRebound);
        };
        slot.labelmap = Some(staged);
        if let Some(state) = self.registry.state_mut(id) {
            state.dirty.segmentation = true;
            state.dirty.data = true;
        }
        Ok(())
    }

    /// Reinstall the resources a recovery rebuilt, marking every viewport
    /// fully dirty because every GPU object the old device owned is gone.
    pub fn install_recovered(&mut self, ctx: &GpuContext, recovered: &Recovered) {
        // `recovered.graph` reports what the rebuild counted, and a graph
        // that registered nothing there still cannot keep this graph's
        // per-viewport objects, which belonged to the dead device all the
        // same. There is no silent return: installing always drops them.
        // `recovered.caps` is the tier `open` copied from the same adapter.
        // Recovery does not resolve a tier.
        self.tier = recovered.caps.tier;
        let compiled = Self::compile(ctx, self.tier, self.format);
        self.uniform_layout = compiled.uniform_layout;
        self.source_layout = compiled.source_layout;
        self.volume_uniform_layout = compiled.volume_uniform_layout;
        self.volume_source_layout = compiled.volume_source_layout;
        self.slab_uniform_layout = compiled.slab_uniform_layout;
        self.slab_source_layout = compiled.slab_source_layout;
        self.labelmap_layout = compiled.labelmap_layout;
        self.brick_slots = compiled.brick_slots;
        self.ray_dummy = compiled.ray_dummy;
        self.keys = compiled.keys;
        self.pipelines = compiled.pipelines;
        // Every per-viewport object was created on the dead device: the
        // uniform buffers, both bind groups, the stored-sample texture, the
        // colour table texture, the labelmap texture and the caller's views
        // as bound. The representation choice stays with the pass. The next
        // matching show after `rebind` uploads the overlay again.
        // They are dropped here, not carried into a pass on the new device.
        // The registry keeps each pass, its last uniform values, the camera
        // floats, the stored extent, the colour table bytes and the opacity
        // table. The caller re-supplies a
        // caller-owned source through `rebind`, or recreates an owned texture
        // through `rebind_stored`. Either restore writes the colour bytes
        // again and does not increment `lut_uploads`. The next
        // `bind_volume_ray` writes the remembered opacity table, the
        // remembered shade weights and the remembered clip planes into new
        // uniforms and does not increment `brick_uploads`. The next bind
        // also replaces the patient frame from the source it is given.
        // Surfaces were configured on the dead device. The frame texture is
        // dropped before the surface it came from. The shell still holds the
        // canvases and supplies them again through `bind_surface`.
        // A viewport with no rebound slot reports unavailable in the tick
        // and keeps its dirt.
        // Brick textures were created on the dead device. They are dropped
        // here and `brick_uploads` is not incremented. The stamps of those
        // bytes are dropped with them, so the next `sync_volume` uploads
        // ready bricks again. The caller supplies the bricks through
        // `upload_brick`.
        for slot in self.gpu.iter_mut() {
            *slot = None;
        }
        self.fusion = None;
        self.bricks.clear();
        self.brick_stamps.clear();
        for held in &mut self.frame_textures {
            *held = None;
        }
        for held in &mut self.surfaces {
            *held = None;
        }
        for state in self.registry.states.iter_mut().flatten() {
            state.dirty = Dirty::full();
        }
    }

    /// One frame tick, driven by the caller's requestAnimationFrame.
    ///
    /// Reads the device state once, encodes each dirty viewport once in
    /// registration order into one encoder, and submits exactly once. Dirt
    /// clears only after a successful encode. A viewport whose pass has no
    /// compiled pipeline keeps its dirt and is reported unavailable.
    ///
    /// A viewport with a surface draws into `get_current_texture` for this
    /// tick. `present` runs after the single submit, once per acquired
    /// texture, not per viewport inside the encoder. A surface that does not
    /// yield a texture keeps its dirt and is not drawn into the caller-supplied
    /// view. A viewport with no surface keeps that view.
    pub fn tick(&mut self, ctx: &GpuContext) -> FrameOutcome {
        if matches!(ctx.state(), crate::gpu::DeviceState::Lost(_)) {
            self.discard_frame_textures();
            return FrameOutcome::DeviceLost;
        }
        let mut first_unavailable: Option<ViewportId> = None;
        let mut any_encodable = false;
        for (index, state) in self.registry.states.iter().enumerate() {
            let Some(state) = state else { continue };
            if !state.dirty.any() {
                continue;
            }
            let key = PipelineKey {
                pass: state.pass.kind(),
                blend: state.pass.blend(),
                tier: self.tier,
            };
            let slot = self.gpu.get(index).and_then(Option::as_ref);
            let rebound = slot.is_some();
            let ready = gpu_binding_ready(state.pass.kind(), slot);
            if self.pipeline(&key, PipelineFamily::Fragment).is_some() && rebound && ready {
                any_encodable = true;
            } else if first_unavailable.is_none() {
                first_unavailable = Some(ViewportId(u32::try_from(index).unwrap_or(u32::MAX)));
            }
        }
        let fusion_due = self.fusion.as_ref().is_some_and(|view| view.needs_encode());
        if !any_encodable && !fusion_due {
            // No encoder is created for a frame that cannot encode, so
            // nothing unfinished is dropped.
            return match first_unavailable {
                Some(viewport) => FrameOutcome::Unavailable { viewport },
                None => FrameOutcome::Noop,
            };
        }
        self.acquire_dirty_surfaces();
        self.encode_frame(ctx, first_unavailable)
    }

    fn discard_frame_textures(&mut self) {
        for held in &mut self.frame_textures {
            *held = None;
        }
    }

    /// Acquire a current texture for each dirty surface viewport that can encode.
    ///
    /// Success and Suboptimal are both drawable. Every other status drops the
    /// result, which discards it, and leaves the slot empty so the encode
    /// skips that viewport without clearing its dirt.
    fn acquire_dirty_surfaces(&mut self) {
        self.discard_frame_textures();
        let tier = self.tier;
        let RenderGraph {
            registry,
            gpu,
            surfaces,
            frame_textures,
            pipelines,
            keys,
            ..
        } = self;
        for index in 0..registry.states.len() {
            if !viewport_encodable(registry, gpu, pipelines, keys, tier, index) {
                continue;
            }
            let Some(attached) = surfaces.get(index).and_then(Option::as_ref) else {
                continue;
            };
            let texture = match attached.surface.get_current_texture() {
                wgpu::CurrentSurfaceTexture::Success(texture)
                | wgpu::CurrentSurfaceTexture::Suboptimal(texture) => texture,
                _ => continue,
            };
            if let Some(slot) = frame_textures.get_mut(index) {
                *slot = Some(texture);
            }
        }
    }

    fn encode_frame(
        &mut self,
        ctx: &GpuContext,
        first_unavailable: Option<ViewportId>,
    ) -> FrameOutcome {
        let tier = self.tier;
        let RenderGraph {
            registry,
            gpu,
            surfaces,
            frame_textures,
            pipelines,
            keys,
            submissions,
            fusion,
            ..
        } = self;
        let mut encoder = ctx
            .device()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("ocelli render graph frame"),
            });
        let mut encoded: u32 = 0;
        for index in 0..registry.states.len() {
            let key = {
                let Some(state) = registry.states.get(index).and_then(Option::as_ref) else {
                    continue;
                };
                if !state.dirty.any() {
                    continue;
                }
                PipelineKey {
                    pass: state.pass.kind(),
                    blend: state.pass.blend(),
                    tier,
                }
            };
            let Some(pipeline) = pipeline_index(keys, &key, PipelineFamily::Fragment)
                .and_then(|found| pipelines.get(found))
            else {
                continue;
            };
            let Some(slot) = gpu.get(index).and_then(Option::as_ref) else {
                continue;
            };
            if !gpu_binding_ready(key.pass, Some(slot)) {
                continue;
            }
            let has_surface = surfaces.get(index).and_then(Option::as_ref).is_some();
            let surface_view = frame_textures
                .get(index)
                .and_then(Option::as_ref)
                .map(|held| {
                    held.texture
                        .create_view(&wgpu::TextureViewDescriptor::default())
                });
            if has_surface && surface_view.is_none() {
                continue;
            }
            let caller = &slot.target;
            let view = surface_view.as_ref().unwrap_or(caller);
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("ocelli render graph pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view,
                        depth_slice: None,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: if key.pass == PassKind::Slab {
                                wgpu::LoadOp::Load
                            } else {
                                wgpu::LoadOp::Clear(wgpu::Color::BLACK)
                            },
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    ..Default::default()
                });
                pass.set_pipeline(pipeline);
                if let Some(ray) = slot
                    .ray
                    .as_ref()
                    .filter(|_| key.pass == PassKind::VolumeRaycast)
                {
                    pass.set_bind_group(0, &ray.bind_uniform, &[]);
                    pass.set_bind_group(1, &ray.bind_source, &[]);
                } else if let Some(slab) = slot.slab.as_ref().filter(|_| key.pass == PassKind::Slab)
                {
                    pass.set_bind_group(0, &slab.bind_uniform, &[]);
                    pass.set_bind_group(1, &slab.bind_source, &[]);
                } else if let Some(label) = slot
                    .labelmap
                    .as_ref()
                    .filter(|_| key.pass == PassKind::SegOverlay)
                {
                    pass.set_bind_group(0, &label.bind, &[]);
                } else if key.pass != PassKind::SegOverlay {
                    pass.set_bind_group(0, &slot.bind_uniform, &[]);
                    pass.set_bind_group(1, &slot.bind_source, &[]);
                }
                pass.draw(0..3, 0..1);
            }
            encoded += 1;
            if let Some(state) = registry.states.get_mut(index).and_then(Option::as_mut) {
                state.dirty.clear();
            }
        }
        let mut fusion_passes = 0_u32;
        if let Some(view) = fusion.as_mut()
            && view.needs_encode()
        {
            fusion_passes = view.encode(&mut encoder);
        }
        if encoded == 0 && fusion_passes == 0 {
            for held in frame_textures.iter_mut() {
                *held = None;
            }
            return match first_unavailable {
                Some(viewport) => FrameOutcome::Unavailable { viewport },
                None => FrameOutcome::Noop,
            };
        }
        ctx.queue().submit(std::iter::once(encoder.finish()));
        *submissions += 1;
        for held in frame_textures.iter_mut() {
            if let Some(texture) = held.take() {
                ctx.queue().present(texture);
            }
        }
        FrameOutcome::Submitted {
            viewports: encoded,
            passes: encoded.saturating_add(fusion_passes),
        }
    }

    /// Bind a fusion view, or refuse before any sample upload.
    ///
    /// `adapter` is the session adapter. The sample planes use the same
    /// preserving float format as a slab upload. A device that cannot bind
    /// that format, or cannot bind `Rgba16Float` as a render target and a
    /// sampled texture, returns [`FusionError::Unavailable`] and does not
    /// build a CPU image.
    ///
    /// Tier C, a UID mismatch, an empty UID, an opacity outside 0 to 1, a
    /// chain with no uniform, a colormap that is not 256 entries, and a plane
    /// that cannot be sampled all return before the published texture changes.
    /// The shader is compiled here, when the view is created, not inside
    /// [`RenderGraph::tick`]. The tick encodes the view in the existing
    /// encoder and still submits once. This does not add a [`Pass`].
    ///
    /// # Errors
    ///
    /// [`FusionError`]. A device loss is [`FusionError::Unavailable`], not a
    /// frame-of-reference mismatch.
    pub fn show_fusion(
        &mut self,
        ctx: &GpuContext,
        adapter: &ResolvedAdapter,
        request: &FusionRequest<'_>,
    ) -> Result<(), FusionError> {
        if matches!(ctx.state(), crate::gpu::DeviceState::Lost(_)) {
            return Err(FusionError::Unavailable);
        }
        let decision = {
            let view = self.fusion.as_ref();
            let dirty = view.map(|view| view.dirty()).unwrap_or(Dirty::full());
            let stamp = view.map(|view| view.stamp());
            let uid = view.and_then(|view| view.uid());
            fusion::decide(self.tier, request, dirty, stamp.as_ref(), uid)
        };
        match decision {
            fusion::Decision::Refuse(err) => Err(err),
            fusion::Decision::Idle => Ok(()),
            fusion::Decision::Encode(stamp) => fusion::stage(
                ctx,
                adapter,
                self.tier,
                &mut self.fusion,
                self.format,
                request,
                *stamp,
            ),
        }
    }

    /// How many times a fusion bind uploaded volume samples.
    ///
    /// An opacity-only change leaves this where it was.
    pub fn fusion_sample_uploads(&self) -> u64 {
        self.fusion
            .as_ref()
            .map(|view| view.sample_uploads())
            .unwrap_or(0)
    }
}

fn viewport_encodable(
    registry: &Registry,
    gpu: &[Option<GpuSlot>],
    pipelines: &[wgpu::RenderPipeline],
    keys: &[CompiledKey],
    tier: Tier,
    index: usize,
) -> bool {
    let Some(state) = registry.states.get(index).and_then(Option::as_ref) else {
        return false;
    };
    if !state.dirty.any() {
        return false;
    }
    let key = PipelineKey {
        pass: state.pass.kind(),
        blend: state.pass.blend(),
        tier,
    };
    let compiled = pipeline_index(keys, &key, PipelineFamily::Fragment)
        .and_then(|found| pipelines.get(found))
        .is_some();
    let slot = gpu.get(index).and_then(Option::as_ref);
    let rebound = slot.is_some();
    let ready = gpu_binding_ready(key.pass, slot);
    compiled && rebound && ready
}

fn gpu_binding_ready(kind: PassKind, slot: Option<&GpuSlot>) -> bool {
    match kind {
        PassKind::VolumeRaycast => slot.and_then(|slot| slot.ray.as_ref()).is_some(),
        PassKind::Slab => slot.and_then(|slot| slot.slab.as_ref()).is_some(),
        PassKind::SegOverlay => slot.and_then(|slot| slot.labelmap.as_ref()).is_some(),
        PassKind::Stack => true,
    }
}

fn voi_of(pass: &Pass) -> VoiParams {
    match pass {
        Pass::Stack(stack) => stack.voi,
        Pass::VolumeRaycast(volume) => volume.voi,
        Pass::Slab(slab) => slab.voi,
        Pass::SegOverlay(_) => VoiParams {
            center: 0.0,
            width: 1.0,
            slope: 1.0,
            intercept: 0.0,
            ymin: 0.0,
            ymax: 255.0,
            fn_kind: 0,
            invert: 0,
        },
    }
}

fn slab_stamp(image: &SlabImage, blend: BlendMode) -> Result<SlabStamp, UploadError> {
    let width = usize::try_from(image.width).map_err(|_| UploadError::ExtentOverflow)?;
    let height = usize::try_from(image.height).map_err(|_| UploadError::ExtentOverflow)?;
    let count = usize::try_from(image.sample_count).map_err(|_| UploadError::ExtentOverflow)?;
    let pixels = width
        .checked_mul(height)
        .ok_or(UploadError::ExtentOverflow)?;
    let expected = pixels
        .checked_mul(count)
        .ok_or(UploadError::ExtentOverflow)?;
    if image.samples.len() != expected {
        return Err(UploadError::WrongLength);
    }
    let mut samples = Vec::with_capacity(expected);
    for sample in &image.samples {
        samples.push(match *sample {
            PlaneSample::Present(value) => SlabSampleStamp::Present(value.to_bits()),
            PlaneSample::Absent => SlabSampleStamp::Absent,
        });
    }
    let [step_i, step_j, step_k] = image.index_step;
    Ok(SlabStamp {
        width: image.width,
        height: image.height,
        sample_count: image.sample_count,
        step_bits: image.step_mm.to_bits(),
        index_bits: [step_i.to_bits(), step_j.to_bits(), step_k.to_bits()],
        samples,
        blend,
    })
}

fn stage_slab(
    ctx: &GpuContext,
    uniform_layout: &wgpu::BindGroupLayout,
    source_layout: &wgpu::BindGroupLayout,
    uniform: &wgpu::Buffer,
    image: &SlabImage,
    format: wgpu::TextureFormat,
    stamp: SlabStamp,
) -> Result<SlabBinding, UploadError> {
    let channels = stored_float_channels(format).ok_or(UploadError::FormatUnavailable)?;
    let texel = channels.checked_mul(4).ok_or(UploadError::ExtentOverflow)?;
    let device = ctx.device();
    let values = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("ocelli slab values"),
        size: wgpu::Extent3d {
            width: image.width,
            height: image.height,
            depth_or_array_layers: image.sample_count,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D3,
        format,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let mask = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("ocelli slab mask"),
        size: wgpu::Extent3d {
            width: image.width,
            height: image.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    });
    let row_bytes = aligned_row_bytes(
        image
            .width
            .checked_mul(texel)
            .ok_or(UploadError::ExtentOverflow)?,
    )
    .ok_or(UploadError::ExtentOverflow)?;
    let mask_bytes = slab_mask_bytes(image, channels, row_bytes)?;
    // One copy of the whole volume. A 1 by 1 write rounds that single texel
    // up to 256 bytes inside the queue, so a wide slab must not upload a
    // column at a time. An absent column stays the zero fill. The mask is 0
    // there, and the shader discards that pixel before it loads a value.
    let value_bytes = slab_value_bytes(image, channels, row_bytes)?;
    ctx.queue().write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &mask,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &mask_bytes,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(row_bytes),
            rows_per_image: Some(image.height),
        },
        wgpu::Extent3d {
            width: image.width,
            height: image.height,
            depth_or_array_layers: 1,
        },
    );
    ctx.queue().write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: &values,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        &value_bytes,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(row_bytes),
            rows_per_image: Some(image.height),
        },
        wgpu::Extent3d {
            width: image.width,
            height: image.height,
            depth_or_array_layers: image.sample_count,
        },
    );
    let values_view = values.create_view(&wgpu::TextureViewDescriptor::default());
    let mask_view = mask.create_view(&wgpu::TextureViewDescriptor::default());
    let bind_uniform = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("ocelli slab voi"),
        layout: uniform_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: uniform.as_entire_binding(),
        }],
    });
    let bind_source = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("ocelli slab source"),
        layout: source_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&values_view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(&mask_view),
            },
        ],
    });
    Ok(SlabBinding {
        bind_uniform,
        bind_source,
        stamp,
    })
}

fn column_samples(image: &SlabImage, x: u32, y: u32) -> Result<&[PlaneSample], UploadError> {
    let width = usize::try_from(image.width).map_err(|_| UploadError::ExtentOverflow)?;
    let count = usize::try_from(image.sample_count).map_err(|_| UploadError::ExtentOverflow)?;
    let column = usize::try_from(x).map_err(|_| UploadError::ExtentOverflow)?;
    let row = usize::try_from(y).map_err(|_| UploadError::ExtentOverflow)?;
    let pixel = row
        .checked_mul(width)
        .and_then(|start| start.checked_add(column))
        .ok_or(UploadError::ExtentOverflow)?;
    let start = pixel
        .checked_mul(count)
        .ok_or(UploadError::ExtentOverflow)?;
    let end = start
        .checked_add(count)
        .ok_or(UploadError::ExtentOverflow)?;
    image
        .samples
        .get(start..end)
        .ok_or(UploadError::WrongLength)
}

fn column_present(samples: &[PlaneSample]) -> bool {
    samples
        .iter()
        .all(|sample| matches!(sample, PlaneSample::Present(_)))
}

fn slab_mask_bytes(
    image: &SlabImage,
    channels: u32,
    row_bytes: u32,
) -> Result<Vec<u8>, UploadError> {
    let height = usize::try_from(image.height).map_err(|_| UploadError::ExtentOverflow)?;
    let row = usize::try_from(row_bytes).map_err(|_| UploadError::ExtentOverflow)?;
    let len = height.checked_mul(row).ok_or(UploadError::ExtentOverflow)?;
    let mut bytes = vec![0_u8; len];
    let texel = usize::try_from(channels.checked_mul(4).ok_or(UploadError::ExtentOverflow)?)
        .map_err(|_| UploadError::ExtentOverflow)?;
    let mut y = 0_u32;
    while y < image.height {
        let mut x = 0_u32;
        while x < image.width {
            if column_present(column_samples(image, x, y)?) {
                let x_us = usize::try_from(x).map_err(|_| UploadError::ExtentOverflow)?;
                let y_us = usize::try_from(y).map_err(|_| UploadError::ExtentOverflow)?;
                let offset = y_us
                    .checked_mul(row)
                    .and_then(|start| start.checked_add(x_us.checked_mul(texel)?))
                    .ok_or(UploadError::ExtentOverflow)?;
                write_float_lanes(&mut bytes, offset, channels, 1.0)?;
            }
            x = x.checked_add(1).ok_or(UploadError::ExtentOverflow)?;
        }
        y = y.checked_add(1).ok_or(UploadError::ExtentOverflow)?;
    }
    Ok(bytes)
}

fn slab_value_bytes(
    image: &SlabImage,
    channels: u32,
    row_bytes: u32,
) -> Result<Vec<u8>, UploadError> {
    let height = usize::try_from(image.height).map_err(|_| UploadError::ExtentOverflow)?;
    let depth = usize::try_from(image.sample_count).map_err(|_| UploadError::ExtentOverflow)?;
    let row = usize::try_from(row_bytes).map_err(|_| UploadError::ExtentOverflow)?;
    let texel = usize::try_from(channels.checked_mul(4).ok_or(UploadError::ExtentOverflow)?)
        .map_err(|_| UploadError::ExtentOverflow)?;
    let plane = row.checked_mul(height).ok_or(UploadError::ExtentOverflow)?;
    let len = plane
        .checked_mul(depth)
        .ok_or(UploadError::ExtentOverflow)?;
    let mut bytes = vec![0_u8; len];
    let mut y = 0_u32;
    while y < image.height {
        let mut x = 0_u32;
        while x < image.width {
            let samples = column_samples(image, x, y)?;
            if column_present(samples) {
                let x_us = usize::try_from(x).map_err(|_| UploadError::ExtentOverflow)?;
                let y_us = usize::try_from(y).map_err(|_| UploadError::ExtentOverflow)?;
                let across = x_us.checked_mul(texel).ok_or(UploadError::ExtentOverflow)?;
                let row_at = y_us
                    .checked_mul(row)
                    .and_then(|start| start.checked_add(across))
                    .ok_or(UploadError::ExtentOverflow)?;
                for (z, sample) in samples.iter().copied().enumerate() {
                    let PlaneSample::Present(value) = sample else {
                        return Err(UploadError::WrongLength);
                    };
                    let offset = z
                        .checked_mul(plane)
                        .and_then(|start| start.checked_add(row_at))
                        .ok_or(UploadError::ExtentOverflow)?;
                    write_float_lanes(&mut bytes, offset, channels, value)?;
                }
            }
            x = x.checked_add(1).ok_or(UploadError::ExtentOverflow)?;
        }
        y = y.checked_add(1).ok_or(UploadError::ExtentOverflow)?;
    }
    Ok(bytes)
}

fn write_float_lanes(
    bytes: &mut [u8],
    offset: usize,
    channels: u32,
    value: f32,
) -> Result<(), UploadError> {
    let count = usize::try_from(channels).map_err(|_| UploadError::ExtentOverflow)?;
    for lane in 0..count {
        let lane_offset = lane.checked_mul(4).ok_or(UploadError::ExtentOverflow)?;
        let start = offset
            .checked_add(lane_offset)
            .ok_or(UploadError::ExtentOverflow)?;
        let end = start.checked_add(4).ok_or(UploadError::ExtentOverflow)?;
        let Some(slot) = bytes.get_mut(start..end) else {
            return Err(UploadError::ExtentOverflow);
        };
        let encoded = if lane == 0 {
            value.to_le_bytes()
        } else {
            0.0_f32.to_le_bytes()
        };
        slot.copy_from_slice(&encoded);
    }
    Ok(())
}

pub(crate) fn aligned_row_bytes(unpadded: u32) -> Option<u32> {
    let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let room = unpadded.checked_add(align - 1)?;
    Some(room / align * align)
}

fn residency_row_fits(capacity: u32, max_texture_dimension_2d: u32) -> bool {
    capacity <= max_texture_dimension_2d
}

fn colour_texel_count(bytes: &[u8]) -> Option<u32> {
    if bytes.is_empty() || !bytes.len().is_multiple_of(4) {
        return None;
    }
    u32::try_from(bytes.len() / 4).ok()
}

/// Texel width of a colour table that this device can create.
///
/// `max_texture_dimension_2d` is [`wgpu::Limits::max_texture_dimension_2d`]
/// from [`wgpu::Device::limits`]. A wider table is refused before
/// `create_texture`. A width equal to the limit still fits.
fn colour_width(bytes: &[u8], max_texture_dimension_2d: u32) -> Option<u32> {
    let width = colour_texel_count(bytes)?;
    if width > max_texture_dimension_2d {
        return None;
    }
    Some(width)
}

fn create_colour_texture(device: &wgpu::Device, width: u32) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("ocelli colour lut"),
        size: wgpu::Extent3d {
            width,
            height: 1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: COLOUR_LUT_FORMAT,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    })
}

fn write_colour_lut(queue: &wgpu::Queue, texture: &wgpu::Texture, bytes: &[u8], width: u32) {
    let Some(row) = width.checked_mul(4) else {
        return;
    };
    if usize::try_from(row).ok() != Some(bytes.len()) {
        return;
    }
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
            bytes_per_row: Some(row),
            rows_per_image: Some(1),
        },
        wgpu::Extent3d {
            width,
            height: 1,
            depth_or_array_layers: 1,
        },
    );
}

/// A selected table, or a 1x1 placeholder so group 1 binding 1 exists.
///
/// The placeholder is mode 0 and is not sampled. Creating it does not
/// increment `lut_uploads`.
fn colour_texture_for(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    held: Option<&HeldColour>,
) -> wgpu::Texture {
    let limit = device.limits().max_texture_dimension_2d;
    if let Some((held, width)) =
        held.and_then(|held| colour_width(&held.bytes, limit).map(|width| (held, width)))
    {
        let texture = create_colour_texture(device, width);
        write_colour_lut(queue, &texture, &held.bytes, width);
        return texture;
    }
    let texture = create_colour_texture(device, 1);
    write_colour_lut(queue, &texture, &[0, 0, 0, 255], 1);
    texture
}

fn create_stored_texture(
    device: &wgpu::Device,
    upload: Upload,
    format: wgpu::TextureFormat,
) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("ocelli stored samples"),
        size: wgpu::Extent3d {
            width: upload.width(),
            height: upload.height(),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        view_formats: &[],
    })
}

fn write_stored_samples(
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    upload: Upload,
    format: wgpu::TextureFormat,
    samples: &[f32],
) -> Result<(), UploadError> {
    let bytes_per_row = stored_bytes_per_row(upload.width(), format)?;
    let channels = stored_float_channels(format).ok_or(UploadError::FormatUnavailable)?;
    let copy = wgpu::TexelCopyTextureInfo {
        texture,
        mip_level: 0,
        origin: wgpu::Origin3d::ZERO,
        aspect: wgpu::TextureAspect::All,
    };
    let layout = wgpu::TexelCopyBufferLayout {
        offset: 0,
        bytes_per_row: Some(bytes_per_row),
        rows_per_image: Some(upload.height()),
    };
    let extent = wgpu::Extent3d {
        width: upload.width(),
        height: upload.height(),
        depth_or_array_layers: 1,
    };
    if channels == 1 {
        queue.write_texture(copy, bytemuck::cast_slice(samples), layout, extent);
        return Ok(());
    }
    let channel_count = usize::try_from(channels).map_err(|_| UploadError::ExtentOverflow)?;
    let padded_len = samples
        .len()
        .checked_mul(channel_count)
        .ok_or(UploadError::ExtentOverflow)?;
    let mut padded = Vec::new();
    padded.resize(padded_len, 0.0);
    for (index, sample) in samples.iter().copied().enumerate() {
        let Some(slot) = index.checked_mul(channel_count) else {
            return Err(UploadError::ExtentOverflow);
        };
        let Some(cell) = padded.get_mut(slot) else {
            return Err(UploadError::ExtentOverflow);
        };
        *cell = sample;
    }
    queue.write_texture(copy, bytemuck::cast_slice(&padded), layout, extent);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        CompiledKey, Dirty, Registry, SlotState, aligned_row_bytes, pipeline_index,
        slab_mask_bytes, slab_value_bytes,
    };
    use crate::caps::Tier;
    use crate::graph::{
        BlendMode, FrameOutcome, Pass, PassKind, PipelineFamily, PipelineKey, RayAxis, RenderGraph,
        Upload, UploadError, ViewportId, VolumePass,
    };
    use ocelli_volume::{PlaneSample, SlabImage};

    fn slab_of(width: u32, height: u32, depth: u32, absent_at: (u32, u32)) -> SlabImage {
        let mut samples = Vec::new();
        let mut y = 0_u32;
        while y < height {
            let mut x = 0_u32;
            while x < width {
                let mut z = 0_u32;
                while z < depth {
                    if (x, y) == absent_at {
                        samples.push(PlaneSample::Absent);
                    } else {
                        let Ok(x_byte) = u8::try_from(x) else {
                            unreachable!("the fixture column fits in a byte");
                        };
                        let Ok(y_byte) = u8::try_from(y) else {
                            unreachable!("the fixture row fits in a byte");
                        };
                        let Ok(z_byte) = u8::try_from(z) else {
                            unreachable!("the fixture depth fits in a byte");
                        };
                        let value =
                            f32::from(x_byte) + f32::from(y_byte) * 64.0 + f32::from(z_byte) * 0.5;
                        samples.push(PlaneSample::Present(value));
                    }
                    z = z.saturating_add(1);
                }
                x = x.saturating_add(1);
            }
            y = y.saturating_add(1);
        }
        SlabImage {
            width,
            height,
            step_mm: 1.0,
            index_step: [0.0, 0.0, 1.0],
            sample_count: depth,
            samples,
        }
    }

    fn f32_at(bytes: &[u8], offset: usize) -> Option<f32> {
        let end = offset.checked_add(4)?;
        let chunk = bytes.get(offset..end)?;
        let Ok(array) = <[u8; 4]>::try_from(chunk) else {
            return None;
        };
        Some(f32::from_le_bytes(array))
    }

    #[test]
    fn a_residency_row_past_the_device_limit_does_not_fit() {
        assert!(super::residency_row_fits(8, 8));
        assert!(!super::residency_row_fits(9, 8));
    }

    #[test]
    fn a_colour_table_wider_than_the_device_limit_is_refused() {
        let two = [0_u8, 0, 0, 255, 1, 1, 1, 255];
        assert_eq!(super::colour_width(&two, 2), Some(2));
        assert_eq!(super::colour_width(&two, 1), None);
        assert_eq!(super::colour_width(&[], 8), None);
        assert_eq!(super::colour_width(&[0, 0, 0], 8), None);
    }

    #[test]
    fn a_slab_volume_is_one_aligned_upload() {
        let width = 64_u32;
        let height = 2_u32;
        let depth = 3_u32;
        let image = slab_of(width, height, depth, (1, 0));
        let Some(row) = aligned_row_bytes(width * 4) else {
            unreachable!("64 texels of 4 bytes is one aligned row");
        };
        assert_eq!(row, 256);
        let Ok(values) = slab_value_bytes(&image, 1, row) else {
            unreachable!("the fixture extent fits");
        };
        let row_us = usize::try_from(row).unwrap_or(0);
        let height_us = usize::try_from(height).unwrap_or(0);
        let depth_us = usize::try_from(depth).unwrap_or(0);
        let plane = row_us.saturating_mul(height_us);
        assert_eq!(values.len(), plane.saturating_mul(depth_us));
        assert_eq!(values.len(), 1536);
        let present = f32_at(&values, row_us + 3 * 4);
        assert_eq!(present, Some(3.0 + 64.0));
        let deep = f32_at(&values, plane.saturating_mul(2) + row_us + 3 * 4);
        assert_eq!(deep, Some(3.0 + 64.0 + 1.0));
        assert_eq!(f32_at(&values, 4), Some(0.0));
        assert_eq!(f32_at(&values, plane + 4), Some(0.0));
        assert_eq!(f32_at(&values, plane.saturating_mul(2) + 4), Some(0.0));
        let Ok(mask) = slab_mask_bytes(&image, 1, row) else {
            unreachable!("the fixture extent fits");
        };
        assert_eq!(mask.len(), plane);
        assert_eq!(f32_at(&mask, 0), Some(1.0));
        assert_eq!(f32_at(&mask, 4), Some(0.0));
        assert_eq!(f32_at(&mask, row_us + 4), Some(1.0));
        let narrow = slab_of(2, 1, 2, (u32::MAX, u32::MAX));
        let Some(narrow_row) = aligned_row_bytes(16) else {
            unreachable!("two rg texels still round up to the copy alignment");
        };
        assert_eq!(narrow_row, 256);
        let Ok(narrow_values) = slab_value_bytes(&narrow, 2, narrow_row) else {
            unreachable!("the narrow fixture fits");
        };
        assert_eq!(narrow_values.len(), 512);
        assert_eq!(f32_at(&narrow_values, 8), Some(1.0));
        assert_eq!(f32_at(&narrow_values, 12), Some(0.0));
        assert_eq!(f32_at(&narrow_values, 264), Some(1.5));
        let mixed = SlabImage {
            width: 1,
            height: 1,
            step_mm: 1.0,
            index_step: [0.0, 0.0, 1.0],
            sample_count: 2,
            samples: vec![PlaneSample::Present(9.0), PlaneSample::Absent],
        };
        let Ok(mixed_values) = slab_value_bytes(&mixed, 1, 256) else {
            unreachable!("one texel fits");
        };
        let Ok(mixed_mask) = slab_mask_bytes(&mixed, 1, 256) else {
            unreachable!("one texel fits");
        };
        assert_eq!(f32_at(&mixed_values, 0), Some(0.0));
        assert_eq!(f32_at(&mixed_values, 256), Some(0.0));
        assert_eq!(f32_at(&mixed_mask, 0), Some(0.0));
    }

    fn any_volume() -> VolumePass {
        VolumePass {
            blend: BlendMode::Composite,
            voi: crate::voi::VoiParams {
                center: 0.0,
                width: 1.0,
                slope: 1.0,
                intercept: 0.0,
                ymin: 0.0,
                ymax: 255.0,
                fn_kind: 1,
                invert: 0,
            },
            axis: RayAxis::Slice,
            from_far: false,
        }
    }

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
    fn pipeline_lookup_never_returns_another_tiers_pipeline() {
        let keys = [CompiledKey {
            key: PipelineKey {
                pass: PassKind::Stack,
                blend: BlendMode::Composite,
                tier: Tier::A,
            },
            family: PipelineFamily::Fragment,
        }];
        assert_eq!(
            pipeline_index(
                &keys,
                &PipelineKey {
                    pass: PassKind::Stack,
                    blend: BlendMode::Composite,
                    tier: Tier::A,
                },
                PipelineFamily::Fragment,
            ),
            Some(0)
        );
        assert_eq!(
            pipeline_index(
                &keys,
                &PipelineKey {
                    pass: PassKind::Stack,
                    blend: BlendMode::Composite,
                    tier: Tier::B,
                },
                PipelineFamily::Fragment,
            ),
            None
        );
    }

    /// A compute or storage request is not the fragment pipeline.
    ///
    /// Section 7: tier B is fragment shaders only. The lookup keeps the
    /// family, so index 0 is not returned for either of the other two.
    #[test]
    fn a_compute_or_storage_key_is_not_the_fragment_pipeline() {
        let key = PipelineKey {
            pass: PassKind::Stack,
            blend: BlendMode::Composite,
            tier: Tier::B,
        };
        let keys = [CompiledKey {
            key,
            family: PipelineFamily::Fragment,
        }];
        assert_eq!(
            pipeline_index(&keys, &key, PipelineFamily::Fragment),
            Some(0)
        );
        assert_eq!(pipeline_index(&keys, &key, PipelineFamily::Compute), None);
        assert_eq!(pipeline_index(&keys, &key, PipelineFamily::Storage), None);
        assert!(RenderGraph::pipeline_compiled(&key));
        assert!(RenderGraph::pipeline_family_compiled(
            &key,
            PipelineFamily::Fragment
        ));
        assert!(!RenderGraph::pipeline_family_compiled(
            &key,
            PipelineFamily::Compute
        ));
        assert!(!RenderGraph::pipeline_family_compiled(
            &key,
            PipelineFamily::Storage
        ));
        let viewport = ViewportId(3);
        assert_eq!(
            RenderGraph::request_outcome(&key, PipelineFamily::Compute, viewport),
            FrameOutcome::Unavailable { viewport }
        );
        assert_eq!(
            RenderGraph::request_outcome(&key, PipelineFamily::Storage, viewport),
            FrameOutcome::Unavailable { viewport }
        );
        assert!(!matches!(
            RenderGraph::request_outcome(&key, PipelineFamily::Fragment, viewport),
            FrameOutcome::Unavailable { .. }
        ));
    }

    /// PS3.3 values stay `f32` after `textureLoad`. `Rgba8Unorm` is not a
    /// candidate. Tier A does not substitute when `R32Float` cannot be sampled.
    #[test]
    fn preserving_format_keeps_r32_and_refuses_a_tier_a_substitute() {
        let features = [sampleable(), sampleable(), sampleable()];
        assert_eq!(
            Upload::preserving_format(Tier::A, features),
            Ok(wgpu::TextureFormat::R32Float)
        );
        assert_eq!(
            Upload::preserving_format(Tier::B, features),
            Ok(wgpu::TextureFormat::R32Float)
        );
        let missing = [unsampleable(), sampleable(), sampleable()];
        assert_eq!(
            Upload::preserving_format(Tier::A, missing),
            Err(UploadError::FormatUnavailable)
        );
        assert_eq!(
            Upload::preserving_format(Tier::B, missing),
            Ok(wgpu::TextureFormat::Rg32Float)
        );
        let rgba_only = [unsampleable(), unsampleable(), sampleable()];
        assert_eq!(
            Upload::preserving_format(Tier::B, rgba_only),
            Ok(wgpu::TextureFormat::Rgba32Float)
        );
        let none = [unsampleable(), unsampleable(), unsampleable()];
        assert_eq!(
            Upload::preserving_format(Tier::B, none),
            Err(UploadError::FormatUnavailable)
        );
        let chosen = Upload::preserving_format(Tier::B, missing);
        assert!(chosen.is_ok());
        assert_ne!(chosen.ok(), Some(wgpu::TextureFormat::Rgba8Unorm));
    }

    #[test]
    fn removal_clears_the_slot_and_reregistration_reuses_it_fully_dirty() {
        let mut registry = Registry::new(1);
        let first = registry.register(Pass::VolumeRaycast(any_volume()));
        assert!(first.is_ok());
        if let Ok(id) = first {
            assert!(registry.remove(id));
            assert!(!registry.remove(id));
        }
        let again = registry.register(Pass::VolumeRaycast(any_volume()));
        assert!(matches!(again, Ok(id) if id.0 == 0));
        if let Ok(id) = again {
            let state = registry
                .states
                .get(usize::try_from(id.0).unwrap_or(usize::MAX))
                .and_then(Option::as_ref);
            assert!(matches!(state, Some(SlotState { dirty, .. }) if *dirty == Dirty::full()));
        }
    }

    #[test]
    fn registry_refuses_beyond_capacity_and_starts_fully_dirty() {
        let mut registry = Registry::new(1);
        let first = registry.register(Pass::VolumeRaycast(any_volume()));
        assert!(first.is_ok());
        let second = registry.register(Pass::VolumeRaycast(any_volume()));
        assert!(second.is_err());
        if let Ok(id) = first {
            let state = registry
                .states
                .get(usize::try_from(id.0).unwrap_or(usize::MAX))
                .and_then(Option::as_ref);
            assert!(matches!(state, Some(SlotState { dirty, .. }) if *dirty == Dirty::full()));
        }
    }
}
