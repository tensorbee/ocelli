# S16 product repair handoff, pass 2

**Reviewed index**: `8d0a94a34d4b35cd38e26c5056311eab05937231`
**HEAD**: `22800f8c1b916818784c63b44dde1760c3512f04`
**Reviewer**: Codex, 2026-10-19

This pass acknowledges the repaired native invalidation, source-colour
reservation, borrowed presentation, raw lifetime tickets, allocation refusal
and native GPU DPR behavior. The supplied three built-wasm cases passed after
rebuilding the dev artifact. Those are concrete improvements.

The public adapters and real browser worker expose remaining failures. The
reports contain 8 blocking findings: one LLD correction, four boundary and
instrument findings, and three browser-publication findings. No product fix,
commit, completion or verification receipt was made by the reviewer.

## Read before editing

- `.claude/reviews/F-048-coordinates-pass-2.md`
- `.claude/reviews/F-101-boundary-pass-2.md`
- `.claude/reviews/F-X003-raster-pass-2.md`
- The approved plans and their recorded repair decisions
- Existing feature scratch notes and the previous pass evidence

The nine Codex instruction refinements remain untouched and unstaged. Their
separate implementing-agent review is already present. Do not regenerate
equivalent instruction text merely to repeat that review.

## Prompt for the implementing agent

Continue S16 from the reviewed index above. Preserve unrelated work and all
earlier findings. Keep F-048, F-101 and F-X003 in progress, and F-X022/F-X023
pending. Do not commit, push, mark done, close S16 or open S17.

First close the remaining public ownership paths:

1. Carry immutable allocation lifetime identity through sessionBulkSink and
   sessionCommandSink. The current adapter takes an old address, substitutes
   its newest ticket and consumes the newer slot if the heap reused that
   address. The raw ticket test is necessary but does not test this caller.
   Record any required public compatibility decision explicitly. Test delayed
   operations through the reused public adapter itself.
2. Reject malformed metadata before allocation where possible. Make every
   later exception release only its own slot, including a write failure.
   An invalid metadata object currently strands the session and prevents the
   next valid transfer. Add a failed-then-valid sequence on one live module.
3. Join acceptTransferredFrame's existing HandoffMeta to a real target sink
   with explicit context. Its series/instance/frame/bytes object currently
   fails the viewport/grid/generation validator. Do not infer the missing
   grid from byte length or erase the consumer from acceptance.

Then repair actual CPU browser publication:

4. Use OffscreenCanvasRenderingContext2D for the owned worker canvas. The
   current HTML CanvasRenderingContext2d cast always fails in the tested
   worker. Observe an unmodified worker frame and its kind-6 event.
5. Keep a failed blit retryable. Drawing currently clears refining and marks
   settlement before putImageData succeeds. A retry returns success without
   trying the blit. Separate pending raster output from accepted presentation,
   or retain the necessary invalidation. Test one injected blit failure,
   retry, a newer generation and exactly one successful current publication.
6. Remove the full-frame to_vec and fresh clamped-array/ImageData allocation
   from the publication loop. Retain the required objects at bind/resize and
   borrow the session data safely. Cover the full worker path, not only a LUT
   clone counter around native drawing. No new unsafe, pixel getter or
   forwarding abstraction is needed for the acceptance contract.

Finally reconcile instrumentation and documentation:

7. Keep native command/raster timings separate from the registered pointer
   and scroll latency subjects. A native command and manually pushed event do
   not include pointer/RAF/batching/presentation. Scroll on an empty sequence
   with a constant sample is not frame-change latency. Implement the actual
   registered quantities or refuse those subjects honestly until ready.
   Retain the native example as explicitly named diagnostics if useful.
8. Update render-graph and viewport LLD for the 80-byte GPU DeviceCamera and
   the DPR argument, including registration and recovery. Preserve the
   distinct 96-byte ViewportState and its 64-byte camera matrix. Do not change
   the readback wire layout to match the GPU-only uniform.

## Permanent discriminating checks

The temporary probes are optional aids. The reports specify enough inputs
to create permanent tests without depending on temporary paths.

- Rebuild wasm, run the explicit boundary suite and add public-adapter lifetime,
  malformed-metadata recovery and real transferred-frame cases.
- Run a real Chromium module worker with an owned OffscreenCanvas, a 1 by 1
  source containing f32 stored 40 and the existing identity presentation.
  Require an actual opaque frame and a kind-6 event. Check failure, retry,
  refining/generation state, unchanged idle and repeated changed frames.
- Measure allocations over complete publication after setup, including pixel
  staging and browser objects. The existing native clone regression remains
  useful but proves a narrower property.
- Keep native GPU CSS regressions. Codex reran both outside the sandbox and
  observed tier A. Rendered DPR 1.25, tier B and broader browser GPU coverage
  remain separate observations.
- Challenge benchmark endpoints and actual changed frame identity. No numeric
  CPU-cost budget, host baseline or colour threshold is authorized by this
  review. No new sample under the wrong subject name should be recorded.

The browser controls in this review used a test-only context-class alias to
isolate retry and allocation behavior. That alias is not a product fix and
must not appear in production or make the permanent happy-path test pass.

## Acceptance and later verification

Keep corpus AC5 and representative-host AC6 open. The one synthetic comparator
call is not the applicable-corpus producer. The host-cost workload currently
always refuses and does not yet collect CPU-time/wall-time through a host
configuration. Separate that missing instrumentation from the missing host.
Likewise record the browser colour and reduced-resolution paths according to
their actual implementation and evidence.

Retain semantic RED and repaired GREEN at the real caller and target. Request
fresh independent review after repairs. Once the intended source, metadata
and reviewed instruction union is ready and staged, invalidate its prior
receipt before final sprint verification. Record actual results in both
evidence stores. Do not reuse the older union's receipt or call this review
a clean whole-sprint pass.
