# S01 sprint review, pass 1

**Reviewed**: `main...16e0d672370628dcd6c797e9d4a094245b8ef2b9`
**Result**: 1 defect, 1 smell, 1 nitpick

## Defects

### D1, the D-09 guard is never run by a gate, hook or CI

**Where**: `scripts/no_std_check.py:1`, `bin/ocelli.sh:55`,
`.github/workflows/ci.yml:42`, `docs/sprints/AS_BUILT.md:114`

**What**: F-001 added `scripts/no_std_check.py` to hold deviation D-09, and the
delivery record says it is what actually holds that deviation. The script is
not present in `GATES`, no `run_gate` arm invokes it, and neither CI nor a Git
hook invokes it directly. A normal floor, feature, sprint or release
verification therefore does not execute the guard.

**Why it is wrong**: D-09 exists because enabling glam's default `std` feature
silently defeats the declared `no_std` posture. The story's own measured
evidence says the Rust and wasm checks stay green under that regression. A
standalone guard that no delivery path runs is the exact defect class described
by the microscope workflow as "a guard that exists and nothing executes".

**Evidence**:

```text
$ rg -n 'no_std_check\.py' --glob '!docs/runbooks/guard-verification.md' \
    --glob '!docs/sprints/AS_BUILT.md' .
[no output]

$ bin/ocelli.sh gate --floor
...
GREEN  15 passed, 2 skipped. A skipped gate is NOT a pass.

$ python3 scripts/no_std_check.py --verbose
  ocelli-core: clean
  ...
OK: 11 no_std crate(s) reach no std feature
```

The last command proves the guard itself works on the current tree. Its absence
from the preceding gate proves the enforcement gap. Add it as a named gate in
`bin/ocelli.sh`, include it in the floor, and let the existing profile expansion
carry it into feature, sprint and release verification. CI should execute the
same named guard or the floor containing it.

## Smells

### S1, corpus coverage trusts labels it never compares with the DICOM metadata

**Where**: `scripts/corpus_check.py:94`, `scripts/corpus_check.py:142`,
`scripts/corpus_check.py:276`, `scripts/corpus_check.py:325`

**What**: coverage derives transfer-syntax and tolerance-class claims entirely
from TSV strings. Digest verification proves only that the bytes did not move.
The documented `--add` path accepts caller-supplied modality, transfer syntax
and category values, hashes the file, and appends the row without comparing any
of those labels with the file's metadata. The synthetic conformance suite
checks generated cases, but no gate performs the same comparison for real cases
or later manually added cases.

**Why it will cause a defect later**: a typo such as a real 8-bit greyscale case
labelled `mono16`, or a transfer syntax copied from the wrong file, remains
digest-valid and makes `--coverage` report coverage the corpus does not have.
This is a quietly wrong evidence problem rather than a malformed-manifest
problem.

**Evidence**:

```text
$ python3 scripts/corpus_check.py --coverage
coverage over 91 manifest rows, 44 of them real
  transfer syntaxes: 16 of 16
  monochrome 16-bit rows: 85
  colour or ultrasound rows: 6 (1 real, of which 0 carry chroma)
OK: coverage complete
```

In the implementation, `coverage()` reads `row["transfer_syntax"]` and category
tokens only. `verify()` reads only path and digest. `add()` writes every semantic
field from command-line arguments. Add a corpus-present metadata audit, using
the repository-approved DICOM tooling, that compares at least Modality,
TransferSyntaxUID and the category-driving pixel attributes against every
manifest row. It should name only the relative corpus path on failure and must
not emit patient attributes.

## Nitpicks

### N1, the perspective property comment carries two unreproducible sample values

**Where**: `crates/ocelli-core/tests/roundtrip.rs:99`

**What**: the comment says prior generated runs reported drifts of 51.1 and
71.4 pixels. Those values depend on whichever case proptest generated and
shrunk, so they cannot be reproduced from the test. The fixed case immediately
below already provides the useful measured value. Delete the two historical
figures or retain only the reproducible fixed case.

## Verified clean

- Read `CLAUDE.md`, `AGENTS.md`, `.claude/WORKFLOW.md`, both approved S01
  design plans, the microscope workflow, the DICOM expert reference and the
  DICOM tooling reference.
- Compared the coordinate and value-space implementation with HLD sections 16
  and 16.1. `Pt<S>` and `Transform<A, B>` preserve space typing. `identity` is
  restricted to `Transform<S, S>`. Composition order is `next.m * self.m` and
  the perspective path uses `project_point3`.
- Checked every Rust cast in the delivered F-001 surface. No `as` cast was
  added. No `unsafe`, `wasm-bindgen`, render-loop allocation, trait object,
  one-implementer trait or forwarding wrapper was added.
- Recomputed the geometry fixture from PS3.3 C.7.6.2.1.1. Column index `i`
  uses `PixelSpacing[1]` along the row cosine. Row index `j` uses
  `PixelSpacing[0]` along the column cosine. The far-corner expected value
  agrees with the recorded hand calculation.
- Ran `bin/ocelli.sh test ocelli-core`. Fourteen unit tests, four compile-fail
  cases, six DICOM geometry fixtures and three round-trip tests passed.
- Ran `python3 scripts/corpus_tests.py`. Seventeen manifest and coverage tests
  and thirty-nine generator, fixture, determinism and conformance tests passed
  with zero skips.
- Checked the stored-value fixture against PS3.3 C.7.6.3.1.4. It shifts by
  `HighBit + 1 - BitsStored`, masks to `BitsStored`, and sign-extends from
  `BitsStored - 1`. The right-aligned and left-aligned cases share raw words
  and deliberately differ in meaning.
- Checked generator determinism controls, fixed UIDs and dates, transfer-syntax
  declarations, encapsulated OB pixel data, JPEG-LS NEAR bound, RLE segment
  count, HTJ2K signalling and progression order, per-frame functional groups,
  planar RGB, YBR_FULL_422 size, and uniform versus non-uniform slice spacing.
- Ran manifest coverage. It reports 91 rows, 44 real cases, all 16 declared
  transfer syntaxes, 85 monochrome 16-bit rows and 6 colour or ultrasound rows.
  It also reports the real-chroma gap rather than hiding it.
- Confirmed DICOM bytes remain outside Git. The tracked manifest carries
  relative paths, lowercase SHA-256 digests, source, licence and licence URL.
  No prohibited source was opened.
- Ran the complete floor profile. Fifteen gates passed and the two expected
  bootstrap gates, docs source and wasm package, were named skips. The missing
  D-09 invocation is D1 above.
- Ran backlog consistency and checked the delivery ledgers. F-001 and F-009
  are done in CURRENT_SPRINT and BACKLOG, and both have tracker and AS_BUILT
  records.
- Checked the S01 oracle exception in `bin/ocelli.sh` and the synchronized
  workflow documents. It is limited to the sprint profile, exact active S01,
  F-010 pending in S02 and absent oracle dependencies. `--all` and release stay
  strict. The canonical command files and generated Codex adapters agree.
- Checked HEAD verification evidence. Commit `16e0d67` carries a sprint-profile
  trailer for its own tree, records corpus pass, names only gates that ran, and
  identifies Codex as generator.
- Reviewed append-only and workflow-change process. The workflow exception has
  its trigger appended to AS_BUILT, updates the affected commands and adapters,
  and landed separately from feature work.
