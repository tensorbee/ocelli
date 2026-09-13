# F-026 review, pass 3

**Reviewed**: exact fully staged 135-path tree
`6be0f642e75046b4ffcc7c963f19a2dfbdde9487` against
`35b7ca14afa1091d5668d1a4ccff767f29e5633f`, including every pass-2
remediation
**Result**: 1 defect, 0 smells, 0 nitpicks

## Defect

### D1, the JPEG 2000 baseline excludes its own calibration population

**Where**: `ci/bench-baseline.json:168-206` and
`docs/lld/benchmarks.md:388-399`

**What**: the checked-in JPEG 2000 baseline is 0.7895 ms with a symmetric
10 per cent tolerance. Its accepted interval is therefore 0.71055 through
0.86845 ms. The tolerance provenance and LLD say the fifteen calibration
medians were 0.6435 through 0.6826 ms. Every stated calibration median is
outside the accepted interval on the faster side.

**Why it is wrong**: this repository applies benchmark tolerances on both
sides so a runner that quietly times less work is detected. That mechanism is
correct, but this baseline and its provenance make the normal population used
to justify the tolerance look like an unexplained improvement. A repeat on the
recording host can therefore report `MOVED` without a code or instrument
change. The record does not support the claim that ten per cent covers the
measured spread with headroom.

**Evidence**: direct arithmetic gives `0.7895 * 0.9 = 0.71055`, above the
documented calibration maximum 0.6826. A fresh sequence of fifteen executions
of the production release subject produced medians from 0.6667 through 0.7491
ms. Nine of fifteen were below 0.71055. The full benchmark harness separately
measured the JPEG subject at 0.082 ms and JPEG 2000 at 0.7419 ms and exited 0.
The successful current point does not repair a band that rejects nine nearby
runs and all fifteen values cited as its derivation.

**Remediation boundary**: recalibrate and accept the JPEG 2000 baseline and
tolerance from one recorded stable series on the named host and instrument,
then make the baseline provenance and LLD cite that same series. Do not widen a
tolerance merely to absorb the contradiction, change the production runner,
or alter the comparison's symmetric-bound rule.

## Pass-2 remediations proved

- `opj_decompress -h` identified OpenJPEG 2.5.4. Running
  `generate_jpeg2000_style1.py` without `--write` exited 0 and reproduced the
  tracked bytes exactly. The source, codestream, and independent raw-reference
  SHA-256 values were respectively
  `c1b3005328ac117e804f779b3c9f1d39add889abef950192f11d95d9e74a6954`,
  `78eb076398c0be23f9df5903d3915caf54e16571f741567bb227b842d775df7c`,
  and `78aacbc3fb34efb8ffa5467b931291ec2bdf5e19564fc45fe97b5affbc893dc6`.
  The 88-byte stream has SOC and terminal EOC, SIZ length 41 with one
  component, COD length 12 with MCT 0, five decomposition levels and reversible
  transform 1, and exactly one QCD of length 5, style 1 and one two-byte step
  entry. The permanent test decoded the OpenJPEG reference through `.91` and
  atomically refused the same stream through `.90`.
- `PATCH-PROVENANCE.md` hashes to
  `3161785b1cbf76437756c68d13ba6b5d53dfc26f88f0dd285d2af0e395aedeff`,
  exactly the constant outside the vendor tree. The unit test that appends a
  false source-change claim and the coordinated source plus inventory test
  both passed. The `pins.ritk-provenance-extra` and inventory-root catalogue
  probes changed their intended inputs and drove the pins check red for the
  declared reason.
- The JPEG 2000 result parser requires exactly 31 kept iterations and one
  warm-up, a positive finite duration, a two-value finite positive ordered
  range enclosing the median, and a positive safe-integer checksum. Its eight
  tests independently refused missing warm-up, wrong counts, missing and
  malformed ranges, non-positive endpoints, reversed endpoints, ranges that
  exclude the median, absent duration and absent checksum.

## Complete-tree evidence

- The JPEG 2000 inspector remains bounded to the main header through first
  SOT. It requires one SIZ, COD, and QCD, validates each QCD style's exact
  entry count from the COD decomposition level, refuses QCC, and requires a
  terminal EOC with only the one legal DICOM NULL pad case. `.90` requires
  reversible transform and no quantization. `.91` permits both Part 1
  transforms and all three legal QCD styles.
- Descriptor checks retain OB, one monochrome component, 8-bit or 16-bit
  containers, and exact dimensions, precision, signedness and MCT. The owned
  `f32` result must have exact sample count and every sample must be finite,
  integral and inside its signed or unsigned stored range. Conversion builds
  little-endian bytes away from the caller and copies only after all checks.
  The full codec suite exited 0 with 56 tests, including seven JPEG 2000
  integration cases and four library tests.
- The vendor tree has 78 files and 2,044,376 bytes, the exact 74-file package
  inventory plus four declared additions. Inventory, both patched manifests,
  both licences and patch provenance are complete-byte bound. No vendored Rust
  source contains `unsafe`. `gate pins` exited 0 and both native and wasm codec
  graphs contained no Rayon. Workspace exclusion and the exact `=0.6.0` path
  dependency remain enforced.
- `bin/ocelli.sh native` exited 0 through all eight steps. The production exact
  and lossy checks executed natively and from plain and SIMD wasm under Node.
  The fresh release modules were 145,689 and 145,297 bytes. The LLD's separate
  paired dependency-patch size evidence remains 222,226 and 222,166 bytes.
- `bin/ocelli.sh gate bench` exited 0 with 12 subjects, three baselines, 27
  Python tests, and 93 Node tests with the one declared browser test skipped.
  The lifecycle contract still permits measurable `in-progress` and `done`
  stories, requires a runner for `done`, and refuses the other statuses. The
  actual harness run exited 0 with four measured subjects, including the
  standing JPEG decode and the new JPEG 2000 decode. F-004's startup runner
  continued to call the production probe.
- `bin/ocelli.sh gate guards` exited 0 after 341 refusal probes drove 32 guards
  red, 41 accept probes stayed green, and 50 controls stayed green. `corpus`
  exited 0 with 92 verified, 0 missing, and 0 mismatched. The focused `unsafe`,
  `provenance`, `content`, `prose`, and `deviations` gates each exited 0.
- The complete floor was run with a private npm cache and exited 0 with all 26
  gates green, including formatting, clippy, workspace tests, packages,
  benchmark checks, guard probes and corpus-tooling tests. The first sandboxed
  benchmark invocation exited 1 only because `uv_uptime` returned `EPERM`.
  Re-running with host-read permission exited 0.

## Independent mutation

The fresh mutation changed scalar-derived QCD's required entry-byte count from
2 to 3 in `validate_qcd`. The permanent
`general_accepts_scalar_derived_qcd_but_lossless_refuses_it` test then returned
`InvalidCodestream`, and its focused command exited 101. This differs from the
author's mutation of the accepted style set. The constant was restored with an
exact reverse patch. `git diff --quiet` then exited 0, and both unstaged and
staged diff checks exited 0 before this report was written.
