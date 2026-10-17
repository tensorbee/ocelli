# F-034, Memory-pressure telemetry and JS-visible budget controls

**Status**: approved
**Epic ref**: E5.4
**Sprint**: S12
**Estimate**: 2w

## Normative source, transcribed

### `docs/hld/06-memory-and-cache.md`, section 8, in full

> One budget, three tiers, explicit eviction. Encoded bytes are transient and dropped as soon as a frame decodes. Decoded frames sit in an LRU sized by the caller. GPU textures are their own tier with their own pressure signal, because evicting a texture and evicting a frame have very different costs.
>
> Volume assembly is progressive by default: the viewport renders a partial volume and refines as slices land, which matters more for perceived speed than any decode optimisation. The absence of a garbage collector is the real memory story, a 300 MB volume load has no pause behaviour to tune around, only a budget to respect.

**"their own pressure signal" is this story.** F-031 built one `Pressure` per `Lru` and this story makes the three of them visible to the shell and lets the shell move the budgets.

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

- **No allocation in the render loop.** Pre-size everything at viewport creation. A frame that allocates is a frame that can stutter.
- **Decode into caller-provided buffers**, `fn decode(&self, src: &[u8], out: &mut [u8])`, never a Vec returned per frame.
- **Use bytemuck::cast_slice** for reinterpreting pixel buffers. Hand-written transmutes are unsafe code with no upside here.
- **Reuse staging buffers by size class.** Texture uploads should draw from a small pool, not allocate.

The section 20 listing is quoted with its semicolons inside the code fence. Prose in this plan carries none.

### `docs/hld/14-the-boundary-in-code.md`, section 17.3, the event ring in full

```rust
// ocelli-wasm/src/ring.rs -- single producer (Rust), single consumer (JS)
#[repr(C)]
pub struct RingHeader {
    pub head: u32, // producer writes
    pub tail: u32, // consumer writes
    pub cap: u32, // power of two
    pub dropped: u32, // overflow count; nonzero means JS is not draining
}
#[repr(C)]
pub struct Event {
    pub kind: u32,
    pub viewport: u32,
    pub seq: u64,
    pub payload: [u8; 32],
}
// 48-byte stride. JS reads with a DataView at header_size + i * 48.
```

> *Fixed stride so the JavaScript side is arithmetic, not deserialisation. Drain once per animation frame, surface `dropped` in telemetry rather than swallowing it.*

### `docs/hld/14-the-boundary-in-code.md`, section 17.4, state readback

```rust
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ViewportState {
    pub camera: [f32; 16],
    pub voi_center: f32,
    pub voi_width: f32,
    pub slice_index: u32,
    pub num_slices: u32,
    pub flags: u32, // bit 0 invert, bit 1 loading, bit 2 error
    pub _pad: [u32; 3],
}
```

> *Copied into a JS-owned buffer on request. Never returned as a JS object graph, that is one allocation and one conversion per field.*

Section 17.4 is the pattern a pressure readback follows if an open question chooses it. It is copied on request and it is not an event.

### `docs/hld/04-boundary-and-data-path.md`, sections 5 and 6

Section 5, in full.

> This is the design decision everything else follows from. Get it wrong and the port is slower than what it replaces while being harder to debug. Three channels, and nothing else crosses.
>
> ### 5.1 Control, typed commands, downward
>
> Small, infrequent, one call per user intent: set camera, set VOI, activate tool, load series. **Never one call per pointer move.** Pointer streams are normalised in TypeScript and batched into a single packed command buffer per animation frame.
>
> ### 5.2 Bulk, raw bytes into linear memory, downward
>
> The core allocates and returns a pointer and length. JavaScript builds a typed-array view immediately before writing and discards it after. Views are never cached across a call that might allocate, because any WebAssembly memory growth relocates the backing buffer and detaches every outstanding view. This is the sharpest edge in the whole design and section 17.2 gives the exact pattern.
>
> ### 5.3 Events, a ring buffer, upward
>
> A fixed-size ring in linear memory, drained once per frame rather than invoking a JavaScript callback per event. This removes a boundary crossing from the hot path and gives coalescing for free: a burst of camera changes during a drag collapses to one delivered event. Viewport state reads back as a small C-layout struct copy, not a JavaScript object graph.

Section 6 names the data path and the LUT chain as one uniform block. It gives no cache sentence and this plan takes none from it.

### `docs/hld/23-performance-rules.md`, section 26, in full

- No allocation in the render loop.
- One `queue.submit()` per frame.
- Prefer a uniform update to a texture update. Window/level is thirty-two bytes, not a re-upload.
- Batch pointer events into one command buffer per animation frame. Never cross the boundary per event.
- Measure with the benchmark harness before optimising anything. The intuitions that work in JavaScript do not transfer.

Applied here, the fourth bullet forbids a boundary crossing per pressure reading and the first forbids allocating to take one. A telemetry path that allocates per frame or crosses per eviction breaks both on its first use.

### `docs/hld/11-decision-log.md`, decisions D3 and D4

| # | Decision | Rejected | Why |
|---|----------|----------|-----|
| D3 | Pixels never cross the boundary | Frames as JS objects | The main architectural gain |
| D4 | Events polled from a ring, per frame | A JS callback per event | Removes the hot-path crossing, free coalescing |

The decision table is transcribed in substance from `CLAUDE.md` and `docs/hld/11-decision-log.md`. D4 governs how a pressure change reaches JS and D3 governs what it never carries.

### What F-031 built, which this story consumes

From `crates/ocelli-cache/src/tier.rs`, character for character in the type shapes.

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CacheTier {
    Encoded,
    Decoded,
    Gpu,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pressure {
    pub tier: CacheTier,
    pub used: usize,
    pub budget: usize,
}
```

From `crates/ocelli-cache/src/lru.rs`.

```rust
pub fn new(tier: CacheTier, budget: usize) -> Self
pub fn insert(&mut self, key: K, value: V) -> Admission<K, V>
pub fn pressure(&self) -> Pressure
pub fn would_admit(&self, bytes: usize) -> bool
```

`Pressure` is one tier, one reading, taken without touching eviction order. `docs/lld/cache.md` records that `contains_key`, `pressure`, `len`, `is_empty` and `would_admit` do not set the use tick, because a telemetry reading is not a use. This story preserves that rule and tests it.

Deviation D-24 widened section 20's return to `Admission<K, V> { evicted, displaced, refused }`. F-034 counts evictions from `evicted` alone, so a replacement counted as an eviction would inflate the telemetry D-24 exists to keep apart. F-032's plan makes the same distinction for the same reason.

### What the tree says, which the story title does not

**There is no event ring.** `crates/ocelli-wasm/src/` holds `lib.rs` and `panic.rs` and nothing else. `ocelli-wasm` depends on `ocelli-core` alone and exposes four free functions. `scripts/unsafe_allowlist_check.py` already names `crates/ocelli-wasm/src/ring.rs` as permitted, so the file was anticipated and never written.

**The boundary is F-101, E16.2 in S16**, "wasm boundary: command channel, event ring buffer, state readback", four sprints away and depending on F-100's API design. So "JS-visible" in this story's title names a channel that does not exist and is owned by another story. See Open question A, which is shared with F-032 and is not re-decided here.

**There is no type holding all three tiers.** `docs/lld/cache.md` records the reason. Three tiers hold three different value types, so one struct over them would need three type parameters with one instantiation each. Three tiers is three `Lru` values, and this story does not change that.

**`Lru` has no way to move a budget.** `new` takes one and no method changes it. A JS-visible budget control therefore needs a new method on `Lru`, which is a change to F-031's type and not only to this story's telemetry. See Open question C.

`docs/sprints/allocation.json` carries no architecture hook for F-034. Its `notes` field is empty, its dependency is F-031 alone, and its layer is Rust.

## What the specification does not cover

1. **What crosses, and when.** Section 8 gives the GPU tier "their own pressure signal" and section 17.3 says to "surface `dropped` in telemetry". Neither says whether pressure is an event on change, a periodic sample, or a struct the shell copies on request under section 17.4. This plan decides it in the Approach and Open question B records the choice as a decision the HLD did not make.
2. **What a pressure event's payload contains.** Section 17.3 fixes the envelope, `kind`, `viewport`, `seq` and 32 payload bytes, and says nothing about what a pressure reading puts in those 32 bytes.
3. **What `viewport` means for a pressure event.** A tier is not a viewport. Section 17.3's envelope has the field regardless. F-032 chose `u32::MAX` for a cache event on the same reasoning and this plan follows it, so the two stories agree on one sentinel rather than inventing two.
4. **Which numbers `kind` takes.** There is no event-kind registry. `ci/error-codes.json` registers error codes under HLD section 23 and no equivalent requirement exists for events. F-032 Open question B asks the same question and this plan shares its answer rather than opening a second one.
5. **How a budget moves at runtime.** Section 8 says decoded frames "sit in an LRU sized by the caller" and section 20's listing has no resize. Nothing says whether shrinking a budget evicts immediately, refuses the shrink, or lets `used` sit above `budget` until the next insert. This is the largest gap in the story and Open question C carries it.
6. **What happens to live entries when a budget shrinks below `used`.** A direct consequence of item 5 and the place a wrong answer becomes visible, because evicting to fit a smaller budget destroys decoded frames a viewport may be showing.
7. **How often is often enough.** No section gives a sampling cadence, a change threshold, or a rule for whether an unchanged reading is worth a ring slot. A per-insert event and a per-frame sample are both consistent with the text and they cost very differently.
8. **Whether the encoded tier is reported at all.** Section 8 calls encoded bytes "transient" and gives the pressure signal to the GPU tier by name. Reporting all three is this plan's reading, from CURRENT_SPRINT.md's "F-034 reports pressure per tier, which is what `CacheTier` and `Pressure` are for". The HLD alone does not settle it.
9. **What a budget control returns.** A shrink that evicts has entries to report, in the same shape `Admission` already uses. Whether the shell learns of them through the ring, through a return value, or not at all is unspecified.
10. **Whether `seq` is per ring or per event kind.** Section 17.3 gives one `seq` field and one ring. F-032 records the same gap and both plans take one ring-wide counter.

## Approach

### 1. `Telemetry`, three readings and the counters that make them mean something

A new module `crates/ocelli-cache/src/telemetry.rs`. It holds no cache and owns no budget. It reads three `Pressure` values the caller already has and folds in the counters a bare reading cannot express.

```rust
/// One tier's pressure, plus what happened since the last reading.
/// The caller owns the three `Lru` values and passes their `pressure()`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TierTelemetry {
    pub pressure: Pressure,
    /// Live entries in the tier at reading time.
    pub entries: u32,
    /// Entries evicted to make room since the previous reading.
    pub evicted: u32,
    /// Bytes evicted to make room since the previous reading.
    pub evicted_bytes: u64,
    /// Entries the budget could not hold since the previous reading.
    pub refused: u32,
}

/// A snapshot across section 8's three tiers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CacheTelemetry {
    pub encoded: TierTelemetry,
    pub decoded: TierTelemetry,
    pub gpu: TierTelemetry,
    /// Ring events the producer dropped because JS did not drain.
    /// Section 17.3: surface it rather than swallowing it.
    pub dropped: u32,
}
```

The counters come from `Admission`, which the caller already receives from every `insert`. `evicted` counts `admission.evicted.len()` and `evicted_bytes` sums the admitted bytes of those entries. `refused` counts a populated `admission.refused`. `displaced` is counted nowhere, because a replacement is neither an eviction nor a refusal and D-24 separated the three so this story would not have to guess.

**The snapshot is a value the caller assembles, not a struct that reaches into three caches.** A `CacheTelemetry::from_pressures` constructor would need the three `Pressure` values and the three counters, which is eight arguments naming one idea, so the caller builds the struct literally and a `Telemetry` accumulator holds the running counters between readings.

```rust
/// Accumulates `Admission` outcomes between readings. Allocation free.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TierCounters {
    pub evicted: u32,
    pub evicted_bytes: u64,
    pub refused: u32,
}

impl TierCounters {
    /// Fold one insert's outcome in. `displaced` is deliberately ignored.
    pub fn record<K, V: Budgeted>(&mut self, admission: &Admission<K, V>);
    /// Take the accumulated values and reset to zero.
    pub fn take(&mut self) -> TierCounters;
}
```

Counters are `u32` and saturate rather than wrap. A wrapped eviction count reads as a quiet cache, which is the telemetry equivalent of the quietly wrong pixel. Saturation keeps a broken drain visible as a large number instead of a small one. `dropped` comes from `RingHeader.dropped` at drain time and is the one counter this crate does not own, so it is a field the boundary layer fills in and defaults to zero natively.

### 2. The section 17.3 encoding, shared with F-032 and not re-invented

F-032 defines `RingEvent`, section 17.3's `Event` field for field at 48 bytes, and `CacheEvent::encode`. This story adds variants beside it in the same style rather than a second envelope.

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TelemetryEvent {
    /// A tier crossed a pressure threshold, or a reading was requested.
    Pressure { tier: CacheTier, telemetry: TierTelemetry },
    /// A budget moved, at the shell's request or the caller's.
    BudgetChanged { tier: CacheTier, budget: usize, evicted: u32 },
}

impl TelemetryEvent {
    /// Encode into section 17.3's envelope.
    pub fn encode(&self, seq: u64) -> RingEvent;
}
```

The payload layout, little-endian throughout because wasm32 is little-endian and section 17.1 says the wire format is.

| Bytes | `Pressure` payload | `BudgetChanged` payload |
|-------|--------------------|-------------------------|
| 0 | tier discriminant, `u8` | tier discriminant, `u8` |
| 1 to 3 | reserved, zeroed | reserved, zeroed |
| 4 to 11 | `used` as `u64` | `budget` as `u64` |
| 12 to 19 | `budget` as `u64` | bytes evicted by the change as `u64` |
| 20 to 23 | `entries` as `u32` | entries evicted by the change as `u32` |
| 24 to 27 | `evicted` as `u32` | `dropped` as `u32` |
| 28 to 31 | `refused` as `u32` | reserved, zeroed |

`evicted_bytes` does not fit alongside the rest and is not carried per event. It is available in `CacheTelemetry` for a readback and it is the number a threshold decision least needs, because `used` against `budget` already says how full the tier is. Dropping it from the wire and keeping it in the struct is stated here so a reader does not have to diff the two layouts to find it.

`viewport` is `u32::MAX`, F-032's sentinel, asserted rather than assumed. `kind` values are constants in `ocelli-cache` beside F-032's three, in one range with one comment. See Open question B, shared.

### 3. When an event is emitted, coalescing instead of a stream

**Pressure is emitted on a threshold crossing and on request, never per insert.** This is the section 5.3 argument applied to telemetry. A burst of inserts during a series load would otherwise fill the ring with near-identical readings and push out the eviction events F-032 exists to deliver, and section 17.3's `dropped` counter would then be reporting this story's noise.

Thresholds are fractions of the tier's own budget, because the three budgets differ by orders of magnitude and one absolute byte threshold cannot mean the same thing in all three.

```rust
/// The pressure bands a tier reports crossings between.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PressureBand {
    /// Below half the budget.
    Low,
    /// At or above half and below nine tenths.
    Elevated,
    /// At or above nine tenths.
    Critical,
}

impl Pressure {
    /// Which band this reading sits in. Integer arithmetic only.
    pub fn band(&self) -> PressureBand;
}
```

The bands are computed in integer arithmetic, `used * 2` against `budget` and `used * 10` against `budget * 9`, with the multiplication done in `u64` so a large budget cannot overflow a `usize` on a 32-bit target. No floating point enters a comparison a test has to reproduce exactly. A budget of zero is `Critical` when `used` is above zero and `Low` when it is not, and the zero case is named because `Lru::would_admit` already treats a zero budget as a tier that is off rather than a tier that is empty.

An event fires when the band changes, in either direction. Falling out of `Critical` matters as much as entering it, because the shell's response to pressure is to stop prefetching and it needs to know when it may start again. Within a band, inserts emit nothing. The caller holds the last reported band per tier in three `PressureBand` values beside the counters, which is pre-sized state and not an allocation.

A reading is also emitted on request, through the same encoding, so the shell can ask rather than wait. That request arrives as a typed command under section 5.1 and leaves as a ring event under section 5.3, which keeps one upward channel instead of adding a return path. Open question B covers the readback alternative.

### 4. Moving a budget, the control half of the story

```rust
impl<K: Ord + Clone, V: Budgeted> Lru<K, V> {
    /// Move this tier's budget. Returns what the move displaced.
    /// Evicts least recently used first until `used` fits, so the
    /// returned value is section 20's vector in Admission's shape.
    pub fn set_budget(&mut self, budget: usize) -> Admission<K, V>;
    /// The budget this tier was given, as `pressure` already reports it.
    pub fn budget(&self) -> usize;
}
```

`set_budget` is the only new mutating method and it reuses `Admission` rather than inventing a second report type. Growing a budget evicts nothing and returns three empty fields. Shrinking evicts least recently used first until `used` fits the new budget, using the same scan `insert` uses, so there is one eviction order in the crate and not two. Setting a budget of zero evicts everything, which follows from `would_admit` already reading zero as off. Setting the budget the tier already has is a no-op that returns an empty `Admission`.

**What `set_budget` does not do is refuse.** A shrink that cannot be satisfied without eviction still evicts, because the caller asked for the memory back and a control that reports success while holding the bytes is the budget equivalent of `used` exceeding `budget`. Open question C records the alternative and why this plan does not take it.

The caller turns a non-empty `set_budget` result into events. Entries evicted by a shrink are `Evicted` under F-032's encoding, because that is what they are, and a `BudgetChanged` event follows so the shell can tell a control-driven eviction from a pressure-driven one without comparing timestamps. Order matters and is specified here. The `Evicted` events come first, in least recently used order, and `BudgetChanged` comes last carrying the count, so a consumer that stops mid-burst still holds a consistent set of removals and a summary that has not yet claimed them.

### 5. What this story does NOT do, and it is the title's second half

**It writes no ring and it reaches no JavaScript.** See Open question A, shared with F-032. The encodings above are `#[repr(C)]` structs with hand-checked layout fixtures, and the thing that carries them across is F-101 in S16. "JS-visible" is delivered as a tested encoding and a control surface in Rust, and the title's last step is recorded as not delivered rather than quietly implied.

It also builds no dashboard, no log sink and no metrics exporter. Telemetry here means the numbers cross the boundary in a form the shell can render. What the shell does with them is F-103's event model and the TypeScript side's business.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no. A `TelemetryEvent` carries a tier, three byte counts and three counters. It never carries a key, a frame or a sample, so it is narrower than F-032's event, which at least names what left. Decision D3
- Render-loop allocation: none. `TierCounters::record` and `Pressure::band` allocate nothing and `CacheTelemetry` is a `Copy` struct of integers. Taking a reading calls `pressure` and `len`, which `docs/lld/cache.md` records as allocation free and tick free. Encoding writes into a caller-owned `RingEvent` value rather than returning a `Vec`, and a burst of evictions is the caller's existing `Admission` vector rather than a new one. No buffer is sized here because none is allocated here, and that is stated rather than left as an omission
- unsafe: none. The 48-byte layout is asserted with `core::mem::size_of` and field reads on a constructed value, not with a transmute. Section 20's third bullet points at `bytemuck::cast_slice` for pixel buffers and this story reinterpret-casts no buffer
- Tier A (WebGPU): n/a. Telemetry reads budgets and counters and holds no GPU resource. The GPU tier on a tier A session reports real texture bytes, because that is what `Budgeted::bytes` was defined to mean in F-031
- Tier B (WebGL2): n/a, same reason. The GPU tier's budget is a number in this crate whatever backend allocated the textures behind it
- Tier C (CPU): n/a. The cache behaves identically on every tier. A tier C session has no GPU textures, so its GPU tier reports `used` of zero against whatever budget it was given, and that reading is honest rather than unavailable. Reporting the tier as absent would need a fourth state the type does not have, and an empty tier is already expressible

All three tier rows are present because the template requires them and because an omitted row and a deliberate "not applicable" read identically later. Here "not applicable" is the true answer three times and it is written three times.

## Tests

This story does no pixel arithmetic and no geometry arithmetic. It moves byte counts and compares ratios of integers. HLD 27.2 R3 makes `fixture` mandatory for pixel and geometry arithmetic, and there is none here to cite a DICOM section against, so no row below invents one. The fixtures that do exist are for the wire layout and the band boundaries, which are the two places this story can be quietly wrong, and they are hand-computed in the same spirit.

| Category | What it proves | Where |
|----------|----------------|-------|
| `fixture` | `RingEvent` is 48 bytes with `kind` at 0, `viewport` at 4, `seq` at 8 and `payload` at 16, asserted as raw bytes against section 17.3. Shared with F-032 and not re-asserted under a second name if F-032 lands first | `crates/ocelli-cache/tests/telemetry.rs` |
| `fixture` | A `Pressure` encoding round-trips field by field, little-endian. Tier `Gpu`, `used` 262,144 and `budget` 524,288 are written out byte by byte in the test, where 262,144 is one padded 512-row texture from `docs/lld/cache.md` and 524,288 is two of them | same |
| `fixture` | Band boundaries at exact fractions. Budget 300 gives `Low` at 149, `Elevated` at 150 and at 269, and `Critical` at 270, computed by hand from `used * 2` against `budget` and `used * 10` against `budget * 9`. One byte either side of each boundary, because a threshold that is wrong by one byte is wrong at exactly the load where it matters | same |
| `fixture` | A zero budget reads `Low` when empty and `Critical` when it holds bytes, and a budget above `usize` range on a 32-bit target does not overflow the band comparison | same |
| `unit` | Three `Lru` values report three independent `Pressure` values, and reading one tier's pressure does not change another tier's eviction order. Per-tier reporting is the story's first sentence | same |
| `unit` | `TierCounters::record` counts an eviction, a refusal and a displacement as one, one and zero. Three inserts, three outcomes, so collapsing `displaced` into `evicted` goes red | same |
| `unit` | Taking a reading does not set the use tick. Fill a tier, read its pressure ten times, insert once, and the entry that leaves is the one that was least recently used before the readings. This pins the rule `docs/lld/cache.md` states in prose | same |
| `unit` | No event is produced while the band is unchanged, and exactly one is produced on each crossing in each direction, up and down | same |
| `unit` | `set_budget` growing evicts nothing. `set_budget` shrinking evicts least recently used first until `used` fits. `set_budget` to zero empties the tier. `set_budget` to the current value is a no-op | same |
| `unit` | After a shrink, `used` is at or below the new `budget` and equals the sum of the live entries. A budget that reports more in use than it permits is the defect D-24's rationale names | same |
| `unit` | Counters saturate at `u32::MAX` rather than wrapping to a small number | same |
| `unit` | `viewport` is `u32::MAX` on every telemetry event, matching F-032 | same |
| `property` | Over a randomised insert, get, remove and `set_budget` sequence, `used` never exceeds `budget` and the sum of `evicted_bytes` across all readings equals the bytes the tier stopped holding through eviction, with refusals and displacements excluded | same |

**Mutation check, HLD 27.3.** Count `displaced` as an eviction and the three-outcome test goes red. Compute the band in floating point with a rounded threshold and the boundary fixture goes red at 269 or 270. Emit on every insert instead of on a crossing and the unchanged-band test goes red. Make `set_budget` evict most recently used first and the shrink test goes red. Let a counter wrap and the saturation test goes red. Any one of those is a plausible implementation and each has a test that kills it.

## Parity surface covered

`docs/hld/B-parity-surface.md` has a row **Core events, count 50**, "plus 53 tool events, re-shaped rather than copied". This story defines two of the re-shaped ones and delivers no event model. F-103, E16.4 in S17, is "Event model: the ~103 cornerstone events, re-shaped and documented". The appendix has no `Covered by` column in this repository to update, and no row in it is a memory or budget row, so the parity coverage here is the event count and nothing else.

`docs/hld/B-parity-surface.md` is keyed on the epic ref through its `Covered by` column in the HLD's own form. E5.4 appears nowhere in the file as held in this repository, which carries the summary table only. Recorded here rather than filled in, because inventing a row would be a parity claim nobody measured.

## Deviations

None expected.

`set_budget` is an addition to section 20's listing rather than a departure from it. The listing shows one method and a `/* ... */` where the rest of the type goes, and F-031 already filled that space with `get`, `remove` and `pressure` without a deviation. A method the listing does not show is not a method the listing forbids. If a reviewer reads section 20 as exhaustive, this becomes a `D-NN` row in the same change and the plan is updated to cite it, rather than the code landing first and the register following.

If Open question A is answered by building the ring, that is not a deviation either. Section 17.3 specifies it and `scripts/unsafe_allowlist_check.py` already permits `crates/ocelli-wasm/src/ring.rs`. It is a scope change and a dependency edge, and the wasm size budget moves for the first time.

## LLD impact

- `docs/lld/cache.md`, which gains the telemetry module, the pressure bands, the counters and `set_budget`, and a note that a reading is still not a use now that something reads on a schedule
- `docs/lld/events.md` only if Open question B lands an event-kind registry, otherwise the kinds live in `cache.md` beside F-032's

## Open questions

None. All questions were answered in the S12 consolidated design round, as recorded below.

## Decisions from the S12 design round

**A, A1. Stop at the encoding.** Shared with F-032 question A, answered once there. F-101 builds the ring.

**B, B1. Both channels.** Band crossings and budget changes are ring events. The full `CacheTelemetry` snapshot is a section 17.4 shape readback layout, with the `wasm-bindgen` surface left to F-101.

**C, C1. Evict at once on a budget shrink,** least recently used first, reporting every eviction.

**D, D1. No event-kind registry this sprint.** Shared with F-032 question B.

**E, E1. Band thresholds fixed** at one half and nine tenths as named constants.

## Recorded questions

**A. Does this story build section 17.3's event ring, or stop at the encoding? Shared with F-032, and not re-decided here.** The title says "JS-visible" and there is no channel. `ocelli-wasm` holds no `Session`, the ring does not exist, and F-101 in S16 owns "command channel, event ring buffer, state readback". F-032's plan carries this question as its Open question A with three options and a recommendation, and this plan adopts that record rather than opening a parallel one that could be answered differently. Two stories emitting into one ring cannot hold two answers to who builds it.

- **A1, shared recommendation, adopted from F-032.** Stop at the encoding. F-034 produces `TelemetryEvent` and its 48-byte `RingEvent`, both tested, and F-101 carries them. The title's "JS-visible" is recorded as not delivered.
- **A2.** Build `crates/ocelli-wasm/src/ring.rs` now, to section 17.3 exactly. Fully specified, the unsafe allowlist already names the path, and testable natively with no JavaScript. Costs a dependency edge from `ocelli-wasm` to `ocelli-cache`, a first move in the wasm size budget, and a narrower meaning for F-101's title.
- **A3.** Build the ring in `ocelli-core` instead, where it needs no `wasm-bindgen`, and let F-101 expose the pointer. Departs from section 17.3's stated path and needs a `D-NN` row.

Whichever way A is answered, it is answered once for F-032 and F-034 together. If it is answered A2 or A3, both plans are updated in the same change.

**B. Does pressure travel as a ring event, as a section 17.4 readback, or as both?** Section 17.3 says to surface `dropped` in telemetry and section 17.4 gives a copied-on-request struct for viewport state. Neither names pressure, and the two mechanisms have opposite shapes. An event is push, coalescable and lossy under a full ring. A readback is pull, exact and costs a boundary call the shell has to schedule.

- **B1, this plan's recommendation.** Both, for different questions. Band crossings and budget changes are ring events, because they are rare, they are the moments the shell must react, and losing one under overflow is visible in `dropped`. The full `CacheTelemetry` snapshot is a readback struct in section 17.4's shape, copied into a JS-owned buffer on request, because a dashboard asking once a second should not consume ring slots and should not be able to miss a reading. This plan specifies the event half in full and the readback as a layout, leaving the `wasm-bindgen` surface to F-101.
- **B2.** Ring events only. One upward channel, no readback type, and the shell reconstructs current pressure from the last event it saw. Cheaper now and wrong after any dropped event, which is the failure `dropped` exists to announce and which this option then cannot recover from without a resync rule nobody has written.
- **B3.** Readback only. No telemetry events at all, and the shell polls. Exact and simple, and the shell learns about a `Critical` crossing one poll late, which during a 300 MB load is the interval the pressure signal exists to shorten.

**C. When a budget shrinks below `used`, does the cache evict at once, refuse the shrink, or hold the excess until inserts evict it naturally?** Section 20 is silent and section 8 says only that a budget is to be respected. This decides what a JS budget control actually does to a live session.

- **C1, this plan's recommendation.** Evict at once, least recently used first, and report every eviction. The shell asked for the memory back, which during a pressure event means it needs the bytes for something else. Holding them leaves `used` above `budget`, and a budget that reports more in use than it permits is the defect D-24's rationale rejects when `insert` raised the same shape. The cost is real. A shrink can discard decoded frames a viewport is showing, and they decode again on return.
- **C2.** Refuse a shrink that would go below `used`, return the refusal, and let the shell evict explicitly or wait. Nothing is destroyed unasked, and a control that cannot reduce memory during the pressure event that prompted it is a control that does not control anything.
- **C3.** Accept the new budget and evict lazily, so `used` sits above `budget` until the next insert. No surprise eviction, and `Pressure` reports a tier over its own budget for an unbounded interval, which every consumer of the signal then has to understand as a state rather than an error.

**D. Do event kinds get a registry, like error codes? Shared with F-032 Open question B.** `ci/error-codes.json` exists because HLD section 23 requires stable numeric codes agreeing across Rust, TypeScript and a registry, and the `errors` gate asserts it. Section 17.3 gives `kind: u32` and no such requirement. F-032 and F-034 together would put five kinds in the ring, two of them from this story.

- **D1, shared recommendation, adopted from F-032.** No registry this sprint. Constants in `ocelli-cache` in one range with one comment covering both stories' kinds, and F-103's event model is where a registry belongs if one is wanted. A registry with five entries and no second language reading it yet is a gate asserting a file against itself.
- **D2.** Add `ci/event-kinds.json` and a gate now, matching the error-code shape, so the numbers are stable before anything reads them.

**E. Are the band thresholds fixed at one half and nine tenths, or does the caller set them?** This plan fixes two fractions so a test can pin them. A caller with a small GPU budget and a large decoded budget may want the GPU tier to speak earlier.

- **E1, this plan's recommendation.** Fixed in the crate for this story, as named constants beside `Pressure::band`. Two thresholds nobody tunes are two thresholds every reader understands identically, and a configurable threshold is a second place for two tiers to disagree about what `Critical` means. Revisit when a caller asks, with a measurement, which is section 26's rule for this kind of change.
- **E2.** Per-tier thresholds passed at construction. More useful to a caller with uneven budgets, and it makes `Pressure::band` a method that needs state the struct does not carry, so the band moves off `Pressure` and onto the cache.
