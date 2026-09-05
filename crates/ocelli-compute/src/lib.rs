//! WGSL compute kernels. Shares the renderer's device, never creates one.
//!
//! Targets: wasm32 yes, native yes. See `docs/hld/03-architecture-and-crates.md`.
//!
//! F-001 creates the crate. F-008 gives it the borrow side of the
//! device-sharing contract. F-125 (E31.1) fills it with kernels.
//!
//! HLD section 31 is the whole specification for this crate, and its first
//! bullet is the one this file exists to enforce:
//!
//! > **Shares the renderer's device.** ocelli-compute never creates a
//! > wgpu::Device; it borrows the one ocelli-render owns. Two devices cannot
//! > share textures, which would defeat the entire point.
//!
//! **This crate is not `no_std`**, unlike the other core crates, because wgpu
//! needs `std`. Deviation D-10.

use ocelli_render::{Caps, GpuContext, Tier};
use wgpu::CommandEncoder;

/// What can go wrong in a dispatch.
///
/// Two variants, and both exist today rather than being anticipated. Section
/// 31 requires a kernel that cannot run to mark its feature **unavailable**,
/// and deviation D-07 generalises that: a feature that cannot run on the
/// resolved tier reports unavailable and never silently produces a different
/// answer. `Unavailable` is how that is said in a return type.
#[derive(Debug, thiserror::Error)]
pub enum ComputeError {
    /// The resolved tier cannot run this kernel and it declared no fallback.
    ///
    /// This is not an error path in the ordinary sense. It is the honest
    /// answer, and the caller's job is to report the feature unavailable
    /// rather than to substitute something that looks similar.
    #[error(
        "kernel requires tier {required:?}, session resolved tier {resolved:?}, and it declares no fallback"
    )]
    Unavailable {
        /// The tier the kernel asked for.
        required: Tier,
        /// The tier the session actually has.
        resolved: Tier,
    },

    /// The kernel asked for a workgroup the device cannot dispatch.
    ///
    /// Section 31: "Workgroup sizes come from `Caps`, never hardcoded. A
    /// hardcoded 256 is a portability bug waiting for a device that reports
    /// less."
    #[error("workgroup {requested:?} exceeds what this device allows")]
    Workgroup {
        /// What the kernel asked for.
        requested: [u32; 3],
    },
}

/// Everything a kernel is given for one dispatch, and nothing more.
///
/// **The lifetime is the mechanism.** A `ComputeCtx` borrows the
/// [`GpuContext`] rather than owning or cloning a device, so a kernel cannot
/// retain one beyond the dispatch it was handed. That is section 31's rule
/// expressed as something the borrow checker enforces rather than something a
/// reviewer has to notice.
///
/// The encoder is borrowed for the same reason and one more. Section 22
/// requires **one `queue.submit()` per frame** across all viewports, and a
/// kernel that owned its encoder would submit its own work.
pub struct ComputeCtx<'a> {
    gpu: &'a GpuContext,
    encoder: &'a mut CommandEncoder,
}

impl<'a> ComputeCtx<'a> {
    /// Borrow a context and an encoder for one dispatch.
    #[must_use]
    pub fn new(gpu: &'a GpuContext, encoder: &'a mut CommandEncoder) -> Self {
        Self { gpu, encoder }
    }

    /// The shared context. Borrowed from a borrow, so still not ownable.
    #[must_use]
    pub fn gpu(&self) -> &GpuContext {
        self.gpu
    }

    /// The capabilities a kernel sizes its workgroup from.
    #[must_use]
    pub fn caps(&self) -> &Caps {
        self.gpu.caps()
    }

    /// The encoder this dispatch records into.
    pub fn encoder(&mut self) -> &mut CommandEncoder {
        self.encoder
    }
}

/// HLD section 31's trait, with its signature as the specification gives it.
///
/// ```text
/// pub trait Kernel {
///     fn tier(&self) -> Tier; // A = WebGPU only, B = has fallback
///     fn workgroup(&self, caps: &Caps) -> [u32; 3];
///     fn dispatch(&self, ctx: &mut ComputeCtx) -> Result<(), ComputeError>;
/// }
/// ```
///
/// **This trait has no implementers today, and `AGENTS.md` forbids that
/// shape.** The rule exists to stop invented abstractions, and this one is
/// prescribed rather than invented: HLD Part II opens by saying that where the
/// specification gives a signature, that signature is the intended
/// implementation. The collision was raised in the design plan and decided in
/// the sprint's design round rather than resolved quietly here. F-125 (E31.1)
/// supplies the kernels.
pub trait Kernel {
    /// The tier this kernel needs. Tier A means WebGPU only.
    fn tier(&self) -> Tier;

    /// The workgroup size, derived from `Caps` and never hardcoded.
    fn workgroup(&self, caps: &Caps) -> [u32; 3];

    /// Record this kernel's work into the borrowed encoder.
    fn dispatch(&self, ctx: &mut ComputeCtx) -> Result<(), ComputeError>;
}

/// The crate's own name. The scaffold test asserts it matches Cargo's, which
/// is the one mistake a copy-pasted crate skeleton actually makes.
pub const CRATE_NAME: &str = env!("CARGO_PKG_NAME");

#[cfg(test)]
mod tests {
    use super::ComputeError;
    use ocelli_render::Tier;

    #[test]
    fn scaffold_declares_its_own_name() {
        assert_eq!(super::CRATE_NAME, env!("CARGO_PKG_NAME"));
        assert!(super::CRATE_NAME.starts_with("ocelli"));
    }

    /// `Unavailable` names both tiers, because "unavailable" without the two
    /// tiers is a message nobody can act on.
    #[test]
    fn unavailable_names_the_required_and_resolved_tiers() {
        let error = ComputeError::Unavailable {
            required: Tier::A,
            resolved: Tier::Cpu,
        };
        let text = error.to_string();
        assert!(text.contains("tier A"), "{text}");
        assert!(text.contains("tier Cpu"), "{text}");
        assert!(text.contains("no fallback"), "{text}");
    }
}
