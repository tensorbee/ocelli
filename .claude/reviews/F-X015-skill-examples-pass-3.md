# F-X015 executable skill examples review, pass 3

**Reviewed**: staged diff from `6eca661` in `/private/tmp/ocelli-f-x015`, 17
paths, 1381 insertions and 147 deletions
**Result**: 0 defects, 1 smell, 0 nitpicks

## Defects

None.

## Smells

### S1, The separate tilde-fence rule has no positive regression

**Where**: `scripts/tests/test_skill_examples_check.py:140`

The pass-2 repair correctly splits backtick and tilde fence openers because a
backtick is forbidden only in a backtick fence's info string. The new test
proves the refusal direction by making the invalid backtick opener ordinary
text. No test proves the other half, that a tilde fence whose info string
contains a backtick remains a valid inert fence.

This is an actionable coverage gap because the corrected plan and LLD both
make the separate tilde behavior an explicit claim. Applying the backtick
restriction to both regexes would make that valid documentation fail without
any focused unit naming why. Add one positive case with a tilde opener such as
four tildes followed by `text` and a backtick, a failing marker-shaped example
inside it, a four-tilde close, and one live passing example afterward. Assert
that only the live id is parsed and executes.

## Nitpicks

None.

## Pass-2 disposition

Pass-2 D1 is fixed. I exercised unclosed list-contained fences with two, three
and four spaces of continuation indentation. Dedenting from each made the
following exact column-zero declaration live, and all three parsed only the
live id. The implementation now checks the active list container before the
zero-to-three-space top-level fence form. The standing regression covers the
same three indentation values.

Pass-2 D2 is fixed in behavior. A four-backtick line whose info contains a
backtick no longer opens an inert fence. The failing marked assertion after it
was parsed alongside a later passing id, and execution failed at the first id
with exit 1. A valid four-tilde fence carrying the same backtick in its info
kept the failing marker inert and exposed only the later live id. S1 records
that this valid direction lacks standing coverage.

Pass-2 D3 is fixed. The expert example now evaluates -160 and 240 against the
LINEAR and LINEAR_EXACT lower and upper predicates before deriving the clamped
rows. Four independent mutations all drove the example red: changing only the
LINEAR lower `<=` to `<`, changing only the LINEAR_EXACT lower `<=` to `<`,
moving LINEAR's upper threshold from `w - 1` to `w + 1`, and changing the
LINEAR_EXACT upper `>` to `>=`. The interior exact fractions and displayed
truncation assertions remain intact.

## Pass-1 closures and independent evidence

The SIGMOID exponent-sign and width-predicate mutations remain red for
`stdout differs` and exit 1 respectively. Root and child skill symlinks
escaping the execution boundary remain refused. Actual `b"ok\r\n"` and
`b"ok\xff\n"` output against declared `ok\n` remain red by exact byte
comparison. A malformed later file still prevents an earlier valid example
from reaching the runner, preserving parse-all-before-execute ordering. The
fixed subprocess argument vector, temporary working directory, minimal
environment, timeout and bounded diagnostics are unchanged.

The three marked examples independently agree with the transcribed PS3.3
formulas and stored-value cases. Pass-1 S1 remains closed because the expert
check uses compact `Fraction` arithmetic rather than a second copy of both VOI
functions. The canonical expert skill is now 601 lines, but the pre-existing
size debt is separate from the duplicated implementation that this story
removed.

`bin/ocelli.sh gate skills` passed all 20 adapter hashes, three marked examples
and 31 checker tests. `bin/ocelli.sh gate ci` proved all 25 floor gates and the
complete `skills` arm are reached. The standalone catalogue suite passed 56
tests. `bin/ocelli.sh gate guards` completed 162 refusal probes, 24 accept
probes and 39 controls with zero open harness defects. The census reported 651
refusals in 62 files, 249 probes and zero sites watched by nothing. Prose,
staged-content and source-provenance gates passed.

The canonical SHA-256 values are
`4cf493ced4df6947897583bd30c49d49b6b328f2fe8f1b6ce5b4e035da08779a`
for `dicom-expert` and
`e113a108dc24489399e676cc3e6d17cef5b4d0ce5bf513fda6c2ec07b1a17fd0`
for `dicom-tooling`. They match the generated adapters, and both canonical
folders pass the skill-creator validator. The generated budget remains at 15
`skill-examples` refusal sites. The generated runbook carries all three
mutation rows, and `guard_census.py --check` passed.

No implementation file, generated artifact, verification record, commit,
integration or remote state was changed by this review.
