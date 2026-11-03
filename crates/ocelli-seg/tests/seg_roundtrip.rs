//! F-098 SEG frame round trip.
//!
//! Hand-stated from `.claude/plans/F-098-design.md`. PS3.3 C.8.20.2 names
//! the segment number. There is no DICOM file. Frame order is not series order.

use ocelli_seg::{SegError, SegFrame, decode_seg, encode_seg};

fn frame(uid: &str, samples: &[f64]) -> SegFrame {
    SegFrame {
        referenced_sop_instance_uid: uid.to_string(),
        rows: 2,
        columns: 1,
        samples: samples.to_vec(),
    }
}

fn must_encode(frames: &[SegFrame]) -> Vec<SegFrame> {
    match encode_seg(frames) {
        Ok(copied) => copied,
        Err(_) => unreachable!("the plan's valid frames"),
    }
}

fn must_decode(frames: &[SegFrame]) -> Vec<SegFrame> {
    match decode_seg(frames) {
        Ok(copied) => copied,
        Err(_) => unreachable!("the plan's encoded frames"),
    }
}

#[test]
fn two_frames_keep_caller_order_and_segment_zero() {
    let input = vec![frame("1.2.9", &[0.0, 1.0]), frame("1.2.8", &[2.0, 0.0])];
    let before = input.clone();
    let decoded = must_decode(&must_encode(&input));
    assert_eq!(decoded, input);
    assert_eq!(input, before);
}

#[test]
fn the_same_sop_stays_twice_in_caller_order() {
    let input = vec![frame("1.2.9", &[0.0, 1.0]), frame("1.2.9", &[4.0, 0.0])];
    let decoded = must_decode(&must_encode(&input));
    assert_eq!(decoded, input);
}

#[test]
fn a_bad_frame_refuses_the_whole_call() {
    assert_eq!(
        encode_seg(&[frame("1.2.9", &[-1.0, 0.0])]),
        Err(SegError::Sample)
    );
    assert_eq!(
        encode_seg(&[frame("1.2.9", &[1.5, 0.0])]),
        Err(SegError::Sample)
    );
    assert_eq!(encode_seg(&[frame("1.2.9", &[0.0])]), Err(SegError::Length));
    let mut zero_rows = frame("1.2.9", &[0.0, 1.0]);
    zero_rows.rows = 0;
    assert_eq!(encode_seg(&[zero_rows]), Err(SegError::Side));
    assert_eq!(encode_seg(&[frame("", &[0.0, 1.0])]), Err(SegError::Uid));
}

#[test]
fn one_bad_frame_does_not_keep_an_earlier_good_frame() {
    let input = vec![frame("1.2.9", &[0.0, 1.0]), frame("", &[0.0, 1.0])];
    let before = input.clone();
    assert_eq!(encode_seg(&input), Err(SegError::Uid));
    assert_eq!(input, before);
}
