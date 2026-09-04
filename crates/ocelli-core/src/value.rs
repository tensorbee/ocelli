//! Value spaces. HLD section 16.1.
//!
//! The same trick as [`crate::space`], applied to pixel values, and for the
//! same reason: the LUT chain is a sequence of transformations whose stages
//! are easy to apply out of order. You cannot accidentally window a stored
//! value, and a reviewer can see the stage from the type.
//!
//! There is deliberately no conversion between the three. `Stored` becomes
//! `Modality` under the modality LUT and `Modality` becomes `Display` under
//! the VOI LUT, and HLD section 18 requires that arithmetic to exist exactly
//! once, in `ocelli-pixel`. A `From` here would be a second copy of it.

/// A raw value from Pixel Data (7FE0,0010), after unpacking and any signed
/// interpretation, and before any LUT.
#[derive(Clone, Copy, Debug)]
pub struct Stored(pub f32);

/// A value after the modality LUT, which is Hounsfield units for CT.
#[derive(Clone, Copy, Debug)]
pub struct Modality(pub f32);

/// A value after the VOI LUT, in the output range `[ymin, ymax]`.
#[derive(Clone, Copy, Debug)]
pub struct Display(pub f32);

#[cfg(test)]
mod tests {
    use super::{Display, Modality, Stored};

    /// Each newtype carries its field through unchanged.
    ///
    /// Be honest about what this is worth: a tuple struct with a public field
    /// cannot round or clamp on construction, so no change to `value.rs` that
    /// still compiles can make this assertion false. It is a shape check, not
    /// a behavioural test, and it is here so that a later story which gives
    /// one of these a real constructor has an existing assertion to break.
    ///
    /// The claim in HLD section 16.1 that is actually load bearing, that you
    /// cannot accidentally window a stored value, is not observable at run
    /// time at all. It is guarded by
    /// `tests/ui/value_spaces_do_not_interconvert.rs`.
    #[test]
    fn each_newtype_carries_its_field_unchanged() {
        // -1024.0 is the air value of a CT stored pixel with the usual
        // rescale, and it is exactly representable in f32, so this is an
        // identity check and not a tolerance one.
        assert!(Stored(-1024.0).0.to_bits() == (-1024.0f32).to_bits());
        assert!(Modality(-1024.0).0.to_bits() == (-1024.0f32).to_bits());
        assert!(Display(255.0).0.to_bits() == 255.0f32.to_bits());
    }

    /// All three are `Copy`, so a value can be read twice without a move.
    /// The LUT chain passes these by value on a hot path and must not be
    /// forced to clone.
    #[test]
    fn each_newtype_is_copy() {
        let s = Stored(1.5);
        let m = Modality(2.5);
        let d = Display(3.5);
        let again = (s, m, d);
        assert!(s.0.to_bits() == again.0.0.to_bits());
        assert!(m.0.to_bits() == again.1.0.to_bits());
        assert!(d.0.to_bits() == again.2.0.to_bits());
    }
}
