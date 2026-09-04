//! Coordinate spaces and the transforms between them. HLD section 16.
//!
//! Cornerstone represents canvas points, world points and voxel indices all
//! as `number[]`, and mixing them is a silent, common and expensive bug. The
//! marker parameter on [`Pt`] and the two parameters on [`Transform`] make
//! that mistake a compile error instead.
//!
//! The payoff, in section 16's words: `Transform<Canvas, World>` composes
//! with `Transform<World, Index>` and will not compose with anything else.
//!
//! # Deviation D-08
//!
//! Section 16 writes the three marker spaces bare, as `pub enum Canvas {}`.
//! They derive here instead. A `derive` on a generic struct bounds the
//! parameter, so section 16's own `#[derive(Debug, PartialEq)]` on `Pt<S>`
//! expands to `impl<S: Debug> Debug for Pt<S>`, and `Pt<Canvas>` satisfies
//! neither trait while `Canvas` is bare. `PhantomData` implements both for
//! any `S` without a bound, which is why the definition compiles and only the
//! call site fails. Deriving on the markers leaves section 16's `Pt` listing
//! character for character as written.
//!
//! D-08 has a consequence worth naming here, because it is easy to miss:
//! giving the markers `Clone` and `Copy` also retires the trap section 16's
//! own `NOTE` describes. The hand-written `Clone` and `Copy` impls on [`Pt`]
//! and [`Transform`] are therefore kept for a different reason than section 16
//! gives, and the comment above them says which.

use core::marker::PhantomData;

use glam::{DMat4, DVec3};

/// CSS pixels inside a viewport element. Origin top-left, y increases down.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Canvas {}

/// DICOM patient coordinate system (LPS), millimetres.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum World {}

/// Voxel indices within a volume. Origin at voxel (0,0,0).
///
/// `x` is the column index `i` and `y` is the row index `j`, which is the
/// order DICOM PS3.3 C.7.6.2.1.1 writes them in. `z` is the slice index, and
/// that one is this crate's own: C.7.6.2.1.1's column vector is `(i, j, 0, 1)`
/// with a literal zero in the third position, because the equation is
/// in-plane and says nothing about stepping between slices.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Index {}

/// A point in the space `S`. Two points in different spaces have different
/// types and cannot be passed to each other's transforms.
#[derive(Debug, PartialEq)]
pub struct Pt<S> {
    /// First component. CSS pixels, millimetres or a column index, by space.
    pub x: f64,
    /// Second component. CSS pixels, millimetres or a row index, by space.
    pub y: f64,
    /// Third component. Depth, millimetres or a slice index, by space.
    pub z: f64,
    _s: PhantomData<S>,
}

// HLD section 16 carries this note above the two impls:
//
//     NOTE: derive(Clone, Copy) would add an S: Clone bound that the marker
//     types do not satisfy. Implement by hand.
//
// It was true of section 16's tree and it is NOT true of this one. D-08 gives
// `Canvas`, `World` and `Index` `Clone` and `Copy` where they are declared
// above, so they do satisfy that bound now and the derived form compiles for
// all three.
//
// The hand-written impls stay for a different and better reason: they are
// unconditional. `impl<S> Copy for Pt<S>` holds whatever a future marker does
// or does not derive, where `#[derive(Copy)]` would quietly make a point's
// `Copy`-ness contingent on the marker's, and a marker is a type nobody ever
// constructs a value of.
impl<S> Clone for Pt<S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S> Copy for Pt<S> {}

impl<S> Pt<S> {
    /// A point from its three components. `const`, so a point can be a
    /// constant.
    pub const fn new(x: f64, y: f64, z: f64) -> Self {
        Self {
            x,
            y,
            z,
            _s: PhantomData,
        }
    }
}

/// A transform from space `A` to space `B`.
///
/// The matrix is a full 4x4, not an affine 3x4, because a PERSPECTIVE
/// viewport produces a projective transform and [`Transform::apply`] has to
/// be correct for both.
#[derive(Debug)]
pub struct Transform<A, B> {
    m: DMat4,
    _p: PhantomData<(A, B)>,
}

// Hand-implemented for the reason given above `impl<S> Clone for Pt<S>`, and
// not for the reason section 16's note gives. `#[derive(Clone, Copy)]` here
// would compile, because the markers derive both, and it would bound `A` and
// `B` on a pair of types that exist only to be named. These impls do not.
impl<A, B> Clone for Transform<A, B> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<A, B> Copy for Transform<A, B> {}

impl<A, B> Transform<A, B> {
    /// A transform from a column-major 4x4 matrix.
    ///
    /// The caller owns the meaning of the matrix. For an image plane that
    /// means DICOM PS3.3 C.7.6.2.1.1, where the first column is the row
    /// direction cosine scaled by `PixelSpacing[1]` and the second is the
    /// column direction cosine scaled by `PixelSpacing[0]`.
    pub const fn from_mat4(m: DMat4) -> Self {
        Self { m, _p: PhantomData }
    }

    /// Map a point from `A` into `B`.
    ///
    /// The point is transformed as `(x, y, z, 1)` and divided by the
    /// resulting `w`. For an affine transform `w` is exactly `1.0` and
    /// division by `1.0` is exact in IEEE 754, so this is the affine answer
    /// as well as the projective one. The affine-only form would be silently
    /// wrong under a PERSPECTIVE viewport, which is the failure mode this
    /// project exists to avoid.
    pub fn apply(&self, p: Pt<A>) -> Pt<B> {
        let v = self.m.project_point3(DVec3::new(p.x, p.y, p.z));
        Pt::new(v.x, v.y, v.z)
    }

    /// The transform back from `B` to `A`.
    ///
    /// Nothing here checks that the matrix is invertible, and glam's
    /// `inverse` returns non-finite components for a singular one rather than
    /// failing. That is deliberate: a constructor which guarantees an
    /// invertible matrix belongs to whoever builds a camera, and no caller
    /// today would have anything useful to do with an error. A caller must
    /// not assume this method checked.
    ///
    /// That is not folklore. `inverse_does_not_check_invertibility` below
    /// executes it against a singular matrix, so if a glam release inside the
    /// `0.30` caret range changes the behaviour, a test says so rather than
    /// F-023 discovering it.
    ///
    /// One qualification, because this workspace denies `panic`, `unwrap` and
    /// `expect` on the grounds that a panic poisons a wasm instance. glam's
    /// `inverse` carries a `glam_assert!` on the determinant. Read from
    /// `glam-0.30.10/src/macros.rs`, that macro expands to `assert!` under
    /// `any(all(debug_assertions, feature = "debug-glam-assert"), feature =
    /// "glam-assert")` and to nothing otherwise, so `glam-assert` panics in
    /// any profile while `debug-glam-assert` panics only in a debug one.
    /// Neither feature is on here, so the non-finite behaviour above is what
    /// this workspace actually gets. If feature unification ever turns one on,
    /// this becomes a panicking path and the test below turns red, which is
    /// the point of it.
    pub fn inverse(&self) -> Transform<B, A> {
        Transform::from_mat4(self.m.inverse())
    }

    /// Compose. `a.then(b)` applies `a` first and then `b`, so the matrix is
    /// `b.m * a.m` and the result carries `a`'s source space and `b`'s
    /// destination space.
    ///
    /// The reversed composition type-checks identically, which is why the
    /// order has a test against a non-commuting pair rather than a comment.
    pub fn then<C>(&self, next: &Transform<B, C>) -> Transform<A, C> {
        Transform::from_mat4(next.m * self.m)
    }
}

/// `identity` is deliberately narrower than the rest of the impl: it is
/// `Transform<S, S>` and not `Transform<A, B>`.
///
/// On `Transform<A, B>` it would be a free cross-space cast.
/// `Transform::<Canvas, World>::identity()` would compile, turn a canvas
/// point into a world point with no arithmetic and nothing for a reviewer to
/// look at, and its own name would assert that no conversion was needed. That
/// is exactly the `number[]` interchange HLD section 16 exists to make
/// impossible, in the terse form a caller reaches for when the real transform
/// does not exist yet.
///
/// `from_mat4` still accepts any matrix, so this closes no hole. It moves the
/// cost: `from_mat4(DMat4::IDENTITY)` makes the caller name a matrix, which is
/// a deliberate act appearing in a diff, where `identity()` reads as a
/// statement that the two spaces were the same all along.
impl<S> Transform<S, S> {
    /// The transform that leaves a point where it is.
    pub const fn identity() -> Self {
        Self::from_mat4(DMat4::IDENTITY)
    }
}

#[cfg(test)]
mod tests {
    use super::{Canvas, Index, Pt, Transform, World};
    use glam::{DMat4, DVec3, DVec4};

    /// The bound for a difference between two points, in the units of
    /// whichever space the points are in. Quantities that are not points use
    /// `DIMENSIONLESS_TOLERANCE` below.
    ///
    /// It is deliberately not labelled with a unit, because these tests span
    /// three spaces and HLD section 25.1's geometry bullet covers two of
    /// them, world and canvas, with different numbers:
    ///
    /// - `Pt<World>` is millimetres, and 25.1's world row is 1e-6 mm, so for
    ///   those cases this is that row exactly.
    /// - `Pt<Canvas>` is CSS pixels, and 25.1's canvas row is a quarter pixel.
    ///   1e-6 is far tighter than that, on purpose.
    /// - `Pt<Index>` is voxels, and 25.1 has no row for it at all.
    ///
    /// Every case below is exact arithmetic that should land on the nose, not
    /// a rendered result allowed to drift, so this is a strictness floor and
    /// not a tolerance to relax toward whichever 25.1 row a reader finds
    /// first. Widening it is a design-plan decision, not a fix.
    const SPACE_UNIT_TOLERANCE: f64 = 1e-6;

    /// The bound for quantities that are not points and have no space. A
    /// determinant is the only one today. Same value, separate name, for the
    /// reason the geometry fixture keeps `DIMENSIONLESS_TOLERANCE` separate
    /// from `TOLERANCE_MM`: borrowing a unit is how the confusion above
    /// starts.
    const DIMENSIONLESS_TOLERANCE: f64 = 1e-6;

    #[track_caller]
    fn assert_close<S>(actual: Pt<S>, expected: (f64, f64, f64)) {
        let (ex, ey, ez) = expected;
        assert!(
            (actual.x - ex).abs() < SPACE_UNIT_TOLERANCE,
            "x was {}, wanted {ex}",
            actual.x
        );
        assert!(
            (actual.y - ey).abs() < SPACE_UNIT_TOLERANCE,
            "y was {}, wanted {ey}",
            actual.y
        );
        assert!(
            (actual.z - ez).abs() < SPACE_UNIT_TOLERANCE,
            "z was {}, wanted {ez}",
            actual.z
        );
    }

    /// Requires every trait D-08 adds to a marker space. Instantiating it is
    /// the whole body: if `T` is missing any one of the six, the call below
    /// does not compile.
    ///
    /// `Clone` and `PartialEq` are supertraits of `Copy` and `Eq` and would
    /// come along anyway. They are named explicitly so the bound list is
    /// D-08's derive list read straight across, and so that deleting one from
    /// the derive fails against a bound that names it.
    fn assert_marker_bounds<T>()
    where
        T: core::fmt::Debug + Clone + Copy + PartialEq + Eq + core::hash::Hash,
    {
    }

    /// THE GUARD FOR DEVIATION D-08. Its only job is to fail if D-08 is
    /// reverted, in whole or in part, and it is the only thing in this crate
    /// that does.
    ///
    /// D-08 gives `Canvas`, `World` and `Index` the derive list
    /// `Debug, Clone, Copy, PartialEq, Eq, Hash`, where HLD section 16 writes
    /// them bare as `pub enum Canvas {}`. The justification recorded in
    /// `docs/hld/DEVIATIONS.md` is that a derive on a generic struct bounds
    /// its parameter, so `#[derive(Debug, PartialEq)]` on `Pt<S>` expands to
    /// `impl<S: Debug> Debug for Pt<S>`, and with a bare marker `Pt<Canvas>`
    /// implements neither trait. That record reports the failure as E0369 and
    /// E0277, verified against rustc rather than reasoned about, and the
    /// `assert_eq!` below is the call it means.
    ///
    /// **All six derives are covered, not just the two the record names.**
    /// `assert_marker_bounds` reaches `Clone`, `Copy`, `Eq` and `Hash`, which
    /// nothing else in the crate uses. That matters because this file states
    /// twice, in the module's D-08 block and in the comment above
    /// `impl<S> Clone for Pt<S>`, that the markers having `Clone` and `Copy`
    /// is what retires section 16's `NOTE`. Both sentences are false the
    /// moment those two are deleted, so something has to notice. A partial
    /// revert to `#[derive(Debug, PartialEq)]` used to leave the whole suite
    /// green.
    ///
    /// **If this stops compiling, someone reverted D-08.** The fix is to
    /// restore the derives on the three marker enums above. It is not to
    /// delete this test, and it is not to hand-implement `Debug` and
    /// `PartialEq` on `Pt<S>`, which would put an exact `f64` comparison in
    /// front of the workspace's `float_cmp = "deny"` in the one place an
    /// exact comparison is correct. `docs/hld/DEVIATIONS.md` weighs those two
    /// options and takes this one.
    ///
    /// Without this test the deviation is exercised by nothing: the crate
    /// compiles and all of its tests pass with the derives removed, so a
    /// contributor who notices the source departing from section 16's listing
    /// can delete them, see green, and land it.
    #[test]
    fn d_08_keeps_the_marker_derives_load_bearing() {
        // `Clone`, `Copy`, `Eq` and `Hash` on each of the three markers.
        // Nothing else in the crate needs them, so without these three lines
        // four of D-08's six derives are deletable in silence.
        assert_marker_bounds::<Canvas>();
        assert_marker_bounds::<World>();
        assert_marker_bounds::<Index>();

        let a = Pt::<Canvas>::new(1.5, -2.25, 3.75);
        let b = Pt::<Canvas>::new(1.5, -2.25, 3.75);
        let c = Pt::<Canvas>::new(1.5, -2.25, 4.0);

        // `Pt<Canvas>: Debug`. `assert_eq!` needs `Debug` too, for its
        // failure message, so this is not a separate trait requirement. What
        // it adds is behavioural: it inspects the rendered text, so a `Debug`
        // that compiled but printed no fields would fail here and pass below.
        let rendered = format!("{a:?}");
        assert!(
            rendered.contains("1.5") && rendered.contains("-2.25"),
            "Debug for Pt<Canvas> rendered {rendered}, which does not carry its own fields"
        );

        // `Pt<Canvas>: PartialEq` and `Pt<Canvas>: Debug` together, which is
        // exactly the call the D-08 record cites.
        //
        // This is an exact `f64` comparison and it is deliberate, so please
        // do not raise it again. Both operands are built from identical
        // literals with no arithmetic between construction and comparison, so
        // exact equality is the correct assertion and a tolerance would be
        // the wrong one. The comparison itself lives inside the derived
        // `PartialEq`, which is where `docs/hld/DEVIATIONS.md` argues it
        // belongs, and it is the construct D-08 exists to make possible. The
        // guard cannot exercise `PartialEq` without an equality.
        assert_eq!(a, b);
        // Pinned from the other side too, so a `PartialEq` that always
        // returned true would not satisfy this test.
        assert_ne!(a, c);
    }

    /// `new` is `const`, which this constant proves at compile time, and it
    /// stores its three arguments in the order it received them.
    #[test]
    fn new_stores_its_arguments_in_order() {
        const P: Pt<World> = Pt::new(1.5, -2.25, 3.75);
        assert_close(P, (1.5, -2.25, 3.75));
    }

    /// `Pt` is `Copy`, so one point feeds two transforms without a move. The
    /// compiler checks the `Copy` half on the second use and needs no
    /// assertion for it. The two hand-worked answers are what makes this a
    /// test rather than a restatement: the second call has to see (4, 5, 6)
    /// and not something the first call left behind.
    ///
    /// ```text
    /// p = (4, 5, 6)
    /// scale by 2 in every axis   -> ( 8, 10, 12)
    /// translate x by +10         -> (14,  5,  6)
    /// ```
    #[test]
    fn a_point_is_copy_and_both_uses_see_the_same_point() {
        let p = Pt::<Canvas>::new(4.0, 5.0, 6.0);
        let scale =
            Transform::<Canvas, World>::from_mat4(DMat4::from_scale(DVec3::new(2.0, 2.0, 2.0)));
        let translate = Transform::<Canvas, World>::from_mat4(DMat4::from_translation(DVec3::new(
            10.0, 0.0, 0.0,
        )));

        assert_close(scale.apply(p), (8.0, 10.0, 12.0));
        assert_close(translate.apply(p), (14.0, 5.0, 6.0));
    }

    #[test]
    fn identity_leaves_a_canvas_point_where_it_is() {
        let p = Pt::<Canvas>::new(-11.0, 0.5, 1e4);
        assert_close(
            Transform::<Canvas, Canvas>::identity().apply(p),
            (-11.0, 0.5, 1e4),
        );
    }

    #[test]
    fn identity_leaves_a_world_point_where_it_is() {
        let p = Pt::<World>::new(-45.2, 118.7, -32.5);
        assert_close(
            Transform::<World, World>::identity().apply(p),
            (-45.2, 118.7, -32.5),
        );
    }

    #[test]
    fn identity_leaves_an_index_point_where_it_is() {
        let p = Pt::<Index>::new(255.0, 191.0, 3.0);
        assert_close(
            Transform::<Index, Index>::identity().apply(p),
            (255.0, 191.0, 3.0),
        );
    }

    /// `then` applies the receiver first. Worked by hand on a deliberately
    /// non-commuting pair, a translation and a scale:
    ///
    /// ```text
    /// a = translate x by +10        b = scale x by 2
    /// p = (1, 0, 0)
    /// a.then(b):  a(p) = (11, 0, 0), then b gives (22, 0, 0)
    /// b.then(a):  b(p) = ( 2, 0, 0), then a gives (12, 0, 0)
    /// ```
    ///
    /// 22 and 12 are far apart, so a reversed composition cannot pass.
    #[test]
    fn then_applies_the_receiver_first() {
        let translate = Transform::<Canvas, World>::from_mat4(DMat4::from_translation(DVec3::new(
            10.0, 0.0, 0.0,
        )));
        let scale =
            Transform::<World, Index>::from_mat4(DMat4::from_scale(DVec3::new(2.0, 1.0, 1.0)));

        let p = Pt::<Canvas>::new(1.0, 0.0, 0.0);
        assert_close(translate.then(&scale).apply(p), (22.0, 0.0, 0.0));
    }

    /// The other order, so the test above is pinned from both sides. If
    /// `then` composed backwards this case would report 22.
    #[test]
    fn the_other_composition_order_gives_the_other_answer() {
        let translate = Transform::<World, Index>::from_mat4(DMat4::from_translation(DVec3::new(
            10.0, 0.0, 0.0,
        )));
        let scale =
            Transform::<Canvas, World>::from_mat4(DMat4::from_scale(DVec3::new(2.0, 1.0, 1.0)));

        let p = Pt::<Canvas>::new(1.0, 0.0, 0.0);
        assert_close(scale.then(&translate).apply(p), (12.0, 0.0, 0.0));
    }

    /// `apply` divides by `w`. Worked by hand on a column-major matrix whose
    /// bottom row is not `(0, 0, 0, 1)`:
    ///
    /// ```text
    /// columns  c0 = (2, 0, 0, 0)   c1 = (0, 2, 0, 0)
    ///          c2 = (0, 0, 2, 0.5) c3 = (1, 2, 3, 1)
    /// p = (4, 6, 8)
    /// q = 4*c0 + 6*c1 + 8*c2 + c3
    ///   = (8, 0, 0, 0) + (0, 12, 0, 0) + (0, 0, 16, 4) + (1, 2, 3, 1)
    ///   = (9, 14, 19, 5)
    /// q.xyz / q.w = (9/5, 14/5, 19/5) = (1.8, 2.8, 3.8)
    /// ```
    ///
    /// Without the divide the answer would be (9, 14, 19).
    #[test]
    fn apply_divides_by_the_resulting_w() {
        let t = Transform::<Canvas, World>::from_mat4(DMat4::from_cols(
            DVec4::new(2.0, 0.0, 0.0, 0.0),
            DVec4::new(0.0, 2.0, 0.0, 0.0),
            DVec4::new(0.0, 0.0, 2.0, 0.5),
            DVec4::new(1.0, 2.0, 3.0, 1.0),
        ));
        assert_close(t.apply(Pt::<Canvas>::new(4.0, 6.0, 8.0)), (1.8, 2.8, 3.8));
    }

    /// `inverse` undoes `apply` for an invertible transform, and it also
    /// swaps the spaces, which is what makes `t.inverse().apply(t.apply(p))`
    /// type-check back to the space it started in.
    #[test]
    fn inverse_returns_a_point_to_the_space_it_came_from() {
        let t = Transform::<Canvas, World>::from_mat4(DMat4::from_cols(
            DVec4::new(0.35, 0.0, 0.0, 0.0),
            DVec4::new(0.0, -0.35, 0.0, 0.0),
            DVec4::new(0.0, 0.0, 1.25, 0.0),
            DVec4::new(-45.2, 118.7, -32.5, 1.0),
        ));
        let p = Pt::<Canvas>::new(512.0, 384.0, 7.0);
        let back: Pt<Canvas> = t.inverse().apply(t.apply(p));
        assert_close(back, (512.0, 384.0, 7.0));
    }

    /// `inverse` does not check invertibility, and this executes what the
    /// doc comment on it claims rather than leaving the claim as prose.
    ///
    /// The matrix is the DICOM PS3.3 C.7.6.2.1.1 image-plane matrix, whose
    /// third column is zero because the equation is in-plane, so a
    /// single-slice transform is singular by construction and this is the
    /// realistic case rather than a contrived one. glam returns non-finite
    /// components instead of failing.
    ///
    /// This matters because `glam = "0.30"` is a caret range and not the
    /// exact pin `wgpu` gets, so a patch release could change the behaviour
    /// F-023 is told here not to rely on. If that happens this test says so,
    /// which is the whole reason it exists.
    #[test]
    fn inverse_does_not_check_invertibility() {
        let singular = Transform::<Index, World>::from_mat4(DMat4::from_cols(
            DVec4::new(0.15, -0.16, 0.12, 0.0),
            DVec4::new(0.4, 0.24, -0.18, 0.0),
            DVec4::ZERO,
            DVec4::new(-45.2, 118.7, -32.5, 1.0),
        ));
        // A zero column makes the determinant exactly zero. Asserted, rather
        // than assumed, so the case cannot stop being singular unnoticed.
        assert!(
            singular.m.determinant().abs() < DIMENSIONLESS_TOLERANCE,
            "the fixture matrix is not singular, determinant is {}",
            singular.m.determinant()
        );

        let p = singular
            .inverse()
            .apply(Pt::<World>::new(-45.2, 118.7, -32.5));
        assert!(
            !p.x.is_finite() && !p.y.is_finite() && !p.z.is_finite(),
            "inverse of a singular transform returned finite components \
             ({}, {}, {}), so the doc comment on `inverse` and whatever F-023 \
             was told about it are both out of date",
            p.x,
            p.y,
            p.z
        );
    }
}
