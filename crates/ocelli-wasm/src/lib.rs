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
//! Scaffold only. F-001 creates the crate, F-002 builds the wasm pipeline
//! around it, F-096 builds the boundary.

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::wasm_bindgen;

/// The crate's own name. The scaffold test asserts it matches Cargo's, which
/// is the one mistake a copy-pasted crate skeleton actually makes.
pub const CRATE_NAME: &str = env!("CARGO_PKG_NAME");

/// The built core's version, as the shell sees it.
///
/// This is the module's entire exported surface until F-096 (E16.2) builds the
/// boundary, and it exists for two reasons that are both about measurement
/// rather than about features.
///
/// The first is that a wasm module with no export measures nothing. HLD
/// section 15.2's release profile is `lto = "fat"` with `strip = true`, and a
/// linker given no reachable root is free to discard the world. The size
/// budget of `ci/wasm-size-budget.json` would then be recording the size of
/// nothing.
///
/// The second is that `packages/core/src/index.ts` already carries a
/// `VERSION` constant and a `coreAvailable()` that returns `false` because no
/// core has ever been built. This is the value those two eventually agree
/// with, so the seam is real rather than invented to have an export.
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
#[must_use]
pub fn ocelli_version() -> String {
    env!("CARGO_PKG_VERSION").to_owned()
}

#[cfg(test)]
mod tests {
    #[test]
    fn scaffold_declares_its_own_name() {
        assert_eq!(super::CRATE_NAME, env!("CARGO_PKG_NAME"));
        assert!(super::CRATE_NAME.starts_with("ocelli"));
    }

    /// The exported version is the workspace version, written out.
    ///
    /// **The literal is deliberate and the obvious form is a bug.** Asserting
    /// `ocelli_version() == env!("CARGO_PKG_VERSION")` restates the function
    /// body, so it passes whatever the body returns and is not a test. HLD
    /// 27.2 R2 is exactly this failure: a test that asserts the
    /// implementation is itself.
    ///
    /// The cost is that a version bump has to update this string too, next to
    /// `[workspace.package].version` and `packages/core/src/index.ts`'s
    /// `VERSION`. `/release` does not do that, deliberately: `docs/RELEASE.md`
    /// says the bump lands earlier through its own F-ID and that `/release`
    /// never edits a version.
    ///
    /// **That cost is the mechanism, not a price paid for one.** No separate
    /// guard asserts the three agree, because this test already does: bump
    /// the workspace version without touching this literal and it goes red
    /// immediately, naming both values.
    #[test]
    fn exported_version_is_the_workspace_version() {
        assert_eq!(super::ocelli_version(), "0.1.0");
    }
}
