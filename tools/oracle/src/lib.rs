//! Differential harness against cornerstone3D 5.8.2, deviation D-11.
//!
//! Nothing else in the port should start before this works
//! (`docs/hld/25-first-ten-files.md`, entry 4).
//!
//! The REFERENCE half is built, and it is not Rust. `../run.mjs` and the rest
//! of `tools/oracle` render every corpus row through the pinned cornerstone3D
//! under headless Chromium and write reference pixels plus a metadata sidecar.
//! See `docs/lld/oracle.md`.
//!
//! **This crate is the COMPARATOR**, F-011. It reads two directories in the
//! shape that half writes and returns a verdict per view against
//! `docs/hld/22-testing-and-tolerance.md` section 25.1. See
//! `docs/lld/comparator.md`.
//!
//! # What it compares today
//!
//! **No port code exists.** Decision D7 is that the validation oracle exists
//! before the port code, and this story does not break it. There is no Ocelli
//! renderer, so there is no Ocelli frame, so the comparator has no second side
//! of the corpus to compare against. A stub renderer was refused: its frames
//! would prove only that the stub and the comparator agree.
//!
//! So the comparator is a function of two directories, and today those two
//! directories are filled by three things, none of which is a renderer.
//!
//! 1. **Hand-constructed fixture frame pairs**, in `tests/`, whose divergence
//!    is known by construction because both sides' bytes are written out by
//!    hand. They run under `cargo test --workspace`, which is a floor gate, so
//!    the comparator's own arithmetic is proved in CI with no GPU and no
//!    corpus. Under deviation D-04 that is a real strengthening.
//! 2. **Identity over the real reference output.** It proves the loader, the
//!    identifier mapping, the class resolution, the sidecar contract and the
//!    report shape over all ninety-eight views, and it proves nothing about
//!    detection, which is why it is never the only corpus-scale exercise.
//! 3. **A declared mutation catalogue**, `mutations`, applied in memory to
//!    real reference frames with the verdict each must produce written down
//!    beside it. That is what proves detection at corpus scale.
//!
//! **The candidate side is a directory contract, not a call into a renderer.**
//! When the port lands, the Ocelli half writes `<id>.raw` and `<id>.json` in
//! the shape `docs/lld/oracle.md` already specifies, points the comparator at
//! it, and nothing here changes. That is also what makes deviation D-07's
//! tier A against tier C divergence bound this same binary with two candidate
//! directories and no reference, at no additional cost.
//!
//! # The tolerance policy
//!
//! Written down once and held, in `docs/hld/22-testing-and-tolerance.md`
//! section 25.1, and transcribed into `tolerance`. **A tolerance change is a
//! pull request with a rationale, reviewed like code.** Tuning tolerance per
//! failure is how a suite stops meaning anything, so the numbers live as
//! constants in source rather than in a configuration file, precisely so that
//! changing one is a diff a reviewer sees.

pub mod attribution;
pub mod frame;
pub mod geometry;
pub mod mutations;
pub mod report;
pub mod sidecar;
pub mod tolerance;

/// The crate's own name. The scaffold test asserts it matches Cargo's, which
/// is the one mistake a copy-pasted crate skeleton actually makes.
pub const CRATE_NAME: &str = env!("CARGO_PKG_NAME");

#[cfg(test)]
mod tests {
    #[test]
    fn the_crate_declares_its_own_name() {
        assert_eq!(super::CRATE_NAME, env!("CARGO_PKG_NAME"));
        assert!(super::CRATE_NAME.starts_with("ocelli"));
    }
}
