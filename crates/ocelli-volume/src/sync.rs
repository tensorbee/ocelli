//! Index of a patient point on one held plane.
//!
//! The plane origin is the anchor plus the stored index times spacing along
//! the normal. DICOM PS3.3 C.7.6.2.1.1 places that origin. The new index is
//! the stored index plus the offset along the normal, divided by spacing.
//! This module does not read a voxel and does not read a UID.

use glam::DVec3;
use ocelli_core::{Pt, World};

/// One plane held at an index. `anchor` is the world point of index 0.
#[derive(Clone, Copy, Debug)]
pub struct HeldPlane {
    /// World point of index 0.
    pub anchor: Pt<World>,
    /// Patient normal. A zero vector is refused.
    pub normal: DVec3,
    /// Millimetres between indexes along `normal`.
    pub spacing_mm: f64,
    /// Current index. The call does not write this field.
    pub index: f64,
}

/// The plane cannot name an index.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlaneSyncError {
    /// Spacing, normal or a coordinate cannot define an index.
    Unavailable,
}

impl HeldPlane {
    /// The index of `point` on this plane.
    ///
    /// The stored index, anchor, normal and spacing stay as they were.
    ///
    /// # Errors
    ///
    /// [`PlaneSyncError::Unavailable`] when spacing is not finite and
    /// greater than zero, when the normal has length zero, or when any
    /// component of the anchor, normal, index or point is not finite.
    pub fn index_through_point(&self, point: Pt<World>) -> Result<f64, PlaneSyncError> {
        if !finite_point(self.anchor)
            || !self.normal.x.is_finite()
            || !self.normal.y.is_finite()
            || !self.normal.z.is_finite()
            || !self.index.is_finite()
            || !finite_point(point)
            || !self.spacing_mm.is_finite()
            || self.spacing_mm <= 0.0
        {
            return Err(PlaneSyncError::Unavailable);
        }
        let length = self.normal.length();
        if !length.is_finite() || length <= 0.0 {
            return Err(PlaneSyncError::Unavailable);
        }
        let step = self.index * self.spacing_mm;
        let origin_x = self.anchor.x + step * self.normal.x;
        let origin_y = self.anchor.y + step * self.normal.y;
        let origin_z = self.anchor.z + step * self.normal.z;
        let offset = (point.x - origin_x) * self.normal.x
            + (point.y - origin_y) * self.normal.y
            + (point.z - origin_z) * self.normal.z;
        let next = self.index + offset / self.spacing_mm;
        if !next.is_finite() {
            return Err(PlaneSyncError::Unavailable);
        }
        Ok(next)
    }
}

fn finite_point(point: Pt<World>) -> bool {
    point.x.is_finite() && point.y.is_finite() && point.z.is_finite()
}
