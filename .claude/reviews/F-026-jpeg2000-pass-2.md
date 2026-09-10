# F-026 review, pass 2

**Reviewed**: fully staged 131-path working tree against
`35b7ca14afa1091d5668d1a4ccff767f29e5633f`, including every pass-1
remediation
**Result**: 3 defects, 0 smells, 0 nitpicks

## Defects

### D1, the general decoder's scalar-derived QCD support has no permanent evidence

**Where**: `crates/ocelli-codec/tests/jpeg2000.rs:43-135` and
`crates/ocelli-codec/src/jpeg2000.rs:227-250`

**What**: the implementation correctly accepts all three JPEG 2000 Part 1 QCD
styles, but the permanent fixtures exercise only no quantization style 0 and
scalar-expounded style 2. Nothing exercises scalar-derived style 1. Replacing
`matches!(style, 0..=2)` with `matches!(style, 0 | 2)` leaves the entire codec
suite green.

**Why it is wrong**: the approved plan and `docs/lld/codecs.md` promise that
the general `.91` adapter accepts any valid Part 1 QCD style. A regression that
rejects one of those three styles is an unsupported valid DICOM codestream,
and the claimed fixture evidence does not detect it.

**Evidence**: a temporary probe converted the tracked scalar-expounded fixture
to a structurally valid scalar-derived QCD by changing `Sqcd` to style 1,
setting `Lqcd` to 5, and retaining its one two-byte step-size entry. The
production general decoder accepted and decoded it at exit 0. The probe was
removed. The independent mutation that rejected style 1 then left
`bin/ocelli.sh test ocelli-codec` at exit 0, including all 6 JPEG 2000 tests.
The mutation was restored completely.

**Remediation boundary**: add a permanent style-1 `.91` fixture or a
spec-derived fixture transformation that fixes its exact QCD length and proves
decode acceptance. Do not change the current decoder policy.

### D2, the patch provenance is not bound to the exact text the plan requires

**Where**: `scripts/pin_and_size_check.py:92-109` and
`scripts/pin_and_size_check.py:282-286`

**What**: inventory, both patched manifests, and both licence texts now have
external complete-byte digests. `PATCH-PROVENANCE.md` does not. The guard only
searches its mutable text for the archive digest, VCS revision, and the words
`MIT alternative`, so any other edit is accepted.

**Why it is wrong**: approved approach step 2 explicitly requires exact
licence and provenance hashes. The patch record is the assertion that no Rust
source changed and that the two manifest edits are the complete local patch.
A substring check does not bind that record to the reviewed text.

**Evidence**: in an isolated vendor copy, the probe appended the false sentence
`The Rust source was changed locally.` while retaining all three searched
substrings. `check_ritk_vendor` returned `[]`, and the command used exit 97 as
the fail-open sentinel. Existing unit tests and the three new catalogue probes
cover coordinated source plus inventory tamper and both manifest additions,
but no probe mutates provenance while preserving the searched substrings.

**Remediation boundary**: add an external exact digest for
`PATCH-PROVENANCE.md`, a unit regression for an otherwise well-formed edit,
and a catalogue probe for the new refusal.

### D3, the JPEG 2000 benchmark parser accepts incomplete and impossible ranges

**Where**:
`tools/bench/src/runners/decode_transfer_syntax_jpeg2000.mjs:21-45` and
`tools/bench/tests/decode_jpeg2000_test.mjs:6-13`

**What**: `parseJpeg2000` does not validate `warmup_iterations`, although the
runner adds it to `iterations_run` and claims one discarded warm-up. Its range
check requires only two finite values. Negative and reversed ranges pass.

**Why it is wrong**: the guard catalogue says this runner refuses results
without positive finite duration, iteration, range, and checksum evidence.
The benchmark record is the evidence for what was timed. Missing warm-up data
produces a non-numeric total, while a negative or reversed elapsed-time range
is impossible and should not be published as a measurement.

**Evidence**: a direct read-only probe passed all three objects through the
production parser at exit 0: one omitted `warmup_iterations`, one carried
`range_ms: [-2, -1]`, and one carried `range_ms: [0.6, 0.4]`. The existing
negative test combines zero value, zero iterations, an empty range, and zero
checksum, so every one of these independent routes remains green.

**Remediation boundary**: require the recorded one warm-up iteration, positive
ordered range endpoints, and a measured value inside that range. Add separate
negative tests so each condition carries its own evidence.

## Smells

None.

## Nitpicks

None.

## Pass-1 remediations proved

- The common main-header inspector reads only through the first SOT, requires
  exactly one main-header QCD, validates styles 0, 1, and 2 against the COD
  decomposition count, and rejects QCC. `.90` separately requires reversible
  COD transform 1 and QCD style 0. A temporary `.91` probe proved missing QCD,
  duplicate QCD, and QCC all preserve caller output and return their declared
  errors. It was removed completely.
- The quantized reversible fixture is 118 bytes and carries COD transform 1,
  QCD style 2, five decomposition levels, the exact 32-byte expounded payload,
  and terminal EOC. `.90` refuses it atomically while `.91` decodes the
  constant 128 frame.
- `PACKAGE-INVENTORY.sha256` has 74 rows and digest
  `066d80ffd6b74bd9410df63f1d694c209431be8e3efde37b7c427f71da0cb351`
  rooted in `scripts/pin_and_size_check.py`. The coordinated source plus
  inventory unit test rejects the former pass-1 escape.
- Removing only `default-features = false` from each current manifest in
  memory reproduces its recorded published-package digest exactly. The current
  complete patched-manifest digests match both external constants. Independent
  unit tests and catalogue probes reject an unrelated addition to each file.
- The permanent JPEG 2000 tests now exercise encapsulated OB, a two-component
  SIZ, invalid COD transform 2, wrong caller output length, fractional and
  out-of-range decoded samples, and wrong decoded sample count. Every refusal
  checks the caller sentinel or reaches the shared allocate-then-copy path.
- The plan's fixture row now cites DICOM PS3.5 A.4.4 and PS3.3 C.7.6.3. Both
  conformance rows name the existing `tests/jpeg2000.rs` and the wasm runner.

## Independent mutation and gate evidence

- Fresh arithmetic mutation changed the QCD subband count from
  `3 * decomposition_levels + 1` to `2 * decomposition_levels + 1`.
  `bin/ocelli.sh test ocelli-codec --test jpeg2000
  lossless_decodes_hand_computed_eight_and_sixteen_bit_signed_domains --
  --nocapture` exited 101 with `InvalidCodestream`. The mutation was restored.
- `bin/ocelli.sh test ocelli-codec --test jpeg2000` exited 0 with all 6 tests.
  `python3 -m unittest scripts.tests.test_pin_and_size_check` exited 0 with all
  13 tests.
- `bin/ocelli.sh native` exited 0 through native execution and actual plain and
  SIMD wasm execution under Node. The release modules measured 145,689 and
  145,297 bytes respectively.
- `bin/ocelli.sh gate pins` exited 0. `bin/ocelli.sh gate guards` exited 0
  after 340 refusal probes drove their guards red, 41 accept probes stayed
  green, and 50 controls stayed green. The inventory-root and both manifest
  probes mutated the intended files and emitted their declared reasons.
- `bin/ocelli.sh gate bench` was green inside the focused gate run with 12
  subjects, 3 baseline entries, 27 Python tests, and 87 Node tests where the
  one declared browser test was skipped. D3 is an uncovered parser route, not
  a currently red benchmark.
- `bin/ocelli.sh corpus` reported 92 verified, 0 missing, and 0 mismatched.
- The first `bin/ocelli.sh gate --floor` exited 1 only because both `npm pack`
  calls encountered the existing root-owned user npm cache. The package gate
  then exited 0 with a private `/private/tmp` cache. Re-running the complete
  floor with that private cache exited 0 with all 26 gates green.
- The production `.90` exact and `.91` divergence proof remains common across
  native, plain wasm, and SIMD wasm. The fixed lossy distribution remains 418
  of 6,144 samples against uncompressed truth, signed sum 30, maximum 1, and no
  difference above 1.
- The vendor tree still has 78 files, consisting of the exact 74-file package
  plus four declared additions. No vendored Rust file contains `unsafe`, both
  native and wasm codec graphs contain no Rayon, the workspace exclusion and
  exact path pin hold, and the source-policy, content, unsafe, prose, and
  provenance gates are green in the floor.
- F-004's runner still invokes its real ignored release probe and reports
  failure rather than inventing a fill rate when no adapter exists. The
  in-progress benchmark lifecycle transition remains covered in both Python
  and JavaScript status tables.
- All temporary probes and mutations were removed. `git diff --quiet` exited
  0 before this report was written. The index still contained exactly the 131
  reviewed feature paths.
