//! Section 17.3's ring. Cache kinds 1 to 5 stay in `ocelli-cache`.
//! Kind 6 is a presented frame. A full ring drops the new event and counts it.

use ocelli_cache::RingEvent;

/// Presented frame. Not a cache kind.
pub const KIND_PRESENTED: u32 = 6;

/// Power-of-two capacity named by the F-101 plan.
pub const RING_CAPACITY: u32 = 256;

const HEADER_BYTES: usize = 16;
const EVENT_BYTES: usize = 48;

/// One producer, one consumer, no shared memory.
#[derive(Clone, Debug)]
pub struct EventRing {
    head: u32,
    tail: u32,
    dropped: u32,
    seq: u64,
    slots: [RingEvent; RING_CAPACITY as usize],
}

impl Default for EventRing {
    fn default() -> Self {
        Self::new()
    }
}

impl EventRing {
    /// Empty ring. Capacity is 256.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            head: 0,
            tail: 0,
            dropped: 0,
            seq: 0,
            slots: [empty_event(); RING_CAPACITY as usize],
        }
    }

    /// Events waiting. Saturates at the capacity.
    #[must_use]
    pub const fn pending(&self) -> u32 {
        self.head.wrapping_sub(self.tail)
    }

    /// Events dropped because the ring was full.
    #[must_use]
    pub const fn dropped(&self) -> u32 {
        self.dropped
    }

    /// Push one event. A full ring increments `dropped` and keeps the old events.
    pub fn push(&mut self, event: RingEvent) {
        if self.pending() == RING_CAPACITY {
            self.dropped = self.dropped.saturating_add(1);
            return;
        }
        let index = usize::try_from(self.head & (RING_CAPACITY - 1)).unwrap_or(0);
        let Some(slot) = self.slots.get_mut(index) else {
            self.dropped = self.dropped.saturating_add(1);
            return;
        };
        *slot = event;
        self.head = self.head.wrapping_add(1);
    }

    /// Record a presented frame and return the sequence number assigned to it.
    ///
    /// The payload is generation, drawn width, drawn height and flags, each
    /// little-endian `u32`. The drawn extent is the reduced grid during
    /// interaction and the full device on settle. The rest of the 32 payload
    /// bytes stay zero.
    pub fn push_presented(
        &mut self,
        viewport: u32,
        generation: u32,
        drawn_width: u32,
        drawn_height: u32,
        flags: u32,
    ) -> u64 {
        let seq = self.seq;
        self.seq = self.seq.wrapping_add(1);
        let mut payload = [0_u8; 32];
        payload[0..4].copy_from_slice(&generation.to_le_bytes());
        payload[4..8].copy_from_slice(&drawn_width.to_le_bytes());
        payload[8..12].copy_from_slice(&drawn_height.to_le_bytes());
        payload[12..16].copy_from_slice(&flags.to_le_bytes());
        self.push(RingEvent {
            kind: KIND_PRESENTED,
            viewport,
            seq,
            payload,
        });
        seq
    }

    /// Copy a 16-byte header and the pending events in order, then advance.
    ///
    /// The copy's tail is 0 and its head is the pending count, so a reader
    /// walks a linear prefix. A short buffer copies nothing and does not
    /// advance.
    pub fn copy_pending(&mut self, out: &mut [u8]) -> usize {
        let pending = self.pending();
        let Ok(count) = usize::try_from(pending) else {
            return 0;
        };
        let bytes = HEADER_BYTES.saturating_add(count.saturating_mul(EVENT_BYTES));
        if out.len() < bytes {
            return 0;
        }
        write_u32(out, 0, pending);
        write_u32(out, 4, 0);
        write_u32(out, 8, RING_CAPACITY);
        write_u32(out, 12, self.dropped);
        let mut produced = 0_u32;
        while produced < pending {
            let index = usize::try_from((self.tail.wrapping_add(produced)) & (RING_CAPACITY - 1))
                .unwrap_or(0);
            let Some(event) = self.slots.get(index) else {
                break;
            };
            let at = HEADER_BYTES.saturating_add(
                usize::try_from(produced)
                    .unwrap_or(0)
                    .saturating_mul(EVENT_BYTES),
            );
            write_event(out, at, event);
            produced = produced.saturating_add(1);
        }
        self.tail = self.head;
        bytes
    }
}

const fn empty_event() -> RingEvent {
    RingEvent {
        kind: 0,
        viewport: 0,
        seq: 0,
        payload: [0_u8; 32],
    }
}

fn write_u32(out: &mut [u8], at: usize, value: u32) {
    let Some(end) = at.checked_add(4) else {
        return;
    };
    let Some(slot) = out.get_mut(at..end) else {
        return;
    };
    slot.copy_from_slice(&value.to_le_bytes());
}

fn write_u64(out: &mut [u8], at: usize, value: u64) {
    let Some(end) = at.checked_add(8) else {
        return;
    };
    let Some(slot) = out.get_mut(at..end) else {
        return;
    };
    slot.copy_from_slice(&value.to_le_bytes());
}

fn write_event(out: &mut [u8], at: usize, event: &RingEvent) {
    write_u32(out, at, event.kind);
    write_u32(out, at.saturating_add(4), event.viewport);
    write_u64(out, at.saturating_add(8), event.seq);
    let payload_at = at.saturating_add(16);
    let Some(end) = payload_at.checked_add(32) else {
        return;
    };
    let Some(slot) = out.get_mut(payload_at..end) else {
        return;
    };
    slot.copy_from_slice(&event.payload);
}
