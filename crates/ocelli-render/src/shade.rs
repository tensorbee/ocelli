//! Fragment shade for the composite volume ray, F-061.
//!
//! The default is ambient 1 and diffuse 0, so a viewport that never sets a
//! shade keeps the F-060 colour. A caller pair must be finite and each weight
//! must lie in `0.0..=1.0`. The fields stay private so a later write cannot
//! bypass that check. The plan sketch showed public fields. A public field
//! would let a caller store NaN after `from_weights` returned.
//!
//! The equation itself lives in `raycast.wgsl`. This module only carries the
//! weights and the spacing predicate the bind checks before it writes them.

/// Why a caller pair was not stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ShadeError {
    /// A weight is NaN or infinite.
    #[error("a shade weight is not finite")]
    NotFinite,
    /// A weight is below 0 or above 1.
    #[error("a shade weight is outside 0 to 1")]
    OutOfRange,
}

/// Ambient and diffuse weights for one volume ray.
///
/// The light direction is not a field. It is the walk axis toward the near
/// end, which the shader owns.
#[derive(Debug, Clone, Copy)]
pub struct ShadeWeights {
    ambient: f32,
    diffuse: f32,
}

impl ShadeWeights {
    /// Ambient 1 and diffuse 0. The shaded colour equals the unlit colour.
    #[must_use]
    pub fn unlit() -> Self {
        Self {
            ambient: 1.0,
            diffuse: 0.0,
        }
    }

    /// Store a caller pair.
    ///
    /// A refusal does not build a value, so the caller's previous weights
    /// stay where the caller left them. Finiteness is tested before range.
    ///
    /// # Errors
    ///
    /// [`ShadeError::NotFinite`] when either weight is not finite.
    /// [`ShadeError::OutOfRange`] when a finite weight is outside `0.0..=1.0`.
    pub fn from_weights(ambient: f32, diffuse: f32) -> Result<Self, ShadeError> {
        if !ambient.is_finite() || !diffuse.is_finite() {
            return Err(ShadeError::NotFinite);
        }
        if !(0.0..=1.0).contains(&ambient) || !(0.0..=1.0).contains(&diffuse) {
            return Err(ShadeError::OutOfRange);
        }
        Ok(Self { ambient, diffuse })
    }

    /// The ambient weight.
    #[must_use]
    pub fn ambient(&self) -> f32 {
        self.ambient
    }

    /// The diffuse weight.
    #[must_use]
    pub fn diffuse(&self) -> f32 {
        self.diffuse
    }

    /// Little-endian bytes for the shade uniform.
    ///
    /// `ambient` occupies bytes 0 to 3. `diffuse` occupies bytes 4 to 7.
    /// The last eight bytes are zero padding so the uniform is 16 bytes.
    #[must_use]
    pub fn uniform_bytes(&self) -> [u8; 16] {
        let mut bytes = [0_u8; 16];
        write_f32(&mut bytes, 0, self.ambient);
        write_f32(&mut bytes, 1, self.diffuse);
        bytes
    }

    pub(crate) fn same(&self, other: &Self) -> bool {
        self.ambient.to_bits() == other.ambient.to_bits()
            && self.diffuse.to_bits() == other.diffuse.to_bits()
    }
}

/// Spacing the ray may use as millimetres per voxel.
///
/// Each axis must be finite and greater than zero. A zero spacing would make
/// the central difference undefined.
#[must_use]
pub(crate) fn spacing_mm_ok(spacing: [f32; 3]) -> bool {
    spacing.iter().all(|axis| axis.is_finite() && *axis > 0.0)
}

fn write_f32(bytes: &mut [u8; 16], index: usize, value: f32) {
    let start = index.saturating_mul(4);
    let end = start.saturating_add(4);
    if let Some(slot) = bytes.get_mut(start..end) {
        slot.copy_from_slice(&value.to_le_bytes());
    }
}

#[cfg(test)]
mod tests {
    use super::{ShadeError, ShadeWeights, spacing_mm_ok};

    #[test]
    fn unlit_bytes_are_ambient_one_and_diffuse_zero() {
        let weights = ShadeWeights::unlit();
        assert_eq!(weights.ambient().to_bits(), 1.0_f32.to_bits());
        assert_eq!(weights.diffuse().to_bits(), 0.0_f32.to_bits());
        let bytes = weights.uniform_bytes();
        let Some(ambient) = bytes.get(0..4) else {
            unreachable!("16 bytes");
        };
        let Some(diffuse) = bytes.get(4..8) else {
            unreachable!("16 bytes");
        };
        let Ok(ambient) = <[u8; 4]>::try_from(ambient) else {
            unreachable!("four bytes");
        };
        let Ok(diffuse) = <[u8; 4]>::try_from(diffuse) else {
            unreachable!("four bytes");
        };
        assert_eq!(f32::from_le_bytes(ambient).to_bits(), 1.0_f32.to_bits());
        assert_eq!(f32::from_le_bytes(diffuse).to_bits(), 0.0_f32.to_bits());
    }

    #[test]
    fn a_bad_weight_does_not_build_a_value() {
        assert!(matches!(
            ShadeWeights::from_weights(f32::NAN, 0.0),
            Err(ShadeError::NotFinite)
        ));
        assert!(matches!(
            ShadeWeights::from_weights(0.0, f32::INFINITY),
            Err(ShadeError::NotFinite)
        ));
        assert!(matches!(
            ShadeWeights::from_weights(1.1, 0.0),
            Err(ShadeError::OutOfRange)
        ));
        assert!(matches!(
            ShadeWeights::from_weights(0.0, -0.1),
            Err(ShadeError::OutOfRange)
        ));
        let previous = ShadeWeights::unlit();
        assert_eq!(previous.ambient().to_bits(), 1.0_f32.to_bits());
        assert_eq!(previous.diffuse().to_bits(), 0.0_f32.to_bits());
    }

    #[test]
    fn spacing_must_be_finite_and_positive() {
        assert!(spacing_mm_ok([1.0, 1.0, 1.0]));
        assert!(!spacing_mm_ok([0.0, 1.0, 1.0]));
        assert!(!spacing_mm_ok([-1.0, 1.0, 1.0]));
        assert!(!spacing_mm_ok([f32::NAN, 1.0, 1.0]));
        assert!(!spacing_mm_ok([1.0, f32::INFINITY, 1.0]));
    }
}
