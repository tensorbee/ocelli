//! Frame-open helper for the example viewer.
//!
//! The page posts either the token `synthetic` or one Part 10 object. This
//! crate does not write that object to disk and does not log it.

mod bytes;
mod reply;

#[cfg(not(target_arch = "wasm32"))]
mod http;

pub use reply::reply_body;

#[cfg(not(target_arch = "wasm32"))]
pub use http::serve;
