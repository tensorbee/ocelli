# F-X008 review, pass 1

**Reviewed**: staged working tree against `deb7a3e`
**Result**: 1 defect, 0 smells, 0 nitpicks

## Defects

### D1, the guard sandbox accepts symlinks that escape the sandbox

**Where**: `scripts/guards/sandbox.py`, `copy_tracked_path`

**What**: Every symlink is recreated verbatim. The helper does not require a
relative target and does not prove the resolved target stays inside the source
repository and copied sandbox. A tracked absolute link, or a relative link
with enough `..` components, therefore points a guard probe outside its
temporary repository.

**Why it is wrong**: The story needs three relative package-local licence
links whose targets remain inside the repository. It does not authorize the
guard harness to follow arbitrary host paths. The sandbox is the safety
boundary for mutation probes, so an unrepresentable or escaping link must be
refused rather than faithfully recreated as an escape.

**Evidence**: Code inspection shows the symlink arm calls
`destination.symlink_to(source.readlink())` without examining the target. The
new focused tests cover a safe `../../LICENSE-MIT` link and a directory, but no
absolute or relative escape. Add source-root and destination-root containment
proofs to the production copy path, plus tests for both escape forms. Preserve
the three safe relative licence links and keep other unsupported shapes
refused.

## Smells

None.

## Nitpicks

None.

## Verified clean

- The two operational parity consumers name 5.8.2 and D-11, and their test
  reads both concrete files rather than scanning the repository loosely.
- The Codex parity adapter is regenerated from the canonical command.
- The three crate-local licence links are relative and tracked as symlinks.
- The wasm package gate requires both MIT and Apache grants and byte-compares
  them with repository originals. Missing and changed grants are unit-tested.
- The named package-licence and stale-target catalogue probes isolate their
  intended refusal conditions.
- The generated runbook, probe budget and LLDs match the expanded guard
  surface. No runtime, pixel, LUT, tolerance, unsafe, or wasm-boundary change
  was introduced.
