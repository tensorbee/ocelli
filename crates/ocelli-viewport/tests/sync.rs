//! F-055 scene. UID equality is the string. Indexes are the design literals.

use ocelli_core::{Pt, World};
use ocelli_viewport::sync::{SliceChange, SyncError, SyncScene};
use ocelli_volume::{FrameOfReferenceUid, HeldPlane};

mod sync {
    use super::{FrameOfReferenceUid, HeldPlane, Pt, SliceChange, SyncError, SyncScene, World};

    #[test]
    fn same_uid_moves_other_uid_stays() {
        let mut scene = pair();
        scene.register(3, Some(uid("1.2.4")), axial(0.0, 0.0));
        scene.register(4, Some(uid("1.2.3 ")), axial(0.0, 0.0));
        let changes = must_set(&mut scene, 1, Pt::<World>::new(4.0, 7.0, 0.0));
        assert_eq!(changes.len(), 1);
        assert_eq!(changes.first().map(|change| change.id), Some(2));
        assert_eq!(index_bits(&scene, 2), Some((-10.0_f64).to_bits()));
        assert_eq!(index_bits(&scene, 1), Some(0.0_f64.to_bits()));
        assert_eq!(index_bits(&scene, 3), Some(0.0_f64.to_bits()));
        assert_eq!(index_bits(&scene, 4), Some(0.0_f64.to_bits()));
        assert_eq!(anchor_z(&scene, 2), Some(10.0_f64.to_bits()));
    }

    #[test]
    fn missing_source_uid_moves_nobody() {
        let mut scene = SyncScene::new();
        scene.register(1, None, axial(0.0, 0.0));
        scene.register(2, Some(uid("1.2.3")), axial(10.0, 0.0));
        assert_eq!(
            scene.set_point(1, Pt::<World>::new(4.0, 7.0, 0.0)),
            Err(SyncError::Unavailable)
        );
        assert_eq!(index_bits(&scene, 1), Some(0.0_f64.to_bits()));
        assert_eq!(index_bits(&scene, 2), Some(0.0_f64.to_bits()));
    }

    #[test]
    fn missing_peer_uid_does_not_join() {
        let mut scene = pair();
        scene.register(3, None, axial(0.0, 4.0));
        let changes = must_set(&mut scene, 1, Pt::<World>::new(0.0, 0.0, 0.0));
        assert_eq!(changes.len(), 1);
        assert_eq!(index_bits(&scene, 2), Some((-10.0_f64).to_bits()));
        assert_eq!(index_bits(&scene, 3), Some(4.0_f64.to_bits()));
        assert_eq!(index_bits(&scene, 1), Some(0.0_f64.to_bits()));
    }

    #[test]
    fn stages_then_refuses() {
        let mut scene = SyncScene::new();
        scene.register(1, Some(uid("1.2.3")), axial(10.0, 0.0));
        let mut broken = axial(0.0, 0.0);
        broken.spacing_mm = 0.0;
        scene.register(2, Some(uid("1.2.3")), broken);
        assert_eq!(
            scene.set_point(1, Pt::<World>::new(4.0, 7.0, 0.0)),
            Err(SyncError::Unavailable)
        );
        assert_eq!(index_bits(&scene, 1), Some(0.0_f64.to_bits()));
        assert_eq!(index_bits(&scene, 2), Some(0.0_f64.to_bits()));
        assert_eq!(
            scene.set_point(9, Pt::<World>::new(0.0, 0.0, 0.0)),
            Err(SyncError::Unavailable)
        );
        assert!(scene.plane(9).is_none());
    }

    #[test]
    fn repeat_is_empty_then_a_new_point_moves() {
        let mut scene = pair();
        let _first = must_set(&mut scene, 1, Pt::<World>::new(4.0, 7.0, 0.0));
        let again = must_set(&mut scene, 1, Pt::<World>::new(4.0, 7.0, 0.0));
        assert!(again.is_empty());
        assert_eq!(index_bits(&scene, 2), Some((-10.0_f64).to_bits()));
        let moved = must_set(&mut scene, 1, Pt::<World>::new(0.0, 0.0, 10.0));
        assert_eq!(
            moved,
            vec![
                SliceChange { id: 1, index: 10.0 },
                SliceChange { id: 2, index: 0.0 },
            ]
        );
        assert_eq!(index_bits(&scene, 1), Some(10.0_f64.to_bits()));
        assert_eq!(index_bits(&scene, 2), Some(0.0_f64.to_bits()));
        assert_eq!(anchor_z(&scene, 1), Some(0.0_f64.to_bits()));
        assert_eq!(anchor_z(&scene, 2), Some(10.0_f64.to_bits()));
    }

    #[test]
    fn signed_zero_is_the_same_index() {
        // Design: the change list uses `==`. `-0` and `0` are one index.
        // Bit patterns differ, so a bit compare would report a move.
        let mut scene = SyncScene::new();
        let mut plane = axial(0.0, 0.0);
        plane.index = -0.0;
        scene.register(1, Some(uid("1.2.3")), plane);
        let changes = must_set(&mut scene, 1, Pt::<World>::new(0.0, 0.0, 0.0));
        assert!(changes.is_empty());
        assert_eq!(index_bits(&scene, 1), Some((-0.0_f64).to_bits()));
    }

    #[test]
    fn refusal_and_register_preserve_then_retry() {
        let mut scene = pair();
        let _moved = must_set(&mut scene, 1, Pt::<World>::new(0.0, 0.0, 0.0));
        scene.register(8, None, axial(0.0, 0.0));
        assert_eq!(
            scene.set_point(8, Pt::<World>::new(0.0, 0.0, 10.0)),
            Err(SyncError::Unavailable)
        );
        assert_eq!(index_bits(&scene, 2), Some((-10.0_f64).to_bits()));
        scene.register(9, Some(uid("1.2.9")), axial(0.0, 3.0));
        assert_eq!(index_bits(&scene, 2), Some((-10.0_f64).to_bits()));
        assert_eq!(index_bits(&scene, 9), Some(3.0_f64.to_bits()));
        let retry = must_set(&mut scene, 1, Pt::<World>::new(0.0, 0.0, 10.0));
        assert_eq!(index_bits(&scene, 2), Some(0.0_f64.to_bits()));
        assert_eq!(index_bits(&scene, 1), Some(10.0_f64.to_bits()));
        assert_eq!(retry.first().map(|change| change.id), Some(1));
    }

    fn pair() -> SyncScene {
        let mut scene = SyncScene::new();
        scene.register(1, Some(uid("1.2.3")), axial(0.0, 0.0));
        scene.register(2, Some(uid("1.2.3")), axial(10.0, 0.0));
        scene
    }

    fn uid(text: &str) -> FrameOfReferenceUid {
        FrameOfReferenceUid(text.to_owned())
    }

    fn axial(anchor_z: f64, index: f64) -> HeldPlane {
        HeldPlane {
            anchor: Pt::<World>::new(0.0, 0.0, anchor_z),
            normal: glam::DVec3::new(0.0, 0.0, 1.0),
            spacing_mm: 1.0,
            index,
        }
    }

    fn must_set(scene: &mut SyncScene, source: u16, point: Pt<World>) -> Vec<SliceChange> {
        match scene.set_point(source, point) {
            Ok(changes) => changes,
            Err(_) => unreachable!("the fixture point is on a registered source"),
        }
    }

    fn index_bits(scene: &SyncScene, id: u16) -> Option<u64> {
        scene.plane(id).map(|plane| plane.index.to_bits())
    }

    fn anchor_z(scene: &SyncScene, id: u16) -> Option<u64> {
        scene.plane(id).map(|plane| plane.anchor.z.to_bits())
    }
}
