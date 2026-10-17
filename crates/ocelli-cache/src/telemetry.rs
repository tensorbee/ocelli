//! Pressure readings and budget-change events for F-034.
//!
//! This module holds no cache and owns no budget. The caller passes the
//! `Pressure` values and `Admission` outcomes it already has. Nothing here
//! reaches JavaScript. F-101 carries the encoding across.

use crate::image::{KIND_BUDGET_CHANGED, KIND_PRESSURE, RingEvent};
use crate::lru::{Admission, Budgeted};
use crate::tier::{CacheTier, Pressure, PressureBand};

/// One tier's pressure, plus what happened since the previous reading.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TierTelemetry {
    /// The tier's own reading.
    pub pressure: Pressure,
    /// Live entries at reading time.
    pub entries: u32,
    /// Entries evicted to make room since the previous reading.
    pub evicted: u32,
    /// Bytes those evictions were admitted at, as `Budgeted::bytes` reports
    /// them. A value whose `bytes` does not change reports the admitted size.
    pub evicted_bytes: u64,
    /// Entries the budget could not hold since the previous reading.
    pub refused: u32,
}

/// A snapshot across section 8's three tiers.
///
/// The caller builds this literally from the three readings. `dropped` is
/// section 17.3's ring overflow, which this crate does not own, so the
/// boundary layer fills it and a native caller leaves it at zero.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CacheTelemetry {
    /// The encoded tier.
    pub encoded: TierTelemetry,
    /// The decoded tier.
    pub decoded: TierTelemetry,
    /// The GPU tier.
    pub gpu: TierTelemetry,
    /// Ring events the producer dropped because the shell did not drain.
    pub dropped: u32,
}

/// Running counters between readings. Taking one resets it.
///
/// A replacement is not counted. Deviation D-24 kept `displaced` apart from
/// `evicted` so a telemetry reading would not have to guess.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TierCounters {
    /// Entries evicted to make room.
    pub evicted: u32,
    /// Bytes those evictions reported.
    pub evicted_bytes: u64,
    /// Entries the budget refused.
    pub refused: u32,
}

impl TierCounters {
    /// Fold one insert or one `set_budget` into the counters.
    ///
    /// `displaced` is ignored. Counts saturate at the top of `u32` rather than
    /// wrapping, because a wrapped eviction count reads as a quiet cache.
    pub fn record<K, V: Budgeted>(&mut self, admission: &Admission<K, V>) {
        for (_, value) in &admission.evicted {
            self.evicted = self.evicted.saturating_add(1);
            let bytes = u64::try_from(value.bytes()).unwrap_or(u64::MAX);
            self.evicted_bytes = self.evicted_bytes.saturating_add(bytes);
        }
        if admission.refused.is_some() {
            self.refused = self.refused.saturating_add(1);
        }
    }

    /// The accumulated values, and this counter returns to zero.
    #[must_use]
    pub fn take(&mut self) -> Self {
        let taken = *self;
        *self = Self::default();
        taken
    }
}

impl PressureBand {
    /// One pressure event when `telemetry` has left this band, otherwise none.
    ///
    /// The caller stores the band it last reported. Inserts inside a band
    /// emit nothing, which is what keeps a series load from filling the ring.
    #[must_use]
    pub fn crossing(self, telemetry: TierTelemetry) -> Option<TelemetryEvent> {
        if telemetry.pressure.band() == self {
            None
        } else {
            Some(TelemetryEvent::Pressure {
                tier: telemetry.pressure.tier,
                telemetry,
            })
        }
    }
}

/// A telemetry fact the shell hears about.
///
/// A requested reading uses [`TelemetryEvent::Pressure`] as well. The shell
/// asks with a typed command and the answer leaves as a ring event, so there
/// is still one upward channel.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TelemetryEvent {
    /// A band crossing, or a reading the shell asked for.
    Pressure {
        /// The tier that moved, repeated from `telemetry.pressure.tier`.
        tier: CacheTier,
        /// The reading.
        telemetry: TierTelemetry,
    },
    /// A budget moved.
    ///
    /// The caller emits one eviction event per entry `set_budget` returned,
    /// in that order, and this event last. `evicted` is that count.
    BudgetChanged {
        /// The tier whose budget moved.
        tier: CacheTier,
        /// The budget now in force.
        budget: usize,
        /// How many entries the change evicted.
        evicted: u32,
        /// The bytes those entries reported.
        evicted_bytes: u64,
        /// Section 17.3's dropped count at the moment of the change. Zero
        /// until a ring exists.
        dropped: u32,
    },
}

impl TelemetryEvent {
    /// Encode into section 17.3's envelope.
    ///
    /// `viewport` is `u32::MAX`, the same sentinel F-032 uses for an event
    /// that belongs to no viewport. Little-endian, because wasm32 is
    /// little-endian and section 17.1 says the wire format is.
    ///
    /// A pressure payload carries `used`, `budget`, `entries`, `evicted` and
    /// `refused`. `evicted_bytes` stays on [`TierTelemetry`] and is not on the
    /// wire, because `used` against `budget` is what a threshold needs.
    #[must_use]
    pub fn encode(&self, seq: u64) -> RingEvent {
        let mut payload = [0_u8; 32];
        let kind = match *self {
            Self::Pressure { tier, telemetry } => {
                payload[0] = tier_code(tier);
                write_u64(&mut payload, 4, usize_as_u64(telemetry.pressure.used));
                write_u64(&mut payload, 12, usize_as_u64(telemetry.pressure.budget));
                write_u32(&mut payload, 20, telemetry.entries);
                write_u32(&mut payload, 24, telemetry.evicted);
                write_u32(&mut payload, 28, telemetry.refused);
                KIND_PRESSURE
            }
            Self::BudgetChanged {
                tier,
                budget,
                evicted,
                evicted_bytes,
                dropped,
            } => {
                payload[0] = tier_code(tier);
                write_u64(&mut payload, 4, usize_as_u64(budget));
                write_u64(&mut payload, 12, evicted_bytes);
                write_u32(&mut payload, 20, evicted);
                write_u32(&mut payload, 24, dropped);
                KIND_BUDGET_CHANGED
            }
        };
        RingEvent {
            kind,
            viewport: u32::MAX,
            seq,
            payload,
        }
    }
}

/// Section 8's order, pinned as a wire discriminant. Encoded, decoded, GPU.
fn tier_code(tier: CacheTier) -> u8 {
    match tier {
        CacheTier::Encoded => 0,
        CacheTier::Decoded => 1,
        CacheTier::Gpu => 2,
    }
}

fn usize_as_u64(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}

fn write_u64(payload: &mut [u8; 32], at: usize, value: u64) {
    let bytes = value.to_le_bytes();
    if let Some(slot) = payload.get_mut(at..at.saturating_add(8))
        && slot.len() == 8
    {
        slot.copy_from_slice(&bytes);
    }
}

fn write_u32(payload: &mut [u8; 32], at: usize, value: u32) {
    let bytes = value.to_le_bytes();
    if let Some(slot) = payload.get_mut(at..at.saturating_add(4))
        && slot.len() == 4
    {
        slot.copy_from_slice(&bytes);
    }
}
