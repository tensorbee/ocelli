# F-X001, Tier C, software-adapter detection, and the feature-availability contract

**Status**: approved
**Epic ref**: X1.1
**Sprint**: S04
**Estimate**: 4w

## Normative source, transcribed

### `docs/hld/05-rendering.md`, section 7

```text
- **Two capability tiers, one codebase.** Tier A is WebGPU: compute shaders, storage buffers, 3D textures to 2048. Tier B is WebGL2 through wgpu's downlevel profile: fragment shaders only, no compute, no storage buffers, a conservative 3D-texture floor of 256. Every feature declares the tier it needs; the tier resolves once at startup.

- **Volume rendering runs on both tiers**, because a 3D-texture ray-cast in a fragment shader is tier-B legal. Anything wanting compute — GPU segmentation, histogram passes, compute-based resampling — is tier A only and must degrade, not fail.
```

### `docs/hld/26-differentiating-capabilities.md`, section 31

```text
- **Every tier-A kernel declares a fallback** — CPU, or a worker — so a feature degrades rather than fails on WebGL2. A kernel with no fallback marks its feature unavailable; it never silently produces a different answer.
```

### `docs/hld/15-lut-chain.md`, section 18

```text
This is the highest-risk arithmetic in the project. It is specified in DICOM PS3.3 C.11 and the stages apply strictly in order. Implement it once, in ocelli-pixel, and let the shader read the parameters — do not let a second copy of this logic appear anywhere.
```

### `docs/hld/20-errors-and-panics.md`, section 23

```text
- Error codes are stable and versioned. The shell switches on the code; the message is for humans and may change.

- A poisoned instance surfaces to the user as a viewport-level error state, never as a silent blank canvas.
```

### `docs/hld/DEVIATIONS.md`, D-07

```text
| D-07 | §7, "Two capability tiers, one codebase", both of them GPU | A third tier, **C, CPU**. The resolved tier may be `Cpu`, and every tier-gated feature declares its CPU answer | §7 leaves a machine with neither WebGPU nor WebGL2 rendering nothing at all, which is a failure mode the specification does not name and does not intend. Operator decision, and spike A7.1 establishes GPU-less sessions as a primary clinical path rather than a fallback. F-X001 to F-X005. | Post-bootstrap |
```

```text
**What it does not claim.** Interactive volume ray-casting on the CPU. F-X005
decides between a slow path and reporting the feature unavailable, against a
measurement. It must not decide the third thing, which is a CPU path that
quietly produces a different image from the GPU one. §31's rule generalises:
**a feature that cannot run on the resolved tier reports unavailable, and never
silently produces a different answer.**
```

### `docs/spikes/A7-tier-c.md`, the detection finding

```text
**So tier resolution must distinguish a hardware adapter from a software one,
and must not treat "WebGL2 is present" as "tier B is appropriate".** This is a
scope addition to F-X001 and is recorded there.
```

```text
**The micro-benchmark is the one to trust, and the strings are the hint.** A
renderer string is a claim. A measured fill rate is a fact.
```

## What the specification does not cover

F-004 already delivered `Tier::Cpu`, the three-signal software-adapter
decision, startup probing, the operator override, and `Caps`. F-005 and F-008
already delivered `ErrorCode::Unavailable` and
`ComputeError::Unavailable { required, resolved }`. Reimplementing those in
this story would create two owners for one decision.

What remains unspecified is the general feature contract above the compute
kernel case. The HLD does not define a feature identifier, availability value,
fallback vocabulary, boundary payload, or shell presentation. It also does not
say whether a feature not yet implemented should be declared now.

This plan recommends a written contract now and no speculative registry of
future features. Each feature story declares one of `Available`, `Degraded`, or
`Unavailable`, names its required and resolved tier, and names a fallback only
for `Degraded`. `Unavailable` uses stable code 700. The shell must keep the
feature visible and explain what capability is missing and what the user can do.
It must not silently hide the control or substitute a different algorithm.

## Approach

1. Treat the existing F-004 tier resolution and software-adapter evidence as
   the implementation of this story's first two title clauses. Verify and cite
   them rather than copying them.
2. Add `docs/lld/feature-availability.md` as the contract future viewport,
   render, codec, and compute stories must implement. Define the three states,
   required fields, fallback rule, and shell presentation rule.
3. Keep stable transport on the existing `ErrorCode::Unavailable`. Reserve its
   three `u64` operands as feature identifier, required tier, and resolved tier.
   Define stable numeric tier encodings beside the contract only if the operator
   approves this boundary now.
4. Add a concrete mapping from `ComputeError::Unavailable` to the core `Record`
   only if the dependency direction can remain acyclic. Otherwise record that
   the mapping belongs to the boundary story F-101. Do not add a forwarding
   wrapper or reverse the existing crate dependency graph.
5. Add shell wording tests only if the stable feature identity is approved.
   The minimum user-facing form is: the named feature is unavailable because
   this session resolved tier C, and the operator override or a GPU-capable
   session is the available action. Diagnostic adapter evidence stays separate
   from clinical-facing text.
6. Update the tier and error LLDs to point to the one contract. F-X005 later
   decides `VOLUME_3D` against it without changing its shape.

### Anticipated implementation write set

Minimum, recommended:

- `docs/lld/feature-availability.md`
- `docs/lld/tier-resolution.md`
- `docs/lld/errors.md`
- `docs/lld/gpu-ownership.md`
- `docs/lld/README.md`

If the operator approves stable feature and tier operands now, also:

- `crates/ocelli-core/src/error.rs`
- `ci/error-codes.json`
- `packages/core/src/errors.ts`
- `packages/core/src/errors.test.ts`
- `scripts/error_code_check.py`
- `scripts/tests/test_error_code_check.py`

No LUT implementation, raster path, or future feature enum is added.

## Boundary and tier

- wasm-bindgen: not touched
- Pixels across the boundary: no
- Render-loop allocation: none
- unsafe: none
- Tier A (WebGPU): full. Features requiring A are available when their other capability checks pass
- Tier B (WebGL2): full. A-only features use their declared fallback or report unavailable
- Tier C (CPU): full. Stack and MPR later gain declared CPU paths, while unsupported features report unavailable

## Tests

| Category | What it proves | Where |
|----------|----------------|-------|
| `unit` | Unavailable records preserve feature, required tier, and resolved tier in distinct operand roles | `crates/ocelli-core/src/error.rs`, if operands are approved |
| `unit` | The shell maps unavailable to actionable text and never maps it to a fatal poisoned-instance state | `packages/core/src/errors.test.ts`, if operands are approved |
| `unit` | Stable tier and feature numbers agree across Rust, TypeScript, and the registry | existing error-code guard, if operands are approved |

`fixture` is not applicable. This story performs no pixel or geometry
arithmetic. Section 18 is a constraint that future tier C rendering reuses
`ocelli-pixel`, not arithmetic implemented here.

## Parity surface covered

None. Appendix B has no `Covered by` column and no X1.1 row. This contract
supports future viewport and transfer-syntax parity but implements none today.

## Deviations

D-07, existing. No new deviation is needed.

## LLD impact

Create `docs/lld/feature-availability.md`, then link it from tier resolution,
errors, GPU ownership, and the LLD index. The file records which parts already
exist and which remain owned by later boundary and feature stories.

## Open questions

None. This story records the contract over the mechanisms already landed. It
does not add unused stable operands. F-101 owns the dependency-safe conversion
when the boundary has a real producer and consumer.
