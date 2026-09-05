//! wgpu device, render graph, WGSL shaders, backend tiers.
//!
//! Targets: wasm32 yes, native yes. See `docs/hld/03-architecture-and-crates.md`.
//!
//! F-001 creates the crate. F-008 gives it the device-ownership contract.
//! F-004 resolves the tier, F-039 creates the device, and the render graph
//! follows.
//!
//! **This crate is the only one permitted to create a `wgpu::Device`.** HLD
//! section 31: "ocelli-compute never creates a wgpu::Device; it borrows the
//! one ocelli-render owns. Two devices cannot share textures, which would
//! defeat the entire point." `ci/check-device-ownership.sh` asserts it.
//!
//! **This crate is not `no_std`**, unlike the other core crates, because wgpu
//! needs `std`. That is part of deviation D-10 and it is recorded there rather
//! than left as an unexplained absence.

pub mod caps;
pub mod gpu;

pub use caps::{Caps, Tier};
pub use gpu::{GpuContext, SharedEncoder};

/// The crate's own name. The scaffold test asserts it matches Cargo's, which
/// is the one mistake a copy-pasted crate skeleton actually makes.
pub const CRATE_NAME: &str = env!("CARGO_PKG_NAME");

#[cfg(test)]
mod tests {
    #[test]
    fn scaffold_declares_its_own_name() {
        assert_eq!(super::CRATE_NAME, env!("CARGO_PKG_NAME"));
        assert!(super::CRATE_NAME.starts_with("ocelli"));
    }
}
