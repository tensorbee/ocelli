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
//! F-005 adds `src/panic.rs`, which is section 23's panic path. It contains no
//! `unsafe`: the fixed record is written through atomics, which need only a
//! shared reference. The allow-list is unchanged and still names two files
//! that do not exist.
//!
//! Scaffold otherwise. F-001 creates the crate, F-002 builds the wasm pipeline
//! around it, F-101 builds the boundary.

pub mod panic;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::wasm_bindgen;

/// The crate's own name. The scaffold test asserts it matches Cargo's, which
/// is the one mistake a copy-pasted crate skeleton actually makes.
pub const CRATE_NAME: &str = env!("CARGO_PKG_NAME");

/// The built core's version, as the shell sees it.
///
/// This is the module's entire exported surface until F-101 (E16.2) builds the
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

/// Install the panic hook. HLD section 23.
///
/// **A worker calls this once, before any other call.** Everything after it
/// depends on the hook already being installed, because a panic before it is
/// a panic nobody recorded.
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn install_panic_hook() {
    panic::install();
}

/// The address of the panic record in linear memory.
///
/// Called once, straight after instantiation, and cached. **Never called
/// again**, and in particular never after a trap: section 23 says a poisoned
/// instance must not be reused, and asking a poisoned instance why it was
/// poisoned is reusing it. See `crates/ocelli-wasm/src/panic.rs`.
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
#[must_use]
pub fn panic_record_ptr() -> u32 {
    panic::record_ptr()
}

/// The length of the panic record in bytes. Cached alongside the pointer.
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
#[must_use]
pub fn panic_record_len() -> u32 {
    panic::record_len()
}

/// Trap this instance on purpose, so `scripts/panic_probe.mjs` can watch what
/// happens next.
///
/// **Behind the `panic-probe` feature, which the shipped artefact does not
/// carry.** `bin/ocelli.sh wasm` builds without it, so the module measured
/// against `ci/wasm-size-budget.json` and published to npm has no way to be
/// asked to panic. `bin/ocelli.sh gate panic` builds a second module with it,
/// into its own out-dir, and probes that.
///
/// AGENTS.md forbids a feature flag without a named user. The named user is
/// `scripts/panic_probe.mjs`, and the property it establishes is the one this
/// whole story rests on: that the hook runs at all under `panic = "abort"` on
/// wasm32, and that linear memory is still readable from JavaScript after the
/// trap.
///
/// **No lint is switched off to write this and no `#[allow]` is added.** The
/// workspace denies `clippy::panic`, which covers `panic!` and
/// `std::panic::panic_any`, and that denial is right. So the panic arrives the
/// way a real one will: an assertion that does not hold. That is also the
/// most representative shape available, because the likeliest panic in this
/// system is a decode worker asserting something about untrusted bytes.
#[cfg(feature = "panic-probe")]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn panic_probe_trigger() {
    assert!(probe_invariant_holds(), "ocelli panic probe, F-005");
}

/// Always false, computed at runtime so it is not a constant.
///
/// `core::hint::black_box` is the documented way to say "treat this value as
/// opaque", and it is what makes the assertion above a runtime check rather
/// than `assert!(false)`.
#[cfg(feature = "panic-probe")]
fn probe_invariant_holds() -> bool {
    core::hint::black_box(false)
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

    /// The two integers the shell caches at startup are readable before
    /// anything can panic.
    ///
    /// `panic_record_len` is asserted as a literal, not as
    /// `PANIC_RECORD_BYTES`, because it is a wire contract with
    /// `packages/core/src/panic.ts`, which asserts the same 528. Restating the
    /// constant would restate the implementation, which HLD 27.2 R2 says is
    /// not a test.
    ///
    /// `panic_record_ptr` is not asserted nonzero here. On a 64-bit host the
    /// address does not fit a `u32` and 0 is the correct answer, and the
    /// wasm32 answer is `scripts/panic_probe.mjs`'s to check.
    #[test]
    fn the_panic_record_length_is_readable_before_any_panic() {
        assert_eq!(super::panic_record_len(), 528);
        assert_ne!(crate::panic::record_address(), 0);
    }
}
