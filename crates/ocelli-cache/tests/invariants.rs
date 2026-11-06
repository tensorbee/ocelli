//! The two invariants HLD section 20's budget rests on, over random operation
//! sequences.
//!
//! Section 20 names two fields, `budget` and `used`, and says nothing about the
//! relationship between them. The relationship is the whole point: a `used`
//! that drifts from the sum of what the cache holds is a budget that does not
//! describe memory, and it drifts silently, one operation at a time.
//!
//! **The model here is driven by what `insert` reports**, not by a second copy
//! of the eviction policy. That is deliberate. A model that re-decided which
//! entry leaves would only ever agree with itself, where this one fails
//! whenever the returned `Admission` and the cache's own accounting disagree.

use std::collections::BTreeMap;

use ocelli_cache::{Budgeted, CacheTier, Lru};
use proptest::prelude::*;

/// A value whose only property is its size, because these two properties are
/// about accounting rather than about what is being cached.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Block {
    bytes: usize,
}

impl Budgeted for Block {
    fn bytes(&self) -> usize {
        self.bytes
    }
}

#[derive(Debug, Clone, Copy)]
enum Op {
    Insert(u8, usize),
    Get(u8),
    Remove(u8),
    /// Release every key between the two bounds, inclusive.
    RemoveRange(u8, u8),
    /// Move the budget. The model tracks the caller's last word.
    SetBudget(u16),
}

fn op() -> impl Strategy<Value = Op> {
    prop_oneof![
        (0u8..8, 0usize..1200).prop_map(|(key, bytes)| Op::Insert(key, bytes)),
        (0u8..8).prop_map(Op::Get),
        (0u8..8).prop_map(Op::Remove),
        // Sorted so the range is never empty. An empty range is covered by
        // the random keys anyway, since a range over absent keys removes
        // nothing.
        (0u8..8, 0u8..8).prop_map(|(lo, hi)| Op::RemoveRange(lo.min(hi), lo.max(hi))),
        (0u16..2000).prop_map(Op::SetBudget),
    ]
}

proptest! {
    /// `used` is the sum of the live entries' admitted bytes, and it never
    /// exceeds the budget. A range removal reports each key at the bytes the
    /// model held for it, exactly once, so a double release goes red here
    /// against the accounting as well.
    #[test]
    fn used_is_the_sum_of_the_live_entries_and_never_exceeds_the_budget(
        budget in 0usize..2000,
        ops in prop::collection::vec(op(), 0..60),
    ) {
        let mut cache: Lru<u8, Block> = Lru::new(CacheTier::Decoded, budget);
        let mut live: BTreeMap<u8, usize> = BTreeMap::new();
        // The budget is the caller's last word, because `set_budget` moves it.
        let mut current_budget = budget;

        for step in ops {
            match step {
                Op::Insert(key, bytes) => {
                    let admission = cache.insert(key, Block { bytes });
                    for (evicted_key, _) in &admission.evicted {
                        live.remove(evicted_key);
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
                Op::RemoveRange(lo, hi) => {
                    for (removed_key, block) in cache.remove_range(lo..=hi) {
                        prop_assert_eq!(live.get(&removed_key).copied(), Some(block.bytes));
                        live.remove(&removed_key);
                    }
                }
                Op::SetBudget(budget) => {
                    current_budget = usize::from(budget);
                    let admission = cache.set_budget(current_budget);
                    for (evicted_key, _) in &admission.evicted {
                        live.remove(evicted_key);
                    }
                }
            }

            let pressure = cache.pressure();
            prop_assert_eq!(pressure.used, live.values().sum::<usize>());
            prop_assert_eq!(pressure.budget, current_budget);
            prop_assert!(pressure.used <= current_budget);
            prop_assert_eq!(cache.len(), live.len());
            prop_assert_eq!(cache.is_empty(), live.is_empty());
        }
    }

    /// Everything `insert` hands back has left the cache, and everything it
    /// did not hand back is still in it.
    #[test]
    fn returned_entries_leave_and_the_rest_stay(
        budget in 0usize..2000,
        ops in prop::collection::vec(op(), 0..60),
    ) {
        let mut cache: Lru<u8, Block> = Lru::new(CacheTier::Decoded, budget);
        let mut live: BTreeMap<u8, usize> = BTreeMap::new();

        for step in ops {
            match step {
                Op::Insert(key, bytes) => {
                    let admission = cache.insert(key, Block { bytes });

                    // A refusal costs the caller nothing it was holding, and
                    // it displaces nothing either, so the equivalence below
                    // is stated for an admission rather than for an insert.
                    if admission.refused.is_some() {
                        prop_assert!(admission.evicted.is_empty());
                        prop_assert!(admission.displaced.is_none());
                        prop_assert_eq!(admission.refused, Some((key, Block { bytes })));
                    } else {
                        // A displaced value is the previous value under this
                        // key, so it arrives exactly when the key was live.
                        prop_assert_eq!(admission.displaced.is_some(), live.contains_key(&key));
                    }

                    // An eviction is never the incoming key, and never a key
                    // the cache did not hold.
                    for (evicted_key, evicted) in &admission.evicted {
                        prop_assert_ne!(*evicted_key, key);
                        prop_assert_eq!(live.get(evicted_key), Some(&evicted.bytes));
                        prop_assert!(!cache.contains_key(evicted_key));
                        live.remove(evicted_key);
                    }

                    if admission.refused.is_none() {
                        live.insert(key, bytes);
                    }
                    prop_assert_eq!(cache.contains_key(&key), live.contains_key(&key));
                }
                Op::Get(key) => {
                    let found = cache.get(&key).copied();
                    prop_assert_eq!(found.map(|block| block.bytes), live.get(&key).copied());
                }
                Op::Remove(key) => {
                    let removed = cache.remove(&key);
                    prop_assert_eq!(removed.map(|block| block.bytes), live.get(&key).copied());
                    live.remove(&key);
                    prop_assert!(!cache.contains_key(&key));
                }
                Op::RemoveRange(lo, hi) => {
                    for (removed_key, removed) in cache.remove_range(lo..=hi) {
                        prop_assert_eq!(live.get(&removed_key).copied(), Some(removed.bytes));
                        live.remove(&removed_key);
                        prop_assert!(!cache.contains_key(&removed_key));
                    }
                }
                Op::SetBudget(budget) => {
                    // A control neither displaces nor refuses, so the two
                    // fields stay empty.
                    let admission = cache.set_budget(usize::from(budget));
                    prop_assert!(admission.displaced.is_none());
                    prop_assert!(admission.refused.is_none());
                    for (evicted_key, evicted) in &admission.evicted {
                        prop_assert_eq!(live.get(evicted_key).copied(), Some(evicted.bytes));
                        prop_assert!(!cache.contains_key(evicted_key));
                        live.remove(evicted_key);
                    }
                }
            }

            // Nothing that was not returned has gone missing.
            for key in live.keys() {
                prop_assert!(cache.contains_key(key));
            }
        }
    }
}
