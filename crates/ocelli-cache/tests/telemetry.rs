//! F-034, pressure telemetry and budget controls.
//!
//! The wire fixtures are hand-computed from the design plan's payload table.
//! 262,144 is one padded 512-row texture from `docs/lld/cache.md`, and 524,288
//! is two of them. Band edges for a budget of 300 are `used * 2` against the
//! budget and `used * 10` against `budget * 9`: 149 is below half, 150 is
//! half, 269 is below nine tenths and 270 is nine tenths.

use std::collections::BTreeMap;

use ocelli_cache::{
    Admission, Budgeted, CacheTelemetry, CacheTier, KIND_BUDGET_CHANGED, KIND_PRESSURE, Lru,
    Pressure, PressureBand, RingEvent, TelemetryEvent, TierCounters, TierTelemetry,
};
use proptest::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Block(usize);

impl Budgeted for Block {
    fn bytes(&self) -> usize {
        self.0
    }
}

fn pressure(tier: CacheTier, used: usize, budget: usize) -> Pressure {
    Pressure { tier, used, budget }
}

fn tier_telemetry(reading: Pressure, entries: u32, counters: TierCounters) -> TierTelemetry {
    TierTelemetry {
        pressure: reading,
        entries,
        evicted: counters.evicted,
        evicted_bytes: counters.evicted_bytes,
        refused: counters.refused,
    }
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
fn pressure_encoding_round_trips_the_hand_written_bytes() {
    // Gpu is discriminant 2. used 262,144 is 00 00 04 00 little-endian inside
    // the u64. budget 524,288 is 00 00 08 00.
    let reading = pressure(CacheTier::Gpu, 262_144, 524_288);
    let telemetry = tier_telemetry(reading, 1, TierCounters::default());
    let event = TelemetryEvent::Pressure {
        tier: CacheTier::Gpu,
        telemetry,
    }
    .encode(9);
    assert_eq!(event.kind, KIND_PRESSURE);
    assert_eq!(KIND_PRESSURE, 4);
    assert_eq!(event.viewport, u32::MAX);
    assert_eq!(event.seq, 9);
    assert_eq!(core::mem::size_of::<RingEvent>(), 48);
    let raw = bytemuck::bytes_of(&event);
    assert_eq!(le_u32(raw, 0), 4);
    assert_eq!(le_u32(raw, 4), u32::MAX);
    assert_eq!(le_u64(raw, 8), 9);
    let payload = raw.get(16..).unwrap_or(&[]);
    assert_eq!(payload.first().copied(), Some(2));
    assert_eq!(payload.get(1..4), Some(&[0, 0, 0][..]));
    assert_eq!(
        payload.get(4..12),
        Some(&[0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x00, 0x00][..])
    );
    assert_eq!(
        payload.get(12..20),
        Some(&[0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x00, 0x00][..])
    );
    assert_eq!(le_u32(payload, 20), 1);
    assert_eq!(le_u32(payload, 24), 0);
    assert_eq!(le_u32(payload, 28), 0);
    assert_eq!(le_u64(payload, 4), 262_144);
    assert_eq!(le_u64(payload, 12), 524_288);
}

#[test]
fn band_boundaries_are_the_integer_fractions() {
    let band = |used: usize| pressure(CacheTier::Decoded, used, 300).band();
    assert_eq!(band(149), PressureBand::Low);
    assert_eq!(band(150), PressureBand::Elevated);
    assert_eq!(band(269), PressureBand::Elevated);
    assert_eq!(band(270), PressureBand::Critical);
}

#[test]
fn a_zero_budget_is_low_when_empty_and_critical_when_it_holds_bytes() {
    assert_eq!(pressure(CacheTier::Encoded, 0, 0).band(), PressureBand::Low);
    assert_eq!(
        pressure(CacheTier::Encoded, 1, 0).band(),
        PressureBand::Critical
    );
    // 3_000_000_000 * 2 does not fit in a u32. Wrapping both products in u32
    // makes 3_000_000_000 * 10 compare above 3_500_000_000 * 9, so that
    // control flow reports Critical. The u64 comparison is at or above half
    // and below nine tenths, so Elevated. Either wrong band fails this test.
    let wide = pressure(CacheTier::Gpu, 3_000_000_000, 3_500_000_000);
    assert_eq!(wide.band(), PressureBand::Elevated);
}

#[test]
fn three_tiers_report_independent_pressure_and_a_reading_is_not_a_use() {
    let mut encoded: Lru<u8, Block> = Lru::new(CacheTier::Encoded, 100);
    let mut decoded: Lru<u8, Block> = Lru::new(CacheTier::Decoded, 30);
    let gpu: Lru<u8, Block> = Lru::new(CacheTier::Gpu, 300);
    let _ = encoded.insert(1, Block(40));
    let _ = decoded.insert(1, Block(10));
    let _ = decoded.insert(2, Block(10));
    let _ = decoded.insert(3, Block(10));
    for _ in 0..10 {
        let _ = encoded.pressure();
        let _ = decoded.pressure();
        let _ = gpu.pressure();
        let _ = decoded.budget();
        let _ = decoded.len();
    }
    assert_eq!(encoded.pressure().used, 40);
    assert_eq!(encoded.pressure().budget, 100);
    assert_eq!(decoded.pressure().used, 30);
    assert_eq!(gpu.pressure().used, 0);
    assert_eq!(gpu.pressure().budget, 300);
    let snapshot = CacheTelemetry {
        encoded: tier_telemetry(encoded.pressure(), 1, TierCounters::default()),
        decoded: tier_telemetry(decoded.pressure(), 3, TierCounters::default()),
        gpu: tier_telemetry(gpu.pressure(), 0, TierCounters::default()),
        dropped: 0,
    };
    assert_eq!(snapshot.encoded.pressure.tier, CacheTier::Encoded);
    assert_eq!(snapshot.decoded.pressure.tier, CacheTier::Decoded);
    assert_eq!(snapshot.gpu.pressure.tier, CacheTier::Gpu);
    let admission = decoded.insert(4, Block(10));
    assert_eq!(admission.evicted.len(), 1);
    assert_eq!(admission.evicted.first().map(|(key, _)| *key), Some(1));
}

#[test]
fn counters_count_an_eviction_and_a_refusal_and_ignore_a_displacement() {
    let mut cache: Lru<u8, Block> = Lru::new(CacheTier::Decoded, 10);
    let mut counters = TierCounters::default();
    counters.record(&cache.insert(1, Block(6)));
    counters.record(&cache.insert(2, Block(6)));
    counters.record(&cache.insert(3, Block(11)));
    counters.record(&cache.insert(2, Block(4)));
    assert_eq!(counters.evicted, 1);
    assert_eq!(counters.refused, 1);
    assert_eq!(counters.evicted_bytes, 6);
    let taken = counters.take();
    assert_eq!(taken.evicted, 1);
    assert_eq!(counters.evicted, 0);
    assert_eq!(counters.refused, 0);
}

#[test]
fn an_unchanged_band_emits_nothing_and_each_crossing_emits_one() {
    let mut cache: Lru<u16, Block> = Lru::new(CacheTier::Decoded, 300);
    let mut counters = TierCounters::default();
    let mut band = pressure(CacheTier::Decoded, 0, 300).band();
    let mut crossings = 0_u32;
    for index in 1..=270 {
        let admission = cache.insert(u16::try_from(index).unwrap_or(0), Block(1));
        counters.record(&admission);
        let reading = tier_telemetry(
            cache.pressure(),
            u32::try_from(cache.len()).unwrap_or(u32::MAX),
            counters,
        );
        if let Some(event) = band.crossing(reading) {
            crossings = crossings.saturating_add(1);
            band = reading.pressure.band();
            assert_eq!(event.encode(0).kind, KIND_PRESSURE);
            assert_eq!(event.encode(0).viewport, u32::MAX);
        }
    }
    // 0 -> 150 enters Elevated, 150 -> 270 enters Critical. Two crossings up.
    assert_eq!(crossings, 2);
    assert_eq!(band, PressureBand::Critical);
    // Drop back through both boundaries. Keys 1..=270 are one byte each.
    // Removing key 1 leaves 269, which is Elevated. Removing keys 1..=121
    // leaves 149, which is Low. Each boundary is one event and the steps
    // between are none.
    let mut down = 0_u32;
    for index in 1..=121 {
        cache.remove(&u16::try_from(index).unwrap_or(0));
        let reading = tier_telemetry(
            cache.pressure(),
            u32::try_from(cache.len()).unwrap_or(u32::MAX),
            counters,
        );
        if let Some(event) = band.crossing(reading) {
            down = down.saturating_add(1);
            band = reading.pressure.band();
            assert_eq!(event.encode(1).viewport, u32::MAX);
        }
    }
    assert_eq!(down, 2);
    assert_eq!(band, PressureBand::Low);
}

#[test]
fn a_recently_read_zero_byte_entry_still_leaves_at_budget_zero() {
    // The zero-byte entry is inserted first and then read, so it is the most
    // recently used. A drain that stops once `used` fits would evict only the
    // five-byte entry and leave this one live in a tier that is off.
    let mut cache: Lru<u8, Block> = Lru::new(CacheTier::Encoded, 10);
    let _ = cache.insert(1, Block(0));
    let _ = cache.insert(2, Block(5));
    assert_eq!(cache.get(&1), Some(&Block(0)));
    let cleared = cache.set_budget(0);
    assert_eq!(cleared.evicted.len(), 2);
    assert!(cache.is_empty());
}

#[test]
fn set_budget_grows_shrinks_empties_and_ignores_the_same_value() {
    let mut cache: Lru<u8, Block> = Lru::new(CacheTier::Gpu, 30);
    let _ = cache.insert(1, Block(10));
    let _ = cache.insert(2, Block(10));
    let _ = cache.insert(3, Block(10));
    let grown = cache.set_budget(80);
    assert!(grown.evicted.is_empty());
    assert!(grown.displaced.is_none());
    assert!(grown.refused.is_none());
    assert_eq!(cache.budget(), 80);
    assert_eq!(cache.pressure().used, 30);
    let same = cache.set_budget(80);
    assert!(same.evicted.is_empty());
    assert_eq!(cache.len(), 3);
    let shrunk = cache.set_budget(10);
    assert_eq!(
        shrunk
            .evicted
            .iter()
            .map(|(key, _)| *key)
            .collect::<Vec<_>>(),
        vec![1, 2]
    );
    assert_eq!(cache.pressure().used, 10);
    assert!(cache.pressure().used <= cache.budget());
    assert_eq!(cache.len(), 1);
    let mut with_empty: Lru<u8, Block> = Lru::new(CacheTier::Encoded, 10);
    let _ = with_empty.insert(1, Block(0));
    let _ = with_empty.insert(2, Block(5));
    let cleared = with_empty.set_budget(0);
    assert_eq!(cleared.evicted.len(), 2);
    assert!(with_empty.is_empty());
    assert_eq!(with_empty.pressure().used, 0);
    assert_eq!(with_empty.budget(), 0);
}

#[test]
fn a_shrink_leaves_used_at_the_sum_of_what_remains() {
    let mut cache: Lru<u8, Block> = Lru::new(CacheTier::Decoded, 90);
    let mut live: BTreeMap<u8, usize> = BTreeMap::new();
    for (key, bytes) in [(1_u8, 30_usize), (2, 30), (3, 30)] {
        let admission = cache.insert(key, Block(bytes));
        assert!(admission.evicted.is_empty());
        live.insert(key, bytes);
    }
    let admission = cache.set_budget(50);
    for (key, _) in &admission.evicted {
        live.remove(key);
    }
    assert_eq!(cache.pressure().used, live.values().sum::<usize>());
    assert!(cache.pressure().used <= cache.budget());
    assert_eq!(cache.pressure().used, 30);
}

#[test]
fn counters_saturate_instead_of_wrapping() {
    let mut counters = TierCounters {
        evicted: u32::MAX,
        evicted_bytes: u64::MAX,
        refused: u32::MAX,
    };
    let admission: Admission<u8, Block> = Admission {
        evicted: vec![(1, Block(8))],
        displaced: Some(Block(8)),
        refused: Some((2, Block(8))),
    };
    counters.record(&admission);
    assert_eq!(counters.evicted, u32::MAX);
    assert_eq!(counters.evicted_bytes, u64::MAX);
    assert_eq!(counters.refused, u32::MAX);
}

#[test]
fn budget_changed_uses_the_sentinel_viewport_and_the_next_kind() {
    let event = TelemetryEvent::BudgetChanged {
        tier: CacheTier::Decoded,
        budget: 64,
        evicted: 2,
        evicted_bytes: 40,
        dropped: 7,
    };
    let encoded = event.encode(3);
    assert_eq!(encoded.kind, KIND_BUDGET_CHANGED);
    assert_eq!(KIND_BUDGET_CHANGED, 5);
    assert_eq!(encoded.viewport, u32::MAX);
    let raw = bytemuck::bytes_of(&encoded);
    let payload = raw.get(16..).unwrap_or(&[]);
    assert_eq!(payload.first().copied(), Some(1));
    assert_eq!(le_u64(payload, 4), 64);
    assert_eq!(le_u64(payload, 12), 40);
    assert_eq!(le_u32(payload, 20), 2);
    assert_eq!(le_u32(payload, 24), 7);
    assert_eq!(payload.get(28..32), Some(&[0, 0, 0, 0][..]));
}

#[derive(Debug, Clone, Copy)]
enum Op {
    Insert(u8, usize),
    Get(u8),
    Remove(u8),
    SetBudget(usize),
}

fn op() -> impl Strategy<Value = Op> {
    prop_oneof![
        (0u8..6, 0usize..400).prop_map(|(key, bytes)| Op::Insert(key, bytes)),
        (0u8..6).prop_map(Op::Get),
        (0u8..6).prop_map(Op::Remove),
        (0usize..500).prop_map(Op::SetBudget),
    ]
}

proptest! {
    #[test]
    fn eviction_bytes_match_what_the_tier_stopped_holding(
        budget in 0usize..800,
        ops in prop::collection::vec(op(), 0..40),
    ) {
        let mut cache: Lru<u8, Block> = Lru::new(CacheTier::Decoded, budget);
        let mut live: BTreeMap<u8, usize> = BTreeMap::new();
        let mut counters = TierCounters::default();
        let mut expected = 0_u64;
        let mut reported = 0_u64;
        for step in ops {
            match step {
                Op::Insert(key, bytes) => {
                    let admission = cache.insert(key, Block(bytes));
                    counters.record(&admission);
                    for (evicted, _) in &admission.evicted {
                        expected = expected.saturating_add(u64::try_from(live.remove(evicted).unwrap_or(0)).unwrap_or(u64::MAX));
                    }
                    if admission.refused.is_none() {
                        live.insert(key, bytes);
                    }
                }
                Op::Get(key) => {
                    let _ = cache.get(&key);
                }
                Op::Remove(key) => {
                    if cache.remove(&key).is_some() {
                        live.remove(&key);
                    }
                }
                Op::SetBudget(next) => {
                    let admission = cache.set_budget(next);
                    counters.record(&admission);
                    for (evicted, _) in &admission.evicted {
                        expected = expected.saturating_add(u64::try_from(live.remove(evicted).unwrap_or(0)).unwrap_or(u64::MAX));
                    }
                }
            }
            let taken = counters.take();
            reported = reported.saturating_add(taken.evicted_bytes);
            let reading = cache.pressure();
            prop_assert!(reading.used <= reading.budget);
            prop_assert_eq!(reading.used, live.values().sum::<usize>());
        }
        prop_assert_eq!(reported, expected);
    }
}
