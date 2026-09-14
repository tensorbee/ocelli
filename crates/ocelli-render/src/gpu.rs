//! The one place a device and a queue are held together.
//!
//! HLD section 31, on `ocelli-compute`:
//!
//! > **Shares the renderer's device.** ocelli-compute never creates a
//! > wgpu::Device; it borrows the one ocelli-render owns. Two devices cannot
//! > share textures, which would defeat the entire point.
//!
//! That is the whole contract. This module is what makes it a mechanism rather
//! than a sentence, and `ci/check-device-ownership.sh` covers the case the
//! type system cannot, which is a crate creating a device it never puts in a
//! `GpuContext` at all.

use std::sync::{Arc, Mutex, PoisonError};

use wgpu::{CommandEncoder, Device, DeviceLostReason, Queue};

use crate::caps::{Caps, compute_available, recovers_from};
use crate::probe::ResolvedAdapter;

/// Why a device could not be opened, or could not be rebuilt.
///
/// `thiserror` in a core crate is HLD section 23's first sentence, and
/// `ocelli-compute`'s `ComputeError` already sets the shape. **None of these is
/// a boundary error code.** Section 23's stable numeric codes live on
/// `ocelli_core::ErrorCode` and are what the shell switches on. This type is
/// internal to the renderer, and a device failure reaches the shell as a
/// viewport-level error state through whatever F-038 and F-039 build, never as
/// a silent blank canvas.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum DeviceError {
    /// The adapter refused to open a device.
    ///
    /// The text is wgpu's `RequestDeviceError` rendered through `Display`,
    /// which is the only access that type gives to its reason. It is
    /// diagnostic and explicitly unstable, so it is for a human and is never
    /// matched on, which is the same contract `caps::FailedAdapter::reason`
    /// already carries.
    #[error("the adapter did not open a device: {0}")]
    Refused(String),

    /// Recovery was requested on a device that has not been lost.
    ///
    /// A separate arm from [`DeviceError::Unrecoverable`] on purpose. "There
    /// is nothing to recover" and "this loss is not the kind we rebuild from"
    /// are different answers, and collapsing them would let a caller that
    /// recovers on a timer look like it was handling a loss.
    #[error("the device has not been lost, so there is nothing to recover")]
    NotLost,

    /// The device was lost for a reason this project does not rebuild from.
    ///
    /// Today that is `Destroyed` alone. See [`crate::caps::recovers_from`] for
    /// the two-row table and why a destroy is the caller's own teardown.
    #[error("a device lost by {0:?} is not rebuilt, see caps::recovers_from")]
    Unrecoverable(DeviceLostReason),
}

/// A device loss, as the callback recorded it.
///
/// HLD section 22: "**Device loss is a real state, not an error path.**" This
/// is that state's payload, and it exists so the state is OBSERVED rather than
/// inferred from a later call failing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceLoss {
    /// wgpu's reason. Exactly two variants in the pinned version.
    pub reason: DeviceLostReason,
    /// wgpu's diagnostic text. For a human, never matched on.
    pub message: String,
}

/// Whether the device behind a [`GpuContext`] is still usable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviceState {
    /// No loss has been reported.
    Live,
    /// A loss was reported, with the reason and text the callback carried.
    Lost(DeviceLoss),
}

/// What a successful [`GpuContext::recover`] produced.
///
/// It carries nothing but `caps` because there is nothing else to carry: HLD
/// section 22 says to "rebuild the device and all resources", and this
/// workspace has no texture, buffer, pipeline or bind-group type yet. F-038,
/// F-039 and F-040 extend this struct rather than change `recover`'s signature,
/// which is why it is a struct and not a bare `Caps`.
///
/// **`caps` cannot currently differ from the pre-loss `caps`, and an earlier
/// version of this comment claimed it could.** [`ResolvedAdapter::open`] copies
/// the `Caps` the tier resolved to, which is immutable on the retained
/// adapter, so a rebuild reports the same four values by construction. The
/// field is therefore a convenience and not a warning. Re-deriving limits from
/// the adapter on a rebuild would be a second place `Caps` is built, and the
/// tier must not be re-resolved at all, because HLD section 7 says it "resolves
/// once at startup". If a driver reset ever does change an adapter's limits
/// under a live session, closing that gap is a decision with its own design
/// plan rather than a field whose documentation promises something the code
/// does not do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Recovered {
    /// The capabilities the rebuilt device resolved to.
    pub caps: Caps,
}

/// The slot a device's loss callback writes into.
///
/// `Arc<Mutex<_>>` rather than a borrow, because
/// `Device::set_device_lost_callback` takes `impl Fn(..) + Send + 'static` in
/// the pinned wgpu and therefore cannot borrow the `GpuContext` it belongs to.
type LossSlot = Arc<Mutex<Option<DeviceLoss>>>;

/// Record a loss into a slot, keeping the FIRST one reported.
///
/// **A function rather than the rule written at each of its two call sites.**
/// A later report about the same dead device would otherwise overwrite the
/// reason the recovery decision is made from. [`recovers_from`] reads that
/// reason, so an `Unknown` overwritten by a `Destroyed` turns a session that
/// should rebuild into one that refuses.
///
/// **On the pinned wgpu that second report cannot arrive, and this guard is
/// defence in depth rather than a live case.** `wgpu-core` 30.0.1 stores the
/// closure as `Mutex<Option<DeviceLostClosure>>` and `resource.rs:987` reads it
/// with `.lock().take()`, so it fires at most once per device, and the browser
/// backend hangs it off a `lost` promise that also settles once. Neither is a
/// documented contract of the `wgpu` facade, and `Device::set_device_lost_callback`
/// promises nothing about multiplicity, so the rule is kept and its real reason
/// is stated: it is the invariant `GpuContext::inject_loss` needs, because the
/// test seam and the callback must apply the same rule or the seam stops
/// standing in for the thing it replaces.
///
/// It exists as its own function because its two callers are the loss callback
/// in [`GpuContext::new`], which needs a real device to reach, and
/// `GpuContext::inject_loss`, which is test-only. Writing the rule twice would
/// put a second copy in the seam that exists to test the first, and writing it
/// once inside the callback would leave it reachable only by hardware.
/// `gpu::tests::the_first_loss_reported_is_the_one_kept` drives this, in the CI
/// floor, with no adapter.
fn record_loss(slot: &Mutex<Option<DeviceLoss>>, loss: DeviceLoss) {
    let mut held = slot.lock().unwrap_or_else(PoisonError::into_inner);
    if held.is_none() {
        *held = Some(loss);
    }
}

/// The device, the queue and the capabilities they resolved to, owned together.
///
/// **There is deliberately no accessor that yields an owned `Device` or
/// `Queue`.** `device()` and `queue()` hand out shared borrows and nothing
/// else, and `crates/ocelli-compute/tests/ui/` asserts the absence as a
/// compile error.
///
/// **What that does and does not defend against, stated precisely, because
/// the obvious reading is too strong.** `wgpu::Device` is itself `Clone`,
/// measured below rather than assumed, and it is a refcounted handle, so
/// cloning one yields the SAME device. Section 31's concern is that "two
/// devices cannot share textures", and a second device only arrives from a
/// second `request_device`. That is what `ci/check-device-ownership.sh`
/// refuses, and it is the load-bearing guard.
///
/// This type is still not `Clone`, for the smaller and separate reason that
/// the device, the queue and the resolved `Caps` should have one owner. A
/// second owner is not a second device, it is a second place to look.
///
/// Nor does this type create anything. `new` takes a device and a queue that
/// already exist. Adapter enumeration and tier resolution are F-004's, in
/// `caps` and `probe`, and doing them here would be a second copy of a decision
/// this project wants exactly once. **F-037 added the loss state and the
/// rebuild**, which are about a device that already exists rather than about
/// which device to open, so they belong to the type that owns one.
#[derive(Debug)]
pub struct GpuContext {
    device: Device,
    queue: Queue,
    caps: Caps,
    /// Written by this device's loss callback and by nothing else.
    ///
    /// **One slot per device, created in `new` beside the callback that writes
    /// it.** That is what makes a loss reported after a rebuild land on the
    /// device it belongs to: the old device's callback holds an `Arc` to the
    /// OLD slot, which is dropped with the old context, so it cannot mark the
    /// new device lost.
    lost: LossSlot,
}

impl GpuContext {
    /// Take ownership of an already-created device and queue, and start
    /// watching it for loss.
    ///
    /// This crate is the only one permitted to call `request_device`, so in
    /// practice the arguments come from within `ocelli-render`, today from
    /// [`ResolvedAdapter::open`]. The constructor is public because the
    /// oracle's software-adapter path in F-X002 needs to build one too.
    ///
    /// **Registering the loss callback is this constructor's job and not the
    /// caller's.** HLD section 22 makes device loss "a real state, not an error
    /// path", and a `GpuContext` whose device nobody is watching would answer
    /// [`GpuContext::state`] with `Live` for a device that is gone, which is a
    /// worse answer than no answer. Putting the registration here means there
    /// is no window in which a context exists and is unobserved, and no second
    /// constructor that forgets.
    #[must_use]
    pub fn new(device: Device, queue: Queue, caps: Caps) -> Self {
        let lost: LossSlot = Arc::new(Mutex::new(None));
        let sink = Arc::clone(&lost);
        device.set_device_lost_callback(move |reason, message| {
            record_loss(&sink, DeviceLoss { reason, message });
        });
        Self {
            device,
            queue,
            caps,
            lost,
        }
    }

    /// Whether this device is still usable, as OBSERVED rather than inferred.
    ///
    /// HLD section 22's "real state". The answer comes from the callback wgpu
    /// invoked, not from a later call having failed, because by the time a call
    /// fails the session has already tried to render with a dead device.
    #[must_use]
    pub fn state(&self) -> DeviceState {
        let slot = self.lost.lock().unwrap_or_else(PoisonError::into_inner);
        slot.clone().map_or(DeviceState::Live, DeviceState::Lost)
    }

    /// Rebuild the device and the queue on the same adapter.
    ///
    /// HLD section 22: "Handle device_lost, rebuild the device and all
    /// resources, and restore viewport state from the shell's copy." This does
    /// the first of those three. **There are no resources and no viewports in
    /// this workspace yet**, so [`Recovered`] is what F-038, F-039 and F-040
    /// extend rather than a claim that nothing else needs rebuilding.
    ///
    /// The adapter is the caller's, retained by [`crate::probe::resolve_adapter`],
    /// so a rebuild costs no second detection pass. Re-running detection here
    /// would also be wrong: HLD section 7 says the tier "resolves once at
    /// startup", and a session that silently changed tier mid-flight is the
    /// quietly-different-answer deviation D-07 refuses.
    ///
    /// # Errors
    ///
    /// [`DeviceError::NotLost`] when the device is live, because recovering a
    /// working device would discard it. [`DeviceError::Unrecoverable`] when
    /// [`crate::caps::recovers_from`] says no, which today is a deliberate
    /// `destroy`. [`DeviceError::Refused`] when the adapter refuses, in which
    /// case **this context is left in its `Lost` state rather than half
    /// rebuilt**, so a caller that retries sees the same loss it started from.
    pub async fn recover(&mut self, adapter: &ResolvedAdapter) -> Result<Recovered, DeviceError> {
        let loss = match self.state() {
            DeviceState::Live => return Err(DeviceError::NotLost),
            DeviceState::Lost(loss) => loss,
        };
        if !recovers_from(loss.reason) {
            return Err(DeviceError::Unrecoverable(loss.reason));
        }
        // Opened BEFORE anything here is replaced, so a refusal leaves the
        // context exactly as it was rather than half rebuilt.
        //
        // **THIS IS THE ONE PLACE TWO DEVICES BRIEFLY COEXIST**, and the
        // module header's claim about the probe is about the probe. Between
        // this line and the assignment below, the old device handle and the new
        // one are both alive. That is deliberate and is the cost of the
        // guarantee above: releasing the old one first would leave a caller
        // whose rebuild then failed with no device at all.
        //
        // HLD section 31's invariant is untouched, because its concern is
        // stated as "two devices cannot share textures, which would defeat the
        // entire point", and nothing shares anything across this line. The old
        // device is lost, by the check above, so it renders nothing, owns no
        // resource this workspace has a type for, and is dropped at the
        // assignment. `ci/check-device-ownership.sh` is about who may CREATE a
        // device and is unaffected.
        let rebuilt = adapter.open().await?;
        let caps = *rebuilt.caps();
        // The whole context, so the old device, the old queue and the old loss
        // slot go together. A rebuild that kept the slot would start `Lost`.
        *self = rebuilt;
        Ok(Recovered { caps })
    }

    /// The shared device. Borrowed, never handed over.
    #[must_use]
    pub fn device(&self) -> &Device {
        &self.device
    }

    /// The shared queue. Borrowed, never handed over.
    ///
    /// Section 22 requires **one `queue.submit()` per frame** across all
    /// viewports. A borrow is what lets the render graph keep that promise,
    /// because nothing else can hold a queue to submit on its own.
    #[must_use]
    pub fn queue(&self) -> &Queue {
        &self.queue
    }

    /// The capabilities this device resolved to.
    #[must_use]
    pub fn caps(&self) -> &Caps {
        &self.caps
    }

    /// Whether a compute kernel may run on this context at all.
    ///
    /// Reads `Caps`, so a tier B or tier C context answers `false` and a
    /// kernel with no fallback reports its feature unavailable rather than
    /// quietly producing a different answer. Section 31, and deviation D-07's
    /// generalisation of it.
    ///
    /// **The decision is [`crate::caps::compute_available`] and this only
    /// forwards.**
    /// It was written out here, and `GpuContext::new` needs a real device, so
    /// the one decision in this module was reachable by no test in the CI
    /// floor: mutating its `&&` to `||` survived nine sprint-review passes.
    /// `lib.rs` already says where a decision belongs, "everything that can be
    /// WRONG about a tier is in `caps`, which needs no adapter to test", and
    /// the arithmetic being here contradicted it. A forwarder is normally a
    /// construct this repository refuses, and it is the right shape in this one
    /// case because the alternative is the decision existing twice.
    ///
    /// **The gap this paragraph used to record is CLOSED on a machine with an
    /// adapter, and still open on one without.** The decision has been tested
    /// exhaustively by `caps::tests` since it moved there. The forwarder was
    /// reachable by no test at all, because reaching it needs a `GpuContext`
    /// and `GpuContext::new` needs a real `Device`. Measured for the S03
    /// review's tenth pass: replacing the body with
    /// `!compute_available(&self.caps)` left `bin/ocelli.sh test ocelli-render`
    /// at exit 0, 63 passed and 0 failed.
    ///
    /// F-037 builds a `GpuContext` a test can construct, and
    /// `tests/device.rs::compute_availability_on_a_real_context_matches_the_decision`
    /// drives this forwarder against `compute_available` on a real device. That
    /// test is `#[ignore]`d, because deviation D-04 leaves the CI floor without
    /// an adapter, and `bin/ocelli.sh gate gpu` is what runs it, in the
    /// `--sprint` and `--all` profiles and never in CI.
    ///
    /// **So the honest statement is that the forwarder is watched by a gate
    /// that needs hardware, rather than by nothing.** On CI it remains
    /// unreached, and the human check `docs/hld/24-agent-code-standards.md`
    /// section 27.3 requires is still the last line of defence there. F-X002's
    /// software-adapter path is what would close it in CI, and it is S14.
    #[must_use]
    pub fn supports_compute(&self) -> bool {
        compute_available(&self.caps)
    }

    /// Write a loss record as if the callback had delivered it.
    ///
    /// **This exists because the recoverable arm of [`GpuContext::recover`] is
    /// otherwise reachable by nothing.** The pinned wgpu can be asked for
    /// exactly one loss, `Destroyed`, through `Device::destroy`, and that is
    /// the reason [`recovers_from`] refuses. `Unknown` is the reason this
    /// project rebuilds from and it arrives from a driver reset, a backgrounded
    /// tab or an OOM, none of which an API call produces. Without this seam,
    /// `*self = rebuilt` would be a line no test executes, which is the shape
    /// this crate has already been caught with twice.
    ///
    /// It is `pub(crate)` and `#[cfg(test)]`, so it is not a public test hook
    /// and cannot be reached from `tests/device.rs`. The recovery test that
    /// needs it lives in this module's own test submodule for that reason.
    ///
    /// **What it does NOT prove**, because a fabricated input is weaker than a
    /// real one: that a real driver reset produces `Unknown` and reaches this
    /// same slot. Nothing on one machine can prove that, and
    /// `docs/lld/gpu-ownership.md` records it as a known limit rather than as
    /// covered.
    /// It applies [`record_loss`], the same rule and the same code the callback
    /// applies, so the seam cannot diverge from the thing it stands in for.
    #[cfg(test)]
    pub(crate) fn inject_loss(&self, loss: DeviceLoss) {
        record_loss(&self.lost, loss);
    }
}

/// A command encoder borrowed for the duration of one dispatch.
///
/// This alias exists so `ocelli-compute` can name the encoder type in its
/// public signature without every caller importing `wgpu` directly. It is not
/// a wrapper and forwards nothing.
pub type SharedEncoder<'a> = &'a mut CommandEncoder;

#[cfg(test)]
mod tests {
    /// `wgpu::Device` is `Clone`, and this project's contract has to be
    /// written knowing that.
    ///
    /// The claim is executable rather than folklore, which is the same reason
    /// F-001 asserted that `Transform::inverse` on a singular transform
    /// returns non-finite values instead of leaving it in a comment. If a
    /// future wgpu makes `Device` non-`Clone`, this goes red and the reasoning
    /// in `GpuContext`'s documentation gets revisited rather than silently
    /// becoming wrong.
    ///
    /// It does NOT mean a clone is a second device. `Device` is a refcounted
    /// handle, so a clone is the same device, and section 31's "two devices
    /// cannot share textures" is about a second `request_device`, which
    /// `ci/check-device-ownership.sh` refuses.
    #[test]
    fn wgpu_device_is_a_clonable_handle() {
        fn assert_clone<T: Clone>() {}
        assert_clone::<wgpu::Device>();
        assert_clone::<wgpu::Queue>();
    }

    /// **The rebuild arm of `recover`, which nothing else reaches.**
    ///
    /// `Unknown` is the reason HLD section 22's population produces and the one
    /// [`crate::caps::recovers_from`] rebuilds from, and the pinned wgpu offers
    /// no way to ask for it. So the loss is injected through the same slot the
    /// callback writes, and what is proved is everything after the decision:
    /// the retained adapter opens a replacement, the context becomes the new
    /// device, the reported capabilities come back, and **the rebuilt context
    /// starts `Live` rather than inheriting the loss it was rebuilt from.**
    ///
    /// That last clause is the one that would otherwise rot. `recover` replaces
    /// the whole context, so the dead device's loss slot goes with it. A
    /// rebuild that kept the slot hands back something that reports `Lost` the
    /// instant it exists, and a caller driving recovery from `state()` would
    /// rebuild forever.
    ///
    /// In this module rather than in `tests/device.rs` because the injection
    /// seam is `pub(crate)`, and it is `pub(crate)` rather than public because
    /// a public way to tell a live device it is dead is not something this
    /// crate should offer.
    #[test]
    #[ignore = "needs a real GPU adapter, which the CI floor does not have (D-04)"]
    fn an_unknown_loss_is_rebuilt_and_the_new_context_is_live() {
        use crate::caps::{Tier, TierRequest};
        use crate::gpu::{DeviceLoss, DeviceState};
        use crate::probe::resolve_adapter;

        let start = std::time::Instant::now();
        let mut clock = || u64::try_from(start.elapsed().as_nanos()).unwrap_or(u64::MAX);
        let Some(adapter) = pollster::block_on(resolve_adapter(TierRequest::Auto, &mut clock))
        else {
            println!("this machine resolved tier C, so there is no device to rebuild");
            return;
        };
        assert_ne!(adapter.resolution().caps.tier, Tier::Cpu);

        // `assert!` then a refutable binding, which is this repository's shape
        // for a fallible value in a test. `unwrap`, `expect` and `panic!` are
        // all denied at the workspace, because HLD section 23 makes a panic
        // inside wasm a poisoned instance rather than an error path.
        let opened = pollster::block_on(adapter.open());
        assert!(opened.is_ok(), "an adapter resolved and no device opened");
        let Ok(mut context) = opened else { return };
        let caps_before = *context.caps();
        // A clone of the handle, kept so the rebuild can be proved to have
        // produced a DIFFERENT device. `wgpu::Device` is a refcounted handle
        // and `gpu::tests::wgpu_device_is_a_clonable_handle` asserts that, so
        // this is the same device rather than a second one.
        let device_before = context.device().clone();

        context.inject_loss(DeviceLoss {
            reason: wgpu::DeviceLostReason::Unknown,
            message: "injected, see this test's documentation".to_owned(),
        });
        assert!(
            matches!(context.state(), DeviceState::Lost(_)),
            "the injected loss did not reach the slot state() reads"
        );

        let outcome = pollster::block_on(context.recover(&adapter));
        assert!(
            outcome.is_ok(),
            "an Unknown loss was not rebuilt: {outcome:?}"
        );
        let Ok(recovered) = outcome else { return };

        // **THE DEVICE ACTUALLY CHANGED.** Without this row a `recover` that
        // kept the dead device and merely cleared the loss flag passes every
        // other assertion here: the caps match because they are copied from the
        // same `ResolvedAdapter`, the state is `Live` because the flag was
        // cleared, and the submission below succeeds because the old device was
        // never really destroyed in this test, only told it was lost. Measured
        // in the F-037 review's first pass, where that mutation was GREEN.
        assert_ne!(
            *context.device(),
            device_before,
            "recover cleared the loss flag and kept the dead device"
        );

        assert_eq!(recovered.caps, caps_before);
        assert_eq!(context.caps(), &caps_before);
        assert_eq!(
            context.state(),
            DeviceState::Live,
            "the rebuilt context inherited the loss it was rebuilt from"
        );

        // A working device, not merely a handle.
        let encoder = context
            .device()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("ocelli recovery probe"),
            });
        context.queue().submit(std::iter::once(encoder.finish()));
        assert!(context.device().poll(wgpu::PollType::Poll).is_ok());
    }

    /// **First loss wins, so a second report cannot rewrite the reason recovery
    /// decides on.**
    ///
    /// Driven through the injection seam, which applies the same rule the
    /// callback does. An `Unknown` overwritten by a `Destroyed` turns a session
    /// that should rebuild into one that refuses, and the two reports would
    /// both be about the same dead device.
    ///
    /// **It drives the production [`super::record_loss`]**, which is the same
    /// function and the same code the loss callback runs, rather than
    /// rebuilding the rule in the test body. A test that reimplements the rule
    /// it is checking asserts the arithmetic against itself and stays green
    /// when the production guard is deleted, which is the failure this crate
    /// has already recorded twice in `probe.rs`.
    ///
    /// It needs no adapter, because `record_loss` is about the slot rather than
    /// about a device, so it runs in the CI floor that deviation D-04 leaves
    /// without one. That is the whole reason the rule is a free function.
    #[test]
    fn the_first_loss_reported_is_the_one_kept() {
        use super::{DeviceLoss, record_loss};
        use std::sync::{Mutex, PoisonError};

        let slot: Mutex<Option<DeviceLoss>> = Mutex::new(None);
        record_loss(
            &slot,
            DeviceLoss {
                reason: wgpu::DeviceLostReason::Unknown,
                message: "first".to_owned(),
            },
        );
        record_loss(
            &slot,
            DeviceLoss {
                reason: wgpu::DeviceLostReason::Destroyed,
                message: "second".to_owned(),
            },
        );

        let held = slot.lock().unwrap_or_else(PoisonError::into_inner);
        assert_eq!(
            held.as_ref().map(|loss| loss.reason),
            Some(wgpu::DeviceLostReason::Unknown),
            "a later report overwrote the reason recovery decides on"
        );
        assert_eq!(
            held.as_ref().map(|loss| loss.message.as_str()),
            Some("first")
        );
    }

    /// An empty slot takes the first loss at all, which the test above assumes
    /// and does not assert on its own: a `record_loss` that never wrote
    /// anything satisfies neither, but one that wrote only when the slot was
    /// ALREADY full would satisfy the message assertion vacuously through
    /// `None`. Separated so each row fails for one reason.
    #[test]
    fn an_empty_slot_takes_the_loss_it_is_given() {
        use super::{DeviceLoss, record_loss};
        use std::sync::{Mutex, PoisonError};

        let slot: Mutex<Option<DeviceLoss>> = Mutex::new(None);
        record_loss(
            &slot,
            DeviceLoss {
                reason: wgpu::DeviceLostReason::Destroyed,
                message: "only".to_owned(),
            },
        );

        let held = slot.lock().unwrap_or_else(PoisonError::into_inner);
        assert_eq!(
            held.as_ref().map(|loss| loss.reason),
            Some(wgpu::DeviceLostReason::Destroyed)
        );
    }
}
