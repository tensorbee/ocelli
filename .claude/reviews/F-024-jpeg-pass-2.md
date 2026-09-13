# F-024 JPEG review, pass 2

**Reviewed**: full staged working tree on `work/f-024-codex` at base `586e503`
**Result**: 2 defects, 0 smells, 0 nitpicks

## Defects

### D1, the public decoder contract still forbids the allocations D-21 permits

**Where**: `crates/ocelli-codec/src/registry.rs:345`

**What**: The public `Decoder::decode` documentation still says that a call
must not allocate. F-024's JPEG implementation allocates dependency-owned
decoded storage on every route and also copies the encoded input for twelve-bit
`.51`.

**Why it is wrong**: D-21 deliberately replaces that unconditional HLD
sentence for concrete JPEG and JPEG 2000 adapters. Leaving the original
sentence on the public trait advertises a contract that the newly registered
JPEG implementation does not satisfy. The deviation register is correct after
the pass-one remediation, but it does not make contradictory API documentation
true.

**Evidence**: `rg -n "Must not allocate|allocation"` found the unconditional
trait sentence at `registry.rs:345`, the D-21 exception at
`docs/hld/DEVIATIONS.md:38`, and the implementation allocations at
`jpeg.rs:156` and `jpeg.rs:198`. `docs/lld/codecs.md:20-24` also states that
JPEG decode allocates under D-21.

### D2, the approved cross-target test row names commands that do not prove its claim

**Where**: `.claude/plans/F-024-design.md:130`

**What**: The row says `bin/ocelli.sh check ocelli-codec` and
`bin/ocelli.sh wasm` prove that every registered UID builds and runs through
the same adapter on native and wasm. The first command is a host check. The
second builds `ocelli-wasm`, whose dependency graph does not contain
`ocelli-codec`. Neither command runs the JPEG adapter on wasm.

**Why it is wrong**: A test matrix must name the executable evidence for its
claim. The repository does have a wasm compile proof in `gate native`, but the
approved row does not name it and overclaims wasm execution that the current
workflow explicitly reserves for a later browser runner.

**Evidence**: `bin/ocelli.sh wasm` exited 0 after compiling `ocelli-core` and
`ocelli-wasm`, with no `ocelli-codec` build. `cargo tree -p ocelli-wasm | rg
'ocelli-codec'` exited 1. `bin/ocelli.sh gate native` exited 0 and its step 2
did compile `ocelli-codec` for `wasm32-unknown-unknown`. Lines 462 through 467
of `bin/ocelli.sh` state that wasm tests need a later browser runner.

## Smells

None.

## Nitpicks

None.

## Pass-one remediation proofs

1. `.51` accepts legal eight-bit process 2 as well as twelve-bit process 4.
   `extended_process_2_eight_bit_decodes_against_independent_dcmtk_truth`
   passed, and DICOM PS3.5 A.4.1 assigns both processes to `.51`.
2. Baseline output is compared with independent synthetic corpus truth through
   fixed per-channel class-two measurements.
   `baseline_colour_reports_rgb_and_class_two_measurement_against_corpus_truth`
   passed.
3. JPEG registration preflights all four exact UIDs. Removing `.70` from the
   preflight made `jpeg_registration_is_atomic_when_the_last_uid_collides`
   fail at exit 101 because `.50`, `.51`, and `.57` became available. Reverting
   the mutation made the same exact test pass at exit 0.
4. D-21 now names the bounded encoded-input copy required by the safe packet
   API as well as dependency-owned decoded output. This matches
   `Packet::new(..., src.to_vec())` in the implementation. D1 records the
   remaining contradictory public API sentence.
5. The plan now says dependency decoders and buffers are constructed inside
   each decode call. Inspection of the enum-only constructors and decode
   functions matches that account.
6. The output-description prose now says the query reports conversion and
   leaves consumers responsible for using it. Repository search found no
   production consumer, matching the corrected limitation.

## Verified clean

- `bin/ocelli.sh test ocelli-codec` exited 0 with 2 unit tests, 8 JPEG tests,
  and 14 registry tests passing.
- `bin/ocelli.sh check ocelli-codec` and
  `bin/ocelli.sh clippy ocelli-codec` both exited 0.
- `bin/ocelli.sh cargo check -p ocelli-codec --all-targets --target
  wasm32-unknown-unknown` exited 0.
- `bin/ocelli.sh gate native` exited 0. Its four proofs included the shared
  workspace wasm build and cross-target feature equality.
- `bin/ocelli.sh wasm` exited 0 and stayed within the existing wasm boundary.
  D2 records why it is not evidence for the codec's wasm build.
- `bin/ocelli.sh gate bench` exited 0 with 25 Python tests and 80 passing Node
  tests. The one browser-only cold-start test was explicitly skipped by the
  existing gate design.
- `fmt`, `unsafe`, `provenance`, `prose`, `content`, `pins`, `deviations`, and
  `bindgen` gates each exited 0.
- Exact JPEG UIDs remain separately configured. `.70` checks selection value 1
  in every scan, matching DICOM PS3.5 A.4.1 and section 10.2.
- Lossless fixtures remain hand-computed and byte-exact. The lossy fixtures use
  independently established truth and the unchanged HLD comparison policies.
- Failure paths inspected before the final copy leave the caller output
  unchanged. No repository `unsafe`, `wasm-bindgen`, patient data, render-loop
  work, or tier-specific pixel arithmetic was added.
- `git diff --cached --check` exited 0 before this report was added.
