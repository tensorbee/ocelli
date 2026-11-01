//! F-054 slab step, absence and the one VOI after a stored reduction.
//!
//! PS3.3 C.7.6.2.1.1 places the world point. The step is volume spacing
//! along the unit normal. PS3.3 C.11.2.1.3.2 maps the reduced stored value.
//! The display numbers are the fixture file, not a value copied from this
//! walker.

use glam::{DMat3, DVec3};
use ocelli_core::{Display, Pt, Stored, World};
use ocelli_pixel::{
    LutChain, ModalityTransform, PhotometricInterpretation, PresentationLutEvidence, VoiFunction,
    VoiTransform,
};
use ocelli_volume::{
    Level, OrthoPlane, PlaneSample, SlabImage, SlabPlane, SlicePresence, Volume, VolumeError,
    VoxelKind, VoxelStore,
};

const FIXTURE: &str = include_str!("fixtures/f054_mip_stored_0_200.txt");

#[test]
fn axial_slab_steps_by_volume_spacing() {
    // Spacing `[0.5, 1.0, 2.5]`, origin `(10, 20, 30)`. Stored `(0, 0, k)`
    // is 0, 200, 0. Thickness 5 mm. Half is 2.5 mm. Step is `spacing[2]`.
    // `t` is -1, 0, 1. A step of 5 mm would keep only the centre.
    let volume = line_volume([1, 1, 3], [0.5, 1.0, 2.5], &[0, 200, 0]);
    assert_world(volume.voxel_world([0.0, 0.0, 1.0]), (10.0, 20.0, 32.5));
    let before = slice_bits(&volume, 1);
    let image = sampled(&volume, OrthoPlane::Axial { slice: 1.0 }, 5.0);
    assert_eq!(image.step_mm.to_bits(), 2.5_f64.to_bits());
    assert_eq!(image.index_step[0].to_bits(), 0.0_f64.to_bits());
    assert_eq!(image.index_step[1].to_bits(), 0.0_f64.to_bits());
    assert_eq!(image.index_step[2].to_bits(), 1.0_f64.to_bits());
    assert_eq!(image.sample_count, 3);
    assert_eq!(present_bits(at(&image, 0)), 0.0_f32.to_bits());
    assert_eq!(present_bits(at(&image, 1)), 200.0_f32.to_bits());
    assert_eq!(present_bits(at(&image, 2)), 0.0_f32.to_bits());
    assert_eq!(slice_bits(&volume, 1), before);
}

#[test]
fn sagittal_slab_does_not_use_slice_spacing() {
    // Spacing `[4, 2, 2.5]`. The normal is the first axis, so the step is
    // 4 mm. A walker that used `spacing[2]` would step 2.5 mm and blend.
    let volume = line_volume([3, 1, 1], [4.0, 2.0, 2.5], &[0, 200, 0]);
    let image = sampled(&volume, OrthoPlane::Sagittal { column: 1.0 }, 8.0);
    assert_eq!(image.step_mm.to_bits(), 4.0_f64.to_bits());
    assert_eq!(image.index_step[0].to_bits(), 1.0_f64.to_bits());
    assert_eq!(image.index_step[1].to_bits(), 0.0_f64.to_bits());
    assert_eq!(image.index_step[2].to_bits(), 0.0_f64.to_bits());
    assert_eq!(present_bits(at(&image, 0)), 0.0_f32.to_bits());
    assert_eq!(present_bits(at(&image, 1)), 200.0_f32.to_bits());
    assert_eq!(present_bits(at(&image, 2)), 0.0_f32.to_bits());
}

#[test]
fn oblique_step_follows_the_normal() {
    // Spacing `[1, 1, 4]`. Unit normal `(0, 0.6, 0.8)`.
    // `d = (0, 0.6, 0.2)`. `length(d)` squared is 0.4.
    // Step is `1 / sqrt(0.4)`, not 4.
    let volume = line_volume([1, 1, 1], [1.0, 1.0, 4.0], &[10]);
    let length = 0.4_f64.sqrt();
    let Ok(image) = volume.slab_samples(
        SlabPlane {
            origin_index: [0.0, 0.0, 0.0],
            normal: [0.0, 0.6, 0.8],
            column: [1.0, 0.0, 0.0],
            row: [0.0, 1.0, 0.0],
            width: 1,
            height: 1,
            column_mm: 1.0,
            row_mm: 1.0,
        },
        0.0,
    ) else {
        unreachable!("the unit normal has a step");
    };
    assert_eq!(image.step_mm.to_bits(), (1.0 / length).to_bits());
    assert_eq!(image.index_step[0].to_bits(), 0.0_f64.to_bits());
    assert_eq!(image.index_step[1].to_bits(), (0.6 / length).to_bits());
    assert_eq!(image.index_step[2].to_bits(), (0.2 / length).to_bits());
    assert_ne!(image.step_mm.to_bits(), 4.0_f64.to_bits());
}

#[test]
fn an_absent_slab_sample_is_not_zero() {
    let mut volume = line_volume([1, 1, 3], [0.5, 1.0, 2.5], &[0, 200, 0]);
    volume.present = SlicePresence::new(3);
    assert!(volume.place(1, &[200]).is_ok());
    assert!(volume.place(2, &[0]).is_ok());
    let image = sampled(&volume, OrthoPlane::Axial { slice: 1.0 }, 5.0);
    assert!(
        image
            .samples
            .iter()
            .all(|sample| matches!(sample, PlaneSample::Absent))
    );
}

#[test]
fn composite_is_the_centre_sample() {
    let volume = line_volume([1, 1, 3], [0.5, 1.0, 2.5], &[0, 200, 0]);
    let image = sampled(&volume, OrthoPlane::Axial { slice: 1.0 }, 5.0);
    let stored = [0.0_f32, 200.0, 0.0];
    assert_eq!(present_bits(at(&image, 0)), stored[0].to_bits());
    assert_eq!(present_bits(at(&image, 1)), stored[1].to_bits());
    assert_eq!(present_bits(at(&image, 2)), stored[2].to_bits());
    let display = window_once(stored[1]);
    assert_eq!(display.to_bits(), fixture_number("composite").to_bits());
    assert_eq!(display.to_bits(), 200.0_f32.to_bits());
    let rejected = (200.0_f32 / 255.0) * 0.5 * 0.5;
    assert_ne!(display.to_bits(), rejected.to_bits());
}

#[test]
fn stored_reduction_then_one_voi_matches_the_fixture() {
    // The walker column is stored 0, 200, 0. Max is 200. Min is 0.
    // The mean of that pair is 100. PS3.3 C.11.2.1.3.2 leaves all three
    // inside this window, so the two orders agree on the pair.
    let volume = line_volume([1, 1, 3], [0.5, 1.0, 2.5], &[0, 200, 0]);
    let image = sampled(&volume, OrthoPlane::Axial { slice: 1.0 }, 5.0);
    let side = present_value(at(&image, 0));
    let centre = present_value(at(&image, 1));
    let other = present_value(at(&image, 2));
    let mip = window_once(side.max(centre).max(other));
    let minip = window_once(side.min(centre).min(other));
    let average = window_once((side.max(centre).max(other) + side.min(centre).min(other)) / 2.0);
    assert_eq!(mip.to_bits(), fixture_number("mip").to_bits());
    assert_eq!(minip.to_bits(), fixture_number("minip").to_bits());
    assert_eq!(average.to_bits(), fixture_number("average").to_bits());
    assert_eq!(mip.to_bits(), 200.0_f32.to_bits());
    assert_eq!(minip.to_bits(), 0.0_f32.to_bits());
    assert_eq!(average.to_bits(), 100.0_f32.to_bits());

    // Stored 0, 30, 300. The mean is 110, and one window leaves 110.
    // Window first: 0, the window of 30, and 255. Their mean is 95.
    // 300 does not fit the u8 fixture voxel. These are the stored values
    // the acceptance row windows, and the GPU readback uploads the same three.
    let outside = [0.0_f32, 30.0, 300.0];
    let reduced = (outside[0] + outside[1] + outside[2]) / 3.0;
    assert_eq!(reduced.to_bits(), 110.0_f32.to_bits());
    let once = window_once(reduced);
    let each = (window_once(outside[0]) + window_once(outside[1]) + window_once(outside[2])) / 3.0;
    assert_eq!(once.to_bits(), fixture_number("outside-average").to_bits());
    assert_eq!(
        each.to_bits(),
        fixture_number("outside-voi-then-average").to_bits()
    );
    assert_eq!(once.to_bits(), 110.0_f32.to_bits());
    assert_eq!(each.to_bits(), 95.0_f32.to_bits());
    assert_ne!(once.to_bits(), each.to_bits());
}

#[test]
fn rotated_direction_is_part_of_the_step() {
    // Columns (0, 1, 0), (0, 0, 1), (1, 0, 0). Spacing [2, 4, 8].
    // Patient Z is the second column, so the step is 4 mm and the index
    // step is (0, 1, 0). A diagonal of spacing steps 8 mm on (0, 0, 1).
    let mut volume = line_volume([1, 1, 1], [2.0, 4.0, 8.0], &[10]);
    volume.direction = DMat3::from_cols(
        DVec3::new(0.0, 1.0, 0.0),
        DVec3::new(0.0, 0.0, 1.0),
        DVec3::new(1.0, 0.0, 0.0),
    );
    assert_world(volume.voxel_world([0.0, 0.0, 1.0]), (18.0, 20.0, 30.0));
    let Ok(image) = volume.slab_samples(
        SlabPlane {
            origin_index: [0.0, 0.0, 0.0],
            normal: [0.0, 0.0, 1.0],
            column: [1.0, 0.0, 0.0],
            row: [0.0, 1.0, 0.0],
            width: 1,
            height: 1,
            column_mm: 1.0,
            row_mm: 1.0,
        },
        0.0,
    ) else {
        unreachable!("the rotated normal has a step");
    };
    assert_eq!(image.step_mm.to_bits(), 4.0_f64.to_bits());
    assert_eq!(image.index_step[0].to_bits(), 0.0_f64.to_bits());
    assert_eq!(image.index_step[1].to_bits(), 1.0_f64.to_bits());
    assert_eq!(image.index_step[2].to_bits(), 0.0_f64.to_bits());
    assert_ne!(image.step_mm.to_bits(), 8.0_f64.to_bits());
}

#[test]
fn slab_request_refusals() {
    let volume = line_volume([1, 1, 3], [0.5, 1.0, 2.5], &[0, 200, 0]);
    let Ok(plane) = volume.slab_plane(OrthoPlane::Axial { slice: 1.0 }) else {
        unreachable!("slice 1 is inside");
    };
    assert_eq!(
        volume.slab_samples(plane, f64::NAN),
        Err(VolumeError::SlabThickness)
    );
    assert_eq!(
        volume.slab_samples(plane, -1.0),
        Err(VolumeError::SlabThickness)
    );
    let mut bad_normal = plane;
    bad_normal.normal = [0.0, 0.0, 0.0];
    assert_eq!(
        volume.slab_samples(bad_normal, 5.0),
        Err(VolumeError::SlabNormal)
    );
    assert_eq!(
        volume.slab_plane(OrthoPlane::Axial { slice: 3.0 }),
        Err(VolumeError::PlaneOutOfRange {
            axis: 2,
            index: 3.0
        })
    );
    let centre = sampled(&volume, OrthoPlane::Axial { slice: 1.0 }, 0.0);
    assert_eq!(centre.sample_count, 1);
    assert_eq!(present_bits(at(&centre, 0)), 200.0_f32.to_bits());
    assert_eq!(
        slice_bits(&volume, 1).first().copied(),
        Some(200.0_f32.to_bits())
    );
}

fn sampled(volume: &Volume, plane: OrthoPlane, thickness_mm: f64) -> SlabImage {
    let Ok(frame) = volume.slab_plane(plane) else {
        unreachable!("the fixture plane is inside");
    };
    let Ok(image) = volume.slab_samples(frame, thickness_mm) else {
        unreachable!("the fixture thickness has a step");
    };
    image
}

fn at(image: &SlabImage, index: usize) -> PlaneSample {
    match image.samples.get(index) {
        Some(sample) => *sample,
        None => unreachable!("the fixture column has this slot"),
    }
}

fn present_value(sample: PlaneSample) -> f32 {
    match sample {
        PlaneSample::Present(value) => value,
        PlaneSample::Absent => unreachable!("the fixture sample has landed"),
    }
}

fn present_bits(sample: PlaneSample) -> u32 {
    match sample {
        PlaneSample::Present(value) => value.to_bits(),
        PlaneSample::Absent => unreachable!("the fixture sample has landed"),
    }
}

fn window_once(stored: f32) -> f32 {
    let Ok(modality) = ModalityTransform::new(None, Some(1.0), Some(0.0)) else {
        unreachable!("slope 1 is a modality");
    };
    let Ok(voi) = VoiTransform::new(
        None,
        &[127.5],
        &[255.0],
        0,
        VoiFunction::LinearExact,
        0.0,
        255.0,
    ) else {
        unreachable!("LINEAR_EXACT width 255 is a window");
    };
    let Ok(chain) = LutChain::new(
        modality,
        voi,
        PhotometricInterpretation::Monochrome2,
        PresentationLutEvidence::Absent,
    ) else {
        unreachable!("identity presentation builds");
    };
    let Display(display) = chain.apply(Stored(stored));
    display
}

fn fixture_number(label: &str) -> f32 {
    let Some(line) = FIXTURE
        .lines()
        .find(|line| line.starts_with(label) && line.as_bytes().get(label.len()) == Some(&b' '))
    else {
        unreachable!("the fixture names the mode");
    };
    let Some(text) = line.split(' ').nth(1) else {
        unreachable!("the fixture has a number");
    };
    let Ok(value) = text.parse::<f32>() else {
        unreachable!("the fixture number is a display code");
    };
    value
}

fn assert_world(point: Pt<World>, expected: (f64, f64, f64)) {
    assert_eq!(point.x.to_bits(), expected.0.to_bits());
    assert_eq!(point.y.to_bits(), expected.1.to_bits());
    assert_eq!(point.z.to_bits(), expected.2.to_bits());
}

fn slice_bits(volume: &Volume, index: u32) -> Vec<u32> {
    let Ok(values) = volume.present_slice_f32(index) else {
        unreachable!("the fixture slice was placed");
    };
    values.iter().copied().map(f32::to_bits).collect()
}

fn line_volume(dims: [u32; 3], spacing: [f64; 3], bytes: &[u8]) -> Volume {
    let mut volume = Volume {
        dims,
        spacing,
        origin: Pt::<World>::new(10.0, 20.0, 30.0),
        direction: DMat3::IDENTITY,
        voxel: VoxelKind::U8,
        data: VoxelStore::new(dims, VoxelKind::U8),
        levels: vec![Level {
            dims,
            spacing,
            offset: 0,
        }],
        present: SlicePresence::new(dims[2]),
        order: Vec::new(),
    };
    let mut offset = 0_usize;
    let mut slice = 0_u32;
    let Ok(width) = usize::try_from(dims[0]) else {
        unreachable!("width fits");
    };
    let Ok(height) = usize::try_from(dims[1]) else {
        unreachable!("height fits");
    };
    let slice_len = width.saturating_mul(height);
    while offset < bytes.len() {
        let end = offset.saturating_add(slice_len);
        let Some(chunk) = bytes.get(offset..end) else {
            unreachable!("the fixture bytes are whole slices");
        };
        assert!(volume.place(slice, chunk).is_ok());
        offset = end;
        slice = slice.saturating_add(1);
    }
    volume
}
