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
//! Scaffold only. F-001 creates the crate, F-007 gives it its two entry
//! points and proves the cross-target build.

// HLD section 4's crate table says `wasm: no` for this crate, and this is what
// makes that cell mean something.
//
// Before F-007 the cell was unenforceable: `cargo check -p ocelli-native
// --target wasm32-unknown-unknown` SUCCEEDED, because the crate is a stub with
// no native-only dependency and nothing declared it native-only. A guard
// asserting "it does not build for wasm32" would therefore have been asserting
// something that was not true.
//
// The fix is to make it true rather than to assert it. This turns the table
// cell into a compile error with a message that says which document it comes
// from, which is cheaper than discovering it when a dependency accidentally
// pulls this crate into a wasm build.
#[cfg(target_arch = "wasm32")]
compile_error!(
    "ocelli-native is native-only. HLD section 4's crate table gives it \
     `wasm: no`, and reaching it from a wasm build means something above it \
     acquired a dependency it must not have."
);

/// The crate's own name. The scaffold test asserts it matches Cargo's, which
/// is the one mistake a copy-pasted crate skeleton actually makes.
pub const CRATE_NAME: &str = env!("CARGO_PKG_NAME");

/// The four extension points of HLD section 13, in the document's order.
///
/// This is the list both entry points print, and it is not decoration. Section
/// 13's whole claim is that Phases 2 and 3 are new ENTRY POINTS rather than new
/// implementations, and these four are what that claim rests on. A stub that
/// prints them says what it is for, which a stub that prints nothing does not.
pub const EXTENSION_POINTS: [&str; 4] = [
    "SeriesSource, so a DIMSE implementation lands without touching anything above it",
    "a render target trait, so server-side rendering reuses ocelli-render unchanged",
    "a dynamic codec registry, so a native build links C codecs the browser cannot",
    "calibrated display presentation, PS3.14, unreachable from a web page",
];

/// What an entry point prints. Shared so the two binaries cannot drift.
#[must_use]
pub fn entry_point_banner(binary: &str) -> String {
    let mut out = format!(
        "{binary} {version}, {CRATE_NAME}\nStub. Phase 2 and Phase 3 fill it. \
         Extension points it will implement:",
        version = env!("CARGO_PKG_VERSION"),
    );
    for point in EXTENSION_POINTS {
        out.push_str("\n  - ");
        out.push_str(point);
    }
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn scaffold_declares_its_own_name() {
        assert_eq!(super::CRATE_NAME, env!("CARGO_PKG_NAME"));
        assert!(super::CRATE_NAME.starts_with("ocelli"));
    }

    /// The banner names the binary it was asked about, the crate, and all four
    /// of section 13's extension points.
    ///
    /// The count is asserted against the literal 4 rather than against
    /// `EXTENSION_POINTS.len()`, which would restate the array and pass
    /// however many entries it had. Section 13 gives four.
    #[test]
    fn banner_names_the_binary_and_all_four_extension_points() {
        let banner = super::entry_point_banner("ocelli-server");
        assert!(banner.starts_with("ocelli-server "));
        assert!(banner.contains(super::CRATE_NAME));
        assert_eq!(banner.matches("\n  - ").count(), 4);
        for point in super::EXTENSION_POINTS {
            assert!(banner.contains(point), "banner omits: {point}");
        }
    }

    /// The two binaries get different banners. A shared helper that ignored
    /// its argument would pass every other assertion here.
    #[test]
    fn the_two_entry_points_are_distinguishable() {
        assert_ne!(
            super::entry_point_banner("ocelli-desktop"),
            super::entry_point_banner("ocelli-server")
        );
    }
}
