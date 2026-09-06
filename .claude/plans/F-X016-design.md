# F-X016, Try the next adapter when the best candidate cannot open a device, and record what was attempted

**Status**: approved
**Epic ref**: Y1.11
**Sprint**: S04
**Estimate**: 1w

## Normative source, transcribed

### `docs/hld/05-rendering.md`, section 7

```text
- **Two capability tiers, one codebase.** Tier A is WebGPU: compute shaders, storage buffers, 3D textures to 2048. Tier B is WebGL2 through wgpu's downlevel profile: fragment shaders only, no compute, no storage buffers, a conservative 3D-texture floor of 256. Every feature declares the tier it needs; the tier resolves once at startup.

- **Volume rendering runs on both tiers**, because a 3D-texture ray-cast in a fragment shader is tier-B legal. Anything wanting compute — GPU segmentation, histogram passes, compute-based resampling — is tier A only and must degrade, not fail.
```

### `docs/hld/19-render-graph.md`, section 22

```rust
pub struct Caps {
    pub compute: bool,
    pub max_tex_3d: u32,
    pub max_buffer: u64,
    pub tier: Tier, // A = WebGPU, B = WebGL2 downlevel
}
```

```text
- **Pipelines compile at init**, keyed by (pass kind, blend mode, tier). Never compile a shader mid-frame.
```

### `docs/hld/20-errors-and-panics.md`, section 23

```text
thiserror in the core crates; the boundary maps everything to a stable numeric code and a message. The important part is what happens when that is not enough.
```

```text
- Error codes are stable and versioned. The shell switches on the code; the message is for humans and may change.
```

### `docs/hld/11-decision-log.md`, decisions D5 and D6

```text
| D5 | Single-threaded, one wasm instance per worker | wasm-bindgen-rayon from the start | Stays on stable Rust |
| D6 | WebGPU primary, WebGL2 a declared tier | WebGL2 only; WebGPU only | Two shader paths; compute degrades, never fails |
```

### `docs/hld/DEVIATIONS.md`, D-07

```text
| D-07 | §7, "Two capability tiers, one codebase", both of them GPU | A third tier, **C, CPU**. The resolved tier may be `Cpu`, and every tier-gated feature declares its CPU answer | §7 leaves a machine with neither WebGPU nor WebGL2 rendering nothing at all, which is a failure mode the specification does not name and does not intend. Operator decision, and spike A7.1 establishes GPU-less sessions as a primary clinical path rather than a fallback. F-X001 to F-X005. | Post-bootstrap |
```

### `docs/sprints/allocation.json`, the architecture note

```text
probe.rs measure_chosen calls request_device on the single adapter choose_candidate returned and never tries another, while native enumerate_adapters(Backends::all()) commonly returns several. On a host with a broken Vulkan ICD beside a working GL driver the best candidate is the Vulkan A-candidate, request_device fails, and the session resolves tier C with a tier-B path present and unattempted. Tier C renders nothing until F-X001 to F-X004, so the outcome is renders nothing rather than renders on tier B. classify's step 4 was justified by the sentence there is no GPU path to be had, and the S03 review's fourth pass replaced that with what is true, that the best candidate could not open a device and no other adapter was tried. THE COST IS THE EVIDENCE DESIGN AND NOT THE LOOP. TierSignals carries a single device_created bool, so it becomes per-adapter or a list of attempts, classify's step 4 and the NoDevice diagnosis change meaning, the classify_is_total invariant that a GPU tier ALWAYS means a device was created needs restating over the new shape, and the OCELLI_TIER override clamp reads the same field. The story must also decide the fallback order and whether a failed adapter is recorded.
```

## What the specification does not cover

The HLD requires one startup resolution and names tiers A and B. D-07 adds tier
C. Neither source says what to do when adapter enumeration returns several
candidates and `request_device` fails on the preferred one.

This plan therefore decides the following repository details:

1. Candidate fallback uses the existing preference relation for every attempt,
   not backend enumeration order. All A candidates precede all B candidates.
   Within a tier, discrete, integrated, virtual, other, then CPU device type is
   the order. Exact ties retain enumeration order.
2. A failed `request_device` is retained in evidence. The failure is not folded
   into `NoAdapter`, and it is not discarded when a later adapter opens.
3. `NoDevice` means every candidate was attempted and every request failed.
   It no longer means only that the preferred candidate failed.
4. Once an adapter opens, the existing benchmark and software-adapter decision
   run on that adapter. A software verdict resolves tier C. This story does not
   continue probing lower-ranked adapters after a successful device creation.
5. An operator override is constructible only against the adapter whose device
   opened. A failed A candidate does not make tier A constructible when the
   successful fallback is tier B.

The exact stability contract for the recorded `RequestDeviceError` text is not
specified. The recommendation is to retain it as diagnostic text that may
change, while tests assert the adapter identity and failed outcome rather than
matching wgpu wording.

## Approach

### Replace the cross-field boolean with a state that cannot lie

`TierSignals` currently permits impossible combinations such as
`device_created = true` with no adapter. Replace the boolean and the separately
selected candidate with one probe outcome:

```rust
pub struct FailedAdapter {
    pub adapter: AdapterFacts,
    pub reason: String,
}

pub enum ProbeOutcome {
    NoAdapter {
        adapters_seen: usize,
    },
    NoDevice {
        adapters_seen: usize,
        failed: Vec<FailedAdapter>,
    },
    Opened {
        adapters_seen: usize,
        failed: Vec<FailedAdapter>,
        adapter: AdapterFacts,
        fill_rate: Option<FillRate>,
    },
}

pub struct TierSignals {
    pub probe: ProbeOutcome,
    pub simd: SimdSupport,
    pub bands: FillRateBands,
}
```

This is one concrete state machine, not a trait or generic. Its three variants
are the three cases `classify` already has to consider. `Opened` names the
adapter that actually owns the successfully created device, so `Caps`, the
benchmark verdict, and override clamping cannot accidentally use a different
enumerated adapter.

The `Vec` and diagnostic `String` allocate only during startup probing. They
are not reachable from the render loop.

### Rank every candidate once, then attempt in that order

Extract the existing `choose_candidate` ordering into a function returning all
candidate indices in preference order. `choose_candidate` can be retired once
both production code and its tests use the ordered list.

`probe.rs` iterates that list. For each adapter it requests an empty feature set
and that adapter's own limits, exactly as today. On failure it appends a
`FailedAdapter`. On success it measures fill rate, returns `Opened`, and drops
the transient device before resolution returns. If the list is empty it returns
`NoAdapter`. If the list is exhausted it returns `NoDevice` with every failure.

The browser path usually yields one adapter, so the same loop has one element
there. No browser-specific branch or binding is added.

### Restate classification and override invariants over the opened adapter

`classify` derives its candidate only from `ProbeOutcome::Opened`.

- `NoAdapter` resolves tier C with `DecidedBy::NoAdapter`.
- `NoDevice` resolves tier C with `DecidedBy::NoDevice`.
- `Opened` runs the existing benchmark, adapter-type, and renderer-string
  combination rule over the successful adapter.
- A GPU result implies `ProbeOutcome::Opened`.
- Tier A requires the opened adapter to be an A candidate.
- Tier B requires an opened A or B candidate.
- A requested tier A is refused when an A candidate failed and only a B
  candidate opened.

`TierEvidence` gains the failed-attempt list and takes `adapters_seen`, the
opened candidate, fill rate, and device-created truth from `ProbeOutcome`.
The redundant `device_created` field is removed. A caller can tell the
difference between no candidate, all candidates failed, and a fallback that
opened.

### Anticipated implementation write set

- `crates/ocelli-render/src/caps.rs`
- `crates/ocelli-render/src/probe.rs`
- `crates/ocelli-render/src/lib.rs`
- `crates/ocelli-render/tests/classify_is_total.rs`
- `docs/lld/tier-resolution.md`

No manifest, gate, threshold, shared sprint record, or HLD file should change.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none. Attempt evidence allocates once during startup
- unsafe: none
- Tier A (WebGPU): full. Failed higher-ranked adapters are recorded before a working A adapter is used
- Tier B (WebGL2): full. A working B adapter is tried after all preferred A adapters that fail to open
- Tier C (CPU): full. It is selected only after no candidate exists, all candidates fail, or the successful adapter is classified as software

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| `unit` | Candidate indices are ordered A before B, then by the existing device-type preference, with enumeration order breaking exact ties | `crates/ocelli-render/src/caps.rs` |
| `unit` | `NoAdapter`, all-failed `NoDevice`, and `Opened` after one or more failures produce distinct evidence and decisions | `crates/ocelli-render/src/caps.rs` |
| `unit` | A failed A adapter followed by an opened B adapter resolves B under auto, refuses an A override, and permits a B override | `crates/ocelli-render/src/caps.rs` |
| `unit` | A successful adapter that the benchmark calls software still resolves C and does not consult a lower-ranked adapter | `crates/ocelli-render/src/caps.rs` |
| `property` | Every GPU resolution has an opened adapter, tier A has an opened A candidate, tier B has an opened candidate, tier C carries zero GPU limits, and `NoDevice` contains at least one failed candidate | `crates/ocelli-render/tests/classify_is_total.rs` |
| `unit` | The startup short-circuit for a requested tier C creates `ProbeOutcome::NoAdapter` without invoking adapter enumeration | `crates/ocelli-render/src/probe.rs` |

`fixture` is not applicable. This story computes no pixel or geometry value.
Its ordering and evidence decisions derive from section 7, D-07, and the story
allocation rather than from DICOM arithmetic.

The mutation checks change the first attempt from the highest-ranked candidate
to the last, discard the first failed attempt, and let a failed A candidate
make an A override constructible after B opened. Each corresponding test must
fail independently.

## Parity surface covered

None. `docs/hld/B-parity-surface.md` has no `Covered by` column and contains no
row keyed by Y1.11. This story corrects startup adapter selection and does not
add a viewport, tool, blend mode, VOI function, transfer syntax, segmentation
representation, event, or interop adapter.

## Deviations

D-07, existing. This story makes the third tier reachable only after every GPU
candidate has genuinely been exhausted. No new deviation is needed.

## LLD impact

`docs/lld/tier-resolution.md` will replace the single-candidate procedure with
the ordered attempt sequence, define `NoDevice` as exhaustion of all
candidates, document retained failures, and restate override construction over
the successfully opened adapter.

## Open questions

None. Retain request-device text as explicitly unstable diagnostics. Stop
fallback after the first successful device even if later software evidence
resolves tier C.
