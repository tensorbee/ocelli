//! Budgeted LRU across encoded, decoded and GPU tiers.
//!
//! Targets: wasm32 yes, native yes. See `docs/hld/03-architecture-and-crates.md`.
//!
//! F-031 implements HLD section 20's `Budgeted` and `Lru<K, V>` over `alloc`'s
//! `BTreeMap` and a monotonic tick, and section 8's three tiers as `CacheTier`
//! and `Pressure`. `insert` returns `Admission<K, V>` rather than section 20's
//! `Vec<(K, V)>`, which is deviation D-24.
//!
//! **The three value types are not here.** Section 8's tiers hold encoded
//! bytes, decoded frames and GPU textures, and none of the three exists in this
//! workspace yet. This crate owns the budget and the rule `Budgeted::bytes`
//! keeps, and the stories that create a value type implement the trait for it.
//! That is also why there is no `wgpu` dependency: `ocelli-cache` never sees a
//! texture, only its size.

#![cfg_attr(not(test), no_std)]

extern crate alloc;

pub mod lru;
pub mod tier;

pub use lru::{Admission, Budgeted, Lru};
pub use tier::{CacheTier, Pressure};

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
