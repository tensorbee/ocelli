//! F-053. An oblique plane is displayed by the existing orthographic upload.
//!
//! PS3.3 C.7.6.2.1.1. The host sample is the identity mix: pixel `(0, 0)` is
//! `Present(95)` and pixel `(1, 0)` is `Absent`. Tier C refuses before the
//! upload closure runs. No adapter is opened.

use glam::{DMat3, DVec3};
use ocelli_core::{Pt, World};
use ocelli_render::{OrthoError, Tier, present_plane};
use ocelli_volume::{
    Level, ObliquePlane, OrthoImage, PlaneSample, SlicePresence, Volume, VoxelKind, VoxelStore,
};

#[test]
fn oblique_display_reuses_present_plane() {
    let volume = identity_volume();
    let mixed = volume.reslice_oblique(mix_plane());
    let Ok(mixed) = mixed else {
        unreachable!("the mix plane is valid on the identity volume")
    };
    assert_eq!(present_bits(at(&mixed, 0, 0)), 95.0_f32.to_bits());
    assert!(matches!(at(&mixed, 1, 0), PlaneSample::Absent));

    let mut calls = 0_u32;
    let refused = present_plane(Tier::Cpu, &mixed, |_| {
        calls = calls.saturating_add(1);
        Ok(())
    });
    assert!(matches!(refused, Err(OrthoError::TierCUnavailable)));
    assert_eq!(calls, 0);

    let one = OrthoImage {
        width: 1,
        height: 1,
        column_mm: 1.0,
        row_mm: 1.0,
        samples: vec![PlaneSample::Present(95.0)],
    };
    let refused = present_plane(Tier::Cpu, &one, |_| {
        calls = calls.saturating_add(1);
        Ok(())
    });
    assert!(matches!(refused, Err(OrthoError::TierCUnavailable)));
    assert_eq!(calls, 0);

    let mut seen = Vec::new();
    let uploaded = present_plane(Tier::A, &one, |samples| {
        seen = samples.to_vec();
        Ok(())
    });
    assert!(uploaded.is_ok());
    assert_eq!(
        seen.iter().copied().map(f32::to_bits).collect::<Vec<_>>(),
        [95.0_f32].map(f32::to_bits)
    );

    let refused = present_plane(Tier::A, &mixed, |_| {
        calls = calls.saturating_add(1);
        Ok(())
    });
    assert!(matches!(refused, Err(OrthoError::PlaneIncomplete)));
    assert_eq!(calls, 0);
}

fn mix_plane() -> ObliquePlane {
    ObliquePlane {
        origin: Pt::<World>::new(0.5, 0.5, 0.5),
        row: DVec3::new(1.0, 0.0, 0.0),
        column: DVec3::new(0.0, 0.0, 1.0),
        columns: 2,
        rows: 1,
        column_mm: 1.0,
        row_mm: 1.0,
    }
}

fn at(image: &OrthoImage, column: u32, row: u32) -> &PlaneSample {
    let width = usize::try_from(image.width).unwrap_or(0);
    let index = usize::try_from(row)
        .unwrap_or(0)
        .saturating_mul(width)
        .saturating_add(usize::try_from(column).unwrap_or(0));
    let Some(sample) = image.samples.get(index) else {
        unreachable!("the fixture pixel is inside the plane")
    };
    sample
}

fn present_bits(sample: &PlaneSample) -> u32 {
    match sample {
        PlaneSample::Present(value) => value.to_bits(),
        PlaneSample::Absent => unreachable!("the fixture sample has landed"),
    }
}

fn identity_volume() -> Volume {
    let mut volume = Volume {
        dims: [2, 2, 2],
        spacing: [1.0, 1.0, 1.0],
        origin: Pt::<World>::new(0.0, 0.0, 0.0),
        direction: DMat3::IDENTITY,
        voxel: VoxelKind::U8,
        data: VoxelStore::new([2, 2, 2], VoxelKind::U8),
        levels: vec![Level {
            dims: [2, 2, 2],
            spacing: [1.0, 1.0, 1.0],
            offset: 0,
        }],
        present: SlicePresence::new(2),
        order: Vec::new(),
    };
    assert!(volume.place(0, &[10, 30, 70, 90]).is_ok());
    assert!(volume.place(1, &[110, 130, 150, 170]).is_ok());
    volume
}
