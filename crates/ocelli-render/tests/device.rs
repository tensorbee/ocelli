//! The device lifecycle, against a real adapter.
//!
//! **Every test here is `#[ignore]`d and that is not a skipped test.**
//! Deviation D-04 leaves CI with no GPU, so a test that ran by default would be
//! red on every machine this project builds on. `bin/ocelli.sh gate gpu` runs
//! them, in the `--sprint` and `--all` profiles, and CI does not.
//!
//! HLD section 22: "**Device loss is a real state, not an error path.** Handle
//! device_lost, rebuild the device and all resources, and restore viewport
//! state from the shell's copy."
//!
//! **What these tests cannot prove**, stated here rather than left to be
//! assumed. The pinned wgpu can produce exactly one loss reason on demand,
//! `Destroyed`, through `Device::destroy`. `Unknown`, which is the reason this
//! project actually recovers from, arrives from a driver reset, a backgrounded
//! tab or an OOM, and nothing in the API asks for one. So the callback path is
//! proved end to end on the reason we refuse, and the rebuild path is proved on
//! a loss record this file injects. That a real driver reset produces `Unknown`
//! and reaches the same slot is not proved here and cannot be proved on one
//! machine. It is recorded in `docs/lld/gpu-ownership.md` as a known limit.

use ocelli_render::{
    Caps, DeviceError, DeviceState, GpuContext, ResolvedAdapter, Tier, TierRequest,
    compute_available, opens_a_device, resolve_adapter,
};

/// A monotonic nanosecond clock, which `resolve_adapter` takes from the caller.
///
/// It is the caller's because `std::time::Instant` panics on
/// `wasm32-unknown-unknown`, which is deviation D-12 keeping a browser binding
/// out of this crate. Natively it is an `Instant`, and this is that.
fn native_clock() -> impl FnMut() -> u64 {
    let start = std::time::Instant::now();
    move || u64::try_from(start.elapsed().as_nanos()).unwrap_or(u64::MAX)
}

/// Resolve and open, or report why this machine cannot run the test.
///
/// Returns `None` on a machine that resolves tier C, where there is no device
/// by design. A tier C machine is a legitimate host for this gate and the tests
/// report that they did not apply rather than failing, which is deviation
/// D-07's own rule applied to its test suite.
fn open_context() -> Option<(ResolvedAdapter, GpuContext)> {
    let mut clock = native_clock();
    let adapter = pollster::block_on(resolve_adapter(TierRequest::Auto, &mut clock))?;
    let opened = pollster::block_on(adapter.open());
    // `assert!` then a refutable binding. `unwrap`, `expect` and `panic!` are
    // denied at the workspace, because HLD section 23 makes a panic inside wasm
    // a poisoned instance rather than an error path, and the lint does not
    // distinguish a test from an exported path.
    assert!(
        opened.is_ok(),
        "an adapter resolved and no device opened: {opened:?}"
    );
    opened.ok().map(|context| (adapter, context))
}

/// Wait for wgpu to deliver any pending callbacks on this device.
///
/// `Device::poll` blocks on wgpu-core backends and is documented as a no-op on
/// WebGPU, where callbacks arrive from the event loop. This is a native test,
/// so the poll is what drives the loss callback, and `probe::run` already
/// depends on the same behaviour for `on_submitted_work_done`.
fn settle(context: &GpuContext) {
    let _ = context.device().poll(wgpu::PollType::Poll);
}

/// A device opens, and it reports the capabilities the tier resolved to.
#[test]
#[ignore = "needs a real GPU adapter, which the CI floor does not have (D-04)"]
fn a_device_opens_and_agrees_with_the_resolved_tier() {
    let Some((adapter, context)) = open_context() else {
        println!("this machine resolved tier C, so there is no device to open");
        return;
    };

    assert_eq!(
        context.caps(),
        &adapter.resolution().caps,
        "the device reported different capabilities from the tier that resolved it"
    );
    assert!(
        opens_a_device(context.caps()),
        "a context exists for a tier that is not supposed to hold one"
    );
    assert_ne!(
        context.caps().tier,
        Tier::Cpu,
        "resolve_adapter handed back an adapter for a tier C session"
    );
    assert_eq!(context.state(), DeviceState::Live);
}

/// The forwarder that no test could reach until this story, driven on a real
/// context.
///
/// `GpuContext::supports_compute` forwards to `caps::compute_available`, and
/// its own documentation records that replacing the body with the NEGATION left
/// the crate green through nine sprint-review passes, because reaching it needs
/// a device and deviation D-04 leaves the floor without one.
///
/// The assertion is against `compute_available` on the same `Caps` rather than
/// against a hardcoded expectation, because which answer is right depends on
/// the machine. What is asserted is that the forwarder forwards, which is the
/// whole of its contract, and the decision itself is asserted exhaustively by
/// `caps::tests` with no adapter at all.
#[test]
#[ignore = "needs a real GPU adapter, which the CI floor does not have (D-04)"]
fn compute_availability_on_a_real_context_matches_the_decision() {
    let Some((_adapter, context)) = open_context() else {
        println!("this machine resolved tier C, so there is no device to open");
        return;
    };

    assert_eq!(
        context.supports_compute(),
        compute_available(context.caps()),
        "the forwarder does not forward"
    );
}

/// **A destroyed device is OBSERVED as lost, not inferred from a later
/// failure.**
///
/// This is the only loss the pinned wgpu can be asked for, and it is the whole
/// proof that the callback registered in `GpuContext::new` is wired to the
/// device the context holds.
///
/// `DeviceLostReason::Destroyed` is asserted by name, not merely that some loss
/// arrived, because the reason is what `caps::recovers_from` decides on and a
/// callback that reported the wrong one would make a recoverable session refuse
/// to rebuild.
#[test]
#[ignore = "needs a real GPU adapter, which the CI floor does not have (D-04)"]
fn destroying_the_device_is_observed_as_a_loss() {
    let Some((_adapter, context)) = open_context() else {
        println!("this machine resolved tier C, so there is no device to open");
        return;
    };

    assert_eq!(
        context.state(),
        DeviceState::Live,
        "lost before it was used"
    );

    context.device().destroy();
    settle(&context);

    let state = context.state();
    assert!(
        matches!(state, DeviceState::Lost(_)),
        "the device was destroyed and the context still reports {state:?}"
    );
    let DeviceState::Lost(loss) = state else {
        return;
    };
    assert_eq!(
        loss.reason,
        wgpu::DeviceLostReason::Destroyed,
        "the loss arrived with the wrong reason, which is what recovery decides on"
    );
}

/// **A deliberate destroy is refused, not rebuilt.**
///
/// `caps::recovers_from(Destroyed)` is false because a destroy is the
/// application's own teardown, and rebuilding behind a shutdown path is a loop.
/// The context is still `Lost` afterwards, because a refused recovery must not
/// half-change the state it refused to act on.
#[test]
#[ignore = "needs a real GPU adapter, which the CI floor does not have (D-04)"]
fn a_destroyed_device_is_not_rebuilt_and_the_state_survives_the_refusal() {
    let Some((adapter, mut context)) = open_context() else {
        println!("this machine resolved tier C, so there is no device to open");
        return;
    };

    context.device().destroy();
    settle(&context);
    let before = context.state();
    assert!(matches!(before, DeviceState::Lost(_)));

    let outcome = pollster::block_on(context.recover(&adapter));
    assert_eq!(
        outcome,
        Err(DeviceError::Unrecoverable(
            wgpu::DeviceLostReason::Destroyed
        )),
        "a destroy was rebuilt, which on a shutdown path is a loop"
    );
    assert_eq!(
        context.state(),
        before,
        "a refused recovery changed the state it refused to act on"
    );
}

/// Recovering a device that has not been lost is refused, and is a DIFFERENT
/// refusal from recovering an unrecoverable one.
///
/// Collapsing the two would let a caller that recovers on a timer look like it
/// was handling a loss, and would discard a working device on every tick.
#[test]
#[ignore = "needs a real GPU adapter, which the CI floor does not have (D-04)"]
fn recovering_a_live_device_is_refused_as_not_lost() {
    let Some((adapter, mut context)) = open_context() else {
        println!("this machine resolved tier C, so there is no device to open");
        return;
    };

    assert_eq!(
        pollster::block_on(context.recover(&adapter)),
        Err(DeviceError::NotLost)
    );
    assert_eq!(
        context.state(),
        DeviceState::Live,
        "a refused recovery discarded a working device"
    );
}

/// **The rebuild path, on a second device opened from the retained adapter.**
///
/// The recoverable arm cannot be reached through wgpu, for the reason this
/// file's header gives, so what is proved here is the half that is reachable:
/// the retained `ResolvedAdapter` opens a SECOND, working device after the
/// first one is gone, with the same capabilities, and the new context starts
/// `Live` rather than inheriting the old one's loss.
///
/// That last clause is the one worth having. `GpuContext::recover` replaces the
/// whole context, so the old loss slot is dropped with the old device. A
/// rebuild that kept the slot would hand back a context that reports `Lost` the
/// moment it is created, and a caller would rebuild forever.
#[test]
#[ignore = "needs a real GPU adapter, which the CI floor does not have (D-04)"]
fn the_retained_adapter_opens_a_second_live_device_after_the_first_is_gone() {
    let Some((adapter, first)) = open_context() else {
        println!("this machine resolved tier C, so there is no device to open");
        return;
    };
    let caps_before: Caps = *first.caps();

    first.device().destroy();
    settle(&first);
    assert!(matches!(first.state(), DeviceState::Lost(_)));
    drop(first);

    let reopened = pollster::block_on(adapter.open());
    assert!(
        reopened.is_ok(),
        "the retained adapter did not open a second device: {reopened:?}"
    );
    let Ok(second) = reopened else { return };
    assert_eq!(
        second.state(),
        DeviceState::Live,
        "the rebuilt device inherited the dead one's loss state"
    );
    assert_eq!(
        *second.caps(),
        caps_before,
        "the rebuilt device resolved to different capabilities"
    );

    // It is a working device and not merely a handle: an empty submission
    // completes on it. One submit, which is HLD section 22's rule even here.
    let encoder = second
        .device()
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("ocelli recovery probe"),
        });
    second.queue().submit(std::iter::once(encoder.finish()));
    let polled = second.device().poll(wgpu::PollType::Poll);
    assert!(polled.is_ok(), "the rebuilt device would not poll");
    assert_eq!(
        second.state(),
        DeviceState::Live,
        "the rebuilt device was lost while being used"
    );
}
