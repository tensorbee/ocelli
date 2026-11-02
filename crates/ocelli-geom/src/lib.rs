//! Hit-testing, projection, measurement mathematics.
//!
//! Targets: wasm32 yes, native yes. See `docs/hld/03-architecture-and-crates.md`.
//!
//! F-001 creates the crate. F-070 fills [`measure`]. F-073 adds length,
//! height, Cobb and bidirectional beside those functions. The angle tool
//! stays [`angle_at`]. The formulas are the local rules in
//! `.claude/plans/F-070-design.md` and `.claude/plans/F-073-design.md`. The
//! HLD names the crate and does not give them. Camera transforms stay in
//! `ocelli-viewport`.

#![cfg_attr(not(test), no_std)]

mod measure;

pub use measure::{
    Bidirectional, GeomError, SegmentProjection, angle_at, bidirectional, cobb, distance, height,
    hit_segment, length, polyline_length, project_onto_plane, project_onto_segment,
};

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
