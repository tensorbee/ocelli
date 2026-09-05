//! Resolved device capabilities and the rendering tier.
//!
//! HLD section 22 gives `Caps` field for field. Deviation D-07 adds the third
//! tier variant.
//!
//! F-008 defined the type. **F-004 added the decision procedure that fills
//! it**, which is everything from `SoftwareVerdict` downwards. Device creation
//! and loss recovery are still F-037, which is E6.1 in S11,
//! "ocelli-render: device init, capability tiering, device-lost recovery".
//! F-039 is E6.3 in S13 and is OffscreenCanvas.
//!
//! **This module makes the decision and touches no GPU.** The wgpu calls and
//! the fill-rate workload live in `probe.rs`, and the split is the point:
//! everything that can be WRONG about a tier is here, and this half needs no
//! adapter to test, which is what makes it exhaustively testable in the CI
//! floor that deviation D-04 leaves us with.

/// The rendering tier a session resolved to.
///
/// HLD section 7 gives two, both GPU. `Cpu` is deviation **D-07**, because
/// section 7 leaves a machine with neither WebGPU nor WebGL2 rendering nothing
/// at all.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Tier {
    /// WebGPU. Compute shaders, storage buffers, 3D textures to 2048.
    A,
    /// WebGL2 through wgpu's downlevel profile. Fragment shaders only, no
    /// compute, no storage buffers, a conservative 3D-texture floor of 256.
    B,
    /// CPU. Deviation D-07. A stack viewport renders, windows, scrolls and
    /// measures, reusing `ocelli-pixel` rather than reimplementing the LUT
    /// chain.
    Cpu,
}

impl Tier {
    /// Whether this tier can run a compute shader at all.
    ///
    /// Only tier A can. Section 7: "Anything wanting compute - GPU
    /// segmentation, histogram passes, compute-based resampling - is tier A
    /// only and must degrade, not fail."
    #[must_use]
    pub fn supports_compute(self) -> bool {
        matches!(self, Tier::A)
    }

    /// Parse an operator override.
    ///
    /// `a`, `b`, `cpu` and `auto`, case-insensitively and with surrounding
    /// whitespace trimmed, because an environment variable and a query
    /// parameter both pick it up. `auto` and an empty value both mean no
    /// override, so a deployment can set the variable unconditionally.
    ///
    /// Anything else is [`TierRequest::Unrecognised`], which is refused and
    /// recorded rather than quietly treated as `auto`.
    ///
    /// **This function reads no environment of its own.** Natively the value
    /// is `OCELLI_TIER`, read by `ocelli-native`'s two entry points. In a
    /// browser it comes from the shell, because reading a query parameter, a
    /// config endpoint or `localStorage` is DOM work and HLD section 10 puts
    /// DOM work in TypeScript.
    #[must_use]
    pub fn from_override_str(raw: &str) -> TierRequest {
        match raw.trim().to_ascii_lowercase().as_str() {
            "" | "auto" => TierRequest::Auto,
            "a" => TierRequest::Requested(Tier::A),
            "b" => TierRequest::Requested(Tier::B),
            "cpu" => TierRequest::Requested(Tier::Cpu),
            _ => TierRequest::Unrecognised,
        }
    }
}

/// HLD section 22, verbatim in its fields.
///
/// ```text
/// pub struct Caps {
///     pub compute: bool,
///     pub max_tex_3d: u32,
///     pub max_buffer: u64,
///     pub tier: Tier, // A = WebGPU, B = WebGL2 downlevel
/// }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Caps {
    /// Whether compute shaders are available.
    pub compute: bool,
    /// The largest 3D texture dimension the adapter reports.
    pub max_tex_3d: u32,
    /// The largest buffer the adapter guarantees.
    pub max_buffer: u64,
    /// The resolved tier.
    pub tier: Tier,
}

// ---------------------------------------------------------------------------
// F-004, the detection half.
//
// HLD section 7 says only "the tier resolves once at startup", and deviation
// D-07 says that resolving from what the platform reports is precisely the
// defect: on a host with no GPU a software rasteriser presents a conforming
// WebGL2 context, section 7's logic resolves tier B, and Ocelli then runs GPU
// paths on a rasteriser slower than its own CPU path. It is invisible, and it
// presents as "the viewer is slow" rather than as a misdetection.
//
// So the resolution combines three signals and states the combination rule
// explicitly, rather than leaving it to the order of an `if` chain.
// `docs/lld/tier-resolution.md` is the prose version of what follows.
// ---------------------------------------------------------------------------

/// Whether a signal says the chosen adapter is real hardware or a software
/// rasteriser.
///
/// `Unknown` is a first-class answer rather than a failure.
/// `docs/spikes/A7-tier-c.md` lists the detection signals and says "None is
/// sufficient alone, so treat this as evidence to combine", and a signal that
/// picks a side it has no evidence for is worse than one that abstains.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoftwareVerdict {
    /// The signal says this is a real GPU.
    Hardware,
    /// The signal says this is a software rasteriser.
    Software,
    /// The signal has nothing to say.
    Unknown,
}

/// The software renderer strings of `docs/spikes/A7-tier-c.md`, lowercased,
/// because the match lowercases the haystack rather than the needle.
///
/// **A match is `Software`. A non-match is `Unknown` and never `Hardware`**,
/// because the absence of a string proves nothing, and browsers increasingly
/// mask the renderer string entirely.
///
/// `gallium` is here because A7 lists it, and it is a **known false positive
/// on real hardware**: Mesa's Gallium framework backs the radeonsi and iris
/// drivers on genuine AMD and Intel GPUs, whose renderer strings have
/// historically read "Gallium 0.4 on AMD ...". What contains it is the
/// combination rule rather than the list.
///
/// **The containment is partial and the residue is stated rather than left to
/// be found.** A string is consulted only where the benchmark AND the adapter
/// type both abstained, so a real GPU that measures `Hardware`, or that
/// reports `DiscreteGpu` or `IntegratedGpu`, is never dropped by this entry.
/// The case that is not covered is the one where both abstain, and it is not
/// exotic. [`FillRateBands::RECORDED`] has no software ceiling and a hardware
/// floor of 400 Mpps derived from one Apple figure, so a genuine but slower
/// GPU measures [`SoftwareVerdict::Unknown`], and wgpu's GLES backend commonly
/// reports `DeviceType::Other` or `VirtualGpu`, which is
/// [`SoftwareVerdict::Unknown`] as well. A real Mesa GPU that lands in both
/// abstentions is demoted to tier C by this entry, and tier C renders nothing
/// until F-X001 to F-X004. `a_real_mesa_gpu_that_both_hints_abstain_on_is_demoted`
/// is the test in that direction.
///
/// `docs/lld/tier-resolution.md` records the narrowing, and amending A7 itself
/// is a separate reviewed change.
pub const SOFTWARE_RENDERER_STRINGS: [&str; 7] = [
    "swiftshader",
    "llvmpipe",
    "softpipe",
    "microsoft basic render driver",
    "gallium",
    "mesa offscreen",
    "angle (software",
];

/// The two recorded fill-rate bands, in pixels shaded per second.
///
/// Two numbers with a deliberate gap, so a figure between them is reported
/// `Unknown` rather than forced to a side. The values live in
/// `ci/tier-thresholds.json` with their provenance stated per figure, and the
/// test `the_recorded_bands_match_the_checked_in_file` is what stops the
/// constant and the file from drifting apart.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FillRateBands {
    /// At or above this rate the benchmark says `Hardware`.
    pub hardware_floor_pps: u64,
    /// At or below this rate the benchmark says `Software`.
    ///
    /// `None` means no software-adapter figure has been measured, so there is
    /// no software band and the benchmark never says `Software`. Spike gate
    /// A7.3's rule is "do not invent a number", and a band derived from
    /// nothing is an invented number wearing a provenance note.
    pub software_ceiling_pps: Option<u64>,
}

impl FillRateBands {
    /// The bands recorded in `ci/tier-thresholds.json`. Read that file for the
    /// provenance of each figure, which is not the same for both of them.
    pub const RECORDED: Self = Self {
        hardware_floor_pps: 400_000_000,
        software_ceiling_pps: None,
    };
}

/// One second, in nanoseconds.
///
/// The comparison is `pixels * NANOS_PER_SECOND >= threshold * elapsed_nanos`,
/// widened to `u128` on both sides, rather than a division into a rate. No
/// float, so `float_cmp` has nothing to say. No `as`, so HLD 27.3's cast
/// review has nothing to find. No rounding, so there is no rounding decision
/// to get wrong.
const NANOS_PER_SECOND: u128 = 1_000_000_000;

/// One startup fill-rate measurement: how many pixels were shaded, and how
/// long it took.
///
/// **The clock is the caller's.** `probe` counts pixels and does not time
/// itself, because `std::time::Instant` panics on `wasm32-unknown-unknown` and
/// the alternative is a dependency reaching `performance.now()` inside
/// `ocelli-render`, which is the browser binding deviation D-12 says this
/// crate must not grow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FillRate {
    /// Fragments shaded across every pass of the workload.
    pub pixels_shaded: u64,
    /// Wall-clock nanoseconds from submission to completion.
    pub elapsed_nanos: u64,
}

impl FillRate {
    /// Which band this measurement falls in.
    #[must_use]
    pub fn verdict(self, bands: &FillRateBands) -> SoftwareVerdict {
        // A zero elapsed time is a clock with no resolution rather than an
        // infinitely fast adapter, and zero pixels measured nothing. Left to
        // the comparison below, both would read as `Hardware`.
        if self.pixels_shaded == 0 || self.elapsed_nanos == 0 {
            return SoftwareVerdict::Unknown;
        }
        let shaded = u128::from(self.pixels_shaded) * NANOS_PER_SECOND;
        let elapsed = u128::from(self.elapsed_nanos);
        if shaded >= u128::from(bands.hardware_floor_pps) * elapsed {
            return SoftwareVerdict::Hardware;
        }
        if bands
            .software_ceiling_pps
            .is_some_and(|ceiling| shaded <= u128::from(ceiling) * elapsed)
        {
            return SoftwareVerdict::Software;
        }
        SoftwareVerdict::Unknown
    }
}

/// What one adapter reported about itself.
///
/// A plain struct that `probe.rs` fills and `classify` reads. No trait and no
/// generic parameter, because there is one filler and one reader.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdapterFacts {
    /// The backend this adapter came from.
    pub backend: wgpu::Backend,
    /// What the adapter says it is.
    pub device_type: wgpu::DeviceType,
    /// The adapter name. On `wasm32` this is the `WEBGL_debug_renderer_info`
    /// unmasked renderer string that A7 asks for, read by wgpu's own GLES
    /// backend, so `ocelli-render` needs no browser binding to see it.
    pub name: String,
    /// The driver name.
    pub driver: String,
    /// The driver version string.
    pub driver_info: String,
    /// Whether `DownlevelFlags::COMPUTE_SHADERS` is present.
    pub compute_shaders: bool,
    /// Whether the adapter is fully WebGPU compliant. Recorded as evidence,
    /// not consulted by the decision.
    pub webgpu_compliant: bool,
    /// `Limits::max_texture_dimension_3d`, as reported.
    pub max_tex_3d: u32,
    /// `Limits::max_buffer_size`, as reported.
    pub max_buffer: u64,
}

impl AdapterFacts {
    /// The tier this adapter could serve, or `None` if it is not a candidate
    /// at all.
    ///
    /// An adapter on a WebGPU-capable backend with `COMPUTE_SHADERS` present
    /// is an A-candidate. An adapter on `Backend::Gl`, or one without compute
    /// shaders, is a B-candidate. `Backend::Noop` performs no rendering and
    /// stores no texels, so it is never a candidate.
    #[must_use]
    pub fn candidate_tier(&self) -> Option<Tier> {
        match self.backend {
            wgpu::Backend::Noop => None,
            wgpu::Backend::Gl => Some(Tier::B),
            wgpu::Backend::BrowserWebGpu
            | wgpu::Backend::Vulkan
            | wgpu::Backend::Metal
            | wgpu::Backend::Dx12 => {
                if self.compute_shaders {
                    Some(Tier::A)
                } else {
                    Some(Tier::B)
                }
            }
        }
    }

    /// The adapter-type signal. `DeviceType::Cpu` is the adapter saying it is
    /// a rasteriser. `VirtualGpu` and `Other` say nothing either way.
    #[must_use]
    pub fn device_type_verdict(&self) -> SoftwareVerdict {
        match self.device_type {
            wgpu::DeviceType::Cpu => SoftwareVerdict::Software,
            wgpu::DeviceType::DiscreteGpu | wgpu::DeviceType::IntegratedGpu => {
                SoftwareVerdict::Hardware
            }
            wgpu::DeviceType::VirtualGpu | wgpu::DeviceType::Other => SoftwareVerdict::Unknown,
        }
    }

    /// Which entry of [`SOFTWARE_RENDERER_STRINGS`] this adapter matches, if
    /// any, over `name`, `driver` and `driver_info` together. Different
    /// backends put the useful text in different fields.
    #[must_use]
    pub fn renderer_string_match(&self) -> Option<&'static str> {
        let haystack = format!("{} {} {}", self.name, self.driver, self.driver_info);
        let haystack = haystack.to_lowercase();
        SOFTWARE_RENDERER_STRINGS
            .into_iter()
            .find(|needle| haystack.contains(*needle))
    }
}

/// How preferable one adapter's own claim about itself is. Used only to break
/// ties within a candidate class, never to decide hardware against software.
fn device_type_rank(device_type: wgpu::DeviceType) -> u8 {
    match device_type {
        wgpu::DeviceType::DiscreteGpu => 4,
        wgpu::DeviceType::IntegratedGpu => 3,
        wgpu::DeviceType::VirtualGpu => 2,
        wgpu::DeviceType::Other => 1,
        wgpu::DeviceType::Cpu => 0,
    }
}

/// The index of the adapter to resolve against, or `None` if none of them is a
/// candidate.
///
/// An A-candidate beats every B-candidate. Within a class the adapter's own
/// device type breaks the tie, and an earlier adapter wins an exact tie so the
/// answer does not depend on enumeration order changing under us.
#[must_use]
pub fn choose_candidate(adapters: &[AdapterFacts]) -> Option<usize> {
    adapters
        .iter()
        .enumerate()
        .filter_map(|(index, facts)| facts.candidate_tier().map(|tier| (index, tier, facts)))
        .max_by_key(|&(index, tier, facts)| {
            (
                u8::from(tier == Tier::A),
                device_type_rank(facts.device_type),
                core::cmp::Reverse(index),
            )
        })
        .map(|(index, _, _)| index)
}

/// Whether the module can use wasm SIMD128.
///
/// **A build fact reported honestly, not a runtime probe**, and the reason is
/// worth writing down: a module compiled with `simd128` will not instantiate
/// on a runtime without it, so by the time this Rust is running the answer is
/// always yes. There is no version of this question a Rust function can
/// usefully ask.
///
/// The real runtime probe belongs in the shell, before the artefact is
/// fetched, and it is `wasmSimd128Supported()` in
/// `packages/core/src/capabilities.ts`.
///
/// This is carried in [`TierEvidence`] and deliberately **not** in [`Caps`].
/// Section 22's four fields are the four fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SimdSupport {
    /// Compiled for wasm32 with `simd128` on.
    Simd128,
    /// Compiled for wasm32 without `simd128`. Deviation D-07 measures this
    /// separately as the worst case.
    None,
    /// Not a wasm build, so the question has no answer.
    NotApplicable,
}

impl SimdSupport {
    /// What this build actually is.
    #[must_use]
    pub fn build_target() -> Self {
        if cfg!(target_arch = "wasm32") {
            if cfg!(target_feature = "simd128") {
                Self::Simd128
            } else {
                Self::None
            }
        } else {
            Self::NotApplicable
        }
    }
}

/// Everything the platform told us, gathered by `probe.rs` and read by
/// [`classify`].
///
/// **There is no threads field, and that is a statement rather than an
/// omission.** Decision D5 keeps the build single-threaded, deviation D-07
/// restates it for a second reason, and the strongest way to hold a decision
/// is to leave nowhere to branch on it. A field would be a place for a future
/// story to write `if signals.threads` and mean it. There is no such field, so
/// there is no such line. `sharedMemoryAvailable()` in
/// `packages/core/src/capabilities.ts` reports it as a diagnostic for the
/// support surface, which is the only place it belongs.
#[derive(Debug, Clone)]
pub struct TierSignals {
    /// Every adapter the instance offered.
    pub adapters: Vec<AdapterFacts>,
    /// The startup fill-rate measurement, if one could be taken.
    pub fill_rate: Option<FillRate>,
    /// Whether a device was created on the chosen adapter.
    pub device_created: bool,
    /// What this build can do about SIMD.
    pub simd: SimdSupport,
    /// The recorded bands the measurement is judged against.
    pub bands: FillRateBands,
}

impl TierSignals {
    /// The signals of a session that never asked the platform anything.
    ///
    /// This is what an override of tier C resolves against: no instance, no
    /// adapter, no device and no benchmark, which is the estate deviation
    /// D-07 names getting its startup cost back.
    #[must_use]
    pub fn unprobed() -> Self {
        Self {
            adapters: Vec::new(),
            fill_rate: None,
            device_created: false,
            simd: SimdSupport::build_target(),
            bands: FillRateBands::RECORDED,
        }
    }
}

/// What the operator asked for.
///
/// Three variants and not `Option<Tier>`, because `auto` and an unrecognised
/// value are different answers. Folding them together would make a typo in a
/// deployment's environment indistinguishable from a deliberate `auto`, which
/// is the silent ignore that A7's "always allow an operator override" is meant
/// to prevent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TierRequest {
    /// No override. `auto`, or nothing set.
    Auto,
    /// A parsed override.
    Requested(Tier),
    /// A value that is none of the four words. Refused, and recorded.
    Unrecognised,
}

impl TierRequest {
    /// The tier asked for, if a recognised one was.
    #[must_use]
    pub fn requested(self) -> Option<Tier> {
        match self {
            TierRequest::Requested(tier) => Some(tier),
            TierRequest::Auto | TierRequest::Unrecognised => None,
        }
    }
}

/// What became of the operator's request.
///
/// **A refused override is an outcome, not an error.** Resolution stays
/// infallible and introduces no error type, which is deviation D-07's own
/// honesty rule applied to the resolver itself. Giving a refusal a code would
/// put a successful resolution into the error space, and F-005's
/// `ErrorCode::Unavailable` keeps its distinct meaning, which is a feature
/// saying it cannot run on the resolved tier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverrideOutcome {
    /// Nothing was asked for.
    NotRequested,
    /// The requested tier was constructible and stands.
    Applied(Tier),
    /// The requested tier cannot be constructed on this machine. The measured
    /// tier stands and the refusal is recorded.
    RefusedUnconstructible(Tier),
    /// The value was not one of `a`, `b`, `cpu` or `auto`.
    RefusedUnrecognised,
}

/// Which step of the combination rule produced the answer.
///
/// This is what turns "the viewer is slow on that estate" into a diagnosis, so
/// it names the step rather than summarising it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecidedBy {
    /// The operator's override, applied.
    Override,
    /// No adapter was a candidate.
    NoAdapter,
    /// A candidate existed and no device could be created on it.
    NoDevice,
    /// The startup measurement. A7: "The micro-benchmark is the one to trust,
    /// and the strings are the hint."
    Benchmark,
    /// The adapter's own reported device type.
    AdapterType,
    /// A match against A7's renderer-string list.
    RendererString,
    /// Nothing impeached the candidate, so it was kept.
    CandidateKept,
}

/// Every signal, every verdict, and which step decided.
///
/// F-005 will want this serialisable into whatever structured-log shape it
/// defines, because this record is how a misdetection gets diagnosed on an
/// estate nobody can attach a debugger to.
#[derive(Debug, Clone)]
pub struct TierEvidence {
    /// Which step produced the answer.
    pub decided_by: DecidedBy,
    /// The tier the evidence resolved to, before the override was applied.
    ///
    /// `None` under the tier C short-circuit, because nothing was measured and
    /// a tier here would be a measurement this resolver never took.
    pub measured_tier: Option<Tier>,
    /// The adapter the answer was reached about.
    pub candidate: Option<AdapterFacts>,
    /// That adapter's own candidate tier.
    pub candidate_tier: Option<Tier>,
    /// The benchmark's verdict.
    pub benchmark: SoftwareVerdict,
    /// The adapter type's verdict.
    pub adapter_type: SoftwareVerdict,
    /// The renderer string's verdict.
    pub renderer_string: SoftwareVerdict,
    /// Which A7 entry matched, if one did.
    pub matched_renderer_string: Option<&'static str>,
    /// The measurement itself, so the figure survives the verdict.
    pub fill_rate: Option<FillRate>,
    /// The bands it was judged against.
    pub bands: FillRateBands,
    /// Whether a device was created.
    pub device_created: bool,
    /// How many adapters the instance offered.
    pub adapters_seen: usize,
    /// What this build can do about SIMD.
    pub simd: SimdSupport,
    /// What became of the operator's request.
    pub override_outcome: OverrideOutcome,
}

/// The resolved capabilities and the record of how they were reached.
#[derive(Debug, Clone)]
pub struct Resolution {
    /// HLD section 22's struct, filled.
    pub caps: Caps,
    /// Why it says what it says.
    pub evidence: TierEvidence,
}

/// Tier C's capabilities. There is no device, so every other value would be
/// invented.
fn cpu_caps() -> Caps {
    Caps {
        compute: false,
        max_tex_3d: 0,
        max_buffer: 0,
        tier: Tier::Cpu,
    }
}

/// Resolve a tier from the gathered signals and the operator's request.
///
/// The procedure, written out because an `if` chain is not a specification:
///
/// 1. **An override of tier C short-circuits everything.** No adapter, no
///    device, no benchmark.
/// 2. **Rank the candidates** and take the best, preferring A over B.
/// 3. **No candidate resolves tier C**, recorded as `NoAdapter`.
/// 4. **No device resolves tier C**, recorded as `NoDevice`. What that
///    records is narrower than "there is no GPU path to be had", and the
///    narrower statement is the true one: the BEST candidate could not open a
///    device, and no other adapter was tried. `probe.rs` calls
///    `request_device` on the single adapter `choose_candidate` returned, so
///    on a host whose best candidate is a broken Vulkan ICD beside a working
///    GL driver the answer is tier C while a tier-B path exists and was never
///    attempted. Trying the next adapter is a design decision for a story
///    rather than something to add here.
/// 5. **The benchmark decides if it decided.** A7: a renderer string is a
///    claim, a measured fill rate is a fact. The two hints are recorded and
///    not consulted.
/// 6. **Otherwise the adapter type, then the renderer string.** If both
///    abstain the GPU candidate is kept, because a figure that landed between
///    two bands an order of magnitude apart is itself evidence that something
///    real executed.
/// 7. **Hardware resolves the candidate's tier. Software resolves tier C.**
///    That single line is the whole defect this exists to prevent.
/// 8. **The override applies last, clamped to what is constructible.** Tier C
///    always is, tier B needs some adapter AND a device to have been created,
///    tier A needs an A-candidate AND a device to have been created. An
///    adapter appearing in the enumeration and a device opening on it are
///    different facts, and the second one is what makes a GPU tier
///    constructible. An override that is not constructible is refused,
///    recorded, and the measured tier stands.
#[must_use]
pub fn classify(signals: &TierSignals, request: TierRequest) -> Resolution {
    // Step 1.
    if request == TierRequest::Requested(Tier::Cpu) {
        return Resolution {
            caps: cpu_caps(),
            evidence: TierEvidence {
                decided_by: DecidedBy::Override,
                measured_tier: None,
                candidate: None,
                candidate_tier: None,
                benchmark: SoftwareVerdict::Unknown,
                adapter_type: SoftwareVerdict::Unknown,
                renderer_string: SoftwareVerdict::Unknown,
                matched_renderer_string: None,
                fill_rate: None,
                bands: signals.bands,
                device_created: false,
                adapters_seen: 0,
                simd: signals.simd,
                override_outcome: OverrideOutcome::Applied(Tier::Cpu),
            },
        };
    }

    // Step 2.
    let candidate = choose_candidate(&signals.adapters)
        .and_then(|index| signals.adapters.get(index))
        .cloned();
    let candidate_tier = candidate.as_ref().and_then(AdapterFacts::candidate_tier);

    // Steps 5 and 6's three verdicts, all computed, all recorded, whichever
    // ends up being consulted. The benchmark is step 5 in the procedure above,
    // and step 4 is the device check below.
    let benchmark = signals.fill_rate.map_or(SoftwareVerdict::Unknown, |rate| {
        rate.verdict(&signals.bands)
    });
    let adapter_type = candidate
        .as_ref()
        .map_or(SoftwareVerdict::Unknown, AdapterFacts::device_type_verdict);
    let matched_renderer_string = candidate
        .as_ref()
        .and_then(AdapterFacts::renderer_string_match);
    let renderer_string = if matched_renderer_string.is_some() {
        SoftwareVerdict::Software
    } else {
        SoftwareVerdict::Unknown
    };

    // Steps 3 to 7.
    let (measured_tier, measured_by) = match candidate_tier {
        None => (Tier::Cpu, DecidedBy::NoAdapter),
        Some(_) if !signals.device_created => (Tier::Cpu, DecidedBy::NoDevice),
        Some(tier) => match (benchmark, adapter_type, renderer_string) {
            (SoftwareVerdict::Hardware, _, _) => (tier, DecidedBy::Benchmark),
            (SoftwareVerdict::Software, _, _) => (Tier::Cpu, DecidedBy::Benchmark),
            (SoftwareVerdict::Unknown, SoftwareVerdict::Hardware, _) => {
                (tier, DecidedBy::AdapterType)
            }
            (SoftwareVerdict::Unknown, SoftwareVerdict::Software, _) => {
                (Tier::Cpu, DecidedBy::AdapterType)
            }
            (SoftwareVerdict::Unknown, SoftwareVerdict::Unknown, SoftwareVerdict::Software) => {
                (Tier::Cpu, DecidedBy::RendererString)
            }
            (SoftwareVerdict::Unknown, SoftwareVerdict::Unknown, _) => {
                (tier, DecidedBy::CandidateKept)
            }
        },
    };

    // Step 8. Tier A needs an A-candidate anywhere in the list, and ranking
    // already guarantees that the chosen candidate is that A-candidate.
    let has_a_candidate = signals
        .adapters
        .iter()
        .any(|facts| facts.candidate_tier() == Some(Tier::A));
    let (tier, decided_by, override_outcome) = match request {
        TierRequest::Auto => (measured_tier, measured_by, OverrideOutcome::NotRequested),
        TierRequest::Unrecognised => (
            measured_tier,
            measured_by,
            OverrideOutcome::RefusedUnrecognised,
        ),
        // Handled by the short-circuit above. Repeated rather than reached for
        // an `unreachable!()`, because a panic in an exported path is a defect
        // and not an error path (HLD section 23).
        TierRequest::Requested(Tier::Cpu) => (
            Tier::Cpu,
            DecidedBy::Override,
            OverrideOutcome::Applied(Tier::Cpu),
        ),
        // `has_a_candidate` alone is NOT enough, and the S03 sprint review
        // found that it was being used alone. An adapter appearing in the
        // enumeration says a tier-A adapter EXISTS, and `device_created` says
        // one could actually be opened. The measured path already refuses a
        // GPU tier without a device, at `DecidedBy::NoDevice`, and the
        // override bypassed that: `OCELLI_TIER=a` on a host where no device
        // could be created returned tier A with `Applied(A)`, against this
        // step's own promise that an override is clamped to what is
        // constructible. `RefusedUnconstructible` already existed for exactly
        // this and was unreachable on this path.
        TierRequest::Requested(Tier::A) if has_a_candidate && signals.device_created => (
            Tier::A,
            DecidedBy::Override,
            OverrideOutcome::Applied(Tier::A),
        ),
        TierRequest::Requested(Tier::A) => (
            measured_tier,
            measured_by,
            OverrideOutcome::RefusedUnconstructible(Tier::A),
        ),
        // Same clamp, same reason. Forcing tier B onto an adapter the evidence
        // called software stays allowed, because that is how a misdetection
        // gets diagnosed on the estate it happens on. Forcing it where no
        // device could be created is a different thing and is refused, because
        // there is nothing to run it on: the one adapter that was tried could
        // not open a device.
        TierRequest::Requested(Tier::B) if candidate_tier.is_some() && signals.device_created => (
            Tier::B,
            DecidedBy::Override,
            OverrideOutcome::Applied(Tier::B),
        ),
        TierRequest::Requested(Tier::B) => (
            measured_tier,
            measured_by,
            OverrideOutcome::RefusedUnconstructible(Tier::B),
        ),
    };

    // Step 9 of the plan's numbering: `Caps` is filled from the chosen adapter
    // and NOT from section 7's tier figures. Those are floors a feature may
    // assume for its tier, and clamping the reported truth down to them would
    // make `Caps` lie in the direction of "we have less than we do".
    let caps = match candidate.as_ref() {
        Some(facts) if tier != Tier::Cpu => Caps {
            compute: tier == Tier::A && facts.compute_shaders,
            max_tex_3d: facts.max_tex_3d,
            max_buffer: facts.max_buffer,
            tier,
        },
        _ => Caps {
            compute: false,
            max_tex_3d: 0,
            max_buffer: 0,
            tier,
        },
    };

    Resolution {
        caps,
        evidence: TierEvidence {
            decided_by,
            measured_tier: Some(measured_tier),
            adapters_seen: signals.adapters.len(),
            candidate,
            candidate_tier,
            benchmark,
            adapter_type,
            renderer_string,
            matched_renderer_string,
            fill_rate: signals.fill_rate,
            bands: signals.bands,
            device_created: signals.device_created,
            simd: signals.simd,
            override_outcome,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::{Caps, Tier};

    /// Only tier A supports compute.
    ///
    /// This test does not notice a fourth tier: all three assertions stay true
    /// when one is added, and `supports_compute` is itself the `matches!` on
    /// one variant. What stops a fourth tier is two exhaustive matches that
    /// fail to compile, `classify`'s on `TierRequest` and
    /// `tests/classify_is_total.rs`'s on `caps.tier`.
    #[test]
    fn only_tier_a_supports_compute() {
        assert!(Tier::A.supports_compute());
        assert!(!Tier::B.supports_compute());
        assert!(!Tier::Cpu.supports_compute());
    }

    /// A `Caps` carries section 22's four fields and D-07's third tier.
    ///
    /// The point of the test is that `Tier::Cpu` is CONSTRUCTIBLE here. HLD
    /// section 7 has two tiers and this project has three, and the deviation
    /// is only real if the third one exists in the type.
    #[test]
    fn caps_carries_the_four_fields_and_the_third_tier() {
        let caps = Caps {
            compute: false,
            max_tex_3d: 256,
            max_buffer: 268_435_456,
            tier: Tier::Cpu,
        };
        assert_eq!(caps.tier, Tier::Cpu);
        assert!(!caps.compute);
        assert_eq!(caps.max_tex_3d, 256);
        assert_eq!(caps.max_buffer, 268_435_456);
    }
}

#[cfg(test)]
mod detection_tests {
    use super::{
        AdapterFacts, Caps, DecidedBy, FillRate, FillRateBands, OverrideOutcome,
        SOFTWARE_RENDERER_STRINGS, SimdSupport, SoftwareVerdict, Tier, TierRequest, TierSignals,
        choose_candidate, classify,
    };

    /// The bands the arithmetic tests use, and deliberately NOT the recorded
    /// ones. A test whose expected values move when a measurement is re-taken
    /// is testing the measurement rather than the arithmetic.
    ///
    /// One gigapixel per second and ten megapixels per second, two orders of
    /// magnitude apart, so every hand-computed figure below lands cleanly on
    /// one side, on the other, or in the gap.
    const TEST_BANDS: FillRateBands = FillRateBands {
        hardware_floor_pps: 1_000_000_000,
        software_ceiling_pps: Some(10_000_000),
    };

    fn facts(
        backend: wgpu::Backend,
        device_type: wgpu::DeviceType,
        name: &str,
        compute_shaders: bool,
    ) -> AdapterFacts {
        AdapterFacts {
            backend,
            device_type,
            name: name.to_owned(),
            driver: String::new(),
            driver_info: String::new(),
            compute_shaders,
            webgpu_compliant: compute_shaders,
            max_tex_3d: 2048,
            max_buffer: 268_435_456,
        }
    }

    /// A hardware WebGPU adapter with nothing suspicious about it.
    fn discrete_webgpu() -> AdapterFacts {
        facts(
            wgpu::Backend::Vulkan,
            wgpu::DeviceType::DiscreteGpu,
            "NVIDIA GeForce RTX 4090",
            true,
        )
    }

    /// A fragment-only adapter with nothing suspicious about it.
    fn integrated_gl() -> AdapterFacts {
        facts(
            wgpu::Backend::Gl,
            wgpu::DeviceType::IntegratedGpu,
            "Intel(R) Iris(R) Xe Graphics",
            false,
        )
    }

    fn signals(adapters: Vec<AdapterFacts>, fill_rate: Option<FillRate>) -> TierSignals {
        let device_created = !adapters.is_empty();
        TierSignals {
            adapters,
            fill_rate,
            device_created,
            simd: SimdSupport::NotApplicable,
            bands: TEST_BANDS,
        }
    }

    /// A rate exactly at the hardware floor of `TEST_BANDS`.
    ///
    /// One million pixels in one million nanoseconds is one pixel per
    /// nanosecond, which is 1e6 / 1e-3 s = 1_000_000_000 pixels per second.
    /// The floor is inclusive, so this is `Hardware`.
    fn at_hardware_floor() -> FillRate {
        FillRate {
            pixels_shaded: 1_000_000,
            elapsed_nanos: 1_000_000,
        }
    }

    /// A rate exactly at the software ceiling of `TEST_BANDS`.
    ///
    /// Ten thousand pixels in one million nanoseconds is 1e4 / 1e-3 s =
    /// 10_000_000 pixels per second. The ceiling is inclusive, so this is
    /// `Software`.
    fn at_software_ceiling() -> FillRate {
        FillRate {
            pixels_shaded: 10_000,
            elapsed_nanos: 1_000_000,
        }
    }

    // -----------------------------------------------------------------------
    // The fill-rate comparison. Integer only, hand-computed at both edges.
    // -----------------------------------------------------------------------

    /// 1_000_000 * 1_000_000_000 = 1e15, and 1_000_000_000 * 1_000_000 = 1e15.
    /// Equal, and the floor is "at or above", so `Hardware`.
    #[test]
    fn exactly_at_the_hardware_floor_is_hardware() {
        assert_eq!(
            at_hardware_floor().verdict(&TEST_BANDS),
            SoftwareVerdict::Hardware
        );
    }

    /// One nanosecond slower than the floor. 1_000_000 * 1e9 = 1e15 against
    /// 1_000_000_000 * 1_000_001 = 1.000001e15, so not `Hardware`. And
    /// 10_000_000 * 1_000_001 = 1.000001e13, well under 1e15, so not
    /// `Software` either. The gap between the bands is `Unknown`.
    #[test]
    fn one_nanosecond_below_the_hardware_floor_is_unknown() {
        let rate = FillRate {
            pixels_shaded: 1_000_000,
            elapsed_nanos: 1_000_001,
        };
        assert_eq!(rate.verdict(&TEST_BANDS), SoftwareVerdict::Unknown);
    }

    /// 10_000 * 1e9 = 1e13, and 10_000_000 * 1_000_000 = 1e13. Equal, and the
    /// ceiling is "at or below", so `Software`.
    #[test]
    fn exactly_at_the_software_ceiling_is_software() {
        assert_eq!(
            at_software_ceiling().verdict(&TEST_BANDS),
            SoftwareVerdict::Software
        );
    }

    /// One pixel faster than the ceiling. 10_001 * 1e9 = 1.0001e13 against
    /// 1e13, so not `Software`, and nowhere near the floor, so `Unknown`.
    #[test]
    fn one_pixel_above_the_software_ceiling_is_unknown() {
        let rate = FillRate {
            pixels_shaded: 10_001,
            elapsed_nanos: 1_000_000,
        };
        assert_eq!(rate.verdict(&TEST_BANDS), SoftwareVerdict::Unknown);
    }

    /// With no software ceiling recorded there is no software band, so a rate
    /// far below any plausible hardware figure is `Unknown` and never
    /// `Software`. Spike gate A7.3 forbids inventing the number, and a band
    /// derived from nothing is an invented number.
    #[test]
    fn without_a_recorded_ceiling_a_slow_rate_is_unknown_not_software() {
        let bands = FillRateBands {
            hardware_floor_pps: 1_000_000_000,
            software_ceiling_pps: None,
        };
        assert_eq!(
            at_software_ceiling().verdict(&bands),
            SoftwareVerdict::Unknown
        );
        assert_eq!(
            at_hardware_floor().verdict(&bands),
            SoftwareVerdict::Hardware
        );
    }

    /// A zero elapsed time is a clock with no resolution rather than an
    /// infinitely fast adapter, and a zero pixel count measured nothing. Both
    /// are `Unknown`. Neither may read as `Hardware`, which is what a naive
    /// "pixels times a billion is at least zero" comparison would say.
    #[test]
    fn a_degenerate_measurement_is_unknown() {
        let no_time = FillRate {
            pixels_shaded: 1_000_000,
            elapsed_nanos: 0,
        };
        let no_pixels = FillRate {
            pixels_shaded: 0,
            elapsed_nanos: 1_000_000,
        };
        assert_eq!(no_time.verdict(&TEST_BANDS), SoftwareVerdict::Unknown);
        assert_eq!(no_pixels.verdict(&TEST_BANDS), SoftwareVerdict::Unknown);
    }

    /// The widest inputs the types allow, on both sides of the comparison.
    ///
    /// `u64::MAX` pixels in `u64::MAX` nanoseconds is exactly one pixel per
    /// nanosecond, which is 1_000_000_000 pixels per second, which is exactly
    /// `TEST_BANDS`'s floor. So the answer is `Hardware`, and getting there
    /// needs `(2^64 - 1) * 10^9`, about 1.8e28, on the left and the same on
    /// the right. Both fit in `u128`, whose maximum is about 3.4e38. The point
    /// of the test is that this does not wrap, does not panic and does not
    /// reach for a float.
    #[test]
    fn the_comparison_does_not_overflow_at_the_extremes() {
        let rate = FillRate {
            pixels_shaded: u64::MAX,
            elapsed_nanos: u64::MAX,
        };
        assert_eq!(rate.verdict(&TEST_BANDS), SoftwareVerdict::Hardware);

        // The largest right-hand side the types permit: (2^64 - 1)^2, about
        // 3.4028e38, still inside u128. Nothing can be at or above a floor of
        // u64::MAX pixels per second at one pixel per nanosecond.
        let absurd = FillRateBands {
            hardware_floor_pps: u64::MAX,
            software_ceiling_pps: Some(u64::MAX),
        };
        assert_eq!(rate.verdict(&absurd), SoftwareVerdict::Software);
    }

    // -----------------------------------------------------------------------
    // Candidate ranking.
    // -----------------------------------------------------------------------

    /// An A-candidate beats a B-candidate however good the B-candidate looks.
    #[test]
    fn an_a_candidate_is_preferred_to_a_b_candidate() {
        let adapters = vec![integrated_gl(), discrete_webgpu()];
        assert_eq!(choose_candidate(&adapters), Some(1));
    }

    /// Within a class, a discrete GPU beats an integrated one.
    #[test]
    fn a_discrete_gpu_is_preferred_within_a_class() {
        let adapters = vec![
            facts(
                wgpu::Backend::Vulkan,
                wgpu::DeviceType::IntegratedGpu,
                "Integrated",
                true,
            ),
            discrete_webgpu(),
        ];
        assert_eq!(choose_candidate(&adapters), Some(1));
    }

    /// The device-type preference order, best first.
    ///
    /// This is the order the tie-break inside a candidate class has to apply,
    /// and it is a strict order rather than a set of preferences: a discrete
    /// GPU is a better answer than an integrated one, an integrated one than a
    /// virtualised one, a virtualised one than an adapter that will not say,
    /// and anything at all than an adapter that has already told us it is a
    /// rasteriser. Spike gate A7 puts the adapter's reported type second of its
    /// three signals, "where a fallback adapter identifies itself as one", so
    /// `DeviceType::Cpu` is last by the adapter's own admission.
    const RANKED_DEVICE_TYPES: [wgpu::DeviceType; 5] = [
        wgpu::DeviceType::DiscreteGpu,
        wgpu::DeviceType::IntegratedGpu,
        wgpu::DeviceType::VirtualGpu,
        wgpu::DeviceType::Other,
        wgpu::DeviceType::Cpu,
    ];

    /// **Every step of that order, in both list positions.** The
    /// discrete-beats-integrated step had a test and the four steps below it
    /// did not, so any reordering of the lower half was invisible.
    ///
    /// Both orders are asserted for each pair, because a rank comparison that
    /// had collapsed into "whichever came first" would satisfy one of them and
    /// fail the other, and a rank comparison that had collapsed into
    /// "whichever came last" would do the reverse.
    ///
    /// Every adapter here is a B-candidate on the same backend, so the class
    /// term of the ranking is constant and only the device type varies.
    #[test]
    fn the_device_type_preference_is_strict_at_every_step() {
        for (position, better) in RANKED_DEVICE_TYPES.into_iter().enumerate() {
            for worse in RANKED_DEVICE_TYPES.into_iter().skip(position + 1) {
                let good = facts(wgpu::Backend::Vulkan, better, "better", false);
                let bad = facts(wgpu::Backend::Vulkan, worse, "worse", false);
                assert_eq!(
                    choose_candidate(&[good.clone(), bad.clone()]),
                    Some(0),
                    "{better:?} listed first lost to {worse:?}"
                );
                assert_eq!(
                    choose_candidate(&[bad, good]),
                    Some(1),
                    "{better:?} listed second lost to {worse:?}"
                );
            }
        }
    }

    /// **A wrong preference is a wrong record and not only a wrong choice.**
    /// The ranked adapter is the one whose reported limits fill [`Caps`] and
    /// whose identity fills [`TierEvidence`], so an adapter ranked above the
    /// one it should be ranked below puts another machine's figures into the
    /// record that a misdetection is later diagnosed from.
    #[test]
    fn the_ranked_adapters_own_limits_are_the_ones_recorded() {
        let mut virtualised = facts(
            wgpu::Backend::Vulkan,
            wgpu::DeviceType::VirtualGpu,
            "Paravirtual Adapter",
            false,
        );
        virtualised.max_tex_3d = 256;
        virtualised.max_buffer = 16_777_216;
        let mut discrete = facts(
            wgpu::Backend::Vulkan,
            wgpu::DeviceType::DiscreteGpu,
            "NVIDIA GeForce RTX 4090",
            false,
        );
        discrete.max_tex_3d = 2048;
        discrete.max_buffer = 268_435_456;

        let resolved = classify(
            &signals(vec![virtualised, discrete], None),
            TierRequest::Auto,
        );
        assert_eq!(resolved.caps.max_tex_3d, 2048);
        assert_eq!(resolved.caps.max_buffer, 268_435_456);
        assert_eq!(
            resolved.evidence.candidate.map(|chosen| chosen.name),
            Some("NVIDIA GeForce RTX 4090".to_owned())
        );
    }

    /// **An exact tie is broken by enumeration order, and the earlier adapter
    /// wins.** Two adapters of the same candidate class and the same device
    /// type are indistinguishable to the ranking, and the answer still has to
    /// be one adapter rather than whichever one the platform happened to
    /// enumerate last. Adapter enumeration order is not a stable fact of a
    /// machine: it moves with a driver update, with a hotplugged display and
    /// with a `WGPU_ADAPTER_NAME` in the environment. A resolver that followed
    /// it would report a different `Caps` on the same machine on two
    /// consecutive days, and deviation D-07's whole point is that the tier is
    /// diagnosable after the fact.
    ///
    /// The limits differ so the assertion can see WHICH adapter was chosen
    /// rather than only that some choice was made.
    #[test]
    fn an_exact_tie_is_won_by_the_earlier_adapter() {
        let mut first = discrete_webgpu();
        first.name = "First Adapter".to_owned();
        first.max_tex_3d = 4096;
        let mut second = discrete_webgpu();
        second.name = "Second Adapter".to_owned();
        second.max_tex_3d = 8192;

        assert_eq!(choose_candidate(&[first.clone(), second.clone()]), Some(0));
        let resolved = classify(&signals(vec![first, second], None), TierRequest::Auto);
        assert_eq!(resolved.caps.max_tex_3d, 4096);
        assert_eq!(
            resolved.evidence.candidate.map(|chosen| chosen.name),
            Some("First Adapter".to_owned())
        );
    }

    /// `Backend::Noop` renders nothing and stores no texels. It is never a
    /// candidate, whatever else is present.
    #[test]
    fn the_noop_backend_is_never_a_candidate() {
        let adapters = vec![facts(
            wgpu::Backend::Noop,
            wgpu::DeviceType::Other,
            "noop",
            true,
        )];
        assert_eq!(choose_candidate(&adapters), None);
    }

    /// A backend that could serve tier A, without compute shaders, is a
    /// B-candidate. Section 7's tier A is defined by compute being available,
    /// not by the name of the backend.
    #[test]
    fn a_compute_less_vulkan_adapter_is_a_b_candidate() {
        let adapter = facts(
            wgpu::Backend::Vulkan,
            wgpu::DeviceType::DiscreteGpu,
            "Odd GPU",
            false,
        );
        assert_eq!(adapter.candidate_tier(), Some(Tier::B));
    }

    /// **The boundary between the two GPU tiers, and it is the backend.** HLD
    /// section 7: "Tier A is WebGPU: compute shaders, storage buffers, 3D
    /// textures to 2048. Tier B is WebGL2 through wgpu's downlevel profile:
    /// fragment shaders only, no compute, no storage buffers, a conservative
    /// 3D-texture floor of 256." Tier A is named by the API that serves it and
    /// tier B by a different API, so an adapter reached through `Backend::Gl`
    /// is a B-candidate however capable it is.
    ///
    /// That distinction is not theoretical. A native GLES 3.1 driver reports
    /// `DownlevelFlags::COMPUTE_SHADERS`, and wgpu's GL backend is the same
    /// backend on the desktop as in a browser. Reading the flag before the
    /// backend would resolve tier A on it, and section 7's tier A promises
    /// storage buffers and 3D textures to 2048 that a WebGL2 context cannot
    /// give. A feature that then assumed them would not fail, it would run
    /// somewhere else, which is the misdetection deviation D-07 exists for
    /// arriving from the other direction.
    ///
    /// `tests/classify_is_total.rs` cannot cover this and must not be asked
    /// to. Its `a_candidates` count calls `candidate_tier`, the function under
    /// test, so on this branch the property is a tautology.
    #[test]
    fn a_gl_adapter_reporting_compute_shaders_is_still_a_b_candidate() {
        let adapter = facts(
            wgpu::Backend::Gl,
            wgpu::DeviceType::DiscreteGpu,
            "Mesa Intel(R) Arc(tm) Graphics",
            true,
        );
        assert_eq!(adapter.candidate_tier(), Some(Tier::B));

        let resolved = classify(&signals(vec![adapter], None), TierRequest::Auto);
        assert_eq!(resolved.caps.tier, Tier::B);
        assert_eq!(resolved.evidence.candidate_tier, Some(Tier::B));
        // Section 7 puts compute in tier A only, so an adapter placed in tier B
        // reports no compute whatever its own downlevel flags said.
        assert!(!resolved.caps.compute);
    }

    // -----------------------------------------------------------------------
    // The adapter-type signal.
    // -----------------------------------------------------------------------

    /// **Every variant, because the two that abstain are the load-bearing
    /// ones.** Spike gate A7 gives the adapter's reported type as a signal
    /// "where a fallback adapter identifies itself as one", which licenses
    /// `Software` on `DeviceType::Cpu` and nothing else. `DiscreteGpu` and
    /// `IntegratedGpu` are the adapter naming real hardware. `VirtualGpu` and
    /// `Other` are neither claim: a paravirtual adapter in a hypervisor may be
    /// backed by a passed-through GPU or by a rasteriser on the host, and the
    /// type does not say which. A7's own rule is that no signal is sufficient
    /// alone, so a signal picking a side it has no evidence for is worse than
    /// one that abstains.
    #[test]
    fn the_adapter_type_signal_speaks_only_where_a7_licenses_it() {
        let verdict = |device_type| {
            facts(wgpu::Backend::Gl, device_type, "Clean Name", false).device_type_verdict()
        };
        assert_eq!(
            verdict(wgpu::DeviceType::Cpu),
            SoftwareVerdict::Software,
            "an adapter identifying itself as a rasteriser was not believed"
        );
        assert_eq!(
            verdict(wgpu::DeviceType::DiscreteGpu),
            SoftwareVerdict::Hardware
        );
        assert_eq!(
            verdict(wgpu::DeviceType::IntegratedGpu),
            SoftwareVerdict::Hardware
        );
        assert_eq!(
            verdict(wgpu::DeviceType::VirtualGpu),
            SoftwareVerdict::Unknown,
            "a virtualised adapter was read as a claim about the hardware \
             behind it"
        );
        assert_eq!(verdict(wgpu::DeviceType::Other), SoftwareVerdict::Unknown);
    }

    /// **What `VirtualGpu` abstaining actually buys.** A rasteriser inside a
    /// hypervisor is the estate deviation D-07 was raised for, and wgpu's GLES
    /// backend reports `VirtualGpu` there as readily as `Other`.
    /// [`SOFTWARE_RENDERER_STRINGS`] states its residue in those terms, so the
    /// claim and the coverage have to agree.
    ///
    /// If the adapter type said `Hardware` on a virtualised adapter it would
    /// take step 6 before the renderer string ever got its turn, and this
    /// llvmpipe adapter would keep tier B on the strength of a device-type
    /// claim that carries no measurement at all.
    #[test]
    fn a_virtual_gpu_does_not_out_rank_a_software_renderer_string() {
        let adapter = facts(
            wgpu::Backend::Gl,
            wgpu::DeviceType::VirtualGpu,
            "llvmpipe (LLVM 15.0.7, 256 bits)",
            false,
        );
        let resolved = classify(&signals(vec![adapter], None), TierRequest::Auto);
        assert_eq!(resolved.caps.tier, Tier::Cpu);
        assert_eq!(resolved.evidence.decided_by, DecidedBy::RendererString);
        assert_eq!(resolved.evidence.adapter_type, SoftwareVerdict::Unknown);
    }

    // -----------------------------------------------------------------------
    // The combination rule.
    // -----------------------------------------------------------------------

    #[test]
    fn no_adapter_at_all_resolves_cpu() {
        let resolved = classify(&signals(Vec::new(), None), TierRequest::Auto);
        assert_eq!(resolved.caps.tier, Tier::Cpu);
        assert_eq!(resolved.evidence.decided_by, DecidedBy::NoAdapter);
    }

    /// An adapter was found and no device could be created on it. `probe.rs`
    /// tries exactly one adapter, the best candidate, so the answer is tier C
    /// and it is recorded as `NoDevice` rather than as `NoAdapter`, because
    /// the two are different diagnoses.
    #[test]
    fn a_device_that_could_not_be_created_resolves_cpu() {
        let mut signals = signals(vec![discrete_webgpu()], None);
        signals.device_created = false;
        let resolved = classify(&signals, TierRequest::Auto);
        assert_eq!(resolved.caps.tier, Tier::Cpu);
        assert_eq!(resolved.evidence.decided_by, DecidedBy::NoDevice);
    }

    /// `DeviceType::Cpu` is the adapter saying so itself. No benchmark ran, so
    /// the adapter type decides.
    #[test]
    fn an_adapter_reporting_device_type_cpu_resolves_cpu() {
        let adapter = facts(
            wgpu::Backend::Vulkan,
            wgpu::DeviceType::Cpu,
            "Some Conforming Rasteriser",
            true,
        );
        let resolved = classify(&signals(vec![adapter], None), TierRequest::Auto);
        assert_eq!(resolved.caps.tier, Tier::Cpu);
        assert_eq!(resolved.evidence.decided_by, DecidedBy::AdapterType);
    }

    /// A hardware adapter type keeps the candidate's own tier.
    #[test]
    fn a_hardware_gl_adapter_resolves_tier_b() {
        let resolved = classify(&signals(vec![integrated_gl()], None), TierRequest::Auto);
        assert_eq!(resolved.caps.tier, Tier::B);
        assert_eq!(resolved.evidence.decided_by, DecidedBy::AdapterType);
        assert!(!resolved.caps.compute);
    }

    /// Nothing said anything. A GPU candidate that no signal impeaches is
    /// kept, because the absence of evidence is not evidence of a rasteriser.
    #[test]
    fn a_candidate_no_signal_impeaches_is_kept() {
        let adapter = facts(
            wgpu::Backend::Gl,
            wgpu::DeviceType::Other,
            "Some Vendor Graphics",
            false,
        );
        let resolved = classify(&signals(vec![adapter], None), TierRequest::Auto);
        assert_eq!(resolved.caps.tier, Tier::B);
        assert_eq!(resolved.evidence.decided_by, DecidedBy::CandidateKept);
    }

    /// **The row deviation D-07 exists for.** A software rasteriser presents a
    /// conforming WebGL2 context and reports a device type nobody can read
    /// anything from. HLD section 7's tier logic sees WebGL2 and resolves
    /// tier B. This resolver reads the renderer string and resolves tier C.
    ///
    /// Each string is typed out as a whole renderer string rather than looped
    /// over `SOFTWARE_RENDERER_STRINGS`, deliberately: a loop over the list
    /// passes vacuously when the list is emptied, and emptying the list is
    /// exactly the mutation that has to go red.
    ///
    /// **And each row stands for exactly one entry**, which is what makes the
    /// claim true entry by entry rather than in aggregate. The `gallium` row
    /// used to read "Gallium 0.4 on llvmpipe", which matches `llvmpipe` on its
    /// own, so removing `gallium` from the list left this test green and the
    /// coverage table in `docs/lld/tier-resolution.md` claiming otherwise. The
    /// paired entry and the single-match assertion below are the fix, and they
    /// hold for a row added later as well as for these seven.
    #[test]
    fn each_a7_software_renderer_string_resolves_cpu() {
        let renderers = [
            (
                "ANGLE (Google, Vulkan 1.3.0 (SwiftShader Device (Subzero) (0x0000C0DE)), SwiftShader driver)",
                "swiftshader",
            ),
            ("llvmpipe (LLVM 15.0.7, 256 bits)", "llvmpipe"),
            ("softpipe", "softpipe"),
            (
                "Microsoft Basic Render Driver",
                "microsoft basic render driver",
            ),
            ("Gallium 0.4 on AMD RADV POLARIS10", "gallium"),
            ("Mesa OffScreen", "mesa offscreen"),
            (
                "ANGLE (Software Adapter, Direct3D11 vs_5_0 ps_5_0)",
                "angle (software",
            ),
        ];
        // One row per entry, in the list's own order. An entry added to A7's
        // list with no row here goes red rather than being covered by
        // somebody else's string.
        assert_eq!(
            renderers.map(|(_, entry)| entry),
            SOFTWARE_RENDERER_STRINGS,
            "the rows and A7's list have drifted apart"
        );

        for (renderer, entry) in renderers {
            let lowered = renderer.to_lowercase();
            let matched: Vec<&str> = SOFTWARE_RENDERER_STRINGS
                .into_iter()
                .filter(|needle| lowered.contains(*needle))
                .collect();
            assert_eq!(
                matched,
                vec![entry],
                "{renderer} does not stand for {entry} alone, so dropping \
                 {entry} would leave this row green"
            );

            let adapter = facts(wgpu::Backend::Gl, wgpu::DeviceType::Other, renderer, false);
            assert_eq!(adapter.renderer_string_match(), Some(entry));
            let resolved = classify(&signals(vec![adapter], None), TierRequest::Auto);
            assert_eq!(
                resolved.caps.tier,
                Tier::Cpu,
                "{renderer} resolved a GPU tier"
            );
            assert_eq!(resolved.evidence.decided_by, DecidedBy::RendererString);
        }
    }

    /// **The direction the list's own doc comment used to exculpate itself
    /// in.** `SOFTWARE_RENDERER_STRINGS` says the combination rule contains
    /// the `gallium` false positive, and it does so only where the benchmark
    /// or the adapter type has something to say. This is the case where
    /// neither does, and it is the case a Mesa GPU on wgpu's GLES backend
    /// actually presents: `DeviceType::Other`, which abstains, and no usable
    /// measurement, which abstains. Real AMD hardware is then demoted to
    /// tier C, which renders nothing until F-X001 to F-X004.
    ///
    /// The two halves are asserted together on purpose. The second is what the
    /// doc comment claims and the first is what it omitted, and separating
    /// them would leave the claim looking complete again.
    #[test]
    fn a_real_mesa_gpu_that_both_hints_abstain_on_is_demoted() {
        let mesa = facts(
            wgpu::Backend::Gl,
            wgpu::DeviceType::Other,
            "Gallium 0.4 on AMD RADV POLARIS10",
            false,
        );

        // Both hints abstain, so the string decides and real hardware goes to
        // tier C.
        let abstained = classify(&signals(vec![mesa.clone()], None), TierRequest::Auto);
        assert_eq!(abstained.caps.tier, Tier::Cpu);
        assert_eq!(abstained.evidence.decided_by, DecidedBy::RendererString);
        assert_eq!(abstained.evidence.benchmark, SoftwareVerdict::Unknown);
        assert_eq!(abstained.evidence.adapter_type, SoftwareVerdict::Unknown);

        // And the containment the doc comment does claim: a measurement in the
        // hardware band keeps the same adapter on its own tier.
        let measured = classify(
            &signals(vec![mesa], Some(at_hardware_floor())),
            TierRequest::Auto,
        );
        assert_eq!(measured.caps.tier, Tier::B);
        assert_eq!(measured.evidence.decided_by, DecidedBy::Benchmark);
        assert_eq!(measured.evidence.renderer_string, SoftwareVerdict::Software);
    }

    /// The list is matched case-insensitively and across all three of `name`,
    /// `driver` and `driver_info`, because different backends put the useful
    /// text in different fields.
    #[test]
    fn the_renderer_string_match_is_case_insensitive_and_covers_every_field() {
        let mut adapter = facts(
            wgpu::Backend::Gl,
            wgpu::DeviceType::Other,
            "Clean Name",
            false,
        );
        adapter.driver_info = "SWIFTSHADER build 1.2.3".to_owned();
        assert_eq!(adapter.renderer_string_match(), Some("swiftshader"));
    }

    /// A7's list is seven entries and every one is lowercase, because the
    /// match lowercases the haystack and not the needle.
    #[test]
    fn the_a7_list_is_seven_lowercase_entries() {
        assert_eq!(SOFTWARE_RENDERER_STRINGS.len(), 7);
        for entry in SOFTWARE_RENDERER_STRINGS {
            assert_eq!(entry, entry.to_lowercase(), "{entry} is not lowercase");
        }
    }

    /// **"A renderer string is a claim. A measured fill rate is a fact."**
    /// The benchmark says hardware, both hints say software, and the
    /// benchmark decides. The hints are still recorded, which is the whole
    /// point of recording them.
    #[test]
    fn the_benchmark_overrules_both_software_hints() {
        let adapter = facts(
            wgpu::Backend::Vulkan,
            wgpu::DeviceType::Cpu,
            "llvmpipe (LLVM 15.0.7, 256 bits)",
            true,
        );
        let resolved = classify(
            &signals(vec![adapter], Some(at_hardware_floor())),
            TierRequest::Auto,
        );
        assert_eq!(resolved.caps.tier, Tier::A);
        assert_eq!(resolved.evidence.decided_by, DecidedBy::Benchmark);
        assert_eq!(resolved.evidence.adapter_type, SoftwareVerdict::Software);
        assert_eq!(resolved.evidence.renderer_string, SoftwareVerdict::Software);
    }

    /// And the other way. A discrete GPU with a clean name that measures in
    /// the software band is tier C, and the hints do not save it.
    #[test]
    fn the_benchmark_overrules_both_hardware_hints() {
        let resolved = classify(
            &signals(vec![discrete_webgpu()], Some(at_software_ceiling())),
            TierRequest::Auto,
        );
        assert_eq!(resolved.caps.tier, Tier::Cpu);
        assert_eq!(resolved.evidence.decided_by, DecidedBy::Benchmark);
        assert_eq!(resolved.evidence.adapter_type, SoftwareVerdict::Hardware);
    }

    /// A figure that landed between two bands set two orders of magnitude
    /// apart decides nothing, so the adapter type gets its turn.
    #[test]
    fn a_benchmark_between_the_bands_falls_through_to_the_adapter_type() {
        let adapter = facts(
            wgpu::Backend::Vulkan,
            wgpu::DeviceType::Cpu,
            "Clean Name",
            true,
        );
        let between = FillRate {
            pixels_shaded: 1_000_000,
            elapsed_nanos: 10_000_000,
        };
        assert_eq!(between.verdict(&TEST_BANDS), SoftwareVerdict::Unknown);
        let resolved = classify(&signals(vec![adapter], Some(between)), TierRequest::Auto);
        assert_eq!(resolved.caps.tier, Tier::Cpu);
        assert_eq!(resolved.evidence.decided_by, DecidedBy::AdapterType);
    }

    // -----------------------------------------------------------------------
    // Caps filling.
    // -----------------------------------------------------------------------

    /// Tier C has no device, so every figure would be invented.
    #[test]
    fn tier_cpu_caps_are_all_zero() {
        let resolved = classify(&signals(Vec::new(), None), TierRequest::Auto);
        assert_eq!(
            resolved.caps,
            Caps {
                compute: false,
                max_tex_3d: 0,
                max_buffer: 0,
                tier: Tier::Cpu,
            }
        );
    }

    /// The adapter's REPORTED limits, unclamped. Section 7's 2048 and 256 MiB
    /// are floors a feature may assume for its tier, not a ceiling to clamp
    /// the truth down to. Clamping would make `Caps` understate the machine,
    /// which is a different lie but still a lie.
    #[test]
    fn a_gpu_tier_carries_the_adapters_reported_limits_unclamped() {
        let mut adapter = discrete_webgpu();
        adapter.max_tex_3d = 8192;
        adapter.max_buffer = 4_294_967_296;
        let resolved = classify(&signals(vec![adapter], None), TierRequest::Auto);
        assert_eq!(resolved.caps.tier, Tier::A);
        assert!(resolved.caps.compute);
        assert_eq!(resolved.caps.max_tex_3d, 8192);
        assert_eq!(resolved.caps.max_buffer, 4_294_967_296);
    }

    /// `compute` is true only for tier A. An operator forcing tier B onto an
    /// adapter that does have compute shaders still gets `compute: false`,
    /// because the tier is what a feature reads.
    #[test]
    fn compute_is_false_on_tier_b_even_with_a_compute_capable_adapter() {
        let resolved = classify(
            &signals(vec![discrete_webgpu()], None),
            TierRequest::Requested(Tier::B),
        );
        assert_eq!(resolved.caps.tier, Tier::B);
        assert!(!resolved.caps.compute);
    }

    // -----------------------------------------------------------------------
    // The operator override.
    // -----------------------------------------------------------------------

    #[test]
    fn the_override_string_parses_case_insensitively() {
        assert_eq!(
            Tier::from_override_str("A"),
            TierRequest::Requested(Tier::A)
        );
        assert_eq!(
            Tier::from_override_str("a"),
            TierRequest::Requested(Tier::A)
        );
        assert_eq!(
            Tier::from_override_str(" B "),
            TierRequest::Requested(Tier::B)
        );
        assert_eq!(
            Tier::from_override_str("CPU"),
            TierRequest::Requested(Tier::Cpu)
        );
        assert_eq!(Tier::from_override_str("Auto"), TierRequest::Auto);
        assert_eq!(Tier::from_override_str(""), TierRequest::Auto);
    }

    /// Anything else is refused and recorded. It is never a silent ignore,
    /// which is what folding it into "auto" would be.
    #[test]
    fn an_unparseable_override_is_a_distinct_answer_from_auto() {
        assert_eq!(Tier::from_override_str("tier-a"), TierRequest::Unrecognised);
        assert_eq!(Tier::from_override_str("gpu"), TierRequest::Unrecognised);
        let resolved = classify(
            &signals(vec![discrete_webgpu()], None),
            TierRequest::Unrecognised,
        );
        assert_eq!(resolved.caps.tier, Tier::A);
        assert_eq!(
            resolved.evidence.override_outcome,
            OverrideOutcome::RefusedUnrecognised
        );
    }

    /// **The accepted set is exactly four words, and nothing near them.**
    ///
    /// The two tests above show that each of the four words is accepted and
    /// that two chosen strings are not. Neither of them notices an alias being
    /// ADDED, and an alias is the shape this parser fails in: `OCELLI_TIER=c`
    /// quietly meaning tier C on one deployment and being refused on the next
    /// is the silent ignore that A7's "always allow an operator override" and
    /// [`TierRequest::Unrecognised`] exist together to prevent. The value
    /// arrives from an environment variable natively and from a query
    /// parameter in a browser, so somebody will type the short form.
    ///
    /// So the set is pinned rather than sampled: every string of at most three
    /// characters over the alphabet an override could plausibly be spelled in,
    /// plus the four words themselves and a handful of longer neighbours. The
    /// four words and the empty string are accepted and every one of the
    /// remaining fifty-six thousand is [`TierRequest::Unrecognised`].
    #[test]
    fn the_override_parser_accepts_exactly_its_four_words() {
        const RECOGNISED: [&str; 5] = ["", "a", "b", "cpu", "auto"];
        const ALPHABET: &str = "abcdefghijklmnopqrstuvwxyz0123456789-_";
        /// Longer than the exhaustive sweep reaches, and each one a plausible
        /// spelling somebody would expect to work.
        const NEIGHBOURS: [&str; 8] = [
            "auto1", "autos", "acpu", "cpu1", "cpus", "tier-a", "tier-c", "c-p-u",
        ];

        let mut candidates: Vec<String> = Vec::new();
        candidates.push(String::new());
        for first in ALPHABET.chars() {
            candidates.push(first.to_string());
            for second in ALPHABET.chars() {
                candidates.push(format!("{first}{second}"));
                for third in ALPHABET.chars() {
                    candidates.push(format!("{first}{second}{third}"));
                }
            }
        }
        for extra in RECOGNISED.into_iter().chain(NEIGHBOURS) {
            candidates.push(extra.to_owned());
        }

        for candidate in &candidates {
            let parsed = Tier::from_override_str(candidate);
            let expected = RECOGNISED.contains(&candidate.as_str());
            assert_eq!(
                parsed != TierRequest::Unrecognised,
                expected,
                "{candidate:?} parsed as {parsed:?}"
            );
        }

        // And the four words mean what they say, so pinning the SIZE of the
        // set cannot be satisfied by swapping two of its members.
        assert_eq!(
            Tier::from_override_str("a"),
            TierRequest::Requested(Tier::A)
        );
        assert_eq!(
            Tier::from_override_str("b"),
            TierRequest::Requested(Tier::B)
        );
        assert_eq!(
            Tier::from_override_str("cpu"),
            TierRequest::Requested(Tier::Cpu)
        );
        assert_eq!(Tier::from_override_str("auto"), TierRequest::Auto);
        assert_eq!(Tier::from_override_str(""), TierRequest::Auto);
    }

    #[test]
    fn auto_is_not_an_override() {
        let resolved = classify(&signals(vec![discrete_webgpu()], None), TierRequest::Auto);
        assert_eq!(
            resolved.evidence.override_outcome,
            OverrideOutcome::NotRequested
        );
    }

    /// An override of tier C short-circuits everything, and the evidence says
    /// so: no candidate was considered, no measurement is recorded, and
    /// `measured_tier` is `None` because nothing was measured. That last field
    /// is what stops the record from claiming a measurement it never took.
    #[test]
    fn an_override_of_cpu_short_circuits_every_other_signal() {
        let resolved = classify(
            &signals(vec![discrete_webgpu()], Some(at_hardware_floor())),
            TierRequest::Requested(Tier::Cpu),
        );
        assert_eq!(resolved.caps.tier, Tier::Cpu);
        assert_eq!(resolved.evidence.decided_by, DecidedBy::Override);
        assert_eq!(
            resolved.evidence.override_outcome,
            OverrideOutcome::Applied(Tier::Cpu)
        );
        assert_eq!(resolved.evidence.measured_tier, None);
        assert_eq!(resolved.evidence.candidate, None);
        assert_eq!(resolved.evidence.fill_rate, None);
    }

    /// Tier A needs an A-candidate. With only a fragment-only adapter present
    /// there is nothing to construct it on, so the override is refused, the
    /// measured tier stands, and the refusal is in the record.
    #[test]
    fn an_override_of_a_is_refused_with_no_a_candidate() {
        let resolved = classify(
            &signals(vec![integrated_gl()], None),
            TierRequest::Requested(Tier::A),
        );
        assert_eq!(resolved.caps.tier, Tier::B);
        assert_eq!(
            resolved.evidence.override_outcome,
            OverrideOutcome::RefusedUnconstructible(Tier::A)
        );
        assert_eq!(resolved.evidence.decided_by, DecidedBy::AdapterType);
    }

    /// Tier B needs some adapter. With none at all it is refused too.
    #[test]
    fn an_override_of_b_is_refused_with_no_adapter_at_all() {
        let resolved = classify(&signals(Vec::new(), None), TierRequest::Requested(Tier::B));
        assert_eq!(resolved.caps.tier, Tier::Cpu);
        assert_eq!(
            resolved.evidence.override_outcome,
            OverrideOutcome::RefusedUnconstructible(Tier::B)
        );
    }

    /// **Forcing tier B onto a rasteriser the evidence called software is
    /// deliberately allowed.** That is how the misdetection of deviation D-07
    /// gets diagnosed on the estate it happens on. It is recorded, so it is
    /// never silent, and `measured_tier` still says what the evidence found.
    #[test]
    fn an_override_of_b_onto_a_software_rasteriser_is_allowed_and_recorded() {
        let adapter = facts(
            wgpu::Backend::Gl,
            wgpu::DeviceType::Other,
            "SwiftShader",
            false,
        );
        let resolved = classify(
            &signals(vec![adapter], None),
            TierRequest::Requested(Tier::B),
        );
        assert_eq!(resolved.caps.tier, Tier::B);
        assert_eq!(
            resolved.evidence.override_outcome,
            OverrideOutcome::Applied(Tier::B)
        );
        assert_eq!(resolved.evidence.measured_tier, Some(Tier::Cpu));
        assert_eq!(resolved.evidence.decided_by, DecidedBy::Override);
    }

    // -----------------------------------------------------------------------
    // The recorded bands, and SIMD.
    // -----------------------------------------------------------------------

    /// The constant in source and the recorded measurement in
    /// `ci/tier-thresholds.json` are one number. Spike gate A7.3 says do not
    /// invent a figure, and a constant that has drifted away from the file it
    /// was recorded in is an invented figure with a provenance note attached.
    #[test]
    fn the_recorded_bands_match_the_checked_in_file() {
        const FILE: &str = include_str!("../../../ci/tier-thresholds.json");
        assert_eq!(
            json_u64(FILE, "hardware_floor_pixels_per_second"),
            Some(FillRateBands::RECORDED.hardware_floor_pps)
        );
        assert_eq!(
            json_u64(FILE, "software_ceiling_pixels_per_second"),
            FillRateBands::RECORDED.software_ceiling_pps
        );
    }

    /// A deliberately small reader, so the test needs no serde and no
    /// dependency. It finds the key, steps past the colon and reads the digits
    /// that follow, which yields `None` for `null`.
    fn json_u64(text: &str, key: &str) -> Option<u64> {
        let after_key = text.split_once(&format!("\"{key}\""))?.1;
        let after_colon = after_key.split_once(':')?.1;
        let digits: String = after_colon
            .trim_start()
            .chars()
            .take_while(char::is_ascii_digit)
            .collect();
        digits.parse().ok()
    }

    /// On a native host SIMD128 is not a question that has an answer, and
    /// saying `None` there would read as "this machine cannot do SIMD".
    #[test]
    fn simd_support_is_not_applicable_off_wasm() {
        assert_eq!(SimdSupport::build_target(), SimdSupport::NotApplicable);
    }
}
