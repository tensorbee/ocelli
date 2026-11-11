//! Display-assembly fixtures for F-192.
//!
//! Expected orders are hand-computed from DICOM PS3.3 C.7.6.2.1.1.
//! Axial row `(1,0,0)` and column `(0,1,0)` give normal `(0,0,1)`, so the
//! projected position is the z component of Image Position Patient.
//! Sagittal row `(0,1,0)` and column `(0,0,-1)` give normal `(-1,0,0)`.

use ocelli_dicom::{
    CineTiming, DisplaySeries, MetadataSet, ParsedDicom, ParsedFrame, Playback, SeriesOpenError,
    SeriesOpenReason, SpatialOrder, assemble_display, cine_timing, parse_part10,
};

const AXIAL: &str = "1\\0\\0\\0\\1\\0";
const SAGITTAL: &str = "0\\1\\0\\0\\0\\-1";
const ROTATED: &str = "0\\1\\0\\-1\\0\\0";
const SERIES: &str = "2.25.10";
const FRAME_OF_REFERENCE: &str = "2.25.11";

struct Slice {
    sop: &'static str,
    series: &'static str,
    frame_of_reference: Option<&'static str>,
    instance: Option<&'static str>,
    position: Option<&'static str>,
    orientation: Option<&'static str>,
    spacing: &'static str,
    temporal: Option<u32>,
    stack_id: Option<&'static str>,
    in_stack: Option<u32>,
    frame_time: Option<&'static str>,
    frame_time_vector: Option<&'static str>,
    recommended_rate: Option<&'static str>,
    cine_rate: Option<&'static str>,
    playback: Option<u16>,
    playback_cs: Option<&'static str>,
    frames: Option<&'static str>,
}

fn slice(sop: &'static str) -> Slice {
    Slice {
        sop,
        series: SERIES,
        frame_of_reference: Some(FRAME_OF_REFERENCE),
        instance: None,
        position: None,
        orientation: Some(AXIAL),
        spacing: "1\\1",
        temporal: None,
        stack_id: None,
        in_stack: None,
        frame_time: None,
        frame_time_vector: None,
        recommended_rate: None,
        cine_rate: None,
        playback: None,
        playback_cs: None,
        frames: None,
    }
}

fn assemble(parts: &[Slice]) -> Result<Vec<DisplaySeries>, SeriesOpenError> {
    let owned: Vec<ParsedDicom> = parts.iter().map(parse_slice).collect();
    let frames: Vec<ParsedFrame<'_>> = owned
        .iter()
        .map(|parsed| ParsedFrame::new(parsed, 0))
        .collect();
    assemble_display(&frames)
}

fn one_group(result: Result<Vec<DisplaySeries>, SeriesOpenError>) -> DisplaySeries {
    assert!(result.is_ok(), "{result:?}");
    let Ok(mut groups) = result else {
        unreachable!("asserted immediately above")
    };
    assert_eq!(groups.len(), 1);
    let Some(group) = groups.pop() else {
        unreachable!("length is 1")
    };
    group
}

fn two_groups(
    result: Result<Vec<DisplaySeries>, SeriesOpenError>,
) -> (DisplaySeries, DisplaySeries) {
    assert!(result.is_ok(), "{result:?}");
    let Ok(groups) = result else {
        unreachable!("asserted immediately above")
    };
    assert_eq!(groups.len(), 2);
    let mut iter = groups.into_iter();
    let Some(first) = iter.next() else {
        unreachable!("length is 2")
    };
    let Some(second) = iter.next() else {
        unreachable!("length is 2")
    };
    (first, second)
}

fn places(group: &DisplaySeries) -> Vec<(u32, u32)> {
    let mut out = Vec::new();
    for slot in group.slots() {
        for place in slot {
            out.push((place.input_index(), place.frame_index()));
        }
    }
    out
}

fn slot_lengths(group: &DisplaySeries) -> Vec<usize> {
    group.slots().iter().map(Vec::len).collect()
}

fn expect_reason(result: Result<Vec<DisplaySeries>, SeriesOpenError>, reason: SeriesOpenReason) {
    assert!(result.is_err(), "{result:?}");
    let Err(error) = result else {
        unreachable!("asserted immediately above")
    };
    assert_eq!(error.reason(), reason);
}

fn parse_slice(part: &Slice) -> ParsedDicom {
    let encoded = encode(part);
    assert!(encoded.is_ok(), "fixture bytes");
    let Ok(encoded) = encoded else {
        unreachable!("asserted immediately above")
    };
    let parsed = parse_part10(&encoded);
    assert!(parsed.is_ok(), "{parsed:?}");
    let Ok(parsed) = parsed else {
        unreachable!("asserted immediately above")
    };
    parsed
}

#[test]
fn series_open_axial_order_is_the_projection() {
    // PS3.3 C.7.6.2.1.1. z = 3 is sent first with the smaller instance number.
    // Projected order is z = 0 then z = 3. Instance order would be the reverse.
    let mut later = slice("2.25.21");
    later.instance = Some("1");
    later.position = Some("0\\0\\3");
    let mut earlier = slice("2.25.20");
    earlier.instance = Some("2");
    earlier.position = Some("0\\0\\0");
    let parts = [later, earlier];
    let group = one_group(assemble(&parts));
    assert_eq!(group.order(), SpatialOrder::Projected);
    assert_eq!(places(&group), vec![(1, 0), (0, 0)]);
    assert!(!group.sheared());
    assert!(!group.non_uniform_step());
    let again = one_group(assemble(&parts));
    assert_eq!(places(&again), places(&group));
}

#[test]
fn series_open_axial_and_sagittal_are_two_groups() {
    let mut axial = slice("2.25.20");
    axial.position = Some("0\\0\\0");
    let mut sagittal = slice("2.25.22");
    sagittal.position = Some("0\\0\\0");
    sagittal.orientation = Some(SAGITTAL);
    let (first, second) = two_groups(assemble(&[axial, sagittal]));
    assert_eq!(first.order(), SpatialOrder::Projected);
    assert_eq!(second.order(), SpatialOrder::Projected);
    assert_eq!(places(&first), vec![(0, 0)]);
    assert_eq!(places(&second), vec![(1, 0)]);
}

#[test]
fn series_open_equal_projection_keeps_temporal_order() {
    // Same z. Temporal index 2 has the smaller instance number, so an instance
    // sort would emit it first. Both frames stay.
    let mut later = slice("2.25.21");
    later.position = Some("0\\0\\0");
    later.instance = Some("1");
    later.temporal = Some(2);
    let mut earlier = slice("2.25.20");
    earlier.position = Some("0\\0\\0");
    earlier.instance = Some("9");
    earlier.temporal = Some(1);
    let group = one_group(assemble(&[later, earlier]));
    assert_eq!(slot_lengths(&group), vec![2]);
    assert_eq!(places(&group), vec![(1, 0), (0, 0)]);
}

#[test]
fn series_open_stack_id_orders_one_slot() {
    let mut later = slice("2.25.21");
    later.position = Some("0\\0\\0");
    later.instance = Some("1");
    later.stack_id = Some("B");
    let mut earlier = slice("2.25.20");
    earlier.position = Some("0\\0\\0");
    earlier.instance = Some("9");
    earlier.stack_id = Some("A");
    let group = one_group(assemble(&[later, earlier]));
    assert_eq!(places(&group), vec![(1, 0), (0, 0)]);
}

#[test]
fn series_open_in_stack_position_orders_one_slot() {
    let mut later = slice("2.25.21");
    later.position = Some("0\\0\\0");
    later.instance = Some("1");
    later.stack_id = Some("S");
    later.in_stack = Some(2);
    let mut earlier = slice("2.25.20");
    earlier.position = Some("0\\0\\0");
    earlier.instance = Some("9");
    earlier.stack_id = Some("S");
    earlier.in_stack = Some(1);
    let group = one_group(assemble(&[later, earlier]));
    assert_eq!(places(&group), vec![(1, 0), (0, 0)]);
}

#[test]
fn series_open_missing_position_orders_by_instance_number() {
    let mut later = slice("2.25.21");
    later.instance = Some("2");
    later.position = None;
    later.orientation = None;
    let mut earlier = slice("2.25.20");
    earlier.instance = Some("1");
    earlier.position = None;
    earlier.orientation = None;
    let group = one_group(assemble(&[later, earlier]));
    assert_eq!(group.order(), SpatialOrder::InstanceNumber);
    assert_eq!(places(&group), vec![(1, 0), (0, 0)]);
    assert_eq!(group.frame_count(), 2);
}

#[test]
fn series_open_frame_time_100_is_loop() {
    let mut part = slice("2.25.20");
    part.position = Some("0\\0\\0");
    part.frame_time = Some("100");
    let group = one_group(assemble(&[part]));
    let CineTiming::FrameTime { milliseconds } = group.timing() else {
        unreachable!("frame time is the only present source")
    };
    assert_eq!(milliseconds.to_bits(), 100.0_f64.to_bits());
    assert_eq!(group.playback(), Playback::Loop);
}

#[test]
fn series_open_sweep_is_playback() {
    // PS3.6 (0018,1244) is US. PS3.3 C.7.6.5 value 1 is sweeping.
    let mut part = slice("2.25.20");
    part.position = Some("0\\0\\0");
    part.playback = Some(1);
    let group = one_group(assemble(&[part]));
    assert_eq!(group.playback(), Playback::Sweep);
    assert_eq!(group.timing(), CineTiming::Unavailable);
}

#[test]
fn series_open_playback_zero_is_loop() {
    let mut part = slice("2.25.20");
    part.position = Some("0\\0\\0");
    part.playback = Some(0);
    let group = one_group(assemble(&[part]));
    assert_eq!(group.playback(), Playback::Loop);
}

#[test]
fn series_open_playback_two_is_an_error() {
    let mut part = slice("2.25.20");
    part.position = Some("0\\0\\0");
    part.playback = Some(2);
    expect_reason(assemble(&[part]), SeriesOpenReason::Playback);
}

#[test]
fn series_open_playback_code_string_is_an_error() {
    let mut part = slice("2.25.20");
    part.position = Some("0\\0\\0");
    part.playback_cs = Some("SWEEP");
    expect_reason(assemble(&[part]), SeriesOpenReason::Playback);
}

#[test]
fn series_open_frame_time_zero_does_not_use_the_rate() {
    let mut part = slice("2.25.20");
    part.position = Some("0\\0\\0");
    part.frame_time = Some("0");
    part.recommended_rate = Some("10");
    expect_reason(assemble(&[part]), SeriesOpenReason::Cine);
}

#[test]
fn series_open_later_frame_does_not_supply_cine() {
    let mut first = slice("2.25.20");
    first.instance = Some("2");
    first.position = Some("0\\0\\0");
    first.frame_time = Some("100");
    let mut second = slice("2.25.21");
    second.instance = Some("1");
    second.position = Some("0\\0\\3");
    second.recommended_rate = Some("30");
    second.playback = Some(1);
    let group = one_group(assemble(&[second, first]));
    assert_eq!(places(&group), vec![(1, 0), (0, 0)]);
    let CineTiming::FrameTime { milliseconds } = group.timing() else {
        unreachable!("the first displayed frame carries frame time")
    };
    assert_eq!(milliseconds.to_bits(), 100.0_f64.to_bits());
    assert_eq!(group.playback(), Playback::Loop);
}

#[test]
fn series_open_vector_wins_when_its_length_matches() {
    let mut first = slice("2.25.20");
    first.instance = Some("2");
    first.position = Some("0\\0\\0");
    first.frame_time_vector = Some("40\\60");
    let mut second = slice("2.25.21");
    second.instance = Some("1");
    second.position = Some("0\\0\\3");
    second.frame_time = Some("100");
    let group = one_group(assemble(&[second, first]));
    assert_eq!(group.timing(), CineTiming::FrameTimeVector);
}

#[test]
fn series_open_wrong_vector_length_falls_through() {
    let mut first = slice("2.25.20");
    first.instance = Some("2");
    first.position = Some("0\\0\\0");
    first.frame_time_vector = Some("40");
    first.frame_time = Some("100");
    let mut second = slice("2.25.21");
    second.instance = Some("1");
    second.position = Some("0\\0\\3");
    let group = one_group(assemble(&[second, first]));
    let CineTiming::FrameTime { milliseconds } = group.timing() else {
        unreachable!("a short vector is skipped")
    };
    assert_eq!(milliseconds.to_bits(), 100.0_f64.to_bits());
}

#[test]
fn series_open_vector_leading_zero_is_the_vector() {
    // PS3.3 C.7.6.5.1.2. The first increment is 0. Frame Time 100 must not win.
    let mut first = slice("2.25.20");
    first.instance = Some("2");
    first.position = Some("0\\0\\0");
    first.frame_time_vector = Some("0\\60");
    first.frame_time = Some("100");
    let mut second = slice("2.25.21");
    second.instance = Some("1");
    second.position = Some("0\\0\\3");
    let group = one_group(assemble(&[second, first]));
    assert_eq!(group.timing(), CineTiming::FrameTimeVector);
}

#[test]
fn series_open_vector_zero_does_not_fall_through() {
    // A later zero is an interval. Frame Time 100 must not replace it.
    let mut first = slice("2.25.20");
    first.position = Some("0\\0\\0");
    first.frame_time_vector = Some("40\\0");
    first.frame_time = Some("100");
    let mut second = slice("2.25.21");
    second.position = Some("0\\0\\3");
    expect_reason(assemble(&[first, second]), SeriesOpenReason::Cine);
}

#[test]
fn series_open_shear_is_reported() {
    // Step (5, 0, 3) mm. The off-axis length is 5 mm, above the geometry tolerance.
    let mut first = slice("2.25.20");
    first.position = Some("0\\0\\0");
    let mut second = slice("2.25.21");
    second.position = Some("5\\0\\3");
    let group = one_group(assemble(&[first, second]));
    assert!(group.sheared());
    assert!(!group.non_uniform_step());
    assert_eq!(places(&group), vec![(0, 0), (1, 0)]);
}

#[test]
fn series_open_non_uniform_step_is_reported() {
    // Projected steps are 1 mm then 3 mm. Instance numbers run the other way.
    let mut far = slice("2.25.23");
    far.instance = Some("1");
    far.position = Some("0\\0\\4");
    let mut origin = slice("2.25.20");
    origin.instance = Some("3");
    origin.position = Some("0\\0\\0");
    let mut middle = slice("2.25.21");
    middle.instance = Some("2");
    middle.position = Some("0\\0\\1");
    let group = one_group(assemble(&[far, origin, middle]));
    assert!(!group.sheared());
    assert!(group.non_uniform_step());
    assert_eq!(places(&group), vec![(1, 0), (2, 0), (0, 0)]);
}

#[test]
fn series_open_distinct_series_do_not_join() {
    let mut first = slice("2.25.20");
    first.position = Some("0\\0\\0");
    let mut second = slice("2.25.21");
    second.position = Some("0\\0\\3");
    second.series = "2.25.12";
    let (left, right) = two_groups(assemble(&[first, second]));
    assert_eq!(left.frame_count(), 1);
    assert_eq!(right.frame_count(), 1);
}

#[test]
fn series_open_missing_frame_of_reference_does_not_join_a_present_one() {
    let mut absent = slice("2.25.20");
    absent.position = Some("0\\0\\0");
    absent.frame_of_reference = None;
    let mut present = slice("2.25.21");
    present.position = Some("0\\0\\3");
    let (left, right) = two_groups(assemble(&[absent, present]));
    assert_eq!(left.frame_count(), 1);
    assert_eq!(right.frame_count(), 1);
}

#[test]
fn series_open_missing_series_uid_does_not_join_a_present_one() {
    let mut absent = slice("2.25.20");
    absent.position = Some("0\\0\\0");
    absent.series = "";
    let mut present = slice("2.25.21");
    present.position = Some("0\\0\\3");
    let (left, right) = two_groups(assemble(&[absent, present]));
    assert_eq!(left.frame_count(), 1);
    assert_eq!(right.frame_count(), 1);
}

#[test]
fn series_open_missing_frame_of_reference_uids_do_not_join() {
    let mut first = slice("2.25.20");
    first.position = Some("0\\0\\0");
    first.frame_of_reference = None;
    let mut second = slice("2.25.21");
    second.position = Some("0\\0\\3");
    second.frame_of_reference = None;
    let (left, right) = two_groups(assemble(&[first, second]));
    assert_eq!(left.frame_count(), 1);
    assert_eq!(right.frame_count(), 1);
}

#[test]
fn series_open_spacing_difference_is_two_groups() {
    let mut first = slice("2.25.20");
    first.position = Some("0\\0\\0");
    let mut second = slice("2.25.21");
    second.position = Some("0\\0\\3");
    second.spacing = "2\\2";
    let (left, right) = two_groups(assemble(&[first, second]));
    assert_eq!(left.frame_count(), 1);
    assert_eq!(right.frame_count(), 1);
}

#[test]
fn series_open_in_plane_rotation_is_not_one_group() {
    // Row (0,1,0) and column (-1,0,0) share the axial normal and are not the
    // same orientation. Comparing normals alone would merge them.
    let mut axial = slice("2.25.20");
    axial.position = Some("0\\0\\0");
    let mut rotated = slice("2.25.21");
    rotated.position = Some("0\\0\\3");
    rotated.orientation = Some(ROTATED);
    let (left, right) = two_groups(assemble(&[axial, rotated]));
    assert_eq!(left.frame_count(), 1);
    assert_eq!(right.frame_count(), 1);
}

#[test]
fn series_open_unpositioned_frame_joins_one_orientation_group() {
    let mut positioned = slice("2.25.20");
    positioned.instance = Some("1");
    positioned.position = Some("0\\0\\0");
    let mut loose = slice("2.25.21");
    loose.instance = Some("2");
    loose.position = None;
    let group = one_group(assemble(&[positioned, loose]));
    assert_eq!(group.order(), SpatialOrder::Projected);
    assert_eq!(places(&group), vec![(0, 0), (1, 0)]);
}

#[test]
fn series_open_unpositioned_frame_does_not_pick_an_orientation() {
    let mut axial = slice("2.25.20");
    axial.position = Some("0\\0\\0");
    let mut sagittal = slice("2.25.22");
    sagittal.position = Some("0\\0\\0");
    sagittal.orientation = Some(SAGITTAL);
    let mut loose = slice("2.25.21");
    loose.position = None;
    let result = assemble(&[axial, sagittal, loose]);
    assert!(result.is_ok(), "{result:?}");
    let Ok(groups) = result else {
        unreachable!("asserted immediately above")
    };
    assert_eq!(groups.len(), 3);
    let mut found = false;
    for group in &groups {
        if group.order() == SpatialOrder::InstanceNumber {
            assert_eq!(places(group), vec![(2, 0)]);
            found = true;
        } else {
            assert_eq!(group.frame_count(), 1);
        }
    }
    assert!(found);
}

#[test]
fn series_open_unpositioned_malformed_orientation_is_an_error() {
    // A zero column is not a direction cosine. Missing position must not hide it.
    let mut part = slice("2.25.20");
    part.position = None;
    part.orientation = Some("1\\0\\0\\0\\0\\0");
    expect_reason(assemble(&[part]), SeriesOpenReason::Geometry);
}

#[test]
fn series_open_unpositioned_negative_spacing_is_an_error() {
    let mut part = slice("2.25.20");
    part.position = None;
    part.spacing = "-1\\1";
    expect_reason(assemble(&[part]), SeriesOpenReason::Geometry);
}

#[test]
fn series_open_unpositioned_unreadable_spacing_is_an_error() {
    let mut part = slice("2.25.20");
    part.position = None;
    part.spacing = "abc\\1";
    expect_reason(assemble(&[part]), SeriesOpenReason::Geometry);
}

#[test]
fn series_open_one_object_without_frame_of_reference_stays_one_group() {
    let parsed = enhanced_parsed(false, true);
    let frames = [ParsedFrame::new(&parsed, 0), ParsedFrame::new(&parsed, 1)];
    let group = one_group(assemble_display(&frames));
    assert_eq!(group.frame_count(), 2);
    assert_eq!(places(&group), vec![(1, 1), (0, 0)]);
}

#[test]
fn series_open_unpositioned_object_without_frame_of_reference_stays_one_group() {
    let parsed = enhanced_parsed(false, false);
    let frames = [ParsedFrame::new(&parsed, 0), ParsedFrame::new(&parsed, 1)];
    let group = one_group(assemble_display(&frames));
    assert_eq!(group.order(), SpatialOrder::InstanceNumber);
    assert_eq!(group.frame_count(), 2);
    assert_eq!(places(&group), vec![(0, 0), (1, 1)]);
}

#[test]
fn series_open_per_frame_position_orders_inside_one_object() {
    // Frame 0 stores z = 3 and frame 1 stores z = 0. There is no top-level
    // Image Position Patient, so ignoring the frame index would drop the order.
    let parsed = enhanced_pair();
    let frames = [ParsedFrame::new(&parsed, 0), ParsedFrame::new(&parsed, 1)];
    let result = assemble_display(&frames);
    let group = one_group(result);
    assert_eq!(group.order(), SpatialOrder::Projected);
    assert_eq!(places(&group), vec![(1, 1), (0, 0)]);
}

#[test]
fn series_open_frame_index_past_the_end_is_an_error() {
    let mut part = slice("2.25.20");
    part.position = Some("0\\0\\0");
    let parsed = parse_slice(&part);
    let frames = [ParsedFrame::new(&parsed, 4)];
    let result = assemble_display(&frames);
    assert!(result.is_err(), "{result:?}");
    let Err(error) = result else {
        unreachable!("asserted immediately above")
    };
    assert_eq!(error.reason(), SeriesOpenReason::FrameOutOfRange);
    assert_eq!(error.index(), 0);
}

/// One object's timing alone, through the public projection.
fn object_timing(part: &Slice) -> Result<CineTiming, SeriesOpenError> {
    let parsed = parse_slice(part);
    let metadata = MetadataSet::from_object(parsed.object());
    assert!(metadata.is_ok(), "the fixture projects");
    let Ok(metadata) = metadata else {
        unreachable!("asserted immediately above")
    };
    cine_timing(&metadata)
}

/// Every frame of one object, in frame order.
fn assemble_object(part: &Slice, frames: u32) -> Result<Vec<DisplaySeries>, SeriesOpenError> {
    let parsed = parse_slice(part);
    let list: Vec<ParsedFrame<'_>> = (0..frames)
        .map(|frame| ParsedFrame::new(&parsed, frame))
        .collect();
    assemble_display(&list)
}

/// One object with every timing source this fixture set varies.
fn timed(
    frames: Option<&'static str>,
    time: Option<&'static str>,
    vector: Option<&'static str>,
    rate: Option<&'static str>,
    cine: Option<&'static str>,
) -> Slice {
    let mut part = slice("2.25.40");
    part.position = Some("0\\0\\0");
    part.frames = frames;
    part.frame_time = time;
    part.frame_time_vector = vector;
    part.recommended_rate = rate;
    part.cine_rate = cine;
    part
}

#[test]
fn cine_timing_matches_display_timing() {
    // F-214 AC9. Each fixture is one object, so assembly's own length check
    // and the object's declared frame count are the same number. Frame Time
    // is PS3.3 C.7.6.5 in milliseconds and the vector is C.7.6.5.1.2.
    let cases: [(Slice, u32); 9] = [
        (timed(None, Some("100"), None, None, None), 1),
        (timed(Some("2"), Some("40"), None, None, None), 2),
        (timed(Some("2"), Some("33.3"), None, Some("10"), None), 2),
        (timed(Some("2"), None, Some("0\\40"), None, None), 2),
        (timed(Some("2"), Some("100"), Some("0\\40"), None, None), 2),
        // A vector of three values on two frames is skipped, so Frame Time wins.
        (
            timed(Some("2"), Some("100"), Some("0\\40\\40"), None, None),
            2,
        ),
        (timed(Some("2"), None, None, Some("25"), None), 2),
        (timed(Some("2"), None, None, None, Some("30")), 2),
        (timed(Some("2"), None, None, None, None), 2),
    ];
    for (part, frames) in &cases {
        let group = one_group(assemble_object(part, *frames));
        assert_eq!(object_timing(part), Ok(group.timing()));
    }
    // The refusals agree too. A later vector component of zero and a Frame
    // Time of zero on two frames do not fall through to Frame Time or a rate.
    for part in [
        timed(Some("2"), Some("100"), Some("40\\0"), None, None),
        timed(Some("2"), Some("0"), None, Some("10"), None),
    ] {
        let assembled = assemble_object(&part, 2).map(|_| ());
        let alone = object_timing(&part).map(|_| ());
        assert_eq!(
            alone.map_err(SeriesOpenError::reason),
            Err(SeriesOpenReason::Cine)
        );
        assert_eq!(
            assembled.map_err(SeriesOpenError::reason),
            Err(SeriesOpenReason::Cine)
        );
    }
}

#[test]
fn cine_timing_reads_frame_time_40_in_milliseconds() {
    // PS3.3 C.7.6.5. Frame Time is DS in milliseconds, so 40 is 40 ms and
    // 1000 / 40 is 25 frames per second. It is not a rate.
    let part = timed(Some("2"), Some("40"), None, Some("10"), Some("30"));
    let Ok(CineTiming::FrameTime { milliseconds }) = object_timing(&part) else {
        unreachable!("Frame Time comes before either rate")
    };
    assert_eq!(milliseconds.to_bits(), 40.0_f64.to_bits());
}

#[test]
fn cine_timing_ignores_a_bad_playback_value() {
    // F-214 amendment 2. Preferred Playback Sequencing 2 is not 0 or 1,
    // PS3.3 C.7.6.5, so assembly refuses the object. The timing alone is
    // still Frame Time 40.
    let mut part = timed(Some("2"), Some("40"), None, None, None);
    part.playback = Some(2);
    expect_reason(assemble_object(&part, 2), SeriesOpenReason::Playback);
    let Ok(CineTiming::FrameTime { milliseconds }) = object_timing(&part) else {
        unreachable!("the playback value does not hide Frame Time")
    };
    assert_eq!(milliseconds.to_bits(), 40.0_f64.to_bits());
}

#[test]
fn cine_timing_single_frame_zero_is_unavailable() {
    // F-214 amendment 4. One frame has no interval between frames, so a
    // Frame Time of 0 there is not refused. It is not replaced by the rate
    // either, because Frame Time was present.
    let alone = timed(None, Some("0"), None, None, None);
    assert_eq!(object_timing(&alone), Ok(CineTiming::Unavailable));
    let declared = timed(Some("1"), Some("0"), None, Some("10"), None);
    assert_eq!(object_timing(&declared), Ok(CineTiming::Unavailable));
    // A negative interval is never legal, PS3.3 C.7.6.5 gives Frame Time as
    // a time in milliseconds, so one frame of -40 stays the Cine refusal.
    let negative = timed(None, Some("-40"), None, None, None);
    assert_eq!(
        object_timing(&negative).map_err(SeriesOpenError::reason),
        Err(SeriesOpenReason::Cine)
    );
    // Two frames with Frame Time 0 stay unusable.
    let two = timed(Some("2"), Some("0"), None, None, None);
    assert_eq!(
        object_timing(&two).map_err(SeriesOpenError::reason),
        Err(SeriesOpenReason::Cine)
    );
    // A one-value vector on one frame is still the vector, which comes first.
    let vector = timed(None, Some("0"), Some("0"), None, None);
    assert_eq!(object_timing(&vector), Ok(CineTiming::FrameTimeVector));
}

fn enhanced_pair() -> ParsedDicom {
    enhanced_parsed(true, true)
}

fn enhanced_parsed(frame_of_reference: bool, positions: bool) -> ParsedDicom {
    let built = enhanced_object(frame_of_reference, positions);
    assert!(built.is_ok(), "enhanced fixture");
    let Ok(built) = built else {
        unreachable!("asserted immediately above")
    };
    let parsed = parse_part10(&built);
    assert!(parsed.is_ok(), "{parsed:?}");
    let Ok(parsed) = parsed else {
        unreachable!("asserted immediately above")
    };
    parsed
}

fn enhanced_object(frame_of_reference: bool, positions: bool) -> Result<Vec<u8>, &'static str> {
    let orientation = sequence((0x0020, 0x9116), &[ds((0x0020, 0x0037), AXIAL)?])?;
    let measures = sequence((0x0028, 0x9110), &[ds((0x0028, 0x0030), "1\\1")?])?;
    let mut shared_item = orientation;
    shared_item.extend(measures);
    let shared = sequence((0x5200, 0x9229), &[shared_item])?;
    let mut data = Vec::new();
    data.extend(ui_element((0x0008, 0x0016), "1.2.840.10008.5.1.4.1.1.2")?);
    data.extend(ui_element((0x0008, 0x0018), "2.25.30")?);
    data.extend(text_element((0x0020, 0x0013), *b"IS", "1")?);
    data.extend(ui_element((0x0020, 0x000E), SERIES)?);
    if frame_of_reference {
        data.extend(ui_element((0x0020, 0x0052), FRAME_OF_REFERENCE)?);
    }
    data.extend(shared);
    if positions {
        let frame_late = sequence((0x0020, 0x9113), &[ds((0x0020, 0x0032), "0\\0\\3")?])?;
        let frame_early = sequence((0x0020, 0x9113), &[ds((0x0020, 0x0032), "0\\0\\0")?])?;
        data.extend(sequence((0x5200, 0x9230), &[frame_late, frame_early])?);
    }
    data.extend(text_element((0x0028, 0x0008), *b"IS", "2")?);
    data.extend(short_le((0x0028, 0x0010), *b"US", &1_u16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0011), *b"US", &1_u16.to_le_bytes())?);
    part10(data)
}

fn encode(part: &Slice) -> Result<Vec<u8>, &'static str> {
    let mut data = Vec::new();
    data.extend(ui_element((0x0008, 0x0016), "1.2.840.10008.5.1.4.1.1.2")?);
    data.extend(ui_element((0x0008, 0x0018), part.sop)?);
    if let Some(rate) = part.recommended_rate {
        data.extend(text_element((0x0008, 0x2144), *b"IS", rate)?);
    }
    if let Some(rate) = part.cine_rate {
        data.extend(text_element((0x0018, 0x0040), *b"IS", rate)?);
    }
    if let Some(time) = part.frame_time {
        data.extend(text_element((0x0018, 0x1063), *b"DS", time)?);
    }
    if let Some(vector) = part.frame_time_vector {
        data.extend(text_element((0x0018, 0x1065), *b"DS", vector)?);
    }
    if let Some(playback) = part.playback {
        data.extend(short_le((0x0018, 0x1244), *b"US", &playback.to_le_bytes())?);
    }
    if let Some(playback) = part.playback_cs {
        data.extend(text_element((0x0018, 0x1244), *b"CS", playback)?);
    }
    if !part.series.is_empty() {
        data.extend(ui_element((0x0020, 0x000E), part.series)?);
    }
    if let Some(instance) = part.instance {
        data.extend(text_element((0x0020, 0x0013), *b"IS", instance)?);
    }
    if let Some(position) = part.position {
        data.extend(text_element((0x0020, 0x0032), *b"DS", position)?);
    }
    if let Some(orientation) = part.orientation {
        data.extend(text_element((0x0020, 0x0037), *b"DS", orientation)?);
    }
    if let Some(frame_of_reference) = part.frame_of_reference {
        data.extend(ui_element((0x0020, 0x0052), frame_of_reference)?);
    }
    if let Some(stack_id) = part.stack_id {
        data.extend(text_element((0x0020, 0x9056), *b"SH", stack_id)?);
    }
    if let Some(in_stack) = part.in_stack {
        data.extend(short_le((0x0020, 0x9057), *b"UL", &in_stack.to_le_bytes())?);
    }
    if let Some(temporal) = part.temporal {
        data.extend(short_le((0x0020, 0x9128), *b"UL", &temporal.to_le_bytes())?);
    }
    if let Some(frames) = part.frames {
        data.extend(text_element((0x0028, 0x0008), *b"IS", frames)?);
    }
    data.extend(short_le((0x0028, 0x0010), *b"US", &1_u16.to_le_bytes())?);
    data.extend(short_le((0x0028, 0x0011), *b"US", &1_u16.to_le_bytes())?);
    data.extend(text_element((0x0028, 0x0030), *b"DS", part.spacing)?);
    part10(data)
}

fn ui_element(tag: (u16, u16), text: &str) -> Result<Vec<u8>, &'static str> {
    short_le(tag, *b"UI", &ui(text.as_bytes()))
}

fn text_element(tag: (u16, u16), vr: [u8; 2], text: &str) -> Result<Vec<u8>, &'static str> {
    short_le(tag, vr, &text_pad(text))
}

fn ds(tag: (u16, u16), text: &str) -> Result<Vec<u8>, &'static str> {
    text_element(tag, *b"DS", text)
}

fn sequence(tag: (u16, u16), items: &[Vec<u8>]) -> Result<Vec<u8>, &'static str> {
    let mut payload = Vec::new();
    for item in items {
        payload.extend(item_bytes(item)?);
    }
    long_le(tag, *b"SQ", &payload)
}

fn item_bytes(data: &[u8]) -> Result<Vec<u8>, &'static str> {
    let length = u32::try_from(data.len()).map_err(|_| "item length")?;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&0xFFFE_u16.to_le_bytes());
    bytes.extend_from_slice(&0xE000_u16.to_le_bytes());
    bytes.extend_from_slice(&length.to_le_bytes());
    bytes.extend_from_slice(data);
    Ok(bytes)
}

fn part10(data: Vec<u8>) -> Result<Vec<u8>, &'static str> {
    let mut bytes = vec![0_u8; 128];
    bytes.extend_from_slice(b"DICM");
    bytes.extend(file_meta()?);
    bytes.extend(data);
    Ok(bytes)
}

fn file_meta() -> Result<Vec<u8>, &'static str> {
    let mut body = Vec::new();
    body.extend(long_le((0x0002, 0x0001), *b"OB", &[0, 1])?);
    body.extend(ui_element((0x0002, 0x0002), "1.2.840.10008.5.1.4.1.1.2")?);
    body.extend(ui_element((0x0002, 0x0003), "2.25.1")?);
    body.extend(ui_element((0x0002, 0x0010), "1.2.840.10008.1.2.1")?);
    body.extend(ui_element((0x0002, 0x0012), "2.25.2")?);
    let length = u32::try_from(body.len()).map_err(|_| "meta length")?;
    let mut meta = short_le((0x0002, 0x0000), *b"UL", &length.to_le_bytes())?;
    meta.extend(body);
    Ok(meta)
}

fn ui(text: &[u8]) -> Vec<u8> {
    let mut value = text.to_vec();
    if !value.len().is_multiple_of(2) {
        value.push(0);
    }
    value
}

fn text_pad(text: &str) -> Vec<u8> {
    let mut value = text.as_bytes().to_vec();
    if !value.len().is_multiple_of(2) {
        value.push(b' ');
    }
    value
}

fn short_le(tag: (u16, u16), vr: [u8; 2], value: &[u8]) -> Result<Vec<u8>, &'static str> {
    if !value.len().is_multiple_of(2) {
        return Err("odd element");
    }
    let length = u16::try_from(value.len()).map_err(|_| "element length")?;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&tag.0.to_le_bytes());
    bytes.extend_from_slice(&tag.1.to_le_bytes());
    bytes.extend_from_slice(&vr);
    bytes.extend_from_slice(&length.to_le_bytes());
    bytes.extend_from_slice(value);
    Ok(bytes)
}

fn long_le(tag: (u16, u16), vr: [u8; 2], value: &[u8]) -> Result<Vec<u8>, &'static str> {
    if !value.len().is_multiple_of(2) {
        return Err("odd element");
    }
    let length = u32::try_from(value.len()).map_err(|_| "element length")?;
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&tag.0.to_le_bytes());
    bytes.extend_from_slice(&tag.1.to_le_bytes());
    bytes.extend_from_slice(&vr);
    bytes.extend_from_slice(&[0, 0]);
    bytes.extend_from_slice(&length.to_le_bytes());
    bytes.extend_from_slice(value);
    Ok(bytes)
}
