//! Round-trip properties for `Transform`, from HLD section 25.
//!
//! Section 25's listing is
//!
//! ```text
//! proptest! {
//!     #[test]
//!     fn canvas_world_roundtrip(x in -1e4f64..1e4, y in -1e4f64..1e4) {
//!         let p = Pt::<Canvas>::new(x, y, 0.0);
//!         let t = viewport.canvas_to_world();
//!         let back = t.inverse().apply(t.apply(p));
//!         prop_assert!((back.x - p.x).abs() < 1e-6);
//!     }
//! }
//! ```
//!
//! There is no viewport yet, that is F-023, so the transform is built here
//! from an explicit matrix. The generator range and the 1e-6 tolerance are
//! section 25's. All three components are checked rather than only `x`,
//! because a transform that is wrong in `z` alone is exactly the kind of
//! error the listing's single assertion would miss.
//!
//! # The unit these tests bound is the CSS pixel, not the millimetre
//!
//! A round trip ends where it started. `t` is a `Transform<Canvas, World>`,
//! so `t.inverse().apply(t.apply(p))` is a `Pt<Canvas>` and every difference
//! asserted below is in canvas coordinates. The one exception is labelled
//! where it appears, in `the_perspective_divide_is_visible_at_this_tolerance`,
//! which separately compares two `Pt<World>` and is in millimetres.
//!
//! This is worth stating because HLD section 25.1 has two geometry rows and
//! they are easy to reach for in the wrong order. World coordinates are
//! within 1e-6 mm, and canvas coordinates within a quarter pixel. The bound
//! here is section 25's own 1e-6, which is roughly a quarter of a million
//! times tighter than 25.1's canvas row, and it is deliberate: these are
//! exact-arithmetic round trips that should return to the same number, not
//! rendered positions allowed to drift. Loosening 1e-6 toward 0.25 on the
//! authority of that row would read as a correction and would be a tolerance
//! change, which AGENTS.md makes a reviewed design decision and not a fix.

use glam::{DMat4, DVec4};
use ocelli_core::{Canvas, Pt, Transform, World};
use proptest::prelude::*;

/// The round-trip bound, in CSS pixels of canvas coordinate. HLD section 25's
/// listing, and far tighter than 25.1's quarter-pixel canvas row. See the
/// module comment before changing it.
const ROUND_TRIP_TOLERANCE_PX: f64 = 1e-6;

/// An orthonormal patient frame. The first two vectors are the row and column
/// direction cosines of `geometry_ps3_3_c7_6_2.rs`, and the third is their
/// cross product, so this is a right-handed LPS frame.
const FRAME_X: (f64, f64, f64) = (0.6, -0.64, 0.48);
const FRAME_Y: (f64, f64, f64) = (0.8, 0.48, -0.36);
const FRAME_Z: (f64, f64, f64) = (0.0, 0.6, 0.8);

/// Millimetres per CSS pixel, both canvas axes.
const MM_PER_PIXEL: f64 = 0.35;

/// The world-space position of the canvas origin.
const CANVAS_ORIGIN: (f64, f64, f64) = (-45.2, 118.7, -32.5);

/// The `w` row's `z` coefficient. Zero is the ordinary orthographic viewport
/// and `PERSPECTIVE_W_PER_MM` is a perspective one.
const PERSPECTIVE_W_PER_MM: f64 = 1.0 / 2000.0;

/// A canvas-to-world transform. `w_per_z` of zero gives an affine transform,
/// which is what an ORTHOGRAPHIC viewport produces. A non-zero `w_per_z`
/// gives a projective one, which is what a PERSPECTIVE viewport produces, and
/// then the `w` divide in `apply` is load bearing.
fn canvas_to_world(w_per_z: f64) -> Transform<Canvas, World> {
    let (xx, xy, xz) = FRAME_X;
    let (yx, yy, yz) = FRAME_Y;
    let (zx, zy, zz) = FRAME_Z;
    let (ox, oy, oz) = CANVAS_ORIGIN;
    let s = MM_PER_PIXEL;

    Transform::from_mat4(DMat4::from_cols(
        DVec4::new(xx * s, xy * s, xz * s, 0.0),
        DVec4::new(yx * s, yy * s, yz * s, 0.0),
        DVec4::new(zx, zy, zz, w_per_z),
        DVec4::new(ox, oy, oz, 1.0),
    ))
}

proptest! {
    /// HLD section 25, transcribed, with a locally built orthographic
    /// transform standing in for the viewport that does not exist yet.
    #[test]
    fn canvas_world_roundtrip(x in -1e4f64..1e4, y in -1e4f64..1e4) {
        let p = Pt::<Canvas>::new(x, y, 0.0);
        let t = canvas_to_world(0.0);
        let back = t.inverse().apply(t.apply(p));
        prop_assert!((back.x - p.x).abs() < ROUND_TRIP_TOLERANCE_PX, "x drifted to {}", back.x);
        prop_assert!((back.y - p.y).abs() < ROUND_TRIP_TOLERANCE_PX, "y drifted to {}", back.y);
        prop_assert!((back.z - p.z).abs() < ROUND_TRIP_TOLERANCE_PX, "z drifted to {}", back.z);
    }

    /// The same round trip under a projective transform. `z` is bounded so
    /// that `w` stays in [0.75, 1.25] and the case is well conditioned. A
    /// transform that ignored the `w` divide comes back tens of CSS pixels
    /// away from where it started. The fixed case below carries the
    /// reproducible measured drift.
    #[test]
    fn canvas_world_roundtrip_under_perspective(
        x in -1e4f64..1e4,
        y in -1e4f64..1e4,
        z in -500.0f64..500.0,
    ) {
        let p = Pt::<Canvas>::new(x, y, z);
        let t = canvas_to_world(PERSPECTIVE_W_PER_MM);
        let back = t.inverse().apply(t.apply(p));
        prop_assert!((back.x - p.x).abs() < ROUND_TRIP_TOLERANCE_PX, "x drifted to {}", back.x);
        prop_assert!((back.y - p.y).abs() < ROUND_TRIP_TOLERANCE_PX, "y drifted to {}", back.y);
        prop_assert!((back.z - p.z).abs() < ROUND_TRIP_TOLERANCE_PX, "z drifted to {}", back.z);
    }
}

/// A fixed projective case, so the perspective behaviour has a deterministic
/// test and not only a generated one, and so the drift has one number rather
/// than whichever one a generator lands on.
///
/// At `x = 9999, z = 499`, dropping the `w` divide returns 9912.435180867995
/// instead of 9999.0, which is 86.5648 CSS pixels. That cannot hide inside
/// any canvas tolerance this project would accept, 25.1's quarter pixel
/// included.
#[test]
fn the_perspective_divide_is_visible_at_this_tolerance() {
    let p = Pt::<Canvas>::new(9999.0, -9999.0, 499.0);
    let t = canvas_to_world(PERSPECTIVE_W_PER_MM);
    let back = t.inverse().apply(t.apply(p));

    assert!(
        (back.x - p.x).abs() < ROUND_TRIP_TOLERANCE_PX,
        "x drifted to {}",
        back.x
    );
    assert!(
        (back.y - p.y).abs() < ROUND_TRIP_TOLERANCE_PX,
        "y drifted to {}",
        back.y
    );
    assert!(
        (back.z - p.z).abs() < ROUND_TRIP_TOLERANCE_PX,
        "z drifted to {}",
        back.z
    );

    // The same point through the orthographic transform lands somewhere else
    // entirely, which is the evidence that the case above is projective at
    // all rather than accidentally affine.
    //
    // This is the one assertion in the file that IS in millimetres. Both
    // operands are `Pt<World>`, forward transforms rather than round trips,
    // so the separation is a world-space distance.
    let flat = canvas_to_world(0.0).apply(p);
    let curved = t.apply(p);
    let separation = (flat.x - curved.x)
        .abs()
        .max((flat.y - curved.y).abs())
        .max((flat.z - curved.z).abs());
    assert!(
        separation > 1.0,
        "the perspective and orthographic transforms differ by only {separation} mm here, \
         so this case does not exercise the w divide"
    );
}
