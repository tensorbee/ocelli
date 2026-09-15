//! The half of tier resolution that touches a GPU.
//!
//! `caps.rs` holds the decision and no I/O. This module holds the wgpu calls
//! and the fill-rate workload, and the split is the point: everything that can
//! be wrong is in the other file, and the other file needs no adapter to test.
//!
//! **The probe device is transient.** It is created, measured on and dropped
//! before [`resolve`] returns. It never becomes a [`crate::GpuContext`], so on
//! this path there is never a moment when two devices exist. The call sits
//! inside `ocelli-render`, which is the only crate permitted to make one, and
//! `ci/check-device-ownership.sh` still passes unchanged.
//!
//! **Recovery is the one path where two handles briefly coexist**, and this
//! header used to make the claim without that qualification.
//! [`crate::GpuContext::recover`] opens the replacement before it replaces the
//! context, so a refused rebuild leaves the caller where it started rather than
//! with no device at all. HLD section 31's invariant is about two devices
//! SHARING textures and is untouched, for the reasons set out at that call site
//! and in `docs/lld/gpu-ownership.md`.
//!
//! **Nothing is read back.** The workload shades an offscreen texture and
//! never maps it. Decision D3 says pixels never cross the boundary, and a
//! readback would put them on a path they have no business being on. It would
//! also measure transfer rather than shading.

use core::future::Future;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::caps::{
    AdapterFacts, FailedAdapter, FillRate, FillRateBands, ProbeOutcome, Resolution, SimdSupport,
    Tier, TierRequest, TierSignals, candidate_order, classify,
};
use crate::gpu::{DeviceError, GpuContext};

/// The format the workload renders into. Mandatorily renderable on every
/// backend, so the measurement is the same shape everywhere.
pub(crate) const TARGET_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

/// The ALU steps per fragment in `fill_rate.wgsl`. Recorded here so
/// `ci/tier-thresholds.json` can name the workload its figures were taken
/// with, because a fill rate without its workload is not a measurement.
pub const FILL_RATE_ALU_STEPS: u32 = 64;

/// One side of the square render target a timed run shades into.
///
/// **A newtype, and F-037 is the story `measure_with` named as the one to argue
/// it.** That function's documentation recorded the residue: `run` took
/// `edge` and `passes` as two adjacent `u32`s, the closure in `measure`
/// forwarded them positionally, and transposing them there compiled and passed
/// the whole suite.
///
/// **It passed because nothing reaches that call site, not because the numbers
/// agree.** They do not. `fragments` is `edge * edge * passes`, which is not
/// symmetric in its two arguments, so the calibration goes from 65,536
/// fragments to 256 and the full pass from 16,777,216 to 262,144, factors of
/// 256 and 64. What stays consistent is that `fragments` and `run` are
/// handed the SAME transposed pair, so the reported numerator matches the
/// trivial workload actually shaded and nothing internal disagrees. Every test
/// here supplies its own `issue` and never reaches `measure`'s closure, and
/// `the_recorded_workload_matches_the_checked_in_file` calls `fragments` on
/// `RUN_PLAN` directly rather than through that closure, so it agrees too.
/// Only a real adapter's measured rate would collapse.
///
/// **An earlier version of this paragraph said the product was unchanged**, in
/// four files, and it was inherited from the comment this replaced. It
/// understated the defect by a factor of 256 in the direction that makes the
/// newtype look less necessary than it is.
///
/// `AGENTS.md`: "Reducing cases is good even when it adds types", with
/// `Pt<Canvas>` against `Pt<World>` as this project's clearest example. Two
/// interchangeable `u32`s at a call site no test reaches is exactly that shape,
/// and the transposition is now a compile error rather than a comment
/// describing a hole.
///
/// **Public because the workload is part of a recorded measurement's
/// provenance**, which is the same reason [`FILL_RATE_ALU_STEPS`] is public:
/// `ci/tier-thresholds.json` states in terms that "a figure taken with a
/// different workload is a different measurement and does not belong in this
/// file".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Edge(pub u32);

/// How many full-viewport draws one timed run issues into its target.
///
/// The other half of [`Edge`]'s argument. `passes` is a factor of sixteen
/// between the two workloads, and it is not decoration: dividing a real
/// adapter's measured rate by sixteen pushes it under
/// [`crate::caps::FillRateBands::hardware_floor_pps`] and demotes a hardware
/// adapter to tier C.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Passes(pub u32);

/// The calibration pass: a small target, one pass.
const CALIBRATION_EDGE: Edge = Edge(256);
const CALIBRATION_PASSES: Passes = Passes(1);

/// The full pass, 256 times the calibration's fragment count.
const FULL_EDGE: Edge = Edge(1024);
const FULL_PASSES: Passes = Passes(16);

/// If the calibration took longer than this, the full pass is not affordable
/// at startup and the calibration figure is itself the answer.
///
/// Two milliseconds for 65,536 fragments is under 33 megapixels per second,
/// which is already far below anything a real adapter produces.
const CALIBRATION_BUDGET_NANOS: u64 = 2_000_000;

/// How long to wait for the submission to complete before giving up and
/// reporting no measurement at all.
///
/// A bound rather than an unbounded loop, because `Device::poll` is a no-op on
/// WebGPU, where the completion callback arrives from the event loop instead.
/// Spinning forever there would hang startup, and reporting no measurement is
/// the honest answer: the combination rule then falls through to the two
/// hints, which is exactly what it is for.
const COMPLETION_TIMEOUT_NANOS: u64 = 5_000_000_000;

/// The three runs `measure` issues, in order, as `(edge, passes)` pairs: the
/// warm-up whose figure is discarded, the calibration, and the full pass.
///
/// **A value rather than three call sites, so the workload is assertable.**
/// `ci/tier-thresholds.json` records this plan as `warm_up_pixels`,
/// `calibration_pixels` and `full_pixels`, because a fill rate without its
/// workload is not a measurement and the 400,000,000 floor in that file was
/// derived from a figure taken with exactly these three runs.
/// `caps::detection_tests::the_recorded_workload_matches_the_checked_in_file`
/// is what stops the plan and the file from drifting apart, and
/// `probe::tests::a_slow_first_submission_does_not_stop_the_full_pass` is what
/// stops the plan from being a statement `measure` does not follow.
pub(crate) const RUN_PLAN: [(Edge, Passes); 3] = [
    (CALIBRATION_EDGE, CALIBRATION_PASSES),
    (CALIBRATION_EDGE, CALIBRATION_PASSES),
    (FULL_EDGE, FULL_PASSES),
];

pub(crate) const WORKLOAD_WGSL: &str = include_str!("fill_rate.wgsl");

/// Resolve the session's tier.
///
/// `clock` returns monotonically increasing nanoseconds from any fixed origin.
/// It is the caller's, because `std::time::Instant` panics on
/// `wasm32-unknown-unknown` and the alternative is a dependency reaching
/// `performance.now()` inside this crate, which is the browser binding
/// deviation D-12 says `ocelli-render` must not grow. Natively it is an
/// `Instant`, in a browser it is the shell's `performance.now()` delta coming
/// across the boundary.
///
/// `async` because `Instance::request_adapter` and `Adapter::request_device`
/// are futures in the pinned wgpu, not because anything here wants to be.
///
/// **Infallible.** Every failure of detection is a resolved tier and a
/// recorded outcome, which is deviation D-07's honesty rule applied to the
/// resolver itself.
pub async fn resolve(request: TierRequest, clock: &mut dyn FnMut() -> u64) -> Resolution {
    detect(request, clock).await.0
}

/// Resolve the session's tier and KEEP the adapter that won.
///
/// [`resolve`] answers "which tier" and drops everything it touched, which is
/// what F-004's callers want. This answers "which tier, and what do I open a
/// device on", which is what a session wants, and it is the entry point HLD
/// section 22's rebuild needs: recovering from a device loss must not re-run
/// detection, because section 7 says the tier "resolves once at startup" and a
/// session that silently changed tier mid-flight is the quietly-different
/// answer deviation D-07 refuses.
///
/// **`None` when the session opens no device**, which is
/// [`crate::caps::opens_a_device`] and is tier C. That includes the case where
/// an adapter opened perfectly well and D-07's combination rule demoted it
/// anyway, because the resolved tier is the answer and the probe outcome is
/// only evidence for it.
///
/// **That second case is reached by no test, and this is the honest statement
/// of what is and is not covered.** The decision itself,
/// [`crate::caps::opens_a_device`], is exhaustively tested over all three tiers
/// with no adapter. What is untested is the GUARD here applying it: deleting
/// the `if` leaves the whole suite green, measured in the F-037 review's first
/// pass. `a_cpu_override_yields_no_adapter` below reaches the tier C path but
/// cannot kill that mutation, because a `Cpu` override short-circuits in
/// `detect` and the adapter is `None` for that reason as well. Killing it
/// needs a machine where a real adapter opens and the combination rule still
/// resolves tier C, which is a software rasteriser, and this repository has no
/// such machine in any gate. **F-X002 is the story that gets one**, through
/// lavapipe in CI and headless Chrome on SwiftShader, and
/// `docs/spikes/A7-tier-c.md` section A7.2 already names that as its acceptance
/// criterion. Until then the only thing watching this line is the human check
/// `docs/hld/24-agent-code-standards.md` section 27.3 requires.
///
/// **The probe device is still transient, and on this path there is still never
/// a moment when two devices exist.** This function retains an `Adapter`, which
/// is not a device. The probe's device is created, measured on and dropped
/// inside `detect` before this returns, and the long-lived one is opened
/// later, by [`ResolvedAdapter::open`], at the caller's choosing.
///
/// **The recovery path is different and says so at the line that does it.**
/// [`GpuContext::recover`] opens the replacement before it replaces the
/// context, so two handles are briefly alive there. That is deliberate, it is
/// what makes a refused rebuild safe, and the reasoning is at that call site and
/// in `docs/lld/gpu-ownership.md`.
pub async fn resolve_adapter(
    request: TierRequest,
    clock: &mut dyn FnMut() -> u64,
) -> Option<ResolvedAdapter> {
    let (resolution, adapter) = detect(request, clock).await;
    if !crate::caps::opens_a_device(&resolution.caps) {
        return None;
    }
    adapter.map(|adapter| ResolvedAdapter {
        adapter,
        resolution,
    })
}

/// The detection pass both public entry points share.
///
/// Returns the verdict, and the adapter the measurement was taken on when there
/// was one. Extracted so [`resolve`] and [`resolve_adapter`] cannot disagree
/// about how a tier is decided, which is the one thing about this module that
/// must not exist twice.
async fn detect(
    request: TierRequest,
    clock: &mut dyn FnMut() -> u64,
) -> (Resolution, Option<wgpu::Adapter>) {
    // An override of tier C short-circuits everything: no instance, no
    // adapter, no device, no benchmark. That is the estate D-07 names getting
    // its startup cost back.
    //
    // NO ASSERTION ON THE RETURN VALUE CAN CATCH DELETING THESE THREE LINES.
    // `classify` short-circuits a tier C override as well, and hardcodes
    // `adapters_seen: 0`, `fill_rate: None` and `device_created: false` into
    // the evidence it returns, while `TierSignals::unprobed()` supplies the
    // same `bands` and `simd` the long path builds. So the `Resolution` is
    // identical either way. What differs is only that an instance, an adapter
    // and a device get created and thrown away first.
    //
    // THE CALLER'S CLOCK IS WHAT OBSERVES IT, and
    // `tests::a_cpu_override_yields_no_adapter_and_spends_no_startup_cost`
    // asserts on that. `clock` is already a parameter, so counting its
    // invocations needs no new seam: this block returns before anything times
    // anything, so the count is deterministically zero, and without it `run`
    // calls the clock around its submission. It is not a timing bound, because
    // the closure returns a constant and the assertion is on the call count.
    //
    // Two earlier attempts at that test asserted the return value instead and
    // were green under this mutation, which is why the route is written out
    // here. The test needs a real adapter to bite, so it is `#[ignore]`d and
    // `bin/ocelli.sh gate gpu` runs it.
    if request.requested() == Some(Tier::Cpu) {
        return (classify(&TierSignals::unprobed(), request), None);
    }

    // `new_instance_with_webgpu_detection` rather than `Instance::new`,
    // because `navigator.gpu` can be defined on a browser that cannot actually
    // produce a WebGPU adapter, and that population is exactly the one D-07
    // cares about. On native it forwards to `Instance::new` unchanged.
    let instance = wgpu::util::new_instance_with_webgpu_detection(
        wgpu::InstanceDescriptor::new_without_display_handle(),
    )
    .await;

    let mut adapters = enumerate(&instance).await;
    let facts: Vec<AdapterFacts> = adapters.iter().map(facts_of).collect();
    let probed = measure_candidates(&adapters, &facts, clock).await;
    let chosen = probed.chosen;

    let signals = TierSignals {
        probe: probed.outcome,
        simd: SimdSupport::build_target(),
        bands: FillRateBands::RECORDED,
    };
    let resolution = classify(&signals, request);

    // `swap_remove` rather than indexing a clone: the winner is taken out by
    // value and the rest are dropped with the vector. Order does not survive,
    // and nothing after this point reads the vector.
    //
    // Dropping `instance` here is safe. `wgpu::Adapter` holds a refcounted
    // handle to the same context, so the adapter outlives the instance value.
    let adapter =
        chosen.and_then(|index| (index < adapters.len()).then(|| adapters.swap_remove(index)));
    (resolution, adapter)
}

/// The adapter a session resolved on, retained so a lost device can be rebuilt.
///
/// **This is not a device and holds none.** HLD section 31's one-device
/// invariant is about `request_device`, and this type only makes it possible to
/// call it again later without a second detection pass.
#[derive(Debug)]
pub struct ResolvedAdapter {
    adapter: wgpu::Adapter,
    resolution: Resolution,
}

impl ResolvedAdapter {
    /// What the tier resolved to, and the evidence it resolved on.
    #[must_use]
    pub fn resolution(&self) -> &Resolution {
        &self.resolution
    }

    /// Open the session's one long-lived device.
    ///
    /// **This and the probe's are the only two `request_device` calls in the
    /// workspace, and both are in `ocelli-render`**, which
    /// `ci/check-device-ownership.sh` asserts on every push. HLD section 31:
    /// "Two devices cannot share textures, which would defeat the entire
    /// point."
    ///
    /// The returned [`GpuContext`] is already watching itself for loss, because
    /// [`GpuContext::new`] registers the callback. There is no window in which
    /// a context exists and is unobserved.
    ///
    /// # Errors
    ///
    /// [`DeviceError::Refused`] carrying wgpu's own diagnostic text, which is
    /// explicitly unstable and is for a human.
    pub async fn open(&self) -> Result<GpuContext, DeviceError> {
        let descriptor = wgpu::DeviceDescriptor {
            label: Some("ocelli device"),
            required_features: wgpu::Features::empty(),
            // The adapter's OWN limits, never `Limits::default()`. A downlevel
            // GL adapter does not meet the WebGPU defaults, which is the whole
            // reason tier B exists, and asking for the defaults on one is how a
            // tier B session fails to start at all. `measure_candidates` makes
            // the same choice for the probe device and for the same reason.
            //
            // NO TEST IN THIS REPOSITORY CATCHES A REGRESSION HERE, and that is
            // measured rather than assumed. F-037 replaced this line with
            // `wgpu::Limits::default()` and `cargo test -p ocelli-render --test
            // device -- --ignored` stayed at exit 0, 6 passed and 0 failed, on
            // the aarch64-apple-darwin Metal adapter in
            // `ci/tier-thresholds.json`. A tier A adapter exceeds the WebGPU
            // defaults, so asking for them succeeds and the mutation is
            // invisible. Only a downlevel adapter separates the two, and HLD
            // section 7's tier B has never been exercised by anything in this
            // project. F-042 is the WebGL2 story and F-X002 is the story that
            // gets a software adapter into a test, and until one of them lands
            // the only thing watching this line is the human check
            // `docs/hld/24-agent-code-standards.md` section 27.3 requires.
            required_limits: self.adapter.limits(),
            ..Default::default()
        };
        let (device, queue) = self
            .adapter
            .request_device(&descriptor)
            .await
            // `RequestDeviceError` exposes its reason only through `Display`.
            .map_err(|error| DeviceError::Refused(error.to_string()))?;
        Ok(GpuContext::new(device, queue, self.resolution.caps))
    }
}

/// Every adapter the instance offers.
///
/// Natively that is enumeration across all backends, which is the only way to
/// see both an A-candidate and a B-candidate on the same host.
#[cfg(not(target_arch = "wasm32"))]
async fn enumerate(instance: &wgpu::Instance) -> Vec<wgpu::Adapter> {
    instance.enumerate_adapters(wgpu::Backends::all()).await
}

/// In a browser, enumeration is not meaningful: the platform hands out one
/// adapter for the options it was asked with, so asking is the only route.
#[cfg(target_arch = "wasm32")]
async fn enumerate(instance: &wgpu::Instance) -> Vec<wgpu::Adapter> {
    instance
        .request_adapter(&wgpu::RequestAdapterOptions::default())
        .await
        .into_iter()
        .collect()
}

fn facts_of(adapter: &wgpu::Adapter) -> AdapterFacts {
    let info = adapter.get_info();
    let limits = adapter.limits();
    let downlevel = adapter.get_downlevel_capabilities();
    AdapterFacts {
        backend: info.backend,
        device_type: info.device_type,
        // On wasm32 this is already the `WEBGL_debug_renderer_info` unmasked
        // renderer string A7 asks for. wgpu's GLES backend reads
        // `GL_UNMASKED_RENDERER_WEBGL` when the extension is available, so
        // this crate imports no web-sys of its own to see it.
        name: info.name,
        driver: info.driver,
        driver_info: info.driver_info,
        compute_shaders: downlevel
            .flags
            .contains(wgpu::DownlevelFlags::COMPUTE_SHADERS),
        webgpu_compliant: downlevel.is_webgpu_compliant(),
        max_tex_3d: limits.max_texture_dimension_3d,
        max_buffer: limits.max_buffer_size,
    }
}

/// Try candidates in preference order, stopping after the first device opens.
///
/// Every failed request is retained with the adapter facts and wgpu's unstable
/// diagnostic text. `NoDevice` therefore means every candidate was tried. A
/// failed A candidate followed by a working B candidate produces `Opened` for
/// B, so classification and override clamping cannot mistake the failed A
/// adapter for a constructible tier.
enum AttemptOutcome<T> {
    NoAdapter {
        adapters_seen: usize,
    },
    NoDevice {
        adapters_seen: usize,
        failed: Vec<FailedAdapter>,
    },
    Opened {
        adapters_seen: usize,
        failed: Vec<FailedAdapter>,
        adapter: AdapterFacts,
        /// Which candidate won, as an index into the caller's adapter list.
        ///
        /// F-037 needs this and `AdapterFacts` cannot supply it: the facts are
        /// a description, and two adapters can legitimately describe the same,
        /// so matching on them to find the winner would be a lookup that can be
        /// ambiguous. The index is what the loop already knew.
        index: usize,
        value: T,
    },
}

/// Run the production candidate-order and continuation control around one
/// supplied device attempt.
///
/// The concrete wgpu caller and deterministic tests are today's distinct
/// instantiations. Keeping the asynchronous operation at this boundary lets
/// the tests exercise the same loop without adding a mock trait.
async fn attempt_candidates<T, Attempt, AttemptFuture>(
    facts: &[AdapterFacts],
    mut attempt: Attempt,
) -> AttemptOutcome<T>
where
    Attempt: FnMut(usize) -> AttemptFuture,
    AttemptFuture: Future<Output = Result<T, String>>,
{
    let adapters_seen = facts.len();
    let mut failed = Vec::new();
    for index in candidate_order(facts) {
        let Some(adapter_facts) = facts.get(index) else {
            continue;
        };
        match attempt(index).await {
            Ok(value) => {
                return AttemptOutcome::Opened {
                    adapters_seen,
                    failed,
                    adapter: adapter_facts.clone(),
                    index,
                    value,
                };
            }
            Err(reason) => failed.push(FailedAdapter {
                adapter: adapter_facts.clone(),
                reason,
            }),
        }
    }
    if failed.is_empty() {
        AttemptOutcome::NoAdapter { adapters_seen }
    } else {
        AttemptOutcome::NoDevice {
            adapters_seen,
            failed,
        }
    }
}

/// A probe outcome, and which adapter produced it.
///
/// `ProbeOutcome` is the evidence `classify` reads and it names the winner by
/// its `AdapterFacts`. `chosen` is the same winner by index, which is what
/// `detect` needs to take the `wgpu::Adapter` itself out of the list. They
/// are two views of one decision made in one place, not two decisions.
struct Probed {
    outcome: ProbeOutcome,
    chosen: Option<usize>,
}

async fn measure_candidates(
    adapters: &[wgpu::Adapter],
    facts: &[AdapterFacts],
    clock: &mut dyn FnMut() -> u64,
) -> Probed {
    let attempted = attempt_candidates(facts, |index| {
        let adapter = adapters.get(index);
        async move {
            let Some(adapter) = adapter else {
                return Err("candidate index was absent from the adapter list".to_owned());
            };
            let descriptor = wgpu::DeviceDescriptor {
                label: Some("ocelli tier probe"),
                required_features: wgpu::Features::empty(),
                // The adapter's OWN limits, never `Limits::default()`. A
                // downlevel GL adapter does not meet the WebGPU defaults,
                // which is the whole reason tier B exists.
                required_limits: adapter.limits(),
                ..Default::default()
            };
            adapter
                .request_device(&descriptor)
                .await
                // `RequestDeviceError` exposes its reason only through
                // `Display`. This text is diagnostic and explicitly unstable.
                .map_err(|error| error.to_string())
        }
    })
    .await;

    match attempted {
        AttemptOutcome::NoAdapter { adapters_seen } => Probed {
            outcome: ProbeOutcome::NoAdapter { adapters_seen },
            chosen: None,
        },
        AttemptOutcome::NoDevice {
            adapters_seen,
            failed,
        } => Probed {
            outcome: ProbeOutcome::NoDevice {
                adapters_seen,
                failed,
            },
            chosen: None,
        },
        AttemptOutcome::Opened {
            adapters_seen,
            failed,
            adapter,
            index,
            value: (device, queue),
        } => {
            let fill_rate = measure(&device, &queue, clock);
            // The probe device and queue are dropped HERE, at the end of this
            // scope, before anything opens the long-lived one. The module
            // header's "there is never a moment when two devices exist" is this
            // line, and F-037 did not move it.
            drop((device, queue));
            Probed {
                outcome: ProbeOutcome::Opened {
                    adapters_seen,
                    failed,
                    adapter,
                    fill_rate,
                },
                chosen: Some(index),
            }
        }
    }
}

/// The fragments one timed run shades: every texel of an `edge` by `edge`
/// target, once per pass.
///
/// **The numerator of the fill rate, extracted so the floor can assert it.**
/// It was an expression inside `run`, and the only test over it recomputed
/// the same expression in its own body, which asserts the arithmetic against
/// itself: dropping the `passes` factor left the crate green. `passes` is not
/// decoration, it is a factor of sixteen between the two workloads, and
/// dividing a real adapter's measured rate by sixteen pushes it under
/// [`crate::caps::FillRateBands::hardware_floor_pps`] and demotes a hardware
/// adapter to tier C, which is deviation D-07's misdetection arriving from
/// the direction the resolver is supposed to catch.
///
/// A plain `fn` rather than a `const fn`. `u64::from` is not callable in a
/// const function on stable, and the alternative is `as`, which HLD section
/// 27.3 makes a human review item and the workspace denies for truncation.
/// The constant-ness buys nothing here and the cast would cost a review.
pub(crate) fn fragments(edge: Edge, passes: Passes) -> u64 {
    u64::from(edge.0) * u64::from(edge.0) * u64::from(passes.0)
}

/// Whether the full pass is affordable, given what the calibration cost.
///
/// [`CALIBRATION_BUDGET_NANOS`] is a ceiling ON the calibration, not a figure
/// the calibration has to beat: the rule is "if the calibration took LONGER
/// than this", and a calibration that took exactly the budget did not take
/// longer than it, so the full pass still runs. Extracted from `measure` so
/// the boundary is assertable without an adapter, which deviation D-04 leaves
/// the floor without.
const fn full_pass_is_affordable(calibration_nanos: u64) -> bool {
    calibration_nanos <= CALIBRATION_BUDGET_NANOS
}

/// The two-stage measurement, on a real device.
///
/// A small calibration pass first, so a slow rasteriser does not hang startup.
/// The full pass runs only if the calibration was fast enough to make it
/// affordable, and if it was not, the calibration figure is itself the answer.
///
/// This half builds the pipeline and hands `measure_with` a way to issue one
/// run. Everything that decides anything is in that function, which needs no
/// adapter, for the reason the module header gives about `caps` and `probe`.
fn measure(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    clock: &mut dyn FnMut() -> u64,
) -> Option<FillRate> {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("ocelli fill rate"),
        source: wgpu::ShaderSource::Wgsl(WORKLOAD_WGSL.into()),
    });
    // Compiled once, before the clock starts. HLD section 22: pipelines
    // compile at init, never mid-frame, and the same discipline applies to a
    // measurement that would otherwise be timing a shader compile.
    let pipeline = build_pipeline(device, &shader);

    measure_with(&mut |edge, passes| run(device, queue, &pipeline, edge, passes, clock))
}

/// [`RUN_PLAN`], issued in order, and the figure that comes out of it.
///
/// `issue` is `run` in the resolver and a recording stand-in in the tests,
/// which deviation D-04 leaves without an adapter. It is a `&mut dyn FnMut` for
/// the same reason `clock` is one on [`resolve`]: the alternative is a generic
/// parameter with one instantiation, and `AGENTS.md` refuses that shape.
///
/// **The transposition this function's documentation used to name as open is
/// CLOSED, by F-037.** `edge` and `passes` were both `u32` and adjacent, the
/// closure in `measure` forwarded them positionally, and transposing them
/// there, `run(device, queue, &pipeline, passes, edge, clock)`, compiled and
/// passed the whole suite. It passed because every test here supplies its own
/// `issue` and never reaches that call, **not** because the numbers agree:
/// `fragments` is `edge * edge * passes` and is not symmetric, so the
/// calibration drops from 65,536 fragments to 256. `fragments` and `run`
/// are handed the same transposed pair, so the reported numerator matches the
/// trivial workload actually shaded and nothing internal disagrees. Only a real
/// adapter's measured rate would have collapsed. See [`Edge`].
///
/// [`Edge`] and [`Passes`] make that transposition a compile error, so the
/// residue is gone rather than smaller, and the human check
/// `docs/hld/24-agent-code-standards.md` section 27.3 requires is no longer the
/// only thing watching it. `tests/ui/workload_dimensions_are_not_interchangeable.rs`
/// asserts the refusal, because a type that merely happens to differ today and
/// a type whose difference is checked read identically in a diff.
///
/// **The first run's figure is thrown away, and that discard is the whole
/// reason this function is separately testable.** The FIRST submission on a
/// fresh device pays for lazy pipeline compilation, driver initialisation and
/// command-buffer setup, and timing it measures startup latency rather than
/// fill rate. Measured on the machine in `ci/tier-thresholds.json`, twenty
/// release runs of this exact pair in fresh processes: the first 65,536
/// fragments took 5.4 to 9.3 ms, median 6.1, and the same work immediately
/// afterwards took 0.32 to 0.70 ms, median 0.36. The ratio ranged from 8.8 to
/// 20.7 and is one machine's noise rather than a bound.
///
/// The only figure this code depends on is the low end. Without the discard,
/// the calibration measures that first-run cost as the machine's fill rate, and
/// the first submission was over [`CALIBRATION_BUDGET_NANOS`] on all twenty
/// runs, so the full pass never runs and the startup latency is the figure that
/// gets recorded. Divided by the 256 the full pass would have brought, a
/// hardware adapter falls under
/// [`crate::caps::FillRateBands::hardware_floor_pps`] and is demoted to tier C,
/// which renders nothing and presents as a slow viewer. That is deviation
/// D-07's misdetection arriving from the direction the resolver exists to
/// catch.
fn measure_with(issue: &mut dyn FnMut(Edge, Passes) -> Option<FillRate>) -> Option<FillRate> {
    let [
        (warm_up_edge, warm_up_passes),
        (calibration_edge, calibration_passes),
        (full_edge, full_passes),
    ] = RUN_PLAN;

    let _warm_up = issue(warm_up_edge, warm_up_passes);

    let calibration = issue(calibration_edge, calibration_passes)?;
    if !full_pass_is_affordable(calibration.elapsed_nanos) {
        return Some(calibration);
    }
    issue(full_edge, full_passes).or(Some(calibration))
}

fn build_pipeline(device: &wgpu::Device, shader: &wgpu::ShaderModule) -> wgpu::RenderPipeline {
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("ocelli fill rate"),
        layout: None,
        vertex: wgpu::VertexState {
            module: shader,
            entry_point: Some("vs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            buffers: &[],
        },
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        fragment: Some(wgpu::FragmentState {
            module: shader,
            entry_point: Some("fs_main"),
            compilation_options: wgpu::PipelineCompilationOptions::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format: TARGET_FORMAT,
                blend: None,
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multiview_mask: None,
        cache: None,
    })
}

/// One timed run: `passes` full-viewport draws into an `edge` by `edge`
/// target, encoded into one command buffer and issued as one submission.
fn run(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    pipeline: &wgpu::RenderPipeline,
    edge: Edge,
    passes: Passes,
    clock: &mut dyn FnMut() -> u64,
) -> Option<FillRate> {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("ocelli fill rate target"),
        size: wgpu::Extent3d {
            width: edge.0,
            height: edge.0,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: TARGET_FORMAT,
        // RENDER_ATTACHMENT and nothing else. No COPY_SRC, because nothing
        // ever copies out of it.
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("ocelli fill rate"),
    });
    for _ in 0..passes.0 {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("ocelli fill rate pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                    // Store rather than Discard, so a driver cannot decide the
                    // whole pass was dead and skip the shading we are timing.
                    store: wgpu::StoreOp::Store,
                },
            })],
            ..Default::default()
        });
        pass.set_pipeline(pipeline);
        pass.draw(0..3, 0..1);
    }

    let done = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&done);
    let started = clock();
    let _submission = queue.submit(std::iter::once(encoder.finish()));
    // Registered after the submit, because the callback is about the previous
    // one. `Device::poll` blocks on wgpu-core backends and is documented as a
    // no-op on WebGPU, so the flag is the portable signal and the poll is what
    // drives it natively.
    queue.on_submitted_work_done(move || flag.store(true, Ordering::SeqCst));

    while !done.load(Ordering::SeqCst) {
        if clock().saturating_sub(started) > COMPLETION_TIMEOUT_NANOS {
            return None;
        }
        if device.poll(wgpu::PollType::Poll).is_err() {
            return None;
        }
    }
    let elapsed_nanos = clock().saturating_sub(started);

    Some(FillRate {
        // Every texel of the target is covered by the oversized triangle
        // exactly once per pass, so this is the fragment count and not an
        // estimate.
        pixels_shaded: fragments(edge, passes),
        elapsed_nanos,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        AttemptOutcome, CALIBRATION_BUDGET_NANOS, CALIBRATION_EDGE, CALIBRATION_PASSES, Edge,
        FULL_EDGE, FULL_PASSES, Passes, RUN_PLAN, WORKLOAD_WGSL, attempt_candidates, fragments,
        full_pass_is_affordable, measure_with,
    };
    use crate::caps::{AdapterFacts, FillRate};

    fn fallback_facts() -> Vec<AdapterFacts> {
        vec![
            AdapterFacts {
                backend: wgpu::Backend::Vulkan,
                device_type: wgpu::DeviceType::DiscreteGpu,
                name: "preferred".into(),
                driver: String::new(),
                driver_info: String::new(),
                compute_shaders: true,
                webgpu_compliant: true,
                max_tex_3d: 4096,
                max_buffer: 1 << 30,
            },
            AdapterFacts {
                backend: wgpu::Backend::Gl,
                device_type: wgpu::DeviceType::IntegratedGpu,
                name: "fallback".into(),
                driver: String::new(),
                driver_info: String::new(),
                compute_shaders: false,
                webgpu_compliant: false,
                max_tex_3d: 2048,
                max_buffer: 1 << 29,
            },
        ]
    }

    #[test]
    fn a_failed_first_attempt_continues_to_the_successful_second_candidate() {
        let facts = fallback_facts();
        let mut attempted = Vec::new();
        let outcome = pollster::block_on(attempt_candidates(&facts, |index| {
            attempted.push(index);
            std::future::ready(if index == 0 {
                Err("preferred failed".to_owned())
            } else {
                Ok(())
            })
        }));

        assert_eq!(attempted, vec![0, 1]);
        let opened = match outcome {
            AttemptOutcome::Opened {
                failed,
                adapter,
                value: (),
                ..
            } => Some((failed, adapter)),
            AttemptOutcome::NoAdapter { .. } | AttemptOutcome::NoDevice { .. } => None,
        };
        assert_eq!(
            opened.as_ref().map(|(_, adapter)| adapter.name.as_str()),
            Some("fallback")
        );
        assert_eq!(opened.as_ref().map(|(failed, _)| failed.len()), Some(1));
        assert_eq!(
            opened
                .as_ref()
                .and_then(|(failed, _)| failed.first())
                .map(|item| item.reason.as_str()),
            Some("preferred failed")
        );
    }

    #[test]
    fn exhausting_the_attempt_loop_retains_every_failure() {
        let facts = fallback_facts();
        let mut attempted = Vec::new();
        let outcome = pollster::block_on(attempt_candidates(&facts, |index| {
            attempted.push(index);
            std::future::ready(Err::<(), _>(format!("failure {index}")))
        }));

        assert_eq!(attempted, vec![0, 1]);
        let failed = match outcome {
            AttemptOutcome::NoDevice { failed, .. } => Some(failed),
            AttemptOutcome::NoAdapter { .. } | AttemptOutcome::Opened { .. } => None,
        };
        assert_eq!(failed.as_ref().map(Vec::len), Some(2));
        assert_eq!(
            failed
                .as_ref()
                .and_then(|items| items.first())
                .map(|item| item.reason.as_str()),
            Some("failure 0")
        );
        assert_eq!(
            failed
                .as_ref()
                .and_then(|items| items.get(1))
                .map(|item| item.reason.as_str()),
            Some("failure 1")
        );
    }

    /// Issue [`RUN_PLAN`] against a stand-in for [`super::run`] that records
    /// what it was asked for and reports `elapsed_nanos` for every run.
    ///
    /// The fragment count it hands back is the production `fragments` of the
    /// pair it was given, so a run issued at the wrong size reports the wrong
    /// numerator here exactly as it would on a device.
    fn issued(elapsed_nanos: u64) -> (Vec<u64>, Option<FillRate>) {
        let mut counts = Vec::new();
        let result = measure_with(&mut |edge, passes| {
            let pixels_shaded = fragments(edge, passes);
            counts.push(pixels_shaded);
            Some(FillRate {
                pixels_shaded,
                elapsed_nanos,
            })
        });
        (counts, result)
    }

    /// **The fragment count, against figures computed by hand from what the
    /// workload does, and not against the expression that produces it.**
    ///
    /// Deviation D-07 judges an adapter on pixels shaded per second, and this
    /// count is that rate's numerator. The workload draws one oversized
    /// triangle covering every texel of the target exactly once, and it does
    /// that once per pass, so a run of `passes` passes over an `edge` by
    /// `edge` target shades `edge * edge * passes` fragments.
    ///
    /// The first three rows have a `passes` greater than one and a product
    /// `edge * edge` alone cannot reach, which is what makes the factor load
    /// bearing. Dropping it from the production expression left the crate
    /// green, because the only test over it rebuilt `edge * edge * passes` in
    /// its own body from the same constants and so asserted the arithmetic
    /// against itself. Sixteen of the seventeen fragments of the full pass
    /// would then vanish from the numerator, a real adapter's measured rate
    /// would be divided by sixteen, and a machine over
    /// `FillRateBands::hardware_floor_pps` would fall under it and be demoted
    /// to tier C, which renders nothing and presents as a slow viewer.
    #[test]
    fn the_fragment_count_is_every_texel_once_per_pass() {
        // Four texels, three passes, twelve fragments.
        assert_eq!(fragments(Edge(2), Passes(3)), 12);
        // Sixteen texels, sixteen passes, 256 fragments.
        assert_eq!(fragments(Edge(4), Passes(16)), 256);
        // One texel, two passes, and a pass is still a pass.
        assert_eq!(fragments(Edge(1), Passes(2)), 2);
        // The smallest case there is.
        assert_eq!(fragments(Edge(1), Passes(1)), 1);
        // The two workloads. 256 * 256 is 65,536, over one pass.
        assert_eq!(fragments(CALIBRATION_EDGE, CALIBRATION_PASSES), 65_536);
        // 1024 * 1024 is 1,048,576 texels, over sixteen passes.
        assert_eq!(fragments(FULL_EDGE, FULL_PASSES), 16_777_216);
    }

    /// The full pass has to be enough larger than the calibration that a real
    /// adapter's figure is not dominated by submission overhead, and the
    /// calibration has to be small enough that a rasteriser reaches the budget
    /// rather than hanging startup.
    ///
    /// Both figures come from the production `fragments`, so the ratio is a
    /// claim about the workloads this crate actually issues.
    #[test]
    fn the_full_pass_is_much_larger_than_the_calibration() {
        let calibration = fragments(CALIBRATION_EDGE, CALIBRATION_PASSES);
        let full = fragments(FULL_EDGE, FULL_PASSES);
        assert_eq!(calibration, 65_536);
        assert_eq!(full, 16_777_216);
        assert_eq!(full / calibration, 256);
    }

    /// The budget is a startup cost, not a timeout. Two milliseconds is what a
    /// startup classifier can afford to spend finding out it is on a
    /// rasteriser.
    #[test]
    fn the_calibration_budget_is_two_milliseconds() {
        assert_eq!(CALIBRATION_BUDGET_NANOS, 2_000_000);
    }

    /// **The budget is a ceiling ON the calibration, and the boundary belongs
    /// to the affordable side.** The rule is "if the calibration took longer
    /// than this", and a calibration that took exactly the budget did not take
    /// longer than the budget, so the full pass still runs.
    ///
    /// The boundary and both its neighbours, because a comparison that had
    /// collapsed to one side satisfies a test that only probes the far ends.
    /// It is driven through [`full_pass_is_affordable`] rather than through
    /// [`super::measure`], which needs an adapter that deviation D-04 leaves
    /// the CI floor without.
    ///
    /// Getting this wrong is not a wrong number, it is a MISSING one: the full
    /// pass never runs, every machine is classified on the 65,536-fragment
    /// calibration, and that figure carries the submission overhead the full
    /// pass exists to dilute.
    #[test]
    fn the_full_pass_runs_at_the_budget_and_not_past_it() {
        assert!(full_pass_is_affordable(0));
        assert!(full_pass_is_affordable(1_999_999));
        assert!(
            full_pass_is_affordable(CALIBRATION_BUDGET_NANOS),
            "a calibration that took exactly the budget was read as over it"
        );
        assert!(!full_pass_is_affordable(2_000_001));
        assert!(!full_pass_is_affordable(u64::MAX));
    }

    /// **Three runs, in the order and at the sizes `ci/tier-thresholds.json`
    /// records the workload as.**
    ///
    /// The figures are that file's `warm_up_pixels`, `calibration_pixels` and
    /// `full_pixels`, written as literals rather than rebuilt from the
    /// constants, so this asserts the workload the 400,000,000 floor was
    /// derived from rather than asserting the code against itself.
    /// `caps::detection_tests::the_recorded_workload_matches_the_checked_in_file`
    /// is what ties those literals to the file.
    #[test]
    fn a_measurement_issues_the_warm_up_the_calibration_and_the_full_pass() {
        let (counts, result) = issued(1);
        assert_eq!(
            counts,
            vec![65_536, 65_536, 16_777_216],
            "the runs issued are not the recorded workload"
        );
        assert_eq!(result.map(|rate| rate.pixels_shaded), Some(16_777_216));
    }

    /// **The measured case, and the reason the first run's figure is
    /// discarded.**
    ///
    /// A first submission of 6.1 ms and 0.36 ms afterwards, the two medians of
    /// the twenty release runs recorded on [`super::measure_with`] and in
    /// `docs/lld/tier-resolution.md`. 6.1 ms is over
    /// [`CALIBRATION_BUDGET_NANOS`], so without the discard the calibration
    /// inherits it, [`super::full_pass_is_affordable`] says no, and the figure
    /// this function returns is 65,536 fragments of startup latency instead of
    /// 16,777,216 fragments of shading. That is a rate roughly seventeen times
    /// too low on the machine `ci/tier-thresholds.json` was recorded on, which
    /// puts a real adapter under
    /// [`crate::caps::FillRateBands::hardware_floor_pps`] and demotes it to
    /// tier C.
    ///
    /// Three statements said the warm-up was load bearing, in the comment
    /// beside it, in `ci/tier-thresholds.json` and in the LLD, and deleting it
    /// left this crate green until the S03 review's eighth pass.
    #[test]
    fn a_slow_first_submission_does_not_stop_the_full_pass() {
        let mut counts = Vec::new();
        let result = measure_with(&mut |edge, passes| {
            let pixels_shaded = fragments(edge, passes);
            counts.push(pixels_shaded);
            Some(FillRate {
                pixels_shaded,
                elapsed_nanos: if counts.len() == 1 {
                    6_100_000
                } else {
                    360_000
                },
            })
        });
        assert_eq!(
            counts.len(),
            3,
            "the first submission's cost was not discarded, so the full pass never ran"
        );
        assert_eq!(
            result.map(|rate| rate.pixels_shaded),
            Some(16_777_216),
            "the recorded figure is the calibration's, which carries startup latency"
        );
    }

    /// A calibration over the budget is itself the answer, and the full pass is
    /// not issued. The warm-up still is: it is a discard, not a stage the
    /// budget gates.
    #[test]
    fn an_unaffordable_calibration_stops_before_the_full_pass() {
        let (counts, result) = issued(CALIBRATION_BUDGET_NANOS + 1);
        assert_eq!(counts, vec![65_536, 65_536]);
        assert_eq!(result.map(|rate| rate.pixels_shaded), Some(65_536));
    }

    /// A calibration that could not be taken is no measurement at all, and the
    /// combination rule then falls through to the two hints. The warm-up's
    /// failure is not that, because its figure was never going to be used.
    #[test]
    fn a_calibration_that_reports_nothing_yields_no_measurement() {
        let mut issues = 0_u32;
        let result = measure_with(&mut |_edge, _passes| {
            issues += 1;
            None
        });
        assert_eq!(result, None);
        assert_eq!(issues, 2, "the calibration's failure did not stop the plan");
    }

    /// **A full pass that reports nothing keeps the calibration figure.**
    ///
    /// This is `.or(Some(calibration))` at the end of [`super::measure_with`],
    /// and nothing drove it: every other test here either fails at the
    /// calibration or succeeds at all three, so no test reached the third run
    /// and made it fail. Measured in the S03 sprint review's ninth pass,
    /// dropping the `.or` left the crate green.
    ///
    /// The consequence of dropping it is not a wrong number, it is a discarded
    /// one. A calibration was taken, it was affordable, and it is a real
    /// measurement of this adapter. Returning `None` throws it away, `classify`
    /// falls through to the adapter type and the renderer string, and D-07's
    /// combination rule is then deciding with one signal fewer than it had.
    /// The third run is the one most likely to fail on a slow adapter, because
    /// it is 256 times the work and the one that can reach
    /// [`COMPLETION_TIMEOUT_NANOS`], so this is the failure path of the
    /// population the resolver exists for.
    ///
    /// The calibration is deliberately given a DIFFERENT elapsed time from the
    /// warm-up, so a result carrying the warm-up's figure instead is caught
    /// too.
    #[test]
    fn a_full_pass_that_reports_nothing_keeps_the_calibration() {
        let mut counts = Vec::new();
        let result = measure_with(&mut |edge, passes| {
            let pixels_shaded = fragments(edge, passes);
            counts.push(pixels_shaded);
            match counts.len() {
                1 => Some(FillRate {
                    pixels_shaded,
                    elapsed_nanos: 900_000,
                }),
                2 => Some(FillRate {
                    pixels_shaded,
                    elapsed_nanos: 360_000,
                }),
                _ => None,
            }
        });
        assert_eq!(
            counts,
            vec![65_536, 65_536, 16_777_216],
            "the full pass was not attempted"
        );
        assert_eq!(
            result,
            Some(FillRate {
                pixels_shaded: 65_536,
                elapsed_nanos: 360_000,
            }),
            "a calibration already taken was discarded when the full pass failed"
        );
    }

    /// The two failures are different answers, which is the whole point of the
    /// fallback and is not asserted by either test on its own.
    ///
    /// A calibration that fails yields `None`, because there is no
    /// measurement. A full pass that fails yields the calibration, because
    /// there is one. A `measure_with` that returned `None` for both, or the
    /// calibration for both, satisfies exactly one of the two tests above and
    /// this one separates them.
    #[test]
    fn the_two_failure_points_do_not_give_the_same_answer() {
        let fails_at = |stage: usize| {
            let mut issues = 0_usize;
            measure_with(&mut |edge, passes| {
                issues += 1;
                if issues == stage {
                    return None;
                }
                Some(FillRate {
                    pixels_shaded: fragments(edge, passes),
                    elapsed_nanos: 360_000,
                })
            })
        };
        assert_eq!(fails_at(2), None);
        assert_eq!(
            fails_at(3).map(|rate| rate.pixels_shaded),
            Some(65_536),
            "the full pass's failure was treated as the calibration's"
        );
        assert_ne!(fails_at(2), fails_at(3));
    }

    /// **A tier C override yields no `ResolvedAdapter` and spends no startup
    /// cost getting there.**
    ///
    /// Deviation D-07 names the estate that overrides to tier C getting its
    /// startup cost back, and a `detect` that enumerated adapters before
    /// reading the override would spend it anyway.
    ///
    /// **The `clock` parameter is what makes that assertable**, and two earlier
    /// versions of this test missed it. `resolve_adapter` already takes
    /// `clock: &mut dyn FnMut() -> u64` from the caller, so counting its
    /// invocations is an assertion on a parameter that already exists. The
    /// short circuit returns before any instance, adapter or device is created,
    /// so nothing times anything and the count is deterministically zero.
    /// Delete the short circuit and `measure` runs, `run` calls `clock`
    /// around its submission, and the count is non-zero.
    ///
    /// **This is not a timing bound.** The closure returns a constant `0` and
    /// the assertion is on how many times it was CALLED, so there is no
    /// duration, no threshold and nothing to be flaky. A timing bound was
    /// rejected for this, because a permanently flaky gate is a gate that gets
    /// disabled, and so was an injectable instance factory, which would be a
    /// seam with one production caller.
    ///
    /// **`#[ignore]`d because the killing power needs an adapter that OPENS.**
    /// `clock` is reached only from `measure`, which only the `Opened` arm of
    /// `measure_candidates` calls. So with the short circuit deleted the
    /// mutation still survives on a machine that enumerates nothing, and also
    /// on one that enumerates an adapter whose device request fails: both reach
    /// `ProbeOutcome` variants that never time anything. `bin/ocelli.sh gate
    /// gpu` is what runs this, on a machine where a device does open.
    ///
    /// The floor's coverage of the same DECISION is
    /// `caps::tests::tiers_a_and_b_open_a_device_and_tier_c_does_not`, which
    /// drives `opens_a_device` over all three tiers with no adapter at all.
    /// What only this test adds is that `detect` honours that decision before
    /// spending any startup cost on it.
    #[test]
    #[ignore = "the startup-cost assertion only bites where adapters exist (D-04)"]
    fn a_cpu_override_yields_no_adapter_and_spends_no_startup_cost() {
        let mut ticks = 0_u32;
        let resolved = {
            let mut clock = || {
                ticks += 1;
                0_u64
            };
            pollster::block_on(super::resolve_adapter(
                crate::caps::TierRequest::Requested(crate::caps::Tier::Cpu),
                &mut clock,
            ))
        };

        assert!(
            resolved.is_none(),
            "a tier C override was handed an adapter to open a device on"
        );
        assert_eq!(
            ticks, 0,
            "the tier C override spent startup cost anyway, so the short circuit is gone"
        );
    }

    /// [`RUN_PLAN`] is what [`super::measure_with`] issues, and the three pairs
    /// are the two workload sizes this module declares.
    #[test]
    fn the_run_plan_is_the_calibration_twice_and_then_the_full_pass() {
        assert_eq!(
            RUN_PLAN,
            [
                (CALIBRATION_EDGE, CALIBRATION_PASSES),
                (CALIBRATION_EDGE, CALIBRATION_PASSES),
                (FULL_EDGE, FULL_PASSES),
            ]
        );
    }

    /// The shader is the workload, so its entry points and its ALU step count
    /// are part of what `ci/tier-thresholds.json` records a figure against.
    #[test]
    fn the_workload_shader_declares_both_entry_points_and_its_step_count() {
        assert!(WORKLOAD_WGSL.contains("fn vs_main"));
        assert!(WORKLOAD_WGSL.contains("fn fs_main"));
        assert!(WORKLOAD_WGSL.contains("step < 64"));
        assert_eq!(super::FILL_RATE_ALU_STEPS, 64);
    }

    /// The instrument that takes the figures in `ci/tier-thresholds.json`.
    ///
    /// **`#[ignore]` and it is not a skipped test.** It needs a real adapter,
    /// and deviation D-04 leaves CI without one, so a test that ran by default
    /// would be red on every machine the project actually builds on. It is
    /// invoked by hand when a band is being recorded:
    ///
    /// ```text
    /// cargo test -p ocelli-render -- --ignored --nocapture \
    ///     measures_a_fill_rate_on_this_machine
    /// ```
    ///
    /// A7.2 is explicit that no software adapter is an oracle, so this asserts
    /// nothing about which tier a machine resolves. It prints, and a human
    /// records.
    #[test]
    #[ignore = "needs a real GPU adapter, which the CI floor does not have"]
    fn measures_a_fill_rate_on_this_machine() {
        let start = std::time::Instant::now();
        let mut clock = || u64::try_from(start.elapsed().as_nanos()).unwrap_or(u64::MAX);
        let resolved =
            pollster::block_on(super::resolve(crate::caps::TierRequest::Auto, &mut clock));
        println!("caps      {:?}", resolved.caps);
        println!("evidence  {:#?}", resolved.evidence);
        if let Some(rate) = resolved.evidence.fill_rate {
            let pps = u128::from(rate.pixels_shaded) * 1_000_000_000
                / u128::from(rate.elapsed_nanos.max(1));
            println!("pixels per second  {pps}");
        }
    }
}
