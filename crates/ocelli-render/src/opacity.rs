//! 256-entry opacity table for the composite volume ray, F-060.
//!
//! The default is every entry `0.5`, which is the integral F-059 locked.
//! The ramp is the second preset so a test can see that the table is read.
//! A caller table is 256 finite entries, each in `0.0..=1.0`.
//!
//! The shader addresses the table with `table_index(display, 255)`, the same
//! rule as [`crate::colour::display_index`]. The bytes are 256 `f32` values
//! packed in order. The uniform stores them as `array<vec4<f32>, 64>` because
//! a uniform array stride has to be a multiple of 16 bytes. That packing is
//! still 1024 bytes and it is not a texture.

/// `Flat` or `Ramp`. No clinical name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpacityPreset {
    /// Every entry is `0.5`.
    Flat,
    /// Entry `i` is `i / 255`.
    Ramp,
}

/// Why a caller table was not stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum OpacityError {
    /// The slice does not have 256 entries.
    #[error("an opacity table has 256 entries")]
    Length,
    /// An entry is NaN or infinite.
    #[error("an opacity entry is not finite")]
    NotFinite,
    /// An entry is below 0 or above 1.
    #[error("an opacity entry is outside 0 to 1")]
    OutOfRange,
}

/// One opacity per display code from 0 to 255.
#[derive(Debug, Clone, Copy)]
pub struct OpacityTable {
    entries: [f32; 256],
}

impl OpacityTable {
    /// The preset named by `id`.
    #[must_use]
    pub fn preset(id: OpacityPreset) -> Self {
        let entries = match id {
            OpacityPreset::Flat => [0.5; 256],
            OpacityPreset::Ramp => ramp_entries(),
        };
        Self { entries }
    }

    /// Copy a caller table.
    ///
    /// A refusal does not build a table, so the caller's previous value stays
    /// where the caller left it.
    ///
    /// # Errors
    ///
    /// [`OpacityError::Length`] when `entries` is not 256 long.
    /// [`OpacityError::NotFinite`] when any entry is not finite.
    /// [`OpacityError::OutOfRange`] when a finite entry is outside `0.0..=1.0`.
    /// Each entry is tested for finiteness before its range.
    pub fn from_entries(entries: &[f32]) -> Result<Self, OpacityError> {
        if entries.len() != 256 {
            return Err(OpacityError::Length);
        }
        let mut stored = [0.0; 256];
        for (index, value) in entries.iter().copied().enumerate() {
            if !value.is_finite() {
                return Err(OpacityError::NotFinite);
            }
            if !(0.0..=1.0).contains(&value) {
                return Err(OpacityError::OutOfRange);
            }
            if let Some(slot) = stored.get_mut(index) {
                *slot = value;
            }
        }
        Ok(Self { entries: stored })
    }

    /// The 256 entries, index 0 first.
    #[must_use]
    pub fn entries(&self) -> &[f32; 256] {
        &self.entries
    }

    /// Little-endian `f32` bytes for the opacity uniform.
    ///
    /// Entry `i` occupies bytes `4 * i`. Four consecutive entries are one
    /// `vec4` in the shader. The length is 1024.
    #[must_use]
    pub fn uniform_bytes(&self) -> [u8; 1024] {
        let mut bytes = [0_u8; 1024];
        for (index, value) in self.entries.iter().enumerate() {
            let start = index.saturating_mul(4);
            let end = start.saturating_add(4);
            if let Some(slot) = bytes.get_mut(start..end) {
                slot.copy_from_slice(&value.to_le_bytes());
            }
        }
        bytes
    }

    pub(crate) fn same(&self, other: &Self) -> bool {
        self.entries
            .iter()
            .zip(other.entries.iter())
            .all(|(left, right)| left.to_bits() == right.to_bits())
    }
}

fn ramp_entries() -> [f32; 256] {
    let mut entries = [0.0; 256];
    for (index, entry) in entries.iter_mut().enumerate() {
        let Ok(numer) = u8::try_from(index) else {
            continue;
        };
        *entry = f32::from(numer) / 255.0;
    }
    entries
}

#[cfg(test)]
mod tests {
    use super::{OpacityError, OpacityPreset, OpacityTable};

    #[test]
    fn display_200_addresses_entry_200() {
        let index = crate::colour::display_index(200.0, 255);
        assert_eq!(index, 200);
        let table = OpacityTable::preset(OpacityPreset::Ramp);
        let at = usize::try_from(index).unwrap_or(0);
        let got = table.entries().get(at).copied().unwrap_or(1.0);
        assert_eq!(got.to_bits(), (200.0_f32 / 255.0).to_bits());
    }

    #[test]
    fn entry_200_occupies_bytes_800() {
        let mut entries = [0.0_f32; 256];
        if let Some(slot) = entries.get_mut(200) {
            *slot = 0.25;
        }
        let Ok(table) = OpacityTable::from_entries(&entries) else {
            unreachable!("256 finite entries in range");
        };
        let bytes = table.uniform_bytes();
        let Some(chunk) = bytes.get(800..804) else {
            unreachable!("1024 bytes");
        };
        let Ok(array) = <[u8; 4]>::try_from(chunk) else {
            unreachable!("four bytes");
        };
        assert_eq!(f32::from_le_bytes(array).to_bits(), 0.25_f32.to_bits());
        assert!(matches!(
            OpacityTable::from_entries(&[0.0; 257]),
            Err(OpacityError::Length)
        ));
    }
}
