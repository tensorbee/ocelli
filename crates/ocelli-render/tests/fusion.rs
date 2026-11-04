//! F-056 fusion. The composite and the display index are hand-computed.
//!
//! LINEAR_EXACT centre 40 width 400 maps stored 40 to display 127.5.
//! The existing index of 127.5 is 128. Table 128 is red. Table 40 is blue.
//!
//! PET `(200, 0, 0)` in front of CT `(10, 10, 10)` is `(200, 0, 0)` when both
//! opacities are 1, and `(102.5, 2.5, 2.5)` when both opacities are 0.5.

use glam::DMat3;
use ocelli_core::{Pt, World};
use ocelli_pixel::{
    ImageOrientationPatient, LutChain, LutDescriptor, ModalityTransform, PhotometricInterpretation,
    PresentationLutEvidence, Rgb, VoiFunction, VoiTransform,
};
use ocelli_render::{
    FrameOutcome, FusionError, FusionLedger, FusionRequest, RenderGraph, Tier, VoiParamsError,
    colour_after_voi, composite, composite_sample, fusion_module,
};
use ocelli_volume::{
    FrameOfReferenceUid, FusionPlane, Level, OrthoPlane, SlicePresence, Volume, VoxelKind,
    VoxelStore, plane_from_ortho,
};

mod fusion {
    use super::{
        DMat3, FrameOfReferenceUid, FrameOutcome, FusionError, FusionLedger, FusionPlane,
        FusionRequest, ImageOrientationPatient, Level, LutChain, LutDescriptor, ModalityTransform,
        OrthoPlane, PhotometricInterpretation, PresentationLutEvidence, Pt, RenderGraph, Rgb,
        SlicePresence, Tier, VoiFunction, VoiParamsError, VoiTransform, Volume, VoxelKind,
        VoxelStore, World, colour_after_voi, composite, composite_sample, fusion_module,
        plane_from_ortho,
    };

    #[test]
    fn fusion_request_has_no_patient_weight() {
        let volume = voxel();
        let uid = FrameOfReferenceUid(String::from("1.2.3"));
        let chain = window_chain();
        let plane = unit_plane();
        let request = FusionRequest {
            volume_a: &volume,
            frame_a: &uid,
            volume_b: &volume,
            frame_b: &uid,
            chain_a: &chain,
            chain_b: &chain,
            opacity_a: 1.0,
            opacity_b: 1.0,
            colormap_a: None,
            colormap_b: None,
            plane: &plane,
        };
        assert_eq!(request.opacity_a.to_bits(), 1.0_f32.to_bits());
        assert_eq!(request.opacity_b.to_bits(), 1.0_f32.to_bits());
    }

    #[test]
    fn pet_in_front_is_not_the_sum_or_the_back_colour() {
        let pet = Rgb {
            r: 200.0,
            g: 0.0,
            b: 0.0,
        };
        let ct = Rgb {
            r: 10.0,
            g: 10.0,
            b: 10.0,
        };
        let full = composite(pet, ct, 1.0, 1.0);
        assert_eq!(full.r.to_bits(), 200.0_f32.to_bits());
        assert_eq!(full.g.to_bits(), 0.0_f32.to_bits());
        assert_eq!(full.b.to_bits(), 0.0_f32.to_bits());
        assert_ne!(full.r.to_bits(), 10.0_f32.to_bits());
        assert_ne!(full.r.to_bits(), 210.0_f32.to_bits());
        let half = composite(pet, ct, 0.5, 0.5);
        assert_eq!(half.r.to_bits(), 102.5_f32.to_bits());
        assert_eq!(half.g.to_bits(), 2.5_f32.to_bits());
        assert_eq!(half.b.to_bits(), 2.5_f32.to_bits());
    }

    #[test]
    fn an_absent_volume_adds_no_colour() {
        let pet = Rgb {
            r: 200.0,
            g: 0.0,
            b: 0.0,
        };
        let ct = Rgb {
            r: 10.0,
            g: 10.0,
            b: 10.0,
        };
        let only_ct = composite_sample(None, Some(ct), 0.5, 0.5);
        let Some(only_ct) = only_ct else {
            unreachable!("the back volume is present");
        };
        assert_eq!(only_ct.r.to_bits(), 5.0_f32.to_bits());
        assert_ne!(only_ct.r.to_bits(), 2.5_f32.to_bits());
        let only_pet = composite_sample(Some(pet), None, 1.0, 1.0);
        let Some(only_pet) = only_pet else {
            unreachable!("the front volume is present");
        };
        assert_eq!(only_pet.r.to_bits(), 200.0_f32.to_bits());
        assert!(composite_sample(None, None, 1.0, 1.0).is_none());
    }

    #[test]
    fn the_colormap_is_addressed_by_the_display_value() {
        // PS3.3 C.11.2.1.3.2 LINEAR_EXACT. Centre 40, width 400, input 40.
        // HLD 18.3 gives display 127.5. The existing index of 127.5 is 128.
        let chain = window_chain();
        let mut table = vec![
            Rgb {
                r: 0.0,
                g: 0.0,
                b: 0.0,
            };
            256
        ];
        let Some(blue) = table.get_mut(40) else {
            unreachable!("entry 40 is inside the table");
        };
        *blue = Rgb {
            r: 0.0,
            g: 0.0,
            b: 255.0,
        };
        let Some(red) = table.get_mut(128) else {
            unreachable!("entry 128 is inside the table");
        };
        *red = Rgb {
            r: 255.0,
            g: 0.0,
            b: 0.0,
        };
        let colour = match colour_after_voi(&chain, 40.0, Some(&table)) {
            Ok(colour) => colour,
            Err(_) => unreachable!("the chain has a uniform and the table has 256 entries"),
        };
        assert_eq!(colour.r.to_bits(), 255.0_f32.to_bits());
        assert_eq!(colour.g.to_bits(), 0.0_f32.to_bits());
        assert_eq!(colour.b.to_bits(), 0.0_f32.to_bits());
        let grey = match colour_after_voi(&chain, 40.0, None) {
            Ok(colour) => colour,
            Err(_) => unreachable!("an omitted table repeats the display value"),
        };
        assert_eq!(grey.r.to_bits(), 127.5_f32.to_bits());
        assert_eq!(grey.g.to_bits(), 127.5_f32.to_bits());
        assert_eq!(grey.b.to_bits(), 127.5_f32.to_bits());
    }

    #[test]
    fn omitted_grey_uses_the_chain_output_range() {
        // LINEAR_EXACT centre 40 width 400, stored 40, is the middle of the
        // declared output range. Range 0 to 1 is display 0.5. Dividing that
        // display by 255 is a different coefficient. Range 0 to 255 is
        // display 127.5, and the same ratio is 0.5.
        let narrow = chain_from(None, &[40.0], &[400.0], 0.0, 1.0);
        let colour = match colour_after_voi(&narrow, 40.0, None) {
            Ok(colour) => colour,
            Err(_) => unreachable!("the narrow chain has a uniform"),
        };
        assert_eq!(colour.r.to_bits(), 0.5_f32.to_bits());
        let range = 1.0_f32 - 0.0_f32;
        let unit = (colour.r - 0.0) / range;
        assert_eq!(unit.to_bits(), 0.5_f32.to_bits());
        assert_ne!(unit.to_bits(), (colour.r / 255.0).to_bits());
        let wide = match colour_after_voi(&window_chain(), 40.0, None) {
            Ok(colour) => colour,
            Err(_) => unreachable!("the wide chain has a uniform"),
        };
        assert_eq!(wide.r.to_bits(), 127.5_f32.to_bits());
        let wide_unit = (wide.r - 0.0) / 255.0;
        assert_eq!(wide_unit.to_bits(), 0.5_f32.to_bits());
    }

    #[test]
    fn tier_c_returns_before_a_bad_uid() {
        let mut ledger = prepared();
        let volume = voxel();
        let empty = FrameOfReferenceUid(String::new());
        let chain = window_chain();
        let plane = unit_plane();
        let request = pair(
            &volume,
            &empty,
            &volume,
            &empty,
            (&chain, &chain),
            (1.0, 1.0),
            &plane,
        );
        let refused = ledger.present(Tier::Cpu, &request, |_| {
            unreachable!("tier C does not encode");
        });
        assert_eq!(refused, Err(FusionError::TierCUnavailable));
        assert_eq!(ledger.submits(), 0);
        assert_eq!(ledger.published().first().copied(), Some(7));
    }

    #[test]
    fn a_mismatched_uid_is_refused_before_opacity() {
        let mut ledger = prepared();
        let volume = voxel();
        let left = FrameOfReferenceUid(String::from("1.2.3"));
        let right = FrameOfReferenceUid(String::from("1.2.4"));
        let chain = window_chain();
        let plane = unit_plane();
        let request = pair(
            &volume,
            &left,
            &volume,
            &right,
            (&chain, &chain),
            (4.0, 4.0),
            &plane,
        );
        let refused = ledger.present(Tier::A, &request, |_| {
            unreachable!("a mismatched UID does not encode");
        });
        assert_eq!(refused, Err(FusionError::FrameMismatch));
        assert_eq!(ledger.published().first().copied(), Some(7));
    }

    #[test]
    fn an_empty_uid_is_not_a_shared_frame() {
        let mut ledger = prepared();
        let volume = voxel();
        let empty = FrameOfReferenceUid(String::new());
        let chain = window_chain();
        let plane = unit_plane();
        let request = pair(
            &volume,
            &empty,
            &volume,
            &empty,
            (&chain, &chain),
            (1.0, 1.0),
            &plane,
        );
        let refused = ledger.present(Tier::B, &request, |_| {
            unreachable!("an empty UID does not encode");
        });
        assert_eq!(refused, Err(FusionError::FrameMismatch));
        let spaced = FrameOfReferenceUid(String::from("1.2.3 "));
        let plain = FrameOfReferenceUid(String::from("1.2.3"));
        let trailing = pair(
            &volume,
            &plain,
            &volume,
            &spaced,
            (&chain, &chain),
            (1.0, 1.0),
            &plane,
        );
        let refused = ledger.present(Tier::B, &trailing, |_| {
            unreachable!("a trailing space is a different UID");
        });
        assert_eq!(refused, Err(FusionError::FrameMismatch));
    }

    #[test]
    fn opacity_is_refused_before_a_sequence_chain() {
        let mut ledger = prepared();
        let volume = voxel();
        let uid = FrameOfReferenceUid(String::from("1.2.3"));
        let chain = sequence_chain();
        let plane = unit_plane();
        let request = pair(
            &volume,
            &uid,
            &volume,
            &uid,
            (&chain, &chain),
            (1.5, 1.0),
            &plane,
        );
        let refused = ledger.present(Tier::A, &request, |_| {
            unreachable!("an opacity above 1 does not encode");
        });
        assert_eq!(refused, Err(FusionError::Opacity));
        let nan = pair(
            &volume,
            &uid,
            &volume,
            &uid,
            (&chain, &chain),
            (f32::NAN, 1.0),
            &plane,
        );
        let refused = ledger.present(Tier::A, &nan, |_| {
            unreachable!("a non-finite opacity does not encode");
        });
        assert_eq!(refused, Err(FusionError::Opacity));
    }

    #[test]
    fn a_sequence_chain_is_refused_before_the_colormap_length() {
        let mut ledger = prepared();
        let volume = voxel();
        let uid = FrameOfReferenceUid(String::from("1.2.3"));
        let chain = sequence_chain();
        let plane = unit_plane();
        let mut request = pair(
            &volume,
            &uid,
            &volume,
            &uid,
            (&chain, &chain),
            (1.0, 1.0),
            &plane,
        );
        let short = [Rgb {
            r: 0.0,
            g: 0.0,
            b: 0.0,
        }];
        request.colormap_a = Some(&short);
        let refused = ledger.present(Tier::A, &request, |_| {
            unreachable!("a sequence chain does not encode");
        });
        assert_eq!(
            refused,
            Err(FusionError::Chain(
                VoiParamsError::ModalitySequenceNotExpressible
            ))
        );
    }

    #[test]
    fn a_short_colormap_is_refused_before_the_plane() {
        let mut ledger = prepared();
        let volume = voxel();
        let uid = FrameOfReferenceUid(String::from("1.2.3"));
        let chain = window_chain();
        let mut plane = unit_plane();
        plane.width = 0;
        let mut request = pair(
            &volume,
            &uid,
            &volume,
            &uid,
            (&chain, &chain),
            (1.0, 1.0),
            &plane,
        );
        let short = [Rgb {
            r: 0.0,
            g: 0.0,
            b: 0.0,
        }];
        request.colormap_b = Some(&short);
        let refused = ledger.present(Tier::A, &request, |_| {
            unreachable!("a short colormap does not encode");
        });
        assert_eq!(refused, Err(FusionError::ColormapLength));
    }

    #[test]
    fn a_bad_plane_or_spacing_does_not_encode() {
        let mut ledger = prepared();
        let volume = voxel();
        let uid = FrameOfReferenceUid(String::from("1.2.3"));
        let chain = window_chain();
        let mut plane = unit_plane();
        plane.row_dir = (2.0, 0.0, 0.0);
        let request = pair(
            &volume,
            &uid,
            &volume,
            &uid,
            (&chain, &chain),
            (1.0, 1.0),
            &plane,
        );
        let refused = ledger.present(Tier::A, &request, |_| {
            unreachable!("a non-unit direction does not encode");
        });
        assert_eq!(refused, Err(FusionError::Plane));
        let mut broken = voxel();
        broken.spacing[0] = 0.0;
        let plane = unit_plane();
        let request = pair(
            &broken,
            &uid,
            &volume,
            &uid,
            (&chain, &chain),
            (1.0, 1.0),
            &plane,
        );
        let refused = ledger.present(Tier::A, &request, |_| {
            unreachable!("a zero spacing does not encode");
        });
        assert_eq!(refused, Err(FusionError::Plane));
        assert_eq!(ledger.submits(), 0);
    }

    #[test]
    fn a_normalised_orientation_presents_and_length_two_does_not() {
        // ImageOrientationPatient normalises inside 2e-6. A 45 degree pair
        // is then one ulp off a squared length of 1. Those columns are the
        // directions an axial plane stores. Length 2 stays a refusal.
        let row = [
            std::f64::consts::FRAC_1_SQRT_2,
            std::f64::consts::FRAC_1_SQRT_2,
            0.0,
        ];
        let column = [
            -std::f64::consts::FRAC_1_SQRT_2,
            std::f64::consts::FRAC_1_SQRT_2,
            0.0,
        ];
        let orientation = match ImageOrientationPatient::new(row, column) {
            Ok(orientation) => orientation,
            Err(_) => unreachable!("the 45 degree pair is inside the constructor tolerance"),
        };
        let mut volume = voxel();
        let row_axis = orientation.row();
        let column_axis = orientation.column();
        let normal = row_axis.cross(column_axis).normalize();
        volume.direction = DMat3::from_cols(row_axis, column_axis, normal);
        assert_eq!(
            (row_axis.length_squared() - 1.0).to_bits(),
            4_372_995_238_176_751_616
        );
        assert_eq!(
            (column_axis.length_squared() - 1.0).to_bits(),
            4_372_995_238_176_751_616
        );
        let plane = match plane_from_ortho(&volume, OrthoPlane::Axial { slice: 0.0 }) {
            Some(plane) => plane,
            None => unreachable!("slice 0 is inside the volume"),
        };
        let mut ledger = prepared();
        let uid = FrameOfReferenceUid(String::from("1.2.3"));
        let chain = window_chain();
        let request = pair(
            &volume,
            &uid,
            &volume,
            &uid,
            (&chain, &chain),
            (1.0, 1.0),
            &plane,
        );
        let presented = ledger.present(Tier::A, &request, |bytes| {
            paint(bytes, 9);
            Ok(())
        });
        assert!(presented.is_ok());
        assert_eq!(ledger.published().first().copied(), Some(9));
        let mut stretched = plane;
        stretched.row_dir = (2.0, 0.0, 0.0);
        let request = pair(
            &volume,
            &uid,
            &volume,
            &uid,
            (&chain, &chain),
            (1.0, 1.0),
            &stretched,
        );
        let refused = ledger.present(Tier::A, &request, |_| {
            unreachable!("a direction of length 2 is not renormalised");
        });
        assert_eq!(refused, Err(FusionError::Plane));
        assert_eq!(ledger.published().first().copied(), Some(9));
    }

    #[test]
    fn a_mismatch_keeps_the_frame_and_a_retry_can_publish() {
        let mut ledger = prepared();
        let volume = voxel();
        let uid = FrameOfReferenceUid(String::from("1.2.3"));
        let other = FrameOfReferenceUid(String::from("1.2.4"));
        let chain = window_chain();
        let plane = unit_plane();
        let request = pair(
            &volume,
            &uid,
            &volume,
            &uid,
            (&chain, &chain),
            (1.0, 1.0),
            &plane,
        );
        let first = ledger.present(Tier::A, &request, |bytes| {
            paint(bytes, 9);
            Ok(())
        });
        assert!(first.is_ok());
        assert_eq!(ledger.bound_uid(), Some("1.2.3"));
        assert_eq!(ledger.submits(), 1);
        let mismatch = pair(
            &volume,
            &uid,
            &volume,
            &other,
            (&chain, &chain),
            (1.0, 1.0),
            &plane,
        );
        let refused = ledger.present(Tier::A, &mismatch, |_| {
            unreachable!("the mismatched pair is not bound");
        });
        assert_eq!(refused, Err(FusionError::FrameMismatch));
        assert_eq!(ledger.bound_uid(), Some("1.2.3"));
        assert_eq!(ledger.published().first().copied(), Some(9));
        let retry = pair(
            &volume,
            &uid,
            &volume,
            &uid,
            (&chain, &chain),
            (0.0, 1.0),
            &plane,
        );
        let second = ledger.present(Tier::A, &retry, |bytes| {
            paint(bytes, 4);
            Ok(())
        });
        assert!(second.is_ok());
        assert_eq!(ledger.published().first().copied(), Some(4));
        assert_eq!(ledger.submits(), 2);
        assert_eq!(ledger.allocations(), 1);
    }

    #[test]
    fn a_settled_request_does_not_submit_again() {
        let mut ledger = prepared();
        let volume = voxel();
        let uid = FrameOfReferenceUid(String::from("1.2.3"));
        let chain = window_chain();
        let plane = unit_plane();
        let request = pair(
            &volume,
            &uid,
            &volume,
            &uid,
            (&chain, &chain),
            (1.0, 0.0),
            &plane,
        );
        let first = ledger.present(Tier::B, &request, |bytes| {
            paint(bytes, 3);
            Ok(())
        });
        assert!(first.is_ok());
        let second = ledger.present(Tier::B, &request, |_| {
            unreachable!("a settled request does not encode");
        });
        assert!(second.is_ok());
        assert_eq!(ledger.submits(), 1);
        assert_eq!(ledger.allocations(), 1);
        ledger.mark_data();
        let third = ledger.present(Tier::B, &request, |bytes| {
            paint(bytes, 8);
            Ok(())
        });
        assert!(third.is_ok());
        assert_eq!(ledger.submits(), 2);
        assert_eq!(ledger.allocations(), 1);
        assert_eq!(ledger.published().first().copied(), Some(8));
    }

    /// The store starts at zero. Marking that slice present does not change
    /// a byte. The front volume is still a new sample, because absence is
    /// not stored zero.
    #[test]
    fn a_zero_byte_slice_that_arrives_is_published_again() {
        let mut ledger = prepared();
        let mut back = absent_volume();
        match back.place(0, &[0]) {
            Ok(()) => {}
            Err(_) => unreachable!("the back slice matches the store"),
        }
        let mut front = absent_volume();
        let uid = FrameOfReferenceUid(String::from("1.2.3"));
        let chain = window_chain();
        let plane = unit_plane();
        {
            let request = pair(
                &back,
                &uid,
                &front,
                &uid,
                (&chain, &chain),
                (1.0, 0.5),
                &plane,
            );
            let first = ledger.present(Tier::A, &request, |_| Ok(()));
            assert!(first.is_ok());
            assert_eq!(ledger.submits(), 1);
            let settled = ledger.present(Tier::A, &request, |_| {
                unreachable!("the absent front volume has not changed");
            });
            assert!(settled.is_ok());
            assert_eq!(ledger.submits(), 1);
        }
        match front.place(0, &[0]) {
            Ok(()) => {}
            Err(_) => unreachable!("the arriving slice matches the store"),
        }
        let arrived = pair(
            &back,
            &uid,
            &front,
            &uid,
            (&chain, &chain),
            (1.0, 0.5),
            &plane,
        );
        let second = ledger.present(Tier::A, &arrived, |_| Ok(()));
        assert!(second.is_ok());
        assert_eq!(ledger.submits(), 2);
    }

    /// `sample_at` reads the voxel kind. The same bytes as `I16` are not the
    /// `U8` sample already published.
    #[test]
    fn a_voxel_kind_change_is_published_again() {
        let mut ledger = prepared();
        let mut volume = voxel();
        let uid = FrameOfReferenceUid(String::from("1.2.3"));
        let chain = window_chain();
        let plane = unit_plane();
        let request = pair(
            &volume,
            &uid,
            &volume,
            &uid,
            (&chain, &chain),
            (1.0, 1.0),
            &plane,
        );
        let first = ledger.present(Tier::A, &request, |_| Ok(()));
        assert!(first.is_ok());
        let settled = ledger.present(Tier::A, &request, |_| {
            unreachable!("the same kind does not encode");
        });
        assert!(settled.is_ok());
        assert_eq!(ledger.submits(), 1);
        volume.voxel = VoxelKind::I16;
        let changed = pair(
            &volume,
            &uid,
            &volume,
            &uid,
            (&chain, &chain),
            (1.0, 1.0),
            &plane,
        );
        let second = ledger.present(Tier::A, &changed, |_| Ok(()));
        assert!(second.is_ok());
        assert_eq!(ledger.submits(), 2);
    }

    /// `sample_at` reads `dims`. A wider axis with the same bytes is not the
    /// sample already published.
    #[test]
    fn a_dimension_change_is_published_again() {
        let mut ledger = prepared();
        let mut volume = voxel();
        let uid = FrameOfReferenceUid(String::from("1.2.3"));
        let chain = window_chain();
        let plane = unit_plane();
        let request = pair(
            &volume,
            &uid,
            &volume,
            &uid,
            (&chain, &chain),
            (1.0, 1.0),
            &plane,
        );
        let first = ledger.present(Tier::A, &request, |_| Ok(()));
        assert!(first.is_ok());
        assert_eq!(ledger.submits(), 1);
        volume.dims[0] = 2;
        let changed = pair(
            &volume,
            &uid,
            &volume,
            &uid,
            (&chain, &chain),
            (1.0, 1.0),
            &plane,
        );
        let second = ledger.present(Tier::A, &changed, |_| Ok(()));
        assert!(second.is_ok());
        assert_eq!(ledger.submits(), 2);
    }

    #[test]
    fn the_shader_calls_the_existing_window_and_puts_pet_in_front() {
        let module = fusion_module();
        assert_eq!(module.matches("fn lut_chain(").count(), 1);
        assert_eq!(module.matches("fn voi_linear(").count(), 1);
        assert_eq!(module.matches("fn table_index(").count(), 1);
        assert!(module.contains("table_index(display, fusion.last)"));
        assert!(
            module.contains("let out = pet * opacity_pet + ct * opacity_ct * (1.0 - opacity_pet);")
        );
        assert!(!module.contains("textureSample"));
        assert!(!module.contains("@compute"));
        assert!(!module.contains("var<storage"));
        assert!(module.starts_with(ocelli_render::VOI_WGSL));
        assert!(module.contains("let range = voi.ymax - voi.ymin;"));
        assert!(
            module.contains("let unit = select(0.0, (display - voi.ymin) / range, range > 0.0);")
        );
        assert!(!module.contains("display / 255.0"));
    }

    #[test]
    #[ignore = "needs a real GPU adapter, which the CI floor does not have (D-04)"]
    fn one_submit_encodes_fusion_and_opacity_does_not_reupload() {
        let Some((adapter, mut context)) = gpu_session() else {
            println!("this machine resolved tier C, so there is no device");
            return;
        };
        let mut graph = match RenderGraph::new(&mut context, 1, wgpu::TextureFormat::Rgba8Unorm) {
            Ok(graph) => graph,
            Err(_) => {
                println!("the graph refused this device");
                return;
            }
        };
        let volume = voxel();
        let uid = FrameOfReferenceUid(String::from("1.2.3"));
        let other = FrameOfReferenceUid(String::from("1.2.4"));
        let chain = window_chain();
        let plane = unit_plane();
        let mut request = pair(
            &volume,
            &uid,
            &volume,
            &uid,
            (&chain, &chain),
            (1.0, 1.0),
            &plane,
        );
        assert!(graph.show_fusion(&context, &adapter, &request).is_ok());
        assert_eq!(graph.fusion_sample_uploads(), 1);
        assert_eq!(
            graph.tick(&context),
            FrameOutcome::Submitted {
                viewports: 0,
                passes: 2,
            }
        );
        assert_eq!(graph.submissions(), 1);
        request.opacity_a = 0.5;
        assert!(graph.show_fusion(&context, &adapter, &request).is_ok());
        assert_eq!(graph.fusion_sample_uploads(), 1);
        assert_eq!(
            graph.tick(&context),
            FrameOutcome::Submitted {
                viewports: 0,
                passes: 2,
            }
        );
        assert_eq!(graph.submissions(), 2);
        request.frame_b = &other;
        assert_eq!(
            graph.show_fusion(&context, &adapter, &request),
            Err(FusionError::FrameMismatch)
        );
        assert_eq!(graph.tick(&context), FrameOutcome::Noop);
        assert_eq!(graph.submissions(), 2);
        assert_eq!(graph.fusion_sample_uploads(), 1);
    }

    fn gpu_session() -> Option<(ocelli_render::ResolvedAdapter, ocelli_render::GpuContext)> {
        let mut clock = native_clock();
        let adapter = pollster::block_on(ocelli_render::resolve_adapter(
            ocelli_render::TierRequest::Auto,
            &mut clock,
        ))?;
        let context = pollster::block_on(adapter.open()).ok()?;
        Some((adapter, context))
    }

    fn native_clock() -> impl FnMut() -> u64 {
        let start = std::time::Instant::now();
        move || u64::try_from(start.elapsed().as_nanos()).unwrap_or(u64::MAX)
    }

    fn prepared() -> FusionLedger {
        let mut ledger = FusionLedger::new();
        ledger.bind_extent(1);
        assert!(ledger.seed_published(&[7, 0, 0, 0]));
        ledger
    }

    fn paint(bytes: &mut [u8], value: u8) {
        let Some(slot) = bytes.first_mut() else {
            unreachable!("the bound plane has a byte");
        };
        *slot = value;
    }

    fn pair<'a>(
        volume_a: &'a Volume,
        frame_a: &'a FrameOfReferenceUid,
        volume_b: &'a Volume,
        frame_b: &'a FrameOfReferenceUid,
        chains: (&'a LutChain, &'a LutChain),
        opacities: (f32, f32),
        plane: &'a FusionPlane,
    ) -> FusionRequest<'a> {
        FusionRequest {
            volume_a,
            frame_a,
            volume_b,
            frame_b,
            chain_a: chains.0,
            chain_b: chains.1,
            opacity_a: opacities.0,
            opacity_b: opacities.1,
            colormap_a: None,
            colormap_b: None,
            plane,
        }
    }

    fn unit_plane() -> FusionPlane {
        FusionPlane {
            origin: Pt::<World>::new(0.0, 0.0, 0.0),
            row_dir: (1.0, 0.0, 0.0),
            column_dir: (0.0, 1.0, 0.0),
            column_mm: 1.0,
            row_mm: 1.0,
            width: 1,
            height: 1,
        }
    }

    fn window_chain() -> LutChain {
        chain_from(None, &[40.0], &[400.0], 0.0, 255.0)
    }

    fn sequence_chain() -> LutChain {
        let lut = match LutDescriptor::new(3, -1, 16, vec![5.0, 15.0, 25.0]) {
            Ok(lut) => lut,
            Err(_) => unreachable!("the descriptor is three entries"),
        };
        chain_from(Some(lut), &[40.0], &[400.0], 0.0, 255.0)
    }

    fn chain_from(
        lut: Option<LutDescriptor>,
        centre: &[f32],
        width: &[f32],
        ymin: f32,
        ymax: f32,
    ) -> LutChain {
        let modality = ModalityTransform::new(lut, Some(1.0), Some(0.0));
        let voi = VoiTransform::new(None, centre, width, 0, VoiFunction::LinearExact, ymin, ymax);
        let (Ok(modality), Ok(voi)) = (modality, voi) else {
            unreachable!("the fixture chain is expressible");
        };
        match LutChain::new(
            modality,
            voi,
            PhotometricInterpretation::Monochrome2,
            PresentationLutEvidence::Absent,
        ) {
            Ok(chain) => chain,
            Err(_) => unreachable!("the fixture chain composes"),
        }
    }

    fn absent_volume() -> Volume {
        let dims = [1, 1, 1];
        Volume {
            dims,
            spacing: [1.0, 1.0, 1.0],
            origin: Pt::<World>::new(0.0, 0.0, 0.0),
            direction: DMat3::IDENTITY,
            voxel: VoxelKind::U8,
            data: VoxelStore::new(dims, VoxelKind::U8),
            levels: vec![Level {
                dims,
                spacing: [1.0, 1.0, 1.0],
                offset: 0,
            }],
            present: SlicePresence::new(1),
            order: Vec::new(),
        }
    }

    fn voxel() -> Volume {
        let dims = [1, 1, 1];
        let mut volume = Volume {
            dims,
            spacing: [1.0, 1.0, 1.0],
            origin: Pt::<World>::new(0.0, 0.0, 0.0),
            direction: DMat3::IDENTITY,
            voxel: VoxelKind::U8,
            data: VoxelStore::new(dims, VoxelKind::U8),
            levels: vec![Level {
                dims,
                spacing: [1.0, 1.0, 1.0],
                offset: 0,
            }],
            present: SlicePresence::new(1),
            order: Vec::new(),
        };
        match volume.place(0, &[1]) {
            Ok(()) => volume,
            Err(_) => unreachable!("the fixture slice matches the store"),
        }
    }
}
