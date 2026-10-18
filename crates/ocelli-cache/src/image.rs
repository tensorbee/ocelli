//! The decoded tier's value type, its cache, and the events an insert emits.
//!
//! HLD section 8: "Decoded frames sit in an LRU sized by the caller." F-031
//! built the `Lru` and deviation D-24 widened its return to `Admission`, so the
//! three outcomes stay distinguishable. This module is the caller section 20
//! means: it converts an `Admission` into one `CacheEvent` per outcome, and it
//! encodes an event into section 17.3's 48-byte envelope.
//!
//! **No ring is built here and nothing reaches JavaScript.** Section 17.3's
//! ring is F-101 in S16. This story stops at the tested encoding, which the
//! S12 design round decided for F-032 and F-034 together.
//!
//! F-035 adds [`HandoffMeta`] beside [`FrameKey`]. It carries the frame
//! identity and the sample length of a section 24 transfer, and no samples.

use alloc::vec::Vec;

use bytemuck::{Pod, Zeroable};
use ocelli_pixel::{ImagePlane, StoredPixelDescription};

use crate::lru::{Budgeted, Lru};
use crate::tier::{CacheTier, Pressure};

/// A series identity inside a `FrameKey`.
///
/// A `u64` rather than a `SeriesInstanceUID` string, because `FrameKey` is
/// `Ord` for the `BTreeMap` `Lru` is built on and a per-frame string comparison
/// on the decode path is a cost with no reader. Assigning an id to a UID
/// belongs to whoever owns a study, which is not this crate.
///
/// `#[repr(transparent)]` keeps this one `u64` in [`HandoffMeta`]'s C layout.
#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SeriesId(pub u64);

/// One frame's identity, which is what makes an eviction reportable.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FrameKey {
    /// The series the frame belongs to.
    pub series: SeriesId,
    /// The instance number within the series.
    pub instance: u32,
    /// The frame number within the instance.
    pub frame: u32,
}

impl FrameKey {
    /// Assemble a frame identity.
    pub const fn new(series: SeriesId, instance: u32, frame: u32) -> Self {
        Self {
            series,
            instance,
            frame,
        }
    }
}

/// Identity and sample length for one section 24 transfer.
///
/// `FrameKey` is the identity. `bytes` is the sample length the transferred
/// buffer must match. The samples stay in that buffer. This value has no heap.
///
/// `#[repr(C)]` puts `series` at 0, `instance` at 8, `frame` at 12 and `bytes`
/// at 16. [`Self::to_le_bytes`] is the wire image. Little-endian is section
/// 17.1's rule, not an assumption about the host.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HandoffMeta {
    /// The series the frame belongs to.
    pub series: SeriesId,
    /// The instance number within the series.
    pub instance: u32,
    /// The frame number within the instance.
    pub frame: u32,
    /// The sample length the transfer must match. Not the samples.
    pub bytes: u64,
}

/// A transferred buffer whose length is not the length [`HandoffMeta`] declared.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HandoffError {
    /// `buffer.len()` is not `expected`. `actual` is that length as `u64`.
    Length { expected: u64, actual: u64 },
}

impl HandoffMeta {
    /// Wire length of [`Self::to_le_bytes`].
    pub const BYTES: usize = 24;

    /// Assemble a handoff from the frame identity and its sample length.
    #[must_use]
    pub const fn new(series: SeriesId, instance: u32, frame: u32, bytes: u64) -> Self {
        Self {
            series,
            instance,
            frame,
            bytes,
        }
    }

    /// The frame identity. The sample length is not part of it.
    #[must_use]
    pub const fn key(self) -> FrameKey {
        FrameKey::new(self.series, self.instance, self.frame)
    }

    /// The 24-byte little-endian image of the four fields.
    #[must_use]
    pub const fn to_le_bytes(self) -> [u8; Self::BYTES] {
        let [s0, s1, s2, s3, s4, s5, s6, s7] = self.series.0.to_le_bytes();
        let [i0, i1, i2, i3] = self.instance.to_le_bytes();
        let [f0, f1, f2, f3] = self.frame.to_le_bytes();
        let [b0, b1, b2, b3, b4, b5, b6, b7] = self.bytes.to_le_bytes();
        [
            s0, s1, s2, s3, s4, s5, s6, s7, i0, i1, i2, i3, f0, f1, f2, f3, b0, b1, b2, b3, b4, b5,
            b6, b7,
        ]
    }

    /// Read the image [`Self::to_le_bytes`] wrote.
    #[must_use]
    pub const fn from_le_bytes(raw: [u8; Self::BYTES]) -> Self {
        let [
            s0,
            s1,
            s2,
            s3,
            s4,
            s5,
            s6,
            s7,
            i0,
            i1,
            i2,
            i3,
            f0,
            f1,
            f2,
            f3,
            b0,
            b1,
            b2,
            b3,
            b4,
            b5,
            b6,
            b7,
        ] = raw;
        Self {
            series: SeriesId(u64::from_le_bytes([s0, s1, s2, s3, s4, s5, s6, s7])),
            instance: u32::from_le_bytes([i0, i1, i2, i3]),
            frame: u32::from_le_bytes([f0, f1, f2, f3]),
            bytes: u64::from_le_bytes([b0, b1, b2, b3, b4, b5, b6, b7]),
        }
    }

    /// Refuse a buffer whose length is not [`Self::bytes`].
    ///
    /// The samples stay in `buffer`. This value does not copy them.
    ///
    /// # Errors
    ///
    /// [`HandoffError::Length`] when the lengths disagree. A `bytes` value
    /// that does not fit in `usize` disagrees with every buffer.
    pub fn check_buffer(self, buffer: &[u8]) -> Result<(), HandoffError> {
        let same = match usize::try_from(self.bytes) {
            Ok(expected) => expected == buffer.len(),
            Err(_too_wide) => false,
        };
        if same {
            return Ok(());
        }
        let actual = u64::try_from(buffer.len()).unwrap_or(u64::MAX);
        Err(HandoffError::Length {
            expected: self.bytes,
            actual,
        })
    }
}

/// One decoded frame, the decoded tier's value type.
///
/// `bytes` is the sample buffer length and not a computed
/// `rows * columns * bytes_per_pixel`, because the buffer is the allocation
/// and a computed figure can disagree with it.
#[derive(Clone, Debug, PartialEq)]
pub struct DecodedFrame {
    description: StoredPixelDescription,
    plane: ImagePlane,
    samples: Vec<u8>,
}

impl DecodedFrame {
    /// Assemble one decoded frame from its description, plane and samples.
    pub fn new(description: StoredPixelDescription, plane: ImagePlane, samples: Vec<u8>) -> Self {
        Self {
            description,
            plane,
            samples,
        }
    }

    /// The stored pixel description this frame was decoded under.
    pub fn description(&self) -> StoredPixelDescription {
        self.description
    }

    /// The image plane this frame sits on.
    pub fn plane(&self) -> ImagePlane {
        self.plane
    }

    /// The decoded sample buffer.
    pub fn samples(&self) -> &[u8] {
        &self.samples
    }
}

impl Budgeted for DecodedFrame {
    /// The allocated size of the sample buffer.
    fn bytes(&self) -> usize {
        self.samples.len()
    }
}

/// One thing an insert did to the decoded tier.
///
/// Three variants because `Admission` has three fields, which is the whole
/// reason deviation D-24 exists. Collapsing them throws away what F-031 was
/// built to preserve.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CacheEvent {
    /// A live entry removed to make room for another.
    Evicted {
        /// The frame that left.
        key: FrameKey,
        /// The bytes it was admitted at.
        bytes: usize,
    },
    /// A previous value replaced under the same key.
    Displaced {
        /// The frame whose value was replaced.
        key: FrameKey,
        /// The bytes the previous value was admitted at.
        bytes: usize,
    },
    /// An entry the budget could not hold, never admitted.
    Refused {
        /// The frame that was refused.
        key: FrameKey,
        /// The bytes it would have cost.
        bytes: usize,
    },
}

/// Event kind for [`CacheEvent::Evicted`]. One range for this crate's kinds,
/// with F-034's telemetry kinds beside these. No registry this sprint, which
/// the S12 design round decided for F-032 and F-034 together.
pub const KIND_EVICTED: u32 = 1;
/// Event kind for [`CacheEvent::Displaced`].
pub const KIND_DISPLACED: u32 = 2;
/// Event kind for [`CacheEvent::Refused`].
pub const KIND_REFUSED: u32 = 3;
/// Event kind for a pressure-band crossing or a requested pressure reading.
///
/// F-034. The number sits in the same range as the three cache kinds. There
/// is no event-kind registry this sprint, so a test pins it.
pub const KIND_PRESSURE: u32 = 4;
/// Event kind for a budget change. F-034, pinned the same way.
pub const KIND_BUDGET_CHANGED: u32 = 5;

/// Section 17.3's `Event`, field for field. 48 bytes.
///
/// `viewport` is `u32::MAX` for a cache event, because an eviction belongs to
/// no viewport and zero is a valid viewport id.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Pod, Zeroable)]
pub struct RingEvent {
    /// Which event this is. One of this crate's `KIND_` constants.
    pub kind: u32,
    /// The viewport the event belongs to, or `u32::MAX` for a cache event.
    pub viewport: u32,
    /// The ring-wide sequence number the caller assigned.
    pub seq: u64,
    /// The event payload. For a cache event, the key and byte count.
    pub payload: [u8; 32],
}

impl CacheEvent {
    /// The key and byte count every variant carries.
    fn parts(&self) -> (u32, FrameKey, usize) {
        match *self {
            Self::Evicted { key, bytes } => (KIND_EVICTED, key, bytes),
            Self::Displaced { key, bytes } => (KIND_DISPLACED, key, bytes),
            Self::Refused { key, bytes } => (KIND_REFUSED, key, bytes),
        }
    }

    /// Encode into section 17.3's envelope with the caller's sequence number.
    ///
    /// The payload is `series` as `u64`, `instance` and `frame` as `u32`, and
    /// `bytes` as `u64`, little-endian, in that order, with the last eight
    /// bytes reserved and zeroed. Little-endian is not a choice: wasm32 is
    /// little-endian and section 17.1 says the wire format is.
    pub fn encode(&self, seq: u64) -> RingEvent {
        let (kind, key, bytes) = self.parts();
        let mut payload = [0_u8; 32];
        payload[0..8].copy_from_slice(&key.series.0.to_le_bytes());
        payload[8..12].copy_from_slice(&key.instance.to_le_bytes());
        payload[12..16].copy_from_slice(&key.frame.to_le_bytes());
        payload[16..24].copy_from_slice(&u64::try_from(bytes).unwrap_or(u64::MAX).to_le_bytes());
        RingEvent {
            kind,
            viewport: u32::MAX,
            seq,
            payload,
        }
    }
}

/// The events one `ImageCache::insert` produced, in the order they happened.
///
/// Built on the insert path, which is a decode-path call and not a render-loop
/// call, so building this value allocates as `Lru::insert` already does.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CacheEvents {
    events: Vec<CacheEvent>,
}

impl CacheEvents {
    /// The events in order. A displaced value comes before any eviction,
    /// because `Lru::insert` releases it before eviction begins.
    pub fn events(&self) -> &[CacheEvent] {
        &self.events
    }

    /// Whether the insert displaced, evicted and refused nothing.
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// How many events the insert produced.
    pub fn len(&self) -> usize {
        self.events.len()
    }
}

/// The decoded tier: decoded frames in an `Lru` sized by the caller.
pub struct ImageCache {
    decoded: Lru<FrameKey, DecodedFrame>,
}

impl ImageCache {
    /// An empty decoded tier holding at most `budget` bytes.
    pub fn new(budget: usize) -> Self {
        Self {
            decoded: Lru::new(CacheTier::Decoded, budget),
        }
    }

    /// Insert a frame and return the events the caller must emit.
    ///
    /// The evicted entries are converted in the order `Admission` returned
    /// them, least recently used first.
    pub fn insert(&mut self, key: FrameKey, frame: DecodedFrame) -> CacheEvents {
        let admission = self.decoded.insert(key, frame);
        let mut events = CacheEvents::default();
        if let Some(displaced) = admission.displaced {
            events.events.push(CacheEvent::Displaced {
                key,
                bytes: displaced.bytes(),
            });
        }
        for (evicted_key, evicted) in admission.evicted {
            events.events.push(CacheEvent::Evicted {
                key: evicted_key,
                bytes: evicted.bytes(),
            });
        }
        if let Some((refused_key, refused)) = admission.refused {
            events.events.push(CacheEvent::Refused {
                key: refused_key,
                bytes: refused.bytes(),
            });
        }
        events
    }

    /// Read a frame. Reading is a use, so it becomes the most recently used.
    pub fn get(&mut self, key: &FrameKey) -> Option<&DecodedFrame> {
        self.decoded.get(key)
    }

    /// Explicitly release a frame. Not an eviction, and emits no event.
    pub fn remove(&mut self, key: &FrameKey) -> Option<DecodedFrame> {
        self.decoded.remove(key)
    }

    /// The decoded tier's pressure signal. Telemetry, not a use.
    pub fn pressure(&self) -> Pressure {
        self.decoded.pressure()
    }
}

#[cfg(test)]
mod handoff_tests {
    use super::{FrameKey, HandoffError, HandoffMeta, SeriesId};

    /// `0x0102_0304_0506_0708` little-endian is `08 07 06 05 04 03 02 01`.
    /// `0xA0B0_C0D0` little-endian is `D0 C0 B0 A0`.
    /// `0x1122_3344` little-endian is `44 33 22 11`.
    /// `524_288 = 0x0008_0000`, eight bytes little-endian, is
    /// `00 00 08 00 00 00 00 00`.
    const WIRE: [u8; 24] = [
        0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01, 0xD0, 0xC0, 0xB0, 0xA0, 0x44, 0x33, 0x22,
        0x11, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00,
    ];

    fn sample() -> HandoffMeta {
        HandoffMeta::new(
            SeriesId(0x0102_0304_0506_0708),
            0xA0B0_C0D0,
            0x1122_3344,
            524_288,
        )
    }

    #[test]
    fn fields_round_trip_little_endian() {
        let meta = sample();
        assert_eq!(core::mem::size_of::<HandoffMeta>(), HandoffMeta::BYTES);
        assert_eq!(HandoffMeta::BYTES, 24);
        assert_eq!(core::mem::align_of::<HandoffMeta>(), 8);
        assert_eq!(core::mem::offset_of!(HandoffMeta, series), 0);
        assert_eq!(core::mem::offset_of!(HandoffMeta, instance), 8);
        assert_eq!(core::mem::offset_of!(HandoffMeta, frame), 12);
        assert_eq!(core::mem::offset_of!(HandoffMeta, bytes), 16);
        assert_eq!(meta.to_le_bytes(), WIRE);
        assert_eq!(HandoffMeta::from_le_bytes(WIRE), meta);
        assert_eq!(
            meta.key(),
            FrameKey::new(SeriesId(0x0102_0304_0506_0708), 0xA0B0_C0D0, 0x1122_3344,)
        );
    }

    #[test]
    fn refuses_a_length_that_disagrees_with_the_buffer() {
        // `instance` is 2 and `frame` is 3. Only `bytes` is the length.
        let meta = HandoffMeta::new(SeriesId(9), 2, 3, 4);
        assert_eq!(meta.check_buffer(&[0, 0, 0, 0]), Ok(()));
        assert_eq!(
            meta.check_buffer(&[0, 0]),
            Err(HandoffError::Length {
                expected: 4,
                actual: 2,
            })
        );
        assert_eq!(
            meta.check_buffer(&[0, 0, 0]),
            Err(HandoffError::Length {
                expected: 4,
                actual: 3,
            })
        );
        assert_eq!(
            meta.check_buffer(&[0, 0, 0, 0, 0]),
            Err(HandoffError::Length {
                expected: 4,
                actual: 5,
            })
        );
        let empty = HandoffMeta::new(SeriesId(9), 2, 3, 0);
        assert_eq!(empty.check_buffer(&[]), Ok(()));
        assert_eq!(
            empty.check_buffer(&[0]),
            Err(HandoffError::Length {
                expected: 0,
                actual: 1,
            })
        );
    }
}
