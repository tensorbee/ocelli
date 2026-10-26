# F-101 boundary review, pass 1

**Reviewed**: staged tree `7de677af36b9122e60d2da4700c96b730f422e0a`
**Base**: `6b562be784f4365e7781869b5c511e9e0a1162a8`
**Author / reviewer**: xAI model per operator handoff / Codex
**Result**: 3 defects, 0 smells, 0 nitpicks
**Date**: 2026-10-19

## Defects

### D1, allocation refusal still writes at address zero

**Where**: `packages/core/src/commands.ts:127` and the unchanged consumers in
`packages/core/src/bulk.ts:103`, `:126` and `:171`.

**Contract**: F-101 AC4 and HLD section 17.2 require a valid owned allocation
before constructing and writing a memory view. `Session::alloc` returns 0
when its one bulk slot is occupied or the requested length exceeds the limit.

**Trigger and actual behavior**: Hold one 8-byte allocation, then call
`submitCommands` with one Zoom command against the real wasm module. Its
second alloc returns 0. The write callback still writes the packed bytes at
address 0. The probe observes those bytes change. `apply_commands(0,len)`
also consumes and drops the held slot before refusing. The bulk helpers have
the same unchecked-address assumption, now exposed to this concrete provider.

**Expected and regression**: Allocation refusal must stop before any write
or apply/commit call, with the approved failure result. Exercise zero from
alloc, the boundary limit, an occupied slot, and success after release using
the actual memory adapter. Verify address-zero bytes and held ownership stay
unchanged. Check all affected bulk consumers, not only the new helper.

### D2, native token tests conceal wasm allocation-address reuse

**Where**: `crates/ocelli-wasm/src/session.rs:134`, `:300` and `:309`.

**Contract**: F-101 AC4 requires stale and duplicate commits to be refused.
The native implementation uses a changing counter. Wasm uses only the current
allocation address as its token.

**Trigger and actual behavior**: In the built wasm module allocate 4 bytes,
commit them successfully, then allocate 4 bytes again. Both pointers are
1769480 in this run. A delayed commit using the old pointer is accepted and
consumes the new allocation, returning 1. The old generation is also accepted.
The exact address is incidental, while equality and stale acceptance matter.

**Expected and regression**: A previous allocation identity must not authorize
a newer slot at the same address. Resolve the identity contract explicitly,
using a separately validated lifetime identity if needed. Do not merely use
another native counter in the test or retain all old buffers indefinitely.
Keep the write address distinct from its authorization token if the design
requires both. Record the compatible API decision before changing that wire.
The regression must execute release, same-size reallocation and a delayed old
commit against wasm, with a valid current commit as a positive control.

### D3, public bulk metadata does not match the generated export

**Where**: `packages/core/src/bulk.ts:64`, `:108` and
`crates/ocelli-wasm/src/session.rs:366`.

**Contract**: F-101's binding-compatible FrameMeta decision and bulk consumer
map require one usable call path. `writeFrame` passes `(ptr,len,metaObject)`.
The generated `commit_frame` requires six numeric arguments.

**Trigger and actual behavior**: Passing the actual generated module to the
existing helper throws `expected a number argument, found object` in a dev
build. Calling the numeric export with the same bytes and explicit metadata
returns 1. There is no concrete adapter in the package that joins them.

**Expected and regression**: Preserve the public metadata contract through an
explicit concrete adapter or an approved compatible export change. Handle
the numeric refusal result and ownership cleanup. Run the real public helper
against the built export, including failure, memory growth and one accepted
commit. A fake BulkSink with a matching object signature cannot qualify it.

## Evidence

Executed now on this host:

| Check | Result |
|-------|--------|
| `bin/ocelli.sh test ocelli-wasm --lib` | exit 0, 19 passed |
| `bin/ocelli.sh test ocelli-viewport` | exit 0, 47 passed |
| `npx vitest run packages/core/src` | exit 0, 95 passed across 10 files |
| `wasm-pack build crates/ocelli-wasm --target nodejs --dev --out-dir /private/tmp/ocelli-s16-review-wasm` | exit 0, actual wasm32 artifact |
| `node /private/tmp/ocelli-s16-boundary-probes.mjs` | exit 1, D1/D2/D3 assertions fail, numeric commit positive control passes |

The Node probe transpiles the existing TS helpers without editing source.
Its source and output are `/private/tmp/ocelli-s16-boundary-probes.mjs` and
`/private/tmp/ocelli-s16-boundary-probes.log`. Node was v24.13.0. Build output
is `/private/tmp/ocelli-s16-wasm-build.log`. No GPU is required for these cases.
This dev build is not a release-size measurement or browser-worker test.

Minimal D2 reproduction on that artifact:

```javascript
const m = require('/private/tmp/ocelli-s16-review-wasm/ocelli_wasm.js');
const old = m.alloc(4);
console.log(m.commit_frame(old, 4, 1, 1, 1, 9));
const current = m.alloc(4);
console.log(old, current, m.commit_frame(old, 4, 1, 1, 1, 0));
```

## Verified clean

Parser lengths, ids, finite values and enum checks were inspected. Native
tests cover command-batch rollback and selected VOI refusal. Ring wrap,
drop-new policy, copied event bytes and state offsets remain covered by the
committed suites. No new unsafe memory read or public pixel export was found.
These strengths do not substitute for the failing live target boundary.

## Smells

None separately recorded.

## Nitpicks

None recorded.

## Limitations and handoff

The failures exist on the reviewed index, not its older HEAD. No product fix
was applied. Review session/raster invalidation jointly with F-X003. Browser
memory-growth/RAF integration and working benchmark runners remain unfinished.
The unconditional throw-only runners demonstrate refusal, not timing readiness.
No full sprint gate, oracle, release-size check or independent whole-sprint
approval is claimed. Repair these findings and request a fresh pass.
