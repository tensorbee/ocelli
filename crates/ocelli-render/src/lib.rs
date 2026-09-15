//! wgpu device, render graph, WGSL shaders, backend tiers.
//!
//! Targets: wasm32 yes, native yes. See `docs/hld/03-architecture-and-crates.md`.
//!
//! F-001 creates the crate. F-008 gives it the device-ownership contract.
//! F-004 resolves the tier, **F-037 (E6.1, S11) creates the long-lived device
//! and makes device loss an observable state**, and the render graph follows in
//! F-038.
//!
//! The device lifecycle is split the same way tier resolution is. `caps` holds
//! the two decisions, [`caps::opens_a_device`] and [`caps::recovers_from`],
//! both total matches testable with no adapter. `probe` holds the
//! `request_device` call and [`probe::ResolvedAdapter`]. `gpu` holds the device
//! once it exists, its loss state and its rebuild.
//!
//! **`voi` is the fourth module and the crate's first shader**, added by F-041
//! (E6.5, S11). It holds HLD section 18.4's uniform, [`voi::VoiParams`], and the
//! WGSL that reads it, [`voi::VOI_WGSL`]. The division section 18 draws is that
//! `ocelli-pixel` owns the LUT values and this crate owns the layout. **The
//! WGSL does evaluate the three window formulas**, because section 18.4's
//! uniform hands a shader `center`, `width` and `fn_kind` and a shader given
//! those has to evaluate something. What it does not do is make a LUT
//! DECISION: it is handed no input from which it could re-select a window,
//! recompute inversion or apply a sequence. The Rust in `voi` computes nothing
//! at all and only reads a resolved chain. The WGSL carries no entry
//! point and is composed by its consumer, which today is a test and tomorrow is
//! F-038's render graph.
//!
//! Tier resolution is split across two modules on purpose. `caps` decides and
//! touches no GPU, `probe` touches the GPU and decides nothing. Everything
//! that can be WRONG about a tier is in `caps`, which needs no adapter to
//! test.
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
pub mod probe;
pub mod voi;

pub use caps::{
    AdapterFacts, Caps, DecidedBy, FailedAdapter, FillRate, FillRateBands, OverrideOutcome,
    ProbeOutcome, Resolution, SimdSupport, SoftwareVerdict, Tier, TierEvidence, TierRequest,
    TierSignals, candidate_order, classify, compute_available, opens_a_device, recovers_from,
};
pub use gpu::{DeviceError, DeviceLoss, DeviceState, GpuContext, Recovered, SharedEncoder};
pub use probe::{Edge, Passes, ResolvedAdapter, resolve, resolve_adapter};
pub use voi::{VOI_WGSL, VoiParams, VoiParamsError};

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
