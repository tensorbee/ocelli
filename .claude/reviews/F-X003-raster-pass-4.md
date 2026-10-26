# F-X003 resize repair review, pass 4

**Reviewed**: staged tree `9ae40681bae54a69a21705d98a40a6337adce74d`
**Previous**: `f6057868244a71679ebfd95a17e440084699aa76`
**Author / reviewer**: implementing xAI model / Codex
**Result**: 0 defects, 0 smells, 0 nitpicks in the resize repair scope
**Date**: 2026-10-19

## Finding disposition

Pass 3 D8 is repaired. The public binding validates the requested extent and
asks the session to accept it before assigning canvas dimensions. When the
bitmap and retained publication already match the device extent, it avoids
the destructive width and height setters. The permanent worker test covers
identical and rejected resizes alongside changed extent and retained storage.

Rebuilt the dev wasm module from the reviewed source. Replayed the independent
`/private/tmp/ocelli-pass3-resize.mjs` probe that originally failed on the
previous tree. Both independent workers started from opaque white. Identical
resize preserved `[255,255,255,255]`, 1 by 1 extent, clear refining and no new
event. Zero width was refused with the same visible state preserved. A later
publication preserved that state too. Exit 0, zero failed preservation checks.
The previous RED observation remains recorded in pass 3, rather than being
replaced by this GREEN result.

## Checks executed now

- `wasm-pack build crates/ocelli-wasm --dev --target web --out-dir target/boundary-wasm`: exit 0.
- `npx vitest run --config vitest.boundary.config.ts`: exit 0, 5 passed.
- `node packages/core/cpu-canvas.mjs`: exit 0 in real headless Chromium with GPU disabled.
- `node /private/tmp/ocelli-pass3-resize.mjs`: exit 0 under the same browser conditions.

Logs are `/private/tmp/ocelli-pass4-build.log`,
`/private/tmp/ocelli-pass4-boundary.log`, `/private/tmp/ocelli-pass4-worker.log`
and `/private/tmp/ocelli-pass4-resize.log`. Temporary logs are local supporting
evidence. The permanent worker selector and the observations above are the
retained reproduction route.

The permanent worker also confirmed failed-blit retry at generation 3, newer
generation 5, and unchanged ImageData construction count across identical and
rejected resizes. Actual changed extent added one ImageData. These constructor
observations do not prove absence of every possible allocation.

## Instruction assessment

The seven canonical instruction refinements and six generated adapter changes
remain outside this staged tree. Their independent review is
`S16-instruction-refinement-pass-2.md`. The current design, implementation,
microscope and verification instructions already require external commit
points, repeated and rejected operations, real canvas observations and explicit
wasm/browser selectors. No additional instruction edit is justified by this
bounded repair pass. Preserve the reviewed guidance and include it in a later
explicit integration union.

## Next implementation handoff

Continue F-X003's missing browser colour and reduced-resolution integration
before claiming product completion. Read its approved design and local progress
handoff first. Map each remaining acceptance row to its production caller,
artifact and permanent selector. Resolve a material design decision through
the existing design process before implementing that decision.

Exercise supported colour through the real wasm worker publication path with
synthetic independent expected RGBA. Exercise reduced-resolution interaction,
then settlement to the latest full-resolution generation. Include failure,
retry and repeated resize controls. Retain the monochrome regressions and the
boundary suite. Do not introduce a shell pixel getter or change a tolerance.

Separately identify the still-missing applicable-corpus candidate producer
and real interaction instruments. Missing implementation is actionable work.
The unavailable representative GPU-less host is an external acceptance
dependency. Keep those dispositions separate and do not turn refusal into a
sample. AC5 and AC6 remain open.

Return the next staged product tree and exact rebuilt-artifact evidence for
independent review. Preserve unrelated unstaged instruction refinements.
Before eventual integrated verification, explicitly assemble the reviewed
union, invalidate stale verification and run the canonical sprint profile
plus affected standalone target checks. A whole-sprint independent review is
still required on that union.

## Limits

This is a scoped repair review, not a whole-feature or whole-sprint approval.
F-048, F-101 and F-X003 remain in progress. No new corpus, representative-host,
release-size, GPU tier or rendered fractional-DPR evidence is claimed. No
completion, commit, push, tag or sprint close is authorized by this report.
