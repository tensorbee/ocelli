# F-X015 executable skill examples review, pass 4

**Reviewed**: staged diff from `6eca661` in `/private/tmp/ocelli-f-x015`, 18
paths, 1487 insertions and 147 deletions
**Result**: 0 defects, 0 smells, 0 nitpicks

## Defects

None.

## Smells

None.

## Nitpicks

None.

## Pass-3 disposition

Pass-3 S1 is fixed. The new positive regression opens a valid four-tilde fence
whose info string contains a backtick, places a failing exact marker inside,
closes with four tildes, and adds a live passing example afterward. It calls
`check_paths`, so the assertion proves both selection and execution rather than
parser output alone. Only `visible-pass` is returned.

I independently replaced the tilde opener rule with the backtick rule's
incorrect no-backtick restriction in memory. The formerly inert failing id
then parsed and execution went red with exit 1. This confirms the new test
protects the distinct CommonMark behavior that motivated the split regexes.

## Prior closure audit

All earlier repairs remain present and coherent in the full staged diff. Active
list continuation matching precedes top-level matching, and the standing test
covers dedent from unclosed two-space, three-space and four-space list fences.
Invalid backtick info cannot hide a live example. Blockquoted and closed
list-contained documentation remains inert. Exact column-zero declarations
return live when their container ends.

The expert check evaluates -160 and 240 through both LINEAR boundary
predicates before deriving its clamped rows. Its assertions still make the
four lower and upper semantic mutations measured in pass 3 observable. The
tooling example still makes the SIGMOID exponent sign and positive-width
predicate observable, and the catalogue carries both mutations. The compact
expert check does not duplicate the two tooling function bodies.

Canonical root, parent and child resolution checks continue to close the
symlink execution boundary. Parsing completes across every selected skill
before any example runs. Exact stdout remains a byte comparison, assertion
mode rejects output, and subprocess execution retains its fixed argument
vector, no-shell invocation, isolated temporary working directory, minimal
environment, timeout and bounded diagnostics.

The named `skills` gate still runs adapter sync, marked examples and the focused
unit suite in order with `&&`. CI invokes that named gate, and the reached
catalogue test asserts its exact arm. The guard catalogue, refusal budget,
generated runbook and LLD agree on the checker and its three semantic probes.

## Independent evidence

`bin/ocelli.sh gate skills` passed all 20 adapter hashes, three marked examples
and 32 checker tests. Applying the wrong tilde-info restriction made the new
positive case red. `bin/ocelli.sh gate ci` proved all 25 floor gates and every
command in their arms are reached, and the standalone catalogue suite passed
56 tests.

The census passed with 651 refusals in 62 files, 249 probes and zero sites
watched by nothing. The generated budget remains at 15 `skill-examples`
refusal sites, and the generated runbook still carries all three mutation
rows. Prose, staged-content and source-provenance gates passed.

The canonical SHA-256 values remain
`4cf493ced4df6947897583bd30c49d49b6b328f2fe8f1b6ce5b4e035da08779a`
for `dicom-expert` and
`e113a108dc24489399e676cc3e6d17cef5b4d0ce5bf513fda6c2ec07b1a17fd0`
for `dicom-tooling`. They match the generated adapters, and both canonical
folders pass the skill-creator validator.

No implementation file, generated artifact, verification record, commit,
integration or remote state was changed by this review.
