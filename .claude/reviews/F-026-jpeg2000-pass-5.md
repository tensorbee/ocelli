# F-026 review, pass 5

**Reviewed**: exact fully staged 137-path tree
`3a5039e0f7f28772ee3f15b78af433359e47376c` against
`35b7ca14afa1091d5668d1a4ccff767f29e5633f`, including all nine batch-4
remediation paths and every earlier F-026 path
**Result**: 1 defect, 0 smells, 0 nitpicks

## Defect

### D1, the accepted timing provenance still describes the old one-call clock

**Where**: `tools/bench/src/runners/decode_transfer_syntax_jpeg2000.mjs:29-38,69-80`
and `ci/bench-baseline.json:199-230`

**What**: the corrected Rust instrument starts one `Instant` before four
consecutive calls and reads it after all four. The emitted run detail and the
accepted baseline still say `std::time::Instant around each Decoder::decode
call`. The parser's iteration refusal likewise calls 31 timing samples a
`31-call runner contract`, even though those samples contain 124 measured
decode calls.

**Why it is wrong**: HLD section 26 requires benchmark evidence to say what was
measured. These strings are part of the run and accepted baseline provenance,
and they now describe a different timing boundary from the executable one.
The plan and LLD correctly distinguish 31 samples, four calls per sample, and
one warm-up call, so a reader comparing the structured record with those
documents gets contradictory answers.

**Evidence**: `decode_jpeg2000.rs:30-37` performs the warm-up before the sample
loop, starts one clock at line 33, executes four calls at lines 34-36, and
divides the batch elapsed time by four at line 37. A fresh release execution
emitted `iterations=31`, `warmup_iterations=1`, and
`decodes_per_sample=4`. The JavaScript wrapper correctly expands that to
`iterations_run=125`, while its `clock` field still claims one clock per call.
The same stale `clock` value is retained in the checked baseline.

**Remediation boundary**: change the emitted and accepted clock provenance to
say that one `Instant` surrounds each four-decode batch and that elapsed time
is normalized by four. Change the parser message from 31 calls to 31 timing
samples. Add a permanent assertion for those provenance fields. Do not change
the timing arithmetic, production subject, calibration series, baseline, or
tolerance.

## Batch-4 remediation proved

- The Rust subject allocates the output and constructs the descriptor and
  decoder before timing. It performs exactly one untimed production decode,
  then records exactly 31 samples. Each sample times four consecutive calls to
  the same production `.91` decoder and divides elapsed milliseconds by four.
  Process startup, fixture setup, decoder construction, and caller allocation
  therefore remain outside every sample. The emitted checksum is 1,563,934.
- The wrapper requires integer values of exactly 31 kept samples, one warm-up,
  and four decodes per sample. It records 125 total decode calls and preserves
  31 as the kept sample count. Its ten focused tests independently refuse
  absent, zero, fractional, and wrong batch counts, along with the previous
  duration, warm-up, range, and checksum errors.
- The temp-only experiment is described identically in the approved plan,
  LLD, and handoff. More warm-ups did not remove the correlated mode, and 63
  or 127 kept one-call samples retained 6.94 and 7.41 per cent upper
  deviations. Moving from one to four calls reduced the measured upper
  deviation from 8.17 to 5.71 per cent. Eight calls improved that by only 0.01
  percentage point, while sixteen reached 4.67 per cent. Four is therefore the
  smallest tested batch with a material improvement while retaining one-call
  units.
- The structured series contains exactly 15 consecutive medians in documented
  run order. Independently sorting them gives `0.6732, 0.6743, 0.6808, 0.6819,
  0.6851, 0.6860, 0.6876, 0.6878, 0.6883, 0.6912, 0.6962, 0.7031, 0.7350,
  0.7361, 0.7426`. The middle value is 0.6878 ms and the exact range is 0.6732
  through 0.7426 ms. The lower and upper deviations are 2.1227 and 7.9674 per
  cent. A 10 per cent band is 0.61902 through 0.75658 ms, leaving 7.8773 and
  2.0326 percentage points of lower and upper headroom.
- The permanent consistency test now requires exactly 15 values and tolerance
  exactly 0.1. It binds the JSON series to the LLD list, recalculates median
  and extrema, checks every value is in band, and requires the exact 10 per
  cent statement in both structured provenance and LLD. The baseline records
  `replaced` as 0.7353 and preserves accepted host, instrument, conditions,
  sample counts, range, checksum, fixture, profile, and exclusions.
- Five fresh same-tree production comparisons measured 0.6825, 0.7020,
  0.7012, 0.7114, and 0.7074 ms. Every command exited 0 and every JPEG 2000
  result was inside the recorded band. A separately rebuilt release subject
  emitted 0.7401 ms and the exact count and checksum fields.

## Earlier remediations and complete-tree evidence

- OpenJPEG 2.5.4 regenerated the scalar-derived style-1 fixture and raw truth
  exactly. The codec suite exited 0 with 56 tests, including seven JPEG 2000
  tests. `.91` accepts the style-1 stream and `.90` refuses it atomically.
  SIZ, COD, QCD, QCC, SOT, EOC, transform, quantization, one legal NULL pad,
  descriptor, stored-domain conversion, and atomic output contracts remain
  covered.
- The vendor tree remains 78 files and 2,044,376 bytes, the exact 74-file
  crate inventory plus four declared additions. Full-byte inventory, both
  manifests, licences, and provenance binding passed 14 pin tests. No vendored
  Rust source contains `unsafe`, and native and wasm codec graphs contain no
  Rayon.
- `bin/ocelli.sh native` exited 0 through all eight steps. Native, plain wasm,
  and SIMD wasm production checks executed under Node. Fresh release modules
  were 145,689 and 145,297 bytes. The separate paired patch evidence remains
  222,226 and 222,166 bytes.
- `gate bench` exited 0 with 12 subjects, three baselines, 27 Python tests, and
  95 Node tests with 94 passing and the one declared browser skip. `gate
  guards` exited 0 after 341 refusal probes drove 32 guards red, 41 accept
  probes stayed green, and 50 controls stayed green. The new four-decode parser
  refusal is included in the 828-site census.
- `gate pins`, `gate unsafe`, `gate provenance`, and `gate deviations` each
  exited 0. The corpus check verified 92 of 92 cases with no missing or
  mismatched files. The full floor, run with a private npm cache, exited 0 with
  all 26 gates green.

## Independent mutation

The fresh arithmetic mutation changed the per-sample divisor from 4.0 to 8.0
without changing the four executed decodes. The production comparison then
measured 0.3488 ms and exited 1, reporting a 49.3 per cent unexplained
improvement. This is distinct from the author's exact-tolerance and series
cardinality mutations and from earlier review mutations. The divisor was
restored to 4.0 and the release executable was rebuilt from the restored
source. `git diff --quiet` then exited 0 before the green gates and this report.
