//! Local series viewer for DICOM Part 10 files.
//!
//! The page chooses the folder. This crate parses with `parse_part10`,
//! decodes with the codec registry, and maps samples with `LutChain`.
//! It does not read a path of its own.

mod preview;
mod synthetic;

#[cfg(not(target_arch = "wasm32"))]
mod held;
#[cfg(not(target_arch = "wasm32"))]
mod serve;

pub use preview::{
    FrameFacts, FrameRequest, Preview, PreviewError, headers_json, inspect_json,
    preview_first_frame, preview_frame, window_function_from_token, window_preset_from_token,
};
pub use synthetic::{SLICES, slice};

#[cfg(not(target_arch = "wasm32"))]
pub use held::{Viewer, ViewerConfig};
#[cfg(not(target_arch = "wasm32"))]
pub use serve::{handle_connection, handle_connection_with, serve, serve_on};
