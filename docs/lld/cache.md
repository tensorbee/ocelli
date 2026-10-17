# Cache

**F-IDs that contributed:** F-031, F-032, F-033, F-034, F-036
**Last updated:** 2026-10-17

One budget, three tiers, explicit eviction. HLD section 8's sentence, and HLD
section 20's `Budgeted` trait and `Lru<K, V>`, made into `crates/ocelli-cache`.

This crate holds sizes and one decoded value type whose contents it carries
without interpreting. `DecodedFrame` holds an `ocelli-pixel` description and
plane as validated evidence and a sample buffer it never reads as pixels. The
crate links no `wgpu`, and is `#![cfg_attr(not(test), no_std)]` over `alloc`.

## The types

| Type | What it is |
|------|-----------|
| `Budgeted` | HLD section 20's trait. One method, `fn bytes(&self) -> usize` |
| `Lru<K: Ord + Clone, V: Budgeted>` | Section 20's struct, with `budget` and `used` spelled as section 20 spells them |
| `Admission<K, V>` | What one `insert` evicted, displaced or refused. Deviation D-24 |
| `CacheTier` | Section 8's three tiers, `Encoded`, `Decoded` and `Gpu` |
| `Pressure` | One tier's reading: the tier, the bytes used and the budget |
| `DecodedFrame` | F-032. The decoded tier's value type. Its `bytes` is the sample buffer length |
| `ImageCache` | F-032. An `Lru<FrameKey, DecodedFrame>` fixed to the decoded tier, returning events |
| `CacheEvent` | F-032. One variant per `Admission` field: `Evicted`, `Displaced`, `Refused` |
| `RingEvent` | F-032. Section 17.3's 48-byte event envelope, `#[repr(C)]` and `Pod` |

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

**Two value types live in this crate.** F-032 built the decoded tier's
`DecodedFrame` and its `ImageCache`. F-036 built `Brick` and `BrickCache` on
the GPU tier, and `bytes` is the host buffer length. F-033's `Volume`
implements `Budgeted` in `ocelli-volume` and keeps a second decoded-tier
`Lru` there, so a volume cannot spend the frame budget. Encoded bytes and
GPU textures still arrive with their own stories. There is no `wgpu`
dependency here: this crate never sees a texture, only its size.

## The decoded tier and its events, F-032

`ImageCache::insert` converts the `Admission` into `CacheEvents`, displaced
first because `Lru` releases the previous value before eviction begins, then
the evictions in least recently used order. **The three variants are never
collapsed.**

`CacheEvent::encode` writes the event into section 17.3's `RingEvent`: kind
1, 2 and 3 for evicted, displaced and refused, `viewport` as `u32::MAX`
because an eviction belongs to no viewport and zero is a valid viewport id,
and a payload of series, instance, frame and bytes, little-endian, with the
last eight bytes reserved and zero. The numbers are the wire contract until
F-103 decides whether event kinds get a registry, so a test pins each one.

**No ring is built and nothing reaches JavaScript.** The ring in linear
memory is F-101 in S16. This story stops at the tested encoding, which the
S12 design round decided for F-032 and F-034 together, and `CacheEvents`
wraps a `Vec` that allocates on the insert path as `Lru::insert` does. One
insert can produce many events when it evicts many entries, and none are
dropped to fit a fixed capacity.

## The volume cache, F-033

`Volume::bytes` is the length of its one contiguous store. `VolumeCache` is
`Lru<VolumeKey, Volume>` constructed on `CacheTier::Decoded` with the budget
the caller passed, and it exposes `insert`, `get`, `remove` and `pressure`
without adding a second eviction policy. The grouping, the projected spacing
and the refusals live in `docs/lld/volumes.md`. This crate still holds no
volume type and still does not read a voxel.

## Pressure bands and budget controls, F-034

`Pressure::band` places a reading in `Low`, `Elevated` or `Critical`. The
edges are half and nine tenths of that tier's own budget, compared as
`used * 2` against `budget` and `used * 10` against `budget * 9`, in `u64`.
A budget of zero is `Low` when it holds nothing and `Critical` when it holds
bytes. `PressureBand::crossing` returns one `TelemetryEvent::Pressure` when
the band has changed and nothing when it has not. The caller keeps the last
reported band.

`TierCounters` folds `Admission` between readings. An eviction counts, a
refusal counts, and a displacement counts as neither. The counts saturate.
`evicted_bytes` sums `Budgeted::bytes` of the evicted values.

`Lru::set_budget` evicts least recently used first until `used` fits the new
budget. A budget of zero empties the tier, zero-byte entries included.
Growing, and setting the budget the tier already has, evict nothing. The
caller emits the returned evictions first and a `BudgetChanged`
event after them.

Both events encode into the same 48-byte `RingEvent` F-032 defined. Pressure
is kind 4 and a budget change is kind 5, `viewport` is `u32::MAX`, and the
payload is little-endian. A pressure event does not carry `evicted_bytes`.
No ring is built and nothing reaches JavaScript, which remains F-101.

`budget`, `pressure`, `len` and `band` do not move the use tick. A reading
is still not a use.

## Brick addressing and residency, F-036

`BrickGrid::new` is the only place the edge is chosen. Eight-bit samples take
128. Sixteen-bit samples and four-byte floats take 64. The grid is a ceiling
division on each axis. A brick coordinate is a floor division. An edge brick
keeps its true extent, and `Brick::bytes` is the buffer length, so a partial
brick does not report a full cube.

`byte_offset` is section 19's layout, `((z * dims[1] + y) * dims[0] + x) *
bytes_per_voxel`. Each product is a checked `u64`, then narrowed once. A
product that does not fit in `u64`, or an address that does not fit in
`usize`, is `BrickError`, never a wrapped index and never a panic. `index_of`
is the same order at brick scale, x fastest, then y, then z.

`BrickCache` is one `Lru` on `CacheTier::Gpu`. `missing` lists absent keys in
`index_of` order and does not touch recency. `is_resident` is that list being
empty for a grid that has at least one brick. There is no whole-volume path
and no size threshold. `Admission` from `insert` is returned to the caller.
This story emits no eviction event.

`Volume::brick_grid` in `ocelli-volume` converts that crate's `VoxelKind` and
calls `BrickGrid::new`. The volume does not keep a second decomposition.

## Allocation, and the render loop

HLD section 20's first bullet forbids allocation in the render loop, and its own
`insert` signature returns a `Vec`. Those are compatible because **`insert` is
not a render-loop call.** Insertion happens on the decode and upload paths, and
the render loop's cache interaction is `get`.

**`insert` and `set_budget` are the methods on `Lru` that allocate.** A shrink
allocates when it pushes an eviction. A grow and a repeated budget push
nothing. The other methods do not build a `Vec`.

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

**Every other method that existed for that measurement returned zero**, `get`
and `remove` on a hit and on a miss alike, and so did an insert that was
refused. `set_budget` was not in that run. A shrink that evicts pays for the
same two things an evicting insert pays for, the `Vec` push and the clone of
the victim key. A grow pushes nothing and clones nothing.

## Tiers A, B and C

Brick addressing is full on A, B and C. It is host arithmetic, and the crate
still creates no device and holds no shader. Tier B's 256 texture floor holds
a 128 edge, and a longer volume axis is more bricks. Tier C runs the same
code. The GPU tier then holds host bricks, because nothing uploads them yet.
Encoded bytes are not in this crate. A tier C session's decoded tier matches
a tier A session's.

## What is not here

No staging-buffer pool, which is section 20's fourth bullet and belongs to the
upload path. No `bytemuck::cast_slice`, which is section 20's third bullet.
No frustum selection, no prefetch, no indirection texture and no GPU upload.
