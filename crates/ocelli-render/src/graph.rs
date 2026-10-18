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
//! counts the brick upload. F-045 writes the sixteen camera floats at group 0
//! binding 2. `set_view` does not write the VOI uniform. The oracle still has
//! no Ocelli frame to judge. Decision D7 stays open.

use ocelli_cache::{Brick, BrickKey};
use ocelli_pixel::LutChain;
use wgpu::util::DeviceExt;

use crate::brick_upload::{BrickResidency, BrickUploadError, brick_format};
use crate::caps::Tier;
use crate::colour::{ColourLut, ColourParams};
use crate::gpu::{GpuContext, Recovered, SurfaceError, first_render_attachment};
use crate::probe::ResolvedAdapter;
use crate::voi::{VOI_WGSL, VoiParams, VoiParamsError};

/// HLD section 22's pass enum, exactly as transcribed.
pub enum Pass {
    /// A stack viewport, windowed through section 18.4's uniform.
    Stack(StackPass),
    /// A volume raycast viewport.
    VolumeRaycast(VolumePass),
    /// A segmentation overlay, scheduled after its stack pass.
    SegOverlay(SegPass),
}

impl Pass {
    /// Which of section 22's three pass kinds this is.
    pub fn kind(&self) -> PassKind {
        match self {
            Self::Stack(_) => PassKind::Stack,
            Self::VolumeRaycast(_) => PassKind::VolumeRaycast,
            Self::SegOverlay(_) => PassKind::SegOverlay,
        }
    }

    /// The blend mode this pass schedules under.
    pub fn blend(&self) -> BlendMode {
        match self {
            Self::Stack(pass) => pass.blend,
            Self::VolumeRaycast(pass) => pass.blend,
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

/// The volume raycast payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VolumePass {
    /// The blend mode the pipeline is keyed by.
    pub blend: BlendMode,
}

/// The segmentation overlay payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SegPass {
    /// The blend mode the pipeline is keyed by.
    pub blend: BlendMode,
}

/// The five blend modes the parity surface counts. Only `Composite` has a
/// shader body in this story. The rest are cache keys, and a lookup for one
/// reports unavailable rather than substituting composite, which is deviation
/// D-07's honesty rule applied to a blend mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BlendMode {
    /// The ordinary composite.
    Composite,
    /// Maximum intensity projection. No body until F-052 and F-054.
    Mip,
    /// Minimum intensity projection. No body yet.
    MinIp,
    /// Average intensity projection. No body yet.
    Average,
    /// Labelmap edge projection. No body yet.
    LabelmapEdgeProjection,
}

/// Section 22's three pass kinds, as a cache-key field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PassKind {
    /// The stack pass.
    Stack,
    /// The volume raycast pass.
    VolumeRaycast,
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
    /// A VOI chain with no section 18.4 uniform.
    #[error("{0}")]
    Voi(#[from] VoiParamsError),
    /// Graph creation on tier C, which holds no device and reports unavailable.
    #[error("the render graph is unavailable on tier C, which holds no device")]
    TierCUnavailable,
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
    /// `Rgba8Unorm` cannot be sampled and copied, or the table has no texels.
    #[error("the colour LUT texture cannot be sampled and copied")]
    ColourUnavailable,
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
    pub fn stored(
        features: wgpu::TextureFormatFeatures,
        width: u32,
        height: u32,
    ) -> Result<Self, UploadError> {
        let upload = Self::from_extent(width, height)?;
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

    fn from_extent(width: u32, height: u32) -> Result<Self, UploadError> {
        if width == 0 || height == 0 {
            return Err(UploadError::EmptyExtent);
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
fn stored_float_channels(format: wgpu::TextureFormat) -> Option<u32> {
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
/// value. Mode 0 is the greyscale normalisation, and it still emits the
/// unclamped display value.
const GRAPH_WGSL: &str = r"
struct ColourParams {
    mode : u32,
    origin : i32,
    last : u32,
    pad : u32,
};

@group(0) @binding(1) var<uniform> colour : ColourParams;
@group(0) @binding(2) var<uniform> camera : mat4x4<f32>;
@group(1) @binding(0) var stored_tex : texture_2d<f32>;
@group(1) @binding(1) var colour_lut : texture_2d<f32>;

// Same address as `display_index` in `colour.rs`. The value is not modified.
// A non-finite value uses texel 0. Values at or below 0 use texel 0.
// Values at or above `last` use texel `last`. Anything between truncates
// toward zero, which is WGSL's `i32(value)` on this range.
fn table_index(value : f32, last : u32) -> i32 {
    if ((value - value) != 0.0) {
        return 0;
    }
    if (!(value > 0.0)) {
        return 0;
    }
    if (value >= f32(last)) {
        return i32(last);
    }
    return i32(value);
}

@vertex
fn vs_main(@builtin(vertex_index) index : u32) -> @builtin(position) vec4<f32> {
    let u = f32((index << 1u) & 2u);
    let v = f32(index & 2u);
    return vec4<f32>(u * 2.0 - 1.0, v * 2.0 - 1.0, 0.0, 1.0);
}

@fragment
fn fs_main(@builtin(position) at : vec4<f32>) -> @location(0) vec4<f32> {
    let mapped = camera * vec4<f32>(at.x, at.y, 0.0, 1.0);
    let dims = textureDimensions(stored_tex);
    // A negative coordinate has to be rejected before `i32` truncation.
    // Truncation toward zero would otherwise select texel 0.
    if (mapped.x < 0.0 || mapped.y < 0.0 || mapped.x >= f32(dims.x) || mapped.y >= f32(dims.y)) {
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }
    let texel = vec2<i32>(i32(mapped.x), i32(mapped.y));
    let stored = textureLoad(stored_tex, texel, 0).r;
    if (colour.mode == 2u) {
        let index = table_index(stored - f32(colour.origin), colour.last);
        return textureLoad(colour_lut, vec2<i32>(index, 0), 0);
    }
    let display = lut_chain(stored);
    if (colour.mode == 1u) {
        let index = table_index(display, colour.last);
        return textureLoad(colour_lut, vec2<i32>(index, 0), 0);
    }
    // Normalise the display value into the colour target range. The full
    // declared output range maps to 0 to 1 whatever its magnitude, because
    // the target is a colour and the range is a value scale, and a zero range
    // has no normalisation, so it writes black rather than dividing by zero.
    // This path does not clamp `display`. A LINEAR body outside `[ymin, ymax]`
    // stays outside it.
    let range = voi.ymax - voi.ymin;
    let unit = select(0.0, (display - voi.ymin) / range, range > 0.0);
    return vec4<f32>(unit, unit, unit, 1.0);
}
";

/// The fragment module tier B compiles, and the module tier A compiles too.
///
/// `VOI_WGSL` is included unchanged. The three window functions are not
/// re-typed here. The entry point samples a float texture with `textureLoad`
/// and has no compute entry and no storage buffer.
pub fn tier_b_fragment_module() -> String {
    format!("{VOI_WGSL}{GRAPH_WGSL}")
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
    /// Extent of the owned stored texture, remembered across a device rebuild.
    stored: Option<Upload>,
    /// The preserving format chosen for that texture. Absent when the
    /// viewport has no owned stored texture.
    stored_format: Option<wgpu::TextureFormat>,
    /// The colour table selected for this viewport. Absent until a selection.
    colour: Option<HeldColour>,
    /// Column-major camera. Identity until `set_view`.
    camera: [f32; 16],
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
                    stored: None,
                    stored_format: None,
                    colour: None,
                    camera: IDENTITY_CAMERA,
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
    format: wgpu::TextureFormat,
    tier: Tier,
    submissions: u64,
    /// Successful `upload_stored` calls. `set_voi` does not touch this.
    uploads: u64,
    /// Successful `select_colour_lut` calls. `set_voi` does not touch this.
    lut_uploads: u64,
    /// One 3D texture per uploaded brick. `set_voi` does not touch this.
    bricks: BrickResidency,
}

/// The one pass and blend this story gives a shader body. `compile` builds
/// exactly this, and `pipeline_compiled` answers from it, so the two cannot
/// drift apart when a later story adds a body.
const COMPILED_PASS: PassKind = PassKind::Stack;
const COMPILED_BLEND: BlendMode = BlendMode::Composite;

impl RenderGraph {
    /// Whether this story compiles a fragment pipeline for a key.
    ///
    /// Only the composite stack pipeline has a shader body, at any tier.
    /// Every other key is declared, and the frame path reports it unavailable.
    /// A compute or storage request is [`RenderGraph::pipeline_family_compiled`],
    /// which is false on tier B and is not this predicate.
    pub fn pipeline_compiled(key: &PipelineKey) -> bool {
        key.pass == COMPILED_PASS && key.blend == COMPILED_BLEND
    }

    /// Whether `family` has a compiled body for `key`.
    ///
    /// Fragment is the composite stack body. Compute and storage have no body
    /// in this story, on either GPU tier. On tier B that refusal is the
    /// section 7 rule: the fragment pipeline is not returned in their place.
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
        if !matches!(state.pass, Pass::Stack(_)) {
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
        let (uniform_layout, source_layout, keys, pipelines) = Self::compile(ctx, tier, format);
        let restored = u32::try_from(pipelines.len()).unwrap_or(0);
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
            keys,
            pipelines,
            uniform_layout,
            source_layout,
            format,
            tier,
            submissions: 0,
            uploads: 0,
            lut_uploads: 0,
            bricks: BrickResidency::new(),
        })
    }

    fn compile(
        ctx: &GpuContext,
        tier: Tier,
        format: wgpu::TextureFormat,
    ) -> (
        wgpu::BindGroupLayout,
        wgpu::BindGroupLayout,
        Vec<CompiledKey>,
        Vec<wgpu::RenderPipeline>,
    ) {
        let device = ctx.device();
        // Tier B's compiled module is this fragment text. Tier A compiles the
        // same text. A compute module is not built for either tier.
        let source = tier_b_fragment_module();
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ocelli render graph"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
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
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ocelli render graph"),
            bind_group_layouts: &[Some(&uniform_layout), Some(&source_layout)],
            immediate_size: 0,
        });
        let key = CompiledKey {
            key: PipelineKey {
                pass: COMPILED_PASS,
                blend: COMPILED_BLEND,
                tier,
            },
            family: PipelineFamily::Fragment,
        };
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("ocelli render graph composite"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &module,
                entry_point: Some("vs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[],
            },
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module: &module,
                entry_point: Some("fs_main"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: None,
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            multiview_mask: None,
            cache: None,
        });
        (uniform_layout, source_layout, vec![key], vec![pipeline])
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
        let slot = self.create_gpu_slot(ctx, target, source, (voi, IDENTITY_CAMERA), None, None);
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
    /// and written. [`UploadError::EmptyExtent`] or
    /// [`UploadError::ExtentOverflow`] for an extent that cannot be a texture.
    /// [`UploadError::CapacityExhausted`] beyond the capacity fixed at
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
        let upload = Upload::from_extent(width, height)?;
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
            (voi, IDENTITY_CAMERA),
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
        uniforms: (VoiParams, [f32; 16]),
        stored: Option<StoredTexture>,
        colour: Option<HeldColour>,
    ) -> GpuSlot {
        let (voi, camera) = uniforms;
        let device = ctx.device();
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
            contents: bytemuck::cast_slice(&camera),
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
        let (voi, camera, colour) = {
            let state = self
                .registry
                .states
                .get(usize::try_from(id.0).unwrap_or(usize::MAX))
                .and_then(Option::as_ref)
                .ok_or(GraphError::UnknownViewport)?;
            if state.stored.is_some() {
                return Err(GraphError::StoredNeedsRebind);
            }
            (voi_of(&state.pass), state.camera, state.colour.clone())
        };
        let slot = self.create_gpu_slot(ctx, target, source, (voi, camera), None, colour);
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
    /// search.
    pub fn rebind_stored(
        &mut self,
        ctx: &GpuContext,
        adapter: &ResolvedAdapter,
        id: ViewportId,
        target: wgpu::TextureView,
    ) -> Result<(), UploadError> {
        let (upload, voi, camera, format, colour) = {
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
                format,
                state.colour.clone(),
            )
        };
        let features = adapter.wgpu_adapter().get_texture_format_features(format);
        if !binding_available(features) {
            return Err(UploadError::FormatUnavailable);
        }
        let texture = create_stored_texture(ctx.device(), upload, format);
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let stored = StoredTexture {
            texture,
            width: upload.width(),
            height: upload.height(),
            format,
        };
        let slot = self.create_gpu_slot(ctx, target, view, (voi, camera), Some(stored), colour);
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
            if let Pass::Stack(stack) = &mut state.pass {
                stack.voi = params;
            }
            state.dirty.voi = true;
        }
        Ok(())
    }

    /// Write the sixteen camera floats and mark the camera dirty.
    ///
    /// The bytes are HLD section 17.4's `camera` field, column-major. This
    /// writes the 64-byte uniform at group 0 binding 2. It does not write
    /// the VOI uniform, the colour uniform, the stored texture or a brick,
    /// and it does not increment [`RenderGraph::uploads`],
    /// [`RenderGraph::lut_uploads`] or [`RenderGraph::brick_uploads`].
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
    /// recreated the slot.
    pub fn set_view(
        &mut self,
        ctx: &GpuContext,
        id: ViewportId,
        camera: [f32; 16],
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
        let slot = self
            .gpu
            .get(usize::try_from(id.0).unwrap_or(usize::MAX))
            .and_then(Option::as_ref)
            .ok_or(GraphError::ViewportNotRebound)?;
        ctx.queue()
            .write_buffer(&slot.camera, 0, bytemuck::cast_slice(&camera));
        if let Some(state) = self.registry.state_mut(id) {
            state.camera = camera;
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
    /// `Rgba8Unorm` cannot be sampled and copied, or the table has no texels.
    /// Nothing is counted in that case.
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
        let Some(width) = colour_texel_count(bytes) else {
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
    /// stays.
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
        let upload = Upload::from_extent(width, height)?;
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
        self.bricks.upload(ctx, format, features, key, brick)
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
        let (uniform_layout, source_layout, keys, pipelines) =
            Self::compile(ctx, self.tier, self.format);
        self.uniform_layout = uniform_layout;
        self.source_layout = source_layout;
        self.keys = keys;
        self.pipelines = pipelines;
        // Every per-viewport object was created on the dead device: the
        // uniform buffers, both bind groups, the stored-sample texture, the
        // colour table texture and the caller's views as bound.
        // They are dropped here, not carried into a pass on the new device.
        // The registry keeps each pass, its last uniform values, the camera
        // floats, the stored extent and the colour table bytes. The caller re-supplies a
        // caller-owned source through `rebind`, or recreates an owned texture
        // through `rebind_stored`. Either restore writes the colour bytes
        // again and does not increment `lut_uploads`.
        // Surfaces were configured on the dead device. The frame texture is
        // dropped before the surface it came from. The shell still holds the
        // canvases and supplies them again through `bind_surface`.
        // A viewport with no rebound slot reports unavailable in the tick
        // and keeps its dirt.
        // Brick textures were created on the dead device. They are dropped
        // here and `brick_uploads` is not incremented. The caller supplies
        // the bricks again through `upload_brick`.
        for slot in self.gpu.iter_mut() {
            *slot = None;
        }
        self.bricks.clear();
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
            let rebound = self.gpu.get(index).and_then(Option::as_ref).is_some();
            if self.pipeline(&key, PipelineFamily::Fragment).is_some() && rebound {
                any_encodable = true;
            } else if first_unavailable.is_none() {
                first_unavailable = Some(ViewportId(u32::try_from(index).unwrap_or(u32::MAX)));
            }
        }
        if !any_encodable {
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
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    ..Default::default()
                });
                pass.set_pipeline(pipeline);
                pass.set_bind_group(0, &slot.bind_uniform, &[]);
                pass.set_bind_group(1, &slot.bind_source, &[]);
                pass.draw(0..3, 0..1);
            }
            encoded += 1;
            if let Some(state) = registry.states.get_mut(index).and_then(Option::as_mut) {
                state.dirty.clear();
            }
        }
        if encoded == 0 {
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
            passes: encoded,
        }
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
    let rebound = gpu.get(index).and_then(Option::as_ref).is_some();
    compiled && rebound
}

fn voi_of(pass: &Pass) -> VoiParams {
    match pass {
        Pass::Stack(stack) => stack.voi,
        _ => VoiParams {
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

fn colour_texel_count(bytes: &[u8]) -> Option<u32> {
    if bytes.is_empty() || !bytes.len().is_multiple_of(4) {
        return None;
    }
    u32::try_from(bytes.len() / 4).ok()
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
    if let Some((held, width)) =
        held.and_then(|held| colour_texel_count(&held.bytes).map(|width| (held, width)))
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
    use super::{CompiledKey, Dirty, Registry, SlotState, pipeline_index};
    use crate::caps::Tier;
    use crate::graph::{
        BlendMode, FrameOutcome, Pass, PassKind, PipelineFamily, PipelineKey, RenderGraph, Upload,
        UploadError, ViewportId, VolumePass,
    };

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
        let first = registry.register(Pass::VolumeRaycast(VolumePass {
            blend: BlendMode::Composite,
        }));
        assert!(first.is_ok());
        if let Ok(id) = first {
            assert!(registry.remove(id));
            assert!(!registry.remove(id));
        }
        let again = registry.register(Pass::VolumeRaycast(VolumePass {
            blend: BlendMode::Composite,
        }));
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
        let first = registry.register(Pass::VolumeRaycast(VolumePass {
            blend: BlendMode::Composite,
        }));
        assert!(first.is_ok());
        let second = registry.register(Pass::VolumeRaycast(VolumePass {
            blend: BlendMode::Composite,
        }));
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
