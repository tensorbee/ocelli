//! Differential harness against cornerstone3D 5.8.2, deviation D-11.
//!
//! Nothing else in the port should start before this works
//! (`docs/hld/25-first-ten-files.md`, entry 4).
//!
//! The REFERENCE half is built, and it is not Rust. `../run.mjs` and the rest
//! of `tools/oracle` render every corpus row through the pinned cornerstone3D
//! under headless Chromium and write reference pixels plus a metadata sidecar.
//! See `docs/lld/oracle.md`. This crate is the OCELLI half of the same
//! harness, which F-011 onwards builds.
//!
//! The tolerance policy is written down once and held, in
//! `docs/hld/22-testing-and-tolerance.md` section 25.1. A tolerance change is
//! a pull request with a rationale, reviewed like code. Tuning tolerance per
//! failure is how a suite stops meaning anything.
//!
//! Scaffold only. F-009 through F-015 build it.

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
