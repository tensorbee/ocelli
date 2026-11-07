//! Frame-open, series-open, overlay and interpolation helper for the example viewer.
//!
//! The page posts either the token `synthetic` or Part 10 bytes. This crate
//! does not write those bytes to disk and does not log them.

#![doc(test(attr(forbid(unsafe_code))))]

mod bytes;
mod calibration;
mod interpolate;
mod overlay;
mod reply;
mod series;
mod tilt;

#[cfg(not(target_arch = "wasm32"))]
mod http;

pub use reply::reply_body;

#[cfg(not(target_arch = "wasm32"))]
pub use http::serve;
