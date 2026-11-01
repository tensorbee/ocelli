//! F-X004. Tier C presents the host reslice through `CpuRaster`.
//!
//! PS3.3 C.7.6.2.1.1. Identity volume, spacing `(1, 1, 1)`, origin `(0, 0, 0)`,
//! direction identity, `U8`. Slice 0 is `[10, 30, 70, 90]`. Slice 1 is
//! `[110, 130, 150, 170]`. The sum is 760. The mix point `(0.5, 0.5, 0.5)`
//! is `760 / 8 = 95`. Nearest of that point is 170. A y = 0 snap is 70.
//! These literals are the samples. Display bytes are only the check that
//! `present_cpu_plane` handed those samples to the existing raster.

use glam::{DMat3, DVec3};
use ocelli_core::{Pt, Stored, World};
use ocelli_pixel::{
    LutChain, ModalityTransform, PhotometricInterpretation, PresentationLutEvidence, VoiFunction,
    VoiTransform,
};
use ocelli_render::{
    CpuRaster, OrthoError, RasterError, RasterGrid, RasterQuality, copy_plane_stored,
    present_cpu_plane,
};
use ocelli_viewport::presentation::{Colormap, Presentation};
use ocelli_viewport::view::View;
use ocelli_volume::{
    Level, ObliquePlane, OrthoImage, OrthoPlane, PlaneSample, SlicePresence, Volume, VoxelKind,
    VoxelStore,
};

#[test]
fn cpu_plane_keeps_the_identity_samples() {
    let volume = identity_volume();
    let axial = resliced(&volume, OrthoPlane::Axial { slice: 0.0 });
    assert_copied(&axial, &[10.0, 30.0, 70.0, 90.0]);
    let upper = resliced(&volume, OrthoPlane::Axial { slice: 1.0 });
    assert_copied(&upper, &[110.0, 130.0, 150.0, 170.0]);
    // Sagittal column 0 steps row then slice: (0,0,0), (0,1,0), (0,0,1), (0,1,1).
    let sagittal = resliced(&volume, OrthoPlane::Sagittal { column: 0.0 });
    assert_copied(&sagittal, &[10.0, 70.0, 110.0, 150.0]);
    // Coronal row 0 steps column then slice: (0,0,0), (1,0,0), (0,0,1), (1,0,1).
    let coronal = resliced(&volume, OrthoPlane::Coronal { row: 0.0 });
    assert_copied(&coronal, &[10.0, 30.0, 110.0, 130.0]);

    let mixed = oblique(&volume, mix_plane(1));
    assert_copied(&mixed, &[95.0]);
    assert_ne!(copied_bits(&mixed), vec![170.0_f32.to_bits()]);
    assert_ne!(copied_bits(&mixed), vec![70.0_f32.to_bits()]);

    let presented = present_frame(&mixed, 1.0, 1.0);
    let direct = direct_frame(&[95.0], [1, 1], 1.0, 1.0);
    let other = direct_frame(&[70.0], [1, 1], 1.0, 1.0);
    assert_eq!(presented, direct);
    assert_ne!(presented, other);

    let axial_frame = present_frame(&axial, 2.0, 2.0);
    let axial_direct = direct_frame(&[10.0, 30.0, 70.0, 90.0], [2, 2], 2.0, 2.0);
    assert_eq!(axial_frame, axial_direct);
}

#[test]
fn a_refusal_publishes_nothing() {
    let volume = identity_volume();
    let mixed = oblique(&volume, mix_plane(2));
    assert_eq!(present_bits(&sample_at(&mixed, 0, 0)), 95.0_f32.to_bits());
    assert!(matches!(sample_at(&mixed, 1, 0), PlaneSample::Absent));

    let mut kept = vec![Stored(3.0), Stored(3.0)];
    let refused = copy_plane_stored(&mixed, &mut kept);
    assert!(matches!(refused, Err(OrthoError::PlaneIncomplete)));
    let Some(first) = kept.first() else {
        unreachable!("the refusal buffer has two slots")
    };
    let Some(second) = kept.get(1) else {
        unreachable!("the refusal buffer has two slots")
    };
    assert_eq!(first.0.to_bits(), 3.0_f32.to_bits());
    assert_eq!(second.0.to_bits(), 3.0_f32.to_bits());

    let mut raster = require_raster(1.0, 1.0);
    let quality = settled(&raster);
    let mut out = vec![9_u8; 4];
    let presented = present_cpu_plane(
        &mut raster,
        &View::new(),
        &passthrough(),
        &mixed,
        quality,
        &mut out,
    );
    assert!(matches!(presented, Err(OrthoError::PlaneIncomplete)));
    assert_eq!(out, vec![9_u8; 4]);

    let axial = resliced(&volume, OrthoPlane::Axial { slice: 0.0 });
    let mut short = vec![Stored(3.0); 3];
    let length = copy_plane_stored(&axial, &mut short);
    assert!(matches!(
        length,
        Err(OrthoError::Raster(RasterError::Length))
    ));
    assert!(
        short
            .iter()
            .all(|sample| sample.0.to_bits() == 3.0_f32.to_bits())
    );

    let one = oblique(&volume, mix_plane(1));
    let quality = settled(&raster);
    let mut bytes = vec![9_u8; 3];
    let output = present_cpu_plane(
        &mut raster,
        &View::new(),
        &passthrough(),
        &one,
        quality,
        &mut bytes,
    );
    assert!(matches!(
        output,
        Err(OrthoError::Raster(RasterError::OutputLength))
    ));
    assert_eq!(bytes, vec![9_u8; 3]);
}

fn assert_copied(image: &OrthoImage, expected: &[f32]) {
    assert_eq!(image.samples.len(), expected.len());
    for (sample, value) in image.samples.iter().zip(expected.iter().copied()) {
        assert_eq!(present_bits(sample), value.to_bits());
    }
    assert_eq!(
        copied_bits(image),
        expected
            .iter()
            .copied()
            .map(f32::to_bits)
            .collect::<Vec<_>>()
    );
}

fn copied_bits(image: &OrthoImage) -> Vec<u32> {
    let mut stored = vec![Stored(1.0); image.samples.len()];
    let copied = copy_plane_stored(image, &mut stored);
    assert!(copied.is_ok());
    match copied {
        Ok(()) => stored.iter().map(|sample| sample.0.to_bits()).collect(),
        Err(_) => unreachable!("the plane was asserted present"),
    }
}

fn present_frame(image: &OrthoImage, css_width: f64, css_height: f64) -> Vec<u8> {
    let mut raster = require_raster(css_width, css_height);
    let quality = settled(&raster);
    let device = raster.device();
    let mut out = vec![9_u8; byte_len(device)];
    let presented = present_cpu_plane(
        &mut raster,
        &View::new(),
        &passthrough(),
        image,
        quality,
        &mut out,
    );
    assert!(presented.is_ok());
    match presented {
        Ok(result) => {
            assert!(result.wrote);
            assert_eq!(result.extent, device);
        }
        Err(_) => unreachable!("the present was asserted ok"),
    }
    out
}

fn direct_frame(values: &[f32], source: [u32; 2], css_width: f64, css_height: f64) -> Vec<u8> {
    let mut raster = require_raster(css_width, css_height);
    let quality = settled(&raster);
    let device = raster.device();
    let stored: Vec<Stored> = values.iter().copied().map(Stored).collect();
    let mut out = vec![9_u8; byte_len(device)];
    let drawn = raster.raster_into(
        &View::new(),
        &passthrough(),
        &stored,
        RasterGrid {
            source,
            target: device,
        },
        quality,
        &mut out,
    );
    assert!(drawn.is_ok());
    match drawn {
        Ok(result) => assert!(result.wrote),
        Err(_) => unreachable!("the direct raster was asserted ok"),
    }
    out
}

fn settled(raster: &CpuRaster) -> RasterQuality {
    RasterQuality::Settled {
        generation: raster.generation(),
    }
}

fn byte_len(device: [u32; 2]) -> usize {
    usize::try_from(device[0])
        .unwrap_or(0)
        .saturating_mul(usize::try_from(device[1]).unwrap_or(0))
        .saturating_mul(4)
}

fn require_raster(css_width: f64, css_height: f64) -> CpuRaster {
    let built = CpuRaster::open(css_width, css_height, 1.0);
    assert!(built.is_ok());
    match built {
        Ok(raster) => raster,
        Err(_) => unreachable!("the canvas size is finite and positive"),
    }
}

fn passthrough() -> Presentation {
    let modality = ModalityTransform::new(None, Some(1.0), Some(0.0));
    assert!(modality.is_ok());
    let modality = match modality {
        Ok(value) => value,
        Err(_) => unreachable!("slope and intercept are finite"),
    };
    let window = VoiTransform::new(
        None,
        &[127.5],
        &[255.0],
        0,
        VoiFunction::LinearExact,
        0.0,
        255.0,
    );
    assert!(window.is_ok());
    let window = match window {
        Ok(value) => value,
        Err(_) => unreachable!("the window is inside the VOI domain"),
    };
    let chain = LutChain::new(
        modality,
        window,
        PhotometricInterpretation::Monochrome2,
        PresentationLutEvidence::Absent,
    );
    assert!(chain.is_ok());
    let chain = match chain {
        Ok(value) => value,
        Err(_) => unreachable!("the monochrome chain is in domain"),
    };
    Presentation::new(
        chain,
        PhotometricInterpretation::Monochrome2,
        PresentationLutEvidence::Absent,
        Colormap::Grey,
    )
}

fn present_bits(sample: &PlaneSample) -> u32 {
    match sample {
        PlaneSample::Present(value) => value.to_bits(),
        PlaneSample::Absent => unreachable!("this fixture plane is complete"),
    }
}

fn sample_at(image: &OrthoImage, column: u32, row: u32) -> PlaneSample {
    let width = usize::try_from(image.width).unwrap_or(0);
    let index = usize::try_from(row)
        .unwrap_or(0)
        .saturating_mul(width)
        .saturating_add(usize::try_from(column).unwrap_or(0));
    let Some(sample) = image.samples.get(index) else {
        unreachable!("the fixture pixel is inside the plane")
    };
    *sample
}

fn resliced(volume: &Volume, plane: OrthoPlane) -> OrthoImage {
    let image = volume.reslice(plane);
    let Ok(image) = image else {
        unreachable!("the identity index is inside the volume")
    };
    image
}

fn oblique(volume: &Volume, plane: ObliquePlane) -> OrthoImage {
    let image = volume.reslice_oblique(plane);
    let Ok(image) = image else {
        unreachable!("the mix plane is valid on the identity volume")
    };
    image
}

fn mix_plane(columns: u32) -> ObliquePlane {
    ObliquePlane {
        origin: Pt::<World>::new(0.5, 0.5, 0.5),
        row: DVec3::new(1.0, 0.0, 0.0),
        column: DVec3::new(0.0, 0.0, 1.0),
        columns,
        rows: 1,
        column_mm: 1.0,
        row_mm: 1.0,
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
