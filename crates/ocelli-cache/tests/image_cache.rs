//! F-032, the decoded tier, its eviction events and the section 17.3 encoding.
//!
//! Fixtures are hand-computed from the plan. A 512 by 512 sixteen-bit frame is
//! 512 * 512 * 2 = 524,288 bytes, from Rows and Columns in DICOM PS3.3
//! C.7.6.3.1 at two bytes per stored sample. The `RingEvent` layout is section
//! 17.3 of `docs/hld/14-the-boundary-in-code.md`, 48 bytes with `kind` at 0,
//! `viewport` at 4, `seq` at 8 and `payload` at 16.

use std::collections::BTreeMap;

use ocelli_cache::{
    Budgeted, CacheEvent, DecodedFrame, FrameKey, ImageCache, KIND_DISPLACED, KIND_EVICTED,
    KIND_REFUSED, RingEvent, SeriesId,
};
use ocelli_pixel::{
    ImageDimensions, ImageOrientationPatient, ImagePlane, ImagePositionPatient,
    PhotometricInterpretation, PixelRepresentation, PixelSpacing, SampleLayout, StoredBits,
    StoredPixelDescription,
};
use proptest::prelude::*;

fn description(
    rows: u32,
    columns: u32,
    bits_allocated: u8,
) -> Result<StoredPixelDescription, ocelli_pixel::PixelError> {
    let layout = SampleLayout::new(1, PhotometricInterpretation::Monochrome2, None)?;
    let bits = StoredBits::new(
        bits_allocated,
        bits_allocated,
        bits_allocated - 1,
        PixelRepresentation::Unsigned,
    )?;
    Ok(StoredPixelDescription::new(
        ImageDimensions::new(rows, columns)?,
        layout,
        bits,
    ))
}

fn plane(rows: u32, columns: u32) -> Result<ImagePlane, ocelli_pixel::PixelError> {
    ImagePlane::new(
        ImagePositionPatient::new([0.0, 0.0, 0.0])?,
        ImageOrientationPatient::new([1.0, 0.0, 0.0], [0.0, 1.0, 0.0])?,
        PixelSpacing::new(1.0, 1.0)?,
        ImageDimensions::new(rows, columns)?,
    )
}

fn make_frame(
    rows: u32,
    columns: u32,
    bits_allocated: u8,
    samples: Vec<u8>,
) -> Result<DecodedFrame, ocelli_pixel::PixelError> {
    Ok(DecodedFrame::new(
        description(rows, columns, bits_allocated)?,
        plane(rows, columns)?,
        samples,
    ))
}

fn key(series: u64, instance: u32, frame_index: u32) -> FrameKey {
    FrameKey::new(SeriesId(series), instance, frame_index)
}

#[test]
fn frame_bytes_is_the_allocated_buffer() -> Result<(), ocelli_pixel::PixelError> {
    // 512 * 512 * 2 = 524,288, hand-computed.
    let frame = make_frame(512, 512, 16, vec![0_u8; 524_288])?;
    assert_eq!(frame.bytes(), 524_288);
    // A malformed pair where the buffer disagrees with the description still
    // reports the buffer, because the buffer is the allocation.
    let short = make_frame(512, 512, 16, vec![0_u8; 100])?;
    assert_eq!(short.bytes(), 100);
    Ok(())
}

fn le_u32(raw: &[u8], at: usize) -> u32 {
    let mut out = 0_u32;
    for (index, byte) in raw.iter().skip(at).take(4).enumerate() {
        out |= u32::from(*byte) << u32::try_from(index * 8).unwrap_or(0);
    }
    out
}

fn le_u64(raw: &[u8], at: usize) -> u64 {
    let mut out = 0_u64;
    for (index, byte) in raw.iter().skip(at).take(8).enumerate() {
        out |= u64::from(*byte) << u32::try_from(index * 8).unwrap_or(0);
    }
    out
}

#[test]
fn ring_event_is_48_bytes_at_section_17_3_offsets() {
    assert_eq!(core::mem::size_of::<RingEvent>(), 48);
    let event = CacheEvent::Evicted {
        key: key(7, 3, 2),
        bytes: 524_288,
    }
    .encode(41);
    assert_eq!(event.viewport, u32::MAX);
    assert_eq!(event.seq, 41);
    let raw = bytemuck::bytes_of(&event);
    assert_eq!(raw.len(), 48);
    // kind at 0, viewport at 4, seq at 8, payload at 16.
    assert_eq!(le_u32(raw, 0), event.kind);
    assert_eq!(le_u32(raw, 4), u32::MAX);
    assert_eq!(le_u64(raw, 8), 41);
}

#[test]
fn eviction_payload_round_trips_little_endian() {
    let event = CacheEvent::Evicted {
        key: key(7, 3, 2),
        bytes: 524_288,
    }
    .encode(1);
    let p = &event.payload;
    assert_eq!(le_u64(p, 0), 7);
    assert_eq!(le_u32(p, 8), 3);
    assert_eq!(le_u32(p, 12), 2);
    assert_eq!(le_u64(p, 16), 524_288);
    assert!(p.iter().skip(24).all(|byte| *byte == 0));
}

#[test]
fn one_insert_evicting_two_yields_two_evicted_in_order() -> Result<(), ocelli_pixel::PixelError> {
    // Budget 250. Two 100-byte frames live, one 200-byte insert evicts both,
    // least recently used first.
    let mut cache = ImageCache::new(250);
    assert!(
        cache
            .insert(key(1, 1, 0), make_frame(10, 5, 16, vec![0_u8; 100])?)
            .is_empty()
    );
    assert!(
        cache
            .insert(key(1, 2, 0), make_frame(10, 5, 16, vec![0_u8; 100])?)
            .is_empty()
    );
    let events = cache.insert(key(1, 3, 0), make_frame(10, 10, 16, vec![0_u8; 200])?);
    assert_eq!(
        events.events(),
        [
            CacheEvent::Evicted {
                key: key(1, 1, 0),
                bytes: 100
            },
            CacheEvent::Evicted {
                key: key(1, 2, 0),
                bytes: 100
            },
        ]
    );
    Ok(())
}

#[test]
fn displaced_and_refused_are_not_evicted() -> Result<(), ocelli_pixel::PixelError> {
    let mut cache = ImageCache::new(250);
    assert!(
        cache
            .insert(key(1, 1, 0), make_frame(10, 5, 16, vec![0_u8; 100])?)
            .is_empty()
    );
    let events = cache.insert(key(1, 1, 0), make_frame(10, 5, 16, vec![1_u8; 100])?);
    assert!(!events.is_empty());
    assert_eq!(events.len(), 1);
    assert_eq!(
        events.events(),
        [CacheEvent::Displaced {
            key: key(1, 1, 0),
            bytes: 100
        }]
    );
    let events = cache.insert(key(1, 9, 0), make_frame(10, 5, 16, vec![0_u8; 251])?);
    assert_eq!(
        events.events(),
        [CacheEvent::Refused {
            key: key(1, 9, 0),
            bytes: 251
        }]
    );
    Ok(())
}

#[test]
fn seq_is_caller_supplied_and_viewport_is_always_the_sentinel() {
    for (seq, event) in [
        CacheEvent::Evicted {
            key: key(1, 1, 0),
            bytes: 1,
        },
        CacheEvent::Displaced {
            key: key(1, 1, 0),
            bytes: 1,
        },
        CacheEvent::Refused {
            key: key(1, 1, 0),
            bytes: 1,
        },
    ]
    .into_iter()
    .enumerate()
    {
        let encoded = event.encode(u64::try_from(seq).unwrap_or(0));
        assert_eq!(encoded.viewport, u32::MAX);
        assert_eq!(encoded.seq, u64::try_from(seq).unwrap_or(0));
    }
    // The three kinds are distinct numbers.
    let kinds = [
        CacheEvent::Evicted {
            key: key(1, 1, 0),
            bytes: 1,
        }
        .encode(0)
        .kind,
        CacheEvent::Displaced {
            key: key(1, 1, 0),
            bytes: 1,
        }
        .encode(0)
        .kind,
        CacheEvent::Refused {
            key: key(1, 1, 0),
            bytes: 1,
        }
        .encode(0)
        .kind,
    ];
    assert!(kinds[0] != kinds[1] && kinds[1] != kinds[2] && kinds[0] != kinds[2]);
}

#[test]
fn kind_numbers_are_pinned_to_the_wire_values() {
    // No registry exists until F-103, so these numbers are the wire contract
    // the JavaScript reader will switch on. Renumbering must go red here.
    assert_eq!(KIND_EVICTED, 1);
    assert_eq!(KIND_DISPLACED, 2);
    assert_eq!(KIND_REFUSED, 3);
    assert_eq!(
        CacheEvent::Evicted {
            key: key(1, 1, 0),
            bytes: 1
        }
        .encode(0)
        .kind,
        1
    );
    assert_eq!(
        CacheEvent::Displaced {
            key: key(1, 1, 0),
            bytes: 1
        }
        .encode(0)
        .kind,
        2
    );
    assert_eq!(
        CacheEvent::Refused {
            key: key(1, 1, 0),
            bytes: 1
        }
        .encode(0)
        .kind,
        3
    );
}

#[test]
fn raw_payload_starts_at_byte_16() {
    let event = CacheEvent::Evicted {
        key: key(7, 3, 2),
        bytes: 524_288,
    }
    .encode(1);
    let raw = bytemuck::bytes_of(&event);
    assert_eq!(le_u64(raw, 16), 7);
    assert_eq!(le_u32(raw, 24), 3);
    assert_eq!(le_u32(raw, 28), 2);
    assert_eq!(le_u64(raw, 32), 524_288);
}

#[test]
fn read_side_and_accessors_are_executed() -> Result<(), ocelli_pixel::PixelError> {
    let mut cache = ImageCache::new(250);
    let stored = make_frame(10, 5, 16, vec![9_u8; 100])?;
    assert_eq!(stored.samples(), [9_u8; 100].as_slice());
    assert_eq!(stored.description().dimensions.rows(), 10);
    assert_eq!(stored.plane().dimensions().columns(), 5);
    assert!(cache.insert(key(1, 1, 0), stored).is_empty());
    assert_eq!(cache.pressure().used, 100);
    assert_eq!(cache.pressure().budget, 250);
    assert_eq!(cache.pressure().tier, ocelli_cache::CacheTier::Decoded);
    assert_eq!(
        cache.get(&key(1, 1, 0)).map(DecodedFrame::samples),
        Some([9_u8; 100].as_slice())
    );
    assert!(cache.get(&key(1, 2, 0)).is_none());
    assert!(cache.remove(&key(1, 1, 0)).is_some());
    assert_eq!(cache.pressure().used, 0);
    assert!(cache.remove(&key(1, 1, 0)).is_none());
    Ok(())
}

#[test]
fn one_insert_can_displace_and_evict_with_displaced_first() -> Result<(), ocelli_pixel::PixelError>
{
    // Budget 250, two 100-byte frames live. Replacing the first with a
    // 200-byte frame releases 100, then evicts the second to fit.
    let mut cache = ImageCache::new(250);
    assert!(
        cache
            .insert(key(1, 1, 0), make_frame(10, 5, 16, vec![0_u8; 100])?)
            .is_empty()
    );
    assert!(
        cache
            .insert(key(1, 2, 0), make_frame(10, 5, 16, vec![0_u8; 100])?)
            .is_empty()
    );
    let events = cache.insert(key(1, 1, 0), make_frame(10, 10, 16, vec![0_u8; 200])?);
    assert_eq!(events.len(), 2);
    assert_eq!(
        events.events(),
        [
            CacheEvent::Displaced {
                key: key(1, 1, 0),
                bytes: 100
            },
            CacheEvent::Evicted {
                key: key(1, 2, 0),
                bytes: 100
            },
        ]
    );
    Ok(())
}

proptest! {
    /// The events of every insert account for what the cache stopped holding.
    /// A shadow map is updated from the events alone, never from the cache,
    /// and its byte sum must equal `pressure().used` after every insert.
    #[test]
    fn events_account_for_the_bytes_the_cache_stopped_holding(
        ops in prop::collection::vec((0_u32..4, 1_usize..300), 1..40)
    ) {
        let mut cache = ImageCache::new(250);
        let mut live: BTreeMap<FrameKey, usize> = BTreeMap::new();
        let mut left_bytes = 0_usize;
        let mut admitted_bytes = 0_usize;
        for (instance, bytes) in ops {
            let frame_key = key(1, instance, 0);
            let frame = make_frame(1, 1, 8, vec![0_u8; bytes])
                .map_err(|error| TestCaseError::fail(format!("{error:?}")))?;
            let events = cache.insert(frame_key, frame);
            let mut refused = false;
            for event in events.events() {
                match *event {
                    CacheEvent::Evicted { key: gone, bytes: gone_bytes } => {
                        prop_assert_eq!(live.remove(&gone), Some(gone_bytes));
                        left_bytes += gone_bytes;
                    }
                    CacheEvent::Displaced { key: gone, bytes: gone_bytes } => {
                        prop_assert_eq!(gone, frame_key);
                        prop_assert_eq!(live.remove(&gone), Some(gone_bytes));
                        left_bytes += gone_bytes;
                    }
                    CacheEvent::Refused { key: gone, bytes: gone_bytes } => {
                        prop_assert_eq!(gone, frame_key);
                        prop_assert_eq!(gone_bytes, bytes);
                        refused = true;
                    }
                }
            }
            if !refused {
                live.insert(frame_key, bytes);
                admitted_bytes += bytes;
            }
            let live_bytes: usize = live.values().sum();
            prop_assert_eq!(cache.pressure().used, live_bytes);
            prop_assert_eq!(admitted_bytes - live_bytes, left_bytes);
        }
    }
}
