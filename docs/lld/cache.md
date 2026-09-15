# Cache

**F-IDs that contributed:** F-031
**Last updated:** 2026-09-14

One budget, three tiers, explicit eviction. HLD section 8's sentence, and HLD
section 20's `Budgeted` trait and `Lru<K, V>`, made into `crates/ocelli-cache`.

This crate holds sizes and never contents it understands. It names no pixel
type, links no `wgpu`, and is `#![cfg_attr(not(test), no_std)]` over `alloc`.

## The types

| Type | What it is |
|------|-----------|
| `Budgeted` | HLD section 20's trait. One method, `fn bytes(&self) -> usize` |
| `Lru<K: Ord + Clone, V: Budgeted>` | Section 20's struct, with `budget` and `used` spelled as section 20 spells them |
| `Admission<K, V>` | What one `insert` evicted, displaced or refused. Deviation D-24 |
| `CacheTier` | Section 8's three tiers, `Encoded`, `Decoded` and `Gpu` |
| `Pressure` | One tier's reading: the tier, the bytes used and the budget |

## What `bytes` means, which is the promise the budget keeps

**`bytes` reports what an entry costs the resource it is budgeted against, not
what it was made from.** A GPU texture reports its allocated footprint,
including whatever row padding the upload path added, and not the size of the
decoded frame it was uploaded from. A budget kept in source sizes is a number
that does not describe memory, and it fails silently, because both numbers are
plausible and only one of them is the device's.

`crates/ocelli-cache/tests/budget.rs` holds that rule down with a fixture whose
two numbers differ: a 512-row texture with 300 bytes of samples per row, padded
to 512 bytes per row, is 262,144 bytes allocated against 153,600 bytes of
source. A cache whose budget is exactly one such texture reports no bytes free
after admitting one, where a `bytes` reporting the source size would report
108,544 bytes that the device does not have.

**`V::bytes()` is read once, at admission, and never again.** It is a trait
method on a value this crate does not own the definition of, so a `V` whose
`bytes` changed afterwards would desynchronise `used` from the entries with
nothing to notice. Reading it once makes `used` the sum of the live entries'
admitted bytes, which `tests/invariants.rs` asserts after every operation of a
random sequence.

Three unit tests hold that down with a value that reports a different size after
admission, one for each place bytes are given back: `remove`, an eviction, and
the replacement of a live key. They exist because every other value in the suite
is immutable, so no number of random cases can tell a cache that kept the
admitted number from one that re-reads it.

## What `insert` returns, and why it is not section 20's vector

Section 20 returns `Vec<(K, V)>`, documented as "the entries evicted to make
room, so the caller can emit events". Three things can come out of an insert
and one of them is that, which is deviation **D-24**.

| Field | When it is populated |
|-------|----------------------|
| `evicted` | Live entries removed to fit the incoming one. Section 20's vector, least recently used first |
| `displaced` | The previous value under the same key. Replaced, not evicted |
| `refused` | The incoming entry, which the budget cannot hold. Never admitted, so never evicted. `would_admit` is the condition |

**A refusal is decided before anything is evicted**, so a refusal never costs
the caller entries it still had room for. `refused` is therefore non-empty only
when `evicted` is empty, and the unit and property suites both pin it.

**A replacement releases before it evicts**, for the same reason in the other
direction: the previous value's bytes leave `used` before the eviction loop
runs, so the incoming value never competes with the value it replaces. At a full
budget that is the difference between evicting nothing and evicting a stranger,
and on the GPU tier a stranger is a texture discarded that did not need to go.
`replacing_a_live_key_at_a_full_budget_evicts_nothing` is the test. Deferring
that release until after the eviction loop leaves the accounting correct and
evicts a stranger, and it fails that test and nothing else.

The alternative, admitting an oversized entry and letting `used` exceed
`budget`, was rejected in the S11 design round. A budget that reports more bytes
in use than it permits is the defect this story exists to avoid.

**A budget of zero refuses everything, an entry of zero bytes included.** A tier
the caller gave no bytes is a tier that is off, and `0 <= 0` would otherwise let
one fill while reporting itself empty. A tier that does have a budget admits an
entry of no bytes, because the budget bounds bytes and such an entry costs none.
`would_admit` answers exactly the question `insert` asks, so the two cannot
disagree, and it is the only place the condition is written.

**The budget bounds bytes and not entries**, which is the other half of that
sentence and is worth stating because the first half invites the wrong reading.
An entry of zero bytes never makes `used` grow, so it never triggers an eviction
at any budget. `entries_of_no_bytes_never_evict_anything` fills a budget and then
adds eight zero-byte entries, and nothing leaves. **Section 20 specifies no
entry count and this crate adds none**, so that is a byte budget doing what a
byte budget does rather than a bound nobody wrote. What it depends on is
`Budgeted::bytes` reporting what an entry costs. An implementation returning
zero for something that occupies memory defeats the budget completely, which is
the rule at the top of this file seen from the other side.

`Admission` has a hand-written `Default` rather than a derived one, because
`#[derive(Default)]` bounds every type parameter and would make
`Admission::default()` unavailable to `insert`, which knows nothing about `K` or
`V`. This is the trap deviation D-08 records for the marker spaces.

## Eviction order

Least recently used first, by a monotonic `u64` tick, and **the returned
`evicted` vector is in that order too**, which F-032 needs because it is the
order eviction events reach JS in. `the_evicted_vector_is_least_recently_used_first`
is the one **deterministic** test that evicts more than one entry, and it is
there because an order no deterministic test evicts twice cannot be got wrong
anywhere a reader can see it. The property test reaches multi-eviction
routinely and asserts nothing about order.

`insert` and `get` set an entry's tick. **`contains_key`, `pressure`, `len`, `is_empty` and `would_admit`
do not**, because a telemetry reading is not a use, and a cache whose pressure
signal changed what leaves next would be a cache nobody could measure.

Finding the smallest tick is a scan of a `BTreeMap`. That is deliberate rather
than overlooked: a map and a counter is the smallest thing that is obviously
correct, and HLD section 26's rule is to measure with `bin/ocelli.sh bench`
before optimising. `benchmarks.md` is where a cache subject lands when there is
a caller to measure one against.

`alloc::collections::BTreeMap` is the map because the crate is `no_std` and
`alloc` has no `HashMap`. The `K: Ord + Clone` bound is what that map needs.

## Three tiers, three caches

`CacheTier` is carried by the cache rather than by the value, because the cost
of an eviction belongs to the budget an entry was admitted against. `Lru::new`
takes it and `Pressure` reports it. **`Admission` does not carry it**, because a
caller emitting an eviction event already knows which cache it called `insert`
on, and a field repeating that would be a second place for it to be wrong.

**There is no type holding all three tiers.** They hold three different value
types, so one struct over them would need three type parameters with one
instantiation each. Three tiers is three `Lru` values, created by the stories
that have a value type to put in them.

**None of the three value types exists yet.** Encoded bytes, decoded frames and
GPU textures arrive with F-032, F-033, F-036 and F-040, and each implements
`Budgeted` for itself. That is also why there is no `wgpu` dependency here: this
crate never sees a texture, only its size.

## Allocation, and the render loop

HLD section 20's first bullet forbids allocation in the render loop, and its own
`insert` signature returns a `Vec`. Those are compatible because **`insert` is
not a render-loop call.** Insertion happens on the decode and upload paths, and
the render loop's cache interaction is `get`.

**`insert` is the only method on `Lru` that allocates.** That is the rule, and
it is stated as one because the alternative is a list of the other eight
maintained in three files, which is how a true sentence becomes a stale one.

**`insert` can allocate**, and the doc comment on it says so, because the
sentence a later story needs is the one that stops it calling this per frame.
The returned `Admission` is not the reason: an insert that evicts nothing pushes
nothing, and `Vec::new` does not allocate until something is pushed into it. The
map is. It takes a node at a time rather than one per entry, so the cost is
irregular rather than absent.

Measured with a counting global allocator during the F-031 review, over forty
consecutive non-evicting inserts into one cache: thirty-four allocated nothing,
five allocated once, and one allocated twice, at a node split. **That split is
not a constant.** Ascending and descending key orders both reproduce it, and the
scattered order `(i * 37) % 101` measured thirty-five, four and one. The scattered order `(i * 3) % 41`
measured thirty-six, three and one, so the number to carry away
is that an insert's allocation count is neither always zero nor always one.

An insert that evicts allocated once more, for the `Vec` push, and **an
eviction also clones the victim's key**, which costs another allocation when the
key owns memory. That was measured at one with a `u32` key and two with a
`String` key. The clone is what ends the scan's borrow of the map before the
removal, and it is why `K: Clone` is on the type.

**Every other method on `Lru` measured zero**, `get` and `remove` on a hit and
on a miss alike, and so did an insert that was refused.

## Tiers A, B and C

Not applicable, all three. There is no rendering or compute feature here to
degrade: the crate holds no GPU code and creates no device, and the GPU tier is
a budget in bytes and a `CacheTier` discriminant. A tier C session's encoded and
decoded tiers are identical to a tier A session's, and its GPU tier simply has
no entries.

## What is not here

No staging-buffer pool, which is section 20's fourth bullet and belongs to the
upload path. No brick addressing, which is section 7's bricking bullet and is
F-036. No `bytemuck::cast_slice`, which is section 20's third bullet and is
about reinterpreting pixel buffers this crate does not hold.
