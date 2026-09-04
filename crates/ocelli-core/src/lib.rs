//! Types, coordinate spaces, geometry primitives, error model. No I/O.
//!
//! Targets: wasm32 yes, native yes. See `docs/hld/03-architecture-and-crates.md`.
//!
//! F-001 adds the two modules HLD section 28 puts first, [`space`] and
//! [`value`]. Everything downstream depends on them, and both are re-exported
//! at the crate root so a caller writes `ocelli_core::Pt` rather than
//! `ocelli_core::space::Pt`.

#![cfg_attr(not(test), no_std)]

pub mod space;
pub mod value;

pub use space::{Canvas, Index, Pt, Transform, World};
pub use value::{Display, Modality, Stored};

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
