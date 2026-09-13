# F-026 review, pass 4

**Reviewed**: exact fully staged 136-path tree
`99ab48abb9eeeb57924359a458bc4757c787c9c8` against
`35b7ca14afa1091d5668d1a4ccff767f29e5633f`, including the complete pass-3
benchmark remediation and all earlier F-026 work
**Result**: 2 defects, 0 smells, 0 nitpicks

## Defects

### D1, the recalibrated band is not stable under same-tree comparisons

**Where**: `ci/bench-baseline.json:168-245`,
`docs/lld/benchmarks.md:388-399`, and
`.claude/plans/F-026-design.md:148-153`

**What**: the replacement baseline is arithmetically consistent with its
recorded calibration series, but one of three fresh production comparisons on
the same recorded host exceeded its 10 per cent upper bound. The second run
measured 0.8367 ms against the 0.7353 ms baseline and reported `MOVED` at
13.8 per cent worse.

**Why it is wrong**: the LLD and approved plan say this is the smallest band
covering the observed series with headroom. A same-tree production comparison
outside the band immediately after recalibration shows that the claimed
headroom does not make the comparison reproducible. It can make an unchanged
tree fail the benchmark gate.

**Evidence**: the checked series has exactly 15 values. In sorted order it is
`0.6720, 0.6814, 0.6840, 0.6882, 0.7246, 0.7284, 0.7306, 0.7353, 0.7355,
0.7380, 0.7386, 0.7400, 0.7412, 0.7458, 0.7942`. Its middle value is 0.7353,
and the extrema are 8.6087 per cent below and 8.0103 per cent above it. The
10 per cent interval is 0.66177 through 0.80883 ms, leaving 1.3913 and 1.9897
percentage points of lower and upper headroom. Three independent
`bin/ocelli.sh bench --compare` runs on Node v24.13.0 measured JPEG 2000 at
0.7354 ms, 0.8367 ms, and 0.7161 ms. Their exits were respectively 0, 1, and
0. The red command named only this subject as 13.8 per cent worse.

**Remediation boundary**: diagnose and control the source of the run-to-run
spread, or record a stable calibration population under reproducible
conditions, then derive a truthful baseline and tolerance that survives
repeated comparisons. Do not merely widen the tolerance, weaken symmetric
comparison, or change the production subject.

### D2, the permanent calibration test does not bind the documented sample count or tolerance

**Where**: `tools/bench/tests/decode_jpeg2000_test.mjs:84-118`,
`ci/bench-baseline.json:168-245`, and `docs/lld/benchmarks.md:388-399`

**What**: the consistency test accepts any series of at least 15 values and
derives its band from whatever tolerance is present in JSON. It therefore does
not enforce the exact 15-run series or the exact 10 per cent band claimed by
the plan, LLD, and structured provenance.

**Why it is wrong**: a coordinated record edit can change the measurement
population or tolerance while leaving the prose and provenance claims stale,
yet the permanent evidence remains green. An even-length series also changes
the meaning of `Math.floor(length / 2)` from the conventional median of two
middle values to the upper middle observation.

**Evidence**: a temporary change from tolerance 0.1 to 0.08 drove the named
test red because the calibration range left the band. After restoring it, a
change from 0.1 to 0.2 left the same named test green even though the LLD and
tolerance provenance still said retained 10 per cent. After restoring that,
appending a sixteenth 0.7353 value to both the JSON series and the LLD list also
left the test green while both still described a series of 15. All temporary
edits were removed, and `git diff --quiet` exited 0.

**Remediation boundary**: require exactly 15 values, bind the structured
tolerance to the documented 0.1, and permanently prove those fields against
the prose and provenance. Keep the existing series, median, range, and
inside-band checks. Do not invent measurements or relax coverage.

## Benchmark remediation proved

- The recorded values are in the documented run order and the independently
  recomputed median, extrema, deviations, band, and headroom match the checked
  numbers. The baseline replaces 0.7895 and records the current subject's
  host class, Node, Playwright and Chromium versions, load and memory
  conditions, 32 run and 31 kept iterations, warm-up exclusion, observed raw
  range, checksum, transfer syntax, dimensions, profile, fixture, and measured
  exclusions. `acceptRecord` preserves these fields from the accepted run and
  carries the previous value into `replaced`.
- The permanent consistency test binds the LLD's listed values to the JSON
  series, checks positive finite values, derives the middle observation and
  extrema, checks the recorded range, and verifies every recorded value is in
  the configured band. The two fail-open cases in D2 are the remaining gap.
- The result parser still requires exactly 31 kept iterations and one warm-up,
  a positive finite duration, a two-value finite positive ordered range
  enclosing the median, and a positive safe-integer checksum. Its nine focused
  tests exited 0.

## Earlier remediations and complete-tree evidence

- OpenJPEG 2.5.4 regenerated the style-1 fixture byte for byte. The permanent
  codec tests retain `.91` acceptance and `.90` refusal, with exact style-1
  fixture and raw-reference hashes. QCD, QCC, SIZ, COD, SOT and EOC checks,
  transform and quantization rules, one legal NULL pad, descriptor validation,
  stored-domain conversion, and atomic output remained covered. The codec
  suite exited 0 with 56 tests.
- The vendor tree remains 78 files and 2,044,376 bytes, comprising the exact
  74-file crate inventory and four declared additions. Complete-byte inventory,
  manifest, licence, and provenance binding stayed green. No vendored Rust
  source contains `unsafe`, no native or wasm codec dependency graph contains
  Rayon, and the exact `=0.6.0` excluded path dependency remains enforced.
- `bin/ocelli.sh native` exited 0 through all eight steps and executed native,
  plain wasm, and SIMD wasm JPEG 2000 checks under Node. Fresh release modules
  were 145,689 and 145,297 bytes. The paired dependency-patch evidence remains
  separately documented as 222,226 and 222,166 bytes.
- `bin/ocelli.sh corpus` exited 0 with 92 verified, 0 missing, and 0
  mismatched. `gate pins`, `gate bench`, `gate unsafe`, `gate provenance`,
  `gate deviations`, and `gate guards` each exited 0. The guard catalogue ran
  341 refusal probes across 32 guards, 41 accept probes, and 50 controls.
- The full floor, run with a private npm cache, exited 0 with all 26 gates
  green. Benchmark checks covered 12 subjects and three baselines, 27 Python
  tests, and 94 Node tests with 93 passing and the one declared browser skip.

## Independent mutation

The fresh red mutation changed the configured tolerance from 0.1 to 0.08. The
permanent complete-calibration-series test then refused the recorded maximum
and its focused command exited 1. This differs from the author's baseline-value
mutation and pass 3's QCD entry-count mutation. The exact reverse patch was
applied before any gate.

Two additional adversarial mutations established D2. Tolerance 0.2 kept the
focused test green, and a coordinated sixteenth series value in JSON and the
LLD also kept it green. Both contradicted unchanged exact prose claims. Every
temporary mutation was restored. `git diff --quiet` exited 0 before this report
was written.
