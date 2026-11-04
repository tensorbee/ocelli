//! Local series viewer for DICOM Part 10 files.
//!
//! The page chooses the folder. This crate parses with `parse_part10`,
//! decodes with the codec registry, and maps samples with `LutChain`.
//! It does not read a path of its own.

mod preview;

#[cfg(not(target_arch = "wasm32"))]
mod serve;

pub use preview::{
    FrameRequest, Preview, PreviewError, headers_json, inspect_json, preview_first_frame,
    preview_frame, window_function_from_token,
};

#[cfg(not(target_arch = "wasm32"))]
pub use serve::{handle_connection, serve};
