//! F-055 slice index on one held plane.
//!
//! PS3.3 C.7.6.2.1.1 places the plane. The expected indexes are the literals
//! from the design, not a second call to the function under test.

use glam::DVec3;
use ocelli_core::{Pt, World};
use ocelli_volume::{HeldPlane, PlaneSyncError};

mod sync {
    use super::{DVec3, HeldPlane, PlaneSyncError, Pt, World};

    #[test]
    fn matching_world_z() {
        // Anchor `(0, 0, 10)`, normal `(0, 0, 1)`, spacing 1, index 0.
        // Both points lie on z = 0. The index is -10. World z is
        // `10 + (-10) * 1 = 0`. A sign error is 10. Adding x is -6.
        let plane = axial(10.0, 0.0);
        let on_axis = must_index(&plane, Pt::<World>::new(0.0, 0.0, 0.0));
        let in_plane = must_index(&plane, Pt::<World>::new(4.0, 7.0, 0.0));
        assert_eq!(on_axis.to_bits(), (-10.0_f64).to_bits());
        assert_eq!(in_plane.to_bits(), (-10.0_f64).to_bits());
        assert_ne!(in_plane.to_bits(), 10.0_f64.to_bits());
        assert_ne!(in_plane.to_bits(), (-6.0_f64).to_bits());
        let world_z = 10.0_f64 + (-10.0_f64) * 1.0;
        assert_eq!(world_z.to_bits(), 0.0_f64.to_bits());
        assert_eq!(plane.index.to_bits(), 0.0_f64.to_bits());
        assert_eq!(plane.anchor.z.to_bits(), 10.0_f64.to_bits());
    }

    #[test]
    fn sagittal_uses_its_own_normal() {
        // Anchor origin, normal `(1, 0, 0)`, spacing 1, index 3.
        // Point `(0, 0, 5)` lies on the plane x = 0, so the index is 0.
        // The slice normal `(0, 0, 1)` would return 5.
        let plane = HeldPlane {
            anchor: Pt::<World>::new(0.0, 0.0, 0.0),
            normal: DVec3::new(1.0, 0.0, 0.0),
            spacing_mm: 1.0,
            index: 3.0,
        };
        let next = must_index(&plane, Pt::<World>::new(0.0, 0.0, 5.0));
        assert_eq!(next.to_bits(), 0.0_f64.to_bits());
        assert_ne!(next.to_bits(), 5.0_f64.to_bits());
        assert_eq!(plane.normal.x.to_bits(), 1.0_f64.to_bits());
        assert_eq!(plane.index.to_bits(), 3.0_f64.to_bits());
    }

    #[test]
    fn refuses_before_a_quotient() {
        let plane = axial(10.0, 0.0);
        assert_eq!(
            plane.index_through_point(Pt::<World>::new(f64::NAN, 0.0, 0.0)),
            Err(PlaneSyncError::Unavailable)
        );
        let mut bad = plane;
        bad.anchor = Pt::<World>::new(0.0, 0.0, f64::INFINITY);
        assert_eq!(
            bad.index_through_point(Pt::<World>::new(0.0, 0.0, 0.0)),
            Err(PlaneSyncError::Unavailable)
        );
        bad = plane;
        bad.normal = DVec3::new(0.0, f64::NAN, 1.0);
        assert_eq!(
            bad.index_through_point(Pt::<World>::new(0.0, 0.0, 0.0)),
            Err(PlaneSyncError::Unavailable)
        );
        bad = plane;
        bad.index = f64::NAN;
        assert_eq!(
            bad.index_through_point(Pt::<World>::new(0.0, 0.0, 0.0)),
            Err(PlaneSyncError::Unavailable)
        );
        bad = plane;
        bad.spacing_mm = 0.0;
        assert_eq!(
            bad.index_through_point(Pt::<World>::new(0.0, 0.0, 0.0)),
            Err(PlaneSyncError::Unavailable)
        );
        bad.spacing_mm = -1.0;
        assert_eq!(
            bad.index_through_point(Pt::<World>::new(0.0, 0.0, 0.0)),
            Err(PlaneSyncError::Unavailable)
        );
        bad = plane;
        bad.normal = DVec3::ZERO;
        assert_eq!(
            bad.index_through_point(Pt::<World>::new(0.0, 0.0, 0.0)),
            Err(PlaneSyncError::Unavailable)
        );
        assert_eq!(plane.index.to_bits(), 0.0_f64.to_bits());
        assert_eq!(
            must_index(&plane, Pt::<World>::new(0.0, 0.0, 0.0)).to_bits(),
            (-10.0_f64).to_bits()
        );
    }

    fn axial(anchor_z: f64, index: f64) -> HeldPlane {
        HeldPlane {
            anchor: Pt::<World>::new(0.0, 0.0, anchor_z),
            normal: DVec3::new(0.0, 0.0, 1.0),
            spacing_mm: 1.0,
            index,
        }
    }

    fn must_index(plane: &HeldPlane, point: Pt<World>) -> f64 {
        match plane.index_through_point(point) {
            Ok(index) => index,
            Err(_) => unreachable!("the fixture plane accepts this point"),
        }
    }
}
