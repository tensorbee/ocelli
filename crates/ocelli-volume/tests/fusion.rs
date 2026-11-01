//! F-056 world sample. The expected points are PS3.3 C.7.6.2.1.1.
//!
//! First volume: origin `(0, 0, 0)`, spacing `[1, 2, 5]`. Column index 3 and
//! row index 4 at `k = 0` are the point `(3, 8, 0)`. Swapped spacing indices
//! are `(6, 4, 0)`.
//!
//! Second volume: origin `(1, 0, 0)`, spacing `[2, 2, 5]`. The point
//! `(3, 8, 10)` is index `(1, 4, 2)`. Reusing column index 3 is the wrong voxel.

use glam::DMat3;
use ocelli_core::{Pt, World};
use ocelli_volume::{
    Level, OrthoPlane, PlaneSample, SlicePresence, Volume, VoxelKind, VoxelStore, plane_from_ortho,
    world_on_plane,
};

mod fusion {
    use super::{
        DMat3, Level, OrthoPlane, PlaneSample, Pt, SlicePresence, Volume, VoxelKind, VoxelStore,
        World, plane_from_ortho, world_on_plane,
    };

    #[test]
    fn world_point_uses_column_then_row_spacing() {
        // `P = 3 * spacing[0] * X + 4 * spacing[1] * Y` with spacing `[1, 2, 5]`.
        // That is `(3, 8, 0)`. The swapped pair is `(6, 4, 0)`.
        let volume = filled([4, 5, 1], [1.0, 2.0, 5.0], (0.0, 0.0, 0.0), 0, 19, 200);
        let point = volume.voxel_world([3.0, 4.0, 0.0]);
        assert_eq!(point.x.to_bits(), 3.0_f64.to_bits());
        assert_eq!(point.y.to_bits(), 8.0_f64.to_bits());
        assert_eq!(point.z.to_bits(), 0.0_f64.to_bits());
        assert_ne!(point.x.to_bits(), 6.0_f64.to_bits());
        assert_ne!(point.y.to_bits(), 4.0_f64.to_bits());
        assert_eq!(
            sample_bits(&volume, Pt::<World>::new(3.0, 8.0, 0.0)),
            200.0_f32.to_bits()
        );
    }

    #[test]
    fn second_volume_index_is_not_the_first_column() {
        // Delta from origin `(1, 0, 0)` is `(2, 8, 10)`. Spacing `[2, 2, 5]`
        // gives index `(1, 4, 2)`. Column 3 would be the first volume's `i`.
        let mut bytes = vec![0_u8; 4 * 5];
        put(&mut bytes, 17, 42);
        put(&mut bytes, 19, 7);
        let volume = volume_with([4, 5, 3], [2.0, 2.0, 5.0], (1.0, 0.0, 0.0), 2, &bytes);
        let on_index = volume.voxel_world([1.0, 4.0, 2.0]);
        assert_eq!(on_index.x.to_bits(), 3.0_f64.to_bits());
        assert_eq!(on_index.y.to_bits(), 8.0_f64.to_bits());
        assert_eq!(on_index.z.to_bits(), 10.0_f64.to_bits());
        assert_eq!(
            sample_bits(&volume, Pt::<World>::new(3.0, 8.0, 10.0)),
            42.0_f32.to_bits()
        );
        assert_ne!(
            sample_bits(&volume, Pt::<World>::new(3.0, 8.0, 10.0)),
            7.0_f32.to_bits()
        );
    }

    #[test]
    fn an_axial_plane_matches_the_column_then_row_map() {
        // Pixel `(3, 4)` on the `k = 0` plane is `(3, 8, 0)`. Swapped steps
        // would be `(6, 4, 0)`.
        let volume = filled([4, 5, 1], [1.0, 2.0, 5.0], (0.0, 0.0, 0.0), 0, 19, 200);
        let plane = match plane_from_ortho(&volume, OrthoPlane::Axial { slice: 0.0 }) {
            Some(plane) => plane,
            None => unreachable!("slice 0 is inside the volume"),
        };
        let point = world_on_plane(&plane, 3, 4);
        assert_eq!(point.x.to_bits(), 3.0_f64.to_bits());
        assert_eq!(point.y.to_bits(), 8.0_f64.to_bits());
        assert_eq!(point.z.to_bits(), 0.0_f64.to_bits());
        assert_ne!(point.x.to_bits(), 6.0_f64.to_bits());
        assert_ne!(point.y.to_bits(), 4.0_f64.to_bits());
    }

    #[test]
    fn a_sagittal_plane_steps_on_the_column_cosine() {
        // Column index 0. Output column 1 steps `spacing[1]` along Y, so the
        // point is `(0, 2, 0)`. Using `spacing[0]` would leave y at 1.
        let volume = filled([4, 5, 3], [1.0, 2.0, 5.0], (0.0, 0.0, 0.0), 0, 0, 1);
        let plane = match plane_from_ortho(&volume, OrthoPlane::Sagittal { column: 0.0 }) {
            Some(plane) => plane,
            None => unreachable!("column 0 is inside the volume"),
        };
        let point = world_on_plane(&plane, 1, 0);
        assert_eq!(point.x.to_bits(), 0.0_f64.to_bits());
        assert_eq!(point.y.to_bits(), 2.0_f64.to_bits());
        assert_eq!(point.z.to_bits(), 0.0_f64.to_bits());
        assert_ne!(point.y.to_bits(), 1.0_f64.to_bits());
    }

    #[test]
    fn a_coronal_plane_steps_on_the_slice_cosine() {
        // Row index 0. Output pixel `(1, 1)` steps `spacing[0]` along X and
        // `spacing[2]` along Z, so the point is `(1, 0, 5)`. Using
        // `spacing[1]` for the column step would put x at 2.
        let volume = filled([4, 5, 3], [1.0, 2.0, 5.0], (0.0, 0.0, 0.0), 0, 0, 1);
        let plane = match plane_from_ortho(&volume, OrthoPlane::Coronal { row: 0.0 }) {
            Some(plane) => plane,
            None => unreachable!("row 0 is inside the volume"),
        };
        let point = world_on_plane(&plane, 1, 1);
        assert_eq!(point.x.to_bits(), 1.0_f64.to_bits());
        assert_eq!(point.y.to_bits(), 0.0_f64.to_bits());
        assert_eq!(point.z.to_bits(), 5.0_f64.to_bits());
        assert_ne!(point.x.to_bits(), 2.0_f64.to_bits());
    }

    #[test]
    fn an_index_outside_the_volume_builds_no_plane() {
        let volume = filled([4, 5, 1], [1.0, 2.0, 5.0], (0.0, 0.0, 0.0), 0, 0, 1);
        assert!(plane_from_ortho(&volume, OrthoPlane::Axial { slice: 5.0 }).is_none());
    }

    #[test]
    fn a_point_outside_the_grid_is_absent() {
        let volume = filled([4, 5, 1], [1.0, 2.0, 5.0], (0.0, 0.0, 0.0), 0, 19, 200);
        let sample = volume.sample_world(Pt::<World>::new(1000.0, 0.0, 0.0));
        assert_eq!(sample, PlaneSample::Absent);
        assert_ne!(sample_bits(&volume, Pt::<World>::new(3.0, 8.0, 0.0)), 0);
    }

    fn filled(
        dims: [u32; 3],
        spacing: [f64; 3],
        origin: (f64, f64, f64),
        slice: u32,
        at: usize,
        value: u8,
    ) -> Volume {
        let mut bytes = vec![0_u8; slice_count(dims)];
        put(&mut bytes, at, value);
        volume_with(dims, spacing, origin, slice, &bytes)
    }

    fn volume_with(
        dims: [u32; 3],
        spacing: [f64; 3],
        origin: (f64, f64, f64),
        slice: u32,
        bytes: &[u8],
    ) -> Volume {
        let mut volume = Volume {
            dims,
            spacing,
            origin: Pt::<World>::new(origin.0, origin.1, origin.2),
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
        match volume.place(slice, bytes) {
            Ok(()) => volume,
            Err(_) => unreachable!("the fixture slice matches the store"),
        }
    }

    fn put(bytes: &mut [u8], at: usize, value: u8) {
        match bytes.get_mut(at) {
            Some(slot) => *slot = value,
            None => unreachable!("the fixture offset is inside the slice"),
        }
    }

    fn slice_count(dims: [u32; 3]) -> usize {
        let width = match usize::try_from(dims[0]) {
            Ok(width) => width,
            Err(_) => unreachable!("the fixture width fits"),
        };
        let height = match usize::try_from(dims[1]) {
            Ok(height) => height,
            Err(_) => unreachable!("the fixture height fits"),
        };
        match width.checked_mul(height) {
            Some(count) => count,
            None => unreachable!("the fixture slice fits"),
        }
    }

    fn sample_bits(volume: &Volume, point: Pt<World>) -> u32 {
        match volume.sample_world(point) {
            PlaneSample::Present(value) => value.to_bits(),
            PlaneSample::Absent => unreachable!("this point lands on a present voxel"),
        }
    }
}
