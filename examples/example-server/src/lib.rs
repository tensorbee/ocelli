//! Frame-open, series-open and overlay helper for the example viewer.
//!
//! The page posts either the token `synthetic` or Part 10 bytes. This crate
//! does not write those bytes to disk and does not log them.

mod bytes;
mod overlay;
mod reply;
mod series;

#[cfg(not(target_arch = "wasm32"))]
mod http;

pub use reply::reply_body;

#[cfg(not(target_arch = "wasm32"))]
pub use http::serve;
