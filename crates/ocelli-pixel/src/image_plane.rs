//! DICOM image-plane evidence and its index-to-world transform.

use glam::{DMat4, DVec3, DVec4};
use ocelli_core::{Index, Transform, World};

use crate::PixelError;

// Image Orientation Patient is dimensionless. Rounding each component to six
// decimal places introduces at most 0.5e-6 component error. Cauchy-Schwarz
// bounds the resulting dot error below 2 * sqrt(3) * 0.5e-6 + 3 * (0.5e-6)^2,
// which is below 1.733e-6. Two parts per million is a conservative boundary.
const DIRECTION_COSINE_TOLERANCE: f64 = 2e-6;

/// Image Position Patient, the LPS-mm centre of the first voxel.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ImagePositionPatient([f64; 3]);

impl ImagePositionPatient {
    /// Validate a three-component Image Position Patient value.
    pub fn new(value: [f64; 3]) -> Result<Self, PixelError> {
        if value.iter().all(|component| component.is_finite()) {
            Ok(Self(value))
        } else {
            Err(PixelError::InvalidImagePosition)
        }
    }
}

/// Image Orientation Patient row and column direction cosines.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ImageOrientationPatient {
    row: DVec3,
    column: DVec3,
}

impl ImageOrientationPatient {
    /// Validate finite, unit-length and mutually orthogonal direction cosines.
    pub fn new(row: [f64; 3], column: [f64; 3]) -> Result<Self, PixelError> {
        let row = DVec3::from_array(row);
        let column = DVec3::from_array(column);
        let row_length_error = (row.length() - 1.0).abs();
        let column_length_error = (column.length() - 1.0).abs();
        let orthogonality_error = row.dot(column).abs();
        if !row.is_finite()
            || !column.is_finite()
            || row_length_error > DIRECTION_COSINE_TOLERANCE
            || column_length_error > DIRECTION_COSINE_TOLERANCE
            || orthogonality_error > DIRECTION_COSINE_TOLERANCE
        {
            return Err(PixelError::InvalidImageOrientation);
        }
        let row = row.normalize();
        let column = (column - row * row.dot(column)).normalize();
        Ok(Self { row, column })
    }
}

/// Pixel Spacing with DICOM's row-then-column value order made explicit.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PixelSpacing {
    /// Distance between adjacent rows, in millimetres.
    row_mm: f64,
    /// Distance between adjacent columns, in millimetres.
    column_mm: f64,
}

impl PixelSpacing {
    /// Validate finite nonnegative row and column spacing.
    ///
    /// Zero remains provisional until [`ImagePlane::new`] can compare it with
    /// the corresponding image dimension.
    pub fn new(row_mm: f64, column_mm: f64) -> Result<Self, PixelError> {
        if !row_mm.is_finite() || !column_mm.is_finite() || row_mm < 0.0 || column_mm < 0.0 {
            return Err(PixelError::InvalidPixelSpacing);
        }
        Ok(Self { row_mm, column_mm })
    }

    /// Distance between adjacent rows, in millimetres.
    pub const fn row_mm(self) -> f64 {
        self.row_mm
    }

    /// Distance between adjacent columns, in millimetres.
    pub const fn column_mm(self) -> f64 {
        self.column_mm
    }
}

/// Nonzero image dimensions.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ImageDimensions {
    /// Number of rows.
    rows: u32,
    /// Number of columns.
    columns: u32,
}

impl ImageDimensions {
    /// Validate nonzero Rows and Columns.
    pub const fn new(rows: u32, columns: u32) -> Result<Self, PixelError> {
        if rows == 0 || columns == 0 {
            Err(PixelError::InvalidDimensions)
        } else {
            Ok(Self { rows, columns })
        }
    }

    /// Number of rows.
    pub const fn rows(self) -> u32 {
        self.rows
    }

    /// Number of columns.
    pub const fn columns(self) -> u32 {
        self.columns
    }
}

/// Validated evidence needed to place one image plane in patient space.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ImagePlane {
    /// Image Position Patient.
    pub position: ImagePositionPatient,
    /// Image Orientation Patient.
    pub orientation: ImageOrientationPatient,
    /// Pixel Spacing.
    spacing: PixelSpacing,
    /// Rows and Columns.
    dimensions: ImageDimensions,
}

impl ImagePlane {
    /// Assemble validated attributes and apply the singleton-spacing rule.
    ///
    /// # Errors
    ///
    /// Returns [`PixelError::InvalidPixelSpacing`] when a zero row spacing is
    /// paired with more than one row, or a zero column spacing is paired with
    /// more than one column.
    pub const fn new(
        position: ImagePositionPatient,
        orientation: ImageOrientationPatient,
        spacing: PixelSpacing,
        dimensions: ImageDimensions,
    ) -> Result<Self, PixelError> {
        if (spacing.row_mm <= 0.0 && dimensions.rows != 1)
            || (spacing.column_mm <= 0.0 && dimensions.columns != 1)
        {
            return Err(PixelError::InvalidPixelSpacing);
        }
        Ok(Self {
            position,
            orientation,
            spacing,
            dimensions,
        })
    }

    /// Validated Pixel Spacing.
    pub const fn spacing(&self) -> PixelSpacing {
        self.spacing
    }

    /// Validated Rows and Columns.
    pub const fn dimensions(&self) -> ImageDimensions {
        self.dimensions
    }

    /// The unit slice normal, DICOM PS3.3 C.7.6.2.1.1's `X cross Y`.
    ///
    /// Exposed so a cross-frame consumer can project an inter-frame vector onto
    /// it without re-deriving the cross product, which would be a second copy
    /// of this plane's geometry.
    pub fn slice_normal(&self) -> DVec3 {
        self.orientation
            .row
            .cross(self.orientation.column)
            .normalize()
    }

    /// Image Position Patient as a vector, for inter-frame differences.
    pub fn position_vector(&self) -> DVec3 {
        DVec3::from_array(self.position.0)
    }

    /// Build the PS3.3 C.7.6.2.1.1 index-to-patient transform.
    ///
    /// `x` is column index `i`, so the first matrix column uses column
    /// spacing and the row direction cosine. `y` is row index `j`, so the
    /// second uses row spacing and the column direction cosine. The third
    /// column is a unit plane normal. It makes the transform invertible while
    /// leaving every in-plane point exactly on the specified equation.
    pub fn index_to_world(&self) -> Transform<Index, World> {
        // A zero spacing is legal only on a singleton axis, where every valid
        // index is zero. A unit extension on that unused axis preserves all
        // valid voxel positions and keeps the four-by-four transform invertible.
        let column_spacing = if self.spacing.column_mm > 0.0 {
            self.spacing.column_mm
        } else {
            1.0
        };
        let row_spacing = if self.spacing.row_mm > 0.0 {
            self.spacing.row_mm
        } else {
            1.0
        };
        let row_step = self.orientation.row * column_spacing;
        let column_step = self.orientation.column * row_spacing;
        let normal = self
            .orientation
            .row
            .cross(self.orientation.column)
            .normalize();
        let position = DVec3::from_array(self.position.0);
        Transform::from_mat4(DMat4::from_cols(
            row_step.extend(0.0),
            column_step.extend(0.0),
            normal.extend(0.0),
            DVec4::new(position.x, position.y, position.z, 1.0),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::{ImageDimensions, ImageOrientationPatient, ImagePositionPatient, PixelSpacing};
    use crate::PixelError;

    #[test]
    fn malformed_plane_attributes_are_refused() {
        assert_eq!(
            ImagePositionPatient::new([0.0, f64::NAN, 0.0]),
            Err(PixelError::InvalidImagePosition)
        );
        assert_eq!(
            ImageOrientationPatient::new([0.0; 3], [0.0, 1.0, 0.0]),
            Err(PixelError::InvalidImageOrientation)
        );
        assert_eq!(
            ImageOrientationPatient::new([1.0, 0.0, 0.0], [2.0, 0.0, 0.0]),
            Err(PixelError::InvalidImageOrientation)
        );
        assert_eq!(
            PixelSpacing::new(-0.5, 1.0),
            Err(PixelError::InvalidPixelSpacing)
        );
        assert_eq!(
            ImageDimensions::new(0, 1),
            Err(PixelError::InvalidDimensions)
        );
    }
}
