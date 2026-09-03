//! Desktop and server entry points. Phase 2 and 3, stubbed now.
//!
//! Targets: wasm32 no, native yes. See `docs/hld/03-architecture-and-crates.md`.
//!
//! This crate must never be reachable from a wasm build. It exists now so the
//! four extension points of `docs/hld/10-extension-points.md` stay cheap: a
//! `SeriesSource` implementation over DIMSE, a render-target trait for
//! offscreen output, a codec registry that can link C codecs the browser
//! cannot, and calibrated display presentation the browser implements as a
//! no-op.
//!
//! Scaffold only. F-001 creates the crate, F-007 proves it builds native.

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
