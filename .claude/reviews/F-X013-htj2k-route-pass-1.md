# F-X013 review, pass 1

**Reviewed**: staged working tree against `021d892`
**Result**: 1 defect, 0 smells, 0 nitpicks

## Defects

### D1, the irreversible syntax can decode the wrong codestream and pass

**Where**: `tools/spikes/x013-htj2k-route/run.mjs`, irreversible `.203`
handling

**What**: The driver prints the `.203` comparison against `ojph_expand`, but
does not make any property of that comparison part of its verdict. It checks
only that native and wasm produce the same bytes. All three builds can decode
the wrong embedded codestream identically and the driver still exits 0.

**Why it is wrong**: The recommendation says F3 decodes all three HTJ2K
transfer syntaxes and records the `.203` result as 41 one-level differences.
The executable evidence does not protect that claim. Cross-target identity is
not conformance when every target is fed the same wrong input.

**Evidence**: Changed `CASE_LOSSY => Some(LOSSY)` to
`CASE_LOSSY => Some(LOSSLESS)`, rebuilt native, plain wasm and simd128 wasm,
then ran `node tools/spikes/x013-htj2k-route/run.mjs`. The `.203` candidate
digest changed to the lossless ramp, 5,377 samples differed from OpenJPH with
maximum absolute difference 41, and the driver still printed
`0 failing comparison(s)` at exit 0. The mutation was reverted. Bind the
irreversible row to its measured candidate result, for example with its pinned
fixture digest and the recorded divergence summary, then show this exact
wrong-codestream mutation makes the driver fail.

## Smells

None.

## Nitpicks

None.

## Verified clean

- Source-policy assessment precedes implementation inspection and records the
  archive checksum, absent packaged notice, registry licence metadata,
  publisher, VCS metadata and production redistribution condition.
- `.201` and `.202` are independently anchored to the synthetic ramp and
  compared exactly on native, wasm and simd128 wasm.
- `.203` is correctly described as a measured divergence between two OpenJPH
  lineages, not independent confirmation or bit-exact reproducibility.
- The spike is isolated from the workspace and adds no production decoder,
  wasm-bindgen use, repository unsafe code, pixel boundary, LUT logic, or
  tolerance change.
- Allocation, dependency, toolchain, binary-size, maintenance and unsafe-audit
  costs are recorded with the exact follow-up owned by E2.6.
