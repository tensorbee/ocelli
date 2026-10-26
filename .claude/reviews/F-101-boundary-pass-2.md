# F-101 boundary review, pass 2

**Reviewed**: staged tree `8d0a94a34d4b35cd38e26c5056311eab05937231`
**Previous**: `7de677af36b9122e60d2da4700c96b730f422e0a`
**Author / reviewer**: implementing xAI model / Codex
**Result**: 4 defects, 0 smells, 0 nitpicks
**Date**: 2026-10-19

## Previous findings and checks now

Allocation returning 0 stops the new helper before writing. The raw wasm
exports distinguish lifetime tickets from addresses and reject an older
ticket while preserving the current slot. A valid FrameCommitMeta object
reaches the numeric export through the new adapter. Those portions of pass 1
are repaired, but the full caller paths below still fail.

Rebuilt the dev web module with the command in the supplied evidence, then
ran `npx vitest run --config vitest.boundary.config.ts`: 3 tests passed.
Native wasm tests passed 23 cases. Core vitest passed 99 cases across 10 files.
These are current executions, not merely the implementing agent's receipts.

## Defects

### D4, public adapters authorize a stale address using the newest ticket

**Where**: `packages/core/src/bulk.ts:153` and
`packages/core/src/commands.ts:148`.
**Contract**: F-101 AC4 and the revised lifetime-ticket contract require old
allocation identities not to authorize new slots at a reused address.
**Trigger**: Reuse one sessionBulkSink. Allocate 4 bytes, commit, allocate
4 bytes again, then pass the old address to that adapter's commit_frame.
**Actual**: The address is reused. The adapter looks up its mutable current
ticket and commits the new slot with it. The stale call returns 1 and clears
the current slot. In this execution both addresses were 1835016, while the
new ticket was 268435458. The numeric address is incidental.
**Expected and regression**: Preserve lifetime identity through the public
adapter, not only the raw export. Apply the same rule to sessionCommandSink.
Add real-target tests that send delayed old operations through reused public
adapters, with a valid current operation as a control. Avoid a repair that
merely substitutes the adapter's newest ticket again.

### D5, metadata exceptions strand the one outstanding allocation

**Where**: `packages/core/src/bulk.ts:132`, `:166` and `:170`.
**Contract**: F-101 AC4 requires refused commits not to leak ownership, and
the approved one-slot model must remain usable after a rejected operation.
**Trigger**: Call writeFrame through sessionBulkSink with a four-byte payload
and metadata whose viewport is a string instead of a u32.
**Actual**: Allocation and the memory write happen first. frameCommitMeta
throws before any numeric commit or cleanup. bulk_ticket remains 268435457,
and the next alloc returns 0. No public cancel path releases it.
**Expected and regression**: Validate before acquiring ownership where
possible and make any post-allocation exception release exactly that allocation.
Do not release a newer or foreign ticket. Test malformed metadata and write
exceptions followed by a successful valid transfer on the same live module.

### D6, the existing transferred-frame metadata cannot use the live adapter

**Where**: `packages/core/src/bulk.ts:236`, `:257` and `:263`.
**Contract**: F-101's consumer map includes the existing bulk/frame handoff.
Its established HandoffMeta has series, instance, frame and bytes fields.
**Trigger**: acceptTransferredFrame receives a four-byte ArrayBuffer and its
valid HandoffMeta, then uses sessionBulkSink as its concrete sink.
**Actual**: The helper forwards that metadata to a validator expecting
viewport, columns, rows and generation. It throws `viewport is not a u32`
and also strands the slot. A matching fake sink does not expose this mismatch.
**Expected and regression**: Provide a concrete mapping/context for this
consumer, or a separately supported explicit sink with its real owner and
call path. Do not invent dimensions from the byte count or silently remove
the consumer from acceptance. Exercise the existing helper with valid transfer
metadata against the built target and prove one accepted commit.

### D7, numeric runners do not measure their registered interaction subjects

**Where**: `crates/ocelli-wasm/examples/presented_frame.rs:45`, `:70`, `:77`
and `tools/bench/src/runners/presented_frame.mjs`.
**Contract**: tools/bench/subjects.json defines window-level latency from a
main-thread pointer event through batching and a later ring drain. Scroll
also changes the resident frame. First-frame latency ends at a presented frame.
**Actual source path**: The example times a native Rust command call, raster
draw and manually enqueued event. There is no pointer event, RAF batching or
browser presentation. try_identity creates an empty stack, so Scroll changes
no selected frame and every draw receives the same constant sample. The
first-frame mode repeats an already warmed session. Wrappers return those
numbers under the existing subject ids.
**Expected and regression**: Preserve the native example as explicitly named
diagnostic data if useful, but do not submit its durations as these registered
quantities. Implement the defined interaction and presentation endpoints, or
report the instrument as unavailable until implemented. Add a discriminating
check proving scroll selects a different frame and that delays before the
proper start/end endpoints affect the measured quantity. No new budget or
representative-host sample is requested by this finding.

## Reproduction and evidence

`node /private/tmp/ocelli-pass2-adapter-probes.mjs` runs the actual TS adapters
against independently initialized instances of the rebuilt wasm module.
It exits 1 with three failed expected-behavior checks for D4, D5 and D6.
The first valid adapter commit returns 1 as a positive control. Source and
output are `/private/tmp/ocelli-pass2-adapter-probes.mjs` and
`/private/tmp/ocelli-pass2-adapter-probes.log`. Node is v24.13.0.

Build and suite logs are `/private/tmp/ocelli-pass2-wasm-build.log`,
`/private/tmp/ocelli-pass2-boundary.log`, `/private/tmp/ocelli-pass2-wasm.log`
and `/private/tmp/ocelli-pass2-core.log`. D7 is established by following the
example and runner against the registry definitions. No new timing or budget
is asserted by this pass.

## Verified clean

Raw ticket rejection, allocation limit/refusal, valid numeric metadata,
existing parser rollback, state copies and the supplied memory-growth cases
passed. No new unsafe site or shell pixel getter was found. Those controls
do not establish the failing public lifetime and handoff paths.

## Smells

None separately recorded.

## Nitpicks

None recorded.

## Limitations and handoff

Keep F-101 in progress. The CPU browser publication failures are in the
F-X003 pass-2 report. Default npm test excludes the explicit built-module
suite, so keep its build and test commands in verification evidence. No full
sprint, corpus, release-size or independent whole-sprint approval is supplied.
The nine previously reviewed instruction edits were left untouched and unstaged.
