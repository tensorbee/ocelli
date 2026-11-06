//! Hit-testing, projection, measurement mathematics.
//!
//! Targets: wasm32 yes, native yes. See `docs/hld/03-architecture-and-crates.md`.
//!
//! F-001 creates the crate. F-070 fills [`measure`]. F-073 adds length,
//! height, Cobb and bidirectional beside those functions. The angle tool
//! stays [`angle_at`]. F-074 adds rectangle, ellipse and circle. A probe
//! is not a second sampler in this crate. F-076 adds a polygon area and one
//! spline point. Freehand length stays [`length`]. The formulas are the
//! local rules in `.claude/plans/F-070-design.md`,
//! `.claude/plans/F-073-design.md`, `.claude/plans/F-074-design.md` and
//! `.claude/plans/F-076-design.md`. F-075 adds ROI statistics, a histogram
//! and SUVbw in [`stats`]. Those numbers are not a second area and not the
//! modality LUT. F-195 adds calibration provenance, SUVlbm, SUVbsa, a length
//! ratio and a contour volume. Those numbers share the F-075 decay and are
//! not a second length. F-077 adds [`livewire_path`] and [`livewire_mask`]. The
//! mask marks visited pixels and does not fill an interior.
//! [`polygon_area`] stays the F-076 function. F-079 adds
//! [`ultrasound_direction`] and [`redact_rect`]. The angle is the image
//! plane and the length includes `z`. A missed rectangle sets `applied`
//! false. [`distance`] stays the F-070 function. The HLD names the crate
//! and does not give them. Camera transforms stay in `ocelli-viewport`.
//! F-211 adds [`region_contains`] and [`region_bounds`]. A sample belongs
//! to a region when its centre, the index point, is inside or on the
//! boundary, and the polygon rule is the F-085 rule of `ocelli-seg`.
//! [`polygon_crosses_itself`] finds a proper crossing of two edges.

#![cfg_attr(not(test), no_std)]

extern crate alloc;

mod calibration;
mod livewire;
mod measure;
mod redact;
mod region;
mod stats;

pub use calibration::{
    Calibration, CalibrationRequest, CalibrationSource, SpacingMm, contour_volume, length_ratio,
    select_calibration,
};
pub use livewire::{livewire_mask, livewire_path};
pub use measure::{
    Bidirectional, Circle, GeomError, Rectangle, SegmentProjection, angle_at, bidirectional,
    circle, cobb, distance, ellipse, height, hit_segment, length, polygon_area, polyline_length,
    project_onto_plane, project_onto_segment, rectangle, spline_point,
};
pub use redact::{Direction, Redaction, redact_rect, ultrasound_direction};
pub use region::{
    IndexBounds, RegionShape, polygon_crosses_itself, region_bounds, region_contains,
};
pub use stats::{
    Histogram, RoiStats, SuvActivity, SuvError, SuvInputs, histogram, roi_statistics, suv_bsa,
    suv_bw, suv_lbm,
};

/// The crate's own name. The scaffold test asserts it matches Cargo's, which
/// is the one mistake a copy-pasted crate skeleton actually makes.
pub const CRATE_NAME: &str = env!("CARGO_PKG_NAME");

#[cfg(test)]
mod tests {
    #[test]
    fn scaffold_declares_its_own_name() {
        assert_eq!(super::CRATE_NAME, env!("CARGO_PKG_NAME"));
        assert!(super::CRATE_NAME.starts_with("ocelli"));
    }
}
