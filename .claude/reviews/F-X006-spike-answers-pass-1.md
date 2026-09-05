# F-X006 review, pass 1

**Reviewed**: working tree on `work/f-x006-claude`, staged diff against `d74ad3a`
**Result**: 0 defects, 0 smells, 2 nitpicks

This is a self-review by the implementing agent and is recorded as such.
`/microscope` requires the reviewer to be independent of the author, so this
pass does not satisfy that requirement and an independent pass is still owed.
What it does record is which claims were executed rather than read.

## Defects

None outstanding. Five were found during the pass and fixed before it closed,
and they are listed because a fixed defect is evidence about where this work is
weak.

### Fixed, F1, a numeric claim taken from truncated output
**Where**: `docs/spikes/A1-htj2k-openjp2.md`, the undefined-symbol counts.
**What**: the record said 5 `malloc`, 6 `calloc`, 4 `realloc`, 5 `free`.
**Why it was wrong**: `rust-lld` stops at 20 errors and prints
`too many errors emitted, stopping now`, so the counted log was truncated.
**Evidence**: re-run with `-C link-arg=--error-limit=0` gives 432 errors, 279
`free`, 60 `malloc`, 51 `calloc`, 41 `realloc`, 1 `strcpy`, and no truncation
line. The file now carries those figures and names the flag.

### Fixed, F2, a wrong dependency count
**Where**: `docs/spikes/A2-jpeg-ls.md`, the maintenance table.
**What**: `dicom-toolkit-codec` listed as 8 direct dependencies "including
`inventory`, `uuid`, `sha2`, `tracing`".
**Why it was wrong**: its `Cargo.toml.orig` declares 9, and `uuid` and `sha2`
are transitive through `dicom-toolkit-core` rather than direct.
**Evidence**: the manifest, and `cargo tree --no-default-features --features
dicom-toolkit`.

### Fixed, F3, an unsourced provenance claim
**Where**: `docs/spikes/A1-htj2k-openjp2.md`, "openjp2 is a C2Rust port".
**What**: asserted from the shape of the code rather than from a source.
**Evidence**: the upstream README carries the heading "## C2Rust port" and the
sentence "An experimental Rust port is being developed". Both are now quoted.

### Fixed, F4, a stale header comment
**Where**: `tools/spikes/a1-htj2k/run.mjs`.
**What**: "Drives four decodes of each of the three HTJ2K corpus rows" after
two Part 1 control rows were added.
**Evidence**: the file drives five rows. Corrected.

### Fixed, F5, an incomplete claim about wasm-bindgen
**Where**: `docs/spikes/A1-htj2k-openjp2.md`.
**What**: "there is no `wasm-bindgen` here to find in any case", written before
the A2 lockfile was audited.
**Evidence**: `grep -c wasm-bindgen tools/spikes/a1-htj2k/Cargo.lock` is 0, and
`tools/spikes/a2-jpeg-ls/Cargo.lock` carries `wasm-bindgen` 0.2.128 as an
unactivated optional entry under `uuid`. Both files now say so, and A2 records
it as a further reason to reject that candidate, because the documented remedy
for its wasm failure is `uuid`'s `js` feature and that feature is what pulls
`wasm-bindgen` in.

## Smells

None outstanding. Two were fixed during the pass.

- `tools/spikes/a2-jpeg-ls/run.mjs` had
  `const suffix = name === "jpegls_lossless" ? ".jls" : ".jls"`, a ternary whose
  branches are identical. Removed.
- The same file read `out_ptr()` without refusing zero, where the A1 driver
  refuses it. Made consistent.

## Nitpicks

- `ERR_SAMPLE_RANGE`, `ERR_SAMPLE_NOT_INTEGRAL`, `ERR_BYTE_COUNT` and
  `ERR_COMPONENT_COUNT` are defensive and were never returned by any run. They
  are shape assertions rather than expected paths, and the alternative to each
  is a silent total mismatch that would read as a decoder defect.
- `wasm_libc.rs`'s `realloc` branch for a pointer not in its table was never
  taken. It returns null rather than guessing, which is the only honest answer.

## Verified clean

**Arithmetic and casts.** `grep -rn " as "` over both harnesses' `src/`:
**the A1 harness has no `as` cast at all**, which is better than the plan's
budget of one, because `out_ptr` uses `<*const u8>::addr()` and
`u32::try_from`. The A2 harness has exactly one, `Ok(truncated as u16)` in
`f32_sample_to_u16`, guarded on the two preceding lines by a range check and an
integrality test, and it exists only because `ritk-codecs` returns `f32`. It is
named in the answer file so a human can find it under HLD 27.3. No sample is
scaled, shifted or rounded anywhere in either harness: `openjp2` and
`pure_jpegls` samples are converted by `u16::try_from`, which refuses rather
than truncating, and the native binary prints the observed min and max on every
run so the refusal can be shown never to have been close.

**Would the test fail if the code were wrong.** The comparator was run green,
then mutated twice in separate commands and observed red each time, then
reverted and re-run green. The mutations were the difference-scan condition
inverted, and the PGM big-endian byte swap removed. Exit codes 0, 1, 1, 0.

**The new guard fires.** `scripts/staged_content_check.py` gained a
`tools/spikes/out/` refusal. Observed: clean tree exit 0, then
`git add -f tools/spikes/out/htj2k_lossless.j2c` gives exit 1 with the new
message, then unstaged gives exit 0. It was tested by making the thing it
protects happen, not by reading it.

**Claims executed rather than read.** The corpus manifest line numbers, the
`ci/wasm-size-budget.json` figure, the `@cornerstonejs/codec-charls` 1.2.5 and
`@cornerstonejs/codec-openjph` 2.4.9 pins in `tools/oracle/package.json`, the
two `scripts/tests/test_corpus_synth.py` test names, every repository licence
file, every crates.io publish date and download count, the
`dicom-transfer-syntax-registry` 0.10 feature map, and every wasm module size
in the A2 size table.

**Full reproduction from clean.** `tools/spikes/out/` and both `target/`
directories were deleted and the documented command sequences re-run. Every
digest, every size and both summary lines came back identical.

**Boundary.** No `wasm-bindgen` in either harness's source or manifest, and
`gate bindgen` green. No pixels cross a worker to main-thread boundary, because
nothing here has a worker. Views over wasm linear memory are built in `.mjs`
drivers only, immediately after the `out_ptr()` that returns the offset, copied
out with `.slice()` and released, which is HLD 17.2's actual requirement. The
ESLint `NO_CACHED_WASM_VIEW` rule is scoped to `**/*.{ts,tsx}` and its
allowance was NOT widened. `eslint.config.js` gained a globals block for
`tools/spikes/**/*.mjs` so `no-undef` still catches a typo, and that block
states the above rather than switching a rule off.

**unsafe.** `grep -rn unsafe` over both harnesses returns only prose in
comments explaining why there is none. `gate unsafe` green over 35 files with 2
permitted. The C allocator shim, which is where the pressure was, is written
with owned `Box<[u64]>` blocks in a side table so `realloc` copies between two
owned slices and no raw pointer is ever dereferenced.

**Structure.** No new trait, generic or `Box<dyn>`. Nothing under `crates/` or
`tools/oracle/` is touched, and the comparator's own header forbids importing
it from either.

**Gates.** `bin/ocelli.sh gate --floor`, all 21 green, after `npm ci` so that
`lint` ran rather than skipping.
