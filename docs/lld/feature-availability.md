# Feature availability

**F-IDs that contributed:** F-037, F-041, F-X001
**Last updated:** 2026-09-15

The contract every tier-gated feature declares, and the distinction between a
working primary path, a declared fallback and an unavailable feature. This is
a documentation contract today. It does not introduce a feature registry,
stable feature number, tier number or boundary payload.

HLD section 7 says every feature declares the tier it needs. Section 31 says a
tier-A kernel declares a CPU or worker fallback, or marks its feature
unavailable. Deviation D-07 extends that rule to tier C: a feature that cannot
run on the resolved tier reports unavailable and never silently produces a
different answer.

## The mechanisms that already exist

This contract has one owner for each fact rather than a second implementation
of any of them.

| Existing mechanism | Owner | What this contract takes from it |
|--------------------|-------|----------------------------------|
| Startup tier resolution and `Caps` | F-004 in `ocelli-render` | One resolved tier and the capabilities a feature may inspect |
| Ordered adapter fallback and retained failures | F-X016 in `ocelli-render` | The tier and limits come only from an adapter whose device opened |
| Benchmark, adapter-type and renderer-string evidence | F-004 in `ocelli-render` | A software adapter resolves tier C instead of being treated as tier B |
| `ErrorCode::Unavailable` | F-005 in `ocelli-core`, the TypeScript mirror and `ci/error-codes.json` | Stable code 700 for a feature that has no valid path |
| `ComputeError::Unavailable { required, resolved }` | F-008 in `ocelli-compute` | An in-process typed example that keeps the two tier roles distinct |
| `caps::opens_a_device` | F-037 in `ocelli-render` | Whether the session has a GPU device at all. Tier C has none, so every GPU-requiring feature is `Unavailable` there by construction rather than by each feature checking |
| `DeviceState` and `GpuContext::recover` | F-037 in `ocelli-render` | A lost device is an OBSERVED state rather than a later call failing, so a feature can report unavailable for the right reason while a rebuild is pending |
| `VoiParams::from_chain` | F-041 in `ocelli-render` | The first feature-level refusal of this shape: a chain driven by a Modality LUT Sequence or a VOI LUT Sequence has no HLD section 18.4 uniform to be expressed in, so the GPU path is `Unavailable` and tier C is `Available` through `LutChain::map_into`. The two error variants name WHICH stage, because a caller reporting the feature unavailable has to say |

The detailed resolver and evidence rules remain in
[tier-resolution.md](tier-resolution.md). This file begins after that one
session-wide answer exists.

## One declaration per feature, when the feature lands

Each feature story declares its answer for tier A, tier B and tier C in its own
LLD and implementation. An omitted tier is not an answer. No central registry
of future features is created before those features have producers and
consumers.

A declaration carries these roles:

| Role | Meaning |
|------|---------|
| Feature | The user-visible operation being evaluated. It is a local typed or human name until a boundary story gives it a stable identity |
| State | `Available`, `Degraded` or `Unavailable` |
| Required tier | The primary implementation's declared tier requirement |
| Resolved tier | The session tier returned by the existing resolver |
| Fallback | The specific CPU, worker or lower-tier path used only for `Degraded` |

Required and resolved are roles, not interchangeable labels. A report that
swaps them tells the operator that the implementation is deficient when the
session is missing a capability. `ComputeError::Unavailable` already preserves
this distinction for compute kernels.

Feature identity is deliberately not stable transport in this story. A future
feature story may use a local enum when it has two real users. It must not add a
global numeric identifier merely to populate this table.

## The three states

### `Available`

The feature's declared primary path can run on the resolved tier and its other
capability checks pass. No fallback is named because none is running.

Availability is not inferred from the presence of a WebGL2 or WebGPU API. It
uses the resolved tier and `Caps`, after the software-adapter decision and any
adapter fallback have completed.

### `Degraded`

The primary path cannot run, and the feature uses the specific fallback it
declared for this resolved tier. The declaration names that fallback.

A fallback may execute on the CPU, in a worker or through a lower-tier GPU
path. It must preserve the feature's specified result. It may trade latency or
throughput, but it must not quietly change the clinical answer. Pixel
arithmetic reuses `ocelli-pixel`, as HLD section 18 requires. A second LUT
chain behind a tier branch is not a fallback.

The existence of some lower-tier algorithm is not enough. If the feature did
not declare and test that path, its state is `Unavailable`.

### `Unavailable`

Neither the primary path nor a declared valid fallback can run on the resolved
tier. No fallback is named, no substitute algorithm runs, and stable error code
700 is the transport code reserved for this answer.

Unavailable is a feature-level result. It is not a poisoned wasm instance, a
fatal viewport state or evidence that tier resolution failed. A healthy tier C
session can have available CPU features and unavailable GPU-only features at
the same time.

## Decision order

For each feature invocation or presentation:

1. Read the session's already resolved tier and `Caps`. Do not probe an adapter
   again.
2. If the primary path is constructible and its other capability checks pass,
   report `Available` and run it.
3. Otherwise, if the feature declared a valid fallback for the resolved tier,
   report `Degraded`, name that fallback and run it.
4. Otherwise report `Unavailable` with code 700. Run no substitute.

This order does not rerun tier resolution and does not reinterpret its adapter
evidence. The resolver answers what the session can construct. The feature
answers what it can honestly do with that session.

## Shell presentation

The shell keeps an unavailable feature visible in an unavailable state. It
does not silently hide the control, leave a blank canvas or present a different
operation under the same name.

The user-facing explanation names:

- the feature
- the capability it requires
- the resolved session tier
- one truthful action available to the user

For a GPU-only feature on tier C, the minimum form is that the named feature is
unavailable because the session resolved tier C, followed by an action such as
using a GPU-capable session. An operator override may be offered only when the
existing resolver says that override is constructible. It is not a promise
that a failed adapter can be forced open.

Adapter names, renderer strings, failed request text and benchmark figures are
diagnostic evidence. They remain available to support tooling and logs, but do
not replace the clinical-facing explanation.

`Degraded` remains visible too. The shell identifies the fallback when that
knowledge changes expected performance or operation. It must not label a
degraded path as the unavailable fatal state.

## Stable transport deliberately stops at code 700

`ErrorCode::Unavailable` is already number 700 in Rust, TypeScript and the
registry. It carries no stable operands today. There is no conversion from
`ComputeError::Unavailable` to the core 32-byte `Record`, and
`ocelli-compute` does not depend on `ocelli-core`.

This story does not assign the record's three `u64` operands to a feature
identifier, required tier and resolved tier. It also does not invent numeric
tier encodings. F-101 owns the dependency-safe boundary conversion when a real
producer, ring and shell consumer exist. That design must choose the stable
identities and add cross-language tests together. Until then, producers keep
typed context in-process and must not encode ad hoc integers into the record.

See [errors.md](errors.md) for the code registry and current absence of a live
producer.

## Ownership of the remaining work

| Work | Owner |
|------|-------|
| Stable feature identity, tier encoding and conversion to the boundary record | F-101 |
| Shell transport and tested actionable presentation | F-101, with the public API designed by F-100 |
| The tier A, B and C declaration for volume rendering | F-X005 |
| Each later feature's three-tier declaration and fallback tests | That feature's own story |

F-X005 may decide that a CPU volume path is too slow and report volume
rendering unavailable on tier C. That is an application of this contract, not
a change to its states.

## What this contract does not build

- A CPU rasteriser, stack viewport, MPR path or volume renderer.
- A feature enum or registry for work that has not landed.
- Stable unavailable operands or numeric tier encodings.
- A `From<ComputeError>` conversion or a new crate dependency.
- Shell copy, a boundary event, a disabled control or its exact wording.
- Pixel, LUT or geometry arithmetic.

There is no fixture or code test in this story because it changes no executable
behavior. The conditional test rows in the approved plan apply only if stable
operands are introduced, and they are not introduced.
