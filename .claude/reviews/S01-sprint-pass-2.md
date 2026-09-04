# S01 sprint review, pass 2

**Reviewed**: staged remediation against
`16e0d672370628dcd6c797e9d4a094245b8ef2b9`
**Result**: 1 defect, 0 smells, 0 nitpicks

## Defects

### D1, the mandatory metadata audit becomes a successful skip without an interpreter

**Where**: `scripts/corpus_tests.py:251`, `bin/ocelli.sh:132`,
`bin/ocelli.sh:195`

**What**: `gate corpus` now chains coverage, digest verification and
`corpus_tests.py --metadata-check`. If interpreter resolution fails, the new
metadata mode returns `SKIPPED_EXIT`, which is 3. `gates_cmd` classifies exit 3
as a skip and returns zero when there are no failed gates. A sprint or release
profile can therefore complete successfully after the corpus bytes and digests
pass even though none of the labels were audited.

**Why it is wrong**: this audit closes pass-1 S1 only if it is mandatory when a
corpus is present. A named skip is honest reporting, but the command's zero
profile status still permits the workflow to advance without the check that is
now claimed to prevent digest-valid misclassification. The new AS_BUILT entry
says a digest-valid row can no longer claim the wrong metadata silently. That
claim is false on a machine with no resolvable DICOM interpreter.

**Evidence**:

```text
$ OCELLI_PYTHON=/definitely/missing/python \
    python3 scripts/corpus_tests.py --metadata-check
SKIPPED: no interpreter can import pydicom and numpy. Tried:
    /definitely/missing/python (OCELLI_PYTHON): not present
    OCELLI_PYTHON was set explicitly, so no fallback was tried. Fix it or unset it.
metadata_missing_exit=3
```

`bin/ocelli.sh:195-213` adds status 3 to `skipped` and returns 0 whenever
`failed` is empty. Make the metadata mode return failure when called from the
corpus gate, or add a required-mode flag equivalent to
`--require-prerequisites` and use it in the corpus arm. Add a dispatch test for
the missing-interpreter case so exit 3 cannot return unnoticed.

## Smells

None.

## Nitpicks

None.

## Verified clean

- Pass-1 D1 is closed in the staged tree. `nostd` is present in `GATES`, its
  `run_gate` arm executes `scripts/no_std_check.py`, profile expansion includes
  it in the floor, sprint and all profiles, and CI invokes the same script.
- Ran `bin/ocelli.sh gate nostd`. It checked all eleven declared `no_std`
  crates and passed. The established red mutation remains recorded in the
  guard-verification runbook.
- Pass-1 N1 is closed. The two unreproducible proptest drift samples were
  removed and the comment now points only to the deterministic fixed case.
- The core metadata comparison is correct for the promised fields. It reads
  only the non-patient attributes Modality, Transfer Syntax UID, Samples per
  Pixel, Photometric Interpretation and Bits Allocated for its decisions.
- `mono16` requires one sample, a monochrome photometric interpretation and 16
  allocated bits. `colour` accepts one-sample palette colour or three-sample
  RGB and YBR photometric interpretations. `us` requires Modality US.
- Modality and Transfer Syntax UID are compared directly with the corresponding
  manifest fields. Missing fields resolve to values that fail the appropriate
  comparison rather than silently satisfying it.
- Metadata failures contain only the manifest-relative path, a fixed
  non-patient attribute description, or an exception class name. They do not
  interpolate attribute values, exception messages, absolute corpus paths or
  patient attributes.
- The parser uses `stop_before_pixels=True` and `specific_tags` for the four
  dataset attributes. Pixel bytes and patient identity fields are not loaded
  into the audit result.
- Ran `python3 scripts/corpus_tests.py`. Seventeen coverage tests and forty-five
  generator tests passed with zero skips. The six new tests cover the clean
  generated set and wrong modality, transfer syntax, mono16, colour and US
  claims.
- Interpreter dispatch uses the resolved interpreter to run
  `scripts/corpus_check.py --metadata`, preserves the child exit status and
  uses the repository root as its working directory. The only incorrect branch
  is the unavailable-interpreter status in D1.
- The corpus gate preserves failure from coverage and digest verification via
  `&&` before reaching metadata audit.
- Corpus README and the corpus LLD accurately describe the attributes checked,
  relative-path failure output and the new operator step, subject to D1's
  missing-interpreter exception.
- The AS_BUILT correction is append-only and its test count is accurate. The
  suite now contains 17 plus 45 tests, for 62 total. Its claim that the audit is
  enforced needs D1 fixed before it becomes true on every supported machine.
- Rechecked the S01 sprint diff where these changes interact. The coordinate
  arithmetic, compile-fail space separation, corpus coverage, strict `--all`
  and release path, and S01-only oracle exception are unchanged.
