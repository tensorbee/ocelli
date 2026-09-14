//! HLD section 20's `Budgeted` and `Lru<K, V>`.
//!
//! Section 20 gives a trait with one method, a struct with two named fields and
//! one method signature. Everything else here is a decision recorded in
//! `.claude/plans/F-031-design.md`: the eviction order, what counts as a use,
//! the map, and what happens to an entry that can never fit.
//!
//! **`insert` is not a render-loop call.** Section 20's first bullet forbids
//! allocation in the render loop and its own `insert` signature returns a
//! `Vec`, which are only compatible if insertion happens on the decode and
//! upload paths. The render loop's cache interaction is `get`.
//!
//! **`insert` is the only method here that allocates**, which is a rule rather
//! than a list, because a list has to be maintained in every file it is copied
//! into and a rule does not. `docs/lld/cache.md` carries the measurement.

use alloc::collections::BTreeMap;
use alloc::vec::Vec;

use crate::tier::{CacheTier, Pressure};

/// HLD section 20. The number the budget is kept in.
///
/// **`bytes` reports what this entry costs the resource it is budgeted
/// against, not what it was made from.** A GPU texture reports its allocated
/// footprint, including any row padding the upload path added, not the size of
/// the decoded frame it was uploaded from. A budget kept in source sizes is a
/// number that does not describe memory.
pub trait Budgeted {
    /// The bytes this entry costs the tier holding it.
    fn bytes(&self) -> usize;
}

/// What one `insert` displaced, evicted or refused.
///
/// HLD section 20 returns `Vec<(K, V)>`. Deviation D-24 widens it, because the
/// vector's own doc comment says "evicted to make room" and two of the three
/// outcomes below were not.
///
/// The common path, an insert that fits with nothing displaced, is this value
/// with three empty fields, and **the value itself allocates nothing**, because
/// `Vec::new` does not allocate until something is pushed into it. That is a
/// claim about the return value and not about `insert`, which can allocate. See
/// `Lru::insert`.
#[derive(Debug, PartialEq, Eq)]
#[must_use = "an eviction, a replacement and a refusal are three different \
              events, and only this value tells them apart"]
pub struct Admission<K, V> {
    /// Live entries removed to fit the incoming one. Section 20's vector,
    /// least recently used first.
    pub evicted: Vec<(K, V)>,
    /// The previous value under the same key. Replaced, not evicted.
    pub displaced: Option<V>,
    /// The incoming entry, handed back because the budget cannot hold it.
    /// `Lru::would_admit` is the exact condition: `bytes` above the whole
    /// budget, or a budget of zero, which is a tier that is off. Never
    /// admitted, so never evicted, and populated only when `evicted` is empty.
    pub refused: Option<(K, V)>,
}

// Written out rather than derived. `#[derive(Default)]` on a generic struct
// bounds every parameter, so it would produce `K: Default, V: Default` and
// make `Admission::default()` unavailable to `insert`, which knows nothing
// about either. This is the trap deviation D-08 records for the marker spaces.
impl<K, V> Default for Admission<K, V> {
    fn default() -> Self {
        Self {
            evicted: Vec::new(),
            displaced: None,
            refused: None,
        }
    }
}

/// One live entry, with the two numbers the cache keeps about it.
struct Entry<V> {
    value: V,
    /// `V::bytes()` read once, at admission. See `Lru`.
    bytes: usize,
    /// The tick of the most recent use.
    used_at: u64,
}

/// HLD section 20's budgeted LRU, over one of section 8's three tiers.
///
/// `budget` and `used` are section 20's two named fields. The rest is its
/// `/* ... */`.
///
/// **`V::bytes()` is read once, at admission, and never again.** It is a trait
/// method on a value this type does not own the definition of, so a `V` whose
/// `bytes` changed after admission would desynchronise `used` from the sum of
/// the entries with nothing to notice it. Reading it once makes
/// `used == entries.values().map(|e| e.bytes).sum()` an invariant that holds
/// after every operation, which `tests/invariants.rs` asserts over random
/// operation sequences.
///
/// **Eviction is least recently used first**, by a monotonic tick that `insert`
/// and `get` set and that nothing else touches. Finding the smallest tick is a
/// scan, which is deliberate: a `BTreeMap` and a counter is the smallest thing
/// that is obviously correct, and HLD section 26's rule is to measure with
/// `bin/ocelli.sh bench` before optimising. `docs/lld/benchmarks.md` is where a
/// cache subject lands when there is one to measure.
pub struct Lru<K: Ord + Clone, V: Budgeted> {
    budget: usize,
    used: usize,
    tier: CacheTier,
    entries: BTreeMap<K, Entry<V>>,
    /// The use counter. Monotonic, and never reset. At one bump per nanosecond
    /// a `u64` wraps after roughly 584 years, so a wrap is not a case this type
    /// handles. It wraps rather than overflows because a panic poisons the wasm
    /// instance, which is HLD section 23.
    clock: u64,
}

impl<K: Ord + Clone, V: Budgeted> Lru<K, V> {
    /// An empty cache over one tier, holding at most `budget` bytes.
    pub fn new(tier: CacheTier, budget: usize) -> Self {
        Self {
            budget,
            used: 0,
            tier,
            entries: BTreeMap::new(),
            clock: 0,
        }
    }

    /// HLD section 20's `insert`, with the return type widened under D-24.
    ///
    /// The three outcomes are checked in the order they can rule each other
    /// out. An entry that cannot fit an empty cache is refused **before**
    /// anything is evicted, so a refusal never costs the caller entries it
    /// still had room for. A repeated key releases its previous value before
    /// eviction begins, so the incoming value never competes with the value it
    /// replaces.
    ///
    /// **A refused insert changes nothing.** A key that was already live keeps
    /// its previous value and its place in the eviction order, and the caller
    /// gets the entry it offered back in `refused`.
    ///
    /// **This can allocate, which is why it is not a render-loop call.** The
    /// map takes a node at a time rather than one per entry, so a given insert
    /// may allocate nothing, once, or more than once when a node fills. An
    /// eviction also pushes onto `evicted`. `docs/lld/cache.md` has the
    /// measurement.
    pub fn insert(&mut self, key: K, value: V) -> Admission<K, V> {
        let mut admission = Admission::default();
        let incoming = value.bytes();

        if !self.would_admit(incoming) {
            admission.refused = Some((key, value));
            return admission;
        }

        if let Some(previous) = self.entries.remove(&key) {
            self.release(previous.bytes);
            admission.displaced = Some(previous.value);
        }

        while incoming > self.free() {
            // `else` is unreachable: `incoming` is at most `budget`, so an empty
            // map leaves `free()` at `budget` and ends the loop before this is
            // reached. It breaks rather than continues, because a loop that
            // asks the same question of the same map would not terminate.
            let Some((victim, entry)) = self.evict_least_recently_used() else {
                break;
            };
            admission.evicted.push((victim, entry.value));
        }

        let used_at = self.tick();
        self.entries.insert(
            key,
            Entry {
                value,
                bytes: incoming,
                used_at,
            },
        );
        self.used += incoming;
        admission
    }

    /// Reading an entry is a use, so it becomes the most recently used.
    pub fn get(&mut self, key: &K) -> Option<&V> {
        let used_at = self.tick();
        let entry = self.entries.get_mut(key)?;
        entry.used_at = used_at;
        Some(&entry.value)
    }

    /// Explicit release. HLD section 8: encoded bytes are "dropped as soon as
    /// a frame decodes", which is this and not an eviction.
    pub fn remove(&mut self, key: &K) -> Option<V> {
        let entry = self.entries.remove(key)?;
        self.release(entry.bytes);
        Some(entry.value)
    }

    /// Whether the key is live. Telemetry, not a use, so it does not change
    /// what leaves next.
    #[must_use]
    pub fn contains_key(&self, key: &K) -> bool {
        self.entries.contains_key(key)
    }

    /// This tier's pressure signal. Telemetry, not a use.
    #[must_use]
    pub fn pressure(&self) -> Pressure {
        Pressure {
            tier: self.tier,
            used: self.used,
            budget: self.budget,
        }
    }

    /// The number of live entries.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the cache holds nothing.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Whether an entry of this size could ever be admitted, so a caller can
    /// ask before building the value rather than learn from a refusal.
    ///
    /// This is a question about the budget and not about what is in the cache,
    /// because everything in the cache can be evicted to make room. It is also
    /// exactly the question `insert` asks before it refuses.
    ///
    /// **A budget of zero admits nothing**, an entry of zero bytes included. A
    /// tier the caller gave no bytes is a tier that is off, and a cache that
    /// took entries into it would hold values against no budget at all.
    #[must_use]
    pub fn would_admit(&self, bytes: usize) -> bool {
        self.budget > 0 && bytes <= self.budget
    }

    /// The bytes the budget still has. Never underflows, because `used` is the
    /// sum of the live entries and every admission keeps it at or below
    /// `budget`.
    fn free(&self) -> usize {
        self.budget.saturating_sub(self.used)
    }

    /// Take the bytes of an entry that has just left off `used`.
    ///
    /// `used` is the sum of the live entries' admitted bytes, so it is always
    /// at least the bytes of an entry that was live a moment ago, and the
    /// saturation cannot fire. The debug assertion is what says so in a build
    /// that can check it. Saturating is the release behaviour rather than
    /// wrapping because an under-report is recoverable, where a `usize` wrap
    /// would make the tier look permanently full.
    ///
    /// The shipped artefact carries the saturation and not the assertion, and
    /// that is the point: HLD section 15.2's `[profile.release]` leaves debug
    /// assertions off, so nothing here can panic in a wasm module. The same
    /// reasoning is recorded for glam's assertions in `ocelli-core`.
    fn release(&mut self, bytes: usize) {
        debug_assert!(self.used >= bytes, "used is the sum of the live entries");
        self.used = self.used.saturating_sub(bytes);
    }

    /// Remove the entry with the smallest use tick and release its bytes.
    ///
    /// Two lookups, a scan for the smallest tick and a `remove_entry` for the
    /// key it found. The scan holds the map, so the key it picks is cloned to
    /// end that borrow before the removal, which is why `K: Clone` is on the
    /// type. A key that owns memory pays one allocation per eviction for it.
    /// Ticks are unique, so the scan has no tie to break. `None`
    /// means there was nothing to evict, which is an empty map, or a second
    /// lookup that did not find what the first did. The caller breaks on it
    /// rather than asking the same question of the same map again, which is
    /// what a loop over a failing removal would do.
    fn evict_least_recently_used(&mut self) -> Option<(K, Entry<V>)> {
        let victim = self
            .entries
            .iter()
            .min_by_key(|(_, entry)| entry.used_at)
            .map(|(key, _)| key.clone())?;
        let (key, entry) = self.entries.remove_entry(&victim)?;
        self.release(entry.bytes);
        Some((key, entry))
    }

    /// The next use tick. A `get` that misses consumes one without changing an
    /// entry, which the ordering does not care about.
    fn tick(&mut self) -> u64 {
        let now = self.clock;
        self.clock = self.clock.wrapping_add(1);
        now
    }
}

#[cfg(test)]
mod tests {
    use core::cell::Cell;

    use super::{Admission, Budgeted, Lru};
    use crate::tier::CacheTier;

    /// A value that is nothing but its size, because these tests are about the
    /// budget's mechanics. The byte semantics of a real entry are the subject
    /// of `tests/budget.rs`.
    #[derive(Debug, PartialEq, Eq)]
    struct Block(usize);

    impl Budgeted for Block {
        fn bytes(&self) -> usize {
            self.0
        }
    }

    /// A value that reports a different size after admission, which is the case
    /// `Entry::bytes` exists to survive. Nothing in this crate can stop a `V`
    /// doing this, so the cache reads `bytes` once and keeps the number, and
    /// the three tests below check each site that gives bytes back.
    struct Shifting(Cell<usize>);

    impl Budgeted for Shifting {
        fn bytes(&self) -> usize {
            self.0.get()
        }
    }

    impl Shifting {
        fn new(bytes: usize) -> Self {
            Self(Cell::new(bytes))
        }
    }

    /// Change what a live entry reports. Returns whether the entry was there,
    /// so a caller asserts it rather than passing over a silent no-op. Reading
    /// the entry is a use, which the callers below account for.
    #[must_use]
    fn restate(cache: &mut Lru<&'static str, Shifting>, key: &'static str, bytes: usize) -> bool {
        let Some(entry) = cache.get(&key) else {
            return false;
        };
        entry.0.set(bytes);
        true
    }

    /// Three 100-byte blocks fill this exactly.
    const BUDGET: usize = 300;

    fn filled() -> Lru<&'static str, Block> {
        let mut cache = Lru::new(CacheTier::Decoded, BUDGET);
        for key in ["a", "b", "c"] {
            let admission = cache.insert(key, Block(100));
            assert!(admission.evicted.is_empty());
        }
        cache
    }

    #[test]
    fn an_empty_cache_reports_no_use_and_the_budget_it_was_given() {
        let cache: Lru<&str, Block> = Lru::new(CacheTier::Encoded, 4096);
        let pressure = cache.pressure();

        assert_eq!(pressure.used, 0);
        assert_eq!(pressure.budget, 4096);
        assert_eq!(pressure.tier, CacheTier::Encoded);
        assert_eq!(cache.len(), 0);
        assert!(cache.is_empty());
    }

    #[test]
    fn an_insert_under_budget_evicts_nothing() {
        let mut cache: Lru<&str, Block> = Lru::new(CacheTier::Decoded, BUDGET);
        let admission = cache.insert("a", Block(100));

        assert!(admission.evicted.is_empty());
        assert!(admission.displaced.is_none());
        assert!(admission.refused.is_none());
        assert_eq!(cache.pressure().used, 100);
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn eviction_takes_the_least_recently_used_entry() {
        let mut cache = filled();

        // "a" was inserted first and is used again here, which leaves "b" as
        // the least recently used entry.
        assert_eq!(cache.get(&"a"), Some(&Block(100)));

        let admission = cache.insert("d", Block(100));

        assert_eq!(admission.evicted, [("b", Block(100))]);
        assert!(admission.displaced.is_none());
        assert!(cache.contains_key(&"a"));
        assert!(!cache.contains_key(&"b"));
        assert!(cache.contains_key(&"c"));
        assert!(cache.contains_key(&"d"));
        assert_eq!(cache.pressure().used, BUDGET);
    }

    #[test]
    fn the_evicted_vector_is_least_recently_used_first() {
        let mut cache = filled();

        // Using "b" puts it last, so the use order is "a", then "c", then "b".
        // That is deliberately not the key order, so a cache evicting by key
        // rather than by recency would return a different vector.
        assert_eq!(cache.get(&"b"), Some(&Block(100)));

        // 200 bytes needs two entries to leave, which is the only deterministic
        // case in the suite where the vector has an order to get wrong.
        let admission = cache.insert("d", Block(200));

        assert_eq!(
            admission.evicted,
            [("a", Block(100)), ("c", Block(100))],
            "section 20's vector is least recently used first"
        );
        assert!(cache.contains_key(&"b"));
        assert!(cache.contains_key(&"d"));
        assert_eq!(cache.len(), 2);
        assert_eq!(cache.pressure().used, BUDGET);
    }

    #[test]
    fn a_refused_insert_does_not_change_the_eviction_order() {
        let mut cache = filled();

        // "a" is live and least recently used. Offering it a value the budget
        // can never hold is refused, and a refusal is not a use.
        let admission = cache.insert("a", Block(BUDGET + 1));
        assert_eq!(admission.refused, Some(("a", Block(BUDGET + 1))));
        assert!(cache.contains_key(&"a"));

        let admission = cache.insert("d", Block(100));

        assert_eq!(admission.evicted, [("a", Block(100))]);
        assert!(cache.contains_key(&"b"));
    }

    #[test]
    fn remove_releases_the_bytes_admitted_and_not_the_bytes_reported_now() {
        let mut cache: Lru<&str, Shifting> = Lru::new(CacheTier::Decoded, BUDGET);
        let admission = cache.insert("a", Shifting::new(100));
        assert!(admission.refused.is_none());
        let admission = cache.insert("b", Shifting::new(100));
        assert!(admission.refused.is_none());
        assert_eq!(cache.pressure().used, 200);

        // "a" now says 250 where it was admitted at 100. The budget is kept in
        // what it was admitted at, so nothing moves until it leaves. The
        // restatement is asserted, because a fixture that quietly failed to
        // change anything would make this test pass against either behaviour.
        assert!(restate(&mut cache, "a", 250));
        assert_eq!(cache.get(&"a").map(Budgeted::bytes), Some(250));
        assert_eq!(cache.pressure().used, 200);

        assert!(cache.remove(&"a").is_some());
        assert_eq!(cache.pressure().used, 100);
    }

    #[test]
    fn eviction_releases_the_bytes_admitted_and_not_the_bytes_reported_now() {
        let mut cache: Lru<&str, Shifting> = Lru::new(CacheTier::Decoded, BUDGET);
        for key in ["a", "b", "c"] {
            let admission = cache.insert(key, Shifting::new(100));
            assert!(admission.evicted.is_empty());
        }

        // "a" now says 10, and reading it made it the most recent, so the reads
        // after it put the use order back to "a", "b", "c".
        assert!(restate(&mut cache, "a", 10));
        assert_eq!(cache.get(&"a").map(Budgeted::bytes), Some(10));
        assert_eq!(cache.get(&"b").map(Budgeted::bytes), Some(100));
        assert_eq!(cache.get(&"c").map(Budgeted::bytes), Some(100));

        let admission = cache.insert("d", Shifting::new(100));

        // One entry leaving frees the 100 it was admitted at. Releasing 10
        // would take a second entry as well.
        assert_eq!(admission.evicted.len(), 1);
        assert_eq!(cache.len(), 3);
        assert_eq!(cache.pressure().used, BUDGET);
    }

    #[test]
    fn replacing_a_key_releases_the_bytes_it_was_admitted_at() {
        let mut cache: Lru<&str, Shifting> = Lru::new(CacheTier::Decoded, BUDGET);
        let admission = cache.insert("a", Shifting::new(100));
        assert!(admission.refused.is_none());
        let admission = cache.insert("b", Shifting::new(100));
        assert!(admission.refused.is_none());

        assert!(restate(&mut cache, "a", 250));
        assert_eq!(cache.get(&"a").map(Budgeted::bytes), Some(250));

        let admission = cache.insert("a", Shifting::new(100));

        assert!(admission.displaced.is_some());
        assert!(admission.evicted.is_empty());
        assert_eq!(cache.pressure().used, 200);
        assert_eq!(cache.len(), 2);
    }

    #[test]
    fn contains_key_is_not_a_use() {
        let mut cache = filled();

        // The same sequence as the test above, with one reading in place of
        // the use. A reading does not save "a".
        assert!(cache.contains_key(&"a"));

        let admission = cache.insert("d", Block(100));

        assert_eq!(admission.evicted, [("a", Block(100))]);
        assert!(!cache.contains_key(&"a"));
        assert!(cache.contains_key(&"b"));
    }

    #[test]
    fn pressure_is_not_a_use() {
        let mut cache = filled();

        // One reading, of the other telemetry method, so a failure says which
        // of the two bumped recency.
        assert_eq!(cache.pressure().used, BUDGET);

        let admission = cache.insert("d", Block(100));

        assert_eq!(admission.evicted, [("a", Block(100))]);
        assert!(!cache.contains_key(&"a"));
        assert!(cache.contains_key(&"b"));
    }

    #[test]
    fn remove_returns_the_value_and_lowers_used_by_that_entry() {
        let mut cache = filled();

        assert_eq!(cache.remove(&"b"), Some(Block(100)));
        assert_eq!(cache.pressure().used, 200);
        assert_eq!(cache.len(), 2);

        assert_eq!(cache.remove(&"b"), None);
        assert_eq!(cache.remove(&"z"), None);
        assert_eq!(cache.pressure().used, 200);
    }

    #[test]
    fn re_inserting_a_live_key_displaces_and_does_not_double_count() {
        let mut cache: Lru<&str, Block> = Lru::new(CacheTier::Decoded, BUDGET);
        let admission = cache.insert("a", Block(100));
        assert!(admission.displaced.is_none());

        let admission = cache.insert("a", Block(200));

        assert_eq!(admission.displaced, Some(Block(100)));
        assert!(admission.evicted.is_empty());
        assert!(admission.refused.is_none());
        assert_eq!(cache.pressure().used, 200);
        assert_eq!(cache.len(), 1);
        assert_eq!(cache.get(&"a"), Some(&Block(200)));
    }

    #[test]
    fn replacing_a_live_key_at_a_full_budget_evicts_nothing() {
        let mut cache = filled();
        assert_eq!(cache.pressure().used, BUDGET);

        // The previous value's bytes leave `used` before eviction begins, so
        // the incoming value never competes with the value it replaces. At a
        // full budget that is the difference between evicting nothing and
        // evicting a stranger, which on the GPU tier is a texture discarded
        // that did not need to go.
        let admission = cache.insert("a", Block(100));

        assert_eq!(admission.displaced, Some(Block(100)));
        assert!(admission.evicted.is_empty());
        assert!(admission.refused.is_none());
        assert_eq!(cache.len(), 3);
        assert_eq!(cache.pressure().used, BUDGET);
        assert!(cache.contains_key(&"b"));
        assert!(cache.contains_key(&"c"));
    }

    #[test]
    fn an_entry_larger_than_the_whole_budget_is_refused_before_anything_leaves() {
        let mut cache = filled();

        let admission = cache.insert("d", Block(BUDGET + 1));

        assert_eq!(admission.refused, Some(("d", Block(BUDGET + 1))));
        assert!(admission.evicted.is_empty());
        assert!(admission.displaced.is_none());
        assert_eq!(cache.pressure().used, BUDGET);
        assert_eq!(cache.len(), 3);
        assert!(cache.contains_key(&"a"));
        assert!(cache.contains_key(&"b"));
        assert!(cache.contains_key(&"c"));
    }

    #[test]
    fn would_admit_agrees_with_insert_at_the_budget_boundary() {
        // One byte under the budget, the budget itself and one byte over it,
        // with the answer written out per case rather than recomputed from the
        // rule under test.
        for (bytes, admitted) in [(BUDGET - 1, true), (BUDGET, true), (BUDGET + 1, false)] {
            let mut cache: Lru<&str, Block> = Lru::new(CacheTier::Decoded, BUDGET);
            let admission = cache.insert("a", Block(bytes));

            assert_eq!(cache.would_admit(bytes), admitted);
            assert_eq!(admission.refused.is_none(), admitted);
            assert_eq!(cache.contains_key(&"a"), admitted);
        }
    }

    #[test]
    fn an_insert_that_fits_with_nothing_displaced_is_the_default_admission() {
        let mut cache: Lru<&str, Block> = Lru::new(CacheTier::Decoded, BUDGET);

        // `Block` has no `Default`, so this also pins the hand-written `Default`
        // impl, which carries no bound on either parameter.
        assert_eq!(cache.insert("a", Block(100)), Admission::default());
    }

    #[test]
    fn a_zero_budget_cache_admits_nothing_and_stays_empty() {
        let mut cache: Lru<&str, Block> = Lru::new(CacheTier::Gpu, 0);

        let admission = cache.insert("a", Block(1));

        assert_eq!(admission.refused, Some(("a", Block(1))));
        assert!(admission.evicted.is_empty());
        assert!(cache.is_empty());
        assert_eq!(cache.pressure().used, 0);
        assert!(!cache.would_admit(1));

        // Nothing includes an entry of no bytes. A tier given no budget is off,
        // and `0 <= 0` would otherwise admit an unbounded number of them into a
        // cache reporting itself empty.
        let admission = cache.insert("b", Block(0));

        assert_eq!(admission.refused, Some(("b", Block(0))));
        assert!(admission.evicted.is_empty());
        assert!(admission.displaced.is_none());
        assert!(cache.is_empty());
        assert_eq!(cache.len(), 0);
        assert!(!cache.would_admit(0));
    }

    #[test]
    fn entries_of_no_bytes_never_evict_anything() {
        // The budget bounds bytes, not entries. An entry costing none never
        // makes `used` grow, so it never makes room, at a full budget or any
        // other. Stated as a test because the alternative is stating it in
        // prose and finding out later.
        let mut cache = filled();
        assert_eq!(cache.pressure().used, BUDGET);

        for key in ["w", "x", "y", "z", "p", "q", "r", "s"] {
            let admission = cache.insert(key, Block(0));
            assert!(admission.evicted.is_empty());
            assert!(admission.refused.is_none());
        }

        assert_eq!(cache.len(), 11);
        assert_eq!(cache.pressure().used, BUDGET);
        assert!(cache.contains_key(&"a"));
    }

    #[test]
    fn a_tier_with_a_budget_admits_an_entry_of_no_bytes() {
        // The counterpart to the zero-budget case above, stated so the rule is
        // not read as "zero bytes is always refused". The budget bounds bytes,
        // and an entry of none costs none.
        let mut cache: Lru<&str, Block> = Lru::new(CacheTier::Encoded, BUDGET);

        assert!(cache.would_admit(0));
        let admission = cache.insert("a", Block(0));

        assert!(admission.refused.is_none());
        assert_eq!(cache.len(), 1);
        assert_eq!(cache.pressure().used, 0);

        // A cache holding an entry is not empty, whatever that entry costs.
        // `used == 0` and `is_empty()` are different questions here, and this
        // is the only state in which they part company.
        assert!(!cache.is_empty());
    }

    #[test]
    fn an_entry_of_exactly_the_remaining_budget_is_admitted_and_one_more_evicts() {
        let mut exact: Lru<&str, Block> = Lru::new(CacheTier::Decoded, BUDGET);
        let admission = exact.insert("a", Block(100));
        assert!(admission.evicted.is_empty());

        // 200 is exactly what is left of 300, so the boundary belongs to the
        // admitting side and "a" stays.
        let admission = exact.insert("b", Block(200));
        assert!(admission.evicted.is_empty());
        assert_eq!(exact.pressure().used, BUDGET);
        assert!(exact.contains_key(&"a"));

        let mut over: Lru<&str, Block> = Lru::new(CacheTier::Decoded, BUDGET);
        let admission = over.insert("a", Block(100));
        assert!(admission.evicted.is_empty());

        // One byte more than is left, so "a" goes.
        let admission = over.insert("b", Block(201));
        assert_eq!(admission.evicted, [("a", Block(100))]);
        assert_eq!(over.pressure().used, 201);
        assert!(!over.contains_key(&"a"));
    }
}
