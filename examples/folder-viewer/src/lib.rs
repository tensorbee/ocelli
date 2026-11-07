//! Local series viewer for DICOM Part 10 files.
//!
//! The page chooses the folder. This crate parses with `parse_part10`,
//! decodes with the codec registry, and maps samples with `LutChain`.
//! It does not read a path of its own.

mod corners;
mod link;
mod measure;
mod preview;
mod synthetic;
mod value;

#[cfg(not(target_arch = "wasm32"))]
mod held;
#[cfg(not(target_arch = "wasm32"))]
mod serve;

pub use corners::{
    age_text, dicom_date_text, dicom_time_text, exposure_mas, patient_orientation_letters,
    person_name_text,
};
pub use link::plane_from_token;
pub use measure::{FREEFORM_POINT_CAP, MeasureShape, measure_frame};
pub use preview::{
    FRAME_LIST_CAP, FrameFacts, FrameRequest, Preview, PreviewError, headers_json, inspect_json,
    plane_token, preview_first_frame, preview_frame, window_function_from_token,
    window_preset_from_token,
};
pub use synthetic::{SLICES, slice};
pub use value::{ImagePoint, value_frame};

#[cfg(not(target_arch = "wasm32"))]
pub use held::{Viewer, ViewerConfig};
#[cfg(not(target_arch = "wasm32"))]
pub use serve::{handle_connection, handle_connection_with, serve, serve_on};
