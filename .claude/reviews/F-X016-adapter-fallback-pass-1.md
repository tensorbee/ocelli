# F-X016 review, pass 1

**Reviewed**: working tree against `1e18647`
**Result**: 1 defect, 0 smells, 0 nitpicks

## Defects

### D1, the fallback loop is not exercised by any test

**Where**: `crates/ocelli-render/src/probe.rs`, `measure_candidates`

**What**: The story's central production behaviour is continuing after a
failed `request_device`, but every new test constructs `ProbeOutcome` directly
or tests the separate ranking function. Nothing executes the error arm and
observes a later success.

**Why it is wrong**: F-X016 exists because the previous implementation stopped
after the preferred adapter failed. The approved plan requires the next adapter
to be attempted. A test that does not fail when this loop stops early cannot
prove that requirement and HLD 27.3 requires the mutation to go red.

**Evidence**: Added `break` immediately after the first failed attempt and ran
`bin/ocelli.sh test ocelli-render`. All 67 non-GPU unit tests and the totality
property still passed. The mutation was reverted. Refactor the attempt loop
behind a production-used test seam, with the concrete wgpu path and at least
one deterministic test instantiation, then prove first failure followed by
second success and all-failed exhaustion.

## Smells

None.

## Nitpicks

None.

## Verified clean

- Candidate ordering is A before B, then the existing device-type rank, then
  enumeration order for exact ties.
- `Backend::Noop` is excluded.
- Classification and override construction read only the successfully opened
  adapter. Failed A evidence cannot construct tier A after B opens.
- Failed adapter identity and diagnostic text survive into evidence.
- Allocations occur during startup probing only. No render-loop allocation,
  unsafe code, wasm boundary change, pixel transfer, or LUT duplication was
  added.
- Native public-API fixtures were updated consistently with the new outcome
  type.
