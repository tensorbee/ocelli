//! F-050. Restoring a viewport snapshot does not touch a render graph.
//!
//! `ocelli-viewport` does not depend on this crate. This test holds both and
//! reads the counters around a restore, which is the integrated half of that
//! claim. A missing adapter returns before any assertion.

use ocelli_render::RenderGraph;

fn graph() -> Option<RenderGraph> {
    let mut clock = {
        let start = std::time::Instant::now();
        move || u64::try_from(start.elapsed().as_nanos()).unwrap_or(u64::MAX)
    };
    let adapter = pollster::block_on(ocelli_render::resolve_adapter(
        ocelli_render::TierRequest::Auto,
        &mut clock,
    ))?;
    let mut context = pollster::block_on(adapter.open()).ok()?;
    RenderGraph::new(&mut context, 4, wgpu::TextureFormat::Rgba8Unorm).ok()
}

#[test]
#[ignore = "needs a real adapter, run by the gpu gate"]
fn restore_does_not_move_render_counters() {
    let Some(graph) = graph() else {
        return;
    };
    let submissions = graph.submissions();
    let uploads = graph.uploads();
    let lut_uploads = graph.lut_uploads();
    let brick_uploads = graph.brick_uploads();

    let built = ocelli_viewport::commands::StackViewport::try_identity(1);
    assert!(built.is_ok());
    let Ok(mut viewport) = built else {
        return;
    };
    let record = viewport.snapshot();
    assert!(record.is_ok());
    let Ok(record) = record else {
        return;
    };
    assert!(viewport.restore(&record).is_ok());

    assert_eq!(graph.submissions(), submissions);
    assert_eq!(graph.uploads(), uploads);
    assert_eq!(graph.lut_uploads(), lut_uploads);
    assert_eq!(graph.brick_uploads(), brick_uploads);
}
