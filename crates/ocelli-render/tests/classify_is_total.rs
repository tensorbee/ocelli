//! `classify` is total, and the tier it returns is one the evidence permits.
//!
//! The table-driven cases in `caps.rs` say what the resolver does on the
//! combinations a human thought of. This says what it never does on the
//! combinations nobody did.
//!
//! Three properties, and the second and third are the ones that would catch a
//! future edit to the combination rule:
//!
//! 1. It always returns. No panic, no `unwrap`, no arithmetic overflow, for
//!    any signal combination and any request. HLD section 23: a panic inside
//!    WebAssembly poisons the module instance, so a panic reachable from an
//!    exported path is a defect and not an error path.
//! 2. **It never returns a tier the evidence cannot construct.** Tier A only
//!    where an A-candidate adapter exists, tier B only where some candidate
//!    does. That is the clamp of the override rule, stated as an invariant
//!    over every path rather than only over the override path.
//! 3. Tier C carries no invented figures.

use ocelli_render::caps::{
    AdapterFacts, DecidedBy, FailedAdapter, FillRate, FillRateBands, OverrideOutcome, ProbeOutcome,
    SimdSupport, Tier, TierRequest, TierSignals, classify,
};
use proptest::prelude::*;

fn any_backend() -> impl Strategy<Value = wgpu::Backend> {
    prop_oneof![
        Just(wgpu::Backend::Noop),
        Just(wgpu::Backend::Vulkan),
        Just(wgpu::Backend::Metal),
        Just(wgpu::Backend::Dx12),
        Just(wgpu::Backend::Gl),
        Just(wgpu::Backend::BrowserWebGpu),
    ]
}

fn any_device_type() -> impl Strategy<Value = wgpu::DeviceType> {
    prop_oneof![
        Just(wgpu::DeviceType::Other),
        Just(wgpu::DeviceType::IntegratedGpu),
        Just(wgpu::DeviceType::DiscreteGpu),
        Just(wgpu::DeviceType::VirtualGpu),
        Just(wgpu::DeviceType::Cpu),
    ]
}

/// Names drawn from a small set that mixes clean strings with A7 matches, so
/// the renderer-string branch is reached often rather than by luck.
fn any_name() -> impl Strategy<Value = String> {
    prop_oneof![
        Just("Apple M5 Max".to_owned()),
        Just("NVIDIA GeForce RTX 4090".to_owned()),
        Just("Intel(R) Iris(R) Xe Graphics".to_owned()),
        Just("SwiftShader Device (Subzero)".to_owned()),
        Just("llvmpipe (LLVM 15.0.7, 256 bits)".to_owned()),
        Just("Gallium 0.4 on AMD RADV POLARIS10".to_owned()),
        Just(String::new()),
    ]
}

fn any_facts() -> impl Strategy<Value = AdapterFacts> {
    (
        any_backend(),
        any_device_type(),
        any_name(),
        any::<bool>(),
        any::<bool>(),
        any::<u32>(),
        any::<u64>(),
    )
        .prop_map(
            |(backend, device_type, name, compute_shaders, webgpu_compliant, tex, buffer)| {
                AdapterFacts {
                    backend,
                    device_type,
                    name,
                    driver: String::new(),
                    driver_info: String::new(),
                    compute_shaders,
                    webgpu_compliant,
                    max_tex_3d: tex,
                    max_buffer: buffer,
                }
            },
        )
}

fn any_bands() -> impl Strategy<Value = FillRateBands> {
    (any::<u64>(), any::<Option<u64>>()).prop_map(|(floor, ceiling)| FillRateBands {
        hardware_floor_pps: floor,
        software_ceiling_pps: ceiling,
    })
}

fn any_failed_adapter() -> impl Strategy<Value = FailedAdapter> {
    any_facts().prop_map(|adapter| FailedAdapter {
        adapter,
        reason: "request_device failed".to_owned(),
    })
}

fn any_signals() -> impl Strategy<Value = TierSignals> {
    let no_adapter =
        (0_usize..4).prop_map(|adapters_seen| ProbeOutcome::NoAdapter { adapters_seen });
    let no_device = (
        prop::collection::vec(any_failed_adapter(), 1..4),
        0_usize..4,
    )
        .prop_map(|(failed, non_candidates)| ProbeOutcome::NoDevice {
            adapters_seen: failed.len() + non_candidates,
            failed,
        });
    let opened = (
        prop::collection::vec(any_failed_adapter(), 0..3),
        any_facts().prop_filter("an opened adapter is a candidate", |facts| {
            facts.candidate_tier().is_some()
        }),
        any::<Option<(u64, u64)>>(),
        0_usize..4,
    )
        .prop_map(
            |(failed, adapter, rate, non_candidates)| ProbeOutcome::Opened {
                adapters_seen: failed.len() + non_candidates + 1,
                failed,
                adapter,
                fill_rate: rate.map(|(pixels_shaded, elapsed_nanos)| FillRate {
                    pixels_shaded,
                    elapsed_nanos,
                }),
            },
        );

    (prop_oneof![no_adapter, no_device, opened], any_bands()).prop_map(|(probe, bands)| {
        TierSignals {
            probe,
            simd: SimdSupport::NotApplicable,
            bands,
        }
    })
}

fn any_request() -> impl Strategy<Value = TierRequest> {
    prop_oneof![
        Just(TierRequest::Auto),
        Just(TierRequest::Unrecognised),
        Just(TierRequest::Requested(Tier::A)),
        Just(TierRequest::Requested(Tier::B)),
        Just(TierRequest::Requested(Tier::Cpu)),
    ]
}

proptest! {
    #[test]
    fn classify_is_total_and_never_invents_a_tier(
        signals in any_signals(),
        request in any_request(),
    ) {
        let resolved = classify(&signals, request);

        let opened_tier = match &signals.probe {
            ProbeOutcome::Opened { adapter, .. } => adapter.candidate_tier(),
            ProbeOutcome::NoAdapter { .. } | ProbeOutcome::NoDevice { .. } => None,
        };

        match resolved.caps.tier {
            // Property 2. Tier A needs an adapter that could serve it, on
            // every path including the override's.
            Tier::A => prop_assert_eq!(opened_tier, Some(Tier::A)),
            Tier::B => prop_assert!(opened_tier.is_some()),
            // Property 3. Tier C has no device, so any other figure would be
            // invented.
            Tier::Cpu => {
                prop_assert!(!resolved.caps.compute);
                prop_assert_eq!(resolved.caps.max_tex_3d, 0);
                prop_assert_eq!(resolved.caps.max_buffer, 0);
            }
        }

        // Compute is a tier A property and nothing else grants it.
        if resolved.caps.compute {
            prop_assert_eq!(resolved.caps.tier, Tier::A);
        }

        // `decided_by: Override` and an applied override are the same event
        // seen from two sides, so neither may appear without the other.
        let applied = matches!(resolved.evidence.override_outcome, OverrideOutcome::Applied(_));
        prop_assert_eq!(resolved.evidence.decided_by == DecidedBy::Override, applied);

        // An unrecognised value is refused as unrecognised, always. It is
        // never quietly folded into `auto`.
        if request == TierRequest::Unrecognised {
            prop_assert_eq!(
                resolved.evidence.override_outcome,
                OverrideOutcome::RefusedUnrecognised
            );
        }

        // A GPU tier ALWAYS means a device was created, whatever was
        // requested. This used to be guarded by `request == TierRequest::Auto`
        // and therefore said nothing about the override path, which is where
        // the S03 sprint review found the defect: `OCELLI_TIER=a` on a host
        // where no device could be opened returned `Tier::A` with
        // `Applied(A)`, because the override arm checked only that a tier-A
        // adapter had been ENUMERATED. An adapter existing and a device
        // opening are different facts.
        //
        // Deviation D-07 is why this is not cosmetic. Tier resolution exists so
        // that a machine which cannot run a GPU path is told so, and an
        // override able to manufacture a tier out of an adapter listing is that
        // same defect arriving through the door marked diagnostic.
        if resolved.caps.tier != Tier::Cpu {
            prop_assert!(
                matches!(signals.probe, ProbeOutcome::Opened { .. }),
                "resolved {:?} without an opened adapter, request {:?}",
                resolved.caps.tier,
                request
            );
            prop_assert!(!resolved.caps.compute || resolved.caps.tier == Tier::A);
        }

        // The candidate identity still only holds without an override, because
        // forcing tier B onto an A-candidate is deliberately allowed and
        // recorded.
        if request == TierRequest::Auto && resolved.caps.tier != Tier::Cpu {
            prop_assert_eq!(resolved.evidence.candidate_tier, Some(resolved.caps.tier));
        }

        if let ProbeOutcome::NoDevice { failed, .. } = &signals.probe {
            prop_assert!(!failed.is_empty());
            if request != TierRequest::Requested(Tier::Cpu) {
                prop_assert_eq!(&resolved.evidence.failed_adapters, failed);
            }
        }
    }
}
