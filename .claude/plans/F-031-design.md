# F-031, ocelli-cache: budgeted LRU across encoded, decoded and GPU tiers

**Status**: approved
**Epic ref**: E5.1
**Sprint**: S11
**Estimate**: 4w

## Normative source, transcribed

### `docs/hld/17-cache-and-allocation.md`, section 20, in full

```rust
pub trait Budgeted { fn bytes(&self) -> usize; }
pub struct Lru<K, V: Budgeted> {
    budget: usize,
    used: usize,
    /* ... */
}
impl<K, V: Budgeted> Lru<K, V> {
    /// Returns the entries evicted to make room, so the caller can emit events.
    pub fn insert(&mut self, k: K, v: V) -> Vec<(K, V)> { /* ... */ }
}
```

The four bullets, transcribed into table rows so the author's text is quoted
exactly and `scripts/prose_check.py` still passes:

|  |
|----|
| **No allocation in the render loop.** Pre-size everything at viewport creation. A frame that allocates is a frame that can stutter. |
| **Decode into caller-provided buffers** — fn decode(&self, src: &\[u8\], out: &mut \[u8\]), never a Vec returned per frame. |
| **Use bytemuck::cast_slice** for reinterpreting pixel buffers. Hand-written transmutes are unsafe code with no upside here. |
| **Reuse staging buffers by size class.** Texture uploads should draw from a small pool, not allocate. |

### `docs/hld/06-memory-and-cache.md`, section 8, in full

|  |
|----|
| One budget, three tiers, explicit eviction. Encoded bytes are transient and dropped as soon as a frame decodes. Decoded frames sit in an LRU sized by the caller. GPU textures are their own tier with their own pressure signal, because evicting a texture and evicting a frame have very different costs. |
| Volume assembly is progressive by default: the viewport renders a partial volume and refines as slices land, which matters more for perceived speed than any decode optimisation. The absence of a garbage collector is the real memory story — a 300 MB volume load has no pause behaviour to tune around, only a budget to respect. |

### `docs/hld/05-rendering.md`, section 7, the bricking bullet

|  |
|----|
| **Bricking above 256 MiB.** A 512×512×600 sixteen-bit CT series is roughly 300 MB against a guaranteed maximum buffer size of 256 MiB, so chunked upload is the normal path, not an optimisation. |

### `docs/hld/03-architecture-and-crates.md`, section 4, the crate row

| **Crate** | **Responsibility** | **wasm** | **native** |
|----|----|----|----|
| ocelli-cache | Budgeted LRU across encoded, decoded and GPU tiers | yes | yes |

### What section 20 does NOT contain, stated because its absence is load bearing

Section 20 gives no eviction order, no lookup method, no removal method, no
capacity accounting rule, no behaviour for a repeated key and no behaviour for
an entry larger than the whole budget. It gives a trait with one method, a
struct with two named fields and one method signature with a one-line doc
comment. **Everything else in this plan is a decision this plan is making**, and
the section below says so item by item rather than letting the transcription
above imply more coverage than it has.

## What the specification does not cover

1. **What `Lru::insert` does with an entry whose own `bytes()` exceeds the
   whole budget.** The signature returns "the entries evicted to make room",
   and an entry that was never admitted was not evicted to make room. Admitting
   it and letting `used` exceed `budget` makes the budget a number that does not
   describe memory, which is the defect `docs/sprints/CURRENT_SPRINT.md` names
   for this story. See Open question A.
2. **What happens to the previous value when a key is inserted twice.** The old
   value is dropped, and for a GPU texture that is a resource release the caller
   has to know about. It is not an eviction either. See Open question A.
3. **The eviction order and the recency signal.** "LRU" is in the crate's
   responsibility row and in this story's title, and section 20 never says what
   touches an entry. This plan says `get` does and `insert` does, and
   `contains_key` and `pressure` do not, because a telemetry read is not a use.
4. **The map.** `ocelli-cache` carries `#![cfg_attr(not(test), no_std)]`, so
   `std::collections::HashMap` is unavailable and `alloc` offers `BTreeMap`.
   This plan takes `BTreeMap` and a `K: Ord + Clone` bound rather than adding
   `hashbrown`, which would be a new shipped dependency outside HLD section
   15.2's list.
5. **How "three tiers" becomes a type.** Section 8 names three tiers with
   distinct pressure. The three value types are an encoded byte run, a decoded
   frame and a GPU texture, and **none of the three exists in this workspace
   today**: F-032, F-033, F-036 and F-040 are the stories that create them, and
   all four are S12 or S13. See Open question B.
6. **What `Budgeted::bytes` means for a GPU texture.** Section 20 says
   `fn bytes(&self) -> usize` and stops. `ocelli-cache` cannot depend on `wgpu`,
   because `ci/check-device-ownership.sh` keeps every wgpu-touching crate but
   `ocelli-render` out of the device business and because a `no_std` crate
   taking wgpu would break `scripts/no_std_check.py`. So this story owns the
   **rule** and F-040 owns the texture's implementation of it.
7. **Whether `insert` runs in the render loop.** Section 20's own first bullet
   says no allocation in the render loop, and its own `insert` signature returns
   a `Vec`. Those are only compatible if `insert` is not a render-loop call.
   This plan states that it is not: insertion happens on the decode and upload
   paths, and the render loop's cache interaction is `get`, which allocates
   nothing. That reading is recorded here rather than left implicit, because the
   alternative reading makes section 20 contradict itself.

## Approach

### 1. `Budgeted`, section 20's signature unchanged

```rust
/// HLD section 20. The number the budget is kept in.
pub trait Budgeted {
    fn bytes(&self) -> usize;
}
```

Its documentation carries the one rule the specification leaves to the
implementer, and it is the rule `docs/sprints/CURRENT_SPRINT.md` says this
story can quietly break:

> **`bytes` reports what this entry costs the resource it is budgeted against,
> not what it was made from.** A GPU texture reports its allocated footprint,
> not the size of the decoded frame it was uploaded from. A budget kept in
> source sizes is a number that does not describe memory.

### 2. `Lru<K, V>`, section 20's struct and signature, plus what it needs to work

```rust
pub struct Lru<K: Ord + Clone, V: Budgeted> {
    budget: usize,
    used: usize,
    tier: CacheTier,
    entries: BTreeMap<K, Entry<V>>,
    clock: u64,
}

struct Entry<V> {
    value: V,
    bytes: usize,   // V::bytes() at admission, see below
    used_at: u64,
}
```

`budget` and `used` are section 20's two named fields, spelled as it spells
them. The other three are the `/* ... */`.

**`bytes` is captured at admission and never re-read.** `V::bytes()` is a trait
method on a value the cache does not own the definition of, and a `V` whose
`bytes` changes after admission would desynchronise `used` from the sum of the
entries silently. Capturing it makes `used` an invariant the type can assert:
`used == entries.values().map(|e| e.bytes).sum()` holds after every operation,
and a test asserts it after a randomised operation sequence.

Methods, all of which are decisions this plan makes rather than transcriptions:

```rust
impl<K: Ord + Clone, V: Budgeted> Lru<K, V> {
    pub fn new(tier: CacheTier, budget: usize) -> Self;

    /// HLD section 20's `insert`, with the return type widened under D-24.
    pub fn insert(&mut self, k: K, v: V) -> Admission<K, V>;

    /// Reading an entry is a use, so it becomes the most recent.
    pub fn get(&mut self, k: &K) -> Option<&V>;

    /// Explicit release. Section 8: encoded bytes are "dropped as soon as a
    /// frame decodes", which is this and not an eviction.
    pub fn remove(&mut self, k: &K) -> Option<V>;

    /// Telemetry. Not a use, so it does not bump recency.
    pub fn contains_key(&self, k: &K) -> bool;
    pub fn pressure(&self) -> Pressure;
    pub fn len(&self) -> usize;
    pub fn is_empty(&self) -> bool;

    /// Whether an entry of this size could ever be admitted. Lets a caller ask
    /// before building the value, rather than learning from a refusal.
    pub fn would_admit(&self, bytes: usize) -> bool;
}
```

### 2a. `Admission`, which is deviation D-24 and the S11 design round's decision

Section 20 returns `Vec<(K, V)>` and its comment says "the entries evicted to
make room, so the caller can emit events". **Three things can come out of an
insert and only one of them is that.**

```rust
/// What one `insert` displaced, evicted or refused.
///
/// HLD section 20 returns `Vec<(K, V)>`. Deviation D-24 widens it, because the
/// vector's own doc comment says "evicted to make room" and two of the three
/// outcomes below were not.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Admission<K, V> {
    /// Live entries removed to fit the incoming one. Section 20's vector.
    pub evicted: Vec<(K, V)>,
    /// The previous value under the same key. Replaced, not evicted.
    pub displaced: Option<V>,
    /// The incoming entry, handed back because `bytes` exceeds the whole
    /// budget. Never admitted, so never evicted.
    pub refused: Option<(K, V)>,
}
```

**`refused` is non-empty only when `evicted` is empty**, and the type asserts
it: an entry that cannot fit an empty cache is refused before anything is
evicted, so a refusal never costs the caller entries it still had room for.
That ordering is the reason the refusal is checked first in `insert` and it is
what the boundary test pins.

`Admission<K, V>` derives `Default`, so the common path, an insert that fits
with nothing displaced, is one value with three empty fields and no
allocation at all: `Vec::new` does not allocate until something is pushed.

**Eviction order.** Least recently used first, by the `used_at` tick.
`insert` and `get` set `used_at = self.clock` and then increment `clock`.
Eviction repeatedly removes the entry with the smallest `used_at` until
`used + incoming <= budget`. That scan is O(n) per evicted entry, which is
deliberate: `BTreeMap` plus a tick is the smallest thing that is obviously
correct, and HLD section 26's rule is to measure with `bin/ocelli.sh bench`
before optimising. `docs/lld/benchmarks.md` is where a cache subject lands when
one exists to measure.

`clock` is `u64` and monotonic. At one bump per nanosecond it wraps after 584
years, so overflow is not a case this type handles, and that sentence is in the
doc comment rather than an unstated assumption.

### 3. `CacheTier` and `Pressure`, which is how section 8's three tiers become a type

```rust
/// HLD section 8's three tiers. The tier is carried by the cache, not by the
/// value, because a texture and a frame are evicted from different budgets.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CacheTier {
    /// "Encoded bytes are transient and dropped as soon as a frame decodes."
    Encoded,
    /// "Decoded frames sit in an LRU sized by the caller."
    Decoded,
    /// "GPU textures are their own tier with their own pressure signal."
    Gpu,
}

/// One tier's pressure signal, which section 8 gives the GPU tier by name and
/// which the other two need for the same reason.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pressure {
    pub tier: CacheTier,
    pub used: usize,
    pub budget: usize,
}
```

**`CacheTier` has a reader today and that is why it exists.** It is on
`Lru::new`, on `Pressure` and on every eviction record, so an eviction leaving
this crate already says which budget it came from. F-032 surfaces those as JS
events and F-034 surfaces the pressure, and an eviction event that cannot say
whether a texture or a frame went is not one the shell can react to. Without
the field those two stories would add it, and adding it here is one place to
look rather than three.

**There is no `CacheSet` holding all three.** The three tiers hold three
different value types, so one struct over them needs three type parameters with
one instantiation each, which `AGENTS.md`'s structural rules refuse. Three
tiers means three `Lru` values, created by the stories that have a value type to
put in them.

### 4. What this story deliberately does not add

- **No `EncodedBytes`, `DecodedFrame` or `GpuTexture` type.** F-032, F-033,
  F-036 and F-040 create them. A `Budgeted` implementation for a texture that no
  texture type exists to be is a `bytes()` nobody can check.
- **No `wgpu` dependency.** See What the specification does not cover, item 6.
- **No staging-buffer pool.** Section 20's fourth bullet is a texture-upload
  concern and F-040 is the upload path.
- **No `bytemuck::cast_slice` use.** Section 20's third bullet is about
  reinterpreting pixel buffers, and this crate holds no pixel buffer.
- **No brick addressing.** Section 7's bricking bullet is F-036, which depends
  on this story.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no. `ocelli-cache` is `no_std`, holds no boundary
  type and names no pixel type. D3 is untouched
- Render-loop allocation: **none on the render loop's path.** `get`,
  `contains_key`, `pressure` and `len` allocate nothing. `insert` returns an
  `Admission` carrying section 20's own `Vec`, and it is called on the decode
  and upload paths rather than per frame. An insert that evicts nothing pushes
  nothing, and `Vec::new` does not allocate until it is pushed to, so even that
  path is allocation-free. See What the specification does not cover, item 7
- unsafe: none
- Tier A (WebGPU): n/a. This crate holds no GPU code and creates no device. The
  GPU tier is a budget in bytes and a `CacheTier` discriminant, not a wgpu call
- Tier B (WebGL2): n/a, for the same reason. A tier B session's textures are
  budgeted by the same `Lru` with a different number in it
- Tier C (CPU): n/a. There is no rendering or compute feature here to degrade.
  The encoded and decoded tiers are identical on a tier C session and the GPU
  tier simply has no entries

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| `unit` | An empty cache reports `used == 0`, and `budget` is what `new` was given | `crates/ocelli-cache/src/lru.rs` |
| `unit` | Insertion under budget evicts nothing and returns an empty vector | same |
| `unit` | Eviction is least-recently-used: insert A, B, C to fill the budget, `get(A)`, insert D, and **B is what leaves**, not A and not C | same |
| `unit` | `contains_key` and `pressure` do NOT bump recency. Same sequence with `contains_key(A)` in place of `get(A)` evicts A | same |
| `unit` | `remove` returns the value, lowers `used` by exactly that entry's admitted bytes, and returns `None` for an absent key | same |
| `unit` | Re-inserting a live key does not double-count `used`, and the old value arrives as `displaced` rather than inside `evicted` | same |
| `unit` | An entry larger than the whole budget arrives as `refused`, `evicted` is empty, `used` is unchanged and the cache still holds everything it held before | same |
| `unit` | `would_admit` agrees with `insert` for the same size at the budget, one byte under it and one byte over it | same |
| `unit` | An insert that fits with nothing displaced returns `Admission::default()`, so the three-field shape does not make the common path noisy | same |
| `unit` | A zero-budget cache admits nothing and stays empty | same |
| `unit` | An entry of exactly the remaining budget is admitted, and one byte more evicts. The boundary belongs to the admitting side | same |
| `fixture` | **The budget is asserted in bytes against hand-computed entry sizes rather than against what the implementation reports.** A 512 by 512 sixteen-bit frame is 512 × 512 × 2 = 524,288 bytes. A budget of 2,097,152 bytes holds exactly four of them and the fifth evicts the first | `crates/ocelli-cache/tests/budget.rs` |
| `fixture` | **`bytes` is the allocated footprint, not the source footprint.** A test value modelling a GPU texture reports a row-padded allocation, 300 bytes per row padded to 512 over 512 rows is 262,144 bytes, against a decoded source of 153,600. A budget of 262,144 holds exactly one, and an implementation reporting the source size would hold one and report 108,544 bytes free | same |
| `fixture` | HLD section 7's bricking figure: 512 × 512 × 600 sixteen-bit is 314,572,800 bytes against a 256 MiB budget of 268,435,456, so the series does not fit and the cache says so through `would_admit`. This is the arithmetic behind "chunked upload is the normal path" | same |
| `property` | After any randomised sequence of `insert`, `get` and `remove`, `used` equals the sum of the live entries' admitted bytes, and `used <= budget` | `crates/ocelli-cache/tests/invariants.rs` |
| `property` | Every entry returned by `insert` is absent from the cache afterwards, and every entry not returned and not displaced is still present | same |

**Mutation check, HLD 27.3.** Each of these is applied, the named test is
observed red, and the mutation reverted:

1. Evict the **most** recently used instead of the least. The LRU-order test
   goes red, and it goes red on B rather than only on a count.
2. Make `contains_key` bump recency. The telemetry test goes red.
3. Add the incoming entry's bytes to `used` before evicting rather than after.
   The exact-fit boundary test goes red.
4. Drop the displaced value on a repeated key without returning it and without
   subtracting its bytes. The double-count test goes red.
5. Report the source size from the texture-shaped test value's `bytes`. The
   allocated-footprint fixture goes red.
6. Evict to make room **before** checking whether the incoming entry could ever
   fit. The refusal test goes red on `evicted` being non-empty, which is the
   case where a refusal costs the caller entries it still had room for.

The fifth is the one that matters most and it is the one a green suite would
otherwise miss, because both numbers are plausible and only one of them
describes memory.

## Parity surface covered

None. `docs/hld/B-parity-surface.md`'s surface table counts viewport types, tool
classes, blend modes, VOI LUT functions, transfer syntaxes, segmentation
representations, events and adapters, and a cache appears in none of those rows.
The appendix has no `Covered by` column in this repository to update.

## Deviations

**D-24, new, and added to `docs/hld/DEVIATIONS.md` in this same change.**
Section 20's `insert` returns `Vec<(K, V)>` documented as "the entries evicted
to make room". This story returns `Admission<K, V>`, which separates that vector
from a displaced value and a refused entry. The reason is in the row and in
Approach section 2a.

No other deviation is expected. `K: Ord + Clone` and `V: Budgeted` are bounds on
a struct section 20 wrote as `Lru<K, V: Budgeted>` with a `/* ... */` body, and
adding the bound a `BTreeMap` needs is filling that body rather than departing
from it.

## LLD impact

- A new `docs/lld/cache.md`, because no LLD file covers this crate today.
  `docs/lld/README.md` gains its row

## Open questions

None. Both were answered in the S11 consolidated design round.

## Decisions from the S11 design round

**1. `insert` returns `Admission<K, V>`, and D-24 records it.** Section 20's
`Vec<(K, V)>` is documented as "the entries evicted to make room", and two of
the three things that can come out of an insert were not evicted to make room.
F-032 turns these into JS events and they are three different events, so the
distinction exists whatever the return type is, and the only question was
whether F-031 states it or F-032 re-derives it from a key comparison. **The
consequence to hold in review:** a reviewer reading section 20 will see a
signature that does not match, and D-24 is what makes that a declared departure
rather than drift. The `refused` arm is also where the third option, admitting
an oversized entry and letting `used` exceed `budget`, was rejected: that is
exactly the shape `CURRENT_SPRINT.md` names as making the budget a number that
does not describe memory.

**2. The three tiers are `CacheTier` and `Pressure`, not three value types.**
`Budgeted`, `Lru<K, V>`, `CacheTier` and `Pressure` are in scope, and
`EncodedBytes`, `DecodedFrame` and a GPU texture type are not. The three value
types belong to F-032, F-033, F-036 and F-040, all of which are S12 or S13, and
a `Budgeted` implementation for a texture that no texture type exists to be is a
`bytes()` nobody can check. The tiers are proved here by three test
instantiations over three distinct `Budgeted` shapes, one of which is the
row-padded allocation the GPU tier's rule is about.
