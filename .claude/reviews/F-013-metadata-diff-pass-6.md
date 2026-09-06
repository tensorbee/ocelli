# F-013 metadata diff review, pass 6

**Reviewed**: the staged F-013 diff before this review, 27 paths with 2,684
insertions and 252 deletions

**Result**: 0 defects, 0 smells, 1 nitpick

## Defects

None.

## Smells

None.

## Nitpicks

### N1. Pass 3 retains a wrong historical fixture total

The pass-3 review says 53 integration fixtures. Its own suite split and the
pass-5 test output total 43. This is confined to a prior review record and does
not weaken the implementation or its evidence.

## Recheck of the pass-5 defect

Pass 5 found that the plan still described the removed helper-only regression.
That finding is closed. The corrected plan now states that the standing
combined mutation:

- supplies a genuine metadata-truth divergence
- refuses either input frame reader if invoked
- requires a metadata-truth failure with no statistics
- makes the mutation gate red when either production frame read is moved ahead
  of metadata truth

This agrees with both the current implementation and the pass-5 adversarial
evidence. The earlier plan account also correctly records why the helper-only
test was insufficient and why the catalogue now exercises the production
caller.

## Current production path

`compare_runs` obtains the independent metadata truth first, then passes both
frame reads inside the closure supplied to `metadata_before_frames`. A metadata
failure is converted into its final record without invoking that closure.

The combined mutation changes the candidate presentation LUT declaration to a
value that disagrees with truth. `read_side` checks the combined refusal before
opening a frame and does not restrict that refusal by mutation side. The normal
frame-damage branch remains restricted to the declared side. The standing
mutation therefore guards both the reference and candidate production reads
without broadening ordinary frame mutations.

The catalogue expectation is still `Fail`, `MetadataTruth`, attributed to
ours. The effect and its documentation consistently say that either input can
refuse. The comparator LLD gives the same account and records 29 catalogue
entries, of which the first eight are the F-013 metadata probes.

## Mutation and restoration evidence

Pass 5 separately moved only the reference read and only the candidate read
ahead of the metadata boundary. In both cases
`metadata-failure-precedes-frame-refusal` was the sole undetected mutation and
the mutation command returned red with 29 mutations and 1 not detected. After
each adversarial edit, the source was restored exactly. The restored baseline
then detected all 29 mutations and compared 99 views as 71 pass and 28
unmeasured with aggregate hash
`413e030d7b202d5a274e85c725e70b53e40e154ed1beea3041f052b254e1c603`.

The current source still has the restored lazy ordering and the two-sided
refusal. Before this review was added, `git diff --name-only` was empty, so the
working tree had no unstaged tracked change. The staged implementation tree
was the same 27-path tree reviewed in pass 5 except for the subsequent plan
correction.

## Lightweight verification

The final recheck passed:

- `bin/ocelli.sh check ocelli-oracle`
- `bin/ocelli.sh fmt --check`
- `bin/ocelli.sh gate prose`
- `bin/ocelli.sh gate content`

The full Rust, JavaScript, comparator, and mutation results from pass 5 remain
applicable because the only later staged change was the plan correction. No
tolerance, implementation source, fixture, corpus row, or generated truth file
changed during this pass.

## Conclusion

All prior defects and smells are closed. The current plan describes the
standing two-sided combined mutation accurately, the source and staged tree
are restored, and this final recheck found no new defect or smell. The one
remaining nitpick is an incorrect count in a historical review artifact.
