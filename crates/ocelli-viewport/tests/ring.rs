//! F-101 event ring. Kind 6 is presented. Kinds 1 to 5 stay cache events.

use ocelli_cache::{
    KIND_BUDGET_CHANGED, KIND_DISPLACED, KIND_EVICTED, KIND_PRESSURE, KIND_REFUSED,
};
use ocelli_viewport::ring::{EventRing, KIND_PRESENTED, RING_CAPACITY};

fn read_u32(bytes: &[u8], at: usize) -> u32 {
    let mut buf = [0_u8; 4];
    let Some(slot) = bytes.get(at..at + 4) else {
        return u32::MAX;
    };
    buf.copy_from_slice(slot);
    u32::from_le_bytes(buf)
}

#[test]
fn presented_is_kind_6_and_a_full_ring_drops_the_new_event() {
    assert_eq!(KIND_PRESENTED, 6);
    assert_ne!(KIND_PRESENTED, KIND_EVICTED);
    assert_ne!(KIND_PRESENTED, KIND_DISPLACED);
    assert_ne!(KIND_PRESENTED, KIND_REFUSED);
    assert_ne!(KIND_PRESENTED, KIND_PRESSURE);
    assert_ne!(KIND_PRESENTED, KIND_BUDGET_CHANGED);

    let mut ring = EventRing::new();
    let mut seq = 0_u64;
    for index in 0..RING_CAPACITY {
        seq = ring.push_presented(3, index, 20, 10, 8);
    }
    assert_eq!(seq, u64::from(RING_CAPACITY - 1));
    assert_eq!(ring.pending(), RING_CAPACITY);
    assert_eq!(ring.dropped(), 0);
    ring.push_presented(3, 999, 1, 1, 0);
    assert_eq!(ring.dropped(), 1);
    assert_eq!(ring.pending(), RING_CAPACITY);

    let mut out = vec![0_u8; 16 + 256 * 48];
    let copied = ring.copy_pending(&mut out);
    assert_eq!(copied, 16 + 256 * 48);
    assert_eq!(read_u32(&out, 0), RING_CAPACITY);
    assert_eq!(read_u32(&out, 8), RING_CAPACITY);
    assert_eq!(read_u32(&out, 4), 0);
    assert_eq!(read_u32(&out, 12), 1);
    assert_eq!(read_u32(&out, 16), KIND_PRESENTED);
    assert_eq!(read_u32(&out, 20), 3);
    // Event layout is kind, viewport, seq u64, then payload. Generation is
    // the first payload word, at byte 32 of the copied buffer, not byte 28.
    assert_eq!(read_u32(&out, 32), 0);
    assert_eq!(read_u32(&out, 36), 20);
    assert_eq!(read_u32(&out, 40), 10);
    assert_eq!(read_u32(&out, 44), 8);
    let last = 16 + 255 * 48;
    assert_eq!(read_u32(&out, last + 16), 255);
    assert_ne!(read_u32(&out, last + 16), 999);
    assert_eq!(ring.pending(), 0);
    assert_eq!(ring.copy_pending(&mut out), 16);

    let wrapped_seq = ring.push_presented(3, 1000, 4, 2, 1);
    assert_eq!(wrapped_seq, u64::from(RING_CAPACITY) + 1);
    let mut wrapped = vec![0_u8; 16 + 48];
    assert_eq!(ring.copy_pending(&mut wrapped), 16 + 48);
    assert_eq!(read_u32(&wrapped, 32), 1000);
    assert_eq!(read_u32(&wrapped, 36), 4);
    assert_eq!(ring.pending(), 0);

    let mut short = [0_u8; 15];
    ring.push_presented(3, 7, 1, 1, 0);
    assert_eq!(ring.copy_pending(&mut short), 0);
    assert_eq!(ring.pending(), 1);
    assert_eq!(short, [0_u8; 15]);
}
