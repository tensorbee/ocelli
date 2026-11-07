//! F-032, the decoded tier, its eviction events and the section 17.3 encoding.
//!
//! Fixtures are hand-computed from the plan. A 512 by 512 sixteen-bit frame is
//! 512 * 512 * 2 = 524,288 bytes, from Rows and Columns in DICOM PS3.3
//! C.7.6.3.1 at two bytes per stored sample. The `RingEvent` layout is section
//! 17.3 of `docs/hld/14-the-boundary-in-code.md`, 48 bytes with `kind` at 0,
//! `viewport` at 4, `seq` at 8 and `payload` at 16.

use std::collections::BTreeMap;

use ocelli_cache::{
    Budgeted, CacheEvent, ContentId, DecodedFrame, FrameKey, FrameLookup, FrameSource, ImageCache,
    KIND_DISPLACED, KIND_EVICTED, KIND_REFUSED, KIND_RELEASED, RingEvent, SeriesId,
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

/// A fixed content id for the tests about eviction and byte accounting,
/// where the identity is not what is under test.
fn content() -> ContentId {
    ContentId::of(b"1.2.3", b"1.2.840.10008.1.2.1", b"encoded bytes")
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
fn contains_does_not_refresh_the_oldest_key() -> Result<(), ocelli_pixel::PixelError> {
    // Budget 300. Three 100-byte frames. The first is the oldest. `contains`
    // must not make it the newest, so the next 100-byte insert still evicts it.
    let mut cache = ImageCache::new(300);
    let oldest = key(1, 1, 0);
    assert!(
        cache
            .insert(oldest, content(), make_frame(10, 5, 16, vec![1_u8; 100])?)
            .is_empty()
    );
    assert!(
        cache
            .insert(
                key(1, 2, 0),
                content(),
                make_frame(10, 5, 16, vec![2_u8; 100])?
            )
            .is_empty()
    );
    assert!(
        cache
            .insert(
                key(1, 3, 0),
                content(),
                make_frame(10, 5, 16, vec![3_u8; 100])?
            )
            .is_empty()
    );
    assert!(cache.contains(&oldest));
    let events = cache.insert(
        key(1, 4, 0),
        content(),
        make_frame(10, 5, 16, vec![4_u8; 100])?,
    );
    assert_eq!(
        events.events(),
        [CacheEvent::Evicted {
            key: oldest,
            bytes: 100
        }]
    );
    assert!(!cache.contains(&oldest));
    Ok(())
}

#[test]
fn one_insert_evicting_two_yields_two_evicted_in_order() -> Result<(), ocelli_pixel::PixelError> {
    // Budget 250. Two 100-byte frames live, one 200-byte insert evicts both,
    // least recently used first.
    let mut cache = ImageCache::new(250);
    assert!(
        cache
            .insert(
                key(1, 1, 0),
                content(),
                make_frame(10, 5, 16, vec![0_u8; 100])?
            )
            .is_empty()
    );
    assert!(
        cache
            .insert(
                key(1, 2, 0),
                content(),
                make_frame(10, 5, 16, vec![0_u8; 100])?
            )
            .is_empty()
    );
    let events = cache.insert(
        key(1, 3, 0),
        content(),
        make_frame(10, 10, 16, vec![0_u8; 200])?,
    );
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
            .insert(
                key(1, 1, 0),
                content(),
                make_frame(10, 5, 16, vec![0_u8; 100])?
            )
            .is_empty()
    );
    let events = cache.insert(
        key(1, 1, 0),
        content(),
        make_frame(10, 5, 16, vec![1_u8; 100])?,
    );
    assert!(!events.is_empty());
    assert_eq!(events.len(), 1);
    assert_eq!(
        events.events(),
        [CacheEvent::Displaced {
            key: key(1, 1, 0),
            bytes: 100
        }]
    );
    let events = cache.insert(
        key(1, 9, 0),
        content(),
        make_frame(10, 5, 16, vec![0_u8; 251])?,
    );
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
    assert_eq!(KIND_RELEASED, 7);
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
    assert_eq!(
        CacheEvent::Released {
            series: SeriesId(1),
            frames: 1,
            bytes: 1
        }
        .encode(0)
        .kind,
        7
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
    assert!(cache.insert(key(1, 1, 0), content(), stored).is_empty());
    assert_eq!(cache.pressure().used, 100);
    assert_eq!(cache.pressure().budget, 250);
    assert_eq!(cache.pressure().tier, ocelli_cache::CacheTier::Decoded);
    let source = FrameSource {
        content: content(),
        description: description(10, 5, 16)?,
        plane: plane(10, 5)?,
    };
    let served = cache.get(&key(1, 1, 0), &source);
    match served {
        FrameLookup::Hit(frame) => assert_eq!(frame.samples(), [9_u8; 100].as_slice()),
        other => unreachable!("the frame is live for its own source: {other:?}"),
    }
    assert_eq!(cache.get(&key(1, 2, 0), &source), FrameLookup::Miss);
    assert!(cache.remove(&key(1, 1, 0)).is_some());
    assert_eq!(cache.pressure().used, 0);
    assert!(cache.remove(&key(1, 1, 0)).is_none());
    Ok(())
}

#[test]
fn a_hit_refreshes_recency_so_the_second_is_evicted() -> Result<(), ocelli_pixel::PixelError> {
    // The state table's row where a hit makes the frame the most recently
    // used, watched through the `ImageCache` entry point. The budget holds
    // exactly two 32-byte frames, so the third frame's insert must evict
    // one. The hit was on the first, so the second is the one evicted. A
    // get that stopped refreshing recency would leave the first the older
    // one and evict it instead, which is the mutation this is red on.
    let mut cache = ImageCache::new(64);
    let first = make_frame(4, 4, 16, vec![0x11_u8; 32])?;
    let second = make_frame(4, 4, 16, vec![0x55_u8; 32])?;
    let third = make_frame(4, 4, 16, vec![0x99_u8; 32])?;
    let source = FrameSource {
        content: content(),
        description: description(4, 4, 16)?,
        plane: plane(4, 4)?,
    };
    assert!(
        cache
            .insert(key(1, 0, 0), content(), first.clone())
            .is_empty()
    );
    assert!(cache.insert(key(1, 1, 0), content(), second).is_empty());
    assert_eq!(cache.get(&key(1, 0, 0), &source), FrameLookup::Hit(&first));
    let events = cache.insert(key(1, 2, 0), content(), third);
    assert_eq!(
        events.events(),
        [CacheEvent::Evicted {
            key: key(1, 1, 0),
            bytes: 32,
        }]
    );
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
            .insert(
                key(1, 1, 0),
                content(),
                make_frame(10, 5, 16, vec![0_u8; 100])?
            )
            .is_empty()
    );
    assert!(
        cache
            .insert(
                key(1, 2, 0),
                content(),
                make_frame(10, 5, 16, vec![0_u8; 100])?
            )
            .is_empty()
    );
    let events = cache.insert(
        key(1, 1, 0),
        content(),
        make_frame(10, 10, 16, vec![0_u8; 200])?,
    );
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

#[test]
fn release_series_releases_one_series() -> Result<(), ocelli_pixel::PixelError> {
    // The state table's first three rows plus a frame at each end of the
    // series range, so the range ends are tested. Series 5 holds five
    // frames of 32 bytes at (5, 0, 0), (5, 1, 0), (5, 2, 0), (5, 3, 0) and
    // (5, u32::MAX, u32::MAX), the outer two being the range's own end
    // keys, and series 6 holds one, so 6 * 32 = 192 fills a 192-byte
    // budget. The expected frames and bytes are this fixture's,
    // hand-computed, and not the state table's figures: the release takes
    // series 5's 5 * 32 = 160 and leaves series 6's 32.
    let mut cache = ImageCache::new(192);
    assert!(
        cache
            .insert(
                key(5, 0, 0),
                content(),
                make_frame(1, 1, 8, vec![0_u8; 32])?
            )
            .is_empty()
    );
    for instance in 1_u32..=3 {
        assert!(
            cache
                .insert(
                    key(5, instance, 0),
                    content(),
                    make_frame(1, 1, 8, vec![0_u8; 32])?
                )
                .is_empty()
        );
    }
    assert!(
        cache
            .insert(
                key(5, u32::MAX, u32::MAX),
                content(),
                make_frame(1, 1, 8, vec![0_u8; 32])?
            )
            .is_empty()
    );
    assert!(
        cache
            .insert(
                key(6, 0, 0),
                content(),
                make_frame(1, 1, 8, vec![0_u8; 32])?
            )
            .is_empty()
    );
    assert_eq!(cache.pressure().used, 192);

    let events = cache.release_series(SeriesId(5));
    assert_eq!(
        events.events(),
        [CacheEvent::Released {
            series: SeriesId(5),
            frames: 5,
            bytes: 160,
        }]
    );
    assert_eq!(cache.pressure().used, 32);
    // Series 6's first key is exactly the key an end bound of
    // (series + 1, 0, 0) inclusive would take, so this is the contains
    // that rejects it. The extreme keys are the ones an end bound of
    // (series, 3, 0) or (series, u32::MAX, 0), or a start bound of
    // (series, 1, 0), would leave behind.
    assert!(cache.contains(&key(6, 0, 0)));
    assert!(!cache.contains(&key(5, 0, 0)));
    assert!(!cache.contains(&key(5, 1, 0)));
    assert!(!cache.contains(&key(5, 2, 0)));
    assert!(!cache.contains(&key(5, 3, 0)));
    assert!(!cache.contains(&key(5, u32::MAX, u32::MAX)));

    // A repeat finds nothing held and emits nothing.
    let events = cache.release_series(SeriesId(5));
    assert!(events.is_empty());
    assert_eq!(cache.pressure().used, 32);

    // A series that holds nothing emits nothing.
    let events = cache.release_series(SeriesId(7));
    assert!(events.is_empty());
    assert_eq!(cache.pressure().used, 32);

    // Series 6's frame is the only survivor and leaves by nothing but a
    // later eviction. 32 + 161 = 193 is one over the budget, so the next
    // insert evicts it. A range that bled into series 6 would have
    // released it here already and evicted nothing.
    let events = cache.insert(
        key(7, 1, 0),
        content(),
        make_frame(1, 1, 8, vec![0_u8; 161])?,
    );
    assert_eq!(
        events.events(),
        [CacheEvent::Evicted {
            key: key(6, 0, 0),
            bytes: 32,
        }]
    );
    Ok(())
}

#[test]
fn a_reprocessed_object_in_the_same_slot_is_not_served() -> Result<(), ocelli_pixel::PixelError> {
    // Decision 5's validity paragraph on one slot. A 4 by 4 sixteen-bit
    // frame is 4 * 4 * 2 = 32 bytes, from Rows and Columns in DICOM PS3.3
    // C.7.6.3.1 at two bytes per stored sample, and the budget is exactly
    // one frame, so `pressure().used` walks 32, 0, 32 by hand.
    let description = description(4, 4, 16)?;
    let plane = plane(4, 4)?;
    let first = make_frame(4, 4, 16, vec![0x11_u8; 32])?;
    let reprocessed = make_frame(4, 4, 16, vec![0x55_u8; 32])?;
    let first_source = FrameSource {
        content: ContentId::of(b"1.2.3", b"1.2.840.10008.1.2.1", b"first processing"),
        description,
        plane,
    };
    let second_source = FrameSource {
        content: ContentId::of(b"1.2.3", b"1.2.840.10008.1.2.1", b"re-processed"),
        description,
        plane,
    };

    let mut cache = ImageCache::new(32);
    assert!(
        cache
            .insert(key(1, 0, 0), first_source.content, first.clone())
            .is_empty()
    );
    assert_eq!(cache.pressure().used, 32);

    // The frame the slot holds is the one the source presents. The hit is
    // the frame itself, samples included.
    assert_eq!(
        cache.get(&key(1, 0, 0), &first_source),
        FrameLookup::Hit(&first)
    );
    assert_eq!(cache.pressure().used, 32);

    // The caller now holds a re-processed object whose content id differs
    // under the same transfer syntax. The held frame leaves, the lookup
    // reports the 32 bytes it was admitted at and the budget reads zero.
    assert_eq!(
        cache.get(&key(1, 0, 0), &second_source),
        FrameLookup::Stale { bytes: 32 }
    );
    assert_eq!(cache.pressure().used, 0);

    // The old version is a miss now. The slot holds at most one version.
    assert_eq!(cache.get(&key(1, 0, 0), &first_source), FrameLookup::Miss);

    // Admitting the re-processed frame is a clean insert with nothing
    // displaced, and its own source hits it with the new samples.
    assert!(
        cache
            .insert(key(1, 0, 0), second_source.content, reprocessed.clone())
            .is_empty()
    );
    assert_eq!(cache.pressure().used, 32);
    assert_eq!(
        cache.get(&key(1, 0, 0), &second_source),
        FrameLookup::Hit(&reprocessed)
    );

    // The first version is stale again, not an alternating hit.
    assert_eq!(
        cache.get(&key(1, 0, 0), &first_source),
        FrameLookup::Stale { bytes: 32 }
    );
    assert_eq!(cache.pressure().used, 0);
    Ok(())
}

#[test]
fn metadata_changes_are_stale() -> Result<(), ocelli_pixel::PixelError> {
    // Decision 3: the description and the plane are compared exactly, not
    // hashed. A re-process can correct Image Position or Pixel
    // Representation and keep both the UID and the encoded bytes, so the
    // same content id with a moved plane or a changed signedness is stale.
    // This rejects an implementation that compares the content id only.
    let mut cache = ImageCache::new(32);
    let description = description(4, 4, 16)?;
    let plane = plane(4, 4)?;
    let content = ContentId::of(b"1.2.3", b"1.2.840.10008.1.2.1", b"encoded bytes");
    assert!(
        cache
            .insert(
                key(1, 0, 0),
                content,
                make_frame(4, 4, 16, vec![0x11_u8; 32])?
            )
            .is_empty()
    );

    // Image Position moved 0.5 mm along x, DICOM PS3.3 C.7.6.2.1.1. The
    // content id and the description are unchanged and the lookup is stale.
    let moved = ImagePlane::new(
        ImagePositionPatient::new([0.5, 0.0, 0.0])?,
        ImageOrientationPatient::new([1.0, 0.0, 0.0], [0.0, 1.0, 0.0])?,
        PixelSpacing::new(1.0, 1.0)?,
        ImageDimensions::new(4, 4)?,
    )?;
    assert_eq!(
        cache.get(
            &key(1, 0, 0),
            &FrameSource {
                content,
                description,
                plane: moved,
            }
        ),
        FrameLookup::Stale { bytes: 32 }
    );

    // The slot is empty after the stale lookup, so the frame goes back in
    // for the second case.
    assert!(
        cache
            .insert(
                key(1, 0, 0),
                content,
                make_frame(4, 4, 16, vec![0x11_u8; 32])?
            )
            .is_empty()
    );

    // Pixel Representation 0 against 1, DICOM PS3.3 C.7.6.3.1.1 tag
    // (0028,0103). The content id and the plane are unchanged and the
    // lookup is stale.
    let layout = SampleLayout::new(1, PhotometricInterpretation::Monochrome2, None)?;
    let signed = StoredBits::new(16, 16, 15, PixelRepresentation::Signed)?;
    let signed_description =
        StoredPixelDescription::new(ImageDimensions::new(4, 4)?, layout, signed);
    assert_eq!(
        cache.get(
            &key(1, 0, 0),
            &FrameSource {
                content,
                description: signed_description,
                plane,
            }
        ),
        FrameLookup::Stale { bytes: 32 }
    );

    // The frame's own source still answers after both refusals.
    assert!(
        cache
            .insert(
                key(1, 0, 0),
                content,
                make_frame(4, 4, 16, vec![0x11_u8; 32])?
            )
            .is_empty()
    );
    let own = cache.get(
        &key(1, 0, 0),
        &FrameSource {
            content,
            description,
            plane,
        },
    );
    assert!(matches!(own, FrameLookup::Hit(_)));
    Ok(())
}

#[test]
fn stale_removal_is_explicit() -> Result<(), ocelli_pixel::PixelError> {
    // Decision 4: a mismatch removes the held frame explicitly and emits no
    // event. The lookup reports the bytes of the frame it removed and the
    // pressure drops by them, with no event of any kind. The insert after
    // the stale finds an empty slot and displaces nothing, which rejects an
    // implementation that reports the removal on the next insert. An insert
    // over a live slot still displaces, which is the control.
    let mut cache = ImageCache::new(32);
    let first = make_frame(4, 4, 16, vec![0x11_u8; 32])?;
    let second = make_frame(4, 4, 16, vec![0x55_u8; 32])?;
    let third = make_frame(4, 4, 16, vec![0x99_u8; 32])?;
    let first_source = FrameSource {
        content: ContentId::of(b"1.2.3", b"1.2.840.10008.1.2.1", b"first processing"),
        description: description(4, 4, 16)?,
        plane: plane(4, 4)?,
    };
    let second_source = FrameSource {
        content: ContentId::of(b"1.2.3", b"1.2.840.10008.1.2.1", b"re-processed"),
        description: description(4, 4, 16)?,
        plane: plane(4, 4)?,
    };
    let third_source = FrameSource {
        content: ContentId::of(b"1.2.3", b"1.2.840.10008.1.2.1", b"third processing"),
        description: description(4, 4, 16)?,
        plane: plane(4, 4)?,
    };

    assert!(
        cache
            .insert(key(1, 0, 0), first_source.content, first)
            .is_empty()
    );
    assert_eq!(
        cache.get(&key(1, 0, 0), &second_source),
        FrameLookup::Stale { bytes: 32 }
    );
    assert_eq!(cache.pressure().used, 0);

    // The slot is empty, so the re-processed frame is admitted with no
    // event at all. A stale removal reported on the next insert would
    // show up here.
    let events = cache.insert(key(1, 0, 0), second_source.content, second);
    assert!(events.is_empty());
    assert_eq!(cache.pressure().used, 32);

    // The control. An insert over a live slot displaces it, by key, with
    // the bytes the displaced frame was admitted at.
    let events = cache.insert(key(1, 0, 0), third_source.content, third);
    assert_eq!(
        events.events(),
        [CacheEvent::Displaced {
            key: key(1, 0, 0),
            bytes: 32,
        }]
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
        // A fixed content id. The identity is not what this property drives,
        // and every insert here goes through the same path.
        let content = ContentId::of(b"1.2.3", b"1.2.840.10008.1.2.1", b"encoded bytes");
        let mut live: BTreeMap<FrameKey, usize> = BTreeMap::new();
        let mut left_bytes = 0_usize;
        let mut admitted_bytes = 0_usize;
        for (instance, bytes) in ops {
            let frame_key = key(1, instance, 0);
            let frame = make_frame(1, 1, 8, vec![0_u8; bytes])
                .map_err(|error| TestCaseError::fail(format!("{error:?}")))?;
            let events = cache.insert(frame_key, content, frame);
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
                    CacheEvent::Released { .. } => {
                        // `release_series` is the only producer and is not
                        // on the insert path this property drives.
                        return Err(TestCaseError::fail("a Released event left an insert"));
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
