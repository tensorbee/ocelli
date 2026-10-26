# F-X003 raster review, pass 1

**Reviewed**: staged tree `7de677af36b9122e60d2da4700c96b730f422e0a`
**Base**: `6b562be784f4365e7781869b5c511e9e0a1162a8`
**Author / reviewer**: xAI model per operator handoff / Codex
**Result**: 4 defects, 0 smells, 0 nitpicks
**Date**: 2026-10-19

## Defects

### D1, successful commands do not invalidate a settled CPU frame

**Where**: `crates/ocelli-wasm/src/session.rs:168`, `:247`,
`crates/ocelli-render/src/cpu.rs:328`.

**Contract**: F-X003 AC4 requires command-driven refinement and current
settled content. AC7 permits idle suppression only until invalidation.

**Trigger**: Bind a 1 by 1 CPU target, draw and settle generation 0, then
submit a successful SetInvert command through Session's real bulk slot.
The viewport flag changes to inverse. Draw the same target again.

**Actual**: The result is `wrote=false`, extent `[0,0]`, generation 0.
No fresh presented event can be emitted. Commands change the viewport but
never invalidate or advance the raster. `commit_frame` is similarly separate
from raster invalidation and generation ownership.

**Expected and regression**: A successful relevant command or new source
invalidates the old settled result and establishes the correct generation.
A refused or empty batch must preserve the applicable current state. Cover
window, invert, geometry, scroll and frame replacement as appropriate, using
the public Session path rather than a manual test-only call to invalidate.
Check changed output/event identity, stale settlement and unchanged idle.

### D2, DPR resize leaves the previous settlement reusable

**Where**: `crates/ocelli-render/src/cpu.rs:172`, `:279` and `:328`.

**Contract**: F-X003 AC2/AC4/AC7 require the resized device frame to be drawn,
while stale completions cannot settle newer content.

**Trigger**: Settle a 1 by 1 raster at DPR 1, change DPR to 2, then request
the new 2 by 2 settled frame using the current generation.

**Actual**: It returns idle with `wrote=false`. The sentinel output remains
unchanged. Storage changes but needs_draw/settled/generation do not.

**Expected and regression**: A successful extent or scale change must invalidate
the cached result, including a DPR change that rounds to the same extent.
Failed resize preserves prior state. Test settle, resize, new draw, idle,
and an old-generation completion without manually invalidating from the test.

### D3, valid colour sources larger than the target are refused

**Where**: `crates/ocelli-render/src/cpu.rs:297` and `:366`.

**Contract**: F-X003 AC2/AC3 declare supported colour rendering through the
shared mapping. The approved plan has no source-size-equals-target restriction.

**Trigger**: A 2 by 1 RGB source with two RGB triples is fit into a 1 by 1
target. The source and transform are valid and the destination fits.

**Actual**: `Err(Extent)`. Colour scratch is sized by target pixels, but
prepare_color asks for one entry per source pixel. Monochrome has no matching
restriction. Ordinary colour downsampling fails when the image exceeds the
viewport, including progressive reduced targets and browser window resizing.

**Expected and regression**: Size or reuse source-colour storage during setup
or source change, or adopt another concrete sampling arrangement that preserves
ColorTransform semantics. Do not allocate inside the draw or silently narrow
colour acceptance. Test source larger, equal and smaller than target for RGB
and stored-keyed palette inputs, with no-write controls for unsupported ICC.

### D4, the session clones owned LUT buffers inside the draw path

**Where**: `crates/ocelli-wasm/src/session.rs:255`,
`crates/ocelli-pixel/src/lut.rs:10` and `:151`/`:241`.

**Contract**: HLD section 26 and F-X003 AC7 prohibit render-loop allocation.
The test must cover the whole draw entry point.

**Trigger and source evidence**: A Presentation containing a selected modality
or VOI sequence owns a LutDescriptor with `inputs` and `values` Vecs.
`Session::draw_cpu_mono` clones that Presentation before calling the raster,
including before the raster's idle check. Derived Clone duplicates both owned
non-empty buffers on every such call. The raster's allocations counter only
counts its own scratch replacement and cannot observe these clones.

**Expected and regression**: Borrow the disjoint session state or use another
concrete allocation-free ownership arrangement. Test actual allocations around
the Session draw with non-empty LUT sequences, both a draw and a settled idle
call. Preserve the no-allocation requirement without another forwarding layer
or new dynamic dispatch. This finding is established from ownership and call
path inspection. No allocator trace or performance measurement is claimed.

## Evidence

`bin/ocelli.sh test ocelli-render` exited 0 with 133 passed and 36 ignored
across its test binaries, plus 0 doc tests. The 11 committed CPU tests passed.
`bin/ocelli.sh test ocelli-wasm --lib` exited 0 with 19 tests. These results
are independent reruns now, not the implementer's old receipts.

Additional probes run in the isolated tree exported before instruction edits:

```bash
bin/ocelli.sh test ocelli-render --test cpu review_
bin/ocelli.sh test ocelli-wasm --lib review_
```

The first exits 101 with two semantic failures: D2 returns idle and D3 returns
Extent. The second exits 101 because D1 returns idle after successful inversion.
Each compiles and reaches its intended assertion. The isolated root is
`/private/tmp/ocelli-s16-review-b1q22i4y`. Logs are
`/private/tmp/ocelli-s16-cpu-probes.log` and
`/private/tmp/ocelli-s16-session-probe.log`. No mutation of product source
was needed to expose these defects. Canonical product source is unchanged.

## Verified clean

The raster calls shared LutChain and ColorTransform arithmetic. Selected
sequence behavior, explicit inversion, black borders, preset owners, copied
state and unsupported ICC are represented in focused tests. Singular maps and
stale explicit settlement refuse without output writes in the existing suite.
No tolerance, baseline or wasm size budget was changed by this review.

## Smells

None separately recorded.

## Nitpicks

None recorded.

## Missing implementation and unavailable evidence

These are separate from the reproduced defects and cannot all be attributed
to the missing representative host:

- The wasm canvas binding still has only the GPU attachment route. Session's
  CPU methods have no live canvas presentation wiring or colour session draw.
  An offscreen RGBA buffer and kind-6 enqueue are not a visible browser frame.
- The bounded tier A/C candidate producer and comparator invocation are absent.
  AC5 is therefore missing implementation as well as observations.
- All six benchmark runner bodies unconditionally throw. Instrument readiness
  remains missing for F-101 AC8 and F-X003 AC8. This is not an implementation
  that merely needs an equipped host to produce samples.
- The representative GPU-less host remains unavailable. AC6, release/SIMD
  observations and measured CPU costs are unperformed. No budget is invented.
- The sandboxed DPR consumer attempt could not open an adapter. Outside the
  sandbox, tier A opened and reproduced F-048's synthetic GPU pixel mismatch.
  That is not a tier A/C raster comparison or WebGL2 browser observation.

## Limitations and handoff

Keep the story in progress. Repair D1 to D4, finish the approved integration
work and preserve each still-unavailable acceptance row explicitly. Obtain a
fresh independent pass before completion. No whole-sprint review or final
verification receipt is supplied by this review. New Codex guidance edits
remain separate from the pinned implementation tree and require another
reviewer.
