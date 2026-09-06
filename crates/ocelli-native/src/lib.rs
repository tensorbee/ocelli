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

use core::future::Future;
use core::pin::pin;
use core::task::{Context, Poll, Waker};
use std::time::Instant;

use ocelli_render::{Resolution, Tier, TierRequest};

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

// ---------------------------------------------------------------------------
// F-004's operator override, and its named user.
//
// `docs/spikes/A7-tier-c.md` says to "always allow an operator override" and
// says nothing else about it. AGENTS.md forbids a feature flag with no named
// user, and an override no caller passes is exactly that, so these two entry
// points are the caller. They read the variable, resolve, and print both the
// tier and the evidence, because "the viewer is slow on that estate" is
// diagnosed from the evidence and not from the tier.
// ---------------------------------------------------------------------------

/// The environment variable the two entry points read.
///
/// Native only, by construction. In a browser the value comes from the shell,
/// because reading a query parameter, a config endpoint or `localStorage` is
/// DOM work and HLD section 10 puts DOM work in TypeScript.
pub const TIER_OVERRIDE_VAR: &str = "OCELLI_TIER";

/// Parse whatever the environment held, including nothing at all.
///
/// Separate from reading the environment so it can be tested without one. An
/// absent variable and `auto` are the same answer, so a deployment can set it
/// unconditionally, and anything unrecognised is refused rather than silently
/// treated as `auto`.
#[must_use]
pub fn tier_request(raw: Option<&str>) -> TierRequest {
    match raw {
        None => TierRequest::Auto,
        Some(value) => Tier::from_override_str(value),
    }
}

/// Resolve the tier for this process, honouring `OCELLI_TIER`.
///
/// `None` means the resolver did not complete, which on a native backend it
/// always should. See [`drive`].
#[must_use]
pub fn resolve_tier() -> Option<Resolution> {
    let raw = std::env::var(TIER_OVERRIDE_VAR).ok();
    let request = tier_request(raw.as_deref());
    let start = Instant::now();
    let mut clock = || u64::try_from(start.elapsed().as_nanos()).unwrap_or(u64::MAX);
    drive(ocelli_render::resolve(request, &mut clock))
}

/// How many times [`drive`] polls before giving up.
///
/// One is enough in practice and the bound exists so that "in practice" is not
/// load bearing.
const MAX_POLLS: u32 = 1024;

/// Run one future to completion with no async runtime.
///
/// `ocelli_render::resolve` is `async` because wgpu's adapter and device
/// requests are futures in the pinned version, not because anything here wants
/// to be. On a native backend both resolve synchronously, and wgpu's own
/// `Device::noop` relies on exactly that, polling each once with a no-op waker
/// and treating `Pending` as unreachable.
///
/// This does the same without asserting it. It polls in a bounded loop,
/// yielding the thread between attempts, and returns `None` if the bound is
/// reached. A future that genuinely parks then cannot hang an entry point, and
/// `unreachable!()` is not a shape this project accepts (HLD section 23: a
/// panic reachable from an exported path is a defect, not an error path).
///
/// It is here rather than in `ocelli-render` because the driver is the
/// caller's, the same way the clock is. A browser awaits the future on the
/// event loop and needs none of this.
fn drive(future: impl Future<Output = Resolution>) -> Option<Resolution> {
    let mut future = pin!(future);
    let mut context = Context::from_waker(Waker::noop());
    for _ in 0..MAX_POLLS {
        match future.as_mut().poll(&mut context) {
            Poll::Ready(resolution) => return Some(resolution),
            Poll::Pending => std::thread::yield_now(),
        }
    }
    None
}

/// The tier line and its evidence, as an entry point prints them.
///
/// The evidence is printed and not summarised. Deviation D-07's failure mode
/// is invisible and presents as "the viewer is slow", so the thing an operator
/// needs is which signal decided and what the other two said.
#[must_use]
pub fn tier_report(resolution: &Resolution) -> String {
    let evidence = &resolution.evidence;
    let mut out = format!(
        "Tier {tier:?}, decided by {decided:?}. {var} override: {outcome:?}.",
        tier = resolution.caps.tier,
        decided = evidence.decided_by,
        var = TIER_OVERRIDE_VAR,
        outcome = evidence.override_outcome,
    );
    out.push_str(&format!(
        "\n  caps      compute {compute}, max_tex_3d {tex}, max_buffer {buffer}",
        compute = resolution.caps.compute,
        tex = resolution.caps.max_tex_3d,
        buffer = resolution.caps.max_buffer,
    ));
    out.push_str(&format!(
        "\n  adapters  {seen} seen, device created {created}, measured tier {measured:?}",
        seen = evidence.adapters_seen,
        created = evidence.device_created,
        measured = evidence.measured_tier,
    ));
    if let Some(candidate) = &evidence.candidate {
        out.push_str(&format!(
            "\n  candidate {name:?} on {backend:?}, reported {device_type:?}",
            name = candidate.name,
            backend = candidate.backend,
            device_type = candidate.device_type,
        ));
    }
    out.push_str(&format!(
        "\n  signals   benchmark {bench:?}, adapter type {adapter:?}, renderer string {string:?}",
        bench = evidence.benchmark,
        adapter = evidence.adapter_type,
        string = evidence.renderer_string,
    ));
    if let Some(rate) = evidence.fill_rate {
        out.push_str(&format!(
            "\n  fill rate {pixels} pixels in {nanos} ns",
            pixels = rate.pixels_shaded,
            nanos = rate.elapsed_nanos,
        ));
    }
    out
}

/// What an entry point prints when resolution did not complete.
///
/// A stated non-answer rather than a tier nobody measured. Inventing one here
/// would be the same defect deviation D-07 exists to prevent, in the one place
/// it would be easiest to excuse.
pub const TIER_UNRESOLVED: &str = "Tier unresolved: the resolver did not complete within its poll bound. \
     No tier is printed, because printing one nobody measured is worse than \
     printing none.";

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

    /// An absent variable and `auto` are the same answer, so a deployment can
    /// set it unconditionally. Everything else round-trips through
    /// `ocelli-render`'s parser, including the refusal.
    #[test]
    fn the_override_variable_is_parsed_and_an_absent_one_means_auto() {
        use ocelli_render::{Tier, TierRequest};
        assert_eq!(super::tier_request(None), TierRequest::Auto);
        assert_eq!(super::tier_request(Some("auto")), TierRequest::Auto);
        assert_eq!(
            super::tier_request(Some("cpu")),
            TierRequest::Requested(Tier::Cpu)
        );
        assert_eq!(
            super::tier_request(Some("nonsense")),
            TierRequest::Unrecognised
        );
    }

    /// The variable is named once, in one constant, so the entry points and
    /// the documentation cannot drift apart on the spelling.
    #[test]
    fn the_override_variable_is_named_once() {
        assert_eq!(super::TIER_OVERRIDE_VAR, "OCELLI_TIER");
    }

    /// The report names the tier, the step that decided it, and what all three
    /// signals said. The last part is the point: deviation D-07's failure mode
    /// presents as "the viewer is slow", and the only thing that separates it
    /// from a slow machine is which signal decided.
    ///
    /// The `Resolution` is built by running `classify` over synthetic signals
    /// rather than by touching a GPU, so this runs in the CI floor.
    #[test]
    fn the_report_names_the_tier_the_decision_and_every_signal() {
        use ocelli_render::{
            AdapterFacts, FillRateBands, SimdSupport, TierRequest, TierSignals, classify,
        };

        let adapter = AdapterFacts {
            backend: wgpu::Backend::Gl,
            device_type: wgpu::DeviceType::Other,
            name: "SwiftShader Device (Subzero)".to_owned(),
            driver: String::new(),
            driver_info: String::new(),
            compute_shaders: false,
            webgpu_compliant: false,
            max_tex_3d: 256,
            max_buffer: 268_435_456,
        };
        let signals = TierSignals {
            adapters: vec![adapter],
            fill_rate: None,
            device_created: true,
            simd: SimdSupport::NotApplicable,
            bands: FillRateBands::RECORDED,
        };
        let report = super::tier_report(&classify(&signals, TierRequest::Auto));

        assert!(report.starts_with("Tier Cpu, "), "{report}");
        assert!(report.contains("decided by RendererString"), "{report}");
        assert!(
            report.contains("OCELLI_TIER override: NotRequested"),
            "{report}"
        );
        assert!(report.contains("benchmark Unknown"), "{report}");
        assert!(report.contains("adapter type Unknown"), "{report}");
        assert!(report.contains("renderer string Software"), "{report}");
        assert!(report.contains("SwiftShader"), "{report}");
    }

    /// A future that is ready on the first poll comes back on the first poll.
    /// The bound in `drive` exists so a future that parks cannot hang an entry
    /// point, not because anything native is expected to park.
    #[test]
    fn the_driver_returns_a_ready_future() {
        use ocelli_render::{FillRateBands, TierRequest, TierSignals, classify};
        let expected = classify(&TierSignals::unprobed(), TierRequest::Auto);
        let driven = super::drive(core::future::ready(expected));
        assert!(driven.is_some());
        assert_eq!(
            driven.map(|resolution| resolution.caps.tier),
            Some(ocelli_render::Tier::Cpu)
        );
        // Named so that changing the recorded bands does not silently change
        // what an unprobed session reports.
        assert_eq!(FillRateBands::RECORDED.software_ceiling_pps, None);
    }
}
