//! The only crate that may import wasm-bindgen. Boundary, commands, event ring.
//!
//! Targets: wasm32 yes, native no. See `docs/hld/03-architecture-and-crates.md`.
//!
//! Three channels cross this boundary and nothing else, per
//! `docs/hld/04-boundary-and-data-path.md`:
//!
//! - Control, typed commands downward, one call per user intent and never one
//!   per pointer move.
//! - Bulk, raw bytes into linear memory downward. Never cache a view across a
//!   call that might allocate.
//! - Events, a fixed-stride ring buffer upward, drained once per frame.
//!
//! `src/ring.rs` is one of the two files in the repository permitted to
//! contain `unsafe` (HLD section 27.2 R5). The other is
//! `ocelli-core/src/cast.rs`. `scripts/unsafe_allowlist_check.py` enforces it.
//!
//! Scaffold only. F-001 creates the crate, F-096 builds the boundary.

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
