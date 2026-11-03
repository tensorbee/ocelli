# F-113, memory profile for a 4D load

**Status**: approved
**Epic ref**: E18.4
**Sprint**: S40
**Estimate**: 3w

## Normative source, transcribed

`docs/hld/06-memory-and-cache.md` section 8 says this.

```text
One budget, three tiers, explicit eviction. Encoded bytes are transient
and dropped as soon as a frame decodes. Decoded frames sit in an LRU
sized by the caller. GPU textures are their own tier with their own
pressure signal, because evicting a texture and evicting a frame have
very different costs.
```

The same section says a 300 MB volume load has no pause behaviour to
tune around, only a budget to respect. It does not define 300 MB as a
pass threshold.

F-034 already reports pressure. This story does not replace that
reading.

## What the specification does not cover

Section 8 does not give a 4D byte formula.

That gap is a local decision. It is not an HLD formula.

Decoded bytes are `columns * rows * slices * times * samples *
bytes_per_sample`. `times` is the fourth dimension and is at least 1.
`samples` is 1 or 3. `bytes_per_sample` is 1 or 2. Every dimension is
an integer of at least 1. Any other input returns nothing. The product
is computed in `u64`. A product that does not fit in `usize` returns
nothing.

Encoded used bytes are 0 after decode. The function does not allocate
the volume and does not read patient bytes.

GPU bytes are the caller figure. On tier C a non-zero GPU figure
returns nothing, and a zero figure reports GPU used of 0. Admission
uses the same comparison as `Lru::would_admit`, through one shared
function, so the profile cannot grow a second rule.

## Problem and reproducer

Nothing reports the three tiers for a load described by dimensions. A
profile that allocated a 300 MB buffer, or that left encoded bytes
equal to the decoded size, would miss section 8.

## Scope, API and compatibility

In scope: `crates/ocelli-cache/src/profile.rs` and a shared admission
helper used by `Lru::would_admit`.

Out of scope: a JavaScript wrapper, a live heap dump, a change to
eviction, and any patient fixture.

```rust
pub struct LoadSpec { /* dimensions, three budgets, gpu_bytes, tier_c */ }
pub struct LoadProfile { /* three Pressure values, decoded_bytes, flags */ }
pub fn profile_load(spec: LoadSpec) -> Option<LoadProfile>
```

When decoded bytes are above the decoded budget, `decoded_bytes` still
carries the request and decoded `used` stays 0. The same shape holds
for GPU bytes off tier C.

## Consumer and provider map

| Consumer / entry point | Concrete provider | Caller context / policy | Observable output |
|------------------------|-------------------|-------------------------|-------------------|
| Cache tests and later callers | `profile_load` | synthetic dimensions | three pressures or nothing |
| `Lru::would_admit` | `admits` | existing caches | the same boolean as today |

## State transitions and invariants

The function is stateless. It does not insert into a cache. A refused
spec leaves no cache changed, because no cache is passed in.

## Ownership, commit and recovery

Not applicable. No allocation of the volume and no external resource.

## Numbered acceptance inventory

| ID | Trigger and expected behavior | Test / observation | Wrong implementation rejected | Evidence stage / owner |
|----|-------------------------------|--------------------|-------------------------------|------------------------|
| AC1 | 512 by 512 by 300 by 1, one sample, two bytes, is 157286400 | `profile.rs` tests | A product that drops a dimension | implementation |
| AC2 | 64 by 64 by 8 by 4, one sample, two bytes, is 262144 | the same | Treating times as unused | implementation |
| AC3 | Encoded used is 0 | the same | Copying decoded bytes into the encoded tier | implementation |
| AC4 | A decoded budget one byte under the product is not admitted and used stays 0 | the same | Admitting past the budget | implementation |
| AC5 | times 0, samples 2, and a product that overflows return nothing | the same | Wrapping the product | implementation |
| AC6 | Tier C with gpu_bytes 0 reports GPU used 0. Tier C with a non-zero GPU figure returns nothing | the same | Claiming a texture on tier C | implementation |
| AC7 | `would_admit` still matches the existing boundary cases | `lru.rs` tests | A second comparison that drifts | implementation |

## Approach

`admits(budget, bytes)` is `budget > 0 && bytes <= budget`.
`Lru::would_admit` calls it. `profile_load` calls it for the decoded
tier and, off tier C, for the GPU tier.

The two hand products are written out in the tests as multiplied
literals, not as a call to the function under test.

## Dependencies, write ownership and resources

Depends on F-034, which is done. Writes `ocelli-cache` only. No GPU.
The new function is not called from `ocelli-wasm`, so a size move is
not expected. If the render module leaves the 5 percent band, that
move is declared under F-110's rule rather than hidden.

## Verification and evidence reuse

`bin/ocelli.sh test ocelli-cache profile` then the feature profile.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none
- unsafe: none
- Tier A (WebGPU): full, GPU bytes are the caller figure
- Tier B (WebGL2): full, the same figure
- Tier C (CPU): GPU used is 0. A non-zero GPU claim returns nothing

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| fixture | hand products 157286400 and 262144 | `profile.rs` |
| admission | the shared comparison | `lru.rs` and `profile.rs` |

The fixtures cite section 8's three tiers. They are not DICOM samples.

## Parity surface covered

None.

## Deviations

None.

## LLD impact

- `docs/lld/cache.md`

## Open questions

None.
