//! Modality and VOI LUT stages from DICOM PS3.3 C.11.

use alloc::vec::Vec;
use glam::Vec2;
use ocelli_core::{Display, Modality, Stored};

use crate::PixelError;

/// Validated LUT Descriptor and its setup-time-owned LUT Data.
#[derive(Clone, Debug, PartialEq)]
pub struct LutDescriptor {
    inputs: Vec<f32>,
    values: Vec<f32>,
    bits_per_entry: u8,
}

impl LutDescriptor {
    /// Validate the DICOM three-value LUT Descriptor against its LUT Data.
    ///
    /// An entry count of zero means 65,536 entries. The first mapped input is
    /// signed or unsigned because the descriptor's VR follows the input pixel
    /// representation.
    pub fn new(
        entry_count: u16,
        first_mapped_input: i32,
        bits_per_entry: u8,
        values: Vec<f32>,
    ) -> Result<Self, PixelError> {
        let expected = if entry_count == 0 {
            65_536
        } else {
            usize::from(entry_count)
        };
        if values.len() != expected {
            return Err(PixelError::InvalidLutLength);
        }
        if !matches!(bits_per_entry, 8 | 16) {
            return Err(PixelError::InvalidLutBits);
        }
        if !(-32_768..=65_535).contains(&first_mapped_input) {
            return Err(PixelError::InvalidLutFirstInput);
        }
        if !values.iter().all(|value| value.is_finite()) {
            return Err(PixelError::NonFiniteLutValue);
        }
        let largest_value = match bits_per_entry {
            8 => 255.0,
            16 => 65_535.0,
            _ => return Err(PixelError::InvalidLutBits),
        };
        if values
            .iter()
            .any(|value| *value < 0.0 || *value > largest_value)
        {
            return Err(PixelError::LutValueOutOfRange);
        }
        if values.iter().any(|value| *value % 1.0 > 0.0) {
            return Err(PixelError::NonIntegerLutValue);
        }

        let mut inputs = Vec::with_capacity(expected);
        let mut input = descriptor_input_to_f32(first_mapped_input)?;
        for _ in 0..expected {
            inputs.push(input);
            input += 1.0;
        }
        Ok(Self {
            inputs,
            values,
            bits_per_entry,
        })
    }

    /// Bits per LUT entry from the descriptor.
    pub const fn bits_per_entry(&self) -> u8 {
        self.bits_per_entry
    }

    fn lookup(&self, input: f32) -> f32 {
        let after = self.inputs.partition_point(|candidate| *candidate <= input);
        let index = after.saturating_sub(1).min(self.values.len() - 1);
        self.values.get(index).copied().unwrap_or(0.0)
    }
}

fn descriptor_input_to_f32(value: i32) -> Result<f32, PixelError> {
    if value < 0 {
        let magnitude =
            u16::try_from(value.unsigned_abs()).map_err(|_| PixelError::InvalidLutFirstInput)?;
        Ok(-f32::from(magnitude))
    } else {
        let magnitude = u16::try_from(value).map_err(|_| PixelError::InvalidLutFirstInput)?;
        Ok(f32::from(magnitude))
    }
}

/// Apply rescale slope and intercept, PS3.3 C.11.1.
pub fn modality(stored: Stored, slope: f32, intercept: f32) -> Modality {
    Modality(stored.0 * slope + intercept)
}

/// A validated Modality LUT Sequence or rescale selection.
#[derive(Clone, Debug, PartialEq)]
pub struct ModalityTransform(ModalitySelection);

#[derive(Clone, Debug, PartialEq)]
enum ModalitySelection {
    /// Modality LUT Sequence. This takes precedence over rescale.
    Lut(LutDescriptor),
    /// Rescale Slope and Rescale Intercept.
    Rescale {
        /// Rescale Slope.
        slope: f32,
        /// Rescale Intercept.
        intercept: f32,
    },
}

impl ModalityTransform {
    /// Select a sequence when present, otherwise validate slope and intercept.
    pub fn new(
        lut: Option<LutDescriptor>,
        slope: Option<f32>,
        intercept: Option<f32>,
    ) -> Result<Self, PixelError> {
        if let Some(lut) = lut {
            return Ok(Self(ModalitySelection::Lut(lut)));
        }
        let (Some(slope), Some(intercept)) = (slope, intercept) else {
            return Err(PixelError::MissingRescale);
        };
        if !slope.is_finite() || !intercept.is_finite() {
            return Err(PixelError::InvalidRescale);
        }
        Ok(Self(ModalitySelection::Rescale { slope, intercept }))
    }

    /// Map one Stored value to Modality space.
    pub fn apply(&self, stored: Stored) -> Modality {
        match &self.0 {
            ModalitySelection::Lut(lut) => Modality(lut.lookup(stored.0)),
            ModalitySelection::Rescale { slope, intercept } => modality(stored, *slope, *intercept),
        }
    }

    /// Map caller-provided slices without allocating.
    pub fn map_into(
        &self,
        source: &[Stored],
        destination: &mut [Modality],
    ) -> Result<(), PixelError> {
        if source.len() != destination.len() {
            return Err(PixelError::DestinationLength);
        }
        for (stored, output) in source.iter().copied().zip(destination.iter_mut()) {
            *output = self.apply(stored);
        }
        Ok(())
    }
}

/// Window function named by VOI LUT Function.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VoiFunction {
    /// PS3.3 C.11.2.1.2, including `c - 0.5` and `w - 1`.
    Linear,
    /// PS3.3 C.11.2.1.3.2.
    LinearExact,
    /// PS3.3 C.11.2.1.3.1.
    Sigmoid,
}

/// A validated VOI LUT Sequence or window selection.
#[derive(Clone, Debug, PartialEq)]
pub struct VoiTransform(VoiSelection);

#[derive(Clone, Debug, PartialEq)]
enum VoiSelection {
    /// VOI LUT Sequence. This takes precedence over window values.
    Lut(LutDescriptor),
    /// One selected centre-width pair and function.
    Window {
        /// Window Center.
        center: f32,
        /// Window Width.
        width: f32,
        /// VOI LUT Function.
        function: VoiFunction,
        /// Minimum output value.
        ymin: f32,
        /// Maximum output value.
        ymax: f32,
    },
}

impl VoiTransform {
    /// Select a sequence when present, otherwise validate one window pair.
    pub fn new(
        lut: Option<LutDescriptor>,
        centers: &[f32],
        widths: &[f32],
        selected: usize,
        function: VoiFunction,
        ymin: f32,
        ymax: f32,
    ) -> Result<Self, PixelError> {
        if let Some(lut) = lut {
            return Ok(Self(VoiSelection::Lut(lut)));
        }
        if centers.is_empty() || centers.len() != widths.len() {
            return Err(PixelError::MismatchedWindowMultiplicity);
        }
        let (Some(center), Some(width)) = (centers.get(selected), widths.get(selected)) else {
            return Err(PixelError::WindowIndexOutOfRange);
        };
        if !ymin.is_finite() || !ymax.is_finite() || ymin > ymax {
            return Err(PixelError::InvalidDisplayRange);
        }
        let width_is_valid = match function {
            VoiFunction::Linear => *width >= 1.0,
            VoiFunction::LinearExact | VoiFunction::Sigmoid => *width > 0.0,
        };
        if !center.is_finite() || !width.is_finite() || !width_is_valid {
            return Err(PixelError::InvalidWindow);
        }
        Ok(Self(VoiSelection::Window {
            center: *center,
            width: *width,
            function,
            ymin,
            ymax,
        }))
    }

    /// Map one Modality value to Display space.
    pub fn apply(&self, modality: Modality) -> Display {
        match &self.0 {
            VoiSelection::Lut(lut) => Display(lut.lookup(modality.0)),
            VoiSelection::Window {
                center,
                width,
                function,
                ymin,
                ymax,
            } => Display(apply_window(
                modality.0, *center, *width, *function, *ymin, *ymax,
            )),
        }
    }

    /// Map caller-provided slices without allocating.
    pub fn map_into(
        &self,
        source: &[Modality],
        destination: &mut [Display],
    ) -> Result<(), PixelError> {
        if source.len() != destination.len() {
            return Err(PixelError::DestinationLength);
        }
        for (modality, output) in source.iter().copied().zip(destination.iter_mut()) {
            *output = self.apply(modality);
        }
        Ok(())
    }
}

fn apply_window(
    x: f32,
    center: f32,
    width: f32,
    function: VoiFunction,
    ymin: f32,
    ymax: f32,
) -> f32 {
    let range = ymax - ymin;
    match function {
        VoiFunction::Linear => {
            let adjusted_center = center - 0.5;
            let adjusted_width = width - 1.0;
            let lower = adjusted_center - adjusted_width / 2.0;
            let upper = adjusted_center + adjusted_width / 2.0;
            if x <= lower {
                ymin
            } else if x > upper {
                ymax
            } else {
                ((x - adjusted_center) / adjusted_width + 0.5) * range + ymin
            }
        }
        VoiFunction::LinearExact => {
            let lower = center - width / 2.0;
            let upper = center + width / 2.0;
            if x <= lower {
                ymin
            } else if x > upper {
                ymax
            } else {
                ((x - center) / width + 0.5) * range + ymin
            }
        }
        VoiFunction::Sigmoid => {
            // `ocelli-pixel` is `no_std`, where scalar `f32::exp` is not
            // available. glam is already configured with its libm backend and
            // exposes the same operation without adding a second math policy.
            let exponent = Vec2::splat(-4.0 * (x - center) / width).exp().x;
            range / (1.0 + exponent) + ymin
        }
    }
}

#[cfg(test)]
mod tests {
    use alloc::vec;

    use super::{LutDescriptor, ModalityTransform, VoiFunction, VoiTransform};
    use crate::{PixelError, lut::modality};
    use ocelli_core::{Modality, Stored};

    #[test]
    fn malformed_lut_descriptors_are_refused() {
        assert_eq!(
            LutDescriptor::new(2, 0, 16, vec![1.0]),
            Err(PixelError::InvalidLutLength)
        );
        assert_eq!(
            LutDescriptor::new(1, 0, 12, vec![1.0]),
            Err(PixelError::InvalidLutBits)
        );
        assert_eq!(
            LutDescriptor::new(1, 0, 16, vec![f32::NAN]),
            Err(PixelError::NonFiniteLutValue)
        );
        assert_eq!(
            LutDescriptor::new(1, 65_536, 16, vec![1.0]),
            Err(PixelError::InvalidLutFirstInput)
        );
    }

    #[test]
    fn malformed_rescale_and_window_evidence_is_refused() {
        assert_eq!(
            ModalityTransform::new(None, Some(1.0), None),
            Err(PixelError::MissingRescale)
        );
        assert_eq!(
            ModalityTransform::new(None, Some(f32::NAN), Some(0.0)),
            Err(PixelError::InvalidRescale)
        );
        assert_eq!(
            VoiTransform::new(
                None,
                &[1.0, 2.0],
                &[3.0],
                0,
                VoiFunction::Linear,
                0.0,
                255.0
            ),
            Err(PixelError::MismatchedWindowMultiplicity)
        );
        assert_eq!(
            VoiTransform::new(None, &[1.0], &[3.0], 1, VoiFunction::Linear, 0.0, 255.0),
            Err(PixelError::WindowIndexOutOfRange)
        );
    }

    #[test]
    fn mapping_refuses_mismatched_destination_before_writing() {
        let transform = ModalityTransform::new(None, Some(2.0), Some(1.0));
        assert!(transform.is_ok());
        let Ok(transform) = transform else { return };
        let mut output = [Modality(77.0)];
        assert_eq!(
            transform.map_into(&[Stored(1.0), Stored(2.0)], &mut output),
            Err(PixelError::DestinationLength)
        );
        assert_eq!(output[0].0.to_bits(), 77.0_f32.to_bits());
    }

    #[test]
    fn free_modality_function_has_the_hld_signature_and_formula() {
        assert_eq!(
            modality(Stored(5.0), 2.0, -3.0).0.to_bits(),
            7.0_f32.to_bits()
        );
    }
}
