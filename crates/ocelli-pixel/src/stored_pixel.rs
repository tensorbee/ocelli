//! Extraction of DICOM stored values from decoded sample containers.

use ocelli_core::Stored;

use crate::{ImageDimensions, PixelError};

/// Byte order of decoded native sample containers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ByteOrder {
    /// Least significant byte first.
    LittleEndian,
    /// Most significant byte first.
    BigEndian,
}

/// Pixel Representation from the Image Pixel module.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PixelRepresentation {
    /// Unsigned integer samples.
    Unsigned,
    /// Two's-complement signed integer samples.
    Signed,
}

/// Planar Configuration for uncompressed three-sample pixels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlanarConfiguration {
    /// RGBRGB sample order.
    Interleaved,
    /// RRR, then GGG, then BBB sample order.
    Planar,
}

/// Photometric Interpretation retained as evidence on the frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PhotometricInterpretation {
    /// Minimum stored value is displayed white at the presentation stage.
    Monochrome1,
    /// Minimum stored value is displayed black.
    Monochrome2,
    /// Stored values index palette colour LUTs.
    PaletteColor,
    /// RGB colour samples.
    Rgb,
    /// Full-range YCbCr.
    YbrFull,
    /// Full-range YCbCr with 4:2:2 source sampling.
    YbrFull422,
    /// Video-range YCbCr with 4:2:2 source sampling.
    YbrPartial422,
    /// JPEG 2000 irreversible colour transform evidence.
    YbrIct,
    /// JPEG 2000 reversible colour transform evidence.
    YbrRct,
}

impl PhotometricInterpretation {
    const fn expected_samples(self) -> u8 {
        match self {
            Self::Monochrome1 | Self::Monochrome2 | Self::PaletteColor => 1,
            Self::Rgb
            | Self::YbrFull
            | Self::YbrFull422
            | Self::YbrPartial422
            | Self::YbrIct
            | Self::YbrRct => 3,
        }
    }
}

/// Validated sample count, photometric interpretation and planar layout.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SampleLayout {
    /// Number of samples per pixel.
    samples_per_pixel: u8,
    /// Photometric Interpretation, including MONOCHROME1 evidence.
    photometric_interpretation: PhotometricInterpretation,
    /// Planar Configuration for three-sample uncompressed data.
    planar_configuration: Option<PlanarConfiguration>,
}

impl SampleLayout {
    /// Validate Samples per Pixel and Planar Configuration against photometric evidence.
    pub const fn new(
        samples_per_pixel: u8,
        photometric_interpretation: PhotometricInterpretation,
        planar_configuration: Option<PlanarConfiguration>,
    ) -> Result<Self, PixelError> {
        if samples_per_pixel != photometric_interpretation.expected_samples() {
            return Err(PixelError::InvalidSamplesPerPixel);
        }
        if (samples_per_pixel == 1 && planar_configuration.is_some())
            || (samples_per_pixel == 3 && planar_configuration.is_none())
            || (matches!(
                photometric_interpretation,
                PhotometricInterpretation::YbrFull422 | PhotometricInterpretation::YbrPartial422
            ) && !matches!(planar_configuration, Some(PlanarConfiguration::Interleaved)))
        {
            return Err(PixelError::InvalidPlanarConfiguration);
        }
        Ok(Self {
            samples_per_pixel,
            photometric_interpretation,
            planar_configuration,
        })
    }

    /// Number of samples per pixel.
    pub const fn samples_per_pixel(self) -> u8 {
        self.samples_per_pixel
    }

    /// Photometric Interpretation retained for later presentation or colour stages.
    pub const fn photometric_interpretation(self) -> PhotometricInterpretation {
        self.photometric_interpretation
    }

    /// Planar Configuration for three-sample uncompressed data.
    pub const fn planar_configuration(self) -> Option<PlanarConfiguration> {
        self.planar_configuration
    }

    /// Stored samples on the wire for each pixel.
    ///
    /// This is **not** Samples per Pixel for a 4:2:2 photometric
    /// interpretation. PS3.3 C.7.6.3.1.2 subsamples the chroma two to one
    /// horizontally and stores each pair of pixels as `Y1 Y2 Cb Cr`, so a
    /// frame is `Rows * Columns * 2` bytes and not `* 3`. Samples per Pixel
    /// stays 3, because that is what the data set carries and what
    /// [`SampleLayout::new`] validated.
    ///
    /// A reader that sizes a buffer from Samples per Pixel alone over-reads by
    /// half a frame, which is the trap
    /// `scripts/tests/test_corpus_synth.py::test_ybr_full_422_frame_is_two_bytes_per_pixel`
    /// asserts about the corpus.
    ///
    /// **Public despite having no caller outside this crate today.** The
    /// alternative leaves [`SampleLayout::samples_per_pixel`] as the only
    /// public answer to "how many stored samples does a pixel have", and that
    /// answer is wrong for exactly the two interpretations where getting it
    /// wrong over-reads. A correct fact that is hard to reach loses to an
    /// incorrect one that is easy to reach.
    pub const fn stored_samples_per_pixel(self) -> u8 {
        match self.photometric_interpretation {
            PhotometricInterpretation::YbrFull422 | PhotometricInterpretation::YbrPartial422 => 2,
            _ => self.samples_per_pixel,
        }
    }

    const fn is_subsampled(self) -> bool {
        matches!(
            self.photometric_interpretation,
            PhotometricInterpretation::YbrFull422 | PhotometricInterpretation::YbrPartial422
        )
    }
}

/// Validated sample-container width, meaningful bits and signedness.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StoredBits {
    /// Container width in bits.
    bits_allocated: u8,
    /// Number of meaningful stored bits.
    bits_stored: u8,
    /// Most significant meaningful bit position.
    high_bit: u8,
    /// Signedness of stored samples.
    pixel_representation: PixelRepresentation,
}

impl StoredBits {
    /// Validate Bits Allocated, Bits Stored, High Bit and Pixel Representation.
    pub const fn new(
        bits_allocated: u8,
        bits_stored: u8,
        high_bit: u8,
        pixel_representation: PixelRepresentation,
    ) -> Result<Self, PixelError> {
        if !matches!(bits_allocated, 8 | 16 | 32) {
            return Err(PixelError::UnsupportedBitsAllocated);
        }
        if bits_stored == 0 || bits_stored > bits_allocated {
            return Err(PixelError::InvalidBitsStored);
        }
        if high_bit != bits_stored - 1 {
            return Err(PixelError::InvalidHighBit);
        }
        Ok(Self {
            bits_allocated,
            bits_stored,
            high_bit,
            pixel_representation,
        })
    }

    /// Container width in bits.
    pub const fn bits_allocated(self) -> u8 {
        self.bits_allocated
    }

    /// Number of meaningful stored bits.
    pub const fn bits_stored(self) -> u8 {
        self.bits_stored
    }

    /// Most significant meaningful bit position.
    pub const fn high_bit(self) -> u8 {
        self.high_bit
    }

    /// Signedness of stored samples.
    pub const fn pixel_representation(self) -> PixelRepresentation {
        self.pixel_representation
    }
}

/// Validated Image Pixel evidence for one decoded frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StoredPixelDescription {
    /// Rows and Columns.
    pub dimensions: ImageDimensions,
    /// Sample and photometric layout.
    pub layout: SampleLayout,
    /// Stored sample-container description.
    pub bits: StoredBits,
}

impl StoredPixelDescription {
    /// Assemble separately validated Image Pixel evidence.
    pub const fn new(dimensions: ImageDimensions, layout: SampleLayout, bits: StoredBits) -> Self {
        Self {
            dimensions,
            layout,
            bits,
        }
    }

    /// Number of stored samples in one frame.
    ///
    /// Sized from [`SampleLayout::stored_samples_per_pixel`] rather than from
    /// Samples per Pixel, so a 4:2:2 frame is two per pixel and not three.
    ///
    /// # Errors
    ///
    /// [`PixelError::SubsampledChromaAlignment`] when Columns is odd and the
    /// photometric interpretation subsamples chroma, because half a
    /// `Y1 Y2 Cb Cr` group cannot be stored. Reported before any length
    /// arithmetic, so no buffer is sized from a count that does not exist.
    /// [`PixelError::LengthOverflow`] when the frame does not fit `usize`.
    pub fn sample_count(&self) -> Result<usize, PixelError> {
        if self.layout.is_subsampled() && !self.dimensions.columns().is_multiple_of(2) {
            return Err(PixelError::SubsampledChromaAlignment);
        }
        let pixels = usize::try_from(self.dimensions.rows())
            .map_err(|_| PixelError::LengthOverflow)?
            .checked_mul(
                usize::try_from(self.dimensions.columns())
                    .map_err(|_| PixelError::LengthOverflow)?,
            )
            .ok_or(PixelError::LengthOverflow)?;
        pixels
            .checked_mul(usize::from(self.layout.stored_samples_per_pixel()))
            .ok_or(PixelError::LengthOverflow)
    }

    /// Unpack one frame into caller-provided stored-value storage.
    ///
    /// Both lengths are checked before the first destination element is
    /// written. Bits Stored are masked and signed samples are extended from
    /// `BitsStored - 1`, as required by PS3.3 C.7.6.3.
    pub fn unpack(
        &self,
        source: &[u8],
        byte_order: ByteOrder,
        destination: &mut [Stored],
    ) -> Result<(), PixelError> {
        let sample_count = self.sample_count()?;
        if destination.len() != sample_count {
            return Err(PixelError::DestinationLength);
        }
        let bytes_per_sample = usize::from(self.bits.bits_allocated() / 8);
        let source_length = sample_count
            .checked_mul(bytes_per_sample)
            .ok_or(PixelError::LengthOverflow)?;
        if source.len() != source_length {
            return Err(PixelError::SourceLength);
        }

        for (container, output) in source
            .chunks_exact(bytes_per_sample)
            .zip(destination.iter_mut())
        {
            let raw = read_container(container, byte_order)?;
            *output = Stored(self.extract(raw)?);
        }
        Ok(())
    }

    fn extract(&self, raw: u32) -> Result<f32, PixelError> {
        let shift = u32::from(self.bits.high_bit() + 1 - self.bits.bits_stored());
        let mask = if self.bits.bits_stored() == 32 {
            u32::MAX
        } else {
            (1_u32 << u32::from(self.bits.bits_stored())) - 1
        };
        let value = (raw >> shift) & mask;
        if self.bits.pixel_representation() == PixelRepresentation::Signed {
            let sign_bit = 1_u32 << u32::from(self.bits.bits_stored() - 1);
            let signed = if value & sign_bit == 0 {
                i64::from(value)
            } else {
                i64::from(value) - (1_i64 << u32::from(self.bits.bits_stored()))
            };
            signed_to_f32(signed)
        } else {
            Ok(unsigned_to_f32(value))
        }
    }
}

fn read_container(container: &[u8], byte_order: ByteOrder) -> Result<u32, PixelError> {
    match (container, byte_order) {
        ([a], _) => Ok(u32::from(*a)),
        ([a, b], ByteOrder::LittleEndian) => Ok(u32::from(u16::from_le_bytes([*a, *b]))),
        ([a, b], ByteOrder::BigEndian) => Ok(u32::from(u16::from_be_bytes([*a, *b]))),
        ([a, b, c, d], ByteOrder::LittleEndian) => Ok(u32::from_le_bytes([*a, *b, *c, *d])),
        ([a, b, c, d], ByteOrder::BigEndian) => Ok(u32::from_be_bytes([*a, *b, *c, *d])),
        _ => Err(PixelError::SourceLength),
    }
}

fn unsigned_to_f32(value: u32) -> f32 {
    let [a, b, c, d] = value.to_le_bytes();
    let low = u16::from_le_bytes([a, b]);
    let high = u16::from_le_bytes([c, d]);
    f32::from(high) * 65_536.0 + f32::from(low)
}

fn signed_to_f32(value: i64) -> Result<f32, PixelError> {
    if value < 0 {
        let magnitude =
            u32::try_from(value.unsigned_abs()).map_err(|_| PixelError::InvalidBitsStored)?;
        Ok(-unsigned_to_f32(magnitude))
    } else {
        let magnitude = u32::try_from(value).map_err(|_| PixelError::InvalidBitsStored)?;
        Ok(unsigned_to_f32(magnitude))
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ByteOrder, PhotometricInterpretation, PixelRepresentation, PlanarConfiguration,
        SampleLayout, StoredBits, StoredPixelDescription,
    };
    use crate::{ImageDimensions, PixelError};
    use ocelli_core::Stored;

    #[test]
    fn malformed_bit_fields_and_layout_are_refused() {
        let mono = PhotometricInterpretation::Monochrome2;
        assert_eq!(
            StoredBits::new(12, 12, 11, PixelRepresentation::Unsigned),
            Err(PixelError::UnsupportedBitsAllocated)
        );
        assert_eq!(
            StoredBits::new(16, 17, 16, PixelRepresentation::Unsigned),
            Err(PixelError::InvalidBitsStored)
        );
        assert_eq!(
            StoredBits::new(16, 12, 15, PixelRepresentation::Unsigned),
            Err(PixelError::InvalidHighBit)
        );
        assert_eq!(
            SampleLayout::new(3, mono, Some(PlanarConfiguration::Interleaved)),
            Err(PixelError::InvalidSamplesPerPixel)
        );
        assert_eq!(
            SampleLayout::new(1, mono, Some(PlanarConfiguration::Planar)),
            Err(PixelError::InvalidPlanarConfiguration)
        );
        assert_eq!(
            SampleLayout::new(
                3,
                PhotometricInterpretation::YbrFull422,
                Some(PlanarConfiguration::Planar)
            ),
            Err(PixelError::InvalidPlanarConfiguration)
        );
    }

    #[test]
    fn wrong_lengths_are_refused_before_output_mutation() {
        let dimensions = ImageDimensions::new(1, 1);
        let layout = SampleLayout::new(1, PhotometricInterpretation::Monochrome2, None);
        let bits = StoredBits::new(16, 12, 11, PixelRepresentation::Signed);
        assert!(dimensions.is_ok());
        assert!(layout.is_ok());
        assert!(bits.is_ok());
        let Ok(dimensions) = dimensions else { return };
        let Ok(layout) = layout else { return };
        let Ok(bits) = bits else { return };
        let description = StoredPixelDescription::new(dimensions, layout, bits);
        let mut output = [Stored(77.0)];
        assert_eq!(
            description.unpack(&[0], ByteOrder::LittleEndian, &mut output),
            Err(PixelError::SourceLength)
        );
        assert_eq!(output[0].0.to_bits(), 77.0_f32.to_bits());
        assert_eq!(
            description.unpack(&[0, 0], ByteOrder::LittleEndian, &mut []),
            Err(PixelError::DestinationLength)
        );
        assert_eq!(output[0].0.to_bits(), 77.0_f32.to_bits());
    }

    #[test]
    fn big_endian_and_thirty_two_bit_containers_are_supported() {
        let dimensions = ImageDimensions::new(1, 1);
        let layout = SampleLayout::new(1, PhotometricInterpretation::Monochrome2, None);
        let bits = StoredBits::new(32, 32, 31, PixelRepresentation::Unsigned);
        assert!(dimensions.is_ok());
        assert!(layout.is_ok());
        assert!(bits.is_ok());
        let Ok(dimensions) = dimensions else { return };
        let Ok(layout) = layout else { return };
        let Ok(bits) = bits else { return };
        let description = StoredPixelDescription::new(dimensions, layout, bits);
        let mut output = [Stored(0.0)];
        assert_eq!(
            description.unpack(&[0, 0, 1, 0], ByteOrder::BigEndian, &mut output),
            Ok(())
        );
        assert_eq!(output[0].0.to_bits(), 256.0_f32.to_bits());
    }

    #[test]
    fn every_container_width_and_byte_order_reaches_the_same_stored_value() {
        let dimensions = ImageDimensions::new(1, 1);
        let layout = SampleLayout::new(1, PhotometricInterpretation::Monochrome1, None);
        assert!(dimensions.is_ok());
        assert!(layout.is_ok());
        let Ok(dimensions) = dimensions else { return };
        let Ok(layout) = layout else { return };
        assert_eq!(
            layout.photometric_interpretation(),
            PhotometricInterpretation::Monochrome1
        );
        assert_eq!(layout.planar_configuration(), None);

        for (bits, bytes, byte_order) in [
            (8, &[0xff][..], ByteOrder::LittleEndian),
            (16, &[0xff, 0x00][..], ByteOrder::LittleEndian),
            (16, &[0x00, 0xff][..], ByteOrder::BigEndian),
            (32, &[0xff, 0x00, 0x00, 0x00][..], ByteOrder::LittleEndian),
            (32, &[0x00, 0x00, 0x00, 0xff][..], ByteOrder::BigEndian),
        ] {
            let stored_bits = StoredBits::new(bits, bits, bits - 1, PixelRepresentation::Unsigned);
            assert!(stored_bits.is_ok());
            let Ok(stored_bits) = stored_bits else { return };
            let description = StoredPixelDescription::new(dimensions, layout, stored_bits);
            let mut output = [Stored(0.0)];
            assert_eq!(description.unpack(bytes, byte_order, &mut output), Ok(()));
            assert_eq!(output[0].0.to_bits(), 255.0_f32.to_bits());
        }
    }
}
