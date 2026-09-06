//! The half of tier resolution that touches a GPU.
//!
//! `caps.rs` holds the decision and no I/O. This module holds the wgpu calls
//! and the fill-rate workload, and the split is the point: everything that can
//! be wrong is in the other file, and the other file needs no adapter to test.
//!
//! **The probe device is transient.** It is created, measured on and dropped
//! before [`resolve`] returns. It never becomes a [`crate::GpuContext`], so
//! HLD section 31's one-device invariant is untouched: there is never a moment
//! when two devices exist. The call sits inside `ocelli-render`, which is the
//! only crate permitted to make one, and `ci/check-device-ownership.sh` still
//! passes unchanged.
//!
//! **Nothing is read back.** The workload shades an offscreen texture and
//! never maps it. Decision D3 says pixels never cross the boundary, and a
//! readback would put them on a path they have no business being on. It would
//! also measure transfer rather than shading.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::caps::{
    AdapterFacts, FillRate, FillRateBands, Resolution, SimdSupport, Tier, TierRequest, TierSignals,
    choose_candidate, classify,
};

/// The format the workload renders into. Mandatorily renderable on every
/// backend, so the measurement is the same shape everywhere.
pub(crate) const TARGET_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

/// The ALU steps per fragment in `fill_rate.wgsl`. Recorded here so
/// `ci/tier-thresholds.json` can name the workload its figures were taken
/// with, because a fill rate without its workload is not a measurement.
pub const FILL_RATE_ALU_STEPS: u32 = 64;

/// The calibration pass: a small target, one pass.
const CALIBRATION_EDGE: u32 = 256;
const CALIBRATION_PASSES: u32 = 1;

/// The full pass, 256 times the calibration's fragment count.
const FULL_EDGE: u32 = 1024;
const FULL_PASSES: u32 = 16;

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

/// The three runs [`measure`] issues, in order, as `(edge, passes)` pairs: the
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
/// stops the plan from being a statement [`measure`] does not follow.
pub(crate) const RUN_PLAN: [(u32, u32); 3] = [
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
    // An override of tier C short-circuits everything: no instance, no
    // adapter, no device, no benchmark. That is the estate D-07 names getting
    // its startup cost back.
    if request.requested() == Some(Tier::Cpu) {
        return classify(&TierSignals::unprobed(), request);
    }

    // `new_instance_with_webgpu_detection` rather than `Instance::new`,
    // because `navigator.gpu` can be defined on a browser that cannot actually
    // produce a WebGPU adapter, and that population is exactly the one D-07
    // cares about. On native it forwards to `Instance::new` unchanged.
    let instance = wgpu::util::new_instance_with_webgpu_detection(
        wgpu::InstanceDescriptor::new_without_display_handle(),
    )
    .await;

    let adapters = enumerate(&instance).await;
    let facts: Vec<AdapterFacts> = adapters.iter().map(facts_of).collect();
    let chosen = choose_candidate(&facts);
    let (device_created, fill_rate) = measure_chosen(&adapters, chosen, clock).await;

    let signals = TierSignals {
        adapters: facts,
        fill_rate,
        device_created,
        simd: SimdSupport::build_target(),
        bands: FillRateBands::RECORDED,
    };
    classify(&signals, request)
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

/// Create a device on the chosen adapter and measure on it.
///
/// Returns whether a device was created, and the measurement if one could be
/// taken. The device and queue are dropped at the end of this function, before
/// the caller sees the answer.
///
/// **Exactly one adapter is tried, and no other is attempted after it fails.**
/// `choose_candidate` returns the single best candidate and this function asks
/// that one for a device. Natively `enumerate_adapters(Backends::all())`
/// commonly returns several, so on a host with a broken Vulkan ICD beside a
/// working GL driver the best candidate is the Vulkan A-candidate,
/// `request_device` fails, and the session resolves tier C while a tier-B path
/// exists and was never attempted. Tier C renders nothing until F-X001 to
/// F-X004, so the outcome is "renders nothing" rather than "renders on tier
/// B". Trying the next adapter is a behaviour change with its own ranking and
/// evidence questions, so it belongs to a story rather than to this comment.
async fn measure_chosen(
    adapters: &[wgpu::Adapter],
    chosen: Option<usize>,
    clock: &mut dyn FnMut() -> u64,
) -> (bool, Option<FillRate>) {
    let Some(index) = chosen else {
        return (false, None);
    };
    let Some(adapter) = adapters.get(index) else {
        return (false, None);
    };
    let descriptor = wgpu::DeviceDescriptor {
        label: Some("ocelli tier probe"),
        required_features: wgpu::Features::empty(),
        // The adapter's OWN limits, never `Limits::default()`. A downlevel GL
        // adapter does not meet the WebGPU defaults, which is the whole reason
        // tier B exists.
        //
        // **The pinned wgpu documents two different answers to asking for more
        // than the adapter has, and this code depends on the second.** The doc
        // comment on `Adapter::request_device` (`src/api/adapter.rs:49` to
        // `:56` in 30.0.1) lists "Limits requested exceed the values provided
        // by the adapter" under `# Panics`, while the same function's
        // signature returns `Result<(Device, Queue), RequestDeviceError>` and
        // the wgpu-core backend converts the core error into `Err` rather than
        // panicking (`src/backend/wgpu_core.rs:943` to `:948`, over
        // `wgpu_core::instance::RequestDeviceError::LimitsExceeded` raised at
        // `wgpu-core-30.0.1/src/instance.rs:941`). The browser backend maps a
        // rejected promise the same way. So the `Err` arm below is reachable
        // and the `NoDevice` path is not dead. Asking for the adapter's own
        // limits means neither answer is exercised here.
        //
        // `RequestDeviceError` is an opaque struct over a private
        // `RequestDeviceErrorKind` (`src/api/device.rs:790` and `:805`), and
        // `wgpu` does not re-export `wgpu_core`, so the reason is reachable
        // from here only through `Display`.
        required_limits: adapter.limits(),
        ..Default::default()
    };
    let Ok((device, queue)) = adapter.request_device(&descriptor).await else {
        return (false, None);
    };
    (true, measure(&device, &queue, clock))
}

/// The fragments one timed run shades: every texel of an `edge` by `edge`
/// target, once per pass.
///
/// **The numerator of the fill rate, extracted so the floor can assert it.**
/// It was an expression inside [`run`], and the only test over it recomputed
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
pub(crate) fn fragments(edge: u32, passes: u32) -> u64 {
    u64::from(edge) * u64::from(edge) * u64::from(passes)
}

/// Whether the full pass is affordable, given what the calibration cost.
///
/// [`CALIBRATION_BUDGET_NANOS`] is a ceiling ON the calibration, not a figure
/// the calibration has to beat: the rule is "if the calibration took LONGER
/// than this", and a calibration that took exactly the budget did not take
/// longer than it, so the full pass still runs. Extracted from [`measure`] so
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
/// This half builds the pipeline and hands [`measure_with`] a way to issue one
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
/// `issue` is [`run`] in the resolver and a recording stand-in in the tests,
/// which deviation D-04 leaves without an adapter. It is a `&mut dyn FnMut` for
/// the same reason `clock` is one on [`resolve`]: the alternative is a generic
/// parameter with one instantiation, and `AGENTS.md` refuses that shape.
///
/// **What the injection does NOT close, stated because the shape of this
/// function otherwise reads as if it closed everything.** `edge` and `passes`
/// are both `u32` and adjacent, and the closure in [`measure`] forwards them
/// positionally. Transposing them there,
/// `run(device, queue, &pipeline, passes, edge, clock)`, compiles and passes
/// the whole suite, because every test here supplies its own `issue` and never
/// reaches that call. The calibration then shades a 1 by 1 target 256 times
/// and the fragment count is unchanged, so [`fragments`] agrees with itself
/// and only a real adapter's timing would differ.
///
/// **That hole is residue rather than something the injection created.** The
/// same transposition existed at each of the three call sites this function
/// replaced, and no test reached those either, so the count of unreachable
/// sites went from three to one. Closing the last one needs a type that makes
/// the two arguments non-interchangeable, which is `AGENTS.md`'s "reducing
/// cases is good even when it adds types" and is F-037's to argue when it
/// builds the long-lived device. Until then the only thing watching it is the
/// human check `docs/hld/24-agent-code-standards.md` section 27.3 requires.
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
fn measure_with(issue: &mut dyn FnMut(u32, u32) -> Option<FillRate>) -> Option<FillRate> {
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
    edge: u32,
    passes: u32,
    clock: &mut dyn FnMut() -> u64,
) -> Option<FillRate> {
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("ocelli fill rate target"),
        size: wgpu::Extent3d {
            width: edge,
            height: edge,
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
    for _ in 0..passes {
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
        CALIBRATION_BUDGET_NANOS, CALIBRATION_EDGE, CALIBRATION_PASSES, FULL_EDGE, FULL_PASSES,
        RUN_PLAN, WORKLOAD_WGSL, fragments, full_pass_is_affordable, measure_with,
    };
    use crate::caps::FillRate;

    /// Issue [`RUN_PLAN`] against a stand-in for [`super::run`] that records
    /// what it was asked for and reports `elapsed_nanos` for every run.
    ///
    /// The fragment count it hands back is the production [`fragments`] of the
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
        assert_eq!(fragments(2, 3), 12);
        // Sixteen texels, sixteen passes, 256 fragments.
        assert_eq!(fragments(4, 16), 256);
        // One texel, two passes, and a pass is still a pass.
        assert_eq!(fragments(1, 2), 2);
        // The smallest case there is.
        assert_eq!(fragments(1, 1), 1);
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
    /// Both figures come from the production [`fragments`], so the ratio is a
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
