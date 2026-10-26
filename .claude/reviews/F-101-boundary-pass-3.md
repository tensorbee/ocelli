# F-101 boundary repair review, pass 3

**Reviewed**: staged tree `f6057868244a71679ebfd95a17e440084699aa76`
**Previous**: `8d0a94a34d4b35cd38e26c5056311eab05937231`
**Author / reviewer**: implementing xAI model / Codex
**Result**: 0 defects, 0 smells, 0 nitpicks in the scoped repairs
**Date**: 2026-10-19

## Previous findings

Pass 2 D4 is closed in the inspected public adapter path. Address-only
operations refuse. The helpers capture the allocation ticket before writing
and use the ticket-taking commit/apply operations. Old and foreign tickets
do not authorize or release the new slot. Same-address reuse is covered by
the built-target suite through the public adapters.

D5 is closed for the tested metadata and write exceptions. The live metadata
is validated before allocation. After acquiring a slot, the helper catches
write/commit exceptions and releases the captured lifetime only. Recovery
on the same module is covered by the permanent boundary suite.

D6 is closed for the explicit transfer path. acceptTransferredFrame takes
a separately supplied FrameCommitMeta target and retains the handoff object
in the submitted metadata. It does not infer dimensions from byte count.
The valid transferred-frame case reaches the numeric wasm sink.

D7's misleading measurements have been removed from the registered subjects.
The three interaction runners now refuse with instrument unavailable. The
native executable is explicitly diagnostic. This is honest partial delivery,
not an implemented interaction instrument or a new acceptance sample.

## Evidence executed now

Rebuilt the web dev artifact from the reviewed product source:

```bash
wasm-pack build crates/ocelli-wasm --dev --target web --out-dir target/boundary-wasm
npx vitest run --config vitest.boundary.config.ts
bin/ocelli.sh test ocelli-wasm
npx vitest run
```

All exited 0. The explicit built-module suite passed 5 cases. Native wasm
tests passed 25 cases, with 0 doc tests. Default vitest passed 103 cases in
10 files and excludes the built-module suite. Logs:
`/private/tmp/ocelli-pass3-build.log`, `/private/tmp/ocelli-pass3-boundary.log`,
`/private/tmp/ocelli-pass3-wasm.log` and `/private/tmp/ocelli-pass3-core.log`.

## Verified clean

Read the ticket and cleanup paths through both public adapters, commitOwned,
the release export and the raw current-ticket check. The live tests exercise
malformed metadata, held ownership, wrong tickets, write failure and later
successful use. The registered interaction subjects no longer return the
native diagnostic's durations. No baseline or budget changed in this review.

## Defects

None found in the scoped repairs.

## Smells

None recorded.

## Nitpicks

None recorded.

## Limitations and handoff

This is a bounded repair review, not completion of the missing registered
interaction instruments, broader worker integration or final sprint profile.
The story stays in progress. The new canvas resize defect is recorded under
F-X003. New Codex instruction refinements require a separate reviewer.
No completion, commit or verification receipt was created.
