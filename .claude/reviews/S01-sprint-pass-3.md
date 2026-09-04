# S01 sprint review, pass 3

**Reviewed**: staged remediation against
`e5ab7baab9f25e75ee681505df5e1fa1cfd1cf75`
**Result**: 0 defects, 1 smell, 0 nitpicks

## Defects

None.

## Smells

### S1, the fail-closed metadata dispatch has no regression test

**Where**: `scripts/corpus_tests.py:251`,
`scripts/tests/test_corpus_synth.py:22`

**What**: pass 2 identified the missing-interpreter branch of
`--metadata-check` as a defect because it returned the shared skip status. The
branch now correctly returns 1, but none of the 62 corpus tooling tests invokes
`corpus_tests.py --metadata-check` with an unavailable interpreter. The six new
metadata tests call `corpus_check.metadata_problems` directly. They do not
exercise interpreter resolution or the dispatch exit status.

**Why it is wrong**: this is the precise boundary that allowed a mandatory
integrity check to become a successful skip. Changing `return 1` back to
`return SKIPPED_EXIT` leaves all current tests green, so the fixed behavior is
not protected from regression. The microscope rule asks whether a test fails
when the reviewed code is wrong. Here no test reaches the reviewed branch.

**Evidence**:

```text
$ OCELLI_PYTHON=/definitely/missing/python \
    python3 scripts/corpus_tests.py --metadata-check
FAIL: no interpreter can import pydicom and numpy. Tried:
    /definitely/missing/python (OCELLI_PYTHON): not present
    OCELLI_PYTHON was set explicitly, so no fallback was tried. Fix it or unset it.
The corpus metadata audit is part of the integrity gate, so an absent reader is a failure rather than a skip.
metadata_missing_exit=1

$ rg -n 'metadata-check|OCELLI_PYTHON|absent reader|SKIPPED_EXIT' scripts/tests
dispatch_test_search_exit=1
```

Add a subprocess-level test that sets `OCELLI_PYTHON` to an explicit missing
path, invokes `corpus_tests.py --metadata-check`, and asserts exit status 1.
The test should also assert the fixed fail-closed diagnostic and should be
included in a suite that `scripts/corpus_tests.py` actually runs.

## Nitpicks

None.

## Verified clean

- Pass-2 D1 is behaviorally closed. With `OCELLI_PYTHON` set to an explicit
  missing path, `scripts/corpus_tests.py --metadata-check` prints `FAIL` and
  returns 1. It no longer returns the gate runner's skip status 3.
- Successful metadata dispatch still runs `scripts/corpus_check.py --metadata`
  using the resolved interpreter, uses the repository root as its working
  directory and preserves the child exit status.
- The corpus gate chains coverage, digest verification and metadata audit with
  `&&`, so a failure at any stage fails the individual corpus gate.
- Ran `bin/ocelli.sh gate nostd`. It checked all eleven declared `no_std`
  crates and passed. The `nostd` entry remains in the gate registry and its
  dispatch arm executes `scripts/no_std_check.py`.
- The floor excludes only oracle and corpus, so the new `nostd` gate is in the
  floor, sprint and all profiles. CI also invokes the same no_std checker
  directly.
- Ran `python3 scripts/corpus_tests.py`. Seventeen coverage tests and
  forty-five generator tests passed with zero skips, for 62 tests total.
- The six metadata audit tests cover a clean generated corpus and wrong
  modality, transfer syntax, mono16, colour and US claims. Their metadata
  checks remain correct and their failures use manifest-relative paths.
- Ran `bash -n bin/ocelli.sh` and compiled `scripts/corpus_check.py` and
  `scripts/corpus_tests.py` with `py_compile`. Both checks passed.
- The staged documentation continues to match the observed fail-closed
  behavior and the 62-test corpus suite count.
- Pass-1 D1 and N1 remain closed. The no_std checker is wired into execution,
  and the unreproducible proptest drift samples remain removed.
