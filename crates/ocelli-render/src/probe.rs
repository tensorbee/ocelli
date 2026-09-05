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
const TARGET_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

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

const WORKLOAD_WGSL: &str = include_str!("fill_rate.wgsl");

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
        // The adapter's OWN limits, never `Limits::default()`. Requesting
        // limits an adapter does not provide fails the request:
        // `Adapter::request_device` returns `Err(wgpu::RequestDeviceError)`,
        // which in the pinned 30.0.1 is an opaque struct over a private
        // `RequestDeviceErrorKind` (`src/api/device.rs:790` and `:805`). The
        // underlying `LimitsExceeded` belongs to `wgpu_core`, which `wgpu`
        // does not re-export, so the reason is reachable from here only
        // through `Display`. A downlevel GL
        // adapter does not meet the WebGPU defaults, which is the whole reason
        // tier B exists.
        required_limits: adapter.limits(),
        ..Default::default()
    };
    let Ok((device, queue)) = adapter.request_device(&descriptor).await else {
        return (false, None);
    };
    (true, measure(&device, &queue, clock))
}

/// The two-stage measurement.
///
/// A small calibration pass first, so a slow rasteriser does not hang startup.
/// The full pass runs only if the calibration was fast enough to make it
/// affordable, and if it was not, the calibration figure is itself the answer.
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

    // A warm-up whose figure is thrown away. The FIRST submission on a fresh
    // device pays for lazy pipeline compilation, driver initialisation and
    // command-buffer setup, and timing it measures startup latency rather than
    // fill rate. Measured on the machine in `ci/tier-thresholds.json`, twenty
    // release runs of this exact pair in fresh processes: the first 65,536
    // fragments took 5.4 to 9.3 ms, median 6.1, and the same work immediately
    // afterwards took 0.32 to 0.70 ms, median 0.36. The ratio ranged from 8.8
    // to 20.7 and is one machine's noise rather than a bound.
    //
    // The only figure this code depends on is the low end. Without the
    // discard, the calibration measures that first-run cost as the machine's
    // fill rate, and the first submission was over
    // `CALIBRATION_BUDGET_NANOS` on all twenty runs, so the full pass never
    // runs and the startup latency is the figure that gets recorded.
    let _warm_up = run(
        device,
        queue,
        &pipeline,
        CALIBRATION_EDGE,
        CALIBRATION_PASSES,
        clock,
    );

    let calibration = run(
        device,
        queue,
        &pipeline,
        CALIBRATION_EDGE,
        CALIBRATION_PASSES,
        clock,
    )?;
    if calibration.elapsed_nanos > CALIBRATION_BUDGET_NANOS {
        return Some(calibration);
    }
    run(device, queue, &pipeline, FULL_EDGE, FULL_PASSES, clock).or(Some(calibration))
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
        // estimate. `u64::from` throughout, so there is no cast to review.
        pixels_shaded: u64::from(edge) * u64::from(edge) * u64::from(passes),
        elapsed_nanos,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        CALIBRATION_BUDGET_NANOS, CALIBRATION_EDGE, CALIBRATION_PASSES, FULL_EDGE, FULL_PASSES,
        WORKLOAD_WGSL,
    };

    /// The full pass has to be enough larger than the calibration that a real
    /// adapter's figure is not dominated by submission overhead, and the
    /// calibration has to be small enough that a rasteriser reaches the budget
    /// rather than hanging startup.
    #[test]
    fn the_full_pass_is_much_larger_than_the_calibration() {
        let calibration = u64::from(CALIBRATION_EDGE)
            * u64::from(CALIBRATION_EDGE)
            * u64::from(CALIBRATION_PASSES);
        let full = u64::from(FULL_EDGE) * u64::from(FULL_EDGE) * u64::from(FULL_PASSES);
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
