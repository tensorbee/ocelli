//! Budgeted LRU across encoded, decoded and GPU tiers.
//!
//! Targets: wasm32 yes, native yes. See `docs/hld/03-architecture-and-crates.md`.
//!
//! F-031 implements HLD section 20's `Budgeted` and `Lru<K, V>` over `alloc`'s
//! `BTreeMap` and a monotonic tick, and section 8's three tiers as `CacheTier`
//! and `Pressure`. `insert` returns `Admission<K, V>` rather than section 20's
//! `Vec<(K, V)>`, which is deviation D-24.
//!
//! **Encoded bytes are not here.** Section 8's tiers hold encoded bytes,
//! decoded frames and GPU textures. F-032 added the decoded tier's
//! `DecodedFrame` and its `ImageCache` in `image`. F-036 added `Brick` on the
//! GPU tier: a host buffer whose cost is residency, not a texture. F-043's
//! upload takes that brick by value and drops it after the copy. There is
//! still no `wgpu` dependency, because this crate never sees a texture, only
//! its size.
//!
//! F-224 adds the decoded tier's frame identity: `ContentId`, `FrameSource`
//! and `FrameLookup` in `image`, so a held frame is served only for the
//! bytes it was decoded from.
//!
//! F-231 adds the committed tier: `CommittedFrame` and `CommittedSource`,
//! and `ImageCache` becomes generic over its value with `DecodedFrame` as
//! the default, so the wasm session reuses this cache instead of a second
//! one. The two tiers share the `Lru`, the events and the stale rule.

#![cfg_attr(not(test), no_std)]

extern crate alloc;

pub mod brick;
pub mod image;
pub mod lru;
pub mod profile;
pub mod telemetry;
pub mod tier;

pub use brick::{
    Brick, BrickCache, BrickError, BrickGrid, BrickKey, BrickSample, BrickWidth, ReleaseSummary,
    VolumeId,
};
pub use image::{
    CacheEvent, CacheEvents, CommittedFrame, CommittedSource, ContentId, DecodedFrame, FrameKey,
    FrameLookup, FrameSource, HandoffError, HandoffMeta, ImageCache, KIND_BUDGET_CHANGED,
    KIND_DISPLACED, KIND_EVICTED, KIND_PRESSURE, KIND_REFUSED, KIND_RELEASED, RingEvent, SeriesId,
};
pub use lru::{Admission, Budgeted, Lru};
pub use profile::{LoadProfile, LoadSpec, profile_load};
pub use telemetry::{CacheTelemetry, TelemetryEvent, TierCounters, TierTelemetry};
pub use tier::{CacheTier, Pressure, PressureBand};

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
