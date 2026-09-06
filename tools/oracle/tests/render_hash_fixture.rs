//! Literal-byte fixtures for F-015's stable render-hash contract.
//!
//! The expected digests were computed independently with Python's
//! `hashlib.sha256`, using the byte framing transcribed in the approved plan.
//! They are constants rather than values produced by the Rust implementation.

use ocelli_oracle::frame::Frame;
use ocelli_oracle::render_hash::{RenderHash, render_hash, run_render_hash};
use ocelli_oracle::sidecar::ViewKind;

const FIXTURE_BYTES: [u8; 8] = [0, 0, 0, 255, 1, 2, 3, 255];
const FIXTURE_VIEW_HASH: &str = "7ce1f3d20a7aa3652620049f76d3b99123acc1ccf35cd59649bef6a87c1785e0";
const FIXTURE_RUN_HASH: &str = "de6faff9bd5be9643ca1972c171b03562e435a48ab1dbaf20446ea10f4a712c9";

fn fixture() -> Result<Frame, Box<dyn std::error::Error>> {
    Ok(Frame::new(2, 1, FIXTURE_BYTES.to_vec())?)
}

#[test]
fn literal_rgba8_fixture_has_the_independent_digest() -> Result<(), Box<dyn std::error::Error>> {
    let hash = render_hash(ViewKind::Stack, "fixture", &fixture()?);
    assert_eq!(hash.sha256, FIXTURE_VIEW_HASH);
    assert_eq!(
        run_render_hash(std::slice::from_ref(&hash)),
        FIXTURE_RUN_HASH
    );
    Ok(())
}

#[test]
fn one_pixel_dimension_kind_and_identifier_are_all_bound() -> Result<(), Box<dyn std::error::Error>>
{
    let frame = fixture()?;
    let baseline = render_hash(ViewKind::Stack, "fixture", &frame);

    let mut changed_pixel = frame.clone();
    changed_pixel.set_pixel(1, 0, [1, 2, 4, 255])?;
    assert_ne!(
        render_hash(ViewKind::Stack, "fixture", &changed_pixel),
        baseline
    );

    let reshaped = Frame::new(1, 2, FIXTURE_BYTES.to_vec())?;
    assert_ne!(render_hash(ViewKind::Stack, "fixture", &reshaped), baseline);
    assert_ne!(
        render_hash(ViewKind::VolumeReformat, "fixture", &frame),
        baseline
    );
    assert_ne!(render_hash(ViewKind::Stack, "fixture-2", &frame), baseline);
    Ok(())
}

#[test]
fn run_hash_is_canonical_over_input_order() -> Result<(), Box<dyn std::error::Error>> {
    let frame = fixture()?;
    let a = render_hash(ViewKind::Stack, "a", &frame);
    let b = render_hash(ViewKind::Stack, "b", &frame);
    assert_eq!(
        run_render_hash(&[a.clone(), b.clone()]),
        run_render_hash(&[b, a])
    );
    Ok(())
}

#[test]
fn changing_any_fixture_byte_changes_both_hashes() -> Result<(), Box<dyn std::error::Error>> {
    let baseline = render_hash(ViewKind::Stack, "fixture", &fixture()?);
    let baseline_run = run_render_hash(std::slice::from_ref(&baseline));
    for index in 0..FIXTURE_BYTES.len() {
        let mut bytes = FIXTURE_BYTES;
        if let Some(byte) = bytes.get_mut(index) {
            *byte ^= 1;
        }
        let changed = render_hash(
            ViewKind::Stack,
            "fixture",
            &Frame::new(2, 1, bytes.to_vec())?,
        );
        assert_ne!(
            changed, baseline,
            "byte {index} did not affect the view hash"
        );
        assert_ne!(
            run_render_hash(&[changed]),
            baseline_run,
            "byte {index} did not affect the run hash"
        );
    }
    Ok(())
}

#[test]
fn duplicate_identifiers_still_contribute_twice() -> Result<(), Box<dyn std::error::Error>> {
    let hash = render_hash(ViewKind::Stack, "fixture", &fixture()?);
    assert_ne!(
        run_render_hash(std::slice::from_ref(&hash)),
        run_render_hash(&[hash.clone(), RenderHash { ..hash }])
    );
    Ok(())
}
