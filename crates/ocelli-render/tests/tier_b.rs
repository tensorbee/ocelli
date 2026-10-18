//! F-042. Tier B is the fragment variant, and a compute key is not that variant.
//!
//! The four section 18.3 rows are PS3.3 C.11.2.1.2 and C.11.2.1.3.2 at centre
//! 40, width 400, output 0 to 255. Deviation D-13 keeps LINEAR_EXACT at -160
//! on ymin, which is 0.000. The tolerance is 0.001 and is not widened.
//!
//! The native GL test records a miss when `Backends::GL` finds nothing. That
//! miss is not a tier B run. The browser run is `tools/tier-b/run.mjs`.

use ocelli_render::{
    BlendMode, FrameOutcome, PassKind, PipelineFamily, PipelineKey, RenderGraph, Section183Error,
    Tier, ViewportId, read_section_18_3, resolve_native_gl,
};

/// Not widened. Matches `tests/voi_shader.rs` and `tests/upload.rs`.
const FIXTURE_TOLERANCE: f32 = 0.001;

/// LINEAR. Centre 40, width 400. The centre sample is 127.819.
const LINEAR: [f32; 4] = [0.0, 127.819, 255.0, 63.910];

/// LINEAR_EXACT. D-13 keeps the first sample at 0.000, not 1.594.
const LINEAR_EXACT: [f32; 4] = [0.0, 127.5, 255.0, 63.75];

#[test]
fn a_compute_or_storage_key_on_tier_b_is_unavailable() {
    let key = PipelineKey {
        pass: PassKind::Stack,
        blend: BlendMode::Composite,
        tier: Tier::B,
    };
    let viewport = ViewportId(3);
    assert!(RenderGraph::pipeline_compiled(&key));
    assert!(RenderGraph::pipeline_family_compiled(
        &key,
        PipelineFamily::Fragment
    ));
    assert!(!RenderGraph::pipeline_family_compiled(
        &key,
        PipelineFamily::Compute
    ));
    assert!(!RenderGraph::pipeline_family_compiled(
        &key,
        PipelineFamily::Storage
    ));
    assert_eq!(
        RenderGraph::request_outcome(&key, PipelineFamily::Compute, viewport),
        FrameOutcome::Unavailable { viewport }
    );
    assert_eq!(
        RenderGraph::request_outcome(&key, PipelineFamily::Storage, viewport),
        FrameOutcome::Unavailable { viewport }
    );
    let same_on_a = PipelineKey {
        pass: PassKind::Stack,
        blend: BlendMode::Composite,
        tier: Tier::A,
    };
    assert_eq!(
        RenderGraph::request_outcome(&same_on_a, PipelineFamily::Compute, viewport),
        FrameOutcome::Unavailable { viewport }
    );
}

fn native_clock() -> impl FnMut() -> u64 {
    let start = std::time::Instant::now();
    move || u64::try_from(start.elapsed().as_nanos()).unwrap_or(u64::MAX)
}

fn assert_close(actual: f32, expected: f32) {
    let difference = (actual - expected).abs();
    assert!(
        difference < FIXTURE_TOLERANCE,
        "was {actual}, wanted {expected}, difference {difference}"
    );
}

fn assert_row(actual: &[f32; 4], expected: &[f32; 4]) {
    for (index, (got, want)) in actual
        .iter()
        .copied()
        .zip(expected.iter().copied())
        .enumerate()
    {
        let difference = (got - want).abs();
        assert!(
            difference < FIXTURE_TOLERANCE,
            "index {index}: was {got}, wanted {want}"
        );
    }
}

#[test]
#[ignore = "native GL is not this host's tier B. A miss is recorded and is not the run."]
fn native_gl_section_18_3_or_a_recorded_miss() {
    let mut clock = native_clock();
    let Some(adapter) = pollster::block_on(resolve_native_gl(&mut clock)) else {
        println!("native Backends::GL found no adapter. This is not a tier B run.");
        return;
    };
    let mut context = match pollster::block_on(adapter.open()) {
        Ok(context) => context,
        Err(error) => {
            println!("native GL adapter did not open a device: {error}. This is not a tier B run.");
            return;
        }
    };
    let tier = context.caps().tier;
    println!("native GL opened tier {tier:?} renderer probe continues");
    match read_section_18_3(&mut context, &adapter) {
        Err(Section183Error::Unavailable) => {
            println!("native GL stack pass is unavailable and drew nothing.");
        }
        Err(Section183Error::TierC) => {
            println!("native GL resolved tier C. This is not a tier B run.");
        }
        Err(error @ (Section183Error::NotSubmitted | Section183Error::Readback(_))) => {
            assert!(
                matches!(error, Section183Error::Unavailable),
                "native GL draw failed: {error:?}"
            );
        }
        Ok(pixels) => {
            println!(
                "native GL format {} tier {tier:?}",
                ocelli_render::stored_format_name(pixels.format)
            );
            assert_ne!(pixels.format, wgpu::TextureFormat::Rgba8Unorm);
            assert_row(&pixels.linear, &LINEAR);
            assert_row(&pixels.linear_exact, &LINEAR_EXACT);
            assert_row(&pixels.linear, &pixels.cpu_linear);
            assert_row(&pixels.linear_exact, &pixels.cpu_linear_exact);
            let exact_low = pixels.linear_exact.first().copied().unwrap_or(f32::NAN);
            assert_close(exact_low, 0.0);
            let centre_linear = pixels.linear.get(1).copied().unwrap_or(f32::NAN);
            let centre_exact = pixels.linear_exact.get(1).copied().unwrap_or(f32::NAN);
            let gap = (centre_linear - centre_exact).abs();
            assert!(gap > 0.2, "centre gap was {gap}");
            if tier != Tier::B {
                println!(
                    "native GL drew matching numbers on tier {tier:?}. This is not a tier B run."
                );
            }
        }
    }
}
