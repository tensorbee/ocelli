# F-024 JPEG review, pass 6

**Reviewed**: full staged working tree on `work/f-024-codex` at base
`586e503`, all five earlier review reports, the guard-catalogue and benchmark
runner remediation, and the approved plan, HLD, LLD, and progress record
**Result**: 0 defects, 0 smells, 0 nitpicks

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Guard-catalogue remediation proof

- The five catalogue builders each replace one exact production refusal in
  `tools/bench/src/runners/decode_frame.mjs`. An independent count found each
  replacement target exactly once. `Sandbox.substitute` rejects an absent
  target and replaces only the first occurrence, so these mutations cannot
  pass by editing no site or a different duplicate site.
- `bench.decode-frame.duration` disables only the positive finite duration
  condition. Its malformed record supplies `value: 0` and valid values for the
  other required fields. The mutation therefore reaches the unique
  `duration guard accepted a non-positive measurement` failure.
- `bench.decode-frame.iterations` disables only the positive integer iteration
  condition. Its malformed record supplies `iterations: 0` and valid values
  elsewhere. The mutation reaches the unique
  `iteration guard accepted an empty measurement` failure.
- `bench.decode-frame.range` disables only the two-element finite range
  condition. Its malformed record supplies one range element and valid values
  elsewhere. The mutation reaches the unique
  `range guard accepted incomplete evidence` failure.
- `bench.decode-frame.checksum` disables only the positive safe-integer
  checksum condition. Its malformed record supplies `checksum: 0` and valid
  values elsewhere. The mutation reaches the unique
  `checksum guard accepted absent output evidence` failure.
- `bench.decode-frame.release-binary` replaces only
  `await access(binary)` with an early return. Its invoke supplies a dedicated
  absent path, so the mutation reaches the unique
  `release guard accepted an absent executable` failure without relying on
  the repository's release artefact state.
- Running only these five probes exited 0. All five refusal probes drove their
  declared guard red, all five distinct unmutated controls were green, and no
  known defect remained open. Listing the same selection showed five level-1
  floor refusal probes. The runner unit suite independently passed its four
  tests on the unmutated implementation.
- `python3 scripts/guard_census.py --check-runbook` exited 0 with all 795
  refusals in 66 files claimed and the generated runbook table current. The
  five new rows accurately name their mutation, trigger, failure phrase, and
  floor profile.
- The complete `guards` gate exited 0. It drove 335 refusal probes red for
  their declared reasons, kept 41 accept probes green, kept 50 controls green,
  and reported no open known defect. Its 56, 6, 4, and 76 test groups all
  passed.

## JPEG and benchmark recheck

- The complete current tree retains the earlier review remediations. JPEG
  Extended `.51` accepts process 2 at eight bits and process 4 at twelve bits.
  Process 14 and SV1 keep their distinct predictor requirements, all four UIDs
  are registered atomically, and output Photometric Interpretation remains a
  separate result property.
- The odd 93-byte lossless codestream accepts exactly one necessary trailing
  NULL byte at the F-019 frame boundary. The even 936-byte baseline codestream
  rejects an unnecessary trailing NULL as `TrailingData`. Both refusal paths
  preserve the caller buffer.
- Header, process, precision, component, dimensions, output length,
  truncation, trailing data, dependency layout, and dependency failure checks
  remain before the caller-buffer copy. The D-21 allocation and input-ownership
  prose still agrees with the implementation.
- The release benchmark continues to time 31 individual decode calls after
  one warm-up. Process startup, fixture setup, decoder construction, and output
  allocation remain outside the timer. The runner requires the fixed release
  executable for `--no-build` and validates the duration, kept iterations,
  observed range, and output checksum before constructing its benchmark
  record.
- No repository `unsafe`, `wasm-bindgen`, patient data, render-loop work,
  tier-specific arithmetic, new trait, or new generic was introduced.

## Fresh mutation and verification evidence

- A fresh arithmetic mutation changed the lossless scan aggregate from
  `predictor == 1` to `predictor == 2`. The exact
  `process_14_selection_value_1_preserves_the_same_exact_samples` regression
  exited 101 with `FrameMismatch`. After restoring the production expression,
  the same exact regression exited 0 and no unstaged implementation diff
  remained.
- `bin/ocelli.sh test ocelli-codec` exited 0. It passed 2 unit tests, 10 JPEG
  tests, and 14 registry tests, 26 tests total.
- `bin/ocelli.sh check ocelli-codec`, `bin/ocelli.sh clippy ocelli-codec`, and
  `bin/ocelli.sh cargo check -p ocelli-codec --all-targets --target
  wasm32-unknown-unknown` each exited 0.
- `bin/ocelli.sh gate bench` exited 0. Its 25 Python tests passed. Its Node
  suites reported 82 passes, one declared browser-only skip, and no failure.
- `bin/ocelli.sh gate native` exited 0. Its four proofs covered native entry
  points, shared-crate wasm compilation, native all-target checks, and equal
  resolution of 17 directly declared dependencies across the native and wasm
  targets.
- `bin/ocelli.sh gate packages` exited 0. Eight test files passed 79 tests, and
  both packages passed version, tarball, export, and outside-workspace consumer
  verification.
- `bin/ocelli.sh gate fmt unsafe provenance prose content pins deviations
  bindgen errors` exited 0 with all nine gates green. Unsafe checked 73 files
  with two permitted, provenance checked 597 files, prose checked 251 files,
  and the error registry's 15 tests passed.
- `git diff --cached --check` and `git diff --check` exited 0 before this report
  was added. This review changed no implementation or canonical sprint state.
