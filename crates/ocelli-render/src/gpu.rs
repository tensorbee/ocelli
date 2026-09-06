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

use wgpu::{CommandEncoder, Device, Queue};

use crate::caps::{Caps, compute_available};

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
/// already exist. Adapter enumeration, tier resolution and device-loss
/// recovery are F-004 and F-037, and doing them here would be a second copy of
/// a decision this project wants exactly once.
#[derive(Debug)]
pub struct GpuContext {
    device: Device,
    queue: Queue,
    caps: Caps,
}

impl GpuContext {
    /// Take ownership of an already-created device and queue.
    ///
    /// This crate is the only one permitted to call `request_device`, so in
    /// practice the arguments come from within `ocelli-render`. The
    /// constructor is public because F-037 will build the device in a sibling
    /// module and the oracle's software-adapter path in F-X002 needs to build
    /// one too.
    #[must_use]
    pub fn new(device: Device, queue: Queue, caps: Caps) -> Self {
        Self {
            device,
            queue,
            caps,
        }
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
    /// **What the move did NOT close, stated because the paragraph above
    /// otherwise reads as if the gap were shut.** The decision is now tested,
    /// exhaustively, by `caps::tests`. This forwarder is not. It is reachable
    /// by no test in the crate, because reaching it needs a `GpuContext`,
    /// `GpuContext::new` takes a real `Device` and `Queue`, and deviation D-04
    /// leaves the floor without an adapter to produce either. Measured for the
    /// S03 review's tenth pass: replacing the body with
    /// `!compute_available(&self.caps)` leaves `bin/ocelli.sh test
    /// ocelli-render` at exit 0, 63 passed and 0 failed, and this method has no
    /// other caller in the workspace.
    ///
    /// **The residue went from a decision nothing reached to a forwarding call
    /// nothing reaches, which is smaller and is not zero.** A wrong forwarder
    /// answers `true` on a tier B context and a kernel with no fallback then
    /// runs where section 31 says it must report unavailable, which is the
    /// failure this method exists to prevent. What closes it is a `GpuContext`
    /// a test can build, which is F-037's long-lived device and F-X002's
    /// software-adapter path, and both are named on `new` above as the reason
    /// that constructor is public. Until one of them lands, the only thing
    /// watching this line is the human check
    /// `docs/hld/24-agent-code-standards.md` section 27.3 requires.
    #[must_use]
    pub fn supports_compute(&self) -> bool {
        compute_available(&self.caps)
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
}
