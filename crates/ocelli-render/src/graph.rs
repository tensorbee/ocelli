//! HLD section 22's render graph and frame scheduler, F-038.
//!
//! The graph registers viewports, tracks dirt per viewport, compiles every
//! pipeline it will use at creation, encodes each dirty viewport once per
//! frame into one command encoder, and submits that encoder exactly once.
//!
//! **No surface is acquired and no texture is uploaded.** Presentation is
//! F-039 and upload is F-040. A viewport renders into a caller-supplied
//! texture view, from a caller-supplied stored-value texture, so the oracle
//! still has no Ocelli frame to judge until F-040. Decision D7 stays open.

use ocelli_pixel::LutChain;
use wgpu::util::DeviceExt;

use crate::caps::Tier;
use crate::gpu::{GpuContext, Recovered};
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
/// mode and tier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PipelineKey {
    /// The pass kind.
    pub pass: PassKind,
    /// The blend mode.
    pub blend: BlendMode,
    /// The resolved tier the pipeline was compiled for.
    pub tier: Tier,
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
    /// Nothing was dirty. No encoder was created and nothing was submitted.
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
}

/// The fragment entry appended to `VOI_WGSL` at module build.
///
/// Group 0 binding 0 is section 18.4's and stays reserved by `voi.wgsl`.
/// Group 1 binding 0 is the caller's stored-value texture, read with
/// `textureLoad`, which is the tier B legal fragment path: no sampler state
/// decides anything and no compute or storage buffer appears.
const GRAPH_WGSL: &str = r"
@group(1) @binding(0) var stored_tex : texture_2d<f32>;

@vertex
fn vs_main(@builtin(vertex_index) index : u32) -> @builtin(position) vec4<f32> {
    let u = f32((index << 1u) & 2u);
    let v = f32(index & 2u);
    return vec4<f32>(u * 2.0 - 1.0, v * 2.0 - 1.0, 0.0, 1.0);
}

@fragment
fn fs_main(@builtin(position) at : vec4<f32>) -> @location(0) vec4<f32> {
    let texel = vec2<i32>(i32(at.x), i32(at.y));
    let stored = textureLoad(stored_tex, texel, 0).r;
    let display = lut_chain(stored);
    // Normalise the display value into the colour target range. The full
    // declared output range maps to 0 to 1 whatever its magnitude, because
    // the target is a colour and the range is a value scale, and a zero range
    // has no normalisation, so it writes black rather than dividing by zero.
    let range = voi.ymax - voi.ymin;
    let unit = select(0.0, (display - voi.ymin) / range, range > 0.0);
    return vec4<f32>(unit, unit, unit, 1.0);
}
";

/// One registered viewport's decision state.
struct SlotState {
    dirty: Dirty,
    pass: Pass,
}

/// One registered viewport's GPU resources, created at registration.
struct GpuSlot {
    target: wgpu::TextureView,
    uniform: wgpu::Buffer,
    bind_uniform: wgpu::BindGroup,
    bind_source: wgpu::BindGroup,
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

/// Find a compiled pipeline by its full key, tier included.
///
/// A free function over the key list so the lookup rule is testable with no
/// adapter. Dropping `tier` from the comparison returns another tier's
/// pipeline, and the unit test beside this function goes red on that.
fn pipeline_index(keys: &[PipelineKey], key: &PipelineKey) -> Option<usize> {
    keys.iter().position(|held| held == key)
}

/// The render graph: registered viewports, their dirt, and the pipelines
/// compiled at creation.
pub struct RenderGraph {
    registry: Registry,
    gpu: Vec<Option<GpuSlot>>,
    keys: Vec<PipelineKey>,
    pipelines: Vec<wgpu::RenderPipeline>,
    uniform_layout: wgpu::BindGroupLayout,
    source_layout: wgpu::BindGroupLayout,
    format: wgpu::TextureFormat,
    tier: Tier,
    submissions: u64,
}

/// The one pass and blend this story gives a shader body. `compile` builds
/// exactly this, and `pipeline_compiled` answers from it, so the two cannot
/// drift apart when a later story adds a body.
const COMPILED_PASS: PassKind = PassKind::Stack;
const COMPILED_BLEND: BlendMode = BlendMode::Composite;

impl RenderGraph {
    /// Whether this story compiles a pipeline for a key.
    ///
    /// Only the composite stack pipeline has a shader body, at any tier.
    /// Every other key is declared, and the frame path reports it unavailable.
    pub fn pipeline_compiled(key: &PipelineKey) -> bool {
        key.pass == COMPILED_PASS && key.blend == COMPILED_BLEND
    }

    /// How many frame submissions this graph has issued.
    ///
    /// One per submitted tick, across all viewports. This is the observable
    /// half of section 22's one-submit rule: a tick that submitted per
    /// viewport would count its viewports instead of one.
    pub fn submissions(&self) -> u64 {
        self.submissions
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
        Ok(Self {
            registry: Registry::new(capacity),
            gpu,
            keys,
            pipelines,
            uniform_layout,
            source_layout,
            format,
            tier,
            submissions: 0,
        })
    }

    fn compile(
        ctx: &GpuContext,
        tier: Tier,
        format: wgpu::TextureFormat,
    ) -> (
        wgpu::BindGroupLayout,
        wgpu::BindGroupLayout,
        Vec<PipelineKey>,
        Vec<wgpu::RenderPipeline>,
    ) {
        let device = ctx.device();
        let source = format!("{VOI_WGSL}{GRAPH_WGSL}");
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("ocelli render graph"),
            source: wgpu::ShaderSource::Wgsl(source.into()),
        });
        let uniform_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ocelli graph uniform"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });
        let source_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("ocelli graph source"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: false },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            }],
        });
        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("ocelli render graph"),
            bind_group_layouts: &[Some(&uniform_layout), Some(&source_layout)],
            immediate_size: 0,
        });
        let key = PipelineKey {
            pass: COMPILED_PASS,
            blend: COMPILED_BLEND,
            tier,
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

    fn pipeline(&self, key: &PipelineKey) -> Option<&wgpu::RenderPipeline> {
        pipeline_index(&self.keys, key).and_then(|index| self.pipelines.get(index))
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
        let voi = match &pass {
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
        };
        let id = self.registry.register(pass)?;
        let slot = self.create_gpu_slot(ctx, target, source, voi);
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
        voi: VoiParams,
    ) -> GpuSlot {
        let device = ctx.device();
        let uniform = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("ocelli graph voi"),
            contents: bytemuck::bytes_of(&voi),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let bind_uniform = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ocelli graph voi"),
            layout: &self.uniform_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform.as_entire_binding(),
            }],
        });
        let bind_source = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("ocelli graph source"),
            layout: &self.source_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&source),
            }],
        });
        GpuSlot {
            target,
            uniform,
            bind_uniform,
            bind_source,
        }
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
        if let Some(slot) = self
            .gpu
            .get_mut(usize::try_from(id.0).unwrap_or(usize::MAX))
        {
            *slot = None;
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
    pub fn rebind(
        &mut self,
        ctx: &GpuContext,
        id: ViewportId,
        target: wgpu::TextureView,
        source: wgpu::TextureView,
    ) -> Result<(), GraphError> {
        let voi = {
            let state = self
                .registry
                .states
                .get(usize::try_from(id.0).unwrap_or(usize::MAX))
                .and_then(Option::as_ref)
                .ok_or(GraphError::UnknownViewport)?;
            match state.pass {
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
        };
        let slot = self.create_gpu_slot(ctx, target, source, voi);
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

    /// Reinstall the resources a recovery rebuilt, marking every viewport
    /// fully dirty because every GPU object the old device owned is gone.
    pub fn install_recovered(&mut self, ctx: &GpuContext, recovered: &Recovered) {
        // `recovered.graph` reports what the rebuild counted, and a graph
        // that registered nothing there still cannot keep this graph's
        // per-viewport objects, which belonged to the dead device all the
        // same. There is no silent return: installing always drops them.
        // The rebuilt tier is the recovered one, because a fallback adapter
        // may resolve differently from the device that was lost.
        self.tier = recovered.caps.tier;
        let (uniform_layout, source_layout, keys, pipelines) =
            Self::compile(ctx, self.tier, self.format);
        self.uniform_layout = uniform_layout;
        self.source_layout = source_layout;
        self.keys = keys;
        self.pipelines = pipelines;
        // Every per-viewport object was created on the dead device: the
        // uniform buffers, both bind groups and the caller's views as bound.
        // They are dropped here, not carried into a pass on the new device.
        // The registry keeps each pass and its last uniform values, the
        // caller re-supplies views through `rebind`, and a viewport with no
        // rebound slot reports unavailable in the tick and keeps its dirt.
        for slot in self.gpu.iter_mut() {
            *slot = None;
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
    pub fn tick(&mut self, ctx: &GpuContext) -> FrameOutcome {
        if matches!(ctx.state(), crate::gpu::DeviceState::Lost(_)) {
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
            if self.pipeline(&key).is_some() && rebound {
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
        // Encode only viewports whose pipeline exists. Unavailable ones keep
        // their dirt, checked again below when clearing.
        let mut encoder = ctx
            .device()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("ocelli render graph frame"),
            });
        let mut encoded: u32 = 0;
        for index in 0..self.registry.states.len() {
            let key = {
                let Some(state) = self.registry.states.get(index).and_then(Option::as_ref) else {
                    continue;
                };
                if !state.dirty.any() {
                    continue;
                }
                PipelineKey {
                    pass: state.pass.kind(),
                    blend: state.pass.blend(),
                    tier: self.tier,
                }
            };
            let Some(pipeline) = self.pipeline(&key) else {
                continue;
            };
            let Some(slot) = self.gpu.get(index).and_then(Option::as_ref) else {
                continue;
            };
            {
                let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("ocelli render graph pass"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: &slot.target,
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
            if let Some(state) = self.registry.states.get_mut(index).and_then(Option::as_mut) {
                state.dirty.clear();
            }
        }
        if encoded == 0 {
            return match first_unavailable {
                Some(viewport) => FrameOutcome::Unavailable { viewport },
                None => FrameOutcome::Noop,
            };
        }
        ctx.queue().submit(std::iter::once(encoder.finish()));
        self.submissions += 1;
        FrameOutcome::Submitted {
            viewports: encoded,
            passes: encoded,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Dirty, Registry, SlotState, pipeline_index};
    use crate::caps::Tier;
    use crate::graph::{BlendMode, Pass, PassKind, PipelineKey, VolumePass};

    #[test]
    fn pipeline_lookup_never_returns_another_tiers_pipeline() {
        let keys = [PipelineKey {
            pass: PassKind::Stack,
            blend: BlendMode::Composite,
            tier: Tier::A,
        }];
        assert_eq!(
            pipeline_index(
                &keys,
                &PipelineKey {
                    pass: PassKind::Stack,
                    blend: BlendMode::Composite,
                    tier: Tier::A,
                }
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
                }
            ),
            None
        );
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
