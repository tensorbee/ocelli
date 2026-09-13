# F-028 review, pass 2

**Reviewed**: working tree after pass 1's remediation. The remediation touched
`crates/ocelli-codec/tests/jpegls.rs` and
`crates/ocelli-codec/tests/fixtures/generate_jpegls.py` only. **No adapter
source changed**, which is the first thing this pass checked.
**Result**: 0 defects, 0 smells, 0 nitpicks

## Defects

None.

## Smells

None.

## Nitpicks

None.

## The remediation is tests only, and that is checkable

`git diff` between the pass 1 tree and this one touches two test-side files and
one generated fixture. `crates/ocelli-codec/src/jpegls.rs`,
`src/sample_convert.rs` and `src/jpeg2000.rs` are byte-unchanged. Pass 1's three
smells were all "a branch nothing reaches", and the correct answer to that is a
test rather than a code change, so a remediation that had touched the adapter
would itself have been the finding.

**One of the three could have gone the other way and is worth recording.** S3's
stray SOI and EOI guard was a candidate for deletion rather than for a test,
because the walk terminates on its own and the guard looked redundant. It is
not: a stray EOI whose length bytes step exactly over themselves lets the walk
read the following SOF55 and SOS as the frame's own, accept a well formed
header pair, and hand the dependency bytes that are not a scan. The guard turns
that from a decoder failure into a structural refusal, which is the right answer
about a structurally invalid stream. The test is built from exactly that input,
so the guard is kept **because** it is falsifiable rather than kept in case.

## Verified clean

- **Fifteen fixtures pass.** The `jpegls` target reports fifteen passed and none
  failed, `bin/ocelli.sh clippy ocelli-codec` exits 0, and the crate is green
  across its eight targets.
- **The three pass 1 probes were re-run against the remediated tree and all
  three are now red**, where all three were green before:

  | Probe | Before | After |
  |-------|--------|-------|
  | Codestream component check inverted | 12 passed, 0 failed | 3 passed, 11 failed |
  | Descriptor sample-count check removed | 12 passed, 0 failed | 13 passed, 1 failed |
  | Stray SOI and EOI guard defeated | 14 passed, 0 failed | 14 passed, 1 failed |

- **The eight probes that were already red stayed red**, and the tree was
  restored from a byte copy and re-run green after each.
- **The new fixture is generated rather than hand-placed.**
  `jpegls_rgb8_ilv1.jls` comes from `generate_jpegls.py`, which asserts its
  SOF55 declares three components before writing it, so the fixture cannot
  quietly become single-component and leave the test passing for the wrong
  reason. Re-running the generator without `--write` reports every one of the
  ten artefacts as `ok`.
- **The hand-built stray-marker stream is spelled out byte by byte in the
  test**, with each marker segment's fields named in the comment, so a reader
  can check it against ISO/IEC 14495-1 C.2.2 and C.2.3 without running anything.
- **Nothing was widened to make a test pass.** No tolerance, no `#[allow]`, no
  gate change, no allow-list entry. The only non-test file touched in the whole
  story after pass 1 is `docs/sprints/BACKLOG.md`, whose F-028 row moved from
  `pending` to `in-progress` because `scripts/bench_check.py` refuses a runner
  for a story that has not started. That is the anti-fabrication rule working
  rather than a rule being bent: it refused the runner, the status was corrected
  to the truth, and it passed.
